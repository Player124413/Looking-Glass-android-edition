//! Mirror Image: one owner for the clocks, machinery, paired bosses and scene handoffs.
mod art;
mod battle;
mod check;
mod data;
mod motion;
mod presentation_check;
mod performance_check;
mod route;
mod saves;
mod scene;
mod sound;
use super::{state, Beat, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    combat::{Feedback, Hit, Target},
    entity::{Id, Registry},
    event::{Condition, Effect, Facts},
    interaction::Events,
    inventory::Stats,
    level::exit::ExitState,
    level::scene::{SceneRunner, SceneState},
    level::spec::{EndSpec, ExitSpec, SceneSpec, ShotSpec},
    level::{LevelArt, LevelController, TriggerInfo},
    movement::Player,
    skeletal::Transform,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use scene::{Kind, Scene};
use serde::{Deserialize, Serialize};
use std::{any::Any, collections::BTreeMap};
const BASE: usize = 8_200_000;
const EXIT: ExitSpec = ExitSpec {
    map: "hatter1",
    entrance: "hatter1_start1",
};
const CLOSETS: [&str; 8] = [
    "auto01", "auto02", "auto03", "auto04", "auto05", "auto06", "phant01", "phant02",
];
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    initialized: bool,
    time: f32,
    arrived: bool,
    cells: u8,
    mirror: bool,
    mirror_open: bool,
    tube: Option<f32>,
    closets: [Option<f32>; 8],
    all_cells: Option<f32>,
    gas: bool,
    gas_start: Option<f32>,
    jack: bool,
    doors: Vec<f32>,
    #[serde(default)]
    door_signs: Vec<f32>,
    #[serde(default)]
    fulcrums: Vec<Vec2>,
    holds: Vec<f32>,
    t96: bool,
    scene: Option<Scene>,
    stand: Option<f32>,
    floor: Option<f32>,
    fight: bool,
    bosses: Vec<battle::Boss>,
    minis: Vec<battle::Boss>,
    serial: usize,
    death_wait: f32,
    step: f32,
    essence: bool,
    essence_side: usize,
    essence_wait: f32,
    essence_generation: u32,
    exit: ExitState,
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("funhouse time", self.time, 1e7)?;
        ensure!(
            self.doors.len() == self.holds.len() && self.doors.len() < 256,
            "Invalid machinery layout"
        );
        for x in &self.doors {
            state::fraction("door", *x)?;
        }
        ensure!(
            self.door_signs.is_empty()
                || (self.door_signs.len() == self.doors.len()
                    && self.door_signs.iter().all(|x| *x == 1. || *x == -1.)),
            "Invalid door directions"
        );
        for x in &self.holds {
            state::clock("door hold", *x, 10.)?;
        }
        ensure!(
            self.fulcrums.is_empty()
                || (self.fulcrums.len() == self.doors.len()
                    && self
                        .fulcrums
                        .iter()
                        .all(|v| v.is_finite() && v.abs().max_element() <= 90.)),
            "Invalid fulcrum angles"
        );
        for x in self
            .closets
            .into_iter()
            .chain([
                self.tube,
                self.all_cells,
                self.gas_start,
                self.stand,
                self.floor,
            ])
            .flatten()
        {
            state::clock("world cue", x, self.time)?;
        }
        ensure!(
            self.all_cells.is_some() == (self.cells == 255),
            "Invalid clock mask"
        );
        ensure!(
            !self.gas || self.cells == 255,
            "Gas doors opened before clocks"
        );
        ensure!(
            self.bosses.len() == 2 && self.minis.len() <= 24 && self.serial < 90_000,
            "Invalid Tweedle roster"
        );
        for (i, b) in self.bosses.iter().enumerate() {
            ensure!(
                b.id == BASE + i && b.family == i && !b.mini,
                "Invalid boss identity"
            );
            b.validate()?;
        }
        let mut ids = std::collections::BTreeSet::new();
        for b in &self.minis {
            ensure!(
                b.mini && b.id >= BASE + 100 && b.id < BASE + 100 + self.serial && ids.insert(b.id),
                "Invalid miniature identity"
            );
            b.validate()?;
        }
        for i in 0..2 {
            ensure!(
                self.minis
                    .iter()
                    .filter(|b| b.family == i && b.health > 0.)
                    .count()
                    <= i + 2,
                "Miniature cap exceeded"
            );
        }
        state::clock("death gate", self.death_wait, 5.)?;
        state::clock("combat remainder", self.step, 0.02)?;
        state::clock("essence timer", self.essence_wait, 10.)?;
        ensure!(self.essence_side < 2, "Invalid essence side");
        ensure!(
            !self.exit.committed
                || (self.floor.is_some() && self.bosses.iter().all(|b| b.health <= 0.)),
            "Exit before defeat"
        );
        if let Some(s) = &self.scene {
            s.validate()?;
        }
        Ok(())
    }
}
struct Funhouse {
    saved: Saved,
    data: data::Data,
    objects: Vec<motion::Object>,
}
impl Funhouse {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let data = data::Data::load(a, map)?;
        let objects = motion::load(map)?;
        let bosses = vec![
            battle::Boss::new(BASE, 0, false, data.points["dee_pos1"]),
            battle::Boss::new(BASE + 1, 1, false, data.points["dum_pos2"]),
        ];
        let mut out = Self {
            saved: Saved {
                version: 1,
                initialized: false,
                time: 0.,
                arrived: false,
                cells: 0,
                mirror: false,
                mirror_open: false,
                tube: None,
                closets: [None; 8],
                all_cells: None,
                gas: false,
                gas_start: None,
                jack: false,
                doors: vec![0.; objects.len()],
                door_signs: vec![1.; objects.len()],
                fulcrums: vec![Vec2::ZERO; objects.len()],
                holds: vec![0.; objects.len()],
                t96: false,
                scene: None,
                stand: None,
                floor: None,
                fight: false,
                bosses,
                minis: vec![],
                serial: 0,
                death_wait: 0.,
                step: 0.,
                essence: false,
                essence_side: 1,
                essence_wait: 0.,
                essence_generation: 0,
                exit: Default::default(),
            },
            data,
            objects,
        };
        out.begin(Kind::Arrival);
        out.rebuild(map)?;
        Ok(out)
    }
    fn cell(&mut self, i: usize) {
        self.saved.cells |= 1 << i;
        if self.saved.cells == 255 {
            self.saved.all_cells.get_or_insert(self.saved.time);
        }
    }
    fn begin(&mut self, kind: Kind) {
        if self.saved.scene.is_none() {
            self.saved.scene = Some(Scene::new(kind));
        }
    }
    fn elapsed(&self, t: Option<f32>, duration: f32) -> f32 {
        t.map_or(0., |t| ((self.saved.time - t) / duration).clamp(0., 1.))
    }
}
impl LevelController for Funhouse {
    fn id(&self) -> &'static str {
        "funhouse"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("funhouse.free", self.saved.scene.is_none());
        f.flag(
            "funhouse.arena",
            self.saved.gas && !self.saved.fight && !self.saved.exit.committed,
        );
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        if self.data.inhibited.contains(&t.id.0) {
            Some(Condition::Always.not())
        } else if t.thread == Kind::Tweedles.id() {
            Some(Condition::All(vec![
                Condition::flag("funhouse.free"),
                Condition::flag("funhouse.arena"),
            ]))
        } else if t.class == crate::level::TriggerClass::Exit {
            Some(Condition::flag("funhouse.exit"))
        } else {
            None
        }
    }
    fn shootable_thread(&self, n: &str) -> bool {
        n == "MirrorTrigger" || (1..=8).any(|i| n == format!("cell{i}"))
    }
    fn receivers(&self, _: &Registry) -> Vec<Id> {
        self.objects
            .iter()
            .filter(|o| matches!(o.name.as_str(), "mirror_path_door" | "t96"))
            .map(|o| Id(o.id))
            .collect()
    }
    fn rules(&self, ctx: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        self.receivers(ctx.registry)
            .into_iter()
            .map(|id| crate::event::Rule {
                key: format!("funhouse/receiver/{}", id.0),
                event: crate::event::Event::Entity(id, crate::event::Input::Activate),
                condition: Condition::Always,
                once: false,
                cooldown: 0.,
                actions: vec![crate::event::Action::Output(Effect::Activate(id))],
            })
            .collect()
    }
    fn output(&mut self, e: &Effect) -> Option<Events> {
        let id = match e {
            Effect::Activate(id) => *id,
            _ => return None,
        };
        let o = self.objects.iter().find(|o| o.id == id.0)?;
        match o.name.as_str() {
            "mirror_path_door" => self.saved.mirror_open = !self.saved.mirror_open,
            "t96" => self.saved.t96 = true,
            _ => return None,
        }
        Some(Events::default())
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        let mut out = Events::default();
        if let Some(i) = (1..=8).find(|i| n == format!("cell{i}")) {
            if self.saved.cells & (1 << (i - 1)) == 0 {
                out.sound = Some(
                    if [1, 4, 6, 8].contains(&i) {
                        "sound/ambience/special/glassimpact1.wav"
                    } else {
                        "sound/ambience/special/glassimpact2.wav"
                    }
                    .into(),
                );
            }
            self.cell(i - 1);
        } else if let Some(i) = CLOSETS.iter().position(|s| n == format!("{s}spawn")) {
            self.saved.closets[i].get_or_insert(self.saved.time);
        } else {
            match n {
                "MirrorTrigger" => {
                    if !self.saved.mirror {
                        self.saved.mirror = true;
                        self.saved.mirror_open = true;
                        out.sound = Some("sound/ambience/special/glassimpact2.wav".into());
                    }
                }
                "Funhouse_Rotate_Tuber" => {
                    self.saved.tube.get_or_insert(self.saved.time);
                }
                "Cat_Jack_Dialog" => {
                    if !self.saved.jack {
                        self.begin(Kind::Jack);
                    }
                }
                "Funhouse_Tweedle_Cinema1" => {
                    if self.saved.gas && !self.saved.fight {
                        self.begin(Kind::Tweedles);
                    }
                }
                "L_KILL_EM1" | "L_KILL_EM2" | "R_KILL_EM1" | "R_KILL_EM2" => (),
                _ => return None,
            }
        }
        Some(out)
    }
    fn prepare_player(&mut self, s: &mut Stats, _: &mut Player) {
        if !self.saved.initialized {
            s.full_stats();
            self.saved.initialized = true;
        }
    }
    fn scripted(&self) -> bool {
        self.saved
            .scene
            .as_ref()
            .is_some_and(|s| s.kind != Kind::Jack)
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn hides_player(&self) -> bool {
        self.scripted()
            && self
                .saved
                .scene
                .as_ref()
                .is_some_and(|s| s.kind != Kind::Gas)
    }
    fn dismiss_summons(&self) -> bool {
        self.saved
            .scene
            .as_ref()
            .is_some_and(|s| matches!(s.kind, Kind::Tweedles | Kind::Hatter))
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved.scene.as_ref().map(|s| s.kind.id())
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, s: &mut Story) -> bool {
        self.prepare_scene_story(s)
    }
    fn sync_story(&mut self, s: &Story) {
        self.sync_scene_story(s)
    }
    fn dialogue_complete(&mut self, n: &str) -> Events {
        if let Some(s) = &mut self.saved.scene {
            if s.kind.id() == n {
                s.ending.get_or_insert(s.clock.time);
            }
        }
        Events::default()
    }
    fn skip(
        &mut self,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        let Some(s) = &self.saved.scene else {
            return Ok(false);
        };
        story.finish_sequence(s.kind.id());
        if s.kind == Kind::Hatter {
            // Skipping dialogue still runs the physical floor/exit tail once.
            let s = self.saved.scene.as_mut().unwrap();
            s.ending.get_or_insert(s.clock.time);
            return Ok(true);
        }
        self.finish_scene(map, w, p)?;
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<crate::cinematic::Camera> {
        self.scene_camera()
    }
    fn scene_fovy(&self, aspect: f32) -> Option<f32> {
        self.scene_fov(aspect)
    }
    fn quake_offset(&self) -> Vec3 {
        let strength = self
            .saved
            .bosses
            .iter()
            .chain(&self.saved.minis)
            .map(|b| b.quake)
            .fold(0., f32::max)
            .min(1.);
        if self.scripted() {
            return Vec3::ZERO;
        }
        let t = self.saved.time;
        vec3((t * 63.).sin(), (t * 79.).sin(), (t * 93.).sin()) * strength * 2.
    }
    fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let dt = dt.min(0.1);
        self.move_world(dt, map, w, p, fixed)?;
        self.saved.exit.advance(dt);
        if self.saved.scene.is_none()
            && !self.saved.gas
            && self
                .saved
                .all_cells
                .is_some_and(|t| self.saved.time >= t + 1.)
        {
            self.saved.gas_start = Some(self.saved.time);
            self.begin(Kind::Gas);
        }
        self.advance_scene(dt, map, w, p)?;
        self.rebuild(map)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        self.traversal(&mut w.traversal);
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| o.draw)
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid)
            .map(|o| o.collider.clone())
            .collect()
    }
    fn camera_colliders(&self) -> Vec<Collider> {
        // The unnamed entrance screen behind Alice has ScriptSlave's non-solid
        // flag. Preserve that movement behavior, but keep the camera in front.
        self.objects.iter()
            .filter(|o| o.draw && !o.solid && o.name.is_empty())
            .map(|o| o.collider.clone())
            .collect()
    }
    fn settled_supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid && o.static_support)
            .map(|o| o.collider.clone())
            .collect()
    }
    fn traversal(&self, t: &mut crate::traversal::Traversal) {
        for path in &mut t.currents {
            for n in &mut path.nodes {
                n.active = (2. ..4.).contains(&self.saved.time.rem_euclid(7.));
            }
        }
        for push in &mut t.pushes {
            if let Some(i) = self.data.pushes.iter().position(|id| *id == push.id.0) {
                push.enabled = self.saved.closets[i].is_none_or(|t| self.saved.time < t + 1.);
            }
        }
    }
    fn particles(&self, p: &mut crate::particles::Steam) {
        self.particle_state(p)
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.world_sound(loops, clocks);
    }
    fn update(&mut self, _: &mut World, _: &Player, _: Vec3, _: bool) -> Events {
        let mut out = Events::default();
        out.transition = self.saved.exit.request(EXIT);
        out
    }
    fn transition_failed(&mut self, e: &(String, Option<String>)) {
        if EXIT.matches(e) {
            self.saved.exit.failed();
        }
    }
    fn targets(&self) -> Vec<Target> {
        if !self.saved.fight || self.scripted() {
            vec![]
        } else {
            self.saved
                .bosses
                .iter()
                .chain(&self.saved.minis)
                .filter(|b| b.health > 0.)
                .map(|b| b.target())
                .collect()
        }
    }
    fn hostile_target(&self, id: usize) -> bool {
        self.targets().iter().any(|t| t.id == id)
    }
    fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if !self.saved.fight || self.scripted() {
            return None;
        }
        self.saved
            .bosses
            .iter_mut()
            .chain(&mut self.saved.minis)
            .find(|b| b.id == h.id)?
            .hit(h)
    }
    fn combat(&mut self, c: &mut crate::level::Combat<'_>) -> Feedback {
        self.fight_step(c)
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let s = state::load::<Saved>(v, state::Visit { returning: false })?;
        ensure!(
            s.doors.len() == self.objects.len(),
            "Changed Funhouse layout"
        );
        self.saved = s;
        self.rebuild(map)
    }
    fn upgraded(&mut self) {
        self.saved.initialized = true;
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            respawn: crate::level::Respawn::Always,
            rearm: vec![Kind::Tweedles.id().into(), Kind::Jack.id().into()],
            ..Default::default()
        }
    }
    fn objective(&self) -> Option<String> {
        Some(if self.saved.fight {
            "Defeat both Tweedles.".into()
        } else if self.saved.gas {
            "Pass the suction mouths and reach the arena.".into()
        } else {
            format!(
                "Break the cell clocks: {}/8.",
                self.saved.cells.count_ones()
            )
        })
    }
    fn sky_origin(&self) -> Option<Vec3> {
        self.data.points.get("skymojo").map(|p| p.translation)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub static REGISTRATION: Registration = Registration {
    id: "funhouse",
    applies: |m, e| super::first_visit(m, e, "funhouse"),
    load: |a, m, _, _| Ok(Box::new(Funhouse::load(a, m)?)),
    art: Some(|a, _, o| {
        Ok(Box::new(art::Art::load(
            a,
            o.downcast_ref::<Funhouse>().context("Funhouse art owner")?,
        )?))
    }),
    owns_submodel: |m, e| m == "funhouse" && motion::owns(e),
    owns_npc: |n, _| {
        matches!(
            n,
            "dee_actor1" | "dum_actor1" | "fake_dum" | "hatter_actor1" | "cat_actor1" | "jack_cat"
        )
    },
    target_base: Some(BASE),
    story_beats: &[
        Beat::linear(
            "funhouse",
            "Funhouse_Start",
            "funhouse_cinematics",
            "Funhouse_Start",
            1,
        ),
        Beat::linear(
            "funhouse",
            "Cat_Jack_Dialog",
            "funhouse_cinematics",
            "Cat_Jack_Dialog",
            1,
        ),
        Beat::linear(
            "funhouse",
            "Funhouse_Tweedle_Cinema1",
            "funhouse_cinematics",
            "Funhouse_Tweedle_Cinema1",
            9,
        ),
        Beat::linear(
            "funhouse",
            "Funhouse_Hatter_Cinema1",
            "funhouse_cinematics",
            "Funhouse_Hatter_Cinema1",
            8,
        ),
    ],
    checks: &[
        Check {
            flag: "--funhouse-route-check",
            help: "Walk the Normal Mirror Image route with native enemies.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--funhouse-skip-route-check",
            help: "Walk Mirror Image with scene skipping.",
            run: Run::Windowed(route::skipped),
        },
        Check {
            flag: "--funhouse-traversal-probe",
            help: "Diagnose Funhouse movement without native combat.",
            run: Run::Headless(route::probe),
        },
        Check {
            flag: "--funhouse-check",
            help: "Verify Mirror Image machinery, clocks, bosses and saved scenes.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--funhouse-render-check",
            help: "Render Mirror Image machinery and scenes.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

pub(crate) use route::drive as drive_route;

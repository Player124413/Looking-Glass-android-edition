//! Burning Curiosity: Gryphon arrival, lava crossing and Caterpillar revelation.
mod art;
mod check;
mod route;
mod saves;
mod scene;
use super::{state, Beat, Check, Registration, Run};
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::Camera,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{self, Events},
    inventory::Stats,
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
const ARRIVAL: &str = "JLair1_Start";
const CATERPILLAR: &str = "JLair1_Caterpillar_Cinema1";
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    initialized: bool,
    arrived: bool,
    caterpillar: bool,
    sink: f32,
    #[serde(default)]
    essence_live: bool,
    #[serde(default)]
    essence_taken: [bool; 2],
    scene: Option<Scene>,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            initialized: false,
            arrived: false,
            caterpillar: false,
            sink: 0.,
            essence_live: false,
            essence_taken: [false; 2],
            scene: Some(Scene::new(Kind::Arrival)),
        }
    }
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        state::clock("lava platform", self.sink, 128.)?;
        ensure!(
            !self.caterpillar || self.arrived,
            "Caterpillar before arrival"
        );
        ensure!(
            !self.essence_taken.iter().any(|b| *b) || self.essence_live,
            "Essence collected before spawn"
        );
        if let Some(s) = &self.scene {
            s.validate()?;
            ensure!(
                match s.kind {
                    Kind::Arrival => !self.arrived && !self.caterpillar,
                    Kind::Caterpillar => self.arrived && !self.caterpillar,
                },
                "Scene conflicts with completed state"
            );
        } else {
            ensure!(self.arrived, "Missing arrival scene");
        }
        Ok(())
    }
}
struct Curiosity {
    saved: Saved,
    points: BTreeMap<String, Transform>,
    paths: BTreeMap<String, crate::fortress::spline::Spline>,
    sink_base: Vec3,
    sink_shape: Collider,
    essence: [Vec3; 2],
}
fn pose(e: &super::Entity) -> Transform {
    let angles = e
        .get("angles")
        .and_then(|s| interaction::vector(s))
        .unwrap_or_else(|| {
            vec3(
                0.,
                e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
                0.,
            )
        });
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| interaction::vector(s))
            .unwrap_or(Vec3::ZERO),
        rotation: Quat::from_rotation_z(angles.y.to_radians())
            * Quat::from_rotation_y(angles.x.to_radians())
            * Quat::from_rotation_x(angles.z.to_radians()),
    }
}
impl Curiosity {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        ensure!(
            map.entities
                .get(770)
                .and_then(|e| e.get("classname"))
                .is_some_and(|n| n == "func_sinkobject")
                && map.entities[770]["model"] == "*40",
            "Unexpected Burning Curiosity platform layout"
        );
        let mut points: BTreeMap<_, _> = map
            .entities
            .iter()
            .filter_map(|e| Some((e.get("targetname")?.clone(), pose(e))))
            .collect();
        for n in [
            "alice_pos1",
            "alice_cater_pos1",
            "gryphon_posx1",
            "caterpillar_actor1",
            "cat_cater_pos1",
        ] {
            ensure!(points.contains_key(n), "Missing scene point {n}");
        }
        // Editor markers are above the floor. The original grounded these actors
        // after warping; use the same checked floor for the puppet and handoff.
        let ground = World::from_bsp(map)?;
        for n in [
            "alice_pos1",
            "gryphon_posx1",
            "alice_cater_pos1",
            "cat_cater_pos1",
        ] {
            let at = points.get_mut(n).unwrap();
            at.translation = ground
                .actor_footing(
                    at.translation + Vec3::Z * 8.,
                    PLAYER_CENTER,
                    PLAYER_HALF,
                    128.,
                )
                .with_context(|| format!("Missing scene support {n}"))?;
        }
        let mut paths = BTreeMap::new();
        for n in [
            "jlair1_path1",
            "jlair1_path2",
            "jlair1_path3",
            "jlair1_path4",
            "jlair1_path5",
            "jlair1_path6",
            "jlair1_jdm1",
            "jlair1_jdm2",
            "jlair1_jdm4",
            "jlair1_jdm5",
            "jlair1_jdm6",
        ] {
            paths.insert(
                n.into(),
                crate::fortress::spline::Spline::camera_track(
                    crate::cinematic::Track::load(a, n)?.controls().collect(),
                ),
            );
        }
        for start in ["gryphon_path1", "gryphon_leave1"] {
            let mut name = start;
            let mut nodes = vec![];
            for _ in 0..32 {
                let e = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|n| n == name))
                    .context("Missing Gryphon path node")?;
                let p = pose(e);
                nodes.push((
                    p.translation,
                    p.rotation,
                    e.get("speed").and_then(|s| s.parse().ok()).unwrap_or(1.),
                ));
                let Some(next) = e.get("target") else {
                    break;
                };
                name = next;
            }
            ensure!(nodes.len() >= 4 && nodes.len() < 32, "Invalid Gryphon path");
            paths.insert(
                start.into(),
                crate::fortress::spline::Spline::new(nodes, false),
            );
        }
        let sink_base = pose(&map.entities[770]).translation;
        Ok(Self {
            saved: Saved::default(),
            points,
            paths,
            sink_base,
            essence: [
                pose(&map.entities[4]).translation,
                pose(&map.entities[908]).translation,
            ],
            sink_shape: Collider::model(map, 40, sink_base, Quat::IDENTITY, true)?,
        })
    }
    fn point(&self, name: &str) -> Transform {
        self.points[name]
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        self.sink_shape = Collider::model(
            map,
            40,
            self.sink_base - Vec3::Z * self.saved.sink,
            Quat::IDENTITY,
            true,
        )?;
        Ok(())
    }
    fn move_sink(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if self.scripted() {
            return Ok(());
        }
        let tr = self.sink_shape.trace(
            p.feet + PLAYER_CENTER,
            p.feet + PLAYER_CENTER - Vec3::Z * 3.,
            PLAYER_HALF,
        );
        let rider = p.velocity.z <= 1. && !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.65;
        let old = self.saved.sink;
        self.saved.sink = (old + if rider { dt * 100. } else { -dt * 100. }).clamp(0., 128.);
        if old == self.saved.sink {
            return Ok(());
        }
        self.rebuild(map)?;
        let mut carry = if rider {
            Vec3::Z * (old - self.saved.sink)
        } else {
            Vec3::ZERO
        };
        w.set_dynamic(fixed.to_vec());
        let trace = w.body_trace(p.feet, p.feet + carry);
        // The pool has an uneven bed. Transfer a descending rider onto that floor
        // instead of stopping the entire platform or pulling Alice through it.
        if carry.z < 0. && !trace.start_solid {
            carry *= trace.fraction;
        }
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        if trace.start_solid
            || (carry.z > 0. && trace.fraction < 1.)
            || !w.body_clear(p.feet + carry)
        {
            self.saved.sink = old;
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        } else {
            p.feet += carry;
        }
        Ok(())
    }
}
impl LevelController for Curiosity {
    fn id(&self) -> &'static str {
        "jlair1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag(
            "jlair1.free",
            self.saved.arrived && self.saved.scene.is_none(),
        );
        f.flag("jlair1.caterpillar", !self.saved.caterpillar);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        match t.id.0 {
            26 => Some(Condition::All(vec![
                Condition::flag("jlair1.free"),
                Condition::flag("jlair1.caterpillar"),
            ])),
            106 => Some(Condition::flag("jlair1.free")),
            _ => None,
        }
    }
    fn event(&mut self, name: &str) -> Option<Events> {
        if name != CATERPILLAR {
            return None;
        }
        if self.saved.arrived && !self.saved.caterpillar && self.saved.scene.is_none() {
            self.saved.scene = Some(Scene::new(Kind::Caterpillar));
        }
        Some(Events::default())
    }
    fn receivers(&self, _: &crate::entity::Registry) -> Vec<crate::entity::Id> {
        vec![crate::entity::Id(4), crate::entity::Id(908)]
    }
    fn rules(&self, _: &crate::level::RuleContext<'_>) -> Vec<crate::event::Rule> {
        use crate::entity::Id;
        use crate::event::{Action, Effect, Event, Input, Rule};
        [4, 908]
            .into_iter()
            .map(|id| Rule {
                key: format!("jlair1/essence/{id}"),
                event: Event::Entity(Id(id), Input::Activate),
                condition: Condition::Always,
                once: true,
                cooldown: 0.,
                actions: vec![Action::Output(Effect::Activate(Id(id)))],
            })
            .collect()
    }
    fn output(&mut self, e: &crate::event::Effect) -> Option<Events> {
        if matches!(
            e,
            crate::event::Effect::Activate(crate::entity::Id(4 | 908))
        ) {
            self.saved.essence_live = true;
            Some(Events::default())
        } else {
            None
        }
    }
    fn prepare_player(&mut self, stats: &mut Stats, p: &mut Player) {
        if !self.saved.initialized {
            stats.full_stats();
            self.saved.initialized = true;
        }
        if self.saved.essence_live && !self.scripted() {
            for (i, at) in self.essence.iter().enumerate() {
                if !self.saved.essence_taken[i]
                    && (p.feet + PLAYER_CENTER).distance(*at) < 42.
                    && stats.apply(crate::inventory::PickupKind::Essence, 15.)
                {
                    self.saved.essence_taken[i] = true;
                }
            }
        }
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
        self.move_sink(dt, map, w, p, fixed)?;
        self.advance_scene(dt, w, p)
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        vec![(
            40,
            self.sink_base - Vec3::Z * self.saved.sink,
            Quat::IDENTITY,
        )]
    }
    fn colliders(&self) -> Vec<Collider> {
        vec![self.sink_shape.clone()]
    }
    fn sky_origin(&self) -> Option<Vec3> {
        Some(self.point("skycamera01").translation)
    }
    fn scene_fog(&self) -> Option<Vec4> {
        Some(vec4(0.25, 0.125, 0.1, 5000.))
    }
    fn scripted(&self) -> bool {
        self.saved.scene.is_some()
    }
    fn controlled(&self) -> bool {
        self.scripted()
    }
    fn hides_player(&self) -> bool {
        self.scripted()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved.scene.as_ref().map(|s| s.kind.id())
    }
    fn entry_story(&mut self, _: &mut Story) -> bool {
        true
    }
    fn prepare_story(&self, story: &mut Story) -> bool {
        self.prepare_scene_story(story)
    }
    fn sync_story(&mut self, story: &Story) {
        self.sync_scene_story(story);
    }
    fn dialogue_complete(&mut self, name: &str) -> Events {
        if let Some(s) = &mut self.saved.scene {
            if s.kind.id() == name {
                s.ending.get_or_insert(0.);
            }
        }
        Events::default()
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, story: &mut Story) -> Result<bool> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(false);
        };
        story.finish_sequence(s.kind.id());
        s.skip.get_or_insert(0.);
        Ok(true)
    }
    fn camera(&self, _: &World) -> Option<Camera> {
        self.scene_camera()
    }
    fn scene_fovy(&self, aspect: f32) -> Option<f32> {
        self.fovy(aspect)
    }
    fn fade(&self) -> Option<(Color, f32)> {
        self.scene_fade()
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.saved.caterpillar {
                "Continue through the caverns to the Jabberwock's lair."
            } else {
                "Cross the lava caverns and find the Caterpillar."
            }
            .into(),
        )
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let saved = state::load(v, state::Visit { returning: false })?;
        self.saved = saved;
        self.rebuild(map)
    }
    fn upgraded(&mut self) {
        self.saved.initialized = true;
        self.saved.arrived = true;
        self.saved.scene = None;
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            rearm: vec![CATERPILLAR.into()],
            ..Default::default()
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
pub static REGISTRATION: Registration = Registration {
    id: "jlair1",
    applies: |m, e| super::first_visit(m, e, "jlair1"),
    load: |a, m, _, _| Ok(Box::new(Curiosity::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| {
        m == "jlair1"
            && (e.get("classname").is_some_and(|s| s == "func_sinkobject")
                || e.get("targetname").is_some_and(|s| s == "skycamera01"))
    },
    owns_npc: |n, _| {
        matches!(
            n,
            "gryphon_actor1" | "gryphon_actor2" | "cat_actor1" | "caterpillar_actor1"
        )
    },
    target_base: Some(8_500_000),
    story_beats: &[
        Beat::linear("jlair1", ARRIVAL, "jlair1_cinematics", ARRIVAL, 1),
        Beat::linear("jlair1", CATERPILLAR, "jlair1_cinematics", CATERPILLAR, 15),
    ],
    checks: &[
        Check {
            flag: "--jlair1-traversal-check",
            help: "Replay the Burning Curiosity physical route without the cast.",
            run: Run::Headless(route::traversal),
        },
        Check {
            flag: "--jlair1-check",
            help: "Check arrival, revelation, lava platform and persistence.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--jlair1-route-check",
            help: "Play Burning Curiosity into Jabberwock survival with the native cast.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--jlair1-render-check",
            help: "Capture Gryphon flight, revelation and lava platform.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

pub(crate) use route::drive as drive_route;

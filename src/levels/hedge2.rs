//! Mystifying Madness: two one-use levers and their persistent door choreography.
mod art;
mod check;
mod route;
mod saves;
use super::{state, Check, Registration, Run};
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
use serde::{Deserialize, Serialize};
use std::any::Any;
const ALIGN_TIME: f32 = 0.2;
const LEVERS: [usize; 2] = [507, 450];
const THREADS: [&str; 2] = ["OpenRobodoors", "OpenEnd"];
const OBJECTS: [usize; 6] = [11, 32, 33, 402, 53, 54];
#[derive(Clone, Serialize, Deserialize)]
struct Pull {
    index: usize,
    time: f32,
    pose: Transform,
}
#[derive(Clone, Serialize, Deserialize)]
struct Scene {
    index: usize,
    time: f32,
}
#[derive(Clone, Serialize, Deserialize)]
struct Saved {
    version: u8,
    used: [bool; 2],
    fired: [bool; 2],
    finished: [bool; 2],
    pull: Option<Pull>,
    scene: Option<Scene>,
    release: Option<f32>,
    // Trapdoor, robot leaves, water gate, end leaves: each is a physical pose.
    motion: [f32; 6],
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            version: 1,
            used: [false; 2],
            fired: [false; 2],
            finished: [false; 2],
            pull: None,
            scene: None,
            release: None,
            motion: std::array::from_fn(|k| if k == 0 { 1. } else { 0. }),
        }
    }
}
impl state::State for Saved {
    const VERSION: u8 = 1;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, _: state::Visit) -> Result<()> {
        for k in 0..2 {
            ensure!(!self.fired[k] || self.used[k], "Lever event without use");
            ensure!(
                !self.finished[k] || self.fired[k],
                "Cutaway completed before lever"
            );
            ensure!(
                !self.used[k] || self.fired[k] || self.pull.as_ref().is_some_and(|p| p.index == k),
                "Missing lever animation"
            );
            ensure!(
                !self.fired[k]
                    || self.finished[k]
                    || self.scene.as_ref().is_some_and(|p| p.index == k),
                "Missing gate cutaway"
            );
        }
        ensure!(
            self.pull.is_none() || self.scene.is_none(),
            "Overlapping lever and cutaway"
        );
        if let Some(p) = &self.pull {
            ensure!(
                p.index < 2 && self.used[p.index] && !self.fired[p.index],
                "Invalid lever pull"
            );
            state::clock("lever", p.time, 10.)?;
            ensure!(
                p.pose.translation.is_finite()
                    && p.pose.rotation.is_finite()
                    && (p.pose.rotation.length() - 1.).abs() < 0.001,
                "Invalid lever pose"
            );
        }
        if let Some(s) = &self.scene {
            ensure!(
                s.index < 2 && self.fired[s.index] && !self.finished[s.index],
                "Invalid gate scene"
            );
            state::clock("cutaway", s.time, if s.index == 0 { 11. } else { 2. })?;
        }
        for m in self.motion {
            state::fraction("gate travel", m)?;
        }
        if let Some(t) = self.release {
            state::clock("robot release", t, 0.1)?;
            ensure!(self.finished[0], "Robot doors before cutaway");
        }
        ensure!(
            self.finished[0] == self.release.is_some(),
            "Missing robot release"
        );
        ensure!(
            self.fired[0] || self.motion == [1., 0., 0., 0., self.motion[4], self.motion[5]],
            "Tunnel changed before lever"
        );
        ensure!(
            self.fired[1] || (self.motion[4] == 0. && self.motion[5] == 0.),
            "Exit opened before lever"
        );
        Ok(())
    }
}
struct Object {
    model: usize,
    base: Vec3,
    delta: Vec3,
    duration: f32,
    collider: Collider,
}
struct Maze {
    saved: Saved,
    objects: Vec<Object>,
    levers: [Transform; 2],
    cameras: Vec<crate::fortress::spline::Spline>,
    pull_duration: f32,
    pull_sound: f32,
    bubbles: Vec<Vec3>,
    bubble_bounds: (Vec3, Vec3),
}
fn at(e: &super::Entity) -> Transform {
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| interaction::vector(s))
            .unwrap_or(Vec3::ZERO),
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    }
}
impl Maze {
    fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        ensure!(
            map.entities[507]
                .get("stop_thread")
                .is_some_and(|s| s == THREADS[0])
                && map.entities[450]
                    .get("move_thread")
                    .is_some_and(|s| s == THREADS[1]),
            "Unexpected hedge2 levers"
        );
        let objects = OBJECTS
            .into_iter()
            .enumerate()
            .map(|(k, id)| -> Result<Object> {
                let e = &map.entities[id];
                let model = e["model"].trim_start_matches('*').parse::<usize>()?;
                let base = at(e).translation;
                let delta = match k {
                    0 => Vec3::Z * 184.,
                    1 | 2 => Vec3::Z * 144.,
                    3 => Vec3::Z * 168.,
                    4 => Vec3::X * 40.,
                    _ => -Vec3::X * 40.,
                };
                Ok(Object {
                    model,
                    base,
                    delta,
                    duration: if k < 4 { 2. } else { 0.4 },
                    collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut cameras = vec![];
        for n in ["hedge2_tunnelcam", "hedge2_endgatecam"] {
            cameras.push(crate::fortress::spline::Spline::camera_track(
                crate::cinematic::Track::load(a, n)?.controls().collect(),
            ));
        }
        let lever = crate::tan::Model::parse(&a.read("models/lever1/pull.tan")?)?;
        let bubble = crate::tan::Model::parse(&a.read("models/fx/fx_boojum_scream/scream.tan")?)?;
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = -lo;
        for p in bubble.surfaces.iter().flat_map(|s| &s.frames).flatten() {
            lo = lo.min(*p * 5.);
            hi = hi.max(*p * 5.);
        }
        ensure!(
            lo.is_finite() && hi.is_finite() && lo.cmple(hi).all(),
            "Invalid air bubble bounds"
        );
        let bubbles = map
            .entities
            .iter()
            .filter(|e| e.get("model").is_some_and(|m| m == "fx_bubbles_air.tik"))
            .map(|e| at(e).translation)
            .collect();
        let mut o = Self {
            bubbles,
            bubble_bounds: (lo, hi),
            saved: Saved::default(),
            objects,
            levers: LEVERS.map(|id| at(&map.entities[id])),
            cameras,
            pull_duration: lever.frame_time * lever.surfaces[0].frames.len() as f32,
            pull_sound: 20. * lever.frame_time,
        };
        o.rebuild(map)?;
        Ok(o)
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        for (k, o) in self.objects.iter_mut().enumerate() {
            o.collider = Collider::model(
                map,
                o.model,
                o.base + o.delta * self.saved.motion[k],
                Quat::IDENTITY,
                true,
            )?;
        }
        Ok(())
    }
    fn pick(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        if self.scripted() {
            return None;
        }
        self.levers.iter().enumerate().find_map(|(k, p)| {
            let target = p.point(vec3(36., 0., 26.));
            let d = target - eye;
            (!self.saved.used[k]
                && d.length() < 120.
                && d.normalize_or_zero().dot(aim.normalize_or_zero()) > 0.35
                && w.sweep(eye, target, Vec3::splat(0.5)).fraction > 0.96)
                .then_some(k)
        })
    }
    fn aligned(&self, k: usize) -> Vec3 {
        self.levers[k].point(vec3(-2.65, 0., 0.2))
    }
    fn pull_pose(&self, p: &Pull) -> Transform {
        Transform {
            translation: p
                .pose
                .translation
                .lerp(self.aligned(p.index), (p.time / ALIGN_TIME).min(1.)),
            rotation: p.pose.rotation,
        }
    }
    fn fire(&mut self, k: usize) {
        if self.saved.fired[k] {
            return;
        }
        self.saved.used[k] = true;
        self.saved.fired[k] = true;
        self.saved.pull = None;
        self.saved.scene = Some(Scene { index: k, time: 0. });
    }
    fn finish(&mut self, k: usize) {
        self.saved.finished[k] = true;
        self.saved.scene = None;
        if k == 0 {
            self.saved.release = Some(0.);
        }
    }
    fn ready(&self) -> bool {
        self.saved.finished == [true, true]
            && self.saved.motion[4] >= 1.
            && self.saved.motion[5] >= 1.
            && !self.scripted()
    }
}
impl LevelController for Maze {
    fn id(&self) -> &'static str {
        "hedge2"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("hedge2.exit", self.ready());
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        (t.id.0 == 52).then(|| Condition::flag("hedge2.exit"))
    }
    fn event(&mut self, n: &str) -> Option<Events> {
        let k = THREADS.iter().position(|s| *s == n)?;
        if !self.scripted() {
            self.fire(k);
        }
        Some(Events::default())
    }
    fn prepare_player(&mut self, s: &mut Stats, p: &mut Player) {
        s.turtle_air = true;
        p.breath.shell = true;
        // Stationary emitters renew overlapping four-second breath objects every two seconds.
        // Collision follows the hidden breath model, not the visible particle cloud.
        let c = p.feet + PLAYER_CENTER;
        let (lo, hi) = self.bubble_bounds;
        if self.bubbles.iter().any(|b| {
            (*b + lo).cmple(c + PLAYER_HALF).all() && (*b + hi).cmpge(c - PLAYER_HALF).all()
        }) {
            p.breath.refill();
        }
    }
    fn active_npcs(&self) -> Vec<usize> {
        if self.saved.fired[0] {
            vec![34, 567]
        } else {
            vec![]
        }
    }
    fn update(&mut self, w: &mut World, p: &Player, aim: Vec3, pressed: bool) -> Events {
        if pressed {
            if let Some(k) = self.pick(w, p.eye(), aim) {
                if w.body_trace(p.feet, self.aligned(k)).fraction < 1.
                    || !w.body_clear(self.aligned(k))
                {
                    return Events::default();
                }
                self.saved.used[k] = true;
                self.saved.pull = Some(Pull {
                    index: k,
                    time: 0.,
                    pose: Transform {
                        translation: p.feet,
                        rotation: self.levers[k].rotation,
                    },
                });
            }
        }
        Events::default()
    }
    fn prompt(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        self.pick(w, eye, aim).map(|_| "E  Pull lever")
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
        if let Some(pull) = &mut self.saved.pull {
            pull.time += dt;
            let anchor = self.levers[pull.index].point(vec3(-2.65, 0., 0.2));
            p.feet = pull
                .pose
                .translation
                .lerp(anchor, (pull.time / ALIGN_TIME).min(1.));
            p.velocity = Vec3::ZERO;
            p.script_facing = {
                let d = pull.pose.rotation * Vec3::X;
                d.y.atan2(d.x)
            };
            if pull.time >= ALIGN_TIME + self.pull_duration {
                let k = pull.index;
                self.fire(k);
            }
        } else if let Some(s) = &mut self.saved.scene {
            s.time += dt;
            p.velocity = Vec3::ZERO;
            if s.time >= if s.index == 0 { 11. } else { 2. } {
                let k = s.index;
                self.finish(k);
            }
        }
        if let Some(t) = &mut self.saved.release {
            *t = (*t + dt).min(0.1);
        }
        let tunnel = self.saved.finished[0]
            || self
                .saved
                .scene
                .as_ref()
                .is_some_and(|s| s.index == 0 && s.time >= 8.);
        let end = self.saved.finished[1]
            || self
                .saved
                .scene
                .as_ref()
                .is_some_and(|s| s.index == 1 && s.time >= 1.);
        let release = self.saved.release == Some(0.1);
        let goals = [!release, release, release, tunnel, end, end];
        // Rebuild each attempted physical step and reject a closing leaf that overlaps Alice.
        // Cutaway clocks keep running even if a player blocks the returning trapdoor.
        for k in 0..6 {
            let old = self.saved.motion[k];
            let target = if goals[k] { 1. } else { 0. };
            let step = dt / self.objects[k].duration;
            self.saved.motion[k] = if old < target {
                (old + step).min(target)
            } else {
                (old - step).max(target)
            };
            if old == self.saved.motion[k] {
                continue;
            }
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            if !w.body_clear(p.feet) {
                self.saved.motion[k] = old;
                self.rebuild(map)?;
            }
        }
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .enumerate()
            .map(|(k, o)| {
                (
                    o.model,
                    o.base + o.delta * self.saved.motion[k],
                    Quat::IDENTITY,
                )
            })
            .collect()
    }
    fn colliders(&self) -> Vec<Collider> {
        self.objects.iter().map(|o| o.collider.clone()).collect()
    }
    fn scripted(&self) -> bool {
        self.saved.pull.is_some() || self.saved.scene.is_some()
    }
    fn hides_player(&self) -> bool {
        self.saved.pull.is_some()
    }
    fn scene_id(&self) -> Option<&'static str> {
        self.saved.scene.as_ref().map(|s| THREADS[s.index])
    }
    fn camera(&self, _: &World) -> Option<Camera> {
        let s = self.saved.scene.as_ref()?;
        let at = self.cameras[s.index].sample(s.time, false);
        Some(Camera {
            eye: at.translation,
            target: at.point(Vec3::X),
            up: at.rotation * Vec3::Z,
        })
    }
    fn skip(&mut self, map: &Bsp, w: &mut World, _: &mut Player, _: &mut Story) -> Result<bool> {
        let Some(s) = &self.saved.scene else {
            return Ok(false);
        };
        let k = s.index;
        self.finish(k);
        // Opening gates is safe; closing machinery still takes physical steps after control returns.
        if k == 0 {
            self.saved.motion[3] = 1.;
        } else {
            self.saved.motion[4] = 1.;
            self.saved.motion[5] = 1.;
        }
        self.rebuild(map)?;
        let _ = w;
        Ok(true)
    }
    fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        for (k, key) in [
            (3, "hedge2/water-gate"),
            (4, "hedge2/end-right"),
            (5, "hedge2/end-left"),
        ] {
            let progress = self.saved.motion[k];
            let origin = self.objects[k].base + self.objects[k].delta * progress;
            if progress > 0. && progress < 1. {
                loops.push(crate::audio::LoopCue {
                    id: 9_000_000 + k,
                    path: "sound/ambience/special/slow scrape.wav",
                    origin,
                    clock: Some(progress * self.objects[k].duration),
                });
            }
            clocks.push(crate::audio::world::Clock {
                key,
                time: progress,
                period: None,
                origin,
                cues: &[(1., "sound/world/mover/spike.wav")],
            });
        }
        if let Some(p) = &self.saved.pull {
            clocks.push(crate::audio::world::Clock {
                key: if p.index == 0 {
                    "hedge2/water-lever"
                } else {
                    "hedge2/end-lever"
                },
                time: (p.time - ALIGN_TIME).max(0.) * 0.8 / self.pull_sound,
                period: None,
                origin: self.levers[p.index].translation,
                cues: &[(0.8, "sound/world/machine/lever1.wav")],
            });
        }
    }
    fn objective(&self) -> Option<String> {
        Some(
            if self.ready() {
                "Enter the tower."
            } else if self.saved.fired[0] {
                "Swim through the opened tunnel and find the last lever."
            } else {
                "Find the lever below the maze."
            }
            .into(),
        )
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.saved)
    }
    fn restore(&mut self, v: &serde_json::Value, map: &Bsp) -> Result<()> {
        let s = state::load::<Saved>(v, state::Visit { returning: false })?;
        if let Some(p) = &s.pull {
            ensure!(
                p.time <= ALIGN_TIME + self.pull_duration,
                "Lever clock exceeds animation"
            );
        }
        self.saved = s;
        self.rebuild(map)
    }
    fn upgrade(&self) -> crate::level::Upgrade {
        crate::level::Upgrade {
            respawn: crate::level::Respawn::Always,
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
    id: "hedge2",
    applies: |m, e| super::first_visit(m, e, "hedge2"),
    load: |a, m, _, _| Ok(Box::new(Maze::load(a, m)?)),
    art: Some(|a, _, _| Ok(Box::new(art::Art::load(a)?))),
    owns_submodel: |m, e| {
        m == "hedge2"
            && e.get("targetname").is_some_and(|s| {
                matches!(
                    s.as_str(),
                    "trapdoor" | "robodoor1" | "robodoor2" | "water_door" | "end_doors"
                )
            })
    },
    owns_npc: |_, m| m == "lever",
    target_base: Some(9_000_000),
    story_beats: &[],
    checks: &[
        Check {
            flag: "--hedge2-check",
            help: "Check hedge levers, movers, cutaways and saved gates.",
            run: Run::Headless(check::check),
        },
        Check {
            flag: "--hedge2-route-check",
            help: "Play both maze lever sequences and the swimming route.",
            run: Run::Windowed(route::check),
        },
        Check {
            flag: "--hedge2-render-check",
            help: "Capture hedge levers and gate cutaways.",
            run: Run::Windowed(check::render),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

#[cfg(test)]
mod tests {
    use super::*;
    use state::State;
    fn valid(s: &Saved) -> bool {
        s.validate(state::Visit { returning: false }).is_ok()
    }
    #[test]
    fn initial_state_is_closed_except_room_entrance() {
        let s = Saved::default();
        assert!(valid(&s));
        assert_eq!(s.motion[0], 1.);
        assert!(s.motion[1..].iter().all(|v| *v == 0.));
    }
    #[test]
    fn missing_callbacks_are_rejected() {
        let mut s = Saved::default();
        s.used[0] = true;
        assert!(!valid(&s));
        s.fired[0] = true;
        assert!(!valid(&s));
        s.scene = Some(Scene { index: 0, time: 9. });
        assert!(valid(&s));
        s.finished[0] = true;
        assert!(!valid(&s));
        s.scene = None;
        s.release = Some(0.);
        assert!(valid(&s));
    }
    #[test]
    fn bad_clocks_and_unearned_gates_are_rejected() {
        let mut s = Saved::default();
        s.motion[3] = 1.;
        assert!(!valid(&s));
        s.motion[3] = 0.;
        s.motion[4] = 1.;
        assert!(!valid(&s));
        s.motion[4] = f32::NAN;
        assert!(!valid(&s));
    }
}

pub(crate) use route::drive as drive_route;

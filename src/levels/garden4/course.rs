//! Frozen approach: shared rock simulation and independently scheduled BSP movers.
use super::*;
use crate::falling_rock::{Spec, State as Rock};
pub(super) const ROCKS: [&str; 7] = [
    "boulder1",
    "boulder2",
    "boulder3",
    "boulder4",
    "boulder_end1",
    "boulder_end2",
    "ice_marble1",
];
pub(super) const MARBLE: &str = "Garden4_Marble";
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Course {
    pub age: f64,
    pub rocks: Vec<Rock>,
    pub crushed: [Option<f64>; 4],
    pub altar: Option<f64>,
    pub wall: Option<f64>,
    pub fog: bool,
    pub marble: Option<f64>,
    pub marble_done: bool,
    pub previous: Option<Vec3>,
    pub damage: f32,
    pub impulse: Vec3,
    pub contacts: u32,
    pub bounce: Option<f64>,
}
impl Course {
    pub fn new(specs: &[Spec]) -> Self {
        Self {
            age: 0.,
            rocks: specs
                .iter()
                .enumerate()
                .map(|(i, s)| Rock::new(s, i != 6, false))
                .collect(),
            crushed: [None; 4],
            altar: None,
            wall: None,
            fog: false,
            marble: None,
            marble_done: false,
            previous: None,
            damage: 0.,
            impulse: Vec3::ZERO,
            contacts: 0,
            bounce: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (0. ..=86400.).contains(&self.age) && self.rocks.len() == 7,
            "Invalid ice course clock/rocks"
        );
        for t in self
            .crushed
            .iter()
            .copied()
            .chain([self.altar, self.wall, self.marble, self.bounce])
            .flatten()
        {
            ensure!((0. ..=self.age).contains(&t), "Invalid ice event clock");
        }
        ensure!(
            !self.marble_done || self.marble.is_some(),
            "Marble handoff before scene"
        );
        ensure!(
            self.previous
                .is_none_or(|v| v.is_finite() && v.abs().max_element() < 100000.)
                && self.impulse.is_finite()
                && (0. ..=1000.).contains(&self.damage),
            "Invalid course contact"
        );
        Ok(())
    }
}
pub(super) struct Object {
    pub model: usize,
    pub name: String,
    pub base: Transform,
    pub pose: Option<Transform>,
    pub solid: bool,
    pub collider: Collider,
}
pub(super) fn owns(e: &crate::levels::Entity) -> bool {
    e.get("classname").is_some_and(|c| c == "script_object")
}
pub(super) fn objects(map: &Bsp) -> Result<Vec<Object>> {
    map.entities
        .iter()
        .filter(|e| owns(e) && e.get("targetname").is_none_or(|n| n != "portal_object"))
        .map(|e| {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = data::at(e);
            Ok(Object {
                model,
                name: e.get("targetname").cloned().unwrap_or_default(),
                base,
                pose: None,
                solid: false,
                collider: Collider::model(map, model, base.translation, base.rotation, true)?,
            })
        })
        .collect()
}
impl Garden {
    pub(super) fn course(&self) -> &Course {
        self.saved.course.as_ref().unwrap()
    }
    pub(super) fn course_mut(&mut self) -> &mut Course {
        self.saved.course.as_mut().unwrap()
    }
    fn activate_rock(&mut self, k: usize) {
        let s = &mut self.saved.course.as_mut().unwrap().rocks[k];
        self.data.rocks[k].activate(s);
        if matches!(k, 2 | 3 | 4) {
            s.solid = false;
        }
    }
    pub(super) fn course_event(&mut self, n: &str) -> bool {
        match n {
            "Ice_Fall1" if !self.data.easy => self.activate_rock(0),
            "Ice_Fall2" if !self.data.easy => self.activate_rock(1),
            "Ice_Fall1" | "Ice_Fall2" => {}
            "Ice_Fall3" => {
                if self.course().altar.is_none() {
                    self.course_mut().altar = Some(self.course().age);
                    self.activate_rock(2);
                }
            }
            "Garden4_Boulder_End" => self.activate_rock(4),
            "Garden4_Boulder_End2" => self.activate_rock(5),
            "Garden4_Fade_Fog" => self.course_mut().fog = true,
            MARBLE => {
                if self.course().marble.is_none() && self.saved.scene.is_none() {
                    self.course_mut().marble = Some(self.course().age);
                }
            }
            _ => return false,
        }
        true
    }
    pub(super) fn marble_time(&self) -> Option<f32> {
        (!self.course().marble_done)
            .then(|| self.course().marble.map(|t| (self.course().age - t) as f32))
            .flatten()
    }
    pub(super) fn course_step(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if self.saved.scene.is_some() {
            return Ok(());
        }
        // The native Easy-only trigger is at the altar; the same pickup-sized volume
        // handles its contact, independently of the inhibited Normal/Hard BSP trigger.
        let before = self.course().previous.unwrap_or(p.feet);
        if self.data.easy
            && self.course().altar.is_none()
            && self.data.easy_altar.touches(
                before + crate::collision::PLAYER_CENTER,
                p.feet + crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
            )
        {
            self.course_event("Ice_Fall3");
        }
        self.course_mut().previous = Some(p.feet);
        self.course_mut().age += f64::from(dt);
        if self
            .course()
            .altar
            .is_some_and(|t| self.course().age - t >= 1.)
        {
            self.activate_rock(3);
        }
        if self.marble_time().is_some_and(|t| t >= 0.5) {
            self.activate_rock(6);
        }
        let cinematic = self.marble_time().is_some();
        let body = crate::combat::Target {
            id: crate::dice::ALICE,
            center: p.feet + crate::collision::PLAYER_CENTER,
            half: crate::collision::PLAYER_HALF,
        };
        let mut callbacks = Vec::new();
        for k in 0..7 {
            let s = self.saved.course.as_mut().unwrap();
            if !s.rocks[k].visible {
                continue;
            }
            let out = self.data.rocks[k].advance(
                &mut s.rocks[k],
                dt,
                if cinematic {
                    &[]
                } else {
                    std::slice::from_ref(&body)
                },
            );
            if !out.arrivals.is_empty() {
                s.bounce = Some(s.age);
            }
            for (_, damage, impulse) in out.hits {
                s.damage = s.damage.max(damage);
                s.impulse = impulse;
                s.contacts += 1;
            }
            callbacks.extend(out.threads);
        }
        for n in callbacks {
            let s = self.course_mut();
            match n.as_str() {
                "Garden4_CrushIce1" => {
                    s.crushed[0].get_or_insert(s.age);
                }
                "Garden4_CrushIce2" => {
                    s.crushed[1].get_or_insert(s.age);
                }
                "Garden4_CrushIce3" => {
                    s.crushed[2].get_or_insert(s.age);
                }
                "Garden4_CrushIce4" => {
                    s.crushed[3].get_or_insert(s.age);
                }
                "Garden4_End_SmashWall" => {
                    s.wall.get_or_insert(s.age);
                }
                _ => anyhow::bail!("Unreviewed garden rock callback {n}"),
            }
        }
        for k in 0..4 {
            if self.course().crushed[k].is_some_and(|t| self.course().age - t >= 8.5) {
                let rock = &mut self.course_mut().rocks[k];
                rock.visible = false;
                rock.solid = false;
            }
        }
        if let Some(t) = self.marble_time() {
            p.cancel_climb();
            p.release_rope();
            p.velocity = Vec3::ZERO;
            p.script_motion = 1;
            if t >= 7. {
                self.course_mut().marble_done = true;
                self.rebuild(map)?;
                w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
                let mut landing = self.data.marble_end;
                landing.translation = w
                    .actor_footing(
                        landing.translation,
                        crate::collision::PLAYER_CENTER,
                        crate::collision::PLAYER_HALF,
                        160.,
                    )
                    .context("Marble handoff lacks support")?;
                ensure!(
                    w.body_clear(landing.translation)
                        && landing
                            .translation
                            .distance(self.course().rocks[6].position)
                            > 240.,
                    "Unsafe marble handoff"
                );
                let mut state = SceneState::new(&spec(Some(landing)));
                SceneRunner {
                    spec: &spec(Some(landing)),
                    state: &mut state,
                }
                .finish(w, p)?;
                self.course_mut().previous = Some(p.feet);
                self.saved.fade = 0.5;
            }
        }
        self.rebuild(map)
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let s = self.saved.course.as_ref().unwrap();
        for o in &mut self.objects {
            let mut pose = Some(o.base);
            let mut solid = true;
            if o.name == "block_back" && !s.fog {
                pose = None;
                solid = false;
            }
            if let Some(n) = o
                .name
                .strip_prefix("break_floor")
                .and_then(|n| n.parse::<usize>().ok())
            {
                let (k, delay, seconds, x, z) = match n {
                    1 => (0, 0., 6., -45., 55.),
                    2 => (0, 0., 6.4, -55., 45.),
                    3 => (1, 0., 6., -45., 55.),
                    4 => (1, 0., 6.4, -55., 45.),
                    5 => (1, 0.5, 6., -45., 55.),
                    8 => (2, 0., 6., -45., 55.),
                    7 => (2, 0., 6.4, -55., 45.),
                    6 => (3, 0., 6., -45., 55.),
                    9 => (3, 0., 6.4, -55., 45.),
                    _ => unreachable!(),
                };
                if let Some(at) = s.crushed[k] {
                    let f = (((s.age - at) as f32 - delay) / seconds).clamp(0., 1.);
                    pose = fall(o.base, f, 2400., x, z);
                }
                if n >= 6 && s.marble_done {
                    pose = None;
                }
            }
            if let Some(n) = o
                .name
                .strip_prefix("brokenwall")
                .and_then(|n| n.parse::<usize>().ok())
            {
                if let Some(at) = s.wall {
                    let (delay, seconds, drop, x, z) = [
                        (0., 0.5, 700., -20., 15.),
                        (0.1, 0.4, 700., -12., 17.),
                        (0.2, 1., 720., -24., 12.),
                        (0.4, 0.3, 700., -16., 15.),
                        (0.5, 0.4, 700., -19., 17.),
                        (0.7, 0.3, 700., -16., 21.),
                        (0.9, 0.2, 700., -23., 19.),
                        (1., 0.4, 700., -17., 24.),
                    ][n - 1];
                    let t = (s.age - at) as f32 - delay;
                    solid = t < 0.;
                    pose = fall(o.base, (t / seconds).clamp(0., 1.), drop, x, z);
                }
            }
            if let Some(pose) = pose {
                if o.pose.is_none_or(|p| {
                    p.translation != pose.translation || p.rotation != pose.rotation
                }) {
                    o.collider =
                        Collider::model(map, o.model, pose.translation, pose.rotation, true)?;
                }
            }
            o.pose = pose;
            o.solid = solid && pose.is_some();
        }
        Ok(())
    }
}
fn fall(mut p: Transform, f: f32, drop: f32, x: f32, z: f32) -> Option<Transform> {
    if f >= 1. {
        return None;
    }
    p.translation.z -= drop * f;
    p.rotation = p.rotation
        * Quat::from_rotation_z((z * f).to_radians())
        * Quat::from_rotation_x((x * f).to_radians());
    Some(p)
}

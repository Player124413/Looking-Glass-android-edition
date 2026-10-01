//! Fortress of Doors playable visits. Original archives remain read-only data.
pub mod check;
pub mod cinema;
pub(crate) mod spline;
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{vector, Events},
    movement::Player,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub fn supported(e: &BTreeMap<String, String>) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("script_object" | "func_fulcrum")
    ) && e.get("model").is_some_and(|s| s.starts_with('*'))
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    pub cinema: cinema::State,
    pub returning: bool,
    age: f32,
    pub splitting: bool,
    pub split: f32,
    pub shutters: bool,
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    solid: bool,
    collider: Collider,
}
pub struct Fortress {
    pub cinema: cinema::Data,
    pub state: State,
    objects: Vec<Object>,
}
fn rotation(pitch: f32, yaw: f32, roll: f32) -> Quat {
    Quat::from_rotation_z(yaw.to_radians())
        * Quat::from_rotation_y(pitch.to_radians())
        * Quat::from_rotation_x(roll.to_radians())
}
fn school(s: &State) -> Quat {
    rotation(
        3.,
        0.,
        6. + 3. * (s.age * std::f32::consts::TAU / 12.).sin(),
    )
}
fn pose(name: &str, base: Vec3, s: &State) -> (Vec3, Quat) {
    let mut p = base;
    let mut r = Quat::IDENTITY;
    match name {
        "split_byebye" => p.y += 400. * s.split,
        "split_middle" => p.y += 220. * s.split,
        "split_fulc" => {
            p.y += 216. * s.split;
            r = rotation(-s.split, -6. * s.split, -3. * s.split);
        }
        "last_door" | "f1_changelevel" => r = rotation(0., 45., 0.),
        "drawbridge" => {
            p.x -= 112.;
            r = rotation(90., 0., 0.);
        }
        "schoolhouse" => r = school(s),
        "schoolbell" | "window_portal" | "shutter1" | "shutter2" | "s1_changelevel" => {
            let pivot = vec3(-4060., 3088., -184.);
            r = school(s);
            p = pivot + r * (base - pivot);
            let local = match name {
                "schoolbell" => rotation(18. * (s.age * std::f32::consts::TAU / 6.).sin(), 0., 0.),
                "shutter1" if s.shutters => rotation(0., -75., 0.),
                "shutter2" if s.shutters => rotation(0., 75., 0.),
                _ => Quat::IDENTITY,
            };
            r *= local;
        }
        "hand1" | "hand3" => r = rotation(0., 0., s.age * 72.),
        "hand2" => r = rotation(0., 0., -s.age * 54.),
        "hand4" => r = rotation(0., 0., s.age * 54.),
        "pendulum1" | "pendulum2" => {
            let t = (s.age - if name == "pendulum2" { 4. } else { 0. }).rem_euclid(8.);
            let amount = if t < 2. {
                t / 2.
            } else if t < 4. {
                (4. - t) / 2.
            } else {
                0.
            };
            r = rotation(
                0.,
                0.,
                amount * if name == "pendulum1" { -40. } else { 40. },
            );
        }
        _ => (),
    }
    (p, r)
}
impl Fortress {
    pub fn objective(&self) -> String {
        if self.state.returning {
            "Follow the upper wall to the Skool. Run and jump from the raised edge through its open window.".into()
        } else {
            "The Skool window opens on your return. Follow the lower platforms to the portal, then cross the splitting room to Beyond the Wall.".into()
        }
    }
    pub fn load(assets: &mut crate::assets::Assets, map: &Bsp, returning: bool) -> Result<Self> {
        let state = State {
            cinema: cinema::State::fresh(returning),
            returning,
            age: 0.,
            splitting: false,
            split: 0.,
            shutters: false,
        };
        let mut objects = Vec::new();
        for e in map.entities.iter().filter(|e| supported(e)) {
            let name = e.get("targetname").cloned().unwrap_or_default();
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = vector(&e["origin"]).context("Invalid fortress origin")?;
            let (p, r) = pose(&name, base, &state);
            objects.push(Object {
                solid: matches!(
                    name.as_str(),
                    "skool_clip_hide"
                        | "split_byebye"
                        | "split_herehere"
                        | "split_fulc"
                        | "split_middle"
                        | "t176"
                ),
                name,
                model,
                base,
                collider: Collider::model(map, model, p, r, true)?,
            });
        }
        Ok(Self {
            cinema: cinema::Data::load(assets, map)?,
            state,
            objects,
        })
    }
    fn visible(&self, name: &str) -> bool {
        match name {
            "skool_clip_hide" => !self.state.returning,
            "window_portal" | "portal_back1" => self.state.returning,
            "second_portal1" => !self.state.returning,
            "skymojo" => false,
            _ => !name.starts_with("stal") || self.state.cinema.beat == Some(cinema::Beat::Arrival),
        }
    }
    pub fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("fortress.returning", self.state.returning);
        f
    }
    pub fn gate(name: &str, thread: &str) -> Condition {
        match (name, thread) {
            ("s1_changelevel" | "skool_cat_trigger" | "boojum_trigger", _) => {
                Condition::flag("fortress.returning")
            }
            ("second_teleport" | "f1_changelevel", _)
            | (_, "push_cat_trigger" | "Fortress1_Start_Split") => {
                Condition::flag("fortress.returning").not()
            }
            _ => Condition::Always,
        }
    }
    pub fn event(&mut self, thread: &str) -> Option<Events> {
        match thread {
            "Fortress1_Start_Split" => {
                self.state.splitting = true;
                Some(Events {
                    sound: Some("sound/ambience/special/roomsplit.wav".into()),
                    ..Default::default()
                })
            }
            "Fortress1_Boojum_Attack" => {
                self.state.shutters = true;
                if self.state.returning
                    && !self.state.cinema.boojum_done
                    && !self.state.cinema.active()
                {
                    self.state.cinema.beat = Some(cinema::Beat::Boojum);
                    self.state.cinema.time = 0.;
                }
                Some(Events::default())
            }
            _ => None,
        }
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .filter(|o| self.visible(&o.name))
            .map(|o| {
                let (p, r) = self
                    .cinema
                    .brush(&o.name, o.base, &self.state.cinema)
                    .unwrap_or_else(|| pose(&o.name, o.base, &self.state));
                (o.model, p, r)
            })
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| o.solid && self.visible(&o.name))
            .map(|o| o.collider.clone())
    }
    pub fn trigger_pose(&self, name: &str, base: Vec3) -> Option<(Vec3, Quat)> {
        match name {
            "s1_changelevel" => Some(pose(name, base, &self.state)),
            "f1_changelevel" => {
                let pivot = vec3(-360., -2680., 128.);
                let r = rotation(0., 45., 0.);
                Some((pivot + r * (base - pivot), r))
            }
            _ => None,
        }
    }
    pub fn snapshot(&self) -> State {
        self.state.clone()
    }
    pub fn restore(&mut self, s: &State, map: &Bsp) -> Result<()> {
        s.cinema.validate(s.returning)?;
        anyhow::ensure!(
            s.returning == self.state.returning
                && s.age.is_finite()
                && s.age >= 0.
                && (0. ..=1.).contains(&s.split)
                && (s.splitting || s.split == 0.),
            "Invalid fortress save"
        );
        self.state = s.clone();
        if self.state.cinema.version == 0 {
            self.migrate_cinema();
        }
        self.rebuild(map)
    }
    pub fn migrate_cinema(&mut self) {
        self.state.cinema = cinema::State {
            version: 1,
            arrival_done: true,
            boojum_done: self.state.shutters,
            ..Default::default()
        };
    }
    pub fn skip(&mut self, player: &mut Player, story: &mut crate::story::Story) -> bool {
        self.cinema.skip(&mut self.state.cinema, player, story)
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        for o in &mut self.objects {
            if o.solid {
                let (p, r) = pose(&o.name, o.base, &self.state);
                o.collider = Collider::model(map, o.model, p, r, true)?;
            }
        }
        Ok(())
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        self.state.age += dt.min(0.1);
        self.cinema.advance(&mut self.state.cinema, dt, player);
        if self.state.splitting && self.state.split < 1. {
            let old = self.state.clone();
            let next = (old.split + dt.min(0.1) / 10.).min(1.);
            let riding = self
                .objects
                .iter()
                .find(|o| {
                    if !o.solid || !self.visible(&o.name) {
                        return false;
                    }
                    let hit = o.collider.trace(
                        player.feet + PLAYER_CENTER,
                        player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                        PLAYER_HALF,
                    );
                    player.velocity.z <= 1.
                        && !hit.start_solid
                        && hit.fraction < 1.
                        && hit.normal.z > 0.65
                })
                .map(|o| (o.name.clone(), o.model, o.base));
            self.state.split = next;
            self.rebuild(map)?;
            let mut feet = player.feet;
            if let Some((name, model, base)) = riding {
                let (p0, r0) = pose(&name, base, &old);
                let (p1, r1) = pose(&name, base, &self.state);
                feet = p1 + r1 * r0.inverse() * (feet - p0);
                if let Some((f, n)) = self
                    .objects
                    .iter()
                    .find(|o| o.model == model)
                    .and_then(|o| o.collider.rider_feet(feet))
                {
                    feet = f;
                    player.ground_normal = n;
                }
            }
            world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            if world.body_clear(feet) {
                player.feet = feet;
            } else {
                self.state.split = old.split;
                self.rebuild(map)?;
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
}

//! Shared soldier/Corporal combat. Scene controllers retain ownership and stable hit IDs.
use crate::collision::World;
use crate::combat::{DamageKind, Feedback, Hit, Opponents, Recoil, Target};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
pub trait Timing {
    fn duration(&self, model: &str, clip: &str) -> f32;
    fn frame(&self, model: &str, clip: &str) -> f32;
    fn speed(&self, model: &str, clip: &str) -> f32;
}
fn unit_scale() -> f32 {
    1.
}
pub fn is_ant(model: &str) -> bool {
    matches!(model, "c_armyant" | "c_armyantcorp")
}
/// Placed actors whose source scene owns AI activation. They remain damageable.
pub fn script_wait(map: &str, name: &str) -> bool {
    match map {
        "garden1" => matches!(
            name,
            "ant_nearstart1" | "ant_corner1" | "ant_run1" | "ant_run2" | "ant_run10" | "ant_run11"
        ),
        "centipede1" => matches!(name, "ant_runner1" | "ant_runner2" | "ant_runner3"),
        "centipede2" => matches!(
            name,
            "ant_guard1" | "ant_guard2" | "ant_guard3" | "ant_guard4" | "ant1" | "ant2"
        ),
        _ => false,
    }
}

pub const CORPORAL: &[&str] = &[
    "idle",
    "walk_medium",
    "retreat_1",
    "attack_1",
    "attack_3",
    "pain1",
    "pain2",
    "pain3",
    "death_11",
    "death_12",
    "death_13",
    "death_frozen",
];
pub const REGULAR: &[&str] = &[
    "idle",
    "push",
    "walk_medium",
    "walk_fast",
    "retreat_1",
    "attack_1",
    "attack_3_idle_2fire",
    "attack_3_fire",
    "attack_3_fire_2idle",
    "pain1",
    "pain2",
    "pain3",
    "death_11",
    "death_12",
    "death_13",
    "death_frozen",
];
const HALF: Vec3 = Vec3::new(36., 36., 36.);
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Phase {
    Idle,
    Chase,
    Retreat,
    Melee,
    Ready,
    Fire,
    Recover,
    Pain,
    Dead,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bullet {
    pub at: Vec3,
    #[serde(default)]
    pub grenade: bool,
    pub direction: Vec3,
    pub age: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Blast {
    pub at: Vec3,
    pub time: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Ant {
    #[serde(default, skip_serializing_if = "crate::electric::inactive")]
    pub electric: f32,
    #[serde(default)]
    pub script_wait: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sight_range: Option<f32>,
    #[serde(default)]
    pub blasts: Vec<Blast>,
    #[serde(default)]
    pub variant: usize,
    #[serde(default = "unit_scale")]
    pub scale: f32,
    #[serde(default)]
    last_seen: Option<Vec3>,
    #[serde(default)]
    memory: f32,
    #[serde(default)]
    detour: crate::combat::navigation::Detour,
    #[serde(skip)]
    pub notarget: bool,
    #[serde(default)]
    pub id: usize,
    #[serde(default)]
    pub corporal: bool,
    pub enabled: bool,
    pub feet: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub phase: Phase,
    pub time: f32,
    pub fired: bool,
    pub frozen: bool,
    pub shots: Vec<Bullet>,
    pub opponents: Opponents,
    recoil: Recoil,
    falling: f32,
    accumulator: f32,
    random: u32,
}
impl Ant {
    pub fn new(id: usize, corporal: bool, feet: Vec3, yaw: f32) -> Self {
        Self {
            script_wait: false,
            sight_range: None,
            blasts: Vec::new(),
            variant: 0,
            scale: 1.,
            last_seen: None,
            memory: 0.,
            detour: Default::default(),
            notarget: false,
            id,
            corporal,
            enabled: true,
            feet,
            yaw,
            health: if corporal { 160. } else { 100. },
            phase: Phase::Idle,
            time: 0.,
            fired: false,
            frozen: false,
            shots: Vec::new(),
            opponents: Opponents::default(),
            recoil: Recoil::default(),
            electric: 0.,
            falling: 0.,
            accumulator: 0.,
            random: id as u32,
        }
    }
    pub fn pusher(feet: Vec3, yaw: f32, seed: u32) -> Self {
        let mut ant = Self::new(seed as usize, false, feet, yaw);
        ant.enabled = false;
        ant
    }
    pub fn loops(&self) -> bool {
        matches!(self.phase, Phase::Idle | Phase::Chase | Phase::Retreat)
    }
    pub fn visual_scale(&self, timing: &impl Timing) -> f32 {
        if self.health > 0. {
            return self.scale;
        }
        let settle = timing.duration(self.model(), self.clip());
        self.scale * (1. - (self.time - settle - 5.).max(0.) / 2.).clamp(0., 1.)
    }
    pub fn grade(&self) -> crate::loot::Grade {
        if self.corporal {
            crate::loot::Grade::Large
        } else {
            crate::loot::Grade::Medium
        }
    }
    pub fn model(&self) -> &'static str {
        if self.corporal {
            "c_armyantcorp"
        } else {
            "c_armyant"
        }
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.feet + Vec3::Z * HALF.z * self.scale,
            half: HALF * self.scale,
        }
    }
    pub fn clip(&self) -> &'static str {
        match self.phase {
            Phase::Idle => "idle",
            Phase::Chase => "walk_medium",
            Phase::Retreat => "retreat_1",
            Phase::Melee => "attack_1",
            Phase::Ready if self.corporal => "attack_3",
            Phase::Ready => "attack_3_idle_2fire",
            Phase::Fire if self.corporal => "attack_3",
            Phase::Fire => "attack_3_fire",
            Phase::Recover if self.corporal => "idle",
            Phase::Recover => "attack_3_fire_2idle",
            Phase::Pain => ["pain1", "pain2", "pain3"][self.variant.min(2)],
            Phase::Dead if self.frozen => "death_frozen",
            Phase::Dead => ["death_11", "death_12", "death_13"][self.variant.min(2)],
        }
    }
    fn set(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
        self.fired = false;
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if !self.enabled || self.health <= 0. || !hit.damage.is_finite() || hit.damage <= 0. {
            return None;
        }
        self.opponents.demon |= hit.kind.is_demon();
        self.health = (self.health - hit.damage).max(0.);
        self.recoil.hit(hit.knockback, 100.);
        crate::electric::hit(&mut self.electric, hit);
        // Repeated damage does not restart pain every frame and stunlock forever.
        if self.phase != Phase::Pain || self.health == 0. {
            self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
            self.variant = self.random as usize % 3;
        }
        if self.health == 0. {
            self.frozen = hit.kind.means() == DamageKind::Ice;
            self.set(Phase::Dead);
            Some(self.sound(true))
        } else {
            if self.phase != Phase::Pain {
                self.set(Phase::Pain);
            }
            Some(self.sound(false))
        }
    }
    fn sound(&self, death: bool) -> &'static str {
        (match (self.corporal, death) {
            (false, false) => [
                "sound/character/army_ant/pain1.wav",
                "sound/character/army_ant/pain2.wav",
                "sound/character/army_ant/pain3.wav",
            ],
            (false, true) => [
                "sound/character/army_ant/death1.wav",
                "sound/character/army_ant/death2.wav",
                "sound/character/army_ant/death3.wav",
            ],
            (true, false) => [
                "sound/character/army_ant_corp/pain1.wav",
                "sound/character/army_ant_corp/pain2.wav",
                "sound/character/army_ant_corp/pain3.wav",
            ],
            (true, true) => [
                "sound/character/army_ant_corp/death1.wav",
                "sound/character/army_ant_corp/death2.wav",
                "sound/character/army_ant_corp/death3.wav",
            ],
        })[self.variant.min(2)]
    }
    fn chance(&mut self) -> bool {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        self.random % 10 < 4
    }
    pub fn advance(
        &mut self,
        ctx: &crate::level::Combat<'_>,
        data: &impl Timing,
        out: &mut Feedback,
    ) {
        self.opponents.summon = ctx.summon;
        self.notarget = ctx.notarget;
        self.update(ctx.dt, ctx.world, ctx.player.eye(), data, out);
    }
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        eye: Vec3,
        data: &impl Timing,
        out: &mut Feedback,
    ) {
        if !self.enabled || !dt.is_finite() || dt <= 0. {
            return;
        }
        self.accumulator += dt.min(0.1);
        while self.accumulator + 0.000001 >= 1. / 120. {
            self.accumulator = (self.accumulator - 1. / 120.).max(0.);
            self.step(world, eye, data, out);
        }
    }
    fn step(&mut self, world: &World, player_eye: Vec3, data: &impl Timing, out: &mut Feedback) {
        self.electric = (self.electric - 1. / 120.).max(0.);
        let dt = 1. / 120.;
        let half = HALF * self.scale;
        self.feet += self.recoil.step(dt, world, self.target(0));
        self.time += dt;
        self.falling = (self.falling + 800. * dt).min(800.);
        let body = self.target(0);
        let down = world.sweep(body.center, body.center - Vec3::Z * self.falling * dt, half);
        if !down.start_solid {
            self.feet.z -= self.falling * dt * down.fraction;
        }
        if down.start_solid || down.fraction < 1. {
            self.falling = 0.;
        }
        let bodies = self.opponents.bodies(player_eye);
        self.blasts.retain_mut(|b| {
            b.time += dt;
            b.time < 0.6
        });
        self.shots.retain_mut(|b| {
            b.age += dt;
            if b.grenade {
                b.direction.z -= 800. * dt;
            }
            let end = b.at + b.direction * (if b.grenade { dt } else { 1500. * dt });
            let hit = crate::combat::contact(
                &crate::combat::Context {
                    world,
                    targets: &bodies,
                },
                b.at,
                end,
                if b.grenade { 8. } else { 1. },
            );
            if let Some((id, _)) = hit {
                out.strike(
                    id,
                    if b.grenade { 5. } else { 10. },
                    b.direction.normalize_or_zero() * 25.,
                    DamageKind::Other,
                );
                if !b.grenade {
                    return false;
                }
            }
            let wall = world.sweep(b.at, end, Vec3::splat(if b.grenade { 8. } else { 1. }));
            if b.grenade && (hit.is_some() || b.age >= 2.) {
                if self.blasts.len() < 32 {
                    self.blasts.push(Blast { at: b.at, time: 0. });
                }
                out.spatial_sounds
                    .push(("sound/character/army_ant_corp/grenade.wav", b.at));
                for body in &bodies {
                    let distance = body.center.distance(b.at);
                    let sight = world.sweep(b.at, body.center, Vec3::splat(0.5));
                    if distance < 160. && !sight.start_solid && sight.fraction == 1. {
                        let direction = (body.center - b.at).normalize_or_zero();
                        out.strike(
                            body.id,
                            80. * (1. - distance / 160.),
                            direction * 100.,
                            DamageKind::Other,
                        );
                    }
                }
                return false;
            }
            if b.grenade && wall.fraction < 1. {
                b.at = b.at.lerp(end, wall.fraction) + wall.normal * 0.1;
                b.direction = (b.direction - 2. * b.direction.dot(wall.normal) * wall.normal) * 0.5;
            } else {
                b.at = end;
            }
            !wall.start_solid && (b.grenade || wall.fraction == 1.) && b.age < 5.
        });
        if self.phase == Phase::Dead {
            self.time = self.time.min(data.duration(self.model(), self.clip()) + 7.);
            return;
        }
        if self.script_wait {
            if self.phase == Phase::Pain && self.time >= data.duration(self.model(), self.clip()) {
                self.set(Phase::Idle);
            }
            // Keep the existing ambient look response without starting an encounter.
            let delta = player_eye - self.target(0).center;
            let sight = world.sweep(self.target(0).center, player_eye, Vec3::splat(0.5));
            if self.phase == Phase::Idle
                && delta.length() < 400.
                && !sight.start_solid
                && sight.fraction == 1.
            {
                let turn = (delta.y.atan2(delta.x) - self.yaw + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                self.yaw += turn.clamp(-dt * 1.8, dt * 1.8);
            }
            return;
        }
        let (eye, notarget, victim) = self.opponents.aim(
            world,
            self.target(0).center,
            player_eye,
            self.notarget,
            matches!(self.phase, Phase::Melee | Phase::Ready | Phase::Fire),
        );
        let mut delta = eye - self.target(0).center;
        let distance = delta.truncate().length();
        let sight = world.sweep(self.target(0).center, eye, Vec3::splat(0.5));
        let visible = !notarget
            && delta.length() < self.sight_range.unwrap_or(if self.corporal { 1200. } else { 1000. })
            && !sight.start_solid
            && sight.fraction >= 1.;
        if visible {
            self.last_seen = Some(eye);
            self.memory = 3.;
        } else {
            self.memory = (self.memory - dt).max(0.);
            if self.notarget {
                self.memory = 0.;
            }
        }
        let pursuing = visible || self.memory > 0.;
        if !visible && pursuing {
            delta = self.last_seen.unwrap_or(eye) - self.target(0).center;
        }
        if pursuing && !matches!(self.phase, Phase::Melee | Phase::Fire | Phase::Pain) {
            let difference = (delta.y.atan2(delta.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += difference.clamp(-dt * 10., dt * 10.);
        }
        let facing =
            vec2(self.yaw.cos(), self.yaw.sin()).dot(delta.truncate().normalize_or_zero()) > 0.7;
        let duration = data.duration(self.model(), self.clip());
        match self.phase {
            Phase::Melee => {
                if !self.fired
                    && self.time
                        >= data.frame(self.model(), "attack_1")
                            * if self.corporal { 13. } else { 15. }
                {
                    self.fired = true;
                    if visible
                        && distance < 120. * self.scale
                        && delta.z.abs() < 72. * self.scale
                        && vec2(self.yaw.cos(), self.yaw.sin())
                            .dot(delta.truncate().normalize_or_zero())
                            > 0.5
                    {
                        out.strike(
                            victim,
                            20.,
                            delta.normalize_or_zero() * 50.,
                            DamageKind::Other,
                        );
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Chase);
                }
            }
            Phase::Ready if self.time >= duration => self.set(Phase::Fire),
            Phase::Fire => {
                if !self.fired
                    && self.time
                        >= data.frame(self.model(), self.clip())
                            * if self.corporal { 25. } else { 2. }
                {
                    self.fired = true;
                    if visible && self.shots.len() < 32 {
                        let at = self.feet + Vec3::Z * 54. * self.scale;
                        self.shots.push(Bullet {
                            at,
                            grenade: self.corporal,
                            direction: if self.corporal {
                                (eye - at).normalize_or_zero() * 500. + Vec3::Z * 180.
                            } else {
                                (eye - at).normalize_or_zero()
                            },
                            age: 0.,
                        });
                        if !self.corporal {
                            out.spatial_sounds
                                .push(("sound/character/army_ant/gunshot2.wav", at));
                        }
                    }
                }
                if self.time >= duration {
                    self.set(Phase::Recover);
                }
            }
            Phase::Recover | Phase::Pain if self.time >= duration => self.set(Phase::Chase),
            Phase::Retreat => {
                if pursuing {
                    self.walk(world, -delta, data.speed(self.model(), "walk_medium") * dt);
                }
                if self.time >= 2. || distance >= 400. {
                    self.set(Phase::Chase);
                }
            }
            Phase::Idle | Phase::Chase if pursuing => {
                if visible && self.health < 45. && distance < 400. && self.time > 0.5 {
                    self.set(Phase::Retreat);
                } else if visible
                    && facing
                    && distance < 110. * self.scale
                    && delta.z.abs() < 72. * self.scale
                {
                    self.set(Phase::Melee);
                } else if visible
                    && facing
                    && (self.phase == Phase::Idle || self.time >= duration)
                    && self.chance()
                {
                    self.set(if self.corporal {
                        Phase::Fire
                    } else {
                        Phase::Ready
                    });
                } else {
                    if self.phase == Phase::Idle || self.time >= duration {
                        self.set(Phase::Chase);
                    }
                    if delta.truncate().length() > 24. {
                        self.walk(world, delta, data.speed(self.model(), "walk_medium") * dt);
                    } else if !visible {
                        self.memory = 0.;
                    }
                }
            }
            Phase::Chase if !pursuing => self.set(Phase::Idle),
            _ => {}
        }
    }
    fn walk(&mut self, world: &World, delta: Vec3, step: f32) {
        self.feet = self.detour.walk(world, self.feet, self.feet + delta.with_z(0.),
            HALF * self.scale, step * self.scale, 1. / 120.);
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.detour.valid() && self.variant < 3
                && self.scale.is_finite()
                && self.sight_range.is_none_or(|r| r.is_finite() && (1. ..=10000.).contains(&r))
                && (0.01..=10.).contains(&self.scale)
                && self.memory.is_finite()
                && (0. ..=3.).contains(&self.memory)
                && self
                    .last_seen
                    .is_none_or(|p| p.is_finite() && p.abs().max_element() < 100_000.)
                && self.feet.is_finite()
                && self.feet.abs().max_element() < 100_000.
                && self.yaw.is_finite()
                && (0. ..=if self.corporal { 160. } else { 100. }).contains(&self.health)
                && (self.health == 0.) == (self.phase == Phase::Dead)
                && [self.time, self.falling, self.accumulator]
                    .iter()
                    .all(|t| t.is_finite() && (0. ..=1e6).contains(t))
                && self.recoil.valid()
                && (0. ..=crate::electric::LIFE).contains(&self.electric)
                && self.blasts.len() <= 32
                && self
                    .blasts
                    .iter()
                    .all(|b| b.at.is_finite() && (0. ..=0.6).contains(&b.time))
                && self.shots.len() <= 32
                && self.shots.iter().all(|b| b.grenade == self.corporal
                    && b.at.is_finite()
                    && b.direction.is_finite()
                    && if b.grenade {
                        b.direction.length() <= 2000.
                    } else {
                        (b.direction.length_squared() - 1.).abs() < 0.01
                    }
                    && (0. ..=5.).contains(&b.age)),
            "Invalid Ant save"
        );
        Ok(())
    }
}

pub fn draw_shot(props: &mut [crate::weapons::Prop; 2], shot: &Bullet, fullbright: bool) {
    let rotation = if shot.grenade {
        Quat::from_euler(
            EulerRot::XYZ,
            (100. * shot.age).to_radians(),
            (200. * shot.age).to_radians(),
            (300. * shot.age).to_radians(),
        )
    } else {
        Quat::from_rotation_arc(Vec3::X, shot.direction)
    };
    props[usize::from(shot.grenade)].draw_frame(
        crate::skeletal::Transform {
            translation: shot.at,
            rotation,
        },
        1.,
        fullbright,
        shot.age,
        true,
    );
}

/// A bounded, depth-tested flash marks grenade damage while the original nested
/// explosion emitters await shared projectile-event support.
pub fn draw_blasts(ant: &Ant) {
    if ant.blasts.is_empty() {
        return;
    }
    crate::render::depth_read_only(|| {
        gl_use_default_material();
        for b in &ant.blasts {
            let f = b.time / 0.6;
            draw_sphere(
                b.at,
                6. + 50. * f,
                None,
                Color::new(1., 0.5, 0.12, (1. - f) * 0.65),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Clips;
    impl Timing for Clips {
        fn duration(&self, _: &str, _: &str) -> f32 {
            1.
        }
        fn frame(&self, _: &str, _: &str) -> f32 {
            0.03
        }
        fn speed(&self, _: &str, _: &str) -> f32 {
            100.
        }
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))])
    }
    #[test]
    fn ant_detours_without_oscillating_at_obstacle_corners() {
        let world=World::fixture(&[(vec3(-500.,-500.,-20.),vec3(500.,500.,0.)),(vec3(90.,-65.,0.),vec3(150.,65.,120.))]);
        let mut a=Ant::new(1,false,Vec3::Z*0.1,0.);
        let goal=vec3(330.,0.,0.1);
        for _ in 0..900 { a.walk(&world,goal-a.feet,1.); }
        assert!(a.feet.distance(goal)<3.,"{:?}",a.feet);
        a.validate().unwrap();
        let mut old=serde_json::to_value(a).unwrap();old.as_object_mut().unwrap().remove("detour");
        serde_json::from_value::<Ant>(old).unwrap().validate().unwrap();
    }
    #[test]
    fn scripted_ants_take_damage_without_starting_their_encounter() {
        let mut a = Ant::new(1, false, Vec3::Z * 0.1, 0.);
        a.script_wait = script_wait("centipede1", "ant_runner1");
        assert!(a.script_wait);
        assert_eq!(tick(&mut a, &floor(), vec3(75., 0., 48.), 5, 60).damage, 0.);
        assert!(a.hit(hit(10., DamageKind::Other)).is_some());
        assert_eq!(a.phase, Phase::Pain);
        let mut restored: Ant = serde_json::from_value(serde_json::to_value(&a).unwrap()).unwrap();
        assert_eq!(
            tick(&mut restored, &floor(), vec3(75., 0., 48.), 5, 60).damage,
            0.
        );
        assert_eq!(restored.phase, Phase::Idle);
        restored.hit(hit(1000., DamageKind::Other));
        tick(&mut restored, &floor(), vec3(75., 0., 48.), 10, 60);
        assert_eq!(restored.visual_scale(&Clips), 0.);
        assert!(script_wait("centipede2", "ant_guard2"));
        assert!(script_wait("garden1", "ant_nearstart1"));
        assert!(!script_wait("potears2", "antguards"));
    }
    fn hit(damage: f32, kind: DamageKind) -> Hit {
        Hit {
            id: 0,
            damage,
            kind,
            knockback: Vec3::ZERO,
        }
    }
    fn tick(a: &mut Ant, world: &World, eye: Vec3, seconds: u32, fps: u32) -> Feedback {
        let mut out = Feedback::default();
        for _ in 0..seconds * fps {
            a.update(1. / fps as f32, world, eye, &Clips, &mut out);
        }
        out
    }
    #[test]
    fn melee_uses_authored_frame_once_and_pause_is_exact() {
        for corporal in [false, true] {
            let mut a = Ant::new(7, corporal, Vec3::Z * 0.1, 0.);
            let eye = vec3(75., 0., 48.);
            a.set(Phase::Melee);
            let mut out = Feedback::default();
            for _ in 0..20 {
                a.update(1. / 120., &floor(), eye, &Clips, &mut out);
            }
            assert_eq!(out.damage, 0.);
            let saved = serde_json::to_value(&a).unwrap();
            for dt in [0., -1., f32::NAN] {
                a.update(dt, &floor(), eye, &Clips, &mut out);
            }
            assert_eq!(saved, serde_json::to_value(&a).unwrap());
            for _ in 0..85 {
                a.update(1. / 120., &floor(), eye, &Clips, &mut out);
            }
            assert_eq!(out.damage, 20.);
            a.validate().unwrap();
        }
    }
    #[test]
    fn pursuit_and_attacks_match_at_30_60_144_hz() {
        for corporal in [false, true] {
            let mut values = Vec::new();
            for fps in [30, 60, 144] {
                let mut a = Ant::new(11, corporal, Vec3::Z * 0.1, 0.);
                let out = tick(&mut a, &floor(), vec3(500., 0., 48.), 12, fps);
                assert!(a.feet.x > 100. && out.damage > 0.);
                a.validate().unwrap();
                values.push((a.feet, out.damage));
            }
            for pair in values.windows(2) {
                assert!(pair[0].0.distance(pair[1].0) < 0.1);
                assert!((pair[0].1 - pair[1].1).abs() < 0.1);
            }
        }
    }
    #[test]
    fn walls_notarget_and_ledge_support_prevent_unfair_attacks_or_falls() {
        let wall = World::fixture(&[
            (vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.)),
            (vec3(200., -2000., 0.), vec3(220., 2000., 300.)),
        ]);
        let mut a = Ant::new(1, false, Vec3::Z * 0.1, 0.);
        assert_eq!(tick(&mut a, &wall, vec3(300., 0., 48.), 5, 60).damage, 0.);
        assert_eq!(a.phase, Phase::Idle);
        a.notarget = true;
        assert_eq!(tick(&mut a, &floor(), vec3(75., 0., 48.), 5, 60).damage, 0.);
        let ledge = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        for _ in 0..400 {
            a.walk(&ledge, Vec3::X, 2.);
        }
        assert!(a.feet.x < 66. && a.feet.z >= 0.);
    }
    #[test]
    fn damage_death_cleanup_loot_and_saved_continuation_are_one_shot() {
        for corporal in [false, true] {
            for kind in [DamageKind::Other, DamageKind::Ice] {
                let mut a = Ant::new(1, corporal, Vec3::Z * 0.1, 0.);
                assert!(a.hit(hit(10., DamageKind::Other)).is_some());
                a.time = 0.5;
                a.hit(hit(10., DamageKind::Other));
                assert_eq!(a.time, 0.5);
                let before = [crate::loot::Source {
                    id: 1,
                    feet: a.feet,
                    grade: a.grade(),
                    dead: false,
                }];
                assert!(a.hit(hit(1000., kind)).is_some());
                assert_eq!(a.phase, Phase::Dead);
                assert_eq!(a.frozen, kind == DamageKind::Ice);
                assert!(a.hit(hit(1., kind)).is_none());
                let mut b: Ant = serde_json::from_value(serde_json::to_value(&a).unwrap()).unwrap();
                let after = [crate::loot::Source {
                    dead: true,
                    ..before[0]
                }];
                let mut loot = crate::loot::Loot::default();
                loot.defeated(&before, &after, &floor());
                loot.defeated(&before, &after, &floor());
                assert_eq!(loot.drops.len(), 1);
                assert_eq!(loot.drops[0].grade, a.grade());
                assert_eq!(
                    tick(&mut a, &floor(), vec3(75., 0., 48.), 10, 60).damage,
                    0.
                );
                tick(&mut b, &floor(), vec3(75., 0., 48.), 10, 60);
                assert_eq!(
                    serde_json::to_value(&a).unwrap(),
                    serde_json::to_value(&b).unwrap()
                );
                assert_eq!(a.visual_scale(&Clips), 0.);
                assert!(a.shots.is_empty());
                a.validate().unwrap();
            }
        }
    }
    #[test]
    fn old_scene_saves_migrate_and_projectiles_expire_after_death() {
        let mut a = Ant::pusher(Vec3::Z * 0.1, 0., 634);
        let mut old = serde_json::to_value(&a).unwrap();
        for key in [
            "id",
            "corporal",
            "variant",
            "scale",
            "memory",
            "last_seen",
            "blasts",
        ] {
            old.as_object_mut().unwrap().remove(key);
        }
        let b: Ant = serde_json::from_value(old).unwrap();
        b.validate().unwrap();
        assert!(!b.enabled);
        a.enabled = true;
        a.shots.push(Bullet {
            at: vec3(0., 0., 80.),
            grenade: false,
            direction: Vec3::Y,
            age: 0.,
        });
        a.hit(hit(1000., DamageKind::Other));
        tick(&mut a, &floor(), vec3(-1000., 0., 48.), 6, 60);
        assert!(a.shots.is_empty());
        a.variant = 3;
        assert!(a.validate().is_err());
    }
    #[test]
    fn grenades_bounce_expire_and_preserve_mid_flight_saves() {
        let mut a = Ant::new(1, true, Vec3::Z * 0.1, 0.);
        a.notarget = true;
        a.shots.push(Bullet {
            at: vec3(0., 0., 20.),
            grenade: true,
            direction: vec3(30., 0., -100.),
            age: 0.,
        });
        let mut out = Feedback::default();
        for _ in 0..60 {
            a.update(1. / 120., &floor(), vec3(1000., 0., 48.), &Clips, &mut out);
            if a.shots[0].direction.z > 0. {
                break;
            }
        }
        assert!(a.shots[0].direction.z > 0.);
        let mut b: Ant = serde_json::from_value(serde_json::to_value(&a).unwrap()).unwrap();
        b.notarget = true;
        tick(&mut a, &floor(), vec3(1000., 0., 48.), 3, 60);
        tick(&mut b, &floor(), vec3(1000., 0., 48.), 3, 60);
        assert!(a.shots.is_empty() && a.blasts.is_empty());
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }
}

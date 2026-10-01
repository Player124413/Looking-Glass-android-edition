//! Independently authored flying enemy and swept scream projectiles.
use crate::{
    assets::Assets,
    collision::World,
    combat::{self, Feedback, Target},
    skeletal::{Animation, Definition, Skeleton},
};
use anyhow::Result;
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum State {
    Fly,
    Scream,
    Pain,
    Dead,
}
#[derive(Clone, Copy)]
pub struct Timing {
    pub frame: f32,
    pub attack: f32,
    pub pain: f32,
    pub death: f32,
}
impl Timing {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let d = Definition::load(assets, "models/c_boojum.tik")?;
        let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", d.path, d.model))?)?;
        let mut read = |n: &str| {
            Animation::parse(
                &assets.read(&format!("{}/{}", d.path, d.animations[n]))?,
                skeleton.bones.len(),
            )
        };
        let attack = read("attack_scream")?;
        Ok(Self {
            frame: attack.frame_time,
            attack: attack.duration(),
            pain: read("pain1")?.duration(),
            death: read("death_part01")?.duration(),
        })
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Wave {
    pub position: Vec3,
    pub direction: Vec3,
    pub age: f32,
    speed: f32,
    damage: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Boojum {
    #[serde(default, skip_serializing_if = "crate::electric::inactive")]
    pub electric: f32,
    #[serde(default)]
    pub opponents: combat::Opponents,
    #[serde(default)]
    pub frozen: bool,
    #[serde(default)]
    pub recoil: combat::Recoil,
    pub notarget: bool,
    pub feet: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub state: State,
    pub time: f32,
    pub active: bool,
    pub waves: Vec<Wave>,
    pub attacks: u32,
    pub shots: u32,
    home: Vec3,
    wait: f32,
    next_shot: usize,
    accumulator: f64,
    drift: f32,
    fall: f32,
}
impl Boojum {
    pub fn validate_save(&self) -> Result<()> {
        anyhow::ensure!(
            (0.0..=1000.).contains(&self.health)
                && self.time >= 0.
                && self.next_shot <= 6
                && self.accumulator.abs() < 0.01
                && self.waves.len() <= 256
                && self.feet.is_finite()
                && self.recoil.valid()
                && (0. ..=crate::electric::LIFE).contains(&self.electric)
                && (!self.frozen || self.health == 0.)
                && self.home.is_finite()
                && ((self.state == State::Dead) == (self.health == 0.)),
            "Invalid saved Boojum"
        );
        anyhow::ensure!(
            self.waves.iter().all(|w| w.age >= 0.
                && w.speed > 0.
                && w.speed <= 5000.
                && (0.0..=1000.).contains(&w.damage)
                && w.position.is_finite()
                && w.direction.is_finite()
                && w.direction.length_squared() <= 1.01),
            "Invalid saved Boojum projectile"
        );
        Ok(())
    }
    pub fn new(feet: Vec3, delay: f32) -> Self {
        Self {
            opponents: Default::default(),
            frozen: false,
            recoil: combat::Recoil::default(),
            electric: 0.,
            notarget: false,
            feet,
            yaw: 0.,
            health: 70.,
            state: State::Fly,
            time: 0.,
            active: false,
            waves: Vec::new(),
            attacks: 0,
            shots: 0,
            home: feet,
            wait: delay,
            next_shot: 0,
            accumulator: 0.,
            drift: delay,
            fall: 0.,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.feet + Vec3::Z * 36.,
            half: vec3(24., 24., 28.),
        }
    }
    fn state(&mut self, s: State) {
        self.state = s;
        self.time = 0.;
        self.next_shot = 0;
    }
    pub fn hit_attack(&mut self, hit: combat::Hit) -> Option<&'static str> {
        if self.health > 0. && hit.damage.is_finite() && hit.damage > 0. && self.active {
            self.recoil.hit(hit.knockback, 200.);
            crate::electric::hit(&mut self.electric, hit);
        }
        let was_alive = self.health > 0.;
        let sound = self.hit(hit.damage);
        if was_alive && self.health == 0. && hit.kind == combat::DamageKind::Ice {
            self.frozen = true;
            Some("sound/character/shared/freeze_death.wav")
        } else {
            sound
        }
    }
    pub fn hit(&mut self, damage: f32) -> Option<&'static str> {
        if !self.active || self.health <= 0. || !damage.is_finite() || damage <= 0. {
            return None;
        }
        self.health = (self.health - damage).max(0.);
        if self.health == 0. {
            self.state(State::Dead);
            Some("sound/character/boojum/death.wav")
        } else {
            if self.state != State::Pain {
                self.state(State::Pain);
            }
            Some("sound/character/boojum/pain01.wav")
        }
    }
    pub fn clip(&self, timing: Timing) -> (&'static str, f32, bool) {
        if self.frozen {
            return ("death_frozen", self.time.min(timing.frame * 6.), false);
        }
        match self.state {
            State::Fly => ("fly", self.time, true),
            State::Scream => ("attack_scream", self.time, false),
            State::Pain => ("pain1", self.time, false),
            State::Dead if self.time < timing.death => ("death_part01", self.time, false),
            State::Dead => ("death_part02", self.time - timing.death, true),
        }
    }
    pub fn advance(&mut self, dt: f32, world: &World, eye: Vec3, timing: Timing) -> Feedback {
        let mut f = Feedback::default();
        if !self.active || !dt.is_finite() || dt <= 0. {
            return f;
        }
        self.accumulator += dt.min(0.1) as f64;
        while self.accumulator + 1e-9 >= 1. / 120. {
            self.accumulator -= 1. / 120.;
            self.step(1. / 120., world, eye, timing, &mut f);
        }
        f
    }
    fn step(&mut self, dt: f32, world: &World, eye: Vec3, timing: Timing, out: &mut Feedback) {
        self.electric = (self.electric - dt).max(0.);
        self.feet += self.recoil.step(dt, world, self.target(0));
        self.time += dt;
        self.wait = (self.wait - dt).max(0.);
        let player = self.opponents.bodies(eye);
        self.waves.retain_mut(|w| {
            let next = w.position + w.direction * w.speed * dt;
            let ctx = combat::Context {
                world,
                targets: &player,
            };
            if let Some((id, _)) = combat::contact(&ctx, w.position, next, 5.) {
                out.strike(
                    id,
                    w.damage,
                    w.direction * if w.damage >= 7. { 300. } else { 80. },
                    combat::DamageKind::Other,
                );
                return false;
            }
            let hit = world.sweep(w.position, next, Vec3::splat(5.));
            w.position = w.position.lerp(next, hit.fraction);
            w.age += dt;
            !hit.start_solid && hit.fraction >= 1. && w.age < 1.
        });
        if self.state == State::Dead {
            self.fall = (self.fall + 400. * dt).min(400.);
            let center = self.target(0).center;
            let end = center - Vec3::Z * self.fall * dt;
            let t = world.sweep(center, end, self.target(0).half);
            if !t.start_solid {
                self.feet -= Vec3::Z * self.fall * dt * t.fraction;
            }
            if t.fraction < 1. {
                self.fall = 0.;
            }
            return;
        }
        let center = self.target(0).center;
        let (eye, notarget, _) = self.opponents.aim(
            world,
            center,
            eye,
            self.notarget,
            self.state == State::Scream,
        );
        let delta = eye - center;
        let distance = delta.length();
        let sight = world.sweep(center, eye, Vec3::splat(0.5));
        let visible = !notarget && !sight.start_solid && sight.fraction >= 1. && distance < 1200.;
        let angle = delta.y.atan2(delta.x);
        let turn = (angle - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        if !notarget {
            self.yaw += turn.clamp(-4. * dt, 4. * dt);
        } else if self.state == State::Scream {
            self.state(State::Fly);
            self.wait = 0.35;
        }
        match self.state {
            State::Pain => {
                if self.time >= timing.pain {
                    self.state(State::Fly);
                    self.wait = 0.35;
                }
            }
            State::Scream => {
                // Reviewed source model events: five small screams, then one wave.
                while self.next_shot < 6
                    && self.time >= timing.frame * (16. + 2. * self.next_shot as f32)
                {
                    let big = self.next_shot == 5;
                    let muzzle = center + Vec3::Z * 15.;
                    self.waves.push(Wave {
                        position: muzzle,
                        direction: (eye - muzzle).normalize_or_zero(),
                        age: 0.,
                        speed: if big { 900. } else { 600. },
                        damage: if big { 7. } else { 2. },
                    });
                    self.next_shot += 1;
                    self.shots += 1;
                }
                if self.time >= timing.attack {
                    self.state(State::Fly);
                    self.wait = 1.25;
                }
            }
            State::Fly => {
                if visible
                    && (100.0..=300.).contains(&distance)
                    && turn.abs() < 0.3
                    && self.wait == 0.
                {
                    self.state(State::Scream);
                    self.attacks += 1;
                    out.sounds.push("sound/character/boojum/attack.wav");
                    return;
                }
                let direction = delta.normalize_or_zero();
                let side =
                    vec3(-direction.y, direction.x, 0.) * (if self.drift < 1. { 1. } else { -1. });
                let desired = if visible {
                    direction
                        * if distance > 240. {
                            140.
                        } else if distance < 140. {
                            -100.
                        } else {
                            0.
                        }
                        + side * 45.
                        + Vec3::Z * ((eye.z + 30. - center.z) * 2.).clamp(-65., 65.)
                } else if !notarget && distance < 1200. {
                    direction * 100.
                } else {
                    (self.home - self.feet).normalize_or_zero() * 50.
                };
                let end = center + desired * dt;
                let half = self.target(0).half;
                let t = world.sweep(center, end, half);
                if !t.start_solid {
                    self.feet += desired * dt * t.fraction;
                    if t.fraction < 1. {
                        let along = desired - t.normal * desired.dot(t.normal).min(0.);
                        let a = self.target(0).center;
                        let b = a + along * dt * (1. - t.fraction);
                        let hit = world.sweep(a, b, half);
                        if !hit.start_solid {
                            self.feet += along * dt * (1. - t.fraction) * hit.fraction;
                        }
                    }
                }
            }
            State::Dead => {}
        }
    }
    pub fn draw_waves(&self) {
        for w in &self.waves {
            let right = w.direction.cross(Vec3::Z).normalize_or_zero();
            let up = right.cross(w.direction);
            let radius = 4. + w.age * 40.;
            let color = Color::new(0.65, 0.85, 1., (1. - w.age) * 0.6);
            for i in 0..16 {
                let a = i as f32 * std::f32::consts::TAU / 16.;
                let b = (i + 1) as f32 * std::f32::consts::TAU / 16.;
                crate::render_fx::line(
                    w.position + (right * a.cos() + up * a.sin()) * radius,
                    w.position + (right * b.cos() + up * b.sin()) * radius,
                    color,
                );
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn screams_target_demons_even_when_alice_is_notarget() {
        let world = World::fixture(&[]);
        let mut b = Boojum::new(Vec3::ZERO, 0.);
        b.active = true;
        b.notarget = true;
        b.opponents.summon = Some(Target {
            id: crate::dice::SUMMON,
            center: vec3(220., 0., 60.),
            half: vec3(40., 40., 50.),
        });
        let mut damage = 0.;
        for _ in 0..120 * 5 {
            let f = b.advance(1. / 120., &world, vec3(-700., 0., 60.), timing());
            assert_eq!(f.damage, 0.);
            damage += f.summon_hits.iter().map(|h| h.damage).sum::<f32>();
        }
        assert!(damage > 0. && b.opponents.demon);
    }
    fn timing() -> Timing {
        Timing {
            frame: 0.05,
            attack: 1.8,
            pain: 0.4,
            death: 0.7,
        }
    }
    #[test]
    fn dormant_paused_pain_and_dead_states_do_not_attack() {
        let world = World::fixture(&[]);
        let eye = vec3(220., 0., 50.);
        let mut b = Boojum::new(Vec3::ZERO, 0.);
        assert!(b.hit(35.).is_none());
        b.advance(1., &world, eye, timing());
        assert_eq!(b.time, 0.);
        b.active = true;
        b.advance(0., &world, eye, timing());
        assert_eq!(b.attacks, 0);
        assert!(b.hit(35.).is_some());
        assert_eq!(b.state, State::Pain);
        b.advance(0.1, &world, eye, timing());
        assert_eq!(b.shots, 0);
        assert!(b.hit(35.).is_some());
        assert_eq!(b.state, State::Dead);
        assert!(b.hit(35.).is_none());
        for _ in 0..50 {
            b.advance(0.1, &world, eye, timing());
        }
        assert_eq!(b.shots, 0);
        assert_eq!(b.health, 0.);
    }
    #[test]
    fn notarget_stops_pursuit_turning_and_unreleased_screams_then_resumes() {
        let world = World::fixture(&[]);
        let mut b = Boojum::new(Vec3::ZERO, 0.);
        b.active = true;
        b.notarget = true;
        b.state(State::Scream);
        for _ in 0..240 {
            assert_eq!(
                b.advance(1. / 120., &world, vec3(0., 240., 50.), timing())
                    .damage,
                0.
            );
        }
        assert_eq!((b.feet, b.yaw, b.shots), (Vec3::ZERO, 0., 0));
        b.notarget = false;
        for _ in 0..600 {
            b.advance(1. / 120., &world, vec3(240., 0., 50.), timing());
        }
        assert!(b.attacks > 0 && b.shots > 0);
    }
    #[test]
    fn six_screams_collide_and_damage_independent_of_display_rate() {
        let world = World::fixture(&[]);
        let eye = vec3(240., 0., 50.);
        let mut results = Vec::new();
        for fps in [30, 60, 144] {
            let mut b = Boojum::new(Vec3::ZERO, 0.);
            b.active = true;
            let mut damage = 0.;
            let mut impulse = Vec3::ZERO;
            for _ in 0..fps * 3 {
                let f = b.advance(1. / fps as f32, &world, eye, timing());
                damage += f.damage;
                impulse += f.impulse;
            }
            assert_eq!(b.attacks, 1);
            assert_eq!(b.shots, 6);
            assert_eq!(damage, 17.);
            assert!(
                impulse.x > 600.,
                "The large wave and five smaller screams must push Alice"
            );
            results.push((b.feet, damage));
        }
        for r in &results[1..] {
            assert!(r.0.distance(results[0].0) < 0.01);
            assert_eq!(r.1, results[0].1);
        }
    }
    #[test]
    fn walls_stop_screams_and_flight_while_a_moving_player_can_dodge() {
        let wall = World::fixture(&[(vec3(90., -200., -200.), vec3(100., 200., 200.))]);
        let eye = vec3(240., 0., 50.);
        let mut b = Boojum::new(Vec3::ZERO, 0.);
        b.active = true;
        b.state(State::Scream);
        let mut damage = 0.;
        for _ in 0..600 {
            damage += b.advance(1. / 120., &wall, eye, timing()).damage;
        }
        assert_eq!(damage, 0.);
        assert!(b.feet.x < 66.1);
        let empty = World::fixture(&[]);
        let mut b = Boojum::new(Vec3::ZERO, 0.);
        b.active = true;
        for _ in 0..110 {
            b.advance(1. / 120., &empty, eye, timing());
        }
        assert!(b.shots > 0);
        b.hit(70.);
        let mut damage = 0.;
        for _ in 0..240 {
            damage += b
                .advance(1. / 120., &empty, eye + Vec3::Y * 200., timing())
                .damage;
        }
        assert_eq!(damage, 0.);
    }
}

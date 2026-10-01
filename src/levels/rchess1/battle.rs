use super::*;
use crate::{
    ant::Timing,
    combat::{self, DamageKind, Recoil},
};
pub const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Idle,
    Walk,
    Fast,
    Melee,
    Beam,
    Grenade,
    Diamond,
    Shield,
    Pain,
    Dead,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn king() -> Boss {
        Boss::new(Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        })
    }
    fn hit(kind: DamageKind, direction: Vec3) -> Hit {
        Hit {
            id: BASE,
            damage: 20.,
            kind,
            knockback: direction,
        }
    }
    #[test]
    fn counter_guard_has_direction_and_an_expiring_window() {
        let mut b = king();
        b.set(Action::Shield);
        b.time = 0.3;
        b.hit(hit(DamageKind::Knife, -Vec3::X));
        assert_eq!(b.health, 1300.);
        b.hit(hit(DamageKind::Cards, Vec3::X));
        assert_eq!(b.health, 1280.);
        b.time = 0.8;
        b.hit(hit(DamageKind::Knife, -Vec3::X));
        assert_eq!(b.health, 1260.);
    }
    #[test]
    fn energy_can_punish_a_counter_and_damage_reacts_once_per_threshold() {
        let mut b = king();
        b.set(Action::Shield);
        b.time = 0.3;
        b.hit(hit(DamageKind::Ice, -Vec3::X));
        assert_eq!(b.health, 1280.);
        for _ in 0..3 {
            b.hit(hit(DamageKind::Ice, -Vec3::X));
        }
        assert_eq!(b.action, Action::Pain);
        b.time = 0.2;
        for _ in 0..4 {
            b.hit(hit(DamageKind::Ice, -Vec3::X));
        }
        assert_eq!(b.time, 0.2, "Cooldown prevents a permanent pain lock");
    }
    #[test]
    fn lethal_ice_cancels_hazards_and_keeps_its_death_variant() {
        let mut b = king();
        b.beams.push(Beam {
            from: Vec3::ZERO,
            to: Vec3::X,
            age: 0.1,
        });
        let mut h = hit(DamageKind::Ice, -Vec3::X);
        h.damage = 1300.;
        b.hit(h);
        assert!(b.frozen && b.beams.is_empty());
        assert_eq!(b.action, Action::Dead);
        b.hit(hit(DamageKind::Knife, Vec3::X));
        assert!(b.frozen);
        assert!(b.validate().is_ok());
    }
    #[test]
    fn restore_rejects_inconsistent_deaths_and_unbounded_hazards() {
        let mut b = king();
        b.health = 0.;
        assert!(b.validate().is_err());
        b.set(Action::Dead);
        assert!(b.validate().is_ok());
        for _ in 0..5 {
            b.beams.push(Beam {
                from: Vec3::ZERO,
                to: Vec3::X,
                age: 0.1,
            });
        }
        assert!(b.validate().is_err());
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Ball,
    Seeker,
    Diamond,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Shot {
    pub id: u32,
    pub kind: Kind,
    pub at: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub ended: Option<f32>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Beam {
    pub from: Vec3,
    pub to: Vec3,
    pub age: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Boss {
    pub at: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub action: Action,
    pub time: f32,
    pub frozen: bool,
    pub back: bool,
    pub shots: Vec<Shot>,
    pub beams: Vec<Beam>,
    pub serial: u32,
    pub used: u8,
    fired: bool,
    pain: f32,
    pain_wait: f32,
    shield_wait: f32,
    random: u32,
    recoil: Recoil,
}
impl Boss {
    pub fn new(p: Transform) -> Self {
        Self {
            at: p.translation,
            yaw: p.rotation.to_euler(EulerRot::ZYX).0,
            health: 1300.,
            action: Action::Idle,
            time: 0.,
            frozen: false,
            back: false,
            shots: vec![],
            beams: vec![],
            serial: 0,
            used: 0,
            fired: false,
            pain: 0.,
            pain_wait: 0.,
            shield_wait: 0.,
            random: 0x51ab22,
            recoil: Default::default(),
        }
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.at,
            rotation: Quat::from_rotation_z(self.yaw),
        }
    }
    pub fn target(&self) -> Target {
        Target {
            id: BASE,
            center: self.at + Vec3::Z * 64.,
            half: vec3(24., 24., 64.),
        }
    }
    pub fn clip(&self) -> &'static str {
        match self.action {
            Action::Idle => "idle",
            Action::Walk => "walk_1",
            Action::Fast => "walk_2",
            Action::Melee => "attack_1",
            Action::Beam => "attack_4",
            Action::Grenade => "attack_3",
            Action::Diamond => "attack_2",
            Action::Shield => "attack_5",
            Action::Pain => "pain1",
            Action::Dead if self.frozen => "death_frozen",
            Action::Dead if self.back => "death_back",
            Action::Dead => "death2",
        }
    }
    pub fn loops(&self) -> bool {
        matches!(self.action, Action::Idle | Action::Walk | Action::Fast)
    }
    pub fn sample_time(&self, d: &Data) -> f32 {
        if self.frozen {
            self.time.min(5. * d.boss().frame("death_frozen"))
        } else {
            self.time
        }
    }
    pub fn scale(&self, d: &Data) -> f32 {
        if self.health > 0. {
            1.
        } else {
            (1. - (self.time - d.boss().duration(self.clip()) - 1.) / 2.).clamp(0., 1.)
        }
    }
    pub fn done(&self, d: &Data) -> bool {
        self.health == 0. && self.scale(d) == 0.
    }
    pub fn set(&mut self, a: Action) {
        self.action = a;
        self.time = 0.;
        self.fired = false;
    }
    fn chance(&mut self) -> f32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 8) as f32 / 16777216.
    }
    pub fn ranged(&self) -> Action {
        // Ordered descending HEALTH conditions are treated as lower bounds; this keeps all three branches reachable.
        if self.health >= 1000. {
            Action::Beam
        } else if self.health >= 500. {
            Action::Grenade
        } else {
            Action::Diamond
        }
    }
    pub fn hit(&mut self, h: Hit) {
        if self.health <= 0. || !h.damage.is_finite() || h.damage <= 0. {
            return;
        }
        let front = h
            .knockback
            .truncate()
            .normalize_or_zero()
            .dot(vec2(self.yaw.cos(), self.yaw.sin()))
            < -0.5;
        if self.action == Action::Shield
            && self.time > 0.1
            && self.time < 0.75
            && front
            && matches!(h.kind.means(), DamageKind::Knife | DamageKind::Cards)
        {
            return;
        }
        self.health = (self.health - h.damage).max(0.);
        self.recoil.hit(h.knockback, 250.);
        if self.health == 0. {
            self.frozen = h.kind.means() == DamageKind::Ice;
            self.back = !front;
            self.set(Action::Dead);
            self.shots.clear();
            self.beams.clear();
            return;
        }
        self.pain = (self.pain + h.damage).min(1300.);
        if self.pain >= 65. && self.pain_wait == 0. {
            self.pain = 0.;
            self.pain_wait = 1.;
            self.set(Action::Pain);
        }
    }
    fn emit(&mut self, kind: Kind, eye: Vec3, d: &Data) {
        if self.shots.len() >= 16 {
            return;
        }
        let at = d
            .boss()
            .tag(
                if kind == Kind::Diamond {
                    "tag_barrel"
                } else {
                    "tag_ball"
                },
                self.clip(),
                self.time,
                self.pose(),
                false,
            )
            .translation;
        let speed = match kind {
            Kind::Ball => 500.,
            Kind::Seeker => 300.,
            Kind::Diamond => 700.,
        };
        let mut velocity = (eye - at).normalize_or_zero() * speed;
        if kind == Kind::Ball {
            velocity.z += 320. * (eye - at).truncate().length() / speed;
        }
        self.serial = self.serial.wrapping_add(1);
        self.shots.push(Shot {
            id: self.serial,
            kind,
            at,
            velocity,
            age: 0.,
            ended: None,
        });
    }
    pub fn step(
        &mut self,
        w: &World,
        eye: Vec3,
        notarget: bool,
        incoming: bool,
        d: &Data,
        out: &mut Feedback,
    ) {
        let old = self.time;
        self.time = (self.time + STEP).min(60.);
        self.pain_wait = (self.pain_wait - STEP).max(0.);
        self.shield_wait = (self.shield_wait - STEP).max(0.);
        self.projectiles(w, eye, out);
        for b in &mut self.beams {
            b.age += STEP;
        }
        self.beams.retain(|b| b.age < 0.8);
        let rig = d.boss();
        let clip = self.clip();
        let duration = rig.duration(clip);
        for cue in rig.audio.between(
            clip,
            crate::audio::events::Span {
                start: old,
                end: self.time,
                duration,
                frame_time: rig.frame(clip),
                looping: self.loops(),
                entered: old == 0.,
            },
        ) {
            out.cue_sounds.push((cue.path, self.at));
        }
        if self.health == 0. {
            return;
        }
        self.at += self.recoil.step(STEP, &d.arena, self.target());
        let to = eye - (self.at + Vec3::Z * 96.);
        let distance = to.truncate().length();
        let trace = w.sweep(self.at + Vec3::Z * 112., eye, Vec3::ZERO);
        let clear = !notarget && distance < 800. && !trace.start_solid && trace.fraction >= 1.;
        if clear && matches!(self.action, Action::Idle | Action::Walk | Action::Fast) {
            let delta = (to.y.atan2(to.x) - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += delta.clamp(-10. * STEP, 10. * STEP);
            if incoming && self.shield_wait == 0. {
                self.set(Action::Shield);
                self.shield_wait = 2.;
                self.used |= 8;
                return;
            }
            if distance < 110. {
                self.set(Action::Melee);
                return;
            }
        }
        let crossed = |f: f32| old < f * rig.frame(clip) && self.time >= f * rig.frame(clip);
        if self.action == Action::Melee
            && !self.fired
            && (crossed(8.) || crossed(9.))
            && clear
            && distance < 130.
            && to.z.abs() < 100.
            && vec2(self.yaw.cos(), self.yaw.sin()).dot(to.truncate().normalize_or_zero()) > 0.5
        {
            // The two authored contact frames share one damage-once window.
            out.damage += 10.;
            self.fired = true;
        }
        if clear {
            match self.action {
                Action::Diamond if crossed(10.) => {
                    self.emit(Kind::Diamond, eye, d);
                    self.used |= 4;
                }
                Action::Grenade if crossed(15.) => {
                    self.emit(Kind::Ball, eye, d);
                    self.used |= 2;
                }
                Action::Shield if crossed(5.) => self.emit(Kind::Seeker, eye, d),
                Action::Beam if [20., 23., 26., 29.].iter().any(|f| crossed(*f)) => {
                    let from = rig
                        .tag("tag_ball", clip, self.time, self.pose(), false)
                        .translation;
                    let end = from + (eye - from).normalize_or_zero() * 500.;
                    let tr = w.sweep(from, end, Vec3::ZERO);
                    let to = from.lerp(end, tr.fraction);
                    if combat::contact(
                        &combat::Context {
                            world: w,
                            targets: &[Target {
                                id: 0,
                                center: eye - Vec3::Z * 20.,
                                half: crate::collision::PLAYER_HALF,
                            }],
                        },
                        from,
                        end,
                        2.,
                    )
                    .is_some()
                    {
                        out.damage += 2.;
                    }
                    if self.beams.len() < 4 {
                        self.beams.push(Beam { from, to, age: 0. });
                    }
                    self.used |= 1;
                }
                _ => {}
            }
        }
        if clear && matches!(self.action, Action::Walk | Action::Fast) && distance > 100. {
            self.at = combat::walk_body(
                &d.arena,
                self.at,
                to.truncate().normalize_or_zero().extend(0.)
                    * d.speed("c_chess_red_king", clip).clamp(30., 200.)
                    * STEP,
                vec3(24., 24., 64.),
            );
        }
        if self.time >= duration {
            let next = if !clear {
                Action::Idle
            } else if self.chance() < if distance < 400. { 0.7 } else { 0.4 } {
                self.ranged()
            } else if distance < 400. {
                Action::Walk
            } else {
                Action::Fast
            };
            self.set(next);
        }
    }
    pub fn projectiles(&mut self, w: &World, eye: Vec3, out: &mut Feedback) {
        let target = Target {
            id: 0,
            center: eye - Vec3::Z * 20.,
            half: crate::collision::PLAYER_HALF,
        };
        self.shots.retain_mut(|s| {
            s.age += STEP;
            if let Some(end) = s.ended {
                return s.age - end < 1.5;
            }
            if s.kind == Kind::Ball {
                s.velocity.z -= 640. * STEP;
            }
            if s.kind == Kind::Seeker {
                let desired = (target.center - s.at).normalize_or_zero();
                let dir = s.velocity.normalize_or_zero();
                let angle = dir.dot(desired).clamp(-1., 1.).acos();
                if angle > 0.0001 {
                    s.velocity = dir
                        .lerp(desired, (90_f32.to_radians() * STEP / angle).min(1.))
                        .normalize_or_zero()
                        * 300.;
                }
            }
            let next = s.at + s.velocity * STEP;
            let wall = w.sweep(s.at, next, Vec3::splat(8.));
            let direct = combat::contact_box(
                &combat::Context {
                    world: w,
                    targets: &[target],
                },
                s.at,
                next,
                Vec3::splat(8.),
            );
            let expired = s.age >= if s.kind == Kind::Diamond { 5. } else { 2. };
            if let Some((_, f)) = direct {
                s.at = s.at.lerp(next, f);
                out.damage += if s.kind == Kind::Diamond { 7. } else { 5. };
                if s.kind == Kind::Diamond {
                    out.impulse += s.velocity.normalize_or_zero() * 150.;
                }
                s.ended = Some(s.age);
            } else if expired || wall.start_solid || (wall.fraction < 1. && s.kind != Kind::Ball) {
                s.at = s.at.lerp(next, wall.fraction);
                s.ended = Some(s.age);
            } else if wall.fraction < 1. {
                s.at = s.at.lerp(next, wall.fraction) + wall.normal * 0.1;
                s.velocity = (s.velocity - 2. * wall.normal * s.velocity.dot(wall.normal)) * 0.6;
            } else {
                s.at = next;
            }
            if s.ended.is_some() {
                if s.kind != Kind::Diamond {
                    out.spatial_sounds
                        .push(("sound/character/army_ant_corp/grenade.wav", s.at));
                    // Shared Explosion convention: radius defaults to damage + 60; direct victims do not take it twice.
                    let offset = target.center - s.at;
                    let distance = offset.length();
                    let ray = w.sweep(s.at, target.center, Vec3::ZERO);
                    if direct.is_none() && distance < 150. && !ray.start_solid && ray.fraction >= 1.
                    {
                        let f = 1. - distance / 150.;
                        out.damage += 90. * f;
                        out.impulse += offset.normalize_or_zero() * 100. * f;
                    }
                } else {
                    out.spatial_sounds
                        .push(("sound/weapon/cards/cards_hit_world1.wav", s.at));
                }
            }
            true
        });
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.at.is_finite()
                && self.at.abs().max_element() < 20000.
                && self.yaw.is_finite()
                && (0. ..=1300.).contains(&self.health)
                && self.recoil.valid(),
            "Invalid Red King body"
        );
        for (n, t, m) in [
            ("action", self.time, 60.),
            ("pain", self.pain, 1300.),
            ("pain wait", self.pain_wait, 1.),
            ("shield wait", self.shield_wait, 2.),
        ] {
            state::clock(n, t, m)?;
        }
        ensure!(
            (self.health == 0.) == (self.action == Action::Dead)
                && (!self.frozen || self.health == 0.)
                && self.used < 16
                && self.shots.len() <= 16
                && self.beams.len() <= 4,
            "Invalid King state"
        );
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.shots {
            ensure!(
                ids.insert(s.id)
                    && s.at.is_finite()
                    && s.at.abs().max_element() < 100000.
                    && s.velocity.is_finite()
                    && s.velocity.length() < 5000.
                    && (0. ..=7.).contains(&s.age)
                    && s.ended
                        .is_none_or(|t| t.is_finite() && t >= 0. && t <= s.age),
                "Invalid King projectile"
            );
        }
        for b in &self.beams {
            ensure!(
                b.from.is_finite() && b.to.is_finite() && (0. ..0.8).contains(&b.age),
                "Invalid King beam"
            );
        }
        Ok(())
    }
}

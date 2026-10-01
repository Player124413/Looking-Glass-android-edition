use super::*;
use crate::combat::{self, Feedback, Hit, Target};
pub(super) const STEP: f32 = 1. / 120.;
pub(super) const CLIPS: &[&str] = &[
    "idle1",
    "alert1",
    "takeoff",
    "fly_1",
    "fly_2",
    "fly_glide",
    "fly_idle1",
    "fly_attack_breath",
    "fly_attack_dive",
    "fly_attack_dive_hit",
    "fly_landing",
    "fly_pain1",
    "ready_idle1",
    "walk_1",
    "attack1d",
    "attack2",
    "attack3",
    "attack4a",
    "attack4b",
    "attack4c",
    "attack5a",
    "attack5b",
    "attack5c",
    "jump",
    "fall",
    "land",
    "pain1",
    "death",
    "eyeloss",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Action {
    Ready,
    Walk,
    Alert,
    Takeoff,
    Fly,
    FlyFire,
    Dive,
    DiveHit,
    Descend,
    Land,
    Punch,
    Bite,
    Spit,
    FireReady,
    Fire,
    FireEnd,
    BeamReady,
    Beam,
    BeamEnd,
    Jump,
    Fall,
    JumpLand,
    Pain,
    Dead,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Shot {
    pub at: Vec3,
    pub start: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub ended: Option<f32>,
    pub spiral: bool,
    pub bounced: bool,
    pub id: u32,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Boss {
    pub at: Vec3,
    pub home: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub flying: bool,
    pub landed: bool,
    pub action: Action,
    pub time: f32,
    pub hold: f32,
    pub velocity: Vec3,
    pub aim: Vec3,
    pub beam: Option<(Vec3, Vec3)>,
    pub shots: Vec<Shot>,
    pub serial: u32,
    #[serde(skip)]
    pub freeze_spirals: bool,
    random: u32,
    pulse: f32,
    pain_wait: f32,
    contact: bool,
}
impl Boss {
    pub fn new(pose: Transform, kind: Kind) -> Self {
        Self {
            at: pose.translation,
            home: pose.translation,
            yaw: pose.rotation.to_euler(EulerRot::ZYX).0,
            health: if kind == Kind::Lair { 99999. } else { 2000. },
            flying: false,
            landed: kind == Kind::Lair,
            action: if kind == Kind::Lair {
                Action::Ready
            } else {
                Action::Alert
            },
            time: 0.,
            hold: 1.,
            velocity: Vec3::ZERO,
            aim: Vec3::ZERO,
            beam: None,
            shots: Vec::new(),
            serial: 0,
            freeze_spirals: false,
            random: 0x7abbe12,
            pulse: 0.,
            pain_wait: 0.,
            contact: false,
        }
    }
    pub fn set(&mut self, a: Action) {
        self.action = a;
        self.time = 0.;
        self.contact = false;
        self.serial = self.serial.wrapping_add(1);
        if !matches!(a, Action::Beam | Action::BeamEnd) {
            self.beam = None;
        }
    }
    fn rand(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        (self.random >> 8) as f32 / 16777216.
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.at,
            rotation: Quat::from_rotation_z(self.yaw),
        }
    }
    pub fn clip(&self) -> &'static str {
        match self.action {
            Action::Ready => "ready_idle1",
            Action::Walk => "walk_1",
            Action::Alert => "alert1",
            Action::Takeoff => "takeoff",
            Action::Fly => "fly_1",
            Action::FlyFire => "fly_attack_breath",
            Action::Dive => "fly_attack_dive",
            Action::DiveHit => "fly_attack_dive_hit",
            Action::Descend => "fly_idle1",
            Action::Land => "fly_landing",
            Action::Punch => "attack1d",
            Action::Bite => "attack2",
            Action::Spit => "attack3",
            Action::FireReady => "attack4a",
            Action::Fire => "attack4b",
            Action::FireEnd => "attack4c",
            Action::BeamReady => "attack5a",
            Action::Beam => "attack5b",
            Action::BeamEnd => "attack5c",
            Action::Jump => "jump",
            Action::Fall => "fall",
            Action::JumpLand => "land",
            Action::Pain => {
                if self.flying {
                    "fly_pain1"
                } else {
                    "pain1"
                }
            }
            Action::Dead => "death",
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.action,
            Action::Ready
                | Action::Walk
                | Action::Fly
                | Action::FlyFire
                | Action::Fire
                | Action::Beam
                | Action::Descend
                | Action::Fall
        )
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.at + Vec3::Z * if self.flying { -19.5 } else { 114. },
            half: vec3(72., 72., if self.flying { 127.5 } else { 114. }),
        }
    }
    pub fn hit(&mut self, h: Hit, kind: Kind) -> Option<&'static str> {
        if self.health <= 0. || !h.damage.is_finite() || h.damage <= 0. {
            return None;
        }
        if kind == Kind::Grounds {
            self.health = (self.health - h.damage).max(0.);
        }
        if self.health <= 0. {
            self.set(Action::Dead);
            self.shots.clear();
            // The death scene clock owns this cue, including restore suppression.
            return None;
        }
        if self.flying
            && self.health <= 1000.
            && !matches!(self.action, Action::Descend | Action::Land)
        {
            self.set(Action::Descend);
        } else if h.damage >= 80.
            && self.pain_wait <= 0.
            && !matches!(
                self.action,
                Action::Descend | Action::Land | Action::Jump | Action::Fall
            )
        {
            self.set(Action::Pain);
            self.pain_wait = 1.;
        }
        Some("sound/character/jabberwock/pain1.wav")
    }
    fn move_air(&mut self, w: &World, delta: Vec3) {
        let b = self.target(0);
        let tr = w.sweep(b.center, b.center + delta, b.half);
        if !tr.start_solid {
            self.at += delta * tr.fraction;
        }
    }
    fn melee(&mut self, w: &World, eye: Vec3, damage: f32, push: f32, out: &mut Feedback) {
        let start = self.at + Vec3::Z * 54.;
        let end = start + Quat::from_rotation_z(self.yaw) * Vec3::X * 190.;
        let target = Target {
            id: 0,
            center: eye - Vec3::Z * 20.,
            half: crate::collision::PLAYER_HALF,
        };
        if combat::contact_box(
            &combat::Context {
                world: w,
                targets: &[target],
            },
            start,
            end,
            vec3(16., 16., 32.),
        )
        .is_some()
        {
            out.damage += damage;
            out.impulse += (eye - start).normalize_or_zero() * push;
        }
    }
    pub(super) fn emit(&mut self, d: &Data, eye: Vec3, spiral: bool) {
        if self.shots.len() >= 128 {
            return;
        }
        let mouth = d.boss().tag(
            "tag_breath",
            self.clip(),
            self.time,
            self.pose(),
            self.loops(),
        );
        let at = mouth.translation;
        let velocity = if spiral {
            (eye - at).normalize_or_zero() * 1000.
        } else {
            // Breath follows the animated mouth, including its pitch and sweep.
            // Gravity bends it after release; aiming an extra lob at Alice made
            // fire leave sideways or upward while the head faced elsewhere.
            mouth.rotation * Vec3::X * 400.
        };
        self.serial = self.serial.wrapping_add(1);
        self.shots.push(Shot {
            at,
            start: at,
            velocity,
            age: 0.,
            ended: None,
            spiral,
            bounced: false,
            id: self.serial,
        });
    }
    pub fn step(
        &mut self,
        kind: Kind,
        w: &World,
        eye: Vec3,
        ignore: bool,
        d: &Data,
        out: &mut Feedback,
    ) {
        let dt = STEP;
        let before = self.time;
        self.time += dt;
        self.pain_wait = (self.pain_wait - dt).max(0.);
        let duration = d.boss().duration(self.clip());
        if self.health <= 0. {
            self.time = self.time.min(duration + 1.);
            return;
        }
        let to = eye - self.at;
        let distance = to.truncate().length();
        let sight = w.sweep(self.target(0).center, eye, Vec3::splat(0.5));
        let clear = !ignore && !sight.start_solid && sight.fraction >= 1.;
        if clear
            && !matches!(
                self.action,
                Action::Beam | Action::Dive | Action::Jump | Action::Fall
            )
        {
            let wanted = to.y.atan2(to.x);
            let diff = (wanted - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += diff.clamp(-2.5 * dt, 2.5 * dt);
        }
        let frame_time = d.boss().frame(self.clip());
        let now = self.time;
        let crossed = |frame: f32| {
            let at = frame * frame_time;
            before < at && now >= at
        };
        for cue in d.boss().audio.between(
            self.clip(),
            crate::audio::events::Span {
                start: before,
                end: self.time,
                duration,
                frame_time,
                looping: self.loops(),
                entered: before == 0.,
            },
        ) {
            out.cue_sounds.push((cue.path, self.at));
        }
        match self.action {
            Action::Punch => {
                if crossed(4.) {
                    self.melee(w, eye, 5., 2., out);
                }
                if crossed(14.) {
                    self.melee(w, eye, 20., 400., out);
                }
            }
            Action::Bite => {
                if crossed(9.) {
                    self.melee(w, eye, 15., 50., out);
                }
            }
            Action::JumpLand => {
                if crossed(5.) {
                    self.melee(w, eye, 5., 2., out);
                }
            }
            Action::Spit => {
                if crossed(20.) {
                    self.emit(d, eye, true);
                }
            }
            Action::Fire | Action::FlyFire => {
                let frames = if self.action == Action::Fire {
                    &[3., 7., 11., 15., 19., 25., 29.][..]
                } else {
                    &[1., 5., 9., 13., 17.][..]
                };
                for frame in frames {
                    let cue = frame * d.boss().frame(self.clip());
                    let lo = (before / duration).floor() as u32;
                    let hi = (self.time / duration).floor() as u32;
                    for cycle in lo..=hi {
                        let at = cycle as f32 * duration + cue;
                        if before < at && self.time >= at {
                            self.emit(d, eye, false);
                        }
                    }
                }
            }
            _ => {}
        }
        if !ignore && matches!(self.action, Action::Beam | Action::BeamEnd) {
            let start = d
                .boss()
                .tag(
                    "tag_eye_emitter",
                    self.clip(),
                    self.time,
                    self.pose(),
                    self.loops(),
                )
                .translation;
            let direction = (self.aim - start).normalize_or_zero();
            let end = start + direction * 1600.;
            let wall = w.sweep(start, end, Vec3::splat(2.));
            self.beam = (!wall.start_solid).then_some((start, start.lerp(end, wall.fraction)));
            self.pulse -= dt;
            if self.pulse <= 0. {
                self.pulse += 0.25;
                let target = Target {
                    id: 0,
                    center: eye - Vec3::Z * 20.,
                    half: crate::collision::PLAYER_HALF,
                };
                if combat::contact(
                    &combat::Context {
                        world: w,
                        targets: &[target],
                    },
                    start,
                    end,
                    3.,
                )
                .is_some()
                {
                    out.damage += 5.;
                }
            }
        }
        match self.action {
            Action::Alert if self.time >= duration => {
                self.set(Action::Takeoff);
            }
            Action::Takeoff => {
                // Keep the foot-based hull until the flight origin has cleared the floor.
                self.move_air(&d.arena, Vec3::Z * (330. * dt));
                if self.at.z - self.home.z >= 148. {
                    self.flying = true;
                }
                if self.time >= duration {
                    self.set(if self.flying {
                        Action::Fly
                    } else {
                        Action::Ready
                    });
                    self.hold = 1.;
                }
            }
            Action::Fly | Action::FlyFire => {
                if clear {
                    let goal =
                        eye + Vec3::Z * 350. - to.truncate().normalize_or_zero().extend(0.) * 330.;
                    self.move_air(&d.arena, (goal - self.at).clamp_length_max(300. * dt));
                }
                if self.time >= self.hold {
                    if self.health <= 1000. {
                        self.set(Action::Descend);
                    } else if clear
                        && self.action == Action::Fly
                        && self.rand() < 0.333
                        && distance > 450.
                    {
                        self.aim = eye;
                        self.set(Action::Dive);
                    } else if clear && self.action == Action::Fly {
                        self.set(Action::FlyFire);
                        self.hold = 2. + self.rand();
                    } else {
                        self.set(Action::Fly);
                        self.hold = 2. + 2. * self.rand();
                    }
                }
            }
            Action::Dive => {
                let delta = (self.aim - self.at).clamp_length_max(750. * dt);
                let old = self.at;
                self.move_air(&d.arena, delta);
                if !self.contact
                    && self.target(0).center.distance(eye - Vec3::Z * 20.) < 150.
                    && clear
                {
                    out.damage += 25.;
                    out.impulse += to.normalize_or_zero() * 250.;
                    self.contact = true;
                }
                if self.time > 2.
                    || self.at.distance(self.aim) < 90.
                    || old.distance(self.at) < 0.001
                {
                    self.set(Action::DiveHit);
                }
            }
            Action::DiveHit if self.time >= duration => {
                self.set(Action::Fly);
                self.hold = 1.;
            }
            Action::Descend => {
                // A checked home landing keeps disengagement away from chasms.
                self.move_air(
                    &d.arena,
                    (self.home + Vec3::Z * 148. - self.at).clamp_length_max(400. * dt),
                );
                if self.at.distance(self.home + Vec3::Z * 148.) < 12. {
                    self.at = self.home;
                    self.flying = false;
                    self.landed = true;
                    self.set(Action::Land);
                }
            }
            Action::Land if self.time >= duration => self.set(Action::Ready),
            Action::Ready | Action::Walk => {
                if self.action == Action::Walk && clear && distance > 175. {
                    self.at = combat::walk_body(
                        &d.arena,
                        self.at,
                        to.truncate().normalize_or_zero().extend(0.)
                            * d.boss().clips["walk_1"].distance
                            * d.boss().def.scale
                            / duration
                            * dt,
                        vec3(72., 72., 114.),
                    );
                }
                if self.time >= duration {
                    if !clear {
                        self.set(Action::Ready);
                    } else if distance > 600. {
                        let direction = to.truncate().normalize_or_zero().extend(0.);
                        if d.arena
                            .actor_footing(
                                self.at + direction * 320. + Vec3::Z * 64.,
                                Vec3::Z * 114.,
                                vec3(72., 72., 114.),
                                192.,
                            )
                            .is_some()
                        {
                            self.velocity = direction * 400. + Vec3::Z * 320.;
                            self.set(Action::Jump);
                        } else {
                            self.set(Action::FireReady);
                        }
                    } else if distance < 180. && self.rand() < 0.2 {
                        let action = if self.rand() < 0.666 {
                            Action::Punch
                        } else {
                            Action::Bite
                        };
                        self.set(action);
                    } else if distance >= 175.
                        && self.rand() < if kind == Kind::Lair { 0.333 } else { 0.666 }
                    {
                        let choice = self.rand();
                        let action = if kind == Kind::Grounds && choice < 0.333 {
                            Action::Spit
                        } else if kind == Kind::Lair && choice > 0.666 {
                            Action::BeamReady
                        } else {
                            Action::FireReady
                        };
                        self.set(action);
                    } else {
                        self.set(if kind == Kind::Lair {
                            Action::Ready
                        } else {
                            Action::Walk
                        });
                    }
                }
            }
            Action::Jump | Action::Fall => {
                let body = self.target(0);
                let delta = self.velocity * dt;
                let trace = d.arena.sweep(body.center, body.center + delta, body.half);
                if !trace.start_solid {
                    self.at += delta * trace.fraction;
                }
                self.velocity.z -= 800. * dt;
                if self.velocity.z < 0. && trace.fraction < 1. && trace.normal.z > 0.65 {
                    self.velocity = Vec3::ZERO;
                    self.set(Action::JumpLand);
                } else if self.time > 3. {
                    self.velocity = Vec3::ZERO;
                    self.set(Action::Ready);
                } else if self.action == Action::Jump && self.time >= duration {
                    self.set(Action::Fall);
                }
            }
            Action::FireReady if self.time >= duration => {
                self.set(Action::Fire);
                self.hold = 1. + 3. * self.rand();
            }
            Action::Fire if self.time >= self.hold => self.set(Action::FireEnd),
            Action::BeamReady if self.time >= duration => {
                self.aim = eye;
                self.pulse = 0.;
                self.set(Action::Beam);
                self.hold = 1. + 2. * self.rand();
            }
            Action::Beam if self.time >= self.hold => self.set(Action::BeamEnd),
            Action::Punch
            | Action::Bite
            | Action::Spit
            | Action::FireEnd
            | Action::BeamEnd
            | Action::JumpLand
            | Action::Pain
                if self.time >= duration =>
            {
                self.set(if self.flying {
                    Action::Fly
                } else if kind == Kind::Lair {
                    Action::Ready
                } else {
                    Action::Walk
                });
                self.hold = 1.;
            }
            _ => {}
        }
        let target = Target {
            id: 0,
            center: eye - Vec3::Z * 20.,
            half: crate::collision::PLAYER_HALF,
        };
        for s in &mut self.shots {
            if s.spiral && self.freeze_spirals {
                continue;
            }
            s.age += dt;
            if s.ended.is_some() {
                continue;
            }
            let end = s.at + s.velocity * dt;
            let half = if s.spiral { 8. } else { 12. };
            let hit = if ignore {
                None
            } else {
                combat::contact(
                    &combat::Context {
                        world: w,
                        targets: &[target],
                    },
                    s.at,
                    end,
                    half,
                )
            };
            let wall = w.sweep(s.at, end, Vec3::splat(half));
            if let Some((_, f)) = hit {
                out.damage += if s.spiral { 150. } else { 5. };
                out.impulse += s.velocity.normalize_or_zero() * if s.spiral { 400. } else { 20. };
                s.at = s.at.lerp(end, f);
                s.ended = Some(s.age);
            } else if wall.start_solid {
                s.ended = Some(s.age);
            } else {
                s.at = s.at.lerp(end, wall.fraction);
                if wall.fraction < 1. {
                    if !s.spiral && !s.bounced {
                        s.velocity -= 2. * wall.normal * s.velocity.dot(wall.normal);
                        s.velocity *= 0.5;
                        s.bounced = true;
                    } else {
                        s.ended = Some(s.age);
                    }
                }
            }
            if !s.spiral {
                s.velocity.z -= 800. * dt;
            }
            if s.age >= if s.spiral { 2.5 } else { 3. } {
                s.ended.get_or_insert(s.age);
            }
            if s.spiral && s.ended == Some(s.age) {
                out.spatial_sounds
                    .push(("sound/weapon/staff/explode1.wav", s.at));
                let offset = target.center - s.at;
                let fraction = (1. - offset.length() / 400.).max(0.);
                let sight = w.sweep(s.at, target.center, Vec3::ZERO);
                if !ignore
                    && hit.is_none()
                    && fraction > 0.
                    && !sight.start_solid
                    && sight.fraction >= 1.
                {
                    out.damage += 100. * fraction;
                    out.impulse += offset.normalize_or_zero() * 400. * fraction;
                }
            }
        }
        self.shots
            .retain(|s| s.ended.is_none_or(|t| s.age - t < 1.5));
    }
    pub fn validate(&self, kind: Kind) -> Result<()> {
        ensure!(
            self.at.is_finite()
                && self.home.is_finite()
                && self.at.abs().max_element() < 100000.
                && self.home.abs().max_element() < 100000.
                && self.yaw.is_finite()
                && self.velocity.is_finite()
                && self.velocity.length() < 4000.
                && self.aim.is_finite(),
            "Invalid boss transform"
        );
        ensure!(
            self.health.is_finite()
                && if kind == Kind::Lair {
                    self.health == 99999.
                } else {
                    (0. ..=2000.).contains(&self.health)
                },
            "Invalid boss health"
        );
        ensure!(
            self.time.is_finite()
                && (0. ..=120.).contains(&self.time)
                && self.hold.is_finite()
                && (0. ..=5.).contains(&self.hold)
                && self.pulse.is_finite()
                && self.pain_wait.is_finite()
                && self.shots.len() <= 128,
            "Invalid boss clock/resources"
        );
        ensure!(!self.landed || !self.flying, "Invalid boss flight phase");
        ensure!(
            self.beam
                .is_none_or(|(a, b)| a.is_finite() && b.is_finite() && a.distance(b) <= 1601.),
            "Invalid eye beam"
        );
        for s in &self.shots {
            ensure!(
                s.at.is_finite()
                    && s.start.is_finite()
                    && s.velocity.is_finite()
                    && s.velocity.length() < 10000.
                    && s.age.is_finite()
                    && (0. ..=4.6).contains(&s.age)
                    && s.ended
                        .is_none_or(|e| e.is_finite() && e >= 0. && e <= s.age),
                "Invalid breath carrier"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn boss(kind: Kind) -> Boss {
        Boss::new(
            Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            kind,
        )
    }
    fn hit(damage: f32) -> Hit {
        Hit {
            id: 0,
            damage,
            kind: combat::DamageKind::Blunderbuss,
            knockback: Vec3::ZERO,
        }
    }
    #[test]
    fn lair_is_survival_and_cannot_be_killed_by_any_weapon() {
        let mut b = boss(Kind::Lair);
        b.hit(hit(999999.), Kind::Lair);
        assert_eq!(b.health, 99999.);
        assert_ne!(b.action, Action::Dead);
        b.validate(Kind::Lair).unwrap();
    }
    #[test]
    fn half_health_landing_takes_priority_over_pain() {
        let mut b = boss(Kind::Grounds);
        b.flying = true;
        b.set(Action::FlyFire);
        b.hit(hit(1000.), Kind::Grounds);
        assert_eq!(b.action, Action::Descend);
        b.hit(hit(100.), Kind::Grounds);
        assert_eq!(b.action, Action::Descend);
    }
    #[test]
    fn death_is_final_and_retires_attacks() {
        let mut b = boss(Kind::Grounds);
        b.beam = Some((Vec3::ZERO, Vec3::X));
        assert!(b.hit(hit(2000.), Kind::Grounds).is_none());
        let serial = b.serial;
        b.hit(hit(10.), Kind::Grounds);
        assert_eq!(b.serial, serial);
        assert_eq!(b.action, Action::Dead);
        assert!(b.beam.is_none() && b.shots.is_empty());
        b.validate(Kind::Grounds).unwrap();
    }
    #[test]
    fn non_finite_hits_and_corrupt_positions_do_not_enter_simulation() {
        let mut b = boss(Kind::Grounds);
        b.hit(hit(f32::NAN), Kind::Grounds);
        assert_eq!(b.health, 2000.);
        b.at.x = f32::INFINITY;
        assert!(b.validate(Kind::Grounds).is_err());
    }
}

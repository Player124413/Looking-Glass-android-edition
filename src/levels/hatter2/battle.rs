use super::*;
use crate::combat;
pub(super) const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Action {
    Idle,
    Ready,
    Walk,
    Run,
    Cane,
    Slap,
    RocketDraw,
    Rocket,
    RocketBack,
    CupOpen,
    Cup,
    CupClose,
    Pain,
    Stand,
    DeathStart,
    Malfunction,
    DeathEnd,
    Gone,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Shot {
    pub at: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub syringe: bool,
    pub id: u32,
    pub ended: Option<f32>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Boss {
    pub at: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub action: Action,
    pub time: f32,
    pub malfunctions: u8,
    pub shots: Vec<Shot>,
    pub serial: u32,
    pub repeats: u8,
    random: u32,
    pain: f32,
    pain_wait: f32,
}
impl Boss {
    pub fn new(p: Transform) -> Self {
        Self {
            at: p.translation,
            yaw: p.rotation.to_euler(EulerRot::ZYX).0,
            health: 2600.,
            action: Action::Idle,
            time: 0.,
            malfunctions: 0,
            shots: vec![],
            serial: 0,
            repeats: 0,
            random: 0x4a77e2,
            pain: 0.,
            pain_wait: 0.,
        }
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.at,
            rotation: Quat::from_rotation_z(self.yaw),
        }
    }
    pub fn set(&mut self, a: Action) {
        self.action = a;
        self.time = 0.;
    }
    fn rand(&mut self) -> f32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 8) as f32 / 16777216.
    }
    pub fn dying(&self) -> bool {
        matches!(
            self.action,
            Action::DeathStart | Action::Malfunction | Action::DeathEnd | Action::Gone
        )
    }
    pub fn clip(&self) -> &'static str {
        match self.action {
            Action::Idle | Action::Gone => "stand_base",
            Action::Ready => "ready",
            Action::Walk => "walk_cane",
            Action::Run => "run",
            Action::Cane => "attack_cane",
            Action::Slap => "attack_bitchslap",
            Action::RocketDraw => "attack_rockets_draw",
            Action::Rocket => "attack_rockets_fire",
            Action::RocketBack => "attack_rockets_return",
            Action::CupOpen => "attack_saucer_open",
            Action::Cup => "attack_saucer_fire",
            Action::CupClose => "attack_saucer_close",
            Action::Pain => "pain_front",
            Action::Stand => "ready_2_stand",
            Action::DeathStart => "death_start",
            Action::Malfunction => "death_malfunction",
            Action::DeathEnd => "death_end",
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.action,
            Action::Idle | Action::Ready | Action::Walk | Action::Run
        )
    }
    pub fn target(&self) -> Target {
        Target {
            id: BASE,
            center: self.at + Vec3::Z * 124.,
            half: vec3(36., 36., 124.),
        }
    }
    pub fn hit(&mut self, h: Hit) {
        if self.dying() || !h.damage.is_finite() || h.damage <= 0. {
            return;
        }
        self.health = (self.health - h.damage).max(0.);
        if self.health <= 100. {
            self.health = 100.;
            self.set(Action::DeathStart);
            self.shots.clear();
            return;
        }
        self.pain = (self.pain + h.damage).min(2600.);
        if self.pain >= 75. && self.pain_wait == 0. {
            self.pain = 0.;
            self.pain_wait = 1.;
            self.set(Action::Pain);
        }
    }
    pub fn projectile_step(
        &mut self,
        dt: f32,
        w: &World,
        eye: Vec3,
        stopped: bool,
        out: &mut Feedback,
    ) {
        let target = Target {
            id: 0,
            center: eye - Vec3::Z * 20.,
            half: crate::collision::PLAYER_HALF,
        };
        self.shots.retain_mut(|s| {
            if stopped && !s.syringe {
                return true;
            }
            s.age += dt;
            if let Some(end) = s.ended {
                return s.age - end < 0.4;
            }
            if s.syringe {
                let desired = (target.center - s.at).normalize_or_zero();
                let dir = s.velocity.normalize_or_zero();
                let angle = dir.dot(desired).clamp(-1., 1.).acos();
                if angle > 0.00001 {
                    s.velocity = dir
                        .lerp(desired, (90_f32.to_radians() * dt / angle).min(1.))
                        .normalize_or_zero()
                        * 500.;
                }
            } else {
                s.velocity.z -= 640. * dt;
            }
            let next = s.at + s.velocity * dt;
            let wall = w.sweep(s.at, next, Vec3::splat(8.));
            let hit = combat::contact_box(
                &combat::Context {
                    world: w,
                    targets: &[target],
                },
                s.at,
                next,
                Vec3::splat(8.),
            );
            if let Some((_, f)) = hit {
                s.at = s.at.lerp(next, f);
                s.ended = Some(s.age);
                out.damage += if s.syringe { 20. } else { 10. };
            } else if wall.start_solid
                || wall.fraction < 1.
                || s.age >= if s.syringe { 5. } else { 3. }
            {
                s.at = s.at.lerp(next, wall.fraction);
                s.ended = Some(s.age);
            } else {
                s.at = next;
            }
            true
        });
    }
    pub fn step(
        &mut self,
        w: &World,
        eye: Vec3,
        active: bool,
        mobile: bool,
        notarget: bool,
        d: &Data,
        out: &mut Feedback,
    ) {
        if self.action == Action::Gone {
            return;
        }
        let dt = STEP;
        let clip = self.clip();
        let rig = d.boss();
        let duration = rig.duration(clip);
        let old = self.time;
        self.time += dt;
        self.pain_wait = (self.pain_wait - dt).max(0.);
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
        if self.dying() {
            if self.time >= duration {
                match self.action {
                    Action::DeathStart => self.set(Action::Malfunction),
                    Action::Malfunction => {
                        self.malfunctions += 1;
                        if self.malfunctions == 3 {
                            self.health = 0.;
                            self.set(Action::DeathEnd);
                        } else {
                            self.set(Action::Malfunction);
                        }
                    }
                    Action::DeathEnd => self.set(Action::Gone),
                    _ => {}
                }
            }
            return;
        }
        if !active {
            if self.action == Action::Idle {
                self.time = self.time % duration;
            }
            if self.action != Action::Idle && self.action != Action::Stand {
                self.set(Action::Idle);
            }
            if self.action == Action::Stand && self.time >= duration {
                self.set(Action::Idle);
            }
            return;
        }
        let to = eye - (self.at + Vec3::Z * 100.);
        let distance = to.truncate().length();
        let trace = w.sweep(self.at + Vec3::Z * 110., eye, Vec3::ZERO);
        let clear = !notarget && !trace.start_solid && trace.fraction >= 1.;
        if clear
            && matches!(
                self.action,
                Action::Idle
                    | Action::Ready
                    | Action::Walk
                    | Action::Run
                    | Action::RocketDraw
                    | Action::CupOpen
            )
        {
            let yaw = to.y.atan2(to.x);
            let delta = (yaw - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += delta.clamp(-6. * dt, 6. * dt);
        }
        let event = match self.action {
            Action::Cane => Some(12.),
            Action::Slap => Some(14.),
            Action::Rocket => Some(1.),
            Action::Cup => Some(6.),
            _ => None,
        };
        if event.is_some_and(|f| old < f * rig.frame(clip) && self.time >= f * rig.frame(clip))
            && clear
        {
            if matches!(self.action, Action::Cane | Action::Slap) {
                let reach = if self.action == Action::Cane {
                    190.
                } else {
                    150.
                };
                if distance < reach
                    && to.z.abs() < 150.
                    && vec2(self.yaw.cos(), self.yaw.sin()).dot(to.truncate().normalize_or_zero())
                        > 0.5
                {
                    out.damage += 15.;
                    out.impulse += to.normalize_or_zero() * 300.;
                }
            } else if self.shots.len() < 24 {
                let syringe = self.action == Action::Rocket;
                let start = rig
                    .tag(
                        if syringe { "tag_finger" } else { "tag_saucer" },
                        clip,
                        self.time,
                        self.pose(),
                        false,
                    )
                    .translation;
                let mut velocity =
                    (eye - start).normalize_or_zero() * if syringe { 500. } else { 600. };
                if !syringe {
                    velocity.z += 320. * (eye - start).truncate().length() / 600.;
                }
                self.serial = self.serial.wrapping_add(1);
                self.shots.push(Shot {
                    at: start,
                    velocity,
                    age: 0.,
                    syringe,
                    id: self.serial,
                    ended: None,
                });
            }
        }
        if mobile && clear && matches!(self.action, Action::Walk | Action::Run) && distance > 135. {
            use crate::ant::Timing;
            self.at = combat::walk_body(
                &d.arena,
                self.at,
                to.truncate().normalize_or_zero().extend(0.)
                    * d.speed("c_madhatter", clip).clamp(40., 240.)
                    * dt,
                vec3(36., 36., 124.),
            );
        }
        if self.time < duration {
            return;
        }
        match self.action {
            Action::RocketDraw => {
                self.repeats = 0;
                self.set(Action::Rocket);
            }
            Action::CupOpen => {
                self.repeats = 0;
                self.set(Action::Cup);
            }
            Action::Rocket | Action::Cup => {
                self.repeats += 1;
                let again = self.repeats < 4 && self.rand() < 0.333;
                self.set(if again {
                    self.action
                } else if self.action == Action::Rocket {
                    Action::RocketBack
                } else {
                    Action::CupClose
                });
            }
            _ => {
                let next = if !clear {
                    Action::Ready
                } else if distance < 150. && mobile {
                    if self.rand() < 0.4 {
                        Action::Cane
                    } else {
                        Action::Slap
                    }
                } else if !mobile || self.rand() < 0.4 {
                    if self.rand() < 0.5 {
                        Action::CupOpen
                    } else {
                        Action::RocketDraw
                    }
                } else if distance > 400. {
                    Action::Run
                } else {
                    Action::Walk
                };
                self.set(next);
            }
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.at.is_finite() && self.at.abs().max_element() < 20000. && self.yaw.is_finite(),
            "Invalid Hatter position"
        );
        for (n, t, max) in [
            ("boss", self.time, 60.),
            ("pain", self.pain, 2600.),
            ("pain wait", self.pain_wait, 1.),
        ] {
            state::clock(n, t, max)?;
        }
        ensure!(
            (0. ..=2600.).contains(&self.health)
                && self.malfunctions <= 3
                && self.repeats <= 4
                && self.shots.len() <= 24,
            "Invalid Hatter resources"
        );
        ensure!(
            if self.dying() {
                if matches!(self.action, Action::DeathEnd | Action::Gone) {
                    self.health == 0. && self.malfunctions == 3
                } else {
                    self.health == 100. && self.malfunctions < 3
                }
            } else {
                self.health > 100.
            },
            "Invalid death sequence"
        );
        let mut ids = std::collections::BTreeSet::new();
        for s in &self.shots {
            ensure!(
                s.at.is_finite()
                    && s.at.abs().max_element() < 100000.
                    && s.velocity.is_finite()
                    && s.velocity.length() < 5000.
                    && (0. ..=6.).contains(&s.age)
                    && s.ended
                        .is_none_or(|t| t.is_finite() && t >= 0. && t <= s.age)
                    && ids.insert(s.id),
                "Invalid Hatter projectile"
            );
        }
        Ok(())
    }
}

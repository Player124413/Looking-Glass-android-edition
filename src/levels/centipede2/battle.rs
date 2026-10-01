use super::*;
use crate::{
    ant::Timing,
    burrow::{Insect, Kind},
    combat::{self, DamageKind},
};
const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Action {
    Walk,
    Ready,
    Crush,
    Spit,
    Larvae,
    Charge,
    Strike,
    Shake,
    Return,
    Pain,
    Death,
    Thrash,
    Dead,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Shot {
    pub at: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub ended: Option<f32>,
    pub start: Vec3,
    pub fragment: bool,
    pub normal: Vec3,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Battle {
    pub pose: Transform,
    pub action: Action,
    pub time: f32,
    pub hits: u8,
    pub stage: u8,
    pub cycle: u8,
    pub variant: u8,
    pub thrashes: u8,
    pub grabbed: bool,
    pub grab_camera: Vec3,
    pub grab_feet: Vec3,
    pub victim_feet: Vec3,
    pub larvae: Vec<Insect>,
    pub shots: Vec<Shot>,
    pub random: u32,
    pub accumulator: f64,
    pub serial: u32,
    pub pain_at: Vec3,
    pub blood_age: f32,
    pub decision: f32,
    pub quake: f32,
}
impl Battle {
    pub fn new(pose: Transform) -> Self {
        Self {
            pose,
            action: Action::Walk,
            time: 0.,
            hits: 0,
            stage: 1,
            cycle: 0,
            variant: 0,
            thrashes: 0,
            grabbed: false,
            grab_camera: Vec3::ZERO,
            grab_feet: Vec3::ZERO,
            victim_feet: Vec3::ZERO,
            larvae: vec![],
            shots: vec![],
            random: 0xc312f2,
            accumulator: 0.,
            serial: 0,
            pain_at: Vec3::ZERO,
            blood_age: 5.,
            decision: 1.,
            quake: 0.,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.pose.translation.is_finite()
                && self.pose.translation.abs().max_element() < 20000.
                && self.pose.rotation.is_finite()
                && (self.pose.rotation.length() - 1.).abs() < 0.01,
            "Invalid Centipede pose"
        );
        ensure!(
            self.hits <= 6
                && (1..=3).contains(&self.stage)
                && self.cycle < 3
                && self.variant < 2
                && self.thrashes <= 3,
            "Invalid boss stage"
        );
        ensure!(
            self.stage
                == ((self
                    .hits
                    .saturating_sub(u8::from(self.action == Action::Pain))
                    / 2)
                    + 1)
                .min(3),
            "Inconsistent boss hit stage"
        );
        ensure!(
            (self.hits == 6)
                == matches!(self.action, Action::Death | Action::Thrash | Action::Dead),
            "Invalid death state"
        );
        ensure!(
            !self.grabbed || self.action == Action::Shake,
            "Invalid grabbed state"
        );
        ensure!(
            self.accumulator.is_finite() && (0. ..0.009).contains(&self.accumulator),
            "Invalid boss simulation remainder"
        );
        for (name, t, max) in [
            ("action", self.time, 3600.),
            ("blood", self.blood_age, 5.),
            ("decision", self.decision, 5.),
            ("quake", self.quake, 1.),
        ] {
            state::clock(name, t, max)?;
        }
        ensure!(
            self.grab_camera.is_finite()
                && self.grab_feet.is_finite()
                && self.victim_feet.is_finite()
                && self.pain_at.is_finite(),
            "Invalid boss attachment"
        );
        ensure!(
            self.larvae.len() <= 16 && self.shots.len() <= 128,
            "Too many boss projectiles"
        );
        for l in &self.larvae {
            l.validate()?;
            ensure!(l.kind == Kind::Larva, "Invalid boss larva");
        }
        for s in &self.shots {
            ensure!(
                s.at.is_finite()
                    && s.start.is_finite()
                    && s.velocity.is_finite()
                    && s.velocity.length() < 1200.
                    && s.normal.is_finite(),
                "Invalid acid trajectory"
            );
            state::clock("acid", s.age, 12.)?;
            if let Some(t) = s.ended {
                ensure!((0. ..=s.age).contains(&t), "Invalid acid impact");
            }
        }
        Ok(())
    }
    fn roll(&mut self, n: u32) -> bool {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 16) % 100 < n
    }
    pub fn set(&mut self, a: Action) {
        self.action = a;
        self.time = 0.;
        self.serial = self.serial.wrapping_add(1);
    }
    pub fn clip(&self) -> &'static str {
        match self.action {
            Action::Walk => {
                if self.stage == 1 {
                    "walk_fast"
                } else {
                    "walk"
                }
            }
            Action::Ready => "pain_front",
            Action::Crush => "attack_crush",
            Action::Spit => "attack_spit",
            Action::Larvae => "attack_spit_buggies",
            Action::Charge => "attack_juggernaught",
            Action::Strike => "attack_grab_strike",
            Action::Shake => "attack_grab_shake",
            Action::Return => "attack_grab_return",
            Action::Pain => match self.stage {
                2 => "pain_rise_right",
                3 => "pain_rise_big",
                _ => {
                    if self.variant == 0 {
                        "pain_rise_front"
                    } else {
                        "pain_rise_big"
                    }
                }
            },
            Action::Death => "death_start",
            Action::Thrash => "death_thrash",
            Action::Dead => "death_end",
        }
    }
    pub fn weak(&self, d: &data::Data) -> bool {
        self.action == Action::Crush
            && self.time < 25. * d.boss().frame("attack_crush")
            && self.hits < 6
    }
    pub fn tag(&self, d: &data::Data, n: &str) -> Transform {
        let time = if self.loops() {
            self.time % d.boss().duration(self.clip())
        } else {
            self.time
        };
        d.boss().tag(n, self.clip(), time, self.pose)
    }
    pub fn loops(&self) -> bool {
        matches!(self.action, Action::Walk | Action::Charge)
    }
    pub fn targets(&self, d: &data::Data) -> Vec<Target> {
        let mut targets = vec![];
        if self.hits < 6 {
            for (i, (tag, min, max)) in [
                ("tag_target", vec3(-64., -64., -16.), vec3(64., 64., 64.)),
                ("tag_target", vec3(-80., -80., 16.), vec3(80., 80., 64.)),
                ("tag_mouth", vec3(-64., -64., -64.), vec3(64., 64., 72.)),
                ("spine_05", vec3(-40., -40., -64.), vec3(40., 40., 64.)),
                ("spine_03", Vec3::splat(-32.), Vec3::splat(32.)),
                ("spine_01", vec3(-48., -48., -64.), vec3(48., 48., 56.)),
                ("tail_05", vec3(-40., -40., -16.), vec3(40., 40., 32.)),
                ("tail_07", vec3(-24., -24., -16.), Vec3::splat(24.)),
            ]
            .into_iter()
            .enumerate()
            {
                let p = self.tag(d, tag);
                // The engine uses axis-aligned damage boxes at animated tags.
                targets.push(Target {
                    id: BASE + i,
                    center: p.translation + (min + max) * 0.5,
                    half: (max - min) * 0.5,
                });
            }
        }
        targets.extend(
            self.larvae
                .iter()
                .enumerate()
                .filter(|(_, l)| l.vulnerable())
                .map(|(i, l)| l.target(BASE + 100 + i)),
        );
        targets
    }
    pub fn hit(&mut self, h: Hit, d: &data::Data) -> Option<&'static str> {
        if h.id >= BASE + 100 {
            return self
                .larvae
                .get_mut(h.id - BASE - 100)
                .and_then(|l| l.hit(h));
        }
        if h.id != BASE
            || !self.weak(d)
            || !h.damage.is_finite()
            || h.damage < 1.
            || matches!(h.kind.means(), DamageKind::Fire | DamageKind::FireSword)
        {
            return None;
        }
        self.pain_at = self.tag(d, "tag_target").translation;
        self.blood_age = 0.;
        self.hits += 1;
        self.variant = u8::from(self.roll(50));
        self.grabbed = false;
        self.set(if self.hits == 6 {
            Action::Death
        } else {
            Action::Pain
        });
        None // The actual pain clip owns its sound, once, on the next combat step.
    }
    pub fn advance(&mut self, c: &mut Combat<'_>, d: &data::Data) -> Feedback {
        let mut out = Feedback::default();
        self.accumulator += f64::from(c.dt.min(0.1));
        while self.accumulator + 1e-9 >= f64::from(STEP) {
            self.accumulator = (self.accumulator - f64::from(STEP)).max(0.);
            self.step(c, d, &mut out);
        }
        out
    }
    fn melee(&self, c: &Combat<'_>, damage: f32, out: &mut Feedback) {
        let toward = self.pose.rotation * Vec3::X;
        let center = self.pose.translation + toward * 128. + Vec3::Z * 44.;
        let delta = c.player.feet + Vec3::Z * 32. - center;
        if delta.x.abs() < 120.
            && delta.y.abs() < 152.
            && delta.z.abs() < 100.
            && c.world
                .sweep(center, c.player.feet + Vec3::Z * 32., Vec3::ZERO)
                .fraction
                >= 0.99
        {
            out.damage += damage;
            out.impulse += delta.with_z(0.).normalize_or_zero() * 200.;
        }
    }
    fn move_toward(&mut self, w: &World, delta: Vec3, speed: f32) {
        let dir = delta.with_z(0.).normalize_or_zero();
        for angle in [0., 0.65, -0.65, 1.1, -1.1] {
            let at = combat::walk_body(
                w,
                self.pose.translation,
                Quat::from_rotation_z(angle) * dir * speed.clamp(0., 600.) * STEP,
                vec3(96., 96., 96.),
            );
            if at.distance_squared(self.pose.translation) > 0.0001 {
                self.pose.translation = at;
                break;
            }
        }
    }
    fn step(&mut self, c: &mut Combat<'_>, d: &data::Data, out: &mut Feedback) {
        let clip = self.clip();
        let old = self.time;
        self.time = (self.time + STEP).min(3600.);
        self.blood_age = (self.blood_age + STEP).min(5.);
        self.quake = (self.quake - STEP).max(0.);
        let rig = d.boss();
        let frame = rig.frame(clip);
        let duration = rig.duration(clip);
        let cross = |f: f32| old <= f * frame && self_time_cross(old, STEP, f * frame);
        for cue in rig.audio.between(
            clip,
            crate::audio::events::Span {
                start: old,
                end: self.time,
                duration,
                frame_time: frame,
                looping: self.action == Action::Walk,
                entered: old == 0.,
            },
        ) {
            if let Some(path) = rig.sounds.get(&cue.path) {
                out.spatial_sounds.push((*path, self.pose.translation));
            }
        }
        for l in &mut self.larvae {
            l.notarget = c.notarget;
            l.opponents.summon = None;
            l.step(c.world, c.player.feet + Vec3::Z * 32., d, out);
        }
        let mut fragments = Vec::new();
        for s in &mut self.shots {
            s.age += STEP;
            if s.ended.is_some() {
                continue;
            }
            if s.fragment {
                s.velocity.z -= 80. * STEP;
            }
            let end = s.at + s.velocity * STEP;
            let half = Vec3::splat(if s.fragment { 8. } else { 16. });
            let wall = c.world.sweep(s.at, end, half);
            let hit = combat::segment_box(
                s.at,
                end,
                c.player.feet + Vec3::Z * 32.,
                vec3(16., 16., 32.) + half,
            );
            let fraction = hit.filter(|f| *f < wall.fraction && !wall.start_solid);
            if fraction.is_some() {
                out.damage += if s.fragment { 10. } else { 15. };
            }
            s.at = s.at.lerp(end, fraction.unwrap_or(wall.fraction));
            if fraction.is_some() || wall.start_solid || wall.fraction < 1. || s.age >= 8. {
                s.ended = Some(s.age);
                let normal = if fraction.is_none() && wall.fraction < 1. {
                    wall.normal
                } else {
                    -s.velocity.normalize_or_zero()
                };
                // Only world impacts leave a surface decal; flesh hits still scatter acid.
                s.normal = if fraction.is_none() && !wall.start_solid && wall.fraction < 1. {
                    normal
                } else {
                    Vec3::ZERO
                };
                out.spatial_sounds.push((
                    if fraction.is_some() {
                        "sound/weapon/shared/splat2.wav"
                    } else {
                        "sound/weapon/shared/splat_small.wav"
                    },
                    s.at,
                ));
                if !s.fragment && !wall.start_solid && s.age < 8. {
                    for n in 0..6 {
                        let angle = (n as f32 / 6.) * std::f32::consts::TAU;
                        let axis = normal.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
                        let up = normal.cross(axis);
                        let dir = (normal * 0.65 + axis * angle.cos() + up * angle.sin())
                            .normalize_or_zero();
                        let start = s.at + normal * 18.;
                        fragments.push(Shot {
                            at: start,
                            start,
                            velocity: dir * 450.,
                            age: 0.,
                            ended: None,
                            fragment: true,
                            normal: Vec3::ZERO,
                        });
                    }
                }
            }
        }
        self.shots
            .retain(|s| s.ended.is_none_or(|t| s.age - t < 1.));
        for s in fragments.into_iter().take(128 - self.shots.len()) {
            self.shots.push(s);
        }
        let delta = c.player.feet - self.pose.translation;
        let distance = delta.truncate().length();
        if matches!(
            self.action,
            Action::Walk | Action::Ready | Action::Spit | Action::Larvae | Action::Strike
        ) && !c.notarget
        {
            let yaw = self.pose.rotation.to_euler(EulerRot::ZYX).0;
            let turn = (delta.y.atan2(delta.x) - yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.pose.rotation = Quat::from_rotation_z(yaw + turn.clamp(-2. * STEP, 2. * STEP));
        }
        match self.action {
            Action::Walk => {
                if c.notarget {
                    return;
                }
                if distance > 240. {
                    self.move_toward(c.world, delta, d.speed("c_centipede", clip));
                }
                if (old / frame / 5.).floor() != (self.time / frame / 5.).floor() {
                    self.melee(c, 0.5, out);
                }
                self.decision = (self.decision - STEP).max(0.);
                if self.decision == 0. {
                    self.decision = 1.;
                    let sight = c
                        .world
                        .sweep(
                            self.pose.translation + Vec3::Z * 96.,
                            c.player.feet + Vec3::Z * 32.,
                            Vec3::ZERO,
                        )
                        .fraction
                        >= 0.99;
                    if sight {
                        let action = if distance < 400. {
                            if self.stage == 3 {
                                match self.cycle {
                                    0 => Action::Strike,
                                    1 => Action::Ready,
                                    _ => Action::Larvae,
                                }
                            } else if self.stage == 2 && distance >= 300. && !self.roll(75) {
                                Action::Walk
                            } else {
                                Action::Ready
                            }
                        } else if self.stage == 3 && distance > 700. && self.roll(70) {
                            Action::Spit
                        } else if self.stage == 3 || self.stage == 2 && self.roll(20) {
                            Action::Charge
                        } else if self.stage == 1 && self.roll(20) {
                            Action::Ready
                        } else if self.roll(50) {
                            Action::Larvae
                        } else {
                            Action::Spit
                        };
                        if action == Action::Strike {
                            self.grab_camera = c.player.feet + vec3(80., 280., 200.);
                            self.victim_feet = c.player.feet;
                        }
                        self.set(action);
                    }
                }
                if self.time >= duration {
                    self.time -= duration;
                }
            }
            Action::Ready => {
                if self.time >= duration {
                    self.set(Action::Crush);
                }
            }
            Action::Crush => {
                if cross(25.) {
                    self.quake = 0.5;
                }
                if cross(30.) {
                    self.melee(c, 25., out);
                }
                if self.time >= duration {
                    if self.stage == 3 {
                        self.cycle = 2;
                    }
                    self.set(Action::Walk);
                }
            }
            Action::Spit | Action::Larvae => {
                let larvae = self.action == Action::Larvae;
                for (n, f) in [8., if larvae { 12. } else { 14. }].into_iter().enumerate() {
                    if cross(f) {
                        let start = self.tag(d, "tag_tongue").translation;
                        let aim = (c.player.feet + Vec3::Z * 32. - start).normalize_or_zero();
                        if larvae {
                            let slot = self
                                .larvae
                                .iter()
                                .position(|l| l.health <= 0. && l.time >= 10.)
                                .unwrap_or(self.larvae.len());
                            if slot < 16 {
                                let mut l = Insect::new(
                                    Kind::Larva,
                                    start,
                                    aim.y.atan2(aim.x),
                                    1.,
                                    self.random as usize + slot,
                                );
                                // A malformed original second launch token is resolved to 250 units/s.
                                l.launch(aim * if n == 0 { 150. } else { 250. } + Vec3::Z * 150.);
                                if slot == self.larvae.len() {
                                    self.larvae.push(l);
                                } else {
                                    self.larvae[slot] = l;
                                }
                            }
                        } else if self.shots.len() < 128 {
                            self.shots.push(Shot {
                                at: start,
                                start,
                                velocity: aim * 550.,
                                age: 0.,
                                ended: None,
                                fragment: false,
                                normal: Vec3::ZERO,
                            });
                        }
                    }
                }
                if self.time >= duration {
                    if larvae && self.stage == 3 {
                        self.cycle = 0;
                    }
                    self.set(Action::Walk);
                }
            }
            Action::Charge => {
                self.move_toward(c.world, self.pose.rotation * Vec3::X, 400.);
                self.quake = 0.15;
                if distance < 240. || self.time > 3. {
                    self.melee(c, if self.stage == 2 { 25. } else { 35. }, out);
                    self.set(Action::Ready);
                }
            }
            Action::Strike => {
                if self.time >= duration {
                    self.grabbed = distance < 400.
                        && c.world
                            .sweep(
                                self.tag(d, "tag_mandibles").translation,
                                c.player.feet + Vec3::Z * 32.,
                                Vec3::ZERO,
                            )
                            .fraction
                            >= 0.99;
                    self.grab_feet = c.player.feet;
                    if self.grabbed {
                        self.set(Action::Shake);
                    } else {
                        self.cycle = 1;
                        self.set(Action::Walk);
                    }
                }
            }
            Action::Shake => {
                if self.grabbed {
                    let mandibles = self.tag(d, "tag_mandibles").translation;
                    // DragEnemy's second tag belongs to its victim, not the Centipede.
                    let gut = d.rigs["alice"]
                        .tag(
                            "tag_gut",
                            "ready",
                            self.time,
                            Transform {
                                translation: Vec3::ZERO,
                                rotation: Quat::IDENTITY,
                            },
                        )
                        .translation;
                    let goal = self
                        .grab_feet
                        .lerp(mandibles - gut, (self.time / 0.15).clamp(0., 1.));
                    let t = c.world.body_trace(c.player.feet, goal);
                    if !t.start_solid {
                        c.player.feet = c.player.feet.lerp(goal, t.fraction);
                        c.player.velocity = Vec3::ZERO;
                        c.player.grounded = false;
                    }
                }
                if cross(17.) && self.grabbed {
                    self.grabbed = false;
                    c.player.velocity = self.pose.rotation * vec3(500., 50., 250.);
                }
                if self.time >= duration {
                    self.grabbed = false;
                    self.set(Action::Return);
                }
            }
            Action::Return => {
                if self.time >= duration {
                    self.cycle = 1;
                    self.set(Action::Walk);
                }
            }
            Action::Pain => {
                if self.time >= duration {
                    self.stage = (self.hits / 2 + 1).min(3);
                    self.cycle = 0;
                    self.set(Action::Walk);
                }
            }
            Action::Death => {
                if cross(18.) {
                    self.quake = 0.5;
                }
                if self.time >= duration {
                    self.set(Action::Thrash);
                }
            }
            Action::Thrash => {
                if self.time >= duration {
                    self.thrashes += 1;
                    self.set(if self.thrashes == 3 {
                        Action::Dead
                    } else {
                        Action::Thrash
                    });
                }
            }
            Action::Dead => {}
        }
        self.victim_feet = c.player.feet;
        // Player clipping is independent of damage boxes, so it cannot mask a weak-point trace.
        if self.hits < 6
            && !self.grabbed
            && distance < 120.
            && (-64. ..452.).contains(&(c.player.feet.z - self.pose.translation.z))
        {
            let goal =
                self.pose.translation + delta.with_z(0.).try_normalize().unwrap_or(Vec3::X) * 120.;
            let goal = goal.with_z(c.player.feet.z);
            let t = c.world.body_trace(c.player.feet, goal);
            if !t.start_solid {
                c.player.feet = c.player.feet.lerp(goal, t.fraction);
            }
        }
    }
}
fn self_time_cross(old: f32, dt: f32, t: f32) -> bool {
    old <= t && old + dt > t
}

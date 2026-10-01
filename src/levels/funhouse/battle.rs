use super::*;
use crate::combat::{self, DamageKind};
const STEP: f32 = 1. / 120.;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Idle,
    Walk,
    Run,
    Ready,
    KnifeOut,
    Knife,
    RattleOut,
    Rattle,
    Split,
    Close,
    Jump,
    Prop,
    Takeoff,
    Fly,
    FallReady,
    Fall,
    Impact,
    Pain,
    Dead,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Weapon {
    None,
    Knife,
    Rattle,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Grenade {
    pub at: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub ended: Option<f32>,
    pub serial: u32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Boss {
    pub id: usize,
    pub family: usize,
    pub mini: bool,
    pub at: Vec3,
    pub yaw: f32,
    pub health: f32,
    pub action: Action,
    pub time: f32,
    pub frozen: bool,
    pub velocity: Vec3,
    pub grenades: Vec<Grenade>,
    pub attacks: u8,
    pub quake: f32,
    pub weapon: Weapon,
    serial: u32,
    random: u32,
    pain: f32,
    pain_wait: f32,
    choice_wait: f32,
}
impl Boss {
    pub fn new(id: usize, family: usize, mini: bool, p: Transform) -> Self {
        Self {
            id,
            family,
            mini,
            at: p.translation,
            yaw: p.rotation.to_euler(EulerRot::ZYX).0,
            health: match (family, mini) {
                (0, false) => 800.,
                (1, false) => 900.,
                (0, true) => 75.,
                _ => 50.,
            },
            action: if mini { Action::Jump } else { Action::Idle },
            time: 0.,
            frozen: false,
            velocity: Vec3::ZERO,
            grenades: vec![],
            attacks: 0,
            quake: 0.,
            weapon: Weapon::None,
            serial: 0,
            random: 0x715a3 + id as u32,
            pain: 0.,
            pain_wait: 0.,
            choice_wait: 2.,
        }
    }
    pub fn model(&self) -> &'static str {
        match (self.family, self.mini) {
            (0, false) => "c_tweedle_dee",
            (1, false) => "c_tweedle_dum",
            (0, true) => "c_tweedle_mini_dee",
            _ => "c_tweedle_mini_dum",
        }
    }
    pub fn scale(&self) -> f32 {
        if self.mini {
            0.65
        } else {
            1.
        }
    }
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.at,
            rotation: Quat::from_rotation_z(self.yaw),
        }
    }
    pub fn target(&self) -> Target {
        let half = self.half();
        Target {
            id: self.id,
            center: self.at + Vec3::Z * half.z,
            half,
        }
    }
    fn half(&self) -> Vec3 {
        match (self.family, self.mini) {
            (0, false) => vec3(40., 40., 56.),
            (_, false) => vec3(32., 32., 43.),
            (0, true) => vec3(24., 24., 36.),
            (_, true) => vec3(20., 20., 28.),
        }
    }
    pub fn clip(&self) -> &'static str {
        match self.action {
            Action::Idle => "idle",
            Action::Walk => "walk",
            Action::Run => "run",
            Action::Ready => "ready",
            Action::KnifeOut => "knife_out",
            Action::Knife => "knife_attack",
            Action::RattleOut => "rattle_out",
            Action::Rattle => "rattle_attack",
            Action::Split => "russian_split",
            Action::Close => "russian_close",
            Action::Jump => "russian_jump",
            Action::Prop => "prop_out",
            Action::Takeoff => "take_off",
            Action::Fly => "fly_forward02",
            Action::FallReady => "fall_ready",
            Action::Fall => "fall_falling",
            Action::Impact => "fall_impact",
            Action::Pain => "pain",
            Action::Dead => {
                if self.frozen {
                    "death_frozen"
                } else {
                    "death1"
                }
            }
        }
    }
    pub fn loops(&self) -> bool {
        matches!(
            self.action,
            Action::Idle | Action::Walk | Action::Run | Action::Ready | Action::Fly | Action::Fall
        )
    }
    fn set(&mut self, a: Action) {
        self.action = a;
        self.time = 0.;
    }
    fn chance(&mut self) -> f32 {
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random >> 8) as f32 / 16777216.
    }
    pub fn hit(&mut self, h: Hit) -> Option<&'static str> {
        if self.health <= 0. || h.damage <= 0. || !h.damage.is_finite() {
            return None;
        }
        self.health = (self.health - h.damage).max(0.);
        if self.health == 0. {
            self.frozen = h.kind.means() == DamageKind::Ice;
            self.set(Action::Dead);
            self.grenades.clear();
            return None;
        }
        self.pain += h.damage;
        if self.pain >= 50.
            && self.pain_wait == 0.
            && !matches!(
                self.action,
                Action::Takeoff | Action::Fly | Action::FallReady | Action::Fall
            )
        {
            self.pain = 0.;
            self.pain_wait = 1.;
            self.set(Action::Pain);
        }
        None
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.family < 2
                && self.at.is_finite()
                && self.at.abs().max_element() < 30000.
                && self.velocity.is_finite()
                && self.velocity.length() < 5000.,
            "Invalid Tweedle pose"
        );
        state::clock(
            "Tweedle health",
            self.health,
            if self.mini { 75. } else { 900. },
        )?;
        state::clock("Tweedle action", self.time, 1e7)?;
        ensure!(
            (self.health == 0.) == (self.action == Action::Dead) && self.grenades.len() <= 16,
            "Invalid Tweedle death/projectiles"
        );
        for g in &self.grenades {
            ensure!(
                g.at.is_finite() && g.velocity.is_finite(),
                "Invalid grenade"
            );
            state::clock("grenade age", g.age, 5.)?;
        }
        Ok(())
    }
    fn projectiles(&mut self, w: &World, eye: Vec3, out: &mut Feedback) {
        let target = Target {
            id: crate::dice::ALICE,
            center: eye - Vec3::Z * 20.,
            half: PLAYER_HALF,
        };
        self.grenades.retain_mut(|g| {
            g.age += STEP;
            if let Some(end) = g.ended {
                return g.age - end < 2.;
            }
            g.velocity.z -= crate::movement::GRAVITY * STEP;
            let to = g.at + g.velocity * STEP;
            let wall = w.sweep(g.at, to, Vec3::splat(8.));
            let direct = combat::contact(
                &combat::Context {
                    world: w,
                    targets: &[target],
                },
                g.at,
                to,
                8.,
            );
            if let Some((_, f)) = direct {
                g.at = g.at.lerp(to, f);
                out.damage += 5.;
                g.ended = Some(g.age);
            } else if g.age >= 2. || wall.start_solid {
                g.ended = Some(g.age);
            } else if wall.fraction < 1. {
                g.at = g.at.lerp(to, wall.fraction) + wall.normal * 0.1;
                g.velocity = (g.velocity - 2. * wall.normal * g.velocity.dot(wall.normal)) * 0.6;
            } else {
                g.at = to;
            }
            if g.ended.is_some() {
                let delta = target.center - g.at;
                let r = w.sweep(g.at, target.center, Vec3::ZERO);
                if direct.is_none() && delta.length() < 140. && !r.start_solid && r.fraction >= 1. {
                    let f = 1. - delta.length() / 140.;
                    out.damage += 80. * f;
                    out.impulse += delta.normalize_or_zero() * 100. * f;
                }
            }
            true
        });
    }
    pub fn step(
        &mut self,
        d: &data::Data,
        w: &World,
        eye: Vec3,
        notarget: bool,
        live_minis: usize,
        out: &mut Feedback,
    ) -> Option<Transform> {
        let old = self.time;
        self.time += STEP;
        self.pain_wait = (self.pain_wait - STEP).max(0.);
        self.quake = (self.quake - STEP).max(0.);
        self.projectiles(w, eye, out);
        let rig = &d.rigs[self.model()];
        let clip = self.clip();
        let duration = rig.duration(clip);
        let done = self.time >= duration;
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
        if self.health <= 0. {
            return None;
        }
        let delta = eye - Vec3::Z * 56. - self.at;
        let distance = delta.truncate().length();
        let aim = delta.truncate().normalize_or_zero().extend(0.);
        let tr = w.sweep(self.target().center, eye - Vec3::Z * 20., Vec3::ZERO);
        let visible = !notarget
            && distance
                <= if self.family == 0 && !self.mini {
                    800.
                } else {
                    1000.
                }
            && !tr.start_solid
            && tr.fraction >= 1.;
        if visible {
            let desired = aim.y.atan2(aim.x);
            let diff = (desired - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += diff.clamp(-7. * STEP, 7. * STEP);
        }
        let crossed =
            |frame: f32| old <= frame * rig.frame(clip) && self.time > frame * rig.frame(clip);
        if self.action == Action::KnifeOut && crossed(14.) {
            self.weapon = Weapon::Knife;
        }
        if self.action == Action::RattleOut && crossed(if self.family == 0 { 14. } else { 20. }) {
            self.weapon = Weapon::Rattle;
        }
        if self.action == Action::Rattle && crossed(if self.family == 0 { 9. } else { 7. }) {
            self.weapon = Weapon::None;
        }
        let mut spawn = None;
        match self.action {
            Action::Idle | Action::Ready
                if visible && (self.action == Action::Idle || self.time >= self.choice_wait) =>
            {
                self.set(Action::Walk)
            }
            Action::Walk | Action::Run => {
                if self.weapon != Weapon::Knife {
                    self.set(Action::KnifeOut);
                } else if visible && distance < 100. {
                    self.set(Action::Knife);
                } else if done {
                    let running = self.action == Action::Run;
                    let split = !self.mini && live_minis < self.family + 2 && distance >= 150.;
                    if !running && self.family == 1 && !self.mini && distance >= 250. {
                        self.choice_wait = 2. + self.chance() * 2.;
                        self.set(Action::Ready);
                    } else if !running && distance >= 300. {
                        self.set(Action::Run);
                    } else if running && distance < 250. {
                        self.set(Action::Walk);
                    } else if running
                        && (self.family == 0 || self.mini)
                        && visible
                        && distance >= if self.family == 0 { 240. } else { 230. }
                        && self.chance() < if self.family == 0 { 0.5 } else { 0.33 }
                    {
                        self.set(Action::Prop);
                    } else if self.family == 0
                        && visible
                        && distance >= if running { 0. } else { 275. }
                        && self.chance() < if running { 0.1 } else { 0.2 }
                    {
                        self.set(Action::RattleOut);
                    } else if split && self.chance() < if self.family == 0 { 0.3 } else { 0.4 } {
                        self.set(Action::Split);
                    } else if self.family == 1
                        && visible
                        && distance >= if running { 0. } else { 250. }
                        && self.chance() < if running { 0.5 } else { 0.3 }
                    {
                        self.set(Action::RattleOut);
                    } else {
                        self.set(if running { Action::Run } else { Action::Walk });
                    }
                }
                if visible && matches!(self.action, Action::Walk | Action::Run) {
                    let speed = rig.clips[clip].distance / duration.max(0.01) * rig.def.scale;
                    self.at = combat::walk_body(
                        w,
                        self.at,
                        aim * speed.clamp(30., 220.) * STEP,
                        self.half(),
                    );
                }
            }
            Action::KnifeOut if done => self.set(Action::Walk),
            Action::Knife => {
                if crossed(if self.family == 0 { 8. } else { 7. })
                    && visible
                    && distance <= 100.
                    && delta.z.abs() < 100.
                {
                    out.damage += 10.;
                    out.impulse += aim * 50.;
                    self.attacks |= 1;
                }
                if done {
                    self.set(Action::Walk);
                }
            }
            Action::RattleOut if done => self.set(Action::Rattle),
            Action::Rattle => {
                if crossed(if self.family == 0 { 10. } else { 8. }) && self.grenades.len() < 16 {
                    let at = rig
                        .tag("tag_sword", clip, self.time, self.pose(), false)
                        .translation;
                    let mut velocity = (eye - at).normalize_or_zero() * 500.;
                    velocity.z +=
                        crate::movement::GRAVITY * 0.5 * (eye - at).truncate().length() / 500.;
                    self.serial = self.serial.wrapping_add(1);
                    self.grenades.push(Grenade {
                        at,
                        velocity,
                        age: 0.,
                        ended: None,
                        serial: self.serial,
                    });
                    self.attacks |= 2;
                }
                if done {
                    self.set(Action::Walk);
                }
            }
            Action::Split => {
                if crossed(28.) && !self.mini && live_minis < self.family + 2 {
                    let at = rig
                        .tag("tag_belly", clip, self.time, self.pose(), false)
                        .translation;
                    let half = if self.family == 0 {
                        vec3(24., 24., 36.)
                    } else {
                        vec3(20., 20., 28.)
                    };
                    let center = at + Vec3::Z * half.z;
                    if !w.sweep(center, center, half).start_solid {
                        spawn = Some(Transform {
                            translation: at,
                            rotation: Quat::from_rotation_z(self.yaw),
                        });
                        self.attacks |= 4;
                    }
                }
                if done {
                    self.set(Action::Close);
                }
            }
            Action::Close | Action::Pain if done => self.set(Action::Walk),
            Action::Jump => {
                if old == 0. {
                    self.velocity =
                        vec3(self.yaw.cos(), self.yaw.sin(), 0.) * 120. + Vec3::Z * 240.;
                }
                self.velocity.z -= crate::movement::GRAVITY * STEP;
                let body = self.target();
                let delta = self.velocity * STEP;
                let tr = w.sweep(body.center, body.center + delta, body.half);
                self.at += delta * tr.fraction;
                if tr.fraction < 1. {
                    self.velocity -= tr.normal * self.velocity.dot(tr.normal).min(0.);
                }
                if done && tr.fraction < 1. && tr.normal.z > 0.65 {
                    self.velocity = Vec3::ZERO;
                    self.set(Action::Walk);
                }
            }
            Action::Prop if done => self.set(Action::Takeoff),
            Action::Takeoff => {
                let to = self.at + Vec3::Z * 160. * STEP;
                let tr = w.sweep(
                    self.target().center,
                    self.target().center + (to - self.at),
                    self.target().half,
                );
                self.at = self.at.lerp(to, tr.fraction);
                if done {
                    self.set(Action::Fly);
                }
            }
            Action::Fly => {
                let desired = (aim * 180.
                    + Vec3::Z * ((eye.z + 160. - self.at.z) * 2.).clamp(-160., 160.))
                    * STEP;
                let tr = w.sweep(
                    self.target().center,
                    self.target().center + desired,
                    self.target().half,
                );
                self.at += desired * tr.fraction;
                if self.time >= 10. || (distance < 150. && delta.z.abs() < 200.) {
                    self.set(Action::FallReady);
                }
            }
            Action::FallReady if done => {
                self.velocity = Vec3::ZERO;
                self.set(Action::Fall);
            }
            Action::Fall => {
                self.velocity.z -= crate::movement::GRAVITY * STEP;
                let delta = self.velocity * STEP;
                let body = self.target();
                let tr = w.sweep(body.center, body.center + delta, body.half);
                self.at += delta * tr.fraction;
                if tr.fraction < 1. && tr.normal.z > 0.65 && self.time > 0.2 {
                    let ground_eye = eye - Vec3::Z * 56.;
                    let d = ground_eye - self.at;
                    let ray = w.sweep(
                        self.at + Vec3::Z * 16.,
                        ground_eye + Vec3::Z * 16.,
                        Vec3::ZERO,
                    );
                    if d.length() < 300. && !ray.start_solid && ray.fraction >= 1. {
                        // Native radiusattack -> RadiusDamage: linear distance falloff.
                        let fraction = 1. - d.length() / 300.;
                        out.damage += 50. * fraction;
                        out.impulse += (d.normalize_or_zero() * 180. + Vec3::Z * 200.) * fraction;
                    }
                    self.quake = if self.family == 0 { 3. } else { 2.5 };
                    self.attacks |= 8;
                    self.set(Action::Impact);
                }
            }
            Action::Impact if done => self.set(Action::Walk),
            _ => (),
        }
        spawn
    }
}
impl Funhouse {
    pub fn essence_position(&self) -> Vec3 {
        self.data.points[if self.saved.essence_side == 0 {
            "help_me1"
        } else {
            "help_me2"
        }]
        .translation
    }
    pub fn fight_step(&mut self, c: &mut crate::level::Combat<'_>) -> Feedback {
        let mut out = Feedback::default();
        if c.dt <= 0. || !self.saved.fight || self.saved.scene.is_some() || c.stats.sanity() <= 0. {
            return out;
        }
        self.saved.step += c.dt.min(0.1);
        while self.saved.step + 1e-7 >= STEP {
            self.saved.step = (self.saved.step - STEP).max(0.);
            let eye = c.player.eye();
            for i in 0..2 {
                let count = self
                    .saved
                    .minis
                    .iter()
                    .filter(|b| b.family == i && b.health > 0.)
                    .count();
                if let Some(p) =
                    self.saved.bosses[i].step(&self.data, c.world, eye, c.notarget, count, &mut out)
                {
                    if self.saved.serial < 89_000 {
                        let id = BASE + 100 + self.saved.serial;
                        self.saved.serial += 1;
                        self.saved.minis.push(Boss::new(id, i, true, p));
                    }
                }
            }
            for b in &mut self.saved.minis {
                b.step(&self.data, c.world, eye, c.notarget, usize::MAX, &mut out);
            }
            self.saved.minis.retain(|b| b.health > 0. || b.time < 10.);
            if self.saved.essence_wait > 0. {
                self.saved.essence_wait = (self.saved.essence_wait - STEP).max(0.);
                if self.saved.essence_wait == 0. {
                    self.saved.essence = true;
                }
            }
            let at = self.essence_position();
            if self.saved.essence
                && (c.player.feet + Vec3::Z * 32. - at).length() < 44.
                && (c.stats.sanity() < 100. || c.stats.will() < 100.)
            {
                let ray = c.world.sweep(c.player.eye(), at, Vec3::ZERO);
                if !ray.start_solid && ray.fraction >= 1. {
                    c.stats.essence(25.);
                    self.saved.essence = false;
                    self.saved.essence_side = 1 - self.saved.essence_side;
                    self.saved.essence_wait = 10.;
                    self.saved.essence_generation += 1;
                }
            }
            if self.saved.bosses.iter().all(|b| b.health <= 0.) {
                self.saved.death_wait = (self.saved.death_wait + STEP).min(5.);
                if self.saved.death_wait >= 5. && c.stats.sanity() > out.damage {
                    self.saved.minis.clear();
                    self.begin(Kind::Hatter);
                    break;
                }
            }
        }
        out
    }
}

//! Bounded card-guard encounters and swept weapon contacts; no original AI executes.
use crate::collision::World;
use macroquad::prelude::*;
pub(crate) mod navigation;

#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub id: usize,
    pub center: Vec3,
    pub half: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub id: usize,
    pub damage: f32,
    pub kind: DamageKind,
    /// Authored knockback amount along the impact direction, before target mass.
    pub knockback: Vec3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    Other,
    Blunderbuss,
    EyeStaff,
    Knife,
    Cards,
    Electric,
    Fire,
    FireSword,
    Ice,
    Jacks,
    DemonMelee,
    DemonElectric,
    DemonFire,
    DemonIce,
}
impl DamageKind {
    pub fn is_demon(self) -> bool {
        matches!(
            self,
            Self::DemonMelee | Self::DemonElectric | Self::DemonFire | Self::DemonIce
        )
    }
    pub fn means(self) -> Self {
        match self {
            Self::DemonMelee | Self::EyeStaff => Self::Other,
            Self::DemonElectric => Self::Electric,
            Self::DemonFire => Self::FireSword,
            Self::DemonIce => Self::Ice,
            k => k,
        }
    }
}
/// Velocity imparted by damage, kept independently of AI steering and saved.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Recoil {
    velocity: Vec3,
}
impl Recoil {
    pub fn valid(&self) -> bool {
        self.velocity.is_finite() && self.velocity.length_squared() <= 4000_f32.powi(2)
    }
    pub fn hit(&mut self, impulse: Vec3, mass: f32) {
        if impulse.is_finite() {
            // Sentient damage: 500 * authored knockback / max(mass, 50).
            self.velocity =
                (self.velocity + impulse * (500. / mass.max(50.))).clamp_length_max(4000.);
        }
    }
    pub fn step(&mut self, dt: f32, world: &World, body: Target) -> Vec3 {
        if dt <= 0. { return Vec3::ZERO; }
        for push in &world.traversal.actor_pushes {
            if push.enabled && push.volume.touches(body.center, body.center + self.velocity * dt, body.half) {
                self.velocity += push.direction * (push.speed - self.velocity.dot(push.direction));
            }
        }
        if self.velocity.length_squared() < 0.01 {
            self.velocity = Vec3::ZERO;
            return Vec3::ZERO;
        }
        let delta = self.velocity * dt;
        let trace = world.sweep(body.center, body.center + delta, body.half);
        if trace.start_solid {
            self.velocity = Vec3::ZERO;
            return Vec3::ZERO;
        }
        if trace.fraction < 1. {
            self.velocity -= trace.normal * self.velocity.dot(trace.normal).min(0.);
        }
        self.velocity *= (-6. * dt).exp();
        delta * trace.fraction
    }
}
pub struct Context<'a> {
    pub world: &'a World,
    pub targets: &'a [Target],
}
/// First contact on the actual swept segment, strictly before any solid world hit.
pub fn contact(ctx: &Context<'_>, start: Vec3, end: Vec3, radius: f32) -> Option<(usize, f32)> {
    contact_box(ctx, start, end, Vec3::splat(radius))
}
pub fn contact_box(ctx: &Context<'_>, start: Vec3, end: Vec3, half: Vec3) -> Option<(usize, f32)> {
    let candidate = ctx
        .targets
        .iter()
        .filter_map(|t| segment_box(start, end, t.center, t.half + half).map(|f| (t.id, f)))
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    let wall = ctx.world.sweep_except(start, end, half, Some(candidate.0));
    (!wall.start_solid && candidate.1 < wall.fraction).then_some(candidate)
}
pub(crate) fn segment_box(start: Vec3, end: Vec3, center: Vec3, half: Vec3) -> Option<f32> {
    let delta = end - start;
    let mut near: f32 = 0.;
    let mut far: f32 = 1.;
    for i in 0..3 {
        let low = center[i] - half[i];
        let high = center[i] + half[i];
        if delta[i].abs() < 0.00001 {
            if start[i] < low || start[i] > high {
                return None;
            }
        } else {
            let a = (low - start[i]) / delta[i];
            let b = (high - start[i]) / delta[i];
            near = near.max(a.min(b));
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
    }
    Some(near)
}
pub fn will_cost(weapon: usize, alternate: bool) -> f32 {
    match (weapon, alternate) {
        (0..=9, _) => crate::weapon_rules::rule(weapon, alternate, 0).cost,
        _ => 0.,
    }
}
pub fn weapon_damage(weapon: usize, alternate: bool) -> f32 {
    match (weapon, alternate) {
        (0, false) => 25.,
        (0, true) => 45.,
        (1, false) => 7.,
        (1, true) => 15., // Carrier only; each of its eight fragments deals seven.
        (2, false) => 24.,
        (2, true) => 30.,
        _ => 0.,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum State {
    Idle,
    Alert,
    Chase,
    Attack,
    Pain,
    Dead,
}
impl State {
    pub fn clip(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Alert => "alert1",
            Self::Chase => "walk",
            Self::Attack => "attack1",
            Self::Pain => "pain1",
            Self::Dead => "death_1",
        }
    }
    pub fn loops(self) -> bool {
        matches!(self, Self::Idle | Self::Chase)
    }
}
#[derive(Clone, Copy)]
pub struct Timing {
    pub sever: Option<&'static [crate::dismember::Recipe; 3]>,
    pub alert: f32,
    pub attack: f32,
    pub hit: f32,
    pub pain: f32,
    pub death: f32,
}
#[derive(Default)]
pub struct Feedback {
    pub will_drain: f32,
    pub cue_sounds: Vec<(String, Vec3)>,
    pub damage: f32,
    pub impulse: Vec3,
    pub summon_hits: Vec<Hit>,
    pub sounds: Vec<&'static str>,
    pub spatial_sounds: Vec<(&'static str, Vec3)>,
}
impl Feedback {
    pub fn strike(&mut self, id: usize, damage: f32, impulse: Vec3, kind: DamageKind) {
        if id == crate::dice::SUMMON {
            self.summon_hits.push(Hit {
                id,
                damage,
                kind,
                knockback: impulse / 2.5,
            });
        } else {
            self.damage += damage;
            self.impulse += impulse;
        }
    }
}

/// Saved hostility, with live bodies supplied by the owning scene each frame.
/// Projectiles always collide with both bodies, independently of current aim.
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Opponents {
    #[serde(skip)]
    pub summon: Option<Target>,
    pub demon: bool,
}
impl Opponents {
    pub fn bodies(&self, eye: Vec3) -> Vec<Target> {
        let mut bodies = vec![Target {
            id: crate::dice::ALICE,
            center: eye - Vec3::Z * 20.,
            half: crate::collision::PLAYER_HALF,
        }];
        bodies.extend(self.summon);
        bodies
    }
    pub fn aim(
        &mut self,
        world: &World,
        from: Vec3,
        eye: Vec3,
        notarget: bool,
        committed: bool,
    ) -> (Vec3, bool, usize) {
        if self.summon.is_none() {
            self.demon = false;
        }
        if !committed && !self.demon {
            if let Some(d) = self.summon {
                let distance = from.distance_squared(d.center);
                let sight = world.sweep(from, d.center, Vec3::splat(0.5));
                if distance < 1200_f32.powi(2)
                    && (notarget || distance < from.distance_squared(eye))
                    && !sight.start_solid
                    && sight.fraction >= 1.
                {
                    self.demon = true;
                }
            }
        }
        if self.demon {
            if let Some(d) = self.summon {
                return (d.center, false, d.id);
            }
        }
        (eye, notarget, crate::dice::ALICE)
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Guard {
    #[serde(default)]
    pub opponents: Opponents,
    #[serde(default)]
    pub frozen: bool,
    #[serde(default)]
    pub cut: bool,
    #[serde(default, skip_serializing_if = "crate::dismember::Variant::is_default")]
    pub cut_variant: crate::dismember::Variant,
    #[serde(default)]
    pub recoil: Recoil,
    #[serde(default)]
    pub dismember: crate::dismember::State,
    #[serde(default)]
    pub electric: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst: Option<crate::dismember::burst::State>,
    pub notarget: bool,
    pub ranged: bool,
    pub shots: Vec<Bolt>,
    pub attacks: u32,
    pub feet: Vec3,
    pub yaw: f32,
    pub state: State,
    pub time: f32,
    pub health: f32,
    pub scale: f32,
    home: Vec3,
    cooldown: f32,
    hit: bool,
    accumulator: f64,
    falling: f32,
    #[serde(default)]
    placed: bool,
}
impl Guard {
    pub fn validate_save(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (0. ..=crate::electric::LIFE).contains(&self.electric),
            "Invalid saved electric hit"
        );
        if let Some(burst) = &self.burst {
            anyhow::ensure!(
                self.health == 0. && !self.frozen && !self.cut,
                "Invalid burst death"
            );
            burst.validate()?;
        }
        self.dismember.validate(self.health == 0.)?;
        anyhow::ensure!(
            (!self.cut || (!self.ranged && !self.frozen && self.health == 0.))
                && (self.cut || self.cut_variant.is_default()),
            "Invalid saved severed guard"
        );
        anyhow::ensure!(
            self.cut || !self.dismember.severed,
            "Detached piece without death recipe"
        );
        anyhow::ensure!(
            (0.0..=1000.).contains(&self.health)
                && (0.01..=10.).contains(&self.scale)
                && self.time >= 0.
                && self.cooldown >= 0.
                && self.accumulator.abs() < 0.01
                && self.shots.len() <= 256
                && self.feet.is_finite()
                && self.recoil.valid()
                && (!self.frozen || self.health == 0.)
                && self.home.is_finite()
                && ((self.state == State::Dead) == (self.health == 0.)),
            "Invalid saved guard"
        );
        anyhow::ensure!(
            self.shots.iter().all(|s| (0.0..=5.).contains(&s.age)
                && s.position.is_finite()
                && s.direction.is_finite()
                && s.direction.length_squared() <= 1.01),
            "Invalid saved Diamond projectile"
        );
        Ok(())
    }
    pub fn new(feet: Vec3, yaw: f32, scale: f32) -> Self {
        Self {
            opponents: Default::default(),
            cut: false,
            cut_variant: Default::default(),
            frozen: false,
            recoil: Recoil::default(),
            dismember: Default::default(),
            electric: 0.,
            burst: None,
            notarget: false,
            ranged: false,
            shots: Vec::new(),
            attacks: 0,
            feet,
            yaw,
            scale,
            home: feet,
            state: State::Idle,
            time: 0.,
            health: 55.,
            cooldown: 0.,
            hit: false,
            accumulator: 0.,
            falling: 0.,
            placed: false,
        }
    }
    pub fn target(&self, id: usize) -> Target {
        Target {
            id,
            center: self.feet + Vec3::Z * 40. * self.scale,
            half: vec3(24., 24., 40.) * self.scale,
        }
    }
    pub fn diamond(feet: Vec3, yaw: f32, scale: f32) -> Self {
        Self {
            ranged: true,
            ..Self::new(feet, yaw, scale)
        }
    }
    pub fn clip(&self) -> &'static str {
        if self.frozen {
            "death_frozen"
        } else if self.cut {
            self.cut_variant.clip()
        } else if self.ranged && self.state == State::Attack {
            "stand_attack"
        } else {
            self.state.clip()
        }
    }
    fn set_state(&mut self, state: State) {
        if self.state != state {
            self.state = state;
            self.time = 0.;
            self.hit = false;
        }
    }
    pub fn hit(&mut self, hit: Hit) -> Option<&'static str> {
        if self.health > 0. && hit.damage.is_finite() && hit.damage > 0. {
            self.recoil.hit(hit.knockback, 200.);
        }
        self.hurt_kind(hit.damage, hit.kind)
    }
    pub fn hurt(&mut self, damage: f32) -> Option<&'static str> {
        self.hurt_kind(damage, DamageKind::Other)
    }
    pub fn hurt_kind(&mut self, damage: f32, kind: DamageKind) -> Option<&'static str> {
        if self.health <= 0. || !damage.is_finite() || damage <= 0. {
            return None;
        }
        self.health = (self.health - damage).max(0.);
        if kind.means() == DamageKind::Electric {
            self.electric = crate::electric::LIFE;
        }
        if self.health == 0. {
            self.frozen = kind == DamageKind::Ice;
            self.cut = !self.ranged && kind == DamageKind::Knife;
            if self.cut {
                self.cut_variant = crate::dismember::Variant::choose(self.home, self.attacks);
            }
            let seed = crate::dismember::burst::seed(self.home, self.attacks);
            if !matches!(kind.means(), DamageKind::Knife | DamageKind::Ice) && seed % 10 == 0 {
                self.burst = Some(crate::dismember::burst::State::new(
                    self.feet, self.yaw, self.scale, seed,
                ));
            }
            self.set_state(State::Dead);
            if self.frozen {
                return Some("sound/character/shared/freeze_death.wav");
            }
            if self.cut {
                return None;
            }
            Some(if self.ranged {
                "sound/character/cardguard/diamond/death1.wav"
            } else {
                "sound/character/cardguard/club/death1.wav"
            })
        } else {
            // Repeated pellets do not endlessly restart the pain pose.
            self.set_state(State::Pain);
            Some(if self.ranged {
                "sound/character/cardguard/diamond/pain1.wav"
            } else {
                "sound/character/cardguard/club/pain1.wav"
            })
        }
    }
    pub fn advance(&mut self, dt: f32, world: &World, eye: Vec3, timing: Timing) -> Feedback {
        self.place(world);
        let mut out = Feedback::default();
        if !dt.is_finite() || dt <= 0. {
            return out;
        }
        self.accumulator += dt.min(0.1) as f64;
        while self.accumulator + 1e-9 >= 1. / 120. {
            self.accumulator -= 1. / 120.;
            self.step(1. / 120., world, eye, timing, &mut out);
        }
        out
    }
    /// Settle newly spawned/restored editor placements before their first draw,
    /// even if a conversation has paused combat. Later falls retain gravity.
    pub fn place(&mut self, world: &World) {
        if self.placed {
            return;
        }
        let body = self.target(0);
        if let Some(feet) =
            world.actor_footing(self.feet, body.center - self.feet, body.half, 1024.)
        {
            if self.home.distance_squared(self.feet) < 0.01 {
                self.home = feet;
            }
            self.feet = feet;
            self.falling = 0.;
        }
        self.placed = true;
    }
    fn step(&mut self, dt: f32, world: &World, eye: Vec3, timing: Timing, out: &mut Feedback) {
        self.electric = (self.electric - dt).max(0.);
        if let Some(burst) = &mut self.burst {
            burst.step(dt, world);
        }
        self.feet += self.recoil.step(dt, world, self.target(0));
        self.time += dt;
        self.cooldown = (self.cooldown - dt).max(0.);
        let bodies = self.opponents.bodies(eye);
        self.shots
            .retain_mut(|s| s.advance(dt, world, &bodies, out));
        // Authored actor starts can sit above the floor; settle with a swept body,
        // including after a platform moves away, instead of firing while suspended.
        self.falling = (self.falling + 800. * dt).min(800.);
        let body = self.target(0);
        let down = world.sweep(
            body.center,
            body.center - Vec3::Z * self.falling * dt,
            body.half,
        );
        if !down.start_solid {
            self.feet.z -= self.falling * dt * down.fraction;
        }
        if down.start_solid || down.fraction < 1. {
            self.falling = 0.;
        }
        if self.state == State::Dead {
            if self.cut {
                let recipe = timing
                    .sever
                    .map(|recipes| &recipes[self.cut_variant as usize]);
                if let Some(recipe) = recipe {
                    for command in recipe.events.between(
                        self.clip(),
                        crate::animation_events::Span {
                            start: self.time - dt,
                            end: self.time,
                            duration: recipe.duration,
                            frame_time: recipe.frame_time,
                            looping: false,
                            entered: false,
                        },
                    ) {
                        if let crate::animation_events::Command::Sound { path, .. } = command {
                            out.spatial_sounds.push((path.as_str(), self.feet));
                        }
                    }
                }
                self.dismember.update(
                    dt, self.time, self.feet, self.yaw, self.scale, world, recipe,
                );
            }
            self.time = self.time.min(if self.frozen {
                2.5
            } else if self.cut {
                crate::dismember::LIFE + crate::dismember::CUT_FRAME_TIME
            } else {
                timing.death
            });
            return;
        }
        let (eye, notarget, victim) = self.opponents.aim(
            world,
            self.target(0).center,
            eye,
            self.notarget,
            self.state == State::Attack,
        );
        let delta = eye - self.target(0).center;
        let reach = if victim == crate::dice::SUMMON {
            self.opponents.summon.map_or(Vec3::ZERO, |t| {
                (t.half - crate::collision::PLAYER_HALF).max(Vec3::ZERO)
            })
        } else {
            Vec3::ZERO
        };
        let distance = delta.truncate().length();
        let aware = !notarget
            && delta.length_squared()
                < if self.ranged {
                    900. * 900.
                } else {
                    420. * 420.
                }
            && eye.distance_squared(self.home)
                < if self.ranged {
                    1000. * 1000.
                } else {
                    700. * 700.
                };
        // Distant actors should not trace across the entire level at 120 Hz.
        let visible = aware && {
            let sight = world.sweep(self.target(0).center, eye, Vec3::splat(0.5));
            !sight.start_solid && sight.fraction >= 1.
        };
        let aware = aware && visible;
        if !matches!(self.state, State::Attack | State::Pain) && aware {
            let angle = delta.y.atan2(delta.x);
            let difference = (angle - self.yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.yaw += difference.clamp(-dt * 4., dt * 4.);
        }
        match self.state {
            State::Idle if aware => {
                self.set_state(State::Alert);
                out.sounds.push(if self.ranged {
                    "sound/character/cardguard/diamond/alert1.wav"
                } else {
                    "sound/character/cardguard/club/alert1.wav"
                });
            }
            State::Alert if self.time >= timing.alert => self.set_state(State::Chase),
            State::Pain if self.time >= timing.pain => self.set_state(State::Chase),
            State::Attack => {
                if !self.hit && self.time >= timing.hit {
                    self.hit = true;
                    if self.ranged && visible {
                        let position = self.target(0).center;
                        self.shots.push(Bolt {
                            position,
                            direction: (eye - Vec3::Z * 16. - position).normalize_or_zero(),
                            age: 0.,
                        });
                        self.attacks += 1;
                    }
                    if !self.ranged
                        && visible
                        && distance <= 78. * self.scale + reach.x
                        && delta.z.abs() < 55. * self.scale + reach.z
                        && delta
                            .truncate()
                            .normalize_or_zero()
                            .dot(vec2(self.yaw.cos(), self.yaw.sin()))
                            > 0.5
                    {
                        out.strike(victim, 12., Vec3::ZERO, DamageKind::Other);
                    }
                }
                if self.time >= timing.attack {
                    self.cooldown = 0.65;
                    self.set_state(State::Chase);
                }
            }
            State::Chase => {
                if !aware {
                    self.set_state(State::Idle);
                } else if (self.ranged && distance < 600.)
                    || (distance < 60. * self.scale + reach.x
                        && delta.z.abs() < 55. * self.scale + reach.z)
                {
                    if self.cooldown <= 0. {
                        self.set_state(State::Attack);
                        out.sounds.push(if self.ranged {
                            "sound/character/cardguard/diamond/stand_attack.wav"
                        } else {
                            "sound/character/cardguard/club/attack1.wav"
                        });
                    }
                } else if self.feet.distance(self.home) < 500. {
                    let wish = delta.truncate().normalize_or_zero().extend(0.) * 95. * dt;
                    self.feet = walk(world, self.feet, wish, self.scale);
                }
            }
            _ => (),
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bolt {
    pub position: Vec3,
    pub direction: Vec3,
    pub age: f32,
}
impl Bolt {
    fn advance(&mut self, dt: f32, world: &World, targets: &[Target], out: &mut Feedback) -> bool {
        let next = self.position + self.direction * 700. * dt;
        if let Some((id, _)) = contact(&Context { world, targets }, self.position, next, 8.) {
            out.strike(id, 7., self.direction * 150., DamageKind::Electric);
            out.sounds
                .push("sound/character/cardguard/diamond/prj_hit_flesh1.wav");
            return false;
        }
        let t = world.sweep(self.position, next, Vec3::splat(8.));
        self.position = self.position.lerp(next, t.fraction);
        self.age += dt;
        !t.start_solid && t.fraction >= 1. && self.age < 5.
    }
}
/// Local steering only: checked headroom, body clearance and support under all four corners.
fn walk(world: &World, feet: Vec3, delta: Vec3, scale: f32) -> Vec3 {
    let half = vec3(24., 24., 40.) * scale;
    walk_body(world, feet, delta, half)
}
pub(crate) fn walk_body(world: &World, feet: Vec3, delta: Vec3, half: Vec3) -> Vec3 {
    walk_body_liquids(world, feet, delta, half, 0)
}
/// Supported liquid bits only; solid support and cliff checks still apply.
pub(crate) fn walk_body_liquids(
    world: &World,
    feet: Vec3,
    delta: Vec3,
    half: Vec3,
    allowed: i32,
) -> Vec3 {
    let lift = Vec3::Z * 16.;
    let center = feet + Vec3::Z * (half.z + 0.1);
    let up = world.sweep(center, center + lift, half);
    if up.start_solid {
        return feet;
    }
    let raised = center.lerp(center + lift, up.fraction);
    let across = world.sweep(raised, raised + delta, half);
    if across.start_solid || across.fraction < 1. {
        return feet;
    }
    let down = world.sweep(raised + delta, raised + delta - Vec3::Z * 34., half);
    if down.start_solid || down.fraction >= 1. || down.normal.z < 0.65 {
        return feet;
    }
    let next =
        (raised + delta).lerp(raised + delta - Vec3::Z * 34., down.fraction) - Vec3::Z * half.z;
    for x in [-half.x + 1., half.x - 1.] {
        for y in [-half.y + 1., half.y - 1.] {
            let p = next + vec3(x, y, 2.);
            // A hull straddling a legal step still needs support on its lower side.
            let support = world.sweep(p, p - Vec3::Z * 18., Vec3::ZERO);
            if support.start_solid || support.fraction >= 1. || support.normal.z < 0.65 {
                return feet;
            }
        }
    }
    if world.liquid_at(next + Vec3::Z * 4.) & !allowed != 0 {
        return feet;
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guards_retaliate_at_demons_without_redirecting_old_shots_to_alice() {
        let world = floor();
        for ranged in [false, true] {
            let mut g = Guard::new(Vec3::Z * 0.1, 0., 1.);
            g.ranged = ranged;
            g.notarget = true;
            g.opponents.summon = Some(Target {
                id: crate::dice::SUMMON,
                center: vec3(if ranged { 300. } else { 55. }, 0., 40.),
                half: vec3(32., 32., 40.),
            });
            let mut damage = 0.;
            for _ in 0..120 * 5 {
                let f = g.advance(1. / 120., &world, vec3(-800., 0., 48.), timing());
                assert_eq!(f.damage, 0.);
                damage += f.summon_hits.iter().map(|h| h.damage).sum::<f32>();
            }
            assert!(damage > 0., "ranged={ranged}");
            let saved = serde_json::to_value(&g).unwrap();
            let restored: Guard = serde_json::from_value(saved).unwrap();
            assert!(restored.opponents.demon);
            assert!(restored.opponents.summon.is_none());
        }
        let opponents = Opponents {
            summon: Some(Target {
                id: crate::dice::SUMMON,
                center: vec3(100., 0., 40.),
                half: Vec3::splat(20.),
            }),
            demon: false,
        };
        let mut bolt = Bolt {
            position: vec3(0., 0., 40.),
            direction: Vec3::X,
            age: 0.,
        };
        let mut f = Feedback::default();
        assert!(!bolt.advance(0.3, &world, &opponents.bodies(vec3(180., 0., 60.)), &mut f));
        assert_eq!(f.damage, 0.);
        assert_eq!(f.summon_hits[0].damage, 7.);
    }
    fn timing() -> Timing {
        Timing {
            sever: None,
            alert: 0.3,
            attack: 0.8,
            hit: 0.4,
            pain: 0.3,
            death: 1.,
        }
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.))])
    }
    #[test]
    fn cards_recoil_uses_mass_persists_and_cannot_push_through_solid_walls() {
        let world = World::fixture(&[(vec3(30., -100., -100.), vec3(35., 100., 100.))]);
        let mut light = Recoil::default();
        let mut heavy = Recoil::default();
        light.hit(Vec3::X * 60., 150.);
        heavy.hit(Vec3::X * 60., 200.);
        assert_eq!(light.velocity.x, 200.);
        assert_eq!(heavy.velocity.x, 150.);
        let mut restored: Recoil =
            serde_json::from_value(serde_json::to_value(&heavy).unwrap()).unwrap();
        let mut center = Vec3::ZERO;
        for _ in 0..120 {
            let body = Target {
                id: 0,
                center,
                half: Vec3::splat(8.),
            };
            let delta = heavy.step(1. / 120., &world, body);
            assert_eq!(delta, restored.step(1. / 120., &world, body));
            center += delta;
        }
        assert!(center.x > 10. && center.x <= 22.);
        assert!(heavy.valid());
    }
    #[test]
    fn spawn_settles_while_combat_pauses_then_later_falls_use_gravity() {
        let mut g = Guard::new(Vec3::Z * 90., 0., 1.);
        g.advance(0., &floor(), Vec3::X * 100., timing());
        assert!(g.feet.z >= 0. && g.feet.z < 0.04);
        assert_eq!((g.time, g.attacks, g.health), (0., 0, 55.));
        let before = g.feet;
        g.advance(0., &World::fixture(&[]), Vec3::X * 100., timing());
        assert_eq!(g.feet, before);
        g.advance(
            1. / 60.,
            &World::fixture(&[]),
            Vec3::splat(10000.),
            timing(),
        );
        assert!(g.feet.z < before.z && g.feet.z > before.z - 1.);
        let mut old = serde_json::to_value(&g).unwrap();
        old.as_object_mut().unwrap().remove("placed");
        let mut old: Guard = serde_json::from_value(old).unwrap();
        old.place(&floor());
        assert!(old.feet.z >= 0. && old.feet.z < 0.04);
    }
    #[test]
    fn swept_contacts_choose_nearest_and_never_cross_walls() {
        let targets = [
            Target {
                id: 7,
                center: vec3(150., 0., 40.),
                half: Vec3::splat(10.),
            },
            Target {
                id: 2,
                center: vec3(80., 0., 40.),
                half: Vec3::splat(10.),
            },
        ];
        let wall = World::fixture(&[(vec3(100., -50., 0.), vec3(105., 50., 100.))]);
        let ctx = Context {
            world: &wall,
            targets: &targets,
        };
        assert_eq!(
            contact(&ctx, vec3(0., 0., 40.), vec3(500., 0., 40.), 1.)
                .unwrap()
                .0,
            2
        );
        assert!(contact(
            &Context {
                world: &wall,
                targets: &targets[..1]
            },
            vec3(0., 0., 40.),
            vec3(500., 0., 40.),
            1.
        )
        .is_none());
        assert!(contact(&ctx, vec3(102., 0., 40.), vec3(500., 0., 40.), 1.).is_none());
        assert!(contact(&ctx, vec3(0., 80., 40.), vec3(500., 80., 40.), 1.).is_none());
    }
    #[test]
    fn pursuit_attack_and_pause_agree_across_frame_rates() {
        let mut results = vec![];
        for fps in [30, 60, 144] {
            let mut g = Guard::new(Vec3::ZERO, 0., 1.);
            let mut damage = 0.;
            for _ in 0..fps * 5 {
                damage += g
                    .advance(1. / fps as f32, &floor(), vec3(200., 0., 48.), timing())
                    .damage;
            }
            results.push((g.feet, damage));
            let before = (g.time, g.feet, g.health);
            g.advance(0., &floor(), Vec3::ZERO, timing());
            assert_eq!(before, (g.time, g.feet, g.health));
        }
        assert!(results[0].1 > 0.);
        for r in &results[1..] {
            assert!((r.0 - results[0].0).length() < 0.1);
            assert_eq!(r.1, results[0].1);
        }
    }
    #[test]
    fn attack_contact_is_once_and_can_be_dodged_or_blocked() {
        let mut g = Guard::new(Vec3::ZERO, 0., 1.);
        g.set_state(State::Attack);
        let mut damage = 0.;
        for _ in 0..70 {
            damage += g
                .advance(1. / 120., &floor(), vec3(50., 0., 48.), timing())
                .damage;
        }
        assert_eq!(damage, 12.);
        for eye in [
            vec3(-50., 0., 48.),
            vec3(150., 0., 48.),
            vec3(50., 0., 200.),
        ] {
            let mut g = Guard::new(Vec3::ZERO, 0., 1.);
            g.set_state(State::Attack);
            let mut damage = 0.;
            for _ in 0..70 {
                damage += g.advance(1. / 120., &floor(), eye, timing()).damage;
            }
            assert_eq!(damage, 0.);
        }
        let wall = World::fixture(&[(vec3(20., -50., 0.), vec3(25., 50., 100.))]);
        let mut g = Guard::new(Vec3::ZERO, 0., 1.);
        g.set_state(State::Attack);
        for _ in 0..70 {
            assert_eq!(
                g.advance(1. / 120., &wall, vec3(50., 0., 48.), timing())
                    .damage,
                0.
            );
        }
    }
    #[test]
    fn guard_does_not_walk_through_wall_or_off_ledge() {
        let world = World::fixture(&[(vec3(-100., -100., -20.), vec3(60., 100., 0.))]);
        let mut feet = Vec3::ZERO;
        for _ in 0..300 {
            feet = walk(&world, feet, Vec3::X, 1.);
        }
        assert!(feet.x < 38. && feet.x > 25.);
        let wall = World::fixture(&[
            (vec3(-100., -100., -20.), vec3(200., 100., 0.)),
            (vec3(60., -100., 0.), vec3(65., 100., 100.)),
        ]);
        let mut feet = Vec3::ZERO;
        for _ in 0..300 {
            feet = walk(&wall, feet, Vec3::X, 1.);
        }
        assert!(feet.x < 37.);
    }
    #[test]
    fn death_cancels_attack_and_remains_dead_until_reset() {
        let mut g = Guard::new(Vec3::ZERO, 0., 1.);
        g.set_state(State::Attack);
        g.hurt(20.);
        assert_eq!(g.health, 35.);
        assert_eq!(g.state, State::Pain);
        g.hurt(35.);
        assert_eq!(g.state, State::Dead);
        assert!(g.hurt(20.).is_none());
        for _ in 0..600 {
            assert_eq!(
                g.advance(1. / 60., &floor(), vec3(50., 0., 48.), timing())
                    .damage,
                0.
            );
        }
        assert_eq!(g.time, 1.);
    }
    #[test]
    fn diamond_shots_agree_across_frame_rates_and_need_sight() {
        let world = floor();
        let mut results = Vec::new();
        for fps in [30, 60, 144] {
            let mut g = Guard::diamond(Vec3::ZERO, 0., 1.);
            let mut damage = 0.;
            for _ in 0..fps * 5 {
                damage += g
                    .advance(1. / fps as f32, &world, vec3(300., 0., 48.), timing())
                    .damage;
            }
            assert!(g.attacks >= 2 && damage >= 14.);
            results.push((g.attacks, damage));
        }
        assert!(results.iter().all(|r| *r == results[0]));
        let wall = World::fixture(&[
            (vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.)),
            (vec3(120., -100., 0.), vec3(125., 100., 120.)),
        ]);
        let mut g = Guard::diamond(Vec3::ZERO, 0., 1.);
        for _ in 0..600 {
            assert_eq!(
                g.advance(1. / 120., &wall, vec3(300., 0., 48.), timing())
                    .damage,
                0.
            );
        }
        assert_eq!(g.attacks, 0);
    }
    #[test]
    fn diamond_projectile_is_swept_and_can_be_dodged() {
        let shot = Bolt {
            position: vec3(0., 0., 40.),
            direction: Vec3::X,
            age: 0.,
        };
        let mut hit = shot.clone();
        let mut out = Feedback::default();
        assert!(!hit.advance(
            0.5,
            &floor(),
            &Opponents::default().bodies(vec3(250., 0., 48.)),
            &mut out
        ));
        assert_eq!(out.damage, 7.);
        let mut dodge = shot.clone();
        let mut out = Feedback::default();
        assert!(dodge.advance(
            0.5,
            &floor(),
            &Opponents::default().bodies(vec3(250., 100., 48.)),
            &mut out
        ));
        assert_eq!(out.damage, 0.);
        let wall = World::fixture(&[(vec3(120., -100., 0.), vec3(125., 100., 120.))]);
        let mut blocked = shot;
        assert!(!blocked.advance(
            0.5,
            &wall,
            &Opponents::default().bodies(vec3(250., 0., 48.)),
            &mut out
        ));
        assert_eq!(out.damage, 0.);
    }
    #[test]
    fn invisibility_prevents_new_guard_attacks_but_is_not_invulnerability() {
        for ranged in [false, true] {
            let mut g = Guard::new(Vec3::ZERO, 0., 1.);
            g.ranged = ranged;
            g.notarget = true;
            for _ in 0..240 {
                assert_eq!(
                    g.advance(1. / 120., &floor(), vec3(50., 0., 48.), timing())
                        .damage,
                    0.
                );
            }
            assert_eq!(g.state, State::Idle);
            assert_eq!(g.attacks, 0);
            g.notarget = false;
            let mut damage = 0.;
            for _ in 0..240 {
                damage += g
                    .advance(1. / 120., &floor(), vec3(50., 0., 48.), timing())
                    .damage;
            }
            assert!(damage > 0.);
        }
        let mut g = Guard::diamond(Vec3::ZERO, 0., 1.);
        g.notarget = true;
        g.shots.push(Bolt {
            position: vec3(40., 0., 40.),
            direction: Vec3::X,
            age: 0.,
        });
        assert_eq!(
            g.advance(0.05, &floor(), vec3(70., 0., 48.), timing())
                .damage,
            7.
        );
    }
}

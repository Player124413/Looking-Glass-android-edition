//! Fixed-step prototype movement. Parameters are provisional, not recovered Alice physics.
use crate::collision::World;
use crate::water::Immersion;
#[cfg(test)]
use macroquad::prelude::vec3;
use macroquad::prelude::{Vec2, Vec3};

pub const EYE_HEIGHT: f32 = 48.;
pub const FIXED_DT: f32 = 1. / 120.;
// Brisk walk: about two authored strides per second (51.3-51.7 units each).
pub const WALK_SPEED: f32 = 104.;
const STEP_HEIGHT: f32 = 18.;
pub const GRAVITY: f32 = 800.;
// Local Player jump event: half gravity in the ordinary, unpowered state.
// Horizontal acceleration and animation timing remain provisional.
const JUMP_SPEED: f32 = GRAVITY * 0.5;
const GROUND_Z: f32 = 0.65;
/// Upright swimmer's shoulders are near 44 units; keep the eye just above water.
pub const FLOAT_DEPTH: f32 = 44.;

#[derive(Clone, Copy, Default)]
pub struct Controls {
    pub use_pressed: bool,
    pub wish: Vec2,
    /// Full look-relative direction, used only in water.
    pub swim: Vec3,
    /// Held ascent/descent input; unlike jumping this is not edge-triggered.
    pub rise: f32,
    pub jump: bool,
    pub run: bool,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Climb {
    start: Vec3,
    raised: Vec3,
    end: Vec3,
    elapsed: f32,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RopeGrip {
    pub id: crate::entity::Id,
    pub length: f32,
    pub rise: f32,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Player {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledge: Option<crate::ledge::Hang>,
    #[serde(default, skip_serializing_if = "zero")]
    ledge_cooldown: f32,
    #[serde(skip)]
    pub tea: bool,
    #[serde(default)]
    pub breath: crate::water::Breath,
    #[serde(default)]
    pub rope: Option<RopeGrip>,
    #[serde(default)]
    rope_cooldown: f32,
    #[serde(default)]
    pub knockback_time: f32,
    #[serde(default)]
    pub updraft_time: f32,
    #[serde(default)]
    pub script_motion: u8,
    /// Per-frame presentation from a controller-owned rope; the grip itself is
    /// already saved by that controller. A resumed simulation recomputes this.
    #[serde(skip)]
    pub script_rope_rise: f32,
    #[serde(skip)]
    pub script_rope_length: Option<f32>,
    #[serde(default)]
    pub script_facing: f32,
    pub steam: bool,
    pub feet: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub ground_normal: Vec3,
    pub jumps: u64,
    pub landings: u64,
    pub landing_speed: f32,
    pub immersion: Immersion,
    pub swimming: bool,
    pub liquid_damage: f32,
    pub climbs: u64,
    pub climb_height: f32,
    pub climb_direction: Vec2,
    climb: Option<Climb>,
}
impl Player {
    pub fn validate_save(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            (0. ..=0.5).contains(&self.ledge_cooldown),
            "Invalid ledge release timer"
        );
        if let Some(hang) = &self.ledge {
            hang.validate()?;
            anyhow::ensure!(
                self.rope.is_none()
                    && self.climb.is_none()
                    && self.script_motion == 0
                    && !self.grounded
                    && !self.swimming,
                "Conflicting saved ledge state"
            );
        }
        self.breath.validate()?;
        anyhow::ensure!(
            (0.0..=0.1).contains(&self.updraft_time)
                && (0.0..=0.5).contains(&self.rope_cooldown)
                && (0.0..=0.25).contains(&self.knockback_time)
                && self.rope.as_ref().is_none_or(|r| r.length.is_finite()
                    && r.length >= 32.
                    && r.length <= 10000.
                    && (-1.0..=1.).contains(&r.rise)),
            "Invalid saved movement restraint"
        );
        anyhow::ensure!(
            self.script_motion <= 3
                && self.script_facing.is_finite()
                && self.velocity.length() <= 10000.
                && self.ground_normal.is_finite()
                && self.ground_normal.length_squared() <= 1.01
                && self.immersion.level <= 3
                && self.liquid_damage >= 0.,
            "Invalid saved movement state: velocity {:?}, ground {:?}, motion {}, facing {}, water {}, damage {}",
            self.velocity, self.ground_normal, self.script_motion, self.script_facing, self.immersion.level, self.liquid_damage
        );
        if let Some(c) = &self.climb {
            anyhow::ensure!(
                (0.0..=0.65).contains(&c.elapsed)
                    && c.start.is_finite()
                    && c.raised.is_finite()
                    && c.end.is_finite(),
                "Invalid saved climb"
            );
        }
        Ok(())
    }
    pub fn new(feet: Vec3) -> Self {
        Self {
            ledge: None,
            ledge_cooldown: 0.,
            tea: false,
            breath: Default::default(),
            rope: None,
            rope_cooldown: 0.,
            knockback_time: 0.,
            updraft_time: 0.,
            script_motion: 0,
            script_rope_rise: 0.,
            script_rope_length: None,
            script_facing: 0.,
            steam: false,
            feet,
            velocity: Vec3::ZERO,
            grounded: false,
            ground_normal: Vec3::Z,
            jumps: 0,
            landings: 0,
            landing_speed: 0.,
            immersion: Immersion::default(),
            swimming: false,
            liquid_damage: 0.,
            climbs: 0,
            climb_height: 0.,
            climb_direction: Vec2::ZERO,
            climb: None,
        }
    }
    pub fn cancel_climb(&mut self) {
        self.climb = None;
        if self.ledge.take().is_some() {
            self.ledge_cooldown = 0.5;
        }
    }
    pub fn release_rope(&mut self) {
        self.rope = None;
        self.rope_cooldown = 0.5;
        if self.script_motion == 1 {
            self.script_motion = 0;
        }
    }
    /// Apply an authored impulse once. Briefly protect it from input acceleration;
    /// collision still clips the resulting velocity on every fixed tick.
    pub fn knockback(&mut self, impulse: Vec3) {
        if !impulse.is_finite() || impulse.length_squared() < 0.01 {
            return;
        }
        self.cancel_climb();
        self.release_rope();
        self.velocity = (self.velocity + impulse).clamp_length_max(10000.);
        self.knockback_time = 0.2;
        if self.velocity.z > 1. {
            self.grounded = false;
        }
    }
    pub fn rope_prompt(&self, world: &World) -> Option<&'static str> {
        if self.ledge.is_some() {
            Some("Forward or Space pull up / A-D move along edge / Back or Ctrl let go")
        } else if self.rope.is_some() {
            Some("E release rope / Space up / Ctrl down / WASD swing")
        } else {
            world
                .traversal
                .nearest_rope(world, self.eye(), 76.)
                .map(|_| "E grab rope")
        }
    }
    pub fn validate_world(&self, world: &World) -> anyhow::Result<()> {
        if let Some(hang) = &self.ledge {
            anyhow::ensure!(
                hang.valid_contact(world)
                    && hang.matches_position(world, self.feet)
                    && world.body_clear(self.feet),
                "Saved ledge is unavailable"
            );
        }
        if let Some(g) = &self.rope {
            let r = world
                .traversal
                .rope(g.id)
                .ok_or_else(|| anyhow::anyhow!("Saved rope is unavailable"))?;
            anyhow::ensure!(
                g.length <= r.length
                    && ((self.feet + Vec3::Z * 40.).distance(r.anchor) - g.length).abs() < 0.1
                    && self.script_motion == 1
                    && self.climb.is_none(),
                "Saved rope grip is detached"
            );
        }
        Ok(())
    }
    fn tick_rope(&mut self, world: &World, input: Controls) -> bool {
        if input.use_pressed && self.rope.is_some() {
            self.release_rope();
            if input.rise > 0. {
                self.velocity.z = self.velocity.z.max(JUMP_SPEED);
            }
            self.knockback_time = 0.2;
            return false;
        }
        if self.rope.is_none() && self.rope_cooldown <= 0. && self.knockback_time <= 0. {
            let automatic = !self.grounded && self.immersion.level < 2;
            if input.use_pressed || automatic {
                if let Some(r) = world.traversal.nearest_rope(
                    world,
                    self.eye(),
                    if input.use_pressed { 76. } else { 28. },
                ) {
                    let hand = self.feet + Vec3::Z * 40.;
                    let length = hand.distance(r.anchor).clamp(32., r.length);
                    let end =
                        r.anchor + (hand - r.anchor).normalize_or_zero() * length - Vec3::Z * 40.;
                    let t = world.body_trace(self.feet, end);
                    if !t.start_solid && t.fraction >= 1. && world.body_clear(end) {
                        self.feet = end;
                        self.climb = None;
                        self.rope = Some(RopeGrip {
                            id: r.id,
                            length,
                            rise: 0.,
                        });
                    }
                }
            }
        }
        let Some(g) = &mut self.rope else {
            return false;
        };
        let Some(r) = world.traversal.rope(g.id) else {
            self.release_rope();
            return false;
        };
        let before = g.length;
        crate::rope::step_grip(
            world,
            r.anchor,
            r.length,
            &mut self.feet,
            &mut g.length,
            &mut self.velocity,
            input,
            FIXED_DT,
        );
        g.rise = if (before - g.length).abs() > 0.001 {
            (before - g.length).signum()
        } else {
            0.
        };
        self.script_facing =
            crate::rope::grip_facing(r.anchor, self.feet, self.script_facing, FIXED_DT);
        self.script_motion = 1;
        self.grounded = false;
        self.swimming = false;
        self.immersion = Immersion::sample(world, self.feet);
        true
    }
    pub fn eye(&self) -> Vec3 {
        self.feet + Vec3::Z * EYE_HEIGHT
    }
    /// A blocked spawn may be lifted slightly, never teleported sideways through geometry.
    pub fn spawn(world: &World, eye: Vec3) -> Option<Self> {
        let base = eye - Vec3::Z * EYE_HEIGHT;
        (0..=16)
            .map(|i| base + Vec3::Z * (i * 8) as f32)
            .find(|&p| world.body_clear(p))
            .map(|feet| {
                let mut player = Self::new(feet);
                player.immersion = Immersion::sample(world, feet);
                player.swimming = player.immersion.level >= 2;
                // The first rendered pose must already reflect actual support.
                // Elevated cinematic starts remain airborne until they land.
                let support = world.body_trace(feet, feet - Vec3::Z * 2.);
                player.grounded = !player.swimming
                    && !support.start_solid
                    && support.fraction < 1.
                    && support.normal.z >= GROUND_Z;
                if player.grounded {
                    player.ground_normal = support.normal;
                }
                player
            })
    }
    pub fn tick(&mut self, world: &World, input: Controls) {
        self.ledge_cooldown = (self.ledge_cooldown - FIXED_DT).max(0.);
        self.rope_cooldown = (self.rope_cooldown - FIXED_DT).max(0.);
        self.knockback_time = (self.knockback_time - FIXED_DT).max(0.);
        self.steam = world.traversal.updraft(self.feet);
        self.updraft_time = if self.steam {
            0.1
        } else {
            (self.updraft_time - FIXED_DT).max(0.)
        };
        let gravity = if self.updraft_time > 0. {
            -GRAVITY * 0.25
        } else {
            GRAVITY
        };
        if self.steam {
            // The original damps rising/falling velocity above +/-50 per contact.
            if self.velocity.z > 50. {
                self.velocity.z *= 0.9_f32.powf(FIXED_DT * 20.);
            } else if self.velocity.z < -50. {
                self.velocity.z *= 0.95_f32.powf(FIXED_DT * 20.);
            }
        }
        if let Some(feet) = world.correct_footing(self.feet) {
            self.feet = feet;
            self.velocity = Vec3::ZERO;
            self.climb = None;
        }
        self.immersion = Immersion::sample(world, self.feet);
        self.liquid_damage +=
            self.immersion.kind.damage_per_second() * self.immersion.level as f32 * FIXED_DT;
        self.liquid_damage += self.breath.tick(self.immersion.level == 3);
        if let Some(mut hang) = self.ledge.take() {
            if let Some((feet, complete)) = hang.tick(world, self.feet, input) {
                self.feet = feet;
                self.velocity = Vec3::ZERO;
                self.grounded = complete;
                self.swimming = false;
                self.climb_direction = hang.forward;
                self.immersion = Immersion::sample(world, feet);
                if complete {
                    self.ledge_cooldown = 0.5;
                    self.climbs += 1;
                } else {
                    self.ledge = Some(hang);
                }
                return;
            }
            self.ledge_cooldown = 0.5;
            self.velocity = -Vec3::Z * 30.;
        }
        if self.tick_rope(world, input) {
            return;
        }
        if self.climb.is_some() {
            self.tick_climb(world);
            return;
        }
        let previous_ground = self.grounded;
        let ground = world.body_trace(self.feet, self.feet - Vec3::Z * 2.);
        self.grounded = !ground.start_solid
            && ground.fraction < 1.
            && ground.normal.z >= GROUND_Z
            && self.velocity.dot(ground.normal) <= 1.;
        if self.grounded {
            self.ground_normal = ground.normal;
        }
        if self.updraft_time > 0. {
            self.grounded = false;
        }
        let was_swimming = self.swimming;
        self.swimming = self.immersion.level == 3
            // A jump from a shallow, supported pool must keep its ascent. Switching
            // to swimming here clamps it back to the float line before it can clear
            // an ordinary bank (notably Herbaceous Border's slime basin).
            || (self.immersion.level >= 2 && !self.grounded
                && (was_swimming || self.velocity.z <= 0.))
            || (was_swimming && self.immersion.level == 1 && !self.grounded);
        if ((self.swimming && input.rise > 0.)
            || (self.grounded && (input.jump || input.use_pressed)))
            && self.knockback_time <= 0.
            && self.immersion.level < 3
            && self.try_climb(world, input.wish)
        {
            self.tick_climb(world);
            return;
        }
        if self.swimming {
            self.grounded = false;
            if !was_swimming {
                // A deep pool arrests a fall before its bottom can create a hard landing.
                self.velocity = self.velocity.clamp_length_max(180.);
            }
            let wish = (input.swim + Vec3::Z * input.rise).clamp_length_max(1.);
            let target =
                wish * if input.run { 220. } else { 170. } * if self.tea { 2. } else { 1. };
            if self.knockback_time <= 0. {
                self.velocity += (target - self.velocity).clamp_length_max(700. * FIXED_DT);
            }
            world.traversal.push(self.feet, &mut self.velocity);
            crate::traversal::current::apply(&world.traversal.currents, world, self.feet, &mut self.velocity);
            let (mut next, mut velocity) = slide(world, self.feet, self.velocity, FIXED_DT);
            let bank = world.body_trace(next, next - Vec3::Z * 2.);
            let supported = !bank.start_solid
                && bank.fraction < 1.
                && bank.normal.z >= GROUND_Z
                && Immersion::sample(world, next).level < 3;
            // Float at chest depth. Space at an edge attempts the checked climb above;
            // holding it in open water must not repeatedly launch Alice into the air.
            if !supported
                && world.liquid_at(next + Vec3::Z) != 0
                && world.liquid_at(next + Vec3::Z * FLOAT_DEPTH) == 0
            {
                let (mut low, mut high) = (1., FLOAT_DEPTH);
                for _ in 0..12 {
                    let mid = (low + high) * 0.5;
                    if world.liquid_at(next + Vec3::Z * mid) != 0 {
                        low = mid;
                    } else {
                        high = mid;
                    }
                }
                // Also settle a swimmer who entered above the float line. Merely
                // testing an upward crossing lets a shallow entry rise out of water.
                let capped = vec3_z(next, (next.z + low - FLOAT_DEPTH).max(self.feet.z - 2.));
                // Capping height changes the slide endpoint. Re-sweep: a sloping bank
                // can occupy the lowered endpoint even though the original slide was clear.
                let hit = world.body_trace(self.feet, capped);
                // Preserve the uncapped slide when it climbed a walkable bank.
                if hit.fraction >= 1. || hit.normal.z < GROUND_Z {
                    next = self.feet.lerp(capped, hit.fraction);
                    velocity.z = velocity.z.min(0.);
                }
            }
            self.feet = next;
            self.velocity = velocity;
            self.immersion = Immersion::sample(world, self.feet);
            if supported {
                self.grounded = true;
                self.ground_normal = bank.normal;
            }
            if self.immersion.level == 0 || supported {
                self.swimming = false;
            }
            return;
        }
        let jump = input.jump && self.grounded;
        let wish = input.wish.clamp_length_max(1.);
        let target = wish
            * if input.run { 320. } else { WALK_SPEED }
            * if self.tea { 2. } else { 1. }
            * if self.immersion.level > 0 { 0.7 } else { 1. };
        let mut horizontal = self.velocity.truncate();
        if self.knockback_time <= 0. && (self.grounded || wish.length_squared() > 0.) {
            let acceleration = if self.grounded { 1800. } else { 420. };
            horizontal += (target - horizontal).clamp_length_max(acceleration * FIXED_DT);
        }
        self.velocity.x = horizontal.x;
        self.velocity.y = horizontal.y;
        if jump {
            self.velocity.z = JUMP_SPEED * if self.tea { 1.5 } else { 1. };
            self.grounded = false;
            self.jumps += 1;
        } else if self.grounded {
            self.velocity.z = -(self.ground_normal.x * self.velocity.x
                + self.ground_normal.y * self.velocity.y)
                / self.ground_normal.z;
        }
        let before_push_z = self.velocity.z;
        let launch = world.traversal.push(self.feet, &mut self.velocity);
        crate::traversal::current::apply(&world.traversal.currents, world, self.feet, &mut self.velocity);
        if launch {
            self.knockback_time = 0.2;
        }
        if launch || self.velocity.z > before_push_z + 1. {
            self.grounded = false;
        }
        let was_ground = self.grounded;
        if !self.grounded {
            self.velocity.z -= gravity * FIXED_DT * 0.5;
        }
        let start = self.feet;
        let velocity = self.velocity;
        let (mut next, mut next_velocity) = slide(world, start, velocity, FIXED_DT);
        if was_ground && horizontal.length_squared() > 0.1 {
            let up = world.body_trace(start, start + Vec3::Z * STEP_HEIGHT);
            if !up.start_solid && !up.all_solid && up.fraction > 0. {
                let height = STEP_HEIGHT * up.fraction;
                let raised = start + Vec3::Z * height;
                let (step, step_velocity) = slide(world, raised, velocity, FIXED_DT);
                let down = world.body_trace(step, step - Vec3::Z * (height + 2.));
                if !down.start_solid && down.fraction < 1. && down.normal.z >= GROUND_Z {
                    let step = step - Vec3::Z * (height + 2.) * down.fraction;
                    if (step - start).truncate().length_squared()
                        > (next - start).truncate().length_squared() + 0.0001
                    {
                        next = step;
                        next_velocity = step_velocity;
                    }
                }
            }
        }
        self.feet = next;
        self.velocity = next_velocity;
        if !was_ground
            && self.ledge_cooldown <= 0.
            && self.knockback_time <= 0.
            && self.immersion.level < 2
            && self.updraft_time <= 0.
        {
            let direction = if input.wish.length_squared() > 0.1 {
                input.wish
            } else {
                velocity.truncate()
            };
            if let Some(hang) =
                crate::ledge::Hang::catch(world, start, self.feet, velocity, direction, input)
            {
                self.feet = hang.start;
                self.velocity = Vec3::ZERO;
                self.climb_direction = hang.forward;
                self.grounded = false;
                self.ledge = Some(hang);
                return;
            }
        }
        if !was_ground {
            self.velocity.z -= gravity * FIXED_DT * 0.5;
        }
        // Only supported walking follows small downward steps; airborne players keep their arc.
        let distance = if was_ground && !jump {
            STEP_HEIGHT + 2.
        } else {
            2.
        };
        let down = world.body_trace(self.feet, self.feet - Vec3::Z * distance);
        self.grounded = !down.start_solid
            && down.fraction < 1.
            && down.normal.z >= GROUND_Z
            && !jump
            && (was_ground || self.velocity.dot(down.normal) <= 1.);
        if self.grounded {
            self.feet -= Vec3::Z * distance * down.fraction;
            self.ground_normal = down.normal;
            self.velocity -= self.ground_normal * self.velocity.dot(self.ground_normal);
            if !previous_ground {
                self.landings += 1;
                self.landing_speed =
                    (-velocity.z).max(0.) * if self.immersion.level > 0 { 0.5 } else { 1. };
            }
        }
        self.immersion = Immersion::sample(world, self.feet);
    }
    pub fn climbing(&self) -> bool {
        self.climb.is_some()
    }
    pub fn climb_progress(&self) -> Option<f32> {
        self.climb
            .as_ref()
            .map(|c| (c.elapsed / 0.65).clamp(0., 1.))
    }
    fn try_climb(&mut self, world: &World, wish: Vec2) -> bool {
        if self.try_climb_direction(world, wish) {
            return true;
        }
        // At a corner, movement slides along the blocking face. Check that same
        // direction for a reachable bank rather than insisting on the tall wall.
        if self.swimming && wish.length_squared() > 0.1 {
            let hit = world.body_trace(self.feet, self.feet + wish.normalize().extend(0.) * 42.);
            let along = wish - hit.normal.truncate() * wish.dot(hit.normal.truncate());
            if hit.fraction < 1.
                && along.length_squared() > 0.1
                && along.normalize().distance(wish.normalize()) > 0.1
            {
                return self.try_climb_direction(world, along);
            }
        }
        false
    }
    fn try_climb_direction(&mut self, world: &World, wish: Vec2) -> bool {
        if wish.length_squared() < 0.1 {
            return false;
        }
        let forward = wish.normalize().extend(0.) * 42.;
        // There must be an actual low obstacle in front, not open water or a distant wall.
        let wall = world.body_trace(self.feet, self.feet + forward);
        if wall.start_solid || wall.fraction >= 1. || wall.normal.z > GROUND_Z {
            return false;
        }
        // Water puts the collider's feet deeper than the old waist-high float.
        // Preserve the same reach above the surface, with the same clearance tests.
        let reach = if self.swimming { 84. } else { 64. };
        let raised = self.feet + Vec3::Z * (reach + 2.);
        let up = world.body_trace(self.feet, raised);
        if up.start_solid || up.fraction < 1. {
            return false;
        }
        let over = world.body_trace(raised, raised + forward);
        if over.start_solid || over.fraction < 1. {
            return false;
        }
        let down = world.body_trace(raised + forward, self.feet + forward);
        if down.start_solid || down.fraction >= 1. || down.normal.z < GROUND_Z {
            return false;
        }
        let end = (raised + forward).lerp(self.feet + forward, down.fraction);
        let height = end.z - self.feet.z;
        if !(18.0..=reach + 0.1).contains(&height)
            || !world.body_clear(end)
            || Immersion::sample(world, end).level >= 2
        {
            return false;
        }
        self.climb_height = height;
        self.climb_direction = wish.normalize();
        self.climb = Some(Climb {
            start: self.feet,
            raised,
            end,
            elapsed: 0.,
        });
        self.climbs += 1;
        self.velocity = Vec3::ZERO;
        self.grounded = false;
        self.swimming = false;
        true
    }
    fn tick_climb(&mut self, world: &World) {
        let climb = self.climb.as_mut().unwrap();
        climb.elapsed += FIXED_DT;
        let t = (climb.elapsed / 0.65).min(1.);
        // Three checked segments, matching the preflight clearance tests.
        let corner = Vec3::new(climb.end.x, climb.end.y, climb.raised.z);
        let next = if t < 0.4 {
            climb.start.lerp(climb.raised, t / 0.4)
        } else if t < 0.8 {
            climb.raised.lerp(corner, (t - 0.4) / 0.4)
        } else {
            corner.lerp(climb.end, (t - 0.8) / 0.2)
        };
        let hit = world.body_trace(self.feet, next);
        // Recheck every tick so a moving door cannot trap Alice mid-climb.
        if hit.start_solid || hit.fraction < 1. {
            self.climb = None;
            self.velocity = Vec3::ZERO;
            return;
        }
        self.feet = next;
        self.immersion = Immersion::sample(world, self.feet);
        if t >= 1. {
            self.climb = None;
            self.grounded = true;
            self.ground_normal = Vec3::Z;
            self.velocity = Vec3::ZERO;
        }
    }
}
fn zero(n: &f32) -> bool {
    *n == 0.
}
fn vec3_z(mut v: Vec3, z: f32) -> Vec3 {
    v.z = z;
    v
}

fn slide(world: &World, mut position: Vec3, mut velocity: Vec3, dt: f32) -> (Vec3, Vec3) {
    let mut remaining = dt;
    let mut normals = Vec::new();
    for _ in 0..5 {
        if velocity.length_squared() < 0.0001 {
            break;
        }
        let end = position + velocity * remaining;
        let hit = world.body_trace(position, end);
        if hit.all_solid {
            return (position, Vec3::ZERO);
        }
        position = position.lerp(end, hit.fraction);
        if hit.fraction >= 1. {
            break;
        }
        remaining *= 1. - hit.fraction;
        if !normals.iter().any(|n: &Vec3| n.dot(hit.normal) > 0.9999) {
            normals.push(hit.normal);
        }
        let tangent_stall = hit.fraction <= 0.000001
            && hit.normal.z.abs() < GROUND_Z
            && velocity.dot(hit.normal).abs() < 0.01;
        velocity = constrain(velocity, &normals);
        // A tangent slide at large map coordinates can round back onto the
        // same plane on every sweep. Merely retaining the tangent velocity
        // then pins Alice in mid-air while gravity accumulates indefinitely.
        // Separate by one collision skin, only along the hit normal and only
        // when the entire tiny move and its endpoint are unobstructed.
        if tangent_stall && velocity.length_squared() > 0.0001 {
            let separated = position + hit.normal * crate::collision::SKIN;
            let escape = world.body_trace(position, separated);
            if !escape.all_solid && escape.fraction >= 1. && world.body_clear(separated) {
                position = separated;
            }
        }
    }
    (position, velocity)
}

pub(crate) fn constrain(v: Vec3, normals: &[Vec3]) -> Vec3 {
    let legal = |candidate: Vec3| normals.iter().all(|n| candidate.dot(*n) >= -0.001);
    if legal(v) {
        return v;
    }
    let mut best = Vec3::ZERO;
    let mut cost = v.length_squared();
    for (i, &n) in normals.iter().enumerate() {
        let projected = v - n * v.dot(n).min(0.);
        if legal(projected) && (projected - v).length_squared() < cost {
            best = projected;
            cost = (best - v).length_squared();
        }
        for &other in &normals[i + 1..] {
            let crease = n.cross(other).normalize_or_zero();
            let candidate = crease * v.dot(crease);
            if legal(candidate) && (candidate - v).length_squared() < cost {
                best = candidate;
                cost = (best - v).length_squared();
            }
        }
    }
    best
}

#[derive(Default)]
pub struct FixedClock {
    accumulator: f64,
    jump_pending: bool,
    use_pending: bool,
    pub ticks: u64,
}
impl FixedClock {
    pub fn advance(&mut self, seconds: f64, world: &World, player: &mut Player, input: Controls) {
        self.accumulator += if seconds.is_finite() {
            seconds.clamp(0., 0.25)
        } else {
            0.
        };
        self.jump_pending |= input.jump;
        self.use_pending |= input.use_pressed;
        const STEP: f64 = 1. / 120.;
        while self.accumulator + 1e-10 >= STEP {
            let controls = Controls {
                jump: self.jump_pending,
                use_pressed: self.use_pending,
                ..input
            };
            self.jump_pending = false;
            self.use_pending = false;
            player.tick(world, controls);
            self.accumulator -= STEP;
            self.ticks += 1;
        }
    }
    pub fn pause(&mut self) {
        self.accumulator = 0.;
        self.jump_pending = false;
        self.use_pending = false;
    }
}

/// A deterministic real-data route, intended for skool1. It checks invariants rather
/// than asserting parity with the original game. Other start scripts may need gameplay.
pub fn check_map(map: &crate::bsp::Bsp) -> anyhow::Result<()> {
    use anyhow::{ensure, Context};
    let world = World::from_bsp(map)?;
    let (eye, yaw) = map.spawn();
    let mut player = Player::spawn(&world, eye).context("No clear player body at map spawn")?;
    let forward = Vec2::new(yaw.cos(), yaw.sin());
    let right = Vec2::new(yaw.sin(), -yaw.cos());
    let origin = player.feet;
    let mut maximum_z = f32::NEG_INFINITY;
    let phases = [
        ("settle", 120, Vec2::ZERO, false),
        ("walk", 360, forward, false),
        ("wall", 180, right, false),
        ("jump", 120, Vec2::ZERO, true),
        ("rest", 120, Vec2::ZERO, false),
    ];
    for (name, count, wish, jump) in phases {
        let before = player.feet;
        for tick in 0..count {
            player.tick(
                &world,
                Controls {
                    wish,
                    jump: jump && tick == 0,
                    run: false,
                    ..Default::default()
                },
            );
            ensure!(
                player.feet.is_finite() && player.velocity.is_finite(),
                "Non-finite state in {name}"
            );
            ensure!(
                world.body_clear(player.feet),
                "Body penetrated collision during {name} tick {tick}: {:?}",
                player.feet
            );
            ensure!(
                player.feet.z > world.min.z - 128.,
                "Fell out of world during {name}"
            );
            if jump {
                maximum_z = maximum_z.max(player.feet.z);
            }
        }
        println!(
            "{name}: feet={:?}, moved={:.2}, grounded={}, jumps={}, landings={}",
            player.feet,
            player.feet.distance(before),
            player.grounded,
            player.jumps,
            player.landings
        );
    }
    ensure!(
        (player.feet - origin).truncate().length() > 50.,
        "Player could not make meaningful progress"
    );
    ensure!(
        player.grounded && player.jumps == 1 && player.landings >= 2,
        "Jump/landing route incomplete"
    );
    println!("PASS: 900 fixed ticks, no solid overlap, jump apex {maximum_z:.2}, {} brush hulls + {} patch facets.",world.brush_count,world.patch_count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn walking_and_running_jumps_use_normal_height_without_self_damage() {
        let world = World::fixture(&[(vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.))]);
        for (run, height) in [(false, 100.), (true, 100.)] {
            let mut p = Player::new(Vec3::Z * 0.04);
            p.tick(&world, Controls::default());
            let floor = p.feet.z;
            let mut apex = floor;
            let mut stats = crate::inventory::Stats::default();
            for tick in 0..240 {
                let landings = p.landings;
                p.tick(
                    &world,
                    Controls {
                        jump: tick == 0,
                        run,
                        ..Default::default()
                    },
                );
                apex = apex.max(p.feet.z);
                if p.landings > landings {
                    stats.fall(p.landing_speed);
                }
            }
            assert!((apex - floor - height).abs() < 2.);
            assert_eq!(stats.sanity(), 100.);
            assert!(p.grounded && p.jumps == 1);
        }
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.))])
    }
    #[test]
    fn brisk_walking_speed_and_running_are_stable() {
        let world = floor();
        for (run, fraction, expected) in [(false, 1., 104.), (false, 0.125, 13.), (true, 1., 320.)]
        {
            let mut p = Player::new(Vec3::Z * 0.04);
            let input = Controls {
                wish: Vec2::X * fraction,
                run,
                ..Default::default()
            };
            for _ in 0..120 {
                p.tick(&world, input);
            }
            let start = p.feet;
            for _ in 0..120 {
                p.tick(&world, input);
            }
            assert!((p.velocity.x - expected).abs() < 0.001);
            assert!((p.feet.x - start.x - expected).abs() < 0.01);
            assert!(world.body_clear(p.feet) && p.grounded);
        }
    }
    #[test]
    fn supported_spawn_stands_immediately_but_elevated_spawn_still_falls() {
        let world = floor();
        let mut standing = Player::spawn(&world, Vec3::Z * (EYE_HEIGHT + 0.04)).unwrap();
        assert!(standing.grounded);
        standing.tick(&world, Controls::default());
        assert!(standing.grounded && standing.landings == 0);
        let mut airborne = Player::spawn(&world, Vec3::Z * (EYE_HEIGHT + 16.)).unwrap();
        assert!(!airborne.grounded);
        for _ in 0..120 {
            airborne.tick(&world, Controls::default());
        }
        assert!(airborne.grounded && airborne.landings == 1);
    }
    #[test]
    fn falls_lands_jumps_and_lands_again() {
        let world = floor();
        let mut p = Player::new(vec3(0., 0., 100.));
        for _ in 0..240 {
            p.tick(&world, Controls::default());
        }
        assert!(p.grounded);
        assert!(p.feet.z > 0. && p.feet.z < 0.1);
        let floor = p.feet.z;
        p.tick(
            &world,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        let mut apex = p.feet.z;
        for _ in 0..120 {
            p.tick(&world, Controls::default());
            apex = apex.max(p.feet.z);
            assert!(world.body_clear(p.feet));
        }
        assert!(apex - floor > 98. && apex - floor < 102.);
        assert!(p.grounded);
        assert_eq!(p.jumps, 1);
        assert_eq!(p.landings, 2);
    }
    #[test]
    fn landing_preserves_impact_for_damage_once_then_stops() {
        let world = floor();
        for (height, damaging) in [(45., false), (500., true)] {
            let mut p = Player::new(vec3(0., 0., height));
            let mut stats = crate::inventory::Stats::default();
            for _ in 0..360 {
                let before = p.landings;
                p.tick(&world, Controls::default());
                if p.landings > before {
                    stats.fall(p.landing_speed);
                }
            }
            assert!(p.grounded);
            assert_eq!(p.landings, 1);
            assert_eq!(p.velocity.z, 0.);
            assert_eq!(stats.sanity() < 100., damaging);
            if damaging {
                assert!((880.0..910.0).contains(&p.landing_speed));
                assert!((45.0..55.0).contains(&stats.sanity()));
            }
        }
    }
    #[test]
    fn slides_along_wall_without_penetrating() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
            (vec3(100., -1000., 0.), vec3(120., 1000., 200.)),
        ]);
        let mut p = Player::new(vec3(0., 0., 0.1));
        // Allow the brisk walking speed to reach and slide along the wall.
        for _ in 0..360 {
            p.tick(
                &world,
                Controls {
                    wish: Vec2::ONE,
                    ..Default::default()
                },
            );
            assert!(world.body_clear(p.feet));
        }
        assert!(p.feet.x < 85. && p.feet.x > 84.);
        assert!(p.feet.y > 150.);
    }
    #[test]
    fn climbs_low_step_but_cannot_climb_high_wall() {
        for height in [12., 32.] {
            let world = World::fixture(&[
                (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
                (vec3(50., -100., 0.), vec3(500., 100., height)),
            ]);
            let mut p = Player::new(vec3(0., 0., 0.1));
            // Walk far enough to exercise the same step geometry at 104 units/s.
            for _ in 0..240 {
                p.tick(
                    &world,
                    Controls {
                        wish: Vec2::X,
                        ..Default::default()
                    },
                );
                assert!(world.body_clear(p.feet));
            }
            if height < 18. {
                assert!(p.feet.x > 180.);
                assert!((p.feet.z - height).abs() < 0.1);
            } else {
                assert!(p.feet.x < 35.);
                assert!(p.feet.z < 0.1);
            }
        }
    }
    #[test]
    fn ceiling_stops_jump_and_player_returns_to_floor() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
            (vec3(-100., -100., 80.), vec3(100., 100., 100.)),
        ]);
        let mut p = Player::new(vec3(0., 0., 0.1));
        p.tick(&world, Controls::default());
        p.tick(
            &world,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        for _ in 0..120 {
            p.tick(&world, Controls::default());
            assert!(p.feet.z + 56. < 80.);
            assert!(world.body_clear(p.feet));
        }
        assert!(p.grounded);
    }
    #[test]
    fn rendering_frame_rate_does_not_change_movement() {
        let world = floor();
        let mut positions = Vec::new();
        for rate in [30, 60, 144] {
            let mut clock = FixedClock::default();
            let mut p = Player::new(vec3(0., 0., 0.1));
            for _ in 0..rate * 2 {
                clock.advance(
                    1. / rate as f64,
                    &world,
                    &mut p,
                    Controls {
                        wish: Vec2::X,
                        ..Default::default()
                    },
                );
            }
            assert_eq!(clock.ticks, 240);
            positions.push(p.feet);
        }
        assert!(positions.windows(2).all(|p| (p[0] - p[1]).length() < 0.001));
    }
    #[test]
    fn short_jump_press_survives_a_frame_without_physics_tick() {
        let world = floor();
        let mut p = Player::new(vec3(0., 0., 0.1));
        let mut clock = FixedClock::default();
        clock.advance(
            0.002,
            &world,
            &mut p,
            Controls {
                jump: true,
                ..Default::default()
            },
        );
        clock.advance(0.008, &world, &mut p, Controls::default());
        assert_eq!(p.jumps, 1);
    }
    #[test]
    fn follows_slope_up_and_down_and_can_jump_while_climbing() {
        let world = World::ramp_fixture();
        let mut p = Player::new(vec3(0., 0., 8.));
        for _ in 0..200 {
            p.tick(
                &world,
                Controls {
                    wish: Vec2::X,
                    ..Default::default()
                },
            );
            assert!(p.grounded);
            assert!(world.body_clear(p.feet));
        }
        assert!(p.feet.x > 140. && p.feet.z > 70.);
        p.tick(
            &world,
            Controls {
                wish: Vec2::X,
                jump: true,
                ..Default::default()
            },
        );
        assert_eq!(p.jumps, 1);
        assert!(!p.grounded);
        for _ in 0..160 {
            p.tick(
                &world,
                Controls {
                    wish: -Vec2::X,
                    ..Default::default()
                },
            );
            assert!(world.body_clear(p.feet));
        }
        assert!(p.grounded);
        let before = p.feet;
        for _ in 0..60 {
            p.tick(
                &world,
                Controls {
                    wish: -Vec2::X,
                    ..Default::default()
                },
            );
            assert!(p.grounded);
        }
        assert!(p.feet.z < before.z - 15.);
    }
    #[test]
    fn two_wall_corner_does_not_oscillate_or_leak() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
            (vec3(100., -1000., 0.), vec3(120., 1000., 200.)),
            (vec3(-1000., 100., 0.), vec3(1000., 120., 200.)),
        ]);
        let mut p = Player::new(vec3(0., 0., 0.1));
        for _ in 0..480 {
            p.tick(
                &world,
                Controls {
                    wish: Vec2::ONE,
                    run: true,
                    ..Default::default()
                },
            );
            assert!(world.body_clear(p.feet));
        }
        assert!(p.feet.x > 84. && p.feet.x < 85. && p.feet.y > 84. && p.feet.y < 85.);
    }
}

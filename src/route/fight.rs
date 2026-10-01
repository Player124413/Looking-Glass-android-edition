//! Combat policy for the headless route drivers.
//!
//! The Vorpal Blade left Alice's hand for 3.5 s after every throw once the Blade/Cards rules
//! were implemented (`docs/BLADE_CARDS.md`), so a driver that throws at the nearest thing it can
//! see, at the position that thing occupied at release, no longer kills anything. The routes
//! that relied on that (both schools, both Fortress halves, Beyond the Wall, the Duchess) ran
//! out of Sanity or were shoved off their timing by Boojum waves. This module gives the drivers
//! the choices a player has, without touching any gameplay constant:
//!
//! - only enemies that are already awake are targets, so a throw is not spent on an idle guard;
//! - the throw leads a moving target for the 1,200 u/s flight and its 100 u/s² drop;
//! - a target inside the Blade's melee volume is struck with the primary attack, which has no
//!   recovery, instead of waiting for the throw to return;
//! - [`clear`] stands and fights: it strafes across the line of fire (waves and bolts are aimed
//!   at where Alice is, not where she is going), finishes adjacent guards, and then walks over
//!   their drops, which is where the Sanity for the rest of the route comes from.
use super::{trace, Route};
use crate::{
    collision::World,
    combat::{self, Context, Target},
    encounters::{Enemy, BASE},
    movement::{Controls, Player},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Thrown Blade launch speed and drop (`weapons/projectile.rs`).
pub const THROW_SPEED: f32 = 1200.;
pub const THROW_GRAVITY: f32 = 100.;
/// The farthest enemy a throw is aimed at; the Blade lives for one second.
pub const THROW_RANGE: f32 = 1000.;
/// Alice does not stray farther than this from where a fight began.
const LEASH: f32 = 120.;
/// A `clear` call gives up after this many ticks (90 s at 120 Hz).
const CLEAR_TICKS: usize = 10_800;
/// A fight that has neither moved Alice nor let her try for this long has stalled (2 s).
const STALL_TICKS: usize = 240;
/// Alice has moved if she is this far from where the last stall count began.
const STALL_MOVE: f32 = 8.;
/// Alice is off the ground the route walked once she stands this far below where a fight began.
const FALL_DEPTH: f32 = 24.;
/// A stall is climbed out of this many times in one `clear`; after that the fight runs on as before.
const CLIMBS: usize = 6;
/// A focus that has taken no damage for this many of the ticks it was the focus (20 s) is left to
/// the route's own timing, like a melee-only guard that cannot be reached.
const NO_DAMAGE_TICKS: usize = 2400;

/// Smoothed velocity of every target, measured between consecutive route ticks.
#[derive(Default, Clone)]
pub struct Tracker {
    /// A route's close-quarters approach can retain the Blade between swings.
    pub hold_blade: bool,
    seen: BTreeMap<usize, (Vec3, Vec3)>,
    pub last_aim: Vec3,
    /// The toy Alice is holding for the fight (0 Blade, 1 Cards); switching costs an equip.
    pub weapon: usize,
    /// The route tick of the last hit that landed on each target.
    landed: BTreeMap<usize, usize>,
}
impl Tracker {
    /// A hit landed on `id` at route tick `tick`.
    pub fn landed(&mut self, id: usize, tick: usize) {
        self.landed.insert(id, tick);
    }
    pub fn last_hit(&self, id: usize) -> Option<usize> {
        self.landed.get(&id).copied()
    }
    pub fn observe(&mut self, targets: &[Target], dt: f32) {
        if dt.is_nan() || dt <= 0. {
            return;
        }
        let mut next = BTreeMap::new();
        for t in targets {
            // Segment hit boxes share an actor ID. Track one stable anchor;
            // treating successive bones as movement invents a large velocity.
            if next.contains_key(&t.id) {
                continue;
            }
            let velocity = self.seen.get(&t.id).map_or(Vec3::ZERO, |(center, v)| {
                let measured = (t.center - *center) / dt;
                // A re-placed or teleported actor is not moving at 100 m/s.
                if measured.length() > 1500. {
                    Vec3::ZERO
                } else {
                    *v * 0.85 + measured * 0.15
                }
            });
            next.insert(t.id, (t.center, velocity));
        }
        self.seen = next;
    }
    pub fn velocity(&self, id: usize) -> Vec3 {
        self.seen.get(&id).map_or(Vec3::ZERO, |(_, v)| *v)
    }
}

/// Direction that lands a thrown Blade on a target moving with `velocity`.
pub fn lead(eye: Vec3, center: Vec3, velocity: Vec3) -> Vec3 {
    let mut time = eye.distance(center) / THROW_SPEED;
    for _ in 0..4 {
        time = eye.distance(center + velocity * time) / THROW_SPEED;
    }
    let point = center + velocity * time + Vec3::Z * (0.5 * THROW_GRAVITY * time * time);
    (point - eye).normalize_or_zero()
}

/// Cards seek their target, live 1.5 s at 700 u/s, and cost 3 Will each.
pub const CARD_RANGE: f32 = 900.;
pub const CARD_COST: f32 = 3.;

/// What to do this tick: the toy (0 Blade, 1 Cards), which of its two attacks, and where to aim.
pub struct Plan {
    pub target: Option<usize>,
    pub aim: Vec3,
    pub weapon: usize,
    pub alternate: bool,
}
/// The Blade's melee swing when a foe is inside its volume (no recovery, no Will); otherwise
/// Cards while there is Will to pay for them (they home in and cannot miss a flying Boojum);
/// otherwise a led throw at the nearest foe.
pub fn aim_point(ctx: &Context<'_>, eye: Vec3, t: &Target) -> Option<Vec3> {
    let mut candidates = vec![t.center];
    if t.half.z > 100. {
        candidates.push(vec3(
            t.center.x,
            t.center.y,
            eye.z
                .clamp(t.center.z - t.half.z + 1., t.center.z + t.half.z - 1.),
        ));
        candidates.push(t.center + Vec3::Z * (t.half.z - 16.));
    }
    // A centre ray can clear a parapet while the actual 8-unit projectile
    // clips it. Prefer an exposed upper-body point before spending a throw.
    if t.half.z <= 100. {
        candidates.push(t.center + Vec3::Z * (t.half.z - 8.).max(0.));
        if let Some(point) = candidates.iter().copied().find(|&point|
            combat::contact(ctx, eye, point, 8.).is_some_and(|(id, _)| id == t.id)) {
            return Some(point);
        }
    }
    candidates
        .into_iter()
        .find(|&point| combat::contact(ctx, eye, point, 1.).is_some_and(|(id, _)| id == t.id))
}
pub fn plan(ctx: &Context<'_>, eye: Vec3, foes: &[Target], tracker: &Tracker, cards: bool) -> Plan {
    let mut throw: Option<(f32, usize, Vec3)> = None;
    let mut melee: Option<(f32, usize, Vec3)> = None;
    let mut shot: Option<(f32, usize, Vec3)> = None;
    for t in foes {
        // A tall boss can have its origin below Alice's platform. Aim at a
        // visible point in its real hit box; tracing to the centre alone can
        // incorrectly treat the whole body as hidden behind the platform rim.
        let visible = combat::contact(ctx, eye, t.center, 1.).is_some_and(|(id, _)| id == t.id);
        let Some(point) = aim_point(ctx, eye, t) else {
            continue;
        };
        let distance = point.distance(eye);
        let direct = (point - eye).normalize_or_zero();
        if crate::weapons::blade_melee(ctx, eye, direct)
            .iter()
            .any(|h| h.id == t.id)
        {
            if melee.is_none_or(|(d, ..)| distance < d) {
                melee = Some((distance, t.id, direct));
            }
        } else {
            if distance <= THROW_RANGE && throw.is_none_or(|(d, ..)| distance < d) {
                throw = Some((distance, t.id, lead(eye, point, tracker.velocity(t.id))));
            }
            if visible && distance <= CARD_RANGE && shot.is_none_or(|(d, ..)| distance < d) {
                shot = Some((distance, t.id, direct));
            }
        }
    }
    let blade = |id, aim, alternate| Plan {
        target: Some(id),
        aim,
        weapon: 0,
        alternate,
    };
    match (melee, shot.filter(|_| cards && !tracker.hold_blade), throw.filter(|_| !tracker.hold_blade)) {
        (Some((_, id, aim)), ..) => blade(id, aim, false),
        (None, Some((_, id, aim)), _) => Plan {
            target: Some(id),
            aim,
            weapon: 1,
            alternate: false,
        },
        (None, None, Some((_, id, aim))) => blade(id, aim, true),
        _ => Plan {
            target: None,
            aim: tracker.last_aim,
            weapon: 0,
            alternate: false,
        },
    }
}

/// What [`clear`] needs from a route driver: the shared [`Route`] and the private school-two one.
pub trait Arena {
    fn player(&self) -> &Player;
    fn world(&self) -> &World;
    fn ticks(&self) -> usize;
    /// Advance one production tick with this input; tactics are switched on first.
    fn step(&mut self, input: Controls) -> Result<()>;
    /// Awake enemies within `radius` of Alice's eye and in her line of sight.
    fn threats(&self, radius: f32, ignored: &BTreeSet<usize>) -> Vec<Target>;
    /// A guard that has no ranged attack and has to be walked up to.
    fn melee_only(&self, id: usize) -> bool;
    /// The route has taken its exit and will not advance again (`Route::stop_at_exit`).
    fn stopped(&self) -> bool {
        false
    }
    /// Loose essence: identity, position and the pickup key that marks it collected.
    fn drops(&self) -> Vec<(usize, Vec3, String)>;
    fn collected(&self, key: &str) -> bool;
    fn navigate(&mut self, goal: Vec3) -> Result<()>;
    /// The tick of the last hit that landed on this target, if any has.
    fn last_hit(&self, id: usize) -> Option<usize>;
}

/// Notices a fight that has stopped going anywhere: Alice has not moved, or has not tried to, for
/// [`STALL_TICKS`]. A knock-off the walkway leaves her on a ledge where every heading is unsafe,
/// so the fight would otherwise stand still until [`CLEAR_TICKS`] runs out.
struct Watchdog {
    anchor: Vec3,
    since: usize,
    idle: usize,
}
impl Watchdog {
    fn new(feet: Vec3, tick: usize) -> Self {
        Self {
            anchor: feet,
            since: tick,
            idle: 0,
        }
    }
    /// Count this tick; true once Alice has neither moved nor wished to move for [`STALL_TICKS`].
    fn stalled(&mut self, feet: Vec3, tick: usize, wish: Vec2) -> bool {
        self.idle = if wish == Vec2::ZERO { self.idle + 1 } else { 0 };
        if feet.distance(self.anchor) >= STALL_MOVE {
            self.anchor = feet;
            self.since = tick;
        }
        self.idle >= STALL_TICKS || tick.saturating_sub(self.since) >= STALL_TICKS
    }
}
/// Is Alice off the ground the route walked: well below where the fight began, or beyond the
/// leash that the fight itself keeps to?
fn off_route(feet: Vec3, home: Vec3) -> bool {
    home.z - feet.z > FALL_DEPTH || (feet - home).truncate().length() > LEASH
}

/// How long each focus has been fought without taking damage.
#[derive(Default)]
struct Starved {
    seen: BTreeMap<usize, (Option<usize>, usize)>,
}
impl Starved {
    /// One tick with `id` as the focus and `last_hit` its latest landed hit; the ticks it has gone
    /// without damage as the focus.
    fn tick(&mut self, id: usize, last_hit: Option<usize>) -> usize {
        let (seen, ticks) = self.seen.entry(id).or_insert((last_hit, 0));
        if *seen != last_hit {
            *seen = last_hit;
            *ticks = 0;
        }
        *ticks += 1;
        *ticks
    }
}

/// Would Alice keep her footing and stay clear of geometry if she held `wish` for a moment and
/// then let go? Running momentum carries her past a ledge that a shorter look would miss.
fn safe_to_move(world: &World, player: &Player, wish: Vec2) -> bool {
    let mut p: Player = player.clone();
    for t in 0..72 {
        p.tick(
            world,
            Controls {
                wish: if t < 24 { wish } else { Vec2::ZERO },
                run: true,
                ..Default::default()
            },
        );
        if !world.body_clear(p.feet) || p.feet.z < player.feet.z - 12. {
            return false;
        }
    }
    true
}

/// Stand and fight the awake enemies around Alice, then walk over what they dropped.
///
/// Alice circles the nearest enemy across its line of fire, walks up to guards that only have a
/// melee attack, and otherwise lets the route's tick choose between the Blade's melee swing and a
/// led throw. It returns once nothing awake is left in view within `radius`, with Alice back
/// where the fight began so that the route's next straight leg starts from the spot it was
/// written for.
pub fn clear<A: Arena>(a: &mut A, radius: f32) -> Result<()> {
    let home = a.player().feet;
    let start = a.ticks();
    trace(|| format!("clear begins t{start} at {home:?} radius {radius}"));
    let mut ignored = BTreeSet::new();
    let (mut sign, mut since_flip) = (1f32, 0usize);
    let mut watchdog = Watchdog::new(home, start);
    let (mut climbs, mut starved) = (0, Starved::default());
    loop {
        if a.stopped() {
            return Ok(());
        }
        let foes = a.threats(radius, &ignored);
        let eye = a.player().eye();
        let Some(focus) = foes.iter().min_by(|x, y| {
            x.center
                .distance_squared(eye)
                .total_cmp(&y.center.distance_squared(eye))
        }) else {
            break;
        };
        ensure!(
            a.ticks() - start < CLEAR_TICKS,
            "Enemies near {:?} were not defeated in {CLEAR_TICKS} ticks",
            a.player().feet
        );
        let offset = (focus.center - eye).truncate();
        let distance = offset.length();
        let toward = offset.normalize_or_zero();
        let across = vec2(-toward.y, toward.x) * sign;
        let melee_only = a.melee_only(focus.id);
        let mut wish = if melee_only {
            if distance > 60. {
                toward
            } else {
                Vec2::ZERO
            }
        } else {
            // Keep about the range at which Boojums hover and Diamonds shoot.
            let radial = if distance > 300. {
                0.6
            } else if distance < 120. {
                -0.5
            } else {
                0.
            };
            (across * 0.9 + toward * radial).normalize_or_zero()
        };
        since_flip += 1;
        if since_flip % 96 == 0 {
            sign = -sign;
        }
        // Stay near ground the route already walked on: platforms move and end.
        let away = (a.player().feet - home).truncate();
        if away.length() > LEASH {
            wish = (wish - away.normalize_or_zero() * 1.5).normalize_or_zero();
            sign = if wish.perp_dot(toward) > 0. { -1. } else { 1. };
        }
        if wish != Vec2::ZERO && !safe_to_move(a.world(), a.player(), wish) {
            sign = -sign;
            wish = if safe_to_move(a.world(), a.player(), -wish) {
                -wish
            } else {
                Vec2::ZERO
            };
        }
        // A hit can knock Alice off the walkway onto a ledge where every heading is unsafe. Stood
        // there, she would only wait for the clock: climb back to where the fight began instead.
        if watchdog.stalled(a.player().feet, a.ticks(), wish)
            && climbs < CLIMBS
            && off_route(a.player().feet, home)
        {
            climbs += 1;
            trace(|| {
                format!(
                    "clear stalled t{} at {:?}: climbing back to {home:?}",
                    a.ticks(),
                    a.player().feet
                )
            });
            a.navigate(home)?;
            watchdog = Watchdog::new(a.player().feet, a.ticks());
            continue;
        }
        let id = focus.id;
        a.step(Controls {
            wish,
            run: true,
            ..Default::default()
        })?;
        // A guard that cannot be approached, or that nothing Alice does can hurt, is left to
        // throws and to the route's own timing.
        let dry = starved.tick(id, a.last_hit(id));
        if (melee_only && a.ticks() - start > 2400) || dry > NO_DAMAGE_TICKS {
            trace(|| {
                format!(
                    "clear leaves {id} at t{}: {dry} ticks without damage",
                    a.ticks()
                )
            });
            ignored.insert(id);
        }
    }
    collect_drops(a, radius)?;
    if a.ticks() > start {
        return_to(a, home)?;
    }
    Ok(())
}

/// Walk onto essence dropped by defeated enemies within `radius`, while it still lasts.
fn collect_drops<A: Arena>(a: &mut A, radius: f32) -> Result<()> {
    let mut given_up = BTreeSet::new();
    for _ in 0..8 {
        if a.stopped() {
            return Ok(());
        }
        let feet = a.player().feet;
        let Some((id, origin, key)) = a
            .drops()
            .into_iter()
            .filter(|(id, origin, key)| {
                !given_up.contains(id)
                    && !a.collected(key)
                    && (*origin - feet).truncate().length() <= radius
                    && (origin.z - feet.z).abs() < 90.
            })
            .min_by(|x, y| {
                x.1.distance_squared(feet)
                    .total_cmp(&y.1.distance_squared(feet))
            })
        else {
            break;
        };
        for _ in 0..480 {
            if a.collected(&key) || a.stopped() {
                break;
            }
            let toward = (origin - a.player().feet).truncate().normalize_or_zero();
            if !safe_to_move(a.world(), a.player(), toward) {
                break;
            }
            a.step(Controls {
                wish: toward,
                run: true,
                ..Default::default()
            })?;
        }
        if !a.collected(&key) {
            given_up.insert(id);
        }
    }
    Ok(())
}

/// Go back to where the fight began: walk directly while that is safe, otherwise plan the way.
fn return_to<A: Arena>(a: &mut A, home: Vec3) -> Result<()> {
    let mut still = 0;
    for _ in 0..720 {
        if a.stopped() {
            return Ok(());
        }
        let offset = (home - a.player().feet).truncate();
        if offset.length() < 16. && (home.z - a.player().feet.z).abs() < 16. {
            return Ok(());
        }
        let before = a.player().feet;
        let wish = offset.normalize_or_zero();
        if !safe_to_move(a.world(), a.player(), wish) {
            break;
        }
        a.step(Controls {
            wish,
            run: true,
            ..Default::default()
        })?;
        still = if a.player().feet.distance(before) < 0.2 {
            still + 1
        } else {
            0
        };
        if still > 30 {
            break;
        }
    }
    if (home - a.player().feet).length() > 24. && !a.stopped() {
        a.navigate(home)?;
    }
    Ok(())
}

impl Route {
    /// Enemies that are awake and able to hurt Alice now. Idle guards are left alone.
    pub(super) fn engaged(&self, targets: &[Target]) -> Vec<Target> {
        targets
            .iter()
            .copied()
            .filter(|t| {
                // Explicit puzzle targets (for example the Staff wall) are
                // shootable even though the owner correctly marks them non-hostile.
                if self.aim_at == Some(t.id) { return true; }
                if t.id == crate::duchess::ID {
                    return true;
                }
                if let Some(s) = crate::level::hit_owner(&self.interactions.levels, t.id) {
                    return self.interactions.levels[s].ctl.hostile_target(t.id);
                }
                if let Some(awake) = self.engaged_foe(t.id) {
                    return awake;
                }
                let Some(actor) = self
                    .interactions
                    .encounters
                    .as_ref()
                    .and_then(|e| e.actors.get(t.id.checked_sub(BASE)?))
                else {
                    return false;
                };
                actor.active
                    && actor.enabled
                    && match &actor.enemy {
                        Enemy::Guard(g) => {
                            !matches!(g.state, combat::State::Idle | combat::State::Dead)
                        }
                        Enemy::Boojum(b) => b.health > 0.,
                        Enemy::Ladybug(b) => b.health > 0.,
                    }
            })
            .collect()
    }
    /// Stand and fight everything awake within `radius`, then collect what it dropped.
    pub fn clear(&mut self, radius: f32) -> Result<()> {
        self.tactics = true;
        clear(self, radius)
    }
}
impl Arena for Route {
    fn player(&self) -> &Player {
        &self.player
    }
    fn world(&self) -> &World {
        &self.world
    }
    fn ticks(&self) -> usize {
        self.ticks
    }
    fn step(&mut self, input: Controls) -> Result<()> {
        self.tick(input)
    }
    fn stopped(&self) -> bool {
        self.exited()
    }
    fn threats(&self, radius: f32, ignored: &BTreeSet<usize>) -> Vec<Target> {
        let mut targets = self
            .interactions
            .encounters
            .as_ref()
            .map_or(Vec::new(), |s| s.targets());
        targets.extend(self.interactions.duchess.as_ref().and_then(|d| d.target()));
        targets.extend(self.interactions.levels_targets());
        targets.extend(self.school2_targets());
        targets.extend(self.guard_targets());
        if let Some(n) = &self.native_cast { targets.extend(n.targets()); }
        let ctx = Context {
            world: &self.world,
            targets: &targets,
        };
        let eye = self.player.eye();
        self.engaged(&targets)
            .into_iter()
            .filter(|t| !ignored.contains(&t.id) && t.center.distance(eye) <= radius)
            .filter(|t| combat::contact(&ctx, eye, t.center, 1.).is_some_and(|(id, _)| id == t.id))
            .collect()
    }
    fn melee_only(&self, id: usize) -> bool {
        if let Some(s) = crate::level::hit_owner(&self.interactions.levels, id) {
            return self.interactions.levels[s].ctl.melee_target(id);
        }
        if let Some(melee) = self.melee_foe(id) {
            return melee;
        }
        self.interactions
            .encounters
            .as_ref()
            .and_then(|e| e.actors.get(id.checked_sub(BASE)?))
            .is_some_and(|a| matches!(&a.enemy, Enemy::Guard(g) if !g.ranged))
    }
    fn drops(&self) -> Vec<(usize, Vec3, String)> {
        self.stats
            .loot
            .get(&self.visit)
            .into_iter()
            .flat_map(|l| l.drops.iter())
            .map(|d| (d.id, d.origin, format!("drop:{}:{}", self.visit, d.id)))
            .collect()
    }
    fn collected(&self, key: &str) -> bool {
        self.stats.collected.contains(key)
    }
    fn navigate(&mut self, goal: Vec3) -> Result<()> {
        Route::navigate(self, goal)
    }
    fn last_hit(&self, id: usize) -> Option<usize> {
        self.fight.last_hit(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapons::Projectile;

    fn body(id: usize, center: Vec3) -> Target {
        Target {
            id,
            center,
            half: vec3(24., 24., 40.),
        }
    }
    /// Fly a thrown Blade at a target that keeps moving, with the real projectile code.
    fn throw_hits(aim: Vec3, start: Vec3, velocity: Vec3) -> bool {
        let world = World::fixture(&[]);
        let mut blade = Projectile::thrown_blade(Vec3::ZERO, aim);
        let mut center = start;
        for _ in 0..120 {
            center += velocity / 120.;
            let targets = [body(7, center)];
            let ctx = Context {
                world: &world,
                targets: &targets,
            };
            if blade.contact_step(1. / 120., &ctx).0.is_some() {
                return true;
            }
        }
        false
    }
    #[test]
    fn a_led_throw_lands_on_a_moving_target_that_a_direct_throw_misses() {
        let start = vec3(700., 0., 0.);
        let velocity = vec3(0., 220., 0.);
        let direct = (start - Vec3::ZERO).normalize();
        assert!(!throw_hits(direct, start, velocity));
        assert!(throw_hits(
            lead(Vec3::ZERO, start, velocity),
            start,
            velocity
        ));
        // The same constants keep a stationary target at the far end of the range in reach.
        let far = vec3(THROW_RANGE, 0., 0.);
        assert!(throw_hits(
            lead(Vec3::ZERO, far, Vec3::ZERO),
            far,
            Vec3::ZERO
        ));
    }
    #[test]
    fn tracker_follows_a_target_and_ignores_a_teleport() {
        let mut t = Tracker::default();
        for i in 0..240 {
            t.observe(&[body(3, vec3(100. + i as f32 * 1.5, 0., 0.))], 1. / 120.);
        }
        assert!((t.velocity(3).x - 180.).abs() < 5.);
        t.observe(&[body(3, vec3(9000., 0., 0.))], 1. / 120.);
        assert_eq!(t.velocity(3), Vec3::ZERO);
        assert_eq!(t.velocity(99), Vec3::ZERO);
    }
    #[test]
    fn plan_swings_in_reach_shoots_cards_with_will_and_throws_without() {
        let world = World::fixture(&[]);
        let tracker = Tracker::default();
        let eye = Vec3::ZERO;
        let near = body(1, vec3(60., 0., 0.));
        let far = body(2, vec3(500., 0., 0.));
        let targets = [near, far];
        let ctx = Context {
            world: &world,
            targets: &targets,
        };
        // Inside the Blade's volume the primary swing wins, with or without Will.
        for cards in [true, false] {
            let plan = plan(&ctx, eye, &targets, &tracker, cards);
            assert_eq!(
                (plan.target, plan.weapon, plan.alternate),
                (Some(1), 0, false)
            );
        }
        let ctx = Context {
            world: &world,
            targets: &targets[1..],
        };
        let paid = plan(&ctx, eye, &targets[1..], &tracker, true);
        assert_eq!(
            (paid.target, paid.weapon, paid.alternate),
            (Some(2), 1, false)
        );
        let unpaid = plan(&ctx, eye, &targets[1..], &tracker, false);
        assert_eq!(
            (unpaid.target, unpaid.weapon, unpaid.alternate),
            (Some(2), 0, true)
        );
        // Nothing awake and visible, nothing to do; the last aim is kept for a release in flight.
        let idle = plan(&ctx, eye, &[], &tracker, true);
        assert_eq!(idle.target, None);
    }
    /// A scripted fight for [`clear`]: one foe, Alice on a slab, and a log of what she was told.
    struct Stage {
        world: World,
        player: Player,
        ticks: usize,
        foe: Target,
        melee: bool,
        /// Alice does not move by herself; the fight knocks her 40 units down on its first tick.
        knocked_down: bool,
        /// The foe is beaten as soon as Alice has climbed back to where the fight began.
        dies_on_navigate: bool,
        navigated: Vec<Vec3>,
        hits: Vec<usize>,
    }
    impl Stage {
        fn new(foe: Vec3, melee: bool, knocked_down: bool) -> Self {
            Self {
                world: World::fixture(&[(vec3(-2000., -2000., -32.), vec3(2000., 2000., 0.))]),
                player: Player::new(vec3(0., 0., 0.03)),
                ticks: 0,
                foe: body(9, foe),
                melee,
                knocked_down,
                dies_on_navigate: knocked_down,
                navigated: Vec::new(),
                hits: Vec::new(),
            }
        }
    }
    impl Arena for Stage {
        fn player(&self) -> &Player {
            &self.player
        }
        fn world(&self) -> &World {
            &self.world
        }
        fn ticks(&self) -> usize {
            self.ticks
        }
        fn step(&mut self, input: Controls) -> Result<()> {
            if self.knocked_down {
                if self.ticks == 0 {
                    self.player.feet.z -= 40.;
                }
            } else {
                self.player.tick(&self.world, input);
            }
            self.ticks += 1;
            Ok(())
        }
        fn threats(&self, _radius: f32, ignored: &BTreeSet<usize>) -> Vec<Target> {
            let beaten = self.dies_on_navigate && !self.navigated.is_empty();
            if ignored.contains(&self.foe.id) || beaten {
                Vec::new()
            } else {
                vec![self.foe]
            }
        }
        fn melee_only(&self, _id: usize) -> bool {
            self.melee
        }
        fn drops(&self) -> Vec<(usize, Vec3, String)> {
            Vec::new()
        }
        fn collected(&self, _key: &str) -> bool {
            false
        }
        fn navigate(&mut self, goal: Vec3) -> Result<()> {
            self.navigated.push(goal);
            self.player.feet = goal;
            self.ticks += 120;
            Ok(())
        }
        fn last_hit(&self, id: usize) -> Option<usize> {
            self.hits.iter().rev().find(|_| id == self.foe.id).copied()
        }
    }
    #[test]
    fn a_fight_that_leaves_alice_standing_below_where_it_began_climbs_back() {
        // A guard within reach that Alice waits on (wish zero), and a knock 40 units down.
        let mut stage = Stage::new(vec3(30., 0., 40.), true, true);
        clear(&mut stage, 700.).unwrap();
        assert_eq!(stage.navigated, [vec3(0., 0., 0.03)]);
        // The climb comes after the stall count, long before the 2,400-tick melee escape.
        assert!(
            (STALL_TICKS - 1..STALL_TICKS + 30).contains(&stage.ticks.saturating_sub(120)),
            "climbed at tick {}",
            stage.ticks
        );
    }
    #[test]
    fn a_stall_on_the_walked_ground_is_not_climbed_out_of() {
        // The same guard and the same wait, but nothing knocked Alice down: no climb, and the
        // unreachable-guard escape ends the fight as before.
        let mut stage = Stage::new(vec3(30., 0., 40.), true, false);
        clear(&mut stage, 700.).unwrap();
        assert!(stage.navigated.is_empty());
        assert!(
            stage.ticks > 2400 && stage.ticks < CLEAR_TICKS,
            "{}",
            stage.ticks
        );
    }
    #[test]
    fn a_foe_that_takes_no_damage_is_left_after_twenty_seconds_of_trying() {
        let mut stage = Stage::new(vec3(500., 0., 40.), false, false);
        clear(&mut stage, 700.).unwrap();
        assert!(stage.navigated.is_empty());
        assert!(
            stage.ticks > NO_DAMAGE_TICKS && stage.ticks < NO_DAMAGE_TICKS + 1200,
            "left at tick {}",
            stage.ticks
        );
    }
    #[test]
    fn a_landing_hit_restarts_the_dry_count() {
        let mut starved = Starved::default();
        for _ in 0..NO_DAMAGE_TICKS - 1 {
            starved.tick(4, None);
        }
        // A hit lands just before the limit: the count starts over and another focus is separate.
        assert_eq!(starved.tick(4, Some(700)), 1);
        assert_eq!(starved.tick(5, None), 1);
        for _ in 0..NO_DAMAGE_TICKS {
            starved.tick(4, Some(700));
        }
        assert!(starved.tick(4, Some(700)) > NO_DAMAGE_TICKS);
        // Ticks spent on another focus do not count against this one.
        assert_eq!(starved.tick(5, None), 2);
    }
    #[test]
    fn the_watchdog_counts_stillness_and_forgives_a_move() {
        let mut dog = Watchdog::new(Vec3::ZERO, 0);
        let go = vec2(1., 0.);
        // Wishing but pinned in place: 240 ticks without moving 8 units.
        assert!((0..STALL_TICKS).all(|t| !dog.stalled(Vec3::ZERO, t, go)));
        assert!(dog.stalled(Vec3::ZERO, STALL_TICKS, go));
        // Moving 8 units restarts the count.
        assert!(!dog.stalled(vec3(8., 0., 0.), STALL_TICKS + 1, go));
        // Not wishing at all is stillness too, whatever the feet do.
        let mut dog = Watchdog::new(Vec3::ZERO, 0);
        assert!((0..STALL_TICKS - 1).all(|t| !dog.stalled(vec3(t as f32, 0., 0.), t, Vec2::ZERO)));
        assert!(dog.stalled(vec3(400., 0., 0.), STALL_TICKS - 1, Vec2::ZERO));
        // Below the walkway, or out past the leash, is off the ground the route walked.
        let home = vec3(10., 20., -256.);
        assert!(off_route(home + vec3(0., 0., -25.), home));
        assert!(!off_route(home + vec3(0., 0., -20.), home));
        assert!(off_route(home + vec3(LEASH + 1., 0., 0.), home));
        assert!(!off_route(home + vec3(LEASH - 1., 0., 0.), home));
    }
    #[test]
    fn the_tracker_remembers_the_last_hit_that_landed_on_each_target() {
        let mut tracker = Tracker::default();
        assert_eq!(tracker.last_hit(3), None);
        tracker.landed(3, 40);
        tracker.landed(3, 90);
        tracker.landed(8, 5);
        assert_eq!(
            (tracker.last_hit(3), tracker.last_hit(8)),
            (Some(90), Some(5))
        );
    }
    #[test]
    fn a_wall_hides_a_foe_and_a_ledge_is_not_safe_ground() {
        let wall = World::fixture(&[(vec3(100., -200., -200.), vec3(120., 200., 200.))]);
        let targets = [body(4, vec3(300., 0., 0.))];
        let ctx = Context {
            world: &wall,
            targets: &targets,
        };
        assert_eq!(
            plan(&ctx, Vec3::ZERO, &targets, &Tracker::default(), true).target,
            None
        );
        // Alice on a slab that ends: running toward the edge is unsafe, away from it is fine.
        let slab = World::fixture(&[(vec3(-400., -400., -32.), vec3(400., 400., 0.))]);
        let alice = Player::new(vec3(385., 0., 0.03));
        assert!(!safe_to_move(&slab, &alice, vec2(1., 0.)));
        assert!(safe_to_move(&slab, &alice, vec2(-1., 0.)));
    }
    #[test]
    fn raised_platform_does_not_hide_a_tall_boss_and_segments_do_not_invent_velocity() {
        let world = World::fixture(&[(vec3(-100., -200., -100.), vec3(160., 200., 100.))]);
        let targets = [Target {
            id: 1,
            center: vec3(500., 0., 0.),
            half: vec3(48., 48., 700.),
        }];
        let choice = plan(
            &Context {
                world: &world,
                targets: &targets,
            },
            vec3(0., 0., 132.),
            &targets,
            &Tracker::default(),
            true,
        );
        assert_eq!(choice.target, Some(1));
        assert_eq!(choice.weapon, 0);
        let segments = [body(7, vec3(200., 0., 0.)), body(7, vec3(400., 0., 0.))];
        let mut tracker = Tracker::default();
        tracker.observe(&segments, 0.1);
        tracker.observe(&segments, 0.1);
        assert_eq!(tracker.velocity(7), Vec3::ZERO);
    }
    #[test]
    fn projectile_clearance_aims_over_a_low_parapet() {
        let world = World::fixture(&[(vec3(160., -80., 0.), vec3(170., 80., 46.))]);
        let targets = [Target { id: 1, center: vec3(200., 0., 40.), half: vec3(20.,20.,32.) }];
        let ctx = Context { world: &world, targets: &targets };
        let eye = vec3(0., 0., 88.);
        assert_eq!(combat::contact(&ctx, eye, targets[0].center, 8.), None);
        let point = aim_point(&ctx, eye, &targets[0]).unwrap();
        assert!(point.z > targets[0].center.z);
        let mut shot = crate::weapons::Projectile::thrown_blade(eye, lead(eye,point,Vec3::ZERO));
        let mut hit = None;
        for _ in 0..120 { hit = hit.or(shot.contact_step(1./120., &ctx).0); }
        assert_eq!(hit.unwrap().id, 1);
    }

}

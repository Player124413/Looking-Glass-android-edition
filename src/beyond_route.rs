//! Uses production movement, contacts and combat; no direct player warps.
//!
//! The driver was recorded from a full Sanity and Will bar and with fixed waits, and neither
//! survives the campaign chain: Alice arrives with what the visits before took from her, and a
//! fight that lasts a little longer or a little shorter moves every fixed wait against the
//! machinery (the rolling walkway, the arches, the shuffled doors). So the timing-critical
//! parts read the machinery's own clocks (`Beyond::long_phase`, `arch_time`, `sliding`,
//! `last_ready`) and wait for the phase they need, and the fights are resource-aware: the
//! route stands its ground where the floor is wide, fights whatever is awake there with the
//! toy it can pay for (Cards while there is Will, the Blade otherwise), collects what the
//! enemies drop (a drop restores Sanity and Will alike) and takes the essence that lies off
//! the way, so that the resources it spends are the resources it earned.
use crate::{
    assets::Assets,
    beyond::{Beyond, LONG_CYCLE},
    encounters::{Enemy, BASE},
    movement::{Controls, FIXED_DT},
    route::{fight::THROW_RANGE, Route},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;

/// A diagnostic line for `LOOKING_GLASS_ROUTE_TRACE=1` runs: where the route stands, what it has
/// left, the machinery's clocks and the enemies that are awake.
fn mark(r: &Route, label: &str) {
    crate::route::trace(|| {
        let b = beyond(r);
        let foes: Vec<String> = foes(r)
            .iter()
            .map(|f| {
                format!(
                    "{}:{:.0}@{:.0},{:.0},{:.0}",
                    f.name, f.health, f.feet.x, f.feet.y, f.feet.z
                )
            })
            .collect();
        let drops: Vec<String> = r
            .stats
            .loot
            .iter()
            .filter(|(visit, _)| visit.starts_with("fortress2"))
            .flat_map(|(_, l)| l.drops.iter())
            .map(|d| {
                format!(
                    "{}@{:.0},{:.0},{:.0}",
                    d.id, d.origin.x, d.origin.y, d.origin.z
                )
            })
            .collect();
        format!(
            "MARK t{} {label} at {:?} sanity {:.1} will {:.1} age {:.2} long {:?} arch {:.2} walkway {} foes [{}] drops [{}]",
            r.ticks,
            r.player.feet,
            r.stats.sanity(),
            r.stats.will(),
            b.state.age,
            b.long_phase(),
            b.arch_time(),
            b.state.walkway,
            foes.join(" "),
            drops.join(" ")
        )
    });
}
fn beyond(r: &Route) -> &Beyond {
    r.interactions
        .beyond
        .as_ref()
        .expect("Beyond the Wall machinery")
}

/// An enemy that is placed, enabled and alive.
struct Foe {
    /// The target id the weapons aim at (`Route::aim_at`).
    id: usize,
    name: String,
    /// Boojums fly to Alice; the guards stay where they stand or walk.
    flies: bool,
    /// A guard that shoots from where it stands (the Diamonds).
    shoots: bool,
    health: f32,
    feet: Vec3,
}
fn foes(r: &Route) -> Vec<Foe> {
    let Some(e) = r.interactions.encounters.as_ref() else {
        return Vec::new();
    };
    e.actors
        .iter()
        .enumerate()
        .filter(|(_, a)| a.active && a.enabled)
        .filter_map(|(i, a)| {
            let (flies, shoots, health, feet, tag) = match &a.enemy {
                Enemy::Guard(g) => (
                    false,
                    g.ranged,
                    g.health,
                    g.feet,
                    if g.ranged { 'D' } else { 'C' },
                ),
                Enemy::Boojum(b) => (true, false, b.health, b.feet, 'B'),
                Enemy::Ladybug(_) => return None,
            };
            (health > 0.).then(|| Foe {
                id: BASE + i,
                name: format!("{tag}{i}{}", if a.name.is_empty() { "" } else { ":" }) + &a.name,
                flies,
                shoots,
                health,
                feet,
            })
        })
        .collect()
}

/// Tick with no input until `ready` holds, for at most `seconds`.
fn wait_until(
    r: &mut Route,
    what: &str,
    seconds: f32,
    ready: impl Fn(&Route) -> bool,
) -> Result<()> {
    for _ in 0..(seconds / FIXED_DT).ceil() as usize {
        if r.exited() || ready(r) {
            return Ok(());
        }
        r.tick(Controls::default())?;
    }
    ensure!(
        r.exited() || ready(r),
        "Gave up waiting for {what} at {:?}",
        r.player.feet
    );
    Ok(())
}

/// Fight from where Alice stands. `Route::clear` deals with whatever is awake and in view within
/// `radius` (it strafes across the line of fire, swings the Blade in reach, shoots Cards while
/// there is Will to pay for them and throws the Blade otherwise) and then walks over what the
/// enemies dropped within `radius`. A Boojum that has come within `lure` but is not in view yet
/// is given up to `patience` seconds to arrive; nothing waits for an enemy that is not coming.
fn secure(r: &mut Route, what: &str, radius: f32, lure: f32, patience: f32) -> Result<()> {
    let mut waited = 0.;
    let portals = r.teleports;
    loop {
        if waited == 0. {
            mark(r, &format!("securing {what}"));
        }
        let fought = r.clear(radius);
        if fought.is_err() {
            mark(r, &format!("clear failed while securing {what}"));
        }
        fought?;
        ensure!(
            r.teleports == portals,
            "Alice took a portal while securing {what}: she fell, or chased a drop into it"
        );
        let coming = foes(r)
            .iter()
            .filter(|f| f.flies && f.feet.distance(r.player.feet) < lure)
            .count();
        if r.exited() || coming == 0 || waited >= patience {
            // Let the last shove die down before the next move.
            for _ in 0..120 {
                if r.player.grounded && r.player.velocity.truncate().length() < 5. {
                    break;
                }
                r.tick(Controls::default())?;
            }
            mark(r, &format!("secured {what} ({coming} still coming)"));
            return Ok(());
        }
        for _ in 0..30 {
            r.tick(Controls::default())?;
        }
        waited += 0.25;
    }
}

/// Throw the Blade at the nearest awake guard that shoots from where it stands, within throwing
/// range, until it falls or `seconds` are up. `Route::clear` fights whatever is nearest; when
/// the Will for Cards is gone the ranged guard is the one that must not be left for last.
fn snipe(r: &mut Route, what: &str, seconds: f32) -> Result<()> {
    let portals = r.teleports;
    let mut fired = false;
    for _ in 0..(seconds / FIXED_DT).ceil() as usize {
        let Some(target) = foes(r)
            .into_iter()
            .filter(|f| f.shoots && f.feet.distance(r.player.feet) < THROW_RANGE)
            .min_by(|a, b| {
                a.feet
                    .distance_squared(r.player.feet)
                    .total_cmp(&b.feet.distance_squared(r.player.feet))
            })
        else {
            break;
        };
        if !fired {
            mark(r, &format!("sniping {what}"));
            fired = true;
        }
        r.aim_at = Some(target.id);
        let ticked = r.tick(Controls::default());
        r.aim_at = None;
        ticked?;
        if r.exited() {
            break;
        }
    }
    ensure!(
        r.teleports == portals,
        "Alice took a portal while sniping {what}"
    );
    if fired {
        mark(r, &format!("sniped {what}"));
    }
    Ok(())
}

/// Seconds until a cycle of `cycle` seconds that stands at `now` next reaches `phase`.
fn until_phase(now: f32, phase: f32, cycle: f32) -> f32 {
    (phase - now).rem_euclid(cycle)
}
/// The first clock reading at or after `from` that stands `beat` seconds into a beat of `period`.
fn next_beat(from: f32, beat: f32, period: f32) -> f32 {
    beat + ((from - beat) / period).ceil() * period
}
/// The tick that reaches a phase must be the one that acts on it: aim half a tick early.
const HALF_TICK: f32 = FIXED_DT / 2.;
/// The period of the swing of the arches' steps, in seconds.
const ARCH_BEAT: f32 = 4.;

/// Wait for the flipping corridor's cycle to reach `phase` seconds. A phase that has only just
/// gone by is taken as reached; a corridor that has not started is an error, because nothing
/// is moving yet.
fn wait_long(r: &mut Route, phase: f32) -> Result<()> {
    let Some(now) = beyond(r).long_phase() else {
        anyhow::bail!("The flipping corridor has not started");
    };
    let ahead = until_phase(now, phase - HALF_TICK, LONG_CYCLE);
    if ahead > LONG_CYCLE - 0.25 {
        return Ok(());
    }
    let due = beyond(r).state.age + ahead;
    wait_until(
        r,
        &format!("the corridor to reach {phase} s"),
        ahead + 1.,
        |r| beyond(r).state.age >= due,
    )
}
/// The arches' steps swing on a four-second beat once they are up. Wait until the arch clock
/// stands `beat` seconds into a beat, no earlier than `not_before`.
fn wait_arch(r: &mut Route, beat: f32, not_before: f32) -> Result<()> {
    let now = beyond(r).arch_time();
    let due = next_beat(now.max(not_before) - HALF_TICK, beat, ARCH_BEAT) - HALF_TICK;
    wait_until(
        r,
        &format!("the arches to reach {due:.2} s"),
        (due - now).max(0.) + 1.,
        |r| beyond(r).arch_time() >= due,
    )
}

// Hold an ordinary direction until an authored portal or exit takes over.
fn portal(r: &mut Route, direction: Vec2) -> Result<()> {
    let count = r.teleports;
    for _ in 0..600 {
        r.tick(Controls {
            wish: direction,
            run: true,
            ..Default::default()
        })?;
        if r.teleports > count || r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!("Portal not reached at {:?}", r.player.feet)
}
/// Play the lever Alice is standing at, facing east.
fn use_lever(r: &mut Route) -> Result<()> {
    r.tick(Controls {
        wish: vec2(1.0, 0.0),
        run: false,
        jump: false,
        use_pressed: true,
        ..Default::default()
    })
}
fn step(r: &mut Route, index: usize) -> Result<()> {
    let approach = beyond(r).step_goal(index) - r.player.feet;
    if approach.truncate().length() > 210. {
        r.walk_xy(r.player.feet.truncate() + approach.truncate().normalize_or_zero() * 28.)?;
    }
    let mut previous = beyond(r).step_goal(index).z;
    let mut launched = false;
    for _ in 0..1200 {
        let goal = beyond(r).step_goal(index);
        let d = goal - r.player.feet;
        let distance = d.truncate().length();
        if distance < 16. && r.player.grounded && d.z.abs() < 3. {
            println!("STEP {index} {:?}", r.player.feet);
            return Ok(());
        }
        let jump = !launched
            && r.player.grounded
            && d.z < 110.
            && goal.z < previous
            && goal.z > 25.6 * (16 - index) as f32 + 80.;
        if jump {
            println!("LAUNCH {index} feet {:?} goal {goal:?}", r.player.feet);
        }
        if jump {
            launched = true;
        }
        let wish = if launched {
            let speed = (2. * 420. * distance).sqrt().min(320.) * 0.95;
            if distance > 1. {
                d.truncate().normalize_or_zero() * speed / 320.
            } else {
                -r.player.velocity.truncate() * 0.001
            }
        } else {
            Vec2::ZERO
        };
        previous = goal.z;
        r.tick(Controls {
            wish,
            run: true,
            jump,
            ..Default::default()
        })?;
    }
    anyhow::bail!("Step {index} blocked at {:?}", r.player.feet)
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let mut r = Route::new(assets, "fortress2", None)?;
    drive(&mut r)?;
    println!("PASS Beyond the Wall entrance to Fortress return: {} ticks, {} jumps, {} Blade throws, {} swings, {} cards, {} combat damage, {} Sanity remaining; three authored portals, no flight/recovery/warps",r.ticks,r.jumps,r.shots,r.swings,r.cards,r.damage,r.stats.sanity());
    Ok(())
}
/// The hallway to the return door: three authored portals and the musical lever puzzle.
pub fn drive(r: &mut Route) -> Result<()> {
    r.tactics = true;
    hall(r)?;
    corridor(r)?;
    portal(r, vec2(1.0, 0.0))?;
    ensure!(r.teleports == 1, "Unexpected recovery portal during route");
    mark(r, "first portal");
    puzzle_room(r)?;
    walkway(r)?;
    portal(r, vec2(1.0, 0.0))?;
    ensure!(r.teleports == 2, "Unexpected recovery portal during route");
    mark(r, "second portal");
    arches(r)?;
    portal(r, vec2(0.0, 1.0))?;
    ensure!(r.teleports == 3, "Unexpected recovery portal during route");
    mark(r, "third portal");
    return_doors(r)?;
    let state = &beyond(r).state;
    ensure!(
        state.solved && state.walkway == 9 && state.last_open,
        "Incomplete level sequence"
    );
    ensure!(
        r.teleports == 3 && r.stats.alive(),
        "Unexpected recovery or death"
    );
    ensure!(
        r.transition == Some(("fortress1".into(), Some("fortress1_start2".into()))),
        "Wrong return destination: {:?}",
        r.transition
    );
    Ok(())
}

/// Enter through the hallway. The guards wake as Alice walks by; the Vial of Will and the
/// essence lie a little off the way, and the resources the fights spend are earned back here.
fn hall(r: &mut Route) -> Result<()> {
    r.wait(1.0)?;
    r.walk_xy(vec2(-750.0, 4128.0))?;
    r.walk_xy(vec2(-750.0, 3800.0))?;
    mark(r, "hall entered");
    // The Vial of Will, before the guards wake.
    r.navigate(vec3(-944.0, 3360.0, 0.0))?;
    r.navigate(vec3(-560.0, 3800.0, 0.0))?;
    mark(r, "vial taken");
    // The club guards are walked away from and shot down with Cards as they follow: they
    // cost Sanity when they are fought at arm's length.
    r.walk_xy(vec2(450.0, 3800.0))?;
    // The essence beside the way.
    r.navigate(vec3(608.0, 4128.0, 0.0))?;
    r.navigate(vec3(450.0, 3800.0, 0.0))?;
    mark(r, "essence taken");
    r.walk_xy(vec2(450.0, 3360.0))?;
    r.walk_xy(vec2(850.0, 3360.0))?;
    r.walk_xy(vec2(1500.0, 3360.0))?;
    r.walk_xy(vec2(1660.0, 3360.0))?;
    mark(r, "corridor door");
    Ok(())
}

/// The flipping corridor: four platforms that hop along it in turn, then come back. Each move
/// waits for its phase of the platforms' own cycle, which starts when Alice crosses the
/// trigger at the door. Easy has no platforms: the corridor is a row of static pads and
/// Alice jumps from one to the next.
fn corridor(r: &mut Route) -> Result<()> {
    if !beyond(r).has_object("longwalk1") {
        for goal in [
            vec3(2200.0, 3452.0, -64.0),
            vec3(2450.0, 3452.0, -64.0),
            vec3(2800.0, 3484.0, -64.0),
            vec3(3100.0, 3452.0, -64.0),
            vec3(3540.0, 3520.0, -64.0),
        ] {
            r.navigate(goal)?;
        }
        r.walk_xy(vec2(3680.0, 3520.0))?;
        return Ok(());
    }
    wait_long(r, 8.883)?;
    r.walk(vec3(1800.0, 3380.0, -64.0), true)?;
    wait_long(r, 12.300)?;
    r.walk_xy(vec2(1880.0, 3392.0))?;
    wait_long(r, 18.801)?;
    r.walk(vec3(2100.0, 3380.0, -64.0), true)?;
    wait_long(r, 27.794)?;
    r.walk(vec3(2340.0, 3392.0, -64.0), true)?;
    wait_long(r, 36.805)?;
    r.walk_xy(vec2(2360.0, 3410.0))?;
    r.navigate(vec3(2664.0, 3504.0, -64.0))?;
    wait_long(r, 46.490)?;
    r.walk_xy(vec2(2760.0, 3488.0))?;
    wait_long(r, 55.954)?;
    r.navigate(vec3(2980.0, 3456.0, -64.0))?;
    wait_long(r, 66.552)?;
    r.navigate(vec3(3150.0, 3580.0, -64.0))?;
    wait_long(r, 75.341)?;
    r.navigate(vec3(3350.0, 3560.0, -64.0))?;
    r.navigate(vec3(3540.0, 3520.0, -64.0))?;
    r.walk_xy(vec2(3680.0, 3520.0))?;
    Ok(())
}

/// Listen, descend and play the three levers in their authored order, then climb back.
fn puzzle_room(r: &mut Route) -> Result<()> {
    r.wait(1.0)?;
    // The stairs down are open from the arrival floor: the fall to the floor below is a few
    // units, not the whole pit.
    r.navigate(vec3(1008.0, 627.0, 0.0))?;
    r.navigate(vec3(1300.0, 790.0, -328.0))?;
    mark(r, "foot of the stairs");
    // The essence under the arrival floor.
    r.navigate(vec3(1000.0, 352.0, -333.0))?;
    for (x, y) in [(1320.0, 465.0), (1320.0, 593.0), (1320.0, 529.0)] {
        r.navigate(vec3(x, y, -336.0))?;
        use_lever(r)?;
    }
    ensure!(beyond(r).state.solved, "Lever puzzle unsolved");
    mark(r, "levers played");
    // The Boojums that came for Alice on the stairs are dealt with on the wide floor down here:
    // a scream's shove on the narrow stairs would drop her into the pit.
    secure(r, "lever room", 1000., 1300., 15.)?;
    // A fight leaves Alice where it left her, and sliding from the last shove: plan the way back
    // to the stairs instead of walking a straight line that may cross the pit's edge.
    r.navigate(vec3(1300.0, 790.0, -328.0))?;
    // Climb the basement spiral to the bridge.
    for (x, y) in [
        (1390.0, 850.0),
        (1435.0, 945.0),
        (1380.0, 1040.0),
        (1300.0, 1050.0),
        (1180.0, 960.0),
        (1160.0, 870.0),
        (1080.0, 790.0),
        (990.0, 710.0),
        (1010.0, 620.0),
        (1200.0, 506.0),
    ] {
        r.walk_xy(vec2(x, y))?;
    }
    // Whatever still hunts Alice is dealt with from the middle of the arrival floor, away from
    // its edges and well short of the walkway's trigger (at x 1640), before the walkway moves
    // under her.
    secure(r, "the walkway's start", 1000., 1300., 12.)?;
    r.navigate(vec3(1300.0, 576.0, 0.0))?;
    r.walk_xy(vec2(1456.0, 576.0))?;
    // A Boojum that perches out of reach only wakes when Alice comes within 1,200 units of it,
    // and from the middle of the floor the far ones are still asleep: they wake on the
    // walkway, where their screams shove Alice off it. Standing at the floor's east end
    // brings them, and they are fought here, on the wide floor.
    secure(r, "the walkway's foot", 1000., 1200., 15.)?;
    r.walk_xy(vec2(1552.0, 608.0))?;
    Ok(())
}

/// The rolling walkway: each step Alice takes onto the piece under her feet slides the piece
/// behind her to the front, and the triggers stay shut until it lands. Wait for the machinery
/// to say so before each step.
fn walkway(r: &mut Route) -> Result<()> {
    let mut expected = beyond(r).state.walkway;
    for (x, y) in [
        (1638.0, 636.0),
        (1734.0, 668.0),
        (1830.0, 700.0),
        (1926.0, 732.0),
        (2022.0, 764.0),
        (2118.0, 796.0),
        (2214.0, 828.0),
    ] {
        r.walk_xy(vec2(x, y))?;
        expected += 1;
        wait_until(r, "the walkway step", 3., |r| {
            let b = beyond(r);
            b.state.walkway >= expected && !b.sliding()
        })?;
    }
    r.walk_xy(vec2(2320.0, 864.0))?;
    mark(r, "walkway crossed");
    // The Diamond in the room north of the bridge shoots as soon as Alice is in its sight, and
    // what it drops pays for the rest of the fight: it goes first, before the club guards close in.
    snipe(r, "the bridge's Diamond", 5.)?;
    r.walk(vec3(2500.0, 896.0, 0.0), true)?;
    secure(r, "bridge", 1000., 900., 6.)?;
    r.navigate(vec3(2860.0, 896.0, 0.0))?;
    r.walk_xy(vec2(3160.0, 896.0))?;
    r.walk_xy(vec2(3450.0, 896.0))?;
    r.walk_xy(vec2(3800.0, 896.0))?;
    Ok(())
}

/// Raise the arches, then time the staircase jumps from the live poses.
fn arches(r: &mut Route) -> Result<()> {
    r.wait(1.0)?;
    r.walk_xy(vec2(5280.0, -4260.0))?;
    // The room is up when its clock passes 18.4 s; the steps then swing on a four-second beat.
    wait_arch(r, 3.39, 19.39)?;
    r.walk_xy(vec2(5280.0, -4040.0))?;
    r.walk(vec3(5280.0, -3872.0, 42.0), true)?;
    r.walk_xy(vec2(5280.0, -3830.0))?;
    r.walk(vec3(5408.0, -3808.0, 90.0), true)?;
    r.wait(1.0)?;
    for index in [12, 11, 7, 6, 5, 3, 1] {
        step(r, index)?;
    }
    r.walk_xy(vec2(5280.0, -3040.0))?;
    wait_arch(r, 1.78, 0.)?;
    for _ in 0..18 {
        r.tick(Controls {
            wish: vec2(0.0, 1.0),
            run: true,
            jump: false,
            use_pressed: false,
            ..Default::default()
        })?;
    }
    r.walk(vec3(5280.0, -2810.0, 576.0), true)?;
    r.walk_xy(vec2(5280.0, -2800.0))?;
    Ok(())
}

/// Follow the revealed door through its shuffle and jump to the left walkway.
fn return_doors(r: &mut Route) -> Result<()> {
    secure(r, "return hall", 1000., 1300., 8.)?;
    wait_until(r, "the doors to settle", 30., |r| beyond(r).last_ready())?;
    r.navigate(vec3(-544.0, 4200.0, 384.0))?;
    r.walk_xy(vec2(-544.0, 4070.0))?;
    r.walk_xy(vec2(-544.0, 3950.0))?;
    r.walk_xy(vec2(-536.0, 3820.0))?;
    portal(r, vec2(0.0, -1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cycle_is_waited_for_forward_and_around_its_turn() {
        assert_eq!(until_phase(8., 10., 88.5), 2.);
        assert_eq!(until_phase(10., 8., 88.5), 86.5);
        assert_eq!(until_phase(80., 4., 88.5), 12.5);
        // Standing on the phase means no wait; a hair past it is nearly a whole turn.
        assert_eq!(until_phase(5., 5., 88.5), 0.);
        assert!(until_phase(5.01, 5., 88.5) > 88.4);
    }
    #[test]
    fn the_next_beat_is_the_first_reading_on_the_beat_at_or_after_the_start() {
        let period = 4.;
        // The arches' clock at the door and at the staircase in the recorded route.
        assert!((next_beat(19.386, 3.39, period) - 19.39).abs() < 1e-3);
        assert!((next_beat(48.33, 1.78, period) - 49.78).abs() < 1e-3);
        // A start already on the beat waits for nothing; one just past it for a whole beat.
        assert!((next_beat(49.78, 1.78, period) - 49.78).abs() < 1e-3);
        assert!((next_beat(49.79, 1.78, period) - 53.78).abs() < 1e-3);
        // A beat more than a period after the start is still the first one on or after it.
        assert!((next_beat(0.39, 13.39, period) - 1.39).abs() < 1e-3);
        for from in [0., 0.39, 3.99, 17.2, 90.] {
            let t = next_beat(from, 1.78, period);
            assert!(t >= from - 1e-4 && t - from < period);
            let turns = (t - 1.78) / period;
            assert!((turns - turns.round()).abs() < 1e-3);
        }
    }
}

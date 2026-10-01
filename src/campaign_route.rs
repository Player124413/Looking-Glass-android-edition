//! The campaign chain's legs: one driver per visit of the opening segment, aligned with
//! `campaign::ROUTE`. A leg is a recorded route body (`drive`) plus what the chain asserts about
//! it. `Route::depart` (`campaign::arrive`) joins legs into a chain that carries one `Stats` and
//! one ledger; `docs/CAMPAIGN.md` describes the API the campaign chain check is built on.
//!
//! `--campaign-legs-check` runs each opening leg under the chain's assertions and hands its exit
//! to the next visit through the shared transition, then reports the legs chained on carried
//! resources; `--route-difficulty-check` proves that a route's difficulty reaches the world.
use crate::{
    assets::Assets,
    campaign,
    inventory::Stats,
    powerups::Difficulty,
    route::{Metrics, Route},
};
use anyhow::{ensure, Context, Result};

/// A route body: it starts at the visit's entrance and ends when the exit is taken.
pub type Drive = fn(&mut Route) -> Result<()>;

/// One visit of the chain.
pub struct Leg {
    pub map: &'static str,
    pub entry: Option<&'static str>,
    /// The exit the leg must end on, spelled the way its level change spells the destination.
    pub exit: (&'static str, Option<&'static str>),
    /// The authored teleports on the way (recovery portals count: they are part of the level).
    pub portals: usize,
    /// Ticks the recorded route wades through slime (damage-over-time that Alice survives). Zero
    /// for a route that keeps out of it; a leg that must wade documents its own figure.
    pub slime: usize,
    pub drive: Drive,
    /// An alternative body that also takes an optional reward (school one's secret room).
    pub optional: Option<Drive>,
}

/// Visits 1 to 8 (`ROUTE` indexes 0 to 7), in order.
pub static LEGS: [Leg; 8] = [
    Leg {
        map: "gvillage",
        entry: None,
        exit: ("pandemonium", Some("player_start")),
        portals: 0,
        slime: 0,
        drive: crate::village_route::drive,
        optional: None,
    },
    Leg {
        map: "pandemonium",
        entry: None,
        exit: ("fortress1", Some("fortress1_start1")),
        portals: 1,
        // The recorded route wades the slime pit at the foot of the rope, and again after the
        // return portal.
        slime: 460,
        drive: crate::pandemonium_route::drive,
        optional: None,
    },
    Leg {
        map: "fortress1",
        entry: None,
        exit: ("fortress2", None),
        portals: 1,
        slime: 0,
        drive: crate::fortress_route::drive_first,
        optional: None,
    },
    Leg {
        map: "fortress2",
        entry: None,
        exit: ("fortress1", Some("fortress1_start2")),
        portals: 3,
        slime: 0,
        drive: crate::beyond_route::drive,
        optional: None,
    },
    Leg {
        map: "fortress1",
        entry: Some("fortress1_start2"),
        exit: ("skool1", Some("skool1_start1")),
        portals: 0,
        slime: 0,
        drive: crate::fortress_route::drive_return,
        optional: None,
    },
    Leg {
        map: "skool1",
        entry: None,
        exit: ("skool2", Some("skool2_start1")),
        portals: 0,
        slime: 0,
        drive: crate::school_route::drive,
        optional: Some(crate::school_route::drive_secret),
    },
    Leg {
        map: "skool2",
        entry: None,
        exit: ("skool1", Some("skool1_start2")),
        portals: 2,
        slime: 0,
        drive: crate::school2_route::drive,
        optional: None,
    },
    Leg {
        map: "skool1",
        entry: Some("skool1_start2"),
        exit: ("potears1", Some("potears1_start1")),
        portals: 0,
        slime: 0,
        drive: crate::school_return_route::drive,
        optional: None,
    },
];

impl Leg {
    /// The body a run of this leg drives: the optional one when asked for and present.
    pub fn body(&self, optional: bool) -> Drive {
        match (optional, self.optional) {
            (true, Some(drive)) => drive,
            _ => self.drive,
        }
    }
    /// The route for this leg: Alice at the entrance carrying `carried`, latched at its exit.
    pub fn start(
        &self,
        assets: &mut Assets,
        carried: Stats,
        difficulty: Difficulty,
    ) -> Result<Route> {
        let mut r = Route::enter(assets, self.map, self.entry, carried, difficulty)?;
        r.stop_at_exit = true;
        Ok(r)
    }
    /// Drive the leg (`optional`: its variant that also takes the optional reward) and assert
    /// what every leg shares: the authored exit, the authored teleports, no lava, no recovery, one
    /// transition and Alice alive throughout.
    pub fn run(&self, r: &mut Route, optional: bool) -> Result<Metrics> {
        self.body(optional)(r)?;
        let exit = r
            .transition
            .clone()
            .with_context(|| format!("{} left no exit at {:?}", self.map, r.player.feet))?;
        ensure!(
            exit.0 == self.exit.0 && exit.1.as_deref() == self.exit.1,
            "{} took exit {exit:?}, expected {:?}",
            self.map,
            self.exit
        );
        r.assert_clean(self.portals, self.slime)?;
        Ok(r.metrics())
    }
}

/// The visit index of a leg's exit, which is always the next `ROUTE` entry.
fn next_index(leg: &Leg) -> Option<usize> {
    campaign::visit_index(leg.exit.0, leg.exit.1)
}

/// Run legs `legs` (positions in [`LEGS`]) back to back: the first starts from its own baseline
/// resources (New Game state for leg 0), and each exit carries one `Stats` and one ledger to the
/// next leg through `Route::depart`, at `difficulty`. `strict` skips the campaign baseline
/// loadout, so every toy must come from its authored pickup. Returns each leg's metrics and the
/// route standing at the entrance of the visit after the last leg (the chain's frontier).
pub fn chain(
    assets: &mut Assets,
    legs: std::ops::Range<usize>,
    difficulty: Difficulty,
    strict: bool,
    skip_cinematics: bool,
) -> Result<(Vec<Metrics>, Route)> {
    ensure!(
        !legs.is_empty() && legs.end <= LEGS.len(),
        "The opening chain has {} legs",
        LEGS.len()
    );
    let first = &LEGS[legs.start];
    let mut r = first.start(assets, Stats::for_level(first.map, first.entry), difficulty)?;
    let mut report = Vec::new();
    for i in legs {
        r.skip_cinematics = skip_cinematics;
        let m = LEGS[i]
            .run(&mut r, false)
            .with_context(|| format!("Visit {:02} {}", i + 1, campaign::visits()[i]))?;
        println!("PASS visit {:02} {}: {m}", i + 1, campaign::visits()[i]);
        report.push(m);
        r = r.depart(assets, strict).with_context(|| {
            format!("Entering visit {:02} from {}", i + 2, campaign::visits()[i])
        })?;
        r.stop_at_exit = true;
    }
    Ok((report, r))
}

/// What an exit hands over, proven on real data: the destination is the leg's authored exit, the
/// visit that was left is in the ledger as completed, the resources carry over untouched by a
/// strict exit, and the ledger stays far inside the save's limits.
fn check_boundary(before: &Stats, leg: &Leg, next: &Route, strict: bool) -> Result<()> {
    ensure!(
        next.ledger.completed.len() == 1
            && next
                .ledger
                .completed
                .contains(&crate::save::visit_key(leg.map, leg.entry))
            && next.ledger.levels.len() == 1,
        "The ledger does not hold exactly the visit that was left"
    );
    let (kept, carried) = (
        serde_json::to_value(before)?,
        serde_json::to_value(&next.stats)?,
    );
    if strict {
        ensure!(kept == carried, "A strict exit changed what Alice carries");
    } else {
        ensure!(
            next.stats.sanity() == before.sanity() && next.stats.will() == before.will(),
            "An exit changed Sanity or Will"
        );
    }
    ensure!(
        next.ticks == 0 && next.transition.is_none() && next.stats.alive(),
        "The next visit did not start fresh at its entrance"
    );
    ensure!(
        serde_json::to_vec(&next.ledger)?.len() < 1 << 20,
        "The ledger is unexpectedly large"
    );
    Ok(())
}

/// Every visit's entrance, on real data. An exit into an obstructed entrance must fail and leave
/// the resources and the ledger as they were (the viewer used to drop Alice into flight there).
/// The entrances of the eight opening visits must be clear; the others are reported, because a
/// visit that is not built yet can be obstructed today.
fn check_entrances(assets: &mut Assets) -> Result<()> {
    let mut obstructed = Vec::new();
    for (i, &(map, entry)) in campaign::route().iter().enumerate() {
        let loaded = campaign::load_visit(assets, map, entry, None, Difficulty::Normal, false)?;
        let (eye, _) = crate::interaction::spawn(&loaded.map, entry);
        if crate::movement::Player::spawn(&loaded.world, eye).is_none() {
            ensure!(
                i >= LEGS.len(),
                "The entrance of visit {:02} is obstructed",
                i + 1
            );
            obstructed.push(i);
        }
    }
    println!(
        "Entrances: {} of {} clear{}",
        campaign::route().len() - obstructed.len(),
        campaign::route().len(),
        if obstructed.is_empty() {
            String::new()
        } else {
            format!(
                "; obstructed: {}",
                obstructed
                    .iter()
                    .map(|&i| campaign::visits()[i].as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    );
    // An exit that cannot be entered (here: a named entrance the map does not have, and any
    // obstructed entrance there may be) fails and leaves the resources and the ledger alone.
    let mut unenterable = vec![("skool1".to_owned(), Some("missing_entrance".to_owned()))];
    unenterable.extend(obstructed.iter().map(|&i| {
        let (map, entry) = campaign::route()[i];
        (map.to_owned(), entry.map(str::to_owned))
    }));
    for exit in unenterable {
        let leaving = Route::new(assets, "gvillage", None)?.level();
        let mut stats = Stats::for_level("gvillage", None);
        let before = serde_json::to_value(&stats)?;
        let mut ledger = crate::save::Campaign::default();
        ensure!(
            campaign::arrive(assets, &mut stats, &mut ledger, leaving, &exit, false).is_err(),
            "An exit into {exit:?} succeeded"
        );
        ensure!(
            serde_json::to_value(&stats)? == before
                && ledger.levels.is_empty()
                && ledger.completed.is_empty(),
            "A failed exit into {exit:?} changed the resources or the ledger"
        );
        println!("PASS an exit into {exit:?} fails and changes nothing");
    }
    Ok(())
}

/// One leg alone from its own baseline resources: the leg's assertions (`Leg::run`) and, for the
/// body that ends the visit, the hand-over of its exit to the next visit through `Route::depart`.
fn alone(
    assets: &mut Assets,
    i: usize,
    difficulty: Difficulty,
    strict: bool,
    optional: bool,
) -> Result<()> {
    let leg = &LEGS[i];
    let mut r = leg.start(assets, Stats::for_level(leg.map, leg.entry), difficulty)?;
    let m = leg.run(&mut r, optional).with_context(|| {
        format!(
            "Visit {:02} {}{}",
            i + 1,
            campaign::visits()[i],
            if optional { " optional" } else { "" }
        )
    })?;
    if optional {
        println!(
            "PASS visit {:02} {} with its optional reward alone: {m}",
            i + 1,
            campaign::visits()[i]
        );
        return Ok(());
    }
    println!(
        "PASS visit {:02} {} alone: {m}",
        i + 1,
        campaign::visits()[i]
    );
    let before = r.stats.clone();
    let next = r
        .depart(assets, strict)
        .with_context(|| format!("Leaving visit {:02}", i + 1))?;
    check_boundary(&before, leg, &next, strict)
        .with_context(|| format!("Leaving visit {:02}", i + 1))
}

/// `--campaign-legs-check`: each of the eight opening legs from its own baseline resources under
/// the chain's assertions (`Leg::run`), with the exit handing over to the next visit through the
/// shared transition (`Route::depart`), then the legs chained on carried resources. At Normal every
/// leg gates. At any other difficulty a leg that does not pass is reported and the run goes on to
/// the next (the plan reports Easy and Hard and never gates on them: the enemies and timings the
/// recorded drivers were tuned against differ there). The chained run is reported at every
/// difficulty: the recorded drivers were tuned from a full Sanity bar at every entrance, so a
/// chain of them can run out of Sanity (`docs/CAMPAIGN.md`).
pub fn check(assets: &mut Assets, difficulty: Difficulty, strict: bool) -> Result<()> {
    ensure!(
        LEGS.len() == 8
            && LEGS.iter().enumerate().all(|(i, leg)| {
                campaign::route()[i] == (leg.map, leg.entry) && next_index(leg) == Some(i + 1)
            }),
        "The legs are not aligned with the campaign route"
    );
    check_entrances(assets)?;
    let gating = difficulty == Difficulty::Normal;
    let mut failing = Vec::new();
    for i in 0..LEGS.len() {
        if let Err(e) = alone(assets, i, difficulty, strict, false) {
            ensure!(!gating, "{e:#}");
            println!("REPORTED not passing at {}: {e:#}", difficulty.name());
            failing.push(campaign::visits()[i].clone());
        }
    }
    // The optional body of a leg (school one's secret room) runs under the same assertions.
    for i in (0..LEGS.len()).filter(|&i| LEGS[i].optional.is_some()) {
        if let Err(e) = alone(assets, i, difficulty, strict, true) {
            ensure!(!gating, "{e:#}");
            println!("REPORTED not passing at {}: {e:#}", difficulty.name());
            failing.push(format!("{} optional", campaign::visits()[i]));
        }
    }
    let bodies = LEGS.len() + LEGS.iter().filter(|l| l.optional.is_some()).count();
    if failing.is_empty() {
        println!(
            "PASS campaign legs 1-8 at {}{}: every leg alone from its baseline and every exit handed over",
            difficulty.name(),
            if strict { ", strict" } else { "" }
        );
    } else {
        println!(
            "REPORT campaign legs at {}: {} of {bodies} leg bodies do not pass alone (not gating): {}",
            difficulty.name(),
            failing.len(),
            failing.join(", ")
        );
    }
    // Chained on carried resources: all eight legs, and the school's three (visits 6 to 8).
    for (label, legs) in [("legs 1-8", 0..LEGS.len()), ("school, visits 6-8", 5..8)] {
        match chain(assets, legs.clone(), difficulty, strict, false) {
            Ok((report, frontier)) => println!(
                "CHAIN {label}: one Stats and one ledger carried through {} ticks; FRONTIER {} entered with {:.0} Sanity, {} visits in the ledger",
                report.iter().map(|m| m.ticks).sum::<usize>(),
                campaign::visits()[legs.end],
                frontier.stats.sanity(),
                frontier.ledger.levels.len()
            ),
            Err(e) => println!("CHAIN {label} not gating: stopped by {e:#}"),
        }
    }
    Ok(())
}

/// What a difficulty changes when a visit loads: the enemies placed (encounter actors and club
/// guards) and the pickups lying about, each filtered by the authored inhibit bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Census {
    actors: usize,
    guards: usize,
    pickups: usize,
}
impl Census {
    fn of(r: &Route) -> Self {
        Self {
            actors: r
                .interactions
                .encounters
                .as_ref()
                .map_or(0, |e| e.actors.len()),
            guards: r.guards.len(),
            pickups: r.pickups.len(),
        }
    }
}

/// `--route-difficulty-check` (R3): the difficulty is a real input of `Route`. For every visit of
/// the campaign the loader builds the enemies and pickups at Easy, Normal and Hard; they differ
/// wherever the authored spawn flags differ. For the eight opening visits the route built by
/// `Route::enter` holds exactly what the loader built (so the difficulty reached its world), and
/// carries the difficulty in its map and its resources, where it scales the damage Alice takes.
pub fn difficulty_check(assets: &mut Assets) -> Result<()> {
    const LEVELS: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];
    let catalog = crate::inventory::Catalog::load(assets)?;
    let mut differing = Vec::new();
    let mut loaded = Vec::new();
    for (i, &(map, entry)) in campaign::route().iter().enumerate() {
        let mut census = Vec::new();
        for d in LEVELS {
            let v = campaign::load_visit(assets, map, entry, None, d, false)?;
            ensure!(v.map.difficulty == d, "{map} did not take {}", d.name());
            census.push(Census {
                actors: v
                    .interactions
                    .encounters
                    .as_ref()
                    .map_or(0, |e| e.actors.len()),
                guards: crate::route::guards_of(&v.map, map, &v.interactions).len(),
                pickups: crate::inventory::pickups_for_visit(&v.map, map, entry, &catalog).len(),
            });
        }
        if census[0] != census[1] || census[1] != census[2] {
            differing.push(i);
            println!(
                "{:02} {}: easy {:?}, normal {:?}, hard {:?}",
                i + 1,
                campaign::visits()[i],
                census[0],
                census[1],
                census[2]
            );
        }
        loaded.push(census);
    }
    ensure!(
        !differing.is_empty(),
        "No visit differs between difficulties: the authored spawn flags never reach the loader"
    );
    // Headless club guards stand in for the cast the viewer builds with textures: school two has
    // its placed guards, and a visit whose controller owns its cast (Pandemonium) has none.
    let guarded: Vec<_> = (0..LEGS.len())
        .filter(|&i| loaded[i][1].guards > 0)
        .collect();
    ensure!(
        guarded.contains(&6) && !guarded.contains(&1),
        "Placed club guards are wrong: {guarded:?}"
    );
    println!(
        "Placed club guards at Normal: {}",
        guarded
            .iter()
            .map(|&i| format!("{} {}", campaign::visits()[i], loaded[i][1].guards))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut opening = 0;
    for (i, leg) in LEGS.iter().enumerate() {
        for (k, d) in LEVELS.into_iter().enumerate() {
            let mut r = leg.start(assets, Stats::for_level(leg.map, leg.entry), d)?;
            ensure!(
                r.difficulty == d && r.map.difficulty == d && r.stats.difficulty == d,
                "{} did not carry {} into the route",
                leg.map,
                d.name()
            );
            ensure!(
                Census::of(&r) == loaded[i][k],
                "{} at {}: the route's world differs from the loader's: {:?} against {:?}",
                leg.map,
                d.name(),
                Census::of(&r),
                loaded[i][k]
            );
            let before = r.stats.sanity();
            r.stats.damage(10.);
            ensure!(
                (before - r.stats.sanity() - 10. * d.incoming()).abs() < 1e-3,
                "{} at {}: damage is not scaled by the difficulty",
                leg.map,
                d.name()
            );
        }
        if differing.contains(&i) {
            opening += 1;
        }
    }
    println!(
        "PASS route difficulty: {} of 39 visits load different enemies or pickups between Easy, Normal and Hard ({opening} of the 8 opening visits); routes carry the difficulty in the map, the resources and the damage they take",
        differing.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legs_follow_the_campaign_route_and_its_authored_portals() {
        let route = campaign::route();
        assert_eq!(
            LEGS.iter().map(|l| l.portals).collect::<Vec<_>>(),
            [0, 1, 1, 3, 0, 0, 2, 0]
        );
        for (i, leg) in LEGS.iter().enumerate() {
            assert_eq!(route[i], (leg.map, leg.entry), "leg {i}");
            // Each exit leads to the next visit, however it spells its entrance.
            assert_eq!(next_index(leg), Some(i + 1), "exit of leg {i}");
            assert_eq!(route[i + 1].0, leg.exit.0, "exit of leg {i}");
        }
    }
    #[test]
    fn a_leg_with_an_optional_body_names_it() {
        // Only school one's secret room is optional in the opening segment.
        let optional: Vec<_> = LEGS
            .iter()
            .filter(|l| l.optional.is_some())
            .map(|l| (l.map, l.entry))
            .collect();
        assert_eq!(optional, [("skool1", None)]);
    }
}

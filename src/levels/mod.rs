//! The registration table (F1.2). A new visit adds its own module under `src/levels/` and
//! uncomments exactly its own `pub mod` line and its own registration line below; it does not
//! edit `interaction.rs`, `viewer.rs`, `route.rs`, `main.rs`, `save_check.rs` or `story.rs`.
//!
//! Visits owned by a typed legacy controller (gvillage, pandemonium, fortress1 both visits,
//! fortress2, skool1 both visits, skool2, potears1 and potears3) have no lines: they extend
//! their legacy module until a `system/port-<id>` task ports them.
//!
//! `LEVELS` is a plain static slice. There is deliberately no `inventory` or `linkme`
//! dependency, so a registration exists only if this file names it.
//!
//! The Keep arrival is registered. Contract helpers for later full controllers
//! remain available even when this partial visit does not use them.
#![allow(dead_code)]
use crate::{
    assets::Assets,
    bsp::Bsp,
    level::{LevelArt, LevelController},
};
use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    pin::Pin,
};

pub mod reservations;
pub mod state;
#[cfg(test)]
pub(crate) mod synthetic;

// One commented module line per remaining map, in campaign order (wforest once). Uncomment
// only your own line.

pub mod potears2;

pub mod utemple;

pub mod garden1;

pub mod garden2;

pub mod garden3;

pub mod garden4;

pub mod centipede1;

pub mod centipede2;

pub mod wforest;

pub mod wchess1;

pub mod wchess2;

pub mod rchess1;

pub mod funhouse;

pub mod hatter1;

pub mod hatter2;

pub mod jlair1;

pub mod jlair2;

pub mod hedge1;

pub mod tower1;

pub mod hedge2;

pub mod tower2;

pub mod hedge3;

pub mod tower3;

pub mod grounds1;

pub mod grounds2;

pub mod facade;

pub mod keep;

pub mod qlair;

/// Every registered visit, in campaign order. A registration is consulted only after the
/// legacy chains, so it can never change how an existing visit behaves. Uncomment only your
/// own line (wforest has two: `REGISTRATION` and `RETURN_REGISTRATION`).
pub static LEVELS: &[&Registration] = &[
    &potears2::REGISTRATION,
    &utemple::REGISTRATION,
    &garden1::REGISTRATION,
    &garden2::REGISTRATION,

    &garden3::REGISTRATION,

    &garden4::REGISTRATION,

    &centipede1::REGISTRATION,

    &centipede2::REGISTRATION,

    &wforest::REGISTRATION,

    &wchess1::REGISTRATION,

    &wchess2::REGISTRATION,

    &rchess1::REGISTRATION,

    &funhouse::REGISTRATION,

    &hatter1::REGISTRATION,

    &hatter2::REGISTRATION,

    &jlair1::REGISTRATION,

    &jlair2::REGISTRATION,

    &wforest::RETURN_REGISTRATION,

    &hedge1::REGISTRATION,

    &tower1::REGISTRATION,

    &hedge2::REGISTRATION,
    &tower2::REGISTRATION,
    &hedge3::REGISTRATION,

    &tower3::REGISTRATION,

    &grounds1::REGISTRATION,

    &grounds2::REGISTRATION,

    &facade::REGISTRATION,
    &keep::REGISTRATION,
    &qlair::REGISTRATION,
];

/// An entity's key/value pairs, as parsed from the map.
pub type Entity = BTreeMap<String, String>;
/// Whether the registration serves this `(map, entry)` visit.
pub type Applies = fn(&str, Option<&str>) -> bool;
/// Build the controller for a visit. `Interactions::set_entry` calls it after the legacy loads.
pub type Load = fn(&mut Assets, &Bsp, &str, Option<&str>) -> Result<Box<dyn LevelController>>;
/// Build the visit's art from its controller. Window-bound; called by the viewer.
pub type LoadArt = fn(&mut Assets, &Bsp, &dyn LevelController) -> Result<Box<dyn LevelArt>>;
/// Whether an inline model belongs to the controller (`map` name, entity).
pub type OwnsSubmodel = fn(&str, &Entity) -> bool;
/// Whether the controller owns a placed NPC (`name`, `model`), so the generic cast skips it.
pub type OwnsNpc = fn(&str, &str) -> bool;
/// The future of a windowed check or fixture.
pub type BoxFuture<'a> = Pin<Box<dyn Future<Output = Result<()>> + 'a>>;
/// A check that returns before any window exists.
pub type HeadlessFn = fn(&mut Assets) -> Result<()>;
/// A check that needs the window (`macroquad::Window::from_config`), so it runs in Anode.
/// Write it as `fn run(a: &mut Assets) -> BoxFuture<'_> { Box::pin(async move { ... }) }`.
pub type WindowedFn = for<'a> fn(&'a mut Assets) -> BoxFuture<'a>;
/// Stage a save case's state on a freshly restored visit before the writer saves it.
pub type StageFn = fn(&mut crate::interaction::Interactions, &Bsp) -> Result<()>;
/// A save case's behaviour checks on the visit `--save-check-read` restored in a fresh process,
/// after its continued simulation matched the writer's: no duplicate rewards, no dialogue
/// replay, no resurrection, scenes resume.
pub type BehaviorFn = fn(
    &mut crate::interaction::Interactions,
    &mut crate::inventory::Stats,
    &mut crate::story::Story,
) -> Result<()>;

/// How a registered `--<id>-*` flag runs.
#[derive(Clone, Copy)]
pub enum Run {
    Headless(HeadlessFn),
    Windowed(WindowedFn),
}

/// A command-line check a visit contributes. `main.rs` resolves unmatched `--<id>-*` flags here.
pub struct Check {
    pub flag: &'static str,
    /// One line for `--help`.
    pub help: &'static str,
    pub run: Run,
}

/// A named async fixture that `--visibility-check` runs after its legacy list.
pub struct VisibilityFixture {
    pub name: &'static str,
    pub run: WindowedFn,
}

/// One `--save-check-write` / `--save-check-read` case a visit contributes.
pub struct SaveCase {
    /// `<id>-<phase>`; `LOOKING_GLASS_SAVE_CASE=<id>-` selects a visit's cases.
    pub name: &'static str,
    /// The visit key the case is saved in (`garden1$first`).
    pub visit: &'static str,
    pub stage: Option<StageFn>,
    pub behavior: Option<BehaviorFn>,
}

/// A dialogue beat the story reader may load for a visit: an event name, the script file that
/// holds the linear dialogue call list (relative to `maps/cinematics/`), and the thread in it.
/// Identifiers only; never dialogue text.
pub use crate::story::BeatSpec as Beat;

pub struct Registration {
    /// The Appendix F id: the map name, or `wforest-return`.
    pub id: &'static str,
    pub applies: Applies,
    pub load: Load,
    pub art: Option<LoadArt>,
    pub owns_submodel: OwnsSubmodel,
    pub owns_npc: OwnsNpc,
    /// The start of this visit's exact hit-ID range `[base, base + 100_000)` (Appendix F).
    pub target_base: Option<usize>,
    pub story_beats: &'static [Beat],
    pub checks: &'static [Check],
    pub save_cases: &'static [SaveCase],
    pub visibility: &'static [VisibilityFixture],
}

/// The same rule as `save::visit_key`: the named return entrance is the return visit; the
/// default entrance and the named first start are the first visit.
fn returning(map: &str, entry: Option<&str>) -> bool {
    entry.is_some() && entry == crate::campaign::return_entry(map)
}
/// `applies` for a first visit of `name`.
pub fn first_visit(map: &str, entry: Option<&str>, name: &str) -> bool {
    map == name && !returning(map, entry)
}
/// `applies` for the return visit of `name`.
pub fn return_visit(map: &str, entry: Option<&str>, name: &str) -> bool {
    map == name && returning(map, entry)
}

/// The registrations of `list` that serve some visit of `map`.
pub fn for_map_in<'a>(
    list: &'a [&'static Registration],
    map: &'a str,
) -> impl Iterator<Item = &'static Registration> + 'a {
    list.iter().copied().filter(move |r| {
        (r.applies)(map, None)
            || crate::campaign::return_entry(map).is_some_and(|e| (r.applies)(map, Some(e)))
    })
}
/// The registrations that serve the exact visit.
pub fn serving_in<'a>(
    list: &'a [&'static Registration],
    map: &'a str,
    entry: Option<&'a str>,
) -> impl Iterator<Item = &'static Registration> + 'a {
    list.iter()
        .copied()
        .filter(move |r| (r.applies)(map, entry))
}
pub fn for_map(map: &str) -> impl Iterator<Item = &'static Registration> + '_ {
    for_map_in(LEVELS, map)
}
pub fn serving<'a>(
    map: &'a str,
    entry: Option<&'a str>,
) -> impl Iterator<Item = &'static Registration> + 'a {
    serving_in(LEVELS, map, entry)
}

/// Whether a registered controller owns this inline model of `map` (drawn and moved by it).
pub fn owns_submodel_in(list: &[&'static Registration], map: &str, e: &Entity) -> bool {
    for_map_in(list, map).any(|r| (r.owns_submodel)(map, e))
}
pub fn owns_submodel(map: &str, e: &Entity) -> bool {
    owns_submodel_in(LEVELS, map, e)
}
/// Whether a registered controller owns this placed NPC on this visit.
pub fn owns_npc_in(
    list: &[&'static Registration],
    map: &str,
    entry: Option<&str>,
    name: &str,
    model: &str,
) -> bool {
    serving_in(list, map, entry).any(|r| (r.owns_npc)(name, model))
}

/// Dialogue beats the registry adds for `map`, in the shape `story::beats` returns.
pub fn beats_in(
    list: &[&'static Registration],
    map: &str,
) -> Vec<(&'static str, &'static str, &'static str)> {
    for_map_in(list, map)
        .flat_map(|r| r.story_beats.iter())
        .filter(|b| b.map == map)
        .map(|b| (b.event, b.script, b.thread))
        .collect()
}
pub fn beats(map: &str) -> Vec<(&'static str, &'static str, &'static str)> {
    beats_in(LEVELS, map)
}
/// `(map, number of distinct beat events)` for every registered map with beats, so
/// `story::check` derives its expected counts instead of hard-coding them.
pub fn story_expectations_in(list: &[&'static Registration]) -> Vec<(&'static str, usize)> {
    let mut maps = BTreeMap::<&'static str, BTreeSet<&'static str>>::new();
    for b in list.iter().flat_map(|r| r.story_beats.iter()) {
        maps.entry(b.map).or_default().insert(b.event);
    }
    maps.into_iter().map(|(m, e)| (m, e.len())).collect()
}
pub fn story_expectations() -> Vec<(&'static str, usize)> {
    story_expectations_in(LEVELS)
}

/// The registered save cases, in registration order.
pub fn save_cases_in(list: &[&'static Registration]) -> Vec<&'static SaveCase> {
    list.iter().flat_map(|r| r.save_cases.iter()).collect()
}
pub fn save_cases() -> Vec<&'static SaveCase> {
    save_cases_in(LEVELS)
}
/// The registered save case with this name.
pub fn save_case_in(list: &[&'static Registration], name: &str) -> Option<&'static SaveCase> {
    save_cases_in(list).into_iter().find(|c| c.name == name)
}
pub fn save_case(name: &str) -> Option<&'static SaveCase> {
    save_case_in(LEVELS, name)
}
/// The visit key a registered case is saved in.
pub fn save_visit(name: &str) -> Option<&'static str> {
    save_case(name).map(|c| c.visit)
}
/// `(map, entry)` of every visit that has a registered save case, for the writer's fixtures.
pub fn save_visits_in(list: &[&'static Registration]) -> Vec<(&'static str, Option<&'static str>)> {
    let mut out = Vec::new();
    for case in save_cases_in(list) {
        let Some((map, kind)) = case.visit.split_once('$') else {
            continue;
        };
        let visit = (
            map,
            if kind == "return" {
                crate::campaign::return_entry(map)
            } else {
                None
            },
        );
        if !out.contains(&visit) {
            out.push(visit);
        }
    }
    out
}
pub fn save_visits() -> Vec<(&'static str, Option<&'static str>)> {
    save_visits_in(LEVELS)
}
/// Windowed fixtures for `--visibility-check`, after its legacy list.
pub fn visibility_in(list: &[&'static Registration]) -> Vec<&'static VisibilityFixture> {
    list.iter().flat_map(|r| r.visibility.iter()).collect()
}
pub fn visibility() -> Vec<&'static VisibilityFixture> {
    visibility_in(LEVELS)
}
/// The registered check a command-line flag names.
pub fn find_check_in(list: &[&'static Registration], flag: &str) -> Option<&'static Check> {
    list.iter()
        .flat_map(|r| r.checks.iter())
        .find(|c| c.flag == flag)
}
pub fn find_check(flag: &str) -> Option<&'static Check> {
    find_check_in(LEVELS, flag)
}
/// `--help` lines for every registered check.
pub fn help_in(list: &[&'static Registration]) -> Vec<String> {
    list.iter()
        .flat_map(|r| r.checks.iter())
        .map(|c| format!("  {:<34} {}", c.flag, c.help))
        .collect()
}
pub fn help() -> Vec<String> {
    help_in(LEVELS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::LevelController;
    use std::any::Any;

    struct Probe;
    impl LevelController for Probe {
        fn id(&self) -> &'static str {
            "probe"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    fn load(_: &mut Assets, _: &Bsp, _: &str, _: Option<&str>) -> Result<Box<dyn LevelController>> {
        Ok(Box::new(Probe))
    }
    fn nothing(_: &mut Assets) -> Result<()> {
        Ok(())
    }
    fn probe_beat_owner(_: &str, e: &Entity) -> bool {
        e.get("targetname").is_some_and(|n| n == "probe_mover")
    }
    fn probe_npc(name: &str, _: &str) -> bool {
        name == "probe_guard"
    }
    static FIRST: Registration = Registration {
        id: "garden2",
        applies: |m, e| first_visit(m, e, "garden2"),
        load,
        art: None,
        owns_submodel: probe_beat_owner,
        owns_npc: probe_npc,
        target_base: Some(7_300_000),
        story_beats: &[
            Beat {
                map: "garden2",
                event: "one",
                script: "s",
                thread: "a",
                source_lines: 1,
                calls: crate::story::registry::Calls::Linear,
            },
            Beat {
                map: "garden2",
                event: "one",
                script: "s",
                thread: "b",
                source_lines: 1,
                calls: crate::story::registry::Calls::Linear,
            },
            Beat {
                map: "garden2",
                event: "two",
                script: "s",
                thread: "c",
                source_lines: 1,
                calls: crate::story::registry::Calls::Linear,
            },
        ],
        checks: &[Check {
            flag: "--garden2-check",
            help: "probe",
            run: Run::Headless(nothing),
        }],
        save_cases: &[SaveCase {
            name: "garden2-open",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        }],
        visibility: &[],
    };
    /// The wforest shape: one map, two registrations keyed on the entry.
    static WOODS: Registration = Registration {
        id: "wforest",
        applies: |m, e| first_visit(m, e, "wforest"),
        target_base: Some(7_800_000),
        ..probe_like("wforest")
    };
    static WOODS_RETURN: Registration = Registration {
        id: "wforest-return",
        applies: |m, e| return_visit(m, e, "wforest"),
        target_base: Some(8_700_000),
        ..probe_like("wforest-return")
    };
    static WOODS_CASES: Registration = Registration {
        id: "wforest-return",
        applies: |m, e| return_visit(m, e, "wforest"),
        target_base: Some(8_700_000),
        save_cases: &[
            SaveCase {
                name: "wforest-return-open",
                visit: "wforest$return",
                stage: None,
                behavior: None,
            },
            SaveCase {
                name: "wforest-return-shut",
                visit: "wforest$return",
                stage: None,
                behavior: None,
            },
        ],
        ..probe_like("wforest-return")
    };
    const fn probe_like(id: &'static str) -> Registration {
        Registration {
            id,
            applies: |_, _| false,
            load,
            art: None,
            owns_submodel: |_, _| false,
            owns_npc: |_, _| false,
            target_base: None,
            story_beats: &[],
            checks: &[],
            save_cases: &[],
            visibility: &[],
        }
    }

    #[test]
    fn an_empty_table_has_no_effects() {
        let empty = &[];
        assert!(for_map_in(empty, "garden1").next().is_none());
        assert!(find_check_in(empty, "--garden1-check").is_none());
        assert!(
            help_in(empty).is_empty()
                && save_cases_in(empty).is_empty()
                && visibility_in(empty).is_empty()
        );
        assert!(story_expectations_in(empty).is_empty() && beats_in(empty, "garden1").is_empty());
        assert!(!owns_submodel_in(empty, "garden1", &Entity::new()));
    }
    #[test]
    fn the_entry_selects_between_two_registrations_of_one_map() {
        let list: &[&'static Registration] = &[&WOODS, &WOODS_RETURN];
        let ids = |entry| {
            serving_in(list, "wforest", entry)
                .map(|r| r.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(None), ["wforest"]);
        assert_eq!(ids(Some("wforest_start1")), ["wforest"]);
        assert_eq!(ids(Some("wforest_start2")), ["wforest-return"]);
        assert!(ids(Some("elsewhere")).contains(&"wforest"));
        // Submodels have no entry, so both registrations are consulted for the map.
        assert_eq!(for_map_in(list, "wforest").count(), 2);
        assert_eq!(for_map_in(list, "wchess1").count(), 0);
    }
    #[test]
    fn at_most_one_registration_serves_each_campaign_visit() {
        let list: &[&'static Registration] = &[&FIRST, &WOODS, &WOODS_RETURN];
        for &(map, entry) in crate::campaign::route() {
            assert!(serving_in(list, map, entry).count() <= 1, "{map} {entry:?}");
        }
        assert_eq!(serving_in(list, "garden2", None).count(), 1);
    }
    #[test]
    fn save_cases_name_the_visit_and_entrance_the_writer_must_stage() {
        let list: &[&'static Registration] = &[&FIRST, &WOODS_CASES];
        assert_eq!(
            save_case_in(list, "wforest-return-shut").map(|c| c.visit),
            Some("wforest$return")
        );
        assert!(save_case_in(list, "school").is_none());
        // Each visit is listed once, the return visit with its named entrance.
        assert_eq!(
            save_visits_in(list),
            [("garden2", None), ("wforest", Some("wforest_start2"))]
        );
        assert!(save_visits_in(&[]).is_empty());
    }
    #[test]
    fn aggregates_collect_beats_cases_checks_and_ownership() {
        let list: &[&'static Registration] = &[&FIRST, &WOODS];
        assert_eq!(
            beats_in(list, "garden2"),
            [("one", "s", "a"), ("one", "s", "b"), ("two", "s", "c")]
        );
        assert!(beats_in(list, "wforest").is_empty());
        // Two threads share the event "one": the reader loads one beat per event.
        assert_eq!(story_expectations_in(list), [("garden2", 2)]);
        assert_eq!(save_cases_in(list).len(), 1);
        assert_eq!(save_cases_in(list)[0].visit, "garden2$first");
        assert_eq!(
            find_check_in(list, "--garden2-check").map(|c| c.help),
            Some("probe")
        );
        assert!(find_check_in(list, "--garden2-route-check").is_none());
        assert_eq!(help_in(list).len(), 1);
        let mover = Entity::from([("targetname".into(), "probe_mover".into())]);
        assert!(owns_submodel_in(list, "garden2", &mover));
        assert!(!owns_submodel_in(list, "garden3", &mover));
        assert!(owns_npc_in(list, "garden2", None, "probe_guard", "m"));
        assert!(!owns_npc_in(list, "garden2", None, "someone_else", "m"));
        assert!(!owns_npc_in(list, "garden3", None, "probe_guard", "m"));
    }
}

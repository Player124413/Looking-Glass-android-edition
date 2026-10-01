//! `--campaign-graph-check` (F3 item 8): the static exit graph of all 39 visits, headless.
//!
//! Every visit must lead to the next `ROUTE` entry, and the campaign is only playable when the
//! exit that leads there is real and gated the way the data gates it. The check compiles
//! Appendix A of `docs/CAMPAIGN_PLAN.md` into an identifier-only table (`ROWS`) and holds each row
//! against the user's data at run time: the exit volumes in the map, their destinations, the
//! verbs the map's scripts apply to them, the level changes the scripts request and the
//! functions they sit in (`script_facts`, verification only), and what the engine would fire on
//! contact in a freshly entered visit (`Interactions::exit_volumes`). It prints three lists, each
//! visit with its reason:
//!
//! 1. script-only exits without an adapter, where the level change exists only as a script
//!    request and no reviewed adapter or registered exit specification covers it;
//! 2. BSP exits that the data fires only from a scene (or lets a scene's fallback volume beside a
//!    scripted change), which must be scene-gated;
//! 3. fresh-entry enablement: the volumes the data closes at load (or never lets Alice touch)
//!    against what the engine leaves live today, each flagged as a bypass until it is gated.
//!
//! Lists 1 and 2 together must equal `EXPECTED` (the thirteen visits Appendix A names). The check
//! fails on an unreviewed exit volume, a table row the data no longer supports, a live volume that
//! leads anywhere but the next visit (unless it is one of the reviewed authored-closed volumes,
//! which list 3 flags), and a visit whose reviewed adapter leads anywhere but the next visit.
//! Nothing here quotes a script: the table holds identifiers and numbers only.
use super::script_facts::Scan;
use crate::{assets::Assets, campaign, interaction, powerups::Difficulty};
use anyhow::{bail, ensure, Result};
use std::collections::{BTreeMap, BTreeSet};

/// How the data treats an exit volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// Live from the start; a map with a physical gate keeps it out of reach.
    Open,
    /// Switched off at load and switched on by a scene or a puzzle.
    Closed,
    /// Switched off and never switched on again.
    Dead,
    /// Never touched: a script fires it from a scene.
    Fired,
    /// A volume beside a scripted level change, for a player who never triggers the scene.
    Fallback,
    /// The exit of the map's other visit.
    OtherVisit,
}

/// A `trigger_changelevel` volume of the visit's map.
struct Vol {
    /// Entity index in the map.
    index: usize,
    role: Role,
    /// The destination is the next visit.
    next: bool,
    /// The verbs the map's scripts apply to its targetname (`E` enable, `D` disable, `F` fire, `B` bind).
    verbs: &'static str,
    /// Named in the plan among the authored-disabled volumes that must be gated at load.
    named: bool,
}

/// A scripted trigger that starts the scene which ends the visit.
struct Scene {
    index: usize,
    thread: &'static str,
    verbs: &'static str,
    role: Role,
    named: bool,
}

struct Row {
    n: usize,
    vols: &'static [Vol],
    scenes: &'static [Scene],
    /// The functions of the map's scripts that request a level change to the next visit.
    script: &'static [&'static str],
    /// What the row says in one line (Appendix A's mechanism, paraphrased).
    why: &'static str,
}

const fn vol(index: usize, role: Role, next: bool, verbs: &'static str) -> Vol {
    Vol {
        index,
        role,
        next,
        verbs,
        named: false,
    }
}
const fn named(index: usize, role: Role, next: bool, verbs: &'static str) -> Vol {
    Vol {
        index,
        role,
        next,
        verbs,
        named: true,
    }
}
const fn scene(index: usize, thread: &'static str, verbs: &'static str, role: Role) -> Scene {
    Scene {
        index,
        thread,
        verbs,
        role,
        named: false,
    }
}
const fn named_scene(index: usize, thread: &'static str, verbs: &'static str, role: Role) -> Scene {
    Scene {
        index,
        thread,
        verbs,
        role,
        named: true,
    }
}

use Role::{Closed, Dead, Fallback, Fired, Open, OtherVisit};

/// Appendix A (verified 2026-09-28), one row per visit in `ROUTE` order.
static ROWS: [Row; 39] = [
    Row {
        n: 1,
        vols: &[vol(41, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "the volume is ungated; the exit waits for the departure dialogue",
    },
    Row {
        n: 2,
        vols: &[],
        scenes: &[],
        script: &[
            "AL2_End",
            "End_Level",
            "Exit_Cinematic_Skipthread",
            "Exit_Dialog_Thread2",
        ],
        why: "a script level change after the departure dialogue and the flight",
    },
    Row {
        n: 3,
        vols: &[
            vol(41, Open, true, "B"),
            vol(40, OtherVisit, false, "BDE"),
        ],
        scenes: &[],
        script: &[],
        why: "the fortress door volume, first visit only",
    },
    Row {
        n: 4,
        vols: &[vol(43, Closed, true, "BDE")],
        scenes: &[],
        script: &[],
        why: "the last door's volume, enabled when the musical puzzle opens it",
    },
    Row {
        n: 5,
        vols: &[
            vol(40, Open, true, "BDE"),
            vol(41, OtherVisit, false, "B"),
        ],
        scenes: &[],
        script: &[],
        why: "the schoolhouse volume that moves with its door, return visit only",
    },
    Row {
        n: 6,
        vols: &[
            vol(77, Open, true, ""),
            vol(92, OtherVisit, false, ""),
        ],
        scenes: &[],
        script: &["Book_Ingredients_Exit"],
        why: "the recipe book's script exit; the volume beside it lies below the floor",
    },
    Row {
        n: 7,
        vols: &[
            vol(29, Closed, true, "DE"),
            vol(75, Dead, true, "D"),
        ],
        scenes: &[],
        script: &[],
        why: "the exit volume opens when the quest completes; the other is permanently off",
    },
    Row {
        n: 8,
        vols: &[
            vol(92, Open, true, ""),
            vol(77, OtherVisit, false, ""),
        ],
        scenes: &[],
        script: &["OS1_End", "observatory_exit_cinematic"],
        why: "the observatory's exit scene, after the potion",
    },
    Row {
        n: 9,
        vols: &[vol(115, Fired, true, "F")],
        scenes: &[scene(82, "Tears1_End_Cinematic", "", Open)],
        script: &[],
        why: "the pool's end scene fires the exit volume about five seconds after its trigger",
    },
    Row {
        n: 10,
        vols: &[named(68, Fired, true, "F")],
        scenes: &[named_scene(50, "tears2_end_cinematic", "DE", Closed)],
        script: &[],
        why: "the end scene starts when both ant guards are dead and then fires the exit volume",
    },
    Row {
        n: 11,
        vols: &[vol(36, Fired, true, "F")],
        scenes: &[],
        script: &[],
        why: "the Duchess controller defers the exit after the outro",
    },
    Row {
        n: 12,
        vols: &[],
        scenes: &[named_scene(30, "Utemple_Exit_Cinematic", "DE", Closed)],
        script: &["Utemple_Exit_Cinematic"],
        why: "the exit scene's trigger opens at the last node, and the scene requests the level change",
    },
    Row {
        n: 13,
        vols: &[vol(131, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 14,
        vols: &[vol(162, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 15,
        vols: &[vol(74, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "a volume below the end platform, reachable after the platform falls",
    },
    Row {
        n: 16,
        vols: &[named(6, Closed, true, "BDE")],
        scenes: &[],
        script: &[],
        why: "the portal volume rides the moving portal and opens when the scene commits",
    },
    Row {
        n: 17,
        vols: &[],
        scenes: &[scene(22, "Centipede1_Ambush_Cinema1", "", Open)],
        script: &["C1_End", "Centipede1_Ambush_Cinema1"],
        why: "the ambush scene requests the level change after its timer",
    },
    Row {
        n: 18,
        vols: &[named(43, Dead, true, "D")],
        scenes: &[scene(34, "Centipede2_Grow_Alice", "DE", Closed)],
        script: &["Centipede2_Grow_Alice"],
        why: "the growth scene requests the level change; the volume is permanently off",
    },
    Row {
        n: 19,
        vols: &[vol(126, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "a volume behind the chess gate",
    },
    Row {
        n: 20,
        vols: &[vol(103, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "a volume behind the castle doors",
    },
    Row {
        n: 21,
        vols: &[named(32, Closed, true, "BDE")],
        scenes: &[],
        script: &[],
        why: "the exit volume rides the moving portal and opens after the king's scene",
    },
    Row {
        n: 22,
        vols: &[],
        scenes: &[],
        script: &["ckkt_End", "cinema_king_killed_thread"],
        why: "the Red King's death scene requests the level change",
    },
    Row {
        n: 23,
        vols: &[],
        scenes: &[],
        script: &["Funhouse_Hatter_Cinema1", "H1_End"],
        why: "the scene after both Tweedles fall requests the level change",
    },
    Row {
        n: 24,
        vols: &[named(121, Closed, true, "DE")],
        scenes: &[],
        script: &[],
        why: "the exit volume opens when the clock stops",
    },
    Row {
        n: 25,
        vols: &[],
        scenes: &[scene(179, "Hatter2_Gryphon_Cinema1", "", Open)],
        script: &["HA1_End", "Hatter2_Gryphon_Cinema1"],
        why: "the Gryphon scene requests the level change once the Hatter is dead",
    },
    Row {
        n: 26,
        vols: &[vol(106, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 27,
        vols: &[],
        scenes: &[scene(4, "pickup_eye", "DE", Open)],
        script: &["pickup_eye"],
        why: "the Eye Staff pickup scene requests the level change after the survival",
    },
    Row {
        n: 28,
        vols: &[named(126, Closed, false, "")],
        scenes: &[scene(83, "Hedge_Maze_Entrance", "", Open)],
        script: &["Hedge_Maze_Entrance"],
        why: "the hedge entrance scene requests the level change once the wall is broken; the volume behind the chess gate leads back",
    },
    Row {
        n: 29,
        vols: &[vol(29, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "the volume opens the maze's end; the scene's own level change is commented out",
    },
    Row {
        n: 30,
        vols: &[vol(22, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 31,
        vols: &[vol(52, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "a volume behind the end doors",
    },
    Row {
        n: 32,
        vols: &[vol(22, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "a volume at the bottom of the tank",
    },
    Row {
        n: 33,
        vols: &[vol(101, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 34,
        vols: &[vol(17, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 35,
        vols: &[vol(33, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "a volume past the drawbridge",
    },
    Row {
        n: 36,
        vols: &[vol(7, Open, true, "")],
        scenes: &[],
        script: &[],
        why: "an ungated volume",
    },
    Row {
        n: 37,
        vols: &[vol(67, Fallback, true, "")],
        scenes: &[scene(56, "Facade_Lift", "", Open)],
        script: &["FA1_End", "Facade_Lift"],
        why: "the lift scene requests the level change; a fallback volume sits beside it",
    },
    Row {
        n: 38,
        vols: &[vol(71, Fallback, true, "")],
        scenes: &[scene(64, "Keep_Cheshire_Dead", "", Open)],
        script: &["K1_End", "Keep_Cheshire_Dialog"],
        why: "the Cheshire Cat's death scene requests the level change; a fallback volume sits beside it",
    },
    Row {
        n: 39,
        vols: &[],
        scenes: &[],
        script: &[],
        why: "the finale plays the ending film instead of loading a map",
    },
];

/// The reviewed script exits that exist as Rust today: (visit number, where, destination).
/// Registered controllers add theirs as exit specifications, which are not defined yet.
const ADAPTERS: [(usize, &str, &str); 4] = [
    (
        2,
        "pandemonium.rs: the exit latch after the departure dialogue and the 15 s flight",
        "fortress1$fortress1_start1",
    ),
    (
        6,
        "school.rs: the Book_Ingredients_Exit event",
        "skool2$skool2_start1",
    ),
    (
        8,
        "interaction.rs: the observatory exit when the return phase completes",
        "potears1$potears1_start1",
    ),
    (
        11,
        "duchess.rs: the deferred exit when the outro completes",
        "utemple",
    ),
];

/// The visits that lists 1 and 2 must name together (Appendix A, F3 item 8).
pub const EXPECTED: [usize; 13] = [9, 10, 12, 17, 18, 22, 23, 25, 27, 28, 37, 38, 39];

/// The volumes list 3 must flag (F3 item 8): (visit number, entity index).
pub const NAMED: [(usize, usize); 8] = [
    (10, 50),
    (10, 68),
    (12, 30),
    (16, 6),
    (18, 43),
    (21, 32),
    (24, 121),
    (28, 126),
];

fn dest_index(spelled: &str) -> Option<usize> {
    let (map, entry) = interaction::destination(spelled)?;
    campaign::visit_index(&map, entry.as_deref())
}

/// The visits an adapter covers.
fn adapter(n: usize) -> Option<&'static str> {
    ADAPTERS.iter().find(|a| a.0 == n).map(|a| a.1)
}

/// What one visit contributes to the three lists.
struct Verdict {
    list1: Option<String>,
    list2: Option<String>,
    list3: Vec<String>,
    /// (visit number, entity index) of each list 3 volume, live or gated.
    seen: Vec<(usize, usize)>,
    problems: Vec<String>,
    live: usize,
}

/// Hold one row against the data and classify it.
fn judge(assets: &mut Assets, row: &Row, scans: &mut BTreeMap<String, Scan>) -> Result<Verdict> {
    let i = row.n - 1;
    let (map, entry) = campaign::route()[i];
    let visit = campaign::visits()[i].clone();
    let loaded = campaign::load_visit(assets, map, entry, None, Difficulty::Normal, false)?;
    if !scans.contains_key(map) {
        let scan = Scan::of_map(assets, map)?;
        scans.insert(map.to_owned(), scan);
    }
    let scan = &scans[map];
    let volumes = loaded.interactions.exit_volumes();
    let entities = &loaded.map.entities;
    let mut v = Verdict {
        list1: None,
        list2: None,
        list3: Vec::new(),
        seen: Vec::new(),
        problems: Vec::new(),
        live: 0,
    };
    let bad = |v: &mut Verdict, text: String| v.problems.push(format!("{visit}: {text}"));

    // Every level-change volume in the map is reviewed.
    for (index, e) in entities.iter().enumerate() {
        if e.get("classname")
            .is_some_and(|c| c == "trigger_changelevel")
            && !row.vols.iter().any(|x| x.index == index)
        {
            bad(&mut v, format!("unreviewed exit volume #{index}"));
        }
    }
    let mut open_live = Vec::new();
    for x in row.vols {
        let Some(e) = entities.get(x.index) else {
            bad(&mut v, format!("exit volume #{} does not exist", x.index));
            continue;
        };
        if e.get("classname").map(String::as_str) != Some("trigger_changelevel") {
            bad(&mut v, format!("#{} is not a level-change volume", x.index));
            continue;
        }
        let target = e.get("map").map(String::as_str).unwrap_or_default();
        let leads = dest_index(target);
        let next = leads == Some(i + 1);
        if next != x.next {
            bad(
                &mut v,
                format!(
                    "#{} leads to {target}, which is {}the next visit",
                    x.index,
                    if next { "" } else { "not " }
                ),
            );
        }
        let name = e.get("targetname").map(String::as_str).unwrap_or_default();
        let verbs = if name.is_empty() {
            String::new()
        } else {
            scan.verbs(name)
        };
        if verbs != x.verbs {
            bad(
                &mut v,
                format!(
                    "#{} ({name:?}): the scripts apply {verbs:?}, the table holds {:?}",
                    x.index, x.verbs
                ),
            );
        }
        let live = volumes
            .iter()
            .find(|t| t.index == x.index)
            .is_some_and(|t| t.live);
        v.live += usize::from(live);
        if live {
            open_live.push(x.index);
        }
        if live && x.role == OtherVisit {
            bad(
                &mut v,
                format!(
                    "#{} belongs to the map's other visit and is live here",
                    x.index
                ),
            );
        }
        if live && !x.next && !matches!(x.role, Closed | Dead) {
            bad(
                &mut v,
                format!("#{} is live and leads to {target}", x.index),
            );
        }
        if matches!(x.role, Closed | Dead) || x.named {
            v.seen.push((row.n, x.index));
            v.list3.push(format!(
                "{visit} #{} {} ({}): {}",
                x.index,
                if name.is_empty() {
                    "<unnamed>".to_owned()
                } else {
                    format!("{name:?}")
                },
                match x.role {
                    Closed => "closed at load, opened later",
                    Dead => "permanently off",
                    Fired => "fired only from a scene",
                    _ => "reviewed",
                },
                if live {
                    "LIVE at fresh entry: a bypass until it is gated at load"
                } else {
                    "gated at load"
                }
            ));
        }
    }
    for s in row.scenes {
        let Some(e) = entities.get(s.index) else {
            bad(&mut v, format!("scene trigger #{} does not exist", s.index));
            continue;
        };
        let thread = e.get("thread").map(String::as_str).unwrap_or_default();
        if !e
            .get("classname")
            .is_some_and(|c| c.starts_with("trigger_"))
            || thread != s.thread
        {
            bad(
                &mut v,
                format!("#{} is not the {} trigger", s.index, s.thread),
            );
        }
        let name = e.get("targetname").map(String::as_str).unwrap_or_default();
        let verbs = if name.is_empty() {
            String::new()
        } else {
            scan.verbs(name)
        };
        if verbs != s.verbs {
            bad(
                &mut v,
                format!(
                    "#{} ({name:?}): the scripts apply {verbs:?}, the table holds {:?}",
                    s.index, s.verbs
                ),
            );
        }
        let live = volumes
            .iter()
            .find(|t| t.index == s.index)
            .is_some_and(|t| t.live);
        if s.role == Closed || s.named {
            v.seen.push((row.n, s.index));
            v.list3.push(format!(
                "{visit} #{} {name:?} (scene trigger {}, closed at load): {}",
                s.index,
                s.thread,
                if live {
                    "LIVE at fresh entry: a bypass until it is gated at load"
                } else {
                    "gated at load"
                }
            ));
        }
    }

    // The level changes the scripts request, to the next visit.
    let mut found: Vec<&str> = scan
        .calls
        .iter()
        .filter(|c| dest_index(&c.dest) == Some(i + 1))
        .map(|c| c.function.as_str())
        .collect();
    found.sort_unstable();
    found.dedup();
    let mut expected: Vec<&str> = row.script.to_vec();
    expected.sort_unstable();
    if found != expected {
        bad(
            &mut v,
            format!(
                "the scripts request the level change in {found:?}, the table holds {expected:?}"
            ),
        );
    }
    // A script may only request a level change to another visit if the table knows about it.
    let stray: BTreeSet<_> = scan
        .calls
        .iter()
        .filter(|c| dest_index(&c.dest).is_none())
        .map(|c| c.dest.as_str())
        .collect();
    if !stray.is_empty() {
        bad(
            &mut v,
            format!("scripts request a destination outside the campaign: {stray:?}"),
        );
    }

    // The reviewed adapter leads to the next visit.
    if let Some(&(_, _, dest)) = ADAPTERS.iter().find(|a| a.0 == row.n) {
        if dest_index(dest) != Some(i + 1) {
            bad(
                &mut v,
                format!("the reviewed adapter leads to {dest}, not the next visit"),
            );
        }
    }

    // The finale requests the ending film instead of a map.
    if row.n == 39 {
        if !assets.contains("video/ending.roq") {
            bad(&mut v, "the ending film is missing".into());
        }
        if !scan.functions.contains("QLair_Ending") {
            bad(&mut v, "the finale's ending function is missing".into());
        }
        if !scan.calls.is_empty() || !volumes.iter().all(|t| t.destination.is_none()) {
            bad(&mut v, "the finale requests a map".into());
        }
    }

    // The lists.
    let bsp_exit = row.vols.iter().any(|x| x.next && x.role != Dead);
    let script_exit = !expected.is_empty();
    let covered = adapter(row.n).is_some();
    let functions = expected.join(", ");
    if !covered && !bsp_exit && (script_exit || row.n == 39) {
        v.list1 = Some(if row.n == 39 {
            format!("{visit}: requests the ending film (no map); {}", row.why)
        } else {
            format!(
                "{visit}: script-only, map() in {functions}; no adapter; {}",
                row.why
            )
        });
    }
    if !covered && bsp_exit {
        let fired: Vec<usize> = row
            .vols
            .iter()
            .filter(|x| x.role == Fired)
            .map(|x| x.index)
            .collect();
        let fallback: Vec<usize> = row
            .vols
            .iter()
            .filter(|x| x.role == Fallback)
            .map(|x| x.index)
            .collect();
        if !fired.is_empty() {
            v.list2 = Some(format!(
                "{visit}: BSP exit {fired:?} fired only from a scene (script-fired); {}",
                row.why
            ));
        } else if script_exit && !fallback.is_empty() {
            v.list2 = Some(format!(
                "{visit}: fallback volume {fallback:?} beside the scene's script exit ({functions}); {}",
                row.why
            ));
        }
    }
    let open: Vec<usize> = row
        .vols
        .iter()
        .filter(|x| x.next && matches!(x.role, Open | Fallback))
        .map(|x| x.index)
        .collect();
    println!(
        "  {visit}: {} exit volume(s), live at fresh entry {open_live:?}, authored open {open:?}{}",
        row.vols.len(),
        adapter(row.n).map_or(String::new(), |a| format!("; reviewed adapter ({a})"))
    );
    Ok(v)
}

pub fn check(assets: &mut Assets) -> Result<()> {
    ensure!(
        campaign::route().len() == 39 && ROWS.iter().enumerate().all(|(i, r)| r.n == i + 1),
        "The exit table does not cover the 39 visits in order"
    );
    let mut scans = BTreeMap::new();
    let (mut list1, mut list2, mut list3) = (Vec::new(), Vec::new(), Vec::new());
    let (mut seen, mut problems, mut live) = (Vec::new(), Vec::new(), 0);
    for row in &ROWS {
        let v = judge(assets, row, &mut scans)?;
        list1.extend(v.list1.map(|t| (row.n, t)));
        list2.extend(v.list2.map(|t| (row.n, t)));
        list3.extend(v.list3);
        seen.extend(v.seen);
        problems.extend(v.problems);
        live += v.live;
    }
    println!(
        "Reviewed script exits: {} adapters in Rust, 0 registered exit specifications (F4 pending)",
        ADAPTERS.len()
    );
    println!(
        "LIST 1 script-only exits without an adapter ({}):",
        list1.len()
    );
    for (_, t) in &list1 {
        println!("  {t}");
    }
    println!(
        "LIST 2 BSP exits fired only from a scene, to be scene-gated ({}):",
        list2.len()
    );
    for (_, t) in &list2 {
        println!("  {t}");
    }
    println!(
        "LIST 3 fresh-entry enablement: authored-closed volumes against what the engine leaves live ({}):",
        list3.len()
    );
    for t in &list3 {
        println!("  {t}");
    }
    let mut union: Vec<usize> = list1.iter().chain(&list2).map(|(n, _)| *n).collect();
    union.sort_unstable();
    union.dedup();
    if union != EXPECTED {
        problems.push(format!(
            "lists 1 and 2 name visits {union:?}, Appendix A names {EXPECTED:?}"
        ));
    }
    for named in NAMED {
        if !seen.contains(&named) {
            problems.push(format!(
                "list 3 does not flag visit {} volume #{}",
                named.0, named.1
            ));
        }
    }
    for p in &problems {
        println!("FAIL {p}");
    }
    if !problems.is_empty() {
        bail!("The exit graph has {} problem(s)", problems.len());
    }
    println!(
        "PASS campaign graph: all 39 visits lead to the next visit or to the ending film; lists 1 and 2 name {} visits {union:?}; list 3 flags the {} named authored-closed volumes ({} live volumes in all)",
        union.len(),
        NAMED.len(),
        live
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_follows_the_route_and_its_expected_visits() {
        assert_eq!(ROWS.len(), campaign::route().len());
        for (i, row) in ROWS.iter().enumerate() {
            assert_eq!(row.n, i + 1);
            // Every volume marked as leading to the next visit says so consistently.
            for x in row.vols {
                assert!(x.next || matches!(x.role, OtherVisit | Closed));
            }
        }
        for (n, _, dest) in ADAPTERS {
            assert_eq!(dest_index(dest), Some(n), "adapter of visit {n}");
        }
        let mut sorted = EXPECTED;
        sorted.sort_unstable();
        assert_eq!(sorted, EXPECTED);
        // Every named volume is in its row.
        for (n, index) in NAMED {
            let row = &ROWS[n - 1];
            assert!(
                row.vols.iter().any(|x| x.index == index && x.named)
                    || row.scenes.iter().any(|s| s.index == index && s.named),
                "visit {n} #{index}"
            );
        }
    }
    #[test]
    fn destinations_normalize_like_a_save_visit() {
        assert_eq!(dest_index("fortress2"), Some(3));
        assert_eq!(dest_index("fortress1$fortress1_start2"), Some(4));
        assert_eq!(dest_index("skool1$skool1_start1"), Some(5));
        assert_eq!(dest_index("wforest$wforest_start2"), Some(27));
        assert_eq!(dest_index("utemple"), Some(11));
        assert_eq!(dest_index("nowhere$start"), None);
        assert_eq!(dest_index("bad path"), None);
    }
    #[test]
    fn a_visit_never_appears_in_both_the_adapter_list_and_list_one() {
        // The adapters cover visits 2, 6, 8 and 11; none of them is a script-only visit.
        for (n, _, _) in ADAPTERS {
            assert!(!EXPECTED.contains(&n), "visit {n}");
        }
    }
}

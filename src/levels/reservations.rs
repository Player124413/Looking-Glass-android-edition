//! Appendix F of `docs/CAMPAIGN_PLAN.md` as code constants (F1.5): the hit-ID base of every
//! visit, its rule-key and fact prefixes, its check flags, its save-case prefix and its private
//! output folder. `validate` holds a registration to them, so two visits can never share a key
//! space and a visit cannot invent a flag name.
//!
//! Hit-ID base = `6_000_000 + ROUTE index * 100_000` (ROUTE index 0 to 38). Existing ranges stay
//! as they are: school2 1,000,000; shootable switches 2,000,000; encounters 3,000,000 + index;
//! the Duchess 4,000,000; Dice summons 4,500,000; Dice Alice 5,000,000; Ice Wand walls
//! 800,000,000 + wall id. Every registry range stays below that last one.
use super::Registration;
use crate::level::HIT_RANGE;
use anyhow::{ensure, Context, Result};
use std::collections::BTreeSet;

/// The hit-ID base of ROUTE index 0.
pub const HIT_BASE: usize = 6_000_000;
/// The distance between two visits' bases (also every range's width).
pub const HIT_STRIDE: usize = HIT_RANGE;

/// The check flags of a visit whose legacy checks predate the `--<id>-*` naming
/// (Appendix E-4). Registry visits use the four derived names instead.
pub struct VisitFlags {
    pub contract: &'static [&'static str],
    pub route: &'static [&'static str],
    pub skip: &'static [&'static str],
    pub render: &'static [&'static str],
}

pub struct Reservation {
    /// The visit's index in `campaign::route()`.
    pub index: usize,
    /// The Appendix F id: the map name, or `fortress1-return`, `skool1-return`, `wforest-return`.
    pub id: &'static str,
    pub map: &'static str,
    pub entry: Option<&'static str>,
    /// Served by a typed legacy controller (F1.4a): it has no registry line.
    pub legacy: bool,
    /// Legacy visits list their existing checks; registry visits derive `--<id>-*`.
    pub flags: Option<VisitFlags>,
}

impl Reservation {
    /// The start of this visit's exact hit-ID range `[hit_base, hit_base + HIT_RANGE)`.
    pub const fn hit_base(&self) -> usize {
        HIT_BASE + self.index * HIT_STRIDE
    }
    /// Rule keys of the visit's controller start with this (`garden1/`).
    pub fn rule_prefix(&self) -> String {
        format!("{}/", self.id)
    }
    /// Fact keys of the visit's controller start with this (`garden1.`).
    pub fn fact_prefix(&self) -> String {
        format!("{}.", self.id)
    }
    /// Save cases of the visit start with this (`garden1-`); `LOOKING_GLASS_SAVE_CASE` selects them.
    pub fn save_prefix(&self) -> String {
        format!("{}-", self.id)
    }
    /// The visit's private evidence and output files (`private/garden1-`).
    pub fn private_prefix(&self) -> String {
        format!("private/{}-", self.id)
    }
    /// The visit's save identity (`garden1$first`).
    pub fn visit_key(&self) -> String {
        crate::save::visit_key(self.map, self.entry)
    }
    /// Every check flag the visit may register, in the order Appendix E-4 lists them.
    pub fn flags_list(&self) -> Vec<String> {
        match &self.flags {
            Some(f) => [f.contract, f.route, f.skip, f.render]
                .into_iter()
                .flatten()
                .map(|s| s.to_string())
                .collect(),
            None => ["check", "route-check", "skip-route-check", "render-check"]
                .map(|s| format!("--{}-{s}", self.id))
                .to_vec(),
        }
    }
}

const fn legacy(
    index: usize,
    id: &'static str,
    map: &'static str,
    flags: VisitFlags,
) -> Reservation {
    Reservation {
        index,
        id,
        map,
        entry: None,
        legacy: true,
        flags: Some(flags),
    }
}
const fn returning(
    index: usize,
    id: &'static str,
    map: &'static str,
    entry: &'static str,
    legacy: bool,
    flags: Option<VisitFlags>,
) -> Reservation {
    Reservation {
        index,
        id,
        map,
        entry: Some(entry),
        legacy,
        flags,
    }
}
const fn visit(index: usize, id: &'static str) -> Reservation {
    Reservation {
        index,
        id,
        map: id,
        entry: None,
        legacy: false,
        flags: None,
    }
}

const FORTRESS: VisitFlags = VisitFlags {
    contract: &["--fortress-cinematic-check"],
    route: &["--fortress-route-check"],
    skip: &[],
    render: &[
        "--fortress-render-check",
        "--fortress-cinematic-render-check",
    ],
};

/// Every visit in `campaign::route()` order.
pub static RESERVATIONS: [Reservation; 39] = [
    legacy(
        0,
        "gvillage",
        "gvillage",
        VisitFlags {
            contract: &[
                "--village-cinematic-check",
                "--village-machinery-check",
                "--progression-check",
            ],
            route: &["--village-route-check"],
            skip: &[],
            render: &[
                "--village-cinematic-render-check",
                "--village-machinery-render-check",
                "--progression-render-check",
            ],
        },
    ),
    legacy(
        1,
        "pandemonium",
        "pandemonium",
        VisitFlags {
            contract: &["--pandemonium-check", "--cinematic-check"],
            route: &["--pandemonium-route-check"],
            skip: &["--pandemonium-skip-route-check"],
            render: &[
                "--pandemonium-render-check",
                "--pandemonium-machinery-render-check",
            ],
        },
    ),
    legacy(2, "fortress1", "fortress1", FORTRESS),
    legacy(
        3,
        "fortress2",
        "fortress2",
        VisitFlags {
            contract: &["--beyond-check"],
            route: &["--beyond-route-check"],
            skip: &[],
            render: &["--beyond-render-check"],
        },
    ),
    returning(
        4,
        "fortress1-return",
        "fortress1",
        "fortress1_start2",
        true,
        Some(FORTRESS),
    ),
    legacy(
        5,
        "skool1",
        "skool1",
        VisitFlags {
            contract: &["--school-check", "--footing-check"],
            route: &["--school-route-check", "--school-secret-check"],
            skip: &[],
            render: &["--school-render-check", "--progression-render-check"],
        },
    ),
    legacy(
        6,
        "skool2",
        "skool2",
        VisitFlags {
            contract: &["--school2-check", "--gym-check"],
            route: &["--school2-route-check"],
            skip: &[],
            render: &["--school2-render-check"],
        },
    ),
    returning(
        7,
        "skool1-return",
        "skool1",
        "skool1_start2",
        true,
        Some(VisitFlags {
            contract: &["--school-return-check"],
            route: &["--school-return-chain-check"],
            skip: &[],
            render: &["--school-return-render-check"],
        }),
    ),
    legacy(
        8,
        "potears1",
        "potears1",
        VisitFlags {
            contract: &["--pool-check", "--ladybug-check"],
            route: &["--potears1-route-check"],
            skip: &["--potears1-skip-route-check"],
            render: &[
                "--potears1-render-check",
                "--pool-render-check",
                "--ladybug-render-check",
            ],
        },
    ),
    Reservation {
        flags: Some(VisitFlags {
            contract: &[
                "--potears2-check",
                "--potears2-transport-check",
                "--potears2-store-write-check",
                "--potears2-store-read-check",
            ],
            route: &["--potears2-route-check", "--potears2-route-native-check"],
            skip: &["--potears2-skip-route-check"],
            render: &["--potears2-render-check"],
        }),
        ..visit(9, "potears2")
    },
    legacy(
        10,
        "potears3",
        "potears3",
        VisitFlags {
            contract: &["--duchess-check"],
            route: &["--duchess-check"],
            skip: &["--duchess-check"],
            render: &["--duchess-render"],
        },
    ),
    visit(11, "utemple"),
    Reservation { flags: Some(VisitFlags {
        contract: &["--garden1-check", "--garden1-world-check", "--garden1-cast-check", "--garden1-path-check"],
        route: &["--garden1-route-check"], skip: &["--garden1-skip-route-check"],
        render: &["--garden1-render-check"],
    }), ..visit(12, "garden1") },
    visit(13, "garden2"),
    visit(14, "garden3"),
    visit(15, "garden4"),
    Reservation {
        flags: Some(VisitFlags {
            contract: &["--centipede1-check", "--centipede1-save-write", "--centipede1-save-read"],
            route: &["--centipede1-route-check"],
            skip: &["--centipede1-skip-route-check"],
            render: &["--centipede1-render-check"],
        }),
        ..visit(16, "centipede1")
    },
    Reservation {
        flags: Some(VisitFlags {
            contract: &["--centipede2-check", "--centipede2-save-write", "--centipede2-save-read"],
            route: &["--centipede2-route-check"],
            skip: &["--centipede2-skip-route-check"],
            render: &["--centipede2-render-check"],
        }),
        ..visit(17, "centipede2")
    },
    visit(18, "wforest"),
    Reservation {
        flags: Some(VisitFlags {
            contract: &["--wchess1-check", "--wchess1-floor-survey"],
            route: &["--wchess1-route-check", "--wchess1-route"],
            skip: &["--wchess1-skip-route-check", "--wchess1-route-skip-check"],
            render: &["--wchess1-render-check"],
        }),
        ..visit(19, "wchess1")
    },
    Reservation { flags: Some(VisitFlags {
        contract: &["--wchess2-check", "--wchess2-floor-survey"], route: &["--wchess2-route-check"],
        skip: &["--wchess2-skip-route-check", "--wchess2-route-skip-check"], render: &["--wchess2-render-check"],
    }), ..visit(20, "wchess2") },
    visit(21, "rchess1"),
    Reservation { flags: Some(VisitFlags {
        contract: &["--funhouse-check", "--funhouse-traversal-probe"],
        route: &["--funhouse-route-check"], skip: &["--funhouse-skip-route-check"],
        render: &["--funhouse-render-check"],
    }), ..visit(22, "funhouse") },
    Reservation {flags:Some(VisitFlags {
        contract:&["--hatter1-check", "--hatter1-traversal-check"],
        route:&["--hatter1-route-check", "--hatter1-route-native-check"], skip:&["--hatter1-skip-route-check"],
        render:&["--hatter1-render-check"],
    }), ..visit(23, "hatter1")},
    visit(24, "hatter2"),
    Reservation { flags: Some(VisitFlags {
        contract: &["--jlair1-check", "--jlair1-traversal-check"],
        route: &["--jlair1-route-check"], skip: &["--jlair1-skip-route-check"],
        render: &["--jlair1-render-check"],
    }), ..visit(25, "jlair1") },
    visit(26, "jlair2"),
    returning(
        27,
        "wforest-return",
        "wforest",
        "wforest_start2",
        false,
        None,
    ),
    Reservation {flags:Some(VisitFlags {
        contract:&["--hedge1-check", "--hedge1-save-write", "--hedge1-save-read"],
        route:&["--hedge1-route-check"], skip:&["--hedge1-skip-route-check"],
        render:&["--hedge1-render-check"],
    }), ..visit(28, "hedge1")},
    Reservation { flags: Some(VisitFlags {
        contract: &["--tower1-check", "--tower1-actors-check"],
        route: &["--tower1-route-check", "--tower1-probe"],
        skip: &["--tower1-skip-route-check"],
        render: &["--tower1-render-check"],
    }), ..visit(29, "tower1") },
    visit(30, "hedge2"),
    Reservation {flags:Some(VisitFlags {
        contract:&["--tower2-check", "--tower2-cast-check"],
        route:&["--tower2-route-check"], skip:&["--tower2-skip-route-check"],
        render:&["--tower2-render-check"],
    }), ..visit(31, "tower2")},
    visit(32, "hedge3"),
    visit(33, "tower3"),
    visit(34, "grounds1"),
    Reservation { flags: Some(VisitFlags {
        contract: &["--grounds2-check", "--grounds2-save-check"],
        route: &["--grounds2-route-check"], skip: &["--grounds2-skip-route-check"],
        render: &["--grounds2-render-check"],
    }), ..visit(35, "grounds2") },
    Reservation { flags: Some(VisitFlags {
        contract: &["--facade-check"],
        route: &["--facade-route-check", "--facade-route-native-check"],
        skip: &["--facade-skip-route-check"],
        render: &["--facade-render-check"],
    }), ..visit(36, "facade") },
    visit(37, "keep"),
    Reservation {
        flags: Some(VisitFlags {
            contract: &["--qlair-check", "--qlair-save-write", "--qlair-save-read"],
            route: &["--qlair-route-check", "--qlair-second-route-check"],
            skip: &["--qlair-skip-route-check"],
            render: &["--qlair-render-check"],
        }),
        ..visit(38, "qlair")
    },
];

pub fn by_id(id: &str) -> Option<&'static Reservation> {
    RESERVATIONS.iter().find(|r| r.id == id)
}
/// The reservation of a route visit.
pub fn by_visit(map: &str, entry: Option<&str>) -> Option<&'static Reservation> {
    let key = crate::save::visit_key(map, entry);
    RESERVATIONS.iter().find(|r| r.visit_key() == key)
}

/// Hold registrations to the reservations. Nothing here is negotiable per visit: a registration
/// serves exactly its own route visit, owns exactly its own hit range, and registers only the
/// flags, save cases and beats Appendix F and E-4 name for it.
pub fn validate(list: &[&'static Registration]) -> Result<()> {
    let mut ids = BTreeSet::new();
    let mut flags = BTreeSet::new();
    let mut cases = BTreeSet::new();
    for reg in list {
        let id = reg.id;
        let res = by_id(id).with_context(|| format!("{id} has no reservation in Appendix F"))?;
        ensure!(
            !res.legacy,
            "{id} is owned by a legacy controller and takes no registration (F1.4a)"
        );
        ensure!(ids.insert(id), "{id} is registered twice");
        for &(map, entry) in crate::campaign::route() {
            ensure!(
                (reg.applies)(map, entry)
                    == (res.map == map && res.visit_key() == crate::save::visit_key(map, entry)),
                "{id} must serve {} and no other visit (it disagrees about {map} {entry:?})",
                res.visit_key()
            );
        }
        if let Some(base) = reg.target_base {
            ensure!(
                base == res.hit_base(),
                "{id} must own hit range {} (it declares {base})",
                res.hit_base()
            );
        }
        for check in reg.checks {
            ensure!(
                res.flags_list().iter().any(|f| f == check.flag),
                "{id} may not register {}; Appendix E-4 names {:?}",
                check.flag,
                res.flags_list()
            );
            ensure!(
                flags.insert(check.flag),
                "{} is registered twice",
                check.flag
            );
        }
        for case in reg.save_cases {
            ensure!(
                case.name.starts_with(&res.save_prefix())
                    && case.name.len() > res.save_prefix().len(),
                "Save case {} of {id} must be {}<phase>",
                case.name,
                res.save_prefix()
            );
            ensure!(
                cases.insert(case.name),
                "Save case {} is registered twice",
                case.name
            );
            ensure!(
                case.visit == res.visit_key(),
                "Save case {} must be saved in {}",
                case.name,
                res.visit_key()
            );
        }
        let mut fixtures = BTreeSet::new();
        for fixture in reg.visibility {
            ensure!(
                !fixture.name.is_empty() && fixtures.insert(fixture.name),
                "{id} has an unnamed or repeated visibility fixture"
            );
        }
        for beat in reg.story_beats {
            ensure!(
                beat.map == res.map && !beat.event.is_empty(),
                "Story beat {} of {id} must belong to {}",
                beat.event,
                res.map
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assets::Assets,
        bsp::Bsp,
        level::LevelController,
        levels::{Beat, Check, Entity, Run, SaveCase, VisibilityFixture},
    };

    #[test]
    fn the_table_is_the_campaign_route_in_order() {
        let route = crate::campaign::route();
        assert_eq!(RESERVATIONS.len(), route.len());
        for (i, r) in RESERVATIONS.iter().enumerate() {
            assert_eq!(r.index, i, "{}", r.id);
            assert_eq!((r.map, r.entry), route[i], "{}", r.id);
            // The id is the map name, except the return visits.
            assert_eq!(
                r.id,
                match r.entry {
                    Some(_) => format!("{}-return", r.map),
                    None => r.map.to_owned(),
                },
                "{}",
                r.id
            );
            assert_eq!(by_id(r.id).unwrap().index, i);
            assert_eq!(by_visit(r.map, r.entry).unwrap().id, r.id);
        }
        let ids = RESERVATIONS.iter().map(|r| r.id).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 39);
    }
    #[test]
    fn exactly_the_legacy_owned_visits_take_no_registry_line() {
        let legacy = RESERVATIONS
            .iter()
            .filter(|r| r.legacy)
            .map(|r| r.id)
            .collect::<Vec<_>>();
        assert_eq!(
            legacy,
            [
                "gvillage",
                "pandemonium",
                "fortress1",
                "fortress2",
                "fortress1-return",
                "skool1",
                "skool2",
                "skool1-return",
                "potears1",
                "potears3",
            ]
        );
    }
    #[test]
    fn hit_ranges_follow_appendix_f_and_never_meet_an_existing_range() {
        let base = |id: &str| by_id(id).unwrap().hit_base();
        // The values Appendix F prints.
        assert_eq!(base("potears1"), 6_800_000);
        assert_eq!(base("potears2"), 6_900_000);
        assert_eq!(base("potears3"), 7_000_000);
        assert_eq!(base("utemple"), 7_100_000);
        assert_eq!(base("wforest"), 7_800_000);
        assert_eq!(base("wforest-return"), 8_700_000);
        assert_eq!(base("hatter2"), 8_400_000);
        assert_eq!(base("tower3"), 9_300_000);
        assert_eq!(base("qlair"), 9_800_000);
        assert_eq!(HIT_STRIDE, crate::level::HIT_RANGE);
        // Disjoint, ascending, and clear of every legacy range.
        let mut end = 0;
        for r in &RESERVATIONS {
            assert!(r.hit_base() >= end, "{} overlaps its predecessor", r.id);
            end = r.hit_base() + HIT_STRIDE;
        }
        let first = RESERVATIONS[0].hit_base();
        assert_eq!(first, 6_000_000);
        for legacy in [
            crate::school2::ENEMY_BASE,
            crate::interaction::SHOT_BASE,
            crate::duchess::ID,
            crate::dice::SUMMON,
            crate::dice::ALICE,
        ] {
            assert!(legacy < first, "legacy id {legacy}");
        }
        // Encounter ids are 3,000,000 plus an actor index; even a million actors stay below.
        assert!(crate::encounters::BASE + 1_000_000 <= first);
        // Ice Wand walls are 800,000,000 plus a wall id (weapons/ice.rs WALL_BASE).
        assert!(end <= 800_000_000);
    }
    #[test]
    fn key_spaces_never_collide_with_the_legacy_ones_or_each_other() {
        let all = || RESERVATIONS.iter();
        // Legacy rule keys start with these words (interaction.rs configure_events).
        for legacy in [
            "actor/",
            "door/",
            "use/",
            "relay/",
            "trigger/",
            "pand/",
            "dialogue/",
            "ladybug/",
            "beyond/",
            "fortress/",
        ] {
            assert!(
                all().all(|r| !r.rule_prefix().starts_with(legacy)),
                "{legacy}"
            );
        }
        // Legacy fact keys (the puzzle flags and counters the legacy controllers declare).
        for legacy in [
            "story.theatre_finished",
            "gym.lever_uses",
            "quest.final_dialogue",
        ] {
            assert!(
                all().all(|r| !legacy.starts_with(&r.fact_prefix())),
                "{legacy}"
            );
        }
        // Legacy save cases and the prefixes their per-case logic tests (save_check.rs).
        for legacy in [
            "guard-cut",
            "item-",
            "movement-",
            "duchess-",
            "return",
            "village",
            "ladybug-",
            "pand-",
            "dice-",
        ] {
            assert!(
                all().all(|r| !r.save_prefix().starts_with(legacy)),
                "{legacy}"
            );
        }
        for case in crate::save_check::CASES {
            assert!(
                all().all(|r| !case.starts_with(&r.save_prefix())),
                "legacy case {case}"
            );
        }
        // Rule and fact prefixes are unique; save prefixes are unique except wforest's, whose
        // first-visit prefix is also the start of the return visit's cases (both are documented).
        let registry = || RESERVATIONS.iter().filter(|r| !r.legacy);
        for (a, b) in registry().flat_map(|a| registry().map(move |b| (a, b))) {
            if a.id == b.id {
                continue;
            }
            assert!(!a.rule_prefix().starts_with(&b.rule_prefix()));
            assert!(!a.fact_prefix().starts_with(&b.fact_prefix()));
            if a.save_prefix().starts_with(&b.save_prefix()) {
                assert_eq!((a.id, b.id), ("wforest-return", "wforest"));
            }
        }
        let wforest = by_id("wforest").unwrap();
        assert_eq!(wforest.rule_prefix(), "wforest/");
        assert_eq!(wforest.fact_prefix(), "wforest.");
        assert_eq!(wforest.private_prefix(), "private/wforest-");
        assert_eq!(by_id("garden1").unwrap().save_prefix(), "garden1-");
    }
    #[test]
    fn flags_follow_appendix_e4() {
        let flags = |id: &str| by_id(id).unwrap().flags_list();
        assert_eq!(
            flags("garden3"),
            [
                "--garden3-check",
                "--garden3-route-check",
                "--garden3-skip-route-check",
                "--garden3-render-check"
            ]
        );
        assert_eq!(
            flags("wforest-return"),
            [
                "--wforest-return-check",
                "--wforest-return-route-check",
                "--wforest-return-skip-route-check",
                "--wforest-return-render-check"
            ]
        );
        assert_eq!(flags("fortress1"), flags("fortress1-return"));
        assert!(flags("gvillage").contains(&"--village-route-check".to_owned()));
        assert!(flags("potears3").contains(&"--duchess-render".to_owned()));
        // A flag is registered once, except where Appendix E-4 gives two visits the same one.
        let mut seen = BTreeSet::new();
        for r in RESERVATIONS.iter().filter(|r| !r.legacy) {
            for f in r.flags_list() {
                assert!(f.starts_with("--") && seen.insert(f.clone()), "{f}");
            }
        }
    }

    struct Probe;
    impl LevelController for Probe {
        fn id(&self) -> &'static str {
            "garden2"
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
    fn load(_: &mut Assets, _: &Bsp, _: &str, _: Option<&str>) -> Result<Box<dyn LevelController>> {
        Ok(Box::new(Probe))
    }
    fn nothing(_: &mut Assets) -> Result<()> {
        Ok(())
    }
    fn fixture(a: &mut Assets) -> crate::levels::BoxFuture<'_> {
        Box::pin(async move {
            nothing(a)?;
            Ok(())
        })
    }
    const fn base(id: &'static str) -> Registration {
        Registration {
            id,
            applies: |_, _| false,
            load,
            art: None,
            owns_submodel: |_: &str, _: &Entity| false,
            owns_npc: |_, _| false,
            target_base: None,
            story_beats: &[],
            checks: &[],
            save_cases: &[],
            visibility: &[],
        }
    }
    static GOOD: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2"),
        target_base: Some(7_300_000),
        checks: &[
            Check {
                flag: "--garden2-check",
                help: "",
                run: Run::Headless(nothing),
            },
            Check {
                flag: "--garden2-render-check",
                help: "",
                run: Run::Windowed(fixture),
            },
        ],
        save_cases: &[SaveCase {
            name: "garden2-open",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        }],
        visibility: &[VisibilityFixture {
            name: "garden2-replay",
            run: fixture,
        }],
        story_beats: &[Beat {
            map: "garden2",
            event: "e",
            script: "s",
            thread: "t",
            source_lines: 1,
            calls: crate::story::registry::Calls::Linear,
        }],
        ..base("garden2")
    };
    static ANOTHER: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden3"),
        ..base("garden3")
    };
    static UNKNOWN: Registration = base("nowhere");
    static LEGACY: Registration = base("skool2");
    static WRONG_VISIT: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2") || m == "garden3",
        ..base("garden2")
    };
    static WRONG_BASE: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2"),
        target_base: Some(7_400_000),
        ..base("garden2")
    };
    static WRONG_FLAG: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2"),
        checks: &[Check {
            flag: "--garden3-check",
            help: "",
            run: Run::Headless(nothing),
        }],
        ..base("garden2")
    };
    static WRONG_CASE: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2"),
        save_cases: &[SaveCase {
            name: "garden3-open",
            visit: "garden2$first",
            stage: None,
            behavior: None,
        }],
        ..base("garden2")
    };
    static WRONG_CASE_VISIT: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2"),
        save_cases: &[SaveCase {
            name: "garden2-open",
            visit: "garden2$return",
            stage: None,
            behavior: None,
        }],
        ..base("garden2")
    };
    static WRONG_BEAT: Registration = Registration {
        applies: |m, e| crate::levels::first_visit(m, e, "garden2"),
        story_beats: &[Beat {
            map: "garden3",
            event: "e",
            script: "s",
            thread: "t",
            source_lines: 1,
            calls: crate::story::registry::Calls::Linear,
        }],
        ..base("garden2")
    };

    #[test]
    fn validation_accepts_the_shipped_table_and_a_registration_that_keeps_to_its_reservation() {
        validate(crate::levels::LEVELS).unwrap();
        validate(&[&GOOD, &ANOTHER]).unwrap();
    }
    #[test]
    fn validation_refuses_a_registration_that_leaves_its_reservation() {
        for (name, list) in [
            ("unknown id", vec![&UNKNOWN]),
            ("legacy-owned visit", vec![&LEGACY]),
            ("serves another visit", vec![&WRONG_VISIT]),
            ("someone else's hit range", vec![&WRONG_BASE]),
            ("someone else's flag", vec![&WRONG_FLAG]),
            ("someone else's save case", vec![&WRONG_CASE]),
            ("save case in the wrong visit", vec![&WRONG_CASE_VISIT]),
            ("someone else's story beat", vec![&WRONG_BEAT]),
            ("registered twice", vec![&GOOD, &GOOD]),
        ] {
            assert!(validate(&list).is_err(), "{name} was accepted");
        }
    }
}

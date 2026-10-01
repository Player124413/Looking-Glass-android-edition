//! Royal Rage: flight, ground combat, Gryphon farewell and drawbridge.
use crate::jabberwock::{self, Encounter, Kind};
use crate::story::registry::BeatSpec;
pub static REGISTRATION: super::Registration = super::Registration {
    id: "grounds1",
    applies: |m, e| super::first_visit(m, e, "grounds1"),
    load: |a, m, _, _| Ok(Box::new(Encounter::load(a, m, Kind::Grounds)?)),
    art: Some(|a, _, o| jabberwock::art(a, o)),
    owns_submodel: |_, e| {
        e.get("classname")
            .is_some_and(|s| s == "script_object" || s == "func_door")
            && e.get("model").is_some_and(|s| s.starts_with('*'))
    },
    owns_npc: |n, m| {
        matches!(m, "c_jabberwock" | "c_gryphon") || matches!(n, "cat" | "gnome" | "dead_guard")
    },
    target_base: Some(9_400_000),
    story_beats: &[
        BeatSpec::linear("grounds1", "CatThread", "../grounds1", "CatThread", 1),
        BeatSpec::linear(
            "grounds1",
            "grounds1_StartCine",
            "grounds1_cine",
            "grounds1_StartCine",
            1,
        ),
        BeatSpec {
            map: "grounds1",
            event: "grounds1_EndCine",
            script: "grounds1_cine",
            thread: "grounds1_EndCine",
            source_lines: 5,
            calls: crate::story::registry::Calls::Gated {
                key: "grounds1.survived",
                indices: &[0, 1, 2, 3, 4],
            },
        },
    ],
    checks: &[
        super::Check {
            flag: "--grounds1-check",
            help: "Verify Royal Rage phases, defeat, drawbridge and saves.",
            run: super::Run::Headless(|a| jabberwock::check::check(a, Kind::Grounds)),
        },
        super::Check {
            flag: "--grounds1-route-check",
            help: "Play the Royal Rage route.",
            run: super::Run::Headless(|a| jabberwock::check::route(a, Kind::Grounds)),
        },
        super::Check {
            flag: "--grounds1-render-check",
            help: "Capture Royal Rage scenes and combat.",
            run: super::Run::Windowed(|a| jabberwock::check::render(a, Kind::Grounds)),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

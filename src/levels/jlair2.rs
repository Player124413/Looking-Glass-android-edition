//! Jabberwock's Lair: survival and the Eye Staff.
use crate::jabberwock::{self, Encounter, Kind};
use crate::story::registry::BeatSpec;
pub static REGISTRATION: super::Registration = super::Registration {
    id: "jlair2",
    applies: |m, e| super::first_visit(m, e, "jlair2"),
    load: |a, m, _, _| Ok(Box::new(Encounter::load(a, m, Kind::Lair)?)),
    art: Some(|a, _, o| jabberwock::art(a, o)),
    owns_submodel: |_, _| false,
    owns_npc: |_, m| matches!(m, "c_jabberwock" | "c_gryphon" | "altar_eyestaff_eye"),
    target_base: Some(8_600_000),
    story_beats: &[
        BeatSpec::linear(
            "jlair2",
            "JLair2_Cinema1",
            "jlair2_cinematics",
            "JLair2_Cinema1",
            8,
        ),
        BeatSpec::linear(
            "jlair2",
            "JLair2_Dead_Jabber",
            "jlair2_cinematics",
            "JLair2_Dead_Jabber",
            1,
        ),
    ],
    checks: &[
        super::Check {
            flag: "--jlair2-check",
            help: "Verify Jabberwock survival, waves, reward and saves.",
            run: super::Run::Headless(|a| jabberwock::check::check(a, Kind::Lair)),
        },
        super::Check {
            flag: "--jlair2-route-check",
            help: "Play the Jabberwock Lair route.",
            run: super::Run::Headless(|a| jabberwock::check::route(a, Kind::Lair)),
        },
        super::Check {
            flag: "--jlair2-render-check",
            help: "Capture Jabberwock Lair scenes and combat.",
            run: super::Run::Windowed(|a| jabberwock::check::render(a, Kind::Lair)),
        },
    ],
    save_cases: &[],
    visibility: &[],
};

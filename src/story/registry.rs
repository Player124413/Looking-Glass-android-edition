//! Reviewed dialogue sources, in saved line order. No source-language execution.
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub enum Calls {
    /// Retain the already shipped interpretation and saved indices.
    Legacy,
    /// Reject branching instead of silently flattening conditional speech.
    Linear,
    /// An owner-selected branch: explicit source call indices and a named gate.
    /// Register alternative branches as separate stable events, never reinterpret
    /// the index of an already saved sequence when a quest flag changes.
    Gated {
        key: &'static str,
        indices: &'static [usize],
    },
}
#[derive(Clone, Copy)]
pub struct BeatSpec {
    pub map: &'static str,
    pub event: &'static str,
    pub script: &'static str,
    pub thread: &'static str,
    /// Number of dialogue calls in the reviewed source function, before selection.
    pub source_lines: usize,
    pub calls: Calls,
}
impl BeatSpec {
    pub const fn linear(
        map: &'static str,
        event: &'static str,
        script: &'static str,
        thread: &'static str,
        source_lines: usize,
    ) -> Self {
        Self {
            map,
            event,
            script,
            thread,
            source_lines,
            calls: Calls::Linear,
        }
    }
    pub fn line_count(&self) -> usize {
        match self.calls {
            Calls::Gated { indices, .. } => indices.len(),
            _ => self.source_lines,
        }
    }
    pub fn gate(&self) -> Option<&'static str> {
        match self.calls {
            Calls::Gated { key, .. } => Some(key),
            _ => None,
        }
    }
}

pub static LEGACY: &[BeatSpec] = &[
    BeatSpec::linear("skool2", "trigger_cat_rage", "../skool2", "trigger_cat_rage", 1),
    BeatSpec {
        map: "potears1",
        event: "Tears1_Turtle_Cinema1",
        script: "potears1_cinematics",
        thread: "Tears1_TurtleDialog1",
        source_lines: 14,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "potears3",
        event: "potears3_dialog",
        script: "../potears3",
        thread: "potears3_dialog",
        source_lines: 6,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "potears3",
        event: "potears3_end_dialog",
        script: "../potears3",
        thread: "potears3_end_dialog",
        source_lines: 4,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool1",
        event: "Cat_Glass_Dialog",
        script: "skool1_cinematics",
        thread: "Cat_Glass_Dialog",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool1",
        event: "mallet_cat",
        script: "../skool1",
        thread: "mallet_cat",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool1",
        event: "Theatre_Cinematic",
        script: "skool1_cinematics",
        thread: "Play_Dialog",
        source_lines: 2,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool1",
        event: "shelf_cinematic",
        script: "skool1_cinematics",
        thread: "shelf_cinematic",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool1",
        event: "book_cinematic",
        script: "skool1_cinematics",
        thread: "book_cinematic",
        source_lines: 3,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool1",
        event: "Book_Ingredients_Exit",
        script: "skool1_cinematics",
        thread: "Book_Ingredients_Exit",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool2",
        event: "Old_Gnome_SpiceDrops",
        script: "skool2_cinematics",
        thread: "Old_Gnome_SpiceDropsDialog",
        source_lines: 4,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool2",
        event: "Skool2_LastGnome_Cinema",
        script: "skool2_cinematics",
        thread: "Old_Gnome_LastDialog",
        source_lines: 2,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool2",
        event: "Old_Gnome_Mushroom",
        script: "skool2_cinematics",
        thread: "Old_Gnome_MushroomDialog",
        source_lines: 3,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool2",
        event: "mallet_cat",
        script: "../skool2",
        thread: "mallet_cat",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "skool2",
        event: "dice_cat",
        script: "../skool2",
        thread: "dice_cat",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress2",
        event: "Fortress2_Rage_Pickup",
        script: "../fortress2",
        thread: "Fortress2_Rage_Pickup",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress2",
        event: "cat_dialog_easy",
        script: "../fortress2",
        thread: "cat_dialog_easy",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress2",
        event: "cat_dialog_hard",
        script: "../fortress2",
        thread: "cat_dialog_hard",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress1",
        event: "Fortress1_Start",
        script: "fortress1_cinematics",
        thread: "Fortress1_Cinematic_Airship_Talk",
        source_lines: 3,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress1",
        event: "Fortress1_Start",
        script: "fortress1_cinematics",
        thread: "Fortress1_Cinema4",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress1",
        event: "push_cat_trigger",
        script: "../fortress1",
        thread: "push_cat_trigger",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "fortress1",
        event: "skool_cat",
        script: "../fortress1",
        thread: "skool_cat",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "pandemonium",
        event: "Rope_Cat_Thread",
        script: "../pandemonium",
        thread: "Rope_Cat_Thread",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "pandemonium",
        event: "Elder_Gnome1_Warn_Thread",
        script: "../pandemonium",
        thread: "Pandemonium_Persue_Dialog",
        source_lines: 9,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "pandemonium",
        event: "cards_cat",
        script: "../pandemonium",
        thread: "cards_cat",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "pandemonium",
        event: "Pand_End_Ship",
        script: "../pandemonium",
        thread: "Exit_Dialog_Thread1",
        source_lines: 2,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "pandemonium",
        event: "Pand_End_Ship",
        script: "../pandemonium",
        thread: "Exit_Dialog_Thread2",
        source_lines: 2,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "entry",
        script: "gvillage_rabbit",
        thread: "Gvillage_RabbitHole_Start",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "entry",
        script: "gvillage_rabbit",
        thread: "Gvillage_Start_Dialog",
        source_lines: 5,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "knife_cat",
        script: "gvillage_rabbit",
        thread: "knife_cat",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "Torchgnome1_Dialog",
        script: "gvillage_rabbit",
        thread: "Torchgnome1_Dialog",
        source_lines: 3,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "Torchgnome2_Dialog",
        script: "gvillage_rabbit",
        thread: "Torchgnome2_Dialog",
        source_lines: 4,
        calls: Calls::Legacy,
    },
    // Append the authored cutaway without renumbering the four saved Gnome lines.
    BeatSpec::linear("gvillage", "Torchgnome2_Dialog", "../gvillage", "Essence_Cat_Dialog_Thread", 1),
    BeatSpec {
        map: "gvillage",
        event: "Torchgnome3_Dialog_part2",
        script: "gvillage_rabbit",
        thread: "Torchgnome3_Dialog_part2",
        source_lines: 9,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "Torchgnome4_Dialog",
        script: "gvillage_rabbit",
        thread: "Torchgnome4_Dialog",
        source_lines: 3,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "Climb_Cat_Thread",
        script: "../gvillage",
        thread: "Climb_Cat_Thread",
        source_lines: 1,
        calls: Calls::Legacy,
    },
    BeatSpec {
        map: "gvillage",
        event: "Exit_Cat_Thread",
        script: "../gvillage",
        thread: "Exit_Cat_Thread",
        source_lines: 1,
        calls: Calls::Legacy,
    },
];
pub fn for_map(map: &str) -> Vec<&'static BeatSpec> {
    LEGACY
        .iter()
        .chain(
            crate::levels::for_map_in(crate::levels::LEVELS, map)
                .flat_map(|r| r.story_beats.iter()),
        )
        .filter(|b| b.map == map)
        .collect()
}
pub fn maps() -> BTreeSet<&'static str> {
    LEGACY
        .iter()
        .chain(crate::levels::LEVELS.iter().flat_map(|r| r.story_beats))
        .map(|b| b.map)
        .collect()
}
pub fn events(map: &str) -> BTreeSet<&'static str> {
    for_map(map).iter().map(|b| b.event).collect()
}

/// Reviewed character animation reference. A missing source clip may use only an
/// explicit fallback; validation still records the unresolved original identifier.
pub struct AnimationRef {
    pub model: &'static str,
    pub requested: &'static str,
    pub fallback: Option<&'static str>,
}
impl AnimationRef {
    pub fn resolve<'a>(&'a self, def: &crate::skeletal::Definition) -> anyhow::Result<&'a str> {
        use anyhow::Context;
        if def.animations.contains_key(self.requested) {
            return Ok(self.requested);
        }
        self.fallback
            .filter(|clip| def.animations.contains_key(*clip))
            .with_context(|| format!("Missing animation {}/{}", self.model, self.requested))
    }
}
pub fn validate_animations(
    assets: &mut crate::assets::Assets,
    refs: &[AnimationRef],
) -> anyhow::Result<()> {
    use crate::skeletal::{Animation, Definition, Skeleton};
    for r in refs {
        let def = Definition::load(assets, &format!("models/{}.tik", r.model))?;
        let clip = r.resolve(&def)?;
        let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
        Animation::parse(
            &assets.read(&format!("{}/{}", def.path, def.animations[clip]))?,
            skeleton.bones.len(),
        )?;
        if clip != r.requested {
            println!(
                "Reviewed animation fallback: {}/{} -> {clip}",
                r.model, r.requested
            );
        }
    }
    Ok(())
}

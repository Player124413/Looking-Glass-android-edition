//! The reviewed additions to School2's legacy encounter ownership (M0 item 8).
//! Existing quest actors keep their School2 slots and hit identities.
use super::School2;
use crate::{encounters::Encounters, event::Runtime, school2_quest::Stage};

pub fn owns(index: usize, model: &str, name: &str) -> bool {
    model == "cardguard_diamond.tik"
        || (model == "cardguard_club.tik"
            && matches!(name, "spawn_floor2_guard1" | "spawn_floor3_guard1"))
        || (index == 202 && model == "c_boojum.tik" && name == "dice_boojum")
}

impl School2 {
    /// Derive encounter handoffs from the quest's durable commits. Repeated ticks,
    /// skips and migrated saves cannot create another actor or revive a dead one.
    pub fn sync_encounters(&self, enemies: &mut Encounters, events: &mut Runtime) {
        if self.dice_guards {
            activate(enemies, events, "dice_boojum");
        }
        let grown = matches!(
            self.quest.stage,
            Stage::Lollipop
                | Stage::FinalDialogue
                | Stage::Mixing
                | Stage::Rewards
                | Stage::Complete
        );
        if self.quest.stage == Stage::Growing || grown {
            activate(enemies, events, "cgd_spawn1");
        }
        if grown {
            // The authored growth ending removes this corridor actor, including
            // the skip ending. Removal is not a kill and grants no death loot.
            if let Some(id) = enemies
                .actors
                .iter()
                .zip(&enemies.identities)
                .find_map(|(a, id)| (a.name == "kill_diamond").then_some(*id))
            {
                events
                    .set_enabled(id, false)
                    .expect("registered School2 actor");
            }
        }
        enemies.sync_enabled(events);
    }
}

fn activate(enemies: &mut Encounters, events: &mut Runtime, name: &str) {
    if enemies.activated.contains(name) {
        return;
    }
    enemies.activate(name);
    for (a, id) in enemies.actors.iter().zip(&enemies.identities) {
        if a.name == name {
            events.import_consumed(&format!("actor/{}", id.0));
        }
    }
}

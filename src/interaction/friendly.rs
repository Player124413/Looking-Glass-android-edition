//! Explicit actor-to-conversation bindings. See docs/FRIENDLY_NPCS.md for the
//! reviewed allow-list and automatic-only exclusions; never infer talk from a model.
use super::*;
pub mod check;
use crate::{npc::Npcs, story::Story};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseOwner {
    None,
    Dialogue,
    SharedRope,
    World,
    Talk,
    Traversal,
}
/// Input and HUD use the same priority. A key edge belongs to exactly one owner.
pub fn use_owner(
    busy: bool,
    scripted: bool,
    shared_rope: bool,
    world_rope: bool,
    talk: bool,
    world: bool,
) -> UseOwner {
    if busy {
        UseOwner::Dialogue
    } else if scripted {
        UseOwner::None
    } else if shared_rope {
        UseOwner::SharedRope
    } else if world_rope {
        UseOwner::World
    } else if talk {
        UseOwner::Talk
    } else if world {
        UseOwner::World
    } else {
        UseOwner::Traversal
    }
}

#[derive(Clone, Copy)]
struct Binding {
    map: &'static str,
    actor: &'static str,
    thread: &'static str,
    label: &'static str,
}
// First-use actions select the shipped trigger by thread and use its existing
// registry gate and consumption. No new event keys, IDs or quest mutations.
const BINDINGS: &[Binding] = &[
    Binding {
        map: "gvillage",
        actor: "torchgnome2",
        thread: "Torchgnome1_Dialog",
        label: "Gnome",
    },
    Binding {
        map: "gvillage",
        actor: "torchgnome1",
        thread: "Torchgnome2_Dialog",
        label: "Gnome",
    },
    Binding {
        map: "gvillage",
        actor: "torchgnome3",
        thread: "Torchgnome3_Dialog_part2",
        label: "Gnome",
    },
    Binding {
        map: "gvillage",
        actor: "torchgnome4",
        thread: "Torchgnome4_Dialog",
        label: "Gnome",
    },
    Binding {
        map: "skool2",
        actor: "old_gnome_1",
        thread: crate::school2::cinema::MUSHROOM,
        label: "Elder Gnome",
    },
    Binding {
        map: "skool2",
        actor: "old_gnome_2",
        thread: crate::school2::cinema::FINAL,
        label: "Elder Gnome",
    },
    Binding {
        map: "potears1",
        actor: "turtle_talk",
        thread: crate::pool::TALK,
        label: "Mock Turtle",
    },
];
#[derive(Clone, Copy)]
pub struct Conversation {
    binding: Binding,
    // None is a completed village conversation's speech-only repeat.
    trigger: Option<Id>,
}
impl Conversation {
    pub fn actor(&self) -> &'static str {
        self.binding.actor
    }
    pub fn prompt(&self) -> String {
        let action = if self.trigger.is_some() {
            "Talk"
        } else {
            "Talk again"
        };
        format!("E  {action} to {}", self.binding.label)
    }
}
impl Interactions {
    fn talk_activation(&self, binding: Binding, story: &Story) -> Option<Option<Id>> {
        if self.map_name != binding.map
            || self.scripted()
            || story.busy()
            || story.pending_exit_map().is_some()
        {
            return None;
        }
        if story.has_seen(binding.thread) {
            return self
                .village
                .as_ref()
                .filter(|v| v.cinema.state.done.contains(binding.thread))
                .map(|_| None);
        }
        self.triggers
            .iter()
            .find(|t| {
                matches!(&t.kind, TriggerKind::Script(n) if n == binding.thread)
                    && !(t.once && t.fired)
                    && t.health <= 0.
                    && t.cooldown <= 0.
                    && self.presentation.enabled(&t.name)
                    && self
                        .event_world
                        .allowed(&Event::Entity(t.id, Input::Activate), &self.event_facts())
            })
            .map(|t| Some(t.id))
    }
    fn controller_talk_target(&self, binding: Binding) -> Option<Vec3> {
        match binding.map {
            "skool2" => self.school2.as_ref()?.conversation_target(binding.thread),
            "potears1" => self.pool.as_ref()?.conversation_target(),
            _ => None,
        }
    }
    fn resolve_conversation(
        &self,
        world: &World,
        eye: Vec3,
        aim: Vec3,
        story: &Story,
        placed: impl Fn(&str) -> Option<Vec3>,
    ) -> Option<Conversation> {
        BINDINGS
            .iter()
            .filter_map(|&binding| {
                let trigger = self.talk_activation(binding, story)?;
                let target = if binding.map == "gvillage" {
                    placed(binding.actor)?
                } else {
                    self.controller_talk_target(binding)?
                };
                crate::npc::talk_reachable(world, eye, aim, target).then_some((
                    target.distance_squared(eye),
                    Conversation { binding, trigger },
                ))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, c)| c)
    }
    pub fn conversation(
        &self,
        world: &World,
        eye: Vec3,
        aim: Vec3,
        npcs: &Npcs,
        story: &Story,
    ) -> Option<Conversation> {
        self.resolve_conversation(world, eye, aim, story, |name| {
            npcs.talk_target(name, world, eye, aim)
        })
    }
    /// Revalidate the token against quest state before consuming it. Rewards,
    /// staging and later contact all go through the original trigger's owner.
    pub fn start_conversation(&mut self, c: Conversation, story: &mut Story) -> Option<Events> {
        if self.talk_activation(c.binding, story)? != c.trigger {
            return None;
        }
        if c.binding.map != "gvillage" && self.controller_talk_target(c.binding).is_none() {
            return None;
        }
        let events = if let Some(id) = c.trigger {
            self.dispatch(Event::Entity(id, Input::Activate))
        } else {
            if !story.repeat_conversation(c.binding.thread) {
                return None;
            }
            Events::default()
        };
        println!(
            "Conversation {} {}: {}",
            c.binding.map,
            c.binding.actor,
            if c.trigger.is_some() {
                "authored trigger"
            } else {
                "speech repeat"
            }
        );
        Some(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_key_has_one_owner_and_dialogue_wins_over_nearby_rope() {
        assert_eq!(
            use_owner(true, true, true, true, true, true),
            UseOwner::Dialogue
        );
        assert_eq!(
            use_owner(false, true, true, true, true, true),
            UseOwner::None
        );
        assert_eq!(
            use_owner(false, false, true, true, true, true),
            UseOwner::SharedRope
        );
        assert_eq!(
            use_owner(false, false, false, true, true, true),
            UseOwner::World
        );
        assert_eq!(
            use_owner(false, false, false, false, true, true),
            UseOwner::Talk
        );
        assert_eq!(
            use_owner(false, false, false, false, false, true),
            UseOwner::World
        );
        assert_eq!(
            use_owner(false, false, false, false, false, false),
            UseOwner::Traversal
        );
    }
}

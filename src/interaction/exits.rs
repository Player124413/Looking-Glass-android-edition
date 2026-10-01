//! A read-only view of a visit's exit volumes for `--campaign-graph-check` (F3b).
//!
//! The check needs to know, for a freshly entered visit, which level-change volumes exist and
//! which of them the engine would fire if Alice touched them now. The answer is the question the
//! trigger loop asks itself (`Interactions::triggers`): the volume's event rule must accept an
//! activation with the current facts, which folds in its enable flag and every controller's gate.
use super::{Event, Input, Interactions, TriggerKind};
use crate::entity::Id;

/// One trigger volume that ends a visit or starts the scene that does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitVolume {
    /// Entity index in the map's entity list.
    pub index: usize,
    pub name: String,
    /// `Some` for a `trigger_changelevel`: where it leads, as (map, named entrance).
    pub destination: Option<(String, Option<String>)>,
    /// The thread of a scripted trigger (`trigger_once`/`trigger_multiple`), empty otherwise.
    pub thread: String,
    /// The engine would fire this volume on contact now.
    pub live: bool,
}

impl Interactions {
    pub fn transition_failed(&mut self, exit: &(String, Option<String>)) {
        for slot in &mut self.levels {
            slot.ctl.transition_failed(exit);
        }
        if crate::pool::cinema::ENDING.end.exit.unwrap().matches(exit) {
            if let Some(s) = self
                .pool
                .as_mut()
                .and_then(|p| p.state.cinema.ending.as_mut())
            {
                s.exit.failed();
            }
        }
    }
    /// Every level-change volume and scripted trigger of the visit, with whether it is live now.
    pub fn exit_volumes(&self) -> Vec<ExitVolume> {
        let facts = self.event_facts();
        self.triggers
            .iter()
            .filter_map(|t| {
                let (destination, thread) = match &t.kind {
                    TriggerKind::Exit(map, entry) => (Some((map.clone(), entry.clone())), ""),
                    TriggerKind::Script(thread) => (None, thread.as_str()),
                    _ => return None,
                };
                let Id(index) = t.id;
                let scene_owned = self.pool.is_some()
                    && destination
                        .as_ref()
                        .is_some_and(|d| crate::pool::cinema::ENDING.end.exit.unwrap().matches(d));
                let live = !scene_owned
                    && t.health <= 0.
                    && !(t.once && t.fired)
                    && self
                        .event_world
                        .allowed(&Event::Entity(t.id, Input::Activate), &facts);
                Some(ExitVolume {
                    index,
                    name: t.name.clone(),
                    destination,
                    thread: thread.to_owned(),
                    live,
                })
            })
            .collect()
    }
}

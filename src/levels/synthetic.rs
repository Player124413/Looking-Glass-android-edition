//! A synthetic registration, controller and map for the unit tests of the generic upgrade and
//! the save plumbing (F2). Nothing here is a visit and `LEVELS` never lists it: the tests hand it
//! to the code under test as a slice, or inject the controller into an `Interactions`.
//!
//! The map (built on the interaction tests' box fixture) has, besides the door:
//!
//! | entity | name | role |
//! | --- | --- | --- |
//! | 1 | `scene_trigger` | a once-only script trigger the old build left pending |
//! | 2 | `ambush_trigger` | the same, but the controller declares its rule already run |
//! | 3 | `committed_trigger` | a once-only trigger the old build handled completely |
//! | 4 | `gated_trigger` | a repeatable trigger the controller gates |
//! | 5 | `lever` | a receiver only the controller activates |
//! | 6 | `relay1` | a plain relay whose target is the lever, so its rule gains a send |
use super::{
    first_visit,
    state::{self, State, Visit},
    Registration, SaveCase,
};
use crate::{
    assets::Assets,
    bsp::Bsp,
    entity::{Id, Registry},
    event::{Action, Condition, Event, Facts, Input, Rule},
    interaction::{Events, Interactions},
    level::{LevelController, RuleContext, Slot, TriggerClass, TriggerInfo, Upgrade},
    movement::Player,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::any::Any;

/// The controller's saved state, in the shape `levels::state` documents.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    pub version: u8,
    pub returning: bool,
    pub clock: f32,
    pub open: bool,
    pub opened_at: Option<f32>,
    #[serde(default)]
    pub later: u32,
}
impl State for Saved {
    const VERSION: u8 = 2;
    fn version(&self) -> u8 {
        self.version
    }
    fn validate(&self, visit: Visit) -> Result<()> {
        ensure!(self.returning == visit.returning, "Saved visit differs");
        state::clock("mover", self.clock, 600.)?;
        ensure!(self.open == self.opened_at.is_some(), "Inconsistent lever");
        if let Some(at) = self.opened_at {
            state::clock("lever", at, self.clock)?;
        }
        Ok(())
    }
}

pub struct Synthetic {
    pub visit: Visit,
    pub state: Saved,
    /// What `upgrade()` declares.
    pub plan: Upgrade,
    /// `validate_player` refuses every position.
    pub reject_beyond_x: Option<f32>,
}
impl Synthetic {
    pub fn new(visit: Visit) -> Self {
        Self {
            visit,
            state: Saved {
                version: Saved::VERSION,
                returning: visit.returning,
                clock: 0.,
                open: false,
                opened_at: None,
                later: 0,
            },
            plan: Upgrade::default(),
            reject_beyond_x: None,
        }
    }
    fn trigger<'a>(ctx: &'a RuleContext<'_>, name: &str) -> Option<&'a TriggerInfo<'a>> {
        ctx.triggers
            .iter()
            .find(|t| t.name == name && t.class == TriggerClass::Script)
    }
}
impl LevelController for Synthetic {
    fn id(&self) -> &'static str {
        "garden1"
    }
    fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("garden1.derived", true);
        f
    }
    fn initial(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("garden1.open", false);
        f.count("garden1.hits", 0);
        f
    }
    fn gate(&self, t: &TriggerInfo<'_>) -> Option<Condition> {
        (t.thread == "gated_thread").then(|| Condition::flag("garden1.derived"))
    }
    fn receivers(&self, registry: &Registry) -> Vec<Id> {
        registry.named("lever").to_vec()
    }
    fn rules(&self, ctx: &RuleContext<'_>) -> Vec<Rule> {
        let rule = |key: &str, id: Id, once: bool, actions: Vec<Action>| Rule {
            key: key.into(),
            event: Event::Entity(id, Input::Activate),
            condition: Condition::Always,
            once,
            cooldown: 0.,
            actions,
        };
        let mut rules = Vec::new();
        for &lever in ctx.registry.named("lever") {
            rules.push(rule(
                "garden1/lever",
                lever,
                false,
                vec![
                    Action::Flag("garden1.open".into(), true),
                    Action::Count("garden1.hits".into(), 1),
                ],
            ));
        }
        for (key, name) in [
            ("garden1/scene", "scene_trigger"),
            ("garden1/ambush", "ambush_trigger"),
            ("garden1/reward", "committed_trigger"),
        ] {
            if let Some(t) = Self::trigger(ctx, name) {
                rules.push(rule(
                    key,
                    t.id,
                    true,
                    vec![Action::Count("garden1.hits".into(), 1)],
                ));
            }
        }
        rules
    }
    fn event(&mut self, thread: &str) -> Option<Events> {
        let message = match thread {
            "scene_thread" => "scene",
            "ambush_thread" => "ambush",
            "gated_thread" => "gated",
            _ => return None,
        };
        Some(Events {
            message: Some(message.into()),
            ..Default::default()
        })
    }
    fn snapshot(&self) -> serde_json::Value {
        state::save(&self.state)
    }
    fn restore(&mut self, saved: &serde_json::Value, _: &Bsp) -> Result<()> {
        self.state = state::load(saved, self.visit)?;
        Ok(())
    }
    fn validate_player(&self, player: &Player) -> Result<()> {
        ensure!(
            self.reject_beyond_x.is_none_or(|x| player.feet.x <= x),
            "The controller rejects the player"
        );
        Ok(())
    }
    fn upgrade(&self) -> Upgrade {
        self.plan.clone()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

fn controller(i: &mut Interactions) -> Result<&mut Synthetic> {
    i.levels
        .first_mut()
        .and_then(|s| s.ctl.downcast_mut::<Synthetic>())
        .context("The synthetic controller does not serve this visit")
}
/// The staging of the `garden1-open` case: the lever thrown at time 1.
fn stage_open(i: &mut Interactions, _: &Bsp) -> Result<()> {
    let c = controller(i)?;
    c.state.clock = 2.;
    c.state.open = true;
    c.state.opened_at = Some(1.);
    Ok(())
}
/// The behaviour check of `garden1-open`: the lever is still thrown after a reload.
fn check_open(
    i: &mut Interactions,
    _: &mut crate::inventory::Stats,
    _: &mut crate::story::Story,
) -> Result<()> {
    let c = controller(i)?;
    ensure!(c.state.open, "The lever reset on reload");
    Ok(())
}
fn nowhere(_: &mut Assets, _: &Bsp, _: &str, _: Option<&str>) -> Result<Box<dyn LevelController>> {
    unreachable!("the unit tests inject the controller")
}
pub static REGISTRATION: Registration = Registration {
    id: "garden1",
    applies: |map, entry| first_visit(map, entry, "garden1"),
    load: nowhere,
    art: None,
    owns_submodel: |_, _| false,
    owns_npc: |name, _| name == "synthetic_guard",
    target_base: Some(7_200_000),
    story_beats: &[],
    checks: &[],
    save_cases: &[
        SaveCase {
            name: "garden1-open",
            visit: "garden1$first",
            stage: Some(stage_open),
            behavior: Some(check_open),
        },
        SaveCase {
            name: "garden1-shut",
            visit: "garden1$first",
            stage: None,
            behavior: None,
        },
    ],
    visibility: &[],
};

/// The synthetic map. Entity ids are listed in the module documentation.
pub fn map() -> Bsp {
    let mut map = crate::interaction::tests::fixture();
    let trigger = |class: &str, name: &str, thread: &str, origin: &str| {
        super::Entity::from([
            ("classname".into(), class.into()),
            ("targetname".into(), name.into()),
            ("model".into(), "*1".into()),
            ("origin".into(), origin.into()),
            ("thread".into(), thread.into()),
        ])
    };
    map.entities.extend([
        trigger("trigger_once", "scene_trigger", "scene_thread", "0 100 0"),
        trigger(
            "trigger_once",
            "ambush_trigger",
            "ambush_thread",
            "200 100 0",
        ),
        trigger(
            "trigger_once",
            "committed_trigger",
            "committed_thread",
            "-200 100 0",
        ),
        trigger(
            "trigger_multiple",
            "gated_trigger",
            "gated_thread",
            "0 300 0",
        ),
        super::Entity::from([
            ("classname".into(), "script_object".into()),
            ("targetname".into(), "lever".into()),
        ]),
        super::Entity::from([
            ("classname".into(), "trigger_relay".into()),
            ("targetname".into(), "relay1".into()),
            ("target".into(), "lever".into()),
        ]),
        super::Entity::from([
            ("classname".into(), "info_player_start".into()),
            ("origin".into(), "0 0 0".into()),
        ]),
    ]);
    map
}
/// The entity id of a named entity of `map()`.
pub fn id(map: &Bsp, name: &str) -> Id {
    Id(map
        .entities
        .iter()
        .position(|e| e.get("targetname").is_some_and(|n| n == name))
        .unwrap_or_else(|| panic!("no entity {name}")))
}
/// A visit served by the synthetic controller: `Interactions::load` plus the injected slot.
pub fn served(map: &Bsp, controller: Synthetic) -> Interactions {
    let mut i = Interactions::load(map).unwrap();
    i.probe_levels(
        map,
        vec![Slot {
            reg: &REGISTRATION,
            ctl: Box::new(controller),
        }],
    )
    .unwrap();
    i
}
/// A first visit's controller.
pub fn first() -> Synthetic {
    Synthetic::new(Visit { returning: false })
}

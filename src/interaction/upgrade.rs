//! The generic upgrade of a visit whose save predates its registered controller (F2).
//!
//! A registration applies to a visit, but the snapshot has no key for it in `levels`: the save
//! was made by a build that served the visit without a controller. `Interactions::restore`
//! calls into this module at two points, so the upgrade needs no legacy chain of its own:
//!
//! 1. `begin_level_upgrade`, before any state is restored: the controllers are held back and the
//!    event program is rebuilt **without** them. That is the program the save was made with
//!    (`Interactions::load` plus every legacy controller exactly as shipped, encounters
//!    included, because it is this visit's own program minus the registry's contribution), and
//!    the shared runtime (step 2) and every legacy `upgrade_*` path then restore against it as
//!    they always did.
//! 2. `finish_level_upgrade`, after all of that: the controllers return, and
//!    * step 3: the current program (built by `set_entry`, with the controllers) is extended
//!      from the restored one by `Runtime::extend_registered_from`, with the trigger rules the
//!      controllers gate (derived from `gate()`, plus `Upgrade::gated`) as the only condition
//!      changes;
//!    * step 4: Script triggers the old build left pending (`reported`) are rearmed, together
//!      with those named in `Upgrade::rearm`: `fired = reported = inside = false`,
//!      `cooldown = 0`, `import_unhandled`;
//!    * step 5: `import_consumed` for the controller's rules keyed on a trigger the old build
//!      already handled, and for every rule in `Upgrade::consumed`, so ambushes and rewards
//!      never replay.
//!
//!    Steps 6 and 7 need the collision world and the NPC models, so they live where those are:
//!    `Interactions::levels_respawn` decides the respawn and `save::settle` performs it, and
//!    `npc::Snapshot::adopt` regenerates the cast in `save::Level::upgrade`.
//!
//! The controllers themselves are not restored: they were loaded fresh, which is exactly the
//! state of a visit that has just been entered, and the next snapshot writes it.
use super::{Interactions, Snapshot, TriggerKind};
use crate::{
    bsp::Bsp,
    collision::World,
    entity::{Id, Registry},
    event::{Condition, Event, Facts, Input, Runtime},
    level::{Respawn, Slot, Upgrade},
    levels::{self, Registration},
    movement::Player,
};
use anyhow::{ensure, Result};
use std::collections::{BTreeMap, BTreeSet};

/// A visit held back while its save is restored against the program it was made with.
pub(super) struct Upgrading {
    /// The controllers that serve the visit, in registration order.
    held: Vec<Slot>,
    /// The event runtime of the current program (with the controllers), built by `set_entry`.
    current: Runtime,
}

impl Interactions {
    pub(super) fn begin_keep_upgrade(&mut self, saved: &BTreeMap<String, serde_json::Value>, map: &Bsp) -> Result<Option<Runtime>> {
        if !self.levels.iter().any(|s|s.reg.id=="keep") || saved.get("keep").and_then(|s|s.get("version")).and_then(|v|v.as_u64())!=Some(1) { return Ok(None); }
        let current=std::mem::replace(&mut self.event_world,Runtime::new(Registry::new(&[]),vec![],Facts::default(),Facts::default())?);
        let held=std::mem::take(&mut self.levels);
        let result=self.configure_events(map);self.levels=held;result?;Ok(Some(current))
    }
    pub(super) fn finish_keep_upgrade(&mut self, current: Runtime, shared: &crate::event::Snapshot) -> Result<()> {
        self.event_world.restore(shared)?;
        let previous=std::mem::replace(&mut self.event_world,current);
        let receivers=self.level_receivers(&self.event_world.registry).into_iter().collect();
        self.event_world.extend_repeating_from(&previous,&self.gated_triggers(),&receivers,&[32,33,34,468,469,470,471,472,473].into_iter().map(|id|format!("trigger/{id}")).collect::<Vec<_>>())?;
        for t in &mut self.triggers {
            // Every puzzle trigger was unsupported in arrival-only builds. Reopen its history,
            // including health spent on old portrait contacts; keep unrelated hazards intact.
            if matches!(t.id.0,1|7|11|29..=36|64|71|466..=473|542..=546) {
                t.fired=false;t.reported=false;t.inside=false;t.cooldown=0.;
                if matches!(t.id.0,32..=34|468..=473){t.health=1.;}
                self.event_world.import_unhandled(&format!("trigger/{}",t.id.0));
            }
        }Ok(())
    }
    /// C3 already registered this visit, but deliberately closed both scene gates.
    /// Rebuild that exact program before importing its usage into task 15.
    pub(super) fn begin_bill_upgrade(
        &mut self,
        saved: &BTreeMap<String, serde_json::Value>,
        map: &Bsp,
    ) -> Result<Option<Runtime>> {
        if saved
            .get("potears2")
            .and_then(|s| s.get("version"))
            .and_then(|v| v.as_u64())
            != Some(1)
        {
            return Ok(None);
        }
        let Some(index) = self.levels.iter().position(|s| s.reg.id == "potears2") else {
            return Ok(None);
        };
        let current = std::mem::replace(
            &mut self.event_world,
            Runtime::new(
                Registry::new(&[]),
                vec![],
                Facts::default(),
                Facts::default(),
            )?,
        );
        self.levels[index]
            .ctl
            .downcast_mut::<crate::levels::potears2::PoolTwo>()
            .unwrap()
            .legacy_program = true;
        let result = self.configure_events(map);
        self.levels[index]
            .ctl
            .downcast_mut::<crate::levels::potears2::PoolTwo>()
            .unwrap()
            .legacy_program = false;
        result?;
        Ok(Some(current))
    }
    /// Steps 1 and 2: hold the controllers back when the save has no state for them, and
    /// rebuild the program the save was made with. `None` when nothing needs upgrading.
    pub(super) fn begin_level_upgrade(
        &mut self,
        saved: &BTreeMap<String, serde_json::Value>,
        map: &Bsp,
    ) -> Result<Option<Upgrading>> {
        if self.levels.is_empty() || self.levels.iter().all(|s| saved.contains_key(s.reg.id)) {
            return Ok(None);
        }
        // Some controller has no saved state, so none may: a save that names controllers this
        // visit lacks, or some but not all of its own, is refused like any other mismatch.
        ensure!(
            saved.is_empty(),
            "Saved level state does not match this visit"
        );
        let held = std::mem::take(&mut self.levels);
        let placeholder = Runtime::new(
            Registry::new(&[]),
            vec![],
            Facts::default(),
            Facts::default(),
        )?;
        let current = std::mem::replace(&mut self.event_world, placeholder);
        self.configure_events(map)?;
        Ok(Some(Upgrading { held, current }))
    }
    /// The trigger rules whose condition the controllers change: every trigger a controller
    /// gates, so a gate cannot be forgotten and make the whole save unloadable.
    pub(super) fn gated_triggers(&self) -> Vec<String> {
        self.triggers
            .iter()
            .filter(|t| !matches!(self.level_gate(t), Condition::Always))
            .map(|t| format!("trigger/{}", t.id.0))
            .collect()
    }
    /// Steps 3 to 5, once the shared runtime and every legacy controller are restored.
    pub(super) fn finish_level_upgrade(&mut self, upgrading: Upgrading) -> Result<()> {
        let Upgrading { held, current } = upgrading;
        // What has been restored so far is the previous program's runtime.
        let previous = std::mem::replace(&mut self.event_world, current);
        self.levels = held;
        let history = self.triggers.iter().filter(|t| t.fired || t.reported).map(|t| t.id).collect::<Vec<_>>();
        for s in &mut self.levels {
            s.ctl.upgraded();
            s.ctl.upgrade_triggers(&history);
        }
        let plans: Vec<Upgrade> = self.levels.iter().map(|s| s.ctl.upgrade()).collect();

        // Step 3: the current program, extended from the restored one.
        let mut gates = self.gated_triggers();
        for plan in &plans {
            for key in &plan.gated {
                ensure!(
                    key.strip_prefix("trigger/")
                        .is_some_and(|n| n.parse::<usize>().is_ok()),
                    "An upgrade may re-gate only trigger rules, not {key}"
                );
                gates.push(key.clone());
            }
        }
        let receivers: BTreeSet<Id> = self
            .level_receivers(&self.event_world.registry)
            .into_iter()
            .collect();
        self.event_world
            .extend_registered_from(&previous, &gates, &receivers)?;

        // Step 4: rearm what the old build left pending, so the new consequences can run.
        let rearm: BTreeSet<&str> = plans
            .iter()
            .flat_map(|p| p.rearm.iter().map(String::as_str))
            .collect();
        let mut rearmed = BTreeSet::new();
        for t in &mut self.triggers {
            let TriggerKind::Script(thread) = &t.kind else {
                continue;
            };
            if !(t.reported || rearm.contains(thread.as_str())) {
                continue;
            }
            t.fired = false;
            t.reported = false;
            t.inside = false;
            t.cooldown = 0.;
            self.event_world
                .import_unhandled(&format!("trigger/{}", t.id.0));
            rearmed.insert(t.id);
        }

        // Step 5: what the old build already ran never runs again.
        let mine = |key: &str| {
            self.levels.iter().any(|s| {
                key.strip_prefix(s.reg.id)
                    .is_some_and(|rest| rest.starts_with('/'))
            })
        };
        let mut consumed = Vec::new();
        for t in self
            .triggers
            .iter()
            .filter(|t| t.fired && !rearmed.contains(&t.id))
        {
            for input in [Input::Activate, Input::Touch, Input::Use, Input::Shot] {
                consumed.extend(
                    self.event_world
                        .rule_keys(&Event::Entity(t.id, input))
                        .into_iter()
                        .filter(|key| mine(key)),
                );
            }
        }
        let known = self.event_world.snapshot().usage;
        for (slot, plan) in self.levels.iter().zip(&plans) {
            for key in &plan.consumed {
                ensure!(
                    known.contains_key(key)
                        && key
                            .strip_prefix(slot.reg.id)
                            .is_some_and(|rest| rest.starts_with('/')),
                    "{} declares {key} as already run, but it is not one of its rules",
                    slot.reg.id
                );
                consumed.push(key.clone());
            }
        }
        for key in consumed {
            self.event_world.import_consumed(&key);
        }
        Ok(())
    }
    /// Step 6: whether Alice restarts at the entrance after an upgrade. She does when a
    /// controller asks for it, when a controller rejects the saved position, or when her body is
    /// not clear there (free flight excepted, as everywhere else in a restore).
    pub fn levels_respawn(&self, player: &Player, world: &World, flying: bool) -> bool {
        self.levels.iter().any(|s| {
            s.ctl.upgrade().respawn == Respawn::Always || s.ctl.validate_player(player).is_err()
        }) || (!flying && !world.body_clear(player.feet))
    }
}

impl Snapshot {
    /// Whether a registered controller serves this visit that the save has no state for: the
    /// save predates the registration and needs the generic upgrade.
    pub fn registry_upgrade(&self, map: &str, entry: Option<&str>) -> bool {
        self.registry_upgrade_in(levels::LEVELS, map, entry)
    }
    /// `registry_upgrade` against an explicit registration list.
    pub fn registry_upgrade_in(
        &self,
        list: &[&'static Registration],
        map: &str,
        entry: Option<&str>,
    ) -> bool {
        levels::serving_in(list, map, entry).any(|r| {
            !self.levels.contains_key(r.id)
                || (r.id == "potears2" && self.levels[r.id]["version"].as_u64().is_some_and(|v| v < 3))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::Value,
        interaction::{Events, Trigger},
        levels::{
            state::Visit,
            synthetic::{self, Saved, Synthetic},
        },
    };
    use macroquad::prelude::*;

    fn json(i: &Interactions) -> serde_json::Value {
        serde_json::to_value(i.snapshot()).unwrap()
    }
    fn signature(i: &Interactions) -> String {
        json(i)["shared"]["signature"].as_str().unwrap().to_owned()
    }
    fn usage(i: &Interactions, key: &str) -> crate::event::Usage {
        i.event_world.snapshot().usage[key].clone()
    }
    fn fact(i: &Interactions, key: &str) -> Value {
        i.event_world.snapshot().facts.0[key].clone()
    }
    fn trigger<'a>(i: &'a Interactions, name: &str) -> &'a Trigger {
        i.triggers.iter().find(|t| t.name == name).unwrap()
    }
    /// Walk over the trigger volume at x (the volumes are 64 wide, 6 deep at y = 100).
    fn walk(i: &mut Interactions, x: f32) -> Events {
        i.triggers(0.1, vec3(x, 40., 0.), vec3(x, 100., 0.))
    }
    const SCENE: f32 = 32.;
    const AMBUSH: f32 = 232.;
    const COMMITTED: f32 = -168.;

    /// The old build: no controller. The player crossed the scene and ambush triggers, which
    /// were pending scripts, and the committed trigger was handled (not pending) and spent.
    fn old_visit(map: &Bsp) -> Interactions {
        let mut old = Interactions::load(map).unwrap();
        for x in [SCENE, AMBUSH] {
            let event = walk(&mut old, x);
            assert_eq!(
                event.message.as_deref(),
                Some("This level event is not playable yet")
            );
            old.reset_contacts();
        }
        let id = synthetic::id(map, "committed_trigger");
        old.dispatch(Event::Entity(id, Input::Activate));
        let t = old.triggers.iter_mut().find(|t| t.id == id).unwrap();
        assert!(
            t.fired && t.reported,
            "the old build reported it as pending"
        );
        t.reported = false; // ... but pretend it handled this one (a dialogue or a sky sequence)
        old.touched = 5;
        old
    }
    fn declared() -> Synthetic {
        let mut c = synthetic::first();
        c.plan.consumed = vec!["garden1/ambush".into()];
        c
    }

    #[test]
    fn an_old_save_joins_a_registered_visit_and_nothing_replays() {
        let map = synthetic::map();
        let old = old_visit(&map);
        let saved = old.snapshot();
        assert!(json(&old).get("levels").is_none());
        assert!(saved.registry_upgrade_in(&[&synthetic::REGISTRATION], "garden1", None));

        let mut new = synthetic::served(&map, declared());
        new.restore(&saved, &map).unwrap();

        // The program is the current one, and the controller's fresh state is now saved.
        let fresh = synthetic::served(&map, declared());
        assert_eq!(signature(&new), signature(&fresh));
        assert_ne!(signature(&new), signature(&old));
        assert_eq!(json(&new)["levels"]["garden1"]["version"], 2);
        assert!(!json(&new)["levels"]["garden1"]["open"].as_bool().unwrap());
        assert!(json(&new).get("levels").is_some());
        assert!(!new
            .snapshot()
            .registry_upgrade_in(&[&synthetic::REGISTRATION], "garden1", None));

        // Old state is kept, not reset.
        assert_eq!(new.touched, 5);
        assert!(trigger(&new, "committed_trigger").fired);
        assert_eq!(usage(&new, "trigger/3").count, 1);

        // Step 4: the two triggers the old build left pending are rearmed ...
        for name in ["scene_trigger", "ambush_trigger"] {
            let t = trigger(&new, name);
            assert!(
                !t.fired && !t.reported && !t.inside && t.cooldown == 0.,
                "{name}"
            );
        }
        let (scene, ambush) = (
            synthetic::id(&map, "scene_trigger").0,
            synthetic::id(&map, "ambush_trigger").0,
        );
        for id in [scene, ambush] {
            assert_eq!(usage(&new, &format!("trigger/{id}")).count, 0);
        }
        // ... and the trigger the old build handled is not.
        assert_eq!(usage(&new, "garden1/reward").count, 1);

        // Step 5: the controller's own rules are fresh, except the ones that count as run.
        assert_eq!(usage(&new, "garden1/scene").count, 0);
        assert_eq!(usage(&new, "garden1/ambush").count, 1, "declared consumed");
        assert_eq!(fact(&new, "garden1.hits"), Value::Count(0));
        assert_eq!(fact(&new, "garden1.open"), Value::Flag(false));

        // Behaviour: the scene now runs, once; the ambush trigger runs but its rule does not
        // replay; the committed trigger stays quiet.
        let event = walk(&mut new, SCENE);
        assert_eq!(event.message.as_deref(), Some("scene"));
        assert_eq!(fact(&new, "garden1.hits"), Value::Count(1));
        new.reset_contacts();
        assert!(walk(&mut new, SCENE).message.is_none(), "spent again");
        let event = walk(&mut new, AMBUSH);
        assert_eq!(event.message.as_deref(), Some("ambush"));
        assert_eq!(
            fact(&new, "garden1.hits"),
            Value::Count(1),
            "no replayed ambush"
        );
        new.reset_contacts();
        assert!(walk(&mut new, COMMITTED).message.is_none());
        assert_eq!(
            fact(&new, "garden1.hits"),
            Value::Count(1),
            "no replayed reward"
        );

        // The new receiver is wired through the relay that already existed.
        let relay = synthetic::id(&map, "relay1");
        new.dispatch(Event::Entity(relay, Input::Activate));
        assert_eq!(fact(&new, "garden1.open"), Value::Flag(true));
        assert_eq!(usage(&new, "garden1/lever").count, 1);
    }
    #[test]
    fn the_program_the_save_was_made_with_is_the_visits_program_without_the_registry() {
        let map = synthetic::map();
        let plain = Interactions::load(&map).unwrap();
        let mut served = synthetic::served(&map, declared());
        assert_ne!(signature(&served), signature(&plain));
        let held = served
            .begin_level_upgrade(&BTreeMap::new(), &map)
            .unwrap()
            .expect("the controller has no saved state, so it is held back");
        // Holding the controllers back leaves exactly `Interactions::load`'s program ...
        assert!(served.levels.is_empty());
        assert_eq!(signature(&served), signature(&plain));
        // ... and finishing puts the controller and its program back.
        served.finish_level_upgrade(held).unwrap();
        assert_eq!(served.levels.len(), 1);
        assert_eq!(
            signature(&served),
            signature(&synthetic::served(&map, declared()))
        );
        // Nothing is held for a visit whose controllers all have state, or that has none.
        let saved: BTreeMap<_, _> = [("garden1".to_owned(), serde_json::Value::Null)].into();
        assert!(served.begin_level_upgrade(&saved, &map).unwrap().is_none());
        let mut bare = Interactions::load(&map).unwrap();
        assert!(bare
            .begin_level_upgrade(&BTreeMap::new(), &map)
            .unwrap()
            .is_none());
        // A partial or foreign map is refused, and nothing is taken.
        let foreign: BTreeMap<_, _> = [("garden2".to_owned(), serde_json::Value::Null)].into();
        assert!(served.begin_level_upgrade(&foreign, &map).is_err());
        assert_eq!(served.levels.len(), 1);
    }
    #[test]
    fn the_upgraded_visit_is_a_fixed_point_and_restores_the_ordinary_way_next_time() {
        let map = synthetic::map();
        let saved = old_visit(&map).snapshot();
        let mut upgraded = synthetic::served(&map, declared());
        upgraded.restore(&saved, &map).unwrap();
        walk(&mut upgraded, SCENE);
        let second = upgraded.snapshot();
        // No further upgrade: the key is there, so the plain path restores it exactly.
        assert!(!second.registry_upgrade_in(&[&synthetic::REGISTRATION], "garden1", None));
        let mut again = synthetic::served(&map, declared());
        again.restore(&second, &map).unwrap();
        assert_eq!(json(&again), json(&upgraded));
        // And resaving the upgraded visit twice is stable.
        let mut third = synthetic::served(&map, declared());
        third.restore(&again.snapshot(), &map).unwrap();
        assert_eq!(json(&third), json(&again));
    }
    #[test]
    fn the_gated_trigger_keys_come_from_the_controller_and_are_accepted_by_the_extension() {
        let map = synthetic::map();
        let served = synthetic::served(&map, declared());
        // The controller gates the trigger with thread `gated_thread`, and only that one.
        assert_eq!(
            served.gated_triggers(),
            [format!(
                "trigger/{}",
                synthetic::id(&map, "gated_trigger").0
            )]
        );
        // The old program ungated it, so the upgrade succeeded only because it was named.
        let plain = Interactions::load(&map).unwrap();
        assert!(plain.gated_triggers().is_empty());
        let mut new = synthetic::served(&map, declared());
        new.restore(&plain.snapshot(), &map).unwrap();
        // An extra gate must name a trigger rule.
        let mut bad = declared();
        bad.plan.gated = vec!["garden1/scene".into()];
        assert!(synthetic::served(&map, bad)
            .restore(&plain.snapshot(), &map)
            .is_err());
    }
    #[test]
    fn a_declaration_may_rearm_a_handled_trigger_and_must_name_its_own_rules() {
        let map = synthetic::map();
        let saved = old_visit(&map).snapshot();
        // The committed trigger was handled (dialogue) but now has consequences: rearm it.
        let mut wants = declared();
        wants.plan.rearm = vec!["committed_thread".into()];
        let mut new = synthetic::served(&map, wants);
        new.restore(&saved, &map).unwrap();
        let t = trigger(&new, "committed_trigger");
        assert!(!t.fired && !t.reported);
        assert_eq!(usage(&new, "trigger/3").count, 0);
        assert_eq!(
            usage(&new, "garden1/reward").count,
            0,
            "a rearmed trigger keeps its rules fresh"
        );
        // A consumed key must be one of the controller's own rules, and must exist.
        for key in ["door/0", "garden1/nothing", "garden10/scene", ""] {
            let mut bad = synthetic::first();
            bad.plan.consumed = vec![key.into()];
            assert!(
                synthetic::served(&map, bad).restore(&saved, &map).is_err(),
                "{key:?}"
            );
        }
    }
    #[test]
    fn an_old_version_one_save_upgrades_too() {
        // A v1 save has no shared runtime: the legacy import marks fired triggers consumed.
        let map = synthetic::map();
        let mut value = json(&old_visit(&map));
        value.as_object_mut().unwrap().remove("shared");
        for list in ["doors", "triggers"] {
            for item in value[list].as_array_mut().unwrap() {
                item["id"] = serde_json::Value::Null;
            }
        }
        let saved: Snapshot = serde_json::from_value(value).unwrap();
        let mut new = synthetic::served(&map, declared());
        new.restore(&saved, &map).unwrap();
        // The legacy import marked every fired trigger consumed; the upgrade then rearmed the
        // pending ones and left the committed one spent.
        assert_eq!(usage(&new, "trigger/1").count, 0);
        assert_eq!(usage(&new, "trigger/3").count, 1);
        assert_eq!(usage(&new, "garden1/reward").count, 1);
        assert_eq!(walk(&mut new, SCENE).message.as_deref(), Some("scene"));
    }
    #[test]
    fn nothing_upgrades_unless_the_visit_has_a_controller_that_the_save_lacks() {
        let map = synthetic::map();
        // A visit without controllers restores as before, whatever the save says.
        let plain = Interactions::load(&map).unwrap();
        let saved = plain.snapshot();
        let mut restored = Interactions::load(&map).unwrap();
        restored.restore(&saved, &map).unwrap();
        assert_eq!(json(&restored), json(&plain));
        assert!(!saved.registry_upgrade_in(&[], "garden1", None));
        // A save that names a controller the visit lacks is refused, not upgraded.
        let served = synthetic::served(&map, synthetic::first());
        assert!(restored.restore(&served.snapshot(), &map).is_err());
        // A registration for another visit never asks for an upgrade.
        assert!(!saved.registry_upgrade_in(&[&synthetic::REGISTRATION], "garden2", None));
        assert!(!saved.registry_upgrade_in(&[&synthetic::REGISTRATION], "skool1", None));
        // Saved state for a controller other than the visit's is refused, with or without its own.
        for levels in [
            serde_json::json!({ "garden2": null }),
            serde_json::json!({ "garden1": null, "garden2": null }),
        ] {
            let mut value = json(&plain);
            value["levels"] = levels;
            let saved: Snapshot = serde_json::from_value(value).unwrap();
            assert!(synthetic::served(&map, synthetic::first())
                .restore(&saved, &map)
                .is_err());
        }
    }
    #[test]
    fn a_controller_with_saved_state_restores_without_any_upgrade() {
        let map = synthetic::map();
        let mut a = synthetic::served(&map, synthetic::first());
        {
            let c = a.levels[0].ctl.downcast_mut::<Synthetic>().unwrap();
            c.state = Saved {
                clock: 7.5,
                open: true,
                opened_at: Some(3.),
                later: 4,
                ..c.state.clone()
            };
        }
        let saved = a.snapshot();
        let mut b = synthetic::served(&map, synthetic::first());
        b.restore(&saved, &map).unwrap();
        assert_eq!(json(&b), json(&a));
        assert_eq!(
            b.levels[0]
                .ctl
                .downcast_ref::<Synthetic>()
                .unwrap()
                .state
                .clock,
            7.5
        );
        // A saved state for the other entrance, or a broken one, is refused by the controller.
        for (key, value) in [
            ("returning", serde_json::json!(true)),
            ("clock", serde_json::json!(-1.)),
            ("opened_at", serde_json::Value::Null),
            ("version", serde_json::json!(3)),
        ] {
            let mut bad = json(&a);
            bad["levels"]["garden1"][key] = value;
            let bad: Snapshot = serde_json::from_value(bad).unwrap();
            assert!(
                synthetic::served(&map, synthetic::first())
                    .restore(&bad, &map)
                    .is_err(),
                "{key}"
            );
        }
        // The other visit's controller never matches this one's entrance flag.
        let mut returning = synthetic::first();
        returning.visit = Visit { returning: true };
        assert!(synthetic::served(&map, returning)
            .restore(&saved, &map)
            .is_err());
    }
    #[test]
    fn legacy_controllers_restore_first_and_keep_their_state_through_the_upgrade() {
        // A legacy-owned visit (the gym) that a controller is injected into: the older save
        // carries the gym's state, which must come back exactly as it always did.
        let mut map = synthetic::map();
        for n in 1..=12 {
            map.entities.push(super::super::Entity::from([
                ("classname".into(), "script_object".into()),
                ("targetname".into(), format!("bleach{n}")),
                ("model".into(), "*1".into()),
                ("origin".into(), "0 100 0".into()),
            ]));
        }
        map.entities.push(super::super::Entity::from([
            ("classname".into(), "script_object".into()),
            ("move_thread".into(), "extendBleachers".into()),
            ("origin".into(), "0 0 0".into()),
        ]));
        let mut old = Interactions::load(&map).unwrap();
        old.gym = Some(crate::gym::Gym::load(&map).unwrap());
        old.gym.as_mut().unwrap().used = true;
        old.configure_events(&map).unwrap();
        let saved = old.snapshot();

        let mut new = Interactions::load(&map).unwrap();
        new.gym = Some(crate::gym::Gym::load(&map).unwrap());
        new.probe_levels(
            &map,
            vec![Slot {
                reg: &synthetic::REGISTRATION,
                ctl: Box::new(synthetic::first()),
            }],
        )
        .unwrap();
        assert!(new.legacy_owned());
        new.restore(&saved, &map).unwrap();
        assert!(new.gym.as_ref().unwrap().used, "the legacy state survived");
        assert_eq!(json(&new)["levels"]["garden1"]["version"], 2);
        assert_eq!(new.levels.len(), 1);
        // A legacy owner answers the gates, so the registry contributes none.
        assert!(new.gated_triggers().is_empty());
    }
    #[test]
    fn a_failed_upgrade_leaves_the_error_to_the_caller_and_never_a_half_restored_visit_in_a_save() {
        // The upgrade is part of `restore`, which builds a replacement visit: a save that fails
        // any check is an error, and `Level::restore_logic` discards the visit with it.
        let map = synthetic::map();
        let mut saved = json(&old_visit(&map));
        saved["shared"]["signature"] = "another program".into();
        let saved: Snapshot = serde_json::from_value(saved).unwrap();
        assert!(synthetic::served(&map, declared())
            .restore(&saved, &map)
            .is_err());
    }
    #[test]
    fn alice_restarts_at_the_entrance_only_when_her_saved_position_cannot_stand() {
        let map = synthetic::map();
        let floor = (vec3(-500., -500., -20.), vec3(500., 500., 0.));
        let open = World::fixture(&[floor]);
        // A crate around the player's feet: the body is not clear.
        let blocked = World::fixture(&[floor, (vec3(-40., -40., 0.), vec3(40., 40., 120.))]);
        let player = Player::new(vec3(0., 0., 0.));
        let served = |c: Synthetic| synthetic::served(&map, c);

        let fine = served(synthetic::first());
        assert!(open.body_clear(player.feet) && !blocked.body_clear(player.feet));
        assert!(!fine.levels_respawn(&player, &open, false));
        // Body not clear: restart, unless she is flying.
        assert!(fine.levels_respawn(&player, &blocked, false));
        assert!(!fine.levels_respawn(&player, &blocked, true));
        // The controller rejects the position, even in free flight.
        let mut picky = synthetic::first();
        picky.reject_beyond_x = Some(-1e6);
        let picky = served(picky);
        assert!(picky.levels_respawn(&player, &open, false));
        assert!(picky.levels_respawn(&player, &open, true));
        // The controller declares that no old position can be trusted.
        let mut always = synthetic::first();
        always.plan.respawn = Respawn::Always;
        assert!(served(always).levels_respawn(&player, &open, false));
        // Without controllers only the body decides (the save code asks only for a visit that
        // has one).
        let plain = Interactions::load(&map).unwrap();
        assert!(!plain.levels_respawn(&player, &open, false));
        assert!(plain.levels_respawn(&player, &blocked, false));
    }
}

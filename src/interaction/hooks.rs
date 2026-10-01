//! The generic hooks that consult registered level controllers (F1.3).
//!
//! `interaction.rs` keeps its legacy chains untouched. Each chain ends with one call into this
//! module, so a controller in `Interactions::levels` is consulted only after every legacy
//! controller has had its turn. That keeps existing precedence and, because nothing is
//! registered for an existing visit, every existing map's event program and snapshot
//! byte-identical (`--registry-check` proves it against the F1.0 baseline).
use super::{Events, Interactions, Trigger, TriggerKind};
use crate::{
    assets::Assets,
    bsp::Bsp,
    cinematic::Camera,
    collision::{Collider, World},
    combat::{Feedback, Hit, Target},
    entity::{Id, Registry},
    event::{Condition, Effect, Facts, Rule},
    level::{self, Combat, LevelController, RuleContext, Slot, TriggerClass, TriggerInfo},
    levels::{self, Registration},
    loot::Source,
    movement::Player,
    story::Story,
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// A read-only view of a trigger for the controllers.
fn info(t: &Trigger) -> TriggerInfo<'_> {
    let (class, thread, exit) = match &t.kind {
        TriggerKind::Dialogue(_) => (TriggerClass::Dialogue, "", None),
        TriggerKind::Exit(map, _) => (TriggerClass::Exit, "", Some(map.as_str())),
        TriggerKind::Teleport(..) => (TriggerClass::Teleport, "", None),
        TriggerKind::Hurt(_) => (TriggerClass::Hurt, "", None),
        TriggerKind::Fall => (TriggerClass::Fall, "", None),
        TriggerKind::Script(thread) => (TriggerClass::Script, thread.as_str(), None),
    };
    TriggerInfo {
        id: t.id,
        name: &t.name,
        class,
        thread,
        exit,
        target: t.target.as_deref(),
    }
}

/// A script thread, offered to each controller in registration order. `None` leaves the thread
/// pending, exactly like an unknown thread on a legacy visit.
pub(super) fn event(levels: &mut [Slot], thread: &str) -> Option<Events> {
    levels.iter_mut().find_map(|s| s.ctl.event(thread))
}

impl Interactions {
    pub fn companions_for_movers(&mut self, actors: &[crate::level::CompanionContact]) {
        for slot in &mut self.levels { slot.ctl.companion_contacts(actors); }
    }
    pub fn actors_for_movers(&mut self, actors: &[Target]) {
        for slot in &mut self.levels { slot.ctl.actor_contacts(actors); }
    }
    pub fn prepare_player(&mut self, stats: &mut crate::inventory::Stats, player: &mut Player) {
        stats.minimum_sanity = 0.;
        for s in &mut self.levels {
            s.ctl.prepare_player(stats, player);
        }
        for d in &mut self.doors {
            if let Some(locked) = self.levels.iter().find_map(|s| s.ctl.door_locked(d.id)) {
                d.locked = locked;
                if locked { d.target = 0.; }
            }
        }
    }
    pub fn filter_level_controls(&self, controls: &mut crate::movement::Controls) {
        for s in &self.levels { s.ctl.filter_controls(controls); }
    }
    pub fn level_hides_player(&self) -> bool { self.levels.iter().any(|s| s.ctl.hides_player()) }
    pub fn level_blocks_weapons(&self) -> bool { self.levels.iter().any(|s| s.ctl.blocks_weapons()) }
    pub fn place_level_particles(&self, steam: &mut crate::particles::Steam) {
        for s in &self.levels {
            s.ctl.particles(steam);
        }
    }
    /// Whether one of the typed legacy controllers owns this visit (F1.4a). Registry hooks that
    /// legacy code answers itself (gates) are consulted only when none does.
    pub(super) fn legacy_owned(&self) -> bool {
        self.school.is_some()
            || self.gym.is_some()
            || self.school2.is_some()
            || self.village.is_some()
            || self.fortress.is_some()
            || self.beyond.is_some()
            || self.pool.is_some()
            || self.pandemonium.is_some()
            || self.duchess.is_some()
            || self.encounters.is_some()
    }
    /// Build the controllers that serve this visit, after every legacy load and before the
    /// event program is configured, so their rules and facts join it.
    pub(super) fn load_levels(
        &mut self,
        assets: &mut Assets,
        map: &Bsp,
        name: &str,
        entry: Option<&str>,
    ) -> Result<()> {
        let serving = levels::serving(name, entry).collect::<Vec<_>>();
        self.load_levels_from(name, serving, |reg| (reg.load)(assets, map, name, entry))?;
        // A registered mover may own a rotating door as part of a scene. Keep its
        // stable save identity, but leave both its pose and collision to that owner.
        for door in &mut self.doors {
            if levels::owns_submodel(name, &map.entities[door.id.0]) {
                door.enabled = false;
            }
        }
        Ok(())
    }
    /// The rules a registration must obey, with the controller factory injected.
    fn load_levels_from(
        &mut self,
        name: &str,
        serving: Vec<&'static Registration>,
        mut build: impl FnMut(&'static Registration) -> Result<Box<dyn LevelController>>,
    ) -> Result<()> {
        self.levels.clear();
        ensure!(
            serving.len() <= 1,
            "More than one level registration serves {name}"
        );
        ensure!(
            serving.is_empty() || !self.legacy_owned(),
            "{name} is owned by a legacy controller and cannot also be registered"
        );
        for reg in serving {
            let ctl = build(reg)?;
            ensure!(
                ctl.id() == reg.id,
                "Controller {} was registered as {}",
                ctl.id(),
                reg.id
            );
            self.levels.push(Slot { reg, ctl });
        }
        Ok(())
    }
    /// The gate of a trigger on a visit no legacy controller owns.
    pub(super) fn level_gate(&self, t: &Trigger) -> Condition {
        if self.legacy_owned() {
            return Condition::Always;
        }
        let info = info(t);
        self.levels
            .iter()
            .find_map(|s| s.ctl.gate(&info))
            .unwrap_or(Condition::Always)
    }
    pub(super) fn level_facts(&self) -> Facts {
        let mut facts = Facts::default();
        for s in &self.levels {
            facts.extend(s.ctl.facts());
        }
        facts
    }
    /// Writable puzzle flags and counters the controllers' rules may set.
    pub(super) fn level_initial(&self) -> Facts {
        let mut facts = Facts::default();
        for s in &self.levels {
            facts.extend(s.ctl.initial());
        }
        facts
    }
    pub(super) fn level_receivers(&self, registry: &Registry) -> Vec<Id> {
        self.levels
            .iter()
            .flat_map(|s| s.ctl.receivers(registry))
            .collect()
    }
    /// The controllers' rules. Rule keys must live under `<id>/` and fact keys under `<id>.`,
    /// so a registration can never collide with a legacy key or another visit's.
    pub(super) fn level_rules(
        &self,
        map: &Bsp,
        registry: &Registry,
        receivers: &BTreeSet<Id>,
    ) -> Result<Vec<Rule>> {
        let triggers = self.triggers.iter().map(info).collect::<Vec<_>>();
        let ctx = RuleContext {
            map,
            registry,
            receivers,
            triggers: &triggers,
        };
        let mut rules = Vec::new();
        for s in &self.levels {
            let id = s.reg.id;
            for key in s.ctl.facts().0.keys().chain(s.ctl.initial().0.keys()) {
                ensure!(
                    key.strip_prefix(id).is_some_and(|k| k.starts_with('.')),
                    "Fact {key} of {id} must start with {id}."
                );
            }
            for rule in s.ctl.rules(&ctx) {
                ensure!(
                    rule.key
                        .strip_prefix(id)
                        .is_some_and(|k| k.starts_with('/')),
                    "Rule {} of {id} must start with {id}/",
                    rule.key
                );
                rules.push(rule);
            }
        }
        Ok(rules)
    }
    /// Offer an event-runtime output to every controller, after the legacy handlers.
    pub(super) fn level_output(&mut self, effect: &Effect) -> Events {
        let mut events = Events::default();
        for s in &mut self.levels {
            if let Some(e) = s.ctl.output(effect) {
                events.merge(e);
            }
        }
        events
    }
    pub(super) fn level_update(
        &mut self,
        world: &mut World,
        player: &Player,
        aim: Vec3,
        use_pressed: bool,
    ) -> Events {
        let mut events = Events::default();
        for s in &mut self.levels {
            events.merge(s.ctl.update(world, player, aim, use_pressed));
        }
        events
    }
    pub(super) fn level_prompt(&self, world: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        self.levels
            .iter()
            .find_map(|s| s.ctl.prompt(world, eye, aim))
    }
    /// Advance the controllers' movers and carry riders, then reposition trigger volumes that
    /// ride them. Runs after every legacy `advance`.
    pub(super) fn level_advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
    ) -> Result<()> {
        if self.levels.is_empty() {
            return Ok(());
        }
        let fixed = self
            .doors
            .iter()
            .filter(|d| d.enabled && self.event_world.enabled(d.id))
            .map(|d| d.collider.clone())
            .collect::<Vec<Collider>>();
        for s in &mut self.levels {
            s.ctl.advance(dt, map, world, player, &fixed)?;
        }
        for t in &mut self.triggers {
            let e = &map.entities[t.id.0];
            let Some(base) = e.get("origin").and_then(|s| super::vector(s)) else {
                continue;
            };
            let Some(index) = super::model(e, map) else {
                continue;
            };
            if let Some((p, r)) = self
                .levels
                .iter()
                .find_map(|s| s.ctl.trigger_pose(&t.name, base))
            {
                t.volume = Collider::model(map, index, p, r, false)?;
            }
        }
        Ok(())
    }
    /// Let a controller begin its entry scene. False falls back to the plain `entry` beat.
    pub(super) fn level_entry_story(&mut self, story: &mut Story) -> bool {
        self.levels.iter_mut().any(|s| s.ctl.entry_story(story))
    }
    /// Whether the story may advance this frame. Every controller is asked.
    pub(super) fn level_prepare_story(&self, story: &mut Story) -> bool {
        self.levels
            .iter()
            .fold(true, |ok, s| s.ctl.prepare_story(story) && ok)
    }
    pub(super) fn level_sync_story(&mut self, story: &Story) {
        for s in &mut self.levels {
            s.ctl.sync_story(story);
        }
    }
    /// The registry's turn to skip a scene: only when no legacy controller skipped one.
    pub(super) fn skip_after(
        &mut self,
        legacy: bool,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        story: &mut Story,
    ) -> Result<bool> {
        if legacy {
            return Ok(true);
        }
        for s in &mut self.levels {
            if s.ctl.skip(map, world, player, story)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn level_dialogue(&mut self, name: &str) -> Events {
        let mut events = Events::default();
        for s in &mut self.levels {
            events.merge(s.ctl.dialogue_complete(name));
        }
        events
    }
    pub(super) fn level_loot_sources(&self) -> Vec<Source> {
        self.levels
            .iter()
            .flat_map(|s| s.ctl.loot_sources())
            .chain(self.pool.iter().flat_map(|p| p.loot_sources()))
            .collect()
    }
    /// Every controller's saved state, keyed by its id. A stateless controller saves `Null`,
    /// so the key still records that it served the visit.
    pub(super) fn level_snapshots(&self) -> BTreeMap<String, serde_json::Value> {
        self.levels
            .iter()
            .map(|s| (s.reg.id.to_owned(), s.ctl.snapshot()))
            .collect()
    }
    pub(super) fn restore_levels(
        &mut self,
        saved: &BTreeMap<String, serde_json::Value>,
        map: &Bsp,
    ) -> Result<()> {
        ensure!(
            saved.len() == self.levels.len()
                && self.levels.iter().all(|s| saved.contains_key(s.reg.id)),
            "Saved level state does not match this visit"
        );
        for s in &mut self.levels {
            s.ctl.restore(&saved[s.reg.id], map)?;
        }
        Ok(())
    }
    /// Owners supply authored summon windows without changing quest state.
    pub fn levels_allow_cheshire(&self) -> bool {
        self.levels.iter().all(|s| s.ctl.allow_cheshire())
    }
    /// Whether a registered controller owns Alice's movement (a ride, a grab, a disguise).
    pub fn levels_controlled(&self) -> bool {
        self.levels.iter().any(|s| s.ctl.controlled())
    }
    /// Whether a registered controller refuses recovery (a scene or ride in progress).
    pub fn levels_transport(&self) -> bool {
        self.levels.iter().any(|s| s.ctl.in_transport())
    }
    /// The skippable scene id: the legacy chain's answer, else the first registered scene.
    pub fn scene_id_after(&self, legacy: Option<&'static str>) -> Option<&'static str> {
        legacy.or_else(|| self.levels.iter().find_map(|s| s.ctl.scene_id()))
    }
    /// The scripted camera: the legacy chain's answer, else the first registered scene camera.
    pub fn camera_after(&self, legacy: Option<Camera>, world: &World) -> Option<Camera> {
        legacy.or_else(|| self.levels.iter().find_map(|s| s.ctl.camera(world)))
    }
    /// The screen fade: the legacy chain's answer, else the first registered fade.
    pub fn fade_after(&self, legacy: Option<(Color, f32)>) -> Option<(Color, f32)> {
        legacy.or_else(|| self.levels.iter().find_map(|s| s.ctl.fade()))
    }
    /// Help text of the first controller that has some. Consulted after every legacy objective.
    pub fn levels_objective(&self) -> Option<String> {
        self.levels.iter().find_map(|s| s.ctl.objective())
    }
    /// Fold the recovery entrance through the controllers, after the legacy ones.
    pub fn levels_recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        self.levels
            .iter()
            .fold(normal, |entry, s| s.ctl.recovery_entry(entry))
    }
    /// Targets the registered controllers publish, each inside its own hit range.
    pub fn levels_targets(&self) -> Vec<Target> {
        let mut targets = level::targets(&self.levels);
        targets.extend(self.pool.iter().flat_map(|p| p.targets()));
        targets
    }
    /// Apply a hit to the controller whose exact range holds it. `None` means no registration
    /// owns the id and the legacy dispatch continues; `Some(sound)` means it was handled. The
    /// viewer asks this after the legacy exact-id arms and before its `>= encounters::BASE`
    /// catch-all, which would otherwise swallow every registry id.
    pub fn hit_level(&mut self, hit: Hit) -> Option<Option<&'static str>> {
        if let Some(result) = self.pool.as_mut().and_then(|p| p.hit(hit)) {
            return Some(result);
        }
        let owner = level::hit_owner(&self.levels, hit.id)?;
        Some(self.levels[owner].ctl.hit(hit))
    }
    /// Alice's Demon Dice summon struck a target: let its controller turn hostile.
    pub fn levels_provoke_summon(&mut self, id: usize) {
        if let Some(p) = &mut self.pool {
            p.provoke(id);
        }
        if let Some(owner) = level::hit_owner(&self.levels, id) {
            self.levels[owner].ctl.provoke_summon(id);
        }
    }
    /// Pool owns its two boulder ants outside the registered level-controller list.
    /// Keep viewer and route eligibility identical so their clocks always advance.
    pub fn has_level_combat(&self) -> bool {
        self.pool.is_some() || !self.levels.is_empty()
    }
    /// One combat step for Pool and every registered controller.
    pub fn levels_step(&mut self, combat: &mut Combat<'_>) -> Feedback {
        let mut out = level::step_controllers(&mut self.levels, combat);
        if let Some(p) = &mut self.pool {
            let f = p.combat(combat);
            out.damage += f.damage;
            out.impulse += f.impulse;
            out.summon_hits.extend(f.summon_hits);
            out.sounds.extend(f.sounds);
            out.spatial_sounds.extend(f.spatial_sounds);
        }
        out
    }
    /// Inject controllers past the legacy-owner guard and rebuild the event program.
    /// Registry and save checks use this to reconstruct the program without a visit owner.
    pub fn probe_levels(&mut self, map: &Bsp, slots: Vec<Slot>) -> Result<()> {
        self.levels = slots;
        self.configure_events(map)
    }
    /// Reject a saved player position a registered controller cannot accept.
    pub fn validate_levels(&self, player: &Player) -> Result<()> {
        for s in &self.levels {
            s.ctl.validate_player(player)?;
        }
        Ok(())
    }
    /// Whether a registered scene is running (`Interactions::scripted` includes it too).
    pub fn levels_scripted(&self) -> bool {
        self.levels.iter().any(|s| s.ctl.scripted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cinematic::Camera,
        collision::Liquid,
        combat::{Hit, Target},
        entity::Id,
        event::{Action, Event},
        interaction::tests::fixture,
        level::LevelController,
        levels::{Entity, Registration},
    };
    use anyhow::Context;
    use std::{any::Any, cell::RefCell, rc::Rc};

    type Log = Rc<RefCell<Vec<String>>>;

    /// A registered controller that records every hook it is asked about.
    struct Probe {
        log: Log,
        scripted: bool,
        entry: bool,
        prepare: bool,
        skip: bool,
        gate: Option<Condition>,
        state: i64,
        bad_rule_key: bool,
        bad_fact: bool,
    }
    fn probe(log: &Log) -> Probe {
        Probe {
            log: log.clone(),
            scripted: false,
            entry: false,
            prepare: true,
            skip: false,
            gate: None,
            state: 0,
            bad_rule_key: false,
            bad_fact: false,
        }
    }
    impl Probe {
        fn note(&self, what: impl Into<String>) {
            self.log.borrow_mut().push(what.into());
        }
    }
    impl LevelController for Probe {
        fn id(&self) -> &'static str {
            "garden2"
        }
        fn facts(&self) -> Facts {
            let mut f = Facts::default();
            f.flag(
                if self.bad_fact {
                    "other.derived"
                } else {
                    "garden2.derived"
                },
                true,
            );
            f
        }
        fn initial(&self) -> Facts {
            let mut f = Facts::default();
            f.flag("garden2.open", false);
            f
        }
        fn gate(&self, _: &crate::level::TriggerInfo<'_>) -> Option<Condition> {
            self.note("gate");
            self.gate.clone()
        }
        fn rules(&self, _: &RuleContext<'_>) -> Vec<Rule> {
            let key = |k: &str| {
                if self.bad_rule_key {
                    "door/0".to_owned()
                } else {
                    k.to_owned()
                }
            };
            vec![
                Rule {
                    key: key("garden2/open"),
                    event: Event::Signal("garden2.go".into()),
                    condition: Condition::flag("garden2.derived"),
                    once: false,
                    cooldown: 0.,
                    actions: vec![Action::Flag("garden2.open".into(), true)],
                },
                Rule {
                    key: "garden2/out".into(),
                    event: Event::Signal("garden2.out".into()),
                    condition: Condition::Always,
                    once: false,
                    cooldown: 0.,
                    actions: vec![Action::Output(Effect::Activate(Id(0)))],
                },
            ]
        }
        fn output(&mut self, effect: &Effect) -> Option<Events> {
            self.note(format!("output {effect:?}"));
            matches!(effect, Effect::Activate(_)).then(|| Events {
                message: Some("out".into()),
                ..Default::default()
            })
        }
        fn event(&mut self, thread: &str) -> Option<Events> {
            self.note(format!("event {thread}"));
            (thread == "probe_thread").then(|| Events {
                message: Some("hit".into()),
                ..Default::default()
            })
        }
        fn dialogue_complete(&mut self, name: &str) -> Events {
            self.note("dialogue");
            Events {
                message: Some(format!("dialogue {name}")),
                ..Default::default()
            }
        }
        fn scripted(&self) -> bool {
            self.scripted
        }
        fn entry_story(&mut self, _: &mut Story) -> bool {
            self.note("entry_story");
            self.entry
        }
        fn prepare_story(&self, _: &mut Story) -> bool {
            self.note("prepare_story");
            self.prepare
        }
        fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, _: &mut Story) -> Result<bool> {
            self.note("skip");
            Ok(self.skip)
        }
        fn camera(&self, _: &World) -> Option<Camera> {
            self.note("camera");
            Some(Camera::look(vec3(1., 2., 3.), Vec3::ZERO))
        }
        fn fade(&self) -> Option<(Color, f32)> {
            self.note("fade");
            Some((BLACK, 0.5))
        }
        fn scene_id(&self) -> Option<&'static str> {
            self.note("scene_id");
            Some("probe_scene")
        }
        fn controlled(&self) -> bool {
            self.scripted
        }
        fn objective(&self) -> Option<String> {
            Some("probe help".into())
        }
        fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
            (normal.0 + Vec3::Z * 9., normal.1)
        }
        fn targets(&self) -> Vec<Target> {
            let target = |id| Target {
                id,
                center: Vec3::ZERO,
                half: Vec3::ONE,
            };
            // The second id lies in the encounter range: the registry must drop it.
            vec![target(7_300_001), target(crate::encounters::BASE + 7)]
        }
        fn hit(&mut self, hit: Hit) -> Option<&'static str> {
            self.note(format!("hit {}", hit.id));
            Some("sound/probe.wav")
        }
        fn provoke_summon(&mut self, id: usize) {
            self.note(format!("provoke {id}"));
        }
        fn colliders(&self) -> Vec<Collider> {
            vec![Collider::fixture(vec3(0., 0., 0.), vec3(1., 1., 1.))]
        }
        fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
            vec![(1, Vec3::X, Quat::IDENTITY)]
        }
        fn liquids(&self) -> Vec<Liquid> {
            vec![Liquid {
                contents: 0x10,
                volume: Collider::fixture(vec3(300., 300., 0.), vec3(400., 400., 50.)),
            }]
        }
        fn snapshot(&self) -> serde_json::Value {
            serde_json::json!(self.state)
        }
        fn restore(&mut self, saved: &serde_json::Value, _: &Bsp) -> Result<()> {
            self.state = saved.as_i64().context("Invalid saved probe state")?;
            Ok(())
        }
        fn validate_player(&self, _: &Player) -> Result<()> {
            ensure!(self.state >= 0, "Probe rejects the player");
            Ok(())
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    struct WrongId;
    impl LevelController for WrongId {
        fn id(&self) -> &'static str {
            "garden3"
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    fn nowhere(
        _: &mut Assets,
        _: &Bsp,
        _: &str,
        _: Option<&str>,
    ) -> Result<Box<dyn LevelController>> {
        unreachable!("the unit tests inject the controller")
    }
    static REG: Registration = Registration {
        id: "garden2",
        applies: |m, e| levels::first_visit(m, e, "garden2"),
        load: nowhere,
        art: None,
        owns_submodel: |_: &str, _: &Entity| false,
        owns_npc: |_, _| false,
        target_base: Some(7_300_000),
        story_beats: &[],
        checks: &[],
        save_cases: &[],
        visibility: &[],
    };
    fn with(i: &mut Interactions, p: Probe, map: &Bsp) {
        i.levels.push(Slot {
            reg: &REG,
            ctl: Box::new(p),
        });
        i.configure_events(map).unwrap();
    }
    fn signature(i: &Interactions) -> String {
        serde_json::to_value(i.snapshot()).unwrap()["shared"]["signature"]
            .as_str()
            .unwrap()
            .to_owned()
    }
    fn synthetic_gym_map() -> Bsp {
        let mut map = fixture();
        for n in 1..=12 {
            map.entities.push(Entity::from([
                ("classname".into(), "script_object".into()),
                ("targetname".into(), format!("bleach{n}")),
                ("model".into(), "*1".into()),
                ("origin".into(), "0 100 0".into()),
            ]));
        }
        map.entities.push(Entity::from([
            ("classname".into(), "script_object".into()),
            ("move_thread".into(), "extendBleachers".into()),
            ("origin".into(), "0 0 0".into()),
        ]));
        map
    }
    fn script_trigger() -> Bsp {
        let mut map = fixture();
        map.entities.push(Entity::from([
            ("classname".into(), "trigger_once".into()),
            ("model".into(), "*1".into()),
            ("origin".into(), "0 100 0".into()),
            ("thread".into(), "probe_thread".into()),
        ]));
        map
    }

    #[test]
    fn controller_facts_receivers_and_rules_join_the_program_and_leaving_restores_it() {
        let map = fixture();
        let mut i = Interactions::load(&map).unwrap();
        let original = signature(&i);
        let legacy_keys = i
            .event_world
            .snapshot()
            .usage
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let log = Log::default();
        with(&mut i, probe(&log), &map);
        assert_ne!(signature(&i), original);
        let usage = i.event_world.snapshot().usage;
        assert!(legacy_keys.iter().all(|k| usage.contains_key(k)));
        assert!(usage.contains_key("garden2/open") && usage.contains_key("garden2/out"));
        // The controller's derived fact gates its rule, and the rule may set its writable flag.
        i.dispatch(Event::Signal("garden2.go".into()));
        let facts = i.event_world.snapshot().facts;
        assert_eq!(
            facts.0.get("garden2.open"),
            Some(&crate::event::Value::Flag(true))
        );
        // Without the controller the program is byte-identical to the original one.
        let mut back = Interactions::load(&map).unwrap();
        back.configure_events(&map).unwrap();
        assert_eq!(signature(&back), original);
    }
    #[test]
    fn keys_outside_the_controllers_own_namespace_are_rejected() {
        let map = fixture();
        for (rule, fact) in [(true, false), (false, true)] {
            let mut i = Interactions::load(&map).unwrap();
            i.levels.push(Slot {
                reg: &REG,
                ctl: Box::new(Probe {
                    bad_rule_key: rule,
                    bad_fact: fact,
                    ..probe(&Log::default())
                }),
            });
            assert!(i.configure_events(&map).is_err(), "rule {rule} fact {fact}");
        }
    }
    #[test]
    fn the_snapshot_gains_levels_only_when_a_controller_serves_the_visit() {
        let map = fixture();
        let plain = Interactions::load(&map).unwrap();
        assert!(serde_json::to_value(plain.snapshot())
            .unwrap()
            .get("levels")
            .is_none());
        let mut a = Interactions::load(&map).unwrap();
        with(
            &mut a,
            Probe {
                state: 7,
                ..probe(&Log::default())
            },
            &map,
        );
        let saved = a.snapshot();
        assert_eq!(
            serde_json::to_value(&saved).unwrap()["levels"]["garden2"],
            7
        );
        let mut b = Interactions::load(&map).unwrap();
        with(&mut b, probe(&Log::default()), &map);
        b.restore(&saved, &map).unwrap();
        assert_eq!(
            b.levels[0].ctl.downcast_ref::<Probe>().unwrap().state,
            7,
            "the controller's state comes back"
        );
        // A save that names a controller this visit lacks is refused. One that omits a
        // controller the visit has predates its registration: F2's generic upgrade joins it
        // (`interaction/upgrade.rs`) instead of refusing it as F1 did.
        let mut without = Interactions::load(&map).unwrap();
        without.configure_events(&map).unwrap();
        assert!(without.restore(&saved, &map).is_err());
        b.restore(&plain.snapshot(), &map).unwrap();
        assert_eq!(b.levels.len(), 1);
        assert!(serde_json::to_value(b.snapshot())
            .unwrap()
            .get("levels")
            .is_some());
        // The saved position is validated by the controller.
        b.levels[0].ctl.downcast_mut::<Probe>().unwrap().state = -1;
        assert!(b.validate_levels(&Player::new(Vec3::ZERO)).is_err());
    }
    #[test]
    fn a_legacy_controller_answers_the_gate_before_the_registry() {
        let map = synthetic_gym_map();
        let mut i = Interactions::load(&map).unwrap();
        let log = Log::default();
        i.levels.push(Slot {
            reg: &REG,
            ctl: Box::new(Probe {
                gate: Some(Condition::flag("garden2.derived")),
                ..probe(&log)
            }),
        });
        let t = Trigger {
            id: Id(0),
            name: "gate".into(),
            volume: Collider::fixture(Vec3::ZERO, Vec3::ONE),
            kind: TriggerKind::Script("thread".into()),
            inside: false,
            cooldown: 0.,
            reported: false,
            target: None,
            once: false,
            fired: false,
            health: 0.,
            bounds: (Vec3::ZERO, Vec3::ONE),
        };
        // No legacy controller: the registry decides.
        assert!(!i.legacy_owned());
        assert!(matches!(i.gate(&t), Condition::Flag(k) if k == "garden2.derived"));
        assert_eq!(log.borrow().as_slice(), ["gate"]);
        // A legacy controller (the gym) owns the visit: the registry is not even asked.
        log.borrow_mut().clear();
        i.gym = Some(crate::gym::Gym::load(&map).unwrap());
        assert!(i.legacy_owned());
        assert!(matches!(i.gate(&t), Condition::Always));
        assert!(log.borrow().is_empty());
    }
    #[test]
    fn a_registration_must_serve_exactly_one_controller_on_a_visit_without_a_legacy_owner() {
        let log = Log::default();
        let build = |_: &'static Registration| -> Result<Box<dyn LevelController>> {
            Ok(Box::new(probe(&log)))
        };
        // A plain visit accepts its one registered controller ...
        let mut i = Interactions::load(&fixture()).unwrap();
        i.load_levels_from("garden2", vec![&REG], build).unwrap();
        assert_eq!(i.levels.len(), 1);
        // ... and none at all clears any earlier one.
        i.load_levels_from("garden2", vec![], build).unwrap();
        assert!(i.levels.is_empty());
        // Two registrations for one visit are refused.
        assert!(i
            .load_levels_from("garden2", vec![&REG, &REG], build)
            .is_err());
        // A visit a legacy controller owns can never also be registered (F1.4a).
        let map = synthetic_gym_map();
        let mut legacy = Interactions::load(&map).unwrap();
        legacy.gym = Some(crate::gym::Gym::load(&map).unwrap());
        assert!(legacy
            .load_levels_from("skool2", vec![&REG], build)
            .is_err());
        assert!(legacy.levels.is_empty());
        // A controller must answer to the id it was registered under.
        let wrong = |_: &'static Registration| -> Result<Box<dyn LevelController>> {
            Ok(Box::new(WrongId))
        };
        assert!(i.load_levels_from("garden2", vec![&REG], wrong).is_err());
    }
    #[test]
    fn scenes_dialogue_and_outputs_reach_controllers_after_the_legacy_hooks() {
        let map = fixture();
        let mut i = Interactions::load(&map).unwrap();
        let log = Log::default();
        with(
            &mut i,
            Probe {
                scripted: true,
                entry: true,
                prepare: false,
                skip: true,
                ..probe(&log)
            },
            &map,
        );
        assert!(i.scripted());
        let mut story = Story::default();
        i.entry_story(&mut story);
        assert!(!i.prepare_story(&mut story));
        // A dialogue's consequences from the controller are returned with the legacy ones.
        assert_eq!(
            i.completed_dialogue("anything").message.as_deref(),
            Some("dialogue anything")
        );
        // An event output is mirrored to the controller after the legacy handlers.
        let e = i.dispatch(Event::Signal("garden2.out".into()));
        assert_eq!(e.message.as_deref(), Some("out"));
        assert!(log
            .borrow()
            .iter()
            .any(|l| l.starts_with("output Activate")));
        // Scene skipping: a legacy skip wins and the controller is never asked ...
        log.borrow_mut().clear();
        let mut world = World::fixture(&[]);
        let mut player = Player::new(Vec3::ZERO);
        assert!(i
            .skip_after(true, &map, &mut world, &mut player, &mut story)
            .unwrap());
        assert!(log.borrow().is_empty());
        // ... and with no legacy scene the controller's own skip runs.
        assert!(i
            .skip_after(false, &map, &mut world, &mut player, &mut story)
            .unwrap());
        assert_eq!(log.borrow().as_slice(), ["skip"]);
        // Nothing serves the visit any more: every hook reverts to the legacy answer.
        i.levels.clear();
        assert!(!i.scripted() && i.prepare_story(&mut story));
        assert!(!i
            .skip_after(false, &map, &mut world, &mut player, &mut story)
            .unwrap());
    }
    #[test]
    fn legacy_camera_fade_and_scene_answers_come_before_the_registry() {
        let map = fixture();
        let mut i = Interactions::load(&map).unwrap();
        let log = Log::default();
        with(&mut i, probe(&log), &map);
        let world = World::fixture(&[]);
        let legacy = Camera::look(vec3(9., 9., 9.), Vec3::ZERO);
        assert_eq!(
            i.camera_after(Some(legacy), &world).unwrap().eye,
            vec3(9., 9., 9.)
        );
        assert_eq!(i.fade_after(Some((RED, 0.25))).unwrap().1, 0.25);
        assert_eq!(i.scene_id_after(Some("legacy_scene")), Some("legacy_scene"));
        // A legacy answer means the controller is not even asked.
        assert!(log.borrow().is_empty());
        // With no legacy answer the registered scene supplies the camera, fade and skip id.
        assert_eq!(i.camera_after(None, &world).unwrap().eye, vec3(1., 2., 3.));
        assert_eq!(i.fade_after(None).unwrap().1, 0.5);
        assert_eq!(i.scene_id_after(None), Some("probe_scene"));
        assert_eq!(log.borrow().as_slice(), ["camera", "fade", "scene_id"]);
        // Help, ownership, transport and the recovery entrance come from the controller too.
        assert_eq!(i.levels_objective().as_deref(), Some("probe help"));
        assert!(!i.levels_controlled() && !i.levels_transport());
        assert_eq!(
            i.levels_recovery_entry((Vec3::ZERO, 1.)),
            (Vec3::Z * 9., 1.)
        );
        i.levels[0].ctl.downcast_mut::<Probe>().unwrap().scripted = true;
        assert!(i.levels_controlled() && i.levels_transport());
        // No controller: nothing answers.
        i.levels.clear();
        assert!(i.camera_after(None, &world).is_none() && i.fade_after(None).is_none());
        assert!(i.scene_id_after(None).is_none() && i.levels_objective().is_none());
    }
    #[test]
    fn hits_go_to_the_owning_range_and_never_to_legacy_ids() {
        let map = fixture();
        let mut i = Interactions::load(&map).unwrap();
        let log = Log::default();
        with(&mut i, probe(&log), &map);
        let hit = |id| Hit {
            id,
            damage: 1.,
            kind: crate::combat::DamageKind::Other,
            knockback: Vec3::ZERO,
        };
        assert_eq!(i.hit_level(hit(7_300_005)), Some(Some("sound/probe.wav")));
        assert_eq!(log.borrow().as_slice(), ["hit 7300005"]);
        log.borrow_mut().clear();
        for id in [
            crate::school2::ENEMY_BASE + 1,
            crate::interaction::SHOT_BASE + 1,
            crate::encounters::BASE + 7,
            crate::duchess::ID,
            crate::dice::SUMMON,
            crate::dice::ALICE,
            7_299_999,
            7_400_000,
        ] {
            assert!(i.hit_level(hit(id)).is_none(), "id {id}");
        }
        assert!(log.borrow().is_empty());
        // Only targets inside the registration's own range are published.
        assert_eq!(
            i.levels_targets().iter().map(|t| t.id).collect::<Vec<_>>(),
            [7_300_001]
        );
        i.levels_provoke_summon(7_300_002);
        i.levels_provoke_summon(crate::encounters::BASE);
        assert_eq!(log.borrow().as_slice(), ["provoke 7300002"]);
    }
    #[test]
    fn world_shape_from_controllers_reaches_the_collision_world() {
        let map = fixture();
        let mut i = Interactions::load(&map).unwrap();
        with(&mut i, probe(&Log::default()), &map);
        let mut world = World::fixture(&[]);
        i.sync(&mut world);
        assert_eq!(world.liquid_at(vec3(350., 350., 10.)), 0x10);
        assert!(i.transforms().iter().any(|t| t.0 == 1 && t.1 == Vec3::X));
        // The controller's box now blocks a sweep through it.
        assert!(
            world
                .sweep(vec3(-20., 0.5, 0.5), vec3(20., 0.5, 0.5), Vec3::ZERO)
                .fraction
                < 1.
        );
        i.levels.clear();
        i.sync(&mut world);
        assert_eq!(
            world
                .sweep(vec3(-20., 0.5, 0.5), vec3(20., 0.5, 0.5), Vec3::ZERO)
                .fraction,
            1.
        );
    }
    #[test]
    fn an_unhandled_script_thread_reaches_the_registry_before_the_pending_notice() {
        let map = script_trigger();
        let (outside, inside) = (vec3(32., 40., 0.), vec3(32., 100., 0.));
        // Without a controller the thread is pending, exactly as before.
        let mut plain = Interactions::load(&map).unwrap();
        let pending = plain.triggers(0.1, outside, inside);
        assert_eq!(
            pending.message.as_deref(),
            Some("This level event is not playable yet")
        );
        // A controller that knows the thread consumes it.
        let mut served = Interactions::load(&map).unwrap();
        let log = Log::default();
        with(&mut served, probe(&log), &map);
        let handled = served.triggers(0.1, outside, inside);
        assert_eq!(handled.message.as_deref(), Some("hit"));
        assert_eq!(served.touched, 1);
        assert!(log.borrow().iter().any(|l| l == "event probe_thread"));
    }
}

//! Deterministic, bounded world events. Rules are code-owned; saves contain only state.
use crate::entity::{Id, Registry};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const BUDGET: usize = 512;
const QUEUE_LIMIT: usize = 512;
const MAX_DELAY: f64 = 86400.;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Input {
    Activate,
    Touch,
    Use,
    Shot,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    Entity(Id, Input),
    DialogueFinished(String),
    Signal(String),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    Flag(bool),
    Count(u32),
}
#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts(pub BTreeMap<String, Value>);
impl Facts {
    pub fn flag(&mut self, key: &str, value: bool) {
        self.0.insert(key.into(), Value::Flag(value));
    }
    pub fn count(&mut self, key: &str, value: u32) {
        self.0.insert(key.into(), Value::Count(value));
    }
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}
#[derive(Clone, Debug, Serialize)]
pub enum Condition {
    Always,
    Flag(String),
    AtLeast(String, u32),
    Enabled(Id),
    All(Vec<Condition>),
    Any(Vec<Condition>),
    Not(Box<Condition>),
}
impl Condition {
    pub fn flag(name: &str) -> Self {
        Self::Flag(name.into())
    }
    pub fn not(self) -> Self {
        Self::Not(Box::new(self))
    }
    pub fn test(&self, facts: &Facts) -> bool {
        self.evaluate(facts, &Facts::default(), &BTreeMap::new())
    }
    fn evaluate(
        &self,
        derived: &Facts,
        persistent: &Facts,
        entities: &BTreeMap<Id, State>,
    ) -> bool {
        let value = |key: &str| derived.0.get(key).or_else(|| persistent.0.get(key));
        let test = |c: &Condition| c.evaluate(derived, persistent, entities);
        match self {
            Self::Always => true,
            Self::Flag(key) => value(key) == Some(&Value::Flag(true)),
            Self::AtLeast(key, n) => matches!(value(key), Some(Value::Count(v)) if v >= n),
            Self::Enabled(id) => entities.get(id).is_some_and(|s| s.enabled),
            Self::All(cs) => cs.iter().all(test),
            Self::Any(cs) => cs.iter().any(test),
            Self::Not(c) => !test(c),
        }
    }
    fn validate(&self, registry: &Registry, facts: &Facts, depth: usize) -> Result<()> {
        ensure!(depth < 16, "Activation condition is too deep");
        match self {
            Self::Flag(k) => ensure!(
                matches!(facts.0.get(k), Some(Value::Flag(_))),
                "Unknown flag {k}"
            ),
            Self::AtLeast(k, _) => ensure!(
                matches!(facts.0.get(k), Some(Value::Count(_))),
                "Unknown counter {k}"
            ),
            Self::Enabled(id) => registry.validate([*id])?,
            Self::All(cs) | Self::Any(cs) => {
                ensure!(cs.len() <= 64, "Too many conditions");
                for c in cs {
                    c.validate(registry, facts, depth + 1)?;
                }
            }
            Self::Not(c) => c.validate(registry, facts, depth + 1)?,
            Self::Always => (),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    Trigger(Id),
    Activate(Id),
    Door { id: Id, open: bool, locked: bool },
    DialogueFinished(String),
}
#[derive(Clone, Debug, Serialize)]
pub enum Action {
    Flag(String, bool),
    Count(String, u32),
    Enable(Id, bool),
    Send { event: Event, delay: f64 },
    Output(Effect),
}
#[derive(Clone, Debug, Serialize)]
pub struct Rule {
    pub key: String,
    pub event: Event,
    pub condition: Condition,
    pub once: bool,
    pub cooldown: f64,
    pub actions: Vec<Action>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub enabled: bool,
    pub activations: u32,
}
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub count: u32,
    pub last: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Scheduled {
    due: f64,
    order: u64,
    event: Event,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    signature: String,
    pub entities: BTreeMap<Id, State>,
    pub facts: Facts,
    pub usage: BTreeMap<String, Usage>,
    clock: f64,
    next: u64,
    queue: Vec<Scheduled>,
    outbox: Vec<Effect>,
}
pub struct Runtime {
    pub registry: Registry,
    rules: Vec<Rule>,
    derived: Facts,
    initial: Facts,
    state: Snapshot,
}
impl Runtime {
    pub fn new(
        registry: Registry,
        mut rules: Vec<Rule>,
        initial: Facts,
        derived: Facts,
    ) -> Result<Self> {
        ensure!(
            initial.0.keys().all(|k| !derived.0.contains_key(k)),
            "Derived facts cannot shadow puzzle state"
        );
        let mut known = initial.clone();
        known.extend(derived.clone());
        // Rule keys define a stable order independent of registration/hash-map order.
        rules.sort_by(|a, b| a.key.cmp(&b.key));
        ensure!(
            rules.windows(2).all(|r| r[0].key != r[1].key),
            "Duplicate event rule key"
        );
        for r in &rules {
            ensure!(
                !r.key.is_empty() && r.key.len() <= 128 && r.actions.len() <= 128,
                "Invalid event rule"
            );
            validate_event(&r.event, &registry)?;
            r.condition.validate(&registry, &known, 0)?;
            ensure!(
                r.cooldown.is_finite() && (0.0..=MAX_DELAY).contains(&r.cooldown),
                "Invalid event cooldown"
            );
            for a in &r.actions {
                match a {
                    Action::Flag(k, _) => ensure!(
                        matches!(initial.0.get(k), Some(Value::Flag(_))),
                        "Unknown writable flag {k}"
                    ),
                    Action::Count(k, _) => ensure!(
                        matches!(initial.0.get(k), Some(Value::Count(_))),
                        "Unknown writable counter {k}"
                    ),
                    Action::Enable(id, _) => registry.validate([*id])?,
                    Action::Send { event, delay } => {
                        validate_event(event, &registry)?;
                        ensure!(
                            rules.iter().any(|receiver| receiver.event == *event),
                            "No handler registered for {event:?}"
                        );
                        ensure!(
                            delay.is_finite() && (0.0..=MAX_DELAY).contains(delay),
                            "Invalid event delay"
                        );
                    }
                    Action::Output(e) => validate_effect(e, &registry)?,
                }
            }
        }
        let signature = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                &registry, &rules, &initial, &derived
            ))?)
        );
        let state = Snapshot {
            signature,
            entities: registry
                .definitions
                .iter()
                .map(|d| {
                    (
                        d.id,
                        State {
                            enabled: true,
                            activations: 0,
                        },
                    )
                })
                .collect(),
            facts: initial.clone(),
            usage: rules
                .iter()
                .map(|r| (r.key.clone(), Usage::default()))
                .collect(),
            clock: 0.,
            next: 0,
            queue: Vec::new(),
            outbox: Vec::new(),
        };
        Ok(Self {
            registry,
            rules,
            initial,
            derived,
            state,
        })
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    pub fn restore(&mut self, saved: &Snapshot) -> Result<()> {
        ensure!(
            saved.signature == self.state.signature,
            "Saved entity/event definitions do not match this visit"
        );
        ensure!(
            saved.entities.keys().eq(self.state.entities.keys())
                && saved.usage.keys().eq(self.state.usage.keys()),
            "Saved entity/rule identities do not match"
        );
        ensure!(
            same_fact_types(&saved.facts, &self.initial),
            "Invalid saved puzzle variables"
        );
        ensure!(
            saved.clock.is_finite() && (0.0..=1e10).contains(&saved.clock),
            "Invalid saved event clock"
        );
        ensure!(
            saved.queue.len() <= QUEUE_LIMIT && saved.outbox.len() <= BUDGET,
            "Saved event queues are too large"
        );
        let mut orders = BTreeSet::new();
        for p in &saved.queue {
            validate_event(&p.event, &self.registry)?;
            ensure!(
                self.rules.iter().any(|r| r.event == p.event),
                "Unregistered saved event"
            );
            ensure!(
                p.due.is_finite()
                    && p.due >= saved.clock
                    && p.due <= saved.clock + MAX_DELAY
                    && p.order < saved.next
                    && orders.insert(p.order),
                "Invalid saved pending event"
            );
        }
        for r in &self.rules {
            let u = &saved.usage[&r.key];
            ensure!(
                (!r.once || u.count <= 1)
                    && (u.count == 0) == u.last.is_none()
                    && u.last
                        .is_none_or(|t| t.is_finite() && t >= 0. && t <= saved.clock),
                "Invalid saved activation history"
            );
        }
        for e in &saved.outbox {
            validate_effect(e, &self.registry)?;
            ensure!(
                self.rules
                    .iter()
                    .flat_map(|r| &r.actions)
                    .any(|a| matches!(a, Action::Output(v) if v == e)),
                "Unregistered saved event output"
            );
        }
        self.state = saved.clone();
        Ok(())
    }
    pub fn enabled(&self, id: Id) -> bool {
        self.state.entities.get(&id).is_some_and(|s| s.enabled)
    }
    /// Explicit migration from a validated older runtime whose rules remain unchanged.
    pub fn extend_from(&mut self, previous: &Self) -> Result<()> {
        self.extend_gated_from(previous, &[])
    }
    /// Reviewed map migrations may add derived facts and replace named trigger conditions.
    /// Existing actions, event identities, one-shots and cooldowns must still match.
    pub fn extend_gated_from(&mut self, previous: &Self, gates: &[String]) -> Result<()> {
        self.extend(previous, gates, None, &[])
    }
    /// The registry's counterpart of `extend_gated_from` (F2): the generic upgrade of a visit
    /// whose save predates its registered controller. On top of the reviewed gate changes it
    /// lets the controller add writable puzzle facts (they start at their initial values) and
    /// lets an existing rule gain `Send`s that activate the controller's own `receivers`.
    /// Nothing else about an existing rule may change, so one-shots, cooldowns, event
    /// identities and every other action still have to match.
    pub fn extend_registered_from(
        &mut self,
        previous: &Self,
        gates: &[String],
        receivers: &BTreeSet<Id>,
    ) -> Result<()> {
        self.extend(previous, gates, Some(receivers), &[])
    }
    /// Reusable puzzle shot volumes may replace explicitly named former one-shots.
    pub fn extend_repeating_from(&mut self, previous: &Self, gates: &[String], receivers: &BTreeSet<Id>, repeats: &[String]) -> Result<()> {
        self.extend(previous, gates, Some(receivers), repeats)
    }
    /// `grow` is `None` for a legacy migration (the schema and every action must be equal) and
    /// the controller's receivers for a registry upgrade.
    fn extend(
        &mut self,
        previous: &Self,
        gates: &[String],
        grow: Option<&BTreeSet<Id>>,
        repeats: &[String],
    ) -> Result<()> {
        ensure!(
            self.state
                .entities
                .keys()
                .eq(previous.state.entities.keys())
                && match grow {
                    None => self.initial == previous.initial,
                    Some(_) => previous.initial.0.iter().all(|(k, v)| {
                        self.initial
                            .0
                            .get(k)
                            .is_some_and(|w| std::mem::discriminant(v) == std::mem::discriminant(w))
                    }),
                }
                && previous
                    .derived
                    .0
                    .iter()
                    .all(|(k, v)| self.derived.0.get(k) == Some(v)),
            "Event extension changed entity/fact schema"
        );
        for rule in &previous.rules {
            let current = self
                .rules
                .iter()
                .find(|r| r.key == rule.key)
                .context("Event extension removed a rule")?;
            let mut comparable = current.clone();
            if gates.contains(&rule.key) {
                comparable.condition = rule.condition.clone();
            }
            if repeats.contains(&rule.key) && rule.once && !current.once { comparable.once = true; }
            if let Some(receivers) = grow {
                comparable.actions =
                    without_added_sends(&current.actions, &rule.actions, receivers);
            }
            ensure!(
                serde_json::to_value(comparable)? == serde_json::to_value(rule)?,
                "Event extension changed existing rule {}",
                rule.key
            );
        }
        let mut upgraded = previous.snapshot();
        upgraded.signature = self.state.signature.clone();
        if grow.is_some() {
            for (key, value) in &self.initial.0 {
                upgraded
                    .facts
                    .0
                    .entry(key.clone())
                    .or_insert_with(|| value.clone());
            }
        }
        for key in self.state.usage.keys() {
            upgraded.usage.entry(key.clone()).or_default();
        }
        self.restore(&upgraded)
    }
    /// The keys of the rules that answer an event, in rule order.
    pub fn rule_keys(&self, event: &Event) -> Vec<String> {
        self.rules
            .iter()
            .filter(|r| r.event == *event)
            .map(|r| r.key.clone())
            .collect()
    }
    pub fn set_enabled(&mut self, id: Id, enabled: bool) -> Result<()> {
        self.state
            .entities
            .get_mut(&id)
            .context("Unknown entity")?
            .enabled = enabled;
        Ok(())
    }
    pub fn allowed(&self, event: &Event, derived: &Facts) -> bool {
        if matches!(event, Event::Entity(id, _) if !self.enabled(*id)) {
            return false;
        }
        self.rules
            .iter()
            .any(|r| r.event == *event && self.accepts(r, derived))
    }
    fn accepts(&self, r: &Rule, derived: &Facts) -> bool {
        let u = &self.state.usage[&r.key];
        (!r.once || u.count == 0)
            && u.last
                .is_none_or(|t| self.state.clock - t + 1e-9 >= r.cooldown)
            && r.condition
                .evaluate(derived, &self.state.facts, &self.state.entities)
    }
    pub fn post(&mut self, event: Event, delay: f64) -> Result<()> {
        validate_event(&event, &self.registry)?;
        ensure!(
            delay.is_finite() && (0.0..=MAX_DELAY).contains(&delay),
            "Invalid event delay"
        );
        ensure!(
            self.rules.iter().any(|r| r.event == event),
            "No handler registered for {event:?}"
        );
        ensure!(
            self.state.queue.len() < QUEUE_LIMIT,
            "World event queue is full"
        );
        let next = self
            .state
            .next
            .checked_add(1)
            .context("World event sequence overflow")?;
        self.state.queue.push(Scheduled {
            event,
            due: self.state.clock + delay,
            order: self.state.next,
        });
        self.state.next = next;
        Ok(())
    }
    /// The transaction rolls back on malformed data or a relay cycle. No partial effects escape.
    pub fn fire(&mut self, event: Event, derived: &Facts) -> Result<()> {
        let before = self.state.clone();
        let result = self.post(event, 0.).and_then(|_| self.pump(derived));
        if result.is_err() {
            self.state = before;
        }
        result
    }
    pub fn advance(&mut self, dt: f64, derived: &Facts) -> Result<()> {
        ensure!(dt.is_finite() && dt >= 0., "Invalid world event time step");
        ensure!(
            same_fact_types(derived, &self.derived),
            "World event inputs do not match their declarations"
        );
        if dt == 0. {
            return Ok(());
        } // Pause does not dispatch even a due event.
        let end = self.state.clock + dt;
        if self.state.queue.is_empty() {
            ensure!(end <= 1e10, "World event clock overflow");
            self.state.clock = end;
            return Ok(());
        }
        let before = self.state.clone();
        let result = (|| {
            ensure!(end <= 1e10, "World event clock overflow");
            self.pump_until(end, derived)?;
            self.state.clock = end;
            Ok(())
        })();
        if result.is_err() {
            self.state = before;
        }
        result
    }
    fn pump(&mut self, derived: &Facts) -> Result<()> {
        self.pump_until(self.state.clock, derived)
    }
    fn pump_until(&mut self, end: f64, derived: &Facts) -> Result<()> {
        ensure!(
            same_fact_types(derived, &self.derived),
            "World event inputs do not match their declarations"
        );
        let mut cost = 0;
        loop {
            let index = self
                .state
                .queue
                .iter()
                .enumerate()
                .filter(|(_, p)| p.due <= end)
                .min_by(|(_, a), (_, b)| a.due.total_cmp(&b.due).then(a.order.cmp(&b.order)))
                .map(|(i, _)| i);
            let Some(index) = index else {
                break;
            };
            cost += 1;
            ensure!(
                cost <= BUDGET,
                "World event cycle or dispatch budget exceeded"
            );
            let p = self.state.queue.remove(index);
            self.state.clock = p.due.max(self.state.clock);
            if matches!(p.event, Event::Entity(id, _) if !self.enabled(id)) {
                continue;
            }
            let matches = self
                .rules
                .iter()
                .enumerate()
                .filter_map(|(i, r)| (r.event == p.event).then_some(i))
                .collect::<Vec<_>>();
            for index in matches {
                let r = &self.rules[index];
                if !self.accepts(r, derived) {
                    continue;
                }
                cost += r.actions.len();
                ensure!(cost <= BUDGET, "World event dispatch budget exceeded");
                let usage = self.state.usage.get_mut(&r.key).unwrap();
                usage.count = usage
                    .count
                    .checked_add(1)
                    .context("Activation counter overflow")?;
                usage.last = Some(self.state.clock);
                if let Event::Entity(id, _) = p.event {
                    let e = self.state.entities.get_mut(&id).unwrap();
                    e.activations = e
                        .activations
                        .checked_add(1)
                        .context("Entity activation counter overflow")?;
                }
                for a in r.actions.clone() {
                    match a {
                        Action::Flag(k, v) => self.state.facts.flag(&k, v),
                        Action::Count(k, n) => {
                            let Value::Count(v) = self.state.facts.0.get_mut(&k).unwrap() else {
                                unreachable!()
                            };
                            *v = v.checked_add(n).context("Puzzle counter overflow")?;
                        }
                        Action::Enable(id, v) => self.set_enabled(id, v)?,
                        Action::Send { event, delay } => self.post(event, delay)?,
                        Action::Output(e) => {
                            ensure!(
                                self.state.outbox.len() < BUDGET,
                                "World event output queue is full"
                            );
                            self.state.outbox.push(e);
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn take_outputs(&mut self) -> Vec<Effect> {
        std::mem::take(&mut self.state.outbox)
    }
    /// v1 saves had already-consumed triggers/dialogue in their component state.
    pub fn import_consumed(&mut self, key: &str) {
        if let Some(u) = self.state.usage.get_mut(key) {
            u.count = 1;
            u.last = Some(self.state.clock);
        }
    }
    pub fn import_unhandled(&mut self, key: &str) {
        if let Some(u) = self.state.usage.get_mut(key) {
            *u = Usage::default();
        }
    }
    pub fn import_flag(&mut self, key: &str, value: bool) {
        if matches!(self.initial.0.get(key), Some(Value::Flag(_))) {
            self.state.facts.flag(key, value);
        }
    }
    pub fn import_count(&mut self, key: &str, value: u32) {
        if matches!(self.initial.0.get(key), Some(Value::Count(_))) {
            self.state.facts.count(key, value);
        }
    }
}
/// `current` without the activation sends to `receivers` that `previous` does not have. The
/// actions both share must appear in the same order; anything else stays in the result, so a
/// real change still fails the caller's comparison.
fn without_added_sends(
    current: &[Action],
    previous: &[Action],
    receivers: &BTreeSet<Id>,
) -> Vec<Action> {
    let same = |a: &Action, b: &Action| matches!((serde_json::to_value(a), serde_json::to_value(b)), (Ok(a), Ok(b)) if a == b);
    let mut old = previous.iter().peekable();
    let mut out = Vec::new();
    for action in current {
        if old.peek().is_some_and(|p| same(p, action)) {
            old.next();
            out.push(action.clone());
        } else if matches!(action, Action::Send { event: Event::Entity(id, Input::Activate), .. } if receivers.contains(id))
        {
            continue;
        } else {
            out.push(action.clone());
        }
    }
    out
}
fn same_fact_types(a: &Facts, b: &Facts) -> bool {
    a.0.len() == b.0.len()
        && a.0.iter().all(|(k, v)| {
            b.0.get(k)
                .is_some_and(|w| std::mem::discriminant(v) == std::mem::discriminant(w))
        })
}
fn validate_event(e: &Event, r: &Registry) -> Result<()> {
    match e {
        Event::Entity(id, _) => r.validate([*id])?,
        Event::DialogueFinished(s) | Event::Signal(s) => {
            ensure!(!s.is_empty() && s.len() <= 128, "Invalid event name")
        }
    }
    Ok(())
}
fn validate_effect(e: &Effect, r: &Registry) -> Result<()> {
    match e {
        Effect::Activate(id) | Effect::Trigger(id) | Effect::Door { id, .. } => {
            r.validate([*id])?
        }
        Effect::DialogueFinished(s) => {
            ensure!(!s.is_empty() && s.len() <= 128, "Invalid dialogue event")
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rule(
        key: &str,
        event: Event,
        condition: Condition,
        once: bool,
        actions: Vec<Action>,
    ) -> Rule {
        Rule {
            key: key.into(),
            event,
            condition,
            once,
            cooldown: 0.,
            actions,
        }
    }
    fn registry() -> Registry {
        Registry::new(&[BTreeMap::new(), BTreeMap::new()])
    }
    fn signal(s: &str) -> Event {
        Event::Signal(s.into())
    }
    #[test]
    fn explicitly_rearmed_shots_keep_history_without_relaxing_other_rules() {
        let rules=vec![rule("portrait",signal("hit"),Condition::Always,true,vec![Action::Output(Effect::Activate(Id(0)))])];
        let make=|r|Runtime::new(registry(),r,Facts::default(),Facts::default()).unwrap();
        let mut old=make(rules.clone());old.fire(signal("hit"),&Facts::default()).unwrap();old.take_outputs();
        let mut repeat=rules.clone();repeat[0].once=false;
        assert!(make(repeat.clone()).extend_registered_from(&old,&[],&BTreeSet::new()).is_err());
        let mut current=make(repeat.clone());current.extend_repeating_from(&old,&[],&BTreeSet::new(),&["portrait".into()]).unwrap();
        assert_eq!(current.state.usage["portrait"].count,1);
        current.fire(signal("hit"),&Facts::default()).unwrap();assert_eq!(current.take_outputs().len(),1);
        repeat[0].cooldown=2.;assert!(make(repeat).extend_repeating_from(&old,&[],&BTreeSet::new(),&["portrait".into()]).is_err());
    }
    #[test]
    fn extending_rules_preserves_pending_events_and_one_shot_history() {
        let rules = vec![rule(
            "old",
            signal("old"),
            Condition::Always,
            true,
            vec![Action::Output(Effect::Activate(Id(0)))],
        )];
        let make =
            |rules| Runtime::new(registry(), rules, Facts::default(), Facts::default()).unwrap();
        let mut old = make(rules.clone());
        old.post(signal("old"), 2.).unwrap();
        old.advance(0.5, &Facts::default()).unwrap();
        let mut expanded = rules.clone();
        expanded.push(rule(
            "new",
            signal("new"),
            Condition::Always,
            true,
            vec![Action::Output(Effect::Activate(Id(1)))],
        ));
        let mut current = make(expanded.clone());
        current.extend_from(&old).unwrap();
        assert_eq!(current.state.queue, old.state.queue);
        current.advance(1.5, &Facts::default()).unwrap();
        assert_eq!(current.take_outputs(), vec![Effect::Activate(Id(0))]);
        current.fire(signal("old"), &Facts::default()).unwrap();
        assert!(current.take_outputs().is_empty());
        current.fire(signal("new"), &Facts::default()).unwrap();
        assert_eq!(current.take_outputs(), vec![Effect::Activate(Id(1))]);
        old.advance(1.5, &Facts::default()).unwrap();
        old.take_outputs();
        let mut migrated = make(expanded.clone());
        migrated.extend_from(&old).unwrap();
        migrated.fire(signal("old"), &Facts::default()).unwrap();
        assert!(migrated.take_outputs().is_empty());
        expanded[0].once = false;
        assert!(make(expanded).extend_from(&old).is_err());
    }
    #[test]
    fn approved_gate_migration_preserves_queue_but_rejects_unrelated_changes() {
        let old_rule = rule(
            "door",
            signal("open"),
            Condition::Always,
            true,
            vec![Action::Output(Effect::Activate(Id(1)))],
        );
        let mut old = Runtime::new(
            registry(),
            vec![old_rule.clone()],
            Facts::default(),
            Facts::default(),
        )
        .unwrap();
        old.post(signal("open"), 1.).unwrap();
        let mut facts = Facts::default();
        facts.flag("key", false);
        let mut changed = old_rule;
        changed.condition = Condition::flag("key");
        let make = |r| Runtime::new(registry(), vec![r], Facts::default(), facts.clone()).unwrap();
        assert!(make(changed.clone()).extend_from(&old).is_err());
        let mut new = make(changed.clone());
        new.extend_gated_from(&old, &["door".into()]).unwrap();
        new.advance(1., &facts).unwrap();
        assert!(new.take_outputs().is_empty());
        facts.flag("key", true);
        new.fire(signal("open"), &facts).unwrap();
        assert_eq!(new.take_outputs(), vec![Effect::Activate(Id(1))]);
        changed.actions.clear();
        let mut bad = Runtime::new(registry(), vec![changed], Facts::default(), facts).unwrap();
        assert!(bad.extend_gated_from(&old, &["door".into()]).is_err());
    }
    #[test]
    fn gated_puzzle_counter_does_not_consume_early_input_and_fires_once() {
        let mut facts = Facts::default();
        facts.flag("power", false);
        facts.count("switches", 0);
        let rules = vec![
            rule(
                "power",
                signal("power"),
                Condition::Always,
                true,
                vec![Action::Flag("power".into(), true)],
            ),
            rule(
                "count",
                signal("switch"),
                Condition::Always,
                false,
                vec![Action::Count("switches".into(), 1)],
            ),
            rule(
                "open",
                signal("open"),
                Condition::All(vec![
                    Condition::flag("power"),
                    Condition::Any(vec![
                        Condition::AtLeast("switches".into(), 2),
                        Condition::flag("power").not(),
                    ]),
                    Condition::Enabled(Id(1)),
                ]),
                true,
                vec![Action::Output(Effect::Activate(Id(1)))],
            ),
        ];
        let mut r = Runtime::new(registry(), rules, facts, Facts::default()).unwrap();
        for event in [
            signal("open"),
            signal("power"),
            signal("switch"),
            signal("open"),
        ] {
            r.fire(event, &Facts::default()).unwrap();
        }
        assert!(r.take_outputs().is_empty());
        assert_eq!(r.state.usage["open"].count, 0);
        r.fire(signal("switch"), &Facts::default()).unwrap();
        r.fire(signal("open"), &Facts::default()).unwrap();
        assert_eq!(r.take_outputs(), vec![Effect::Activate(Id(1))]);
        r.fire(signal("open"), &Facts::default()).unwrap();
        assert!(r.take_outputs().is_empty());
    }
    #[test]
    fn delayed_fifo_chain_and_pause_survive_json_restoration_at_any_frame_rate() {
        let rules = vec![
            rule(
                "first",
                signal("start"),
                Condition::Always,
                true,
                vec![
                    Action::Send {
                        event: signal("a"),
                        delay: 0.5,
                    },
                    Action::Send {
                        event: signal("b"),
                        delay: 0.5,
                    },
                ],
            ),
            rule(
                "a",
                signal("a"),
                Condition::Always,
                true,
                vec![
                    Action::Output(Effect::Activate(Id(0))),
                    Action::Send {
                        event: signal("c"),
                        delay: 0.25,
                    },
                ],
            ),
            rule(
                "b",
                signal("b"),
                Condition::Always,
                true,
                vec![Action::Output(Effect::Activate(Id(1)))],
            ),
            rule(
                "c",
                signal("c"),
                Condition::Always,
                true,
                vec![Action::Output(Effect::Door {
                    id: Id(0),
                    open: true,
                    locked: false,
                })],
            ),
        ];
        let make = || {
            Runtime::new(
                registry(),
                rules.clone(),
                Facts::default(),
                Facts::default(),
            )
            .unwrap()
        };
        let mut original = make();
        original.fire(signal("start"), &Facts::default()).unwrap();
        original.advance(0.25, &Facts::default()).unwrap();
        let saved: Snapshot =
            serde_json::from_slice(&serde_json::to_vec(&original.snapshot()).unwrap()).unwrap();
        for hz in [30, 60, 144] {
            let mut r = make();
            r.restore(&saved).unwrap();
            r.advance(0., &Facts::default()).unwrap();
            assert_eq!(r.snapshot(), saved);
            for _ in 0..hz {
                r.advance(1. / hz as f64, &Facts::default()).unwrap();
            }
            assert_eq!(
                r.take_outputs(),
                vec![
                    Effect::Activate(Id(0)),
                    Effect::Activate(Id(1)),
                    Effect::Door {
                        id: Id(0),
                        open: true,
                        locked: false
                    }
                ]
            );
            assert_eq!(r.state.usage["a"].last, Some(0.5));
            assert_eq!(r.state.usage["c"].last, Some(0.75));
        }
    }
    #[test]
    fn disabled_receivers_drop_events_and_cooldowns_survive_restore() {
        let event = Event::Entity(Id(0), Input::Use);
        let mut rule = rule(
            "use",
            event.clone(),
            Condition::Always,
            false,
            vec![Action::Output(Effect::Activate(Id(1)))],
        );
        rule.cooldown = 1.;
        let make = || {
            Runtime::new(
                registry(),
                vec![rule.clone()],
                Facts::default(),
                Facts::default(),
            )
            .unwrap()
        };
        let mut r = make();
        r.set_enabled(Id(0), false).unwrap();
        r.post(event.clone(), 0.25).unwrap();
        r.advance(0.5, &Facts::default()).unwrap();
        assert!(r.take_outputs().is_empty());
        assert_eq!(r.state.usage["use"].count, 0);
        r.set_enabled(Id(0), true).unwrap();
        r.fire(event.clone(), &Facts::default()).unwrap();
        r.take_outputs();
        let mut restored = make();
        restored.restore(&r.snapshot()).unwrap();
        restored.fire(event.clone(), &Facts::default()).unwrap();
        assert!(restored.take_outputs().is_empty());
        restored.advance(1., &Facts::default()).unwrap();
        restored.fire(event, &Facts::default()).unwrap();
        assert_eq!(restored.take_outputs(), vec![Effect::Activate(Id(1))]);
    }
    #[test]
    fn relay_cycle_rolls_back_flags_history_queue_and_outputs() {
        let mut facts = Facts::default();
        facts.flag("changed", false);
        let mut r = Runtime::new(
            registry(),
            vec![rule(
                "loop",
                signal("loop"),
                Condition::Always,
                false,
                vec![
                    Action::Flag("changed".into(), true),
                    Action::Output(Effect::Activate(Id(0))),
                    Action::Send {
                        event: signal("loop"),
                        delay: 0.,
                    },
                ],
            )],
            facts,
            Facts::default(),
        )
        .unwrap();
        let before = r.snapshot();
        assert!(r.fire(signal("loop"), &Facts::default()).is_err());
        assert_eq!(r.snapshot(), before);
        r.post(signal("loop"), 0.25).unwrap();
        let before = r.snapshot();
        assert!(r.advance(1., &Facts::default()).is_err());
        assert_eq!(r.snapshot(), before);
    }
    #[test]
    fn malformed_snapshot_and_definition_changes_are_rejected_without_partial_restore() {
        let rules = vec![rule(
            "test",
            signal("test"),
            Condition::Always,
            true,
            vec![Action::Output(Effect::Activate(Id(1)))],
        )];
        let make = || {
            Runtime::new(
                registry(),
                rules.clone(),
                Facts::default(),
                Facts::default(),
            )
            .unwrap()
        };
        let mut r = make();
        r.post(signal("test"), 1.).unwrap();
        let good = r.snapshot();
        let mut bad = good.clone();
        bad.queue[0].due = -1.;
        assert!(r.restore(&bad).is_err());
        assert_eq!(r.snapshot(), good);
        let mut bad = good.clone();
        bad.entities.remove(&Id(1));
        assert!(r.restore(&bad).is_err());
        let mut bad = good.clone();
        bad.queue[0].event = signal("unregistered");
        assert!(r.restore(&bad).is_err());
        let mut bad = good.clone();
        bad.outbox.push(Effect::Activate(Id(0)));
        assert!(r.restore(&bad).is_err());
        let mut changed = rules;
        changed[0].once = false;
        let mut other =
            Runtime::new(registry(), changed, Facts::default(), Facts::default()).unwrap();
        assert!(other.restore(&good).is_err());
    }
    #[test]
    fn definitions_reject_missing_entities_unknown_flags_and_duplicate_rule_keys() {
        let bad = rule("bad", signal("bad"), Condition::flag("typo"), true, vec![]);
        assert!(Runtime::new(registry(), vec![bad], Facts::default(), Facts::default()).is_err());
        let r = rule(
            "test",
            Event::Entity(Id(9), Input::Touch),
            Condition::Always,
            true,
            vec![],
        );
        assert!(Runtime::new(registry(), vec![r], Facts::default(), Facts::default()).is_err());
        let r = rule("test", signal("test"), Condition::Always, true, vec![]);
        assert!(Runtime::new(
            registry(),
            vec![r.clone(), r],
            Facts::default(),
            Facts::default()
        )
        .is_err());
        let r = rule(
            "broken link",
            signal("test"),
            Condition::Always,
            false,
            vec![Action::Send {
                event: signal("missing"),
                delay: 0.,
            }],
        );
        assert!(Runtime::new(registry(), vec![r], Facts::default(), Facts::default()).is_err());
    }

    fn three() -> Registry {
        Registry::new(&[BTreeMap::new(), BTreeMap::new(), BTreeMap::new()])
    }
    fn activate(id: usize) -> Event {
        Event::Entity(Id(id), Input::Activate)
    }
    fn send(id: usize) -> Action {
        Action::Send {
            event: activate(id),
            delay: 0.,
        }
    }
    /// The shipped program: one relay rule that reports and one that a lever will join.
    fn shipped() -> Vec<Rule> {
        vec![
            rule(
                "relay",
                signal("relay"),
                Condition::Always,
                true,
                vec![Action::Output(Effect::Activate(Id(0)))],
            ),
            rule("plain", activate(1), Condition::Always, false, vec![]),
        ]
    }
    /// The program after a controller joined: entity 2 is its receiver, `relay` now wakes it,
    /// and a writable flag and a rule that sets it are new.
    fn joined(open: bool) -> (Vec<Rule>, Facts) {
        let mut rules = shipped();
        rules[0].actions.insert(0, send(2));
        rules.push(rule(
            "ctl/lever",
            activate(2),
            Condition::Always,
            false,
            vec![Action::Flag("ctl.open".into(), true)],
        ));
        let mut initial = Facts::default();
        initial.flag("ctl.open", open);
        (rules, initial)
    }
    fn registry_from(rules: Vec<Rule>, initial: Facts) -> Result<Runtime> {
        Runtime::new(three(), rules, initial, Facts::default())
    }
    #[test]
    fn a_registered_controller_may_add_writable_facts_and_receiver_sends_and_nothing_else() {
        let mut old = registry_from(shipped(), Facts::default()).unwrap();
        old.fire(signal("relay"), &Facts::default()).unwrap();
        old.take_outputs();
        old.post(signal("relay"), 4.).unwrap();
        let receivers = BTreeSet::from([Id(2)]);
        let (rules, initial) = joined(false);
        let mut new = registry_from(rules, initial.clone()).unwrap();
        // The legacy extension is deliberately strict: a grown schema is still refused.
        assert!(new.extend_gated_from(&old, &[]).is_err());
        new.extend_registered_from(&old, &[], &receivers).unwrap();
        let state = new.snapshot();
        // The old history survives (the one-shot stays spent, the queued event stays queued)
        // and the controller's fact starts at its initial value.
        assert_eq!(state.usage["relay"].count, 1);
        assert_eq!(state.usage["ctl/lever"], Usage::default());
        assert_eq!(state.facts.0["ctl.open"], Value::Flag(false));
        assert_eq!(state.queue, old.state.queue);
        assert_eq!(state.signature, new.state.signature);
        // The new rule runs, and the spent one-shot stays spent even though it gained a send.
        new.fire(activate(2), &Facts::default()).unwrap();
        assert_eq!(new.state.facts.0["ctl.open"], Value::Flag(true));
        new.fire(signal("relay"), &Facts::default()).unwrap();
        assert!(new.take_outputs().is_empty());

        // Only sends that activate the controller's own receivers may be added.
        let (mut rules, initial) = joined(false);
        rules[0].actions[0] = send(1);
        let mut other = registry_from(rules, initial.clone()).unwrap();
        assert!(other.extend_registered_from(&old, &[], &receivers).is_err());
        let mut none = registry_from(joined(false).0, initial.clone()).unwrap();
        assert!(none
            .extend_registered_from(&old, &[], &BTreeSet::new())
            .is_err());
        // A removed action, a changed one-shot, a changed condition and a removed rule fail.
        for (label, change) in [
            ("removed action", 0),
            ("one-shot", 1),
            ("condition", 2),
            ("removed rule", 3),
        ] {
            let (mut rules, initial) = joined(false);
            match change {
                0 => rules[0].actions.retain(|a| !matches!(a, Action::Output(_))),
                1 => rules[0].once = false,
                2 => rules[1].condition = Condition::flag("ctl.open"),
                _ => rules.retain(|r| r.key != "plain"),
            }
            let Ok(mut bad) = registry_from(rules, initial) else {
                continue;
            };
            assert!(
                bad.extend_registered_from(&old, &[], &receivers).is_err(),
                "{label}"
            );
        }
        // A condition may change only where the controller names the rule.
        let (mut rules, initial) = joined(false);
        rules[1].condition = Condition::flag("ctl.open");
        let mut gated = registry_from(rules, initial).unwrap();
        assert!(gated.extend_registered_from(&old, &[], &receivers).is_err());
        gated
            .extend_registered_from(&old, &["plain".into()], &receivers)
            .unwrap();
    }
    #[test]
    fn a_registry_extension_never_drops_or_retypes_an_existing_writable_fact() {
        let mut facts = Facts::default();
        facts.flag("old.flag", false);
        facts.count("old.count", 3);
        let make = |rules, initial| Runtime::new(three(), rules, initial, Facts::default());
        let mut old = make(shipped(), facts.clone()).unwrap();
        old.import_count("old.count", 2);
        // Growth alone keeps the old values.
        let mut grown = facts.clone();
        grown.flag("new.flag", true);
        let mut new = make(shipped(), grown).unwrap();
        new.extend_registered_from(&old, &[], &BTreeSet::new())
            .unwrap();
        assert_eq!(new.state.facts.0["old.count"], Value::Count(2));
        assert_eq!(new.state.facts.0["new.flag"], Value::Flag(true));
        // A fact that disappears, or changes type, is a different puzzle.
        let mut dropped = facts.clone();
        dropped.0.remove("old.flag");
        assert!(make(shipped(), dropped)
            .unwrap()
            .extend_registered_from(&old, &[], &BTreeSet::new())
            .is_err());
        let mut retyped = facts.clone();
        retyped.count("old.flag", 0);
        assert!(make(shipped(), retyped)
            .unwrap()
            .extend_registered_from(&old, &[], &BTreeSet::new())
            .is_err());
    }
    #[test]
    fn the_rules_that_answer_an_event_are_listed_in_rule_order() {
        let (rules, initial) = joined(false);
        let runtime = registry_from(rules, initial).unwrap();
        assert_eq!(runtime.rule_keys(&activate(2)), ["ctl/lever"]);
        assert_eq!(runtime.rule_keys(&activate(1)), ["plain"]);
        assert!(runtime.rule_keys(&activate(0)).is_empty());
        assert_eq!(runtime.rule_keys(&signal("relay")), ["relay"]);
    }
}

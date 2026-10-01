//! World entities plus explicit school progression; unhandled scripts stay visible as pending.
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    movement::Player,
};
use crate::{
    entity::{Id, Registry},
    event::{self, Action, Condition, Effect, Event, Facts, Input, Rule, Runtime},
};
use anyhow::Result;
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
type Entity = BTreeMap<String, String>;
mod exits;
pub mod friendly;
mod hooks;
pub(crate) mod school2_check;
mod upgrade;
/// Bounded support for the school's theatre floor, before cinematic movers exist.
/// Both sets are loaded for drawing, but only the entry's set is active.
pub fn school_platform(e: &Entity) -> Option<bool> {
    if e.get("classname").map(String::as_str) != Some("script_object") {
        return None;
    }
    let name = e.get("targetname")?;
    for (prefix, returning) in [("theatre_plat", false), ("skip_plat", true)] {
        if name
            .strip_prefix(prefix)
            .is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8"))
        {
            return Some(returning);
        }
    }
    None
}
pub fn vector(s: &str) -> Option<Vec3> {
    let v = s
        .split_whitespace()
        .map(str::parse::<f32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    if v.len() != 3 || v.iter().any(|v| !v.is_finite()) {
        None
    } else {
        Some(vec3(v[0], v[1], v[2]))
    }
}
fn value(e: &Entity, key: &str, default: f32) -> f32 {
    e.get(key)
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}
fn model(e: &Entity, map: &Bsp) -> Option<usize> {
    e.get("model")?
        .strip_prefix('*')?
        .parse::<usize>()
        .ok()
        .filter(|&i| i > 0 && i < map.models.len())
}
pub fn destination(s: &str) -> Option<(String, Option<String>)> {
    let mut p = s.split('$');
    let map = p.next()?;
    let spawn = p.next();
    if p.next().is_some()
        || map.is_empty()
        || !map.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        || spawn.is_some_and(|s| {
            s.is_empty() || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    {
        return None;
    }
    Some((map.into(), spawn.map(str::to_owned)))
}
pub fn spawn(map: &Bsp, name: Option<&str>) -> (Vec3, f32) {
    // This visit has no arrival performance. Settle its elevated editor start
    // before drawing Alice, including when entering through the chapter chooser.
    if name.is_none() || name == Some("hedge3_start1") {
        if let Some(e) = map.entities.iter().find(|e| {
            e.get("classname").is_some_and(|s| s == "info_player_start")
                && e.get("targetname").is_some_and(|s| s == "hedge3_start1")
        }) {
            if let Some(at) = e.get("origin").and_then(|s| vector(s)) {
                if let Some(feet) = World::from_bsp(map).ok().and_then(|w| {
                    w.actor_footing(at, PLAYER_CENTER, PLAYER_HALF, 64.)
                }) {
                    return (feet + Vec3::Z * 48., value(e, "angle", 0.).to_radians());
                }
            }
        }
    }
    // This arrival's collapsing approach belongs to an unfinished cinematic.
    // Its end marker is the supported hand-off, as with the village intro.
    if (name.is_none() || name == Some("grounds2_start1"))
        && map.entities.iter().any(|e| {
            e.get("classname").is_some_and(|s| s == "info_player_start")
                && e.get("targetname").is_some_and(|s| s == "grounds2_start1")
        })
    {
        if let Some(e) = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == "alicestop"))
        {
            if let Some(p) = e.get("origin").and_then(|s| vector(s)) {
                return (p + Vec3::Z * 48., value(e, "angle", 90.).to_radians());
            }
        }
    }
    // The airship cinematic starts offstage. Enter at its authored hand-off.
    if (name.is_none() || name == Some("fortress1_start1"))
        && map
            .entities
            .iter()
            .any(|e| e.get("thread").is_some_and(|s| s == "Fortress1_Start"))
    {
        if let Some(e) = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == "camerarest"))
        {
            if let Some(p) = e.get("origin").and_then(|s| vector(s)) {
                return (p + Vec3::Z * 48., value(e, "angle", 180.).to_radians());
            }
        }
    }
    // The village's normal start is inside a scripted falling cinematic. Its
    // authored end-of-intro marker is the actual playable arrival point.
    if name.is_none()
        && map.entities.iter().any(|e| {
            e.get("thread")
                .is_some_and(|s| s == "Gvillage_RabbitHole_Start")
        })
    {
        if let Some(e) = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == "alice_cat_talk1"))
        {
            if let Some(p) = e.get("origin").and_then(|s| vector(s)) {
                return (p + Vec3::Z * 48., value(e, "angle", 0.).to_radians());
            }
        }
    }
    if let Some(name) = name {
        if let Some(e) = map.entities.iter().find(|e| {
            e.get("classname").is_some_and(|s| s == "info_player_start")
                && e.get("targetname").is_some_and(|s| s == name)
        }) {
            if let Some(p) = e.get("origin").and_then(|s| vector(s)) {
                return (p + Vec3::Z * 48., value(e, "angle", 0.).to_radians());
            }
        }
    }
    map.spawn()
}

struct Door {
    id: Id,
    name: String,
    enabled: bool,
    model: usize,
    origin: Vec3,
    center: Vec3,
    angle: f32,
    target: f32,
    duration: f32,
    sign: f32,
    locked: bool,
    collider: Collider,
    sound: Option<String>,
}
impl Door {
    fn rotation(&self) -> Quat {
        Quat::from_rotation_z(self.angle.to_radians())
    }
}
#[derive(Clone)]
enum TriggerKind {
    Dialogue(String),
    Exit(String, Option<String>),
    Teleport(Vec3, f32),
    Hurt(f32),
    Fall,
    Script(String),
}
struct Trigger {
    id: Id,
    name: String,
    volume: Collider,
    kind: TriggerKind,
    inside: bool,
    cooldown: f32,
    reported: bool,
    target: Option<String>,
    once: bool,
    fired: bool,
    health: f32,
    bounds: (Vec3, Vec3),
}
pub struct Interactions {
    pub presentation: crate::sky_sequence::Controller,
    pub event_world: Runtime,
    gym_entity: Option<Id>,
    map_name: String,
    pub school: Option<crate::school::School>,
    pub gym: Option<crate::gym::Gym>,
    pub school2: Option<crate::school2::School2>,
    pub village: Option<crate::village::Village>,
    pub fortress: Option<crate::fortress::Fortress>,
    pub beyond: Option<crate::beyond::Beyond>,
    pub pool: Option<crate::pool::Pool>,
    pub pandemonium: Option<crate::pandemonium::Pandemonium>,
    pub duchess: Option<crate::duchess::Duchess>,
    pub encounters: Option<crate::encounters::Encounters>,
    /// Registered level controllers (`levels::LEVELS`), consulted after every legacy controller.
    pub levels: Vec<crate::level::Slot>,
    activations: Vec<String>,
    doors: Vec<Door>,
    platforms: Vec<(usize, Vec3, Collider)>,
    triggers: Vec<Trigger>,
    pub used: u64,
    pub touched: u64,
}
#[derive(Default)]
pub struct Events {
    pub story: Vec<String>,
    pub message: Option<String>,
    pub sound: Option<String>,
    pub transition: Option<(String, Option<String>)>,
    pub teleport: Option<(Vec3, f32)>,
    pub damage: f32,
}
impl Events {
    pub fn merge(&mut self, other: Self) {
        self.story.extend(other.story);
        if other.message.is_some() {
            self.message = other.message;
        }
        if other.sound.is_some() {
            self.sound = other.sound;
        }
        if other.transition.is_some() {
            self.transition = other.transition;
        }
        if other.teleport.is_some() {
            self.teleport = other.teleport;
        }
        self.damage = self.damage.max(other.damage);
    }
}
impl Interactions {
    pub fn loot_sources(&self) -> Vec<crate::loot::Source> {
        let mut out = self
            .encounters
            .as_ref()
            .map_or(Vec::new(), |s| s.loot_sources());
        if let Some(s) = &self.school2 {
            out.extend(s.loot_sources());
        }
        if let Some(d) = &self.duchess {
            out.push(crate::loot::Source {
                id: crate::duchess::ID,
                feet: d.state.feet,
                grade: crate::loot::Grade::Super,
                dead: d.state.health <= 0.,
            });
        }
        out.extend(self.level_loot_sources());
        out
    }

    fn init_traversal(&mut self, map: &Bsp) -> Result<()> {
        for (index, e) in map.entities.iter().enumerate() {
            if crate::traversal::initially_disabled(&self.map_name, e) {
                self.event_world.set_enabled(Id(index), false)?;
            }
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn empty() -> Self {
        Self {
            presentation: Default::default(),
            event_world: Runtime::new(
                Registry::default(),
                vec![],
                Facts::default(),
                Facts::default(),
            )
            .unwrap(),
            gym_entity: None,
            map_name: String::new(),
            school: None,
            gym: None,
            school2: None,
            village: None,
            fortress: None,
            beyond: None,
            pool: None,
            pandemonium: None,
            duchess: None,
            encounters: None,
            levels: Vec::new(),
            activations: Vec::new(),
            doors: Vec::new(),
            platforms: Vec::new(),
            triggers: Vec::new(),
            used: 0,
            touched: 0,
        }
    }
    pub fn load(map: &Bsp) -> Result<Self> {
        let mut doors = Vec::new();
        let mut triggers = Vec::new();
        for (entity_index, e) in map.entities.iter().enumerate() {
            let Some(index) = model(e, map) else {
                continue;
            };
            let class = e.get("classname").map(String::as_str).unwrap_or("");
            let origin = e
                .get("origin")
                .and_then(|s| vector(s))
                .unwrap_or(Vec3::ZERO);
            if class == "func_rotatingdoor" {
                // Authored geometry is already oriented around this hinge. `angle` is not an initial transform.
                let bounds = &map.models[index];
                let center = origin + (bounds.min + bounds.max) * 0.5;
                let flags = value(e, "spawnflags", 0.) as i32;
                doors.push(Door {
                    id: Id(entity_index),
                    name: e.get("targetname").cloned().unwrap_or_default(),
                    enabled: true,
                    model: index,
                    origin,
                    center,
                    angle: 0.,
                    target: 0.,
                    duration: value(e, "time", 1.).clamp(0.1, 20.),
                    sign: 1.,
                    locked: flags & 4096 != 0,
                    collider: Collider::model(map, index, origin, Quat::IDENTITY, true)?,
                    sound: e.get("sound_move").cloned(),
                });
                continue;
            }
            let kind = match class {
                "trigger_catmessage" => e.get("target").map(|s| TriggerKind::Dialogue(s.clone())),
                "trigger_changelevel" => e
                    .get("map")
                    .and_then(|s| destination(s))
                    .map(|(m, s)| TriggerKind::Exit(m, s)),
                "trigger_teleport" => e
                    .get("target")
                    .and_then(|name| {
                        map.entities.iter().find(|t| {
                            t.get("targetname") == Some(name)
                                && t.get("classname").is_some_and(|c| c == "func_teleportdest")
                        })
                    })
                    .and_then(|target| {
                        Some(TriggerKind::Teleport(
                            vector(target.get("origin")?)?,
                            value(target, "angle", 0.).to_radians(),
                        ))
                    }),
                "trigger_hurt" => Some(TriggerKind::Hurt(
                    value(e, "setdamage", value(e, "damage", 10.)).clamp(0., 10000.),
                )),
                // Local TriggerFall callback emits KILLED_FALLING for the player.
                // Implement its fatal outcome, not the original cinematic/timing.
                "trigger_fall" => Some(TriggerKind::Fall),

                "trigger_once" | "trigger_multiple" => (e.contains_key("thread")
                    || e.contains_key("target"))
                .then(|| TriggerKind::Script(e.get("thread").cloned().unwrap_or_default())),
                _ => None,
            };
            if let Some(kind) = kind {
                triggers.push(Trigger {
                    id: Id(entity_index),
                    name: e.get("targetname").cloned().unwrap_or_default(),
                    volume: Collider::model(map, index, origin, Quat::IDENTITY, false)?,
                    kind,
                    inside: false,
                    cooldown: 0.,
                    reported: false,
                    target: e.get("target").cloned(),
                    once: class == "trigger_once",
                    fired: false,
                    health: value(e, "health", 0.),
                    bounds: (
                        origin + map.models[index].min,
                        origin + map.models[index].max,
                    ),
                });
            }
        }
        println!(
            "World interactions: {} doors, {} trigger volumes (script threads remain pending)",
            doors.len(),
            triggers.len()
        );
        let mut result = Self {
            presentation: Default::default(),
            event_world: Runtime::new(
                Registry::new(&map.entities),
                vec![],
                Facts::default(),
                Facts::default(),
            )?,
            gym_entity: None,
            map_name: String::new(),
            school: None,
            gym: None,
            school2: None,
            village: None,
            fortress: None,
            beyond: None,
            pool: None,
            pandemonium: None,
            duchess: None,
            encounters: None,
            levels: Vec::new(),
            activations: Vec::new(),
            doors,
            platforms: Vec::new(),
            triggers,
            used: 0,
            touched: 0,
        };
        result.configure_events(map)?;
        Ok(result)
    }
    pub fn set_entry(
        &mut self,
        assets: &mut crate::assets::Assets,
        map: &Bsp,
        name: &str,
        entry: Option<&str>,
    ) -> Result<()> {
        self.map_name = name.into();
        self.presentation = crate::sky_sequence::Controller::load(assets, map, name)?;
        self.duchess = if name == "potears3" {
            Some(crate::duchess::Duchess::load(assets, map)?)
        } else {
            None
        };
        self.encounters = if matches!(
            name,
            "skool1"
                | "skool2"
                | "gvillage"
                | "potears1"
                | "pandemonium"
                | "fortress1"
                | "fortress2"
        ) {
            Some(crate::encounters::Encounters::load(
                assets, map, name, entry,
            )?)
        } else {
            None
        };
        self.pandemonium = if name == "pandemonium" {
            Some(crate::pandemonium::Pandemonium::load(assets, map)?)
        } else {
            None
        };
        if name == "pandemonium" {
            for d in &mut self.doors {
                d.locked = d.name == "t141";
            }
        }
        self.pool = if name == "potears1" {
            {
                let mut pool = crate::pool::Pool::load(assets, map)?;
                pool.select_arrival(map, entry);
                Some(pool)
            }
        } else {
            None
        };
        self.beyond = if name == "fortress2" {
            Some(crate::beyond::Beyond::load(map)?)
        } else {
            None
        };
        self.fortress = if name == "fortress1" {
            Some(crate::fortress::Fortress::load(
                assets,
                map,
                entry == Some("fortress1_start2"),
            )?)
        } else {
            None
        };
        self.village = if name == "gvillage" {
            Some(crate::village::Village::load(assets, map)?)
        } else {
            None
        };
        self.school2 = if name == "skool2" {
            Some(crate::school2::School2::load(assets, map)?)
        } else {
            None
        };
        self.gym = if name == "skool2" {
            Some(crate::gym::Gym::load(map)?)
        } else {
            None
        };
        self.school = if name == "skool1" {
            Some(crate::school::School::load(
                map,
                entry == Some("skool1_start2"),
            )?)
        } else {
            None
        };
        if name == "skool2" {
            for d in &mut self.doors {
                d.locked = false;
            }
        }
        self.platforms.clear();
        if let Some(b) = &mut self.beyond {
            b.configure_cinema(assets, map)?;
        }
        if let Some(s) = &mut self.school {
            s.configure_cinema(assets, map)?;
        }
        if name == "skool1" {
            let returning = entry == Some("skool1_start2");
            for e in &map.entities {
                if school_platform(e) != Some(returning) {
                    continue;
                }
                if let Some(index) = model(e, map) {
                    let origin = e
                        .get("origin")
                        .and_then(|s| vector(s))
                        .unwrap_or(Vec3::ZERO);
                    self.platforms.push((
                        index,
                        origin,
                        Collider::model(map, index, origin, Quat::IDENTITY, true)?,
                    ));
                }
            }
            println!(
                "School steam floor: {} authored platforms, {} entry (cinematic movement pending)",
                self.platforms.len(),
                if returning { "return" } else { "first" }
            );
        }
        if self.beyond.is_some() {
            for d in &mut self.doors {
                d.locked = false;
            }
        }
        self.load_levels(assets, map, name, entry)?;
        self.configure_events(map)?;
        Ok(())
    }
    pub fn event_facts(&self) -> Facts {
        let mut facts = Facts::default();
        if let Some(p) = &self.pool {
            facts.extend(p.facts());
        }
        if let Some(f) = &self.beyond {
            facts.extend(f.facts());
        }
        if let Some(f) = &self.fortress {
            facts.extend(f.facts());
        }
        if let Some(d) = &self.duchess {
            facts.extend(d.facts());
        }
        if let Some(p) = &self.pandemonium {
            facts.extend(p.facts());
        }
        if let Some(s) = &self.school {
            facts.extend(s.event_facts());
        }
        if let Some(s) = &self.school2 {
            facts.extend(s.event_facts());
        }
        facts.extend(self.level_facts());
        facts
    }
    fn gate(&self, t: &Trigger) -> Condition {
        if self.pool.is_some() {
            return crate::pool::Pool::gate(&t.name);
        }
        if self.beyond.is_some() {
            return crate::beyond::Beyond::gate(&t.name);
        }
        if self.fortress.is_some() {
            return crate::fortress::Fortress::gate(
                &t.name,
                match &t.kind {
                    TriggerKind::Script(n) => n,
                    _ => "",
                },
            );
        }
        let thread = if let TriggerKind::Script(s) = &t.kind {
            s.as_str()
        } else {
            ""
        };
        let exit = if let TriggerKind::Exit(s, _) = &t.kind {
            Some(s.as_str())
        } else {
            None
        };
        if self.duchess.is_some() {
            crate::duchess::Duchess::gate(&t.name, thread, exit.is_some())
        } else if self.pandemonium.is_some() {
            crate::pandemonium::Pandemonium::gate(
                thread,
                matches!(t.kind, TriggerKind::Teleport(..)),
            )
        } else if self.school.is_some() {
            self.school
                .as_ref()
                .unwrap()
                .condition(&t.name, thread, exit)
        } else if self.school2.is_some() {
            crate::school2::School2::trigger_condition(&t.name, thread, exit.is_some())
        } else {
            self.level_gate(t)
        }
    }
    fn configure_events(&mut self, map: &Bsp) -> Result<()> {
        let registry = Registry::new(&map.entities);
        let mut rules = Vec::new();
        let mut initial = Facts::default();
        let rule = |key: String, event, condition, once, actions| Rule {
            key,
            event,
            condition,
            once,
            cooldown: 0.,
            actions,
        };
        // Components explicitly register activation receivers. An arbitrary entity name never
        // executes an original script or becomes a supported enemy by coincidence.
        // Existing BSP targets are only routed to reviewed encounter/relay receivers.
        // Door activation is explicit in level rules: blindly wiring old door targets would
        // override the established paired-leaf opening direction and quest ownership.
        let mut receivers: BTreeSet<Id> = BTreeSet::new();
        if let Some(s) = &self.encounters {
            for &id in &s.identities {
                receivers.insert(id);
                rules.push(rule(
                    format!("actor/{}", id.0),
                    Event::Entity(id, Input::Activate),
                    Condition::Always,
                    true,
                    vec![Action::Output(Effect::Activate(id))],
                ));
            }
        }
        for d in &self.doors {
            rules.push(rule(
                format!("door/{}", d.id.0),
                Event::Entity(d.id, Input::Activate),
                Condition::Always,
                false,
                vec![Action::Output(Effect::Door {
                    id: d.id,
                    open: true,
                    locked: false,
                })],
            ));
            rules.push(rule(
                format!("use/{}", d.id.0),
                Event::Entity(d.id, Input::Use),
                Condition::Always,
                false,
                vec![],
            ));
        }
        let relays = registry
            .definitions
            .iter()
            .filter(|d| d.class == "trigger_relay" && !map.entities[d.id.0].contains_key("thread"))
            .map(|d| d.id)
            .collect::<Vec<_>>();
        receivers.extend(&relays);
        receivers.extend(self.level_receivers(&registry));
        let send = |name: &str| {
            registry
                .named(name)
                .iter()
                .filter(|id| receivers.contains(id))
                .map(|&id| Action::Send {
                    event: Event::Entity(id, Input::Activate),
                    delay: 0.,
                })
                .collect::<Vec<_>>()
        };
        if self.pandemonium.is_some() {
            rules.push(rule(
                "pand/cart_done".into(),
                Event::Signal("pand.cart_done".into()),
                Condition::Always,
                true,
                send("airship_cardguard1"),
            ));
            for t in &self.triggers {
                let TriggerKind::Script(thread) = &t.kind else {
                    continue;
                };
                let groups: Vec<(&str, f64)> = match thread.as_str() {
                    "Airship_CardGuards_Thread" => vec![("airship_cardguard2", 0.), ("t129", 3.)],
                    "alice_return_trigger1" => {
                        vec![("return_di1", 0.), ("return_di2", 0.), ("club_spawn1", 0.)]
                    }
                    _ => continue,
                };
                let mut actions = Vec::new();
                for (name, delay) in groups {
                    for &id in registry.named(name) {
                        if receivers.contains(&id) {
                            actions.push(Action::Send {
                                event: Event::Entity(id, Input::Activate),
                                delay,
                            });
                        }
                    }
                }
                rules.push(rule(
                    format!("pand/{}", t.id.0),
                    Event::Entity(t.id, Input::Activate),
                    self.gate(t),
                    true,
                    actions,
                ));
            }
        }
        for &id in &relays {
            // Only plain relays are supported; never infer arbitrary thread semantics.
            let e = &map.entities[id.0];
            if e.contains_key("thread") {
                continue;
            }
            let actions = e.get("target").map(|s| send(s)).unwrap_or_default();
            rules.push(rule(
                format!("relay/{}", id.0),
                Event::Entity(id, Input::Activate),
                Condition::Always,
                false,
                actions,
            ));
        }
        for t in &self.triggers {
            let mut actions = if self.beyond.is_none() && matches!(t.kind, TriggerKind::Script(_)) {
                t.target.as_ref().map(|s| send(s)).unwrap_or_default()
            } else {
                Vec::new()
            };
            actions.push(Action::Output(Effect::Trigger(t.id)));
            rules.push(rule(
                format!("trigger/{}", t.id.0),
                Event::Entity(t.id, Input::Activate),
                Condition::All(vec![Condition::Enabled(t.id),
                    if self.school2.is_some()
                        && matches!(&t.kind, TriggerKind::Script(s) if s == crate::school2::rage_hint::EVENT)
                        && !map.difficulty.allows(map.entities[t.id.0].get("spawnflags")
                            .and_then(|s| s.parse().ok()).unwrap_or(0))
                    { Condition::Any(vec![]) } else { self.gate(t) }]),
                t.once || (t.health > 0. && !matches!(&t.kind, TriggerKind::Script(n) if self.levels.iter().any(|s| s.ctl.repeatable_shot(n)))),
                actions,
            ));
            if self.map_name == "potears1" && self.encounters.is_some() {
                if let TriggerKind::Script(thread) = &t.kind {
                    if let Some(groups) = crate::ladybug::activation(thread) {
                        let mut actions = Vec::new();
                        for (name, delay) in groups {
                            for &id in registry.named(name) {
                                if receivers.contains(&id) {
                                    actions.push(Action::Send {
                                        event: Event::Entity(id, Input::Activate),
                                        delay,
                                    });
                                }
                            }
                        }
                        rules.push(rule(
                            format!("ladybug/{}", t.id.0),
                            Event::Entity(t.id, Input::Activate),
                            Condition::Enabled(t.id),
                            true,
                            actions,
                        ));
                    }
                }
            }
        }
        if self.school.is_some() {
            initial.flag("story.theatre_finished", false);
            let mut actions = vec![Action::Flag("story.theatre_finished".into(), true)];
            actions.extend(send("play_guard1"));
            actions.extend(send("play_guard2"));
            for d in self
                .doors
                .iter()
                .filter(|d| d.name.starts_with("play_door"))
            {
                actions.push(Action::Enable(d.id, true));
                actions.push(Action::Output(Effect::Door {
                    id: d.id,
                    open: true,
                    locked: false,
                }));
            }
            rules.push(rule(
                "dialogue/Theatre_Cinematic".into(),
                Event::DialogueFinished("Theatre_Cinematic".into()),
                Condition::Always,
                true,
                actions,
            ));
        }
        if self.beyond.is_some() {
            for t in &self.triggers {
                if let TriggerKind::Script(thread) = &t.kind {
                    // Keep the pre-machinery trigger rule unchanged so old saves
                    // can gain reviewed actor receivers through an additive rule.
                    if let Some(target) = &t.target {
                        rules.push(rule(
                            format!("beyond/target/{}", t.id.0),
                            Event::Entity(t.id, Input::Activate),
                            Condition::All(vec![Condition::Enabled(t.id), self.gate(t)]),
                            t.once || (t.health > 0. && !matches!(&t.kind, TriggerKind::Script(n) if self.levels.iter().any(|s| s.ctl.repeatable_shot(n)))),
                            send(target),
                        ));
                    }
                    let group = match thread.as_str() {
                        "HubAmbush" => "jumpers",
                        "boo3" => "boojum3",
                        _ => continue,
                    };
                    rules.push(rule(
                        format!("beyond/actors/{}", t.id.0),
                        Event::Entity(t.id, Input::Activate),
                        self.gate(t),
                        true,
                        send(group),
                    ));
                }
            }
        }
        if self.fortress.is_some() {
            for t in self.triggers.iter().filter(
                |t| matches!(&t.kind, TriggerKind::Script(n) if n == "Fortress1_Boojum_Attack"),
            ) {
                let mut actions = send("get_s1booj1");
                actions.extend(send("get_s1booj2"));
                rules.push(rule(
                    "fortress/boojums".into(),
                    Event::Entity(t.id, Input::Touch),
                    self.gate(t),
                    true,
                    actions,
                ));
            }
        }
        if self.village.is_some() {
            rules.push(rule(
                "dialogue/Torchgnome3_Dialog_part2".into(),
                Event::DialogueFinished("Torchgnome3_Dialog_part2".into()),
                Condition::Always,
                true,
                send("guard_rabbit"),
            ));
        }
        if self.school2.is_some() {
            for (name, flag) in [
                ("Old_Gnome_Mushroom", "quest.mushroom_dialogue"),
                ("Old_Gnome_SpiceDrops", "quest.spice_dialogue"),
                ("Skool2_LastGnome_Cinema", "quest.final_dialogue"),
            ] {
                rules.push(rule(
                    format!("dialogue/{name}"),
                    Event::DialogueFinished(name.into()),
                    Condition::flag(flag),
                    true,
                    vec![Action::Output(Effect::DialogueFinished(name.into()))],
                ));
            }
            self.gym_entity = map
                .entities
                .iter()
                .position(|e| e.get("move_thread").is_some_and(|s| s == "extendBleachers"))
                .map(Id);
            if let Some(id) = self.gym_entity {
                initial.count("gym.lever_uses", 0);
                rules.push(rule(
                    format!("use/{}", id.0),
                    Event::Entity(id, Input::Use),
                    Condition::Always,
                    true,
                    vec![Action::Count("gym.lever_uses".into(), 1)],
                ));
            }
        }
        rules.extend(self.level_rules(map, &registry, &receivers)?);
        initial.extend(self.level_initial());
        self.event_world = Runtime::new(registry, rules, initial, self.event_facts())?;
        self.init_traversal(map)?;
        Ok(())
    }
    pub fn dispatch(&mut self, event: Event) -> Events {
        let mut events = Events::default();
        if let Err(e) = self.event_world.fire(event, &self.event_facts()) {
            eprintln!("World event rejected: {e:#}");
            events.message = Some(format!("World event failed: {e}"));
        } else {
            events.merge(self.apply_outputs());
        }
        events
    }
    fn apply_outputs(&mut self) -> Events {
        self.sync_entity_flags();
        let mut events = Events::default();
        for output in self.event_world.take_outputs() {
            let mirror = (!self.levels.is_empty()).then(|| output.clone());
            match output {
                Effect::Trigger(id) => {
                    if let Some(index) = self.triggers.iter().position(|t| t.id == id) {
                        events.merge(self.trigger_effect(index));
                    }
                }
                Effect::Activate(id) => {
                    if let Some(s) = &mut self.encounters {
                        s.activate_entity(id);
                    }
                }
                Effect::Door { id, open, locked } => {
                    if let Some(d) = self.doors.iter_mut().find(|d| d.id == id) {
                        d.locked = locked;
                        d.target = if open { 90. } else { 0. };
                    }
                }
                Effect::DialogueFinished(name) => {
                    if let Some(s) = &mut self.school2 {
                        if s.completed_dialogue(&name) {
                            events.message = Some(s.quest.objective());
                            events.sound = Some(
                                if s.quest.stage == crate::school2_quest::Stage::Mixing {
                                    "sound/character/gnome/elder/mixing.wav"
                                } else {
                                    "sound/character/gnome/elder/vanish.wav"
                                }
                                .into(),
                            );
                        }
                    }
                }
            }
            if let Some(effect) = mirror {
                events.merge(self.level_output(&effect));
            }
        }
        events
    }
    fn sync_entity_flags(&mut self) {
        if let Some(s) = &mut self.encounters {
            s.sync_enabled(&self.event_world);
            // The old Touch rule is retained for save signatures. The persisted
            // shutter event owns this ambush, including saves from that build.
            if self.fortress.as_ref().is_some_and(|f| f.state.shutters) {
                s.activate("s1_booj1");
                s.activate("s1_booj2");
            }
        }
    }
    pub fn sync(&self, world: &mut World) {
        world.set_camera_obstacles(self.levels.iter().flat_map(|s| s.ctl.camera_colliders()).collect());
        world.set_settled_supports(self.levels.iter().flat_map(|s| s.ctl.settled_supports()).collect());
        world
            .traversal
            .sync(&self.event_world, self.pandemonium.is_some());
        for slot in &self.levels {
            slot.ctl.traversal(&mut world.traversal);
        }
        world.set_dynamic(
            self.doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .chain(self.platforms.iter().map(|p| p.2.clone()))
                .chain(self.school.iter().flat_map(|s| s.colliders()))
                .chain(self.gym.iter().flat_map(|g| g.colliders()))
                .chain(self.school2.iter().flat_map(|g| g.colliders()))
                .chain(self.village.iter().flat_map(|g| g.colliders()))
                .chain(self.fortress.iter().flat_map(|g| g.colliders()))
                .chain(self.beyond.iter().flat_map(|g| g.colliders()))
                .chain(self.pool.iter().flat_map(|g| g.colliders()))
                .chain(self.pandemonium.iter().flat_map(|g| g.colliders()))
                .chain(self.duchess.iter().flat_map(|g| g.colliders()))
                .chain(self.levels.iter().flat_map(|l| l.ctl.colliders()))
                .collect(),
        );
        if !self.levels.is_empty() {
            world.set_dynamic_liquids(self.levels.iter().flat_map(|l| l.ctl.liquids()).collect());
        }
    }
    pub fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        self.doors
            .iter()
            .filter(|d| d.enabled && self.event_world.enabled(d.id))
            .map(|d| (d.model, d.origin, d.rotation()))
            .chain(self.platforms.iter().map(|p| {
                (
                    p.0,
                    p.1 + Vec3::Z * self.school.as_ref().map_or(0., |s| s.scene_platform(p.0)),
                    Quat::IDENTITY,
                )
            }))
            .chain(self.school.iter().flat_map(|s| s.transforms()))
            .chain(self.gym.iter().flat_map(|g| g.transforms()))
            .chain(self.school2.iter().flat_map(|g| g.transforms()))
            .chain(self.village.iter().flat_map(|g| g.transforms()))
            .chain(self.fortress.iter().flat_map(|g| g.transforms()))
            .chain(self.beyond.iter().flat_map(|g| g.transforms()))
            .chain(self.pool.iter().flat_map(|g| g.transforms()))
            .chain(self.pandemonium.iter().flat_map(|g| g.transforms()))
            .chain(self.duchess.iter().flat_map(|g| g.transforms()))
            .chain(self.levels.iter().flat_map(|l| l.ctl.transforms()))
            .collect()
    }
    pub fn advance_school(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
    ) -> Result<()> {
        let _profile = crate::frame_profile::span("movers");
        if let Some(d) = &mut self.duchess {
            d.advance(dt, map, world, player)?;
        }
        if let Some(p) = &mut self.pandemonium {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .collect::<Vec<_>>();
            p.advance(dt, map, world, player, &fixed)?;
        }
        if let Some(p) = &mut self.pool {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .collect::<Vec<_>>();
            p.advance(dt, map, world, player, &fixed)?;
        }
        if let Some(f) = &mut self.beyond {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .collect::<Vec<_>>();
            f.advance(dt, map, world, player, &fixed)?;
            for t in &mut self.triggers {
                let e = &map.entities[t.id.0];
                if let Some(base) = e.get("origin").and_then(|s| vector(s)) {
                    if let Some((p, r)) = f.trigger_pose(&t.name, base) {
                        t.volume = Collider::model(map, model(e, map).unwrap(), p, r, false)?;
                    }
                }
            }
        }
        if let Some(f) = &mut self.fortress {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .collect::<Vec<_>>();
            let boojums = f.state.cinema.beat == Some(crate::fortress::cinema::Beat::Boojum);
            f.advance(dt, map, world, player, &fixed)?;
            if boojums && dt > 0. {
                if let Some(e) = &mut self.encounters {
                    f.cinema.place_boojums(f.state.cinema.time, e);
                }
            }
            for t in &mut self.triggers {
                let e = &map.entities[t.id.0];
                if let Some(base) = e.get("origin").and_then(|s| vector(s)) {
                    if let Some((p, r)) = f.trigger_pose(&t.name, base) {
                        t.volume = Collider::model(map, model(e, map).unwrap(), p, r, false)?;
                    }
                }
            }
        }
        if let Some(village) = &mut self.village {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .collect::<Vec<_>>();
            village.advance(dt, map, world, player, &fixed)?;
        }
        if let Some(school) = &mut self.school {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .chain(self.platforms.iter().map(|p| p.2.clone()))
                .collect::<Vec<_>>();
            school.advance(dt, map, world, player, &fixed)?;
        }
        if let Some(gym) = &mut self.gym {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .chain(self.school2.iter().flat_map(|s| s.colliders()))
                .collect::<Vec<_>>();
            let clock = if self.school2.as_ref().is_some_and(|s| s.hold_bleachers()) { 0. } else { dt };
            gym.advance(clock, map, world, player, &fixed)?;
        }
        if let Some(s) = &mut self.school2 {
            let fixed = self
                .doors
                .iter()
                .filter(|d| d.enabled && self.event_world.enabled(d.id))
                .map(|d| d.collider.clone())
                .chain(self.gym.iter().flat_map(|g| g.colliders()))
                .collect::<Vec<_>>();
            s.advance(dt, map, world, player, &fixed)?;
        }
        self.level_advance(dt, map, world, player)?;
        if dt > 0. {
            for slot in &mut self.levels {
                if let Some(name) = slot.ctl.take_presentation_event() { self.presentation.event(name); }
            }
        }
        Ok(())
    }
    pub fn hazardous(&self, feet: Vec3) -> bool {
        self.triggers.iter().any(|t| {
            matches!(t.kind, TriggerKind::Hurt(_) | TriggerKind::Fall)
                && self.event_world.enabled(t.id)
                && self
                    .school
                    .as_ref()
                    .is_none_or(|s| s.trigger_enabled(&t.name, "", None))
                && t.volume
                    .touches(feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF)
        })
    }
    fn nearest(&self, world: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        self.doors
            .iter()
            .enumerate()
            .filter_map(|(i, d)| {
                if !d.enabled || !self.event_world.enabled(d.id) {
                    return None;
                }
                let center = d.origin + d.rotation() * (d.center - d.origin);
                let delta = center - eye;
                let distance = delta.length();
                if distance > 128. || delta.normalize_or_zero().dot(aim) < 0.15 {
                    return None;
                }
                let hit = d.collider.trace(eye, center, Vec3::ZERO);
                let trace = world.sweep(eye, center, Vec3::ZERO);
                if trace.start_solid || trace.fraction + 0.02 < hit.fraction {
                    return None;
                }
                Some((i, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }
    pub fn prompt(&self, world: &World, eye: Vec3, aim: Vec3) -> Option<&'static str> {
        if let Some(p) = self.beyond.as_ref().and_then(|b| b.prompt(world, eye, aim)) {
            return Some(p);
        }
        if self
            .gym_entity
            .is_some_and(|id| self.event_world.enabled(id))
        {
            if let Some(prompt) = self.gym.as_ref().and_then(|g| g.prompt(world, eye, aim)) {
                return Some(prompt);
            }
        }
        if let Some(p) = self.pandemonium.as_ref().and_then(|p| p.prompt(world, eye)) {
            return Some(p);
        }
        if let Some(p) = self.level_prompt(world, eye, aim) {
            return Some(p);
        }
        self.nearest(world, eye, aim).map(|i| {
            let d = &self.doors[i];
            if d.locked {
                "Door locked / requires a level event"
            } else if d.target.abs() > 1. {
                "E  close door"
            } else {
                "E  open door"
            }
        })
    }
    pub fn update(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &Player,
        aim: Vec3,
        use_pressed: bool,
    ) -> Result<Events> {
        let mut events = Events::default();
        if dt <= 0. {
            return Ok(events);
        }
        self.presentation.update(dt);
        if self
            .school
            .as_mut()
            .is_some_and(|s| s.take_scene_completion())
        {
            events.merge(self.dispatch(Event::DialogueFinished(
                crate::school::cinema::THEATRE.into(),
            )));
        }
        if let Some(s) = &mut self.school {
            events.merge(s.library_events());
        }
        if self.beyond.as_mut().is_some_and(|b| b.rage_wave()) {
            if let Some(encounters) = &mut self.encounters {
                encounters.activate("rage_guard2");
            }
        }
        if let Some(p) = &mut self.pool {
            events.merge(p.scene_update());
        }
        if dt > 0. {
            if let Some(sound) = self.gym.as_mut().and_then(|g| g.take_sound()) {
                events.sound = Some(sound.into());
            }
        }
        if let Some(r) = self.school.as_ref().and_then(|s| s.return_visit.as_ref()) {
            for d in &mut self.doors {
                if [14, 15].contains(&d.model) {
                    d.locked = r.phase < crate::school_return::Phase::Observatory;
                }
            }
        }
        if let Some(s) = &self.school2 {
            for d in &mut self.doors {
                if d.name == "t57" {
                    d.enabled = s.quest.stage != crate::school2_quest::Stage::Complete;
                }
                if d.name.starts_with("boojum_door") {
                    d.locked = s.quest.stage == crate::school2_quest::Stage::Battle
                        && player.feet.x > 1370.;
                    if d.locked {
                        d.target = 0.;
                    }
                }
            }
        }
        let rope_use = use_pressed
            && self
                .pandemonium
                .as_ref()
                .is_some_and(|p| p.prompt(world, player.eye()).is_some());
        if let Some(p) = &mut self.pandemonium {
            events.merge(p.update(world, player, rope_use));
            if p.state.leaving {
                for d in &mut self.doors {
                    if d.name == "t141" {
                        d.enabled = false;
                    }
                }
            }
        }
        if let Some(b) = &mut self.beyond {
            events.merge(b.update());
        }
        let use_pressed = use_pressed && !rope_use;
        events.merge(self.level_update(world, player, aim, use_pressed));
        if use_pressed {
            if let Some(e) = self
                .beyond
                .as_mut()
                .and_then(|b| b.activate(world, player.eye(), aim))
            {
                events.merge(e);
                self.used += 1;
            }
            if self.gym_entity.is_some_and(|id| {
                self.event_world
                    .allowed(&Event::Entity(id, Input::Use), &self.event_facts())
            }) && self
                .gym
                .as_mut()
                .is_some_and(|g| g.activate(world, player.eye(), aim))
            {
                events.message = Some("Bleachers extending / climb toward the Elder Gnome".into());
                if let Some(s) = &mut self.school2 { s.begin_bleachers(); }
                events.sound = Some("sound/world/machine/lever1.wav".into());
                self.used += 1;
                if let Some(id) = self.gym_entity {
                    events.merge(self.dispatch(Event::Entity(id, Input::Use)));
                }
            } else if let Some(index) = self.nearest(world, player.eye(), aim) {
                if self.doors[index].locked
                    || !self.event_world.allowed(
                        &Event::Entity(self.doors[index].id, Input::Use),
                        &self.event_facts(),
                    )
                {
                    events.message =
                        Some("This door opens when the current quest is complete".into());
                } else {
                    let open = self.doors[index].target.abs() < 1.;
                    let center = self.doors[index].center;
                    // Adjacent leaves operate together; remote doors do not receive a global 'use'.
                    for d in &mut self.doors {
                        if !d.enabled
                            || !self.event_world.enabled(d.id)
                            || d.locked
                            || d.center.distance(center) > 100.
                        {
                            continue;
                        }
                        if open {
                            let local = d.center - d.origin;
                            let plus = d.origin
                                + Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * local;
                            let minus = d.origin
                                + Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * local;
                            d.sign = if plus.distance_squared(player.eye())
                                >= minus.distance_squared(player.eye())
                            {
                                1.
                            } else {
                                -1.
                            };
                            d.target = 90. * d.sign;
                        } else {
                            d.target = 0.;
                        }
                    }
                    events.sound = self.doors[index].sound.clone();
                    self.used += 1;
                    events.merge(self.dispatch(Event::Entity(self.doors[index].id, Input::Use)));
                }
            }
        }
        for d in &mut self.doors {
            if !d.enabled || !self.event_world.enabled(d.id) {
                continue;
            }
            let delta = (d.target - d.angle).clamp(-90. * dt / d.duration, 90. * dt / d.duration);
            if delta.abs() < 1e-5 {
                continue;
            }
            // Sample the swept arc to avoid trapping or sweeping a leaf through Alice.
            let steps = (delta.abs() / 2.).ceil().max(1.) as usize;
            let mut clear = true;
            let mut collider = d.collider.clone();
            for step in 1..=steps {
                let rot = Quat::from_rotation_z(
                    (d.angle + delta * step as f32 / steps as f32).to_radians(),
                );
                let next = Collider::model(map, d.model, d.origin, rot, true)?;
                if next.touches(
                    player.feet + PLAYER_CENTER,
                    player.feet + PLAYER_CENTER,
                    PLAYER_HALF,
                ) {
                    clear = false;
                    break;
                }
                collider = next;
            }
            if clear {
                d.angle += delta;
                d.collider = collider;
            }
        }
        self.sync(world);
        Ok(events)
    }
    pub fn triggers(&mut self, dt: f32, before: Vec3, after: Vec3) -> Events {
        let mut events = Events::default();
        if dt <= 0. || !dt.is_finite() {
            return events;
        }
        if let Err(e) = self.event_world.advance(dt as f64, &self.event_facts()) {
            eprintln!("World event time rejected: {e:#}");
            events.message = Some(format!("World event failed: {e}"));
        } else {
            events.merge(self.apply_outputs());
        }
        if let Some(p) = &self.pandemonium {
            if p.state.cart == crate::pandemonium::Cart::Finished {
                events.merge(self.dispatch(Event::Signal("pand.cart_done".into())));
            }
        }
        if self.school.as_ref().is_some_and(|s| s.cinematic()) {
            if self
                .school
                .as_ref()
                .and_then(|s| s.return_visit.as_ref())
                .is_some_and(|s| s.phase == crate::school_return::Phase::Complete)
            {
                events.transition = Some(("potears1".into(), Some("potears1_start1".into())));
            }
            return events;
        }
        if self.scripted() {
            return events;
        }
        for index in 0..self.triggers.len() {
            let t = &self.triggers[index];
            // Pool's final jump owns its exit. Keep the legacy rule identity and
            // signature for existing saves, but don't let physical contact with
            // the downstream volume bypass the scene's one-shot transition.
            if self.pool.is_some()
                && matches!(&t.kind, TriggerKind::Exit(map, _) if map == "potears2")
            {
                continue;
            }
            if self.school.as_ref().is_some_and(|s| s.owns_recipe_exit())
                && matches!(&t.kind, TriggerKind::Exit(map, _) if map == "skool2")
            {
                continue;
            }
            if t.health > 0. || (t.once && t.fired) {
                continue;
            }
            let event = Event::Entity(t.id, Input::Activate);
            if !self.event_world.allowed(&event, &self.event_facts()) {
                self.triggers[index].inside = false;
                continue;
            }
            let t = &mut self.triggers[index];
            t.cooldown = (t.cooldown - dt).max(0.);
            let crossed =
                t.volume
                    .touches(before + PLAYER_CENTER, after + PLAYER_CENTER, PLAYER_HALF);
            let inside =
                t.volume
                    .touches(after + PLAYER_CENTER, after + PLAYER_CENTER, PLAYER_HALF);
            let fire = crossed
                && (!t.inside || matches!(t.kind, TriggerKind::Hurt(_)))
                && t.cooldown == 0.;
            t.inside = inside;
            if fire {
                events.merge(self.dispatch(event));
                if self.scripted() {
                    break;
                }
            }
        }
        events
    }
    fn trigger_effect(&mut self, index: usize) -> Events {
        let mut events = Events::default();
        let t = &mut self.triggers[index];
        match t.kind.clone() {
            TriggerKind::Dialogue(path) => {
                if self.map_name == "skool2" && !t.reported {
                    events.story.push(path);
                    t.reported = true;
                }
            }
            TriggerKind::Exit(map, spawn) => {
                if let Some(e) = self.levels.iter_mut().find_map(|s| s.ctl.exit_contact(&(map.clone(), spawn.clone()))) { return e; }
                if self.pool.is_some()
                    && crate::pool::cinema::ENDING
                        .end
                        .exit
                        .unwrap()
                        .matches(&(map.clone(), spawn.clone()))
                {
                    return events;
                }
                events.transition = Some((map, spawn));
                t.cooldown = 1.;
                self.touched += 1;
            }
            TriggerKind::Teleport(pos, yaw) => {
                let landing=self.levels.iter().find_map(|s|s.ctl.teleport_destination(pos)).unwrap_or(pos);
                events.teleport = Some((landing, yaw));
                if let Some(b) = &mut self.beyond {
                    if pos.distance(vec3(1056., 376., 4.)) < 1. {
                        b.event("GS_Movers_Reset");
                    }
                }
                t.cooldown = 1.;
                self.touched += 1;
            }
            TriggerKind::Hurt(damage) => {
                events.damage = damage;
                t.cooldown = 0.5;
            }
            TriggerKind::Fall => {
                events.damage = 10000.;
                events.message = Some("Fatal fall / Enter to retry".into());
                t.cooldown = 0.5;
            }
            TriggerKind::Script(thread) => {
                if !self.presentation.enabled(&t.name) {
                    return events;
                }
                t.fired = true;
                if self.presentation.event(&thread) {
                    self.touched += 1;
                    return events;
                }
                if thread.is_empty() {
                    return events;
                }
                // Complete visit owners also record progression handled by the placed cast.
                if matches!(self.map_name.as_str(), "wchess1" | "wchess2" | "hedge1") {
                    if let Some(e) = hooks::event(&mut self.levels, &thread) {
                        events.merge(e);self.touched += 1;return events;
                    }
                }
                if crate::npc::chess_thread_supported(&self.map_name, &thread)
                    || crate::npc::creature_thread_supported(&self.map_name, &thread)
                {
                    self.touched += 1;
                    return events;
                }
                if self.map_name == "potears1" && crate::ladybug::activation(&thread).is_some() {
                    self.touched += 1;
                    return events;
                }
                if let Some(e) = self
                    .school
                    .as_mut()
                    .and_then(|s| s.event(&thread))
                    .or_else(|| self.school2.as_mut().and_then(|s| s.event(&thread)))
                    .or_else(|| self.village.as_mut().and_then(|s| s.event(&thread)))
                    .or_else(|| self.fortress.as_mut().and_then(|s| s.event(&thread)))
                    .or_else(|| self.beyond.as_mut().and_then(|s| s.event(&thread)))
                    .or_else(|| self.pool.as_mut().and_then(|s| s.event(&thread)))
                    .or_else(|| self.pandemonium.as_mut().and_then(|s| s.event(&thread)))
                    .or_else(|| hooks::event(&mut self.levels, &thread))
                {
                    events.merge(e);
                    self.touched += 1;
                } else if self.school.is_none() && crate::story::supports(&self.map_name, &thread) {
                    events.story.push(thread);
                } else if !t.reported {
                    eprintln!("Pending world script: {thread}");
                    events.message = Some("This level event is not playable yet".into());
                    t.reported = true;
                }
            }
        }
        events
    }
    pub fn take_activations(&mut self) -> Vec<String> {
        std::mem::take(&mut self.activations)
    }
    pub fn scripted(&self) -> bool {
        self.beyond.as_ref().is_some_and(|b| b.scene_id().is_some())
            || self.village.as_ref().is_some_and(|v| v.cinema.active())
            || self.school.as_ref().is_some_and(|s| s.scene_id().is_some())
            || self.school2.as_ref().is_some_and(|s| s.cinematic())
            || self.pandemonium.as_ref().is_some_and(|p| p.cinematic())
            || self.pool.as_ref().is_some_and(|p| p.scene_id().is_some())
            || self
                .fortress
                .as_ref()
                .is_some_and(|f| f.state.cinema.active())
            || self.levels_scripted()
    }
    pub fn entry_story(&mut self, story: &mut crate::story::Story) {
        if let Some(v) = &mut self.village {
            v.cinema.begin(story);
        } else if let Some(f) = &mut self.fortress {
            f.cinema.begin(&mut f.state.cinema, story);
        } else if let Some(p) = &mut self.pool {
            p.begin();
        } else if !self.level_entry_story(story) {
            story.trigger("entry");
        }
    }
    pub fn prepare_story(&mut self, story: &mut crate::story::Story) -> bool {
        story.line_limit = None;
        if let Some(s) = &mut self.school2 { s.begin_queued_dice(story); }
        if self
            .school
            .as_ref()
            .is_some_and(|s| !s.prepare_scene_story(story))
        {
            return false;
        }
        if self
            .school2
            .as_ref()
            .is_some_and(|s| !s.prepare_scene_story(story))
        {
            return false;
        }
        if let Some(f) = &self.fortress {
            return f.cinema.prepare_story(&f.state.cinema, story);
        }
        if self.pool.as_ref().is_some_and(|p| {
            p.state.cinema.beat == Some(crate::pool::cinema::Beat::Talk)
                && p.state.cinema.time < 0.5
        }) {
            return false;
        }
        self.village
            .as_ref()
            .is_none_or(|v| v.cinema.prepare_story(story))
            && self.level_prepare_story(story)
    }
    pub fn skip_cinematic(
        &mut self,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        story: &mut crate::story::Story,
    ) -> Result<bool> {
        let skipped = if let Some(d) = &mut self.duchess {
            d.skip(map, world, player, story)?
        } else if let Some(v) = &mut self.village {
            v.cinema.skip(player, world, story)?
        } else if let Some(p) = &mut self.pandemonium {
            p.skip(map, world, player, story)?
        } else if let Some(p) = &mut self.pool {
            p.skip(world, player, story)?
        } else if let Some(b) = &mut self.beyond {
            b.skip_scene(map, world, player)?
        } else if let Some(s) = &mut self.school {
            s.skip_first_scene(world, player, story)?
        } else if let Some(s) = &mut self.school2 {
            s.skip_scene(world, player, story)?
        } else if let Some(f) = &mut self.fortress {
            let boojums = f.state.cinema.beat == Some(crate::fortress::cinema::Beat::Boojum);
            let skipped = f.skip(player, story);
            if skipped && boojums {
                if let Some(e) = &mut self.encounters {
                    f.cinema.place_boojums(f.state.cinema.time, e);
                }
            }
            skipped
        } else {
            false
        };
        let skipped = self.skip_after(skipped, map, world, player, story)?;
        if skipped {
            self.sync(world);
        }
        Ok(skipped)
    }
    pub fn take_story_exit(
        &mut self,
        story: &mut crate::story::Story,
    ) -> Option<(String, Option<String>)> {
        if self
            .school
            .as_mut()
            .is_some_and(|s| !s.guard_recipe_exit(story))
        {
            return None;
        }
        story.take_exit()
    }
    pub fn sync_cinematic_story(&mut self, story: &crate::story::Story) {
        if let Some(s) = &mut self.school2 {
            s.sync_scene_story(story);
        }
        if let Some(s) = &mut self.school {
            s.sync_library_story(story);
        }
        if let Some(v) = &mut self.village {
            v.cinema.sync_story(story);
        }
        if let Some(p) = &mut self.pandemonium {
            p.sync_story(story);
        }
        if let Some(p) = &mut self.pool {
            p.sync_story(story);
        }
        self.level_sync_story(story);
    }
    pub fn sync_pickups(
        &mut self,
        stats: &crate::inventory::Stats,
        items: &mut Vec<crate::inventory::Pickup>,
        story: &mut crate::story::Story,
    ) {
        for item in items.iter_mut() {
            if let Some(p) = self.levels.iter().find_map(|l| l.ctl.pickup_origin(&item.id)) {
                item.origin = p;
            }
        }
        if let Some(v) = &mut self.village {
            v.cinema.sync_pickups(stats, story);
        }
        if let Some(b) = &mut self.beyond {
            if b.pickup(stats) {
                story.trigger("Fortress2_Rage_Pickup");
            }
            if b.state.last_started.is_some() {
                items.retain(|p| p.id != "fortress2:37");
            }
        }
    }
    pub fn completed_dialogue(&mut self, name: &str) -> Events {
        if name == "dice_cat" {
            if let Some(s) = &mut self.school2 {
                if s.hold_dice_completion() { return Events::default(); }
                s.event(name);
            }
        }
        if self
            .school
            .as_mut()
            .is_some_and(|s| s.hold_dialogue_completion(name))
        {
            return Events::default();
        }
        if name == crate::fortress::cinema::ARRIVAL {
            if let Some(f) = &mut self.fortress {
                f.state.cinema.dialogue_done = true;
            }
        }
        if let Some(v) = &mut self.village {
            v.cinema.completed(name);
        }
        if let Some(p) = &mut self.pool {
            p.dialogue_complete(name);
        }
        if let Some(d) = &mut self.duchess {
            d.dialogue_complete(name);
        }
        if let Some(p) = &mut self.pandemonium {
            p.dialogue_complete(name);
        }
        let mut events = self.level_dialogue(name);
        let event = Event::DialogueFinished(name.into());
        if self.event_world.allowed(&event, &self.event_facts()) {
            events.merge(self.dispatch(event));
        }
        events
    }
    pub fn activate_enemies(&mut self) {
        self.sync_entity_flags();
        let targets = self.take_activations();
        if let Some(s) = &mut self.encounters {
            for target in targets {
                s.activate(&target);
            }
            if let Some(school) = &self.school {
                school.stage_guards(s);
            }
            if let Some(school) = &self.school2 {
                school.sync_encounters(s, &mut self.event_world);
            }
        }
    }
    pub fn shot_targets(&self) -> Vec<crate::combat::Target> {
        self.triggers
            .iter()
            .enumerate()
            .filter_map(|(i, t)| {
                let TriggerKind::Script(thread) = &t.kind else {
                    return None;
                };
                if !matches!(
                    (self.map_name.as_str(), thread.as_str()),
                    ("skool1", "Open_Bookcase_Goodie") | ("gvillage", "openthis")
                ) && !self.levels.iter().any(|s| s.ctl.shootable_thread(thread)) {
                    return None;
                }
                if t.health <= 0.
                    || (t.fired && !self.levels.iter().any(|s| s.ctl.repeatable_shot(thread)))
                    || !self
                        .event_world
                        .allowed(&Event::Entity(t.id, Input::Activate), &self.event_facts())
                {
                    return None;
                }
                Some(crate::combat::Target {
                    id: SHOT_BASE + i,
                    center: (t.bounds.0 + t.bounds.1) * 0.5,
                    half: (t.bounds.1 - t.bounds.0) * 0.5,
                })
            })
            .collect()
    }
    pub fn switch_target(&self, thread: &str) -> Option<crate::combat::Target> {
        self.shot_targets().into_iter().find(|target| matches!(&self.triggers[target.id-SHOT_BASE].kind, TriggerKind::Script(s) if s==thread))
    }
    pub fn shoot(&mut self, hit: crate::combat::Hit) -> Events {
        let Some(i) = hit.id.checked_sub(SHOT_BASE) else {
            return Events::default();
        };
        // Check the same entry gates as touching; inactive or already-fired switches cannot be shot.
        if !self.shot_targets().iter().any(|t| t.id == hit.id)
            || !hit.damage.is_finite()
            || hit.damage <= 0.
        {
            return Events::default();
        }
        let t = &mut self.triggers[i];
        let repeat = matches!(&t.kind, TriggerKind::Script(n) if self.levels.iter().any(|s| s.ctl.repeatable_shot(n)));
        if repeat { let id=t.id; return self.dispatch(Event::Entity(id, Input::Activate)); }
        t.health = (t.health - hit.damage).max(0.);
        if t.health > 0. {
            return Events::default();
        }
        let id = t.id;
        self.dispatch(Event::Entity(id, Input::Activate))
    }
    pub fn reset_contacts(&mut self) {
        for t in &mut self.triggers {
            t.inside = false;
            t.cooldown = 0.;
        }
    }
}
pub const SHOT_BASE: usize = 2_000_000;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    presentation: Option<crate::sky_sequence::State>,
    #[serde(default)]
    traversal_version: u8,
    #[serde(default)]
    shared: Option<event::Snapshot>,
    doors: Vec<DoorSave>,
    triggers: Vec<TriggerSave>,
    school: Option<crate::school::Snapshot>,
    gym: Option<crate::gym::Snapshot>,
    school2: Option<crate::school2::Snapshot>,
    village: Option<crate::village::Snapshot>,
    #[serde(default)]
    fortress: Option<crate::fortress::State>,
    #[serde(default)]
    beyond: Option<crate::beyond::State>,
    #[serde(default)]
    pool: Option<crate::pool::State>,
    #[serde(default)]
    pandemonium: Option<crate::pandemonium::Snapshot>,
    #[serde(default)]
    duchess: Option<crate::duchess::State>,
    encounters: Option<crate::encounters::Snapshot>,
    activations: Vec<String>,
    used: u64,
    touched: u64,
    /// Registered controllers' state, keyed by id. Absent (and unserialized) for every legacy visit.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    levels: BTreeMap<String, serde_json::Value>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct DoorSave {
    #[serde(default)]
    id: Option<Id>,
    name: String,
    model: usize,
    enabled: bool,
    angle: f32,
    target: f32,
    sign: f32,
    locked: bool,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct TriggerSave {
    #[serde(default)]
    id: Option<Id>,
    name: String,
    inside: bool,
    cooldown: f32,
    reported: bool,
    fired: bool,
    health: f32,
}
impl Interactions {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            presentation: Some(self.presentation.state.clone()),
            traversal_version: 1,
            shared: Some(self.event_world.snapshot()),
            doors: self
                .doors
                .iter()
                .map(|d| DoorSave {
                    id: Some(d.id),
                    name: d.name.clone(),
                    model: d.model,
                    enabled: d.enabled,
                    angle: d.angle,
                    target: d.target,
                    sign: d.sign,
                    locked: d.locked,
                })
                .collect(),
            triggers: self
                .triggers
                .iter()
                .map(|t| TriggerSave {
                    id: Some(t.id),
                    name: t.name.clone(),
                    inside: t.inside,
                    cooldown: t.cooldown,
                    reported: t.reported,
                    fired: t.fired,
                    health: t.health,
                })
                .collect(),
            school: self.school.as_ref().map(|s| s.snapshot()),
            gym: self.gym.as_ref().map(|s| s.snapshot()),
            school2: self.school2.as_ref().map(|s| s.snapshot()),
            village: self.village.as_ref().map(|s| s.snapshot()),
            fortress: self.fortress.as_ref().map(|s| s.snapshot()),
            beyond: self.beyond.as_ref().map(|s| s.snapshot()),
            pool: self.pool.as_ref().map(|s| s.snapshot()),
            pandemonium: self.pandemonium.as_ref().map(|s| s.snapshot()),
            duchess: self.duchess.as_ref().map(|s| s.snapshot()),
            encounters: self.encounters.as_ref().map(|s| s.snapshot()),
            activations: self.activations.clone(),
            used: self.used,
            touched: self.touched,
            levels: self.level_snapshots(),
        }
    }
    pub fn restore(&mut self, s: &Snapshot, map: &Bsp) -> Result<()> {
        if let Some(state) = &s.presentation {
            self.presentation.restore(state)?;
        }
        anyhow::ensure!(
            s.traversal_version <= 1,
            "Unsupported saved traversal state"
        );
        let upgrade_pool = self.map_name == "potears1" && s.pool.is_none();
        let upgrade_pool_scenes =
            self.map_name == "potears1" && s.pool.as_ref().is_none_or(|p| p.cinema.version == 0);
        let upgrade_pool_ending =
            self.map_name == "potears1" && s.pool.as_ref().is_none_or(|p| p.cinema.version < 2);
        let upgrade_beyond = self.map_name == "fortress2" && s.beyond.is_none();
        let upgrade_fortress = self.map_name == "fortress1" && s.fortress.is_none();
        let upgrade_duchess = self.map_name == "potears3" && s.duchess.is_none();
        let upgrade_return = s.school.as_ref().is_some_and(|s| s.legacy_return());
        let upgrade_ladybugs = self.map_name == "potears1" && s.encounters.is_none();
        let upgrade_school2 = self.map_name == "skool2" && s.encounters.is_none();
        let upgrade_pandemonium = self.map_name == "pandemonium" && s.pandemonium.is_none();
        anyhow::ensure!(
            self.doors.len() == s.doors.len()
                && self.triggers.len() == s.triggers.len()
                && self.school.is_some() == s.school.is_some()
                && (self.duchess.is_some() == s.duchess.is_some() || upgrade_duchess)
                && self.gym.is_some() == s.gym.is_some()
                && self.school2.is_some() == s.school2.is_some()
                && self.village.is_some() == s.village.is_some()
                && (self.beyond.is_some() == s.beyond.is_some() || upgrade_beyond)
                && (self.pool.is_some() == s.pool.is_some() || upgrade_pool)
                && (self.fortress.is_some() == s.fortress.is_some() || upgrade_fortress)
                && (self.pandemonium.is_some() == s.pandemonium.is_some() || upgrade_pandemonium)
                && (self.encounters.is_some() == s.encounters.is_some()
                    || upgrade_fortress
                    || upgrade_beyond
                    || upgrade_ladybugs
                    || upgrade_school2
                    || upgrade_pandemonium),
            "Saved interactions do not match this visit"
        );
        // A save that predates a registered controller is restored against the program it was
        // made with (F2): the controller is held back until the end of this function.
        let upgrading = self.begin_level_upgrade(&s.levels, map)?;
        let bill_program = self.begin_bill_upgrade(&s.levels, map)?;
        let keep_program = self.begin_keep_upgrade(&s.levels, map)?;
        // Build the old program while controller facts still have their fresh
        // values: the signature includes their declared defaults, not saved state.
        let school2_program = if upgrade_school2 && s.shared.is_some() {
            let current = std::mem::replace(
                &mut self.event_world,
                Runtime::new(
                    Registry::new(&[]),
                    vec![],
                    Facts::default(),
                    Facts::default(),
                )?,
            );
            let enemies = self.encounters.take();
            self.configure_events(map)?;
            self.encounters = enemies;
            Some(current)
        } else {
            None
        };
        for (d, v) in self.doors.iter_mut().zip(&s.doors) {
            anyhow::ensure!(
                (v.id == Some(d.id) || (s.shared.is_none() && v.id.is_none()))
                    && d.name == v.name
                    && d.model == v.model
                    && v.angle.abs() <= 90.
                    && v.target.abs() <= 90.
                    && v.sign.abs() == 1.,
                "Invalid saved door"
            );
            d.enabled = v.enabled
                && !crate::levels::owns_submodel(&self.map_name, &map.entities[d.id.0]);
            d.angle = v.angle;
            d.target = v.target;
            d.sign = v.sign;
            d.locked = v.locked;
            d.collider = Collider::model(map, d.model, d.origin, d.rotation(), true)?;
        }
        for (t, v) in self.triggers.iter_mut().zip(&s.triggers) {
            anyhow::ensure!(
                (v.id == Some(t.id) || (s.shared.is_none() && v.id.is_none()))
                    && t.name == v.name
                    && v.cooldown >= 0.
                    && v.health >= 0.,
                "Invalid saved trigger"
            );
            t.inside = v.inside;
            t.cooldown = v.cooldown;
            t.reported = v.reported;
            t.fired = v.fired;
            t.health = v.health;
        }
        if let (Some(a), Some(b)) = (&mut self.school, &s.school) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.duchess, &s.duchess) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.gym, &s.gym) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.school2, &s.school2) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.pandemonium, &s.pandemonium) {
            a.restore(b, map)?;
            if a.state.cinema.version == 0 {
                a.state.cinema.version = 1;
                a.state.cinema.warning_done = self.triggers.iter().any(|t| t.fired && matches!(&t.kind, TriggerKind::Script(n) if n == crate::pandemonium::cinema::WARNING));
                a.state.cinema.return_done = a.state.leaving;
            }
        }
        if let (Some(a), Some(b)) = (&mut self.beyond, &s.beyond) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.pool, &s.pool) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.fortress, &s.fortress) {
            a.restore(b, map)?;
        } else if upgrade_fortress {
            if let Some(a) = &mut self.fortress {
                a.migrate_cinema();
            }
        }
        if let (Some(a), Some(b)) = (&mut self.village, &s.village) {
            a.restore(b, map)?;
        }
        if let (Some(a), Some(b)) = (&mut self.encounters, &s.encounters) {
            a.restore(b)?;
        }
        self.restore_levels(&s.levels, map)?;
        self.activations = s.activations.clone();
        self.used = s.used;
        self.touched = s.touched;
        if let Some(shared) = &s.shared {
            if let Some(current) = keep_program {
                self.finish_keep_upgrade(current, shared)?;
            } else if upgrade_pool {
                // Validate against the registry this save was made with, retaining
                // its Ladybug activation state while reopening newly gated rides.
                let pool = self.pool.take();
                let encounters = if upgrade_ladybugs {
                    self.encounters.take()
                } else {
                    None
                };
                self.configure_events(map)?;
                self.event_world.restore(shared)?;
                let previous = self.event_world.snapshot();
                let mut old = Interactions::load(map)?;
                if !upgrade_ladybugs {
                    old.encounters = self.encounters.take();
                }
                old.map_name = self.map_name.clone();
                old.configure_events(map)?;
                old.event_world.restore(&previous)?;
                self.pool = pool;
                self.encounters = if upgrade_ladybugs {
                    encounters
                } else {
                    old.encounters.take()
                };
                self.configure_events(map)?;
                let gates = self
                    .triggers
                    .iter()
                    .filter(|t| !matches!(self.gate(t), Condition::Always))
                    .map(|t| format!("trigger/{}", t.id.0))
                    .collect::<Vec<_>>();
                self.event_world
                    .extend_gated_from(&old.event_world, &gates)?;
            } else if upgrade_return {
                let visit = self.school.as_mut().unwrap().return_visit.take();
                self.configure_events(map)?;
                self.event_world.restore(shared)?;
                let previous = std::mem::replace(
                    &mut self.event_world,
                    Runtime::new(
                        Registry::new(&[]),
                        vec![],
                        Facts::default(),
                        Facts::default(),
                    )?,
                );
                self.school.as_mut().unwrap().return_visit = visit;
                self.configure_events(map)?;
                let gates = self.triggers.iter().filter(|t| matches!(&t.kind, TriggerKind::Script(n) if self.school.as_ref().unwrap().return_visit.as_ref().unwrap().gate(n).is_some()) || matches!(&t.kind, TriggerKind::Exit(n, _) if n == "potears1"))
                    .map(|t| format!("trigger/{}", t.id.0)).collect::<Vec<_>>();
                self.event_world.extend_gated_from(&previous, &gates)?;
            } else if upgrade_pandemonium || upgrade_duchess || upgrade_fortress || upgrade_beyond {
                let mut previous = Interactions::load(map)?;
                previous.event_world.restore(shared)?;
                let gates = self
                    .triggers
                    .iter()
                    .filter(|t| !matches!(self.gate(t), Condition::Always))
                    .map(|t| format!("trigger/{}", t.id.0))
                    .collect::<Vec<_>>();
                self.event_world
                    .extend_gated_from(&previous.event_world, &gates)?;
            } else if let Some(current) = bill_program {
                self.event_world.restore(shared)?;
                let previous = std::mem::replace(&mut self.event_world, current);
                self.event_world.extend_registered_from(
                    &previous,
                    &self.gated_triggers(),
                    &Default::default(),
                )?;
            } else if upgrade_school2 {
                self.event_world.restore(shared)?;
                let previous = std::mem::replace(&mut self.event_world, school2_program.unwrap());
                let receivers = self
                    .encounters
                    .as_ref()
                    .unwrap()
                    .identities
                    .iter()
                    .copied()
                    .collect();
                self.event_world
                    .extend_registered_from(&previous, &[], &receivers)?;
            } else if upgrade_ladybugs {
                let mut previous = Interactions::load(map)?;
                previous.event_world.restore(shared)?;
                self.event_world.extend_from(&previous.event_world)?;
            } else {
                self.event_world.restore(shared)?;
            }
        } else {
            // v0.19/v1 already committed these effects; importing must never emit them again.
            for t in &self.triggers {
                if t.fired {
                    self.event_world
                        .import_consumed(&format!("trigger/{}", t.id.0));
                }
            }
            if let Some(s) = &self.encounters {
                for (a, id) in s.actors.iter().zip(&s.identities) {
                    if a.active {
                        self.event_world.import_consumed(&format!("actor/{}", id.0));
                    }
                }
                if s.activated.contains("play_guard1")
                    || s.activated.contains("play_guard2")
                    || self.activations.iter().any(|s| s.starts_with("play_guard"))
                {
                    self.event_world
                        .import_consumed("dialogue/Theatre_Cinematic");
                    self.event_world.import_flag("story.theatre_finished", true);
                }
                if s.activated.contains("guard_rabbit")
                    || self.activations.iter().any(|s| s == "guard_rabbit")
                {
                    self.event_world
                        .import_consumed("dialogue/Torchgnome3_Dialog_part2");
                }
            }
            if self.gym.as_ref().is_some_and(|g| g.used) {
                if let Some(id) = self.gym_entity {
                    self.event_world.import_consumed(&format!("use/{}", id.0));
                }
                self.event_world.import_count("gym.lever_uses", 1);
            }
            if let Some(q) = &self.school2 {
                use crate::school2_quest::Stage as S;
                if !matches!(q.quest.stage, S::Explore | S::MushroomDialogue) {
                    self.event_world
                        .import_consumed("dialogue/Old_Gnome_Mushroom");
                }
                if matches!(
                    q.quest.stage,
                    S::Jumbogrow
                        | S::Growing
                        | S::Lollipop
                        | S::FinalDialogue
                        | S::Mixing
                        | S::Rewards
                        | S::Complete
                ) {
                    self.event_world
                        .import_consumed("dialogue/Old_Gnome_SpiceDrops");
                }
                if matches!(q.quest.stage, S::Mixing | S::Rewards | S::Complete) {
                    self.event_world
                        .import_consumed("dialogue/Skool2_LastGnome_Cinema");
                }
            }
        }
        if upgrade_return {
            for t in &mut self.triggers {
                if matches!(&t.kind, TriggerKind::Script(n) if ["Skool1_Setup_OLift", "Skool1_OLift_Up", "observatory_exit_cinematic"].contains(&n.as_str()))
                {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world
                        .import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
        }
        if upgrade_ladybugs {
            // Do not replay ambushes the player already passed in an older build.
            for t in &self.triggers {
                if t.fired {
                    self.event_world
                        .import_consumed(&format!("ladybug/{}", t.id.0));
                }
            }
        }
        if upgrade_pool {
            for t in &mut self.triggers {
                if matches!(&t.kind,TriggerKind::Script(n) if n.starts_with("rideleaf") || n.starts_with("leaftrain") || n.starts_with("Turtle_Encounter") || n == crate::pool::TALK)
                {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world
                        .import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
        }
        if upgrade_pool_scenes {
            if let Some(p) = &mut self.pool {
                p.state.cinema.done[0] = true;
            }
            for t in &mut self.triggers {
                if matches!(&t.kind, TriggerKind::Script(n) if crate::pool::cinema::newly_supported(n))
                {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world
                        .import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
        }
        if upgrade_pool_ending
            && self
                .pool
                .as_ref()
                .is_some_and(|p| !p.state.cinema.done[4] && p.state.cinema.ending.is_none())
        {
            // Only the newly adopted pending contact; entry and completed scenes stay consumed.
            for t in &mut self.triggers {
                if matches!(&t.kind, TriggerKind::Script(n) if n == crate::pool::cinema::EXIT) {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world
                        .import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
        }
        if upgrade_pandemonium || upgrade_fortress || upgrade_beyond {
            // These script contacts had no implemented consequences in previous builds.
            for t in &mut self.triggers {
                if matches!(t.kind, TriggerKind::Script(_)) {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world
                        .import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
            for d in &mut self.doors {
                d.locked = upgrade_pandemonium && d.name == "t141";
            }
        }
        if self.map_name == "potears2"
            && s.levels.get("potears2").is_some_and(|s| s["version"].as_u64().is_some_and(|v| v < 3))
        {
            for t in &mut self.triggers {
                if matches!(t.id.0, 4 | 72 | 86 | 93 | 198) && t.reported {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world.import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
        }
        if self.map_name == "garden1" && s.levels.get("garden1").is_some_and(|s| s.get("world").is_none()) {
            for t in &mut self.triggers {
                if matches!(t.id.0,85|207|62) && t.reported {
                    t.fired=false;t.reported=false;t.inside=false;t.cooldown=0.;
                    self.event_world.import_unhandled(&format!("trigger/{}",t.id.0));
                }
            }
        }
        if let Some(upgrading) = upgrading {
            self.finish_level_upgrade(upgrading)?;
        }
        // Only previously unhandled vial hints are rearmed. Successful/new hints
        // retain their saved one-shot clock and never regrant the nearby pickup.
        if self.school2.as_ref().is_some_and(|s| s.rage_hint_unseen()) {
            for t in &mut self.triggers {
                if matches!(&t.kind, TriggerKind::Script(thread) if thread == crate::school2::rage_hint::EVENT)
                    && t.reported
                {
                    t.fired = false;
                    t.reported = false;
                    t.inside = false;
                    t.cooldown = 0.;
                    self.event_world.import_unhandled(&format!("trigger/{}", t.id.0));
                }
            }
        }
        if self.map_name == "garden4" {
            let history = self.triggers.iter().filter(|t| t.fired || t.reported)
                .map(|t| t.id).collect::<Vec<_>>();
            crate::levels::garden4::restore_course_history(&mut self.levels, map, &history)?;
        }
        if s.traversal_version == 0 {
            self.init_traversal(map)?;
        }
        if upgrade_school2 {
            if let (Some(school), Some(enemies)) = (&self.school2, &mut self.encounters) {
                school.sync_encounters(enemies, &mut self.event_world);
            }
        }
        self.sync_entity_flags();
        Ok(())
    }
}
impl Snapshot {
    pub fn has_pool(&self) -> bool {
        self.pool.is_some()
    }
    pub fn has_pool_boulders(&self) -> bool {
        self.pool.as_ref().is_some_and(|s| s.boulders.is_some())
    }
    pub fn has_pool_arrival(&self) -> bool {
        self.pool.as_ref().is_some_and(|s| s.cinema.version >= 3)
    }
    pub fn has_pool_pilot(&self) -> bool {
        self.pool.as_ref().is_some_and(|s| s.cinema.version >= 2)
    }
    pub fn has_beyond(&self) -> bool {
        self.beyond.is_some()
    }
    pub fn has_fortress(&self) -> bool {
        self.fortress.is_some()
    }
    pub fn has_traversal(&self) -> bool {
        self.traversal_version == 1
    }
    pub fn has_duchess(&self) -> bool {
        self.duchess.is_some()
    }
    pub fn has_school_return(&self) -> bool {
        self.school
            .as_ref()
            .is_some_and(|s| s.return_visit.is_some())
    }
    pub fn has_shared_state(&self) -> bool {
        self.shared.is_some()
    }
    pub fn has_encounters(&self) -> bool {
        self.encounters.is_some()
    }
    pub fn has_pandemonium_cinema(&self) -> bool {
        self.pandemonium
            .as_ref()
            .is_some_and(|p| p.cinema.version == 1)
    }
    pub fn has_pandemonium(&self) -> bool {
        self.pandemonium.is_some()
    }
}

pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    let specs = crate::texture::read_materials(assets)?;
    let mut doors = 0;
    let mut triggers = 0;
    let mut fogs = 0;
    let mut distance = 0;
    let mut pending = BTreeSet::new();
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let interaction = Interactions::load(&map)?;
        let atmosphere = crate::environment::Atmosphere::load(assets, &name, &map, &specs)?;
        doors += interaction.doors.len();
        triggers += interaction.triggers.len();
        fogs += atmosphere.volumes.len();
        distance += usize::from(atmosphere.distance.w > 0.);
        for e in &map.entities {
            if let Some(thread) = e.get("thread") {
                pending.insert(format!("{name}: {thread}"));
            }
        }
        for trigger in &interaction.triggers {
            if let TriggerKind::Exit(next, target) = &trigger.kind {
                anyhow::ensure!(
                    assets.contains(&format!("maps/{next}.bsp")),
                    "Exit points to missing map {next}"
                );
                if let Some(target) = target {
                    let destination = Bsp::parse(&assets.read(&format!("maps/{next}.bsp"))?)?;
                    anyhow::ensure!(
                        destination.entities.iter().any(|e| e
                            .get("classname")
                            .is_some_and(|c| c == "info_player_start")
                            && e.get("targetname") == Some(target)),
                        "Exit points to missing start {next}${target}"
                    );
                }
            }
        }
    }
    println!("World data: {doors} doors, {triggers} triggers, {fogs} fog volumes, {distance} unambiguous distance-fog levels; {} pending named script entry points. No map completion claim.",pending.len());
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    check_school_door(&map)?;
    check_school_floor(assets, &map)?;
    Ok(())
}

fn check_school_floor(assets: &mut crate::assets::Assets, map: &Bsp) -> Result<()> {
    use crate::movement::{Controls, FixedClock};
    for entry in [None, Some("skool1_start2")] {
        let mut world = World::from_bsp(map)?;
        let mut interactions = Interactions::load(map)?;
        interactions.set_entry(assets, map, "skool1", entry)?;
        interactions.sync(&mut world);
        anyhow::ensure!(interactions.platforms.len() == 8, "Missing steam platforms");
        for (_, origin, _) in &interactions.platforms {
            let mut p = Player::spawn(&world, *origin + Vec3::Z * 100.).unwrap();
            for _ in 0..240 {
                p.tick(&world, Controls::default());
            }
            anyhow::ensure!(
                p.grounded && (p.feet.z - (origin.z + 4.)).abs() < 0.1 && world.body_clear(p.feet),
                "Steam platform did not catch Alice: {:?}",
                p.feet
            );
        }
        for fps in [30, 60, 144] {
            let mut p = Player::spawn(&world, vec3(-3580., 2432., -624. + 48.)).unwrap();
            let mut clock = FixedClock::default();
            for _ in 0..fps * 2 {
                clock.advance(
                    1. / fps as f64,
                    &world,
                    &mut p,
                    Controls {
                        wish: Vec2::Y,
                        ..Default::default()
                    },
                );
                anyhow::ensure!(
                    p.feet.z >= -625. && world.body_clear(p.feet),
                    "Fell through steam floor at {fps} Hz: {:?}",
                    p.feet
                );
            }
            anyhow::ensure!(p.feet.y > 2800., "Steam floor path blocked: {:?}", p.feet);
            println!("PASS steam floor {entry:?}, {fps} Hz: {:?}", p.feet);
        }
        // This authored falling-death volume is elsewhere in the school, not under the theatre.
        let e = interactions.triggers(0.1, vec3(692., 4364., 120.), vec3(692., 4364., -20.));
        let mut stats = crate::inventory::Stats::default();
        stats.damage(e.damage);
        anyhow::ensure!(!stats.alive(), "School fatal-fall volume failed");
        interactions.reset_contacts();
        anyhow::ensure!(
            interactions
                .triggers(0.1, vec3(692., 4364., 120.), vec3(692., 4364., -20.))
                .damage
                >= 100.,
            "Fatal volume did not reset on retry"
        );
    }
    println!("PASS both school entries: all eight platforms support Alice, walking crosses the seams, fatal falls kill and reset");
    Ok(())
}

fn check_school_door(map: &Bsp) -> Result<()> {
    use crate::movement::{Controls, FixedClock};
    for fps in [30, 60, 144] {
        let mut world = World::from_bsp(map)?;
        let mut interactions = Interactions::load(map)?;
        interactions.sync(&mut world);
        let mut player = Player::spawn(&world, map.spawn().0)
            .ok_or_else(|| anyhow::anyhow!("School start obstructed"))?;
        let mut clock = FixedClock::default();
        let dt = 1. / fps as f32;
        let input = Controls {
            wish: Vec2::Y,
            ..Default::default()
        };
        for _ in 0..fps * 4 {
            clock.advance(dt as f64, &world, &mut player, input);
        }
        anyhow::ensure!(
            player.feet.y > 2100. && player.feet.y < 2212.,
            "School door did not block player: {:?}",
            player.feet
        );
        // The paired door's centre is offset from the seam; approach either leaf.
        interactions.update(dt, map, &mut world, &player, Vec3::Y, true)?;
        anyhow::ensure!(
            interactions.used == 1,
            "School door was not reachable with E"
        );
        for _ in 0..fps * 2 {
            interactions.update(dt, map, &mut world, &player, Vec3::Y, false)?;
            clock.advance(dt as f64, &world, &mut player, input);
            anyhow::ensure!(world.body_clear(player.feet), "Door embedded player");
        }
        anyhow::ensure!(
            player.feet.y > 2260.,
            "Opened door did not allow passage: {:?}",
            player.feet
        );
        println!("PASS school door route at {fps} Hz: closed leaf blocks, E opens both, player walks through to {:?}",player.feet);
    }
    Ok(())
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture() -> Bsp {
        use crate::bsp::{MapBrush, Model, Plane, Shader};
        let boxes = [
            (vec3(-500., -500., -20.), vec3(500., 500., 0.)),
            (vec3(0., -3., 0.), vec3(64., 3., 96.)),
        ];
        let mut planes = Vec::new();
        for (min, max) in boxes {
            for axis in 0..3 {
                let mut n = Vec3::ZERO;
                n[axis] = 1.;
                planes.push(Plane {
                    normal: n,
                    distance: max[axis],
                });
                planes.push(Plane {
                    normal: -n,
                    distance: -min[axis],
                });
            }
        }
        let door = Entity::from([
            ("classname".into(), "func_rotatingdoor".into()),
            ("model".into(), "*1".into()),
            ("origin".into(), "0 100 0".into()),
        ]);
        Bsp {
            visibility: Default::default(),
            difficulty: Default::default(),
            models: boxes
                .into_iter()
                .enumerate()
                .map(|(i, (min, max))| Model {
                    min,
                    max,
                    surfaces: 0..0,
                    brushes: i..i + 1,
                })
                .collect(),
            fogs: Vec::new(),
            shaders: vec![Shader {
                name: String::new(),
                flags: 0,
                contents: 1,
            }],
            vertices: Vec::new(),
            indices: Vec::new(),
            surfaces: Vec::new(),
            lightmaps: Vec::new(),
            entities: vec![door],
            world_surfaces: 0..0,
            planes,
            side_planes: (0..12).collect(),
            brushes: vec![
                MapBrush {
                    sides: 0..6,
                    shader: 0,
                },
                MapBrush {
                    sides: 6..12,
                    shader: 0,
                },
            ],
            world_brushes: 0..1,
            world_min: boxes[0].0,
            world_max: boxes[0].1,
        }
    }
    #[test]
    fn doors_require_reach_visibility_and_pause_and_do_not_crush() {
        let map = fixture();
        let mut world = World::from_bsp(&map).unwrap();
        let mut i = Interactions::load(&map).unwrap();
        i.sync(&mut world);
        let mut player = Player::new(vec3(32., 40., 0.));
        assert!(i.prompt(&world, player.eye(), Vec3::Y).is_some());
        assert!(i.prompt(&world, player.eye(), -Vec3::Y).is_none());
        assert!(i.prompt(&world, vec3(32., -100., 48.), Vec3::Y).is_none());
        i.update(0., &map, &mut world, &player, Vec3::Y, true)
            .unwrap();
        assert_eq!(i.used, 0);
        i.update(0.05, &map, &mut world, &player, Vec3::Y, true)
            .unwrap();
        for _ in 0..30 {
            i.update(0.05, &map, &mut world, &player, Vec3::Y, false)
                .unwrap();
        }
        assert_eq!(i.doors[0].angle, 90.);
        player.feet = vec3(23., 123., 0.);
        assert!(world.body_clear(player.feet));
        i.doors[0].target = 0.;
        for _ in 0..30 {
            i.update(0.05, &map, &mut world, &player, Vec3::Y, false)
                .unwrap();
            assert!(world.body_clear(player.feet));
        }
        assert!(i.doors[0].angle > 1.);
        // A solid wall closer than the handle prevents interaction through it.
        let wall = World::fixture(&[(vec3(-100., 55., 0.), vec3(100., 65., 100.))]);
        assert!(i.prompt(&wall, vec3(32., 40., 48.), Vec3::Y).is_none());
    }
    #[test]
    fn triggers_use_swept_volume_contacts_pause_and_reentry() {
        let mut map = fixture();
        map.entities = vec![Entity::from([
            ("classname".into(), "trigger_changelevel".into()),
            ("model".into(), "*1".into()),
            ("origin".into(), "0 100 0".into()),
            ("map".into(), "next$entry".into()),
        ])];
        let mut i = Interactions::load(&map).unwrap();
        let outside = vec3(32., 40., 0.);
        let inside = vec3(32., 100., 0.);
        assert!(i.triggers(0., outside, inside).transition.is_none());
        assert_eq!(
            i.triggers(0.1, outside, inside).transition,
            Some(("next".into(), Some("entry".into())))
        );
        assert!(i.triggers(2., inside, inside).transition.is_none());
        i.triggers(0.1, inside, outside);
        assert!(i.triggers(0.1, outside, inside).transition.is_some());
        i.reset_contacts();
        assert!(i
            .triggers(0.1, outside, vec3(32., 160., 0.))
            .transition
            .is_some());
    }
    #[test]
    fn destination_is_data_not_path_or_command() {
        assert_eq!(
            destination("next$entrance"),
            Some(("next".into(), Some("entrance".into())))
        );
        for bad in ["../next", "next$one$two", "next;quit", "", "next$"] {
            assert!(destination(bad).is_none());
        }
    }
    #[test]
    fn transforms_are_finite() {
        assert!(vector("NaN 0 0").is_none());
        assert!(vector("1 2").is_none());
        assert_eq!(vector("1 2 3"), Some(vec3(1., 2., 3.)));
    }
    #[test]
    fn fatal_fall_is_swept_paused_and_not_a_recovery_destination() {
        let mut map = fixture();
        map.entities[0].insert("classname".into(), "trigger_fall".into());
        let mut i = Interactions::load(&map).unwrap();
        let before = vec3(32., 40., 0.);
        let after = vec3(32., 160., 0.);
        assert_eq!(i.triggers(0., before, after).damage, 0.);
        assert!(i.triggers(0.1, before, after).damage >= 100.);
        assert!(i.hazardous(vec3(32., 100., 0.)));
        assert!(!i.hazardous(before));
        i.reset_contacts();
        assert!(i.triggers(0.1, before, after).damage >= 100.);
    }
}

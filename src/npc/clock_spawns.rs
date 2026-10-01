//! Bounded touch-trigger/relay links for Clockwork Automaton ambushes; never executes scripts.
use super::*;
use crate::collision::{Collider, PLAYER_HALF};
pub struct Ambush {
    volume: Collider,
    pub center: Vec3,
    pub receivers: Vec<(usize, f32)>,
}
impl Ambush {
    pub fn touches(&self, eye: Vec3) -> bool {
        self.volume
            .trace(eye - Vec3::Z * 20., eye - Vec3::Z * 20., PLAYER_HALF)
            .start_solid
    }
}
fn enabled(e: &Entity, map: &Bsp) -> bool {
    map.difficulty.allows(number(e, "spawnflags", 0.) as u32)
}
pub fn ambushes(map: &Bsp, name: &str) -> Result<Vec<Ambush>> {
    if !matches!(name, "funhouse" | "hatter1" | "hedge2" | "hedge3") {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for trigger in &map.entities {
        if !matches!(
            trigger.get("classname").map(String::as_str),
            Some("trigger_once" | "trigger_multiple")
        ) || !enabled(trigger, map)
            || (trigger.contains_key("thread") && !(name == "funhouse" && trigger.get("thread").is_some_and(|s| matches!(s.as_str(), "auto01spawn"|"auto02spawn"|"auto03spawn"|"auto04spawn"|"auto05spawn"|"auto06spawn"))))
            || number(trigger, "health", 0.) > 0.
            || number(trigger, "spawnflags", 0.) as u32 & 8 != 0
        {
            continue;
        }
        let reviewed = trigger.get("thread").map(|s| s.trim_end_matches("spawn").to_owned());
        let Some(target) = reviewed.as_ref().or_else(|| trigger.get("target")) else {
            continue;
        };
        let mut queue = vec![(target.clone(), number(trigger, "delay", 0.).clamp(0., 30.))];
        let mut visited = BTreeSet::new();
        let mut receivers = BTreeMap::<usize, f32>::new();
        for _ in 0..64 {
            let Some((name, delay)) = queue.pop() else {
                break;
            };
            if !visited.insert(name.clone()) {
                continue;
            }
            for (id, e) in map
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.get("targetname") == Some(&name) && enabled(e, map))
            {
                let class = e.get("classname").map(String::as_str).unwrap_or("");
                if class == "trigger_relay" && !e.contains_key("thread") {
                    if let Some(next) = e.get("target") {
                        queue.push((
                            next.clone(),
                            (delay + number(e, "delay", 0.).max(0.)).min(30.),
                        ));
                    }
                    continue;
                }
                let spawning = class == "func_spawn";
                let model = e
                    .get(if spawning { "modelname" } else { "model" })
                    .and_then(|s| model_name(&s.to_lowercase()));
                let flags = number(e, "spawnflags", 0.) as u32;
                if model.as_deref() == Some(crate::clockwork::MODEL)
                    && (spawning
                        || (class.eq_ignore_ascii_case("Enemies_ClockworkAutomaton")
                            && flags & !0x700 == 64))
                {
                    receivers
                        .entry(id)
                        .and_modify(|d| *d = d.min(delay))
                        .or_insert(delay);
                }
            }
        }
        if receivers.is_empty() {
            continue;
        }
        let Some(index) = trigger
            .get("model")
            .and_then(|s| s.strip_prefix('*'))
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        let origin = trigger
            .get("origin")
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO);
        let volume = Collider::model(map, index, origin, Quat::IDENTITY, false)?;
        result.push(Ambush {
            volume,
            center: origin + (map.models[index].min + map.models[index].max) * 0.5,
            receivers: receivers.into_iter().collect(),
        });
    }
    Ok(result)
}
pub fn placements(map: &Bsp, name: &str) -> Vec<Spawn> {
    let Ok(ambushes) = ambushes(map, name) else {
        return Vec::new();
    };
    let mut ids: BTreeSet<_> = ambushes
        .iter()
        .flat_map(|a| a.receivers.iter().map(|(id, _)| *id))
        .collect();
    if name == "hedge2" {
        // The authored lever reaches these two spawns through water_door_relays.
        ids.extend([34, 567].into_iter().filter(|id| enabled(&map.entities[*id], map)));
    }
    ids.into_iter()
        .filter_map(|id| {
            let e = &map.entities[id];
            let spawning = e.get("classname").is_some_and(|s| s == "func_spawn");
            let scale = number(e, "scale", 1.);
            if !(0.01..=10.).contains(&scale) {
                return None;
            }
            Some(Spawn {
                resident_spawn: None,
                clock_spawn: Some(id),
                imp_spawn: None,
                chess_spawn: None,
                difficulty_variant: number(e, "spawnflags", 0.) as u32 & 0x700 != 0,
                name: e
                    .get(if spawning {
                        "spawntargetname"
                    } else {
                        "targetname"
                    })
                    .cloned()
                    .unwrap_or_default(),
                hidden: false,
                model: crate::clockwork::MODEL.into(),
                origin: vector(e.get("origin")?)?,
                yaw: number(e, "angle", 0.).to_radians(),
                scale,
                animation: None,
            })
        })
        .collect()
}

//! Bounded touch-trigger/relay links for shared resident ambushes; never executes scripts.
use super::*;
use crate::collision::{Collider, PLAYER_HALF};
pub struct Ambush {
    volume: Collider,
    pub center: Vec3,
    pub receivers: Vec<(usize, f32)>,
    pub patrols: Vec<String>,
    pub wake: Vec<String>,
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
    let mut result = Vec::new();
    for trigger in &map.entities {
        if !matches!(
            trigger.get("classname").map(String::as_str),
            Some("trigger_once" | "trigger_multiple")
        ) || !enabled(trigger, map)
            || number(trigger, "health", 0.) > 0.
            || number(trigger, "spawnflags", 0.) as u32 & 8 != 0
        {
            continue;
        }
        let base_delay = number(trigger, "delay", 0.).clamp(0., 30.);
        let mut queue = if let Some(thread) = trigger.get("thread") {
            thread_targets(name, thread)
                .into_iter()
                .map(|(s, d)| (s.to_owned(), (d + base_delay).min(30.)))
                .collect()
        } else {
            trigger
                .get("target")
                .map(|s| vec![(s.clone(), base_delay)])
                .unwrap_or_default()
        };
        // This reviewed Garden2 trigger has both a direct Antlion target and a Ladybug callback.
        if name == "garden2"
            && trigger.get("thread").is_some_and(|s| s == "Spawn_Lady2")
            && trigger.get("target").is_some_and(|s| s == "t133")
        {
            queue.push(("t133".into(), base_delay));
        }
        let thread = trigger
            .get("thread")
            .map(|s| s.trim_end_matches("()"))
            .unwrap_or("");
        let (patrols, wake): (Vec<String>, Vec<String>) = match (name, thread) {
            ("funhouse", "L_KILL_EM1") => (vec![], vec!["L_wall_death1".into(),"L_wall_death2".into()]),
            ("funhouse", "L_KILL_EM2") => (vec![], vec!["L_wall_death3".into(),"L_wall_death4".into()]),
            ("funhouse", "R_KILL_EM1") => (vec![], vec!["R_wall_death1".into(),"R_wall_death2".into()]),
            ("funhouse", "R_KILL_EM2") => (vec![], vec!["R_wall_death3".into(),"R_wall_death4".into()]),
            ("hatter1", "chairspiders") => (vec![], vec!["get_her1".into(), "get_her2".into()]),
            ("hatter1", "spider_wall12") => {
                (vec![], vec!["spider_wall1".into(), "spider_wall2".into()])
            }
            ("hatter1", "spider_wall67") => {
                (vec![], vec!["spider_wall6".into(), "spider_wall7".into()])
            }
            ("hatter1", "spider_wall5") => (vec![], vec!["spider_wall5".into()]),
            ("hatter1", "spider_wall8") => (vec![], vec!["spider_wall8".into()]),
            ("hatter1", "end_spider") => (vec![], vec!["end_spider".into()]),
            ("potears2", "ladies1and2") => (vec!["lady1".into(), "lady2".into()], vec![]),
            ("potears2", "ladies3and4") => (vec!["lady3".into(), "lady4".into()], vec![]),
            ("potears2", "ladies5and6") => (vec!["lady5".into(), "lady6".into()], vec![]),
            ("garden1", "Lady_Bug1_On") => (vec![], vec!["lady1".into()]),
            ("garden1", "Lady_Bug2_On") => (vec![], vec!["lady2".into()]),
            _ => (vec![], vec![]),
        };
        let mut visited = BTreeSet::new();
        let mut receivers = BTreeMap::<usize, f32>::new();
        for _ in 0..64 {
            let Some((target_name, delay)) = queue.pop() else {
                break;
            };
            if !visited.insert(target_name.clone()) {
                continue;
            }
            for (id, e) in map
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.get("targetname") == Some(&target_name) && enabled(e, map))
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
                if model
                    .as_deref()
                    .is_some_and(|m| resident::supported(m) && !resident::legacy(name, m))
                    && (spawning
                        || (class.to_lowercase().starts_with("enemies_") && flags & !0x700 == 64))
                {
                    receivers
                        .entry(id)
                        .and_modify(|d| *d = d.min(delay))
                        .or_insert(delay);
                }
            }
        }
        if receivers.is_empty() && patrols.is_empty() && wake.is_empty() {
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
        let center = volume
            .interior_point()
            .context("Resident trigger has no interior")?;
        result.push(Ambush {
            volume,
            patrols,
            wake,
            center,
            receivers: receivers.into_iter().collect(),
        });
    }
    Ok(result)
}
pub fn placements(map: &Bsp, name: &str) -> Vec<Spawn> {
    let Ok(ambushes) = ambushes(map, name) else {
        return Vec::new();
    };
    let ids: BTreeSet<_> = ambushes
        .iter()
        .flat_map(|a| a.receivers.iter().map(|(id, _)| *id))
        .collect();
    // New families append after the already shipped resident slots: saved actor/loot IDs stay stable.
    let mut ids: Vec<_> = ids.into_iter().collect();
    ids.sort_by_key(|id| {
        let e = &map.entities[*id];
        let generation = e
            .get("modelname")
            .or_else(|| e.get("model"))
            .and_then(|s| model_name(&s.to_lowercase()))
            .map_or(0, |m| {
                if crate::wildlife::Kind::from_model(&m).is_some() {
                    5
                } else if m == crate::magma::MODEL {
                    4
                } else if crate::burrow::Kind::from_model(&m).is_some() {
                    3
                } else if crate::snark::Kind::from_model(&m).is_some() {
                    2
                } else if crate::cards::Kind::from_model(&m).is_some() {
                    1
                } else {
                    0
                }
            });
        (generation, *id)
    });
    ids.into_iter()
        .filter_map(|id| {
            let e = &map.entities[id];
            let spawning = e.get("classname").is_some_and(|s| s == "func_spawn");
            let scale = number(e, "scale", 1.);
            if !(0.01..=10.).contains(&scale) {
                return None;
            }
            Some(Spawn {
                resident_spawn: Some(id),
                clock_spawn: None,
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
                model: e
                    .get(if spawning { "modelname" } else { "model" })
                    .and_then(|s| model_name(&s.to_lowercase()))?,
                origin: vector(e.get("origin")?)?,
                yaw: number(e, "angle", 0.).to_radians(),
                scale,
                animation: None,
            })
        })
        .collect()
}

/// Reviewed spawn-only callbacks. Puzzle, cutscene and boss callbacks remain with their owner.
fn thread_targets(map: &str, thread: &str) -> Vec<(&'static str, f32)> {
    match (map, thread.trim_end_matches("()")) {
        ("funhouse", "phant01spawn") => vec![("phant01",0.)],
        ("funhouse", "phant02spawn") => vec![("phant02",0.)],
        ("garden1", "Spawn_Ladybug4") => vec![("get_lady14", 0.)],
        ("garden1", "Spawn_Ladybug3") => vec![("get_lady12", 0.), ("get_lady13", 1.6)],
        ("garden1", "Spawn_Ladybug2") => vec![("get_lady10", 0.)],
        ("garden1", "Spawn_Ladybug") => vec![("get_lady4", 0.), ("get_lady11", 1.6)],
        ("garden1", "Lady_Bug4_On") => {
            vec![("get_lady2", 0.), ("get_lady3", 1.6), ("get_lady9", 3.2)]
        }
        ("garden1", "Lady_Bug3_On") => vec![("get_lady6", 0.)],
        _ => Vec::new(),
    }
}
pub fn patrol(map: &Bsp, name: &str, actor: &str) -> Result<Vec<Vec3>> {
    let path = match (name, actor) {
        ("potears2", a) if a.starts_with("lady") => return crate::ladybug::route_for(map, a),
        ("centipede1", "lady1") => "t9",
        ("centipede1", "lady2") => "t48",
        ("centipede1", "lady3") => "t18",
        ("centipede1", "lady4") => "t26",
        ("garden1", "lady1") => "t177",
        ("garden1", "lady2") => "t195",
        ("garden1", "lady3") => "t341",
        ("garden1", "lady4" | "lady11") => "t388",
        ("garden1", "lady5") => "t312",
        ("garden1", "lady6") => "t242",
        ("garden1", "lady7" | "lady8" | "lady9") => "t414",
        ("garden1", "lady10") => "t423",
        ("garden1", "lady12" | "lady13") => "t436",
        ("garden1", "lady14") => "t444",
        ("garden4", "ladybug01") => "t2215",
        ("garden4", "ladybug02") => "t2200",
        ("garden4", "ladybug03") => "t2209",
        _ => return Ok(Vec::new()),
    };
    crate::ladybug::route_from(map, path)
}

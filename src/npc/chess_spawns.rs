//! Reviewed ambush receivers, separate from chess lessons, duels and royal scenes.
use super::*;
use crate::collision::{Collider, PLAYER_HALF};

pub struct Ambush {
    volume: Collider,
    pub center: Vec3,
    pub entities: Vec<usize>,
    pub suppress: Vec<usize>,
    pub delay: f32,
}
/// Only callbacks whose supported consequence is a one-shot enemy spawn.
pub fn thread_target(map: &str, thread: &str) -> Option<String> {
    match map {
        "rchess1" => (1..=10)
            .find(|n| thread == format!("enemy_group{n}_thread"))
            .map(|n| format!("spawn_enemy_group{n}")),
        "wchess1" => match thread {
            "red_pawns2_thread" => Some("spawn_red_pawn2".into()),
            "red_pawns4_thread" => Some("spawn_red_pawn4".into()),
            _ => None,
        },
        "wchess2" => (1..=3)
            .flat_map(|n| ['a', 'b'].map(move |s| (n, s)))
            .find(|(n, s)| thread == format!("enemy_group{n}{s}_thread"))
            .map(|(n, s)| format!("spawn_enemy_group{n}{s}")),
        _ => None,
    }
}
fn enabled(e: &Entity, map: &Bsp) -> bool {
    map.difficulty.allows(number(e, "spawnflags", 0.) as u32)
}
pub fn ambushes(map: &Bsp, name: &str) -> Result<Vec<Ambush>> {
    if !matches!(name, "wforest" | "wchess1" | "wchess2" | "rchess1") {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    for e in &map.entities {
        if e.get("classname").is_none_or(|c| c != "trigger_once")
            || !enabled(e, map)
            || number(e, "health", 0.) > 0.
            || number(e, "spawnflags", 0.) as u32 & 8 != 0
        {
            continue;
        }
        let target = if let Some(thread) = e.get("thread") {
            thread_target(name, thread)
        } else {
            e.get("target").cloned()
        };
        let thread = e.get("thread").map(String::as_str).unwrap_or("");
        let mut cancel_names = Vec::new();
        if name == "rchess1" && thread == "enemy_group5_thread" {
            cancel_names.push("spawn_enemy_group3".to_owned());
        }
        if name == "wchess2" {
            for n in 1..=3 {
                for side in ['a', 'b'] {
                    if thread == "start_battle_group2_thread"
                        || thread == format!("enemy_group{n}{side}_thread")
                    {
                        let other = if side == 'a' { 'b' } else { 'a' };
                        cancel_names.push(format!("spawn_enemy_group{n}{other}"));
                    }
                }
            }
        }
        let suppress = map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.get("classname").is_some_and(|s| s == "func_spawn")
                    && r.get("targetname")
                        .is_some_and(|s| cancel_names.contains(s))
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if target.is_none() && suppress.is_empty() {
            continue;
        }
        let mut entities = Vec::new();
        let mut names = target.into_iter().collect::<Vec<_>>();
        let mut seen = BTreeSet::new();
        // Resolve only authored relay links, with a strict cycle/depth bound.
        for _ in 0..16 {
            let Some(target) = names.pop() else {
                break;
            };
            if !seen.insert(target.clone()) {
                continue;
            }
            for (id, r) in map
                .entities
                .iter()
                .enumerate()
                .filter(|(_, r)| r.get("targetname") == Some(&target) && enabled(r, map))
            {
                let class = r.get("classname").map(String::as_str).unwrap_or("");
                let model = r
                    .get(if class == "func_spawn" {
                        "modelname"
                    } else {
                        "model"
                    })
                    .and_then(|s| model_name(&s.to_lowercase()));
                if model
                    .as_deref()
                    .and_then(crate::chess::Kind::from_model)
                    .is_some()
                    && (class == "func_spawn"
                        || (class.starts_with("Characters_")
                            && number(r, "spawnflags", 0.) as u32 & 64 != 0))
                {
                    entities.push(id);
                } else if class == "trigger_relay"
                    && !r.contains_key("thread")
                    && number(r, "delay", 0.) == 0.
                {
                    if let Some(next) = r.get("target") {
                        names.push(next.clone());
                    }
                }
            }
        }
        if entities.is_empty() && suppress.is_empty() {
            continue;
        }
        let Some(index) = e
            .get("model")
            .and_then(|s| s.strip_prefix('*'))
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        let origin = e
            .get("origin")
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO);
        result.push(Ambush {
            volume: Collider::model(map, index, origin, Quat::IDENTITY, false)?,
            center: origin + (map.models[index].min + map.models[index].max) * 0.5,
            entities,
            suppress,
            delay: number(e, "delay", 0.).clamp(0., 30.),
        });
    }
    Ok(result)
}
impl Ambush {
    pub fn touches(&self, eye: Vec3) -> bool {
        // Same body volume as ordinary world triggers; no sight or distance shortcut.
        self.volume
            .trace(eye - Vec3::Z * 20., eye - Vec3::Z * 20., PLAYER_HALF)
            .start_solid
    }
}
pub(super) fn placements(map: &Bsp, name: &str) -> Vec<Spawn> {
    let Ok(ambushes) = ambushes(map, name) else {
        return Vec::new();
    };
    let ids: BTreeSet<_> = ambushes
        .iter()
        .flat_map(|a| a.entities.iter().copied())
        .collect();
    ids.into_iter()
        .filter_map(|id| {
            let e = &map.entities[id];
            let spawning = e.get("classname").is_some_and(|c| c == "func_spawn");
            let model = model_name(
                &e.get(if spawning { "modelname" } else { "model" })?
                    .to_lowercase(),
            )?;
            let scale = number(e, "scale", 1.);
            if !(0.01..=10.).contains(&scale) {
                return None;
            }
            Some(Spawn {
                resident_spawn: None,
                clock_spawn: None,
                imp_spawn: None,
                chess_spawn: Some(id),
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
                model,
                origin: vector(e.get("origin")?)?,
                yaw: number(e, "angle", 0.).to_radians(),
                scale,
                animation: None,
            })
        })
        .collect()
}

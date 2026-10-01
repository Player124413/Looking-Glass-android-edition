//! Dry Landing's missing authored activations; existing actor identities and health survive.
use super::*;
const DEADTREE_RUNNER: &str = "ant_run_deadtree1";
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Saved {
    time: f32,
    fired: BTreeSet<usize>,
    runs: BTreeMap<String, (f32, bool)>,
    #[serde(default)]
    nodes: BTreeMap<String, usize>,
}
struct Contact {
    id: usize,
    volume: crate::collision::Collider,
    thread: String,
    target: String,
}
pub(super) fn can_wait(s: &Spawn) -> bool {
    s.resident_spawn.is_some() || matches!(s.name.as_str(), "ant_run2" | "ant_corner1")
}
pub(super) struct Logic {
    pub saved: Saved,
    contacts: Vec<Contact>,
    paths: BTreeMap<String, Vec<Vec3>>,
}
pub(super) fn extras(map: &Bsp, name: &str) -> Vec<Spawn> {
    if name != "garden1" {
        return vec![];
    }
    let existing: BTreeSet<_> = resident_spawns::placements(map, name)
        .iter()
        .filter_map(|s| s.resident_spawn)
        .collect();
    map.entities
        .iter()
        .enumerate()
        .filter_map(|(id, e)| {
            let flags = number(e, "spawnflags", 0.) as u32;
            if !map.difficulty.allows(flags) || existing.contains(&id) {
                return None;
            }
            let spawning = matches!(id, 79 | 83);
            let model = model_name(e.get(if spawning { "modelname" } else { "model" })?)?;
            if !spawning
                && !(flags & !0x700 == 64
                    && (crate::ant::is_ant(&model) || resident::supported(&model)))
            {
                return None;
            }
            Some(Spawn {
                resident_spawn: Some(id),
                clock_spawn: None,
                imp_spawn: None,
                chess_spawn: None,
                difficulty_variant: flags & 0x700 != 0,
                name: e
                    .get(if spawning {
                        "spawntargetname"
                    } else {
                        "targetname"
                    })
                    .cloned()
                    .unwrap_or_default(),
                hidden: false,
                origin: vector(e.get("origin")?)?,
                yaw: number(e, "angle", 0.).to_radians(),
                scale: number(e, "scale", 1.),
                model,
                animation: None,
            })
        })
        .collect()
}
fn run_spec(name: &str) -> Option<(&'static str, f32, f32)> {
    Some(match name {
        "ant_run2" => ("ant_pos2", 0.2, 1024.),
        "ant_corner1" => ("ant_pos_corner1", 0., 2000.),
        "ant_corp1" => ("ant_corp_pos1", 0., 2000.),
        "ant_corp2" => ("ant_corp_pos2", 0.1, 2000.),
        "ant_run_deadtree1" => ("ant_run_deadtree_pos1", 0., 1200.),
        _ => return None,
    })
}
impl Logic {
    pub fn load(map: &Bsp, name: &str, actors: &mut [Actor]) -> Result<Option<Self>> {
        if name != "garden1" {
            return Ok(None);
        }
        let mut contacts = vec![];
        for (id, e) in map.entities.iter().enumerate().filter(|(_, e)| {
            e.get("classname").is_some_and(|s| s == "trigger_once")
                && map.difficulty.allows(number(e, "spawnflags", 0.) as u32)
        }) {
            let Some(model) = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .and_then(|s| s.parse().ok())
            else {
                continue;
            };
            contacts.push(Contact {
                id,
                volume: crate::collision::Collider::model(
                    map,
                    model,
                    e.get("origin").and_then(|s| vector(s)).unwrap_or_default(),
                    Quat::IDENTITY,
                    false,
                )?,
                thread: e.get("thread").cloned().unwrap_or_default(),
                target: e.get("target").cloned().unwrap_or_default(),
            });
        }
        Self::initial(actors);
        Ok(Some(Self {
            saved: Saved::default(),
            contacts,
            paths: [
                ("ant_run2", vec![408, 461, 409]),
                ("ant_corner1", vec![132, 436]),
                ("ant_corp1", vec![457, 456, 455, 454, 453]),
                ("ant_corp2", vec![427, 428, 632, 429]),
                ("ant_run_deadtree1", vec![604, 492, 603]),
            ]
            .into_iter()
            .map(|(name, ids)| {
                (
                    name.into(),
                    ids.into_iter()
                        .map(|id| vector(&map.entities[id]["origin"]).unwrap())
                        .collect(),
                )
            })
            .collect(),
        }))
    }
    fn initial(actors: &mut [Actor]) {
        for a in actors {
            if let Some(ant) = &mut a.ant {
                if a.spawn.resident_spawn.is_some()
                    || matches!(a.spawn.name.as_str(), "ant_run2" | "ant_corner1")
                {
                    ant.enabled = false;
                }
                if a.spawn.name == "ant_run_deadtree1" {
                    ant.sight_range = Some(1200.);
                }
            }
        }
    }
    pub fn validate(&self, s: Option<&Saved>) -> Result<()> {
        if let Some(s) = s {
            ensure!(
                s.time.is_finite() && (0. ..=1e7).contains(&s.time),
                "Invalid garden cast time"
            );
            ensure!(
                s.fired
                    .iter()
                    .all(|id| self.contacts.iter().any(|c| c.id == *id)),
                "Invalid garden cast contact"
            );
            ensure!(
                s.runs.iter().all(|(n, (t, _))| run_spec(n).is_some()
                    && t.is_finite()
                    && (0. ..=s.time).contains(t)),
                "Invalid garden actor run"
            );
            ensure!(
                s.nodes
                    .iter()
                    .all(|(n, i)| self.paths.get(n).is_some_and(|p| *i <= p.len())),
                "Invalid garden run waypoint"
            );
        }
        Ok(())
    }
    pub fn restore(&mut self, s: Option<&Saved>, actors: &mut [Actor]) -> Result<()> {
        self.validate(s)?;
        if let Some(s) = s {
            self.saved = s.clone();
        } else {
            self.saved = Saved::default();
            // Only previously inert actors are held. No health, corpse, loot or
            // previously active resident state is reset by adding this adapter.
            for a in actors {
                if let Some(ant) = &mut a.ant {
                    if ant.script_wait
                        && matches!(a.spawn.name.as_str(), "ant_run2" | "ant_corner1")
                    {
                        ant.enabled = false;
                    }
                }
            }
        }
        Ok(())
    }
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        eye: Vec3,
        actors: &mut [Actor],
        models: &[Model],
    ) {
        if dt <= 0. {
            return;
        }
        let dt = dt.min(0.1);
        self.saved.time = (self.saved.time + dt).min(1e7);
        for c in &self.contacts {
            if self.saved.fired.contains(&c.id)
                || !c.volume.touches(
                    eye - Vec3::Z * 24.,
                    eye - Vec3::Z * 24.,
                    crate::collision::PLAYER_HALF,
                )
            {
                continue;
            }
            self.saved.fired.insert(c.id);
            let extra = match c.thread.as_str() {
                "Ant_Ambush1" => "lady5",
                "Bridge_Drop" => "lady3",
                _ => "",
            };
            let running = match c.thread.as_str() {
                "Ant_Ambush1" => "ant_run2",
                "Ant_Ambush2" => "ant_corner1",
                "Ant_Ambush3" => "ant_corp1",
                "Ant_Ambush4" => "ant_corp2",
                "Ant_Deadtree_Ambush" => DEADTREE_RUNNER,
                _ => "",
            };
            for a in actors.iter_mut() {
                if (!c.target.is_empty() && a.spawn.name == c.target)
                    || (!extra.is_empty() && a.spawn.name == extra)
                {
                    if let Some(p) = &mut a.resident {
                        if !p.active && p.delay.is_none() {
                            p.delay = Some(0.);
                        }
                    }
                    if let Some(ant) = &mut a.ant {
                        if ant.health > 0. {
                            ant.enabled = true;
                        }
                    }
                }
                if !running.is_empty() && a.spawn.name == running {
                    if let Some(ant) = &mut a.ant {
                        if ant.health > 0. {
                            ant.enabled = true;
                            ant.script_wait = true;
                            self.saved
                                .runs
                                .entry(running.into())
                                .or_insert((self.saved.time, false));
                        }
                    }
                }
            }
        }
        for a in actors.iter_mut() {
            let Some((start, done)) = self.saved.runs.get_mut(&a.spawn.name) else {
                continue;
            };
            let Some(ant) = &mut a.ant else {
                continue;
            };
            let (_dest, delay, sight) = run_spec(&a.spawn.name).unwrap();
            if *done {
                continue;
            }
            if ant.health <= 0. {
                *done = true;
                continue;
            }
            if self.saved.time - *start < delay {
                continue;
            }
            let path = &self.paths[&a.spawn.name];
            let node = self.saved.nodes.entry(a.spawn.name.clone()).or_default();
            if *node >= path.len() {
                *done = true;
                ant.script_wait = false;
                ant.sight_range = Some(sight);
                continue;
            }
            let delta = path[*node] - ant.feet;
            if delta.truncate().length() < 32. && delta.z.abs() < 128. {
                *node += 1;
                continue;
            }
            if ant.phase == crate::ant::Phase::Pain {
                continue;
            }
            let d = &models[a.model].data;
            let clip = &d.clips["walk_medium"];
            ant.phase = crate::ant::Phase::Chase;
            ant.yaw = delta.y.atan2(delta.x);
            ant.feet = walk_script(
                world,
                ant.feet,
                delta.truncate().extend(0.).normalize_or_zero() * clip.distance * d.def.scale
                    / clip.duration()
                    * dt,
                ant.target(0).half,
            );
        }
    }
}

// Scripted runs may cross sloping tree limbs. Keep the swept actor hull and
// legal step height, without combat's conservative four-corner cliff veto.
fn walk_script(world: &World, feet: Vec3, delta: Vec3, half: Vec3) -> Vec3 {
    let center = feet + Vec3::Z * (half.z + 0.1);
    let up = world.sweep(center, center + Vec3::Z * 18., half);
    if up.start_solid {
        return feet;
    }
    let raised = center + Vec3::Z * 18. * up.fraction;
    let across = world.sweep(raised, raised + delta, half);
    if across.start_solid {
        return feet;
    }
    let next = raised + delta * across.fraction;
    let down = world.sweep(next, next - Vec3::Z * 38., half);
    if down.start_solid || down.fraction >= 1. || down.normal.z < 0.65 {
        return feet;
    }
    next - Vec3::Z * (38. * down.fraction + half.z)
}

pub(super) fn check(a: &mut Assets) -> Result<()> {
    let mut map = Bsp::parse(&a.read("maps/garden1.bsp")?)?;
    for difficulty in crate::powerups::Difficulty::ALL {
        map.difficulty = difficulty;
        let world = World::from_bsp(&map)?;
        let mut n = Npcs::load(a, &map, "garden1", None, false, false)?;
        let s = n.snapshot();
        n.restore(&s)?;
        let count = |name: &str| n.actors.iter().filter(|a| a.spawn.name == name).count();
        ensure!(
            count("ant_run2") == usize::from(difficulty != crate::powerups::Difficulty::Easy),
            "Wrong Ant difficulty gate"
        );
        ensure!(
            count("lady3") == 1 && count("lady5") == 1,
            "Missing/duplicate new Ladybugs"
        );
        let ids = [
            121, 120, 118, 113, 109, 101, 85, 207, 111, 114, 69, 72, 75, 80, 99, 86,
        ];
        for id in ids {
            let Some(contact) = n
                .garden
                .as_ref()
                .unwrap()
                .contacts
                .iter()
                .find(|c| c.id == id)
            else {
                continue;
            };
            let at = contact
                .volume
                .interior_point()
                .context("Garden contact lacks interior")?
                + Vec3::Z * 24.;
            n.update(1. / 120., &world, at);
        }
        let saved = n.snapshot();
        let mut other = Npcs::load(a, &map, "garden1", None, false, false)?;
        other.restore(&saved)?;
        let before = serde_json::to_value(&saved)?;
        n.update(0., &world, Vec3::ZERO);
        ensure!(
            serde_json::to_value(n.snapshot())? == before,
            "Garden cast advanced on pause"
        );
        for _ in 0..120 * 25 {
            n.update(1. / 120., &world, vec3(10000., 10000., 10000.));
            other.update(1. / 120., &world, vec3(10000., 10000., 10000.));
        }
        ensure!(
            serde_json::to_value(n.snapshot())? == serde_json::to_value(other.snapshot())?,
            "Garden cast saved future diverged"
        );
        for a in &n.actors {
            if run_spec(&a.spawn.name).is_some() {
                let ant = a.ant.as_ref().context("Garden Ant lacks combat")?;
                println!(
                    "Garden {difficulty:?} run {} done={:?} feet={:?}",
                    a.spawn.name,
                    n.garden.as_ref().unwrap().saved.runs.get(&a.spawn.name),
                    ant.feet
                );
                ensure!(
                    ant.enabled
                        && n.garden
                            .as_ref()
                            .unwrap()
                            .saved
                            .runs
                            .get(&a.spawn.name)
                            .is_some_and(|(_, done)| *done),
                    "Triggered Ant stayed hidden or failed its authored run"
                );
            }
        }
        // The new activations remain ordinary damageable actors, including after death/load.
        for name in ["ant_run_deadtree1", "lady3", "lady5"] {
            let id = n
                .actors
                .iter()
                .position(|a| a.spawn.name == name)
                .context("Missing garden damage fixture")?;
            let health = |n: &Npcs| {
                n.actors[id]
                    .ant
                    .as_ref()
                    .map(|a| a.health)
                    .or_else(|| n.actors[id].resident.as_ref().map(|p| p.health()))
                    .unwrap()
            };
            let before = health(&n);
            ensure!(
                n.hit(combat::Hit {
                    id,
                    damage: 10.,
                    kind: combat::DamageKind::Knife,
                    knockback: Vec3::ZERO
                })
                .is_some()
                    && health(&n) < before,
                "Garden actor ignores damage: {name}"
            );
            n.hit(combat::Hit {
                id,
                damage: 10000.,
                kind: combat::DamageKind::Knife,
                knockback: Vec3::ZERO,
            });
            ensure!(health(&n) == 0., "Garden actor cannot die: {name}");
            let dead = n.snapshot();
            n.restore(&dead)?;
            ensure!(health(&n) == 0., "Garden actor revived on load: {name}");
        }
        let saved = n.snapshot();
        let mut bad = serde_json::to_value(&saved)?;
        bad["garden"]["time"] = serde_json::json!(-1.);
        ensure!(
            n.restore(&serde_json::from_value(bad)?).is_err()
                && serde_json::to_value(n.snapshot())? == serde_json::to_value(&saved)?,
            "Invalid garden cast save partially applied"
        );
        let mut legacy = saved.clone();
        legacy.garden = None;
        n.restore(&legacy)?;
        ensure!(
            n.actors
                .iter()
                .zip(&saved.actors)
                .all(|(a, b)| a.ant.as_ref().map(|a| a.health) == b.ant.as_ref().map(|a| a.health)),
            "Old save lost health"
        );
        println!("PASS Garden cast {difficulty:?}: contacts, difficulty, saved future, pause, legacy health");
    }
    Ok(())
}

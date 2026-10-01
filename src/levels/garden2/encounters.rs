use super::*;
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Lady {
    pub id: usize,
    pub actor: crate::ladybug::Ladybug,
    pub enabled: bool,
    pub delay: Option<f32>,
}
pub(super) fn allowed(map: &Bsp, e: &crate::levels::Entity) -> bool {
    map.difficulty.allows(
        e.get("spawnflags")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    )
}
pub(super) fn load(map: &Bsp) -> Result<(Vec<crate::ant::Ant>, Vec<Lady>)> {
    let mut ants = vec![];
    let mut ladies = vec![];
    for (id, e) in map
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| allowed(map, e))
    {
        let m = e
            .get("model")
            .map_or("", String::as_str)
            .trim_start_matches("models/")
            .trim_end_matches(".tik");
        if matches!(m, "c_armyant" | "c_armyantcorp") {
            let p = data::at(e);
            let mut ant = crate::ant::Ant::new(
                id,
                m == "c_armyantcorp",
                p.translation,
                p.rotation.to_euler(EulerRot::ZYX).0,
            );
            ant.enabled = e
                .get("spawnflags")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
                & 64
                == 0;
            ants.push(ant);
        }
        if matches!(id, 154 | 599 | 76 | 676 | 70 | 736 | 697) {
            let mut route = vec![];
            let mut next = match id {
                154 | 599 => "t94",
                70 | 697 => "t157",
                _ => "t149",
            };
            let mut seen = std::collections::BTreeSet::new();
            while seen.insert(next) {
                let e = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|n| n == next))
                    .context("Missing Garden Ladybug path")?;
                route.push(data::at(e).translation);
                let Some(n) = e.get("target") else { break };
                next = n;
            }
            let p = data::at(e);
            ladies.push(Lady {
                id,
                actor: crate::ladybug::Ladybug::new(
                    p.translation,
                    p.rotation.to_euler(EulerRot::ZYX).0,
                    1.,
                    route,
                ),
                enabled: false,
                delay: None,
            });
        }
    }
    Ok((ants, ladies))
}
impl Garden {
    pub(super) fn spawn_ladies(&mut self, group: u8) {
        let ids: &[(usize, f32)] = match group {
            1 => &[(154, 0.), (599, 2.1)],
            2 => &[(76, 0.), (676, 2.1)],
            3 => &[(736, 0.)],
            _ => &[(70, 0.), (697, 2.1)],
        };
        for &(id, t) in ids {
            if let Some(l) = self.saved.ladies.iter_mut().find(|l| l.id == id) {
                if !l.enabled && l.delay.is_none() {
                    if t == 0. {
                        l.enabled = true;
                        l.actor.patrol_started = true;
                    } else {
                        l.delay = Some(t);
                    }
                }
            }
        }
    }
    pub(super) fn activate_named(&mut self, map: &Bsp, name: &str) {
        for a in &mut self.saved.ants {
            if map.entities[a.id]
                .get("targetname")
                .is_some_and(|n| n == name)
            {
                a.enabled = true;
            }
        }
    }
    pub(super) fn encounter_targets(&self) -> Vec<crate::combat::Target> {
        if self.scripted() {
            return vec![];
        }
        self.saved
            .ants
            .iter()
            .filter(|a| a.enabled && a.health > 0.)
            .map(|a| a.target(BASE + a.id))
            .chain(
                self.saved
                    .ladies
                    .iter()
                    .filter(|l| l.enabled && l.actor.health > 0.)
                    .map(|l| l.actor.target(BASE + l.id)),
            )
            .collect()
    }
    pub(super) fn encounter_hit(&mut self, h: crate::combat::Hit) -> Option<&'static str> {
        if self.scripted() {
            return None;
        }
        if let Some(a) = self
            .saved
            .ants
            .iter_mut()
            .find(|a| a.enabled && BASE + a.id == h.id)
        {
            return a.hit(h);
        }
        self.saved
            .ladies
            .iter_mut()
            .find(|l| l.enabled && BASE + l.id == h.id)?
            .actor
            .hit_attack(h)
    }
    pub(super) fn encounter_combat(
        &mut self,
        c: &mut crate::level::Combat<'_>,
    ) -> crate::combat::Feedback {
        let mut f = crate::combat::Feedback::default();
        if self.scripted() || c.dt <= 0. {
            return f;
        }
        if let Some(t) = &mut self.saved.under_ambush {
            *t = (*t + c.dt).min(8.);
            if let Some(a) = self.saved.ants.iter_mut().find(|a| a.id == 139) {
                let delta = self.data.points["ant_under_pos1"].translation - a.feet;
                if a.health <= 0. || *t >= 8. || (*t >= 1. && delta.truncate().length() < 16.) {
                    *t = 8.;
                    a.script_wait = false;
                    a.sight_range = Some(1500.);
                } else if *t >= 1. && a.phase != crate::ant::Phase::Pain {
                    a.phase = crate::ant::Phase::Chase;
                    a.yaw = delta.y.atan2(delta.x);
                    a.feet = crate::combat::walk_body(
                        c.world,
                        a.feet,
                        delta.truncate().extend(0.).normalize_or_zero()
                            * self.data.speed(a.model(), "walk_medium")
                            * c.dt,
                        a.target(0).half,
                    );
                }
            }
        }
        for a in &mut self.saved.ants {
            if a.enabled {
                a.advance(c, &self.data, &mut f);
            }
        }
        for l in self.saved.ladies.iter_mut().filter(|l| l.enabled) {
            l.actor.notarget = c.notarget;
            l.actor.opponents.summon = c.summon;
            let r = l
                .actor
                .advance(c.dt, c.world, c.player.eye(), self.data.lady);
            f.damage += r.damage;
            f.impulse += r.impulse;
            f.sounds.extend(r.sounds);
            f.spatial_sounds.extend(r.spatial_sounds);
            f.summon_hits.extend(r.summon_hits);
        }
        f
    }
}

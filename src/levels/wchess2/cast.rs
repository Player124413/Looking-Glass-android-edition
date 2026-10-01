//! Stable placed/spawned actors. Faction damage is resolved by this owner, not generic NPCs.
use super::*;
use crate::{
    ant::Timing,
    chess::{Kind, Piece as CombatPiece},
    combat::{DamageKind, Feedback, Hit, Target},
};
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Walk {
    pub points: Vec<Vec3>,
    pub leg: usize,
    pub pause: f32,
    pub remove: bool,
    pub fight_after: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Flight {
    pub velocity: Vec3,
    pub time: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Actor {
    pub flight: Option<Flight>,
    pub pad: Option<usize>,
    pub entity: usize,
    pub instance: usize,
    pub name: String,
    pub source: String,
    pub model: String,
    pub white: bool,
    pub floor: f32,
    pub damageable: bool,
    pub counted: bool,
    pub idle: f32,
    #[serde(default)]
    pub unseen: f32,
    pub foe: Option<String>,
    pub walk: Option<Walk>,
    pub piece: CombatPiece,
}
impl Actor {
    pub fn id(&self) -> usize {
        BASE + self.entity * 4 + self.instance
    }
    pub fn validate(&self) -> Result<()> {
        let mut ordinary = self.piece.clone();
        ensure!(
            self.piece.health.is_finite()
                && self.piece.health >= 0.
                && self.piece.health
                    <= if self.white && self.name.starts_with("battle") {
                        10000.
                    } else {
                        self.piece.kind.health()
                    },
            "Invalid Castling actor health"
        );
        ordinary.health = ordinary.health.min(ordinary.kind.health());
        ordinary.validate()?;
        if let Some(f) = &self.flight {
            ensure!(
                f.velocity.is_finite() && f.velocity.length() < 10000.,
                "Invalid chess leap"
            );
            state::clock("chess flight", f.time, 10.)?;
        }
        state::clock("chess inactivity", self.idle, 15.)?;
        state::clock("chess enemy retention", self.unseen, 10.)?;
        ensure!(
            self.instance < 4
                && self.floor.is_finite()
                && (0. ..=self.piece.kind.health()).contains(&self.floor),
            "Invalid chess actor"
        );
        if let Some(w) = &self.walk {
            ensure!(
                w.leg < w.points.len()
                    && w.points.len() <= 10
                    && w.points.iter().all(|v| v.is_finite()),
                "Invalid chess walk"
            );
            state::clock("chess walk pause", w.pause, 3.)?;
        }
        Ok(())
    }
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Actor>> {
    let mut result = Vec::new();
    for (id, e) in map.entities.iter().enumerate() {
        let flags = e
            .get("spawnflags")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        if !map.difficulty.allows(flags) {
            continue;
        }
        let class = e.get("classname").map(String::as_str).unwrap_or("");
        if !(class.starts_with("Characters_") || class == "func_spawn") {
            continue;
        }
        let model = e
            .get(if class == "func_spawn" {
                "modelname"
            } else {
                "model"
            })
            .map(|v| v.trim_start_matches("models/").trim_end_matches(".tik"))
            .unwrap_or("");
        let red = model.contains("_red_");
        let normalized = if red {
            model.into()
        } else {
            model.replace("c_chess_", "c_chess_red_")
        };
        let Some(kind) = Kind::from_model(&normalized) else {
            continue;
        };
        let name = e
            .get(if class == "func_spawn" {
                "spawntargetname"
            } else {
                "targetname"
            })
            .cloned()
            .unwrap_or_default();
        for instance in 0..if name == "red_pawn_bully" { 4 } else { 1 } {
            let mut p = CombatPiece::new(
                kind,
                data::at(e).translation,
                data::at(e).rotation.to_euler(EulerRot::ZYX).0,
                e.get("scale").and_then(|s| s.parse().ok()).unwrap_or(1.),
                BASE + id * 4 + instance,
            );
            p.active = class != "func_spawn";
            p.script_wait = class != "func_spawn";
            if !red && name.starts_with("battle") {
                p.health = 10000.;
            }
            result.push(Actor {
                flight: None,
                pad: None,
                entity: id,
                instance,
                name: name.clone(),
                source: e.get("targetname").cloned().unwrap_or_default(),
                model: model.into(),
                white: !red,
                floor: if name.starts_with("w_rook") { 10. } else { 0. },
                damageable: true,
                counted: false,
                idle: 0.,
                unseen: 0.,
                foe: None,
                walk: None,
                piece: p,
            });
        }
    }
    Ok(result)
}
impl Castle {
    pub(super) fn activate_entity(&mut self, id: usize, delay: f32) -> bool {
        let mut found = false;
        for a in &mut self.saved.cast {
            if a.entity == id {
                found = true;
                if !a.piece.active
                    && a.piece.spawn_delay.is_none()
                    && a.piece.health > 0.
                    && a.instance == 0
                {
                    a.piece.spawn_delay = Some(delay.max(0.001));
                }
            }
        }
        found
    }
    pub(super) fn spawn(&mut self, source: &str) {
        for a in &mut self.saved.cast {
            if a.source == source
                && !a.piece.active
                && a.piece.spawn_delay.is_none()
                && a.piece.health > 0.
            {
                a.piece.spawn_delay = Some(0.1 + a.instance as f32 * 2.1);
            }
        }
    }
    pub(super) fn show(&mut self, n: &str) {
        for a in &mut self.saved.cast {
            if a.name == n {
                a.piece.active = true;
            }
        }
    }
    pub(super) fn actor(&self, n: &str) -> Option<&Actor> {
        self.saved.cast.iter().find(|a| a.name == n)
    }
    pub(super) fn actor_mut(&mut self, n: &str) -> Option<&mut Actor> {
        self.saved.cast.iter_mut().find(|a| a.name == n)
    }
    pub(super) fn walk(&mut self, n: &str, points: &[&str], remove: bool, fight: bool) {
        let points = points
            .iter()
            .map(|n| self.data.point(n).translation)
            .collect();
        if let Some(a) = self.actor_mut(n) {
            a.piece.time = 0.;
            a.walk = Some(Walk {
                points,
                leg: 0,
                pause: 0.,
                remove,
                fight_after: fight,
            });
            a.piece.script_wait = true;
        }
    }
    pub(super) fn warp_actor(&mut self, n: &str, point: &str) {
        let p = self.data.point(point);
        if let Some(a) = self.actor_mut(n) {
            a.piece.feet = p.translation;
            a.piece.yaw = p.rotation.to_euler(EulerRot::ZYX).0;
            a.walk = None;
        }
    }
    pub(super) fn cast_event(&mut self, n: &str) -> bool {
        if n == "start_battle_group1_thread" || n == "start_battle_group2_thread" {
            if !self.fire(n) {
                return true;
            }
            let group = if n.contains("group1") { 1 } else { 2 };
            self.spawn(&format!("spawn_battle_group{group}"));
            let pairs: &[(&str, &str)] = if group == 1 {
                &[
                    ("battle1_w_knight1", "battle1_r_bishop1"),
                    ("battle1_w_bishop1", "battle1_r_rook1"),
                ]
            } else {
                &[
                    ("battle2_w_knight1", "battle2_r_rook1"),
                    ("battle2_w_bishop1", "battle2_r_knight1"),
                    ("battle2_w_pawn1", "battle2_r_pawn1"),
                    ("battle2_w_pawn2", "battle2_r_pawn2"),
                ]
            };
            for &(x, y) in pairs {
                for (one, other) in [(x, y), (y, x)] {
                    if let Some(a) = self.actor_mut(one) {
                        a.foe = Some(other.into());
                        a.piece.script_wait = false;
                    }
                }
            }
            // Source death counters reference absent actors. Do not invent completed gates.
            return true;
        }
        if let Some(group) = n
            .strip_prefix("enemy_group")
            .and_then(|v| v.strip_suffix("_thread"))
        {
            if !self.group_ready(group) {
                return true;
            }
            self.fire(n);
            self.spawn(&format!("spawn_enemy_group{group}"));
            return true;
        }
        false
    }
    pub(super) fn move_cast(&mut self, dt: f32) {
        for a in &mut self.saved.cast {
            if a.walk.is_some() {
                if let Some(delay) = &mut a.piece.spawn_delay {
                    *delay = (*delay - dt).max(0.);
                    if *delay == 0. {
                        a.piece.active = true;
                        a.piece.spawn_delay = None;
                    }
                }
            }
            if !a.piece.active || a.piece.health <= 0. {
                continue;
            }
            let Some(w) = &mut a.walk else {
                continue;
            };
            a.piece.time += dt;
            if w.pause > 0. {
                w.pause = (w.pause - dt).max(0.);
                continue;
            }
            let delta = w.points[w.leg] - a.piece.feet;
            let speed = self
                .data
                .speed(
                    a.piece.kind.model(),
                    if a.piece.kind == Kind::Pawn {
                        "walk"
                    } else {
                        "walk_1"
                    },
                )
                .max(1.);
            if delta.length() > 0.01 {
                a.piece.yaw = delta.y.atan2(delta.x);
            }
            a.piece.feet += delta.normalize_or_zero() * (speed * dt).min(delta.length());
            if a.piece.feet.distance(w.points[w.leg]) < 0.01 {
                w.leg += 1;
                if w.leg == w.points.len() {
                    a.piece.active = !w.remove;
                    a.piece.script_wait = !w.fight_after;
                    a.damageable |= w.fight_after;
                    a.walk = None;
                } else if a.name == "pawn_victim" {
                    w.pause = 3.;
                }
            }
        }
    }
    pub(super) fn cast_targets(&self) -> Vec<Target> {
        self.saved
            .cast
            .iter()
            .filter(|a| a.piece.active && a.piece.health > 0. && a.damageable)
            .map(|a| a.piece.target(a.id()))
            .collect()
    }
    pub(super) fn cast_hit(&mut self, mut h: Hit) -> Option<&'static str> {
        let a = self
            .saved
            .cast
            .iter_mut()
            .find(|a| a.id() == h.id && a.damageable)?;
        h.damage = h.damage.min((a.piece.health - a.floor).max(0.));
        a.piece.hit(h)
    }
    pub(super) fn fight(&mut self, c: &mut crate::level::Combat<'_>) -> Feedback {
        if c.dt <= 0. || self.scripted() {
            return Feedback::default();
        }
        let dt = c.dt.min(0.1);
        let mut out = Feedback {
            damage: std::mem::take(&mut self.saved.damage),
            ..Default::default()
        };
        let roster: Vec<_> = self
            .saved
            .cast
            .iter()
            .map(|a| {
                (
                    a.name.clone(),
                    a.white,
                    a.piece.active && a.piece.health > 0.,
                    a.piece.target(a.id()),
                    a.damageable,
                )
            })
            .collect();
        let mut hits = Vec::new();
        for a in &mut self.saved.cast {
            if a.walk.is_some() {
                continue;
            }
            let forced = a
                .foe
                .as_ref()
                .and_then(|n| roster.iter().find(|r| &r.0 == n && r.2));
            let foe = forced.or_else(|| {
                roster
                    .iter()
                    .filter(|r| {
                        r.1 != a.white && r.2 && r.4 && r.3.center.distance(a.piece.feet) < 600.
                    })
                    .min_by(|x, y| {
                        x.3.center
                            .distance(a.piece.feet)
                            .total_cmp(&y.3.center.distance(a.piece.feet))
                    })
            });
            // Explicit duels win over proximity. Otherwise red pieces may choose Alice.
            let foe = foe.filter(|f| {
                a.white
                    || forced.is_some()
                    || f.3.center.distance(a.piece.feet) < c.player.eye().distance(a.piece.feet)
            });
            a.piece.notarget = if foe.is_some() {
                false
            } else {
                c.notarget || a.white
            };
            a.piece.opponents.summon = if foe.is_none() && !a.white {
                c.summon
            } else {
                None
            };
            a.piece.threatened((c.threatens)(&a.piece.target(a.id())));
            let mut f = Feedback::default();
            a.piece.update(
                dt,
                c.world,
                foe.map_or(c.player.eye(), |f| f.3.center),
                &self.data,
                &mut f,
            );
            if let Some(v) = foe {
                if f.damage > 0. {
                    hits.push(Hit {
                        id: v.3.id,
                        damage: f.damage,
                        kind: DamageKind::Other,
                        knockback: Vec3::ZERO,
                    });
                }
            } else if !a.white {
                out.damage += f.damage;
                out.impulse += f.impulse;
                out.summon_hits.extend(f.summon_hits);
            }
            out.sounds.extend(f.sounds);
            out.spatial_sounds.extend(f.spatial_sounds);
        }
        for h in hits {
            self.cast_hit(h);
        }
        out
    }
    pub(super) fn cast_loot(&self) -> Vec<crate::loot::Source> {
        self.saved
            .cast
            .iter()
            .filter(|a| !a.white)
            .map(|a| crate::loot::Source {
                id: a.id(),
                feet: a.piece.feet,
                grade: a.piece.kind.grade(),
                dead: a.piece.health == 0.,
            })
            .collect()
    }
}

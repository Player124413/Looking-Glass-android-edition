//! The enemies the shared route fights that no encounter table owns: school two's Boojums and
//! quest guards, and placed club guards. The viewer simulates the guards through `Npcs`, which
//! needs textures, so a headless route keeps the same `Guard` values itself.
use super::{trace, Route};
use crate::{
    assets::Assets,
    bsp::Bsp,
    combat::{Guard, Hit, State, Target, Timing},
    interaction::{Interactions, SHOT_BASE},
    loot::{Grade, Source},
    school2::ENEMY_BASE,
};
use anyhow::Result;

/// The placed club guards of a visit. A controller that owns its cast keeps its own guards,
/// exactly as `Npcs::update` skips them (Pandemonium, the Duchess and registered visits), so
/// those visits get none here.
pub fn guards_of(map: &Bsp, name: &str, interactions: &Interactions) -> Vec<Guard> {
    if interactions.pandemonium.is_some()
        || interactions.duchess.is_some()
        || !interactions.levels.is_empty()
    {
        return Vec::new();
    }
    crate::npc::placed_guards(map, name)
}
/// The guards and, when there are any, their clip timing (it needs the club guard's data).
pub(super) fn place_guards(
    assets: &mut Assets,
    map: &Bsp,
    name: &str,
    interactions: &Interactions,
) -> Result<(Vec<Guard>, Option<Timing>)> {
    let guards = guards_of(map, name, interactions);
    if guards.is_empty() {
        return Ok((guards, None));
    }
    Ok((guards, Some(crate::npc::guard_timing(assets)?)))
}

impl Route {
    pub fn enable_native_cast(&mut self, assets: &mut Assets) -> Result<()> {
        anyhow::ensure!(self.guards.is_empty(), "Native cast would duplicate route guards");
        self.native_cast = Some(crate::npc::Npcs::load(assets, &self.map, &self.name,
            self.entry.as_deref(), false, false)?);
        Ok(())
    }
    pub(super) fn native_step(&mut self, dt: f32) {
        let busy = self.story.busy() || self.interactions.scripted();
        if let Some(n) = &mut self.native_cast {
            n.activate_levels(&self.interactions.levels);
            n.notarget(self.stats.ignores_alice());
            n.level_patrols(&self.interactions.levels);
            let f = n.update(if busy {0.} else {dt}, &self.world, self.player.eye());
            self.stats.damage(f.damage);
            self.player.knockback(f.impulse);
            self.damage += f.damage;
        }
    }
    fn native_foe(&self, id: usize) -> bool {
        self.native_cast.as_ref().is_some_and(|n| n.targets().iter().any(|t| t.id == id))
    }
    /// School two's controller with the viewer's gating: the quest and its Boojums and guards
    /// advance with the world clock (frozen while the Watch stops time), the Boojums ignore Alice
    /// while she is invisible, and Alice's quest items follow the controller's every tick.
    pub(super) fn school2_step(&mut self, world_dt: f32) {
        let ignored = self.stats.ignores_alice()
            || self
                .interactions
                .school
                .as_ref()
                .is_some_and(|s| s.cinematic());
        let busy = self.story.busy();
        let Some(s) = self.interactions.school2.as_mut() else {
            return;
        };
        s.notarget(ignored);
        let (events, feedback) = s.update(world_dt, &self.world, self.player.feet, busy);
        self.stats.school_items = s.quest.items.clone();
        if feedback.damage > 0. {
            trace(|| {
                format!(
                    "enemy t{} -{} at {:?} sanity {}",
                    self.ticks,
                    feedback.damage,
                    self.player.feet,
                    self.stats.sanity()
                )
            });
        }
        self.stats.damage(feedback.damage);
        self.player.knockback(feedback.impulse);
        self.damage += feedback.damage;
        for event in events.story {
            self.story.trigger(&event);
        }
        if let Some(m) = events.message {
            println!("  {m}");
        }
    }
    /// Placed club guards advance whenever the viewer's cast does: not during dialogue or scenes.
    pub(super) fn guard_step(&mut self, world_dt: f32) {
        let Some(timing) = self.guard_timing else {
            return;
        };
        if self.story.busy()
            || self.interactions.scripted()
            || self
                .interactions
                .school
                .as_ref()
                .is_some_and(|s| s.cinematic())
        {
            return;
        }
        let ignored = self.stats.ignores_alice();
        let eye = self.player.eye();
        for g in &mut self.guards {
            g.notarget = ignored;
            let f = g.advance(world_dt, &self.world, eye, timing);
            self.stats.damage(f.damage);
            self.player.knockback(f.impulse);
            self.damage += f.damage;
        }
    }
    pub(super) fn school2_targets(&self) -> Vec<Target> {
        self.interactions
            .school2
            .as_ref()
            .map_or(Vec::new(), |s| s.targets())
    }
    pub(super) fn guard_targets(&self) -> Vec<Target> {
        self.guards
            .iter()
            .enumerate()
            .filter(|(_, g)| g.health > 0.)
            .map(|(i, g)| g.target(i))
            .collect()
    }
    pub(super) fn guard_sources(&self) -> Vec<Source> {
        self.guards
            .iter()
            .enumerate()
            .map(|(id, g)| Source {
                id,
                feet: g.feet,
                grade: Grade::Small,
                dead: g.health <= 0.,
            })
            .collect()
    }
    /// Is this target an enemy the route shoots without being told to (`aim_at`)? Encounter and
    /// boss ids, school two's Boojums and guards, and placed club guards.
    pub(super) fn is_foe(&self, id: usize) -> bool {
        if let Some(s) = crate::level::hit_owner(&self.interactions.levels, id) {
            return self.interactions.levels[s].ctl.hostile_target(id);
        }
        id >= crate::encounters::BASE
            || self.native_foe(id)
            || self.is_school2_target(id)
            || (id < self.guards.len() && id < ENEMY_BASE)
    }
    fn is_school2_target(&self, id: usize) -> bool {
        self.interactions.school2.is_some() && (ENEMY_BASE..SHOT_BASE).contains(&id)
    }
    /// Enemies of these kinds that are awake and able to hurt Alice now.
    pub(super) fn engaged_foe(&self, id: usize) -> Option<bool> {
        if self.interactions.pool.as_ref().is_some_and(|p| {
            p.targets().iter().any(|t| t.id == id)
        }) {
            return Some(true);
        }
        if self.native_foe(id) {
            return Some(self.native_cast.as_ref().unwrap().route_engaged(id, self.player.eye()));
        }
        if self.is_school2_target(id) {
            return Some(true);
        }
        if id < ENEMY_BASE {
            return self.guards.get(id).map(|g| !matches!(g.state, State::Idle));
        }
        None
    }
    /// Does this target have to be walked up to, having no ranged attack?
    pub(super) fn melee_foe(&self, id: usize) -> Option<bool> {
        if id < ENEMY_BASE {
            return (id < self.guards.len()).then_some(true);
        }
        let s = self.interactions.school2.as_ref()?;
        let i = id.checked_sub(ENEMY_BASE).filter(|_| id < SHOT_BASE)?;
        Some(if i < s.boojums.len() {
            false
        } else if i >= 200 {
            true
        } else {
            s.guards.get(i.wrapping_sub(100)).is_some_and(|g| !g.ranged)
        })
    }
    /// Deliver a hit to school two's enemies or a placed club guard, with the weapon's knockback
    /// and damage kind as the viewer does. False when the target is neither.
    pub(super) fn strike(&mut self, hit: Hit) -> bool {
        if self.native_foe(hit.id) {
            self.native_cast.as_mut().unwrap().hit(hit);
            return true;
        }
        if self.is_school2_target(hit.id) {
            if let Some(s) = self.interactions.school2.as_mut() {
                s.hit_attack(hit);
            }
            return true;
        }
        if hit.id < ENEMY_BASE {
            if let Some(g) = self.guards.get_mut(hit.id) {
                g.hit(hit);
                return true;
            }
        }
        false
    }
}

//! Checked local exploration recovery, also retained by the Rust save system.
use crate::{
    collision::World,
    interaction::Interactions,
    movement::{Player, EYE_HEIGHT},
    water::Immersion,
};
use macroquad::prelude::*;
use std::collections::VecDeque;

pub(crate) mod retry_check;

/// A retry owns one coherent world/player snapshot, separate from the position-
/// only exploration shortcuts below. It is never nested inside a saved Game.
#[derive(Default)]
pub struct RetryPoint {
    checkpoint: Option<Box<crate::save::Game>>,
}

impl RetryPoint {
    /// New visits, successful saves and loads establish the next attempt.
    /// Never replace a live checkpoint with the failed attempt's death state.
    pub fn remember(&mut self, game: crate::save::Game) {
        if game.stats.alive() {
            self.checkpoint = Some(Box::new(game));
        }
    }

    pub fn game_for(&self, visit: &str) -> anyhow::Result<crate::save::Game> {
        use anyhow::{ensure, Context};
        let game = self
            .checkpoint
            .as_ref()
            .context("No retry checkpoint available")?;
        ensure!(
            game.current == visit,
            "Retry checkpoint belongs to another visit"
        );
        ensure!(game.stats.alive(), "Retry checkpoint is not alive");
        Ok((**game).clone())
    }
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Footing {
    feet: Vec3,
    yaw: f32,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Recovery {
    last_ground: Option<Footing>,
    departures: VecDeque<Footing>,
}

fn supported(world: &World, feet: Vec3) -> bool {
    if !world.body_clear(feet) || Immersion::sample(world, feet).level != 0 {
        return false;
    }
    // A recovery point must have floor around it, not balance on an edge.
    [
        Vec3::ZERO,
        Vec3::X * 20.,
        -Vec3::X * 20.,
        Vec3::Y * 20.,
        -Vec3::Y * 20.,
    ]
    .iter()
    .all(|offset| {
        let p = feet + *offset + Vec3::Z * 1.;
        let hit = world.sweep(p, p - Vec3::Z * 4., Vec3::ZERO);
        !hit.start_solid && hit.fraction < 1. && hit.normal.z >= 0.65
    })
}

impl Recovery {
    pub fn observe(
        &mut self,
        world: &World,
        interactions: &Interactions,
        player: &Player,
        yaw: f32,
    ) {
        if player.grounded && !player.climbing() && !player.swimming {
            if crate::android::is_android()
                && self
                    .last_ground
                    .is_some_and(|g| g.feet.distance_squared(player.feet) < 16. * 16.)
            {
                if let Some(g) = &mut self.last_ground {
                    g.yaw = yaw;
                }
            } else if supported(world, player.feet) && !interactions.hazardous(player.feet) {
                self.last_ground = Some(Footing {
                    feet: player.feet,
                    yaw,
                });
            }
        } else if let Some(footing) = self.last_ground.take() {
            if self
                .departures
                .back()
                .is_none_or(|p| p.feet.distance(footing.feet) > 48.)
            {
                self.departures.push_back(footing);
                if self.departures.len() > 32 {
                    self.departures.pop_front();
                }
            }
        }
    }
    /// Pop the last usable departure. Repeated recovery backs out of earlier drops.
    pub fn restore(
        &mut self,
        world: &World,
        interactions: &Interactions,
        current: Vec3,
        entry: (Vec3, f32),
    ) -> Option<(Player, f32, bool)> {
        // A lethal trigger can be crossed in the very frame that leaves support.
        // Keep the preceding footing even if no live airborne observation followed.
        if let Some(p) = self.last_ground.take() {
            self.departures.push_back(p);
        }
        while let Some(p) = self.departures.pop_back() {
            if p.feet.distance(current) > 48.
                && supported(world, p.feet)
                && !interactions.hazardous(p.feet)
            {
                return Some((
                    Player::spawn(world, p.feet + Vec3::Z * EYE_HEIGHT)?,
                    p.yaw,
                    true,
                ));
            }
        }
        Player::spawn(world, entry.0)
            .filter(|p| !interactions.hazardous(p.feet))
            .map(|p| (p, entry.1, false))
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

pub fn outside_world(world: &World, feet: Vec3) -> bool {
    feet.z < world.min.z - 256.
        || feet.x < world.min.x - 512.
        || feet.x > world.max.x + 512.
        || feet.y < world.min.y - 512.
        || feet.y > world.max.y + 512.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Controls;

    fn empty_interactions() -> Interactions {
        Interactions::empty()
    }

    #[test]
    fn recovery_returns_before_drop_even_after_jumping_in_pit() {
        let world = World::fixture(&[
            (vec3(-200., -100., -20.), vec3(0., 100., 0.)),
            (vec3(0., -100., -220.), vec3(200., 100., -200.)),
        ]);
        let interactions = empty_interactions();
        let mut recovery = Recovery::default();
        let mut p = Player::new(vec3(-80., 0., 0.));
        p.tick(&world, Controls::default());
        recovery.observe(&world, &interactions, &p, 1.);
        p.grounded = false;
        recovery.observe(&world, &interactions, &p, 1.);
        p.feet = vec3(80., 0., -200.);
        p.tick(&world, Controls::default());
        recovery.observe(&world, &interactions, &p, 2.);
        p.grounded = false;
        recovery.observe(&world, &interactions, &p, 2.);
        let (restored, yaw, nearby) = recovery
            .restore(&world, &interactions, p.feet, (Vec3::Z * 48., 0.))
            .unwrap();
        assert!(nearby && restored.feet.x == -80. && yaw == 1.);
        assert_eq!(restored.velocity, Vec3::ZERO);
        assert_eq!(restored.liquid_damage, 0.);
    }

    #[test]
    fn recovery_rechecks_support_and_falls_back_to_entry() {
        let floor = (vec3(-200., -100., -20.), vec3(200., 100., 0.));
        let world = World::fixture(&[floor]);
        let interactions = empty_interactions();
        let mut recovery = Recovery::default();
        let mut p = Player::new(vec3(80., 0., 0.));
        p.tick(&world, Controls::default());
        recovery.observe(&world, &interactions, &p, 1.);
        p.grounded = false;
        recovery.observe(&world, &interactions, &p, 1.);
        // A door/platform has moved into the old departure position.
        let blocked = World::fixture(&[floor, (vec3(60., -40., 0.), vec3(100., 40., 100.))]);
        let (restored, _, nearby) = recovery
            .restore(
                &blocked,
                &interactions,
                Vec3::new(0., 0., -200.),
                (vec3(-80., 0., 48.), 0.),
            )
            .unwrap();
        assert!(!nearby && restored.feet.x == -80.);
        assert!(!supported(&world, vec3(199., 0., 0.)));
        recovery.clear();
        assert!(recovery.departures.is_empty());
    }
}

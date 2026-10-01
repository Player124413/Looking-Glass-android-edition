//! Save and continue for headless routes (F3b, `docs/CAMPAIGN.md`).
//!
//! A `Checkpoint` is what the chain harness writes at a visit boundary or in the middle of a leg:
//! Alice's resources and body, the ledger of finished visits, and the visit being played as the
//! save format stores it (the controllers', story and hints snapshots), plus the few pieces of
//! route state a save would keep in Alice's character block (the weapon action, the projectiles in
//! flight) or in the cast (the placed guards). `Route::resume` rebuilds a route from one through
//! the same loader the viewer's Continue uses, so a rebuilt route and the route it was taken from
//! must stay equal under identical input. `--campaign-route-check` proves that at every boundary
//! and once in every leg.
//!
//! What a checkpoint leaves out is route bookkeeping that is not game state: the fight tracker's
//! velocity smoothing, the transient `aim_at`/`use_pressed` requests and the metrics. The probe
//! clears those on both routes before it compares them.
use super::Route;
use crate::{
    assets::Assets,
    campaign,
    combat::Guard,
    inventory::Stats,
    movement::{Controls, Player},
    powerups::Difficulty,
    save::{Campaign, Level},
};
use anyhow::{bail, ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FORMAT: &str = "campaign-chain-checkpoint";
pub const VERSION: u32 = 1;

/// The route state a save keeps outside the visit's controllers.
#[derive(Clone, Serialize, Deserialize)]
pub struct RouteState {
    pub ticks: usize,
    pub jumps: u64,
    pub shots: usize,
    pub swings: usize,
    pub cards: usize,
    pub damage: f32,
    pub teleports: usize,
    /// The placed club guards (the headless stand-in for the saved cast).
    pub guards: Vec<Guard>,
    #[serde(default)]
    pub native_cast: bool,
    /// Alice's weapon action, as the character block of a save stores it.
    pub actions: crate::weapons::Snapshot,
    /// Blades and cards in flight.
    pub projectiles: Vec<crate::weapons::Projectile>,
    #[serde(default)]
    pub heavy: crate::weapons::route_heavy::Heavy,
}

/// One saved moment of a route.
#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub format: String,
    pub version: u32,
    pub difficulty: Difficulty,
    pub stats: Stats,
    /// The visits finished before this one (the current visit is not in it).
    pub ledger: Campaign,
    pub player: Player,
    /// The visit being played: controllers, story, hints and the visual clocks.
    pub level: Level,
    pub route: RouteState,
}

impl Route {
    /// Start a visit at its entrance carrying resources and the ledger of finished visits, the
    /// way `campaign::arrive` hands one over.
    pub fn enter_carrying(
        assets: &mut Assets,
        name: &str,
        entry: Option<&str>,
        carried: Stats,
        ledger: Campaign,
        difficulty: Difficulty,
    ) -> Result<Self> {
        let loaded = campaign::load_visit(assets, name, entry, None, difficulty, true)?;
        let player = Player::spawn(
            &loaded.world,
            crate::interaction::spawn(&loaded.map, entry).0,
        )
        .with_context(|| format!("{name} spawn blocked"))?;
        Self::assemble(assets, name, entry, loaded, player, carried, ledger)
    }

    /// This moment of the route, as a checkpoint.
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            format: FORMAT.into(),
            version: VERSION,
            difficulty: self.difficulty,
            stats: self.stats.clone(),
            ledger: self.ledger.clone(),
            player: self.player.clone(),
            level: self.level(),
            route: RouteState {
                ticks: self.ticks,
                jumps: self.jumps,
                shots: self.shots,
                swings: self.swings,
                cards: self.cards,
                damage: self.damage,
                teleports: self.teleports,
                guards: self.guards.clone(),
                native_cast: self.native_cast.is_some(),
                actions: self.actions.snapshot(),
                projectiles: self.projectiles.clone(),
                heavy: self.heavy.clone(),
            },
        }
    }

    /// Rebuild a route from a checkpoint: the visit's logic through the cached-visit restore the
    /// viewer's Continue uses, Alice exactly where she was, and the route state a save keeps.
    pub fn resume(assets: &mut Assets, cp: &Checkpoint) -> Result<Self> {
        ensure!(
            cp.format == FORMAT && cp.version == VERSION,
            "Not a campaign chain checkpoint of format {VERSION}"
        );
        let level = &cp.level;
        let loaded = campaign::load_visit(
            assets,
            &level.map,
            level.entry.as_deref(),
            Some(level),
            cp.difficulty,
            false,
        )?;
        cp.player.validate_save()?;
        cp.player.validate_world(&loaded.world)?;
        let mut route = Self::assemble(
            assets,
            &level.map,
            level.entry.as_deref(),
            loaded,
            cp.player.clone(),
            cp.stats.clone(),
            cp.ledger.clone(),
        )?;
        let saved = &cp.route;
        if saved.native_cast {
            // Assembly creates stand-ins for window-free routes. A native
            // checkpoint owns these same guards in its NPC snapshot instead.
            route.guards.clear();
            route.enable_native_cast(assets)?;
            route.native_cast.as_mut().unwrap().restore(&level.npcs)?;
        }
        ensure!(
            route.guards.len() == saved.guards.len(),
            "The checkpoint holds {} guards, the visit places {}",
            saved.guards.len(),
            route.guards.len()
        );
        route.guards = saved.guards.clone();
        route.actions.restore(&saved.actions)?;
        route.projectiles = saved.projectiles.clone();
        saved.heavy.validate()?;
        route.heavy = saved.heavy.clone();
        route.environment_clock = level.environment_clock;
        route.pickup_clock = level.pickup_clock;
        route.ticks = saved.ticks;
        route.jumps = saved.jumps;
        route.shots = saved.shots;
        route.swings = saved.swings;
        route.cards = saved.cards;
        route.damage = saved.damage;
        route.teleports = saved.teleports;
        Ok(route)
    }

    /// Everything the probe compares: the resources, Alice, the visit's controllers, story and
    /// hints, and the route state a checkpoint keeps.
    pub fn state(&self) -> Result<serde_json::Value> {
        let cp = self.checkpoint();
        Ok(serde_json::json!({
            "stats": serde_json::to_value(&cp.stats)?,
            "player": serde_json::to_value(&cp.player)?,
            "level": serde_json::to_value(&cp.level)?,
            "route": serde_json::to_value(&cp.route)?,
        }))
    }

    /// SHA-256 of `state`, for reports.
    pub fn state_hash(&self) -> Result<String> {
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&self.state()?)?)
        ))
    }

    /// Stop at the first tick at or after `ticks`: every later tick does nothing and the walking
    /// and waiting helpers return, as at a latched exit. The probe uses it to take a checkpoint in
    /// the middle of a leg by driving the leg again.
    pub fn freeze_at(&mut self, ticks: usize) {
        self.freeze_at = Some(ticks);
    }
    pub fn frozen(&self) -> bool {
        self.frozen
    }
    /// Let a frozen route run again.
    pub fn thaw(&mut self) {
        self.freeze_at = None;
        self.frozen = false;
    }
    pub(super) fn freeze_check(&mut self) {
        if self.freeze_at.is_some_and(|n| self.ticks >= n) {
            self.frozen = true;
        }
    }
    /// Clear the route bookkeeping that is not game state, so that two routes can be compared:
    /// the fight tracker, the transient click and aim requests, and the tactics that use them.
    pub fn quiet(&mut self) {
        self.tactics = false;
        self.fight = Default::default();
        self.aim_at = None;
        self.heavy_weapon = None;
        self.use_pressed = false;
    }
}

/// The input both routes of a probe receive, tick for tick: short walks in every direction with
/// pauses between them, an occasional jump and a use. It never depends on either route's state.
pub fn probe_controls(t: usize) -> Controls {
    let chunk = (t / 45) % 8;
    let wish = match chunk {
        0 => vec2(1., 0.),
        2 => vec2(0., 1.),
        3 => vec2(-1., 0.),
        5 => vec2(0., -1.),
        6 => vec2(0.7, 0.7),
        _ => Vec2::ZERO,
    };
    Controls {
        wish,
        jump: chunk == 6 && t % 45 == 0,
        run: chunk % 2 == 0,
        use_pressed: chunk == 3 && t % 45 == 0,
        ..Default::default()
    }
}

/// Run two routes for `ticks` ticks of identical input and require them to stay equal: the same
/// result at every tick (or the same error at the same tick) and equal states every 60 ticks and at
/// the end. Returns the tick at which both ended, if they did.
pub fn run_identical(
    a: &mut Route,
    b: &mut Route,
    ticks: usize,
) -> Result<Option<(usize, String)>> {
    for t in 0..ticks {
        let input = probe_controls(t);
        let (ra, rb) = (a.tick(input), b.tick(input));
        match (ra, rb) {
            (Ok(()), Ok(())) => {}
            (Err(x), Err(y)) if format!("{x:#}") == format!("{y:#}") => {
                ensure!(
                    a.state()? == b.state()?,
                    "The routes ended together at tick {t} in different states"
                );
                return Ok(Some((t, format!("{x:#}"))));
            }
            (ra, rb) => bail!(
                "The routes diverged at tick {t}: {} against {}",
                ra.map_or_else(|e| format!("{e:#}"), |_| "ok".into()),
                rb.map_or_else(|e| format!("{e:#}"), |_| "ok".into())
            ),
        }
        if t % 60 == 59 {
            ensure!(a.state()? == b.state()?, "The routes differ at tick {t}");
        }
    }
    ensure!(
        a.state()? == b.state()?,
        "The routes differ after {ticks} ticks"
    );
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_probe_input_walks_jumps_and_uses_and_repeats_every_360_ticks() {
        let inputs: Vec<Controls> = (0..360).map(probe_controls).collect();
        assert!(inputs.iter().any(|c| c.wish.x > 0.) && inputs.iter().any(|c| c.wish.x < 0.));
        assert!(inputs.iter().any(|c| c.wish.y > 0.) && inputs.iter().any(|c| c.wish.y < 0.));
        assert!(inputs.iter().any(|c| c.wish == Vec2::ZERO));
        assert_eq!(inputs.iter().filter(|c| c.jump).count(), 1);
        assert_eq!(inputs.iter().filter(|c| c.use_pressed).count(), 1);
        for t in 0..360 {
            assert_eq!(
                probe_controls(t).wish,
                probe_controls(t + 360).wish,
                "tick {t}"
            );
        }
    }
}

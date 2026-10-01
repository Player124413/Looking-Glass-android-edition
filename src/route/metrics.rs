//! What a route leg reports, and the assertions every leg shares (`docs/CAMPAIGN.md`).
use super::Route;
use crate::water::Liquid;
use anyhow::{ensure, Result};
use std::{collections::BTreeSet, fmt};

/// The numbers a leg prints and a chain accumulates.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Metrics {
    pub ticks: usize,
    pub jumps: u64,
    /// Blade throws.
    pub throws: usize,
    /// Blade melee swings (tactics only).
    pub swings: usize,
    /// Cards released (tactics only).
    pub cards: usize,
    /// Damage enemies dealt to Alice.
    pub damage: f32,
    /// Authored teleports taken.
    pub teleports: usize,
    pub sanity_in: f32,
    pub sanity_out: f32,
    pub will_in: f32,
    pub will_out: f32,
    /// Distinct pickups collected during the leg.
    pub pickups: usize,
    /// Distinct enemy drops (essence) collected during the leg.
    pub loot: usize,
    /// Ticks spent in lava or slime (zero unless a leg documents an allowance).
    pub hazard_ticks: usize,
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ticks, {} jumps, {} throws, {} swings, {} cards, {} combat damage, sanity {:.0} -> {:.0}, will {:.0} -> {:.0}, {} pickups, {} loot, {} authored teleports",
            self.ticks,
            self.jumps,
            self.throws,
            self.swings,
            self.cards,
            self.damage,
            self.sanity_in,
            self.sanity_out,
            self.will_in,
            self.will_out,
            self.pickups,
            self.loot,
            self.teleports
        )?;
        if self.hazard_ticks > 0 {
            write!(f, ", {} ticks in slime", self.hazard_ticks)?;
        }
        Ok(())
    }
}

/// What a leg started with, so its metrics can report resources in and out.
#[derive(Clone, Debug, Default)]
pub struct Start {
    pub sanity: f32,
    pub will: f32,
    pub collected: BTreeSet<String>,
}

/// What the route did that a leg must not do, counted every tick.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Audit {
    /// Ticks Alice spent in lava.
    pub lava_ticks: usize,
    /// Ticks Alice spent in slime.
    pub slime_ticks: usize,
    /// Ticks Alice spent beyond the level, where the viewer counts her as lost and asks for a
    /// retry: any such tick would be a recovery, not progress.
    pub lost_ticks: usize,
    /// Exits taken. A route that latches at its exit takes exactly one.
    pub transitions: usize,
    /// The lowest Sanity Alice reached.
    pub lowest_sanity: f32,
}

impl Route {
    pub fn metrics(&self) -> Metrics {
        let new = self.stats.collected.difference(&self.start.collected);
        let (mut pickups, mut loot) = (0, 0);
        for key in new {
            if key.starts_with("drop:") {
                loot += 1;
            } else {
                pickups += 1;
            }
        }
        Metrics {
            ticks: self.ticks,
            jumps: self.jumps,
            throws: self.shots,
            swings: self.swings,
            cards: self.cards,
            damage: self.damage,
            teleports: self.teleports,
            sanity_in: self.start.sanity,
            sanity_out: self.stats.sanity(),
            will_in: self.start.will,
            will_out: self.stats.will(),
            pickups,
            loot,
            hazard_ticks: self.audit.lava_ticks + self.audit.slime_ticks,
        }
    }
    /// Record one tick's worth of audit counters.
    pub(super) fn audit_tick(&mut self) {
        let immersion = self.player.immersion;
        if immersion.level > 0 {
            let counter = match immersion.kind {
                Liquid::Lava => Some(&mut self.audit.lava_ticks),
                Liquid::Slime => Some(&mut self.audit.slime_ticks),
                _ => None,
            };
            if let Some(counter) = counter {
                *counter += 1;
                if *counter == 1 {
                    super::trace(|| {
                        format!(
                            "hazard liquid begins t{} {:?} level {} at {:?}",
                            self.ticks, immersion.kind, immersion.level, self.player.feet
                        )
                    });
                }
            }
        }
        if crate::recovery::outside_world(&self.world, self.player.feet) {
            self.audit.lost_ticks += 1;
        }
        self.audit.lowest_sanity = self.audit.lowest_sanity.min(self.stats.sanity());
    }
    /// The assertions every leg shares (R5): Alice never stood in lava, and stood in slime for no
    /// more ticks than the leg documents (`slime_allowance`, zero for a route that keeps out of
    /// it); she never left the level (which the viewer would answer with a recovery); she took
    /// exactly the authored teleports and one exit; and she was alive throughout (a tick that
    /// ends with Alice dead already fails the route).
    pub fn assert_clean(&self, teleports: usize, slime_allowance: usize) -> Result<()> {
        ensure!(
            self.audit.lava_ticks == 0,
            "Alice spent {} ticks in lava",
            self.audit.lava_ticks
        );
        ensure!(
            self.audit.slime_ticks <= slime_allowance,
            "Alice spent {} ticks in slime, {slime_allowance} allowed",
            self.audit.slime_ticks
        );
        ensure!(
            self.audit.lost_ticks == 0,
            "Alice spent {} ticks beyond the level (a recovery, not progress)",
            self.audit.lost_ticks
        );
        ensure!(
            self.teleports == teleports,
            "Expected {teleports} authored teleports, took {}",
            self.teleports
        );
        ensure!(
            self.audit.transitions == 1 && self.transition.is_some(),
            "Expected a single exit, took {} ({:?})",
            self.audit.transitions,
            self.transition
        );
        ensure!(
            self.stats.alive() && self.audit.lowest_sanity > 0.,
            "Alice was not alive throughout"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_print_every_figure_a_leg_reports() {
        let mut m = Metrics {
            ticks: 10,
            jumps: 2,
            throws: 3,
            swings: 4,
            cards: 5,
            damage: 6.,
            teleports: 1,
            sanity_in: 100.,
            sanity_out: 73.,
            will_in: 100.,
            will_out: 5.,
            pickups: 7,
            loot: 8,
            hazard_ticks: 0,
        };
        let text = m.to_string();
        for part in [
            "10 ticks",
            "2 jumps",
            "3 throws",
            "6 combat damage",
            "sanity 100 -> 73",
            "will 100 -> 5",
            "7 pickups",
            "8 loot",
            "1 authored teleports",
        ] {
            assert!(text.contains(part), "{text} lacks {part}");
        }
        assert!(!text.contains("slime"));
        m.hazard_ticks = 12;
        assert!(m.to_string().ends_with("12 ticks in slime"));
    }
}

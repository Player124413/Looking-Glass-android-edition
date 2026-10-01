//! The reward-provenance report of the campaign chain (T9, `docs/CAMPAIGN.md`).
//!
//! A strict chain skips the campaign baseline loadout, so every toy Alice holds must come from its
//! authored source: the pickup the level places (found in the map's entities at run time, never
//! named here), a scripted grant, or an arrival grant the level makes on entry. After each leg the
//! chain settles the rewards that leg's visit grants:
//!
//! * a required reward that Alice does not hold is a missing grant and fails the leg;
//! * one she holds without having collected an authored pickup was filled in by the baseline,
//!   which fails a strict chain and is reported by a non-strict one;
//! * optional rewards (the Blunderbuss behind the hedge-maze secret door) are reported, never
//!   required.
//!
//! The report lists all twelve milestones of `campaign::rewards()` plus the temple's arrival
//! grant, whether or not the chain reached them.
use crate::{
    campaign,
    inventory::{Catalog, PickupKind, Stats, WEAPONS},
    route::Route,
};
use serde::Serialize;

/// Where a reward comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Source {
    /// A pickup the visit's map places.
    Pickup,
    /// A grant the visit's script makes (the Eye Staff after the survival); no adapter exists yet.
    ScriptGrant,
}

#[derive(Clone, Debug, Serialize)]
pub struct Reward {
    pub visit: usize,
    pub map: &'static str,
    pub entry: Option<&'static str>,
    pub slot: usize,
    pub copies: u8,
    pub name: &'static str,
    pub source: Source,
    pub required: bool,
}

/// What the chain found out about a reward.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum Status {
    NotReached,
    /// The visit lies before the start of this run (a checkpoint): what it granted is carried, not proven here.
    Unproven,
    /// Held, from these authored pickups collected in the visit.
    Pickup(Vec<String>),
    /// Held without an authored pickup: the baseline loadout filled it in.
    BaselineFill,
    /// Held, granted by the visit's script.
    ScriptGrant,
    /// A required reward Alice does not hold.
    Missing,
    /// An optional reward she did not collect.
    Skipped,
    /// A scripted grant with no adapter yet.
    Pending,
}

impl Status {
    /// What the report says. A viewer-style chain fills the baseline in on arrival, so a grant
    /// the visit did not make is reported as such, not as a failure.
    pub fn text(&self, strict: bool) -> String {
        match self {
            Self::NotReached => "not reached".into(),
            Self::Unproven => "not proven in this run (it started from a checkpoint)".into(),
            Self::Pickup(ids) => format!("authored pickup {}", ids.join(", ")),
            Self::BaselineFill => "BASELINE FILL (no authored pickup was collected)".into(),
            Self::ScriptGrant => "script grant".into(),
            Self::Missing if strict => {
                "MISSING required grant (no authored pickup was collected)".into()
            }
            Self::Missing => {
                "not granted by the visit (the baseline fill supplies it on exit)".into()
            }
            Self::Skipped => "skipped (optional, not required)".into(),
            Self::Pending => "MISSING required grant (scripted, no adapter yet)".into(),
        }
    }
}

/// The twelve milestones, in campaign order.
pub fn table() -> Vec<Reward> {
    let mut rewards: Vec<Reward> = campaign::rewards()
        .iter()
        .map(|&(map, entry, slot, copies)| Reward {
            visit: campaign::visit_index(map, entry).expect("A reward visit is in the route"),
            map,
            entry,
            slot,
            copies,
            name: WEAPONS[slot].1,
            // Slot 7 is the Eye Staff, granted by the jlair2 script.
            source: if slot == 7 {
                Source::ScriptGrant
            } else {
                Source::Pickup
            },
            // Slot 8 is the Blunderbuss behind the wforest secret door.
            required: slot != 8,
        })
        .collect();
    rewards.sort_by_key(|r| r.visit);
    rewards
}

/// Settle one reward against the route that just finished its visit.
pub fn settle(reward: &Reward, route: &Route, catalog: &Catalog) -> Status {
    let held = route.stats.copies(reward.slot) >= reward.copies;
    if reward.source == Source::ScriptGrant {
        return if held {
            Status::ScriptGrant
        } else {
            Status::Pending
        };
    }
    if !held {
        return if reward.required {
            Status::Missing
        } else {
            Status::Skipped
        };
    }
    let collected = collected_pickups(reward, route, catalog);
    if collected.is_empty() {
        Status::BaselineFill
    } else {
        Status::Pickup(collected)
    }
}

/// The ids of the authored pickups of this toy that the visit places and Alice has collected.
fn collected_pickups(reward: &Reward, route: &Route, catalog: &Catalog) -> Vec<String> {
    crate::inventory::pickups_for_visit(&route.map, reward.map, reward.entry, catalog)
        .into_iter()
        .filter(|p| matches!(p.kind, PickupKind::Weapon(slot) if slot == reward.slot))
        .filter(|p| route.stats.collected.contains(&p.id))
        .map(|p| p.id)
        .collect()
}

/// Whether a settled reward stops the chain: a required grant that is missing, or, in a strict
/// chain, one that only the baseline supplied.
pub fn fails(reward: &Reward, status: &Status, strict: bool) -> Option<String> {
    match status {
        // A viewer-style exit fills the baseline in on arrival, so a grant the leg did not make
        // is only reported; a strict chain has no fill to lean on.
        Status::Missing | Status::Pending if reward.required && strict => Some(format!(
            "missing required grant: {} ({} visit {})",
            reward.name,
            reward.map,
            campaign::visits()[reward.visit]
        )),
        Status::BaselineFill if strict => Some(format!(
            "{} was filled in by the baseline loadout, not granted by {}",
            reward.name,
            campaign::visits()[reward.visit]
        )),
        _ => None,
    }
}

/// The temple's shell is an authored arrival grant: it applies in a strict chain too.
pub fn arrival_grant(stats: &Stats) -> bool {
    stats.turtle_air
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_holds_the_twelve_milestones_in_campaign_order() {
        let table = table();
        assert_eq!(table.len(), 12);
        assert!(table.windows(2).all(|w| w[0].visit <= w[1].visit));
        assert_eq!(table[0].name, WEAPONS[0].1);
        // Only the Blunderbuss is optional, and only the Eye Staff is scripted.
        assert_eq!(table.iter().filter(|r| !r.required).count(), 1);
        assert!(table.iter().any(|r| r.slot == 8 && !r.required));
        assert!(table
            .iter()
            .any(|r| r.slot == 7 && r.source == Source::ScriptGrant));
        // The three Demon Dice pickups are three milestones of one toy.
        let dice: Vec<u8> = table
            .iter()
            .filter(|r| r.slot == 6)
            .map(|r| r.copies)
            .collect();
        assert_eq!(dice, [1, 2, 3]);
    }
    #[test]
    fn a_missing_required_grant_fails_and_an_optional_one_does_not() {
        let table = table();
        let blade = &table[0];
        let blunderbuss = table.iter().find(|r| r.slot == 8).unwrap();
        assert!(fails(blade, &Status::Missing, true).is_some());
        assert!(fails(blade, &Status::Pending, true).is_some());
        // A viewer-style exit fills the baseline in on arrival, so it only reports the gap.
        assert!(fails(blade, &Status::Missing, false).is_none());
        assert!(fails(blade, &Status::Pickup(vec!["gvillage:15".into()]), true).is_none());
        assert!(fails(blunderbuss, &Status::Skipped, true).is_none());
        // The baseline fill fails a strict chain and is only reported by a non-strict one.
        assert!(fails(blade, &Status::BaselineFill, true).is_some());
        assert!(fails(blade, &Status::BaselineFill, false).is_none());
    }
    #[test]
    fn every_status_prints_a_reason() {
        for status in [
            Status::NotReached,
            Status::Unproven,
            Status::Pickup(vec!["a:1".into()]),
            Status::BaselineFill,
            Status::ScriptGrant,
            Status::Missing,
            Status::Skipped,
            Status::Pending,
        ] {
            assert!(!status.text(true).is_empty() && !status.text(false).is_empty());
        }
    }
    #[test]
    fn the_temple_shell_is_an_arrival_grant() {
        let mut stats = Stats::for_level("gvillage", None);
        assert!(!arrival_grant(&stats));
        stats.arrival_grants("utemple");
        assert!(arrival_grant(&stats));
    }
}

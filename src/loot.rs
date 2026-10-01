//! Death rewards belong to a visit and an enemy identity, never to an animation frame.
use crate::{
    collision::World,
    inventory::{Pickup, PickupKind, Stats},
};
use anyhow::{ensure, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grade {
    Small,
    Medium,
    Large,
    Super,
}
impl Grade {
    pub fn amount(self) -> f32 {
        match self {
            Self::Small => 15.,
            Self::Medium => 25.,
            Self::Large => 50.,
            Self::Super => 100.,
        }
    }
    pub fn model(self) -> &'static str {
        match self {
            Self::Small => "w_me_small",
            Self::Medium => "w_me_medium",
            Self::Large => "w_me_large",
            Self::Super => "w_me_super",
        }
    }
    fn duration(self) -> f32 {
        match self {
            Self::Small => 40.,
            Self::Medium => 20.,
            _ => 10.,
        }
    }
    fn smaller(self) -> Option<Self> {
        match self {
            Self::Super => Some(Self::Large),
            Self::Large => Some(Self::Medium),
            Self::Medium => Some(Self::Small),
            Self::Small => None,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Source {
    pub id: usize,
    pub feet: Vec3,
    pub grade: Grade,
    pub dead: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Drop {
    pub id: usize,
    pub origin: Vec3,
    pub grade: Grade,
    pub remaining: f32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Loot {
    pub awarded: BTreeSet<usize>,
    pub drops: Vec<Drop>,
}
impl Loot {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.awarded.len() <= 4096 && self.drops.len() <= 4096,
            "Too many saved drops"
        );
        let mut ids = BTreeSet::new();
        for d in &self.drops {
            ensure!(
                ids.insert(d.id)
                    && self.awarded.contains(&d.id)
                    && d.origin.is_finite()
                    && d.origin.abs().max_element() <= 100000.
                    && d.remaining > 0.
                    && d.remaining <= d.grade.duration(),
                "Invalid saved enemy drop"
            );
        }
        Ok(())
    }
    pub fn update(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        self.drops.retain_mut(|d| {
            d.remaining -= dt;
            while d.remaining <= 0. {
                let Some(next) = d.grade.smaller() else {
                    return false;
                };
                d.grade = next;
                d.remaining += next.duration();
            }
            true
        });
    }
    pub fn defeated(&mut self, before: &[Source], after: &[Source], world: &World) {
        for s in after.iter().filter(|s| s.dead) {
            // Old corpses, scripted hiding and despawning do not create fresh rewards.
            if !before.iter().any(|b| b.id == s.id && !b.dead) || !self.awarded.insert(s.id) {
                continue;
            }
            let start = s.feet + Vec3::Z * 12.;
            let floor = world.sweep(start, start - Vec3::Z * 4096., Vec3::splat(3.));
            let origin = if floor.start_solid || floor.fraction == 1. {
                start
            } else {
                start.lerp(start - Vec3::Z * 4096., floor.fraction)
            };
            self.drops.push(Drop {
                id: s.id,
                origin,
                grade: s.grade,
                remaining: s.grade.duration(),
            });
        }
    }
    pub fn pickups(&self, visit: &str) -> Vec<Pickup> {
        self.drops
            .iter()
            .map(|d| Pickup {
                id: format!("drop:{visit}:{}", d.id),
                model: d.grade.model().into(),
                origin: d.origin,
                kind: PickupKind::Essence,
                amount: d.grade.amount(),
            })
            .collect()
    }
}
pub fn tick(
    stats: &mut Stats,
    visit: &str,
    dt: f32,
    before: &[Source],
    after: &[Source],
    feet: Vec3,
    world: &World,
) -> Vec<String> {
    tick_with(stats, visit, dt, before, after, feet, world, |_| {})
}
pub fn tick_with(
    stats: &mut Stats,
    visit: &str,
    dt: f32,
    before: &[Source],
    after: &[Source],
    feet: Vec3,
    world: &World,
    collected: impl FnMut(&Pickup),
) -> Vec<String> {
    let loot = stats.loot.entry(visit.into()).or_default();
    loot.update(dt);
    loot.defeated(before, after, world);
    let items = loot.pickups(visit);
    let messages = crate::inventory::collect_with(stats, &items, feet, world, collected);
    stats
        .loot
        .get_mut(visit)
        .unwrap()
        .drops
        .retain(|d| !stats.collected.contains(&format!("drop:{visit}:{}", d.id)));
    messages
}

/// Real brush geometry around the reported first-school library doorway.
pub fn check_school_contacts(assets: &mut crate::assets::Assets) -> Result<()> {
    let map = crate::bsp::Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    crate::interaction::Interactions::load(&map)?.sync(&mut world);
    for origin in [
        vec3(-754., 2943., -252.96875),
        vec3(-754., 3062., -252.96875),
    ] {
        for offset in [Vec3::ZERO, vec3(-20., 20., 0.)] {
            let feet = origin + offset - Vec3::Z * 3.;
            ensure!(
                world.body_clear(feet),
                "School contact fixture is obstructed"
            );
            for (damage, spent) in [(0., 0.), (20., 0.), (0., 20.), (20., 20.)] {
                let mut stats = Stats::default();
                stats.damage(damage);
                stats.spend_will(spent);
                stats.loot.insert(
                    "skool1$first".into(),
                    Loot {
                        awarded: BTreeSet::from([9]),
                        drops: vec![Drop {
                            id: 9,
                            origin,
                            grade: Grade::Small,
                            remaining: 15.,
                        }],
                    },
                );
                let mut stats: Stats = serde_json::from_slice(&serde_json::to_vec(&stats)?)?;
                let full = damage == 0. && spent == 0.;
                let mut feedback = crate::inventory::PickupFeedback::default();
                ensure!(
                    feedback.update(&stats, &[], "skool1$first", feet, &world)
                        == full.then_some("Sanity and Will full"),
                    "Incorrect full-meter notice at library door"
                );
                ensure!(
                    tick(&mut stats, "skool1$first", 0., &[], &[], feet, &world).len()
                        == usize::from(!full),
                    "School drop did not collect on contact"
                );
                ensure!(
                    stats.loot["skool1$first"].drops.len() == usize::from(full),
                    "Incorrect school drop persistence"
                );
                ensure!(
                    stats.sanity() == (115. - damage).min(100.)
                        && stats.will() == (115. - spent).min(100.),
                    "School essence restored incorrect resources"
                );
                ensure!(
                    tick(&mut stats, "skool1$first", 0., &[], &[], feet, &world).is_empty(),
                    "School drop collected twice"
                );
            }
        }
    }
    println!("PASS school library essence: both doorway drops, direct/diagonal contact, each depleted meter, full-meter notice and saved-drop exactly-once collection");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dropped_essence_is_reachable_from_a_step_after_reload() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
            (vec3(-1000., -1000., 0.), vec3(0., 1000., 40.)),
        ]);
        let before = [Source {
            id: 9,
            feet: vec3(8., 0., 0.1),
            grade: Grade::Large,
            dead: false,
        }];
        let after = [Source {
            dead: true,
            ..before[0]
        }];
        let feet = vec3(-20., 0., 40.1);
        assert!(world.body_clear(feet));
        let mut stats = Stats::default();
        // A full player leaves it available; an old live drop needs no migration.
        assert!(tick(&mut stats, "step", 0.1, &before, &after, feet, &world).is_empty());
        let origin = stats.loot["step"].drops[0].origin;
        let old_ray = world.sweep(feet + Vec3::Z * 24., origin + Vec3::Z * 24., Vec3::ZERO);
        assert!(
            old_ray.fraction < 0.99,
            "Fixture must reproduce the fixed-height obstruction"
        );
        let mut restored: Stats =
            serde_json::from_str(&serde_json::to_string(&stats).unwrap()).unwrap();
        restored.damage(60.);
        restored.spend_will(80.);
        assert_eq!(
            tick(&mut restored, "step", 0., &after, &after, feet, &world).len(),
            1
        );
        assert_eq!((restored.sanity(), restored.will()), (90., 70.));
        assert!(restored.loot["step"].drops.is_empty());
        assert!(tick(&mut restored, "step", 0.1, &before, &after, feet, &world).is_empty());
    }
    #[test]
    fn item_box_corners_collect_but_outside_or_separated_boxes_do_not() {
        let world = World::fixture(&[]);
        for (feet, expected) in [
            (vec3(30., 30., 0.), true),
            (vec3(-30., -30., 0.), true),
            (vec3(31.1, 0., 0.), false),
            (vec3(0., 0., 64.1), false),
            (vec3(0., 0., -56.1), false),
        ] {
            let mut stats = Stats::default();
            stats.spend_will(50.);
            stats.loot.insert(
                "corner".into(),
                Loot {
                    awarded: BTreeSet::from([4]),
                    drops: vec![Drop {
                        id: 4,
                        origin: Vec3::ZERO,
                        grade: Grade::Small,
                        remaining: 40.,
                    }],
                },
            );
            assert_eq!(
                !tick(&mut stats, "corner", 0., &[], &[], feet, &world).is_empty(),
                expected,
                "{feet:?}"
            );
            assert_eq!(stats.will(), if expected { 65. } else { 50. });
        }
    }
    #[test]
    fn enemy_death_collection_occlusion_and_no_duplicate_after_reload() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -100.), vec3(1000., 1000., 0.)),
            (vec3(8., -100., 0.), vec3(12., 100., 100.)),
        ]);
        let before = [Source {
            id: 3,
            feet: vec3(20., 0., 1.),
            grade: Grade::Large,
            dead: false,
        }];
        let after = [Source {
            dead: true,
            ..before[0]
        }];
        let mut stats = Stats::default();
        stats.damage(60.);
        stats.spend_will(80.);
        assert!(tick(
            &mut stats,
            "school$first",
            0.1,
            &before,
            &after,
            vec3(0., 0., 0.1),
            &world
        )
        .is_empty());
        assert_eq!(stats.loot["school$first"].drops.len(), 1);
        let mut restored: Stats =
            serde_json::from_str(&serde_json::to_string(&stats).unwrap()).unwrap();
        assert_eq!(
            tick(
                &mut restored,
                "school$first",
                0.1,
                &after,
                &after,
                vec3(20., 0., 0.1),
                &world
            )
            .len(),
            1
        );
        assert_eq!((restored.sanity(), restored.will()), (90., 70.));
        assert!(tick(
            &mut restored,
            "school$first",
            0.1,
            &before,
            &after,
            vec3(20., 0., 0.1),
            &world
        )
        .is_empty());
        assert!(restored.loot["school$first"].drops.is_empty());
        restored.validate_save().unwrap();
        let mut old = Loot::default();
        old.defeated(&after, &after, &world);
        assert!(old.drops.is_empty()); // Migrated corpses are not fresh kills.
    }
    #[test]
    fn decay_and_reload_preserve_tier_and_expiry() {
        let mut a = Loot {
            awarded: BTreeSet::from([7]),
            drops: vec![Drop {
                id: 7,
                origin: Vec3::ZERO,
                grade: Grade::Large,
                remaining: 10.,
            }],
        };
        a.update(10.);
        assert_eq!(a.drops[0].grade, Grade::Medium);
        let mut b: Loot = serde_json::from_str(&serde_json::to_string(&a).unwrap()).unwrap();
        b.update(20.);
        assert_eq!(b.drops[0].grade, Grade::Small);
        b.update(40.);
        assert!(b.drops.is_empty());
        assert!(b.awarded.contains(&7));
    }
}

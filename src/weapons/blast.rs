//! One-shot radius damage from the original Explosion/RadiusDamage rules.
use crate::combat::{Context, DamageKind, Hit, Target};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Kind {
    ElectricHit,
    Croquet,
    Jack,
    Staff,
    Cannon,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Blast {
    pub kind: Kind,
    pub origin: Vec3,
    pub age: f32,
    pub seed: u32,
}
impl Blast {
    pub fn new(kind: Kind, origin: Vec3, seed: u32) -> Self {
        Self {
            kind,
            origin,
            age: 0.,
            seed,
        }
    }
    pub fn valid(&self) -> bool {
        self.origin.is_finite() && (0. ..=5.).contains(&self.age)
    }
    pub fn damage(&self, ctx: &Context<'_>, owner: Target, direct: Option<usize>) -> Vec<Hit> {
        if self.kind == Kind::ElectricHit {
            return Vec::new();
        }
        // The ball's `radius 100` is not copied into the separately constructed
        // Explosion. Its omitted explosion radius defaults to damage + 60.
        let (damage, radius, knockback, kind) = match self.kind {
            Kind::ElectricHit => unreachable!(),
            Kind::Croquet => (30., 90., 30., DamageKind::Other),
            Kind::Jack => (300., 300., 300., DamageKind::Fire),
            Kind::Staff => (100., 400., 400., DamageKind::EyeStaff),
            Kind::Cannon => (998., 1058., 800., DamageKind::Blunderbuss),
        };
        let mut seen = std::collections::BTreeSet::new();
        ctx.targets
            .iter()
            .copied()
            .chain([owner])
            .filter_map(|target| {
                if Some(target.id) == direct || !seen.insert(target.id) {
                    return None;
                }
                let offset = target.center - self.origin;
                let distance = offset.length();
                let falloff = (1. - distance / radius).max(0.);
                if falloff <= 0. {
                    return None;
                }
                let self_hit = target.id == owner.id;
                // Native RadiusDamage deliberately skips the world trace inside
                // half the radius (quarter for a Player). Outside it, solids block.
                if distance >= radius * if self_hit { 0.25 } else { 0.5 }
                    && ctx
                        .world
                        .sweep(self.origin, target.center, Vec3::ZERO)
                        .fraction
                        < 1.
                {
                    return None;
                }
                Some(Hit {
                    id: target.id,
                    damage: damage * falloff * if self_hit { 0.5 } else { 1. },
                    kind,
                    knockback: offset.normalize_or_zero() * knockback * falloff,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::World;
    #[test]
    fn splash_excludes_direct_target_falls_off_and_halves_owner_damage() {
        let world = World::fixture(&[]);
        let actor = |id, x| Target {
            id,
            center: vec3(x, 0., 0.),
            half: Vec3::splat(8.),
        };
        let targets = [actor(1, 0.), actor(2, 150.)];
        let owner = actor(crate::dice::ALICE, 75.);
        let hits = Blast::new(Kind::Jack, Vec3::ZERO, 1).damage(
            &Context {
                world: &world,
                targets: &targets,
            },
            owner,
            Some(1),
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].damage, 150.);
        assert_eq!(hits[1].damage, 112.5);
        assert_eq!(hits[0].knockback.length(), 150.);
        let hits = Blast::new(Kind::Croquet, Vec3::ZERO, 1).damage(
            &Context {
                world: &world,
                targets: &[],
            },
            actor(crate::dice::ALICE, 45.),
            None,
        );
        assert!((hits[0].damage - 7.5).abs() < 0.001);
    }
    #[test]
    fn outer_splash_is_blocked_by_world_but_native_inner_radius_is_not() {
        let world = World::fixture(&[(vec3(30., -100., -100.), vec3(32., 100., 100.))]);
        let targets = [
            Target {
                id: 1,
                center: vec3(60., 0., 0.),
                half: Vec3::ONE,
            },
            Target {
                id: 2,
                center: vec3(200., 0., 0.),
                half: Vec3::ONE,
            },
        ];
        let hits = Blast::new(Kind::Jack, Vec3::ZERO, 1).damage(
            &Context {
                world: &world,
                targets: &targets,
            },
            Target {
                id: crate::dice::ALICE,
                center: vec3(-1000., 0., 0.),
                half: Vec3::ONE,
            },
            None,
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, 1);
    }
}

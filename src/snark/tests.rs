use super::*;
use crate::{
    ant::Timing,
    collision::{Collider, Liquid},
    skeletal::Transform,
};
struct Clips;
impl Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.2
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.04
    }
    fn speed(&self, _: &str, _: &str) -> f32 {
        240.
    }
}
impl Rig for Clips {
    fn tag(&self, _: &str, _: f32, _: &str) -> Transform {
        Transform {
            translation: vec3(35., 0., 0.),
            rotation: Quat::IDENTITY,
        }
    }
}
fn pool(mask: i32) -> World {
    let mut w = World::fixture(&[(vec3(-2000., -2000., -100.), vec3(2000., 2000., -50.))]);
    w.add_liquid(
        vec3(-1000., -1000., -50.),
        vec3(1000., 1000., 100.),
        mask,
        None,
    );
    w
}
fn fish(kind: Kind) -> Snark {
    Snark::new(kind, Vec3::ZERO, 0., 1., 7)
}
fn run(p: &mut Snark, w: &World, eye: Vec3, n: usize) -> Feedback {
    let mut f = Feedback::default();
    for _ in 0..n {
        p.step(w, eye, &Clips, &mut f);
    }
    f
}
fn hit(damage: f32, kind: DamageKind) -> Hit {
    Hit {
        id: 0,
        damage,
        kind,
        knockback: Vec3::ZERO,
    }
}
#[test]
fn water_and_fire_use_their_liquids_and_vertical_pursuit() {
    for kind in Kind::ALL {
        let w = pool(if kind == Kind::Fire { 8 } else { 32 });
        let mut p = fish(kind);
        let f = run(&mut p, &w, vec3(350., 0., 80.), 1000);
        assert!(p.feet.x > 150.);
        assert!(kind.wet(&w, p.feet));
        assert!(f.damage > 0.);
        p.validate().unwrap();
    }
    assert!(!Kind::Water.wet(&pool(8), Vec3::ZERO));
    assert!(!Kind::Water.wet(&pool(16), Vec3::ZERO));
}
#[test]
fn swimming_never_crosses_banks_or_solid_walls() {
    let mut w = World::fixture(&[(vec3(130., -1000., -100.), vec3(135., 1000., 1000.))]);
    w.add_liquid(vec3(-100., -100., -100.), vec3(100., 100., 100.), 32, None);
    let mut p = fish(Kind::BiteOnly);
    run(&mut p, &w, vec3(500., 0., 0.), 1200);
    assert!(p.feet.x <= 100.);
    assert!(Kind::Water.wet(&w, p.feet));
    p.hit(Hit {
        knockback: Vec3::X * 1000.,
        ..hit(1., DamageKind::Cards)
    });
    run(&mut p, &w, vec3(500., 0., 0.), 120);
    assert!(p.feet.x <= 100.);
    assert!(Kind::Water.wet(&w, p.feet));
    w.add_liquid(vec3(-500., -500., -500.), vec3(500., 500., 500.), 32, None);
    p.feet = Vec3::ZERO;
    run(&mut p, &w, vec3(120., 0., 0.), 1200);
    assert!(p.feet.x <= 106.);
}
#[test]
fn removed_moving_liquid_falls_without_teleporting() {
    let mut w = World::fixture(&[(vec3(-500., -500., -100.), vec3(500., 500., -50.))]);
    w.set_dynamic_liquids(vec![Liquid {
        contents: 32,
        volume: Collider::box_bounds(Vec3::splat(-100.), Vec3::splat(100.)),
    }]);
    let mut p = fish(Kind::BiteOnly);
    p.notarget = true;
    run(&mut p, &w, Vec3::ZERO, 120);
    assert_eq!(p.feet, Vec3::ZERO);
    w.set_dynamic_liquids(vec![]);
    run(&mut p, &w, Vec3::ZERO, 1);
    assert!(p.feet.z < 0. && p.feet.z > -1.);
    run(&mut p, &w, Vec3::ZERO, 360);
    assert!(p.feet.z >= -26.1 && p.feet.z < -25.);
}
#[test]
fn bite_contacts_once_at_frame_eight_and_respects_cover() {
    for kind in Kind::ALL {
        let mut p = fish(kind);
        p.set(Phase::Bite);
        let w = pool(32);
        let eye = vec3(80., 0., 25.);
        assert_eq!(run(&mut p, &w, eye, 38).damage, 0.);
        assert_eq!(run(&mut p, &w, eye, 1).damage, 10.);
        assert_eq!(run(&mut p, &w, eye, 80).damage, 0.);
        p.set(Phase::Bite);
        let w = World::fixture(&[(vec3(40., -500., -500.), vec3(45., 500., 500.))]);
        assert_eq!(run(&mut p, &w, eye, 100).damage, 0.);
    }
}
#[test]
fn bite_only_never_leaps_or_shoots_at_dry_targets() {
    let mut p = fish(Kind::BiteOnly);
    let w = pool(32);
    for _ in 0..3000 {
        run(&mut p, &w, vec3(350., 0., 300.), 1);
        assert!(!matches!(
            p.phase,
            Phase::Rise | Phase::Spit | Phase::Tongue | Phase::Dive
        ));
    }
    assert!(p.shots.is_empty());
    assert!(p.feet.z <= 100.);
}
#[test]
fn dry_bank_attack_rises_fires_and_returns_to_liquid() {
    for kind in [Kind::Water, Kind::Fire] {
        let mut p = fish(kind);
        let mut w = World::fixture(&[(vec3(-2000., -2000., -100.), vec3(2000., 2000., -50.))]);
        w.add_liquid(
            vec3(-1000., -1000., -50.),
            vec3(100., 1000., 100.),
            if kind == Kind::Fire { 8 } else { 32 },
            None,
        );
        let mut seen = std::collections::BTreeSet::new();
        let mut damage = 0.;
        let mut rose = false;
        for _ in 0..2400 {
            damage += run(&mut p, &w, vec3(350., 0., 180.), 1).damage;
            seen.insert(format!("{:?}", p.phase));
            rose |= p.feet.z > 100.;
        }
        assert!(
            rose && damage > 0.,
            "{kind:?}: damage={damage}, rose={rose}"
        );
        for phase in ["Rise", "Spit", "Dive", "Swim"] {
            assert!(seen.contains(phase), "Missing {phase}");
        }
        p.validate().unwrap();
    }
}
#[test]
fn tongue_pull_stops_on_pain_death_cover_and_range() {
    let mut p = fish(Kind::Water);
    p.set(Phase::Tongue);
    p.time = 0.3;
    let eye = vec3(180., 0., 50.);
    let w = pool(32);
    assert!(run(&mut p, &w, eye, 1).impulse.x < 0.);
    assert!(p.tongue.is_some());
    p.hit(hit(1., DamageKind::Knife));
    assert!(p.tongue.is_none());
    assert_eq!(run(&mut p, &w, eye, 1).impulse, Vec3::ZERO);
    p.set(Phase::Tongue);
    p.time = 0.3;
    assert_eq!(run(&mut p, &w, Vec3::X * 1000., 1).impulse, Vec3::ZERO);
    let wall = World::fixture(&[(vec3(60., -1000., -1000.), vec3(65., 1000., 1000.))]);
    assert_eq!(run(&mut p, &wall, eye, 1).impulse, Vec3::ZERO);
    p.hit(hit(100., DamageKind::Knife));
    assert!(p.tongue.is_none());
}
#[test]
fn shots_obey_cover_and_acid_has_three_bounded_ticks() {
    for kind in [Kind::Water, Kind::Fire] {
        let mut p = fish(kind);
        p.notarget = true;
        p.shots.push(Shot {
            serial: 1,
            position: vec3(35., 0., 0.),
            direction: Vec3::X,
            age: 0.,
        });
        let mut restored: Snark =
            serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        let eye = vec3(150., 0., 20.);
        let a = run(&mut p, &pool(32), eye, 600);
        let b = run(&mut restored, &pool(32), eye, 600);
        assert_eq!(a.damage, if kind == Kind::Fire { 25. } else { 20. });
        assert_eq!(a.damage, b.damage);
        assert!(p.shots.is_empty() && p.acid.is_empty());
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
        p.shots.push(Shot {
            serial: 2,
            position: vec3(35., 0., 0.),
            direction: Vec3::X,
            age: 0.,
        });
        let wall = World::fixture(&[(vec3(80., -500., -500.), vec3(85., 500., 500.))]);
        assert_eq!(run(&mut p, &wall, eye, 600).damage, 0.);
    }
}
#[test]
fn attack_cues_and_acid_continue_exactly_after_save_in_every_phase() {
    let w = pool(32);
    let eye = vec3(120., 0., 20.);
    for phase in [
        Phase::Idle,
        Phase::Swim,
        Phase::Rise,
        Phase::Spit,
        Phase::Tongue,
        Phase::Dive,
        Phase::Bite,
        Phase::Pain,
        Phase::Dead,
    ] {
        let mut p = fish(Kind::Water);
        p.set(phase);
        p.time = 0.31;
        if phase == Phase::Dead {
            p.health = 0.;
        }
        p.acid.push(Acid {
            victim: crate::dice::ALICE,
            time: 0.99,
            ticks: 1,
        });
        let mut restored: Snark =
            serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
        let a = run(&mut p, &w, eye, 500);
        let b = run(&mut restored, &w, eye, 500);
        assert_eq!((a.damage, a.impulse), (b.damage, b.impulse));
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
        p.validate().unwrap();
    }
}
#[test]
fn demon_damage_targets_demon_and_not_alice() {
    let mut p = fish(Kind::Water);
    p.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: Vec3::X * 80.,
        half: Vec3::splat(20.),
    });
    p.opponents.demon = true;
    p.set(Phase::Bite);
    let f = run(&mut p, &pool(32), vec3(500., 0., 20.), 60);
    assert_eq!(f.damage, 0.);
    assert_eq!(f.summon_hits.iter().map(|h| h.damage).sum::<f32>(), 10.);
}
#[test]
fn hurt_death_freeze_and_retirement_are_bounded() {
    for kind in Kind::ALL {
        let mut p = fish(kind);
        p.hit(hit(7., DamageKind::Cards));
        assert_eq!(p.phase, Phase::Pain);
        p.hit(hit(1000., DamageKind::Ice));
        assert_eq!(p.phase, Phase::Dead);
        assert!(p.frozen);
        let at = p.feet;
        run(&mut p, &pool(32), Vec3::ZERO, 5000);
        assert_eq!(p.feet, at);
        assert_eq!(p.sample_time(), 0.);
        assert_eq!(p.visual_scale(&Clips), 0.);
        let snapshot = serde_json::to_value(&p).unwrap();
        run(&mut p, &pool(32), Vec3::ZERO, 500);
        assert_eq!(snapshot, serde_json::to_value(&p).unwrap());
        p.validate().unwrap();
    }
}
#[test]
fn malformed_saves_reject_extra_attacks_nan_and_resurrection() {
    let mut p = fish(Kind::BiteOnly);
    p.set(Phase::Spit);
    assert!(p.validate().is_err());
    let mut p = fish(Kind::Water);
    p.health = 0.;
    assert!(p.validate().is_err());
    let mut p = fish(Kind::Water);
    p.feet.x = f32::NAN;
    assert!(p.validate().is_err());
    let mut p = fish(Kind::Water);
    p.acid.push(Acid {
        victim: 123,
        time: 0.,
        ticks: 0,
    });
    assert!(p.validate().is_err());
    let mut p = fish(Kind::Water);
    p.shots = vec![
        Shot {
            serial: 1,
            position: Vec3::ZERO,
            direction: Vec3::X,
            age: 0.
        };
        5
    ];
    assert!(p.validate().is_err());
}

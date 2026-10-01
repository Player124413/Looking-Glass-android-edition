use super::*;
use crate::{ant::Timing, skeletal::Transform};
struct Clips;
impl Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.2
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.04
    }
    fn speed(&self, _: &str, clip: &str) -> f32 {
        match clip {
            "walk_1" => 200.,
            "walk_2" => 140.,
            _ => 70.,
        }
    }
}
impl Rig for Clips {
    fn tag(&self, _: &str, _: f32, _: &str) -> Transform {
        Transform {
            translation: vec3(50., 0., 45.),
            rotation: Quat::IDENTITY,
        }
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-2000., -2000., -30.), vec3(2000., 2000., 0.))])
}
fn actor() -> Magma {
    let mut g = Magma::new(Vec3::Z * 0.1, 0., 1., 7);
    g.place(&floor());
    g
}
fn run(g: &mut Magma, w: &World, eye: Vec3, n: usize) -> Feedback {
    let mut f = Feedback::default();
    for _ in 0..n {
        g.step(w, eye, &Clips, &mut f);
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
fn punch_extends_once_and_never_hits_through_cover() {
    for (distance, first) in [(90., 10), (220., 12), (600., 15), (825., 16)] {
        let mut g = actor();
        g.set(Phase::Punch);
        g.notarget = true;
        let before = (first as f32 * 0.04 / DT).ceil() as usize - 1;
        assert_eq!(
            run(&mut g, &floor(), vec3(distance, 0., 48.), before).damage,
            0.
        );
        let damage = run(&mut g, &floor(), vec3(distance, 0., 48.), 100).damage;
        assert_eq!(damage, 10., "distance={distance}");
    }
    let w = World::fixture(&[
        (vec3(-2000., -2000., -30.), vec3(2000., 2000., 0.)),
        (vec3(140., -200., 0.), vec3(150., 200., 300.)),
    ]);
    let mut g = actor();
    g.set(Phase::Punch);
    assert_eq!(run(&mut g, &w, vec3(300., 0., 48.), 120).damage, 0.);
}
#[test]
fn forms_cool_reheat_and_lock_during_attacks() {
    let mut g = actor();
    g.notarget = true;
    run(&mut g, &floor(), Vec3::splat(10000.), 800);
    assert_eq!(g.form, 1);
    run(&mut g, &floor(), Vec3::splat(10000.), 800);
    assert_eq!(g.form, 2);
    let mut lava = floor();
    lava.add_liquid(vec3(-1000., -1000., 0.), vec3(1000., 1000., 20.), 8, None);
    g.set(Phase::Rock);
    run(&mut g, &lava, Vec3::splat(10000.), 100);
    assert_eq!(g.form, 2);
    run(&mut g, &lava, Vec3::splat(10000.), 500);
    assert_eq!(g.form, 0);
}
#[test]
fn fire_sword_immunity_pain_threshold_and_both_deaths() {
    let mut g = actor();
    g.hit(hit(999., DamageKind::FireSword));
    assert_eq!(g.health, 200.);
    g.hit(hit(24., DamageKind::Cards));
    assert_eq!(g.phase, Phase::Idle);
    g.hit(hit(25., DamageKind::Knife));
    assert_eq!(g.phase, Phase::Pain);
    for form in 0..3 {
        for kind in [DamageKind::Knife, DamageKind::Ice] {
            let mut g = actor();
            g.form = form;
            g.hit(hit(999., kind));
            g.validate().unwrap();
            assert_eq!(g.health, 0.);
            assert_eq!(g.frozen, kind == DamageKind::Ice);
            run(&mut g, &floor(), Vec3::ZERO, 1200);
            assert_eq!(g.visual_scale(&Clips), 0.);
            assert!(g.shots.is_empty());
        }
    }
}
#[test]
fn projectiles_contact_expire_and_cannot_start_through_a_wall() {
    let mut g = actor();
    g.set(Phase::Spit);
    let f = run(&mut g, &floor(), vec3(350., 0., 48.), 120);
    assert_eq!(f.damage, 25.);
    let w = World::fixture(&[
        (vec3(-2000., -2000., -30.), vec3(2000., 2000., 0.)),
        (vec3(45., -200., 0.), vec3(46., 200., 300.)),
    ]);
    let mut g = actor();
    g.set(Phase::Spit);
    run(&mut g, &w, vec3(350., 0., 48.), 70);
    assert!(g.shots.is_empty());
    let mut g = actor();
    g.set(Phase::Spit);
    run(&mut g, &floor(), vec3(1900., 0., 48.), 60);
    assert_eq!(g.shots.len(), 1);
    g.hit(hit(999., DamageKind::Knife));
    run(&mut g, &floor(), Vec3::splat(10000.), 300);
    assert!(g.shots.is_empty());
}
#[test]
fn molten_walk_accepts_supported_lava_but_rejects_water_and_cliffs() {
    for mask in [8, 16, 32] {
        let mut w = floor();
        w.add_liquid(vec3(50., -500., 0.), vec3(1000., 500., 20.), mask, None);
        let mut g = actor();
        g.set(Phase::Walk);
        run(&mut g, &w, vec3(700., 0., 48.), 600);
        assert_eq!(g.feet.x > 60., mask == 8, "mask={mask} x={}", g.feet.x);
    }
    let w = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
    let mut g = actor();
    run(&mut g, &w, vec3(700., 0., 48.), 1800);
    assert!(g.feet.x < 60.);
}
#[test]
fn phase_cues_projectiles_cooling_and_launch_resume_exactly() {
    for phase in [
        Phase::Punch,
        Phase::Rock,
        Phase::Bash,
        Phase::Spit,
        Phase::Pain,
        Phase::Walk,
    ] {
        let mut g = actor();
        g.set(phase);
        run(&mut g, &floor(), vec3(400., 0., 48.), 50);
        let mut restored: Magma =
            serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
        let a = run(&mut g, &floor(), vec3(400., 0., 48.), 400);
        let b = run(&mut restored, &floor(), vec3(400., 0., 48.), 400);
        assert_eq!(a.damage, b.damage);
        assert_eq!(
            serde_json::to_value(&g).unwrap(),
            serde_json::to_value(&restored).unwrap()
        );
        g.validate().unwrap();
    }
    let mut g = actor();
    g.launch = Some(vec3(100., 0., 600.));
    g.notarget = true;
    run(&mut g, &floor(), Vec3::splat(10000.), 45);
    let mut r: Magma = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
    run(&mut g, &floor(), Vec3::splat(10000.), 180);
    run(&mut r, &floor(), Vec3::splat(10000.), 180);
    assert_eq!(g.feet, r.feet);
    assert!(g.feet.z < 1.);
}
#[test]
fn corrupt_saved_projectiles_and_identity_are_rejected() {
    let mut g = actor();
    g.form = 3;
    assert!(g.validate().is_err());
    g.form = 0;
    g.shots = vec![
        crate::snark::Shot {
            serial: 1,
            position: Vec3::ZERO,
            direction: Vec3::X,
            age: 0.
        };
        5
    ];
    assert!(g.validate().is_err());
}
#[test]
fn rock_bash_hits_each_exposed_opponent_once() {
    let mut g = actor();
    g.form = 2;
    g.cooling = 12.;
    g.set(Phase::Bash);
    g.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: vec3(-100., 0., 32.),
        half: Vec3::splat(20.),
    });
    let f = run(&mut g, &floor(), vec3(100., 0., 48.), 100);
    assert_eq!(f.damage, 40.);
    assert_eq!(f.summon_hits.len(), 1);
    assert_eq!(f.summon_hits[0].damage, 40.);
    let later = run(&mut g, &floor(), Vec3::splat(10000.), 30);
    assert_eq!(later.damage, 0.);
}

use super::*;
struct Clips;
impl Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.04
    }
    fn speed(&self, _: &str, clip: &str) -> f32 {
        if clip == "walk_norm" {
            100.
        } else {
            60.
        }
    }
}
impl Rig for Clips {
    fn tag(&self, _: &str, _: f32, tag: &str) -> Transform {
        Transform {
            translation: vec3(20., if tag == "tag_left_hand" { -20. } else { 20. }, 60.),
            rotation: Quat::IDENTITY,
        }
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))])
}
fn clock() -> Automaton {
    Automaton::new(Vec3::Z * 0.1, 0., 1., 7, true)
}
fn hit(damage: f32, kind: DamageKind) -> Hit {
    Hit {
        id: 0,
        damage,
        kind,
        knockback: Vec3::ZERO,
    }
}
fn tick(p: &mut Automaton, w: &World, eye: Vec3, frames: usize, fps: f32) -> Feedback {
    let mut out = Feedback::default();
    for _ in 0..frames {
        p.update(1. / fps, w, eye, &Clips, &mut out);
    }
    out
}
#[test]
fn three_punch_contacts_once_each_and_invalid_hits_are_ignored() {
    let mut p = clock();
    p.set(Phase::Punch);
    let eye = vec3(95., 0., 48.);
    assert_eq!(tick(&mut p, &floor(), eye, 42, 120.).damage, 0.);
    assert_eq!(tick(&mut p, &floor(), eye, 2, 120.).damage, 5.);
    assert_eq!(tick(&mut p, &floor(), eye, 14, 120.).damage, 5.);
    assert_eq!(tick(&mut p, &floor(), eye, 20, 120.).damage, 5.);
    assert_eq!(tick(&mut p, &floor(), eye, 25, 120.).damage, 0.);
    for d in [f32::NAN, f32::INFINITY, -1., 0.] {
        p.hit(hit(d, DamageKind::Other));
    }
    assert_eq!(p.health, 400.);
    p.hit(hit(400., DamageKind::Knife));
    assert_eq!(tick(&mut p, &floor(), eye, 240, 120.).damage, 0.);
}
#[test]
fn steam_damages_each_body_once_and_cannot_reach_through_cover() {
    let mut p = clock();
    p.set(Phase::SteamFire);
    p.repeat = true;
    p.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: vec3(250., -15., 48.),
        half: Vec3::splat(20.),
    });
    let out = tick(&mut p, &floor(), vec3(200., 0., 60.), 100, 120.);
    assert_eq!(out.damage, 12.);
    assert_eq!(out.summon_hits.len(), 1);
    assert_eq!(out.summon_hits[0].damage, 12.);
    let wall = World::fixture(&[
        (vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.)),
        (vec3(120., -200., 0.), vec3(130., 200., 200.)),
    ]);
    let mut q = clock();
    q.set(Phase::SteamFire);
    assert_eq!(
        tick(&mut q, &wall, vec3(200., 0., 48.), 100, 120.).damage,
        0.
    );
    let mut q = clock();
    q.set(Phase::SteamFire);
    assert_eq!(
        tick(&mut q, &floor(), vec3(50., 200., 48.), 1, 120.).damage,
        0.
    );
}
#[test]
fn fists_are_separate_swept_projectiles_and_do_not_retarget_a_missing_summon() {
    let mut p = clock();
    p.set(Phase::FistFire);
    p.repeat = true;
    tick(&mut p, &floor(), vec3(900., 0., 48.), 8, 120.);
    assert!(p.fists.is_empty());
    tick(&mut p, &floor(), vec3(900., 0., 48.), 2, 120.);
    assert_eq!(p.fists.len(), 1);
    tick(&mut p, &floor(), vec3(900., 0., 48.), 50, 120.);
    assert_eq!(p.fists.len(), 2);
    let wall = World::fixture(&[(vec3(300., -500., -100.), vec3(305., 500., 500.))]);
    let out = tick(&mut p, &wall, vec3(900., 0., 48.), 200, 120.);
    assert_eq!(out.damage, 0.);
    assert!(p.fists.is_empty());
    let mut q = clock();
    q.phase = Phase::Dead;
    q.health = 0.;
    q.fists.push(Fist {
        serial: 1,
        position: vec3(0., 0., 100.),
        direction: Vec3::X,
        age: 0.,
        victim: crate::dice::SUMMON,
    });
    tick(&mut q, &floor(), vec3(0., 500., 48.), 1, 120.);
    assert_eq!(q.fists[0].direction, Vec3::X);
}
#[test]
fn mass_pain_threshold_frozen_and_normal_deaths_preserve_outcome() {
    let mut p = clock();
    let mut h = hit(30., DamageKind::Knife);
    h.knockback = Vec3::X * 30.;
    p.hit(h);
    assert_eq!(p.phase, Phase::Idle);
    p.hit(hit(35., DamageKind::Cards));
    assert_eq!(p.phase, Phase::Pain);
    let v = p.variant;
    tick(&mut p, &floor(), Vec3::splat(10000.), 12, 120.);
    let at = p.time;
    p.hit(hit(65., DamageKind::Other));
    assert_eq!(p.time, at);
    assert_eq!(p.variant, v);
    let x = p.feet.x;
    tick(&mut p, &floor(), Vec3::splat(10000.), 12, 120.);
    assert!(p.feet.x > x && p.feet.x < 10.);
    p.hit(hit(1000., DamageKind::Ice));
    assert!(p.frozen);
    assert_eq!(p.clip(), "death_frozen");
    tick(&mut p, &floor(), Vec3::ZERO, 1000, 120.);
    assert!(p.visual(&Clips).is_none());
    p.validate().unwrap();
    for y in [-1., 1.] {
        let mut p = clock();
        p.hit(Hit {
            knockback: Vec3::Y * y,
            ..hit(500., DamageKind::Other)
        });
        assert_eq!(p.variant, usize::from(y > 0.));
    }
}
#[test]
fn fps_pause_and_mid_attack_roundtrips_keep_the_same_future() {
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut p = clock();
        let f = tick(&mut p, &floor(), vec3(700., 0., 48.), 20 * fps, fps as f32);
        assert!(f.damage > 0.);
        p.validate().unwrap();
        results.push((p.feet, f.damage));
    }
    for pair in results.windows(2) {
        assert!(pair[0].0.distance(pair[1].0) < 0.1);
        assert_eq!(pair[0].1, pair[1].1);
    }
    for phase in [Phase::Punch, Phase::FistFire, Phase::SteamFire] {
        let mut p = clock();
        p.set(phase);
        tick(&mut p, &floor(), vec3(250., 0., 48.), 48, 120.);
        let saved = serde_json::to_value(&p).unwrap();
        for dt in [0., -1., f32::NAN] {
            p.update(dt, &floor(), Vec3::ZERO, &Clips, &mut Feedback::default());
        }
        assert_eq!(saved, serde_json::to_value(&p).unwrap());
        let mut q: Automaton = serde_json::from_value(saved).unwrap();
        let a = tick(&mut p, &floor(), vec3(250., 0., 48.), 240, 120.);
        let b = tick(&mut q, &floor(), vec3(250., 0., 48.), 240, 120.);
        assert_eq!(a.damage, b.damage);
        assert_eq!(
            serde_json::to_value(&p).unwrap(),
            serde_json::to_value(q).unwrap()
        );
    }
}
#[test]
fn dormant_notarget_delays_resource_bounds_and_retired_physics() {
    let mut p = clock();
    p.active = false;
    assert_eq!(
        tick(&mut p, &floor(), vec3(80., 0., 48.), 600, 60.).damage,
        0.
    );
    assert_eq!(p.phase, Phase::Idle);
    p.spawn_delay = Some(1.);
    tick(&mut p, &floor(), Vec3::splat(10000.), 30, 60.);
    assert!(!p.active);
    tick(&mut p, &floor(), Vec3::splat(10000.), 31, 60.);
    assert!(p.active);
    p.notarget = true;
    assert_eq!(
        tick(&mut p, &floor(), vec3(80., 0., 48.), 600, 60.).damage,
        0.
    );
    p.notarget = false;
    for _ in 0..100 {
        p.set(Phase::FistFire);
        tick(&mut p, &floor(), vec3(950., 0., 48.), 60, 120.);
        assert!(p.fists.len() <= 8);
        p.validate().unwrap();
    }
    p.hit(hit(500., DamageKind::Knife));
    let empty = World::fixture(&[]);
    tick(&mut p, &empty, Vec3::splat(10000.), 1500, 120.);
    let feet = p.feet;
    tick(&mut p, &empty, Vec3::splat(10000.), 1500, 120.);
    assert_eq!(p.feet, feet);
    assert!(p.fists.is_empty() && p.impacts.is_empty());
    let mut bad = p.clone();
    bad.variant = 3;
    assert!(bad.validate().is_err());
    bad = p;
    bad.health = 5.;
    assert!(bad.validate().is_err());
}

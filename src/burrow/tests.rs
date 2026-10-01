use super::*;
struct Clips;
impl Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.4
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.04
    }
    fn speed(&self, _: &str, _: &str) -> f32 {
        200.
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))])
}
fn wall(min: Vec3, max: Vec3) -> World {
    World::fixture(&[
        (vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.)),
        (min, max),
    ])
}
fn insect(k: Kind) -> Insect {
    Insect::new(k, vec3(0., 0., 0.1), 0., 1., 7)
}
fn run(p: &mut Insect, w: &World, eye: Vec3, n: usize) -> Feedback {
    let mut out = Feedback::default();
    for _ in 0..n {
        p.step(w, eye, &Clips, &mut out);
    }
    out
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
fn underground_ambush_needs_sight_and_cannot_be_hit_while_hidden() {
    let mut p = insect(Kind::Underground);
    p.hit(hit(1000., DamageKind::Ice));
    assert_eq!(p.health, 120.);
    let w = World::fixture(&[(vec3(40., -1000., -20.), vec3(50., 1000., 400.))]);
    run(&mut p, &w, vec3(100., 0., 48.), 60);
    assert_eq!(p.phase, Phase::Hidden);
    let mut p = insect(Kind::Underground);
    run(&mut p, &floor(), vec3(80., 0., 48.), 1);
    assert_eq!(p.phase, Phase::Rise);
    assert!(p.vulnerable());
    assert!(run(&mut p, &floor(), vec3(80., 0., 48.), 900).damage > 0.);
}
#[test]
fn original_melee_frames_have_exact_damage_once_per_contact() {
    for k in [Kind::Antlion, Kind::Underground] {
        for variant in 0..3 {
            let mut p = insect(k);
            p.variant = variant;
            p.set(Phase::Melee);
            let expected: f32 = p.melee_events().iter().map(|e| e.1).sum();
            let actual = run(&mut p, &floor(), vec3(80., 0., 48.), 165).damage;
            assert_eq!(actual, expected, "{k:?}/{variant}");
        }
    }
}
#[test]
fn missed_cues_are_consumed_and_never_damage_through_cover() {
    let mut p = insect(Kind::Antlion);
    p.set(Phase::Melee);
    let w = wall(vec3(40., -1000., 0.), vec3(45., 1000., 200.));
    assert_eq!(run(&mut p, &w, vec3(80., 0., 48.), 140).damage, 0.);
    assert_eq!(run(&mut p, &floor(), vec3(80., 0., 48.), 20).damage, 0.);
}
#[test]
fn tunnel_is_bounded_and_uses_solid_and_ledge_checks() {
    let mut p = insect(Kind::Antlion);
    p.set(Phase::Tunnel);
    run(&mut p, &floor(), vec3(800., 0., 48.), 600);
    assert!(!matches!(p.phase, Phase::Hidden | Phase::Tunnel));
    let w = World::fixture(&[(vec3(-200., -200., -20.), vec3(60., 200., 0.))]);
    let mut p = insect(Kind::Antlion);
    p.set(Phase::Tunnel);
    run(&mut p, &w, vec3(800., 0., 48.), 480);
    assert!(p.feet.x < 70. && p.feet.z > -1.);
}
#[test]
fn covered_tunnel_waits_for_safe_emergence_without_teleporting() {
    let mut p = insect(Kind::Antlion);
    p.set(Phase::Tunnel);
    let w = wall(vec3(-50., -50., 1.), vec3(50., 50., 100.));
    run(&mut p, &w, vec3(80., 0., 48.), 900);
    assert_eq!(p.phase, Phase::Tunnel);
    assert!(p.time <= 4.);
    assert_eq!(p.feet.x, 0.);
    run(&mut p, &floor(), vec3(80., 0., 48.), 1);
    assert_eq!(p.phase, Phase::Rise);
}
fn attached() -> Insect {
    let mut p = insect(Kind::Larva);
    for _ in 0..1200 {
        run(&mut p, &floor(), vec3(80., 0., 48.), 1);
        if p.phase == Phase::Suck {
            return p;
        }
    }
    panic!("Larva never attached: {:?} {:?}", p.phase, p.feet);
}
#[test]
fn larva_leaps_attaches_drains_then_dies_with_bounded_lifetime() {
    let mut p = attached();
    assert_eq!(p.victim, Some(crate::dice::ALICE));
    let out = run(&mut p, &floor(), vec3(80., 0., 48.), 1200);
    assert_eq!(out.damage, 6.);
    assert_eq!(p.phase, Phase::Dead);
    assert_eq!(p.health, 0.);
    assert!(p.victim.is_none());
    p.validate().unwrap();
}
#[test]
fn attachment_tracks_motion_and_releases_on_teleport_or_cover() {
    let mut p = attached();
    let before = p.feet;
    run(&mut p, &floor(), vec3(82., 0., 48.), 1);
    assert!((p.feet.x - before.x - 2.).abs() < 0.01);
    let out = run(&mut p, &floor(), vec3(500., 0., 48.), 1);
    assert_eq!(out.damage, 0.);
    assert_eq!(p.phase, Phase::Detach);
    let mut p = attached();
    let body = p.target(0);
    let w = wall(body.center - Vec3::splat(1.), body.center + Vec3::splat(1.));
    assert_eq!(run(&mut p, &w, vec3(80., 0., 48.), 1).damage, 0.);
    assert_eq!(p.phase, Phase::Detach);
}
#[test]
fn frozen_or_hurt_larva_releases_host_and_stops_damage() {
    for kind in [DamageKind::Cards, DamageKind::Ice] {
        let mut p = attached();
        p.hit(hit(1., kind));
        assert_eq!(p.health, 0.);
        assert!(p.victim.is_none());
        assert_eq!(p.frozen, kind == DamageKind::Ice);
        assert_eq!(run(&mut p, &floor(), vec3(80., 0., 48.), 1200).damage, 0.);
        p.validate().unwrap();
    }
}
#[test]
fn attachment_serialization_preserves_drain_cues_and_future() {
    let mut p = attached();
    run(&mut p, &floor(), vec3(80., 0., 48.), 30);
    let mut restored: Insect = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(
        run(&mut p, &floor(), vec3(80., 0., 48.), 800).damage,
        run(&mut restored, &floor(), vec3(80., 0., 48.), 800).damage
    );
    assert_eq!(
        serde_json::to_value(p).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
}
#[test]
fn demon_attachment_routes_damage_and_releases_when_summon_expires() {
    let mut p = attached();
    p.victim = Some(crate::dice::SUMMON);
    p.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: vec3(80., 0., 28.),
        half: vec3(15., 15., 28.),
    });
    let f = run(&mut p, &floor(), vec3(1000., 0., 48.), 1);
    assert_eq!(f.damage, 0.);
    assert_eq!(f.summon_hits.len(), 1);
    p.opponents.summon = None;
    run(&mut p, &floor(), vec3(1000., 0., 48.), 1);
    assert_eq!(p.phase, Phase::Detach);
}
#[test]
fn larva_cannot_leap_through_a_wall() {
    let mut p = insect(Kind::Larva);
    p.set(Phase::Leap);
    p.velocity = vec3(300., 0., 200.);
    let w = wall(vec3(40., -1000., 0.), vec3(45., 1000., 200.));
    assert_eq!(run(&mut p, &w, vec3(80., 0., 48.), 120).damage, 0.);
    assert!(p.feet.x < 40.);
    assert!(p.victim.is_none());
}
#[test]
fn validation_rejects_wrong_phase_host_and_nonfinite_state() {
    let mut p = insect(Kind::Antlion);
    p.set(Phase::Suck);
    assert!(p.validate().is_err());
    let mut p = attached();
    p.victim = Some(123);
    assert!(p.validate().is_err());
    let mut p = insect(Kind::Larva);
    p.velocity.x = f32::NAN;
    assert!(p.validate().is_err());
    let mut p = insect(Kind::Antlion);
    p.health = 0.;
    assert!(p.validate().is_err());
}

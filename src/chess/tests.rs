use super::*;
struct Clips;
impl Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.03
    }
    fn speed(&self, _: &str, clip: &str) -> f32 {
        if clip.ends_with("_a") {
            300.
        } else {
            100.
        }
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))])
}
fn piece(kind: Kind) -> Piece {
    Piece::new(kind, Vec3::Z * 0.1, 0., 1., 11)
}
fn hit(damage: f32, kind: DamageKind) -> Hit {
    Hit {
        id: 0,
        damage,
        kind,
        knockback: Vec3::ZERO,
    }
}
fn tick(p: &mut Piece, w: &World, eye: Vec3, seconds: u32, fps: u32) -> Feedback {
    let mut out = Feedback::default();
    for _ in 0..seconds * fps {
        p.update(1. / fps as f32, w, eye, &Clips, &mut out);
    }
    out
}
#[test]
fn all_pieces_attack_deterministically_and_pause_exactly() {
    for kind in Kind::ALL {
        let mut results = Vec::new();
        for fps in [30, 60, 144] {
            let mut p = piece(kind);
            let out = tick(&mut p, &floor(), vec3(450., 0., 48.), 15, fps);
            assert!(out.damage > 0., "{kind:?}");
            if kind != Kind::Bishop {
                assert!(p.feet.x > 100.);
            }
            let saved = serde_json::to_value(&p).unwrap();
            for dt in [0., -1., f32::NAN] {
                p.update(dt, &floor(), Vec3::ZERO, &Clips, &mut Feedback::default());
            }
            assert_eq!(saved, serde_json::to_value(&p).unwrap());
            p.validate().unwrap();
            results.push((p.feet, out.damage));
        }
        for r in results.windows(2) {
            assert!(r[0].0.distance(r[1].0) < 0.1);
            assert!((r[0].1 - r[1].1).abs() < 0.1);
        }
    }
}
#[test]
fn contact_occurs_once_at_authored_frame_and_stops_on_death() {
    for kind in Kind::ALL {
        let mut p = piece(kind);
        p.set(Phase::Melee);
        let mut out = Feedback::default();
        for _ in 0..20 {
            p.update(1. / 120., &floor(), vec3(70., 0., 48.), &Clips, &mut out);
        }
        assert_eq!(out.damage, 0.);
        for _ in 0..85 {
            p.update(1. / 120., &floor(), vec3(70., 0., 48.), &Clips, &mut out);
        }
        assert_eq!(
            out.damage,
            match kind {
                Kind::Pawn => 20.,
                Kind::Knight => 10.,
                _ => 15.,
            }
        );
        p.hit(hit(1000., DamageKind::Other));
        assert_eq!(
            tick(&mut p, &floor(), vec3(70., 0., 48.), 10, 60).damage,
            0.
        );
        assert_eq!(p.visual_scale(&Clips), 0.);
        let saved = serde_json::to_value(&p).unwrap();
        p.hit(hit(1000., DamageKind::Ice));
        assert_eq!(saved, serde_json::to_value(&p).unwrap());
    }
}
#[test]
fn knight_blocks_front_but_rear_energy_and_recovery_are_vulnerable() {
    let mut p = piece(Kind::Knight);
    let mut attack = hit(25., DamageKind::Knife);
    attack.knockback = -Vec3::X * 20.;
    p.hit(attack);
    assert_eq!(p.health, 150.);
    assert_eq!(p.phase, Phase::BlockEnd);
    p.hit(attack);
    assert_eq!(p.health, 125.);
    let mut p = piece(Kind::Knight);
    attack.knockback = Vec3::X * 20.;
    p.hit(attack);
    assert_eq!(p.health, 125.);
    let mut p = piece(Kind::Knight);
    p.threatened(true);
    assert_eq!(p.phase, Phase::Block);
    p.hit(hit(40., DamageKind::Electric));
    assert_eq!(p.health, 110.);
    p.hit(hit(1000., DamageKind::DemonIce));
    assert!(p.frozen);
    p.validate().unwrap();
}
#[test]
fn damage_does_not_restart_pain_and_waiting_actors_do_not_attack() {
    for kind in Kind::ALL {
        let mut p = piece(kind);
        p.script_wait = true;
        assert_eq!(tick(&mut p, &floor(), vec3(70., 0., 48.), 2, 60).damage, 0.);
        p.hit(hit(1., DamageKind::Other));
        p.update(0.1, &floor(), Vec3::ZERO, &Clips, &mut Feedback::default());
        let time = p.time;
        p.hit(hit(1., DamageKind::Other));
        assert_eq!(p.time, time);
        p.hit(hit(1000., DamageKind::Ice));
        assert!(p.frozen);
        let saved = serde_json::to_vec(&p).unwrap();
        let mut restored: Piece = serde_json::from_slice(&saved).unwrap();
        tick(&mut p, &floor(), Vec3::ZERO, 10, 60);
        tick(&mut restored, &floor(), Vec3::ZERO, 10, 60);
        assert_eq!(
            serde_json::to_value(p).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
}
#[test]
fn cover_notarget_and_ledge_stop_attacks_and_charges() {
    let wall = World::fixture(&[
        (vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.)),
        (vec3(200., -2000., 0.), vec3(220., 2000., 300.)),
    ]);
    for kind in Kind::ALL {
        let mut p = piece(kind);
        assert_eq!(tick(&mut p, &wall, vec3(300., 0., 48.), 5, 60).damage, 0.);
        let mut p = piece(kind);
        p.notarget = true;
        assert_eq!(tick(&mut p, &floor(), vec3(70., 0., 48.), 5, 60).damage, 0.);
    }
    let ledge = World::fixture(&[(vec3(-200., -200., -20.), vec3(150., 200., 0.))]);
    let mut p = piece(Kind::Rook);
    p.set(Phase::Charge);
    assert_eq!(tick(&mut p, &ledge, vec3(400., 0., 48.), 5, 60).damage, 0.);
    assert!(
        p.feet.x + p.kind.half().x - 1. <= 150.1 && p.feet.z >= 0.,
        "{:?}",
        p.feet
    );
}
#[test]
fn delayed_spawn_pause_and_roundtrip_never_revive_a_dead_piece() {
    let mut p = piece(Kind::Pawn);
    p.active = false;
    p.spawn_delay = Some(0.75);
    p.update(0., &floor(), Vec3::ZERO, &Clips, &mut Feedback::default());
    assert_eq!(p.spawn_delay, Some(0.75));
    let mut p: Piece = serde_json::from_slice(&serde_json::to_vec(&p).unwrap()).unwrap();
    tick(&mut p, &floor(), vec3(70., 0., 48.), 1, 60);
    assert!(p.active);
    p.hit(hit(1000., DamageKind::Other));
    tick(&mut p, &floor(), Vec3::ZERO, 10, 60);
    assert_eq!(p.health, 0.);
    p.validate().unwrap();
}
#[test]
fn malformed_states_are_rejected_before_clip_lookup() {
    let mut p = piece(Kind::Pawn);
    p.phase = Phase::Beam;
    assert!(p.validate().is_err());
    p.phase = Phase::Pain;
    p.variant = 99;
    assert!(p.validate().is_err());
    p.variant = 0;
    p.feet.x = f32::NAN;
    assert!(p.validate().is_err());
}

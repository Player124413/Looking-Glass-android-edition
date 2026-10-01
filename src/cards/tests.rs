use super::*;
struct Clips;
impl crate::ant::Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        2.5
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.05
    }
    fn speed(&self, _: &str, _: &str) -> f32 {
        120.
    }
}
impl crate::clockwork::Rig for Clips {
    fn tag(&self, _: &str, _: f32, _: &str) -> crate::skeletal::Transform {
        crate::skeletal::Transform {
            translation: vec3(30., 0., 70.),
            rotation: Quat::IDENTITY,
        }
    }
}
impl Rig for Clips {
    fn sever(&self) -> Option<&dismember::Recipe> {
        None
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-2000., -2000., -30.), vec3(2000., 2000., 0.))])
}
fn guard(kind: Kind) -> Guard {
    Guard::new(kind, Vec3::Z * 0.1, 0., 1., 7)
}
fn tick(g: &mut Guard, w: &World, eye: Vec3, n: usize) -> Feedback {
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
fn melee_contacts_are_once_and_can_be_dodged() {
    for (kind, variant, expected) in [
        (Kind::Heart, 0, 15.),
        (Kind::Heart, 1, 15.),
        (Kind::Heart, 2, 30.),
        (Kind::Heart, 3, 30.),
        (Kind::Spade, 0, 20.),
        (Kind::Spade, 1, 15.),
    ] {
        let mut g = guard(kind);
        g.variant = variant;
        g.set(Phase::Melee);
        assert_eq!(
            tick(&mut g, &floor(), vec3(90., 0., 48.), 295).damage,
            expected
        );
        let mut g = guard(kind);
        g.variant = variant;
        g.set(Phase::Melee);
        assert_eq!(tick(&mut g, &floor(), vec3(250., 0., 48.), 295).damage, 0.);
    }
    let mut g = guard(Kind::Heart);
    g.set(Phase::Combo);
    assert_eq!(tick(&mut g, &floor(), vec3(90., 0., 48.), 295).damage, 60.);
}
#[test]
fn cover_and_notarget_cancel_committed_damage() {
    let wall = World::fixture(&[(vec3(45., -500., -100.), vec3(55., 500., 500.))]);
    for kind in [Kind::Heart, Kind::Spade] {
        let mut g = guard(kind);
        g.set(Phase::Melee);
        assert_eq!(tick(&mut g, &wall, vec3(90., 0., 48.), 295).damage, 0.);
        g = guard(kind);
        g.set(Phase::Fire);
        g.notarget = true;
        assert_eq!(tick(&mut g, &floor(), vec3(180., 0., 48.), 295).damage, 0.);
        assert!(g.shots.is_empty());
    }
}
#[test]
fn pain_threshold_does_not_change_a_running_heart_attack() {
    let mut g = guard(Kind::Heart);
    g.set(Phase::Melee);
    g.variant = 3;
    g.time = 0.8;
    g.hit(hit(20., DamageKind::Other));
    assert_eq!(g.clip(), "attack4");
    assert_eq!(g.time, 0.8);
    g.hit(hit(15., DamageKind::Other));
    assert_eq!(g.phase, Phase::Pain);
    for d in [f32::NAN, f32::INFINITY, -1., 0.] {
        g.hit(hit(d, DamageKind::Other));
    }
    assert_eq!(g.health, 165.);
    let mut g = guard(Kind::Spade);
    g.hit(hit(1., DamageKind::Other));
    assert_eq!(g.phase, Phase::Pain);
}
#[test]
fn ranged_frames_emit_one_heart_or_paired_spades() {
    for (kind, n) in [(Kind::Heart, 1), (Kind::Spade, 2)] {
        let mut g = guard(kind);
        g.vision = 4000.;
        g.set(Phase::Fire);
        tick(&mut g, &floor(), vec3(1800., 0., 48.), 190);
        assert_eq!(g.serial, n);
    }
    let mut g = guard(Kind::Heart);
    g.vision = 4000.;
    g.set(Phase::Slam);
    tick(&mut g, &floor(), vec3(1800., 0., 48.), 227);
    assert_eq!(g.serial, 0);
    tick(&mut g, &floor(), vec3(1800., 0., 48.), 2);
    assert_eq!(g.serial, 1);
}
#[test]
fn shots_stop_at_world_and_direct_damage_is_not_doubled() {
    for (kind, damage) in [(Kind::Heart, 20.), (Kind::Spade, 13.)] {
        let shot = Shot {
            serial: 1,
            position: vec3(0., 0., 28.),
            direction: Vec3::X,
            age: 0.,
            victim: crate::dice::ALICE,
        };
        let mut g = guard(kind);
        g.notarget = true;
        g.shots.push(shot.clone());
        assert_eq!(
            tick(&mut g, &floor(), vec3(80., 0., 48.), 60).damage,
            damage
        );
        assert!(g.shots.is_empty());
        let mut g = guard(kind);
        g.notarget = true;
        g.shots.push(shot);
        let wall = World::fixture(&[(vec3(30., -100., -100.), vec3(40., 100., 100.))]);
        assert_eq!(tick(&mut g, &wall, vec3(80., 0., 48.), 60).damage, 0.);
        assert!(g.shots.is_empty());
    }
}
#[test]
fn heart_seeking_is_bounded_and_does_not_retarget_missing_demon() {
    let mut g = guard(Kind::Heart);
    g.notarget = true;
    g.shots.push(Shot {
        serial: 1,
        position: vec3(0., 0., 28.),
        direction: Vec3::X,
        age: 0.,
        victim: crate::dice::ALICE,
    });
    g.projectiles(&floor(), vec3(200., 200., 48.), &mut Feedback::default());
    assert!(g.shots[0].direction.y > 0. && g.shots[0].direction.y < 0.018);
    g.shots[0].victim = crate::dice::SUMMON;
    let direction = g.shots[0].direction;
    g.projectiles(&floor(), vec3(-200., -200., 48.), &mut Feedback::default());
    assert_eq!(g.shots[0].direction, direction);
}
#[test]
fn demon_can_draw_guard_aggression_and_take_its_hit() {
    let mut g = guard(Kind::Spade);
    g.notarget = true;
    g.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: vec3(90., 0., 48.),
        half: Vec3::splat(16.),
    });
    g.set(Phase::Melee);
    g.opponents.demon = true;
    let f = tick(&mut g, &floor(), vec3(500., 0., 48.), 295);
    assert_eq!(f.damage, 0.);
    assert_eq!(f.summon_hits.len(), 2);
}
#[test]
fn death_is_final_and_all_saved_resources_expire() {
    for kind in [Kind::Heart, Kind::Spade] {
        for means in [DamageKind::Other, DamageKind::Knife, DamageKind::Ice] {
            let mut g = guard(kind);
            g.hit(hit(1000., means));
            assert_eq!(g.cut, kind == Kind::Spade && means == DamageKind::Knife);
            assert_eq!(g.frozen, means == DamageKind::Ice);
            assert_eq!(tick(&mut g, &floor(), vec3(90., 0., 48.), 1800).damage, 0.);
            assert!(g.shots.is_empty() && g.impacts.is_empty() && g.dismember.fragment.is_none());
            assert_eq!(g.fade(&Clips), 0.);
            g.validate().unwrap();
            let saved = serde_json::to_vec(&g).unwrap();
            g.hit(hit(100., DamageKind::Ice));
            assert_eq!(saved, serde_json::to_vec(&g).unwrap());
        }
    }
}
#[test]
fn saved_attacks_projectiles_recoil_and_cuts_continue_exactly() {
    for kind in [Kind::Heart, Kind::Spade] {
        for cut in [false, true] {
            let mut g = guard(kind);
            g.set(Phase::Fire);
            tick(&mut g, &floor(), vec3(900., 0., 48.), 100);
            if cut {
                g.hit(Hit {
                    knockback: Vec3::X * 100.,
                    ..hit(1000., DamageKind::Knife)
                });
                tick(&mut g, &floor(), vec3(900., 0., 48.), 40);
            }
            let mut restored: Guard =
                serde_json::from_slice(&serde_json::to_vec(&g).unwrap()).unwrap();
            restored.validate().unwrap();
            let a = tick(&mut g, &floor(), vec3(700., 0., 48.), 900);
            let b = tick(&mut restored, &floor(), vec3(700., 0., 48.), 900);
            assert_eq!(a.damage, b.damage);
            assert_eq!(
                serde_json::to_vec(&g).unwrap(),
                serde_json::to_vec(&restored).unwrap()
            );
        }
    }
}
#[test]
fn malformed_saves_are_rejected() {
    let mut g = guard(Kind::Spade);
    g.phase = Phase::Charge;
    assert!(g.validate().is_err());
    g = guard(Kind::Heart);
    g.cut = true;
    assert!(g.validate().is_err());
    g = guard(Kind::Spade);
    g.shots = vec![
        Shot {
            serial: 1,
            position: Vec3::ZERO,
            direction: Vec3::X,
            age: 0.,
            victim: crate::dice::ALICE
        };
        13
    ];
    assert!(g.validate().is_err());
    g = guard(Kind::Heart);
    g.time = f32::NAN;
    assert!(g.validate().is_err());
}
#[test]
fn local_chase_stays_before_walls_and_ledge() {
    let world = World::fixture(&[
        (vec3(-100., -100., -30.), vec3(100., 100., 0.)),
        (vec3(80., -200., 0.), vec3(90., 200., 300.)),
    ]);
    for kind in [Kind::Heart, Kind::Spade] {
        let mut g = guard(kind);
        g.set(Phase::Chase);
        g.memory = 6.;
        g.last_seen = vec3(1000., 0., 48.);
        tick(&mut g, &world, vec3(1000., 0., 48.), 600);
        assert!(g.feet.x < 50. && g.feet.z >= 0.);
    }
}

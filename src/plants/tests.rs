use super::*;
use crate::{ant::Timing, skeletal::Transform};
struct Clips;
impl Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.5
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.04
    }
    fn speed(&self, _: &str, _: &str) -> f32 {
        0.
    }
}
impl Rig for Clips {
    fn tag(&self, _: &str, _: f32, _: &str) -> Transform {
        Transform {
            translation: vec3(50., 0., 60.),
            rotation: Quat::IDENTITY,
        }
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-3000., -3000., -20.), vec3(3000., 3000., 0.))])
}
fn plant(kind: Kind) -> Plant {
    Plant::new(kind, Vec3::Z * 0.1, 0., 1., 12)
}
fn run(p: &mut Plant, w: &World, eye: Vec3, n: usize) -> Feedback {
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
        knockback: vec3(100., 0., 0.),
    }
}
#[test]
fn dormant_plants_respect_sight_range_and_invulnerability() {
    for kind in [Kind::Rose, Kind::Mushroom] {
        let mut p = plant(kind);
        assert!(!p.vulnerable());
        p.hit(hit(500., DamageKind::Knife));
        assert_eq!(p.health, kind.health());
        run(&mut p, &floor(), vec3(1100., 0., 48.), 240);
        assert_eq!(p.phase, Phase::Dormant);
        let wall = World::fixture(&[(vec3(60., -1000., -100.), vec3(80., 1000., 1000.))]);
        run(&mut p, &wall, vec3(180., 0., 48.), 240);
        assert_eq!(p.phase, Phase::Dormant);
        p.notarget = true;
        run(&mut p, &floor(), vec3(180., 0., 48.), 120);
        assert_eq!(p.phase, Phase::Dormant);
        p.notarget = false;
        run(&mut p, &floor(), vec3(180., 0., 48.), 1);
        assert!(p.vulnerable());
    }
}
#[test]
fn rose_growth_survives_pain_and_ignores_knockback() {
    let mut p = plant(Kind::Rose);
    run(&mut p, &floor(), vec3(300., 0., 80.), 30);
    let grown = p.growth;
    let pos = p.feet;
    p.hit(hit(5., DamageKind::Knife));
    assert_eq!(p.phase, Phase::Pain);
    assert_eq!(p.growth, grown);
    run(&mut p, &floor(), vec3(300., 0., 80.), 120);
    assert_eq!(p.growth, 0.7);
    assert_eq!(p.feet, pos);
}
#[test]
fn rose_melee_contacts_once_and_cover_blocks_damage() {
    let mut p = plant(Kind::Rose);
    p.grow_for_check();
    p.set(Phase::Melee);
    assert_eq!(run(&mut p, &floor(), vec3(180., 0., 48.), 65).damage, 0.);
    assert_eq!(run(&mut p, &floor(), vec3(180., 0., 48.), 8).damage, 25.);
    assert_eq!(run(&mut p, &floor(), vec3(180., 0., 48.), 60).damage, 0.);
    p.set(Phase::Melee);
    let wall = World::fixture(&[(vec3(60., -1000., -100.), vec3(80., 1000., 1000.))]);
    assert_eq!(run(&mut p, &wall, vec3(180., 0., 48.), 160).damage, 0.);
}
#[test]
fn fan_spawns_five_distinct_frame_contacts_without_replaying_after_save() {
    let mut p = plant(Kind::Rose);
    p.grow_for_check();
    p.set(Phase::Fan);
    run(&mut p, &floor(), vec3(900., 0., 70.), 60);
    assert_eq!(p.serial, 1);
    let mut restored: Plant = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    for _ in 0..65 {
        run(&mut p, &floor(), vec3(900., 0., 70.), 1);
        run(&mut restored, &floor(), vec3(900., 0., 70.), 1);
    }
    assert_eq!(p.serial, 5);
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(&restored).unwrap()
    );
}
#[test]
fn projectile_first_contact_world_occlusion_and_resource_cap() {
    let mut p = plant(Kind::Rose);
    p.grow_for_check();
    p.set(Phase::Spit);
    let eye = vec3(300., 0., 68.);
    for _ in 0..100 {
        p.fire(&Clips, eye, 10., 0.);
    }
    assert_eq!(p.shots.len(), 12);
    let wall = World::fixture(&[(vec3(150., -1000., -100.), vec3(160., 1000., 1000.))]);
    assert_eq!(run(&mut p, &wall, eye, 120).damage, 0.);
    assert!(p.shots.is_empty());
    p.fire(&Clips, eye, 10., 0.);
    assert_eq!(run(&mut p, &floor(), eye, 80).damage, 10.);
}
#[test]
fn mushroom_pull_stops_for_cover_distance_notarget_and_death() {
    for stop in 0..4 {
        let mut p = plant(Kind::Mushroom);
        p.set(Phase::Suck);
        let eye = vec3(250., 0., 48.);
        assert!(run(&mut p, &floor(), eye, 1).impulse.x < 0.);
        let wall = World::fixture(&[(vec3(100., -1000., -100.), vec3(110., 1000., 1000.))]);
        match stop {
            0 => {}
            1 => p.notarget = true,
            2 => {
                p.hit(hit(500., DamageKind::Knife));
            }
            _ => {}
        }
        let open = floor();
        let f = run(
            &mut p,
            if stop == 0 { &wall } else { &open },
            if stop == 3 { vec3(500., 0., 48.) } else { eye },
            1,
        );
        assert_eq!(f.impulse, Vec3::ZERO);
    }
}
#[test]
fn mushroom_digest_is_bounded_interruptible_and_does_not_lock_player() {
    let mut p = plant(Kind::Mushroom);
    p.set(Phase::Digest);
    let f = run(&mut p, &floor(), vec3(80., 0., 48.), 181);
    assert_eq!(f.damage, 15.);
    assert_eq!(p.phase, Phase::Release);
    assert!(f.impulse.x > 0.);
    p.set(Phase::Digest);
    p.hit(hit(30., DamageKind::Knife));
    assert_eq!(p.phase, Phase::Pain);
    assert_eq!(run(&mut p, &floor(), vec3(80., 0., 48.), 100).damage, 0.);
}
#[test]
fn demon_can_draw_plant_aggro_and_receive_damage() {
    let mut p = plant(Kind::Rose);
    p.grow_for_check();
    p.set(Phase::Melee);
    p.notarget = true;
    p.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: vec3(180., 0., 80.),
        half: Vec3::splat(20.),
    });
    p.opponents.demon = true;
    let f = run(&mut p, &floor(), Vec3::splat(2000.), 100);
    assert_eq!(f.damage, 0.);
    assert_eq!(f.summon_hits.len(), 1);
    assert_eq!(f.summon_hits[0].damage, 25.);
}
#[test]
fn death_retires_and_malformed_state_is_rejected() {
    for kind in [Kind::Rose, Kind::Mushroom] {
        let mut p = plant(kind);
        p.set(Phase::Ready);
        p.hit(hit(1000., DamageKind::Ice));
        run(&mut p, &floor(), vec3(300., 0., 48.), 1200);
        p.validate().unwrap();
        let state = serde_json::to_value(&p).unwrap();
        run(&mut p, &floor(), vec3(300., 0., 48.), 120);
        assert_eq!(state, serde_json::to_value(&p).unwrap());
        p.time = f32::NAN;
        assert!(p.validate().is_err());
        p.time = 0.;
        p.phase = Phase::Ready;
        assert!(p.validate().is_err());
    }
}

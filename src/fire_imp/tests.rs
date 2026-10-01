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
        if clip == "run" {
            200.
        } else {
            90.
        }
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))])
}
#[test]
fn dormant_pad_launch_pauses_and_resumes_in_flight() {
    let mut p=Imp::new(Vec3::Z*0.1,0.,1.,0,false);
    p.launch=Some(vec3(100.,0.,300.));
    let w=floor();
    tick(&mut p,&w,Vec3::splat(10000.),60,60.);
    assert_eq!(p.feet,Vec3::Z*0.1);
    p.spawn_delay=Some(0.);
    tick(&mut p,&w,Vec3::splat(10000.),12,60.);
    assert!(p.feet.x>15. && p.feet.z>30.);
    let saved=serde_json::to_value(&p).unwrap();
    p.update(0.,&w,Vec3::ZERO,&Clips,&mut Feedback::default());
    assert_eq!(serde_json::to_value(&p).unwrap(),saved);
    let mut q:Imp=serde_json::from_value(saved).unwrap();
    for _ in 0..120 {tick(&mut p,&w,Vec3::splat(10000.),1,120.);tick(&mut q,&w,Vec3::splat(10000.),1,120.);}
    assert_eq!(serde_json::to_value(&p).unwrap(),serde_json::to_value(&q).unwrap());
    assert!(p.flight.is_none() && p.launch.is_none() && p.feet.z<1.);
    p.validate().unwrap();
}
fn imp(seed: usize) -> Imp {
    Imp::new(Vec3::Z * 0.1, 0., 1., seed, true)
}
fn hit(damage: f32, kind: DamageKind) -> Hit {
    Hit {
        id: 0,
        damage,
        kind,
        knockback: Vec3::ZERO,
    }
}
fn tick(p: &mut Imp, w: &World, eye: Vec3, frames: usize, fps: f32) -> Feedback {
    let mut out = Feedback::default();
    for _ in 0..frames {
        p.update(1. / fps, w, eye, &Clips, &mut out);
    }
    out
}
#[test]
fn pursuit_attack_frame_and_frame_rate_independence() {
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut p = imp(11);
        let out = tick(&mut p, &floor(), vec3(450., 0., 48.), fps * 15, fps as f32);
        assert!(p.feet.x > 200. && out.damage > 0.);
        p.validate().unwrap();
        results.push((p.feet, out.damage));
    }
    for pair in results.windows(2) {
        assert!(pair[0].0.distance(pair[1].0) < 0.1);
        assert_eq!(pair[0].1, pair[1].1);
    }
    let mut p = imp(0);
    p.set(Phase::Attack);
    assert_eq!(
        tick(&mut p, &floor(), vec3(70., 0., 48.), 42, 120.).damage,
        0.
    );
    assert_eq!(
        tick(&mut p, &floor(), vec3(70., 0., 48.), 2, 120.).damage,
        10.
    );
    assert_eq!(
        tick(&mut p, &floor(), vec3(70., 0., 48.), 50, 120.).damage,
        0.
    );
}
#[test]
fn fire_sword_immunity_pain_threshold_ice_and_invalid_damage() {
    let mut p = imp(0);
    for kind in [DamageKind::FireSword, DamageKind::DemonFire] {
        p.hit(hit(100., kind));
    }
    for damage in [f32::NAN, f32::INFINITY, -10., 0.] {
        p.hit(hit(damage, DamageKind::Other));
    }
    assert_eq!(p.health, 35.);
    p.hit(hit(7., DamageKind::Fire));
    assert_eq!(p.health, 28.);
    assert_eq!(p.phase, Phase::Idle);
    p.hit(hit(18., DamageKind::Knife));
    assert_eq!(p.phase, Phase::Pain);
    p.hit(hit(10., DamageKind::Ice));
    assert!(p.frozen && !p.gibbed && p.health == 0.);
    assert_eq!(
        tick(&mut p, &floor(), vec3(30., 0., 48.), 600, 60.).damage,
        0.
    );
    assert_eq!(p.visual_scale(&Clips), 0.);
    p.validate().unwrap();
}
#[test]
fn walls_height_retreat_and_ledge_prevent_unfair_contact() {
    let wall = World::fixture(&[
        (vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.)),
        (vec3(35., -200., 0.), vec3(40., 200., 200.)),
    ]);
    for (w, eye) in [
        (&wall, vec3(70., 0., 48.)),
        (&floor(), vec3(70., 0., 300.)),
        (&floor(), vec3(180., 0., 48.)),
    ] {
        let mut p = imp(0);
        p.set(Phase::Attack);
        assert_eq!(tick(&mut p, w, eye, 100, 120.).damage, 0.);
    }
    let ledge = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
    let mut p = imp(1);
    tick(&mut p, &ledge, vec3(450., 0., 48.), 1200, 60.);
    assert!(p.feet.x <= 85.1 && p.feet.z > -1.);
}
#[test]
fn summon_receives_melee_and_notarget_stops_player_attacks() {
    let mut p = imp(0);
    p.notarget = true;
    assert_eq!(
        tick(&mut p, &floor(), vec3(60., 0., 48.), 600, 60.).damage,
        0.
    );
    p.opponents.summon = Some(Target {
        id: crate::dice::SUMMON,
        center: vec3(65., 0., 30.),
        half: Vec3::splat(20.),
    });
    p.opponents.demon = true;
    let out = tick(&mut p, &floor(), vec3(600., 0., 48.), 600, 60.);
    assert_eq!(out.damage, 0.);
    assert!(!out.summon_hits.is_empty());
}
#[test]
fn delayed_spawn_pause_and_save_continue_without_duplicate_contact() {
    let mut p = imp(11);
    p.active = false;
    p.spawn_delay = Some(2.);
    let saved = serde_json::to_value(&p).unwrap();
    for dt in [0., -1., f32::NAN] {
        p.update(dt, &floor(), Vec3::ZERO, &Clips, &mut Feedback::default());
    }
    assert_eq!(saved, serde_json::to_value(&p).unwrap());
    tick(&mut p, &floor(), vec3(65., 0., 48.), 60, 60.);
    assert!(!p.active);
    let mut restored: Imp = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    let a = tick(&mut p, &floor(), vec3(65., 0., 48.), 300, 60.);
    let b = tick(&mut restored, &floor(), vec3(65., 0., 48.), 300, 60.);
    assert!(p.active && a.damage > 0.);
    assert_eq!(a.damage, b.damage);
    assert_eq!(
        serde_json::to_value(p).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
}
#[test]
fn gib_probability_restrictions_saved_physics_cleanup_and_validation() {
    let mut count = 0;
    for seed in 0..1000 {
        let mut p = imp(seed);
        p.hit(hit(100., DamageKind::Other));
        if p.gibbed {
            count += 1;
            assert_eq!(p.fragments.as_ref().unwrap().len(), 5);
        }
        p.validate().unwrap();
        for kind in [DamageKind::Knife, DamageKind::Ice] {
            let mut q = imp(seed);
            q.hit(hit(100., kind));
            assert!(!q.gibbed);
        }
    }
    assert!((100..200).contains(&count));
    let mut p = (0..1000)
        .map(|seed| {
            let mut p = imp(seed);
            p.hit(hit(100., DamageKind::Other));
            p
        })
        .find(|p| p.gibbed)
        .unwrap();
    tick(&mut p, &floor(), Vec3::ZERO, 30, 60.);
    let mut q: Imp = serde_json::from_value(serde_json::to_value(&p).unwrap()).unwrap();
    tick(&mut p, &floor(), Vec3::ZERO, 90, 60.);
    tick(&mut q, &floor(), Vec3::ZERO, 90, 60.);
    assert_eq!(
        serde_json::to_value(&p).unwrap(),
        serde_json::to_value(&q).unwrap()
    );
    q.variant = 2;
    assert!(q.validate().is_err());
    q = p.clone();
    q.health = 35.;
    assert!(q.validate().is_err());
    tick(&mut p, &floor(), Vec3::ZERO, 600, 60.);
    assert!(p.fragments.is_none());
    p.validate().unwrap();
}

#[test]
fn retired_corpse_stops_falling_and_real_blade_hits_kill_in_two_swings() {
    let mut p = imp(0);
    for _ in 0..2 {
        let targets = [p.target(0)];
        let hits = crate::weapons::blade_melee(
            &crate::combat::Context {
                world: &floor(),
                targets: &targets,
            },
            vec3(-30., 0., 25.),
            Vec3::X,
        );
        assert_eq!(hits.len(), 1);
        p.hit(hits[0]);
    }
    assert_eq!(p.health, 0.);
    assert!(!p.gibbed);
    let empty = World::fixture(&[]);
    tick(&mut p, &empty, Vec3::ZERO, 1200, 60.);
    let feet = p.feet;
    tick(&mut p, &empty, Vec3::ZERO, 12000, 60.);
    assert_eq!(p.feet, feet);
    p.validate().unwrap();
}

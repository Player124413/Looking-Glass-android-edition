use super::*;
struct Clips;
impl crate::ant::Timing for Clips {
    fn duration(&self, _: &str, _: &str) -> f32 {
        1.5
    }
    fn frame(&self, _: &str, _: &str) -> f32 {
        0.04
    }
    fn speed(&self, _: &str, _: &str) -> f32 {
        160.
    }
}
impl Rig for Clips {
    fn tag(&self, _: &str, _: f32, _: &str) -> crate::skeletal::Transform {
        crate::skeletal::Transform {
            translation: vec3(20., 0., 45.),
            rotation: Quat::IDENTITY,
        }
    }
}
fn floor() -> World {
    World::fixture(&[(vec3(-3000., -3000., -20.), vec3(3000., 3000., 0.))])
}
fn wall() -> World {
    World::fixture(&[
        (vec3(-3000., -3000., -20.), vec3(3000., 3000., 0.)),
        (vec3(60., -1000., 0.), vec3(65., 1000., 400.)),
    ])
}
#[test]
fn spider_pursues_last_seen_position_but_does_not_attack_through_cover() {
    let mut g=make(Kind::Spider);
    g.set(Phase::Run);
    run(&mut g,&floor(),vec3(700.,0.,48.),1);
    assert_eq!(g.memory,3.);
    let cover=World::fixture(&[(vec3(-3000.,-3000.,-20.),vec3(3000.,3000.,0.)),(vec3(300.,-200.,0.),vec3(320.,200.,300.))]);
    let before=g.feet;
    let f=run(&mut g,&cover,vec3(700.,0.,48.),120);
    assert!(g.feet.x>before.x+50.,"Spider forgot target on losing sight");
    assert_eq!(f.damage,0.);
    let mut r:Creature=serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
    run(&mut g,&cover,vec3(700.,0.,48.),300);
    run(&mut r,&cover,vec3(700.,0.,48.),300);
    assert_eq!(serde_json::to_value(&g).unwrap(),serde_json::to_value(r).unwrap());
    assert_eq!(g.memory,0.);
    g.validate().unwrap();
    let mut legacy=serde_json::to_value(g).unwrap();
    for key in ["detour","last_seen","memory"] {legacy.as_object_mut().unwrap().remove(key);}
    serde_json::from_value::<Creature>(legacy).unwrap().validate().unwrap();
}
fn make(k: Kind) -> Creature {
    Creature::new(k, Vec3::Z * 0.1, 0., 1., 13)
}
fn run(g: &mut Creature, w: &World, eye: Vec3, n: usize) -> Feedback {
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
fn source_melee_cues_are_consumed_once() {
    for k in [
        Kind::Spider,
        Kind::WallSpider,
        Kind::Jabber,
        Kind::Sleeping3,
    ] {
        for v in 0..3 {
            let mut g = make(k);
            g.variant = v;
            g.set(Phase::Melee);
            assert_eq!(
                run(&mut g, &floor(), vec3(90., 0., 48.), 175).damage,
                if k.spider() { 10. } else { [10., 20., 10.][v] },
                "{k:?}/{v}"
            );
        }
    }
}
#[test]
fn gas_hits_each_body_only_once_and_restore_keeps_victim_bits() {
    for k in [Kind::Spider, Kind::WallSpider] {
        let mut g = make(k);
        g.set(Phase::Ranged);
        assert_eq!(run(&mut g, &floor(), vec3(190., 0., 48.), 50).damage, 10.);
        let mut restored: Creature =
            serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
        assert_eq!(
            run(&mut restored, &floor(), vec3(190., 0., 48.), 120).damage,
            0.
        );
        run(&mut g, &floor(), vec3(190., 0., 48.), 120);
        assert_eq!(
            serde_json::to_value(g).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
}
#[test]
fn cover_blocks_cues_and_missed_attacks_do_not_replay() {
    for k in [Kind::Spider, Kind::Jabber, Kind::Phantom] {
        let mut g = make(k);
        g.set(Phase::Melee);
        let f = run(&mut g, &wall(), vec3(90., 0., 48.), 150);
        assert_eq!(f.damage + f.will_drain, 0.);
        let f = run(&mut g, &floor(), vec3(90., 0., 48.), 20);
        assert_eq!(f.damage + f.will_drain, 0.);
    }
}
#[test]
fn phantom_drains_will_and_chain_releases_on_cover_or_death() {
    let mut g = make(Kind::Phantom);
    g.set(Phase::Melee);
    let f = run(&mut g, &floor(), vec3(90., 0., 48.), 160);
    assert_eq!(f.will_drain, 10.);
    assert_eq!(f.damage, 0.);
    g.set(Phase::Ranged);
    assert!(run(&mut g, &floor(), vec3(250., 0., 48.), 30).impulse.x < 0.);
    assert_eq!(
        run(&mut g, &wall(), vec3(250., 0., 48.), 1).impulse,
        Vec3::ZERO
    );
    assert!(g.beam.is_none());
    g.hit(hit(1000., DamageKind::Knife));
    assert!(g.beam.is_none());
    let f = run(&mut g, &floor(), vec3(90., 0., 48.), 900);
    assert_eq!(f.damage + f.will_drain, 0.);
    assert_eq!(g.phase, Phase::Dead);
    assert_eq!(g.visual_scale(&Clips), 0.);
}
#[test]
fn sleeping_jabber_wakes_and_beam_is_a_bounded_pulse() {
    let mut g = make(Kind::Sleeping3);
    assert_eq!(g.health, 250.);
    assert_eq!(g.clip(), "idle_sleep");
    run(&mut g, &wall(), vec3(190., 0., 48.), 100);
    assert_eq!(g.phase, Phase::Idle);
    run(&mut g, &floor(), vec3(190., 0., 48.), 1);
    assert_eq!(g.phase, Phase::Wake);
    assert_eq!(g.clip(), "idle_sleep_2_ready");
    g.set(Phase::Ranged);
    assert_eq!(run(&mut g, &floor(), vec3(300., 0., 48.), 175).damage, 5.);
    assert!(g.beam.is_none());
}
#[test]
fn wall_spider_waits_for_clear_departure_and_lands_without_crossing_walls() {
    let mut g = make(Kind::WallSpider);
    g.hit(hit(1000., DamageKind::Knife));
    assert_eq!(g.health, 150.);
    run(&mut g, &wall(), vec3(150., 0., 48.), 100);
    assert_eq!(g.phase, Phase::Wall);
    g.feet.z = 180.;
    let start = g.feet;
    run(&mut g, &floor(), vec3(300., 0., 48.), 1);
    assert_eq!(g.phase, Phase::WallJump);
    let mut peak = g.feet.z;
    for _ in 0..300 {
        run(&mut g, &floor(), vec3(300., 0., 48.), 1);
        peak = peak.max(g.feet.z);
    }
    assert!(peak > start.z);
    assert!(g.feet.z < 50.);
    let mut g = make(Kind::Jabber);
    g.leap(Vec3::X, false);
    run(&mut g, &wall(), vec3(300., 0., 48.), 180);
    assert!(g.feet.x < 60.);
    g.validate().unwrap();
}
#[test]
fn rocks_flee_settle_never_take_damage_and_respect_cliffs() {
    for k in [Kind::Rock, Kind::SmallRock] {
        let mut g = make(k);
        g.hit(hit(99999., DamageKind::Ice));
        assert_eq!(g.health, 10000.);
        assert!(!g.vulnerable());
        let f = run(&mut g, &floor(), vec3(60., 0., 48.), 900);
        assert_eq!(f.damage + f.will_drain, 0.);
        assert!(g.feet.x < -80.);
        assert_eq!(g.phase, Phase::Idle);
        g.validate().unwrap();
        let edge = World::fixture(&[(vec3(-40., -100., -20.), vec3(100., 100., 0.))]);
        let mut g = make(k);
        run(&mut g, &edge, vec3(60., 0., 48.), 1000);
        assert!(g.feet.x > -40. && g.feet.z >= 0.);
    }
}
#[test]
fn all_variants_save_mid_action_and_retire_after_ordinary_or_ice_death() {
    for k in Kind::ALL {
        let mut g = make(k);
        run(&mut g, &floor(), vec3(100., 0., 48.), 231);
        let mut r: Creature = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
        let a = run(&mut g, &floor(), vec3(100., 0., 48.), 750);
        let b = run(&mut r, &floor(), vec3(100., 0., 48.), 750);
        assert_eq!(
            (a.damage, a.will_drain, a.impulse),
            (b.damage, b.will_drain, b.impulse)
        );
        assert_eq!(
            serde_json::to_value(g).unwrap(),
            serde_json::to_value(r).unwrap()
        );
        if k.rock() {
            continue;
        }
        for hitkind in [DamageKind::Knife, DamageKind::Ice] {
            let mut g = make(k);
            g.set(Phase::Walk);
            g.hit(hit(1000., hitkind));
            run(&mut g, &floor(), vec3(100., 0., 48.), 1500);
            assert_eq!(g.visual_scale(&Clips), 0.);
            g.validate().unwrap();
        }
    }
}
#[test]
fn demon_contacts_do_not_drain_alice() {
    for k in [Kind::Phantom, Kind::Spider, Kind::Jabber] {
        let mut g = make(k);
        g.notarget = true;
        g.opponents.demon = true;
        g.opponents.summon = Some(Target {
            id: crate::dice::SUMMON,
            center: vec3(85., 0., 40.),
            half: Vec3::splat(20.),
        });
        g.set(Phase::Melee);
        let f = run(&mut g, &floor(), vec3(-500., 0., 48.), 160);
        assert_eq!(f.damage + f.will_drain, 0.);
        assert!(!f.summon_hits.is_empty(), "{k:?}");
    }
}
#[test]
fn malformed_saves_are_rejected() {
    let mut g = make(Kind::Jabber);
    g.time = f32::NAN;
    assert!(g.validate().is_err());
    let mut g = make(Kind::Rock);
    g.set(Phase::Ranged);
    assert!(g.validate().is_err());
    let mut g = make(Kind::Spider);
    g.set(Phase::Wall);
    assert!(g.validate().is_err());
    let mut g = make(Kind::Phantom);
    g.beam = Some(Vec3::ZERO);
    assert!(g.validate().is_err());
}

#[test]
fn web_anchor_is_finite_saved_and_released_on_pain_or_leap() {
    let mut g = make(Kind::Spider);
    g.set(Phase::WebStart);
    g.web = Some(vec3(0., 0., 260.));
    g.validate().unwrap();
    let mut restored: Creature = serde_json::from_value(serde_json::to_value(&g).unwrap()).unwrap();
    run(&mut g, &floor(), vec3(350., 0., 48.), 240);
    run(&mut restored, &floor(), vec3(350., 0., 48.), 240);
    assert_eq!(
        serde_json::to_value(&g).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
    assert!(matches!(g.phase, Phase::Web | Phase::Leap | Phase::Air));
    run(&mut g, &floor(), vec3(350., 0., 48.), 180);
    assert!(g.web.is_none());
    g.set(Phase::WebStart);
    g.web = Some(vec3(0., 0., 260.));
    g.hit(hit(30., DamageKind::Knife));
    assert!(g.web.is_none());
}

#[test]
fn summon_navigation_steps_steers_and_refuses_unsupported_ledges() {
    let world=World::fixture(&[
        (vec3(-400.,-400.,-20.),vec3(500.,400.,0.)),
        (vec3(130.,-400.,0.),vec3(500.,400.,12.)),
        (vec3(60.,-25.,0.),vec3(70.,25.,200.)),
    ]);
    let mut s=demon(1);
    let d=s.demon.as_mut().unwrap();
    d.feet=vec3(0.,-130.,0.1);
    for _ in 0..480 { move_demon(d,&world,vec3(900.,-130.,60.),120.); }
    assert!(d.feet.x>200. && d.feet.z>=12. && d.feet.z<12.1);
    assert!(d.feet.x<500.-d.target().half.x);
    d.feet=vec3(0.,0.,0.1);
    for _ in 0..480 { move_demon(d,&world,vec3(400.,0.,60.),120.); }
    assert!(d.feet.x>130., "Local obstruction should not pin a pursuing demon: {:?}",d.feet);
    assert!(!world.sweep(d.center(),d.center(),d.target().half).start_solid);
}

#[test]
fn restored_attack_events_cover_claw_ice_lightning_and_king_followup() {
    let world = floor();
    for (kind, phase, distance, minimum) in [
        (1, Phase::Claw, 80., 25.),
        (1, Phase::Ranged, 400., 25.),
        (2, Phase::Ice, 400., 75.),
        (2, Phase::Ranged, 400., 8.),
        (2, Phase::Melee, 80., 40.),
        (2, Phase::ChargeHit, 80., 35.),
    ] {
        let mut totals = Vec::new();
        for hz in [30, 60, 144] {
            let mut s = demon(kind);
            let d = s.demon.as_mut().unwrap();
            d.set(phase);
            d.target = Some(42);
            let target = Target { id:42, center:d.center()+Vec3::X*distance, half:Vec3::splat(24.) };
            let ctx = Context {world:&world, targets:&[target]};
            let mut damage = 0.;
            for frame in 0..hz*3 {
                if frame == hz/2 {
                    s=serde_json::from_value(encoded(&s)).unwrap();
                    s.validate().unwrap();
                }
                damage += s.advance(1./hz as f32, &ctx, Vec3::Z*60., &data()).hits.iter().map(|h|h.damage).sum::<f32>();
            }
            assert!(damage>=minimum, "{kind} {phase:?}: {damage}");
            totals.push(damage);
        }
        assert_eq!(totals[0],totals[1]);
        assert_eq!(totals[0],totals[2]);
    }
}

#[test]
fn summon_lasts_while_fighting_dismisses_and_normal_shield_is_not_invulnerability() {
    let world = floor();
    let mut s = demon(1);
    s.hurt(10.);
    let d=s.demon.as_ref().unwrap();
    assert_eq!((d.health,d.phase),(90.,Phase::Idle));
    assert!(d.shield>0.);
    s.hurt(40.);
    assert_eq!(s.demon.as_ref().unwrap().phase,Phase::Pain);
    let target=Target{id:1,center:vec3(200.,0.,60.),half:Vec3::splat(30.)};
    let ctx=Context{world:&world,targets:&[target]};
    for _ in 0..120*40 { s.advance(STEP,&ctx,Vec3::Z*60.,&data()); }
    assert!(s.target().is_some(), "Combat has no arbitrary thirty-second expiry");
    s.dismiss();
    for _ in 0..240 { assert!(s.advance(STEP,&ctx,Vec3::Z*60.,&data()).hits.is_empty()); }
    assert!(s.demon.is_none());
    let mut s=demon(2);
    s.hit(Hit{id:SUMMON,damage:200.,kind:combat::DamageKind::Ice,knockback:Vec3::X*100.});
    assert_eq!(s.demon.as_ref().unwrap().clip().0,"death_frozen");
    let saved=encoded(&s);
    s.advance_timed(0.1,false,&ctx,Vec3::Z*60.,&data());
    assert_eq!(saved["demon"],encoded(&s)["demon"]);
}

#[test]
fn rage_three_dice_force_king_and_failed_summons_refund_once() {
    let world=floor();
    let ctx=Context{world:&world,targets:&[]};
    for count in 1..=3 {
        let mut s=State {rage:true,..Default::default()};
        s.throw(count,Vec3::Z*60.,Vec3::X);
        for d in &mut s.dice { d.pip=1; }
        s=serde_json::from_value(encoded(&s)).unwrap();
        for _ in 0..300 { s.advance(STEP,&ctx,Vec3::Z*60.,&data()); }
        assert_eq!(s.demon.as_ref().unwrap().kind,if count==3 {2}else{0});
    }
    let world=World::fixture(&[(Vec3::splat(-1000.),Vec3::splat(1000.))]);
    let ctx=Context{world:&world,targets:&[]};
    let mut s=State::default();
    s.throw(3,Vec3::ZERO,Vec3::X);
    let mut refunded=0.;
    for _ in 0..1200 { refunded+=s.advance(STEP,&ctx,Vec3::Z*60.,&data()).refund; }
    assert_eq!(refunded,40.);
    assert!(s.ready());
}

#[test]
fn expiry_uses_only_unfrozen_fraction_and_preserves_beams_and_bolts() {
    let world=floor();
    let target=Target{id:1,center:vec3(500.,0.,60.),half:Vec3::splat(30.)};
    let ctx=Context{world:&world,targets:&[target]};
    let mut s=demon(2);
    s.demon.as_mut().unwrap().set(Phase::Ranged);
    s.demon.as_mut().unwrap().target=Some(1);
    s.beams.push(Beam {a:Vec3::Z*60.,b:target.center,life:0.1,kind:2});
    s.bolts.push(Bolt {id:1,position:Vec3::Z*150.,velocity:Vec3::X*800.,age:0.,hostile:false,ice:true});
    let mut restored:State=serde_json::from_value(encoded(&s)).unwrap();
    s.advance_clocks(0.05,0.025,&ctx,Vec3::Z*60.,&data());
    restored.advance_clocks(0.05,0.025,&ctx,Vec3::Z*60.,&data());
    assert_eq!(encoded(&s),encoded(&restored));
    assert!((s.demon.as_ref().unwrap().time-0.025).abs()<1e-6);
    assert!((s.beams[0].life-0.075).abs()<1e-6);
    assert!((s.bolts[0].position.x-20.).abs()<0.001);
}

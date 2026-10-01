use super::*;
pub fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/potears2.bsp")?)?;
    for hz in [30, 60, 144] {
        let mut o = PoolTwo::load(a, &map)?;
        let mut w = World::from_bsp(&map)?;
        ensure!(
            o.pond.pads.len() == 7 && o.pond.props.len() == 12,
            "Missing pond objects"
        );
        let pad = &o.pond.pads[5];
        let high = pad.pose.translation + Vec3::Z * 100.;
        let low = high - Vec3::Z * 200.;
        let hit = pad.collider.trace(
            high + crate::collision::PLAYER_CENTER,
            low + crate::collision::PLAYER_CENTER,
            crate::collision::PLAYER_HALF,
        );
        ensure!(hit.fraction < 1., "No deck");
        let feet = high.lerp(low, hit.fraction);
        let mut p = Player::new(feet);
        p.grounded = true;
        o.event("leaf1startmoving");
        for _ in 0..hz * 4 {
            o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
            ensure!(w.body_clear(p.feet), "Embedded rider");
        }
        ensure!(p.feet.distance(feet) > 300., "No ride/cue");
        let saved = o.snapshot();
        o.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(o.snapshot() == saved, "Paused pond moved");
        let mut restored = PoolTwo::load(a, &map)?;
        restored.restore(&saved, &map)?;
        ensure!(restored.snapshot() == saved, "Pond restore changed state");
        let mut q = p.clone();
        let mut v = World::from_bsp(&map)?;
        for _ in 0..hz * 4 {
            o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
            restored.advance(1. / hz as f32, &map, &mut v, &mut q, &[])?;
        }
        ensure!(
            p.feet.distance(q.feet) < 0.01 && o.snapshot() == restored.snapshot(),
            "Restored ride diverged"
        );
        o.event("leaf2startmoving");
        p = Player::new(vec3(-4592., 1792., 224.));
        for _ in 0..hz * 40 {
            o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            o.saved.pond.ladies
                && o.saved.pond.removed
                && o.pond.colliders(&o.saved.pond).len() == 6,
            "Second leaf never removed"
        );
        println!("PASS Hollow pads/leaves, rider, cue, pause, restore and removal at {hz} Hz");
    }
    let mut o = PoolTwo::load(a, &map)?;
    let mut w = World::from_bsp(&map)?;
    let mut p = Player::new(vec3(-448., 192., -272.));
    let mut stats = crate::inventory::Stats::default();
    for _ in 0..5 * 120 {
        o.advance(1. / 120., &map, &mut w, &mut p, &[])?;
    }
    ensure!(
        o.saved.fish.attack.is_none() && o.saved.fish.exposure > 4.9,
        "Wrong fish contact"
    );
    let saved = o.snapshot();
    let mut restored = PoolTwo::load(a, &map)?;
    restored.restore(&saved, &map)?;
    for _ in 0..121 {
        restored.advance(1. / 120., &map, &mut w, &mut p, &[])?;
    }
    ensure!(
        restored.saved.fish.attack.is_some()
            && restored.scene_id().is_none()
            && restored.camera(&w).is_some(),
        "Fish presentation missing/skippable"
    );
    for _ in 0..241 {
        restored.advance(1. / 120., &map, &mut w, &mut p, &[])?;
        restored.prepare_player(&mut stats, &mut p);
    }
    ensure!(
        !stats.alive() && restored.hides_player() && restored.saved.fish.killed,
        "Fish attack was not lethal"
    );
    let saved = restored.snapshot();
    restored.advance(0., &map, &mut w, &mut p, &[])?;
    ensure!(
        saved == restored.snapshot(),
        "Dead fish scene advanced while paused"
    );
    println!(
        "PASS fish contact, mid-timer restore, fixed camera, unskippable bite and standard death"
    );
    Ok(())
}

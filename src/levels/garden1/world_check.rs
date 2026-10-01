use super::*;
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/garden1.bsp")?)?;
    for hz in [30, 60, 144] {
        let mut o = Garden::load(a, &map)?;
        o.upgraded();
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(o.colliders());
        ensure!(
            o.movers.pads.len() == 3 && o.data.currents.len() == 2,
            "Missing garden movers/currents"
        );
        let mut player = Player::new(Vec3::ZERO);
        for pad in 0..3 {
            let p = &o.movers.pads[pad];
            let top = p.pose.translation + Vec3::Z * 128.;
            let trace = p.collider.trace(
                top + crate::collision::PLAYER_CENTER,
                top - Vec3::Z * 256. + crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
            );
            ensure!(
                !trace.start_solid && trace.fraction < 1.,
                "Lily lacks support"
            );
            player = Player::new(top - Vec3::Z * 256. * trace.fraction);
            let local = player.feet - p.pose.translation;
            for _ in 0..hz * 75 {
                o.advance(1. / hz as f32, &map, &mut w, &mut player, &[])?;
            }
            ensure!(
                (player.feet - o.movers.pads[pad].pose.translation - local).length() < 1.,
                "Lily rider drift at {hz}Hz: {:?}",
                player.feet
            );
        }
        let saved = o.snapshot();
        o.advance(0., &map, &mut w, &mut player, &[])?;
        ensure!(saved == o.snapshot(), "Pause advanced world");
        o.event("Bridge_Drop");
        o.event("Ant_Deadtree_Ambush");
        for _ in 0..hz * 3 {
            o.advance(1. / hz as f32, &map, &mut w, &mut player, &[])?;
        }
        let mut other = Garden::load(a, &map)?;
        other.restore(&o.snapshot(), &map)?;
        ensure!(
            o.transforms() == other.transforms() && o.snapshot() == other.snapshot(),
            "Mover restore changed pose"
        );
        let mut bad = o.snapshot();
        bad["world"]["time"] = serde_json::json!(-1.);
        let old = o.snapshot();
        ensure!(
            o.restore(&bad, &map).is_err() && o.snapshot() == old,
            "Invalid world save partially applied"
        );
        for _ in 0..hz * 15 {
            o.advance(1. / hz as f32, &map, &mut w, &mut player, &[])?;
        }
        ensure!(
            o.transforms().len() == 1,
            "Collapsed bridge pieces remain drawn"
        );
        println!("PASS garden {hz}Hz: three full lily cycles, bound clips/riders, pause, bridge and saved future");
    }
    let o = Garden::load(a, &map)?;
    let mut previous = o.data.deadtree_pose(0.).translation;
    for step in 1..=(o.data.deadtree_duration() * 120.).ceil() as usize {
        let feet = o.data.deadtree_pose(step as f32 / 120.).translation;
        ensure!(
            feet.distance(previous) < 20.,
            "Dead-tree Rabbit ground discontinuity {previous:?} -> {feet:?}"
        );
        ensure!(
            !o.data
                .world
                .sweep(
                    feet + Vec3::Z * 32.,
                    feet + Vec3::Z * 32.,
                    vec3(10., 10., 30.)
                )
                .start_solid,
            "Dead-tree Rabbit inside world at {feet:?}"
        );
        previous = feet;
    }
    println!("PASS dead-tree Rabbit follows supplied path nodes around terrain");
    let mut w = World::from_bsp(&map)?;
    o.traversal(&mut w.traversal);
    for (at, sign) in [
        (vec3(-1792., 128., 32.), Vec3::NEG_Y),
        (vec3(-440., 1900., 860.), vec3(-1., -1., 0.)),
    ] {
        let mut velocity = Vec3::ZERO;
        for _ in 0..120 {
            crate::traversal::current::apply(&w.traversal.currents, &w, at, &mut velocity);
        }
        ensure!(
            velocity.dot(sign) > 80. && velocity.length() <= 200.1,
            "River flow wrong at {at:?}: {velocity:?}"
        );
    }
    let mut velocity = Vec3::ZERO;
    crate::traversal::current::apply(
        &w.traversal.currents,
        &w,
        vec3(3000., 3000., 3000.),
        &mut velocity,
    );
    ensure!(
        velocity == Vec3::ZERO,
        "Current leaks outside authored bounds"
    );
    println!("PASS lower/upper river directions, speed cap and off-path exclusion");
    for (id, field) in [(85, "rabbit"), (207, "bridge")] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(a, &map, "garden1", None)?;
        owner(&mut i)?.upgraded();
        let e = &map.entities[id];
        let model = e["model"].trim_start_matches('*').parse()?;
        let volume = Collider::model(&map, model, data::at(e).translation, Quat::IDENTITY, false)?;
        let feet = volume
            .interior_point()
            .context("World contact lacks interior")?
            - crate::collision::PLAYER_CENTER;
        i.triggers(crate::movement::FIXED_DT, feet, feet);
        let first = owner(&mut i)?.snapshot()["world"][field].clone();
        ensure!(first.is_number(), "Real contact {id} did not start {field}");
        i.triggers(crate::movement::FIXED_DT, feet, feet);
        ensure!(
            owner(&mut i)?.snapshot()["world"][field] == first,
            "Repeated contact restarted {field}"
        );
    }
    println!("PASS real bridge/dead-tree volumes dispatch once to the existing owner");
    for reported in [false, true] {
        let mut i = Interactions::load(&map)?;
        i.set_entry(a, &map, "garden1", None)?;
        owner(&mut i)?.upgraded();
        let mut old = serde_json::to_value(i.snapshot())?;
        old["levels"]["garden1"]
            .as_object_mut()
            .unwrap()
            .remove("world");
        for trigger in old["triggers"].as_array_mut().unwrap() {
            if [62, 85, 207].contains(&trigger["id"].as_u64().unwrap()) {
                trigger["fired"] = serde_json::json!(true);
                trigger["reported"] = serde_json::json!(reported);
            }
        }
        i.restore(&serde_json::from_value(old)?, &map)?;
        let saved = serde_json::to_value(i.snapshot())?;
        for trigger in saved["triggers"].as_array().unwrap() {
            if [62, 85, 207].contains(&trigger["id"].as_u64().unwrap()) {
                ensure!(
                    trigger["fired"] == serde_json::json!(!reported),
                    "Migration replayed a completed event or retained pending contact"
                );
            }
        }
    }
    println!("PASS only formerly pending world contacts rearm on old saves");
    Ok(())
}

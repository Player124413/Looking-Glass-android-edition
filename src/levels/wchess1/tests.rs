//! Behavioral fixtures use the production owner, combat and save loader.
use super::*;
use crate::{
    combat::{DamageKind, Hit},
    movement::FIXED_DT,
};
fn settle(r: &mut Realm, map: &Bsp, w: &mut World, p: &mut Player, seconds: f32) -> Result<()> {
    for _ in 0..(seconds / FIXED_DT).ceil() as usize {
        r.advance(FIXED_DT, map, w, p, &[])?;
    }
    Ok(())
}
fn roundtrip(a: &mut Assets, map: &Bsp, r: &Realm) -> Result<Realm> {
    let bytes = serde_json::to_vec(&r.snapshot())?;
    let mut other = Realm::load(a, map)?;
    other.restore(&serde_json::from_slice(&bytes)?, map)?;
    ensure!(r.snapshot() == other.snapshot(), "Chess JSON changed state");
    Ok(other)
}
pub(super) fn run(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/wchess1.bsp")?)?;
    clock_hands(a, &map)?;
    // Continuous campaign regression: Alice can enter beneath the descending lift.
    // Its underside must stop at contact, including after restoring the moving lift.
    for hz in [30., 60., 144.] {
        let mut r = Realm::load(a, &map)?;
        r.saved.pending = None;
        r.saved.intro = true;
        r.saved.elevator = motion::Lift { phase: 0, time: 2.2833356, height: -135.82239 };
        r.rebuild(&map)?;
        let mut p = Player::new(vec3(-755.6678, -459.48755, 56.146362));
        let mut w = World::from_bsp(&map)?;
        w.set_dynamic(r.colliders());
        ensure!(w.body_clear(p.feet), "Lift contact fixture starts embedded");
        let mut restored = roundtrip(a, &map, &r)?;
        let mut q = p.clone();
        let mut v = World::from_bsp(&map)?;
        v.set_dynamic(restored.colliders());
        let before = r.saved.elevator.height;
        for _ in 0..3 {
            r.advance(1. / hz, &map, &mut w, &mut p, &[])?;
            restored.advance(1. / hz, &map, &mut v, &mut q, &[])?;
            ensure!(w.body_clear(p.feet) && v.body_clear(q.feet), "Lift crushed a non-rider");
            ensure!(r.saved.elevator.height == before && r.snapshot() == restored.snapshot(), "Blocked lift or restored clock diverged");
        }
        // Once Alice leaves its swept space, the same lift continues normally.
        p = Player::new(r.data.point("wchess1_start1").translation);
        r.advance(1. / hz, &map, &mut w, &mut p, &[])?;
        ensure!(r.saved.elevator.height < before, "Lift stayed stuck after clearance");
    }
    println!("PASS descending lift non-rider clearance and restored motion at 30/60/144 Hz");
    // Check the actual origin-bone attachment, which turns downward-authored
    // chess TAN coordinates upright. A raw feet transform buries the disguise.
    let def = crate::skeletal::Definition::load(a, "models/alice.tik")?;
    let rig = crate::skeletal::Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
    let idle = crate::skeletal::Animation::parse(
        &a.read(&format!("{}/{}", def.path, def.animations["idle"]))?,
        rig.bones.len(),
    )?;
    let origin = rig
        .bones
        .iter()
        .position(|b| b.name == "ORIGIN")
        .context("Alice origin missing")?;
    let tag = rig.global_pose(&idle.sample(0., false))[origin];
    for model in ["bishop", "knight"] {
        let tan = crate::tan::Model::parse(&a.read(&format!("models/{model}/notmoving.tan"))?)?;
        let (lo, hi) = tan
            .surfaces
            .iter()
            .flat_map(|s| &s.frames[0])
            .map(|p| tag.point(*p))
            .fold(
                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                |(lo, hi), p| (lo.min(p), hi.max(p)),
            );
        ensure!(
            lo.z >= -2. && hi.z > 70. && hi.z < 150.,
            "Incorrect {model} attachment {lo:?} {hi:?}"
        );
    }
    println!("PASS supplied chess TANs on Alice origin attachment");
    for kind in [
        scene::Kind::Intro,
        scene::Kind::Bishop,
        scene::Kind::Knight,
        scene::Kind::Bell,
        scene::Kind::Water,
    ] {
        for skip in [false, true] {
            let mut r = Realm::load(a, &map)?;
            r.saved.pending = Some(kind);
            r.saved.intro = kind != scene::Kind::Intro;
            if kind == scene::Kind::Knight {
                r.saved.bishop_done = true;
                r.saved.knight_gate = true;
            }
            let start = match kind {
                scene::Kind::Bishop => "alice_bishop_puzzle_dest1",
                scene::Kind::Knight => "alice_knight_puzzle_dest1",
                _ => "wchess1_start1",
            };
            let mut p = Player::new(r.data.point(start).translation);
            let mut w = World::from_bsp(&map)?;
            settle(&mut r, &map, &mut w, &mut p, 1.)?;
            let snapshot = r.snapshot();
            r.advance(0., &map, &mut w, &mut p, &[])?;
            ensure!(snapshot == r.snapshot(), "Pause advanced a scene");
            let mut other = roundtrip(a, &map, &r)?;
            let mut q = p.clone();
            let mut v = World::from_bsp(&map)?;
            for tick in 0..24000 {
                if skip {
                    r.skip_scene(&map, &mut w, &mut p)?;
                    other.skip_scene(&map, &mut v, &mut q)?;
                }
                r.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
                other.advance(FIXED_DT, &map, &mut v, &mut q, &[])?;
                if tick % 120 == 0 {
                    ensure!(
                        r.snapshot() == other.snapshot() && p.feet == q.feet,
                        "Restored scene diverged"
                    );
                }
                if !r.scripted() && !r.controlled() {
                    break;
                }
            }
            ensure!(
                !r.scripted() && !r.controlled(),
                "Scene failed to hand off {kind:?}"
            );
            if kind == scene::Kind::Water {
                ensure!(r.saved.age >= 4.7, "Water skip bypassed staging");
                settle(&mut r, &map, &mut w, &mut p, 8.)?;
                ensure!(r.water_ready(), "Water never released exit");
                let water = r.objects.iter().find(|o| o.name == "water").unwrap();
                ensure!(
                    (water.pose.translation.z - water.base.translation.z - 96.).abs() < 0.01,
                    "Wrong water height"
                );
                ensure!(
                    r.objects
                        .iter()
                        .find(|o| o.name == "water_exit_block")
                        .is_some_and(|o| !o.solid),
                    "Exit block remains"
                );
            }
            ensure!(
                w.body_clear(p.feet),
                "Scene handoff overlaps {kind:?} at {:?}",
                p.feet
            );
            roundtrip(a, &map, &r)?;
        }
    }
    println!("PASS five scenes: watched/skipped, paused, mid-scene JSON restore and safe handoffs");
    for gang in [false, true] {
        let mut r = Realm::load(a, &map)?;
        r.saved.pending = None;
        r.saved.intro = true;
        r.saved.bishop_done = true;
        let mut w = World::from_bsp(&map)?;
        let mut p = Player::new(vec3(-2496., 128., -128.));
        for name in [
            "rook_guard1_attacked_thread",
            "red_knight3_thread",
            "red_pawns3_thread",
        ] {
            r.event(name);
        }
        if gang {
            r.event("white_pawn_bullied_thread");
        }
        let mut stats = Stats::for_level("wchess1", None);
        let mut story = Story::load(a, "wchess1");
        let hit = |id| Hit {
            id,
            damage: 1000.,
            kind: DamageKind::Other,
            knockback: Vec3::ZERO,
        };
        for _ in 0..1200 {
            r.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            r.fight(&mut crate::level::Combat {
                dt: FIXED_DT,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
        }
        let rook = r.actor("rook_guard1").unwrap().id();
        r.cast_hit(hit(rook));
        ensure!(
            r.actor("rook_guard1").unwrap().piece.health >= 180.,
            "Rook minhealth failed"
        );
        let ids: Vec<_> = r
            .saved
            .cast
            .iter()
            .filter(|c| c.counted)
            .map(|c| c.id())
            .collect();
        let threshold = if gang { 8 } else { 4 };
        let chosen: Vec<_> = ids
            .into_iter()
            .filter(|id| r.saved.cast.iter().any(|c| c.id() == *id && c.piece.active))
            .collect();
        ensure!(
            chosen.len() == threshold,
            "Incorrect counted activation roster"
        );
        for (n, id) in chosen.iter().enumerate() {
            r.cast_hit(hit(*id));
            r.fight(&mut crate::level::Combat {
                dt: FIXED_DT,
                world: &w,
                player: &mut p,
                stats: &mut stats,
                story: &mut story,
                notarget: true,
                summon: None,
                threatens: &|_| false,
            });
            ensure!(
                r.saved.knight_gate == (n + 1 == threshold),
                "Gate threshold {threshold}, death {}",
                n + 1
            );
        }
        r.event("white_pawn_bullied_thread");
        ensure!(r.saved.knight_gate, "Late gang closed gate");
        roundtrip(a, &map, &r)?;
    }
    println!(
        "PASS actual damage/death callbacks: four/eight gate thresholds, late latch and rook floor"
    );
    for skip in [false, true] {
        let mut r = Realm::load(a, &map)?;
        r.saved.pending = Some(scene::Kind::Water);
        let mut w = World::from_bsp(&map)?;
        let mut p = Player::new(r.data.point("wchess1_start1").translation);
        while r.saved.age < 12.65 {
            if skip {
                r.skip_scene(&map, &mut w, &mut p)?;
            }
            r.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            if r.saved.age < 12.65 {
                ensure!(!r.water_ready(), "Early water exit on skip={skip}");
            }
        }
        settle(&mut r, &map, &mut w, &mut p, 0.12)?;
        ensure!(r.water_ready(), "Late water raise on skip={skip}");
    }
    for (piece, square) in [(Piece::Bishop, 4), (Piece::Knight, 10)] {
        let mut r = Realm::load(a, &map)?;
        r.saved.pending = None;
        r.saved.intro = true;
        if piece == Piece::Knight {
            r.saved.bishop_done = true;
            r.saved.knight_gate = true;
        }
        let mut p = Player::new(r.data.square(piece, square));
        let mut w = World::from_bsp(&map)?;
        for _ in 0..120 {
            r.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        }
        ensure!(
            r.saved.damage == 1000.,
            "Crusher square {square} did not damage"
        );
    }
    println!("PASS exact water timing on watch/skip and real crusher volumes");
    let mut binding = Interactions::load(&map)?;
    binding.set_entry(a, &map, "wchess1", Some("wchess1_start1"))?;
    {
        let r = owner(&mut binding)?;
        r.saved.pending = None;
        r.saved.intro = true;
        r.saved.bishop_done = true;
    }
    binding.triggers(FIXED_DT, vec3(-384., 1700., 256.), vec3(-384., 1632., 256.));
    ensure!(
        owner(&mut binding)?
            .saved
            .fired
            .contains("red_pawns2_thread"),
        "Placed-NPC fallback swallowed owned trigger"
    );
    let shot = binding
        .switch_target("white_pawn_bullied_thread")
        .context("Bully shot volume missing")?;
    binding.shoot(Hit {
        id: shot.id,
        damage: 1.,
        kind: DamageKind::Knife,
        knockback: Vec3::ZERO,
    });
    let before = owner(&mut binding)?.snapshot();
    binding.triggers(
        FIXED_DT,
        vec3(2020., 1664., -224.),
        vec3(2080., 1664., -224.),
    );
    ensure!(
        before == owner(&mut binding)?.snapshot(),
        "Shot and touch duplicated the gang"
    );
    ensure!(
        binding.switch_target("white_pawn_bullied_thread").is_none(),
        "Consumed shot volume remains live"
    );
    println!("PASS real trigger ownership and gang shot/touch one-shot binding");

    // Bad-square fixtures still use the real mover, contact and recovery paths.
    for (piece, from_name, d) in [
        (Piece::Bishop, "bishop_3_5_thread", 1),
        (Piece::Knight, "knight_3_2_thread", 0),
        (Piece::Knight, "knight_2_11_thread", 1),
    ] {
        let mut r = Realm::load(a, &map)?;
        r.saved.pending = None;
        r.saved.intro = true;
        if piece == Piece::Knight {
            r.saved.bishop_done = true;
            r.saved.knight_gate = true;
        }
        let node = piece
            .nodes()
            .iter()
            .position(|n| n.name == from_name)
            .context("Bad-square predecessor missing")?;
        r.saved.board = Some(Board {
            piece,
            node,
            moving: None,
        });
        let mut p = Player::new(
            r.data.square(piece, piece.nodes()[node].square) + Vec3::Z * crate::collision::SKIN,
        );
        let mut w = World::from_bsp(&map)?;
        r.rebuild(&map)?;
        w.set_dynamic(r.colliders());
        r.direction(d, &p);
        for _ in 0..1200 {
            r.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            if r.saved.board.is_none() {
                break;
            }
        }
        // A crusher may release immediately; a drop releases at the hazard square.
        if r.saved.board.is_some() {
            settle(&mut r, &map, &mut w, &mut p, 2.)?;
            ensure!(
                r.saved.damage == 1000.,
                "Crusher did not inflict lethal damage"
            );
            let mut dead = Stats::for_level("wchess1", None);
            dead.damage(1000.);
            r.prepare_player(&mut dead, &mut p);
        }
        ensure!(
            r.saved.board.is_none() && r.saved.retry == Some(piece),
            "Bad square did not release {piece:?}"
        );
        roundtrip(a, &map, &r)?;
        let (feet, yaw) = r.recovery_entry((Vec3::ZERO, 0.));
        p = Player::new(feet);
        p.script_facing = yaw;
        for _ in 0..180 {
            r.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            if r.scripted() {
                break;
            }
            p.tick(
                &w,
                Controls {
                    wish: Vec2::Y,
                    run: true,
                    ..Default::default()
                },
            );
        }
        ensure!(
            r.scripted(),
            "Retry failed to restart {piece:?} at {feet:?}"
        );
    }
    let mut undef = Realm::load(a, &map)?;
    undef.saved.pending = None;
    undef.saved.bishop_done = true;
    undef.saved.knight_gate = true;
    let node = graph::KNIGHT
        .iter()
        .position(|n| n.name == "knight_25_26_thread")
        .unwrap();
    undef.saved.board = Some(Board {
        piece: Piece::Knight,
        node,
        moving: None,
    });
    let p = Player::new(undef.data.square(Piece::Knight, 26));
    let before = undef.snapshot();
    undef.direction(2, &p);
    ensure!(
        before == undef.snapshot(),
        "Undefined knight direction started a move"
    );
    println!(
        "PASS pit/crusher disguise release, saved retry and blocked undefined knight direction"
    );
    let mut duel = Realm::load(a, &map)?;
    duel.saved.pending = None;
    duel.saved.intro = true;
    duel.saved.bishop_done = true;
    duel.warp_actor("rook_guard1", "rook_guard1_dest2");
    duel.event("rook_guard1_attacked_thread");
    let mut w = World::from_bsp(&map)?;
    let mut p = Player::new(vec3(-2496., 128., -128.));
    let mut stats = Stats::for_level("wchess1", None);
    let mut story = Story::load(a, "wchess1");
    let mut fought = false;
    for _ in 0..24000 {
        duel.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        duel.fight(&mut crate::level::Combat {
            dt: FIXED_DT,
            world: &w,
            player: &mut p,
            stats: &mut stats,
            story: &mut story,
            notarget: true,
            summon: None,
            threatens: &|_| false,
        });
        let h = duel.actor("red_bishop2").unwrap().piece.health;
        fought |= duel.actor("rook_guard1").unwrap().piece.health < 200.;
        if h == 0. {
            break;
        }
    }
    ensure!(
        fought
            && duel.actor("red_bishop2").unwrap().piece.health == 0.
            && duel.actor("rook_guard1").unwrap().piece.health >= 180.,
        "Authored faction duel stalled"
    );
    println!("PASS scripted bishop damage to rook, health floor and enemy-loss death callback");
    let mut i = Interactions::load(&map)?;
    let old = i.snapshot();
    i.set_entry(a, &map, "wchess1", Some("wchess1_start1"))?;
    i.restore(&old, &map)?;
    let r = owner(&mut i)?;
    ensure!(
        r.scripted() && !r.water_ready() && !r.saved.knight_gate,
        "Legacy save skipped new gates"
    );
    let before = r.snapshot();
    let mut bad = before.clone();
    bad["board"] = serde_json::json!({"piece":"Bishop","node":10000,"moving":null});
    ensure!(
        r.restore(&bad, &map).is_err() && before == r.snapshot(),
        "Bad state changed live owner"
    );
    let mut old = Interactions::load(&map)?;
    for at in [vec3(-1856., 304., -128.), vec3(-384., 1632., 256.)] {
        old.triggers(FIXED_DT, at, at);
    }
    let mut legacy = serde_json::to_value(old.snapshot())?;
    for t in legacy["triggers"].as_array_mut().unwrap() {
        if [292, 938].contains(&t["id"].as_u64().unwrap_or(0)) {
            ensure!(t["fired"] == true, "Legacy contact fixture missed trigger");
            t["reported"] = false.into();
        }
    }
    let mut loaded = Interactions::load(&map)?;
    loaded.set_entry(a, &map, "wchess1", Some("wchess1_start1"))?;
    loaded.restore(&serde_json::from_value(legacy)?, &map)?;
    let migrated = serde_json::to_value(loaded.snapshot())?;
    for t in migrated["triggers"].as_array().unwrap() {
        if [292, 938].contains(&t["id"].as_u64().unwrap_or(0)) {
            ensure!(
                t["fired"] == false && t["reported"] == false,
                "Consumed legacy thread was not rearmed"
            );
        }
    }
    println!("PASS legacy entrance/consumed-trigger migration and atomic rejection");
    Ok(())
}

fn clock_hands(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut r = Realm::load(a, map)?;
    let hands: Vec<_> = r
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| matches!(o.name.as_str(), "hand_long" | "hand_short"))
        .map(|(i, o)| {
            let vertices: Vec<_> = map.models[o.model]
                .surfaces
                .clone()
                .flat_map(|s| {
                    let f = &map.surfaces[s];
                    map.vertices[f.first_vertex..f.first_vertex + f.vertex_count]
                        .iter()
                        .map(|v| v.position)
                })
                .collect();
            (i, vertices)
        })
        .collect();
    ensure!(hands.len() == 2, "Missing clock hands");
    // Sample a whole slow-hand turn, including every fast-hand quarter turn.
    // Every real mesh vertex must keep its depth and radius about the fixed axle.
    for sample in 0..=192 {
        r.saved.age = sample as f32 * 0.25;
        r.rebuild(map)?;
        for (i, vertices) in &hands {
            let o = &r.objects[*i];
            ensure!(
                o.pose.translation == o.base.translation && o.draw && !o.solid,
                "Clock axle or visibility changed: {}",
                o.name
            );
            for &v in vertices {
                let p = o.pose.point(v) - o.base.translation;
                ensure!(
                    (p.y - v.y).abs() < 0.001,
                    "Clock hand leaves its face: {} at {}s",
                    o.name,
                    r.saved.age
                );
                ensure!(
                    (vec2(p.x, p.z).length() - vec2(v.x, v.z).length()).abs() < 0.001,
                    "Clock hand drifts from its axle: {}",
                    o.name
                );
            }
        }
    }
    r.saved.age = 1.375;
    r.rebuild(map)?;
    let mut restored = roundtrip(a, map, &r)?;
    let mut p = Player::new(r.data.point("wchess1_start1").translation);
    let mut w = World::from_bsp(map)?;
    restored.advance(0., map, &mut w, &mut p, &[])?;
    for (i, _) in &hands {
        ensure!(
            r.objects[*i].pose.rotation == restored.objects[*i].pose.rotation
                && r.objects[*i].pose.translation == restored.objects[*i].pose.translation,
            "Pause/restore changed clock phase"
        );
    }
    println!("PASS clock hands: 193 real-mesh samples over 48s keep face depth and axle; pause/restore retain phase");
    Ok(())
}

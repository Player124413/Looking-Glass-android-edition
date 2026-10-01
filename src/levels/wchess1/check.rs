use super::*;
use crate::movement::FIXED_DT;
const BISHOP: &[usize] = &[0, 1, 1, 0, 0, 0, 1, 1, 2, 1, 2, 2, 3, 2, 2, 1, 1, 0, 1, 1];
const KNIGHT: &[usize] = &[3, 3, 2, 3, 3, 0, 0, 3, 1, 0, 1, 1, 1, 1];
fn wish(piece: Piece, d: usize) -> Vec2 {
    match piece {
        Piece::Bishop => {
            [vec2(-1., 1.), vec2(1., 1.), vec2(1., -1.), vec2(-1., -1.)][d].normalize()
        }
        Piece::Knight => [Vec2::Y, Vec2::X, -Vec2::Y, -Vec2::X][d],
    }
}
fn board_route(r: &mut crate::route::Route, piece: Piece, dirs: &[usize]) -> Result<()> {
    r.wait_for_cinematic()?;
    for (step, &d) in dirs.iter().enumerate() {
        let before = owner(&mut r.interactions)?
            .saved
            .board
            .as_ref()
            .context("Disguise missing")?
            .node;
        let mut moved = false;
        for _ in 0..1200 {
            r.tick(Controls {
                wish: if moved { Vec2::ZERO } else { wish(piece, d) },
                run: true,
                jump: true,
                ..Default::default()
            })?;
            let g = owner(&mut r.interactions)?;
            ensure!(
                r.stats.alive(),
                "{piece:?} died at move {} {:?}",
                step + 1,
                r.player.feet
            );
            let Some(b) = &g.saved.board else {
                break;
            };
            moved |= b.moving.is_some();
            if moved && b.moving.is_none() && b.node != before {
                break;
            }
        }
        let g = owner(&mut r.interactions)?;
        ensure!(
            g.saved
                .board
                .as_ref()
                .is_none_or(|b| b.node != before && b.moving.is_none()),
            "{piece:?} move {} failed at {:?}",
            step + 1,
            r.player.feet
        );
        println!("  {piece:?} move {} at {:?}", step + 1, r.player.feet);
    }
    ensure!(
        owner(&mut r.interactions)?.saved.board.is_none(),
        "Puzzle did not release disguise"
    );
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    tests::run(a)?;
    let map = Bsp::parse(&a.read("maps/wchess1.bsp")?)?;
    let r = Realm::load(a, &map)?;
    let mut restored = Realm::load(a, &map)?;
    restored.restore(&r.snapshot(), &map)?;
    ensure!(
        r.snapshot() == restored.snapshot(),
        "Pale Realm initial state differs"
    );
    println!(
        "Pale Realm loaded: {} actors, {} objects",
        r.saved.cast.len(),
        r.objects.len()
    );
    let mut entry = crate::route::Route::new(a, "wchess1", Some("wchess1_start1"))?;
    entry.wait_for_cinematic()?;
    ensure!(
        entry.stats.alive() && entry.world.body_clear(entry.player.feet),
        "Unsafe introduction"
    );
    println!("PASS intro watch {:?}", entry.player.feet);
    for (piece, dirs) in [(Piece::Bishop, BISHOP), (Piece::Knight, KNIGHT)] {
        // Explicit mechanics fixture, separate from the entrance-to-exit route below.
        let mut r = crate::route::Route::new(a, "wchess1", Some("wchess1_start1"))?;
        let g = owner(&mut r.interactions)?;
        g.saved.pending = None;
        g.saved.intro = true;
        if piece == Piece::Knight {
            g.saved.bishop_done = true;
            g.saved.knight_gate = true;
        }
        g.saved.board = Some(Board {
            piece,
            node: 0,
            moving: None,
        });
        let feet = g.data.square(piece, piece.nodes()[0].square) + Vec3::Z * 0.03125;
        g.rebuild(&r.map)?;
        r.interactions.sync(&mut r.world);
        r.player = Player::new(feet);
        board_route(&mut r, piece, dirs)?;
        ensure!(r.jumps == 0, "Disguised jump escaped");
        println!("PASS {piece:?} real touch-strip solution");
    }
    Ok(())
}
fn walk_to(r: &mut crate::route::Route, goal: Vec3) -> Result<()> {
    for t in 0..2400 {
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
            return Ok(());
        }
        let d = goal - r.player.feet;
        if d.length() < 24. && (r.player.grounded || r.player.swimming) {
            return Ok(());
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            swim: d.normalize_or_zero(),
            rise: if r.player.swimming && d.z > 30. {
                1.
            } else {
                0.
            },
            run: d.truncate().length() > 25.,
            jump: r.player.ledge.is_some()
                || (r.player.grounded && d.z > 24. && d.truncate().length() < 330.)
                || t == 240 || t == 480,
            ..Default::default()
        })?;
    }
    // Combat displacement can put the next straight leg behind a corner.
    // Recover with ordinary physics inputs instead of assuming its old start.
    r.navigate(goal)
}
pub(super) fn route(a: &mut Assets) -> Result<()> {
    route_inner(a, false)
}
pub(crate) fn drive(r: &mut crate::route::Route) -> Result<()> {
    let skip = r.skip_cinematics;
    r.tactics = true;
    r.conserve_will = false;
    r.ice_stream = true;
    if !owner(&mut r.interactions)?.saved.bishop_done {
        r.wait_for_cinematic()?;
        for p in [vec3(-2176., 128., -128.), vec3(-1856., 128., -128.)] {
            r.navigate(p)?;
        }
        r.navigate_until_scene(vec3(-1856., 304., -128.))?;
        board_route(r, Piece::Bishop, BISHOP)?;
        println!("ROUTE bishop end {:?}", r.player.feet);
    }
    if std::env::var_os("LOOKING_GLASS_WCHESS1_RESUME").is_none() {
        for p in GALLERIES {
            if *p == [-784., -384., 64.] || *p == [-752., -480., 256.] { continue; }
            println!("GALLERY {p:?} at {:?}, sanity {}", r.player.feet, r.stats.sanity());
            walk_to(r, Vec3::from_array(*p))?;
            if *p == [-736., -512., 64.] {
                // Wait clear of the cage, board its bottom and stay on the deck.
                r.walk(vec3(-720., -544., 64.), false)?;
                for _ in 0..1200 {
                    if owner(&mut r.interactions)?.saved.elevator.phase == 1 { break; }
                    r.tick(Controls::default())?;
                }
                ensure!(owner(&mut r.interactions)?.saved.elevator.phase == 1, "Lift did not lower");
                r.walk(vec3(-800., -384., 64.), false)?;
                for _ in 0..1200 {
                    if r.player.feet.z > 250. && r.player.grounded { break; }
                    r.tick(Controls::default())?;
                }
                ensure!(r.player.feet.z > 250., "Lift did not carry Alice");
                r.walk(vec3(-800., -256., 256.), false)?;
            }
            if *p == [-480., -288., 64.] {
                // Collect the difficulty-selected supply before the exposed gallery climb.
                for goal in [vec3(-192., -192., 32.), vec3(64., -288., 32.), vec3(64., -432., 64.)] { r.navigate(goal)?; }
                for goal in [vec3(64., -464., 64.), vec3(32., -496., 64.), vec3(0., -528., 64.), vec3(-64., -576., 64.), vec3(0., -528., 64.), vec3(32., -496., 64.), vec3(64., -464., 64.)] { r.walk(goal, false)?; }
                for goal in [vec3(64., -288., 32.), vec3(-192., -192., 32.), Vec3::from_array(*p)] { r.navigate(goal)?; }
            }
        }
        close_fight(r, 800.)?;
        walk_to(r, vec3(-1232., 960., 192.))?;
        close_fight(r, 800.)?;
        walk_to(r, vec3(-1408., 1088., 192.))?;
        close_fight(r, 700.)?;
        for p in UPPER {
            walk_to(r, Vec3::from_array(*p))?;
        }
        use_lever(r, "bell_lever")?;
    }
    if let Some(path) = std::env::var_os("LOOKING_GLASS_WCHESS1_PROBE") {
        let ops: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(path)?)?;
        for op in ops {
            println!("PROBE {op}");
            if op.is_array() {
                let p: [f32; 3] = serde_json::from_value(op)?;
                walk_to(r, Vec3::from_array(p))?;
            } else if let Some(t) = op.get("wait") {
                r.wait(t.as_f64().unwrap() as f32)?;
            } else if let Some(t) = op.get("clear") {
                r.clear(t.as_f64().unwrap() as f32)?;
            } else if let Some(t) = op.get("rush") {
                close_fight(r, t.as_f64().unwrap() as f32)?;
            } else if let Some(n) = op.get("use") {
                use_lever(r, n.as_str().unwrap())?;
            } else if op.get("knight").is_some() {
                board_route(r, Piece::Knight, KNIGHT)?;
            }
            std::fs::write(
                "C:/DEV/McGee/private/wchess1/probe-checkpoint.json",
                serde_json::to_vec(&r.checkpoint())?,
            )?;
            println!(
                "PROBE reached {:?} sanity {}",
                r.player.feet,
                r.stats.sanity()
            );
        }
    }
    if std::env::var_os("LOOKING_GLASS_WCHESS1_PROBE").is_some() {
        return Ok(());
    }
    eastern(r)?;
    ensure!(
        r.transition == Some(("wchess2".into(), Some("wchess2_start1".into()))),
        "Castling exit missing"
    );
    ensure!(
        r.teleports == 0 && r.stats.alive(),
        "Pale Realm route used a teleport or died"
    );
    println!("PASS Pale Realm full route skip={skip}: {} ticks, {} throws / {} cards / {} swings, sanity {}, teleports {}",r.ticks,r.shots,r.cards,r.swings,r.stats.sanity(),r.teleports);
    Ok(())
}
fn route_inner(a: &mut Assets, skip: bool) -> Result<()> {
    let mut r = if let Some(path) = std::env::var_os("LOOKING_GLASS_WCHESS1_RESUME") {
        crate::route::Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        crate::route::Route::new(a, "wchess1", Some("wchess1_start1"))?
    };
    r.skip_cinematics = skip;
    drive(&mut r)?;
    let carried = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "wchess2"
            && next.world.body_clear(next.player.feet)
            && next.stats.alive(),
        "Unsafe Castling arrival"
    );
    ensure!(
        serde_json::to_value(&next.stats)? == carried,
        "Castling changed carried state"
    );
    println!("PASS Castling loaded at {:?}", next.player.feet);
    Ok(())
}
fn eastern(r: &mut crate::route::Route) -> Result<()> {
    walk_to(r, vec3(416.00, 1216.00, 416.00))?;
    walk_to(r, vec3(512.00, 1184.00, 64.00))?;
    walk_to(r, vec3(768.00, 1184.00, 0.00))?;
    walk_to(r, vec3(960.00, 960.00, 0.00))?;
    walk_to(r, vec3(960.00, 320.00, 0.00))?;
    walk_to(r, vec3(1408.00, 128.00, 0.00))?;
    walk_to(r, vec3(1664.00, 128.00, 0.00))?;
    walk_to(r, vec3(1920.00, 352.00, -40.00))?;
    walk_to(r, vec3(1920.00, 448.00, -64.00))?;
    r.wait(3.0)?;
    r.clear(600.0)?;
    walk_to(r, vec3(2016.00, 704.00, -128.00))?;
    walk_to(r, vec3(2048.00, 800.00, -128.00))?;
    walk_to(r, vec3(2304.00, 800.00, -188.00))?;
    r.wait(1.0)?;
    r.clear(700.0)?;
    walk_to(r, vec3(3072.00, 800.00, -192.00))?;
    walk_to(r, vec3(3168.00, 960.00, -192.00))?;
    walk_to(r, vec3(3200.00, 1088.00, -188.00))?;
    walk_to(r, vec3(3424.00, 1152.00, -128.00))?;
    r.wait(1.0)?;
    r.clear(700.0)?;
    walk_to(r, vec3(3648.00, 1024.00, -128.00))?;
    r.wait(2.0)?;
    r.clear(750.0)?;
    walk_to(r, vec3(3456.00, 1600.00, -128.00))?;
    walk_to(r, vec3(3392.00, 1760.00, -128.00))?;
    for _ in 0..3600 {
        if r.interactions.scripted() {
            break;
        }
        r.tick(Controls::default())?;
    }
    if !owner(&mut r.interactions)?.saved.knight_gate {
        for a in &owner(&mut r.interactions)?.saved.cast {
            if a.counted {
                println!(
                    "GATE {} {} {:?} {:?} unseen {} idle {}",
                    a.name, a.piece.health, a.piece.feet, a.piece.phase, a.unseen, a.idle
                );
            }
        }
    }
    ensure!(
        owner(&mut r.interactions)?.saved.knight_gate,
        "Four-kill gate did not open"
    );
    board_route(r, Piece::Knight, KNIGHT)?;
    walk_to(r, vec3(3520.00, 2624.00, -128.00))?;
    walk_to(r, vec3(3648.00, 2720.00, -128.00))?;
    walk_to(r, vec3(3648.00, 3008.00, -64.00))?;
    walk_to(r, vec3(3712.00, 3072.00, 0.00))?;
    walk_to(r, vec3(3712.00, 2880.00, 0.00))?;
    walk_to(r, vec3(3840.00, 2784.00, 0.00))?;
    close_fight(r, 650.)?;
    // Let the last combat shove settle before choosing the upper or lower path.
    r.wait(1.)?;
    if r.player.feet.z < -24. {
        // A shove off the balcony requires its lower approach and stairs.
        for p in [
            vec3(3648., 2720., -128.),
            vec3(3648., 3008., -64.),
            vec3(3712., 3072., 0.),
            vec3(3712., 2880., 0.),
            vec3(3840., 2784., 0.),
        ] {
            walk_to(r, p)?;
        }
    }
    // Watch/skip timing can leave the last bishop fight on either side of
    // this corner. Follow the balcony before descending the eastern stairs.
    if r.player.feet.y > 2900. {
        walk_to(r, vec3(3712., 2880., 0.))?;
    }
    walk_to(r, vec3(3840., 2784., 0.))?;
    walk_to(r, vec3(3808.00, 2560.00, 0.00))?;
    walk_to(r, vec3(3840.00, 2464.00, 8.00))?;
    walk_to(r, vec3(4000.00, 2464.00, 64.00))?;
    walk_to(r, vec3(4000.00, 2304.00, 128.00))?;
    walk_to(r, vec3(3840.00, 2272.00, 192.00))?;
    walk_to(r, vec3(3744.00, 2304.00, 200.00))?;
    walk_to(r, vec3(3680.00, 2368.00, 256.00))?;
    r.clear(650.)?;
    walk_to(r, vec3(3424.00, 2368.00, 256.00))?;
    walk_to(r, vec3(3264.00, 2304.00, 256.00))?;
    walk_to(r, vec3(3232.00, 1952.00, 256.00))?;
    walk_to(r, vec3(3264.00, 1664.00, 256.00))?;
    walk_to(r, vec3(3136.00, 1536.00, 256.00))?;
    walk_to(r, vec3(2816.00, 1504.00, 256.00))?;
    r.clear(650.)?;
    walk_to(r, vec3(2592.00, 1440.00, 256.00))?;
    r.clear(650.)?;
    walk_to(r, vec3(2368., 1440., 256.))?;
    walk_to(r, vec3(2592., 1440., 256.))?;
    walk_to(r, vec3(2816., 1504., 256.))?;
    r.wait(30.)?;
    walk_to(r, vec3(2592., 1440., 256.))?;
    // The diagonal view clears the narrow doorway for the eight-unit Blade.
    walk_to(r, vec3(2480., 1392., 256.))?;
    // Close with the gallery bishop instead of waiting in its line of fire.
    close_fight(r, 650.)?;
    walk_to(r, vec3(2104.00, 1440.00, 256.00))?;
    use_lever(r, "water_lever")?;
    walk_to(r, vec3(2592.00, 1440.00, 256.00))?;
    walk_to(r, vec3(2816.00, 1504.00, 256.00))?;
    walk_to(r, vec3(3040.00, 1504.00, 256.00))?;
    walk_to(r, vec3(3136.00, 1504.00, 256.00))?;
    walk_to(r, vec3(3232.00, 1632.00, 256.00))?;
    walk_to(r, vec3(3232.00, 2304.00, 256.00))?;
    walk_to(r, vec3(3424.00, 2368.00, 256.00))?;
    walk_to(r, vec3(3680.00, 2368.00, 256.00))?;
    walk_to(r, vec3(3744.00, 2304.00, 200.00))?;
    walk_to(r, vec3(3840.00, 2272.00, 192.00))?;
    walk_to(r, vec3(4000.00, 2304.00, 128.00))?;
    walk_to(r, vec3(4000.00, 2464.00, 64.00))?;
    walk_to(r, vec3(3840.00, 2464.00, 8.00))?;
    walk_to(r, vec3(3808.00, 2560.00, 0.00))?;
    walk_to(r, vec3(3840.00, 2864.00, 0.00))?;
    walk_to(r, vec3(3712.00, 2880.00, 0.00))?;
    walk_to(r, vec3(3712.00, 3072.00, 0.00))?;
    walk_to(r, vec3(3648.00, 3008.00, -64.00))?;
    walk_to(r, vec3(3648.00, 2720.00, -128.00))?;
    walk_to(r, vec3(3264.00, 2624.00, -128.00))?;
    walk_to(r, vec3(3008.00, 2624.00, -328.00))?;
    walk_to(r, vec3(2496.00, 2624.00, -328.00))?;
    walk_to(r, vec3(2176.00, 2624.00, -256.00))?;
    walk_to(r, vec3(2048.00, 2624.00, -256.00))?;
    walk_to(r, vec3(1952.00, 2784.00, -256.00))?;
    walk_to(r, vec3(1952.00, 3072.00, -208.00))?;
    walk_to(r, vec3(1920.00, 3168.00, -192.00))?;
    walk_to(r, vec3(1792.00, 3328.00, -192.00))?;
    walk_to(r, vec3(1552.00, 3328.00, -192.00))?;
    walk_to(r, vec3(1312.00, 3264.00, -192.00))?;
    walk_to(r, vec3(1312.00, 2976.00, -256.00))?;
    walk_to(r, vec3(1280.00, 2896.00, -256.00))?;
    Ok(())
}
// A player-input melee approach for enclosed galleries. The generic ranged
// circling policy backs into their railings and cannot dodge the bishop beam.
fn close_fight(r: &mut crate::route::Route, radius: f32) -> Result<()> {
    // Keep the Blade in hand when closing: a brief knockback must not turn the
    // next melee swing into a throw followed by three seconds without a weapon.
    r.hold_blade(true);
    let home = r.player.feet;
    for _ in 0..10800 {
        let eye = r.player.eye();
        let targets = r.interactions.levels_targets();
        let g = owner(&mut r.interactions)?;
        let nearest = targets
            .iter()
            .filter(|t| g.hostile_target(t.id) && t.center.distance(home + PLAYER_CENTER) < radius)
            .filter(|t| {
                let tr = r.world.sweep(eye, t.center, Vec3::splat(1.));
                !tr.start_solid && tr.fraction >= 1.
            })
            .min_by(|x, y| {
                x.center
                    .distance_squared(eye)
                    .total_cmp(&y.center.distance_squared(eye))
            });
        let Some(target) = nearest else {
            r.aim_at = None;
            r.hold_blade(false);
            r.clear(radius)?;
            return Ok(());
        };
        let retreat = r.actions.selected == 0
            && r.actions.equipment().1 < 0.1;
        r.aim_at = Some(target.id);
        let d = (target.center - eye).truncate();
        r.tick(Controls {
            wish: if retreat {
                if d.length() < 180. {
                    -d.normalize_or_zero()
                } else {
                    Vec2::ZERO
                }
            } else if d.length() > 48. {
                d.normalize_or_zero()
            } else {
                Vec2::ZERO
            },
            run: true,
            ..Default::default()
        })?;
    }
    anyhow::bail!("Gallery fight did not finish at {:?}", r.player.feet)
}
fn use_lever(r: &mut crate::route::Route, n: &str) -> Result<()> {
    let at = owner(&mut r.interactions)?
        .data
        .point(n)
        .point(vec3(36., 0., 26.));
    let wish = (at - r.player.eye()).truncate().normalize_or_zero();
    ensure!(
        r.interactions
            .prompt(&r.world, r.player.eye(), wish.extend(0.))
            == Some("E: use lever"),
        "Lever out of reach {n} at {:?}",
        r.player.feet
    );
    r.tick(Controls {
        wish,
        use_pressed: true,
        ..Default::default()
    })?;
    let duration = owner(&mut r.interactions)?.data.lever_duration;
    r.wait(duration + 0.1)?;
    if n == "bell_lever" {
        r.wait(5.1)?;
    }
    r.wait_for_cinematic()?;
    println!("PASS real E {n}");
    Ok(())
}
pub(super) fn route_skip(a: &mut Assets) -> Result<()> {
    route_inner(a, true)
}
const GALLERIES: &[[f32; 3]] = &[
    [-704., 1408., -128.],
    [-704., 1280., -88.47],
    [-704., 1152., -24.47],
    [-640., 992., 0.],
    [-512., 800., 0.],
    [-512., 544., 0.],
    [-416., 384., 0.],
    [-416., 128., 32.],
    [-512., -32., 32.],
    [-576., -160., 64.],
    [-480., -288., 64.],
    [-608., -384., 64.],
    [-736., -512., 64.],
    [-784., -384., 64.],
    [-752., -480., 256.],
    [-752., -256., 256.],
    [-704., -192., 256.],
    [-704., -80., 256.],
    [-688., 256., 256.],
    [-800., 400., 256.],
    [-1088., 384., 224.],
    [-1232., 608., 192.],
    [-1232., 800., 192.],
];
const UPPER: &[[f32; 3]] = &[
    [-1248., 1056., 192.],
    [-1248., 1200., 192.],
    [-1344., 1248., 192.],
    [-1344., 1472., 224.],
    [-1344., 1728., 192.],
    [-1600., 1728., 192.],
    [-1744., 1760., 192.],
    [-1760., 1824., 192.],
    [-1600., 1728., 192.],
    [-1344., 1728., 192.],
    [-928., 1728., 192.],
    [-928., 1952., 192.],
    [-704., 1952., 192.],
    [-448., 1952., 192.],
    [-320., 1856., 200.],
    [-320., 1696., 256.],
    [-384., 1632., 256.],
    [-224., 1472., 256.],
    [224., 1472., 416.],
    [320., 1360., 416.],
    [320., 1240., 416.],
];
pub(super) fn survey(a: &mut Assets) -> Result<()> {
    use std::io::Write;
    let map = Bsp::parse(&a.read("maps/wchess1.bsp")?)?;
    let w = World::from_bsp(&map)?;
    let mut file = std::io::BufWriter::new(std::fs::File::create(
        "C:/DEV/McGee/private/wchess1/floors.csv",
    )?);
    writeln!(file, "x,y,z")?;
    let mut count = 0;
    for x in (-2816..=4352).step_by(32) {
        for y in (-896..=3648).step_by(32) {
            let mut heights = BTreeSet::new();
            for z in (-512..=1024).step_by(64) {
                let p = vec3(x as f32, y as f32, z as f32 + 0.1);
                if !w.body_clear(p) {
                    continue;
                }
                let t = w.body_trace(p, p - Vec3::Z * 128.);
                if !t.start_solid && t.fraction < 1. && t.normal.z > 0.65 {
                    heights.insert(((p.z - 128. * t.fraction) * 32.).round() as i32);
                }
            }
            for z in heights {
                writeln!(file, "{x},{y},{}", z as f32 / 32.)?;
                count += 1;
            }
        }
    }
    println!("Survey: {count} supported body positions");
    Ok(())
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "wchess1")?;
        let mut r = Realm::load(a, &scene.map)?;
        let mut art = art::Art::load(a, &r)?;
        for (i, time) in [0., 0.5, 1., 1.5, 2., 3., 4.].into_iter().enumerate() {
            r.saved.age = time;
            r.rebuild(&scene.map)?;
            let camera = r.data.camera("gate_camera1", time);
            capture(&mut scene, &mut art, &r, camera, &format!("clock-{i}")).await?;
        }
        for (name, kind, t) in [
            ("intro", scene::Kind::Intro, 12.),
            ("bishop", scene::Kind::Bishop, 5.),
            ("knight", scene::Kind::Knight, 5.),
            ("bell", scene::Kind::Bell, 5.),
            ("water", scene::Kind::Water, 10.),
        ] {
            r.saved.pending = Some(kind);
            r.saved.scene = None;
            r.saved.board = None;
            r.saved.age = 100.;
            let at = match kind {
                scene::Kind::Bishop => "alice_bishop_puzzle_dest1",
                scene::Kind::Knight => "alice_knight_puzzle_dest1",
                scene::Kind::Bell => "bell_lever",
                scene::Kind::Water => "water_lever",
                _ => "wchess1_start1",
            };
            let mut p = Player::new(r.data.point(at).translation);
            let mut w = World::from_bsp(&scene.map)?;
            for _ in 0..(t / FIXED_DT) as usize {
                r.advance(FIXED_DT, &scene.map, &mut w, &mut p, &[])?;
            }
            let c = r
                .scene_camera()
                .context("Scene ended before render sample")?;
            for frame in 0..3 {
                clear_background(BLACK);
                let view = Camera3D {
                    position: c.eye,
                    target: c.target,
                    up: c.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                };
                set_camera(&view);
                crate::render_fx::begin_view(&view, r.saved.age, &scene.atmosphere, false);
                scene.draw(c.eye, r.saved.age, false, false, &r.transforms());
                art.draw(&r, &scene.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(c.eye, r.saved.age, false, true, &r.transforms())
                });
                art.effects(&r, c.eye, &scene.atmosphere);
                crate::render_fx::finish();
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "C:/DEV/McGee/private/wchess1/{name}.png"
                    )))?;
                }
                next_frame().await;
            }
        }
        for (name, piece) in [
            ("bishop-board", Piece::Bishop),
            ("knight-board", Piece::Knight),
        ] {
            let mut r = Realm::load(a, &scene.map)?;
            r.saved.pending = None;
            r.saved.intro = true;
            r.saved.bishop_done = piece == Piece::Knight;
            r.saved.knight_gate = piece == Piece::Knight;
            r.saved.board = Some(Board {
                piece,
                node: 0,
                moving: None,
            });
            r.rebuild(&scene.map)?;
            let at = r.data.square(piece, piece.nodes()[0].square);
            r.player_pose = Transform {
                translation: at,
                rotation: Quat::IDENTITY,
            };
            let c = crate::cinematic::Camera::look(at + vec3(0., -60., 170.), at + Vec3::Z * 12.);
            capture(&mut scene, &mut art, &r, c, name).await?;
        }
        for raised in [false, true] {
            let mut r = Realm::load(a, &scene.map)?;
            r.saved.pending = None;
            r.saved.age = 20.;
            if raised {
                r.saved.water = Some(0.);
                r.saved.knight_done = true;
                r.saved.bishop_done = true;
                r.saved.knight_gate = true;
            }
            r.rebuild(&scene.map)?;
            let c =
                crate::cinematic::Camera::look(vec3(3072., 2720., 80.), vec3(2560., 2624., -350.));
            capture(
                &mut scene,
                &mut art,
                &r,
                c,
                if raised { "pool-raised" } else { "pool-low" },
            )
            .await?;
        }
        Ok(())
    })
}
async fn capture(
    scene: &mut crate::render::Scene,
    art: &mut art::Art,
    r: &Realm,
    c: crate::cinematic::Camera,
    name: &str,
) -> Result<()> {
    for frame in 0..3 {
        clear_background(BLACK);
        let view = Camera3D {
            position: c.eye,
            target: c.target,
            up: c.up,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        };
        set_camera(&view);
        crate::render_fx::begin_view(&view, r.saved.age, &scene.atmosphere, false);
        scene.draw(c.eye, r.saved.age, false, false, &r.transforms());
        art.draw(r, &scene.atmosphere, c.eye, false);
        crate::render::depth_read_only(|| {
            scene.draw(c.eye, r.saved.age, false, true, &r.transforms())
        });
        art.effects(r, c.eye, &scene.atmosphere);
        crate::render_fx::finish();
        set_default_camera();
        if frame == 2 {
            crate::viewer::save_capture(std::path::Path::new(&format!(
                "C:/DEV/McGee/private/wchess1/{name}.png"
            )))?;
        }
        next_frame().await;
    }
    Ok(())
}

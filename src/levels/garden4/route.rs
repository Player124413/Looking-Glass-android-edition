use super::*;
use crate::{movement::Controls, powerups::Difficulty, route::Route};
pub(super) fn watched(a: &mut Assets) -> Result<()> {
    walk(a, false)
}
pub(super) fn skipped(a: &mut Assets) -> Result<()> {
    walk(a, true)
}
fn g(r: &Route) -> &Garden {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref())
        .unwrap()
}
fn walk(a: &mut Assets, skip: bool) -> Result<()> {
    for difficulty in [Difficulty::Normal, Difficulty::Easy] {
        walk_one(a, skip, difficulty)?;
    }
    Ok(())
}
fn walk_one(a: &mut Assets, skip: bool, difficulty: Difficulty) -> Result<()> {
    let mut r = Route::enter(
        a,
        "garden4",
        Some("garden4_start1"),
        Stats::for_level("garden4", None),
        difficulty,
    )?;
    r.skip_cinematics = skip;
    drive(&mut r)?;
    let carried = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "centipede1"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == carried,
        "Bad Flora arrival/resources"
    );
    println!("PASS {difficulty:?} Icy Reception skip={skip}: authored route, Ice Wand and normal Flora arrival");
    Ok(())
}
pub(crate) fn drive(r: &mut Route) -> Result<()> {
    let skip = r.skip_cinematics;
    let difficulty = r.difficulty;
    r.stop_at_exit = true;
    let goals = if let Ok(path) = std::env::var("LOOKING_GLASS_GARDEN4_ROUTE") {
        serde_json::from_slice::<Vec<[f32; 3]>>(&std::fs::read(path)?)?
    } else {
        vec![
            [640., 976., -4120.],
            [512., 544., -4200.],
            [256., 80., -4290.],
            [-256., -128., -4160.],
            [-448., -512., -4280.],
            [-800., -1080., -4290.],
            [-1084., -1190., -4300.],
            [-800., -1080., -4290.],
            [-1024., -1010., -4400.],
            [-1418., -1066., -4300.],
            [-1704., -360., -4460.],
            [-2720., -64., -4600.],
            [-2720., -1000., -4660.],
            [-2840., -1456., -4880.],
            [-3700., -1400., -4800.],
            [-3700., -600., -5000.],
            [-3632., 16., -5090.],
            [-3000., 200., -5150.],
            [-2600., 1100., -5350.],
            [-2624., 1800., -5490.],
            [-2496., 2176., -5480.],
            [-2240., 2560., -5480.],
            [-1984., 2816., -5500.],
            [-1536., 2816., -5500.],
            [-1152., 2688., -5500.],
            [-976., 2764., -5230.],
            [-848., 3204., -4940.],
            [-704., 3520., -4940.],
            [-896., 3776., -4900.],
            [-768., 3904., -4800.],
            [-768., 4096., -4720.],
            [-768., 4352., -4650.],
            [-1024., 4544., -4700.],
            [-1632., 5120., -4800.],
            [-1508., 5300., -4730.],
        ]
    };
    for xyz in goals {
        let guess = Vec3::from_array(xyz);
        if matches!(xyz[0] as i32, -976 | -848) && xyz[1] >= 2700. && xyz[1] <= 3250. {
            ride(r, guess)?;
            continue;
        }
        let goal = [0., 64., 128., 256., -64.]
            .into_iter()
            .find_map(|z| {
                r.world.actor_footing(
                    guess + Vec3::Z * z,
                    crate::collision::PLAYER_CENTER,
                    crate::collision::PLAYER_HALF,
                    768.,
                )
            })
            .with_context(|| format!("No route support {guess:?}"))?;
        println!("ICE GOAL {goal:?} from {:?}", r.player.feet);
        r.navigate_until_scene(goal)?;
        if g(&r).marble_time().is_some() {
            r.wait_for_cinematic()?;
            println!(
                "Marble handoff {:?}, lead {}",
                r.player.feet,
                r.player.feet.distance(g(&r).course().rocks[6].position)
            );
        }
        if g(&r).saved.scene.is_some() {
            break;
        }
    }
    ensure!(
        r.stats.copies(4) == 1 && r.stats.collected.contains("garden4:42"),
        "Route missed Ice Wand"
    );
    ensure!(
        g(&r).course().altar.is_some()
            && g(&r).course().marble_done
            && g(&r).course().wall.is_some()
            && g(&r).course().fog,
        "Route missed required cave sequence"
    );
    ensure!(
        g(&r).course().crushed[0].is_some() == (difficulty != Difficulty::Easy)
            && g(&r).course().crushed[1].is_some() == (difficulty != Difficulty::Easy),
        "Route used wrong difficulty floors: {:?}",
        g(&r).course().crushed
    );
    ensure!(g(&r).saved.scene.is_some(), "Route missed Caterpillar");
    let mut skipped = false;
    for _ in 0..120 * 300 {
        if skip && !skipped {
            skipped =
                r.interactions
                    .skip_cinematic(&r.map, &mut r.world, &mut r.player, &mut r.story)?;
        }
        r.tick(Controls::default())?;
        if g(&r).ready() {
            break;
        }
    }
    ensure!(g(&r).ready(), "Portal not ready");
    for _ in 0..300 {
        r.tick(Controls {
            wish: Vec2::X,
            ..Default::default()
        })?;
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(EXIT.destination()) && r.stats.alive(),
        "No ordinary portal exit"
    );
    Ok(())
}

fn ride(r: &mut Route, goal: Vec3) -> Result<()> {
    let mut touched = false;
    for tick in 0..120 * 24 {
        let delta = goal.truncate() - r.player.feet.truncate();
        let steering = (delta * 3. - r.player.velocity.truncate() * 0.65).clamp_length_max(1.);
        r.tick(Controls {
            wish: steering,
            run: true,
            jump: tick == 0,
            ..Default::default()
        })?;
        touched |= r.player.steam;
        if tick % 120 == 0 {
            println!(
                "VENT {goal:?} at {:?} rising {} steam {}",
                r.player.feet, r.player.velocity.z, r.player.steam
            );
        }
        ensure!(r.stats.alive(), "Vent route died");
        if touched && r.player.feet.z >= goal.z && delta.length() < 80. {
            println!("PASS authored vent rise {:?}", r.player.feet);
            return Ok(());
        }
    }
    anyhow::bail!("Vent ride failed toward {goal:?} at {:?}", r.player.feet)
}

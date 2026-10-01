//! Continuous player-input traversal. Calibration changes goals, never game state.
use super::*;
use crate::{inventory::Stats, movement::Controls, powerups::Difficulty, route::Route};

pub(super) fn watched(a: &mut Assets) -> Result<()> {
    check(a, false)
}
pub(super) fn skipped(a: &mut Assets) -> Result<()> {
    check(a, true)
}
fn g(r: &Route) -> &Garden {
    r.interactions
        .levels
        .iter()
        .find_map(|s| s.ctl.downcast_ref())
        .unwrap()
}
fn check(a: &mut Assets, skip: bool) -> Result<()> {
    for paced in [false, true] {
        let (r, tape) = walk(a, skip, paced)?;
        tape.verify(a, &r, skip, paced)?;
        let carried = serde_json::to_value(&r.stats)?;
        let next = r.depart(a, true)?;
        ensure!(
            next.level().map == "garden4"
                && next.world.body_clear(next.player.feet)
                && serde_json::to_value(&next.stats)? == carried,
            "Bad Icy Reception arrival"
        );
        println!("PASS Rolling Stones skip={skip} paced={paced}: continuous live chase and saves into Icy Reception");
    }
    Ok(())
}
fn walk(a: &mut Assets, skip: bool, paced: bool) -> Result<(Route, Tape)> {
    let mut r = Route::enter(
        a,
        "garden3",
        Some("garden3_start1"),
        Stats::for_level("garden3", None),
        Difficulty::Normal,
    )?;
    let tape = drive_inner(&mut r, skip, paced)?;
    Ok((r, tape))
}

pub(crate) fn drive(r: &mut Route) -> Result<()> {
    drive_inner(r, r.skip_cinematics, false).map(|_| ())
}
fn drive_inner(r: &mut Route, skip: bool, paced: bool) -> Result<Tape> {
    r.stop_at_exit = true;
    if skip {
        r.interactions
            .skip_cinematic(&r.map, &mut r.world, &mut r.player, &mut r.story)?;
    }
    r.wait_for_cinematic()?;
    r.quiet();
    let mut tape = Tape::default();
    println!(
        "CHASE handoff {:?}, rock {:?}",
        r.player.feet,
        g(&r).saved.rock.position
    );
    let goals: Vec<[f32; 3]> = if let Ok(path) = std::env::var("LOOKING_GLASS_GARDEN3_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        vec![
            [-1488.0, 2152.0, -380.0],
            [-1144.0, 2104.0, -492.0],
            [-880.0, 1896.0, -584.0],
            [-832.0, 1552.0, -692.0],
            [-920.0, 1216.0, -784.0],
            [-1136.0, 1032.0, -860.0],
            [-1432.0, 960.0, -932.0],
            [-1760.0, 1016.0, -1004.0],
            [-2040.0, 1192.0, -1072.0],
            [-2216.0, 1480.0, -1156.0],
            [-2248.0, 1824.0, -1232.0],
            [-2192.0, 2040.0, -1276.0],
            [-1952.0, 2328.0, -1348.0],
            [-1640.0, 2488.0, -1436.0],
            [-1288.0, 2528.0, -1488.0],
            [-1040.0, 2664.0, -1520.0],
            [-736.0, 2936.0, -1512.0],
            [-480.0, 3064.0, -1480.0],
            [-312.0, 3092.0, -1448.0],
            [-184.0, 2920.0, -1496.0],
            [104.0, 2656.0, -1584.0],
            [376.0, 2520.0, -1592.0],
            [632.0, 2504.0, -1600.0],
            [920.0, 2496.0, -1532.0],
            // Approach the launch pad toward its eastern side, then cross the
            // cap itself. The older replay cut through its formerly non-solid rim.
            [1200.0, 2376.0, -1690.0],
            [1630.0, 2512.0, -1690.0],
            [1760.0, 2720.0, -1690.0],
            [1810.0, 2780.0, -1800.0],
            [2424.0, 2500.0, -1780.0],
            [2832.0, 2432.0, -1720.0],
            [3064.0, 2368.0, -1650.0],
            [3112.0, 1928.0, -1850.0],
            [3112.0, 1664.0, -2000.0],
            [3104.0, 1392.0, -2140.0],
            [3104.0, 1056.0, -2210.0],
            [3200.0, 784.0, -2210.0],
            [3192.0, 512.0, -2256.0],
            [2864.0, 256.0, -2104.0],
            [2816.0, 0.0, -2024.0],
            [2816.0, -256.0, -2008.0],
            [2840.0, -656.0, -2088.0],
            [2968.0, -904.0, -2160.0],
            [3184.0, -1048.0, -2232.0],
            [3432.0, -1128.0, -2280.0],
            [3752.0, -1040.0, -2360.0],
            [4048.0, -824.0, -2448.0],
            [4160.0, -520.0, -2584.0],
            [4128.0, -128.0, -2712.0],
            [3936.0, 24.0, -2840.0],
            [3688.0, 144.0, -2976.0],
            [3328.0, 184.0, -3032.0],
            [3200.0, 300.0, -2980.0],
            [2700.0, 330.0, -2980.0],
            [2200.0, 340.0, -2980.0],
            [1680.0, 367.0, -2980.0],
            [1300.0, 400.0, -2980.0],
            [1250.0, 900.0, -2980.0],
            [1100.0, 1300.0, -2980.0],
            [600.0, 1300.0, -2980.0],
        ]
    };
    for (k, xyz) in goals.into_iter().enumerate() {
        let goal = Vec3::from_array(xyz);
        println!(
            "CHASE goal {k} {goal:?}, at {:?}, age {:.2}, marble node {}",
            r.player.feet,
            g(&r).saved.age,
            g(&r).saved.rock.node
        );
        steer(
            r,
            &mut tape,
            goal,
            !(paced && (k < 24 || (29..51).contains(&k))),
        )?;
        if k == 3 {
            tape.mark("gates", &r);
        }
        if k == 16 {
            tape.mark("canyon", &r);
        }
        if r.transition.is_some() {
            break;
        }
    }
    for _ in 0..120 * 8 {
        if r.transition.is_some() {
            break;
        }
        tape.tick(r, Controls::default())?;
    }
    ensure!(
        r.transition == Some(("garden4".into(), Some("garden4_start1".into()))) && r.stats.alive(),
        "Chase did not enter Icy Reception at {:?}",
        r.player.feet
    );
    println!(
        "CHASE complete age {:.3}, sanity {}, marble node {}, ice {:?}",
        g(&r).saved.age,
        r.stats.sanity(),
        g(&r).saved.rock.node,
        g(&r).saved.ice
    );
    Ok(tape)
}

fn steer(r: &mut Route, tape: &mut Tape, goal: Vec3, run: bool) -> Result<()> {
    let mut last = r.player.feet;
    let pad = r
        .world
        .traversal
        .pushes
        .iter()
        .find(|p| p.origin.truncate().distance(goal.truncate()) < 160.)
        .map(|p| p.id);
    for tick in 0..120 * 16 {
        let touching_pad = pad.is_some_and(|id| {
            r.world.traversal.pushes.iter().any(|p| {
                p.id == id
                    && p.volume.touches(
                        r.player.feet + crate::collision::PLAYER_CENTER,
                        r.player.feet + crate::collision::PLAYER_CENTER + r.player.velocity / 120.,
                        crate::collision::PLAYER_HALF,
                    )
            })
        });
        let delta = goal.truncate() - r.player.feet.truncate();
        if delta.length() < 60. && r.player.feet.z < goal.z + 180. {
            return Ok(());
        }
        let jump = tick % 90 == 89 && r.player.feet.distance(last) < 25.;
        if tick % 90 == 89 {
            last = r.player.feet;
        }
        tape.tick(
            r,
            Controls {
                wish: delta.normalize_or_zero(),
                run,
                jump,
                ..Default::default()
            },
        )?;
        if touching_pad && r.player.velocity.z > 100. {
            return Ok(());
        }
        ensure!(
            r.stats.alive(),
            "Chase died at {:?}, rock {:?}, damage {}",
            r.player.feet,
            g(r).saved.rock.position,
            r.damage
        );
        if r.transition.is_some() {
            return Ok(());
        }
        if tick % 120 == 119 {
            println!(
                "  CHASE {:?} velocity {:?} sanity {} rock node {}",
                r.player.feet,
                r.player.velocity,
                r.stats.sanity(),
                g(r).saved.rock.node
            );
        }
    }
    anyhow::bail!("Chase stuck toward {goal:?} at {:?}", r.player.feet)
}

#[derive(Default)]
struct Tape {
    input: Vec<Controls>,
    saves: Vec<(&'static str, usize, crate::route::Checkpoint)>,
    hashes: Vec<(usize, String)>,
    pads: u8,
}
impl Tape {
    fn mark(&mut self, name: &'static str, r: &Route) {
        if !self.saves.iter().any(|s| s.0 == name) {
            println!(
                "SAVE {name}: tick {} age {:.3}, Alice {:?}, marble node {}",
                self.input.len(),
                g(r).saved.age,
                r.player.feet,
                g(r).saved.rock.node
            );
            self.saves.push((name, self.input.len(), r.checkpoint()));
        }
    }
    fn tick(&mut self, r: &mut Route, input: Controls) -> Result<()> {
        r.tick(input)?;
        self.input.push(input);
        if r.player.feet.x > 1050.
            && r.player.feet.x < 1450.
            && r.player.feet.y > 2300.
            && r.player.velocity.z > 100.
        {
            self.pads |= 1;
            self.mark("airborne", r);
        }
        if (1750. ..2100.).contains(&r.player.feet.x)
            && r.player.feet.y > 2600.
            && r.player.feet.z > -1900.
            && r.player.velocity.z > 350.
        {
            self.pads |= 2;
        }
        if g(r).saved.pillar.is_some_and(|t| g(r).saved.age - t > 0.5) {
            self.mark("pillar", r);
        }
        if g(r).saved.ice.is_some_and(|t| g(r).saved.age - t > 1.2) {
            self.mark("ice", r);
        }
        if g(r).saved.end.is_some_and(|t| g(r).saved.age - t > 0.65) && r.transition.is_none() {
            self.mark("ending", r);
        }
        if self.input.len() % 120 == 0 {
            self.hashes.push((self.input.len(), r.state_hash()?));
        }
        Ok(())
    }
    fn verify(&self, a: &mut Assets, end: &Route, skip: bool, paced: bool) -> Result<()> {
        ensure!(self.pads == 3, "Route missed an authored mushroom launch");
        let snapshot = serde_json::to_value(end.interactions.snapshot())?;
        // The shared changelevel trigger reports a transition rather than setting
        // its legacy fired bit; walk() checks its exact destination separately.
        for id in [41, 48, 47, 73, 16] {
            ensure!(
                snapshot["triggers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|t| t["id"] == id && t["fired"] == true),
                "Route missed trigger {id}"
            );
        }
        for name in ["gates", "canyon", "airborne", "pillar", "ending"] {
            ensure!(
                self.saves.iter().any(|s| s.0 == name),
                "Route missed {name}"
            );
        }
        if paced {
            ensure!(
                self.saves.iter().any(|s| s.0 == "ice"),
                "Paced route missed live ice collapse"
            );
            let ice = &self.saves.iter().find(|s| s.0 == "ice").unwrap().2;
            ensure!(
                ice.player.feet.x > 1920.
                    && ice.player.feet.x < 3552.
                    && ice.player.feet.z > -3280.,
                "Ice save was taken after leaving the collapsing floor"
            );
        }
        ensure!(
            end.damage < 100. && end.teleports == 0,
            "Route bypassed or was struck by the marble"
        );
        std::fs::create_dir_all("private/garden3-work")?;
        for (name, tick, cp) in &self.saves {
            let path = format!(
                "private/garden3-work/{}-{}-{name}.json",
                if skip { "skipped" } else { "watched" },
                if paced { "paced" } else { "run" }
            );
            std::fs::write(&path, serde_json::to_vec(cp)?)?;
            let loaded = serde_json::from_slice(&std::fs::read(&path)?)?;
            let mut replay = Route::resume(a, &loaded)?;
            replay.stop_at_exit = true;
            replay.quiet();
            ensure!(
                serde_json::to_value(replay.checkpoint())? == serde_json::to_value(cp)?,
                "Save changed {name} before resuming"
            );
            for (n, input) in self.input.iter().enumerate().skip(*tick) {
                replay.tick(*input)?;
                if let Some((_, expected)) = self.hashes.iter().find(|h| h.0 == n + 1) {
                    ensure!(
                        &replay.state_hash()? == expected,
                        "Save {name} diverged after input {}",
                        n + 1
                    );
                }
            }
            ensure!(
                replay.state()? == end.state()?
                    && replay.transition == end.transition
                    && replay.stats.alive(),
                "Save {name} lost its exit"
            );
            println!(
                "PASS saved {name}: {} identical live-input ticks through garden4 exit",
                self.input.len() - tick
            );
        }
        if paced && !skip {
            for name in ["gates", "ice"] {
                let cp = &self.saves.iter().find(|s| s.0 == name).unwrap().2;
                let mut waiting = Route::resume(a, cp)?;
                let mut died = false;
                for _ in 0..120 * 20 {
                    if let Err(error) = waiting.tick(Controls::default()) {
                        if waiting.stats.alive() {
                            return Err(error);
                        }
                    }
                    if !waiting.stats.alive() {
                        died = true;
                        break;
                    }
                }
                ensure!(died, "Waiting at {name} evaded the live hazard");
                println!("PASS waiting at {name} is fatal with unmodified hazards");
            }
        }
        Ok(())
    }
}
pub(super) fn fixture(a: &mut Assets, name: &str) -> Result<crate::route::Checkpoint> {
    let (_, tape) = walk(a, false, true)?;
    tape.saves
        .into_iter()
        .find(|s| s.0 == name)
        .map(|s| s.2)
        .context("Chase did not reach requested save")
}

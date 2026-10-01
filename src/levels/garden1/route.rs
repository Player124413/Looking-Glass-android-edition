//! Ordinary-input course; the native cast supplies the same combat as the viewer.
use super::*;
use crate::{movement::Controls, route::Route};
#[derive(Deserialize)]
struct Step {
    pos: [f32; 3],
    #[serde(default)]
    mode: String,
    #[serde(default)]
    frames: usize,
}
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move { walk(a, false, false) })
}
pub(super) fn skipped(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move { walk(a, false, true) })
}
pub(super) fn probe(a: &mut Assets) -> Result<()> {
    walk(a, true, false)
}
fn walk(a: &mut Assets, probe: bool, skip: bool) -> Result<()> {
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_GARDEN1_FROM") {
        {
            let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
            if probe {
                saved["route"]["native_cast"] = serde_json::json!(false);
            }
            Route::resume(a, &serde_json::from_value(saved)?)?
        }
    } else {
        let mut r = Route::new(a, "garden1", Some("garden1_start1"))?;
        if !probe {
            r.enable_native_cast(a)?;
        }
        r
    };
    r.skip_cinematics = skip;
    drive(&mut r)?;
    ensure!(!probe, "Movement-only diagnostic is not campaign proof");
    let stats = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "garden2"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == stats,
        "Bad Garden2 arrival"
    );
    println!("PASS complete Normal Dry Landing into Garden2, alive, exact carried resources");
    Ok(())
}
pub(crate) fn drive(r: &mut Route) -> Result<()> {
    r.tactics = true;
    r.conserve_will = true;
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    let data = if let Ok(file) = std::env::var("LOOKING_GLASS_GARDEN1_ROUTE") {
        std::fs::read(file)?
    } else {
        include_bytes!("route_steps.json").to_vec()
    };
    let steps: Vec<Step> = serde_json::from_slice(&data)?;
    std::fs::create_dir_all("private/garden1-world")?;
    for (k, s) in steps.iter().enumerate() {
        let goal = Vec3::from_array(s.pos);
        println!(
            "GARDEN1 step{k} {} {goal:?} from {:?} sanity {}",
            s.mode,
            r.player.feet,
            r.stats.sanity()
        );
        let result = (|| -> Result<()> {
            match s.mode.as_str() {
                "cards" => r.conserve_will = goal.x == 0.,
                "nav" => r.navigate(goal)?,
                "hold" => {
                    for _ in 0..s.frames {
                        let mut wish =
                            ((goal - r.player.feet).truncate() * 4. / 320.).clamp_length_max(1.);
                        if wish.length_squared() < 0.000001 {
                            wish = vec2(0.001, 0.);
                        }
                        r.tick(Controls {
                            wish,
                            run: true,
                            ..Default::default()
                        })?;
                    }
                }
                "clear" => {
                    r.clear(goal.x)?;
                }
                "input" | "inputjump" | "release" => {
                    for n in 0..s.frames {
                        r.tick(Controls {
                            wish: goal.truncate(),
                            swim: goal,
                            rise: goal.z,
                            run: true,
                            jump: s.mode == "inputjump" && n == 0,
                            use_pressed: s.mode == "release" && n == 0,
                            ..Default::default()
                        })?;
                    }
                }
                "wait" => {
                    for _ in 0..s.frames {
                        r.tick(Controls::default())?;
                    }
                }
                "rope" => {
                    for n in 0..2400 {
                        let d = goal.z - r.player.feet.z;
                        if r.player.rope.is_some() && d.abs() < 8. {
                            break;
                        }
                        r.tick(Controls {
                            rise: d.signum(),
                            use_pressed: n == 0 && r.player.rope.is_none(),
                            ..Default::default()
                        })?;
                    }
                    ensure!(
                        r.player.rope.is_some() && (r.player.feet.z - goal.z).abs() < 14.,
                        "Rope climb failed {:?}",
                        r.player.feet
                    );
                }
                _ => steer(r, goal, &s.mode)?,
            }
            Ok(())
        })();
        let cp = serde_json::to_vec(&r.checkpoint())?;
        std::fs::write("private/garden1-world/last-checkpoint.json", &cp)?;
        std::fs::write(format!("private/garden1-world/route-{k}.json"), cp)?;
        result?;
        r.wait_for_cinematic()?;
        ensure!(r.stats.alive(), "Garden route died at {:?}", r.player.feet);
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("garden2".into(), Some("garden2_start1".into()))),
        "Dry Landing course incomplete at {:?}",
        r.player.feet
    );
    ensure!(r.teleports == 0, "Unexpected recovery teleport");
    ensure!(!r.stats.god, "Route used invulnerability");
    let o = owner(&mut r.interactions)?;
    ensure!(
        o.saved.arrived && o.saved.rabbit_done && o.saved.world.rabbit.is_some(),
        "Route missed the arrival, conversation or dead-tree contact"
    );
    println!(
        "Garden route: ticks {} sanity {} will {} shots {} cards {} damage {} teleports {} skip {}",
        r.ticks,
        r.stats.sanity(),
        r.stats.will(),
        r.shots,
        r.cards,
        r.damage,
        r.teleports,
        r.skip_cinematics
    );
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut last = r.player.feet;
    for n in 0..3600 {
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
        }
        if mode == "launch" && r.player.velocity.z > 500. {
            return Ok(());
        }
        let d = goal - r.player.feet;
        if mode == "grab" && r.player.rope.is_some() {
            return Ok(());
        }
        if d.truncate().length() < 20. && d.z.abs() < 52. && mode != "grab" {
            return Ok(());
        }
        let stuck = n % 120 == 119 && r.player.feet.distance(last) < 18.;
        if n % 120 == 119 {
            last = r.player.feet;
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            swim: d.normalize_or_zero(),
            rise: if d.z.abs() > 25. { d.z.signum() } else { 0. },
            run: true,
            jump: (mode == "jump" && n == 0) || (mode.is_empty() && stuck),
            use_pressed: mode == "grab" && n % 4 == 0 && r.player.rope.is_none(),
            ..Default::default()
        })?;
        if r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!("Failed {mode} to {goal:?} at {:?}", r.player.feet)
}

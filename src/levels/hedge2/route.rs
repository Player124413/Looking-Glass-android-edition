//! Test driver: all travel uses the production walk, jump, use and swim inputs.
use super::*;
use crate::{movement::Controls, route::Route};
#[path = "swim_path.rs"]
mod swim_path;
#[derive(Deserialize)]
struct Step {
    pos: [f32; 3],
    #[serde(default)]
    mode: String,
    #[serde(default)]
    frames: usize,
}
pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        for skip in [false, true] {
            if std::env::var("LOOKING_GLASS_HEDGE2_SKIP")
                .is_ok_and(|v| v != if skip { "1" } else { "0" })
            {
                continue;
            }
            walk(a, skip, true)?;
        }
        Ok(())
    })
}
pub(super) fn walk(a: &mut Assets, skip: bool, native: bool) -> Result<()> {
    std::fs::create_dir_all("private/hedge2-work")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_HEDGE2_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        Route::new(a, "hedge2", Some("hedge2_start1"))?
    };
    if native && r.native_cast.is_none() {
        r.enable_native_cast(a)?;
    }
    r.skip_cinematics=skip;
    drive(&mut r)?;
    let stats = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().entry.as_deref() == Some("tower2_start1")
            && next.difficulty == crate::powerups::Difficulty::Normal,
        "Wrong tower entrance or difficulty"
    );
    let spawn = Player::spawn(
        &next.world,
        interaction::spawn(&next.map, Some("tower2_start1")).0,
    )
    .context("Blocked tower arrival")?;
    ensure!(
        next.player.feet.distance(spawn.feet) < 0.01,
        "Changed tower arrival position"
    );
    ensure!(
        next.level().map == "tower2"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == stats,
        "Bad tower2 handoff"
    );
    println!("PASS hedge2 native={native} skip={skip}: both levers and swimming into tower2");
    Ok(())
}
fn checkpoint(r: &Route) -> Result<()> {
    std::fs::write(
        "private/hedge2-work/last-checkpoint.json",
        serde_json::to_vec(&r.checkpoint())?,
    )?;
    Ok(())
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut last = r.player.feet;
    let mut jumped = false;
    for n in 0..120 * 20 {
        let d = goal - r.player.feet;
        let swim = mode == "swim";
        if d.truncate().length() < if swim { 10. } else { 24. }
            && d.z.abs() < if swim { 10. } else { 64. }
        {
            return Ok(());
        }
        let jump = r.player.ledge.as_ref().is_some_and(|h| !h.pulling)
            || mode == "jump" && !jumped && r.player.grounded
            || !swim && n % 120 == 119 && r.player.feet.distance(last) < 8.;
        jumped |= jump;
        if n % 120 == 119 {
            last = r.player.feet;
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            swim: if swim {
                d.normalize_or_zero()
            } else {
                d.truncate().normalize_or_zero().extend(0.)
            },
            rise: if swim {
                0.
            } else if r.player.immersion.level > 0 {
                1.
            } else {
                0.
            },
            jump,
            run: true,
            ..Default::default()
        })?;
        if r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!("Blocked {} from {:?} towards {goal:?}", mode, r.player.feet)
}

pub(crate) fn drive(r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/hedge2-work")?;
    let native = r.native_cast.is_some();
    let skip = r.skip_cinematics;
    r.tactics = true;
    r.ice_stream = true;
    r.conserve_will = false;
    r.stop_at_exit = true;
    let steps: Vec<Step> = if let Ok(path) = std::env::var("LOOKING_GLASS_HEDGE2_ROUTE") {
        serde_json::from_slice(&std::fs::read(path)?)?
    } else {
        serde_json::from_str(include_str!("route_steps.json"))?
    };
    for (k, s) in steps.iter().enumerate() {
        let goal = Vec3::from_array(s.pos);
        println!(
            "HEDGE goal {k} {goal:?} {} at {:?} sanity {} air {}",
            s.mode,
            r.player.feet,
            r.stats.sanity(),
            r.player.breath.remaining()
        );
        checkpoint(&r)?;
        let result = match s.mode.as_str() {
            "pipe" => swim_path::swim(r, goal),
            "nav" => r.navigate(goal).or_else(|_| steer(r, goal, "jump")),
            "clear" => r.clear(goal.x),
            "wait" => r.wait(goal.x),
            "watch_ready" => r.wait(r.stats.powers.recharge + 0.1),
            "watch" => {
                // A skipped preceding scene can leave the carried Watch cooling down.
                r.wait(r.stats.powers.recharge + 0.1)?;
                r.use_watch()
            },
            "staff" => { r.heavy_weapon = Some(7); Ok(()) },
            "use" => {
                // The Watch freezes world interactions too; release E after it expires.
                if r.stats.powers.stopped > 0. {
                    r.wait(r.stats.powers.stopped + 0.02)?;
                }
                let id = goal.x as usize;
                let target = check::owner(r).levers[id].point(vec3(36., 0., 26.));
                let aim = (target - r.player.eye()).normalize_or_zero();
                ensure!(
                    r.interactions.levels[0]
                        .ctl
                        .downcast_ref::<Maze>()
                        .unwrap()
                        .pick(&r.world, r.player.eye(), aim)
                        == Some(id),
                    "Lever unreachable"
                );
                r.tick(Controls {
                    wish: aim.truncate().normalize_or_zero(),
                    use_pressed: true,
                    ..Default::default()
                })?;
                ensure!(check::owner(r).saved.used[id], "E did not pull lever");
                r.wait_for_cinematic()?;
                r.wait(2.2)
            }
            "input" | "inputjump" => {
                for n in 0..s.frames {
                    r.tick(Controls {
                        wish: goal.truncate(),
                        swim: goal,
                        rise: goal.z,
                        run: true,
                        jump: n == 0 && s.mode == "inputjump",
                        ..Default::default()
                    })?;
                }
                Ok(())
            }
            _ => steer(r, goal, &s.mode),
        };
        checkpoint(&r)?;
        result?;
        ensure!(r.stats.alive(), "Maze route died");
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("tower2".into(), Some("tower2_start1".into()))),
        "Route not at tower2: {:?}",
        r.player.feet
    );
    ensure!(
        check::owner(r).saved.finished == [true, true]
            && r.stats.turtle_air
            && r.teleports == 0
            && !r.stats.god
            && !r.stats.notarget
            && r.player.breath.hits == 0,
        "Bypassed hedge route"
    );
    let metrics = serde_json::json!({"native":native,"skip":skip,"sanity":r.stats.sanity(),"will":r.stats.will(),"ticks":r.ticks,"shots":r.shots,"cards":r.cards,"damage":r.damage,"teleports":r.teleports,"breath_hits":r.player.breath.hits});
    std::fs::write(
        format!("private/hedge2-work/route-{native}-{skip}.json"),
        serde_json::to_vec_pretty(&metrics)?,
    )?;
    Ok(())
}

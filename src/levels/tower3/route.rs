use super::*;
use crate::route::Route;
#[derive(Deserialize)]
struct Step {
    pos: [f32; 3],
    #[serde(default)]
    mode: String,
    #[serde(default)]
    frames: usize,
    #[serde(default)]
    save: Option<String>,
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    run(a, false)
}
pub(super) fn skip_check(a: &mut Assets) -> Result<()> {
    run(a, true)
}
fn run(a: &mut Assets, skip: bool) -> Result<()> {
    std::fs::create_dir_all("private/tower3-work")?;
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_TOWER3_FROM") {
        Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
    } else {
        Route::new(a, "tower3", Some("tower3_start1"))?
    };
    r.skip_cinematics=skip;
    drive(a,&mut r)?;
    let stats=serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "grounds1"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == stats
            && next.interactions.scripted(),
        "Royal Rage handoff failed"
    );
    println!(
        "PASS Machinations: live traversal, saved continuations and authored Royal Rage arrival"
    );
    Ok(())
}
fn phase(r: &mut Route, id: usize, spec: Vec3, hold: bool) -> Result<()> {
    let anchor = r.player.feet;
    for _ in 0..120 * 80 {
        let t = owner(&mut r.interactions)?;
        let mut k = t
            .objects
            .iter()
            .position(|o| o.id == id)
            .context("Bad phase part")?;
        while let Some(p) = t.objects[k].parent {
            k = p;
        }
        let g = t.roots.iter().position(|i| *i == k).unwrap();
        let phase = t.saved.clocks[g].rem_euclid(spec.y as f64);
        if (phase - spec.x as f64).abs() < FIXED_DT as f64 {
            return Ok(());
        }
        r.tick(Controls {
            wish: if hold {
                ((anchor - r.player.feet).truncate() * 4. - r.player.velocity.truncate() * 0.5)
                    .clamp_length_max(320.)
                    / 320.
            } else {
                Vec2::ZERO
            },
            run: true,
            ..Default::default()
        })?;
    }
    anyhow::bail!("Tower phase wait failed")
}
fn rise(r: &mut Route, goal: Vec3) -> Result<()> {
    for _ in 0..120 * 90 {
        if r.player.feet.z >= goal.z && r.player.grounded {
            return Ok(());
        }
        r.tick(Controls::default())?;
    }
    anyhow::bail!("Tower lift ride missed height at {:?}", r.player.feet)
}
fn board(r: &mut Route, id: usize, local: Vec3) -> Result<()> {
    let mut jumped = false;
    for _ in 0..120 * 15 {
        let t = owner(&mut r.interactions)?;
        let o = t
            .objects
            .iter()
            .find(|o| o.id == id)
            .context("Missing boarding part")?;
        if r.player.grounded
            && o.collider
                .as_ref()
                .is_some_and(|c| motion::support(c, &r.player))
        {
            return Ok(());
        }
        let goal = o.pose.translation + o.pose.rotation * local;
        let d = (goal - r.player.feet).truncate();
        let jump = !jumped && r.player.grounded;
        jumped |= jump;
        r.tick(Controls {
            wish: if d.length() < 60. {
                (d * 4. - r.player.velocity.truncate() * 0.5).clamp_length_max(320.) / 320.
            } else {
                d.normalize_or_zero()
            },
            run: true,
            jump,
            ..Default::default()
        })?;
    }
    anyhow::bail!("Failed to board Tower part {id} at {:?}", r.player.feet)
}
fn fall(r: &mut Route, goal: Vec3) -> Result<()> {
    let before = r.teleports;
    for _ in 0..120 * 20 {
        r.tick(Controls {
            wish: goal.truncate(),
            run: true,
            ..Default::default()
        })?;
        if r.teleports > before {
            ensure!(r.teleports == before + 1, "Multiple floor returns");
            println!("PASS Tower fall return {:?}", r.player.feet);
            return Ok(());
        }
    }
    anyhow::bail!("Tower fall missed floor return at {:?}", r.player.feet)
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut last = r.player.feet;
    let mut stalled = 0;
    let mut jumped = false;
    for n in 0..120 * 30 {
        let d = goal - r.player.feet;
        if d.truncate().length() < 22.
            && d.z.abs() < 64.
            && (mode == "air" || r.player.grounded || r.player.swimming)
        {
            return Ok(());
        }
        let jump = r.player.ledge.as_ref().is_some_and(|l| !l.pulling)
            || (r.player.grounded
                && (mode == "hop"
                    || matches!(mode, "jump" | "air" | "precise") && (!jumped || d.z > 24.)));
        jumped |= jump;
        let input = Controls {
            wish: if mode == "precise" || mode == "air" && d.truncate().length() < 30. {
                (d.truncate() * 3. - r.player.velocity.truncate() * 0.7).clamp_length_max(320.)
                    / 320.
            } else {
                d.truncate().normalize_or_zero()
            },
            swim: d.normalize_or_zero(),
            rise: d.z.signum(),
            run: true,
            jump,
            ..Default::default()
        };
        r.tick(input)?;
        if r.transition.is_some() {
            return Ok(());
        }
        if n % 120 == 119 {
            println!(
                "  TOWER3 {:?} velocity {:?} grounded {} sanity {}",
                r.player.feet,
                r.player.velocity,
                r.player.grounded,
                r.stats.sanity()
            );
            if r.player.feet.distance(last) < 1. {
                stalled += 1;
            } else {
                stalled = 0;
            }
            last = r.player.feet;
            ensure!(stalled < 12, "Tower stalled toward {goal:?}");
        }
    }
    anyhow::bail!(
        "Tower traversal timeout at {:?} toward {goal:?}",
        r.player.feet
    )
}
fn continuation(a: &mut Assets, r: &mut Route, name: &str) -> Result<()> {
    let selected_heavy = r.heavy_weapon;
    r.quiet();
    r.heavy_weapon = selected_heavy;
    r.tactics = true;
    let path = format!("private/tower3-work/route-{name}.json");
    std::fs::write(&path, serde_json::to_vec(&r.checkpoint())?)?;
    let cp = serde_json::from_slice(&std::fs::read(&path)?)?;
    let audit = r.audit.clone();
    let mut saved = Route::resume(a, &cp)?;
    saved.tactics = true;
    saved.heavy_weapon = selected_heavy;
    saved.stop_at_exit = true;
    ensure!(
        r.state()? == saved.state()?,
        "Tower save changed before continuation"
    );
    for tick in 0..120 {
        let live = r.tick(Controls::default());
        let loaded = saved.tick(Controls::default());
        ensure!(
            live.as_ref().err().map(ToString::to_string)
                == loaded.as_ref().err().map(ToString::to_string),
            "Tower saved continuation outcome diverged"
        );
        ensure!(
            r.state()? == saved.state()?,
            "Tower saved continuation diverged"
        );
        if let Err(error) = live {
            // A reload must reproduce an exposed player's death too. This idle
            // comparison does not require the actual route to wait under fire.
            ensure!(
                !r.stats.alive() && error.to_string().starts_with("Route died at"),
                "Tower continuation failed: {error}"
            );
            println!("TOWER3 continuation {name}: identical idle death at tick {tick}");
            break;
        }
    }
    *r = Route::resume(a, &cp)?;
    r.audit = audit;
    r.heavy_weapon = selected_heavy;
    r.tactics = true;
    r.stop_at_exit = true;
    println!("PASS Tower disk continuation {name}");
    Ok(())
}
fn probe(r: &Route, spec: &str) -> Result<()> {
    let values = spec
        .split(',')
        .map(str::parse::<f32>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    ensure!(
        values.len() == 6,
        "Probe expects xmin,xmax,ymin,ymax,zmax,spacing"
    );
    let mut points = vec![];
    let mut x = values[0];
    while x <= values[1] {
        let mut y = values[2];
        while y <= values[3] {
            let mut z = values[4];
            for _ in 0..32 {
                let from = vec3(x, y, z);
                let tr = r.world.sweep(from, vec3(x, y, -1000.), vec3(1., 1., 1.));
                if tr.start_solid {
                    z -= 32.;
                    continue;
                }
                if tr.fraction >= 1. {
                    break;
                }
                let p = from.lerp(vec3(x, y, -1000.), tr.fraction);
                points.push((p.to_array(), tr.normal.to_array(), r.world.body_clear(p)));
                z = p.z - 64.;
            }
            y += values[5];
        }
        x += values[5];
    }
    std::fs::write(
        "private/tower3-work/floors.json",
        serde_json::to_vec(&points)?,
    )?;
    println!("TOWER3 probe {} floor samples", points.len());
    Ok(())
}

pub(crate) fn drive(a: &mut Assets, r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/tower3-work")?;
    let skip=r.skip_cinematics;
    let initial = r.stats.clone();
    r.stop_at_exit = true;
    if skip && r.interactions.scripted() {
        r.interactions
            .skip_cinematic(&r.map, &mut r.world, &mut r.player, &mut r.story)?;
    }
    for _ in 0..120 * 60 {
        if !r.interactions.scripted() {
            break;
        }
        r.tick(Controls::default())?;
    }
    ensure!(
        !r.interactions.scripted() && r.story.has_seen(INTRO),
        "Tower intro incomplete"
    );
    if let Ok(spec) = std::env::var("LOOKING_GLASS_TOWER3_PROBE") {
        return probe(&r, &spec);
    }
    let text = std::env::var("LOOKING_GLASS_TOWER3_ROUTE")
        .ok()
        .map(std::fs::read_to_string)
        .transpose()?
        .unwrap_or_else(|| include_str!("route_steps.json").into());
    let steps: Vec<Step> = serde_json::from_str(&text)?;
    for (n, s) in steps.iter().enumerate() {
        let goal = Vec3::from_array(s.pos);
        println!(
            "TOWER3 goal {n} {} {goal:?} from {:?} room {} health {}",
            s.mode,
            r.player.feet,
            owner(&mut r.interactions)?.saved.room,
            r.stats.sanity()
        );
        let result = match s.mode.as_str() {
            "room" => {
                ensure!(
                    owner(&mut r.interactions)?.saved.room == s.frames as u8,
                    "Wrong Tower room checkpoint"
                );
                Ok(())
            }
            "nav" => r.navigate(goal),
            "board" => board(r, s.frames, goal),
            "phase" => phase(r, s.frames, goal, false),
            "holdphase" => phase(r, s.frames, goal, true),
            "rise" => rise(r, goal),
            "fall" => fall(r, goal),
            "wait" | "input" | "inputjump" => {
                let mut out = Ok(());
                for k in 0..s.frames {
                    out = r.tick(Controls {
                        wish: if s.mode == "wait" {
                            Vec2::ZERO
                        } else {
                            goal.truncate()
                        },
                        run: true,
                        jump: s.mode == "inputjump" && k == 0,
                        ..Default::default()
                    });
                    if out.is_err() {
                        break;
                    }
                }
                out
            }
            _ => steer(r, goal, &s.mode),
        };
        std::fs::write(
            "private/tower3-work/last-checkpoint.json",
            serde_json::to_vec(&r.checkpoint())?,
        )?;
        result?;
        if let Some(name) = &s.save {
            continuation(a, r, name)?;
        }
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        r.transition == Some(("grounds1".into(), Some("grounds1_start1".into()))),
        "Tower route has not reached Royal Rage at {:?}",
        r.player.feet
    );
    ensure!(
        !r.stats.god
            && !r.stats.notarget
            && r.stats.alive()
            && r.audit.lost_ticks == 0
            && r.audit.transitions == 1
            && r.teleports == 3,
        "Tower route bypassed live traversal"
    );
    let _stats = serde_json::to_value(&r.stats)?;
    let mut metrics = r.metrics();
    metrics.sanity_in = initial.sanity();
    metrics.will_in = initial.will();
    metrics.pickups = r
        .stats
        .collected
        .difference(&initial.collected)
        .filter(|k| !k.starts_with("drop:"))
        .count();
    metrics.loot = r
        .stats
        .collected
        .difference(&initial.collected)
        .filter(|k| k.starts_with("drop:"))
        .count();
    println!("TOWER3 completed {metrics}");
    std::fs::write(
        "private/tower3-work/route-result.json",
        serde_json::to_vec_pretty(
            &serde_json::json!({"skip":skip,"metrics":metrics,"audit":r.audit}),
        )?,
    )?;
    Ok(())
}

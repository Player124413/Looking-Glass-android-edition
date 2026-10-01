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
    #[serde(default)]
    thread: String,
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
    let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_FUNHOUSE_FROM") {
        {
            let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
            if probe {
                saved["route"]["native_cast"] = serde_json::json!(false);
            }
            Route::resume(a, &serde_json::from_value(saved)?)?
        }
    } else {
        let mut r = Route::new(a, "funhouse", Some("funhouse_start1"))?;
        if !probe {
            r.enable_native_cast(a)?;
        }
        r
    };
    r.skip_cinematics = skip || std::env::var_os("LOOKING_GLASS_FUNHOUSE_SKIP").is_some();
    drive_inner(&mut r, probe)?;
    let stats = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "hatter1"
            && next.world.body_clear(next.player.feet)
            && serde_json::to_value(&next.stats)? == stats,
        "Bad Hatter1 arrival"
    );
    println!("PASS complete Normal Mirror Image into Hatter1, alive, exact carried resources");
    Ok(())
}
fn shot_point(ctx: &crate::combat::Context<'_>, eye: Vec3, target: Target) -> Option<Vec3> {
    for z in [-0.75, -0.4, 0., 0.4, 0.75] {
        for y in [-0.7, 0., 0.7] {
            for x in [-0.7, 0., 0.7] {
                let point = target.center + target.half * vec3(x, y, z);
                if crate::combat::contact(ctx, eye, point, 1.).is_none() {
                    continue;
                }
                let mut p =
                    crate::weapons::Projectile::thrown_blade(eye, (point - eye).normalize());
                for _ in 0..120 {
                    let (hit, normal) = p.contact_step(1. / 120., ctx);
                    if hit.is_some_and(|h| h.id == target.id) {
                        return Some(point);
                    }
                    if normal.is_some() {
                        break;
                    }
                }
            }
        }
    }
    None
}
fn pendulum(r: &mut Route, name: &str) -> Result<()> {
    let bank = r.player.feet;
    let mut airborne = None;
    let mut runup = None;
    let mut flight = 0.;
    for n in 0..3600 {
        let o = r
            .interactions
            .levels
            .iter()
            .find_map(|l| l.ctl.downcast_ref::<Funhouse>())
            .unwrap();
        let object = o
            .objects
            .iter()
            .find(|o| o.name == name)
            .context("Unknown pendulum")?;
        let local = vec3(
            0.,
            0.,
            match name {
                "pend1" => -592.,
                "pend2" => -624.,
                _ => -560.,
            },
        );
        let point = |ahead: f32| {
            let (amp, phase) = if name == "pend2" {
                (12f32, 0.5)
            } else {
                (24f32, 0.)
            };
            let angle = (amp
                * (((o.saved.time + ahead) / 8. + phase) * std::f32::consts::TAU).sin())
            .to_radians();
            object.base.translation + Quat::from_rotation_x(angle) * local
        };
        let current = point(0.);
        let support = object.collider.trace(
            r.player.feet + PLAYER_CENTER,
            r.player.feet + PLAYER_CENTER - Vec3::Z * 3.,
            PLAYER_HALF,
        );
        if airborne.is_some()
            && r.player.grounded
            && support.fraction < 1.
            && (r.player.feet.x - current.x).abs() < 65.
        {
            println!(
                "PEND landed {name} at {:?} time {}",
                r.player.feet, o.saved.time
            );
            return Ok(());
        }
        let mut jump = false;
        let goal = if let Some(start) = airborne {
            point((flight - (n - start) as f32 / 120.).max(0.))
        } else {
            flight = (400.
                + (160000. - 1600. * (current.z - r.player.feet.z))
                    .max(0.)
                    .sqrt())
                / 800.;
            for _ in 0..3 {
                flight = (400.
                    + (160000. - 1600. * (point(flight).z - r.player.feet.z))
                        .max(0.)
                        .sqrt())
                    / 800.;
            }
            let target = point(flight + 0.1);
            let delta = target - r.player.feet;
            if runup.is_some()
                || (n > 6
                    && r.player.grounded
                    && delta.truncate().length() < flight * 285.
                    && delta.z < 70.)
            {
                let start = *runup.get_or_insert(n);
                if n >= start + 12 {
                    airborne = Some(n);
                    jump = true;
                    println!(
                        "PEND jump {name} from {:?} to {target:?} flight {flight} time {}",
                        r.player.feet, o.saved.time
                    );
                }
                target
            } else {
                // Wait on the current mover using its actual pose; no actor relocation.
                o.objects
                    .iter()
                    .find_map(|q| {
                        let z = match q.name.as_str() {
                            "pend1" => -592.,
                            "pend2" => -624.,
                            "pend3" => -560.,
                            _ => return None,
                        };
                        let p = q.pose.translation + q.pose.rotation * vec3(0., 0., z);
                        ((p - r.player.feet).length() < 120.).then_some(p + Vec3::X * 32.)
                    })
                    .unwrap_or(vec3(bank.x, target.y.clamp(-3750., -3450.), bank.z))
            }
        };
        let mut wish = ((goal - r.player.feet).truncate() * 8. / 320.).clamp_length_max(1.);
        if wish.length_squared() < 0.000001 {
            wish = vec2(0.001, 0.);
        }
        r.tick(Controls {
            wish,
            run: true,
            jump: jump || r.player.ledge.as_ref().is_some_and(|h| !h.pulling),
            ..Default::default()
        })?;
    }
    anyhow::bail!("Cannot cross {name} at {:?}", r.player.feet)
}
fn steer(r: &mut Route, goal: Vec3, mode: &str) -> Result<()> {
    let mut best = (f32::MAX, 0);
    for n in 0..3600 {
        if r.interactions.scripted() {
            r.wait_for_cinematic()?;
        }
        if mode == "launch" && r.player.velocity.z > 500. {
            return Ok(());
        }
        let d = goal - r.player.feet;
        if d.truncate().length() < best.0 - 2. {
            best = (d.truncate().length(), n);
        } else if mode.is_empty() && n - best.1 > 240 {
            return r.navigate(goal);
        }
        if mode == "grab" && r.player.rope.is_some() {
            return Ok(());
        }
        if d.truncate().length() < 20. && d.z.abs() < 52. && mode != "grab" && r.player.grounded {
            return Ok(());
        }
        if mode == "jump"
            && n % 12 == 0
            && std::env::var_os("LOOKING_GLASS_FUNHOUSE_JUMP_TRACE").is_some()
        {
            println!(
                "JUMP {n} feet {:?} velocity {:?} ground {} normal {:?} health {}",
                r.player.feet,
                r.player.velocity,
                r.player.grounded,
                r.player.ground_normal,
                r.stats.sanity()
            );
        }
        r.tick(Controls {
            wish: d.truncate().normalize_or_zero(),
            swim: d.normalize_or_zero(),
            rise: if d.z.abs() > 25. { d.z.signum() } else { 0. },
            run: true,
            jump: (mode == "jump" && n == 0) || r.player.ledge.as_ref().is_some_and(|h| !h.pulling),
            use_pressed: mode == "grab" && n % 4 == 0 && r.player.rope.is_none(),
            ..Default::default()
        })?;
        if r.transition.is_some() {
            return Ok(());
        }
    }
    anyhow::bail!("Failed {mode} to {goal:?} at {:?}", r.player.feet)
}

fn drive_inner(r: &mut Route, probe: bool) -> Result<()> {
    r.tactics = true;
    r.conserve_will = true;
    r.stop_at_exit = true;
    r.wait_for_cinematic()?;
    let data = if let Ok(file) = std::env::var("LOOKING_GLASS_FUNHOUSE_ROUTE") {
        std::fs::read(file)?
    } else {
        include_bytes!("route_steps.json").to_vec()
    };
    let steps: Vec<Step> = serde_json::from_slice(&data)?;
    std::fs::create_dir_all("private/funhouse-work")?;
    for (k, s) in steps.iter().enumerate() {
        let goal = Vec3::from_array(s.pos);
        println!(
            "FUNHOUSE step{k} {} {goal:?} from {:?} sanity {}",
            s.mode,
            r.player.feet,
            r.stats.sanity()
        );
        let result = (|| -> Result<()> {
            match s.mode.as_str() {
                "scan" => {
                    let target = r
                        .interactions
                        .switch_target(&s.thread)
                        .context("Missing clock target")?;
                    let ctx = crate::combat::Context {
                        world: &r.world,
                        targets: &[target],
                    };
                    let wall = r.world.sweep(r.player.eye(), target.center, Vec3::ONE);
                    println!(
                        "SCAN target {target:?} eye {:?} wall {wall:?}",
                        r.player.eye()
                    );
                    let mut hits = vec![];
                    for x in -25..=25 {
                        for y in -25..=25 {
                            let feet = goal + vec3(x as f32 * 16., y as f32 * 16., 0.);
                            if !r.world.body_clear(feet + Vec3::Z * 0.1) {
                                continue;
                            }
                            if let Some(aim) = shot_point(
                                &ctx,
                                feet + Vec3::Z * crate::movement::EYE_HEIGHT,
                                target,
                            ) {
                                hits.push((feet.to_array(), aim.to_array()));
                            }
                        }
                    }
                    std::fs::write(
                        "private/funhouse-work/vantages.json",
                        serde_json::to_vec_pretty(&hits)?,
                    )?;
                    println!("SCAN {} clear clock rays", hits.len());
                    anyhow::bail!("Vantage diagnostic only");
                }
                "shot" => {
                    // Aim at the clock even if a distant foe remains awake. The live
                    // cast keeps attacking; do not pathfind off the turning gear to chase it.
                    let tactics = r.tactics;
                    r.tactics = false;
                    let result = r.shoot_switch(&s.thread);
                    r.tactics = tactics;
                    result?;
                }
                "shotjump" => {
                    let target = r
                        .interactions
                        .switch_target(&s.thread)
                        .context("Missing clock target")?;
                    for n in 0..2400 {
                        r.aim_at = Some(target.id);
                        r.tick(Controls {
                            jump: r.player.grounded && n % 120 == 0,
                            ..Default::default()
                        })?;
                        if r.interactions.switch_target(&s.thread).is_none() {
                            break;
                        }
                    }
                    r.aim_at = None;
                    ensure!(
                        r.interactions.switch_target(&s.thread).is_none(),
                        "Cannot jump-shoot {} from {:?}",
                        s.thread,
                        r.player.feet
                    );
                }
                "scene" => r.wait_for_cinematic()?,
                "pendulum" => pendulum(r, &s.thread)?,
                "cards" => r.conserve_will = goal.x == 0.,
                "ice" => r.ice_stream = goal.x != 0.,
                "nav" => r.navigate(goal)?,
                "xy" => r.walk_xy(goal.truncate())?,
                "hold" | "holdjump" => {
                    // The smaller Easy encounter needs only the brief bank pause.
                    // Repeated jumps here expose Alice to unnecessary knockback.
                    let easy_bank = s.mode == "holdjump"
                        && r.stats.difficulty == crate::powerups::Difficulty::Easy;
                    let frames = if easy_bank { 180 } else { s.frames };
                    for n in 0..frames {
                        let mut wish =
                            ((goal - r.player.feet).truncate() * 4. / 320.).clamp_length_max(1.);
                        if wish.length_squared() < 0.000001 {
                            wish = vec2(0.001, 0.);
                        }
                        r.tick(Controls {
                            wish,
                            run: true,
                            jump: s.mode == "holdjump" && !easy_bank
                                && n % 120 == 0 && r.player.grounded,
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
        std::fs::write("private/funhouse-work/last-checkpoint.json", &cp)?;
        std::fs::write(
            format!(
                "private/funhouse-work/{}-{k}.json",
                if probe { "probe" } else { "native" }
            ),
            cp,
        )?;
        result?;
        r.wait_for_cinematic()?;
        ensure!(
            r.stats.alive(),
            "Funhouse route died at {:?}",
            r.player.feet
        );
        if r.transition.is_some() {
            break;
        }
    }
    ensure!(
        !probe,
        "Movement-only diagnostic finished; not a combat route proof"
    );
    ensure!(
        r.transition == Some(("hatter1".into(), Some("hatter1_start1".into()))),
        "Mirror Image course incomplete at {:?}",
        r.player.feet
    );
    ensure!(
        r.teleports == 0,
        "Unexpected recovery teleport on required tube route"
    );
    ensure!(!r.stats.god, "Route used invulnerability");
    let o = r
        .interactions
        .levels
        .iter()
        .find_map(|l| l.ctl.downcast_ref::<Funhouse>())
        .unwrap();
    ensure!(
        o.saved.gas
            && o.saved.cells == 255
            && o.saved.bosses.iter().all(|b| b.health <= 0.)
            && o.saved.exit.committed,
        "Missing Funhouse route outcomes"
    );
    println!(
        "Funhouse route: ticks {} sanity {} will {} shots {} cards {} damage {} teleports {} skip {}",
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

pub(crate) fn drive(r: &mut Route) -> Result<()> { std::fs::create_dir_all("private/funhouse-work")?; drive_inner(r, false) }

use super::*;
use crate::{
    movement::{Controls, FIXED_DT},
    route::Route,
};
fn owner(i: &mut Interactions) -> Result<&mut Temple> {
    i.levels
        .iter_mut()
        .find_map(|s| s.ctl.downcast_mut::<Temple>())
        .context("Temple owner missing")
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/utemple.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(a, &map, "utemple", None)?;
    let mut w = World::from_bsp(&map)?;
    i.sync(&mut w);
    let mut p = Player::spawn(&w, crate::interaction::spawn(&map, None).0)
        .context("Temple spawn blocked")?;
    let mut stats = Stats::for_level("utemple", None);
    i.prepare_player(&mut stats, &mut p);
    let at = owner(&mut i)?
        .data
        .guide
        .nodes
        .iter()
        .find(|n| n.0 == "tp105")
        .unwrap()
        .2;
    ensure!(
        crate::water::Immersion::sample(&w, at).level == 3,
        "Brush water is dry"
    );
    for _ in 0..3600 {
        i.advance_school(FIXED_DT, &map, &mut w, &mut p)?;
        i.prepare_player(&mut stats, &mut p);
        p.tick(&w, Controls::default());
    }
    ensure!(p.breath.hits == 0, "Waiting player drowned");
    let t = owner(&mut i)?;
    println!(
        "Temple: {} solid/visual objects; water at tp105; breath bounds {:?}; guide path {}",
        t.motion.objects.len(),
        t.data.bubble_bounds,
        t.data.guide.nodes.len()
    );
    t.event("startturtle");
    for _ in 0..24000 {
        t.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        if t.saved.open {
            break;
        }
    }
    ensure!(t.saved.open, "Guide failed to open exit");
    println!(
        "Guide opened wall at {:?}; events {:?}",
        t.saved.guide.map(|s| t.saved.clock - s),
        t.saved.events
    );
    let saved = t.snapshot();
    t.advance(0., &map, &mut w, &mut p, &[])?;
    ensure!(saved == t.snapshot(), "Paused temple advanced");
    t.restore(&saved, &map)?;
    super::verify::check(a, &map, &saved)?;
    println!("PASS temple wet arrival, wait protection, guide node events and pause/restore");
    Ok(())
}
pub(super) fn route(a: &mut Assets, skip: bool) -> Result<()> {
    let mut r = Route::new(a, "utemple", None)?;
    r.skip_cinematics = skip;
    drive(a, &mut r)?;
    let stats = serde_json::to_value(&r.stats)?;
    let next = r.depart(a, true)?;
    ensure!(
        next.level().map == "garden1" && next.level().entry.as_deref() == Some("garden1_start1"),
        "Wrong temple destination"
    );
    ensure!(
        next.world.body_clear(next.player.feet) && next.stats.alive(),
        "Unsafe garden arrival"
    );
    ensure!(
        stats == serde_json::to_value(&next.stats)?,
        "Temple exit changed carried resources"
    );
    println!(
        "PASS strict garden arrival {:?}, resources {}/{}",
        next.player.feet,
        next.stats.sanity(),
        next.stats.will()
    );
    Ok(())
}
pub(crate) fn drive(a: &mut Assets, r: &mut Route) -> Result<()> {
    let skip = r.skip_cinematics;
    r.stop_at_exit = true;
    let mut last = 0.;
    let mut goal = r.player.feet;
    for tick in 0..120 * 260 {
        let t = owner(&mut r.interactions)?;
        let pose = t.turtle_pose(t.saved.clock);
        let mut target = if t.saved.guide.is_none() {
            vec3(-88., -5980., 1330.)
        } else if t.saved.open {
            vec3(-2144., 3352., 488.)
        } else {
            t.data.shell_pose(pose, t.saved.clock as f32).translation - PLAYER_CENTER
        };
        if tick % 12 == 0 {
            let mut hazards = t.motion.ahead(&r.map, &t.saved, 1.)?;
            hazards.extend(t.motion.ahead(&r.map, &t.saved, 0.5)?);
            hazards.extend(
                r.world
                    .traversal
                    .pushes
                    .iter()
                    .filter(|p| p.enabled)
                    .map(|p| p.volume.clone()),
            );
            goal = super::swim_check::waypoint(&r.world, r.player.feet, target, &hazards);
        }
        target = goal;
        let offset = target - r.player.feet;
        let input = Controls {
            swim: offset.normalize_or_zero() * (offset.length() / 30.).min(1.),
            run: true,
            ..Default::default()
        };
        if r.interactions.scripted() && skip {
            r.interactions
                .skip_cinematic(&r.map, &mut r.world, &mut r.player, &mut r.story)?;
        }
        r.tick(input)?;
        if [1440, 4080, 13200, 20160].contains(&tick) {
            let before = r.checkpoint();
            let mut restored = Route::resume(a, &before)?;
            ensure!(
                serde_json::to_value(&r.player)? == serde_json::to_value(&restored.player)?,
                "Restored swimmer differs"
            );
            ensure!(
                serde_json::to_value(&r.stats)? == serde_json::to_value(&restored.stats)?,
                "Restored resources differ"
            );
            ensure!(
                owner(&mut r.interactions)?.snapshot()
                    == owner(&mut restored.interactions)?.snapshot(),
                "Restored temple clock differs"
            );
            *r = restored;
            r.stop_at_exit = true;
        }
        let t = owner(&mut r.interactions)?;
        if tick % 600 == 0 {
            println!(
                "t {:.1} node {} feet {:?} gap {:.1} air {:.2} contacts {} health {}",
                tick as f32 / 120.,
                t.saved.node,
                r.player.feet,
                offset.length(),
                r.player.breath.remaining(),
                t.saved.contacts,
                r.stats.sanity()
            );
        }
        ensure!(
            r.stats.alive(),
            "Temple route died at {:?}, node {}, air {}, gap {}",
            r.player.feet,
            t.saved.node,
            r.player.breath.remaining(),
            offset.length()
        );
        ensure!(
            r.world.body_clear(r.player.feet),
            "Temple route embedded at {:?}",
            r.player.feet
        );
        if r.transition.is_some() {
            last = t.saved.clock;
            break;
        }
    }
    ensure!(
        r.transition.as_ref().is_some_and(|e| EXIT.matches(e)),
        "Temple route did not reach exit at {:?}",
        r.player.feet
    );
    ensure!(
        r.teleports == 0 && r.player.breath.hits == 0,
        "Temple route bypassed traversal or drowned"
    );
    println!(
        "PASS temple normal swimming route, skip={skip}, time={last}, shell={}, contacts={}",
        r.stats.turtle_air,
        owner(&mut r.interactions)?.saved.contacts
    );
    Ok(())
}
pub fn stage(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
) -> Result<Story> {
    let elapsed = match case {
        "utemple-waiting" => 0.,
        "utemple-mid-guide" => 12.,
        "utemple-mid-collapse" => 34.,
        "utemple-oyster-closed" => 42.,
        "utemple-brush-water" => 110.,
        "utemple-exit-scene" | "utemple-exit-done" => 166.,
        _ => anyhow::bail!("Unknown temple fixture"),
    };
    let t = owner(i)?;
    t.saved = Saved::default();
    t.saved.initialized = true;
    if elapsed > 0. {
        t.event("startturtle");
    }
    *p = Player::new(data::origin(&map.entities[17]));
    for _ in 0..(elapsed * 120.) as usize {
        t.advance(FIXED_DT, map, w, p, &[])?;
    }
    t.saved.crush_damage = 0.;
    if case == "utemple-brush-water" {
        p.feet = t
            .data
            .guide
            .nodes
            .iter()
            .find(|n| n.0 == "tp105")
            .unwrap()
            .2;
    } else if elapsed > 0. {
        p.feet = t
            .data
            .shell_pose(t.turtle_pose(t.saved.clock), t.saved.clock as f32)
            .translation
            - PLAYER_CENTER;
    }
    if case.starts_with("utemple-exit-") {
        p.feet = vec3(-2100., 3352., 480.);
    }
    ensure!(
        w.body_clear(p.feet),
        "Temple fixture body blocked: {case} {:?}",
        p.feet
    );
    p.velocity = Vec3::ZERO;
    p.breath.shell = true;
    p.breath.submerged = 8.;
    p.tick(w, Controls::default());
    if case.starts_with("utemple-exit-") {
        let before = p.feet;
        p.feet.x = -2134.;
        i.triggers(0.01, before, p.feet);
        ensure!(i.scripted(), "Temple fixture did not contact gated exit");
        let end = if case.ends_with("done") { 7.1 } else { 3.5 };
        for _ in 0..(end * 120.) as usize {
            i.advance_school(FIXED_DT, map, w, p)?;
        }
    }
    Ok(Story::load(a, "utemple"))
}
pub(super) async fn render(a: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(a, "utemple")?;
    let mut i = Interactions::load(&scene.map)?;
    i.set_entry(a, &scene.map, "utemple", None)?;
    let mut art = art::Art::load(a, owner(&mut i)?)?;
    for case in [
        "waiting",
        "mid-guide",
        "mid-collapse",
        "oyster-closed",
        "brush-water",
        "exit-scene",
    ] {
        let mut p = Player::new(Vec3::ZERO);
        stage(
            &format!("utemple-{case}"),
            a,
            &scene.map,
            &mut i,
            &mut scene.world,
            &mut p,
        )?;
        let t = owner(&mut i)?;
        let guide = t.turtle_pose(t.saved.clock).translation;
        let camera = t.camera(&scene.world).unwrap_or(crate::cinematic::Camera {
            eye: guide + vec3(160., 180., 60.),
            target: guide,
            up: Vec3::Z,
        });
        scene.atmosphere.liquid =
            crate::water::Liquid::from_contents(scene.world.liquid_at(camera.eye)).tint();
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: camera.eye,
                target: camera.target,
                up: camera.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            scene.draw(
                camera.eye,
                t.saved.clock as f32,
                false,
                false,
                &t.transforms(),
            );
            art.draw(t, &scene.atmosphere, camera.eye, false);
            crate::render::depth_read_only(|| {
                scene.draw(
                    camera.eye,
                    t.saved.clock as f32,
                    false,
                    true,
                    &t.transforms(),
                );
                art.effects(t, camera.eye, &scene.atmosphere);
            });
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/temple-guide/{case}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    Ok(())
}

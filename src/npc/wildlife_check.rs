//! Real-asset checks for the remaining shared creature families.
use super::*;
use crate::wildlife::{Kind, Phase};
pub fn check(assets: &mut Assets) -> Result<()> {
    clockwork_departures(assets)?;
    companions::check(assets)?;
    let map = Bsp::parse(&assets.read("maps/hedge1.bsp")?)?;
    let template = placements(&map, "hedge1", None, false)
        .into_iter()
        .find(|s| s.model == "c_jabberspawn")
        .context("Jabberspawn fixture missing")?;
    let world = World::fixture(&[(vec3(-4000., -4000., -20.), vec3(4000., 4000., 0.))]);
    for kind in Kind::ALL {
        let data = Data::load(assets, kind.model(), &[])?;
        for clip in kind.clips() {
            ensure!(data.clips[*clip].duration() > 0., "Empty creature clip");
        }
        if kind.jabber() {
            ensure!(
                data.skeleton.bones.iter().any(|b| b.name == "tag_beam"),
                "Jabber beam tag missing"
            );
        }
        let mut spawn = template.clone();
        spawn.model = kind.model().into();
        spawn.origin = Vec3::Z * 0.1;
        spawn.yaw = 0.;
        spawn.name.clear();
        spawn.resident_spawn = None;
        let mut endings = Vec::new();
        for hz in [30, 60, 144] {
            let mut p = resident::Resident::new(&spawn, &map, "hedge1")?.unwrap();
            p.place(&world);
            let eye = vec3(90., 0., 48.);
            let mut out = combat::Feedback::default();
            for _ in 0..hz * 8 {
                p.update(1. / hz as f32, &world, eye, &data, &mut out);
            }
            ensure!(
                kind.rock() || out.damage + out.will_drain > 0.,
                "Creature never attacked: {kind:?}, at {:?}",
                p.position()
            );
            ensure!(
                !kind.rock() || out.damage + out.will_drain == 0.,
                "Walkrock attacked"
            );
            p.validate(&spawn)?;
            let saved = serde_json::to_value(&p)?;
            p.update(0., &world, eye, &data, &mut out);
            ensure!(saved == serde_json::to_value(&p)?, "Paused creature moved");
            let mut restored: resident::Resident = serde_json::from_value(saved)?;
            let mut a = combat::Feedback::default();
            let mut b = combat::Feedback::default();
            for _ in 0..240 {
                p.update(1. / 120., &world, eye, &data, &mut a);
                restored.update(1. / 120., &world, eye, &data, &mut b);
            }
            ensure!(
                serde_json::to_value(&p)? == serde_json::to_value(&restored)?
                    && a.damage == b.damage
                    && a.will_drain == b.will_drain
                    && a.impulse == b.impulse,
                "Creature restore diverged"
            );
            endings.push(p.position());
            if !kind.rock() {
                for hitkind in [combat::DamageKind::Knife, combat::DamageKind::Ice] {
                    let mut dead = p.clone();
                    dead.hit(combat::Hit {
                        id: 0,
                        damage: 1000.,
                        kind: hitkind,
                        knockback: Vec3::ZERO,
                    });
                    for _ in 0..1200 {
                        dead.update(1. / 120., &world, eye, &data, &mut out);
                    }
                    dead.validate(&spawn)?;
                    let resident::Body::Wildlife(g) = dead.body else {
                        unreachable!()
                    };
                    ensure!(
                        g.health == 0. && g.visual_scale(&data) == 0.,
                        "Creature death did not retire"
                    );
                }
            }
        }
        ensure!(
            endings.iter().all(|p| p.distance(endings[0]) < 0.1),
            "Creature frame-rate drift: {kind:?}"
        );
        println!(
            "PASS {kind:?}: source clips, 30/60/144 Hz behaviour, pause, continuation and death"
        );
    }
    for name in [
        "funhouse", "hatter1", "hedge1", "hedge2", "hedge3", "jlair1", "facade", "garden1",
        "garden2", "garden4", "potears1", "potears3", "wforest",
    ] {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let spawns = placements(&map, name, None, false);
            let mut count = 0;
            for s in spawns
                .iter()
                .filter(|s| Kind::from_model(&s.model).is_some())
            {
                let p = resident::Resident::new(s, &map, name)?.unwrap();
                p.validate(s)?;
                count += 1;
            }
            println!("PASS creature placement {name}/{difficulty:?}: {count}");
        }
    }
    Ok(())
}

fn clockwork_departures(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/hatter1.bsp")?)?;
    let world = World::actor_world(&map)?;
    for s in placements(&map, "hatter1", None, false).into_iter().filter(|s| matches!(s.name.as_str(), "spider_wall6" | "spider_wall7")) {
        let mut g = crate::wildlife::Creature::new(Kind::WallSpider, s.origin, s.yaw, s.scale, 1);
        let b = g.target(0);
        let f = vec3(s.yaw.cos(), s.yaw.sin(), 0.);
        let data = Data::load(assets, "c_spider_wall", &[])?;
        let mut out = combat::Feedback::default();
        for _ in 0..1200 { g.step(&world, b.center + f * 200., &data, &mut out); }
        ensure!(g.phase != Phase::Wall && g.feet.distance(s.origin) > 100., "Clockwork spider failed to leave wall");
        g.validate()?;
    }
    println!("PASS Clockwork spiders: real wall departure and pursuit with source animations");
    Ok(())
}

pub async fn native(assets: &mut Assets) -> Result<()> {
    attack_fx::render_check(assets).await?;
    let mut scene = crate::render::Scene::load(assets, "hedge1")?;
    let template = placements(&scene.map, "hedge1", None, false)
        .into_iter()
        .find(|s| s.model == "c_jabberspawn" && s.origin.z > 1700.)
        .context("Missing native creature fixture")?;
    let specs = texture::read_materials(assets)?;
    let beams = attack_fx::Art::load(assets, &specs)?;
    let material = crate::character::skin_material()?;
    std::fs::create_dir_all("private/wildlife/captures")?;
    for kind in Kind::ALL {
        let mut model = Model::load(assets, kind.model(), &[], &specs)?;
        let mut spawn = template.clone();
        spawn.model = kind.model().into();
        spawn.name.clear();
        spawn.resident_spawn = None;
        let mut p = resident::Resident::new(&spawn, &scene.map, "hedge1")?.unwrap();
        p.place(&scene.world);
        let b = p.target(0);
        let target = b.center;
        let eye = (0..64)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 64.;
                target
                    + vec3(
                        a.cos() * if kind == Kind::Phantom { 420. } else { 260. },
                        a.sin() * if kind == Kind::Phantom { 420. } else { 260. },
                        40.,
                    )
            })
            .find(|eye| {
                let t = scene.world.sweep(target, *eye, Vec3::splat(2.));
                !t.start_solid && t.fraction >= 1.
            })
            .context("No creature camera")?;
        for (pose, phase) in [
            ("idle", Phase::Idle),
            ("wake", Phase::Wake),
            ("attack", Phase::Melee),
            ("ranged", Phase::Ranged),
            ("pain", Phase::Pain),
            ("death", Phase::Dead),
            ("ice", Phase::Dead),
        ] {
            if kind.rock() && !matches!(phase, Phase::Idle | Phase::Wake) {
                continue;
            }
            let resident::Body::Wildlife(g) = &mut p.body else {
                unreachable!()
            };
            g.yaw = (eye.y - target.y).atan2(eye.x - target.x);
            g.set(phase);
            g.health = if phase == Phase::Dead {
                0.
            } else {
                kind.health()
            };
            g.frozen = pose == "ice";
            g.time = if pose == "ranged" { 0.45 } else { 0.4 };
            if kind.jabber() && phase == Phase::Ranged {
                g.beam = Some(target + (eye - target) * 0.7);
            }
            let mut art = wildlife_art::Art::new(&model);
            for frame in 0..3 {
                clear_background(BLACK);
                let camera = Camera3D {
                    position: eye,
                    target,
                    up: Vec3::Z,
                    fovy: 65_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                };
                set_camera(&camera);
                material.atmosphere(&scene.atmosphere, eye);
                crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
                scene.draw(eye, 0., false, false, &[]);
                ensure!(
                    scene.stats.triangles > 0,
                    "Creature camera sees no world: {:?} -> {:?}",
                    eye,
                    target
                );
                p.draw(&mut model, false, &material);
                let resident::Body::Wildlife(g) = &p.body else {
                    unreachable!()
                };
                art.draw(g, &model, eye, &scene.atmosphere, &beams);
                crate::render::depth_read_only(|| scene.draw(eye, 0., false, true, &[]));
                crate::render_fx::finish();
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/wildlife/captures/{}-{pose}.png",
                        kind.model()
                    )))?;
                }
                next_frame().await;
            }
        }
        println!("PASS native creature poses {}", kind.model());
    }
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    spawn: Spawn,
    cast: resident::Resident,
    expected: resident::Resident,
    damage: f32,
    will: f32,
    impulse: Vec3,
}
/// Run writer and reader as separate processes against one immutable executable.
pub fn fresh(assets: &mut Assets, read: bool) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/hedge1.bsp")?)?;
    let template = placements(&map, "hedge1", None, false)
        .into_iter()
        .find(|s| s.model == "c_jabberspawn")
        .context("Missing creature save fixture")?;
    let world = World::fixture(&[(vec3(-3000., -3000., -20.), vec3(3000., 3000., 0.))]);
    let eye = vec3(90., 0., 48.);
    std::fs::create_dir_all("private/wildlife/saves")?;
    for kind in Kind::ALL {
        let data = Data::load(assets, kind.model(), &[])?;
        for (name, phase) in [
            ("idle", Phase::Idle),
            ("wake", Phase::Wake),
            ("attack", Phase::Melee),
            ("ranged", Phase::Ranged),
            ("pain", Phase::Pain),
            ("death", Phase::Dead),
            ("ice", Phase::Dead),
            ("delay", Phase::Idle),
        ] {
            if kind.rock() && !matches!(phase, Phase::Idle | Phase::Wake) {
                continue;
            }
            let path = format!("private/wildlife/saves/{}-{name}.json", kind.model());
            if read {
                let saved: Saved = serde_json::from_slice(&std::fs::read(path)?)?;
                let mut p = saved.cast;
                p.validate(&saved.spawn)?;
                let seed = resident::Resident::new(&saved.spawn, &map, "hedge1")?.unwrap();
                p.matches(&seed)?;
                let mut out = combat::Feedback::default();
                for _ in 0..240 {
                    p.update(1. / 120., &world, eye, &data, &mut out);
                }
                p.validate(&saved.spawn)?;
                ensure!(
                    serde_json::to_value(p)? == serde_json::to_value(saved.expected)?
                        && out.damage == saved.damage
                        && out.will_drain == saved.will
                        && out.impulse == saved.impulse,
                    "Fresh creature continuation failed: {kind:?}/{name}"
                );
            } else {
                let mut spawn = template.clone();
                spawn.model = kind.model().into();
                spawn.origin = Vec3::Z * 0.1;
                spawn.yaw = 0.;
                spawn.name.clear();
                spawn.resident_spawn = None;
                let mut p = resident::Resident::new(&spawn, &map, "hedge1")?.unwrap();
                p.place(&world);
                let resident::Body::Wildlife(g) = &mut p.body else {
                    unreachable!()
                };
                g.set(phase);
                g.health = if phase == Phase::Dead {
                    0.
                } else {
                    kind.health()
                };
                g.frozen = name == "ice";
                if name == "delay" {
                    p.active = false;
                    p.delay = Some(0.7);
                }
                for _ in 0..51 {
                    p.update(
                        1. / 120.,
                        &world,
                        eye,
                        &data,
                        &mut combat::Feedback::default(),
                    );
                }
                p.validate(&spawn)?;
                let cast = p.clone();
                let mut out = combat::Feedback::default();
                for _ in 0..240 {
                    p.update(1. / 120., &world, eye, &data, &mut out);
                }
                std::fs::write(
                    path,
                    serde_json::to_vec(&Saved {
                        spawn,
                        cast,
                        expected: p,
                        damage: out.damage,
                        will: out.will_drain,
                        impulse: out.impulse,
                    })?,
                )?;
            }
            println!(
                "PASS creature fresh {} {kind:?}/{name}",
                if read { "read" } else { "write" }
            );
        }
    }
    Ok(())
}

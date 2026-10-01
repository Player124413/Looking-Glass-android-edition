//! Visible electrical coverage and protected pupil contact through actual weapon input.
use super::*;
use crate::{
    character::{Character, WeaponInput},
    movement::Player,
};

fn changed(a: &Image, b: &Image) -> usize {
    a.bytes
        .chunks_exact(4)
        .zip(b.bytes.chunks_exact(4))
        .filter(|(a, b)| (0..3).any(|i| a[i].abs_diff(b[i]) > 15))
        .count()
}
fn view() -> Camera3D {
    Camera3D {
        position: vec3(125., -210., 110.),
        target: vec3(0., 0., 40.),
        up: Vec3::Z,
        fovy: 45_f32.to_radians(),
        z_near: 0.5,
        z_far: 5000.,
        ..Default::default()
    }
}
fn start(clock: f32) {
    clear_background(Color::new(0.07, 0.08, 0.10, 1.));
    let camera = view();
    set_camera(&camera);
    crate::render_fx::begin_view(&camera, clock, &Atmosphere::default(), true);
}
async fn capture(name: &str) -> Result<Image> {
    crate::render_fx::finish();
    let image = get_screen_data();
    crate::viewer::save_capture(std::path::Path::new(&format!(
        "private/electric-check/{name}.png"
    )))?;
    next_frame().await;
    Ok(image)
}
pub async fn render(assets: &mut Assets) -> Result<()> {
    std::fs::create_dir_all("private/electric-check")?;
    let specs = texture::read_materials(assets)?;
    let shock = &specs["powerups/shock1"];
    ensure!(
        shock
            .stages
            .iter()
            .all(|s| s.alpha_test == 5 && (s.alpha_reference - 0.3).abs() < 0.001),
        "Shock alpha test was dropped"
    );
    for name in [
        "cardguard_club",
        "cardguard_diamond",
        "c_insanechild_muzzle",
        "c_armyant",
        "c_boojum",
        "c_ladybug",
    ] {
        let mut p = Puppet::load(assets, name, &[], &specs)?;
        p.atmosphere(&Atmosphere::default(), view().position);
        let clip = p.idle_clip().to_owned();
        let mut frames = Vec::new();
        for (label, remaining, clock) in [("off", 0., 7.), ("on", 0.4, 7.), ("paused", 0.4, 901.)] {
            start(clock);
            p.draw(
                &clip,
                0.2,
                true,
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                1.,
                true,
            );
            p.draw_electric(remaining);
            frames.push(capture(&format!("{name}-{label}")).await?);
        }
        let pixels = changed(&frames[0], &frames[1]);
        ensure!(
            pixels > 500,
            "{name}: shell invisible ({pixels} changed pixels)"
        );
        // An inward shell can still change a few edge pixels. Require bright
        // arcs outside the opaque silhouette, where the original effect lives.
        let background = &frames[0].bytes[..3];
        let exterior = frames[0]
            .bytes
            .chunks_exact(4)
            .zip(frames[1].bytes.chunks_exact(4))
            .filter(|(a, b)| {
                &a[..3] == background && b[2] > 100 && b[2] as f32 > b[0] as f32 * 1.15
            })
            .count();
        ensure!(
            exterior > frames[0].width as usize * frames[0].height as usize / 1100,
            "{name}: lightning is buried in the body ({exterior} exterior pixels)"
        );
        println!("PASS outward lightning {name}: {exterior} bright exterior pixels");
        ensure!(
            frames[1].bytes == frames[2].bytes,
            "{name}: paused shell depends on level clock"
        );
        println!("PASS visible electric {name}: {pixels} changed pixels; stable paused clock");
        if name == "cardguard_club" {
            start(7.);
            p.draw(
                &clip,
                0.2,
                true,
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                1.,
                true,
            );
            ensure!(!p.0.electric_attachments.is_empty(), "Guard staff missing");
            p.0.electric_attachments.clear();
            p.draw_electric(0.4);
            let body_only = capture("club-without-staff-electric").await?;
            ensure!(
                changed(&frames[1], &body_only) > 500,
                "Staff has no visible lightning"
            );
            p.show_attachments(false);
            start(7.);
            p.draw(
                &clip,
                0.2,
                true,
                Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                1.,
                true,
            );
            ensure!(
                p.0.electric_attachments.is_empty(),
                "Hidden staff retained shock geometry"
            );
            p.draw_electric(0.4);
            capture("club-hidden-staff").await?;
        }
    }

    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let world = World::fixture(&[(vec3(-5000., -5000., -20.), vec3(5000., 5000., 0.))]);
    let mut npcs = Npcs::load(assets, &map, "skool1", None, false, false)?;
    let pupil = npcs
        .actors
        .iter()
        .position(|a| a.spawn.model == "c_insanechild_muzzle" && !a.spawn.hidden)
        .context("Missing original Skool pupil")?;
    npcs.actors = vec![npcs.actors[pupil].clone()];
    npcs.pupil_path.clear();
    let mut alice = Character::load(assets)?;
    let mut player = Player::new(Vec3::Z * 0.04);
    player.grounded = true;
    alice.reset(&player, 0.);
    let initial = alice.snapshot();
    for alternate in [false, true] {
        let x = if alternate { 220. } else { 55. };
        let a = &mut npcs.actors[0];
        a.footing = Some(vec3(x, 0., 0.04));
        a.electric = 0.;
        a.time = 0.;
        a.walk = None;
        a.yaw = std::f32::consts::PI;
        ensure!(npcs.targets().is_empty(), "Pupil became a hostile target");
        let targets = npcs.mallet_contacts();
        ensure!(targets.len() == 1, "Pupil has no contact bounds");
        alice.restore(&initial)?;
        let mut stats = crate::inventory::Stats::weapon_preview();
        stats.select(2);
        let input = WeaponInput {
            selected: 2,
            aim: Vec3::X,
            ..Default::default()
        };
        for _ in 0..90 {
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                input.clone(),
                &combat::Context {
                    world: &world,
                    targets: &[],
                },
                Some(&mut stats),
            );
        }
        let mut hits = 0;
        for tick in 0..if alternate { 49 } else { 26 } {
            let mut i = input.clone();
            i.click = (tick == 0).then_some(alternate);
            alice.update_funded(
                1. / 60.,
                &player,
                false,
                i,
                &combat::Context {
                    world: &world,
                    targets: &targets,
                },
                Some(&mut stats),
            );
            for hit in alice.take_hits() {
                if hit.id == 0 {
                    hits += 1;
                    npcs.hit(hit);
                }
            }
            npcs.update(1. / 60., &world, Vec3::splat(10000.));
        }
        ensure!(
            hits == 1 && npcs.actors[0].electric > 0. && npcs.targets().is_empty(),
            "Pupil direct contact failed"
        );
        ensure!(
            stats.will() == if alternate { 92. } else { 100. },
            "Pupil contact changed weapon cost"
        );
        let saved = npcs.snapshot();
        let encoded = serde_json::to_vec(&saved)?;
        npcs.update(0., &world, Vec3::splat(10000.));
        ensure!(
            encoded == serde_json::to_vec(&npcs.snapshot())?,
            "Pause changed pupil state"
        );
        npcs.restore(&serde_json::from_slice(&encoded)?)?;
        // Move the camera fixture to its origin after testing the actual swept hit.
        npcs.actors[0].footing = Some(Vec3::Z * 0.04);
        start(123.);
        npcs.draw(view().position, -Vec3::X, &Atmosphere::default(), true);
        capture(if alternate {
            "pupil-ball-hit"
        } else {
            "pupil-melee-hit"
        })
        .await?;
        npcs.restore(&saved)?;
        for _ in 0..61 {
            npcs.update(1. / 60., &world, Vec3::splat(10000.));
        }
        let future = serde_json::to_vec(&npcs.snapshot())?;
        npcs.restore(&saved)?;
        for _ in 0..61 {
            npcs.update(1. / 60., &world, Vec3::splat(10000.));
        }
        ensure!(
            npcs.actors[0].electric == 0. && future == serde_json::to_vec(&npcs.snapshot())?,
            "Pupil saved continuation differs"
        );
        println!("PASS protected pupil alternate={alternate}: actual Mallet contact, aura, pause, restored future and cleanup");
    }
    Ok(())
}

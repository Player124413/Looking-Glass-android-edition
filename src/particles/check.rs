use super::*;
use anyhow::ensure;

pub(crate) async fn ambient_render(assets: &mut Assets) -> Result<()> {
    let mut lanterns = 0;
    let mut fireflies = 0;
    for map_name in [
        "gvillage",
        "pandemonium",
        "garden2",
        "garden3",
        "potears2",
        "potears3",
    ] {
        let map = Bsp::parse(&assets.read(&format!("maps/{map_name}.bsp"))?)?;
        let mut steam = Steam::load(assets, &map)?;
        for (id, entity) in map.entities.iter().enumerate() {
            let Some(name) = entity.get("model") else {
                continue;
            };
            let name = name.trim_start_matches("models/");
            if !matches!(name, "lantern.tik" | "firefly.tik") {
                continue;
            }
            let emitter = steam
                .emitters
                .iter()
                .find(|e| e.id == id)
                .ok_or_else(|| anyhow::anyhow!("Missing ambient emitter {map_name}:{id}"))?;
            if name == "lantern.tik" {
                lanterns += 1;
                ensure!(
                    emitter.spec.constrain == Some(Vec3::splat(3.5)) && emitter.textures.len() == 1,
                    "Lantern bounds or sprite missing"
                );
                ensure!(
                    emitter.spec.min_velocity == Vec3::splat(5.) && emitter.carry == 1.,
                    "Lantern initial motion missing"
                );
            } else {
                fireflies += 1;
                let style = emitter
                    .light_style
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Missing firefly style"))?;
                ensure!(
                    style.sample(0.) != style.sample(1.),
                    "Firefly style is static"
                );
            }
        }
        steam.animate(1.125);
        let before = steam.lights().iter().map(|l| l.color).collect::<Vec<_>>();
        steam.animate(1.125);
        ensure!(
            before == steam.lights().iter().map(|l| l.color).collect::<Vec<_>>(),
            "Paused light changed"
        );
        let mut restored = Steam::load(assets, &map)?;
        restored.animate(1.125);
        ensure!(
            before
                == restored
                    .lights()
                    .iter()
                    .map(|l| l.color)
                    .collect::<Vec<_>>(),
            "Restored light phase changed"
        );
    }
    ensure!(
        lanterns == 31 && fireflies == 15,
        "Ambient placement coverage changed: {lanterns}/{fireflies}"
    );
    println!("PASS 31 lantern emitters, 15 firefly styles, paused and restored light clocks");

    for (name, id, offset) in [
        ("gvillage", 56, vec3(80., -75., 35.)),
        ("potears2", 28, vec3(70., -100., 25.)),
    ] {
        let mut scene = crate::render::Scene::load(assets, name)?;
        let mut steam = Steam::load(assets, &scene.map)?;
        let emitter = steam.emitters.iter().find(|e| e.id == id).unwrap();
        let target = emitter.origin
            + if name == "gvillage" {
                Vec3::Z * 10.
            } else {
                Vec3::ZERO
            };
        let eye = if name == "potears2" {
            [Vec3::X, Vec3::Y, -Vec3::X, -Vec3::Y]
                .into_iter()
                .map(|axis| target + emitter.rotation * axis * 96. + Vec3::Z * 12.)
                .find(|eye| {
                    let sight = scene.world.sweep(target, *eye, Vec3::splat(1.));
                    !sight.start_solid && sight.fraction == 1.
                })
                .ok_or_else(|| anyhow::anyhow!("No clear firefly inspection camera"))?
        } else {
            target + offset
        };
        for _ in 0..100 {
            steam.update(0.05, eye, &scene.world);
        }
        if name == "gvillage" {
            ensure!(
                steam
                    .puffs
                    .iter()
                    .any(|p| steam.emitters[p.emitter].id == id),
                "Lantern pixie died inside its enclosure"
            );
            for p in &steam.puffs {
                if let Some(bounds) = steam.emitters[p.emitter].spec.constrain {
                    ensure!(
                        (p.position - p.constraint_center)
                            .abs()
                            .cmple(bounds + Vec3::splat(0.001))
                            .all(),
                        "Escaped lantern pixie"
                    );
                }
            }
        }
        let mut reference: Option<Vec<u8>> = None;
        for shot in ["live", "paused"] {
            steam.animate(5.);
            steam.update(0., eye, &scene.world);
            clear_background(scene.atmosphere.background());
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 60_f32.to_radians(),
                z_near: 1.,
                z_far: 10000.,
                ..Default::default()
            });
            crate::lighting::select(steam.lights(), eye, &scene.world);
            let direction = (target - eye).normalize();
            crate::render_fx::begin(eye, direction, 5., &scene.atmosphere, false);
            scene.draw(eye, 5., false, false, &[]);
            scene.draw_with_particles(eye, direction, 5., false, &[], &steam);
            let (_, dropped) = crate::render_fx::finish();
            ensure!(dropped == 0, "Ambient effects exceeded render budget");
            let image = get_screen_data();
            if let Some(before) = &reference {
                ensure!(
                    before
                        .iter()
                        .zip(&image.bytes)
                        .all(|(a, b)| a.abs_diff(*b) <= 1),
                    "Paused ambient image changed"
                );
            }
            image.export_png(&format!("private/ambient-{name}-{shot}.png"));
            reference = Some(image.bytes);
            gl_use_default_material();
            set_default_camera();
            next_frame().await;
        }
        println!("PASS staged {name} ambient detail and paused pixels");
    }
    Ok(())
}

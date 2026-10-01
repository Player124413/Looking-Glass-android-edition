//! Native regression for reverse sky-model faces visible through the school floor.
use super::*;
use anyhow::ensure;

pub async fn check_school_floor(assets: &mut Assets) -> Result<()> {
    let mut scene = Scene::load(assets, "skool1")?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "skool1", None)?;
    let poses = interactions.transforms();
    let sides: Vec<_> = scene.sky_props.iter().map(|b| b.face_side).collect();
    ensure!(
        sides.contains(&crate::materials::FaceSide::Front),
        "No front-sided school sky models"
    );
    for (name, eye, target) in [
        (
            "hall",
            vec3(-2300., 3040., -80.),
            vec3(-2120., 3020., -390.),
        ),
        (
            "broken-floor",
            vec3(-1890., 2500., -305.),
            vec3(-1650., 2580., -530.),
        ),
        (
            "library-west",
            vec3(-180., 3300., -80.),
            vec3(80., 3340., -450.),
        ),
        (
            "library-east",
            vec3(580., 3300., -80.),
            vec3(350., 3340., -450.),
        ),
        (
            "upper-sky",
            vec3(208., 4200., 1450.),
            vec3(208., 4700., 1700.),
        ),
    ] {
        let mut frames = Vec::new();
        for two_sided in [false, true] {
            for (batch, &side) in scene.sky_props.iter_mut().zip(&sides) {
                batch.face_side = if two_sided {
                    crate::materials::FaceSide::Both
                } else {
                    side
                };
            }
            for frame in 0..3 {
                clear_background(BLACK);
                let camera = Camera3D {
                    position: eye,
                    target,
                    up: Vec3::Z,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 10000.,
                    ..Default::default()
                };
                set_camera(&camera);
                crate::render_fx::begin_view(&camera, 0., &scene.atmosphere, false);
                scene.draw(eye, 0., false, false, &poses);
                depth_read_only(|| scene.draw(eye, 0., false, true, &poses));
                crate::render_fx::finish();
                set_default_camera();
                if frame == 2 {
                    frames.push(get_screen_data());
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/floor-{name}-{}.png",
                        if two_sided { "old" } else { "fixed" }
                    )))?;
                }
                next_frame().await;
            }
        }
        if name == "broken-floor" {
            let a = &frames[0];
            let b = &frames[1];
            let changed = a
                .bytes
                .chunks_exact(4)
                .zip(b.bytes.chunks_exact(4))
                .filter(|(a, b)| a[..3] != b[..3])
                .count();
            ensure!(
                changed > 2000,
                "Reverse sky faces reappeared: only {changed} different pixels"
            );
            // Foreground carpet stays identical; this fixture only changes sky faces.
            for x in 40..300 {
                for y in 20..100 {
                    let x = x * a.width as u32 / 1200;
                    let y = y * a.height as u32 / 680;
                    ensure!(
                        a.get_pixel(x, y) == b.get_pixel(x, y),
                        "Sky culling changed foreground carpet"
                    );
                }
            }
            println!("PASS school sky backface regression: {changed} corrected pixels; foreground unchanged");
        }
    }
    Ok(())
}

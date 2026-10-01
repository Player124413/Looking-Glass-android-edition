//! Source-data and GPU checks for camera-facing world effects.
use super::*;
use anyhow::ensure;
use std::collections::BTreeSet;

fn source_quad(vertices: &[crate::bsp::MapVertex]) -> Vec<Vertex> {
    vertices
        .iter()
        .map(|v| Vertex {
            position: v.position,
            uv: v.uv,
            color: v.color,
            normal: Vec4::ZERO,
        })
        .collect()
}

fn audit(assets: &mut Assets) -> Result<serde_json::Value> {
    let specs = texture::read_materials(assets)?;
    let names = assets.maps();
    let mut maps = BTreeMap::<String, usize>::new();
    let mut shaders = BTreeSet::new();
    let mut views = 0;
    for name in &names {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        for (face, surface) in map.surfaces.iter().enumerate() {
            let shader = map.shaders[surface.shader].name.to_ascii_lowercase();
            let Some(spec) = specs.get(&shader) else {
                continue;
            };
            if !spec
                .deforms
                .iter()
                .any(|d| matches!(d, crate::materials::Deform::Sprite(true)))
            {
                continue;
            }
            let (vertices, _) = map.triangulate(surface);
            ensure!(
                vertices.len() == 4,
                "Non-quad axial sprite: {name}/{face}/{shader}"
            );
            let source = source_quad(&vertices);
            let center = source.iter().map(|v| v.position).sum::<Vec3>() * 0.25;
            let length = |a: usize, b: usize| source[a].position.distance(source[b].position);
            let first = usize::from(length(0, 1) > length(1, 2));
            let [a, b, c, d] = [0, 1, 2, 3].map(|i| (first + i) % 4);
            let lower = (source[a].position + source[b].position) * 0.5;
            let upper = (source[c].position + source[d].position) * 0.5;
            let authored_edge = source[b].position - source[a].position;
            for elevation in [-0.8, 0., 0.8] {
                for angle in 0..16 {
                    let angle = angle as f32 * std::f32::consts::TAU / 16.;
                    let camera = center + vec3(angle.cos(), angle.sin(), elevation) * 250.;
                    let mut quad = source.clone();
                    axial_billboard(&mut quad, camera);
                    let edge = quad[b].position - quad[a].position;
                    // Independent contract: don't turn a two-sided effect over
                    // relative to the authored surface, including horizontal axes.
                    ensure!(
                        edge.dot(authored_edge) >= -0.01
                            && ((quad[a].position + quad[b].position) * 0.5).distance(lower) < 0.01
                            && ((quad[c].position + quad[d].position) * 0.5).distance(upper) < 0.01
                            && (edge.length() - authored_edge.length()).abs() < 0.01
                            && quad.iter().zip(&source).all(|(v, s)| v.position.is_finite() && v.uv == s.uv),
                        "Billboard flipped or changed its mount/size/UVs: {name}/{face}/{shader}, {camera:?}"
                    );
                    views += 1;
                }
            }
            *maps.entry(name.clone()).or_default() += 1;
            shaders.insert(shader);
        }
    }
    ensure!(!maps.is_empty(), "No source billboard surfaces audited");
    let count: usize = maps.values().sum();
    println!(
        "PASS billboard corpus: {} maps scanned, {count} surfaces in {} maps, {views} camera views",
        names.len(),
        maps.len()
    );
    Ok(
        serde_json::json!({"maps_scanned": names.len(), "maps": maps, "surfaces": count, "shaders": shaders, "camera_views": views}),
    )
}

fn pixel_at(frame: &Image, camera: &Camera3D, point: Vec3) -> Color {
    use macroquad::camera::Camera;
    let clip = camera.matrix() * point.extend(1.);
    let ndc = clip.truncate() / clip.w;
    // GPU readback is bottom-up; the capture helper flips rows for PNGs.
    let x = ((ndc.x * 0.5 + 0.5) * frame.width as f32) as u32;
    let y = ((ndc.y * 0.5 + 0.5) * frame.height as f32) as u32;
    frame.get_pixel(
        x.min(frame.width as u32 - 1),
        y.min(frame.height as u32 - 1),
    )
}

pub async fn check(assets: &mut Assets) -> Result<()> {
    let mut report = audit(assets)?;
    let mut scene = Scene::load(assets, "skool1")?;
    let target = vec3(-2372., 2622., -435.);
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "skool1", None)?;
    let poses = interactions.transforms();
    for (index, (offset, time)) in [
        (vec3(200., 0., 30.), 0.025),
        (vec3(160., -70., 20.), 0.175),
        (vec3(160., 70., 40.), 0.425),
    ]
    .into_iter()
    .enumerate()
    {
        let view = Camera3D {
            position: target + offset,
            target,
            up: Vec3::Z,
            fovy: 55_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        };
        clear_background(BLACK);
        set_camera(&view);
        crate::render_fx::begin_view(&view, time, &scene.atmosphere, false);
        scene.draw(view.position, time, false, false, &poses);
        scene.draw(view.position, time, false, true, &poses);
        ensure!(
            crate::render_fx::finish().1 == 0,
            "Billboard capture queue overflow"
        );
        crate::viewer::save_capture(std::path::Path::new(&format!(
            "private/billboards/skool1-fire-{index}.png"
        )))?;
        gl_use_default_material();
        set_default_camera();
        next_frame().await;
    }

    // Use the real fireplace geometry/materials and the normal submission
    // paths, with an asymmetric image so a flip has an unambiguous GPU oracle.
    scene.batches.retain(|b| {
        b.center.distance(target) < 0.1
            && b.deforms
                .iter()
                .any(|d| matches!(d, crate::materials::Deform::Sprite(true)))
    });
    ensure!(
        scene.batches.len() == 1,
        "Expected the skool1 fireplace billboard"
    );
    scene.sky_origin = None;
    scene.atmosphere = Atmosphere::default();
    let bytes = (0..64)
        .flat_map(|i| {
            if i / 8 < 4 {
                [255, 0, 0, 255]
            } else {
                [0, 0, 255, 255]
            }
        })
        .collect::<Vec<_>>();
    // Match world texture wrapping: the source fireplace UVs span ~1..2.
    let marker = upload(texture::RgbaImage {
        width: 8,
        height: 8,
        pixels: bytes,
    });
    marker.set_filter(FilterMode::Nearest);
    for layer in &mut scene.batches[0].layers {
        for texture in &mut layer.textures {
            *texture = marker.clone();
        }
    }
    let mut frames = 0;
    for queued in [false, true] {
        for offset in [
            vec3(200., 0., 0.),
            vec3(-200., 0., 0.),
            vec3(180., 70., 30.),
            vec3(-180., -70., -30.),
        ] {
            for time in [0.025, 0.175, 0.425] {
                let view = Camera3D {
                    position: target + offset,
                    target,
                    up: Vec3::Z,
                    projection: Projection::Orthographics,
                    fovy: 100.,
                    z_near: 2.,
                    z_far: 1000.,
                    ..Default::default()
                };
                clear_background(BLACK);
                set_camera(&view);
                if queued {
                    crate::render_fx::begin_view(&view, time, &scene.atmosphere, true);
                }
                scene.draw(view.position, time, true, true, &poses);
                if queued {
                    ensure!(
                        crate::render_fx::finish().1 == 0,
                        "Billboard queue overflow"
                    );
                }
                let image = get_screen_data();
                let top = pixel_at(&image, &view, target + Vec3::Z * 14.);
                let bottom = pixel_at(&image, &view, target - Vec3::Z * 14.);
                ensure!(top.r > 0.8 && top.b < 0.1 && bottom.b > 0.8 && bottom.r < 0.1,
                    "Fireplace is inverted or missing: queued={queued}, camera={offset:?}, time={time}, top={top:?}, bottom={bottom:?}");
                frames += 1;
                gl_use_default_material();
                set_default_camera();
                next_frame().await;
            }
        }
    }
    report["gpu_frames"] = frames.into();
    report["status"] = "passed".into();
    std::fs::write(
        "private/billboards/report.json",
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("PASS {frames} billboard GPU frames: both sides, raised/lowered cameras, animation frames, legacy and queued draw paths");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_fireplace_never_turns_over_from_either_side() {
        // skool1 face 3147 is slightly wider than tall: its long axis is Y,
        // unlike the narrow vertical flames that concealed the original bug.
        let positions = [
            vec3(-2372., 2586., -470.),
            vec3(-2372., 2586., -400.),
            vec3(-2372., 2658., -400.),
            vec3(-2372., 2658., -470.),
        ];
        for rotation in [Quat::IDENTITY, Quat::from_rotation_y(0.65)] {
            for reversed in [false, true] {
                for shift in 0..4 {
                    let source = (0..4)
                        .map(|i| {
                            let index = (shift + if reversed { 4 - i } else { i }) % 4;
                            Vertex {
                                position: rotation * positions[index],
                                uv: vec2(
                                    1.98958 - f32::from(index >= 2),
                                    if index == 0 || index == 3 {
                                        1.995536
                                    } else {
                                        0.995536
                                    },
                                ),
                                color: [255; 4],
                                normal: Vec4::ZERO,
                            }
                        })
                        .collect::<Vec<_>>();
                    let center = source.iter().map(|v| v.position).sum::<Vec3>() * 0.25;
                    for offset in [
                        Vec3::X * 200.,
                        Vec3::NEG_X * 200.,
                        vec3(180., 70., 30.),
                        vec3(-180., -70., -30.),
                        Vec3::Y * 200.,
                    ] {
                        let mut quad = source.clone();
                        axial_billboard(&mut quad, center + rotation * offset);
                        for (v, s) in quad.iter().zip(&source) {
                            let p = rotation.conjugate() * v.position;
                            let original = rotation.conjugate() * s.position;
                            assert!(
                                (p.z + 435.) * (original.z + 435.) > 0.,
                                "Flipped flame corner: {p:?}"
                            );
                            assert!((p.y - original.y).abs() < 0.002);
                            assert_eq!(v.uv, s.uv);
                        }
                    }
                }
            }
        }
    }
}

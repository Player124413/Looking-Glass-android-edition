//! Original water materials through the production world renderer.
use super::*;
use anyhow::ensure;
use crate::materials::Deform;

fn pixels(scene: &mut Scene, camera: &Camera3D, time: f32) -> Image {
    clear_background(scene.atmosphere.background());
    set_camera(camera);
    crate::lighting::select(vec![], camera.position, &scene.world);
    crate::render_fx::begin_view(camera, time, &scene.atmosphere, false);
    scene.draw(camera.position, time, false, false, &[]);
    depth_read_only(|| scene.draw(camera.position, time, false, true, &[]));
    crate::render_fx::finish();
    set_default_camera();
    get_screen_data()
}

pub async fn check(assets: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(assets)?;
    for name in ["tears2water", "blood", "mercury", "bw_water", "chess_red", "testwater2", "tower2", "hedge"] {
        let spec = specs.get(&format!("textures/liquid/{name}")).context("Missing original water shader")?;
        ensure!(spec.deforms.iter().any(|d| matches!(d, Deform::WaveNormal(..))), "Missing water normal wave: {name}");
    }
    let root = std::path::Path::new("private/water-fix/captures");
    std::fs::create_dir_all(root)?;
    let mut scene = Scene::load(assets, "potears2")?;
    let original: Vec<_> = scene.batches.iter().map(|b| b.deforms.clone()).collect();
    let wave_batches = original.iter().filter(|ds| ds.iter().any(|d| matches!(d, Deform::WaveNormal(..)))).count();
    // Hollow's opaque pond must remain merged/cullable; ripple support must not
    // turn each authored 128-unit tile into a separate draw call.
    ensure!(wave_batches > 0 && wave_batches < 10, "Unbounded pond batching: {wave_batches}");
    for (name, eye, target) in [
        ("hollow-bank", vec3(-4600., 1800., 300.), vec3(-4250., 2400., 180.)),
        ("hollow-falls", vec3(-3970., 1750., 290.), vec3(-4460., 2480., 300.)),
    ] {
        let camera = Camera3D { position: eye, target, up: Vec3::Z,
            fovy: 75_f32.to_radians(), z_near: 2., z_far: 30000., ..Default::default() };
        let mut images = Vec::new();
        for (label, time, waves) in [("without-ripples", 0.75, false), ("fixed", 0.75, true), ("paused", 0.75, true), ("later", 2., true)] {
            for (batch, deforms) in scene.batches.iter_mut().zip(&original) {
                batch.deforms = deforms.iter().filter(|d| waves || !matches!(d, Deform::WaveNormal(..))).cloned().collect();
            }
            // Warm the shared pipeline and render target before pixel comparison.
            pixels(&mut scene, &camera, time);
            next_frame().await;
            images.push(pixels(&mut scene, &camera, time));
            crate::viewer::save_capture(&root.join(format!("{name}-{label}.png")))?;
            next_frame().await;
        }
        let changed = |a: usize, b: usize| images[a].bytes.chunks_exact(4).zip(images[b].bytes.chunks_exact(4)).filter(|(x,y)| x != y).count();
        ensure!(changed(0,1) > 500, "Water reflection did not respond to normal waves at {name}");
        ensure!(changed(1,2) == 0, "Paused water changed at {name}");
        ensure!(changed(1,3) > 500, "Water did not animate at {name}");
        println!("PASS {name}: {} ripple pixels, exact paused image, {} changing water/fall pixels; {wave_batches} pond batches", changed(0,1), changed(1,3));
    }
    // Isolate the pond itself: waterfall scrolling must not be able to make
    // the surface-animation assertion pass while the horizontal water is still.
    scene.batches.retain(|b| b.deforms.iter().any(|d| matches!(d, Deform::WaveNormal(..))));
    let camera = Camera3D { position: vec3(-4600., 1800., 300.), target: vec3(-4250., 2400., 180.),
        up: Vec3::Z, fovy: 75_f32.to_radians(), z_near: 2., z_far: 30000., ..Default::default() };
    let first = pixels(&mut scene, &camera, 0.);
    next_frame().await;
    let later = pixels(&mut scene, &camera, 1.);
    let moving = first.bytes.chunks_exact(4).zip(later.bytes.chunks_exact(4))
        .filter(|(a,b)| a[..3].iter().zip(&b[..3]).any(|(x,y)| x.abs_diff(*y) >= 3)).count();
    ensure!(moving > 500, "Pond itself stays still: {moving} visibly changing pixels");
    println!("PASS isolated pond: {moving} visibly changing RGB pixels in one second, without waterfall");
    next_frame().await;
    Ok(())
}

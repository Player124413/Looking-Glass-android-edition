//! A clean game frame stays on the GPU until a save needs a small JPEG preview.
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

const MAX_BYTES: usize = 128 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub struct Preview {
    jpeg: Vec<u8>,
}
impl Preview {
    fn encode(frame: Image) -> Result<Self> {
        let mut pixels =
            image::RgbaImage::from_raw(frame.width.into(), frame.height.into(), frame.bytes)
                .context("Invalid preview frame")?;
        // OpenGL texture readback starts with the bottom row.
        image::imageops::flip_vertical_in_place(&mut pixels);
        let small = image::DynamicImage::ImageRgba8(pixels)
            .resize(320, 240, image::imageops::FilterType::Triangle)
            .to_rgb8();
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85).encode(
            &small,
            small.width(),
            small.height(),
            image::ColorType::Rgb8,
        )?;
        ensure!(jpeg.len() <= MAX_BYTES, "Preview is too large");
        Ok(Self { jpeg })
    }
    pub fn from_value(value: serde_json::Value) -> Option<Self> {
        serde_json::from_value::<Self>(value)
            .ok()
            .filter(|p| p.jpeg.len() <= MAX_BYTES)
    }
    pub fn image(&self) -> Result<image::RgbaImage> {
        ensure!(self.jpeg.len() <= MAX_BYTES, "Preview is too large");
        let mut reader = image::io::Reader::with_format(
            std::io::Cursor::new(&self.jpeg),
            image::ImageFormat::Jpeg,
        );
        let mut limits = image::io::Limits::default();
        limits.max_image_width = Some(320);
        limits.max_image_height = Some(240);
        limits.max_alloc = Some(4 * 1024 * 1024);
        reader.limits(limits);
        Ok(reader.decode()?.to_rgba8())
    }
    pub fn texture(&self) -> Option<Texture2D> {
        let image = self.image().ok()?;
        let texture = Texture2D::from_rgba8(image.width() as u16, image.height() as u16, &image);
        texture.set_filter(FilterMode::Linear);
        Some(texture)
    }
}

#[derive(Default)]
pub struct Frame {
    texture: Option<Texture2D>,
    tick: u32,
}

/// Isolated native persistence/menu regression; never opens player save slots.
pub async fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::{
        movement::Player,
        save::{Campaign, Game, Level, Slot, Store, View},
    };
    use std::{collections::BTreeMap, fs, path::Path};
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "skool1", None)?;
    interactions.sync(&mut scene.world);
    let (spawn, yaw) = crate::interaction::spawn(&scene.map, None);
    let player = Player::spawn(&scene.world, spawn).context("Preview fixture spawn obstructed")?;
    let mut alice = crate::character::Character::load(assets)?;
    alice.reset(&player, yaw);
    let npcs = crate::npc::Npcs::load(assets, &scene.map, "skool1", None, false, false)?;
    let story = crate::story::Story::load(assets, "skool1");
    let hints = crate::cheshire::Hints::load(assets, &scene.map, "skool1")?;
    let level = Level {
        map: "skool1".into(),
        entry: None,
        interactions: interactions.snapshot(),
        npcs: npcs.snapshot(),
        story: story.snapshot(),
        hints: hints.snapshot(),
        environment_clock: 0.,
        pickup_clock: 0.,
    };
    let game = Game {
        current: level.key(),
        campaign: Campaign {
            levels: BTreeMap::from([(level.key(), level)]),
            ..Default::default()
        },
        stats: crate::inventory::Stats::for_level("skool1", None),
        view: View {
            position: player.eye(),
            yaw,
            pitch: 0.,
            third_person: false,
            flying: false,
            fullbright: false,
            spawn_landing: true,
        },
        player: player.clone(),
        recovery: Default::default(),
        character: alice.snapshot(),
    };
    let root = Path::new("private/save-preview-check");
    let fingerprint = assets.fingerprint()?;
    let store = Store::new(root.into(), fingerprint.clone());
    let mut frame = Frame::default();
    let mut previews = Vec::new();
    for (index, turn) in [0_f32, 1.2].into_iter().enumerate() {
        clear_background(BLACK);
        let direction = vec3((yaw + turn).cos(), (yaw + turn).sin(), 0.);
        set_camera(&Camera3D {
            position: player.eye(),
            target: player.eye() + direction,
            up: Vec3::Z,
            fovy: 75_f32.to_radians(),
            z_near: 2.,
            z_far: 30000.,
            ..Default::default()
        });
        scene.draw(player.eye(), 0., false, false, &interactions.transforms());
        crate::render::depth_read_only(|| {
            scene.draw(player.eye(), 0., false, true, &interactions.transforms())
        });
        frame.update();
        let preview = frame.preview().context("No clean frame captured")?;
        set_default_camera();
        draw_rectangle(0., 0., screen_width(), screen_height(), GREEN);
        // A menu drawn after capture must never become the stored screenshot.
        ensure!(
            frame.preview().unwrap().jpeg == preview.jpeg,
            "Menu contaminated game preview"
        );
        preview
            .image()?
            .save(format!("private/save-preview-view-{index}.png"))?;
        previews.push(preview);
        next_frame().await;
    }
    ensure!(
        previews[0].jpeg != previews[1].jpeg,
        "Different views produced identical previews"
    );
    store.write_with_preview(Slot::One, &game, Some(&previews[0]))?;
    let mut newer = game.clone();
    newer.stats.damage(5.);
    store.write_with_preview(Slot::One, &newer, Some(&previews[1]))?;
    let reopen = Store::new(root.into(), fingerprint);
    let loaded = reopen.read(Slot::One)?;
    ensure!(
        loaded.preview.unwrap().jpeg == previews[1].jpeg && loaded.game.stats.sanity() == 95.,
        "Overwrite mismatched state and preview"
    );
    let committed = fs::read(root.join("slot1.json"))?;
    let mut bad = newer.clone();
    bad.view.pitch = 99.;
    ensure!(
        reopen
            .write_with_preview(Slot::One, &bad, Some(&previews[0]))
            .is_err(),
        "Invalid save accepted"
    );
    ensure!(
        fs::read(root.join("slot1.json"))? == committed,
        "Failed write damaged save/preview"
    );
    fs::write(root.join("slot1.json"), b"{interrupted")?;
    let recovered = reopen.read(Slot::One)?;
    ensure!(
        recovered.backup
            && recovered.game.stats.sanity() == 100.
            && recovered.preview.unwrap().jpeg == previews[0].jpeg,
        "Backup image mismatched restored game"
    );
    reopen.write_with_preview(Slot::One, &game, Some(&previews[0]))?;
    reopen.write_with_preview(Slot::Two, &newer, Some(&previews[1]))?;
    reopen.write_with_preview(Slot::Quick, &game, Some(&previews[0]))?;
    reopen.write_with_preview(Slot::Auto, &newer, Some(&previews[1]))?;
    reopen.write_with_preview(Slot::Three, &game, Some(&previews[0]))?;
    reopen.write(Slot::Three, &game)?;
    ensure!(
        reopen.read(Slot::Three)?.preview.is_none(),
        "Legacy save retained unrelated old preview"
    );
    reopen.write(Slot::Four, &game)?;
    let mut malformed: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("slot4.json"))?)?;
    malformed["preview"] = serde_json::json!({ "jpeg": [1, 2, 3] });
    fs::write(root.join("slot4.json"), serde_json::to_vec(&malformed)?)?;
    let loaded = reopen.read(Slot::Four)?;
    ensure!(
        !loaded.backup && loaded.preview.unwrap().texture().is_none(),
        "Invalid preview prevented loading valid game"
    );
    crate::menu::Menu::load(assets)?
        .check_save_previews(&reopen)
        .await?;
    println!("PASS clean game capture, overwrite, disk reopen, failed save, matching backup, legacy/damaged image fallback and menu previews");
    Ok(())
}
impl Frame {
    pub fn clear(&mut self) {
        self.texture = None;
    }
    /// Call after the world and first-person toy, before any menus/HUD/cursor.
    pub fn update(&mut self) {
        let interval = crate::android::active_preset().save_preview_interval();
        if crate::android::is_android() {
            return;
        }
        self.tick = self.tick.wrapping_add(1);
        if self.texture.is_some() && interval > 1 && self.tick % interval != 0 {
            return;
        }
        let (w, h) = macroquad::miniquad::window::screen_size();
        if w < 1. || h < 1. {
            return;
        }
        unsafe {
            get_internal_gl().flush();
        }
        if self
            .texture
            .as_ref()
            .is_none_or(|t| t.width() != w || t.height() != h)
        {
            self.texture = Some(Texture2D::from_image(&Image::gen_image_color(
                w as u16, h as u16, BLACK,
            )));
        }
        self.texture.as_ref().unwrap().grab_screen();
    }
    pub fn preview(&self) -> Option<Preview> {
        Preview::encode(self.texture.as_ref()?.get_texture_data())
            .map_err(|e| {
                eprintln!("Save preview unavailable: {e:#}");
            })
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_preserves_aspect_and_corrects_gl_orientation() {
        let mut bytes = Vec::new();
        for y in 0..360 {
            for _ in 0..640 {
                bytes.extend_from_slice(if y < 180 {
                    &[0, 0, 255, 255]
                } else {
                    &[255, 0, 0, 255]
                });
            }
        }
        let preview = Preview::encode(Image {
            width: 640,
            height: 360,
            bytes,
        })
        .unwrap();
        let saved = serde_json::to_value(&preview).unwrap();
        let decoded = Preview::from_value(saved).unwrap().image().unwrap();
        assert_eq!(decoded.dimensions(), (320, 180));
        assert!(decoded.get_pixel(100, 20)[0] > 240);
        assert!(decoded.get_pixel(100, 160)[2] > 240);
        assert!(Preview::from_value(serde_json::json!({"jpeg": [1,2,3]}))
            .unwrap()
            .image()
            .is_err());
        assert!(Preview::from_value(serde_json::json!("bad preview")).is_none());
    }
}

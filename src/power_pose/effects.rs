//! Rage's supplied surface layer and the collected vial's cosmetic performance.
use super::*;
use crate::{environment::Atmosphere, materials::Stage, particles::Attached, tan};
use std::cell::Cell;

pub(super) fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}

pub(super) struct Aura {
    texture: Texture2D,
    stage: Stage,
    deforms: Vec<crate::materials::Deform>,
    pub camera: Cell<Vec3>,
    pub remaining: Cell<f32>,
}
impl Aura {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let spec = specs
            .get("powerups/rage")
            .context("Missing Rage surface layer")?;
        let image = texture::decode(
            assets,
            &texture::resolve(assets, &spec.stages[0].images[0], specs)
                .context("Missing Rage reflection image")?,
        )?;
        let texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        texture.set_filter(FilterMode::Linear);
        unsafe {
            get_internal_gl().quad_context.texture_set_wrap(
                texture.raw_miniquad_id(),
                macroquad::miniquad::TextureWrap::Repeat,
                macroquad::miniquad::TextureWrap::Repeat,
            );
        }
        Ok(Self {
            texture,
            stage: spec.stages[0].clone(),
            deforms: spec.deforms.clone(),
            camera: Cell::new(Vec3::ZERO),
            remaining: Cell::new(0.),
        })
    }
    pub fn draw(&self, mesh: &Mesh, state: &State) {
        // The native light begins at seven seconds. The short ramps avoid a
        // hard visual switch while inventory remains the sole duration owner.
        let strength = if state.form == Some(Form::Rage) {
            ease((state.age - 7.) / 0.4) * ease(self.remaining.get() / 0.4)
        } else {
            0.
        };
        if strength <= 0. || !crate::render_fx::active() {
            return;
        }
        let stage = &self.stage;
        let mut copy = crate::render_fx::copy_mesh(mesh);
        copy.texture = Some(self.texture.clone());
        for v in &mut copy.vertices {
            let normal = v.normal.truncate().normalize_or_zero();
            for d in &self.deforms {
                v.position = d.position(v.position, normal, v.uv, state.age);
            }
            v.uv = stage.view_uv(v.uv, v.position, normal, self.camera.get(), state.age);
            v.color = [255; 4];
        }
        if let Ok(mut pass) = crate::render_fx::pass(copy, stage, true, false, Default::default()) {
            pass.gain *= strength;
            crate::render_fx::submit(vec![pass], true);
        }
    }
}

pub(crate) struct Pickup {
    prop: Prop,
    model: tan::Model,
    scale: f32,
    particles: Attached,
    duration: f32,
    layers: Vec<(Stage, Texture2D)>,
}
impl Pickup {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let (def, model) = crate::weapons::read_model_clip(assets, "w_ragebox", Some("pickup"))?;
        ensure!(
            ["tag_vent01", "tag_vent02", "tag_vent03", "tag_vent04"]
                .iter()
                .all(|tag| model.tags.contains_key(*tag)),
            "Missing Rage vent anchors"
        );
        let duration = model.surfaces[0].frames.len() as f32 * model.frame_time;
        let mut layers = Vec::new();
        for stage in &specs
            .get("models/weapons/ragebox/skin01")
            .context("Missing Rage vial layers")?
            .stages
        {
            let path = texture::resolve(assets, &stage.images[0], &BTreeMap::new())
                .context("Missing Rage vial layer image")?;
            let mut image = texture::decode(assets, &path)?;
            let mut stage = stage.clone();
            // Test the supplied mask before applying the entity fade. Testing
            // faded alpha would drop the cage abruptly halfway through.
            if stage.alpha_test == 2 {
                for pixel in image.pixels.chunks_exact_mut(4) {
                    pixel[3] = if pixel[3] >= 128 { 255 } else { 0 };
                }
                stage.alpha_test = 0;
            }
            let texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
            texture.set_filter(FilterMode::Linear);
            unsafe {
                get_internal_gl().quad_context.texture_set_wrap(
                    texture.raw_miniquad_id(),
                    macroquad::miniquad::TextureWrap::Repeat,
                    macroquad::miniquad::TextureWrap::Repeat,
                );
            }
            layers.push((stage, texture));
        }
        Ok(Self {
            prop: Prop::load_animation(assets, "w_ragebox", "pickup", specs)?,
            model,
            scale: def.scale,
            duration,
            layers,
            particles: Attached::load(assets, "w_ragebox", specs)?.context("Missing Rage vents")?,
        })
    }
    pub fn draw(
        &mut self,
        time: f32,
        at: Transform,
        fullbright: bool,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        if !(0. ..RAGE_SECONDS).contains(&time) {
            return;
        }
        let alpha = 1. - ease((time - self.duration) / 10.);
        for mesh in self.prop.meshes_at(at, 1., fullbright, time, false) {
            if !crate::render_fx::active() {
                continue;
            }
            let mut passes = Vec::new();
            for (stage, texture) in &self.layers {
                let mut copy = crate::render_fx::copy_mesh(mesh);
                copy.texture = Some(texture.clone());
                for v in &mut copy.vertices {
                    v.uv = stage.view_uv(
                        v.uv,
                        v.position,
                        v.normal.truncate().normalize_or_zero(),
                        camera,
                        time,
                    );
                }
                let mut fade = stage.clone();
                if fade.blend == crate::materials::Blend::Opaque {
                    fade.blend = crate::materials::Blend::Alpha;
                }
                if let Ok(mut pass) =
                    crate::render_fx::pass(copy, &fade, true, !fade.identity, Default::default())
                {
                    if fade.blend == crate::materials::Blend::Add {
                        pass.gain *= alpha;
                    } else {
                        pass.gain.w *= alpha;
                    }
                    passes.push(pass);
                }
            }
            crate::render_fx::submit(passes, true);
        }
        let model = &self.model;
        let scale = self.scale;
        self.particles.draw(
            time,
            scale,
            |birth, name| {
                let name = name.unwrap_or("tag_vent01");
                let points = &model.tags[name];
                let rotations = &model.tag_rotations[name];
                let frame = (birth / model.frame_time).clamp(0., (points.len() - 1) as f32);
                let i = frame as usize;
                let j = (i + 1).min(points.len() - 1);
                Transform {
                    translation: at.point(points[i].lerp(points[j], frame.fract()) * scale),
                    rotation: at.rotation * rotations[i].slerp(rotations[j], frame.fract()),
                }
            },
            |name, birth, _| vent_on(name, birth, model.frame_time),
            camera,
            atmosphere,
        );
    }
}
fn vent_on(name: &str, birth: f32, frame: f32) -> bool {
    matches!(
        name,
        "vent_front" | "vent_right" | "vent_back" | "vent_left"
    ) && birth >= 39. * frame
        && birth < RAGE_SECONDS - 0.7
}

pub(super) fn blend_skin(mesh: &Mesh, texture: &Texture2D, amount: f32) {
    if !(0. ..1.).contains(&amount) || amount == 0. || !crate::render_fx::active() {
        return;
    }
    let mut copy = crate::render_fx::copy_mesh(mesh);
    copy.texture = Some(texture.clone());
    let stage = Stage {
        blend: crate::materials::Blend::Alpha,
        depth_equal: true,
        ..Default::default()
    };
    if let Ok(mut pass) = crate::render_fx::pass(copy, &stage, true, true, Default::default()) {
        pass.gain.w *= amount;
        crate::render_fx::submit(vec![pass], true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_four_authored_vents_run_and_end_before_cleanup() {
        for name in ["vent_front", "vent_right", "vent_back", "vent_left"] {
            assert!(!vent_on(name, 1.94, 0.05));
            assert!(vent_on(name, 1.96, 0.05));
            assert!(vent_on(name, 8., 0.05));
            assert!(!vent_on(name, 12., 0.05));
        }
        assert!(!vent_on("plunger", 5., 0.05));
        assert!(!vent_on("light", 5., 0.05));
    }
}

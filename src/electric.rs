//! Short, pose-following electrical shell using the original shock material.
use crate::{assets::Assets, render_fx, texture};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::{collections::BTreeMap, rc::Rc};
pub const LIFE: f32 = 0.5;
pub fn inactive(value: &f32) -> bool {
    *value == 0.
}
pub fn hit(remaining: &mut f32, hit: crate::combat::Hit) {
    if hit.damage.is_finite()
        && hit.damage > 0.
        && hit.kind.means() == crate::combat::DamageKind::Electric
    {
        *remaining = LIFE;
    }
}
pub struct Art {
    texture: Texture2D,
    deforms: Vec<crate::materials::Deform>,
    _surface: Rc<render_fx::Surface>,
}
thread_local! { static CACHE: std::cell::RefCell<std::rc::Weak<Art>> = Default::default(); }
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Rc<Self>> {
        if let Some(art) = CACHE.with(|c| c.borrow().upgrade()) {
            return Ok(art);
        }
        let name = "powerups/shock1";
        let path =
            texture::resolve(assets, name, specs).context("Missing electrical hit material")?;
        let image = texture::decode(assets, &path)?;
        let texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        texture.set_filter(FilterMode::Linear);
        let art = Rc::new(Self {
            _surface: render_fx::register(assets, &texture, &[name.into()], specs)?,
            deforms: specs[name].deforms.clone(),
            texture,
        });
        CACHE.with(|c| *c.borrow_mut() = Rc::downgrade(&art));
        Ok(art)
    }
    pub fn draw(&self, mesh: &Mesh, remaining: f32) {
        if remaining <= 0. {
            return;
        }
        let age = LIFE - remaining;
        let mut copy = render_fx::copy_mesh(mesh);
        copy.texture = Some(self.texture.clone());
        for v in &mut copy.vertices {
            // These assets use clockwise triangles. The renderer rebuilds the
            // opposite of their authored outward normal; expanding along it
            // buries the shock surface inside the opaque body.
            let normal = -v.normal.truncate().normalize_or_zero();
            v.position += normal * 0.2;
            for deform in &self.deforms {
                v.position = deform.position(v.position, normal, v.uv, age);
            }
            v.color = [255; 4];
        }
        render_fx::skin_effect(&copy, age, (remaining / 0.15).min(1.));
    }
}

//! Original iceblock secondary shaders, shared across actors and the water shell.
use crate::{assets::Assets, render_fx, texture};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::{collections::BTreeMap, rc::Rc};
pub(crate) struct Art {
    textures: [Texture2D; 2],
    _surfaces: [Rc<render_fx::Surface>; 2],
}
thread_local! { static CACHE: std::cell::RefCell<std::rc::Weak<Art>> = Default::default(); }
impl Art {
    pub fn load(
        a: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Rc<Self>> {
        if let Some(a) = CACHE.with(|c| c.borrow().upgrade()) {
            return Ok(a);
        }
        let mut textures = vec![];
        let mut surfaces = vec![];
        for name in ["powerups/iceblock1", "powerups/iceblock2"] {
            let path = texture::resolve(a, name, specs)
                .with_context(|| format!("Missing frozen shader {name}"))?;
            let im = texture::decode(a, &path)?;
            let tex = Texture2D::from_rgba8(im.width, im.height, &im.pixels);
            tex.set_filter(FilterMode::Linear);
            surfaces.push(render_fx::register(a, &tex, &[name.into()], specs)?);
            textures.push(tex);
        }
        let a = Rc::new(Self {
            textures: textures.try_into().ok().unwrap(),
            _surfaces: surfaces.try_into().ok().unwrap(),
        });
        CACHE.with(|c| *c.borrow_mut() = Rc::downgrade(&a));
        Ok(a)
    }
    pub fn draw(&self, mesh: &Mesh) {
        for (i, tex) in self.textures.iter().enumerate() {
            let mut m = render_fx::copy_mesh(mesh);
            m.texture = Some(tex.clone());
            for v in &mut m.vertices {
                let n = v.normal.truncate().normalize_or_zero();
                // iceblock2's static bulge: width 10, height 10, speed 0.
                v.position += n * if i == 1 {
                    (v.uv.x * 10.).sin() * 10.
                } else {
                    0.2
                };
                v.color = [255; 4];
            }
            render_fx::skin(&m);
        }
    }
}

//! Skin variants and emitter histories are actor-local. Shared meshes are restored after drawing.
use super::*;
use crate::{
    clockwork::Rig,
    magma::{Magma, Phase},
};
pub(super) struct Skins {
    textures: Vec<[Option<Texture2D>; 3]>,
    _layers: Vec<std::rc::Rc<crate::render_fx::Surface>>,
}
impl Skins {
    pub fn load(
        assets: &mut Assets,
        data: &Data,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut textures = vec![];
        let mut layers = vec![];
        let mut cache = BTreeMap::<String, Texture2D>::new();
        for s in &data.skeleton.surfaces {
            let names = match s.name.as_str() {
                "top_torso" | "bot_body" | "cap_body" => {
                    ["skin01.tga", "skin01a.tga", "skin01b.tga"]
                }
                "bot_outer" => ["magmaskin03.tga", "skin03a.tga", "skin03b.tga"],
                _ => {
                    textures.push([None, None, None]);
                    continue;
                }
            };
            let mut skins = [None, None, None];
            for (i, name) in names.iter().enumerate() {
                let shader = format!("{}/{name}", data.def.path);
                let tex = if let Some(tex) = cache.get(&shader) {
                    tex.clone()
                } else {
                    let path = texture::resolve(assets, &shader, specs)
                        .with_context(|| format!("Magma skin {shader}"))?;
                    let image = texture::decode(assets, &path)?;
                    let t = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                    t.set_filter(FilterMode::Linear);
                    layers.push(crate::render_fx::register(
                        assets,
                        &t,
                        std::slice::from_ref(&shader),
                        specs,
                    )?);
                    cache.insert(shader, t.clone());
                    t
                };
                skins[i] = Some(tex);
            }
            textures.push(skins);
        }
        Ok(Self {
            textures,
            _layers: layers,
        })
    }
}
impl Model {
    pub(super) fn draw_magma(
        &mut self,
        g: &Magma,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        self.visible.fill(false);
        let scale = g.visual_scale(&self.data);
        if scale <= 0. {
            return;
        }
        let time = g.sample_time(&self.data);
        let pose = self.data.clips[g.clip()].sample(time, g.loops());
        let skins = self.magma_skins.as_ref().unwrap();
        let mut original = Vec::new();
        let mut removed = Vec::new();
        for (i, surface) in self.data.skeleton.surfaces.iter().enumerate() {
            if self
                .data
                .meta
                .hidden
                .iter()
                .any(|p| crate::animation_events::matches(p, &surface.name))
            {
                removed.push((i, self.meshes[i].take()));
                continue;
            }
            if let Some(mesh) = &mut self.meshes[i] {
                original.push((i, mesh.texture.clone()));
                if let Some(tex) = &skins.textures[i][g.form as usize] {
                    mesh.texture = Some(tex.clone());
                }
                self.visible[i] = true;
            }
        }
        material.bind();
        crate::character::draw_skin_alpha(
            &self.data.skeleton,
            &mut self.meshes,
            &pose,
            Transform {
                translation: g.feet,
                rotation: Quat::from_rotation_z(g.yaw),
            },
            self.data.def.scale * scale,
            fullbright,
            if g.form < 2 {
                (g.cooling / 6. - g.form as f32).clamp(0., 1.)
            } else {
                1.
            },
        );
        for (i, tex) in original {
            self.meshes[i].as_mut().unwrap().texture = tex;
        }
        for (i, mesh) in removed {
            self.meshes[i] = mesh;
        }
        if g.frozen {
            self.draw_frozen();
        }
    }
}
pub(super) struct Art {
    smoke: Option<crate::particles::Attached>,
    trails: BTreeMap<u32, crate::particles::Attached>,
    phase: Option<Phase>,
    time: f32,
}
impl Art {
    pub fn new(template: Option<&crate::particles::Attached>) -> Self {
        Self {
            smoke: template.map(|t| t.fork()),
            trails: BTreeMap::new(),
            phase: None,
            time: 0.,
        }
    }
    pub fn draw(
        &mut self,
        g: &Magma,
        data: &Data,
        fire: Option<&crate::particles::Attached>,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        self.trails
            .retain(|id, _| g.shots.iter().any(|s| s.serial == *id));
        if let Some(template) = fire {
            for s in &g.shots {
                self.trails
                    .entry(s.serial)
                    .or_insert_with(|| template.fork())
                    .draw(
                        s.age,
                        1.,
                        |at, _| Transform {
                            translation: s.position - s.direction * 700. * (s.age - at),
                            rotation: Quat::from_rotation_arc(Vec3::X, s.direction),
                        },
                        |_, _, default| default,
                        camera,
                        atmosphere,
                    );
            }
        }
        let Some(effect) = &mut self.smoke else {
            return;
        };
        if self.phase != Some(g.phase) || g.time < self.time {
            *effect = effect.fork();
        }
        self.phase = Some(g.phase);
        self.time = g.time;
        if g.health <= 0. {
            return;
        }
        let transform = Transform {
            translation: g.feet,
            rotation: Quat::from_rotation_z(g.yaw),
        };
        let clip = &data.clips[g.clip()];
        effect.draw(
            g.time,
            g.scale,
            |at, tag| {
                if let Some(tag) = tag {
                    let local = data.tag(
                        g.clip(),
                        if g.loops() {
                            at.rem_euclid(clip.duration())
                        } else {
                            at
                        },
                        tag,
                    );
                    Transform {
                        translation: transform.point(local.translation * g.scale),
                        rotation: transform.rotation * local.rotation,
                    }
                } else {
                    transform
                }
            },
            |name, at, default| {
                data.events
                    .visual(g.clip(), at, clip.duration(), clip.frame_time, g.loops())
                    .emitters
                    .get(name)
                    .copied()
                    .unwrap_or(default)
            },
            camera,
            atmosphere,
        );
    }
}

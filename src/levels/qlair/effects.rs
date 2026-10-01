//! Saved impact/debris lifetimes; effects continue after their source actor disappears.
use super::*;
use crate::{environment::Atmosphere, weapons::Prop};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Particle {
    model: String,
    at: Vec3,
    velocity: Vec3,
    angles: Vec3,
    spin: Vec3,
    age: f32,
    life: f32,
    scale: f32,
    growth: f32,
    gravity: f32,
    bounce: bool,
}
impl Particle {
    pub fn valid(&self) -> bool {
        self.model.len() < 80
            && !self.model.contains("..")
            && self.at.is_finite()
            && self.at.abs().max_element() < 100000.
            && self.velocity.is_finite()
            && self.velocity.length() < 10000.
            && self.angles.is_finite()
            && self.spin.is_finite()
            && (-1. ..=12.).contains(&self.age)
            && (0. ..=12.).contains(&self.life)
            && (0. ..=10.).contains(&self.scale)
            && self.growth.is_finite()
            && self.gravity.is_finite()
    }
    fn pose(&self) -> Transform {
        Transform {
            translation: self.at,
            rotation: Quat::from_euler(EulerRot::XYZ, self.angles.x, self.angles.y, self.angles.z),
        }
    }
}
fn random(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
    (*seed >> 8) as f32 / 16777216.
}
impl Queen {
    pub(super) fn death_explosions(&mut self) {
        let start = self.data.rigs["c_q2_body"].duration("death_start") + 2.75;
        let frames = self.data.rigs["c_q2_body"].clips["death_1endloop"].frame_time;
        for (i, (offset, clip, tag)) in [
            (0., "death_1endloop", "tag_deathspray01"),
            (10. * frames, "death_1endloop", "tag_deathspray03"),
            (2., "death_2endloop", "tag_deathspray02"),
            (2. + 20. * frames, "death_2endloop", "tag_deathspray04"),
            (4., "death_3endloop", "tag_deathspray03"),
            (4., "death_3endloop", "tag_deathspray05"),
        ]
        .into_iter()
        .enumerate()
        {
            let age = self.saved.time - start - offset;
            if age >= 0. && self.saved.death_effects & (1 << i) == 0 {
                self.saved.death_effects |= 1 << i;
                if age < 5. {
                    let local = offset % 2.;
                    let at = self.data.rigs["c_q2_body"]
                        .tag(tag, clip, local, self.saved.body, false)
                        .translation;
                    self.burst("fx_jackbombexp", at, 1. + i as f32 * 0.3, -age, 5.);
                }
            }
        }
    }
    pub(super) fn burst(&mut self, model: &str, at: Vec3, scale: f32, delay: f32, life: f32) {
        self.saved.debris.push(Particle {
            model: model.into(),
            at,
            velocity: Vec3::ZERO,
            angles: Vec3::ZERO,
            spin: Vec3::ZERO,
            age: -delay,
            life,
            scale,
            growth: 0.,
            gravity: 0.,
            bounce: false,
        });
    }
    pub(super) fn gibs(&mut self, at: Vec3, count: usize, scale: f32, finale: bool) {
        let mut seed = at.x.to_bits() ^ at.y.to_bits().rotate_left(7) ^ self.saved.clock.to_bits();
        for n in 0..count {
            let velocity = vec3(
                random(&mut seed) * 2. - 1.,
                random(&mut seed) * 2. - 1.,
                random(&mut seed),
            ) * if finale { 5000. } else { 350. };
            let spin = (vec3(random(&mut seed), random(&mut seed), random(&mut seed)) * 2.
                - Vec3::ONE)
                * std::f32::consts::PI;
            self.saved.debris.push(Particle {
                model: if finale {
                    "gb_meatbone_q2death".into()
                } else {
                    format!("gb_meatbone{}", n % 3 + 1)
                },
                at,
                velocity,
                angles: Vec3::ZERO,
                spin,
                age: if finale {
                    -5. * self.data.rigs["c_q2_body"].clips["death_endloop"].frame_time
                } else {
                    0.
                },
                life: if finale { 10. } else { 5. },
                scale: scale * (0.2 + 0.8 * random(&mut seed)),
                growth: 0.,
                gravity: 500.,
                bounce: true,
            });
        }
    }
    pub(super) fn final_burst(&mut self) {
        if self.saved.death_burst {
            return;
        }
        self.saved.death_burst = true;
        let rig = &self.data.rigs["c_q2_body"];
        let at = rig
            .tag("bone_spine02", "death_endloop", 0., self.saved.body, false)
            .translation;
        let delay = rig.clips["death_endloop"].frame_time * 5.;
        self.burst("fx_jackbombexp", at, 4., 0., 5.);
        self.burst("fx_hatterheadsplode", at, 8., delay, 0.9);
        self.saved.debris.last_mut().unwrap().growth = 3.5;
        self.gibs(at, 50, 1., true);
        self.gibs(at, 9, 2., false);
    }
    pub(super) fn advance_effects(&mut self, dt: f32, world: &World) {
        self.saved.debris.retain_mut(|p| {
            let previous = p.age;
            p.age += dt;
            if p.age < 0. {
                return true;
            }
            if p.age >= p.life {
                return false;
            }
            let dt = (p.age - previous.max(0.)).max(0.);
            p.velocity.z -= p.gravity * dt;
            let end = p.at + p.velocity * dt;
            if p.bounce {
                let hit = world.sweep(p.at, end, Vec3::splat(2. * p.scale));
                if hit.start_solid {
                    return false;
                }
                p.at = p.at.lerp(end, hit.fraction);
                if hit.fraction < 1. {
                    p.at += hit.normal * 0.1;
                    p.velocity = (p.velocity - 2. * p.velocity.dot(hit.normal) * hit.normal) * 0.6;
                    if hit.normal.z > 0.7 && p.velocity.length() < 30. {
                        p.velocity = Vec3::ZERO;
                        p.spin = Vec3::ZERO;
                    }
                }
            } else {
                p.at = end;
            }
            p.angles += p.spin * dt;
            true
        });
        if self.saved.debris.len() > 192 {
            self.saved.debris.drain(..self.saved.debris.len() - 192);
        }
    }
}

pub(super) struct Art {
    props: BTreeMap<String, Prop>,
    particles: BTreeMap<String, crate::particles::Attached>,
    bursts: BTreeMap<String, crate::particles::Attached>,
    children: BTreeMap<String, Vec<crate::particles::ModelBurst>>,
    textures: BTreeMap<String, Texture2D>,
    _materials: Vec<std::rc::Rc<crate::render_fx::Surface>>,
}
impl Art {
    pub fn load(a: &mut Assets, q: &Queen) -> Result<Self> {
        let materials = crate::texture::read_materials(a)?;
        let mut names = std::collections::BTreeSet::new();
        for spec in q.data.projectiles.values() {
            if let Some(n) = &spec.explosion {
                names.insert(n.trim_end_matches(".tik").to_owned());
            }
        }
        names.extend(
            [
                "fx_jackbombexp",
                "fx_hatterheadsplode",
                "gb_meatbone_q2death",
                "gb_meatbone1",
                "gb_meatbone2",
                "gb_meatbone3",
                "fx_particle_burst",
                "fx_lightning_hit",
            ]
            .into_iter()
            .map(str::to_owned),
        );
        let (mut props, mut particles, mut textures) =
            (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
        let (mut bursts, mut children) = (BTreeMap::new(), BTreeMap::new());
        let mut layers = Vec::new();
        let mut loaded = std::collections::BTreeSet::new();
        while let Some(name) = names.pop_first() {
            if !loaded.insert(name.clone()) {
                continue;
            }
            ensure!(loaded.len() <= 64, "Finale effect dependency limit");
            let text =
                String::from_utf8_lossy(&a.read(&format!("models/{name}.tik"))?).into_owned();
            let def = crate::skeletal::Definition::load(a, &format!("models/{name}.tik"))?;
            let frame = if let Some(file) = def.animations.get("idle") {
                let bytes = a.read(&format!("{}/{file}", def.path))?;
                let b = crate::skeletal::Bytes(&bytes);
                b.float(84)? / b.count(72, 10000)?.max(1) as f32
            } else {
                0.05
            };
            let nested = crate::particles::ModelBurst::parse(&text, frame);
            for child in &nested {
                if !loaded.contains(child.model()) {
                    names.insert(child.model().to_owned());
                }
            }
            if !nested.is_empty() {
                children.insert(name.clone(), nested);
            }
            if let Ok(prop) = Prop::load(a, &name, &materials) {
                props.insert(name.clone(), prop);
            }
            if let Some(fx) = crate::particles::Attached::load(a, &name, &materials)? {
                particles.insert(name.clone(), fx);
            }
            if let Some(fx) = crate::particles::Attached::load_clip_bursts(
                a,
                &name,
                Some("idle"),
                frame,
                &materials,
            )? {
                bursts.insert(name.clone(), fx);
            }
            ensure!(
                props.contains_key(&name)
                    || particles.contains_key(&name)
                    || bursts.contains_key(&name)
                    || children.contains_key(&name),
                "Unrendered finale effect {name}"
            );
        }
        for name in [
            "queenbeam1",
            "queenbeam2",
            "queenbeam3",
            "queenbeam4",
            "textures/special/swipe_queen",
            "boojum_impact_decal",
        ] {
            if let Some(path) = crate::texture::resolve(a, name, &materials) {
                let image = crate::texture::decode(a, &path)?;
                let texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                layers.push(crate::render_fx::register(
                    a,
                    &texture,
                    &[name.to_owned()],
                    &materials,
                )?);
                textures.insert(name.into(), texture);
            }
            ensure!(
                textures.contains_key(name),
                "Unresolved finale trail material {name}"
            );
        }
        Ok(Self {
            props,
            particles,
            bursts,
            children,
            textures,
            _materials: layers,
        })
    }
    fn effect(
        &mut self,
        model: &str,
        pose: Transform,
        age: f32,
        scale: f32,
        emission: f32,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        self.effect_tree(model, pose, age, scale, emission, camera, atmosphere, 1., 0);
    }
    #[allow(clippy::too_many_arguments)]
    fn effect_tree(
        &mut self,
        model: &str,
        pose: Transform,
        age: f32,
        scale: f32,
        emission: f32,
        camera: Vec3,
        atmosphere: &Atmosphere,
        alpha: f32,
        depth: usize,
    ) {
        if depth > 4 || age < 0. || alpha <= 0. {
            return;
        }
        if let Some(prop) = self.props.get_mut(model) {
            for mesh in prop.meshes_at(pose, scale, true, age, false) {
                crate::render_fx::skin_effect(mesh, age, alpha);
            }
        }
        if let Some(fx) = self.bursts.get(model) {
            fx.fork().draw(
                age,
                scale,
                |_, _| pose,
                |_, _, default| default,
                camera,
                atmosphere,
            );
        }
        let children = self
            .children
            .get(model)
            .into_iter()
            .flatten()
            .enumerate()
            .flat_map(|(i, b)| {
                b.sample(
                    age,
                    pose.translation.x.to_bits() ^ pose.translation.y.to_bits() ^ i as u32,
                )
            })
            .collect::<Vec<_>>();
        for child in children {
            let origin = child
                .tag
                .as_ref()
                .and_then(|tag| self.props.get(model).map(|p| p.point(pose, tag, scale)))
                .unwrap_or(pose.translation);
            let p = Transform {
                translation: origin + pose.rotation * child.offset * scale,
                ..pose
            };
            self.effect_tree(
                &child.model,
                p,
                child.age,
                scale * child.scale,
                emission,
                camera,
                atmosphere,
                alpha * child.alpha,
                depth + 1,
            );
        }
        if let Some(fx) = self.particles.get(model) {
            fx.fork().draw(
                age,
                scale,
                |_, _| pose,
                |_, at, default| default && at < emission,
                camera,
                atmosphere,
            );
        }
    }
    fn ribbon(&self, name: &str, a: Vec3, b: Vec3, c: Vec3, d: Vec3, alpha: f32) {
        let Some(texture) = self.textures.get(name) else {
            return;
        };
        let color = Color::new(1., 1., 1., alpha);
        let mesh = Mesh {
            vertices: [
                (a, vec2(0., 0.)),
                (b, vec2(1., 0.)),
                (c, vec2(1., 1.)),
                (d, vec2(0., 1.)),
            ]
            .map(|(p, uv)| Vertex::new(p.x, p.y, p.z, uv.x, uv.y, color))
            .to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: Some(texture.clone()),
        };
        crate::render_fx::skin_effect(&mesh, crate::render_fx::time(), alpha);
    }
    pub fn draw(&mut self, q: &Queen, camera: Vec3, atmosphere: &Atmosphere) {
        for p in &q.saved.debris {
            if p.age >= 0. {
                self.effect(
                    &p.model,
                    p.pose(),
                    p.age,
                    p.scale + p.growth * p.age,
                    2.,
                    camera,
                    atmosphere,
                );
            }
        }
        for impact in &q.saved.impacts {
            let pose = Transform {
                translation: impact.at,
                rotation: Quat::from_rotation_arc(
                    Vec3::Z,
                    impact.normal.try_normalize().unwrap_or(Vec3::Z),
                ),
            };
            self.effect(&impact.model, pose, impact.age, 1., 0.5, camera, atmosphere);
            if let Some(mark) = &impact.mark {
                let r = impact.mark_radius;
                self.ribbon(
                    mark,
                    pose.point(vec3(-r, -r, 0.)),
                    pose.point(vec3(r, -r, 0.)),
                    pose.point(vec3(r, r, 0.)),
                    pose.point(vec3(-r, r, 0.)),
                    (1. - impact.age / 8.).min(0.8),
                );
            }
        }
        for shot in &q.saved.projectiles {
            if let Some(end) = shot.end {
                let side = (camera - shot.at).cross(end - shot.at).normalize_or_zero() * 3.;
                self.ribbon(
                    &shot.model,
                    shot.at - side,
                    end - side,
                    end + side,
                    shot.at + side,
                    1.,
                );
            }
        }
        for (n, tags) in [
            (0, ["b_t01_claw_top01", "tag_claw"]),
            (3, ["bone_t04_05", "Bone50"]),
        ] {
            let (clip, at) = q.part_animation(n);
            if q.saved.phase != Phase::Queen2 || clip != "attack" {
                continue;
            }
            let name = format!("c_q2_t0{}", n + 1);
            let rig = &q.data.rigs[&name];
            let p = q.part_pose(n);
            for i in 0..10 {
                let t = at - i as f32 * 0.05;
                if t < 0.05 {
                    break;
                }
                let a = rig.tag(tags[0], clip, t, p, false).translation;
                let b = rig.tag(tags[1], clip, t, p, false).translation;
                let c = rig.tag(tags[1], clip, t - 0.05, p, false).translation;
                let d = rig.tag(tags[0], clip, t - 0.05, p, false).translation;
                self.ribbon(
                    "textures/special/swipe_queen",
                    a,
                    b,
                    c,
                    d,
                    1. - i as f32 / 10.,
                );
            }
        }
        // The popup's attachment exists from frame zero to frame four; its births live on.
        let popup = if q.saved.phase == Phase::Corridor && q.saved.time < 4. {
            let birth = (q.saved.time / 0.5).floor().min(4.) as usize;
            Some((
                q.data.points[&format!("qlair_popup{}", birth + 1)],
                q.saved.time - birth as f32 * 0.5,
            ))
        } else if q.saved.phase == Phase::Queen1
            && q.saved.attack == battle::Attack::Popup
            && q.saved.attack_time >= 2.25
            && q.saved.fired & 8 == 0
        {
            Some((
                Transform {
                    translation: q.saved.attack_target - Vec3::Z * 32.,
                    rotation: Quat::IDENTITY,
                },
                q.saved.attack_time - 2.25,
            ))
        } else {
            None
        };
        if let Some((pose, age)) = popup {
            self.effect(
                "fx_particle_burst",
                pose,
                age,
                1.,
                4. * q.data.popup_frame,
                camera,
                atmosphere,
            );
        }
        if q.saved.phase == Phase::Queen1
            && q.saved.attack == battle::Attack::Popup
            && (1. ..2.25).contains(&q.saved.attack_time)
            && q.saved.fired & 8 == 0
        {
            self.effect(
                "fx_particle_burst",
                q.popup_pose(),
                q.saved.attack_time - 1.,
                0.3,
                1.25,
                camera,
                atmosphere,
            );
        }
    }
}

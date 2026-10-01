use super::*;
use crate::{npc::Puppet, particles::Attached, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    ui: std::rc::Rc<crate::ui::Ui>,
    actors: BTreeMap<String, Puppet>,
    essence: crate::loot_art::Art,
    acid: Option<Prop>,
    acid_fx: Option<Attached>,
    blood: Option<Attached>,
    spit: Option<Attached>,
    serial: u32,
    mark: Texture2D,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Centipede) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for (name, rig) in &o.data.rigs {
            actors.insert(
                name.clone(),
                Puppet::load(
                    a,
                    name,
                    &rig.clips.keys().map(String::as_str).collect::<Vec<_>>(),
                    &specs,
                )?,
            );
        }
        actors
            .get_mut("c_centipede")
            .unwrap()
            .surface_visible("material4", false);
        let path = crate::texture::resolve(a, "acid_splat_decal", &specs)
            .context("Missing acid impact material")?;
        let image = crate::texture::decode(a, &path)?;
        let mark = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        Ok(Self {
            ui: crate::ui::Ui::load(a)?,
            actors,
            mark,
            essence: crate::loot_art::Art::load(a, &specs)?,
            acid: Some(Prop::load(a, "prj_centipede_spit", &specs)?),
            acid_fx: Attached::load(a, "prj_centipede_spit", &specs)?,
            blood: Attached::load(a, "blood_spray", &specs)?,
            spit: Attached::load_clip_bursts(
                a,
                "c_centipede",
                Some("attack_spit"),
                o.data.boss().frame("attack_spit"),
                &specs,
            )?,
            serial: u32::MAX,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.actors
            .get_mut("alice")
            .unwrap()
            .mouth(s.mouth(&["fakeplayer", "alice"]));
        self.actors
            .get_mut("c_cheshire")
            .unwrap()
            .mouth(s.mouth(&["cat_actor1", "cat"]));
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atm: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Centipede>().unwrap();
        let b = &o.saved.battle;
        for p in self.actors.values_mut() {
            p.atmosphere(atm, camera);
        }
        if matches!(
            o.saved.phase,
            Phase::Intro | Phase::Slide | Phase::Fight | Phase::Drop
        ) {
            let clip = if matches!(o.saved.phase, Phase::Intro | Phase::Slide) {
                "idle_base"
            } else {
                b.clip()
            };
            self.actors.get_mut("c_centipede").unwrap().draw(
                clip,
                b.time,
                b.loops(),
                b.pose,
                1.,
                bright,
            );
        }
        for larva in &b.larvae {
            let pose = Transform {
                translation: larva.feet,
                rotation: Quat::from_rotation_z(larva.yaw),
            };
            self.actors.get_mut("c_larva").unwrap().draw(
                larva.clip(),
                larva.sample_time(&o.data),
                larva.loops(),
                pose,
                larva.visual_scale(&o.data),
                bright,
            );
        }
        if let Some((clip, time, looping, pose, scale)) = o.alice_performance() {
            self.actors
                .get_mut("alice")
                .unwrap()
                .draw(clip, time, looping, pose, scale, bright);
        }
        for (name, base) in &o.data.ants {
            let (clip, t, looping, p) = o.ant_performance(name, *base);
            self.actors
                .get_mut("c_armyant")
                .unwrap()
                .draw(clip, t, looping, p, 1., bright);
        }
        if matches!(
            o.saved.phase,
            Phase::Greeting | Phase::Eat | Phase::Grow | Phase::Done
        ) {
            let cat = self.actors.get_mut("c_cheshire").unwrap();
            let pose = o.data.points["cat_actor1"];
            if o.saved.phase == Phase::Greeting && o.saved.time < 2.5 {
                cat.draw_afterimage(
                    "sit_idle1",
                    o.saved.time,
                    pose,
                    ((o.saved.time - 0.5) / 2.).clamp(0., 1.),
                );
            } else {
                cat.draw("sit_idle1", o.saved.time, true, pose, 1., bright);
            }
        }
        if o.saved.phase == Phase::Fight && o.saved.essence_wait == 0. {
            self.essence.draw(crate::loot::Grade::Medium, o.essence_position(), atm, camera, o.saved.clock);
        }
        if let Some(acid) = &mut self.acid {
            for s in b.shots.iter().filter(|s| s.ended.is_none()) {
                acid.draw_frame(
                    Transform {
                        translation: s.at,
                        rotation: Quat::from_rotation_z(s.age * 250_f32.to_radians()),
                    },
                    if s.fragment { 0.4 } else { 1. },
                    bright,
                    s.age,
                    true,
                );
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atm: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Centipede>().unwrap();
        let b = &o.saved.battle;
        if self.serial != b.serial {
            self.actors.get_mut("c_centipede").unwrap().reset_effects();
            if let Some(p) = &self.spit {
                self.spit = Some(p.fork());
            }
            self.serial = b.serial;
        }
        if o.saved.phase == Phase::Fight {
            self.actors.get_mut("c_centipede").unwrap().draw_effects(
                b.clip(),
                b.time,
                b.loops(),
                b.pose,
                1.,
                camera,
                atm,
            );
            if b.action == battle::Action::Spit {
                if let Some(p) = &mut self.spit {
                    p.draw(
                        b.time,
                        1.,
                        |t, tag| {
                            o.data
                                .boss()
                                .tag(tag.unwrap_or("tag_tongue"), b.clip(), t, b.pose)
                        },
                        |_, _, on| on,
                        camera,
                        atm,
                    );
                }
            }
            if let Some(p) = &mut self.blood {
                p.draw(
                    b.blood_age,
                    1.,
                    |_, _| Transform {
                        translation: b.pain_at,
                        rotation: Quat::IDENTITY,
                    },
                    |_, t, on| on && t < 0.15,
                    camera,
                    atm,
                );
            }
            if let Some(template) = &self.acid_fx {
                for s in &b.shots {
                    let mut p = template.fork();
                    p.draw(
                        s.age,
                        if s.fragment { 0.6 } else { 1.5 },
                        |t, _| {
                            let t = t.min(s.ended.unwrap_or(s.age));
                            let initial = s.velocity
                                + if s.fragment {
                                    Vec3::Z * 80. * s.ended.unwrap_or(s.age)
                                } else {
                                    Vec3::ZERO
                                };
                            Transform {
                                translation: s.start + initial * t
                                    - if s.fragment {
                                        Vec3::Z * 40. * t * t
                                    } else {
                                        Vec3::ZERO
                                    },
                                rotation: Quat::IDENTITY,
                            }
                        },
                        |_, t, on| on && s.ended.is_none_or(|e| t < e),
                        camera,
                        atm,
                    );
                }
            }
            for s in &b.shots {
                if let Some(end) = s.ended {
                    if s.normal.length_squared() < 0.9 {
                        continue;
                    }
                    let radius = if s.fragment { 16. } else { 24. };
                    let x = s.normal.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X) * radius;
                    let y = s.normal.cross(x);
                    let center = s.at - s.normal * (if s.fragment { 8. } else { 16. } - 0.25);
                    let alpha = (1. - (s.age - end)).clamp(0., 1.);
                    let mesh = Mesh {
                        vertices: [
                            (center - x - y, vec2(0., 0.)),
                            (center + x - y, vec2(1., 0.)),
                            (center + x + y, vec2(1., 1.)),
                            (center - x + y, vec2(0., 1.)),
                        ]
                        .map(|(p, uv)| Vertex::new(p.x, p.y, p.z, uv.x, uv.y, WHITE))
                        .to_vec(),
                        indices: vec![0, 1, 2, 0, 2, 3],
                        texture: Some(self.mark.clone()),
                    };
                    crate::render_fx::skin_effect(&mesh, o.saved.clock, alpha);
                }
            }
        }
    }
    fn hud(&mut self, l: &dyn LevelController) {
        let o = l.downcast_ref::<Centipede>().unwrap();
        if o.saved.phase == Phase::Fight && o.saved.battle.hits < 6 {
            // Each of the six accepted weak-point reactions advances the fight.
            crate::hud::boss_meter(
                &self.ui,
                "CENTIPEDE",
                (6 - o.saved.battle.hits) as f32 / 6.,
            );
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

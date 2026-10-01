use super::*;
use crate::{npc::Puppet, particles::Attached, weapons::Prop};
pub struct Art {
    actors: BTreeMap<String, Puppet>,
    weapons: BTreeMap<String, Prop>,
    grenade: Prop,
    essence: crate::loot_art::Art,
    explosion: Option<Attached>,
    bursts: BTreeMap<(usize, u32), Attached>,
    ui: std::rc::Rc<crate::ui::Ui>,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Funhouse) -> Result<Self> {
        let materials = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for (n, r) in &o.data.rigs {
            actors.insert(
                n.clone(),
                Puppet::load(
                    a,
                    n,
                    &r.clips.keys().map(String::as_str).collect::<Vec<_>>(),
                    &materials,
                )?,
            );
        }
        for (name, actor) in &mut actors {
            if name.starts_with("c_tweedle") {
                actor.show_attachments(false);
            }
        }
        let mut weapons = BTreeMap::new();
        for n in ["w_dee_knife", "w_dee_rattle", "w_dum_knife", "w_dum_rattle"] {
            weapons.insert(n.into(), Prop::load(a, n, &materials)?);
        }
        Ok(Self {
            actors,
            weapons,
            grenade: Prop::load(a, "prj_grenade", &materials)?,
            essence: crate::loot_art::Art::load(a, &materials)?,
            explosion: Attached::load(a, "fx_grenade_exp", &materials)?,
            bursts: BTreeMap::new(),
            ui: crate::ui::Ui::load(a)?,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        for (m, n) in [
            ("alice", &["fakeplayer", "alice"][..]),
            ("c_cheshire", &["cat_actor1", "jack_cat"][..]),
            ("c_tweedle_dee", &["dee_actor1"][..]),
            ("c_tweedle_dum", &["dum_actor1"][..]),
            ("c_madhatter", &["hatter_actor1"][..]),
        ] {
            self.actors.get_mut(m).unwrap().mouth(s.mouth(n));
        }
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atm: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Funhouse>().unwrap();
        for a in self.actors.values_mut() {
            a.atmosphere(atm, camera);
        }
        for p in o.performances() {
            let a = self.actors.get_mut(p.model).unwrap();
            if p.alpha < 1. {
                a.draw_afterimage(p.clip, p.time, p.pose, p.alpha);
            } else {
                a.draw(p.clip, p.time, p.looping, p.pose, 1., bright);
            }
        }
        if o.saved
            .scene
            .as_ref()
            .is_none_or(|s| s.kind != Kind::Tweedles)
        {
            for b in o.saved.bosses.iter().chain(&o.saved.minis) {
                self.actors.get_mut(b.model()).unwrap().draw(
                    b.clip(),
                    b.time,
                    b.loops(),
                    b.pose(),
                    1.,
                    bright,
                );
                for g in &b.grenades {
                    if g.ended.is_none() {
                        self.grenade.draw(
                            Transform {
                                translation: g.at,
                                rotation: Quat::from_euler(
                                    EulerRot::XYZ,
                                    g.age * 100f32.to_radians(),
                                    g.age * 200f32.to_radians(),
                                    g.age * 300f32.to_radians(),
                                ),
                            },
                            1.,
                            bright,
                        );
                    }
                }
            }
        }
        if o.saved.essence {
            self.essence.draw(crate::loot::Grade::Medium, o.essence_position(), atm, camera, o.saved.time);
        }
        if o.saved.fight {
            for b in o.saved.bosses.iter().chain(&o.saved.minis) {
                let weapon = match (b.family, b.weapon) {
                    (_, battle::Weapon::None) => continue,
                    (0, battle::Weapon::Knife) => "w_dee_knife",
                    (0, battle::Weapon::Rattle) => "w_dee_rattle",
                    (_, battle::Weapon::Knife) => "w_dum_knife",
                    (_, battle::Weapon::Rattle) => "w_dum_rattle",
                };
                let pose =
                    o.data.rigs[b.model()].tag("tag_sword", b.clip(), b.time, b.pose(), b.loops());
                self.weapons
                    .get_mut(weapon)
                    .unwrap()
                    .draw(pose, b.scale(), bright);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atm: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Funhouse>().unwrap();
        for p in o.performances() {
            self.actors
                .get_mut(p.model)
                .unwrap()
                .draw_effects(p.clip, p.time, p.looping, p.pose, 1., camera, atm);
        }
        let mut live = std::collections::BTreeSet::new();
        if o.saved.fight {
            for b in o.saved.bosses.iter().chain(&o.saved.minis) {
                self.actors.get_mut(b.model()).unwrap().draw_effects(
                    b.clip(),
                    b.time,
                    b.loops(),
                    b.pose(),
                    1.,
                    camera,
                    atm,
                );
                for g in &b.grenades {
                    if let (Some(end), Some(template)) = (g.ended, &self.explosion) {
                        let key = (b.id, g.serial);
                        live.insert(key);
                        let effect = self.bursts.entry(key).or_insert_with(|| template.fork());
                        effect.draw(
                            g.age - end,
                            1.,
                            |_, _| Transform {
                                translation: g.at,
                                rotation: Quat::IDENTITY,
                            },
                            |_, _, on| on,
                            camera,
                            atm,
                        );
                    }
                }
            }
        }
        self.bursts.retain(|k, _| live.contains(k));
    }
    fn hud(&mut self, l: &dyn LevelController) {
        let o = l.downcast_ref::<Funhouse>().unwrap();
        if !o.saved.fight {
            return;
        }
        for (i, b) in o.saved.bosses.iter().enumerate() {
            let x = screen_width() * 0.5 - 220. + i as f32 * 240.;
            self.ui.dialog(Rect::new(x - 12., 12., 214., 48.));
            self.ui.label(
                if i == 0 { "TWEEDLEDEE" } else { "TWEEDLEDUM" },
                x,
                31.,
                18.,
                WHITE,
            );
            draw_rectangle(
                x,
                40.,
                190. * b.health / if i == 0 { 800. } else { 900. },
                7.,
                Color::new(0.65, 0.16, 0.21, 0.9),
            );
        }
    }
    fn handoff_pose(&self) -> Option<&crate::cinematic::ActorPose> {
        self.actors["alice"].handoff_pose()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

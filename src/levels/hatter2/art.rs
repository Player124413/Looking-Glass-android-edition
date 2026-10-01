use super::*;
use crate::{npc::Puppet, particles::Attached, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    actors: BTreeMap<String, Puppet>,
    adds: Vec<Puppet>,
    essence: crate::loot_art::Art,
    blade: Prop,
    cup: Prop,
    syringe: Prop,
    fist: Prop,
    altar: Option<Attached>,
    cup_fx: Option<Attached>,
    syringe_fx: Option<Attached>,
    splash: Option<Attached>,
    drop: Prop,
    splash_mesh: Prop,
    shots: BTreeMap<u32, Attached>,
    ui: std::rc::Rc<crate::ui::Ui>,
    boss_action: Option<battle::Action>,
    boss_time: f32,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Encounter) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for (n, r) in &o.data.rigs {
            if n == "c_clockwork" {
                continue;
            }
            actors.insert(
                n.clone(),
                Puppet::load(
                    a,
                    n,
                    &r.clips.keys().map(String::as_str).collect::<Vec<_>>(),
                    &specs,
                )?,
            );
        }
        let mut adds = vec![];
        for _ in 0..2 {
            adds.push(Puppet::load(
                a,
                "c_clockwork",
                crate::clockwork::CLIPS,
                &specs,
            )?);
        }
        let mut altar = Attached::load(a, "altar_eyestaff_blade", &specs)?;
        if let Some(fx) = &mut altar {
            fx.attenuate(0.12);
        }
        // The source timer has 100 frames over five seconds and no render surfaces.
        Ok(Self {
            actors,
            adds,
            essence: crate::loot_art::Art::load(a, &specs)?,
            blade: Prop::load(a, "a_eyestaff_blade", &specs)?,
            cup: Prop::load(a, "prj_teacup", &specs)?,
            syringe: Prop::load(a, "prj_syringe", &specs)?,
            fist: Prop::load(a, "prj_hand", &specs)?,
            altar,
            cup_fx: Attached::load(a, "prj_teacup", &specs)?,
            syringe_fx: Attached::load(a, "prj_syringe", &specs)?,
            splash: Attached::load_clip_bursts(a, "fx_hatterexp", Some("splash"), 0.05, &specs)?,
            drop: Prop::load(a, "fx_hatterdrop", &specs)?,
            splash_mesh: Prop::load(a, "fx_hattersplash", &specs)?,
            shots: BTreeMap::new(),
            ui: crate::ui::Ui::load(a)?,
            boss_action: None,
            boss_time: 0.,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        for (m, n) in [
            ("c_gryphon", &["gryphon_actor1"][..]),
            ("c_cheshire", &["hatter_cat", "watch_cat"][..]),
            ("alice", &["fakeplayer", "alice"][..]),
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
        let o = l.downcast_ref::<Encounter>().unwrap();
        let s = &o.saved;
        for a in self.actors.values_mut() {
            a.atmosphere(atm, camera);
        }
        for p in o.performances() {
            let actor = self.actors.get_mut(p.model).unwrap();
            if p.model == "c_cheshire" {
                let alpha = if s.talk_done {
                    (1. - (s.after_talk - 1.) / 2.).clamp(0., 1.)
                } else {
                    (s.time / 2.).min(1.)
                };
                actor.draw_dissolving(p.clip, p.time, p.loops, p.pose, p.scale, bright, 1. - alpha);
            } else {
                actor.draw(p.clip, p.time, p.loops, p.pose, p.scale, bright);
            }
        }
        for (i, w) in s.waves.iter().enumerate() {
            if let Some(p) = w.actor.visual(&o.data) {
                self.adds[i].atmosphere(atm, camera);
                self.adds[i].draw(w.actor.clip(), w.actor.time, w.actor.loops(), p, 1., bright);
            }
            for f in &w.actor.fists {
                self.fist.draw(
                    Transform {
                        translation: f.position,
                        rotation: Quat::from_rotation_arc(Vec3::X, f.direction),
                    },
                    1.,
                    bright,
                );
            }
        }
        if !s.blade {
            self.blade.draw_frame(
                Transform {
                    translation: o.data.blade,
                    rotation: Quat::from_rotation_z(s.clock),
                },
                1.,
                bright,
                s.clock,
                true,
            );
        }
        if s.essence_live {
            self.essence.draw(crate::loot::Grade::Medium, o.essence_position(), atm, camera, s.clock);
        }
        for (at, age) in &s.drops {
            let grade = if *age < 10. { crate::loot::Grade::Large } else if *age < 30. { crate::loot::Grade::Medium } else { crate::loot::Grade::Small };
            self.essence.draw(grade, *at, atm, camera, s.clock);
        }
        for shot in &s.boss.shots {
            if shot.ended.is_none() {
                let prop = if shot.syringe {
                    &mut self.syringe
                } else {
                    &mut self.cup
                };
                prop.draw(
                    Transform {
                        translation: shot.at,
                        rotation: if shot.syringe {
                            Quat::from_rotation_arc(Vec3::X, shot.velocity.normalize_or_zero())
                        } else {
                            Quat::from_rotation_z(shot.age * 200_f32.to_radians())
                        },
                    },
                    1.,
                    bright,
                );
            }
        }
        let t = s.cycle as f32 / 120. - 72.;
        if s.phase == Phase::Fight && (0. ..3.).contains(&t) && !s.boss.dying() {
            let mut p = o.data.points["hatter_splash"];
            if t < 2.5 {
                p.translation.z += if t < 1. {
                    350.
                } else {
                    300. - 150. * (t - 1.).powi(2)
                };
                self.drop
                    .draw_frame(p, if t < 1. { 0.1 + 2. * t } else { 1. }, bright, t, false);
            } else {
                self.splash_mesh.draw_frame(p, 5., bright, t - 2.5, false);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atm: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Encounter>().unwrap();
        let s = &o.saved;
        if self.boss_action != Some(s.boss.action) || s.boss.time < self.boss_time {
            self.actors.get_mut("c_madhatter").unwrap().reset_effects();
        }
        self.boss_action = Some(s.boss.action);
        self.boss_time = s.boss.time;
        for p in o.performances() {
            if p.model != "c_cheshire" {
                self.actors
                    .get_mut(p.model)
                    .unwrap()
                    .draw_effects(p.clip, p.time, p.loops, p.pose, p.scale, camera, atm);
            }
        }
        for (i, w) in s.waves.iter().enumerate() {
            if let Some(p) = w.actor.visual(&o.data) {
                self.adds[i].draw_effects(
                    w.actor.clip(),
                    w.actor.time,
                    w.actor.loops(),
                    p,
                    1.,
                    camera,
                    atm,
                );
            }
        }
        self.shots
            .retain(|id, _| s.boss.shots.iter().any(|s| s.id == *id));
        for shot in &s.boss.shots {
            let template = if shot.syringe {
                &self.syringe_fx
            } else {
                &self.cup_fx
            };
            if let Some(template) = template {
                let fx = self.shots.entry(shot.id).or_insert_with(|| template.fork());
                fx.draw(
                    shot.age,
                    1.,
                    |t, _| Transform {
                        translation: shot.at - shot.velocity * (shot.age - t).clamp(0., 0.1),
                        rotation: Quat::IDENTITY,
                    },
                    |_, t, on| on && shot.ended.is_none_or(|end| t < end),
                    camera,
                    atm,
                );
            }
        }
        if !s.blade {
            if let Some(fx) = &mut self.altar {
                fx.draw(
                    s.clock,
                    1.,
                    |_, _| Transform {
                        translation: o.data.blade,
                        rotation: Quat::IDENTITY,
                    },
                    |_, _, on| on,
                    camera,
                    atm,
                );
            }
        }
        let t = s.cycle as f32 / 120. - 72.;
        if s.phase == Phase::Fight && (0. ..5.).contains(&t) && !s.boss.dying() {
            if let Some(fx) = &mut self.splash {
                fx.draw(
                    t,
                    1.,
                    |_, _| o.data.points["hatter_splash"],
                    |_, _, on| on,
                    camera,
                    atm,
                );
            }
        }
    }
    fn hud(&mut self, l: &dyn LevelController) {
        let o = l.downcast_ref::<Encounter>().unwrap();
        if matches!(o.saved.phase, Phase::Arrival | Phase::Cat | Phase::Fight) {
            let w = 240.;
            let x = (screen_width() - w) * 0.5;
            self.ui.dialog(Rect::new(x - 22., 12., w + 44., 48.));
            self.ui.label("MAD HATTER", x, 30., 18., WHITE);
            draw_rectangle(
                x,
                39.,
                w * (o.saved.boss.health / 2600.),
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

use super::*;
use crate::{npc::Puppet, particles::Attached, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    ui: std::rc::Rc<crate::ui::Ui>,
    eye_beam: crate::weapons::EnemyBeam,
    beams: std::rc::Rc<crate::npc::attack_fx::Art>,
    actors: BTreeMap<String, Puppet>,
    essence: crate::loot_art::Art,
    eye: Prop,
    altar: Option<Attached>,
    breath: Option<Attached>,
    spiral: Option<Attached>,
    shots: BTreeMap<u32, Attached>,
    phase: Option<battle::Action>,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Encounter) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for (name, r) in &o.data.rigs {
            actors.insert(
                name.clone(),
                Puppet::load(
                    a,
                    name,
                    &r.clips.keys().map(String::as_str).collect::<Vec<_>>(),
                    &specs,
                )?,
            );
        }
        Ok(Self {
            ui: crate::ui::Ui::load(a)?,
            eye_beam: crate::weapons::EnemyBeam::load(a, &specs)?,
            beams: crate::npc::attack_fx::Art::load(a, &specs)?,
            actors,
            essence: crate::loot_art::Art::load(a, &specs)?,
            eye: Prop::load(a, "a_eyestaff_eye", &specs)?,
            altar: Attached::load(a, "altar_eyestaff_eye", &specs)?,
            breath: Attached::load(a, "prj_jabberwock_breath", &specs)?,
            spiral: Attached::load(a, "fx_eyestaff_spiral", &specs)?,
            shots: BTreeMap::new(),
            phase: None,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        for (model, names) in [
            ("alice", &["fakeplayer", "alice"][..]),
            ("c_jabberwock", &["jabber_actor1", "jabberwock", "j"][..]),
            ("c_gryphon", &["gryphon_actor1", "gryphon_dying", "g"][..]),
        ] {
            self.actors.get_mut(model).unwrap().mouth(s.mouth(names));
        }
        if let Some(cat) = self.actors.get_mut("c_cheshire") {
            cat.mouth(s.mouth(&["cat"]));
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
        let lost = s.kind == Kind::Grounds
            || (s.phase == Phase::Outro && s.time >= 3.95)
            || matches!(s.phase, Phase::Reward | Phase::Done);
        let boss = self.actors.get_mut("c_jabberwock").unwrap();
        boss.surface_visible("material11", !lost);
        boss.surface_visible("material12", lost);
        for n in ["material7", "material8"] {
            boss.surface_visible(
                n,
                s.phase != Phase::Death || s.boss.time < 3. * o.data.boss().frame("death"),
            );
        }
        for n in ["material9", "material10"] {
            boss.surface_visible(
                n,
                s.phase != Phase::Death || s.boss.time < 37. * o.data.boss().frame("death"),
            );
        }
        for p in o.performances() {
            if p.model == "c_cheshire" {
                let t = s.cat.unwrap_or(0.);
                let alpha = if s.cat_ending {
                    (1. - t / 2.).clamp(0., 1.)
                } else {
                    (t / 2.).clamp(0., 1.)
                };
                self.actors
                    .get_mut(p.model)
                    .unwrap()
                    .draw_afterimage(p.clip, p.time, p.pose, alpha);
                continue;
            }
            self.actors
                .get_mut(p.model)
                .unwrap()
                .draw(p.clip, p.time, p.loops, p.pose, 1., bright);
        }
        for w in &s.waves {
            let g = &w.actor;
            self.actors.get_mut("c_jabberspawn").unwrap().draw(
                g.clip(),
                g.sample_time(&o.data),
                g.loops(),
                Transform {
                    translation: g.feet,
                    rotation: Quat::from_rotation_z(g.yaw),
                },
                g.visual_scale(&o.data),
                bright,
            );
        }
        if s.phase == Phase::Fight && s.essence_wait == 0. {
            self.essence.draw(crate::loot::Grade::Medium, o.essence_position(), atm, camera, s.clock);
        }
        if o.eye_visible() {
            self.eye
                .draw_frame(o.data.points["eye_altar"], 1., bright, s.clock, true);
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atm: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Encounter>().unwrap();
        let b = &o.saved.boss;
        if self.phase != Some(b.action) {
            self.actors.get_mut("c_jabberwock").unwrap().reset_effects();
            self.phase = Some(b.action);
        }
        for p in o.performances() {
            self.actors
                .get_mut(p.model)
                .unwrap()
                .draw_effects(p.clip, p.time, p.loops, p.pose, 1., camera, atm);
        }
        self.shots
            .retain(|id, _| b.shots.iter().any(|s| s.id == *id));
        for s in &b.shots {
            let template = if s.spiral { &self.spiral } else { &self.breath };
            let Some(template) = template else { continue };
            let fx = self.shots.entry(s.id).or_insert_with(|| template.fork());
            fx.draw(
                s.age,
                1.,
                |at, _| Transform {
                    translation: if s.ended.is_some() {
                        s.at
                    } else {
                        s.at - s.velocity * (s.age - at).clamp(0., 0.1)
                    },
                    rotation: Quat::IDENTITY,
                },
                |_, at, on| on && s.ended.is_none_or(|end| at < end),
                camera,
                atm,
            );
        }
        if o.eye_visible() {
            if let Some(fx) = &mut self.altar {
                fx.draw(
                    o.saved.clock,
                    0.35,
                    |_, _| o.data.points["eye_altar"],
                    |_, _, on| on,
                    camera,
                    atm,
                );
            }
        }
        for w in &o.saved.waves {
            let g = &w.actor;
            if let Some(end) = g.beam {
                let from = o.data.rigs["c_jabberspawn"].tag("tag_beam", g.clip(), g.time,
                    Transform { translation: g.feet, rotation: Quat::from_rotation_z(g.yaw) }, g.loops()).translation;
                self.beams.lightning(from, end, camera, (g.time-0.05).max(0.), 1000., false, atm);
            }
        }
        if let Some((start, end)) = b.beam {
            self.eye_beam.draw(start, end, b.time, camera);
        }
    }
    fn hud(&mut self, l: &dyn LevelController) {
        let o = l.downcast_ref::<Encounter>().unwrap();
        if o.saved.phase != Phase::Fight {
            return;
        }
        if o.saved.kind == Kind::Lair {
            // The invulnerable first encounter is won by surviving 90 seconds.
            let remaining = 10800_u32.saturating_sub(o.saved.ticks) as f32 / 120.;
            crate::hud::boss_meter(
                &self.ui,
                &format!("JABBERWOCK / SURVIVE {:.0}s", remaining.ceil()),
                remaining / 90.,
            );
        } else if o.saved.boss.health > 0. {
            crate::hud::boss_meter(&self.ui, "JABBERWOCK", o.saved.boss.health / 2000.);
        }
    }
    fn handoff_pose(&self) -> Option<&crate::cinematic::ActorPose> {
        self.actors.get("alice").and_then(Puppet::handoff_pose)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

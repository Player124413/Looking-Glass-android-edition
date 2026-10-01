use super::*;
use crate::{npc::Puppet, particles::Attached, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    beams: std::rc::Rc<crate::npc::attack_fx::Art>,
    actors: BTreeMap<String, Puppet>,
    pieces: Vec<Puppet>,
    essence: crate::loot_art::Art,
    ball: Prop,
    diamond: Prop,
    ball_fx: Option<Attached>,
    diamond_fx: Option<Attached>,
    blast: Option<Attached>,
    appear: Option<Attached>,
    shots: BTreeMap<u32, (Option<Attached>, Option<Attached>)>,
    fires: BTreeMap<String, Attached>,
    last: Option<(battle::Action, f32)>,
    ui: std::rc::Rc<crate::ui::Ui>,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Encounter) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for (n, m) in o
            .data
            .cast
            .iter()
            .map(|c| (c.name.as_str(), c.model.as_str()))
            .chain([("r_king", "c_chess_red_king"), ("alice", "alice")])
        {
            let r = &o.data.rigs[m];
            actors.insert(
                n.into(),
                Puppet::load(
                    a,
                    m,
                    &r.clips.keys().map(String::as_str).collect::<Vec<_>>(),
                    &specs,
                )?,
            );
        }
        actors
            .get_mut("w_queen_head")
            .context("Missing queen head")?
            .surface_visible("body", false);
        actors
            .get_mut("w_queen_body")
            .context("Missing queen body")?
            .surface_visible("head", false);
        let mut pieces = vec![];
        for (_, p) in &o.data.pieces {
            pieces.push(Puppet::load(a, p.kind.model(), p.kind.clips(), &specs)?);
        }
        let mut appear = Attached::load(a, "fx_pickup", &specs)?;
        if let Some(a) = &mut appear {
            a.attenuate(0.3);
        }
        Ok(Self {
            beams: crate::npc::attack_fx::Art::load(a, &specs)?,
            actors,
            pieces,
            essence: crate::loot_art::Art::load(a, &specs)?,
            ball: Prop::load(a, "prj_kingball", &specs)?,
            diamond: Prop::load(a, "prj_diamond", &specs)?,
            ball_fx: Attached::load(a, "prj_kingball", &specs)?,
            diamond_fx: Attached::load(a, "prj_diamond", &specs)?,
            blast: Attached::load_clip_bursts(a, "fx_kingball_exp", Some("idle"), 0.05, &specs)?,
            appear,
            shots: BTreeMap::new(),
            fires: BTreeMap::new(),
            last: None,
            ui: crate::ui::Ui::load(a)?,
        })
    }
}
impl LevelArt for Art {
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atm: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Encounter>().unwrap();
        for p in o.performances() {
            let a = self.actors.get_mut(&p.name).unwrap();
            a.atmosphere(atm, camera);
            a.draw(p.clip, p.time, p.loops, p.pose, p.scale, bright);
        }
        for ((_, p), art) in o.saved.pieces.iter().zip(&mut self.pieces) {
            if !p.active || p.suppressed {
                continue;
            }
            art.atmosphere(atm, camera);
            art.draw(
                p.clip(),
                if p.frozen { p.time.min(0.25) } else { p.time },
                p.loops(),
                Transform {
                    translation: p.feet,
                    rotation: Quat::from_rotation_z(p.yaw),
                },
                p.visual_scale(&o.data),
                bright,
            );
        }
        if o.saved.phase == Phase::Fight && o.saved.essence_wait == 0. {
            self.essence.draw(crate::loot::Grade::Medium, o.essence_position(), atm, camera, o.saved.clock);
        }
        for s in &o.saved.boss.shots {
            if s.ended.is_some() {
                continue;
            }
            let p = Transform {
                translation: s.at,
                rotation: Quat::from_rotation_arc(Vec3::X, s.velocity.normalize_or_zero()),
            };
            if s.kind == battle::Kind::Diamond {
                self.diamond.draw_frame(p, 1., bright, s.age, true);
            } else {
                self.ball.draw_frame(p, 1., bright, s.age, true);
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
        let b = &o.saved.boss;
        if self.last.is_none_or(|(a, t)| a != b.action || b.time < t) {
            self.actors.get_mut("r_king").unwrap().reset_effects();
        }
        self.last = Some((b.action, b.time));
        for p in o.performances() {
            self.actors
                .get_mut(&p.name)
                .unwrap()
                .draw_effects(p.clip, p.time, p.loops, p.pose, p.scale, camera, atm);
            if p.scale < 1.
                && (p.name.starts_with("king_")
                    || (p.name == "r_king" && o.saved.phase == Phase::Intro)
                    || p.name == "revived_queen")
            {
                if let Some(template) = &self.appear {
                    self.fires
                        .entry(p.name)
                        .or_insert_with(|| template.fork())
                        .draw(p.scale, 1., |_, _| p.pose, |_, _, on| on, camera, atm);
                }
            }
        }
        for ((_, p), art) in o.saved.pieces.iter().zip(&mut self.pieces) {
            if !p.active || p.suppressed {
                continue;
            }
            let at = Transform {
                translation: p.feet,
                rotation: Quat::from_rotation_z(p.yaw),
            };
            art.draw_effects(
                p.clip(),
                p.time,
                p.loops(),
                at,
                p.visual_scale(&o.data),
                camera,
                atm,
            );
            if let Some(b) = &p.beam {
                self.beams.lightning(b.from, b.to, camera, b.age, 500., true, atm);
            }
        }
        self.shots
            .retain(|id, _| b.shots.iter().any(|s| s.id == *id));
        for s in &b.shots {
            let template = if s.kind == battle::Kind::Diamond {
                &self.diamond_fx
            } else {
                &self.ball_fx
            };
            let (trail, blast) = self.shots.entry(s.id).or_insert_with(|| {
                (
                    template.as_ref().map(Attached::fork),
                    self.blast.as_ref().map(Attached::fork),
                )
            });
            if let Some(f) = trail {
                f.draw(
                    s.age,
                    1.,
                    |t, _| Transform {
                        translation: s.at - s.velocity * (s.age - t).clamp(0., 0.1),
                        rotation: Quat::IDENTITY,
                    },
                    |_, t, on| on && s.ended.is_none_or(|end| t < end),
                    camera,
                    atm,
                );
            }
            if let Some(end) = s.ended {
                if s.kind != battle::Kind::Diamond {
                    if let Some(f) = blast {
                        f.draw(
                            s.age - end,
                            1.,
                            |_, _| Transform {
                                translation: s.at,
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
        for b in &b.beams {
            self.beams.lightning(b.from, b.to, camera, b.age, 500., true, atm);
        }
    }

    fn hud(&mut self, l: &dyn LevelController) {
        let o = l.downcast_ref::<Encounter>().unwrap();
        if matches!(o.saved.phase, Phase::Intro | Phase::Fight) {
            let w = 240.;
            let x = (screen_width() - w) * 0.5;
            self.ui.dialog(Rect::new(x - 22., 12., w + 44., 48.));
            self.ui.label("RED KING", x, 30., 18., WHITE);
            draw_rectangle(
                x,
                39.,
                w * o.saved.boss.health / 1300.,
                7.,
                Color::new(0.7, 0.05, 0.1, 1.),
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

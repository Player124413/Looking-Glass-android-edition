use super::*;
use crate::{npc::Puppet, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    beams: std::rc::Rc<crate::npc::attack_fx::Art>,
    actors: BTreeMap<String, Puppet>,
    alice: Puppet,
    bishop: Prop,
    knight: Prop,
    lever: Prop,
    lever_pull: Prop,
    sparkles: crate::particles::Attached,
}
impl Art {
    pub fn load(a: &mut Assets, r: &Realm) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for c in &r.saved.cast {
            if !actors.contains_key(&c.model) {
                let mut clips = c.piece.kind.clips().to_vec();
                if c.piece.kind == crate::chess::Kind::Rook {
                    clips.push("twitchc");
                }
                actors.insert(c.model.clone(), Puppet::load(a, &c.model, &clips, &specs)?);
            }
        }
        Ok(Self {
            beams: crate::npc::attack_fx::Art::load(a, &specs)?,
            actors,
            alice: Puppet::load(a, "alice", &["idle", "walk", "run", "use_lever"], &specs)?,
            bishop: Prop::load(a, "bishop", &specs)?,
            knight: Prop::load(a, "knight", &specs)?,
            lever: Prop::load_animation(a, "lever", "start", &specs)?,
            lever_pull: Prop::load_animation(a, "lever", "move", &specs)?,
            sparkles: crate::particles::Attached::load_clip_bursts(
                a,
                "fx_pickup",
                Some("on"),
                0.05,
                &specs,
            )?
            .context("Missing chess transformation emitter")?,
        })
    }
}
impl LevelArt for Art {
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let r = l.downcast_ref::<Realm>().unwrap();
        for a in &r.saved.cast {
            if !a.piece.active {
                continue;
            }
            let scale = a.piece.visual_scale(&r.data);
            if scale <= 0. {
                continue;
            }
            let puppet = self.actors.get_mut(&a.model).unwrap();
            puppet.atmosphere(atmo, camera);
            let scene_walk = r.saved.scene.as_ref().is_some_and(|s| {
                let q = r.scene_times(s);
                let t = s.clock.time;
                match (s.kind, a.name.as_str()) {
                    (scene::Kind::Intro, "rook_guard1") => {
                        (q[0]..q[1]).contains(&t) || (q[1] + 0.8..q[2]).contains(&t)
                    }
                    (scene::Kind::Bishop, "bishop_instructor1") => (q[0]..q[1]).contains(&t),
                    (scene::Kind::Knight, "knight_instructor1") => (q[0] + 1. ..q[2]).contains(&t),
                    (scene::Kind::Bell, "rook_guard1") => (q[0]..q[1]).contains(&t),
                    _ => false,
                }
            });
            let refusal = r.saved.refusal.filter(|t| {
                a.name == "rook_guard1"
                    && r.saved.age - t
                        < crate::ant::Timing::duration(&r.data, "c_chess_red_rook", "twitchc")
            });
            let clip = if refusal.is_some() {
                "twitchc"
            } else if a.walk.is_some() || scene_walk {
                if a.piece.kind == crate::chess::Kind::Pawn {
                    "walk"
                } else {
                    "walk_1"
                }
            } else {
                a.piece.clip()
            };
            puppet.draw(
                clip,
                refusal.map_or(a.piece.time, |t| r.saved.age - t),
                refusal.is_none() && (a.walk.is_some() || a.piece.loops()),
                Transform {
                    translation: a.piece.feet,
                    rotation: Quat::from_rotation_z(a.piece.yaw),
                },
                scale,
                bright,
            );
            if let Some(b) = &a.piece.beam {
                self.beams.lightning(b.from, b.to, camera, b.age, 500., true, atmo);
            }
        }
        if let Some((pose, scale, walk)) = r.alice_scene_pose() {
            self.alice.atmosphere(atmo, camera);
            self.alice.draw(
                if walk { "walk" } else { "idle" },
                r.saved.scene.as_ref().unwrap().clock.time,
                true,
                pose,
                scale,
                bright,
            );
        }
        if let Some((_, t)) = r.saved.pull {
            self.alice.atmosphere(atmo, camera);
            self.alice
                .draw("use_lever", t, false, r.player_pose, 1., bright);
        }
        if let Some(b) = &r.saved.board {
            let prop = if b.piece == Piece::Bishop {
                &mut self.bishop
            } else {
                &mut self.knight
            };
            // These TANs are authored below local Z zero. The original attaches
            // them to Alice's rotated origin bone, not directly to world feet.
            let pose = self
                .alice
                .tag("ORIGIN", "idle", 0., r.player_pose, 1.)
                .expect("Validated Alice origin");
            prop.draw(pose, 1., bright);
        }
        if let Some(s) = &r.saved.scene {
            if matches!(s.kind, scene::Kind::Bishop | scene::Kind::Knight) {
                let t = s.clock.time - r.scene_times(s)[3] - 0.5;
                if t >= 0. {
                    let (model, marker, clip) = if s.kind == scene::Kind::Bishop {
                        ("c_chess_bishop", "bishop_grow", "idlea")
                    } else {
                        ("c_chess_knight", "knight_grow", "idle")
                    };
                    let puppet = self.actors.get_mut(model).unwrap();
                    puppet.atmosphere(atmo, camera);
                    puppet.draw(
                        clip,
                        t,
                        true,
                        r.data.point(marker),
                        (t * 2.).clamp(0.1, 1.),
                        bright,
                    );
                }
            }
        }
        for (i, n) in ["bell_lever", "water_lever"].into_iter().enumerate() {
            let started = r.saved.lever_started[i];
            if let Some(t) = started {
                self.lever_pull
                    .draw_frame(r.data.point(n), 1., bright, r.saved.age - t, false);
            } else {
                self.lever.draw(r.data.point(n), 1., bright);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmo: &crate::environment::Atmosphere,
    ) {
        let r = l.downcast_ref::<Realm>().unwrap();
        let Some(s) = &r.saved.scene else {
            return;
        };
        if !matches!(s.kind, scene::Kind::Bishop | scene::Kind::Knight) {
            return;
        }
        let t = s.clock.time - r.scene_times(s)[3];
        if !(0. ..3.).contains(&t) {
            return;
        }
        let pose = r.alice_scene_pose().unwrap().0;
        self.sparkles
            .draw(t, 2., |_, _| pose, |_, age, _| age < 1., camera, atmo);
    }
    fn handoff_pose(&self) -> Option<&crate::cinematic::ActorPose> {
        self.alice.handoff_pose()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

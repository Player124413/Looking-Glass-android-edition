use super::*;
use crate::{ant::Timing, npc::Puppet, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    beams: std::rc::Rc<crate::npc::attack_fx::Art>,
    actors: BTreeMap<String, Puppet>,
    alice: Puppet,
    king: Puppet,
    queen: Puppet,
    rook: Puppet,
    pawn: Puppet,
    sceptre: Prop,
    ball: Prop,
    dead: Vec<(String, Transform, f32)>,
}
impl Art {
    pub fn load(a: &mut Assets, r: &Castle) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut actors = BTreeMap::new();
        for c in &r.saved.cast {
            if !actors.contains_key(&c.model) {
                actors.insert(
                    c.model.clone(),
                    Puppet::load(a, &c.model, c.piece.kind.clips(), &specs)?,
                );
            }
        }
        let map = Bsp::parse(&a.read("maps/wchess2.bsp")?)?;
        let dead = map
            .entities
            .iter()
            .filter(|e| e.get("targetname").is_some_and(|n| n.starts_with("dead_")))
            .map(|e| {
                (
                    e["model"].trim_end_matches(".tik").to_string(),
                    data::at(e),
                    match e["targetname"].as_str() {
                        "dead_rook" => 32.,
                        "dead_knight" => 24.,
                        _ => 22.,
                    },
                )
            })
            .collect();
        Ok(Self {
            beams: crate::npc::attack_fx::Art::load(a, &specs)?,
            actors,
            alice: Puppet::load(a, "alice", ALICE, &specs)?,
            king: Puppet::load(a, "c_chess_king", KING, &specs)?,
            queen: Puppet::load(a, "c_chess_queen", &["idle_base", "dragging"], &specs)?,
            rook: Puppet::load(a, "c_chess_red_rook", &["idle", "walk"], &specs)?,
            pawn: Puppet::load(a, "c_chess_pawn", &["idle", "pain1", "walk"], &specs)?,
            sceptre: Prop::load(a, "w_kingsceptre", &specs)?,
            ball: Prop::load(a, "w_kingball", &specs)?,
            dead,
        })
    }
}
fn watch(p: &mut Puppet, at: Transform, target: Vec3) {
    let mut w = crate::facial::Watch::default();
    w.update(
        10.,
        Some(at.rotation.conjugate() * (target - at.translation)),
    );
    p.watch(w);
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.alice.mouth(s.mouth(&["alice", "fakeplayer"]));
        self.king.mouth(s.mouth(&["w_king"]));
        self.alice.mouth_angle(45.);
        self.king.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let r = l.downcast_ref::<Castle>().unwrap();
        let time = r.saved.scene.as_ref().map_or(r.saved.age, |s| s.time);
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
            let clip = if a.walk.is_some() {
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
                a.piece.time,
                a.walk.is_some() || a.piece.loops(),
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
        if !r.saved.queen {
            let time = r
                .saved
                .scene
                .as_ref()
                .filter(|s| s.kind == scene::Kind::Queen)
                .map_or(0., |s| s.time);
            let parent = r.queen_parent();
            let base = r.data.point("w_queen_parent");
            for (name, model) in [
                ("w_queen", "c_chess_queen"),
                ("r_rook_abductors", "c_chess_red_rook"),
            ] {
                // Both abductors share a targetname, so retain every original binding.
                for at in r
                    .data
                    .bound
                    .iter()
                    .filter(|(n, _)| n == name)
                    .map(|(_, p)| p)
                {
                    let pose = Transform {
                        translation: parent.point(at.translation - base.translation),
                        rotation: parent.rotation * at.rotation,
                    };
                    let puppet = if model == "c_chess_queen" {
                        &mut self.queen
                    } else {
                        &mut self.rook
                    };
                    puppet.atmosphere(atmo, camera);
                    puppet.draw(
                        if time < 1. {
                            if name == "w_queen" {
                                "idle_base"
                            } else {
                                "idle"
                            }
                        } else if name == "w_queen" {
                            "dragging"
                        } else {
                            "walk"
                        },
                        (time - 1.).max(0.),
                        true,
                        pose,
                        1.,
                        bright,
                    );
                }
            }
            for (model, pose, frame) in &self.dead {
                let p = self.actors.get_mut(model).unwrap();
                p.atmosphere(atmo, camera);
                p.draw(
                    "death1",
                    frame
                        * r.data
                            .frame(&model.replace("c_chess_", "c_chess_red_"), "death1"),
                    false,
                    *pose,
                    1.,
                    bright,
                );
            }
        }
        let (king, moving) = r.king_pose();
        self.king.atmosphere(atmo, camera);
        watch(
            &mut self.king,
            king,
            r.data.point("alice_king_dest2").translation + Vec3::Z * 64.,
        );
        let clip = if moving { "walk" } else { "idle" };
        self.king.draw(clip, time, true, king, 1., bright);
        if let Some(t) = self.king.tag("tag_weapon", clip, time, king, 1.) {
            self.sceptre.draw(t, 1., bright);
        }
        if let Some(t) = self.king.tag("tag_ball", clip, time, king, 1.) {
            self.ball.draw(t, 1., bright);
        }
        if let Some((pose, clip, t, looping)) = r.alice_pose() {
            self.alice.atmosphere(atmo, camera);
            watch(&mut self.alice, pose, king.translation + Vec3::Z * 112.);
            self.alice.draw(clip, t, looping, pose, 1., bright);
        }
        if let Some((pose, scale, clip, t)) = r.pawn_pose() {
            self.pawn.atmosphere(atmo, camera);
            self.pawn.draw(clip, t, true, pose, scale, bright);
        }
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

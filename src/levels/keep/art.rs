use super::*;
use crate::npc::Puppet;
pub(super) struct Art {
    alice: Puppet,
    cat: Puppet,
    lever: Vec<crate::weapons::Prop>,
    popup: crate::weapons::Prop,
    burst: Option<crate::particles::Attached>,
    guards: Vec<Puppet>,
    bolt: crate::weapons::Prop,
    card_bolts: Vec<crate::weapons::Prop>,
    rubble: Vec<crate::weapons::Prop>,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            alice: Puppet::load(a, "alice", data::ALICE, &specs)?,
            cat: Puppet::load(a, "c_cheshire", data::CAT, &specs)?,
            lever: ["start", "move", "move_backward"]
                .iter()
                .map(|n| crate::weapons::Prop::load_animation(a, "lever", n, &specs))
                .collect::<Result<_>>()?,
            popup: crate::weapons::Prop::load_animation(a, "c_queen1_popup", "popup", &specs)?,
            burst: crate::particles::Attached::load(a, "fx_particle_burst", &specs)?,
            guards: ["club", "diamond", "spade", "heart"]
                .iter()
                .map(|s| Puppet::load(a, &format!("cardguard_{s}"), &[], &specs))
                .collect::<Result<_>>()?,
            bolt: crate::weapons::Prop::load_animation(a, "prj_diamond", "idle", &specs)?,
            card_bolts: ["prj_heart", "prj_spade"]
                .iter()
                .map(|n| crate::weapons::Prop::load_animation(a, n, "idle", &specs))
                .collect::<Result<_>>()?,
            rubble: (1..=3)
                .map(|n| crate::weapons::Prop::load(a, &format!("obj_debris_rubble{n}"), &specs))
                .collect::<Result<_>>()?,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.alice.mouth(s.mouth(&["alice", "fakeplayer"]));
        self.cat.mouth(s.mouth(&[
            "lift_cat",
            "lever_cat",
            "cheshire_actor1",
            "cheshire_actor2",
            "cheshire_actor3",
        ]));
        self.cat.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let k = l.downcast_ref::<Keep>().unwrap();
        if !k.all_won() {
            let t = k
                .saved
                .scene
                .as_ref()
                .filter(|s| s.kind == Kind::Mirror)
                .map(|s| s.time);
            match t {
                Some(t) if t < 1. => {
                    self.lever[1].draw_frame(k.data.at("mirror_lever"), 1., bright, t, false)
                }
                Some(t) if t < 2. => {
                    self.lever[2].draw_frame(k.data.at("mirror_lever"), 1., bright, t - 1., false)
                }
                _ => self.lever[0].draw(k.data.at("mirror_lever"), 1., bright),
            };
        }
        if let Some(at) = k.alice_pose() {
            self.alice.atmosphere(atmo, camera);
            let (c, t, b) = k.alice_act();
            self.alice.draw(c, t, b, at, 1., bright);
        }
        if let Some((at, c, t, b, alpha)) = k.cat() {
            self.cat.atmosphere(atmo, camera);
            if k.saved.head.severed {
                self.cat.draw_split(c, t, b, at, "head*", false, bright);
            } else {
                self.cat
                    .draw_dissolving(c, t, b, at, 1., bright, 1. - alpha);
            }
            if let Some(head) = &k.saved.head.fragment {
                self.cat.draw_split(
                    "death_head",
                    k.saved.head.age,
                    false,
                    head.transform(),
                    "head*",
                    true,
                    bright,
                );
            }
        }
        if let Some(t) = k.strike_time().filter(|t| *t >= 0. && *t < 0.5) {
            self.popup
                .draw_frame(k.data.at("cat_popup"), 1., bright, t, false);
        }
        for a in &k.saved.spawned {
            let group = &k
                .data
                .spawns
                .iter()
                .find(|(id, _, _)| *id == a.entity)
                .unwrap()
                .1;
            let i = SUITS.iter().position(|s| s == group).unwrap_or(3);
            match &a.guard {
                guards::Body::Basic(g) => {
                    self.guards[i].draw_guard(g, bright, camera, atmo);
                    for shot in &g.shots {
                        self.bolt.draw_frame(
                            Transform {
                                translation: shot.position,
                                rotation: Quat::from_rotation_arc(Vec3::X, shot.direction),
                            },
                            1.,
                            bright,
                            shot.age,
                            true,
                        );
                    }
                }
                guards::Body::Card(g) => {
                    self.guards[i].draw_card(g, bright, camera, atmo);
                    for shot in &g.shots {
                        self.card_bolts[usize::from(g.kind == crate::cards::Kind::Spade)]
                            .draw_frame(
                                Transform {
                                    translation: shot.position,
                                    rotation: Quat::from_rotation_arc(Vec3::X, shot.direction),
                                },
                                1.,
                                bright,
                                shot.age,
                                true,
                            );
                    }
                }
            }
            self.guards[i].draw_electric(a.electric);
        }
        if let Some(i) = k.saved.smashed.filter(|_| k.saved.win_time < 1.5) {
            let t = k.saved.win_time;
            let at = match i {
                0 => vec3(512., -2693., 960.),
                1 => vec3(-2388., -273., 960.),
                _ => vec3(3411., -272., 960.),
            };
            for n in 0..4 {
                let a = n as f32 * 2.4;
                self.rubble[n % 3].draw(
                    Transform {
                        translation: at + vec3(a.cos() * 150., a.sin() * 150., 220.) * t
                            - Vec3::Z * 400. * t * t,
                        rotation: Quat::from_euler(EulerRot::XYZ, t + a, t * 2., t * 3.),
                    },
                    1.,
                    bright,
                );
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmo: &crate::environment::Atmosphere,
    ) {
        let k = l.downcast_ref::<Keep>().unwrap();
        if let Some(t) = k.strike_time().filter(|t| *t >= 0. && *t < 5.) {
            self.cat
                .draw_effects("death", t, false, k.data.at("cat_pace2"), 1., camera, atmo);
            if t < 0.2 {
                if let Some(burst) = &mut self.burst {
                    let at = k.data.at("cat_popup");
                    let origin = self.popup.point(at, "tag_burst", 1.);
                    burst.draw(
                        t,
                        1.,
                        |_, _| Transform {
                            translation: origin,
                            rotation: at.rotation,
                        },
                        |_, _, default| default,
                        camera,
                        atmo,
                    );
                }
            }
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

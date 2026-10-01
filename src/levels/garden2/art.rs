use super::*;
use crate::{npc::Puppet, weapons::Prop};
pub(super) struct Art {
    alice: Puppet,
    rabbit: Puppet,
    hatter: Puppet,
    cat: Puppet,
    dead: Prop,
    ants: [Puppet; 2],
    shots: [Prop; 2],
    lady: crate::ladybug::Art,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            alice: Puppet::load(a, "alice", ALICE, &specs)?,
            rabbit: Puppet::load(a, "c_whiterabbit", RABBIT, &specs)?,
            hatter: Puppet::load(a, "c_madhatter", HATTER, &specs)?,
            cat: Puppet::load(a, "c_cheshire", CAT, &specs)?,
            dead: Prop::load(a, "c_whiterabbit_stomped", &specs)?,
            ants: [
                Puppet::load(a, "c_armyant", crate::ant::REGULAR, &specs)?,
                Puppet::load(a, "c_armyantcorp", crate::ant::CORPORAL, &specs)?,
            ],
            shots: [
                Prop::load_animation(a, "prj_bullet", "idle", &specs)?,
                Prop::load_animation(a, "prj_grenade", "idle", &specs)?,
            ],
            lady: crate::ladybug::Art::load(a, &specs)?,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.alice.mouth(s.mouth(&["fakeplayer", "alice"]));
        self.rabbit.mouth(s.mouth(&["rabbit_actor"]));
        self.cat.mouth(s.mouth(&["cat_actor1", "last_cat"]));
        self.cat.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Garden>().unwrap();
        for a in o.saved.ants.iter().filter(|a| a.enabled) {
            let p = &mut self.ants[usize::from(a.corporal)];
            p.atmosphere(atmo, camera);
            p.draw(
                a.clip(),
                a.time,
                a.loops(),
                Transform {
                    translation: a.feet,
                    rotation: Quat::from_rotation_z(a.yaw),
                },
                a.visual_scale(&o.data),
                bright,
            );
            for b in &a.shots {
                self.shots[usize::from(b.grenade)].draw_frame(
                    Transform {
                        translation: b.at,
                        rotation: Quat::from_rotation_arc(Vec3::X, b.direction),
                    },
                    1.,
                    bright,
                    b.age,
                    true,
                );
            }
        }
        for l in o.saved.ladies.iter().filter(|l| l.enabled) {
            self.lady.draw(&l.actor, o.data.lady, bright);
        }
        let s = o.saved.scene.as_ref();
        let dead = o.saved.arrived
            || s.is_some_and(|s| {
                s.kind == Kind::Arrival
                    && (matches!(s.phase, Phase::Kneel | Phase::Outro)
                        || (s.phase == Phase::Squish && s.elapsed() >= 15.))
            });
        if dead {
            self.dead
                .draw(o.data.points["rabbit_actor_dead"], 1., bright);
        }
        for (p, m, pose) in [
            (&mut self.alice, "alice", o.alice_pose()),
            (&mut self.hatter, "c_madhatter", o.hatter_pose()),
            (&mut self.rabbit, "c_whiterabbit", o.rabbit_pose(false)),
        ] {
            if (m == "alice" && s.is_none()) || (m == "c_whiterabbit" && dead) {
                continue;
            }
            let (clip, t, loops) = o.acting(m);
            let target = if m == "alice" {
                if dead {
                    o.data.points["rabbit_actor_dead"].translation
                } else if s.is_some_and(|s| s.phase == Phase::Squish) {
                    o.data.points["alice_looker"].translation
                } else {
                    o.rabbit_pose(false).translation + Vec3::Z * 48.
                }
            } else if m == "c_whiterabbit" && s.is_some_and(|s| s.phase == Phase::Squish) {
                o.data.points["rabbit_looker"].translation
            } else {
                o.alice_pose().translation + Vec3::Z * 48.
            };
            let mut watch = crate::facial::Watch::default();
            if m != "c_madhatter" {
                watch.update(
                    10.,
                    Some(pose.rotation.conjugate() * (target - pose.translation)),
                );
            }
            p.watch(watch);
            p.atmosphere(atmo, camera);
            p.draw(clip, t, loops, pose, 1., bright);
        }
        if s.is_some_and(|s| {
            s.kind == Kind::Arrival
                && matches!(
                    s.phase,
                    Phase::Approach | Phase::Talk | Phase::Run | Phase::Squish
                )
                && !(s.phase == Phase::Squish && s.elapsed() >= 13.1)
        }) {
            let moving = s.is_some_and(|s| s.phase == Phase::Squish && s.elapsed() >= 11.5);
            self.rabbit.draw(
                if moving { "run" } else { "i_calm_l" },
                s.map_or(0., |s| (s.elapsed() - 11.5).max(0.)),
                true,
                o.rabbit_pose(true),
                0.1,
                bright,
            );
        }
        let alpha = o.cat_alpha();
        if alpha > 0. {
            let final_cat = s.is_some_and(|s| s.kind == Kind::Cat);
            let pose = o.data.points[if final_cat {
                "cat_end_pos"
            } else {
                "cat_squish_pos1"
            }];
            let (clip, t, loops) = o.acting("c_cheshire");
            self.cat.atmosphere(atmo, camera);
            let mut watch = crate::facial::Watch::default();
            watch.update(
                10.,
                Some(pose.rotation.conjugate() * (o.alice_pose().translation - pose.translation)),
            );
            self.cat.watch(watch);
            if alpha >= 0.999 {
                self.cat.draw(clip, t, loops, pose, 1., bright);
            } else {
                self.cat.draw_afterimage(clip, t, pose, alpha);
            }
        }
    }
    fn effects(&mut self, l: &dyn LevelController, _: Vec3, _: &crate::environment::Atmosphere) {
        let o = l.downcast_ref::<Garden>().unwrap();
        for l in o.saved.ladies.iter().filter(|l| l.enabled) {
            crate::ladybug::Art::effects(&l.actor);
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

use super::*;
use crate::npc::Puppet;
pub(super) struct Art {
    alice: Puppet,
    guards: Puppet,
    duel: Vec<(Puppet, String, f32)>,
    pawn: Puppet,
    bolt: crate::weapons::Prop,
    trail: Option<crate::particles::Attached>,
    trails: std::collections::BTreeMap<(usize, u32), crate::particles::Attached>,
}
fn action(model: &str) -> &'static str {
    match model {
        "c_chess_knight" => "attack1",
        "c_chess_rook" => "attack1",
        "c_chess_pawn" => "attack1",
        "cardguard_diamond" => "stand_attack",
        _ => "attack1",
    }
}
impl Art {
    pub fn load(a: &mut Assets, o: &Royale) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut duel = Vec::new();
        for (model, _, _) in &o.data.duel {
            let def = crate::skeletal::Definition::load(a, &format!("models/{model}.tik"))?;
            let preferred = action(model);
            let clip = if def.animations.contains_key(preferred) {
                preferred.to_owned()
            } else {
                def.animations
                    .keys()
                    .find(|n| n.contains("attack"))
                    .context("Duel attack missing")?
                    .clone()
            };
            let skeleton =
                crate::skeletal::Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
            let anim = crate::skeletal::Animation::parse_tags(
                &a.read(&format!("{}/{}", def.path, def.animations[&clip]))?,
                &skeleton.unskinned_tags(),
            )?
            .0;
            duel.push((
                Puppet::load(a, model, &[&clip], &specs)?,
                clip,
                anim.duration(),
            ));
        }
        Ok(Self {
            alice: Puppet::load(a, "alice", &["run", "idle_stand"], &specs)?,
            guards: Puppet::load(a, "cardguard_spade", &[], &specs)?,
            duel,
            pawn: Puppet::load(a, "c_chess_pawn", &["idle"], &specs)?,
            bolt: crate::weapons::Prop::load_animation(a, "prj_spade", "idle", &specs)?,
            trail: crate::particles::Attached::load(a, "prj_spade", &specs)?,
            trails: Default::default(),
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
        let o = l.downcast_ref::<Royale>().unwrap();
        if !o.saved.done {
            let t = o.saved.time;
            let moving = t * 320. < o.data.length;
            self.alice.atmosphere(atmo, camera);
            self.alice.draw(
                if moving { "run" } else { "idle_stand" },
                if moving { t } else { t - o.data.length / 320. },
                true,
                o.alice_pose(),
                1.,
                bright,
            );
            for (i, ((puppet, clip, duration), (_, at, _))) in
                self.duel.iter_mut().zip(&o.data.duel).enumerate()
            {
                puppet.atmosphere(atmo, camera);
                puppet.draw(
                    if t < 14. { "idle" } else { clip },
                    if t < 14. {
                        t
                    } else {
                        (t - 14. + i as f32 * 0.19) % *duration
                    },
                    t < 14.,
                    *at,
                    1.,
                    bright,
                );
            }
            if (10. ..18.).contains(&t) {
                let age = t - 10.;
                let mut at = o.data.pawn;
                at.translation += o.data.pawn_velocity * age
                    - Vec3::Z * 0.5 * crate::movement::GRAVITY * age * age;
                self.pawn.atmosphere(atmo, camera);
                self.pawn.draw("idle", age, true, at, 1., bright);
            }
        }
        self.trails.retain(|(id, serial), _| {
            o.saved.actors[*id]
                .body
                .shots
                .iter()
                .any(|s| s.serial == *serial)
        });
        for (id, a) in o.saved.actors.iter().enumerate() {
            if !a.spawned {
                continue;
            }
            self.guards.draw_card(&a.body, bright, camera, atmo);
            self.guards.draw_electric(a.electric);
            for s in &a.body.shots {
                self.bolt.draw_frame(
                    Transform {
                        translation: s.position,
                        rotation: Quat::from_rotation_arc(Vec3::X, s.direction),
                    },
                    1.,
                    bright,
                    s.age,
                    true,
                );
                if let Some(template) = &self.trail {
                    self.trails
                        .entry((id, s.serial))
                        .or_insert_with(|| template.fork())
                        .draw(
                            s.age,
                            1.,
                            |time, tag| {
                                let mut pose = Transform {
                                    translation: s.position - s.direction * 850. * (s.age - time),
                                    rotation: Quat::from_rotation_arc(Vec3::X, s.direction),
                                };
                                if let Some(tag) = tag {
                                    pose.translation = self.bolt.point(pose, tag, 1.);
                                }
                                pose
                            },
                            |_, _, default| default,
                            camera,
                            atmo,
                        );
                }
            }
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

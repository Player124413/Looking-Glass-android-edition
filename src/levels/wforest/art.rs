use super::*;
use crate::npc::Puppet;
pub(super) struct Art {
    alice: Puppet,
    cat: Puppet,
    humpty: Puppet,
    staff: crate::weapons::Prop,
    blast: crate::particles::Attached,
    essence: crate::loot_art::Art,
    rubble: Vec<crate::weapons::Prop>,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            alice: Puppet::load(a, "alice", data::ALICE, &specs)?,
            cat: Puppet::load(a, "c_cheshire", data::CAT, &specs)?,
            humpty: Puppet::load(a, "c_humptydumpty", data::HUMPTY, &specs)?,
            staff: crate::weapons::Prop::load(a, "w_eyestaff_staff", &specs)?,
            blast: crate::particles::Attached::load(a, "fx_blundersplode_wall", &specs)?
                .context("Missing wall explosion")?,
            essence: crate::loot_art::Art::load(a, &specs)?,
            rubble: (1..=3)
                .map(|n| crate::weapons::Prop::load(a, &format!("obj_debris_rubble{n}"), &specs))
                .collect::<Result<_>>()?,
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
        self.alice.mouth(s.mouth(&["alice"]));
        self.cat
            .mouth(s.mouth(&["cat_actor2", "cat_actor3", "blunder_cat"]));
        self.cat.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let f = l.downcast_ref::<Forest>().unwrap();
        if f.altar() {
            self.staff.draw_pickup(
                f.data.at("first_eyestaff").translation
                    + Vec3::Z * (44. + (f.saved.clock * 2.).sin() * 2.),
                f.saved.clock * 0.6,
            );
        }
        if let Some(at) = f.alice_pose() {
            self.alice.atmosphere(atmo, camera);
            let target = match f.saved.scene.as_ref().unwrap().kind {
                Kind::Caterpillar => f.data.at("caterpillar_actor1").translation,
                Kind::Wall => f.data.at("alicelookwall").translation,
                _ => f
                    .cat()
                    .map_or(at.translation + at.rotation * Vec3::X * 100., |c| {
                        c.0.translation + Vec3::Z * 45.
                    }),
            };
            watch(&mut self.alice, at, target);
            let (clip, t, looping) = f.alice_act();
            self.alice.draw(clip, t, looping, at, 1., bright);
        }
        if let Some((at, clip, t, looping, alpha)) = f.cat() {
            self.cat.atmosphere(atmo, camera);
            if let Some(a) = f.alice_pose() {
                watch(&mut self.cat, at, a.translation + Vec3::Z * 56.);
            }
            self.cat
                .draw_dissolving(clip, t, looping, at, 1., bright, 1. - alpha);
        }
        if f.saved.returning {
            self.humpty.atmosphere(atmo, camera);
            let (clip, t, looping) = f.humpty_act();
            self.humpty
                .draw(clip, t, looping, f.data.at("humpty_actor1"), 1., bright);
        }
        // The smashable wall also emits fifteen model fragments, separate from its seven
        // large authored brushes. Deterministic trajectories survive a saved explosion.
        if let Some(t) = f.saved.destruction.filter(|t| *t < 1.5) {
            for n in 0..15 {
                let a = n as f32 * 2.3999631;
                let spread = vec3(
                    a.cos() * 260.,
                    -80. - (n % 4) as f32 * 24.,
                    (n / 5) as f32 * 120. - 120.,
                );
                let velocity = vec3(
                    a.sin() * 160.,
                    -180. - (n % 3) as f32 * 50.,
                    220. + (n % 5) as f32 * 25.,
                );
                let at = Transform {
                    translation: f.data.wall_center + spread + velocity * t
                        - Vec3::Z * 400. * t * t,
                    rotation: Quat::from_euler(EulerRot::XYZ, t * 3. + a, t * 2., t * 4.),
                };
                self.rubble[n % 3].draw(at, 1., bright);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmo: &crate::environment::Atmosphere,
    ) {
        let f = l.downcast_ref::<Forest>().unwrap();
        if f.saved.returning {
            let (clip, t, looping) = f.humpty_act();
            self.humpty.draw_effects(
                clip,
                t,
                looping,
                f.data.at("humpty_actor1"),
                1.,
                camera,
                atmo,
            );
        }
        if let Some(t) = f.saved.destruction {
            if t < 5.5 {
                for (delay, name) in [(0., "splodeme"), (0.5, "splodeme2")] {
                    if t >= delay {
                        self.blast.draw(
                            t - delay,
                            1.,
                            |_, _| f.data.at(name),
                            |_, _, _| true,
                            camera,
                            atmo,
                        );
                    }
                }
            }
        }
        if f.saved.returning && f.saved.essence_delay == 0. {
            let at = f.data.at(if f.saved.essence == 0 {
                "get_me1"
            } else {
                "get_me2"
            });
            self.essence
                .draw(crate::loot::Grade::Medium, at.translation, atmo, camera, f.saved.clock);
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

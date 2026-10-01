use super::*;
use crate::npc::Puppet;
pub(super) struct Art {
    alice: Puppet,
    cater: Puppet,
    smoke: crate::particles::Attached,
    rocks: Vec<crate::weapons::Prop>,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            rocks: [
                "rock_ice_06",
                "rock_ice_05",
                "rock_ice_05",
                "rock_ice_05",
                "rock_ice_06",
                "rock_ice_06",
                "marble",
            ]
            .iter()
            .map(|n| crate::weapons::Prop::load(a, n, &specs))
            .collect::<Result<_>>()?,
            alice: Puppet::load(a, "alice", ALICE, &specs)?,
            cater: Puppet::load(a, "c_caterpillar", CATER, &specs)?,
            smoke: crate::particles::Attached::load(a, "c_caterpillar", &specs)?
                .context("Missing Caterpillar emitters")?,
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
        self.cater.mouth(s.mouth(&["caterpillar_actor1"]));
        self.cater.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let g = l.downcast_ref::<Garden>().unwrap();
        for (k, r) in g
            .course()
            .rocks
            .iter()
            .enumerate()
            .filter(|(_, r)| r.visible)
        {
            self.rocks[k].draw(
                Transform {
                    translation: r.position,
                    rotation: r.rotation,
                },
                g.data.rocks[k].scale,
                bright,
            );
        }
        if g.marble_time().is_some() {
            self.alice.atmosphere(atmo, camera);
            self.alice.draw(
                "idle_stand",
                g.marble_time().unwrap(),
                true,
                g.data.marble_start,
                1.,
                bright,
            );
            return;
        }
        let t = g
            .saved
            .scene
            .as_ref()
            .map_or(g.saved.idle, |s| s.clock.time);
        self.cater.atmosphere(atmo, camera);
        watch(
            &mut self.cater,
            g.data.cater,
            g.data.alice.translation + Vec3::Z * 64.,
        );
        let (clip, time, looping) = g.acting_at("c_caterpillar", t);
        self.cater
            .draw(clip, time, looping, g.data.cater, 1., bright);
        if g.scripted() {
            self.alice.atmosphere(atmo, camera);
            watch(
                &mut self.alice,
                g.data.alice,
                g.data.cater.translation + Vec3::Z * 48.,
            );
            let (clip, time, looping) = g.acting_at("alice", t);
            self.alice
                .draw(clip, time, looping, g.data.alice, 1., bright);
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmo: &crate::environment::Atmosphere,
    ) {
        let g = l.downcast_ref::<Garden>().unwrap();
        let Some(age) = g
            .saved
            .scene
            .as_ref()
            .map(|s| s.clock.time)
            .or(g.saved.outro)
        else {
            return;
        };
        let acting = |t| {
            if g.scripted() {
                g.acting_at("c_caterpillar", t)
            } else {
                g.last_acting(t)
            }
        };
        self.smoke.draw(
            age,
            2.,
            |t, tag| {
                let (clip, time, looping) = acting(t);
                if looping {
                    self.cater
                        .looping_tag(tag.unwrap_or("tag_smoke"), clip, time, g.data.cater, 1.)
                } else {
                    self.cater
                        .tag(tag.unwrap_or("tag_smoke"), clip, time, g.data.cater, 1.)
                }
                .unwrap_or(g.data.cater)
            },
            |name, t, _| {
                let (clip, time, _) = acting(t);
                name == "portalsmoke"
                    && clip == "portal_smoke"
                    && (83. ..105.).contains(&(time / g.data.frame("c_caterpillar", clip)))
            },
            camera,
            atmo,
        );
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

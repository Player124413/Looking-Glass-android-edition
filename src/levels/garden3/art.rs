use super::*;
use crate::{npc::Puppet, weapons::Prop};
pub(super) struct Art {
    alice: Puppet,
    bug: Puppet,
    marble: Prop,
    shroom: Prop,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let s = crate::texture::read_materials(a)?;
        Ok(Self {
            alice: Puppet::load(a, "alice", ALICE, &s)?,
            bug: Puppet::load(a, "c_ladybug", BUG, &s)?,
            marble: Prop::load(a, "marble", &s)?,
            shroom: Prop::load(a, "g_shroom07", &s)?,
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
        let g = l.downcast_ref::<Garden>().unwrap();
        self.bug.show_attachments(false);
        self.bug.atmosphere(atmo, camera);
        self.alice.atmosphere(atmo, camera);
        for k in 1..=10 {
            if let Some((pose, clip, time, scale)) = g.bug_pose(k) {
                self.bug.draw(clip, time, true, pose, scale, bright);
                if k == 10 && g.saved.age < 18.5 {
                    if let Some(mut tag) =
                        self.bug.looping_tag("tag_weapon", clip, time, pose, scale)
                    {
                        tag.translation += tag.rotation * vec3(0., 0., -116.);
                        self.marble.draw(tag, 0.33 * scale, bright);
                    }
                }
            }
        }
        if g.scripted() {
            let t = g.saved.age;
            let (clip, time) = if t >= 23.5 {
                ("run", t - 23.5)
            } else if t >= RELEASE {
                ("ready", t - RELEASE)
            } else {
                ("idle", t)
            };
            self.alice
                .draw(clip, time, true, g.alice_pose(), 1., bright);
        }
        let r = &g.saved.rock;
        if r.visible {
            self.marble.draw(
                Transform {
                    translation: r.position,
                    rotation: r.rotation,
                },
                g.data.rock.scale,
                bright,
            );
        }
        if let Some((pose, _)) = &g.shroom {
            self.shroom.draw(*pose, g.data.shroom_scale, bright);
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

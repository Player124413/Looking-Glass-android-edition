use super::*;
use crate::{npc::Puppet, weapons::Prop};
pub(super) struct Art {
    boss: Puppet,
    alice: Puppet,
    ant: Puppet,
    shots: [Prop; 2],
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut boss = Puppet::load(a, "c_centipede", data::BOSS, &specs)?;
        boss.mouth_angle(45.);
        boss.surface_visible("material4", false);
        Ok(Self {
            boss,
            alice: Puppet::load(a, "alice", data::ALICE, &specs)?,
            ant: Puppet::load(a, "c_armyant", crate::ant::REGULAR, &specs)?,
            shots: [
                Prop::load_animation(a, "prj_bullet", "idle", &specs)?,
                Prop::load_animation(a, "prj_grenade", "idle", &specs)?,
            ],
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.boss.mouth(s.mouth(&["centipede_actor1"]));
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atm: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Flora>().unwrap();
        for a in [&mut self.boss, &mut self.alice, &mut self.ant] {
            a.atmosphere(atm, camera);
        }
        if o.reveal() {
            let (clip, t, looping) = o.boss_clip();
            self.boss
                .draw(clip, t, looping, o.data.points["cent_posx1"], 1., bright);
            self.alice.draw(
                "idle_stand",
                o.saved.time as f32,
                true,
                o.saved.alice,
                1.,
                bright,
            );
        }
        for s in o.saved.ants.iter().filter(|s| s.ant.enabled) {
            let a = &s.ant;
            let moving =
                o.saved.phase == Phase::Rush && o.data.runners.contains_key(&a.id) && a.health > 0.;
            self.ant.draw(
                if moving { "walk_fast" } else { a.clip() },
                a.time,
                moving || a.loops(),
                Transform {
                    translation: a.feet,
                    rotation: Quat::from_rotation_z(a.yaw),
                },
                a.visual_scale(&o.data),
                bright,
            );
            self.ant.draw_electric(a.electric);
            for shot in &a.shots {
                crate::ant::draw_shot(&mut self.shots, shot, bright);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atm: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Flora>().unwrap();
        if o.reveal() {
            let (clip, t, looping) = o.boss_clip();
            self.boss.draw_effects(
                clip,
                t,
                looping,
                o.data.points["cent_posx1"],
                1.,
                camera,
                atm,
            );
        }
        for s in o.saved.ants.iter().filter(|s| s.ant.enabled) {
            let a = &s.ant;
            self.ant.draw_effects(
                a.clip(),
                a.time,
                a.loops(),
                Transform {
                    translation: a.feet,
                    rotation: Quat::from_rotation_z(a.yaw),
                },
                a.visual_scale(&o.data),
                camera,
                atm,
            );
            crate::ant::draw_blasts(a);
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

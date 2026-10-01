use super::*;
pub(super) struct Art {
    levers: Vec<crate::weapons::Prop>,
    alice: crate::npc::Puppet,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            levers: ["start", "move", "stop"]
                .iter()
                .map(|n| crate::weapons::Prop::load_animation(a, "lever", n, &specs))
                .collect::<Result<_>>()?,
            alice: crate::npc::Puppet::load(a, "alice", &["use_lever"], &specs)?,
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
        let o = l.downcast_ref::<Maze>().unwrap();
        for k in 0..2 {
            if let Some(p) = o.saved.pull.as_ref().filter(|p| p.index == k) {
                self.levers[1].draw_frame(
                    o.levers[k],
                    1.,
                    bright,
                    (p.time - ALIGN_TIME).max(0.),
                    false,
                );
            } else {
                self.levers[if o.saved.used[k] { 2 } else { 0 }].draw(o.levers[k], 1., bright);
            }
        }
        if let Some(p) = &o.saved.pull {
            self.alice.atmosphere(atmo, camera);
            self.alice.draw(
                "use_lever",
                (p.time - ALIGN_TIME).max(0.),
                false,
                o.pull_pose(p),
                1.,
                bright,
            );
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

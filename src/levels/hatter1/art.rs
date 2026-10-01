use super::*;
use crate::npc::Puppet;
pub(super) struct Art {
    alice: Puppet,
    hare: Puppet,
    mouse: Puppet,
    gryphon: Puppet,
    hatter: Puppet,
    cat: Puppet,
    levers: Vec<crate::weapons::Prop>,
    machine: crate::weapons::Prop,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            alice: Puppet::load(a, "alice", &["idle_stand", "walk", "use_lever"], &specs)?,
            hare: Puppet::load(
                a,
                "c_marchhare",
                &[
                    "idle_base",
                    "idle_base_struggle",
                    "talk01",
                    "talk02",
                    "talk03",
                ],
                &specs,
            )?,
            mouse: Puppet::load(
                a,
                "c_dormouse",
                &["idle_base", "giveustea", "idle_base_twich"],
                &specs,
            )?,
            gryphon: Puppet::load(a, "c_gryphon", &[], &specs)?,
            hatter: Puppet::load(
                a,
                "c_madhatter",
                &["stand_base", "stand_alert02", "attack_cane"],
                &specs,
            )?,
            cat: Puppet::load(
                a,
                "c_cheshire",
                &["sit_idle1", "sit_talk1", "appear"],
                &specs,
            )?,
            levers: ["start", "move"]
                .iter()
                .map(|n| crate::weapons::Prop::load_animation(a, "lever", n, &specs))
                .collect::<Result<_>>()?,
            machine: crate::weapons::Prop::load_animation(a, "obj_door_machine01", "run", &specs)?,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.alice.mouth(s.mouth(&["alice", "fakeplayer"]));
        self.hare.mouth(s.mouth(&["hare_actor1"]));
        self.mouse.mouth(s.mouth(&["mouse_actor1"]));
        self.gryphon.mouth(s.mouth(&["gryphon_actor1"]));
        self.cat.mouth(s.mouth(&["hatter_cat"]));
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Clockwork>().unwrap();
        let s = &o.saved;
        if let Some(pull) = &s.pull {
            self.alice.atmosphere(atmo, camera);
            self.alice.draw(if pull.started {"use_lever"} else {"walk"}, pull.time,
                !pull.started, pull.pose, 1., bright);
        }
        for (k, at) in o.levers.iter().enumerate() {
            if let Some(t) = s.levers[k] {
                self.levers[1].draw_frame(*at, 1., bright, s.age - t, false);
            } else {
                self.levers[0].draw(*at, 1., bright);
            }
        }
        self.machine.draw_frame(
            o.point("da_machine"),
            1.,
            bright,
            s.levers[3].map_or(0., |t| (s.age - t - 2.).max(0.)),
            true,
        );
        if let Some(at) = o.alice_pose() {
            let sc = s.scene.as_ref().unwrap();
            let walking = sc.kind == Kind::Hare
                && ((1.5..4.).contains(&sc.time) || (16. ..18.).contains(&sc.time));
            self.alice.atmosphere(atmo, camera);
            self.alice.draw(
                if walking { "walk" } else { "idle_stand" },
                sc.time,
                true,
                at,
                1.,
                bright,
            );
        }
        let sc = s.scene.as_ref().filter(|s| s.kind == Kind::Hare);
        let line = sc.map(|s| s.line);
        let t = sc.map_or(s.age, |s| s.line_time);
        let mut hare = o.point("hare_actor1");
        if let Some(t) = s.hare {
            hare.translation.z -= motion::dunk(s.age - t);
        }
        self.hare.atmosphere(atmo, camera);
        self.hare.draw(
            if matches!(line, Some(0 | 5 | 7 | 10)) {
                "talk02"
            } else if line == Some(2) {
                "idle_base_struggle"
            } else {
                "idle_base"
            },
            t,
            true,
            hare,
            1.5,
            bright,
        );
        self.mouse.atmosphere(atmo, camera);
        self.mouse.draw(
            if matches!(line, Some(1 | 9)) {
                "giveustea"
            } else if line == Some(3) || s.hare.is_some_and(|a| (s.age - a).rem_euclid(8.) < 1.) {
                "idle_base_twich"
            } else {
                "idle_base"
            },
            t,
            true,
            o.point("mouse_actor1"),
            1.,
            bright,
        );
        self.gryphon.atmosphere(atmo, camera);
        let idle = self.gryphon.idle_clip().to_owned();
        self.gryphon
            .draw(&idle, s.age, true, o.point("gryphon_actor1"), 1., bright);
        if let Some(sc) = s
            .scene
            .as_ref()
            .filter(|s| s.kind == Kind::Port && (6. ..14.75).contains(&s.time))
        {
            let (clip, epoch) = if sc.time < 9. {
                ("stand_base", 7.)
            } else if sc.time < 10.55 {
                ("stand_alert02", 9.)
            } else if sc.time < 11.65 {
                ("attack_cane", 10.55)
            } else {
                ("stand_base", 11.65)
            };
            self.hatter.atmosphere(atmo, camera);
            self.hatter.draw(
                clip,
                (sc.time - epoch).max(0.),
                false,
                o.point("loco_hatter"),
                1.,
                bright,
            );
        }
        if let Some(sc) = s.scene.as_ref().filter(|s| s.kind == Kind::Hint) {
            let alpha = (sc.time / 2.).min(1.)
                * sc.ending
                    .map_or(1., |t| (1. - (t - 1.).max(0.) / 2.).max(0.));
            self.cat.atmosphere(atmo, camera);
            self.cat.draw_dissolving(
                "sit_talk1",
                sc.time,
                true,
                o.point("hatter_cat"),
                1.,
                bright,
                1. - alpha,
            );
        }
    }
    fn effects(&mut self, l: &dyn LevelController, _: Vec3, _: &crate::environment::Atmosphere) {
        let o = l.downcast_ref::<Clockwork>().unwrap();
        if let Some(t) = o.saved.hare {
            let phase = (o.saved.age - t).rem_euclid(8.);
            if phase < 1. {
                self.mouse.draw_electric(1. - phase);
            }
            if (3. ..4.).contains(&phase) {
                self.hare.draw_electric(4. - phase);
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

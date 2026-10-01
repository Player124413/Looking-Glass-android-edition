use super::*;
use crate::{npc::Puppet, weapons::Prop};
pub(super) struct Art {
    turtle: Puppet,
    alice: Puppet,
    shell: Prop,
    props: BTreeMap<String, Prop>,
    bubbles: crate::particles::Attached,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Temple) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut props = BTreeMap::new();
        for (name, (model, _)) in &o.data.models {
            if name == "turtle" || name == "fake_alice" {
                continue;
            }
            let clips: &[&str] = if name.starts_with("oyster") {
                &["ready", "closing", "idle", "openning"]
            } else if name == "fishhead" {
                &["idle", "attack"]
            } else if o.data.fish.contains_key(name) {
                &["swim"]
            } else {
                &["idle"]
            };
            for clip in clips {
                let key = format!("{model}/{clip}");
                if let std::collections::btree_map::Entry::Vacant(entry) = props.entry(key) {
                    entry.insert(Prop::load_animation(a, model, clip, &specs)?);
                }
            }
        }
        Ok(Self {
            turtle: Puppet::load(a, "c_mockturtle", &["swim"], &specs)?,
            alice: Puppet::load(a, "alice", &["swim_forward_frog"], &specs)?,
            shell: Prop::load(a, "mock_shell", &specs)?,
            props,
            bubbles: crate::particles::Attached::load(a, "fx_mockturtle_launcher", &specs)?
                .context("Turtle particles missing")?,
        })
    }
}
impl LevelArt for Art {
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        let o = l.downcast_ref::<Temple>().unwrap();
        let age = o.saved.clock as f32;
        let pose = o.turtle_pose(o.saved.clock);
        self.turtle.atmosphere(atmosphere, camera);
        self.turtle.draw("swim", age, true, pose, 1., fullbright);
        self.shell
            .draw_frame(o.data.shell_pose(pose, age), 1., fullbright, age, true);
        if let Some(s) = &o.saved.scene {
            self.alice.atmosphere(atmosphere, camera);
            self.alice.draw(
                "swim_forward_frog",
                s.cast_time,
                true,
                o.data.alice.curve.sample(s.cast_time, true),
                1.,
                fullbright,
            );
        }
        for (name, (model, base)) in &o.data.models {
            if name == "turtle" || name == "fake_alice" {
                continue;
            }
            let mut pose = *base;
            let (mut clip, mut clock) = ("idle", age);
            if let Some(n) = name
                .strip_prefix("oyster")
                .and_then(|s| s.parse::<usize>().ok())
            {
                let t = o.saved.clams[n - 1].unwrap_or(-1.);
                let phase = t.max(0.) as f32 % 4.91;
                (clip, clock) = if t < 0. || phase < 1. {
                    ("ready", phase)
                } else if phase < 1.21 {
                    ("closing", phase - 1.)
                } else if phase < 3.21 {
                    ("idle", phase - 1.21)
                } else if phase < 3.91 {
                    ("openning", phase - 3.21)
                } else {
                    ("ready", phase - 3.91)
                };
            } else if name == "fishhead" {
                if let Some(t) = o.saved.fishhead {
                    clip = "attack";
                    clock = t;
                }
            } else if let Some(path) = o.data.fish.get(name) {
                let switch = if name == "fish_school6" {
                    o.saved
                        .events
                        .get("FishCalm")
                        .map(|t| ("FishCalm", *t))
                        .or_else(|| {
                            o.saved
                                .events
                                .get("Column3Fall")
                                .filter(|t| o.saved.clock - **t >= 2.)
                                .map(|t| ("FishDart", t + 2.))
                        })
                } else if name == "fish_school4d" {
                    o.saved.events.get("Fish2Dart").map(|t| ("Fish2Dart", *t))
                } else {
                    None
                };
                let (path, time) = switch.map_or((path, age), |(n, t)| {
                    (&o.data.fish[n], (o.saved.clock - t) as f32)
                });
                pose = path.curve.sample(time, true);
                clip = "swim";
            }
            if let Some(p) = self.props.get_mut(&format!("{model}/{clip}")) {
                p.draw_frame(pose, 1., fullbright, clock, true);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Temple>().unwrap();
        self.bubbles.draw(
            o.saved.clock as f32,
            1.,
            |t, _| o.data.shell_pose(o.turtle_pose(t as f64), t),
            |_, _, _| true,
            camera,
            atmosphere,
        );
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

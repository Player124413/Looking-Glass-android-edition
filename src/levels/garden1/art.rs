use super::*;
use crate::{npc::Puppet, weapons::Prop};
pub(super) struct Art {
    alice: Puppet,
    lily: Prop,
    rabbit2: Puppet,
    turtle: Puppet,
    rabbit: Puppet,
    shell: Prop,
    turtle_shell: Prop,
    glow: crate::particles::Attached,
    bubbles: crate::particles::Attached,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        Ok(Self {
            lily: Prop::load(a, "lilypad", &specs)?,
            rabbit2: Puppet::load(a, "c_whiterabbit", RABBIT_CLIPS, &specs)?,
            alice: Puppet::load(a, "alice", ALICE_CLIPS, &specs)?,
            turtle: Puppet::load(a, "c_mockturtle", TURTLE_CLIPS, &specs)?,
            rabbit: Puppet::load(a, "c_whiterabbit", RABBIT_CLIPS, &specs)?,
            shell: Prop::load_animation(a, "mock_shell_alice", "shell_alice", &specs)?,
            turtle_shell: Prop::load(a, "mock_shell", &specs)?,
            glow: crate::particles::Attached::load_clip_bursts(
                a,
                "fx_pickup",
                Some("on"),
                0.05,
                &specs,
            )?
            .context("Missing shell particles")?,
            bubbles: crate::particles::Attached::load(a, "fx_mockturtle_launcher", &specs)?
                .context("Missing Turtle bubbles")?,
        })
    }
}
fn watch(p: &mut Puppet, at: Transform, target: Option<Vec3>) {
    // Seekable attention: no render-frame clock, so a restored frame is identical.
    let mut w = crate::facial::Watch::default();
    w.update(
        10.,
        target.map(|t| at.rotation.conjugate() * (t - at.translation)),
    );
    p.watch(w);
}
fn tag(
    p: &Puppet,
    name: &str,
    clip: &str,
    time: f32,
    looping: bool,
    at: Transform,
) -> Option<Transform> {
    if looping {
        p.looping_tag(name, clip, time, at, 1.)
    } else {
        p.tag(name, clip, time, at, 1.)
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, story: &Story) {
        self.alice.mouth(story.mouth(&["alice", "fakeplayer"]));
        self.turtle.mouth(story.mouth(&["turtle_actor"]));
        self.rabbit.mouth(story.mouth(&["rabbit_actor"]));
        self.turtle.mouth_angle(45.);
        self.rabbit.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        let o = l.downcast_ref::<Garden>().unwrap();
        for p in &o.movers.pads {
            self.lily.draw(p.pose, p.scale, fullbright);
        }
        let t = o.saved.world.rabbit.map(|start| o.saved.world.time - start);
        let duration = o.data.deadtree_duration();
        if t.is_none_or(|t| t < duration) {
            let pose = if let Some(t) = t {
                o.data.deadtree_pose(t)
            } else {
                o.data
                    .footing(o.data.points["rabbit_follow1"], vec3(12., 12., 32.))
            };
            self.rabbit2.atmosphere(atmosphere, camera);
            self.rabbit2.draw(
                if t.is_some() { "run" } else { "i_calm_l" },
                t.unwrap_or(o.saved.world.time),
                true,
                pose,
                1.,
                fullbright,
            );
        }
        let s = o.saved.scene.as_ref();
        let alice = o.alice_pose();
        let turtle = o.turtle_pose();
        let rabbit = o.rabbit_pose();
        if !o.saved.arrived {
            let (clip, time, looping) = o.acting("c_mockturtle");
            self.turtle.atmosphere(atmosphere, camera);
            watch(
                &mut self.turtle,
                turtle,
                s.filter(|s| s.phase != Phase::Leave)
                    .map(|_| alice.translation + Vec3::Z * 64. - Vec3::Z * 96.),
            );
            self.turtle
                .draw(clip, time, looping, turtle, 1., fullbright);
            if let Some(p) = tag(&self.turtle, "tag_shell", clip, time, looping, turtle) {
                self.turtle_shell.draw(p, 1., fullbright);
            }
        }
        if !o.saved.rabbit_done {
            let (clip, time, looping) = o.acting("c_whiterabbit");
            self.rabbit.atmosphere(atmosphere, camera);
            watch(
                &mut self.rabbit,
                rabbit,
                s.filter(|s| s.kind == Kind::Rabbit && s.phase == Phase::Talk)
                    .map(|_| alice.translation),
            );
            self.rabbit
                .draw(clip, time, looping, rabbit, 1., fullbright);
        }
        let Some(s) = s else {
            return;
        };
        if s.kind == Kind::Arrival && s.phase == Phase::Launch && s.clock.time < 2. {
            return;
        }
        let (clip, time, looping) = o.acting("alice");
        self.alice.atmosphere(atmosphere, camera);
        let target = if s.kind == Kind::Arrival {
            turtle.translation + Vec3::Z * 32.
        } else {
            rabbit.translation
        };
        watch(
            &mut self.alice,
            alice,
            (s.phase != Phase::Launch).then_some(target),
        );
        self.alice.draw(clip, time, looping, alice, 1., fullbright);
        if let Some(t) = s.shell_at {
            if let Some(p) = tag(&self.alice, "tag_back", clip, time, looping, alice) {
                let age = s.clock.time - t;
                for mesh in self.shell.meshes_at(p, 1., fullbright, age, true) {
                    crate::render_fx::skin_effect(mesh, age, (age / 2.).min(1.));
                }
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Garden>().unwrap();
        let Some(s) = &o.saved.scene else {
            return;
        };
        if s.kind != Kind::Arrival {
            return;
        }
        if let Some(t) = s.shell_at {
            if s.phase == Phase::Talk && s.clock.line == 3 {
                self.glow.draw(
                    s.clock.time - t,
                    1.,
                    |_, _| o.data.points["alice_shell_fire"],
                    |_, _, _| true,
                    camera,
                    atmosphere,
                );
            }
        }
        if s.phase == Phase::Leave && s.elapsed() >= 1.6 && !s.skipping {
            let time = s.elapsed() - 1.6;
            self.bubbles.draw(
                time,
                1.,
                |t, _| {
                    let pose = o.data.swim.sample(t, true);
                    self.turtle
                        .looping_tag("tag_shell", "swim", t, pose, 1.)
                        .unwrap_or(pose)
                },
                |_, _, _| true,
                camera,
                atmosphere,
            );
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

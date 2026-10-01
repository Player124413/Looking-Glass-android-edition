use super::*;
use crate::cinematic::Camera;

fn pose(feet: Vec3, target: Vec3) -> Transform {
    let d = target - feet;
    Transform {
        translation: feet,
        rotation: Quat::from_rotation_z(d.y.atan2(d.x)),
    }
}
fn travel(a: Vec3, b: Vec3, time: f32, speed: f32) -> Transform {
    pose(
        a.lerp(b, (time.max(0.) * speed / a.distance(b).max(0.001)).min(1.)),
        b + (b - a),
    )
}
impl PoolTwo {
    pub(super) fn start_ending(&mut self) {
        self.saved.scene.phase = Phase::Ending;
        self.saved.scene.ending = Some(SceneState::new(&ENDING));
    }
    pub(super) fn advance_scene(
        &mut self,
        dt: f32,
        world: &World,
        player: &mut Player,
    ) -> Result<()> {
        let s = &mut self.saved.scene;
        if s.phase == Phase::Dormant {
            return Ok(());
        }
        s.time += dt;
        s.shot_time += dt;
        if s.line >= 1 && s.phase == Phase::Conversation {
            s.run_time += dt;
        }
        if let Some(t) = &mut s.acting {
            *t += dt;
        }
        if s.phase == Phase::LegacyDialogue {
            return Ok(());
        }
        player.velocity = Vec3::ZERO;
        player.script_motion = 1;
        player.cancel_climb();
        player.release_rope();
        if s.phase == Phase::Approach
            && s.time
                >= 0.3
                    + self
                        .data
                        .travel_time("c_bill", "bill_endcinematic_start", "bill_1", "walk")
        {
            if self.saved.dialogue == Dialogue::Read {
                let total = 0.3
                    + self
                        .data
                        .travel_time("c_bill", "bill_endcinematic_start", "bill_1", "walk")
                    + self
                        .data
                        .travel_time("c_bill", "bill_1", "bill_stand", "run");
                if self.saved.scene.time >= total {
                    self.start_ending();
                }
            } else {
                self.saved.dialogue = Dialogue::Playing;
                self.saved.scene.phase = Phase::Conversation;
                self.saved.scene.time = 0.;
            }
        }
        if let Some(e) = &mut self.saved.scene.ending {
            let mut runner = SceneRunner {
                spec: &ENDING,
                state: e,
            };
            runner.capture(player);
            if runner.advance(dt) {
                self.finish(world, player)?;
            }
        }
        Ok(())
    }
    pub(super) fn sync_line(&mut self, story: &Story) {
        let s = &mut self.saved.scene;
        if !matches!(s.phase, Phase::Conversation | Phase::LegacyDialogue) {
            return;
        }
        if let Some((line, time)) = story.progress(DIALOGUE) {
            s.line = line;
            s.line_time = time;
            if line >= 2 {
                s.acting.get_or_insert(0.);
            }
            let shot = match line {
                0 => 0,
                1..=5 => 1,
                6..=8 => 2,
                9 => 4,
                10 if time < 1.5 => 4,
                10..=11 => 5,
                _ => 3,
            };
            if s.shot != shot {
                s.shot = shot;
                s.shot_time = 0.;
            }
        }
    }
    pub(super) fn scene_camera(&self) -> Option<Camera> {
        let s = &self.saved.scene;
        if s.phase == Phase::Approach && self.saved.dialogue == Dialogue::Read {
            let first = 0.3
                + self
                    .data
                    .travel_time("c_bill", "bill_endcinematic_start", "bill_1", "walk");
            if s.time >= first {
                return Some(self.data.tracks[0].camera(s.time - first));
            }
        }
        match s.phase {
            Phase::Dormant => None,
            Phase::Approach | Phase::Conversation | Phase::LegacyDialogue => {
                if s.shot == 0 {
                    // The initial func_camera has its authored yaw, before a path is assigned.
                    let eye = self.data.points["bill_cam"];
                    Some(Camera::look(eye, eye + vec3(-1., 1., 0.) * 100.))
                } else {
                    Some(self.data.tracks[s.shot - 1].camera(s.shot_time))
                }
            }
            Phase::Ending | Phase::Done => {
                let t = s.ending.as_ref()?.time;
                if t < 4. {
                    return Some(self.data.tracks[2].camera(s.shot_time));
                }
                let target = if t < 5.6 {
                    self.bill_pose().translation
                } else {
                    self.alice_pose().translation
                };
                Some(Camera::look(
                    self.data.points["suck_cam"],
                    target + Vec3::Z * 36.,
                ))
            }
        }
    }
    pub(super) fn bill_pose(&self) -> Transform {
        self.footing(self.bill_path(), vec3(8., 8., 16.))
    }
    fn footing(&self, mut at: Transform, half: Vec3) -> Transform {
        at.translation = self
            .data
            .world
            .actor_footing(at.translation, Vec3::Z * half.z, half, 256.)
            .unwrap_or(at.translation);
        at
    }
    fn bill_path(&self) -> Transform {
        let p = &self.data.points;
        let s = &self.saved.scene;
        if s.phase == Phase::Approach && self.saved.dialogue == Dialogue::Read {
            let first = 0.3
                + self
                    .data
                    .travel_time("c_bill", "bill_endcinematic_start", "bill_1", "walk");
            if s.time >= first {
                return travel(
                    p["bill_1"],
                    self.bill,
                    s.time - first,
                    self.data.speed("c_bill", "run"),
                );
            }
        }
        match s.phase {
            Phase::Approach => travel(
                p["bill_endcinematic_start"],
                p["bill_1"],
                s.time - 0.3,
                self.data.speed("c_bill", "walk"),
            ),
            Phase::Conversation if s.line == 0 => pose(p["bill_1"], self.alice),
            Phase::Conversation => {
                let mut at = travel(
                    p["bill_1"],
                    self.bill,
                    s.run_time,
                    self.data.speed("c_bill", "run"),
                );
                if at.translation.distance(self.bill) < 0.1 {
                    at = pose(self.bill, self.alice);
                }
                at
            }
            Phase::Ending | Phase::Done => {
                let t = s.ending.as_ref().unwrap().time;
                let top = p["alice_gets_sucked_in"];
                if t < 4.4 {
                    travel(self.bill, top, t, self.data.speed("c_bill", "walk"))
                } else {
                    travel(
                        top,
                        p["bill_leads_the_way"],
                        t - 4.4,
                        self.data.speed("c_bill", "paniked_run"),
                    )
                }
            }
            _ => pose(self.bill, self.alice),
        }
    }
    pub(super) fn alice_pose(&self) -> Transform {
        let at = self.alice_path();
        if self
            .saved
            .scene
            .ending
            .as_ref()
            .is_some_and(|e| e.time >= 5.6)
        {
            at
        } else {
            self.footing(at, crate::collision::PLAYER_HALF)
        }
    }
    fn alice_path(&self) -> Transform {
        let s = &self.saved.scene;
        if let Some(e) = &s.ending {
            let top = self.data.points["alice_gets_sucked_in"];
            if e.time < 5.6 {
                return travel(
                    self.alice,
                    top,
                    e.time - 4.,
                    self.data.speed("alice", "walk"),
                );
            }
            // The native push volume applies +Y speed 200 to the cinematic actor.
            let feet = top + Vec3::Y * (200. * (e.time - 5.8).clamp(0., 2.));
            return pose(feet, feet - Vec3::Y);
        }
        pose(self.alice, self.bill_pose().translation)
    }
    fn door_angle(&self) -> f32 {
        let Some(e) = &self.saved.scene.ending else {
            return 0.;
        };
        let t = e.time - 4.;
        if e.finished || t < 0. {
            return 0.;
        }
        if t >= 1.8 {
            return 90.;
        }
        let tick = (t / 0.1).floor() as i32;
        let amplitude = 2. * (4 + tick / 2).min(12) as f32;
        if tick % 2 == 0 {
            -amplitude
        } else {
            amplitude
        }
    }
    pub(super) fn door_poses(&self) -> Vec<(usize, Vec3, Quat)> {
        self.data
            .doors
            .iter()
            .enumerate()
            .map(|(i, &(model, base, pivot))| {
                let angle = self.door_angle() * if i == 0 { 1. } else { -1. };
                let r = Quat::from_rotation_z(angle.to_radians());
                (model, pivot + r * (base - pivot), r)
            })
            .collect()
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        self.pond.rebuild(map, &self.saved.pond)?;
        self.colliders = self
            .door_poses()
            .into_iter()
            .map(|(m, p, r)| Collider::model(map, m, p, r, true))
            .collect::<Result<_>>()?;
        Ok(())
    }
    fn acting(&self, model: &str) -> (&'static str, f32, bool) {
        let s = &self.saved.scene;
        if model == "c_bill" {
            if s.phase == Phase::Approach && self.saved.dialogue == Dialogue::Read {
                let first = 0.3
                    + self
                        .data
                        .travel_time("c_bill", "bill_endcinematic_start", "bill_1", "walk");
                if s.time >= first {
                    return ("run", s.time - first, true);
                }
            }
            match s.phase {
                Phase::Approach => return ("walk", (s.time - 0.3).max(0.), true),
                Phase::Conversation
                    if s.line > 0
                        && s.run_time
                            < self
                                .data
                                .travel_time("c_bill", "bill_1", "bill_stand", "run") =>
                {
                    return ("run", s.run_time, true)
                }
                Phase::Ending | Phase::Done => {
                    let t = s.ending.as_ref().unwrap().time;
                    return if t >= 4.8 {
                        ("paniked_run", t - 4.8, true)
                    } else if t >= 4.4 {
                        ("run", t - 4.4, true)
                    } else if t < self.data.travel_time(
                        "c_bill",
                        "bill_stand",
                        "alice_gets_sucked_in",
                        "walk",
                    ) {
                        ("walk", t, true)
                    } else {
                        ("idle_01", t, true)
                    };
                }
                _ => {}
            }
        } else if let Some(e) = &s.ending {
            return if e.time >= 5.8 {
                ("held", e.time - 5.8, true)
            } else if e.time >= 4. {
                ("walk", e.time - 4., true)
            } else {
                ("idle_stand", e.time, true)
            };
        }
        let Some(mut time) = s.acting else {
            return (
                if model == "c_bill" {
                    "idle_01"
                } else {
                    "idle_base_02"
                },
                s.time,
                true,
            );
        };
        let steps: &[(&str, Option<f32>)] = if model == "c_bill" {
            &[
                ("idle_01", Some(12.)),
                ("idle_liftbelt", None),
                ("idle_01", Some(4.)),
                ("talk_shrug", None),
                ("idle_01", Some(2.)),
                ("idle_eyes", None),
                ("idle_01", Some(14.)),
                ("talk_fist", None),
                ("idle_01", Some(4.)),
                ("talk_shrug", None),
                ("idle_liftbelt", None),
                ("idle_01", Some(3.)),
                ("talk_no", None),
            ]
        } else {
            &[
                ("idle_base_02", Some(6.)),
                ("idle_base_02_2_stand", None),
                ("idle_stand", Some(8.)),
                ("idle_stand_nodyes", None),
                ("idle_stand", Some(10.)),
                ("idle_stand_shakeno", None),
                ("idle_stand", Some(12.)),
                ("idle_stand_tiptoes", None),
                ("idle_stand_nodyes", None),
            ]
        };
        for &(clip, duration) in steps {
            let d = duration.unwrap_or_else(|| self.data.duration(model, clip));
            if time < d {
                return (clip, time, duration.is_some());
            }
            time -= d;
        }
        (
            if model == "c_bill" {
                "idle_01"
            } else {
                "idle_stand"
            },
            time,
            true,
        )
    }
}

pub(super) struct Art {
    pub bill: Puppet,
    pub alice: Puppet,
    guards: [Puppet; 2],
    shots: [crate::weapons::Prop; 2],
    belt: crate::weapons::Prop,
    pub clock: f32,
    leaf: crate::weapons::Prop,
    lily: crate::weapons::Prop,
    fish: crate::weapons::Prop,
    splash: crate::weapons::Prop,
    spray: crate::particles::Attached,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        Ok(Self {
            bill: Puppet::load(assets, "c_bill", BILL_CLIPS, &specs)?,
            alice: Puppet::load(assets, "alice", ALICE_CLIPS, &specs)?,
            guards: [
                Puppet::load(assets, "c_armyant", guards::REGULAR, &specs)?,
                Puppet::load(assets, "c_armyantcorp", guards::CORPORAL, &specs)?,
            ],
            shots: [
                crate::weapons::Prop::load_animation(assets, "prj_bullet", "idle", &specs)?,
                crate::weapons::Prop::load_animation(assets, "prj_grenade", "idle", &specs)?,
            ],
            clock: 0.,
            leaf: crate::weapons::Prop::load(assets, "leaf_ride", &specs)?,
            lily: crate::weapons::Prop::load(assets, "lilypad", &specs)?,
            fish: crate::weapons::Prop::load_animation(assets, "fish_head", "attack", &specs)?,
            splash: crate::weapons::Prop::load_animation(
                assets,
                "fx_watersplash2",
                "idle",
                &specs,
            )?,
            spray: crate::particles::Attached::load_clip_bursts(
                assets,
                "fx_watersplash2",
                Some("idle"),
                0.05,
                &specs,
            )?
            .context("Missing fish splash")?,
            material: crate::character::skin_material()?,
            belt: crate::weapons::Prop::load_animation(assets, "tool_belt", "tool_belt", &specs)?,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, story: &Story) {
        self.bill.mouth(story.mouth(&["billthelizard"]));
        self.alice.mouth(story.mouth(&["fakeplayer"]));
        let (line, time) = story.progress(DIALOGUE).unwrap_or_default();
        self.bill.mouth_angle(if line == 0 { 10. } else { 45. });
        self.clock = time;
    }
    fn draw(
        &mut self,
        level: &dyn LevelController,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        let Some(o) = level.downcast_ref::<PoolTwo>() else {
            return;
        };
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        for (i, p) in o.pond.props.iter().enumerate() {
            if transport::Data::prop_visible(&o.saved.pond, i) {
                let art = if p.model == "lilypad" {
                    &mut self.lily
                } else {
                    &mut self.leaf
                };
                art.draw_frame(p.at, p.scale, fullbright, o.saved.pond.age as f32, true);
            }
        }
        if let Some(t) = o.saved.fish.attack {
            if t < 1. {
                self.alice.atmosphere(atmosphere, camera);
                self.alice.draw(
                    "held",
                    t as f32,
                    true,
                    Transform {
                        translation: o.saved.fish.at,
                        rotation: Quat::IDENTITY,
                    },
                    1.,
                    fullbright,
                );
                self.material.bind();
            }
            if t >= 0.5 {
                self.fish.draw_frame(
                    Transform {
                        translation: o.saved.fish.at - Vec3::X * 64.,
                        rotation: Quat::IDENTITY,
                    },
                    1.,
                    fullbright,
                    t as f32 - 0.5,
                    false,
                );
                if t < 1.5 {
                    self.splash.draw_frame(
                        Transform {
                            translation: o.saved.fish.at + Vec3::Z * 64.,
                            rotation: Quat::IDENTITY,
                        },
                        1.5,
                        fullbright,
                        t as f32 - 0.5,
                        false,
                    );
                }
            }
        }
        for g in &o.saved.guards {
            let p = &mut self.guards[usize::from(g.corporal)];
            p.atmosphere(atmosphere, camera);
            p.draw(
                g.clip(),
                if g.frozen {
                    g.time.min(o.data.frame(g.model(), g.clip()) * 5.)
                } else {
                    g.time
                },
                g.loops(),
                Transform {
                    translation: g.feet,
                    rotation: Quat::from_rotation_z(g.yaw),
                },
                g.visual_scale(&o.data),
                fullbright,
            );
            for shot in &g.shots {
                let rotation = if shot.grenade {
                    Quat::from_euler(
                        EulerRot::XYZ,
                        (100. * shot.age).to_radians(),
                        (200. * shot.age).to_radians(),
                        (300. * shot.age).to_radians(),
                    )
                } else {
                    Quat::from_rotation_arc(Vec3::X, shot.direction)
                };
                self.shots[usize::from(shot.grenade)].draw_frame(
                    Transform {
                        translation: shot.at,
                        rotation,
                    },
                    1.,
                    fullbright,
                    shot.age,
                    true,
                );
            }
            crate::ant::draw_blasts(g);
        }
        let s = &o.saved.scene;
        if matches!(s.phase, Phase::Dormant | Phase::Done) || s.skipping {
            return;
        }
        let bill_visible = s.phase != Phase::Approach || s.time >= 0.1;
        for (p, model, at) in [
            (&mut self.bill, "c_bill", o.bill_pose()),
            (&mut self.alice, "alice", o.alice_pose()),
        ] {
            if model == "c_bill" && !bill_visible {
                continue;
            }
            let (clip, time, looping) = o.acting(model);
            p.atmosphere(atmosphere, camera);
            p.draw(clip, time, looping, at, 1., fullbright);
        }
        // Bill's local declaration omits attachment scale; the generic legacy
        // reader handles only explicit scales. Bind this reviewed prop here.
        if bill_visible {
            let (clip, time, looping) = o.acting("c_bill");
            let tag = if looping {
                self.bill
                    .looping_tag("tag_tools", clip, time, o.bill_pose(), 1.)
            } else {
                self.bill.tag("tag_tools", clip, time, o.bill_pose(), 1.)
            };
            if let Some(at) = tag {
                self.belt.draw_frame(at, 1., fullbright, time, true);
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<PoolTwo>().unwrap();
        if let Some(t) = o.saved.fish.attack.filter(|t| *t >= 0.5) {
            self.spray.draw(
                t as f32 - 0.5,
                1.5,
                |_, _| Transform {
                    translation: o.saved.fish.at + Vec3::Z * 64.,
                    rotation: Quat::IDENTITY,
                },
                |_, _, _| true,
                camera,
                atmosphere,
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

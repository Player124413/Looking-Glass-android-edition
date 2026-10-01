use super::*;
use crate::{npc::Puppet, weapons::Prop};
use std::collections::BTreeMap;
pub(super) struct Art {
    ui: std::rc::Rc<crate::ui::Ui>,
    puppets: BTreeMap<String, Puppet>,
    props: BTreeMap<String, Prop>,
    halo: Prop,
    essence: crate::loot_art::Art,
    particles: BTreeMap<String, crate::particles::Attached>,
    effects: effects::Art,
}
impl Art {
    pub fn load(a: &mut Assets, o: &Queen) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let mut puppets = BTreeMap::new();
        for (name, rig) in &o.data.rigs {
            puppets.insert(
                name.clone(),
                Puppet::load(
                    a,
                    name,
                    &rig.clips.keys().map(String::as_str).collect::<Vec<_>>(),
                    &specs,
                )
                .with_context(|| format!("Finale actor {name}"))?,
            );
        }
        let mut props = BTreeMap::new();
        for (model, clips) in [
            (
                "c_queen1_throne",
                &["idle_throne1", "throne_break1", "throne_break_2"][..],
            ),
            ("c_queen1_mask", &["idle"][..]),
            ("c_queen1_popup", &["popup"][..]),
            ("fx_queenlight", &["idle", "rise"][..]),
            ("fx_cone", &["idle"][..]),
            ("fx_wave", &["idle"][..]),
        ] {
            for clip in clips {
                props.insert(
                    format!("{model}/{clip}"),
                    Prop::load_animation(a, model, clip, &specs)
                        .with_context(|| format!("Finale prop {model}/{clip}"))?,
                );
            }
        }
        let mut hidden_projectiles = std::collections::BTreeSet::new();
        for model in o.data.projectiles.keys() {
            let name = model.trim_end_matches(".tik");
            let source = String::from_utf8_lossy(&a.read(&format!("models/{model}"))?).into_owned();
            if crate::materials::lines(&source)
                .iter()
                .any(|row| row.first().is_some_and(|s| s == "hide"))
            {
                // Scream damage carriers are deliberately hidden; the body owns
                // the visible mouth emitter and wave animation commands.
                hidden_projectiles.insert(name.to_string());
                continue;
            }
            // Some original projectile definitions are emitter-only; their particles
            // are rendered by the effect pass below rather than a nonexistent mesh.
            if let Ok(p) = Prop::load(a, name, &specs) {
                props.insert(name.into(), p);
            }
        }
        puppets.get_mut("c_q2_body").unwrap().mouth_angle(45.);
        puppets.get_mut("c_queen1").unwrap().show_attachments(false);
        let mut particles = BTreeMap::new();
        for name in o
            .data
            .projectiles
            .keys()
            .map(|s| s.trim_end_matches(".tik"))
            .chain([
                "fx_queenlight",
                "alice_halo",
                "fx_queen1_sphere",
                "fx_queen_bspurt1",
                "fx_bspurt4",
                "fx_bspurt5",
            ])
        {
            if let Some(p) = crate::particles::Attached::load(a, name, &specs)? {
                particles.insert(name.into(), p);
            }
        }
        for model in o.data.projectiles.keys() {
            let name = model.trim_end_matches(".tik");
            ensure!(
                props.contains_key(name)
                    || particles.contains_key(name)
                    || hidden_projectiles.contains(name),
                "Finale projectile {model} has neither a mesh nor an effect"
            );
        }
        Ok(Self {
            ui: crate::ui::Ui::load(a)?,
            effects: effects::Art::load(a, o)?,
            puppets,
            props,
            essence: crate::loot_art::Art::load(a, &specs)?,
            particles,
            halo: Prop::load(a, "alice_halo", &specs)?,
        })
    }
    fn actor(
        &mut self,
        name: &str,
        clip: &str,
        time: f32,
        looping: bool,
        pose: Transform,
        bright: bool,
    ) {
        self.puppets
            .get_mut(name)
            .unwrap()
            .draw(clip, time, looping, pose, 1., bright);
    }
    fn prop(&mut self, name: &str, pose: Transform, time: f32, looping: bool, bright: bool) {
        if let Some(p) = self.props.get_mut(name) {
            p.draw_frame(pose, 1., bright, time, looping);
        }
    }
    fn mesh_effect(&mut self, name: &str, pose: Transform, time: f32, scale: f32, alpha: f32) {
        if let Some(p) = self.props.get_mut(name) {
            for mesh in p.meshes_at(pose, scale, true, time, true) {
                crate::render_fx::skin_effect(mesh, time, alpha);
            }
        }
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        for (name, actor) in [
            ("alice", "alice"),
            ("c_queen1", "queen"),
            ("c_q2_body", "body"),
        ] {
            self.puppets.get_mut(name).unwrap().mouth(s.mouth(&[actor]));
        }
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        let o = l.downcast_ref::<Queen>().unwrap();
        let s = &o.saved;
        for p in self.puppets.values() {
            p.atmosphere(atmosphere, camera);
        }
        if matches!(s.phase, Phase::Corridor | Phase::Intro | Phase::Queen1)
            || (s.phase == Phase::Birth && s.time < o.data.pull_end())
        {
            let (q, qt, t, tt) = o.queen1_performance();
            for n in 0..4 {
                let cut = s.queen1 < 1100.
                    && (s.attack != battle::Attack::Wound
                        || s.attack_time
                            >= ((n + 1) * 4) as f32
                                * o.data.rigs["c_queen1"].clips["pain_ready_2_wounded"].frame_time);
                let queen = self.puppets.get_mut("c_queen1").unwrap();
                queen.surface_visible(&format!("material{}", 3 + n), !cut);
                queen.surface_visible(&format!("material{}", 13 + n), cut);
            }
            self.actor(
                "c_queen1_bigtent",
                t,
                tt,
                s.phase == Phase::Queen1,
                o.data.points["throne"],
                fullbright,
            );
            self.actor(
                "c_queen1",
                q,
                qt,
                matches!(s.phase, Phase::Corridor | Phase::Queen1),
                o.queen1_pose(),
                fullbright,
            );
            if s.phase == Phase::Corridor
                || (s.phase == Phase::Intro
                    && (q != "thronelift"
                        || qt < 60. * o.data.rigs["c_queen1"].clips["thronelift"].frame_time))
            {
                let mask = o.data.rigs["c_queen1"].tag("tag_mask", q, qt, o.queen1_pose(), true);
                self.prop("c_queen1_mask/idle", mask, qt, true, fullbright);
            }
        }
        if matches!(
            s.phase,
            Phase::Corridor | Phase::Intro | Phase::Queen1 | Phase::Birth
        ) {
            let (clip, t) = if s.phase == Phase::Birth && s.time >= o.data.pull_end() {
                ("throne_break_2", s.time - o.data.pull_end())
            } else if s.phase != Phase::Corridor
                && (s.phase != Phase::Intro || s.time > 6. + o.data.intro_line)
            {
                (
                    "throne_break1",
                    if s.phase == Phase::Intro {
                        s.time - 6. - o.data.intro_line
                    } else {
                        100.
                    },
                )
            } else {
                ("idle_throne1", s.time)
            };
            self.prop(
                &format!("c_queen1_throne/{clip}"),
                o.data.points["throne"],
                t,
                false,
                fullbright,
            );
        }
        if s.phase == Phase::Corridor && s.time < 2.5 {
            let n = (s.time / 0.5) as usize + 1;
            self.prop(
                "c_queen1_popup/popup",
                o.data.points[&format!("qlair_popup{n}")],
                s.time % 0.5,
                false,
                fullbright,
            );
        }
        if s.phase == Phase::Queen1
            && s.attack == battle::Attack::Popup
            && s.attack_time >= 2.25
            && s.fired & 8 == 0
        {
            let p = o.popup_pose();
            self.prop(
                "c_queen1_popup/popup",
                p,
                s.attack_time - 2.25,
                false,
                fullbright,
            );
        }
        if matches!(s.phase, Phase::Birth | Phase::Queen2 | Phase::Death) {
            let (clip, t, pose) = o.body_performance();
            self.actor(
                "c_q2_body",
                clip,
                t,
                matches!(s.phase, Phase::Birth | Phase::Queen2),
                pose,
                fullbright,
            );
            let halo_pose = o.data.rigs["c_q2_body"].tag("tag_halo", clip, t, pose, true);
            let (halo_clip, halo_time) = o.halo_performance();
            self.actor(
                "c_q2_halo",
                halo_clip,
                halo_time,
                s.phase != Phase::Death,
                halo_pose,
                fullbright,
            );
            if s.phase != Phase::Birth || s.time >= o.data.reveal() {
                for n in 0..4 {
                    if s.parts[n] == 0. {
                        continue;
                    }
                    let name = format!("c_q2_t0{}", n + 1);
                    let (clip, t) = o.part_animation(n);
                    if n == 1 {
                        let p = self.puppets.get_mut(&name).unwrap();
                        p.watch(s.head_watch.clone());
                        p.bone_scales(&o.data.rigs[&name].scales(clip, t));
                        for (surface, active) in [
                            ("MATERIAL11", clip.contains("jabberwock")),
                            ("MATERIAL13", clip.contains("centipede")),
                            ("MATERIAL14", clip.contains("centipede")),
                            ("MATERIAL15", clip.contains("hatter")),
                        ] {
                            p.surface_visible(surface, active);
                        }
                    }
                    self.actor(&name, clip, t, s.parts[n] > 25., o.part_pose(n), fullbright);
                }
            }
        }
        if matches!(s.phase, Phase::Intro | Phase::Birth) {
            let (clip, t, p) = o.alice_performance();
            self.actor("alice", clip, t, true, p, fullbright);
            if s.powered {
                let at = self.puppets["alice"]
                    .looping_tag("tag_gut", clip, t, p, 1.)
                    .unwrap_or(p);
                self.halo.draw_frame(at, 1., fullbright, s.clock, true);
            }
        } else if s.powered {
            let mut p = s.alice;
            p.translation.z += 30.;
            self.halo.draw_frame(p, 1., fullbright, s.clock, true);
        }
        if s.phase == Phase::Birth && (16. ..o.data.pull_end()).contains(&s.time) {
            self.prop(
                "fx_queenlight/rise",
                o.data.points["alice_magic_power"],
                s.time - 16.,
                false,
                fullbright,
            );
        }
        if matches!(s.phase, Phase::Queen1 | Phase::Queen2) && s.essence_wait == 0. {
            self.essence.draw(if s.phase == Phase::Queen1 { crate::loot::Grade::Medium } else { crate::loot::Grade::Large }, o.essence_pose().translation, atmosphere, camera, s.clock);
        }
        for p in &s.projectiles {
            if p.end.is_none() {
                self.prop(
                    p.model_key(),
                    Transform {
                        translation: p.at,
                        rotation: Quat::from_rotation_arc(Vec3::X, p.velocity.normalize_or_zero()),
                    },
                    p.age,
                    true,
                    fullbright,
                );
            }
        }
    }
    fn effects(
        &mut self,
        l: &dyn LevelController,
        camera: Vec3,
        atmosphere: &crate::environment::Atmosphere,
    ) {
        let o = l.downcast_ref::<Queen>().unwrap();
        let s = &o.saved;
        // These authored model particles are not sprite emitters. Their hidden
        self.effects.draw(o, camera, atmosphere);
        // damage carriers are separate from the visible expanding cone/wave.
        if s.phase == Phase::Queen2 && s.attack == battle::Attack::Scream && s.attack_variant == 0 {
            let rig = &o.data.rigs["c_q2_body"];
            let clip = "attack_slit_scream_fire1";
            let t = s.attack_time - rig.duration("attack_slit_scream_open");
            let end = 38. * rig.clips[clip].frame_time;
            for n in 0..=(end * 10.) as usize {
                let birth = n as f32 / 10.;
                let age = t - birth;
                if !(0. ..1.).contains(&age) {
                    continue;
                }
                let mut pose = rig.tag("tag_slit", clip, birth, s.body, false);
                let direction =
                    (s.attack_target + Vec3::Z * 32. - pose.translation).normalize_or_zero();
                pose.translation += direction * 800. * age;
                pose.rotation = Quat::from_rotation_arc(Vec3::X, direction);
                self.mesh_effect("fx_cone/idle", pose, age, 1. + 4. * age, 0.4 * (1. - age));
            }
        }
        for shot in &s.projectiles {
            if shot.is_scream_wave() {
                self.mesh_effect(
                    "fx_wave/idle",
                    Transform {
                        translation: shot.at,
                        rotation: Quat::from_rotation_arc(
                            Vec3::X,
                            shot.velocity.normalize_or_zero(),
                        ),
                    },
                    shot.age,
                    3. + 4. * shot.age,
                    0.5 * (1. - shot.age / 2.).max(0.),
                );
            }
        }
        if matches!(s.phase, Phase::Corridor | Phase::Intro | Phase::Queen1)
            || (s.phase == Phase::Birth && s.time < o.data.pull_end())
        {
            let (q, qt, t, tt) = o.queen1_performance();
            self.puppets.get_mut("c_queen1").unwrap().draw_effects(
                q,
                qt,
                s.phase == Phase::Queen1,
                o.queen1_pose(),
                1.,
                camera,
                atmosphere,
            );
            self.puppets
                .get_mut("c_queen1_bigtent")
                .unwrap()
                .draw_effects(
                    t,
                    tt,
                    s.phase == Phase::Queen1,
                    o.data.points["throne"],
                    1.,
                    camera,
                    atmosphere,
                );
        }
        if matches!(s.phase, Phase::Birth | Phase::Queen2 | Phase::Death) {
            let (clip, t, p) = o.body_performance();
            self.puppets.get_mut("c_q2_body").unwrap().draw_effects(
                clip,
                t,
                s.phase != Phase::Death,
                p,
                1.,
                camera,
                atmosphere,
            );
            let halo_pose =
                o.data.rigs["c_q2_body"].tag("tag_halo", clip, t, p, s.phase != Phase::Death);
            let (halo_clip, halo_time) = o.halo_performance();
            self.puppets.get_mut("c_q2_halo").unwrap().draw_effects(
                halo_clip,
                halo_time,
                s.phase != Phase::Death,
                halo_pose,
                1.,
                camera,
                atmosphere,
            );
            for n in 0..4 {
                if s.parts[n] == 0. {
                    continue;
                }
                if s.phase != Phase::Birth || s.time >= o.data.reveal() {
                    let (clip, t) = o.part_animation(n);
                    self.puppets
                        .get_mut(&format!("c_q2_t0{}", n + 1))
                        .unwrap()
                        .draw_effects(
                            clip,
                            t,
                            s.parts[n] > 25.,
                            o.part_pose(n),
                            1.,
                            camera,
                            atmosphere,
                        );
                    if s.parts[n] <= 25. {
                        let tags = match n {
                            0 => ["bone_t01_02", "bone_t01_03", "bone_t01_04", "tag_claw"],
                            1 => ["bone_t02_02", "bone_t02_03", "b_t02_04", "tag_jab_mouth"],
                            2 => ["bone_t03_02", "bone_t03_03", "bone_t03_04", "tag_tip"],
                            _ => ["bone_t04_02", "bone_t04_03", "bone_t04_05", "Bone30"],
                        };
                        for (i, tag) in tags.iter().enumerate() {
                            if let Some(fx) = self.particles.get(if i % 2 == 0 {
                                "fx_bspurt4"
                            } else {
                                "fx_bspurt5"
                            }) {
                                let mut fx = fx.fork();
                                fx.draw(
                                    t,
                                    2.,
                                    |at, _| {
                                        o.data.rigs[&format!("c_q2_t0{}", n + 1)].tag(
                                            tag,
                                            "death",
                                            at,
                                            o.part_pose(n),
                                            false,
                                        )
                                    },
                                    |_, _, default| default,
                                    camera,
                                    atmosphere,
                                );
                            }
                        }
                    }
                }
            }
        }
        if s.powered {
            if let Some(fx) = self.particles.get_mut("alice_halo") {
                let mut p = s.alice;
                p.translation.z += 30.;
                let halo = &self.halo;
                fx.draw(
                    s.clock,
                    1.,
                    |_, tag| Transform {
                        translation: halo.point(p, tag.unwrap_or("tag_halo1"), 1.),
                        rotation: p.rotation,
                    },
                    |_, _, _| true,
                    camera,
                    atmosphere,
                );
            }
        }
        for p in &s.projectiles {
            if p.end.is_none() {
                if let Some(fx) = self.particles.get(p.model_key()) {
                    let mut fx = fx.fork();
                    fx.draw(
                        p.age,
                        1.,
                        |t, _| p.birth_pose(t),
                        |_, _, _| true,
                        camera,
                        atmosphere,
                    );
                }
            }
        }
        if s.phase == Phase::Birth && (16. ..o.data.pull_end()).contains(&s.time) {
            if let Some(fx) = self.particles.get_mut("fx_queenlight") {
                fx.draw(
                    s.time - 16.,
                    1.,
                    |_, _| o.data.points["alice_magic_power"],
                    |_, _, _| true,
                    camera,
                    atmosphere,
                );
            }
        }
        if s.phase == Phase::Death {
            let (clip, t, p) = o.body_performance();
            let tags = match clip {
                "death_1endloop" => ["tag_deathspray01", "tag_deathspray03"],
                "death_2endloop" => ["tag_deathspray02", "tag_deathspray04"],
                "death_3endloop" => ["tag_deathspray03", "tag_deathspray05"],
                _ => ["", ""],
            };
            if let Some(fx) = self.particles.get("fx_queen_bspurt1") {
                for tag in tags.into_iter().filter(|s| !s.is_empty()) {
                    let mut fx = fx.fork();
                    fx.draw(
                        t,
                        1.,
                        |at, _| o.data.rigs["c_q2_body"].tag(tag, clip, at, p, false),
                        |_, _, _| true,
                        camera,
                        atmosphere,
                    );
                }
            }
        }
        if matches!(
            s.attack,
            battle::Attack::SlamLeft | battle::Attack::SlamRight | battle::Attack::Grab
        ) && s.attack_time >= o.tele_prep()
        {
            if let Some(fx) = self.particles.get_mut("fx_queen1_sphere") {
                let t = s.attack_time - o.tele_prep();
                fx.draw(
                    t,
                    1.,
                    |_, _| Transform {
                        translation: if s.grabbed {
                            s.alice.translation + Vec3::Z * 64.
                        } else {
                            s.attack_target + Vec3::Z * 32.
                        },
                        rotation: Quat::IDENTITY,
                    },
                    |name, at, _| {
                        if s.grabbed {
                            name == "pullsparkle"
                        } else {
                            name == "sparkle" && at < 1.666
                        }
                    },
                    camera,
                    atmosphere,
                );
            }
        }
    }
    fn hud(&mut self, l: &dyn LevelController) {
        let o = l.downcast_ref::<Queen>().unwrap();
        let remaining = match o.saved.phase {
            Phase::Queen1 if o.saved.queen1 > 0. => o.saved.queen1 / 2500.,
            // The second form dies at the authored 1,000-health threshold.
            Phase::Queen2 if o.saved.queen2 >= 1000. => (o.saved.queen2 - 1000.) / 3500.,
            _ => return,
        };
        crate::hud::boss_meter(&self.ui, "QUEEN OF HEARTS", remaining);
    }
    fn handoff_pose(&self) -> Option<&crate::cinematic::ActorPose> {
        self.puppets["alice"].handoff_pose()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

use super::*;
use crate::combat::{self, DamageKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Attack {
    Idle,
    Pain,
    Wound,
    Popup,
    Spit,
    Combo,
    Sweep,
    SlamLeft,
    SlamRight,
    Grab,
    Ice,
    Claw,
    Club,
    Centipede,
    JabberEye,
    JabberSpit,
    Hatter,
    Spore,
    Scream,
}
pub(super) fn default_delay() -> f32 {
    1.5
}
/// Tracking beams aim at the opponent. Sweeps retain the animated tag's
/// horizontal direction while pitching toward the opponent's elevation.
pub(super) fn beam_end(source: Transform, target: Vec3, range: f32, tracking: bool) -> Vec3 {
    let offset = target - source.translation;
    if tracking {
        return target;
    }
    let forward = source.rotation * Vec3::X;
    let direction = vec3(
        forward.x * offset.length(),
        forward.y * offset.length(),
        offset.z,
    )
    .normalize_or_zero();
    source.translation + direction * range
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Shot {
    pub model: String,
    pub at: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub damage: f32,
    pub force: f32,
    pub life: f32,
    pub end: Option<Vec3>,
    #[serde(default = "projectile::first_seek")]
    pub seek_at: f32,
    #[serde(default)]
    pub victim: Option<usize>,
    #[serde(default)]
    pub trail: Vec<(f32, Transform)>,
}
impl Shot {
    pub fn model_key(&self) -> &str {
        self.model.trim_end_matches(".tik")
    }
    pub fn is_scream_wave(&self) -> bool {
        self.model_key() == "fx_queen_wave"
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.seek_at.is_finite()
                && (0. ..=16.).contains(&self.seek_at)
                && self.trail.len() <= 128
                && self.trail.iter().all(|(t, p)| t.is_finite()
                    && *t >= 0.
                    && *t <= self.age + 0.001
                    && p.translation.is_finite()
                    && p.rotation.is_finite()
                    && (p.rotation.length_squared() - 1.).abs() < 0.01)
                && self.trail.windows(2).all(|p| p[0].0 < p[1].0),
            "Invalid projectile steering/trail state"
        );
        ensure!(
            self.at.is_finite()
                && self.at.abs().max_element() < 100000.
                && self.velocity.is_finite()
                && self.velocity.length() < 10000.
                && self
                    .end
                    .is_none_or(|e| e.is_finite() && e.abs().max_element() < 100000.),
            "Invalid Queen projectile pose"
        );
        ensure!(
            self.model.len() < 100 && !self.model.contains(".."),
            "Invalid Queen projectile model"
        );
        for (n, v, max) in [
            ("shot age", self.age, 15.),
            ("shot life", self.life, 15.),
            ("shot damage", self.damage, 1000.),
            ("shot impulse", self.force, 4000.),
        ] {
            state::clock(n, v, max)?;
        }
        Ok(())
    }
}
impl Queen {
    pub(super) fn popup_pose(&self) -> Transform {
        let end = self.saved.attack_target - Vec3::Z * 32.;
        let origin = self.queen1_pose().translation;
        let from = origin + (end - origin).normalize_or_zero() * 100.;
        Transform {
            translation: from.lerp(end, ((self.saved.attack_time - 1.) / 1.25).clamp(0., 1.)),
            rotation: Quat::IDENTITY,
        }
    }
    pub(super) fn attack_part(attack: Attack) -> Option<usize> {
        match attack {
            Attack::Claw => Some(0),
            Attack::Centipede | Attack::JabberEye | Attack::JabberSpit | Attack::Hatter => Some(1),
            Attack::Club => Some(3),
            _ => None,
        }
    }
    // Ordered independent chance checks, visibility/range and ready-part gates.
    // This is reviewed encounter logic; no original state-machine text is interpreted.
    pub(super) fn choose_attack(&mut self, first: bool, distance: f32, visible: bool) -> Attack {
        if first {
            if visible && self.random() < 0.35 {
                return Attack::Popup;
            }
            if visible && self.random() < 0.7 {
                return if self.random() < 0.333 {
                    Attack::Spit
                } else if self.random() < 0.666 {
                    Attack::Combo
                } else {
                    Attack::Sweep
                };
            }
            if self.random() >= 0.95 {
                return Attack::Idle;
            }
            return if self.random() < 0.2 {
                Attack::SlamLeft
            } else if self.random() < 0.4 {
                Attack::SlamRight
            } else if self.random() < 0.66 {
                Attack::Grab
            } else if visible {
                Attack::Ice
            } else {
                Attack::Idle
            };
        }
        let ready = |n: usize| self.saved.parts[n] > 25. && self.saved.part_attack[n].is_none();
        let (claw, head, club) = (ready(0), ready(1), ready(3));
        if distance < 1000. && ((claw && self.random() < 0.75) || (club && self.random() < 0.75)) {
            return if claw && (self.random() < 0.5 || !club) {
                Attack::Claw
            } else {
                Attack::Club
            };
        }
        if head && ((distance < 1000. && self.random() < 0.9) || self.random() < 0.666) {
            return if self.random() < 0.333 {
                Attack::Centipede
            } else if self.random() < 0.666 {
                if self.random() < 0.5 {
                    Attack::JabberSpit
                } else {
                    Attack::JabberEye
                }
            } else {
                Attack::Hatter
            };
        }
        if self.random() < 0.666 {
            Attack::Spore
        } else {
            Attack::Scream
        }
    }
    fn random(&mut self) -> f32 {
        let s = &mut self.saved.random;
        *s ^= *s << 13;
        *s ^= *s >> 17;
        *s ^= *s << 5;
        (*s >> 8) as f32 / 16777216.
    }
    fn q1_main(&self) -> &'static str {
        let wounded = self.saved.queen1 < 1100.;
        match (self.saved.attack, wounded) {
            (Attack::Pain, false) => ["pain_ready1", "pain_ready2", "pain_ready3"]
                [self.saved.attack_variant.min(2) as usize],
            (Attack::Pain, true) => ["pain_wounded1", "pain_wounded2", "pain_wounded3"]
                [self.saved.attack_variant.min(2) as usize],
            (Attack::Wound, _) => "pain_ready_2_wounded",
            (Attack::Popup, false) => "attack_normal_popup",
            (Attack::Popup, true) => "attack_wounded_popup",
            (Attack::Spit, false) => "attack_laser_spitfire",
            (Attack::Spit, true) => "attack_wounded_laser_spitfire",
            (Attack::Combo, false) => "attack_laser_combo",
            (Attack::Combo, true) => "attack_wounded_laser_combo",
            (Attack::Sweep, false) => "attack_laser_sweep",
            (Attack::Sweep, true) => "attack_wounded_laser_sweep",
            (Attack::Ice, false) => "attack_tel_iceblast2",
            (Attack::Ice, true) => "attack_wounded_tel_iceblast2",
            (Attack::SlamLeft, false) => "attack_tel_slam_left",
            (Attack::SlamLeft, true) => "attack_wounded_tel_slam_left",
            (Attack::SlamRight, false) => "attack_tel_slam_right",
            (Attack::SlamRight, true) => "attack_wounded_tel_slam_right",
            (Attack::Grab, false) => "attack_tel_squeeze",
            (Attack::Grab, true) => "attack_wounded_tel_squeeze",
            (_, false) => "ready_normal_base",
            (_, true) => "ready_wounded_base",
        }
    }
    fn q2_main(&self) -> &'static str {
        match self.saved.attack {
            Attack::Claw => "attack_claw",
            Attack::Club => "attack_club",
            Attack::Spore if self.saved.attack_variant == 1 => "attack_slit_spore_fire2",
            Attack::Spore => "attack_slit_spore_fire1",
            Attack::Scream if self.saved.attack_variant == 1 => "attack_slit_scream_fire2",
            Attack::Scream => "attack_slit_scream_fire1",
            _ => "idle",
        }
    }
    fn part_main(&self, n: usize) -> &'static str {
        if self.saved.parts[n] <= 25. {
            return "death";
        }
        match (n, self.saved.part_attack[n].unwrap_or(self.saved.attack)) {
            (0, Attack::Claw) | (3, Attack::Club) => "attack",
            (1, Attack::Centipede) => "attack_centipede",
            (1, Attack::JabberEye) => "attack_jabberwock_eye",
            (1, Attack::JabberSpit) => "attack_jabberwock_spit",
            (1, Attack::Hatter) => "attack_hatter",
            _ => "idle",
        }
    }
    pub(super) fn sequence(&self, name: &str) -> Vec<(&'static str, f32)> {
        let rig = &self.data.rigs[name];
        let a = name
            .strip_prefix("c_q2_t0")
            .and_then(|n| n.parse::<usize>().ok())
            .and_then(|n| self.saved.part_attack[n - 1])
            .unwrap_or(self.saved.attack);
        let clips = if name == "c_queen1" {
            let wounded = self.saved.queen1 < 1100.;
            match a {
                Attack::Spit | Attack::Combo | Attack::Sweep => vec![
                    if wounded {
                        "wounded_2_wounded_laser"
                    } else {
                        "ready_2_laser"
                    },
                    self.q1_main(),
                    if wounded {
                        "wounded_laser_2_wounded"
                    } else {
                        "laser_2_ready"
                    },
                ],
                Attack::Ice => vec![
                    if wounded {
                        "wounded_2_wounded_tel"
                    } else {
                        "ready_2_tel"
                    },
                    if wounded {
                        "attack_wounded_tel_iceblast1"
                    } else {
                        "attack_tel_iceblast1"
                    },
                    self.q1_main(),
                    if wounded {
                        "attack_wounded_tel_iceblast3"
                    } else {
                        "attack_tel_iceblast3"
                    },
                    if wounded {
                        "wounded_tel_2_wounded"
                    } else {
                        "tel_2_ready"
                    },
                ],
                Attack::SlamLeft | Attack::SlamRight | Attack::Grab => {
                    return vec![
                        (
                            if wounded {
                                "wounded_2_wounded_tel"
                            } else {
                                "ready_2_tel"
                            },
                            self.tele_prep(),
                        ),
                        (
                            if wounded {
                                "attack_wounded_tel_idle"
                            } else {
                                "attack_tel_idle"
                            },
                            1.666,
                        ),
                        (
                            self.q1_main(),
                            if a == Attack::Grab {
                                3.1
                            } else {
                                rig.duration(self.q1_main())
                            },
                        ),
                        (
                            if wounded {
                                "wounded_tel_2_wounded"
                            } else {
                                "tel_2_ready"
                            },
                            rig.duration(if wounded {
                                "wounded_tel_2_wounded"
                            } else {
                                "tel_2_ready"
                            }),
                        ),
                    ]
                }
                _ => vec![self.q1_main()],
            }
        } else if name == "c_q2_body" {
            match a {
                Attack::Spore => vec![
                    "attack_slit_spore_open",
                    self.q2_main(),
                    "attack_slit_spore_close",
                ],
                Attack::Scream => vec![
                    "attack_slit_scream_open",
                    self.q2_main(),
                    "attack_slit_scream_close",
                ],
                Attack::Claw | Attack::Club => {
                    return vec![("idle", 1.), (self.q2_main(), rig.duration(self.q2_main()))]
                }
                Attack::Centipede | Attack::JabberEye | Attack::JabberSpit | Attack::Hatter => {
                    return vec![("idle", 2. + rig.duration("idle"))]
                }
                _ => vec!["idle"],
            }
        } else if name == "c_q2_t02" && self.saved.parts[1] > 25. {
            match a {
                Attack::Centipede => vec![
                    "grow_centipede",
                    "ready_centipede",
                    self.part_main(1),
                    "return_centipede",
                ],
                Attack::JabberEye | Attack::JabberSpit => {
                    vec![
                        "grow_jabberwock",
                        "ready_jabberwock",
                        self.part_main(1),
                        "return_jabberwock",
                    ]
                }
                Attack::Hatter => vec![
                    "grow_hatter",
                    "ready_hatter",
                    self.part_main(1),
                    "return_hatter",
                ],
                _ => vec!["idle"],
            }
        } else {
            let n = name.trim_start_matches("c_q2_t0").parse::<usize>().unwrap() - 1;
            if matches!((n, a), (0, Attack::Claw) | (3, Attack::Club)) && self.saved.parts[n] > 25.
            {
                return vec![("ready", 1.), ("attack", rig.duration("attack"))];
            }
            vec![self.part_main(n)]
        };
        clips.into_iter().map(|c| (c, rig.duration(c))).collect()
    }
    pub(super) fn tele_prep(&self) -> f32 {
        self.data.rigs["c_queen1"].duration(if self.saved.queen1 < 1100. {
            "wounded_2_wounded_tel"
        } else {
            "ready_2_tel"
        })
    }
    pub(super) fn attack_animation(&self, name: &str) -> (&'static str, f32) {
        let mut t = name
            .strip_prefix("c_q2_t0")
            .and_then(|n| n.parse::<usize>().ok())
            .filter(|n| self.saved.part_attack[n - 1].is_some())
            .map_or(self.saved.attack_time, |n| self.saved.part_time[n - 1]);
        let seq = self.sequence(name);
        for (i, &(clip, d)) in seq.iter().enumerate() {
            if t < d || i == seq.len() - 1 {
                return (clip, t);
            }
            t -= d;
        }
        unreachable!()
    }
    pub(super) fn part_animation(&self, n: usize) -> (&'static str, f32) {
        if self.saved.parts[n] <= 25. {
            ("death", self.saved.part_death[n])
        } else {
            self.attack_animation(&format!("c_q2_t0{}", n + 1))
        }
    }
    pub(super) fn battle_targets(&self) -> Vec<Target> {
        if self.saved.phase == Phase::Queen1 && self.saved.queen1 > 0. {
            return vec![Target {
                id: BASE,
                center: self.queen1_pose().translation + Vec3::Z * 12.,
                half: vec3(40., 40., 60.),
            }];
        }
        if self.saved.phase != Phase::Queen2 {
            return Vec::new();
        }
        let mut targets = vec![Target {
            id: BASE + 1,
            center: self.saved.body.translation + Vec3::Z * 700.,
            half: vec3(48., 48., 700.),
        }];
        for n in 0..4 {
            if self.saved.parts[n] > 0. {
                let p = self.part_pose(n);
                let rig = &self.data.rigs[&format!("c_q2_t0{}", n + 1)];
                let tags = match n {
                    0 => ["bone_t01_02", "bone_t01_03", "bone_t01_04", "tag_claw"],
                    1 => ["bone_t02_02", "bone_t02_03", "b_t02_04", "tag_jab_mouth"],
                    2 => ["bone_t03_02", "bone_t03_03", "bone_t03_04", "tag_tip"],
                    _ => ["bone_t04_02", "bone_t04_03", "bone_t04_05", "Bone30"],
                };
                let (clip, t) = self.part_animation(n);
                for (i, tag) in tags.iter().enumerate() {
                    let watch = if n == 1 {
                        self.saved.head_watch.clone()
                    } else {
                        Default::default()
                    };
                    let tip = rig.tag_watched(tag, clip, t, p, true, &watch).translation;
                    targets.push(Target {
                        id: BASE + 2 + n,
                        center: tip,
                        half: if n < 2 && i == 3 {
                            vec3(80., 80., 64.)
                        } else {
                            Vec3::splat(64.)
                        },
                    });
                }
            }
        }
        targets
    }
    pub(super) fn battle_hit(&mut self, h: Hit) -> Option<&'static str> {
        if !h.damage.is_finite() || h.damage <= 0. {
            return None;
        }
        if h.id == BASE && self.saved.phase == Phase::Queen1 {
            let before = self.saved.queen1;
            self.saved.queen1 = (self.saved.queen1 - h.damage).max(0.);
            if before >= 1100. && self.saved.queen1 < 1100. && self.saved.queen1 > 0. {
                self.saved.attack = Attack::Wound;
                self.saved.attack_variant = 0;
                self.saved.attack_time = 0.;
                self.saved.grabbed = false;
            } else if self.saved.attack == Attack::Idle && self.random() < 0.15 {
                self.saved.attack = Attack::Pain;
                self.saved.attack_variant = (self.random() * 3.) as u8;
                self.saved.attack_time = 0.;
            }
        } else if self.saved.phase == Phase::Queen2 {
            if h.id == BASE + 1 {
                self.saved.queen2 = (self.saved.queen2 - h.damage).max(0.);
            } else if (BASE + 2..BASE + 6).contains(&h.id) {
                if h.kind == DamageKind::Blunderbuss {
                    return None;
                }
                let hp = &mut self.saved.parts[h.id - BASE - 2];
                let before = *hp;
                if before <= 0. {
                    return None;
                }
                *hp = (*hp - h.damage).max(0.);
                // The authored START_DEATH state collapses a tentacle below
                // 1030 and leaves 25 health for the final dismembering hit.
                if before >= 1030. && *hp < 1030. && *hp > 0. {
                    *hp = 25.;
                    self.saved.part_death[h.id - BASE - 2] = 0.;
                    self.saved.part_attack[h.id - BASE - 2] = None;
                    return Some("sound/character/queen/t_death.wav");
                }
                if *hp == 0. {
                    let n = h.id - BASE - 2;
                    self.saved.part_attack[n] = None;
                    self.gibs(self.part_pose(n).translation, 5, 3., false);
                }
            } else {
                return None;
            }
        } else {
            return None;
        }
        Some("sound/character/queen/pain_ready1.wav")
    }
    fn move_body(&mut self, dt: f32) {
        let s = &mut self.saved;
        if s.motion != 0 {
            s.motion_time = (s.motion_time + dt).min(100.);
        }
        if s.motion == 1 {
            // Source DownBitch ramps to 70 units / 0.05 s for its first 50 steps.
            let speed = 1400. * (s.motion_time / 2.5).clamp(0.02, 1.);
            s.body.translation.z = (s.body.translation.z - speed * dt).max(-2960.);
            if s.body.translation.z <= -2960. {
                s.position = if s.middle > 0. || s.platform < 0 {
                    5
                } else {
                    s.platform as u8
                };
                s.body = self.data.points[&format!("point{}", s.position)];
                s.body.translation.z = -2960.;
                s.motion = 2;
                s.motion_time = 0.;
            }
        } else if s.motion == 2 {
            let speed = if s.body.translation.z < -472. {
                900.
            } else {
                900. * ((90. - s.body.translation.z) / 512.)
            };
            s.body.translation.z = (s.body.translation.z + speed * dt).min(40.);
            if s.body.translation.z >= 40. {
                s.motion = 0;
                s.motion_time = 0.;
            }
        } else if s.position == 5 && s.middle > 0. {
            s.middle = (s.middle - dt).max(0.);
        } else if s.platform > 0 && s.position != s.platform as u8 {
            s.motion = 1;
            s.motion_time = 0.;
        }
    }
    pub(super) fn battle(&mut self, c: &mut Combat<'_>) -> Feedback {
        let mut f = Feedback::default();
        let dt = c.dt.min(0.1);
        if dt <= 0. || !c.stats.alive() {
            return f;
        }
        if self.saved.phase == Phase::Queen1 && self.saved.queen1 <= 0. {
            self.start(Phase::Birth);
            return f;
        }
        if self.saved.phase == Phase::Queen2 && self.saved.queen2 < 1000. {
            self.saved.body.translation.z = 40.;
            self.saved.motion = 0;
            self.start(Phase::Death);
            return f;
        }
        if !matches!(self.saved.phase, Phase::Queen1 | Phase::Queen2) {
            return f;
        }
        for n in 0..4 {
            if self.saved.parts[n] <= 25. {
                self.saved.part_death[n] = (self.saved.part_death[n] + dt).min(100.);
            }
        }
        let q1 = self.saved.phase == Phase::Queen1;
        if !q1 {
            self.move_body(dt);
            if self.saved.time < 5. {
                return f;
            }
        }
        let bodies = crate::combat::Opponents {
            summon: c.summon,
            demon: false,
        }
        .bodies(c.player.eye());
        let target = if c.notarget {
            c.summon
        } else {
            Some(Target {
                id: crate::dice::ALICE,
                center: c.player.feet + Vec3::Z * 32.,
                half: vec3(16., 16., 32.),
            })
        };
        self.projectiles(dt, &bodies, c.world, &mut f);
        let Some(target) = target else {
            return f;
        };
        if !q1 {
            let pose = self.part_pose(1);
            let (clip, time) = self.part_animation(1);
            let head = self.data.rigs["c_q2_t02"]
                .tag("bip01 head", clip, time, pose, false)
                .translation;
            self.saved.head_watch.update(
                dt,
                (self.saved.parts[1] > 25.)
                    .then_some(pose.rotation.conjugate() * (target.center - head)),
            );
        }
        let mut before = self.saved.attack_time;
        self.saved.attack_time += dt;
        if self.saved.attack == Attack::Idle {
            let delay = if q1 {
                if self.saved.queen1 < 1100. {
                    1.5
                } else {
                    2.5
                }
            } else {
                self.saved.attack_delay
            };
            if self.saved.attack_time >= delay && (q1 || self.saved.motion == 0) {
                let eye = if q1 {
                    self.queen1_pose().translation
                } else {
                    self.saved.body.translation + Vec3::Z * 1000.
                };
                let visible = clear(c.world, eye, target.center);
                self.saved.attack = self.choose_attack(q1, eye.distance(target.center), visible);
                // The unsuffixed AI animation aliases select both authored variants.
                self.saved.attack_variant =
                    if !q1 && matches!(self.saved.attack, Attack::Spore | Attack::Scream) {
                        (self.random() >= 0.5) as u8
                    } else {
                        0
                    };
                self.saved.attack_time = 0.;
                self.saved.fired = 0;
                self.saved.grab_time = 0.;
                self.saved.grab_arrived = false;
                self.saved.attack_target = target.center;
                if self.saved.attack == Attack::Popup {
                    let high = target.center + Vec3::Z * 10.;
                    let low = high - Vec3::Z * 250.;
                    let floor = c.world.sweep(high, low, Vec3::ZERO);
                    if !floor.start_solid && floor.fraction < 1. {
                        self.saved.attack_target = high.lerp(low, floor.fraction) + Vec3::Z * 32.;
                    }
                }
                if let Some(n) = Self::attack_part(self.saved.attack) {
                    if self.saved.part_attack[n].is_none() {
                        self.saved.part_attack[n] = Some(self.saved.attack);
                        self.saved.part_time[n] = 0.;
                        self.saved.part_fired[n] = 0;
                    }
                }
                before = 0.;
            }
        }
        let t = self.saved.attack_time;
        if self.saved.attack == Attack::Popup && before < 1. && t >= 1. {
            let high = target.center + Vec3::Z * 10.;
            let low = high - Vec3::Z * 250.;
            let floor = c.world.sweep(high, low, Vec3::ZERO);
            self.saved.attack_target = high.lerp(low, floor.fraction) + Vec3::Z * 32.;
            if floor.start_solid
                || floor.fraction == 1.
                || self
                    .queen1_pose()
                    .translation
                    .distance(self.saved.attack_target)
                    < 300.
            {
                self.saved.fired |= 8;
            }
        }
        let tele = matches!(
            self.saved.attack,
            Attack::SlamLeft | Attack::SlamRight | Attack::Grab
        );
        let capture = self.tele_prep() + 1.666;
        if tele {
            if before < self.tele_prep() && t >= self.tele_prep() {
                self.saved.attack_target = target.center;
            }
            if before < capture
                && t >= capture
                && target.center.distance(self.saved.attack_target) <= 96.
                && clear(c.world, self.queen1_pose().translation, target.center)
            {
                if self.saved.attack == Attack::Grab && target.id == crate::dice::ALICE {
                    self.saved.grabbed = true;
                } else {
                    self.saved.fired |= 1;
                }
            }
            if self.saved.grabbed {
                let (clip, local) = self.attack_animation("c_queen1");
                let at = self.data.rigs["c_queen1"]
                    .tag("tag_squeeze", clip, local, self.queen1_pose(), false)
                    .translation;
                let next = c.player.feet.lerp(at, (dt * 4.).min(1.));
                let trace = c.world.body_trace(c.player.feet, next);
                if !trace.start_solid {
                    c.player.feet = c.player.feet.lerp(next, trace.fraction);
                    c.player.velocity = Vec3::ZERO;
                }
                if (c.player.feet - at).length() < 64. {
                    self.saved.grab_arrived = true;
                }
                if self.saved.grab_arrived {
                    self.saved.grab_time += dt;
                }
                // Native squeeze pulses on whole world seconds, for three seconds
                // after arrival. Crossing tests survive different update rates.
                if self.saved.grab_arrived
                    && self.saved.clock.floor() > (self.saved.clock - dt).floor()
                {
                    f.damage +=
                        ((c.stats.sanity() - 1.) / c.stats.difficulty.incoming()).clamp(0., 10.);
                }
            }
            if matches!(self.saved.attack, Attack::SlamLeft | Attack::SlamRight)
                && self.saved.fired & 1 != 0
            {
                if before < capture + 0.5 && t >= capture + 0.5 {
                    let right = self.queen1_pose().rotation * Vec3::Y;
                    let dir = if self.saved.attack == Attack::SlamLeft {
                        -right
                    } else {
                        right
                    };
                    f.strike(target.id, 0., dir * 1200., DamageKind::Other);
                    self.saved.fired |= 2;
                } else if self.saved.fired & 2 != 0
                    && self.saved.fired & 4 == 0
                    && (target.id != crate::dice::ALICE || c.player.velocity.length() < 800.)
                {
                    f.strike(target.id, 20., Vec3::ZERO, DamageKind::Other);
                    self.saved.fired |= 4;
                }
            }
        }
        if self.saved.attack == Attack::Popup
            && self.saved.fired & 8 == 0
            && before < 2.25
            && t >= 2.25
        {
            let at = self.saved.attack_target - Vec3::Z * 22.;
            let ctx = combat::Context {
                world: c.world,
                targets: &bodies,
            };
            if let Some((id, _)) =
                combat::contact_box(&ctx, at, at + Vec3::Z * 250., vec3(10., 10., 0.))
            {
                f.strike(id, 30., Vec3::Z * 400., DamageKind::Other);
            }
        }
        let mut actors = Vec::new();
        if q1 {
            actors.push(("c_queen1".to_owned(), self.queen1_pose()));
        } else {
            actors.push(("c_q2_body".into(), self.saved.body));
            for n in 0..4 {
                if self.saved.parts[n] > 25. {
                    actors.push((format!("c_q2_t0{}", n + 1), self.part_pose(n)));
                }
            }
        }
        let mut duration: f32 = 0.;
        for (name, pose) in actors {
            let part = name
                .strip_prefix("c_q2_t0")
                .and_then(|n| n.parse::<usize>().ok())
                .map(|n| n - 1);
            let (from, to) = if let Some(n) = part {
                if self.saved.part_attack[n].is_none() {
                    continue;
                }
                let from = self.saved.part_time[n];
                self.saved.part_time[n] += dt;
                (from, self.saved.part_time[n])
            } else {
                (before, t)
            };
            let seq = self.sequence(&name);
            let actor_duration: f32 = seq.iter().map(|(_, d)| d).sum();
            if part.is_none() {
                duration = actor_duration;
            }
            let rig = &self.data.rigs[&name];
            let mut offset = 0.;
            for (clip, d) in seq {
                let start = offset;
                offset += d;
                if to > start && from < offset {
                    let animation = &rig.clips[clip];
                    for command in rig.events.between(
                        clip,
                        crate::animation_events::Span {
                            start: (from - start).max(0.),
                            end: (to - start).min(d),
                            duration: animation.duration(),
                            frame_time: animation.frame_time,
                            looping: false,
                            entered: from <= start,
                        },
                    ) {
                        if let crate::animation_events::Command::Sound { path, .. } = command {
                            f.spatial_sounds.push((rig.sounds[path], pose.translation));
                        }
                    }
                }
                let Some(events) = rig.attacks.get(clip) else {
                    continue;
                };
                for (at, row) in events
                    .iter()
                    .filter(|(at, _)| *at + start >= from && *at + start < to)
                {
                    match row[0].as_str() {
                        "proj" | "q2_proj" => {
                            let watch = if name == "c_q2_t02" {
                                self.saved.head_watch.clone()
                            } else {
                                Default::default()
                            };
                            let source = rig
                                .tag_watched(&row[1], clip, *at, pose, false, &watch)
                                .translation;
                            let spec = &self.data.projectiles[&row[2]];
                            self.saved.projectiles.push(Shot {
                                model: row[2].trim_end_matches(".tik").into(),
                                at: source,
                                velocity: (target.center - source).normalize_or_zero() * spec.speed,
                                age: 0.,
                                damage: spec.damage,
                                force: spec.force,
                                life: spec.life,
                                end: None,
                                seek_at: projectile::first_seek(),
                                victim: Some(target.id),
                                trail: Vec::new(),
                            });
                        }
                        "beamattack" => {
                            let watch = if name == "c_q2_t02" {
                                self.saved.head_watch.clone()
                            } else {
                                Default::default()
                            };
                            let source = rig.tag_watched(&row[1], clip, *at, pose, false, &watch);
                            let distance = row
                                .get(5)
                                .and_then(|s| s.parse::<f32>().ok())
                                .unwrap_or(2000.);
                            let end = beam_end(source, target.center, distance, row[4] != "0");
                            let ctx = combat::Context {
                                world: c.world,
                                targets: &bodies,
                            };
                            let contact = combat::contact(&ctx, source.translation, end, 5.);
                            if let Some((id, _)) = contact {
                                let damage = row[2].parse().unwrap_or(1.);
                                f.strike(id, damage, Vec3::ZERO, DamageKind::Electric);
                            }
                            let trace = c.world.sweep(source.translation, end, Vec3::ZERO);
                            let end = source
                                .translation
                                .lerp(end, contact.map_or(trace.fraction, |(_, f)| f));
                            if contact.is_some() || trace.fraction < 1. {
                                self.saved.impacts.push(projectile::Impact {
                                    model: "fx_lightning_hit".into(),
                                    at: end,
                                    normal: trace.normal,
                                    age: 0.,
                                    mark: None,
                                    mark_radius: 0.,
                                });
                            }
                            self.saved.projectiles.push(Shot {
                                model: row.get(6).cloned().unwrap_or_else(|| "queenbeam2".into()),
                                at: source.translation,
                                velocity: Vec3::ZERO,
                                age: 0.,
                                damage: 0.,
                                force: 0.,
                                life: row.get(8).and_then(|v| v.parse().ok()).unwrap_or(0.125),
                                end: Some(end),
                                seek_at: projectile::first_seek(),
                                victim: None,
                                trail: Vec::new(),
                            });
                        }
                        "melee" => {
                            // A continuous contact window may strike each opponent only once.
                            let mask = part.map_or(self.saved.fired, |n| self.saved.part_fired[n]);
                            if mask & 3 != 3 {
                                let source = rig.tag(&row[2], clip, *at, pose, false).translation;
                                let previous = rig
                                    .tag(
                                        &row[2],
                                        clip,
                                        (*at - rig.clips[clip].frame_time).max(0.),
                                        pose,
                                        false,
                                    )
                                    .translation;
                                let half = row
                                    .get(4)
                                    .and_then(|v| crate::interaction::vector(v))
                                    .unwrap_or(vec3(128., 150., 128.))
                                    * 0.5;
                                for body in &bodies {
                                    let bit = if body.id == crate::dice::ALICE { 1 } else { 2 };
                                    if mask & bit != 0 {
                                        continue;
                                    }
                                    if combat::segment_box(
                                        previous,
                                        source,
                                        body.center,
                                        body.half + half,
                                    )
                                    .is_some()
                                        && clear(c.world, source, body.center)
                                    {
                                        f.strike(
                                            body.id,
                                            row[1].parse().unwrap_or(25.),
                                            (body.center - source).normalize_or_zero() * 250.,
                                            DamageKind::Other,
                                        );
                                        if let Some(n) = part {
                                            self.saved.part_fired[n] |= bit;
                                        } else {
                                            self.saved.fired |= bit;
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            if let Some(n) = part {
                if to >= actor_duration {
                    self.saved.part_attack[n] = None;
                    self.saved.part_time[n] = 0.;
                    self.saved.part_fired[n] = 0;
                }
            }
        }
        if self.saved.attack == Attack::Popup {
            duration = duration.max(2.25 + self.data.popup_duration);
        }
        let finished = t >= duration
            && (!self.saved.grabbed || self.saved.grab_time >= 3. || t >= duration + 5.);
        if self.saved.attack != Attack::Idle && finished {
            if self.saved.grabbed {
                c.player.knockback(
                    (c.player.feet - self.queen1_pose().translation).normalize_or_zero() * 800.,
                );
            }
            self.saved.grabbed = false;
            self.saved.attack = Attack::Idle;
            self.saved.attack_variant = 0;
            self.saved.attack_delay = 0.75 + self.random() * 1.75;
            self.saved.attack_time = 0.;
            self.saved.fired = 0;
        }
        if self.saved.grabbed {
            f.damage = f
                .damage
                .min(((c.stats.sanity() - 1.) / c.stats.difficulty.incoming()).max(0.));
        }
        f
    }
}

fn clear(w: &World, a: Vec3, b: Vec3) -> bool {
    let t = w.sweep(a, b, Vec3::ZERO);
    !t.start_solid && t.fraction == 1.
}

//! Greenhouse and final laboratory presentation, using reviewed local asset cues.
use super::*;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct State {
    mixing: Option<f32>,
    pub presentation: Option<f32>,
    pub departure: Option<f32>,
}
impl State {
    pub fn validate(&self, time: f32) -> Result<()> {
        ensure!(
            [self.mixing, self.presentation, self.departure]
                .iter()
                .all(|v| v.is_none_or(|t| t.is_finite() && (0. ..=time).contains(&t)))
                && self
                    .presentation
                    .is_none_or(|p| self.mixing.is_some_and(|m| m <= p))
                && self
                    .departure
                    .is_none_or(|d| self.presentation.is_some_and(|p| p <= d)),
            "Invalid potion presentation order"
        );
        Ok(())
    }
}

pub(super) fn advance(s: &mut super::State, data: &Data) {
    if s.beat == Beat::Growth {
        if s.time >= 0.5 {
            s.cue(0, 0);
        }
        if s.time >= 4.9 {
            s.cue(1, 2);
        }
    } else if s.beat == Beat::Final {
        let p = s.potion.as_mut().unwrap();
        if p.mixing
            .is_some_and(|at| s.time - at >= data.lengths["mixing"] + 2.5)
        {
            p.presentation.get_or_insert(s.time);
        }
        if s.dialogue_done && p.presentation.is_some_and(|at| s.time - at >= 5.) {
            p.departure.get_or_insert(s.time);
        }
        let mixing = p.mixing.is_some();
        let vanish = p
            .departure
            .is_some_and(|at| s.time - at >= data.lengths["vanish01"]);
        if mixing {
            s.cue(0, 3);
        }
        if vanish {
            s.cue(2, 0);
        }
    }
}

pub(super) fn fade(s: &super::State, data: &Data) -> f32 {
    let Some(p) = &s.potion else {
        return 0.;
    };
    if let Some(at) = p.presentation {
        return (1. - (s.time - at) / 0.5).clamp(0., 1.);
    }
    p.mixing.map_or(0., |at| {
        ((s.time - at - data.lengths["mixing"] - 2.) / 0.5).clamp(0., 1.)
    })
}

impl School2 {
    pub(crate) fn potion_sound(&self, loops: &mut Vec<crate::audio::LoopCue>) {
        if self.cinema.as_ref().is_some_and(|s| {
            s.active
                && s.beat == Beat::Final
                && s.potion.as_ref().is_some_and(|p| p.mixing.is_some())
        }) {
            loops.push(crate::audio::LoopCue {
                id: 1002,
                path: "sound/world/machine/condenser.wav",
                origin: self.items["gnome_condenser"],
                clock: Some(self.condenser_time()),
            });
        }
    }

    pub(super) fn prepare_potion_story(&self, story: &mut Story) -> bool {
        let s = self.cinema.as_ref().unwrap();
        story.line_limit = if s.potion.as_ref().unwrap().presentation.is_some() {
            None
        } else {
            Some(0)
        };
        s.time >= 0.5
    }
    pub(super) fn sync_potion_story(&mut self, story: &Story) {
        let Some(s) = self
            .cinema
            .as_mut()
            .filter(|s| s.active && s.beat == Beat::Final)
        else {
            return;
        };
        if story.progress(FINAL).is_some_and(|(line, _)| line == 0) && story.line_finished() {
            s.potion.as_mut().unwrap().mixing.get_or_insert(s.time);
        }
    }
    pub(crate) fn condenser_time(&self) -> f32 {
        if let Some(s) = &self.cinema {
            if s.beat == Beat::Final {
                return if !s.active {
                    self.cinema_data.lengths["mixing"] + 2.5
                } else {
                    s.potion
                        .as_ref()
                        .and_then(|p| p.mixing)
                        .map_or(0., |at| s.time - at)
                };
            }
        }
        if self.quest.stage == Stage::Mixing {
            self.quest.time
        } else {
            0.
        }
    }
}

pub(super) struct Art {
    beaker: Prop,
    vapor: crate::particles::Attached,
    lollipop: Prop,
    potion: Prop,
    star: Prop,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, crate::texture::MaterialSpec>,
    ) -> Result<Self> {
        Ok(Self {
            beaker: Prop::load(assets, "beaker01", specs)?,
            vapor: crate::particles::Attached::load(assets, "beaker01", specs)?
                .context("Jumbogrow emitter missing")?,
            lollipop: Prop::load(assets, "lollypop", specs)?,
            potion: Prop::load(assets, "beaker02", specs)?,
            star: Prop::load(assets, "star", specs)?,
        })
    }
}
impl super::Art {
    pub(super) fn draw_potion(
        &mut self,
        school: &School2,
        bright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) -> bool {
        let s = school.cinema.as_ref().unwrap();
        // Final completion removes the Gnome; subsequent reward pickups cannot respawn him.
        if !s.active {
            return s.beat == Beat::Final;
        }
        let d = &school.cinema_data;
        for puppet in [&mut self.alice, &mut self.gnome] {
            puppet.atmosphere(atmosphere, camera);
            puppet.watch(Default::default());
        }
        if s.beat == Beat::Growth {
            let alice = d.points["alice_pop1"];
            self.alice
                .watch(watch(alice, school.items["ig_lollypop"] + Vec3::Z * 50.));
            self.alice
                .draw("idle_stand", s.time, true, alice, 1., bright);
            self.gnome
                .draw("idle", s.time, true, d.points["gnome_pos1"], 1., bright);
            let scale = if s.time < 4. {
                0.1
            } else {
                (0.07 * (1. + ((s.time - 4.) / 0.1).floor())).min(3.78)
            };
            if s.time >= 0.5 {
                self.potion.lollipop.draw(
                    Transform {
                        translation: school.items["ig_lollypop"],
                        rotation: Quat::IDENTITY,
                    },
                    scale,
                    bright,
                );
            }
            let pose = beaker_pose(school.items["lolly_jg_beaker"], s.time);
            let alpha = ((s.time - 0.5) / 2.).clamp(0., 1.) * ((6. - s.time) / 2.).clamp(0., 1.);
            draw_faded(&mut self.potion.beaker, pose, 1., bright, s.time, alpha);
            let beaker = &self.potion.beaker;
            crate::render::depth_read_only(|| {
                self.potion.vapor.draw(
                    s.time - 0.5,
                    1.,
                    |birth, tag| {
                        let p = beaker_pose(school.items["lolly_jg_beaker"], birth + 0.5);
                        Transform {
                            translation: tag.map_or(p.translation, |tag| beaker.point(p, tag, 1.)),
                            ..p
                        }
                    },
                    |_, birth, enabled| enabled && birth < 5.5,
                    camera,
                    atmosphere,
                )
            });
            return true;
        }
        let p = s.potion.as_ref().unwrap();
        let alice = d.points["alice_last_pos1"];
        let gnome = d.points[if p.presentation.is_some() {
            "gnome_star_pos1"
        } else {
            "gnome_pos1"
        }];
        self.alice
            .watch(watch(alice, gnome.translation + Vec3::Z * 48.));
        self.alice
            .draw("idle_stand", s.time, true, alice, 1., bright);
        let (clip, time, looping, scale) = if let Some(at) = p.departure {
            let age = s.time - at;
            let shrink = age - d.lengths["vanish01"];
            (
                if shrink < 0. { "vanish01" } else { "vanish02" },
                if shrink < 0. { age } else { shrink },
                false,
                (1. - (shrink.max(0.) / 0.1).floor() * 0.04).clamp(0., 1.),
            )
        } else if let Some(at) = p.presentation {
            let age = s.time - at;
            if (6. ..6. + d.lengths["talk"]).contains(&age) {
                ("talk", age - 6., false, 1.)
            } else {
                ("idle", age, true, 1.)
            }
        } else if let Some(at) = p.mixing {
            ("mixing", s.time - at, false, 1.)
        } else {
            ("idle", s.time, true, 1.)
        };
        // The mixing clip supplies the ingredient handling. Alice_Last_Anims is empty.
        self.gnome.draw(clip, time, looping, gnome, scale, bright);
        if let Some(at) = p.presentation {
            let age = s.time - at;
            let scale = (0.01 + 0.02 * (age / 0.1).floor()).min(1.);
            draw_faded(
                &mut self.potion.potion,
                Transform {
                    translation: school.items["shrink_potion"],
                    rotation: Quat::IDENTITY,
                },
                scale,
                bright,
                age,
                (age / 2.).min(1.),
            );
            if p.departure.is_some() {
                self.potion.star.draw(
                    Transform {
                        translation: school.items["lucky_star"],
                        rotation: Quat::from_rotation_z(school.age),
                    },
                    1.,
                    bright,
                );
            } else {
                let tag = if looping {
                    self.gnome.looping_tag("tag_pipe", clip, time, gnome, 1.)
                } else {
                    self.gnome.tag("tag_pipe", clip, time, gnome, 1.)
                };
                if let Some(tag) = tag {
                    self.potion.star.draw(tag, 1., bright);
                }
            }
        }
        if let Some(at) = p.departure {
            let age = s.time - at;
            if age < 4. {
                let pose = d.points["gnome_fire3"];
                crate::render::depth_read_only(|| {
                    self.fire
                        .draw(age, 1., |_, _| pose, |_, _, v| v, camera, atmosphere)
                });
            }
        }
        true
    }
}
fn draw_faded(prop: &mut Prop, pose: Transform, scale: f32, bright: bool, _age: f32, alpha: f32) {
    if alpha <= 0. {
        return;
    }
    for mesh in prop.meshes_at(pose, scale, bright, 0., false) {
        if alpha >= 1. {
            crate::render_fx::skin(mesh);
        } else {
            for v in &mut mesh.vertices {
                v.color[3] = (alpha * 255.) as u8;
            }
            crate::render_fx::effect(mesh, crate::materials::Blend::Alpha);
        }
    }
}

fn beaker_pose(origin: Vec3, time: f32) -> Transform {
    let move_t = ((time - 1.5) / 4.).clamp(0., 1.);
    // Native Euler component 2 is roll; north/up translate in world space.
    Transform {
        translation: origin + vec3(0., 8., 64.) * move_t,
        rotation: Quat::from_rotation_x(135_f32.to_radians() * move_t),
    }
}

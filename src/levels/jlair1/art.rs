use super::*;
use crate::npc::Puppet;
pub(super) struct Art {
    alice: Puppet,
    gryphon: Puppet,
    cat: Puppet,
    caterpillar: Puppet,
    gestures: [f32; 2],
    essence: crate::loot_art::Art,
    cater_gestures: Vec<(&'static str, f32)>,
    cat_gestures: Vec<(&'static str, f32)>,
}
fn load_gestures(
    a: &mut Assets,
    model: &str,
    keys: &[&'static str],
) -> Result<Vec<(&'static str, f32)>> {
    let def = crate::skeletal::Definition::load(a, &format!("models/{model}.tik"))?;
    let sk = crate::skeletal::Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
    keys.iter()
        .map(|key| {
            Ok((
                *key,
                if *key == "idle_base" {
                    4.
                } else {
                    crate::skeletal::Animation::parse_tags(
                        &a.read(&format!("{}/{}", def.path, def.animations[*key]))?,
                        &sk.unskinned_tags(),
                    )?
                    .0
                    .duration()
                },
            ))
        })
        .collect()
}
fn gesture<'a>(sequence: &[(&'a str, f32)], mut time: f32, idle: &'a str) -> (&'a str, f32, bool) {
    for (clip, length) in sequence {
        if time < *length {
            return (clip, time.max(0.), false);
        }
        time -= length;
    }
    (idle, time, true)
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(a)?;
        let def = crate::skeletal::Definition::load(a, "models/c_gryphon.tik")?;
        let sk =
            crate::skeletal::Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
        let mut gestures = [0.; 2];
        for (i, key) in ["talk_paw_circle", "talk_paw"].iter().enumerate() {
            gestures[i] = crate::skeletal::Animation::parse_tags(
                &a.read(&format!("{}/{}", def.path, def.animations[*key]))?,
                &sk.unskinned_tags(),
            )?
            .0
            .duration();
        }
        Ok(Self {
            gestures,
            cater_gestures: load_gestures(
                a,
                "c_caterpillar",
                &[
                    "talk04",
                    "talk05",
                    "talk06",
                    "talk07",
                    "talk01",
                    "talk03",
                    "idle_base",
                    "talk04",
                    "talk06",
                    "talk02",
                    "talk07",
                ],
            )?,
            cat_gestures: load_gestures(
                a,
                "c_cheshire",
                &["sit_idle2", "sit_talk3", "sit_talk1", "sit_idle2"],
            )?,
            essence: crate::loot_art::Art::load(a, &specs)?,
            alice: Puppet::load(
                a,
                "alice",
                &["riding", "ready", "talk_02", "idle_shrug"],
                &specs,
            )?,
            gryphon: Puppet::load(
                a,
                "c_gryphon",
                &[
                    "fly",
                    "fly_takeoff",
                    "talk_paw_circle",
                    "talk_paw",
                    "idle_look_left",
                    "idle",
                ],
                &specs,
            )?,
            cat: Puppet::load(
                a,
                "c_cheshire",
                &["sit_idle2", "sit_talk3", "sit_talk1"],
                &specs,
            )?,
            caterpillar: Puppet::load(
                a,
                "c_caterpillar",
                &[
                    "idle_base",
                    "talk01",
                    "talk02",
                    "talk03",
                    "talk04",
                    "talk05",
                    "talk06",
                    "talk07",
                ],
                &specs,
            )?,
        })
    }
    pub fn rider(&self, o: &Curiosity, time: f32) -> Result<Transform> {
        self.gryphon
            .looping_tag(
                "tag_alice",
                "fly",
                time,
                o.paths["gryphon_path1"].sample(time, true),
                1.,
            )
            .context("Gryphon riding tag missing")
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.alice.mouth(s.mouth(&["fakeplayer", "alice"]));
        self.gryphon.mouth(s.mouth(&["gryphon_actor1"]));
        self.cat.mouth(s.mouth(&["cat_actor1"]));
        self.caterpillar.mouth(s.mouth(&["caterpillar_actor1"]));
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atm: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let o = l.downcast_ref::<Curiosity>().unwrap();
        if o.saved.essence_live {
            for (i, at) in o.essence.iter().enumerate() {
                if !o.saved.essence_taken[i] {
                    self.essence.draw(crate::loot::Grade::Small, *at, atm, camera, crate::render_fx::time());
                }
            }
        }
        let Some(s) = &o.saved.scene else {
            return;
        };
        for p in [
            &mut self.alice,
            &mut self.gryphon,
            &mut self.cat,
            &mut self.caterpillar,
        ] {
            p.atmosphere(atm, camera);
        }
        match s.kind {
            Kind::Arrival => {
                if s.time < 6.5 {
                    self.gryphon.draw(
                        "fly",
                        s.time,
                        true,
                        o.paths["gryphon_path1"].sample(s.time, true),
                        1.,
                        bright,
                    );
                    if let Ok(at) = self.rider(o, s.time) {
                        self.alice.draw("riding", s.time, true, at, 1., bright);
                    }
                } else {
                    self.alice.draw(
                        "ready",
                        s.time - 6.5,
                        true,
                        o.point("alice_pos1"),
                        1.,
                        bright,
                    );
                    if let Some(t) = s.ending {
                        if t < 0.5 {
                            self.gryphon.draw(
                                "fly_takeoff",
                                t,
                                false,
                                o.point("gryphon_posx1"),
                                1.,
                                bright,
                            );
                        } else {
                            self.gryphon.draw(
                                "fly",
                                t - 0.5,
                                true,
                                o.paths["gryphon_leave1"].sample(t - 0.5, true),
                                1.,
                                bright,
                            );
                        }
                    } else {
                        let t = (s.time - 7.).max(0.);
                        let (clip, at) = if t < self.gestures[0] {
                            ("talk_paw_circle", t)
                        } else if t < self.gestures[0] + self.gestures[1] {
                            ("talk_paw", t - self.gestures[0])
                        } else {
                            ("idle_look_left", t - self.gestures[0] - self.gestures[1])
                        };
                        self.gryphon
                            .draw(clip, at, false, o.point("gryphon_posx1"), 1., bright);
                    }
                }
            }
            Kind::Caterpillar => {
                let (clip, time) = if s.line == 7 && s.line_time >= 4.5 {
                    ("talk_02", s.line_time - 4.5)
                } else if s.line == 13 || s.line == 8 {
                    ("idle_shrug", s.line_time)
                } else {
                    ("ready", s.time)
                };
                self.alice.draw(
                    clip,
                    time,
                    clip == "ready",
                    o.point("alice_cater_pos1"),
                    1.,
                    bright,
                );
                if s.line >= 1 {
                    let (clip, time, loops) = if s.line >= 2 {
                        gesture(&self.cat_gestures, s.time - s.starts[2], "sit_idle2")
                    } else {
                        ("sit_idle2", s.time, true)
                    };
                    self.cat.draw_dissolving(
                        clip,
                        time,
                        loops,
                        o.point("cat_cater_pos1"),
                        1.,
                        bright,
                        1. - ((s.time - s.starts[1]) / 2.).clamp(0., 1.),
                    );
                }
                if s.reveal.is_some() {
                    let (clip, time, loops) = if s.line >= 4 {
                        gesture(&self.cater_gestures, s.time - s.starts[4], "idle_base")
                    } else {
                        ("idle_base", s.time, true)
                    };
                    self.caterpillar.draw(
                        clip,
                        time,
                        loops,
                        o.point("caterpillar_actor1"),
                        1.,
                        bright,
                    );
                }
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

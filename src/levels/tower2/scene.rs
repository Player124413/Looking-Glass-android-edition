use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    level::{
        scene::{SceneRunner, SceneState},
        spec::{EndSpec, SceneSpec, ShotSpec},
    },
    npc::Puppet,
    skeletal::{Animation, Definition, Skeleton},
};
const CAT: &[&str] = &["sit_idle1", "sit_smile_open"];
pub(super) fn spec() -> SceneSpec {
    SceneSpec {
        id: INTRO,
        version: 1,
        duration: 3600.,
        shots: &[ShotSpec {
            start: 0.,
            track: "tower2_p1",
            offset: 0.,
            hold: 3600.,
        }],
        cues: &[],
        end: EndSpec {
            landing: None,
            exit: None,
        },
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Intro {
    pub clock: SceneState,
    pub skip: Option<f32>,
    pub dialogue_done: bool,
}
impl Intro {
    pub fn new() -> Self {
        Self {
            clock: SceneState::new(&spec()),
            skip: None,
            dialogue_done: false,
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.clock.validate(&spec())?;
        ensure!(
            !self.clock.finished && self.clock.line < 1,
            "Invalid Tower scene"
        );
        if let Some(t) = self.skip {
            state::clock("Tower skip", t, 0.5)?;
        }
        Ok(())
    }
}
pub(super) struct Data {
    pub yaw: f32,
    pub camera: Spline,
    pub cat: Transform,
    pub idle: f32,
    pub smile: f32,
    pub bubbles: Vec<Vec3>,
    pub bubble_bounds: (Vec3, Vec3),
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == "cat_pos1"))
            .context("Missing Tower Cat marker")?;
        let cat = Transform {
            translation: interaction::vector(&e["origin"]).context("Bad Cat position")?,
            rotation: Quat::from_rotation_z(
                e.get("angle")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.)
                    .to_radians(),
            ),
        };
        let d = Definition::load(a, "models/c_cheshire.tik")?;
        let rig = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
        let mut lengths = vec![];
        for n in CAT {
            let c = Animation::parse(
                &a.read(&format!("{}/{}", d.path, d.animations[*n]))?,
                rig.bones.len(),
            )?;
            lengths.push(c.duration());
        }
        let bubble = crate::tan::Model::parse(&a.read("models/fx/fx_boojum_scream/scream.tan")?)?;
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = -lo;
        for p in bubble.surfaces.iter().flat_map(|s| &s.frames).flatten() {
            lo = lo.min(*p * 5.);
            hi = hi.max(*p * 5.);
        }
        let bubbles = map
            .entities
            .iter()
            .filter(|e| e.get("model").is_some_and(|m| m == "fx_bubbles_air.tik"))
            .filter_map(|e| e.get("origin").and_then(|s| interaction::vector(s)))
            .collect();
        Ok(Self {
            yaw: interaction::spawn(map, None).1,
            camera: Spline::camera_track(Track::load(a, "tower2_p1")?.controls().collect()),
            cat,
            idle: lengths[0],
            smile: lengths[1],
            bubbles,
            bubble_bounds: (lo, hi),
        })
    }
    fn fade_at(&self) -> f32 {
        2.5 + self.idle + self.smile
    }
}
impl Tower {
    pub(super) fn advance_intro(&mut self, dt: f32, w: &World, p: &mut Player) -> Result<()> {
        let Some(s) = &mut self.saved.scene else {
            return Ok(());
        };
        if s.clock.home.is_none() {
            p.script_facing = self.data.yaw;
        }
        {
            let mut r = SceneRunner {
                spec: &spec(),
                state: &mut s.clock,
            };
            r.capture(p);
            r.advance(dt);
        }
        p.velocity = Vec3::ZERO;
        p.script_motion = 1;
        p.cancel_climb();
        p.release_rope();
        let done = if let Some(t) = &mut s.skip {
            *t = (*t + dt).min(0.5);
            *t >= 0.5
        } else {
            s.clock.time >= self.data.fade_at() + 1. && s.dialogue_done
        };
        if done {
            SceneRunner {
                spec: &spec(),
                state: &mut s.clock,
            }
            .finish_at_home(w, p)?;
            self.saved.scene = None;
            self.saved.arrived = true;
            self.saved.fade = 0.5;
            p.script_motion = 0;
        }
        Ok(())
    }
    pub(super) fn intro_fade(&self) -> Option<(Color, f32)> {
        let Some(s) = &self.saved.scene else {
            return (self.saved.fade > 0.).then_some((WHITE, self.saved.fade / 0.5));
        };
        if let Some(t) = s.skip {
            return Some((WHITE, t / 0.5));
        }
        if s.clock.time < 2. {
            Some((BLACK, 1. - s.clock.time / 2.))
        } else {
            Some((
                WHITE,
                ((s.clock.time - self.data.fade_at() - 0.5) / 0.5).clamp(0., 1.),
            ))
        }
    }
    fn cat_alpha(&self) -> f32 {
        self.saved.scene.as_ref().map_or(0., |s| {
            ((s.clock.time - 2.5) / 2.).clamp(0., 1.)
                * (1. - (s.clock.time - self.data.fade_at()) / 2.).clamp(0., 1.)
        })
    }
    pub(super) fn sounds(&self, clocks: &mut Vec<crate::audio::world::Clock>) {
        use crate::audio::world::Clock;
        for (i, key) in ["tower2.flush1", "tower2.flush2", "tower2.flush3"]
            .iter()
            .enumerate()
        {
            if let Some(time) = self.saved.stages[i] {
                clocks.push(Clock {
                    key,
                    time,
                    period: None,
                    origin: self
                        .objects
                        .iter()
                        .find(|o| o.name == format!("fliptop{}", i + 1))
                        .unwrap()
                        .base,
                    cues: &[
                        (0., "sound/ambience/special/flush_water.wav"),
                        (0., "sound/ambience/special/flush_platform.wav"),
                    ],
                });
            }
        }
        if let Some(s) = &self.saved.scene {
            if s.skip.is_none() {
                clocks.push(Clock {
                    key: "tower2.cat.appear",
                    time: s.clock.time,
                    period: None,
                    origin: self.data.cat.translation,
                    cues: &[(2.5, "sound/character/cheshire_cat/appear.wav")],
                });
                if s.clock.time >= self.data.fade_at() {
                    clocks.push(Clock {
                        key: "tower2.cat.leave",
                        time: s.clock.time - self.data.fade_at(),
                        period: None,
                        origin: self.data.cat.translation,
                        cues: &[(0., "sound/character/cheshire_cat/disappear.wav")],
                    });
                }
            }
        }
    }
}
pub(super) struct Art {
    cat: Puppet,
    alice: Puppet,
}
impl Art {
    pub fn load(a: &mut Assets) -> Result<Self> {
        let m = crate::texture::read_materials(a)?;
        Ok(Self {
            cat: Puppet::load(a, "c_cheshire", CAT, &m)?,
            alice: Puppet::load(a, "alice", &["idle_stand"], &m)?,
        })
    }
}
impl LevelArt for Art {
    fn story_pose(&mut self, s: &Story) {
        self.cat.mouth(s.mouth(&["cat_actor1"]));
        self.cat.mouth_angle(45.);
    }
    fn draw(
        &mut self,
        l: &dyn LevelController,
        atmo: &crate::environment::Atmosphere,
        camera: Vec3,
        bright: bool,
    ) {
        let t = l.downcast_ref::<Tower>().unwrap();
        let Some(s) = &t.saved.scene else {
            return;
        };
        if let Some(home) = s.clock.home {
            self.alice.atmosphere(atmo, camera);
            self.alice
                .draw("idle_stand", s.clock.time, true, home, 1., bright);
            let mut watch = crate::facial::Watch::default();
            watch.update(
                10.,
                Some(
                    t.data.cat.rotation.conjugate()
                        * (home.translation + Vec3::Z * 48. - t.data.cat.translation),
                ),
            );
            self.cat.watch(watch);
        }
        let alpha = t.cat_alpha();
        if alpha <= 0. {
            return;
        }
        let age = (s.clock.time - 2.5).max(0.);
        let (clip, time, looping) = if age < t.data.idle {
            ("sit_idle1", age, false)
        } else if age < t.data.idle + t.data.smile {
            ("sit_smile_open", age - t.data.idle, false)
        } else {
            ("sit_idle1", age - t.data.idle - t.data.smile, true)
        };
        self.cat.atmosphere(atmo, camera);
        if alpha >= 0.999 {
            self.cat.draw(clip, time, looping, t.data.cat, 1., bright);
        } else {
            self.cat.draw_afterimage(clip, time, t.data.cat, alpha);
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

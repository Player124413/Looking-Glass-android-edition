//! Observatory presentation; quest gates and the physical lift remain School-owned.
use super::*;
use crate::{cinematic::Camera, fortress::spline::Spline, school_return::Phase};

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Beat {
    Star,
    Exit,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub beat: Beat,
    pub time: f32,
    pub home: Option<Vec3>,
    pub active: bool,
    pub skipped: bool,
    pub doors_closed: bool,
}
impl State {
    pub fn new(beat: Beat) -> Self {
        Self {
            beat,
            time: 0.,
            home: None,
            active: true,
            skipped: false,
            doors_closed: false,
        }
    }
    pub fn validate(&self, phase: Phase) -> Result<()> {
        anyhow::ensure!(
            self.time.is_finite()
                && (0. ..=30.).contains(&self.time)
                && self
                    .home
                    .is_none_or(|p| p.is_finite() && p.abs().max_element() < 100000.)
                && match self.beat {
                    Beat::Star => matches!(phase, Phase::Rising | Phase::Observatory),
                    Beat::Exit => phase >= Phase::Opening,
                },
            "Invalid observatory performance"
        );
        Ok(())
    }
    pub fn fade(&self) -> f32 {
        let t = self.time;
        let fade = |start: f32| ((t - start) / 0.5).clamp(0., 1.);
        match self.beat {
            Beat::Star if t < 0.5 => fade(0.),
            Beat::Star if t < 1. => 1. - fade(0.5),
            Beat::Star if t < 6. => fade(5.5),
            Beat::Star => 1. - fade(6.),
            Beat::Exit if t < 7.5 => fade(7.),
            Beat::Exit if t < 8. => 1. - fade(7.5),
            Beat::Exit if t < 11.8 => fade(11.3),
            Beat::Exit if t < 12. => 1.,
            Beat::Exit => 1. - fade(12.),
        }
    }
}
pub struct Data {
    tracks: BTreeMap<String, Spline>,
    pub points: BTreeMap<String, Transform>,
    jump: Spline,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut tracks = BTreeMap::new();
        for name in [
            "skool1_olift_p1",
            "skool1_path1",
            "skool1_obs1",
            "skool1_obs2",
        ] {
            let track = crate::cinematic::Track::load(assets, name)?;
            tracks.insert(
                name.into(),
                Spline::camera_track(track.controls().collect()),
            );
        }
        let mut points = BTreeMap::new();
        for e in &map.entities {
            if let (Some(n), Some(p)) =
                (e.get("targetname"), e.get("origin").and_then(|s| vector(s)))
            {
                points.entry(n.clone()).or_insert(Transform {
                    translation: p,
                    rotation: Quat::from_rotation_z(
                        e.get("angle")
                            .and_then(|v| v.parse::<f32>().ok())
                            .unwrap_or(0.)
                            .to_radians(),
                    ),
                });
            }
        }
        for n in [
            "alice_olift_pos",
            "alice_drink_pos",
            "ob_star",
            "globe_puzzle",
        ] {
            anyhow::ensure!(points.contains_key(n), "Missing observatory marker {n}");
        }
        let mut controls = Vec::new();
        let mut name = "alice_jump";
        while let Some(e) = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == name))
        {
            let pose = points[name];
            controls.push((
                pose.translation,
                pose.rotation,
                e.get("speed")
                    .and_then(|v| v.parse::<f32>().ok())
                    .unwrap_or(1.),
            ));
            anyhow::ensure!(controls.len() <= 64, "Cyclic observatory jump");
            let Some(next) = e.get("target") else { break };
            name = next;
        }
        anyhow::ensure!(controls.len() >= 2, "Missing observatory jump");
        Ok(Self {
            tracks,
            points,
            jump: Spline::new(controls, false),
        })
    }
    pub fn actor(&self, s: &State) -> (Transform, &'static str, f32, f32) {
        let t = s.time;
        if s.beat == Beat::Star {
            let mut pose = self.points["alice_olift_pos"];
            // The repaired lift floor is lower than the native presentation marker.
            pose.translation.z = s.home.map_or(pose.translation.z, |p| p.z);
            return (pose, "idle_stand", t, 1.);
        }
        let mut pose = self.points["alice_drink_pos"];
        if t < 7.5 {
            pose.translation = s.home.unwrap_or(pose.translation);
            (pose, "idle_stand", t, 1.)
        } else if t < 11.3 {
            (pose, "drink", t - 7.5, 1.)
        } else if t < 11.8 {
            (pose, "jump_long_takeoff", t - 11.3, 1.)
        } else {
            pose.translation = self.jump.sample(t - 11.8, false).translation;
            (
                pose,
                "jump_falling3",
                t - 11.8,
                (1. - ((t - 12.).max(0.) / 0.1).floor() * 0.04).clamp(0.01, 1.),
            )
        }
    }
    pub fn star(&self, s: &State) -> Transform {
        let mut pose = self.points["ob_star"];
        pose.translation.z += (s.time - 2.).clamp(0., 12.) * 64.;
        pose.rotation *= Quat::from_rotation_z((s.time - 0.5).max(0.) * 120_f32.to_radians());
        pose
    }
}
impl School {
    pub(crate) fn return_scene_id(&self) -> Option<&'static str> {
        let s = self.return_visit.as_ref()?.scene.as_ref()?;
        // The shortened tail still owns input and the camera until handoff.
        s.active.then_some(match s.beat {
            Beat::Star => "Skool1_OLift_Up",
            Beat::Exit => "observatory_exit_cinematic",
        })
    }
    pub(crate) fn return_camera(&self) -> Option<Camera> {
        let s = self.return_visit.as_ref()?.scene.as_ref()?;
        if !s.active {
            return None;
        }
        let (name, age) = match s.beat {
            Beat::Star => ("skool1_olift_p1", (s.time - 0.5).max(0.)),
            Beat::Exit if s.time < 7.5 => ("skool1_path1", s.time),
            Beat::Exit if s.time < 11.8 => ("skool1_obs1", s.time - 7.5),
            Beat::Exit => ("skool1_obs2", s.time - 11.8),
        };
        Some(self.return_data.as_ref()?.tracks[name].camera(age))
    }
    pub(crate) fn skip_return_scene(&mut self) -> bool {
        let Some(s) = self
            .return_visit
            .as_mut()
            .and_then(|r| r.scene.as_mut())
            .filter(|s| s.active && !s.skipped)
        else {
            return false;
        };
        s.skipped = true;
        // Keep the closing fade and jump tail; no item grant or direct map change.
        s.time = s.time.max(match s.beat {
            Beat::Star => 5.5,
            Beat::Exit => 11.3,
        });
        true
    }
    pub(crate) fn return_step(&mut self, dt: f32, player: &mut Player) {
        let Some(r) = &mut self.return_visit else {
            return;
        };
        let Some(s) = &mut r.scene else { return };
        s.time = (s.time + dt).min(30.);
        if !s.active {
            return;
        }
        let home = *s.home.get_or_insert(player.feet);
        r.position = home;
        player.feet = home;
        player.velocity = Vec3::ZERO;
        player.grounded = true;
        player.script_motion = 1;
        player.cancel_climb();
        player.release_rope();
        let star = s.beat == Beat::Star;
        let close = star && s.time >= 4. && !s.doors_closed;
        if close {
            s.doors_closed = true;
        }
        let lift = star && s.time >= 6.5;
        if lift {
            s.active = false;
            player.script_motion = 0;
        }
        if !star {
            r.phase = if s.time >= 14.5 {
                Phase::Complete
            } else if s.time >= 11.8 {
                Phase::Shrinking
            } else if s.time >= 7.5 {
                Phase::Drinking
            } else {
                Phase::Opening
            };
            r.time = s.time
                - if s.time >= 11.8 {
                    11.8
                } else if s.time >= 7.5 {
                    7.5
                } else {
                    0.
                };
            if r.phase >= Phase::Drinking {
                r.potion = false;
            }
            r.position = self.return_data.as_ref().unwrap().actor(s).0.translation;
        }
        if close {
            for name in ["star_door1", "star_door2"] {
                let o = self.object_mut(name);
                o.move_to(o.base, 1.);
            }
        }
        if lift {
            let o = self.object_mut("observatory_lift");
            o.move_to(
                Pose {
                    origin: o.base.origin + Vec3::Z * 632.,
                    ..o.base
                },
                5.,
            );
        }
    }
    pub(crate) fn return_sound(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        let Some(r) = &self.return_visit else { return };
        if let Some(s) = &r.scene {
            if !s.skipped {
                clocks.push(crate::audio::world::Clock {
                    key: if s.beat == Beat::Star {
                        "return-star"
                    } else {
                        "return-exit"
                    },
                    time: s.time,
                    period: None,
                    origin: r.position,
                    cues: if s.beat == Beat::Star {
                        &[
                            (0.5, "sound/character/gnome/elder/vanish.wav"),
                            (4., "sound/world/door/door wood open 01.wav"),
                        ]
                    } else {
                        &[
                            (3., "sound/world/machine/globe_open.wav"),
                            (7.5, "sound/character/alice/drink.wav"),
                        ]
                    },
                });
            }
            if let Some(m) = &self.object("observatory_lift").motion {
                loops.push(crate::audio::LoopCue {
                    id: 1003,
                    path: "sound/ambience/special/observatory_lift.wav",
                    origin: self.object("observatory_lift").pose.origin,
                    clock: Some(m.time),
                });
            }
        }
    }
}

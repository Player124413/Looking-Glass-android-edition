//! Puzzle presentation has no authority to grant puzzle or route progress.
use super::*;
use crate::{assets::Assets, cinematic::Camera, fortress::spline::Spline};
use anyhow::Context;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Beat {
    Demonstration,
    Reset,
    Solved,
    Arches,
    Rage,
}
impl Beat {
    pub fn duration(self) -> f32 {
        match self {
            Self::Demonstration => 12.7,
            Self::Reset => 7.1,
            Self::Solved => 7.9,
            Self::Arches => 17.,
            Self::Rage => crate::power_pose::RAGE_SECONDS,
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Demonstration => "Start_GetSmart",
            Self::Reset => "Reset_GetSmart",
            Self::Solved => "End_GetSmart",
            Self::Arches => "ARCH_CAM_MOVE",
            Self::Rage => "Fortress2_Rage_Pickup",
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub beat: Beat,
    pub time: f32,
    pub finished: bool,
    pub home: Option<Transform>,
}
impl State {
    pub fn new(beat: Beat) -> Self {
        Self {
            beat,
            time: 0.,
            finished: false,
            home: None,
        }
    }
    pub fn validate(&self, puzzle: &super::State) -> Result<()> {
        ensure!(
            self.time.is_finite()
                && (0. ..=self.beat.duration() + 0.51).contains(&self.time)
                && (!self.finished || self.time >= self.beat.duration())
                && self.home.is_none_or(|p| p.translation.is_finite()
                    && p.translation.abs().max_element() < 100000.
                    && p.rotation.is_finite()
                    && (p.rotation.length_squared() - 1.).abs() < 0.01),
            "Invalid Beyond camera state"
        );
        ensure!(
            match self.beat {
                Beat::Arches => puzzle.raised.is_some(),
                Beat::Rage => puzzle.rage_started.is_some(),
                Beat::Solved => puzzle.solved,
                Beat::Demonstration | Beat::Reset =>
                    puzzle.puzzle_started && (self.finished || !puzzle.solved),
            },
            "Camera does not match puzzle state"
        );
        Ok(())
    }
}
pub struct Data {
    demonstration: Spline,
    solved: Spline,
    arches: Spline,
    reset: Camera,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut load = |name| -> Result<Spline> {
            Ok(Spline::camera_track(
                crate::cinematic::Track::load(assets, name)?
                    .controls()
                    .collect(),
            ))
        };
        let demonstration = load("path_getsmart")?;
        let solved = load("getsmart_complete")?;
        let arches = load("fortress2_archroom")?;
        let marker = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == "reset_path2"))
            .context("Missing puzzle reset camera")?;
        let eye = marker
            .get("origin")
            .and_then(|s| vector(s))
            .context("Invalid reset camera")?;
        let angles = marker
            .get("angles")
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO);
        let q = rot(angles.x, angles.y, angles.z);
        Ok(Self {
            demonstration,
            solved,
            arches,
            reset: Camera {
                eye,
                target: eye + q * Vec3::X * 100.,
                up: q * Vec3::Z,
            },
        })
    }
}
impl Beyond {
    pub fn configure_cinema(&mut self, assets: &mut Assets, map: &Bsp) -> Result<()> {
        self.cinema_data = Some(Data::load(assets, map)?);
        Ok(())
    }
    pub fn scene_id(&self) -> Option<&'static str> {
        self.state
            .cinema
            .as_ref()
            .filter(|s| !s.finished)
            .map(|s| s.beat.id())
    }
    pub fn scene_camera(&self) -> Option<Camera> {
        let s = self.state.cinema.as_ref().filter(|s| !s.finished)?;
        let data = self.cinema_data.as_ref()?;
        Some(match s.beat {
            Beat::Demonstration => data.demonstration.camera(s.time),
            Beat::Reset => data.reset,
            Beat::Solved => data.solved.camera(s.time),
            Beat::Arches => data.arches.camera((s.time - 0.5).max(0.)),
            Beat::Rage => crate::power_pose::rage_camera(s.time, s.home?),
        })
    }
    pub fn rage_scene(&self) -> Option<(f32, bool)> {
        self.state
            .cinema
            .as_ref()
            .filter(|s| s.beat == Beat::Rage)
            .map(|s| (s.time, s.finished))
    }
    /// Capture body facing once at collection; a restored shot retains its basis.
    pub fn bind_rage_player(&mut self, player: &mut Player, facing: f32) {
        if self
            .state
            .cinema
            .as_ref()
            .is_some_and(|s| s.beat == Beat::Rage && !s.finished && s.home.is_none())
        {
            player.script_facing = facing;
            self.scene_hold(player);
        }
    }
    pub fn scene_camera_in(&self, world: &World) -> Option<Camera> {
        let mut camera = self.scene_camera()?;
        if let Some(s) = self.state.cinema.as_ref().filter(|s| s.beat == Beat::Rage) {
            let center = s.home?.translation + PLAYER_CENTER;
            let trace = world.sweep(center, camera.eye, Vec3::splat(5.));
            if !trace.start_solid {
                camera.eye = center.lerp(camera.eye, trace.fraction);
            }
        }
        Some(camera)
    }
    pub fn scene_fovy(&self, aspect: f32) -> Option<f32> {
        let s = self.state.cinema.as_ref().filter(|s| !s.finished)?;
        // This track declares horizontal FOV 120. Other shots retain the port lens.
        (s.beat == Beat::Arches)
            .then(|| 2. * ((60_f32.to_radians()).tan() / aspect.max(0.1)).atan())
    }
    pub fn scene_fade(&self) -> (Color, f32) {
        let Some(s) = &self.state.cinema else {
            return (WHITE, 0.);
        };
        if s.beat == Beat::Rage {
            return (WHITE, 0.);
        }
        if s.finished {
            return (WHITE, 1. - ramp(s.time, s.beat.duration(), 0.5));
        }
        let opening = match s.beat {
            Beat::Reset | Beat::Arches => 1. - ramp(s.time, 0., 0.5),
            _ => 0.,
        };
        let closing = ramp(s.time, s.beat.duration() - 0.5, 0.5);
        (WHITE, opening.max(closing))
    }
    pub fn levers_visible(&self) -> bool {
        !self.state.solved
            || self
                .state
                .cinema
                .as_ref()
                .is_some_and(|s| s.beat == Beat::Solved && !s.finished)
    }
    pub(super) fn door_open(&self, i: usize) -> f32 {
        if let Some(s) = self.state.cinema.as_ref().filter(|s| !s.finished) {
            match s.beat {
                Beat::Demonstration => return 1. - ramp(s.time, 1.5 + (i - 1) as f32, 0.2),
                Beat::Reset => {
                    return ramp(s.time, 0.5, 0.1)
                        * (1. - ramp(s.time, 0.6 + (i - 1) as f32 * 1.5, 1.5))
                }
                Beat::Solved => return ramp(s.time, 7. - i as f32, 1.4),
                Beat::Arches | Beat::Rage => (),
            }
        }
        if self.state.puzzle_started && !self.state.solved {
            0.
        } else {
            1.
        }
    }
    pub(super) fn note_time(&self) -> f32 {
        match self.state.cinema.as_ref().map(|s| s.beat) {
            Some(Beat::Demonstration) => 1.7 + self.state.next_note as f32,
            Some(Beat::Reset) => 2.1 + self.state.next_note as f32 * 1.5,
            _ => 0.5 + self.state.next_note as f32,
        }
    }
    // Freeze input/velocity, but let the machinery's normal rider transform carry
    // Alice. The saved home follows that transform; no fixed world-space teleport.
    pub(super) fn scene_hold(&mut self, player: &mut Player) {
        if let Some(s) = self.state.cinema.as_mut().filter(|s| !s.finished) {
            s.home.get_or_insert(Transform {
                translation: player.feet,
                rotation: Quat::from_rotation_z(player.script_facing),
            });
            player.velocity = Vec3::ZERO;
            player.cancel_climb();
            player.release_rope();
            player.script_motion = 1;
        }
    }
    pub(super) fn scene_finish_step(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let Some(s) = self.state.cinema.as_mut().filter(|s| !s.finished) else {
            return Ok(());
        };
        if let Some(home) = &mut s.home {
            home.translation = player.feet;
        }
        if s.time >= s.beat.duration() {
            self.finish_scene(world, player)?;
        }
        Ok(())
    }
    fn finish_scene(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let Some(s) = self.state.cinema.as_mut().filter(|s| !s.finished) else {
            return Ok(());
        };
        let home = s.home.unwrap_or(Transform {
            translation: player.feet,
            rotation: Quat::from_rotation_z(player.script_facing),
        });
        crate::cinematic::land_player(player, world, home)?;
        s.time = s.beat.duration();
        s.finished = true;
        if s.beat == Beat::Rage && matches!(self.state.rage_lift, Some(RageLift::Waiting)) {
            self.state.rage_lift = Some(RageLift::Lowering(self.state.age));
        }
        Ok(())
    }
    pub fn skip_scene(&mut self, map: &Bsp, world: &World, player: &mut Player) -> Result<bool> {
        if self.scene_id().is_none() {
            return Ok(false);
        }
        // Completion may release the Rage lift, but cannot grant puzzle progress
        // or advance map time. Its normal rider path owns the subsequent descent.
        self.finish_scene(world, player)?;
        if self
            .state
            .cinema
            .as_ref()
            .is_some_and(|s| matches!(s.beat, Beat::Demonstration | Beat::Reset))
        {
            self.state.note_replay = None;
        }
        self.rebuild(map)?;
        Ok(true)
    }
}

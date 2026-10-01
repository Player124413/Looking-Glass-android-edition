//! Saved lifecycle shared by opted-in scenes; no script execution or visit controller.
use super::{exit::ExitState, spec::SceneSpec};
use crate::{collision::World, movement::Player, skeletal::Transform};
use anyhow::{ensure, Result};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneState {
    pub id: String,
    pub version: u8,
    pub time: f32,
    pub shot: usize,
    pub shot_time: f32,
    pub cast_time: f32,
    pub line: usize,
    pub line_time: f32,
    pub fired: u64,
    pub finished: bool,
    pub home: Option<Transform>,
    pub exit: ExitState,
}
impl SceneState {
    pub fn new(spec: &SceneSpec) -> Self {
        Self {
            id: spec.id.into(),
            version: spec.version,
            time: 0.,
            shot: 0,
            shot_time: 0.,
            cast_time: 0.,
            line: 0,
            line_time: 0.,
            fired: 0,
            finished: false,
            home: None,
            exit: ExitState::default(),
        }
    }
    pub fn validate(&self, spec: &SceneSpec) -> Result<()> {
        ensure!(
            self.id == spec.id
                && self.version == spec.version
                && self.shot < spec.shots.len()
                && self.line < 4096
                && [
                    self.time,
                    self.shot_time,
                    self.cast_time,
                    self.line_time,
                    self.exit.retry_time
                ]
                .iter()
                .all(|t| t.is_finite() && *t >= 0. && *t <= 1e6)
                && self.time <= spec.duration
                && self.cast_time <= spec.duration
                && self.shot_time <= self.time
                && self.fired & !cue_mask(spec) == 0
                && (!self.finished || self.time == spec.duration)
                && self.exit.committed == (self.finished && spec.end.exit.is_some())
                && self.home.is_none_or(|h| h.translation.is_finite()
                    && h.translation.abs().max_element() < 100_000.
                    && h.rotation.is_finite()
                    && (h.rotation.length_squared() - 1.).abs() < 0.01),
            "Invalid scene state {}",
            spec.id
        );
        Ok(())
    }
}
fn cue_mask(spec: &SceneSpec) -> u64 {
    assert!(spec.cues.len() < 64);
    (1u64 << spec.cues.len()) - 1
}
pub struct SceneRunner<'a> {
    pub spec: &'a SceneSpec,
    pub state: &'a mut SceneState,
}
impl SceneRunner<'_> {
    pub fn capture(&mut self, player: &Player) {
        self.state.home.get_or_insert(Transform {
            translation: player.feet,
            rotation: macroquad::prelude::Quat::from_rotation_z(player.script_facing),
        });
    }
    /// Owners pass zero while paused. No clock, cue or retry advances then.
    pub fn advance(&mut self, dt: f32) -> bool {
        if !dt.is_finite() || dt <= 0. {
            return false;
        }
        let dt = dt.min(0.1);
        self.state.exit.advance(dt);
        if self.state.finished {
            return false;
        }
        let s = &mut self.state;
        s.time = (s.time + dt).min(self.spec.duration);
        s.cast_time = s.time;
        s.line_time += dt;
        self.select_shot();
        self.state.time >= self.spec.duration
    }
    fn select_shot(&mut self) {
        self.state.shot = self
            .spec
            .shots
            .partition_point(|shot| shot.start <= self.state.time)
            .saturating_sub(1);
        self.state.shot_time = self.state.time - self.spec.shots[self.state.shot].start;
    }
    pub fn line(&mut self, line: usize) {
        if self.state.line != line && !self.state.finished {
            self.state.line = line;
            self.state.line_time = 0.;
        }
    }
    pub fn cue(&mut self, index: usize) -> bool {
        let mask = 1u64 << index;
        if self.state.finished
            || self.state.time < self.spec.cues[index]
            || self.state.fired & mask != 0
        {
            return false;
        }
        self.state.fired |= mask;
        true
    }
    /// Natural playback and skip call this exact path. Commit only after landing succeeds.
    pub fn finish(&mut self, world: &World, player: &mut Player) -> Result<bool> {
        if self.state.finished {
            return Ok(false);
        }
        self.capture(player);
        if self.spec.end.exit.is_none() {
            crate::cinematic::land_player(
                player,
                world,
                self.spec.end.landing.or(self.state.home).unwrap(),
            )?;
        }
        self.commit();
        Ok(true)
    }
    /// Some scenes explicitly return to the untouched pre-scene pose. Do not run
    /// a landing search: it can choose the fallen deck below an approach edge.
    pub fn finish_at_home(&mut self, world: &World, player: &mut Player) -> Result<bool> {
        if self.state.finished {
            return Ok(false);
        }
        self.capture(player);
        let home = self.state.home.unwrap();
        ensure!(world.body_clear(home.translation), "Scene home obstructed");
        let half = crate::collision::PLAYER_HALF;
        let center = home.translation + crate::collision::PLAYER_CENTER;
        let floor = world.sweep(center, center - macroquad::prelude::Vec3::Z * 8., half);
        ensure!(
            floor.fraction < 1. && floor.normal.z > 0.65,
            "Scene home has no remaining support"
        );
        player.feet = home.translation;
        player.velocity = macroquad::prelude::Vec3::ZERO;
        player.script_facing = home.rotation.to_euler(macroquad::prelude::EulerRot::ZYX).0;
        player.script_motion = 0;
        player.cancel_climb();
        player.release_rope();
        player.grounded = true;
        player.ground_normal = floor.normal;
        player.immersion = crate::water::Immersion::sample(world, player.feet);
        player.swimming = player.immersion.level >= 2;
        self.commit();
        Ok(true)
    }
    fn commit(&mut self) {
        self.state.time = self.spec.duration;
        self.state.cast_time = self.spec.duration;
        self.state.line_time = 0.;
        self.select_shot();
        self.state.fired = cue_mask(self.spec);
        self.state.finished = true;
        self.state.exit.committed = self.spec.end.exit.is_some();
    }
    pub fn track_time(spec: &SceneSpec, state: &SceneState) -> (&'static str, f32) {
        let shot = &spec.shots[state.shot];
        (
            shot.track,
            (state.shot_time + shot.offset).clamp(0., shot.hold),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::spec::{EndSpec, ShotSpec};
    use macroquad::prelude::*;
    const SPEC: SceneSpec = SceneSpec {
        id: "fixture",
        version: 1,
        duration: 5.,
        shots: &[
            ShotSpec {
                start: 0.,
                track: "a",
                offset: -0.5,
                hold: 1.,
            },
            ShotSpec {
                start: 2.,
                track: "b",
                offset: 0.25,
                hold: 2.,
            },
        ],
        cues: &[1., 3.],
        end: EndSpec {
            landing: None,
            exit: None,
        },
    };
    #[test]
    fn exact_home_preserves_offset_and_refuses_removed_support() {
        let world = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        let mut s = SceneState::new(&SPEC);
        let mut p = Player::new(vec3(13., 17., 2.));
        let home = p.feet;
        let mut r = SceneRunner {
            spec: &SPEC,
            state: &mut s,
        };
        r.capture(&p);
        p.feet = vec3(40., 40., 0.);
        let empty = World::fixture(&[]);
        assert!(r.finish_at_home(&empty, &mut p).is_err());
        assert!(!r.state.finished && p.feet == vec3(40., 40., 0.));
        assert!(r.finish_at_home(&world, &mut p).unwrap());
        assert_eq!(p.feet, home);
        assert!(p.grounded && p.velocity == Vec3::ZERO);
        assert!(!r.finish_at_home(&world, &mut p).unwrap());
    }
    #[test]
    fn clocks_cues_shots_and_lines_survive_pause_and_serialization() {
        let mut s = SceneState::new(&SPEC);
        let mut r = SceneRunner {
            spec: &SPEC,
            state: &mut s,
        };
        for _ in 0..25 {
            r.advance(0.1);
        }
        assert!(r.cue(0));
        assert!(!r.cue(0) && !r.cue(1));
        r.line(2);
        for _ in 0..3 {
            r.advance(0.1);
        }
        assert_eq!(r.state.shot, 1);
        assert!((r.state.line_time - 0.3).abs() < 1e-5);
        let saved = serde_json::to_vec(r.state).unwrap();
        r.advance(0.);
        assert_eq!(saved, serde_json::to_vec(r.state).unwrap());
        *r.state = serde_json::from_slice(&saved).unwrap();
        r.state.validate(&SPEC).unwrap();
        assert!(!r.cue(0));
        let (_, t) = SceneRunner::track_time(&SPEC, r.state);
        assert!((t - 1.05).abs() < 1e-5);
        for _ in 0..3 {
            r.advance(0.1);
        }
        assert!(r.cue(1));
    }
    #[test]
    fn watching_and_skipping_share_completion_and_vertical_landing() {
        let world = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        let mut digest = None;
        for hz in [30, 60, 144] {
            for skip in [None, Some(0.), Some(2.5), Some(4.9)] {
                let mut s = SceneState::new(&SPEC);
                let mut p = Player::new(vec3(0., 0., 2.));
                let mut r = SceneRunner {
                    spec: &SPEC,
                    state: &mut s,
                };
                r.capture(&p);
                for _ in 0..(6 * hz) {
                    if skip.is_some_and(|t| r.state.time >= t) || r.advance(1. / hz as f32) {
                        assert!(r.finish(&world, &mut p).unwrap());
                        break;
                    }
                }
                assert!(r.state.finished && !r.finish(&world, &mut p).unwrap());
                r.state.validate(&SPEC).unwrap();
                assert!(p.grounded && p.feet.z.abs() < 0.1 && p.velocity == Vec3::ZERO);
                let result = serde_json::to_value(r.state).unwrap();
                if let Some(expected) = &digest {
                    assert_eq!(&result, expected);
                } else {
                    digest = Some(result);
                }
            }
        }
    }
    #[test]
    fn obstructed_endpoint_does_not_commit_or_search_sideways() {
        let world = World::fixture(&[(vec3(-1., -1., 0.), vec3(1., 1., 100.))]);
        let mut s = SceneState::new(&SPEC);
        let mut p = Player::new(Vec3::ZERO);
        let mut r = SceneRunner {
            spec: &SPEC,
            state: &mut s,
        };
        assert!(r.finish(&world, &mut p).is_err());
        assert!(!r.state.finished && !r.state.exit.committed && p.feet == Vec3::ZERO);
    }
    #[test]
    fn exit_is_gated_one_shot_and_retries_only_after_failure_or_load() {
        let spec = crate::pool::cinema::ENDING.end.exit.unwrap();
        let mut e = ExitState::default();
        assert!(e.request(spec).is_none());
        e.committed = true;
        assert_eq!(e.request(spec), Some(spec.destination()));
        assert!(e.request(spec).is_none());
        e.failed();
        e.advance(0.);
        assert!(e.request(spec).is_none());
        e.advance(1.);
        assert_eq!(e.request(spec), Some(spec.destination()));
        let mut restored: ExitState =
            serde_json::from_value(serde_json::to_value(e).unwrap()).unwrap();
        assert_eq!(restored.request(spec), Some(spec.destination()));
        assert!(restored.request(spec).is_none());
    }
}

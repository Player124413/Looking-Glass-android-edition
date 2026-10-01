//! The entrance thread's cast and common watch/skip commitment, owned by Pool.
use super::cinema::{Beat, ENTRY};
use super::*;
use crate::level::{
    scene::{SceneRunner, SceneState},
    spec::{EndSpec, SceneSpec, ShotSpec},
};

pub(super) struct Data {
    enabled: bool,
    pub(super) rabbit_world: World,
    start: Transform,
    end: Transform,
    rabbit: [Transform; 4],
    routes: [Vec<Transform>; 3],
}
fn marker(map: &Bsp, name: &str) -> Result<Transform> {
    let e = map
        .entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|n| n == name))
        .with_context(|| format!("Missing arrival marker {name}"))?;
    Ok(Transform {
        translation: e
            .get("origin")
            .and_then(|s| vector(s))
            .context("Arrival marker origin")?,
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    })
}
impl Data {
    pub(super) fn load(map: &Bsp) -> Result<Self> {
        Ok(Self {
            enabled: false,
            rabbit_world: World::actor_world(map)?,
            start: marker(map, "potears1_start1")?,
            end: marker(map, "alice_posx1")?,
            routes: [
                vec![738, 745, 744, 743, 742, 741, 740, 739, 737],
                vec![737, 746, 747],
                vec![747, 748],
            ]
            .map(|ids| {
                ids.into_iter()
                    .map(|id| {
                        let e = &map.entities[id];
                        ensure!(
                            e.get("classname").is_some_and(|c| c == "info_pathnode"),
                            "Invalid Rabbit route node {id}"
                        );
                        Ok(Transform {
                            translation: e
                                .get("origin")
                                .and_then(|s| vector(s))
                                .context("Rabbit route origin")?,
                            rotation: Quat::IDENTITY,
                        })
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .into_iter()
            .collect::<Result<Vec<_>>>()?
            .try_into()
            .map_err(|_| anyhow::anyhow!("Rabbit route count"))?,
            rabbit: [
                marker(map, "rabbit_start_pos1")?,
                marker(map, "rabbit_run_pos1")?,
                marker(map, "rabbit_run_pos2")?,
                marker(map, "rabbit_run_pos3")?,
            ],
        })
    }
}
fn footing(world: &World, at: Transform, half: Vec3, lift: f32, drop: f32) -> Transform {
    let center = Vec3::Z * half.z;
    Transform {
        translation: world
            .actor_footing(at.translation + Vec3::Z * lift, center, half, drop)
            .unwrap_or(at.translation),
        ..at
    }
}
fn moving(a: Transform, b: Transform, distance: f32) -> Transform {
    let delta = b.translation - a.translation;
    let length = delta.truncate().length();
    if distance >= length {
        return b;
    }
    Transform {
        translation: a
            .translation
            .lerp(b.translation, (distance / length.max(0.001)).clamp(0., 1.)),
        rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
    }
}
impl Pool {
    /// Only the selected BSP entrance may request the introduction. Restores do
    /// not invoke this hook, and completed visits cannot request it again.
    pub fn select_arrival(&mut self, map: &Bsp, entry: Option<&str>) {
        self.cinema.arrival.enabled = map
            .entities
            .iter()
            .find(|e| {
                e.get("classname").is_some_and(|c| c == "info_player_start")
                    && entry.is_none_or(|n| e.get("targetname").is_some_and(|v| v == n))
            })
            .is_some_and(|e| e.get("thread").is_some_and(|t| t == ENTRY));
    }
    fn arrival_end(&self) -> Transform {
        footing(
            &self.cinema.camera_world,
            self.cinema.arrival.end,
            PLAYER_HALF,
            0.,
            64.,
        )
    }
    fn arrival_spec(&self) -> SceneSpec {
        let home = self
            .state
            .cinema
            .arrival
            .as_ref()
            .and_then(|a| a.home)
            .or(self.state.cinema.home)
            .unwrap_or(self.cinema.arrival.start);
        SceneSpec {
            id: ENTRY,
            version: 1,
            duration: 8.
                + (self.cinema.arrival.end.translation - home.translation)
                    .truncate()
                    .length()
                    / self.cinema.speed("alice", "walk"),
            shots: &[ShotSpec {
                start: 0.,
                track: "tears1_path4",
                offset: 0.,
                hold: 1e6,
            }],
            cues: &[4.],
            end: EndSpec {
                landing: Some(self.arrival_end()),
                exit: None,
            },
        }
    }
    pub(super) fn begin_arrival(&mut self) {
        if !self.cinema.arrival.enabled
            || self.state.cinema.done[0]
            || self.state.cinema.beat.is_some()
        {
            return;
        }
        let home = footing(
            &self.cinema.camera_world,
            self.cinema.arrival.start,
            PLAYER_HALF,
            0.,
            64.,
        );
        self.state.cinema.start(Beat::Entry);
        self.state.cinema.home = Some(home);
        let mut state = SceneState::new(&self.arrival_spec());
        state.home = Some(home);
        self.state.cinema.arrival = Some(state);
    }
    fn sync_arrival(&mut self) {
        let s = &mut self.state.cinema;
        if let Some(a) = &s.arrival {
            s.time = a.time;
            s.shot = a.shot;
            s.shot_time = a.shot_time;
            s.home = a.home;
            s.done[0] = a.finished;
        }
    }
    pub(super) fn restore_arrival(&mut self) -> Result<()> {
        let spec = self.arrival_spec();
        let s = &mut self.state.cinema;
        if s.version < 3 {
            if s.beat == Some(Beat::Entry) && !s.done[0] {
                let mut a = SceneState::new(&spec);
                a.time = s.time.min(spec.duration);
                a.cast_time = a.time;
                a.shot_time = a.time;
                a.home = s.home;
                a.fired = u64::from(s.rocks[3].is_some());
                s.arrival = Some(a);
            }
            s.version = 3;
        }
        if let Some(a) = &s.arrival {
            a.validate(&spec)?;
            ensure!(
                a.finished == s.done[0] && a.finished == (s.beat != Some(Beat::Entry)),
                "Inconsistent Pool arrival"
            );
        }
        ensure!(
            s.beat != Some(Beat::Entry) || s.arrival.is_some(),
            "Missing Pool arrival clock"
        );
        if s.beat == Some(Beat::Entry) {
            self.sync_arrival();
        }
        Ok(())
    }
    pub(super) fn advance_arrival(
        &mut self,
        dt: f32,
        world: &World,
        player: &mut Player,
    ) -> Result<()> {
        let spec = self.arrival_spec();
        let a = self
            .state
            .cinema
            .arrival
            .as_mut()
            .context("Missing arrival state")?;
        let mut runner = SceneRunner {
            spec: &spec,
            state: a,
        };
        runner.capture(player);
        let complete = runner.advance(dt);
        if runner.cue(0) {
            self.state.cinema.rocks[3]
                .get_or_insert((self.state.age - (runner.state.time - 4.).max(0.)).max(0.));
        }
        player.feet = runner.state.home.unwrap().translation;
        player.velocity = Vec3::ZERO;
        player.cancel_climb();
        player.release_rope();
        player.script_motion = 1;
        self.sync_arrival();
        if complete {
            self.finish_arrival(world, player)?;
        }
        Ok(())
    }
    pub(super) fn finish_arrival(&mut self, world: &World, player: &mut Player) -> Result<()> {
        let spec = self.arrival_spec();
        SceneRunner {
            spec: &spec,
            state: self
                .state
                .cinema
                .arrival
                .as_mut()
                .context("Missing arrival state")?,
        }
        .finish(world, player)?;
        self.sync_arrival();
        self.state.cinema.rocks[3].get_or_insert(self.state.age);
        self.state.cinema.beat = None;
        self.finish_arrival_rock();
        Ok(())
    }
    pub(super) fn arrival_alice(&self) -> (&'static str, f32, Transform) {
        let s = &self.state.cinema;
        let home = s.home.unwrap_or(self.cinema.arrival.start);
        if s.time < 8. {
            return ("idle_stand", s.time, home);
        }
        let time = s.time - 8.;
        let at = moving(
            home,
            self.cinema.arrival.end,
            time * self.cinema.speed("alice", "walk"),
        );
        (
            "walk",
            time,
            footing(&self.cinema.camera_world, at, PLAYER_HALF, 8., 64.),
        )
    }
    pub(super) fn arrival_rabbit_half(&self) -> Vec3 {
        let length: f32 = self.cinema.arrival.routes[0]
            .windows(2)
            .map(|p| (p[1].translation - p[0].translation).truncate().length())
            .sum();
        if (self.state.cinema.time - 5.).max(0.) * self.cinema.speed("c_whiterabbit", "run")
            < length
        {
            vec3(24., 24., 32.)
        } else {
            Vec3::splat(0.5)
        }
    }
    pub(super) fn arrival_rabbit(&self) -> (&'static str, f32, Transform) {
        let t = self.state.cinema.time;
        let nodes = &self.cinema.arrival.rabbit;
        if t < 5. {
            return (
                "idle",
                t,
                footing(
                    &self.cinema.arrival.rabbit_world,
                    nodes[0],
                    vec3(24., 24., 32.),
                    0.,
                    64.,
                ),
            );
        }
        let time = t - 5.;
        let mut distance = time * self.cinema.speed("c_whiterabbit", "run");
        for route in &self.cinema.arrival.routes {
            for pair in route.windows(2) {
                let length = (pair[1].translation - pair[0].translation)
                    .truncate()
                    .length();
                if distance < length {
                    let half = self.arrival_rabbit_half();
                    let at = moving(pair[0], pair[1], distance);
                    return (
                        "run",
                        time,
                        footing(&self.cinema.arrival.rabbit_world, at, half, 64., 512.),
                    );
                }
                distance -= length;
            }
        }
        (
            "idle",
            distance / self.cinema.speed("c_whiterabbit", "run"),
            footing(
                &self.cinema.arrival.rabbit_world,
                nodes[3],
                Vec3::splat(0.5),
                0.,
                64.,
            ),
        )
    }
}

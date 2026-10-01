use super::*;
use crate::{cinematic::Track, fortress::spline::Spline};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Data {
    pub world: World,
    pub points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Spline>,
    pub paths: BTreeMap<String, Spline>,
    pub rock: crate::falling_rock::Spec,
    pub travel: [f32; 4],
    pub shroom_triangles: Vec<[Vec3; 3]>,
    pub shroom_scale: f32,
}
pub(super) fn at(e: &crate::levels::Entity) -> Transform {
    let v = e
        .get("angles")
        .and_then(|s| crate::interaction::vector(s))
        .unwrap_or_else(|| {
            vec3(
                0.,
                e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
                0.,
            )
        });
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| crate::interaction::vector(s))
            .unwrap_or_default(),
        rotation: Quat::from_rotation_z(v.y.to_radians())
            * Quat::from_rotation_y(v.x.to_radians())
            * Quat::from_rotation_x(v.z.to_radians()),
    }
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = BTreeMap::new();
        for e in &map.entities {
            if let Some(n) = e.get("targetname") {
                points.entry(n.clone()).or_insert(at(e));
            }
        }
        for n in [
            "alice_walk_pos1",
            "alice_run_pos1",
            "garden3_start1",
            "big_bug1",
            "bug_marble1",
            "falling_pillar1",
            "falling_pillar_shroom1",
        ] {
            ensure!(points.contains_key(n), "Missing chase marker {n}");
        }
        let mut cameras = BTreeMap::new();
        for k in 1..=5 {
            let n = format!("garden3_path{k}");
            cameras.insert(
                n.clone(),
                Spline::camera_track(Track::load(a, &n)?.controls().collect()),
            );
        }
        let mut paths = BTreeMap::new();
        for n in [
            "bug_path1",
            "bug_path2",
            "bug_path4",
            "bug_path5",
            "bug_path6",
            "big_bugpath1",
            "lady_flyaway",
        ] {
            let mut next = Some(n);
            let mut nodes = Vec::new();
            let mut seen = BTreeSet::new();
            while let Some(name) = next {
                ensure!(
                    seen.insert(name) && seen.len() < 512,
                    "Cyclic fleet path {n}"
                );
                let e = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|v| v == name))
                    .context("Missing fleet node")?;
                let p = at(e);
                let speed: f32 = e.get("speed").map_or(Ok(1.), |v| v.parse())?;
                ensure!(speed.is_finite() && speed > 0., "Invalid fleet speed");
                nodes.push((p.translation, p.rotation, speed));
                next = e.get("target").map(String::as_str);
            }
            paths.insert(n.into(), Spline::new(nodes, false));
        }
        let mut rock = crate::falling_rock::Spec::load(map, "chase_marble1")?;
        ensure!(
            rock.nodes.len() == 56 && rock.flags == 6 && rock.damage == 999.,
            "Chase definition changed"
        );
        ensure!(
            rock.nodes.iter().filter_map(|n| n.thread.as_deref()).eq([
                "Garden3_FirstQuake",
                "Garden3_Pillar_Break1",
                "Garden3_Quake1",
                "Garden3_IceFloor_Break1"
            ]),
            "Unreviewed chase callbacks"
        );
        rock.speed = 175.;
        let mut travel = [0.; 4];
        for (k, &id) in DOORS.iter().enumerate() {
            let e = &map.entities[id];
            let m: usize = e["model"].trim_start_matches('*').parse()?;
            travel[k] = map.models[m].max.z
                - map.models[m].min.z
                - e.get("lip").map_or(Ok(8.), |v| v.parse::<f32>())?;
        }
        let shroom = map.entities.iter().find(|e|
            e.get("targetname").is_some_and(|n| n == "falling_pillar_shroom1"))
            .context("Missing pillar mushroom")?;
        let shroom_scale: f32 = shroom.get("scale").map_or(Ok(1.), |s| s.parse())?;
        ensure!(shroom_scale.is_finite() && shroom_scale > 0., "Invalid pillar mushroom scale");
        // Use the same first frame and scale as Prop::draw; a bounding box would
        // leave invisible corners around the round cap and fill the space below it.
        let (def, model) = crate::weapons::read_model(a, "g_shroom07")?;
        let shroom_triangles = model.surfaces.iter().flat_map(|s|
            s.indices.chunks_exact(3).map(|t|
                [t[0], t[1], t[2]].map(|i| s.frames[0][i as usize] * def.scale * shroom_scale)))
            .collect();
        Ok(Self {
            world: World::from_bsp(map)?,
            points,
            cameras,
            paths,
            rock,
            travel,
            shroom_triangles,
            shroom_scale,
        })
    }
    pub fn landing(&self, w: &World) -> Result<Transform> {
        let a = self.points["alice_walk_pos1"];
        let b = self.points["alice_run_pos1"];
        // Authored run corridor, deterministic feet and facing; no random safe-spawn search.
        let mut p = a.translation.lerp(b.translation, 0.8);
        p = w
            .actor_footing(
                p,
                crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
                160.,
            )
            .context("Chase handoff lacks a floor")?;
        ensure!(w.body_clear(p), "Chase handoff blocked");
        Ok(Transform {
            translation: p,
            rotation: b.rotation,
        })
    }
}
const BOUNCE_KEYS: [&str; 56] = [
    "garden3.bounce.0",
    "garden3.bounce.1",
    "garden3.bounce.2",
    "garden3.bounce.3",
    "garden3.bounce.4",
    "garden3.bounce.5",
    "garden3.bounce.6",
    "garden3.bounce.7",
    "garden3.bounce.8",
    "garden3.bounce.9",
    "garden3.bounce.10",
    "garden3.bounce.11",
    "garden3.bounce.12",
    "garden3.bounce.13",
    "garden3.bounce.14",
    "garden3.bounce.15",
    "garden3.bounce.16",
    "garden3.bounce.17",
    "garden3.bounce.18",
    "garden3.bounce.19",
    "garden3.bounce.20",
    "garden3.bounce.21",
    "garden3.bounce.22",
    "garden3.bounce.23",
    "garden3.bounce.24",
    "garden3.bounce.25",
    "garden3.bounce.26",
    "garden3.bounce.27",
    "garden3.bounce.28",
    "garden3.bounce.29",
    "garden3.bounce.30",
    "garden3.bounce.31",
    "garden3.bounce.32",
    "garden3.bounce.33",
    "garden3.bounce.34",
    "garden3.bounce.35",
    "garden3.bounce.36",
    "garden3.bounce.37",
    "garden3.bounce.38",
    "garden3.bounce.39",
    "garden3.bounce.40",
    "garden3.bounce.41",
    "garden3.bounce.42",
    "garden3.bounce.43",
    "garden3.bounce.44",
    "garden3.bounce.45",
    "garden3.bounce.46",
    "garden3.bounce.47",
    "garden3.bounce.48",
    "garden3.bounce.49",
    "garden3.bounce.50",
    "garden3.bounce.51",
    "garden3.bounce.52",
    "garden3.bounce.53",
    "garden3.bounce.54",
    "garden3.bounce.55",
];
impl Garden {
    pub(super) fn scene_camera(&self) -> Option<crate::cinematic::Camera> {
        let s = self.saved.scene.as_ref()?;
        let sp = spec(None);
        let shot = &sp.shots[s.shot];
        let mut c = self.data.cameras[shot.track]
            .camera((s.time - shot.start + shot.offset).min(shot.hold));
        if let Some(at) = self.saved.first_quake {
            let t = self.saved.age - at;
            if t < 2. {
                let v = vec3((t * 97.).sin(), (t * 113.).sin(), (t * 83.).cos()) * 0.4 * (2. - t);
                c.eye += v;
                c.target += v;
            }
        }
        Some(c)
    }
    pub(super) fn alice_pose(&self) -> Transform {
        let t = self.saved.age;
        if t < 19. {
            return self
                .saved
                .scene
                .as_ref()
                .and_then(|s| s.home)
                .unwrap_or(self.data.points["garden3_start1"]);
        }
        let a = self.data.points["alice_walk_pos1"];
        let b = self.data.points["alice_run_pos1"];
        let d = b.translation - a.translation;
        let position = a
            .translation
            .lerp(b.translation, ((t - 23.5) / 1.5).clamp(0., 0.8));
        let feet = self
            .data
            .world
            .actor_footing(
                position,
                crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
                160.,
            )
            .unwrap_or(position);
        Transform {
            translation: feet,
            rotation: if t < 23.5 {
                a.rotation
            } else {
                Quat::from_rotation_z(d.y.atan2(d.x))
            },
        }
    }
    pub(super) fn bug_pose(&self, k: usize) -> Option<(Transform, &'static str, f32, f32)> {
        if self.saved.arrived {
            return None;
        }
        let t = self.saved.age;
        let (n, delay, clip) = match k {
            4 => ("bug_path1", 2., "fly_normal"),
            5 => ("bug_path2", 4., "fly_fast"),
            7 => ("bug_path4", 2., "fly_normal"),
            8 => ("bug_path5", 2.4, "fly_normal"),
            9 => ("bug_path6", 2.9, "fly_normal"),
            10 => ("big_bugpath1", 3.9, "fly_normal"),
            _ => ("", 0., "special_cheering_middle"),
        };
        if (matches!(k, 4 | 5) && t >= 8.) || (matches!(k, 7..=9) && t >= 16.5) {
            return None;
        }
        let p = if k <= 3 {
            self.data.points[&format!("bug_pos{k}")]
        } else if !n.is_empty() && t >= delay {
            self.data.paths[n].sample(t - delay, true)
        } else {
            self.data.points[&if k == 10 {
                "big_bug1".into()
            } else {
                format!("lady_model{k}")
            }]
        };
        Some((
            p,
            clip,
            (t - if k == 2 {
                0.5
            } else if k == 3 {
                0.7
            } else {
                delay
            })
            .max(0.),
            if k == 10 { 3. } else { 1. },
        ))
    }
    pub(super) fn sounds(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        if self.scripted() {
            for k in 1..=10 {
                if let Some((pose, _, time, _)) = self.bug_pose(k) {
                    loops.push(crate::audio::LoopCue {
                        id: BASE + k,
                        clock: Some(time),
                        path: if k <= 3 {
                            "sound/character/ladybug/cheer_loop.wav"
                        } else {
                            "sound/character/ladybug/idle.wav"
                        },
                        origin: pose.translation,
                    });
                }
            }
        }
        for (key, time, origin, cues) in [
            (
                BOUNCE_KEYS[self.saved.rock.callbacks.saturating_sub(1).min(55)],
                self.saved.bounce,
                self.saved.rock.position,
                &[(0., "sound/ambience/special/marble1.wav")][..],
            ),
            (
                "garden3.pillar",
                self.saved.pillar,
                self.data.points["falling_pillar1"].translation,
                &[(0., "sound/ambience/special/quake1.wav")][..],
            ),
            (
                "garden3.first",
                self.saved.first_quake,
                self.data.points["first_quake1"].translation,
                &[(0., "sound/ambience/special/quake1.wav")][..],
            ),
            (
                "garden3.second",
                self.saved.second_quake,
                self.data.points["pillar_quake2"].translation,
                &[(0., "sound/ambience/special/quake1.wav")][..],
            ),
            (
                "garden3.ice",
                self.saved.ice,
                vec3(3328., 184., -3032.),
                &[
                    (0., "sound/ambience/special/quake1.wav"),
                    (0.9, "sound/ambience/background/ice_crack1.wav"),
                    (1.1, "sound/ambience/background/ice_crack2.wav"),
                    (1.2, "sound/ambience/background/ice_crack3.wav"),
                    (1.7, "sound/ambience/special/thronebreak3.wav"),
                    (2.6, "sound/ambience/special/thronebreak1.wav"),
                ][..],
            ),
            (
                "garden3.end",
                self.saved.end,
                vec3(592., 1304., -3060.),
                &[
                    (0.1, "sound/ambience/special/thronebreak2.wav"),
                    (0.3, "sound/ambience/special/thronebreak3.wav"),
                ][..],
            ),
        ] {
            if let Some(t) = time {
                clocks.push(crate::audio::world::Clock {
                    key,
                    time: self.saved.age - t,
                    period: None,
                    origin,
                    cues,
                });
            }
        }
    }
}

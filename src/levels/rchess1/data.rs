use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Rig {
    pub def: Definition,
    pub skeleton: Skeleton,
    pub clips: BTreeMap<String, Animation>,
    pub audio: crate::audio::events::Model,
}
impl Rig {
    pub fn load(a: &mut Assets, name: &str, selected: &[&str]) -> Result<Self> {
        let def = Definition::load(a, &format!("models/{name}.tik"))?;
        let skeleton = Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
        let mut clips = BTreeMap::new();
        for (key, file) in &def.animations {
            if file.ends_with(".ska") && (selected.is_empty() || selected.contains(&key.as_str())) {
                let (clip, _) = Animation::parse_tags(
                    &a.read(&format!("{}/{file}", def.path))?,
                    &skeleton.unskinned_tags(),
                )?;
                clips.insert(key.clone(), clip);
            }
        }
        for key in selected {
            ensure!(clips.contains_key(*key), "Missing {name}/{key}");
        }
        let audio = crate::audio::events::Model::load(a, &format!("models/{name}.tik"))?;
        Ok(Self {
            def,
            skeleton,
            clips,
            audio,
        })
    }
    pub fn duration(&self, clip: &str) -> f32 {
        self.clips[clip].duration()
    }
    pub fn frame(&self, clip: &str) -> f32 {
        self.clips[clip].frame_time
    }
    pub fn tag(
        &self,
        name: &str,
        clip: &str,
        time: f32,
        pose: Transform,
        looping: bool,
    ) -> Transform {
        let index = self
            .skeleton
            .bones
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case(name))
            .expect("checked King scene tag");
        let local = self.clips[clip].sample(time, looping);
        let global = self.skeleton.global_pose(&local);
        Transform {
            translation: pose.point(global[index].translation * self.def.scale),
            rotation: pose.rotation * global[index].rotation,
        }
    }
}

pub(super) fn pose(e: &crate::levels::Entity) -> Transform {
    let rotation = if let Some(v) = e.get("angles").and_then(|s| crate::interaction::vector(s)) {
        Quat::from_rotation_z(v.y.to_radians())
            * Quat::from_rotation_y(v.x.to_radians())
            * Quat::from_rotation_x(v.z.to_radians())
    } else {
        Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        )
    };
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| crate::interaction::vector(s))
            .unwrap_or_default(),
        rotation,
    }
}

pub(super) struct Brush {
    pub name: String,
    pub index: usize,
    pub pose: Transform,
}
pub(super) struct Cast {
    pub name: String,
    pub model: String,
    pub pose: Transform,
}
pub(super) struct Data {
    pub arena: World,
    pub points: BTreeMap<String, Transform>,
    pub brushes: Vec<Brush>,
    pub cast: Vec<Cast>,
    pub rigs: BTreeMap<String, Rig>,
    pub cameras: BTreeMap<String, Spline>,
    pub queener: Spline,
    pub queener_time: f32,
    pub ambushes: Vec<crate::npc::chess_spawns::Ambush>,
    pub pieces: Vec<(usize, crate::chess::Piece)>,
    pub usable_doors: BTreeSet<usize>,
    pub fire_ids: Vec<crate::entity::Id>,
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), pose(e))))
            .collect::<BTreeMap<_, _>>();
        for n in [
            "r_king",
            "king_bridge",
            "king_clip_way1",
            "alice_king_dest1",
            "alice_king_dest2",
            "alice_king_dest3",
            "alice_king_dest4",
            "beheading_node1",
            "queener_way1",
            "t2",
            "get_me1",
            "get_me2",
            "get_me3",
        ] {
            ensure!(points.contains_key(n), "Missing Red King marker {n}");
        }
        let mut arena = World::actor_world(map)?;
        let king = points.get_mut("r_king").unwrap();
        king.translation = arena
            .actor_footing(king.translation, Vec3::Z * 64., vec3(24., 24., 64.), 256.)
            .context("King has no footing")?;
        let mut clips = vec![];
        for e in &map.entities {
            if e.get("targetname").is_some_and(|n| n == "king_clip") {
                let i = e["model"].trim_start_matches('*').parse()?;
                clips.push(Collider::model(
                    map,
                    i,
                    points["king_clip_way1"].translation,
                    Quat::IDENTITY,
                    false,
                )?);
            }
        }
        arena.set_dynamic(clips);
        let brushes = map
            .entities
            .iter()
            .filter(|e| owns_brush(e))
            .filter_map(|e| {
                let i = e.get("model")?.strip_prefix('*')?.parse().ok()?;
                let mut p = pose(e);
                if e.get("classname").is_some_and(|c| c == "func_rotatingdoor") {
                    p.rotation = Quat::IDENTITY;
                }
                Some(Brush {
                    name: e.get("targetname").cloned().unwrap_or_default(),
                    index: i,
                    pose: p,
                })
            })
            .collect();
        let mut cast = vec![];
        for (i, e) in map.entities.iter().enumerate() {
            let n = e.get("targetname").map(String::as_str).unwrap_or("");
            let Some(m) = e
                .get("model")
                .map(|m| m.trim_start_matches("models/").trim_end_matches(".tik"))
            else {
                continue;
            };
            if !m.starts_with('*') && scene_actor(n) && n != "asylum_hatter" {
                cast.push(Cast {
                    name: if n == "peanut_gallary" {
                        format!("gallery{i}")
                    } else {
                        n.to_owned()
                    },
                    model: m.into(),
                    pose: pose(e),
                });
            }
        }
        for (n, m) in [
            ("king_knight1", "c_chess_red_knight"),
            ("king_knight2", "c_chess_red_knight"),
            ("king_rook1", "c_chess_red_rook"),
            ("king_rook2", "c_chess_red_rook"),
        ] {
            cast.push(Cast {
                name: n.into(),
                model: m.into(),
                pose: points[&format!("{n}_dest1")],
            });
        }
        let mut rigs = BTreeMap::new();
        for m in cast
            .iter()
            .map(|c| c.model.as_str())
            .chain(["alice", "c_chess_red_king"])
            .chain(crate::chess::Kind::ALL.iter().map(|k| k.model()))
        {
            if !rigs.contains_key(m) {
                let selected = if m == "alice" {
                    &[
                        "idle_stand",
                        "walk",
                        "run",
                        "idle_shrug",
                        "darkened_lookingglass",
                        "ready",
                        "death_faint",
                    ][..]
                } else {
                    &[]
                };
                rigs.insert(m.to_owned(), Rig::load(a, m, selected)?);
            }
        }
        for tag in ["tag_ball", "tag_barrel"] {
            ensure!(
                rigs["c_chess_red_king"]
                    .skeleton
                    .bones
                    .iter()
                    .any(|b| b.name == tag),
                "Missing King tag {tag}"
            );
        }
        let mut cameras = BTreeMap::new();
        for suffix in [
            "queen_camera1",
            "queen_camera2",
            "queen_camera3",
            "queen_camera4",
            "king_camera1",
            "king_camera2",
            "king_camera3",
            "king_camera4",
            "king_camera5",
            "kmpx1",
            "kmpx2",
            "kmpx3",
            "kmpx4",
            "portal",
            "funhouse",
        ] {
            let n = format!("rchess1_{suffix}");
            cameras.insert(
                suffix.into(),
                Spline::camera_track(Track::load(a, &n)?.controls().collect()),
            );
        }
        let mut path = vec![];
        let mut next = Some("queener_path1");
        let mut seen = BTreeSet::new();
        while let Some(n) = next {
            ensure!(seen.insert(n) && seen.len() < 128, "Cyclic queener path");
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|s| s == n))
                .context("Missing queener path node")?;
            let p = pose(e);
            let speed = e
                .get("speed")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.)
                .clamp(0.01, 100.);
            path.push((p.translation, p.rotation, speed));
            next = e.get("target").map(String::as_str);
        }
        let queener = Spline::new(path, false);
        let end = points["t2"].translation;
        let mut queener_time = 0.1;
        while queener_time < 30.
            && queener
                .sample(queener_time, false)
                .translation
                .distance(end)
                > 0.5
        {
            queener_time += 0.01;
        }
        let ambushes = crate::npc::chess_spawns::ambushes(map, "rchess1")?;
        let ids = ambushes
            .iter()
            .flat_map(|b| b.entities.iter().copied())
            .collect::<BTreeSet<_>>();
        let mut pieces = vec![];
        let footing = World::actor_world(map)?;
        for id in ids {
            let e = &map.entities[id];
            let m = e
                .get("modelname")
                .or(e.get("model"))
                .context("Missing chess model")?
                .trim_start_matches("models/")
                .trim_end_matches(".tik");
            let kind = crate::chess::Kind::from_model(m).context("Unsupported chess ambush")?;
            let p = pose(e);
            let half = kind.half();
            let feet = footing
                .actor_footing(p.translation, Vec3::Z * half.z, half, 256.)
                .unwrap_or(p.translation);
            let mut piece =
                crate::chess::Piece::new(kind, feet, p.rotation.to_euler(EulerRot::ZYX).0, 1., id);
            piece.active = false;
            pieces.push((id, piece));
        }
        let usable_doors = map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.get("classname").is_some_and(|c| c == "func_rotatingdoor")
                    && e.get("spawnflags")
                        .and_then(|s| s.parse::<u32>().ok())
                        .is_some_and(|s| s & 4096 != 0)
            })
            .map(|(i, _)| i)
            .collect();
        let fire_ids = map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                e.get("targetname")
                    .is_some_and(|n| n.ends_with("_fire") || n == "revived_queen_effect")
            })
            .map(|(i, _)| crate::entity::Id(i))
            .collect();
        Ok(Self {
            arena,
            points,
            brushes,
            cast,
            rigs,
            cameras,
            queener,
            queener_time,
            ambushes,
            pieces,
            usable_doors,
            fire_ids,
        })
    }
    pub fn boss(&self) -> &Rig {
        &self.rigs["c_chess_red_king"]
    }
}
impl crate::ant::Timing for Data {
    fn duration(&self, m: &str, c: &str) -> f32 {
        self.rigs[m].duration(c)
    }
    fn frame(&self, m: &str, c: &str) -> f32 {
        self.rigs[m].frame(c)
    }
    fn speed(&self, m: &str, c: &str) -> f32 {
        let r = &self.rigs[m];
        r.clips[c].distance * r.def.scale / r.duration(c)
    }
}

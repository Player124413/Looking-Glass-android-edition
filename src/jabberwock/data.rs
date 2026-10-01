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
    fn load(a: &mut Assets, name: &str, selected: &[&str]) -> Result<Self> {
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
            .expect("checked Jabberwock tag");
        let local = self.clips[clip].sample(time, looping);
        let global = self.skeleton.global_pose(&local);
        Transform {
            translation: pose.point(global[index].translation * self.def.scale),
            rotation: pose.rotation * global[index].rotation,
        }
    }
}
pub(super) struct Brush {
    pub name: String,
    pub index: usize,
    pub pose: Transform,
    pub slide: Vec3,
}
pub(super) struct Data {
    pub eye_altar: Option<crate::entity::Id>,
    pub arena: World,
    pub points: BTreeMap<String, Transform>,
    pub volumes: BTreeMap<String, Collider>,
    pub brushes: Vec<Brush>,
    pub rigs: BTreeMap<String, Rig>,
    pub cameras: BTreeMap<String, Spline>,
    pub paths: BTreeMap<String, Spline>,
    pub waves: Vec<(Vec3, Vec3)>,
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
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp, kind: Kind) -> Result<Self> {
        let eye_altar = map.entities.iter().position(|e| {
            e.get("targetname").is_some_and(|n| n == "eye_altar")
                && e.get("model").is_some_and(|m| m.trim_start_matches("models/") == "altar_eyestaff_eye.tik")
        }).map(crate::entity::Id);
        ensure!(eye_altar.is_some() == (kind == Kind::Lair), "Unexpected Eye altar placement");
        let points = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), pose(e))))
            .collect::<BTreeMap<_, _>>();
        for n in if kind == Kind::Lair {
            &[
                "jabber_pos3",
                "alice_dead_pos1",
                "eye_altar",
                "pickup_eye_trigger",
                "end_trigger",
                "help_me1",
                "help_me2",
            ][..]
        } else {
            &[
                "jabberwock",
                "gdeath",
                "alice_end1",
                "drawbridge",
                "get_me1",
                "get_me2",
                "get_me3",
            ][..]
        } {
            ensure!(points.contains_key(*n), "Missing {} marker {n}", kind.map());
        }
        let mut volumes = BTreeMap::new();
        let mut brushes = Vec::new();
        for e in &map.entities {
            let Some(index) = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let name = e
                .get("targetname")
                .or_else(|| e.get("thread"))
                .cloned()
                .unwrap_or_default();
            let mut p = pose(e);
            if e.get("classname")
                .is_some_and(|s| s == "script_object" || s == "func_door")
                && kind == Kind::Grounds
            {
                let door = e.get("classname").is_some_and(|s| s == "func_door");
                let slide = if door {
                    let axis = p.rotation * Vec3::X;
                    p.rotation = Quat::IDENTITY;
                    let size = map.models[index].max - map.models[index].min;
                    axis * (size.dot(axis.abs()) - 8.).max(0.)
                } else {
                    Vec3::ZERO
                };
                brushes.push(Brush {
                    name: if door {
                        format!("exit-door-{index}")
                    } else {
                        name
                    },
                    index,
                    pose: p,
                    slide,
                });
            } else if !name.is_empty() {
                volumes.insert(
                    name,
                    Collider::model(map, index, p.translation, p.rotation, false)?,
                );
            }
        }
        let mut rigs = BTreeMap::new();
        for (name, clips) in [
            ("c_jabberwock", &[][..]),
            ("c_gryphon", &[][..]),
            ("c_jabberspawn", crate::wildlife::Kind::Jabber.clips()),
            (
                "alice",
                &[
                    "idle_stand",
                    "walk",
                    "run",
                    "pain_knockdown",
                    "sit",
                    "kneel_idle",
                    "kneel_2_weep",
                    "weep_sobbing",
                ][..],
            ),
            ("c_cheshire", &["sit_idle1", "sit_talk1"][..]),
            ("c_gnomeold", &["idle", "walk", "letsgo"][..]),
            ("cardguard_diamond", &["death_1"][..]),
        ] {
            if kind == Kind::Lair
                && matches!(name, "c_cheshire" | "c_gnomeold" | "cardguard_diamond")
            {
                continue;
            }
            rigs.insert(name.into(), Rig::load(a, name, clips)?);
        }
        let boss = &rigs["c_jabberwock"];
        for clip in battle::CLIPS {
            ensure!(boss.clips.contains_key(*clip), "Missing boss clip {clip}");
        }
        for tag in ["tag_breath", "tag_eye_emitter"] {
            ensure!(
                boss.skeleton
                    .bones
                    .iter()
                    .any(|b| b.name.eq_ignore_ascii_case(tag)),
                "Missing boss tag {tag}"
            );
        }
        let mut cameras = BTreeMap::new();
        for n in scene::cameras(kind) {
            cameras.insert(
                (*n).into(),
                Spline::camera_track(Track::load(a, n)?.controls().collect()),
            );
        }
        let mut paths = BTreeMap::new();
        for e in map
            .entities
            .iter()
            .filter(|e| e.get("classname").is_some_and(|s| s == "info_splinepath"))
        {
            let Some(name) = e.get("targetname") else {
                continue;
            };
            let mut next = Some(name.as_str());
            let mut seen = BTreeSet::new();
            let mut values = Vec::new();
            while let Some(n) = next {
                if !seen.insert(n) || seen.len() > 256 {
                    break;
                }
                let Some(node) = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|s| s == n))
                else {
                    break;
                };
                let p = pose(node);
                let speed = node
                    .get("speed")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.)
                    .clamp(0.01, 100.);
                values.push((p.translation, p.rotation, speed));
                next = node.get("target").map(String::as_str);
            }
            if !values.is_empty() {
                paths.insert(name.clone(), Spline::new(values, false));
            }
        }
        let mut waves = Vec::new();
        if kind == Kind::Lair {
            // Match each under-arena spawn to its authored launch volume and destination.
            for e in map.entities.iter().filter(|e| {
                e.get("targetname").is_some_and(|s| s == "t2")
                    && e.get("classname").is_some_and(|s| s == "func_spawn")
            }) {
                let start = pose(e).translation;
                let launch = map
                    .entities
                    .iter()
                    .filter(|e| e.get("classname").is_some_and(|s| s == "trigger_push"))
                    .min_by(|a, b| {
                        pose(a)
                            .translation
                            .distance_squared(start)
                            .total_cmp(&pose(b).translation.distance_squared(start))
                    })
                    .context("Missing wave launcher")?;
                let target = &points[launch.get("target").context("Missing wave landing")?];
                waves.push((start, target.translation));
            }
            ensure!(waves.len() == 3, "Changed Jabberspawn wave count");
        }
        let mut arena = World::actor_world(map)?;
        arena.set_dynamic(
            brushes
                .iter()
                .filter(|b| matches!(b.name.as_str(), "wock_clips" | "jabberjump_clips"))
                .map(|b| Collider::model(map, b.index, b.pose.translation, b.pose.rotation, false))
                .collect::<Result<_>>()?,
        );
        Ok(Self {
            eye_altar,
            arena,
            points,
            volumes,
            brushes,
            rigs,
            cameras,
            paths,
            waves,
        })
    }
    pub fn boss(&self) -> &Rig {
        &self.rigs["c_jabberwock"]
    }
    pub fn path(&self, name: &str, time: f32) -> Transform {
        self.paths
            .get(name)
            .map_or_else(|| self.points[name], |p| p.sample(time, true))
    }
}
impl crate::ant::Timing for Data {
    fn duration(&self, model: &str, clip: &str) -> f32 {
        self.rigs[model].duration(clip)
    }
    fn frame(&self, model: &str, clip: &str) -> f32 {
        self.rigs[model].frame(clip)
    }
    fn speed(&self, model: &str, clip: &str) -> f32 {
        let r = &self.rigs[model];
        r.clips[clip].distance * r.def.scale / r.duration(clip)
    }
}
impl crate::clockwork::Rig for Data {
    fn tag(&self, clip: &str, time: f32, tag: &str) -> Transform {
        self.rigs["c_jabberspawn"].tag(
            tag,
            clip,
            time,
            Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            false,
        )
    }
}

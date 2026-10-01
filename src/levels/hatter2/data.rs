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
            .expect("checked Hatter scene tag");
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
    pub min: Vec3,
    pub max: Vec3,
}
pub(super) struct Data {
    pub arena: World,
    pub points: BTreeMap<String, Transform>,
    pub volumes: BTreeMap<String, Collider>,
    pub brushes: Vec<Brush>,
    pub rigs: BTreeMap<String, Rig>,
    pub cameras: BTreeMap<String, Spline>,
    pub paths: BTreeMap<String, Spline>,
    pub blade: Vec3,
    pub healing: Vec<Vec3>,
    pub watch_key: String,
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
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), pose(e))))
            .collect::<BTreeMap<_, _>>();
        for n in [
            "tower_up",
            "tower_down",
            "end_dest",
            "alice_pos2",
            "alice_pos3",
            "gryphon_actor1",
            "hatter_cat",
            "watch_cat",
            "t85",
            "t86",
        ] {
            ensure!(points.contains_key(n), "Missing Hatter marker {n}");
        }
        let mut brushes = vec![];
        let mut volumes = BTreeMap::new();
        for e in &map.entities {
            let Some(i) = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let name = e
                .get("targetname")
                .or(e.get("thread"))
                .cloned()
                .unwrap_or_default();
            let mut p = pose(e);
            if owns_brush(e) {
                // Inline geometry is authored around its pivot; a door's angle chooses its swing, not its rest pose.
                if name == "end_doors1" {
                    p.rotation = Quat::IDENTITY;
                }
                brushes.push(Brush {
                    name,
                    index: i,
                    pose: p,
                    min: map.models[i].min,
                    max: map.models[i].max,
                });
            } else if !name.is_empty() {
                volumes.insert(
                    name,
                    Collider::model(map, i, p.translation, p.rotation, false)?,
                );
            }
        }
        let mut rigs = BTreeMap::new();
        for (n, clips) in [
            ("c_madhatter", &[][..]),
            ("c_clockwork", crate::clockwork::CLIPS),
            ("c_gryphon", &[][..]),
            ("c_cheshire", &["sit_idle1", "sit_talk1"][..]),
            (
                "alice",
                &[
                    "idle_stand",
                    "run",
                    "walk",
                    "talk_04",
                    "talk_01",
                    "idle_shrug",
                    "idle_shrug_headtilt",
                    "idle_shrug_nodyes",
                    "riding",
                ][..],
            ),
        ] {
            rigs.insert(n.into(), Rig::load(a, n, clips)?);
        }
        for tag in ["tag_finger", "tag_saucer"] {
            ensure!(
                rigs["c_madhatter"]
                    .skeleton
                    .bones
                    .iter()
                    .any(|b| b.name == tag),
                "Missing Hatter tag {tag}"
            );
        }
        ensure!(
            rigs["c_gryphon"]
                .skeleton
                .bones
                .iter()
                .any(|b| b.name == "tag_alice"),
            "Missing rider tag"
        );
        let mut cameras = BTreeMap::new();
        for i in 1..=8 {
            let n = format!("hatter2_path{i}");
            cameras.insert(
                n.clone(),
                Spline::camera_track(Track::load(a, &n)?.controls().collect()),
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

        let blade = map
            .entities
            .iter()
            .find(|e| {
                e.get("classname")
                    .is_some_and(|n| n == "ambient_eyestaff_blade")
            })
            .map(pose)
            .context("Missing blade component")?
            .translation;
        let healing = map
            .entities
            .iter()
            .filter(|e| {
                e.get("targetname")
                    .is_some_and(|n| n.starts_with("help_me"))
            })
            .map(|e| pose(e).translation)
            .collect();
        let watch_key = map
            .entities
            .iter()
            .position(|e| {
                e.get("classname")
                    .is_some_and(|n| n == "Item_WeaponPickup_DeadtimeWatch")
            })
            .map(|i| format!("hatter2:{i}"))
            .context("Missing Watch")?;
        let arena = World::actor_world(map)?;
        // Script-owned actors still receive the engine's initial floor drop.
        // The arena Cat is authored above the clock, unlike the Watch Cat
        // whose origin already rests on its ledge. Bounds match c_cheshire.tik.
        for name in ["hatter_cat", "watch_cat"] {
            let p = points.get_mut(name).unwrap();
            p.translation = arena
                .actor_footing(p.translation, Vec3::Z * 24., vec3(32., 32., 24.), 128.)
                .with_context(|| format!("No Cheshire footing at {name}"))?;
        }
        for name in ["tower_up", "tower_down"] {
            let p = points.get_mut(name).unwrap();
            p.translation = arena
                .actor_footing(p.translation, Vec3::Z * 124., vec3(36., 36., 124.), 256.)
                .with_context(|| format!("No Hatter footing at {name}"))?;
        }
        Ok(Self {
            arena,
            points,
            volumes,
            brushes,
            rigs,
            cameras,
            paths,
            blade,
            healing,
            watch_key,
        })
    }
    pub fn boss(&self) -> &Rig {
        &self.rigs["c_madhatter"]
    }
    pub fn path(&self, name: &str, t: f32) -> Transform {
        self.paths
            .get(name)
            .map_or_else(|| self.points[name], |p| p.sample(t, true))
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
impl crate::clockwork::Rig for Data {
    fn tag(&self, c: &str, t: f32, tag: &str) -> Transform {
        self.rigs["c_clockwork"].tag(
            tag,
            c,
            t,
            Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            false,
        )
    }
}

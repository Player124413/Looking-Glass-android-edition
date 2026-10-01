use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;

pub(super) const ALICE: &[&str] = &[
    "idle_shrug",
    "walk",
    "idle_stand",
    "idle_stand_rocktoes",
    "slide",
    "eat_mushroom",
    "ready",
];
pub(super) const ANT: &[&str] = &[
    "idle",
    "walk",
    "alert_1",
    "alert_2",
    "attack_2_start",
    "attack_3_fire_idle",
];
pub(super) struct Rig {
    pub def: Definition,
    pub skeleton: Skeleton,
    pub clips: BTreeMap<String, Animation>,
    pub audio: crate::audio::events::Model,
    pub sounds: BTreeMap<String, &'static str>,
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
        let mut sounds = BTreeMap::new();
        for path in audio.paths() {
            if a.contains(path) {
                sounds.insert(path.to_owned(), intern(path));
            }
        }
        Ok(Self {
            def,
            skeleton,
            clips,
            audio,
            sounds,
        })
    }
    pub fn duration(&self, clip: &str) -> f32 {
        self.clips[clip].duration()
    }
    pub fn frame(&self, clip: &str) -> f32 {
        self.clips[clip].frame_time
    }
    pub fn tag(&self, name: &str, clip: &str, time: f32, pose: Transform) -> Transform {
        let bone = self
            .skeleton
            .bones
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case(name))
            .expect("validated boss tag");
        let global = self
            .skeleton
            .global_pose(&self.clips[clip].sample(time, false));
        Transform {
            translation: pose.point(global[bone].translation * self.def.scale),
            rotation: pose.rotation * global[bone].rotation,
        }
    }
}
fn intern(path: &str) -> &'static str {
    static N: std::sync::OnceLock<std::sync::Mutex<BTreeMap<String, &'static str>>> =
        std::sync::OnceLock::new();
    *N.get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry(path.into())
        .or_insert_with(|| Box::leak(path.to_owned().into_boxed_str()))
}
pub(super) struct Brush {
    pub name: String,
    pub model: usize,
    pub origin: Vec3,
}
pub(super) struct Data {
    pub points: BTreeMap<String, Transform>,
    pub brushes: Vec<Brush>,
    pub rigs: BTreeMap<String, Rig>,
    pub cameras: BTreeMap<String, Spline>,
    pub ants: Vec<(String, Transform)>,
}
fn pose(e: &super::super::Entity) -> Transform {
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| crate::interaction::vector(s))
            .unwrap_or_default(),
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    }
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let points = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), pose(e))))
            .collect::<BTreeMap<_, _>>();
        for name in [
            "centipede_actor1",
            "centipede2_start1",
            "alice_slide_pos1",
            "alice_pos1",
            "alice_pos2",
            "alice_pos3",
            "cat_actor1",
            "get_me1",
            "get_me2",
            "get_me3",
        ] {
            ensure!(points.contains_key(name), "Missing Centipede marker {name}");
        }
        let brushes = map
            .entities
            .iter()
            .filter(|e| e.get("classname").is_some_and(|s| s == "script_object"))
            .map(|e| {
                Ok(Brush {
                    name: e
                        .get("targetname")
                        .context("Unnamed Centipede brush")?
                        .clone(),
                    model: e
                        .get("model")
                        .and_then(|s| s.strip_prefix('*'))
                        .context("Non-inline Centipede brush")?
                        .parse()?,
                    origin: pose(e).translation,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let ants = map
            .entities
            .iter()
            .filter(|e| e.get("model").is_some_and(|s| s == "c_armyant.tik"))
            .map(|e| (e["targetname"].clone(), pose(e)))
            .collect();
        let mut rigs = BTreeMap::new();
        for (name, clips) in [
            ("c_centipede", &[][..]),
            ("c_larva", crate::burrow::Kind::Larva.clips()),
            ("alice", ALICE),
            ("c_armyant", ANT),
            ("c_cheshire", &["sit_idle1", "sit_talk1"]),
        ] {
            rigs.insert(name.into(), Rig::load(a, name, clips)?);
        }
        let r = &rigs["c_centipede"];
        for tag in [
            "tag_target",
            "tag_mouth",
            "spine_05",
            "spine_03",
            "spine_01",
            "tail_05",
            "tail_07",
            "tag_tongue",
            "tag_mandibles",
        ] {
            ensure!(
                r.skeleton
                    .bones
                    .iter()
                    .any(|b| b.name.eq_ignore_ascii_case(tag)),
                "Missing Centipede tag {tag}"
            );
        }
        let mut cameras = BTreeMap::new();
        for n in [
            "centipede2_path1",
            "centipede2_path2",
            "centipede2_path3",
            "centipede2_jump1a",
            "centipede2_path4",
            "centipede2_path5",
            "centipede2_path6",
            "centipede2_path7",
        ] {
            cameras.insert(
                n.into(),
                Spline::camera_track(Track::load(a, n)?.controls().collect()),
            );
        }
        Ok(Self {
            points,
            brushes,
            rigs,
            cameras,
            ants,
        })
    }
    pub fn boss(&self) -> &Rig {
        &self.rigs["c_centipede"]
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
        let c = &self.rigs[model].clips[clip];
        c.distance / c.duration()
    }
}

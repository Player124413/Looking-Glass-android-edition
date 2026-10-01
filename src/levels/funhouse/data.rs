use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;
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
            .expect("checked Funhouse tag");
        let local = self.clips[clip].sample(time, looping);
        let global = self.skeleton.global_pose(&local);
        Transform {
            translation: pose.point(global[index].translation * self.def.scale),
            rotation: pose.rotation * global[index].rotation,
        }
    }
}

pub(super) struct Data {
    pub inhibited: std::collections::BTreeSet<usize>,
    pub points: BTreeMap<String, Transform>,
    pub rigs: BTreeMap<String, Rig>,
    pub cameras: BTreeMap<String, Spline>,
    pub emitters: Vec<(usize, String, Transform)>,
    pub pushes: Vec<usize>,
}
pub fn pose(e: &super::super::Entity) -> Transform {
    let v = e
        .get("angles")
        .and_then(|s| crate::interaction::vector(s))
        .unwrap_or(vec3(
            0.,
            e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
            0.,
        ));
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
        let points = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), pose(e))))
            .collect::<BTreeMap<_, _>>();
        for n in [
            "funhouse_start1",
            "alice_pos1",
            "alice_pos2",
            "dee_pos1",
            "dum_pos1",
            "dum_pos2",
            "cat_pos1",
            "cat_pos2",
            "tuber_bind",
            "break_plat",
        ] {
            ensure!(points.contains_key(n), "Missing Funhouse marker {n}");
        }
        let mut cameras = BTreeMap::new();
        for n in [
            "funhouse_end1",
            "funhouse_cell2",
            "funhouse_jpath1",
            "funhouse_jpath2",
            "funhouse_pipe1",
            "funhouse_awatch1",
            "funhouse_path1",
            "funhouse_path2",
            "funhouse_path3",
            "funhouse_path4",
            "funhouse_path5",
            "funhouse_path6",
        ] {
            cameras.insert(
                n.into(),
                Spline::camera_track(Track::load(a, n)?.controls().collect()),
            );
        }
        let mut rigs = BTreeMap::new();
        for n in [
            "c_tweedle_dee",
            "c_tweedle_dum",
            "c_tweedle_mini_dee",
            "c_tweedle_mini_dum",
        ] {
            let mut clips = vec![
                "idle",
                "walk",
                "run",
                "ready",
                "knife_out",
                "knife_attack",
                "rattle_out",
                "rattle_attack",
                "russian_split",
                "russian_close",
                "russian_jump",
                "prop_out",
                "take_off",
                "fly_forward02",
                "fall_ready",
                "fall_falling",
                "fall_impact",
                "pain",
                "death1",
                "death_frozen",
            ];
            if !n.contains("mini") {
                clips.extend(["twitch1", "twitch2", "alert02"]);
            }
            rigs.insert(n.into(), Rig::load(a, n, &clips)?);
        }
        rigs.insert(
            "c_cheshire".into(),
            Rig::load(
                a,
                "c_cheshire",
                &["sit_idle1", "sit_talk1", "sit_talk2", "sit_talk3"],
            )?,
        );
        rigs.insert(
            "c_madhatter".into(),
            Rig::load(a, "c_madhatter", &["stand_base"])?,
        );
        rigs.insert(
            "alice".into(),
            Rig::load(
                a,
                "alice",
                &[
                    "idle_stand",
                    "idle_base_01",
                    "idle_base_01_2_base_02",
                    "idle_base_02",
                    "death_falling1",
                ],
            )?,
        );
        let emitters = map
            .entities
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                e.get("targetname")
                    .filter(|n| n.contains("gas") || n.contains("steam"))
                    .map(|n| (i, n.clone(), pose(e)))
            })
            .collect();
        let pushes = CLOSETS
            .iter()
            .map(|n| {
                map.entities
                    .iter()
                    .position(|e| e.get("targetname") == Some(&format!("{n}push")))
                    .unwrap_or(usize::MAX)
            })
            .collect();
        Ok(Self {
            inhibited: map
                .entities
                .iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    let flags = e
                        .get("spawnflags")
                        .and_then(|v| v.parse::<u32>().ok())
                        .unwrap_or(0);
                    (!map.difficulty.allows(flags)).then_some(i)
                })
                .collect(),
            points,
            rigs,
            cameras,
            emitters,
            pushes,
        })
    }
}

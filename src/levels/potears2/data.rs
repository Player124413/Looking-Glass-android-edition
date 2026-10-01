use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;

pub(super) struct Data {
    pub world: World,
    pub points: BTreeMap<String, Vec3>,
    pub tracks: Vec<Spline>,
    clips: BTreeMap<String, (f32, f32, f32)>,
    pub doors: Vec<(usize, Vec3, Vec3)>,
}
impl Data {
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = BTreeMap::new();
        for e in &map.entities {
            if let (Some(name), Some(p)) = (
                e.get("targetname"),
                e.get("origin").and_then(|s| crate::interaction::vector(s)),
            ) {
                points.entry(name.clone()).or_insert(p);
            }
        }
        for name in [
            "bill_endcinematic_start",
            "bill_1",
            "bill_stand",
            "alice_stand",
            "alice_gets_sucked_in",
            "bill_leads_the_way",
            "bill_cam",
            "suck_cam",
        ] {
            ensure!(
                points.contains_key(name),
                "Missing Hollow Hideaway marker {name}"
            );
        }
        let tracks = [
            "potears2_kmxp1",
            "potears2_kmpx2",
            "potears2_kmpx3",
            "potears2_kmpx4",
            "potears2_kmpx5",
        ]
        .iter()
        .map(|n| {
            Ok(Spline::camera_track(
                Track::load(assets, n)?.controls().collect(),
            ))
        })
        .collect::<Result<_>>()?;
        let mut clips = BTreeMap::new();
        for (model, names) in [
            ("c_bill", BILL_CLIPS),
            ("alice", ALICE_CLIPS),
            ("c_armyant", guards::REGULAR),
            ("c_armyantcorp", guards::CORPORAL),
        ] {
            let d = Definition::load(assets, &format!("models/{model}.tik"))?;
            let rig = Skeleton::parse(&assets.read(&format!("{}/{}", d.path, d.model))?)?;
            for name in names {
                let file = d
                    .animations
                    .get(*name)
                    .with_context(|| format!("Missing clip {model}/{name}"))?;
                let a = Animation::parse(
                    &assets.read(&format!("{}/{file}", d.path))?,
                    rig.bones.len(),
                )?;
                clips.insert(
                    format!("{model}/{name}"),
                    (
                        a.duration(),
                        a.frame_time,
                        (a.distance * d.scale / a.duration()).max(1.),
                    ),
                );
            }
        }
        let mut doors = Vec::new();
        for n in 1..=2 {
            let obj = format!("duchessdoor{n}obj");
            let pivot = format!("duchessdoor{n}org");
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname") == Some(&obj))
                .context("Missing house door")?;
            let model = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .context("House door is not a brush")?
                .parse()?;
            doors.push((model, points[&obj], points[&pivot]));
        }
        Ok(Self {
            world: World::from_bsp(map)?,
            points,
            tracks,
            clips,
            doors,
        })
    }
    pub fn duration(&self, model: &str, clip: &str) -> f32 {
        self.clips[&format!("{model}/{clip}")].0
    }
    pub fn frame(&self, model: &str, clip: &str) -> f32 {
        self.clips[&format!("{model}/{clip}")].1
    }
    pub fn speed(&self, model: &str, clip: &str) -> f32 {
        self.clips[&format!("{model}/{clip}")].2
    }
    pub fn travel_time(&self, model: &str, a: &str, b: &str, clip: &str) -> f32 {
        self.points[a].distance(self.points[b]) / self.speed(model, clip)
    }
}

impl crate::ant::Timing for Data {
    fn duration(&self, model: &str, clip: &str) -> f32 {
        self.duration(model, clip)
    }
    fn frame(&self, model: &str, clip: &str) -> f32 {
        self.frame(model, clip)
    }
    fn speed(&self, model: &str, clip: &str) -> f32 {
        self.speed(model, clip)
    }
}

use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;
pub(super) struct Data {
    pub points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Spline>,
    pub world: World,
    clips: BTreeMap<String, (f32, f32, f32)>,
    pub lady: crate::ladybug::Timing,
}
pub(super) fn at(e: &crate::levels::Entity) -> Transform {
    let angles = e
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
        rotation: Quat::from_rotation_z(angles.y.to_radians())
            * Quat::from_rotation_y(angles.x.to_radians())
            * Quat::from_rotation_x(angles.z.to_radians()),
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
            "alice_pos1",
            "alice_pos_squish1",
            "alice_pos_squish2",
            "alice_pos_squish3",
            "rabbit_pos1",
            "rabbit_pos2",
            "rabbit_pos3",
            "rabbit_tiny_pos1",
            "rabbit_tiny_pos2",
            "rabbit_pos_dead",
            "rabbit_actor_dead",
            "hatter_newpos1",
            "hatter_newpos3",
            "hatter_jump_pos1",
            "hatter_jump_pos2",
            "fakeplayer_bridge_pos2",
            "cat_end_pos",
            "cat_squish_pos1",
        ] {
            ensure!(points.contains_key(n), "Missing Herbaceous marker {n}");
        }
        let mut clips = BTreeMap::new();
        for (model, names) in [
            ("alice", ALICE),
            ("c_whiterabbit", RABBIT),
            ("c_madhatter", HATTER),
            ("c_cheshire", CAT),
            ("c_armyant", crate::ant::REGULAR),
            ("c_armyantcorp", crate::ant::CORPORAL),
        ] {
            let d = Definition::load(a, &format!("models/{model}.tik"))?;
            let rig = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
            for n in names {
                let file = d
                    .animations
                    .get(*n)
                    .with_context(|| format!("Missing Herbaceous clip {model}/{n}"))?;
                let c = Animation::parse(&a.read(&format!("{}/{file}", d.path))?, rig.bones.len())?;
                clips.insert(
                    format!("{model}/{n}"),
                    (
                        c.duration(),
                        c.frame_time,
                        (c.distance * d.scale / c.duration()).max(1.),
                    ),
                );
            }
        }
        let mut cameras = BTreeMap::new();
        for n in (1..=8)
            .map(|n| format!("garden2_path{n}"))
            .chain((1..=4).map(|n| format!("garden2_jump{n}")))
            .chain(["garden2_catend".into()])
        {
            cameras.insert(
                n.clone(),
                Spline::camera_track(Track::load(a, &n)?.controls().collect()),
            );
        }
        Ok(Self {
            points,
            cameras,
            world: World::from_bsp(map)?,
            clips,
            lady: crate::ladybug::Timing::load(a)?,
        })
    }
    pub fn duration(&self, m: &str, n: &str) -> f32 {
        self.clips[&format!("{m}/{n}")].0
    }
    pub fn speed(&self, m: &str, n: &str) -> f32 {
        self.clips[&format!("{m}/{n}")].2
    }
    pub fn travel(&self, m: &str, n: &str, a: &str, b: &str) -> f32 {
        self.points[a]
            .translation
            .distance(self.points[b].translation)
            / self.speed(m, n)
    }
    pub fn collapse(&self) -> f32 {
        3.3 + self.duration("c_madhatter", "jump")
    }
    pub fn footing(&self, mut p: Transform, half: Vec3) -> Transform {
        p.translation = self
            .world
            .actor_footing(p.translation, Vec3::Z * half.z, half, 96.)
            .unwrap_or(p.translation);
        p
    }
}
impl crate::ant::Timing for Data {
    fn duration(&self, m: &str, n: &str) -> f32 {
        self.duration(m, n)
    }
    fn frame(&self, m: &str, n: &str) -> f32 {
        self.clips[&format!("{m}/{n}")].1
    }
    fn speed(&self, m: &str, n: &str) -> f32 {
        self.speed(m, n)
    }
}

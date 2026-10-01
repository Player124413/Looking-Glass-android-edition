use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::{BTreeMap, BTreeSet};
pub(super) struct Data {
    pub alice: Transform,
    pub cater: Transform,
    pub cameras: Vec<Spline>,
    pub portal: Spline,
    pub portal_origin: Vec3,
    pub rocks: Vec<crate::falling_rock::Spec>,
    pub marble_cameras: Vec<Spline>,
    pub marble_start: Transform,
    pub marble_end: Transform,
    pub easy: bool,
    pub easy_altar: Collider,
    clips: BTreeMap<String, (f32, f32)>,
}
pub(super) fn at(e: &super::super::Entity) -> Transform {
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
        let point = |n: &str| -> Result<Transform> {
            Ok(at(map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|v| v == n))
                .with_context(|| format!("Missing Garden4 marker {n}"))?))
        };
        let mut alice = point("alice_pos1")?;
        // The script actor settles under gravity. Keep the authored XY/facing,
        // with its feet supported by the same BSP used for gameplay.
        alice.translation = World::from_bsp(map)?
            .actor_footing(
                alice.translation,
                crate::collision::PLAYER_CENTER,
                crate::collision::PLAYER_HALF,
                128.,
            )
            .context("Caterpillar handoff has no supporting floor")?;
        let cater = point("caterpillar_actor1")?;
        let portal_origin = point("portal_object")?.translation;
        let cameras = [
            "garden4_path1",
            "garden4_path2",
            "garden4_newp1",
            "garden4_newp2",
            "garden4_newp3",
            "garden4_path4",
            "garden4_newp4",
            "garden4_portalw1",
        ]
        .into_iter()
        .map(|n| {
            Ok(Spline::camera_track(
                Track::load(a, n)?.controls().collect(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
        let mut nodes = Vec::new();
        let mut seen = BTreeSet::new();
        let mut next = Some("portal_path");
        while let Some(n) = next {
            ensure!(seen.insert(n), "Cyclic portal path");
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|v| v == n))
                .context("Missing portal node")?;
            let p = at(e);
            let speed = e.get("speed").map_or(Ok(1.), |v| v.parse::<f32>())?;
            ensure!(speed.is_finite() && speed > 0., "Invalid portal speed");
            nodes.push((p.translation, p.rotation, speed));
            next = e.get("target").map(String::as_str);
        }
        ensure!(nodes.len() == 6, "Portal path changed");
        let mut clips = BTreeMap::new();
        for (model, names) in [("alice", ALICE), ("c_caterpillar", CATER)] {
            let d = Definition::load(a, &format!("models/{model}.tik"))?;
            let rig = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
            for n in names {
                let f = d
                    .animations
                    .get(*n)
                    .with_context(|| format!("Missing scene animation {model}/{n}"))?;
                let anim = Animation::parse(&a.read(&format!("{}/{f}", d.path))?, rig.bones.len())?;
                clips.insert(format!("{model}/{n}"), (anim.duration(), anim.frame_time));
            }
        }
        Ok(Self {
            alice,
            cater,
            cameras,
            portal: Spline::new(nodes, false),
            portal_origin,
            rocks: course::ROCKS
                .iter()
                .enumerate()
                .map(|(k, n)| {
                    if k == 6 {
                        crate::falling_rock::Spec::load(map, n)
                    } else {
                        crate::falling_rock::Spec::load_ice(a, map, n)
                    }
                })
                .collect::<Result<_>>()?,
            marble_cameras: ["garden4_path3", "garden4_path3x"]
                .iter()
                .map(|n| {
                    Ok(Spline::camera_track(
                        Track::load(a, n)?.controls().collect(),
                    ))
                })
                .collect::<Result<_>>()?,
            marble_start: point("alice_watch_marble1")?,
            marble_end: point("alice_watch_marble2")?,
            easy: map.difficulty == crate::powerups::Difficulty::Easy,
            easy_altar: Collider::box_bounds(
                vec3(-1450., -1098., -4480.),
                vec3(-1386., -1034., -4416.),
            ),
            clips,
        })
    }
    pub fn duration(&self, model: &str, clip: &str) -> f32 {
        self.clips[&format!("{model}/{clip}")].0
    }
    pub fn frame(&self, model: &str, clip: &str) -> f32 {
        self.clips[&format!("{model}/{clip}")].1
    }
}

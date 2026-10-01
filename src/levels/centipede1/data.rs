use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::{BTreeMap, BTreeSet};
pub(super) const BOSS: &[&str] = &["idle_base", "idle_yawn", "idle_snarl"];
pub(super) const ALICE: &[&str] = &["idle_stand", "ready"];
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
pub(super) struct Data {
    pub points: BTreeMap<String, Transform>,
    pub runners: BTreeMap<usize, Transform>,
    pub placed: Vec<Ant>,
    pub braves: Vec<Transform>,
    pub killers: Vec<Transform>,
    pub lid: Collider,
    pub lid_pose: Transform,
    pub lid_model: usize,
    camera: Spline,
    clips: BTreeMap<String, (f32, f32, f32)>,
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let points: BTreeMap<_, _> = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), at(e))))
            .collect();
        for n in [
            "centipede1_start1",
            "cent_posx1",
            "shroom_lid",
            "ant_brave_pos1",
            "ant_brave_pos2",
            "ant_brave_pos3",
        ] {
            ensure!(points.contains_key(n), "Missing Flora marker {n}");
        }
        let lid_entity = &map.entities[34];
        let lid_model = lid_entity["model"].trim_start_matches('*').parse()?;
        let lid_pose = at(lid_entity);
        let lid = Collider::model(
            map,
            lid_model,
            lid_pose.translation,
            lid_pose.rotation,
            true,
        )?;
        let mut world = World::from_bsp(map)?;
        world.set_dynamic(vec![lid.clone()]);
        let mut placed = Vec::new();
        let mut runners = BTreeMap::new();
        for (id, e) in map.entities.iter().enumerate() {
            let flags = e
                .get("spawnflags")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if !map.difficulty.allows(flags)
                || e.get("model")
                    .is_none_or(|m| m.trim_start_matches("models/") != "c_armyant.tik")
            {
                continue;
            }
            let p = at(e);
            let feet = world
                .actor_footing(p.translation, Vec3::Z * 36., Vec3::splat(36.), 256.)
                .with_context(|| format!("No footing for Flora Ant #{id}"))?;
            let mut ant = Ant::new(id, false, feet, p.rotation.to_euler(EulerRot::ZYX).0);
            ant.enabled = flags & 64 == 0;
            if let Some(n) = e
                .get("targetname")
                .and_then(|n| n.strip_prefix("ant_runner"))
            {
                let mut goal = points[&format!("ant_brave_pos{n}")];
                goal.translation = world
                    .actor_footing(goal.translation, Vec3::Z * 36., Vec3::splat(36.), 128.)
                    .context("Runner goal obstructed")?;
                runners.insert(id, goal);
                ant.script_wait = true;
            }
            placed.push(ant);
        }
        let mut clips = BTreeMap::new();
        for (model, names) in [
            ("c_armyant", crate::ant::REGULAR),
            ("c_centipede", BOSS),
            ("alice", ALICE),
        ] {
            let def = Definition::load(a, &format!("models/{model}.tik"))?;
            let rig = Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
            for n in names {
                let file = def
                    .animations
                    .get(*n)
                    .with_context(|| format!("Missing Flora clip {model}/{n}"))?;
                let anim =
                    Animation::parse(&a.read(&format!("{}/{file}", def.path))?, rig.bones.len())?;
                clips.insert(
                    format!("{model}/{n}"),
                    (
                        anim.duration(),
                        anim.frame_time,
                        anim.distance * def.scale / anim.duration(),
                    ),
                );
            }
        }
        let camera = Spline::camera_track(Track::load(a, "centipede1_path1")?.controls().collect());
        Ok(Self {
            braves: chain(map, "brave_spawn1")?,
            killers: chain(map, "brave_killer1")?,
            points,
            runners,
            placed,
            lid,
            lid_pose,
            lid_model,
            camera,
            clips,
        })
    }
    pub fn camera(&self, t: f32) -> crate::cinematic::Camera {
        let p = self.camera.sample(t, false);
        crate::cinematic::Camera::look(p.translation, p.translation + p.rotation * Vec3::X * 100.)
    }
}
impl Timing for Data {
    fn duration(&self, m: &str, c: &str) -> f32 {
        self.clips[&format!("{m}/{c}")].0
    }
    fn frame(&self, m: &str, c: &str) -> f32 {
        self.clips[&format!("{m}/{c}")].1
    }
    fn speed(&self, m: &str, c: &str) -> f32 {
        self.clips[&format!("{m}/{c}")].2
    }
}
fn chain(map: &Bsp, first: &str) -> Result<Vec<Transform>> {
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next = first;
    loop {
        ensure!(
            seen.insert(next) && seen.len() <= 16,
            "Cyclic Flora spawn chain"
        );
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == next))
            .context("Flora spawn link missing")?;
        ensure!(
            e.get("modelname").is_some_and(|m| m == "c_armyant.tik"),
            "Unexpected Flora spawn model"
        );
        result.push(at(e));
        match e.get("target") {
            Some(n) => next = n,
            None => break,
        }
    }
    Ok(result)
}

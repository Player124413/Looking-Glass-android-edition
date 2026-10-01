use super::*;
use crate::{
    ant::Timing,
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;

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
pub(super) struct Pad {
    pub id: usize,
    pub volume: Collider,
    pub velocity: Vec3,
}
pub(super) struct Data {
    pub pads: Vec<Pad>,
    pub lever_duration: f32,
    pub points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Spline>,
    pub volumes: BTreeMap<String, Collider>,
    pub clips: BTreeMap<String, (f32, f32, f32)>,
    pub difficulty: crate::powerups::Difficulty,
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = BTreeMap::new();
        let mut volumes = BTreeMap::new();
        for e in &map.entities {
            if let Some(n) = e.get("targetname") {
                // Duplicate instructor marker: first BSP entity (#293), stable map order.
                points.entry(n.clone()).or_insert_with(|| at(e));
                if let Some(m) = e
                    .get("model")
                    .and_then(|m| m.strip_prefix('*'))
                    .and_then(|m| m.parse().ok())
                {
                    if e.get("classname").is_some_and(|c| c.starts_with("trigger")) {
                        volumes.insert(
                            n.clone(),
                            Collider::model(map, m, at(e).translation, Quat::IDENTITY, false)?,
                        );
                    }
                }
            }
        }
        for (kind, count) in [("bishop", 34), ("knight", 31)] {
            for i in 1..=count {
                for suffix in ["", "_path"] {
                    ensure!(
                        points.contains_key(&format!("{kind}{i}{suffix}")),
                        "Missing chess square {kind}{i}{suffix}"
                    );
                }
            }
        }
        points.insert("bell_lever".into(), at(&map.entities[1185]));
        points.insert("water_lever".into(), at(&map.entities[1210]));
        let e = &map.entities[292];
        volumes.insert(
            "bishop_retry".into(),
            Collider::model(map, 102, at(e).translation, Quat::IDENTITY, false)?,
        );
        let mut pads = Vec::new();
        for (id, e) in map.entities.iter().enumerate() {
            if e.get("classname").map(String::as_str) != Some("trigger_push")
                || e.get("spawnflags")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0)
                    & 8
                    != 0
            {
                continue;
            }
            let origin = at(e).translation;
            if let Some(target) = e.get("target").and_then(|t| points.get(t)) {
                if let Some(velocity) =
                    crate::traversal::launch_velocity(origin, target.translation)
                {
                    let model = e["model"].trim_start_matches('*').parse()?;
                    pads.push(Pad {
                        id,
                        volume: Collider::model(map, model, origin, Quat::IDENTITY, false)?,
                        velocity,
                    });
                }
            }
        }
        let mut cameras = BTreeMap::new();
        for n in [
            "gate_camera1",
            "gate_camera2",
            "bishop_camera1a",
            "bishop_camera1b",
            "bishop_camera1c",
            "bishop_camera2",
            "quad_camera1",
            "knight_camera1",
            "knight_camera2",
            "water_camera1",
            "water_camera2",
        ] {
            cameras.insert(
                n.into(),
                Spline::camera_track(
                    Track::load(a, &format!("wchess1_{n}"))?
                        .controls()
                        .collect(),
                ),
            );
        }
        let mut clips = BTreeMap::new();
        for kind in crate::chess::Kind::ALL {
            Self::clips(a, kind.model(), kind.clips(), &mut clips)?;
        }
        Self::clips(a, "c_chess_red_rook", &["twitchc"], &mut clips)?;
        Self::clips(a, "alice", &["idle", "walk", "run"], &mut clips)?;
        let lever = crate::tan::Model::parse(&a.read("models/lever1/pull.tan")?)?;
        let lever_duration = lever.frame_time * lever.surfaces[0].frames.len() as f32;
        Ok(Self {
            lever_duration,
            pads,
            points,
            cameras,
            volumes,
            clips,
            difficulty: map.difficulty,
        })
    }
    fn clips(
        a: &mut Assets,
        model: &str,
        names: &[&str],
        into: &mut BTreeMap<String, (f32, f32, f32)>,
    ) -> Result<()> {
        let d = Definition::load(a, &format!("models/{model}.tik"))?;
        let rig = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
        for n in names {
            let file = d
                .animations
                .get(*n)
                .with_context(|| format!("Missing chess clip {model}/{n}"))?;
            let anim = Animation::parse(&a.read(&format!("{}/{file}", d.path))?, rig.bones.len())?;
            into.insert(
                format!("{model}/{n}"),
                (
                    anim.duration(),
                    anim.frame_time,
                    anim.distance * d.scale / anim.duration(),
                ),
            );
        }
        Ok(())
    }
    pub fn point(&self, n: &str) -> Transform {
        self.points[n]
    }
    pub fn square(&self, piece: Piece, number: u8) -> Vec3 {
        self.point(&format!("{}{number}_path", piece.name()))
            .translation
    }
    pub fn camera(&self, n: &str, t: f32) -> crate::cinematic::Camera {
        let p = self.cameras[n].sample(t.max(0.), false);
        crate::cinematic::Camera::look(p.translation, p.translation + p.rotation * Vec3::X)
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

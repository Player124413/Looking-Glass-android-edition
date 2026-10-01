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
    pub portal: Spline,
    pub pawn: Spline,
    pub pawn_duration: f32,
    pub points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Spline>,
    pub volumes: BTreeMap<String, Collider>,
    pub clips: BTreeMap<String, (f32, f32, f32)>,
    pub bound: Vec<(String, Transform)>,
    pub threads: Vec<String>,
    pub king_floor: Vec<f32>,
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
        let pads = Vec::new();
        let mut cameras = BTreeMap::new();
        for n in [
            "queen_camera1",
            "queen_camera2",
            "king_camera1",
            "king_camera2",
            "king_camera3",
            "king_camera4",
            "grab",
            "grab2",
            "king_talk",
            "alice_talk",
        ] {
            cameras.insert(
                n.into(),
                Spline::camera_track(
                    Track::load(a, &format!("wchess2_{n}"))?
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
        Self::clips(a, "alice", ALICE, &mut clips)?;
        Self::clips(a, "c_chess_king", KING, &mut clips)?;
        Self::clips(a, "c_chess_queen", &["idle_base", "dragging"], &mut clips)?;
        let portal = path(map, "exit_portal_path1", true)?;
        let pawn = path(map, "kings_pawn_path1", false)?;
        let mut pawn_duration = 0.;
        while pawn.0.parameter(pawn_duration) < pawn.1 as f64 - 2. && pawn_duration < 120. {
            pawn_duration += 0.001;
        }
        let world = World::from_bsp(map)?;
        let from = points["w_king"].translation;
        let to = points["w_king_dest1"].translation;
        let king_floor = (0..=120)
            .map(|i| {
                let at = from.lerp(to, i as f32 / 120.);
                world
                    .actor_footing(at + Vec3::Z * 64., Vec3::Z * 64., vec3(24., 24., 64.), 128.)
                    .map(|p| p.z)
                    .context("King approach has no floor")
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            king_floor,
            portal: portal.0,
            pawn: pawn.0,
            pawn_duration,
            pads,
            points,
            cameras,
            volumes,
            clips,
            bound: map
                .entities
                .iter()
                .filter(|e| {
                    e.get("targetname")
                        .is_some_and(|n| n == "w_queen" || n == "r_rook_abductors")
                })
                .map(|e| (e["targetname"].clone(), at(e)))
                .collect(),
            threads: map
                .entities
                .iter()
                .filter_map(|e| e.get("thread").cloned())
                .collect(),
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
    pub fn camera(&self, n: &str, t: f32) -> crate::cinematic::Camera {
        self.cameras[n].camera(t.max(0.))
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

fn path(map: &Bsp, start: &str, looping: bool) -> Result<(Spline, usize)> {
    let mut seen = BTreeSet::new();
    let mut nodes = Vec::new();
    let mut next = Some(start);
    while let Some(n) = next {
        if !seen.insert(n) {
            ensure!(looping && n == start, "Invalid spline cycle");
            break;
        }
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|v| v == n))
            .context("Missing Castling spline node")?;
        let mut p = at(e);
        if let Some(v) = e.get("angles").and_then(|s| crate::interaction::vector(s)) {
            p.rotation = Quat::from_rotation_z(v.y.to_radians())
                * Quat::from_rotation_y(v.x.to_radians())
                * Quat::from_rotation_x(v.z.to_radians());
        }
        let speed = e.get("speed").map_or(Ok(1.), |v| v.parse::<f32>())?;
        ensure!(speed.is_finite() && speed > 0., "Invalid spline speed");
        nodes.push((p.translation, p.rotation, speed));
        next = e.get("target").map(String::as_str);
    }
    let count = nodes.len();
    ensure!(count >= 4, "Short Castling path");
    Ok((Spline::new(nodes, looping), count))
}

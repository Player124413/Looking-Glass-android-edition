use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;
pub(super) const ALICE: &[&str] = &[
    "idle_stand",
    "idle_shrug",
    "idle_shrug_headtilt",
    "idle_stand_rocktoes",
];
pub(super) const CAT: &[&str] = &[
    "sit_idle1",
    "sit_talk1",
    "sit_talk2",
    "sit_talk3",
    "sit_smile_open",
];
pub(super) const HUMPTY: &[&str] = &[
    "idle_hand",
    "idle_hand_ash",
    "idle_hand_poke",
    "idle_hand_smokein",
    "idle_hand_smokeout",
    "cigar_2_mouth",
    "idle_mouth",
    "idle_mouth_strech",
    "cigar_2_hand",
];
pub(super) struct Data {
    points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Spline>,
    clips: BTreeMap<String, f32>,
    pub wall_center: Vec3,
    pub wall_half: Vec3,
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = BTreeMap::new();
        let world = World::from_bsp(map)?;
        for n in [
            "first_eyestaff",
            "alice_eyestaff_pos1",
            "cat_eyestaff_pos1",
            "cat_pos_start_first",
            "alice_pos1",
            "cat_chess_pos",
            "alice_chess_pos",
            "alice_wall_posx1",
            "humpty_actor1",
            "blunder_cat",
            "get_me1",
            "get_me2",
            "splodeme",
            "splodeme2",
            "caterpillar_actor1",
            "alice_chess_look",
            "alicelookwall",
        ] {
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|s| s == n))
                .with_context(|| format!("Missing WForest marker {n}"))?;
            let mut p = e
                .get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .context("Invalid WForest marker")?;
            if n.starts_with("alice_") && n != "alice_chess_look" {
                p = world
                    .actor_footing(p, PLAYER_CENTER, PLAYER_HALF, 256.)
                    .with_context(|| format!("WForest scene handoff unsupported: {n}"))?;
            }
            if matches!(
                n,
                "cat_pos_start_first" | "cat_eyestaff_pos1" | "cat_chess_pos" | "blunder_cat"
            ) {
                p = world
                    .actor_footing(p, Vec3::Z * 24., vec3(12., 12., 24.), 512.)
                    .with_context(|| format!("Unsupported WForest Cat placement: {n}"))?;
            }
            points.insert(
                n.into(),
                Transform {
                    translation: p,
                    rotation: Quat::from_rotation_z(
                        e.get("angle")
                            .and_then(|s| s.parse::<f32>().ok())
                            .unwrap_or(0.)
                            .to_radians(),
                    ),
                },
            );
        }
        let mut cameras = BTreeMap::new();
        for n in [
            "wforest_catp1",
            "wforest_catp2",
            "wforest_path1",
            "wforest_path2",
            "wforest_walice1",
            "wforest_path3",
            "wforest_gatecam1",
            "wforest_gatecam1x",
            "wforest_jdm2",
        ] {
            cameras.insert(
                n.into(),
                Spline::camera_track(Track::load(a, n)?.controls().collect()),
            );
        }
        let mut clips = BTreeMap::new();
        for (model, names) in [
            ("alice", ALICE),
            ("c_cheshire", CAT),
            ("c_humptydumpty", HUMPTY),
        ] {
            let d = Definition::load(a, &format!("models/{model}.tik"))?;
            let skeleton = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
            for n in names {
                let path = d
                    .animations
                    .get(*n)
                    .with_context(|| format!("Missing animation {model}/{n}"))?;
                clips.insert(
                    format!("{model}/{n}"),
                    Animation::parse(
                        &a.read(&format!("{}/{path}", d.path))?,
                        skeleton.bones.len(),
                    )?
                    .duration(),
                );
            }
        }
        let wall = &map.models[36];
        let origin = vec3(6496., 5424., 1004.);
        Ok(Self {
            points,
            cameras,
            clips,
            wall_center: origin + (wall.min + wall.max) * 0.5,
            wall_half: (wall.max - wall.min) * 0.5,
        })
    }
    pub fn at(&self, name: &str) -> Transform {
        self.points[name]
    }
    pub fn duration(&self, model: &str, clip: &str) -> f32 {
        self.clips[&format!("{model}/{clip}")]
    }
}

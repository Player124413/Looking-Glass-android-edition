use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::BTreeMap;
pub(super) const ALICE: &[&str] = &[
    "idle_stand",
    "idle_base_02",
    "idle_base_02_kneel",
    "kneel_idle",
    "kneel_shakeno",
    "kneel_2_weep",
    "weep_loop",
];
pub(super) const CAT: &[&str] = &[
    "sit_idle1",
    "sit_idle2",
    "sit_talk1",
    "sit_talk3",
    "sit_smile_open",
    "sit_smile_shut",
    "walk",
    "death",
    "death_idle",
    "death_head",
];
pub(super) struct Data {
    pub points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Spline>,
    clips: BTreeMap<String, f32>,
    pub spawns: Vec<(usize, String, guards::Body)>,
    pub sever: crate::dismember::Recipe,
    pub cards: Vec<crate::npc::CardRig>,
    pub guard: crate::combat::Timing,
    pub diamond: crate::combat::Timing,
}
pub(super) fn at(e: &super::super::Entity) -> Transform {
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| interaction::vector(s))
            .unwrap_or(Vec3::ZERO),
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    }
}
pub(super) fn answer(n: &str) -> Option<(usize, bool)> {
    match n {
        "Tweedle_Club_Win" => Some((0, true)),
        "Hatter_Club_Lose" | "Jabber_Club_Lose" => Some((0, false)),
        "Jabber_Diamond_Win" => Some((1, true)),
        "Hatter_Diamond_Lose" | "Tweedle_Diamond_Lose" => Some((1, false)),
        "Hatter_Spade_Win" => Some((2, true)),
        "Jabber_Spade_Lose" | "Tweedle_Spade_Lose" => Some((2, false)),
        _ => None,
    }
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let mut points = BTreeMap::new();
        let world = World::from_bsp(map)?;
        let actor_world = World::actor_world(map)?;
        for name in [
            "black_sky",
            "grounds_sky",
            "lift_cat",
            "cat_lift_pos1",
            "lift_cat_goaway",
            "lever_cat_pos1",
            "alice_levercat_pos",
            "alice_pos1",
            "cat_spawn1",
            "cat_pos1",
            "cat_pace1",
            "cat_pace2",
            "cat_popup",
            "mirror_lever",
            "t9",
        ] {
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|s| s == name))
                .with_context(|| format!("Missing Keep marker {name}"))?;
            let mut p = at(e);
            if matches!(name, "alice_pos1" | "alice_levercat_pos" | "t9") {
                p.translation = world
                    .actor_footing(p.translation, PLAYER_CENTER, PLAYER_HALF, 256.)
                    .with_context(|| format!("Unsupported Keep marker {name}"))?;
            }
            if name == "lift_cat" {
                // The placed actor settles onto the landing; path-node heights
                // describe navigation, not the seated model's support plane.
                p.translation = actor_world
                    .actor_footing(p.translation, Vec3::Z * 24., vec3(32., 32., 24.), 128.)
                    .context("Unsupported Keep arrival cat")?;
            }
            points.insert(name.into(), p);
        }
        let mut cameras = BTreeMap::new();
        for n in [
            "keep_path1",
            "keep_path2",
            "keep_heartdoor1",
            "keep_introp1",
            "keep_leverpx1",
            "keep_nop1",
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
            (
                "cardguard_diamond",
                &["alert1", "stand_attack", "pain1", "death_1"][..],
            ),
        ] {
            let d = Definition::load(a, &format!("models/{model}.tik"))?;
            let sk = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
            for n in names {
                let path = d
                    .animations
                    .get(*n)
                    .with_context(|| format!("Missing {model}/{n}"))?;
                clips.insert(
                    format!("{model}/{n}"),
                    Animation::parse(&a.read(&format!("{}/{path}", d.path))?, sk.bones.len())?
                        .duration(),
                );
            }
        }
        let guard = crate::npc::guard_timing(a)?;
        let diamond = crate::combat::Timing {
            sever: None,
            alert: clips["cardguard_diamond/alert1"],
            attack: clips["cardguard_diamond/stand_attack"],
            hit: 0.35,
            pain: clips["cardguard_diamond/pain1"],
            death: clips["cardguard_diamond/death_1"],
        };
        let mut spawns = vec![];
        for (id, e) in map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.get("classname").is_some_and(|s| s == "func_spawn"))
        {
            let p = at(e);
            let group = e
                .get("spawntargetname")
                .context("Missing Keep spawn group")?
                .trim_end_matches("_guards");
            let guard = if group == "diamond" {
                Guard::diamond(p.translation, p.rotation.to_euler(EulerRot::ZYX).0, 1.)
            } else {
                Guard::new(p.translation, p.rotation.to_euler(EulerRot::ZYX).0, 1.)
            };
            let guard =
                if let Some(kind) = crate::cards::Kind::from_model(&format!("cardguard_{group}")) {
                    guards::Body::Card(crate::cards::Guard::new(
                        kind,
                        p.translation,
                        p.rotation.to_euler(EulerRot::ZYX).0,
                        1.,
                        id,
                    ))
                } else {
                    guards::Body::Basic(guard)
                };
            spawns.push((id, group.into(), guard));
        }
        ensure!(spawns.len() == 17, "Keep spawn identities changed");
        Ok(Self {
            points,
            cameras,
            clips,
            spawns,
            guard,
            diamond,
            sever: crate::dismember::actor_recipe(a, "c_cheshire", "death")?,
            cards: vec![
                crate::npc::CardRig::load(a, crate::cards::Kind::Heart)?,
                crate::npc::CardRig::load(a, crate::cards::Kind::Spade)?,
            ],
        })
    }
    pub fn at(&self, n: &str) -> Transform {
        self.points[n]
    }
    pub fn duration(&self, model: &str, n: &str) -> f32 {
        self.clips[&format!("{model}/{n}")]
    }
}

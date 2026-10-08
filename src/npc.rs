//! Placed, animated characters with bounded local reactions. No original AI/scripts run.
mod ambient;
pub(crate) mod attack_fx;
mod garden;
pub(crate) fn garden_check(a:&mut Assets)->Result<()> { garden::check(a) }
pub mod ant_check;
pub mod electric_check;
pub mod opening_check;
mod burrow_art;
pub mod burrow_check;
mod card_art;
mod card_check;
pub mod chess_check;
pub(crate) mod chess_spawns;
mod clock_art;
pub mod clock_check;
mod clock_spawns;
mod companions;
pub(crate) fn companion_check(a: &mut Assets) -> Result<()> { companions::check(a) }
pub mod imp_check;
mod imp_spawns;
mod magma_art;
mod magma_check;
mod route_threats;
mod resident;
pub mod resident_check;
mod resident_spawns;
mod snark_art;
mod snark_check;
mod wildlife_art;
pub mod wildlife_check;
pub fn chess_thread_supported(map: &str, thread: &str) -> bool {
    chess_spawns::thread_target(map, thread).is_some()
}
pub fn creature_thread_supported(map: &str, thread: &str) -> bool {
    matches!(
        (map, thread.trim_end_matches("()")),
        (
            "hatter1",
            "chairspiders"
                | "spider_wall12"
                | "spider_wall67"
                | "spider_wall5"
                | "spider_wall8"
                | "end_spider"
        ) | ("hedge1", "SeekStart" | "SeekEnd")
    )
}
use crate::{
    assets::Assets,
    bsp::{self, Bsp},
    collision::World,
    combat::{self, Guard, Timing},
    environment::Atmosphere,
    interaction::vector,
    skeletal::{Animation, Definition, Skeleton, Transform},
    texture,
    weapons::Prop,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

type Entity = BTreeMap<String, String>;
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Spawn {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clock_spawn: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resident_spawn: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    imp_spawn: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chess_spawn: Option<usize>,
    #[serde(default)]
    difficulty_variant: bool,
    name: String,
    hidden: bool,
    model: String,
    origin: Vec3,
    yaw: f32,
    scale: f32,
    animation: Option<String>,
}
impl Spawn {
    fn saved_identity(&self, saved: &Self, maze: bool) -> bool {
        if self == saved { return true; }
        // Sealed-room precache actors were active in older Hedge1 and Tower1 saves.
        // Accept only that one reviewed identity change; live cast identities stay exact.
        if maze && self.hidden && !saved.hidden && self.origin.y > 6000. && self.origin.z < 1200. {
            let mut old = saved.clone();
            old.hidden = true;
            return self == &old;
        }
        false
    }
}
fn number(e: &Entity, key: &str, default: f32) -> f32 {
    e.get(key)
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}
fn model_name(s: &str) -> Option<String> {
    let s = s
        .strip_prefix("models/")
        .unwrap_or(s)
        .strip_suffix(".tik")?;
    (!s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c)))
    .then(|| s.to_lowercase())
}
fn placements(map: &Bsp, name: &str, entry: Option<&str>, preview: bool) -> Vec<Spawn> {
    if preview && name == "skool1" && entry.is_none() {
        return [
            ("c_cheshire", -1940., 1880.),
            ("c_gnomeold", -1836., 1880.),
            ("cardguard_club", -1940., 2000.),
            ("c_insanechild_muzzle", -1836., 2000.),
        ]
        .into_iter()
        .map(|(model, x, y)| Spawn {
            resident_spawn: None,
            clock_spawn: None,
            imp_spawn: None,
            chess_spawn: None,
            difficulty_variant: false,
            name: String::new(),
            hidden: false,
            model: model.into(),
            origin: vec3(x, y, -504.),
            yaw: -std::f32::consts::FRAC_PI_2,
            scale: 1.,
            animation: None,
        })
        .collect();
    }
    map.entities
        .iter()
        .enumerate()
        .filter_map(|(id, e)| {
            if name == "hedge2" && (id == 1 || (746..=752).contains(&id)) { return None; }
            let class = e.get("classname")?.to_lowercase();
            if !(class.starts_with("characters_") || class.starts_with("enemies_")) {
                return None;
            }
            // Fortress combat is owned by Encounters; the airship cast is offstage.
            if matches!(name, "fortress1" | "fortress2") && class != "characters_cheshirecat" {
                return None;
            }
            let flags = number(e, "spawnflags", 0.) as u32;
            if !map.difficulty.allows(flags) || flags & !0x700 != 0 {
                return None;
            }
            let model = model_name(&e.get("model")?.to_lowercase())?;
            let target = e.get("targetname").map(String::as_str).unwrap_or("");
            if (name == "skool1" && class.starts_with("enemies_"))
                || (name == "gvillage" && target == "guard_rabbit")
            {
                return None;
            }
            if name == "skool2"
                && (model == "c_boojum"
                    || [
                        "old_gnome_1",
                        "old_gnome_2",
                        "gnome_killer1",
                        "gnome_killer2",
                    ]
                    .contains(&target))
            {
                return None;
            }
            let mut origin = vector(e.get("origin")?)?;
            let mut scale = number(e, "scale", 1.);
            let hidden = (name == "hedge1" && origin.y > 6000. && origin.z < 1200.)
                || (name == "tower1" && id == 410)
                || matches!(name, "fortress1" | "fortress2")
                || (name == "skool1" && ["book_cat", "shelf_cat", "mallet_cat"].contains(&target))
                || (name == "skool2" && ["mallet_cat", "dice_cat"].contains(&target))
                || (name == "gvillage"
                    && ["cat_hole1", "cat_shrink1", "climb_cat", "exit_cat"].contains(&target));
            if name == "fortress1" && target == "skool_cat" {
                origin = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|s| s == "skool_cat_node"))
                    .and_then(|e| e.get("origin"))
                    .and_then(|s| vector(s))
                    .unwrap_or(origin);
            }
            if name == "skool1" {
                let returning = entry == Some("skool1_start2");
                if [
                    "cat_fluff",
                    "gnome_shelf",
                    "tiny_gnome1",
                    "insane_actor1",
                    "insane_actor2",
                    "insane_actor3",
                ]
                .contains(&target)
                {
                    return None;
                }
                if returning
                    && (target.starts_with("library_guard")
                        || [
                            "return_insane1",
                            "return_insane2",
                            "cgd_top1",
                            "talk_gnome1",
                        ]
                        .contains(&target))
                {
                    return None;
                }
                if target == "talk_gnome1" {
                    origin = map
                        .entities
                        .iter()
                        .find(|e| e.get("targetname").is_some_and(|n| n == "gnome_start"))
                        .and_then(|e| e.get("origin"))
                        .and_then(|s| vector(s))
                        .unwrap_or(origin);
                    scale = 1.;
                }
            }
            if name == "skool2"
                && [
                    "rage_cat",
                    "dice_boojum",
                    "old_gnome_2",
                    "gnome_killer1",
                    "gnome_killer2",
                ]
                .contains(&target)
            {
                return None;
            }
            if name == "gvillage" && target == "knife_cat" {
                return None;
            }
            if name == "gvillage" && target == "cat_hole1" {
                origin = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|n| n == "cat_start_pos1"))
                    .and_then(|e| e.get("origin"))
                    .and_then(|s| vector(s))
                    .unwrap_or(origin);
                scale = 1.;
            }
            if !(0.01..=10.).contains(&scale) {
                return None;
            }
            Some(Spawn {
                resident_spawn: None,
                clock_spawn: None,
                imp_spawn: None,
                chess_spawn: None,
                difficulty_variant: flags & 0x700 != 0,
                name: target.into(),
                hidden,
                model,
                origin,
                yaw: number(e, "angle", 0.).to_radians(),
                scale,
                animation: e.get("anim").cloned(),
            })
        })
        .chain(chess_spawns::placements(map, name))
        .chain(imp_spawns::placements(map, name))
        .chain(clock_spawns::placements(map, name))
        .chain(resident_spawns::placements(map, name))
        .chain(garden::extras(map, name))
        .take(256)
        .collect()
}

#[derive(Default)]
struct Metadata {
    hidden: BTreeSet<String>,
    attachments: Vec<(String, String, f32)>,
    friendly: bool,
    name: String,
    bounds: Option<(Vec3, Vec3)>,
    airborne: bool,
}
/// Read only the init/server declaration, excluding comments, client effects and animation events.
fn metadata(text: &str) -> Result<Metadata> {
    let tokens = bsp::tokens(text)?;
    let mut result = Metadata::default();
    let Some(start) = tokens.iter().position(|t| t.eq_ignore_ascii_case("init")) else {
        return Ok(result);
    };
    let mut depth = 0;
    let mut server = false;
    for i in start + 1..tokens.len() {
        match tokens[i].as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                if depth == 1 {
                    server = false;
                }
            }
            _ if depth == 1 && tokens[i].eq_ignore_ascii_case("server") => server = true,
            _ if depth == 2 && server => {
                let args = &tokens[i..];
                match args[0].to_lowercase().as_str() {
                    "fly" | "swim" => result.airborne = true,
                    "gravity" if args.get(1).is_some_and(|v| v.parse::<f32>() == Ok(0.)) => {
                        result.airborne = true;
                    }
                    "setsize" if args.len() >= 3 => {
                        if let (Some(min), Some(max)) = (vector(&args[1]), vector(&args[2])) {
                            if min.cmplt(max).all() && (max - min).max_element() < 4096. {
                                result.bounds = Some((min, max));
                            }
                        }
                    }
                    "friend" | "civilian" => result.friendly = true,
                    "name" if args.len() >= 2 => result.name = args[1].clone(),
                    "surface" if args.len() >= 3 && args[2] == "+nodraw" => {
                        result.hidden.insert(args[1].to_lowercase());
                    }
                    "attachmodel" if args.len() >= 4 => {
                        if let (Some(model), Ok(scale)) =
                            (model_name(&args[1]), args[3].parse::<f32>())
                        {
                            if (0.01..=10.).contains(&scale) && result.attachments.len() < 8 {
                                result.attachments.push((model, args[2].clone(), scale));
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
    Ok(result)
}
struct Data {
    card_cut: Option<crate::dismember::Recipe>,
    card_muzzle: Option<(usize, Vec3)>,
    ladybug_timing: Option<crate::ladybug::Timing>,
    events: crate::animation_events::Model,
    def: Definition,
    meta: Metadata,
    skeleton: Skeleton,
    clips: BTreeMap<String, Animation>,
    idle: String,
    greeting: String,
    idle_variants: Vec<String>,
    talk_variants: Vec<String>,
    friendly: bool,
    combat: Option<Timing>,
    /// Attachment tag bones whose stored rotations were renormalised on load.
    off_norm_tags: BTreeSet<String>,
}
/// Models whose animations store non-unit packed rotations on unskinned
/// attachment tag bones: `(model, tag bones, reason)`. The tag list names every
/// bone observed across the model's clips; which of them a run meets depends on
/// the clips it loads. `check` fails for any other model that needs the
/// tolerance, for a tag missing from its entry, and for an entry that no loaded
/// clip needs any more, so this list cannot drift from the data. Skinned bones
/// are never listed.
const OFF_NORM_TAGS: &[(&str, &[&str], &str)] = &[(
    "c_clockwork",
    &["tag_left_hand", "tag_right_hand"],
    "authored hand tags are stored at a reduced length (squared length about 0.60 to 0.75); they skin nothing and are renormalised on load",
)];
impl Data {
    fn load(assets: &mut Assets, name: &str, extra: &[String]) -> Result<Self> {
        let path = format!("models/{name}.tik");
        let def = Definition::load(assets, &path)?;
        ensure!(!def.model.is_empty(), "NPC requires a skeleton: {name}");
        let meta = metadata(&String::from_utf8_lossy(&assets.read(&path)?))?;
        let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
        let idle = [
            "idle",
            "idle1",
            "idle01",
            "stand_idle1",
            "sit_idle1",
            "fly",
            "stand",
            "stand_base",
            "ready",
            "insidethrone",
        ]
        .into_iter()
        .find(|n| def.animations.get(*n).is_some_and(|n| n.ends_with(".ska")))
        .map(str::to_owned)
        .or_else(|| {
            def.animations
                .iter()
                .find(|(n, f)| n.starts_with("idle") && f.ends_with(".ska"))
                .map(|(n, _)| n.clone())
        })
        .with_context(|| format!("NPC {name} has no supported idle clip"))?;
        let greeting = [
            "talk",
            "talk1",
            "talk01",
            "sit_talk1",
            "stand_talk1",
            "idle2",
            "idle02",
            "stand_idle2",
        ]
        .into_iter()
        .find(|n| def.animations.contains_key(*n))
        .unwrap_or(&idle)
        .to_owned();
        let mut names: BTreeSet<String> = extra
            .iter()
            .filter(|n| def.animations.contains_key(*n))
            .cloned()
            .collect();
        names.extend([idle.clone(), greeting.clone()]);
        let mut idle_variants = family(&def, &idle);
        if name.starts_with("c_torchgnome") {
            idle_variants.extend(
                ["twitch1", "twitch2"]
                    .into_iter()
                    .filter(|n| def.animations.contains_key(*n))
                    .map(str::to_owned),
            );
        }
        let talk_variants = family(&def, &greeting);
        names.extend(idle_variants.iter().chain(&talk_variants).cloned());
        names.extend(
            ["talkbegin", "talkend"]
                .into_iter()
                .filter(|n| def.animations.contains_key(*n))
                .map(str::to_owned),
        );
        if crate::ant::is_ant(name) {
            names.extend(
                if name == "c_armyantcorp" {
                    crate::ant::CORPORAL
                } else {
                    crate::ant::REGULAR
                }
                .iter()
                .map(|s| s.to_string()),
            );
        }
        if name == "cardguard_diamond" {
            names.extend(resident::DIAMOND_CLIPS.iter().map(|s| s.to_string()));
        }
        if let Some(kind) = crate::cards::Kind::from_model(name) {
            names.extend(kind.clips().iter().map(|s| s.to_string()));
        }
        if let Some(kind) = crate::wildlife::Kind::from_model(name) {
            names.extend(kind.clips().iter().map(|s| s.to_string()));
        }
        if name == crate::magma::MODEL {
            names.extend(crate::magma::CLIPS.iter().map(|s| s.to_string()));
        }
        if let Some(kind) = crate::burrow::Kind::from_model(name) {
            names.extend(kind.clips().iter().map(|s| s.to_string()));
        }
        if let Some(kind) = crate::snark::Kind::from_model(name) {
            names.extend(kind.clips().iter().map(|s| s.to_string()));
            ensure!(
                skeleton.bones.iter().any(|b| b.name == "tag_mouth"),
                "Snark mouth missing"
            );
        }
        if let Some(kind) = crate::plants::Kind::from_model(name) {
            names.extend(kind.clips().iter().map(|s| s.to_string()));
            ensure!(
                skeleton.bones.iter().any(|b| b.name == kind.tag()),
                "Plant attachment missing"
            );
        }
        if name == "c_boojum" {
            names.extend(resident::BOOJUM_CLIPS.iter().map(|s| s.to_string()));
        }
        if name == "c_ladybug" {
            names.extend(crate::ladybug::CLIPS.iter().map(|s| s.to_string()));
        }
        if name == crate::clockwork::MODEL {
            for tag in [
                "tag_left_hand",
                "tag_right_hand",
                "tag_steam",
                "tag_steam01",
            ] {
                ensure!(
                    skeleton.bones.iter().any(|b| b.name == tag),
                    "Clockwork attachment missing: {tag}"
                );
            }
            names.extend(crate::clockwork::CLIPS.iter().map(|s| s.to_string()));
        }
        if name == crate::fire_imp::MODEL {
            names.extend(crate::fire_imp::CLIPS.iter().map(|s| s.to_string()));
        }
        if let Some(kind) = crate::chess::Kind::from_model(name) {
            names.extend(kind.clips().iter().map(|s| s.to_string()));
        }
        if name == "cardguard_club" {
            names.extend(
                [
                    "walk",
                    "alert1",
                    "attack1",
                    "pain1",
                    "death_1",
                    "death_3a",
                    "death_3b",
                    "death_3c",
                    "death_top",
                ]
                .map(str::to_owned),
            );
        }
        if def.animations.contains_key("death_frozen") {
            names.insert("death_frozen".into());
        }
        let mut clips = BTreeMap::new();
        let tags = skeleton.unskinned_tags();
        let mut off_norm_tags = BTreeSet::new();
        for n in names {
            let file = def
                .animations
                .get(&n)
                .with_context(|| format!("Missing {name} animation {n}"))?;
            if file.ends_with(".ska") {
                let (clip, repaired) =
                    Animation::parse_tags(&assets.read(&format!("{}/{file}", def.path))?, &tags)?;
                off_norm_tags.extend(repaired.into_iter().map(|b| skeleton.bones[b].name.clone()));
                clips.insert(n, clip);
            }
        }
        let friendly = meta.friendly
            || [
                "c_cheshire",
                "c_gnomeold",
                "c_whiterabbit",
                "c_mockturtle",
                "c_gryphon",
                "c_caterpillar",
                "c_bill",
                "c_humptydumpty",
                "c_dormouse",
                "c_marchhare",
            ]
            .contains(&name)
            || name.starts_with("c_torchgnome");
        let sever = if name == "cardguard_club" {
            Some(crate::dismember::club_recipes(assets)?)
        } else {
            None
        };
        let combat = (name == "cardguard_club").then(|| Timing {
            sever,
            alert: clips["alert1"].duration(),
            attack: clips["attack1"].duration(),
            hit: clips["attack1"].frame_time * 14.,
            pain: clips["pain1"].duration(),
            death: clips["death_1"].duration(),
        });
        let card_muzzle = if name == "cardguard_spade" {
            let (staff, model) = crate::weapons::read_model(assets, "w_spadestaff")?;
            let bone = skeleton
                .bones
                .iter()
                .position(|b| b.name == "tag_weapon")
                .context("Spade staff attachment missing")?;
            let point = model
                .tags
                .get("tag_barrel")
                .and_then(|v| v.first())
                .context("Spade staff muzzle missing")?;
            Some((bone, *point * staff.scale))
        } else {
            None
        };
        if name == "cardguard_heart" {
            ensure!(
                skeleton.bones.iter().any(|b| b.name == "tag_barrel"),
                "Heart muzzle missing"
            );
        }
        Ok(Self {
            card_muzzle,
            card_cut: if name == "cardguard_spade" {
                Some(crate::dismember::card::recipe(assets)?)
            } else {
                None
            },
            ladybug_timing: if name == "c_ladybug" {
                Some(crate::ladybug::Timing::load(assets)?)
            } else {
                None
            },
            events: crate::animation_events::Model::load(assets, &path)?,
            def,
            meta,
            skeleton,
            clips,
            idle,
            greeting,
            idle_variants,
            talk_variants,
            friendly,
            combat,
            off_norm_tags,
        })
    }
    fn performance(
        &self,
        idle: &str,
        speaking: bool,
        ending: bool,
        time: f32,
    ) -> (Vec<Transform>, String, f32, bool) {
        let mut time = time;
        let transition = if speaking { "talkbegin" } else { "talkend" };
        if (speaking || ending) && self.clips.contains_key(transition) {
            let clip = &self.clips[transition];
            if time < clip.duration() {
                return (clip.sample(time, false), transition.into(), time, false);
            }
            time -= clip.duration();
        }
        let base = if speaking { &self.greeting } else { idle };
        let variants = if speaking {
            &self.talk_variants
        } else {
            &self.idle_variants
        };
        if !speaking && idle != self.idle {
            return (self.clips[idle].sample(time, true), idle.into(), time, true);
        }
        let mut sequence = vec![(
            base,
            if speaking {
                self.clips[base].duration()
            } else {
                // Retain complete idle cycles before a fidget, preserving its rest pose.
                self.clips[base].duration() * (14. / self.clips[base].duration()).ceil().max(1.)
            },
        )];
        sequence.extend(
            variants
                .iter()
                .filter(|n| n.as_str() != base)
                .map(|n| (n.as_str(), self.clips[n].duration())),
        );
        let total: f32 = sequence.iter().map(|(_, duration)| duration).sum();
        time = time.rem_euclid(total);
        for (i, (name, duration)) in sequence.iter().enumerate() {
            if time < *duration {
                let mut pose = self.clips[*name].sample(time, true);
                if sequence.len() > 1 && time < 0.12 {
                    let previous = sequence[(i + sequence.len() - 1) % sequence.len()].0;
                    let previous = &self.clips[previous];
                    for (p, old) in pose
                        .iter_mut()
                        .zip(previous.sample(previous.duration(), false))
                    {
                        *p = old.blend(*p, time / 0.12);
                    }
                }
                return (pose, (*name).into(), time, true);
            }
            time -= duration;
        }
        (self.clips[base].sample(0., true), base.to_owned(), 0., true)
    }
}
// Only numbered variants of the same posture: never mix seated and standing acts,
// or turn a script-specific gesture (such as talk_goaway) into a generic greeting.
fn family(def: &Definition, base: &str) -> Vec<String> {
    let numbered = def
        .animations
        .iter()
        .find(|(name, file)| {
            def.animations.get(base) == Some(*file) && name.ends_with(|c: char| c.is_ascii_digit())
        })
        .map(|(name, _)| name.as_str())
        .unwrap_or(base);
    let prefix = numbered.trim_end_matches(|c: char| c.is_ascii_digit());
    def.animations
        .iter()
        .filter(|(name, file)| {
            file.ends_with(".ska")
                && name.strip_prefix(prefix).is_some_and(|tail| {
                    !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
                })
        })
        .map(|(name, _)| name.clone())
        .collect()
}
struct Model {
    bone_scales: Vec<(usize, f32)>,
    frozen: Option<std::rc::Rc<crate::frozen::Art>>,
    electric: Option<std::rc::Rc<crate::electric::Art>>,
    burst: Option<crate::dismember::burst::Art>,
    visible: Vec<bool>,
    props: BTreeMap<String, Prop>,
    effects: BTreeMap<String, crate::particles::Attached>,
    magma_skins: Option<magma_art::Skins>,
    own_effect: Option<crate::particles::Attached>,
    _material_layers: Vec<std::rc::Rc<crate::render_fx::Surface>>,
    face: crate::facial::Face,
    data: Data,
    meshes: Vec<Option<Mesh>>,
    attachments: Vec<(usize, f32, Prop)>,
    electric_attachments: Vec<usize>,
    electric_props: Vec<String>,
}
/// A script-owned actor using the same mesh, attachment and animation path as placed NPCs.
pub struct Puppet(
    Model,
    bool,
    f32,
    crate::character::SkinMaterial,
    Option<crate::cinematic::ActorPose>,
    crate::facial::Watch,
);
impl Puppet {
    pub fn draw_electric(&self, remaining: f32) {
        self.0.draw_electric(remaining);
        self.3.bind();
    }
    pub fn atmosphere(&self, atmosphere: &Atmosphere, camera: Vec3) {
        self.3.atmosphere(atmosphere, camera);
    }
    pub fn handoff_pose(&self) -> Option<&crate::cinematic::ActorPose> {
        self.4.as_ref().filter(|p| !p.local.is_empty())
    }
    pub fn idle_clip(&self) -> &str {
        &self.0.data.idle
    }
    pub fn load(
        assets: &mut Assets,
        name: &str,
        clips: &[&str],
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        Ok(Self(
            Model::load(
                assets,
                name,
                &clips.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                specs,
            )?,
            true,
            0.,
            crate::character::skin_material()?,
            (name == "alice").then(|| crate::cinematic::ActorPose {
                local: Vec::new(),
                transform: Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                scale: 1.,
            }),
            Default::default(),
        ))
    }
    pub fn tag(
        &self,
        name: &str,
        clip: &str,
        time: f32,
        transform: Transform,
        scale: f32,
    ) -> Option<Transform> {
        self.sample_tag(name, clip, time, transform, scale, false)
    }
    pub fn looping_tag(
        &self,
        name: &str,
        clip: &str,
        time: f32,
        transform: Transform,
        scale: f32,
    ) -> Option<Transform> {
        self.sample_tag(name, clip, time, transform, scale, true)
    }
    fn sample_tag(
        &self,
        name: &str,
        clip: &str,
        time: f32,
        transform: Transform,
        scale: f32,
        looping: bool,
    ) -> Option<Transform> {
        let d = &self.0.data;
        let tag = d.skeleton.bones.iter().position(|b| b.name == name)?;
        let mut local = d.clips.get(clip)?.sample(time, looping);
        self.5.apply(&d.skeleton, &mut local);
        let mut pose = d.skeleton.global_pose(&local);
        crate::skeletal::scale_pose(&d.skeleton, &mut pose, &self.0.bone_scales);
        Some(Transform {
            translation: transform.point(pose[tag].translation * d.def.scale * scale),
            rotation: transform.rotation * pose[tag].rotation,
        })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_filtered(
        &mut self,
        clip: &str,
        time: f32,
        looping: bool,
        transform: Transform,
        scale: f32,
        fullbright: bool,
        hidden: &[&str],
    ) {
        let mut removed = Vec::new();
        for (i, surface) in self.0.data.skeleton.surfaces.iter().enumerate() {
            if hidden.contains(&surface.name.as_str()) {
                removed.push((i, self.0.meshes[i].take()));
            }
        }
        self.draw(clip, time, looping, transform, scale, fullbright);
        for (i, mesh) in removed {
            self.0.meshes[i] = mesh;
        }
    }
    pub fn mouth(&mut self, amount: f32) {
        self.2 = amount;
    }
    /// Scene-local authored jaw range; caller resets it between scenes.
    pub fn mouth_angle(&mut self, degrees: f32) {
        self.0.face.mouth_angle(degrees);
    }
    pub fn watch(&mut self, watch: crate::facial::Watch) {
        self.5 = watch;
    }
    pub(crate) fn bone_scales(&mut self, scales: &[(&str, f32)]) {
        self.0.bone_scales = scales.iter().filter_map(|(name, scale)| {
            self.0.data.skeleton.bones.iter().position(|b| b.name.eq_ignore_ascii_case(name)).map(|i| (i, scale.clamp(0., 4.)))
        }).collect();
    }
    /// A translucent skeletal silhouette, used by the Duchess's phase trail.
    pub(crate) fn draw_afterimage(
        &mut self,
        clip: &str,
        time: f32,
        transform: Transform,
        alpha: f32,
    ) {
        let d = &self.0.data;
        let Some(animation) = d.clips.get(clip) else {
            return;
        };
        let pose = d.skeleton.global_pose(&animation.sample(time, true));
        for (surface, mesh) in d.skeleton.surfaces.iter().zip(&mut self.0.meshes) {
            if surface.name.starts_with("cap_") {
                continue;
            }
            let Some(mesh) = mesh else { continue };
            for (src, dst) in surface.vertices.iter().zip(&mut mesh.vertices) {
                dst.position = transform.point(src.position(&pose) * d.def.scale);
                dst.color = [255, 255, 255, (alpha.clamp(0., 1.) * 255.) as u8];
            }
            crate::render_fx::effect(mesh, crate::materials::Blend::Alpha);
        }
        gl_use_default_material();
    }
    pub fn show_attachments(&mut self, visible: bool) {
        self.1 = visible;
    }
    /// Persist script-owned surface state across animation changes.
    pub(crate) fn surface_visible(&mut self, name: &str, visible: bool) {
        let name = name.to_lowercase();
        if visible { self.0.data.meta.hidden.remove(&name); }
        else { self.0.data.meta.hidden.insert(name); }
    }
    pub(crate) fn reset_effects(&mut self) {
        if let Some(effect) = &self.0.own_effect {
            self.0.own_effect = Some(effect.fork());
        }
        for effect in self.0.effects.values_mut() {
            *effect = effect.fork();
        }
    }
    pub(crate) fn tune_effects(&mut self, size: f32, opacity: f32) {
        for effect in self
            .0
            .own_effect
            .iter_mut()
            .chain(self.0.effects.values_mut())
        {
            effect.resize(size);
            effect.attenuate(opacity);
        }
    }
    pub fn draw(
        &mut self,
        clip: &str,
        time: f32,
        looping: bool,
        transform: Transform,
        scale: f32,
        fullbright: bool,
    ) {
        self.draw_dissolving(clip, time, looping, transform, scale, fullbright, 0.);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_dissolving(
        &mut self,
        clip: &str,
        time: f32,
        looping: bool,
        transform: Transform,
        scale: f32,
        fullbright: bool,
        dissolve: f32,
    ) {
        let d = &self.0.data;
        let animation = d.clips.get(clip).unwrap_or(&d.clips[&d.idle]);
        let mut local = animation.sample(time, looping);
        self.5.apply(&d.skeleton, &mut local);
        if let Some(pose) = &mut self.4 {
            pose.local.clone_from(&local);
            pose.transform = transform;
            pose.scale = scale;
        }
        let visual = d.events.visual(
            clip,
            time,
            animation.duration(),
            animation.frame_time,
            looping,
        );
        let material = self.3.clone();
        material.draw_dissolving(dissolve, || {
            self.0.draw_pose(
                &local,
                self.2,
                time,
                !clip.starts_with("death"),
                transform,
                scale,
                fullbright,
                self.1,
                &visual,
                None,
            )
        });
        if clip == "death_frozen" {
            self.0.draw_frozen();
        }
    }
    pub fn draw_performance(
        &mut self,
        speaking: bool,
        time: f32,
        transform: Transform,
        scale: f32,
        fullbright: bool,
    ) {
        let d = &self.0.data;
        let (local, clip, at, looping) = d.performance(&d.idle, speaking, false, time);
        if let Some(pose) = &mut self.4 {
            pose.local.clone_from(&local);
            pose.transform = transform;
            pose.scale = scale;
        }
        let animation = &d.clips[&clip];
        let visual = d.events.visual(
            &clip,
            at,
            animation.duration(),
            animation.frame_time,
            looping,
        );
        self.3.bind();
        self.0.draw_pose(
            &local, self.2, time, true, transform, scale, fullbright, self.1, &visual, None,
        );
    }
    pub fn draw_guard(
        &mut self,
        guard: &Guard,
        fullbright: bool,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        self.3.atmosphere(atmosphere, camera);
        self.0
            .draw_guard(guard, fullbright, camera, atmosphere, &self.3);
    }
    #[allow(clippy::too_many_arguments)]
    pub fn draw_effects(
        &mut self,
        clip: &str,
        time: f32,
        looping: bool,
        transform: Transform,
        scale: f32,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        self.0
            .draw_effects(clip, time, looping, transform, scale, camera, atmosphere);
        self.3.bind();
    }
}
pub fn placed_guards(map: &Bsp, name: &str) -> Vec<Guard> {
    placements(map, name, None, false)
        .into_iter()
        .filter(|s| s.model == "cardguard_club")
        .map(|s| Guard::new(s.origin, s.yaw, s.scale))
        .collect()
}
/// Headless rig access for scene-owned Heart/Spade encounter pools.
pub struct CardRig(Data);
impl CardRig {
    pub fn load(assets: &mut Assets, kind: crate::cards::Kind) -> Result<Self> { Ok(Self(Data::load(assets, kind.model(), &kind.clips().iter().map(|s| (*s).to_owned()).collect::<Vec<_>>())?)) }
    pub fn step(&self, guard: &mut crate::cards::Guard, world: &World, eye: Vec3, out: &mut combat::Feedback) { guard.step(world, eye, &self.0, out); }
}
impl Puppet {
    /// Draw a supplied death animation with only one side of its authored surface split.
    pub fn draw_split(&mut self, clip: &str, time: f32, looping: bool, at: Transform, pattern: &str, detached: bool, bright: bool) {
        let data=&self.0.data;let animation=&data.clips[clip];
        let local=animation.sample(time,looping);
        let visual=data.events.visual(clip,time,animation.duration(),animation.frame_time,looping);
        self.3.bind();self.0.draw_pose(&local,0.,time,looping,at,1.,bright,false,&visual,Some((pattern,"",detached)));
    }
    pub fn draw_card(&mut self, guard: &crate::cards::Guard, bright: bool, camera: Vec3, atmosphere: &Atmosphere) {
        self.3.atmosphere(atmosphere, camera); self.0.draw_card(guard, bright, &self.3);
    }
}
pub fn guard_timing(assets: &mut Assets) -> Result<Timing> {
    Data::load(assets, "cardguard_club", &[])?
        .combat
        .context("Missing club guard clips")
}
impl Model {
    fn draw_electric(&self, remaining: f32) {
        if remaining <= 0. {
            return;
        }
        if let Some(art) = &self.electric {
            for (mesh, visible) in self.meshes.iter().zip(&self.visible) {
                if *visible {
                    if let Some(mesh) = mesh {
                        art.draw(mesh, remaining);
                    }
                }
            }
            for &i in &self.electric_attachments {
                self.attachments[i].2.draw_electric(art, remaining);
            }
            for name in &self.electric_props {
                self.props[name].draw_electric(art, remaining);
            }
        }
    }
    fn draw_frozen(&self) {
        if let Some(art) = &self.frozen {
            for (mesh, visible) in self.meshes.iter().zip(&self.visible) {
                if *visible {
                    if let Some(mesh) = mesh {
                        art.draw(mesh);
                    }
                }
            }
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_pose(
        &mut self,
        local: &[Transform],
        mouth: f32,
        time: f32,
        alive: bool,
        transform: Transform,
        scale: f32,
        fullbright: bool,
        attachments: bool,
        visual: &crate::animation_events::VisualState,
        piece: Option<(&str, &str, bool)>,
    ) {
        self.visible.fill(false);
        self.electric_attachments.clear();
        self.electric_props.clear();
        if scale <= 0. {
            return;
        }
        let d = &self.data;
        let mut pose = self.face.pose(&d.skeleton, local, mouth);
        let scales = if self.bone_scales.is_empty() { Vec::new() } else { crate::skeletal::scale_pose(&d.skeleton, &mut pose, &self.bone_scales) };
        self.face.blink(&mut self.meshes, time, alive);
        let mut removed = Vec::new();
        for (i, surface) in d.skeleton.surfaces.iter().enumerate() {
            let mut hidden = d
                .meta
                .hidden
                .iter()
                .any(|p| crate::animation_events::matches(p, &surface.name));
            for (pattern, value) in &visual.surfaces {
                if crate::animation_events::matches(pattern, &surface.name) {
                    hidden = *value;
                }
            }
            if let Some((pattern, cap, fragment)) = piece {
                hidden = if surface.name == cap {
                    false
                } else if fragment {
                    !crate::animation_events::matches(pattern, &surface.name)
                } else {
                    hidden || crate::animation_events::matches(pattern, &surface.name)
                };
            }
            if hidden {
                removed.push((i, self.meshes[i].take()));
            }
        }
        crate::character::draw_skin_scaled(
            &d.skeleton,
            &mut self.meshes,
            &pose,
            transform,
            d.def.scale * scale,
            fullbright,
            &scales,
        );
        for (visible, mesh) in self.visible.iter_mut().zip(&self.meshes) {
            *visible = mesh.is_some();
        }
        // Model instances are shared by many actors. Always restore every mesh.
        for (i, mesh) in removed {
            self.meshes[i] = mesh;
        }
        if !attachments {
            return;
        }
        let tag_pose = |bone: usize| Transform {
            translation: transform.point(pose[bone].translation * d.def.scale * scale),
            rotation: transform.rotation * pose[bone].rotation,
        };
        for (i, (tag, size, prop)) in self.attachments.iter_mut().enumerate() {
            let (model, name, _) = &d.meta.attachments[i];
            if visual.removed.contains(name)
                || visual.removed.contains(model)
                || visual.removed.contains(&format!("{model}.tik"))
                || (visual.hide_weapon && name == "tag_weapon")
                || visual.attachments.contains_key(name)
            {
                continue;
            }
            prop.draw_frame(tag_pose(*tag), *size * scale, fullbright, time, true);
            self.electric_attachments.push(i);
        }
        for (name, attachment) in &visual.attachments {
            if let (Some(prop), Some(tag)) = (
                self.props.get_mut(&attachment.model),
                d.skeleton.bones.iter().position(|b| b.name == *name),
            ) {
                prop.draw_frame(
                    tag_pose(tag),
                    attachment.scale * scale,
                    fullbright,
                    attachment.age,
                    true,
                );
                self.electric_props.push(attachment.model.clone());
            }
        }
    }
    fn draw_guard(
        &mut self,
        g: &Guard,
        fullbright: bool,
        camera: Vec3,
        atmosphere: &Atmosphere,
        material: &crate::character::SkinMaterial,
    ) {
        if let Some(state) = &g.burst {
            material.bind();
            if let Some(art) = &mut self.burst {
                art.draw(state, fullbright);
            }
            material.bind();
            return;
        }
        let d = &self.data;
        let clip = &d.clips[g.clip()];
        let visual = d.events.visual(
            g.clip(),
            g.time,
            clip.duration(),
            clip.frame_time,
            g.state.loops(),
        );
        let recipe = d
            .events
            .clips
            .get(g.clip())
            .into_iter()
            .flatten()
            .find_map(|e| match &e.command {
                crate::animation_events::Command::Gib {
                    cap,
                    animation,
                    surfaces,
                    ..
                } => Some((surfaces.clone(), cap.clone(), animation.clone())),
                _ => None,
            });
        let pose_time = if g.frozen {
            g.time
                .min(if g.ranged { 5. } else { 30. } * clip.frame_time)
        } else {
            g.time
        };
        let local = clip.sample(pose_time, g.state.loops());
        let piece = recipe
            .as_ref()
            .filter(|_| g.dismember.severed)
            .map(|(pattern, cap, _)| (pattern.as_str(), cap.as_str(), false));
        let scale = if g.frozen {
            g.scale * (2.5 - g.time).clamp(0., 1.)
        } else if g.cut {
            g.scale * (1. - g.dismember.fade())
        } else if g.health <= 0. {
            g.scale * (clip.duration() - g.time).clamp(0., 1.)
        } else {
            g.scale
        };
        material.bind();
        self.draw_pose(
            &local,
            0.,
            g.time,
            g.health > 0.,
            Transform {
                translation: g.feet,
                rotation: Quat::from_rotation_z(g.yaw),
            },
            scale,
            fullbright,
            true,
            &visual,
            piece,
        );
        if g.frozen {
            self.draw_frozen();
        }
        if let Some(art) = &self.electric {
            if g.electric > 0. {
                for (mesh, visible) in self.meshes.iter().zip(&self.visible) {
                    if *visible {
                        if let Some(mesh) = mesh {
                            art.draw(mesh, g.electric);
                        }
                    }
                }
            }
        }
        if let (Some(fragment), Some((pattern, cap, animation))) = (&g.dismember.fragment, &recipe)
        {
            let local = self.data.clips[animation].sample(g.dismember.age, false);
            material.draw_dissolving(g.dismember.fade(), || {
                self.draw_pose(
                    &local,
                    0.,
                    g.dismember.age,
                    false,
                    fragment.transform(),
                    fragment.scale,
                    fullbright,
                    false,
                    &Default::default(),
                    Some((pattern, cap, true)),
                )
            });
        }
        if g.cut && g.dismember.age < crate::dismember::LIFE {
            self.draw_effects(
                g.clip(),
                g.time,
                false,
                Transform {
                    translation: g.feet,
                    rotation: Quat::from_rotation_z(g.yaw),
                },
                scale,
                camera,
                atmosphere,
            );
        }
        material.bind();
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_effects(
        &mut self,
        clip: &str,
        time: f32,
        looping: bool,
        transform: Transform,
        scale: f32,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        let d = &self.data;
        let Some(animation) = d.clips.get(clip) else {
            return;
        };
        let visual = d.events.visual(
            clip,
            time,
            animation.duration(),
            animation.frame_time,
            looping,
        );
        let tag_pose = |tag: Option<&str>, at: f32| {
            let Some(bone) =
                tag.and_then(|name| d.skeleton.bones.iter().position(|b| b.name == name))
            else {
                return transform;
            };
            let mut pose = d.skeleton.global_pose(&animation.sample(at, looping));
            crate::skeletal::scale_pose(&d.skeleton, &mut pose, &self.bone_scales);
            Transform {
                translation: transform.point(pose[bone].translation * d.def.scale * scale),
                rotation: transform.rotation * pose[bone].rotation,
            }
        };
        for (tag, a) in &visual.attachments {
            if let Some(effect) = self.effects.get_mut(&a.model) {
                let start = time - a.age;
                let prop = self.props.get(&a.model);
                effect.draw(
                    a.age,
                    scale * a.scale,
                    |at, child_tag| {
                        let mut pose = tag_pose(Some(tag), start + at);
                        if let (Some(prop), Some(child_tag)) = (prop, child_tag) {
                            pose.translation = prop.point(pose, child_tag, scale * a.scale);
                        }
                        pose
                    },
                    |_, _, default| default,
                    camera,
                    atmosphere,
                );
            }
        }
        if let Some(effect) = &mut self.own_effect {
            effect.draw(
                time,
                scale,
                |at, tag| tag_pose(tag, at),
                |name, at, default| {
                    d.events
                        .visual(
                            clip,
                            at,
                            animation.duration(),
                            animation.frame_time,
                            looping,
                        )
                        .emitters
                        .get(name)
                        .copied()
                        .unwrap_or(default)
                },
                camera,
                atmosphere,
            );
        }
    }
    fn load(
        assets: &mut Assets,
        name: &str,
        extra: &[String],
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let data = Data::load(assets, name, extra)?;
        let mut meshes = Vec::new();
        let mut material_layers = Vec::new();
        let mut textures: BTreeMap<String, Texture2D> = BTreeMap::new();
        for s in &data.skeleton.surfaces {
            let used_by_event = data
                .events
                .clips
                .values()
                .flatten()
                .any(|e| match &e.command {
                    crate::animation_events::Command::Gib { cap, .. } => cap == &s.name,
                    crate::animation_events::Command::Surface { name, .. } => {
                        crate::animation_events::matches(name, &s.name)
                    }
                    _ => false,
                });
            if !used_by_event
                && (data.meta.hidden.contains(&s.name) || data.meta.hidden.contains("all"))
            {
                meshes.push(None);
                continue;
            }
            let skin = data
                .def
                .skins
                .get(&s.name)
                .or_else(|| data.def.skins.get("all"))
                .context("NPC surface skin missing")?;
            let path = texture::resolve(assets, &format!("{}/{skin}", data.def.path), specs)
                .or_else(|| texture::resolve(assets, skin, specs))
                .with_context(|| format!("NPC skin {skin}"))?;
            let tex = if let Some(t) = textures.get(&path) {
                t.clone()
            } else {
                let image = texture::decode(assets, &path)?;
                let t = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                t.set_filter(FilterMode::Linear);
                textures.insert(path, t.clone());
                t
            };
            material_layers.push(crate::render_fx::register(
                assets,
                &tex,
                &[format!("{}/{skin}", data.def.path), skin.clone()],
                specs,
            )?);
            meshes.push(Some(Mesh {
                vertices: s
                    .vertices
                    .iter()
                    .map(|v| Vertex {
                        position: Vec3::ZERO,
                        uv: v.uv,
                        color: [255; 4],
                        normal: Vec4::ZERO,
                    })
                    .collect(),
                indices: s.indices.clone(),
                texture: Some(tex),
            }));
        }
        let mut attachments = Vec::new();
        for (name, tag, scale) in &data.meta.attachments {
            let bone = data
                .skeleton
                .bones
                .iter()
                .position(|b| &b.name == tag)
                .with_context(|| format!("NPC attachment tag {tag}"))?;
            attachments.push((bone, *scale, Prop::load(assets, name, specs)?));
        }
        let face = crate::facial::Face::load(
            assets,
            &format!("models/{name}.tik"),
            &data.def,
            &data.skeleton,
            &meshes,
            specs,
        )?;
        let mut props = BTreeMap::new();
        let mut effects = BTreeMap::new();
        for (clip, events) in &data.events.clips {
            if !data.clips.contains_key(clip) {
                continue;
            }
            for event in events {
                if let crate::animation_events::Command::Attach { model, tag, .. } = &event.command
                {
                    if !data.skeleton.bones.iter().any(|b| b.name == *tag)
                        || props.contains_key(model)
                        || effects.contains_key(model)
                    {
                        continue;
                    }
                    let mut effect = crate::particles::Attached::load(assets, model, specs)?;
                    if effect.is_none() && crate::wildlife::Kind::from_model(name).is_some() {
                        effect = crate::particles::Attached::load_bursts(
                            assets,
                            model,
                            1. / 30.,
                            specs,
                        )?;
                    }
                    if let Some(effect) = effect {
                        effects.insert(model.clone(), effect);
                    }
                    // An attachment can have both a mesh and emitters (the elder's
                    // pipe is one). Loading its smoke must not suppress the prop.
                    // Effect-only rigs use unskinned dummy geometry as a tag
                    // carrier. Only attachments with assigned skins draw it.
                    if !effects.contains_key(model)
                        || !Definition::load(assets, &format!("models/{model}.tik"))?
                            .skins
                            .is_empty()
                    {
                        props.insert(model.clone(), Prop::load(assets, model, specs)?);
                    }
                }
            }
        }
        let mut own_effect = crate::particles::Attached::load(assets, name, specs)?;
        if name == "c_jabberwock" {
            if let Some(effect) = &mut own_effect {
                // The breath jet's velocity is local to the animated mouth.
                // Pipe smoke and other ambient emitters keep their world axes.
                effect.orient_velocity("sparkle");
            }
        }
        Ok(Self {
            visible: vec![false; meshes.len()],
            bone_scales: Vec::new(),
            electric: Some(crate::electric::Art::load(assets, specs)?),
            burst: matches!(name, "cardguard_club" | "cardguard_diamond")
                .then(|| {
                    crate::dismember::burst::Art::load(assets, specs, name == "cardguard_diamond")
                })
                .transpose()?,
            frozen: (data.clips.contains_key("death_frozen") || matches!(name,"c_firesnark" | "c_phantasm"))
                .then(|| crate::frozen::Art::load(assets, specs))
                .transpose()?,
            props,
            effects,
            magma_skins: if name == crate::magma::MODEL {
                Some(magma_art::Skins::load(assets, &data, specs)?)
            } else {
                None
            },
            own_effect,
            face,
            _material_layers: material_layers,
            data,
            meshes,
            attachments,
            electric_attachments: Vec::new(),
            electric_props: Vec::new(),
        })
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Actor {
    #[serde(default, skip_serializing_if = "crate::electric::inactive")]
    electric: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    walk: Option<ambient::Walk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    guide: Option<companions::State>,
    #[serde(default)]
    watch: crate::facial::Watch,
    #[serde(skip)]
    mouth: f32,
    story_visible: bool,
    speaking: bool,
    #[serde(default)]
    ending_talk: bool,
    model: usize,
    spawn: Spawn,
    yaw: f32,
    time: f32,
    greeting: f32,
    cooldown: f32,
    guard: Option<Guard>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ant: Option<crate::ant::Ant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chess: Option<crate::chess::Piece>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    imp: Option<crate::fire_imp::Imp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clock: Option<crate::clockwork::Automaton>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resident: Option<resident::Resident>,
    // Keep the authored Spawn unchanged: it is the identity used by old saves.
    #[serde(default)]
    footing: Option<Vec3>,
    #[serde(default)]
    falling: f32,
}
impl Actor {
    /// The bounds `Npcs::restore` enforces on every saved actor.
    fn validate_save(&self) -> Result<()> {
        ensure!(
            self.watch.valid()
                && (0. ..=crate::electric::LIFE).contains(&self.electric)
                && self
                    .walk
                    .as_ref()
                    .is_none_or(|w| self.spawn.name == "return_insane2" && w.valid())
                && self.time >= 0.
                && self.greeting >= 0.
                && self.cooldown >= 0.
                && self.footing.is_none_or(|p| p.is_finite())
                && self.yaw.is_finite()
                && (0. ..=800.).contains(&self.falling),
            "Invalid saved NPC timers"
        );
        ensure!(
            self.guide.as_ref().is_none_or(|g| g.valid(&self.spawn)
                && self.resident.is_none()
                && self.guard.is_none()),
            "Invalid companion state"
        );
        if let Some(g) = &self.guard {
            g.validate_save()?;
        }
        if let Some(a) = &self.ant {
            a.validate()?;
            ensure!(
                (a.enabled || garden::can_wait(&self.spawn))
                    && a.model() == self.spawn.model
                    && a.scale == self.spawn.scale
                    && self.guard.is_none(),
                "Saved Ant type changed"
            );
        }
        if let Some(p) = &self.resident {
            p.validate(&self.spawn)?;
            ensure!(
                self.guard.is_none()
                    && self.ant.is_none()
                    && self.chess.is_none()
                    && self.imp.is_none()
                    && self.clock.is_none(),
                "Resident has two owners"
            );
        }
        if let Some(p) = &self.clock {
            p.validate()?;
            ensure!(
                self.spawn.model == crate::clockwork::MODEL
                    && p.scale == self.spawn.scale
                    && self.guard.is_none()
                    && self.ant.is_none()
                    && self.chess.is_none()
                    && self.imp.is_none(),
                "Saved Clockwork type changed"
            );
        }
        if let Some(p) = &self.imp {
            p.validate()?;
            ensure!(
                self.spawn.model == crate::fire_imp::MODEL
                    && p.scale == self.spawn.scale
                    && self.guard.is_none()
                    && self.ant.is_none()
                    && self.chess.is_none(),
                "Saved Fire Imp type changed"
            );
        }
        if let Some(p) = &self.chess {
            p.validate()?;
            ensure!(
                p.kind.model() == self.spawn.model
                    && p.scale == self.spawn.scale
                    && self.guard.is_none()
                    && self.ant.is_none(),
                "Saved chess type changed"
            );
        }
        Ok(())
    }
    fn position(&self) -> Vec3 {
        self.ant
            .as_ref()
            .map(|a| a.feet)
            .or_else(|| self.chess.as_ref().map(|p| p.feet))
            .or_else(|| self.imp.as_ref().map(|p| p.feet))
            .or_else(|| self.clock.as_ref().map(|p| p.feet))
            .or_else(|| self.resident.as_ref().map(|p| p.position()))
            .or_else(|| self.guard.as_ref().map(|g| g.feet))
            .unwrap_or(self.footing.unwrap_or(self.spawn.origin))
    }
    fn target(&self) -> Vec3 {
        self.position() + Vec3::Z * (32. * self.spawn.scale).clamp(12., 100.)
    }
    fn visible(&self, world: &World, eye: Vec3) -> bool {
        let trace = world.sweep(eye, self.target(), Vec3::splat(0.5));
        !trace.start_solid && trace.fraction >= 1.
    }
    fn can_greet(&self, world: &World, eye: Vec3, aim: Vec3) -> bool {
        self.cooldown <= 0. && talk_reachable(world, eye, aim, self.target())
    }
    fn tick(&mut self, dt: f32, world: &World, eye: Vec3) {
        if dt <= 0. {
            return;
        }
        self.time += dt;
        let was_greeting = self.greeting > 0.;
        self.greeting = (self.greeting - dt).max(0.);
        if was_greeting && self.greeting == 0. && !self.speaking {
            self.ending_talk = true;
            self.time = 0.;
        }
        self.cooldown = (self.cooldown - dt).max(0.);
        self.face(dt, world, eye, self.greeting > 0.);
    }
    fn face(&mut self, dt: f32, world: &World, eye: Vec3, attentive: bool) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let delta = eye - self.target();
        if delta.truncate().length_squared() < 0.01 {
            return;
        }
        let range = if attentive { 1200. } else { 400. };
        let desired = if delta.length_squared() < range * range && self.visible(world, eye) {
            delta.y.atan2(delta.x)
        } else if attentive {
            self.yaw
        } else {
            self.spawn.yaw
        };
        let difference = (desired - self.yaw + std::f32::consts::PI)
            .rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        self.yaw += difference.clamp(-dt * 1.8, dt * 1.8);
    }
    fn ground(&mut self, dt: f32, world: &World, meta: &Metadata) {
        if let Some(p) = &mut self.resident {
            p.place(world);
            return;
        }
        if let Some(p) = &mut self.clock {
            if self.footing.is_none() {
                let target = p.target(0);
                p.feet = world
                    .actor_footing(p.feet, Vec3::Z * target.half.z, target.half, 1024.)
                    .unwrap_or(p.feet);
                self.footing = Some(p.feet);
            }
            return;
        }
        if let Some(p) = &mut self.imp {
            if p.launch.is_some() { self.footing = Some(p.feet); }
            if self.footing.is_none() {
                let target = p.target(0);
                p.feet = world
                    .actor_footing(p.feet, Vec3::Z * target.half.z, target.half, 1024.)
                    .unwrap_or(p.feet);
                self.footing = Some(p.feet);
            }
            return;
        }
        if let Some(p) = &mut self.chess {
            if self.footing.is_none() {
                let target = p.target(0);
                p.feet = world
                    .actor_footing(p.feet, Vec3::Z * target.half.z, target.half, 1024.)
                    .unwrap_or(p.feet);
                self.footing = Some(p.feet);
            }
            return;
        }
        if let Some(a) = &mut self.ant {
            if self.footing.is_none() {
                let target = a.target(0);
                a.feet = world
                    .actor_footing(a.feet, Vec3::Z * target.half.z, target.half, 1024.)
                    .unwrap_or(a.feet);
                self.footing = Some(a.feet);
            }
            return;
        }
        if let Some(g) = &mut self.guard {
            g.place(world);
            return;
        }
        if meta.airborne {
            return;
        }
        let (min, max) = meta
            .bounds
            .unwrap_or((vec3(-16., -16., 0.), vec3(16., 16., 60.)));
        let center = (min + max) * (0.5 * self.spawn.scale);
        let half = (max - min) * (0.5 * self.spawn.scale);
        let feet = self.position();
        if self.footing.is_none() {
            self.footing = Some(
                world
                    .actor_footing(feet, center, half, 1024.)
                    .unwrap_or(feet),
            );
            return;
        }
        if dt <= 0. || !dt.is_finite() {
            return;
        }
        // Follow support under a stationary actor, and fall if it moves away.
        // Do not snap downward to a different floor after initial placement.
        let dt = dt.min(0.1);
        self.falling = (self.falling + 800. * dt).min(800.);
        let end = feet - Vec3::Z * self.falling * dt;
        let trace = world.sweep(feet + center, end + center, half);
        if !trace.start_solid {
            self.footing = Some(feet.lerp(end, trace.fraction));
        } else if let Some(p) = world.actor_footing(feet, center, half, 8.) {
            self.footing = Some(p);
        }
        if trace.start_solid || trace.fraction < 1. {
            self.falling = 0.;
        }
    }
}
/// Original port interaction geometry, shared by placed and controller-owned actors.
pub fn talk_reachable(world: &World, eye: Vec3, aim: Vec3, target: Vec3) -> bool {
    let delta = target - eye;
    delta.length_squared() < 140. * 140.
        && (delta.length_squared() < 20. * 20. || delta.normalize().dot(aim) > 0.75)
        && {
            let trace = world.sweep(eye, target, Vec3::splat(0.5));
            !trace.start_solid && trace.fraction >= 1.
        }
}

pub struct Npcs {
    attack_fx: std::rc::Rc<attack_fx::Art>,
    /// Battle Royale places enemies above a player-only ceiling.
    actor_world: Option<World>,
    garden: Option<garden::Logic>,
    resident_ambushes: Vec<resident_spawns::Ambush>,
    resident_initial: Vec<Option<resident::Resident>>,
    acorn: Option<Prop>,
    resident_diamond: Option<Prop>,
    thorn: Option<Prop>,
    spore: Option<crate::particles::Attached>,
    spores: BTreeMap<(usize, u32), crate::particles::Attached>,
    clock_ambushes: Vec<clock_spawns::Ambush>,
    clock_prop: Option<Prop>,
    clock_trail: Option<crate::particles::Attached>,
    clock_art: BTreeMap<usize, clock_art::Art>,
    card_art: BTreeMap<usize, card_art::Art>,
    snark_art: BTreeMap<usize, snark_art::Art>,
    magma_art: BTreeMap<usize, magma_art::Art>,
    wildlife_art: BTreeMap<usize, wildlife_art::Art>,
    magma_fire: Option<crate::particles::Attached>,
    burrow_art: BTreeMap<usize, burrow_art::Art>,
    snark_props: BTreeMap<crate::snark::Kind, snark_art::Projectiles>,
    card_props: BTreeMap<crate::cards::Kind, card_art::Projectiles>,
    imp_ambushes: Vec<imp_spawns::Ambush>,
    imp_debris: Vec<Prop>,
    chess_ambushes: Vec<chess_spawns::Ambush>,
    pupil_path: Vec<Vec3>,
    companion_routes: companions::Routes,
    scene_hidden: BTreeSet<String>,
    owns_village: bool,
    // Retain legacy decorative identities in snapshots; the encounter owns their live actor.
    owns_duchess: bool,
    owns_ladybugs: bool,
    owns_pandemonium: bool,
    owns_school_first: bool,
    owns_school2: bool,
    /// Registered controllers serving this visit; they own the actors `owns_npc` names.
    levels: Vec<&'static crate::levels::Registration>,
    ant_shots: Option<[crate::weapons::Prop; 2]>,
    models: Vec<Model>,
    actors: Vec<Actor>,
    material: crate::character::SkinMaterial,
    pub greetings: u64,
}
impl Npcs {
    pub(crate) fn check_school2_ownership(&self) -> Result<()> {
        let diamonds = self
            .actors
            .iter()
            .filter(|a| a.spawn.model == "cardguard_diamond")
            .collect::<Vec<_>>();
        ensure!(
            self.owns_school2
                && diamonds.len() == 6
                && diamonds.iter().all(|a| self.registry_owns(a)),
            "School2 decorative Diamonds overlap combat ownership"
        );
        Ok(())
    }
    /// Whether a registered controller owns this placed actor.
    fn registry_owns(&self, actor: &Actor) -> bool {
        (self.owns_village
            && matches!(
                actor.spawn.name.as_str(),
                "rabbit_actor1"
                    | "cat_hole1"
                    | "knife_cat"
                    | "cat_shrink1"
                    | "essence_cat"
                    | "bridge_cat"
            ))
            || (self.owns_ladybugs && crate::pool::cinema::owns(&actor.spawn.name))
            || (self.owns_school_first && crate::school::cinema::owns(&actor.spawn.name))
            || (self.owns_school2 && (actor.spawn.model == "cardguard_diamond" || actor.spawn.name == "dice_cat"))
            || self
                .levels
                .iter()
                .any(|r| (r.owns_npc)(&actor.spawn.name, &actor.spawn.model))
    }
    pub fn scene_hidden(&mut self, names: &[&str]) {
        self.scene_hidden = names.iter().map(|n| n.to_string()).collect();
    }
    pub fn scene_place(&mut self, name: &str, pose: Transform) {
        if let Some(a) = self.actors.iter_mut().find(|a| a.spawn.name == name) {
            // Rebuild the location after old saves without resetting the actor's
            // later turns toward Alice every frame at an unchanged marker.
            if a.footing
                .is_none_or(|p| p.distance_squared(pose.translation) > 1.)
            {
                a.yaw = pose.rotation.to_euler(EulerRot::ZYX).0;
            }
            a.footing = Some(pose.translation);
            a.falling = 0.;
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            actors: self.actors.clone(),
            greetings: self.greetings,
            garden: self.garden.as_ref().map(|g|g.saved.clone()),
        }
    }
    pub fn restore(&mut self, s: &Snapshot) -> Result<()> {
        if let Some(g) = &self.garden { g.validate(s.garden.as_ref())?; }
        self.card_art.clear();
        self.snark_art.clear();
        self.burrow_art.clear();
        self.magma_art.clear();
        self.wildlife_art.clear();
        self.clock_art.clear();
        self.spores.clear();
        let mut matched = std::collections::BTreeSet::new();
        let maze = self.levels.iter().any(|r| matches!(r.id, "hedge1" | "tower1"));
        for b in &s.actors {
            let (index, a) = self
                .actors
                .iter_mut()
                .enumerate()
                .find(|(i, a)| !matched.contains(i) && a.spawn.saved_identity(&b.spawn, maze))
                .context("Saved NPC identity does not match map")?;
            if a.spawn != b.spawn {
                b.validate_save()?;
                matched.insert(index);
                continue; // Keep these formerly active precache actors in their fresh inactive state.
            }
            ensure!(
                a.guard.is_some() == b.guard.is_some(),
                "Saved NPC type changed"
            );
            b.validate_save()?;
            ensure!(
                b.ant.is_none() || a.ant.is_some(),
                "Saved Ant ownership changed"
            );
            ensure!(
                b.imp.is_none() || a.imp.is_some(),
                "Saved Fire Imp ownership changed"
            );
            let mut imp = b.imp.clone().or_else(|| {
                a.imp.as_ref().map(|_| {
                    crate::fire_imp::Imp::new(
                        b.position(),
                        b.yaw,
                        b.spawn.scale,
                        index,
                        b.spawn.imp_spawn.is_none(),
                    )
                })
            });
            if let Some(p) = imp.as_mut().filter(|p| !p.active && p.launch.is_none()) {
                p.launch = a.imp.as_ref().and_then(|p| p.launch);
            }
            ensure!(
                b.clock.is_none() || a.clock.is_some(),
                "Saved Clockwork ownership changed"
            );
            let clock = b.clock.clone().or_else(|| {
                a.clock.as_ref().map(|_| {
                    crate::clockwork::Automaton::new(
                        b.position(),
                        b.yaw,
                        b.spawn.scale,
                        index,
                        b.spawn.clock_spawn.is_none(),
                    )
                })
            });
            ensure!(
                b.resident.is_none() || a.resident.is_some(),
                "Resident ownership changed"
            );
            let resident = if let Some(b) = &b.resident {
                b.matches(
                    self.resident_initial[index]
                        .as_ref()
                        .context("Missing resident seed")?,
                )?;
                Some(b.clone())
            } else {
                self.resident_initial[index].clone().map(|mut p| {
                    match &mut p.body {
                        resident::Body::Boojum(c) => c.feet = b.position(),
                        resident::Body::Ladybug(c) => c.feet = b.position(),
                        resident::Body::Diamond(c) => c.feet = b.position(),
                        resident::Body::Card(c) => c.feet = b.position(),
                        resident::Body::Snark(c) => c.feet = b.position(),
                        resident::Body::Insect(c) => c.feet = b.position(),
                        resident::Body::Magma(c) => c.feet = b.position(),
                        resident::Body::Wildlife(c) => c.feet = b.position(),
                        resident::Body::Plant(_) => {}
                    };
                    p
                })
            };
            let model = a.model;
            ensure!(
                b.chess.is_none() || a.chess.is_some(),
                "Saved chess ownership changed"
            );
            let chess = b.chess.clone().or_else(|| {
                a.chess.as_ref().map(|current| {
                    let mut p = crate::chess::Piece::new(
                        current.kind,
                        b.position(),
                        b.yaw,
                        b.spawn.scale,
                        index,
                    );
                    p.active = b.spawn.chess_spawn.is_none() && current.active;
                    p.script_wait = current.script_wait;
                    p
                })
            });
            // Older format-12 saves had inert Ant scenery. Keep its placement/identity;
            // the new controller starts there without replacing any saved live combat.
            let ant = b.ant.clone().or_else(|| {
                a.ant.as_ref().map(|current| {
                    let mut ant =
                        crate::ant::Ant::new(current.id, current.corporal, b.position(), b.yaw);
                    ant.scale = b.spawn.scale;
                    ant.script_wait = current.script_wait;
                    ant
                })
            });
            ensure!(
                b.guide.is_none()
                    || a.guide
                        .as_ref()
                        .is_some_and(|g| Some(g.role) == b.guide.as_ref().map(|g| g.role)),
                "Companion ownership changed"
            );
            let guide = b.guide.clone().or_else(|| a.guide.clone());
            *a = b.clone();
            a.guide = guide;
            a.ant = ant;
            a.chess = chess;
            a.imp = imp;
            a.clock = clock;
            a.resident = resident;
            a.model = model;
            matched.insert(index);
        }
        for (index, actor) in self.actors.iter_mut().enumerate() {
            if !matched.contains(&index) && actor.spawn.resident_spawn.is_some() {
                actor.resident = self.resident_initial[index].clone();
            }
            if !matched.contains(&index)
                && actor.spawn.clock_spawn.is_some()
                && actor.clock.is_some()
            {
                actor.clock = Some(crate::clockwork::Automaton::new(
                    actor.spawn.origin,
                    actor.spawn.yaw,
                    actor.spawn.scale,
                    index,
                    false,
                ));
                actor.footing = None;
                actor.yaw = actor.spawn.yaw;
            }
            if !matched.contains(&index) && actor.spawn.imp_spawn.is_some() && actor.imp.is_some() {
                let launch = actor.imp.as_ref().and_then(|p| p.launch);
                actor.imp = Some(crate::fire_imp::Imp::new(
                    actor.spawn.origin,
                    actor.spawn.yaw,
                    actor.spawn.scale,
                    index,
                    false,
                ));
                actor.imp.as_mut().unwrap().launch = launch;
                actor.footing = None;
                actor.yaw = actor.spawn.yaw;
            }
            if !matched.contains(&index) && actor.spawn.chess_spawn.is_some() {
                if let Some(current) = &actor.chess {
                    let mut p = crate::chess::Piece::new(
                        current.kind,
                        actor.spawn.origin,
                        actor.spawn.yaw,
                        actor.spawn.scale,
                        index,
                    );
                    p.active = false;
                    actor.chess = Some(p);
                    actor.footing = None;
                    actor.yaw = actor.spawn.yaw;
                }
            }
        }
        ensure!(
            self.actors
                .iter()
                .enumerate()
                .all(|(i, a)| matched.contains(&i)
                    || a.spawn.difficulty_variant
                    || a.spawn.chess_spawn.is_some()
                    || a.spawn.imp_spawn.is_some()
                    || a.spawn.clock_spawn.is_some()
                    || a.spawn.resident_spawn.is_some()),
            "Saved NPC cast does not match map"
        );
        if let Some(g)=&mut self.garden {g.restore(s.garden.as_ref(), &mut self.actors)?;}
        self.greetings = s.greetings;
        Ok(())
    }
    pub fn notarget(&mut self, value: bool) {
        for p in self.actors.iter_mut().filter_map(|a| a.resident.as_mut()) {
            p.notarget(value);
        }
        for p in self.actors.iter_mut().filter_map(|a| a.clock.as_mut()) {
            p.notarget = value;
        }
        for p in self.actors.iter_mut().filter_map(|a| a.imp.as_mut()) {
            p.notarget = value;
        }
        for p in self.actors.iter_mut().filter_map(|a| a.chess.as_mut()) {
            p.notarget = value;
        }
        for ant in self.actors.iter_mut().filter_map(|a| a.ant.as_mut()) {
            ant.notarget = value;
        }
        for guard in self.actors.iter_mut().filter_map(|a| a.guard.as_mut()) {
            guard.notarget = value;
        }
    }
    pub fn lights(&self) -> Vec<crate::lighting::Light> {
        self.actors
            .iter()
            .filter_map(|a| a.clock.as_ref())
            .flat_map(|p| p.fists.iter())
            .map(|f| crate::lighting::Light {
                position: f.position,
                color: vec3(1., 0.7, 0.2),
                radius: 170.,
                only_models: false,
                flare: false,
            })
            .chain(
                self.actors
                    .iter()
                    .filter_map(|a| a.resident.as_ref())
                    .filter(|p| p.active)
                    .flat_map(|p| match &p.body {
                        resident::Body::Insect(g)
                            if g.kind == crate::burrow::Kind::Larva && g.health > 0. =>
                        {
                            vec![crate::lighting::Light {
                                position: g.feet + Vec3::Z * 10.,
                                color: vec3(0.25, 0., 0.),
                                radius: 50.,
                                only_models: false,
                                flare: false,
                            }]
                        }
                        resident::Body::Card(g) => g
                            .shots
                            .iter()
                            .map(|s| crate::lighting::Light {
                                position: s.position,
                                color: vec3(1., 0.2, 0.2),
                                radius: 150.,
                                only_models: false,
                                flare: false,
                            })
                            .collect::<Vec<_>>(),
                        resident::Body::Snark(g) if g.kind == crate::snark::Kind::Fire => g
                            .shots
                            .iter()
                            .map(|s| crate::lighting::Light {
                                position: s.position,
                                color: vec3(1., 0.45, 0.1),
                                radius: 120.,
                                only_models: false,
                                flare: false,
                            })
                            .collect(),
                        _ => Vec::new(),
                    }),
            )
            .chain(self.actors.iter().filter_map(|a| {
                let p = a.resident.as_ref()?;
                let resident::Body::Card(g) = &p.body else {
                    return None;
                };
                let data = &self.models[a.model].data;
                if !p.active
                    || g.kind != crate::cards::Kind::Heart
                    || !(g.phase == crate::cards::Phase::Charge
                        || (g.phase == crate::cards::Phase::Slam
                            && g.time < 44. * data.clips[g.clip()].frame_time))
                {
                    return None;
                }
                let tag = crate::clockwork::Rig::tag(data, g.clip(), g.time, "tag_barrel");
                Some(crate::lighting::Light {
                    position: g.feet + Quat::from_rotation_z(g.yaw) * tag.translation * g.scale,
                    color: vec3(1., 0.2, 0.2),
                    radius: 100.,
                    only_models: false,
                    flare: false,
                })
            }))
            .collect()
    }
    pub fn chess_threats(&mut self, threatens: impl Fn(&combat::Target) -> bool) {
        for (id, actor) in self.actors.iter_mut().enumerate() {
            if let Some(p) = &mut actor.chess {
                p.threatened(threatens(&p.target(id)));
            }
        }
    }
    pub fn load(
        assets: &mut Assets,
        map: &Bsp,
        name: &str,
        entry: Option<&str>,
        preview: bool,
        combat_preview: bool,
    ) -> Result<Self> {
        let spawns = if combat_preview && name == "skool1" && entry.is_none() {
            vec![Spawn {
                resident_spawn: None,
                clock_spawn: None,
                imp_spawn: None,
                chess_spawn: None,
                difficulty_variant: false,
                name: String::new(),
                hidden: false,
                model: "cardguard_club".into(),
                origin: vec3(-1888., 2040., -504.),
                yaw: -std::f32::consts::FRAC_PI_2,
                scale: 1.,
                animation: None,
            }]
        } else {
            placements(map, name, entry, preview)
        };
        let specs = texture::read_materials(assets)?;
        let mut models = Vec::new();
        let mut indices = BTreeMap::new();
        let unique: BTreeSet<_> = spawns.iter().map(|s| &s.model).collect();
        for model in unique {
            let mut extra = spawns
                .iter()
                .filter(|s| &s.model == model)
                .filter_map(|s| s.animation.clone())
                .collect::<Vec<_>>();
            if matches!(
                (name, model.as_str()),
                ("hedge1", "c_insanechild_muzzle" | "c_insanechild_chase")
                    | ("funhouse", "c_insanechild-runner")
            ) {
                extra.extend(["idle", "walk", "run_panic"].map(str::to_string));
                if model == "c_insanechild_muzzle" {
                    extra.push("idle01".into());
                }
            }
            if model == "c_insanechild_muzzle" && name == "skool1" {
                extra.push("walk".into());
            }
            match Model::load(assets, model, &extra, &specs) {
                Ok(data) => {
                    indices.insert(model.clone(), models.len());
                    models.push(data);
                }
                Err(e) => eprintln!("NPC deferred {model}: {e:#}"),
            }
        }
        let mut residents = BTreeMap::new();
        if !preview {
            for (id, spawn) in spawns.iter().enumerate() {
                if !spawn.hidden
                    && !resident::legacy(name, &spawn.model)
                    && !crate::levels::serving(name, entry)
                        .any(|r| (r.owns_npc)(&spawn.name, &spawn.model))
                {
                    if let Some(p) = resident::Resident::new(spawn, map, name)? {
                        residents.insert(id, p);
                    }
                }
            }
        }
        let mut actors = spawns
            .into_iter()
            .enumerate()
            .filter_map(|(i, s)| {
                indices.get(&s.model).map(|&model| Actor {
                    resident: residents.remove(&i),
                    walk: None,
                    guide: if preview {
                        None
                    } else {
                        companions::State::new(name, &s)
                    },
                    watch: Default::default(),
                    story_visible: false,
                    speaking: false,
                    ending_talk: false,
                    mouth: 0.,
                    model,
                    clock: (s.model == crate::clockwork::MODEL
                        && !preview
                        && !s.hidden
                        && !crate::levels::serving(name, entry)
                            .any(|r| (r.owns_npc)(&s.name, &s.model)))
                    .then(|| {
                        crate::clockwork::Automaton::new(
                            s.origin,
                            s.yaw,
                            s.scale,
                            i,
                            s.clock_spawn.is_none(),
                        )
                    }),
                    imp: (s.model == crate::fire_imp::MODEL
                        && !preview
                        && !s.hidden
                        && !crate::levels::serving(name, entry)
                            .any(|r| (r.owns_npc)(&s.name, &s.model)))
                    .then(|| {
                        let mut imp = crate::fire_imp::Imp::new(
                            s.origin,
                            s.yaw,
                            s.scale,
                            i,
                            s.imp_spawn.is_none(),
                        );
                        imp.launch = imp_spawns::launch(map, name, &s);
                        imp
                    }),
                    chess: crate::chess::Kind::from_model(&s.model)
                        .filter(|_| {
                            !(preview
                                || s.hidden
                                || crate::levels::serving(name, entry)
                                    .any(|r| (r.owns_npc)(&s.name, &s.model)))
                        })
                        .map(|kind| {
                            let mut p = crate::chess::Piece::new(kind, s.origin, s.yaw, s.scale, i);
                            p.script_wait = crate::chess::script_wait(name, &s.name);
                            p.active = s.chess_spawn.is_none()
                                && !(name == "rchess1" && s.name.starts_with("king_"));
                            p
                        }),
                    ant: (crate::ant::is_ant(&s.model)
                        && !(preview
                            || s.hidden
                            || (name == "potears1" && crate::pool::cinema::owns(&s.name))
                            || crate::levels::serving(name, entry)
                                .any(|r| (r.owns_npc)(&s.name, &s.model))))
                    .then(|| {
                        let mut a =
                            crate::ant::Ant::new(i, s.model == "c_armyantcorp", s.origin, s.yaw);
                        a.scale = s.scale;
                        a.script_wait = crate::ant::script_wait(name, &s.name);
                        a
                    }),
                    guard: (models[model].data.combat.is_some() && !preview)
                        .then(|| Guard::new(s.origin, s.yaw, s.scale)),
                    yaw: s.yaw,
                    spawn: s,
                    time: i as f32 * 0.31,
                    greeting: 0.,
                    cooldown: 0.,
                    footing: None,
                    falling: 0.,
                    electric: 0.,
                })
            })
            .collect::<Vec<_>>();
        println!(
            "NPCs: {} placed actors, {} models; club guard combat enabled outside peaceful preview",
            actors.len(),
            models.len()
        );
        let garden = garden::Logic::load(map,name,&mut actors)?;
        Ok(Self {
            attack_fx: attack_fx::Art::load(assets, &specs)?,
            garden,
            resident_diamond: if actors.iter().any(|a| {
                a.resident
                    .as_ref()
                    .is_some_and(|p| p.model() == "cardguard_diamond")
            }) {
                Some(Prop::load_animation(assets, "prj_diamond", "idle", &specs)?)
            } else {
                None
            },
            thorn: if actors.iter().any(|a| a.spawn.model == "c_bloodrose") {
                Some(Prop::load_animation(assets, "prj_thorn", "idle", &specs)?)
            } else {
                None
            },
            spore: if actors.iter().any(|a| a.spawn.model == "c_evilmushroom") {
                crate::particles::Attached::load(assets, "prj_spore2", &specs)?
            } else {
                None
            },
            spores: BTreeMap::new(),
            resident_ambushes: resident_spawns::ambushes(map, name)?,
            actor_world: (name == "grounds2").then(|| World::actor_world(map)).transpose()?,
            resident_initial: actors.iter().map(|a| a.resident.clone()).collect(),
            acorn: if actors.iter().any(|a| {
                a.resident
                    .as_ref()
                    .is_some_and(|p| p.model() == "c_ladybug")
            }) {
                Some(Prop::load_animation(assets, "prj_acorn", "acorn", &specs)?)
            } else {
                None
            },
            chess_ambushes: chess_spawns::ambushes(map, name)?,
            imp_ambushes: imp_spawns::ambushes(map, name)?,
            clock_ambushes: clock_spawns::ambushes(map, name)?,
            card_art: BTreeMap::new(),
            snark_art: BTreeMap::new(),
            burrow_art: BTreeMap::new(),
            magma_art: BTreeMap::new(),
            wildlife_art: BTreeMap::new(),
            magma_fire: if actors.iter().any(|a| a.spawn.model == crate::magma::MODEL) {
                crate::particles::Attached::load(assets, "prj_fireball", &specs)?
            } else {
                None
            },
            snark_props: snark_art::Projectiles::load(assets, &actors, &specs)?,
            card_props: card_art::Projectiles::load(assets, &actors, &specs)?,
            clock_art: BTreeMap::new(),
            clock_prop: if actors.iter().any(|a| a.clock.is_some()) {
                Some(Prop::load(assets, "prj_hand", &specs)?)
            } else {
                None
            },
            clock_trail: if actors.iter().any(|a| a.clock.is_some()) {
                crate::particles::Attached::load(assets, "prj_hand", &specs)?
            } else {
                None
            },
            imp_debris: if actors.iter().any(|a| a.imp.is_some()) {
                [
                    "gb_meatbone1",
                    "gb_meatbone2",
                    "gb_meatbone3",
                    "gb_ribs",
                    "w_fork",
                ]
                .iter()
                .map(|n| Prop::load(assets, n, &specs))
                .collect::<Result<_>>()?
            } else {
                Vec::new()
            },
            pupil_path: ambient::path(map, name, entry)?,
            companion_routes: companions::Routes::load(map, name)?,
            scene_hidden: BTreeSet::new(),
            owns_village: name == "gvillage",
            owns_duchess: name == "potears3",
            owns_ladybugs: name == "potears1",
            owns_pandemonium: name == "pandemonium",
            owns_school_first: name == "skool1" && entry != Some("skool1_start2"),
            owns_school2: name == "skool2",
            levels: crate::levels::serving(name, entry).collect(),
            ant_shots: if actors.iter().any(|a| a.ant.is_some()) {
                Some([
                    crate::weapons::Prop::load_animation(assets, "prj_bullet", "idle", &specs)?,
                    crate::weapons::Prop::load_animation(assets, "prj_grenade", "idle", &specs)?,
                ])
            } else {
                None
            },
            models,
            actors,
            material: crate::character::skin_material()?,
            greetings: 0,
        })
    }
    pub fn summon(&mut self, target: Option<combat::Target>) {
        for a in &mut self.actors {
            if let Some(p) = &mut a.resident {
                p.opponents().summon = target;
            }
            if let Some(p) = &mut a.clock {
                p.opponents.summon = target;
            }
            if let Some(p) = &mut a.imp {
                p.opponents.summon = target;
            }
            if let Some(p) = &mut a.chess {
                p.opponents.summon = target;
            }
            if let Some(ant) = &mut a.ant {
                ant.opponents.summon = target;
            }
            if let Some(g) = &mut a.guard {
                g.opponents.summon = target;
            }
        }
    }
    pub fn provoke_summon(&mut self, id: usize) {
        if let Some(p) = self.actors.get_mut(id).and_then(|a| a.resident.as_mut()) {
            p.opponents().demon = true;
        }
        if let Some(p) = self.actors.get_mut(id).and_then(|a| a.clock.as_mut()) {
            p.opponents.demon = true;
        }
        if let Some(p) = self.actors.get_mut(id).and_then(|a| a.imp.as_mut()) {
            p.opponents.demon = true;
        }
        if let Some(p) = self.actors.get_mut(id).and_then(|a| a.chess.as_mut()) {
            p.opponents.demon = true;
        }
        if let Some(a) = self.actors.get_mut(id).and_then(|a| a.ant.as_mut()) {
            a.opponents.demon = true;
        }
        if let Some(g) = self.actors.get_mut(id).and_then(|a| a.guard.as_mut()) {
            g.opponents.demon = true;
        }
    }
    pub fn companions(&self) -> Vec<crate::level::CompanionContact> {
        self.actors.iter().filter_map(|a| a.guide.as_ref().map(|g| crate::level::CompanionContact {
            name: a.spawn.name.clone(), feet: a.position(), half: vec3(16.,16.,30.)*a.spawn.scale,
            visible: g.visible(), holding: g.holding(),
        })).collect()
    }
    pub fn level_patrols(&mut self, levels: &[crate::level::Slot]) {
        for level in levels {
            let names = level.ctl.patrols();
            if names.is_empty() { continue; }
            for actor in &mut self.actors {
                if names.contains(&actor.spawn.name.as_str()) {
                    if let Some(resident) = &mut actor.resident {
                        if let resident::Body::Ladybug(b) = &mut resident.body {
                            b.patrol_started = true;
                        }
                    }
                }
            }
        }
    }
    /// Idempotent map-driven spawns, shared by play and route verification.
    pub fn activate_levels(&mut self, levels: &[crate::level::Slot]) {
        let ids: BTreeSet<_> = levels.iter().flat_map(|l| l.ctl.active_npcs()).collect();
        for a in &mut self.actors {
            if a.spawn.clock_spawn.is_some_and(|id| ids.contains(&id)) {
                if let Some(c) = &mut a.clock { c.active = true; c.spawn_delay = None; }
            }
        }
    }
    pub fn update(&mut self, dt: f32, world: &World, eye: Vec3) -> combat::Feedback {
        let mut _profile = crate::frame_profile::span("npcs");
        if dt <= 0. { _profile.cancel(); }
        if let Some(mut actors) = self.actor_world.take() {
            actors.copy_dynamic_from(world);
            let result = self.update_cast(dt, &actors, eye);
            self.actor_world = Some(actors);
            result
        } else {
            self.update_cast(dt, world, eye)
        }
    }
    fn update_cast(&mut self, dt: f32, world: &World, eye: Vec3) -> combat::Feedback {
        let mut feedback = combat::Feedback::default();
        if let Some(g)=&mut self.garden {g.update(dt,world,eye,&mut self.actors,&self.models);}
        self.companion_routes
            .update(&mut self.actors, dt, world, eye, |index, clip| {
                let d = &self.models[index].data;
                d.clips[clip].distance * d.def.scale / d.clips[clip].duration()
            });
        if dt.is_finite() && dt > 0. {
            for ambush in &self.resident_ambushes {
                if !ambush.touches(eye) {
                    continue;
                }
                for actor in &mut self.actors {
                    if let Some(p) = &mut actor.resident {
                        if ambush.wake.contains(&actor.spawn.name) {
                            p.awake = true;
                        }
                        if ambush.patrols.contains(&actor.spawn.name) {
                            if let resident::Body::Ladybug(b) = &mut p.body {
                                b.patrol_started = true;
                            }
                        }
                    }

                    if let (Some(id), Some(p)) =
                        (actor.spawn.resident_spawn, actor.resident.as_mut())
                    {
                        if !p.active && p.delay.is_none() {
                            if let Some((_, delay)) =
                                ambush.receivers.iter().find(|(e, _)| *e == id)
                            {
                                p.delay = Some(*delay);
                            }
                        }
                    }
                }
            }
            for ambush in &self.clock_ambushes {
                if !ambush.touches(eye) {
                    continue;
                }
                for actor in &mut self.actors {
                    if let (Some(id), Some(imp)) = (actor.spawn.clock_spawn, actor.clock.as_mut()) {
                        if !imp.active && imp.spawn_delay.is_none() {
                            if let Some((_, delay)) =
                                ambush.receivers.iter().find(|(entity, _)| *entity == id)
                            {
                                imp.spawn_delay = Some(*delay);
                            }
                        }
                    }
                }
            }
            for ambush in &self.imp_ambushes {
                if !ambush.touches(eye) {
                    continue;
                }
                for actor in &mut self.actors {
                    if let (Some(id), Some(imp)) = (actor.spawn.imp_spawn, actor.imp.as_mut()) {
                        if !imp.active && imp.spawn_delay.is_none() {
                            if let Some((_, delay)) =
                                ambush.receivers.iter().find(|(entity, _)| *entity == id)
                            {
                                imp.spawn_delay = Some(*delay);
                            }
                        }
                    }
                }
            }
            for ambush in &self.chess_ambushes {
                if ambush.touches(eye) {
                    for actor in &mut self.actors {
                        if actor
                            .spawn
                            .chess_spawn
                            .is_some_and(|id| ambush.suppress.contains(&id))
                        {
                            if let Some(p) = &mut actor.chess {
                                if !p.active && p.spawn_delay.is_none() {
                                    p.suppressed = true;
                                }
                            }
                        }
                        if actor
                            .spawn
                            .chess_spawn
                            .is_some_and(|id| ambush.entities.contains(&id))
                        {
                            if let Some(p) = &mut actor.chess {
                                if !p.active && !p.suppressed && p.spawn_delay.is_none() {
                                    p.spawn_delay = Some(ambush.delay);
                                }
                            }
                        }
                    }
                }
            }
        }
        for actor in &mut self.actors {
            actor.electric = (actor.electric - dt.max(0.)).max(0.);
            if (self.owns_ladybugs && crate::pool::cinema::owns(&actor.spawn.name))
                || (self.owns_school_first && crate::school::cinema::owns(&actor.spawn.name))
                || (self.owns_school2 && (actor.spawn.model == "cardguard_diamond" || actor.spawn.name == "dice_cat"))
                || self
                    .levels
                    .iter()
                    .any(|r| (r.owns_npc)(&actor.spawn.name, &actor.spawn.model))
                || (self.owns_duchess
                    && matches!(
                        actor.spawn.name.as_str(),
                        "billthelizard" | "mockturtle" | "walkyrocks"
                    ))
                || (self.owns_ladybugs && actor.spawn.model == "c_ladybug")
                || (self.owns_pandemonium
                    && (actor.spawn.model.starts_with("cardguard_")
                        || matches!(
                            actor.spawn.name.as_str(),
                            "airship_gnome"
                                | "elder_gnome1"
                                | "leave_gnome"
                                | "minecart_torchgnome1"
                                | "minecart_torchgnome2"
                                | "minecart_torchgnome3"
                        )))
            {
                continue;
            }
            if actor.guide.is_some() {
                continue;
            }
            if !(crate::android::is_android()
                && actor.footing.is_some()
                && actor.falling == 0.
                && actor.position().distance_squared(eye) > 1200. * 1200.)
            {
                actor.ground(dt, world, &self.models[actor.model].data.meta);
            }
            if let Some(p) = &mut actor.resident {
                p.update(
                    dt,
                    world,
                    eye,
                    &self.models[actor.model].data,
                    &mut feedback,
                );
                actor.yaw = p.yaw();
                continue;
            }
            if let Some(p) = &mut actor.clock {
                p.update(
                    dt,
                    world,
                    eye,
                    &self.models[actor.model].data,
                    &mut feedback,
                );
                actor.yaw = p.yaw;
                continue;
            }
            if let Some(p) = &mut actor.imp {
                p.update(
                    dt,
                    world,
                    eye,
                    &self.models[actor.model].data,
                    &mut feedback,
                );
                actor.yaw = p.yaw;
                continue;
            }
            if let Some(piece) = &mut actor.chess {
                piece.update(
                    dt,
                    world,
                    eye,
                    &self.models[actor.model].data,
                    &mut feedback,
                );
                actor.yaw = piece.yaw;
                continue;
            }
            if let Some(ant) = &mut actor.ant {
                ant.update(
                    dt,
                    world,
                    eye,
                    &self.models[actor.model].data,
                    &mut feedback,
                );
                actor.yaw = ant.yaw;
                continue;
            }
            if let Some(guard) = &mut actor.guard {
                let events = guard.advance(
                    dt,
                    world,
                    eye,
                    self.models[actor.model].data.combat.unwrap(),
                );
                actor.yaw = guard.yaw;
                feedback.summon_hits.extend(events.summon_hits);
                feedback.damage += events.damage;
                feedback.impulse += events.impulse;
                feedback.spatial_sounds.extend(
                    events
                        .sounds
                        .into_iter()
                        .map(|s| (s, guard.target(0).center)),
                );
                feedback.spatial_sounds.extend(events.spatial_sounds);
            } else if actor.spawn.name == "return_insane2"
                && !self.pupil_path.is_empty()
                && actor.greeting <= 0.
                && !actor.speaking
            {
                let clip = &self.models[actor.model].data.clips["walk"];
                ambient::advance(
                    actor,
                    dt,
                    world,
                    &self.pupil_path,
                    clip.distance / clip.duration(),
                );
            } else {
                actor.tick(dt, world, eye);
            }
        }
        feedback
    }
    pub fn loot_sources(&self) -> Vec<crate::loot::Source> {
        if self.owns_pandemonium {
            return Vec::new();
        }
        self.actors
            .iter()
            .enumerate()
            .filter(|(_, a)| !self.registry_owns(a))
            .filter_map(|(id, a)| {
                if let Some(p) = &a.resident {
                    return p.active.then(|| crate::loot::Source {
                        id,
                        feet: p.position(),
                        dead: p.health() <= 0.,
                        grade: p.grade(),
                    });
                }
                if let Some(p) = &a.clock {
                    return p.active.then_some(crate::loot::Source {
                        id,
                        feet: p.feet,
                        dead: p.health <= 0.,
                        grade: crate::loot::Grade::Large,
                    });
                }
                if let Some(p) = &a.imp {
                    return p.active.then_some(crate::loot::Source {
                        id,
                        feet: p.feet,
                        dead: p.health <= 0.,
                        grade: crate::loot::Grade::Small,
                    });
                }
                if let Some(p) = &a.chess {
                    return p.active.then_some(crate::loot::Source {
                        id,
                        feet: p.feet,
                        dead: p.health <= 0.,
                        grade: p.kind.grade(),
                    });
                }
                if let Some(ant) = &a.ant {
                    return ant.enabled.then_some(crate::loot::Source {
                        id,
                        feet: ant.feet,
                        dead: ant.health <= 0.,
                        grade: ant.grade(),
                    });
                }
                a.guard.as_ref().map(|g| crate::loot::Source {
                    id,
                    feet: g.feet,
                    dead: g.health <= 0.,
                    grade: match a.spawn.model.as_str() {
                        "cardguard_heart" => crate::loot::Grade::Large,
                        "cardguard_spade" | "cardguard_diamond" => crate::loot::Grade::Medium,
                        _ => crate::loot::Grade::Small,
                    },
                })
            })
            .collect()
    }
    pub fn targets(&self) -> Vec<combat::Target> {
        self.actors
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                if self.owns_pandemonium || self.registry_owns(a) {
                    return None;
                }
                if let Some(ant) = &a.ant {
                    return (ant.enabled && ant.health > 0.).then(|| ant.target(i));
                }
                if let Some(p) = &a.resident {
                    return p.vulnerable().then(|| p.target(i));
                }
                if let Some(p) = &a.clock {
                    return (p.active && p.health > 0.).then(|| p.target(i));
                }
                if let Some(p) = &a.imp {
                    return (p.active && p.health > 0.).then(|| p.target(i));
                }
                if let Some(p) = &a.chess {
                    return (p.active && p.health > 0.).then(|| p.target(i));
                }
                a.guard
                    .as_ref()
                    .filter(|g| g.health > 0.)
                    .map(|g| g.target(i))
            })
            .collect()
    }
    /// Protected pupils are contact surfaces, never hostile/auto-aim targets.
    pub fn mallet_contacts(&self) -> Vec<combat::Target> {
        self.actors
            .iter()
            .enumerate()
            .filter_map(|(id, a)| {
                if !a.spawn.model.starts_with("c_insanechild")
                    || self.registry_owns(a)
                    || a.guide.as_ref().is_some_and(|g| !g.visible())
                    || self.scene_hidden.contains(&a.spawn.name)
                    || (a.spawn.hidden && !a.story_visible)
                {
                    return None;
                }
                let (min, max) = self.models[a.model].data.meta.bounds?;
                Some(combat::Target {
                    id,
                    center: a.position() + (min + max) * (0.5 * a.spawn.scale),
                    half: (max - min) * (0.5 * a.spawn.scale),
                })
            })
            .collect()
    }
    pub fn hit(&mut self, hit: combat::Hit) -> Option<(String, &'static str)> {
        let actor = self.actors.get_mut(hit.id)?;
        if let Some(p) = &mut actor.resident {
            return p.hit(hit).map(|s| {
                (
                    if p.health() <= 0. {
                        "Enemy defeated".into()
                    } else {
                        format!("Enemy hit / {:.0} health", p.health())
                    },
                    s,
                )
            });
        }
        if let Some(p) = &mut actor.clock {
            return p.hit(hit).map(|s| {
                (
                    if p.health == 0. {
                        "Clockwork Automaton defeated".into()
                    } else {
                        format!("Clockwork hit / {:.0} health", p.health)
                    },
                    s,
                )
            });
        }
        if let Some(p) = &mut actor.imp {
            let fork = self.models[actor.model].imp_fork(p);
            let old_health = p.health;
            let sound = p.hit(hit);
            if old_health > 0. && p.gibbed {
                if let Some(fork) = fork {
                    p.fork_origin(fork);
                }
            }
            return sound.map(|s| {
                (
                    if p.health == 0. {
                        "Fire Imp defeated".into()
                    } else {
                        format!("Fire Imp hit / {:.0} health", p.health)
                    },
                    s,
                )
            });
        }
        if let Some(p) = &mut actor.chess {
            let old = p.health;
            let sound = p.hit(hit)?;
            return Some((
                if p.health == 0. {
                    format!("{} defeated", p.kind.name())
                } else if old == p.health {
                    "Shield blocked the hit".into()
                } else {
                    format!("{} hit / {:.0} health", p.kind.name(), p.health)
                },
                sound,
            ));
        }
        if actor.spawn.model.starts_with("c_insanechild") {
            // Pupils retain their original protected health and zero knockback.
            // Direct contact still produces the shared electrocution shell.
            crate::electric::hit(&mut actor.electric, hit);
            return None;
        }
        if let Some(ant) = &mut actor.ant {
            let sound = ant.hit(hit)?;
            return Some((
                if ant.health == 0. {
                    "Ant defeated".into()
                } else {
                    format!("Ant hit / {:.0} health", ant.health)
                },
                sound,
            ));
        }
        let guard = actor.guard.as_mut()?;
        let sound = guard.hit(hit)?;
        println!(
            "Club guard hit: {} damage, {} health",
            hit.damage, guard.health
        );
        Some((
            if guard.health == 0. {
                "Card guard defeated".into()
            } else {
                format!("Card guard hit / {:.0} health", guard.health)
            },
            sound,
        ))
    }
    pub fn reset_combat(&mut self) {
        for a in &mut self.actors {
            if a.guard.is_some() {
                a.guard = Some(Guard::new(a.spawn.origin, a.spawn.yaw, a.spawn.scale));
            }
        }
    }
    /// A binding must name a drawn, available actor; model friendliness alone never
    /// creates a conversation. Controller-owned actors provide their own drawn pose.
    pub fn talk_target(&self, name: &str, world: &World, eye: Vec3, aim: Vec3) -> Option<Vec3> {
        self.actors
            .iter()
            .find(|a| {
                a.spawn.name == name
                    && !self.owns_duchess
                    && !self.registry_owns(a)
                    && !self.scene_hidden.contains(name)
                    && (!a.spawn.hidden || a.story_visible)
                    && self.models[a.model].data.friendly
                    && a.can_greet(world, eye, aim)
            })
            .map(Actor::target)
    }
    pub fn begin_talk(&mut self, name: &str) {
        if let Some(actor) = self.actors.iter_mut().find(|a| a.spawn.name == name) {
            actor.greeting = 3.;
            actor.ending_talk = false;
            actor.cooldown = 4.;
            actor.time = 0.;
        }
        self.greetings += 1;
    }
    pub fn story_pose(&mut self, story: &crate::story::Story, dt: f32, world: &World, eye: Vec3) {
        let cast = story.cast();
        let speaker = story.line().map(|l| l.actor.as_str());
        let targets: BTreeMap<_, _> = self
            .actors
            .iter()
            .map(|a| (a.spawn.name.clone(), a.target()))
            .collect();
        for actor in &mut self.actors {
            let target = story
                .line()
                .and_then(|l| l.watches.get(&actor.spawn.name))
                .and_then(|name| {
                    if crate::story::speakers::is_alice(name) {
                        Some(eye)
                    } else {
                        targets.get(name).copied()
                    }
                });
            actor.watch.update(
                dt,
                target.map(|p| Quat::from_rotation_z(-actor.yaw) * (p - actor.target())),
            );
            actor.story_visible = cast.contains(&actor.spawn.name.as_str());
            // Advancing the last line must not immediately reopen the whole
            // conversation. Keep this existing saved cooldown armed throughout
            // automatic conversations too; gameplay time releases it afterward.
            if self.owns_village && actor.story_visible && self.models[actor.model].data.friendly {
                actor.cooldown = actor.cooldown.max(1.);
            }
            let speaking = speaker == Some(actor.spawn.name.as_str());
            if speaking != actor.speaking {
                actor.time = 0.;
                actor.ending_talk = !speaking;
                actor.greeting = 0.;
            }
            actor.speaking = speaking;
            actor.mouth = story.mouth(&[&actor.spawn.name]);
            // Normal NPC simulation pauses during dialogue; advance only the cast's
            // presentation with the same pause-aware clock as the subtitle.
            if actor.story_visible {
                actor.time += dt;
                actor.face(dt, world, eye, true);
            }
        }
    }
    pub fn draw(
        &mut self,
        camera: Vec3,
        direction: Vec3,
        atmosphere: &Atmosphere,
        fullbright: bool,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        for (id, actor) in self.actors.iter().enumerate() {
            if actor.ant.as_ref().is_some_and(|a| !a.enabled) { continue; }
            if actor.guide.as_ref().is_some_and(|g| !g.visible()) {
                continue;
            }
            if self.scene_hidden.contains(&actor.spawn.name) {
                continue;
            }
            if self.registry_owns(actor) {
                continue;
            }
            if (self.owns_duchess
                && matches!(
                    actor.spawn.name.as_str(),
                    "billthelizard" | "mockturtle" | "walkyrocks"
                ))
                || (self.owns_ladybugs && actor.spawn.model == "c_ladybug")
                || (self.owns_pandemonium
                    && (actor.spawn.model.starts_with("cardguard_")
                        || matches!(
                            actor.spawn.name.as_str(),
                            "airship_gnome"
                                | "elder_gnome1"
                                | "leave_gnome"
                                | "minecart_torchgnome1"
                                | "minecart_torchgnome2"
                                | "minecart_torchgnome3"
                        )))
            {
                continue;
            }
            if (actor.spawn.hidden
                || (self.owns_pandemonium
                    && matches!(actor.spawn.name.as_str(), "rope_cat" | "cards_cat")))
                && !actor.story_visible
            {
                continue;
            }
            if let Some(p) = &actor.resident {
                if let resident::Body::Card(g) = &p.body {
                    if p.active {
                        let art = self.card_art.entry(id).or_insert_with(|| {
                            card_art::Art::new(self.models[actor.model].own_effect.as_ref())
                        });
                        if let Some(props) = self.card_props.get_mut(&g.kind) {
                            art.shots(g, props, camera, atmosphere, fullbright, &self.material);
                        }
                    }
                }
                if let resident::Body::Wildlife(g) = &p.body {
                    if p.active {
                        self.wildlife_art
                            .entry(id)
                            .or_insert_with(|| wildlife_art::Art::new(&self.models[actor.model]))
                            .draw(g, &self.models[actor.model], camera, atmosphere, &self.attack_fx);
                    }
                }
                if let resident::Body::Magma(g) = &p.body {
                    if p.active {
                        self.magma_art
                            .entry(id)
                            .or_insert_with(|| {
                                magma_art::Art::new(self.models[actor.model].own_effect.as_ref())
                            })
                            .draw(
                                g,
                                &self.models[actor.model].data,
                                self.magma_fire.as_ref(),
                                camera,
                                atmosphere,
                            );
                    }
                }
                if let resident::Body::Insect(g) = &p.body {
                    if p.active {
                        self.burrow_art
                            .entry(id)
                            .or_insert_with(|| {
                                burrow_art::Art::new(self.models[actor.model].own_effect.as_ref())
                            })
                            .draw(g, &self.models[actor.model].data, camera, atmosphere);
                    }
                }
                if let resident::Body::Snark(g) = &p.body {
                    if p.active {
                        let art = self.snark_art.entry(id).or_insert_with(|| {
                            snark_art::Art::new(self.models[actor.model].own_effect.as_ref())
                        });
                        if let Some(props) = self.snark_props.get_mut(&g.kind) {
                            art.shots(g, props, camera, atmosphere, fullbright, &self.material);
                        }
                        art.tongue(g, &self.models[actor.model].data);
                    }
                }
                if let resident::Body::Plant(plant) = &p.body {
                    self.spores.retain(|(owner, serial), _| {
                        *owner != id || plant.shots.iter().any(|s| s.serial == *serial)
                    });
                    for shot in &plant.shots {
                        let transform = Transform {
                            translation: shot.position,
                            rotation: Quat::from_rotation_arc(Vec3::X, shot.direction),
                        };
                        if plant.kind == crate::plants::Kind::Rose {
                            if let Some(prop) = &mut self.thorn {
                                self.material.bind();
                                prop.draw_frame(transform, 1., fullbright, shot.age, true);
                            }
                        } else if let Some(template) = &self.spore {
                            let effect = self
                                .spores
                                .entry((id, shot.serial))
                                .or_insert_with(|| template.fork());
                            effect.draw(
                                shot.age,
                                1.,
                                |at, _| Transform {
                                    translation: shot.position
                                        - shot.direction * 600. * (shot.age - at),
                                    ..transform
                                },
                                |_, _, default| default,
                                camera,
                                atmosphere,
                            );
                        }
                    }
                }
                p.effects(&mut self.acorn, &mut self.resident_diamond, fullbright);
                self.material.bind();
            }
            if let (Some(p), Some(prop)) = (&actor.clock, &mut self.clock_prop) {
                let art = self.clock_art.entry(id).or_insert_with(|| {
                    clock_art::Art::new(
                        self.models[actor.model].own_effect.as_ref(),
                        self.clock_trail.as_ref(),
                    )
                });
                art.shots(p, prop, camera, atmosphere, fullbright, &self.material);
            }
            if let (Some(ant), Some(props)) = (&actor.ant, &mut self.ant_shots) {
                for shot in &ant.shots {
                    crate::ant::draw_shot(props, shot, fullbright);
                }
                crate::ant::draw_blasts(ant);
                self.material.bind();
            }
            if let Some(p) = &actor.imp {
                if let Some(parts) = &p.fragments {
                    self.material.bind();
                    for (prop, part) in self.imp_debris.iter_mut().zip(parts) {
                        prop.draw(
                            Transform {
                                translation: part.position,
                                rotation: part.rotation,
                            },
                            p.scale * (5. - p.time).clamp(0., 1.),
                            fullbright,
                        );
                    }
                }
            }
            if let Some(beam) = actor.chess.as_ref().and_then(|p| p.beam.as_ref()) {
                self.attack_fx.lightning(beam.from, beam.to, camera, beam.age, 500., true, atmosphere);
                self.material.bind();
            }
            let max_dist = 3500. * crate::android::active_preset().lod_distance_scale();
            let offset = actor.target() - camera;
            if offset.length_squared() > max_dist * max_dist
                || (offset.length_squared() > 200. * 200.
                    && offset.normalize().dot(direction) < -0.3)
            {
                continue;
            }
            let model = &mut self.models[actor.model];
            if let Some(p) = &actor.resident {
                p.draw(model, fullbright, &self.material);
                if p.active {
                    if let resident::Body::Snark(g) = &p.body {
                        if let Some(art) = self.snark_art.get_mut(&id) {
                            art.smoke(g, &model.data, camera, atmosphere);
                        }
                    }
                }
                if p.active {
                    if let resident::Body::Card(g) = &p.body {
                        if let Some(art) = self.card_art.get_mut(&id) {
                            art.charge(g, &model.data, camera, atmosphere);
                        }
                    }
                }
                self.material.bind();
                continue;
            }
            if let Some(p) = &actor.clock {
                model.draw_clock(p, fullbright, &self.material);
                if let Some(art) = self.clock_art.get_mut(&id) {
                    art.steam(p, &model.data, camera, atmosphere);
                }
                self.material.bind();
                continue;
            }
            if let Some(p) = &actor.imp {
                model.draw_imp(p, fullbright, &self.material);
                continue;
            }
            if let Some(piece) = &actor.chess {
                model.draw_chess(piece, fullbright, &self.material);
                continue;
            }
            if let Some(ant) = &actor.ant {
                model.draw_ant(ant, fullbright, &self.material);
                continue;
            }
            if let Some(guard) = &actor.guard {
                model.draw_guard(guard, fullbright, camera, atmosphere, &self.material);
                continue;
            }
            let d = &model.data;
            let idle = actor
                .spawn
                .animation
                .as_ref()
                .filter(|n| d.clips.contains_key(*n))
                .unwrap_or(&d.idle);
            let idle = if let Some(g) = &actor.guide {
                g.clip()
            } else if actor.walk.is_some() {
                "walk"
            } else {
                idle
            };
            let (mut local, clip, at, looping) = d.performance(
                idle,
                actor.greeting > 0. || actor.speaking,
                actor.ending_talk,
                actor.time,
            );
            actor.watch.apply(&d.skeleton, &mut local);
            let animation = &d.clips[&clip];
            let visual = d.events.visual(
                &clip,
                at,
                animation.duration(),
                animation.frame_time,
                looping,
            );
            let transform = Transform {
                translation: actor.position(),
                rotation: Quat::from_rotation_z(actor.yaw),
            };
            model.draw_pose(
                &local,
                actor.mouth,
                actor.time,
                true,
                transform,
                actor.spawn.scale,
                fullbright,
                true,
                &visual,
                None,
            );
            model.draw_effects(
                &clip,
                at,
                looping,
                transform,
                actor.spawn.scale,
                camera,
                atmosphere,
            );
            model.draw_electric(actor.electric);
            self.material.bind();
        }
        gl_use_default_material();
    }
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    #[serde(default, skip_serializing_if="Option::is_none")]
    garden: Option<garden::Saved>,
    actors: Vec<Actor>,
    greetings: u64,
}
impl Snapshot {
    /// The cast-identity half of `Npcs::restore` without loading any model: every saved actor
    /// must still be an authored placement of this visit (the identity old saves rely on), keep
    /// valid timers, and every current placement must be saved unless it is a difficulty
    /// variant. It assumes every placement's model loads, as it does in a normal run.
    pub fn matches_placements(&self, map: &Bsp, name: &str, entry: Option<&str>) -> Result<()> {
        let spawns = placements(map, name, entry, false);
        let mut matched = BTreeSet::new();
        for b in &self.actors {
            let (index, _) = spawns
                .iter()
                .enumerate()
                .find(|(i, s)| !matched.contains(i) && s.saved_identity(&b.spawn, matches!(name, "hedge1" | "tower1")))
                .context("Saved NPC identity does not match map")?;
            b.validate_save()?;
            matched.insert(index);
        }
        ensure!(
            spawns.iter().enumerate().all(|(i, s)| matched.contains(&i)
                || s.difficulty_variant
                || s.chess_spawn.is_some()
                || s.imp_spawn.is_some()
                || s.clock_spawn.is_some()
                || s.resident_spawn.is_some()),
            "Saved NPC cast does not match map"
        );
        Ok(())
    }
    /// Whether a registered controller owns any actor of this saved cast: the cast that save
    /// describes was made before the controller took those actors over (F2 step 7).
    pub fn owned_by(&self, registrations: &[&'static crate::levels::Registration]) -> bool {
        self.actors.iter().any(|a| {
            registrations
                .iter()
                .any(|r| (r.owns_npc)(&a.spawn.name, &a.spawn.model))
        })
    }
    /// The saved cast after registered controllers took actors over. `fresh` is the cast a new
    /// visit spawns, which `Npcs::restore` then accepts exactly. Owned actors, and actors the
    /// save lacks, start fresh; every other actor keeps its saved state, matched by its authored
    /// placement like `Npcs::restore` does. A saved actor no placement matches is dropped.
    pub fn adopt(
        &self,
        fresh: Snapshot,
        registrations: &[&'static crate::levels::Registration],
    ) -> Snapshot {
        let mut taken = BTreeSet::new();
        let actors = fresh
            .actors
            .into_iter()
            .map(|new| {
                let owned = registrations
                    .iter()
                    .any(|r| (r.owns_npc)(&new.spawn.name, &new.spawn.model));
                let saved = (!owned)
                    .then(|| {
                        self.actors
                            .iter()
                            .enumerate()
                            .find(|(i, a)| !taken.contains(i) && a.spawn == new.spawn)
                    })
                    .flatten();
                match saved {
                    Some((index, saved)) => {
                        taken.insert(index);
                        // The model index belongs to this load; everything else is saved state.
                        Actor {
                            model: new.model,
                            ..saved.clone()
                        }
                    }
                    None => new,
                }
            })
            .collect();
        Snapshot {
            garden: self.garden.clone(),
            actors,
            greetings: self.greetings,
        }
    }
    pub fn import_guards(&self, encounters: &mut crate::encounters::Encounters) {
        for old in &self.actors {
            if let Some(saved) = &old.guard {
                for actor in &mut encounters.actors {
                    if let crate::encounters::Enemy::Guard(g) = &mut actor.enemy {
                        if old.spawn.origin.distance(g.feet) < 1. {
                            *g = saved.clone();
                        }
                    }
                }
            }
        }
    }
}
pub fn check_combat(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let interactions = crate::interaction::Interactions::load(&map)?;
    interactions.sync(&mut world);
    let data = Data::load(assets, "cardguard_club", &[])?;
    let timing = data.combat.context("Missing club guard combat clips")?;
    ensure!(
        timing.hit < timing.attack,
        "Strike falls outside attack clip"
    );
    let eye = vec3(-1888., 1752., -464.);
    let mut results = Vec::new();
    for fps in [30, 60, 144] {
        let mut guard = Guard::new(vec3(-1888., 2040., -504.), -std::f32::consts::FRAC_PI_2, 1.);
        let mut damage = 0.;
        for _ in 0..fps * 12 {
            damage += guard.advance(1. / fps as f32, &world, eye, timing).damage;
        }
        ensure!(
            damage > 0.,
            "Guard never reached or struck Alice at {fps} Hz"
        );
        ensure!(
            guard.feet.truncate().distance(eye.truncate()) < 65.,
            "Guard failed pursuit"
        );
        let targets = [guard.target(0)];
        let ctx = combat::Context {
            world: &world,
            targets: &targets,
        };
        let direction = (targets[0].center - eye).normalize();
        ensure!(
            combat::contact(&ctx, eye, eye + direction * 65., 2.).is_some(),
            "Blade cannot reach guard"
        );
        results.push((guard.feet, damage));
        for _ in 0..3 {
            guard.hurt(combat::weapon_damage(0, false));
        }
        ensure!(
            guard.health == 0. && guard.state == combat::State::Dead,
            "Guard survives three Blade hits"
        );
        println!("PASS school encounter {fps} Hz: pursuit, {damage} outgoing damage, Blade contact, defeat");
    }
    for r in &results[1..] {
        ensure!(
            (r.0 - results[0].0).length() < 0.1 && r.1 == results[0].1,
            "Frame-rate-dependent encounter"
        );
    }
    println!("PASS club guard: alert/walk/attack/pain/death clips and school floor encounter");
    Ok(())
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(assets)?;
    let mut models: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut actors = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let spawns = placements(&map, &name, None, false);
        println!("{name}: {} eligible placements", spawns.len());
        actors += spawns.len();
        for s in spawns {
            let names = models.entry(s.model).or_default();
            if let Some(a) = s.animation {
                names.insert(a);
            }
        }
    }
    let mut valid = 0;
    let mut failed = 0;
    let mut clips = 0;
    for (name, extra) in &models {
        let checked = (|| -> Result<usize> {
            let data = Data::load(assets, name, &extra.iter().cloned().collect::<Vec<_>>())?;
            for s in &data.skeleton.surfaces {
                if data.meta.hidden.contains(&s.name) || data.meta.hidden.contains("all") {
                    continue;
                }
                let skin = data
                    .def
                    .skins
                    .get(&s.name)
                    .or_else(|| data.def.skins.get("all"))
                    .context("Missing skin")?;
                let path = texture::resolve(assets, &format!("{}/{skin}", data.def.path), &specs)
                    .or_else(|| texture::resolve(assets, skin, &specs))
                    .context("Unresolved NPC skin")?;
                texture::decode(assets, &path)?;
            }
            for (attachment, tag, _) in &data.meta.attachments {
                ensure!(
                    data.skeleton.bones.iter().any(|b| &b.name == tag),
                    "Missing attachment bone"
                );
                let (def, model) = crate::weapons::read_model(assets, attachment)?;
                for surface in &model.surfaces {
                    let skin = def
                        .skins
                        .get(&surface.name)
                        .or_else(|| def.skins.get("all"))
                        .context("Missing attachment skin")?;
                    let path = texture::resolve(assets, &format!("{}/{skin}", def.path), &specs)
                        .or_else(|| texture::resolve(assets, skin, &specs))
                        .context("Unresolved attachment texture")?;
                    texture::decode(assets, &path)?;
                }
            }
            for clip in data.clips.values() {
                for frame in &clip.frames {
                    let pose = data.skeleton.global_pose(&frame.pose);
                    ensure!(
                        data.skeleton
                            .surfaces
                            .iter()
                            .flat_map(|s| &s.vertices)
                            .all(|v| v.position(&pose).is_finite()),
                        "Invalid NPC skinning"
                    );
                }
            }
            let listed = OFF_NORM_TAGS.iter().find(|(model, ..)| model == name);
            let allowed: BTreeSet<&str> = listed
                .map(|(_, tags, _)| tags.iter().copied().collect())
                .unwrap_or_default();
            let found: BTreeSet<&str> = data.off_norm_tags.iter().map(String::as_str).collect();
            ensure!(
                found.is_subset(&allowed),
                "Off-norm tag rotations {found:?} are not covered by the OFF_NORM_TAGS entry {allowed:?}"
            );
            ensure!(
                listed.is_none() || !found.is_empty(),
                "Stale OFF_NORM_TAGS entry: no loaded clip needs the tolerance"
            );
            if let Some((_, _, reason)) = listed {
                println!("ALLOWED NPC {name}: renormalised tag bones {found:?}: {reason}");
            }
            Ok(data.clips.len())
        })();
        match checked {
            Ok(count) => {
                valid += 1;
                clips += count;
                println!("OK NPC {name}: {count} clips");
            }
            Err(e) => {
                println!("DEFERRED NPC {name}: {e:#}");
                failed += 1;
            }
        }
    }
    ensure!(
        failed == 0 && valid > 0,
        "{failed} unexpected NPC validation failures"
    );
    println!("PASS NPC audit: {actors} eligible placements, {valid}/{} supported models, {clips} loaded clips",models.len());
    Ok(())
}

/// Read-only placement audit against each map's current solid geometry. This
/// does not imply that an unsupported map's scripted cast has been restored.
pub fn check_placement(assets: &mut Assets) -> Result<()> {
    let mut supported = 0;
    let mut airborne = 0;
    let mut unresolved = 0;
    let mut moved = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let mut world = World::from_bsp(&map)?;
        let mut interactions = crate::interaction::Interactions::load(&map)?;
        interactions.set_entry(assets, &map, &name, None)?;
        interactions.sync(&mut world);
        for s in placements(&map, &name, None, false) {
            let meta = metadata(&String::from_utf8_lossy(
                &assets.read(&format!("models/{}.tik", s.model))?,
            ))?;
            if meta.airborne {
                airborne += 1;
                continue;
            }
            let (min, max) = meta
                .bounds
                .unwrap_or((vec3(-16., -16., 0.), vec3(16., 16., 60.)));
            let center = (min + max) * 0.5 * s.scale;
            let half = (max - min) * 0.5 * s.scale;
            if let Some(p) = world.actor_footing(s.origin, center, half, 1024.) {
                let trace = world.sweep(p + center, p + center - Vec3::Z, half);
                ensure!(
                    !trace.start_solid && trace.fraction < 1. && trace.normal.z >= 0.65,
                    "Actor did not reach clear support: {name}/{}",
                    s.name
                );
                supported += 1;
                if (p.z - s.origin.z).abs() > 1. {
                    moved += 1;
                    println!(
                        "SETTLED {name}/{} {}: {:?} -> {:?}",
                        s.name, s.model, s.origin, p
                    );
                }
            } else {
                unresolved += 1;
                println!("UNRESOLVED {name}/{} {} {:?}", s.name, s.model, s.origin);
                if name.starts_with("centipede") {
                    let t = world.sweep(s.origin, s.origin - Vec3::Z * 1024., Vec3::splat(0.5));
                    let clear = [0., 8., 16., 32., 64., 128.].map(|lift| {
                        !world
                            .sweep(
                                s.origin + center + Vec3::Z * lift,
                                s.origin + center + Vec3::Z * lift,
                                half,
                            )
                            .start_solid
                    });
                    println!("  SUPPORT point {t:?}, clear at lift 0/8/16/32/64/128 {clear:?}");
                }
            }
        }
    }
    ensure!(
        supported > 0 && moved > 0 && airborne > 0,
        "Empty actor placement audit"
    );
    println!("PASS actor support audit: {supported} supported ({moved} corrected), {airborne} flyers/swimmers preserved, {unresolved} editor placements without safe support");
    Ok(())
}

/// Explicit staged views of the reported maps; does not touch campaign saves.
pub async fn render_placement(assets: &mut Assets) -> Result<()> {
    ambient::render_check(assets).await?;
    for (map, name) in [
        ("gvillage", "torchgnome1"),
        ("gvillage", "torchgnome3"),
        ("centipede1", "ant_runner1"),
        ("centipede2", "ant_guard2"),
        ("skool1", "return_insane2"),
    ] {
        let mut scene = crate::render::Scene::load(assets, map)?;
        let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
        interactions.set_entry(assets, &scene.map, map, None)?;
        interactions.sync(&mut scene.world);
        let mut npcs = Npcs::load(assets, &scene.map, map, None, false, false)?;
        npcs.update(0., &scene.world, Vec3::splat(100000.));
        ensure!(
            npcs.actors.iter().any(|a| a.spawn.name == name),
            "Missing placement fixture actor"
        );
        // Isolate the inspected actor so a neighbouring ant cannot obscure the
        // feet or face. The authored world and this actor's placement are intact.
        npcs.actors.retain(|a| a.spawn.name == name);
        let index = 0;
        let target = npcs.actors[index].target();
        let mut candidates = (0..16).filter_map(|i| {
            let angle = i as f32 * std::f32::consts::TAU / 16.;
            let eye = target + vec3(angle.cos() * 155., angle.sin() * 155., 24.);
            let t = scene.world.sweep(target, eye, Vec3::splat(2.));
            (!t.start_solid && t.fraction >= 1.).then_some(eye)
        });
        let eye = candidates.next().context("No clear actor fixture camera")?;
        let target_yaw = (eye.y - target.y).atan2(eye.x - target.x);
        npcs.actors[index].yaw = target_yaw + std::f32::consts::PI;
        for _ in 0..240 {
            npcs.update(1. / 60., &scene.world, eye);
        }
        let a = &npcs.actors[index];
        ensure!(
            a.walk.is_some()
                || vec2(a.yaw.cos(), a.yaw.sin()).dot((eye - a.target()).truncate().normalize())
                    > 0.99,
            "Actor did not turn toward Alice: {map}/{name}"
        );
        let identity = a.spawn.clone();
        let saved = npcs.snapshot();
        npcs.restore(&saved)?;
        ensure!(
            npcs.actors[index].spawn == identity,
            "Grounding changed saved identity"
        );
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 65_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            let transforms = interactions.transforms();
            scene.draw(eye, 0., false, false, &transforms);
            npcs.draw(eye, (target - eye).normalize(), &scene.atmosphere, false);
            crate::render::depth_read_only(|| scene.draw(eye, 0., false, true, &transforms));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/actor-{map}-{name}.png"
                )))?;
            }
            next_frame().await;
        }
        println!(
            "PASS staged actor {map}/{name}: {:?} -> {:?}, facing and saved identity retained",
            identity.origin,
            npcs.actors[index].position()
        );
    }
    Ok(())
}

impl crate::ant::Timing for Data {
    fn duration(&self, _: &str, clip: &str) -> f32 {
        self.clips[clip].duration()
    }
    fn frame(&self, _: &str, clip: &str) -> f32 {
        self.clips[clip].frame_time
    }
    fn speed(&self, _: &str, clip: &str) -> f32 {
        let c = &self.clips[clip];
        (c.distance * self.def.scale / c.duration()).max(1.)
    }
}
impl crate::clockwork::Rig for Data {
    fn tag(&self, clip: &str, time: f32, tag: &str) -> Transform {
        let bone = self
            .skeleton
            .bones
            .iter()
            .position(|b| b.name == tag)
            .or_else(|| {
                (tag == "tag_barrel")
                    .then_some(self.card_muzzle.map(|(bone, _)| bone))
                    .flatten()
            })
            .expect("Actor tag validated at load");
        let pose = self.skeleton.global_pose(&self.clips[clip].sample(
            time,
            matches!(
                clip,
                "idle" | "ready" | "walk_slow" | "walk_norm" | "walk_fast"
            ),
        ));
        Transform {
            translation: pose[bone].translation * self.def.scale
                + self
                    .card_muzzle
                    .filter(|(b, _)| *b == bone && tag == "tag_barrel")
                    .map_or(Vec3::ZERO, |(_, p)| pose[bone].rotation * p),
            rotation: pose[bone].rotation,
        }
    }
}
impl Model {
    fn imp_fork(&self, p: &crate::fire_imp::Imp) -> Option<Transform> {
        let bone = self
            .data
            .skeleton
            .bones
            .iter()
            .position(|b| b.name == "tag_fork")?;
        let pose = self
            .data
            .skeleton
            .global_pose(&self.data.clips[p.clip()].sample(p.time, p.loops()));
        let rotation = Quat::from_rotation_z(p.yaw);
        Some(Transform {
            translation: p.feet + rotation * pose[bone].translation * self.data.def.scale * p.scale,
            rotation: rotation * pose[bone].rotation,
        })
    }
    fn draw_clock(
        &mut self,
        piece: &crate::clockwork::Automaton,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        let Some(transform) = piece.visual(&self.data) else {
            return;
        };
        let scale = piece.scale;
        let clip = &self.data.clips[piece.clip()];
        let time = if piece.frozen {
            piece.time.min(clip.frame_time * 8.)
        } else {
            piece.time
        };
        let local = clip.sample(time, piece.loops());
        let visual = self.data.events.visual(
            piece.clip(),
            time,
            clip.duration(),
            clip.frame_time,
            piece.loops(),
        );
        material.bind();
        self.draw_pose(
            &local,
            0.,
            piece.time,
            piece.health > 0.,
            transform,
            scale,
            fullbright,
            true,
            &visual,
            None,
        );
        if piece.frozen {
            self.draw_frozen();
        }
    }
    fn draw_imp(
        &mut self,
        piece: &crate::fire_imp::Imp,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        let scale = piece.visual_scale(&self.data);
        if scale <= 0. {
            return;
        }
        let clip = &self.data.clips[piece.clip()];
        let time = if piece.frozen {
            piece.time.min(clip.frame_time * 5.)
        } else {
            piece.time
        };
        let local = clip.sample(time, piece.loops());
        let visual = self.data.events.visual(
            piece.clip(),
            time,
            clip.duration(),
            clip.frame_time,
            piece.loops(),
        );
        material.bind();
        self.draw_pose(
            &local,
            0.,
            piece.time,
            piece.health > 0.,
            Transform {
                translation: piece.feet,
                rotation: Quat::from_rotation_z(piece.yaw),
            },
            scale,
            fullbright,
            true,
            &visual,
            None,
        );
        if piece.frozen {
            self.draw_frozen();
        }
    }
    fn draw_chess(
        &mut self,
        piece: &crate::chess::Piece,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        let scale = piece.visual_scale(&self.data);
        if scale <= 0. {
            return;
        }
        let clip = &self.data.clips[piece.clip()];
        let time = if piece.frozen {
            piece.time.min(clip.frame_time * 3.)
        } else {
            piece.time
        };
        let local = clip.sample(time, piece.loops());
        let visual = self.data.events.visual(
            piece.clip(),
            time,
            clip.duration(),
            clip.frame_time,
            piece.loops(),
        );
        material.bind();
        self.draw_pose(
            &local,
            0.,
            piece.time,
            piece.health > 0.,
            Transform {
                translation: piece.feet,
                rotation: Quat::from_rotation_z(piece.yaw),
            },
            scale,
            fullbright,
            true,
            &visual,
            None,
        );
        if piece.frozen {
            self.draw_frozen();
        }
    }
    fn draw_ant(
        &mut self,
        ant: &crate::ant::Ant,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        let scale = ant.visual_scale(&self.data);
        if scale <= 0. {
            return;
        }
        let clip = &self.data.clips[ant.clip()];
        let time = if ant.frozen {
            ant.time.min(clip.frame_time * 5.)
        } else {
            ant.time
        };
        let local = clip.sample(time, ant.loops());
        let visual = self.data.events.visual(
            ant.clip(),
            time,
            clip.duration(),
            clip.frame_time,
            ant.loops(),
        );
        material.bind();
        self.draw_pose(
            &local,
            0.,
            ant.time,
            ant.health > 0.,
            Transform {
                translation: ant.feet,
                rotation: Quat::from_rotation_z(ant.yaw),
            },
            scale,
            fullbright,
            true,
            &visual,
            None,
        );
        if ant.frozen {
            self.draw_frozen();
        }
        self.draw_electric(ant.electric);
    }
}

impl crate::cards::Rig for Data {
    fn sever(&self) -> Option<&crate::dismember::Recipe> {
        self.card_cut.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maze_precache_upgrade_accepts_only_the_reviewed_hidden_flag() {
        let mut current = actor().spawn;
        current.origin = vec3(0., 6500., 968.);
        current.hidden = true;
        let mut old = current.clone();
        old.hidden = false;
        assert!(current.saved_identity(&old, true));
        assert!(!current.saved_identity(&old, false));
        old.origin.x += 1.;
        assert!(!current.saved_identity(&old, true));
        old = current.clone();
        old.model.push('x');
        assert!(!current.saved_identity(&old, true));
        current.origin.y = 4288.;
        old = current.clone();
        old.hidden = false;
        assert!(!current.saved_identity(&old, true));
    }
    #[test]
    fn off_norm_tag_allow_list_names_unique_models_with_tag_bones_and_a_reason() {
        let models: BTreeSet<_> = OFF_NORM_TAGS.iter().map(|e| e.0).collect();
        assert_eq!(models.len(), OFF_NORM_TAGS.len());
        for (model, tags, reason) in OFF_NORM_TAGS {
            assert!(model.starts_with("c_") && !tags.is_empty() && !reason.is_empty());
            assert!(tags.iter().all(|t| t.starts_with("tag_")));
        }
    }
    #[test]
    fn idle_alias_uses_same_posture_variants_and_talk_excludes_script_gestures() {
        let def = Definition {
            path: String::new(),
            model: String::new(),
            scale: 1.,
            skins: BTreeMap::new(),
            animations: [
                ("idle", "sit1.ska"),
                ("sit_idle1", "sit1.ska"),
                ("sit_idle2", "sit2.ska"),
                ("stand_idle1", "stand.ska"),
                ("sit_talk1", "talk1.ska"),
                ("sit_talk2", "talk2.ska"),
                ("sit_talk_goaway", "goaway.ska"),
            ]
            .into_iter()
            .map(|(n, f)| (n.into(), f.into()))
            .collect(),
        };
        assert_eq!(family(&def, "idle"), ["sit_idle1", "sit_idle2"]);
        assert_eq!(family(&def, "sit_talk1"), ["sit_talk1", "sit_talk2"]);
    }
    pub(super) fn actor() -> Actor {
        Actor {
            electric: 0.,
            walk: None,
            guide: None,
            watch: Default::default(),
            story_visible: false,
            speaking: false,
            ending_talk: false,
            mouth: 0.,
            model: 0,
            spawn: Spawn {
                resident_spawn: None,
                clock_spawn: None,
                imp_spawn: None,
                chess_spawn: None,
                difficulty_variant: false,
                name: String::new(),
                hidden: false,
                model: "test".into(),
                origin: Vec3::ZERO,
                yaw: 0.,
                scale: 1.,
                animation: None,
            },
            yaw: 0.,
            time: 1.,
            greeting: 2.,
            cooldown: 0.,
            guard: None,
            ant: None,
            chess: None,
            imp: None,
            resident: None,
            clock: None,
            footing: None,
            falling: 0.,
        }
    }
    fn map(entities: Vec<Entity>) -> Bsp {
        Bsp {
            visibility: Default::default(),
            difficulty: Default::default(),
            entities,
            models: vec![],
            fogs: vec![],
            shaders: vec![],
            vertices: vec![],
            indices: vec![],
            surfaces: vec![],
            lightmaps: vec![],
            world_surfaces: 0..0,
            planes: vec![],
            side_planes: vec![],
            brushes: vec![],
            world_brushes: 0..0,
            world_min: Vec3::ZERO,
            world_max: Vec3::ZERO,
        }
    }
    fn entity(target: &str) -> Entity {
        Entity::from([
            ("classname".into(), "Characters_Test".into()),
            ("model".into(), "c_gnomeold.tik".into()),
            ("origin".into(), "0 0 0".into()),
            ("targetname".into(), target.into()),
        ])
    }
    #[test]
    fn greeting_requires_reach_facing_clear_sight_and_cooldown() {
        let open = World::fixture(&[]);
        let mut a = actor();
        let eye = vec3(100., 0., 32.);
        assert!(a.can_greet(&open, eye, -Vec3::X));
        assert!(!a.can_greet(&open, eye, Vec3::X));
        assert!(!a.can_greet(&open, eye + Vec3::X * 100., -Vec3::X));
        let wall = World::fixture(&[(vec3(40., -50., 0.), vec3(60., 50., 100.))]);
        assert!(!a.can_greet(&wall, eye, -Vec3::X));
        a.cooldown = 1.;
        assert!(!a.can_greet(&open, eye, -Vec3::X));
    }
    #[test]
    fn talk_geometry_keeps_exact_distance_and_facing_boundaries() {
        let open = World::fixture(&[]);
        assert!(!talk_reachable(&open, Vec3::X * 140., -Vec3::X, Vec3::ZERO));
        assert!(talk_reachable(&open, Vec3::X * 139.9, -Vec3::X, Vec3::ZERO));
        assert!(talk_reachable(&open, Vec3::X * 19.9, Vec3::X, Vec3::ZERO));
        assert!(!talk_reachable(&open, Vec3::X * 20., Vec3::X, Vec3::ZERO));
        assert!(!talk_reachable(
            &open,
            Vec3::X * 100.,
            vec3(-0.75, 0.6614378, 0.),
            Vec3::ZERO
        ));
        assert!(talk_reachable(
            &open,
            Vec3::X * 100.,
            vec3(-0.7501, 0.6613, 0.),
            Vec3::ZERO
        ));
        let solid = World::fixture(&[(vec3(-1., -1., -1.), vec3(1., 1., 1.))]);
        assert!(!talk_reachable(&solid, Vec3::ZERO, Vec3::X, Vec3::X * 10.));
    }
    #[test]
    fn pause_freezes_reaction_and_wall_blocks_tracking() {
        let mut a = actor();
        a.cooldown = 4.;
        let eye = vec3(0., 100., 32.);
        let wall = World::fixture(&[(vec3(-50., 40., 0.), vec3(50., 60., 100.))]);
        a.tick(0., &World::fixture(&[]), eye);
        assert_eq!((a.time, a.greeting, a.cooldown, a.yaw), (1., 2., 4., 0.));
        a.tick(0.1, &wall, eye);
        assert_eq!(a.yaw, 0.);
        a.tick(0.1, &World::fixture(&[]), eye);
        assert!((a.yaw - 0.18).abs() < 0.001);
        assert!((a.cooldown - 3.8).abs() < 0.001);
    }
    #[test]
    fn server_metadata_excludes_animation_events_and_client_commands() {
        let m = metadata(
            r#"TIKI
          init { server { friend name "Test Person" surface cap +nodraw
            attachmodel hat.tik tag_head 0.5 "" 0
            // attachmodel fake.tik tag_head 1
          } client { surface face +nodraw attachmodel fake.tik tag_head 1 } }
          animations { idle pose.ska { server { first surface body +nodraw } } }
        "#,
        )
        .unwrap();
        assert!(m.friendly);
        assert_eq!(m.name, "Test Person");
        assert_eq!(m.hidden, BTreeSet::from(["cap".into()]));
        assert_eq!(m.attachments, vec![("hat".into(), "tag_head".into(), 0.5)]);
        assert!(model_name("../outside.tik").is_none());
        assert!(model_name("C:/outside.tik").is_none());
    }
    #[test]
    fn school_start_hides_scripted_cast_and_return_removes_first_visit_cast() {
        let mut marker = Entity::new();
        marker.insert("targetname".into(), "gnome_start".into());
        marker.insert("origin".into(), "1 2 3".into());
        let scene = map(vec![
            entity("book_cat"),
            entity("return_insane2"),
            entity("talk_gnome1"),
            entity("library_guard1"),
            entity("ordinary"),
            marker,
        ]);
        let first = placements(&scene, "skool1", None, false);
        assert_eq!(first.len(), 5);
        assert!(first[0].hidden);
        assert_eq!(first[2].origin, vec3(1., 2., 3.));
        let returning = placements(&scene, "skool1", Some("skool1_start2"), false);
        assert_eq!(returning.len(), 2);
        assert!(returning[0].hidden);
        assert_eq!(placements(&scene, "skool1", None, true).len(), 4);
        assert_eq!(
            placements(&scene, "skool1", Some("skool1_start2"), true).len(),
            2
        );
    }
    #[test]
    fn malformed_and_activation_gated_placements_are_deferred() {
        let mut flagged = entity("flagged");
        flagged.insert("spawnflags".into(), "1".into());
        let mut bad_model = entity("bad_model");
        bad_model.insert("model".into(), "../x.tik".into());
        let mut bad_origin = entity("bad_origin");
        bad_origin.insert("origin".into(), "NaN 0 0".into());
        let mut bad_scale = entity("bad_scale");
        bad_scale.insert("scale".into(), "-1".into());
        let scene = map(vec![
            flagged,
            bad_model,
            bad_origin,
            bad_scale,
            entity("normal"),
        ]);
        assert_eq!(placements(&scene, "test", None, false).len(), 1);
    }
    #[test]
    fn grounded_actors_use_scaled_bounds_preserve_identity_and_airborne_poses() {
        let world = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        let meta = metadata("init { server { setsize \"-8 -8 -4\" \"8 8 28\" } }").unwrap();
        let mut a = actor();
        a.spawn.origin.z = 100.;
        a.spawn.scale = 2.;
        let identity = a.spawn.clone();
        a.ground(0., &world, &meta);
        assert!((a.position().z - 8. - crate::collision::SKIN).abs() < 0.001);
        assert_eq!(a.spawn, identity);
        let landed = a.position();
        a.ground(0., &World::fixture(&[]), &meta);
        assert_eq!(a.position(), landed);
        a.ground(0.1, &World::fixture(&[]), &meta);
        assert!(a.position().z < landed.z);
        for command in ["fly", "swim", "gravity 0"] {
            let meta = metadata(&format!("init {{ server {{ {command} }} }}")).unwrap();
            let mut a = actor();
            a.spawn.origin.z = 100.;
            a.ground(1., &world, &meta);
            assert_eq!(a.position().z, 100.);
        }
    }
    #[test]
    fn support_uses_dynamic_platforms_and_cannot_cross_walls_or_ceilings() {
        let mut world = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        world.set_dynamic(vec![crate::collision::Collider::fixture(
            vec3(-50., -50., 25.),
            vec3(50., 50., 30.),
        )]);
        let center = Vec3::Z * 20.;
        let half = vec3(8., 8., 20.);
        let p = world
            .actor_footing(Vec3::Z * 100., center, half, 200.)
            .unwrap();
        assert!((p.z - 30. - crate::collision::SKIN).abs() < 0.001);
        assert!(world
            .actor_footing(Vec3::Z * 100., center, half, 10.)
            .is_none());
        let blocked = World::fixture(&[(vec3(-100., -100., 0.), vec3(100., 100., 80.))]);
        assert!(blocked
            .actor_footing(Vec3::ZERO, center, half, 200.)
            .is_none());
        let p = world
            .actor_footing(Vec3::new(70., 0., -1.), center, half, 20.)
            .unwrap();
        assert!((0. ..=crate::collision::SKIN + 0.001).contains(&p.z));
    }
    #[test]
    fn attentive_facing_turns_toward_alice_during_dialogue_without_advancing_ai() {
        let mut a = actor();
        a.yaw = 179_f32.to_radians();
        let eye = vec3(-600., -10., 32.);
        let open = World::fixture(&[]);
        let clock = (a.time, a.greeting, a.cooldown);
        for _ in 0..60 {
            a.face(1. / 60., &open, eye, true);
        }
        let forward = vec2(a.yaw.cos(), a.yaw.sin());
        assert!(forward.dot((eye - a.target()).truncate().normalize()) > 0.999);
        assert_eq!((a.time, a.greeting, a.cooldown), clock);
        let yaw = a.yaw;
        a.face(0., &open, Vec3::Y * 100., true);
        assert_eq!(a.yaw, yaw);
        a.face(1., &open, a.target() + Vec3::Z * 100., true);
        assert_eq!(a.yaw, yaw);
    }
    #[test]
    fn actor_support_roundtrips_and_old_saves_keep_authored_identity() {
        let mut a = actor();
        a.footing = Some(vec3(0., 0., -25.));
        a.falling = 16.;
        let mut encoded = serde_json::to_value(&a).unwrap();
        let restored: Actor = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(restored.position(), a.position());
        assert_eq!(restored.falling, a.falling);
        encoded.as_object_mut().unwrap().remove("footing");
        encoded.as_object_mut().unwrap().remove("falling");
        let old: Actor = serde_json::from_value(encoded).unwrap();
        assert_eq!(old.spawn, a.spawn);
        assert_eq!(old.position(), old.spawn.origin);
    }
    /// A synthetic cast: `(name, model, origin x, greeting)` per actor.
    fn cast(actors: &[(&str, &str, f32, f32)]) -> Snapshot {
        Snapshot {
            actors: actors
                .iter()
                .enumerate()
                .map(|(model, &(name, kind, x, greeting))| {
                    let mut a = actor();
                    a.model = model;
                    a.spawn.name = name.into();
                    a.spawn.model = kind.into();
                    a.spawn.origin = vec3(x, 0., 0.);
                    a.greeting = greeting;
                    a
                })
                .collect(),
            greetings: 3,
            garden: None,
        }
    }
    fn names(s: &Snapshot) -> Vec<(String, f32, usize)> {
        s.actors
            .iter()
            .map(|a| (a.spawn.name.clone(), a.greeting, a.model))
            .collect()
    }
    #[test]
    fn a_saved_cast_is_owned_when_a_registered_controller_claims_any_actor() {
        use crate::levels::synthetic::REGISTRATION;
        let older = cast(&[
            ("gnome", "c_gnome", 0., 5.),
            ("synthetic_guard", "c_guard", 64., 7.),
        ]);
        assert!(older.owned_by(&[&REGISTRATION]));
        assert!(!older.owned_by(&[]));
        let unowned = cast(&[("gnome", "c_gnome", 0., 5.)]);
        assert!(!unowned.owned_by(&[&REGISTRATION]));
        assert!(!cast(&[]).owned_by(&[&REGISTRATION]));
    }
    #[test]
    fn adopting_a_regenerated_cast_resets_owned_actors_and_keeps_the_rest() {
        use crate::levels::synthetic::REGISTRATION;
        // The save: two ordinary actors that greeted Alice, the controller's guard, and one
        // actor that no longer exists in the map.
        let older = cast(&[
            ("gnome", "c_gnome", 0., 5.),
            ("synthetic_guard", "c_guard", 64., 7.),
            ("elder", "c_elder", 128., 9.),
            ("gone", "c_gone", 192., 11.),
        ]);
        // What a fresh visit spawns: the same authored actors (in another model order), and
        // a difficulty variant the save never had.
        let mut fresh = cast(&[
            ("elder", "c_elder", 128., 2.),
            ("synthetic_guard", "c_guard", 64., 2.),
            ("gnome", "c_gnome", 0., 2.),
            ("variant", "c_variant", 256., 2.),
        ]);
        fresh.greetings = 0;
        let adopted = older.adopt(fresh, &[&REGISTRATION]);
        // Saved state survives for the actors the generic cast still simulates, the owned actor
        // starts fresh, and model indices come from the fresh load.
        assert_eq!(
            names(&adopted),
            [
                ("elder".to_string(), 9., 0),
                ("synthetic_guard".to_string(), 2., 1),
                ("gnome".to_string(), 5., 2),
                ("variant".to_string(), 2., 3),
            ]
        );
        // The greeting count is saved state, not something a regeneration resets.
        assert_eq!(adopted.greetings, 3);
        // With nothing owned the same merge keeps every saved actor.
        let again = older.adopt(cast(&[("gnome", "c_gnome", 0., 2.)]), &[]);
        assert_eq!(names(&again), [("gnome".to_string(), 5., 0)]);
    }
    #[test]
    fn duplicate_placements_adopt_one_saved_actor_each() {
        let older = cast(&[("guard", "c_guard", 0., 1.), ("guard", "c_guard", 0., 2.)]);
        let fresh = cast(&[
            ("guard", "c_guard", 0., 0.),
            ("guard", "c_guard", 0., 0.),
            ("guard", "c_guard", 0., 0.),
        ]);
        let adopted = older.adopt(fresh, &[]);
        assert_eq!(
            adopted
                .actors
                .iter()
                .map(|a| a.greeting)
                .collect::<Vec<_>>(),
            [1., 2., 0.]
        );
    }
}

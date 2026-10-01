//! Original decorative placements, excluding game actors, pickups and script-owned props.
use crate::{
    assets::Assets,
    bsp::Bsp,
    interaction::vector,
    skeletal::{Definition, Transform},
    texture,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    rc::Rc,
};

type Entity = BTreeMap<String, String>;
pub struct Surface {
    pub detail: Vec<crate::model_detail::Level>,
    pub name: String,
    pub uv: Vec<Vec2>,
    pub indices: Vec<u16>,
    pub frames: Rc<Vec<Vec<Vec3>>>,
}
pub struct Model {
    pub def: Definition,
    pub surfaces: Vec<Surface>,
    pub frame_time: f32,
    offset: Vec3,
}
pub struct Placement {
    pub entity: usize,
    pub name: String,
    pub model: Rc<Model>,
    /// Relative to the parent pivot when bound, otherwise world coordinates.
    pub pose: Transform,
    pub scale: f32,
    pub parent: Option<usize>,
    pub sky: bool,
}
impl Placement {
    pub fn point(&self, p: Vec3) -> Vec3 {
        self.pose.point(p * self.scale)
    }
}
pub struct Catalog {
    pub placements: Vec<Placement>,
    pub baked: usize,
    pub deferred: Vec<(usize, String)>,
}

#[derive(Default)]
struct Metadata {
    offset: Vec3,
    hidden: BTreeSet<String>,
    invisible: bool,
    clip: Option<String>,
}
fn metadata(text: &str) -> Result<Metadata> {
    let t = crate::bsp::tokens(text)?;
    let mut m = Metadata::default();
    let (mut depth, mut setup, mut init, mut server) = (0, false, false, false);
    for (i, word) in t.iter().enumerate() {
        match word.as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    setup = false;
                    init = false;
                }
                if depth < 2 {
                    server = false;
                }
            }
            "setup" if depth == 0 => setup = true,
            "init" if depth == 0 => init = true,
            "server" if init && depth == 1 => server = true,
            "origin" if setup && depth == 1 => {
                let args = t
                    .get(i + 1..i + 4)
                    .context("Truncated decorative model origin")?;
                m.offset = vector(&args.join(" ")).context("Invalid decorative model origin")?;
            }
            "surface"
                if init && server && depth == 2 && t.get(i + 2).is_some_and(|s| s == "+nodraw") =>
            {
                m.hidden.insert(t[i + 1].to_lowercase());
            }
            "hide" if init && server && depth == 2 => m.invisible = true,
            "anim" if init && server && depth == 2 => m.clip = t.get(i + 1).cloned(),
            _ => (),
        }
    }
    Ok(m)
}

pub fn rotation(e: &Entity) -> Quat {
    let angles = e.get("angles").and_then(|s| vector(s)).unwrap_or_else(|| {
        let yaw = e
            .get("angle")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.);
        match yaw {
            -1. => vec3(-90., 0., 0.),
            -2. => vec3(90., 0., 0.),
            _ => vec3(0., yaw, 0.),
        }
    });
    Quat::from_rotation_z(angles.y.to_radians())
        * Quat::from_rotation_y(angles.x.to_radians())
        * Quat::from_rotation_x(angles.z.to_radians())
}
fn relative_pose(child: Transform, parent: Transform) -> Transform {
    let inverse = parent.rotation.inverse();
    Transform {
        translation: inverse * (child.translation - parent.translation),
        rotation: inverse * child.rotation,
    }
}
fn model_name(e: &Entity) -> Option<String> {
    let name = e.get("model")?.replace('\\', "/").to_ascii_lowercase();
    Some(
        name.strip_prefix("models/")
            .unwrap_or(&name)
            .strip_suffix(".tik")?
            .to_owned(),
    )
}
fn candidate(class: &str, model: &str) -> bool {
    if ["c_", "w_", "prj_", "gb_", "alice"]
        .iter()
        .any(|p| model.starts_with(p))
        || ["null", "func_spline", "crying_alice", "mock_shell_alice"].contains(&model)
    {
        return false;
    }
    let effect = model.starts_with("fx_waterfall_") || model.starts_with("fx_wfall_face");
    if model.starts_with("fx_") {
        return effect;
    }
    [
        "ambient_",
        "lamp_",
        "bust_",
        "statue_",
        "skool_",
        "mushroom_",
        "garden_",
        "hedge_",
        "chess_",
        "tears_natural_",
        "utemple_kelp",
    ]
    .iter()
    .any(|p| class.starts_with(p))
        || [
            "script_model",
            "gnome_plant",
            "hatter_candle_huge",
            "hatter_teapot_huge",
            "tower_blowface",
            "tears_painting",
            "utemple_fountain02",
        ]
        .contains(&class)
}
fn owned(map: &str, e: &Entity, model: &str) -> bool {
    let name = e.get("targetname").map(String::as_str).unwrap_or("");
    [
        "lever",
        "key_skeleton",
        "leaf_ride",
        "lilypad",
        "globepuzzle",
        "obj_bitterbook",
        "beakerhand",
        "star",
        "tart",
        "taffy",
        "lollypop",
        "lolly_seed",
        "condenser",
        "altar_eyestaff_blade",
        "altar_eyestaff_eye",
        "altar_eyestaff_staff",
        "obj_door_machine01",
        "obj_door_machine02",
    ]
    .contains(&model)
        || (matches!(map, "pandemonium" | "fortress1") && model == "gnome_airship")
        || (map == "skool2"
            && [
                "floating_books1",
                "floating_cabinet1",
                "gym_light1",
                "gym_light2",
                "gym_light3",
                "jumbo_shelf1",
                "lolly_jg_beaker",
                "shrink_potion",
            ]
            .contains(&name))
        || (map == "potears3"
            && (name.ends_with("deco") || name == "turtleshellitem" || model == "table_button"))
}

/// Extract a small declarative subset. Calls are data, never an executable script.
#[derive(Default)]
struct Calls(BTreeMap<String, Vec<(String, String)>>);
impl Calls {
    fn load(assets: &mut Assets, path: &str) -> Result<Self> {
        Self::with_includes(path, |path| {
            if assets.contains(path) {
                Ok(Some(
                    String::from_utf8_lossy(&assets.read(path)?).into_owned(),
                ))
            } else {
                Ok(None)
            }
        })
    }
    fn with_includes(
        path: &str,
        mut read: impl FnMut(&str) -> Result<Option<String>>,
    ) -> Result<Self> {
        let mut out = Self::default();
        let mut pending = vec![path.to_owned()];
        let mut visited = BTreeSet::new();
        while let Some(path) = pending.pop() {
            let path = path.replace('\\', "/").to_ascii_lowercase();
            if !visited.insert(path.clone()) {
                continue;
            }
            ensure!(visited.len() <= 128, "Too many decoration script includes");
            let Some(text) = read(&path)? else {
                continue;
            };
            // Cinematic setup lives in included scripts, too. Its hidden or
            // attached props must not appear at their editor staging positions.
            for line in crate::materials::lines(&text) {
                if line.first().is_some_and(|s| s == "#include") && line.len() == 2 {
                    pending.push(line[1].clone());
                }
            }
            for (target, calls) in Self::parse(&text).0 {
                out.0.entry(target).or_default().extend(calls);
            }
        }
        Ok(out)
    }
    fn parse(text: &str) -> Self {
        let mut out = Self::default();
        for line in crate::materials::lines(text) {
            let line = line.join(" ");
            for statement in line.split(';') {
                let s = statement.trim();
                let Some(s) = s.strip_prefix('$') else {
                    continue;
                };
                let Some((target, call)) = s.split_once('.') else {
                    continue;
                };
                if target.is_empty()
                    || !target
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    continue;
                }
                let Some((method, args)) = call.split_once('(') else {
                    continue;
                };
                let Some((args, _)) = args.split_once(')') else {
                    continue;
                };
                out.0.entry(target.into()).or_default().push((
                    method.trim().to_ascii_lowercase(),
                    args.trim().trim_matches('"').into(),
                ));
            }
        }
        out
    }
}
fn parent_supported(map: &str, e: &Entity) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("func_door" | "func_rotatingdoor")
    ) || match map {
        "skool1" => {
            crate::school::supported_inline(e) || crate::interaction::school_platform(e).is_some()
        }
        "skool2" => crate::school2::supported(e) || crate::gym::supported(e),
        "fortress1" => crate::fortress::supported(e),
        "fortress2" => crate::beyond::supported(e),
        "gvillage" => crate::village::supported(e),
        "pandemonium" => crate::pandemonium::supported(e),
        "potears1" => crate::pool::supported(e),
        _ => crate::levels::owns_submodel(map, e),
    }
}

fn material_key(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    n.strip_suffix(".tga")
        .or_else(|| n.strip_suffix(".ftx"))
        .or_else(|| n.strip_suffix(".jpg"))
        .unwrap_or(&n)
        .to_owned()
}
/// Match geometry as well as its material: another lamp elsewhere is not a duplicate.
#[derive(Default)]
struct Baked(HashMap<String, HashMap<IVec3, Vec<Vec3>>>);
impl Baked {
    fn new(map: &Bsp, model: usize) -> Self {
        let mut out = Self::default();
        for s in map.models[model].surfaces.clone().map(|i| &map.surfaces[i]) {
            let points = out
                .0
                .entry(material_key(&map.shaders[s.shader].name))
                .or_default();
            for v in &map.vertices[s.first_vertex..s.first_vertex + s.vertex_count] {
                points
                    .entry(v.position.floor().as_ivec3())
                    .or_default()
                    .push(v.position);
            }
        }
        out
    }
    fn has(&self, material: &str, p: Vec3) -> bool {
        let Some(points) = self.0.get(&material_key(material)) else {
            return false;
        };
        let cell = p.floor().as_ivec3();
        (-1..=1).any(|x| {
            (-1..=1).any(|y| {
                (-1..=1).any(|z| {
                    points
                        .get(&(cell + ivec3(x, y, z)))
                        .is_some_and(|v| v.iter().any(|q| q.distance_squared(p) < 0.75 * 0.75))
                })
            })
        })
    }
    fn contains(&self, p: &Placement) -> bool {
        let mut tested = 0;
        let mut found = 0;
        for s in &p.model.surfaces {
            let Some(skin) = p
                .model
                .def
                .skins
                .get(&s.name)
                .or_else(|| p.model.def.skins.get("all"))
            else {
                return false;
            };
            let relative = format!("{}/{skin}", p.model.def.path);
            let points = &s.frames[0];
            for point in points.iter().step_by((points.len() / 32).max(1)) {
                tested += 1;
                let point = p.point(*point);
                if self.has(&relative, point) || self.has(skin, point) {
                    found += 1;
                }
            }
        }
        tested >= 3 && found * 10 >= tested * 9
    }
}

impl Catalog {
    pub fn load(assets: &mut Assets, map: &Bsp, name: &str) -> Result<Self> {
        let script = format!("maps/{name}.scr");
        let calls = Calls::load(assets, &script)?;
        let mut baked = HashMap::from([(0, Baked::new(map, 0))]);
        let mut models = BTreeMap::<(String, String), Rc<Model>>::new();
        let mut result = Self {
            placements: vec![],
            baked: 0,
            deferred: vec![],
        };
        for (id, e) in map.entities.iter().enumerate() {
            let Some(model) = model_name(e) else {
                continue;
            };
            let class = e
                .get("classname")
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            if !candidate(&class, &model) || owned(name, e, &model) {
                continue;
            }
            if e.get("make_static").is_some_and(|s| s == "1") {
                result.baked += 1;
                continue;
            }
            let flags = e
                .get("spawnflags")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if !map.difficulty.allows(flags) {
                continue;
            }
            let target = e.get("targetname").map(String::as_str).unwrap_or("");
            let mut actions = calls.0.get(target).cloned().unwrap_or_default();
            // Funhouse_World_Init lays these six fixtures sideways on the mirror
            // wall. rotateZupto addresses the source angle vector's roll (X axis).
            // Their flame surfaces are already baked into the BSP at this pose.
            let entrance_lamp = name == "funhouse" && target == "begin_lamps";
            if entrance_lamp {
                actions.retain(|(method, args)| method != "rotatezupto" || args != "270");
            }
            let sky = model.contains("sky") || model == "bookstacks";
            // Existing sky presentations keep their established placement policy.
            let unsupported = flags & !0x700 != 0
                || (!sky
                    && actions.iter().any(|(method, _)| {
                        ![
                            "anim",
                            "bind",
                            "notsolid",
                            "solid",
                            "stationary",
                            "rendereffects",
                            "noshadow",
                        ]
                        .contains(&method.as_str())
                    }));
            if unsupported {
                result.deferred.push((
                    id,
                    "conditional visibility or unsupported scripted movement".into(),
                ));
                continue;
            }
            let Some(origin) = e.get("origin").and_then(|s| vector(s)) else {
                continue;
            };
            let mut pose = Transform {
                translation: origin,
                rotation: if entrance_lamp {
                    Quat::from_rotation_x(270f32.to_radians())
                } else {
                    rotation(e)
                },
            };
            let mut parent = None;
            let bindings: BTreeSet<_> = actions
                .iter()
                .filter(|(method, _)| method == "bind")
                .map(|(_, args)| args.trim().trim_start_matches('$'))
                .collect();
            if !bindings.is_empty() {
                let candidates: Vec<_> = map
                    .entities
                    .iter()
                    .filter(|e| {
                        bindings.len() == 1
                            && e.get("targetname")
                                .is_some_and(|t| bindings.contains(t.as_str()))
                    })
                    .collect();
                if candidates.len() != 1 || !parent_supported(name, candidates[0]) {
                    result
                        .deferred
                        .push((id, "binding requires its map controller".into()));
                    continue;
                }
                let p = candidates[0];
                parent = p
                    .get("model")
                    .and_then(|s| s.strip_prefix('*'))
                    .and_then(|s| s.parse().ok())
                    .filter(|&i| i < map.models.len());
                if parent.is_none() {
                    result
                        .deferred
                        .push((id, "binding is not an inline map model".into()));
                    continue;
                }
                let base = p
                    .get("origin")
                    .and_then(|s| vector(s))
                    .context("Missing decoration parent origin")?;
                pose = relative_pose(
                    pose,
                    Transform {
                        translation: base,
                        rotation: rotation(p),
                    },
                );
            }
            let mut clip = crate::ambient_animation::clip(name, e).map(str::to_owned);
            if clip.is_none() {
                let clips: BTreeSet<_> = actions
                    .iter()
                    .filter(|(m, _)| m == "anim")
                    .map(|(_, a)| a.as_str())
                    .collect();
                if clips.len() > 1 {
                    result
                        .deferred
                        .push((id, "animation changes with scripted state".into()));
                    continue;
                }
                clip = clips.first().map(|s| s.to_string());
            }
            let key = (model.clone(), clip.clone().unwrap_or_default());
            if !models.contains_key(&key) {
                let path = format!("models/{model}.tik");
                let meta = metadata(&String::from_utf8_lossy(&assets.read(&path)?))?;
                if meta.invisible {
                    result.deferred.push((id, "model initially hidden".into()));
                    continue;
                }
                let (mut def, tan) = crate::weapons::read_model_clip(
                    assets,
                    &model,
                    clip.as_deref().or(meta.clip.as_deref()),
                )
                .with_context(|| format!("Decoration {name}/{id}: {model}"))?;
                // Root and kelp contain undeclared surface names in the original
                // mesh. A sole declared image is unambiguous; never guess between
                // different skins or replace an explicitly declared surface.
                if let Some(skin) = def.skins.values().next().cloned() {
                    if def.skins.values().all(|s| s == &skin) {
                        def.skins.entry("all".into()).or_insert(skin);
                    }
                }
                let surfaces = tan
                    .surfaces
                    .into_iter()
                    .filter(|s| !meta.hidden.contains("all") && !meta.hidden.contains(&s.name))
                    .map(|s| Surface {
                        detail: s.detail,
                        name: s.name,
                        uv: s.uv,
                        indices: s.indices,
                        frames: Rc::new(s.frames),
                    })
                    .collect();
                models.insert(
                    key.clone(),
                    Rc::new(Model {
                        def,
                        surfaces,
                        frame_time: tan.frame_time,
                        offset: meta.offset,
                    }),
                );
            }
            let model_data = models[&key].clone();
            let scale = e
                .get("scale")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.);
            ensure!(
                scale.is_finite() && scale > 0. && scale <= 100.,
                "Invalid decoration scale {name}/{id}"
            );
            pose.translation += pose.rotation * (model_data.offset * scale);
            let placement = Placement {
                entity: id,
                name: model,
                scale: scale * model_data.def.scale,
                pose,
                parent,
                sky,
                model: model_data,
            };
            let baked_model = parent.unwrap_or(0);
            if baked
                .entry(baked_model)
                .or_insert_with(|| Baked::new(map, baked_model))
                .contains(&placement)
            {
                result.baked += 1;
                continue;
            }
            result.placements.push(placement);
        }
        Ok(result)
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(assets)?;
    let (mut total, mut baked, mut deferred, mut bound) = (0, 0, 0, 0);
    let mut textures = BTreeSet::new();
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let catalog = Catalog::load(assets, &map, &name)?;
        if name == "skool1" {
            let pipe = map
                .entities
                .iter()
                .position(|e| e.get("targetname").is_some_and(|n| n == "talk_gnome_pipe"))
                .context("Missing school pipe fixture")?;
            ensure!(
                catalog.deferred.iter().any(|(id, _)| *id == pipe)
                    && !catalog.placements.iter().any(|p| p.entity == pipe),
                "Cinematic pipe appeared at its unattached editor position"
            );
        }
        for p in &catalog.placements {
            for s in &p.model.surfaces {
                let skin = p
                    .model
                    .def
                    .skins
                    .get(&s.name)
                    .or_else(|| p.model.def.skins.get("all"))
                    .with_context(|| {
                        format!(
                            "Missing decoration skin {name}/{} {} surface {}",
                            p.entity, p.name, s.name
                        )
                    })?;
                let path =
                    texture::resolve(assets, &format!("{}/{skin}", p.model.def.path), &specs)
                        .or_else(|| texture::resolve(assets, skin, &specs))
                        .context("Unresolved decoration skin")?;
                if textures.insert(path.clone()) {
                    texture::decode(assets, &path)?;
                }
                ensure!(
                    s.frames.iter().flatten().all(|v| p.point(*v).is_finite()),
                    "Invalid decoration geometry"
                );
            }
            println!("PROP {name}/{} {} parent {:?}", p.entity, p.name, p.parent);
        }
        for (id, why) in &catalog.deferred {
            println!("DEFER {name}/{id}: {why}");
        }
        println!(
            "MAP {name}: {} props, {} baked, {} deferred",
            catalog.placements.len(),
            catalog.baked,
            catalog.deferred.len()
        );
        total += catalog.placements.len();
        baked += catalog.baked;
        deferred += catalog.deferred.len();
        bound += catalog
            .placements
            .iter()
            .filter(|p| p.parent.is_some())
            .count();
    }
    ensure!(
        total > 500 && bound > 0,
        "Missing decorative placement coverage"
    );
    println!("PASS decorations: {total} placements, {bound} bound to movers, {baked} baked duplicates avoided, {deferred} scripted placements deferred, {} textures", textures.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn included_cinematics_keep_attached_props_out_of_static_placements() {
        let files = BTreeMap::from([
            ("maps/school.scr", "#include \"maps/cinematics/school.scr\"\n$lamp.anim(\"idle\");"),
            ("maps/cinematics/school.scr", "#include \"MAPS/school.scr\"\n// #include \"missing.scr\"\n$pipe.hide(); $pipe.attach($gnome, \"tag_pipe\");"),
        ]);
        let mut reads = Vec::new();
        let calls = Calls::with_includes("maps/school.scr", |path| {
            reads.push(path.to_owned());
            Ok(files.get(path).map(|s| s.to_string()))
        })
        .unwrap();
        assert_eq!(reads, ["maps/school.scr", "maps/cinematics/school.scr"]);
        assert_eq!(
            calls.0["pipe"]
                .iter()
                .map(|(method, _)| method.as_str())
                .collect::<Vec<_>>(),
            ["hide", "attach"]
        );
        assert_eq!(calls.0["lamp"], [("anim".into(), "idle".into())]);
    }
    #[test]
    fn calls_ignore_comments_and_keep_group_bindings() {
        let c = Calls::parse(
            "// $lamp.hide();\n/* $lamp.remove(); */\n$lamp.bind( $room );\n$lamp.anim(\"idle\");",
        );
        assert_eq!(
            c.0["lamp"],
            [
                ("bind".into(), "$room".into()),
                ("anim".into(), "idle".into())
            ]
        );
    }
    #[test]
    fn matching_material_elsewhere_does_not_suppress_a_prop() {
        let mut b = Baked::default();
        b.0.entry("models/lamp/skin".into())
            .or_default()
            .entry(IVec3::ZERO)
            .or_default()
            .push(Vec3::ZERO);
        assert!(b.has("models/lamp/skin.tga", Vec3::splat(0.1)));
        assert!(!b.has("models/lamp/skin.tga", Vec3::X * 100.));
        assert!(!b.has("models/other/skin", Vec3::ZERO));
    }
    #[test]
    fn gameplay_props_are_not_duplicated_as_decorations() {
        let e = Entity::from([("targetname".into(), "gym_light1".into())]);
        assert!(owned("skool2", &e, "lamp_hanging02"));
        assert!(!owned("hatter1", &Entity::new(), "lamp_hanging02"));
        assert!(!candidate("script_model", "c_madhatter"));
        assert!(candidate("lamp_hanging_lantern03", "lantern3"));
    }
    #[test]
    fn parent_rotation_preserves_authored_placement_and_carries_the_prop() {
        let parent = Transform {
            translation: vec3(100., 200., 300.),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        };
        let child = Transform {
            translation: vec3(90., 200., 250.),
            rotation: Quat::IDENTITY,
        };
        let local = relative_pose(child, parent);
        for vertex in [Vec3::ZERO, Vec3::X, vec3(3., 2., -4.)] {
            assert!(
                parent
                    .point(local.point(vertex))
                    .distance(child.point(vertex))
                    < 0.0001
            );
        }
        let moved = Transform {
            translation: vec3(100., 200., 300.),
            rotation: Quat::IDENTITY,
        };
        assert!(
            moved
                .point(local.translation)
                .distance(vec3(100., 210., 250.))
                < 0.0001
        );
    }
    #[test]
    fn authored_angles_include_pitch_roll_and_vertical_direction() {
        let e = Entity::from([("angles".into(), "90 90 90".into())]);
        let r = rotation(&e);
        assert!((r * Vec3::X).distance(-Vec3::Z) < 0.0001);
        assert!((r * Vec3::Y).distance(Vec3::Y) < 0.0001);
        assert!(
            (rotation(&Entity::from([("angle".into(), "-1".into())])) * Vec3::X).distance(Vec3::Z)
                < 0.0001
        );
    }
    #[test]
    fn setup_origin_and_initial_visibility_are_scoped_to_the_model() {
        let m = metadata("setup { origin 0 0 -64 } init { server { surface material2 +nodraw anim idle } } animations { idle idle.tan { server { hide } } }").unwrap();
        assert_eq!(m.offset, vec3(0., 0., -64.));
        assert!(m.hidden.contains("material2"));
        assert!(!m.invisible);
        assert_eq!(m.clip.as_deref(), Some("idle"));
    }
}

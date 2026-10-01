use super::*;
use crate::{
    cinematic::Track,
    fortress::spline::Spline,
    skeletal::{Animation, Definition, Skeleton},
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn origin(e: &super::super::Entity) -> Vec3 {
    e.get("origin")
        .and_then(|s| crate::interaction::vector(s))
        .unwrap_or_default()
}
pub(super) fn rotation(e: &super::super::Entity) -> Quat {
    let a = e
        .get("angles")
        .and_then(|s| crate::interaction::vector(s))
        .unwrap_or_else(|| {
            vec3(
                0.,
                e.get("angle").and_then(|s| s.parse().ok()).unwrap_or(0.),
                0.,
            )
        });
    Quat::from_rotation_z(a.y.to_radians())
        * Quat::from_rotation_y(a.x.to_radians())
        * Quat::from_rotation_x(a.z.to_radians())
}
pub(super) struct Path {
    pub curve: Spline,
    pub nodes: Vec<(String, String, Vec3)>,
}
impl Path {
    pub fn load(map: &Bsp, first: &str) -> Result<Self> {
        let mut next = Some(first);
        let mut seen = BTreeSet::new();
        let mut controls = Vec::new();
        let mut nodes = Vec::new();
        while let Some(n) = next {
            if !seen.insert(n) {
                break;
            }
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|s| s == n))
                .with_context(|| format!("Missing temple path node {n}"))?;
            let speed = e
                .get("speed")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.);
            ensure!(speed.is_finite() && speed > 0., "Invalid temple path speed");
            controls.push((origin(e), rotation(e), speed));
            nodes.push((
                n.into(),
                e.get("thread").cloned().unwrap_or_default(),
                origin(e),
            ));
            next = e.get("target").map(String::as_str);
            ensure!(nodes.len() <= 1024, "Temple path too long");
        }
        ensure!(!nodes.is_empty(), "Empty temple path");
        Ok(Self {
            curve: Spline::new(controls, true),
            nodes,
        })
    }
}
pub(super) struct Data {
    pub world: World,
    pub guide: Path,
    pub alice: Path,
    pub camera: Spline,
    pub turtle: Transform,
    rig: Skeleton,
    swim: Animation,
    scale: f32,
    shell: usize,
    pub bubble_bounds: (Vec3, Vec3),
    pub emitters: Vec<(usize, String, Vec3, bool)>,
    pub models: BTreeMap<String, (String, Transform)>,
    pub fish: BTreeMap<String, Path>,
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let guide = Path::load(map, "tp2")?;
        ensure!(guide.nodes.len() == 160, "Temple guide path changed");
        let d = Definition::load(a, "models/c_mockturtle.tik")?;
        let rig = Skeleton::parse(&a.read(&format!("{}/{}", d.path, d.model))?)?;
        let swim = Animation::parse(
            &a.read(&format!("{}/{}", d.path, d.animations["swim"]))?,
            rig.bones.len(),
        )?;
        let shell = rig
            .bones
            .iter()
            .position(|b| b.name == "tag_shell")
            .context("Turtle lacks shell attachment")?;
        // Use the source model bounds, not the much larger visible particle cloud.
        // Model setup ordering in the native engine remains a documented approximation.
        let bubble = crate::tan::Model::parse(&a.read("models/fx/fx_boojum_scream/scream.tan")?)?;
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = -lo;
        for p in bubble.surfaces.iter().flat_map(|s| &s.frames).flatten() {
            lo = lo.min(*p * 5.);
            hi = hi.max(*p * 5.);
        }
        ensure!(
            lo.is_finite() && hi.is_finite() && lo.cmple(hi).all(),
            "Invalid breath model bounds"
        );
        let mut emitters = Vec::new();
        let mut models = BTreeMap::new();
        for (id, e) in map.entities.iter().enumerate() {
            let Some(model) = e.get("model").filter(|s| s.ends_with(".tik")) else {
                continue;
            };
            let name = e
                .get("targetname")
                .cloned()
                .unwrap_or_else(|| format!("entity{id}"));
            if model.contains("fx_bubbles_") || model.contains("fx_mockturtle_launcher") {
                emitters.push((
                    id,
                    name,
                    origin(e),
                    model.contains("fx_mockturtle_launcher") || model.contains("fx_bubbles_air"),
                ));
            } else if !model.contains("bubblestack")
                && e.get("classname")
                    .is_some_and(|s| s == "script_model" || s.starts_with("utemple_"))
            {
                models.insert(
                    name,
                    (
                        model
                            .trim_start_matches("models/")
                            .trim_end_matches(".tik")
                            .into(),
                        Transform {
                            translation: origin(e),
                            rotation: rotation(e),
                        },
                    ),
                );
            }
        }
        let turtle = models.get("turtle").context("Missing turtle")?.1;
        let mut fish = BTreeMap::new();
        for (name, path) in [
            ("fish1", "t113"),
            ("fish_school2", "t124"),
            ("fish_school3", "t134"),
            ("fish_school4d", "t144"),
            ("fish_school5", "t148"),
            ("fish_school6", "t164"),
            ("fish_school7", "t166"),
            ("fish_school8", "t178"),
            ("fish_school9", "t183"),
            ("fish_school10", "t191"),
            ("lanternfish", "t215"),
            ("FishDart", "tp57"),
            ("FishCalm", "t175"),
            ("Fish2Dart", "t213"),
        ] {
            fish.insert(name.into(), Path::load(map, path)?);
        }
        Ok(Self {
            world: World::from_bsp(map)?,
            guide,
            alice: Path::load(map, "alice_path")?,
            camera: Spline::camera_track(Track::load(a, "utemple_path1")?.controls().collect()),
            turtle,
            rig,
            swim,
            scale: d.scale,
            shell,
            bubble_bounds: (lo, hi),
            emitters,
            models,
            fish,
        })
    }
    pub fn shell_pose(&self, transform: Transform, age: f32) -> Transform {
        let pose = self.rig.global_pose(&self.swim.sample(age, true))[self.shell];
        Transform {
            translation: transform.point(pose.translation * self.scale),
            rotation: transform.rotation * pose.rotation,
        }
    }
    pub fn swim_duration(&self) -> f32 {
        self.swim.duration()
    }
}

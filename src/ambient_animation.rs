//! Reviewed ambient clip bindings from the original map scripts.
use crate::{assets::Assets, bsp::Bsp, skeletal::Definition, tan};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub fn clip<'a>(map: &str, entity: &'a BTreeMap<String, String>) -> Option<&'a str> {
    let target = entity.get("targetname").map(String::as_str).unwrap_or("");
    match (map, target) {
        ("skool1" | "skool2", "book_stack") => Some("sway"),
        ("hatter1" | "hatter2", "sky_watch1" | "sky_watch2" | "sky_watch3" | "sky_watch4") => {
            Some("handsmove")
        }
        _ => entity.get("anim").map(String::as_str),
    }
}

pub fn read(assets: &mut Assets, def: &Definition, clip: &str) -> Result<tan::Model> {
    let file = def
        .animations
        .get(clip)
        .with_context(|| format!("Missing ambient clip {clip}"))?;
    ensure!(
        file.ends_with(".tan"),
        "Expected vertex animation for ambient {clip}"
    );
    tan::Model::parse(&assets.read(&format!("{}/{file}", def.path))?)
}

pub struct Frames {
    pub positions: std::rc::Rc<Vec<Vec<Vec3>>>,
    pub frame_time: f32,
    pub transform: crate::skeletal::Transform,
    pub scale: f32,
}
impl Frames {
    pub fn position(&self, vertex: usize, time: f32) -> Vec3 {
        let frame = (time.max(0.) / self.frame_time).rem_euclid(self.positions.len() as f32);
        let a = frame.floor() as usize;
        let p = self.positions[a][vertex].lerp(
            self.positions[(a + 1) % self.positions.len()][vertex],
            frame.fract(),
        );
        self.transform.point(p * self.scale)
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let mut count = 0;
    for name in ["skool1", "skool2", "hatter1", "hatter2"] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let script =
            String::from_utf8_lossy(&assets.read(&format!("maps/{name}.scr"))?).into_owned();
        for entity in &map.entities {
            let Some(clip) = clip(name, entity).filter(|c| ["sway", "handsmove"].contains(c))
            else {
                continue;
            };
            let target = &entity["targetname"];
            ensure!(
                script.contains(&format!("${target}.anim( \"{clip}\" )")),
                "Ambient binding absent in original {name}"
            );
            let model = &entity["model"];
            let def = Definition::load(
                assets,
                &format!("models/{}", model.trim_start_matches("models/")),
            )?;
            let tan = read(assets, &def, clip)?;
            ensure!(
                tan.surfaces
                    .iter()
                    .any(|s| s.frames.len() > 1 && s.frames.iter().any(|f| f != &s.frames[0])),
                "Static ambient animation {model}"
            );
            count += 1;
        }
    }
    ensure!(
        count == 10,
        "Expected both book stacks and eight clock placements, got {count}"
    );
    println!("PASS {count} authored ambient animation placements with moving frames");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vertex_animation_interpolates_wraps_and_is_repeatable() {
        let frames = Frames {
            positions: std::rc::Rc::new(vec![vec![Vec3::ZERO], vec![Vec3::X * 2.]]),
            frame_time: 0.5,
            transform: crate::skeletal::Transform {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
            },
            scale: 1.,
        };
        assert_eq!(frames.position(0, 0.25), Vec3::X);
        assert_eq!(frames.position(0, 0.75), Vec3::X);
        assert_eq!(frames.position(0, 1.), Vec3::ZERO);
        assert_eq!(frames.position(0, 1.25), frames.position(0, 0.25));
    }
}

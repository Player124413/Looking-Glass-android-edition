//! Summoned hints use local recordings and authored regions, never quest events.
pub mod beat;
pub mod check;
mod spec;
use beat::CatBeat;

use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    skeletal::Transform,
    story::{self, Line, Story},
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

struct Region {
    collider: Collider,
    line: Line,
    entity: usize,
    spec: Option<&'static spec::HintSpec>,
    enabled: bool,
}
#[cfg_attr(test, derive(Default))]
pub struct Hints {
    allowed: bool,
    regions: Vec<Region>,
    fallback: Vec<Line>,
    selected: Option<usize>,
    next: usize,
    cooldown: f32,
    pub appearance: Option<Appearance>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Appearance {
    transform: Option<Transform>,
    #[serde(flatten)]
    beat: CatBeat,
}
impl Hints {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            selected: self.selected,
            next: self.next,
            cooldown: self.cooldown,
            appearance: self.appearance.clone(),
        }
    }
    pub fn restore(&mut self, s: &Snapshot) -> Result<()> {
        anyhow::ensure!(
            s.selected.is_none_or(|i| i < self.regions.len())
                && s.cooldown.is_finite()
                && (0. ..=120.).contains(&s.cooldown),
            "Invalid saved Cheshire state"
        );
        if let Some(a) = &s.appearance {
            a.beat.validate()?;
            anyhow::ensure!(
                a.transform.is_none_or(|t| t.translation.is_finite()
                    && t.translation.abs().max_element() < 100_000.
                    && t.rotation.is_finite()
                    && (t.rotation.length_squared() - 1.).abs() < 0.01),
                "Invalid saved Cheshire placement"
            );
        }
        self.selected = s.selected;
        self.next = s.next;
        self.cooldown = s.cooldown;
        self.appearance = s.appearance.clone();
        Ok(())
    }
    pub fn saved_line(&self, path: &str) -> Option<Line> {
        self.fallback
            .iter()
            .chain(self.regions.iter().map(|r| &r.line))
            .find(|l| l.path == path)
            .cloned()
    }
    pub fn load(assets: &mut Assets, map: &Bsp, name: &str) -> Result<Self> {
        let mut table = BTreeMap::new();
        for file in ["cat_idle", "cat_unhelp", "cat_action", name] {
            if let Ok(bytes) = assets.read(&format!("dialog/{file}.tlk")) {
                table.extend(story::subtitles(&bytes));
            }
        }
        // Some BSP hints refer to lines stored in a different map's TLK.
        let tables = assets
            .names()
            .filter(|p| p.starts_with("dialog/") && p.ends_with(".tlk"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        for file in tables {
            for (path, text) in story::subtitles(&assets.read(&file)?) {
                if path.contains("/cheshire_cat/") {
                    table.entry(path).or_insert(text);
                }
            }
        }
        // These eleven aliases are the original Alice model's cat_snide group.
        let fallback = (37..=47)
            .map(|i| {
                story::load_line(
                    assets,
                    &table,
                    "summoned_cheshire".into(),
                    format!("sound/character/cheshire_cat/vo/cat{i:03}.wav"),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let mut regions = Vec::new();
        // Append newly supported named regions, keeping saved legacy indices stable.
        for (entity, e) in map
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.contains_key("targetname"))
            .chain(
                map.entities
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.contains_key("targetname")),
            )
        {
            if e.get("classname").map(String::as_str) != Some("trigger_catmessage") {
                continue;
            }
            // Named regions in unfinished maps require script activation/replacement.
            // Do not offer those potentially premature hints until their gates exist.
            let spec = e.get("targetname").and_then(|n| spec::named(name, n));
            if e.contains_key("targetname") && spec.is_none() {
                continue;
            }
            if let Some(s) = spec {
                anyhow::ensure!(
                    s.entities.contains(&entity),
                    "Unreviewed named hint {name}#{entity}"
                );
            }
            let Some(path) = e.get("target").filter(|s| {
                s.starts_with("sound/character/cheshire_cat/")
                    && s.ends_with(".wav")
                    && !s.contains("..")
            }) else {
                continue;
            };
            let Some(index) = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let origin = e
                .get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .unwrap_or(Vec3::ZERO);
            let line = story::load_line(assets, &table, "summoned_cheshire".into(), path.clone())?;
            regions.push(Region {
                entity,
                spec,
                enabled: spec.is_none(),
                collider: Collider::model(map, index, origin, Quat::IDENTITY, false)?,
                line,
            });
        }
        println!(
            "Cheshire {name}: {} hint regions, {} fallback lines",
            regions.len(),
            fallback.len()
        );
        Ok(Self {
            allowed: true,
            regions,
            fallback,
            selected: None,
            next: 0,
            cooldown: 0.,
            appearance: None,
        })
    }
    pub fn observe(&mut self, before: Vec3, after: Vec3) {
        for (i, r) in self.regions.iter().enumerate() {
            if !r.enabled {
                continue;
            }
            if r.collider
                .touches(before + PLAYER_CENTER, after + PLAYER_CENTER, PLAYER_HALF)
            {
                self.selected = Some(i);
            }
        }
    }
    /// Derived from the existing visit owner, including immediately after restore.
    /// This has no quest writes and introduces no additional controller or event rules.
    pub fn sync(&mut self, i: &crate::interaction::Interactions) {
        self.allowed = !i.scripted()
            && !i.duchess.as_ref().is_some_and(|d| d.cinematic())
            && i.levels_allow_cheshire();
        for r in &mut self.regions {
            r.enabled = r.spec.is_none_or(|s| (s.enabled)(i));
        }
        if self
            .selected
            .is_some_and(|index| !self.regions[index].enabled)
        {
            self.selected = None;
        }
    }
    pub fn prepare_story(&self, story: &Story) -> bool {
        !story.hint_active()
            || self
                .appearance
                .as_ref()
                .is_none_or(|a| a.beat.speech_ready(a.transform.is_some()))
    }
    pub fn visible(&self) -> bool {
        self.appearance
            .as_ref()
            .is_some_and(|a| a.transform.is_some())
    }
    pub fn summon(
        &mut self,
        story: &mut Story,
        world: &World,
        feet: Vec3,
        yaw: f32,
        swimming: bool,
    ) -> Result<(), &'static str> {
        if !self.allowed {
            return Err("Cheshire is unavailable during this scene.");
        }
        if story.busy() {
            return Err("Let the current conversation finish first.");
        }
        if self.cooldown > 0. || self.appearance.is_some() {
            return Err("Cheshire will return in a moment.");
        }
        self.observe(feet, feet);
        let line = self.selected.map_or_else(
            || self.fallback[self.next % self.fallback.len()].clone(),
            |i| self.regions[i].line.clone(),
        );
        let path = line.path.clone();
        if !story.summon_hint(line) {
            return Err("Let the current conversation finish first.");
        }
        self.next = self.next.wrapping_add(1);
        self.appearance = Some(Appearance {
            transform: if swimming {
                None
            } else {
                place(world, feet, yaw)
            },
            beat: CatBeat::new(),
        });
        println!(
            "Cheshire summoned: {path}; visible={}",
            self.appearance.as_ref().unwrap().transform.is_some()
        );
        Ok(())
    }
    /// Return true once, when the disappearance begins, for its sound effect.
    pub fn update(&mut self, dt: f32, speaking: bool) -> bool {
        if dt <= 0. || !dt.is_finite() {
            return false;
        }
        self.cooldown = (self.cooldown - dt).max(0.);
        let Some(a) = &mut self.appearance else {
            return false;
        };
        let visible = a.transform.is_some();
        let (vanish, done) = a.beat.tick(dt, speaking, visible);
        if done {
            self.cooldown = a.beat.cooldown(visible);
            self.appearance = None;
        }
        vanish
    }
    pub fn recover(&mut self) {
        self.appearance = None;
        self.cooldown = 0.;
        self.selected = None;
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    selected: Option<usize>,
    next: usize,
    cooldown: f32,
    appearance: Option<Appearance>,
}
fn place(world: &World, feet: Vec3, yaw: f32) -> Option<Transform> {
    let half = vec3(26., 26., 24.);
    for angle in [0., 0.55, -0.55, 1.05, -1.05] {
        for distance in [145., 110., 190.] {
            let d = vec3((yaw + angle).cos(), (yaw + angle).sin(), 0.);
            let at = feet + d * distance;
            let trace = world.sweep(at + Vec3::Z * 88., at - Vec3::Z * 72., half);
            if trace.start_solid || trace.fraction >= 1. || trace.normal.z < 0.65 {
                continue;
            }
            let center =
                (at + Vec3::Z * 88.).lerp(at - Vec3::Z * 72., trace.fraction) + Vec3::Z * 0.1;
            let position = center - Vec3::Z * 24.;
            if world.sweep(center, center, half).start_solid
                || world.liquid_at(position + Vec3::Z * 10.) != 0
            {
                continue;
            }
            let sight = world.sweep(feet + Vec3::Z * 52., center, Vec3::splat(1.));
            if sight.start_solid || sight.fraction < 1. {
                continue;
            }
            return Some(Transform {
                translation: position,
                rotation: Quat::from_rotation_z((feet.y - position.y).atan2(feet.x - position.x)),
            });
        }
    }
    None // In a narrow space, keep the voice and subtitle without embedding a body.
}
pub struct Art {
    model: crate::npc::Puppet,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        Ok(Self {
            model: crate::npc::Puppet::load(
                assets,
                "c_cheshire",
                &["sit_talk1", "sit_idle1"],
                &specs,
            )?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn story_pose(&mut self, story: &Story) {
        self.model.mouth(story.mouth(&["summoned_cheshire"]));
    }
    pub fn draw(
        &mut self,
        hints: &Hints,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
        fullbright: bool,
    ) {
        let Some(a) = &hints.appearance else {
            return;
        };
        let Some(transform) = a.transform else {
            return;
        };
        let opacity = a.beat.opacity();
        if opacity <= 0. {
            return;
        }
        self.material.atmosphere(atmosphere, camera);
        let speaking = a.beat.speech_ready(true) && a.beat.leaving.is_none();
        self.model.draw_dissolving(
            if speaking { "sit_talk1" } else { "sit_idle1" },
            a.beat.elapsed,
            true,
            transform,
            1.,
            fullbright,
            1. - opacity,
        );
        gl_use_default_material();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Hints {
        Hints {
            allowed: true,
            regions: vec![],
            fallback: vec![Line {
                watches: BTreeMap::new(),
                lips: Default::default(),
                actor: "summoned_cheshire".into(),
                path: "test.wav".into(),
                text: "Hint".into(),
                duration: 1.,
            }],
            selected: None,
            next: 0,
            cooldown: 0.,
            appearance: None,
        }
    }
    #[test]
    fn summoning_respects_dialogue_pause_cooldown_and_never_completes_quests() {
        let world = World::fixture(&[(vec3(-1000., -1000., -32.), vec3(1000., 1000., 0.))]);
        let mut h = fixture();
        let mut s = Story::default();
        assert!(h.summon(&mut s, &world, Vec3::ZERO, 0., false).is_ok());
        assert!(h.appearance.as_ref().unwrap().transform.is_some());
        assert!(h.summon(&mut s, &world, Vec3::ZERO, 0., false).is_err());
        s.tick(0.1, false);
        for _ in 0..20 {
            s.tick(0., true);
            h.update(0., s.hint_active());
        }
        assert!(s.hint_active());
        assert!(s.take_completed().is_empty());
        s.tick(0.1, true);
        assert!(!s.hint_active());
        assert!(s.take_completed().is_empty());
        assert_eq!(s.completed, 0);
        assert!(h.update(0.5, false));
        assert!(h.summon(&mut s, &world, Vec3::ZERO, 0., false).is_err());
        for _ in 0..15 {
            h.update(1., false);
        }
        assert!(h.summon(&mut s, &world, Vec3::ZERO, 0., true).is_ok());
        assert!(h.appearance.as_ref().unwrap().transform.is_none());
    }
    #[test]
    fn placement_does_not_cross_wall_or_use_an_unsupported_floor() {
        let wall = World::fixture(&[
            (vec3(-1000., -1000., -32.), vec3(1000., 1000., 0.)),
            (vec3(35., -1000., 0.), vec3(80., 1000., 200.)),
        ]);
        assert!(place(&wall, Vec3::ZERO, 0.).is_none());
        assert!(place(&World::fixture(&[]), Vec3::ZERO, 0.).is_none());
    }
}

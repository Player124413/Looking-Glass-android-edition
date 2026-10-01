//! Trigger-owned campaign enemies; logic is also usable without a renderer.
use crate::{
    assets::Assets,
    boojum::{self, Boojum},
    bsp::Bsp,
    collision::World,
    combat::{self, Guard, Target, Timing},
    interaction::vector,
    skeletal::{Animation, Definition, Skeleton, Transform},
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeSet;
pub const BASE: usize = 3_000_000;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Enemy {
    Guard(Guard),
    Boojum(Boojum),
    Ladybug(crate::ladybug::Ladybug),
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Actor {
    #[serde(skip, default = "entity_enabled")]
    pub enabled: bool,
    pub name: String,
    pub active: bool,
    pub enemy: Enemy,
}
fn entity_enabled() -> bool {
    true
}
pub struct Encounters {
    pub identities: Vec<crate::entity::Id>,
    pub actors: Vec<Actor>,
    pub activated: BTreeSet<String>,
    club: Timing,
    diamond: Timing,
    boojum: boojum::Timing,
    ladybug: Option<crate::ladybug::Timing>,
}
impl Encounters {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            identities: Some(self.identities.clone()),
            actors: self.actors.clone(),
            activated: self.activated.clone(),
        }
    }
    pub fn restore(&mut self, s: &Snapshot) -> Result<()> {
        anyhow::ensure!(
            s.identities
                .as_ref()
                .is_none_or(|ids| ids == &self.identities),
            "Saved enemy identities do not match map"
        );
        anyhow::ensure!(
            self.actors.len() == s.actors.len(),
            "Saved enemy list does not match map"
        );
        for (a, b) in self.actors.iter().zip(&s.actors) {
            anyhow::ensure!(a.name == b.name, "Saved enemy identity does not match map");
            match (&a.enemy, &b.enemy) {
                (Enemy::Guard(a), Enemy::Guard(b)) => {
                    b.validate_save()?;
                    anyhow::ensure!(a.ranged == b.ranged, "Saved guard type changed");
                }
                (Enemy::Boojum(_), Enemy::Boojum(b)) => b.validate_save()?,
                (Enemy::Ladybug(a), Enemy::Ladybug(b)) => b.validate_save(a)?,
                _ => anyhow::bail!("Saved enemy type does not match map"),
            }
        }
        self.actors = s.actors.clone();
        self.activated = s.activated.clone();
        Ok(())
    }
    pub fn notarget(&mut self, value: bool) {
        for a in &mut self.actors {
            match &mut a.enemy {
                Enemy::Guard(g) => g.notarget = value,
                Enemy::Boojum(b) => b.notarget = value,
                Enemy::Ladybug(b) => b.notarget = value,
            }
        }
    }
    pub fn load(assets: &mut Assets, map: &Bsp, name: &str, entry: Option<&str>) -> Result<Self> {
        let returning = entry == Some("skool1_start2");
        let mut actors = Vec::new();
        let mut identities = Vec::new();
        if matches!(
            name,
            "skool1"
                | "skool2"
                | "gvillage"
                | "potears1"
                | "pandemonium"
                | "fortress1"
                | "fortress2"
        ) {
            for (index, e) in map.entities.iter().enumerate() {
                let class = e.get("classname").map(String::as_str).unwrap_or("");
                let spawn = class == "func_spawn";
                if !class.to_lowercase().starts_with("enemies_") && !spawn {
                    continue;
                }
                let target = e
                    .get(if spawn {
                        "spawntargetname"
                    } else {
                        "targetname"
                    })
                    .or_else(|| spawn.then(|| e.get("targetname")).flatten())
                    .cloned()
                    .unwrap_or_default();
                let flags = e
                    .get("spawnflags")
                    .and_then(|s| s.parse::<u32>().ok())
                    .unwrap_or(0);
                // Quake-family spawn flags: bit 9 excludes medium difficulty, bit 6 delays spawn.
                if !map.difficulty.allows(flags) {
                    continue;
                }
                if name == "fortress1"
                    && !target.starts_with("end_spawn")
                    && !target.starts_with("s1_booj")
                    && e.get("model")
                        .is_none_or(|m| m.trim_start_matches("models/") != "c_boojum.tik")
                {
                    continue;
                }
                if name == "fortress1"
                    && entry == Some("fortress1_start2")
                    && matches!(target.as_str(), "end_spawn1" | "end_spawn2")
                {
                    continue;
                }
                if name == "gvillage"
                    && target != "guard_rabbit"
                    && !(name == "fortress2" && target == "boojum3")
                {
                    continue;
                }
                if name == "skool1"
                    && ((returning
                        && (target.starts_with("library_guard")
                            || target == "cgd_top1"
                            || spawn
                            || target == "t188"
                            || target == "t186"))
                        || (!returning && matches!(target.as_str(), "t187" | "t189")))
                {
                    continue;
                }
                let model = e
                    .get(if spawn { "modelname" } else { "model" })
                    .map(String::as_str)
                    .unwrap_or("");
                if name == "skool2"
                    && !crate::school2::encounters::owns(
                        index,
                        model.trim_start_matches("models/"),
                        &target,
                    )
                {
                    continue;
                }
                let feet = vector(e.get("origin").context("Enemy has no origin")?)
                    .context("Invalid enemy origin")?;
                let yaw = e
                    .get("angle")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.)
                    .to_radians();
                let scale = e
                    .get("scale")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.);
                let active = (name == "fortress2"
                    && spawn
                    && e.get("targetname").is_some_and(|n| n == "get_booj"))
                    || (name == "fortress1" && target.starts_with("end_spawn"))
                    || !spawn
                        && flags & 64 == 0
                        && target != "guard_rabbit"
                        && !(name == "fortress2" && target == "boojum3")
                        && !(name == "pandemonium"
                            && matches!(
                                target.as_str(),
                                "airship_cardguard1" | "airship_cardguard2" | "t129"
                            ));
                if name == "potears1" && model.trim_start_matches("models/") != "c_ladybug.tik" {
                    continue;
                }
                let enemy = match model.trim_start_matches("models/") {
                    "c_ladybug.tik" if name == "potears1" => {
                        Enemy::Ladybug(crate::ladybug::Ladybug::new(
                            feet,
                            yaw,
                            scale,
                            crate::ladybug::route_for(map, &target)?,
                        ))
                    }
                    "cardguard_club.tik" => Enemy::Guard(Guard::new(feet, yaw, scale)),
                    "cardguard_diamond.tik" => Enemy::Guard(Guard::diamond(feet, yaw, scale)),
                    "c_boojum.tik" => {
                        let mut b = Boojum::new(feet, 1.);
                        b.active = active;
                        Enemy::Boojum(b)
                    }
                    _ => continue,
                };
                actors.push(Actor {
                    enabled: true,
                    name: target,
                    active,
                    enemy,
                });
                identities.push(crate::entity::Id(index));
            }
        }
        let d = Definition::load(assets, "models/cardguard_diamond.tik")?;
        let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", d.path, d.model))?)?;
        let mut clip = |name: &str| {
            Animation::parse(
                &assets.read(&format!("{}/{}", d.path, d.animations[name]))?,
                skeleton.bones.len(),
            )
        };
        let attack = clip("stand_attack")?;
        let diamond = Timing {
            sever: None,
            alert: clip("alert1")?.duration(),
            attack: attack.duration(),
            hit: attack.frame_time * 5.,
            pain: clip("pain1")?.duration(),
            death: clip("death_1")?.duration(),
        };
        Ok(Self {
            identities,
            actors,
            activated: BTreeSet::new(),
            club: crate::npc::guard_timing(assets)?,
            diamond,
            boojum: boojum::Timing::load(assets)?,
            ladybug: if name == "potears1" {
                Some(crate::ladybug::Timing::load(assets)?)
            } else {
                None
            },
        })
    }
    pub fn activate(&mut self, name: &str) {
        if !self.activated.insert(name.into()) {
            return;
        }
        for a in self.actors.iter_mut().filter(|a| a.name == name) {
            a.active = true;
            if let Enemy::Boojum(b) = &mut a.enemy {
                b.active = true;
            }
            if let Enemy::Ladybug(b) = &mut a.enemy {
                b.patrol_started = true;
            }
        }
    }
    pub fn activate_entity(&mut self, id: crate::entity::Id) {
        if let Some(index) = self.identities.iter().position(|i| *i == id) {
            let a = &mut self.actors[index];
            self.activated.insert(a.name.clone());
            a.active = true;
            if let Enemy::Boojum(b) = &mut a.enemy {
                b.active = true;
            }
            if let Enemy::Ladybug(b) = &mut a.enemy {
                b.patrol_started = true;
            }
        }
    }
    pub fn sync_enabled(&mut self, events: &crate::event::Runtime) {
        for (a, id) in self.actors.iter_mut().zip(&self.identities) {
            a.enabled = events.enabled(*id);
        }
    }
    pub fn loot_sources(&self) -> Vec<crate::loot::Source> {
        use crate::loot::{Grade, Source};
        self.actors
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let (feet, health, grade) = match &a.enemy {
                    Enemy::Guard(g) => (
                        g.feet,
                        g.health,
                        if g.ranged {
                            Grade::Medium
                        } else {
                            Grade::Small
                        },
                    ),
                    Enemy::Boojum(b) => (b.feet, b.health, Grade::Large),
                    Enemy::Ladybug(b) => (b.feet, b.health, Grade::Medium),
                };
                Source {
                    id: BASE + i,
                    feet,
                    grade,
                    dead: health <= 0.,
                }
            })
            .collect()
    }
    pub fn targets(&self) -> Vec<Target> {
        self.actors
            .iter()
            .enumerate()
            .filter(|(_, a)| a.active && a.enabled)
            .filter_map(|(i, a)| match &a.enemy {
                Enemy::Guard(g) if g.health > 0. => Some(g.target(BASE + i)),
                Enemy::Boojum(b) if b.health > 0. => Some(b.target(BASE + i)),
                Enemy::Ladybug(b) if b.health > 0. => Some(b.target(BASE + i)),
                _ => None,
            })
            .collect()
    }
    pub fn hit(&mut self, hit: combat::Hit) -> Option<&'static str> {
        let a = self.actors.get_mut(hit.id.checked_sub(BASE)?)?;
        if !a.active || !a.enabled {
            return None;
        }
        match &mut a.enemy {
            Enemy::Guard(g) => g.hit(hit),
            Enemy::Boojum(b) => b.hit_attack(hit),
            Enemy::Ladybug(b) => b.hit_attack(hit),
        }
    }
    pub fn summon(&mut self, target: Option<Target>) {
        for a in &mut self.actors {
            match &mut a.enemy {
                Enemy::Guard(g) => g.opponents.summon = target,
                Enemy::Boojum(g) => g.opponents.summon = target,
                Enemy::Ladybug(g) => g.opponents.summon = target,
            }
        }
    }
    pub fn provoke_summon(&mut self, id: usize) {
        if let Some(a) = id.checked_sub(BASE).and_then(|i| self.actors.get_mut(i)) {
            match &mut a.enemy {
                Enemy::Guard(g) => g.opponents.demon = true,
                Enemy::Boojum(g) => g.opponents.demon = true,
                Enemy::Ladybug(g) => g.opponents.demon = true,
            }
        }
    }
    pub fn update(&mut self, dt: f32, world: &World, eye: Vec3) -> combat::Feedback {
        let mut out = combat::Feedback::default();
        for a in self.actors.iter_mut().filter(|a| a.active && a.enabled) {
            let f = match &mut a.enemy {
                Enemy::Guard(g) => g.advance(
                    dt,
                    world,
                    eye,
                    if g.ranged { self.diamond } else { self.club },
                ),
                Enemy::Boojum(b) => b.advance(dt, world, eye, self.boojum),
                Enemy::Ladybug(b) => b.advance(dt, world, eye, self.ladybug.unwrap()),
            };
            out.summon_hits.extend(f.summon_hits);
            out.damage += f.damage;
            out.impulse += f.impulse;
            let origin = match &a.enemy {
                Enemy::Guard(g) => g.target(0).center,
                Enemy::Boojum(b) => b.target(0).center,
                Enemy::Ladybug(b) => b.target(0).center,
            };
            out.spatial_sounds
                .extend(f.sounds.into_iter().map(|s| (s, origin)));
            out.spatial_sounds.extend(f.spatial_sounds);
        }
        out
    }
    pub fn draw_waves(&self) {
        for a in self.actors.iter().filter(|a| a.active && a.enabled) {
            if let Enemy::Boojum(b) = &a.enemy {
                b.draw_waves();
            }
            if let Enemy::Ladybug(b) = &a.enemy {
                crate::ladybug::Art::effects(b);
            }
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    identities: Option<Vec<crate::entity::Id>>,
    actors: Vec<Actor>,
    activated: BTreeSet<String>,
}
pub struct Art {
    club: crate::npc::Puppet,
    diamond: crate::npc::Puppet,
    boojum: crate::npc::Puppet,
    bolt: crate::weapons::Prop,
    material: crate::character::SkinMaterial,
    ladybug: crate::ladybug::Art,
    pub school_staging: bool,
}
impl Art {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        Ok(Self {
            school_staging: false,
            ladybug: crate::ladybug::Art::load(assets, &specs)?,
            club: crate::npc::Puppet::load(assets, "cardguard_club", &[], &specs)?,
            diamond: crate::npc::Puppet::load(
                assets,
                "cardguard_diamond",
                &["walk", "run", "alert1", "stand_attack", "pain1", "death_1"],
                &specs,
            )?,
            boojum: crate::npc::Puppet::load(
                assets,
                "c_boojum",
                &[
                    "fly",
                    "attack_scream",
                    "pain1",
                    "death_part01",
                    "death_part02",
                ],
                &specs,
            )?,
            bolt: crate::weapons::Prop::load_animation(assets, "prj_diamond", "idle", &specs)?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn draw(
        &mut self,
        s: &Encounters,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        for a in s.actors.iter().filter(|a| a.active && a.enabled) {
            match &a.enemy {
                Enemy::Ladybug(b) => self.ladybug.draw(b, s.ladybug.unwrap(), fullbright),
                Enemy::Guard(g) => {
                    let art = if g.ranged {
                        &mut self.diamond
                    } else {
                        &mut self.club
                    };
                    if self.school_staging
                        && a.name.starts_with("play_guard")
                        && g.state == crate::combat::State::Chase
                    {
                        art.atmosphere(atmosphere, camera);
                        art.draw(
                            "run",
                            g.time,
                            true,
                            Transform {
                                translation: g.feet,
                                rotation: Quat::from_rotation_z(g.yaw),
                            },
                            g.scale,
                            fullbright,
                        );
                    } else {
                        art.draw_guard(g, fullbright, camera, atmosphere);
                    }
                    for shot in &g.shots {
                        self.bolt.draw_frame(
                            Transform {
                                translation: shot.position,
                                rotation: Quat::from_rotation_arc(Vec3::X, shot.direction),
                            },
                            1.,
                            fullbright,
                            shot.age,
                            true,
                        );
                    }
                }
                Enemy::Boojum(b) => {
                    let (clip, time, looping) = b.clip(s.boojum);
                    let scale = if b.frozen {
                        (2.5 - b.time).clamp(0., 1.)
                    } else if b.health <= 0. {
                        1. - ((b.time - s.boojum.death) / 1.4).clamp(0., 1.)
                    } else {
                        1.
                    };
                    self.boojum.draw(
                        clip,
                        time,
                        looping,
                        Transform {
                            translation: b.feet,
                            rotation: Quat::from_rotation_z(b.yaw),
                        },
                        scale,
                        fullbright,
                    );
                    self.boojum.draw_electric(b.electric);
                }
            }
        }
        gl_use_default_material();
    }
}

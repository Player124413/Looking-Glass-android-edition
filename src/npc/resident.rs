//! Shared residency for enemy controllers outside their original encounter maps.
use super::*;
use crate::{boojum, ladybug};

pub const BOOJUM_CLIPS: &[&str] = &[
    "fly",
    "attack_scream",
    "pain1",
    "death_part01",
    "death_part02",
    "death_frozen",
];
pub const DIAMOND_CLIPS: &[&str] = &[
    "walk",
    "run",
    "alert1",
    "stand_attack",
    "pain1",
    "death_1",
    "death_frozen",
];
pub fn supported(model: &str) -> bool {
    matches!(model, "c_boojum" | "c_ladybug" | "cardguard_diamond")
        || crate::plants::Kind::from_model(model).is_some()
        || crate::cards::Kind::from_model(model).is_some()
        || crate::snark::Kind::from_model(model).is_some()
        || crate::burrow::Kind::from_model(model).is_some()
        || model == crate::magma::MODEL
        || crate::wildlife::Kind::from_model(model).is_some()
}
pub fn legacy(map: &str, model: &str) -> bool {
    match model {
        "c_boojum" | "cardguard_diamond" => matches!(
            map,
            "gvillage" | "pandemonium" | "fortress1" | "fortress2" | "skool1" | "skool2"
        ),
        "c_ladybug" => map == "potears1",
        _ => false,
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Body {
    Boojum(boojum::Boojum),
    Ladybug(ladybug::Ladybug),
    Diamond(combat::Guard),
    Plant(crate::plants::Plant),
    Card(crate::cards::Guard),
    Snark(crate::snark::Snark),
    Insect(crate::burrow::Insect),
    Magma(crate::magma::Magma),
    Wildlife(crate::wildlife::Creature),
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Resident {
    pub active: bool,
    pub delay: Option<f32>,
    pub awake: bool,
    pub body: Body,
    accumulator: f64,
}
impl Resident {
    pub fn new(spawn: &Spawn, map: &Bsp, name: &str) -> Result<Option<Self>> {
        let body = match spawn.model.as_str() {
            "cardguard_diamond" => {
                Body::Diamond(combat::Guard::diamond(spawn.origin, spawn.yaw, spawn.scale))
            }
            "c_boojum" => {
                ensure!(spawn.scale == 1., "Unsupported scaled Boojum placement");
                let mut b = boojum::Boojum::new(spawn.origin, 0.);
                b.yaw = spawn.yaw;
                b.active = true;
                Body::Boojum(b)
            }
            "c_ladybug" => {
                let route = super::resident_spawns::patrol(map, name, &spawn.name)?;
                let mut b = ladybug::Ladybug::new(spawn.origin, spawn.yaw, spawn.scale, route);
                b.patrol_started = matches!(name, "garden1" | "garden4" | "centipede1")
                    || spawn.resident_spawn.is_some();
                Body::Ladybug(b)
            }
            model if crate::plants::Kind::from_model(model).is_some() => {
                Body::Plant(crate::plants::Plant::new(
                    crate::plants::Kind::from_model(model).unwrap(),
                    spawn.origin,
                    spawn.yaw,
                    spawn.scale,
                    spawn.resident_spawn.unwrap_or(0),
                ))
            }
            model if crate::cards::Kind::from_model(model).is_some() => {
                let mut g = crate::cards::Guard::new(
                    crate::cards::Kind::from_model(model).unwrap(),
                    spawn.origin,
                    spawn.yaw,
                    spawn.scale,
                    spawn
                        .resident_spawn
                        .unwrap_or_else(|| spawn.origin.x.to_bits() as usize),
                );
                if name == "grounds2"
                    && matches!(spawn.name.as_str(), "heartguard01" | "heartguard02")
                {
                    g.vision = 4000.;
                }
                Body::Card(g)
            }
            model if crate::snark::Kind::from_model(model).is_some() => {
                Body::Snark(crate::snark::Snark::new(
                    crate::snark::Kind::from_model(model).unwrap(),
                    spawn.origin,
                    spawn.yaw,
                    spawn.scale,
                    spawn
                        .resident_spawn
                        .unwrap_or_else(|| spawn.origin.x.to_bits() as usize),
                ))
            }
            model if crate::burrow::Kind::from_model(model).is_some() => {
                Body::Insect(crate::burrow::Insect::new(
                    crate::burrow::Kind::from_model(model).unwrap(),
                    spawn.origin,
                    spawn.yaw,
                    spawn.scale,
                    spawn
                        .resident_spawn
                        .unwrap_or_else(|| spawn.origin.x.to_bits() as usize),
                ))
            }
            model if crate::wildlife::Kind::from_model(model).is_some() => {
                Body::Wildlife(crate::wildlife::Creature::new(
                    crate::wildlife::Kind::from_model(model).unwrap(),
                    spawn.origin,
                    spawn.yaw,
                    spawn.scale,
                    spawn
                        .resident_spawn
                        .unwrap_or_else(|| spawn.origin.x.to_bits() as usize),
                ))
            }
            crate::magma::MODEL => {
                let mut g = crate::magma::Magma::new(
                    spawn.origin,
                    spawn.yaw,
                    spawn.scale,
                    spawn.resident_spawn.unwrap_or(0),
                );
                g.launch = super::magma_check::launch(map, spawn)?;
                Body::Magma(g)
            }
            _ => return Ok(None),
        };
        Ok(Some(Self {
            accumulator: 0.,
            active: spawn.resident_spawn.is_none(),
            delay: None,
            awake: !(name == "funhouse" && (spawn.name.starts_with("L_wall_death") || spawn.name.starts_with("R_wall_death")))
                && !(name == "garden1" && matches!(spawn.name.as_str(), "lady1" | "lady2"))
                && !(name == "hatter1"
                    && matches!(
                        spawn.name.as_str(),
                        "spider_wall1"
                            | "spider_wall2"
                            | "spider_wall3"
                            | "spider_wall4"
                            | "spider_wall5"
                            | "spider_wall6"
                            | "spider_wall7"
                            | "spider_wall8"
                            | "end_spider"
                            | "get_her1"
                            | "get_her2"
                    )),
            body,
        }))
    }
    pub fn model(&self) -> &'static str {
        match &self.body {
            Body::Boojum(_) => "c_boojum",
            Body::Ladybug(_) => "c_ladybug",
            Body::Diamond(_) => "cardguard_diamond",
            Body::Plant(b) => b.kind.model(),
            Body::Card(b) => b.kind.model(),
            Body::Snark(b) => b.kind.model(),
            Body::Insect(b) => b.kind.model(),
            Body::Magma(_) => crate::magma::MODEL,
            Body::Wildlife(b) => b.kind.model(),
        }
    }
    pub fn position(&self) -> Vec3 {
        match &self.body {
            Body::Boojum(b) => b.feet,
            Body::Ladybug(b) => b.feet,
            Body::Diamond(b) => b.feet,
            Body::Plant(b) => b.feet,
            Body::Card(b) => b.feet,
            Body::Snark(b) => b.feet,
            Body::Insect(b) => b.feet,
            Body::Magma(b) => b.feet,
            Body::Wildlife(b) => b.feet,
        }
    }
    pub fn yaw(&self) -> f32 {
        match &self.body {
            Body::Boojum(b) => b.yaw,
            Body::Ladybug(b) => b.yaw,
            Body::Diamond(b) => b.yaw,
            Body::Plant(b) => b.yaw,
            Body::Card(b) => b.yaw,
            Body::Snark(b) => b.yaw,
            Body::Insect(b) => b.yaw,
            Body::Magma(b) => b.yaw,
            Body::Wildlife(b) => b.yaw,
        }
    }
    pub fn health(&self) -> f32 {
        match &self.body {
            Body::Boojum(b) => b.health,
            Body::Ladybug(b) => b.health,
            Body::Diamond(b) => b.health,
            Body::Plant(b) => b.health,
            Body::Card(b) => b.health,
            Body::Snark(b) => b.health,
            Body::Insect(b) => b.health,
            Body::Magma(b) => b.health,
            Body::Wildlife(b) => b.health,
        }
    }
    pub fn opponents(&mut self) -> &mut combat::Opponents {
        match &mut self.body {
            Body::Boojum(b) => &mut b.opponents,
            Body::Ladybug(b) => &mut b.opponents,
            Body::Diamond(b) => &mut b.opponents,
            Body::Plant(b) => &mut b.opponents,
            Body::Card(b) => &mut b.opponents,
            Body::Snark(b) => &mut b.opponents,
            Body::Insect(b) => &mut b.opponents,
            Body::Magma(b) => &mut b.opponents,
            Body::Wildlife(b) => &mut b.opponents,
        }
    }
    pub fn notarget(&mut self, value: bool) {
        match &mut self.body {
            Body::Boojum(b) => b.notarget = value,
            Body::Ladybug(b) => b.notarget = value,
            Body::Diamond(b) => b.notarget = value,
            Body::Plant(b) => b.notarget = value,
            Body::Card(b) => b.notarget = value,
            Body::Snark(b) => b.notarget = value,
            Body::Insect(b) => b.notarget = value,
            Body::Magma(b) => b.notarget = value,
            Body::Wildlife(b) => b.notarget = value,
        }
    }
    pub fn target(&self, id: usize) -> combat::Target {
        match &self.body {
            Body::Boojum(b) => b.target(id),
            Body::Ladybug(b) => b.target(id),
            Body::Diamond(b) => b.target(id),
            Body::Plant(b) => b.target(id),
            Body::Card(b) => b.target(id),
            Body::Snark(b) => b.target(id),
            Body::Insect(b) => b.target(id),
            Body::Magma(b) => b.target(id),
            Body::Wildlife(b) => b.target(id),
        }
    }
    pub fn grade(&self) -> crate::loot::Grade {
        match &self.body {
            Body::Card(b) => {
                if b.kind == crate::cards::Kind::Heart {
                    crate::loot::Grade::Large
                } else {
                    crate::loot::Grade::Medium
                }
            }
            Body::Insect(b) => {
                if b.kind == crate::burrow::Kind::Larva {
                    crate::loot::Grade::Small
                } else {
                    crate::loot::Grade::Medium
                }
            }
            Body::Snark(_) => crate::loot::Grade::Medium,
            Body::Wildlife(b) => {
                if b.kind == crate::wildlife::Kind::Phantom {
                    crate::loot::Grade::Large
                } else {
                    crate::loot::Grade::Medium
                }
            }
            Body::Boojum(_) | Body::Magma(_) => crate::loot::Grade::Large,
            Body::Ladybug(_) | Body::Diamond(_) => crate::loot::Grade::Medium,
            Body::Plant(b) => {
                if b.kind == crate::plants::Kind::Rose {
                    crate::loot::Grade::Small
                } else {
                    crate::loot::Grade::Medium
                }
            }
        }
    }
    pub fn validate(&self, spawn: &Spawn) -> Result<()> {
        ensure!(
            self.accumulator.is_finite()
                && self.accumulator.abs() < 0.01
                && self.model() == spawn.model
                && self
                    .delay
                    .is_none_or(|d| !self.active && (0. ..=30.).contains(&d)),
            "Invalid resident identity/activation"
        );
        match &self.body {
            Body::Boojum(b) => b.validate_save()?,
            Body::Ladybug(b) => b.validate_save(b)?,
            Body::Plant(b) => b.validate()?,
            Body::Card(b) => b.validate()?,
            Body::Snark(b) => b.validate()?,
            Body::Insect(b) => b.validate()?,
            Body::Magma(b) => b.validate()?,
            Body::Wildlife(b) => b.validate()?,
            Body::Diamond(b) => {
                b.validate_save()?;
                ensure!(b.ranged, "Resident Diamond changed family");
            }
        }
        ensure!(self.active || self.health() > 0., "Dormant enemy is dead");
        Ok(())
    }
    pub fn matches(&self, original: &Self) -> Result<()> {
        ensure!(self.model() == original.model(), "Resident family changed");
        if let (Body::Ladybug(b), Body::Ladybug(a)) = (&self.body, &original.body) {
            b.validate_save(a)?;
        }
        if let (Body::Plant(b), Body::Plant(a)) = (&self.body, &original.body) {
            ensure!(
                b.kind == a.kind && b.scale == a.scale && b.feet == a.feet,
                "Plant placement changed"
            );
        }
        if let (Body::Wildlife(b), Body::Wildlife(a)) = (&self.body, &original.body) {
            ensure!(
                b.kind == a.kind && b.scale == a.scale,
                "Creature identity changed"
            );
        }
        if let (Body::Magma(b), Body::Magma(a)) = (&self.body, &original.body) {
            ensure!(
                b.scale == a.scale && b.launch == a.launch,
                "Magma placement changed"
            );
        }
        if let (Body::Insect(b), Body::Insect(a)) = (&self.body, &original.body) {
            ensure!(
                b.kind == a.kind && b.scale == a.scale,
                "Insect placement changed"
            );
        }
        if let (Body::Snark(b), Body::Snark(a)) = (&self.body, &original.body) {
            ensure!(
                b.kind == a.kind && b.scale == a.scale,
                "Snark placement identity changed"
            );
        }
        if let (Body::Card(b), Body::Card(a)) = (&self.body, &original.body) {
            ensure!(
                b.kind == a.kind && b.scale == a.scale && b.vision == a.vision,
                "Card placement identity changed"
            );
        }
        Ok(())
    }
    pub fn vulnerable(&self) -> bool {
        self.active
            && self.health() > 0.
            && match &self.body {
                Body::Plant(b) => b.vulnerable(),
                Body::Insect(b) => b.vulnerable(),
                Body::Wildlife(b) => b.vulnerable(),
                _ => true,
            }
    }
    pub fn hit(&mut self, hit: combat::Hit) -> Option<&'static str> {
        if !self.active {
            return None;
        }
        self.opponents().demon |= hit.kind.is_demon();
        let hit = combat::Hit {
            kind: hit.kind.means(),
            ..hit
        };
        match &mut self.body {
            Body::Boojum(b) => b.hit_attack(hit),
            Body::Ladybug(b) => b.hit_attack(hit),
            Body::Diamond(b) => b.hit(hit),
            Body::Plant(b) => b.hit(hit),
            Body::Card(b) => b.hit(hit),
            Body::Snark(b) => b.hit(hit),
            Body::Insect(b) => b.hit(hit),
            Body::Magma(b) => b.hit(hit),
            Body::Wildlife(b) => b.hit(hit),
        }
    }
    pub fn place(&mut self, world: &World) {
        if let Body::Wildlife(g) = &mut self.body {
            g.place(world);
        }
        if let Body::Magma(g) = &mut self.body {
            g.place(world);
        }
        if let Body::Insect(g) = &mut self.body {
            g.place(world);
        }
        if let Body::Card(g) = &mut self.body {
            g.place(world);
        }
        if let Body::Diamond(g) = &mut self.body {
            g.place(world);
        }
    }
    pub fn update(
        &mut self,
        dt: f32,
        world: &World,
        eye: Vec3,
        data: &Data,
        out: &mut combat::Feedback,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        self.accumulator += dt.min(0.1) as f64;
        while self.accumulator + 1e-9 >= 1. / 120. {
            self.accumulator -= 1. / 120.;
            self.step(world, eye, data, out);
        }
    }
    fn step(&mut self, world: &World, eye: Vec3, data: &Data, out: &mut combat::Feedback) {
        const DT: f32 = 1. / 120.;
        if let Some(delay) = &mut self.delay {
            *delay = (*delay - DT).max(0.);
            if *delay == 0. {
                self.active = true;
                self.delay = None;
            }
            return;
        }
        if !self.active || (!self.awake && matches!(self.body, Body::Wildlife(_))) {
            return;
        }
        let retired = match &self.body {
            Body::Wildlife(_)
            | Body::Plant(_)
            | Body::Card(_)
            | Body::Snark(_)
            | Body::Insect(_)
            | Body::Magma(_) => false,
            Body::Diamond(g) => {
                g.health <= 0.
                    && g.time
                        >= if g.frozen {
                            2.5
                        } else {
                            diamond_timing(data).death
                        }
                    && g.shots.is_empty()
            }
            Body::Boojum(b) => {
                b.health <= 0. && b.time >= boo_timing(data).death + 3. && b.waves.is_empty()
            }
            Body::Ladybug(b) => {
                b.health <= 0. && b.time >= 13. && b.acorns.is_empty() && b.bursts.is_empty()
            }
        };
        if retired {
            return;
        }
        if let Body::Wildlife(g) = &self.body {
            for cue in data.events.between(
                g.clip(),
                crate::animation_events::Span {
                    start: g.time,
                    end: g.time + DT,
                    duration: data.clips[g.clip()].duration(),
                    frame_time: data.clips[g.clip()].frame_time,
                    looping: g.loops(),
                    entered: g.time == 0.,
                },
            ) {
                if let crate::animation_events::Command::Sound { path, .. } = cue {
                    out.cue_sounds.push((path.clone(), g.feet));
                }
            }
        }
        let f = match &mut self.body {
            Body::Wildlife(g) => {
                let mut f = combat::Feedback::default();
                g.step(world, eye, data, &mut f);
                f
            }
            Body::Magma(g) => {
                let mut f = combat::Feedback::default();
                g.step(world, eye, data, &mut f);
                f
            }
            Body::Insect(g) => {
                let mut f = combat::Feedback::default();
                g.step(world, eye, data, &mut f);
                f
            }
            Body::Snark(g) => {
                let mut f = combat::Feedback::default();
                g.step(world, eye, data, &mut f);
                f
            }
            Body::Card(g) => {
                let mut f = combat::Feedback::default();
                g.step(world, eye, data, &mut f);
                f
            }
            Body::Plant(b) => {
                let mut f = combat::Feedback::default();
                b.step(world, eye, data, &mut f);
                f
            }
            Body::Diamond(g) => g.advance(DT, world, eye, diamond_timing(data)),
            Body::Boojum(b) => b.advance(DT, world, eye, boo_timing(data)),
            Body::Ladybug(b) => {
                let notarget = b.notarget;
                b.notarget |= !self.awake;
                let result = b.advance(DT, world, eye, data.ladybug_timing.unwrap());
                b.notarget = notarget;
                result
            }
        };
        out.damage += f.damage;
        out.will_drain += f.will_drain;
        out.cue_sounds.extend(f.cue_sounds);
        out.impulse += f.impulse;
        out.summon_hits.extend(f.summon_hits);
        out.spatial_sounds.extend(f.spatial_sounds);
        out.spatial_sounds
            .extend(f.sounds.into_iter().map(|s| (s, self.position())));
    }
    pub fn draw(
        &self,
        model: &mut Model,
        fullbright: bool,
        material: &crate::character::SkinMaterial,
    ) {
        if !self.active {
            return;
        }
        if let Body::Magma(g) = &self.body {
            model.draw_magma(g, fullbright, material);
            return;
        }
        if let Body::Card(g) = &self.body {
            model.draw_card(g, fullbright, material);
            return;
        }
        let (clip, time, looping, scale, attached, frozen) = match &self.body {
            Body::Card(_) | Body::Magma(_) => unreachable!(),
            Body::Wildlife(b) => (
                b.clip(),
                b.sample_time(&model.data),
                b.loops(),
                b.visual_scale(&model.data),
                true,
                b.frozen,
            ),
            Body::Insect(b) => (
                b.clip(),
                b.sample_time(&model.data),
                b.loops(),
                b.visual_scale(&model.data),
                true,
                b.frozen,
            ),
            Body::Snark(b) => (
                b.clip(),
                b.sample_time(),
                b.loops(),
                b.visual_scale(&model.data),
                true,
                b.frozen,
            ),
            Body::Plant(b) => (
                b.clip(),
                b.sample_time(&model.data),
                b.loops(),
                b.visual_scale(&model.data),
                true,
                b.frozen,
            ),
            Body::Diamond(g) => {
                let clip = &model.data.clips[g.clip()];
                let time = if g.frozen {
                    g.time.min(clip.frame_time * 5.)
                } else {
                    g.time
                };
                let scale = g.scale
                    * if g.frozen {
                        (2.5 - g.time).clamp(0., 1.)
                    } else if g.health <= 0. {
                        (clip.duration() - g.time).clamp(0., 1.)
                    } else {
                        1.
                    };
                (g.clip(), time, g.state.loops(), scale, true, g.frozen)
            }

            Body::Boojum(b) => {
                let timing = boo_timing(&model.data);
                let (clip, time, looping) = b.clip(timing);
                let scale = if b.frozen {
                    (2.5 - b.time).clamp(0., 1.)
                } else if b.health <= 0. {
                    1. - ((b.time - timing.death) / 1.4).clamp(0., 1.)
                } else {
                    1.
                };
                (clip, time, looping, scale, true, b.frozen)
            }
            Body::Ladybug(b) => {
                let (clip, time, looping) = b.clip(model.data.ladybug_timing.unwrap());
                (clip, time, looping, b.visual_scale(), b.carrying, b.frozen)
            }
        };
        if scale <= 0. {
            return;
        }
        let c = &model.data.clips[clip];
        let pose = c.sample(time, looping);
        let mut visual = model
            .data
            .events
            .visual(clip, time, c.duration(), c.frame_time, looping);
        let alpha = if let Body::Wildlife(g) = &self.body {
            if g.kind == crate::wildlife::Kind::Phantom {
                let shown = matches!(
                    g.phase,
                    crate::wildlife::Phase::Melee
                        | crate::wildlife::Phase::Ranged
                        | crate::wildlife::Phase::DeathLoop
                        | crate::wildlife::Phase::Dead
                ) || (g.phase == crate::wildlife::Phase::Windup
                    && time >= 3. * c.frame_time)
                    || (g.phase == crate::wildlife::Phase::Recover && time < 14. * c.frame_time);
                visual
                    .surfaces
                    .extend([("material3".into(), !shown), ("material4".into(), !shown)]);
            }
            g.alpha(&model.data)
        } else {
            1.
        };
        material.bind();
        crate::render_fx::with_skin_opacity(alpha, || {
            model.draw_pose(
                &pose,
                0.,
                time,
                self.health() > 0.,
                Transform {
                    translation: self.position(),
                    rotation: Quat::from_rotation_z(self.yaw()),
                },
                scale,
                fullbright,
                attached,
                &visual,
                None,
            )
        });
        if frozen {
            model.draw_frozen();
        }
        let electric = match &self.body {
            Body::Diamond(b) => b.electric,
            Body::Boojum(b) => b.electric,
            Body::Ladybug(b) => b.electric,
            _ => 0.,
        };
        model.draw_electric(electric);
    }
    pub fn effects(&self, acorn: &mut Option<Prop>, diamond: &mut Option<Prop>, fullbright: bool) {
        if !self.active {
            return;
        }
        match &self.body {
            Body::Wildlife(_)
            | Body::Plant(_)
            | Body::Card(_)
            | Body::Snark(_)
            | Body::Insect(_)
            | Body::Magma(_) => {}
            Body::Diamond(g) => {
                if let Some(prop) = diamond {
                    for shot in &g.shots {
                        prop.draw_frame(
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
            }

            Body::Boojum(b) => {
                gl_use_default_material();
                b.draw_waves()
            }
            Body::Ladybug(b) => {
                if let Some(prop) = acorn {
                    for p in &b.acorns {
                        prop.draw_frame(
                            Transform {
                                translation: p.position,
                                rotation: Quat::from_euler(
                                    EulerRot::XYZ,
                                    p.age * 1.75,
                                    p.age * 3.5,
                                    p.age * 5.25,
                                ),
                            },
                            1.,
                            fullbright,
                            p.age,
                            true,
                        );
                    }
                }
                gl_use_default_material();
                ladybug::Art::effects(b);
            }
        }
    }
}
fn boo_timing(data: &Data) -> boojum::Timing {
    boojum::Timing {
        frame: data.clips["attack_scream"].frame_time,
        attack: data.clips["attack_scream"].duration(),
        pain: data.clips["pain1"].duration(),
        death: data.clips["death_part01"].duration(),
    }
}

fn diamond_timing(data: &Data) -> combat::Timing {
    combat::Timing {
        sever: None,
        alert: data.clips["alert1"].duration(),
        attack: data.clips["stand_attack"].duration(),
        hit: data.clips["stand_attack"].frame_time * 5.,
        pain: data.clips["pain1"].duration(),
        death: data.clips["death_1"].duration(),
    }
}

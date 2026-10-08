//! Bounded declarative emitters from local TIKI definitions. No client script VM.
use crate::{assets::Assets, bsp::Bsp, collision::World, environment::Atmosphere, texture};
use anyhow::Result;
use macroquad::{
    miniquad::{BlendFactor, BlendState, BlendValue, Equation, ShaderSource},
    prelude::*,
};

#[derive(Clone, Copy, Debug, Default)]
struct Range {
    low: f32,
    high: f32,
}
impl Range {
    fn fixed(v: f32) -> Self {
        Self { low: v, high: v }
    }
    fn sample(self, seed: &mut u32) -> f32 {
        self.low + (self.high - self.low) * random(seed)
    }
}
fn scalar(t: &[String], index: &mut usize) -> Option<Range> {
    let mut mode = "";
    while t
        .get(*index)
        .is_some_and(|s| s == "random" || s == "crandom")
    {
        mode = t[*index].as_str();
        *index += 1;
    }
    let v = t
        .get(*index)?
        .parse::<f32>()
        .ok()
        .filter(|v| v.is_finite() && v.abs() < 100000.)?;
    *index += 1;
    Some(match mode {
        "random" => Range { low: 0., high: v },
        "crandom" => Range {
            low: -v.abs(),
            high: v.abs(),
        },
        _ => Range::fixed(v),
    })
}
fn vector(t: &[String]) -> Option<[Range; 3]> {
    let mut i = 1;
    Some([scalar(t, &mut i)?, scalar(t, &mut i)?, scalar(t, &mut i)?])
}
fn sampled(v: [Range; 3], seed: &mut u32) -> Vec3 {
    vec3(v[0].sample(seed), v[1].sample(seed), v[2].sample(seed))
}
#[derive(Clone, Debug)]
struct Spec {
    no_deadtime: bool,
    burst: Option<(f32, u32)>,
    name: String,
    light: Option<(Vec3, f32, bool, bool)>,
    light_style: String,
    min_velocity: Vec3,
    constrain: Option<Vec3>,
    tag: Option<String>,
    rate: f32,
    size: Range,
    growth: f32,
    life: Range,
    offset: [Range; 3],
    axis_offset: [Range; 3],
    velocity: [Range; 3],
    tag_velocity: bool,
    accel: [Range; 3],
    forward: f32,
    image: String,
    fade: bool,
    fade_in: f32,
    alpha: f32,
    color: Vec3,
    random_roll: bool,
    sphere: bool,
    start_off: bool,
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            no_deadtime: false,
            burst: None,
            name: String::new(),
            light: None,
            light_style: String::new(),
            min_velocity: Vec3::ZERO,
            constrain: None,
            tag: None,
            rate: 0.,
            size: Range::fixed(0.3),
            growth: 0.,
            life: Range::fixed(1.),
            offset: [Range::default(); 3],
            axis_offset: [Range::default(); 3],
            velocity: [Range::default(); 3],
            tag_velocity: false,
            accel: [Range::default(); 3],
            forward: 0.,
            image: String::new(),
            fade: false,
            fade_in: 0.,
            alpha: 1.,
            color: Vec3::ONE,
            random_roll: false,
            sphere: false,
            start_off: false,
        }
    }
}
fn parse(text: &str) -> Vec<Spec> {
    let mut out = Vec::new();
    let mut current: Option<Spec> = None;
    for t in crate::materials::lines(text) {
        if t[0] == "originemitter" || t[0] == "tagemitter" {
            current = Some(Spec {
                name: t
                    .get(if t[0] == "tagemitter" { 2 } else { 1 })
                    .cloned()
                    .unwrap_or_default(),
                tag: if t[0] == "tagemitter" {
                    t.get(1).cloned()
                } else {
                    None
                },
                ..Default::default()
            });
            continue;
        }
        let Some(s) = current.as_mut() else { continue };
        if t[0] == ")" {
            let s = current.take().unwrap();
            if ((s.light.is_some() && s.image.is_empty())
                || (s.rate > 0.
                    && s.rate <= 1024.
                    && s.life.low >= 0.
                    && s.life.high > 0.
                    && s.life.high <= 30.
                    && s.size.low > 0.
                    && s.size.high <= 10.
                    && !s.image.is_empty()))
                && out.len() < 16
            {
                out.push(s);
            }
            continue;
        }
        let n = |i: usize| {
            t.get(i)
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite() && v.abs() < 100000.)
        };
        match t[0].as_str() {
            "no_deadtime" => s.no_deadtime = true,
            "dlight" => {
                if let (Some(r), Some(g), Some(b), Some(radius)) = (n(1), n(2), n(3), n(4)) {
                    if radius > 0. && radius <= 8192. {
                        s.light = Some((
                            vec3(r, g, b).clamp(Vec3::ZERO, Vec3::splat(2.)),
                            radius,
                            t.iter().any(|s| s == "onlylightents"),
                            radius <= 1.,
                        ));
                    }
                }
            }
            "lightstyle" => s.light_style = t.get(1).cloned().unwrap_or_default(),
            "minvel" => {
                if let (Some(x), Some(y), Some(z)) = (n(1), n(2), n(3)) {
                    s.min_velocity = vec3(x, y, z);
                }
            }
            "constrain" => {
                if let (Some(x), Some(y), Some(z)) = (n(1), n(2), n(3)) {
                    s.constrain = Some(vec3(x, y, z));
                }
            }
            "spawnrate" => s.rate = n(1).unwrap_or(0.),
            "model" => {
                s.image = t
                    .get(1)
                    .map(|v| v.trim_end_matches(".spr").into())
                    .unwrap_or_default()
            }
            "scale" => s.size = Range::fixed(n(1).unwrap_or(0.3)),
            "scalemin" => s.size.low = n(1).unwrap_or(s.size.low),
            "scalemax" => s.size.high = n(1).unwrap_or(s.size.high),
            "scalerate" => s.growth = n(1).unwrap_or(0.),
            "life" => {
                let mut i = 1;
                s.life = scalar(&t, &mut i).unwrap_or_default();
                if t.get(i).is_some_and(|v| v == "random") {
                    s.life.high += n(i + 1).unwrap_or(0.);
                }
            }
            "offset" => {
                if let Some(v) = vector(&t) {
                    s.offset = v;
                }
            }
            "offsetalongaxis" => {
                if let Some(v) = vector(&t) {
                    s.axis_offset = v;
                }
            }
            "randvel" => {
                if let Some(v) = vector(&t) {
                    s.velocity = v;
                }
            }
            "accel" => {
                if let Some(v) = vector(&t) {
                    s.accel = v;
                }
            }
            "velocity" => s.forward = n(1).unwrap_or(0.),
            "fade" => s.fade = true,
            "fadein" => s.fade_in = n(1).unwrap_or(0.).max(0.),
            "alpha" => s.alpha = n(1).unwrap_or(1.).clamp(0., 1.),
            "color" if t.len() >= 4 => {
                s.color = vec3(n(1).unwrap_or(1.), n(2).unwrap_or(1.), n(3).unwrap_or(1.))
            }
            "randomroll" => s.random_roll = true,
            "sphere" => s.sphere = true,
            "startoff" => s.start_off = true,
            _ => {}
        }
    }
    out
}
fn burst_specs(text: &str, wanted: Option<&str>, frame_time: f32) -> Vec<Spec> {
    let mut definitions = Vec::new();
    let mut block = String::new();
    let mut current = None;
    let mut count = 1;
    let (mut depth, mut animations, mut client) = (0, false, false);
    let mut clip = String::new();
    for t in crate::materials::lines(text) {
        match t[0].as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth < 3 {
                    client = false;
                }
                if depth == 0 {
                    animations = false;
                }
            }
            "animations" if depth == 0 => animations = true,
            "client" if animations && depth == 2 => client = true,
            "server" if animations && depth == 2 => client = false,
            _ if animations
                && depth == 1
                && t.get(1)
                    .is_some_and(|s| s.ends_with(".ska") || s.ends_with(".tan")) =>
            {
                clip.clone_from(&t[0])
            }
            _ => (),
        }
        if wanted.is_some_and(|wanted| !animations || !client || clip != wanted) {
            continue;
        }
        if t.len() >= 2 && (t[1] == "originspawn" || t[1] == "tagspawn") {
            let at = if t[0] == "entry" || t[0] == "enter" {
                0.
            } else {
                t[0].parse::<f32>().unwrap_or(0.) * frame_time
            };
            current = Some(at);
            count = 1;
            block = if t[1] == "tagspawn" {
                format!(
                    "tagemitter {} burst{}\n(\nspawnrate 1\n",
                    t.get(2).map_or("", String::as_str),
                    definitions.len()
                )
            } else {
                format!("originemitter burst{}\n(\nspawnrate 1\n", definitions.len())
            };
        } else if current.is_some() {
            if t[0] == "count" {
                count = t
                    .get(1)
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(1)
                    .min(512);
            }
            if t[0] != "(" {
                block.push_str(&t.join(" "));
                block.push('\n');
            }
            if t[0] == ")" {
                for mut spec in parse(&block) {
                    spec.burst = Some((current.unwrap(), count));
                    definitions.push(spec);
                }
                current = None;
            }
        }
    }
    definitions
}

fn supported(model: &str) -> bool {
    model.ends_with(".tik")
        && (model.starts_with("altar_")
            || model.starts_with("fx_emitter_")
            || model.starts_with("fx_aaron_")
            || matches!(
                model,
                "fx_emitter_steam.tik"
                    | "firefly.tik"
                    | "lantern.tik"
                    | "p_h1.tik"
                    | "p_h2.tik"
                    | "p_m1.tik"
                    | "p_m2.tik"
                    | "fx_emitter_hotsteam.tik"
                    | "fx_emitter_steamslow.tik"
                    | "fx_emitter_steamrealslow.tik"
                    | "fx_emitter_steamrealslow_down.tik"
                    | "fx_mushroom_steam.tik"
                    | "al_fire1.tik"
                    | "fx_emitter_fire_expensive.tik"
                    | "fx_bubbles_noair.tik"
                    | "fx_bubbles_air.tik"
                    | "fx_mockturtle_launcher.tik"
                    | "fx_waterfall_g1.tik"
                    | "fx_waterfall_g2.tik"
                    | "fx_waterfall_g3.tik"
                    | "fx_wfall_face.tik"
                    | "fx_wfall_face_qlair.tik"
            ))
}
/// Model children use the same bounded declarative burst properties as sprites.
/// Sampling from the owning clock keeps pauses/restores free of repeated births.
pub(crate) struct ModelBurst(Spec);
pub(crate) struct ModelParticle {
    pub model: String,
    pub tag: Option<String>,
    pub offset: Vec3,
    pub age: f32,
    pub scale: f32,
    pub alpha: f32,
}
impl ModelBurst {
    pub(crate) fn parse(text: &str, frame_time: f32) -> Vec<Self> {
        burst_specs(text, Some("idle"), frame_time).into_iter()
            .filter(|s| s.image.ends_with(".tik")).map(Self).collect()
    }
    pub(crate) fn model(&self) -> &str {
        self.0.image.trim_start_matches("models/").trim_end_matches(".tik")
    }
    pub(crate) fn sample(&self, age: f32, seed: u32) -> Vec<ModelParticle> {
        let s = &self.0;
        let (birth, count) = s.burst.unwrap();
        let age = age - birth;
        if age < 0. || age >= s.life.high { return Vec::new(); }
        (0..count.min(128)).filter_map(|i| {
            let mut seed = seed.wrapping_add(i.wrapping_mul(2654435761));
            let life = s.life.sample(&mut seed);
            if age >= life { return None; }
            let offset = sampled(s.offset, &mut seed) + sampled(s.axis_offset, &mut seed);
            let velocity = sampled(s.velocity, &mut seed) + Vec3::X * s.forward;
            let accel = sampled(s.accel, &mut seed);
            Some(ModelParticle {
                model: self.model().into(), tag: s.tag.clone(), age,
                offset: offset + velocity * age + accel * (0.5 * age * age),
                scale: (s.size.sample(&mut seed) + s.growth * age).max(0.),
                alpha: s.alpha * if s.fade { (1. - age / life).max(0.) } else { 1. },
            })
        }).collect()
    }
}
pub(crate) fn declared_lights(
    assets: &mut Assets,
    name: &str,
) -> Result<Vec<crate::lighting::Light>> {
    let text = String::from_utf8_lossy(&assets.read(&format!("models/{name}.tik"))?).into_owned();
    Ok(parse(&text)
        .into_iter()
        .filter(|s| !s.start_off)
        .filter_map(|s| {
            s.light.map(
                |(color, radius, only_models, flare)| crate::lighting::Light {
                    position: sampled(s.offset, &mut 1),
                    color,
                    radius,
                    only_models,
                    flare,
                },
            )
        })
        .collect())
}
/// Preserve attachment names and start-off flags for a controller-owned actor.
pub(crate) fn attached_lights(assets: &mut Assets, name: &str) -> Result<Vec<(String, Option<String>, bool, crate::lighting::Light)>> {
    let text = String::from_utf8_lossy(&assets.read(&format!("models/{name}.tik"))?).into_owned();
    Ok(parse(&text).into_iter().filter_map(|s| {
        s.light.map(|(color, radius, only_models, flare)| (s.name, s.tag, !s.start_off, crate::lighting::Light {
            position: sampled(s.offset, &mut 1), color, radius, only_models, flare,
        }))
    }).collect())
}
mod light_style;
mod check;
pub(crate) use check::ambient_render;

#[derive(Clone)]
struct Emitter {
    id: usize,
    pickup: bool,
    origin: Vec3,
    rotation: Quat,
    local: crate::skeletal::Transform,
    scale: f32,
    spec: Spec,
    light_style: Option<light_style::Style>,
    light_color: Option<Vec3>,
    textures: Vec<Texture2D>,
    stage: crate::materials::Stage,
    additive: bool,
    carry: f32,
    enabled: bool,
    animation_enabled: Option<bool>,
}
struct AnimationBinding {
    id: usize,
    events: crate::animation_events::Model,
    clip: String,
    model: Option<crate::tan::Model>,
    origin: crate::skeletal::Transform,
    scale: f32,
    duration: f32,
    frame_time: f32,
}
struct Puff {
    position: Vec3,
    constraint_center: Vec3,
    velocity: Vec3,
    accel: Vec3,
    age: f32,
    life: f32,
    size: f32,
    roll: f32,
    emitter: usize,
}
pub struct Steam {
    animations: Vec<AnimationBinding>,
    emitters: Vec<Emitter>,
    puffs: Vec<Puff>,
    seed: u32,
    materials: Vec<std::rc::Rc<Material>>,
}
fn random(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    (*seed >> 8) as f32 / 16777216.
}
impl Steam {
    /// Emission and living puffs for a placed entity, used by mechanism checks.
    pub fn emission(&self, id: crate::entity::Id) -> Option<(bool, usize)> {
        self.emitters.iter().any(|e|e.id==id.0).then(|| (
            self.emitters.iter().any(|e|e.id==id.0 && e.enabled),
            self.puffs.iter().filter(|p|self.emitters[p.emitter].id==id.0).count()
        ))
    }
    /// Reconstruct emitter switches from the saved animation clock. No historical
    /// bursts are replayed on loading, and paused clocks never advance emission.
    pub fn animate(&mut self, clock: f32) {
        for emitter in &mut self.emitters {
            emitter.light_color = emitter.light_style.as_ref().map(|s| s.sample(clock));
        }
        for binding in &self.animations {
            let visual = binding.events.visual(
                &binding.clip,
                clock,
                binding.duration,
                binding.frame_time,
                true,
            );
            for e in self.emitters.iter_mut().filter(|e| e.id == binding.id) {
                e.animation_enabled = visual.emitters.get(&e.spec.name).copied();
                if let (Some(tag), Some(model)) = (&e.spec.tag, &binding.model) {
                    if let (Some(points), Some(axes)) =
                        (model.tags.get(tag), model.tag_rotations.get(tag))
                    {
                        let f =
                            (clock.max(0.) / binding.frame_time).rem_euclid(points.len() as f32);
                        let a = f.floor() as usize;
                        let b = (a + 1) % points.len();
                        e.local.translation = points[a].lerp(points[b], f.fract()) * binding.scale;
                        e.local.rotation = axes[a].slerp(axes[b], f.fract());
                        e.origin = binding.origin.point(e.local.translation);
                        e.rotation = binding.origin.rotation * e.local.rotation;
                    }
                }
            }
        }
    }
    pub fn lights(&self) -> Vec<crate::lighting::Light> {
        self.emitters
            .iter()
            .filter(|e| e.enabled)
            .filter_map(|e| {
                e.spec.light.map(
                    |(color, radius, only_models, flare)| crate::lighting::Light {
                        position: e.origin
                            + sampled(e.spec.offset, &mut 1) * e.scale
                            + e.rotation * sampled(e.spec.axis_offset, &mut 1) * e.scale,
                        color: e.light_color.unwrap_or(color),
                        radius: radius * e.scale,
                        only_models,
                        flare,
                    },
                )
            })
            .collect()
    }
    pub fn check_runtime(
        &mut self,
        world: &World,
        state: &mut crate::event::Runtime,
    ) -> Result<()> {
        use anyhow::ensure;
        ensure!(
            !self.emitters.is_empty(),
            "Particle runtime fixture needs emitters"
        );
        let saved = state.snapshot();
        let eye = self.emitters[0].origin;
        self.sync(state);
        self.update(0.05, eye, world);
        ensure!(!self.puffs.is_empty(), "Active waterfall did not emit");
        let before = self
            .puffs
            .iter()
            .map(|p| (p.position, p.age))
            .collect::<Vec<_>>();
        self.update(0., eye, world);
        ensure!(
            before
                == self
                    .puffs
                    .iter()
                    .map(|p| (p.position, p.age))
                    .collect::<Vec<_>>(),
            "Pause moved particles"
        );
        for e in &self.emitters {
            state.set_enabled(crate::entity::Id(e.id), false)?;
        }
        // Round-trip the shared saved state, then verify it controls rebuilt emitters.
        let encoded = serde_json::to_vec(&state.snapshot())?;
        state.restore(&serde_json::from_slice(&encoded)?)?;
        self.sync(state);
        self.puffs.clear();
        for _ in 0..120 {
            self.update(1. / 60., eye, world);
        }
        ensure!(self.puffs.is_empty(), "Disabled saved emitter restarted");
        state.restore(&saved)?;
        self.sync(state);
        self.update(0.05, eye, world);
        ensure!(
            !self.puffs.is_empty(),
            "Re-enabled emitter remained stopped"
        );
        self.stop();
        ensure!(
            self.emitters.iter().any(|e| e.enabled),
            "School steam shutdown disabled waterfall spray"
        );
        for _ in 0..900 {
            self.update(1. / 60., eye, world);
        }
        ensure!(
            self.puffs.len() <= 4096
                && self
                    .puffs
                    .iter()
                    .all(|p| p.position.is_finite() && p.age < p.life),
            "Unbounded/invalid particle lifetime"
        );
        self.puffs.clear();
        println!("PASS emitter pause, persisted activation, reactivation, selective steam stop and bounded expiry");
        Ok(())
    }
    pub fn stop(&mut self) {
        for e in &mut self.emitters {
            if e.spec.image.contains("steam") {
                e.spec.start_off = true;
                e.enabled = false;
                e.animation_enabled = Some(false);
            }
        }
        self.puffs
            .retain(|p| !self.emitters[p.emitter].spec.start_off);
    }
    pub fn sync(&mut self, state: &crate::event::Runtime) {
        for e in &mut self.emitters {
            e.enabled = state.enabled(crate::entity::Id(e.id))
                && e.animation_enabled.unwrap_or(!e.spec.start_off);
        }
    }
    /// Apply a mechanism's current emission interval after the saved activation
    /// flags. Existing puffs finish naturally; this only stops new emission.
    pub fn gate(&mut self, intervals: &[(crate::entity::Id, bool)]) {
        for e in &mut self.emitters {
            if let Some((_, active)) = intervals.iter().find(|(id, _)| id.0 == e.id) {
                e.enabled &= active;
                if !e.enabled {
                    e.carry = 0.;
                }
            }
        }
    }
    /// Pickup emitters belong to the collectible's entity, including its light.
    pub fn collected(
        &mut self,
        items: &[crate::inventory::Pickup],
        stats: &crate::inventory::Stats,
    ) {
        let available = items.iter().filter(|p| !stats.collected.contains(&p.id))
            .filter_map(|p| p.id.rsplit(':').next()?.parse::<usize>().ok())
            .collect::<std::collections::BTreeSet<_>>();
        let ids = items
            .iter()
            .filter(|p| stats.collected.contains(&p.id))
            .filter_map(|p| p.id.rsplit(':').next()?.parse::<usize>().ok())
            .collect::<std::collections::BTreeSet<_>>();
        for e in &mut self.emitters {
            if ids.contains(&e.id) || (e.pickup && !available.contains(&e.id)) {
                e.enabled = false;
                e.carry = 0.;
            }
        }
        self.puffs
            .retain(|p| {
                let e = &self.emitters[p.emitter];
                !ids.contains(&e.id) && (!e.pickup || available.contains(&e.id))
            });
    }
    /// Move future emission with its owning entity. Already emitted particles
    /// remain in world space; tag offsets/orientation stay local to the entity.
    pub fn place(&mut self, poses: &[(crate::entity::Id, crate::skeletal::Transform)]) {
        for e in &mut self.emitters {
            if let Some((_, pose)) = poses.iter().find(|(id, _)| id.0 == e.id) {
                e.origin = pose.point(e.local.translation);
                e.rotation = pose.rotation * e.local.rotation;
            }
        }
    }
    pub fn load(assets: &mut Assets, map: &Bsp) -> Result<Self> {
        let specs = texture::read_materials(assets)?;
        let mut emitters = Vec::new();
        let mut animations = Vec::new();
        for (id, e) in map.entities.iter().enumerate() {
            let Some(model) = e.get("model") else {
                continue;
            };
            let model = model.trim_start_matches("models/");
            if !supported(model) {
                continue;
            }
            let flags = e
                .get("spawnflags")
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(0);
            if !map.difficulty.allows(flags) {
                continue;
            }
            let Some(origin) = e.get("origin").and_then(|v| crate::interaction::vector(v)) else {
                continue;
            };
            let text =
                String::from_utf8_lossy(&assets.read(&format!("models/{model}"))?).into_owned();
            let definitions = parse(&text);
            let selected_clip = e.get("anim").map(String::as_str)
                .or_else(|| (model == "lantern.tik").then_some("lantern"));
            let tan = crate::weapons::read_model_clip(
                assets, model.trim_end_matches(".tik"), selected_clip,
            ).ok();
            let scale = e
                .get("scale")
                .and_then(|v| v.parse::<f32>().ok())
                .filter(|v| v.is_finite())
                .unwrap_or(1.)
                .clamp(0.01, 10.)
                * tan.as_ref().map_or(1., |(d, _)| d.scale);
            let rotation = crate::decorations::rotation(e);
            let events = crate::animation_events::Model::parse(&text)?;
            let clip = selected_clip.unwrap_or("idle").to_owned();
            let frame_time = tan.as_ref().map_or(0.05, |(_, m)| m.frame_time);
            let duration = tan
                .as_ref()
                .and_then(|(_, m)| {
                    m.surfaces
                        .first()
                        .map(|s| s.frames.len() as f32 * m.frame_time)
                })
                .unwrap_or(1.)
                .max(0.001);
            for mut spec in definitions {
                // The original 200+100/s additive altar plume saturates this
                // renderer's LDR target. Keep the weapon readable without
                // reducing the authored motion, colours or other emitters.
                if crate::inventory::WEAPONS
                    .iter()
                    .any(|(id, _)| model == format!("altar_{id}.tik"))
                {
                    spec.alpha *= 0.12;
                }
                let mut point = origin;
                let mut axes = rotation;
                if let Some(tag) = &spec.tag {
                    let Some((_, m)) = tan.as_ref() else {
                        continue;
                    };
                    let Some(offset) = m.tags.get(tag).and_then(|v| v.first()) else {
                        continue;
                    };
                    point += rotation * *offset * scale;
                    axes *= m.tag_rotations[tag][0];
                }
                let stage = specs
                    .get(&spec.image)
                    .and_then(|s| s.stages.first())
                    .cloned()
                    .unwrap_or_else(|| crate::materials::Stage {
                        images: vec![spec.image.clone()],
                        ..Default::default()
                    });
                let mut textures = Vec::new();
                for frame in &stage.images {
                    let Some(path) = texture::resolve(assets, frame, &specs) else {
                        continue;
                    };
                    let image = texture::decode(assets, &path)?;
                    let t = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                    t.set_filter(FilterMode::Linear);
                    textures.push(t);
                }
                if spec.light.is_none()
                    && (textures.len() != stage.images.len() || textures.is_empty())
                {
                    continue;
                }
                let additive = matches!(
                    stage.blend,
                    crate::materials::Blend::Add | crate::materials::Blend::AlphaAdd
                );
                let light_style = light_style::Style::load(assets, &spec.light_style)?;
                let light_color = light_style.as_ref().map(|s| s.sample(0.));
                if model == "lantern.tik" {
                    // The source sprite is 16 pixels, not the legacy 64-pixel baseline.
                    let factor = textures.first().map_or(1., |t| t.width() / 64.);
                    spec.size.low *= factor;
                    spec.size.high *= factor;
                }
                let enabled = !spec.start_off;
                emitters.push(Emitter {
                    id,
                    pickup: e.get("classname").is_some_and(|c| c.to_ascii_lowercase().starts_with("item_")),
                    origin: point,
                    rotation: axes,
                    local: crate::skeletal::Transform {
                        translation: rotation.inverse() * (point - origin),
                        rotation: rotation.inverse() * axes,
                    },
                    scale,
                    spec,
                    light_style,
                    light_color,
                    textures,
                    stage,
                    additive,
                    // One ten-second pixie is alive from entry, then replaced.
                    carry: if model == "lantern.tik" { 1. } else { 0. },
                    enabled,
                    animation_enabled: None,
                });
            }
            animations.push(AnimationBinding {
                id,
                events,
                clip,
                model: tan.map(|(_, m)| m),
                origin: crate::skeletal::Transform {
                    translation: origin,
                    rotation,
                },
                scale,
                duration,
                frame_time,
            });
        }
        let materials = shared_materials()?;
        println!("Particles: {} authored emitters", emitters.len());
        Ok(Self {
            animations,
            emitters,
            puffs: Vec::new(),
            seed: 0x56789abc,
            materials,
        })
    }
    pub fn update(&mut self, dt: f32, eye: Vec3, world: &World) {
        self.update_clocks(dt, dt, eye, world);
    }
    pub fn update_clocks(&mut self, dt: f32, world_dt: f32, eye: Vec3, world: &World) {
        let _profile = crate::frame_profile::span("particles");
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let dt = dt.min(0.05);
        let android = crate::android::is_android();
        for p in &mut self.puffs {
            let dt = if self.emitters[p.emitter].spec.no_deadtime {
                dt
            } else {
                world_dt.clamp(0., dt)
            };
            if dt == 0. { continue; }
            p.age += dt;
            p.velocity += p.accel * dt;
            let mut next = p.position + p.velocity * dt;
            if let Some(bounds) = self.emitters[p.emitter].spec.constrain {
                for axis in 0..3 {
                    if bounds[axis] >= 0.
                        && (next[axis] - p.constraint_center[axis]).abs() > bounds[axis]
                    {
                        p.velocity[axis] = -p.velocity[axis];
                        p.accel[axis] = -p.accel[axis];
                        next[axis] += p.velocity[axis] * dt;
                    }
                }
            }
            // The enclosed pixie requests axis bounds, not world collision.
            // Its casing still occludes the sprite through the depth buffer.
            if !android
                && self.emitters[p.emitter].spec.constrain.is_none()
                && world.sweep(p.position, next, Vec3::ZERO).fraction < 1.
            {
                p.age = p.life;
            }
            if !android
                && self.emitters[p.emitter].spec.image == "bubble"
                && world.liquid_at(next) == 0
            {
                p.age = p.life;
            }
            p.position = next;
        }
        self.puffs.retain(|p| p.age < p.life);
        let cull_dist_sq = crate::android::active_preset().particle_cull_distance_sq();
        let max_puffs = if android { 192 } else { 4096 };
        for (index, e) in self.emitters.iter_mut().enumerate() {
            let dt = if e.spec.no_deadtime {
                dt
            } else {
                world_dt.min(dt)
            };
            let rate = if android && e.spec.constrain.is_none() {
                (e.spec.rate * 0.35).max(e.spec.rate.min(1.))
            } else {
                e.spec.rate
            };
            if !e.enabled
                || rate <= 0.
                || (e.spec.image.is_empty() && e.spec.light.is_some())
                || e.origin.distance_squared(eye) > cull_dist_sq
            {
                // Keep the enclosed light ready when its lantern comes into range.
                e.carry = if e.enabled && e.spec.constrain.is_some() {
                    (e.carry + dt * rate).min(1.)
                } else { 0. };
                continue;
            }
            e.carry = (e.carry + dt * rate).min(if android { 4. } else { 32. });
            while e.carry >= 1. {
                e.carry -= 1.;
                if self.puffs.len() >= max_puffs {
                    break;
                }
                let offset = sampled(e.spec.offset, &mut self.seed)
                    + e.rotation * sampled(e.spec.axis_offset, &mut self.seed);
                let axis = if e.spec.sphere {
                    vec3(
                        random(&mut self.seed) * 2. - 1.,
                        random(&mut self.seed) * 2. - 1.,
                        random(&mut self.seed) * 2. - 1.,
                    )
                    .normalize_or_zero()
                } else {
                    e.rotation * Vec3::X
                };
                let velocity = sampled(e.spec.velocity, &mut self.seed) + axis * e.spec.forward;
                self.puffs.push(Puff {
                    position: e.origin + offset * e.scale,
                    constraint_center: e.origin + offset,
                    velocity: e.spec.min_velocity + velocity * e.scale,
                    accel: sampled(e.spec.accel, &mut self.seed),
                    age: 0.,
                    life: e.spec.life.sample(&mut self.seed),
                    size: 32. * e.spec.size.sample(&mut self.seed) * e.scale,
                    roll: if e.spec.random_roll {
                        random(&mut self.seed) * std::f32::consts::TAU
                    } else {
                        0.
                    },
                    emitter: index,
                });
            }
        }
    }
    pub fn depths(&self, camera: Vec3, direction: Vec3) -> impl Iterator<Item = (usize, f32)> + '_ {
        self.puffs
            .iter()
            .enumerate()
            .map(move |(i, p)| (i, (p.position - camera).dot(direction)))
    }
    pub fn prepare_draw(&self, camera: Vec3, atmosphere: &Atmosphere) {
        for m in &self.materials {
            atmosphere.apply(m, camera);
        }
    }
    pub fn draw_one(&self, index: usize, camera: Vec3, direction: Vec3) {
        let right = direction.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(direction).normalize_or_zero();
        let p = &self.puffs[index];
        let e = &self.emitters[p.emitter];
        gl_use_material(&self.materials[usize::from(e.additive)]);
        let radius = p.size * (1. + e.spec.growth * p.age).max(0.);
        let x = (right * p.roll.cos() + up * p.roll.sin()) * radius;
        let y = (-right * p.roll.sin() + up * p.roll.cos()) * radius;
        let opacity = e.spec.alpha
            * if e.spec.fade { 1. - p.age / p.life } else { 1. }
            * (p.age / e.spec.fade_in.max(0.02)).min(1.)
            * (camera.distance(p.position) / 28.).min(1.);
        let color = Color::new(e.spec.color.x, e.spec.color.y, e.spec.color.z, opacity);
        let vertices = [
            p.position - x - y,
            p.position + x - y,
            p.position + x + y,
            p.position - x + y,
        ]
        .into_iter()
        .zip([vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)])
        .map(|(position, uv)| Vertex {
            position,
            uv,
            color: color.into(),
            normal: Vec4::ZERO,
        })
        .collect();
        crate::render_fx::effect(
            &Mesh {
                vertices,
                indices: vec![0, 1, 2, 0, 2, 3],
                texture: Some(e.textures[e.stage.frame(p.age)].clone()),
            },
            if e.additive {
                crate::materials::Blend::Add
            } else {
                crate::materials::Blend::Alpha
            },
        );
    }
}
const FRAGMENT: &str = r#"#version 100
precision mediump float; uniform sampler2D Texture; uniform float Additive; varying highp vec2 uv; varying lowp vec4 color;
// FOG
void main(){vec4 t=texture2D(Texture,uv)*color; if(Additive>0.5)t.rgb*=t.a;gl_FragColor=vec4(fogged(t.rgb,Additive),t.a);}
"#;

/// Gnome disappearance: the original pickup sprite and a deterministic, finite
/// 200-particle burst. Sampling the saved scene age also reconstructs a mid-burst load.
pub struct PickupBurst {
    texture: Texture2D,
    material: std::rc::Rc<Material>,
}
thread_local! {
    static BURST_MATERIAL: std::cell::RefCell<std::rc::Weak<Material>>=Default::default();
}
impl PickupBurst {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let specs = texture::read_materials(assets)?;
        let path = texture::resolve(assets, "supra_p", &specs)
            .ok_or_else(|| anyhow::anyhow!("Missing gnome disappearance sprite"))?;
        let image = texture::decode(assets, &path)?;
        let texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        texture.set_filter(FilterMode::Linear);
        let material = BURST_MATERIAL.with(|cache| -> Result<std::rc::Rc<Material>> {
            if let Some(m) = cache.borrow().upgrade() {
                return Ok(m);
            }
            let mut uniforms = crate::environment::uniforms();
            uniforms.push(UniformDesc::new("Additive", UniformType::Float1));
            let m = load_material(
                ShaderSource::Glsl {
                    vertex: crate::character::VERTEX,
                    fragment: &crate::environment::fragment(FRAGMENT),
                },
                MaterialParams {
                    uniforms,
                    pipeline_params: crate::render::depth_pipeline(Some(BlendState::new(
                        Equation::Add,
                        BlendFactor::One,
                        BlendFactor::One,
                    ))),
                    ..Default::default()
                },
            )
            .map_err(|e| anyhow::anyhow!("Burst shader: {e:?}"))?;
            m.set_uniform("Additive", 1_f32);
            let m = std::rc::Rc::new(m);
            *cache.borrow_mut() = std::rc::Rc::downgrade(&m);
            Ok(m)
        })?;
        Ok(Self { texture, material })
    }
    pub fn draw(&self, origin: Vec3, age: f32, camera: Vec3, atmosphere: &Atmosphere) {
        if !(0. ..4.).contains(&age) {
            return;
        }
        atmosphere.apply(&self.material, camera);
        gl_use_material(&self.material);
        let direction = (origin - camera).normalize_or_zero();
        let right = direction.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(direction).normalize_or_zero();
        let mut seed = 0x4824acd;
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for _ in 0..200 {
            let offset = vec3(
                (random(&mut seed) * 2. - 1.) * 20.,
                (random(&mut seed) * 2. - 1.) * 20.,
                random(&mut seed) * 60.,
            );
            let life = 3. + random(&mut seed);
            let accel = random(&mut seed) * 50.;
            let size = (0.25 + random(&mut seed) * 0.25) * 32.;
            if age >= life {
                continue;
            }
            let center = origin + offset + Vec3::Z * (0.5 * accel * age * age);
            let alpha = (age / 0.2).min(1.) * (1. - age / life);
            let color: [u8; 4] = Color::new(1., 1., 1., alpha).into();
            let base = vertices.len() as u16;
            for (delta, uv) in [
                (-right - up, vec2(0., 1.)),
                (right - up, vec2(1., 1.)),
                (right + up, vec2(1., 0.)),
                (-right + up, vec2(0., 0.)),
            ] {
                vertices.push(Vertex {
                    position: center + delta * size,
                    uv,
                    normal: Vec4::ZERO,
                    color,
                });
            }
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        crate::render_fx::effect(
            &Mesh {
                vertices,
                indices,
                texture: Some(self.texture.clone()),
            },
            crate::materials::Blend::Add,
        );
        gl_use_default_material();
    }
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let names = assets
        .names()
        .filter(|n| n.starts_with("models/") && supported(n.trim_start_matches("models/")))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut total = 0;
    for name in names {
        let specs = parse(&String::from_utf8_lossy(&assets.read(&name)?));
        if specs.is_empty() {
            println!("Emitter declaration deferred (no supported sprite/light): {name}");
        }
        total += specs.len();
    }
    println!("PASS {total} supported emitter definitions including multi-emitter fire and tagged waterfall spray");
    Ok(())
}

// Emitters and attached animation effects share two pipelines across model/map reloads.
thread_local! {
    static PARTICLE_MATERIALS: std::cell::RefCell<Vec<std::rc::Weak<Material>>> = const { std::cell::RefCell::new(Vec::new()) };
}
fn shared_materials() -> Result<Vec<std::rc::Rc<Material>>> {
    PARTICLE_MATERIALS.with(|cache| {
        let existing: Option<Vec<_>> = cache.borrow().iter().map(|m| m.upgrade()).collect();
        if let Some(m) = existing.filter(|m| m.len() == 2) {
            return Ok(m);
        }
        let fragment = crate::environment::fragment(FRAGMENT);
        let mut materials = Vec::new();
        for additive in [false, true] {
            let mut uniforms = crate::environment::uniforms();
            uniforms.push(UniformDesc::new("Additive", UniformType::Float1));
            let material = load_material(
                ShaderSource::Glsl {
                    vertex: crate::character::VERTEX,
                    fragment: &fragment,
                },
                MaterialParams {
                    uniforms,
                    pipeline_params: crate::render::depth_pipeline(Some(BlendState::new(
                        Equation::Add,
                        if additive {
                            BlendFactor::One
                        } else {
                            BlendFactor::Value(BlendValue::SourceAlpha)
                        },
                        if additive {
                            BlendFactor::One
                        } else {
                            BlendFactor::OneMinusValue(BlendValue::SourceAlpha)
                        },
                    ))),
                    ..Default::default()
                },
            )
            .map_err(|e| anyhow::anyhow!("Particle shader: {e:?}"))?;
            material.set_uniform("Additive", if additive { 1_f32 } else { 0. });
            materials.push(std::rc::Rc::new(material));
        }

        *cache.borrow_mut() = materials.iter().map(std::rc::Rc::downgrade).collect();
        Ok(materials)
    })
}

/// A pure sample of an animation-owned emitter. Birth IDs provide repeatable
/// randomness; sample the tag at birth so existing particles do not follow it.
pub struct Attached {
    steam: Steam,
    // Keep emitted world poses while time advances: moving the owning actor
    // must not drag smoke already in the air. Rewinds rebuild cosmetic births.
    births: std::collections::BTreeMap<(usize, u32), crate::skeletal::Transform>,
    age: f32,
}
impl Attached {
    /// Item sprites use their image dimensions as the unscaled billboard size.
    /// Keep this opt-in: older effect owners retain their existing size tuning.
    pub(crate) fn use_sprite_dimensions(&mut self) {
        for e in &mut self.steam.emitters {
            let diameter = e.textures.first().map_or(64., Texture2D::width);
            let factor = diameter / 64.; // Puff's legacy unit diameter is 64.
            e.spec.size.low *= factor;
            e.spec.size.high *= factor;
        }
    }
    /// Attenuate this instance without changing shared materials or other effects.
    pub(crate) fn attenuate(&mut self, opacity: f32) {
        for e in &mut self.steam.emitters {
            e.spec.alpha *= opacity.clamp(0., 1.);
        }
    }
    /// Change only billboard size, preserving tag placement and particle travel.
    pub(crate) fn resize(&mut self, factor: f32) {
        for e in &mut self.steam.emitters {
            e.spec.size.low *= factor.clamp(0., 1.);
            e.spec.size.high *= factor.clamp(0., 1.);
        }
    }
    /// Mouth jets use the animated attachment frame for their spread.
    pub(crate) fn orient_velocity(&mut self, name: &str) {
        for e in &mut self.steam.emitters {
            if e.spec.name == name {
                e.spec.tag_velocity = true;
            }
        }
    }
    /// A separate emitter clock sharing immutable textures/materials.
    pub(crate) fn fork(&self) -> Self {
        Self {
            births: Default::default(),
            age: 0.,
            steam: Steam {
                animations: Vec::new(),
                emitters: self.steam.emitters.clone(),
                puffs: Vec::new(),
                seed: 1,
                materials: self.steam.materials.clone(),
            },
        }
    }
    pub fn load(
        assets: &mut Assets,
        name: &str,
        materials: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Option<Self>> {
        let definitions = parse(&String::from_utf8_lossy(
            &assets.read(&format!("models/{name}.tik"))?,
        ));
        Self::from_specs(assets, definitions, materials)
    }
    /// One-shot client animation commands, sampled from a saved owning clock.
    pub(crate) fn load_bursts(
        assets: &mut Assets,
        name: &str,
        frame_time: f32,
        materials: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Option<Self>> {
        Self::load_clip_bursts(assets, name, None, frame_time, materials)
    }
    pub(crate) fn load_clip_bursts(
        assets: &mut Assets,
        name: &str,
        clip: Option<&str>,
        frame_time: f32,
        materials: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Option<Self>> {
        let text =
            String::from_utf8_lossy(&assets.read(&format!("models/{name}.tik"))?).into_owned();
        Self::from_specs(assets, burst_specs(&text, clip, frame_time), materials)
    }
    fn from_specs(
        assets: &mut Assets,
        definitions: Vec<Spec>,
        materials: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Option<Self>> {
        if definitions.is_empty() {
            return Ok(None);
        }
        let mut emitters = Vec::new();
        for spec in definitions {
            let stage = materials
                .get(&spec.image)
                .and_then(|s| s.stages.first())
                .cloned()
                .unwrap_or_else(|| crate::materials::Stage {
                    images: vec![spec.image.clone()],
                    ..Default::default()
                });
            let mut textures = Vec::new();
            for frame in &stage.images {
                let Some(path) = texture::resolve(assets, frame, materials) else {
                    continue;
                };
                let image = texture::decode(assets, &path)?;
                let t = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                t.set_filter(FilterMode::Linear);
                textures.push(t);
            }
            if textures.is_empty() || textures.len() != stage.images.len() {
                continue;
            }
            let additive = matches!(
                stage.blend,
                crate::materials::Blend::Add | crate::materials::Blend::AlphaAdd
            );
            emitters.push(Emitter {
                id: 0,
                pickup: false,
                origin: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                local: crate::skeletal::Transform {
                    translation: Vec3::ZERO,
                    rotation: Quat::IDENTITY,
                },
                scale: 1.,
                spec,
                light_style: None,
                light_color: None,
                textures,
                stage,
                additive,
                carry: 0.,
                enabled: true,
                animation_enabled: None,
            });
        }
        Ok(Some(Self {
            births: Default::default(),
            age: 0.,
            steam: Steam {
                animations: Vec::new(),
                emitters,
                puffs: Vec::new(),
                seed: 1,
                materials: shared_materials()?,
            },
        }))
    }
    pub fn draw(
        &mut self,
        age: f32,
        scale: f32,
        birth_pose: impl Fn(f32, Option<&str>) -> crate::skeletal::Transform,
        enabled: impl Fn(&str, f32, bool) -> bool,
        camera: Vec3,
        atmosphere: &Atmosphere,
    ) {
        self.sample(age, scale, birth_pose, enabled);
        self.steam.prepare_draw(camera, atmosphere);
        let mut order: Vec<_> = self
            .steam
            .puffs
            .iter()
            .enumerate()
            .map(|(i, p)| (i, p.position.distance_squared(camera)))
            .collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (i, _) in order {
            self.steam.draw_one(
                i,
                camera,
                (self.steam.puffs[i].position - camera).normalize_or_zero(),
            );
        }
        gl_use_default_material();
    }
    fn sample(
        &mut self,
        age: f32,
        scale: f32,
        birth_pose: impl Fn(f32, Option<&str>) -> crate::skeletal::Transform,
        enabled: impl Fn(&str, f32, bool) -> bool,
    ) {
        self.steam.puffs.clear();
        let mut previous = std::mem::take(&mut self.births);
        if !age.is_finite() || age < 0. || !scale.is_finite() || scale <= 0. {
            return;
        }
        if age < self.age {
            previous.clear();
        }
        self.age = age;
        let android = crate::android::is_android();
        let max_lookback = if android { 32 } else { 1024 };
        let max_puffs = if android { 48 } else { 4096 };
        for (index, e) in self.steam.emitters.iter().enumerate() {
            if e.spec.burst.is_some_and(|(at, _)| age < at) {
                continue;
            }
            let rate = if android && e.spec.burst.is_none() {
                (e.spec.rate * 0.4).max(e.spec.rate.min(1.))
            } else {
                e.spec.rate
            };
            let last = e.spec.burst.map_or_else(
                || (age * rate).floor() as u32,
                |(_, n)| n.saturating_sub(1),
            );
            let first = if e.spec.burst.is_some() {
                0
            } else {
                ((age - e.spec.life.high).max(0.) * rate).ceil() as u32
            };
            for birth in first.max(last.saturating_sub(max_lookback))..=last {
                if self.steam.puffs.len() >= max_puffs {
                    return;
                }
                let at = e
                    .spec
                    .burst
                    .map_or(birth as f32 / rate, |(at, _)| at);
                if !enabled(&e.spec.name, at, !e.spec.start_off) {
                    continue;
                }
                let mut seed =
                    birth.wrapping_add(1).wrapping_mul(0x9e3779b9) ^ ((index as u32 + 1) * 7919);
                let life = e.spec.life.sample(&mut seed);
                let elapsed = age - at;
                if elapsed >= life {
                    continue;
                }
                let key = (index, birth);
                let pose = previous
                    .remove(&key)
                    .unwrap_or_else(|| birth_pose(at, e.spec.tag.as_deref()));
                self.births.insert(key, pose);
                let offset = sampled(e.spec.offset, &mut seed)
                    + pose.rotation * sampled(e.spec.axis_offset, &mut seed);
                let random_velocity = sampled(e.spec.velocity, &mut seed);
                let velocity = if e.spec.tag_velocity {
                    pose.rotation * random_velocity
                } else {
                    random_velocity
                } + pose.rotation * Vec3::X * e.spec.forward;
                let accel = sampled(e.spec.accel, &mut seed);
                self.steam.puffs.push(Puff {
                    constraint_center: pose.translation + offset,
                    position: pose.translation
                        + offset * scale
                        + (e.spec.min_velocity + velocity * scale) * elapsed
                        + accel * (0.5 * elapsed * elapsed),
                    velocity: Vec3::ZERO,
                    accel: Vec3::ZERO,
                    age: elapsed,
                    life,
                    size: 32. * e.spec.size.sample(&mut seed) * scale,
                    roll: if e.spec.random_roll {
                        random(&mut seed) * std::f32::consts::TAU
                    } else {
                        0.
                    },
                    emitter: index,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn constrained_pixie_moves_reverses_and_stays_inside_after_pause() {
        let mut fixture = attached_fixture(1);
        let steam = &mut fixture.steam;
        let e = &mut steam.emitters[0];
        e.spec.rate = 0.1;
        e.spec.life = Range::fixed(10.);
        e.spec.min_velocity = Vec3::splat(5.);
        e.spec.velocity = [Range { low: -2., high: 2. }; 3];
        e.spec.constrain = Some(Vec3::splat(3.5));
        e.spec.offset[2] = Range::fixed(10.);
        e.scale = 0.7;
        e.carry = 1.;
        let world = World::fixture(&[(Vec3::splat(-20.), Vec3::splat(20.))]);
        assert!(world.sweep(Vec3::ZERO, Vec3::Z, Vec3::ZERO).start_solid);
        steam.update(0.05, Vec3::X * 5000., &world);
        assert!(steam.puffs.is_empty());
        steam.update(0.05, Vec3::ZERO, &world);
        assert_eq!(steam.puffs.len(), 1);
        assert!(steam.puffs[0].velocity.cmpge(Vec3::splat(3.6)).all());
        let start = steam.puffs[0].position;
        for _ in 0..10 {
            steam.update(0.05, Vec3::X * 5000., &world);
            steam.update(0.05, Vec3::ZERO, &world);
            assert_eq!(steam.puffs.len(), 1, "Crossing the range boundary duplicated a pixie");
        }
        let mut reversed = false;
        for _ in 0..160 {
            steam.update(0.05, Vec3::ZERO, &world);
            let p = &steam.puffs[0];
            assert!((p.position - p.constraint_center).abs().cmple(Vec3::splat(3.501)).all());
            reversed |= p.velocity.x < 0.;
        }
        assert!(reversed && steam.puffs[0].position != start);
        let paused = (steam.puffs[0].position, steam.puffs[0].velocity, steam.puffs[0].age);
        for _ in 0..60 { steam.update_clocks(0.05, 0., Vec3::ZERO, &world); }
        assert_eq!(paused, (steam.puffs[0].position, steam.puffs[0].velocity, steam.puffs[0].age));
        steam.update(0.05, Vec3::ZERO, &world);
        assert_ne!(steam.puffs[0].position, paused.0);
    }
    #[test]
    fn negative_constraint_axes_remain_free_and_disabled_emitters_stay_off() {
        let mut fixture = attached_fixture(1);
        let steam = &mut fixture.steam;
        let e = &mut steam.emitters[0];
        e.spec.rate = 1.;
        e.spec.min_velocity = Vec3::splat(5.);
        e.spec.constrain = Some(vec3(0.5, 0.5, -1.));
        e.carry = 1.;
        let world = World::fixture(&[]);
        steam.update(0.05, Vec3::ZERO, &world);
        steam.emitters[0].enabled = false;
        for _ in 0..40 { steam.update(0.05, Vec3::ZERO, &world); }
        assert_eq!(steam.puffs.len(), 1);
        assert!(steam.puffs[0].position.z > 9.);
        assert!(steam.puffs[0].position.x.abs() <= 0.501);
    }
    #[test]
    fn pickup_emitters_follow_visit_visibility_and_collection() {
        let mut a = attached_fixture(2);
        a.steam.emitters[0].pickup = true;
        a.steam.emitters[0].id = 7;
        a.steam.emitters[1].id = 8;
        let p = crate::inventory::Pickup {
            id: "fixture:7".into(), model: "p_m1".into(), origin: Vec3::ZERO,
            kind: crate::inventory::PickupKind::Will, amount: 15.,
        };
        let mut s = crate::inventory::Stats::default();
        a.steam.collected(std::slice::from_ref(&p), &s);
        assert!(a.steam.emitters.iter().all(|e| e.enabled));
        a.steam.collected(&[], &s); // Reserved for another visit.
        assert!(!a.steam.emitters[0].enabled);
        assert!(a.steam.emitters[1].enabled);
        a.steam.emitters[0].enabled = true; // Next event-world sync.
        s.collected.insert(p.id.clone());
        a.steam.collected(&[p], &s);
        assert!(!a.steam.emitters[0].enabled);
        assert!(a.steam.emitters[1].enabled);
    }
    fn attached_fixture(count: usize) -> Attached {
        Attached {
            births: Default::default(),
            age: 0.,
            steam: Steam {
                animations: vec![],
                emitters: (0..count)
                    .map(|_| Emitter {
                        id: 0,
                        pickup: false,
                        origin: Vec3::ZERO,
                        rotation: Quat::IDENTITY,
                        local: crate::skeletal::Transform {
                            translation: Vec3::ZERO,
                            rotation: Quat::IDENTITY,
                        },
                        scale: 1.,
                        spec: Spec {
                            name: "smoke".into(),
                            tag: Some("tag_tip".into()),
                            rate: 240.,
                            life: Range::fixed(30.),
                            start_off: true,
                            ..Spec::default()
                        },
                        light_style: None,
                        light_color: None,
                        textures: vec![],
                        stage: Default::default(),
                        additive: false,
                        carry: 0.,
                        enabled: true,
                        animation_enabled: None,
                    })
                    .collect(),
                puffs: vec![],
                seed: 1,
                materials: vec![],
            },
        }
    }
    #[test]
    fn watch_particle_exceptions_preserve_frozen_births_and_partial_expiry() {
        let mut fixture = attached_fixture(2);
        let steam = &mut fixture.steam;
        steam.emitters[0].spec.no_deadtime = false;
        steam.emitters[1].spec.no_deadtime = true;
        let world = World::fixture(&[]);
        steam.update_clocks(0.05, 0.05, Vec3::ZERO, &world);
        let count = steam.puffs.iter().filter(|p| p.emitter == 0).count();
        assert!(count > 0);
        steam.update_clocks(0.05, 0., Vec3::ZERO, &world);
        assert_eq!(steam.puffs.iter().filter(|p| p.emitter == 0).count(), count);
        assert!(steam
            .puffs
            .iter()
            .filter(|p| p.emitter == 0)
            .all(|p| p.age == 0.));
        assert!(steam.puffs.iter().any(|p| p.emitter == 1 && p.age == 0.05));
        steam.update_clocks(0.05, 0.025, Vec3::ZERO, &world);
        assert_eq!(steam.puffs[0].age, 0.025);
        assert!(
            parse("originemitter e\n(\nno_deadtime\nspawnrate 1\nmodel steam.spr\n)\n")[0]
                .no_deadtime
        );
    }
    #[test]
    fn attached_bursts_keep_exact_birth_count_and_time_on_rebuild() {
        let mut emitter = attached_fixture(1);
        emitter.steam.emitters[0].spec.burst = Some((0.5, 30));
        emitter.steam.emitters[0].spec.life = Range::fixed(1.);
        let pose = |at: f32, _: Option<&str>| crate::skeletal::Transform {
            translation: Vec3::X * at * 100.,
            rotation: Quat::IDENTITY,
        };
        emitter.sample(0.49, 1., pose, |_, _, _| true);
        assert!(emitter.steam.puffs.is_empty());
        emitter.sample(0.5, 1., pose, |_, _, _| true);
        assert_eq!(emitter.steam.puffs.len(), 30);
        emitter.sample(0.75, 1., pose, |_, _, _| true);
        assert_eq!(emitter.steam.puffs.len(), 30);
        assert!(emitter
            .steam
            .puffs
            .iter()
            .all(|p| p.age == 0.25 && p.position.x == 50.));
        emitter.births.clear();
        emitter.sample(0.75, 1., pose, |_, _, _| true);
        assert_eq!(emitter.steam.puffs.len(), 30);
        assert!(emitter.steam.puffs.iter().all(|p| p.position.x == 50.));
        emitter.sample(1.5, 1., pose, |_, _, _| true);
        assert!(emitter.steam.puffs.is_empty());
    }
    #[test]
    fn burst_clips_and_random_lifetimes_are_independent() {
        let text = "animations\n{\nattack a.ska\n{\nclient\n{\n4 tagspawn tag_tip\n(\ncount 7\nmodel smoke.spr\nlife 1 random 0.5\n)\n}\n}\ndeath d.ska\n{\nclient\n{\n87 tagspawn tag_head\n(\ncount 30\nmodel blood.spr\nlife random 1\n)\n}\n}\n}";
        let attack = burst_specs(text, Some("attack"), 0.05);
        let death = burst_specs(text, Some("death"), 0.05);
        assert_eq!(attack.len(), 1);
        assert_eq!(attack[0].burst, Some((0.2, 7)));
        assert_eq!((attack[0].life.low, attack[0].life.high), (1., 1.5));
        assert_eq!(death.len(), 1);
        assert_eq!(death[0].burst, Some((87. * 0.05, 30)));
        assert_eq!((death[0].life.low, death[0].life.high), (0., 1.));
        assert_eq!(death[0].tag.as_deref(), Some("tag_head"));
        assert!(burst_specs(text, Some("idle"), 0.05).is_empty());
        assert_eq!(burst_specs(text, None, 0.05).len(), 2);
    }
    #[test]
    fn attached_events_movement_pause_restore_and_total_budget() {
        let events = crate::animation_events::Model::parse(
            "animations { act a.tan { client { 1 emitteron smoke 6 emitteroff smoke } } }",
        )
        .unwrap();
        let pose = |at: f32, tag: Option<&str>| {
            assert_eq!(tag, Some("tag_tip"));
            crate::skeletal::Transform {
                translation: Vec3::X * (at * 100.),
                rotation: Quat::IDENTITY,
            }
        };
        let enabled = |name: &str, at: f32, default: bool| {
            events
                .visual("act", at, 2., 0.1, false)
                .emitters
                .get(name)
                .copied()
                .unwrap_or(default)
        };
        let snapshot = |a: &Attached| {
            a.steam
                .puffs
                .iter()
                .map(|p| (p.position, p.age, p.size))
                .collect::<Vec<_>>()
        };
        let mut a = attached_fixture(1);
        a.sample(0.05, 1., pose, enabled);
        assert!(a.steam.puffs.is_empty());
        a.sample(0.4, 1., pose, enabled);
        let before = snapshot(&a);
        assert!(!before.is_empty());
        assert!(before.iter().all(|(p, _, _)| p.x >= 10. && p.x <= 40.));
        a.sample(0.4, 1., pose, enabled);
        assert_eq!(
            before,
            snapshot(&a),
            "pause must freeze exact birth samples"
        );
        let clock: f32 = serde_json::from_str(&serde_json::to_string(&0.4_f32).unwrap()).unwrap();
        let mut restored = attached_fixture(1);
        restored.sample(clock, 1., pose, enabled);
        assert_eq!(
            before,
            snapshot(&restored),
            "restore must reproduce the same particles"
        );
        restored.sample(
            0.5,
            1.,
            |at, tag| {
                let mut p = pose(at, tag);
                p.translation.x += 500.;
                p
            },
            enabled,
        );
        assert!(
            restored
                .steam
                .puffs
                .iter()
                .filter(|p| p.age >= 0.1)
                .all(|p| p.position.x <= 40.),
            "moving parent dragged previously emitted particles"
        );
        assert!(
            restored.steam.puffs.iter().any(|p| p.position.x > 540.),
            "moving parent failed to carry new births"
        );
        a.sample(0.9, 1., pose, enabled);
        assert!(
            a.steam.puffs.iter().all(|p| p.position.x < 60.),
            "disabled emitter created new births"
        );
        a.sample(31., 1., pose, enabled);
        assert!(a.steam.puffs.is_empty());
        let mut many = attached_fixture(16);
        many.sample(10., 1., pose, |_, _, _| true);
        assert_eq!(many.steam.puffs.len(), 4096);
        assert_eq!(many.births.len(), 4096);
        many.sample(f32::NAN, 1., pose, |_, _, _| true);
        assert!(many.steam.puffs.is_empty());
    }
    #[test]
    fn light_only_and_invalid_combined_emitters_are_bounded() {
        let p = parse("originemitter flare\n(\ndlight .5 .5 1 1 1\n)\n");
        assert_eq!(p.len(), 1);
        assert!(p[0].light.unwrap().3);
        assert!(parse(
            "originemitter invalid\n(\ndlight .5 .5 1 100\nmodel fire.spr\nspawnrate 999999\n)\n"
        )
        .is_empty());
    }
    #[test]
    fn moving_attachment_only_moves_new_puffs_and_preserves_disabled_state() {
        use crate::{entity::Id, skeletal::Transform};
        let mut steam = Steam {
            animations: vec![],
            emitters: vec![Emitter {
                pickup: false,
                id: 7,
                origin: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                local: Transform {
                    translation: Vec3::X * 3.,
                    rotation: Quat::IDENTITY,
                },
                scale: 1.,
                spec: Spec {
                    rate: 60.,
                    life: Range::fixed(2.),
                    forward: 10.,
                    ..Spec::default()
                },
                light_style: None,
                light_color: None,
                textures: vec![],
                stage: crate::materials::Stage::default(),
                additive: false,
                carry: 0.,
                enabled: true,
                animation_enabled: None,
            }],
            puffs: vec![],
            seed: 12345,
            materials: vec![],
        };
        let pose = Transform {
            translation: vec3(20., 30., 40.),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        };
        steam.place(&[(Id(7), pose)]);
        let world = World::fixture(&[]);
        steam.update(1. / 60., pose.translation, &world);
        assert_eq!(steam.puffs.len(), 1);
        let first = steam.puffs[0].position;
        assert!(first.distance(vec3(20., 33., 40.)) < 0.0001);
        assert!(steam.puffs[0].velocity.distance(Vec3::Y * 10.) < 0.0001);
        steam.place(&[(
            Id(7),
            Transform {
                translation: pose.translation + Vec3::X * 100.,
                ..pose
            },
        )]);
        steam.update(0., pose.translation, &world);
        assert_eq!(steam.puffs[0].position, first);
        steam.update(1. / 60., pose.translation, &world);
        assert_eq!(steam.puffs.len(), 2);
        assert!(steam.puffs[1].position.distance(first + Vec3::X * 100.) < 0.0001);
        steam.gate(&[(Id(7), false)]);
        steam.gate(&[(Id(7), true)]);
        assert!(!steam.emitters[0].enabled);
        steam.update(0.05, pose.translation, &world);
        assert_eq!(steam.puffs.len(), 2);
        assert!(steam.puffs[0].age > 0.);
    }
    #[test]
    fn bounded_multi_emitter_definitions_preserve_ranges_and_acceleration() {
        let s="originemitter steam\n(\nspawnrate 60\nmodel steam.spr\nlife 1 random .35\nrandvel crandom 25 crandom 25 200\n)\ntagemitter tag_base spray\n(\nspawnrate 20\nmodel water.spr\naccel 0 0 -250\nlife 2\nfade\n)\nspawnrate 99999";
        let p = parse(s);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].rate, 60.);
        assert_eq!(p[0].velocity[2].low, 200.);
        assert_eq!(p[0].velocity[0].low, -25.);
        assert_eq!(p[0].life.high, 1.35);
        assert_eq!(p[1].tag.as_deref(), Some("tag_base"));
        assert_eq!(p[1].accel[2].high, -250.);
        assert_eq!(parse(&s.replace("spawnrate 60", "spawnrate 280"))[0].rate, 280.);
        assert_eq!(parse(&s.replace("spawnrate 60", "spawnrate 1025")).len(), 1);
    }
    #[test]
    fn authored_offset_modes_do_not_turn_constants_into_random_offsets() {
        let p =
            parse("originemitter a\n(\nspawnrate 1\nmodel a.spr\noffset crandom 2 random 3 4\n)\n");
        let p = &p[0];
        assert_eq!((p.offset[0].low, p.offset[0].high), (-2., 2.));
        assert_eq!((p.offset[1].low, p.offset[1].high), (0., 3.));
        assert_eq!((p.offset[2].low, p.offset[2].high), (4., 4.));
    }
    #[test]
    fn model_bursts_follow_saved_clock_and_expire_without_rebirth() {
        let source = "animations\n{\nidle timer.tan\n{\nclient\n{\n2 originspawn\n(\ncount 2\nmodel models/test_ring.tik\nlife 1\nscale 1\nscalerate 2\noffset crandom 4 0 0\nfade\n)\n}\n}\n}";
        let bursts = ModelBurst::parse(source, 0.1);
        assert_eq!(bursts.len(), 1);
        let b = &bursts[0];
        assert!(b.sample(0.19, 123).is_empty());
        let p = b.sample(0.45, 123);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].model, "test_ring");
        assert!((p[0].scale - 1.5).abs() < 0.0001);
        assert!((p[0].alpha - 0.75).abs() < 0.0001);
        assert_eq!(p[0].offset, b.sample(0.45, 123)[0].offset);
        assert!(b.sample(1.21, 123).is_empty());
    }
}

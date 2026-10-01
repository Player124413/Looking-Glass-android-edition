//! W9: reviewed timed falling-rock motion. No original script is executed.
use crate::{
    bsp::Bsp,
    collision::Collider,
    combat::{segment_box, Target},
    interaction::vector,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct Node {
    pub point: Vec3,
    pub speed: f32,
    pub thread: Option<String>,
}
pub struct Spec {
    pub origin: Vec3,
    pub nodes: Vec<Node>,
    pub scale: f32,
    pub half: Vec3,
    pub speed: f32,
    pub gravity: f32,
    pub damage: f32,
    pub flags: u32,
    pub wait: f32,
    pub distance: f32,
    pub magnitude: f32,
    pub sound: Option<String>,
}
impl Spec {
    pub fn load(map: &Bsp, name: &str) -> Result<Self> {
        Self::load_inner(map, name, false)
    }
    /// Garden ice rocks have model-derived bounds instead of the marble's server setsize.
    pub fn load_ice(a: &mut crate::assets::Assets, map: &Bsp, name: &str) -> Result<Self> {
        let mut spec = Self::load_inner(map, name, true)?;
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == name))
            .unwrap();
        let d = crate::skeletal::Definition::load(a, &e["model"])?;
        let model =
            crate::tan::Model::parse(&a.read(&format!("{}/{}", d.path, d.animations["idle"]))?)?;
        let half = model
            .surfaces
            .iter()
            .flat_map(|s| &s.frames[0])
            .map(|v| v.abs())
            .reduce(Vec3::max)
            .context("Empty ice rock model")?;
        spec.half = half * d.scale * spec.scale;
        Ok(spec)
    }
    fn load_inner(map: &Bsp, name: &str, ice: bool) -> Result<Self> {
        let find = |n: &str| {
            map.entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|v| v == n))
        };
        let e = find(name).context("Missing falling rock")?;
        let number = |key: &str, default: f32| -> Result<f32> {
            let v = e.get(key).map_or(Ok(default), |s| s.parse::<f32>())?;
            ensure!(v.is_finite() && v >= 0., "Invalid rock {key}");
            Ok(v)
        };
        let flags = number("spawnflags", 0.)? as u32;
        // The non-timed mode requires a separate world-bounce adapter.
        ensure!(
            flags & 2 != 0
                && e.get("model").is_some_and(|s| matches!(
                    s.as_str(),
                    "models/boulder.tik" | "models/marble.tik"
                ) || ice
                    && matches!(
                        s.as_str(),
                        "models/rock_ice_05.tik" | "models/rock_ice_06.tik"
                    )),
            "Unreviewed rock mode/model"
        );
        let scale = number("scale", 1.)?;
        let speed = number("speed", 200.)?;
        ensure!(scale > 0. && speed > 0., "Invalid rock dimensions/speed");
        let mut next = e.get("target");
        let mut seen = BTreeSet::new();
        let mut nodes = Vec::new();
        while let Some(n) = next {
            ensure!(seen.insert(n) && seen.len() < 512, "Cyclic rock path");
            let node = find(n).context("Missing rock waypoint")?;
            let point = node
                .get("origin")
                .and_then(|s| vector(s))
                .context("Invalid rock waypoint")?;
            let speed = node.get("speed").map_or(Ok(0.), |s| s.parse::<f32>())?;
            ensure!(
                point.is_finite() && speed.is_finite() && speed >= 0.,
                "Invalid rock waypoint values"
            );
            nodes.push(Node {
                point,
                speed,
                thread: node.get("thread").cloned(),
            });
            next = node.get("target");
        }
        ensure!(!nodes.is_empty(), "Empty rock path");
        Ok(Self {
            origin: e
                .get("origin")
                .and_then(|s| vector(s))
                .context("Invalid rock origin")?,
            nodes,
            scale,
            // Both reviewed model definitions specify this server setsize. Their
            // rendering scales are independent of this collision envelope.
            half: vec3(120., 120., 64.) * scale,
            speed,
            gravity: 800. * number("gravity", 1.)?,
            damage: number("dmg", 20.)?.trunc(),
            flags,
            wait: number("wait", 0.)?,
            distance: number("distance", 0.)?,
            magnitude: number("magnitude", 0.)?,
            sound: e.get("bouncesound").cloned(),
        })
    }
    fn launch(&self, s: &mut State) {
        let delta = self.nodes[s.node].point - s.position;
        let speed = if s.node == 0 || self.nodes[s.node - 1].speed == 0. {
            self.speed
        } else {
            self.nodes[s.node - 1].speed
        };
        let time = delta.length() / speed;
        s.remaining = if time < 0.1 { 0.2 } else { time };
        s.velocity = delta.normalize_or_zero() * speed;
        s.velocity.z = delta.z / s.remaining + 0.5 * self.gravity * s.remaining;
        s.axis = Vec3::Z.cross(delta).normalize_or_zero();
        s.blocked = false;
    }
    pub fn activate(&self, s: &mut State) -> bool {
        if s.started && !(self.flags & 1 != 0 && s.node == self.nodes.len()) {
            return false;
        }
        if s.started {
            s.position = self.origin;
        }
        s.started = true;
        s.node = 0;
        s.wait = self.wait;
        s.solid = true;
        s.visible = true;
        if s.wait == 0. {
            self.launch(s);
        }
        true
    }
    /// Continuous body sweeps, independent of frame rate. Mode 18 deliberately
    /// excludes solid scenery; contacts do not apply a generic bounce impulse.
    pub fn advance(&self, s: &mut State, dt: f32, bodies: &[Target]) -> Output {
        let mut out = Output::default();
        if !s.started || dt <= 0. {
            return out;
        }
        let mut left = dt;
        while left > 0.000001 {
            if s.wait > 0. {
                let step = left.min(s.wait);
                s.wait = (s.wait - step).max(0.);
                left -= step;
                if s.wait == 0. {
                    self.launch(s);
                }
                continue;
            }
            if s.node == self.nodes.len() && self.flags & 8 != 0 {
                break;
            }
            // Retire escaped rocks without allowing unbounded saved coordinates.
            if s.position.z < -32768. {
                s.visible = false;
                s.solid = false;
                s.velocity = Vec3::ZERO;
                break;
            }
            let h = left.min(1. / 120.).min(if s.node < self.nodes.len() {
                s.remaining
            } else {
                f32::MAX
            });
            s.cooldown = (s.cooldown - h).max(0.);
            let end = s.position + s.velocity * h - Vec3::Z * (0.5 * self.gravity * h * h);
            let contact = if s.solid {
                bodies
                    .iter()
                    .filter_map(|b| {
                        segment_box(
                            s.position,
                            end,
                            b.center,
                            b.half + self.half + Vec3::splat(0.04),
                        )
                        .map(|f| (b.id, f))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
            } else {
                None
            };
            s.position = s.position.lerp(end, contact.map_or(1., |c| c.1));
            if let Some((id, _)) = contact {
                s.blocked = true;
                if s.velocity.length_squared() > 0. && s.cooldown == 0. {
                    out.hits
                        .push((id, self.damage, s.velocity.normalize_or_zero() * 125.));
                    s.cooldown = 0.05;
                }
            }
            s.velocity.z -= self.gravity * h;
            if s.axis.length_squared() > 0. {
                s.rotation = (Quat::from_axis_angle(s.axis, self.speed * h / (64. * self.scale))
                    * s.rotation)
                    .normalize();
            }
            s.elapsed += h;
            left -= h;
            if s.node < self.nodes.len() {
                s.remaining = (s.remaining - h).max(0.);
                if s.remaining < 0.000001 {
                    if !s.blocked {
                        s.position = self.nodes[s.node].point;
                    }
                    out.arrivals.push(s.position);
                    if s.node >= s.callbacks {
                        if let Some(n) = &self.nodes[s.node].thread {
                            out.threads.push(n.clone());
                        }
                        s.callbacks = s.node + 1;
                    }
                    s.node += 1;
                    if s.node < self.nodes.len() {
                        self.launch(s);
                    } else if self.flags & 8 != 0 {
                        s.velocity = Vec3::ZERO;
                    }
                }
            }
        }
        out
    }
    /// Save migration and skip consume the omitted time without emitting old hits/cues.
    pub fn seek(&self, s: &mut State, time: f32) {
        self.activate(s);
        self.advance(s, time.min(120.), &[]);
    }
    pub fn validate(&self, s: &State) -> Result<()> {
        ensure!(
            s.node <= self.nodes.len()
                && s.callbacks <= self.nodes.len()
                && s.position.is_finite()
                && s.position.abs().max_element() <= 100_000.
                && s.velocity.is_finite()
                && s.velocity.abs().max_element() <= 100_000.
                && s.axis.is_finite()
                && s.rotation.is_finite()
                && (s.rotation.length_squared() - 1.).abs() < 0.01
                && [s.remaining, s.wait, s.cooldown, s.elapsed]
                    .iter()
                    .all(|x| x.is_finite() && (0. ..=1e6).contains(x))
                && (!s.started || s.node == self.nodes.len() || s.wait > 0. || s.remaining > 0.),
            "Invalid falling-rock save"
        );
        Ok(())
    }
    pub fn collider(&self, s: &State) -> Option<Collider> {
        (s.visible && s.solid)
            .then(|| Collider::box_bounds(s.position - self.half, s.position + self.half))
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub started: bool,
    pub visible: bool,
    pub solid: bool,
    pub position: Vec3,
    pub velocity: Vec3,
    pub rotation: Quat,
    pub axis: Vec3,
    pub node: usize,
    pub remaining: f32,
    pub wait: f32,
    pub cooldown: f32,
    pub elapsed: f32,
    pub callbacks: usize,
    pub blocked: bool,
}
impl State {
    pub fn new(spec: &Spec, visible: bool, solid: bool) -> Self {
        Self {
            started: false,
            visible,
            solid,
            position: spec.origin,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            axis: Vec3::ZERO,
            node: 0,
            remaining: 0.,
            wait: 0.,
            cooldown: 0.,
            elapsed: 0.,
            callbacks: 0,
            blocked: false,
        }
    }
}
#[derive(Default)]
pub struct Output {
    pub hits: Vec<(usize, f32, Vec3)>,
    pub arrivals: Vec<Vec3>,
    pub threads: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec(flags: u32) -> Spec {
        Spec {
            origin: Vec3::ZERO,
            nodes: vec![
                Node {
                    point: vec3(160., 0., 0.),
                    speed: 80.,
                    thread: Some("first".into()),
                },
                Node {
                    point: vec3(320., 0., 0.),
                    speed: 0.,
                    thread: Some("last".into()),
                },
            ],
            scale: 0.5,
            half: Vec3::splat(4.),
            speed: 160.,
            gravity: 240.,
            damage: 50.,
            flags,
            wait: 0.,
            distance: 600.,
            magnitude: 1.,
            sound: None,
        }
    }
    #[test]
    fn ballistic_apex_waypoint_speed_and_terminal_flags() {
        let d = spec(14);
        let mut s = State::new(&d, true, true);
        d.activate(&mut s);
        let out = d.advance(&mut s, 0.5, &[]);
        assert!(out.arrivals.is_empty());
        assert!((s.position - vec3(80., 0., 30.)).length() < 0.001);
        let out = d.advance(&mut s, 0.5, &[]);
        assert_eq!(out.threads, ["first"]);
        assert_eq!(s.node, 1);
        assert!((s.remaining - 2.).abs() < 0.001);
        let out = d.advance(&mut s, 2., &[]);
        assert_eq!(out.threads, ["last"]);
        assert_eq!(s.position, vec3(320., 0., 0.));
        assert_eq!(s.velocity, Vec3::ZERO);
        assert!(!d.activate(&mut s));
        assert!(d.advance(&mut s, 1., &[]).threads.is_empty());
        let d = spec(6);
        let mut s = State::new(&d, true, true);
        d.seek(&mut s, 3.5);
        assert!(s.position.z < 0. && s.velocity.z < 0.);
    }
    #[test]
    fn swept_actor_contact_has_cooldown_and_stationary_rock_is_harmless() {
        let d = spec(14);
        let mut s = State::new(&d, true, true);
        let body = Target {
            id: 12,
            center: Vec3::ZERO,
            half: Vec3::ONE,
        };
        assert!(d.advance(&mut s, 1., &[body]).hits.is_empty());
        d.activate(&mut s);
        assert_eq!(d.advance(&mut s, 0.04, &[body]).hits.len(), 1);
        let saved = serde_json::to_string(&s).unwrap();
        let mut restored: State = serde_json::from_str(&saved).unwrap();
        d.validate(&restored).unwrap();
        let a = d.advance(&mut s, 0.1, &[body]);
        let b = d.advance(&mut restored, 0.1, &[body]);
        assert_eq!(a.hits, b.hits);
        assert_eq!(
            serde_json::to_value(s).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
    #[test]
    fn pause_and_frame_rates_keep_the_path() {
        let d = spec(6);
        let mut positions = Vec::new();
        for hz in [30, 60, 144] {
            let mut s = State::new(&d, true, true);
            d.activate(&mut s);
            let before = serde_json::to_value(&s).unwrap();
            d.advance(&mut s, 0., &[]);
            assert_eq!(before, serde_json::to_value(&s).unwrap());
            for _ in 0..hz * 2 {
                d.advance(&mut s, 1. / hz as f32, &[]);
            }
            positions.push(s.position);
        }
        assert!(positions.iter().all(|p| p.distance(positions[0]) < 0.02));
    }
}

//! Reviewed bindings and timed paths from the supplied BSP; no scripts are executed.
use super::*;
use crate::collision::{PLAYER_CENTER, PLAYER_HALF};
use std::collections::BTreeSet;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub age: f64,
    pub leaves: [Option<f64>; 2],
    pub clocks: [f64; 7],
    pub ladies: bool,
    pub remove_at: Option<f64>,
    pub removed: bool,
}
impl State {
    pub fn validate(&self) -> Result<()> {
        ensure!((0. ..=1e8).contains(&self.age), "Invalid pond clock");
        ensure!(
            self.clocks
                .iter()
                .all(|t| (0. ..=self.age + 0.001).contains(t)),
            "Invalid pond path clock"
        );
        for t in self.leaves.into_iter().chain([self.remove_at]).flatten() {
            ensure!((0. ..=self.age).contains(&t), "Invalid pond activation");
        }
        ensure!(
            !self.ladies || self.leaves[0].is_some(),
            "Ladybugs before leaf cue"
        );
        ensure!(
            self.remove_at.is_none() || self.leaves[1].is_some(),
            "Removal before leaf launch"
        );
        ensure!(
            !self.removed || self.remove_at.is_some_and(|t| self.age >= t + 2. - 0.001),
            "Early leaf removal"
        );
        Ok(())
    }
}
struct Path {
    spline: crate::fortress::spline::Spline,
    cue: Option<f32>,
}
impl Path {
    fn load(map: &Bsp, root: &str, looping: bool) -> Result<Self> {
        let mut next = root;
        let mut seen = BTreeSet::new();
        let mut nodes = vec![];
        let mut speed = 1.;
        let mut cue = None;
        let mut controls = vec![];
        while seen.insert(next) {
            ensure!(seen.len() <= 256, "Oversized pond path");
            let e = map
                .entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|n| n == next))
                .context("Missing pond node")?;
            ensure!(
                e.get("classname").is_some_and(|s| s == "info_splinepath"),
                "Invalid pond node"
            );
            let at =
                crate::interaction::vector(&e["origin"]).context("Invalid pond node position")?;
            if let Some(s) = e.get("speed") {
                speed = s.parse::<f32>()?;
            }
            ensure!(
                speed.is_finite() && speed > 0. && speed <= 20.,
                "Invalid pond speed"
            );
            if e.get("thread")
                .is_some_and(|n| matches!(n.as_str(), "ladies3and4" | "remove_end_leaf"))
            {
                cue = Some(nodes.len() as f64 - 1.);
            }
            nodes.push(at);
            controls.push((at, Quat::IDENTITY, speed));
            let Some(n) = e.get("target") else { break };
            next = n;
        }
        ensure!(
            nodes.len() >= 2 && (!looping || next == root),
            "Incomplete pond path"
        );
        let spline = crate::fortress::spline::Spline::new(controls, looping);
        let cue = cue.map(|parameter| {
            let (mut lo, mut hi) = (0_f32, 120_f32);
            for _ in 0..28 {
                let mid = (lo + hi) * 0.5;
                if spline.parameter(mid) < parameter {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            hi
        });
        Ok(Self { spline, cue })
    }
    fn pose(&self, time: f64, _angles: bool) -> Transform {
        self.spline.sample(time as f32, false)
    }
}

pub(super) struct Pad {
    pub name: String,
    pub model: usize,
    root: Vec3,
    base: Vec3,
    pub pose: Transform,
    pub collider: Collider,
    path: Path,
}
pub(super) struct Prop {
    pub model: String,
    pub at: Transform,
    pub scale: f32,
    owner: usize,
    local: Vec3,
    yaw: f32,
}
pub(super) struct Data {
    pub pads: Vec<Pad>,
    pub props: Vec<Prop>,
    pub pickup: Vec3,
    pub fish: Collider,
}
fn entity<'a>(map: &'a Bsp, name: &str, class: Option<&str>) -> Result<&'a super::super::Entity> {
    map.entities
        .iter()
        .find(|e| {
            e.get("targetname").is_some_and(|n| n == name)
                && class.is_none_or(|c| e.get("classname").is_some_and(|s| s == c))
        })
        .with_context(|| format!("Missing pond object {name}"))
}
fn origin(e: &super::super::Entity) -> Result<Vec3> {
    crate::interaction::vector(&e["origin"]).context("Invalid pond origin")
}
pub(super) fn owns(e: &super::super::Entity) -> bool {
    e.get("classname").is_some_and(|s| s == "script_object")
        && e.get("targetname").is_some_and(|s| {
            matches!(
                s.as_str(),
                "lily1obj"
                    | "lily2obj"
                    | "lily3obj"
                    | "lily4obj"
                    | "lily5obj"
                    | "leaf1obj"
                    | "leaf2obj"
            )
        })
}
impl Data {
    pub fn load(map: &Bsp) -> Result<Self> {
        let mut pads = vec![];
        let mut props = vec![];
        for (i, name) in [
            "lily1", "lily2", "lily3", "lily4", "lily5", "leaf1", "leaf2",
        ]
        .into_iter()
        .enumerate()
        {
            let e = entity(map, &format!("{name}obj"), Some("script_object"))?;
            let root = origin(entity(map, &format!("{name}org"), None)?)?;
            let base = origin(e)?;
            let model = e["model"].trim_start_matches('*').parse()?;
            pads.push(Pad {
                name: name.into(),
                model,
                root,
                base,
                pose: Transform {
                    translation: base,
                    rotation: Quat::IDENTITY,
                },
                collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                path: Path::load(map, &format!("{name}path"), i < 5)?,
            });
            let p = entity(map, &format!("{name}mdl"), None)?;
            props.push(Prop {
                model: p["model"].trim_end_matches(".tik").into(),
                at: Transform {
                    translation: origin(p)?,
                    rotation: Quat::IDENTITY,
                },
                scale: p.get("scale").and_then(|s| s.parse().ok()).unwrap_or(1.),
                owner: i,
                local: origin(p)? - root,
                yaw: p
                    .get("angle")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.)
                    .to_radians(),
            });
        }
        for name in [
            "lily6",
            "fallingleaf1",
            "fallingleaf2",
            "fallingleaf3",
            "fallingleaf4",
        ] {
            let p = entity(map, &format!("{name}mdl"), None)?;
            props.push(Prop {
                model: p["model"].trim_end_matches(".tik").into(),
                at: Transform {
                    translation: origin(p)?,
                    rotation: Quat::IDENTITY,
                },
                scale: 1.,
                owner: props.len(),
                local: origin(p)?,
                yaw: p
                    .get("angle")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.)
                    .to_radians(),
            });
        }
        let e = &map.entities[4];
        Ok(Self {
            pads,
            props,
            pickup: origin(&map.entities[36])?,
            fish: Collider::model(
                map,
                e["model"].trim_start_matches('*').parse()?,
                origin(e)?,
                Quat::IDENTITY,
                false,
            )?,
        })
    }
    pub fn visible(s: &State, i: usize) -> bool {
        i != 6 || !s.removed
    }
    pub fn rebuild(&mut self, map: &Bsp, s: &State) -> Result<()> {
        for (i, p) in self.pads.iter_mut().enumerate() {
            let mut root = if i < 5 || s.leaves[i - 5].is_some() {
                p.path.pose(s.clocks[i], i >= 4)
            } else {
                Transform {
                    translation: p.root,
                    rotation: if i == 6 {
                        Quat::from_rotation_z(135_f32.to_radians())
                    } else {
                        Quat::IDENTITY
                    },
                }
            };
            // Leaf clips retain their binding offset, with an upright collision deck.
            root.translation += root.rotation * (p.base - p.root);
            if root.translation != p.pose.translation || root.rotation != p.pose.rotation {
                p.collider = Collider::model(map, p.model, root.translation, root.rotation, true)?;
                p.pose = root;
            }
        }
        for p in &mut self.props {
            let i = p.owner;
            if i < 7 {
                let pad = &self.pads[i];
                let root = pad.pose.translation - pad.pose.rotation * (pad.base - pad.root);
                let bob = if i < 4 {
                    let periods = [4., 2., 3., 4.];
                    let phases = [0.3, 0., 0., 0.2];
                    let x = (s.age as f32 * std::f32::consts::TAU / periods[i] + phases[i]).sin();
                    Quat::from_euler(
                        EulerRot::ZYX,
                        10_f32.to_radians() * x,
                        2_f32.to_radians() * x,
                        2_f32.to_radians() * x,
                    )
                } else {
                    Quat::IDENTITY
                };
                p.at = Transform {
                    translation: root
                        + pad.pose.rotation
                            * (vec3(0., 0., 16.) + bob * (p.local - vec3(0., 0., 16.))),
                    rotation: pad.pose.rotation * bob * Quat::from_rotation_z(p.yaw),
                };
            } else if i == 7 {
                let x = (s.age as f32 * std::f32::consts::TAU / 3.).sin();
                p.at = Transform {
                    translation: p.local,
                    rotation: Quat::from_rotation_z(p.yaw)
                        * Quat::from_rotation_x(2_f32.to_radians() * x),
                };
            } else {
                let t = (s.age as f32 - (i - 8) as f32 * 8.6).rem_euclid(34.4);
                let f = ((t - 0.1) / 8.).clamp(0., 1.);
                p.at = Transform {
                    translation: p.local + Vec3::Z * (2048. * (1. - f)),
                    rotation: Quat::from_rotation_z(p.yaw + 0.7 * (t * std::f32::consts::PI).sin())
                        * Quat::from_rotation_x(0.7 * (t * std::f32::consts::PI).sin()),
                };
            }
        }
        Ok(())
    }
    pub fn prop_visible(s: &State, i: usize) -> bool {
        Self::visible(s, i)
            && (i < 8 || {
                let t = (s.age as f32 - (i - 8) as f32 * 8.6).rem_euclid(34.4);
                (0.1..8.6).contains(&t)
            })
    }
    pub fn future(&self, i: usize, t: f64) -> Vec3 {
        let p = &self.pads[i];
        p.path.pose(t, i >= 4).translation + p.base - p.root
    }
    pub fn pickup(&self) -> Vec3 {
        let p = &self.pads[4];
        p.pose.translation + p.pose.rotation * (self.pickup - p.base)
    }
    pub fn colliders(&self, s: &State) -> Vec<Collider> {
        self.pads
            .iter()
            .enumerate()
            .filter(|(i, _)| Self::visible(s, *i))
            .map(|(_, p)| p.collider.clone())
            .collect()
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        s: &mut State,
        w: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let old = s.clone();
        let rider = self.pads.iter().enumerate().find_map(|(i, p)| {
            let h = p.collider.trace(
                player.feet + PLAYER_CENTER,
                player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                PLAYER_HALF,
            );
            (Self::visible(s, i)
                && player.rope.is_none()
                && player.ledge.is_none()
                && player.velocity.z <= 1.
                && !h.start_solid
                && h.fraction < 1.
                && h.normal.z > 0.65)
                .then_some((
                    i,
                    p.pose.rotation.conjugate() * (player.feet - p.pose.translation),
                ))
        });
        s.age += f64::from(dt);
        for i in 0..7 {
            if i < 5 || s.leaves[i - 5].is_some() {
                s.clocks[i] += f64::from(dt);
            }
        }
        self.rebuild(map, s)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders(s)).collect());
        let mut feet = player.feet;
        if let Some((i, local)) = rider {
            if Self::visible(s, i) {
                let p = &self.pads[i];
                let candidate = p.pose.point(local);
                if let Some((f, _)) = p.collider.rider_feet(candidate) {
                    feet = f;
                }
            }
        }
        if !w.body_clear(feet) {
            for h in [1., 2., 4., 8., 12., 18.] {
                let p = feet + Vec3::Z * h;
                if w.body_clear(p) {
                    feet = p;
                    break;
                }
            }
        }
        if w.body_clear(feet) {
            player.feet = feet;
        } else {
            // An obstructed pad waits locally. Other paths, fish and the scene never rewind.
            for (i, p) in self.pads.iter().enumerate() {
                if p.collider
                    .trace(
                        player.feet + PLAYER_CENTER,
                        player.feet + PLAYER_CENTER,
                        PLAYER_HALF,
                    )
                    .start_solid
                    || rider.is_some_and(|(j, _)| j == i)
                {
                    s.clocks[i] = old.clocks[i];
                }
            }
            self.rebuild(map, s)?;
        }
        if self.pads[5]
            .path
            .cue
            .is_some_and(|t| s.clocks[5] >= f64::from(t))
        {
            s.ladies = true;
        }
        if self.pads[6]
            .path
            .cue
            .is_some_and(|t| s.clocks[6] >= f64::from(t))
        {
            s.remove_at.get_or_insert(s.age);
        }
        s.removed = s.remove_at.is_some_and(|t| s.age >= t + 2.);
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders(s)).collect());
        Ok(())
    }
}

//! Saved physical pads, bound clips and the optional collapsing bridge.
use super::*;
use crate::fortress::spline::Spline;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct State {
    pub time: f32,
    pub bridge: Option<f32>,
    pub rabbit: Option<f32>,
    pub rock_seen: bool,
}
impl State {
    pub fn validate(&self) -> Result<()> {
        state::clock("garden world", self.time, 1e7)?;
        for t in [self.bridge, self.rabbit].into_iter().flatten() {
            state::clock("garden activation", t, self.time)?;
        }
        Ok(())
    }
}
pub(super) struct Pad {
    pub name: &'static str,
    pub base: Transform,
    pub scale: f32,
    path: Spline,
    model: usize,
    clip: Transform,
    pub pose: Transform,
    pub collider: Collider,
}
pub(super) struct Brush {
    pub id: usize,
    model: usize,
    base: Transform,
    pose: Option<Transform>,
    collider: Collider,
}
pub(super) struct WorldData {
    pub pads: Vec<Pad>,
    brushes: Vec<Brush>,
}
pub(super) fn owns(e: &super::super::Entity) -> bool {
    let n = e.get("targetname").map_or("", String::as_str);
    matches!(
        n,
        "lily2" | "lily4" | "lily99" | "clip2" | "clip4" | "clip99"
    ) || n
        .strip_prefix("fall")
        .is_some_and(|s| s.parse::<u8>().is_ok_and(|n| (1..=7).contains(&n)))
        || (e.get("classname").is_some_and(|s| s == "script_object")
            && e.get("thread").is_some_and(|s| s == "end_fallingrock"))
}
impl WorldData {
    pub fn load(map: &Bsp) -> Result<Self> {
        let entity = |name: &str| {
            map.entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|n| n == name))
                .with_context(|| format!("Missing garden object {name}"))
        };
        let mut pads = vec![];
        for (name, clip, path) in [
            ("lily2", "clip2", "lily_path1"),
            ("lily4", "clip4", "lily_path2"),
            ("lily99", "clip99", "t395"),
        ] {
            let e = entity(name)?;
            let base = data::at(e);
            let c = entity(clip)?;
            let clip = data::at(c);
            let model = c["model"].trim_start_matches('*').parse()?;
            let mut next = path;
            let mut seen = std::collections::BTreeSet::new();
            let mut nodes = vec![];
            while seen.insert(next) {
                let e = entity(next)?;
                nodes.push((
                    data::at(e).translation,
                    Quat::IDENTITY,
                    e.get("speed").and_then(|s| s.parse().ok()).unwrap_or(1.),
                ));
                next = e.get("target").context("Open garden lily path")?;
            }
            ensure!(next == path && nodes.len() >= 3, "Invalid garden lily loop");
            pads.push(Pad {
                name,
                base,
                scale: e.get("scale").and_then(|s| s.parse().ok()).unwrap_or(1.),
                path: Spline::new(nodes, true),
                model,
                clip,
                pose: base,
                collider: Collider::model(map, model, clip.translation, clip.rotation, true)?,
            });
        }
        let mut brushes = vec![];
        for (id, e) in map.entities.iter().enumerate().filter(|(id, e)| {
            *id == 61 || (owns(e) && e.get("targetname").is_some_and(|n| n.starts_with("fall")))
        }) {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = data::at(e);
            brushes.push(Brush {
                id,
                model,
                base,
                pose: Some(base),
                collider: Collider::model(map, model, base.translation, base.rotation, true)?,
            });
        }
        Ok(Self { pads, brushes })
    }
    pub fn rebuild(&mut self, map: &Bsp, s: &State) -> Result<()> {
        for p in &mut self.pads {
            p.pose = Transform {
                translation: p.path.sample(s.time, false).translation,
                rotation: p.base.rotation,
            };
            // Binding retains the editor-space offset; ignoreangles leaves pad yaw unchanged.
            p.collider = Collider::model(
                map,
                p.model,
                p.clip.translation + p.pose.translation - p.base.translation,
                p.clip.rotation,
                true,
            )?;
        }
        for b in &mut self.brushes {
            let old = b.pose;
            b.pose = Some(b.base);
            if let Some(start) = s.bridge {
                let t = s.time - start;
                if b.id == 61 {
                    continue;
                }
                if t >= 16.6 {
                    b.pose = None;
                    continue;
                }
                let (delay, duration, x, z): (f32, f32, f32, f32) = match b.id {
                    201 => (0.4, 6., -55., 55.),
                    202 => (0.6, 6.4, 0., 55.),
                    205 => (0.7, 6.4, -55., 0.),
                    204 => (1.1, 7.5, 0., 55.),
                    203 => (1.1, 7.5, 55., 0.),
                    200 => (1.7, 4.8, 0., -55.),
                    154 => (2.3, 4.8, 55., 0.),
                    _ => unreachable!(),
                };
                let f = ((t - delay) / duration).clamp(0., 1.);
                b.pose = Some(Transform {
                    translation: b.base.translation - Vec3::Z * 2400. * f,
                    rotation: b.base.rotation
                        * Quat::from_rotation_y((x * f).to_radians())
                        * Quat::from_rotation_x((z * f).to_radians()),
                });
            }
            if let Some(p) = b.pose {
                if old.is_none_or(|v| v.translation != p.translation || v.rotation != p.rotation) {
                    b.collider = Collider::model(map, b.model, p.translation, p.rotation, true)?;
                }
            }
        }
        Ok(())
    }
    pub fn colliders(&self) -> Vec<Collider> {
        self.pads
            .iter()
            .map(|p| p.collider.clone())
            .chain(
                self.brushes
                    .iter()
                    .filter(|b| b.pose.is_some())
                    .map(|b| b.collider.clone()),
            )
            .collect()
    }
    pub fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        // Clips are collision-only; never draw their shader.
        self.brushes
            .iter()
            .filter_map(|b| b.pose.map(|p| (b.model, p.translation, p.rotation)))
            .collect()
    }
    pub fn carry(
        &mut self,
        dt: f32,
        map: &Bsp,
        s: &mut State,
        w: &mut World,
        player: &mut Player,
        fixed: &[Collider],
        portals: &[Collider],
    ) -> Result<()> {
        let rider = self.pads.iter().enumerate().find_map(|(i, p)| {
            let t = p.collider.trace(
                player.feet + crate::collision::PLAYER_CENTER,
                player.feet + crate::collision::PLAYER_CENTER - Vec3::Z * 3.,
                crate::collision::PLAYER_HALF,
            );
            (player.rope.is_none()
                && player.ledge.is_none()
                && player.velocity.z <= 1.
                && !t.start_solid
                && t.fraction < 1.
                && t.normal.z >= 0.65)
                .then_some((i, player.feet - p.pose.translation))
        });
        let bridge_rider = self.brushes.iter().enumerate().find_map(|(i, b)| {
            let pose = b.pose?;
            let t = b.collider.trace(
                player.feet + crate::collision::PLAYER_CENTER,
                player.feet + crate::collision::PLAYER_CENTER - Vec3::Z * 3.,
                crate::collision::PLAYER_HALF,
            );
            (player.rope.is_none()
                && player.ledge.is_none()
                && player.velocity.z <= 1.
                && !t.start_solid
                && t.fraction < 1.
                && t.normal.z >= 0.65)
                .then_some((
                    i,
                    pose.rotation.conjugate() * (player.feet - pose.translation),
                ))
        });
        s.time = (s.time + dt).min(1e7);
        self.rebuild(map, s)?;
        w.set_dynamic(
            fixed
                .iter()
                .cloned()
                .chain(portals.iter().cloned())
                .chain(self.colliders())
                .collect(),
        );
        if let Some((i, offset)) = rider {
            if let Some((feet, normal)) = self.pads[i]
                .collider
                .rider_feet(self.pads[i].pose.translation + offset)
            {
                if w.body_clear(feet) {
                    player.feet = feet;
                    player.grounded = true;
                    player.ground_normal = normal;
                }
            }
        }
        if let Some((i, local)) = bridge_rider {
            let b = &self.brushes[i];
            if let Some(pose) = b.pose {
                if let Some((feet, normal)) = b.collider.rider_feet(pose.point(local)) {
                    if w.body_clear(feet) {
                        player.feet = feet;
                        player.ground_normal = normal;
                        player.grounded = true;
                    }
                }
            }
        }
        Ok(())
    }
}

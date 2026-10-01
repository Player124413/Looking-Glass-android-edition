//! The same authored brush pose supplies drawing and collision.
use super::*;
pub(super) struct Object {
    pub model: usize,
    pub base: Transform,
    pub group: u8,
    pub piece: usize,
    pub pose: Option<Transform>,
    pub collider: Collider,
}
pub(super) fn owns(e: &crate::levels::Entity) -> bool {
    let n = e.get("targetname").map_or("", String::as_str);
    n.starts_with("bridge_fall")
        || n.starts_with("secbridge_fall")
        || n.starts_with("entrance_rock")
        || e.get("classname").is_some_and(|s| s == "func_fulcrum")
        || (e.get("classname").is_some_and(|s| s == "script_object") && n.is_empty())
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Object>> {
    map.entities
        .iter()
        .filter(|e| owns(e) && encounters::allowed(map, e))
        .map(|e| {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = data::at(e);
            let n = e.get("targetname").map_or("", String::as_str);
            let (group, piece) = if let Some(n) = n.strip_prefix("secbridge_fall") {
                (2, n.parse()?)
            } else if let Some(n) = n.strip_prefix("bridge_fall") {
                (1, n.parse()?)
            } else {
                (0, 0)
            };
            Ok(Object {
                model,
                base,
                group,
                piece,
                pose: Some(base),
                collider: Collider::model(map, model, base.translation, base.rotation, true)?,
            })
        })
        .collect()
}
// Piece number, start delay, duration, X rotation, Z rotation. Independent paraphrase.
const FIRST: &[(usize, f32, f32, f32, f32)] = &[
    (1, 0., 5., -55., 55.),
    (2, 0.2, 5.2, 0., 55.),
    (4, 0.3, 5., -55., 0.),
    (3, 0.6, 5., 0., 55.),
    (5, 0.6, 5.4, 55., 0.),
    (6, 1., 5., 0., -55.),
];
const SECOND: &[(usize, f32, f32, f32, f32)] = &[
    (1, 0., 5., -55., 55.),
    (5, 0.2, 5.2, 0., 55.),
    (6, 0.3, 5.4, 55., 0.),
    (2, 0.6, 5., 0., 55.),
    (3, 0.8, 5., 0., 55.),
    (10, 0.8, 5., 0., 55.),
    (7, 1.1, 5., 0., 55.),
    (11, 1.2, 5., 0., 55.),
    (8, 1.4, 5., -55., 0.),
    (4, 1.4, 5.2, 0., 55.),
    (9, 1.5, 5., -55., 0.),
    (12, 1.5, 5.2, 0., 55.),
];
pub(super) fn desired(o: &Object, s: &Saved, collapse: f32) -> Option<Transform> {
    if (o.group == 1 && s.bridge1) || (o.group == 2 && s.bridge2) {
        return None;
    }
    let mut p = o.base;
    if o.model == 18 {
        p.rotation *= Quat::from_rotation_y(s.fulcrum.y.to_radians())
            * Quat::from_rotation_x(s.fulcrum.x.to_radians());
    }
    if let Some(scene) = &s.scene {
        let group = match scene.kind {
            Kind::North | Kind::South => 1,
            Kind::Second => 2,
            _ => 0,
        };
        if group != 0 && o.group == group {
            let piece = if scene.kind == Kind::South {
                7 - o.piece
            } else {
                o.piece
            };
            let (_, delay, duration, x, z) = (if group == 1 { FIRST } else { SECOND })
                .iter()
                .find(|r| r.0 == piece)
                .unwrap();
            let f = ((scene.clock.time - collapse - delay) / duration).clamp(0., 1.);
            p.translation.z -= 2400. * f;
            p.rotation = p.rotation
                * Quat::from_rotation_z((z * f).to_radians())
                * Quat::from_rotation_x((x * f).to_radians());
        }
    }
    Some(p)
}
impl Garden {
    pub(super) fn advance_fulcrum(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let o = self
            .objects
            .iter()
            .find(|o| o.model == 18)
            .context("Missing fulcrum")?;
        let old_pose = o.pose.context("Missing fulcrum pose")?;
        let contact = o.collider.trace(
            player.feet + crate::collision::PLAYER_CENTER,
            player.feet + crate::collision::PLAYER_CENTER - Vec3::Z * 3.,
            crate::collision::PLAYER_HALF,
        );
        let rider = player.velocity.z <= 1.
            && !contact.start_solid
            && contact.fraction < 1.
            && contact.normal.z >= 0.65;
        let local = old_pose.rotation.inverse() * (player.feet - old_pose.translation);
        let target = if rider {
            vec2(-local.y, local.x) / 160. * 12.
        } else {
            Vec2::ZERO
        };
        let old = self.saved.fulcrum;
        self.saved.fulcrum += (target.clamp(Vec2::splat(-12.), Vec2::splat(12.)) - old)
            .clamp(Vec2::splat(-2. * dt), Vec2::splat(2. * dt));
        if self.saved.fulcrum == old {
            return Ok(());
        }
        self.rebuild(map)?;
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        let o = self.objects.iter().find(|o| o.model == 18).unwrap();
        let new = o.pose.unwrap();
        let carried = new.translation + new.rotation * local;
        let landing = if rider {
            o.collider.rider_feet(carried)
        } else {
            None
        };
        let next = landing.map_or(player.feet, |p| p.0);
        if w.body_clear(next) && (!rider || landing.is_some()) {
            if let Some((feet, normal)) = landing {
                player.feet = feet;
                player.grounded = true;
                player.ground_normal = normal;
            }
        } else {
            self.saved.fulcrum = old;
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        }
        Ok(())
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let collapse = self.data.collapse();
        for o in &mut self.objects {
            let p = desired(o, &self.saved, collapse);
            if let Some(p) = p {
                if o.pose.is_none_or(|old| {
                    old.translation != p.translation || old.rotation != p.rotation
                }) {
                    o.collider = Collider::model(map, o.model, p.translation, p.rotation, true)?;
                }
            }
            o.pose = p;
        }
        Ok(())
    }
}

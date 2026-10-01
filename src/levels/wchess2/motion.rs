use super::*;
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct DoorState {
    pub id: usize,
    pub amount: f32,
    pub opening: bool,
    pub hold: f32,
    pub sign: f32,
    pub started: Option<f32>,
}
impl DoorState {
    pub fn open(&mut self) {
        self.opening = true;
    }
    pub fn validate(&self) -> Result<()> {
        state::clock("door fraction", self.amount, 1.)?;
        state::clock("door wait", self.hold, 10.)?;
        ensure!(
            self.sign == 1. || self.sign == -1.,
            "Invalid door direction"
        );
        if let Some(t) = self.started {
            state::clock("door movement epoch", t, 1e7)?;
        }
        Ok(())
    }
}
pub(super) struct Object {
    pub id: usize,
    pub model: usize,
    pub name: String,
    pub base: Transform,
    pub pose: Transform,
    pub draw: bool,
    pub solid: bool,
    pub collider: Collider,
    pub door: Option<usize>,
    pub bounds: (Vec3, Vec3),
    pub flags: u32,
    pub locked: bool,
    pub duration: f32,
    pub wait: f32,
    pub slide: Vec3,
}
pub(super) fn owns(e: &super::super::Entity) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("func_rotatingdoor" | "func_door")
    ) || e
        .get("targetname")
        .is_some_and(|n| matches!(n.as_str(), "exit_portal" | "red_platform"))
}
pub(super) fn load(map: &Bsp) -> Result<(Vec<Object>, Vec<DoorState>)> {
    let mut objects = Vec::new();
    let mut doors = Vec::new();
    for (id, e) in map.entities.iter().enumerate().filter(|(_, e)| owns(e)) {
        let model: usize = e["model"].trim_start_matches('*').parse()?;
        let base = Transform {
            rotation: Quat::IDENTITY,
            ..data::at(e)
        };
        let flags = e
            .get("spawnflags")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let door = if e["classname"].starts_with("func_") {
            let k = doors.len();
            doors.push(DoorState {
                id,
                amount: 0.,
                opening: false,
                hold: 0.,
                sign: 1.,
                started: None,
            });
            Some(k)
        } else {
            None
        };
        let m = &map.models[model];
        let bounds = (base.translation + m.min, base.translation + m.max);
        let angle = e
            .get("angle")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.);
        let direction = if angle == -1. {
            Vec3::Z
        } else if angle == -2. {
            -Vec3::Z
        } else {
            vec3(angle.to_radians().cos(), angle.to_radians().sin(), 0.)
        };
        let slide = if e["classname"] == "func_door" {
            direction * ((m.max - m.min).dot(direction.abs()) - 8.).max(0.)
        } else {
            Vec3::ZERO
        };
        objects.push(Object {
            id,
            model,
            name: e.get("targetname").cloned().unwrap_or_default(),
            base,
            pose: base,
            draw: true,
            solid: true,
            collider: Collider::model(map, model, base.translation, base.rotation, true)?,
            door,
            bounds,
            flags,
            locked: e.get("targetname").is_some_and(|s| s == "locked_doors"),
            duration: e.get("time").and_then(|s| s.parse().ok()).unwrap_or(1.5),
            wait: e.get("wait").and_then(|s| s.parse().ok()).unwrap_or(3.),
            slide,
        });
    }
    Ok((objects, doors))
}
impl Castle {
    pub(super) fn portal_pose(&self) -> Transform {
        if let Some(t) = self.saved.portal {
            let mut p = self.data.portal.sample(self.saved.age - t, false);
            p.rotation = Quat::IDENTITY;
            p
        } else {
            self.data.point("exit_portal")
        }
    }
    pub(super) fn portal_support(&self, p: &Player) -> bool {
        if !self.saved.king || self.scripted() {
            return false;
        }
        self.objects.iter().find(|o| o.id == 23).is_some_and(|o| {
            let tr = o.collider.trace(
                p.feet + PLAYER_CENTER,
                p.feet + PLAYER_CENTER - Vec3::Z * 3.,
                PLAYER_HALF,
            );
            !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.7
        })
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let portal = self.portal_pose();
        let queen_t = self
            .saved
            .scene
            .as_ref()
            .filter(|s| s.kind == scene::Kind::Queen)
            .map_or(0., |s| s.time);
        for o in &mut self.objects {
            let old = o.pose;
            o.pose = o.base;
            o.draw = true;
            o.solid = true;
            if let Some(k) = o.door {
                let d = &self.saved.doors[k];
                if o.slide != Vec3::ZERO {
                    o.pose.translation += o.slide * d.amount;
                } else {
                    o.pose.rotation =
                        Quat::from_rotation_z(d.amount * d.sign * std::f32::consts::FRAC_PI_2);
                }
            }
            if o.id == 23 {
                o.pose = portal;
                o.draw = self.saved.king;
                o.solid = self.saved.king;
            }
            if o.id == 31 {
                o.draw = !self.saved.queen;
                o.solid = !self.saved.queen;
                o.pose.translation.x -= 512. * ((queen_t - 15.25) / 5.).clamp(0., 1.);
            }
            if old.translation != o.pose.translation || old.rotation != o.pose.rotation {
                o.collider =
                    Collider::model(map, o.model, o.pose.translation, o.pose.rotation, true)?;
            }
        }
        Ok(())
    }
    pub(super) fn use_door(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        self.objects.iter().find_map(|o| {
            let k = o.door?;
            if o.locked || o.flags & (64 | 128 | 8) != 0 {
                return None;
            }
            let target = (o.bounds.0 + o.bounds.1) * 0.5;
            let d = target - eye;
            if d.length() > 150. || d.normalize_or_zero().dot(aim.normalize_or_zero()) < 0.4 {
                return None;
            }
            let tr = w.sweep(eye, target, Vec3::splat(0.5));
            // The selected door itself is the final surface; a wall before it rejects use.
            let leaf = o.collider.trace(eye, target, Vec3::splat(0.5));
            (!tr.start_solid && tr.fraction + 0.02 >= leaf.fraction).then_some(k)
        })
    }
    pub(super) fn step_doors(&mut self, dt: f32, p: &Player) {
        for o in &self.objects {
            let Some(k) = o.door else {
                continue;
            };
            let d = &mut self.saved.doors[k];
            let lo = o.bounds.0 - vec3(60., 60., 8.) - PLAYER_HALF;
            let hi = o.bounds.1 + vec3(60., 60., 8.) + PLAYER_HALF;
            let at = p.feet + PLAYER_CENTER;
            let touch = at.cmpge(lo).all() && at.cmple(hi).all();
            let actor_touch = o.flags & 16 == 0
                && self.saved.cast.iter().any(|a| {
                    a.piece.active && a.piece.health > 0. && {
                        let at = a.piece.feet + Vec3::Z * 32.;
                        at.cmpge(lo).all() && at.cmple(hi).all()
                    }
                });
            let scripted = o.name == "castle_frontdoor"
                && self
                    .saved
                    .scene
                    .as_ref()
                    .is_some_and(|s| s.kind == scene::Kind::Queen);
            if !o.locked
                && ((o.flags & 0xa4 == 0
                    && ((touch && o.flags & 64 != 0 && o.flags & 8 == 0) || actor_touch))
                    || scripted)
            {
                if d.amount == 0. {
                    let local = (o.bounds.0 + o.bounds.1) * 0.5 - o.base.translation;
                    let plus = o.base.translation
                        + Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * local;
                    let minus = o.base.translation
                        + Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * local;
                    d.sign = if plus.distance_squared(p.eye()) >= minus.distance_squared(p.eye()) {
                        1.
                    } else {
                        -1.
                    };
                    if o.flags & 2 != 0 {
                        d.sign = -d.sign;
                    }
                }
                d.open();
                d.hold = o.wait;
            }
            let before = d.amount;
            let step = dt / o.duration;
            if d.opening {
                d.amount = (d.amount + step).min(1.);
                if d.amount == 1. && !touch && !actor_touch && !scripted {
                    d.hold = (d.hold - dt).max(0.);
                    if d.hold == 0. && o.wait > 0. && (o.flags & 32 == 0 || o.flags & 4096 != 0) {
                        d.opening = false;
                    }
                }
            } else {
                d.amount = (d.amount - step).max(0.);
            }
            if d.amount != before && (before == 0. || before == 1.) {
                d.started = Some(self.saved.age);
            }
        }
    }
}

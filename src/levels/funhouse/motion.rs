use super::*;
pub(super) struct Object {
    pub id: usize,
    pub model: usize,
    pub name: String,
    pub base: Transform,
    pub pose: Transform,
    pub collider: Collider,
    shape: crate::collision::model_shape::Template,
    pub draw: bool,
    pub solid: bool,
    pub static_support: bool,
    class: String,
    flags: u32,
    bounds: (Vec3, Vec3),
    slide: Vec3,
    duration: f32,
    wait: f32,
}
pub fn owns(e: &super::super::Entity) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some(
            "script_object"
                | "func_door"
                | "func_rotatingdoor"
                | "func_fulcrum"
                | "func_smashablewall"
        )
    )
}
fn num(e: &super::super::Entity, key: &str, default: f32) -> f32 {
    e.get(key).and_then(|s| s.parse().ok()).unwrap_or(default)
}
pub fn load(map: &Bsp) -> Result<Vec<Object>> {
    map.entities
        .iter()
        .enumerate()
        .filter(|(_, e)| owns(e) && e.get("model").is_some_and(|m| m.starts_with('*')))
        .map(|(id, e)| {
            let model = e["model"][1..].parse::<usize>()?;
            let mut base = data::pose(e);
            base.rotation = Quat::IDENTITY;
            let m = &map.models[model];
            let angle = num(e, "angle", 0.);
            let dir = if angle == -1. {
                Vec3::Z
            } else if angle == -2. {
                -Vec3::Z
            } else {
                vec3(angle.to_radians().cos(), angle.to_radians().sin(), 0.)
            };
            let slide = dir * ((m.max - m.min).dot(dir.abs()) - num(e, "lip", 8.)).max(0.);
            let shape = crate::collision::model_shape::Template::brush_model(map, model)?;
            Ok(Object {
                id,
                model,
                name: e.get("targetname").cloned().unwrap_or_default(),
                base,
                pose: base,
                collider: shape.at(base.translation, base.rotation),
                shape,
                draw: true,
                solid: true,
                static_support: false,
                class: e["classname"].clone(),
                flags: num(e, "spawnflags", 0.) as u32,
                bounds: (base.translation + m.min, base.translation + m.max),
                slide,
                duration: num(e, "time", slide.length() / num(e, "speed", 100.).max(1.)).max(0.01),
                wait: num(e, "wait", 1.5),
            })
        })
        .collect()
}
fn approach(x: f32, y: f32, d: f32) -> f32 {
    if x < y {
        (x + d).min(y)
    } else {
        (x - d).max(y)
    }
}
impl Funhouse {
    pub fn rebuild(&mut self, _map: &Bsp) -> Result<()> {
        let s = &self.saved;
        let tube = s
            .tube
            .map_or(0., |t| ((s.time - t - 0.25) / 8.).clamp(0., 1.));
        for (i, o) in self.objects.iter_mut().enumerate() {
            let old = o.pose;
            o.pose = o.base;
            o.draw = true;
            // Native ScriptSlave bit 1 posts the non-solid event after construction.
            o.solid = !(o.class == "script_object" && o.flags & 1 != 0);
            o.static_support = false;
            let name = o.name.as_str();
            match o.class.as_str() {
                "func_rotatingdoor" => {
                    let local = (o.bounds.0 + o.bounds.1) * 0.5 - o.base.translation;
                    let h = if local.x.abs() > local.y.abs() {
                        local.x
                    } else {
                        -local.y
                    };
                    o.pose.rotation = Quat::from_rotation_z(
                        s.doors[i]
                            * s.door_signs
                                .get(i)
                                .copied()
                                .unwrap_or(if h < 0. { -1. } else { 1. })
                            * std::f32::consts::FRAC_PI_2,
                    );
                }
                "func_door" => {
                    let fraction = match name {
                        "mirror_path_door" => s.doors[i],
                        "t96" => {
                            if o.flags & 1 != 0 {
                                1. - s.doors[i]
                            } else {
                                s.doors[i]
                            }
                        }
                        _ => 0.,
                    };
                    o.pose.translation += o.slide * fraction;
                }
                "func_fulcrum" => {
                    let tilt = s.fulcrums.get(i).copied().unwrap_or_default();
                    o.pose.rotation = Quat::from_rotation_y(tilt.y.to_radians())
                        * Quat::from_rotation_x(tilt.x.to_radians());
                    o.static_support = tilt == Vec2::ZERO;
                }
                _ => (),
            }
            if let Some(c) = name
                .strip_prefix("clock_cell")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|i| (1..=8).contains(i))
            {
                o.draw = s.cells & (1 << (c - 1)) == 0;
                o.solid = o.draw;
            }
            if let Some(c) = name
                .strip_prefix("pend_cell")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|i| (1..=8).contains(i))
            {
                if s.cells & (1 << (c - 1)) == 0 {
                    let angle = (15. * (s.time * std::f32::consts::TAU).sin()).to_radians();
                    o.pose.rotation = if [1, 2, 7, 8].contains(&c) {
                        Quat::from_rotation_y(angle)
                    } else {
                        Quat::from_rotation_x(angle)
                    };
                }
            }
            if let Some(i) = CLOSETS.iter().position(|c| name == format!("{c}_wall")) {
                o.draw = s.closets[i].is_none();
                o.solid = o.draw;
            }
            match name {
                "clock_mirror" => {
                    o.draw = !s.mirror;
                    o.solid = o.draw;
                }
                "pend_mirror" => {
                    if !s.mirror {
                        o.pose.rotation = Quat::from_rotation_y(
                            (15. * (s.time * std::f32::consts::TAU).sin()).to_radians(),
                        );
                    }
                }
                "cin_clock_break" => {
                    o.draw = !s.arrived
                        && s.scene
                            .as_ref()
                            .is_none_or(|sc| sc.kind != Kind::Arrival || sc.clock.time < 14.5);
                    o.solid = o.draw;
                }
                "cine_stand" => {
                    let f = s
                        .stand
                        .map_or(0., |t| ((s.time - t - 2.) / 0.5).clamp(0., 1.));
                    o.pose.translation.y -= 64. * f;
                }
                "tuber" => {
                    let pivot = self.data.points["tuber_bind"].translation;
                    let r = Quat::from_rotation_z((-90. * tube).to_radians());
                    o.pose.translation = pivot + r * (o.base.translation - pivot);
                    o.pose.rotation = r;
                    o.static_support = tube == 0. || tube == 1.;
                }
                "biggear1" | "biggear1_bind" => {
                    let pivot = self.data.points["biggear1"].translation;
                    let r = Quat::from_rotation_z((s.time * 30.).to_radians());
                    o.pose.translation = pivot + r * (o.base.translation - pivot);
                    o.pose.rotation = r;
                }
                "broken_pipe" => {
                    o.draw = false;
                }
                "gas_door1" | "gas_door2" => {
                    let t = s.gas_start.map_or(0., |t| s.time - t);
                    if s.gas || t >= 8.3 {
                        o.draw = false;
                        o.solid = false;
                    } else if t > 7.7 {
                        let dest = self.data.points.get(if name == "gas_door1" {
                            "gas_door1_blowout"
                        } else {
                            "gas_door2_blowout"
                        });
                        if let Some(p) = dest {
                            let f = ((t - 7.7) / 0.5).min(1.);
                            o.pose.translation = o.base.translation.lerp(p.translation, f);
                            o.pose.rotation =
                                Quat::from_euler(EulerRot::XYZ, f * 6.28, f * 6.28, f * 6.28);
                        }
                    }
                }
                "head2_teeth" => {
                    let t = s.time.rem_euclid(7.);
                    let down = if t < 2. {
                        0.
                    } else if t < 2.5 {
                        (t - 2.) * 2.
                    } else if t < 4. {
                        1.
                    } else {
                        (1. - (t - 4.) * 2.).max(0.)
                    };
                    o.pose.translation.z -= 100. * down;
                }
                "pend1" | "pend2" | "pend3" => {
                    let (amp, phase) = if name == "pend2" {
                        (12., 0.5)
                    } else {
                        (24., 0.)
                    };
                    o.pose.rotation = Quat::from_rotation_x(
                        (amp * ((s.time / 8. + phase) * std::f32::consts::TAU).sin()).to_radians(),
                    );
                }
                "end_door1" | "end_door2" => {
                    let f = s
                        .scene
                        .as_ref()
                        .filter(|s| s.kind == Kind::Tweedles)
                        .map_or(0., |s| {
                            if s.clock.time < 2.5 {
                                (s.clock.time / 1.5).min(1.)
                            } else {
                                (1. - (s.clock.time - 2.5) / 0.1).max(0.)
                            }
                        });
                    o.pose.rotation = Quat::from_rotation_z(
                        f * if name == "end_door1" { 90f32 } else { -90f32 }.to_radians(),
                    );
                }
                "break_plat" => {
                    if let Some(t) = s.floor {
                        o.pose.translation.z -= 512. * ((s.time - t - 2.) / 2.).clamp(0., 1.);
                    }
                }
                _ => (),
            }
            if let Some(j) = name
                .strip_prefix('t')
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| (176..=185).contains(n))
            {
                let deltas = [
                    (-500., -300., -200., 1., 100.),
                    (-300., -500., -200., 1., 100.),
                    (-300., -400., -200., 1., 50.),
                    (-500., -400., -200., 0.8, 100.),
                    (-400., -100., -100., 1., -50.),
                    (-100., -300., -300., 1.2, -100.),
                    (-400., -200., -100., 0.6, 0.),
                    (0., -400., -300., 1., 0.),
                    (-400., -500., 0., 1.2, 0.),
                    (-500., -200., 300., 1.2, 0.),
                ];
                let (x, y, z, d, r) = deltas[j - 176];
                let f = if s.gas {
                    1.
                } else {
                    s.gas_start
                        .map_or(0., |t| ((s.time - t - 7.6) / d).clamp(0., 1.))
                };
                o.pose.translation += vec3(x, y, z) * f;
                o.pose.rotation = Quat::from_rotation_y((r * f).to_radians());
            }
            if name == "pipe_hide" {
                o.draw = !s.gas && s.gas_start.is_none_or(|t| s.time < t + 7.6);
            }
            if name.starts_with("icefloor2nd") || name.starts_with("marble_ice") {
                if let Some(t) = s.floor {
                    let (delay, duration, rx, ry) = floor_motion(name);
                    let f = ((s.time - t - delay) / duration).clamp(0., 1.);
                    o.pose.translation.z -= 2400. * f;
                    o.pose.rotation = Quat::from_rotation_x((rx * f).to_radians())
                        * Quat::from_rotation_y((ry * f).to_radians());
                } else {
                    o.static_support = true;
                }
            }
            if old.translation != o.pose.translation || old.rotation != o.pose.rotation {
                o.collider = o.shape.at(o.pose.translation, o.pose.rotation);
            }
        }
        Ok(())
    }
    pub fn move_world(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        // A board that was at rest on the previous frame is about to move.
        // Remove its old hanging-support copy before testing the new pose.
        w.set_settled_supports(vec![]);
        if self.saved.door_signs.is_empty() {
            self.saved.door_signs = self
                .objects
                .iter()
                .map(|o| {
                    let local = (o.bounds.0 + o.bounds.1) * 0.5 - o.base.translation;
                    let h = if local.x.abs() > local.y.abs() {
                        local.x
                    } else {
                        -local.y
                    };
                    if h < 0. {
                        -1.
                    } else {
                        1.
                    }
                })
                .collect();
        }
        if self.saved.fulcrums.is_empty() {
            self.saved.fulcrums = vec![Vec2::ZERO; self.objects.len()];
        }
        let before = self.saved.clone();
        let mut rider = None;
        if !self.scripted() && p.grounded {
            let c = p.feet + PLAYER_CENTER;
            rider = self
                .objects
                .iter()
                .enumerate()
                .find(|(_, o)| {
                    // The entrance stand withdraws to drop Alice into the corridor.
                    // It is not a passenger lift.
                    o.solid && o.name != "cine_stand" && {
                        let t = o.collider.trace(c, c - Vec3::Z * 3., PLAYER_HALF);
                        t.fraction < 1. && t.normal.z > 0.65
                    }
                })
                .map(|(i, o)| {
                    (
                        i,
                        o.pose.rotation.conjugate() * (p.feet - o.pose.translation),
                    )
                });
        }
        self.saved.time += dt;
        for (i, o) in self.objects.iter().enumerate() {
            if o.class == "func_fulcrum" {
                let speed = num(&map.entities[o.id], "speed", 48.);
                let limit = num(&map.entities[o.id], "limit", 90.).clamp(0., 90.);
                let old = self.saved.fulcrums[i];
                let velocity = if let Some((_, local)) = rider.filter(|(j, _)| *j == i) {
                    let b = &map.models[o.model];
                    let span = b.min.abs().max(b.max.abs()).max(Vec3::ONE);
                    let mut v = vec2(-local.y / span.y, local.x / span.x)
                        .clamp(Vec2::splat(-1.), Vec2::ONE)
                        * speed;
                    if o.flags & 2 != 0 {
                        v.y = 0.;
                    }
                    if o.flags & 4 != 0 {
                        v.x = 0.;
                    }
                    v
                } else {
                    -old * num(&map.entities[o.id], "resetspeed", speed * 0.002)
                        * 20.
                        * num(&map.entities[o.id], "dampening", 0.95)
                };
                let mut tilt = (old + velocity * dt).clamp(Vec2::splat(-limit), Vec2::splat(limit));
                if rider.is_none_or(|(j, _)| j != i) && tilt.abs().max_element() < 0.01 {
                    tilt = Vec2::ZERO;
                }
                self.saved.fulcrums[i] = tilt;
            }
            // 4096 is auto-close, not a lock. Bit 16 excludes Actors, not players.
            // Both the 4176 castle doors and 4160 cell doors admit player proximity.
            let target = if o.class == "func_rotatingdoor"
                && o.flags & 64 != 0
                && o.flags & (4 | 8 | 32 | 128) == 0
            {
                let c = p.feet + PLAYER_CENTER;
                let field = vec3(60., 60., 8.) + PLAYER_HALF;
                let close = c.cmpge(o.bounds.0 - field).all() && c.cmple(o.bounds.1 + field).all();
                if close {
                    if self.saved.doors[i] == 0. {
                        // As with the shared doors, choose the side away from the activator.
                        let local = (o.bounds.0 + o.bounds.1) * 0.5 - o.base.translation;
                        let plus = o.base.translation
                            + Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * local;
                        let minus = o.base.translation
                            + Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * local;
                        self.saved.door_signs[i] =
                            if plus.distance_squared(c) >= minus.distance_squared(c) {
                                1.
                            } else {
                                -1.
                            };
                    }
                    self.saved.holds[i] = o.wait.max(0.);
                } else {
                    self.saved.holds[i] = (self.saved.holds[i] - dt).max(0.);
                }
                if self.saved.holds[i] > 0. {
                    1.
                } else {
                    0.
                }
            } else if o.name == "mirror_path_door" {
                self.saved.mirror_open as u8 as f32
            } else if o.name == "t96" {
                self.saved.t96 as u8 as f32
            } else {
                0.
            };
            self.saved.doors[i] = approach(self.saved.doors[i], target, dt / o.duration);
        }
        self.rebuild(map)?;
        // A blocked leaf pauses only its own motion. Closing reverses without crush
        // damage; unrelated suction, tube and scene clocks must continue.
        let mut door_blocked = false;
        if !self.scripted() {
            let c = p.feet + PLAYER_CENTER;
            for (i, o) in self.objects.iter().enumerate() {
                if matches!(o.class.as_str(), "func_door" | "func_rotatingdoor")
                    && self.saved.doors[i] != before.doors[i]
                    && o.collider.trace(c, c, PLAYER_HALF).start_solid
                {
                    if self.saved.doors[i] < before.doors[i] {
                        self.saved.holds[i] = o.wait.max(1.5);
                    }
                    self.saved.doors[i] = before.doors[i];
                    door_blocked = true;
                }
            }
        }
        if door_blocked {
            self.rebuild(map)?;
        }
        let mut blocked = false;
        if let Some((i, local)) = rider {
            let raw = self.objects[i].pose.point(local);
            let tilting = self.objects[i].class == "func_fulcrum"
                || matches!(self.objects[i].name.as_str(), "pend1" | "pend2" | "pend3");
            let to = if tilting {
                self.objects[i]
                    .collider
                    .rider_feet(raw)
                    .map_or(raw, |v| v.0)
            } else {
                raw
            };
            w.set_dynamic(fixed.to_vec());
            let tr = w.sweep(p.feet + PLAYER_CENTER, to + PLAYER_CENTER, PLAYER_HALF);
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            if tr.start_solid || tr.fraction < 0.999 || !w.body_clear(to) {
                if std::env::var_os("LOOKING_GLASS_FUNHOUSE_MOVER_TRACE").is_some() {
                    static LOGS: std::sync::atomic::AtomicUsize =
                        std::sync::atomic::AtomicUsize::new(0);
                    if LOGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 12 {
                        println!(
                            "MOVER blocked id {} feet {:?} raw {:?} to {:?} trace {:?} ground {:?}",
                            self.objects[i].id,
                            p.feet,
                            raw,
                            to,
                            tr,
                            self.objects[i].collider.rider_feet(raw)
                        );
                    }
                }
                // The whole saved world step rolls back: an obstructed rider cannot be crushed or detached.
                self.saved = before.clone();
                self.rebuild(map)?;
                blocked = true;
            } else {
                p.feet = to;
                if tilting {
                    if let Some((_, normal)) = self.objects[i].collider.rider_feet(to) {
                        // Walking velocity was tangent to yesterday's slope. Keep it
                        // tangent to the carried support so a normal jump remains available.
                        p.ground_normal = normal;
                        p.velocity.z =
                            -(normal.x * p.velocity.x + normal.y * p.velocity.y) / normal.z;
                        p.grounded = true;
                    }
                }
            }
        }
        if rider.is_none() && !self.scripted() {
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
            if !w.body_clear(p.feet) && !blocked {
                self.saved = before.clone();
                self.rebuild(map)?;
            }
        }
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        w.set_settled_supports(self.settled_supports());
        Ok(())
    }
    pub fn particle_state(&self, p: &mut crate::particles::Steam) {
        let mut gates = vec![];
        let mut poses = vec![];
        for (id, name, base) in &self.data.emitters {
            let active = if name.starts_with("head") && name.contains("steam") {
                self.saved.time.rem_euclid(7.) < 4.
            } else if let Some(i) = name
                .strip_prefix("steam")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|i| (1..=7).contains(i))
            {
                let t = self.saved.gas_start.map_or(-1., |t| self.saved.time - t);
                let times = [2., 4., 5., 6., 6.5, 6.8, 7.1];
                !self.saved.gas && t >= times[i - 1] && t < if i <= 5 { 8.6 } else { 9.6 }
            } else if name.contains("gas") {
                if let Some(i) = name
                    .chars()
                    .find_map(|c| c.to_digit(10))
                    .filter(|i| (1..=8).contains(i))
                {
                    self.saved.cells & (1 << (i - 1)) == 0
                } else {
                    false
                }
            } else {
                continue;
            };
            gates.push((Id(*id), active));
            let bind = match name.as_str() {
                "steam1" => "t183",
                "steam3" => "t177",
                "steam4" => "t180",
                "steam5" => "t184",
                _ => "",
            };
            if let Some(o) = self.objects.iter().find(|o| o.name == bind) {
                poses.push((
                    Id(*id),
                    Transform {
                        translation: o.pose.point(base.translation - o.base.translation),
                        rotation: o.pose.rotation * base.rotation,
                    },
                ));
            }
        }
        p.gate(&gates);
        p.place(&poses);
    }
}
fn floor_motion(n: &str) -> (f32, f32, f32, f32) {
    if let Some(i) = n
        .strip_prefix("marble_ice")
        .and_then(|s| s.parse::<usize>().ok())
    {
        return match i {
            1 => (0., 6., 55., -55.),
            2 => (0.3, 6.4, 55., 0.),
            3 => (0.4, 6.4, 0., -55.),
            _ => (0.7, 7.5, 55., 0.),
        };
    }
    let i = n
        .strip_prefix("icefloor2nd")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 19)
        - 1;
    let delays = [
        0., 0.2, 0.3, 0.6, 0.8, 0.8, 0.9, 1.1, 1.1, 1.4, 1.4, 1.5, 1.7, 1.7, 1.9, 2.2, 2.2, 2.3,
        2.6,
    ];
    let x = [
        55., 0., 55., 45., 60., -55., 55., 0., 35., 55., 45., -35., 45., 0., 55., 55., 45., 55.,
        45.,
    ];
    let y = [
        -55., -55., 0., -45., 0., 0., -55., -55., 0., 0., -45., 0., -45., -55., -45., 0., -45., 0.,
        -45.,
    ];
    (0.7 + delays[i], 2., x[i], y[i])
}

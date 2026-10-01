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
    class: String,
    flags: u32,
    bounds: (Vec3, Vec3),
    slide: Vec3,
    duration: f32,
    wait: f32,
    speed: f32,
    limit: f32,
}
pub(super) fn owns(e: &super::super::Entity) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("script_object" | "func_door" | "func_rotatingdoor" | "func_sinkobject")
    )
}
pub(super) fn fake(n: &str) -> bool {
    n.strip_prefix("fake")
        .is_some_and(|s| s.parse::<usize>().is_ok_and(|v| (1..=8).contains(&v)))
}
fn num(e: &super::super::Entity, n: &str, d: f32) -> f32 {
    e.get(n).and_then(|s| s.parse().ok()).unwrap_or(d)
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Object>> {
    map.entities
        .iter()
        .enumerate()
        .filter(|(_, e)| owns(e))
        .map(|(id, e)| {
            let model = e["model"].trim_start_matches('*').parse::<usize>()?;
            let mut base = at(e);
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
            let class = e["classname"].clone();
            let speed = num(
                e,
                "speed",
                if class == "func_sinkobject" { 8. } else { 100. },
            );
            // Cache local brush geometry once. Placing it only transforms the
            // bounds until a collision query actually reaches the machine.
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
                bounds: (base.translation + m.min, base.translation + m.max),
                flags: num(e, "spawnflags", 0.) as u32,
                duration: num(
                    e,
                    "time",
                    if class == "func_door" {
                        slide.length() / speed.max(1.)
                    } else {
                        1.5
                    },
                )
                .max(0.01),
                wait: num(e, "wait", 3.),
                speed,
                limit: num(
                    e,
                    "limit",
                    if (961..=964).contains(&id) { 16. } else { 192. },
                ),
                class,
                slide,
            })
        })
        .collect()
}
fn approach(v: f32, t: f32, d: f32) -> f32 {
    if v < t {
        (v + d).min(t)
    } else {
        (v - d).max(t)
    }
}
impl Clockwork {
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let s = &self.saved;
        let ct = s.elapsed(s.clock, 5.);
        let scene = s.scene.as_ref();
        let stop = scene
            .filter(|s| s.kind == Kind::Stop)
            .map(|s| ((s.time - 1.5) / 6.).clamp(0., 1.))
            .unwrap_or(if s.stopped { 1. } else { 0. });
        // Capture the turning phase at the stop cue; continually resampling it
        // during the six-second settle would reverse and jitter the hands.
        let stop_time = if s.stopped {
            s.levers[4].unwrap_or(s.age) + 1.5
        } else {
            scene
                .filter(|sc| sc.kind == Kind::Stop && sc.time >= 1.5)
                .map_or(s.age, |sc| s.age - sc.time + 1.5)
        };
        for (k, o) in self.objects.iter_mut().enumerate() {
            let old = o.pose;
            o.pose = o.base;
            o.draw = true;
            o.solid = true;
            let t = s.age;
            if o.class == "func_rotatingdoor" {
                let sign = if o.flags & 2 != 0 { -1. } else { 1. };
                // Paired leaves use their authored hinge sides, including unnamed partners.
                let local = (o.bounds.0 + o.bounds.1) * 0.5 - o.base.translation;
                let hinge = if local.x.abs() > local.y.abs() {
                    local.x
                } else {
                    -local.y
                };
                let side = if hinge < 0. { -1. } else { 1. };
                o.pose.rotation =
                    Quat::from_rotation_z(s.doors[k] * sign * side * std::f32::consts::FRAC_PI_2);
            } else if o.class == "func_door" {
                o.pose.translation += o.slide * s.doors[k];
            }
            if o.class == "func_sinkobject" {
                o.pose.translation.z -= s.sinks[k];
            }
            if fake(&o.name) {
                o.draw = false;
                o.solid = false;
                let name = o.name.replacen("fake", "sink", 1);
                if let Some((j, _)) = map
                    .entities
                    .iter()
                    .enumerate()
                    .find(|(_, e)| e.get("targetname") == Some(&name))
                {
                    // The index order of owned brushes is stable across loads.
                    let index = map.entities.iter().take(j).filter(|e| owns(e)).count();
                    o.pose.translation.z -= s.sinks[index];
                }
            }
            match o.name.as_str() {
                "astralmonkey" => {
                    o.pose.translation.z += 440. * (1. - s.elapsed(s.both(), 1.));
                    let first = s.levers[0]
                        .into_iter()
                        .chain(s.levers[1])
                        .min_by(|a, b| a.total_cmp(b));
                    o.pose.rotation = Quat::from_rotation_z(
                        first
                            .map_or(0., |a| (t - a - 2.).max(0.) * 50.)
                            .to_radians(),
                    );
                }
                "no_return" => {
                    o.pose.translation.z +=
                        300. * s.elapsed(s.both(), 1.) * (1. - s.elapsed(s.no_return, 0.2));
                }
                "no_more_cheating" => {
                    o.pose.rotation =
                        Quat::from_rotation_y((120. * (1. - s.elapsed(s.extendo, 4.))).to_radians())
                }
                "extendo" => o.pose.translation.x += 332. * (1. - s.elapsed(s.extendo, 4.)),
                "Lift1" => o.pose.translation.z -= s.lift,
                "floater1" => {
                    if let Some(t) = s.floater {
                        o.pose.translation = self.floater.sample(s.age - t, false).translation;
                    }
                }
                "ClockDoor_Right" => {
                    o.pose.translation.y += 100.
                        * s.elapsed(s.levers[2], 1.)
                        * (1. - s.elapsed(s.clockroom_closed, 0.1))
                }
                "ClockDoor_Left" => {
                    o.pose.translation.y -= 100.
                        * s.elapsed(s.levers[2], 1.)
                        * (1. - s.elapsed(s.clockroom_closed, 0.1))
                }
                "donna_hunt" | "post_gryph_door" => {
                    o.draw = o.name != "donna_hunt" && s.gryphon.is_none();
                    o.solid = s.gryphon.is_none();
                }
                "rage_gear" => o.pose.translation.z -= 400. * s.elapsed(s.gryphon, 8.),
                "float_clock" | "clockdoor" | "monkey3" => {
                    o.pose.translation += vec3(0., 70., -360.) * ct;
                    if o.name == "clockdoor" {
                        o.pose.rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2 * ct);
                    } else if o.name == "monkey3" {
                        o.pose.rotation = Quat::from_rotation_y(
                            (15. * (t * std::f32::consts::PI).sin()).to_radians(),
                        );
                    }
                }
                "spiral_o_doom" => {
                    o.pose.rotation = Quat::from_rotation_z(
                        ((-60. * (t / 4.).floor())
                            + if t.rem_euclid(4.) < 3. {
                                -30. * t.rem_euclid(4.)
                            } else {
                                -90. + 30. * (t.rem_euclid(4.) - 3.)
                            })
                        .to_radians(),
                    )
                }
                "cam_gear1" => o.pose.rotation = Quat::from_rotation_y((t * 90.).to_radians()),
                "plunger" => {
                    let a = t.rem_euclid(4.);
                    o.pose.translation.z += if a < 3. { 40. * a } else { 120. * (4. - a) };
                }
                "brk_pend" => {
                    o.pose.rotation = Quat::from_rotation_x(
                        (15. * (t * std::f32::consts::TAU).sin()).to_radians(),
                    )
                }
                "gear1" | "gear2" => {
                    o.pose.rotation = Quat::from_rotation_x(
                        (if o.name == "gear1" { 30. } else { -30. } * stop_time).to_radians(),
                    )
                }
                "clockmin" | "clockhr" | "monkeyass666" | "monkeyass667" => {
                    let (speed, end) = match o.name.as_str() {
                        "clockmin" => (60., 0.),
                        "clockhr" => (-10., -90.),
                        "monkeyass666" => (30., 0.),
                        _ => (-70., 0.),
                    };
                    let angle = (speed * stop_time).rem_euclid(360.);
                    o.pose.rotation =
                        Quat::from_rotation_y((angle + (end - angle) * stop).to_radians());
                }
                "clock_minute_hand" | "clock_hour_hand" => {
                    let (speed, end) = if o.name == "clock_minute_hand" {
                        (-32., 0.)
                    } else {
                        (17., 90.)
                    };
                    let start = s.gryphon.or_else(|| {
                        scene
                            .filter(|s| s.kind == Kind::Gryphon)
                            .map(|v| t - v.time)
                    });
                    let angle = start.map_or(0., |a| ((stop_time - a) * speed).rem_euclid(360.));
                    o.pose.rotation =
                        Quat::from_rotation_z((angle + (end - angle) * stop).to_radians());
                }
                "loco_hatter_key" => {
                    let pt = scene
                        .filter(|s| s.kind == Kind::Port)
                        .map_or(0., |s| s.time);
                    let spin = (if s.port.is_some() {
                        540.
                    } else {
                        (pt - 11.25).clamp(0., 0.5) * 1080.
                    })
                    .to_radians();
                    o.pose.rotation = Quat::from_rotation_z(std::f32::consts::PI + spin);
                    o.pose.translation.z -= 130.
                        * if s.port.is_some() {
                            1.
                        } else {
                            ((pt - 12.65) / 1.).clamp(0., 1.)
                        };
                }
                "dunkpanel" => {
                    if let Some(a) = s.hare {
                        o.pose.translation.z -= dunk(t - a);
                    }
                }
                "dormouse_beam_wand" => {
                    if let Some(a) = s.hare {
                        o.pose.rotation = Quat::from_rotation_z(
                            (60. * (((t - a).rem_euclid(8.) / 0.5).min(1.))).to_radians(),
                        );
                    }
                }
                n if n.contains("sky") || n.ends_with("_light") || n == "clock_light" => {
                    o.draw = false;
                    o.solid = false;
                }
                _ => {}
            }
            if old.translation != o.pose.translation || old.rotation != o.pose.rotation {
                o.collider = o.shape.at(o.pose.translation, o.pose.rotation);
            }
        }
        Ok(())
    }
    fn door_target(&self, o: &Object) -> Option<bool> {
        match o.id {
            96 | 97 | 455 | 456 | 457 => Some(self.saved.port.is_some()),
            116 | 117 => Some(
                self.saved.stopped
                    || self
                        .saved
                        .scene
                        .as_ref()
                        .is_some_and(|s| s.kind == Kind::Stop && s.time >= 28.),
            ),
            _ if o.name == "port_bent" => Some(
                self.saved.port.is_some()
                    || self
                        .saved
                        .scene
                        .as_ref()
                        .is_some_and(|s| s.kind == Kind::Port && s.time >= 1.),
            ),
            _ => None,
        }
    }
    pub(super) fn door_pick(&self, w: &World, eye: Vec3, aim: Vec3) -> Option<usize> {
        self.objects.iter().enumerate().find_map(|(k, o)| {
            if !o.class.contains("door")
                || self.door_target(o).is_some()
                || o.flags & (8 | 128) != 0
            {
                return None;
            }
            let target = (o.bounds.0 + o.bounds.1) * 0.5;
            let d = target - eye;
            let tr = w.sweep(eye, target, Vec3::splat(0.5));
            let own = o.collider.trace(eye, target, Vec3::splat(0.5));
            (d.length() < 150.
                && d.normalize_or_zero().dot(aim.normalize_or_zero()) > 0.3
                && !tr.start_solid
                && tr.fraction + 0.02 >= own.fraction)
                .then_some(k)
        })
    }
    pub(super) fn use_door(&mut self, w: &World, p: &Player, aim: Vec3) {
        let selected = self.door_pick(w, p.eye(), aim);
        if let Some(k) = selected {
            let center = (self.objects[k].bounds.0 + self.objects[k].bounds.1) * 0.5;
            for (j, o) in self.objects.iter().enumerate() {
                if o.class.contains("door")
                    && self.door_target(o).is_none()
                    && o.flags & 128 == 0
                    && ((o.bounds.0 + o.bounds.1) * 0.5).distance(center) < 150.
                {
                    self.saved.holds[j] = o.wait.max(3.);
                }
            }
        }
    }
    pub(super) fn advance_motion(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let old = self.saved.clone();
        let support = self
            .objects
            .iter()
            .enumerate()
            .find(|(_, o)| {
                o.solid && p.velocity.z <= 1. && {
                    let tr = o.collider.trace(
                        p.feet + PLAYER_CENTER,
                        p.feet + PLAYER_CENTER - Vec3::Z * 3.,
                        PLAYER_HALF,
                    );
                    !tr.start_solid && tr.fraction < 1. && tr.normal.z > 0.65
                }
            })
            .map(|(k, o)| (k, o.pose));
        self.saved.age = (self.saved.age + dt).min(1e7);
        self.saved.lift = approach(
            self.saved.lift,
            if self.saved.lift_up { 0. } else { 170. },
            dt * 850.,
        );
        for (k, o) in self.objects.iter().enumerate() {
            if o.class == "func_sinkobject" {
                let occupied = support.is_some_and(|(j, _)| j == k);
                let speed = if occupied { o.speed } else { o.speed.min(100.) };
                self.saved.sinks[k] = approach(
                    self.saved.sinks[k],
                    if occupied { o.limit } else { 0. },
                    speed * dt,
                );
            }
            if !o.class.contains("door") {
                continue;
            }
            let scripted = self.door_target(o);
            let center = p.feet + PLAYER_CENTER;
            if scripted.is_none()
                && o.flags & 64 != 0
                && o.flags & (8 | 128) == 0
                && center
                    .cmpge(o.bounds.0 - vec3(60., 60., 8.) - PLAYER_HALF)
                    .all()
                && center
                    .cmple(o.bounds.1 + vec3(60., 60., 8.) + PLAYER_HALF)
                    .all()
            {
                self.saved.holds[k] = o.wait.max(3.);
            }
            let open = scripted.unwrap_or(self.saved.holds[k] > 0.);
            self.saved.holds[k] = (self.saved.holds[k] - dt).max(0.);
            self.saved.doors[k] = approach(
                self.saved.doors[k],
                if open { 1. } else { 0. },
                dt / o.duration,
            );
        }
        self.rebuild(map)?;
        let carry = support.map_or(Vec3::ZERO, |(k, old)| {
            let new = self.objects[k].pose;
            new.point(old.rotation.inverse() * (p.feet - old.translation)) - p.feet
        });
        w.set_dynamic(fixed.to_vec());
        let tr = w.sweep(
            p.feet + PLAYER_CENTER,
            p.feet + carry + PLAYER_CENTER,
            PLAYER_HALF,
        );
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        if !self.scripted() && (tr.start_solid || tr.fraction < 1. || !w.body_clear(p.feet + carry))
        {
            // A cushion can reach fixed geometry before its authored sink limit.
            // Stop that rider's cushion without stopping the independent clock,
            // doors and scene clocks while Alice remains on the chair.
            if let Some((k, _)) = support.filter(|(k, _)| self.objects[*k].class == "func_sinkobject") {
                self.saved.sinks[k] = old.sinks[k];
                self.rebuild(map)?;
                w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
                if w.body_clear(p.feet) {
                    return Ok(());
                }
            }
            self.saved = old;
            self.rebuild(map)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        } else if !self.scripted() {
            p.feet += carry;
        }
        Ok(())
    }
}
pub(super) fn dunk(t: f32) -> f32 {
    let t = (t - 3.).max(0.).rem_euclid(12.);
    if t < 0.5 {
        t * 600.
    } else if t < 6. {
        300.
    } else {
        300. * (1. - ((t - 6.) / 3.6).min(1.))
    }
}

use super::*;
#[path = "waves.rs"]
mod waves;
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Lift {
    pub phase: u8,
    pub time: f32,
    pub height: f32,
}
impl Lift {
    pub fn new() -> Self {
        Self {
            phase: 0,
            time: 0.,
            height: 0.,
        }
    }
    pub fn up(&mut self) {
        if self.phase == 1 {
            self.phase = 2;
            self.time = 0.;
        }
    }
    pub fn down(&mut self) {
        if self.phase == 4 {
            self.phase = 0;
            self.time = 0.;
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.phase < 5
                && self.time.is_finite()
                && (0. ..=5.).contains(&self.time)
                && self.height.is_finite()
                && (-192. ..=0.).contains(&self.height),
            "Invalid chess elevator"
        );
        Ok(())
    }
    fn step(&mut self, dt: f32) {
        self.time += dt;
        match self.phase {
            0 => {
                self.height = -travel(self.time);
                if self.time >= 4.5 {
                    self.phase = 1;
                    self.time = 0.;
                }
            }
            1 => self.time = 0.,
            2 => {
                if self.time >= 1. {
                    self.phase = 3;
                    self.time = 0.;
                }
            }
            3 => {
                self.height = -192. + travel(self.time);
                if self.time >= 4.5 {
                    self.phase = 4;
                    self.time = 0.;
                }
            }
            _ => {
                if self.time >= 2. {
                    self.phase = 0;
                    self.time = 0.;
                }
            }
        }
    }
}
fn travel(t: f32) -> f32 {
    let mut elapsed = 0.;
    let mut pos = 0.;
    for (duration, dist) in [
        (0.25, 4.),
        (0.25, 8.),
        (0.25, 12.),
        (0.25, 16.),
        (1.5, 112.),
        (0.25, 16.),
        (0.25, 12.),
        (0.25, 8.),
        (0.25, 4.),
    ] {
        pos += dist * ((t - elapsed) / duration).clamp(0., 1.);
        elapsed += duration;
    }
    pos
}
pub(super) struct Object {
    pub id: usize,
    pub model: usize,
    pub name: String,
    pub base: Transform,
    pub pose: Transform,
    pub draw: bool,
    pub solid: bool,
    pub liquid: bool,
    pub hazard: bool,
    pub collider: Collider,
    pub volume: Collider,
}
pub(super) fn owns(e: &super::super::Entity) -> bool {
    e.get("classname")
        .is_some_and(|v| matches!(v.as_str(), "script_object" | "func_door"))
        || e.get("targetname")
            .is_some_and(|n| n == "float_secret_smash")
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Object>> {
    map.entities
        .iter()
        .enumerate()
        .filter(|(_, e)| owns(e))
        .map(|(id, e)| {
            let model = e["model"].trim_start_matches('*').parse()?;
            let base = Transform {
                rotation: Quat::IDENTITY,
                ..data::at(e)
            };
            Ok(Object {
                id,
                model,
                name: e.get("targetname").cloned().unwrap_or_default(),
                base,
                pose: base,
                draw: false,
                solid: false,
                liquid: false,
                hazard: false,
                collider: Collider::model(map, model, base.translation, base.rotation, true)?,
                volume: Collider::model(map, model, base.translation, base.rotation, false)?,
            })
        })
        .collect()
}
impl Realm {
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let gate = self.quad_open();
        for o in &mut self.objects {
            let old = o.pose;
            let n = o.name.as_str();
            o.pose = o.base;
            o.draw = true;
            o.solid = true;
            o.liquid = false;
            o.hazard = false;
            match n {
                "skycamera" | "bishop_actor_clips" | "knight_actor_clips" => {
                    o.draw = false;
                    o.solid = false;
                }
                "knight_mclip" => {
                    o.draw = false;
                    o.solid = self
                        .saved
                        .board
                        .as_ref()
                        .is_some_and(|b| b.piece == Piece::Knight);
                }
                "water" => {
                    o.liquid = true;
                    o.solid = false;
                    o.pose.translation.z += self
                        .saved
                        .water
                        .map_or(0., |t| 96. * ((self.saved.age - t - 3.) / 5.).clamp(0., 1.));
                }
                "elevator_water" => {
                    o.liquid = true;
                    o.solid = false;
                }
                "water_exit_block" => {
                    o.draw = false;
                    o.solid = !self.saved.water.is_some_and(|t| self.saved.age - t >= 8.);
                }
                "waterwheel" => {
                    o.pose.translation = self.data.point("waterwheel_start").translation;
                    let t = self.saved.water.map_or(0., |t| self.saved.age - t);
                    let mut angle = 0.;
                    for i in 0..10 {
                        angle -= 6. * (i + 1) as f32 * (t - i as f32 * 0.3).clamp(0., 0.3);
                    }
                    angle -= 60. * (t - 3.).max(0.);
                    o.pose.rotation = Quat::from_rotation_x(angle.to_radians());
                }
                "elevator" => o.pose.translation.z += self.saved.elevator.height,
                "elevator_weight_mounts" => o.pose.translation.z -= self.saved.elevator.height,
                "quad_gate_top" | "quad_gate_bottom" => {
                    o.pose.translation = self
                        .data
                        .point(&format!("{n}_way1"))
                        .translation
                        .lerp(self.data.point(&format!("{n}_way2")).translation, gate)
                }
                "knight_gate" => {
                    let m = &map.models[o.model];
                    let duration = ((m.max.y - m.min.y - 32.).max(0.) / 100.).max(0.01);
                    let open = if self.saved.knight_gate {
                        self.saved
                            .knight_opened
                            .map_or(1., |t| ((self.saved.age - t) / duration).clamp(0., 1.))
                    } else {
                        0.
                    };
                    let sign = if o.id == 966 { 1. } else { -1. };
                    let m = &map.models[o.model];
                    o.pose.translation.y += sign * (m.max.y - m.min.y - 32.).max(0.) * open;
                }
                "hand_long" | "hand_short" => {
                    let speed = if n == "hand_long" { 90. } else { 7.5 };
                    // The script's X angle is pitch: a turn about world Y.
                    // The hands lie in X/Z, with their BSP origins at the axle.
                    o.pose.rotation = Quat::from_rotation_y((self.saved.age * speed).to_radians());
                    o.solid = false;
                }
                "float_secret_wall" | "float_secret_smash" => {
                    o.draw = !self.saved.secret;
                    o.solid = o.draw;
                }
                _ if n.starts_with("bsspike") || n.starts_with("knspike") => {
                    o.pose.translation = self.data.point(&format!("{n}_start")).translation;
                    // Stable per-identity phase in the source's initial 0..0.5 second window.
                    let t = (self.saved.age - (o.id % 17) as f32 / 34.)
                        .max(0.)
                        .rem_euclid(1.);
                    let depth = if t < 0.25 {
                        t * 384.
                    } else if t < 0.5 {
                        (0.5 - t) * 384.
                    } else {
                        0.
                    };
                    o.pose.translation.z -= depth;
                    o.hazard = true;
                }
                _ if n.starts_with("op1_spike") => {
                    let i = n.trim_start_matches("op1_spike").parse::<u8>().unwrap_or(0);
                    o.draw = false;
                    o.solid = false;
                    if let Some(start) = self.saved.spikes.filter(|_| {
                        self.saved
                            .spikes_off
                            .is_none_or(|t| self.saved.age - t < 1.)
                    }) {
                        let t = (self.saved.age - start).rem_euclid(waves::PERIOD);
                        if let Some((_, at, pos)) = waves::WAVE
                            .iter()
                            .find(|(k, at, _)| *k == i && (0. ..0.7).contains(&(t - at)))
                        {
                            let u = t - at;
                            let rise = if u < 0.1 {
                                u * 1600.
                            } else if u < 0.6 {
                                160.
                            } else {
                                (0.7 - u) * 1600.
                            };
                            o.pose.translation = Vec3::from_array(*pos) + Vec3::Z * rise;
                            o.draw = true;
                            o.solid = true;
                            o.hazard = true;
                        }
                    }
                }
                _ if n.starts_with("bsarrow_")
                    || n.starts_with("bsclip_")
                    || n.starts_with("knarrow")
                    || n.starts_with("knclip_") =>
                {
                    let piece = if n.starts_with("bs") {
                        Piece::Bishop
                    } else {
                        Piece::Knight
                    };
                    o.draw = false;
                    o.solid = false;
                    if let Some(b) = self.saved.board.as_ref().filter(|b| b.piece == piece) {
                        let node = &piece.nodes()[b.node];
                        o.pose.translation = self
                            .data
                            .point(&format!("{}{}", piece.name(), node.square))
                            .translation;
                        let dir = n.rsplit('_').next().unwrap();
                        let d = piece.dirs().iter().position(|s| *s == dir).unwrap();
                        let armed = b.moving.is_none() && node.next[d].is_some();
                        if n.contains("clip") {
                            o.solid = b.moving.is_none() && !armed;
                        } else {
                            o.draw = armed
                                && (!n.starts_with("knarrow")
                                    || n.starts_with(if node.dark & (1 << d) != 0 {
                                        "knarrowdk"
                                    } else {
                                        "knarrowlt"
                                    }));
                        }
                    }
                }
                _ if n.starts_with("bishop") && n.ends_with("_clip")
                    || n.starts_with("square") && n.ends_with("_clip") =>
                {
                    o.draw = false;
                    o.solid = false;
                }
                "t98" | "t99" | "t100" | "t101" => {
                    o.draw = false;
                    o.solid = self
                        .saved
                        .board
                        .as_ref()
                        .is_some_and(|b| b.piece == Piece::Bishop);
                }
                _ => {}
            }
            if old.translation != o.pose.translation || old.rotation != o.pose.rotation {
                o.collider =
                    Collider::model(map, o.model, o.pose.translation, o.pose.rotation, true)?;
                o.volume =
                    Collider::model(map, o.model, o.pose.translation, o.pose.rotation, false)?;
            }
        }
        Ok(())
    }
    pub(super) fn move_world(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let before = self.saved.elevator.clone();
        let before_feet = p.feet;
        let elevator = self.objects.iter().find(|o| o.name == "elevator").unwrap();
        let support = elevator.collider.trace(
            p.feet + PLAYER_CENTER,
            p.feet + PLAYER_CENTER - Vec3::Z * 3.,
            vec3(1., 1., PLAYER_HALF.z),
        );
        let rider = p.velocity.z <= 1.
            && !support.start_solid
            && support.fraction < 1.
            && support.normal.z > 0.65;
        self.saved.elevator.step(dt);
        self.rebuild(map)?;
        if rider {
            let feet = p.feet + Vec3::Z * (self.saved.elevator.height - before.height);
            w.set_dynamic(
                fixed
                    .iter()
                    .cloned()
                    .chain(
                        self.objects
                            .iter()
                            .filter(|o| o.solid && o.name != "elevator")
                            .map(|o| o.collider.clone()),
                    )
                    .collect(),
            );
            let trace = w.body_trace(p.feet, feet);
            if !trace.start_solid && trace.fraction >= 1. && w.body_clear(feet) {
                p.feet = feet;
            } else {
                self.saved.elevator = before.clone();
                self.rebuild(map)?;
            }
        }
        // A non-rider can be underneath the descending lift or entering its edge.
        // Moving the brush first embeds that player before ordinary movement gets
        // a chance to sweep. Keep the last safe lift pose until the space clears.
        let elevator = self.objects.iter().find(|o| o.name == "elevator").unwrap();
        if elevator.collider.trace(
            p.feet + PLAYER_CENTER, p.feet + PLAYER_CENTER, PLAYER_HALF,
        ).start_solid {
            self.saved.elevator = before;
            p.feet = before_feet;
            self.rebuild(map)?;
        }
        if !self.scripted() {
            for o in &self.objects {
                if o.hazard
                    && o.volume
                        .trace(p.feet + PLAYER_CENTER, p.feet + PLAYER_CENTER, PLAYER_HALF)
                        .start_solid
                {
                    self.saved.damage = 1000.;
                }
            }
        }
        self.move_cast(dt);
        Ok(())
    }
}

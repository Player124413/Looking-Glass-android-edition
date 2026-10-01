use super::*;
pub(super) fn owned(e: &super::super::Entity) -> bool {
    e.get("model").is_some_and(|m| m.starts_with('*'))
        && e.get("classname").is_some_and(|c| {
            matches!(
                c.as_str(),
                "script_object" | "func_door" | "func_smashablewall" | "func_sinkobject"
            )
        })
}
struct Object {
    entity: usize,
    model: usize,
    name: String,
    base: Vec3,
    pose: Transform,
    collider: Collider,
    solid: bool,
    draw: bool,
    delta: Vec3,
}
pub(super) struct Motion {
    objects: Vec<Object>,
    hall_duration: f32,
}
pub(super) fn mirror_angle(s: &Saved) -> f32 {
    s.selected.map_or(-45., |i| {
        s.mirror_from + (ANGLES[i] - s.mirror_from) * (s.mirror_time / 5.)
    })
}
impl Motion {
    pub fn load(map: &Bsp) -> Result<Self> {
        Ok(Self {
            hall_duration: map.entities[67]
                .get("time")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.)
                .max(0.01),
            objects: map
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| owned(e))
                .map(|(entity, e)| {
                    let model: usize = e["model"].trim_start_matches('*').parse()?;
                    let base =
                        interaction::vector(&e["origin"]).context("Invalid Keep brush origin")?;
                    let m = &map.models[model];
                    let yaw = e
                        .get("angle")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.)
                        .to_radians();
                    let dir = vec3(yaw.cos(), yaw.sin(), 0.);
                    let lip = e
                        .get("lip")
                        .and_then(|s| s.parse::<f32>().ok())
                        .unwrap_or(0.);
                    let delta = dir * ((m.max - m.min).dot(dir.abs()) - lip).max(0.);
                    Ok(Object {
                        entity,
                        model,
                        name: e.get("targetname").cloned().unwrap_or_default(),
                        base,
                        pose: Transform {
                            translation: base,
                            rotation: Quat::IDENTITY,
                        },
                        collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                        solid: false,
                        draw: false,
                        delta,
                    })
                })
                .collect::<Result<_>>()?,
        })
    }
    pub fn rebuild(&mut self, map: &Bsp, s: &Saved) -> Result<()> {
        for o in &mut self.objects {
            let mut pose = Transform {
                translation: o.base,
                rotation: Quat::IDENTITY,
            };
            let mut draw = true;
            let mut solid = true;
            match o.name.as_str() {
                "keep_monster_clip" | "black_sky" | "grounds_sky" | "default_sky" => {
                    draw = false;
                    solid = false;
                }
                "keep_lift" => pose.translation.z += 192. * s.elapsed / 5.,
                "mirror" => pose.rotation = Quat::from_rotation_z(mirror_angle(s).to_radians()),
                "door_club1" => pose.translation += o.delta * s.doors[0],
                "door_diamond1" => pose.translation += o.delta * s.doors[1],
                "door_spade1" => pose.translation += o.delta * s.doors[2],
                "heart_door1" => pose.translation += o.delta * s.doors[3],
                "queen_door1" | "queen_door2" => pose.translation += o.delta * s.doors[4],
                n if n.starts_with("painting_") => {
                    solid = false;
                    let i = match n {
                        "painting_tweedle" | "painting_club" => 0,
                        "painting_jabber" | "painting_diamond" => 1,
                        _ => 2,
                    };
                    draw = !s.won[i]
                        && !matches!(n, "painting_club" | "painting_diamond" | "painting_spade");
                }
                n if n.ends_with("_shaft") => {
                    solid = false;
                    let i = SUITS.iter().position(|v| n.starts_with(v)).unwrap();
                    draw = !s.won[i] && s.room != Some(i);
                }
                n if n.starts_with("hatter_")
                    || n.starts_with("jabber_")
                    || n.starts_with("tweedle_") =>
                {
                    let i = SUITS.iter().position(|v| n.ends_with(v)).unwrap();
                    draw = s.room == Some(i) && s.room_time == 1.;
                    solid = draw;
                }
                _ => match o.entity {
                    67 | 68 => pose.translation += o.delta * s.hall_open,
                    70 => {
                        pose.translation.z -= s.sink;
                        draw = false;
                    }
                    _ => {}
                },
            }
            if pose.translation != o.pose.translation || pose.rotation != o.pose.rotation {
                o.collider = Collider::model(map, o.model, pose.translation, pose.rotation, true)?;
            }
            o.pose = pose;
            o.draw = draw;
            o.solid = solid;
        }
        Ok(())
    }
    pub fn transforms(&self, _: &Saved) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| o.draw)
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    pub fn colliders(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.solid)
            .map(|o| o.collider.clone())
            .collect()
    }
    pub fn reflection(&self, s: &Saved) -> (Vec<(usize, Vec3, Quat)>, Vec<usize>) {
        let hide = self
            .objects
            .iter()
            .filter(|o| o.name.starts_with("painting_"))
            .map(|o| o.model)
            .collect();
        let extra = self
            .objects
            .iter()
            .filter(|o| {
                s.selected.is_some_and(|i| {
                    s.mirror_time == 5. && !s.won[i] && o.name == format!("painting_{}", SUITS[i])
                })
            })
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect();
        (extra, hide)
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
        s: &mut Saved,
    ) -> Result<()> {
        let before = s.clone();
        let mut carry = Vec3::ZERO;
        let support = self
            .objects
            .iter()
            .find(|o| {
                matches!(o.entity, 5 | 70) && {
                    let t = o.collider.trace(
                        p.feet + PLAYER_CENTER,
                        p.feet + PLAYER_CENTER - Vec3::Z * 3.,
                        vec3(1., 1., PLAYER_HALF.z),
                    );
                    p.velocity.z <= 1. && !t.start_solid && t.fraction < 1. && t.normal.z > 0.65
                }
            })
            .map(|o| o.entity);
        s.elapsed = (s.elapsed + dt).min(5.);
        if support == Some(70) {
            s.sink = (s.sink + dt * 8.).min(44.);
        } else {
            s.sink = (s.sink - dt * 8.).max(0.);
        }
        if support == Some(5) {
            carry.z = 192. * (s.elapsed - before.elapsed) / 5.;
        } else if support == Some(70) {
            carry.z = before.sink - s.sink;
        }
        if let Some(scene) = &s.scene {
            if scene.kind == Kind::Mirror && scene.time >= 1.5 {
                s.mirror_time = (s.mirror_time + dt).min(5.);
            }
        }
        for i in 0..3 {
            s.holds[i] = (s.holds[i] - dt).max(0.);
            let target = if s.holds[i] > 0. { 1. } else { 0. };
            s.doors[i] = approach(s.doors[i], target, dt / 0.2);
        }
        s.hall = (s.hall - dt).max(0.);
        s.hall_open = approach(
            s.hall_open,
            if s.hall > 0. { 1. } else { 0. },
            dt / self.hall_duration,
        );
        s.doors[3] = approach(s.doors[3], if s.heart_open { 1. } else { 0. }, dt / 0.1);
        s.doors[4] = approach(s.doors[4], if s.queen_open { 1. } else { 0. }, dt / 6.);
        self.rebuild(map, s)?;
        w.set_dynamic(fixed.to_vec());
        let feet = p.feet + carry;
        let sweep = w.sweep(p.feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF);
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        if sweep.start_solid || sweep.fraction < 1. || !w.body_clear(feet) {
            // Only reverse physical progress; scene/puzzle decisions remain committed.
            s.elapsed = before.elapsed;
            s.sink = before.sink;
            s.doors = before.doors;
            s.hall = before.hall;
            s.hall_open = before.hall_open;
            s.mirror_time = before.mirror_time;
            self.rebuild(map, s)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        } else {
            p.feet = feet;
        }
        Ok(())
    }
}
fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}

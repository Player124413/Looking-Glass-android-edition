use super::*;
use crate::collision::model_shape::Template;

// A side contact with a turning tooth can include a downward component. Let
// the floor block that component before deciding whether Alice is trapped.
// Both the swept world path and the moved gear must be clear at the result.
fn slide_side_push(w: &World, mover: &Collider, from: Vec3, to: Vec3) -> Option<Vec3> {
    let mut next = to;
    for _ in 0..32 {
        let hit = w.body_trace(from, next);
        if hit.start_solid {
            return None;
        }
        if hit.fraction < 1. {
            let rest = (next - from) * (1. - hit.fraction);
            next = from.lerp(next, hit.fraction)
                + rest - hit.normal * rest.dot(hit.normal).min(0.);
        }
        if next.distance(from) > 20. {
            return None;
        }
        let path = w.body_trace(from, next);
        if !path.start_solid && path.fraction == 1. && w.body_clear(next)
            && !mover.touches(next + PLAYER_CENTER, next + PLAYER_CENTER, PLAYER_HALF) {
            return Some(next);
        }
        // An upright box may still overlap a sloping tooth after the floor
        // clips its downward carry. Alternate the two contact constraints;
        // accept only a fully swept, clear result within the existing limit.
        let separated = mover.separate_body(next, 20.)?;
        if separated.distance_squared(next) < 1e-10 {
            return None;
        }
        next = separated;
    }
    None
}

#[cfg(test)]
mod side_push_tests {
    use super::*;

    #[test]
    fn a_side_push_can_slide_on_the_floor_but_cannot_escape_a_real_pinch() {
        let floor = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        let feet = vec3(0., 0., 0.03125);
        let tooth = Collider::fixture(vec3(-40., -40., 0.), vec3(-14., 40., 70.));
        assert!(tooth.touches(feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF));
        let pushed = feet + vec3(3., 0., -0.5);
        assert!(floor.body_trace(feet, pushed).fraction < 1.);
        let slid = slide_side_push(&floor, &tooth, feet, pushed).unwrap();
        assert!((slid - vec3(3., 0., feet.z)).length() < 0.04);
        let pin = Collider::fixture(vec3(-40., -40., 0.), vec3(40., 40., 70.));
        assert!(slide_side_push(&floor, &pin, feet, pushed).is_none());
        let wall = World::fixture(&[
            (vec3(-100., -100., -20.), vec3(100., 100., 0.)),
            (vec3(16., -100., 0.), vec3(100., 100., 100.)),
        ]);
        assert!(slide_side_push(&wall, &tooth, feet, pushed).is_none());
    }
}

pub(super) struct Object {
    pub id: usize,
    pub name: String,
    pub model: usize,
    pub base: Vec3,
    pub pose: Transform,
    pub collider: Collider,
    pub shape: Template,
    pub class: String,
    pub flags: u32,
    pub wait: f32,
    pub duration: f32,
    pub slide: Vec3,
    pub bounds: (Vec3, Vec3),
    pub sound: bool,
}
pub(super) fn owned(e: &super::super::Entity) -> bool {
    e.get("model").is_some_and(|m| m.starts_with('*'))
        && matches!(
            e.get("classname").map(String::as_str),
            Some("script_object" | "func_door" | "func_sinkobject")
        )
        && !e
            .get("targetname")
            .is_some_and(|n| n.starts_with("sky_camera"))
}
fn num(e: &super::super::Entity, n: &str, d: f32) -> f32 {
    e.get(n).and_then(|s| s.parse().ok()).unwrap_or(d)
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Object>> {
    let mut objects: Vec<Object> = map
        .entities
        .iter()
        .enumerate()
        .filter(|(_, e)| owned(e))
        .map(|(id, e)| {
            let model: usize = e["model"].trim_start_matches('*').parse()?;
            let base = e
                .get("origin")
                .and_then(|s| interaction::vector(s))
                .unwrap_or(Vec3::ZERO);
            let m = &map.models[model];
            let angle = num(e, "angle", 0.);
            let dir = if angle == -1. {
                Vec3::Z
            } else if angle == -2. {
                -Vec3::Z
            } else {
                vec3(angle.to_radians().cos(), angle.to_radians().sin(), 0.)
            };
            let distance = ((m.max - m.min).dot(dir.abs()) - num(e, "lip", 8.)).max(0.);
            let shape = Template::model(map, model)?;
            Ok(Object {
                id,
                model,
                base,
                name: e.get("targetname").cloned().unwrap_or_default(),
                class: e["classname"].clone(),
                flags: num(e, "spawnflags", 0.) as u32,
                wait: num(e, "wait", 3.),
                duration: num(e, "time", distance / num(e, "speed", 100.).max(1.)).max(0.01),
                slide: dir * distance,
                bounds: (base + m.min, base + m.max),
                sound: e.contains_key("sound_move"),
                pose: Transform {
                    translation: base,
                    rotation: Quat::IDENTITY,
                },
                collider: shape.at(base, Quat::IDENTITY),
                shape,
            })
        })
        .collect::<Result<_>>()?;
    // Adjacent leaves share one proximity field, as the authored door chain does.
    let initial = objects.iter().map(|o| o.bounds).collect::<Vec<_>>();
    for k in 0..objects.len() {
        if !objects[k].door() || objects[k].flags & 4 != 0 {
            continue;
        }
        let mut group = vec![k];
        loop {
            let count = group.len();
            for j in 0..objects.len() {
                if objects[j].door()
                    && objects[j].flags & 4 == 0
                    && !group.contains(&j)
                    && group.iter().any(|i| {
                        initial[*i].0.cmple(initial[j].1 + Vec3::ONE).all()
                            && initial[*i].1.cmpge(initial[j].0 - Vec3::ONE).all()
                    })
                {
                    group.push(j);
                }
            }
            if count == group.len() {
                break;
            }
        }
        objects[k].bounds = (
            group
                .iter()
                .map(|j| initial[*j].0)
                .reduce(Vec3::min)
                .unwrap(),
            group
                .iter()
                .map(|j| initial[*j].1)
                .reduce(Vec3::max)
                .unwrap(),
        );
    }
    Ok(objects)
}
fn spin(axis: Vec3, rate: f64, time: f64) -> Quat {
    Quat::from_axis_angle(
        axis,
        (rate * time).rem_euclid(360.) as f32 * std::f32::consts::PI / 180.,
    )
}
impl Object {
    pub fn door(&self) -> bool {
        self.class == "func_door"
    }
    pub fn crusher(&self) -> bool {
        matches!(
            self.name.as_str(),
            "ridegear01"
                | "ridegear02"
                | "ridegear03"
                | "wallgear"
                | "gearwithspikeaxle"
                | "turbine01"
                | "turbine02"
                | "pendulum01"
                | "pendulum02"
                | "pendulum03"
                | "pendulum04"
        )
    }
    pub fn at(&self, s: &Mechanism) -> Transform {
        let t = s.time;
        let mut p = self.base;
        let mut r = Quat::IDENTITY;
        if self.door() {
            p += self.slide * s.door;
        }
        if self.class == "func_sinkobject" {
            p.z -= s.depth;
        }
        match self.name.as_str() {
            "lavagear" => r = spin(Vec3::Z, 20., t),
            "gearobstacle01" => r = spin(Vec3::Z, 30., t),
            "ridegear01" | "ridegear02" => r = spin(Vec3::Z, 12., t),
            "ridegear03" => r = spin(Vec3::Z, -12., t),
            "wallgear" | "gearwithspikeaxle" => r = spin(Vec3::X, -12., t),
            "turbine01" | "turbine02" => r = spin(Vec3::Z, 90., t),
            "uppergear01" | "uppergear03" => r = spin(Vec3::X, -30., t),
            "uppergear02" => r = spin(Vec3::X, 30., t),
            "bellow1gear" | "bellow2gear" => r = spin(Vec3::Y, -80., t),
            "bellow1pumpwheel" | "bellow2pumpwheel" => r = spin(Vec3::Y, 45., t),
            "bellow1" | "bellow2" => {
                let c = t.rem_euclid(8.) as f32;
                let angle = if c < 4. {
                    -5. * c
                } else {
                    -20. + 5. * (c - 4.)
                };
                r = Quat::from_rotation_y(angle.to_radians());
            }
            "bellow1arm" | "bellow2arm" => {
                let c = t.rem_euclid(8.) as f32;
                let local = if c < 2. {
                    7.5 * c
                } else if c < 6. {
                    30. - 7.5 * c
                } else {
                    -15. + 7.5 * (c - 6.)
                };
                let parent = spin(Vec3::Y, 45., t);
                let pivot = vec3(4512., -1778., 176.);
                p = pivot + parent * (self.base - pivot);
                r = parent * Quat::from_rotation_y(local.to_radians());
            }
            "pendulum01" => {
                let c = t.rem_euclid(8.) as f32;
                p.z += if c < 4. {
                    68. * c
                } else {
                    272. * (1. - (c - 4.).min(1.))
                };
            }
            "pendulum02" => {
                let c = t.rem_euclid(8.) as f32;
                p.z += if c < 4. {
                    -272. * c.min(1.)
                } else {
                    -272. + 68. * (c - 4.)
                };
            }
            "pendulum03" | "pendulum04" => {
                let sign = if self.name == "pendulum03" { 1. } else { -1. };
                r = Quat::from_rotation_y(
                    sign * 45_f32.to_radians()
                        * ((t.rem_euclid(3.) as f32) * std::f32::consts::TAU / 3.).sin(),
                );
            }
            "ambigear01" => r = spin(Vec3::Y, 800., t),
            "turbinegear" => r = spin(Vec3::Y, 30., t),
            "piston01" | "piston02" => {
                let start = if self.name == "piston02" { 0.5 } else { 0. };
                let c = (t - start).max(0.).rem_euclid(0.2) as f32;
                p.z += if c < 0.1 { 320. * c } else { 320. * (0.2 - c) };
            }
            _ => {}
        }
        Transform {
            translation: p,
            rotation: r,
        }
    }
    fn contact(&self, center: Vec3, half: Vec3) -> bool {
        center
            .cmpge(self.bounds.0 - vec3(60., 60., 8.) - half)
            .all()
            && center
                .cmple(self.bounds.1 + vec3(60., 60., 8.) + half)
                .all()
    }
}
fn support(c: &Collider, p: &Player) -> bool {
    if p.velocity.z > 1. {
        return false;
    }
    let h = c.trace(
        p.feet + PLAYER_CENTER,
        p.feet + PLAYER_CENTER - Vec3::Z * 3.,
        PLAYER_HALF,
    );
    !h.start_solid && h.fraction < 1. && h.normal.z >= 0.65
}
fn rider_landing(o: &Object, c: &Collider, carried: Vec3) -> Option<(Vec3, Vec3)> {
    c.rider_feet(carried).or_else(|| {
        // Alice's box stays upright while a turntable changes its bearing. A
        // corner can overlap the adjacent 16-unit tooth even though her feet
        // stayed on the same face. Permit an ordinary step onto that tooth.
        if !matches!(
            o.name.as_str(),
            "lavagear" | "ridegear01" | "ridegear02" | "ridegear03" | "gearobstacle01"
        ) {
            return None;
        }
        let above = carried + Vec3::Z * 20.;
        let below = carried - Vec3::Z * 2.;
        let tr = c.trace(above + PLAYER_CENTER, below + PLAYER_CENTER, PLAYER_HALF);
        let feet = above.lerp(below, tr.fraction);
        (!tr.start_solid
            && tr.fraction < 1.
            && tr.normal.z >= 0.65
            && feet.z - carried.z <= 18.
            && !c.touches(feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF))
        .then_some((feet, tr.normal))
    })
}
impl Labyrinth {
    fn sync(&self, w: &mut World, fixed: &[Collider]) {
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
    }
    pub(super) fn move_all(
        &mut self,
        dt: f32,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        for k in 0..self.objects.len() {
            let o = &self.objects[k];
            let before = self.saved.machines[k].clone();
            let old = o.pose;
            let original = p.clone();
            let ridden = support(&o.collider, p);
            let mut after = before.clone();
            after.time = (after.time + dt as f64).min(1e8);
            if o.door() {
                let player = o.flags & 8 == 0 && o.contact(p.feet + PLAYER_CENTER, PLAYER_HALF);
                let monster =
                    o.flags & 16 == 0 && self.actors.iter().any(|a| o.contact(a.center, a.half));
                if o.flags & 64 != 0 && (player || monster) {
                    after.hold = o.wait.max(3.);
                    after.latched |= o.wait < 0.;
                }
                let target = if after.hold > 0. || after.latched {
                    1.
                } else {
                    0.
                };
                after.door += (target - after.door).clamp(-dt / o.duration, dt / o.duration);
                after.hold = (after.hold - dt).max(0.);
                if before.door < 1. && after.door == 1. {
                    after.hold = o.wait.max(3.);
                }
                if before.door != after.door && (after.door == 0. || after.door == 1.) {
                    after.stops = (after.stops + 1).min(1_000_000);
                }
            }
            if o.class == "func_sinkobject" {
                if ridden {
                    after.velocity += 0.004 * 60. * 20. * dt;
                } else {
                    after.velocity -= after.depth * 0.0004 * 60. * 20. * dt;
                    after.velocity *= 0.95_f32.powf(20. * dt);
                }
                after.depth += after.velocity * dt;
                if after.depth >= 1000. {
                    after.depth = 1000.;
                    after.velocity = 0.;
                }
                if after.depth <= 0. || !ridden && after.depth < 0.5 && after.velocity.abs() < 1. {
                    after.depth = 0.;
                    after.velocity = 0.;
                }
            }
            let end = o.at(&after);
            if old.translation == end.translation && old.rotation == end.rotation {
                self.saved.machines[k] = after;
                continue;
            }
            let angular = old.rotation.angle_between(end.rotation).abs().to_degrees();
            let steps = ((angular / 2.)
                .max(old.translation.distance(end.translation) / 8.)
                .ceil() as usize)
                .max(1);
            let mut blocked = false;
            for step in 1..=steps {
                let f = step as f32 / steps as f32;
                let pose = if step == steps {
                    end
                } else {
                    Transform {
                        translation: old.translation.lerp(end.translation, f),
                        rotation: old.rotation.slerp(end.rotation, f),
                    }
                };
                let prev = self.objects[k].pose;
                let c = self.objects[k].shape.at(pose.translation, pose.rotation);
                let hit = c.touches(p.feet + PLAYER_CENTER, p.feet + PLAYER_CENTER, PLAYER_HALF);
                if ridden || hit {
                    let local = prev.rotation.inverse() * (p.feet - prev.translation);
                    let carried = pose.translation + pose.rotation * local;
                    let landing = if ridden {
                        rider_landing(&self.objects[k], &c, carried)
                    } else {
                        None
                    };
                    let mut next = landing.map_or(carried, |p| p.0);
                    if c.touches(next + PLAYER_CENTER, next + PLAYER_CENTER, PLAYER_HALF) {
                        if let Some(clear) = c.separate_body(next, 20.) {
                            next = clear;
                        }
                    }
                    w.set_dynamic(
                        fixed
                            .iter()
                            .cloned()
                            .chain(
                                self.objects
                                    .iter()
                                    .enumerate()
                                    .filter(|(j, _)| *j != k)
                                    .map(|(_, o)| o.collider.clone()),
                            )
                            .collect(),
                    );
                    let mut tr = w.body_trace(p.feet, next);
                    if !ridden && !tr.start_solid && tr.fraction < 1. {
                        if let Some(slid) = slide_side_push(w, &c, p.feet, next) {
                            next = slid;
                            tr = w.body_trace(p.feet, next);
                        }
                    }
                    if tr.start_solid
                        || tr.fraction < 1.
                        || !w.body_clear(next)
                        || c.touches(next + PLAYER_CENTER, next + PLAYER_CENTER, PLAYER_HALF)
                    {
                        crate::route::trace(|| {
                            format!("Hedge blocked {} rider {ridden}, from {:?} to {next:?}; world solid {} fraction {}, own {}",self.objects[k].name,p.feet,tr.start_solid,tr.fraction,c.touches(next+PLAYER_CENTER,next+PLAYER_CENTER,PLAYER_HALF))
                        });
                        blocked = true;
                        break;
                    }
                    p.feet = next;
                    if let Some((_, normal)) = landing {
                        p.grounded = true;
                        p.ground_normal = normal;
                    }
                }
                self.objects[k].pose = pose;
                self.objects[k].collider = c;
            }
            if blocked {
                if self.objects[k].crusher() {
                    self.saved.crush = true;
                }
                if self.objects[k].door() {
                    self.saved.machines[k].hold = self.objects[k].wait.max(3.);
                }
                self.objects[k].pose = old;
                self.objects[k].collider = self.objects[k].shape.at(old.translation, old.rotation);
                *p = original;
            } else {
                self.saved.machines[k] = after;
            }
        }
        // Contact checks above publish the current poses excluding the pusher.
        // The other branches query each object's collider directly; publishing
        // the whole list after every distant gear was redundant quadratic work.
        self.sync(w, fixed);
        self.traversal(&mut w.traversal);
        Ok(())
    }
}

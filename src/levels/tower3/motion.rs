use super::*;
use crate::collision::model_shape::Template;
pub(super) struct Object {
    pub id: usize,
    pub name: String,
    pub model: Option<usize>,
    pub base: Vec3,
    pub parent: Option<usize>,
    pub local_angles: bool,
    pub solid: bool,
    pub damage: f32,
    pub pose: Transform,
    pub shape: Option<Template>,
    pub collider: Option<Collider>,
}
pub(super) fn owned(e: &super::super::Entity) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("script_object" | "func_fulcrum")
    )
}
pub(super) fn load(map: &Bsp) -> Result<Vec<Object>> {
    let mut objects = vec![];
    for (id, e) in map.entities.iter().enumerate().filter(|(_, e)| owned(e)) {
        let model = e
            .get("model")
            .map(|m| m.trim_start_matches('*').parse::<usize>())
            .transpose()?;
        let base = interaction::vector(&e["origin"]).context("Bad Tower machine origin")?;
        let shape = model.map(|m| Template::model(map, m)).transpose()?;
        let name = e.get("targetname").cloned().unwrap_or_default();
        let damage = if matches!(
            name.as_str(),
            "big_wheel" | "big_cage" | "big_gear2" | "big_gear2_latch" | "grinder01" | "grinder02"
        ) {
            1000.
        } else {
            e.get("dmg").and_then(|n| n.parse().ok()).unwrap_or(0.)
        };
        objects.push(Object {
            id,
            name,
            model,
            base,
            parent: None,
            local_angles: false,
            solid: id != 5,
            damage,
            pose: Transform {
                translation: base,
                rotation: Quat::IDENTITY,
            },
            collider: shape.as_ref().map(|s| s.at(base, Quat::IDENTITY)),
            shape,
        });
    }
    for (child, parent, local) in [
        (2, 1, true),
        (3, 1, true),
        (52, 54, true),
        (53, 54, true),
        (49, 51, true),
        (50, 51, true),
        (55, 56, true),
        (57, 56, true),
        (59, 58, true),
        (48, 37, false),
        (47, 37, false),
        (139, 27, false),
        (145, 144, false),
        (155, 154, false),
        (156, 163, false),
        (46, 163, false),
        (175, 177, false),
        (174, 20, false),
        (138, 31, false),
        (433, 38, false),
    ] {
        let p = objects
            .iter()
            .position(|o| o.id == parent)
            .context("Missing Tower parent")?;
        let c = objects
            .iter_mut()
            .find(|o| o.id == child)
            .context("Missing bound Tower part")?;
        c.parent = Some(p);
        c.local_angles = local;
    }
    ensure!(objects.len() == 49, "Tower object count changed");
    Ok(objects)
}
impl Object {
    pub fn set(&mut self, pose: Transform) {
        self.pose = pose;
        self.collider = self
            .shape
            .as_ref()
            .map(|s| s.at(pose.translation, pose.rotation));
    }
    fn local(&self, t: f64) -> Transform {
        let mut p = self.base;
        let mut r = Quat::IDENTITY;
        let (axis, rate) = match self.name.as_str() {
            "gear01" => (Vec3::Z, -90.),
            "gear02" => (Vec3::Z, 90.),
            "big_gear1" | "gear04" | "gear06_2" | "gearup_02" => (Vec3::Z, 30.),
            "gear03" => (Vec3::Z, 60.),
            "gear03_2" => (Vec3::Z, -60.),
            "gear05" | "gear06" | "gear_plat" | "gearup_01" => (Vec3::Z, -30.),
            "pedns1_bar" => (Vec3::X, 60.),
            "pedns2_bar" => (Vec3::X, -60.),
            "pedew1_bar" => (Vec3::Y, 60.),
            "pedew2_bar" => (Vec3::Y, 20.),
            "med_gear" => (Vec3::Y, 30.),
            "grinder01" => (Vec3::X, 30.),
            "grinder02" => (Vec3::Z, 30.),
            "big_cage" => (Vec3::Z, 24.),
            "big_wheel" => (Vec3::Y, -12.),
            "big_gear2" => (Vec3::X, 180. / 32.),
            _ => (Vec3::Z, 0.),
        };
        if rate != 0. {
            r = rotation(axis, t * rate);
        }
        let (height, leg) = match self.name.as_str() {
            "gear03_bind" => (256., 4.),
            "gear04_bind" => (512., 12.),
            "gear05_bind" => (304., 10.),
            "gear06_bind" | "gear_plat_bind" | "gearup_01_bind" => (256., 10.),
            "gearup_02_bind" => (-256., 12.),
            _ => (0., 1.),
        };
        p.z += height * triangle(t, leg);
        match self.name.as_str() {
            "penew1_bar" => r = rotation(Vec3::Y, 30. * (t * std::f64::consts::TAU / 4.).sin()),
            "big_gear2_latch" => {
                let c = t.rem_euclid(4.);
                r = rotation(
                    Vec3::X,
                    if c < 0.2 {
                        40. * c
                    } else {
                        8. * (4. - c) / 3.8
                    },
                );
            }
            "arm_bar" => {
                // The authored relative "down" command subtracts its signed
                // argument. Its negative first leg therefore sweeps north.
                let c = t.rem_euclid(24.);
                let a = if c < 10. {
                    7. * c
                } else if c < 12. {
                    70.
                } else if c < 22. {
                    70. - 7. * (c - 12.)
                } else {
                    0.
                };
                r = rotation(Vec3::Z, a);
            }
            "arm_wheel" => {
                let c = t.rem_euclid(24.);
                let a = if c < 10. {
                    -70. * c
                } else if c < 12. {
                    -700.
                } else if c < 22. {
                    -700. + 70. * (c - 12.)
                } else {
                    0.
                };
                r = rotation(Vec3::Z, a);
            }
            _ => {}
        }
        Transform {
            translation: p,
            rotation: r,
        }
    }
}
fn rotation(axis: Vec3, degrees: f64) -> Quat {
    Quat::from_axis_angle(axis, (degrees.rem_euclid(360.) as f32).to_radians())
}
fn triangle(t: f64, leg: f64) -> f32 {
    let c = t.rem_euclid(2. * leg);
    (if c < leg { c / leg } else { 2. - c / leg }) as f32
}
fn root(objects: &[Object], mut i: usize) -> usize {
    while let Some(p) = objects[i].parent {
        i = p;
    }
    i
}
pub(super) fn poses(objects: &[Object], group: usize, t: f64) -> Vec<(usize, Transform)> {
    fn at(o: &[Object], i: usize, t: f64) -> Transform {
        let mut p = o[i].local(t);
        if let Some(parent) = o[i].parent {
            let q = at(o, parent, t);
            p.translation = q.translation + q.rotation * (p.translation - o[parent].base);
            if !o[i].local_angles {
                p.rotation = q.rotation * p.rotation;
            }
        }
        p
    }
    objects
        .iter()
        .enumerate()
        .filter(|(i, _)| root(objects, *i) == group)
        .map(|(i, _)| (i, at(objects, i, t)))
        .collect()
}
pub(super) fn support(c: &Collider, p: &Player) -> bool {
    let h = c.trace(
        p.feet + PLAYER_CENTER,
        p.feet + PLAYER_CENTER - Vec3::Z * 3.,
        PLAYER_HALF,
    );
    p.velocity.z <= 1. && !h.start_solid && h.fraction < 1. && h.normal.z >= 0.65
}
impl Tower {
    pub(super) fn move_all(
        &mut self,
        dt: f32,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        for g in 0..self.roots.len() {
            let root = self.roots[g];
            let before = self.saved.clocks[g];
            let after = (before + dt as f64).min(1e8);
            let old = poses(&self.objects, root, before);
            let end = poses(&self.objects, root, after);
            if old
                .iter()
                .zip(&end)
                .all(|((_, a), (_, b))| a.translation == b.translation && a.rotation == b.rotation)
            {
                self.saved.clocks[g] = after;
                continue;
            }
            let original = p.clone();
            let mut blocked = false;
            let mut damage = 0_f32;
            let n = old
                .iter()
                .zip(&end)
                .map(|((_, a), (_, b))| {
                    (a.translation.distance(b.translation) / 4.)
                        .max(a.rotation.angle_between(b.rotation).abs().to_degrees() / 2.)
                        .ceil() as usize
                })
                .max()
                .unwrap_or(1)
                .max(1);
            for step in 1..=n {
                let future = poses(
                    &self.objects,
                    root,
                    before + (after - before) * step as f64 / n as f64,
                );
                // The child's ride surface takes priority where it overlaps its
                // parent. Alternating between the bar's orbit and the wheel's
                // spin changes the rider's local point and walks her off its rim.
                let carrier = old
                    .iter()
                    .filter_map(|(i, _)| {
                        self.objects[*i]
                            .collider
                            .as_ref()
                            .filter(|c| self.objects[*i].solid && support(c, p))
                            .map(|_| {
                                let mut depth = 0;
                                let mut current = *i;
                                while let Some(parent) = self.objects[current].parent {
                                    depth += 1;
                                    current = parent;
                                }
                                (*i, depth)
                            })
                    })
                    .max_by_key(|(_, depth)| *depth)
                    .map(|(i, _)| i);
                let mut contacts = vec![];
                for (i, pose) in &future {
                    let o = &self.objects[*i];
                    if o.solid {
                        if let Some(shape) = &o.shape {
                            let c = shape.at(pose.translation, pose.rotation);
                            if c.touches(
                                p.feet + PLAYER_CENTER,
                                p.feet + PLAYER_CENTER,
                                PLAYER_HALF,
                            ) {
                                contacts.push(*i);
                            }
                        }
                    }
                }
                let mover = carrier.or_else(|| contacts.first().copied());
                if let Some(i) = mover {
                    let a = self.objects[i].pose;
                    let b = future.iter().find(|(j, _)| *j == i).unwrap().1;
                    let mut next = b.translation
                        + b.rotation * (a.rotation.conjugate() * (p.feet - a.translation));
                    for (j, pose) in &future {
                        self.objects[*j].set(*pose);
                    }
                    if carrier.is_some() {
                        let above = next + Vec3::Z * 20.;
                        // A rider can cross a raised parent lip while a child
                        // spins underneath. Step onto the highest reachable
                        // surface in the group before considering side escape.
                        let floor = future
                            .iter()
                            .filter_map(|(j, _)| {
                                let o = &self.objects[*j];
                                o.collider.as_ref().filter(|_| o.solid).and_then(|c| {
                                    let tr = c.trace(
                                        above + PLAYER_CENTER,
                                        next + PLAYER_CENTER - Vec3::Z * 4.,
                                        PLAYER_HALF,
                                    );
                                    (!tr.start_solid && tr.fraction < 1. && tr.normal.z >= 0.65)
                                        .then_some(tr.fraction)
                                })
                            })
                            .min_by(f32::total_cmp);
                        if let Some(fraction) = floor {
                            next = above.lerp(next - Vec3::Z * 4., fraction);
                        }
                    }
                    for (j, _) in &future {
                        if !self.objects[*j].solid {
                            continue;
                        }
                        if let Some(c) = &self.objects[*j].collider {
                            if c.touches(next + PLAYER_CENTER, next + PLAYER_CENTER, PLAYER_HALF) {
                                if let Some(clear) = c.separate_body(next, 20.) {
                                    next = clear;
                                }
                            }
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
                                    .filter(|(j, o)| o.solid && !old.iter().any(|(i, _)| i == j))
                                    .filter_map(|(_, o)| o.collider.clone()),
                            )
                            .collect(),
                    );
                    let tr = w.body_trace(p.feet, next);
                    blocked = tr.start_solid
                        || tr.fraction < 1.
                        || !w.body_clear(next)
                        || future.iter().any(|(j, _)| {
                            self.objects[*j].solid
                                && self.objects[*j].collider.as_ref().is_some_and(|c| {
                                    c.touches(
                                        next + PLAYER_CENTER,
                                        next + PLAYER_CENTER,
                                        PLAYER_HALF,
                                    )
                                })
                        });
                    if blocked {
                        damage = contacts
                            .iter()
                            .chain(carrier.iter())
                            .map(|j| self.objects[*j].damage)
                            .fold(0., f32::max);
                        crate::route::trace(|| {
                            format!(
                                "Tower blocked {} rider {:?} from {:?} to {:?}",
                                self.objects[i].name, carrier, p.feet, next
                            )
                        });
                        break;
                    }
                    p.feet = next;
                    if carrier.is_some() {
                        p.grounded = true;
                        p.ground_normal = Vec3::Z;
                    }
                } else {
                    for (i, pose) in future {
                        self.objects[i].set(pose);
                    }
                }
            }
            if blocked {
                for (i, pose) in old {
                    self.objects[i].set(pose);
                }
                *p = original;
                if self.saved.hurt_wait <= 0. && damage > 0. {
                    self.saved.damage = self.saved.damage.max(damage);
                    self.saved.hurt_wait = 0.5;
                }
            } else {
                self.saved.clocks[g] = after;
            }
        }
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lift_turnarounds_and_down_first() {
        assert_eq!(triangle(0., 12.), 0.);
        assert_eq!(triangle(12., 12.), 1.);
        assert_eq!(triangle(24., 12.), 0.);
        assert_eq!(-256. * triangle(6., 12.), -128.);
    }
    #[test]
    fn periodic_rotation_stays_finite() {
        for t in [0., 15., 30., 1e8] {
            assert!(rotation(Vec3::Y, -12. * t).is_finite());
        }
    }
}

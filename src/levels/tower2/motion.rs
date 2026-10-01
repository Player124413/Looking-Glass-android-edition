use super::*;
use crate::collision::{PLAYER_CENTER, PLAYER_HALF};
fn index(name: &str, prefix: &str) -> Option<usize> {
    name.strip_prefix(prefix)?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)
}
fn axis(i: usize) -> Vec3 {
    [Vec3::X, Vec3::X, Vec3::Z, Vec3::Y, Vec3::Z][i]
}
const SPEED: [f32; 5] = [256., -256., 256., -256., 100.];
impl Tower {
    fn pose(&self, o: &Object) -> (Vec3, Quat) {
        let m = self.machine();
        if o.name == "water" {
            return (o.base + Vec3::Z * (self.height() - 624.), Quat::IDENTITY);
        }
        if let Some(i) = index(&o.name, "fliptop") {
            return (o.base, Quat::from_rotation_x(m.lids[i].to_radians()));
        }
        if let Some(i) = index(&o.name, "flusher") {
            return (o.base - Vec3::Z * m.sink[i], Quat::IDENTITY);
        }
        if let Some(i) = index(&o.name, "fan") {
            return (
                o.base,
                Quat::from_axis_angle(axis(i), m.fans[i].to_radians()),
            );
        }
        (o.base, Quat::IDENTITY)
    }
    pub(super) fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        for j in 0..self.objects.len() {
            let (p, r) = self.pose(&self.objects[j]);
            let o = &mut self.objects[j];
            if o.pose.translation != p || o.pose.rotation != r {
                o.collider = Collider::model(map, o.model, p, r, o.name != "water")?;
                o.pose = Transform {
                    translation: p,
                    rotation: r,
                };
            }
            if o.name == "water" {
                self.water.volume = Collider::model(map, o.model, p, r, false)?;
            }
        }
        Ok(())
    }
    fn sync(&self, w: &mut World, fixed: &[Collider]) {
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        w.set_dynamic_liquids(self.liquids());
    }
    fn supported(o: &Object, p: &Player) -> bool {
        let c = p.feet + PLAYER_CENTER;
        let h = o.collider.trace(c, c - Vec3::Z * 3., PLAYER_HALF);
        p.velocity.z <= 1. && !h.start_solid && h.fraction < 1. && h.normal.z >= 0.65
    }
    pub(super) fn move_machinery(
        &mut self,
        dt: f32,
        map: &Bsp,
        w: &mut World,
        p: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        let world_dt = if self.stopped { 0. } else { dt };
        for t in self.saved.stages.iter_mut().flatten() {
            *t = (*t + world_dt).min(8.);
        }
        self.rebuild(map)?;
        self.sync(w, fixed);
        for j in 0..self.objects.len() {
            let o = &self.objects[j];
            let name = o.name.clone();
            if name == "water" {
                continue;
            }
            let before = self.machine().clone();
            let old_pose = o.pose;
            let rider = !name.starts_with("fan") && Self::supported(o, p);
            let local = old_pose.rotation.inverse() * (p.feet - old_pose.translation);
            let m = self.saved.machine.as_mut().unwrap();
            if let Some(i) = index(&name, "flusher") {
                // The authored speed controls a per-20Hz velocity increment;
                // it is not 500 units/second. Vacated trays spring back with damping.
                let step = 20. * dt;
                if rider {
                    m.sink_velocity[i] += 0.004 * 500. * step;
                } else {
                    m.sink_velocity[i] -= m.sink[i] * 0.0004 * 500. * step;
                    m.sink_velocity[i] *= 0.95_f32.powf(step);
                }
                m.sink[i] += m.sink_velocity[i] * dt;
                if m.sink[i] >= 64. {
                    m.sink[i] = 64.;
                    m.sink_velocity[i] = 0.;
                }
                if m.sink[i] <= 0. || !rider && m.sink[i] < 0.5 && m.sink_velocity[i].abs() < 1. {
                    m.sink[i] = 0.;
                    m.sink_velocity[i] = 0.;
                }
            } else if let Some(i) = index(&name, "fliptop") {
                if world_dt == 0. || self.saved.stages[i].is_none() {
                    continue;
                }
                let closing = i < 2 && self.saved.stages[i + 1].is_some();
                let target = if closing {
                    90.
                } else if i == 2 {
                    -35.
                } else {
                    35.
                };
                let duration = if closing {
                    8.
                } else if i == 2 {
                    5.
                } else {
                    6.
                };
                let rate = if closing {
                    55. / duration
                } else {
                    35. / duration
                };
                let goal = if !closing
                    && self.saved.stages[i].is_some_and(|t| t >= duration)
                    && (m.lids[i] - target).abs() <= 2.1
                {
                    m.sway[i] = (m.sway[i] + world_dt).rem_euclid(10.);
                    target + 2. * (m.sway[i] * std::f32::consts::TAU / 10.).sin()
                } else {
                    target
                };
                m.lids[i] += (goal - m.lids[i]).clamp(-rate * world_dt, rate * world_dt);
            } else if let Some(i) = index(&name, "fan") {
                if world_dt == 0. {
                    continue;
                }
                // Sweep in small angular steps, so a thin blade cannot tunnel through Alice.
                let steps = (SPEED[i].abs() * world_dt / 2.).ceil() as usize;
                let mut blocked = false;
                for k in 1..=steps {
                    let angle = (before.fans[i] + SPEED[i] * world_dt * k as f32 / steps as f32)
                        .rem_euclid(360.);
                    let c = Collider::model(
                        map,
                        o.model,
                        o.base,
                        Quat::from_axis_angle(axis(i), angle.to_radians()),
                        true,
                    )?;
                    let center = p.feet + PLAYER_CENTER;
                    if c.trace(center, center, PLAYER_HALF).start_solid {
                        blocked = true;
                        break;
                    }
                }
                if blocked {
                    m.crush |= !self.saved.scene.is_some();
                    continue;
                }
                m.fans[i] = (before.fans[i] + SPEED[i] * world_dt).rem_euclid(360.);
            }
            self.rebuild(map)?;
            self.sync(w, fixed);
            let o = &self.objects[j];
            let carried = o.pose.translation + o.pose.rotation * local;
            let landing = if rider {
                o.collider.rider_feet(carried)
            } else {
                None
            };
            let next = landing.map_or(p.feet, |p| p.0);
            if w.body_clear(next) && (!rider || landing.is_some()) {
                if let Some((feet, normal)) = landing {
                    p.feet = feet;
                    p.grounded = true;
                    p.ground_normal = normal;
                }
            } else {
                self.saved.machine = Some(before);
                self.rebuild(map)?;
                self.sync(w, fixed);
            }
        }
        Ok(())
    }
    pub(super) fn move_currents(&mut self, dt: f32, p: &mut Player) {
        if self.scripted() {
            return;
        }
        for (k, current) in self.currents.iter().enumerate() {
            let timer = &mut self.saved.machine.as_mut().unwrap().currents[k];
            *timer = (*timer - dt).max(0.);
            let c = p.feet + PLAYER_CENTER;
            if *timer <= 0.00001 && current.volume.touches(c, c + p.velocity * dt, PLAYER_HALF) {
                // TriggerPush inherits the native Trigger's 0.2 s wait. Applying
                // its velocity projection every physics tick removes all
                // swimming control and traps Alice against these drain mouths.
                p.velocity +=
                    current.direction * (current.speed - p.velocity.dot(current.direction));
                *timer = 0.2;
            }
        }
    }
    pub(super) fn supports(&self) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| {
                index(&o.name, "flusher").is_some_and(|i| self.machine().sink[i] == 0.)
                    || index(&o.name, "fliptop").is_some_and(|i| {
                        self.machine().lids[i] == 0. || self.machine().lids[i] == 90.
                    })
            })
            .map(|o| o.collider.clone())
            .collect()
    }
}

//! Queen projectile declarations and the shared native Seek/RadiusDamage conventions.
use super::*;
use crate::combat::{self, DamageKind};

#[derive(Clone, Debug)]
pub(super) struct Spec {
    pub speed: f32,
    pub damage: f32,
    pub force: f32,
    pub life: f32,
    pub half: Vec3,
    pub seeker: f32,
    pub kind: DamageKind,
    pub explosion: Option<String>,
    pub splash: f32,
    pub radius: f32,
    pub blast_force: f32,
    pub mark: Option<String>,
    pub mark_radius: f32,
}
/// Only init/server properties; client emitter lifetimes must not change physics.
fn properties(text: &str) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut out = std::collections::BTreeMap::new();
    let (mut depth, mut server) = (0, false);
    for row in crate::materials::lines(text) {
        if row[0] == "server" && depth == 1 {
            server = true;
        }
        if server && depth == 2 && row.len() > 1 {
            out.insert(row[0].clone(), row[1..].to_vec());
        }
        for word in row {
            if word == "{" {
                depth += 1;
            }
            if word == "}" {
                depth -= 1;
                if depth < 2 {
                    server = false;
                }
            }
        }
    }
    out
}
impl Spec {
    pub fn load(a: &mut Assets, model: &str) -> Result<Self> {
        let text = String::from_utf8_lossy(&a.read(&format!("models/{model}"))?).into_owned();
        let p = properties(&text);
        let number = |key: &str, default| {
            p.get(key)
                .and_then(|v| v.first()?.parse::<f32>().ok())
                .unwrap_or(default)
        };
        let explosion = p.get("explosionmodel").and_then(|v| v.first()).cloned();
        let blast = if let Some(name) = &explosion {
            properties(&String::from_utf8_lossy(
                &a.read(&format!("models/{name}"))?,
            ))
        } else {
            Default::default()
        };
        let b = |key: &str, default| {
            blast
                .get(key)
                .and_then(|v| v.first()?.parse::<f32>().ok())
                .unwrap_or(default)
        };
        // Explosion is a separately constructed entity: projectile radius is not inherited.
        let splash = b("radiusdamage", 0.);
        let half = p
            .get("setsize")
            .and_then(|v| {
                Some(
                    (crate::interaction::vector(v.get(1)?)?
                        - crate::interaction::vector(v.first()?)?)
                        * 0.5,
                )
            })
            .unwrap_or(Vec3::splat(8.));
        let out = Self {
            speed: number("speed", 800.),
            damage: number("hitdamage", 10.),
            force: number("knockback", 100.),
            life: number("life", 5.).min(15.),
            half,
            seeker: number("seeker", 0.),
            kind: match p
                .get("meansofdeath")
                .and_then(|v| v.first())
                .map(String::as_str)
            {
                Some("frozen") => DamageKind::Ice,
                Some("eyebeam" | "electric") => DamageKind::Electric,
                Some("on_fire" | "fire") => DamageKind::Fire,
                _ => DamageKind::Other,
            },
            explosion,
            splash,
            radius: b("radius", splash + 60.),
            blast_force: b("knockback", splash),
            mark: p.get("impactmarkshader").and_then(|v| v.first()).cloned(),
            mark_radius: number("impactmarkradius", 16.),
        };
        ensure!(
            out.half.is_finite() && out.half.min_element() >= 0. && out.half.max_element() <= 128.,
            "Invalid Queen projectile bounds"
        );
        Ok(out)
    }
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Impact {
    pub model: String,
    pub at: Vec3,
    pub normal: Vec3,
    pub age: f32,
    pub mark: Option<String>,
    pub mark_radius: f32,
}
impl Impact {
    pub fn valid(&self) -> bool {
        self.at.is_finite()
            && self.at.abs().max_element() < 100000.
            && self.normal.is_finite()
            && (0. ..=8.).contains(&self.age)
            && self.model.len() < 100
            && !self.model.contains("..")
            && self
                .mark
                .as_ref()
                .is_none_or(|s| s.len() < 150 && !s.contains(".."))
            && (0. ..=256.).contains(&self.mark_radius)
    }
}
pub(super) fn first_seek() -> f32 {
    0.2
}
pub(super) fn steer(velocity: Vec3, toward: Vec3, degrees: f32) -> Vec3 {
    if toward.length_squared() < 0.00001 {
        return velocity;
    }
    let angles = |v: Vec3| vec2((-v.z).atan2(v.truncate().length()), v.y.atan2(v.x));
    let a = angles(velocity);
    let b = angles(toward);
    let approach = |current: f32, target: f32| {
        current
            + ((target - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI)
                .clamp(-degrees.to_radians(), degrees.to_radians())
    };
    let p: f32 = approach(a.x, b.x);
    let y: f32 = approach(a.y, b.y);
    vec3(p.cos() * y.cos(), p.cos() * y.sin(), -p.sin()) * velocity.length()
}
fn splash(
    spec: &Spec,
    origin: Vec3,
    direct: Option<usize>,
    ctx: &combat::Context<'_>,
    out: &mut Feedback,
) {
    for body in ctx.targets {
        if Some(body.id) == direct {
            continue;
        }
        let offset = body.center - origin;
        let distance = offset.length();
        let falloff = (1. - distance / spec.radius.max(1.)).max(0.);
        let inner = if body.id == crate::dice::ALICE {
            0.25
        } else {
            0.5
        };
        if falloff <= 0.
            || (distance >= spec.radius * inner
                && ctx.world.sweep(origin, body.center, Vec3::ZERO).fraction < 1.)
        {
            continue;
        }
        out.strike(
            body.id,
            spec.splash * falloff,
            offset.normalize_or_zero() * spec.blast_force * falloff,
            spec.kind,
        );
    }
}
impl battle::Shot {
    pub fn pose(&self) -> Transform {
        Transform {
            translation: self.at,
            rotation: Quat::from_rotation_arc(
                Vec3::X,
                self.velocity.try_normalize().unwrap_or(Vec3::X),
            ),
        }
    }
    pub fn birth_pose(&self, age: f32) -> Transform {
        let i = self.trail.partition_point(|(t, _)| *t <= age);
        if i == 0 {
            return self.trail.first().map_or_else(|| self.pose(), |p| p.1);
        }
        let a = self.trail[i - 1];
        self.trail.get(i).map_or(a.1, |b| {
            a.1.blend(b.1, ((age - a.0) / (b.0 - a.0)).clamp(0., 1.))
        })
    }
    fn record(&mut self) {
        if self.trail.last().is_some_and(|(t, _)| self.age - *t < 0.04) {
            return;
        }
        self.trail.push((self.age, self.pose()));
        if self.trail.len() > 128 {
            self.trail.remove(0);
        }
    }
}
impl Queen {
    pub(super) fn projectiles(
        &mut self,
        dt: f32,
        bodies: &[Target],
        world: &World,
        out: &mut Feedback,
    ) {
        let ctx = combat::Context {
            world,
            targets: bodies,
        };
        let mut impacts = Vec::new();
        self.saved.projectiles.retain_mut(|shot| {
            if shot.end.is_some() {
                shot.age += dt;
                return shot.age < shot.life;
            }
            let Some(spec) = self
                .data
                .projectiles
                .get(&format!("{}.tik", shot.model_key()))
            else {
                return false;
            };
            let mut left = dt;
            while left > 0.000001 && shot.age < shot.life {
                if spec.seeker > 0. && shot.age + 0.000001 >= shot.seek_at {
                    if let Some(target) = bodies.iter().find(|t| Some(t.id) == shot.victim) {
                        shot.velocity = steer(shot.velocity, target.center - shot.at, spec.seeker);
                    } else {
                        shot.victim = None;
                    }
                    shot.seek_at += 0.1;
                }
                let step = left.min(shot.life - shot.age).min(if spec.seeker > 0. {
                    (shot.seek_at - shot.age).max(0.000001)
                } else {
                    left
                });
                shot.record();
                let end = shot.at + shot.velocity * step;
                let contact = combat::contact_box(&ctx, shot.at, end, spec.half);
                let wall = world.sweep(shot.at, end, spec.half);
                shot.at = shot.at.lerp(end, contact.map_or(wall.fraction, |(_, f)| f));
                shot.age += step;
                left -= step;
                if contact.is_some() || wall.start_solid || wall.fraction < 1. {
                    if let Some((id, _)) = contact {
                        out.strike(
                            id,
                            shot.damage,
                            shot.velocity.normalize_or_zero() * shot.force,
                            spec.kind,
                        );
                    }
                    let normal = if contact.is_some() {
                        -shot.velocity.normalize_or_zero()
                    } else {
                        wall.normal
                    };
                    let at = shot.at + normal * 0.1;
                    splash(spec, at, contact.map(|(id, _)| id), &ctx, out);
                    let sound = match spec.explosion.as_deref() {
                        Some("fx_queen1blaster_splode.tik") => {
                            Some("sound/character/queen/blaster_explode.wav")
                        }
                        Some("fx_kingball_exp.tik") => {
                            Some("sound/character/army_ant_corp/grenade.wav")
                        }
                        Some("fx_explosion.tik" | "fx_heartburn.tik") => {
                            Some("sound/weapon/shared/explode1.wav")
                        }
                        _ => None,
                    };
                    if let Some(sound) = sound {
                        out.spatial_sounds.push((sound, at));
                    }
                    impacts.push(Impact {
                        model: spec
                            .explosion
                            .as_deref()
                            .unwrap_or("")
                            .trim_end_matches(".tik")
                            .into(),
                        at,
                        normal,
                        age: 0.,
                        mark: if contact.is_none() {
                            spec.mark.clone()
                        } else {
                            None
                        },
                        mark_radius: spec.mark_radius,
                    });
                    return false;
                }
            }
            shot.age < shot.life
        });
        self.saved.impacts.extend(impacts);
        if self.saved.impacts.len() > 128 {
            self.saved.impacts.drain(..self.saved.impacts.len() - 128);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeking_turns_each_axis_without_changing_speed() {
        let v = steer(Vec3::X * 850., vec3(0., 100., 100.), 7.);
        assert!((v.length() - 850.).abs() < 0.001);
        assert!((v.y.atan2(v.x).to_degrees() - 7.).abs() < 0.001);
        assert!((v.z.atan2(v.truncate().length()).to_degrees() - 7.).abs() < 0.001);
    }
    #[test]
    fn server_properties_ignore_emitters_and_last_assignment_wins() {
        let p = properties("init\n{\nserver\n{\nlife 5\nradiusdamage 25\nradiusdamage 100\n}\nclient\n{\nlife 0.5\n}\n}");
        assert_eq!(p["life"][0], "5");
        assert_eq!(p["radiusdamage"][0], "100");
    }
}

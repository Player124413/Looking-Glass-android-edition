use super::*;
use crate::{particles::Attached, weapons::Prop};
pub(super) struct Face {
    pub pose: Transform,
    pub scale: f32,
    pub push: usize,
}
pub(super) fn load(m: &Bsp) -> Result<Vec<Face>> {
    [403, 164, 165, 163]
        .into_iter()
        .zip([404, 21, 405, 408])
        .map(|(id, push)| {
            let e = &m.entities[id];
            ensure!(
                e.get("classname").is_some_and(|c| c == "tower_blowface"),
                "Missing blow face {id}"
            );
            Ok(Face {
                push,
                pose: Transform {
                    translation: interaction::vector(&e["origin"])
                        .context("Invalid face position")?,
                    rotation: Quat::from_rotation_z(e["angle"].parse::<f32>()?.to_radians()),
                },
                scale: e["scale"].parse()?,
            })
        })
        .collect()
}
pub(super) fn clip(t: f64) -> (&'static str, f32, bool) {
    if t < 0.35 {
        ("close", t as f32, false)
    } else if t < 2.55 {
        ("hold", (t - 0.35) as f32, true)
    } else if t < 2.95 {
        ("open", (t - 2.55) as f32, false)
    } else {
        ("idle", 0., true)
    }
}
pub(super) struct Art {
    clips: std::collections::BTreeMap<String, Prop>,
    smoke: Vec<Attached>,
}
impl Art {
    pub fn load(
        a: &mut Assets,
        m: &std::collections::BTreeMap<String, crate::texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut clips = std::collections::BTreeMap::new();
        for name in ["idle", "close", "hold", "open"] {
            clips.insert(
                name.into(),
                Prop::load_animation(a, "tower_face01", name, m)?,
            );
        }
        let mut smoke = vec![];
        for _ in 0..4 {
            let mut fx = Attached::load(a, "tower_face01", m)?.context("Missing face smoke")?;
            fx.orient_velocity("smoke");
            smoke.push(fx);
        }
        Ok(Self { clips, smoke })
    }
    pub fn draw(&mut self, t: &Tower, bright: bool) {
        for (n, f) in t.faces.iter().enumerate() {
            let (name, time, looping) = clip(t.saved.faces[n]);
            self.clips
                .get_mut(name)
                .unwrap()
                .draw_frame(f.pose, f.scale, bright, time, looping);
        }
    }
    pub fn effects(&mut self, t: &Tower, camera: Vec3, atmo: &crate::environment::Atmosphere) {
        for (n, f) in t.faces.iter().enumerate() {
            let origin = self.clips["hold"].point(f.pose, "tag_point", f.scale);
            self.smoke[n].draw(
                t.saved.faces[n] as f32,
                f.scale,
                |_, _| Transform {
                    translation: origin,
                    rotation: f.pose.rotation,
                },
                |_, age, _| (0.35..2.55).contains(&age),
                camera,
                atmo,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_boundaries() {
        assert_eq!(clip(0.349).0, "close");
        assert_eq!(clip(0.35).0, "hold");
        assert_eq!(clip(2.55).0, "open");
        assert_eq!(clip(2.95).0, "idle");
    }
}

pub(super) fn actor_pushes(m: &Bsp) -> Result<Vec<crate::traversal::Push>> {
    let mut pushes = vec![];
    for (id, e) in m.entities.iter().enumerate() {
        if e.get("classname").is_none_or(|s| s != "trigger_push")
            || e.get("spawnflags")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0)
                & 8
                != 0
        {
            continue;
        }
        let origin = interaction::vector(&e["origin"]).context("Invalid actor pad position")?;
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
        let model = e["model"].trim_start_matches('*').parse()?;
        pushes.push(crate::traversal::Push {
            id: crate::entity::Id(id),
            enabled: true,
            volume: Collider::model(m, model, origin, Quat::IDENTITY, false)?,
            origin,
            direction,
            speed: e.get("speed").and_then(|s| s.parse().ok()).unwrap_or(1000.),
            launch: false,
            accelerate: false,
        });
    }
    Ok(pushes)
}

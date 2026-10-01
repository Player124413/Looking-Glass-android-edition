//! Reviewed temple transforms. Rendering, solids and bound emitters share these poses.
use super::*;

#[derive(Clone, Copy, Default)]
struct Key {
    time: f32,
    delta: Vec3,
    angles: Vec3,
}
fn keys(steps: &[(f32, Vec3, Vec3)]) -> Vec<Key> {
    let mut out = vec![Key::default()];
    for &(dt, p, r) in steps {
        let last = *out.last().unwrap();
        out.push(Key {
            time: last.time + dt,
            delta: last.delta + p,
            angles: last.angles + r,
        });
    }
    out
}
fn fall(delay: f32, drops: &[f32], turn: f32, tilt: f32) -> Vec<Key> {
    let mut steps = vec![(delay, Vec3::ZERO, Vec3::ZERO)];
    steps.extend(drops.iter().enumerate().map(|(i, z)| {
        (
            0.5,
            -Vec3::Z * *z,
            vec3(if i == 0 { tilt } else { 0. }, turn, 0.),
        )
    }));
    keys(&steps)
}
pub(super) struct Object {
    pub name: String,
    pub model: usize,
    pub base: Vec3,
    pub damage: f32,
    frames: Vec<Key>,
    pub pose: Transform,
    pub collider: Collider,
}
pub(super) struct Motion {
    pub objects: Vec<Object>,
    pub water: Liquid,
}
impl Motion {
    pub fn ahead(&self, map: &Bsp, s: &Saved, seconds: f32) -> Result<Vec<Collider>> {
        let mut future = s.clone();
        for t in future.movers.values_mut() {
            *t = (*t + seconds).min(60.);
        }
        let parent = self.objects.iter().find(|o| o.name == "column1a").unwrap();
        let parent_pose = desired(parent, &future);
        self.objects
            .iter()
            .filter(|o| {
                s.movers
                    .get(&o.name)
                    .is_some_and(|t| *t >= -seconds && *t < 10.)
            })
            .map(|o| {
                let mut p = desired(o, &future);
                if o.name == "column1b" {
                    p.translation = parent_pose.point(p.translation - parent.base);
                    p.rotation = parent_pose.rotation * p.rotation;
                }
                Collider::model(map, o.model, p.translation, p.rotation, true)
            })
            .collect()
    }
    pub fn load(map: &Bsp) -> Result<Self> {
        let mut objects = Vec::new();
        let mut water = None;
        for e in &map.entities {
            if e.get("classname").is_none_or(|s| s != "script_object") {
                continue;
            }
            let model = e
                .get("model")
                .and_then(|s| s.strip_prefix('*'))
                .context("Temple object lacks inline model")?
                .parse::<usize>()?;
            let base = data::origin(e);
            let contents = map.models[model]
                .brushes
                .clone()
                .fold(0, |v, i| v | map.shaders[map.brushes[i].shader].contents)
                & crate::collision::LIQUID_MASK;
            if contents != 0 {
                water = Some(Liquid {
                    contents,
                    volume: Collider::model(map, model, base, Quat::IDENTITY, false)?,
                });
                continue;
            }
            let name = e.get("targetname").context("Unnamed temple solid")?.clone();
            let mut p = base;
            if name.starts_with("oyster_block") {
                p.z -= 100.
            }
            objects.push(Object {
                name: name.clone(),
                model,
                base,
                damage: e.get("dmg").and_then(|s| s.parse().ok()).unwrap_or(2.),
                frames: program(&name),
                pose: Transform {
                    translation: p,
                    rotation: Quat::IDENTITY,
                },
                collider: Collider::model(map, model, p, Quat::IDENTITY, true)?,
            });
        }
        Ok(Self {
            objects,
            water: water.context("Temple brush water missing")?,
        })
    }
    pub fn pose(&self, name: &str) -> Option<Transform> {
        self.objects.iter().find(|o| o.name == name).map(|o| o.pose)
    }
    pub fn visible(name: &str, s: &Saved) -> bool {
        match name {
            "fakewall" => !s.open,
            "beam1" => s.events.get("Panels3").is_some_and(|t| s.clock - t >= 1.6),
            "beam2" => false,
            "beam3" => s
                .events
                .get("Panel12Fall")
                .is_some_and(|t| s.clock - t >= 1.3),
            _ => true,
        }
    }
    pub fn rebuild(&mut self, map: &Bsp, s: &Saved) -> Result<()> {
        let column_base = self
            .objects
            .iter()
            .find(|o| o.name == "column1a")
            .map(|o| o.base)
            .unwrap();
        let parent = self
            .objects
            .iter()
            .find(|o| o.name == "column1a")
            .map(|o| self.desired(o, s))
            .unwrap();
        for o in &mut self.objects {
            let mut pose = desired(o, s);
            if o.name == "column1b" && s.movers.contains_key("column1a") {
                pose.translation = parent.point(pose.translation - column_base);
                pose.rotation = parent.rotation * pose.rotation;
            }
            if pose.translation != o.pose.translation || pose.rotation != o.pose.rotation {
                o.pose = pose;
                o.collider = Collider::model(map, o.model, pose.translation, pose.rotation, true)?;
            }
        }
        Ok(())
    }
    fn desired(&self, o: &Object, s: &Saved) -> Transform {
        desired(o, s)
    }
    pub fn transforms(&self, s: &Saved) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| Self::visible(&o.name, s))
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    pub fn colliders(&self, s: &Saved) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| o.name != "fakewall" || !s.open)
            .map(|o| o.collider.clone())
            .collect()
    }
}
fn desired(o: &Object, s: &Saved) -> Transform {
    if let Some(n) = o
        .name
        .strip_prefix("oyster_block")
        .and_then(|s| s.parse::<usize>().ok())
    {
        let z = s.clams[n - 1].map_or(-100., |t| {
            let t = (t.max(0.) as f32) % 4.91;
            if t < 1.01 {
                -100.
            } else if t < 1.21 {
                -100. + 500. * (t - 1.01)
            } else if t < 3.21 {
                0.
            } else {
                -200. * (t - 3.21).min(0.5)
            }
        });
        return Transform {
            translation: o.base + Vec3::Z * z,
            rotation: Quat::IDENTITY,
        };
    }
    let age = s.movers.get(&o.name).copied().unwrap_or(-1.);
    let (p, r) = sample(&o.frames, age);
    Transform {
        translation: o.base + p,
        rotation: Quat::from_euler(
            EulerRot::XYZ,
            r.x.to_radians(),
            r.y.to_radians(),
            r.z.to_radians(),
        ),
    }
}
fn sample(frames: &[Key], age: f32) -> (Vec3, Vec3) {
    let at = frames.partition_point(|k| k.time <= age.max(0.));
    let a = frames[at.saturating_sub(1)];
    let Some(b) = frames.get(at) else {
        return (a.delta, a.angles);
    };
    let f = ((age - a.time) / (b.time - a.time)).clamp(0., 1.);
    (a.delta.lerp(b.delta, f), a.angles.lerp(b.angles, f))
}
pub(super) fn activate(s: &mut Saved, thread: &str) {
    let mut start = |name: &str, delay: f32| {
        s.movers.entry(name.into()).or_insert(-delay);
    };
    match thread {
        "Panels" | "Panels2" => {
            let base = if thread == "Panels" { 1 } else { 5 };
            for (i, delay) in [0., 0.5, 1.5, 2.2].iter().enumerate() {
                start(
                    &format!("panel{}", base + i),
                    delay + if base == 1 { 1. } else { 0. },
                );
            }
        }
        "Panels3" => {
            start("panel9", 0.);
            start("panel10", 1.);
        }
        "Panels4" => {
            start("bigpanel", 0.);
            start("bigpanel2", 0.5);
        }
        "Panel12Fall" => start("panel12", 0.),
        "Spike1Fall" => {
            start("spike1", 4.);
            start("spike2", 5.5);
        }
        "Spike3Fall" => {
            start("spike3", 3.);
            start("spike4", 4.);
        }
        "Column1Fall" => {
            start("column1a", 0.);
            start("column1b", 2.);
            start("column2", 2.);
        }
        "Column3Fall" => {
            start("column3b", 2.);
            start("column3c", 2.);
        }
        "Column6Fall" => start("column6", 0.),
        "Pillar1Fall" => start("pillar1", 0.),
        "Pillar2Fall" => start("pillar2", 0.),
        "breakwall" => {
            for n in 1..=4 {
                start(&format!("piece{n}"), 0.);
            }
        }
        _ => {}
    }
}
fn program(name: &str) -> Vec<Key> {
    let z = Vec3::ZERO;
    if let Some(n) = name
        .strip_prefix("panel")
        .and_then(|s| s.parse::<usize>().ok())
    {
        let drops: &[f32] = match n {
            1..=4 => &[10., 20., 40., 80., 120., 120., 140., 140., 160.],
            5..=8 => &[10., 30., 60., 80., 90.],
            9..=10 => &[10., 20., 40., 80., 120., 120., 140., 150.],
            12 => &[10., 20., 40., 80., 120., 140., 150.],
            _ => &[],
        };
        let turn = if n <= 8 {
            [10., 14., 17., 23.][(n - 1) % 4]
        } else if n == 12 {
            90.
        } else {
            10.
        };
        return fall(1., drops, turn, 50.);
    }
    match name {
        "bigpanel" | "bigpanel2" => fall(
            1.,
            &[10., 20., 40., 80., 120., 120., 140., 150.],
            if name == "bigpanel" { -40. } else { 34. },
            50.,
        ),
        "spike1" => fall(0., &[10., 30., 60., 80., 100., 120.], 30., 0.),
        "spike2" => fall(0., &[20., 40., 80., 120., 120., 120.], -50., 0.),
        "spike3" | "spike4" => fall(0., &[20., 40., 80., 120., 160.], -40., 0.),
        "pillar1" => keys(&[
            (1., vec3(0., 0., -18.), vec3(-15., 0., 0.)),
            (3., z, vec3(0., 0., 30.)),
            (3., vec3(0., -80., -105.), vec3(0., 0., 60.)),
        ]),
        "pillar2" => keys(&[
            (1., vec3(0., 0., -18.), z),
            (3., z, vec3(0., 0., 30.)),
            (3., vec3(0., -50., -95.), vec3(0., 0., 60.)),
        ]),
        "column3b" => keys(&[(3., vec3(-70., 0., -70.), vec3(-90., 0., 0.))]),
        "column3c" => keys(&[(4., vec3(120., 0., -120.), vec3(70., 30., 0.))]),
        "column2" => keys(&[(7., vec3(-120., 80., -240.), vec3(-90., -30., 0.))]),
        "column1a" => {
            let mut k = keys(&[(2., vec3(30., 0., -90.), z)]);
            for n in 1..=500 {
                let t = n as f32 / 100.;
                k.push(Key {
                    time: 2. + t,
                    delta: vec3(
                        30. + 15. * (t / 3.).min(1.),
                        0.,
                        -90. - 75. * (t / 3.).min(1.),
                    ),
                    angles: vec3(
                        7.5 * t * t.min(1.6) + if t > 1.6 { 12. * (t - 1.6) } else { 0. },
                        0.,
                        0.,
                    ),
                });
            }
            k
        }
        "column1b" => keys(&[
            (4., vec3(0., 0., 10.), vec3(-25., 0., 0.)),
            (1., z, z),
            (1., vec3(0., 0., 20.), vec3(35., 0., 0.)),
        ]),
        "column6" => {
            let mut k = keys(&[(2., vec3(-60., 0., -40.), z), (3., z, z)]);
            for n in 1..=400 {
                let t = n as f32 / 100.;
                k.push(Key {
                    time: 5. + t,
                    delta: vec3(-60., 0., -40.),
                    angles: vec3(
                        -7.5 * t * t.min(1.6) - if t > 1.6 { 12. * (t - 1.6) } else { 0. },
                        0.,
                        0.,
                    ),
                });
            }
            k
        }
        "piece1" | "piece2" | "piece3" | "piece4" => {
            let (d, r) = match name {
                "piece1" => (100., vec3(120., 0., 0.)),
                "piece2" => (250., vec3(0., 0., 120.)),
                "piece3" => (150., vec3(0., 120., 0.)),
                _ => (200., vec3(-130., 0., 0.)),
            };
            keys(&[(0.5, vec3(-d, 0., 0.), r), (0.5, vec3(0., 0., -d), z)])
        }
        _ => keys(&[]),
    }
}

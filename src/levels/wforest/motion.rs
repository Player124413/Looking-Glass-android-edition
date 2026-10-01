use super::*;
pub(super) fn owned(e: &super::super::Entity) -> bool {
    e.get("targetname").is_some_and(|n| {
        matches!(
            n.as_str(),
            "cavegate"
                | "chessgate"
                | "chesswall"
                | "eyestaff_wall"
                | "wall_clip"
                | "block_trigger"
                | "fake_humpty_wall1"
                | "secretdoorbutton"
                | "secretdoor"
                | "broken_wall1"
                | "broken_wall2"
                | "hedge_door1"
                | "hedge_door2"
        ) || n.starts_with("debris")
    })
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    pose: Transform,
    collider: Collider,
}
pub(super) struct Motion {
    objects: Vec<Object>,
}
impl Motion {
    pub fn load(map: &Bsp) -> Result<Self> {
        let objects = map
            .entities
            .iter()
            .filter(|e| owned(e))
            .map(|e| {
                let model = e["model"]
                    .strip_prefix('*')
                    .context("WForest inline model missing")?
                    .parse()?;
                let base = crate::interaction::vector(&e["origin"])
                    .context("Invalid WForest mover origin")?;
                Ok(Object {
                    name: e["targetname"].clone(),
                    model,
                    base,
                    pose: Transform {
                        translation: base,
                        rotation: Quat::IDENTITY,
                    },
                    collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            objects.len() == 25,
            "WForest mover set changed: {}",
            objects.len()
        );
        Ok(Self { objects })
    }
    pub fn rebuild(&mut self, map: &Bsp, s: &Saved) -> Result<()> {
        for o in &mut self.objects {
            let next = pose(o, s);
            if next.translation != o.pose.translation || next.rotation != o.pose.rotation {
                o.pose = next;
                o.collider =
                    Collider::model(map, o.model, o.pose.translation, o.pose.rotation, true)?;
            }
        }
        Ok(())
    }
    pub fn transforms(&self, s: &Saved) -> Vec<(usize, Vec3, Quat)> {
        self.objects
            .iter()
            .filter(|o| visible(&o.name, s))
            .map(|o| (o.model, o.pose.translation, o.pose.rotation))
            .collect()
    }
    pub fn colliders(&self, s: &Saved) -> Vec<Collider> {
        self.objects
            .iter()
            .filter(|o| solid(&o.name, s))
            .map(|o| o.collider.clone())
            .collect()
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
        let old = s.clone();
        let rider = self
            .objects
            .iter()
            .find(|o| {
                if !solid(&o.name, s) || p.velocity.z > 1. {
                    return false;
                }
                let t = o.collider.trace(
                    p.feet + PLAYER_CENTER,
                    p.feet + PLAYER_CENTER - Vec3::Z * 3.,
                    vec3(1., 1., PLAYER_HALF.z),
                );
                !t.start_solid && t.fraction < 1. && t.normal.z > 0.65
            })
            .map(|o| (o.model, o.pose));
        if s.done[1]
            || s.scene
                .as_ref()
                .is_some_and(|s| s.kind == Kind::Staff && s.ending.is_some_and(|t| t >= 4.5))
        {
            s.cave = (s.cave + dt).min(4.);
        }
        if s.done[2]
            || s.scene
                .as_ref()
                .is_some_and(|s| s.kind == Kind::Caterpillar && s.ending.is_some())
        {
            s.chess = (s.chess + dt).min(4.2);
        }
        if let Some(t) = &mut s.secret {
            *t = (*t + dt).min(6.);
        }
        if let Some(t) = &mut s.destruction {
            *t = (*t + dt).min(5.5);
        }
        self.rebuild(map, s)?;
        let feet = rider.map_or(p.feet, |(id, old)| {
            let new = self.objects.iter().find(|o| o.model == id).unwrap().pose;
            new.translation + new.rotation * old.rotation.conjugate() * (p.feet - old.translation)
        });
        w.set_dynamic(fixed.to_vec());
        let sweep = w.sweep(p.feet + PLAYER_CENTER, feet + PLAYER_CENTER, PLAYER_HALF);
        w.set_dynamic(fixed.iter().cloned().chain(self.colliders(s)).collect());
        if sweep.start_solid || sweep.fraction < 1. || !w.body_clear(feet) {
            // Pause moving solids when Alice is in their sweep; destruction remains committed.
            s.cave = old.cave;
            s.chess = old.chess;
            s.secret = old.secret;
            if old.destruction.is_some() {
                s.destruction = old.destruction;
            }
            self.rebuild(map, s)?;
            w.set_dynamic(fixed.iter().cloned().chain(self.colliders(s)).collect());
        } else {
            p.feet = feet;
        }
        Ok(())
    }
}
fn solid(n: &str, s: &Saved) -> bool {
    match n {
        "wall_clip" | "block_trigger" => !s.returning,
        "chesswall" => s.returning,
        "secretdoorbutton" => s.returning && s.secret.is_none(),
        "eyestaff_wall" => s.wall_health > 0.,
        "broken_wall1" => s.returning,
        "broken_wall2" => !s.returning,
        _ if n.starts_with("debris") => false,
        _ => true,
    }
}
fn visible(n: &str, s: &Saved) -> bool {
    match n {
        "wall_clip" | "block_trigger" => false,
        "fake_humpty_wall1" | "broken_wall2" => !s.returning,
        "chesswall" | "secretdoorbutton" | "broken_wall1" => s.returning,
        "eyestaff_wall" => s.wall_health > 0. && s.wall_visible,
        _ if n.starts_with("debris") => s.destruction.is_none_or(|t| t < 1.5),
        _ => true,
    }
}
fn pose(o: &Object, s: &Saved) -> Transform {
    let mut p = Transform {
        translation: o.base,
        rotation: Quat::IDENTITY,
    };
    match o.name.as_str() {
        "cavegate" => p.translation.z -= 152. * s.cave / 4.,
        "chessgate" => p.translation.z += 192. * s.chess / 4.2,
        "secretdoorbutton" => p.translation.x -= 24. * s.secret.unwrap_or(0.).min(2.) / 2.,
        "secretdoor" => p.translation.z += 96. * ((s.secret.unwrap_or(0.) - 2.) / 4.).clamp(0., 1.),
        "hedge_door1" | "hedge_door2" => {
            p.rotation = Quat::from_rotation_z(
                (if o.name == "hedge_door1" {
                    -65_f32
                } else {
                    65_f32
                })
                .to_radians()
                    * ((s.destruction.unwrap_or(0.) - 1.5) / 4.).clamp(0., 1.),
            )
        }
        n if n.starts_with("debris") => {
            let k = n[6..].parse::<usize>().unwrap() - 1;
            let t = (s.destruction.unwrap_or(0.) - 0.5).clamp(0., 1.);
            p.translation += vec3(
                [-300., -500., -400., 0., 300., 700., 600.][k],
                -1800.,
                [-300., -400., -500., -300., -200., -300., -400.][k],
            ) * t;
            let axis = [Vec3::X, Vec3::Z, Vec3::Y][k % 3];
            p.rotation = Quat::from_axis_angle(
                axis,
                50_f32.to_radians() * t * if (3..6).contains(&k) { -1. } else { 1. },
            );
        }
        _ => {}
    }
    p
}

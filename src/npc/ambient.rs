//! Reviewed entrance-pupil circuit; no additional player collision or quest state.
use super::*;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Walk {
    node: usize,
    debt: f64,
    ticks: u64,
}
impl Walk {
    pub(super) fn valid(&self) -> bool {
        self.node < 5 && self.debt.is_finite() && (0. ..0.009).contains(&self.debt)
    }
}
pub(super) fn path(map: &Bsp, name: &str, entry: Option<&str>) -> Result<Vec<Vec3>> {
    if name != "skool1" || entry == Some("skool1_start2") {
        return Ok(Vec::new());
    }
    let names = ["t172", "t167", "t168", "t169", "t170"];
    let mut path = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|n| n == name))
            .context("Missing pupil waypoint")?;
        ensure!(
            e.get("target")
                .is_some_and(|n| n == names[(i + 1) % names.len()]),
            "Broken pupil circuit"
        );
        path.push(
            e.get("origin")
                .and_then(|s| crate::interaction::vector(s))
                .context("Invalid pupil waypoint")?,
        );
    }
    Ok(path)
}
pub(super) fn advance(actor: &mut Actor, dt: f32, world: &World, path: &[Vec3], speed: f32) {
    if dt <= 0. || !dt.is_finite() || actor.greeting > 0. || actor.speaking {
        return;
    }
    let mut walk = actor.walk.take().unwrap_or_default();
    walk.debt += dt.min(0.1) as f64;
    while walk.debt + 1e-9 >= 1. / 120. {
        walk.debt = (walk.debt - 1. / 120.).max(0.);
        let from = actor.position();
        let delta = path[walk.node].truncate() - from.truncate();
        let direction = delta.normalize_or_zero();
        let step = (speed / 120.).min(delta.length());
        let wanted = from + direction.extend(0.) * step;
        let center = vec3(0., 0., 30.);
        let half = vec3(16., 16., 30.);
        if let Some(to) = world.actor_footing(wanted + Vec3::Z * 2., center, half, 4.) {
            let hit = world.sweep(from + center, to + center, half);
            if !hit.start_solid && hit.fraction >= 1. {
                actor.footing = Some(to);
                actor.falling = 0.;
                walk.ticks += 1;
                actor.time = (walk.ticks as f64 / 120.) as f32;
                let desired = direction.y.atan2(direction.x);
                let turn = (desired - actor.yaw + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                actor.yaw += turn.clamp(-1.8 / 120., 1.8 / 120.);
                if delta.length() <= speed / 120. + 0.01 {
                    walk.node = (walk.node + 1) % path.len();
                }
            }
        }
    }
    actor.walk = Some(walk);
}

pub(super) async fn render_check(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let mut i = crate::interaction::Interactions::load(&scene.map)?;
    i.set_entry(assets, &scene.map, "skool1", None)?;
    i.sync(&mut scene.world);
    let mut endings = Vec::new();
    for hz in [30, 60, 144] {
        let mut npcs = Npcs::load(assets, &scene.map, "skool1", None, false, false)?;
        npcs.actors.retain(|a| a.spawn.name == "return_insane2");
        ensure!(npcs.actors.len() == 1, "Pupil missing");
        let eye = vec3(-2100., 1810., -410.);
        npcs.update(0., &scene.world, eye);
        let start = npcs.actors[0].position();
        let mut far = 0_f32;
        for frame in 0..hz * 32 {
            npcs.update(1. / hz as f32, &scene.world, eye);
            far = far.max(npcs.actors[0].position().distance(start));
            if frame == hz * 7 {
                let saved = npcs.snapshot();
                npcs.restore(&serde_json::from_value(serde_json::to_value(saved)?)?)?;
            }
            if hz == 60 && [120, 420, 840, 1560].contains(&frame) {
                let target = npcs.actors[0].target();
                let eye = (0..32)
                    .find_map(|n| {
                        let a = n as f32 * std::f32::consts::TAU / 32.;
                        let eye = target + vec3(a.cos() * 150., a.sin() * 150., 24.);
                        let hit = scene.world.sweep(target, eye, Vec3::splat(2.));
                        (!hit.start_solid && hit.fraction >= 1.).then_some(eye)
                    })
                    .context("No clear walking pupil camera")?;
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: eye,
                    target,
                    up: Vec3::Z,
                    fovy: 65_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                });
                crate::render_fx::begin(
                    eye,
                    target - eye,
                    frame as f32 / 60.,
                    &scene.atmosphere,
                    false,
                );
                let transforms = i.transforms();
                scene.draw(eye, frame as f32 / 60., false, false, &transforms);
                npcs.draw(eye, (target - eye).normalize(), &scene.atmosphere, false);
                scene.draw(eye, frame as f32 / 60., false, true, &transforms);
                crate::render_fx::finish();
                set_default_camera();
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/pupil-walk-{frame}.png"
                )))?;
                next_frame().await;
            }
        }
        ensure!(far > 150., "Pupil did not follow the circuit: {far}");
        let mut repeated = npcs.snapshot();
        let pause = serde_json::to_value(&repeated)?;
        npcs.update(0., &scene.world, eye);
        ensure!(
            pause == serde_json::to_value(npcs.snapshot())?,
            "Pupil moved while paused"
        );
        for _ in 0..600 {
            npcs.update(1. / 120., &scene.world, eye);
        }
        let expected = serde_json::to_value(npcs.snapshot())?;
        npcs.restore(&repeated)?;
        for _ in 0..600 {
            npcs.update(1. / 120., &scene.world, eye);
        }
        ensure!(
            expected == serde_json::to_value(npcs.snapshot())?,
            "Pupil continuation differs"
        );
        endings.push(npcs.actors[0].position());
        repeated.actors[0].walk = None;
        npcs.restore(&repeated)?;
        npcs.update(1. / 60., &scene.world, eye);
        ensure!(
            npcs.actors[0].spawn == repeated.actors[0].spawn,
            "Legacy pupil identity changed"
        );
        println!("PASS pupil circuit {hz} Hz: two loops, {far:.1} displacement, pause, mid-walk save and legacy identity");
    }
    ensure!(
        endings.iter().all(|p| p.distance(endings[0]) < 0.1),
        "Pupil frame rate drift"
    );
    ensure!(
        !placements(&scene.map, "skool1", Some("skool1_start2"), false)
            .iter()
            .any(|p| p.name == "return_insane2"),
        "Pupil reappears on return"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn circuit_is_fixed_step_pauses_and_continues_after_save() {
        let world = World::fixture(&[(vec3(-100., -100., -20.), vec3(100., 100., 0.))]);
        let path = [
            vec3(40., 0., 0.),
            vec3(40., 40., 0.),
            vec3(0., 40., 0.),
            vec3(-40., 0., 0.),
            Vec3::ZERO,
        ];
        let mut endings = Vec::new();
        for hz in [30, 60, 144] {
            let mut a = super::super::tests::actor();
            a.greeting = 0.;
            for _ in 0..hz * 20 {
                advance(&mut a, 1. / hz as f32, &world, &path, 50.);
            }
            assert!(a.walk.as_ref().unwrap().valid());
            let saved = serde_json::to_vec(&a).unwrap();
            advance(&mut a, 0., &world, &path, 50.);
            assert_eq!(saved, serde_json::to_vec(&a).unwrap());
            let mut b: Actor = serde_json::from_slice(&saved).unwrap();
            for _ in 0..600 {
                advance(&mut a, 1. / 120., &world, &path, 50.);
                advance(&mut b, 1. / 120., &world, &path, 50.);
            }
            assert_eq!(
                serde_json::to_vec(&a).unwrap(),
                serde_json::to_vec(&b).unwrap()
            );
            endings.push(a.position());
        }
        assert!(endings.iter().all(|p| p.distance(endings[0]) < 0.01));
    }
}

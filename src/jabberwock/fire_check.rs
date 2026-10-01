//! Real-rig regressions for breath direction, flight, collision and rendering.
use super::*;
use battle::{Action, Boss};

pub(super) fn check(data: &Data, kind: Kind) -> Result<()> {
    let mut samples = 0;
    let mut worst_old_angle = 0_f32;
    let mut animated_directions = Vec::new();
    for (action, frames) in [
        (Action::Fire, &[3., 7., 11., 15., 19., 25., 29.][..]),
        (Action::FlyFire, &[1., 5., 9., 13., 17.][..]),
    ] {
        for yaw in [-2.1_f32, 0., 1.3] {
            let mut boss = Boss::new(
                Transform {
                    translation: vec3(120., -230., 800.),
                    rotation: Quat::from_rotation_z(yaw),
                },
                kind,
            );
            boss.set(action);
            boss.flying = action == Action::FlyFire;
            boss.hold = 10.;
            for &frame in frames {
                boss.time = frame * data.boss().frame(boss.clip());
                let mouth =
                    data.boss()
                        .tag("tag_breath", boss.clip(), boss.time, boss.pose(), true);
                let direction = mouth.rotation * Vec3::X;
                if yaw == 0. {
                    animated_directions.push(direction);
                }
                for offset in [
                    Vec3::X * 700.,
                    Vec3::NEG_X * 700.,
                    Vec3::Y * 700.,
                    Vec3::Z * 700.,
                ] {
                    boss.shots.clear();
                    let eye = mouth.translation + offset;
                    boss.emit(data, eye, false);
                    let shot = &boss.shots[0];
                    ensure!(
                        shot.at.distance(mouth.translation) < 0.001 && shot.start == shot.at,
                        "Fire did not originate at animated mouth"
                    );
                    ensure!(
                        (shot.velocity.length() - 400.).abs() < 0.01
                            && shot.velocity.normalize().dot(direction) > 0.99999,
                        "Breath diverged from head at {action:?}/{frame}"
                    );
                    let old = offset.normalize() * 400. + Vec3::Z * offset.length();
                    worst_old_angle = worst_old_angle.max(
                        old.normalize()
                            .dot(direction)
                            .clamp(-1., 1.)
                            .acos()
                            .to_degrees(),
                    );
                    samples += 1;
                }
            }
        }
    }
    ensure!(
        animated_directions
            .iter()
            .any(|v| v.dot(animated_directions[0]) < 0.95),
        "Fixture never exercises an animated head turn"
    );

    let mut boss = Boss::new(
        Transform {
            translation: Vec3::Z * 1000.,
            rotation: Quat::IDENTITY,
        },
        kind,
    );
    boss.set(Action::Fire);
    boss.hold = 10.;
    boss.time = 3. * data.boss().frame(boss.clip());
    let mouth = data
        .boss()
        .tag("tag_breath", boss.clip(), boss.time, boss.pose(), true);
    let forward = mouth.rotation * Vec3::X;
    let eye = mouth.translation + forward * 60. + Vec3::Z * 20.;
    boss.emit(data, eye, false);
    let original = boss.clone();
    let empty = World::fixture(&[]);
    let mut hit = Feedback::default();
    for _ in 0..24 {
        boss.step(kind, &empty, eye, false, data, &mut hit);
    }
    ensure!(
        hit.damage >= 5.,
        "Visible fire cannot damage a target in its path"
    );
    let mut wall_boss = original.clone();
    let center = mouth.translation + forward * 40.;
    let blocked = World::fixture(&[(center - Vec3::splat(20.), center + Vec3::splat(20.))]);
    let mut blocked_hit = Feedback::default();
    for _ in 0..24 {
        wall_boss.step(
            kind,
            &blocked,
            eye + forward * 120.,
            false,
            data,
            &mut blocked_hit,
        );
    }
    ensure!(
        blocked_hit.damage == 0.
            && (wall_boss.shots[0].bounced || wall_boss.shots[0].ended.is_some()),
        "Breath passed through cover"
    );

    let mut moving = original;
    let launch_velocity = moving.shots[0].velocity;
    moving.step(kind, &empty, eye, true, data, &mut Feedback::default());
    ensure!(
        moving.shots[0]
            .velocity
            .distance(launch_velocity - Vec3::Z * (800. * battle::STEP))
            < 0.001,
        "Breath gravity changed"
    );
    let mut restored: Boss = serde_json::from_value(serde_json::to_value(&moving)?)?;
    for _ in 0..30 {
        moving.step(kind, &empty, eye, true, data, &mut Feedback::default());
        restored.step(kind, &empty, eye, true, data, &mut Feedback::default());
    }
    ensure!(
        serde_json::to_value(&moving)? == serde_json::to_value(&restored)?,
        "Saved fire diverged on continuation"
    );
    moving.shots.clear();
    moving.emit(data, eye, true);
    let spiral = &moving.shots[0];
    ensure!(
        spiral
            .velocity
            .normalize()
            .dot((eye - spiral.start).normalize())
            > 0.99999
            && (spiral.velocity.length() - 1000.).abs() < 0.01,
        "Spiral attack aim changed"
    );
    println!("PASS {} fire direction: {samples} animated mouth launches; removed up to {worst_old_angle:.1} degrees of off-axis aim; gravity, damage, cover, spiral aim and save continuation", kind.map());
    Ok(())
}

pub(super) async fn render(a: &mut Assets, kind: Kind) -> Result<()> {
    std::fs::create_dir_all("private/jabberwock/captures")?;
    let mut scene = crate::render::Scene::load(a, kind.map())?;
    let mut o = Encounter::load(a, &scene.map, kind)?;
    let mut art = art::Art::load(a, &o)?;
    let base = o.saved.boss.pose();
    o.saved.phase = Phase::Fight;
    for (label, action) in [("ground", Action::Fire), ("flying", Action::FlyFire)] {
        if kind == Kind::Lair && action == Action::FlyFire {
            continue;
        }
        o.saved.boss = Boss::new(base, kind);
        let b = &mut o.saved.boss;
        b.set(action);
        b.hold = 10.;
        b.flying = action == Action::FlyFire;
        if b.flying {
            b.at.z += 350.;
        }
        let yaw = Quat::from_rotation_z(b.yaw);
        let camera = crate::cinematic::Camera::look(
            b.at + yaw * vec3(350., -450., 180.),
            b.at + Vec3::Z * 120.,
        );
        // Place the target to the side: aim toward Alice must not override the mouth.
        let target = b.at + yaw * vec3(0., 700., 50.);
        for frame in 0..90 {
            let time = frame as f32 / 60.;
            let b = &mut o.saved.boss;
            b.time = time;
            if frame % 4 == 0 {
                b.emit(&o.data, target, false);
            }
            for shot in &mut b.shots {
                shot.at += shot.velocity / 60.;
                shot.velocity.z -= 800. / 60.;
                shot.age += 1. / 60.;
            }
            let cam = Camera3D {
                position: camera.eye,
                target: camera.target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            };
            clear_background(BLACK);
            set_camera(&cam);
            crate::render_fx::begin_view(&cam, time, &scene.atmosphere, false);
            scene.draw(camera.eye, time, false, false, &o.transforms());
            art.draw(&o, &scene.atmosphere, camera.eye, false);
            crate::render::depth_read_only(|| {
                scene.draw(camera.eye, time, false, true, &o.transforms());
                art.effects(&o, camera.eye, &scene.atmosphere);
            });
            let (_, dropped) = crate::render_fx::finish();
            ensure!(dropped == 0, "Fire effect submissions dropped");
            set_default_camera();
            if [20, 40, 70].contains(&frame) {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/jabberwock/captures/{}-mouth-{label}-{frame}.png",
                    kind.map()
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS {} mouth/fire native captures", kind.map());
    Ok(())
}

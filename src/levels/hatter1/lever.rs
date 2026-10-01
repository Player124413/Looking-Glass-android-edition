use super::*;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Pull {
    pub lever: usize,
    pub pose: Transform,
    pub goal: Vec3,
    pub time: f32,
    pub started: bool,
    #[serde(default)]
    accumulator: f32,
}
impl Pull {
    pub fn validate(&self, saved: &Saved) -> Result<()> {
        ensure!(
            self.lever < LEVERS.len()
                && self.pose.translation.is_finite()
                && self.pose.translation.abs().max_element() < 100_000.
                && self.goal.is_finite()
                && self.goal.abs().max_element() < 100_000.
                && self.pose.rotation.is_finite()
                && (self.pose.rotation.length() - 1.).abs() < 0.01
                && self.time.is_finite()
                && (0. ..=10.).contains(&self.time)
                && self.accumulator.is_finite()
                && (0. ..=1. / 120.).contains(&self.accumulator)
                && self.started == saved.levers[self.lever].is_some()
                && saved.scene.is_none(),
            "Invalid lever action"
        );
        Ok(())
    }
}
pub(super) fn duration(a: &mut Assets) -> Result<f32> {
    use crate::skeletal::{Animation, Definition, Skeleton};
    let def = Definition::load(a, "models/alice.tik")?;
    let rig = Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
    let clip = Animation::parse(
        &a.read(&format!("{}/{}", def.path, def.animations["use_lever"]))?,
        rig.bones.len(),
    )?;
    let (_, lever) = crate::weapons::read_model_clip(a, "lever", Some("move"))?;
    ensure!(
        (lever.frame_time - 0.05).abs() < 0.001,
        "Changed lever cue timing"
    );
    Ok(clip
        .duration()
        .max(lever.surfaces[0].frames.len() as f32 * lever.frame_time))
}
impl Clockwork {
    pub(super) fn start_pull(&mut self, k: usize, world: &World, player: &Player) {
        if !self.can_lever(k) || !player.grounded || player.ledge.is_some() || player.rope.is_some()
        {
            return;
        }
        let at = self.levers[k];
        let anchor = at.point(vec3(-2.65, 0., 0.2));
        let Some(goal) =
            world.actor_footing(anchor + Vec3::Z * 16., PLAYER_CENTER, PLAYER_HALF, 64.)
        else {
            return;
        };
        self.saved.pull = Some(Pull {
            lever: k,
            pose: Transform {
                translation: player.feet,
                rotation: at.rotation,
            },
            goal,
            time: 0.,
            started: false,
            accumulator: 0.,
        });
    }
    pub(super) fn advance_pull(&mut self, dt: f32, world: &World, player: &mut Player) {
        let Some(mut pull) = self.saved.pull.take() else {
            return;
        };
        if !pull.started {
            pull.time += dt;
            pull.accumulator += dt;
            while pull.accumulator + 0.000001 >= crate::movement::FIXED_DT {
                pull.accumulator = (pull.accumulator - crate::movement::FIXED_DT).max(0.);
                player.tick(
                    world,
                    crate::movement::Controls {
                        wish: ((pull.goal - player.feet).truncate() / 24.).clamp_length_max(1.),
                        ..Default::default()
                    },
                );
            }
            let next = player.feet;
            pull.pose.translation = next;
            if next.truncate().distance(pull.goal.truncate()) < 2. && player.grounded {
                player.velocity = Vec3::ZERO;
                self.press(pull.lever);
                // The final camera sequence follows the hand action.
                if pull.lever == 4 {
                    self.saved.scene = None;
                }
                pull.started = true;
                pull.time = 0.;
            } else if pull.time >= 1.5 {
                return;
            }
        } else {
            player.velocity = Vec3::ZERO;
            pull.time = (pull.time + dt).min(self.pull_duration);
            pull.pose.translation = player.feet;
            if pull.time >= self.pull_duration {
                if pull.lever == 4 {
                    self.begin(Kind::Stop);
                }
                return;
            }
        }
        self.saved.pull = Some(pull);
    }
    pub(super) fn pull_camera(&self, world: &World) -> Option<Camera> {
        let pull = self.saved.pull.as_ref()?;
        let target = pull.pose.translation + Vec3::Z * 32.;
        let eyes: Vec<_> = [
            vec3(-110., 95., 80.),
            vec3(-110., -95., 80.),
            vec3(-160., 0., 90.),
            vec3(30., 140., 80.),
            vec3(30., -140., 80.),
        ]
        .into_iter()
        .filter_map(|offset| {
            let desired = pull.pose.point(offset);
            let tr = world.sweep(target, desired, Vec3::splat(4.));
            (!tr.start_solid).then_some(target.lerp(desired, tr.fraction * 0.95))
        })
        .collect();
        // Prefer an oblique view that shows the hand and handle together.
        let eye = eyes
            .iter()
            .copied()
            .find(|p| p.distance(target) > 95.)
            .or_else(|| {
                eyes.into_iter().max_by(|a, b| {
                    a.distance_squared(target)
                        .total_cmp(&b.distance_squared(target))
                })
            })?;
        Some(Camera::look(eye, target))
    }
}

pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/hatter1.bsp")?)?;
    let (_, model) = crate::weapons::read_model_clip(a, "lever", Some("move"))?;
    ensure!(
        model.surfaces.iter().any(|s| s.frames[0]
            .iter()
            .zip(s.frames.last().unwrap())
            .any(|(a, b)| a.distance(*b) > 10.)),
        "Lever frames never move"
    );
    for hz in [30, 60, 144] {
        for k in 0..5 {
            let mut o = Clockwork::load(a, &map)?;
            o.saved.initialized = true;
            o.saved.age = 100.;
            if k >= 2 {
                o.saved.levers[0] = Some(1.);
                o.saved.levers[1] = Some(2.);
            }
            if k >= 3 {
                o.saved.levers[2] = Some(5.);
                o.saved.hare = Some(10.);
            }
            if k == 4 {
                o.saved.levers[3] = Some(12.);
                o.saved.port = Some(30.);
                o.saved.gryphon = Some(40.);
                o.saved.extendo = Some(60.);
                o.saved.cubes = [true; 4];
                o.saved.clock = Some(90.);
            }
            o.rebuild(&map)?;
            let mut w = World::from_bsp(&map)?;
            w.set_dynamic(o.colliders());
            let at = o.levers[k];
            let approach = at.point(vec3(-64., 0., 60.));
            let feet = w
                .actor_footing(approach, PLAYER_CENTER, PLAYER_HALF, 128.)
                .context("Lever approach unsupported")?;
            let mut p = Player::new(feet);
            p.grounded = true;
            let aim = (at.translation + Vec3::Z * 20. - p.eye()).normalize();
            ensure!(
                o.use_lever(&w, p.eye(), aim) == Some(k),
                "Lever {k} unavailable"
            );
            o.update(&mut w, &p, aim, true);
            ensure!(o.controlled() && o.blocks_weapons(), "Lever {k} did not start approach");
            for _ in 0..hz * 2 {
                if o.saved.pull.as_ref().is_some_and(|p| p.started) {
                    break;
                }
                o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
            }
            ensure!(
                o.saved.pull.as_ref().is_some_and(|p| p.started),
                "Lever {k} approach blocked at {:?}",
                p.feet
            );
            for _ in 0..hz {
                o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
            }
            let saved = o.snapshot();
            if k == 4 && hz == 30 {
                let mut interrupted = Clockwork::load(a, &map)?;
                interrupted.restore(&saved, &map)?;
                let mut stats = Stats::for_level("hatter1", None);
                stats.set_health(0.)?;
                interrupted.prepare_player(&mut stats, &mut p.clone());
                ensure!(
                    !interrupted.controlled() && interrupted.saved.levers[4].is_none(),
                    "Death locked the final lever"
                );
            }
            o.advance(0., &map, &mut w, &mut p, &[])?;
            ensure!(saved == o.snapshot(), "Paused lever changed");
            let mut r = Clockwork::load(a, &map)?;
            r.restore(&saved, &map)?;
            let mut rp = p.clone();
            let mut rw = World::from_bsp(&map)?;
            rw.set_dynamic(r.colliders());
            for _ in 0..hz * 5 {
                o.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
                r.advance(1. / hz as f32, &map, &mut rw, &mut rp, &[])?;
                ensure!(
                    o.snapshot() == r.snapshot() && p.feet == rp.feet,
                    "Lever continuation diverged"
                );
                ensure!(w.body_clear(p.feet), "Lever action entered solid");
            }
            ensure!(
                o.saved.pull.is_none() && o.saved.levers[k].is_some(),
                "Lever {k} did not release"
            );
            ensure!(
                o.saved.scene.is_some() == (k == 3 || k == 4),
                "Wrong lever scene handoff"
            );
        }
    }
    println!("PASS five Clockwork lever actions: source animation, physical approach, pause, restore, completion at 30/60/144 Hz");
    Ok(())
}

pub(super) async fn render(
    a: &mut Assets,
    world: &mut crate::render::Scene,
    art: &mut super::art::Art,
) -> Result<()> {
    let mut o = Clockwork::load(a, &world.map)?;
    world.world.set_dynamic(o.colliders());
    let at = o.levers[0];
    let feet = world
        .world
        .actor_footing(
            at.point(vec3(-64., 0., 60.)),
            PLAYER_CENTER,
            PLAYER_HALF,
            128.,
        )
        .context("Lever render approach")?;
    let mut p = Player::new(feet);
    p.grounded = true;
    o.update(
        &mut world.world,
        &p,
        (at.translation + Vec3::Z * 20. - p.eye()).normalize(),
        true,
    );
    let mut elapsed = 0.;
    for (name, time) in [
        ("lever-approach", 0.),
        ("lever-reach", 1.),
        ("lever-pull", 2.),
        ("lever-finish", 3.8),
    ] {
        while elapsed < time {
            o.advance(1. / 120., &world.map, &mut world.world, &mut p, &[])?;
            elapsed += 1. / 120.;
        }
        let camera = o
            .pull_camera(&world.world)
            .context("Lever camera missing")?;
        let eye = camera.eye;
        let cam = Camera3D {
            position: eye,
            target: camera.target,
            up: Vec3::Z,
            fovy: 65_f32.to_radians(),
            z_near: 2.,
            z_far: 20000.,
            ..Default::default()
        };
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&cam);
            crate::render_fx::begin_view(&cam, elapsed, &world.atmosphere, false);
            world.draw(eye, elapsed, false, false, &o.transforms());
            art.draw(&o, &world.atmosphere, eye, false);
            crate::render::depth_read_only(|| {
                world.draw(eye, elapsed, false, true, &o.transforms())
            });
            crate::render_fx::finish();
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/hatter1-work/captures/{name}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS native Clockwork lever approach/reach/pull/finish captures");
    Ok(())
}

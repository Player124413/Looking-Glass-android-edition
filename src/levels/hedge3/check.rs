use super::*;
fn arrival(a: &mut Assets, map: &Bsp) -> Result<()> {
    for entry in [None, Some("hedge3_start1")] {
        let mut i = Interactions::load(map)?;
        i.set_entry(a, map, "hedge3", entry)?;
        let mut w = World::from_bsp(map)?;
        i.sync(&mut w);
        let (eye, yaw) = interaction::spawn(map, entry);
        let mut p = Player::spawn(&w, eye).context("Blocked Hedge arrival")?;
        ensure!(
            p.grounded,
            "First Hedge frame still selects the falling pose"
        );
        let start = p.feet;
        ensure!(
            start.truncate() == vec2(6144., -7168.) && (start.z - 232.).abs() < 0.3,
            "Hedge arrival does not stand on the entrance platform: {start:?}"
        );
        ensure!(
            (yaw - std::f32::consts::FRAC_PI_2).abs() < 0.001,
            "Hedge arrival faces away from route"
        );
        for _ in 0..120 {
            i.advance_school(FIXED_DT, map, &mut w, &mut p)?;
            p.tick(&w, Controls::default());
            i.sync(&mut w);
            ensure!(
                w.body_clear(p.feet) && p.feet.distance(start) < 0.2 && p.grounded,
                "Hedge arrival falls or floats before gameplay: {:?}",
                p.feet
            );
        }
    }
    println!("PASS Hedge arrival: named/default entry grounded before first tick, stable idle and route-facing yaw");
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/hedge3.bsp")?)?;
    axle_side_contact(&map)?;
    arrival(a, &map)?;
    if std::env::var_os("LOOKING_GLASS_HEDGE3_GAPS").is_some() {
        return pump_windows(&map);
    }
    contact_checks(&map)?;
    tooth_rider(&map)?;
    door_audio(&map)?;
    sky_triggers(a, &map)?;
    let mut m = Labyrinth::load(&map)?;
    println!(
        "HEDGE machines {} doors {}",
        m.objects.len(),
        m.objects.iter().filter(|o| o.door()).count()
    );
    for (k, o) in m.objects.iter().enumerate() {
        println!(
            "HEDGE object {k} entity {} {} model {} hulls {:?} bounds {:?}",
            o.id,
            o.name,
            o.model,
            o.shape.counts(),
            o.bounds
        );
    }
    ensure!(
        m.objects.iter().filter(|o| o.door()).count() == 27,
        "Missing Hedge doors"
    );
    for name in ["turbine01", "turbine02", "piston01", "piston02", "t26"] {
        ensure!(
            m.objects[m.index(name)].shape.counts().1 > 0,
            "Missing patch hulls for {name}"
        );
    }
    for hz in [30, 60, 144] {
        let mut t = Labyrinth::load(&map)?;
        let mut w = World::from_bsp(&map)?;
        let mut p =
            Player::spawn(&w, interaction::spawn(&map, None).0).context("Blocked Hedge arrival")?;
        let home = p.feet;
        for _ in 0..hz * 16 {
            t.advance(1. / hz as f32, &map, &mut w, &mut p, &[])?;
            ensure!(w.body_clear(p.feet), "Hedge mechanism embedded arrival");
        }
        ensure!(p.feet == home, "Distant machinery moved Alice");
        let save = t.snapshot();
        let mut loaded = Labyrinth::load(&map)?;
        loaded.restore(&save, &map)?;
        let mut v = World::from_bsp(&map)?;
        let mut q = p.clone();
        for _ in 0..120 {
            t.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
            loaded.advance(FIXED_DT, &map, &mut v, &mut q, &[])?;
            ensure!(
                t.snapshot() == loaded.snapshot() && p.feet == q.feet,
                "Hedge phase restore diverged"
            );
        }
        let state = t.snapshot();
        t.advance(0., &map, &mut w, &mut p, &[])?;
        ensure!(t.snapshot() == state, "Paused Hedge moved");
        println!(
            "PASS Hedge {hz}Hz: moving hulls, clear arrival, exact saved continuation and pause"
        );
    }
    for name in ["hedge1", "hedge3"] {
        let map = Bsp::parse(&a.read(&format!("maps/{name}.bsp"))?)?;
        let mut sky = crate::sky_sequence::Controller::load(a, &map, name)?;
        for n in 1..=7 {
            let enabled = sky.event(&format!("change_to_sky{n}"));
            ensure!(
                enabled == (name == "hedge3" || n <= 3),
                "Sky registration changed in {name}"
            );
            if enabled {
                ensure!(
                    sky.state.sky.as_deref() == Some(&format!("sky_camera{n}")),
                    "Wrong selected sky"
                );
            }
            let mut saved = crate::sky_sequence::Controller::load(a, &map, name)?;
            saved.restore(&sky.state)?;
            ensure!(saved.state.sky == sky.state.sky, "Lost saved sky");
        }
        println!("PASS {name} sky switches and restored selection");
    }
    let mut w = World::from_bsp(&map)?;
    let mut p = Player::new(vec3(6144., -7168., 224.));
    for n in 0..960 {
        m.advance(FIXED_DT, &map, &mut w, &mut p, &[])?;
        for (id, num) in [(57, 2), (822, 1)] {
            ensure!(
                w.traversal
                    .pushes
                    .iter()
                    .find(|p| p.id == Id(id))
                    .unwrap()
                    .enabled
                    == m.gust(num),
                "Bellow wind is detached from phase"
            );
        }
        if n == 600 {
            ensure!(m.gust(1) && m.gust(2), "Missing bellow gust");
        }
    }
    for time in [0., 3.99, 4., 7.99, 8.] {
        for s in &mut m.saved.machines {
            s.time = time;
        }
        m.traversal(&mut w.traversal);
        for id in [Id(57), Id(822)] {
            let push = w.traversal.pushes.iter().find(|p| p.id == id).unwrap();
            let mut velocity = Vec3::ZERO;
            w.traversal.push(push.origin - PLAYER_CENTER, &mut velocity);
            ensure!(
                (velocity.x < -999.) == m.gust(1),
                "Bellow phase fails to exert actual force"
            );
        }
    }
    println!(
        "PASS Hedge bellows: saved four-stage machine clocks drive both pushes and actual force"
    );
    Ok(())
}

fn axle_side_contact(map: &Bsp) -> Result<()> {
    // Saved live-route contact beside the second bellows. Rotating the feet
    // around this axle produces a downward component against the fixed floor.
    let mut t = Labyrinth::load(map)?;
    for s in &mut t.saved.machines {
        s.time = 145.6916742650792;
    }
    t.rebuild();
    let mut w = World::from_bsp(map)?;
    w.set_dynamic(t.colliders());
    let mut p = Player::new(vec3(3808.0005, -1836.9924, 360.03125));
    p.grounded = true;
    ensure!(w.body_clear(p.feet), "Axle side-contact fixture starts embedded");
    t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
    ensure!(!t.saved.crush, "Axle side contact falsely crushed Alice");
    ensure!(w.body_clear(p.feet), "Axle side contact embedded Alice");
    println!("PASS Hedge authored axle side contact: clear displacement without false crush");
    Ok(())
}
fn tooth_rider(map: &Bsp) -> Result<()> {
    let mut t = Labyrinth::load(map)?;
    for s in &mut t.saved.machines {
        s.time = 165.0000086054206;
    }
    t.rebuild();
    let mut w = World::from_bsp(map)?;
    w.set_dynamic(t.colliders());
    let mut p = Player::new(vec3(3479.6826, -3366.2825, 968.03125));
    p.velocity = vec3(24.608715, -14.765301, 0.);
    p.grounded = true;
    for _ in 0..240 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        ensure!(
            !t.saved.crush,
            "Turntable tooth killed a free rider at {:?}",
            p.feet
        );
        p.tick(&w, Controls::default());
        ensure!(w.body_clear(p.feet), "Turntable tooth embedded rider");
    }
    println!("PASS Hedge upright rider steps across raised turntable teeth without a false crush");
    // A descending jump meets the side of a rotating brace. The brace can push
    // Alice into open space; contact with its own face is not a world blockage.
    let mut t = Labyrinth::load(map)?;
    for s in &mut t.saved.machines {
        s.time = 165.8750086;
    }
    t.rebuild();
    let mut w = World::from_bsp(map)?;
    w.set_dynamic(t.colliders());
    let mut p = Player::new(vec3(3604.8665, -3275.271, 1009.2535));
    p.velocity = vec3(240., 150., -250.);
    for _ in 0..30 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        ensure!(!t.saved.crush, "Rotating brace killed an unconfined jumper");
        p.tick(&w, Controls::default());
        ensure!(w.body_clear(p.feet), "Rotating brace embedded jumper");
    }
    Ok(())
}
pub(super) async fn render(a: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(a, "hedge3")?;
    let map = Bsp::parse(&a.read("maps/hedge3.bsp")?)?;
    let mut m = Labyrinth::load(&map)?;
    let mut sky = crate::sky_sequence::Controller::load(a, &map, "hedge3")?;
    sky.apply(&mut scene);
    for (name, eye, target, time) in [
        (
            "lava",
            vec3(5960., -7080., 480.),
            vec3(5540., -6100., -350.),
            3.,
        ),
        (
            "bellows",
            vec3(4250., -2048., 450.),
            vec3(3700., -2048., 410.),
            6.,
        ),
        (
            "gears",
            vec3(3648., -2820., 1120.),
            vec3(3600., -3136., 990.),
            7.5,
        ),
        (
            "crushers",
            vec3(5930., -3708., 1390.),
            vec3(6540., -3708., 1330.),
            4.5,
        ),
        (
            "upper",
            vec3(5130., -2048., 2190.),
            vec3(4400., -2048., 2320.),
            2.,
        ),
    ] {
        for s in &mut m.saved.machines {
            s.time = time;
        }
        m.rebuild();
        let mut particles = crate::particles::Steam::load(a, &map)?;
        particles.animate(time as f32);
        let mut interactions = Interactions::load(&map)?;
        interactions.set_entry(a, &map, "hedge3", None)?;
        particles.sync(&interactions.event_world);
        m.particles(&mut particles);
        let mut world = World::from_bsp(&map)?;
        world.set_dynamic(m.colliders());
        ensure!(
            !world.sweep(eye, eye, Vec3::ONE).start_solid,
            "Render camera inside {name} geometry"
        );
        for _ in 0..120 {
            particles.update(FIXED_DT, eye, &world);
        }
        for id in [Id(818), Id(823)] {
            let (enabled, count) = particles.emission(id).context("Missing bellow emitter")?;
            ensure!(enabled == m.gust(1), "Bellow particle phase mismatch");
            if name == "bellows" {
                ensure!(count > 0, "Bellow on without visible puffs");
            }
        }
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            scene.draw(eye, time as f32, false, false, &m.transforms());
            crate::render::depth_read_only(|| {
                scene.draw_with_particles(
                    eye,
                    (target - eye).normalize_or_zero(),
                    time as f32,
                    false,
                    &m.transforms(),
                    &particles,
                )
            });
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/hedge3-work/{name}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    // One open-air viewpoint keeps the geometry/camera fixed while changing only
    // the authored sky origin, including the four previously ignored selections.
    for n in 1..=7 {
        sky.event(&format!("change_to_sky{n}"));
        sky.apply(&mut scene);
        let eye = vec3(6144., -7100., 340.);
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target: eye + vec3(0., 400., 700.),
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 20000.,
                ..Default::default()
            });
            scene.draw(eye, 2., false, false, &m.transforms());
            crate::render::depth_read_only(|| scene.draw(eye, 2., false, true, &m.transforms()));
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/hedge3-work/sky{n}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS Hedge machine, bellow-particle and seven-sky render captures");
    Ok(())
}
fn phase(i: &mut Interactions, _: &Bsp, time: f64, sink: bool) -> Result<()> {
    let t = owner(i)?;
    for m in &mut t.saved.machines {
        m.time = time;
    }
    if sink {
        let k = t.objects.iter().position(|o| o.id == 28).unwrap();
        t.saved.machines[k].depth = 28.;
        t.saved.machines[k].velocity = 8.;
    }
    t.rebuild();
    Ok(())
}
pub(super) const SAVES: &[super::super::SaveCase] = &[
    super::super::SaveCase {
        name: "hedge3-gears",
        visit: "hedge3$first",
        stage: Some(|i, m| phase(i, m, 7.5, false)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "hedge3-gust",
        visit: "hedge3$first",
        stage: Some(|i, m| phase(i, m, 5., false)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "hedge3-crusher",
        visit: "hedge3$first",
        stage: Some(|i, m| phase(i, m, 4.5, false)),
        behavior: None,
    },
    super::super::SaveCase {
        name: "hedge3-sink",
        visit: "hedge3$first",
        stage: Some(|i, m| phase(i, m, 3., true)),
        behavior: None,
    },
];

fn isolated(map: &Bsp, name: &str) -> Result<Labyrinth> {
    let mut t = Labyrinth::load(map)?;
    t.objects
        .retain(|o| o.name == name || name == "sink" && o.id == 28);
    ensure!(t.objects.len() == 1, "Missing contact fixture {name}");
    t.saved.machines = vec![Mechanism::default()];
    t.rebuild();
    Ok(t)
}
fn top(o: &motion::Object) -> Result<Vec3> {
    for x in -8..=8 {
        for y in -8..=8 {
            let a = o.base + vec3(x as f32 * 36., y as f32 * 36., 600.);
            let b = a - Vec3::Z * 1000.;
            let hit = o
                .collider
                .trace(a + PLAYER_CENTER, b + PLAYER_CENTER, PLAYER_HALF);
            let p = a.lerp(b, hit.fraction);
            if !hit.start_solid
                && hit.fraction < 1.
                && hit.normal.z > 0.99
                && p.truncate().distance(o.base.truncate()) > 80.
                && !o
                    .collider
                    .touches(p + PLAYER_CENTER, p + PLAYER_CENTER, PLAYER_HALF)
            {
                return Ok(p);
            }
        }
    }
    anyhow::bail!("No clear rider on {}", o.name)
}
fn contact_checks(map: &Bsp) -> Result<()> {
    for name in ["lavagear", "ridegear01", "ridegear02", "ridegear03", "sink"] {
        let mut t = isolated(map, name)?;
        let mut w = World::fixture(&[]);
        let mut p = Player::new(top(&t.objects[0])?);
        p.grounded = true;
        w.set_dynamic(t.colliders());
        let start = p.feet;
        for _ in 0..120 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            ensure!(w.body_clear(p.feet), "Embedded rider on {name}");
        }
        ensure!(
            p.feet.distance(start) > 1.,
            "{name} failed to carry a rider"
        );
        if name == "sink" {
            ensure!(p.feet.z < start.z - 1., "Occupied sink did not descend");
        }
        let mut b = isolated(map, name)?;
        b.restore(&t.snapshot(), map)?;
        let mut v = World::fixture(&[]);
        let mut q = p.clone();
        for _ in 0..120 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            b.advance(FIXED_DT, map, &mut v, &mut q, &[])?;
            ensure!(
                p.feet == q.feet && t.snapshot() == b.snapshot(),
                "Saved rider diverged {name}: {:?} {:?}",
                p.feet,
                q.feet
            );
        }
        println!("PASS Hedge live rider {name}, clear hull and exact mid-ride disk state");
    }
    // The drop-trap door excludes Alice, opens for an actor, slides down 240 and stays open.
    let mut t = isolated(map, "t28")?;
    let mut w = World::fixture(&[]);
    let mut p = Player::new(t.objects[0].base + vec3(-48., 0., -80.));
    for _ in 0..60 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
    }
    ensure!(
        t.saved.machines[0].door == 0.,
        "Monster-only trap opened for Alice"
    );
    t.actor_contacts(&[Target {
        id: 1,
        center: t.objects[0].base + vec3(40., 0., 0.),
        half: vec3(15., 15., 28.),
    }]);
    for _ in 0..360 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
    }
    t.actor_contacts(&[]);
    for _ in 0..480 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
    }
    ensure!(
        t.saved.machines[0].door == 1.
            && t.saved.machines[0].latched
            && (t.objects[0].pose.translation.z - t.objects[0].base.z + 240.).abs() < 0.01,
        "Trap travel/latch differs"
    );
    // The paired main doors must wake together even from one end of the doorway.
    let mut t = Labyrinth::load(map)?;
    let pair = t
        .objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.id == 55 || o.id == 56)
        .map(|(k, _)| k)
        .collect::<Vec<_>>();
    ensure!(
        t.objects[pair[0]].bounds == t.objects[pair[1]].bounds,
        "Door leaves have separate triggers"
    );
    let lo = t.objects[pair[0]].bounds.0;
    let mut p = Player::new(lo + vec3(-45., -40., 0.));
    let mut w = World::fixture(&[]);
    for _ in 0..60 {
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
    }
    ensure!(
        pair.iter().all(|k| t.saved.machines[*k].door > 0.),
        "Only one door leaf opened"
    );
    let snap = t.snapshot();
    let mut stats = crate::inventory::Stats::for_level("hedge3", None);
    stats.watch().map_err(anyhow::Error::msg)?;
    t.advance(stats.powers.world_dt(FIXED_DT), map, &mut w, &mut p, &[])?;
    ensure!(t.snapshot() == snap, "Watch did not freeze Hedge machinery");
    let mut invalid = snap.clone();
    invalid["machines"][0]["door"] = serde_json::json!(2.);
    ensure!(
        t.restore(&invalid, map).is_err() && t.snapshot() == snap,
        "Invalid save mutated machinery"
    );
    println!(
        "PASS Hedge paired/monster doors, full travel, latch, Watch and rejected invalid save"
    );
    // A slam pushes a free Alice, but cannot crush through a solid floor unnoticed.
    for floor in [false, true] {
        let mut t = isolated(map, "pendulum01")?;
        t.saved.machines[0].time = 4.;
        t.rebuild();
        let bottom = t.objects[0].base.z - 151. + 272.;
        let mut p =
            Player::new(t.objects[0].base + vec3(0., 0., bottom - t.objects[0].base.z - 56. - 1.));
        let mut w = World::fixture(&if floor {
            vec![(
                p.feet - vec3(500., 500., 20.),
                p.feet + vec3(500., 500., -0.04),
            )]
        } else {
            vec![]
        });
        let start = p.feet;
        for _ in 0..30 {
            t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
            if t.saved.crush {
                break;
            }
        }
        ensure!(
            t.saved.crush == floor,
            "Crusher contact damage ignored blocked/free distinction"
        );
        if !floor {
            ensure!(p.feet.z < start.z - 40., "Free player not pushed by slam");
        }
    }
    println!("PASS Hedge slam: free displacement, blocked crush, no silent interpenetration");
    Ok(())
}

fn sky_triggers(a: &mut Assets, map: &Bsp) -> Result<()> {
    let mut count = 0;
    for e in &map.entities {
        let Some(thread) = e.get("thread").filter(|s| s.starts_with("change_to_sky")) else {
            continue;
        };
        let mut i = Interactions::load(map)?;
        i.set_entry(a, map, "hedge3", None)?;
        let p = interaction::vector(&e["origin"]).context("Sky trigger without origin")?
            - PLAYER_CENTER;
        i.triggers(FIXED_DT, p, p);
        ensure!(
            i.presentation.state.sky.as_deref()
                == Some(&format!(
                    "sky_camera{}",
                    thread.trim_start_matches("change_to_sky")
                )),
            "Authored {thread} volume did not select its sky"
        );
        count += 1;
    }
    ensure!(count >= 20, "Missing sky volumes");
    println!("PASS {count} authored Hedge sky volumes, including selections 4-7");
    Ok(())
}

fn door_audio(map: &Bsp) -> Result<()> {
    let mut t = Labyrinth::load(map)?;
    t.objects.retain(|o| o.id == 6 || o.id == 7);
    t.saved.machines = vec![Mechanism::default(); 2];
    t.rebuild();
    let mut w = World::fixture(&[]);
    let mut p = Player::new(vec3(6656., -2460., 1280.));
    t.actor_contacts(&[Target {
        id: 1,
        center: p.feet + PLAYER_CENTER,
        half: PLAYER_HALF,
    }]);
    let mut absent = p.clone();
    absent.feet.x -= 1000.;
    for _ in 0..120 {
        t.advance(FIXED_DT, map, &mut w, &mut absent, &[])?;
    }
    ensure!(
        t.saved.machines.iter().all(|m| m.door == 0.),
        "NOT_MONSTERS door opened for actor"
    );
    t.actor_contacts(&[]);
    let mut audio = crate::audio::world::State::default();
    let mut cues = vec![];
    for n in 0..720 {
        if n == 180 {
            p.feet.x -= 1000.;
        }
        t.advance(FIXED_DT, map, &mut w, &mut p, &[])?;
        let mut loops = vec![];
        let mut clocks = vec![];
        t.sound_state(&mut loops, &mut clocks);
        if n == 30 {
            ensure!(loops.len() == 1, "Missing authored door movement loop");
        }
        cues.extend(audio.update(clocks));
    }
    ensure!(
        cues.len() == 2 && cues.iter().all(|c| c.0.ends_with("metal_door_end.wav")),
        "Door endpoints silent or repeated"
    );
    let mut clocks = vec![];
    t.sound_state(&mut vec![], &mut clocks);
    ensure!(
        crate::audio::world::State::default()
            .update(clocks)
            .is_empty(),
        "Saved door replayed stop"
    );
    ensure!(
        t.saved.machines.iter().all(|m| m.door == 0.),
        "Door did not close after vacancy"
    );
    println!("PASS Hedge player-only doors, vacant close, movement/stop sound and no load replay");
    Ok(())
}

fn pump_windows(map: &Bsp) -> Result<()> {
    let mut t = Labyrinth::load(map)?;
    let mut w = World::from_bsp(map)?;
    let mut found = 0;
    for step in 0..80 {
        for s in &mut t.saved.machines {
            s.time = step as f64 / 10.;
        }
        t.rebuild();
        w.set_dynamic(t.colliders());
        for (pump, shift) in [(1, 0.), (2, 592.)] {
            for x in [4544., 4560., 4580., 4600., 4630., 4660., 4670.] {
                for z in [48.1, 80., 112., 144.] {
                    let tr = w.body_trace(vec3(x, -2440. + shift, z), vec3(x, -2200. + shift, z));
                    if !tr.start_solid && tr.fraction == 1. {
                        println!(
                            "HEDGE pump{pump} window time {} x {x} feet {z}",
                            step as f32 / 10.
                        );
                        found += 1;
                    }
                }
            }
        }
    }
    ensure!(found > 0, "No traversable pump window");
    Ok(())
}

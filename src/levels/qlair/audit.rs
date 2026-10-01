//! Asset-backed regression checks for findings from the content-completeness audit.
//! These staged attack probes are not a combat-completion claim.
use super::*;
use battle::Attack;
use std::collections::BTreeSet;

pub(super) fn check(a: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&a.read("maps/qlair.bsp")?)?;
    let world = World::from_bsp(&map)?;
    let mut q = Queen::load(a, &map)?;
    let base = q.saved.clone();
    beam_directions(&q)?;
    physics(&mut q, &map)?;
    let mut stats = Stats::for_level("qlair", None);
    let mut story = Story::load(a, "qlair");
    for hz in [30, 60, 144] {
        for escape in [false, true] {
            q.saved = base.clone();
            q.saved.phase = Phase::Queen1;
            q.saved.attack = Attack::Grab;
            let mut player = Player::new(q.data.points["alice_start_pos1"].translation);
            let empty = World::fixture(&[]);
            let mut health = Stats::for_level("qlair", None);
            health.set_health(10.)?;
            let mut captured = false;
            let mut restored = false;
            for tick in 0..hz * 20 {
                let dt = 1. / hz as f32;
                q.saved.clock += dt;
                if escape
                    && q.saved.attack_time > q.tele_prep() + 0.5
                    && q.saved.attack_time < q.tele_prep() + 1.0
                {
                    player.feet.x = 400.;
                }
                let feedback = q.battle(&mut Combat {
                    dt,
                    world: &empty,
                    player: &mut player,
                    stats: &mut health,
                    story: &mut story,
                    notarget: false,
                    summon: None,
                    threatens: &|_| false,
                });
                health.damage(feedback.damage);
                captured |= q.saved.grabbed;
                if q.saved.grab_arrived && !restored {
                    let snapshot = q.snapshot();
                    q.restore(&snapshot, &map)?;
                    ensure!(
                        q.snapshot() == snapshot,
                        "Grab camera/clock changed on restore"
                    );
                    restored = true;
                }
                if q.saved.attack == Attack::Idle && tick > 0 {
                    break;
                }
            }
            ensure!(
                q.saved.attack == Attack::Idle && !q.controlled(),
                "Grab failed to release Alice at {hz} Hz"
            );
            ensure!(
                captured != escape && health.alive(),
                "Grab escape/survival failed at {hz} Hz"
            );
            if !escape {
                ensure!(
                    restored && health.sanity() >= 1. && health.sanity() < 10.,
                    "Squeeze did not pulse or preserve minimum health"
                );
            }
        }
    }
    println!("PASS telekinetic escape, squeeze survival, saved camera and release at 30/60/144 Hz");
    let mut all = BTreeSet::new();
    for (phase, attack, variant) in [
        (Phase::Queen1, Attack::Spit, 0),
        (Phase::Queen1, Attack::Combo, 0),
        (Phase::Queen1, Attack::Sweep, 0),
        (Phase::Queen1, Attack::Ice, 0),
        (Phase::Queen2, Attack::Claw, 0),
        (Phase::Queen2, Attack::Club, 0),
        (Phase::Queen2, Attack::Centipede, 0),
        (Phase::Queen2, Attack::JabberEye, 0),
        (Phase::Queen2, Attack::JabberSpit, 0),
        (Phase::Queen2, Attack::Hatter, 0),
        (Phase::Queen2, Attack::Spore, 0),
        (Phase::Queen2, Attack::Spore, 1),
        (Phase::Queen2, Attack::Scream, 0),
        (Phase::Queen2, Attack::Scream, 1),
    ] {
        q.saved = base.clone();
        q.saved.phase = phase;
        q.saved.time = 10.;
        q.saved.attack = attack;
        q.saved.attack_variant = variant;
        if let Some(n) = Queen::attack_part(attack) {
            q.saved.part_attack[n] = Some(attack);
        }
        q.saved.platform = -4;
        let mut player = Player::new(
            q.data.points[if phase == Phase::Queen1 {
                "alice_start_pos1"
            } else {
                "alice_fight_queen"
            }]
            .translation,
        );
        q.saved.attack_target = player.eye();
        let mut seen = BTreeSet::new();
        for _ in 0..6000 {
            q.battle(&mut Combat {
                dt: crate::movement::FIXED_DT,
                world: &world,
                player: &mut player,
                stats: &mut stats,
                story: &mut story,
                notarget: false,
                summon: None,
                threatens: &|_| false,
            });
            for shot in &q.saved.projectiles {
                seen.insert(shot.model_key().to_string());
                if shot.model_key() == "fx_queen_wave" {
                    ensure!(shot.is_scream_wave(), "Live wave cannot reach its renderer");
                    let mut old = shot.clone();
                    old.model.push_str(".tik");
                    ensure!(
                        old.is_scream_wave(),
                        "Legacy wave name cannot reach its renderer"
                    );
                }
            }
            if q.saved.attack == Attack::Idle && q.saved.part_attack.iter().all(Option::is_none) {
                break;
            }
        }
        ensure!(
            q.saved.attack == Attack::Idle,
            "Attack failed to recover: {attack:?}/{variant}"
        );
        if !matches!(attack, Attack::Claw | Attack::Club) {
            ensure!(
                !seen.is_empty(),
                "Attack emitted nothing: {attack:?}/{variant}"
            );
        }
        if attack == Attack::Spore && variant == 1 {
            ensure!(
                seen.contains("prj_diamond"),
                "Diamond variant is unreachable"
            );
        }
        if attack == Attack::Scream && variant == 1 {
            ensure!(
                seen.contains("prj_spade") && seen.contains("prj_heart"),
                "Card variant is unreachable"
            );
        }
        println!(
            "PASS staged {attack:?}/{variant}: {} projectile types and recovery",
            seen.len()
        );
        all.extend(seen);
    }
    for name in q.data.projectiles.keys() {
        ensure!(
            all.contains(name.trim_end_matches(".tik")),
            "Loaded but never emitted projectile: {name}"
        );
    }
    q.saved = base.clone();
    q.saved.phase = Phase::Queen2;
    q.saved.queen1 = 0.;
    q.saved.powered = true;
    q.saved.checkpoint = true;
    for n in 0..4 {
        let hit = |damage| Hit {
            id: BASE + 2 + n,
            damage,
            kind: crate::combat::DamageKind::Knife,
            knockback: Vec3::ZERO,
        };
        let before = q.saved.parts[n];
        q.hit(hit(before - 1030.));
        ensure!(
            q.saved.parts[n] == 1030.,
            "Tentacle collapsed at the inclusive boundary"
        );
        q.hit(hit(1.));
        ensure!(
            q.saved.parts[n] == 25. && q.part_animation(n).0 == "death",
            "Tentacle collapse threshold was missed"
        );
        q.saved.part_death[n] = 0.5;
        let saved = q.snapshot();
        q.restore(&saved, &map)?;
        ensure!(
            q.saved.parts[n] == 25. && q.part_animation(n) == ("death", 0.5),
            "Collapsed tentacle did not resume"
        );
        q.hit(hit(25.));
        ensure!(
            !q.targets().iter().any(|t| t.id == BASE + 2 + n),
            "Destroyed tentacle retained hit boxes"
        );
    }
    q.saved.attack = Attack::Hatter;
    ensure!(q.halo_performance().0 == "stiff", "Attack halo is idle");
    q.saved.attack = Attack::Scream;
    ensure!(q.halo_performance().0 == "strobe", "Scream halo is idle");
    q.saved.attack = Attack::Idle;
    ensure!(q.halo_performance().0 == "idle", "Idle halo stays on fire");
    let mut legacy = serde_json::to_value(&q.saved)?;
    legacy.as_object_mut().unwrap().remove("attack_variant");
    let legacy: Saved = serde_json::from_value(legacy)?;
    ensure!(
        legacy.attack_variant == 0,
        "Previous finale saves lost compatibility"
    );
    q.saved.parts[0] = 500.;
    let legacy = q.snapshot();
    q.restore(&legacy, &map)?;
    ensure!(
        q.saved.parts[0] == 25. && q.part_animation(0).0 == "death",
        "Old sub-threshold fighting tentacle was not migrated"
    );
    println!("PASS audit: all 10 projectile types emitted; both slit variants; live/legacy wave keys; four tentacle collapse, finish and restore transitions; legacy save migration; halo attack states");
    for _ in 0..200 {
        ensure!(
            !matches!(
                q.choose_attack(false, 1500., true),
                Attack::Claw | Attack::Club
            ),
            "Long-range melee selected"
        );
        ensure!(
            !matches!(
                q.choose_attack(true, 500., false),
                Attack::Popup | Attack::Spit | Attack::Sweep | Attack::Combo | Attack::Ice
            ),
            "Occluded sight attack selected"
        );
    }
    q.saved.phase = Phase::Queen2;
    q.saved.parts = base.parts;
    q.saved.attack = Attack::Hatter;
    q.saved.attack_time = 1.;
    ensure!(!q.actor_lights().is_empty(), "Attack crown lights missing");
    let growth = q.data.rigs["c_q2_t02"].scales("grow_centipede", 0.);
    for (bone, _) in &growth {
        ensure!(
            q.data.rigs["c_q2_t02"]
                .skeleton
                .bones
                .iter()
                .any(|b| b.name.eq_ignore_ascii_case(bone)),
            "Unbound head scale controller {bone}"
        );
    }
    ensure!(
        growth.iter().any(|(_, scale)| *scale == 0.),
        "Head growth scale track missing"
    );
    q.saved.debris.clear();
    q.final_burst();
    let count = q.saved.debris.len();
    q.final_burst();
    ensure!(
        count >= 61 && q.saved.debris.len() == count,
        "Final destruction missing or duplicated"
    );
    let saved = q.snapshot();
    q.restore(&saved, &map)?;
    q.final_burst();
    ensure!(
        q.saved.debris.len() == count,
        "Reload replayed final destruction"
    );
    q.saved = base.clone();
    q.saved.phase = Phase::Death;
    q.saved.queen1 = 0.;
    q.saved.queen2 = 900.;
    q.saved.powered = true;
    q.saved.checkpoint = true;
    q.saved.time = q.data.rigs["c_q2_body"].duration("death_start") + 7.;
    q.death_explosions();
    ensure!(
        q.saved.death_effects == 63 && !q.saved.debris.is_empty(),
        "Intermediate death explosions missing"
    );
    let snapshot = q.snapshot();
    q.restore(&snapshot, &map)?;
    q.death_explosions();
    ensure!(q.snapshot() == snapshot, "Saved death explosions replayed");
    println!("PASS gap closure: range/visibility gates, attached lights, head growth, final debris and once-only restore");
    Ok(())
}

fn beam_directions(q: &Queen) -> Result<()> {
    let source = Transform {
        translation: vec3(100., 40., 500.),
        rotation: Quat::from_rotation_z(0.7),
    };
    let target = vec3(-300., 200., 48.);
    let aimed = battle::beam_end(source, target, 2000., true);
    ensure!(
        aimed.distance(target) < 0.001,
        "Tracking Queen beam misses its target"
    );
    let swept = battle::beam_end(source, target, 2000., false) - source.translation;
    ensure!(
        swept.z < 0. && (swept.length() - 2000.).abs() < 0.01,
        "Sweeping Queen beam ignores target elevation/range"
    );
    ensure!(
        swept
            .truncate()
            .normalize()
            .dot((source.rotation * Vec3::X).truncate().normalize())
            > 0.999,
        "Sweeping Queen beam lost the animation's horizontal direction"
    );
    let shaders = q
        .data
        .rigs
        .values()
        .flat_map(|rig| rig.attacks.values().flatten())
        .filter(|(_, row)| row[0] == "beamattack")
        .map(|(_, row)| row[6].as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        shaders == BTreeSet::from(["queenbeam1", "queenbeam2", "queenbeam3", "queenbeam4"]),
        "Queen beam shader coverage changed: {shaders:?}"
    );
    println!("PASS Queen tracking target, animated sweep direction/elevation and all four beam materials");
    Ok(())
}

fn physics(q: &mut Queen, map: &Bsp) -> Result<()> {
    let base = q.saved.clone();
    let shot = |at, velocity| battle::Shot {
        model: "prj_heart".into(),
        at,
        velocity,
        age: 0.,
        damage: 20.,
        force: 150.,
        life: 5.,
        end: None,
        seek_at: projectile::first_seek(),
        victim: Some(crate::dice::ALICE),
        trail: Vec::new(),
    };
    let body = |at| Target {
        id: crate::dice::ALICE,
        center: at,
        half: Vec3::ONE,
    };
    let empty = World::fixture(&[]);
    let mut endpoints = Vec::new();
    for hz in [30, 60, 144] {
        q.saved = base.clone();
        q.saved.projectiles = vec![shot(Vec3::ZERO, Vec3::X * 850.)];
        for _ in 0..hz {
            q.projectiles(
                1. / hz as f32,
                &[body(vec3(2000., 2000., 0.))],
                &empty,
                &mut Feedback::default(),
            );
        }
        endpoints.push(q.saved.projectiles[0].at);
    }
    ensure!(
        endpoints.iter().all(|p| p.distance(endpoints[0]) < 0.05),
        "Seeking depends on frame rate"
    );
    q.saved = base.clone();
    let mut feedback = Feedback::default();
    q.saved.projectiles = vec![shot(Vec3::ZERO, Vec3::X * 850.)];
    let moving = body(vec3(1000., 1000., 0.));
    for _ in 0..30 {
        q.projectiles(1. / 120., &[moving], &empty, &mut feedback);
    }
    ensure!(
        q.saved.projectiles[0].velocity.y > 0.,
        "Heart does not seek a moving target"
    );
    let checkpoint = q.snapshot();
    for _ in 0..30 {
        q.projectiles(1. / 120., &[moving], &empty, &mut feedback);
    }
    let expected = q.saved.projectiles[0].at;
    q.restore(&checkpoint, map)?;
    for _ in 0..30 {
        q.projectiles(1. / 120., &[moving], &empty, &mut feedback);
    }
    ensure!(
        q.saved.projectiles[0].at.distance(expected) < 0.001,
        "Seeking changed after save/load"
    );
    q.saved = base.clone();
    q.saved.projectiles = vec![shot(Vec3::ZERO, Vec3::X * 850.)];
    let mut feedback = Feedback::default();
    q.projectiles(0.1, &[body(vec3(50., 0., 0.))], &empty, &mut feedback);
    ensure!(
        feedback.damage == 20. && q.saved.projectiles.is_empty(),
        "Direct hit received duplicate splash"
    );
    q.projectiles(0.1, &[body(vec3(50., 0., 0.))], &empty, &mut feedback);
    ensure!(
        feedback.damage == 20. && q.saved.impacts.len() == 1,
        "Impact damage/effect repeated"
    );
    let wall = World::fixture(&[(vec3(40., -100., -100.), vec3(42., 100., 100.))]);
    q.saved = base.clone();
    q.saved.projectiles = vec![shot(Vec3::ZERO, Vec3::X * 850.)];
    let mut feedback = Feedback::default();
    q.projectiles(0.1, &[body(vec3(80., 0., 0.))], &wall, &mut feedback);
    ensure!(
        feedback.damage == 0. && q.saved.impacts.len() == 1,
        "Projectile or outer splash passed through wall"
    );
    let wall = World::fixture(&[(vec3(90., -100., -100.), vec3(92., 100., 100.))]);
    q.saved = base.clone();
    q.saved.projectiles = vec![shot(Vec3::ZERO, Vec3::X * 850.)];
    let mut feedback = Feedback::default();
    q.projectiles(0.15, &[body(vec3(50., 50., 0.))], &wall, &mut feedback);
    ensure!(
        feedback.damage > 0. && feedback.damage < 50.,
        "Nearby impact has no splash falloff"
    );
    q.saved = base;
    println!("PASS projectile physics: seeking and exact saved continuation, direct-hit exclusion, once-only impact, world occlusion and splash falloff");
    Ok(())
}

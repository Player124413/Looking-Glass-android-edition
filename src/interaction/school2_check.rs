//! Real-map encounter ownership, activation, damage and migration probes.
use super::*;
use crate::{
    assets::Assets,
    combat::{self, DamageKind, Hit},
    encounters::{Enemy, BASE},
    powerups::Difficulty,
    school2_quest::Stage,
};
use anyhow::{ensure, Context};

fn load(assets: &mut Assets, map: &Bsp) -> Result<Interactions> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "skool2", None)?;
    Ok(i)
}
fn index(i: &Interactions, id: usize) -> usize {
    i.encounters
        .as_ref()
        .unwrap()
        .identities
        .iter()
        .position(|a| *a == Id(id))
        .unwrap()
}
fn actor(i: &Interactions, id: usize) -> &crate::encounters::Actor {
    &i.encounters.as_ref().unwrap().actors[index(i, id)]
}
fn health(i: &Interactions, id: usize) -> f32 {
    match &actor(i, id).enemy {
        Enemy::Guard(g) => g.health,
        Enemy::Boojum(b) => b.health,
        _ => unreachable!(),
    }
}
fn shot(i: &mut Interactions, id: usize, damage: f32) {
    let id = BASE + index(i, id);
    i.encounters.as_mut().unwrap().hit(Hit {
        id,
        damage,
        kind: DamageKind::Cards,
        knockback: Vec3::ZERO,
    });
}
fn roundtrip(assets: &mut Assets, map: &Bsp, i: &Interactions) -> Result<Interactions> {
    let s = i.snapshot();
    let mut fresh = load(assets, map)?;
    fresh.restore(&serde_json::from_slice(&serde_json::to_vec(&s)?)?, map)?;
    ensure!(
        serde_json::to_value(&s)? == serde_json::to_value(fresh.snapshot())?,
        "School2 save changed state"
    );
    Ok(fresh)
}

pub fn check(assets: &mut Assets) -> Result<()> {
    vial_hint(assets)?;
    for difficulty in Difficulty::ALL {
        let mut map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
        map.difficulty = difficulty;
        let mut i = load(assets, &map)?;
        let enemies = i.encounters.as_ref().unwrap();
        let mut expected = vec![6, 25, 37, 48, 65, 68, 69, 70, 595];
        if difficulty != Difficulty::Easy {
            expected.push(202);
        }
        expected.sort();
        ensure!(
            enemies.identities.iter().map(|id| id.0).collect::<Vec<_>>() == expected,
            "School2 ownership/difficulty mismatch"
        );
        ensure!(
            enemies.targets().len() == 6,
            "Fresh School2 must expose six Diamonds"
        );
        ensure!(
            i.school2.as_ref().unwrap().boojums.len() == 10,
            "Quest actor slots changed"
        );
        for id in [6, 68, 69] {
            ensure!(!actor(&i, id).active, "Spawner woke without activation");
            let hp = health(&i, id);
            shot(&mut i, id, 7.);
            ensure!(health(&i, id) == hp, "Dormant spawner can be hit");
        }
        for id in [68, 69] {
            i.dispatch(Event::Entity(Id(id), Input::Activate));
            ensure!(
                actor(&i, id).active,
                "Explicit floor-spawner receiver absent"
            );
            shot(&mut i, id, 7.);
            let hp = health(&i, id);
            i = roundtrip(assets, &map, &i)?;
            i.dispatch(Event::Entity(Id(id), Input::Activate));
            ensure!(health(&i, id) == hp, "Repeated floor spawn reset damage");
        }
        let mut story = crate::story::Story::load(assets, "skool2");
        ensure!(story.trigger("dice_cat"), "Dice conversation unavailable");
        i.activate_enemies();
        ensure!(
            !i.school2.as_ref().unwrap().boojums[5].active,
            "Dice ambush released before conversation"
        );
        for _ in 0..6000 {
            story.tick(1. / 60., false);
            for name in story.take_completed() {
                i.completed_dialogue(&name);
            }
            if !story.busy() {
                break;
            }
        }
        ensure!(!story.busy(), "Dice conversation never completed");
        i.activate_enemies();
        ensure!(
            i.school2.as_ref().unwrap().boojums[5].active,
            "Legacy dice Boojum lost"
        );
        if difficulty != Difficulty::Easy {
            ensure!(actor(&i, 202).active, "Second dice Boojum absent");
            shot(&mut i, 202, 7.);
            i = roundtrip(assets, &map, &i)?;
            shot(&mut i, 202, 1000.);
            i.activate_enemies();
            i = roundtrip(assets, &map, &i)?;
            ensure!(
                health(&i, 202) == 0.,
                "Dice callback resurrected dead Boojum"
            );
        }
        // Exercise an actual placed Diamond's ranged attack against Alice's
        // body, and the real swept weapon contact back through its hit owner.
        let mut fight = load(assets, &map)?;
        let mut world = World::from_bsp(&map)?;
        fight.sync(&mut world);
        let eye = vec3(800., -2560., 48.);
        let mut damage = 0.;
        let mut projectile = false;
        for _ in 0..1440 {
            let f = fight
                .encounters
                .as_mut()
                .unwrap()
                .update(1. / 120., &world, eye);
            damage += f.damage;
            if let Enemy::Guard(g) = &actor(&fight, 25).enemy {
                projectile |= !g.shots.is_empty();
            }
        }
        ensure!(
            damage > 0. && projectile,
            "Diamond did not shoot/hurt Alice"
        );
        let targets = fight.encounters.as_ref().unwrap().targets();
        let id = BASE + index(&fight, 25);
        let target = targets
            .iter()
            .find(|t| t.id == id)
            .context("Diamond target absent")?;
        let contact = combat::contact(
            &combat::Context {
                world: &world,
                targets: &targets,
            },
            eye,
            target.center,
            1.,
        );
        ensure!(
            contact.is_some_and(|(hit, _)| hit == id),
            "Diamond not reachable by swept weapon"
        );
        let before = health(&fight, 25);
        shot(&mut fight, 25, combat::weapon_damage(1, false));
        ensure!(
            health(&fight, 25) < before && health(&fight, 25) > 0.,
            "Card damage lost"
        );
        fight = roundtrip(assets, &map, &fight)?;
        while health(&fight, 25) > 0. {
            shot(&mut fight, 25, combat::weapon_damage(0, true));
        }
        fight = roundtrip(assets, &map, &fight)?;
        fight.dispatch(Event::Entity(Id(25), Input::Activate));
        ensure!(
            health(&fight, 25) == 0.
                && !fight
                    .encounters
                    .as_ref()
                    .unwrap()
                    .targets()
                    .iter()
                    .any(|t| t.id == id),
            "Dead Diamond revived"
        );
        println!("PASS School2 {:?}: {:?}; ranged damage {}, swept hits, dead/partial saves, dormant floor receivers", difficulty, expected, damage);
        migration(assets, &map)?;
        if difficulty == Difficulty::Normal {
            for case in [
                "school2-enemy-live",
                "school2-enemy-bolt",
                "school2-enemy-dice-live",
                "school2-enemy-growth",
            ] {
                let mut player = Player::new(Vec3::ZERO);
                let staged = stage(case, assets, &map, &mut world, &mut player)?;
                roundtrip(assets, &map, &staged)?;
            }
        }
    }
    Ok(())
}

fn vial_hint(assets: &mut Assets) -> Result<()> {
    const HINT: &str = crate::school2::rage_hint::EVENT;
    for difficulty in Difficulty::ALL {
        let mut map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
        map.difficulty = difficulty;
        let mut i = load(assets, &map)?;
        let rejected = if difficulty == Difficulty::Easy { 32 } else { 783 };
        let absent = i.triggers.iter().find(|t| t.id == Id(rejected)).unwrap();
        let away = (absent.bounds.0 + absent.bounds.1) * 0.5 - PLAYER_CENTER;
        ensure!(i.triggers(1. / 60., away, away).story.is_empty(), "Wrong-difficulty vial hint fired");
        let t = i.triggers.iter().find(|t| t.id != Id(rejected) && matches!(&t.kind, TriggerKind::Script(s) if s == HINT))
            .context("Vial hint difficulty contact missing")?;
        let id = t.id;
        let feet = (t.bounds.0 + t.bounds.1) * 0.5 - PLAYER_CENTER;
        let quest = serde_json::to_value(i.school2.as_ref().unwrap().quest.clone())?;
        let before = i.snapshot();
        ensure!(i.triggers(0., feet, feet).story.is_empty(), "Paused vial contact fired");
        let e = i.triggers(1. / 60., feet, feet);
        ensure!(e.story == [HINT] && !i.scripted(), "Vial hint missing or takes camera control");
        let world = World::from_bsp(&map)?;
        let mut story = crate::story::Story::load(assets, "skool2");
        let hints = crate::cheshire::Hints::load(assets, &map, "skool2")?;
        ensure!(story.trigger(HINT), "Missing registered vial voice");
        let mut cues = Vec::new();
        for n in 0..600 {
            story.tick(1. / 60., n == 70);
            i.sync_cinematic_story(&story);
            let (_, f) = i.school2.as_mut().unwrap().update(1. / 60., &world, feet, true);
            cues.extend(f.spatial_sounds.into_iter().map(|(s, _)| s));
            if n == 30 || n == 100 || n == 240 {
                i = roundtrip(assets, &map, &i)?;
                let saved = story.snapshot();
                story.restore(&saved, &hints)?;
            }
        }
        ensure!(cues == ["sound/character/cheshire_cat/appear.wav", "sound/character/cheshire_cat/disappear.wav"],
            "Vial appearance sound replay/missing cue");
        ensure!(i.school2.as_mut().unwrap().event(HINT).unwrap().story.is_empty(), "Vial hint replayed");
        ensure!(serde_json::to_value(&i.school2.as_ref().unwrap().quest)? == quest,
            "Vial hint changed school ingredients or progression");
        // The former pending contact was consumed. Rearm only that failed contact.
        let mut old = before;
        let trigger = old.triggers.iter_mut().find(|t| t.id == Some(id)).unwrap();
        trigger.fired = true;
        trigger.reported = true;
        let mut migrated = load(assets, &map)?;
        migrated.restore(&old, &map)?;
        ensure!(migrated.triggers(1. / 60., feet, feet).story == [HINT], "Pending vial contact not migrated");
        println!("PASS school vial hint {:?}: real contact {}, voice, appearance cues, saved clocks, one-shot and pending migration", difficulty, id.0);
    }
    Ok(())
}

fn migration(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for stage in [
        Stage::Explore,
        Stage::Battle,
        Stage::Rescue,
        Stage::Jumbogrow,
        Stage::Growing,
        Stage::Lollipop,
        Stage::Mixing,
        Stage::Rewards,
        Stage::Complete,
    ] {
        let mut old = load(assets, map)?;
        old.encounters = None;
        old.configure_events(map)?;
        let school = old.school2.as_mut().unwrap();
        school.quest.enter(stage);
        school.boojums[0].hit(1000.);
        school.guards[0].hurt(20.);
        if stage != Stage::Explore {
            school.event("dice_cat");
        }
        let saved = old.snapshot();
        let mut new = load(assets, map)?;
        new.restore(&saved, map)?;
        ensure!(
            serde_json::to_value(&saved.school2)? == serde_json::to_value(new.snapshot().school2)?,
            "Migration changed School2 quest/combat"
        );
        let current = new.event_world.snapshot();
        for (key, usage) in &saved.shared.as_ref().unwrap().usage {
            ensure!(
                serde_json::to_value(usage)? == serde_json::to_value(&current.usage[key])?,
                "Migration changed old rule {key}"
            );
        }
        let growth = matches!(
            stage,
            Stage::Growing | Stage::Lollipop | Stage::Mixing | Stage::Rewards | Stage::Complete
        );
        ensure!(
            actor(&new, 6).active == growth,
            "Migration lost growth activation"
        );
        ensure!(
            actor(&new, 48).enabled == (!growth || stage == Stage::Growing),
            "Migration lost authored removal"
        );
        ensure!(
            !actor(&new, 68).active && !actor(&new, 69).active,
            "Migration invented floor encounter"
        );
        let migrated = new.snapshot();
        new.activate_enemies();
        ensure!(
            serde_json::to_value(migrated)? == serde_json::to_value(new.snapshot())?,
            "Migration not idempotent"
        );
        roundtrip(assets, map, &new)?;
    }
    println!("PASS School2 migration: nine quest phases, retained old rules/kill gates/resources, activation/removal, idempotent current saves");
    Ok(())
}

/// Native writer fixtures also provide reproducible live Diamond and Boojum
/// scenes for real input. These staged fixtures are not traversal proof.
pub fn stage(
    case: &str,
    assets: &mut Assets,
    map: &Bsp,
    world: &mut World,
    player: &mut Player,
) -> Result<Interactions> {
    let mut i = load(assets, map)?;
    i.sync(world);
    *player = Player::spawn(world, vec3(800., -2560., 8.)).context("Diamond fixture obstructed")?;
    if case.contains("dice") {
        *player =
            Player::spawn(world, vec3(-1100., -3020., -328.)).context("Dice fixture obstructed")?;
        if !case.ends_with("before") && !case.ends_with("dialogue") {
            i.school2.as_mut().unwrap().event("dice_cat");
        }
    }
    if case.contains("growth") {
        i.school2.as_mut().unwrap().quest.enter(Stage::Lollipop);
    }
    i.activate_enemies();
    if case.ends_with("hurt") {
        shot(&mut i, 25, 7.);
    }
    if case.ends_with("dead") {
        shot(&mut i, if case.contains("dice") { 202 } else { 25 }, 1000.);
    }
    if case.ends_with("bolt") {
        for _ in 0..1200 {
            i.encounters
                .as_mut()
                .unwrap()
                .update(1. / 120., world, player.eye());
            if let Enemy::Guard(g) = &actor(&i, 25).enemy {
                if !g.shots.is_empty() {
                    break;
                }
            }
        }
        ensure!(
            matches!(&actor(&i,25).enemy,Enemy::Guard(g) if !g.shots.is_empty()),
            "No saved Diamond projectile"
        );
    }
    Ok(i)
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "skool2")?;
    let mut npcs = crate::npc::Npcs::load(assets, &scene.map, "skool2", None, false, false)?;
    npcs.check_school2_ownership()?;
    let mut enemies = crate::encounters::Art::load(assets)?;
    for case in [
        "live",
        "bolt",
        "dead",
        "dice-before",
        "dice-live",
        "dice-dead",
        "growth",
    ] {
        let mut p = Player::new(Vec3::ZERO);
        let mut i = stage(
            &format!("school2-enemy-{case}"),
            assets,
            &scene.map,
            &mut scene.world,
            &mut p,
        )?;
        if case.ends_with("dead") {
            for _ in 0..360 {
                i.encounters
                    .as_mut()
                    .unwrap()
                    .update(1. / 120., &scene.world, p.eye());
            }
        }
        let school = i.school2.as_ref().unwrap();
        let mut art = crate::school2::Art::load(assets, school)?;
        let (eye, target) = if case.starts_with("dice") {
            (vec3(-1150., -3180., -240.), vec3(-1000., -2620., -180.))
        } else if case == "growth" {
            (vec3(-1300., -2740., 620.), vec3(-1104., -2568., 620.))
        } else {
            (p.eye(), vec3(1008., -2560., 40.))
        };
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            crate::render_fx::begin(
                eye,
                (target - eye).normalize(),
                0.,
                &scene.atmosphere,
                false,
            );
            let transforms = i.transforms();
            scene.draw(eye, 0., false, false, &transforms);
            art.draw(school, false, &scene.atmosphere, eye);
            npcs.draw(eye, (target - eye).normalize(), &scene.atmosphere, false);
            enemies.draw(
                i.encounters.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                eye,
            );
            crate::render::depth_read_only(|| scene.draw(eye, 0., false, true, &transforms));
            crate::render_fx::finish();
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school2-encounters/{case}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS School2 encounter render ownership: seven staged captures");
    Ok(())
}

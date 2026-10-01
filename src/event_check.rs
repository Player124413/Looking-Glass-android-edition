//! Real-school verification of shared identities, gates, dispatch and legacy migration.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::World,
    entity::Id,
    event::{Event, Input},
    interaction::{Interactions, Snapshot},
    movement::{Player, FIXED_DT},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;

fn reload(assets: &mut Assets, map: &Bsp, name: &str, snapshot: &Snapshot) -> Result<Interactions> {
    let bytes = serde_json::to_vec(snapshot)?;
    let mut restored = Interactions::load(map)?;
    restored.set_entry(assets, map, name, None)?;
    restored.restore(&serde_json::from_slice(&bytes)?, map)?;
    Ok(restored)
}
fn named(map: &Bsp, key: &str, name: &str) -> Result<Id> {
    map.entities
        .iter()
        .position(|e| e.get(key).is_some_and(|s| s == name))
        .map(Id)
        .context("School test entity missing")
}
fn activate(i: &mut Interactions, id: Id) {
    i.dispatch(Event::Entity(id, Input::Activate));
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "skool1", None)?;
    let trigger = named(&map, "targetname", "first_monster_trigger1")?;
    let feet =
        crate::interaction::vector(&map.entities[trigger.0]["origin"]).unwrap() - Vec3::Z * 28.;
    i.event_world.set_enabled(trigger, false)?;
    let saved = i.snapshot();
    let mut i = reload(assets, &map, "skool1", &saved)?;
    i.triggers(FIXED_DT, feet, feet);
    ensure!(
        !i.encounters.as_ref().unwrap().activated.contains("t188"),
        "Disabled saved trigger activated enemies"
    );
    i.event_world.set_enabled(trigger, true)?;
    i.triggers(FIXED_DT, feet, feet);
    let actors = i.encounters.as_ref().unwrap();
    let actor_index = actors.actors.iter().position(|a| a.name == "t188").unwrap();
    let actor_id = actors.identities[actor_index];
    let target_count = actors.targets().len();
    i.event_world.set_enabled(actor_id, false)?;
    let mut i = reload(assets, &map, "skool1", &i.snapshot())?;
    ensure!(
        i.encounters.as_ref().unwrap().targets().len() + 1 == target_count,
        "Disabled saved actor remained targetable"
    );
    i.event_world.set_enabled(actor_id, true)?;
    i.activate_enemies();
    ensure!(
        i.encounters.as_ref().unwrap().targets().len() == target_count,
        "Reenabled actor failed to resume"
    );
    let e = i.encounters.as_mut().unwrap();
    ensure!(
        e.actors
            .iter()
            .filter(|a| a.name == "t188" && a.active)
            .count()
            == 4,
        "Group activation did not fan out"
    );
    let dead = e.actors.iter_mut().find(|a| a.name == "t188").unwrap();
    match &mut dead.enemy {
        crate::encounters::Enemy::Guard(g) => {
            g.hurt(1000.);
        }
        crate::encounters::Enemy::Boojum(b) => {
            b.hit(1000.);
        }
        crate::encounters::Enemy::Ladybug(b) => {
            b.hit(1000.);
        }
    }
    let state = i.event_world.snapshot();
    i.reset_contacts();
    i.triggers(FIXED_DT, feet, feet);
    ensure!(
        i.event_world.snapshot().usage == state.usage,
        "Repeated touch consumed one-shot again"
    );
    let mut i = reload(assets, &map, "skool1", &i.snapshot())?;
    let frozen = i.encounters.as_ref().unwrap().snapshot();
    activate(&mut i, trigger);
    ensure!(
        serde_json::to_value(frozen)?
            == serde_json::to_value(i.encounters.as_ref().unwrap().snapshot())?,
        "Reload resurrected enemy group"
    );

    // Existing library condition: early touches do not consume a trigger, and only all four
    // completed flying-book paths permit the recipe. No new school progression is invented.
    let theatre = named(&map, "thread", "Theatre_Cinematic")?;
    let library = named(&map, "thread", "Skool1_OG_MoveShelf")?;
    let recipe = named(&map, "thread", "book_cinematic")?;
    activate(&mut i, library);
    ensure!(
        !i.school.as_ref().unwrap().event_facts().0["school.library"]
            .eq(&crate::event::Value::Flag(true)),
        "Library opened before theatre"
    );
    activate(&mut i, theatre);
    // Progression commits now follow a performance. Skip the presentation
    // through the public path; test delayed receiver dispatch separately below.
    let mut scene_world = World::from_bsp(&map)?;
    i.sync(&mut scene_world);
    let mut scene_player =
        Player::spawn(&scene_world, map.spawn().0).context("Scene event fixture spawn")?;
    let mut scene_story = crate::story::Story::load(assets, "skool1");
    i.skip_cinematic(&map, &mut scene_world, &mut scene_player, &mut scene_story)?;
    activate(&mut i, library);
    i.skip_cinematic(&map, &mut scene_world, &mut scene_player, &mut scene_story)?;
    ensure!(
        i.school.as_ref().unwrap().event_facts().0["school.library"]
            == crate::event::Value::Flag(true),
        "Library gate remained consumed/closed"
    );
    activate(&mut i, recipe);
    ensure!(
        !i.event_world
            .allowed(&Event::Entity(recipe, Input::Activate), &i.event_facts()),
        "Recipe accepted fewer than four books"
    );
    for name in ["start_book1", "start_book2", "start_book3", "start_book4"] {
        activate(&mut i, named(&map, "thread", name)?);
    }
    let mut world = World::from_bsp(&map)?;
    i.sync(&mut world);
    let mut p = Player::new(vec3(-1888., 1752., -504.));
    for _ in 0..1500 {
        i.advance_school(FIXED_DT, &map, &mut world, &mut p)?;
    }
    ensure!(
        i.event_world
            .allowed(&Event::Entity(recipe, Input::Activate), &i.event_facts()),
        "Four books failed shared counter gate"
    );

    // Save a real theatre completion in the future. Resuming must operate its actual doors
    // and both enemy receivers once, at the same time at different update rates.
    i.event_world
        .post(Event::DialogueFinished("Theatre_Cinematic".into()), 0.25)?;
    let pending = i.snapshot();
    for hz in [30, 60, 144] {
        let mut r = reload(assets, &map, "skool1", &pending)?;
        r.triggers(0., p.feet, p.feet);
        ensure!(
            r.event_world.snapshot() == i.event_world.snapshot(),
            "Paused event clock advanced"
        );
        for _ in 0..hz {
            r.triggers(1. / hz as f32, p.feet, p.feet);
        }
        ensure!(
            r.encounters
                .as_ref()
                .unwrap()
                .activated
                .contains("play_guard1")
                && r.encounters
                    .as_ref()
                    .unwrap()
                    .activated
                    .contains("play_guard2"),
            "Delayed theatre activation lost on reload"
        );
        let snapshot = serde_json::to_value(r.snapshot())?;
        let doors = snapshot["doors"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["name"].as_str().unwrap().starts_with("play_door"))
            .collect::<Vec<_>>();
        ensure!(
            !doors.is_empty()
                && doors
                    .iter()
                    .all(|d| d["locked"] == false && d["target"].as_f64() == Some(90.)),
            "Delayed theatre completion failed to unlock/open its doors"
        );
        let fired = r.event_world.snapshot();
        r.completed_dialogue("Theatre_Cinematic");
        ensure!(
            r.event_world.snapshot() == fired,
            "Theatre completion replayed"
        );
    }
    i.triggers(0.5, p.feet, p.feet);
    let mut legacy = serde_json::to_value(i.snapshot())?;
    legacy.as_object_mut().unwrap().remove("shared");
    for field in ["doors", "triggers"] {
        for value in legacy[field].as_array_mut().unwrap() {
            value.as_object_mut().unwrap().remove("id");
        }
    }
    legacy["encounters"]
        .as_object_mut()
        .unwrap()
        .remove("identities");
    let old: Snapshot = serde_json::from_value(legacy)?;
    let mut migrated = reload(assets, &map, "skool1", &old)?;
    let before = migrated.event_world.snapshot();
    migrated.completed_dialogue("Theatre_Cinematic");
    ensure!(
        migrated.event_world.snapshot() == before,
        "Legacy migration replayed committed theatre effects"
    );
    ensure!(
        !migrated.event_world.allowed(
            &Event::Entity(trigger, Input::Activate),
            &migrated.event_facts()
        ),
        "Legacy one-shot rearmed"
    );

    // Anonymous entity slots cannot be silently swapped now that IDs are persisted.
    let mut bad = serde_json::to_value(i.snapshot())?;
    bad["triggers"].as_array_mut().unwrap().swap(0, 1);
    ensure!(
        reload(assets, &map, "skool1", &serde_json::from_value(bad)?).is_err(),
        "Reordered trigger identity accepted"
    );

    let map = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
    let mut i = Interactions::load(&map)?;
    i.set_entry(assets, &map, "skool2", None)?;
    let lever_id = named(&map, "move_thread", "extendBleachers")?;
    let lever = &map.entities[lever_id.0];
    let origin = crate::interaction::vector(&lever["origin"]).unwrap();
    let rotation = Quat::from_rotation_z(
        lever
            .get("angle")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.)
            .to_radians(),
    );
    let mut world = World::from_bsp(&map)?;
    i.sync(&mut world);
    let mut p = Player::spawn(&world, origin + vec3(-45., 0., 48.))
        .context("Shared lever test approach obstructed")?;
    for _ in 0..240 {
        p.tick(&world, crate::movement::Controls::default());
    }
    let aim = (origin + rotation * vec3(36., 0., 26.) - p.eye()).normalize();
    i.event_world.set_enabled(lever_id, false)?;
    let mut i = reload(assets, &map, "skool2", &i.snapshot())?;
    i.update(FIXED_DT, &map, &mut world, &p, aim, true)?;
    ensure!(
        !i.gym.as_ref().unwrap().used,
        "Disabled saved lever still operates bleachers"
    );
    i.event_world.set_enabled(lever_id, true)?;
    i.update(FIXED_DT, &map, &mut world, &p, aim, true)?;
    ensure!(
        i.gym.as_ref().unwrap().used,
        "Reenabled lever cannot be used"
    );
    let mut i = reload(assets, &map, "skool2", &i.snapshot())?;
    i.update(FIXED_DT, &map, &mut world, &p, aim, true)?;
    ensure!(
        i.event_world.snapshot().facts.0["gym.lever_uses"] == crate::event::Value::Count(1),
        "Saved lever count duplicated"
    );
    i.completed_dialogue("Old_Gnome_Mushroom");
    ensure!(
        i.school2.as_ref().unwrap().quest.stage == crate::school2_quest::Stage::Explore,
        "Premature dialogue advanced quest"
    );
    activate(&mut i, named(&map, "thread", "Old_Gnome_Mushroom")?);
    let mut i = reload(assets, &map, "skool2", &i.snapshot())?;
    i.completed_dialogue("Old_Gnome_Mushroom");
    ensure!(
        i.scripted() && !i.school2.as_ref().unwrap().quest.items.mushroom,
        "Dialogue bypassed the school-two scene"
    );
    let mut story = crate::story::Story::load(assets, "skool2");
    story.trigger("Old_Gnome_Mushroom");
    i.skip_cinematic(&map, &mut world, &mut p, &mut story)?;
    let mut i = reload(assets, &map, "skool2", &i.snapshot())?;
    ensure!(
        i.school2.as_ref().unwrap().quest.items.mushroom
            && i.school2.as_ref().unwrap().boojums[..3]
                .iter()
                .all(|b| b.active),
        "Shared quest completion lost reward/encounter"
    );
    let before = serde_json::to_value(i.snapshot())?;
    i.completed_dialogue("Old_Gnome_Mushroom");
    ensure!(
        before == serde_json::to_value(i.snapshot())?,
        "Quest completion duplicated"
    );
    println!("PASS shared school entity IDs, saved disable/enable, group activation, one-shots, library conditions, four-book count, delayed theatre at 30/60/144 Hz, pause/restart, legacy migration and school-two dialogue/reward gates");
    Ok(())
}

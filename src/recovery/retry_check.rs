//! Real-map retry fixtures, separate from a continuous campaign playthrough.
use super::RetryPoint;
use crate::{
    assets::Assets,
    character::Character,
    cheshire::Hints,
    interaction::Interactions,
    inventory::Stats,
    movement::{Controls, Player, FIXED_DT},
    npc::Npcs,
    render::Scene,
    save::{Campaign, Game, Level, Restored, Slot, Store, View},
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;

fn capture(r: &Restored) -> Game {
    let mut game = r.game.clone();
    let level = game.campaign.levels.get_mut(&game.current).unwrap();
    level.interactions = r.interactions.snapshot();
    level.npcs = r.npcs.snapshot();
    level.story = r.story.snapshot();
    level.hints = r.hints.snapshot();
    game.view.position = game.player.eye();
    game.character = r.alice.snapshot();
    game
}

fn fixture(a: &mut Assets, name: &str, case: &str) -> Result<Restored> {
    let mut scene = Scene::load(a, name)?;
    let mut interactions = Interactions::load(&scene.map)?;
    interactions.set_entry(a, &scene.map, name, None)?;
    interactions.sync(&mut scene.world);
    let (eye, yaw) = crate::interaction::spawn(&scene.map, None);
    let mut player = Player::spawn(&scene.world, eye).context("Retry fixture entrance blocked")?;
    let mut story = Story::load(a, name);
    if case.starts_with("pool-") {
        story = crate::pool::boulder_check::stage(
            case,
            a,
            &scene.map,
            &mut interactions,
            &mut scene.world,
            &mut player,
        )?;
    } else if case.starts_with("utemple-") {
        story = crate::levels::utemple::stage(
            case,
            a,
            &scene.map,
            &mut interactions,
            &mut scene.world,
            &mut player,
        )?;
    } else {
        interactions.entry_story(&mut story);
    }
    let mut stats = Stats::for_level(name, None);
    stats.arrival_grants(name);
    interactions.prepare_player(&mut stats, &mut player);
    stats.set_health(37.)?;
    stats.spend_will(59.);
    let mut hints = Hints::load(a, &scene.map, name)?;
    hints.sync(&interactions);
    let npcs = Npcs::load(a, &scene.map, name, None, false, false)?;
    let mut alice = Character::load(a)?;
    alice.reset(&player, yaw);
    let level = Level {
        map: name.into(),
        entry: None,
        interactions: interactions.snapshot(),
        story: story.snapshot(),
        hints: hints.snapshot(),
        npcs: npcs.snapshot(),
        environment_clock: 0.,
        pickup_clock: 0.,
    };
    let current = level.key();
    let mut campaign = Campaign::default();
    campaign.levels.insert(current.clone(), level);
    let game = Game {
        current,
        campaign,
        stats,
        view: View {
            position: player.eye(),
            yaw,
            pitch: 0.,
            third_person: true,
            flying: false,
            fullbright: false,
            spawn_landing: false,
        },
        player,
        recovery: Default::default(),
        character: alice.snapshot(),
    };
    Restored::build(a, game)
}

fn advance(r: &mut Restored, ticks: usize) -> Result<()> {
    for _ in 0..ticks {
        r.interactions.advance_school(
            FIXED_DT,
            &r.scene.map,
            &mut r.scene.world,
            &mut r.game.player,
        )?;
        r.interactions
            .prepare_player(&mut r.game.stats, &mut r.game.player);
        if !r.interactions.scripted() {
            r.game.player.tick(&r.scene.world, Controls::default());
        }
        let _ = r.interactions.update(
            FIXED_DT,
            &r.scene.map,
            &mut r.scene.world,
            &mut r.game.player,
            Vec3::X,
            false,
        )?;
        if r.interactions.prepare_story(&mut r.story) {
            r.story.tick(FIXED_DT, false);
        }
        r.interactions.sync_cinematic_story(&r.story);
        for id in r.story.take_completed() {
            r.interactions.completed_dialogue(&id);
        }
        let level = r.game.campaign.levels.get_mut(&r.game.current).unwrap();
        level.environment_clock += FIXED_DT;
        level.pickup_clock += FIXED_DT;
    }
    Ok(())
}

pub async fn check(a: &mut Assets) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    for (name, case) in [
        ("potears1", "pool-boulder-three-wait"),
        ("utemple", "utemple-mid-guide"),
        ("potears1", "pool-boulder-one-push"),
        ("skool2", "entry"),
    ] {
        let mut live = fixture(a, name, case)?;
        let saved = capture(&live);
        let mut retry = RetryPoint::default();
        retry.remember(saved.clone());
        ensure!(
            retry.game_for("another$first").is_err(),
            "Retry crossed visits"
        );
        let store = Store::new(
            format!("private/death-retry-check/{case}").into(),
            fingerprint.clone(),
        );
        store.write(Slot::Quick, &saved)?;
        advance(&mut live, 600)?;
        ensure!(
            serde_json::to_value(live.interactions.snapshot())?
                != serde_json::to_value(&saved.level()?.interactions)?,
            "Fixture did not advance: {case}"
        );
        live.game
            .stats
            .collected
            .insert("retry-check:later-pickup".into());
        live.game.stats.set_health(0.)?;
        retry.remember(capture(&live));
        for attempt in 0..2 {
            let mut restored = Restored::build(a, retry.game_for(&saved.current)?)?;
            ensure!(
                serde_json::to_value(capture(&restored))? == serde_json::to_value(&saved)?,
                "Incomplete world/player rollback: {case}"
            );
            let mut reference = Restored::build(a, store.read(Slot::Quick)?.game)?;
            advance(&mut restored, 240)?;
            advance(&mut reference, 240)?;
            ensure!(
                serde_json::to_value(capture(&restored))?
                    == serde_json::to_value(capture(&reference))?,
                "Retry continuation diverged: {case}"
            );
            println!("PASS death retry {case} attempt {attempt}: exact world, actors, dialogue, resources, player and clocks; 240 matching continuation ticks");
            next_frame().await;
        }
        // A successful later save/load becomes the new checkpoint, while failed
        // attempts cannot advance it or retain pickups collected after it.
        let mut later = Restored::build(a, saved.clone())?;
        advance(&mut later, 60)?;
        let later = capture(&later);
        retry.remember(later.clone());
        ensure!(
            serde_json::to_value(retry.game_for(&saved.current)?)? == serde_json::to_value(later)?,
            "Later checkpoint ignored"
        );
        println!("PASS {case}: later checkpoint replaces entry; death cannot overwrite it");
    }
    Ok(())
}

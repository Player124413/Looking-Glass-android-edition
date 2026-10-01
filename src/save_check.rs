//! Staged persistence fixtures, intentionally separate from normal traversal proof.
//! Writer and reader must run in different OS processes. All files stay private.
use crate::{
    assets::Assets,
    character::{Character, WeaponInput},
    cheshire::Hints,
    combat::{Context as CombatContext, Hit},
    encounters::Enemy,
    interaction::{self, Interactions},
    inventory::{self, Catalog, Stats},
    movement::{Player, FIXED_DT},
    npc::Npcs,
    recovery::Recovery,
    render::Scene,
    save::{Campaign, Game, Level, Restored, Slot, Store, View},
    school2_quest::Stage,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::Serialize;
use std::{fs, path::Path};

const ROOT: &str = "private/save-check";
pub(crate) const CASES: &[&str] = &[
    "pool-boulder-one-push",
    "pool-boulder-one-flight",
    "pool-boulder-two-flight",
    "pool-boulder-three-wait",
    "pool-boulder-three-live",
    "pool-boulder-ant-dead",
    "pool-arrival-ready",
    "pool-arrival-fade",
    "pool-arrival-rock",
    "pool-arrival-rabbit",
    "pool-arrival-walk",
    "pool-arrival-complete",
    "cheshire-pool-before-47",
    "cheshire-pool-before-728",
    "cheshire-pool-after-46",
    "cheshire-pool-fade-out",
    "cheshire-pool-cooldown",
    "cheshire-pool-swimming",
    "pool-pilot-fade",
    "pool-pilot-jump",
    "pool-pilot-fall",
    "pool-pilot-late",
    "pool-pilot-complete",
    "school2-enemy-live",
    "school2-enemy-hurt",
    "school2-enemy-bolt",
    "school2-enemy-dead",
    "school2-enemy-dice-before",
    "school2-enemy-dice-dialogue",
    "school2-enemy-dice-live",
    "school2-enemy-dice-dead",
    "school2-enemy-growth",
    "friendly-village-first",
    "friendly-village-repeat-ready",
    "friendly-village-repeat-queued",
    "friendly-village-repeat-active",
    "friendly-school-mushroom",
    "friendly-school-final",
    "friendly-pool",
    "guard-cut-before",
    "guard-cut-live",
    "guard-cut-expired",
    "guard-head-before",
    "guard-head-live",
    "guard-head-expired",
    "item-rage",
    "item-tea",
    "item-glass",
    "item-watch",
    "item-watch-expiry",
    "item-watch-recharge",
    "item-drop-live",
    "item-drop-collected",
    "item-drop-expired",
    "movement-current",
    "movement-rope",
    "movement-ledge",
    "movement-ledge-shimmy",
    "movement-ledge-pullup",
    "movement-breath",
    "movement-knockback",
    "movement-updraft",
    "duchess-expand",
    "duchess-intro",
    "duchess-phase",
    "duchess-pepper",
    "duchess-pig",
    "duchess-bite",
    "duchess-death",
    "duchess-rescue",
    "duchess-reward",
    "duchess-complete",
    "school",
    "school-theatre-scene",
    "school-theatre-dialogue",
    "school-theatre-dialogue-gap",
    "school-theatre-before-guards",
    "school-theatre-guards",
    "school-theatre-disappear",
    "school-theatre-skipped",
    "school-shelf-scene",
    "school-library-shelves",
    "school-library-book-talk",
    "school-library-book-push",
    "school-library-book-drop",
    "school-library-book-skip",
    "school-library-recipe",
    "school-walking-pupil",
    "fortress-rage-pickup",
    "fortress-rage-growing",
    "fortress-rage-lowering",
    "fortress-rage-complete",
    "fortress-rage-skipped",
    "beyond-demonstration",
    "beyond-reset",
    "beyond-solved",
    "beyond-arches",
    "school2-scene-mushroom",
    "school2-scene-mushroom-handout",
    "school2-scene-mushroom-boojums",
    "school2-scene-warp",
    "school2-scene-warp-walk",
    "school2-scene-spice",
    "school2-scene-spice-cabinet",
    "school2-scene-spice-return",
    "school2-scene-spice-skipped",
    "school2-scene-growth-fade",
    "school2-scene-growth-pour",
    "school2-scene-growth-grown",
    "school2-scene-growth-skipped",
    "school2-scene-final-intro",
    "school2-scene-final-mixing",
    "school2-scene-final-star",
    "school2-scene-final-departure",
    "school2-scene-final-skipped",
    "school2-scene-final-potion-collected",
    "battle",
    "growing",
    "mixing",
    "rewards",
    "return",
    "return-star",
    "return-star-close",
    "return-star-skipped",
    "return-globe",
    "return-takeoff",
    "return-skipped",
    "return-complete",
    "return-lift",
    "return-potion",
    "return-shrink",
    "village",
    "village-fall",
    "village-knife",
    "village-shrink",
    "village-gnome",
    "exit",
    "dice-cast",
    "dice-roll",
    "dice-summon",
    "ladybug-delay",
    "ladybug-bomb",
    "ladybug-dead",
    "pand-skipped-cart",
    "pand-skipped-warning",
    "pand-warning",
    "pand-vanish",
    "pand-leave",
    "pand-landing",
    "pand-board",
    "pand-cart",
    "pand-key",
    "pand-return",
    "pand-flight",
];
/// The legacy cases followed by every registered visit's cases.
fn cases() -> Vec<&'static str> {
    cases_in(crate::levels::LEVELS)
}
/// `cases` for an explicit registration list (the unit tests use a synthetic one).
fn cases_in(list: &[&'static crate::levels::Registration]) -> Vec<&'static str> {
    CASES
        .iter()
        .copied()
        .chain(
            crate::levels::save_cases_in(list)
                .into_iter()
                .map(|c| c.name),
        )
        .collect()
}
fn same(a: &impl Serialize, b: &impl Serialize, label: &str) -> Result<()> {
    ensure!(
        serde_json::to_value(a)? == serde_json::to_value(b)?,
        "Restored {label} differs"
    );
    Ok(())
}
fn snapshot(r: &Restored) -> Level {
    let original = r.game.level().unwrap();
    Level {
        map: original.map.clone(),
        entry: original.entry.clone(),
        interactions: r.interactions.snapshot(),
        npcs: r.npcs.snapshot(),
        story: r.story.snapshot(),
        hints: r.hints.snapshot(),
        environment_clock: original.environment_clock,
        pickup_clock: original.pickup_clock,
    }
}
fn advance(r: &mut Restored, movement: bool) -> Result<()> {
    for _ in 0..if r.game.level()?.map == "potears1" {
        480
    } else {
        60
    } {
        let world_dt = r.game.stats.powers.world_dt(FIXED_DT);
        r.game.stats.prepare_player(&mut r.game.player);
        r.interactions
            .prepare_player(&mut r.game.stats, &mut r.game.player);
        let before = r.interactions.loot_sources();
        r.interactions.advance_school(
            world_dt,
            &r.scene.map,
            &mut r.scene.world,
            &mut r.game.player,
        )?;
        r.interactions.update(
            world_dt,
            &r.scene.map,
            &mut r.scene.world,
            &r.game.player,
            Vec3::X,
            false,
        )?;
        if movement {
            r.game.player.tick(
                &r.scene.world,
                crate::movement::Controls {
                    rise: if r.game.player.rope.is_some() { 1. } else { 0. },
                    ..Default::default()
                },
            );
            r.game
                .stats
                .damage(std::mem::take(&mut r.game.player.liquid_damage));
        }
        if let Some(s) = &mut r.interactions.school {
            s.sync_inventory(&mut r.game.stats);
        }
        if let Some(d) = &mut r.interactions.duchess {
            let f = d.update(
                world_dt,
                &r.scene.world,
                &mut r.game.player,
                &mut r.game.stats,
                &mut r.story,
            );
            r.game.stats.damage(f.damage);
        }
        if r.interactions.pool.is_some() || r.game.level()?.map == "potears2" {
            let combat_dt = if r.interactions.scripted() {
                0.
            } else {
                world_dt
            };
            let feedback = r.interactions.levels_step(&mut crate::level::Combat {
                dt: combat_dt,
                world: &r.scene.world,
                player: &mut r.game.player,
                stats: &mut r.game.stats,
                story: &mut r.story,
                notarget: false,
                summon: None,
                threatens: &|_| false,
            });
            r.game.stats.damage(feedback.damage);
            r.game.player.velocity += feedback.impulse;
        }
        r.npcs.update(world_dt, &r.scene.world, r.game.player.eye());
        let events = r
            .interactions
            .triggers(FIXED_DT, r.game.player.feet, r.game.player.feet);
        for event in events.story {
            r.story.trigger(&event);
        }
        if let Some(e) = &mut r.interactions.encounters {
            e.update(world_dt, &r.scene.world, r.game.player.eye());
        }
        if let Some(s) = &mut r.interactions.school2 {
            s.update(world_dt, &r.scene.world, r.game.player.feet, false);
        }
        r.alice.power_appearance(&r.game.stats);
        r.alice.world_time(FIXED_DT, world_dt);
        r.alice.update(
            FIXED_DT,
            &r.game.player,
            false,
            WeaponInput {
                dice: r.game.stats.copies(6),
                selected: r.game.stats.selected(),
                click: None,
                aim: Vec3::X,
                first_person: !r.game.view.third_person,
            },
            &CombatContext {
                world: &r.scene.world,
                targets: &[],
            },
        );
        if let Some((time, finished)) = r.interactions.beyond.as_ref().and_then(|b| b.rage_scene())
        {
            r.alice.rage_scene(time, finished, &r.game.player);
        }
        r.hints.sync(&r.interactions);
        if r.interactions.prepare_story(&mut r.story) && r.hints.prepare_story(&r.story) {
            r.story.tick(FIXED_DT, false);
        }
        r.interactions.sync_cinematic_story(&r.story);
        for event in r.story.take_completed() {
            r.interactions.completed_dialogue(&event);
        }
        r.hints.update(FIXED_DT, r.story.hint_active());
        crate::loot::tick(
            &mut r.game.stats,
            &r.game.current,
            world_dt,
            &before,
            &r.interactions.loot_sources(),
            r.game.player.feet,
            &r.scene.world,
        );
        r.game.stats.update(FIXED_DT);
    }
    Ok(())
}
fn future(r: &Restored) -> Result<serde_json::Value> {
    Ok(
        serde_json::json!({ "level": snapshot(r), "player": r.game.player,
        "character": r.alice.snapshot(), "stats": r.game.stats, "transforms": r.interactions.transforms() }),
    )
}

pub async fn check(assets: &mut Assets, write: bool) -> Result<()> {
    clear_background(BLACK);
    draw_text("Checking persistent saves...", 24., 40., 26., WHITE);
    next_frame().await;
    let root = Path::new(ROOT);
    let selected = std::env::var("LOOKING_GLASS_SAVE_CASE").ok();
    let fingerprint = assets.fingerprint()?;
    if write {
        fs::create_dir_all(root)?;
        let mut campaign = Campaign::default();
        let mut stats = Stats::for_level("skool2", None);
        stats.damage(63.);
        stats.spend_will(59.);
        stats.apply(inventory::PickupKind::Weapon(6), 1.);
        stats.apply(inventory::PickupKind::Weapon(6), 1.);
        stats.select(2);
        let catalog = Catalog::load(assets)?;
        let mut visits = vec![
            ("gvillage", None),
            ("fortress2", None),
            ("skool1", None),
            ("skool1", Some("skool1_start2")),
            ("skool2", None),
            ("potears1", None),
            ("pandemonium", None),
            ("potears3", None),
            ("garden1", None),
        ];
        for visit in crate::levels::save_visits() {
            if !visits.contains(&visit) {
                visits.push(visit);
            }
        }
        for (name, entry) in visits {
            let mut scene = Scene::load(assets, name)?;
            let mut i = Interactions::load(&scene.map)?;
            i.set_entry(assets, &scene.map, name, entry)?;
            i.sync(&mut scene.world);
            let mut p = Player::spawn(&scene.world, interaction::spawn(&scene.map, entry).0)
                .context("Fixture entrance obstructed")?;
            let mut story = Story::load(assets, name);
            let mut hints = Hints::load(assets, &scene.map, name)?;
            if name == "skool1" && entry.is_none() {
                let center = scene
                    .map
                    .entities
                    .iter()
                    .find(|e| {
                        e.get("targetname")
                            .is_some_and(|s| s == "first_monster_trigger1")
                    })
                    .and_then(|e| e.get("origin"))
                    .and_then(|s| interaction::vector(s))
                    .unwrap();
                i.triggers(FIXED_DT, center - Vec3::Z * 28., center - Vec3::Z * 28.);
                i.activate_enemies();
                let mut enemies = i
                    .encounters
                    .as_mut()
                    .unwrap()
                    .actors
                    .iter_mut()
                    .filter(|a| a.active);
                for (a, damage) in enemies.by_ref().take(2).zip([1000., 17.]) {
                    match &mut a.enemy {
                        Enemy::Guard(g) => {
                            g.hurt(damage);
                        }
                        Enemy::Boojum(b) => {
                            b.hit(damage);
                        }
                        Enemy::Ladybug(b) => {
                            b.hit(damage);
                        }
                    }
                }
                i.event_world.post(
                    crate::event::Event::DialogueFinished("Theatre_Cinematic".into()),
                    0.25,
                )?;
                i.activate_enemies();
                let s = i.school.as_mut().unwrap();
                for event in [
                    "Theatre_Cinematic",
                    "Skool1_OG_MoveShelf",
                    "shelf_cinematic",
                    "Skool1_Lift_Up",
                    "start_book1",
                    "start_book2",
                    "Open_Bookcase_Goodie",
                ] {
                    s.event(event);
                }
                i.sync(&mut scene.world);
                i.school.as_mut().unwrap().collect_secret(
                    &mut stats,
                    vec3(624., 2581., -240.),
                    &scene.world,
                );
                stats.update(0.75);
                // Take a real authored resource pickup; repeated contact must not refill it later.
                let items = inventory::pickups(&scene.map, name, &catalog);
                for item in &items {
                    inventory::collect(
                        &mut stats,
                        std::slice::from_ref(item),
                        item.origin,
                        &scene.world,
                    );
                    if !stats.collected.is_empty() {
                        break;
                    }
                }
                ensure!(!stats.collected.is_empty(), "No fixture pickup collected");
                story.trigger("Theatre_Cinematic");
                story.tick(FIXED_DT, false);
                story.tick(0.2, false);
            } else if name == "skool2" {
                i.gym.as_mut().unwrap().used = true;
                let s = i.school2.as_mut().unwrap();
                s.event("Old_Gnome_Mushroom");
                s.skip_scene(&scene.world, &mut p, &mut story)?;
                s.boojums[0].hit(1000.);
                s.boojums[1].hit(19.);
                s.guards[0].hurt(13.);
            } else if name == "gvillage" {
                i.completed_dialogue("Gnome3_Dialog");
                i.activate_enemies();
                hints
                    .summon(&mut story, &scene.world, p.feet, 0., false)
                    .map_err(anyhow::Error::msg)?;
                story.tick(FIXED_DT, false);
                hints.update(0.2, true);
            }
            for _ in 0..48 {
                i.advance_school(FIXED_DT, &scene.map, &mut scene.world, &mut p)?;
            }
            let mut npcs = Npcs::load(assets, &scene.map, name, entry, false, false)?;
            if let Some(t) = npcs.targets().first() {
                npcs.hit(Hit {
                    knockback: Vec3::ZERO,
                    kind: crate::combat::DamageKind::Other,
                    id: t.id,
                    damage: 23.,
                });
            }
            npcs.update(0.12, &scene.world, p.eye());
            let level = Level {
                map: name.into(),
                entry: entry.map(str::to_owned),
                interactions: i.snapshot(),
                npcs: npcs.snapshot(),
                story: story.snapshot(),
                hints: hints.snapshot(),
                environment_clock: 3.25,
                pickup_clock: 2.625,
            };
            campaign.completed.insert(level.key());
            campaign.levels.insert(level.key(), level);
            next_frame().await;
        }
        // Weapon pickups refill meters; save deliberately depleted meters after collecting.
        stats.damage(63.);
        stats.spend_will(59.);
        for case in cases().into_iter().filter(|case| {
            selected
                .as_ref()
                .is_none_or(|prefix| case.starts_with(prefix))
        }) {
            let key = crate::levels::save_visit(case).unwrap_or(match case {
                "movement-current" | "movement-rope" => "potears1$first",
                "movement-breath"
                | "movement-ledge"
                | "movement-ledge-shimmy"
                | "movement-ledge-pullup" => "garden1$first",
                "movement-knockback" => "skool1$first",
                c if c.starts_with("guard-cut") || c.starts_with("guard-head") => "skool1$first",
                "movement-updraft" => "gvillage$first",
                c if c.starts_with("duchess-") => "potears3$first",
                "school" | "dice-cast" | "dice-roll" | "dice-summon" => "skool1$first",
                c if c.starts_with("school-") => "skool1$first",
                c if c.starts_with("fortress-rage-") => "fortress2$first",
                c if c.starts_with("beyond-") => "fortress2$first",
                c if c == "return" || c.starts_with("return-") => "skool1$return",
                c if c.starts_with("pand-") => "pandemonium$first",
                c if c.starts_with("village") || c.starts_with("friendly-village") => {
                    "gvillage$first"
                }
                "friendly-pool" => "potears1$first",
                c if c.starts_with("cheshire-pool-") => "potears1$first",
                c if c.starts_with("pool-pilot-")
                    || c.starts_with("pool-arrival-")
                    || c.starts_with("pool-boulder-") =>
                {
                    "potears1$first"
                }
                "ladybug-delay" | "ladybug-bomb" | "ladybug-dead" => "potears1$first",
                _ => "skool2$first",
            });
            let mut c = campaign.clone();
            if ["growing", "mixing", "rewards", "exit"].contains(&case) {
                let level = c.levels.get_mut(key).unwrap();
                let map = crate::bsp::Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
                let (mut i, _, _) = level.restore_logic(assets, &map)?;
                let s = i.school2.as_mut().unwrap();
                for b in &mut s.boojums[..3] {
                    b.hit(1000.);
                }
                for g in &mut s.guards {
                    g.hurt(1000.);
                }
                s.quest.stage = Stage::SpiceDialogue;
                s.completed_dialogue("Old_Gnome_SpiceDrops");
                ensure!(s.quest.collect("jumbo_shelf1"), "Jumbogrow setup failed");
                s.quest.begin("Skool2_GrowLollypop");
                s.quest.time = 2.25;
                if case != "growing" {
                    s.quest.enter(Stage::Lollipop);
                    ensure!(s.quest.collect("ig_lollypop"), "Lollipop setup failed");
                    s.quest.begin("Skool2_LastGnome_Cinema");
                    s.completed_dialogue("Skool2_LastGnome_Cinema");
                    s.quest.time = 3.5;
                }
                if case == "rewards" || case == "exit" {
                    for _ in 0..16 {
                        s.quest.tick(0.1, 3, 2);
                    }
                    ensure!(s.quest.collect("shrink_potion"), "Potion setup failed");
                    if case == "exit" {
                        ensure!(s.quest.collect("lucky_star"), "Star setup failed");
                    }
                }
                level.interactions = i.snapshot();
            }
            let mut ladybug_eye = None;
            if case.starts_with("ladybug-") {
                let l = c.levels.get_mut(key).unwrap();
                let map = crate::bsp::Bsp::parse(&assets.read("maps/potears1.bsp")?)?;
                let world = crate::collision::World::from_bsp(&map)?;
                let (mut i, _, _) = l.restore_logic(assets, &map)?;
                if case == "ladybug-delay" {
                    let e = map
                        .entities
                        .iter()
                        .find(|e| e.get("thread").is_some_and(|s| s == "Spawn_LadyX4"))
                        .unwrap();
                    let feet = interaction::vector(&e["origin"]).unwrap() - Vec3::Z * 28.;
                    i.triggers(FIXED_DT, feet, feet);
                    let cast = i.encounters.as_ref().unwrap();
                    ensure!(
                        cast.actors.iter().any(|a| a.name == "x_lady7" && a.active)
                            && cast.actors.iter().any(|a| a.name == "x_lady8" && !a.active),
                        "Wrong pending ambush fixture"
                    );
                } else {
                    let timing = crate::ladybug::Timing::load(assets)?;
                    let cast = i.encounters.as_mut().unwrap();
                    let b = cast
                        .actors
                        .iter_mut()
                        .find_map(|a| match &mut a.enemy {
                            Enemy::Ladybug(b) if a.name == "lady3" => Some(b),
                            _ => None,
                        })
                        .unwrap();
                    let eye = b.target(0).center - Vec3::Z * 192.;
                    ladybug_eye = Some(eye);
                    for _ in 0..600 {
                        b.advance(FIXED_DT, &world, eye, timing);
                        if !b.acorns.is_empty() {
                            break;
                        }
                    }
                    ensure!(!b.acorns.is_empty(), "No live acorn for restart fixture");
                    if case == "ladybug-dead" {
                        b.hit(100.);
                    }
                }
                l.interactions = i.snapshot();
            }
            if let Some(stage) = crate::levels::save_case(case).and_then(|s| s.stage) {
                // A registered case stages its own state on the restored visit.
                let l = c.levels.get_mut(key).unwrap();
                let map = crate::bsp::Bsp::parse(&assets.read(&format!("maps/{}.bsp", l.map))?)?;
                let (mut i, _, _) = l.restore_logic(assets, &map)?;
                stage(&mut i, &map)?;
                l.interactions = i.snapshot();
            }
            let l = &c.levels[key];
            let mut scene = Scene::load(assets, &l.map)?;
            let (mut i, _, _) = l.restore_logic(assets, &scene.map)?;
            if case.starts_with("movement-") {
                crate::levels::garden1::stage_gameplay(&mut i);
            }
            i.sync(&mut scene.world);
            let mut player = Player::spawn(
                &scene.world,
                interaction::spawn(&scene.map, l.entry.as_deref()).0,
            )
            .unwrap();
            if case == "exit" {
                player = Player::spawn(&scene.world, vec3(-64., -1150., 304.))
                    .context("Exit approach obstructed")?;
            }
            player.grounded = true;
            if case.starts_with("guard-cut") || case.starts_with("guard-head") {
                let cast = i.encounters.as_mut().context("Missing school guards")?;
                let timing = crate::npc::guard_timing(assets)?;
                let guard = cast
                    .actors
                    .iter_mut()
                    .find_map(|actor| match &mut actor.enemy {
                        Enemy::Guard(g) if !g.ranged && g.health > 0. => Some(g),
                        _ => None,
                    })
                    .context("No live Club Guard")?;
                guard.hurt_kind(1000., crate::combat::DamageKind::Knife);
                guard.cut_variant = if case.starts_with("guard-head") {
                    crate::dismember::Variant::Head
                } else {
                    crate::dismember::Variant::Torso
                };
                let frames = match case {
                    "guard-cut-before" | "guard-head-before" => 3,
                    "guard-cut-live" | "guard-head-live" => 30,
                    _ => 750,
                };
                for _ in 0..frames {
                    guard.advance(FIXED_DT, &scene.world, player.eye(), timing);
                }
                ensure!(guard.cut, "Cut fixture failed");
                if case.ends_with("-live") {
                    ensure!(
                        guard.dismember.fragment.is_some(),
                        "Cut fixture needs a live detached piece"
                    );
                }
                if case.ends_with("-expired") {
                    ensure!(
                        guard.dismember.severed && guard.dismember.fragment.is_none(),
                        "Cut cleanup fixture failed"
                    );
                }
                c.levels.get_mut(key).unwrap().interactions = i.snapshot();
            }
            if let Some(eye) = ladybug_eye {
                player = Player::new(eye - Vec3::Z * crate::movement::EYE_HEIGHT);
                ensure!(
                    scene.world.body_clear(player.feet),
                    "Ladybug fixture player obstructed"
                );
            }
            if case.starts_with("village-") {
                use crate::village::cinema::Beat;
                let mut story = Story::load(assets, "gvillage");
                let cinema = &mut i.village.as_mut().unwrap().cinema;
                cinema.state = Default::default();
                cinema.state.home = Some(player.feet);
                if case == "village-fall" {
                    cinema.begin(&mut story);
                } else {
                    cinema.state.done.insert("entry".into());
                    cinema.state.beat = Some(match case {
                        "village-knife" => Beat::Knife,
                        "village-shrink" => Beat::Shrink,
                        _ => Beat::Gnome(0),
                    });
                    let event = if case == "village-gnome" {
                        "Torchgnome1_Dialog"
                    } else {
                        "knife_cat"
                    };
                    if case == "village-shrink" {
                        story.finish_sequence(event);
                        story.take_completed();
                    } else {
                        story.trigger(event);
                        story.tick(FIXED_DT, false);
                    }
                }
                cinema.state.time = if case == "village-fall" { 2. } else { 3. };
                cinema.advance(FIXED_DT, &mut player, &scene.world)?;
                i.sync(&mut scene.world);
                c.levels.get_mut(key).unwrap().story = story.snapshot();
                c.levels.get_mut(key).unwrap().interactions = i.snapshot();
            }
            if case.starts_with("pand-") {
                i.pandemonium.as_mut().unwrap().fixture(
                    match case {
                        "pand-skipped-cart" => "pand-cart",
                        "pand-skipped-warning" => "pand-warning",
                        _ => case,
                    },
                    &scene.map,
                    &mut scene.world,
                    &mut player,
                )?;
                if case.starts_with("pand-skipped-") {
                    let mut story = Story::load(assets, "pandemonium");
                    if case.ends_with("warning") {
                        story.trigger(crate::pandemonium::cinema::WARNING);
                    }
                    i.skip_cinematic(&scene.map, &mut scene.world, &mut player, &mut story)?;
                    for id in story.take_completed() {
                        i.completed_dialogue(&id);
                    }
                    i.triggers(FIXED_DT, player.feet, player.feet);
                    i.activate_enemies();
                    c.levels.get_mut(key).unwrap().story = story.snapshot();
                }
                i.sync(&mut scene.world);
                c.levels.get_mut(key).unwrap().interactions = i.snapshot();
                if matches!(case, "pand-flight" | "pand-warning" | "pand-board") {
                    let mut story = Story::load(assets, "pandemonium");
                    story.trigger(if case == "pand-warning" {
                        crate::pandemonium::cinema::WARNING
                    } else {
                        crate::pandemonium::cinema::DEPARTURE
                    });
                    story.tick(FIXED_DT, false);
                    c.levels.get_mut(key).unwrap().story = story.snapshot();
                }
            }
            let mut recovery = Recovery::default();
            recovery.observe(&scene.world, &i, &player, 0.3);
            let mut alice = Character::load(assets)?;
            alice.reset(&player, 0.3);
            for _ in 0..90 {
                alice.update(
                    FIXED_DT,
                    &player,
                    false,
                    WeaponInput {
                        dice: 1,
                        selected: stats.selected(),
                        click: None,
                        aim: Vec3::X,
                        first_person: true,
                    },
                    &CombatContext {
                        world: &scene.world,
                        targets: &[],
                    },
                );
            }
            alice.update(
                FIXED_DT,
                &player,
                false,
                WeaponInput {
                    dice: 1,
                    selected: stats.selected(),
                    click: Some(true),
                    aim: Vec3::X,
                    first_person: true,
                },
                &CombatContext {
                    world: &scene.world,
                    targets: &[],
                },
            );
            let mut game_stats = stats.clone();
            if case.starts_with("item-") {
                game_stats.invisible = 0.;
                use crate::powerups::Kind;
                match case {
                    c if c.starts_with("movement-ledge") => {
                        let (feet, forward) = crate::ledge::probe(&scene.map, &scene.world)
                            .context("No saved ledge fixture")?;
                        player = Player::new(feet);
                        player.velocity = -Vec3::Z * 80.;
                        player.tick(
                            &scene.world,
                            crate::movement::Controls {
                                wish: forward,
                                ..Default::default()
                            },
                        );
                        ensure!(player.ledge.is_some(), "Save fixture failed to catch ledge");
                        player.tick(&scene.world, crate::movement::Controls::default());
                        if c.ends_with("pullup") {
                            player.tick(
                                &scene.world,
                                crate::movement::Controls {
                                    wish: forward,
                                    ..Default::default()
                                },
                            );
                            for _ in 0..130 {
                                player.tick(&scene.world, crate::movement::Controls::default());
                            }
                        } else if c.ends_with("shimmy") {
                            for _ in 0..30 {
                                player.tick(
                                    &scene.world,
                                    crate::movement::Controls {
                                        wish: vec2(-forward.y, forward.x),
                                        ..Default::default()
                                    },
                                );
                            }
                        }
                        player.validate_world(&scene.world)?;
                    }
                    "item-rage" => {
                        game_stats.powerup(Kind::Rage);
                        game_stats.update(5.25);
                    }
                    "item-tea" => {
                        game_stats.powerup(Kind::Tea);
                        game_stats.update(5.25);
                    }
                    "item-glass" => {
                        game_stats.powerup(Kind::Glass);
                        game_stats.update(5.25);
                    }
                    c if c.starts_with("item-watch") => {
                        game_stats.apply(inventory::PickupKind::Weapon(9), 100.);
                        game_stats.watch().unwrap();
                        game_stats.update(match c {
                            "item-watch-expiry" => 19.75,
                            "item-watch-recharge" => 359.75,
                            _ => 7.25,
                        });
                    }
                    _ => {
                        let before = i.loot_sources();
                        let s = i.school2.as_mut().unwrap();
                        s.boojums[1].active = true;
                        s.hit(crate::school2::ENEMY_BASE + 1, 1000.);
                        let after = i.loot_sources();
                        crate::loot::tick(
                            &mut game_stats,
                            key,
                            0.1,
                            &before,
                            &after,
                            player.feet,
                            &scene.world,
                        );
                        ensure!(
                            game_stats.loot[key].drops.len() == 1,
                            "Missing fixture death reward"
                        );
                        if case == "item-drop-collected" {
                            let drop = game_stats.loot[key].drops[0].origin;
                            crate::loot::tick(
                                &mut game_stats,
                                key,
                                0.1,
                                &after,
                                &after,
                                drop,
                                &scene.world,
                            );
                        } else if case == "item-drop-expired" {
                            game_stats.loot.get_mut(key).unwrap().update(80.);
                        } else {
                            game_stats.loot.get_mut(key).unwrap().update(9.75);
                        }
                        c.levels.get_mut(key).unwrap().interactions = i.snapshot();
                    }
                }
                game_stats.prepare_player(&mut player);
                if matches!(case, "item-rage" | "item-tea") {
                    alice.reset(&player, 0.);
                    alice.power_appearance(&game_stats);
                    for _ in 0..105 {
                        alice.update(
                            0.05,
                            &player,
                            false,
                            WeaponInput {
                                selected: game_stats.selected(),
                                aim: Vec3::X,
                                ..Default::default()
                            },
                            &CombatContext {
                                world: &scene.world,
                                targets: &[],
                            },
                        );
                    }
                }
            }
            if case.starts_with("movement-") {
                match case {
                    "movement-current" => player = Player::new(vec3(-2148., 3056., 1252.)),
                    "movement-updraft" => player = Player::new(vec3(-5088., 5076., -112.)),
                    "movement-rope" => {
                        player = Player::new(vec3(-2296., 1224., 488.));
                        player.tick(
                            &scene.world,
                            crate::movement::Controls {
                                use_pressed: true,
                                ..Default::default()
                            },
                        );
                        ensure!(player.rope.is_some(), "Save fixture failed to grab rope");
                    }
                    "movement-breath" => {
                        player = Player::new(
                            crate::traversal::water_probe(&scene.world, player.feet)
                                .context("No quiet submerged fixture")?,
                        );
                        game_stats.turtle_air = true;
                        player.breath.shell = true;
                        for _ in 0..2376 {
                            player.tick(&scene.world, crate::movement::Controls::default());
                        }
                        ensure!(
                            player.breath.remaining() < 0.3 && player.breath.remaining() > 0.,
                            "Breath fixture did not remain submerged"
                        );
                    }
                    _ => player.knockback(vec3(0., 250., 80.)),
                }
                if case != "movement-breath" {
                    player.tick(&scene.world, crate::movement::Controls::default());
                }
                ensure!(
                    scene.world.body_clear(player.feet),
                    "Movement save fixture obstructed"
                );
                alice.reset(&player, 0.);
            }
            if case.starts_with("duchess-") {
                let mut story = Story::load(assets, "potears3");
                i.duchess.as_mut().unwrap().fixture(
                    case,
                    &scene.map,
                    &mut scene.world,
                    &mut player,
                    &mut game_stats,
                    &mut story,
                )?;
                let l = c.levels.get_mut(key).unwrap();
                l.interactions = i.snapshot();
                l.story = story.snapshot();
                alice.reset(&player, player.script_facing);
            }
            if case == "return" || case.starts_with("return-") {
                i.school.as_mut().unwrap().return_fixture(
                    case,
                    &scene.map,
                    &mut scene.world,
                    &mut player,
                    &mut game_stats,
                )?;
                i.sync(&mut scene.world);
                c.levels.get_mut(key).unwrap().interactions = i.snapshot();
                alice.reset(&player, 0.);
            }
            if case.starts_with("dice-") {
                game_stats.restore();
                ensure!(game_stats.select(6), "Fixture lacks Dice");
                alice.reset(&player, std::f32::consts::FRAC_PI_2);
                let input = |click| WeaponInput {
                    dice: game_stats.copies(6),
                    selected: 6,
                    click,
                    aim: Vec3::Y,
                    first_person: true,
                };
                for _ in 0..90 {
                    alice.update(
                        FIXED_DT,
                        &player,
                        false,
                        input(None),
                        &CombatContext {
                            world: &scene.world,
                            targets: &[],
                        },
                    );
                }
                ensure!(alice.ready_to_attack(6), "Dice not ready after equip");
                alice.update(
                    FIXED_DT,
                    &player,
                    false,
                    input(Some(true)),
                    &CombatContext {
                        world: &scene.world,
                        targets: &[],
                    },
                );
                let ticks = match case {
                    "dice-cast" => 0,
                    "dice-roll" => 75,
                    _ => 400,
                };
                for _ in 0..ticks {
                    alice.update(
                        FIXED_DT,
                        &player,
                        false,
                        input(None),
                        &CombatContext {
                            world: &scene.world,
                            targets: &[],
                        },
                    );
                }
                ensure!(
                    game_stats.spend_will(crate::combat::will_cost(6, true)),
                    "Dice fixture resource debit failed"
                );
                let snapshot = serde_json::to_value(alice.snapshot())?;
                if case == "dice-roll" {
                    ensure!(
                        snapshot["projectiles"]["dice"]["dice"]
                            .as_array()
                            .unwrap()
                            .len()
                            == 2,
                        "Dice fixture did not roll two dice"
                    );
                }
                if case == "dice-summon" {
                    ensure!(
                        !snapshot["projectiles"]["dice"]["demon"].is_null(),
                        "Dice fixture failed to summon"
                    );
                }
            }
            if case.starts_with("school2-scene-") {
                use crate::school2::cinema::Beat;
                let beat = if case.contains("growth") {
                    Beat::Growth
                } else if case.contains("final-") {
                    Beat::Final
                } else if case.contains("mushroom") {
                    Beat::Mushroom
                } else if case.contains("warp") {
                    Beat::Warp
                } else {
                    Beat::Spice
                };
                let (mut next_i, mut world, mut p, mut story) =
                    crate::school2::cinema_check::setup(assets, &scene.map, beat)?;
                let seconds = match case {
                    "school2-scene-mushroom-handout" => 17.,
                    "school2-scene-mushroom-boojums" => 30.,
                    "school2-scene-warp" => 4.5,
                    "school2-scene-warp-walk" => 7.5,
                    "school2-scene-spice-cabinet" => 22.,
                    "school2-scene-spice-return" => 33.,
                    "school2-scene-growth-fade" => 1.,
                    "school2-scene-growth-pour" => 3.,
                    "school2-scene-growth-grown" => 8.,
                    "school2-scene-final-mixing" => 8.,
                    "school2-scene-final-star" => 20.,
                    "school2-scene-final-departure" => 26.,
                    _ => 3.,
                };
                for _ in 0..(seconds / FIXED_DT) as usize {
                    crate::school2::cinema_check::tick(
                        &mut next_i,
                        &scene.map,
                        &mut world,
                        &mut p,
                        &mut story,
                        FIXED_DT,
                        false,
                    )?;
                }
                if case.ends_with("skipped") || case.ends_with("potion-collected") {
                    next_i.skip_cinematic(&scene.map, &mut world, &mut p, &mut story)?;
                }
                if case.ends_with("potion-collected") {
                    ensure!(
                        next_i
                            .school2
                            .as_mut()
                            .unwrap()
                            .quest
                            .collect("shrink_potion"),
                        "Partial potion reward fixture failed"
                    );
                }
                i = next_i;
                scene.world = world;
                player = p;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                level.npcs =
                    Npcs::load(assets, &scene.map, "skool2", None, false, false)?.snapshot();
                alice.reset(&player, 0.);
            }
            if let Some(s) = &i.school2 {
                game_stats.school_items = s.quest.items.clone();
            }
            if case.starts_with("school-") {
                let mut story = Story::load(assets, "skool1");
                if case == "school-walking-pupil" {
                    i = Interactions::load(&scene.map)?;
                    i.set_entry(assets, &scene.map, "skool1", None)?;
                    i.sync(&mut scene.world);
                    player = Player::spawn(&scene.world, scene.map.spawn().0)
                        .context("Pupil fixture spawn")?;
                } else {
                    let name = if case == "school-library-shelves" {
                        crate::school::cinema::library::SHELVES
                    } else if case == "school-library-recipe" {
                        crate::school::cinema::library::RECIPE
                    } else if case.starts_with("school-library-book-") {
                        crate::school::cinema::library::BOOK
                    } else if case.starts_with("school-theatre-") {
                        crate::school::cinema::THEATRE
                    } else {
                        crate::school::cinema::SHELF
                    };
                    (i, scene.world, player, story) =
                        crate::school::cinema_check::setup(assets, &scene.map, name)?;
                    let seconds = match case {
                        "school-library-shelves" => 8.,
                        "school-library-book-talk" => 5.,
                        "school-library-book-push" => 20.,
                        "school-library-book-drop" => 27.,
                        "school-library-book-skip" => 10.,
                        "school-library-recipe" => 2.,
                        "school-theatre-dialogue" => 24.,
                        "school-theatre-dialogue-gap" => 25.85,
                        "school-theatre-before-guards" => 42.5,
                        "school-theatre-guards" => 44.,
                        "school-theatre-disappear" => 47.8,
                        "school-theatre-skipped" => 33.4,
                        _ => 3.,
                    };
                    for _ in 0..(seconds / FIXED_DT) as usize {
                        crate::school::cinema_check::tick(
                            &mut i,
                            &scene.map,
                            &mut scene.world,
                            &mut player,
                            &mut story,
                            FIXED_DT,
                        )?;
                    }
                }
                if case == "school-theatre-skipped" || case == "school-library-book-skip" {
                    i.skip_cinematic(&scene.map, &mut scene.world, &mut player, &mut story)?;
                    crate::school::cinema_check::tick(
                        &mut i,
                        &scene.map,
                        &mut scene.world,
                        &mut player,
                        &mut story,
                        FIXED_DT,
                    )?;
                }
                let mut npcs = Npcs::load(assets, &scene.map, "skool1", None, false, false)?;
                for _ in 0..840 {
                    npcs.update(FIXED_DT, &scene.world, player.eye());
                }
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                level.npcs = npcs.snapshot();
                alice.reset(&player, 0.);
            }
            if case.starts_with("beyond-") {
                use crate::beyond::cinema::Beat;
                let beat = match case {
                    "beyond-demonstration" => Beat::Demonstration,
                    "beyond-reset" => Beat::Reset,
                    "beyond-solved" => Beat::Solved,
                    _ => Beat::Arches,
                };
                (i, scene.world, player) =
                    crate::beyond::cinema_check::setup(assets, &scene.map, beat)?;
                for _ in 0..(beat.duration() * 60.) as usize {
                    i.advance_school(FIXED_DT, &scene.map, &mut scene.world, &mut player)?;
                    i.update(
                        FIXED_DT,
                        &scene.map,
                        &mut scene.world,
                        &player,
                        Vec3::X,
                        false,
                    )?;
                }
                c.levels.get_mut(key).unwrap().interactions = i.snapshot();
                alice.reset(&player, 0.);
            }
            if case.starts_with("fortress-rage-") {
                let mut items = inventory::pickups(&scene.map, "fortress2", &catalog);
                let item = items
                    .iter()
                    .find(|p| p.id == "fortress2:37")
                    .context("Missing Rage save item")?;
                player =
                    Player::spawn(&scene.world, item.origin).context("Rage save fixture spawn")?;
                game_stats.powers.clear_effects();
                game_stats.invisible = 0.;
                inventory::collect(
                    &mut game_stats,
                    std::slice::from_ref(item),
                    player.feet,
                    &scene.world,
                );
                ensure!(
                    game_stats.collected.contains("fortress2:37"),
                    "Rage save collection failed"
                );
                let mut story = Story::load(assets, "fortress2");
                i.sync_pickups(&game_stats, &mut items, &mut story);
                i.beyond
                    .as_mut()
                    .unwrap()
                    .bind_rage_player(&mut player, 0.7);
                let frames = match case {
                    "fortress-rage-growing" => 900,
                    "fortress-rage-lowering" => 1740,
                    "fortress-rage-complete" => 2160,
                    _ => 90,
                };
                for _ in 0..frames {
                    i.advance_school(FIXED_DT, &scene.map, &mut scene.world, &mut player)?;
                    i.update(
                        FIXED_DT,
                        &scene.map,
                        &mut scene.world,
                        &player,
                        Vec3::X,
                        false,
                    )?;
                    if i.prepare_story(&mut story) {
                        story.tick(FIXED_DT, false);
                    }
                }
                if case == "fortress-rage-skipped" {
                    i.skip_cinematic(&scene.map, &mut scene.world, &mut player, &mut story)?;
                }
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                alice.reset(&player, 0.);
                alice.power_appearance(&game_stats);
                if let Some((time, finished)) = i.beyond.as_ref().and_then(|b| b.rage_scene()) {
                    alice.rage_scene(time, finished, &player);
                }
            }
            let mut friendly_aim = None;
            if case.starts_with("school2-enemy-") {
                i = interaction::school2_check::stage(
                    case,
                    assets,
                    &scene.map,
                    &mut scene.world,
                    &mut player,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                let mut story = Story::load(assets, "skool2");
                if case.ends_with("dialogue") {
                    story.trigger("dice_cat");
                    story.tick(FIXED_DT, false);
                }
                level.story = story.snapshot();
                let npcs = Npcs::load(assets, &scene.map, "skool2", None, false, false)?;
                npcs.check_school2_ownership()?;
                level.npcs = npcs.snapshot();
                game_stats.school_items = i.school2.as_ref().unwrap().quest.items.clone();
                game_stats.invisible = 0.;
                game_stats.notarget = false;
                game_stats.god = false;
                game_stats.select(0);
                friendly_aim = Some(Vec3::X);
                alice.reset(&player, 0.);
            }
            if case.starts_with("friendly-") {
                let (story, aim) = interaction::friendly::check::stage(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                let mut npcs = Npcs::load(
                    assets,
                    &scene.map,
                    &level.map,
                    level.entry.as_deref(),
                    false,
                    false,
                )?;
                for _ in 0..60 {
                    npcs.update(FIXED_DT, &scene.world, player.eye());
                }
                if !story.busy() {
                    let talk = i
                        .conversation(&scene.world, player.eye(), aim, &npcs, &story)
                        .context("Native friendly fixture has no actual Talk prompt")?;
                    if level.map == "gvillage" {
                        npcs.scene_hidden(&[talk.actor()]);
                        ensure!(
                            i.conversation(&scene.world, player.eye(), aim, &npcs, &story)
                                .is_none(),
                            "Hidden village actor retained Talk prompt"
                        );
                        npcs.scene_hidden(&[]);
                    }
                }
                level.npcs = npcs.snapshot();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(aim);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("cheshire-pool-") {
                let (hints, story) = crate::cheshire::check::stage(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                level.hints = hints.snapshot();
                friendly_aim = Some(vec3(
                    player.script_facing.cos(),
                    player.script_facing.sin(),
                    0.,
                ));
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("pool-pilot-")
                || case.starts_with("pool-arrival-")
                || case.starts_with("pool-boulder-")
            {
                let stage = if case.starts_with("pool-boulder-") {
                    crate::pool::boulder_check::stage
                } else if case.starts_with("pool-arrival-") {
                    crate::pool::arrival_check::stage
                } else {
                    crate::pool::pilot_check::stage
                };
                let story = stage(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::NEG_Y);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("potears2-scene-") {
                let story = crate::levels::potears2::stage_scene(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::Y);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("potears2-dialogue-") {
                let story = crate::levels::potears2::stage(case, assets, &mut i)?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
            }
            if case.starts_with("movement-") {
                c.levels.get_mut(key).unwrap().interactions = i.snapshot();
            }
            if case.starts_with("garden1-") {
                let story = crate::levels::garden1::stage(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                    &mut game_stats,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::X);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("garden2-") {
                let story = crate::levels::garden2::stage(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                    &mut game_stats,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::X);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("garden3-") {
                let story = crate::levels::garden3::stage(case, assets, &scene.map, &mut i, &mut scene.world, &mut player, &mut game_stats)?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::X);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("wchess1-") {
                let story = crate::levels::wchess1::stage(case, assets, &scene.map, &mut i, &mut scene.world, &mut player, &mut game_stats)?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();level.story = story.snapshot();
                friendly_aim = Some(Vec3::X);alice.reset(&player, player.script_facing);
            }
            if case.starts_with("wchess2-") {
                let story = crate::levels::wchess2::stage(case, assets, &scene.map, &mut i, &mut scene.world, &mut player, &mut game_stats)?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();level.story = story.snapshot();
                friendly_aim = Some(Vec3::X);alice.reset(&player, player.script_facing);
            }
            if case.starts_with("facade-") {
                let story = crate::levels::facade::stage_native(case, assets, &scene.map, &mut i, &mut scene.world, &mut player)?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::Y);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("garden4-") {
                let story = crate::levels::garden4::stage(case, assets, &scene.map, &mut i, &mut scene.world, &mut player, &mut game_stats)?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::X);
                alice.reset(&player, player.script_facing);
            }
            if case.starts_with("utemple-") {
                game_stats.arrival_grants("utemple");
                let story = crate::levels::utemple::stage(
                    case,
                    assets,
                    &scene.map,
                    &mut i,
                    &mut scene.world,
                    &mut player,
                )?;
                let level = c.levels.get_mut(key).unwrap();
                level.interactions = i.snapshot();
                level.story = story.snapshot();
                friendly_aim = Some(Vec3::NEG_Y);
                alice.reset(&player, player.script_facing);
            }
            let game = Game {
                current: key.into(),
                campaign: c,
                stats: game_stats,
                view: View {
                    position: player.eye(),
                    yaw: if let Some(aim) = friendly_aim {
                        aim.y.atan2(aim.x)
                    } else if case == "exit" {
                        -std::f32::consts::FRAC_PI_2
                    } else {
                        0.3
                    },
                    pitch: if let Some(aim) = friendly_aim {
                        aim.z.asin()
                    } else if case == "exit" {
                        0.
                    } else {
                        0.1
                    },
                    third_person: false,
                    flying: false,
                    fullbright: false,
                    spawn_landing: false,
                },
                player,
                recovery,
                character: alice.snapshot(),
            };
            let dir = root.join(case);
            let store = Store::new(dir.clone(), fingerprint.clone());
            store.write(Slot::Quick, &game)?;
            // This future runs from the original objects rebuilt via the same native loader.
            let mut live = Restored::build(assets, game.clone())?;
            same(&snapshot(&live), game.level()?, "writer level")?;
            same(&live.alice.snapshot(), &game.character, "writer character")?;
            advance(&mut live, case.starts_with("movement-"))?;
            fs::write(
                dir.join("future.json"),
                serde_json::to_vec(&future(&live)?)?,
            )?;
            fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
            println!(
                "PASS wrote {case}; resources={},{} collected={} visits={}",
                game.stats.sanity(),
                game.stats.will(),
                game.stats.collected.len(),
                game.campaign.levels.len()
            );
            next_frame().await;
        }
    } else {
        if selected.is_none() {
            legacy_saves(assets, root, &fingerprint)?;
        }
        for case in cases().into_iter().filter(|case| {
            selected
                .as_ref()
                .is_none_or(|prefix| case.starts_with(prefix))
        }) {
            let dir = root.join(case);
            let writer = fs::read_to_string(dir.join("writer.pid"))?;
            ensure!(
                writer != std::process::id().to_string(),
                "Reader must be a fresh process"
            );
            let store = Store::new(dir.clone(), fingerprint.clone());
            let game = store.read(Slot::Quick)?.game;
            let mut live = Restored::build(assets, game.clone())?;
            same(
                &snapshot(&live),
                game.level()?,
                "level, enemies, puzzle and dialogue",
            )?;
            same(
                &live.alice.snapshot(),
                &game.character,
                "Alice animation and actions",
            )?;
            // Every cached visit is rebuilt too, including first/return school separation.
            for level in game.campaign.levels.values() {
                let mut map =
                    crate::bsp::Bsp::parse(&assets.read(&format!("maps/{}.bsp", level.map))?)?;
                map.difficulty = game.stats.difficulty;
                let (i, story, hints) = level.restore_logic(assets, &map)?;
                same(
                    &i.snapshot(),
                    &level.interactions,
                    "cached puzzle and enemies",
                )?;
                same(&story.snapshot(), &level.story, "cached story")?;
                same(&hints.snapshot(), &level.hints, "cached Cheshire")?;
                let mut npcs = Npcs::load(
                    assets,
                    &map,
                    &level.map,
                    level.entry.as_deref(),
                    false,
                    false,
                )?;
                npcs.restore(&level.npcs)?;
                same(&npcs.snapshot(), &level.npcs, "cached NPCs")?;
            }
            advance(&mut live, case.starts_with("movement-"))?;
            let expected: serde_json::Value =
                serde_json::from_slice(&fs::read(dir.join("future.json"))?)?;
            same(
                &future(&live)?,
                &expected,
                "subsequent movement, attacks and dialogue timeline",
            )?;
            behavior(case, &mut live, assets)?;
            // A registered visit's own behaviour checks (no duplicate rewards, no dialogue
            // replay, scenes resume) run here, so a visit never edits `behavior` above.
            if let Some(check) = crate::levels::save_case(case).and_then(|c| c.behavior) {
                check(
                    &mut live.interactions,
                    &mut live.game.stats,
                    &mut live.story,
                )
                .with_context(|| format!("{case}: behaviour after reload"))?;
            }
            println!("PASS restarted {case}; writer={writer} reader={} exact state and continued simulation", std::process::id());
            if case == "school" {
                disk_failures(root, &fingerprint, &game, assets)?;
            }
            next_frame().await;
        }
    }
    println!(
        "PASS save {} checks",
        if write { "write" } else { "restart/read" }
    );
    Ok(())
}

fn legacy_saves(assets: &mut Assets, root: &Path, fingerprint: &str) -> Result<()> {
    let original = Path::new("private/duchess-native-reward/auto.json");
    if original.exists() {
        let dir = root.join("legacy-v8-movement");
        fs::create_dir_all(&dir)?;
        fs::copy(original, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let old = store.read(Slot::Quick)?.game;
        ensure!(
            old.level()?.map == "utemple",
            "Expected temple migration fixture"
        );
        ensure!(
            old.stats.turtle_air && old.player.breath.shell && old.player.breath.remaining() == 20.,
            "Legacy temple save lost its breathing upgrade"
        );
        let restored = Restored::build(assets, old)?;
        store.write(Slot::Auto, &restored.game)?;
        let next = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &restored.game.player,
            &next.game.player,
            "migrated breathing state",
        )?;
        same(
            &restored.game.stats,
            &next.game.stats,
            "migrated campaign resources",
        )?;
        println!("PASS actual v0.26 temple save gained 20-second air and retained inventory after resaving");
    }
    let original = Path::new("private/cinema-legacy-v6/quick.json");
    if original.exists() {
        let dir = root.join("legacy-v6-cinema");
        fs::create_dir_all(&dir)?;
        fs::copy(original, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let old = store.read(Slot::Quick)?.game;
        ensure!(
            !old.level()?.interactions.has_pandemonium_cinema(),
            "Expected v0.24 cinematic migration fixture"
        );
        let restored = Restored::build(assets, old.clone())?;
        same(
            &old.stats,
            &restored.game.stats,
            "legacy cinematic resources",
        )?;
        same(
            &old.player,
            &restored.game.player,
            "legacy cinematic attachment",
        )?;
        store.write(Slot::Auto, &restored.game)?;
        let next = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &snapshot(&restored),
            &snapshot(&next),
            "upgraded cinematic state",
        )?;
        println!("PASS actual v0.24 airship save upgraded without replaying boarding or changing inventory");
    }
    let original = Path::new("private/pand-legacy-v4/quick.json");
    if original.exists() {
        let dir = root.join("legacy-v4-pandemonium");
        fs::create_dir_all(&dir)?;
        fs::copy(original, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let old = store.read(Slot::Quick)?.game;
        ensure!(
            !old.level()?.interactions.has_pandemonium(),
            "Expected actual v0.22 Pandemonium save"
        );
        let restored = Restored::build(assets, old.clone())?;
        same(
            &old.stats,
            &restored.game.stats,
            "legacy Pandemonium resources",
        )?;
        same(
            &old.player,
            &restored.game.player,
            "legacy Pandemonium player",
        )?;
        same(
            &old.level()?.npcs,
            &restored.npcs.snapshot(),
            "legacy Pandemonium NPC identities",
        )?;
        ensure!(
            restored.interactions.pandemonium.is_some()
                && restored
                    .interactions
                    .encounters
                    .as_ref()
                    .unwrap()
                    .actors
                    .len()
                    == 7,
            "Missing migrated Pandemonium components"
        );
        store.write(Slot::Auto, &restored.game)?;
        let next = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &snapshot(&restored),
            &snapshot(&next),
            "upgraded Pandemonium save",
        )?;
        println!("PASS actual v0.22 Pandemonium save migrated, resaved and reloaded");
    }
    // The retained format-7 Duchess save (F2): a potears3 visit that predates its controller.
    // Skipped, like every retained fixture, when the private file is missing.
    let original = Path::new("private/duchess-legacy-v7/quick.json");
    if original.exists() {
        let dir = root.join("legacy-v7-duchess");
        fs::create_dir_all(&dir)?;
        fs::copy(original, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let old = store.read(Slot::Quick)?.game;
        ensure!(
            old.level()?.map == "potears3" && !old.level()?.interactions.has_duchess(),
            "Expected the format-7 Duchess fixture"
        );
        let restored = Restored::build(assets, old.clone())?;
        same(&old.stats, &restored.game.stats, "legacy Duchess resources")?;
        ensure!(
            restored.interactions.duchess.is_some(),
            "Missing migrated Duchess controller"
        );
        store.write(Slot::Auto, &restored.game)?;
        let next = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &snapshot(&restored),
            &snapshot(&next),
            "upgraded Duchess save",
        )?;
        println!("PASS retained format-7 Duchess save migrated, resaved and reloaded");
    }

    for case in ["school", "battle"] {
        let original = std::path::PathBuf::from(format!("private/event-legacy-v1-{case}.json"));
        if !original.exists() {
            continue;
        }
        let dir = root.join(format!("legacy-{case}"));
        fs::create_dir_all(&dir)?;
        fs::copy(original, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let original = store.read(Slot::Quick)?.game;
        ensure!(
            original
                .campaign
                .levels
                .values()
                .all(|l| !l.interactions.has_shared_state()),
            "Legacy fixture is not v1"
        );
        let restored = Restored::build(assets, original.clone())?;
        same(
            &restored.game.stats,
            &original.stats,
            "legacy resources and inventory",
        )?;
        same(&restored.game.player, &original.player, "legacy player")?;
        ensure!(
            restored
                .game
                .campaign
                .levels
                .values()
                .all(|l| l.interactions.has_shared_state()),
            "A cached visit escaped migration"
        );
        store.write(Slot::Auto, &restored.game)?;
        let reloaded = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &snapshot(&restored),
            &snapshot(&reloaded),
            "migrated save written/read in current format",
        )?;
        println!("PASS original v0.19 {case} save: all four visits migrated, resources retained, current-format resave/reload");
    }
    for case in ["school", "battle"] {
        let original = std::path::PathBuf::from(format!("private/dice-legacy-v2-{case}.json"));
        if !original.exists() {
            continue;
        }
        let dir = root.join(format!("legacy-v2-{case}"));
        fs::create_dir_all(&dir)?;
        fs::copy(original, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let old = store.read(Slot::Quick)?.game;
        let restored = Restored::build(assets, old.clone())?;
        same(&old.stats, &restored.game.stats, "v0.20 player resources")?;
        // This save predates the power-up presentation, footing and idle-performance blocks,
        // so they load as defaults. Restoring deliberately initialises the power-up block to
        // the stats' current form without replaying its pickup performance
        // (Character::power_appearance), so the raw fixture block can never equal the restored
        // one. Everything else must come back exactly as saved, and the migrated block is
        // pinned explicitly rather than skipped.
        let mut before = serde_json::to_value(&old.character)?;
        let mut after = serde_json::to_value(restored.alice.snapshot())?;
        let power = after
            .as_object_mut()
            .and_then(|a| a.remove("power"))
            .context("Restored Alice lost the power-up presentation")?;
        before.as_object_mut().and_then(|b| b.remove("power"));
        same(
            &before,
            &after,
            "v0.20 Alice/projectile state outside the power-up presentation",
        )?;
        ensure!(
            power["initialized"] == true
                && power["performing"] == false
                && power["form"] == serde_json::to_value(crate::character::power_form(&old.stats))?
                && power["age"].as_f64().is_some_and(|a| a >= 2.5),
            "Restored v0.20 power-up presentation replays a pickup or misses its form"
        );
        same(
            &old.character.migrated(&old.stats),
            &restored.alice.snapshot(),
            "v0.20 Alice/projectile state after its documented migration",
        )?;
        // A current-format resave carries the migrated Alice block, which must load unchanged.
        let mut resaved = restored.game.clone();
        resaved.character = restored.alice.snapshot();
        store.write(Slot::Auto, &resaved)?;
        let reloaded = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &restored.alice.snapshot(),
            &reloaded.alice.snapshot(),
            "v0.20 resaved with Dice state",
        )?;
        same(
            &resaved.character,
            &reloaded.alice.snapshot(),
            "v0.20 current-format Alice block",
        )?;
        println!(
            "PASS original v0.20 {case} save retained state and resaved/reloaded in current format"
        );
    }
    let old_return = Path::new("private/return-legacy-v5/quick.json");
    if old_return.exists() {
        let dir = root.join("legacy-v5-return");
        fs::create_dir_all(&dir)?;
        fs::copy(old_return, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.to_owned());
        let old = store.read(Slot::Quick)?.game;
        let restored = Restored::build(assets, old.clone())?;
        same(&old.stats, &restored.game.stats, "legacy return inventory")?;
        same(&old.player, &restored.game.player, "legacy return player")?;
        ensure!(
            restored
                .interactions
                .school
                .as_ref()
                .unwrap()
                .return_visit
                .is_some(),
            "Return migration missing"
        );
        store.write(Slot::Auto, &restored.game)?;
        let next = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &snapshot(&restored),
            &snapshot(&next),
            "upgraded return save",
        )?;
        println!("PASS actual v0.23 return save migrated, resaved and reloaded");
    }
    let legacy = Path::new("private/ladybug-legacy-v3/quick.json");
    if legacy.exists() {
        let dir = root.join("legacy-v3-potears1");
        fs::create_dir_all(&dir)?;
        fs::copy(legacy, dir.join("quick.json"))?;
        let store = Store::new(dir, fingerprint.into());
        let old = store.read(Slot::Quick)?.game;
        ensure!(
            !old.level()?.interactions.has_encounters(),
            "Expected actual old Pool of Tears save"
        );
        let restored = Restored::build(assets, old.clone())?;
        same(
            &old.stats,
            &restored.game.stats,
            "legacy Pool of Tears resources",
        )?;
        if old.level()?.interactions.has_pool() {
            same(
                &old.player,
                &restored.game.player,
                "legacy Pool of Tears player",
            )?;
        } else {
            // The leaf-transport upgrade deliberately starts pre-transport saves
            // at the playable entrance, retaining inventory and encounter history.
            let entrance = interaction::spawn(&restored.scene.map, old.level()?.entry.as_deref()).0;
            let expected = Player::spawn(&restored.scene.world, entrance)
                .context("Legacy Pool of Tears entrance obstructed")?;
            same(
                &expected,
                &restored.game.player,
                "legacy Pool of Tears entrance migration",
            )?;
        }
        let old_npcs = serde_json::to_value(&old.level()?.npcs)?;
        let new_npcs = serde_json::to_value(restored.npcs.snapshot())?;
        for old_actor in old_npcs["actors"].as_array().unwrap() {
            let actor = new_npcs["actors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["spawn"] == old_actor["spawn"])
                .context("Legacy actor lost")?;
            let mut a = actor.clone();
            let mut b = old_actor.clone();
            a.as_object_mut().unwrap().remove("model");
            b.as_object_mut().unwrap().remove("model");
            same(&a, &b, "legacy actor state")?;
        }
        ensure!(
            restored
                .interactions
                .encounters
                .as_ref()
                .unwrap()
                .actors
                .len()
                == 12,
            "Missing upgraded Ladybugs"
        );
        store.write(Slot::Auto, &restored.game)?;
        let next = Restored::build(assets, store.read(Slot::Auto)?.game)?;
        same(
            &snapshot(&restored),
            &snapshot(&next),
            "upgraded Pool of Tears save",
        )?;
        println!(
            "PASS actual v0.21 Pool of Tears save migrated and resaved/reloaded with Ladybug state"
        );
    }
    Ok(())
}

fn behavior(case: &str, r: &mut Restored, assets: &mut Assets) -> Result<()> {
    if case.starts_with("pool-boulder-") {
        return crate::pool::boulder_check::behavior(r);
    }
    if case.starts_with("pool-arrival-") {
        crate::pool::arrival_check::behavior(r)?;
    }
    if case.starts_with("cheshire-pool-") {
        crate::cheshire::check::behavior(r)?;
    }
    if case.starts_with("friendly-village-repeat-") && !case.ends_with("ready") {
        let before = serde_json::to_value(r.interactions.snapshot())?;
        let count = r.story.completed;
        for _ in 0..100 {
            r.story.tick(0.25, true);
            r.interactions.sync_cinematic_story(&r.story);
        }
        ensure!(
            !r.story.busy() && r.story.take_completed().is_empty() && r.story.completed == count,
            "Restarted repeated speech emitted a quest callback"
        );
        ensure!(
            before == serde_json::to_value(r.interactions.snapshot())?,
            "Restarted repeat changed world state"
        );
    }
    if case.starts_with("beyond-") {
        let before = r.interactions.beyond.as_ref().unwrap().state.clone();
        ensure!(r.interactions.scripted(), "Restored puzzle camera absent");
        ensure!(
            r.interactions.skip_cinematic(
                &r.scene.map,
                &mut r.scene.world,
                &mut r.game.player,
                &mut r.story
            )?,
            "Restored camera cannot skip"
        );
        let after = &r.interactions.beyond.as_ref().unwrap().state;
        ensure!(
            before.solved == after.solved
                && before.notes == after.notes
                && before.lever_used == after.lever_used
                && before.age == after.age
                && before.raised == after.raised
                && before.last_open == after.last_open
                && before.step_slots == after.step_slots
                && before.walkway == after.walkway,
            "Restored skip changed puzzle progress"
        );
        ensure!(
            !r.interactions.scripted()
                && r.game.player.script_motion == 0
                && r.game.player.velocity == Vec3::ZERO
                && r.game.player.grounded
                && r.scene.world.body_clear(r.game.player.feet),
            "Restored camera unsafe handoff"
        );
        return Ok(());
    }

    if case.starts_with("item-") {
        match case {
            "item-rage" | "item-tea" | "item-glass" => {
                ensure!(
                    r.game.stats.active_power().is_some(),
                    "Restored power-up disappeared"
                );
                r.game.stats.update(50.);
                ensure!(
                    r.game.stats.active_power().is_none(),
                    "Power-up failed to expire after reload"
                );
            }
            "item-watch" => ensure!(
                r.game.stats.powers.stopped > 12. && r.game.stats.watch().is_err(),
                "Watch recharge reset"
            ),
            "item-watch-expiry" => ensure!(
                r.game.stats.powers.stopped == 0. && r.game.stats.watch().is_err(),
                "Watch did not resume the world"
            ),
            "item-watch-recharge" => ensure!(
                r.game.stats.watch().is_ok(),
                "Watch did not finish recharging"
            ),
            "item-drop-live" => ensure!(
                r.game.stats.loot[&r.game.current].drops[0].grade == crate::loot::Grade::Medium,
                "Restored essence did not decay"
            ),
            _ => {
                let before = r.interactions.loot_sources();
                crate::loot::tick(
                    &mut r.game.stats,
                    &r.game.current,
                    FIXED_DT,
                    &before,
                    &before,
                    r.game.player.feet,
                    &r.scene.world,
                );
                ensure!(
                    r.game.stats.loot[&r.game.current].drops.is_empty(),
                    "A spent reward respawned"
                );
            }
        }
    }
    if case == "return" || case.starts_with("return-") {
        use crate::school_return::Phase as P;
        let s = r.interactions.school.as_ref().unwrap();
        let state = s.return_visit.as_ref().unwrap();
        ensure!(
            state.phase
                == match case {
                    "return-lift" | "return-star" | "return-star-close" | "return-star-skipped" =>
                        P::Rising,
                    "return-globe" => P::Opening,
                    "return-takeoff" | "return-skipped" => P::Shrinking,
                    "return-complete" => P::Complete,
                    "return-potion" => P::Drinking,
                    "return-shrink" => P::Shrinking,
                    _ => P::Locked,
                },
            "Wrong resumed return phase"
        );
        ensure!(
            r.game.stats.school_items.star == (case == "return")
                && r.game.stats.school_items.potion
                    == [
                        "return",
                        "return-lift",
                        "return-star",
                        "return-star-close",
                        "return-star-skipped",
                        "return-globe"
                    ]
                    .contains(&case),
            "Restored quest rewards were lost or duplicated"
        );
        ensure!(
            r.game.stats.selected() == 2
                && r.game.stats.sanity() == 37.
                && r.game.stats.will() == 41.
                && r.game.stats.copies(6) == 2,
            "Return save altered weapon selection, copies or resources"
        );
        let checkpoint = Player::spawn(&r.scene.world, vec3(692., 4100., 1072.));
        ensure!(
            checkpoint.is_some(),
            "Observatory retry checkpoint obstructed"
        );
    }
    if case.starts_with("ladybug-") {
        let cast = r.interactions.encounters.as_ref().unwrap();
        ensure!(cast.actors.len() == 12, "Duplicated Ladybug actor");
        if case == "ladybug-delay" {
            ensure!(
                cast.actors.iter().any(|a| a.name == "x_lady8" && a.active),
                "Delayed Ladybug lost after restart"
            );
        } else {
            let b = cast
                .actors
                .iter()
                .find_map(|a| match &a.enemy {
                    Enemy::Ladybug(b) if a.name == "lady3" => Some(b),
                    _ => None,
                })
                .unwrap();
            ensure!(b.drops >= 1, "Saved bomb release lost");
            if case == "ladybug-dead" {
                ensure!(
                    b.health == 0. && b.drops == 1,
                    "Dead Ladybug revived or attacked"
                );
            }
        }
    }
    if case == "school" {
        let before = r.game.stats.clone();
        let catalog = Catalog::load(assets)?;
        for p in inventory::pickups(&r.scene.map, "skool1", &catalog)
            .into_iter()
            .filter(|p| before.collected.contains(&p.id))
        {
            ensure!(
                inventory::collect(
                    &mut r.game.stats,
                    std::slice::from_ref(&p),
                    p.origin,
                    &r.scene.world
                )
                .is_empty(),
                "Collected pickup reappeared"
            );
        }
        let s = r.interactions.school.as_mut().unwrap();
        s.collect_secret(&mut r.game.stats, vec3(624., 2581., -240.), &r.scene.world);
        same(&before, &r.game.stats, "one-shot pickups and secret reward")?;
        ensure!(
            !s.trigger_enabled("", "", Some("skool2")),
            "Partial library bypasses exit gate"
        );
        let before_story = r.story.snapshot();
        r.story.trigger("Theatre_Cinematic");
        same(
            &r.story.snapshot(),
            &before_story,
            "one-shot dialogue history",
        )?;
        r.interactions.reset_contacts();
        r.interactions.completed_dialogue("Theatre_Cinematic");
        r.interactions.activate_enemies();
        ensure!(
            r.interactions
                .encounters
                .as_ref()
                .unwrap()
                .actors
                .iter()
                .any(|a| match &a.enemy {
                    Enemy::Guard(g) => g.health == 0.,
                    Enemy::Boojum(b) => b.health == 0.,
                    Enemy::Ladybug(b) => b.health == 0.,
                }),
            "Dead enemy respawned"
        );
    }
    if let Some(s) = &mut r.interactions.school2 {
        ensure!(
            s.trigger_enabled("exit_trigger", "", true) == (case == "exit"),
            "Incomplete potion quest permits exit"
        );
        match case {
            "battle" => {
                ensure!(
                    s.boojums[0].health == 0. && s.boojums[1].health > 0.,
                    "Boojum health reset"
                );
                for b in &mut s.boojums[..3] {
                    b.hit(1000.);
                }
                s.update(FIXED_DT, &r.scene.world, r.game.player.feet, false);
                ensure!(
                    s.quest.stage == Stage::Laboratory,
                    "Restored battle cannot finish"
                );
            }
            "growing" => {
                ensure!(
                    !s.quest.collect("ig_lollypop"),
                    "Lollipop available before growth"
                );
                for _ in 0..60 {
                    s.quest.tick(0.1, 3, 2);
                }
                ensure!(s.quest.collect("ig_lollypop"), "Growth did not resume");
                ensure!(!s.quest.collect("ig_lollypop"), "Lollipop duplicated");
            }
            "mixing" | "rewards" => {
                for _ in 0..60 {
                    s.quest.tick(0.1, 3, 2);
                }
                if case == "mixing" {
                    ensure!(s.quest.collect("shrink_potion"), "Mixing did not resume");
                }
                ensure!(!s.quest.collect("shrink_potion"), "Potion duplicated");
                ensure!(
                    !s.trigger_enabled("exit_trigger", "", true),
                    "Potion alone opens exit"
                );
                ensure!(s.quest.collect("lucky_star"), "Star reward unavailable");
                ensure!(
                    s.trigger_enabled("exit_trigger", "", true),
                    "Both rewards fail to open exit"
                );
            }
            _ => (),
        }
    }
    Ok(())
}
fn disk_failures(root: &Path, fingerprint: &str, game: &Game, assets: &mut Assets) -> Result<()> {
    let dir = root.join("failures");
    let store = Store::new(dir.clone(), fingerprint.into());
    store.write(Slot::Quick, game)?;
    let first = fs::read(dir.join("quick.json"))?;
    let mut newer = game.clone();
    newer.stats.damage(1.);
    store.write(Slot::Quick, &newer)?;
    same(
        &store.read(Slot::Quick)?.game.stats,
        &newer.stats,
        "atomic overwrite",
    )?;
    fs::write(dir.join("quick.json"), b"{interrupted")?;
    let recovered = store.read(Slot::Quick)?;
    ensure!(recovered.backup, "Damaged primary did not recover backup");
    same(&recovered.game.stats, &game.stats, "previous save backup")?;
    // A new primary must not copy the damaged one over the valid backup.
    store.write(Slot::Quick, &newer)?;
    fs::write(dir.join("quick.json"), b"{damaged again")?;
    same(
        &store.read(Slot::Quick)?.game.stats,
        &game.stats,
        "backup after repairing primary",
    )?;
    let isolated = root.join("invalid");
    fs::create_dir_all(&isolated)?;
    let reject = Store::new(isolated.clone(), fingerprint.into());
    for field in ["version", "game_data", "checksum"] {
        let mut e: serde_json::Value = serde_json::from_slice(&first)?;
        e[field] = if field == "version" {
            999.into()
        } else {
            "invalid".into()
        };
        fs::write(isolated.join("quick.json"), serde_json::to_vec(&e)?)?;
        ensure!(
            reject.read(Slot::Quick).is_err(),
            "Invalid {field} accepted"
        );
    }
    fs::write(isolated.join("quick.json"), &first[..first.len() / 2])?;
    ensure!(reject.read(Slot::Quick).is_err(), "Truncated save accepted");
    fs::write(isolated.join("quick.json"), &first)?;
    fs::write(isolated.join("quick.json.interrupted.tmp"), b"unfinished")?;
    ensure!(
        reject.read(Slot::Quick).is_ok(),
        "Temporary write hides valid primary"
    );
    let mut bad = game.clone();
    bad.view.pitch = 99.;
    ensure!(
        reject.write(Slot::Quick, &bad).is_err(),
        "Invalid state saved"
    );
    same(
        &reject.read(Slot::Quick)?.game.stats,
        &game.stats,
        "failed save preserves primary",
    )?;
    bad = game.clone();
    bad.player.feet = Vec3::ZERO;
    // A valid envelope cannot bypass rebuilding/validating actual level identities.
    bad.campaign.levels.get_mut(&bad.current).unwrap().entry = Some("missing_entrance".into());
    ensure!(
        Restored::build(assets, bad).is_err(),
        "Missing entrance accepted"
    );
    println!("PASS atomic replacement, backup repair, truncation, version/data/checksum rejection and failed-save preservation");
    Ok(())
}

/// The registry's save-case plumbing (F1 added the slice, F2 exercises it with a synthetic
/// registration): no registered visit exists yet, so the real table adds nothing to `CASES`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::{self, synthetic, Registration};
    use std::collections::BTreeSet;

    const SYNTHETIC: &[&Registration] = &[&synthetic::REGISTRATION];

    #[test]
    fn registered_cases_follow_the_legacy_ones_and_are_selected_by_their_id_prefix() {
        assert_eq!(cases_in(&[]), CASES);
        // Registrations append cases; every legacy case retains its position.
        let shipped = cases();
        assert_eq!(&shipped[..CASES.len()], CASES);
        assert_eq!(shipped.iter().collect::<BTreeSet<_>>().len(), shipped.len());
        let all = cases_in(SYNTHETIC);
        assert_eq!(&all[..CASES.len()], CASES);
        assert_eq!(&all[CASES.len()..], ["garden1-open", "garden1-shut"]);
        assert_eq!(all.iter().collect::<BTreeSet<_>>().len(), all.len());
        // `LOOKING_GLASS_SAVE_CASE=garden1-` runs a visit's cases and nothing else.
        let chosen = |prefix: &str| {
            all.iter()
                .copied()
                .filter(|c| c.starts_with(prefix))
                .collect::<Vec<_>>()
        };
        assert_eq!(chosen("garden1-"), ["garden1-open", "garden1-shut"]);
        assert!(chosen("garden2-").is_empty());
        // The reservation table accepts them: cases of a visit are named for its id.
        levels::reservations::validate(SYNTHETIC).unwrap();
    }
    #[test]
    fn the_writer_finds_each_cases_visit_and_hooks() {
        let case = |name| levels::save_case_in(SYNTHETIC, name).unwrap();
        assert_eq!(case("garden1-open").visit, "garden1$first");
        assert_eq!(crate::save::visit_key("garden1", None), "garden1$first");
        assert_eq!(levels::save_visits_in(SYNTHETIC), [("garden1", None)]);
        assert!(case("garden1-open").stage.is_some() && case("garden1-open").behavior.is_some());
        assert!(case("garden1-shut").stage.is_none() && case("garden1-shut").behavior.is_none());
        assert!(levels::save_case_in(SYNTHETIC, "school").is_none());
    }
    #[test]
    fn a_staged_case_survives_the_round_trip_and_its_behaviour_check_can_fail() {
        let map = synthetic::map();
        let case = levels::save_case_in(SYNTHETIC, "garden1-open").unwrap();
        // The writer stages the case on its visit and saves it ...
        let mut writer = synthetic::served(&map, synthetic::first());
        (case.stage.unwrap())(&mut writer, &map).unwrap();
        let saved = writer.snapshot();
        // ... a fresh process rebuilds the visit and restores exactly that state ...
        let mut reader = synthetic::served(&map, synthetic::first());
        reader.restore(&saved, &map).unwrap();
        assert_eq!(
            serde_json::to_value(reader.snapshot()).unwrap(),
            serde_json::to_value(&saved).unwrap()
        );
        // ... and the case's behaviour check passes there, but not on a visit that lost it.
        let (mut stats, mut story) = (Stats::for_level("skool1", None), Story::default());
        let check = case.behavior.unwrap();
        check(&mut reader, &mut stats, &mut story).unwrap();
        let mut lost = synthetic::served(&map, synthetic::first());
        assert!(check(&mut lost, &mut stats, &mut story).is_err());
    }
}

//! Explicitly staged native Store coverage; the input route proves progression separately.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for (name, phase) in [
        ("arrival", Phase::Arrival),
        ("fight", Phase::Fight),
        ("waves", Phase::Fight),
        ("death", Phase::Fight),
        ("lifts", Phase::Victory),
        ("cat", Phase::WatchCat),
        ("gryphon", Phase::Gryphon),
        ("done", Phase::Done),
    ] {
        let dir = std::path::PathBuf::from(format!("private/hatter2/saves/{name}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "hatter2", None)?;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Encounter>())
                .unwrap();
            o.saved.initialized = true;
            o.saved.phase = phase;
            o.saved.time = 12.;
            o.saved.clock = 64.;
            if phase != Phase::Arrival {
                o.saved.boss.at = o.data.points["tower_down"].translation;
                o.saved.cycle = 3000;
                o.saved.bridge = Some(1.);
                o.saved.blade = true;
                r.stats.collected.insert("hatter2:11".into());
            }
            if phase == Phase::Fight {
                o.saved.boss.set(battle::Action::Cup);
                o.saved.boss.time = 0.6;
                o.saved.boss.shots.push(battle::Shot {
                    at: o.saved.boss.at + vec3(0., -150., 140.),
                    velocity: vec3(0., -400., 120.),
                    age: 0.4,
                    syringe: false,
                    id: 1,
                    ended: None,
                });
                o.saved.boss.serial = 1;
            }
            if name == "waves" {
                o.saved.cycle = 7000;
                o.saved.boss.at = o.data.points["tower_up"].translation;
                o.spawn_wave();
                for w in &mut o.saved.waves {
                    w.launch = 1.5;
                    w.actor.feet = w.to;
                }
            }
            if name == "death" {
                o.saved.boss.hit(Hit {
                    id: BASE,
                    damage: 3000.,
                    kind: crate::combat::DamageKind::Knife,
                    knockback: Vec3::ZERO,
                });
                o.saved.boss.set(battle::Action::Malfunction);
                o.saved.boss.malfunctions = 1;
                o.saved.boss.time = 0.4;
            }
            if o.won() {
                o.saved.boss.health = 0.;
                o.saved.boss.malfunctions = 3;
                o.saved.boss.set(battle::Action::Gone);
                o.saved.lifts = 1.5;
            }
            if matches!(phase, Phase::WatchCat | Phase::Gryphon | Phase::Done) {
                o.saved.watch = true;
                r.stats.apply(crate::inventory::PickupKind::Weapon(9), 1.);
                r.stats.collected.insert(o.data.watch_key.clone());
            }
            if matches!(phase, Phase::Gryphon | Phase::Done) {
                o.saved.door = 1.4;
                o.saved.scene_from = o.data.points["end_dest"];
                o.saved.talk_done = true;
                o.saved.after_talk = 4.;
            }
            if phase == Phase::Done {
                o.finish_escape();
            }
            o.rebuild(&r.map)?;
            r.interactions.sync(&mut r.world);
            let position = if matches!(phase, Phase::Gryphon | Phase::Done) {
                vec3(1248., 1280., 48.)
            } else if name == "lifts" {
                vec3(5568., 988., 8. + 392. * scene::lift(1.5) + 48.)
            } else if phase == Phase::Arrival {
                crate::interaction::spawn(&r.map, None).0
            } else {
                vec3(5500., 0., 48.)
            };
            r.player = Player::spawn(&r.world, position).context("Hatter save fixture blocked")?;
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "hatter2", None, false, false)?.snapshot();
            alice.reset(&r.player, 0.);
            alice.power_appearance(&r.stats);
            let key = cp.level.key();
            let mut campaign = Campaign::default();
            campaign.levels.insert(key.clone(), cp.level);
            let game = Game {
                current: key,
                campaign,
                stats: r.stats,
                view: View {
                    position: r.player.eye(),
                    yaw: 0.,
                    pitch: 0.,
                    third_person: true,
                    flying: false,
                    fullbright: false,
                    spawn_landing: false,
                },
                player: r.player,
                recovery: Default::default(),
                character: alice.snapshot(),
            };
            store.write(Slot::One, &game)?;
            std::fs::write(dir.join("writer.pid"), std::process::id().to_string())?;
        } else {
            ensure!(
                std::fs::read_to_string(dir.join("writer.pid"))? != std::process::id().to_string(),
                "Reader must be a fresh process"
            );
        }
        let game = store.read(Slot::One)?.game;
        let restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "{name} changed world on restore"
        );
        ensure!(
            serde_json::to_value(restored.story.snapshot())?
                == serde_json::to_value(&game.level()?.story)?,
            "{name} changed dialogue on restore"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "{name} moved Alice on restore"
        );
        let o = restored
            .interactions
            .levels
            .iter()
            .find_map(|s| s.ctl.downcast_ref::<Encounter>())
            .context("Missing Hatter after restore")?;
        ensure!(o.saved.phase == phase, "{name} lost encounter phase");
        let _art = art::Art::load(a, o)?;
        println!(
            "PASS Hatter {} {name}: world, actor, projectiles, rewards and dialogue",
            if write {
                "Store write"
            } else {
                "fresh-process read"
            }
        );
        next_frame().await;
    }
    Ok(())
}

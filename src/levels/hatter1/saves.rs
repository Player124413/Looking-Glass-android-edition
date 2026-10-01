//! Separate-process native Store fixtures; these are staged persistence tests.
use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for phase in 0..8 {
        let dir = std::path::PathBuf::from(format!("private/hatter1-work/saves/{phase}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "hatter1", None)?;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Clockwork>())
                .unwrap();
            o.saved.initialized = true;
            o.saved.age = 100.;
            o.saved.levers[0] = Some(99.);
            if phase >= 1 {
                o.saved.levers[1] = Some(96.);
                o.saved.levers[2] = Some(98.);
                let k = o.objects.iter().position(|s| s.name == "sink2").unwrap();
                o.saved.sinks[k] = 76.;
            }
            if phase >= 2 {
                o.saved.hare = Some(30.);
                o.saved.levers[3] = Some(40.);
                o.saved.port = Some(60.);
                o.saved.gryphon = Some(70.);
                o.saved.extendo = Some(80.);
                o.saved.cubes = [true, false, true, false];
            }
            if phase >= 3 {
                o.saved.cubes = [true; 4];
                o.saved.clock = Some(98.);
            }
            if phase == 4 {
                o.saved.levers[4] = Some(80.);
                o.begin(Kind::Stop);
                let s = o.saved.scene.as_mut().unwrap();
                s.time = 16.;
                s.home = Some(Transform {
                    translation: r.player.feet,
                    rotation: Quat::IDENTITY,
                });
            }
            if phase == 5 {
                o.saved.levers[4] = Some(80.);
                o.saved.stopped = true;
            }
            if phase == 6 {
                o.saved = Saved::new(o.objects.len());
                o.saved.initialized = true;
                o.saved.age = 50.;
                o.saved.levers = [Some(0.), Some(1.), Some(3.), None, None];
                o.begin(Kind::Hare);
                let scene = o.saved.scene.as_mut().unwrap();
                scene.time = 40.;
                scene.line = 3;
                scene.line_time = 0.8;
                scene.starts[3] = 39.2;
                scene.home = Some(Transform {
                    translation: r.player.feet,
                    rotation: Quat::IDENTITY,
                });
                o.prepare_scene_story(&mut r.story);
            }
            if phase == 7 {
                o.saved = Saved::new(o.objects.len());
                o.saved.initialized=true;
                o.rebuild(&r.map)?;
                r.world.set_dynamic(o.colliders());
                let at=o.levers[0];
                let feet=r.world.actor_footing(at.point(vec3(-64.,0.,60.)),PLAYER_CENTER,PLAYER_HALF,128.).context("Lever save footing")?;
                r.player=Player::new(feet);r.player.grounded=true;
                o.start_pull(0,&r.world,&r.player);
                for _ in 0..240 {o.advance(1./120.,&r.map,&mut r.world,&mut r.player,&[])?;}
                ensure!(o.saved.pull.as_ref().is_some_and(|p|p.started),"Lever save must be mid-action");
            }
            o.rebuild(&r.map)?;
            r.interactions.sync(&mut r.world);
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "hatter1", None, false, false)?.snapshot();
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
                "Reader must be a separate process"
            );
        }
        let game = store.read(Slot::One)?.game;
        let restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "Clockwork phase {phase} changed on restore"
        );
        ensure!(
            serde_json::to_value(restored.story.snapshot())?
                == serde_json::to_value(&game.level()?.story)?,
            "Clockwork phase {phase} replayed speech"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "Clockwork phase {phase} moved Alice on restore"
        );
        println!(
            "PASS Clockwork Store {} phase {phase}",
            if write { "write" } else { "fresh process read" }
        );
        next_frame().await;
    }
    Ok(())
}

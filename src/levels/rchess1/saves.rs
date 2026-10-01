use super::*;
use crate::save::{Campaign, Game, Slot, Store, View};
pub(super) async fn run(a: &mut Assets, write: bool) -> Result<()> {
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    for (name, phase) in [
        ("approach", Phase::Approach),
        ("beheading", Phase::Beheading),
        ("intro", Phase::Intro),
        ("fight", Phase::Fight),
        ("dead", Phase::Fight),
        ("killed", Phase::Killed),
        ("done", Phase::Done),
    ] {
        let dir = std::path::PathBuf::from(format!("private/rchess1/saves/{name}"));
        let store = Store::new(dir.clone(), fingerprint.clone());
        if write {
            let mut r = crate::route::Route::new(a, "rchess1", None)?;
            let o = r
                .interactions
                .levels
                .iter_mut()
                .find_map(|s| s.ctl.downcast_mut::<Encounter>())
                .unwrap();
            o.saved.phase = phase;
            o.saved.time = 3.;
            if phase == Phase::Fight {
                o.saved.boss.set(battle::Action::Grenade);
                o.saved.boss.time = 0.7;
                o.saved.boss.health = 750.;
                o.saved.boss.shots.push(battle::Shot {
                    id: 1,
                    kind: battle::Kind::Ball,
                    at: o.saved.boss.at + vec3(-100., 0., 100.),
                    velocity: -Vec3::X * 500.,
                    age: 0.2,
                    ended: None,
                });
                o.saved.boss.serial = 1;
            }
            if name == "dead" || matches!(phase, Phase::Killed | Phase::Done) {
                o.saved.boss.hit(Hit {
                    id: BASE,
                    damage: 2000.,
                    kind: crate::combat::DamageKind::Ice,
                    knockback: Vec3::X,
                });
                o.saved.boss.time = 0.5;
            }
            if phase == Phase::Killed {
                o.saved.time = o.portal_time() + 3.5;
            }
            if phase == Phase::Done {
                o.finish_exit();
            }
            o.rebuild(&r.map)?;
            let at = if matches!(phase, Phase::Approach | Phase::Beheading) {
                crate::interaction::spawn(&r.map, None).0
            } else {
                o.data.points["alice_king_dest1"].translation + Vec3::Z * 48.
            };
            r.interactions.sync(&mut r.world);
            r.player = Player::spawn(&r.world, at).context("King Store player blocked")?;
            let mut cp = r.checkpoint();
            cp.level.npcs =
                crate::npc::Npcs::load(a, &r.map, "rchess1", None, false, false)?.snapshot();
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
                "Reader needs a fresh process"
            );
        }
        let game = store.read(Slot::One)?.game;
        let restored = crate::save::Restored::build(a, game.clone())?;
        ensure!(
            serde_json::to_value(restored.interactions.snapshot())?
                == serde_json::to_value(&game.level()?.interactions)?,
            "{name} changed encounter on restore"
        );
        ensure!(
            restored.game.player.feet == game.player.feet,
            "{name} displaced Alice"
        );
        let o = restored
            .interactions
            .levels
            .iter()
            .find_map(|s| s.ctl.downcast_ref::<Encounter>())
            .context("Missing restored King")?;
        ensure!(o.saved.phase == phase, "{name} changed phase");
        let _art = art::Art::load(a, o)?;
        println!(
            "PASS King Store {} {name}",
            if write { "write" } else { "fresh read" }
        );
        next_frame().await;
    }
    Ok(())
}

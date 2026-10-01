//! Checks against the locally supplied TIKI facts; no original script is executed.
use super::*;
use crate::{
    cards::{Guard, Kind, Phase},
    clockwork::Rig,
};
fn clip_tokens<'a>(tokens: &'a [String], name: &str) -> Result<&'a [String]> {
    let start = tokens
        .windows(2)
        .position(|w| w[0] == name && w[1].ends_with(".ska"))
        .context("Missing authored card clip")?
        + 2;
    ensure!(
        tokens.get(start).is_some_and(|s| s == "{"),
        "Missing clip events"
    );
    let mut depth = 0;
    for (i, s) in tokens.iter().enumerate().skip(start) {
        if s == "{" {
            depth += 1;
        } else if s == "}" {
            depth -= 1;
            if depth == 0 {
                return Ok(&tokens[start..=i]);
            }
        }
    }
    anyhow::bail!("Unclosed source clip")
}
fn has(tokens: &[String], fact: &[&str]) -> bool {
    tokens
        .windows(fact.len())
        .any(|w| w.iter().zip(fact).all(|(a, b)| a == b))
}
pub(super) fn check(assets: &mut Assets) -> Result<()> {
    for kind in [Kind::Heart, Kind::Spade] {
        let text = String::from_utf8_lossy(&assets.read(&format!("models/{}.tik", kind.model()))?)
            .into_owned();
        let tokens = crate::bsp::tokens(&text)?;
        let data = Data::load(assets, kind.model(), &[])?;
        let facts: &[(&str, &[(&str, &str)])] = if kind == Kind::Heart {
            &[
                ("attack1", &[("8", "15")]),
                ("attack2", &[("11", "15")]),
                ("attack3", &[("6", "15"), ("9", "15")]),
                ("attack4", &[("25", "15"), ("38", "15")]),
                (
                    "attack_5_mid",
                    &[("2", "15"), ("5", "15"), ("9", "15"), ("12", "15")],
                ),
            ]
        } else {
            &[
                ("attack_basic", &[("11", "15")]),
                ("attack_spin", &[("9", "5"), ("13", "15")]),
            ]
        };
        for (clip, contacts) in facts {
            let t = clip_tokens(&tokens, clip)?;
            for (frame, damage) in *contacts {
                ensure!(has(t, &[frame, "melee", damage]), "Changed {clip} contact");
                ensure!(
                    frame.parse::<usize>()? < data.clips[*clip].frames.len(),
                    "Contact beyond clip"
                );
            }
        }
        let shots: &[(&str, &[&str])] = if kind == Kind::Heart {
            &[("fire1", &["entry"]), ("fire2", &["38"])]
        } else {
            &[("attack_ranged", &["16", "30"])]
        };
        for (clip, frames) in shots {
            let t = clip_tokens(&tokens, clip)?;
            for frame in *frames {
                ensure!(
                    has(
                        t,
                        &[
                            frame,
                            "proj",
                            "tag_barrel",
                            &format!("{}.tik", kind.projectile())
                        ]
                    ),
                    "Changed {clip} projectile cue"
                );
                let time = frame.parse::<f32>().unwrap_or(0.) * data.clips[*clip].frame_time;
                let muzzle = data.tag(clip, time, "tag_barrel");
                ensure!(
                    muzzle.translation.is_finite() && muzzle.translation.length() > 10.,
                    "Invalid card muzzle"
                );
            }
        }
        let projectile = crate::bsp::tokens(&String::from_utf8_lossy(
            &assets.read(&format!("models/{}.tik", kind.projectile()))?,
        ))?;
        for fact in [
            vec!["speed", "850"],
            vec!["life", "5"],
            vec!["hitdamage", if kind == Kind::Heart { "20" } else { "13" }],
        ] {
            ensure!(has(&projectile, &fact), "Changed projectile fact {fact:?}");
        }
        let mut g = Guard::new(kind, Vec3::ZERO, 0., 1., 7);
        g.phase = Phase::Dead;
        g.health = 0.;
        g.frozen = true;
        g.time = 100.;
        ensure!(
            (g.sample_time(&data)
                - data.clips["death_frozen"].frame_time
                    * if kind == Kind::Heart { 9. } else { 6. })
            .abs()
                < 0.001,
            "Frozen frame mismatch"
        );
        if kind == Kind::Spade {
            ensure!(
                data.card_muzzle.is_some() && data.card_cut.is_some(),
                "Missing staff/cut recipe"
            );
            let floor = World::fixture(&[(vec3(-2000., -2000., -20.), vec3(2000., 2000., 0.))]);
            let mut cut = Guard::new(kind, Vec3::Z * 0.1, 0., 1., 7);
            cut.hit(combat::Hit {
                id: 0,
                damage: 1000.,
                kind: combat::DamageKind::Knife,
                knockback: Vec3::ZERO,
            });
            for _ in 0..48 {
                cut.step(&floor, Vec3::ZERO, &data, &mut combat::Feedback::default());
            }
            ensure!(
                cut.dismember.fragment.is_some(),
                "Original Spade torso failed to separate above the floor"
            );
            let mut restored: Guard = serde_json::from_slice(&serde_json::to_vec(&cut)?)?;
            for _ in 0..720 {
                for g in [&mut cut, &mut restored] {
                    g.step(&floor, Vec3::ZERO, &data, &mut combat::Feedback::default());
                }
            }
            ensure!(
                serde_json::to_vec(&cut)? == serde_json::to_vec(&restored)?
                    && cut.dismember.fragment.is_none(),
                "Original Spade torso restart/cleanup differs"
            );
        }
        println!(
            "PASS {}: original contact frames, projectile values, sampled muzzle and frozen pose",
            kind.model()
        );
    }
    Ok(())
}

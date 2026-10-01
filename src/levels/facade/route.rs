//! Ascension input course from the earned entrance; no staged player placement.
use super::*;
use crate::{movement::Controls, route::Route};

pub(super) fn check(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut r = if let Ok(path) = std::env::var("LOOKING_GLASS_FACADE_FROM") {
            Route::resume(a, &serde_json::from_slice(&std::fs::read(path)?)?)?
        } else {
            let mut r = Route::new(a, "facade", Some("facade_start1"))?;
            r.enable_native_cast(a)?;
            r
        };
        let result = drive(&mut r);
        crate::campaign_playtest::diagnostic_save(a, &r, "private/facade-route/view")?;
        result?;
        let next = r.depart(a, true)?;
        ensure!(next.level().map == "keep" && next.world.body_clear(next.player.feet), "Blocked castle arrival");
        Ok(())
    })
}

pub(crate) fn drive(r: &mut Route) -> Result<()> {
    std::fs::create_dir_all("private/facade-route")?;
    r.tactics = true;
    r.ice_stream = true;
    // Leave enough of the carried Will for the two paid Watch crossings.
    r.will_reserve = 2.;
    r.stop_at_exit = true;
    // A carried Watch can still be cooling down after the battlefield. Wait
    // at the entrance before the course reaches the exposed Watch crossing.
    if r.stats.powers.recharge > 0. {
        r.wait(r.stats.powers.recharge + 0.1)?;
    }
    let text = std::env::var("LOOKING_GLASS_FACADE_INPUT").ok()
        .map(std::fs::read_to_string).transpose()?
        .unwrap_or_else(|| include_str!("route_steps.json").into());
    let steps: Vec<serde_json::Value> = serde_json::from_str(&text)?;
    for (n, s) in steps.iter().enumerate() {
        println!("ASCENSION {n} {s} at {:?}, sanity {}", r.player.feet, r.stats.sanity());
        let result = (|| -> Result<()> {
            if let Some(v) = s.get("node").or_else(|| s.get("pickup")) {
                let id = v.as_u64().context("Invalid pathnode")? as usize;
                let e = r.map.entities.get(id).context("Missing pathnode")?;
                ensure!(e.get("classname").is_some_and(|c| if s.get("pickup").is_some() {
                    c.starts_with("Item_")
                } else { c == "info_pathnode" }), "Not an authored route marker");
                let raw = interaction::vector(&e["origin"]).context("Bad pathnode position")?;
                let goal = [32.,64.,128.].into_iter().find_map(|h|
                    r.world.actor_footing(raw + Vec3::Z*h, PLAYER_CENTER, PLAYER_HALF, 256.))
                    .context("Unsupported pathnode")?;
                r.navigate(goal)?;
            } else if let Some(v) = s.get("nav") {
                r.navigate(Vec3::from_array(serde_json::from_value(v.clone())?))?;
            } else if let Some(v) = s.get("walk") {
                r.walk(Vec3::from_array(serde_json::from_value(v.clone())?), false)?;
            } else if let Some(v) = s.get("rise") {
                let p: [f32;6] = serde_json::from_value(v.clone())?;
                super::check::rise(r, vec2(p[0],p[1]), p[2], vec3(p[3],p[4],p[5]))?;
            } else if let Some(v) = s.get("clear") {
                r.clear(v.as_f64().context("Invalid range")? as f32)?;
            } else if let Some(v) = s.get("wait") {
                r.wait(v.as_f64().context("Invalid wait")? as f32)?;
            } else if s.get("watch").is_some() {
                r.use_watch()?;
            } else if s.get("board").is_some() {
                for _ in 0..600 {
                    r.tick(Controls { wish: Vec2::Y, jump: r.player.grounded, ..Default::default() })?;
                    if r.interactions.scripted() { break; }
                }
                ensure!(r.interactions.scripted(), "Lift not reached from {:?}", r.player.feet);
                r.wait_for_cinematic()?;
                r.wait(0.1)?;
            } else { anyhow::bail!("Unknown Ascension input {s}"); }
            Ok(())
        })();
        std::fs::write("private/facade-route/last.json", serde_json::to_vec(&r.checkpoint())?)?;
        std::fs::write(format!("private/facade-route/step-{n}.json"), serde_json::to_vec(&r.checkpoint())?)?;
        result?;
        if r.transition.is_some() { break; }
    }
    ensure!(r.transition.as_ref().is_some_and(|d| EXIT.matches(d)), "Ascension route incomplete");
    r.assert_clean(0,0)?;
    ensure!(!r.stats.god && !r.stats.notarget, "Assisted Ascension route");
    Ok(())
}

//! Continuous native campaign replay: earned resources, live cast and real exits.
use crate::{assets::Assets, campaign, campaign_chain, campaign_route, inventory::Stats,
    powerups::Difficulty, route::{Checkpoint, Route}};
use anyhow::{ensure, Context, Result};
use std::{fs, path::Path};

const DIR: &str = "private/full-campaign";

pub async fn run(a: &mut Assets, args: campaign_chain::Args, difficulty: Difficulty) -> Result<()> {
    ensure!(!args.allow_retry && args.expect.is_none(), "Full playtest does not grant retry resources or accept expected failures");
    fs::create_dir_all(DIR)?;
    let from = args.from.as_deref().map(campaign_chain::parse_visit).transpose()?.unwrap_or(0);
    let last = args.to.as_deref().map(campaign_chain::parse_visit).transpose()?.unwrap_or(38);
    ensure!(from <= last, "Empty playtest range");
    // Private replay continuation: retain the exact earned state when a long
    // native job ends between route commands. The caller supplies the remaining
    // ordinary-input course; this never edits resources or progression.
    let continuation = std::env::var_os("LOOKING_GLASS_CAMPAIGN_RESUME").map(std::path::PathBuf::from);
    ensure!(continuation.is_none() || from > 0, "Mid-route continuation requires --campaign-from");
    let mut r = if from == 0 {
        Route::enter(a, "gvillage", None, Stats::for_level("gvillage", None), difficulty)?
    } else {
        let cp: Checkpoint = serde_json::from_slice(&fs::read(continuation.clone().unwrap_or_else(|| entrance(from)))
            .context("Resume requires an earned native checkpoint")?)?;
        ensure!(cp.route.native_cast && cp.difficulty == difficulty
            && campaign::visit_index(&cp.level.map, cp.level.entry.as_deref()) == Some(from)
            && cp.ledger.completed.len() >= from, "Checkpoint is not a native campaign continuation");
        Route::resume(a, &cp)?
    };
    let catalog = crate::inventory::Catalog::load(a)?;
    let rewards = campaign_chain::provenance::table();
    let fingerprint = a.fingerprint()?;
    let mut alice = crate::character::Character::load(a)?;
    let report_path = Path::new(DIR).join("report.json");
    let mut rows: Vec<serde_json::Value> = if from == 0 { Vec::new() } else {
        serde_json::from_slice(&fs::read(&report_path)?)?
    };
    rows.retain(|v| v["visit"].as_u64().is_some_and(|n| n <= from as u64));
    for i in from..=last {
        if r.native_cast.is_none() {
            // Replace the headless guard stand-ins at fresh entry, before any simulation.
            ensure!(r.ticks == 0, "Cannot replace an advanced headless cast");
            r.guards.clear();
            r.enable_native_cast(a)?;
        }
        r.stop_at_exit = true;
        r.skip_cinematics = args.skip_cinematics;
        ensure!(!r.stats.god && !r.stats.notarget, "Assisted campaign state");
        // Match the viewer: the new owner replaces transient protections from the last map.
        r.interactions.prepare_player(&mut r.stats, &mut r.player);
        r.stats.prepare_player(&mut r.player);
        let entry_resources = r.stats.clone();
        let continued = i == from && continuation.is_some();
        let checkpoint_path = if continued { Path::new(DIR).join(format!("{:02}-resumed.json",i+1)) } else { entrance(i) };
        fs::write(checkpoint_path, serde_json::to_vec(&r.checkpoint())?)?;
        alice.reset(&r.player, r.player.script_facing);
        alice.power_appearance(&r.stats);
        let game = campaign_chain::save_chain::build_game(&r.checkpoint(), &r.ledger,
            r.player.script_facing, false, alice.snapshot());
        let store = crate::save::Store::new(Path::new(DIR).join(format!("{:02}-save",i+1)),fingerprint.clone());
        store.write(crate::save::Slot::Auto,&game)?;
        let restored = crate::save::Restored::build(a,store.read(crate::save::Slot::Auto)?.game)?;
        ensure!(serde_json::to_value(&restored.game.player)? == serde_json::to_value(&r.player)?,
            "Native arrival save changed Alice in {}", campaign::visits()[i]);
        ensure!(serde_json::to_value(&restored.game.stats)? == serde_json::to_value(&r.stats)?,
            "Native arrival save changed resources in {}", campaign::visits()[i]);
        ensure!(serde_json::to_value(restored.npcs.snapshot())? == serde_json::to_value(&r.level().npcs)?,
            "Native arrival save changed enemies in {}", campaign::visits()[i]);
        if !restored.scene.missing.is_empty() { println!("TEXTURE AUDIT {}: {:?}", campaign::visits()[i], restored.scene.missing); }
        drop(restored);
        println!("FULL START {:02} {} sanity {} will {}", i+1, campaign::visits()[i], r.stats.sanity(), r.stats.will());
        let result = drive(a, i, &mut r).and_then(|_| {
            for reward in rewards.iter().filter(|v| v.visit == i) {
                let status = campaign_chain::provenance::settle(reward, &r, &catalog);
                if let Some(message) = campaign_chain::provenance::fails(reward, &status, true) { anyhow::bail!(message); }
            }
            Ok(())
        });
        // A map's mid-scene resume may reset its local baseline; report the whole visit.
        let mut metrics = r.metrics();
        metrics.sanity_in = entry_resources.sanity();
        metrics.will_in = entry_resources.will();
        let new = r.stats.collected.difference(&entry_resources.collected);
        metrics.loot = new.clone().filter(|k| k.starts_with("drop:")).count();
        metrics.pickups = new.filter(|k| !k.starts_with("drop:")).count();
        fs::write(Path::new(DIR).join(format!("{:02}-last.json",i+1)), serde_json::to_vec(&r.checkpoint())?)?;
        rows.push(serde_json::json!({"visit":i+1,"map":r.level().map,"metrics":metrics,"audit":r.audit,
            "mid_route_continuation":continued,
            "passed":result.is_ok(),"error":result.as_ref().err().map(|e|format!("{e:#}"))}));
        fs::write(Path::new(DIR).join("report.json"),serde_json::to_vec_pretty(&rows)?)?;
        result.with_context(||format!("Full campaign visit {:02} {} at {:?}",i+1,campaign::visits()[i],r.player.feet))?;
        ensure!(r.stats.alive() && r.native_cast.is_some(),"Incomplete live cast or dead Alice");
        println!("FULL PASS {:02} {}: {}",i+1,campaign::visits()[i],metrics);
        if i == 38 {
            ensure!(r.interactions.levels.iter().any(|s| s.ctl.ending_ready()), "Final encounter did not request the ending");
            diagnostic_save(a, &r, &format!("{DIR}/39-ending-save"))?;
            if from == 0 {
                println!("PASS FULL CAMPAIGN ROUTE: all 39 visits in this replay; earned ending save ready for film/credits verification");
            } else {
                println!("PASS CAMPAIGN CONTINUATION: visits {} through 39 in this replay; earned ending save ready for film/credits verification", from + 1);
            }
            return Ok(());
        }
        let before = serde_json::to_value(&r.stats)?;
        r = r.depart(a, true)?;
        ensure!(campaign::visit_index(&r.level().map,r.level().entry.as_deref()) == Some(i+1), "Wrong campaign destination");
        if r.level().map != "utemple" { ensure!(serde_json::to_value(&r.stats)? == before,"Unexpected resource grant at transition"); }
        macroquad::prelude::next_frame().await;
    }
    println!("PASS requested native campaign segment; remaining visits are not certified");
    Ok(())
}

fn entrance(i: usize) -> std::path::PathBuf { Path::new(DIR).join(format!("{:02}-entrance.json", i+1)) }
/// A private visual-debug save of an earned route checkpoint, never a campaign pass.
pub(crate) fn diagnostic_save(a: &mut Assets, r: &Route, directory: &str) -> Result<()> {
    let mut alice = crate::character::Character::load(a)?;
    alice.reset(&r.player, r.player.script_facing);
    alice.power_appearance(&r.stats);
    let game = campaign_chain::save_chain::build_game(&r.checkpoint(), &r.ledger,
        r.player.script_facing, false, alice.snapshot());
    crate::save::Store::new(Path::new(directory).into(), a.fingerprint()?)
        .write(crate::save::Slot::Auto, &game)?;
    Ok(())
}
fn drive(a: &mut Assets, i: usize, r: &mut Route) -> Result<()> {
    if let Some(leg) = campaign_route::LEGS.get(i) {
        leg.run(r, false)?;
        return Ok(());
    }
    if i == 8 { return crate::pool::route::drive(r); }
    if i == 9 { return crate::levels::potears2::drive_route(r); }
    match i {
        10 => return crate::duchess_check::drive(a,r),
        11 => return crate::levels::utemple::drive_route(a,r),
        12 => return crate::levels::garden1::drive_route(r),
        13 => return crate::levels::garden2::drive_route(a,r),
        14 => return crate::levels::garden3::drive_route(r),
        15 => return crate::levels::garden4::drive_route(r),
        16 => return crate::levels::centipede1::drive_route(a,r),
        17 => return crate::levels::centipede2::drive_route(a,r),
        18 | 27 => return crate::levels::wforest::drive_route(r),
        19 => return crate::levels::wchess1::drive_route(r),
        20 => return crate::levels::wchess2::drive_route(r),
        21 => return crate::levels::rchess1::drive_route(r),
        22 => return crate::levels::funhouse::drive_route(r),
        24 => return crate::levels::hatter2::drive_route(r),
        25 => return crate::levels::jlair1::drive_route(r),
        26 => return crate::jabberwock::check::drive(r, crate::jabberwock::Kind::Lair),
        29 => return crate::levels::tower1::drive_route(r),
        34 => return crate::jabberwock::check::drive(r, crate::jabberwock::Kind::Grounds),
        28 => return crate::levels::hedge1::drive_route(r),
        30 => return crate::levels::hedge2::drive_route(r),
        31 => return crate::levels::tower2::drive_route(a,r),
        32 => return crate::levels::hedge3::drive_route(a,r),
        33 => return crate::levels::tower3::drive_route(a,r),
        35 => return crate::levels::grounds2::drive_route(r),
        36 => return crate::levels::facade::drive_route(r),
        37 => return crate::levels::keep::drive_route(r),
        38 => return crate::levels::qlair::drive_route(r),
        23 => return crate::levels::hatter1::drive_route(r),
        _ => (),
    }
    anyhow::bail!("Native campaign driver not yet connected for {}",campaign::visits()[i])
}

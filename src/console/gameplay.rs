//! Gameplay console operations shared by the UI and behavioural checks.
use crate::{
    assets::Assets,
    inventory::{Catalog, PickupKind, Stats, WEAPONS},
    powerups::Kind,
};
use anyhow::{bail, ensure, Result};

const EXTRA_ITEMS: &[&str] = &[
    "p_h1.tik",
    "p_h2.tik",
    "p_m1.tik",
    "p_m2.tik",
    "w_me_small.tik",
    "w_me_medium.tik",
    "w_me_large.tik",
    "w_me_super.tik",
    "w_ragebox.tik",
    "w_grasshopperteaitem.tik",
    "w_lookingglass.tik",
    "w_turtleshell.tik",
    "beaker.tik",
    "beaker01.tik",
    "beaker02.tik",
    "beakerhand.tik",
    "lollypop.tik",
    "star.tik",
    "tart.tik",
];

pub fn items(assets: &Assets) -> Vec<String> {
    WEAPONS
        .iter()
        .map(|(id, _)| format!("w_{id}.tik"))
        .chain(EXTRA_ITEMS.iter().map(|s| (*s).to_owned()))
        .filter(|s| assets.contains(&format!("models/{s}")))
        .collect()
}

/// Filename resolution only reads mounted assets. It cannot load native code,
/// execute TIK commands, or silently grant a different item for an unknown name.
pub fn give(name: &str, assets: &Assets, catalog: &Catalog, stats: &mut Stats) -> Result<String> {
    ensure!(
        stats.alive(),
        "Set health above zero or retry before giving items."
    );
    match name {
        "all" => {
            stats.grant_weapons();
            // The original giveall declaration also grants the breathing shell.
            stats.turtle_air = true;
            stats.set_health(100.)?;
            stats.apply(PickupKind::Will, 100.);
            return Ok("All toys and Turtle Shell granted / Sanity 100 / Will 100".into());
        }
        "weapons" => {
            stats.grant_weapons();
            return Ok("All toys granted / resources unchanged".into());
        }
        "health" => {
            stats.set_health(100.)?;
            return Ok("Sanity 100".into());
        }
        "will" => {
            stats.apply(PickupKind::Will, 100.);
            return Ok("Will 100".into());
        }
        _ => (),
    }
    ensure!(
        assets.contains(&format!("models/{name}")),
        "Item file not found: {name}. Type itemlist."
    );
    let stem = name.trim_end_matches(".tik");
    if let Some(i) = WEAPONS.iter().position(|(id, _)| stem == format!("w_{id}")) {
        stats.apply(PickupKind::Weapon(i), 1.);
        return Ok(format!("Granted {} / Will {}", WEAPONS[i].1, stats.will()));
    }
    if let Some((_, amount)) = catalog.amounts.iter().find(|(id, _)| id == stem) {
        let kind = if stem.starts_with("p_h") {
            PickupKind::Sanity
        } else if stem.starts_with("p_m") {
            PickupKind::Will
        } else {
            PickupKind::Essence
        };
        stats.apply(kind, *amount);
        return Ok(format!(
            "Granted {name} / Sanity {} / Will {}",
            stats.sanity(),
            stats.will()
        ));
    }
    let power = match stem {
        "w_ragebox" => Some(Kind::Rage),
        "w_grasshopperteaitem" => Some(Kind::Tea),
        "w_lookingglass" => Some(Kind::Glass),
        _ => None,
    };
    if let Some(power) = power {
        ensure!(
            stats.powerup(power),
            "A power-up is already active; wait for it to expire."
        );
        return Ok(format!("Activated {}", power.name()));
    }
    let item = match stem {
        "w_turtleshell" => { stats.turtle_air = true; "Turtle Shell" }
        "beaker" | "beaker01" => { stats.school_items.jumbogrow = true; "Jumbogrow" }
        "beaker02" | "beakerhand" => { stats.school_items.potion = true; "Drink Me potion" }
        "lollypop" => { stats.school_items.lollipop = true; "Lollipop" }
        "star" => { stats.school_items.star = true; "Lucky Star" }
        "tart" => { stats.school_items.mushroom = true; "Mushroom" }
        _ => bail!("That model is not a supported Alice inventory item. Type itemlist for usable filenames."),
    };
    Ok(format!("Granted {item}"))
}

pub fn camera(
    value: f32,
    third_person: &mut bool,
    preferences: &mut crate::preferences::Preferences,
) {
    *third_person = value > 0.;
    if *third_person {
        preferences.camera_distance = value;
    }
}

pub fn sync_inventory(stats: &Stats, interactions: &mut crate::interaction::Interactions) {
    if let Some(school) = &mut interactions.school2 {
        school.quest.items = stats.school_items.clone();
        // Receiving the last reward is equivalent to owning both pickups, but
        // an early grant never completes unrelated dialogue or combat stages.
        if school.quest.stage == crate::school2_quest::Stage::Rewards
            && school.quest.items.potion
            && school.quest.items.star
        {
            school.quest.enter(crate::school2_quest::Stage::Complete);
        }
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let catalog = Catalog::load(assets)?;
    let mut stats = Stats::for_level("gvillage", None);
    stats.damage(40.);
    stats.spend_will(85.);
    give("weapons", assets, &catalog, &mut stats)?;
    ensure!(
        stats.sanity() == 60. && stats.will() == 15. && stats.copies(6) == 3,
        "Wuss changed resources or missed the three Dice"
    );
    stats.god = true;
    stats.damage(999999.);
    for (i, weapon) in catalog.weapons.iter().enumerate() {
        for cost in [Some(weapon.primary), weapon.alternate]
            .into_iter()
            .flatten()
        {
            ensure!(
                stats.spend_will(cost) && stats.will() == 15.,
                "God charged toy {i}"
            );
        }
    }
    ensure!(stats.sanity() == 60., "God permitted damage");
    stats.god = false;
    ensure!(!stats.spend_will(40.), "Disabling god left free attacks");
    give("all", assets, &catalog, &mut stats)?;
    ensure!(
        stats.sanity() == 100. && stats.will() == 100. && stats.turtle_air,
        "Give all incomplete"
    );
    for item in items(assets) {
        let mut stats = Stats::for_level("gvillage", None);
        stats.damage(50.);
        stats.spend_will(80.);
        give(&item, assets, &catalog, &mut stats)?;
        stats.validate_save()?;
        ensure!(
            stats.collected.is_empty(),
            "Console grant consumed a world pickup"
        );
    }
    let before = serde_json::to_value(&stats)?;
    for bad in ["does_not_exist.tik", "c_boojum.tik"] {
        ensure!(
            give(bad, assets, &catalog, &mut stats).is_err(),
            "Invalid give accepted"
        );
        ensure!(
            serde_json::to_value(&stats)? == before,
            "Failed give changed state"
        );
    }
    stats.set_health(250.)?;
    stats.notarget = true;
    stats.god = true;
    let encoded = serde_json::to_vec(&stats)?;
    let restored: Stats = serde_json::from_slice(&encoded)?;
    restored.validate_save()?;
    ensure!(
        restored.sanity() == 250. && restored.notarget && restored.god,
        "Cheat state did not restore"
    );
    let mut preferences = crate::preferences::Preferences::default();
    let mut third_person = true;
    camera(-45., &mut third_person, &mut preferences);
    ensure!(
        !third_person,
        "Negative camera distance did not enter first person"
    );
    camera(128., &mut third_person, &mut preferences);
    ensure!(
        third_person && preferences.camera_distance == 128.,
        "Camera distance did not return to third person"
    );
    let map = crate::bsp::Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
    let mut interactions = crate::interaction::Interactions::load(&map)?;
    interactions.set_entry(assets, &map, "skool2", None)?;
    let mut stats = Stats::for_level("skool2", None);
    give("beaker.tik", assets, &catalog, &mut stats)?;
    sync_inventory(&stats, &mut interactions);
    ensure!(
        interactions.school2.as_ref().unwrap().quest.items.jumbogrow
            && interactions.school2.as_ref().unwrap().quest.stage
                == crate::school2_quest::Stage::Explore,
        "Item grant was lost or completed unrelated school stages"
    );
    interactions
        .school2
        .as_mut()
        .unwrap()
        .quest
        .enter(crate::school2_quest::Stage::Rewards);
    give("beaker02.tik", assets, &catalog, &mut stats)?;
    sync_inventory(&stats, &mut interactions);
    ensure!(
        interactions.school2.as_ref().unwrap().quest.stage == crate::school2_quest::Stage::Rewards,
        "Potion alone completed the two-reward exit"
    );
    give("star.tik", assets, &catalog, &mut stats)?;
    sync_inventory(&stats, &mut interactions);
    ensure!(
        interactions.school2.as_ref().unwrap().quest.stage == crate::school2_quest::Stage::Complete,
        "Granted school rewards did not unlock their inventory gate"
    );
    println!("PASS console gameplay: God protects Sanity/Will for all declared attack costs; Wuss preserves resources; Give All, {} item filenames, invalid-item atomicity and saved cheat state", items(assets).len());
    Ok(())
}

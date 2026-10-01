//! Entry loadouts for exploration, authored from the supplied campaign sequence.
//! This does not execute level scripts or simulate their completion. See docs/LOADOUTS.md.
use crate::{
    assets::Assets,
    bsp::Bsp,
    cheshire::Hints,
    collision::World,
    interaction::{self, Interactions},
    inventory::{self, Stats, WEAPONS},
    movement::Player,
    powerups::Difficulty,
    save::{self, Campaign, Level},
    story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::Vec3;
use std::sync::OnceLock;

// Repeated maps are distinct visits. None means that map's normal first entrance.
const ROUTE: [(&str, Option<&str>); 39] = [
    ("gvillage", None),
    ("pandemonium", None),
    ("fortress1", None),
    ("fortress2", None),
    ("fortress1", Some("fortress1_start2")),
    ("skool1", None),
    ("skool2", None),
    ("skool1", Some("skool1_start2")),
    ("potears1", None),
    ("potears2", None),
    ("potears3", None),
    ("utemple", None),
    ("garden1", None),
    ("garden2", None),
    ("garden3", None),
    ("garden4", None),
    ("centipede1", None),
    ("centipede2", None),
    ("wforest", None),
    ("wchess1", None),
    ("wchess2", None),
    ("rchess1", None),
    ("funhouse", None),
    ("hatter1", None),
    ("hatter2", None),
    ("jlair1", None),
    ("jlair2", None),
    ("wforest", Some("wforest_start2")),
    ("hedge1", None),
    ("tower1", None),
    ("hedge2", None),
    ("tower2", None),
    ("hedge3", None),
    ("tower3", None),
    ("grounds1", None),
    ("grounds2", None),
    ("facade", None),
    ("keep", None),
    ("qlair", None),
];

/// Every visit in story order: the 36 maps' first entrances plus the three return visits.
pub fn route() -> &'static [(&'static str, Option<&'static str>)] {
    &ROUTE
}

/// A selectable visit, keeping the asset map index separate from story order.
pub struct LevelChoice {
    pub map: usize,
    pub entry: Option<&'static str>,
    pub title: String,
}

pub fn level_choices(maps: &[String]) -> Vec<LevelChoice> {
    let mut choices = ROUTE
        .iter()
        .filter_map(|&(name, entry)| {
            let map = maps.iter().position(|m| m == name)?;
            let title = level_title(name);
            Some(LevelChoice {
                map,
                entry,
                title: if entry.is_some() {
                    format!("{title} - return")
                } else {
                    title.to_owned()
                },
            })
        })
        .collect::<Vec<_>>();
    // Keep any additional maps from the user's archives accessible after the story.
    for (map, name) in maps.iter().enumerate() {
        if !choices.iter().any(|c| c.map == map) {
            choices.push(LevelChoice {
                map,
                entry: None,
                title: name.clone(),
            });
        }
    }
    choices
}

pub fn choice_position(choices: &[LevelChoice], map: usize, entry: Option<&str>) -> usize {
    choices
        .iter()
        .position(|c| c.map == map && c.entry == entry)
        .or_else(|| choices.iter().position(|c| c.map == map))
        .unwrap_or(0)
}

pub(crate) fn level_title(map: &str) -> &str {
    match map {
        "gvillage" => "Dementia - opening village",
        "pandemonium" => "Pandemonium",
        "fortress1" => "Fortress of Doors",
        "fortress2" => "Beyond the Wall",
        "skool1" => "Skool Daze",
        "skool2" => "Skool's Out",
        "potears1" => "Pool of Tears",
        "potears2" => "Hollow Hideaway",
        "potears3" => "Just Desserts - Duchess encounter",
        "utemple" => "Wholly Morel Ground",
        "garden1" => "Dry Landing",
        "garden2" => "Herbaceous Border",
        "garden3" => "Rolling Stones",
        "garden4" => "Icy Reception",
        "centipede1" => "Fungiferous Flora",
        "centipede2" => "Centipede's Sanctum",
        "wforest" => "Caterpillar's Plot",
        "wchess1" => "Pale Realm",
        "wchess2" => "Castling",
        "rchess1" => "Checkmate in Red",
        "funhouse" => "Mirror Image",
        "hatter1" => "Crazed Clockwork",
        "hatter2" => "About Face",
        "jlair1" => "Burning Curiosity",
        "jlair2" => "Jabberwock's Lair",
        "hedge1" => "Majestic Maze",
        "tower1" => "Airborne Terror",
        "hedge2" => "Mystifying Madness",
        "tower2" => "Water Logged",
        "hedge3" => "Labyrinthine Revenge",
        "tower3" => "Machinations",
        "grounds1" => "Royal Rage",
        "grounds2" => "Battle Royale",
        "facade" => "Ascension",
        "keep" => "Castle Keep",
        "qlair" => "Heart of Darkness - finale",
        _ => map,
    }
}

pub fn after_temple(map: &str, entry: Option<&str>) -> bool {
    ROUTE
        .iter()
        .position(|&(m, e)| m == map && (e.is_none() || e == entry))
        .is_some_and(|i| i >= 11)
}

// Completion milestones: map, visit, toy slot, cumulative copies after finishing.
// Ordinary duplicate pickups refill resources; only distinct Dice increase copies.
const REWARDS: [(&str, Option<&str>, usize, u8); 12] = [
    ("gvillage", None, 0, 1),
    ("pandemonium", None, 1, 1),
    ("skool1", None, 2, 1),
    ("skool2", None, 6, 1),
    ("potears3", None, 3, 1),
    ("garden4", None, 4, 1),
    ("centipede1", None, 6, 2),
    ("rchess1", None, 6, 3),
    ("funhouse", None, 5, 1),
    ("hatter2", None, 9, 1),
    ("jlair2", None, 7, 1),
    ("wforest", Some("wforest_start2"), 8, 1),
];

/// The completion milestones (map, visit, toy slot, copies held afterwards) that the baseline
/// loadout is built from. The chain's reward-provenance report holds the strict chain to them.
pub fn rewards() -> &'static [(&'static str, Option<&'static str>, usize, u8)] {
    &REWARDS
}

pub fn return_entry(map: &str) -> Option<&'static str> {
    ROUTE
        .iter()
        .find_map(|&(name, entry)| (name == map).then_some(entry).flatten())
}

fn stage(map: &str, entry: Option<&str>) -> Option<usize> {
    // Named first starts behave like the default first start; unknown custom entries
    // do not accidentally receive the later visit's weapons.
    ROUTE
        .iter()
        .position(|&(m, e)| m == map && e.is_some() && e == entry)
        .or_else(|| ROUTE.iter().position(|&(m, _)| m == map))
}

pub fn loadout(map: &str, entry: Option<&str>) -> Option<[u8; 10]> {
    let at = stage(map, entry)?;
    let mut owned = [0; 10];
    for (m, e, weapon, count) in REWARDS {
        if stage(m, e).expect("Known reward visit") < at {
            owned[weapon] = count;
        }
    }
    Some(owned)
}

/// Where an exit leads: the destination map and its named entrance (`None`: the default start).
pub type Exit = (String, Option<String>);

/// The 39 visit keys in story order, aliased exactly like a save's `save::visit_key`: a named
/// first entrance shares its map's first visit, and only a map's return entrance is `$return`.
pub fn visits() -> &'static [String] {
    static KEYS: OnceLock<Vec<String>> = OnceLock::new();
    KEYS.get_or_init(|| {
        ROUTE
            .iter()
            .map(|&(map, entry)| save::visit_key(map, entry))
            .collect()
    })
}

/// Position of a visit in `ROUTE`, or `None` for a map or entrance outside the campaign.
pub fn visit_index(map: &str, entry: Option<&str>) -> Option<usize> {
    let key = save::visit_key(map, entry);
    visits().iter().position(|k| *k == key)
}

/// One visit loaded without a window: the map with its difficulty applied, collision, the
/// controllers, the story and the Cheshire hints. The viewer builds its scene on top of this and a
/// headless route runs on it directly, so both start a visit from the same state.
pub struct Loaded {
    pub map: Bsp,
    pub world: World,
    pub interactions: Interactions,
    pub story: Story,
    pub hints: Hints,
}

/// Load a visit. A cached visit is restored from its ledger entry; a fresh one starts its
/// controllers and, when `play_entry` is set, its opening scene. Cached visits never replay it.
pub fn load_visit(
    assets: &mut Assets,
    name: &str,
    entry: Option<&str>,
    cached: Option<&Level>,
    difficulty: Difficulty,
    play_entry: bool,
) -> Result<Loaded> {
    let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)
        .with_context(|| format!("Loading {name}"))?;
    map.difficulty = difficulty;
    let mut world = World::from_bsp(&map)?;
    save::validate_entry(&map, entry)?;
    let (mut interactions, mut story, hints) = if let Some(saved) = cached {
        let mut saved = saved.clone();
        saved.entry = entry.map(str::to_owned);
        saved.restore_logic(assets, &map)?
    } else {
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, name, entry)?;
        (
            i,
            Story::load(assets, name),
            Hints::load(assets, &map, name)?,
        )
    };
    if cached.is_none() && play_entry {
        interactions.entry_story(&mut story);
    }
    interactions.sync(&mut world);
    Ok(Loaded {
        map,
        world,
        interactions,
        story,
        hints,
    })
}

/// Alice at an entrance, or an error when the entrance is obstructed: an exit never falls back
/// to flight, which used to hide a destination Alice cannot stand in.
fn clear_entrance(world: &World, eye: Vec3, key: &str) -> Result<Player> {
    Player::spawn(world, eye).with_context(|| format!("The entrance of {key} is obstructed"))
}

/// What a normal exit hands the next visit, decided without a window.
pub struct NextVisit {
    pub map: String,
    pub entry: Option<String>,
    /// The ledger already held this visit: its controllers were restored, not started.
    pub cached: Option<Level>,
    pub loaded: Loaded,
    /// Alice at the entrance, on clear footing.
    pub player: Player,
    /// The authored eye position and yaw of the entrance.
    pub eye: Vec3,
    pub yaw: f32,
}

/// The transition semantics of a normal exit, shared by the viewer and the headless routes so that
/// they cannot drift: the next visit starts at `stats.difficulty`, the visit that was left is
/// recorded as completed in the ledger, the campaign baseline loadout is filled in (`strict`
/// keeps only authored arrival grants, such as the Mock Turtle's shell in the temple), the
/// destination's opening scene plays only when it is not cached, and an obstructed entrance
/// fails the exit instead of dropping Alice into flight.
///
/// The exit either happens completely or not at all: `stats` and `ledger` change only after the
/// destination has loaded and its entrance has proven clear.
pub fn arrive(
    assets: &mut Assets,
    stats: &mut Stats,
    ledger: &mut Campaign,
    leaving: Level,
    exit: &Exit,
    strict: bool,
) -> Result<NextVisit> {
    let (name, entry) = (exit.0.as_str(), exit.1.as_deref());
    let key = save::visit_key(name, entry);
    let cached = ledger.levels.get(&key).cloned();
    let loaded = load_visit(assets, name, entry, cached.as_ref(), stats.difficulty, true)?;
    let (eye, yaw) = interaction::spawn(&loaded.map, entry);
    let player = clear_entrance(&loaded.world, eye, &key)?;
    ledger.completed.insert(leaving.key());
    ledger.levels.insert(leaving.key(), leaving);
    if strict {
        stats.arrival_grants(name);
    } else {
        stats.ensure_level_weapons(name, entry);
    }
    Ok(NextVisit {
        map: name.into(),
        entry: entry.map(str::to_owned),
        cached,
        loaded,
        player,
        eye,
        yaw,
    })
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let maps = assets.maps();
    ensure!(
        maps.iter().all(|m| loadout(m, None).is_some()),
        "Unmapped campaign level"
    );
    for (name, entry) in ROUTE {
        ensure!(
            maps.iter().any(|m| m == name),
            "Missing campaign level {name}"
        );
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        if let Some(entry) = entry {
            ensure!(
                map.entities.iter().any(|e| e
                    .get("classname")
                    .is_some_and(|c| c == "info_player_start")
                    && e.get("targetname").is_some_and(|n| n == entry)),
                "Missing campaign entry {name}${entry}"
            );
        }
        let stats = Stats::for_level(name, entry);
        ensure!(
            stats.equipped().is_some() || name == "gvillage",
            "Selected unowned weapon"
        );
        println!(
            "{name}{}: {}",
            entry.map_or(String::new(), |s| format!("${s}")),
            stats.weapon_summary()
        );
    }
    // Corroborate each pickup milestone against the actual supplied map entities.
    for (name, _, weapon, _) in REWARDS {
        if weapon == 7 {
            continue;
        } // Script-granted complete Eye Staff after jlair2.
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        ensure!(
            map.entities.iter().any(|e| e
                .get("classname")
                .and_then(|c| inventory::weapon_pickup_index(c))
                == Some(weapon)),
            "Missing {} milestone in {name}",
            WEAPONS[weapon].1
        );
    }
    let script = assets.read("maps/cinematics/jlair2_cinematics.scr")?;
    ensure!(
        String::from_utf8_lossy(&script).contains("w_eyestaff.tik"),
        "Missing Eye Staff grant evidence"
    );
    let catalog = inventory::Catalog::load(assets)?;
    let map = Bsp::parse(&assets.read("maps/hatter2.bsp")?)?;
    ensure!(
        inventory::pickups(&map, "hatter2", &catalog)
            .iter()
            .any(|p| matches!(p.kind, inventory::PickupKind::Weapon(9))),
        "The authored Watch must also be collectible in its own level"
    );
    println!(
        "PASS: {} maps, {} visits, all pickup milestones and the scripted Eye Staff grant verified",
        maps.len(),
        ROUTE.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn weapons_arrive_after_their_acquisition_level() {
        let expected = [
            ("gvillage", [0; 10]),
            ("pandemonium", [1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ("skool1", [1, 1, 0, 0, 0, 0, 0, 0, 0, 0]),
            ("skool2", [1, 1, 1, 0, 0, 0, 0, 0, 0, 0]),
            ("potears3", [1, 1, 1, 0, 0, 0, 1, 0, 0, 0]),
            ("garden1", [1, 1, 1, 1, 0, 0, 1, 0, 0, 0]),
            ("garden4", [1, 1, 1, 1, 0, 0, 1, 0, 0, 0]),
            ("centipede1", [1, 1, 1, 1, 1, 0, 1, 0, 0, 0]),
            ("wforest", [1, 1, 1, 1, 1, 0, 2, 0, 0, 0]),
            ("funhouse", [1, 1, 1, 1, 1, 0, 3, 0, 0, 0]),
            ("hatter2", [1, 1, 1, 1, 1, 1, 3, 0, 0, 0]),
            ("jlair2", [1, 1, 1, 1, 1, 1, 3, 0, 0, 1]),
            ("hedge1", [1, 1, 1, 1, 1, 1, 3, 1, 1, 1]),
            ("qlair", [1, 1, 1, 1, 1, 1, 3, 1, 1, 1]),
        ];
        for (m, owned) in expected {
            assert_eq!(loadout(m, None), Some(owned), "{m}");
        }
        assert!(loadout("custom", None).is_none());
    }
    #[test]
    fn all_visit_keys_are_unique_and_follow_the_save_identity() {
        let keys = visits();
        assert_eq!(keys.len(), 39);
        assert_eq!(
            keys.iter().collect::<std::collections::BTreeSet<_>>().len(),
            39,
            "two visits share one save identity"
        );
        for (i, &(map, entry)) in ROUTE.iter().enumerate() {
            assert_eq!(keys[i], save::visit_key(map, entry));
            assert_eq!(visit_index(map, entry), Some(i), "{map} {entry:?}");
        }
        // The three repeated maps keep distinct first and return visits.
        assert_eq!(visit_index("fortress1", None), Some(2));
        assert_eq!(visit_index("fortress1", Some("fortress1_start2")), Some(4));
        assert_eq!(visit_index("skool1", Some("skool1_start2")), Some(7));
        assert_eq!(visit_index("wforest", Some("wforest_start2")), Some(27));
        // Named first entrances alias the map's first visit, as in a save.
        assert_eq!(visit_index("fortress1", Some("fortress1_start1")), Some(2));
        assert_eq!(visit_index("skool1", Some("skool1_start1")), Some(5));
        assert_eq!(visit_index("gvillage", Some("player_start")), Some(0));
        assert_eq!(visit_index("wforest", Some("wforest_start1")), Some(18));
        assert_eq!(visit_index("custom", None), None);
    }
    #[test]
    fn the_eight_opening_exits_normalize_to_the_next_visit() {
        // Appendix A rows 01 to 08: each exit names a destination whose visit key is the next
        // `ROUTE` entry, however the level change spells its entrance.
        let exits: [(&str, Option<&str>); 8] = [
            ("pandemonium", Some("player_start")),
            ("fortress1", Some("fortress1_start1")),
            ("fortress2", None),
            ("fortress1", Some("fortress1_start2")),
            ("skool1", Some("skool1_start1")),
            ("skool2", Some("skool2_start1")),
            ("skool1", Some("skool1_start2")),
            ("potears1", Some("potears1_start1")),
        ];
        for (i, (map, entry)) in exits.into_iter().enumerate() {
            assert_eq!(visit_index(map, entry), Some(i + 1), "exit of visit {i}");
            let parsed = interaction::destination(&match entry {
                Some(e) => format!("{map}${e}"),
                None => map.to_owned(),
            })
            .unwrap();
            assert_eq!(visit_index(&parsed.0, parsed.1.as_deref()), Some(i + 1));
        }
    }
    #[test]
    fn an_obstructed_entrance_fails_the_exit_instead_of_falling_back_to_flight() {
        use macroquad::prelude::vec3;
        // A slab of floor with a solid block standing where Alice would arrive.
        let floor = (vec3(-200., -200., -32.), vec3(200., 200., 0.));
        let clear = World::fixture(&[floor]);
        let eye = vec3(0., 0., crate::movement::EYE_HEIGHT);
        assert!(clear_entrance(&clear, eye, "fixture$first").is_ok());
        let blocked = World::fixture(&[floor, (vec3(-64., -64., 0.), vec3(64., 64., 400.))]);
        let error = clear_entrance(&blocked, eye, "fixture$first").unwrap_err();
        assert!(error.to_string().contains("fixture$first is obstructed"));
    }
    #[test]
    fn a_strict_arrival_grants_only_what_the_visit_itself_grants() {
        let mut strict = Stats::for_level("gvillage", None);
        let mut filled = Stats::for_level("gvillage", None);
        assert!(!strict.turtle_air && strict.equipped().is_none());
        strict.arrival_grants("utemple");
        filled.ensure_level_weapons("utemple", None);
        // The temple's shell is an authored arrival grant, whether or not the baseline is filled.
        assert!(strict.turtle_air && filled.turtle_air);
        // Only the baseline fill gives the toys of the earlier visits.
        assert_eq!((0..10).map(|i| strict.copies(i)).sum::<u8>(), 0);
        assert_eq!(filled.copies(1), 1);
        assert_eq!(filled.copies(6), 1);
        // Other maps grant nothing on their own.
        let mut other = Stats::for_level("gvillage", None);
        other.arrival_grants("skool1");
        assert!(!other.turtle_air);
    }
    #[test]
    fn return_visits_do_not_unlock_later_weapons_on_first_visit() {
        let school = loadout("skool1", Some("skool1_start2")).unwrap();
        assert_eq!((school[2], school[6]), (1, 1));
        let woods = loadout("wforest", Some("wforest_start2")).unwrap();
        assert_eq!(woods, [1, 1, 1, 1, 1, 1, 3, 1, 0, 1]);
        assert_eq!(
            loadout("wforest", Some("wforest_start1")),
            loadout("wforest", None)
        );
        assert_eq!(
            loadout("fortress1", Some("fortress1_start2")),
            loadout("fortress1", None)
        );
        assert_eq!(return_entry("wforest"), Some("wforest_start2"));
        assert_eq!(return_entry("garden1"), None);
        assert!(ROUTE.windows(2).all(|p| {
            let a = loadout(p[0].0, p[0].1).unwrap();
            let b = loadout(p[1].0, p[1].1).unwrap();
            a.iter().zip(b).all(|(&a, b)| a <= b)
        }));
    }
}

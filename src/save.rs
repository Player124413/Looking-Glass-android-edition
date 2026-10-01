//! Versioned local saves. Only gameplay state is stored; assets and collision are rebuilt.
use crate::{
    assets::Assets, bsp::Bsp, character::Character, cheshire::Hints, collision::World,
    interaction::Interactions, inventory::Stats, movement::Player, npc::Npcs, recovery::Recovery,
    render::Scene, story::Story,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Format 12 is the only bump the campaign makes (DG-6, F2): it adds the registered
/// controllers' state (`interactions.levels`). Older executables then refuse such a file with a
/// version message instead of failing later on an event-signature mismatch.
const VERSION: u32 = 12;
/// The first format whose files may carry a non-empty `interactions.levels` map.
const LEVELS_SINCE: u32 = 12;
pub(crate) const LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Quick,
    Auto,
    One,
    Two,
    Three,
    Four,
}
impl Slot {
    pub const ALL: [Self; 6] = [
        Self::One,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Quick,
        Self::Auto,
    ];
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "quick" => Ok(Self::Quick),
            "auto" => Ok(Self::Auto),
            "slot1" => Ok(Self::One),
            "slot2" => Ok(Self::Two),
            "slot3" => Ok(Self::Three),
            "slot4" => Ok(Self::Four),
            _ => anyhow::bail!("Use quick, auto or slot1-slot4"),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Auto => "auto",
            Self::One => "slot1",
            Self::Two => "slot2",
            Self::Three => "slot3",
            Self::Four => "slot4",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::One => "Slot 1",
            Self::Two => "Slot 2",
            Self::Three => "Slot 3",
            Self::Four => "Slot 4",
            Self::Quick => "Quick save",
            Self::Auto => "Automatic save",
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Campaign {
    pub levels: BTreeMap<String, Level>,
    pub completed: BTreeSet<String>,
}
/// The supported return visits have distinct setup and must never overwrite first visits.
/// Named first entrances and the default entrance share the same logical visit.
pub fn visit_key(map: &str, entry: Option<&str>) -> String {
    format!(
        "{map}${}",
        if entry.is_some() && entry == crate::campaign::return_entry(map) {
            "return"
        } else {
            "first"
        }
    )
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Level {
    pub map: String,
    pub entry: Option<String>,
    pub interactions: crate::interaction::Snapshot,
    pub npcs: crate::npc::Snapshot,
    pub story: crate::story::Snapshot,
    pub hints: crate::cheshire::Snapshot,
    pub environment_clock: f32,
    pub pickup_clock: f32,
}
impl Level {
    pub fn key(&self) -> String {
        visit_key(&self.map, self.entry.as_deref())
    }
    pub fn restore_logic(
        &self,
        assets: &mut Assets,
        map: &Bsp,
    ) -> Result<(Interactions, Story, Hints)> {
        validate_entry(map, self.entry.as_deref())?;
        let mut interactions = Interactions::load(map)?;
        interactions.set_entry(assets, map, &self.map, self.entry.as_deref())?;
        interactions.restore(&self.interactions, map)?;
        let mut hints = Hints::load(assets, map, &self.map)?;
        hints.restore(&self.hints)?;
        hints.sync(&interactions);
        let mut story = Story::load(assets, &self.map);
        story.restore(&self.story, &hints)?;
        Ok((interactions, story, hints))
    }
}
pub fn validate_entry(map: &Bsp, entry: Option<&str>) -> Result<()> {
    if let Some(entry) = entry {
        ensure!(
            map.entities.iter().any(|e| e
                .get("classname")
                .is_some_and(|s| s == "info_player_start")
                && e.get("targetname").is_some_and(|s| s == entry)),
            "Saved entrance is missing from the level"
        );
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize)]
pub struct View {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub third_person: bool,
    pub flying: bool,
    pub fullbright: bool,
    pub spawn_landing: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Game {
    pub current: String,
    pub campaign: Campaign,
    pub stats: Stats,
    pub player: Player,
    pub view: View,
    pub recovery: Recovery,
    pub character: crate::character::Snapshot,
}
impl Game {
    pub fn level(&self) -> Result<&Level> {
        self.campaign
            .levels
            .get(&self.current)
            .context("Saved current level is missing")
    }
    pub fn validate(&self) -> Result<()> {
        self.stats.validate_save()?;
        ensure!(
            self.campaign.levels.len() <= 72 && self.campaign.completed.len() <= 72,
            "Saved campaign is too large"
        );
        for (key, level) in &self.campaign.levels {
            ensure!(
                *key == level.key()
                    && crate::interaction::destination(&level.map).is_some()
                    && level.environment_clock >= 0.
                    && level.pickup_clock >= 0.,
                "Invalid saved campaign visit"
            );
        }
        ensure!(
            self.campaign
                .completed
                .iter()
                .all(|k| self.campaign.levels.contains_key(k)),
            "Saved campaign completion lacks level state"
        );
        self.level()?;
        ensure!(
            self.player.feet.is_finite()
                && self.player.velocity.is_finite()
                && self.view.position.is_finite()
                && self.view.yaw.is_finite()
                && self.view.pitch.is_finite()
                && self.view.pitch.abs() <= 1.56,
            "Invalid saved player position/view"
        );
        self.player.validate_save()?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    game_data: String,
    saved_at: u64,
    checksum: String,
    payload: serde_json::Value,
    // Cosmetic metadata shares the atomic file/backup but never invalidates
    // gameplay state when absent or malformed. Older readers ignore this field.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    preview: serde_json::Value,
}
pub struct Loaded {
    pub game: Game,
    pub saved_at: u64,
    pub backup: bool,
    pub preview: Option<crate::save_preview::Preview>,
}
pub struct Store {
    pub directory: PathBuf,
    fingerprint: String,
}
impl Store {
    pub fn new(directory: PathBuf, fingerprint: String) -> Self {
        Self {
            directory,
            fingerprint,
        }
    }
    fn path(&self, slot: Slot, backup: bool) -> PathBuf {
        self.directory.join(format!(
            "{}{}.json",
            slot.name(),
            if backup { ".previous" } else { "" }
        ))
    }
    pub fn exists(&self, slot: Slot) -> bool {
        self.path(slot, false).is_file() || self.path(slot, true).is_file()
    }
    pub fn write(&self, slot: Slot, game: &Game) -> Result<()> {
        self.write_with_preview(slot, game, None)
    }
    pub fn write_with_preview(
        &self,
        slot: Slot,
        game: &Game,
        preview: Option<&crate::save_preview::Preview>,
    ) -> Result<()> {
        game.validate()?;
        ensure!(
            game.campaign
                .levels
                .values()
                .all(|l| l.map != "potears3" || l.interactions.has_duchess()),
            "Saved Duchess state is missing"
        );
        ensure!(
            game.campaign
                .levels
                .values()
                .all(|l| l.map != "pandemonium" || l.interactions.has_pandemonium_cinema()),
            "Saved cinematic state is missing"
        );
        ensure!(
            game.campaign
                .levels
                .values()
                .all(|l| l.key() != "skool1$return" || l.interactions.has_school_return()),
            "Saved observatory state is missing"
        );
        ensure!(
            game.campaign
                .levels
                .values()
                .all(|l| l.interactions.has_shared_state()),
            "Load legacy level state before saving in the current format"
        );
        let payload = serde_json::to_value(game)?;
        validate_json(&payload)?;
        // The writer guard: registered state never leaves in a format that predates it.
        require_format_for_levels(VERSION, &payload)?;
        let data = serde_json::to_vec(&payload)?;
        let envelope = Envelope {
            format: "looking-glass-save".into(),
            version: VERSION,
            game_data: self.fingerprint.clone(),
            saved_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
            checksum: format!("{:x}", Sha256::digest(&data)),
            payload,
            preview: preview
                .map(serde_json::to_value)
                .transpose()?
                .unwrap_or_default(),
        };
        let bytes = serde_json::to_vec(&envelope)?;
        ensure!(bytes.len() as u64 <= LIMIT, "Save exceeds size limit");
        fs::create_dir_all(&self.directory).context("Cannot create save folder")?;
        let path = self.path(slot, false);
        // Never replace a usable backup with a damaged primary file.
        if let Ok(previous) = read_limited(&path) {
            if self.decode(&previous).is_ok() {
                atomic_write(&self.path(slot, true), &previous)?;
            }
        }
        atomic_write(&path, &bytes)
            .context("Could not write save; the previous save remains available")?;
        println!(
            "Saved {}: {} ({})",
            slot.name(),
            game.current,
            path.display()
        );
        Ok(())
    }
    fn decode(&self, bytes: &[u8]) -> Result<Loaded> {
        let e: Envelope =
            serde_json::from_slice(bytes).context("Save file is damaged or incomplete")?;
        ensure!(
            e.format == "looking-glass-save" && (1..=VERSION).contains(&e.version),
            "This save uses an unsupported format version"
        );
        ensure!(
            e.game_data == self.fingerprint,
            "This save belongs to different game data"
        );
        let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&e.payload)?));
        ensure!(digest == e.checksum, "Save integrity check failed");
        validate_json(&e.payload)?;
        require_format_for_levels(e.version, &e.payload)?;
        ensure!(
            e.version < 3
                || (e.payload["character"]["projectiles"]["dice"].is_object()
                    && e.payload["character"]["projectiles"]["dice_count"].is_u64()),
            "Saved Demon Dice state is missing"
        );
        ensure!(
            e.version < 9
                || (e.payload["player"]["breath"].is_object()
                    && e.payload["stats"]["turtle_air"].is_boolean()),
            "Saved breath state is missing"
        );
        ensure!(
            e.version < 10
                || (e.payload["stats"]["powers"].is_object()
                    && e.payload["stats"]["loot"].is_object()
                    && e.payload["stats"]["difficulty"].is_string()),
            "Saved power-up state is missing"
        );
        ensure!(
            e.version < 11 || e.payload["stats"]["notarget"].is_boolean(),
            "Saved console targeting flag is missing"
        );
        let mut game: Game =
            serde_json::from_value(e.payload).context("Save contains invalid gameplay state")?;
        if e.version < 9 {
            let level = game.level()?;
            game.stats.turtle_air =
                crate::campaign::after_temple(&level.map, level.entry.as_deref())
                    || game.campaign.levels.values().any(|l| l.map == "utemple");
            game.player.breath.shell = game.stats.turtle_air;
        }
        game.validate()?;
        ensure!(
            e.version < 8
                || game
                    .campaign
                    .levels
                    .values()
                    .all(|l| l.map != "potears3" || l.interactions.has_duchess()),
            "Saved Duchess state is missing"
        );
        ensure!(
            e.version < 7
                || game
                    .campaign
                    .levels
                    .values()
                    .all(|l| l.map != "pandemonium" || l.interactions.has_pandemonium_cinema()),
            "Saved cinematic state is missing"
        );
        ensure!(
            e.version < 6
                || game
                    .campaign
                    .levels
                    .values()
                    .all(|l| l.key() != "skool1$return" || l.interactions.has_school_return()),
            "Saved observatory state is missing"
        );
        ensure!(
            e.version < 4
                || game
                    .campaign
                    .levels
                    .values()
                    .all(|l| l.map != "potears1" || l.interactions.has_encounters()),
            "Saved Ladybug encounter state is missing"
        );
        ensure!(
            e.version < 5
                || game.campaign.levels.values().all(|l| l.map != "pandemonium"
                    || (l.interactions.has_pandemonium() && l.interactions.has_encounters())),
            "Saved Pandemonium state is missing"
        );
        ensure!(
            e.version == 1
                || game
                    .campaign
                    .levels
                    .values()
                    .all(|l| l.interactions.has_shared_state()),
            "Saved shared entity state is missing"
        );
        Ok(Loaded {
            game,
            saved_at: e.saved_at,
            backup: false,
            preview: crate::save_preview::Preview::from_value(e.preview),
        })
    }
    pub fn read(&self, slot: Slot) -> Result<Loaded> {
        match read_limited(&self.path(slot, false)).and_then(|b| self.decode(&b)) {
            Ok(s) => Ok(s),
            Err(primary) => {
                match read_limited(&self.path(slot, true)).and_then(|b| self.decode(&b)) {
                    Ok(mut s) => {
                        s.backup = true;
                        Ok(s)
                    }
                    Err(_) => Err(primary).context(format!("Cannot load {} save", slot.name())),
                }
            }
        }
    }
    pub fn latest(&self) -> Option<(Slot, Loaded)> {
        let latest = Slot::ALL
            .into_iter()
            .filter_map(|slot| self.read(slot).ok().map(|s| (slot, s)))
            .max_by_key(|(_, s)| s.saved_at);
        // Completion leaves the six player slots intact. Continue can replay
        // the final encounter from its separate, pre-ending checkpoint.
        if let Ok(bytes) = read_limited(&self.directory.join("campaign-complete.json")) {
            if let Ok(at) = serde_json::from_slice::<u64>(&bytes) {
                if latest.as_ref().is_none_or(|(_, s)| s.saved_at <= at) {
                    if let Ok(mut saved) = self.finale_store().read(Slot::Auto) {
                        saved.game.campaign.completed.insert("qlair$first".into());
                        return Some((Slot::Auto, saved));
                    }
                }
            }
        }
        latest
    }
    fn finale_store(&self) -> Self {
        Self::new(
            self.directory.join("finale-checkpoint"),
            self.fingerprint.clone(),
        )
    }
    pub fn write_finale_checkpoint(&self, game: &Game) -> Result<()> {
        self.finale_store().write(Slot::Auto, game)?;
        self.write(Slot::Auto, game)
    }
    pub fn mark_complete(&self) -> Result<()> {
        std::fs::create_dir_all(&self.directory)?;
        let at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        atomic_write(
            &self.directory.join("campaign-complete.json"),
            &serde_json::to_vec(&at)?,
        )
    }
}
fn read_limited(path: &Path) -> Result<Vec<u8>> {
    let file =
        File::open(path).with_context(|| format!("No readable save at {}", path.display()))?;
    ensure!(file.metadata()?.len() <= LIMIT, "Save file is too large");
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= LIMIT, "Save file is too large");
    Ok(bytes)
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("json.{}.tmp", std::process::id()));
    // A per-process temporary file cannot replace the committed save before flush succeeds.
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}
/// Whether any cached visit holds registered-controller state. It fails closed: a `levels` value
/// that is anything but absent, null or an empty object counts as state.
fn holds_registry_state(payload: &serde_json::Value) -> bool {
    payload["campaign"]["levels"]
        .as_object()
        .is_some_and(|visits| {
            visits
                .values()
                .any(|visit| match &visit["interactions"]["levels"] {
                    serde_json::Value::Null => false,
                    serde_json::Value::Object(map) => !map.is_empty(),
                    _ => true,
                })
        })
}
/// Registered-controller state exists only in format 12 and later. The reader rejects it in an
/// older file; the writer never writes it under an older version.
fn require_format_for_levels(version: u32, payload: &serde_json::Value) -> Result<()> {
    ensure!(
        version >= LEVELS_SINCE || !holds_registry_state(payload),
        "Saved level-controller state needs save format {LEVELS_SINCE}"
    );
    Ok(())
}
pub(crate) fn validate_json(value: &serde_json::Value) -> Result<()> {
    match value {
        serde_json::Value::Number(n) => ensure!(
            n.as_f64().is_some_and(|v| v.is_finite() && v.abs() <= 1e12),
            "Invalid save number"
        ),
        serde_json::Value::String(s) => ensure!(s.len() <= 1024, "Invalid save string"),
        serde_json::Value::Array(a) => {
            ensure!(a.len() <= 10000, "Invalid save array");
            for v in a {
                validate_json(v)?;
            }
        }
        serde_json::Value::Object(o) => {
            for v in o.values() {
                validate_json(v)?;
            }
        }
        _ => (),
    }
    Ok(())
}

/// Construct a replacement scene first. A failed load cannot partially mutate live play.
pub struct Restored {
    pub scene: Scene,
    pub interactions: Interactions,
    pub story: Story,
    pub hints: Hints,
    pub npcs: Npcs,
    pub alice: Character,
    pub game: Game,
    pub steam: crate::particles::Steam,
}
impl Level {
    /// A cached visit whose saved logic predates a shared-state, traversal or controller upgrade.
    fn needs_upgrade(&self) -> bool {
        let i = &self.interactions;
        !i.has_traversal()
            || !i.has_shared_state()
            || (self.map == "potears3" && !i.has_duchess())
            || (self.map == "fortress1" && !i.has_fortress())
            || (self.map == "fortress2" && !i.has_beyond())
            || (self.map == "potears1" && !i.has_encounters())
            || (self.map == "skool2" && !i.has_encounters())
            || (self.map == "potears1" && !i.has_pool())
            || (self.map == "potears1" && !i.has_pool_pilot())
            || (self.map == "potears1" && !i.has_pool_arrival())
            || (self.map == "potears1" && !i.has_pool_boulders())
            || (self.map == "pandemonium" && !i.has_pandemonium_cinema())
            || (self.key() == "skool1$return" && !i.has_school_return())
            || self.registry_upgrade()
    }
    /// A registered controller now serves this visit that the save has no state for (F2).
    fn registry_upgrade(&self) -> bool {
        self.interactions
            .registry_upgrade(&self.map, self.entry.as_deref())
    }
    /// Rebuild a cached visit's logic in the current format. The fortress cast is regenerated
    /// from its models, which needs a window: window-free callers pass `regenerate_cast: false`.
    fn upgrade(
        &mut self,
        assets: &mut Assets,
        difficulty: crate::powerups::Difficulty,
        regenerate_cast: bool,
    ) -> Result<()> {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", self.map))?)?;
        map.difficulty = difficulty;
        let registry = self.registry_upgrade();
        let school2 = self.map == "skool2" && !self.interactions.has_encounters();
        let (mut i, _, _) = self.restore_logic(assets, &map)?;
        if school2 {
            // Keep legacy decorative slots so saved Club hit/loot identities do
            // not shift. The rebuilt NPC owner suppresses Diamonds everywhere.
            self.npcs
                .matches_placements(&map, "skool2", self.entry.as_deref())?;
            if regenerate_cast {
                let fresh =
                    Npcs::load(assets, &map, &self.map, self.entry.as_deref(), false, false)?
                        .snapshot();
                self.npcs = self.npcs.adopt(fresh, &[]);
            }
        }
        if self.map == "pandemonium" {
            if let Some(c) = &mut i.encounters {
                self.npcs.import_guards(c);
            }
        }
        if regenerate_cast && matches!(self.map.as_str(), "fortress1" | "fortress2") {
            self.npcs = Npcs::load(assets, &map, &self.map, self.entry.as_deref(), false, false)?
                .snapshot();
        }
        // F2 step 7: a controller that took actors over changes who the generic cast simulates.
        // The saved cast is regenerated from the models and keeps the state of every actor the
        // controller does not own.
        if regenerate_cast && registry {
            let serving =
                crate::levels::serving(&self.map, self.entry.as_deref()).collect::<Vec<_>>();
            let entry = self.entry.clone();
            if let Some(cast) = adopt_cast(&self.npcs, &serving, || {
                Ok(Npcs::load(assets, &map, &self.map, entry.as_deref(), false, false)?.snapshot())
            })? {
                self.npcs = cast;
            }
        }
        self.interactions = i.snapshot();
        Ok(())
    }
}

/// F2 step 7: the saved cast after registered controllers took actors over, or `None` when they
/// own none of it and the saved cast stands. `fresh` loads the cast a new visit spawns (it needs
/// the NPC models, so it is only called when something changed).
fn adopt_cast(
    saved: &crate::npc::Snapshot,
    serving: &[&'static crate::levels::Registration],
    fresh: impl FnOnce() -> Result<crate::npc::Snapshot>,
) -> Result<Option<crate::npc::Snapshot>> {
    if !saved.owned_by(serving) {
        return Ok(None);
    }
    Ok(Some(saved.adopt(fresh()?, serving)))
}

/// Visits whose older saves start at the level entrance, because their movement was rebuilt.
#[derive(Clone, Copy)]
struct Migrations {
    pool: bool,
    beyond: bool,
    fortress: bool,
    duchess: bool,
    /// A registered controller joined the visit after the save was made (F2).
    levels: bool,
}
/// What a restore needs from the map's logic and collision alone: no window, model or texture.
pub struct Logic {
    pub interactions: Interactions,
    pub story: Story,
    pub hints: Hints,
}
/// Validation and cached-visit upgrades shared by `Restored::build` and `rebuild_headless`.
/// Returns the current visit and the player-moving migrations that apply to it.
fn prepare(
    assets: &mut Assets,
    game: &mut Game,
    regenerate_cast: bool,
) -> Result<(Level, Migrations)> {
    game.validate()?;
    let migrate = Migrations {
        pool: game.level()?.map == "potears1" && !game.level()?.interactions.has_pool(),
        beyond: game.level()?.map == "fortress2" && !game.level()?.interactions.has_beyond(),
        fortress: game.level()?.map == "fortress1" && !game.level()?.interactions.has_fortress(),
        duchess: game.level()?.map == "potears3" && !game.level()?.interactions.has_duchess(),
        levels: game.level()?.registry_upgrade(),
    };
    // Upgrade every cached visit, including newly supported encounter ownership.
    let difficulty = game.stats.difficulty;
    for level in game
        .campaign
        .levels
        .values_mut()
        .filter(|l| l.needs_upgrade())
    {
        level.upgrade(assets, difficulty, regenerate_cast)?;
    }
    let level = game.level()?.clone();
    let maps = assets.maps();
    ensure!(
        game.campaign.levels.values().all(|l| maps.contains(&l.map)),
        "A saved campaign level is unavailable"
    );
    Ok((level, migrate))
}
/// Restore the current visit's logic against its map and collision world, then validate and
/// (for migrated visits) reset the player. Shared by `Restored::build` and `rebuild_headless`.
fn resume(
    assets: &mut Assets,
    game: &mut Game,
    level: &Level,
    map: &Bsp,
    world: &mut World,
    migrate: Migrations,
) -> Result<Logic> {
    let (mut interactions, story, hints) = level.restore_logic(assets, map)?;
    for slot in &mut interactions.levels { slot.ctl.restore_position(&game.player, map)?; }
    settle(
        game,
        level.entry.as_deref(),
        &interactions,
        map,
        world,
        migrate,
    )?;
    Ok(Logic {
        interactions,
        story,
        hints,
    })
}
/// Put Alice back in a restored visit: sync the world, validate her, move her to the entrance
/// where a migration or an upgrade says so, and refuse a position that cannot stand.
fn settle(
    game: &mut Game,
    entry: Option<&str>,
    interactions: &Interactions,
    map: &Bsp,
    world: &mut World,
    migrate: Migrations,
) -> Result<()> {
    interactions.sync(world);
    game.stats.prepare_player(&mut game.player);
    game.player.validate_world(world)?;
    // F2 step 6: a visit a controller joined restarts at the entrance instead of failing when the
    // controller rejects the saved position or her body is not clear there.
    let joined =
        migrate.levels && interactions.levels_respawn(&game.player, world, game.view.flying);
    if migrate.beyond
        || migrate.pool
        || (migrate.duchess && !world.body_clear(game.player.feet))
        || (migrate.fortress && (game.player.feet.x > 2000. || !world.body_clear(game.player.feet)))
        || joined
    {
        let (eye, yaw) = crate::interaction::spawn(map, entry);
        game.player = Player::spawn(world, eye).context("Migrated level entrance obstructed")?;
        game.stats.prepare_player(&mut game.player);
        game.view.position = game.player.eye();
        game.view.yaw = yaw;
        game.recovery.clear();
    }
    if let Some(p) = &interactions.pandemonium {
        p.validate_player(&game.player)?;
    }
    interactions.validate_levels(&game.player)?;
    ensure!(
        game.view.flying
            || interactions
                .village
                .as_ref()
                .is_some_and(|v| v.cinema.active())
            || interactions.duchess.as_ref().is_some_and(|d| d.cinematic())
            || interactions
                .pandemonium
                .as_ref()
                .is_some_and(|p| p.cinematic())
            || interactions.school.as_ref().is_some_and(|s| s.cinematic())
            || interactions.levels_scripted()
            || world.body_clear(game.player.feet),
        "The saved player position is obstructed"
    );
    Ok(())
}
/// The window-free part of `Restored::build`, in the same order: validation, cached-visit
/// upgrades, then the current visit's logic against its collision world. Only the fortress
/// cast regeneration, the scene, NPC models, Alice and steam need a window and are left out.
pub struct Headless {
    pub game: Game,
    pub level: Level,
    pub map: Bsp,
    pub world: World,
    pub logic: Logic,
}
pub fn rebuild_headless(assets: &mut Assets, mut game: Game) -> Result<Headless> {
    let (level, migrate) = prepare(assets, &mut game, false)?;
    let mut map = Bsp::parse(&assets.read(&format!("maps/{}.bsp", level.map))?)?;
    // `Scene::load` builds its world before the saved difficulty is applied to the map.
    let mut world = World::from_bsp(&map)?;
    map.difficulty = game.stats.difficulty;
    let logic = resume(assets, &mut game, &level, &map, &mut world, migrate)?;
    Ok(Headless {
        game,
        level,
        map,
        world,
        logic,
    })
}
impl Restored {
    pub fn build(assets: &mut Assets, mut game: Game) -> Result<Self> {
        let (level, migrate) = prepare(assets, &mut game, true)?;
        let mut scene = Scene::load(assets, &level.map)?;
        scene.map.difficulty = game.stats.difficulty;
        let Logic {
            interactions,
            story,
            hints,
        } = resume(
            assets,
            &mut game,
            &level,
            &scene.map,
            &mut scene.world,
            migrate,
        )?;
        let mut npcs = Npcs::load(
            assets,
            &scene.map,
            &level.map,
            level.entry.as_deref(),
            false,
            false,
        )?;
        npcs.restore(&level.npcs)?;
        let mut alice = Character::load(assets)?;
        alice.restore(&game.character)?;
        scene.world.set_weapon_obstacles(alice.ice_targets());
        alice.power_appearance(&game.stats);
        let steam = crate::particles::Steam::load(assets, &scene.map)?;
        Ok(Self {
            scene,
            interactions,
            story,
            hints,
            npcs,
            alice,
            game,
            steam,
        })
    }
}

#[cfg(test)]
mod format12;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_slots_have_separate_files_and_cannot_escape_save_directory() {
        let store = Store::new(PathBuf::from("test-saves"), "fixture".into());
        let paths = Slot::ALL.map(|slot| store.path(slot, false));
        assert_eq!(
            paths.iter().collect::<std::collections::HashSet<_>>().len(),
            6
        );
        for slot in Slot::ALL {
            assert_eq!(Slot::parse(slot.name()).unwrap(), slot);
            assert_eq!(
                store.path(slot, false).parent(),
                Some(Path::new("test-saves"))
            );
            assert_ne!(store.path(slot, false), store.path(slot, true));
        }
        for invalid in ["slot0", "slot5", "../quick", "C:\\save"] {
            assert!(Slot::parse(invalid).is_err());
        }
    }
    #[test]
    fn campaign_cache_unifies_first_entrances_without_overwriting_return_visits() {
        for (map, first, returning) in [
            ("skool1", "skool1_start1", "skool1_start2"),
            ("fortress1", "fortress1_start1", "fortress1_start2"),
            ("wforest", "wforest_start1", "wforest_start2"),
        ] {
            assert_eq!(visit_key(map, None), visit_key(map, Some(first)));
            assert_ne!(visit_key(map, None), visit_key(map, Some(returning)));
        }
        assert_ne!(visit_key("skool1", None), visit_key("skool2", None));
    }
}

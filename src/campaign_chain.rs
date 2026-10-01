//! The campaign chain (F3b, `docs/CAMPAIGN.md`): `--campaign-route-check`, headless.
//!
//! The chain runs the recorded route legs (`campaign_route::LEGS`) back to back on one `Stats` and
//! one ledger of finished visits, through the shared transition `campaign::arrive`, and reports
//! how far New Game state gets. It prints one `PASS` line per leg with its metrics, a
//! reward-provenance report (`provenance`), and `FRONTIER <map$entry>`: the first visit the chain
//! cannot complete, with the failing leg's message or, for a visit without a driver yet, the start
//! thread nothing handles. It writes a checkpoint per visit and a `report.json` under
//! `private/campaign-chain/`, and it can resume from a checkpoint.
//!
//! * `--campaign-from <n|map[$entry]>` and `--campaign-to <n|map[$entry]>` bound the legs (visit
//!   numbers count from 1, `--campaign-to` is inclusive). Without `--campaign-to` the chain runs as
//!   far as it can and reports the frontier; with it, the run fails unless that leg passes.
//! * `--difficulty <d>` is the chain's difficulty.
//! * `--campaign-skip-cinematics` skips every scene the way holding Enter does.
//! * `--campaign-strict` skips the `campaign::loadout` baseline fill of every exit and starts
//!   nothing from a chapter loadout: every toy must come from its authored source, and a missing
//!   required grant fails the leg. Without it the chain fills the baseline like the viewer.
//! * `--campaign-allow-retry` restores Alice's Sanity and Will and drives a failed leg again
//!   (twice at most). It is diagnosis only: the run is always reported as failing.
//! * `--campaign-expect frontier=<visit>` or `no-checkpoint` pins an honest failure. The chain
//!   still reports exactly what it found (the FRONTIER line, the failing leg's message, the
//!   report), but the process exits 0 only while the pinned failure is what happened: the chain
//!   stops short of its range with its frontier at that visit, or a strict chain refuses to start
//!   from a visit that has no checkpoint. It exits 1 the day the frontier moves, and it never
//!   turns a chain that completed its range, or a diagnosis run, into a pass: the report of a
//!   retry run says `diagnosis` whatever is expected.
//!
//! Save and continue (item 7) is part of every run: at each boundary the entrance is serialized
//! (Stats, player, controllers, story, hints and the ledger), a second route is rebuilt from the
//! text, and both run 600 ticks of identical input; in every leg that passes, the leg is driven
//! again to its midpoint and the same is done there. They must stay equal. The ledger stays inside
//! the save limits (8 MiB, 72 cached visits, 10,000 elements per array).
//!
//! Limits: the drivers are the recorded ones (auto-aimed Blade throws only, no `Npcs`
//! simulation apart from the headless guards); this harness does not retune them.
mod graph;
pub(crate) mod provenance;
pub(crate) mod save_chain;
mod script_facts;

pub use graph::check as graph_check;
pub use save_chain::check as save_chain_check;

use crate::{
    assets::Assets,
    bsp::Bsp,
    campaign,
    campaign_route::LEGS,
    inventory::{Catalog, Stats},
    powerups::Difficulty,
    route::{Checkpoint, Metrics, Route},
    save,
};
use anyhow::{bail, ensure, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

/// Where checkpoints and the report go, relative to the working directory.
pub const DIR: &str = "private/campaign-chain";
/// Ticks of identical input each save/continue probe runs both routes for.
const PROBE_TICKS: usize = 600;
/// How long before its exit the windowed companions' pre-exit save is taken (1 s at 120 Hz).
const PRE_EXIT_TICKS: usize = 120;
const VISIT_FORMAT: &str = "campaign-chain-visit";

/// A named way through the chain: the legs that take their optional body, a collect-or-skip
/// choice. The base variant is the chain proper; the others run from the entrance of the first leg
/// where they differ, so a later visit's choice (the wforest return's Blunderbuss) only adds an
/// entry here.
pub struct Variant {
    pub name: &'static str,
    /// Positions in `LEGS` (0-based) whose optional body this variant drives.
    pub optional: &'static [usize],
}
pub const VARIANTS: [Variant; 2] = [
    Variant {
        name: "base",
        optional: &[],
    },
    // School one's secret room: the optional shelf, taken on the way.
    Variant {
        name: "skool1-secret",
        optional: &[5],
    },
];

#[derive(Default)]
pub struct Args {
    pub from: Option<String>,
    pub to: Option<String>,
    pub skip_cinematics: bool,
    pub strict: bool,
    pub allow_retry: bool,
    /// `--campaign-expect`: a failure this run is pinned to (`Expect::parse`).
    pub expect: Option<String>,
    /// Where the caller redirects this run's log, recorded in the report and the failure lines.
    pub log: Option<String>,
}

/// What a run asserts about its own ending. An expectation asserts a failure: it pins the frontier
/// a chain is known to stop at, or the refusal a strict chain is known to give, so that the check
/// of an honest failure exits 0 while the failure holds and exits 1 the day it does not.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Expect {
    /// The chain stops short of its range with its frontier at this visit (0-based).
    Frontier(usize),
    /// A strict chain refuses to start: the visit it is asked to resume from has no checkpoint.
    NoCheckpoint,
}

impl Expect {
    fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        if text == "no-checkpoint" {
            return Ok(Self::NoCheckpoint);
        }
        match text.split_once('=') {
            Some(("frontier", visit)) => Ok(Self::Frontier(parse_visit(visit)?)),
            _ => bail!(
                "--campaign-expect takes frontier=<n|map[$entrance]> or no-checkpoint, not {text:?}"
            ),
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Frontier(i) => format!("frontier={}", campaign::visits()[*i]),
            Self::NoCheckpoint => "no-checkpoint".into(),
        }
    }
}

/// A strict chain asked to resume from a visit that has no checkpoint: it never starts from a
/// chapter loadout. Typed so that `--campaign-expect no-checkpoint` pins this refusal and not
/// another.
#[derive(Debug)]
struct NoCheckpoint(usize);
impl std::fmt::Display for NoCheckpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "No checkpoint for visit {:02} in {DIR}: a strict chain never starts from a chapter loadout; run it from visit 1 or copy the checkpoint here",
            self.0
        )
    }
}
impl std::error::Error for NoCheckpoint {}

struct Config {
    difficulty: Difficulty,
    strict: bool,
    skip: bool,
    allow_retry: bool,
    fixture: bool,
}

/// Parse a visit: a number from 1, a map (its first visit), `map$entrance` (the entrance
/// normalized like a save's visit), or a visit key (`map$first`, `map$return`).
pub fn parse_visit(text: &str) -> Result<usize> {
    let text = text.trim();
    if let Ok(n) = text.parse::<usize>() {
        ensure!(
            (1..=campaign::route().len()).contains(&n),
            "Visit numbers run from 1 to {}",
            campaign::route().len()
        );
        return Ok(n - 1);
    }
    let (map, entry) = match text.split_once('$') {
        None => (text, None),
        Some((map, "first")) => (map, None),
        Some((map, "return")) => (
            map,
            Some(
                campaign::return_entry(map)
                    .with_context(|| format!("{map} has no return visit"))?,
            ),
        ),
        Some((map, entry)) => (map, Some(entry)),
    };
    campaign::visit_index(map, entry)
        .with_context(|| format!("{text} is not a visit of the campaign"))
}

/// A visit spelled `map$entrance`, with the map's first entrance named for a first visit.
fn spelled(assets: &mut Assets, index: usize) -> Result<String> {
    let (map, entry) = campaign::route()[index];
    if let Some(entry) = entry {
        return Ok(format!("{map}${entry}"));
    }
    let bsp = Bsp::parse(&assets.read(&format!("maps/{map}.bsp"))?)?;
    // A map that has a return visit also has its entrance, which is never the first visit's.
    let returning = campaign::return_entry(map);
    let first = bsp.entities.iter().find_map(|e| {
        if e.get("classname").is_none_or(|c| c != "info_player_start") {
            return None;
        }
        e.get("targetname")
            .filter(|n| Some(n.as_str()) != returning)
    });
    Ok(first.map_or_else(|| map.to_owned(), |name| format!("{map}${name}")))
}

fn file_name(index: usize) -> String {
    let key = &campaign::visits()[index];
    let (map, kind) = key.split_once('$').unwrap_or((key, "first"));
    format!("{:02}-{map}-{kind}.json", index + 1)
}
fn file_path(index: usize) -> PathBuf {
    Path::new(DIR).join(file_name(index))
}

/// What a visit's checkpoint file holds: the entrance, and afterwards what its leg did.
#[derive(Serialize, Deserialize)]
struct VisitFile {
    format: String,
    version: u32,
    /// 1-based visit number.
    index: usize,
    visit: String,
    spelled: String,
    strict: bool,
    skip_cinematics: bool,
    /// The chain started from a chapter's baseline loadout instead of New Game state.
    fixture: bool,
    /// Written by a `--campaign-allow-retry` run, which restores Sanity and Will: never proof.
    #[serde(default)]
    diagnosis: bool,
    entrance: Checkpoint,
    entrance_digest: String,
    #[serde(default)]
    leg: Option<LegFile>,
}

/// What the visit's leg did, written beside its entrance.
#[derive(Serialize, Deserialize)]
struct LegFile {
    status: String,
    retried: usize,
    message: Option<String>,
    metrics: Option<serde_json::Value>,
    exit_digest: Option<String>,
    mid_leg: Option<ContinueReport>,
    /// The leg driven again to one second before its exit, for the windowed companions' manual
    /// pre-exit save. Its ledger is left out: it is the entrance's.
    pre_exit: Option<Checkpoint>,
}

/// The save limits a ledger and a checkpoint must stay inside: 8 MiB, 72 cached visits and 10,000
/// elements per array (`save.rs`), using the save's own validator.
struct Sizes {
    visits: usize,
    bytes: usize,
}
fn within_limits(ledger: &save::Campaign, checkpoint: &Checkpoint) -> Result<Sizes> {
    ensure!(
        ledger.levels.len() <= 72 && ledger.completed.len() <= 72,
        "The ledger holds {} cached visits and {} completed; a save allows 72",
        ledger.levels.len(),
        ledger.completed.len()
    );
    let value = serde_json::to_value(ledger)?;
    save::validate_json(&value).context("The ledger breaks a save limit")?;
    let bytes = serde_json::to_vec(&value)?.len();
    ensure!(
        bytes as u64 <= save::LIMIT,
        "The ledger is {bytes} bytes; a save allows {}",
        save::LIMIT
    );
    let whole = serde_json::to_value(checkpoint)?;
    save::validate_json(&whole).context("The checkpoint breaks a save limit")?;
    ensure!(
        serde_json::to_vec(&whole)?.len() as u64 <= save::LIMIT,
        "The checkpoint exceeds the save size limit"
    );
    Ok(Sizes {
        visits: ledger.levels.len(),
        bytes,
    })
}

#[derive(Clone, Serialize, Deserialize)]
struct ContinueReport {
    /// The tick of the visit at which the checkpoint was taken.
    at_tick: usize,
    /// Ticks both routes ran together.
    ticks: usize,
    /// Both routes ended together at this tick with this error, if they did.
    ended: Option<String>,
    /// State hash of the rebuilt route at the checkpoint.
    digest: String,
    ledger_visits: usize,
    ledger_bytes: usize,
}

/// A checkpoint written as text and read back, so that the file format is what is proven.
fn round_trip(checkpoint: &Checkpoint) -> Result<Checkpoint> {
    let text = serde_json::to_string(checkpoint)?;
    Ok(serde_json::from_str(&text)?)
}

/// Compare a rebuilt route with the one it was taken from, then run both on identical input.
fn continue_identically(
    a: &mut Route,
    b: &mut Route,
    at_tick: usize,
    sizes: Sizes,
) -> Result<ContinueReport> {
    ensure!(
        a.state()? == b.state()?,
        "The rebuilt route differs from the route it was taken from at tick {at_tick}"
    );
    let digest = b.state_hash()?;
    // Both routes take the same input, cinematic skipping included.
    b.skip_cinematics = a.skip_cinematics;
    for r in [&mut *a, &mut *b] {
        r.quiet();
        r.stop_at_exit = false;
    }
    let ended = crate::route::run_identical(a, b, PROBE_TICKS)?;
    Ok(ContinueReport {
        at_tick,
        ticks: ended.as_ref().map_or(PROBE_TICKS, |(t, _)| *t),
        ended: ended.map(|(t, message)| format!("tick {t}: {message}")),
        digest,
        ledger_visits: sizes.visits,
        ledger_bytes: sizes.bytes,
    })
}

/// Save and continue at a boundary: the entrance is serialized, a route is rebuilt from the text
/// and another is entered fresh, the chain's own route is checked against the fresh one, and the
/// rebuilt and fresh routes run 600 ticks of identical input.
fn verify_boundary(
    assets: &mut Assets,
    entrance: &Checkpoint,
    live: &Route,
) -> Result<ContinueReport> {
    let saved = round_trip(entrance)?;
    let sizes = within_limits(&saved.ledger, &saved)?;
    let mut rebuilt = Route::resume(assets, &saved)?;
    let level = &entrance.level;
    let mut fresh = Route::enter_carrying(
        assets,
        &level.map,
        level.entry.as_deref(),
        entrance.stats.clone(),
        entrance.ledger.clone(),
        entrance.difficulty,
    )?;
    ensure!(
        live.state()? == fresh.state()?,
        "The chain's route differs from a fresh entry of {}",
        level.key()
    );
    continue_identically(&mut fresh, &mut rebuilt, 0, sizes)
}

/// The leg driven again from its entrance and stopped at `tick`. The leg's drivers are
/// deterministic, so this reaches the same moment the chain's own run passed through.
fn drive_to(
    assets: &mut Assets,
    cfg: &Config,
    index: usize,
    optional: bool,
    entrance: &Checkpoint,
    tick: usize,
) -> Result<Route> {
    let mut live = Route::resume(assets, &round_trip(entrance)?)?;
    live.skip_cinematics = cfg.skip;
    live.stop_at_exit = true;
    live.freeze_at(tick);
    // The drive stops early when the route freezes; only an unfrozen route has something to say.
    let driven = LEGS[index].body(optional)(&mut live);
    ensure!(
        live.frozen() && live.transition.is_none(),
        "Driven again, the leg did not reach tick {tick} before its exit: it ended at tick {} ({})",
        live.ticks,
        driven
            .err()
            .map_or_else(|| "it returned".into(), |e| format!("{e:#}"))
    );
    Ok(live)
}

/// Save and continue in the middle of a leg: drive the leg again from its entrance until the
/// midpoint, take a checkpoint there, rebuild a route from its text, and run both on identical
/// input.
fn verify_mid_leg(
    assets: &mut Assets,
    cfg: &Config,
    index: usize,
    optional: bool,
    entrance: &Checkpoint,
    leg_ticks: usize,
) -> Result<ContinueReport> {
    let mut live = drive_to(
        assets,
        cfg,
        index,
        optional,
        entrance,
        (leg_ticks / 2).clamp(1, 4000),
    )?;
    let at = live.ticks;
    let saved = round_trip(&live.checkpoint())?;
    let sizes = within_limits(&saved.ledger, &saved)?;
    let mut rebuilt = Route::resume(assets, &saved)?;
    live.thaw();
    continue_identically(&mut live, &mut rebuilt, at, sizes)
}

/// The leg driven again to one second before its exit: the state of a manual save made just
/// before leaving. The exit has not been taken yet.
fn pre_exit(
    assets: &mut Assets,
    cfg: &Config,
    index: usize,
    optional: bool,
    entrance: &Checkpoint,
    leg_ticks: usize,
) -> Result<Checkpoint> {
    let live = drive_to(
        assets,
        cfg,
        index,
        optional,
        entrance,
        leg_ticks.saturating_sub(PRE_EXIT_TICKS).max(1),
    )?;
    let mut checkpoint = live.checkpoint();
    checkpoint.ledger = save::Campaign::default();
    Ok(checkpoint)
}

#[derive(Clone, Serialize)]
struct Frontier {
    visit: String,
    spelled: String,
    /// 1-based visit number.
    index: usize,
    reason: &'static str,
    /// What kind of stop: `route` (a leg's driver or assertion failed), `grant` (a required grant
    /// is missing), `arrival` (the next entrance could not be entered), `driver` (the visit has no
    /// driver yet), `target` (the requested leg passed) or `complete`.
    kind: &'static str,
    message: String,
    start_thread: Option<String>,
    start_thread_handled: Option<bool>,
}

/// A required grant a visit did not make: a failure of the chain's provenance, not of a driver.
#[derive(Debug)]
struct GrantFailure(String);
impl std::fmt::Display for GrantFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for GrantFailure {}

#[derive(Serialize)]
struct LegReport {
    index: usize,
    visit: String,
    spelled: String,
    variant: &'static str,
    status: &'static str,
    retried: usize,
    metrics: Option<Metrics>,
    entrance_digest: String,
    exit_digest: Option<String>,
    message: Option<String>,
    boundary: ContinueReport,
    mid_leg: Option<ContinueReport>,
    checkpoint: Option<String>,
}

#[derive(Serialize)]
struct ProvenanceRow {
    visit: String,
    toy: String,
    copies: u8,
    required: bool,
    status: provenance::Status,
    text: String,
}

/// What one run of the chain (or of a variant from a divergence) found.
struct Segment {
    legs: Vec<LegReport>,
    frontier: Frontier,
    statuses: Vec<provenance::Status>,
    /// The rewards' statuses on entering each visit, for the variants that branch there.
    statuses_at: BTreeMap<usize, Vec<provenance::Status>>,
    entrances: BTreeMap<usize, Checkpoint>,
    /// The temple's arrival grant, once the chain entered the temple.
    arrival: Option<bool>,
    /// Pickup and enemy-drop keys in the ledger of collected items at the last entrance.
    keys: (usize, usize),
    retries: usize,
    /// Every visit in the requested range passed.
    reached: bool,
}

/// The pickups and enemy drops the resources remember (DG-15: keyed per map, so a return visit
/// does not respawn what the first visit's pickups gave; drops are keyed per visit).
fn collected_keys(stats: &Stats) -> (usize, usize) {
    let drops = stats
        .collected
        .iter()
        .filter(|k| k.starts_with("drop:"))
        .count();
    (stats.collected.len() - drops, drops)
}

fn start_thread(r: &Route) -> (Option<String>, Option<bool>) {
    let level = r.level();
    let thread = r
        .map
        .entities
        .iter()
        .filter(|e| e.get("classname").is_some_and(|c| c == "info_player_start"))
        .find(|e| {
            level
                .entry
                .as_deref()
                .is_none_or(|entry| e.get("targetname").is_some_and(|n| n == entry))
        })
        .and_then(|e| e.get("thread").cloned());
    // The entry story or scene starts something, or nothing handles the thread.
    let handled = r.interactions.scripted() || r.story.busy();
    (thread.clone(), thread.map(|_| handled))
}

#[allow(clippy::too_many_arguments)]
fn run_segment(
    assets: &mut Assets,
    cfg: &Config,
    catalog: &Catalog,
    table: &[provenance::Reward],
    mut r: Route,
    from: usize,
    last: usize,
    variant: &Variant,
    write: bool,
    mut statuses: Vec<provenance::Status>,
) -> Result<Segment> {
    let mut seg = Segment {
        legs: Vec::new(),
        frontier: Frontier {
            visit: String::new(),
            spelled: String::new(),
            index: 0,
            reason: "",
            kind: "",
            message: String::new(),
            start_thread: None,
            start_thread_handled: None,
        },
        statuses: Vec::new(),
        statuses_at: BTreeMap::new(),
        entrances: BTreeMap::new(),
        arrival: None,
        keys: (0, 0),
        retries: 0,
        reached: false,
    };
    let total = campaign::route().len();
    let mut i = from;
    loop {
        let visit = campaign::visits()[i].clone();
        let spelled_visit = spelled(assets, i)?;
        let entrance = r.checkpoint();
        let entrance_digest = r.state_hash()?;
        seg.keys = collected_keys(&r.stats);
        let boundary = verify_boundary(assets, &entrance, &r).with_context(|| {
            format!(
                "Save and continue at the entrance of visit {:02} {visit}",
                i + 1
            )
        })?;
        println!(
            "PASS save/continue at the entrance of visit {:02} {visit}: {} ticks of identical input, {} visits and {} bytes in the ledger, state {}{}",
            i + 1,
            boundary.ticks,
            boundary.ledger_visits,
            boundary.ledger_bytes,
            &boundary.digest[..12],
            boundary.ended.as_deref().map_or(String::new(), |e| format!(" (both ended: {e})"))
        );
        let file = file_path(i);
        if write {
            write_visit(i, cfg, &spelled_visit, &entrance, &entrance_digest, None)?;
            let label = format!("{}-auto", file_name(i).trim_end_matches(".json"));
            let bytes = save_chain::verify_headless(
                assets,
                &entrance,
                &entrance.ledger,
                save::Slot::Auto,
                &label,
            )
            .with_context(|| format!("The arrival autosave of visit {:02} {visit}", i + 1))?;
            println!(
                "PASS arrival autosave of visit {:02} {visit}: written by the save Store ({bytes} bytes), read back and rebuilt window-free",
                i + 1
            );
        }
        seg.entrances.insert(i, entrance.clone());
        seg.statuses_at.insert(i, statuses.clone());
        if entrance.level.map == "utemple" {
            seg.arrival = Some(provenance::arrival_grant(&r.stats));
            println!(
                "REWARD utemple arrival grant: Mock Turtle shell {}",
                if r.stats.turtle_air {
                    "held (authored, kept in strict mode)"
                } else {
                    "MISSING"
                }
            );
        }
        let frontier_here =
            |reason: &'static str, kind: &'static str, message: String, r: &Route| {
                let (start_thread, start_thread_handled) = start_thread(r);
                Frontier {
                    visit: visit.clone(),
                    spelled: spelled_visit.clone(),
                    index: i + 1,
                    reason,
                    kind,
                    message,
                    start_thread,
                    start_thread_handled,
                }
            };
        if i > last {
            seg.frontier = frontier_here(
                "chain target reached",
                "target",
                format!("visit {} was the last leg requested", last + 1),
                &r,
            );
            seg.reached = true;
            break;
        }
        let Some(leg) = LEGS.get(i) else {
            seg.frontier = frontier_here(
                "no driver",
                "driver",
                format!("no route driver exists for {visit} yet"),
                &r,
            );
            break;
        };
        let optional = variant.optional.contains(&i);
        let started = Instant::now();
        let mut attempt = 0;
        // The state the passing attempt started from (a retry restores Sanity and Will first).
        let mut origin = entrance.clone();
        let result: Result<Metrics> = loop {
            r.skip_cinematics = cfg.skip;
            r.stop_at_exit = true;
            let played = leg
                .run(&mut r, optional)
                .with_context(|| format!("Visit {:02} {visit}", i + 1));
            let settled = played.and_then(|m| {
                let mut failure = None;
                for (k, reward) in table.iter().enumerate().filter(|(_, x)| x.visit == i) {
                    let status = provenance::settle(reward, &r, catalog);
                    println!(
                        "REWARD {visit}: {} x{} {}",
                        reward.name,
                        reward.copies,
                        status.text(cfg.strict)
                    );
                    if let Some(message) = provenance::fails(reward, &status, cfg.strict) {
                        failure.get_or_insert(message);
                    }
                    statuses[k] = status;
                }
                match failure {
                    Some(message) => Err(anyhow::Error::new(GrantFailure(message))
                        .context(format!("Visit {:02} {visit}", i + 1))),
                    None => Ok(m),
                }
            });
            match settled {
                // Refilling Sanity and Will cannot supply a grant the visit did not make.
                Err(e)
                    if cfg.allow_retry
                        && attempt < 2
                        && e.downcast_ref::<GrantFailure>().is_none() =>
                {
                    attempt += 1;
                    seg.retries += 1;
                    println!(
                        "RETRY visit {:02} {visit}, attempt {attempt}: Sanity and Will restored (diagnosis only): {e:#}",
                        i + 1
                    );
                    let mut again = entrance.clone();
                    again.stats.restore();
                    r = Route::resume(assets, &again)?;
                    origin = again;
                }
                other => break other,
            }
        };
        let elapsed = started.elapsed().as_secs();
        match result {
            Ok(m) => {
                let exit_digest = r.state_hash()?;
                println!(
                    "PASS visit {:02} {visit}{}: {m} [{elapsed} s]",
                    i + 1,
                    if optional { " (optional body)" } else { "" }
                );
                let mid = verify_mid_leg(assets, cfg, i, optional, &origin, m.ticks).with_context(
                    || {
                        format!(
                            "Save and continue in the middle of visit {:02} {visit}",
                            i + 1
                        )
                    },
                )?;
                println!(
                    "PASS save/continue in the middle of visit {:02} {visit} at tick {}: {} ticks of identical input, state {}{}",
                    i + 1,
                    mid.at_tick,
                    mid.ticks,
                    &mid.digest[..12],
                    mid.ended.as_deref().map_or(String::new(), |e| format!(" (both ended: {e})"))
                );
                if write {
                    let pre_exit = pre_exit(assets, cfg, i, optional, &origin, m.ticks)
                        .with_context(|| {
                            format!("The pre-exit save of visit {:02} {visit}", i + 1)
                        })?;
                    let label = format!("{}-pre-exit", file_name(i).trim_end_matches(".json"));
                    let bytes = save_chain::verify_headless(
                        assets,
                        &pre_exit,
                        &entrance.ledger,
                        save::Slot::One,
                        &label,
                    )
                    .with_context(|| format!("The pre-exit save of visit {:02} {visit}", i + 1))?;
                    println!(
                        "PASS pre-exit manual save of visit {:02} {visit} at tick {}: written by the save Store ({bytes} bytes), read back and rebuilt window-free",
                        i + 1,
                        pre_exit.route.ticks
                    );
                    write_visit(
                        i,
                        cfg,
                        &spelled_visit,
                        &entrance,
                        &entrance_digest,
                        Some(LegFile {
                            status: "pass".into(),
                            retried: attempt,
                            message: None,
                            metrics: Some(serde_json::to_value(&m)?),
                            exit_digest: Some(exit_digest.clone()),
                            mid_leg: Some(mid.clone()),
                            pre_exit: Some(pre_exit),
                        }),
                    )?;
                }
                seg.legs.push(LegReport {
                    index: i + 1,
                    visit: visit.clone(),
                    spelled: spelled_visit.clone(),
                    variant: variant.name,
                    status: "pass",
                    retried: attempt,
                    metrics: Some(m),
                    entrance_digest,
                    exit_digest: Some(exit_digest),
                    message: None,
                    boundary,
                    mid_leg: Some(mid),
                    checkpoint: write.then(|| file.display().to_string()),
                });
            }
            Err(e) => {
                let message = format!("{e:#}");
                println!("FAIL {message}");
                if write {
                    write_visit(
                        i,
                        cfg,
                        &spelled_visit,
                        &entrance,
                        &entrance_digest,
                        Some(LegFile {
                            status: "fail".into(),
                            retried: attempt,
                            message: Some(message.clone()),
                            metrics: None,
                            exit_digest: None,
                            mid_leg: None,
                            pre_exit: None,
                        }),
                    )?;
                }
                seg.legs.push(LegReport {
                    index: i + 1,
                    visit: visit.clone(),
                    spelled: spelled_visit.clone(),
                    variant: variant.name,
                    status: "fail",
                    retried: attempt,
                    metrics: None,
                    entrance_digest,
                    exit_digest: None,
                    message: Some(message.clone()),
                    boundary,
                    mid_leg: None,
                    checkpoint: write.then(|| file.display().to_string()),
                });
                let kind = if e.downcast_ref::<GrantFailure>().is_some() {
                    "grant"
                } else {
                    "route"
                };
                seg.frontier = Frontier {
                    visit: visit.clone(),
                    spelled: spelled_visit.clone(),
                    index: i + 1,
                    reason: "leg failed",
                    kind,
                    message,
                    start_thread: None,
                    start_thread_handled: None,
                };
                break;
            }
        }
        if i + 1 == total {
            seg.frontier = Frontier {
                visit,
                spelled: spelled_visit,
                index: total,
                reason: "campaign complete",
                kind: "complete",
                message: "the last visit's leg passed".into(),
                start_thread: None,
                start_thread_handled: None,
            };
            seg.reached = true;
            break;
        }
        let before = r.stats.clone();
        match r.depart(assets, cfg.strict) {
            Ok(next) => {
                // A strict exit grants nothing but the level's own arrival grants; the baseline
                // fill of a viewer-style exit is reported.
                let gained: Vec<&str> = (0..10)
                    .filter(|&s| next.stats.copies(s) > before.copies(s))
                    .map(|s| crate::inventory::WEAPONS[s].1)
                    .collect();
                if cfg.strict {
                    ensure!(
                        gained.is_empty(),
                        "A strict exit filled in {gained:?} from the baseline loadout"
                    );
                } else if !gained.is_empty() {
                    println!(
                        "BASELINE FILL entering visit {:02}: {}",
                        i + 2,
                        gained.join(", ")
                    );
                }
                r = next;
            }
            Err(e) => {
                let message = format!("{e:#}");
                println!("FAIL entering visit {:02} from {visit}: {message}", i + 2);
                let next = i + 1;
                seg.frontier = Frontier {
                    visit: campaign::visits()[next].clone(),
                    spelled: spelled(assets, next)?,
                    index: next + 1,
                    reason: "arrival failed",
                    kind: "arrival",
                    message,
                    start_thread: None,
                    start_thread_handled: None,
                };
                break;
            }
        }
        i += 1;
    }
    seg.statuses = statuses;
    Ok(seg)
}

fn write_visit(
    index: usize,
    cfg: &Config,
    spelled: &str,
    entrance: &Checkpoint,
    digest: &str,
    leg: Option<LegFile>,
) -> Result<()> {
    fs::create_dir_all(DIR)?;
    let file = VisitFile {
        format: VISIT_FORMAT.into(),
        version: 1,
        index: index + 1,
        visit: campaign::visits()[index].clone(),
        spelled: spelled.into(),
        strict: cfg.strict,
        skip_cinematics: cfg.skip,
        fixture: cfg.fixture,
        diagnosis: cfg.allow_retry,
        entrance: entrance.clone(),
        entrance_digest: digest.into(),
        leg,
    };
    let bytes = serde_json::to_vec(&file)?;
    ensure!(
        bytes.len() as u64 <= save::LIMIT,
        "The checkpoint of visit {} exceeds the save size limit",
        index + 1
    );
    let path = file_path(index);
    fs::write(&path, bytes).with_context(|| format!("Writing {}", path.display()))
}

fn read_visit(index: usize) -> Result<Option<VisitFile>> {
    let path = file_path(index);
    if !path.is_file() {
        return Ok(None);
    }
    let file: VisitFile = serde_json::from_slice(&fs::read(&path)?)
        .with_context(|| format!("Reading {}", path.display()))?;
    ensure!(
        file.format == VISIT_FORMAT
            && file.index == index + 1
            && file.visit == campaign::visits()[index],
        "{} is not the checkpoint of visit {:02}",
        path.display(),
        index + 1
    );
    Ok(Some(file))
}

/// Whether this run may start from a visit's checkpoint file. A checkpoint is proof only for the
/// run that would have produced it: the same difficulty, and, for a strict chain, one that was
/// strict, came from New Game state and restored nothing.
fn may_resume(
    file: &VisitFile,
    difficulty: Difficulty,
    strict: bool,
    allow_retry: bool,
) -> Result<()> {
    let n = file.index;
    ensure!(
        file.entrance.difficulty == difficulty,
        "The checkpoint of visit {n:02} was written at {}, this run is {}",
        file.entrance.difficulty.name(),
        difficulty.name()
    );
    ensure!(
        file.strict || !strict,
        "The checkpoint of visit {n:02} came from a chain that was not strict"
    );
    ensure!(
        !file.diagnosis || allow_retry,
        "The checkpoint of visit {n:02} came from a --campaign-allow-retry run, which restores Sanity and Will: it is diagnosis, never proof"
    );
    ensure!(
        !file.fixture || !strict,
        "The checkpoint of visit {n:02} came from a staged chapter loadout, not from New Game state"
    );
    Ok(())
}

/// The route the chain starts from: New Game state, a checkpoint, or (outside a strict chain) the
/// visit's chapter loadout, labelled as the staged fixture it is.
fn start_route(
    assets: &mut Assets,
    from: usize,
    difficulty: Difficulty,
    strict: bool,
    allow_retry: bool,
) -> Result<(Route, &'static str, bool)> {
    let (map, entry) = campaign::route()[from];
    if from == 0 {
        let stats = Stats::for_level(map, entry);
        return Ok((
            Route::enter(assets, map, entry, stats, difficulty)?,
            "New Game state",
            false,
        ));
    }
    if let Some(file) = read_visit(from)? {
        may_resume(&file, difficulty, strict, allow_retry)?;
        let route = Route::resume(assets, &file.entrance)?;
        ensure!(
            route.state_hash()? == file.entrance_digest,
            "The checkpoint of visit {:02} does not rebuild to the state it was written from",
            from + 1
        );
        return Ok((route, "checkpoint", file.fixture));
    }
    if strict {
        return Err(NoCheckpoint(from + 1).into());
    }
    let stats = Stats::for_level(map, entry);
    Ok((
        Route::enter(assets, map, entry, stats, difficulty)?,
        "staged chapter loadout (a fixture, not New Game state)",
        true,
    ))
}

#[derive(Serialize)]
struct VariantReport {
    name: &'static str,
    status: &'static str,
    legs: Vec<LegReport>,
    frontier: Option<Frontier>,
    note: String,
}

#[derive(Serialize)]
struct Report {
    format: &'static str,
    version: u32,
    difficulty: &'static str,
    strict: bool,
    skip_cinematics: bool,
    allow_retry: bool,
    start: &'static str,
    from: usize,
    to: Option<usize>,
    log: Option<String>,
    legs: Vec<LegReport>,
    frontier: Frontier,
    provenance: Vec<ProvenanceRow>,
    utemple_arrival_grant: Option<bool>,
    /// Pickup and drop keys the resources remember at the last entrance (the ledger is per map).
    pickup_ledger_keys: (usize, usize),
    variants: Vec<VariantReport>,
    retries: usize,
    /// `reached`, `frontier`, or `diagnosis` for a retry run (which is never a pass).
    result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    expectation: Option<ExpectationReport>,
}

/// What `--campaign-expect` pinned and whether the run held it.
#[derive(Serialize)]
struct ExpectationReport {
    expected: String,
    met: bool,
}

/// The report of a run that refused to start: written so that the report of an earlier run never
/// stands in for it.
#[derive(Serialize)]
struct RefusalReport {
    format: &'static str,
    version: u32,
    difficulty: &'static str,
    strict: bool,
    allow_retry: bool,
    from: usize,
    to: Option<usize>,
    log: Option<String>,
    message: String,
    result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    expectation: Option<ExpectationReport>,
}

/// A run that could not start: report it, and end it as `--campaign-expect no-checkpoint` says.
fn refused(
    args: &Args,
    difficulty: Difficulty,
    from: usize,
    to: Option<usize>,
    expect: Option<&Expect>,
    error: anyhow::Error,
) -> Result<()> {
    let met =
        expect == Some(&Expect::NoCheckpoint) && error.downcast_ref::<NoCheckpoint>().is_some();
    let report = RefusalReport {
        format: "campaign-chain-report",
        version: 1,
        difficulty: difficulty.name(),
        strict: args.strict,
        allow_retry: args.allow_retry,
        from: from + 1,
        to: to.map(|t| t + 1),
        log: args.log.clone(),
        message: format!("{error:#}"),
        result: "refused",
        expectation: expect.map(|x| ExpectationReport {
            expected: x.text(),
            met,
        }),
    };
    fs::create_dir_all(DIR)?;
    let path = Path::new(DIR).join("report.json");
    fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
    println!("Report: {} (the run refused to start)", path.display());
    if met {
        println!("REFUSED (expected, --campaign-expect no-checkpoint): {error:#}");
        return Ok(());
    }
    Err(error)
}

fn provenance_rows(
    table: &[provenance::Reward],
    statuses: &[provenance::Status],
    strict: bool,
) -> Vec<ProvenanceRow> {
    table
        .iter()
        .zip(statuses)
        .map(|(reward, status)| ProvenanceRow {
            visit: campaign::visits()[reward.visit].clone(),
            toy: reward.name.into(),
            copies: reward.copies,
            required: reward.required,
            status: status.clone(),
            text: status.text(strict),
        })
        .collect()
}

/// `--campaign-route-check`.
pub fn check(assets: &mut Assets, difficulty: Difficulty, args: &Args) -> Result<()> {
    let total = campaign::route().len();
    let expect = args.expect.as_deref().map(Expect::parse).transpose()?;
    let from = args
        .from
        .as_deref()
        .map(parse_visit)
        .transpose()?
        .unwrap_or(0);
    let to = args.to.as_deref().map(parse_visit).transpose()?;
    ensure!(
        to.is_none_or(|t| t >= from),
        "--campaign-to comes before --campaign-from"
    );
    let last = to.unwrap_or(total - 1);
    let (start, start_kind, fixture) = match start_route(
        assets,
        from,
        difficulty,
        args.strict,
        args.allow_retry,
    ) {
        Ok(started) => {
            ensure!(
                    expect != Some(Expect::NoCheckpoint),
                    "Expected the chain to refuse to start for want of a checkpoint, but it started from {}",
                    started.1
                );
            started
        }
        Err(e) => return refused(args, difficulty, from, to, expect.as_ref(), e),
    };
    let cfg = Config {
        difficulty,
        strict: args.strict,
        skip: args.skip_cinematics,
        allow_retry: args.allow_retry,
        fixture,
    };
    println!(
        "Campaign chain at {}{}{}{}: visits {:02} to {} from {start_kind}",
        cfg.difficulty.name(),
        if cfg.strict { ", strict" } else { "" },
        if cfg.skip { ", cinematics skipped" } else { "" },
        if cfg.allow_retry {
            ", retry allowed (diagnosis only)"
        } else {
            ""
        },
        from + 1,
        to.map_or_else(|| "the frontier".to_owned(), |t| format!("{:02}", t + 1)),
    );
    let catalog = Catalog::load(assets)?;
    let table = provenance::table();
    let fresh: Vec<_> = table
        .iter()
        .map(|reward| {
            if reward.visit < from {
                provenance::Status::Unproven
            } else {
                provenance::Status::NotReached
            }
        })
        .collect();
    let base = run_segment(
        assets,
        &cfg,
        &catalog,
        &table,
        start,
        from,
        last,
        &VARIANTS[0],
        true,
        fresh,
    )?;
    // The variants that branch from a leg the base run entered.
    let mut variants = Vec::new();
    for variant in &VARIANTS[1..] {
        let Some(&branch) = variant.optional.iter().min() else {
            continue;
        };
        let entered = base.entrances.get(&branch);
        let passed = base
            .legs
            .iter()
            .any(|l| l.index == branch + 1 && l.status == "pass");
        let (Some(entrance), true) = (entered, passed && branch <= last) else {
            let note = format!(
                "not run: the base chain did not pass visit {:02} ({})",
                branch + 1,
                campaign::visits()[branch]
            );
            println!("VARIANT {}: {note}", variant.name);
            variants.push(VariantReport {
                name: variant.name,
                status: "not run",
                legs: Vec::new(),
                frontier: None,
                note,
            });
            continue;
        };
        println!(
            "VARIANT {}: from the entrance of visit {:02}",
            variant.name,
            branch + 1
        );
        let route = Route::resume(assets, entrance)?;
        // The variant ends where its optional legs end: one leg past the last optional one.
        let end = variant
            .optional
            .iter()
            .copied()
            .max()
            .unwrap_or(branch)
            .min(last);
        let statuses = base
            .statuses_at
            .get(&branch)
            .cloned()
            .unwrap_or_else(|| vec![provenance::Status::NotReached; table.len()]);
        let seg = run_segment(
            assets, &cfg, &catalog, &table, route, branch, end, variant, false, statuses,
        )?;
        let ok = seg.reached;
        println!(
            "VARIANT {}: {}",
            variant.name,
            if ok {
                "PASS"
            } else {
                "stopped, see the FAIL line above"
            }
        );
        variants.push(VariantReport {
            name: variant.name,
            status: if ok { "pass" } else { "fail" },
            note: format!("legs {:02} to {:02}", branch + 1, end + 1),
            frontier: Some(seg.frontier),
            legs: seg.legs,
        });
    }
    // The reward-provenance report: every milestone, whether or not the chain reached it.
    println!(
        "REWARD PROVENANCE ({}, {})",
        if cfg.strict {
            "strict: no baseline fill"
        } else {
            "viewer-style: baseline fill on"
        },
        cfg.difficulty.name()
    );
    let checkpoints = base.entrances.len();
    let rows = provenance_rows(&table, &base.statuses, cfg.strict);
    for row in &rows {
        println!(
            "  {:<24} {:<22} x{} {}{}",
            row.visit,
            row.toy,
            row.copies,
            row.text,
            if row.required { "" } else { " [optional]" }
        );
    }
    println!(
        "  utemple arrival grant    Mock Turtle shell       {}",
        match base.arrival {
            Some(true) => "held on entering the temple (authored, kept in strict mode)",
            Some(false) => "MISSING",
            None => "not reached",
        }
    );
    println!(
        "  pickup ledger (DG-15, keyed per map): {} pickup keys and {} drop keys at the last entrance",
        base.keys.0, base.keys.1
    );
    let f = &base.frontier;
    println!(
        "FRONTIER {} ({}: {}){}",
        f.spelled,
        f.reason,
        f.message,
        f.start_thread.as_ref().map_or(String::new(), |t| format!(
            "; start thread {t} {}",
            if f.start_thread_handled == Some(true) {
                "starts a scene or dialogue"
            } else {
                "is unhandled: nothing starts on entry"
            }
        ))
    );
    let ending = gate(cfg.allow_retry, to, base.reached, f.index, expect.as_ref());
    let result = if cfg.allow_retry {
        "diagnosis"
    } else if base.reached {
        "reached"
    } else {
        "frontier"
    };
    let report = Report {
        format: "campaign-chain-report",
        version: 1,
        difficulty: cfg.difficulty.name(),
        strict: cfg.strict,
        skip_cinematics: cfg.skip,
        allow_retry: cfg.allow_retry,
        start: start_kind,
        from: from + 1,
        to: to.map(|t| t + 1),
        log: args.log.clone(),
        legs: base.legs,
        frontier: f.clone(),
        provenance: rows,
        utemple_arrival_grant: base.arrival,
        pickup_ledger_keys: base.keys,
        variants,
        retries: base.retries,
        result,
        expectation: expect.as_ref().map(|x| ExpectationReport {
            expected: x.text(),
            met: ending == Gate::Expected,
        }),
    };
    fs::create_dir_all(DIR)?;
    let path = Path::new(DIR).join("report.json");
    fs::write(&path, serde_json::to_vec_pretty(&report)?)?;
    let passed = report.legs.iter().filter(|l| l.status == "pass").count();
    println!(
        "Report: {} ({passed} legs passed, {} visit checkpoints in {DIR})",
        path.display(),
        checkpoints
    );
    match ending {
        Gate::Expected => println!(
            "EXPECTED FRONTIER {} confirmed{}: the pinned failure holds, and this process exits 0 only because it is pinned (the chain still cannot complete its range)",
            f.spelled,
            if cfg.allow_retry {
                "; --campaign-allow-retry is diagnosis only, so the report says diagnosis"
            } else {
                ""
            }
        ),
        Gate::Missed => bail!(
            "Expected {}, but {}",
            expect.as_ref().map_or_else(String::new, Expect::text),
            if base.reached {
                "the chain completed its range".to_owned()
            } else {
                format!("the chain stopped at FRONTIER {} ({})", f.spelled, f.message)
            }
        ),
        Gate::Diagnosis => bail!(
            "--campaign-allow-retry is diagnosis only: {} retries restored Sanity and Will, so this run is reported as failing{}",
            base.retries,
            if base.reached {
                ""
            } else {
                " and the chain did not complete"
            }
        ),
        Gate::Fail => bail!(
            "The chain did not reach visit {:02} {}: FRONTIER {} ({})",
            to.map_or(0, |t| t + 1),
            to.map_or("", |t| campaign::visits()[t].as_str()),
            f.spelled,
            f.message
        ),
        Gate::Pass => println!(
            "PASS campaign chain reached visit {:02} {}",
            to.map_or(0, |t| t + 1),
            to.map_or("", |t| campaign::visits()[t].as_str())
        ),
        Gate::Complete => println!("PASS campaign chain completed"),
        Gate::Report => println!(
            "REPORT campaign chain stopped at the frontier; exit 0 because no --campaign-to gates this run"
        ),
    }
    Ok(())
}

/// How a run ends. Only `--campaign-to` gates a chain (13d asks for the last visit it wants);
/// without it the chain runs as far as it can and reports the frontier, like the legs check
/// reports its chained runs. A retry run is diagnosis only and always fails. A pinned frontier
/// (`--campaign-expect frontier=<visit>`) replaces all of that with one question: is the chain's
/// failure the one that was pinned.
#[derive(Debug, PartialEq, Eq)]
enum Gate {
    /// The requested last leg passed.
    Pass,
    /// The requested last leg was not reached.
    Fail,
    /// No target: the whole campaign passed.
    Complete,
    /// No target: the chain stopped at its frontier, which is reported and does not fail the run.
    Report,
    /// Retries restored Sanity and Will: reported as failing whatever happened.
    Diagnosis,
    /// The chain stopped short of its range at exactly the pinned frontier.
    Expected,
    /// A frontier was pinned and the chain did something else: it went further, stopped earlier
    /// or completed its range.
    Missed,
}
/// `frontier` is the 1-based number of the visit the chain stopped at.
fn gate(
    allow_retry: bool,
    to: Option<usize>,
    reached: bool,
    frontier: usize,
    expect: Option<&Expect>,
) -> Gate {
    match expect {
        Some(Expect::Frontier(pinned)) if !reached && frontier == pinned + 1 => Gate::Expected,
        Some(_) => Gate::Missed,
        None => match (allow_retry, to, reached) {
            (true, _, _) => Gate::Diagnosis,
            (false, Some(_), true) => Gate::Pass,
            (false, Some(_), false) => Gate::Fail,
            (false, None, true) => Gate::Complete,
            (false, None, false) => Gate::Report,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visits_parse_as_numbers_maps_entrances_and_keys() {
        assert_eq!(parse_visit("1").unwrap(), 0);
        assert_eq!(parse_visit("39").unwrap(), 38);
        assert!(parse_visit("0").is_err() && parse_visit("40").is_err());
        assert_eq!(parse_visit("gvillage").unwrap(), 0);
        assert_eq!(parse_visit("fortress1").unwrap(), 2);
        assert_eq!(parse_visit("fortress1$fortress1_start1").unwrap(), 2);
        assert_eq!(parse_visit("fortress1$fortress1_start2").unwrap(), 4);
        assert_eq!(parse_visit("skool1$return").unwrap(), 7);
        assert_eq!(parse_visit("skool1$first").unwrap(), 5);
        assert_eq!(parse_visit("wforest$wforest_start2").unwrap(), 27);
        assert!(parse_visit("garden1$return").is_err());
        assert!(parse_visit("nowhere").is_err());
    }
    #[test]
    fn a_checkpoint_resumes_only_the_run_it_is_proof_for() {
        let file = |strict, fixture, diagnosis| VisitFile {
            format: VISIT_FORMAT.into(),
            version: 1,
            index: 4,
            visit: "fortress2$first".into(),
            spelled: "fortress2$fortress2_start1".into(),
            strict,
            skip_cinematics: false,
            fixture,
            diagnosis,
            entrance: synthetic(),
            entrance_digest: String::new(),
            leg: None,
        };
        let normal = Difficulty::Normal;
        // A strict New Game checkpoint resumes a strict or a viewer-style chain.
        assert!(may_resume(&file(true, false, false), normal, true, false).is_ok());
        assert!(may_resume(&file(true, false, false), normal, false, false).is_ok());
        // Another difficulty, a viewer-style checkpoint, a staged loadout and a retry run are not
        // proof for a strict chain.
        assert!(may_resume(&file(true, false, false), Difficulty::Easy, true, false).is_err());
        assert!(may_resume(&file(false, false, false), normal, true, false).is_err());
        assert!(may_resume(&file(true, true, false), normal, true, false).is_err());
        assert!(may_resume(&file(true, false, true), normal, true, false).is_err());
        // A viewer-style chain may start from a staged loadout, and a retry run from a retry run.
        assert!(may_resume(&file(false, true, false), normal, false, false).is_ok());
        assert!(may_resume(&file(true, false, true), normal, true, true).is_ok());
        assert!(may_resume(&file(true, false, true), normal, false, false).is_err());
    }
    #[test]
    fn only_a_target_gates_a_chain_and_a_retry_run_always_fails() {
        assert_eq!(gate(false, None, false, 4, None), Gate::Report);
        assert_eq!(gate(false, None, true, 4, None), Gate::Complete);
        assert_eq!(gate(false, Some(3), true, 4, None), Gate::Pass);
        assert_eq!(gate(false, Some(3), false, 4, None), Gate::Fail);
        for to in [None, Some(3)] {
            for reached in [false, true] {
                assert_eq!(gate(true, to, reached, 4, None), Gate::Diagnosis);
            }
        }
    }
    #[test]
    fn expectations_parse_as_a_pinned_frontier_or_a_refusal() {
        assert_eq!(
            Expect::parse("frontier=fortress2$fortress2_start1").unwrap(),
            Expect::Frontier(3)
        );
        assert_eq!(Expect::parse("frontier=4").unwrap(), Expect::Frontier(3));
        assert_eq!(
            Expect::parse("frontier=skool1").unwrap(),
            Expect::Frontier(5)
        );
        assert_eq!(
            Expect::parse(" no-checkpoint ").unwrap(),
            Expect::NoCheckpoint
        );
        assert_eq!(Expect::Frontier(3).text(), "frontier=fortress2$first");
        for bad in [
            "",
            "frontier",
            "frontier=",
            "frontier=40",
            "frontier=nowhere",
            "pass",
        ] {
            assert!(Expect::parse(bad).is_err(), "{bad:?}");
        }
    }
    #[test]
    fn a_pinned_frontier_passes_only_while_the_chain_stops_exactly_there() {
        let pin = Expect::Frontier(3);
        // The chain stops at visit 4: with or without a target, with or without retries.
        for to in [None, Some(3)] {
            for retry in [false, true] {
                assert_eq!(gate(retry, to, false, 4, Some(&pin)), Gate::Expected);
            }
        }
        // It went further, stopped earlier, or completed its range: the pin no longer holds.
        for frontier in [1, 3, 5, 6, 39] {
            assert_eq!(gate(false, None, false, frontier, Some(&pin)), Gate::Missed);
        }
        assert_eq!(gate(false, Some(3), true, 4, Some(&pin)), Gate::Missed);
        assert_eq!(gate(true, None, true, 4, Some(&pin)), Gate::Missed);
        // A refusal pin says nothing about a chain that started.
        assert_eq!(
            gate(false, None, false, 4, Some(&Expect::NoCheckpoint)),
            Gate::Missed
        );
    }
    #[test]
    fn a_refusal_is_pinned_only_when_it_is_the_missing_checkpoint() {
        let missing: anyhow::Error = NoCheckpoint(6).into();
        assert!(missing.downcast_ref::<NoCheckpoint>().is_some());
        assert!(missing.to_string().contains("visit 06"));
        let other =
            anyhow::anyhow!("The checkpoint of visit 06 came from a chain that was not strict");
        assert!(other.downcast_ref::<NoCheckpoint>().is_none());
    }
    #[test]
    fn checkpoint_files_are_named_by_visit_number_map_and_kind() {
        assert_eq!(file_name(0), "01-gvillage-first.json");
        assert_eq!(file_name(3), "04-fortress2-first.json");
        assert_eq!(file_name(4), "05-fortress1-return.json");
        assert_eq!(file_name(38), "39-qlair-first.json");
        let names: std::collections::BTreeSet<_> = (0..39).map(file_name).collect();
        assert_eq!(names.len(), 39);
    }
    #[test]
    fn variants_name_legs_that_have_an_optional_body() {
        assert_eq!(VARIANTS[0].name, "base");
        assert!(VARIANTS[0].optional.is_empty());
        for variant in &VARIANTS[1..] {
            for &leg in variant.optional {
                assert!(
                    LEGS.get(leg).is_some_and(|l| l.optional.is_some()),
                    "{} names leg {leg}",
                    variant.name
                );
            }
        }
    }
    /// A checkpoint of a synthetic visit: no game data, no window.
    fn synthetic() -> Checkpoint {
        use serde_json::json;
        let level: save::Level = serde_json::from_value(json!({
            "map": "skool1",
            "entry": null,
            "interactions": serde_json::to_value(crate::interaction::Interactions::empty().snapshot()).unwrap(),
            "npcs": { "actors": [], "greetings": 0 },
            "story": { "seen": [], "queue": [], "current": null, "exit": null, "completed": 0, "finished": [] },
            "hints": { "selected": null, "next": 0, "cooldown": 0., "appearance": null },
            "environment_clock": 3.25,
            "pickup_clock": 2.625
        }))
        .unwrap();
        serde_json::from_value(json!({
            "format": "campaign-chain-checkpoint",
            "version": 1,
            "difficulty": "Normal",
            "stats": serde_json::to_value(Stats::for_level("skool1", None)).unwrap(),
            "ledger": { "levels": {}, "completed": [] },
            "player": serde_json::to_value(crate::movement::Player::new(macroquad::prelude::Vec3::ZERO)).unwrap(),
            "level": serde_json::to_value(&level).unwrap(),
            "route": {
                "ticks": 7, "jumps": 1, "shots": 0, "swings": 0, "cards": 0, "damage": 0.0, "teleports": 0,
                "guards": [],
                "actions": { "playing": null, "selected": 0, "pose": [], "start": [], "variation": 0 },
                "projectiles": []
            }
        }))
        .unwrap()
    }
    #[test]
    fn a_checkpoint_survives_its_own_text() {
        let cp = synthetic();
        let back = round_trip(&cp).unwrap();
        assert_eq!(
            serde_json::to_value(&cp).unwrap(),
            serde_json::to_value(&back).unwrap()
        );
        assert_eq!(back.route.ticks, 7);
        assert_eq!(back.level.key(), "skool1$first");
    }
    #[test]
    fn a_ledger_of_all_visits_stays_inside_the_save_limits() {
        let mut ledger = save::Campaign::default();
        let cp = synthetic();
        for key in campaign::visits() {
            let mut level = cp.level.clone();
            level.map = key.split('$').next().unwrap().into();
            ledger.levels.insert(key.clone(), level);
            ledger.completed.insert(key.clone());
        }
        // Thirty-nine visits are far inside the 72-visit limit.
        let sizes = within_limits(&ledger, &cp).unwrap();
        assert_eq!(sizes.visits, 39);
        for i in 0..40 {
            let mut level = cp.level.clone();
            level.map = format!("extra{i}");
            ledger.levels.insert(format!("extra{i}$first"), level);
        }
        assert!(
            within_limits(&ledger, &cp).is_err(),
            "79 cached visits break the limit"
        );
    }
    #[test]
    fn an_array_of_more_than_10000_elements_breaks_the_limit() {
        let mut cp = synthetic();
        cp.route.projectiles = Vec::new();
        let ledger = save::Campaign::default();
        assert!(within_limits(&ledger, &cp).is_ok());
        // The save's own validator rejects the oversized array inside a checkpoint's value.
        let mut value = serde_json::to_value(&cp).unwrap();
        value["stats"]["collected"] = (0..10_001)
            .map(|i| format!("p:{i}"))
            .collect::<Vec<_>>()
            .into();
        assert!(save::validate_json(&value).is_err());
    }
}

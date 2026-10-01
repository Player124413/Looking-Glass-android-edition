//! Headless `--registry-check` (F1). It fingerprints a fresh visit of every campaign entry, so a
//! refactor that must not change behaviour can be compared against a recorded baseline.
//!
//! For each of the 39 visits (the 36 maps' first entries plus the fortress1, skool1 and wforest
//! returns) it builds the state the viewer builds on entering a level (`Interactions::load`,
//! `set_entry`, the entry story), serializes `Interactions::snapshot()` and prints the SHA-256 of
//! those bytes. The snapshot carries the event signature, which hashes every entity definition,
//! rule, condition and fact, so a changed rule or gate cannot hide.
//!
//! * `--registry-record <file>` writes the hashes (the F1.0 baseline, `private/registry-baseline`).
//! * `--registry-baseline <file>` compares against a recorded file and fails on any difference.
//! * With neither, the default file is compared when it exists.
//!
//! Two more parts run every time. The registration table is held to Appendix F
//! (`levels::reservations::validate`), and a probe controller that answers every hook
//! aggressively is injected into each legacy-owned visit to prove on real data that legacy
//! answers come first: the event program of every one of those visits is unchanged by it, and a
//! real scene skip is taken by the legacy controller before the registry is asked.
//!
//! The check never touches saves, settings or a window.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::World,
    event::Condition,
    interaction::{self, Interactions},
    level::{LevelController, Slot, TriggerInfo},
    levels::{self, Registration},
    movement::Player,
    save,
    story::Story,
};
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

/// Compared when no `--registry-baseline` is given and the file exists.
pub const DEFAULT_BASELINE: &str = "private/registry-baseline/hashes.json";

#[derive(Default)]
pub struct Args {
    pub baseline: Option<PathBuf>,
    pub record: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisitHash {
    pub map: String,
    pub entry: Option<String>,
    /// SHA-256 of the serialized `Interactions::snapshot()`.
    pub sha256: String,
    /// The event signature inside that snapshot.
    pub signature: String,
    pub bytes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Baseline {
    pub version: u32,
    pub visits: BTreeMap<String, VisitHash>,
}

fn fresh(assets: &mut Assets, name: &str, entry: Option<&str>) -> Result<VisitHash> {
    let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
    let mut interactions = Interactions::load(&map)?;
    interactions.set_entry(assets, &map, name, entry)?;
    // The viewer plays the entry story on a fresh visit; some controllers begin a scene there.
    let mut story = Story::load(assets, name);
    interactions.entry_story(&mut story);
    let bytes = serde_json::to_vec(&interactions.snapshot())?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let signature = value["shared"]["signature"]
        .as_str()
        .with_context(|| format!("{name}: the snapshot has no event signature"))?
        .to_owned();
    Ok(VisitHash {
        map: name.into(),
        entry: entry.map(str::to_owned),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
        signature,
        bytes: bytes.len(),
    })
}

/// Every campaign visit in story order, keyed by its save identity.
pub fn capture(assets: &mut Assets) -> Result<BTreeMap<String, VisitHash>> {
    let maps = assets.maps();
    let mut visits = BTreeMap::new();
    for &(name, entry) in crate::campaign::route() {
        ensure!(
            maps.iter().any(|m| m == name),
            "Missing campaign level {name}"
        );
        let key = save::visit_key(name, entry);
        let first = fresh(assets, name, entry)?;
        // A second, independent build must be identical: unordered maps or clocks would
        // otherwise make every later comparison meaningless.
        let second = fresh(assets, name, entry)?;
        ensure!(first == second, "{key}: two fresh builds differ");
        println!(
            "HASH {key} sha256={} signature={} bytes={}",
            first.sha256, first.signature, first.bytes
        );
        ensure!(
            visits.insert(key.clone(), first).is_none(),
            "{key} appears twice in the campaign route"
        );
    }
    Ok(visits)
}

fn read(path: &Path) -> Result<Baseline> {
    let baseline: Baseline = serde_json::from_slice(
        &fs::read(path).with_context(|| format!("Cannot read baseline {}", path.display()))?,
    )
    .with_context(|| format!("Invalid baseline {}", path.display()))?;
    ensure!(baseline.version == 1, "Unsupported baseline version");
    Ok(baseline)
}

fn compare(now: &BTreeMap<String, VisitHash>, baseline: &Baseline) -> Result<()> {
    ensure!(
        now.keys().eq(baseline.visits.keys()),
        "The baseline covers different visits than the campaign route"
    );
    let mut failed = Vec::new();
    for (key, hash) in now {
        let old = &baseline.visits[key];
        // A visit a registered controller now serves legitimately differs from its baseline.
        if levels::serving(&hash.map, hash.entry.as_deref())
            .next()
            .is_some()
        {
            println!("REGISTERED {key}: served by a level controller, not compared");
            continue;
        }
        if hash.sha256 != old.sha256 {
            failed.push(format!(
                "{key}: {}",
                if hash.signature != old.signature {
                    "event signature and snapshot changed"
                } else {
                    "snapshot changed, event signature unchanged"
                }
            ));
        }
    }
    ensure!(
        failed.is_empty(),
        "Fresh-visit snapshots differ from the baseline:\n  {}",
        failed.join("\n  ")
    );
    println!(
        "PASS {} fresh-visit snapshots (with event signatures) reproduce the baseline byte for byte",
        now.len()
    );
    Ok(())
}

pub fn check(assets: &mut Assets, args: &Args) -> Result<()> {
    ensure!(
        args.baseline.is_none() || args.record.is_none(),
        "--registry-baseline and --registry-record cannot be combined"
    );
    let now = capture(assets)?;
    levels::reservations::validate(levels::LEVELS)?;
    println!(
        "PASS registration table: {} registrations keep to their Appendix F reservations",
        levels::LEVELS.len()
    );
    precedence(assets)?;
    if let Some(path) = &args.record {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }
        let baseline = Baseline {
            version: 1,
            visits: now,
        };
        fs::write(path, serde_json::to_vec_pretty(&baseline)?)?;
        println!(
            "RECORDED {} visit hashes in {}",
            baseline.visits.len(),
            path.display()
        );
        return Ok(());
    }
    match &args.baseline {
        Some(path) => compare(&now, &read(path)?)?,
        None if Path::new(DEFAULT_BASELINE).is_file() => {
            compare(&now, &read(Path::new(DEFAULT_BASELINE))?)?
        }
        None => println!(
            "NOTE no baseline at {DEFAULT_BASELINE}: only build determinism was checked (pass --registry-baseline <file> to compare)"
        ),
    }
    println!(
        "PASS registry check: {} visits, each built twice with identical snapshots",
        now.len()
    );
    Ok(())
}

static SKIPS: AtomicUsize = AtomicUsize::new(0);

/// A controller that answers every hook it can. If any of its answers were consulted before a
/// legacy controller's, the event program or the scene skip of a legacy visit would change.
struct Probe;
impl LevelController for Probe {
    fn id(&self) -> &'static str {
        "garden1"
    }
    fn gate(&self, _: &TriggerInfo<'_>) -> Option<Condition> {
        Some(Condition::Always.not())
    }
    fn skip(&mut self, _: &Bsp, _: &mut World, _: &mut Player, _: &mut Story) -> Result<bool> {
        SKIPS.fetch_add(1, Ordering::SeqCst);
        Ok(true)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
fn never(_: &mut Assets, _: &Bsp, _: &str, _: Option<&str>) -> Result<Box<dyn LevelController>> {
    unreachable!("the probe is injected, never loaded")
}
/// Never in `LEVELS`: `applies` is false for every visit.
static PROBE: Registration = Registration {
    id: "garden1",
    applies: |_, _| false,
    load: never,
    art: None,
    owns_submodel: |_, _| false,
    owns_npc: |_, _| false,
    target_base: None,
    story_beats: &[],
    checks: &[],
    save_cases: &[],
    visibility: &[],
};
fn probe() -> Vec<Slot> {
    vec![Slot {
        reg: &PROBE,
        ctl: Box::new(Probe),
    }]
}

fn signature(interactions: &Interactions) -> Result<String> {
    let value = serde_json::to_value(interactions.snapshot())?;
    Ok(value["shared"]["signature"]
        .as_str()
        .context("The snapshot has no event signature")?
        .to_owned())
}

/// Every legacy-owned visit, with the probe injected: its gates are answered by the legacy
/// controller (or `Always`), so the event program cannot change, and a scene the legacy
/// controller skips is never offered to the registry.
pub fn precedence(assets: &mut Assets) -> Result<()> {
    let mut skipped = 0;
    for r in levels::reservations::RESERVATIONS
        .iter()
        .filter(|r| r.legacy)
    {
        let name = r.map;
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let mut world = World::from_bsp(&map)?;
        let mut interactions = Interactions::load(&map)?;
        interactions.set_entry(assets, &map, name, r.entry)?;
        let mut story = Story::load(assets, name);
        interactions.entry_story(&mut story);
        let before = signature(&interactions)?;
        let scene = interactions.scripted();
        interactions.probe_levels(&map, probe())?;
        ensure!(
            signature(&interactions)? == before,
            "{}: a registry gate was consulted before the legacy gate",
            r.id
        );
        let feet = interaction::spawn(&map, r.entry).0;
        let mut player = Player::spawn(&world, feet).unwrap_or_else(|| Player::new(feet));
        SKIPS.store(0, Ordering::SeqCst);
        let legacy = interactions.skip_cinematic(&map, &mut world, &mut player, &mut story)?;
        if scene {
            // The visit opens in a legacy scene: the legacy controller takes the skip ...
            ensure!(
                legacy && SKIPS.load(Ordering::SeqCst) == 0,
                "{}: the registry was asked to skip before the legacy controller",
                r.id
            );
            skipped += 1;
            // ... and only with no legacy scene left is the registry consulted.
            ensure!(
                interactions.skip_cinematic(&map, &mut world, &mut player, &mut story)?
                    && SKIPS.load(Ordering::SeqCst) == 1,
                "{}: the registry was not offered the skip after the legacy scene ended",
                r.id
            );
        }
        println!(
            "PASS precedence {}: legacy gates and scenes answer first",
            r.id
        );
    }
    ensure!(skipped > 0, "No legacy visit opened in a scene to skip");
    println!("PASS precedence: legacy answers precede registry answers on every legacy-owned visit ({skipped} opened in a scene)");
    Ok(())
}

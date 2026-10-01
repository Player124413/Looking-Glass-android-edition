//! Player resources and a narrow, data-driven inventory/pickup layer.
use crate::{
    assets::Assets,
    bsp::{tokens, Bsp},
    collision::World,
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeSet;

pub const WEAPONS: [(&str, &str); 10] = [
    ("knife", "Vorpal Blade"),
    ("cards", "Cards"),
    ("mallet", "Croquet Mallet"),
    ("jackbomb", "Jackbomb"),
    ("icewand", "Ice Wand"),
    ("jacks", "Jacks"),
    ("demondice", "Demon Dice"),
    ("eyestaff", "Jabberwock's Eye Staff"),
    ("blunderbuss", "Blunderbuss"),
    ("watch", "Deadtime Watch"),
];
pub const MAX_CHEAT_HEALTH: f32 = 1_000_000.;
pub struct Weapon {
    pub primary: f32,
    pub alternate: Option<f32>,
}
pub struct Catalog {
    pub weapons: Vec<Weapon>,
    pub amounts: Vec<(String, f32)>,
}

/// Read command lines inside init/server, ignoring comments, macros and client scripts.
fn server_commands(text: &str) -> Result<Vec<Vec<String>>> {
    let t = tokens(text)?;
    let Some(init) = t.iter().position(|s| s.eq_ignore_ascii_case("init")) else {
        return Ok(Vec::new());
    };
    let mut depth = 0;
    let mut server = false;
    let mut result = Vec::new();
    let mut i = init + 1;
    while i < t.len() {
        match t[i].as_str() {
            "{" => depth += 1,
            "}" => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                if depth == 1 {
                    server = false;
                }
            }
            _ if depth == 1 && t[i].eq_ignore_ascii_case("server") => server = true,
            _ if depth == 2 && server => {
                let alternate = t[i].eq_ignore_ascii_case("alternate");
                let key = i + usize::from(alternate);
                if t.get(key).is_some_and(|s| {
                    ["ammorequired", "amount"].contains(&s.to_ascii_lowercase().as_str())
                }) {
                    let value = t.get(key + 1).context("Truncated item command")?;
                    result.push(vec![
                        if alternate {
                            "alternate".into()
                        } else {
                            "primary".into()
                        },
                        t[key].to_ascii_lowercase(),
                        value.clone(),
                    ]);
                    i = key + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Ok(result)
}
impl Catalog {
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let mut weapons = Vec::new();
        for (id, _) in WEAPONS {
            let bytes = assets.read(&format!("models/w_{id}.tik"))?;
            let commands = server_commands(&String::from_utf8_lossy(&bytes))?;
            let mut primary = None;
            let mut alternate = None;
            for c in commands {
                if c[1] == "ammorequired" {
                    let v: f32 = c[2].parse()?;
                    ensure!(
                        v.is_finite() && (0.0..=100.).contains(&v),
                        "Invalid Will requirement"
                    );
                    if c[0] == "alternate" {
                        alternate = Some(v);
                    } else {
                        primary = Some(v);
                    }
                }
            }
            weapons.push(Weapon {
                primary: primary.context("Missing weapon Will cost")?,
                alternate,
            });
        }
        let mut amounts = Vec::new();
        for model in [
            "p_h1",
            "p_h2",
            "p_m1",
            "p_m2",
            "w_me_small",
            "w_me_medium",
            "w_me_large",
            "w_me_super",
        ] {
            let bytes = assets.read(&format!("models/{model}.tik"))?;
            let commands = server_commands(&String::from_utf8_lossy(&bytes))?;
            let c = commands
                .iter()
                .find(|c| c[1] == "amount")
                .context("Missing pickup amount")?;
            let amount: f32 = c[2].parse()?;
            ensure!(
                amount.is_finite() && (0.0..=100.).contains(&amount),
                "Invalid pickup amount"
            );
            amounts.push((model.to_owned(), amount));
        }
        Ok(Self { weapons, amounts })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Stats {
    /// Recomputed by the current visit before movement; never carried in a save.
    #[serde(skip)]
    pub minimum_sanity: f32,
    #[serde(default)]
    pub difficulty: crate::powerups::Difficulty,
    #[serde(default)]
    pub powers: crate::powerups::State,
    #[serde(default)]
    pub loot: std::collections::BTreeMap<String, crate::loot::Loot>,
    #[serde(default)]
    pub turtle_air: bool,
    #[serde(default)]
    pub staff_component: bool,
    #[serde(default)]
    pub scripted_immunity: f32,
    pub god: bool,
    #[serde(default)]
    pub notarget: bool,
    pub invisible: f32,
    sanity: f32,
    will: f32,
    owned: [u8; 10],
    selected: usize,
    /// Legacy save field. Original Will recovery has no post-attack delay.
    regen_delay: f32,
    pub collected: BTreeSet<String>,
    pub school_items: crate::school2_quest::Items,
}
impl Default for Stats {
    fn default() -> Self {
        // Minimal school/test baseline; game entry uses for_level's campaign profile.
        Self {
            minimum_sanity: 0.,
            difficulty: Default::default(),
            powers: Default::default(),
            loot: Default::default(),
            turtle_air: false,
            staff_component: false,
            scripted_immunity: 0.,
            god: false,
            notarget: false,
            invisible: 0.,
            sanity: 100.,
            will: 100.,
            owned: [1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
            selected: 0,
            regen_delay: 0.,
            collected: BTreeSet::new(),
            school_items: Default::default(),
        }
    }
}
impl Stats {
    pub fn validate_save(&self) -> Result<()> {
        ensure!((0. ..=2.).contains(&self.scripted_immunity), "Invalid scripted protection");
        self.powers.validate(self.invisible)?;
        ensure!(self.loot.len() <= 72, "Too many saved drop visits");
        for (visit, loot) in &self.loot {
            ensure!(visit.len() <= 128, "Invalid drop visit");
            loot.validate()?;
        }
        ensure!(
            (0.0..=MAX_CHEAT_HEALTH).contains(&self.sanity) && (0.0..=100.).contains(&self.will),
            "Invalid saved resources"
        );
        ensure!(
            (0.0..=45.).contains(&self.invisible) && (0.0..=2.).contains(&self.regen_delay),
            "Invalid saved resource timers"
        );
        ensure!(
            self.selected < 10
                && (self.owned[self.selected] > 0
                    || (self.selected == 0 && self.owned.iter().all(|&n| n == 0))),
            "Invalid saved weapon selection"
        );
        ensure!(
            self.owned
                .iter()
                .enumerate()
                .all(|(i, n)| *n <= if i == 6 { 3 } else { 1 }),
            "Invalid saved inventory"
        );
        ensure!(
            self.collected.len() <= 10000 && self.collected.iter().all(|s| s.len() <= 128),
            "Invalid saved pickups"
        );
        Ok(())
    }
    pub fn for_level(map: &str, entry: Option<&str>) -> Self {
        Self {
            turtle_air: crate::campaign::after_temple(map, entry),
            school_items: crate::school2_quest::Items {
                potion: map == "skool1" && entry == Some("skool1_start2"),
                star: map == "skool1" && entry == Some("skool1_start2"),
                ..Default::default()
            },
            owned: crate::campaign::loadout(map, entry).unwrap_or([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            ..Self::default()
        }
    }
    /// What entering a map grants on its own authority, whether or not the campaign baseline
    /// loadout is filled in (a strict chain skips only the baseline).
    pub fn arrival_grants(&mut self, map: &str) {
        // The original temple entry grants the breathing shell flag. It persists
        // with inventory, including when returning to earlier maps.
        if map == "utemple" {
            self.turtle_air = true;
        }
    }
    /// Real exits keep earned items, selection, resources and pickup history.
    /// Fill missing earlier milestones without counting a Dice pickup twice.
    pub fn ensure_level_weapons(&mut self, map: &str, entry: Option<&str>) {
        self.arrival_grants(map);
        if let Some(baseline) = crate::campaign::loadout(map, entry) {
            for (owned, minimum) in self.owned.iter_mut().zip(baseline) {
                *owned = (*owned).max(minimum);
            }
        }
    }
    pub fn weapon_summary(&self) -> String {
        if self.equipped().is_none() {
            return "Unarmed".into();
        }
        WEAPONS
            .iter()
            .enumerate()
            .filter(|(i, _)| self.owned[*i] > 0)
            .map(|(i, (_, name))| {
                if i == 6 {
                    format!("{name} x{}", self.owned[i])
                } else {
                    (*name).into()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
    pub fn sanity(&self) -> f32 {
        self.sanity
    }
    pub fn will(&self) -> f32 {
        self.will
    }
    pub fn selected(&self) -> usize {
        self.selected
    }
    /// Keep the saved slot compatible while allowing the opening unarmed state.
    pub fn equipped(&self) -> Option<usize> {
        (self.copies(self.selected) > 0).then_some(self.selected)
    }
    pub fn copies(&self, i: usize) -> u8 {
        self.owned.get(i).copied().unwrap_or(0)
    }
    pub fn alive(&self) -> bool {
        self.sanity > 0.
    }
    pub fn select(&mut self, i: usize) -> bool {
        if self.alive() && self.copies(i) > 0 {
            self.selected = i;
            true
        } else {
            false
        }
    }
    pub fn cycle(&mut self, direction: i32) {
        for n in 1..=10 {
            let i = (self.selected as i32 + direction.signum() * n).rem_euclid(10) as usize;
            if self.select(i) {
                break;
            }
        }
    }
    pub fn damage(&mut self, amount: f32) {
        if !self.god && self.scripted_immunity == 0. && amount.is_finite() && amount > 0. {
            self.sanity = (self.sanity - amount * self.difficulty.incoming()).max(self.minimum_sanity.clamp(0., self.sanity.min(100.)));
            if !self.alive() {
                self.powers.clear_effects();
                self.invisible = 0.;
            }
        }
    }
    pub fn set_health(&mut self, value: f32) -> Result<()> {
        ensure!(
            (0. ..=MAX_CHEAT_HEALTH).contains(&value),
            "Health must be between 0 and {MAX_CHEAT_HEALTH}"
        );
        self.sanity = value;
        if !self.alive() {
            self.powers.clear_effects();
            self.invisible = 0.;
        }
        Ok(())
    }
    /// Wuss grants inventory without triggering a pickup's starting-ammo refill.
    pub fn grant_weapons(&mut self) {
        let unarmed = self.equipped().is_none();
        self.owned = [1; 10];
        self.owned[6] = 3;
        if unarmed {
            self.selected = 0;
        }
    }
    pub fn ignores_alice(&self) -> bool {
        self.notarget || self.invisible > 0.
    }
    pub fn fall(&mut self, speed: f32) -> f32 {
        let before = self.sanity;
        if speed.is_finite() && speed > 550. {
            self.damage(((speed - 550.) * 0.15).floor());
        }
        before - self.sanity
    }
    /// Debit an accepted gameplay action, never selection, a busy click or an unsupported attack.
    pub fn spend_will(&mut self, cost: f32) -> bool {
        if !self.alive() || !cost.is_finite() || cost < 0. {
            return false;
        }
        if self.god {
            return true;
        }
        if cost > self.will {
            return false;
        }
        self.will = (self.will - cost).max(0.);
        self.regen_delay = 0.;
        true
    }
    pub fn update(&mut self, dt: f32) {
        if !self.alive() || !dt.is_finite() || dt <= 0. {
            return;
        }
        self.scripted_immunity = (self.scripted_immunity - dt).max(0.);
        self.invisible = (self.invisible - dt).max(0.);
        self.powers.update(dt);
        self.regen_delay = 0.;
        let (rate, cap) = self.difficulty.regen();
        if self.will < cap {
            self.will = (self.will + dt * rate).min(cap);
        }
    }
    pub fn restore(&mut self) {
        self.invisible = 0.;
        self.powers.clear_effects();
        self.sanity = 100.;
        self.will = 100.;
        self.regen_delay = 0.;
    }
    /// Scripted full_stats restores resources without removing timed powers.
    pub fn full_stats(&mut self) {
        self.sanity = 100.;
        self.will = 100.;
        self.regen_delay = 0.;
    }
    pub fn essence(&mut self, amount: f32) {
        if self.alive() && amount.is_finite() && amount > 0. {
            self.sanity = self.sanity.max((self.sanity + amount).min(100.));
            self.will = (self.will + amount).min(100.);
        }
    }
    pub fn active_power(&self) -> Option<(crate::powerups::Kind, f32)> {
        use crate::powerups::Kind;
        [
            (Kind::Rage, self.powers.rage),
            (Kind::Tea, self.powers.tea),
            (Kind::Glass, self.invisible),
        ]
        .into_iter()
        .find(|(_, t)| *t > 0.)
    }
    pub fn powerup(&mut self, kind: crate::powerups::Kind) -> bool {
        if !self.alive() || self.active_power().is_some() {
            return false;
        }
        let t = self.difficulty.duration(kind.duration());
        match kind {
            crate::powerups::Kind::Rage => self.powers.rage = t,
            crate::powerups::Kind::Tea => self.powers.tea = t,
            crate::powerups::Kind::Glass => self.invisible = t,
        }
        true
    }
    pub fn watch(&mut self) -> std::result::Result<(), &'static str> {
        if !self.alive() || self.copies(9) == 0 {
            return Err("Pocket Watch unavailable");
        }
        if self.powers.recharge > 0. {
            return Err("Pocket Watch is recharging");
        }
        if !self.spend_will(1.) {
            return Err("Not enough Will");
        }
        self.powers.stopped = 20.;
        self.powers.recharge = 360.;
        Ok(())
    }
    pub fn attack_damage(&self, base: f32) -> f32 {
        let rage = if self.powers.rage > 0. {
            match self.selected {
                0 => 4.,
                2 => 3.,
                _ => 2.,
            }
        } else {
            1.
        };
        base * self.difficulty.outgoing() * rage
    }
    pub fn prepare_player(&self, player: &mut crate::movement::Player) {
        player.tea = self.powers.tea > 0.;
        player.breath.shell = self.turtle_air;
    }
    pub fn preview() -> Self {
        let mut s = Self {
            owned: [1; 10],
            ..Self::default()
        };
        s.damage(36.);
        s.spend_will(62.);
        s.select(1);
        s
    }
    pub fn weapon_preview() -> Self {
        Self {
            owned: [1; 10],
            ..Self::default()
        }
    }
    pub fn apply(&mut self, kind: PickupKind, amount: f32) -> bool {
        if !self.alive() || !amount.is_finite() || amount < 0. {
            return false;
        }
        let before = (self.sanity, self.will, self.owned);
        match kind {
            PickupKind::Power(kind) => return self.powerup(kind),
            PickupKind::Sanity => self.sanity = self.sanity.max((self.sanity + amount).min(100.)),
            PickupKind::Will => self.will = (self.will + amount).min(100.),
            PickupKind::Essence => {
                self.sanity = self.sanity.max((self.sanity + amount).min(100.));
                self.will = (self.will + amount).min(100.);
            }
            PickupKind::Weapon(i) if i < 10 => {
                let new = self.owned[i] == 0;
                self.owned[i] = (self.owned[i] + 1).min(if i == 6 { 3 } else { 1 });
                self.will = 100.;
                if new {
                    self.selected = i;
                }
            }
            _ => return false,
        }
        before != (self.sanity, self.will, self.owned)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PickupKind {
    Sanity,
    Will,
    Essence,
    Weapon(usize),
    Power(crate::powerups::Kind),
}
pub struct Pickup {
    pub id: String,
    pub model: String,
    pub origin: Vec3,
    pub kind: PickupKind,
    pub amount: f32,
}
pub fn weapon_pickup_index(class: &str) -> Option<usize> {
    let class = class.to_ascii_lowercase();
    let id = class.strip_prefix("item_weaponpickup_")?;
    // The map classname and the weapon definition use different Watch names.
    let id = if id == "deadtimewatch" { "watch" } else { id };
    WEAPONS.iter().position(|(name, _)| *name == id)
}
impl Pickup {
    pub fn name(&self) -> &str {
        match self.kind {
            PickupKind::Power(kind) => kind.name(),
            PickupKind::Sanity => "Sanity shard",
            PickupKind::Will => "Vial of Will",
            PickupKind::Essence => "Meta-essence",
            PickupKind::Weapon(i) => WEAPONS[i].1,
        }
    }
}
pub fn pickups_for_visit(
    map: &Bsp,
    name: &str,
    entry: Option<&str>,
    catalog: &Catalog,
) -> Vec<Pickup> {
    pickups(map, name, catalog)
        .into_iter()
        .filter(|p| {
            if name == "wforest" && entry != Some("wforest_start2") && p.id == "wforest:73" {
                return false;
            }
            if name != "skool1" {
                return true;
            }
            let Some(e) =
                p.id.rsplit(':')
                    .next()
                    .and_then(|i| i.parse::<usize>().ok())
                    .and_then(|i| map.entities.get(i))
            else {
                return false;
            };
            let target = e.get("targetname").map(String::as_str).unwrap_or("");
            if entry == Some("skool1_start2") {
                target != "mallet_skool1"
            } else {
                target != "mana_skool1"
            }
        })
        .collect()
}
pub fn pickups(map: &Bsp, name: &str, catalog: &Catalog) -> Vec<Pickup> {
    map.entities
        .iter()
        .enumerate()
        .filter_map(|(index, e)| {
            let flags = e
                .get("spawnflags")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0);
            if !map.difficulty.allows(flags) || flags & 64 != 0 {
                return None;
            }
            let class = e.get("classname")?.to_ascii_lowercase();
            // This reviewed spawner is activated by fortress2's initial setup.
            // Keep its BSP identity; other delayed spawners need their own gates.
            let rage_spawn = name == "fortress2"
                && class == "func_spawn"
                && e.get("targetname").is_some_and(|n| n == "spawn_ragebox")
                && e.get("modelname").is_some_and(|n| n == "w_ragebox.tik");
            // School's secret shelf owns its own gated Glass and story reward.
            if name == "skool1" && class == "item_darkenedlookingglass" {
                return None;
            }
            let kind = if rage_spawn {
                PickupKind::Power(crate::powerups::Kind::Rage)
            } else if let Some(kind) = crate::powerups::Kind::class(&class) {
                PickupKind::Power(kind)
            } else if class.starts_with("item_weaponpickup_") {
                PickupKind::Weapon(weapon_pickup_index(&class)?)
            } else if class.starts_with("item_health_") {
                PickupKind::Sanity
            } else if class.starts_with("item_mana_") {
                PickupKind::Will
            } else if class.starts_with("item_metaessence_")
                || class.starts_with("item_metaessense-")
            {
                PickupKind::Essence
            } else {
                return None;
            };
            let xyz = e
                .get("origin")?
                .split_whitespace()
                .map(str::parse::<f32>)
                .collect::<std::result::Result<Vec<_>, _>>()
                .ok()?;
            if xyz.len() != 3 {
                return None;
            }
            let origin = vec3(xyz[0], xyz[1], xyz[2]);
            if !origin.is_finite() {
                return None;
            }
            let model = e
                .get(if rage_spawn { "modelname" } else { "model" })?
                .trim_start_matches("models/")
                .trim_end_matches(".tik");
            let amount = if matches!(kind, PickupKind::Weapon(_) | PickupKind::Power(_)) {
                100.
            } else {
                catalog.amounts.iter().find(|(m, _)| m == model)?.1
            };
            Some(Pickup {
                id: format!("{name}:{index}"),
                model: model.to_owned(),
                origin,
                kind,
                amount,
            })
        })
        .collect()
}
pub fn collect(stats: &mut Stats, items: &[Pickup], feet: Vec3, world: &World) -> Vec<String> {
    collect_with(stats, items, feet, world, |_| {})
}
/// Presentation is notified only after a successful grant, never by save restore.
pub fn collect_with(
    stats: &mut Stats,
    items: &[Pickup],
    feet: Vec3,
    world: &World,
    mut collected: impl FnMut(&Pickup),
) -> Vec<String> {
    let mut messages = Vec::new();
    for p in items {
        if stats.collected.contains(&p.id) || !touches_pickup(feet, p.origin, world) {
            continue;
        }
        let changed = stats.apply(p.kind, p.amount);
        // Story pickups must also work in old saves that already granted the toy.
        // The opening Blade used to be granted early; the Jackbomb starts its boss.
        if changed
            || (stats.alive()
                && ((p.id.starts_with("potears3:") && matches!(p.kind, PickupKind::Weapon(3)))
                    || (p.id.starts_with("gvillage:") && matches!(p.kind, PickupKind::Weapon(0)))))
        {
            stats.collected.insert(p.id.clone());
            collected(p);
            messages.push(format!("Collected {}", p.name()));
        }
    }
    messages
}

fn touches_pickup(feet: Vec3, origin: Vec3, world: &World) -> bool {
    // The original item box is (-16,-16,0)..(16,16,64). Test it against
    // Alice's body, including the corners, rather than a smaller centre radius.
    let reach = crate::collision::PLAYER_HALF.truncate() + Vec2::splat(16.);
    let low = feet.z.max(origin.z);
    let high = (feet.z + crate::collision::PLAYER_HALF.z * 2.).min(origin.z + 64.);
    if (feet - origin).truncate().abs().cmpgt(reach).any() || low > high {
        return false;
    }
    // Trace through the overlapping part of both boxes. Fixed-height endpoints
    // could put the item end inside a step even while Alice touched its top.
    // Multiple heights permit contact above low cover, never through a wall.
    [0.5, 0.1, 0.9].into_iter().any(|t| {
        let z = low + (high - low) * t;
        let hit = world.sweep(
            Vec3::new(feet.x, feet.y, z),
            Vec3::new(origin.x, origin.y, z),
            Vec3::ZERO,
        );
        !hit.start_solid && hit.fraction == 1.
    })
}

/// A contact notice is transient UI state, never part of a save or item grant.
#[derive(Default)]
pub struct PickupFeedback {
    full: Option<&'static str>,
}
impl PickupFeedback {
    pub fn update(
        &mut self,
        stats: &Stats,
        items: &[Pickup],
        visit: &str,
        feet: Vec3,
        world: &World,
    ) -> Option<&'static str> {
        let placed = items
            .iter()
            .filter(|p| !stats.collected.contains(&p.id))
            .map(|p| (p.kind, p.origin));
        let drops = stats
            .loot
            .get(visit)
            .into_iter()
            .flat_map(|loot| &loot.drops)
            .map(|drop| (PickupKind::Essence, drop.origin));
        let full = placed.chain(drops).find_map(|(kind, origin)| {
            let message = match kind {
                PickupKind::Sanity if stats.sanity() >= 100. => "Sanity full",
                PickupKind::Will if stats.will() >= 100. => "Will full",
                PickupKind::Essence if stats.sanity() >= 100. && stats.will() >= 100. => {
                    "Sanity and Will full"
                }
                _ => return None,
            };
            touches_pickup(feet, origin, world).then_some(message)
        });
        let notice = full.filter(|_| self.full != full);
        self.full = full;
        notice
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let c = Catalog::load(assets)?;
    for (i, w) in c.weapons.iter().enumerate() {
        println!(
            "{}: base Will {}, alternate {:?}",
            WEAPONS[i].1, w.primary, w.alternate
        );
    }
    let mut count = 0;
    for name in assets.maps() {
        let mut map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let n = pickups(&map, &name, &c).len();
        count += n;
        println!("{name}: {n} supported static pickups");
        for difficulty in crate::powerups::Difficulty::ALL {
            map.difficulty = difficulty;
            let items = pickups(&map, &name, &c);
            for (id, entity) in map.entities.iter().enumerate() {
                let class = entity.get("classname").map(|s| s.to_ascii_lowercase()).unwrap_or_default();
                if !class.starts_with("item_") { continue; }
                let flags = entity.get("spawnflags").and_then(|s| s.parse().ok()).unwrap_or(0);
                let owned = name == "skool1" && class == "item_darkenedlookingglass";
                let expected = difficulty.allows(flags) && flags & 64 == 0 && !owned;
                ensure!(items.iter().any(|p| p.id == format!("{name}:{id}")) == expected,
                    "Unaccounted item {name}:{id} {class} on {}", difficulty.name());
            }
        }
    }
    let mut stats = Stats::default();
    stats.damage(40.);
    ensure!(stats.spend_will(30.), "Will debit failed");
    ensure!(
        stats.apply(PickupKind::Essence, 25.) && stats.sanity() == 85. && stats.will() == 95.,
        "Resource restore failed"
    );
    println!("PASS: 10 weapon definitions, 8 pickup definitions, {count} supported placed pickups; resource transitions passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn collection_feedback_follows_real_grant_and_never_save_load() {
        let item = Pickup { id: "fixture:0".into(), model: "p_h1".into(),
            origin: Vec3::ZERO, kind: PickupKind::Sanity, amount: 15. };
        let items = [item];
        let open = World::fixture(&[]);
        let wall = World::fixture(&[(vec3(8., -100., -100.), vec3(12., 100., 100.))]);
        let mut stats = Stats::default();
        let mut events = Vec::new();
        collect_with(&mut stats, &items, Vec3::ZERO, &open, |p| events.push(p.id.clone()));
        assert!(events.is_empty());
        stats.damage(50.);
        collect_with(&mut stats, &items, Vec3::X * 20., &wall, |p| events.push(p.id.clone()));
        assert!(events.is_empty());
        collect_with(&mut stats, &items, Vec3::ZERO, &open, |p| events.push(p.id.clone()));
        assert_eq!(events, ["fixture:0"]);
        let mut restored: Stats = serde_json::from_slice(&serde_json::to_vec(&stats).unwrap()).unwrap();
        collect_with(&mut restored, &items, Vec3::ZERO, &open, |p| events.push(p.id.clone()));
        assert_eq!(events.len(), 1);
        assert_eq!(restored.sanity(), stats.sanity());
    }
    #[test]
    fn scripted_minimum_preserves_a_slide_without_healing_or_protecting_later_combat() {
        let mut s=Stats::default();s.minimum_sanity=30.;
        s.damage(1000.);assert_eq!(s.sanity(),30.);
        s.set_health(12.).unwrap();s.damage(1000.);assert_eq!(s.sanity(),12.);
        s.minimum_sanity=0.;s.damage(1000.);assert!(!s.alive());
        s.minimum_sanity=30.;s.damage(1.);assert!(!s.alive());
        let restored:Stats=serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
        assert_eq!(restored.minimum_sanity,0.);
    }
    #[test]
    fn full_pickup_notice_requires_contact_and_does_not_repeat_or_consume() {
        let mut stats = Stats::default();
        let world = World::fixture(&[]);
        let items = [Pickup {
            id: "essence".into(),
            model: "w_me_small".into(),
            origin: Vec3::ZERO,
            kind: PickupKind::Essence,
            amount: 15.,
        }];
        let mut feedback = PickupFeedback::default();
        assert_eq!(
            feedback.update(&stats, &items, "test", Vec3::X * 100., &world),
            None
        );
        assert_eq!(
            feedback.update(&stats, &items, "test", Vec3::ZERO, &world),
            Some("Sanity and Will full")
        );
        assert_eq!(
            feedback.update(&stats, &items, "test", Vec3::ZERO, &world),
            None
        );
        assert!(collect(&mut stats, &items, Vec3::ZERO, &world).is_empty());
        assert!(stats.collected.is_empty());
        stats.spend_will(20.);
        assert_eq!(
            feedback.update(&stats, &items, "test", Vec3::ZERO, &world),
            None
        );
        assert_eq!(collect(&mut stats, &items, Vec3::ZERO, &world).len(), 1);
        assert_eq!(stats.will(), 95.);
        assert_eq!(
            feedback.update(&stats, &items, "test", Vec3::ZERO, &world),
            None
        );
        let wall = World::fixture(&[(vec3(8., -100., -100.), vec3(12., 100., 100.))]);
        let mut feedback = PickupFeedback::default();
        assert_eq!(
            feedback.update(&Stats::default(), &items, "test", Vec3::X * 20., &wall),
            None
        );
    }
    #[test]
    fn developer_god_mode_protects_health_and_energy_without_changing_quest() {
        let mut s = Stats::default();
        s.damage(12.);
        assert_eq!(s.sanity(), 88.);
        s.school_items.mushroom = true;
        s.god = true;
        s.damage(10000.);
        assert_eq!(s.fall(1000.), 0.);
        assert_eq!(s.sanity(), 88.);
        assert!(s.spend_will(20.));
        assert_eq!(s.will(), 100.);
        assert!(s.school_items.mushroom);
        s.god = false;
        s.damage(12.);
        assert_eq!(s.sanity(), 76.);
        assert!(!Stats::for_level("skool2", None).god);
    }
    #[test]
    fn wuss_health_and_zero_energy_god_keep_distinct_semantics() {
        let mut s = Stats::for_level("gvillage", None);
        s.damage(60.);
        s.spend_will(100.);
        s.grant_weapons();
        assert_eq!(
            (s.sanity(), s.will(), s.equipped(), s.copies(6)),
            (40., 0., Some(0), 3)
        );
        s.select(4);
        s.grant_weapons();
        assert_eq!((s.selected(), s.will()), (4, 0.));
        s.god = true;
        for _ in 0..20 {
            assert!(s.spend_will(99.));
            s.damage(1000.);
        }
        assert_eq!((s.sanity(), s.will()), (40., 0.));
        assert!(!s.spend_will(f32::NAN));
        assert!(!s.spend_will(-1.));
        s.god = false;
        assert!(!s.spend_will(1.));
        s.set_health(250.).unwrap();
        s.apply(PickupKind::Essence, 15.);
        assert_eq!(s.sanity(), 250.);
        s.damage(20.);
        assert_eq!(s.sanity(), 230.);
        s.notarget = true;
        let saved = serde_json::to_vec(&s).unwrap();
        let restored: Stats = serde_json::from_slice(&saved).unwrap();
        restored.validate_save().unwrap();
        assert_eq!(restored.sanity(), 230.);
        assert!(restored.notarget);
        s.set_health(0.).unwrap();
        assert!(!s.alive());
        s.set_health(100.).unwrap();
        assert!(s.alive());
    }
    #[test]
    fn fresh_level_loadouts_reset_future_toys_but_exits_preserve_earned_state() {
        let mut s = Stats::for_level("skool2", None);
        assert_eq!(s.copies(2), 1);
        s.apply(PickupKind::Weapon(6), 100.);
        s.apply(PickupKind::Weapon(9), 100.); // A previously earned bonus is retained at exits.
        s.select(2);
        s.damage(30.);
        s.spend_will(40.);
        s.collected.insert("skool2:dice".into());
        for _ in 0..3 {
            s.ensure_level_weapons("skool1", Some("skool1_start2"));
        }
        assert_eq!(s.copies(6), 1); // Already collected, not another free die per return.
        assert_eq!(
            (s.copies(9), s.selected(), s.sanity(), s.will()),
            (1, 2, 70., 60.)
        );
        assert!(s.collected.contains("skool2:dice"));
        s.ensure_level_weapons("hatter1", None);
        assert_eq!((s.copies(5), s.copies(6)), (1, 3));
        s.restore();
        assert_eq!(s.copies(6), 3);
        let early = Stats::for_level("skool1", None);
        assert_eq!((early.copies(2), early.copies(9)), (0, 0));
        assert!(early.collected.is_empty());
        assert!(early.copies(early.selected()) > 0);
    }
    #[test]
    fn village_starts_unarmed_and_pickup_equips_once_in_new_and_legacy_saves() {
        let world = World::fixture(&[]);
        let pickup = Pickup {
            id: "gvillage:knife".into(),
            model: "w_knife".into(),
            origin: Vec3::ZERO,
            kind: PickupKind::Weapon(0),
            amount: 100.,
        };
        let mut stats = Stats::for_level("gvillage", None);
        assert_eq!(stats.equipped(), None);
        stats.cycle(1);
        assert!(!stats.select(0));
        stats.validate_save().unwrap();
        let json = serde_json::to_string(&stats).unwrap();
        stats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.equipped(), None);
        for mut stats in [stats, Stats::for_level("pandemonium", None)] {
            let messages = collect(
                &mut stats,
                std::slice::from_ref(&pickup),
                Vec3::ZERO,
                &world,
            );
            assert_eq!(messages, ["Collected Vorpal Blade"]);
            assert_eq!(stats.equipped(), Some(0));
            stats.validate_save().unwrap();
            assert!(collect(
                &mut stats,
                std::slice::from_ref(&pickup),
                Vec3::ZERO,
                &world
            )
            .is_empty());
            let json = serde_json::to_string(&stats).unwrap();
            let restored: Stats = serde_json::from_str(&json).unwrap();
            assert_eq!(restored.equipped(), Some(0));
            assert!(restored.collected.contains(&pickup.id));
        }
    }
    #[test]
    fn watch_map_alias_is_collectible_and_preview_keeps_all_toys() {
        assert_eq!(
            weapon_pickup_index("Item_WeaponPickup_DeadtimeWatch"),
            Some(9)
        );
        assert_eq!(weapon_pickup_index("item_weaponpickup_watch"), Some(9));
        assert_eq!(weapon_pickup_index("item_weaponpickup_unknown"), None);
        let mut preview = Stats::weapon_preview();
        preview.ensure_level_weapons("gvillage", None);
        assert!((0..10).all(|i| preview.copies(i) > 0));
        assert_eq!(preview.sanity(), 100.);
        assert!(Stats::preview().sanity() < 100.);
    }
    #[test]
    fn ownership_selection_upgrades_and_resource_invariants() {
        let mut s = Stats::default();
        assert!(!s.select(9));
        s.cycle(1);
        assert_eq!(s.selected(), 1);
        s.cycle(1);
        assert_eq!(s.selected(), 0);
        s.cycle(-1);
        assert_eq!(s.selected(), 1);
        assert!(!s.select(100));
        s.damage(40.);
        s.damage(f32::NAN);
        assert!(s.spend_will(80.));
        assert!(!s.spend_will(30.));
        assert!(!s.spend_will(-1.));
        s.apply(PickupKind::Essence, 25.);
        assert_eq!((s.sanity(), s.will()), (85., 45.));
        s.apply(PickupKind::Sanity, 1000.);
        assert_eq!(s.sanity(), 100.);
        assert!(!s.apply(PickupKind::Sanity, 15.));
        s.apply(PickupKind::Weapon(6), 100.);
        assert_eq!(s.selected(), 6);
        assert_eq!(s.will(), 100.);
        for _ in 0..5 {
            s.apply(PickupKind::Weapon(6), 100.);
        }
        assert_eq!(s.copies(6), 3);
        s.damage(1000.);
        assert!(!s.alive());
        assert!(!s.apply(PickupKind::Sanity, 100.));
        assert!(!s.spend_will(0.));
        s.update(20.);
        assert_eq!(s.sanity(), 0.);
        s.restore();
        assert!(s.alive());
        assert_eq!(s.copies(6), 3);
    }
    #[test]
    fn recovery_is_frame_independent_and_never_heals_sanity() {
        use crate::powerups::Difficulty;
        for (difficulty, expected) in [(Difficulty::Easy, 15.), (Difficulty::Normal, 5.)] {
            for hz in [30, 60, 144] {
                let mut s = Stats { difficulty, ..Stats::default() };
                s.set_health(60.).unwrap();
                assert!(s.spend_will(100.));
                for _ in 0..50 * hz {
                    s.update(1. / hz as f32);
                }
                assert!((s.will() - expected).abs() < 0.002, "{difficulty:?}, {hz} Hz");
                assert_eq!(s.sanity(), 60.);
                let before = s.will();
                for dt in [0., -1., f32::NAN, f32::INFINITY] {
                    s.update(dt);
                    assert_eq!(s.will(), before);
                }
                s.damage(100.);
                s.update(50.);
                assert_eq!(s.will(), before);
                assert_eq!(s.sanity(), 0.);
            }
        }
    }
    #[test]
    fn saved_legacy_recovery_delay_does_not_block_recharge() {
        let mut s = Stats::default();
        s.spend_will(100.);
        s.regen_delay = 2.; // Save written by a build with the obsolete cooldown.
        let mut loaded: Stats = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        loaded.validate_save().unwrap();
        loaded.update(1.);
        assert!((loaded.will() - 0.1).abs() < 0.001);
        assert_eq!(loaded.regen_delay, 0.);
        assert!(loaded.spend_will(0.));
        loaded.update(1.);
        assert!((loaded.will() - 0.2).abs() < 0.001);
    }
    #[test]
    fn recovery_preserves_resources_above_the_difficulty_cap_and_normal_jumps_are_safe() {
        let mut a = Stats::default();
        a.spend_will(50.);
        let mut b = a.clone();
        a.update(3.);
        for _ in 0..180 {
            b.update(1. / 60.);
        }
        assert_eq!((a.will(), b.will()), (50., 50.));
        assert_eq!(a.fall(300.), 0.);
        assert!(a.fall(800.) > 30.);
        let before = a.will();
        a.update(0.);
        assert_eq!(a.will(), before);
    }
    #[test]
    fn definition_reader_ignores_comments_and_client_commands() {
        let s="init { server { // ammorequired 99\n ammorequired 3 alternate ammorequired 20 } client { amount 999 } }";
        let c = server_commands(s).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0][2], "3");
        assert_eq!(c[1][0], "alternate");
    }
    #[test]
    fn owned_jackbomb_at_full_will_still_consumes_boss_trigger_pickup_once() {
        let mut stats = Stats::for_level("potears3", None);
        stats.apply(PickupKind::Weapon(3), 1.);
        let pickup = Pickup {
            id: "potears3:21".into(),
            model: "w_jackbomb".into(),
            origin: Vec3::ZERO,
            kind: PickupKind::Weapon(3),
            amount: 1.,
        };
        let world = World::fixture(&[]);
        assert_eq!(
            collect(
                &mut stats,
                std::slice::from_ref(&pickup),
                Vec3::ZERO,
                &world
            )
            .len(),
            1
        );
        assert!(stats.collected.contains(&pickup.id));
        assert_eq!(stats.copies(3), 1);
        assert!(collect(&mut stats, &[pickup], Vec3::ZERO, &world).is_empty());
    }
    #[test]
    fn pickups_are_single_use_and_cannot_be_collected_through_walls() {
        let world = World::fixture(&[(vec3(10., -100., -100.), vec3(12., 100., 100.))]);
        let p = Pickup {
            id: "test:1".into(),
            model: "p_h1".into(),
            origin: vec3(20., 0., 0.),
            kind: PickupKind::Sanity,
            amount: 15.,
        };
        let mut s = Stats::default();
        s.damage(50.);
        assert!(collect(&mut s, &[p], Vec3::ZERO, &world).is_empty());
        assert_eq!(s.sanity(), 50.);
        let p = Pickup {
            id: "test:2".into(),
            model: "p_h1".into(),
            origin: Vec3::ZERO,
            kind: PickupKind::Sanity,
            amount: 15.,
        };
        assert_eq!(
            collect(&mut s, std::slice::from_ref(&p), Vec3::ZERO, &world).len(),
            1
        );
        assert!(collect(&mut s, &[p], Vec3::ZERO, &world).is_empty());
        assert_eq!(s.sanity(), 65.);
    }
}

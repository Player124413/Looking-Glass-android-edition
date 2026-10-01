//! Shared live input replay for village/school progression and combat.
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::World,
    interaction::Interactions,
    inventory::Stats,
    movement::{Controls, Player, FIXED_DT},
};
use anyhow::{Context, Result};
use macroquad::prelude::*;

pub mod fight;
mod foes;
mod metrics;
mod resume;

pub use foes::guards_of;
pub use metrics::{Audit, Metrics};
pub use resume::{run_identical, Checkpoint};

/// How close to a navigation goal the planner must bring Alice, and the wider radius it falls
/// back to when no state of the search stands inside the first.
const NAV_TOLERANCE: f32 = 22.;
const NAV_TOLERANCE_WIDE: f32 = 40.;

/// Set `LOOKING_GLASS_ROUTE_TRACE=1` to print every hit Alice takes, each throw and swing, and
/// what they hit. The route checks are re-tuned whenever the combat rules change; this is how.
pub(crate) fn trace(message: impl FnOnce() -> String) {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *ON.get_or_init(|| std::env::var_os("LOOKING_GLASS_ROUTE_TRACE").is_some()) {
        println!("  TRACE {}", message());
    }
}

pub struct Route {
    visit: String,
    name: String,
    entry: Option<String>,
    /// Fight with `fight::plan` (awake targets only, melee in reach, led throws) instead of
    /// throwing at the nearest visible enemy. Set by [`Route::clear`]; routes recorded before
    /// the Blade/Cards rules keep the original driver so their metrics stay comparable.
    pub tactics: bool,
    /// Driver weapon choice: keep Will for later encounters, still fund every attack.
    pub conserve_will: bool,
    /// Route policy only: leave enough earned Will for a required later action.
    pub will_reserve: f32,
    /// Use the earned Ice Wand's funded dry primary stream at close range.
    pub ice_stream: bool,
    watch_input: bool,
    fight: fight::Tracker,
    pub skip_cinematics: bool,
    pub use_pressed: bool,
    pub aim_at: Option<usize>,
    /// Optional Staff/cannon input for finale replays; effects use production physics.
    pub heavy_weapon: Option<usize>,
    pub heavy: crate::weapons::route_heavy::Heavy,
    pub map: Bsp,
    pub world: World,
    pub interactions: Interactions,
    pub player: Player,
    pub stats: Stats,
    pub transition: Option<(String, Option<String>)>,
    pub ticks: usize,
    pub jumps: u64,
    pub story: crate::story::Story,
    pub actions: crate::weapons::Actions,
    pub base: Vec<crate::skeletal::Transform>,
    pub projectiles: Vec<crate::weapons::Projectile>,
    pub shots: usize,
    /// Blade melee swings released (tactics only; throws are counted in `shots`).
    pub swings: usize,
    /// Cards released (tactics only); each is paid for with Will.
    pub cards: usize,
    pub damage: f32,
    pub teleports: usize,
    pub pickups: Vec<crate::inventory::Pickup>,
    /// The Cheshire hints of this visit, observed and updated like the viewer's.
    pub hints: crate::cheshire::Hints,
    /// The visits completed so far, handed from leg to leg by [`Route::depart`].
    pub ledger: crate::save::Campaign,
    pub difficulty: crate::powerups::Difficulty,
    /// Freeze the route the moment it takes its exit: every later tick does nothing, and the
    /// walking and waiting helpers return, so a leg ends at the exit instead of playing on in a
    /// level the viewer would already have left. Off for the recorded single-visit checks, whose
    /// metrics include the ticks after the exit.
    pub stop_at_exit: bool,
    /// Placed club guards: the headless stand-in for the `Npcs` the viewer builds (they need
    /// textures). Only visits whose cast is not owned by a controller have any.
    pub guards: Vec<crate::combat::Guard>,
    /// Opt-in production cast for windowed route checks, including resident enemies.
    pub native_cast: Option<crate::npc::Npcs>,
    guard_timing: Option<crate::combat::Timing>,
    /// The viewer's visual clocks, kept because they are part of a saved visit.
    pub environment_clock: f32,
    pub pickup_clock: f32,
    pub audit: Audit,
    start: metrics::Start,
    /// Save/continue probe (`resume.rs`): stop at the first tick at or after this count.
    freeze_at: Option<usize>,
    frozen: bool,
}
impl Route {
    /// The route every recorded check uses: the visit's own baseline resources, Normal difficulty.
    pub fn new(assets: &mut Assets, name: &str, entry: Option<&str>) -> Result<Self> {
        Self::enter(
            assets,
            name,
            entry,
            Stats::for_level(name, entry),
            crate::powerups::Difficulty::Normal,
        )
    }
    /// Start a visit at its entrance carrying `carried` resources at `difficulty`. The
    /// difficulty reaches the world the way the viewer applies it: through the map's spawn and
    /// pickup filters, and through the damage Alice takes and deals.
    pub fn enter(
        assets: &mut Assets,
        name: &str,
        entry: Option<&str>,
        carried: Stats,
        difficulty: crate::powerups::Difficulty,
    ) -> Result<Self> {
        let loaded = crate::campaign::load_visit(assets, name, entry, None, difficulty, true)?;
        let player = Player::spawn(
            &loaded.world,
            crate::interaction::spawn(&loaded.map, entry).0,
        )
        .with_context(|| format!("{name} spawn blocked"))?;
        Self::assemble(
            assets,
            name,
            entry,
            loaded,
            player,
            carried,
            crate::save::Campaign::default(),
        )
    }
    /// Take the exit this route ended on and enter the next visit the way the viewer does
    /// (`campaign::arrive`): Alice's resources and the ledger of completed visits carry over,
    /// and a destination whose entrance is obstructed fails the leg. `strict` skips the campaign
    /// baseline loadout and keeps only what the visits themselves grant.
    pub fn depart(self, assets: &mut Assets, strict: bool) -> Result<Self> {
        let exit = self
            .transition
            .clone()
            .context("The route has not reached an exit")?;
        let leaving = self.level();
        let Self {
            mut stats,
            mut ledger,
            ..
        } = self;
        let visit =
            crate::campaign::arrive(assets, &mut stats, &mut ledger, leaving, &exit, strict)?;
        Self::assemble(
            assets,
            &visit.map,
            visit.entry.as_deref(),
            visit.loaded,
            visit.player,
            stats,
            ledger,
        )
    }
    /// This visit as the ledger stores it. A headless route has no cast (`Npcs` needs textures),
    /// so its cast snapshot is empty.
    pub fn level(&self) -> crate::save::Level {
        crate::save::Level {
            map: self.name.clone(),
            entry: self.entry.clone(),
            interactions: self.interactions.snapshot(),
            npcs: self.native_cast.as_ref().map_or_else(Default::default, |n| n.snapshot()),
            story: self.story.snapshot(),
            hints: self.hints.snapshot(),
            environment_clock: self.environment_clock,
            pickup_clock: self.pickup_clock,
        }
    }
    fn assemble(
        assets: &mut Assets,
        name: &str,
        entry: Option<&str>,
        loaded: crate::campaign::Loaded,
        player: Player,
        mut stats: Stats,
        ledger: crate::save::Campaign,
    ) -> Result<Self> {
        let crate::campaign::Loaded {
            map,
            world,
            interactions,
            story,
            hints,
        } = loaded;
        let difficulty = map.difficulty;
        stats.difficulty = difficulty;
        let def = crate::skeletal::Definition::alice(assets)?;
        let skeleton = crate::skeletal::Skeleton::parse(
            &assets.read(&format!("{}/{}", def.path, def.model))?,
        )?;
        let base = crate::skeletal::Animation::parse(
            &assets.read("models/alice/ready.ska")?,
            skeleton.bones.len(),
        )?
        .sample(0., true);
        let actions = crate::weapons::Actions::load(assets, skeleton.bones.len(), &base)?;
        let pickups = crate::inventory::pickups_for_visit(
            &map,
            name,
            entry,
            &crate::inventory::Catalog::load(assets)?,
        );
        let (guards, guard_timing) = foes::place_guards(assets, &map, name, &interactions)?;
        Ok(Self {
            visit: crate::save::visit_key(name, entry),
            name: name.into(),
            entry: entry.map(str::to_owned),
            tactics: false,
            conserve_will: false,
            will_reserve: 0.,
            ice_stream: false,
            watch_input: false,
            fight: Default::default(),
            skip_cinematics: false,
            use_pressed: false,
            aim_at: None,
            heavy_weapon: None,
            heavy: Default::default(),
            map,
            world,
            interactions,
            player,
            start: metrics::Start {
                sanity: stats.sanity(),
                will: stats.will(),
                collected: stats.collected.clone(),
            },
            audit: Audit {
                lowest_sanity: stats.sanity(),
                ..Default::default()
            },
            stats,
            transition: None,
            ticks: 0,
            jumps: 0,
            story,
            hints,
            ledger,
            difficulty,
            stop_at_exit: false,
            guards,
            native_cast: None,
            guard_timing,
            environment_clock: 0.,
            pickup_clock: 0.,
            actions,
            base,
            projectiles: Vec::new(),
            shots: 0,
            swings: 0,
            cards: 0,
            damage: 0.,
            teleports: 0,
            pickups,
            freeze_at: None,
            frozen: false,
        })
    }
    /// True once a route that latches at its exit (`stop_at_exit`) has taken it, or has been
    /// frozen by the save/continue probe (`freeze_at`).
    pub fn exited(&self) -> bool {
        self.frozen || (self.stop_at_exit && self.transition.is_some())
    }
    pub fn wait_for_cinematic(&mut self) -> Result<()> {
        for _ in 0..24000 {
            if (!self.interactions.scripted() && !self.interactions.levels_controlled())
                || self.transition.is_some()
            {
                return Ok(());
            }
            self.tick(Controls::default())?;
        }
        anyhow::bail!("Cinematic did not release control")
    }
    pub fn navigate(&mut self, goal: Vec3) -> Result<()> {
        self.navigate_replan(goal, 8, false)
    }
    /// Yield to a newly started scene instead of following an obsolete arena goal.
    pub fn navigate_until_scene(&mut self, goal: Vec3) -> Result<()> {
        self.navigate_replan(goal, 8, true)
    }
    /// Search for inputs that carry a copy of `start` to within `tolerance` of `goal`, standing.
    /// Each step is 24 ticks of one of nine headings, with or without a jump; the plan is the list
    /// of inputs with the position each is expected to end at. `Err` is the number of states
    /// searched before giving up.
    fn plan(
        world: &World,
        start: &Player,
        goal: Vec3,
        tolerance: f32,
    ) -> Result<Vec<(Controls, Vec3)>, usize> {
        use std::cmp::Reverse;
        use std::collections::{BinaryHeap, HashSet};
        let mut nodes = vec![(start.clone(), 0usize, Controls::default(), 0usize)];
        let mut queue = BinaryHeap::from([Reverse((0u32, 0usize))]);
        let mut seen = HashSet::new();
        while let Some(Reverse((_, index))) = queue.pop() {
            let (p, _, _, cost) = nodes[index].clone();
            if p.feet.distance(goal) < tolerance && p.grounded {
                let mut path = Vec::new();
                let mut at = index;
                while at != 0 {
                    path.push((nodes[at].2, nodes[at].0.feet));
                    at = nodes[at].1;
                }
                path.reverse();
                return Ok(path);
            }
            if nodes.len() > 100000 {
                break;
            }
            for d in 0..9 {
                let angle = d as f32 * std::f32::consts::FRAC_PI_4;
                let wish = if d == 8 {
                    Vec2::ZERO
                } else {
                    vec2(angle.cos(), angle.sin())
                };
                for jump in [false, true] {
                    if jump && !p.grounded && p.ledge.is_none() {
                        continue;
                    }
                    let input = Controls {
                        wish,
                        jump,
                        run: true,
                        ..Default::default()
                    };
                    let mut next = p.clone();
                    for t in 0..24 {
                        next.tick(
                            world,
                            Controls {
                                jump: jump && t == 0,
                                ..input
                            },
                        );
                    }
                    if next.feet.z < goal.z - 320. || !world.body_clear(next.feet) {
                        continue;
                    }
                    let key = (
                        (next.feet.x / 12.).round() as i32,
                        (next.feet.y / 12.).round() as i32,
                        (next.feet.z / 12.).round() as i32,
                        (next.velocity.z / 120.).round() as i32,
                        // The early pull-up moves less than one position cell. Keep its
                        // animation clock distinct so the search can finish the climb.
                        next.ledge.as_ref().map(|h| (h.pulling, (h.elapsed * 10.).round() as u8)),
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    let priority = ((cost + 1) as f32 * 40. + next.feet.distance(goal) * 2.) as u32;
                    let id = nodes.len();
                    nodes.push((next, index, input, cost + 1));
                    queue.push(Reverse((priority, id)));
                }
            }
        }
        Err(nodes.len())
    }
    fn navigate_replan(&mut self, goal: Vec3, replans: u8, yield_scene: bool) -> Result<()> {
        if yield_scene && self.interactions.scripted() {
            return Ok(());
        }
        self.wait_for_cinematic()?;
        if self.exited() {
            return Ok(());
        }
        // A goal on a narrow ledge can fall between two 24-tick steps: no state of the search
        // stands within the tolerance, although Alice can walk there. A goal that plans as
        // written keeps its plan (so every recorded route is unchanged); one that does not gets
        // a second search with a wider tolerance, whatever the difficulty put Alice at.
        let (world, start) = (&self.world, &self.player);
        let (path, tolerance) = match Self::plan(world, start, goal, NAV_TOLERANCE) {
            Ok(path) => (path, NAV_TOLERANCE),
            Err(first) => match Self::plan(world, start, goal, NAV_TOLERANCE_WIDE) {
                Ok(path) => {
                    println!(
                        "  Widened the tolerance for {goal:?} to {NAV_TOLERANCE_WIDE} (none within {NAV_TOLERANCE} in {first} states)"
                    );
                    (path, NAV_TOLERANCE_WIDE)
                }
                Err(wide) => anyhow::bail!(
                    "No physics-input route to {goal:?} from {:?} ({wide} states)",
                    self.player.feet
                ),
            },
        };
        for (input, expected) in path {
            if yield_scene && self.interactions.scripted() {
                return Ok(());
            }
            self.wait_for_cinematic()?;
            for t in 0..24 {
                self.tick(Controls {
                    jump: input.jump && t == 0,
                    ..input
                })?;
                if self.interactions.scripted() {
                    if yield_scene {
                        return Ok(());
                    }
                    self.wait_for_cinematic()?;
                    return self.navigate_replan(goal, 8, yield_scene);
                }
            }
            if self.exited() {
                return Ok(());
            }
            println!("  PATH {:?}", self.player.feet);
            // The potion contact takes control before the last planned step.
            // Accept that authored hand-off before treating it as displacement.
            if self
                .interactions
                .school
                .as_ref()
                .is_some_and(|s| s.cinematic())
                && self.player.feet.distance(goal) < 80.
            {
                return Ok(());
            }
            if self.player.feet.distance(expected) > 8. {
                anyhow::ensure!(
                    replans > 0 && self.stats.alive(),
                    "Live route could not recover from displacement"
                );
                println!("  Replan after live displacement");
                return self.navigate_replan(goal, replans - 1, yield_scene);
            }
        }
        // A reached exit trigger can take over Alice before the last planned step.
        if self
            .interactions
            .school
            .as_ref()
            .is_some_and(|s| s.cinematic())
            && self.player.feet.distance(goal) < 80.
        {
            return Ok(());
        }
        if self.player.feet.distance(goal) >= tolerance + 2. && replans > 0 {
            // Small live displacements can stay below the per-step threshold yet
            // leave the final position just outside the requested tolerance.
            // Replan from the real position instead of treating a reachable goal
            // as a failed route. The bounded budget still rejects real stalls.
            return self.navigate_replan(goal, replans - 1, yield_scene);
        }
        anyhow::ensure!(
            self.player.feet.distance(goal) < tolerance + 2.,
            "Planned route diverged"
        );
        Ok(())
    }
    pub fn tick(&mut self, mut input: Controls) -> Result<()> {
        self.interactions.filter_level_controls(&mut input);
        if self.exited() {
            return Ok(());
        }
        let world_dt = self.stats.powers.world_dt(FIXED_DT);
        let mover_dt = if self
            .interactions
            .levels
            .iter()
            .any(|s| s.ctl.ignores_watch())
        {
            FIXED_DT
        } else {
            world_dt
        };
        self.interactions
            .prepare_player(&mut self.stats, &mut self.player);
        let mut loot_before = self.interactions.loot_sources();
        loot_before.extend(self.guard_sources());
        if let Some(n) = &self.native_cast { loot_before.extend(n.loot_sources()); }
        input.use_pressed |= std::mem::take(&mut self.use_pressed);
        if self.skip_cinematics {
            self.interactions.skip_cinematic(
                &self.map,
                &mut self.world,
                &mut self.player,
                &mut self.story,
            )?;
        }
        if let Some(s) = &mut self.interactions.school {
            s.sync_inventory(&mut self.stats);
        }
        let aim = input.wish.extend(0.);
        let use_door = self
            .interactions
            .prompt(&self.world, self.player.eye(), aim)
            == Some("E  open door");
        let shared_rope = self.player.rope_prompt(&self.world).is_some();
        let updates = self.interactions.update(
            world_dt,
            &self.map,
            &mut self.world,
            &self.player,
            aim,
            use_door || (input.use_pressed && !shared_rope),
        )?;
        self.interactions.companions_for_movers(&self.native_cast.as_ref().map_or_else(Vec::new, |n| n.companions()));
        self.interactions.actors_for_movers(&self.native_cast.as_ref().map_or_else(Vec::new, |n| n.targets()));
        self.interactions
            .advance_school(mover_dt, &self.map, &mut self.world, &mut self.player)?;
        let before = self.player.feet;
        let landings = self.player.landings;
        let jumps = self.player.jumps;
        let controlled = self
            .interactions
            .pandemonium
            .as_mut()
            .is_some_and(|p| p.control(FIXED_DT, &self.world, &mut self.player, input));
        if !controlled
            && !self.interactions.scripted()
            && !self
                .interactions
                .duchess
                .as_ref()
                .is_some_and(|d| d.controlled())
            && !self
                .interactions
                .school
                .as_ref()
                .is_some_and(|s| s.cinematic())
            && !self.interactions.levels_controlled()
        {
            self.stats.prepare_player(&mut self.player);
            self.player.tick(&self.world, input);
        }
        if self.player.liquid_damage > 0. {
            trace(|| format!("liquid t{} -{}", self.ticks, self.player.liquid_damage));
        }
        self.stats
            .damage(std::mem::take(&mut self.player.liquid_damage));
        self.jumps += self.player.jumps - jumps;
        if self.player.landings > landings {
            let lost = self.stats.fall(self.player.landing_speed);
            trace(|| format!("fall t{} -{lost} at {:?}", self.ticks, self.player.feet));
        }
        self.hints.sync(&self.interactions);
        self.hints.observe(before, self.player.feet);
        let mut e = self
            .interactions
            .triggers(FIXED_DT, before, self.player.feet);
        e.merge(updates);
        if e.damage > 0. {
            trace(|| {
                format!(
                    "trigger t{} -{} at {:?}",
                    self.ticks, e.damage, self.player.feet
                )
            });
        }
        self.stats.damage(e.damage);
        for event in e.story {
            self.story.trigger(&event);
        }
        if let Some((feet, _)) = e.teleport {
            anyhow::ensure!(self.world.body_clear(feet), "Authored teleport blocked");
            self.player = Player::new(feet);
            self.teleports += 1;
            println!("  TELEPORT {feet:?}");
        }
        self.interactions
            .sync_pickups(&self.stats, &mut self.pickups, &mut self.story);
        if self.interactions.prepare_story(&mut self.story) && self.hints.prepare_story(&self.story)
        {
            self.story.tick(FIXED_DT, self.ticks % 30 == 0);
        }
        self.interactions.sync_cinematic_story(&self.story);
        self.hints.sync(&self.interactions);
        self.hints.update(FIXED_DT, self.story.hint_active());
        for event in self.story.take_completed() {
            self.interactions.completed_dialogue(&event);
        }
        self.school2_step(world_dt);
        self.interactions.activate_enemies();
        if !self.story.busy()
            && !self
                .interactions
                .duchess
                .as_ref()
                .is_some_and(|d| d.cinematic())
            && !self.interactions.scripted()
        {
            if let Some(s) = self.interactions.encounters.as_mut() {
                s.notarget(
                    self.stats.invisible > 0.
                        || self
                            .interactions
                            .school
                            .as_ref()
                            .is_some_and(|s| s.cinematic()),
                );
                let f = s.update(world_dt, &self.world, self.player.eye());
                if f.damage > 0. {
                    trace(|| {
                        format!(
                            "enemy t{} -{} at {:?} sanity {}",
                            self.ticks,
                            f.damage,
                            self.player.feet,
                            self.stats.sanity()
                        )
                    });
                }
                self.stats.damage(f.damage);
                self.player.knockback(f.impulse);
                self.damage += f.damage;
            }
        }
        if let Some(d) = &mut self.interactions.duchess {
            if let Some(t) = d.target() {
                d.threatened(self.projectiles.iter().any(|p| p.threatens(&self.world, t)));
            }
            let f = d.update(
                world_dt,
                &self.world,
                &mut self.player,
                &mut self.stats,
                &mut self.story,
            );
            if f.damage > 0. {
                trace(|| {
                    format!(
                        "duchess t{} -{} at {:?}",
                        self.ticks, f.damage, self.player.feet
                    )
                });
            }
            self.stats.damage(f.damage);
            self.player.knockback(f.impulse);
            self.damage += f.damage;
        }
        if self.interactions.has_level_combat() {
            // Pool pushers and registered controllers use the same combat hook as the viewer.
            let dt = if !self.story.busy() && !self.interactions.scripted() {
                world_dt
            } else {
                0.
            };
            let notarget = self.stats.ignores_alice();
            let (world, projectiles) = (&self.world, &self.projectiles);
            let threatens =
                |t: &crate::combat::Target| projectiles.iter().any(|p| p.threatens(world, *t));
            let f = self.interactions.levels_step(&mut crate::level::Combat {
                dt,
                world,
                player: &mut self.player,
                stats: &mut self.stats,
                story: &mut self.story,
                notarget,
                summon: None,
                threatens: &threatens,
            });
            if f.damage > 0. {
                trace(|| {
                    format!(
                        "level t{} -{} at {:?}",
                        self.ticks, f.damage, self.player.feet
                    )
                });
            }
            self.stats.damage(f.damage);
            self.player.knockback(f.impulse);
            self.damage += f.damage;
        }
        self.guard_step(world_dt);
        self.native_step(world_dt);
        let mut targets = self
            .interactions
            .encounters
            .as_ref()
            .map_or(Vec::new(), |s| s.targets());
        targets.extend(self.interactions.duchess.as_ref().and_then(|d| d.target()));
        targets.extend(self.interactions.shot_targets());
        targets.extend(self.interactions.levels_targets());
        targets.extend(self.school2_targets());
        targets.extend(self.guard_targets());
        if let Some(n) = &self.native_cast { targets.extend(n.targets()); }
        let ctx = crate::combat::Context {
            world: &self.world,
            targets: &targets,
        };
        let nearest = targets
            .iter()
            .filter(|t| {
                self.aim_at
                    .map_or_else(|| self.is_foe(t.id), |id| t.id == id)
            })
            .filter(|t| {
                crate::combat::contact(&ctx, self.player.eye(), t.center, 1.)
                    .is_some_and(|(id, _)| id == t.id)
            })
            .min_by(|a, b| {
                a.center
                    .distance_squared(self.player.eye())
                    .total_cmp(&b.center.distance_squared(self.player.eye()))
            });
        // The aim is sampled at release, so no allowance for Alice's own movement is needed.
        let aim = nearest.map_or(Vec3::Y, |t| {
            (t.center - self.player.eye()).normalize_or_zero()
        });
        let tactics = self.tactics
            && (self.aim_at.is_none()
                || self.aim_at.is_some_and(|id| {
                    crate::level::hit_owner(&self.interactions.levels, id).is_some()
                }));
        let mut plan = if tactics {
            self.fight.observe(&targets, world_dt);
            let mut foes = self.engaged(&targets);
            if let Some(id) = self.aim_at {
                foes.retain(|t| t.id == id);
            }
            let cards = !self.conserve_will
                && self.stats.copies(1) > 0
                && self.stats.will() >= self.will_reserve + fight::CARD_COST;
            Some(fight::plan(
                &ctx,
                self.player.eye(),
                &foes,
                &self.fight,
                cards,
            ))
        } else {
            None
        };
        if self.ice_stream && self.stats.copies(4) > 0 && self.stats.will() >= self.will_reserve + 0.75
            && self.player.immersion.level == 0 {
            if let Some(p) = plan.as_mut() {
                if let Some((t, point)) = self.engaged(&targets).iter()
                    .filter(|t| self.aim_at.is_none_or(|id| id == t.id))
                    .filter_map(|t| fight::aim_point(&ctx, self.player.eye(), t).map(|point| (*t,point)))
                    .filter(|(_,point)| point.distance(self.player.eye()) < 360.)
                    .min_by(|a,b| a.1.distance_squared(self.player.eye()).total_cmp(&b.1.distance_squared(self.player.eye()))) {
                    p.target = Some(t.id);
                    p.weapon = 4;
                    p.alternate = false;
                    p.aim = (point-self.player.eye()).normalize_or_zero();
                }
            }
        }
        if let (Some(weapon), Some(p)) = (self.heavy_weapon, plan.as_mut()) {
            if self.stats.copies(weapon) > 0
                && (self.heavy.charging()
                    || self.stats.will() > if weapon == 7 { 21. } else { 99. })
            {
                if let Some((target, point)) = self
                    .engaged(&targets)
                    .iter()
                    .filter(|t| self.aim_at.is_none_or(|id| id == t.id))
                    .filter_map(|t| {
                        fight::aim_point(&ctx, self.player.eye(), t).map(|point| (*t, point))
                    })
                    .filter(|(_, point)| point.distance(self.player.eye()) <= 2000.)
                    .min_by(|a, b| {
                        a.1.distance_squared(self.player.eye())
                            .total_cmp(&b.1.distance_squared(self.player.eye()))
                    })
                {
                    p.target = Some(target.id);
                    p.weapon = weapon;
                    p.alternate = false;
                    p.aim = (point - self.player.eye()).normalize_or_zero();
                }
            }
        }
        if let Some(p) = plan.as_ref().filter(|p| p.target.is_some()) {
            self.fight.last_aim = p.aim;
            self.fight.weapon = p.weapon;
        }
        if !self.story.busy()
            && !self
                .interactions
                .duchess
                .as_ref()
                .is_some_and(|d| d.cinematic())
            && !self.interactions.scripted()
        {
            let armed = self.stats.copies(0) > 0;
            // Every attack waits for its toy to be in the hand, so a click is only issued when
            // the attack can start now; the toy and mode are chosen with the target.
            let selected = if self.watch_input {
                9
            } else if !armed {
                crate::weapons::UNARMED
            } else if tactics {
                self.fight.weapon
            } else {
                0
            };
            let (mut click, aim) = match &plan {
                Some(p) => (
                    p.target
                        .filter(|_| {
                            armed
                                && (self.actions.ready_to_attack(p.weapon)
                                    || (p.weapon == 7 && self.actions.selected == 7))
                        })
                        .map(|_| p.alternate),
                    p.aim,
                ),
                None => (nearest.map(|_| true), aim),
            };
            if self.watch_input {
                // Release the previous stream before holding the Watch. Keeping
                // primary held can sustain Ice/Staff while selection is queued.
                click = (self.actions.selected == 9 && self.stats.will() >= 1.).then_some(false);
            }
            let e = if self.player.script_motion > 0 || self.interactions.level_blocks_weapons() {
                self.actions.reset(&self.base);
                crate::weapons::Events::default()
            } else {
                self.actions.update_funded(
                    FIXED_DT,
                    crate::weapons::WeaponInput {
                        dice: 1,
                        selected,
                        click,
                        aim,
                        first_person: true,
                    },
                    &self.base,
                    &vec![true; self.base.len()],
                    true,
                    // Tactics pay for what they fire; the recorded drivers never did.
                    tactics.then_some(&mut self.stats),
                )
            };
            let mut hits = Vec::new();
            if e.fire && self.actions.selected < 7 {
                let released = self
                    .actions
                    .playing
                    .as_ref()
                    .filter(|p| matches!(p.action, crate::weapons::Action::Attack { .. }))
                    .map(|p| (p.alternate(), p.aim));
                if let Some((alternate, aim)) = released {
                    if self.tactics && !alternate && self.actions.selected == 4 {
                        hits.extend(crate::weapons::ice_primary(&ctx, self.player.eye(), aim));
                    } else if self.tactics && !alternate && self.actions.selected == 1 {
                        let eye = self.player.eye();
                        let target = crate::weapons::acquire_target(&ctx, eye, aim);
                        self.projectiles
                            .push(crate::weapons::Projectile::card(eye, aim, target));
                        self.cards += 1;
                        trace(|| format!("card t{} aim {aim:?} target {target:?}", self.ticks));
                    } else if alternate || !self.tactics {
                        self.projectiles
                            .push(crate::weapons::Projectile::thrown_blade(
                                self.player.eye(),
                                aim,
                            ));
                        self.shots += 1;
                        trace(|| {
                            format!(
                                "throw t{} aim {aim:?} at {:?}",
                                self.ticks, self.player.feet
                            )
                        });
                    } else {
                        hits = crate::weapons::blade_melee(&ctx, self.player.eye(), aim);
                        self.swings += 1;
                        trace(|| {
                            format!(
                                "swing t{} hits {} at {:?}",
                                self.ticks,
                                hits.len(),
                                self.player.feet
                            )
                        });
                    }
                }
            }
            if e.fire && matches!(self.actions.selected, 7 | 8) {
                self.shots += 1;
            }
            hits.extend(self.heavy.update(
                FIXED_DT,
                e,
                &mut self.actions,
                self.player.eye(),
                aim,
                crate::combat::Target {
                    id: crate::dice::ALICE,
                    center: self.player.feet + Vec3::Z * 28.,
                    half: vec3(15., 15., 28.),
                },
                &ctx,
                &mut self.stats,
            ));
            self.actions.finish_frame();
            for p in &mut self.projectiles {
                if let (Some(hit), _) = p.contact_step(FIXED_DT, &ctx) {
                    hits.push(hit);
                }
            }
            for mut hit in hits {
                if hit.id == crate::dice::ALICE {
                    let damage = hit.damage;
                    self.stats.damage(damage);
                    self.player.knockback(hit.knockback);
                    self.damage += damage;
                    continue;
                }
                hit.damage = self.stats.attack_damage(hit.damage);
                trace(|| format!("hit t{} id {} damage {}", self.ticks, hit.id, hit.damage));
                if hit.damage > 0. {
                    self.fight.landed(hit.id, self.ticks);
                }
                if hit.id == crate::duchess::ID {
                    self.interactions.duchess.as_mut().unwrap().hit(hit.damage);
                } else if self.interactions.hit_level(hit).is_some() {
                    // A registered controller owns this id's exact range, which the
                    // catch-all below would otherwise swallow.
                } else if hit.id >= crate::encounters::BASE {
                    self.interactions.encounters.as_mut().unwrap().hit(hit);
                } else if self.strike(hit) {
                    // School two's Boojums and guards, or a placed club guard.
                } else {
                    let e = self.interactions.shoot(hit);
                    if let Some(m) = e.message {
                        println!("  {m}");
                    }
                }
            }
            self.projectiles.retain(|p| p.age < 2.5);
        }
        let dice_before = self.stats.copies(6);
        for m in crate::inventory::collect(
            &mut self.stats,
            &self.pickups,
            self.player.feet,
            &self.world,
        ) {
            println!("  {m}");
        }
        if let Some(s) = &mut self.interactions.school {
            s.sync_inventory(&mut self.stats);
            let e = s.collect_secret(&mut self.stats, self.player.feet, &self.world);
            for event in e.story {
                self.story.trigger(&event);
            }
            if let Some(m) = e.message {
                println!("  {m}");
            }
        }
        if self.stats.copies(6) > dice_before {
            // The conversation completion releases the School2 dice ambush.
            self.story.trigger("dice_cat");
        }
        let mut loot_after = self.interactions.loot_sources();
        loot_after.extend(self.guard_sources());
        if let Some(n) = &self.native_cast { loot_after.extend(n.loot_sources()); }
        crate::loot::tick(
            &mut self.stats,
            &self.visit,
            world_dt,
            &loot_before,
            &loot_after,
            self.player.feet,
            &self.world,
        );
        self.stats.update(FIXED_DT);

        if let Some(m) = e.message {
            println!("  {m}");
        }
        if let Some(exit) = e.transition {
            self.story.defer_exit(exit);
        }
        if let Some(exit) = self.interactions.take_story_exit(&mut self.story) {
            self.audit.transitions += 1;
            self.transition = Some(exit);
        }
        self.environment_clock += world_dt;
        self.pickup_clock += FIXED_DT;
        self.audit_tick();
        self.ticks += 1;
        self.freeze_check();
        anyhow::ensure!(
            self.interactions.scripted()
                || self
                    .interactions
                    .school
                    .as_ref()
                    .is_some_and(|s| s.cinematic())
                || self.world.body_clear(self.player.feet),
            "Route body overlap at {:?}",
            self.player.feet
        );
        anyhow::ensure!(self.stats.alive(), "Route died at {:?}", self.player.feet);
        Ok(())
    }
    pub fn hold_blade(&mut self, hold: bool) {
        self.fight.hold_blade = hold;
        if hold { self.fight.weapon = 0; }
    }
    /// Equip the earned Watch and hold primary through its ordinary release
    /// frame. Actions owns the cost, recharge and stopped-world clock.
    pub fn use_watch(&mut self) -> Result<()> {
        anyhow::ensure!(self.stats.copies(9) > 0
            && self.stats.powers.recharge <= 0., "Watch is unavailable");
        self.tactics = true;
        self.watch_input = true;
        let result = (|| {
            for _ in 0..2400 {
                self.tick(Controls::default())?;
                if self.stats.powers.stopped > 0. { return Ok(()); }
            }
            anyhow::bail!("Watch did not release");
        })();
        self.watch_input = false;
        result
    }
    pub fn walk(&mut self, goal: Vec3, jump: bool) -> Result<()> {
        // With tactics on, a fight can leave Alice at a corner of a straight leg that was
        // written from a slightly different spot; plan the way once she stops making progress.
        let mut best = (f32::MAX, 0);
        for t in 0..3600 {
            if self.exited() {
                return Ok(());
            }
            let remaining = (goal - self.player.feet).truncate().length();
            if remaining < best.0 - 2. {
                best = (remaining, t);
            } else if self.tactics && t - best.1 > 240 {
                return self.navigate(goal);
            }
            if self
                .interactions
                .pandemonium
                .as_ref()
                .is_some_and(|p| p.state.cart == crate::pandemonium::Cart::Boarding)
                && self.player.feet.distance(goal) < 150.
            {
                return Ok(()); // The reached cart contact now owns Alice's movement.
            }
            let delta = goal - self.player.feet;
            if delta.truncate().length() < 7. && delta.z.abs() < 35. {
                println!(
                    "  WALK {:?} sanity {}",
                    self.player.feet,
                    self.stats.sanity()
                );
                return Ok(());
            }
            self.tick(Controls {
                wish: delta.truncate().normalize_or_zero(),
                jump: jump && t % 200 == 0,
                run: jump,
                ..Default::default()
            })?;
        }
        anyhow::bail!(
            "Route blocked toward {goal:?} at {:?}, grounded={}",
            self.player.feet,
            self.player.grounded
        )
    }
    pub fn walk_xy(&mut self, goal: Vec2) -> Result<()> {
        for _ in 0..2400 {
            if self.exited() {
                return Ok(());
            }
            let d = goal - self.player.feet.truncate();
            if d.length() < 8. && self.player.grounded {
                println!("  WALK XY {:?}", self.player.feet);
                return Ok(());
            }
            self.tick(Controls {
                wish: d.normalize_or_zero(),
                ..Default::default()
            })?;
        }
        anyhow::bail!("Walk blocked toward {goal:?} at {:?}", self.player.feet)
    }
    pub fn wait(&mut self, seconds: f32) -> Result<()> {
        for _ in 0..(seconds / FIXED_DT) as usize {
            if self.exited() {
                break;
            }
            self.tick(Controls::default())?;
        }
        println!("  WAIT {:?}", self.player.feet);
        Ok(())
    }
    pub fn shoot_switch(&mut self, thread: &str) -> Result<()> {
        let t = self
            .interactions
            .switch_target(thread)
            .context("Switch missing or already used")?;
        for _ in 0..1200 {
            if self.exited() {
                return Ok(());
            }
            if self.tactics {
                // Whatever is attacking comes first; the switch is shot in between.
                self.aim_at = None;
                self.clear(700.)?;
            }
            self.aim_at = Some(t.id);
            self.tick(Controls::default())?;
            if self.interactions.switch_target(thread).is_none() {
                self.aim_at = None;
                return Ok(());
            }
        }
        anyhow::bail!(
            "Cannot shoot {thread} from {:?}, target {:?}",
            self.player.feet,
            t.center
        )
    }
    pub fn ride_to(&mut self, height: f32) -> Result<()> {
        for _ in 0..3600 {
            if self.exited() {
                return Ok(());
            }
            self.tick(Controls::default())?;
            if self.player.grounded && (self.player.feet.z - height).abs() < 0.1 {
                println!("  RIDE {:?}", self.player.feet);
                return Ok(());
            }
        }
        anyhow::bail!("Missed lift to {height} at {:?}", self.player.feet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 96-unit room with walls on every side. The state space is small, and a goal 70 units east
    /// of its centre lies beyond the east wall, out of reach at the first tolerance and inside the
    /// wider one.
    fn walled_room() -> (World, Player) {
        let world = World::fixture(&[
            (vec3(-48., -48., -32.), vec3(48., 48., 0.)),
            (vec3(48., -56., -32.), vec3(56., 56., 400.)),
            (vec3(-56., -56., -32.), vec3(-48., 56., 400.)),
            (vec3(-56., 48., -32.), vec3(56., 56., 400.)),
            (vec3(-56., -56., -32.), vec3(56., -48., 400.)),
            (vec3(-56., -56., 400.), vec3(56., 56., 408.)),
        ]);
        let mut player = Player::new(Vec3::ZERO);
        for _ in 0..30 {
            player.tick(&world, Controls::default());
        }
        assert!(player.grounded, "the room's floor holds Alice up");
        (world, player)
    }

    #[test]
    fn a_goal_between_two_steps_plans_at_the_wider_tolerance_only() {
        let (world, player) = walled_room();
        // Alice's body is 30 units wide, so the nearest she can stand to the east wall is 33.
        let goal = vec3(70., 0., 0.);
        assert!(Route::plan(&world, &player, goal, NAV_TOLERANCE).is_err());
        let path = Route::plan(&world, &player, goal, NAV_TOLERANCE_WIDE).unwrap();
        let (_, end) = path.last().copied().unwrap();
        assert!(
            end.distance(goal) < NAV_TOLERANCE_WIDE && end.distance(goal) > NAV_TOLERANCE,
            "the plan ends {} from the goal",
            end.distance(goal)
        );
    }
    #[test]
    fn a_reachable_goal_keeps_its_plan_at_the_first_tolerance() {
        let (world, player) = walled_room();
        let goal = vec3(20., 20., 0.);
        let path = Route::plan(&world, &player, goal, NAV_TOLERANCE).unwrap();
        assert!(path.last().unwrap().1.distance(goal) < NAV_TOLERANCE);
        // The search is deterministic: the same start and goal give the same plan.
        let again = Route::plan(&world, &player, goal, NAV_TOLERANCE).unwrap();
        assert_eq!(
            path.iter().map(|(_, at)| *at).collect::<Vec<_>>(),
            again.iter().map(|(_, at)| *at).collect::<Vec<_>>()
        );
    }
}

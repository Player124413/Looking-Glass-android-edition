//! The level-controller contract (F1.1). A visit that is not owned by one of the typed legacy
//! controllers plugs into the game as one `LevelController`, registered by a single line in
//! `levels/mod.rs`, instead of adding arms to the chains in `interaction.rs`, `viewer.rs` and
//! `route.rs`.
//!
//! Every method has a no-op default that mirrors the duck-typed API of today's controllers
//! (`school`, `pool`, `pandemonium`, `duchess`, ...), so a controller overrides only what its
//! visit needs. The generic hooks that consult controllers are always placed *after* the legacy
//! chains, which keeps existing precedence and every existing map's event program unchanged
//! (`docs/LEVEL_REGISTRY.md`).
//!
//! The Keep arrival uses the mover contract. Other optional contract types remain
//! available for the later full visit controllers.
#![allow(dead_code)]
pub mod exit;
pub mod scene;
pub mod spec;
use crate::{
    bsp::Bsp,
    cinematic::{ActorPose, Camera},
    collision::{Collider, Liquid, World},
    combat::{Feedback, Hit, Target},
    entity::{Id, Registry},
    environment::Atmosphere,
    event::{Condition, Effect, Facts, Rule},
    interaction::Events,
    inventory::Stats,
    levels::Registration,
    loot::Source,
    movement::Player,
    story::Story,
};
use anyhow::Result;
use macroquad::prelude::{Color, Quat, Vec3};
use std::{any::Any, collections::BTreeSet};

/// The width of every visit's hit-ID range: `[target_base, target_base + HIT_RANGE)`.
pub const HIT_RANGE: usize = 100_000;

/// Where Alice restarts when a save that predates a registered controller joins its visit
/// (F2 step 6). Either way the entrance is used when the controller rejects the saved
/// position (`validate_player`) or the body is not clear there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Respawn {
    /// Keep the saved position when it is valid.
    #[default]
    IfInvalid,
    /// The controller changes the visit so much (movers, a flooded floor, a scene) that a
    /// position saved before it existed cannot be trusted: always restart at the entrance.
    Always,
}

/// What a controller declares for the generic upgrade of a save that predates it
/// (`docs/SAVES.md`, F2). The default declares nothing and is right for a controller that only
/// adds its own rules: the gated trigger keys are derived from `gate()`, formerly pending
/// script triggers are rearmed, and rules keyed on triggers the old build already handled are
/// counted as run.
#[derive(Clone, Debug, Default)]
pub struct Upgrade {
    pub respawn: Respawn,
    /// Extra existing rule keys (`trigger/<id>`) whose condition this controller changes beyond
    /// what its `gate()` reports. Usually empty.
    pub gated: Vec<String>,
    /// Script threads whose triggers must run again although the old build handled them
    /// (a dialogue or sky sequence that now has world consequences). Triggers the old build
    /// left pending are rearmed without being listed.
    pub rearm: Vec<String>,
    /// Own rule keys (`<id>/...`) that count as already run: ambushes whose actors an old
    /// controller spawned, rewards already granted. Applied even to a rearmed trigger, so the
    /// trigger runs again but these rules never replay.
    pub consumed: Vec<String>,
}

/// What kind of world trigger a gate or rule is asked about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerClass {
    Dialogue,
    Exit,
    Teleport,
    Hurt,
    Fall,
    Script,
}

/// A read-only view of one trigger volume for `gate` and `rules`.
#[derive(Clone, Copy, Debug)]
pub struct TriggerInfo<'a> {
    pub id: Id,
    pub name: &'a str,
    pub class: TriggerClass,
    /// The authored script thread of a `Script` trigger; empty for every other class.
    pub thread: &'a str,
    /// The destination map of an `Exit` trigger.
    pub exit: Option<&'a str>,
    /// The authored `target` key.
    pub target: Option<&'a str>,
}

/// What a controller may read while it contributes event rules.
pub struct RuleContext<'a> {
    pub map: &'a Bsp,
    pub registry: &'a Registry,
    /// Entities that accept activation messages (encounter identities, relays, registry receivers).
    pub receivers: &'a BTreeSet<Id>,
    pub triggers: &'a [TriggerInfo<'a>],
}

/// A friendly actor keeps its own NPC save identity while contributing machinery occupancy.
#[derive(Clone)]
pub struct CompanionContact {
    pub name: String,
    pub feet: Vec3,
    pub half: Vec3,
    pub visible: bool,
    pub holding: bool,
}

/// Everything a controller's combat step may read and change for one frame. The viewer and the
/// headless route both build it and call `step_controllers`, so a route proves the viewer.
pub struct Combat<'a> {
    /// Simulated seconds. Zero while paused, in a scene, in conversation or when Alice is dead.
    pub dt: f32,
    pub world: &'a World,
    pub player: &'a mut Player,
    pub stats: &'a mut Stats,
    pub story: &'a mut Story,
    /// Enemies ignore Alice (Darkened Looking Glass, `notarget`).
    pub notarget: bool,
    /// Alice's Demon Dice ally, when one is summoned.
    pub summon: Option<Target>,
    /// Whether one of Alice's own projectiles is about to hit the target.
    pub threatens: &'a dyn Fn(&Target) -> bool,
}

/// A visual actor visible only in the current map's planar mirror.
#[derive(Clone, Copy)]
pub struct ReflectionActor {
    pub pose: crate::skeletal::Transform, pub time: f32, pub alpha: f32,
}

pub trait LevelController: Any {
    /// The Appendix F id (`garden1`, `wforest-return`). Rule keys are `<id>/...`, fact keys `<id>....`
    fn id(&self) -> &'static str;

    // Events and gates.

    /// Derived facts the rules read (recomputed, never saved).
    fn facts(&self) -> Facts {
        Facts::default()
    }
    /// Writable puzzle flags and counters the rules may set (saved with the shared event state).
    /// Authored cast spawns released by persistent level machinery.
    fn active_npcs(&self) -> Vec<usize> { Vec::new() }
    fn initial(&self) -> Facts {
        Facts::default()
    }
    /// The activation condition of a trigger, or `None` to leave it ungated. Consulted only
    /// on visits no legacy controller owns.
    fn gate(&self, _trigger: &TriggerInfo<'_>) -> Option<Condition> {
        None
    }
    fn shootable_thread(&self, _thread: &str) -> bool { false }
    /// A puzzle may rearm the same shot volume after a wrong answer. Its saved gate owns reuse.
    fn repeatable_shot(&self, _thread: &str) -> bool { false }
    /// A scene-owned physical exit shares the same transaction as its scripted exit.
    fn exit_contact(&mut self, _destination: &(String, Option<String>)) -> Option<Events> { None }
    /// Reviewed landing adjustment for a supplied marker embedded in its portal graphic.
    fn teleport_destination(&self, _destination: Vec3) -> Option<Vec3> { None }
    fn sky_origin(&self) -> Option<Vec3> { None }
    /// Extra brush poses drawn only in a mirror, and ordinary poses omitted from it.
    fn reflection(&self) -> (Vec<(usize, Vec3, macroquad::prelude::Quat)>, Vec<usize>) { (vec![], vec![]) }
    fn reflection_actor(&self) -> Option<ReflectionActor> { None }
    /// Entities that accept activation messages from `Action::Send`.
    fn receivers(&self, _registry: &Registry) -> Vec<Id> {
        Vec::new()
    }
    /// Additional event rules. Every key must start with `<id>/`.
    fn rules(&self, _ctx: &RuleContext<'_>) -> Vec<Rule> {
        Vec::new()
    }
    /// An output the event runtime emitted (`Door`, `Activate`, ...), after the legacy handlers.
    fn output(&mut self, _effect: &Effect) -> Option<Events> {
        None
    }
    /// A script thread a trigger reached. Unknown threads stay pending: return `None`.
    fn event(&mut self, _thread: &str) -> Option<Events> {
        None
    }
    /// A dialogue finished. The controller returns its own consequences.
    fn dialogue_complete(&mut self, _name: &str) -> Events {
        Events::default()
    }

    // Per-frame simulation.

    /// Apply map-owned arrival grants and breath contacts before ordinary movement.
    fn prepare_player(&mut self, _stats: &mut Stats, _player: &mut Player) {}
    /// Authored disguises constrain ordinary input without taking over the camera.
    fn filter_controls(&self, _controls: &mut crate::movement::Controls) {}
    /// Bound pickups use their moving support's current world position.
    fn pickup_origin(&self, _id: &str) -> Option<Vec3> { None }
    /// Persistent path cues release existing residents without creating a second cast.
    fn patrols(&self) -> &'static [&'static str] { &[] }
    fn hides_player(&self) -> bool { false }
    fn blocks_weapons(&self) -> bool { false }
    /// Boss arenas may prohibit Demon Dice allies while keeping ordinary toys available.
    fn dismiss_summons(&self) -> bool { false }
    fn quake_offset(&self) -> Vec3 { Vec3::ZERO }
    /// A visit may hold a shared door locked; its collider still belongs to the door system.
    fn door_locked(&self, _id: Id) -> Option<bool> { None }
    /// A durable, map-authored checkpoint; acknowledge only after a successful write.
    fn checkpoint_requested(&self) -> bool {
        false
    }
    fn checkpoint_written(&mut self) {}
    /// Finale completion is separate from a map exit.
    fn ending_ready(&self) -> bool {
        false
    }
    fn fog_distance(&self) -> Option<f32> {
        None
    }
    fn music_mood(&self) -> Option<&'static str> {
        None
    }
    /// Active native actor bounds used by automatic machinery contacts.
    fn actor_contacts(&mut self, _actors: &[Target]) {}
    fn companion_contacts(&mut self, _actors: &[CompanionContact]) {}
    fn ignores_watch(&self) -> bool {
        false
    }
    /// A controller-less save joined this owner; preserve already-carried resources.
    fn upgraded(&mut self) {}
    /// Trigger history from a controller-less save, before pending rules are rearmed.
    fn upgrade_triggers(&mut self, _ids: &[Id]) {}
    /// Current-visit migration may classify the saved position before world collision is synced.
    fn restore_position(&mut self, _player: &Player, _map: &Bsp) -> Result<()> { Ok(()) }
    /// A scene may use an authored atmosphere temporarily, without changing world time.
    fn scene_fog(&self) -> Option<macroquad::prelude::Vec4> { None }
    /// Commit an existing presentation thread through the same fog/sky owner as a trigger.
    fn take_presentation_event(&mut self) -> Option<&'static str> { None }
    /// Authored emitter visibility and binding; server breath points are independent.
    fn particles(&self, _steam: &mut crate::particles::Steam) {}
    /// Animated lights owned by this visit, selected with ordinary world lights.
    fn lights(&self) -> Vec<crate::lighting::Light> { Vec::new() }

    /// Saved scene ownership can enable or retire a placed movement volume.
    fn traversal(&self, _traversal: &mut crate::traversal::Traversal) {}

    /// Script-only exits and lever presses; keep their latches in the saved state.
    fn update(
        &mut self,
        _world: &mut World,
        _player: &Player,
        _aim: Vec3,
        _use_pressed: bool,
    ) -> Events {
        Events::default()
    }
    /// A committed exit may retry after the destination failed to load.
    fn transition_failed(&mut self, _exit: &(String, Option<String>)) {}
    fn prompt(&self, _world: &World, _eye: Vec3, _aim: Vec3) -> Option<&'static str> {
        None
    }
    /// Move movers and riders. A no-op when `dt <= 0`; clamp with `dt.min(0.1)`; never embed Alice.
    fn advance(
        &mut self,
        _dt: f32,
        _map: &Bsp,
        _world: &mut World,
        _player: &mut Player,
        _fixed: &[Collider],
    ) -> Result<()> {
        Ok(())
    }
    /// The pose of a trigger volume that rides a mover.
    fn trigger_pose(&self, _name: &str, _base: Vec3) -> Option<(Vec3, Quat)> {
        None
    }

    // World shape. One pose function should feed both `transforms` and `colliders`.

    fn transforms(&self) -> Vec<(usize, Vec3, Quat)> {
        Vec::new()
    }
    fn colliders(&self) -> Vec<Collider> {
        Vec::new()
    }
    /// Optional visible, passable scenery that still occludes the follow camera.
    fn camera_colliders(&self) -> Vec<Collider> {
        Vec::new()
    }
    /// A permanently settled subset of colliders, eligible for ledge hanging.
    fn settled_supports(&self) -> Vec<Collider> { Vec::new() }
    /// Brush-entity and moving liquid volumes.
    fn liquids(&self) -> Vec<Liquid> {
        Vec::new()
    }

    // Scenes.

    /// Authored summon window, derived from this owner's saved state.
    fn allow_cheshire(&self) -> bool {
        true
    }

    /// A scene owns the camera and the controls.
    fn scripted(&self) -> bool {
        false
    }
    /// The controller owns Alice's movement (a ride, a grab, a disguise).
    fn controlled(&self) -> bool {
        false
    }
    /// Recovery is refused while true. Defaults to `scripted`.
    fn in_transport(&self) -> bool {
        self.scripted()
    }
    /// The scene the skip button applies to.
    fn scene_id(&self) -> Option<&'static str> {
        None
    }
    fn camera(&self, _world: &World) -> Option<Camera> {
        None
    }
    /// Authored cinematic field of view, converted to vertical radians for this viewport.
    fn scene_fovy(&self, _aspect: f32) -> Option<f32> { None }
    fn fade(&self) -> Option<(Color, f32)> {
        None
    }
    /// Begin the entry scene. Return true when this controller handled the entry.
    fn entry_story(&mut self, _story: &mut Story) -> bool {
        false
    }
    /// Prepare the story for this frame. Return false to hold the next line.
    fn prepare_story(&self, _story: &mut Story) -> bool {
        true
    }
    fn sync_story(&mut self, _story: &Story) {}
    /// Commit the current scene as if it had played. Returns whether a scene was skipped.
    fn skip(
        &mut self,
        _map: &Bsp,
        _world: &mut World,
        _player: &mut Player,
        _story: &mut Story,
    ) -> Result<bool> {
        Ok(false)
    }

    // Help and recovery.

    /// Independently written help text; never original dialogue.
    fn objective(&self) -> Option<String> {
        None
    }
    fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        normal
    }

    // Combat. Hit IDs stay inside the registration's `[target_base, target_base + HIT_RANGE)`.

    fn targets(&self) -> Vec<Target> {
        Vec::new()
    }
    /// Targets can take friendly fire without being enemies for an automated combat driver.
    fn hostile_target(&self, _id: usize) -> bool { true }
    fn melee_target(&self, _id: usize) -> bool { false }
    /// Apply a hit; the sound to play at the target, if any.
    fn hit(&mut self, _hit: Hit) -> Option<&'static str> {
        None
    }
    /// Alice's Demon Dice summon struck this target's id.
    fn provoke_summon(&mut self, _id: usize) {}
    fn combat(&mut self, _combat: &mut Combat<'_>) -> Feedback {
        Feedback::default()
    }
    fn loot_sources(&self) -> Vec<Source> {
        Vec::new()
    }

    // Audio and persistence.

    /// Loops and mover clocks; every clock key must be unique across the game.
    fn sound_state(
        &self,
        _loops: &mut Vec<crate::audio::LoopCue>,
        _clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
    }
    /// The controller's saved state. Stateless controllers may return `Null`.
    fn snapshot(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
    /// Validate exhaustively: finite and bounded clocks, consistent flags.
    fn restore(&mut self, _saved: &serde_json::Value, _map: &Bsp) -> Result<()> {
        Ok(())
    }
    fn validate_player(&self, _player: &Player) -> Result<()> {
        Ok(())
    }
    /// How this controller joins a visit whose save predates it (F2). Nothing to declare by
    /// default. See `levels::state` for the saved-state pattern that goes with it.
    fn upgrade(&self) -> Upgrade {
        Upgrade::default()
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl dyn LevelController {
    pub fn downcast_ref<T: LevelController>(&self) -> Option<&T> {
        self.as_any().downcast_ref()
    }
    pub fn downcast_mut<T: LevelController>(&mut self) -> Option<&mut T> {
        self.as_any_mut().downcast_mut()
    }
}

/// The drawing half of a visit. It owns GPU resources, so it lives in the viewer's `LevelArt`
/// group and is replaced on every level change. It downcasts its controller inside the module.
pub trait LevelArt: Any {
    fn draw(
        &mut self,
        _level: &dyn LevelController,
        _atmosphere: &Atmosphere,
        _camera: Vec3,
        _fullbright: bool,
    ) {
    }
    fn story_pose(&mut self, _story: &Story) {}
    /// Effects drawn with the particle pass, after the world.
    fn effects(&mut self, _level: &dyn LevelController, _camera: Vec3, _atmosphere: &Atmosphere) {}
    fn hud(&mut self, _level: &dyn LevelController) {}
    /// The pose Alice resumes from when a scene hands control back.
    fn handoff_pose(&self) -> Option<&ActorPose> {
        None
    }
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// A registered controller together with the registration that created it.
pub struct Slot {
    pub reg: &'static Registration,
    pub ctl: Box<dyn LevelController>,
}

impl Slot {
    /// The exact hit range this visit owns, when it publishes targets.
    pub fn hit_range(&self) -> Option<std::ops::Range<usize>> {
        self.reg.target_base.map(|b| b..b + HIT_RANGE)
    }
}

/// The controller whose exact hit range holds `id`. Ranges never overlap the legacy ranges
/// (`levels::reservations` proves it), and the viewer asks this before its `>= encounters::BASE`
/// catch-all, which would otherwise swallow every registry id.
pub fn hit_owner(slots: &[Slot], id: usize) -> Option<usize> {
    slots
        .iter()
        .position(|s| s.hit_range().is_some_and(|r| r.contains(&id)))
}

/// Targets a controller publishes, restricted to its own range so a stray id can never be
/// dispatched to (or stolen from) another owner.
pub fn targets(slots: &[Slot]) -> Vec<Target> {
    slots
        .iter()
        .flat_map(|s| {
            let range = s.hit_range();
            s.ctl
                .targets()
                .into_iter()
                .filter(move |t| range.as_ref().is_some_and(|r| r.contains(&t.id)))
        })
        .collect()
}

/// One combat step for every registered controller. The viewer and `Route::tick` both call this,
/// so a headless route keeps proving what the viewer does.
pub fn step_controllers(slots: &mut [Slot], combat: &mut Combat<'_>) -> Feedback {
    let mut total = Feedback::default();
    for slot in slots {
        let f = slot.ctl.combat(combat);
        total.damage += f.damage;
        total.impulse += f.impulse;
        total.summon_hits.extend(f.summon_hits);
        total.sounds.extend(f.sounds);
        total.spatial_sounds.extend(f.spatial_sounds);
        total.cue_sounds.extend(f.cue_sounds);
        total.will_drain += f.will_drain;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::{Entity, Registration};
    use macroquad::prelude::vec3;

    /// Publishes configurable targets and combat feedback, so aggregation can be observed.
    struct Fake {
        id: &'static str,
        targets: Vec<Target>,
        damage: f32,
        notarget_seen: bool,
    }
    impl LevelController for Fake {
        fn id(&self) -> &'static str {
            self.id
        }
        fn targets(&self) -> Vec<Target> {
            self.targets.clone()
        }
        fn combat(&mut self, c: &mut Combat<'_>) -> Feedback {
            self.notarget_seen = c.notarget;
            Feedback {
                damage: self.damage * c.dt,
                impulse: vec3(1., 0., 0.),
                sounds: vec!["sound/a.wav"],
                cue_sounds: vec![("sound/owned.wav".into(), Vec3::X)],
                will_drain: self.damage * c.dt,
                ..Default::default()
            }
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }
    fn fake_load(
        _: &mut crate::assets::Assets,
        _: &Bsp,
        _: &str,
        _: Option<&str>,
    ) -> Result<Box<dyn LevelController>> {
        unreachable!("the unit tests build slots directly")
    }
    const fn registration(id: &'static str, base: Option<usize>) -> Registration {
        Registration {
            id,
            applies: |_, _| false,
            load: fake_load,
            art: None,
            owns_submodel: |_: &str, _: &Entity| false,
            owns_npc: |_, _| false,
            target_base: base,
            story_beats: &[],
            checks: &[],
            save_cases: &[],
            visibility: &[],
        }
    }
    static POOL: Registration = registration("potears2", Some(6_900_000));
    static TEMPLE: Registration = registration("utemple", Some(7_100_000));
    static QUIET: Registration = registration("garden1", None);
    fn target(id: usize) -> Target {
        Target {
            id,
            center: Vec3::ZERO,
            half: Vec3::ONE,
        }
    }
    fn slot(reg: &'static Registration, ids: &[usize], damage: f32) -> Slot {
        Slot {
            reg,
            ctl: Box::new(Fake {
                id: reg.id,
                targets: ids.iter().map(|&i| target(i)).collect(),
                damage,
                notarget_seen: false,
            }),
        }
    }

    #[test]
    fn hit_ranges_are_exact_and_never_reach_legacy_ranges() {
        let slots = [
            slot(&POOL, &[], 0.),
            slot(&QUIET, &[], 0.),
            slot(&TEMPLE, &[], 0.),
        ];
        assert_eq!(hit_owner(&slots, 6_900_000), Some(0));
        assert_eq!(hit_owner(&slots, 6_900_000 + HIT_RANGE - 1), Some(0));
        assert_eq!(hit_owner(&slots, 6_900_000 + HIT_RANGE), None);
        assert_eq!(hit_owner(&slots, 6_899_999), None);
        assert_eq!(hit_owner(&slots, 7_100_042), Some(2));
        // A controller that publishes no targets owns no range.
        assert_eq!(hit_owner(&slots, 0), None);
        // Every legacy id is left to its legacy dispatch, in the viewer's order.
        for legacy in [
            crate::school2::ENEMY_BASE + 3,
            crate::interaction::SHOT_BASE + 9,
            crate::encounters::BASE + 40,
            crate::duchess::ID,
            crate::dice::SUMMON,
            crate::dice::ALICE,
            // Ice Wand walls: 800_000_000 + wall id (weapons/ice.rs WALL_BASE).
            800_000_005,
        ] {
            assert_eq!(hit_owner(&slots, legacy), None, "legacy id {legacy}");
        }
    }
    #[test]
    fn a_controller_can_only_publish_targets_inside_its_own_range() {
        let slots = [
            slot(&POOL, &[6_900_001, 7_100_001, 3_000_001], 0.),
            slot(&TEMPLE, &[7_100_007], 0.),
            slot(&QUIET, &[6_800_000], 0.),
        ];
        let ids = targets(&slots).iter().map(|t| t.id).collect::<Vec<_>>();
        assert_eq!(ids, [6_900_001, 7_100_007]);
    }
    #[test]
    fn combat_feedback_from_every_controller_is_summed_in_slot_order() {
        let mut slots = [slot(&POOL, &[], 10.), slot(&TEMPLE, &[], 5.)];
        let world = World::fixture(&[]);
        let mut player = Player::new(vec3(0., 0., 0.));
        let mut stats = Stats::default();
        let mut story = Story::default();
        let never = |_: &Target| false;
        let mut c = Combat {
            dt: 0.5,
            world: &world,
            player: &mut player,
            stats: &mut stats,
            story: &mut story,
            notarget: true,
            summon: None,
            threatens: &never,
        };
        let f = step_controllers(&mut slots, &mut c);
        assert_eq!(f.damage, 7.5);
        assert_eq!(f.impulse, vec3(2., 0., 0.));
        assert_eq!(f.sounds, ["sound/a.wav", "sound/a.wav"]);
        assert_eq!(f.cue_sounds.len(), 2);
        assert_eq!(f.will_drain, 7.5);
        assert!(slots
            .iter()
            .all(|s| s.ctl.downcast_ref::<Fake>().unwrap().notarget_seen));
        // Paused time produces no damage even though the controllers ran.
        c.dt = 0.;
        assert_eq!(step_controllers(&mut slots, &mut c).damage, 0.);
    }
    #[test]
    fn a_controller_that_overrides_nothing_is_inert() {
        let mut q = slot(&QUIET, &[], 0.);
        let c = &mut q.ctl;
        let trigger = TriggerInfo {
            id: Id(0),
            name: "t1",
            class: TriggerClass::Script,
            thread: "thread",
            exit: None,
            target: None,
        };
        assert!(c.gate(&trigger).is_none());
        assert!(c.event("thread").is_none());
        assert!(!c.scripted() && !c.controlled() && !c.in_transport());
        assert!(c.scene_id().is_none() && c.objective().is_none());
        assert_eq!(c.recovery_entry((Vec3::X, 2.)), (Vec3::X, 2.));
        assert!(c.snapshot().is_null());
        let upgrade = c.upgrade();
        assert_eq!(upgrade.respawn, Respawn::IfInvalid);
        assert!(
            upgrade.gated.is_empty() && upgrade.rearm.is_empty() && upgrade.consumed.is_empty()
        );
        assert!(c.facts().0.is_empty() && c.initial().0.is_empty());
        assert!(c.transforms().is_empty() && c.colliders().is_empty() && c.liquids().is_empty());
        let mut story = Story::default();
        assert!(!c.entry_story(&mut story));
        assert!(c.prepare_story(&mut story));
    }
}

# Shared entities and puzzle events — version 0.20

Version 0.22 uses these systems for six Pool of Tears Ladybug trigger groups. Separate one-shot rules activate resident patrols or send the second ambush actor an activation after 2.1 seconds. Save format 4 explicitly extends validated older Pool of Tears event programs: unchanged rules, entity flags, clocks, usage and queued events survive; passed new ambush rules are marked consumed without replay. The extension rejects changes to existing rules or fact/entity schemas. See [LADYBUGS.md](LADYBUGS.md).

This is an independently written Rust foundation for further level interactions. It reuses the existing playable schools to verify the interfaces; it does not translate or execute the original script VM. Controls remain unchanged: E interacts, H shows help, F5/F9 save/load, and Continue restores the last session.

## Identity and component ownership

`entity::Registry` assigns every BSP entity its ordinal `Id`, including unnamed entities. IDs belong to one map and campaign visit. The first and return visits retain separate runtime state. `targetname` and `spawntargetname` are group aliases: one name may resolve to several IDs, in map order, without duplicating an entity that has the same alias twice.

Doors, triggers and implemented encounter actors now persist those IDs alongside their component state. A definition signature and the existing game-data fingerprint prevent reordered or incompatible entities from silently receiving another object's state. IDs are stable for the same map data, not across arbitrary map revisions.

The shared runtime owns enable flags and accepted-rule activation counts. The component still owns movement, collision geometry, health, animation and quest choreography. Doors honor shared enable flags for interaction, movement, rendering and collision; encounter actors honor them for simulation, targeting and rendering; trigger contacts and the gym lever honor them before activation. Disabling an actor does not heal or respawn it. Other mover/NPC types need an explicit adapter before their shared flag affects gameplay.

## Rules, conditions and actions

`event::Runtime` is built from code-owned `Rule` definitions. A rule has a stable key, an input event, a condition, a one-shot flag, a cooldown and ordered actions. Supported inputs are entity events (`Activate`, `Touch`, `Use`, `Shot`), dialogue completion and named signals. Current physical triggers normalize successful contact/shot checks to `Activate`; the additional input kinds are available to new adapters.

Conditions combine `All`, `Any`, `Not`, boolean flags, minimum counter values and entity enable state. Persistent puzzle facts are declared when the runtime is built. Derived facts are declared separately and supplied by the school controllers, such as the number of books that have finished forming the bridge or the current ingredient stage. Facts are type-checked; derived facts cannot overwrite persistent ones. A blocked attempt does not consume a one-shot rule or start its cooldown.

Actions set persistent flags, add to counters, enable/disable entities, send immediate/delayed events, or emit a typed component effect. Effects currently activate encounters, operate doors, invoke reviewed trigger adapters and complete supported quest dialogue. Original target links automatically reach registered encounter receivers and plain relays only; door links require explicit rules so they cannot accidentally override paired-leaf opening direction or quest ownership.

For a new puzzle, declare its variables and register all receivers, then construct rules such as:

```rust
Rule {
    key: "puzzle/open_exit".into(),
    event: Event::Signal("puzzle/try_exit".into()),
    condition: Condition::All(vec![
        Condition::flag("puzzle.power"),
        Condition::AtLeast("puzzle.switches".into(), 3),
    ]),
    once: true,
    cooldown: 0.,
    actions: vec![Action::Send {
        event: Event::Entity(exit_id, Input::Activate),
        delay: 0.5,
    }],
}
```

Here `exit_id` must be a registered activation receiver, `puzzle.power` a declared flag and `puzzle.switches` a declared counter. Pass normal player input through the adapter's reach, line-of-sight and interaction checks before dispatch. A signal is a code-owned event, not an arbitrary console/script command.

Register bindings in `Interactions::configure_events`, expose any controller facts through `event_facts`, and handle a new effect explicitly in `apply_outputs`. Retain the controller's physical behavior and use shared conditions for its gate. Add a real-map interaction regression and save/restart continuation when introducing a new puzzle. General original script threads remain visibly pending unless supported by a reviewed adapter.

## Time, order and failure behavior

Rules execute in stable key order; actions retain their declared order. Events are ordered by due time, then insertion sequence. Delayed events use simulation time and survive saving with their remaining deadline. An `advance(0)` pause dispatches nothing, including events already due. Cached visits remain frozen.

Each dispatch/advance permits at most 512 event/action steps, 512 queued events and 512 pending outputs. Delays and rule cooldowns are bounded to one day of simulation time; counters use checked arithmetic. Definitions reject missing entities/handlers, unknown or mistyped facts, duplicate keys and invalid timings. A malformed event or relay cycle rolls back shared state, history, queue changes and outputs from that dispatch; it reports an error rather than freezing the game or leaking partial effects.

Controller-derived facts are sampled for a dispatch. Persistent facts changed by an earlier action are visible to subsequent rules in that dispatch. Component effects are applied after the shared transaction succeeds, so a condition that depends on a controller mutation observes it on the next dispatch. This is a bounded event layer, not a general transaction over every gameplay system.

## Existing-school bindings

| Interaction | Shared behavior exercised |
| --- | --- |
| First/return school encounter triggers | Entity group fan-out, enabled state, entry conditions, one-shot activation and retained enemy death |
| Theatre conversation completion | Saved one-shot completion, reinforcement events, explicit door unlock/open effects |
| Library, flying books and recipe | Composed progression flags and the four-completed-books counter gate |
| Gym lever | E interaction gate, saved one-shot usage and persistent counter |
| Elder Gnome ingredient conversations | Stage-gated completion, existing Boojum/reward effects, duplicate rejection |
| Potion and star exit | Existing distinct-reward condition, shared trigger dispatch, correct return entrance |
| Shootable secret picture | Existing weapon contact/health checks, shared activation gate and one-shot history |

The village's reviewed guard activation also uses the same dispatcher. Existing lifts, books, pendulums, ingredient animations and rewards remain in their dedicated controllers.

## Saved state and verification

Save format 2 stores shared entity state, flags/counters, usage timestamps, event clock, sequence, queue and pending typed effects. Definitions remain in compiled Rust. Restoration validates state against the current program before replacing it. Version 1 saves from v0.19 import completed interactions without emitting their effects; all cached visits migrate before the next version 2 save. See [SAVES.md](SAVES.md).

`--event-check` uses the local school BSPs for group activation, disabling/re-enabling across a save, retained enemy death, blocked early library/recipe events, the four-book gate, delayed theatre doors/reinforcements at 30/60/144 Hz, paused queues, stable IDs, legacy migration, gym lever use and quest dialogue/reward gates. Synthetic unit tests cover composed conditions, FIFO chains, cooldowns, malformed saves and cycle rollback without proprietary assets.

The normal school route, school secret detour, complete second-school quest and village route are independent headless progression regressions. Native `--save-check-write` followed by `--save-check-read` in a different process verifies reconstruction and continued simulation, including the queued theatre event. These are automated and staged checks, not a new claim of a native human campaign playthrough. Exact results and limitations are recorded in [VALIDATION.md](VALIDATION.md).


Pandemonium (v0.23) uses shared derived flags for cart completion, the key and return access; the cart completion signal, delayed airship guards and return spawners use ordinary persistent rules. Its v4-to-v5 migration permits condition changes only on explicitly selected gates, preserves unchanged actions/identities/timing, and rearms script contacts that had no implemented consequences. Unit coverage rejects unapproved rule/action changes.

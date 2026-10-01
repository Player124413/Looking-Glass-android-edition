# Pickup and particle audit — 2026-10-01

Scope: locally supplied, override-resolved BSP/TIKI data and the current main working tree. Identifiers, numeric properties and paraphrase only. This is a static campaign audit with focused runtime checks, not a claim of visual parity for every scene.

## Inventory coverage

- 392 `Item_*` placements in 33 maps, covering 20 classes. Every eligible class has a pickup binding. `--hud-check` now asserts presence/exclusion for every placed item on all four difficulties instead of only reporting supported counts.
- Normal exposes 315 generic pickups, Easy 339, Hard/Nightmare 294. These totals include the initial fortress2 Rage spawner and exclude the school-owned Glass. They are not raw BSP placement totals.
- 38 item/effect spawners reviewed for ownership. Existing owners control repeatable essence, encounter activation and quest rewards. Their timers, grants and save keys are unchanged.
- Return-only pickups retain the school and forest visit restrictions. Weapon components, school ingredients, Star, potion and breathing upgrade remain under their existing level/quest owners; the class count above does not count them as ordinary items.

## Fixed findings

| Finding | Change | Evidence / confidence |
|---|---|---|
| `wforest` return rewards could be collected but lacked their essence mesh | Draw the active `get_me1` / `get_me2` reward with the shared essence renderer, retaining its ten-second delay and alternating ownership | `src/levels/wforest/art.rs`; high |
| Controller-owned essence bypassed the three-pass material | Share the coloured reflection/filter/swirl renderer in `potears3`, `centipede2`, `rchess1`, `funhouse`, `hatter2`, `jlair1`, `jlair2`, `grounds1`, `qlair` | Level art and `src/jabberwock/art.rs`, `src/duchess.rs`, `src/loot_art.rs`; high |
| Placed, dropped and controller-owned essence omitted idle particles | Sample the supplied idle tags and emitter state from the existing presentation clock; separate birth positions for every item | `src/pickup_effects.rs`, `src/loot_art.rs`; high |
| Generic health/Will/essence pickups vanished without their authored collection particles | Notify cosmetic presentation after an actual accepted grant; handle both placed items and enemy drops | `src/inventory.rs`, `src/loot.rs`, `src/powerups.rs`, `src/viewer.rs`; high |
| New item particles initially used the older renderer's oversized billboard baseline | Size these sprites from their 16/32 pixel source images, preserving motion and authored emission numbers | `src/particles.rs`, `src/pickup_effects.rs`; source dimensions high, exact native screen parity unclaimed |
| Health/Will item lights and firefly lights were not loaded | Register their supplied light definitions with the existing world-light owner | `src/particles.rs`; high |
| Pickup emitters could survive the corresponding visit exclusion | Gate item emitters against available pickup IDs as well as collected IDs | `src/particles.rs`; high |

The collection effect is transient presentation, not a saved grant. All load and map-change paths clear pending flashes. Full resources, blocked reach and previously collected IDs emit nothing. A load does not infer a collection by comparing inventories. Existing collision/reach checks remain authoritative.

## Spawner ownership

| Maps | Supplied identifiers | Owner / treatment |
|---|---|---|
| `fortress2` | `spawn_ragebox` | Existing initial Rage pickup identity and cinematic owner retained |
| `fortress2` | five `hub_booj1` mana spawners | A spline binding is present; no live spawn activation found in the resolved scripts. Do not turn them into free pickups. Activation remains unverified |
| `potears3` | `mespawner1/2/3/5` | Duchess encounter and four independent cooldowns |
| `centipede2`, `rchess1`, `grounds1` | `get_me1/2/3` | Existing fight-owned alternating supply |
| `funhouse`, `hatter2`, `jlair2` | `help_me*` | Existing encounter gates and supply timers |
| `jlair1` | entities 4, 908 | Existing activation and two one-time rewards |
| `qlair` | `get_me1/2`, `help_me1/2/3/4` | Queen phase controls medium/large supply |
| `wforest` | `get_me1/2` | Return visit only; visible mesh restored |

## Effect coverage and remaining fidelity work

The private inventory lists 92 placed model definitions containing emitters or bursts, with all placements and candidate owner references. A reference match is not proof of complete runtime support. World steam/fire/waterfalls and altar effects use `particles::Steam`; actors and scene props use their owning animation/controller clocks. Effects marked off or attached to a scripted parent must not be enabled globally.

Remaining separately recorded limitations:

Follow-up: `MINOR_DETAILS_AUDIT.md` records the subsequent lantern pixie and firefly image-style fixes. The list below describes the original 09961D65 audit boundary.

- `lantern.tik` (31 placements): its enclosed pixie needs verified `constrain`/`minvel` motion. The lantern model is drawn, but that particle motion is not restored by this patch. Confidence: high on missing binding, medium on required native motion semantics.
- `hatter1:248` / `finger_bloodspurt1`: blood belongs to the machine's finger staging; the current owner has no matching attachment performance. Restore that staging before adding a permanently running blood emitter. Confidence: high on missing attachment binding.
- `potears3:31/32/33` / `firesparklies`: these grouped fireplace effects are separate from the restored Duchess attack/victory effects. Their authored activation and attachment need a focused owner audit. Confidence: medium; no free-running effect added.
- Firefly light presence is restored; its native image-driven light-style modulation is not decoded. Confidence: high.
- Generic collection bursts cover static items and standard enemy drops. Boss-specific repeatable resource grants keep their existing collection presentation. Idle particle/material fixes apply to their rewards. Confidence: high.
- Exact sprite orientation, native light-style curves, constrained particles and all actor-specific emitter commands are not certified by this pass. Existing Rage transformation effects and weapon effects remain under their existing owners.

## Verification

Private evidence: `private/item-fx-audit-20261001/`. The `pickups.json`, `spawners.json` and `effects.json` inventories record identifiers, placement numbers and source-reference candidates. Runtime logs, captures, compiled-source manifest and the final executable digest are recorded alongside them.

Required acceptance for this update: full unit suite; all-difficulty item coverage; loadout, event and 39-visit graph checks; affected owner checks; native essence materials/particles/occlusion checks; native affected-map rendering. These checks do not establish a completed 39-visit human playthrough.

Save version 12, event rule keys, visit order, hit-ID reservations, item amounts and combat gates are unchanged. No original assets or script/dialogue text are committed.

Final validation for build **09961D65**: 646 unit tests, 15 focused headless checks and ten affected-map native render suites passed. All eight health/Will/essence collection effects pass paused-frame, expiry and restoration-reset checks. A prior-release save loaded in the normal viewer and ran successfully; root launcher smoke check passed. Exact source/executable provenance is in `private/item-fx-audit-20261001/release.json`.

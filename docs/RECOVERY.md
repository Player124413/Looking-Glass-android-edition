# Footing, fatal falls and recovery

## Version 0.15.1: fallen shelves and tiny ground overlaps

The reported `skool1` crossing reproduced an inconsistency between the bounds of a rotated bookshelf and its swept player collision shape. The old rotated brush planes lacked world-axis and edge/axis bevels. A stationary query could call Alice's feet clear while a downward ground probe called that same start embedded. Sliding stopped, grounding failed, the airborne pose persisted, and Space could not jump.

Inline brush collision now derives vertices and edge directions from the supplied convex planes and builds the separating planes for the current rotation against an upright box. Each brush uses its own transformed bounds. A separate tipping-rider bug is fixed by finding support for Alice's upright body at the new shelf pose, then checking the carry against the surrounding world. An obstructed mover stops instead of embedding its rider.

For small numerical overlaps, movement can correct the feet by at most **one map unit**. The endpoint must hold the full body and the escape sweep must avoid newly encountered solids. It cannot teleport through another wall or a ceiling; larger obstructions still use the explicit recovery controls below. These changes apply to the shared collision/movement code, not only a special case at one school coordinate.

`--footing-check` stages 315 drop locations over the real fallen shelves at each of 30/60/144 Hz. Each must settle, permit walking away, survive eight walking directions without embedding, and jump. Both shelves must also complete their full tilt with a standing rider at each rate. `--shelf-preview --no-audio` opens an interactive staged crossing for inspection; it skips earlier school events and is not a saved playthrough. A new visit starts at the entrance; Continue restores a saved position.

## Earlier steam-floor repair (version 0.9)

**Version 0.14:** recovery also preserves school puzzle progress. Library lifts and books now move; the theatre floor still omits cinematic choreography. See [SCHOOL.md](SCHOOL.md).

The school steam area was missing its authored floor platforms. The remake now draws and collides with the eight `theatre_plat` objects on first entry, or the eight `skip_plat` objects at `skool1_start2`. It does not load both overlapping sets at once. Their geometry and textures still come from the user's archives.

This is bounded support for their initial positions. The supplied setup scripts make the corresponding set visible and solid; the cinematics move some platforms and later exchange sets. Those timed movements, actors and script events are not implemented. This repair removes the missing-floor trap, but it does not complete the school puzzle or prove every hole is escapable.

## Controls and recovery state

- **R:** return to footing before a jump/fall. If it is too close to the current position, obstructed, unsupported, wet or hazardous, try an earlier remembered departure, then the current map entrance.
- **Enter after death:** restore the complete retry checkpoint. R, Home and console `restart` also retry this way when Alice is dead, including during a scripted ride.
- **Home:** return directly to the current map entrance.

While alive, footing/entrance recovery restores Sanity and Will and clears velocity, liquid damage, climbing, animation/action effects and trigger contacts. It returns to walking/swimming, including underwater entrances. Inventory, collected pickups and door state are retained. History holds at most 32 departures and is cleared on map changes/teleports. If the entrance cannot hold Alice safely, recovery reports that instead of placing her inside a hazard. Repeated R presses can step back through earlier departures.

Death retry instead restores Alice and the whole saved world together: moving hazards, transports and guides, trigger history, enemies, dialogue, pickups, resources, puzzle gates and clocks. Each fresh visit or normal map transition establishes an in-memory entry checkpoint. A successful quick/manual save, load or authored checkpoint replaces it. With no later save, retry returns to that visit's entry state; it cannot use a save from an unrelated visit. Repeated deaths retry the same snapshot, and failed writes do not replace it. This uses the normal save loader without changing the save format or overwriting save slots on death. In a new process, Continue establishes the loaded save as the retry point. Staged developer previews retain their exploration recovery controls.

Version 0.10's Tab level chooser deliberately starts a fresh visit with its campaign inventory. This differs from the recovery controls above; see [LOADOUTS.md](LOADOUTS.md).

Dry supported footing is sampled away from ledge edges, checked against hazard volumes, and checked again when used. Jumping while trapped does not erase the earlier route; a nearby pit-floor departure is skipped. Death freezes controls and plays a bounded faint/fall/drowning pose, then offers retry. The third-person death presentation and timing are provisional, not an original-game cinematic.

## Original fatal-fall evidence

The theatre vents do not overlap the school's three authored `trigger_fall` volumes. It would be incorrect to infer that this specific missing floor was originally a death pit.

A narrow, read-only Ghidra inspection of the local `fgamex86.dll` identified the `TriggerFall` registration, its event callback, and a player state transition named `KILLED_FALLING`. The supplied player state definition corroborates the falling-death animation. The independently written Rust handler implements the fatal outcome on swept player contact. Raw strings, tables and pseudocode remain in `private/fall-research` and other private logs; none is packaged as release source.

All 60 authored falling-death volumes are recognised across the supplied maps. Their original spawn-flag conditions, timing and cinematic events are not reproduced. Existing hurt volumes and high-impact landing damage also lead to the same retry screen when Sanity reaches zero. Going beyond the map bounds now ends the attempt instead of switching to free flight. Ordinary nonfatal drops are not automatically made lethal.

## Reproducible checks

`--world-check` drops Alice onto each of the eight steam platforms for both entry states and walks across their seams at 30/60/144 rendering Hz. It also crosses a real school fatal-fall volume, checks death, resets contact state and crosses it again. The existing school door route remains covered.

Synthetic checks cover remembering pre-fall footing, jumping in a pit, fallback when geometry changes, edge rejection, swept fatal triggers, pause, reentry and death-animation reset. Native Anode checks exercise the waterline, bank exit, steam-floor walking, R recovery and Enter retry using the actual Rust window. These are targeted tests, not a full school playthrough.

`--death-retry-check` exercises full-world rollback and repeated continuation against real Pool boulder, underwater turtle-guide and cinematic fixtures, plus a fresh school visit. It checks that death cannot overwrite the checkpoint, resources and pickups rewind with the world, and a later checkpoint replaces the entry snapshot. Its isolated quick-save fixtures remain under `private/death-retry-check/`.

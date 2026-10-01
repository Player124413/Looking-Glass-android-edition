# Crazed Clockwork — hatter1

The visit now has a persistent controller for its machinery, five levers, mirror puzzle, chair puzzle, scenes and gated `hatter2` exit. The original assets stay in the user's local installation; the controller does not execute original scripts.

## Restored progression

- The two gear levers work in either order. Each works once, with the original two-second delay. The first starts the great gear; the second lowers it and raises the passage barrier. Crossing the passage closes the barrier behind Alice.
- The spiral and plunger move. The lift rises and lowers with its rider, and the floating platform follows its authored loop.
- The mirror field has five stationary pillars and eight sinking traps. The false versions appear only in the mirror, follow the real traps and have no collision. Cheshire's hint also appears only in the mirror.
- The clockroom lever opens its paired leaves; crossing the room closes them. The laboratory conversation enables the port lever. Its scene opens the port, both Hatter door leaves and the three automatic gates.
- The Gryphon conversation removes the exit clip and door. The extending bridge and its rescue teleport become available on the onward route.
- Four distinct chair contacts lower the floating clock. Repeated contact with one chair cannot solve the puzzle. The clock door and teleport move with the clock. The floor rescues cannot bypass the preceding puzzle.
- The final lever plays the clock-stop camera sequence, settles the clock hands and opens both exit leaves. The exit enables only when the sequence finishes. Watching and skipping use the same final puzzle state.
- The onward exit loads the existing About Face battle at its default arrival. The battle controller is registered in the combined build.

Arrival restores resources once, as the original map does. Save data includes lever use, door movement, sinking platforms, chair contacts, scene position, speech cursor and completion flags. Older saves without this controller restart this visit at its entrance while retaining their carried resources.

## Verification

The mirror's near-plane clipping accounts for its reversed horizontal projection.
The reflected room remains visible at shallow angles beside the entrance pillar.
Native mirror checks cover the normal approach, Cheshire's hint and two close
side views, and reject an empty reflection. Projection tests cover seven view
angles at 4:3, 16:9 and 21:9 while retaining the aperture clipping plane.

The mirror pass now honors the main camera's PVS for stationary apertures and
uses its reflected, obliquely clipped projection for geometry and transparent
effect rejection. Moving mirrors retain transformed visibility checks; missing
PVS data fails open. Reflection resolution, update rate and puzzle state are
unchanged. The native render check rejects unnecessary reflections in the gear
room and laboratory, alongside the four visible-mirror checks above.

Ten fixed Clockwork scenes matched pixel-for-pixel before and after this change.
Native release profiling and exact diagnostic sources are retained locally in
`private/clockwork-performance-fix`. Timings include driver submission/readback
costs and vary with system load; they are not hardware-independent FPS claims.

### Lever interaction correction — 1 October 2026

All five levers now include Alice's supplied `use_lever` performance, the moving
handle and its timed sound. Alice approaches using normal player collision and
step movement; an obstructed approach cancels without consuming the lever. A
collision-checked oblique camera frames the interaction. The final clock scene
follows the pull. Pause and optional saved action state retain the approach and
animation phase; old saves do not replay already-used levers.

The lever contract exercises all five contacts at 30/60/144 Hz, pause and
restored continuation. Native captures cover approach, reach, pull and finish.
The Clockwork Store suite includes an eighth case saved during the hand action.
These are component and persistence checks, not a new full-map campaign run.

- `--hatter1-check`: lever ordering, repeated/out-of-order events, pause, rejected saves, moving teleport, 30/60/144 Hz clock-stop completion, watched/skipped conversations and an input-driven crossing into the implemented Hatter arena.
- `--hatter1-traversal-check`: production lever input, all thirteen mirror-field pillars, a lift rider, four cushion contacts and movement from each authored launch pad onto its cushion. The component starting positions are staged; this is not a full-map combat playthrough.
- `--hatter1-render-check`: native machinery, mirror and scene captures. `LOOKING_GLASS_CLOCKWORK_SAVE=write` / `read` selects separate-process native Store fixtures, including a conversation in progress.

## Fidelity limits

The existing spline and character renderers interpret the supplied camera and animation assets. The laboratory's introductory walks use bounded timings, and its peripheral child-processing machinery is not fully reenacted. The post-conversation dunk/electric loop is a bounded visual interpretation. Sink limits without an explicit map value use 192 units (16 for cushions), with a capped return speed; native sink dead-time semantics remain an approximation. The mirror renders the room, puzzle brushes and its own Cheshire actor; it does not add reflections of every dynamic creature. The script references a nonexistent `rage_gear`, so there is no such mover to restore. These limits do not replace the required puzzle contacts or enable an early exit.

Native checks run muted on the hidden test desktop. They do not verify audible playback or every combat encounter and difficulty.

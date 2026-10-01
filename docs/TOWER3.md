# Machinations

`tower3$tower3_start1` now has its bound elevators, rotating gears, pedal
plates, swinging arm, cage, pendulum and ratchet. The controller owns all 47
script objects, including the seven invisible parent origins, and the two
static fulcrums. Entity IDs preserve both objects named `gear01`. The lamp is
visible but non-solid; the two unnamed platforms remain solid.

Drawing and collision use the same parent-and-child transforms. Alice is
carried by translation and rotation while remaining upright. Machinery can
push her when space is available; a blocked damaging part applies its crush
damage. An obstructed group stops without freezing unrelated machinery.
The Watch pauses the mechanism clocks. Saves retain each group's clock,
including a group delayed by contact, and restore the same rider future.

The room triggers select exactly one of the three fall-return floors. The
overlapping room-two floor cannot intercept a room-one fall. Room selection
survives saves, and explicit recovery uses that room's destination without
resetting the machines. Old saves without this controller use entrance
recovery and retain campaign resources.

The Cheshire introduction uses the supplied camera path, seated animations,
speech, fades and appearance/disappearance cues. Watching and skipping both
finish the dialogue and return control at the original landing. The separate
hint region cannot interrupt the introduction. The ratchet's repeating impact
cue follows its saved clock.

`ladder_drop` is consumed as a no-op: its ladder, crank and debris targets are
absent from the supplied BSP. The map contains no enemies. Its three small
essences and ordinary exit #17 remain part of the route. That exit loads
`grounds1$grounds1_start1` and carries resources into Royal Rage.

## Validation

### Machinery overlap correction — 1 October 2026

The long arm's relative yaw direction was reversed. The supplied signed
direction command moves it north from its eastern rest position; the previous
controller sent it into the southern pedal room, through the pendulum and lift
shafts. Its 24-second travel/rest cycle now uses the correct direction. The
attached wheel retains its independent counter-rotation. Rendering, collision
and rider movement all use this corrected pose.

The audit reviewed all 49 owned parts, their pivots, axes, bindings and timing.
A sampled solid-brush overlap survey covered 64 seconds; the large arm
intersections disappeared, leaving the shallow grinder and ratchet contacts.
The new arm contract checks the entire cycle at 120 samples per second, both
end rests, clearance from lift cores and the wheel attachment. Native paired
captures cover the reported pedal-room view and representative machinery.
This survey is not an exhaustive triangle-intersection proof for every possible
combination of independently delayed machine clocks.

The Normal route drives production movement and collision from a fresh entry,
with the introduction watched and skipped in separate runs. It crosses the
gears, elevators, pedals, pendulum, cage, middle wheel, eastern balcony, final
platform and ratchet. All three deliberate falls use the authored return
floors. Both runs reach Royal Rage alive, with no extra teleports or recovery,
and compare 46 disk continuations. Each continuation checks up to 120 identical
idle ticks before resuming the saved point. No enemies are omitted from this
map; none are authored here.

`--tower3-check` checks bind poses, duplicate names, save validation, scene
outcomes at 30/60/144 Hz, rider carry, blocked/free crushing, checkpoint trigger
contacts and production recovery. `--tower3-route-check` and
`--tower3-skip-route-check` run the route. `--tower3-render-check` captures the
introduction or a saved machinery view. Private evidence is under
`private/tower3-work/`; release details are in `docs/VALIDATION.md`.

Production Store fixtures are selected with `LOOKING_GLASS_SAVE_CASE=tower3-`
and the separate `--save-check-write` / `--save-check-read` processes. The
fixtures cover lift, pedal, room-three and in-progress introduction state.
The route separately exercises actual saved rider positions.

## Remaining fidelity limits

The two fulcrums are solid and static, as specified for this playable version.
The pendulum uses a bounded four-second sinusoid at the authored 30-degree
amplitude; its exact original integrator remains unverified. Pedal plates stay
level while their parent positions orbit, matching the supplied local-angle
binding behavior. Directional rotation commands continue
through 360 degrees, rather than reversing at 180. Native captures are silent;
audible playback, a manual keyboard playthrough, other difficulties and a full
campaign-chain run are not claimed.

# Labyrinthine Revenge

Use `Launch.cmd` for ordinary play, or `tools/launchers/Launch-Labyrinthine-Revenge.cmd`
to start this visit with separate playtest saves.

`hedge3$hedge3_start1` has one machinery controller, state version 1, inside
save envelope 12. It owns 29 inline script objects, the weighted lava platform
and 27 sliding door leaves. The rendered and collision poses share saved state.
The existing native enemy cast and authored traversal volumes remain active.
There are no cinematics or invented enemy-kill gates in this visit.
Fresh arrivals settle the elevated editor start onto the entrance platform
before the first frame, for both the named entrance and default chapter entry.
The entrance position and facing are retained; saved mid-level positions are unchanged.

The lava and ride gears carry an upright rider. Rolling wheels, turbines,
crusher slabs, swinging hammers, bellows, pump arms, pistons and the small
ambient gear move on their authored axes. Explicit 1,000-point mover damage
is applied when displacement is blocked, allowing Alice to stand on ride
gears. The original lava, fall and turbine hurt volumes remain lethal.
Moving curved surfaces use an opt-in brush/patch collision template; existing
maps keep their current collider selection. Local geometry and conservative
bounds are cached. Exact transformed hulls are built only when queried and
shared by subsequent queries at that pose. Substeps cap motion at two
degrees/eight units, and the final pose is exact so restored rider contact is
identical. Upright riders can step across the turntables' 16-unit teeth.
Small corrections separate a body from a rotating face; displacement still
has to clear the rest of the world. A free corner contact does not count as
a crush, while the central shaft and neighbouring gear can still block a rider.

The two bellows run eight-second cycles. Their westward pushes and steam emit
only during the second four seconds. Steam already in flight expires normally.
The Watch freezes the controller clocks and movement. The weighted platform
uses the researched native acceleration/spring law for speed 60 and the
implicit 1,000-unit limit, retaining depth and velocity through saves.

Automatic doors share contact bounds across adjacent leaves. Player and actor
masks are separate: the two NOT_MONSTERS leaves ignore enemy proximity, while
`t28` excludes Alice, opens for the drop-trap actor, slides down 240 units and
stays open. Ordinary doors close after three seconds without contact and
reopen if their closing motion would embed Alice. Actor contacts are transient
and refreshed by both the viewer and native route driver before machinery.

Sky trigger threads select all seven authored camera origins; hedge1 retains
its three selections. Save/load keeps the selected sky. The exit remains BSP
trigger #101 and leads to `tower3$tower3_start1` with carried resources.

The two timed crushers preserve their alternating four-second raises,
one-second slams and eight-second cycles. Their chain loops and slam cues use
saved clocks. Sliding doors retain movement loops and endpoint cues; loading does not replay
a previous stop. Swinging hammers use a
bounded three-second sinusoid at the authored 45-degree amplitude; the exact
original pendulum integrator remains a fidelity follow-up.
The rotate-up/down commands continue through 360 degrees; they do not reverse
at 180 degrees. Old controller-less saves use the existing entrance recovery
path without discarding campaign inventory.

The native Normal route is verified from fresh arrival through unedited live
checkpoint continuations into Tower3, with 76 disk checks and live enemies.
It includes combat and earned essence pickups before the exposed gear jumps.
This is not a single uninterrupted process. Validation is recorded in
`docs/VALIDATION.md`. Contract checks cover real
asset geometry, rider carry and restored futures, raised teeth and airborne
contacts with rotating braces, blocked/free crusher
contacts, door flags and pairing, Watch pause, malformed saves, bellows force
and sky switches. Native save fixtures cover gears, gust, slam and sink state.

## Reproducing the checks

`--hedge3-check` runs the asset-backed machinery contracts and verifies that both
fresh entrance paths remain grounded from the first tick through idle gameplay.
`--hedge3-route-check` starts a fresh Normal visit, drives ordinary movement
and native combat, checks disk continuations and takes the actual BSP exit.
At each checkpoint the original and restored worlds receive the same idle
input for up to 120 ticks. They must match at every tick, including the same
death if waiting in danger is lethal. The route then resumes the checkpoint;
the comparison does not impose an idle pause on the player's traversal.
`--hedge3-render-check` captures machinery and all seven skies. The route and
render artifacts are written under `private/hedge3-work/`.

For production saves, set `LOOKING_GLASS_SAVE_CASE=hedge3-` and run
`--save-check-write`, then `--save-check-read` in a separate process. These use
isolated fixtures under `private/save-check/`, not the player's save folder.
Native checks in this restoration were silent and ran on a private desktop.
They drive production controls programmatically; they are not a manual
keyboard playthrough or a full campaign-chain proof.

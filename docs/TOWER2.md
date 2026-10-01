# Water Logged

`tower2$tower2_start1` retains save envelope 12 and controller state version 1.
One owner drives the three weighted flushers, three lids, five fans, moving
liquid and the opening Cheshire scene. Rendered and collision poses come from
the same saved machinery state.

The initial surface is 624. Each ordered flusher raises it 256 units over five
seconds: 880, 1136, then 1392. The first two lids open to +35 degrees in six
seconds; the last opens to -35 in five. Opening a later lid closes the preceding
one to 90 degrees over eight seconds. Open lids sway by two degrees on a saved
ten-second cycle. The exit requires the third rise and an open final lid.

The authored flusher speed controls acceleration, rather than a fixed sinking
speed. Riders depress each tray by up to 64 units; vacated trays return with
damping. Their depth and velocity persist. Trays continue during the Watch;
water, lids, fans and enemies freeze. Swept collision checks carry riders and
reject machinery motion that would embed Alice. The five fans rotate on their
authored axes and rates, with lethal 1,000-point damage on actual blade contact.

The four drain currents retain their native velocity projection and inherited
0.2-second trigger wait. Previously the projection ran every physics tick,
removing Alice's ability to steer away from the drain. Each current's remaining
wait now persists with the tank state; other maps' force volumes are unchanged.

The introduction uses the original camera track and Cat clips, runtime-loaded
dialogue, appearance/disappearance sounds and fades. Watched, skipped and saved
playback return control at the supported arrival pose. Entry grants the Reptile
Shell. Four underwater air emitters refresh its twenty-second breath allowance;
their overlapping server lifetimes leave a continuous local refresh volume.
The existing particle emitters remain visible.

The native resident cast contains seven placed Snarks and twenty spawned Snarks
in five distinct waves (3, 6, 5, 4 and 2). Duplicate target names retain separate
entity identities. Their ordinary swimming, tongue/projectile combat, damage,
death and loot remain active. There are no invented enemy-kill exit gates.

Old rising-water saves preserve their water stage and skip the newly added
introduction. Missing machinery state is derived from the saved stage clocks.
Controller-less legacy imports use the established campaign recovery path.

`--tower2-check` verifies introduction variants, all machinery, Watch behavior,
legacy stage migration, air refresh and save continuation at 30/60/144 Hz.
`--tower2-cast-check` exercises all five native trigger volumes, distinct wave
receivers, exact saved futures and no resurrection on retrigger/load.
`--tower2-render-check` captures each water height and three introduction shots.
Four native save fixtures cover introduction, first rise, second-stage machinery
and the final opened shaft, using separate writer and reader processes.

The second climb exposed a beveled pipe rim that rejected both hand probes.
The ledge check can now reach across a narrow connected bevel to a horizontal
cap. It still rejects empty gaps and requires both hands and a collision-clear
pull-up. The asset check includes a real jump and saved mid-pull continuation.

`--tower2-route-check` completes the whole Normal route from fresh arrival with
the introduction watched and skipped. Ordinary movement reaches all three
flushers through the live currents, fans and native Snark combat, collects the
large essence, then dives through the real exit into `hedge3$hedge3_start1`.
Both runs finish at 75 sanity with about 13.9 seconds of air remaining and no
drowning damage. Each run reloads ten disk checkpoints and checks identical
two-second futures, including rising water, air sources, the fan approach and
the final dive. No god mode, notarget, teleport or staged progression is used.
All five waves are checked independently; this route does not require every
optional encounter to be cleared. This is standalone native route validation,
not certification of the complete campaign chain or every difficulty.

`tools/launchers/Launch-Water-Logged.cmd` starts a separate playtest save directory through the
regular root launcher. The normal launcher includes these changes as well.

Fidelity limits: the two-degree sinusoidal lid sway is a bounded reconstruction,
not a verified port of the native pendulum integrator. Flusher acceleration is
resampled from the researched 20 Hz law. Air refresh uses the same model-bounds
approximation as the restored temple bubbles; exact native bounds setup order
is unverified. Fan motion has no additional synthetic blur. Native checks run
muted; original script and subtitle prose are not stored.

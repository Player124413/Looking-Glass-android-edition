# Dry Landing

Visit13 (`garden1$garden1_start1`) owns its arrival and Rabbit conversation through
one registered controller. Save envelope12, visit order and hit reservation7200000
are unchanged. No original dialogue or script bodies are stored here.

## Entry and outcomes

`Garden1_Start` runs on fresh entry, as called by the supplied map's main thread.
There is no arrival touch trigger. `push_start1` (entity117) supplies the launch
volume and target-apex impulse; it is active only during the launch phase and is
retired afterward. Alice is initially hidden for2seconds, then launches, performs
`pain_knockdown` and `idle`, and joins `Garden1_Cinema2`.

`garden1_path1/2/4` supply the arrival shots. Turtle and Alice use the authored
markers and ordered animation clips. The five source dialogue calls retain their
order and post-voice waits0.5/0.3/0.4/0.2/0.4seconds. After the third line the owner
grants `turtle_air`, presents `mock_shell_alice` at `tag_back`, and runs the supplied
`fx_pickup` burst. Turtle jumps, swims along `turtle_swimpath1` with travel-derived
orientation and emits bubbles. Completion removes his rendered presence and
returns Alice to `turtle_waterpos1` (-1368,-2472,-384), yaw0.

The real entity58 volume starts `Garden1_Rabbit_Cinema1` once. `garden1_path3`,
Alice's approach to `alice_posx1`, Rabbit's gestures, five dialogue calls and
departure through `rabbit_jumppos1`/`rabbit_jump1` are restored. The post-voice waits
are0.4/0/0.3/0.4/0.2seconds. Completion removes `rabbit_actor` and returns Alice at
`alice_posx1` (-1032,-360,680), yaw0, settling to the actual supporting floor.

Watching and skipping share the C1 landing transaction. Saved phase, shot/cast,
line and shell clocks resume without repeating the grant or activation. Pause
advances none of them. C3 supplies original voice/subtitle references at runtime,
Alice aliases and lip envelopes; missing lip data uses a neutral mouth.

Fresh entry refills Sanity and Will once without clearing carried timed powers.
The breathing upgrade is monotonic. Existing controller-less saves continue in
gameplay without replaying arrival or refilling resources; formerly pending
Rabbit contacts rearm through F2. Other scene completions and item histories are
not reset. A fresh visit or pre-entry save is needed to watch the arrival.

## Gates and scope

Neither conversation opens the level exit or spawns an enemy. The existing
scene contract pauses encounter simulation and returns it to gameplay afterward.
The recovery shortcut (entity2) remains closed until real `Open_Portals`
contact(entity1); its two frames become visible and solid then. Exit131 retains
its authored destination `garden2$garden2_start1` and has no invented dialogue gate.

The three supplied lily loops (`lily2`, `lily4`, `lily99`) now move their
bound collision brushes (`clip2`, `clip4`, `clip99`) with the same saved spline
clock. Clips stay invisible; pads retain their authored yaw and scale. A standing
rider moves with the support, subject to ordinary body clearance. Missing pad
and path references remain no-ops.

Both gravity-path chains contribute river flow through normal player movement.
Only the nearest eligible path acts; radius, water eligibility, direction, node
speed and maximum speed come from map data. This adapter uses normalized segment
interpolation and resamples the native server contribution at the fixed movement
rate. It does not claim exact parity with every native interpolation edge case.

`Bridge_Drop` starts seven physical pieces with their individual delays, travel
times and rotations, then removes their collision and rendering after16.6seconds.
The scene-less dead-tree event starts `rabbit_actor2` along the supplied ground
pathnodes to `rabbit_follow2`, then hides him. Neither event takes camera control,
grants an item or opens the exit. The unaddressable rock callback stays a no-op;
its placed rock remains visible and solid.

The shared NPC owner retains all old actor identities and appends the missing
Ladybugs and trigger-spawn actors. Direct targets fire alongside supported
callbacks. Five scripted ant runs use the supplied pathnodes, swept hulls and
saved waypoint progress before releasing normal combat. Difficulty exclusions,
health, death and loot state survive restoration. Old saves only rearm world
contacts previously reported as unhandled; completed events are not replayed.

Movement uses the port's swept launch, waypoint interpolation and bounded jump
arcs. These preserve supplied markers and timing but do not claim exact native
pathfinding or jump physics. Cameras use the existing native-derived spline
adapter. Particle compositing uses the shared renderer.

## Verification

- `--garden1-check`: original clips, real Rabbit entry via Player movement,
  watched/skipped playback at30/60/144Hz, every line, repeated mid-scene restoration,
  pause, supported handoffs, one-time refill/upgrade, real recovery/exit contacts,
  and legacy pending-trigger migration.
- `--garden1-world-check`: full lily loops and riders at30/60/144Hz, bridge
  pose/retirement, pause, saved continuation, terrain-safe Rabbit route, current
  direction/caps/bounds and pending-only migration.
- `--garden1-cast-check`: real trigger volumes, all difficulty filters, scripted
  run completion, unchanged identities and saved cast continuation.
- `--garden1-render-check`: original-camera scene samples plus world samples.
- `--garden1-route-check`: production cast and ordinary inputs through the visit.
  `--garden1-skip-route-check` covers the same full course with scenes skipped.
  Both fresh Normal runs cross the real exit and load Garden2 alive, with exactly
  carried resources and no recovery teleport. The course uses actual pickups,
  funded attacks and the lower riverbank shelf; combat is not weakened.
  `--garden1-path-check` is a movement-only diagnostic and is not combat proof.
- `LOOKING_GLASS_SAVE_CASE=garden1-` with `--save-check-write` then
  `--save-check-read`: twelve native saved futures in separate processes.
- `tools/launchers/Launch-Dry-Landing.cmd`: fresh Dry Landing entry with separate playtest saves.
  Root `Launch.cmd` uses the same published executable.

Private captures, source manifest, build provenance and test results are retained
under `private/garden1-scenes/` and `private/garden1-world/`. No listening comparison is claimed by silent
native tests.

Full-route certification here is for Normal difficulty. All difficulty filters
are checked separately at the encounter level. The broader opening campaign
chain still encounters a pre-existing School Two navigation failure, reproduced
in an earlier selected build; it is not a Dry Landing exit failure.

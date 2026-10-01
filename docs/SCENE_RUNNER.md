# Shared scene lifecycle: Pool pilot

The first C1 adoption is `Tears1_End_Cinematic`, inside the existing `Pool` owner.
There is no Pool registry controller and no new scene-specific command-line arm.
Earlier Pool scenes keep their existing timelines.

## Library contract

`level::spec` declares the minimal `SceneSpec`, track shots, `EndSpec` and `ExitSpec`.
`level::scene::SceneRunner` advances saved scene, shot, cast and dialogue clocks;
zero delta pauses them. Shot offsets and endpoint holds are independent of the
scene clock. Cue bits prevent repeat effects after restoration. The pilot has one
cast clock, used by Alice's existing puppet choreography, and no dialogue lines.

Watching and skipping call the same `finish`. Completion normalizes clocks and
consumes outstanding cues; a skip does not play the skipped sound. A non-exit end
uses `cinematic::land_player` at the authored endpoint or captured home, allowing
only the existing vertical support adjustment. A failed landing cannot commit.
An exit end keeps Alice under scene ownership until departure succeeds.

`level::exit::ExitState` separates durable completion from an in-flight delivery.
It emits once until a reported load failure, then retries after one active second.
Loading a still-current completed scene re-emits its pending exit. The viewer
reports failed transitions through `Interactions::transition_failed`; this does
not reset puzzle, reward or scene state. A successful load replaces the owner.

Other planned C1 shot types, generic actor commands, world cues, FOV tracks and
automatic registry-wide scene checks are still future work. Existing legacy
staging is not converted implicitly.

## Pilot and dependency order

1. W1: Pool BSP entities 1–6 are collision-only player-clip brushes. Their original
   difficulty flags select none on Easy, 3/4 on Normal, all six on Hard/Nightmare.
   They are excluded from rendering. No unreviewed script objects are enabled.
2. W4 subset: `trigger_hurt` reads `setdamage`, then `damage`, then the existing
   default. Pool entity 127 now delivers its authored 9999 damage.
3. W22b subset: the exit adapter owns the one-shot delivery and failed-load retry.
   Physical or indirect activation of Pool entity 115 cannot bypass the scene;
   exit inspection reports that volume as unavailable.
4. Minimal C1: entity 82 starts a five-second scene using `tears1_path3`, with a
   -0.5 second track offset, the existing Alice jump and a sound cue at 2 seconds.
   Completion requests `potears2$potears2_start1`. No quest/item grants are added.

## Persistence

Save format remains 12. Pool's inner cinema version becomes 2. The optional
`ending` state stores the runner clocks and durable exit commitment. Older active
endings retain their time and captured position; completed endings retain their
commitment and can deliver an outstanding exit. Previously consumed pending end
contacts rearm without replaying arrival or the Turtle conversation. Cached Pool
visits are upgraded too. Existing event keys and program signatures are unchanged;
there is no gate-program migration because the legacy event definitions did not
change. The scene owner applies the exit gate before delivery.

## Checks

`--pool-check` includes the pilot matrix at 30/60/144 Hz, watch and early/middle/late
skip, five saved checkpoints, paused clocks, equal completion state, pending and
active legacy migration, one-shot exit and failed-load retry. It also checks the
difficulty clip sets, authored hazard damage and physical last-leaf approach into
the scene, followed by the strict shared campaign transition.

`--pool-render-check` includes four ending times. Native save cases use the
`pool-pilot-` prefix for fade, jump, fall, late and complete checkpoints. Writer
and reader run in separate processes. These are staged checks, not proof of the
entire Pool route or the full campaign chain.

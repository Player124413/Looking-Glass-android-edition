# Dialogue registry (C3)

Task 10 provides reviewed dialogue and speaker bindings. It does not complete the Bill scene in task 15.

## Registering a new scene

A visit contributes `story_beats: &[BeatSpec]` through its existing registration. Legacy visits retain their saved event names, source-function order, line indices and pause rules in `story::registry::LEGACY`. Functions sharing an event append in specification order. Loading an event is atomic: a failed part removes the whole event, preventing partial dialogue from reporting completion.

`BeatSpec::linear` declares the exact number of source calls and rejects branching. Conditional dialogue uses `Calls::Gated { key, indices }`: reviewed, increasing source-call indices and an explicit owner gate. Register alternative branches as distinct stable events. The owner evaluates the condition and calls `Story::trigger_gated(event, key, allowed)`. The generic trigger cannot start a gated event. Restore reconstructs the selected event and its fixed list; it does not re-evaluate changing quest facts or renumber saved lines. This is a call-list reader, not a script interpreter. Conditional headwatch/acting and every world consequence remain with the scene owner.

The shared speaker table binds voice directories to labels and model families. `alice`, `player` and `fakeplayer` resolve consistently, including callers that still request an old alias. Placed NPC identities remain distinct. The reviewed school book Cat misassignment retains its existing correction. Never merge different Gnome/Cat instances just because their recordings share a directory.

Audio and subtitle data load from the user's archives. New beats use actual WAV lengths, including answers shorter than one second; legacy timelines keep their shipped durations. Lip envelopes sample the saved line clock. Missing envelopes produce a neutral mouth; corrupt supplied envelopes fail validation. The documented `catz306` missing-envelope case is a validation fixture, not an implemented Hatter scene.

`AnimationRef` validates model, clip and skeletal data. Missing animations need a named fallback. Do not silently accept the renderer's generic idle substitution as successful validation.

## Bill fixture and owner

`levels::potears2` is the single registered owner for that visit. Its C3 component joins the one line from `tears2_end_cinematic` and twelve from `tears2_dialog`, in that order. The event remains `tears2_end_cinematic`. The owner begins dialogue after an explicit readiness decision, receives its one-time callback, and persists `Dormant`, `Playing` or `Read`. Story owns the saved line/index clock; the owner does not keep a competing voice clock. `Read` means dialogue consumed, not the full scene completed. Consuming this dialogue component alone cannot open an exit or grant a reward. Task 15 now wraps it in the separately gated house scene; see [HOLLOW_HIDEAWAY.md](HOLLOW_HIDEAWAY.md).

The first line retains a 1-second post-voice wait; the remaining lines use 0.2 seconds. The source's 1.5-second camera cue during `bliz006` does not add a second delay after that recording. Bill's later lines apply the scene's 45-degree mouth range; the first uses the model/default 10 degrees. `talk_liftbelt` is absent from the supplied model; its explicit fallback is `idle_liftbelt`. Task 15 schedules the acting independently of the saved voice clock.

Bill is controller-owned and hidden while dormant. Task 15 gates the actual ending trigger on two guard deaths and the authored delay. Its camera/actor/door/suction lifecycle alone commits the Duchess transition; the raw exit stays closed. Generic dialogue dispatch cannot bypass this owner. The staged fixtures call `begin_dialogue` directly and do not prove traversal, rescue or a complete cinematic.

## Persistence and verification

Save format stays 12. No existing event keys, route numbers or hit ranges change. Only `potears2` adds controller state and closed gate conditions; its event signature changes. The existing F2 upgrade handles older controller-less visits, pending triggers and adoption of the formerly generic Bill actor. Other visit snapshots/signatures should remain identical.

Checks:

- `--story-check`: counts derive from BeatSpecs and existing BSP hint identifiers; decodes audio and validates matching subtitles/timing.
- `--facial-check`: speaker-table rigs, lip envelopes and neutral fallback. Queen first form has no mouth joint and remains neutral; second form (`c_q2_body`) has `tag_mouth` and uses the checked SKL/SKAN v2 reader.
- `--facial-render`: registered facial models in the isolated desktop.
- `--potears2-check`: thirteen ordered voice/actor bindings, explicit animation fallback, 30/60/144 Hz, per-line pause/restore and one-time dialogue completion; also the full staged house-scene checks.
- `--potears2-render-check`: staged house-scene captures through the owner's art component; also registered for visibility checks.
- `LOOKING_GLASS_SAVE_CASE=potears2-dialogue-` with separate-process save writer/reader: first, middle, last and consumed dialogue states.

The house scene and guard gate are implemented by task 15; normal lily/leaf transport traversal remains pending, so its tests are labelled staged. New facial models being supported does not mean their later campaign scenes are implemented.

Task 15 migrates C3 controller revision 1 and its exact closed-gate program to revision 2 in the same owner. Active dialogue continues; already-read dialogue stays consumed. Both still require the actual guard gate. They are not treated as controller-less F2 inputs.

# Cheshire bindings (C4, Pool pilot)

C summons a contextual hint from the most recently crossed enabled region, or a general reply. Entering a hint brush only selects a recording; it does not start a quest scene. E advances the recording. Pause, inventory and the console stop the presentation and dialogue clocks.

## Reviewed bindings

| Map | Entity | Target name | Voice | Condition |
| --- | --- | --- | --- | --- |
| potears1 | 46 | cat_after_talk | catz206 | Turtle conversation has started |
| potears1 | 47 | cat_before_talk | catz205 | Turtle conversation has not started |
| potears1 | 728 | cat_before_talk | catz206 | Turtle conversation has not started |

`cheshire/spec.rs` owns these HintSpecs. Conditions read the existing Pool owner, including immediately after loading; they cannot write quest state. The authored swap occurs at the start of `Tears1_Turtle_Cinema1`, before speech. Watching and skipping use the existing Turtle completion. A selected disabled region is cleared; a new overlap can select the enabled replacement. Two brushes sharing a voice remain separate identities.

The saved unnamed indices remain `[43,44,45,729,730]`, followed by `[46,47,728]`. Earlier index-only saves retain their selection, fallback position and cooldown. An already playing hint restores by its voice path, even if its former region is now disabled.

## Presentation and ownership

`cheshire/beat.rs` provides CatBeat: a saved fade/speech/departure clock. Visible new summons fade in over 2 s, begin speech at 2.5 s, start a 2 s fade 0.5 s after the recording, and are removed 1 s after the fade. The existing Story post-line pause contributes 0.35 s of that hold. The remaining cooldown is 9 s; advancing speech also advances the departure schedule. Fixed-size stippling approximates the original alpha effect. Talking uses `sit_talk1`, waiting/departure use `sit_idle1`, and supplied lips follow the saved line clock.

Placement retains floor, clearance, liquid and sight checks. Swimming or unavailable placement uses immediate voice/subtitles without a body or its appearance/disappearance effects. No new collision actor is created. Existing active format-12 appearances omit the CatBeat version and finish using their earlier presentation timing; they do not replay an entrance cue. New appearances save version 1 inside the existing hints snapshot. Envelope version 12 is unchanged.

Hints keep the separate `cheshire_hint` dialogue identity and never populate quest seen/completion history, grant rewards or request exits. One active appearance and its saved cooldown reject repeated requests. Owner-controlled cinematics reject new summons. `LevelController::allow_cheshire` is the extension point for a later owner's authored windows; it derives from that owner's saved state and does not create a permanent cross-map lock. Tower introductions and centipede2's post-boss hint still belong to their future map owners. This pilot does not claim those unfinished windows are implemented.

## Verification

`--cheshire-check`: all supported regions/audio, the three Pool brush identities, old indices, inactive-selection clearing, actual Turtle entry/skip windows, repeated watched/advanced/restored hints at 30/60/144 Hz, paused clocks, no quest completion, and native fixture placements.

`--cheshire-render-check`: six Pool views covering fade-in, before/after speech, fade-out, cooldown and voice-only fallback. `LOOKING_GLASS_SAVE_CASE=cheshire-pool-` selects six native fixtures for `--save-check-write` and `--save-check-read` in separate processes. Their continued snapshots must match exactly; repeated restored hints must leave quest state and resources intact. Run the retained old-save checks and compare visit snapshots/event signatures too.

Manual: in Pool, summon before speaking to the Turtle; pause during appearance, F5/F9, resume, advance and summon again after cooldown. Repeat after watching or skipping the Turtle. Verify the intended recording, a fixed-size fade, the same leaf/reward/exit state, and no repeated Turtle scene. No native listening or pixel-identical original renderer comparison is implied by these checks.

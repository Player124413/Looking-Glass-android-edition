# Heart of Darkness — Final

The `qlair` controller restores the finale as a standalone playable visit using the locally installed original game data. This is scoped to the finale; it is not approval of the full M8 campaign milestone or of the unfinished preceding visits.

The content audit and fixes cover both Queen fights, supporting actors, all ten projectile types, effects, cinematics and completion. See `docs/QLAIR_AUDIT.md` for the actor inventory, corrected findings, verification and implementation limits.

Use root `Launch.cmd`, press Tab and select **Heart of Darkness — Final**. The root launcher guards against older playtest builds that contained the map without its mission controller. Alternatively, double-click `tools/launchers/Launch-Heart-of-Darkness.cmd` for the dedicated finale build and separate saves. New Visit begins in the corridor; Continue resumes the latest save. F5/F9 quick-save/load, P pauses, and holding Enter skips scenes and the ending film.

## Implemented flow

- Corridor popups, the Queen's introduction and voiced dialogue, throne break, falling floor and stairs, then the first fight.
- Queen1's normal and wounded attacks, laser and telekinetic preparation/recovery, escape window, grab camera/release, and alternating medium essence pickups.
- The transformation sequence, original camera paths, Alice and Queen animations and dialogue, halo and resource refill, second-arena reveal and one durable checkpoint.
- Queen2's body and four separately damageable tentacles, authored health thresholds and Blunderbuss immunity on tentacles, attack animations and projectiles, bridge-triggered submerge/relocation, four large essence stations, and the secret teleport.
- Independent tentacle attacks, head growth and attention, collapse and finishing hits, seeking projectiles, splash damage, impact effects, claw/club trails, crown/head lights and authored sound cues.
- Final death sequence with tagged explosions and debris, ten-second ending delay, original ending film, Credits, and Main. Cinematic control remains active during the delay so Alice cannot fall after winning. A completion marker preserves the pre-ending encounter for Continue. Ending and post-game quit do not overwrite that checkpoint.

Pause, restore and cinematic skipping keep the same state and handoff destinations. Boss health, independent part clocks, projectiles and trails, impact/debris clocks, grab state, movement, dialogue, essence timers, power and checkpoint state are saved. The finale ignores the Watch's world slowdown, as authored.

## Verification

Evidence is stored privately under `private/qlair-work/`. Asset-backed contract checks cover scene completion at 30/60/144 Hz, early/middle/late skip, pause, saved-clock continuation, both defeat gates, dead-player guards, platform events, tentacle immunity and the ending delay. Combat route and native validation results are recorded below when complete; staged scene fixtures are not combat proof.

Native checks run in the hidden Anode desktop with separate save folders and `--no-audio`. The original ending was watched through to Credits; returning to Main and then relaunching Continue restored a live second encounter. The five scene save fixtures were written and restored in separate native processes. Audio files and movie soundtrack decoding are checked, but audible playback has not been assessed.

### Reproduce

Build with `cargo build --locked --release --target-dir private/qlair-work/build`. The executable is `private/qlair-work/build/release/looking-glass.exe`. The launcher uses a separately copied, verified release in `private/heart-of-darkness/looking-glass.exe`.

- `--qlair-check`: asset-backed scene, state, combat and ending contracts.
- `--qlair-route-check`: actual corridor and both fights on Normal/Hard, watching scenes.
- `--qlair-skip-route-check`: the same route with scene skips.
- `--qlair-second-route-check`: explicitly staged second-arena diagnostic; it does not prove the first fight.
- In Anode: `--qlair-render-check`, then `--qlair-save-write` and `--qlair-save-read` in fresh processes.

The additional finale diagnostic flags are explicitly reserved in `src/levels/reservations.rs`; the original four finale flag names remain available.

The full combat routes start at the corridor and pay for Staff attacks through the production weapon actions and Will wallet, falling back to the Blade. They use ordinary movement inputs, collect the arena's essence and dodge projectiles. Both bosses reach their defeat thresholds through weapon hits. God mode, no-target mode, free ammunition and injected boss damage are excluded. Navigation yields when a defeat starts a scene, and brakes before jumping on narrow essence pedestals.

## Fidelity limits

The state machine is an independently implemented, bounded Rust controller, not execution of the original game DLL or script VM. Body movement, popup travel, head attention and cosmetic fragments use this engine's integration and rendering conventions. Recursive effect children have explicit depth and particle limits. One missing wounded laser-preparation recording falls back to an existing normal recording. These choices are not bit-for-bit original-engine parity. Audio assets are validated without audible playback.

### Initial audit results

Initial audited release, 30 September 2026: SHA-256 `D767AD5DC10EC80305ADA8A97A37E23D0E332576FF0FEA83EE034079051A54E3`, preserved as `private/playtest/looking-glass-d767ad5d.exe`. The current launcher copies include the later beam/Ice fixes described in `QLAIR_AUDIT.md`; `private/qlair-audit/beam-evidence.json` records their hash and checks.

- 565 unit tests and all asset-backed finale contracts passed. Coverage includes 14 staged attack cases, all ten projectile types, independent tentacles, projectile physics, grab escape/release, scene timing, save continuation and ending control.
- Continuous watched Normal and Hard routes reached ENDING in 35,877 and 44,854 ticks, with Alice alive and without cheats. Normal with all scenes skipped also completed, in 17,060 ticks.
- **Remaining verification gap:** the automated Hard skipped route fails after combat knockback near an essence pedestal. Its navigation cannot recover; this run is not reported as passing. Early/middle/late scene skips and handoffs independently pass at 30/60/144 Hz. A complete Hard skipped playthrough still needs verification.
- 34 native scene/projectile/impact captures passed with no dropped effect passes. Visual inspection confirmed the restored explosion layers, final destruction, ice sparks, popup dust, beams, head growth and swipe effects.
- Five native save fixtures were written and restored in separate processes. Completed-campaign Continue selected the preserved second-arena checkpoint.
- The exact packaged build passed native root-launch Tab entry, introduction skipping, movement and quick saving. A staged ending restore proceeded to the film, then Credits and Main. Native manual boss victories and audible audio quality are not claimed.

Current logs and `gaps-evidence.json` are under `private/qlair-audit/`; `published-root-tab-proof.json` records the root-launch reproduction. Earlier `evidence.json` and `final-*` logs belong to the previous release (`7213EC10...`). The original film was previously watched through on an earlier candidate; the current interactive check verified film entry and skipping to Credits.

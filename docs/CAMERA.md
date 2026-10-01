# Camera behaviour (v0.31.3)

The ordinary third-person camera keeps mouse aim responsive and follows lateral movement directly. Small step and landing height changes are damped, with at most 12 units of vertical lag. The old automatic overhead lift when backing into a wall is removed. A swept camera box retracts immediately when required by collision, then waits briefly before easing outward. Visibility hysteresis stops Alice flickering in and out when that arm is very short. Menus and pausing freeze the smoothing; load, recovery, teleport, level changes and view changes reset it.

Pandemonium's cart now turns continuously across track nodes using a short symmetric sample of its rail direction. Its position, travel times, triggers, landing and authored camera cuts are retained. During the rail-follow shot, the camera checks tunnel clearance along its arm and up to 0.6 seconds ahead, allowing it to ease inward before a narrow section. Scripted scenes own the view while playing; mouse/arrow look cannot accumulate an unseen rotation behind them. Normal look control resumes after the scene.

The player-facing camera-distance option remains in the original Settings menu. First-person mouse aim is unchanged. Cinematic camera cuts remain intentional; collision safety can still require immediate retraction if an unexpected moving object closes on the camera.

## Cutscene return

Nearby scene endings ease back to the gameplay camera over 0.45 seconds, orbiting around Alice rather than passing through her body. The return path is checked against map collision before it starts and swept again while moving. Distant cutaways, obstructed paths, first-person returns and skipped scenes use a short covered cut with a 0.25-second reveal. Existing end fades carry into that reveal instead of disappearing in one frame. Cuts within a cinematic remain authored cuts; pause freezes the return and load, travel, recovery and view changes clear it.

Alice resumes at the final reviewed scene marker and facing, with only a small vertical floor adjustment. The Village intro uses the end of her walk, and Gnome conversations retain the final speaking location. The Blade scene's Rabbit cutaway leaves Alice at the Blade marker; it must not move her to the distant Gnome marker (that warp is commented out in the original script). Pandemonium's cart landing preserves the final 225-degree facing. Intentional transport and level transitions retain their destinations.

Alice puppets retain their last local pose in memory. A nearby, normal-sized pose seeds the gameplay animator's existing blend, avoiding a sudden idle-pose change or reloading animation assets. Distant and scaled poses are excluded. Scene art, including the hint Cat, is prepared during level setup rather than loaded in the middle of drawing a scene. This does not promise a particular frame rate or eliminate unrelated loading costs.

Validation: `--camera-check` exercises full orbits around 48 supported positions in each of gvillage, skool1 and skool2, checking the swept camera volume and sightline. It also checks every cart turn boundary and the live rail ride at several frame rates. Unit regressions cover cramped walls, intermittent obstruction, orbiting a corner, pause, step smoothing, teleport/reset and frame-rate consistency. See VALIDATION.md for results and native test coverage.

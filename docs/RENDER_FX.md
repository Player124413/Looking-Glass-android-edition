# Shared materials, lights and attached particles

The September 2026 renderer pass reads the supplied shader and TIKI declarations. It does not run the original renderer or client scripts.

## Materials and transparency

World transparency, registered skeletal/vertex model materials, Meta Essence layers, pickup billboards, projectile trails, boss effects and particle sprites join one triangle-depth queue in the normal world view. Solid/cutout surfaces establish depth first. Transparent triangles test that depth without replacing it. Each submitted surface retains its authored material-layer order and its own dissolve, ghost, tint, fog and light state. Resources remain alive until the GPU submission has flushed, including short-lived opaque packets.

Model skins now resolve every supported original material stage rather than only the first image. Source/destination blend factors, animated textures, environment/vector coordinates, UV transforms and existing stage colour/alpha operations are interpreted. Material pipelines are shared, and unused indexing slots do not trigger repeated bindings. Additive sprites preserve both texture alpha and lifetime fade.

Intersecting triangles still use centroid sorting; this is not order-independent transparency or exact original sort-key emulation. First-person toys keep their separate depth layer. Unsupported shader operations and missing source textures still require individual fidelity work.

## Lighting

Supported map emitter declarations supply point-light colour, radius, offsets, activation and the `onlylightents` mask. Live croquet projectiles use their supplied light definition. At most eight lights are submitted, selected from at most 32 camera-visibility candidates. Pausing freezes their owning simulation clocks.

The supplied flare texture is used for the lens-flare emitter; opaque-world ray tests suppress hidden flares. Alice receives a projected contact shadow using the supplied shadow texture, with support checks and height fade. These are representative implementations: lighting uses radial attenuation, camera visibility is an approximation to room occlusion, and shadows are contact projections rather than complete per-light actor/prop shadow maps. Receiver-specific light occlusion, full actor shadow coverage and additional scripted light styles remain future work.

## Particles

Named animation `emitteron`/`emitteroff` events control supported emitters. Map TAN tags sample their animation axes; shared NPC/prop effects sample attachment poses at birth. Previously emitted particles retain their world birth pose while the actor moves. Paused clocks preserve samples. Rewinding or reloading reconstructs cosmetic births from the restored clock and pose; historical actor travel and individual cosmetic particles are not serialized.

World particles retain collision and liquid checks. Attached effects currently use analytical trajectories without world collision. Parsed models have at most 16 emitters, supported sprite rates at most 240 per second and lifetimes at most 30 seconds. Each attached effect has at most 4,096 live particles and retained birth poses, with at most 513 candidate births per emitter per sample. World particles retain their separate 4,096 limit and distance cutoff. The transparent frame queue has a one-million-vertex budget; regression fixtures fail if it drops a group.

## Repeatable checks

- `--billboard-check --no-audio` audits every supplied BSP axial billboard from 48 camera directions, preserving its mount, dimensions, texture coordinates and authored orientation. It captures the School fireplace with its actual animated materials and checks an asymmetric diagnostic texture through both world rendering paths, from both sides and at different camera heights. Captures and the corpus report are written to `private/billboards/`. This guards against wide or tilted fire surfaces turning upside down as the camera crosses their authored plane. The original game data is read only.
- `cargo test --locked` checks named activation, moving attachments, pause, restored clocks, expiry, combined budgets, light occlusion and material parsing.
- `--fidelity-check` audits the supplied data, including 85 supported emitter definitions; an unsupported sparkle declaration is reported separately.
- `--render-fx-check --no-audio` checks GPU compositing under three submission orders, solid occlusion, registered multi-layer model surfaces, additive texture alpha, dynamic light colour and masks, source shadow height and flare occlusion. It captures School, Pool of Tears and Pandemonium and writes synchronized CPU/GPU samples to `private/render-fx/performance.json`.
- Performance modes compare the legacy submission path, the shared queue with no lights, and the shared queue with a source-defined croquet light staged ahead of the fixed camera plus any visible map lights. These are rendering fixtures, not whole-game frame-rate measurements.
- `--fidelity-corpus-check` exercises the queue across every supplied map while retaining another scene and verifying its exact reference pixels after each switch.
- `tools/test_render_fx.ps1 -Executable <candidate.exe>` runs the billboard orientation check, GPU suite, all-map corpus and complete visibility suite against one binary, recording its SHA-256 and exit codes in `private/render-fx/run-summary.json`. `tools/test_visibility.ps1` remains available separately.

On the tested Windows release candidate, synchronized mean rendering times (six measured frames after two warmups, milliseconds) were:

| Scene | Legacy submission | Shared queue | Queue + authored test light |
| --- | ---: | ---: | ---: |
| School | 16.18 | 16.70 | 16.80 |
| Pool of Tears | 16.01 | 16.50 | 16.57 |
| Pandemonium | 16.20 | 16.51 | 16.65 |

These samples are display-paced in the background Windows desktop. Differences below a millisecond should not be treated as isolated GPU costs, and this is not an eight-light worst-case benchmark. The run did catch and resolve a large Pool of Tears regression: sorted runs now upload only their used geometry, adjacent compatible single-stage runs merge, and the normal camera rejects triangles wholly outside its clip volume before sorting. Triangles crossing a clip plane are retained.

Extracted declarations, captures and reports stay in `private/` and are excluded from source distribution. No pixel-exact comparison against the original executable is claimed.

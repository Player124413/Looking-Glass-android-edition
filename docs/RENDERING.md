# Rendering fidelity — version 0.29

**October 2026 shared water correction:** `deformVertexes wavenormal` now animates
the surface normals used by environment reflections and view-dependent opacity.
The original normal-wave instructions were previously ignored, leaving static,
flat reflections in Hollow Hideaway and thirteen other maps. UV turbulence now
converts its spatial phase from cycles to radians together with its time phase;
the former mixed units stretched distortion over too large an area.

These rules were checked against the supplied material declarations and read-only
inspection of the original renderer. Water retains its original images, colours,
blend modes and rates, including Hollow's intentionally opaque pond base. Normal
ripples preserve surface positions, collision, PVS/frustum culling and opaque
batching; Hollow still uses 197 scene batches and one grouped pond batch.

`--water-render-check` exercises the real Hollow pond and waterfall from two
viewpoints, comparing missing/restored ripples and checking animated and exactly
paused pixels. The update passes 662 unit tests (one unrelated test ignored),
the material audit, native shader/depth checks and native loading/drawing of all
36 maps. Private evidence is in `private/water-fix/`. Waveforms use continuous
sine evaluation rather than original lookup-table quantization; this is not a
pixel-exact original-runtime comparison.

**October 2026 school floor correction:** sky TAN decorations now honor
material face selection, with clockwise front winding evaluated after their
animated pose. This removes stretched reverse-face sheets seen through
`skool1`'s broken floor. Explicit two-sided materials retain both faces.
The school fixture compares five viewpoints against the former two-sided
rendering and checks foreground pixel stability. All 36 maps passed the native
render corpus check. See [SCHOOL.md](SCHOOL.md) for the focused evidence.

**September 2026 skies/performance update:** reviewed Hedge/Hatter/Garden sequences, saved fog fades, PVS/frustum culling, conservative TAN detail reduction and readable world weapon pickups are described in [SKY_PERFORMANCE.md](SKY_PERFORMANCE.md). It supersedes the corresponding unfinished items below.

**September 2026 shared-effects update:** model material layers, the cross-category transparency queue, representative original-data lights/flares/contact shadows, and animation-controlled attached emitters are now implemented. [RENDER_FX.md](RENDER_FX.md) supersedes the corresponding unfinished items below and records their verification and remaining limits.

The Fortress follow-up extends its authored vortex into the exterior's former black shell. Black window/prop batches stay opaque. Its original black fog volumes now fade world surfaces toward a GPU copy of the rendered sky, preventing dark tower caps. Other maps retain their existing fog blending. The native Fortress fixture also verifies ordinary cutouts, slime blending, fog neutral colours and sky occlusion before checking the new fade.

**0.31.5 correction:** Pandemonium's `green_slime2_1` material uses source-alpha/source-colour blending for its base, followed by destination-colour/one detail. The base stage was previously classified as unsupported and dropped, leaving a detail operation that cannot produce colour over black. Both original turbulent texture stages are now retained. The new blend's fully fogged neutral is white RGB with zero alpha. Pipeline indexing includes the new mode without overlapping equal-depth variants, and shared pipeline ownership remains in use. Source definitions, colours, view-alpha and geometry are retained; this is an independent renderer, not a pixel-exact original-runtime comparison.

**0.29.1 correction:** the earlier world-material sharing did not cover character/prop pipelines or the level chooser's retained art. Actors and props now share one weakly cached pipeline, and Tab, console map changes, campaign exits and loads replace the map's art as a group. `--level-swap-check` exercises 78 complete replacements with simultaneous old/new worlds, players, NPCs, particles, map art, Cheshire and power-up art. It also verifies per-draw power tint with 64 shader owners. This is a resource/renderer stress test, not a campaign traversal.

Relaunch the usual launcher to see the changes. Existing saves remain compatible; this update does not change collision, quests or the save format.

## Restored presentation

- **Skies:** original portal viewpoints and placed sky meshes now form the distant background, including the schools' stars and Wonderland Woods' sky. They follow camera rotation without sliding with Alice. Sky openings retain depth, so scenery beyond the opening cannot cover the sky and foreground walls remain opaque. Maps using cloud material layers receive a curved, camera-relative projection.
- **Camera portals:** linked `portal_surface`/`portal_camera` apertures render the remote room with independent visibility and an oblique near plane. Moving brushes carry the aperture and viewing frame; the main depth buffer preserves foreground occlusion. Visible apertures reuse a render target capped at 1,024 pixels wide. The Fortress school window includes its original pupil and walk path. This is a single-level view; recursive portals and mirrors remain unsupported.
- **Materials:** original surface normals drive environment reflections and view-dependent alpha. Vertex alpha, constant colours, cutout thresholds, equal-depth detail layers, noise waves and supported wave/move/bulge deformations are interpreted. Animated frame paths with a leading slash resolve correctly. World textures use repeating, trilinear mipmapped sampling to reduce distant shimmer.
- **Water and transparency:** transparent surfaces are kept separate instead of merging a whole map's water into one batch. World surfaces and environmental particles share a back-to-front pass, with triangle ordering inside each transparent surface. The original garden waterfall meshes and their layered materials now draw.
- **Particles:** steam variants, mushroom steam, supported fire, bubbles and tagged waterfall spray read their authored emission rates, offsets, orientation, velocity, acceleration, lifetimes, sizes, growth, fade and animated sprite images. Multiple emitters per model and TAN attachment axes are supported. Particles collide with the world, bubbles disappear outside liquid, and the population is bounded to 4,096 within a 3,000-unit emission range.
- **Fog:** additive effects fade to black, multiplicative layers to white, and doubled multiplicative layers to half intensity. These neutral blend colours prevent bright or dark patches in dense fog. Normal layers still approach the fog colour. Transparent effects continue to test the opaque depth buffer without replacing it.

Pause freezes environmental animation and emission. Persistent shared entity enable flags control supported emitters after restoration; completing the school theatre stops its steam without disabling other particle families. Individual cosmetic particles are rebuilt after loading, rather than saved.

Scenes share reusable GPU material pipelines and bind their own lightmaps before drawing. This keeps save restoration and level changes within the graphics library's fixed pipeline limit while the old and new scenes coexist.

## Reference and verification scope

Leading opaque `$lightmap` stages are retained as an atlas-lit white base when
the next drawable layer is detail or additive, instead of incorrectly treating
the whole surface as transparent. Ordinary lightmap-plus-filter surfaces still
combine into one diffuse draw. This restores About Face's clock floor and other
materials with the same layer ordering.

`--water-render-check` also isolates Hollow Hideaway's pond from its waterfall
and checks visible RGB changes over one second. The original `wavenormal`
deformation changes reflection normals, not the height of the water mesh; its
small authored amplitudes produce subtle ripples rather than geometric waves.

References are the locally supplied original `.shader`, `.tik`, TAN, BSP and declarative map-script data, read without running the original binaries. The audit finds 26 maps containing sky materials, 19 unambiguous portal viewpoints, 38 reflective map-material references and 22 supported emitter definitions. These are data-coverage counts, not a claim that every original sky sequence or material is reproduced.

`--fidelity-check` audits that data. `--fidelity-render-check` exercises the actual GPU material shader at the GT0, GE128 and LT128 cutout boundaries, verifies fog blend neutrality and sky-opening occlusion, and runs the existing wall/effect and first-person depth regressions. It also captures staged views of both schools, the garden, Pool of Tears, village and Wonderland Woods, and checks emitter pause, saved activation and expiry. These cameras are inspection fixtures, not traversal proof. `--fidelity-corpus-check` loads and draws every supplied map while retaining a second scene, checking both loading failures and exact restoration of its reference pixels after rebinding shared materials.

Logs and captures stay in `private/`; they contain original artwork and are excluded from the source-review archive. See [the verification record](VALIDATION.md) for the executed checks and results. No original-renderer runtime comparison was performed, so visual matching remains qualitative.

## Remaining fidelity work

- Reviewed Hedge camera switches and Hatter camera/miniature motion are supported. Other conditional camera scripts need level owners. Portal backdrop lighting remains approximate and drawn without fog; spline interpolation and callback timing are reconstructed from the supplied data.
- Cloud projection, procedural noise, billboard deformation and view-dependent alpha are independent approximations. Representative dynamic lights, flares and Alice contact shadows are implemented; full shadow coverage, other unsupported deformation types and all material sort conventions remain unfinished.
- Alpha sorting is approximate for intersecting surfaces. Supported world-view weapon effects, actors, pickups and environmental particles share a triangle-depth queue. BSP visibility/frustum culling is supported; order-independent transparency remains future work.
- The existing distance-fog curve and box-volume density remain approximate. Garden 2/4 trigger-driven fades are supported alongside literal distance settings; other conditional fog scripts need reviewed owners. The supplied eleven brush fog volumes fit the existing two-axis-aligned-volumes-per-map limit.
- Supported emitter families now include named animation switches, model attachments and light-only declarations. Arbitrary script-controlled activation and unsupported emitter commands remain deferred. Missing source images are reported rather than invented.

The renderer still needs work before a pixel-accurate comparison would be meaningful, but this update restores the major missing background and layered environmental effects available through the supported data paths.

# NPC beam effects

The generic chess cast, scripted chess encounters and Red King share the original
`queenbeam1` material, including additive blending and scrolling, with
`fx_lightning_hit` interception bursts. The supplied bishop and Red King TIK
`beamattack` commands explicitly name these assets. Collision endpoints, damage,
attack durations and saved state remain owned by the existing simulation.

Jabberspawn now use the supplied `emap1` lightning material, including the waves
owned by the Jabberwock encounter. This is a compatibility fallback: their TIK
omits the material argument, and the native `FuncBeam` default `beamshader` is not
present in the supplied archive material definitions. It is not a claim of exact
recovery of that default. The Jabberwock eye attack uses the existing Eye Staff
`fx_beam` model rig: its original `eyebeamattack` handler uses the same native
effect controller. Ghost tethers and spider webs are outside this change.

`--chess-check` checks the archive references, textures and blend modes.
`--wildlife-render-check` additionally checks visible GPU pixels, animation,
repeatability at the same saved time and solid-cover occlusion before capturing
the creatures. `--rchess1-render-check`,
`--jlair2-render-check` and `--grounds1-render-check` capture affected NPC and boss
attacks. These are staged visual fixtures, not full-route playthroughs.

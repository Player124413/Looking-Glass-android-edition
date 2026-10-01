# Easter egg and secret audit — 2026-10-01

The identified Easter egg and the tested secrets are already present in build
**92CC824C**. This audit found no missing feature to port. No gameplay code,
selected executable, original assets or player saves were changed.

## Findings

| Content | Result | Evidence |
| --- | --- | --- |
| Pool of Tears fork-shaped symbol, reported as an Alice/Thief connection | Present in the original `potears1` world geometry and visible through the port's production scene renderer. | Native diagnostic capture; see location below. |
| School Looking Glass secret | Already implemented. The shootable face respects occlusion, opens its panel, and awards the unique 45-second power-up without replaying the reward or changing the recipe gate. | Selected release: `--progression-check`, passed. |
| Caterpillar's Plot return-visit Humpty passage | Already implemented. Staff-only wall damage, saved partial damage and the Humpty passage passed the return-visit check. | Selected release: `--wforest-return-check`, passed at 30/60/144 Hz with watched/skipped scene cases. |
| Pale Realm hidden breakable passage | Already implemented. A real combat trace reaches the target from the accessible side; the wall requires its damage threshold, disappears once, and stays removed after restoring state. | Private diagnostic using the selected release's unchanged interaction/controller code, passed. |
| Original cheat commands | Already supported. God, Wuss, Give All, individual items and saved cheat state passed. | Selected release: `--console-check`. The reference website explicitly labels its cheat entry as not an Easter egg. |

The Castling manual secret door and the secret teleport connections in
`funhouse` and `qlair` were also inspected in the map data and implementation.
Their handlers exist; this audit did not perform a continuous player traversal
of those specific secret routes. The general `--wchess2-check` passed, but is
not claimed as proof of manually opening its secret door.

## Pool of Tears symbol

The [user-supplied listing](https://eeggs.com/tree/3974.html) links to the
[Alice–Thief connection report](https://eeggs.com/items/38652.html). The
distinctive four-pronged silhouette matches the rendered map object. The
reference's interpretation of its meaning is not needed for the porting result.

Technical locator for future regressions:

- Map: `potears1`, world model 0.
- Geometry bounds: `(1792, 2144, 2352)` to `(1904, 2160, 2464)`.
- 54 connected surfaces, starting at surface 4573; material `textures/enlarged/bark3_1`.
- Inspection camera: eye `(1848, 1984, 2412)`, target `(1848, 2152, 2408)`.
- Capture: `private/easter-egg-audit/captures/pool-of-tears-symbol.png`.

The capture uses a staged inspection camera and the production scene draw
path. The diagnostic omits the sky and actor passes; its black background is
not a gameplay screenshot or a newly identified sky defect. Foliage partially
overlaps the symbol at this angle. No replacement geometry was added.

## Reproducibility and limits

The selected release SHA-256 is
`92cc824c3d1fbe9b91ba60e29b8d7ffccf406a2eda63d47621d4e8a2bab28ca1`.
Its frozen source is recorded in `private/presentation-polish/release.json`.
The audit fixture changes only the diagnostic entry point and adds an audit
module; renderer, assets, collision, combat and level controllers are unchanged
from that snapshot. The fixture is not a published game build.

Logs, view coordinates, retained fixture source and hashes are under
`private/easter-egg-audit/`, with `verification.json` recording provenance.
Native checks ran in the background Windows desktop without audio. This is a
targeted audit of the supplied reference and identifiable secret interactions,
not a claim that every obscure background reference in all maps has been found.
Unplaced asset names alone were not treated as evidence of playable Easter eggs.

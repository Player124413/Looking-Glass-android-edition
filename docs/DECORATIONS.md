# Original map decorations

The September 2026 placement audit restored previously omitted scenery across 29 maps. After excluding cinematic props and the transport-owned Fortress airship, the scene loads 1,251 environmental model placements in total, including the existing sky models, waterfalls and lanterns.

Restored scenery includes giant flowers, mushrooms, grass walls, roots, branches, kelp, hanging and wall lamps, torches, classroom chairs and desks, clocks, shelves, banners, busts, statues, urns and small display airships. Both school sky book stacks now pass through the normal scene loader with their original sway animation.

## Placement and rendering

- Placements come from the mounted original BSP entities: position, scale, pitch/yaw/roll and difficulty flags. TIKI setup offsets are applied too; the root/mushroom offsets are necessary for the correct ground height.
- The original model definitions, vertex/skeletal animation frames and material layers are loaded from the user's read-only archives. Animation frames are shared between instances. Nothing is extracted into a replacement game-data package.
- Six decorations attach to existing inline room movers. Their local poses follow the same room transforms and visibility as the rendered room.
- Twenty-four compiled placements are skipped. Duplicate detection checks material **and matching vertex positions**, including inline room geometry; finding another instance of the same material elsewhere does not suppress a prop.
- The original root and kelp meshes contain surface names absent from their TIKI skin lists. Where all declared surfaces use one image, the missing surface uses that sole image. Multiple different skins are never guessed between.
- Existing pickup, puzzle and transport renderers retain ownership of their objects. The pass adds presentation; original map collision and existing gameplay controllers remain responsible for collision and interaction.

Script checks follow archive `#include` references, including cinematic setup, with cycle protection. The first school's Gnome pipe is initially hidden and attached to `tag_pipe`; it must never render as scenery at its editor position. This also excludes a scripted school airship and a Garden 3 character's attached marble.

## Scope still pending

Forty candidate placements remain deferred because their visibility, animation selection, binding or movement depends on an unfinished script controller. Examples include free pendulum lights, a falling-pillar mushroom, conditional Funhouse/chess/temple props and Tower blowfaces. Falling rocks, traps, emitters, actors and pickups are separate gameplay systems, not static decoration imports. This is not a claim of complete campaign or cinematic fidelity.

## Verification

`--decorations-check` audits all 36 original maps, loads every accepted model/animation, checks finite geometry and resolves/decodes all 108 referenced images. It prints each accepted and deferred placement plus per-map totals.

`--decorations-render-check` creates staged before/after captures through the production scene renderer. Its cameras inspect school furniture, garden vegetation, a Hatter bust and an attached Fortress light, and require a visible pixel difference. These are presentation fixtures, not traversal tests. `--fidelity-corpus-check` separately loads and draws every map while keeping a second scene resident to check shared rendering resources.

Unit checks cover scripted-call parsing, model setup/visibility scope, full Euler orientation, parent-relative attachments and material/spatial duplicate matching. Existing save formats are unchanged; relaunch the game to load the restored placements.

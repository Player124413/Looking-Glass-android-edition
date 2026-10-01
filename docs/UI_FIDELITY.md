# Original-art interface — v0.31

The interface now shares the original archive fonts and decorative artwork across menus and play. The normal launcher uses this version; existing preferences and format-10 campaign saves remain compatible. Modern display choices and the 39-visit chapter order remain available.

## What changed

| Surface | Original reference used |
| --- | --- |
| Main menu, Settings, New Game, Quit and Credits | Existing original URC layouts, backgrounds, highlights and animated mirror |
| Settings controls | Original checkbox, slider, arrow and Apply/Cancel artwork; Asrafel lettering |
| Continue, confirmations, help, chapters, inventory and death | Original `ui/dialog/leftframe` and `rightframe` parchment frames and Asrafel font |
| Dialogue, interaction prompts and temporary notices | Original `gfx/2d/talkmenu` frame; original Verdana subtitle font |
| Pause | Original transparent `ui/pausewatch/pausewatch` artwork |
| Developer console | Original `gfx/2d/backtile` texture and Courier-14 bitmap font |
| Pointer | Original `gfx/2d/mouse_arrow` at its authored aspect ratio |
| Sanity, Will and toy cards | Original HUD TAN geometry, animation frames, texture coordinates and textures |

The new shared renderer reads the Asrafel, Verdana-12 and Courier-14 ritual fonts. Wrapping measures those glyphs, and punctuation maps to characters the original fonts support. Shared assets remain alive through menu and level changes without creating new material pipelines.

The old prototype headings, permanently visible instruction strips, flat coloured inventory cells, generic font overlays and fabricated meter rectangles are removed. **H** still brings up temporary help, **I** opens inventory, **Tab** opens chapters and **P** pauses. Existing gameplay, key bindings and progression rules are unchanged. Help includes the configured interaction and Cheshire keys.

The HUD now projects the authored bar, back and riser meshes. The riser's original 101-frame animation selects the visible liquid level from the current resource value. Toy cards use the open folding mesh, so unused opaque areas of their texture sheets no longer appear as black rectangles. Inventory still shows exact resources and ownership.

Load/Save places all six working slot photographs behind the original ornate frames, with brass selectors, camera shutters, the animated sepia/scratch layer and projector/button sounds. The camera's plates show chapter/date information and the focused action. Four manual slots, quick save and autosave retain their existing behavior, including legacy/no-image fallbacks and overwrite/load confirmation. See [MENUS.md](MENUS.md). Continue uses the original main-menu scene behind its choices. No new original-game assets are included in the source package: all are read from the user's local archives.

## Fidelity boundaries

This is an original-art adaptation, not a claim of pixel-identical original-engine rendering. Help, chapter selection, the expanded inventory, modern settings and the six-slot save flow contain remake-specific content, arranged using original frames, lettering and controls. They do not reproduce an original screen that had exactly those functions.

The HUD uses a fixed projection and the open card pose; the original perspective, folding transitions and additive scrolling/pulsing liquid shader layers are not yet reproduced. Pause is static. Decorative transitions and sounds outside Load/Save, save deletion and the full original multi-page save browser remain unfinished. World pickup visuals and other unfinished world rendering are outside this interface pass.

## Verification

Native Anode input checked the final layout at 800×600 windowed and 1280×720 fullscreen: settings, help, chapter selection, inventory, pause, console, dialogue, save/load confirmations and Continue after restarting. Mouse selection still equips owned toys. An explicit HUD preview checks partial resource fills; normal starts use the existing loadout. Actual 4K output and non-100% DPI were not available on the test desktop.

All 164 existing unit tests and strict Clippy pass. The release executable and source-review package are rebuilt. Captures, test saves, settings and extracted reference artwork remain under `private/`; the player's save slots were not used. See [VALIDATION.md](VALIDATION.md) for the evidence paths.
# Presentation polish — October 2026

Gameplay meters, encounter labels, dialogue, power timers and retry/pause prompts
scale for high-resolution displays. Original artwork and meter/icon attachment
remain intact. Notices wrap within the screen instead of compressing an entire
message into one tiny line; longer notices remain visible for longer. Pickup and
status notices sit below encounter meters. Chapter transitions show the visit's
display title, including return visits, and the application title is Looking Glass.

The opt-in native layout fixture runs with `--frames 1` and
`LOOKING_GLASS_PRESENTATION_CHECK` set to a capture directory. It draws gameplay,
dialogue, pause and death layouts at 640×480, 1200×680, 1920×1080 and 3840×2160.
It requires a sufficiently large test desktop and does not load preferences or
touch saves. Gameplay rules, dialogue clocks and save fields are unchanged.

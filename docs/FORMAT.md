# Format and provenance notebook

These notes describe observations, not a recovered original source tree. The Rust implementation was authored from archive layout observations, arithmetic on the supplied files, and general graphics techniques. Ghidra was then used to confirm selected constants and module relationships. No decompiled function body was translated into the Rust source. This is not a formal clean-room claim.

## Identified input

The supplied `alice_202106_meta.xml` describes the 2011 remastered release. An embedded engine build-date string reads 5 May 2011. SHA-256 fingerprints and archive counts are recorded locally in `private/inventory.json` by `tools/inventory.py`.

The Windows executable is 32-bit x86 and names `cgamex86.dll` and `fgamex86.dll`. The similarly named `cgame.dll` and `fgame.dll` have PE machine value `0x1f2`, unlike the Windows files' `0x14c`. Ghidra's automatic import misidentified those two other binaries as 16-bit x86 and recovered only one function each: **those analyses are invalid and must not be used as evidence**. They are not needed by the current viewer. Do not select them just because their filenames are shorter.

The supplied `renderer.lib` is also preserved for future analysis; it has not been incorporated or linked into Rust.

Version 0.4's observed SKB/SKA layouts, TIKI setup subset and character-specific Ghidra evidence are recorded in [CHARACTER.md](CHARACTER.md).
Version 0.6's observed TAN v2 layout and weapon attachment/event subset are recorded in [WEAPONS.md](WEAPONS.md).
Version 0.8's liquid contents, original swim-state/clip observations and the OpenGL depth-state fix are recorded in [SWIMMING.md](SWIMMING.md).

## FAKK map container

All 36 mounted maps use little-endian `FAKK`, version 42. Header size is 172 bytes: four-byte magic, version, checksum-like value, then 20 offset/length pairs beginning at byte 12. The checksum field is retained as an observation; this prototype does not validate its algorithm.

Implemented lump interpretations:

| Index | Contents | Record size |
| --- | --- | --- |
| 0 | Material name, flags and contents mask | 76 bytes; name occupies first 64 |
| 1 | Collision planes | 16 bytes |
| 2 | RGB lightmap images | 128 × 128 × 3 bytes per image |
| 3 | Render surfaces | 108 bytes |
| 4 | Vertices | 44 bytes |
| 5 | Surface-relative indices | 32-bit integers |
| 10 | Brush sides: plane and material indices | 8 bytes |
| 11 | Brushes: side range and material index | 12 bytes |
| 12 | Fog: material name, brush index, visible-side index | 72 bytes |
| 13 | Brush models; first is the world | 40 bytes |
| 14 | Quoted entity key/value dictionaries | Text |

Vertex offsets: position 0, diffuse UV 12, lightmap UV 20, normal 28, RGBA bytes 40. Normals are present but unused by this initial renderer.

Surface offsets: material 0, kind 8, first vertex 12, vertex count 16, first index 20, index count 24, lightmap index 28, patch width 96, patch height 100. Surface kinds 1 and 3 use explicit triangles; kind 2 uses quadratic patch control grids. Other kinds are not rendered. A model's first surface and count are at offsets 24 and 28.

A model's first brush and count are at offsets 32 and 36. Material flags/contents are at 64/68. See [MOVEMENT.md](MOVEMENT.md) for collision details and the additional narrowly scoped Ghidra observations made for version 0.2.

Version 0.7 retains and validates every model's bounds and surface/brush ranges, allowing matching transforms for rotating-door rendering and collision. Fog records use a 64-byte material name followed by two signed 32-bit indices at offsets 64 and 68. Brush/side references are checked; the visible-side index is not used by the approximate volume renderer. All eleven supplied fog brushes have axis-aligned planes. See [WORLD.md](WORLD.md) for the supported subset and provisional behaviour.

The reader rejects truncated/overlapping lumps, negative ranges, invalid references, non-finite coordinates and malformed patch dimensions. It accepts only the observed version. Lightmap appearance and material semantics remain approximate even though the binary structures validate.

## Textures, archives and material files

FTX files in the supplied mounted data contain three little-endian 32-bit header values followed by RGBA bytes. The first two values are width and height. The third value's semantics are not asserted; decoding here requires exactly `12 + width × height × 4` bytes. All 2,953 mounted FTX entries pass that condition. Dimension and decoded-size limits prevent uncontrolled allocation.

The archive reader uses ZIP without extracting entries to disk. Names are case-insensitive and backslashes are normalised. Top-level packs are mounted in alphabetic order; later entries override earlier ones. This precedence is an implementation choice consistent with this pack naming scheme, not yet a differential proof of every original search-path rule. Loose overrides and `loc/` are not mounted.

Material image resolution considers `map`, `clampmap`, `animMap` images, and editor-image fallback. FTX counterparts are preferred to TGA references. Version 0.7 preserves declarative stage boundaries and line-ended animation lists, rendering a bounded subset of blending, texture transforms, waves and detail layers. Alpha/additive geometry is sorted by batch after opaque geometry. Unsupported stage directives do not imply support for their original effect. Sky and nodraw surfaces are omitted; the sky background is still a plain colour. Authored fog settings and steam definitions are read as data, without executing map or TIKI scripts. See [WORLD.md](WORLD.md).

## Ghidra evidence

Portable Ghidra 11.4.2 with JDK 21 ran locally. `tools/analyze.ps1` creates `private/ghidra/AliceResearch.gpr`, plus function lists and string-reference reports. The three usable Windows analyses recovered:

| Program | Functions identified | Relevant string cross-references |
| --- | ---: | ---: |
| alice.exe | 5,394 | 55 |
| cgamex86.dll | 1,940 | 2 |
| fgamex86.dll | 9,237 | 126 |

These are tool-detected counts, not a claim that every function is correctly delimited, named or understood. Some analysis warnings remain in the logs.

Three selected engine functions were decompiled locally using `AliceInspectFunction.java`:

- `0x0040df30`: collision map loader, identified through its diagnostic string. Confirms a 172-byte header and version 42 check.
- `0x00491e80`: lightmap loader, identified through lightmap-name/debug references. Confirms 49,152-byte blocks and 128-pixel dimensions. The original lighting transform has not been reproduced.
- `0x00464940`: game-module loader. Confirms loading `fgamex86.dll` and resolving `GetGameAPI`.

Only facts needed to reason about compatibility belong in this notebook. Raw pseudocode remains under `private/analysis/functions` and is excluded from source packaging. The current Rust viewer does not call any original DLL.

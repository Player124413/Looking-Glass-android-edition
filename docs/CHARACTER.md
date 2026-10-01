# Alice's model and animation — version 0.4

Current body-animation coverage also includes the restored idle routines, dialogue gestures, pain reaction and equipment-dependent movement described in [ANIMATIONS.md](ANIMATIONS.md). The six-clip description below is historical.

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

The notes below record the original character milestone. Version 0.6 extends it with armed locomotion, named weapon attachments, equip/action clips and an upper-body action layer; see [WEAPONS.md](WEAPONS.md) for current weapon behaviour.

Version 0.11 adds a separate first-person weapon presentation, using camera-relative toy models and action timing while the full character body is hidden. It has no separate hand mesh. Third-person skeletal animation remains the original body presentation.

The viewer reads the user's `models/alice.tik`, `alice_baseframe.skb`, normal skin textures and six original SKA clips directly from the mounted archives. It does not extract or redistribute them. No original game executable or DLL is called.

## Playable behaviour

Third-person view is the default while walking. The mouse orbits Alice; WASD movement is relative to the view, and her body turns toward travel. V switches between third-person and the existing first-person view. Free flight stays first-person; returning to walking restores the selected view.

The current states are idle (`idle_base_01`), walk (`walk_nowep`), run (`run_nowep`), takeoff (`jump_small_takeoff`), airborne (`jump_air_03`) and landing (`jump_small_land`). The controller selects them from actual velocity, ground contact and jump/landing counters. Moving interrupts the landing recovery. Local bone poses interpolate between source frames and crossfade for 120 ms between states; paused gameplay and menus freeze the pose. Resets and level changes reset animation state.

The follow camera sweeps a four-unit box through the static collision world. Obstructions shorten its arm; very close walls also lift it to frame Alice from above, with collision checks on both the lift and sightline. The skin is hidden if the space is too cramped to keep the camera outside her body. Camera collision does not include future moving doors or actors.

Locomotion rate responds to velocity, capped at 2.5 times the source rate. Original animation delta motion does not drive the physical controller. The earlier provisional speeds and body collider are retained, so foot sliding and occasional skin clipping in tight spaces remain possible. Takeoff starts partway through its source clip to fit the controller's immediate jump. Root motion, exact original blend rules, animation events, scripted gestures, weapon poses/attachments, combat, facial changes and cinematics remain unfinished. Rendering uses the normal intact skin, alpha cutoff and approximate directional lighting; damage cap surfaces and alternate skins are not active.

## Observed binary layouts

Only the observed version 3 layouts are accepted. All integers/floats are little-endian. Range, count, topology, finite-value, quaternion, weight and index checks precede use.

**SKB:** `SKL ` magic; version at 4; name at 8 (64 bytes); surface/bone counts at 72/76; bone/surface offsets at 80/84; end offset at 88. The header is 92 bytes. Each bone is 72 bytes: parent index, flags, 64-byte name. Alice has 131 bones; parents precede children, and -1 denotes a root.

Each surface has a 96-byte header: magic, 64-byte name, triangle/vertex/minimum-LOD counts at 68/72/76, then relative triangle, vertex, collapse-map and end offsets at 80/84/88/92. Triangles are three 32-bit indices. A variable-size vertex stores a float normal, float UV, weight count, then weights of 20 bytes each: bone index, float weight and float bone-local position. Weights are checked to sum approximately to one and normalised. The collapse map is range-checked but full-detail geometry is used. The 19 source surfaces include three dismemberment caps, leaving 16 drawn surfaces for intact Alice.

**SKA:** `SKAN` magic; version at 4; flags at 72; frame/bone counts at 76/80; duration/frame interval at 84/88; total delta vector at 92; frame offset at 104. Each frame begins with 40 bytes: bounds, radius and delta vector. Each bone then has eight signed 16-bit values: quaternion XYZW, translation XYZ, padding. Rotation components divide by 32767 and the quaternion is conjugated to match glam's convention; translation divides by 64. Every rotation is normalised on load; a rotation more than 0.02 from unit squared length is rejected, except on an unskinned `tag_*` attachment bone, where a squared length between 0.5 and 1.5 is accepted and reported so the model can be named explicitly (`OFF_NORM_TAGS`, NPCS.md). Parent transforms accumulate before weighted skinning. Per-surface normals are reconstructed from animated triangles for the approximate lighting.

Alice's TIKI setup supplies model path, scale and surface shaders. The first shader per surface selects the normal skin; subsequent variants are not silently treated as the default. This is a narrow setup reader, not a complete TIKI command interpreter.

## Evidence and checks

Offsets and arithmetic were derived from the local files. In the existing read-only Ghidra project, `alice.exe` function `0x00431990` confirms SKA version 3 and bone/frame count offsets 80/76. The older loaders at `0x00437d00` and `0x00438010` require version 2 and are not evidence that the supplied files use version 2. Raw function output stays in `private/analysis/functions`; no decompiled function body was translated into Rust.

`--character-check` decoded and skinned all **222 Alice clips / 8,688 frames**. Every output vertex was finite. The largest excursion outside a frame's stored bounds was **0.676 units**, below the four-unit validation tolerance. This checks transform consistency across the corpus, not original-versus-Rust behavioural fidelity. Only six clips are currently connected to gameplay. The log is `private/character-validation.txt`.

Synthetic fixtures cover truncated and malformed files, bad indices and parent cycles, hierarchical transforms, interpolation/loop/clamp boundaries, quaternion sign equivalence, movement-state transitions, pause/reset and camera collision. Run:

```powershell
cargo test --locked
cargo run --release --locked -- --character-check
```

Game artwork, animation files and visual captures are excluded from the source-review package. The existing local-use and publication notes still apply.


Version 0.23 connects the original rope-hang, minecart-seat and airship-seat clips in Pandemonium (22 locomotion clips total). Mounted poses suppress weapon actions. Alice and the Elder Gnome use the airship skeleton's original attachment tags; rope motion is an independent constrained swing rather than an exact reproduction of the original rope simulation.


C3 additionally reads the observed version-2 Queen second-form assets (`c_q2_body.tik`). SKB uses the same bounded header, bone and weighted-surface layout. SKAN v2 stores frame/bone counts at 72/76, duration/frame time at 80/84, delta at 88 and frame offset at 104 (minimum 112). Each frame has the same 40-byte bounds/delta prefix followed by seven float components per bone: quaternion then translation, 28 bytes. Quaternion conjugation matches v3; float components are finite and rotations retain the same unit-length checks. Legacy export-size words are not trusted: the actual payload must exactly fit the bounded frame count and stride. V3 offsets, packed-short decoding and validation remain unchanged. Synthetic cross-format pose tests and the local Queen body clips exercise this reader.

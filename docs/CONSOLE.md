# Console and Cheshire — version 0.31.7

Current facial support: original lip-sync envelopes and Alice’s blink texture are now connected for implemented dialogue. See [facial animation, verification and remaining limits](FACIAL.md). Earlier facial limitations below describe previous milestones.

Relaunch the usual launcher to use the rebuilt executable. **~ / backtick** opens the developer console. **C** summons Cheshire while playing. **H** retains the existing temporary controls and objective panel.

## Developer console

The console pauses the game and releases the mouse. Typing is exclusive: W does not move Alice, I does not open inventory, and Home edits the command line instead of moving Alice. Close with the same key or Escape; an already paused game stays paused. The console is enabled by default in this Rust build.

Enter executes a command; Up/Down browses history; Tab completes command names; Left/Right, Home/End, Backspace/Delete edit the line; Ctrl+U clears it. Page Up/Down scrolls output. F12 captures the console. Console history lasts for the current session. Fast typing preserves character order, including multiple characters received during one frame. Save files retain god mode, notarget, granted inventory, health, view mode, free flight and inspection lighting. Camera distance is stored with the options.

| Command | Effect |
| --- | --- |
| `help` or `cmdlist` | List the supported commands |
| `clear` | Clear console output |
| `version` | Display the Rust runtime version |
| `status` or `getpos` | Show map/entrance, feet coordinates, view angles, resources, cheats, weapons and school objective |
| `maplist` | List available maps in the local archives |
| `map skool2` | Start a fresh visit to a map, resetting its quest and developer cheats |
| `map skool1$skool1_start2` | Start at a verified named entrance |
| `restart` | Recover at the current entrance with resources restored; keep this visit's quest progress |
| `noclip` | Toggle the existing free flight mode; leaving flight requires a clear body position |
| `god` | Toggle immunity to Sanity damage and free weapon-energy use, including attacks when Will is zero |
| `notarget` | Toggle enemy targeting of Alice; already released projectiles and environmental hazards still work |
| `wuss` | Give all ten toys (including three Dice) without changing Sanity or Will |
| `health 100` | Set Sanity directly; finite values from 0 to 1,000,000 are accepted. Zero kills; a positive value can revive Alice |
| `cg_cameradist -45` | Enter first-person view, with the existing first-person weapon presentation |
| `cg_cameradist 128` | Return to third-person view at distance 128 |
| `cg_cameradist` | Report the current camera setting; positive values up to 1024 select third person, zero/negative values select first person |
| `give all` | Give all toys and the Turtle Shell, and set Sanity/Will to 100, as declared in the original give-all data |
| `give weapons`, `give health`, `give will` | Weapon-only alias for `wuss`, or set the indicated resource to 100 |
| `give w_knife.tik` | Grant the named original inventory item; an individual weapon grant also refills Will like its pickup |
| `itemlist` | List usable original item filenames available in the mounted archives |
| `fullbright` | Toggle inspection lighting |
| `cheshire` | Request the same hint as C and close the console on success |
| `save` or `save quick` | Write the quick-save slot, also available with F5 |
| `load` or `load quick` | Restore the quick-save slot and pause, also available with F9 |
| `load auto` | Restore the automatic slot and pause |
| `quit` | Save automatically when alive and close the game cleanly |

Commands are case-insensitive and accept the traditional leading slash or backslash. Unknown commands, invalid arguments and missing maps/entrances report an error. The console implements these commands in Rust; it does not execute arbitrary OS commands, original console scripts, native DLLs or every original engine variable. `give` and `wuss` require Alice to be alive; use `health 100` or retry after death. Noclip freezes ordinary simulation, like F4. Map changes start fresh visits; normal exits preserve the current run's inventory and targeting/god settings.

### Item filenames

The ten toy filenames are `w_knife.tik`, `w_cards.tik`, `w_mallet.tik`, `w_jackbomb.tik`, `w_icewand.tik`, `w_jacks.tik`, `w_demondice.tik`, `w_eyestaff.tik`, `w_blunderbuss.tik` and `w_watch.tik`. The command grants inventory; it does not add missing attack implementations for unfinished toys.

Resources: `p_h1.tik`, `p_h2.tik`, `p_m1.tik`, `p_m2.tik`, and `w_me_small.tik`, `w_me_medium.tik`, `w_me_large.tik`, `w_me_super.tik`. Amounts come from the original definitions. Power-ups: `w_ragebox.tik`, `w_grasshopperteaitem.tik`, `w_lookingglass.tik`; these activate immediately under the existing duration/non-stacking rules. `w_turtleshell.tik` grants the breathing upgrade.

School inventory items: `tart.tik` (Mushroom), `beaker.tik`/`beaker01.tik` (Jumbogrow), `lollypop.tik`, `star.tik`, and `beaker02.tik`/`beakerhand.tik` (Drink Me). Grants are synchronized with school inventory and do not complete unrelated combat or dialogue stages. Giving both final rewards during the reward stage satisfies the usual inventory gate. World pickups are not consumed by console grants. Enemy weapons, decorative props and other models without a supported Alice inventory use are rejected explicitly.

An optional `models/` or `models\` prefix and quotes are accepted, e.g. `give "models/w_cards.tik"`. Names are case-insensitive. `notarget` covers the implemented guards, Boojums, Ladybugs, Duchess and hostile summoned demons; independent patrols and attacks on other characters may continue. Disabling it restores normal target acquisition. God mode does not remove attack animations, cooldowns or Pocket Watch recharge.

`--console-check` validates archive-backed item grants, all declared attack costs under God, resource-preserving Wuss, camera selection, school reward gates and saved cheat flags. Unit tests cover parsing, invalid values, over-health, Boojum pursuit/attack cancellation and hostile demon targeting. Wider original engine commands/cvars remain outside this gameplay-command update.

## Cheshire Help

C uses the original `cheshire` binding. The summon is available when Alice is alive and outside menus/free flight. It waits until the current conversation has finished, and has a short cooldown after departure. Cheshire faces Alice and uses his original sitting/talking model and recorded line. E advances it; P, inventory and the console pause it. The existing dialogue system pauses enemies during speech while allowing exploration.

The latest crossed supported `trigger_catmessage` region supplies the contextual recording. Without one, eleven original general replies are cycled. All maps can summon him; **41 regions and 42 unique hint recordings** are available in this build. The three second-school regions select their original recordings. Pool also supports its three named before/after-Turtle regions; the existing Pool owner switches them when the conversation starts, and loading immediately restores that condition. Existing automatic story hints continue to work independently, and replaying a summoned hint never emits a quest completion or grants ingredients.

Placement checks floor support, body clearance, water and line of sight. If no suitable nearby position is found, or Alice is swimming, the voice/subtitle still plays without placing a body in a wall. New summons fade at fixed body size over two seconds, start speech at 2.5 seconds, and fade away after the recording. Mouth movement follows the supplied lip track and saved dialogue clock. The renderer uses stippled visibility as its alpha-fade approximation. Voice-only hints have no body appearance/departure sounds. Named hint regions in unfinished maps that require script activation/replacement are deferred. The general reply remains available there. This does not complete those maps' progression scripts.

## Evidence and verification

The local `default.cfg` binds backtick/tilde to `toggleconsole` and C to `cheshire`; the Alice model defines eleven `cat_snide` aliases. Map hint regions, subtitle tables and Cheshire's model/animation data remain in the user's archives. Read-only Ghidra inspection of `fgamex86.dll` at `100c0fc0` identifies `CheshireCatHelp`, its local model placement search and voice-only fallback. The Rust implementation is independently written; decompilation, transcripts, recordings and captures stay excluded from packaging.

`--cheshire-check` loads every map's supported hint data, fully decodes 42 hint recordings and two body-effect recordings, and verifies all three second-school region selections and that their summoned playback cannot complete quest events. Unit tests cover explicit command parsing, rejected chained input, bounded history/text/output, hint pause/cooldown/busy behaviour, placement obstruction, and opt-in god mode. Pool checks additionally exercise all three named brushes at 30/60/144 Hz, old index selections, owner gates, watch/advance/pause/load and repeated summons without quest commits. `--cheshire-render-check` captures six Pool presentation fixtures; the six `cheshire-pool-` save cases verify separate-process continuation. See [CHESHIRE.md](CHESHIRE.md) for the shared binding and save contract. Native Anode tests cover actual typing, completion/history, command execution, exclusive input, map changes, C and console summoning, rendering and paused subtitles. Native tests were silent; voice decoding and sound wiring are verified, but no new listening comparison is claimed. See [VALIDATION.md](VALIDATION.md).

# Campaign level specs

Per-visit implementation specs for [CAMPAIGN_PLAN.md](CAMPAIGN_PLAN.md). Written **2026-09-28** from verified read-only research of the user's data and the code at `main` `73e4165`; revised **2026-09-29**. The finale sections v35-v40 were added in that revision as drafts ([Finale](#m8-finale)). The canonical copy of this file lives on `campaign/integration` (CAMPAIGN_PLAN.md header and P0.2b).

> **Anode is mandatory.** Every window-opening check named in these specs runs inside Anode's hidden desktop through the `seat_*` tools, never on the user's desktop. That covers every `--*-render-check`, `--visibility-check`, `--level-swap-check`, `--save-check-write/read`, `--frames`/`--capture`, `--movie`, previews, normal play and the `tools/*.ps1` suites. Follow the protocol in CAMPAIGN_PLAN.md §12. Only the recipe-13c agent runs these checks (CAMPAIGN_PLAN.md §12.3); never the computer-use tools, the Browser pane or the user's desktop. Wherever an Anode checklist below says "real input" for a timed action (a jump or catch from a moving rider, a chase, a dodge) or a whole boss fight, read it as `--route-replay` plus a real-input segment of at least 30 s (CAMPAIGN_PLAN.md §12.6 latency rule). The seat reports no audio device, so audible output is never claimed from it.

## How to read these specs

- There is one section per visit, in campaign order, under a stable anchor from `#v00-new-game` to `#v40-ending`. Visit numbers are 1-based; the `ROUTE` index is the number minus 1.
- **Standard obligations are not repeated in each section.** They are the recipe in CAMPAIGN_PLAN.md §10, the common exit criteria in §7.0, and the per-visit Anode protocol in §12.6. Each section lists only the visit-specific work, checks and captures.
- **Provenance.** Everything below is identifiers, counts, coordinates, timings and paraphrase. The **Refs** lines name the scripts, threads and entity numbers to re-read locally with `python tools/alice_data.py cat|ents|summary|grep ...`. Never copy script bodies or dialogue or subtitle text into the repository. Voice-file ids such as `catz504` are identifiers.
- **Entity numbers.** `#N` is the entity index in the BSP entity lump as printed by the helper. `*N` is an inline model number.
- **Line numbers** in Rust files are as of 2026-09-28 and drift. Re-resolve them before editing. Research line refs that sit in files other sessions were editing (`duchess.rs`, `npc.rs`, `encounters.rs`, `viewer.rs`, `combat.rs`) are marked with `~`.
- **Track task ids** (W*, E*, B*, T*, C*, R*, F*) refer to CAMPAIGN_PLAN.md §5 and §6. **DG-n** are decision gates (§14.3).
- **Status codes:** OK, RED, PARTIAL, BLOCKED, BYPASS, UNPROVEN (CAMPAIGN_PLAN.md §2).
- **Cinematic "MVP treatment":** the minimum for PLAYABLE. That means authored camera tracks or statics, puppets for speaking or moving actors, beats with subtitles, every world change, and an identical commit on hold-Enter skip (DG-3, DG-4). Everything beyond that is under **Fidelity follow-ups**.
- **Check names.** Each visit's check flags are listed in CAMPAIGN_PLAN.md Appendix E-4. The legacy visits keep their existing flag names (for example `--pool-check` or `--school2-route-check`). Never construct a flag from a map name.
- **Ownership.** Visits served by a legacy typed controller (01-08, potears1's Pool and potears3's Duchess) are extended inside that legacy module, with a gated-trigger migration for every changed gate (CAMPAIGN_PLAN.md F1.4a). The reservations still apply to their new IDs, facts and save cases.
- **Research sources.** The verified planning research behind these specs, including the opening audit and each group's corrected spec with its correction log, is in `C:/DEV/McGee/private/campaign-research/research/` (local only; CAMPAIGN_PLAN.md §0.2 item 6). These specs and CAMPAIGN_PLAN.md win any conflict. Use the research files for detail and re-verify it against the data.
- **Staged saves.** "From a staged save" in an Anode checklist means a save case written by `--save-check-write` and loaded with `--load quick` from a copy of its folder (CAMPAIGN_PLAN.md §12.6). If an item names no case, add one to the visit's save cases first.
- **Unresolved choices.** Where a section says Decide or Choose, apply these rules and record the outcome as a reviewed deviation in `docs/<MAP>.md` and as an agent decision in CAMPAIGN_PLAN.md Appendix G:
  - keep the authored reachability and commit state, and commit the watched path's end state;
  - prefer faithful behaviour unless it can strand Alice or break save/restore; then use the minimal safe deviation;
  - stop and ask only if the choice changes an exit, a gate, a reward or the save format;
  - list every such decision in the milestone exit report.

  Recommended defaults, to be confirmed in that report:
  - v12: `oyster4` cycles like `oyster3`, without a block;
  - v15: the marble does not collide with the gates, and each forward gate latches open once the marble has passed;
  - v17: one 4 s delay on both paths, and no `ant_at_end` spawns on either path;
  - v18: neither path refills (`full_stats`);
  - v19: `cat_message1` is enabled on both paths;
  - v20: `knight_11_13` is a faithful crusher death with the authored reset;
  - v22: the guillotine blade rests at its dropped pose;
  - v23: the secret teleport #12 stays authored, and the route does not use it;
  - v25: the Hatter stays dormant until the `hatter_cat` beat ends, and a death at `tower_up` plays the malfunction there;
  - v27: a retry restarts at the intro end;
  - v29: Hold honours only the kid's own occupancy;
  - v36: `spawnfall01` is removed on both paths;
  - v38: the Lose-thread slip is fixed, so each suit room disables its own portrait triggers.

## Contents

| Segment | Visits |
| --- | --- |
| Opening (M0) | [v00 New Game](#v00-new-game) · [v01 gvillage](#v01-gvillage) · [v02 pandemonium](#v02-pandemonium) · [v03 fortress1](#v03-fortress1) · [v04 fortress2](#v04-fortress2) · [v05 fortress1 return](#v05-fortress1-return) · [v06 skool1](#v06-skool1) · [v07 skool2](#v07-skool2) · [v08 skool1 return](#v08-skool1-return) |
| Pool and Temple (M1) | [v09 potears1](#v09-potears1) · [v10 potears2](#v10-potears2) · [v11 potears3](#v11-potears3) · [v12 utemple](#v12-utemple) |
| Gardens (M2) | [v13 garden1](#v13-garden1) · [v14 garden2](#v14-garden2) · [v15 garden3](#v15-garden3) · [v16 garden4](#v16-garden4) |
| Centipede and Woods (M3) | [v17 centipede1](#v17-centipede1) · [v18 centipede2](#v18-centipede2) · [v19 wforest](#v19-wforest) |
| Chess (M4) | [v20 wchess1](#v20-wchess1) · [v21 wchess2](#v21-wchess2) · [v22 rchess1](#v22-rchess1) |
| Funhouse and Hatter (M5) | [v23 funhouse](#v23-funhouse) · [v24 hatter1](#v24-hatter1) · [v25 hatter2](#v25-hatter2) |
| Jabberwock and Woods return (M6) | [v26 jlair1](#v26-jlair1) · [v27 jlair2](#v27-jlair2) · [v28 wforest return](#v28-wforest-return) |
| Hedges and Towers (M7) | [v29 hedge1](#v29-hedge1) · [v30 tower1](#v30-tower1) · [v31 hedge2](#v31-hedge2) · [v32 tower2](#v32-tower2) · [v33 hedge3](#v33-hedge3) · [v34 tower3](#v34-tower3) |
| Finale (M8; [drafts pending 13a](#m8-finale)) | [v35 grounds1](#v35-grounds1) · [v36 grounds2](#v36-grounds2) · [v37 facade](#v37-facade) · [v38 keep](#v38-keep) · [v39 qlair](#v39-qlair) · [v40 ending](#v40-ending) |

---

# Opening segment (M0)

The first eight visits have working controllers and standalone route checks. Their specs are short: they list what is needed to make them hold up in a strict, continuous campaign chain, plus the native evidence that is still missing. Source: the planning opening audit (`C:/DEV/McGee/private/campaign-research/research/verified-opening.json`; see How to read).

<a id="v00-new-game"></a>
## 00 · New Game and the opening film

| Field | Value |
| --- | --- |
| Status | Implemented. Verified natively in Anode once, via Escape > New Game (`docs/CUTSCENE-AUDIT.md:41`, `docs/MENUS.md:21,43`) |
| Size / milestone | M · M0 · tasks T10, DG-16 |

**Flow today:**
1. Escape > New Game (`menu.rs:429`) shows the four `ui/newgame.urc` buttons. They are zipped in file order with `Difficulty::ALL` (`menu.rs:451-462`); their data commands are never executed.
2. A Yes/No confirmation follows (`menu.rs:474-489`).
3. The viewer autosaves the living session to Auto, unless Alice is dead (`viewer.rs:952-972`).
4. `movie::play("opening")` plays `video/opening.roq` with `video/opening.mp3`. P or Start pauses, focus loss pauses, holding Enter or A skips, and closing the window quits (`movie.rs:259-361`, `viewer.rs:1008-1025`).
5. The game selects gvillage with no entry, sets the difficulty and `fly = false`, then takes the chapter-start path (`viewer.rs:1026-1035, 1655-1715`): unarmed `starting_stats` and `Campaign::default`.

From the CLI, `--new-game --map gvillage [--difficulty X]` plays the film before the first frame (`viewer.rs:827-851`).

**Blockers for D1:**
- Launched with no arguments, `Options.map` defaults to skool1 (`main.rs:141`). The start prompt then offers only "Continue game" or "New visit - skool1" (`viewer.rs:464-554`). There is no difficulty choice and no film.
- `--new-game` on its own starts skool1 without the film, although `docs/SAVES.md:56` and `docs/ITEMS.md:21` document it as a new-game switch.
- Closing the window during a film started from the menu breaks out of the loop, and the clean-quit path writes the previous session to Auto a second time (`viewer.rs:1019` and the clean-quit autosave near `viewer.rs:3007-3013`).

**PLAYABLE MVP:**
- [ ] DG-16 answered. Recommended: a start screen offering New Game (difficulty, then the film, then gvillage), Continue, and Chapters.
- [ ] Implement it (T10), and make `--new-game` behaviour match its docs, or correct the docs.
- [ ] Closing the film window must not rewrite Auto a second time.
- [ ] Docs: `docs/MENUS.md`, `docs/INSTALL.md:3,12`, `README.md:113`, `docs/LOADOUTS.md:5`, `docs/SAVES.md:56,64`.

**Checks:**
- `--movie-check` (headless: decodes every frame and the soundtrack).
- Chain: `--campaign-route-check` starts from exactly the New Game state (`Stats::for_level("gvillage", None)` plus the chosen difficulty).

**Anode:**
- [ ] Empty save dir, then start screen, New Game, Normal, film (P to pause, 1,000 ms Enter hold to skip), village fall introduction, F5, Escape > Quit > Yes, relaunch, Continue. Screenshots at every step.
- [ ] One full film watch. Its soundtrack sync is unverified (NoDevice).

**Fidelity:** Escape does not skip the film (only holding Enter or A does). The film is not a save point, as in the original.

**Refs:** `menu.rs:415-500`, `movie.rs:240-397`, `viewer.rs:447-600, 827-851, 952-1035`, `ui/newgame.urc` (skill button order).

<a id="v01-gvillage"></a>
## 01 · gvillage (Dementia)

| Field | Value |
| --- | --- |
| Visit | `gvillage` (entry None; New Game state) · ROUTE 0 · next `pandemonium$player_start` |
| Status | OK. Last pass: 13,987 ticks, 6 jumps, 4 throws, 100 Sanity. Re-run in P0.7 |
| Size / milestone | S · M0 |

**Progression:**
1. Falling introduction: start #39, thread `Gvillage_RabbitHole_Start`, run by the village cinema from `entry_story` (`interaction.rs:1421-1429`). Entry None maps to the `alice_cat_talk1` marker (`interaction.rs:90-107`).
2. Gnome conversations.
3. Blade altar #15 (`Item_WeaponPickup_Knife`, pickup_thread `knife_cat`, `story.rs:84`) and the Rabbit scene.
4. Ungated `trigger_changelevel` #41. The exit waits for any running dialogue.

The shootable hatch `openthis` (#6, health 100) is optional.

**Enemies:** club guard `guard_rabbit` (#16), owned by Encounters. It activates once, when `Torchgnome3_Dialog_part2` finishes (`interaction.rs:806-813`). It gates nothing.

**Reward:** Vorpal Blade (REWARDS slot 0). The route asserts it comes from the authored pickup (`village_route.rs:35-45`).

**Existing checks:**
- headless: `--village-route-check`, `--village-cinematic-check` (six scenes, watched and skipped), `--village-machinery-check`, `--progression-check`;
- Anode: `--village-cinematic-render-check`, `--village-machinery-render-check`, `--progression-render-check`, `--visibility-check`;
- save cases `village`, `village-fall`, `village-knife`, `village-shrink`, `village-gnome`, `movement-updraft`.

**Blockers:** no chain or carry-over proof yet; no native gvillage to pandemonium crossing with autosave and Continue recorded.

**PLAYABLE MVP:**
- [ ] Turn the check body into `drive(r)` (F3), with 0 authored portals.
- [ ] Chain leg from the New Game state, asserting the Blade came from altar #15.

**Route check:** the existing check, unchanged. The chain version enters pandemonium through `player_start` with the carried Stats.

**Anode:** segment S0 (CAMPAIGN_PLAN.md §8.3). Cross #41 from the pre-exit fixture, confirm the autosave, then Continue in a fresh process.

**Fidelity:** camera interpolation, head tracking and the Cat dissolve are approximate. Steam lifts use a bounded acceleration instead of the original floating controller (`docs/VILLAGE.md:39`, `docs/VILLAGE_CINEMATICS.md:25`).

**Refs:** `src/village_route.rs`, `src/village/cinema.rs`, `docs/VILLAGE.md`.

<a id="v02-pandemonium"></a>
## 02 · pandemonium (Pandemonium)

| Field | Value |
| --- | --- |
| Visit | `pandemonium$player_start` (#77) · ROUTE 1 · next `fortress1$fortress1_start1` |
| Status | OK, watched (21,765 ticks) and skipped (21,150 ticks), 70.6 Sanity |
| Size / milestone | S · M0 · W22a |

**Gates:**
- the upper door `t141` stays locked until the key is taken (`interaction.rs:422-426`);
- the return portal #35 (to `t127`) and `alice_return_trigger1` need both the key and `cart_done`;
- `alice_leave` and `Pand_End_Ship` need `pand.returned` (`pandemonium.rs:403-414`).

**Exit:** a Rust adapter for the script `map()` calls in `maps/pandemonium.scr`. It emits `fortress1$fortress1_start1` once, after the departure dialogue and 15 s of flight (`pandemonium.rs:536-543`).

**Enemies (Normal):**
- placed clubs #64, #238, #430 and diamond #438;
- func_spawns `return_spawn2`/`return_spawn3` (diamond) and `club_spawn1`;
- #638 exists on Hard only.

No kill gate: the kill threads feed an empty thread.

**Reward:** Cards altar #27 (asserted by the route).

**Existing checks:**
- headless: `--pandemonium-route-check` (asserts one cart ride, the key, one return teleport, Cards, and a clear fortress1 spawn), `--pandemonium-skip-route-check`, `--pandemonium-check`, `--cinematic-check`;
- Anode: `--pandemonium-render-check`, `--pandemonium-machinery-render-check`;
- 11 `pand-*` save cases.

**Blockers:**
- **W22a:** `exit_sent` is set when the exit is emitted. If `enter_level` then fails, the viewer only shows "Could not enter next map" (`viewer.rs` ~2170) and never re-emits, stranding Alice on the finished airship. The latch must survive a failed load.
- The last native crossing predates the Fortress arrival scene (`docs/VALIDATION.md` ~331).

**PLAYABLE MVP:**
- [ ] W22a latch re-emission, with a unit test that simulates a failed load.
- [ ] `drive(r)` with 1 authored portal.
- [ ] Carry-over from gvillage.

**Anode:** segment S0. Load a boarding save, hold Enter, and confirm the Fortress arrival scene and autosave. Continue in a fresh process.

**Fidelity:** spline interpolation, boarding arcs, head tracking, rope pendulums, the return-room rear panel, machinery audio loops (`docs/PANDEMONIUM.md:45,55`).

**Refs:** `src/pandemonium.rs`, `src/pandemonium/cinema.rs`, `src/pandemonium_route.rs`.

<a id="v03-fortress1"></a>
## 03 · fortress1 (Fortress of Doors, first visit)

| Field | Value |
| --- | --- |
| Visit | `fortress1$fortress1_start1` (#69, thread `Fortress1_Start`) · ROUTE 2 · next `fortress2` (default start) |
| Status | OK: 10 jumps, 9 throws, 32 combat damage |
| Size / milestone | S · M0 · DG-7 |

**Progression:**
1. The arrival scene (Rust fortress cinema).
2. Lower platforms.
3. `second_teleport` #66 (first visit only).
4. The splitting room (`Fortress1_Start_Split`, #144).
5. `f1_changelevel` #41 (first visit only; `fortress.rs:164-175`).

Spawn alias: entry None and `fortress1_start1` both resolve to `camerarest` (`interaction.rs:72-89`).

**Enemies:**
- Boojums #1 and #561 (#561 is absent on Easy);
- Diamond spawners `end_spawn2`-`4` (`end_spawn1` is Hard only). They are live from load (`encounters.rs:162-174`), whereas the original triggers them at the end of the arrival; that is a fidelity difference;
- the lookout, bridge and tower guards (#53, #59, #221, #222, #282) appear only in the arrival cinema.

**Existing checks:**
- headless: `--fortress-route-check` first half (state checks, then the route), `--fortress-cinematic-check`;
- Anode: `--fortress-render-check`, `--fortress-cinematic-render-check`.

**Blockers:**
- no native save fixture (`save_check.rs:230-239` lists none for fortress1);
- carry-over not proven;
- **difficulty bits are not applied to triggers** (`interaction.rs:272-357`): the Hard-only `trigger_fall` #129/#141 are live on Easy/Normal, and the Easy/Normal-only recovery teleports #11-#15 are live on Hard (DG-7).

**PLAYABLE MVP:**
- [ ] DG-7 filter, then re-baseline.
- [ ] Native fixtures via `--campaign-save-chain-write`.
- [ ] `drive(r)` with 1 portal.

**Anode:** S1 protocol. Arrival scene watched and skipped; F5/F9 across a restart in the splitting room; cross #41.

**Fidelity:** path interpolation, guard projectile effects, head tracking, prop attachments, portal camera views, weight-responsive fulcrum tilt, background flying paths (`docs/FORTRESS.md:19`).

**Refs:** `src/fortress.rs`, `src/fortress/cinema.rs`, `src/fortress_route.rs`.

<a id="v04-fortress2"></a>
## 04 · fortress2 (Beyond the Wall)

| Field | Value |
| --- | --- |
| Visit | `fortress2` (default `fortress2_start1`) · ROUTE 3 · next `fortress1$fortress1_start2` |
| Status | OK: 3 authored portals, 20 jumps, 44 throws, 7 damage, 92 Sanity |
| Size / milestone | S · M0 · DG-7 |

**Gates** (`beyond.rs:444-454`):
- the walkway triggers need `beyond.walkway_ready`;
- the `toskool` teleport #83 needs `raised`;
- the last doors need `last_ready`;
- `last_changelevel` #43 needs `last_open`.

No kill gates.

**Enemies (Normal):**
- placed clubs #62 and #76-#78, diamond #36;
- Boojums #53, #389, #646 (#645 and #647 are Hard only);
- 14 enemy func_spawns.

The `hub_booj1` mana spawners never spawn (fidelity).

**Existing checks:** `--beyond-check`, `--beyond-route-check`; `--beyond-render-check` (Anode). A native F5/F9 has passed once (`docs/BEYOND.md:21`).

**Blockers:**
- **Arch-room pit:** Hard-only `trigger_fall` #91 (top z -176) sits above the Easy/Normal-only recovery teleport #17 (top -192). A fall on Normal therefore kills, where the original teleports Alice back (DG-7). Enter retry recovers, so this is not a softlock.
- The musical-lever puzzle expects the player to hear three notes. That is an accessibility follow-up, not a blocker.
- No native save fixture.

**PLAYABLE MVP:**
- [ ] DG-7 filter.
- [ ] Headless probe: a fall into the arch-room pit on Normal teleports.
- [ ] Native fixture.
- [ ] `drive(r)` with 3 portals.

**Anode:** S1 protocol, plus the pit probe with real input.

**Fidelity:** full puzzle camera sequences (including the arch room), background props, Rage Box staging.

**Refs:** `src/beyond.rs`, `src/beyond_route.rs`, `docs/BEYOND.md`.

<a id="v05-fortress1-return"></a>
## 05 · fortress1 return (Fortress of Doors)

| Field | Value |
| --- | --- |
| Visit | `fortress1$fortress1_start2` (#44, thread `Setup_Skool`, emulated) · ROUTE 4 · next `skool1$skool1_start1` |
| Status | OK: 0 teleports; shutters and Boojum reveal asserted |
| Size / milestone | S · M0 |

**Progression:**
1. The upper walkways.
2. `boojum_trigger` #34 (`Fortress1_Boojum_Attack`: shutters and reveal, one-shot).
3. A running jump from the raised edge into the moving `s1_changelevel` #40 (return visit only).

**Enemies:** Diamond spawners `end_spawn3`/`end_spawn4`; reveal Boojums `s1_booj1`/`s1_booj2` (`get_s1booj1`/`get_s1booj2`).

**Existing checks:**
- headless: `--fortress-route-check` second half, `--fortress-cinematic-check` (the reveal fires once, and watched, skipped and loaded runs are equivalent);
- a native F5/F9 has passed once (`docs/FORTRESS.md:31`).

**Blockers (U9):** what a missed window jump does is unverified.
- `trigger_fall` #129 is Hard-only (768) but live on Normal.
- The Easy/Normal teleport #12 (to `t174`) sits above it.
- The Easy/Normal essences #9 and #560 at z 16 near (-3700, 3808) point to a lower floor.
- `second_teleport` is disabled on the return visit.

Alice may therefore be stranded below the route, with R/Home recovery the only way out.

**PLAYABLE MVP:**
- [ ] Headless probe of the missed jump (and an Anode probe).
- [ ] If Alice can be stranded, add a reviewed recovery entry.
- [ ] `drive(r)` with 0 portals.
- [ ] Carry the pickup ledger through the first visit's ids (DG-15).

**Anode:** S1 protocol. Watch and skip the reveal; do one real-input jump and one deliberate miss; confirm the skool1 autosave.

**Refs:** `src/fortress.rs:164-175`, `src/fortress_route.rs:137-172`.

<a id="v06-skool1"></a>
## 06 · skool1 (Skool Daze, first visit)

| Field | Value |
| --- | --- |
| Visit | `skool1$skool1_start1` (entry None resolves to it, `bsp.rs:304-309`) · ROUTE 5 · next `skool2$skool2_start1` |
| Status | **RED.** Since the Blade/Cards change of 2026-09-28, the route runs out of Sanity near the recipe exit (`docs/BLADE_CARDS.md:58`, `private/blade-cards-school-route-2.log`). Last pass before that: 15,039 ticks, 15 jumps, 27 Sanity |
| Size / milestone | S · M0 · P0.8 |

**Progression and gates** (`school.rs:383-425`):
1. `Setup_FirstPass`.
2. `Theatre_Cinematic`.
3. `Skool1_OG_MoveShelf`.
4. The library lifts and `start_book1`-`4`.
5. `book_cinematic`.
6. The recipe.
7. The exit: the Rust adapter for `trigger_once` #58 `ig_trigger` (`Book_Ingredients_Exit`, `school.rs:643-647`). The BSP exit #77 `toskool2` lies below the floor. Both are gated on the first visit plus `recipe_ready`.

**Enemies:**
- first-visit club, diamond and Boojum groups (`t188`, `t186`, `play_guard1`/`play_guard2`, eight `theatre_*`, `library_guard*`);
- `t187` and `t189` are reserved for the return visit.

None gate.

**Reward:** Mallet altar #78 `mallet_skool1`, first visit only (`inventory.rs:495-523`). The route never asserts it.

**Existing checks:**
- headless: `--school-route-check`, `--school-secret-check`, `--school-check`, `--footing-check`, `--progression-check`;
- Anode: `--school-render-check`, `--progression-render-check`;
- save cases `school`, `dice-*`, `guard-cut-*`, `movement-knockback`.

**PLAYABLE MVP:**
- [ ] P0.8: retune the route runner (weapon choice, aim lead, Will budget, health-pickup detours), not the gameplay values. Re-baseline the metrics.
- [ ] Strict chain: the Mallet comes from altar #78, and Sanity survives the carry-over from fortress1.
- [ ] `drive(r)` with 0 portals.

**Anode:** S1 protocol (CAMPAIGN_PLAN.md §8.3). Watch and skip `Theatre_Cinematic`. F5 on a library lift, kill, `--load quick`. Cross `ig_trigger` #58 (`Book_Ingredients_Exit`) from the pre-exit fixture, confirm the skool2 autosave, then Continue in a fresh process. Run `--school-render-check` and `--progression-render-check`.

**Fidelity:** condensed theatre and library choreography; linear book paths (`docs/SCHOOL.md:33`). The docs line `docs/SCHOOL.md:25` is stale.

**Refs:** `src/school.rs`, `src/school_route.rs`, `docs/SCHOOL.md`.

<a id="v07-skool2"></a>
## 07 · skool2 (Skool's Out)

| Field | Value |
| --- | --- |
| Visit | `skool2$skool2_start1` (#479) · ROUTE 6 · next `skool1$skool1_start2` |
| Status | **Route passes, strict chain pending (2026-09-30).** 35,802 ticks, 86 Sanity, live Diamonds. Strict watched gym combat / skipped Dice provenance remain unresolved. |
| Size / milestone | M · M0 · F3 |

**Progression** (`school2_quest.rs`, `school2.rs:394-414`):
1. The gym lever, then `Old_Gnome_Mushroom`.
2. **Kill the three gym Boojums** `spawn_newbooj1`-`3` (Battle, then Laboratory).
3. `Kill_The_Gnome`, then **kill both lab guards** `gnome_killer1`/`gnome_killer2` (Rescue, then SpiceDialogue).
4. Jumbogrow, the mirror portals, `Skool2_GrowLollypop`, `Skool2_LastGnome_Cinema`.
5. The potion and star, then `exit_trigger` #29 (`quest.complete`). `s2_changelevel` #75 is permanently disabled.

These two kill counts are the only kill gates in the opening segment.

**Rewards:** potion and star. Optional: the Demon Die altar #44 (`dice_cat`) and a second Mallet altar #18.

**Existing checks:**
- headless: `--school2-route-check` (a private Route), `--school2-check`, `--gym-check`;
- Anode: `--school2-render-check`;
- save cases `battle`, `growing`, `mixing`, `rewards`, `exit`, `item-*` (9).

**Current gaps and resolved ownership:**
- The shared Route carries Stats and uses the viewer's story deferral and quest gates. The fresh route and continuous School2-to-return checks pass. Strict campaign completion remains unproven: watched gym combat death; skipped route does not collect altar #44.
- M0 item 8 is implemented: six Diamonds #25/#37/#48/#65/#70/#595, growth spawner #6 and the second `dice_boojum` #202 use legacy Encounters. Entity #202 is excluded on Easy. Existing quest actors retain their owners. Format-12 upgrade preserves quest state and NPC hit/loot slots, adds the new runtime rules, and resumes activation from saved quest commits.
- Floor spawners #68/#69 have supported receivers but no incoming BSP targets or executable script calls in the supplied data. They remain dormant; no activation is invented. See [SCHOOL2.md](SCHOOL2.md#encounter-ownership-m0-item-8).

**PLAYABLE MVP:**
- [x] F3: shared Route, carried Stats and viewer-equivalent quest/story gating.
- [x] P0.8 route driver updated; combat values unchanged.
- [ ] Strict: the Die comes from altar #44.
- [x] M0 work item 8: live Diamonds, growth activation, difficulty-gated second Dice Boojum, stable ownership and format-12 migration. Floor markers supported without invented triggers.

**Anode:** S1 protocol. F5 mid-Boojum-battle, kill, `--load quick`. Cross `exit_trigger` #29 from the pre-exit fixture, confirm that the `skool1$return` autosave keeps the potion and star, then Continue in a fresh process. Once item 8 lands, fight a live Diamond guard with real input. Run `--school2-render-check`.

**Fidelity:** camera cuts, choreography, floating-path timing (`docs/SCHOOL2.md:23`). The docs line `docs/SCHOOL2.md:15` is stale.

**Refs:** `src/school2.rs`, `src/school2_quest.rs`, `src/school2_route.rs`.


<a id="v08-skool1-return"></a>
## 08 · skool1 return (Skool Daze)

| Field | Value |
| --- | --- |
| Visit | `skool1$skool1_start2` (thread `Setup_LastPass`) · ROUTE 7 · next `potears1$potears1_start1` |
| Status | OK standalone. `--school-return-chain-check` (12,270 ticks, 82 Sanity) starts from a fresh skool2 completion, so it inherits skool2's RED state |
| Size / milestone | S · M0 · W22a |

**Progression:**
1. The `lastpass_trigger` (`t187`) and `return_booj_trigger` (`t189`) ambushes.
2. The star doors (`Skool1_Setup_OLift` spends the star, `school.rs:462-465`).
3. The observatory lift and the globe.
4. Drinking the potion, then shrinking.
5. At `Phase::Complete` the Rust code emits `potears1$potears1_start1` **every frame** (`interaction.rs:1286-1299`). The BSP exit #92 is gated on `return.finished` (`school.rs:432-434`).

**Existing checks:**
- headless: `--school-return-check`, `--school-return-chain-check`;
- Anode: `--school-return-render-check`;
- save cases `return`, `return-lift`, `return-potion`, `return-shrink`;
- a native drinking save reached potears1 with its autosave (`docs/SCHOOL-RETURN.md:28`).

**Blockers:**
- If the next map fails to load, the every-frame exit re-runs the full `enter_level` on every frame (W22a throttle).
- The chain starts from a fresh skool2, so the first-visit pickups are still present. The 82 Sanity figure is optimistic (DG-15).
- `ensure_level_weapons` grants the Mallet and one Die at `skool1_start2` whether or not they were collected (`inventory.rs:228-239`); strict mode skips that `campaign::loadout` baseline fill (CAMPAIGN_PLAN.md F3 item 6).

**PLAYABLE MVP:**
- [ ] W22a throttle.
- [ ] `drive(r)` with 0 portals, fed by the real chain.
- [ ] Strict reward provenance.
- [ ] The chain enters potears1 and reports the unhandled start thread `Tears1_Start_Cinematic` as the frontier (until M1).

**Anode:** S1 protocol from the drinking fixture. Confirm the potears1 autosave, then Continue in a fresh process.

**Fidelity:** camera cuts, star presentation, cutscene effects (`docs/SCHOOL-RETURN.md:32`).

**Refs:** `src/school_return.rs`, `src/school_return_route.rs`.

---

# Pool of Tears and Underwater Temple (M1)

Shared M1 systems: the spline mover, bind and rider kit (F4, proven by pilot A); spline path-node thread events; brush-entity liquids (W13); breath bubbles (T4); `func_fallingrock` (W9); Army Ant, Corporal, Snark, Bloodrose and Mushroom archetypes (E1-E6); SceneRunner (C1); hazard trigger semantics (W4); and chain carry-over pool, then potears2, then Duchess, then temple.

<a id="v09-potears1"></a>
## 09 · potears1 (Pool of Tears)

Implementation update, 2026-09-30: the ordinary standalone Normal route now
passes from the entrance to Hollow Hideaway with native enemies active: bank
climb and boulders, Turtle conversation, all four rides, bank pickups, lily pads,
Normal posts, rope jump/grab/climb, upper bank and the one-shot ending. The
destination preserves the departing resources exactly. Five naturally reached
Store checkpoints pass separate-process restoration; a real Space input climbs
the restored rope. See [POOL.md](POOL.md#complete-transport-route) for commands,
evidence and limitations. The gap list and detailed acceptance recipe below are
the original planning inventory, not a current statement that these systems are
absent. Easy/Hard and the strict incoming campaign chain remain separate work.

| Field | Value |
| --- | --- |
| Visit | `potears1$potears1_start1` (#284 at -3728 2912 280, no angle key, so yaw 0) · ROUTE 8 · visit key `potears1$first` · next `potears2$potears2_start1` |
| Status | **PLAYABLE standalone on Normal.** Continuous entrance-to-exit proof, native combat, no assists, exact outgoing resources; fidelity and other difficulties remain partial |
| Size / milestone | L · M1 |
| Depends on | F4; pilot B (the first M1 item, CAMPAIGN_PLAN.md §5 F4 Pilots); W1 (clip posts), W4 (`setdamage`, named hint gates), W5, W9, E1, E2, E4, C1, C3, C4, R1 (ride, hop, rope), R3, R5 |
| Reservations | hit-ID 6,800,000 · `potears1/` · `potears1.` · `--potears1-*` · save cases `potears1-*` |
| Owner | legacy Pool (`pool.rs`), extended per CAMPAIGN_PLAN.md F1.4a; the reservations apply to new IDs, facts and save cases |

**Implemented today:**
- 15 transport movers: `rideleaf1`-`4`, `leaftrain1a`-`d`/`2a`-`d`, `woblily1`/`2`, `swingingbranchobj`. They have drop gates, rider carry, trains and saves (`pool.rs:126-135` supported, `150-208` load, `209-237` ready/trains, `295-337` events, `402-472` advance).
- 14 leaf and lily props drawn by `pool::Art` (`pool.rs:475-523`).
- The Turtle conversation: 14 lines, whose completion drops leaf 1 (`pool.rs:14, 295-301, 332-337`; `story.rs:12-16`).
- 12 Ladybugs (`lady1`-`4` plus `x_lady1`-`8`) with activations and saves (`ladybug.rs:35-45, 482-523`; `encounters.rs:175-186`; `interaction.rs:712-735, 1382`).
- Currents #7/#882 and rope #57 via the shared traversal code.

**Original gaps (historical; see implementation update above):**
- The entry scene: the #284 thread never runs, because `entry_story` only fires `entry` (`interaction.rs:1421-1428`).
- `Tears1_Boulder1/2/3`, `Tears1_End_Cinematic` and `Turtle_Encounter2/3`. They are marked fired and print the pending notice once (`interaction.rs:1374, 1400-1404`).
- `func_fallingrock`.
- AI for the Army Ants, river Snarks and Bloodrose. The three Mock Turtles, the White Rabbit and the Walkrocks stand at their editor positions (`npc.rs:46-80`).
- A gated exit: #115 is ungated (`Pool::gate` returns Always except for the ride-leaf triggers, `pool.rs:287-294`).
- Kill volume #127 carries only `setdamage 9999`, but `interaction.rs:326-328` parses `damage` (default 10 every 0.5 s), so the plunge does not kill.
- The difficulty clip posts #1-#6 are not solid.

Implementation update, 2026-09-30 (task 12): the actual entrance thread now
selects the arrival inside the existing Pool owner. C1 clocks, terrain-supported
Alice/Rabbit staging, the final rock, and one watched/skipped landing are covered
by the arrival fixtures. Active old scenes resume; completed saves do not replay.
See [POOL.md](POOL.md) for acceptance and remaining navigation/rock fidelity.

**Arrival (`Tears1_Start_Cinematic`, no skipthread, about 10 s):**
1. A 2 s fade-in; camera `alice_watchx` #640 follows `cams/tears1_path4`.
2. `dk_boulder4` #751 is triggered at +4 s and rolls down `t492`-`t495`.
3. `rabbit_actor1` #732 runs from +5 s through `rabbit_run_pos1`-`3` (#737, #747, #748) and is then removed.
4. The fake Alice walks to `alice_posx1` #643 (-3536 2912 280).
5. `boulder4` becomes non-solid.

Level init (`Tears1_World_Init`, `Tears1_Cinematics_Init`):
- `rideleaf1`/`4` and `turtleleaf` are raised 2048 and `rideleaf2` 2100; `rideleaf3` is only rotated;
- the ride triggers 1/2/4 are disabled;
- the branch and woblily pendulums start;
- `cat_after_talk` is off and `cat_before_talk` on;
- `riversnarks` get `max_inactive_time 20`;
- `setfarplane 2000`;
- `dk_boulder2`/`3` are hidden and `boulder3` is non-solid;
- `ant_pusher1` is warped to `ant_pos1` #629; `ant_pusher1`/`2` get AI off, and `pusher2` is non-solid;
- `turtle2` is warped to `turtle2_doit` #59;
- the Rabbit is at `rabbit_start_pos1` #738 with AI off;
- `Arm_Ladies` rearms `x_lady1`-`8` every 2 s.

**Progression and gates:**
1. **Arrival scene.** No gate.
2. **Riverbank climb.** A switchback from z 280 to about 2000, then east along the top. Currents #7/#882 push -X at 200. On the way:
   - `Tears1_Boulder1` #70 (z about 836) and `Tears1_Boulder2` #667 (z about 1304) are short camera scenes (`tears1_path5`/`tears1_path7` on `dialog_watchx` #641). Ant pushers roll `dk_boulder1`/`2` down the waypoint chains `t306`..`t519` and `t397`..`t518`;
   - `Spawn_LadyX4` #50 is an ambush;
   - `Tears1_Boulder3` #69 (z about 1744, target `dk_boulder3`) shows and rolls boulder 3 down `t323`..`t522`; 2 s later `ant_pusher1`/`2` become solid, live enemies;
   - `trigger_fall` #52 lies below.
3. **Mock Turtle conversation.** Trigger #110 runs `Tears1_Turtle_Cinema1`: 14 lines, then the turtle's leaf drops and it rides away, then `RideLeaf1_Fall` (8 s plus 1 s settle) enables `rideleaf1trig`. Gate: conversation completion (implemented as `pool.ready0` = dialogue end + 9 s).
4. **Leaf 1 ride.** `rideleaf1path` #214, 46 nodes, about 64.3 s. Along it:
   - `Spawn_LadyX1` #67 at about 9.4 s;
   - `leaftrain1` #32 at about 11.4 s;
   - `Turtle_Encounter1` #128 at about 39 s. `turtle1` rides `turtle1path` #484; node `t198` #506 runs `killturtle1` at +22.5 s, which starts `rideleaf2fall`;
   - `leaftrain1end` #9 at about 62.5 s drops Alice into the pool below.
5. **Lower east bank.** Climb out of the first-fall pool (z about 1204) to the leaf-2 bank (z about 1364). `ladies1and2` #118 starts `lady1`/`lady2`. Army Ants #125/#508 wait near (2628, -470, 1464). The woblily pads #765/#769 lead to `rideleaf2`. Gate: `pool.ready1`.
6. **Leaf 2 ride.** `rideleaf2path` #213, 55 nodes, about 82.6 s. Along it:
   - `leaftrain2` #34 at about 7.7 s;
   - Hard-only clip post #6 at about 10.5 s;
   - `Spawn_LadyX2` #63 at about 11.7 s;
   - the swinging branch #130;
   - `ladies3and4` #116 at about 66 s;
   - `leaftrain2end` #8 at about 76.7 s drops Alice at the second fall.
7. **Leaf 3 ride to the rope.** From the second-fall pool (z about 140), board `rideleaf3` #352 (always ready) via #351. `rideleaf3path` #197, 39 nodes, about 62 s. Along it:
   - clip posts: #5 (Hard) at about 3.5 s, **#4 at about 9 s and #3 at about 12 s (Normal and above)**, #2 (Hard) at about 26 s;
   - `Spawn_LadyX3` #56 at about 15 s;
   - `leaftrain3` #13 at about 24.2 s;
   - a large essence #16 at about 29 s;
   - `Turtle_Encounter2` #62 at about 47 s;
   - the leaf passes under **rope #57** (x -2336..-2304, y 1208..1240, z 96..608) at about 60.1-60.2 s, with the rope bottom about 114 units above the leaf surface;
   - the leaf then plunges through kill volume #127 at about 61.2 s.

   The inferred hand-off is a timed jump and E grab of #57, then a climb to the ledge at about z 330.
8. **Upper ledge.** `Turtle_Encounter3` #58: `turtle2` rides `last_turtleleafpath` #573 as a visual lead. `rideleaf4falldown` #109 drops `rideleaf4` (8 s plus 1 s) and enables `rideleaf4trig` #354. Gate: `pool.ready3`.
9. **Leaf 4 ride and end scene.** `rideleaf4path` #196, 24 nodes, 23 s. Along it:
   - `leaftrain4` #12 at about 7.9 s;
   - the Hard-only clip block #1 encloses the path at 9.9-11.6 s;
   - **#82** at about 16.1 s starts `Tears1_End_Cinematic`: camera `alice_end_cam` #81 on `tears1_path3`, the fake Alice at `alice_jumptoend` #51 jumping to `alice_end_jump` #80, then `end_of_level` about 5.0 s after contact;
   - the leaf itself crosses #115 at about 18.4 s and plunges to z -860.

**Exits:** `trigger_once` #82 (-3840 -2412 512) runs the end scene, which fires `trigger_changelevel` #115 `end_of_level` (-3824 -2756 288) and leads to `potears2$potears2_start1`. #115 must be gated on the scene. The real player must be held and hidden at #82, because the leaf crosses #115 about 2.7 s before the scripted trigger.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Ladybug residents `lady1`-`4` | 4 | #118, #116 (implemented) | no |
| Ladybug ambush `x_lady1`-`8` (func_spawn) | 8 | `Spawn_LadyX1`-`4`, second of each pair 2.1 s later; rearm every 2 s (implemented) | no |
| Army Ant | 5 (Easy 4, Hard 6) | Placed; `ant_pusher1`/`2` AI off until Boulder3 +2 s | no |
| Snark-BiteOnly | 18 | `riversnarks` plus #29, river biters | no |
| Bloodrose | 1 | #53, proximity grow | no |
| Mock Turtles (`turtle_talk`, `turtle1`, `turtle2`) | 3 | Scripted, nodamage | - |

**Cinematics (MVP treatment):**
- `Tears1_Start_Cinematic`: Track `tears1_path4`, Rabbit and Alice puppets, the boulder 4 roll. The commit puts the player at `alice_posx1` facing 0, the Rabbit hidden, and `dk_boulder4` at rest at `t495` and non-solid. Fresh visits only.
- `Tears1_Boulder1`/`Tears1_Boulder2`: short camera scenes (skip allowed as a convenience). The commit puts each boulder at the chain end, solid, and the pusher idle.
- `Tears1_Turtle_Cinema1`: keep the implemented dialogue gate. Watched or skipped, it commits the `PO1_End` state: `turtle_talk` and the turtle leaf hidden, `RideLeaf1_Fall` running.
- `Tears1_End_Cinematic`: hold and hide the real player at #82; camera `tears1_path3` with the Alice puppet jump (about 5 s); then the transition. The skip commits the same transition and Stats. #115 fires exactly once.

**Puzzles and movers:**
- the ride leaves, trains, lilies and branch (implemented);
- the turtle leaves (`turtleleafobj`/`mdl`, `leaf_turtle1`, `last_turtleleafobj`/`mdl`; not in `pool::supported`);
- the falling rocks `dk_boulder1`-`4` (speed 160, dmg 50, gravity 0.2-0.3, spawnflags 6/14);
- rope #57;
- the clip posts #1-#6. These are unnamed `script_object`s with the `clip` shader (contents 0xd0000): collision only, filtered by difficulty (#3/#4 on Normal and above; #1/#2/#5/#6 on Hard), nothing to render.

**NPCs and dialogue:** `Tears1_TurtleDialog1` (14 voiced lines, `alcz2001`-`2007`, `mkt001`-`007`) is implemented; add `potears1` to `check_faces` (`story.rs:803-812`). Of the eight `trigger_catmessage` regions, the five unnamed ones (#43, #44, #45, #729, #730) are summon hints. The named `cat_before_talk` (#47, #728) and `cat_after_talk` (#46) use HintSpec gating: they swap when the Turtle conversation starts. The existing Pool owner supplies the saved condition; see `docs/CHESHIRE.md`.

**Special mechanics:**
- spline leaf riding and waterfall support removal;
- a timed rope grab from a moving leaf (a window of about 0.2 s; carried riders do not inherit leaf velocity);
- clip posts at rider height;
- the kill volume `setdamage 9999`;
- the Ladybug rearm loop;
- the river Snarks' inactivity limit.

**Rewards:** no toy. Essences: large #16, medium #38/#65, super #75/#122. Ladybug loot is medium grade.

**Entity classes:** `info_player_start` (thread), `trigger_once`, `trigger_changelevel` (scene-gated), `trigger_push`, `trigger_fall`, `trigger_hurt` (`setdamage`), `trigger_catmessage`, `func_rope`, `func_fallingrock`, `func_spawn`, `func_camera`, `script_object`, `script_origin`, `script_model`/`tears_giant_leaf`, `info_splinepath` (node thread `killturtle1`), `info_waypoint`, `info_pathnode`, `Enemies_Ladybug`/`Army-Ant`/`Snark-BiteOnly`/`Bloodrose`/`Mockturtle`, `Characters_WhiteRabbit`, `Characters_Walkrock-*`, `Item_MetaEssence*`.

**PLAYABLE MVP checklist:**
- [ ] R1 route primitives (shared with potears2 and utemple): `ride(pred)`, `hop` (the posts are 96-120 units tall; the jump apex is about 100), rope grab and climb via the shared rope prompt (`route.rs:239-246`), `swim_to`; a Difficulty parameter (R3).
- [ ] W4: parse `setdamage` so #127 kills.
- [x] Replace the global Pool rollback with per-object blocking and rider push-out. Keep the rule that leaves a blocked rider behind. Confined/unconfined collision, saved delays and pause are checked by `--pool-check`.
- [ ] W1: the clip posts #1-#6 as difficulty-filtered, collision-only colliders. Prove the Normal route over #3/#4 before enabling them in play.
- [ ] Entry scene (C1) through the registry `entry_story`, fresh visits only, with its commit.
- [ ] W9 `func_fallingrock` for `dk_boulder1`-`4`; the Boulder1/2 scenes; Boulder3 rolls at the player and the pushers activate.
- [ ] Scripted actor ownership: the Rabbit is hidden after the entry scene; `turtle_talk` and its leaf are hidden after the talk; the `turtle1` ride visual; `turtle2` staging (`Turtle_Encounter2`/`3`); the ant pushers stay inactive until Boulder3.
- [ ] End scene and gated #115 (fact `potears1.end_started`), exactly one transition. This is pilot B: implement it inside `pool.rs` with the gated-trigger migration (CAMPAIGN_PLAN.md F1.4a).
- [ ] E1/E2/E4: Snark-BiteOnly, Army Ants and the Bloodrose fight (DG-3). None gates the exit.
- [x] HintSpec for `cat_before_talk`/`cat_after_talk` (C4 Pool pilot; `docs/CHESHIRE.md`, three named brushes, saved selection and repeated-summon checks).
- [ ] Saves: the upgrade path rearms `Tears1_Boulder1/2/3`, `Tears1_End_Cinematic` and `Turtle_Encounter2/3` (which the pending path marked fired) and never replays the entry scene. Save cases: `potears1-entry-scene`, `potears1-boulder-rolling`, `potears1-leaf3-ride`, `potears1-leaf4-before-82`, `potears1-end-jump`.
- [ ] Docs: `docs/POOL.md`, `docs/LADYBUGS.md`, the CUTSCENE-AUDIT Pool row.

**Route check (`--potears1-route-check` and `--potears1-skip-route-check`):**
- `Route::enter` with the Stats carried from the skool1 return (chain), or `Route::new` standalone, on Normal.
- Sequence:
  1. Wait for, or skip, the entry scene.
  2. Climb the bank through #70, #667, #50 and #69 with auto-aim combat.
  3. Walk into #110, let the story auto-advance, and assert the Turtle talk completed.
  4. Wait for `pool.ready0`, board `rideleaf1obj` and ride until support is removed.
  5. Climb out and cross the woblily pads to leaf 2; ride it.
  6. Board leaf 3 and hop the Normal posts #4/#3.
  7. At about 60.1 s, jump and grab rope #57 with `use_pressed`, then climb to the ledge.
  8. Navigate to #58 and #109, wait for `pool.ready3`, and ride leaf 4 into #82.
- Assertions:
  - `transition == ("potears2", Some("potears2_start1"))`;
  - teleports == 0;
  - alive on every tick;
  - body clear;
  - no pending-script output;
  - every scene trigger fired exactly once;
  - the carried Stats equal the departing Stats after `campaign::arrive` in the chain's mode (strict or not);
  - watched and skipped runs produce identical Stats and pool history.
- A Hard contract documents what block #1 does to a leaf-4 rider.

**Anode checks:**
- [ ] `--potears1-render-check`: the entry scene (Rabbit, boulder 4), the Boulder1/2 shots, the turtle leaving, the end jump.
- [ ] Hold Enter to skip the entry scene, F5, kill, `--load quick`: the game is at `alice_posx1`, with no replay and the same resources.
- [ ] Save/restart mid-ride on leaf 1 and after the first waterfall (writer and reader as separate processes).
- [ ] From the `potears1-leaf3-ride` save: the post hops and the rope catch from the moving leaf through `--route-replay` (latency rule); then real input: Space climbs, dismount screenshot.
- [ ] From a staged leaf-4 save before #82, run once watched and once with Enter held. Both autosave `potears2$first`, and a fresh process with `--load auto` spawns at `potears2_start1` with identical inventory JSON.

**Fidelity follow-ups:**
- Army Ant stab-fling details;
- the Walkrock ambient AI;
- the Turtle conversation cameras (`tears1_path1`/`2`/`6`, `potears1_kmpx1`-`3`) and acting threads;
- exact spline and pendulum curves;
- the leaf landing sound;
- `setfarplane` fog density;
- the falling-rock earthquake;
- the `crying_alice` statue (`decorations.rs:131-133`).

**Risks:**
- Bank climbs, the rope hand-off and the ledge route now pass on Normal; other difficulties and exact original movement remain to be verified.
- The Normal clip posts on the leaf-3 centre line can strip Alice off the leaf.
- Linear path timing (`pool.rs:83-91`) may be misaligned with the original B-splines.
- The #115 double-fire risk.
- Old saves have consumed the pending triggers, which needs the rearm migration.

**Refs:**
- `maps/potears1.scr` (`Tears1_World_Init`, `Arm_Ladies`);
- `maps/cinematics/potears1_cinematics.scr` (`Tears1_Start_Cinematic`, `Tears1_Rabbit_Run`, `Tears1_Boulder1`-`3`, `Tears1_Turtle_Cinema1`, `Skipthread_PO1`/`PO1_End`, `Tears1_End_Cinematic` ~474, `Turtle_Encounter1`-`3`);
- entities #1-#6, #7, #9, #12, #13, #16, #32, #34, #50-#53, #56, #57, #58, #62, #63, #67, #69, #70, #82, #109, #110, #115, #116, #118, #127, #128, #196, #197, #213, #214, #284, #484, #506, #573, #640-#643, #648-#650, #667, #732, #751;
- `ai/c_armyant.st`, `ai/c_snark_biteonly.st`, `ai/c_bloodrose.st`.

<a id="v10-potears2"></a>
## 10 · potears2 (Hollow Hideaway)

| Field | Value |
| --- | --- |
| Visit | `potears2$potears2_start1` (#103 at -4592 1792 224, yaw 0) · ROUTE 9 · next `potears3$potears3_start1` |
| Status | **Playable on Normal.** Ordinary pond traversal reaches both antguards, Bill and scene-gated exit #68. See `HOLLOW_HIDEAWAY.md` for evidence and remaining approximations. |
| Size / milestone | XL · M1 |
| Depends on | F4; pilot A (the first M1 item, CAMPAIGN_PLAN.md §5 F4 Pilots); W4 (continuous-presence hazard), W5, T3 (slide surface), E2 and E3 (**gating**), E4, E5, C1, C2 (door flap), C3 (Bill rig, 13 lines), C6, R1 |
| Reservations | hit-ID 6,900,000 · `potears2/` · `potears2.` · `--potears2-*` · `potears2-*` |

**Implemented:**
- The registered `potears2` controller owns seven transport clips, two house doors, twelve pond props, the moving essence, fish death and the Bill ending.
- Native ants, Corporals, plants and six Ladybugs keep their combat/save identities; three reviewed pair activations include leaf one's spline cue.
- Normal entrance-to-Just-Desserts traversal uses ordinary controls, both leaf rides, the lily crossing and the live two-antguard gate. No recovery, position edits or resource grants are used.
- Bill stays hidden until the gate releases the house scene. The elevated exit cannot bypass it.
- Transport/ending detail, save migration, commands and fidelity limits: `HOLLOW_HIDEAWAY.md`.

**Arrival (setup):**
- `setfarplane 2000`;
- bind hierarchies: `lilyNmdl` to `lilyNorg2` to `lilyNorg`, and `lilyNobj` to `lilyNorg`; `leaf2org` starts at yaw -225;
- Bill is hidden and immortal;
- `end_cinematic_trigger` off; the `antguard_killed` kill thread registered;
- after the player spawns:
  - `lily1`-`4` loop their paths (ignoreangles) with `org2` pendulum bobs;
  - `lily5` loops with normalangles and no bob;
  - `lily6` swings as a pendulum;
  - the falling-leaf loop starts;
  - the fish timer starts;
- the Duchess doors are bound; `suck_push` is off; music `potears2.mus`.

**Progression and gates:**
1. **West shore:** ants #14/#89/#217/#553 (plus Corporals #8/#9 on Hard). `ladies1and2` #86 starts `lady1`/`lady2`. Essences #2/#577.
2. **Pond crossing east.**
   - The `lily1`/`lily2` pads ride small loops (8 nodes, 14 s each).
   - `leaf1startmoving` #93 sends leaf 1 once along `leaf1path` #94: 17 nodes, **about 30.8 s** (the first segment at speed 2, then 0.5, then 0.3/0.1 at the end). Node `t6` #185 runs `ladies3and4` about 5.5 s in.
   - The pond fish trap is `trigger_multiple` #4, spanning the whole pond surface layer (x -3456..2560, y -2176..2560, z -288..-256). **Six seconds of continuous presence** (gaps up to 0.5 s tolerated) runs `GoFish`, which is lethal.
3. **East area:** ants #75/#79/#85/#255/#293, Corporals #7 (Hard) and #84, Mushrooms #6 (Hard) and #78, Bloodroses #77/#81, super essences #62/#67, `ladies5and6` #72, and the `lily6` pendulum pad.
4. **Return on leaf 2 and lilies 3-5.** `leaf2startmoving` #198 runs `leaf2path` #199: 21 nodes, **about 37.5 s**. Node `t177` #486 fires `remove_end_leaf` at about 35.5 s, and leaf 2 is removed 2 s later, so Alice must step off by about 35 s. The `lily3`-`5` pads lead to the house front (z about 128). `lily5` carries the essence also named `lily5obj` (#36).
5. **Antguard gate.** The two antguards are Army Ants #71/#337 (spawnflags 1024, Easy/Normal) or Corporals #5/#554 (768, Hard/Nightmare), always exactly two per difficulty. `antguard_killed` counts deaths; at two it waits 2 s and enables #50. **Gate:** kill both.
6. **Bill scene and suction exit.** #50 runs `tears2_end_cinematic`:
   - AI off; the fake Alice at `alice_stand`;
   - Bill walks from `bill_endcinematic_start` to `bill_1`;
   - 13 voiced lines over `bill_cam`, `convo_cam` and `alice_cam`;
   - Bill climbs to `alice_gets_sucked_in`;
   - the doors flap (growing to about ±24 degrees), then snap open to 90/270 with an earthquake on Bill;
   - Bill flees (`paniked_run`);
   - `suck_push` pulls Alice in;
   - the doors reset, and the script triggers #68.

   The skip path (`Skipthread_PO2a`/`PO2a_End`) fades and fires #68.

**Exit:** #50, the scene, then `trigger_changelevel` #68 `end_of_level` (0 128 496), leading to `potears3$potears3_start1`. Gate #68 on the scene's completion.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Antguards (Army Ant #71/#337, or Corporal #5/#554 on Hard) | 2 | Placed at the house front | **yes** |
| Army Ant (others) | 9 | Placed | no |
| Army Ant Corporal (others) | 1 (Normal), 4 (Hard) | Placed | no |
| Ladybug `lady1`-`6` | 6 | #86; node `t6`; #72. No rearm loop in this map | no |
| Evil Mushroom | 1-2 | Placed | no |
| Bloodrose | 2 | Placed | no |
| Bill the Lizard | 1 | Hidden and immortal until the scene | - |

**Cinematics:**
- `tears2_end_cinematic` (with `tears2_dialog`, `flap_duchess_doors_start`, `flap_duchess_doors`, `Bill_Anims`, `Alice_Bill_Anims`, `Skipthread_PO2a`/`PO2a_End`). Cameras: `potears2_kmxp1`, `potears2_kmpx2`-`5`; func_cameras `bill_cam` #58, `convo_cam` #555, `alice_cam` #57, `suck_cam` #546. **MVP:** tracks, the Bill and Alice puppets, 13 beats, the door flap on the script_origin pivots, the hold and pull, then the transition. The skip commits the same transition.
- `GoFish` (fish-trap death): a camera at +80 +80 +200 watching Alice, the `fish_head.tik` attack at -64 X, and `fx_watersplash2` scale 1.5. Alice is hidden at 1 s and her health set to 0 at 2 s, followed by the standard death and retry. Not skippable.

**Movers:**
- lily pads 1-5 (`lilyNobj` clips with lilypad or leaf props on `lilyNpath` loops, with bobs on 1-4 only);
- lily 6 (a pendulum prop without a brush);
- ride leaves 1 and 2 (one-shot followpath, normalangles);
- falling leaves 1-4 (decorative loop);
- the Duchess house doors (`duchessdoor1`/`2obj` on `duchessdoor1`/`2org`);
- `suck_push` #15 (angle 90, speed 200; authored off, enabled only in the scene).

**NPCs and dialogue:** register `tears2_end_cinematic` with two functions under one event (the precedent is `Pand_End_Ship`, `story.rs:78-79`): `bliz001`, then `tears2_dialog` alternating `alcz2009`-`2014` and `bliz002`-`007`. Add `potears2` to the story and lip checks. The Bill puppet needs the clips `run`, `paniked_run`, `idle_01`, `idle_liftbelt`, `idle_eyes`, `idle_sneeze`, `talk_shrug` and `talk_fist`. `talk_liftbelt` is not in `c_bill.tik`. Catmessages `catz209` (#30) and `catz210` (#31) are unnamed hints.

**Special mechanics:**
- moving lily pads and one-shot ride leaves;
- a pad-bound essence (moving pickup origin);
- spline node thread events (`t6`, `t177`);
- the pond-wide continuous-presence fish trap;
- the difficulty-dependent two-kill gate with a 2 s delay;
- the scripted suction push;
- **slide surfaces** in this BSP (`textures/common/slide`, T3).

**Rewards:** no toy. Medium essences #2, #577, #36 (on lily pad 5); super essences #62, #67.

**Entity classes:** `info_player_start`, `trigger_once`, `trigger_multiple` (continuous presence), `trigger_changelevel` (scene-gated), `trigger_push` (authored off), `trigger_catmessage`, `script_object`, `script_origin`, `script_model`, `tears_giant_leaf`, `info_splinepath` (node threads), `info_pathnode`/`info_waypoint`, `func_camera`, `Enemies_Army-Ant`, `Enemies_Army-Ant-Corporal`, `Enemies_Ladybug`, `Enemies_EvilMushroom`, `Enemies_Bloodrose`, `Characters_BillTheLizard`, `Item_MetaEssence*`.

**PLAYABLE MVP checklist:**
- [x] Pilot A: data-driven movers for `lily1`-`6`, `leaf1`/`2` (removal 2 s after `t177`), the doors and falling leaves; render and collide them; the moving pickup origin for `lily5obj`.
- [x] Spline node events fired once per pass (`t6` runs `ladies3and4`; `t177` runs the leaf removal).
- [x] Native resident cast and all three Ladybug pair activations; the level controller owns the two difficulty-selected antguards.
- [x] Antguard gate: kill signals increment `potears2.antguards`; at 2, a delayed 2.0 s Enable of #50; #50 authored off at load; #68 gated on `potears2.complete`.
- [x] The fish trap as a controller-owned timer over the #4 volume (starts on entry; resets after a gap longer than 0.5 s; fires at 6 s) with its death presentation. No pending message.
- [x] The end scene (C1), with the Bill puppet and his clips, hidden until the scene (NPC ownership plus snapshot regeneration).
- [ ] T3 slide surfaces (SLIDING/SLIDE_JUMP) if the route crosses them; confirm where they are with the helper.
- [x] Saves: controller state; migrate old generic visits (rearm consumed triggers; relocate to the entrance when the saved position depends on the new supports). Cases: `potears2-antguard-one-dead`, `potears2-leaf1-ride`, `potears2-fish-timer`, `potears2-scene-line`.
- [x] Launcher `tools/launchers/Launch-Hollow-Hideaway.cmd`; `docs/HOLLOW_HIDEAWAY.md`.

**Route check:**
- Start from the carried pool Stats.
- West shore; board the lily pads and leaf 1 with the ride primitive. Assert `ladies3and4` fires about 5.5 s into the ride.
- Ride to the east area; board leaf 2 and step off before about 35 s; cross lilies 3-5.
- Kill the antguards. Assert the gate stays closed after one kill and opens 2 s after the second.
- Walk into #50.
- Never spend more than 5 s continuously in the pond.
- Assert:
  - `transition == ("potears3", Some("potears3_start1"))`;
  - teleports == 0;
  - alive;
  - no pending-script output;
  - identical carried Stats between watched and skipped runs;
  - the fish trap never fired.
- A separate contract covers the Hard antguard set and the fish-trap timing and reset.

**Anode checks:**
- [ ] `--potears2-render-check`: lily pads and leaves at two path phases; antguard Army Ant and Corporal poses (idle, fire, melee, pain, death); the fish-trap death; the Bill scene shots and door flap.
- [ ] Board a pad and leaf 1 with W/Space; F5 mid-ride; a fresh process with `--load quick` resumes, paused, with identical poses.
- [ ] From a staged save with one antguard dead: kill the second, wait 2 s, enter #50, hold Enter. potears3 loads and autosaves; a fresh `--load auto` keeps the Stats.
- [ ] A watched run of the scene with E advancing lines: F5 mid-scene, then F9 in a new process resumes on the same line and camera.
- [ ] Fish trap: swim 6 s and see the death and retry; a second run leaves the water for more than 0.5 s at 5 s and survives.
- [ ] `--difficulty hard`: Corporals guard the door, and the gate opens after both die.

**Fidelity follow-ups:**
- Mushroom suction and digest; Bloodrose grow;
- Army Ant stab-fling; Corporal grenade fidelity;
- resident Ladybug rearm cadence (there is no `Arm_Ladies` here);
- Bill acting and head-watch;
- the `card_doors` loop sound;
- the earthquake shake;
- falling-leaf curves;
- fog density.

**Risks:**
- Normal route reachability is verified; full Hard/Nightmare transport runs remain unverified.
- The guard gate is verified separately for every difficulty-selected pair.
- The pond-wide lethal volume.
- Bill ownership changes the NPC snapshot identity.
- The ScriptSlave spline timing assumption.

**Refs:**
- `maps/potears2.scr` (Setup, `tears2_end_cinematic` ~392, `PO2a_End` ~275, `fishtimer`/`fishtrigger`/`GoFish` ~20-80, `antguard_killed`, `ladies*`, `remove_end_leaf`);
- entities #2, #4-#9, #13-#19, #30, #31, #35-#50, #55-#59, #62, #67, #68, #71, #72, #75-#86, #89, #93-#95, #103, #156-#162, #181, #185, #198-#201, #217, #255, #293, #337, #472-#492, #519, #527-#546, #553-#555, #577;
- `ai/c_armyant.st`, `ai/c_armyantcorp.st`, `ai/c_evilmushroom.st`, `ai/c_bloodrose.st`, `models/c_bill.tik`.

<a id="v11-potears3"></a>
## 11 · potears3 (Just Desserts: the Duchess)

| Field | Value |
| --- | --- |
| Visit | `potears3$potears3_start1` (#70 at -832 -1360 200, yaw 90) · ROUTE 10 · next `utemple` (default start) |
| Status | **OK** standalone (`--duchess-check`, a normal entrance-to-exit fight) |
| Size / milestone | S · M1 · T2 |
| Reservations | The Duchess keeps `duchess::ID` 4,000,000. The 7,000,000 range is reserved for any new registry arms |
| Owner | legacy Duchess controller (`duchess.rs`), extended per CAMPAIGN_PLAN.md F1.4a |

**Implemented** (`src/duchess.rs`; line refs `~` because the finished Dice/Watch session changed this file after the survey):
- the Stage machine: Waiting, Expanding, Introduction, Fighting, Dying, Rescue, Farewell, Well, Complete;
- the fight starts on pickup identity (the Jackbomb pickup `potears3:21`, `inventory.rs:587-606`);
- gates for `catstuff`, `expand_room` and the exit (`Duchess::gate` ~349-359);
- `full_stats` applied once (~616-619);
- the exit through `story.defer_exit(("utemple", None))` (~631-634);
- hold-Enter skip committing an identical arena (~573-602);
- save state with a legacy upgrade (`interaction.rs:1683, 1832-1842`);
- `--duchess-check` and `--duchess-render`.

The concurrent Dice/Watch session added a serde-default `opponents` field for Dice summons.

**Arrival and progression:**
1. main: `bosslevel(1)`, `killdemons`, `full_stats`. Setup binds the wall groups, ceiling, beams, doors, fireplace and shell, and raises the entry door 128. The `catstuff` hint (#18, `catz211`) is active only while Waiting; it is gated but never presented today.
2. The Jackbomb pickup #21 (target `bomb_picked_up` #11) runs `expand_room`: quake #48; walls, ceiling and beams move from small to big over 3 s; the entry door drops 128; `spawn_duchess` grows her from the chimney; the intro plays 6 lines (`dch001`/`002`/`004`, `alcz2015`/`2017`/`2018`); skip is `Skipthread_D1`/`D1_End`.
3. The fight (below). Four medium essences respawn 10 s after each pickup.
4. Defeat runs `duchess_dead`, then `opensecretdoor`:
   - the walking rocks are removed;
   - the secret door opens;
   - Bill and the Turtle enter, and the shell is reattached to the Turtle;
   - the farewell plays 4 lines (`bliz008`, `mkt008`, `mkt009`, `alcz2019`);
   - the lever opens the well doors;
   - the Turtle jumps in, then Alice;
   - `end_of_level` #36 fires.

   Skip is `Skipthread_OS1`/`OS1_End`.

**Exit:** #36 (320 224 -432), gated on `duchess.complete`, leads to `utemple`.

**Boss (B0, the template):** 600 hp. Pursuit with detours; running beyond 350; melee inside 110. Attacks:
- pepper (frame 7, 750 u/s, 25);
- pig (frame 21, 600 u/s, 70, plus a 400-radius 15 splash with 400 knockback);
- smack (15);
- grab/bite (3 x 5, then a toss);
- phase dodges.

Pain threshold 50; arena sealed.

**Rewards:** Jackbomb (slot 3); the Mock Turtle shell, realised as `turtle_air` on utemple entry; the `full_stats` refill; respawning essences; the Duchess loot grade is super.

**PLAYABLE MVP checklist:**
- [ ] Carry-over: parametrize `duchess_check::check` with (entry, Stats). Run it with `Some("potears3_start1")` and a synthetic carried Stats (non-default Sanity, Will, selection and collected ids, and Dice copies), then from the real potears2 chain. Assert spawn #70, visit key `potears3$first`, `full_stats` applied once, owned toys preserved, Jackbomb copies at 1, and the transition `("utemple", None)`.
- [ ] T2 (re-resolve line numbers first: the finished Dice/Watch session changed `duchess.rs`): remove the stale Jackbomb-to-Blade deselect (~637-639). Add a contract that a Jackbomb blast within its radius damages `duchess::ID`. Fix `docs/DUCHESS.md:11,36` and `docs/ROADMAP.md:5`.
- [ ] Exit carry-over: after `campaign::arrive` into utemple, in strict and non-strict mode, `turtle_air` is true and the Jackbomb and selection are retained.
- [ ] A save case after the reward stage (F5, reload in a new process, continue to the utemple transition).

**Route check:** keep `--duchess-check` and its contracts (early-exit refusal, ownership-only refusal, pause, 30/60/144 Hz, watched versus skipped). The chain drives it with carried Stats. One variant selects the Jackbomb (after R2).

**Anode checks:**
- [ ] From the potears2 exit autosave produced in Anode: a fresh `--load auto` enters `potears3_start1`. The HUD refill happens once; the carried toys remain.
- [ ] Collect the Jackbomb with real input, hold Enter to skip the intro, and throw the Jackbomb at the Duchess (damage lands). F5 mid-fight, then `--load quick` resumes paused.
- [ ] Hold Enter through the farewell. The utemple autosave loads in a fresh process with the 20 s air budget and the Jackbomb, Dice and selection retained.
- [ ] `--duchess-render` only if the presentation changed.

**Fidelity follow-ups:**
- authored cameras (`potears3_kmpx1`-`4`, `tears3path1`/`2`, the `duchess_cam` orbit, `table_cam`, `end_cam`, `end_cam_guys`);
- the quake;
- `bosslevel` research;
- walking-rock animation and removal;
- random attack selection;
- the grow intro;
- fireplace emitters and debris;
- presenting the `catstuff` hint.

**Risks:** carry-over depends on potears2; removing the deselect changes weapon selection at fight start, so rerun the full `--duchess-check` and the Duchess save cases.

**Refs:** `maps/potears3.scr` (`expand_room`, `spawn_duchess`, `potears3_dialog`, `duchess_dead`, `opensecretdoor`, `potears3_end_dialog`, D1/OS1 skip threads); `ai/c_duchess.st`; `docs/DUCHESS.md`; entities #11, #18, #21, #36, #47, #48, #70, #135, #164, #166.

<a id="v12-utemple"></a>
## 12 · utemple (Wholly Morel Ground)

| Field | Value |
| --- | --- |
| Visit | `utemple` (default: the only start, #17 `utemple_start1` at -64 -5592 1480, yaw 270, inside world water) · ROUTE 11 · next `garden1$garden1_start1` |
| Status | **PARTIAL.** Task 16 supplies the guide, breath contacts, brush water, movers and gated scene exit. Non-gating Snark combat and remaining fidelity work are pending; see `UTEMPLE.md`. |
| Size / milestone | XL · M1 |
| Depends on | W1, W4 (hurt toggles and pulses), W13 (**static brush-entity water #32**), T4 (**breath bubbles**), T6 (directives), F4 movers (guide spline with node events, collapse programs, crush), E6 (Snark, non-gating), C1, R1 (`swim_follow`) |
| Reservations | hit-ID 7,100,000 · `utemple/` · `utemple.` · `--utemple-*` · `utemple-*` |

**Baseline audit, before task 16 (current implementation: `UTEMPLE.md`):**
- `turtle_air` is granted on entry (`inventory.rs:228-233`, 20 s of air; an authored arrival grant that strict mode keeps, CAMPAIGN_PLAN.md F3 item 6) and `after_temple` loadouts apply (`campaign.rs:140-145`).
- Only the 4 bubble-stack props draw. The fish schools and lanternfish are deferred. The guide turtle, `fake_alice`, the oysters, the fish head and the bubble chest are not drawn.
- The 4 Snarks are static.
- None of the 38 `script_object`s render or collide: 34 movers and obstacles, 3 surface-only beams, and **#32**, which is a `water_nodraw` liquid volume (contents 0x20000020; x -1536..328, y 992..2080, z 360..936). Rust builds liquids from world brushes only (`collision.rs:335-373`), so guide nodes `tp102`-`tp124` are dry in Rust.
- Every thread trigger is pending.
- All six `trigger_hurt` volumes are **always active**. `oyster_hurt1`-`4` and #45 deal the default 10 every 0.5 s; `fishhead_hurt` deals 20. The script instead disables `oyster_hurt1`/`2` and `fishhead_hurt` at load and only pulses them.
- There is no bubble air refill (`water.rs:88-104`), and `fx_mockturtle_launcher.tik` is not rendered (`particles.rs:220-243`).

**Arrival (main):**
- `full_stats`;
- `exit_trigger` off;
- `fake_alice` hidden and non-solid;
- oyster blocks 1-3 lowered 100;
- `oyster_hurt1`/`2` and `fishhead_hurt` off (`3`/`4` on);
- the column, panel and spike bubble emitters, beams and end bubbles hidden;
- `swimsinwater(1)`, `hasmockturtleshell(1)`, `god(1)` until `startturtle`;
- the turtle swims in place, and its launcher already emits breath;
- 10 fish schools and the lanternfish loop;
- the `ClamAttack3` loop starts;
- music `utemple.mus`.

**Progression and gates:**
1. **Invulnerability window** from arrival until the `startturtle` trigger (#25, about 200 units south). The oyster 3 cycle is already running.
2. **The guide.** At `startturtle`, `script_model turtle` #24 follows spline `tp2`: 160 nodes, about 34,800 units, about 194 s under the 1/speed rule. It averages about 220 u/s over the first 22 s and about 174 u/s to `tp156`, with bursts up to about 427 u/s. Its swim animation attaches `fx_mockturtle_launcher` at `tag_shell`, which spawns one `prj_mockturtle_breath` point per second (`timer.tan` has 100 frames over 5 s; each point lives 4 s). **This is Alice's main air source.** God mode turns off here.
3. **Player-triggered set pieces:**
   - `Column1Fall` #33 (then `Column2Fall`);
   - `Panel12Fall` #36 with beam 3;
   - `Panels3` #34 (panels 9/10 with beam 1);
   - `Panels4` #35 (the big panels; also fired by node `tp145`, so it must be idempotent);
   - `Column6Fall` #27;
   - `FishHeadAttack` #28 (`trigger_multiple`, wait 3; a 20-damage window between 0.5 and 1.0 s);
   - the oyster cycles (about 4.9 s each: close, one hurt pulse, block up 100, reopen);
   - the steam-vent hurt #45 above the launch push #537.
4. **Guide node events,** in seconds after `startturtle`:

   | Time | Node | Event |
   | --- | --- | --- |
   | about 22 | `tp26` | `Fish2Dart` |
   | about 29 | `tp31` | `Spike1Fall`, then `Spike2Fall` |
   | about 35 | `tp36` | `Spike3Fall`, then `Spike4Fall` |
   | about 39 | `tp39` | `ClamAttack1` |
   | about 45 | `tp45` | `ClamAttack2` |
   | about 58 | `tp55` | `Column3Fall` |
   | about 66 | `tp61` | `FishCalm` |
   | about 82 | `tp72` | `Panels` |
   | about 105 | `tp90` | `ClamAttack4` |
   | about 119 | `tp101` | `Pillar1Fall` |
   | about 125 | `tp105` | `Panels2` |
   | about 138 | `tp115` | `Pillar2Fall` |
   | about 178 | `tp145` | `Panels4` |

   The `tp102`-`tp124` stretch lies inside water volume #32.
5. **Breakwall** at node `tp158` (about 193.4 s): quake; `exit_trigger` becomes triggerable; the fake wall hides; `piece1`-`4` are thrown. The dry exit shaft beyond (`tp159`-`tp161`, emitter #31) opens. **Gate:** the guide reaches `tp158`.
6. **Exit scene.** `exit_trigger` #30 (x -2176..-2112, just east of the fake wall) runs `Utemple_Exit_Cinematic`: `utemplecam` follows `utemple_path1`, the player is hidden and invulnerable, and the fake Alice swims `alice_path` (8 nodes, 7 s), then the script changes level to `garden1$garden1_start1`.

**Exit:** `trigger_once` #30, then the script `map()` (`utemple.scr:895`), leading to `garden1$garden1_start1`. Enabled only after `breakwall`.

**Enemies and hazards:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Snark (full, 25 hp, bite 10) | 4 (#41, #42, #46, #531) | Placed | no |
| Oyster traps (`utemple_oyster_trap`) with hurt 1-4 and blocks 1-3 | 4 | `oyster3` from start; `oyster1`/`2` at `tp39`/`tp45` (+3 s); `oyster4` at `tp90` (there is no `oyster_block4`) | no |
| Fish-head trap with `fishhead_hurt` (20) | 1 | #28, wait 3 | no |
| Steam-vent hurt #45 | 1 | Always on | no |

**Cinematic:** `Utemple_Exit_Cinematic` (`utemplecam` #29, `cams/utemple_path1`, `fake_alice` #550 on `alice_path` #16). **MVP:** the track, the fake-Alice puppet for 7 s with the player hidden and invulnerable, then the transition. The skip commits the same transition.

**Movers:**
- **Guide turtle** on `tp2` with node threads.
- **Collapse set pieces:** `column1a`/`1b`, `column2`, `column3b`/`3c`, `column6`, `spike1`-`4`, `pillar1`-`2`, `panel1`-`12`, `bigpanel`/`bigpanel2`, plus the surface-only beams 1-3 and their bound bubble emitters. These are timed relative rotate/translate programs (0.5-7 s segments, some with acceleration). Crush `dmg` is 10 or 100 (0 on panels 9/10). `beam2` is never shown and `panel11` never falls.
- **Breakable wall:** `fakewall` #23 plus `piece1`-`4` #19-#22.
- **Water volume #32** (liquid only; never rendered or solid).
- **Oyster traps** (`shell_oyster.tik` clips, nodraw blocks).
- **Fish-head trap.**
- **Bubble-stack launch pushes** #47, #533, #537, #539 (to targets `t221`-`t224`; they use land-gravity apex math).
- **Fish schools and lanternfish** (decor; `FishDart`/`FishCalm` swap paths).

**NPCs and dialogue:** none. The Mock Turtle is a silent guide. There are no catmessages.

**Special mechanics:**
- fully submerged traversal with the 20 s shell air budget;
- air from breath points spawned by `fx_mockturtle_launcher` models: the guide, plus `panel8_bubbles` #406 and `panel10_bubbles` #410 until their panels fall. `panel11_bubbles` #412 stays hidden, #31 is in the dry shaft, and `bigpanel2_bubbles` #528 uses the no-air model;
- an escort guide on its own schedule;
- node-keyed world events;
- timed collapses with crush damage;
- hurt windows;
- invulnerability until `startturtle`;
- a breakable exit wall.

**Rewards:** the breathing upgrade (`turtle_air`, 20 s; persists); the `full_stats` refill; essences (small #12/#13/#560-#562 on Easy only and #559 on Easy and Normal; medium #14/#15/#43/#529/#530; super #26).

**Entity classes:** `info_player_start`, `trigger_once`, `trigger_multiple` (wait 3), `trigger_hurt` (controller-owned enable and pulses), `trigger_push` (target launches), `func_earthquake`, `func_camera`, `script_object` (37 movers/solids/visuals plus #32 as liquid), `script_model`, `info_splinepath` (the guide path with node threads), `utemple_oyster_trap`, `utemple_fishhead_trap`, `utemple_bubble_chest`, `emitter_BubbleEmitter_spensive`/`_noair` (the model key decides air), `Enemies_Snark`, `Item_MetaEssence_*` (difficulty flags).

**PLAYABLE MVP checklist:**
- [ ] Private read-only research (U2, U3):
  - the `bubbles` touch routine and whether it calls the air reset (5 s + 15 s with the shell flag, matching `water.rs:48-49`);
  - whether hidden launcher emitters keep spawning;
  - whether the model key or the classname picks emitter behaviour (#406, #528);
  - ScriptSlave spline timing and `loop` on open chains (`tp2` ends at `tp161`);
  - the original underwater swim speed against the guide.
- [ ] W13: register brush-entity liquids (#32 now). Add a `--swim-check` case asserting that node `tp105` is submerged.
- [ ] T4: `Breath::refill()` plus a deterministic breath-point module (guide launcher whenever the swim clip plays; #406/#410 until their falls; #412 hidden; #528 visual only; one point per second, each living 4 s). Render `fx_mockturtle_launcher`, and save points and clocks.
- [ ] Controller stages:
  - Waiting: controller-owned invulnerability and no drowning;
  - Guided: the guide on `tp2`, node events fired once and in order;
  - Open: after `breakwall`;
  - ExitScene;
  - then the transition.

  Apply `full_stats` once on first entry.
- [ ] Collapse programs (F4 objects) with idempotent triggers, crush with push-out, surface-only beams per program, and quake hooks.
- [ ] Hazards: `oyster_hurt1`/`2` and `fishhead_hurt` off at load; `oyster_hurt3`/`4` on until their first pulse, then single-tick pulses; the oyster cycles with rising blocks; the fish-head window (wait 3); #45 stays on. Decide what `oyster4` does, since it has no block (default in How to read).
- [ ] The exit scene with its transition.
- [ ] R1 `swim_to`/`swim_follow` (requires W13 first).
- [ ] Saves: migrate old cached utemple visits to a fresh controller at the entrance, keeping inventory and `turtle_air`. The real v0.26 save `private/duchess-native-reward/auto.json` has utemple as its current visit. Cases: `utemple-mid-guide` (live bubbles, partial air), `utemple-mid-collapse`, `utemple-oyster-closed`, `utemple-exit-scene`.
- [ ] Launcher `tools/launchers/Launch-Wholly-Morel-Ground.cmd`; `docs/UTEMPLE.md`.

**Route check (`--utemple-route-check` and skip variant; only after W13):**
- Enter with the Duchess-chain Stats (`turtle_air` true).
- Enter `startturtle`.
- `swim_follow` steers toward the guide's current position (fallback: the next `tp2` node) with run, never warping, and breathes through breath-point contact.
- Per tick: alive; body clear; `breath.hits == 0`; submerged through `tp102`-`tp124`; trap and collapse damage within budget.
- Node events fire once and in authored order; `breakwall` happens; swim into #30.
- Watched waits the 7 s scene; the skipped run commits the same.
- Assert:
  - `transition == ("garden1", Some("garden1_start1"))`;
  - `turtle_air` retained;
  - teleports == 0;
  - no pending-script output.
- A contract covers the idempotent `Panels4`, the oyster timing, the initial hurt states, and invulnerability ending at `startturtle`.

**Anode checks:**
- [ ] `--utemple-render-check`: the guide emitting bubbles, collapses mid-motion, oysters open and closed, the fish-head attack, the broken wall, the exit camera with the fake Alice, and the underwater fog inside #32.
- [ ] Real input: swim (WASD/Space/Ctrl/Shift) behind the guide for at least 60 s. Screenshots show the air meter refilling on breath contact. F5 mid-guide, then a fresh `--load quick` gives an identical guide clock, bubbles and air.
- [ ] From a staged save near `tp105`: Alice swims rather than walks inside #32.
- [ ] From a staged save after `breakwall`: once watched and once skipped. garden1 loads and autosaves, and a fresh `--load auto` keeps inventory and `turtle_air`.
- [ ] Oyster and fish-head damage with real input.

**Fidelity follow-ups:**
- Snark swimming AI detail;
- fish schools and lanternfish paths;
- the no-air bubble show/hide on collapses;
- the `underwater_rumble` sound;
- quake shake;
- bubble-chest animations;
- the shell attachment on Alice;
- the turtle swim loop sound;
- exact spline curves;
- launch math under water.

**Risks:**
- **Air fairness:** only the moving guide and two panel launchers supply air, and the guide bursts past the provisional swim speed.
- The bubble semantics are unconfirmed.
- 37 relative-motion movers; a wrong final pose can block or trap.
- Hurt volumes that are permanently live today distort the damage budgets.
- The guide `loop` flag and duplicate event sources.
- There is no underwater navigation in the harness.
- The legacy save migration.

**Refs:** `maps/utemple.scr` (main, `startturtle`, `breakwall` ~844, `Utemple_Exit_Cinematic` ~883-896, `PanelFall*`/`Spike*Fall`/`Pillar*Fall`/`Column*Fall`, `ClamAttack*`, `FishHeadAttack`, `AnimFish`, `FishDart`/`FishCalm`); models `c_mockturtle.tik`, `fx_mockturtle_launcher.tik`, `prj_mockturtle_breath.tik`, `fx_bubbles_noair.tik`, `shell_oyster.tik`, `fish_head.tik`; entities #12-#47, #49, #51-#98, #174-#177, #233-#386 (`tp2` chain), #391-#412, #445, #485-#488, #521, #528-#550, #559-#562.

---

# Gardens (M2)

Shared M2 systems:
- the brush-mover kit and reviewed collapse programs (W1 plus F4 objects). The numbers are transcribed into Rust tables and verified by `--level-spec-check`; a runtime reader of move lines like `sky_sequence::motions` would fall under DG-2 B2;
- `func_door` (W2), `func_fallingrock` (W9), `info_grav_pathnode` (W12), the fulcrum static pose (W11);
- the garden3 fog loop through `#include` (W20);
- the Ladybug generalization (E5), Antlion (E7), Mushroom and Bloodrose (E4);
- the caterpillar facial rig (C3);
- route primitives: swim, rope climb and descent, ride, launch, chase-with-lead (R1).

<a id="v13-garden1"></a>
## 13 · garden1 (Dry Landing)

| Field | Value |
| --- | --- |
| Visit | `garden1$garden1_start1` (#170 at -952 -2336 -616, yaw 210, inside a 1,216-unit-deep pool; water z -1664..-448) · ROUTE 12 · next `garden2$garden2_start1` |
| Status | **BLOCKED.** The lily pads and their clip brushes are missing, so rope #482 is out of reach. `info_grav_pathnode` is unsupported |
| Size / milestone | L · M2 |
| Depends on | W1, W5, W6, W12, W14 (recovery portal), W15 (fidelity), E2, E3, E4, E5, E6, C1, C3, T6 (the load-time `full_stats`; the idempotent shell grant, `w_turtleshell`), R1, R4 |
| Reservations | hit-ID 7,200,000 · `garden1/` · `garden1.` · `--garden1-*` · `garden1-*` |

**Today:**
- No controller or encounters.
- The 10 TRIGGER_SPAWNED enemies (spawnflags 64) never appear (`npc.rs:78-81`). The placed enemies are inert: 14 on Normal, 6 on Easy, 17 on Hard.
- The Mock Turtle, both White Rabbits and 11 walkrocks stand at their editor poses.
- The 12 Ladybug func_spawns never fire.
- Script triggers are pending. `g1_changelevel` and teleporter #2 are always live.
- Inline brush entities are neither solid nor drawn. The lilypad props are "owned" and skipped (`decorations.rs:173`).

Working today: the underwater spawn with `push_start1` launching Alice out of the pool; swimming and bank climbs; 8 ropes; target-apex pushes; the 3 unnamed catmessage hints; pickups; the `turtle_air` flag.

**Arrival.** `Garden1_Cinematics_Init` calls `full_stats` at load, so Sanity and Will refill on arrival. It also:
- places `rabbit_actor` at `rabbit_posx1`, visible and idling until its scene;
- places `rabbit_actor2` at `rabbit_follow1` and `turtle_actor` at `turtle_pos1`.

main then runs `Garden1_Start` at once.

**Progression and gates:**
1. **Arrival scene** (`Garden1_Start` and `Garden1_Cinema2`; skip `Skipthread_G1`/`G1_C1_End`):
   - the fake player waits hidden with gravity 0 and `push_start1` off; after 2 s, gravity on and `push_start1` (to `t248`) on, launching it out of the pool;
   - the turtle conversation, 5 lines. After line `mkt011`, the shell is granted (`w_turtleshell`) and its prop attached on `tag_back`;
   - the turtle swims away;
   - the fake player warps to `turtle_waterpos1` (-1368 -2472 -384).

   `push_start1` is removed straight after the knockdown on both paths.
2. **Valley north.**
   - `Lady_Bug1_On` #121: `lady1` on, and TS ants `t236` (#495, plus #494 on Hard);
   - hint #52;
   - `Lady_Bug2_On` #120;
   - `Ant_Ambush1` #118: `get_lady1` on spline `t312`; `ant_run2` #481 (not Easy) runs to `ant_pos2`.
3. **Lily pads to rope #482.** `lily2`, `lily4` and `lily99` (#180/#181/#594, with bound clips #213/#212/#595) loop `lily_path1` (5 nodes), `lily_path2` (6) and `t395` (5) at speed 0.1 per node on the pond (water -504..-384). `lily2`'s loop passes under rope #482 (z -288..168, bottom about 96 above the surface). Jump, catch the rope, and climb to the west trail. Snarks #87 (Normal/Hard) and #601 (Hard) swim here. **This appears to be the only way out of the start basin** (coarse probe; U9).
4. **West trail** to the launch ledge (about z 212 near -1518, -532). `Lady_Bug3_On` #114.
5. **Launch pad into rope #614.** Walk off the ledge into the mid-air `trigger_push` #74 (z 232..240), which launches to apex `t426` (-1664 -416 736) beside rope #614 (z 632..1040). Catch it in the air (`Spawn_Ladybug4` #69 wraps the rope), swing and release east onto the rabbit ledge (z about 672).
6. **White Rabbit scene.** Trigger #58 runs `Garden1_Rabbit_Cinema1`: the fake player at `alice_posx1`, 5 lines, the rabbit runs and jumps away, then is removed. Skip `Skipthread_GR1`/`GR1_End`.
7. **Ledge east, down rope #334.** `Ant_Ambush2` #113 (`ant_corner1`, not Easy); hint #51; `Lady_Bug4_On` #111 (`lady7`/`8`/`9` on `t414` at 0/1.6/3.2 s; bloodroses `t382`).
8. **Rope #352 to the plateau.** At the plateau: `Spawn_Ladybug3` #72, and `Open_Portals` #1, which shows and solidifies the teleporter frames #4/#704 and enables teleport #2 to `teleporter_end_dest` (-32 952 792). Corporal #663 (not Easy) waits here.
9. **Upper river crossing.** Water z 704..936. The grav current runs from head #700 through `t454`..`t461` (96 u/s) and `t374`..`t381` (128 to 450, radius 312) to waterfall g2, then through #105, `t310`..`t323` to waterfall g1. Cross near (-400, 2128), where the pull is weakest. If Alice is swept away, the opened portal returns her. Snarks #35 and #662 (Hard).
10. **North ridge to the exit.** `Garden1_RockFall` #62 targets a missing `$end_fallingrock` and does nothing; rock #61 stays static. Exit #131.
11. *Optional east loop:*
    - the Jackbomb altar #44 and RageBox #36;
    - `Ant_Ambush3`/`4`, `Ant_Deadtree_Ambush` #85 (`rabbit_actor2` flees and is removed);
    - the collapsing stone bridge `Bridge_Drop` #207 (`fall1`-`7` rotate ±55 and drop 2400; one-way);
    - pads #483/#569; ropes #123, #351, #353, #354;
    - EvilMushroom #48.

**Exits:**
- `g1_changelevel` #131 (x -1696..-1424, y 3752..3784) leads to `garden2$garden2_start1`. Ungated.
- Recovery teleport #2 goes to #3, and stays off until `Open_Portals`.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Ladybug residents `lady1`/`lady2` | 2 | Patrol loops `t177`/`t195`; attack after #121/#120; `Arm_Ladies` rearms every 2 s | no |
| Ladybug func_spawn (`lady3`-`lady14`) | 12 | Reviewed table: thread, then func_spawn, spline and delay (all splines loop) | no |
| Army Ant | 8 | TS pairs `t236`/`t337`; scripted runs `ant_run2`, `ant_corner1`, `ant_run_deadtree1`; #47 placed | no |
| Corporal | 3 | `ant_corp1`/`ant_corp2` (TS plus runs), #663 | no |
| Bloodrose | 9 | TS groups `t382`/`t416`/`t131`, plus 5 placed | no |
| Snark (swimmer) | 4 | Pond and river | no |
| Evil Mushroom | 1 | #48 | no |

**Cinematics:**
- `Garden1_Start` and `Garden1_Cinema2` (cameras `garden1_path1`/`2`/`4`; `alice_watch` #92, `turtle_watch` #583). **MVP:** the hidden Alice puppet at the spawn, the launch arc and knockdown, the turtle conversation (5 beats), the shell on `tag_back` after `mkt011`, the turtle's swim spline. The commit sets `turtle_air`, hides the turtle, disables `push_start1`, and lands Alice at `turtle_waterpos1` yaw 0. The `full_stats` refill happens on fresh visits only.
- `Garden1_Rabbit_Cinema1` (`garden1_path3`). The commit removes `rabbit_actor` and lands Alice at `alice_posx1` yaw 0.

**Movers:**
- lily pads with bound clips;
- launch pad #74 into rope #614;
- `Bridge_Drop` `fall1`-`7` (optional);
- the recovery portal frames;
- the river grav currents (19 nodes);
- end rock #61, static.

**NPCs and dialogue:**
- Mock Turtle `mkt010`-`012`, with Alice `alcz2020b` and `alcz2021`;
- Rabbit `rbt001`-`003`, with Alice `alcz2022` and `alcz2023`;
- catmessages #50/#51 (`catz212`) and #52 (`catz214`) are unnamed hints.

The Turtle and Rabbit use `maxmouthangle 45`.

**Special mechanics:**
- the pool arrival launch;
- the `full_stats` refill at load;
- lily-pad riders;
- the launch-pad air-catch of a rope;
- rope climbs;
- the grav-current river and gated recovery portal;
- the one-way collapsing bridge;
- scripted ambush runs.

**Rewards:**
- the shell (`turtle_air`) is already true, so the grant is idempotent;
- the refill;
- the Jackbomb duplicate (#44, refills Will) and the RageBox;
- essences and vials by difficulty (Normal: 9 essences and 4 Will vials).

**Entity classes:** `info_player_start`, `trigger_once`, `trigger_changelevel`, `trigger_teleport` plus `func_teleportdest` (gated), `trigger_push` (target apex; current), `trigger_fall`, `trigger_catmessage`, `func_rope`, `info_grav_pathnode`, `script_object`, `script_model`, `info_splinepath`/`info_waypoint`/`info_pathnode`, `func_spawn`, `func_earthquake`, `func_camera`, `Enemies_*` (Ladybug, Army-Ant, Corporal, Bloodrose, Snark, EvilMushroom, Mockturtle), `Characters_WhiteRabbit`, `Characters_Walkrock-*`, `SFX_GnomeHide`, `emitter_BubbleEmitter_spensive`, items.

**PLAYABLE MVP checklist:**
- [ ] Registry controller `src/levels/garden1.rs`: lily pads plus clips (loops, riders), `fall1`-`7` (collapse program), teleporter frames (hidden and non-solid until `Open_Portals`), static rock #61.
- [ ] Rules: `Open_Portals` enables #2 and shows the frames; `Bridge_Drop` runs the collapse, quake `t131` and the `lady3` spawn; `push_start1` is disabled on commit; `Garden1_RockFall` is recorded as a no-op.
- [ ] Arrival scene and Rabbit scene (C1) with their commits; `full_stats` on fresh visits.
- [ ] W12 grav currents (researched first). Verify the crossing near the head is swimmable, and that a fall into the fast section goes over waterfall g2.
- [ ] Encounters: residents plus the reviewed ladybug table; TS groups; scripted ambush runs; the families from F5.
- [ ] Cast rules: `rabbit_actor` stays visible at `rabbit_posx1` until its scene; `rabbit_actor2` is removed after the dead-tree ambush; the turtle is hidden after the arrival.
- [ ] Recovery after the arrival uses `turtle_waterpos1`, never the underwater spawn.
- [ ] Saves: migrate old generic visits (scene done; rearm pending triggers; relocate if overlapping new solids). Update the `movement-breath` save_check fixture (`save_check.rs:238, 377`), because staging at the fresh spawn now plays the scene. Cases: mid-arrival line, lily ride, hanging on #614, after `Open_Portals`, mid `Bridge_Drop`.
- [ ] Launcher `tools/launchers/Launch-Dry-Landing.cmd`; `docs/GARDEN1.md`. Note that `tools/launchers/Launch-Swimming-Preview.cmd` now plays the arrival scene.

**Route check (`--garden1-route-check` and skip variant):**
1. `wait_for_cinematic`; assert `turtle_air`, feet near `turtle_waterpos1`, `push_start1` disabled.
2. Navigate (-1296, -1936, -331) [#121], then (-1552, -1392, -272) [#120], then (-1808, -1136, -255), then the pond bank.
3. `ride_mover` on `lily99`/`lily2` until under #482; jump and E-catch; climb to about 160; release onto (-2256, -400, 34).
4. Navigate (-2128, -336, 76), then (-1616, -248, 208), then (-1518, -532, 212) [#114].
5. Walk west into #74; air-catch #614; climb, swing and release onto (-1300, -330, 672).
6. Walk through #58 and wait for the scene; assert feet near `alice_posx1` and the rabbit removed.
7. Navigate (-740, -310, 638), then (-448, -136, 412) [#113]; climb down #334; then (208, -272, 104) [#111] and (272, 464, 82) to the ledge edge.
8. Climb #352 to about 1110; walk to (-400, 1104, 1168) [#72, #1]; assert teleporter #2 is enabled.
9. `swim_to` across the river near (-400, 2128, 936) with the current active.
10. Navigate (-464, 2192, 1040), then (-656, 2352, 1137), (-1128, 2936, 1008), (-1512, 3112, 1072), (-1576, 3560, 1216); hold north into #131.

Assert `transition == ("garden2", Some("garden2_start1"))`, teleports == 0, and Sanity > 0.

Variants:
- the east loop over the bridge;
- a deliberate river fall recovered through the portal (teleports == 1).

The coordinates come from pathnodes and a coarse probe. Confirm them with `navigate` before freezing them.

**Anode checks:**
- [ ] `--garden1-render-check`: the launch arc and knockdown; the turtle two-shot with the shell; the turtle dive; the rabbit idle before and during its scene; a lily pad under #482; the #614 catch; the portal frames before and after; the bridge mid-collapse; the river surface.
- [ ] Fresh visit: watch 5 s, hold Enter, verify the handoff pose and full Sanity/Will; F5, kill, `--load quick`, F9: scene complete, identical pose.
- [ ] Real input: board `lily99`/`lily2`, E-catch #482, climb; walk into #74, catch #614, swing, release. F5 while hanging, restart, F9 restores the grip, height and momentum.
- [ ] Rabbit scene: F5 mid-line, restart; it resumes paused on the same line.
- [ ] Walk into #131: garden2 loads and autosaves; a fresh `--load auto` keeps inventory.

**Fidelity follow-ups:**
- exact launch arc and fades;
- head-watch;
- the shell fade-in and sparkle;
- turtle acting;
- waterfall spray and bubble launchers;
- quake `t131`;
- walkrock AI;
- the Ladybug rearm cadence;
- Mushroom presentation.

**Risks:**
- Grav-current semantics (U1) could make the river impassable.
- The lily pads are the likely only way out of the start basin, so a rider bug makes the level unwinnable. Add a real-map rider regression.
- Checks that stage the fresh spawn now play the scene.
- Missing script targets must stay no-ops: `lily5`/`6`/`8`, `clip5`/`6`/`8`, `t353`/`t373`/`t367`, `ant_run1`/`10`/`11`, `ant_nearstart1`, `boulder2`, `fall8`, `end_fallingrock`, `t384`.

**Refs:** `maps/garden1.scr` (1-341: `Open_Portals` 14-21, `Garden1_World_Init` 246-259, `Bridge_Drop` 276-341, `Arm_Ladies` 261-268); `maps/cinematics/garden1_cinematics.scr` (10-54, 63-94, 96-215, 217-328); `utemple.scr:883-896, 937`; entities #1-#4, #36, #44, #47-#52, #55, #58-#62, #69-#75, #80, #85, #86, #87, #90, #92, #93, #99, #101, #105, #109, #111-#123, #127-#135, #170, #180, #181, #199, #207, #211-#213, #224, #225, #334, #351-#354, #481-#483, #494, #495, #508, #540, #550, #569, #577-#587, #594, #595, #601, #602, #614, #636, #660, #662, #663, #693-#701, #704.

<a id="v14-garden2"></a>
## 14 · garden2 (Herbaceous Border)

| Field | Value |
| --- | --- |
| Visit | `garden2$garden2_start1` (#173 at 2424 -2576 -136, yaw 60; thread `Garden2_Start`) · ROUTE 13 · next `garden3$garden3_start1` |
| Status | **PLAYABLE.** Fresh watched/skipped Normal routes cross both collapses, the underworld and surface vines, and enter garden3 with live enemies and saved continuations. See [GARDEN2.md](GARDEN2.md). |
| Size / milestone | XL · M2 |
| Depends on | W1, F4 collapse programs, W11 (saved, rider-responsive fulcrum #86), W15, W16 (speakers), W20 (existing fog layer), E2, E3, E4, E5 (the duplicate `get_lady5`), E7 (Antlion), C1, C2 (miniature Hatter, scene-local slow motion, dissolve), C3, R1 (rope #640 descent, vent rides, slime-aware paths), R4 |
| Reservations | hit-ID 7,300,000 · `garden2/` · `garden2.` · `--garden2-*` · `garden2-*` |

**Verified implementation (2026-09-30):**
- The registry owner restores arrival, both bridge collapses and the final Cat, with the same watched/skipped completion rules.
- All seven Ladybug patrols and the saved Corporal ambush run are active. Other enemy families use the production resident cast.
- The fulcrum tilts under its rider with shared visual/collision poses and saved angles.
- Actual traversal exposed and fixed shallow-slime jump cancellation and diagonal ledge-grab reach. Both retain ordinary collision, damage and saved movement.
- The native route driver covers the cave and surface path on Normal. Optional branches and all-difficulty route coverage are separate from this proof.

**Arrival: the squish scene** (unskippable in the original, DG-4):
1. `Garden2_Start`: fade in; the fake player at `alice_pos1`; the Rabbit runs; camera `garden2_path1`.
2. `Garden2_Start_Cinema2`: line `rbt004`, camera stop/continue, the `hatter_step5` quake.
3. `Garden2_Squish` and `Garden2_Squish_Cinema2`/`3`:
   - slow-motion Hatter steps at timescale 0.5;
   - `CinemaRabbit`;
   - `CinemaStomp` at timescale 0.8: the miniature `hatter_actor_tiny`, warped to `rabbit_tiny_pos1`, stomps `rabbit_actor_tiny` (scale 0.1, removed) on the forced-perspective set near (-3640, -3320), ending in `ready`;
   - the `hatter_step1`-`6` quakes and `hatter_*` speakers.
4. The Rabbit is hidden and `rabbit_actor_dead` shown. Alice runs to `alice_pos_squish2` (910 -1282 -392), kneels and weeps (`alcz2024`).
5. The Cheshire `cat_actor1` fades in at `cat_squish_pos1` (`catz213`), then out, and is removed.
6. `Garden2_Squish_End`: `Spawn_Lady1`, then End_Cinematic_Fast. The warp target `alice_squish_pos2` is missing, so Alice stays at `alice_pos_squish2`.

**Progression and gates:**
1. Squish scene.
2. **Upper middle and east.** TS groups via trigger targets `t105` (#148), `t112` (#147), `t116` (#140), `t159` (#67), `t126` (#95). `Spawn_Lady4` #71.
3. **Ramp down; bridge 1 collapses ahead of Alice.**
   - Path: (720, -1712, -403), (592, -2032, -94), (912, -2352, -389), (1296, -2512, -592), through the `upperfog1`/`underfog1` pair (#119/#120), to (1712, -2832, -758).
   - `bridge_trigger_hole2` #129 is a plane at x about 1888 on solid approach ground, about 200 units before the deck. It runs `Hatter_StompBridge1` and `Collapse_Bridge2`: cameras `garden2_jump1`/`garden2_jump2`, quakes and speakers, and `bridge_fall6..1` rotate ±55 and drop 2400 over 5-5.4 s.
   - **Alice is released where she stood, with the bridge gone.** The fake player is never warped.
   - The approach continues as a walkable slope north (x about 1950: z -927, then -832, then -1052) onto the Mallet altar ledge (#57 at 1998 -2166 -1036), across a crack of about 50 units, to a -1120 ledge next to **rope #640** (z -1888..-1176), whose lower end is about 84 units above the pit floor.
   - From the north, #130 (plane y about -1508) runs `Collapse_Bridge1` instead.
4. **Under world.**
   - The floor is slime (z about -2088..-2016) with dry mounds (about -1963..-1992).
   - Vent pairs: #635/#636 to `t120`, #633/#634 to `t119`, #654/#655 to `t124`, #143/#628 to `t114`, #630/#144 to `t115`, #660/#661 to `t127`, #669/#670 to `t130`. Pads #128 (to `t118`) and #97 (to `t125`).
   - Ropes #640, #138, #59, #92, #662.
   - TS groups `t132` (#81) and `t160` (#63).
   - `Ant_Ambush2` #141 (`ant_corp_under1`).
   - The Tea at (-160, -1544, -1976); Mushrooms #89/#123.
5. **Second bridge collapse onto the fulcrum.** `Collapse_Second_Bridge1` #114 (on the fulcrum) runs `Hatter_StompBridge2` (`garden2_jump3`/`jump4`; the fake player is warped to `fakeplayer_bridge_pos2` (1040 -784 -1472) on `func_fulcrum` #86). `secbridge_fall1`-`12` drop: three walkways radiating from the fulcrum. Then `bridge_ant_spawn1` (#673 to `t131`, two ants).
6. **West under world.** `lastcat_trigger2` #52 or `lastcat_trigger1` #53 runs `Garden2_Cat_End`: `last_cat` fades in at `cat_end_pos` (`catz207`), and both triggers are disabled. Skippable.
7. **The Cards-altar ramp** (altar at -1106 -2482 -640) rises west through `underfog4`/`upperfog4` to the upper west: `Spawn_Lady2` #79, `Spawn_Lady3` #74, ropes #713/#87; kill volumes #60/#61/#73/#693/#895/#896 fill the west chasm (z -1632..-512). Exit #162.

**Exit:** `trigger_changelevel` #162 (x -2488..-2152, y 800..864) leads to `garden3$garden3_start1`. Ungated.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Antlion | 6 (TS) | `t159`, `t116` x2, `t112`, `t133` x2 (not Easy) | no |
| Bloodrose | 8 | 6 TS (`t133` x3, `t126` x2, `t105`), 2 placed | no |
| Army Ant | 7 (TS) | `t132` x2, `t131` x2 (the second-bridge end), `t116`, `t112`, `t159` | no |
| Corporal | 2 | `t160`; `ant_corp_under1` (#141 run) | no |
| Evil Mushroom | 3 | Placed #78, #89, #123 | no |
| Ladybug (func_spawn `lady1`-`6`) | 7 | `Spawn_Lady1`-`4`. The two func_spawns both named `get_lady5` (#70 near `t157`, #736 near `t149`) each spawn once, on their own spline. Rearm loop | no |

**Cinematics:**
- **Squish scene.** Puppets; the miniature Hatter and tiny rabbit; scene-local slow motion; quakes; beats `rbt004`, `alcz2024`, `catz213`. The commit (watched or hold-Enter skip, DG-4): `rabbit_actor` and `rabbit_actor_tiny` removed; `rabbit_actor_dead` shown; `cat_actor1` removed; the tiny Hatter `ready` at `rabbit_tiny_pos1`; `Spawn_Lady1` queued (`lady2` +2.1 s); Alice landed at `alice_pos_squish2` yaw 135.
- **`Hatter_StompBridge1` with `Collapse_Bridge1`/`2`.** Cut-ins and the physical collapse. The commit: all `bridge_fall` pieces fallen, hidden and non-solid; both hole triggers consumed; under fog kept; **Alice restored at her pre-scene pose with zero velocity** (no warp, no landing search).
- **`Hatter_StompBridge2` with `Collapse_Second_Bridge1`.** The commit: `secbridge_fall1`-`12` gone; Alice landed at `fakeplayer_bridge_pos2` on the now-solid fulcrum; the `t131` ants active; #114/#673 consumed.
- **`Garden2_Cat_End`.** Cat fade and `catz207`. The commit: `last_cat` removed, #52/#53 disabled, Alice's pose unchanged.

**Movers:**
- bridge 1 (`bridge_fall1`-`6`; triggers #129/#130);
- the second bridge (`secbridge_fall1`-`12`; #114, #673);
- fulcrum #86 (limit 12, speed 2; rider tilt and unloaded return);
- steam vents (push plus negative-speed accelerate);
- fog zones (existing);
- static rocks `entrance_rock1`-`3` and the difficulty pieces #16/#17/#18/#888 (absent on Hard).

**NPCs and dialogue:**
- Rabbit `rbt004`; Alice `alcz2024`; Cheshire `cat_actor1` (`catz213`) and `last_cat` (`catz207`), both at alpha 0 outside their scenes;
- the silent miniature `hatter_actor_tiny` (clips `ready`, `jump`, `stomp_yo_ass`) and `rabbit_actor_tiny`.

Missing references stay no-ops: `hatter_actor`, `jump1`-`3`, `t3`/`t5`/`t10`, `alice_squish_pos2`.

**Special mechanics:**
- a slime-floored under world;
- bridges collapsing ahead of Alice;
- steam updrafts;
- forced-perspective miniatures;
- scene-local slow motion (never `FIXED_DT`);
- the fulcrum floor;
- fog switching;
- Grasshopper Tea.

**Rewards:** duplicates only: the Cards altar #46 and Mallet altar #57 (Will refills). Tea #47. Essences (Normal: 14).

**Entity classes:** `info_player_start` (thread), `trigger_once`/`trigger_multiple`, `trigger_changelevel`, `trigger_push` plus `trigger_accelerate`, `trigger_fall`, `func_rope`, `func_fulcrum`, `script_object`, `script_model` (`c_whiterabbit_stomped`), `sound_speaker`, `func_earthquake`, `func_camera`, `func_spawn`, `info_null`/`info_waypoint`/`info_pathnode`/`info_splinepath`, `Characters_CheshireCat`, `Characters_WhiteRabbit`, `Enemies_MadHatter` (cinematic), `Enemies_AntLion`/`Bloodrose`/`Army-Ant`/`Army-Ant-Corporal`/`EvilMushroom`, `Characters_Walkrock-*`, `garden_firefly`, steam emitters, slime brushes, items.

**Completed playable scope:**
- Physical bridge collapse tables, entrance rocks, difficulty pieces and the tilting fulcrum.
- Fresh-arrival, bridge and Cat scenes with watched/skipped commits, cast visibility, saved dialogue and legacy visit migration.
- Trigger-spawned enemy groups, independent duplicate Ladybug spawners, patrols, loot and the Corporal's one-shot run.
- `tools/launchers/Launch-Herbaceous-Border.cmd` and [GARDEN2.md](GARDEN2.md).

**Route proof (`--garden2-route-check`, both scene outcomes):**
1. Arrival, upper approach, south bridge trigger and preserved approach handoff.
2. Southeast slime descent, bank escape and the two thermals into pad #128.
3. Mallet detour and root path into the second collapse and fulcrum.
4. Pad #97, launch ledge, running jump and northern tunnel to Cat trigger #52.
5. Thermal #661, vines #92/#662, two wall ledges and the Cards ramp.
6. The diagonal exit steps, western surface path, vine #87 and essence ledge.
7. Surface encounters, final rabbit-hole descent and the real #162 exit into `garden3$garden3_start1` with unchanged carried resources.

The complete measured input goals are stored in `src/levels/garden2/route_steps.json`.
Thirteen disk saves per route compare live futures and then resume the main route
from the saved state. No health grants, god mode, flight, player-position injection
or forced story completion are used. Normal hazards remain active.

**Native and regression verification:**
- Six rendered scene fixtures; six separate-process native save writer/reader cases, including both falling bridges.
- All watched/skipped scene handoffs, trigger migration, seven patrols, saved ambush run and directional fulcrum rider checks.
- Full route with the production native cast; saved updraft, fulcrum, launch ledge and all three vine grips.
- Shallow-liquid and diagonal-ledge regressions, existing Pool/forest ledge cases, and adjacent Rolling Stones route/save proofs.

**Fidelity follow-ups:**
- the miniature Hatter's jump and stomp timing;
- camera stop/continue;
- fades and timescale ramps;
- Cheshire acting;
- 10 quakes and 13 speakers;
- scene farplane overrides;
- fireflies;
- walkrocks.

**Risks:**
- Optional cave branches and other difficulty routes have not been certified by this path.
- Unskippable scenes made skippable (DG-4).
- The duplicate func_spawn names.
- The Cheshire ownership change.

**Refs:** `maps/garden2.scr` (5-28 fog, 104); `maps/cinematics/garden2_cinematics.scr` (15-80 Cat end, 83-148 and 210-303 second bridge, 150-208 and 305-429 bridge 1, 431-477 ambush and ladies, 479-788 squish); entities #4, #16-#18, #46, #47, #52-#54, #56, #57, #59-#61, #63, #67, #70, #71, #73, #74, #78-#82, #85-#87, #89, #92, #95, #97, #114, #119-#120, #123, #128-#138, #140, #141, #143-#148, #160-#162, #167-#169, #173, #272, #541-#543, #595, #628-#636, #640, #642, #654, #655, #660-#662, #669, #670, #673, #693, #695, #713, #736, #888, #895, #896.

<a id="v15-garden3"></a>
## 15 · garden3 (Rolling Stones)

| Field | Value |
| --- | --- |
| Visit | `garden3$garden3_start1` (#90 at -1616 1080 -40, yaw 135) · ROUTE 14 · next `garden4$garden4_start1` |
| Status | **PLAYABLE.** Watched/skipped continuous routes reach garden4 with the live marble, both pads, gate dispatch and final drop. A slower route traverses the ice during collapse; 22 saved continuations reach the same exit. See [GARDEN3.md](GARDEN3.md). |
| Size / milestone | L · M2 |
| Depends on | W1, W2 (**toggle gates**), W9 (**marble**), W20 (the fog loop in the included cinematics file), C1, R1 (`chase_with_lead`), U1, U8 |
| Reservations | hit-ID 7,400,000 · `garden3/` · `garden3.` · `--garden3-*` · `garden3-*` |

**Verified implementation (2026-09-30):**
- The visit owner drives the real marble, gate transforms and collision, pillar/shroom, ice and ending pieces, camera handoff and fog cycle.
- Ordinary movement crosses the complete authored route; the outer descent avoids the fatal cavern shortcut. Both paces finish with 80 Sanity on Normal.
- Mid-chase disk saves replay identical input to the same garden4 arrival; native writers/readers cover six scene and five live-route moments.
- Later quake/audio feedback and settled-platform ledge support are restored. Exact original spin/quake waveform and stray Ladybug navigation remain fidelity follow-ups.

**Arrival (`Garden3_Start`, unskippable in the original, DG-4):**
1. `lady_model1`-`3` cheer at `bug_pos1`-`3`. `lady_model4`/`5`/`7`/`8`/`9` fly `bug_path1`/`2`/`4`/`5`/`6`. `big_bug1` (scale 3, `bug_marble1` on `tag_weapon`) flies `big_bugpath1`. `bug_path3` does not exist.
2. Cameras `garden3_path1`-`5` on `bug_watch1`/`alice_watch1`.
3. The non-solid fake player is at `alice_walk_pos1` (-1792 1504 -112).
4. `chase_marble1` is shown, given speed 175 and triggered.
5. 4 s later the fake player runs toward `alice_run_pos1` (-1704 2104 -336).
6. 1 s later: marble gravity 0.2; `Garden3_Start_End` (the fleet flies off on `lady_flyaway`; `Garden3_Loop_Farplane` starts); the marble becomes solid.

**Progression and gates:**
1. **Arrival scene.**
2. **Marble chase down the canyon.** `func_fallingrock` `chase_marble1` #89 (target `t124`, gravity 0.6, **dmg 999**, bounce quake) follows 56 `info_waypoint`s from `t124` to `crash3`, at speeds 170 to 320, about 90 units above a floor descending from -32 to -3136. Waypoint threads fire `Garden3_FirstQuake` (`t125`), `Garden3_Pillar_Break1` (`t44`), `Garden3_Quake1` (`t46`) and `Garden3_IceFloor_Break1` (`crash2`). Contact with the marble is lethal.
3. **Canyon gates** (`func_door`, angle -1, speed 10000; spawnflags 160 = TOGGLE+TARGETED, 161 also START_OPEN):
   - `t70` door #44 (START_OPEN) is toggled by #41 and closes behind Alice;
   - the `t71` doors #49 (closed), #46 (START_OPEN) and #45 (closed) are all toggled together by #48, #47 and #73.

   Each gate on the path opens ahead and closes behind. Travel is height plus 16: 464 units for #44/#49 and 496 for #45; #46 uses the default lip.
4. **Pillar crossing.** Pads #22 (to `t126`) and #379 (to `t127`) lead around `falling_pillar1` #29 (a clip brush, top about -1760, with shroom #354 bound on top). **Cross before the marble reaches `t44`,** where the pillar topples; `trigger_fall` #14 lies below.
5. **Ice cavern collapse.** `alice_fall_trigger` #5 (a thin plane at z about -2482), or the marble reaching `crash2`, runs `Garden3_IceFloor_Break1` (`marble_ice1`-`4`), then `Break2`: `icefloor2nd01`-`19` drop east to west over about 3.5 s. Kill plane #15 lies below. Run west through door #45.
6. **End platform.** `Garden3_Fall_End` #16: `end_piece1`-`8` drop and `end_fall1` (a clip brush) is removed after about 0.6 s. Alice falls about 400-500 units into exit #74.

**Exit:** `trigger_changelevel` #74 (x 416..744, y 1176..1448, z -3584..-3448), entered while falling, leads to `garden4$garden4_start1`.

**Enemies:** a stray Ladybug `lady_actor1` #31 (no script). The main hazard is the marble (dmg 999).

**Cinematic `Garden3_Start`:**
- **MVP:** the staged fleet and big-bug flight with the carried marble, the release, and the Alice puppet run.
- The commit, watched or skipped:
  - the marble at a fixed path time with speed 175, gravity 0.2, solid;
  - `FirstQuake` applied;
  - the fleet props gone;
  - the fog loop running;
  - `land_player` at a deterministic handoff between `alice_walk_pos1` and `alice_run_pos1`, facing down-canyon, with a fair lead on the marble.

**Movers:**
- the marble with its waypoint thread callbacks;
- the canyon gates (`func_door` toggles);
- the falling pillar with its bound shroom (angular acceleration topple, removed after 4 s);
- the ice floor (`marble_ice1`-`4`, `icefloor2nd01`-`19`; the pieces `icefloor-break2nd-34`/`-45` stay static);
- the end platform (`end_piece1`-`8` plus `end_fall1`; #79/#80 static).

**NPCs and dialogue:** none. The cast is prop ladybugs `lady_model1`-`9` and `big_bug1`.

**Special mechanics:** a rolling-marble chase with instant kill; waypoint-thread events; toggle gates; a collapsing ice floor over a kill plane; a final drop into the exit volume; the looping fog cycle (3 colours, 10 s fades).

**Rewards:** 3 Sanity pickups (all Easy-only); 4-5 essences per difficulty. No toy.

**Entity classes:** `info_player_start`, `func_fallingrock`, `info_waypoint` (thread and speed keys), `func_door`, `trigger_multiple`, `trigger_once`, `trigger_push`, `trigger_fall`, `trigger_changelevel`, `script_object`, `script_model`, `func_earthquake`, `func_camera`, `info_splinepath`, `info_pathnode`, `Enemies_Ladybug`, `garden_firefly`, items.

**PLAYABLE MVP checklist:**
- [x] W9 for `chase_marble1`, after private research (U1). Covers per-waypoint speed, thread callbacks mapped to the reviewed garden3 events, lethal contact, the script overrides (speed 175, gravity 0.2, solid), and a saved path time. **Research whether the marble collides with the gates** before choosing a policy.
- [x] W2 for #44/#45/#46/#49 (default lip included), with explicit rules: #41 toggles `t70`; #47/#48/#73 toggle `t71`. Latch each forward gate open once the marble has passed (a safety rule against missed toggles; document it).
- [x] Movers: the pillar topple and removal, the ice pieces (`Break1`/`Break2` tables), the end platform, and the static pieces.
- [x] Arrival scene commit (DG-4).
- [x] W20: the `Garden3_Loop_Farplane` fog cycle. The reader must follow `#include`, because `sky_sequence::script()` reads only `maps/<map>.scr` (`sky_sequence.rs:36-43`).
- [x] Saves: marble path time, speed and gravity; door positions and toggle parity; piece motions; scene clock. Migration restarts generic visits at the post-scene handoff with the chase re-armed.
- [x] Launcher `tools/launchers/Launch-Rolling-Stones.cmd`; `docs/GARDEN3.md`.

**Route check (`--garden3-route-check` and skip variant):**
- Wait for the scene; assert the marble handoff time, speed 175 and gravity 0.2.
- The chase helper moves Alice along `t21`..`t68`, keeping at least 200 units from the marble and more than its radius plus `PLAYER_HALF`. It crosses #41, #48, #47 and #73 in order, asserting the door states after each:
  - #44 closed;
  - #49 open, then closed, then open;
  - #46 closed, then open, then closed;
  - #45 open, then closed, then open.
- Use pads #22/#379; leave the pillar before `t44`.
- Drop through #5 (it costs about 70 under the current fall rule) or follow to `crash1`.
- Run west over the ice pieces, asserting the floor under Alice is still static; pass #45; walk into #16.
- Assert `transition == ("garden4", Some("garden4_start1"))` during the fall, zero marble contacts, and Sanity > 0.
- `--garden3-check` adds 30/60/144 Hz equivalence for the marble, doors and pieces, and save round trips mid-chase.

**Anode checks:**
- [ ] `--garden3-render-check`: the fleet flyover; the marble release; the marble behind Alice from the gameplay camera; each gate before and after its toggle; the pillar topple; the ice floor mid-collapse; the end platform falling.
- [ ] A real-input chase from the handoff to the cavern. The marble never clips through Alice, and the gates move at the right crossings.
- [ ] F5 mid-chase, then restart and F9: the marble, doors and collapsing pieces resume, not reset.
- [ ] The exit fall into garden4 loads its arrival; `--load auto` in a fresh process works.
- [ ] `tools/test_visibility.ps1` (the garden3 scene fixture), `tools/test_render_fx.ps1`, `tools/test_sky_performance.ps1` (the fog cycle).

**Fidelity follow-ups:**
- exact bounce arcs and the marble spin;
- the bounce quake and sound cadence;
- fleet flight speeds (0.6/0.7) and clip timing;
- the pillar visual;
- ice crack sounds;
- the stray Ladybug's behaviour.

**Risks:**
- `func_fallingrock` semantics (spawnflags 6, bounce, speed override).
- The marble reaches 290-320 u/s near the end, against Alice's run of 320, so the margin is thin.
- Toggle parity (the `trigger_multiple` waits are 15 s and 2 s).
- The cavern drop costs about 70 Sanity.
- A gate closing ahead of the marble could stop it.
- The original intro is unskippable; a skip must not place the marble unfairly close.

**Refs:** `maps/garden3.scr` (5-14, 24-37); `maps/cinematics/garden3_cinematics.scr` (1-47, 49-67, 92-159, 161-180 unused skip path, 192-236, 238-279, 281-398); entities #5, #14-#16, #22, #29, #31, #41-#49, #73, #74, #79, #80, #89, #90, #354, #379, and the waypoint chain `t124`..`crash3`.

<a id="v16-garden4"></a>
## 16 · garden4 (Icy Reception)

| Field | Value |
| --- | --- |
| Visit | `garden4$garden4_start1` (#79 at 848 1264 -3968, yaw 225) · ROUTE 15 · next `centipede1$centipede1_start1` |
| Status | **BLOCKED and BYPASS.** The break-floor footprints are holes down to about -5824, below the kill volumes #45-#47. The wall is missing (so the cave is open). `portal_trigger` is live at its editor pose |
| Size / milestone | L · M2 |
| Depends on | W1, W9 (7 rocks, including harmless `setsize 0` rocks), W4 and T13 (**difficulty-flagged triggers**), W20 (the side effects of `Fade_Fog`), W22b (**a bound, moving, gated portal exit**), E5 (residents), C1, C3 (caterpillar rig), R1 (vent rides) |
| Reservations | hit-ID 7,500,000 · `garden4/` · `garden4.` · `--garden4-*` · `garden4-*` |

**Today:**
- Only the fog fade of `Garden4_Fade_Fog` exists (`sky_sequence.rs:132-135`). `block_back`, `icebreathe`, the Ladybug `ai_on` and the 20 s quake loop are missing.
- No rock falls.
- `break_floor1`-`9`, `brokenwall01`-`08`, `block_back`, `portal_object` and the static pieces #52-#64 are neither drawn nor solid.
- The Caterpillar is idle.
- Triggers ignore difficulty, and the script-spawned Easy trigger is absent.
- The Ice Wand altar #42 is a working pickup (REWARDS slot 4, `campaign.rs:155`).
- Vents #341/#344 and lava damage work.

**Arrival:** no scene. main sets:
- `icebreathe(1)`;
- `ladybug01`-`03` on splines `t2215`/`t2200`/`t2209` with the rearm loop;
- the `world_quake1` loop every 20 s;
- posed frozen snarks and ants;
- the **Easy-only** `Ice_Fall3` trigger, spawned by script.

**Progression and gates:**
1. **Frozen path south-west.** The 13 static pieces #52-#64: #52-#62 are invisible nodraw collision ramps up to about 140 units above the rough ice; only #63/#64 are visible.
2. **Ice cavern boulders.**
   - `Ice_Fall1` (#251/#252, both named `ice_fall1_trigger1`, **not on Easy**) drops boulder 1 (dmg 50) to `t2244`, whose thread `Garden4_CrushIce1` drops `break_floor1`/`2`. The boulder is removed after 8 s.
   - `Ice_Fall2` (#253, not on Easy) drops boulder 2, running `CrushIce2` (floors 3-5).
   - Kill planes #45-#47 at z -5016.
3. **Ice Wand altar** #42 (-1418 -1066 -4448), inside `Ice_Fall3` #254 (Normal/Hard) or the spawned Easy trigger:
   - the Cheshire line `cat014` (played on the caterpillar actor);
   - boulders 3 and 4 (`setsize 0`, harmless), triggering `CrushIce3` (floors 8/7) and `CrushIce4` (floors 6/9);
   - `weapon_quake`.

   The target of #254, `boulder03`, does not exist (a no-op). **Reward: the Ice Wand.**
4. **Marble scene** (#110): `worldpos = 0` stops the quake loop. `ice_marble1` (dmg 200, speed 190, gravity 0.3) runs `marble_path1` (38 waypoints). The fake player stands at `alice_watch_marble1` for the cameras `garden4_path3`/`path3x` (about 7 s); floors 6-9 are removed; the fake player is warped to `alice_watch_marble2` (-1704 -288 -4520). **The marble reaches that point about 3-4 s after control returns**, so the handoff needs a defined safe lead. Unskippable in the original (DG-4).
5. **End boulders and wall smash.** `Garden4_Boulder_End` #37 (plane x about -2944): the harmless `boulder_end1` rolls `endboulderpath1` (16 nodes, 400-640 u/s) and overtakes Alice. At `t2264` its thread `Garden4_End_SmashWall` drops `brokenwall01`-`08`. `Garden4_Boulder_End2` #36: `boulder_end2` (dmg 20). **The wall blocks the only way north until smashed.**
6. **One-way fog trigger** `Garden4_Fade_Fog` #78 (plane y about 1632): `icebreathe` off; `block_back` shown and solid; the fog fade; `ladybug01`-`03` `ai_on`.
7. **Cave vents over lava.** Pushes #341 (to `t2239`) with accelerate #342, and #344 (to `t2240`) with #345, carry Alice over the chasm (kill #43 at about -6024; lava below).
8. **Caterpillar scene.** Trigger #49 runs `Garden4_StartCinema1`/`Garden4_Caterpillar_Cinema1`:
   - the fake player at `alice_pos1`;
   - lines `cpl001`-`006` with `alcz2025`-`2028`;
   - cameras `garden4_path1`/`2`/`4`, `newp1`-`3`, `portalw1`;
   - `portal_smoke` with drugview;
   - `portal_object` shown, following `portal_path` (6 nodes, speed 0.5; rotateY 48);
   - `portal_trigger` made triggerable.

   Skip: `Skipthread_G4`/`G4_CatEnd`, where the trigger is enabled 4 s after control returns.

**Exit:** `portal_trigger` #6, a `trigger_changelevel` bound to `portal_object` #8 (initial bounds x -1824..-1668, y 5492..5504), leads to `centipede1$centipede1_start1`. It is **disabled until the Caterpillar scene commits**. `$g4_changelevel` in `Garden4_Pre_Init` names no entity.

**Enemies:** Ladybug residents `ladybug01`-`03` on 31-node patrol loops north of the wall. The scripts never switch their AI off, so do not assume they are passive before the fog trigger. Rock hazards: boulders 1/2 (dmg 50), `boulder_end2` (20), `ice_marble1` (200); boulders 3/4 and `boulder_end1` are harmless.

**Cinematics:**
- `Garden4_Marble` (`garden4_path3`/`path3x`; `alice_watch1` #50, `dialog_watch1` #339). The commit: quake loop stopped, floors 6-9 removed, the marble at a fixed path time, Alice landed at `alice_watch_marble2` yaw 90 **with a defined safe lead from the marble**.
- The Caterpillar scenes: puppet acting (`idle_smoke`, `talk01`-`07`, `portal_smoke`, `idle_adjust`), 10 beats, portal reveal. The commit: portal at the end of `portal_path`, the trigger enabled, drugview off, Alice at `alice_pos1` yaw 90.
- The `Ice_Fall3` Cat line (`cat014`) as a story beat.

**Movers:**
- the ice boulders and crush floors (`break_floor1` has 3 brushes, `break_floor2` has 4);
- the wall smash;
- `block_back` #9;
- the portal plus the bound trigger (rebuild the collider each tick, following the `trigger_pose` pattern at `interaction.rs:985-992`);
- the static frozen-path pieces;
- the steam vents.

**NPCs and dialogue:** the Caterpillar (`caterpillar_actor1` #328, near the `cater_couch` prop #389; `maxmouthangle 45`): `cpl001`-`006`. Alice: `alcz2025`-`2028`. Cheshire: `cat014` (on the caterpillar actor). Add `c_caterpillar` to `--facial-check`.

**Special mechanics:**
- falling boulders that crush floors into kill pits;
- a second marble rolling along the player's route;
- a harmless boulder that smashes the blocking wall;
- one-way `block_back`;
- vent updrafts over lava;
- the 20 s quake loop;
- `icebreathe`, drugview;
- the moving, rotating exit portal;
- difficulty-specific triggers (`Ice_Fall1`/`2` absent on Easy; `Ice_Fall3` via the map trigger on Normal/Hard or the spawned trigger on Easy).

**Rewards:** **the Ice Wand** (#42, slot 4; auto-selected, `inventory.rs:449-455`); a Jackbomb duplicate #5; 4 essences per difficulty.

**Entity classes:** `info_player_start`, `func_fallingrock`, `info_waypoint` (thread keys), `script_object`, `trigger_changelevel` (bound and gated), `trigger_once` (plus the script-spawned Easy trigger), `trigger_push` plus `trigger_accelerate`, `trigger_fall`, lava, `info_splinepath`, `info_pathnode`, `func_earthquake`, `func_camera`, `script_model` (posed corpses, couch), `characters_caterpillar`, `Enemies_Ladybug`, `Characters_Walkrock-*`, steam emitters, items.

**PLAYABLE MVP checklist:**
- [ ] W9 for boulders 1-4, `boulder_end1`/`2` and `ice_marble1`. Thread callbacks `CrushIce1`-`4` and `End_SmashWall`.
- [ ] Movers: `break_floor1`-`9`, `brokenwall01`-`08` (solid until the smash), `block_back`, the static pieces (collision for all, drawing only for #63/#64), and the portal (hidden; `portal_path`; rotateY 48) with its bound, per-tick trigger.
- [ ] W4 and T13: trigger difficulty for the reviewed garden4 set, plus the Easy-only `Ice_Fall3` trigger at the altar from a reviewed table.
- [ ] Rules: `Ice_Fall1` (disables both volumes); `Ice_Fall2`; `Ice_Fall3` (the `cat014` beat, boulders at 0/1 s, the quake); `Boulder_End`/`End2`; `Fade_Fog` (extend the existing action with `block_back`, `icebreathe` off and the Ladybug activation; rerun `tools/test_sky_performance.ps1`); the quake loop until the marble scene.
- [ ] The marble scene commit, with a safe lead or dodge window.
- [ ] The Caterpillar scene plus the portal gate.
- [ ] Encounters: the residents through the generalized Ladybug table.
- [ ] Saves: boulder and marble path times, crushed floors, wall, `block_back`, portal and scene clocks. Migration: generic visits north of y about 1600 mark `SmashWall` and `Fade_Fog` done; otherwise restart at the entrance.
- [ ] Launcher `tools/launchers/Launch-Icy-Reception.cmd`; `docs/GARDEN4.md`.

**Route check (`--garden4-route-check` on Normal, plus an Easy variant with no boulders 1/2 and the spawned `Ice_Fall3`):**
1. Walk the frozen path.
2. Cross #251/#252 and #253 without standing under the boulders at impact; assert the floors crushed.
3. Take the Ice Wand: assert `copies(4) == 1`, `selected() == 4`, and `Ice_Fall3` fired once.
4. Cross #110 and wait; assert feet near `alice_watch_marble2`, floors 6-9 removed, the quake loop stopped. Dodge or outrun `ice_marble1` with zero contacts.
5. Navigate (-2720, -64), (-2720, -1000), (-2840, -1456) across #37 and #36; north via (-3700, -600) and (-3632, 16); east (-3000, 200); north (-2600, 1100).
6. Wait for `SmashWall`, cross #78 (assert `block_back` solid and the Ladybugs active), ride vents #341/#344.
7. Cross #49 and wait; assert the portal is visible and following its path, and the trigger enabled.
8. Walk into the moving portal.

Assert `transition == ("centipede1", Some("centipede1_start1"))`, the Ice Wand retained, and Sanity > 0. Run skip variants for both scenes.

**Anode checks:**
- [ ] `--garden4-render-check`: a boulder mid-fall and a crushed floor; the altar and HUD slot 4; marble scene shots; `boulder_end1` overtaking and the wall slabs falling; `block_back`; a vent ride over the chasm; Caterpillar two-shots; the portal on its path.
- [ ] Real input: take the Ice Wand and confirm auto-select and a Will-costed action. F5 during the boulder sequence, restart, F9: no replay.
- [ ] Skip the Caterpillar scene with a held Enter and walk into the moving portal; centipede1 loads with the Ice Wand; the autosave loads in a fresh process.
- [ ] F5 mid-Caterpillar line, restart, F9: same line, drugview state restored.
- [ ] `tools/test_visibility.ps1`, `tools/test_render_fx.ps1`, `tools/test_sky_performance.ps1`.

**Fidelity follow-ups:**
- drugview warp and `icebreathe` breath;
- 8 quakes;
- exact boulder and crush timing, ice sounds;
- frozen corpse poses (`snark07`/`08` unposed);
- the exact portal rotation;
- Caterpillar lip-sync and head-watch;
- dead code (`Cave_End_First`, the orphan `ROCKDIE`).

**Risks:**
- The dmg-200 marble handoff.
- Global trigger-difficulty changes affect other maps (DG-7).
- The thin rotating exit volume may be hard to enter; a swept volume is a documented deviation.
- Legacy saves already inside the cave.
- drugview and `icebreathe` must reset on exit.

**Refs:** `maps/garden4.scr` (5-8, 30-39, 46-67, 79-94); `maps/cinematics/garden4_cinematics.scr` (6-15, 49-60, 72-101, 103-156, 158-190, 192-286, 288-533); entities #5, #6, #8, #9, #36, #37, #42, #43, #45-#47, #49, #50, #52-#64, #78, #79, #110, #251-#254, #328, #339, #341-#345, #389, #393, #394.

---

# Centipede and Wonderland Woods, first visit (M3)

Shared M3 systems:
- timed script exits with persisted latches (W22b);
- `func_spawnchain` and live-capped spawn loops (W6);
- the Centipede boss (B1) and Larva (`spawnactor`, E8);
- the weak-spot hit window;
- essence respawners (T8);
- quest altars (T7);
- the card-guard, imp, magma, phantasm and chess-pawn families for wforest (E9 to E14, non-gating);
- Sanity-floor directives (T6);
- boss and arena recovery (T12).

<a id="v17-centipede1"></a>
## 17 · centipede1 (Fungiferous Flora)

| Field | Value |
| --- | --- |
| Visit | `centipede1$centipede1_start1` (1136 -1712 -440, yaw 90; no entry thread) · ROUTE 16 · next `centipede2$centipede2_start1` |
| Status | **BLOCKED: no exit entity.** The only exit is a script `map()` at the end of the ambush scene |
| Size / milestone | M · M3 |
| Depends on | W6 (`func_spawnchain`), W22b (timed exit and latch), W20 (worldspawn farplane, fidelity), T6 (`minhealth 10`), C1, C3, E2, E4, E5 (the t9/t18 shared loops), E6, E7, R1 |
| Reservations | hit-ID 7,600,000 · `centipede1/` · `centipede1.` · `--centipede1-*` · `centipede1-*` |

**Today:**
- Both thread triggers (#24 `Centipede1_Talk1` and #22 `Centipede1_Ambush_Cinema1`) are pending.
- No story beats; no controller; no encounters.
- The delayed `ants-01` group (#36, #201, #229) is **absent**: non-difficulty spawnflags are dropped (`npc.rs:79`).
- The other actors are inert statues. The Ladybugs are decorative.
- `centipede_actor1` is drawn as a statue at z -544, although the script hides it.
- The Dice altar #299 works (Dice copies go from 1 to 2, the milestone at `campaign.rs:156`).
- `shroom_lid` #34 is not drawn or solid.
- The three unnamed catmessages (#19-#21) work as summon hints.
- The worldspawn farplane is not applied (`environment.rs:26-32` reads only script literals).

**Arrival:**
- main starts the `ArmLadies` loop (rearm every 2 s).
- `c1_changelevel` is made nottriggerable, but no such entity exists (a no-op).
- After the player spawns: `lady1` to `t9`, `lady2` to `t48` (one shared 14-node loop), `lady3` to `t18`, `lady4` to `t26` (one shared 12-node loop).
- Four fire braziers (`trigger_hurt` #26, #279-#281; 48x48x96 at z -392, about 250-330 units from the start) ring the start without containing it.

**Progression and gates:**
1. **Arrive.** No gate.
2. **Traverse north** (about 6.5k units, y from -1700 to 4800): stationary Mushrooms and Bloodroses; Antlions (one underground); Snark #202 in the pool near (1440, 2464, -752). `trigger_once` #18 (a 1216x64x832 wall at -736 992) targets `ants-01`.
3. **Second Demon Die**, altar #299 (578 3294 -428). Optional in the data; strict mode asserts it.
4. **Voice-over.** #24 (a 384x16x384 wall at y 4120) runs `Centipede1_Talk1`: the hidden `centipede_actor1` plays `cpd001` as a voice only. Alice keeps control.
5. **Ambush.** #22 (a 704x16x232 wall at y 4792) runs `Centipede1_Ambush_Cinema1`:
   - `level_ai(0)`; `ant_runner1`-`3` run to `ant_brave_pos1`-`3`;
   - after 2 s, the fake player and a letterboxed `Start_Cinematic`. **The skip thread is set only now**;
   - a 0.5 s fade; the Centipede is shown at `cent_posx1` (-248 5696 -560); `alice_watch1` follows `cams/centipede1_path1`; `idle_yawn`, then `idle_snarl`, then `cpd002`;
   - fade; the Centipede is removed; `End_Cinematic_Fast`;
   - the runners get `ai_on`; the player gets **`minhealth 10`**;
   - two 3 s loops spawn `ant_attack` (`brave_spawn1` chain) and `ant_killer` (`brave_killer1` chain), both told to attack the player, while fewer than 3 `brave_ants` are alive. The killer loop reads the same counter, which is a data quirk.

   On the watched path, spawns come at about 0, 3.1 and 6.2 s. The skip path (`C1_End`) commits the same state and adds `ant_at_end1`/`2` (#394, #4), which get no attack order.
6. **Exit.** The script changes level to `centipede2$centipede2_start1` 10 s after the watched scene ends (`centipede1_cinematics.scr:166-167`), or 4 s after `C1_End` starts (:118-119). The Sanity floor makes the countdown unlosable.

**Exit:** script `map()` to `centipede2$centipede2_start1`. Choose **one** delay and one `ant_at_end` policy for both the watched and skipped paths, and document it (default in How to read).

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Army Ant (placed) | 9 | Runners #162/#38/#154 AI off until the ambush ends; #155, #198, #235 live; `ants-01` (#36/#201/#229) via #18; #228 Hard only | no |
| Army Ant (post-ambush spawns) | about 6 (at most 8) | `brave_spawn1` (to `t30`, `t31`, `t32`) and `brave_killer1` (to `t33`, `t34`) every about 3.1 s while fewer than 3 are alive (research the chain-node rule, U1) | no |
| Evil Mushroom | 12 | Stationary (#402 Hard only); 6 have unresolved support (#16, #35, #303, #304, #305, #406) | no |
| Ladybug | 4 | Shared loops; rearm every 2 s | no |
| Bloodrose | 2 | #39, #153 (unresolved support) | no |
| Antlion, surface and underground | 2 | #27, #265 (#395/#399 Hard only) | no |
| Snark | 1 | #202 (#396 Hard only) | no |
| Centipede (cinematic actor) | 1 | Hidden until the ambush, then removed | - |

**Cinematics:**
- `Centipede1_Talk1`: a story beat, voice only, with a subtitle; Alice keeps control.
- `Centipede1_Ambush_Cinema1` with `Skipthread_C1`/`C1_End`. **MVP:** the 2 s runner rush (not skippable), fades, the `c_centipede` puppet at `cent_posx1` on `centipede1_path1` (`idle_yawn`, then `idle_snarl`), the `cpd002` beat, removal. The commit: Centipede gone, runners active, the Sanity floor, spawners running, the exit timer latched.

**Movers and props:** `shroom_lid` #34 (static, solid, drawn); the brave spawners (7 `func_spawnchain`, 2 `func_spawn`); the braziers.

**NPCs and dialogue:** the Centipede (`cpd001`, `cpd002`). Cheshire hints: `catz215` (#21 at the start), `catz217` (#20), `catz218` (#19 over the ambush zone).

**Special mechanics:** a timed script exit; an AI pause; the Sanity floor during the countdown; spawn loops with live caps; Ladybug bombers; brazier hazards; underground Antlions.

**Rewards:** **the second Demon Die** #299 (Dice at 2); the Darkened Looking Glass #1; essences.

**Entity classes:** `trigger_once` (threads; target `ants-01`), `trigger_hurt`, `trigger_catmessage`, `func_spawn`, `func_spawnchain`, `func_camera`, `info_pathnode`, `info_splinepath`, `info_waypoint`, `script_object`, `Enemies_Centipede` (puppet), `Enemies_Army-Ant`, `Enemies_EvilMushroom`, `Enemies_Bloodrose`, `Enemies_Snark`, `Enemies_AntLion`/`-UnderGround`, `Enemies_Ladybug`, items.

**PLAYABLE MVP checklist:**
- [ ] Registry controller with state: scene beat and clock, `runners_active`, `floor_active`, exit timer and latch, brave counters.
- [ ] Bind #24 and #22; add reviewed beats for the `Talk1` and ambush dialogue functions.
- [ ] The ambush SceneSpec (Track `centipede1_path1`), with hold-Enter skip accepted after the rush.
- [ ] Emit the transition exactly once through a persisted latch (W22b).
- [ ] T6 `min_health` in `Stats::damage` (`inventory.rs:293-301`): set 10 at the end of the ambush, clear it on level change, save it. Shared with centipede2.
- [ ] Exclude `centipede_actor1` from generic placement; keep the runners inert until the ambush.
- [ ] Encounters (F5 opt-in): E2 including `ants-01`; E4, E5 (reviewed route table for the two shared loops), E6, E7; the spawn loops through W6.
- [ ] Render and collide `shroom_lid`.
- [ ] Saves: upgrade by rearming #22/#24 if they were consumed as "Pending". Cases: `centipede1-ambush-mid`, `centipede1-exit-countdown`.
- [ ] Launcher `tools/launchers/Launch-Fungiferous-Flora.cmd`; `docs/CENTIPEDE1.md`; CUTSCENE-AUDIT row 24.

**Route check (`--centipede1-route-check` and skip variant):**
- Planned waypoints are grounded floor points, because trigger centres are not valid navigation goals.
- Sequence: around the braziers; through the #18 wall near (-736, 992), asserting the `ants-01` activation event fires once; into the altar #299 (assert `copies(6) == 2` and `centipede1:299` collected); across #24 (assert the `cpd001` beat queued with no loss of control); across #22.
- Watched: wait for the cinematic, then survive. Assert Sanity never drops below 10 while the floor is active, and exactly one transition `("centipede2", Some("centipede2_start1"))` after the chosen delay (±1 tick).
- Skipped: the same assertions. Compare the snapshot and Stats with the watched run.
- Unit tests: the countdown at 30/60/144 Hz and a pause.

**Anode checks:**
- [ ] `--centipede1-render-check`: the pre-ambush forest with no visible Centipede; Ladybug bombing; the runner rush; the reveal with a subtitle; the post-scene ants.
- [ ] Walk to the Dice altar; the HUD shows two Dice; F5, kill, `--load quick`: two Dice and an empty altar.
- [ ] Enter the ambush and hold Enter at the letterbox. The automatic transition to centipede2 happens; `--load auto` shows the centipede2 intro with the Stats carried.
- [ ] Watched variant: F5 during the countdown, restart. The countdown, ants and floor resume, and the exit fires once.

**Fidelity follow-ups:**
- Ladybug parity;
- full AI for the Army Ant (`prj_bullet`), Mushroom (`prj_spore2`) and Bloodrose (`prj_thorn`), plus Antlion burrowing and Snark swimming;
- support fixes for the 8 unresolved plants;
- the killer-loop quirk;
- Centipede puppet emitters;
- drugview;
- the worldspawn farplane.

**Risks:**
- The watched and skipped timings differ (10 s versus 4 s).
- The Sanity floor must not leak through saves or transitions.
- The delayed actors are absent today.
- The Ladybug controller is specific to potears1.
- Concurrent edits in `npc.rs` and `encounters.rs`.

**Refs:** `maps/centipede1.scr:5-57`; `maps/cinematics/centipede1_cinematics.scr` (15-17, 23-69, 78-84, 101-168); entities #1, #3, #4, #16, #18-#27, #31, #34-#39, #55, #86, #127, #130, #132, #153-#155, #162, #166, #168, #198, #201, #202, #228-#230, #235, #265, #279-#291, #295-#299, #303-#305, #394-#406.

<a id="v18-centipede2"></a>
## 18 · centipede2 (Centipede's Sanctum)

| Field | Value |
| --- | --- |
| Visit | `centipede2$centipede2_start1` (-7040 3776 4176; thread `Centipede2_Start`) · ROUTE 17 · next `wforest$wforest_start1` |
| Status | **BLOCKED and BYPASS.** `c2_changelevel` #43 is an always-live Exit (`interaction.rs:308-311`), reachable from the spawn through the open mouth and slide, although the data disables it permanently |
| Size / milestone | XL · M3 |
| Depends on | W1, W22b (`c2_changelevel` permanently off; the grow exit), B1, E8, T1 (fire immunity), T6 (`minhealth 30`), T8, T12, W15, C1, C2 (scale ramp, grab camera), C3, C4 (catmessage after the boss), R1, R2 (weak-spot aim) |
| Reservations | hit-ID 7,700,000 (the Centipede and larvae inside this range) · `centipede2/` · `centipede2.` · `--centipede2-*` · `centipede2-*` |

**Today:**
- None of the 23 `script_object`s (teeth, `no_back1`, climbers, spikes and tips) are drawn or solid.
- The start thread is ignored.
- `ant_trigger1` is held off (`traversal.rs:218-225`) and nothing enables it.
- The boss and the five intro ants are inert statues. `cat_actor1` is visible.
- `eat_shroom1` and the `#15` minhealth reset are pending.
- No boss, larva or projectile code. The Tea works as a plain pickup.

**Arrival (`Centipede2_Start`, `centipede2.scr:165-205`; `centipede2_cinematics.scr:174-318`):**
- World init: `no_back1` and the climbers hidden and non-solid; `ant_trigger1`, `c2_changelevel`, `eat_shroom1` and `catmessage` disabled; `get_me3` triggered; the ants' AI off. The Centipede's AI is left on.
- `bosslevel(1)` and `killdemons`.
- The scene:
  1. the teeth chatter (bottom ±48, top ±32 per 0.5 s) until `teethpos = 1`;
  2. a 3 s fade; `ant_guard1`/`3` play alerts;
  3. Alice walks to `alice_pos1`; `ant_guard4` aims from `ant_shoot_pos1`; `ant_guard2` walks to `ant_push_pos1` and shoves;
  4. `ant_trigger1` is enabled; Alice slides (`centipede2_jump1a`); the teeth open;
  5. 3 s later Alice is at `alice_slide_pos1` (-4800 3776 3624); `no_back1` becomes solid and visible;
  6. `End_Cinematic_Fast`, then **`minhealth 30`**.

  The skip (`C2_End`) commits the same state plus `full_stats`, which only the skip path does; resolve this (recommendation: neither path refills). `C2_End` does not kill the chatter thread, so the teeth can over-open (cosmetic).

**Progression and gates:**
1. **Arrival scene.**
2. **One-way slide.** Push `*31` #89 launches toward `t1`; `*28` #50 is a +X push. `trigger_once` #15 (`Centipede2_MinHealth_Reset`) clears the Sanity floor. `no_back1` blocks the way back; `trigger_fall` #16 lies below the arena.
3. **Boss** (below). The respawning medium essence cycles `get_me3`, `get_me1`, `get_me2`, reappearing 10 s after each pickup.
4. **Spike drop** (the killthread `Centipede2_DropSpike` runs only if Alice is alive; unskippable in the data):
   - `bosslevel(0)`; the Centipede non-solid;
   - `centipede2_path6` with Alice at `alice_pos3` (-3168 3904 2872);
   - spike threads at 0/0.4/0.9/1.5/1.9/2.2 s: `spike01tip` drops 1090 over 3 s, `spike01atip` 1050 over 2 s, `spike03tip` 1132 over 2.2 s; the upper spikes are removed; quakes;
   - `eat_shroom1` and `catmessage` enabled at about 2.2 s;
   - about 5.5 s later the tips are removed and the climbers become solid and visible.
5. **Climb to the mushroom.** The climbers brush #3 (584x376x440; stepping floors at about 2960/3016/3152) leads to the world mushroom cap (floor about 3268). The last step is a jump of about 80 units.
6. **Growth and exit.** `eat_shroom1` #34 runs `Centipede2_Grow_Alice` (unskippable in the data):
   - the Cat fades in; `Grow_Dialog1` (`catz219`, `alcz2030`); Alice plays `eat_mushroom`;
   - `Grow_Dialog2` (`catz220`) together with `Grow_Loop` (a scale ramp of +0.07 per 0.1 s, a quake, the grow sound);
   - 5 s later the script changes level to `wforest$wforest_start1`.

**Exits:**
- The script `map()` in `Centipede2_Grow_Alice` via `eat_shroom1` #34, leading to `wforest$wforest_start1`.
- `c2_changelevel` #43 (-2684 3904 2880) is **permanently off**; gate it with an unsatisfiable condition.

**Enemies:** the boss; larvae (2 per spit-buggies attack, accumulating; 7 hp; acid-immune; jump, attach to Alice's back, suck 2); five Army Ants (`ant_guard1`-`4`, `ant2`) as inert props.

**Boss B1: Centipede** (`centipede_actor1`, `c_centipede.tik`, `ai/c_centipede.st`). Health 999999, pain threshold 1, immune to fire and firesword, bbox 256x256x452.
- **Vulnerability:** only the `tag_target` hit box (±64, ±64, -16..64) takes damage, and only from `attack_crush` entry to frame 25 (setbboxdamage events). Research whether radius damage bypasses per-box multipliers (U6).
- **Stages:** each needs two pains; the first sets a per-stage flag and the second advances. The sixth pain runs `death_start` (quake at frame 18), three `death_thrash`, then `suicide`, then the killthread.
- **Stage 1:** walk with 0.5 contact pulses (knockback 200); from APPROACH, a 20% readycrush roll outside 400. Outside 400: 50% spit buggies (2 larvae), otherwise spit (2 x `prj_centipede_spit`, 550 u/s, 15 acid). Inside 400 with sight: readycrush (glow windup), then `attack_crush` (quake at frame 25, melee 25 at frame 30).
- **Stage 2:** juggernaut 20% (`GetCloseToEnemy` charge, 25 on exit). Readycrush 75% inside 400, always at 300 or closer.
- **Stage 3:** spit beyond 700 (70%); juggernaut beyond 400 (35 damage). Inside 400 a sub-stage cycle: grab strike (`GrabCam`, a camera at +80 +280 +200 watching Alice), shake (drag `tag_mandibles` to `tag_gut`, `letgo`, toss 500/50/250), `ReturnCam`, then readycrush/crush, then buggies.
- **Data quirk:** `ATTACK_READYCRUSH_STAGE2`/`3` route pain to `PAIN_STAGE1` (`st:347, 591`). Document it; it matters only if some damage bypasses the multipliers.

**Cinematics:**
- `Centipede2_Start`/`C2_End`. The commit: Alice at `alice_slide_pos1` through `land_player`, `no_back1` solid, `ant_trigger1` enabled, the teeth in one documented pose, the floor at 30.
- `GrabCam`/`ReturnCam` (in-fight camera).
- `Centipede2_DropSpike` (spikes, climbers, enables; a skip if allowed commits the same).
- `Centipede2_Grow_Alice`: the Cat fade, beats, `eat_mushroom`, scale ramp, quake, then the transition.
- `GTeaPickup`: the unbound `Cent2_Get_Tea` stays a no-op.

**Movers:** the teeth `teeth_top` #31 and `teeth_bottom` #32; `no_back1` #1; the slide pushes (`ant_trigger1` #21 to `t3`; #89; #50; #53); the spikes `spike01`-`06`, `spike01a` and their tips (duplicate names are Registry group aliases); the climbers #3; 13 `func_earthquake`; the respawning essence `get_me1`-`3`.

**NPCs and dialogue:** Cheshire `cat_actor1` (`catz219`, `catz220`); Alice `alcz2030`. The `catmessage` (`catz219`) stays disabled until after the boss (HintSpec).

**Special mechanics:** the intro push into the mouth; a one-way slide; the Sanity floor; the frame-window weak spot; acid projectiles and latching larvae; grab, drag and toss with a camera; quakes; post-boss climbers; the essence 3-cycle; `bosslevel`/`killdemons`; the growth scale ramp.

**Rewards:** no toy. Grasshopper Tea #17; respawning medium essences; 3 placed essences. Dice stay at 2.

**Entity classes:** `info_player_start` (thread), `script_object`, `trigger_push` (initially disabled; launch), `trigger_once`, `trigger_catmessage` (script-enabled), `trigger_fall`, `trigger_changelevel` (permanently off), `Enemies_Centipede`, `c_larva` (spawnactor), `prj_centipede_spit`, `Enemies_Army-Ant` (props), `Characters_CheshireCat`, `func_spawn`, `func_earthquake`, `func_camera`, `script_model`, `info_splinepath`/`info_pathnode`/`info_waypoint`/`info_notnull`, `script_skyorigin`, items.

**PLAYABLE MVP checklist:**
- [ ] Controller owning the teeth, `no_back1`, the climbers, spikes and tips.
- [ ] Initial facts: `no_back1` and the climbers hidden and non-solid; `eat_shroom1` and `catmessage` off; `get_me3` spawned; the Centipede and `cat_actor1` owned.
- [ ] **Gate `c2_changelevel` permanently**, with a contract: touching it before or after the boss never transitions.
- [ ] Arrival scene with an identical commit; resolve the skip-only `full_stats`.
- [ ] #15 clears the floor; verify the slide with production movement.
- [ ] B1 boss per the section above, with seeded deterministic chance rolls and a registry target for the weak spot. Add a `route.rs` target via the registry.
- [ ] E8 larvae owned and saved by the boss.
- [ ] `bosslevel` dismisses Dice demons at the fight and scene starts.
- [ ] DropSpike, climbers and the growth scene, then the exit.
- [ ] T8 essence 3-cycle.
- [ ] Saves: boss stage, flag, action and cue clocks, projectiles, larvae, spikes, climbers, growth beat, exit latch. Reject impossible states (a living boss after DropSpike). The migration keeps a completion reached through the bypass.
- [ ] Launcher `tools/launchers/Launch-Centipedes-Sanctum.cmd`; `docs/CENTIPEDE2.md`.

**Route check (`--centipede2-route-check` and skip variant):**
1. Watch the intro. Assert feet near `alice_slide_pos1`, `no_back1` solid, `ant_trigger1` enabled, the floor at 30.
2. Slide through `*31`/`*28` across #15 (assert the floor cleared) to the arena floor (about 2848).
3. Fight loop (the `duchess_check` pattern): hold 200-400 units from the boss, outside contact range but inside the crush radius. Aim with R2 at the weak-spot target during its exposed window. Collect essences.
4. Assert: pains increase only inside the window; stages go 1, 2, 3; death follows the sixth pain; Alice alive; larva counts logged.
5. After DropSpike, assert the spikes removed, the climbers solid, `eat_shroom1` enabled. Climb, jumping the last step, to the cap near (-2200, 3900, 3268). Wait out the growth.
6. Assert `transition == ("wforest", Some("wforest_start1"))`.

`--centipede2-check` contracts:
- `c2_changelevel` never transitions;
- `eat_shroom1` is inert before the boss dies;
- no damage lands outside the window;
- determinism at 30/60/144 Hz;
- a pause freezes everything;
- a restore at every stage.

**Anode checks:**
- [ ] `--centipede2-render-check`: intro tracks (`path1`/`2`/`3`/`jump1a`), the teeth closed and open, the slide landing, spit, larvae, the readycrush glow, crush, juggernaut, grab/shake with the grab camera, the death thrash, the spike drop (`path6`), the climbers, growth (`path4`/`5`/`7`).
- [ ] A real-input boss fight with the Blade and Cards on the weak spot. F5 in stage 2, kill, `--load quick`: the stage, flag, larvae, essence cycle and floor all restore. Finish the fight.
- [ ] After DropSpike: F5 and restart, the climbers stay solid. Climb, eat, then watch or skip the growth. The wforest autosave loads in a fresh process.
- [ ] Negative: walk over the `c2_changelevel` area before and after the boss; no transition.

**Fidelity follow-ups:** the teeth chatter sound; ant alert, push and aim clips; weak-spot dlight; acid emitters; exact grab camera and toss; a solid boss body; exact quakes; the spike easing; growth to scale 8 over 10 s; the Cat fade sound; native `bosslevel`.

**Risks:**
- The weak spot depends on per-frame `tag_target` sampling (clip frame times from SKA).
- Seeded determinism for CHANCE transitions.
- Slide physics on steep slopes.
- The climbers must fit the 18-unit step and 100-unit jump.
- Scenes that are unskippable in the data (DG-4).
- Actor settling (the Centipede sits 72 units below its origin).

**Refs:** `maps/centipede2.scr` (6-28, 31-47, 50-163, 165-205); `maps/cinematics/centipede2_cinematics.scr` (5-13, 61-170, 174-213, 231-318); `ai/c_centipede.st` (1-783); `models/c_centipede.tik` (17-68, 317-451); `ai/c_larva.st`; entities #1-#9, #10-#35, #40, #42-#54, #82, #89, #90, #100, #217, #224-#237, #327-#335, #346, #347.

<a id="v19-wforest"></a>
## 19 · wforest (Caterpillar's Plot, first visit)

| Field | Value |
| --- | --- |
| Visit | `wforest$wforest_start1` (4528 460 296, yaw 45; thread `Setup_FirstPass`; entry None resolves to the same start) · ROUTE 18 · visit key `wforest$first` · next `wchess1$wchess1_start1` |
| Status | **BYPASS.** None of the 25 `script_object`s is drawn or collidable, nor is the smashable wall #82. The cavegate, chessgate and chesswall are therefore absent, and `trigger_changelevel` #126 to wchess1 can be reached without the staff or the Caterpillar. The Blunderbuss altar #73 can be collected on the first visit |
| Size / milestone | L · M3 |
| Depends on | W1 (setup table; hide keeps collision), W5 (relay `t97` imps), W8 (optional), W22b (#126 gated to the first visit), T7 (**the staff altar as a quest pickup with its pickup_thread**), E9-E14 (non-gating), C1 (4 scenes), C3, C4, R1 |
| Reservations | hit-ID 7,800,000 · `wforest/` · `wforest.` · `--wforest-*` · `wforest-*` |

**Today:**
- The Woods sky and the farplane literal work (`sky.rs:142`, `environment.rs:26-32`). First and return visits have separate save keys.
- The staff altar (`ambient_eyestaff_staff` #96) is excluded from decorations (`decorations.rs:183-185`) and has no handler.
- The Cats, the Caterpillar, Humpty and `blunder_cat` are drawn as statues.
- Encounters are off: all 11 Boojums and every delayed actor are absent; guards, pawns, imps and Fire Snarks are inert.
- Every thread trigger is pending.

**Arrival (`Setup_FirstPass`, `wforest.scr:5-49`):**
- Humpty hidden and non-solid;
- `fake_humpty_wall1` shown; `secretdoorbutton` hidden and non-solid;
- disabled: `secretdoor_trigger`, `wall_hide_trigger` (`wall_show_trigger` stays live), `cat_chess_trigger`, `cat_message1`, `wf_cinematrigger1`, `alice_destory_wall_trigger`;
- `eyestaff_trigger` enabled;
- `eyestaff_wall` hidden (**still solid**);
- the Caterpillar and `broken_wall1` hidden;
- `blunder_cat` hidden;
- chesswall portal open, non-solid and hidden;
- `block_trigger` (a clip brush over the button) solid; `wall_clip` in place.

References to `t139`, `lastpass_trigger` and `humpty_trigger1` are no-ops. Then the `Cat1_Start` scene (`cat_actor2` at `cat_pos_start_first`, `catz222`), then `full_stats`.

**Progression and gates:**
1. **Arrival.**
2. **Staff altar.**
   - `eyestaff_trigger` #81 (`Give_Eyestaff`) enables both `wf_cinematrigger1` volumes.
   - Touching `first_eyestaff` #96 (a staff-part pickup, box ±10 by 0..75) runs `Alice_Gets_Eyestaff`: the fake player at `alice_eyestaff_pos1`, which coincides with the altar origin, so land clear of it; `cat_actor3` at `cat_eyestaff_pos1` (`catz221`); cameras `wforest_gatecam1x` then `gatecam1`.
   - **The cavegate (#26-#28) lowers 152 over 4 s.** The skip `ES_End` removes it.
   - The altar sits on a pillar (floor about 216) above a -224 pit.
3. **Cave and western woods.** Through the cavegate (about 2774 1300 0). Group triggers: `tunnelgroup` #29, `pathtocave` #36, `boojumpatrol` #52, `2ndround` #720 and `boojumparty` #60 (both `trigger_multiple`). `trigger_once` #104 fires relay `t97`, which spawns imps: `t94` (#410) at 1.5 s, `t96` (#409) at 2 s, `t95` (#107) at 2.25 s. They leap on monster-only pads (spawnflags 20). Dead receivers: #115 (target `t31`) and #701 (`pathtocave02`). Pits include #118.
4. **Caterpillar.** Either `wf_cinematrigger1` (#119 or #716; the scene removes both, so it is one-shot across both) runs `WForest_Cinema1`:
   - the fake player at `alice_pos1` (176 56 400);
   - cameras `wforest_path1`, `path2`, `walice1`, `path3`;
   - lines `cpl007`, `cpl008`, `alcz2031`, `cpl009`, `alcz2032`;
   - `cat_message1` enabled (watched path only);
   - **chessgate #75 rises 192 over 4.2 s** (0.1 s on the skip `WF1_End`);
   - `cat_chess_trigger` enabled.

   The Caterpillar actor stays hidden throughout (data ambiguity).
5. **Chess approach and Cat.** Triggers `intropawns` #55, `chessboojum` #45, `checkerpawnb` #88, `checkerpawnc` #700. `cat_chess_trigger` #63 (a wall at y 4480) runs `Cat_ChessTalk`: Alice at `alice_chess_pos` (3512 4432 128), the Cat at `cat_chess_pos`, camera `wforest_catp2`, lines `catz224`, `alcz2033`, `catz226`.
6. **Exit.** `trigger_changelevel` #126 (1528 4352 640; corridor floor about 592) behind the raised chessgate #75.

**Exits:**
- #126 to `wchess1$wchess1_start1`, gated on the first visit and the chessgate raised.
- #83 `Hedge_Maze_Entrance` must **not** fire on the first visit (it sits inside `eyestaff_wall`, with `wall_clip` present).

**Enemies (Normal; the families arrive through F5 and E9-E14):**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Boojum | 11 (delayed) | `pathtocave`, `boojumpatrol`, `chessboojum`, `boojumparty`, `2ndround` | no |
| Red chess pawn | 13 | 8 delayed (`intropawns`, `checkerpawnb`/`c`), 5 placed | no |
| Card guards: Diamond, Club | 5 + 2 | Delayed and placed | no |
| Card guards: Heart, Spade | 2 + 2 | `hedgeguards` (#487, #44), `intropawns` (#33), placed #490. `spawn_heart1`/`2` (#37, #699) are only triggered from keep scripts, so they never fire here | no |
| Fire Imp | 7 plus 3 relay | 4 placed; `2ndround` 3 delayed; relay `t97` | no |
| Fire Snark / Phantasm / Magma | 3 / 2 / 2 | Placed / delayed (`tunnelgroup`, `2ndround`) / delayed (#74, #18) | no |

**Cinematics:**
- `Cat1_Start` (`wforest_catp1`).
- `Alice_Gets_Eyestaff` with `ES_End`. The commit: the cavegate passable, Alice clear of the altar.
- `WForest_Cinema1` with `WF1_End`. The commit: gate raised 192, `cat_chess_trigger` enabled, Alice at `alice_pos1`. Decide `cat_message1` (only the watched path enables it; default in How to read).
- `Cat_ChessTalk` with `C1Chess_End`. The commit: Alice at `alice_chess_pos`, `cat_actor2` removed.

**Movers:** the cavegate (portcullis with a grille of clip and monsterclip); the chessgate; the chesswall (4 brushes); the Humpty secret area (sealed on the first pass); the `eyestaff_wall` visibility toggles; the delayed imp spawns.

**NPCs and dialogue:**
- Cheshire `cat_actor2` (`catz222`, `catz224`, `catz226`) and `cat_actor3` (`catz221`);
- the Caterpillar (`cpl007`-`009`, hidden);
- Alice `alcz2031`-`2033`;
- Humpty hidden;
- the three catmessages carry no target line.

**Special mechanics:** a per-entry setup table (hidden brushes stay solid); an altar pickup starting a scene; a scripted gate lift; a chess wall portal; paired visibility triggers; delayed relay imps; `trigger_multiple` groups; 5 pits; the sky portal.

**Rewards:** the Eye Staff **staff part** (a quest flag, not a weapon; `docs/LOADOUTS.md:52`); Looking Glass #11; health, Will and essences. The Blunderbuss must be **excluded on the first visit** (`pickups_for_visit`, `inventory.rs:495-523`).

**Entity classes:** `info_player_start` (thread), `ambient_eyestaff_staff`, `trigger_once`, `trigger_multiple`, `trigger_relay`, `trigger_changelevel` (gated), `trigger_catmessage`, `trigger_fall`, `trigger_push` (monster-only), `func_spawn`, `func_smashablewall` (hidden but solid), `script_object`, `script_model` (sky), `func_camera`, `info_pathnode`, `Characters_CheshireCat`, `characters_caterpillar`, `Characters_HumptyDumpty`, `Characters_chess_red_pawn`, `Enemies_Boojum`, the card guards (Club/Diamond/Heart/Spade), `Enemies_FireImp`, `Enemies_FireSnark`, `Enemies_Phantasmagoria`, `enemies_magmaman`, `Item_WeaponPickup_Blunderbuss` (excluded on the first visit), `sound_speaker`.

**PLAYABLE MVP checklist:**
- [ ] One `src/levels/wforest.rs` controller for both visits (`returning = entry == Some("wforest_start2")`). It owns the cavegate x3, chesswall x4, chessgate, secret door and button, fake Humpty wall, broken walls, `block_trigger`, `wall_clip`, hedge doors, debris and `eyestaff_wall`. `skycamera01` is excluded.
- [ ] The `Setup_FirstPass` table, with visibility and solidity modelled separately.
- [ ] The `Cat1_Start` scene.
- [ ] Staff altar: a controller-owned contact pickup with light and smoke art, recording a `staff_part` quest flag (T7) and running the scene. `eyestaff_trigger` enables both scene volumes.
- [ ] `WForest_Cinema1` (one-shot across both volumes) and `Cat_ChessTalk`.
- [ ] Gate #126: first visit and chessgate raised, plus the collider.
- [ ] Hide the Cats, the Caterpillar, Humpty and `blunder_cat`; exclude Blunderbuss #73 on the first visit; keep the secret door closed.
- [ ] Encounters: Boojum, Club and Diamond now; the relay `t97` spawns; no wiring for the dead receivers. The other families come as they land (DG-3).
- [ ] Beats: `Cat1_Dialog`, `Alice_Gets_Eyestaff`, `WForest_Cinema1`, `Cat_ChessTalk`.
- [ ] Saves under `wforest$first`: rearm #81/#119/#716/#63/#62 if they were consumed as "Pending"; move obstructed old positions to the entrance; document that an early Blunderbuss already collected is kept.
- [ ] Launcher `tools/launchers/Launch-Caterpillars-Plot.cmd`; `docs/WFOREST.md`.

**Route check (`--wforest-route-check` and skip variant):**
1. Watch `Cat1_Start`.
2. Reach the staff pillar by a reviewed path (derive a grounded approach from the geometry) and walk into the altar. Assert the cavegate is open, the scene volumes enabled, and Alice clear of the altar.
3. West through the cave (about 2774 1300 0) to `alice_pos1`. Assert the chessgate raised and `cat_chess_trigger` enabled.
4. To `alice_chess_pos`, then north across #63 for the chess Cat scene.
5. The corridor floor (about 1528 4352 592).
6. Assert `transition == ("wchess1", Some("wchess1_start1"))`.

Contracts:
- before the staff, navigation through the cavegate fails and #126 cannot be reached;
- the first-visit pickups lack the Blunderbuss;
- #83 does nothing;
- 2 Dice and no Eye Staff weapon.

**Anode checks:**
- [ ] `--wforest-render-check` (first-visit fixtures): the Cat intro; altar light and smoke; the Cat at `cat_eyestaff_pos1`; the portcullis lowering; the Caterpillar-scene cameras; the chessgate lift; the chess Cat; the sealed secret wall with Humpty hidden.
- [ ] Real input: skip the intro, take the staff; F5 during the portcullis motion, restart, the gate state matches. Watch the Caterpillar scene, F5/F9, skip the chess Cat, cross into wchess1. `--load auto`: wchess1 with 2 Dice and no Blunderbuss.
- [ ] Negative probes: the closed cavegate and chessgate hold before their events; the Blunderbuss room is sealed.

**Fidelity follow-ups:** Caterpillar staging (confirm privately whether it is visible); portcullis and gate sounds; Cat animation sequences; area-portal culling; fog density; the AI of the non-Boojum families.

**Risks:**
- Hidden brushes that stay solid (`eyestaff_wall`, the fake wall).
- Duplicate scene volumes.
- About 49 placed enemies.
- The staff pillar approach.
- Old saves that walked through the missing gates or took the Blunderbuss early.

**Refs:** `maps/wforest.scr` (5-49, 53-69, 71-89, 162-189); `maps/cinematics/wforest_cinematics.scr` (12-72, 218-381, 435-479, 502-594); entities #11, #12, #16-#23, #26-#29, #33, #35-#45, #51-#55, #60, #62-#64, #67-#75, #81, #83, #88, #96, #100-#123, #138, #144, #303, #315, #347, #351, #359, #367, #409-#416, #487-#505, #622, #623, #625, #632, #637, #654, #668, #682, #699-#731.

---

# Chess: Pale Realm, Castling, Checkmate in Red (M4)

Shared M4 systems:
- the chess family, with red enemies, white allies (whose TIKI declares `nodamage`, with scripts opting in through `takedamage`/`minhealth`), factions and idle self-kill (E14);
- chess-piece disguise mode (T5);
- reviewed puzzle tables (DG-17);
- generic levers (W7);
- moving liquid (W13);
- sliding `func_door` (W2);
- the door 4096 audit (W3);
- bound portal exits (W22b);
- the Red King boss (B2);
- five, two and three scenes (C1);
- chess-king lip-sync (C3).

<a id="v20-wchess1"></a>
## 20 · wchess1 (Pale Realm)

| Field | Value |
| --- | --- |
| Visit | `wchess1$wchess1_start1` (the only start, #134 at -2496 128 -128, no angle) · ROUTE 19 · next `wchess2$wchess2_start1` |
| Status | **BLOCKED.** No controller. `script_object`s are neither drawn nor solid (quad gate, water exit block, puzzle wedges and posts, crushers, spike wave, lift). `knight_gate` (`func_door` #966/#967) is drawn but can be walked through. The pool is dry, because the `water` brush entity is not a liquid. Chess func_spawns never spawn. `#103` is a live exit (the bypass is unconfirmed; the castle doors #920/#921 are the final chokepoint) |
| Size / milestone | XL · M4 |
| Depends on | E14 (**gating**), T5, DG-17, W1 (contents-correct playerclip/monsterclip), W2 (knight gate), W4 (the #109 shot trigger, killtarget), W5, W6 (re-triggered spawners, leap pads), W7 (levers #1185 and #1210), W8 (optional secret), W13 (**rising water +96**), C1 (5 scenes), C2 (scale tweens, `SFX_GnomeHide`), T12 (puzzle reset), R1 (`chess_step`, `use_at`, swim and climb-out, lift ride) |
| Reservations | hit-ID 7,900,000 · `wchess1/` · `wchess1.` · `--wchess1-*` · `wchess1-*` |

**Arrival:**
- main threads `cinema_wchess1_intro_thread` with `level_ai` off and a faded start.
- Init (`wchess1.scr:143-296`):
  - the instructors (`minhealth` 125/150) and `rook_guard1` (180) get AI off; `pawn_npc2` is hidden and nodamage;
  - `bishop_grow`/`knight_grow` hidden at scale 0.1;
  - `locked_doors` #121/#122 locked;
  - the quad gate parts are bound and parked at `way1` (closed);
  - `knight_mclip` off; `bishop_actor_clips` and `knight_actor_clips` removed (:213-214);
  - the puzzle triggers are bound to their `script_origin` parents;
  - the crushers `bsspike1`-`5`/`knspike1`-`5` armed (setdamage 1000); the `op1` spikes armed;
  - the waterwheel starts at `waterwheel_start`;
  - `red_knight2_trigger` and `cinema_knight_start_trigger` disabled;
  - 14 camera paths loaded; the elevator starts descending.
- Calls on absent entities are no-ops: `r_gate_pawns_trigger`, `r_rook1`, `rook_hurt_trigger`, `elevator_lever`, `elevator_weight*`.
- Loadout: Blade, Cards, Mallet, Jackbomb, Ice Wand, 2 Dice.

**Progression and gates:**
1. **Arrival gate flyby.** The bell; clock hands spin (90 and 7.5 deg/s); cameras `wchess1_gate_camera1`/`2`; the quad gate opens to `way2` for 1 s; `rook_guard1` walks to `rook_guard1_cinema_dest1b`; the gate closes. The skip `cwit_End` commits the gate closed and the rook at `dest1b`.
2. **Ambient pawn exit.** #1056 sends `pawn_npc1` to `pawn_npc1_dest1`, where it is removed.
3. **Bishop transformation.** #292 runs `cinema_bishop_start_thread`:
   - cameras `wchess1_bishop_camera1a`/`1b`/`1c`/`2`;
   - the instructor walks to `bishop_instructor1_dest1` (two nodes share this name, #293 and #1021: pick one);
   - Alice walks to `alice_bishop_puzzle_dest1`/`2`; the `bishop_effect` sparkle and vanish sound; Alice shrinks and `bishop_grow` grows;
   - disguise: Alice's surfaces nodraw, `bishop.tik` attached, weapon hidden, `chesspuzzle(1)` (no jump or attack states), `bosslevel(1)`;
   - `bishop_start_thread` runs Alice to square 1 (-1856 576) and arms only NW.

   Skip `cbst_End`.
4. **Bishop puzzle** (34 squares, `info_waypoint` `bishop1`-`34` at z -120 on a 128-unit diagonal lattice).
   - Controls re-parent to the current square: corner triggers `bstrig_nw`/`sw`/`se`/`ne` (40x40x256), wedge clips `bsclip_*` (playerclip), floor arrows `bsarrow_*`, and 28 posts plus `t98`-`t101`.
   - Squares 4, 6, 8, 9 and 29 are **crushers** (0.25 s down 96, 0.25 s up, 0.5 s rest, random start up to 0.5 s, 1000 damage).
   - Squares 11, 17, 20, 21 and 27 are **pits**: Alice reverts and falls through the monsterclip floors into `trigger_fall` #130, which kills.
   - Goal: square 34 (-832 1600), which reverts the disguise and removes the controls.
   - **Data defect:** after the first move, every wedge is re-solidified, and all 139 per-direction notsolid calls are commented out. Each corner trigger lies inside its playerclip wedge, so a literal port is unplayable. The controller must open the armed corners (a reviewed deviation, documented; U4).
   - Shortest safe solution (20 moves): NW, NE, NE, NW, NW, NW, NE, NE, SE, NE, SE, SE, SW, SE, SE, NE, NE, NW, NE, NE. Squares: 1-2-3-5-10-14-19-24-30-25-31-26-22-16-12-7-13-18-23-28-34.
5. **Upper galleries.**
   - #108 starts the `op1` spike wave: pop-up spikes `op1_spike1`-`8` in six diagonal rows, 1000 damage. `pawn_npc2` is shown.
   - #641 spawns 2 `red_bishop1` beyond the corridor. When both die, the spikes are removed and `pawn_npc2` walks off, then turns into an ally.
   - `red_pawns1_trigger` #663 (leap pads #51/#1376), `t138` #23, `red_knight1_thread` #102 (it attacks `pawn_npc2`), `red_pawns2_thread` #938, the Cards altar #70.
   - Trigger #47 overlaps the **Mallet altar #71**: collecting it relays `t129`, which leaps knight #1385 in after 1 s (Normal/Hard) and knight #46 after 0.5 s (Hard). Pads #1386/#45 also push Alice.
   - Elevator: an eased 192-unit lift. Bottom trigger #98 sends it up after 1 s; at the top it waits 1 s, then arms #968, and returns about 3 s after arriving unless sent.
   - Confirm the exact order with `navigate` (U9).
6. **Bell lever.** Before the bell, `nono_trig` #24 makes the rook twitch a refusal. Lever #1185 (move_thread `cinema_ring_bell_thread`, count 1) runs:
   - a 5 s free delay (skip not yet registered);
   - the bell;
   - `wchess1_quad_camera1` on `quad_camera2`;
   - the **gate opens** to `way2` (x = 1728);
   - the rook goes to `rook_guard1_dest1`, then `dest2`;
   - `red_knight2_trigger` #662 armed (red knight #635 attacks); `nono_trig` disabled.

   Skip `crbt_End`.
7. **Knight-gate kill count.**
   - `rook_guard1_attacked_thread` #940: the rook gets takedamage; `red_bishop2` spawns, walks, then duels the rook (the rook's `minhealth` is 180).
   - `red_knight3_thread` #952; `red_pawns3_thread` #953 (2 pawns).
   - Every counted piece self-kills after 10 s inactive (`max_inactive_time` plus an idle hurt of 1000; U6).
   - The optional bully scene is triggered by #109 (shootable, health 1) or #1198 (touch): 4 `red_pawn_bully` spawn 2 s apart (counted), `red_knight_bully` (not counted) attacks, and `pawn_victim` flees.
   - **Threshold:** 4 kills, or 8 if the gang was activated before the fourth kill. Then `knight_gate` opens and `cinema_knight_start_trigger` #30 is armed. **Latch both**, so a late gang cannot re-open or re-arm them.
8. **Knight transformation.** #30 (`trigger_multiple`, wait 1) runs `cinema_knight_start_thread`: cameras `wchess1_knight_camera1`/`2`; the instructor walks; `knight_effect`; the grow tween; the `knight.tik` disguise with `knight_mclip`; Alice at square 9 (3264 1856), armed W. Skip `ckst_End`.
9. **Knight puzzle** (31 squares over the pool).
   - Edge triggers `kntrig_n`/`s`/`e`/`w` (128x32 strips); `knclip` walls (there are 111 active per-direction opens, unlike the bishop file); arrows.
   - Moves are one or two scripted legs.
   - Crushers on squares 10, 12, 17, 19, 21. Drop squares 5 and 18 drop Alice into the pool, and the start trigger is re-armed (an authored retry).
   - **Data defects:**
     - `knight_11_19`, `knight_11_4` and `knight_26_22` are referenced but undefined, so their presses do nothing. `knight_2_11` opens walls toward undefined targets, so a disguised Alice could walk off the grid; block that.
     - `knight_11_13` runs east onto crusher 12 and moves the controls to square 10. Choose a reviewed fix or a faithful crusher death (default in How to read).
     - `knight_6_4`, `knight_22_13` and `knight_20_22` stop on the hazard square they cross (plausibly intentional).
   - Shortest safe solution (14 moves): W, W, S, W, W, N, N, W, E, N, E, E, E, E. Squares: 9-14-13-3-2-(1)-4-11-16-15-16-24-25-(26)-22-(23)-28-(30)-31.
   - Goal square 31 (3264 2624) reverts the disguise. **Never re-arm the start after success.**
10. **North and east loop.** #77 (bishop #76, pawn #75, and #1368 on Hard), #35 (`t135` pawns), #73 (`t107` knight), `red_pawns4_thread` #959, the Jackbomb altar #69. The optional float secret: shoot #27 (health 10), which opens the glass #28; floaters #118-#120; `trigger_fall` #91; Health_Large #95.
11. **Water lever.** #1210 (move_thread `cinema_raise_water_thread`) plays 4.7 s of staging (`wchess1_water_camera1`/`2`), then `raise_water_thread`:
    - the waterwheel spins up to 60 deg/s;
    - after 3 s, the `water` brush entity (*51, top z -384) rises 96 over 5 s (top -288);
    - then `water_exit_block` (*31, a clip) is removed.

    That is **about 12.7 s after the move_thread starts**. The skip `crwt_End` is registered only after the raise begins and does not shorten it.
12. **Exit.** Swim to the pool's west edge and climb out at the passage (floor about -256). #1369 spawns `red_pawn5a`/`5b`; #1370 calls the undefined `last_enemy_attack_thread` (a no-op). Go through the castle doors #920/#921 to #103.

**Exit:** `trigger_changelevel` #103 (x 1152..1408, y 2880..2944) leads to `wchess2$wchess2_start1`. It is authored as always triggerable. Access requires the raise (water plus the removed block) and the castle doors. `navigate` must prove there is no tower-ledge bypass.

**Enemies (Normal):**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Red pawn `red_pawn3` | 2 | #953; counted; idle self-kill | **yes** |
| Red bishop `red_bishop2` | 1 | #940; the rook duel; counted | **yes** |
| Red knight `red_knight3` | 1 | #952; counted | **yes** |
| Red pawn bully gang (one spawner fired 4 times) | 4 | #109/#1198; counted; raises the threshold to 8 only if activated before the fourth kill | **yes** (conditional) |
| Red bishop `red_bishop1` | 2 | #641; both dead removes the spike wave | only if the corridor is critical |
| Red pawns (`red_pawn1`, `red_pawn2`, `red_pawn4`, `red_pawn5` pairs; target-linked; placed #20) | 14 | Various triggers | no |
| Red bishop `t106` | 1 | #77 | no |
| Red knights (`red_knight1`, `red_knight2`, the bully, `t107`, `t131`, placed #49) | 6 | Various | no |
| Hard-only extras | 9 | spawnflags 768 | no |
| `red_rook2` | 0 | Never: no entity references `red_rook2_thread` | - |

Allies: `bishop_instructor1` (`minhealth` 125), `knight_instructor1` (150), `rook_guard1` (180, takedamage after #940), `pawn_npc1`, `pawn_npc2` (a 10 hp escort), `pawn_victim`, and static rook #129.

**Cinematics** (all have hold-Enter skips that commit the matching `*_End` state; saved clocks; no replay):
- the arrival flyby (`cwit_End`);
- the bishop transformation (`cbst_End`);
- the bell and quad gate (`crbt_End`, after the 5 s free delay);
- the knight transformation (`ckst_End`);
- the water raise (`crwt_End`, identical outcome).

The red-rook wall-break scene (`cinema_red_rook1_attack_thread`, :1349-1447), `red_rook2_thread` and `r_gate_pawns_thread` are **dead data**; do not implement them.

**Puzzles and movers:**
- the bishop board and knight board (above);
- crushers;
- the quad gate (parked at `way1`; 1 s slides);
- the knight gate (script-only `func_door`, lip 32);
- the lift and counterweight (rider carry, call and auto-return);
- the `op1` spike wave;
- the rising pool, exit block and waterwheel;
- the clock hands and bell;
- spawn leap pads (spawnflags 20 are monster-only; the unflagged #39, #45, #57, #58, #83, #1386, #1388 also push Alice);
- the float secret.

**NPCs and dialogue:** no voiced lines. SFX: the bell, the gnome vanish sound, spike, lift and waterwheel loops. Tolerate the absent names: `knight_guard1`/`2`, `knight_instructor2`, `r_rook1`, `rook_hurt_trigger`, `rook_wall_area`, `elevator_lever`, `elevator_weight*`, `elevator_spray`, `r_gate_pawns_trigger`, `spawn_r_gate_pawns`, `bishop_start_trigger`.

**Special mechanics:**
- chess disguise (no jump or attack, a TAN piece, scripted square runs);
- corner and edge move volumes;
- crusher and pit squares;
- the kill-count gate with idle self-kill;
- the spike wave removed by kills;
- the counterweighted lift;
- mover-owned rising liquid;
- monster leap pads;
- NPC duels;
- a pickup-triggered ambush;
- shootable triggers and killtarget;
- difficulty spawns.

**Rewards:** no new toy. Refill altars: Jackbomb #69, Cards #70, Mallet #71. Secret Health_Large #95.

**Entity classes:** `trigger_once` (thread, target, killtarget, delay, shootable), `trigger_multiple`, `trigger_changelevel`, `trigger_relay`, `trigger_push` (monster-only leaps), `trigger_fall`, `func_spawn` (re-trigger instances), `func_rotatingdoor` (script lock), `func_door` (knight gate), `func_smashablewall`, `script_object` (movers, clips by contents, liquid, damaging posts), `script_origin`, `script_model`, `info_waypoint`/`info_pathnode`/`info_notnull`, `func_camera`, `Objects_Lever`, `SFX_GnomeHide`, `sound_speaker`, `Characters_chess_*` (white and red), items.

**PLAYABLE MVP checklist:**
- [ ] Private research, recorded as paraphrase in `docs/WCHESS1.md` (U4, U6): the `chesspuzzle`/`bosslevel`/`killdemons` player effects; the idle-thread rule; ScriptSlave solid plus bound-trigger touch rules (this settles the bishop wedge question); the HEALTH condition direction; door flags 4096/64 and `func_door` 160.
- [ ] E14 chess family (TIKI-loaded pawn 75/knight 150/bishop 125/rook 200, with melee, beam, block and charge), allies, scripted commands, idle self-kill, kill callbacks, staged duels with `minhealth` floors.
- [ ] NPC ownership: `owns_npc` excludes the controller-owned chess Characters, with snapshot regeneration.
- [ ] Movers and colliders: quad gate, knight gate, lift and counterweight, crushers and spike wave (deterministic saved phases), puzzle wedges, posts and clips (contents-correct), arrows, exit block, wheel and clock decor.
- [ ] W13 moving liquid: the pool starts at top -384 and rises 96 over 5 s; the offset is saved.
- [ ] T5 disguise mode (movement, weapons and viewer input; Route weapon blocking at `route.rs:379-440`), with scripted runs marked `scripted()` and the camera kept.
- [ ] Puzzle data per DG-17 (reviewed tables recommended): square numbers, the direction-to-destination-and-legs graph with the documented defects handled, the armed-corner rule for the bishop, and edge strips for the knight.
- [ ] Death and recovery (T12): dying while disguised clears the disguise, resets the puzzle, re-arms its start (a reviewed deviation for the bishop's trigger_once), and recovers to the puzzle start position. The knight drop squares keep the authored re-arm.
- [ ] Trigger adapters for all listed threads; target-only triggers routed to spawners; #109 added to the shootable set.
- [ ] Levers #1185/#1210 (single use) through W7.
- [ ] Facts and counters: the `red_bishop1` deaths, knight-gate kills plus the gang flag (latched), bell rung, water raised, puzzle states, scene latches.
- [ ] The 5 scenes, with commits identical to their `*_End` states.
- [ ] Saves (scene id and clock; puzzle kind, square and arrival leg; disguise; mover phases; counters; levers; water offset; knight latch), plus an entrance restart for legacy saves. Cases: `wchess1-intro-mid`, `wchess1-bishop-square`, `wchess1-bishop-move`, `wchess1-knight-gate`, `wchess1-bell-delay`, `wchess1-water-rising`.
- [ ] Launcher `tools/launchers/Launch-Pale-Realm.cmd`; `docs/WCHESS1.md`.

**Route check (`--wchess1-route-check` and skip variant; Normal):**
1. Wait for or skip the intro; assert the gate closed and the rook at `dest1b`.
2. Bishop trigger at (-1856, 304); assert Bishop disguise at square 1, that jump leaves z unchanged, and that shots stay unchanged with a visible target.
3. `chess_step` through the 20 moves; assert the squares; unbishop.
4. Galleries: `op1` (-688, 256, 352); `red_bishop1` (-1232, 608, 320), fighting until both die (spikes removed); the lift if needed; `red_knight1` (-704, 1952, 320); `red_pawns2` (-384, 1632, 384).
5. The bell lever at (320, 1216, 416): assert the 5 s delay, the gate open, the rook at `dest2`, and #662 armed.
6. East through x = 1728 to #940, #952, #953; fight to 4 kills without touching #1198 or #109; assert the knight gate is open. A second run touches #1198 before the fourth kill and asserts the gate stays shut until 8.
7. Knight trigger (3392, 1760); the 14 moves; assert the squares and no re-arm.
8. The loop via #77, #35, #73, #959 to lever (2073, 1431, 256). Assert water top -288 and the exit block gone **about 12.7 s** after the move_thread starts, with identical timing when the scene is skipped.
9. Swim to about (2176, 2624, -290), climb out, walk the passage and the doors into #103.

Assert `transition == ("wchess2", Some("wchess2_start1"))`, teleports == 0, and alive. Waypoints come from `navigate` and are then frozen. `--wchess1-check` covers pits and crushers with reset, undefined knight arrows, `knight_11_13`, the exit blocked before the water, the gate closed before the bell, mid-move, mid-rise and mid-scene JSON round trips, and legacy migration.

**Anode checks:**
- [ ] `--wchess1-render-check`: the intro gate shot; the bishop disguise with the NW arrow; a crusher down; a pit fall start; the knight arrows; the knight gate open; the bell gate-open frame; the pool before and after with the block gone; each red piece's attack pose.
- [ ] `tools/test_visibility.ps1` fixtures: Alice hidden only while disguised, and visible after unbishop and unknight, after every scene, and after F9 of a disguised save.
- [ ] Real input:
  - hold Enter to skip the intro, watch the transformation, move NW and NE to square 3, F5, restart, `--load quick`: the disguise, square and arrows restore and Space does not jump;
  - E on the bell lever, F5 inside the 5 s delay, restart, F9: the scene plays once;
  - from a staged save on knight square 26 reached from 25: S does nothing (undefined target), then E, N, E to the goal, with F5 mid-leg;
  - after the raise: drop, swim, climb out, change level; wchess2 autosaves, and Continue in a fresh process restores it.

**Fidelity follow-ups:**
- camera interpolation and acting;
- the `SFX_GnomeHide` puff;
- the arrow show and hide per move;
- the random crusher phase;
- clock and lift mounts;
- instructor walks; the rook's refusal twitch; `pawn_victim`'s flee path;
- full faction AI;
- the water surface and wheel sounds;
- the float secret;
- the Hard-only doors.

**Risks:**
- The bishop data inconsistency (deviation required).
- DG-17 and rule 1 (tables versus a runtime reader).
- Knight data defects and the start re-arm.
- Idle self-kill semantics; stuck pieces could soft-lock the gate.
- Moving liquid is a new API.
- The gates' real value has not been proven by `navigate`.
- NPC-versus-NPC duels.
- Duplicate node names.
- Legacy saves in a dry pool.

**Refs:** `maps/wchess1.scr` (98-132, 134-141, 143-296, 300-1561); `maps/cinematics/bishop_puzzle.scr` (82-1930); `maps/cinematics/knight_puzzle.scr` (77-1720); `ai/c_chess_*.st`; `models/c_chess_*.tik`, `bishop.tik`, `knight.tik`; `global/alice_legs.st:13, 45-77`; `global/alice_torso.st:15, 79-88`; entities #1, #20-#51, #69-#71, #75-#77, #91, #95-#98, #102, #103, #107-#113, #116, #118-#131, #134, #141-#287, #292, #293, #333-#366, #531, #532, #635-#664, #804, #920-#972, #1021, #1056, #1134-#1212, #1340, #1341, #1359-#1401.

<a id="v21-wchess2"></a>
## 21 · wchess2 (Castling)

| Field | Value |
| --- | --- |
| Visit | `wchess2$wchess2_start1` (the only start, #37 at 256 -4800 0, angle 270) · ROUTE 20 · next `rchess1$rchess1_start1` |
| Status | **BLOCKED: start-room soft-lock.** Doors #40/#41 (x 16, spawnflags 4176) are locked by the provisional 4096 rule (`interaction.rs:300`), and locked doors refuse use (`interaction.rs:1191-1199`). The start area is enclosed. After a door fix, the static always-live `exit_trigger` #32 would become a bypass |
| Size / milestone | M · M4 |
| Depends on | W3 (**the 4096 rule for #40/#41**), E14, W22b (bound portal trigger and gate), F4 movers (portal on a looping spline with rider carry; the queen transport; `red_platform`), C1 (2 scenes), C3 (King rig and speaker mapping), R1 |
| Reservations | hit-ID 8,000,000 · `wchess2/` · `wchess2.` · `--wchess2-*` · `wchess2-*` |

**Today:**
- No controller. All thread triggers are pending.
- `w_queen`, `r_rook_abductors`, `kings_pawn` and the `dead_*` props are `c_*` script_models excluded by `decorations.rs:130-136`, so they are invisible.
- The King, rooks and knights are stationary NPCs. The battle groups never spawn.
- There are no story beats for the King conversation, and `speaker()` (`story.rs:283-300`) has no chess-king mapping.
- `c_chess_king` is not in the facial rig check (`facial.rs:318-327`).

**Arrival and init** (`wchess2.scr:79-159`):
- `r_knight_queen1`/`2` and `w_rook1`/`2` AI off (the rooks get takedamage and `minhealth 10`);
- `w_king` AI off and non-solid;
- the `dead_*` pieces frozen on their death frames;
- `w_king` and the fake player at `maxmouthangle 45`;
- 10 camera paths;
- `w_queen` and `r_rook_abductors` bound to `w_queen_parent`;
- `kings_pawn` hidden;
- the group4/5 triggers disabled;
- **`exit_trigger` disabled and bound to the hidden, non-solid `exit_portal`**.

**Progression and gates:**
1. **Leave the start room** through the double door #40/#41 (x = 16). Requires the door rule to be fixed.
2. **Queen abduction.** #28 (a band at x -1600..-512, y -4032..-3840) runs `cinema_queen_abduction_thread` (about 20 s):
   - cameras `wchess2_queen_camera1`/`2`;
   - `castle_frontdoor` opens;
   - the red knights run;
   - `w_queen` with the two abductors travels `way1`..`way10` onto `red_platform`, which slides 512 west over `trigger_fall` #21;
   - all of them are removed, and the dead pieces are removed;
   - both red knights and the `r_rook_attack1`/`2` func_spawns (pawn models) attack.

   The watched path leaves Alice in place (the warp is commented out). The skip `cqat_End` warps her to `alice_queen_dest1` (-1024 -4096 -64).
3. **Battle group 1** (#34): 5 spawns, red rook, bishop and pawn against white bishop and knight (health 10000). The red pawn attacks Alice.
4. **Optional side groups** (whichever of each pair is touched first removes its sibling): 1a/1b red rook, 2a/2b two knights, 3a/3b bishop.
5. **Battle group 2** (#370, #679-#681): removes the group 1-3 triggers and spawns 8. Its counter points at missing names, so it never completes (a faithful no-op).
6. **King audience.** #24 runs `cinema_king_thread` (**about 75-85 s**; 11 lines of about 53 s total, each with a 1 s pad):
   - Alice warps to `alice_king_dest1`, then walks to `dest2`;
   - `kings_pawn` follows `kings_pawn_path1` and shrinks;
   - Alice plays `darkened_lookingglass`, `changeweapon`, `ready`;
   - **`exit_portal` becomes solid and shown and loops `exit_portal_path1`; `exit_trigger` is armed**; the King becomes solid;
   - the group 1-3 actors are removed; the group4/5 triggers are armed; the battle-2 allies are removed;
   - `w_rook1`/`2` walk 10-node routes to flank the portal.

   The skip `ckt_End` commits the same, except that it never makes `w_king` solid; make him solid in both paths.
7. **Exit portal.** `exit_portal` #23 is a door frame on a 128x128 slab bobbing along the spline `exit_portal_path1`, `t1`, `t2`, `t3` (origin z 448/424/400/424, so the slab top moves 512 to 464). `exit_trigger` #32 fills the frame opening. Step onto the slab from the z-512 platform and walk through. The group-4 spawners have their targetname and spawntargetname swapped, so they never spawn. The group-5 triggers sit on the far-south walkways and are not on this path.

**Exit:** `exit_trigger` #32, bound to `exit_portal` #23, leads to `rchess1$rchess1_start1`. Off from `post_player_init` (`wchess2.scr:155`) until `cinema_king_thread` ends (:780) or `ckt_End` runs (:567).

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Red knights `r_knight_queen1`/`2` | 2 | Hostile after the queen scene | no |
| Red pawns `r_rook_attack1`/`2` | 2 | After the queen scene | no |
| Battle 1 reds (rook, bishop, pawn) | 3 | #34 | no |
| Side groups | 4 | First side touched only | no |
| Battle 2 reds | 4 | Any battle-2 trigger; duel allies | no |
| Group 5 red pawn | 1 | Armed after the King scene | no |
| Group 4 knights; battle-3 pawns (#724/#725) | 0 | Never (swapped names; unreferenced thread) | - |

**Cinematics:**
- **Queen abduction.** The commit is `cqat_End`: props removed; dead pieces removed; Alice at `alice_queen_dest1`; the knights hostile; the attack pawns spawned.
- **King audience.** Cuts driven by beat completion. The commit is `ckt_End` plus the King made solid.

**Movers:** the exit portal (slab rider carry; the trigger follows the pose, following the `trigger_pose` pattern at `interaction.rs:988/1011`); the queen transport (`w_queen_parent`, `way1`-`10`, `red_platform`); `kings_pawn` (spline and shrink); doors (`castle_frontdoor` opened by script; the start doors must be usable; 30 leaves with spawnflags 64; the secret `func_door` #10 in front of the RageBox); 45 small flame `trigger_hurt` volumes (default damage).

**NPCs and dialogue:** `cinema_king_thread` has 11 voiced lines: King `wtk001`-`006` (lip tracks exist) and Alice `alcz3001`-`3005`. Needed: a story beat, a chess-king speaker mapping, and `c_chess_king` in the facial checks. `c_chess_king.tik` has `nodamage` and health 300.

**Special mechanics:** a moving, bound changelevel inside a bobbing door-frame portal; transport over a fall pit; ally battles (allies at 10000 hp); mutually exclusive trigger pairs; trigger removal by later triggers; flame hurts; faithful no-ops for the broken names.

**Rewards:** no toy. RageBox #9 (behind `func_door` #10); super essence #1; pickups.

**Entity classes:** `trigger_once` (removable; spawnflags 8 variant), `trigger_changelevel` bound to a mover, `trigger_hurt`, `trigger_fall`, `func_spawn`, `func_rotatingdoor`, `func_door` (secret), `script_object` (`exit_portal`, `red_platform`), `script_origin`, `script_model`, `info_splinepath`, `info_pathnode`/`info_waypoint`, `func_camera`, `Characters_chess_King`/`rook`/`red_knight`, items.

**PLAYABLE MVP checklist:**
- [ ] W3: the start leaves #40/#41 must not be locked. Use a reviewed per-map override, or a global fix after research. Prove it with `navigate` from the start to (-1024, -4096, -64).
- [ ] Registry controller owning `w_king`, `w_rook1`/`2`, `r_knight_queen1`/`2`, the dead-piece poses, the queen transport props, `kings_pawn` and the exit portal and trigger.
- [ ] The exit gate (`wchess2.exit_open`); the portal solid, visible and looping; slab rider carry; the pose-following bound trigger.
- [ ] Chess encounters: battles 1 and 2 with deterministic staged duels, pair exclusivity, trigger removal, post-King removals and group-5 arming; the broken names stay documented no-ops.
- [ ] The two scenes with their commits, with the King lines as story beats (C3: speaker mapping, rig).
- [ ] Saves: scene ids and clocks, `exit_open`, the portal spline time, rook route indices, group and battle activations. Entrance restart for legacy saves. Cases: `wchess2-queen-mid`, `wchess2-king-mid`, `wchess2-exit-open`.
- [ ] Launcher `tools/launchers/Launch-Castling.cmd`; `docs/WCHESS2.md`.

**Route check (`--wchess2-route-check` and skip variant):**
1. Assert the start doors open; into the courtyard.
2. Cross the queen band near (-1056, -3936). Watch or skip; assert the props removed and the hostiles active; fight or outpace them.
3. Navigate through `castle_frontdoor` (-1024, -3320) and battle 1 (-1024, -2616, 128); assert the allies alive; battle 2 (-272, -576, 192); the throne doors (-1024, 224, 384).
4. **Negative first:** standing at the exit volume or the platform edge must not transition before the King scene.
5. Enter the King volume; advance all 11 lines, or skip. Assert `exit_open`, the portal visible and solid, and the rooks walking.
6. Onto the z-512 platform; wait for the slab top at 512; step on and walk through the frame.

Assert `transition == ("rchess1", Some("rchess1_start1"))` and alive. The watched and skipped final snapshots are equal except for clocks.

**Anode checks:**
- [ ] `--wchess2-render-check`: queen transport frames; the King two-shot with his mouth open on a line; `kings_pawn` mid-shrink; the portal at both bob extremes.
- [ ] `tools/test_visibility.ps1`: Alice visible after both scene endings and after F9 of a mid-dialogue save.
- [ ] Real input: walk through the start doors (proves the door fix); skip the queen scene; at the King, watch two lines, F5, kill, `--load quick` (same line); hold Enter to skip; ride the slab through the frame. rchess1 autosaves and the save restores in a fresh process.

**Fidelity follow-ups:** queen drag and struggle animation and loop sound; King and Alice lip-sync and head-watch; poses; the rooks' escort walk; ally battle choreography; flame hurt timing; chandelier fullbright; the secret door; the group-5 trigger spawnflags 8.

**Risks:**
- The 4096 semantics (U4).
- The bound trigger on a moving portal.
- The King lines in a map the story reader does not yet support.
- Ally duels stalling with 10000 hp.
- Legacy saves.

**Refs:** `maps/wchess2.scr` (79-159, 161-364, 366-461, 463-540, 555-885, 887-949); entities #1, #6, #9, #10, #13, #16-#18, #21-#24, #27-#41, #370, #418-#426, #432-#442, #473-#478, #494-#496, #499-#650, #662-#699, #718-#726, #789-#796, #985-#990, #1007-#1139.

<a id="v22-rchess1"></a>
## 22 · rchess1 (Checkmate in Red)

| Field | Value |
| --- | --- |
| Visit | `rchess1$rchess1_start1` (the only start, #34 at -2240 992 64, angle 270) · ROUTE 21 · next `funhouse$funhouse_start1` |
| Status | **BLOCKED.** There is no `trigger_changelevel`. Leaf #242 (546 2544 320, spawnflags 4160, locked by the 4096 rule) cuts off everything after the Dice room. The drawbridge `script_object` is missing, so the King trigger #28 is unreachable. The Red King has no combat |
| Size / milestone | L · M4 |
| Depends on | W3 (#242 plus eight other 4160 leaves), W1 (drawbridge and chains, `king_clip` monsterclip walls, `bridge_monster_clip`, blade, queener), E14 (groups 1-10), **B2 Red King**, T6 and C6 (`killdemons`, `bosslevel`), T8 (the 1 s single-live essence cycle), W22b (a script exit that fires once), C1 (3 scenes), C4 (catmessage #826), R1 (optional rope #32), R2 (boss loop) |
| Reservations | hit-ID 8,100,000 (the King inside it) · `rchess1/` · `rchess1.` · `--rchess1-*` · `rchess1-*` |

**Arrival and init** (`rchess1.scr:93-213`):
- the fires hidden;
- the spectators, beheader and queen parts AI off;
- the queen head and body each hide one surface and loop their struggle;
- `r_king` AI off, hidden and nodamage;
- `king_bishop1`/`2`, `revived_queen` (plus its effect), `kings_pawn` and `portal_hatter` hidden and non-solid (`kings_pawn_shrink` does not exist);
- `asylum_hatter` removed; `asylum_alice` posed;
- `bridge_monster_clip` off; `locked_doors` locked;
- the waterwheel spins at 45 deg/s; the begin lamps rotate;
- 15 camera paths;
- `king_bridge_chains` bound to `king_bridge`;
- the queener (with `portal_alice`, `portal_hatter`, `revived_queen`, `kings_pawn`) parked at `queener_way1`;
- `exit_portal` hidden.

**Progression and gates:**
1. **Western approach.** Group 1 (#24, 2 pawns); group 2 (#462, rook); group 3 (#465, knight) or group 5 (#25, at the rope head: pawn and bishop, which removes group 3's trigger); rope #32 over the `chess_red` water shaft (optional per a coarse probe); group 4 (#466, bishop and pawn); group 6 (#469, 2 pawns).
2. **Third Demon Die**: altar #9 (30 2302 296); Dice copies become 3.
3. **Beheading.** Through leaf #242 to #670, which runs `cinema_beheading_thread` (about 20 s):
   - cameras `rchess1_queen_camera1`-`4`;
   - the guillotine blade rises 48, then drops 96;
   - the queen head and body poses;
   - `spec_exit_door` opens; the spectators walk off and are removed;
   - the beheader is removed;
   - Alice walks to `beheading_node1` (608 2736 264).

   The skip `cbt_End` warps her there but leaves the blade at its start height; choose one canonical blade pose (default in How to read).
4. **Northern loop.** Group 7 (#484, bishop), group 8 (#485, 2 knights), group 9 (#489, bishop and 2 pawns), group 10 (#488, 2 rooks); then down to the drawbridge.
5. **Red King battle start.** #28 runs `cinema_king_battle_thread`:
   - `bosslevel(1)`, `killdemons`;
   - Alice to `alice_king_dest1` (3936 2944 0);
   - `king_bridge` rises (pitch 80 over 4 s; chains 65); `king_clip` (4 monsterclip walls) drops 256;
   - the King grows (fire effect, scale 0.05 to 1 over about 1 s, 48x48x128, solid);
   - control returns after 5.5 s; King AI on; `get_me1` spawned.

   The skip windows `ckbt_End` and `ckbt2_End` differ; commit **one canonical state**: bridge raised, clip placed, King full size, solid and active, the essence cycle started, Dice summons dismissed.
6. **Red King fight** (B2, below). The respawning medium essence cycles `get_me1`, `get_me2`, `get_me3`, each appearing 1 s after the previous pickup, one live at a time. `trigger_fall` #796 lies under the bridge and arena.
7. **King killed, then funhouse.** `cinema_king_killed_thread` runs only if Alice is alive:
   - `bosslevel(0)`; the King, `peanut_gallary` and `king_clip` removed;
   - Alice to `alice_king_dest2`, then `dest3`/`dest4`;
   - cameras `rchess1_kmpx3`/`kmpx4`/`kmpx1`/`kmpx2`/`portal`/`funhouse`;
   - the queener follows `queener_path1`; `bridge_monster_clip` becomes solid;
   - the pawn grows and shrinks; the revived queen grows; the king pieces pop in with fire;
   - `portal_hatter` strikes `portal_alice`;
   - a white fade and a drugview camera fall (about 11 s);
   - the level change to `funhouse$funhouse_start1` (:1122).

   The skip `ckkt_End` reaches the same level change (:708). The commented old version (:720-879) is dead.

**Exits:** the script `map()` at the end of the killed scene, or in `ckkt_End`, leading to `funhouse$funhouse_start1`. Emit it once, only if Alice is alive.

**Enemies:** groups 1-10 (red pawns, rooks, knights, bishops; about 19 pieces on Normal), activated by their triggers. None gate the exit.

**Boss B2: Red King** (`r_king` #30, `c_chess_red_king.tik`, `ai/c_chess_king.st`):
- 1300 hp; pain threshold 65; 48x48x128; vision 800; deathsink.
- Walks inside 400. Melee `attack_1` inside 110 (two 10-damage frames).
- The ranged attack (chance 0.7/0.4) is chosen by HEALTH (the condition direction needs research, U6):
  - `attack_4`: seeker, four lightning beam pulses from the orb (500 units);
  - `attack_3`: grenade `prj_kingball` (500 u/s, 5 damage, explosion);
  - `attack_2`: blast `prj_diamond` (700 u/s, 7 damage, knockback 150).
- `BLOCK_SHIELD` on an incoming projectile plays `attack_5` and fires a seeker `prj_kingballseek` (300 u/s, 5 damage).
- Pain, and death variants including an ice-frozen death.
- The arena is sealed by the raised bridge and monster clips. Retry after Alice's death happens inside the arena and keeps the King's state (the Duchess precedent).

**Cinematics:**
- **Beheading.** The commit: queen parts in their final poses, spectators and beheader gone, `spec_exit_door` open, Alice at `beheading_node1`, the canonical blade pose.
- **Battle start.** The canonical commit above.
- **King killed.** A minimal staging (queener, tweens, piece pop-ins, the Hatter strike, fade, fall camera), then one transition. The skip commits the same transition; the transition survives F5/F9 mid-scene.

**Movers:** the drawbridge (raised behind Alice; the unused lowering thread stays unused); `king_clip` and `bridge_monster_clip` (monster-only); the guillotine blade; the queener and portal paths; the essence spawners `get_me1`-`3` (the `pickup_thread` names `Centipede2_ME1`-`3` are reused data); rope #32 and the red water; the doors (`locked_doors` stay locked; `spec_exit_door` opens only by script; #242 must be usable); the decorative waterwheel and lamps.

**NPCs and dialogue:** no voiced lines. `trigger_catmessage` #826 (`catz312`) is near the asylum tableau; it is only a hint, and the area may be unreachable (U9). The scene-only cast includes `portal_hatter` (`Enemies_MadHatter`), which must never become a combat enemy or a generic NPC here.

**Special mechanics:** a boss arena sealed by raising the drawbridge; monster-only confinement clips; health-selected ranged attacks; counter-fire shield; seekers and beams; `killdemons`/`bosslevel`; a single-live respawning essence; a script exit after a long scene, guarded by Alice being alive.

**Rewards:** **the third Demon Die** #9 (Dice at 3); RageBox #10 and #825; large essence #830; mana #22; respawning essences. #825 and #830 are in the asylum area, whose accessibility is unconfirmed.

**Entity classes:** `trigger_once`, `trigger_fall`, `trigger_catmessage`, `func_spawn`, `func_rotatingdoor`, `func_rope`, `script_object`, `script_model`, `SFX_GnomeHide`, `info_splinepath`/`info_pathnode`/`info_waypoint`, `func_camera`, `Characters_chess_red_King`, `Characters_chess_queen`, `Characters_chess_red_*`, `Enemies_MadHatter` (scene-only), items, air emitter, chess props.

**PLAYABLE MVP checklist:**
- [ ] W3 for rchess1's nine 4160 leaves (#242 is on the main route). `locked_doors` #145/#146 stay locked; `spec_exit_door` #216/#217 opens only by script. Prove the entrance to Dice room to beheading to bridge path with `navigate`.
- [ ] Controller: load-time hiding, queen surface hiding, NPC ownership (`r_king`, `portal_hatter`, the spectators and queen parts are not generic), movers.
- [ ] Groups 1-10 through E14; group 5 removes group 3's trigger.
- [ ] B2 boss module (on the Duchess pattern): TIKI stats, attacks and projectiles, beam, shield counter-fire, confinement, death and sink, loot, notarget, knockback, arena recovery.
- [ ] Battle start with the canonical commit; `killdemons` through the Dice dismiss (`dice.rs:458`); `bosslevel`; the essence cycle.
- [ ] The beheading scene.
- [ ] The killed scene with its once-only transition, only if Alice is alive.
- [ ] Saves: scene ids and clocks, bridge angle, clip, boss state (hp, action, timers, projectiles, beams), essence index and timer, summons dismissed, transition latch. Cases: `rchess1-beheading`, `rchess1-king-start`, `rchess1-king-fight`, `rchess1-king-killed`.
- [ ] Launcher `tools/launchers/Launch-Checkmate-in-Red.cmd`; `docs/RCHESS1.md`.

**Route check (`--rchess1-route-check` and skip variant):**
1. Groups along the discovered path (group 1 and group 6 are likely); the rope only if needed.
2. The Dice altar (30, 2302, 296): assert `copies(6) == 3`.
3. Through #242 to the beheading at (560, 2720, 352); assert the scene commit.
4. Groups 7-10; cross the drawbridge into #28. Assert the bridge raised, the clip placed, no Dice summons, the King at 1300 hp.
5. Boss loop: strafe and throw with R2 weapons; divert to the live essence when Sanity is below 50. Assert every ranged mode and the counter-fire occurred, and the King's hp reaches 0.
6. Watch (about 35 s) or skip.

Assert `transition == ("funhouse", Some("funhouse_start1"))`, Dice at 3, alive, one transition.

Contracts: Alice's death mid-fight recovers inside the arena with no transition; a King death while Alice is dead starts no scene; mid-fight and mid-scene JSON round trips.

**Anode checks:**
- [ ] `--rchess1-render-check`: beheading shots; the raised bridge; the King's grow; each King attack (beam, kingball, diamond, seeker) and death; the portal and Hatter frame; the fall camera.
- [ ] `tools/test_visibility.ps1` after each scene and after F9 mid-fight; `tools/test_render_fx.ps1` (beams and explosions).
- [ ] A real-input boss fight from the bridge fixture: skip the intro; fight with the Blade, Cards and Dice (confirm existing summons were dismissed); F5 at about 600 hp, kill, `--load quick`; win; watch the scene. funhouse has Dice at 3. A second run skips the death scene with the same result.
- [ ] Real input through door #242; the Dice pickup, then F5/F9 across a restart.

**Fidelity follow-ups:** beheading blood and sounds; spectator walk paths; King randomness and HEALTH semantics; beam and kingball visuals; seeker homing; deathsink; the ice-frozen death; killed-scene choreography; the `catz312` hint and asylum tableau; ambience.

**Risks:**
- The 4096 rule (U4).
- The HEALTH condition direction (U6).
- The length of the 1300 hp fight in the harness.
- The once-only transition.
- `portal_hatter` and `r_king` are generic NPCs today, which affects save identity.
- `killdemons`/`bosslevel` semantics.
- Unknown route details.

**Refs:** `maps/rchess1.scr` (40-56, 93-213, 215-257, 261-504, 506-532, 535-716, 882-1124, 1146-1297); `ai/c_chess_king.st`; `models/c_chess_red_king.tik`, `prj_kingball.tik`, `prj_kingballseek.tik`, `prj_diamond.tik`; entities #1, #4, #5, #9-#18, #22-#32, #34, #107, #145, #146, #216, #217, #241-#262, #407-#409, #461-#492, #670, #691, #695-#715, #748-#783, #794-#797, #815-#826, #830, #860-#866.

---

# Funhouse and Hatter (M5)

Shared M5 systems:
- shootable switch triggers (W4) with smashable clocks (W8);
- sliding `func_door` (W2) and the door-flag policy (W3);
- fulcrums (W11: static MVP), sink objects (W10), grav suction (W12);
- inline patch collision (W21), planar mirrors (W19), beams (W17), `trigger_remove` (W18);
- generic levers (W7);
- the Clockwork (E15), Nightmare Spider (E16) and Phantasm (E12) archetypes;
- the Tweedle (B3) and Mad Hatter (B4) bosses;
- quest pickups and pickup_thread (T7); essence chains (T8);
- `allow_cheshire` windows (C4, DG-8);
- the Gryphon attach ride (C2).

<a id="v23-funhouse"></a>
## 23 · funhouse (Mirror Image)

| Field | Value |
| --- | --- |
| Visit | `funhouse$funhouse_start1` (the only start, #149 at 32 2928 208, yaw 90; rchess1 calls `map()` into it) · ROUTE 22 · next `hatter1$hatter1_start1` |
| Status | Machinery, clock gates, paired Tweedles, scenes and exit implemented in playtest 6C9850B5; full combat route remains unverified at the pendulums. Current evidence and fidelity limits: [FUNHOUSE.md](FUNHOUSE.md). |
| Size / milestone | XL · M5 |
| Depends on | W1 (movers and unreferenced static art), W2, W3, W4 (shootable clocks), W6 (closets), W8, W11 (static fulcrums), W12 (head suction), W14 (secret teleport #12), W18, T13 (difficulty triggers), E12, E13, E15, E16, **B3**, T7 (the Jacks Cat line), T8 (`help_me`), C1 (5 scenes), C3, C6, R1, R2 |
| Reservations | hit-ID 8,200,000 (the Tweedles and minis) · `funhouse/` · `funhouse.` · `--funhouse-*` · `funhouse-*` |

**Implementation:** One visit owner in `src/levels/funhouse.rs` and companion modules. It owns the machinery, clock mask, named bosses and minis, five scene lifecycles and exit. Shared placed-enemy owners handle the reviewed closet and wall-spider activations. Save envelope 12 and the existing hit-ID reservation are unchanged.

**Native door correction:** 4096 means auto-close; 64 permits player proximity; 16 excludes Actors. Both 4160 cell doors and 4176 castle doors admit Alice. The map script does not lock the cell doors. The former closed-cell assumption is superseded by native handler research.

**Arrival:**
- Alice stands on `cine_stand` #104, an invisible 80x16x8 player-clip ledge about 200 units above the hall floor.
- main:
  - hides the steam and binds pipe fragments;
  - sets `ai_off` on `L`/`R_wall_death1`-`4`;
  - `Funhouse_Cinematics_Init` (13 cams; hides and warps the Cats; AI off on the Tweedles and Hatter, with the Hatter hidden, non-solid and nodamage; registers the killthread `Funhouse_Check_Exit` on both Tweedles);
  - `full_stats`;
  - `Funhouse_World_Init`: the `skymojo` sky; pendulums; `biggear1` spinning at 30 deg/s; the tube bound; the gas doors bound; 16 cell kids AI off.
- Then the intro, and the endless `head2` suction loop.
- The intro `Funhouse_Start` is skippable (`Skipthread_F1`/`F1_StartEnd`). Shots: `path3`, `jpath2`, `cell2` (smashes `clock_cell2`), `jpath1` (smashes `cin_clock_break`; `fake_dum` alerts), `path4` (`cat_actor1`, line `catz309`). `fake_dum` is removed; 2 s later **`cine_stand` slides 64 units south, out from under Alice**, and `gas_count` becomes 7.

**Progression and gates:**
1. **Intro commit.** Cell 2 is freed. The gas counter must be an **idempotent 8-bit mask**, because `F1_StartEnd` re-runs cell 2.
2. **Jacks altar** (32 2272 8, inside the `auto03` closet; the push `auto03push` #48 lies in front of the glass #1138). Its pickup_thread `Cat_Jack_Dialog` plays `cat011` with a Cat fade. It is the reward, not a gate.
3. **Clockwork and phantasm hall** (y 700-3300). Closet spawns: `autoNN`/`phantNN` fire func_spawns, smash the closet glass, and disable the push after 1 s. Targets activate the delayed groups `t92`, `t102`, `t163`, `t168`. Trigger #529 toggles the four `t96` doors around clockwork #434. The `t86` door stays shut. #1155/#1156 target missing `t171`/`t173`.
4. **Mirror clock.** Shoot #66 (health 1): `clock_mirror` shatters into gear debris, `pend_mirror` stops, and `mirror_path_door` (a `func_door`, TARGETED+TOGGLE, speed 50) opens.
5. **Tube.** `tuber` #1121 lies over `trigger_fall` #75. #65 toggles the mirror door shut behind Alice. #112 runs `Funhouse_Rotate_Tuber`: yaw -90 over 8 s, carrying Alice. **The secret teleport #12 lands at `t198` (-784 -448 0) past the tube**, which may bypass the mirror and tube (U9).
6. **Cell wings.** `L`/`R_KILL_EM1`/`2` wake the wall spiders. **Shoot the 7 clock switches** (health 1 each): `cell1` #63, `cell3` #1160, `cell4` #1159, `cell5` #1161, `cell6` #1162, `cell7` #1163, `cell8` #1164. Each smashes its clock, stops its pendulum, hides its gas emitter (cell 4 sets its animation to off), wakes its kids, and sets a mask bit. The clocks are at z 256-320, above cell doors that open on player proximity.
7. **`GasOpenCine`** once all 8 cells are freed (no skip thread is authored): `pipe_cam` on `funhouse_pipe1`; pipe fragments fly; `FlipMove` blows both gas-door pairs out; all four doors are removed.
8. **Castle doors and fulcrum rooms** (either wing). Pass the doors (4176). Cross 9 fulcrum see-saws per side over a pit.
   - Normal/Hard: `trigger_fall` #74/#1165 kill.
   - Easy: the teleports #4/#1269 return Alice, and `trigger_remove` #2/#1270 clear the pit bottom.
   - Boojums `t164`-`t167` wake on #80-#82 and #1146. Ten non-hostile children stand at z 400.
9. **Heads area and pendulum corridor.**
   - `head2` loop, 7 s cycle: steam for 2 s; suction for 2 s through `info_grav_pathnode` (speed 400) toward the mouths (`trigger_hurt` 1000), with `head2_teeth` dropping 100 and rising; 3 s of rest.
   - Slime under both areas.
   - `pend1`-`3`: solid blocks about 128x256x620, swinging ±24/12/24 degrees, one with a phase offset.
10. **Tweedle arena.** #43 runs `Funhouse_Tweedle_Cinema1`:
    - `bosslevel(1)`, `killdemons`, `level_ai(0)`;
    - Dee and Dum at their positions; the `end_door1`/`2` cycle;
    - Alice at `alice_pos1`;
    - lines `tde001`-`003`, `tdm001`-`002`, `alcz3006`-`3007`, `catz310`-`311`;
    - Dum to `dum_pos2`; both `ai_on`; `help_me2` spawned.

    Skip `F2_End`. **The arena floor (`break_plat` plus the ice pieces) must be solid from load.**
11. **Boss defeat.** Each named Tweedle's death runs `Funhouse_Check_Exit`. When `tweedle_counter` reaches 2, a 5 s wait, then the Hatter scene if Alice is alive. `help_me1`/`help_me2` alternate every 10 s.
12. **Hatter scene and exit.** `Funhouse_Hatter_Cinema1`:
    - `hatter_actor1`; Alice at `alice_pos2`;
    - cameras `path5`, `awatch1` (FOV 50 to 130), `path6`;
    - lines `mhat001`-`004`, `alcz3008`-`3011`;
    - camera `end1`: `FunhouseFloorBreak1`/`2` drop the ice pieces and `break_plat` 512;
    - the watched path clears `bosslevel`; then the level change to `hatter1$hatter1_start1`.

    Skip `H1_End`. **Remove live minis explicitly**: the script's `$dee_spawned`/`$dum_spawned` match nothing.

**Exit:** the script `map()` (`funhouse_cinematics.scr:357`; skip :252) leads to `hatter1$hatter1_start1`, after both Tweedles die, then 5 s, with Alice alive.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Tweedledee / Tweedledum (plus minis) | 2 (minis capped at 2 and 3 live) | After the Tweedle scene | **yes** |
| Clockwork Automaton | 16 (10 placed, 6 closets) | On sight; `t96` via #529; #16 on Normal+ | no |
| Phantasmagoria | 9 (7 placed, 2 closets) | On sight; delayed groups | no |
| Nightmare Spider (wall) | 10 | `L`/`R_KILL_EM`; `t172` never; #168 on sight | no |
| Boojum | 22 | 14 on sight; 4 delayed (fulcrum rooms); 2 Normal+; 2 Hard | no |
| Insane children | 29 | Non-hostile; cell kids wake per clock | no |

**Boss B3: Tweedledee and Tweedledum** (`ai/c_tweedle_dee.st`, `ai/c_tweedle_dum.st`):
- **Dee:** 800 hp, pain threshold 50, vision 800.
  - Knife: 10 inside 100.
  - Rattle: `prj_grenade`, 500 u/s, bounces.
  - Propeller flight: up to 10 s, then a fall impact (radius 300, 50 damage, quake 3).
  - `russian_split`: spawns a mini (75 hp, flies) at frame 28, with a player-clip while open.
  - Frozen death; deathsink.
- **Dum:** 900 hp, vision 1000; the same kit; the READY state; splits with a live cap of 3 (minis 50 hp, fly); quake 2.5.
- Minis do not count toward the exit.

**Cinematics** (skip commits the listed `*_End` state):
- `Funhouse_Start` (`F1_StartEnd`);
- `Cat_Jack_Dialog` (pickup beat);
- `GasOpenCine` (skip allowed as a deviation; the doors are removed);
- `Funhouse_Tweedle_Cinema1` (`F2_End`; boss HUD; demons killed);
- `Funhouse_Hatter_Cinema1` (`H1_End`; the transition once; minis removed; `bosslevel` cleared).

`funhouse_gas1` is loaded but never used.

**Movers:**
- shootable clock switches (`trigger_once` with health 1, `func_smashablewall`, pendulums; cell pendulums are patch-only);
- `mirror_path_door` (toggle; model its state explicitly);
- the tube (patch plus 3 allclip troughs, rider carry);
- `cine_stand`;
- closet spawners;
- the `t96`/`t86` doors;
- 18 fulcrums;
- head suction (6 grav nodes, `head2_teeth`, steam, 1000-damage mouths);
- `pend1`-`3`;
- `biggear1` (the secret ledge);
- gas doors and pipe fragments;
- the arena doors and floor;
- unreferenced static art (`rattle_door1`-`6`, #105, `faux_pipe`, #83-#86);
- the `help_me` spawners.

**NPCs and dialogue:**
- Cheshire: `cat_actor1` (`catz309`, `catz310`, `catz311`) and `jack_cat` (`cat011`);
- Tweedles `tde001`-`003`, `tdm001`-`002`; the Hatter `mhat001`-`004`; Alice `alcz3006`-`3011`;
- 11 unnamed hint regions;
- 29 ambient children.

`fake_dum` is a `c_` script_model, now drawn by the scene owner during the intro.

**Special mechanics:** shootable clocks; a rotating tube over a fall; fulcrum pits (difficulty-dependent); timed suction to kill mouths; slime; closet spawners; the essence chain; cinematic side effects; the end-door area portals; an arena made only of script_objects; the idempotent gas mask.

**Rewards:** **Jacks** (#101, slot 5; REWARDS `campaign.rs:158`); RageBoxes #10/#68; the secret Looking Glass #11; essences and vials.

**Entity classes:** `script_object`, `func_door`, `func_rotatingdoor` (native proximity and auto-close flags), `func_fulcrum`, `func_smashablewall`, `trigger_once` (shootable), `trigger_push` (managed closets), `func_spawn` (actors and pickups), `info_grav_pathnode`, `trigger_hurt`, `trigger_fall`/`trigger_teleport`/`trigger_remove`/`func_teleportdest` (difficulty), `func_camera`, `info_pathnode`, `Enemies_tweedledee`/`tweedledum`/`ClockworkAutomaton`/`Phantasmagoria`/`NightmareSpider_OnWall`/`Boojum`/`MadHatter` (scene), `Characters_CheshireCat`, `Characters_InsaneChild_*`, `Item_WeaponPickup_Jacks`, `script_model` (`fake_dum`), emitters.

**PLAYABLE MVP checklist:**
- [ ] Controller covering every `script_object`, fulcrum, smashable wall and `func_door`. **`break_plat`, `marble_ice1`-`4` and the ice pieces are solid from load.** Draw and collide the unreferenced static art.
- [ ] W3: native proximity and automatic closing for the 12 castle doors (4176) and 8 cell doors (4160). W2 for `mirror_path_door` (MirrorTrigger opens it; #65 toggles it), the `t96` doors (#529), and `t86` (shut).
- [ ] W4 shootable switches for `cell1`-`8` and `MirrorTrigger`; the 8-bit `cells_freed` mask.
- [ ] The intro scene and its commit (the stand offset, so Alice drops to the floor).
- [ ] The tube mover (yaw 0 to -90 over 8 s, rider carry, stops if blocked); decide how the secret teleport is handled (default in How to read).
- [ ] `GasOpenCine` when the mask reads 0xFF.
- [ ] W11 weighted fulcrums (verify fairness in Anode); T13 difficulty filtering for the pit triggers.
- [ ] Head suction (the 7 s cycle), teeth, and `pend1`-`3`.
- [ ] Encounters: E15, E16, E12, E13 with activation; closets with glass smash and push disable; `owns_npc`.
- [ ] B3 Tweedle controller (minis, knife, rattle, flight and ground-pound, splits, deathsink, boss HUD) and the `help_me` chain.
- [ ] The Tweedle scene (`F2_End`, `killdemons`, `bosslevel`, `level_ai`).
- [ ] Exit: both dead, 5 s, Alice alive, the Hatter scene (`H1_End`), minis removed, one transition.
- [ ] Beats for `Funhouse_Start`, `Cat_Jack_Dialog`, `Funhouse_Tweedle_Cinema1`, `Funhouse_Hatter_Cinema1`. The pickup_thread hook for the Jacks. `full_stats` on first entry.
- [ ] Saves: the upgrade path, entrance restart, NPC regeneration. Cases: `funhouse-intro`, `funhouse-tube`, `funhouse-cells-7`, `funhouse-fulcrum`, `funhouse-tweedles`, `funhouse-hatter-scene`.
- [ ] Launcher `tools/launchers/Launch-Mirror-Image.cmd`; `docs/FUNHOUSE.md`.

**Route check (`--funhouse-route-check` and skip variant; add a difficulty parameter for the Easy/Normal pit contract):**
1. Wait for the intro; assert bit 2 of the mask, `cin_clock_break` smashed, and Alice on the floor.
2. Walk to (32, 2272, 8) without stepping on `auto03push`; assert `copies(5) == 1` and the Cat beat.
3. South through the hall with auto-aim.
4. `shoot_switch("MirrorTrigger")`; assert the door opens.
5. Ride the tube; assert Alice was carried without falling.
6. Vantage shots for `cell1` and `cell3`-`8`; assert `mask == 0xFF` and the gas doors removed.
7. Cross one wing's doors and fulcrums on Normal (teleports == 0).
8. Numbered pathnodes through the heads area, waiting for the suction-off phase, then past the pendulums without touching slime.
9. The Tweedle scene, then a circle-strafe on the solid arena, collecting `help_me` spawns. Assert both named Tweedles are dead.
10. Assert `transition == ("hatter1", Some("hatter1_start1"))`, and that the next `Route::enter` spawns body-clear.

Contracts:
- the exit is blocked while either Tweedle lives;
- the gas doors block until 8 cells are freed;
- the arena floor is solid until the Hatter scene;
- the 5 scenes are equal watched and skipped;
- determinism and pause;
- mid-motion JSON restore;
- the secret-teleport behaviour matches the decision.

**Anode checks:**
- [ ] `--funhouse-render-check`: intro shots; mirror and cell clocks smashing; the tube mid-rotation; the gas burst; a fulcrum with Alice; heads with steam and teeth; pendulums; the solid arena and the Tweedle two-shot; minis; flight and ground-pound; the Hatter scene and the floor break. Look for magenta fallbacks and missing static art.
- [ ] Real input: skip the intro, take the Jacks (HUD and Cat subtitle), break the mirror clock with Cards, F5, kill, `--load quick` (door open, Jacks owned), ride the tube.
- [ ] A real-input boss fight from a `funhouse-tweedles` fixture: boss bars, minis, knockback, both deaths, the 5 s wait, the Hatter scene (once watched, once skipped). hatter1 autosaves, and Continue works in a fresh process.

**Fidelity follow-ups:** camera interpolation and the FOV zoom; pipe arcs and steam; debris; child waypoint walks; split player-clip; frozen deaths; exact AI probabilities; knife and rattle attachments; lip-sync; the sky, lamps, gear, suction and tube sounds; the secret ledge; area-portal culling; fulcrum tilt; patch collision for the pendulums.

**Risks:**
- Fulcrum semantics (U1); a static pose may be unfair.
- Trigger difficulty filtering is not yet implemented.
- Grav radius and strength against the 1000-damage mouths.
- The arena floor must be solid from load.
- Patch-only models.
- The secret bypass.
- Vantage points for the clocks.
- Many simultaneous actors.
- The 4096 intent.
- Save identity.
- Hot files.

**Refs:** `maps/funhouse.scr` (22-58, 61-100, 103-165, 167-273, 275-309, 311-412); `maps/cinematics/funhouse_cinematics.scr` (28, 32-42, 45-143, 145-203, 205-227, 236-373, 439-569, 571-812); `ai/c_tweedle_dee.st`, `ai/c_tweedle_dum.st`; models `c_tweedle_*`, `c_clockwork`, `c_spider_wall`, `c_phantasm`; entities #2, #4-#20, #31, #36-#79, #83-#86, #101-#105, #112, #115-#171, #290, #411-#481, #494-#507, #529, #875-#934, #1110-#1270.

<a id="v24-hatter1"></a>
## 24 · hatter1 (Crazed Clockwork)

| Field | Value |
| --- | --- |
| Visit | `hatter1$hatter1_start1` (the only start, #138 at -2576 3632 320, yaw 270) · ROUTE 23 · next `hatter2` (default start) |
| Status | **BLOCKED and BYPASS.** The movers are absent; the levers are unhandled and invisible; the targeted doors (#96/#97, `autodoor1`-`3`, #116/#117) open with E; `exit_level` #121 and the teleports `lastminute` #10 and `extendo_trigger` #35 are always live; a pit jump reaches `lastminute`, and from there the `stop_clock` lever (a likely sequence break) |
| Size / milestone | L · M5 |
| Depends on | W1, W2 (`port_bent`), W3 (targeted doors locked until their scenes, **including the partner leaves #97 and #117**), W7 (**5 levers**), W10 (**18 sink objects**), W14 (`lastminute`, `extendo`, `clock_teleport`, **the chair-room floor teleport #128**), W17, W19 (the mirror hint, or a recorded fallback), W21 (`brk_pend`), T13, E12, E13, E15, E16, C1 (4 scenes), C3, C6, R1 |
| Reservations | hit-ID 8,300,000 · `hatter1/` · `hatter1.` · `--hatter1-*` · `hatter1-*` |

**Arrival:** no intro (`Hatter1_Start` is commented out). Alice faces the clock pit; the big clock is south.
- `Hatter1_Cinematics_Init` (68-105): loads 21 cams; the Hare, Mouse and Gryphon get AI off; the sprays are hidden; the key is turned.
- main (642-800):
  - binds `fake_hare` to `dunkpanel` and hides it;
  - `extendo_trigger` off;
  - retracts `extendo` 332 east; rotates `no_more_cheating` to 120; raises `astralmonkey` 440;
  - AI off on `spider_wall1`-`8`, `end_spider` and `get_her1`/`2`;
  - hides `loco_hatter`; `mirror_cat` becomes onlymirror; `hatter_cat` goes to alpha 0 and its trigger is disabled;
  - `sink1`-`8` become nomirror, and `fake1`-`8` (onlymirror) are bound to them;
  - starts `st_boo1` on `boopath1`, `Spiral_Thread`, and the `brk_pend` pendulum.
- `Hatter1_World_Init`: sky; watch-hand animations; `monkey3` pendulum; spinning gears (`monkey1`/`2`/`4`, `pend1`-`4`, `gear3` and `biggear1` do not exist); `lastminute` and `exit_level` off.
- `full_stats`.

**Progression and gates.** The order is inferred from coordinates (U9).
1. **Start ledge to the spiral room.** Auto doors #119/#120. `spiral_o_doom` spins; the plunger rises 120 over 3 s and drops over 1 s. #23 wakes `spider_wall8`. The clock-pit rescue: on Easy/Normal, `remove_this_trigger` #133 teleports to `tel2`; on Hard, `trigger_fall` #50 kills.
2. **Gear-room levers** #376 (`Gear_Bridge_One`) and #378 (`Gear_Bridge_Two`), count 1 each, in either order. The first sets `astralmonkey` spinning (50 deg/s); the second raises `no_return` 300 and lowers `astralmonkey` 440, so its disc top (z 208) is flush with the floor: **a spinning bridge over slime** (dmg 1000 as crush only).
3. **One-way passage.** `NORETURN` #21 drops `no_return` behind Alice. Model it as a state.
4. **Mirror, sink and lift zone.** Doors #112/#113.
   - 13 sink pillars (80x80x192, tops at z 320) stand in slime. Five are safe (speed 0: #803-#807). Eight are traps (`sink1`-`8`, speed 2000); their `fake1`-`8` show only in `portal_surface` #800.
   - Gaps are 160-170 units edge to edge: check the jump reach.
   - `mirror_cat_trigger` #48 (`catz306`, visible only in the mirror).
   - `Lift1` (`Lift_Up` #53, `Lift_Down` #816, ±170, `lift_is_up`).
   - `Start_Floater1` #56: `floater1` loops `f_pth1`-`4` over slime.
   - Tea #55.
5. **ClockRoom doors.** Lever #289 (`Open_ClockRoom_Door`) slides both doors 100 units; #60 closes them behind Alice. Model open and closed as states.
6. **Far west and the March Hare scene.** The Hatter hall (#27 wakes `spider_wall5`); the low west route (#25; clockworks); the tea-party doors #98/#99. #199 runs `Hatter1_Cinema1`: the Hare and Dormouse dialogue (`mhare001`-`005`, `drms001`-`004`, `alcz3012`/`3014`/`3015`). At the end the beam, dormouse and dunk loops start. The skip `MD1_End` warps Alice to `alice_watch_hare`.
7. **Port lever** #694 (`Open_Port`) runs `Port_Cine` (`hatter1_jdm1`-`3`): `port_bent` (a `func_door`, TARGETED) opens; `loco_hatter` turns the key; `hatter_door` (with its partner #97) and `autodoor1`-`3` open; `hatter_cat_trigger` is enabled. The skip `PC1_End` re-triggers doors, so commits must be idempotent.
8. **Gryphon.** East along y about 2272. `hatter_cat` #46 (`catz316`, optional). `got_rage` #32 (spiders). South through #92/#93 to #158, which runs `Hatter1_Cinema2` (`alcz3016`-`3019`, `gry001`-`003`). This removes **`donna_hunt`** (an invisible player-clip wall) and **`post_gryph_door`**, and triggers the ground spider `spider_spawn3`. Skip `HG1_End`. The Dice duplicate #36 and the unexplained sink #33 are nearby.
9. **Extendo bridge.** The `end_spider` trigger #14. `extendo` #37 moves the bridge 332 units west over 4 s; `extendo_trigger` becomes triggerable (the Easy/Normal fall rescue); `no_more_cheating` rotates down. Walk into teleport #45, which goes to `tel3` (-768 3232 72).
10. **Giant-chair room.** Four launch pads (#152/#153/#151/#129, to `jump1`-`4`) throw Alice onto the seat cushions (sink objects with spawnflags 6 over mercury). `cube_1`-`4` flags. Floor teleport **#128** (all difficulties) returns a missed launch to `tel3`. When all four are set: `float_clock` moves down 360 and north 70 over 5 s, and `lastminute` is enabled. Candle hurts #81/#660; chair spiders #13; `get_her3`/`4` are live from load. `clock_teleport` #125 goes to `gumpymonkey` (-2144 2272 192).
11. **Stop the clock.** Lever #44 (`stop_clock`) runs `Hatter1_End` (`hatter1_end1`-`3`): the hands turn to 12 and the gears stop; `exit_door1` (#116, with #117) is triggered at about 28 s; `exit_level` becomes triggerable at about 30 s. Skip `End_End`.
12. **Exit** #121 (behind `exit_door1`).

**Exit:** `trigger_changelevel` `exit_level` #121 leads to `hatter2` (default start). Off until `Hatter1_End`/`End_End`.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Nightmare Spider (wall) | 13 | Threads `spider_wall12`/`5`/`67`/`8`, `got_rage`, `end_spider`, `chairspiders`; `get_her3`/`4` live from load | no |
| Nightmare Spider (ground, `spider_spawn3`) | 1 | End of the Gryphon scene | no |
| Clockwork | 3 | Placed #122, #168, #169 | no |
| Phantasm | 1 | Placed | no |
| Boojum | 2 | `st_boo1` patrols `boopath1`; #683 Hard | no |

**Cinematics** (commit = the skip state, idempotent): `Hatter1_Cinema1` (`MD1_End`; start the ambient machinery threads in both outcomes, or in neither); `Port_Cine` (`PC1_End`); `Hatter1_Cinema2` (`HG1_End`); `Hatter1_End` (`End_End`; `level_ai(1)` is restored by `End_Cinematic_Fast`). The Cat beats `mirror_cat` and `hatter_cat`.

**Movers:**
- the gear bridge (`astralmonkey` #123, levers, `no_return` #7);
- the mirror sink field (13 pillars, fakes, `portal_surface`);
- `Lift1` and `floater1`;
- the ClockRoom doors #58/#59;
- the spiral room;
- the `extendo` bridge #39 and `no_more_cheating` #6;
- chair cubes and `float_clock` (#65, #11, #144, #125, #10, #128);
- the targeted doors;
- the corridor blockers #1 and #2;
- sink #33;
- static art (`crazydoor1`-`5`, `magic_clock`, `spin_block`, `standpanel`);
- beams #267/#270/#277;
- candles;
- slime and mercury.

**NPCs and dialogue:**
- the March Hare (`hare_actor1`, later `fake_hare` on `dunkpanel`);
- the Dormouse (an electrocution loop);
- the caged Gryphon (nodamage);
- `loco_hatter` (the key);
- the Cheshire (`mirror_cat` `catz306`, visible in the mirror only, **no .lip** so the mouth stays neutral; `hatter_cat` `catz316`);
- Alice `alcz3012`, `alcz3014`-`3019` (there is no `alcz3013`).

New rigs: March Hare, Dormouse, Gryphon.

**Special mechanics:** levers; mirror-only rendering hints; trap and safe sinks; one-way doors; the four-flag chair puzzle with a floor reset; the float-clock teleporter; difficulty-specific pit rescue; beams; slime and mercury; the `Start`/`End_Cinematic` side effects.

**Rewards:** a Dice duplicate #36 (Will refill at the 3-die cap); RageBox #34; Tea #55; pickups. No toy.

**Entity classes:** `script_object`, `Objects_Lever`, `func_sinkobject`, `portal_surface`, `func_rotatingdoor` (targeted), `func_door`, `trigger_multiple` (lift), `trigger_push` (launch pads), `trigger_teleport` (gated), `trigger_changelevel` (gated), `trigger_hurt`/`fall`/`remove` (difficulty), `func_beam`, `func_spawn`, `Enemies_NightmareSpider_OnWall`, `c_spider`, `Enemies_ClockworkAutomaton`, `Enemies_Phantasmagoria`, `Enemies_Boojum`, `Enemies_MadHatter` (scene), `Characters_MarchHare`, `Characters_Dormouse`, `Characters_Gryphon`, `Characters_CheshireCat`, script_models, items.

**PLAYABLE MVP checklist:**
- [ ] Controller and movers (listed above), including static art and patch colliders.
- [ ] W7 levers: `stop_clock`, `Open_ClockRoom_Door`, `Gear_Bridge_One`/`Two`, `Open_Port`.
- [ ] Gates:
  - `exit_level` needs `clock_stopped`;
  - `lastminute` needs `cubes_done`;
  - `extendo_trigger` needs `extended`;
  - `hatter_cat_trigger` needs `port_open`;
  - `Start_Floater1` disables `mirror_cat_trigger`;
  - `NORETURN` and the ClockRoom close are one-way.

  Lock the targeted doors and their partners until their scenes. `port_bent` is a `func_door`.
- [ ] W10 sinks (traps and safe pillars; cushions; #33); `Lift1`; the floater with rider carry.
- [ ] W19 mirror (or a recorded fallback, DG-10/K13): the fakes and `mirror_cat` visible in the reflection only.
- [ ] Four scenes with idempotent commits; beats; Cat beats.
- [ ] Beams (the dunk and dormouse loops) and `astralmonkey` crush.
- [ ] Encounters: wall spiders with their activation threads; `spider_spawn3`; clockworks; the phantasm; the `st_boo1` patrol.
- [ ] T13: pit triggers by difficulty.
- [ ] Exit gating; `full_stats` on entry.
- [ ] Saves (gear lowered and `no_return` closed; lift down; floater mid-path; after `Port_Cine`; mid `Hatter1_End`), plus entrance restart.
- [ ] Launcher `tools/launchers/Launch-Crazed-Clockwork.cmd`; `docs/HATTER1.md`.

**Route check (`--hatter1-route-check` and skip variant):**
1. Doors #119/#120; the spiral room with phase waits.
2. Both gear levers with `use_at`; assert `astralmonkey` lowered and `no_return` open. Cross; assert `NORETURN` closed it.
3. The sinks using only the safe pillars (check the jump reach); ride `Lift1`; the floater. Assert no trap sank under Alice and no slime damage.
4. The ClockRoom lever; the low west route to the tea party; `Hatter1_Cinema1` (assert `alice_watch_hare`).
5. The Port lever; assert the doors unlocked.
6. Back east and south to #158; the Gryphon scene; assert the blockers removed and the spider active.
7. The `extendo` trigger, the bridge, teleport #45 (teleports == 1).
8. The four chairs (flags 1111); count missed launches separately; the clock descends; enter it (arrive at `gumpymonkey`).
9. `stop_clock`; the scene; walk into #121.
10. Assert `transition == ("hatter2", None)`, and that the destination spawns clear.

Contracts:
- `exit_level`, `lastminute` and `extendo_trigger` are inert before their events;
- the targeted doors refuse E before `Port_Cine`;
- the four scenes are equal watched and skipped;
- levers are one-shot;
- `level_ai` is restored after every scene;
- restore works with movers mid-motion;
- 30/60/144 Hz.

**Anode checks:**
- [ ] `--hatter1-render-check`: the spiral room; the gear lowering; the sink field with its mirror reflection (fakes only in the mirror); the lift and floater; the ClockRoom doors; the Hare/Dormouse beams; the Port key shot; the Gryphon cage; the `extendo` bridge; the chairs and the float clock; the clock stopping; the visible levers and static art.
- [ ] Real input: E on a gear lever and on the `stop_clock` lever; cross the sinks by keyboard (confirm fatal traps or rescue by difficulty). F5 after `Port_Cine`, restart, F9: the autodoors are still open and the levers consumed.
- [ ] From a staged save beside `stop_clock`: pull it, hold Enter, walk into `exit_level`. hatter2 autosaves, and Continue works in a fresh process.

**Fidelity follow-ups:** the `Hatter1_Cinema1` machinery (fingers, conveyors, grabber, gib splines, whirler); lightstyles and electrocution effects; sprays and bubbles; full reflections; gong and key sounds; `st_boo1` spline fidelity; exact cameras; lip-sync.

**Risks:**
- The topology is inferred.
- The mirror renderer does not exist yet.
- Sink semantics (U1).
- The existing sequence breaks.
- `donna_hunt` absent today.
- Pit trigger overlaps.
- The watched and skipped machinery threads differ.
- Missing names are no-ops (`$rage_gear`, `$hair_splash_sound`, `$dunk_panel`, `$ClockRoom_Door`, `monkey1`/`2`/`4`, `pend1`-`4`, `gear3`, `biggear1`).
- The fast lift and crusher.
- Door toggles re-triggered by skips.

**Refs:** `maps/hatter1.scr` (50-55, 96-179, 434-472, 477-517, 519-568, 571-640, 642-834, 842-966); `maps/cinematics/hatter1_cinematics.scr` (34-49, 68-105, 114-199, 201-324, 326-340, 349-447, 450-676); entities #1-#14, #21, #23, #25-#27, #32-#48, #50, #53-#60, #65, #67, #68, #70-#72, #77-#82, #92, #93, #96-#100, #112, #113, #116, #117, #119-#129, #133, #134, #138, #144, #151-#153, #158, #168, #169, #199, #204, #267-#277, #289, #376, #378, #409-#411, #445, #446, #455-#458, #569, #577, #660, #683, #694, #751-#754, #800-#824, #892, #961-#964.

<a id="v25-hatter2"></a>
## 25 · hatter2 (About Face)

| Field | Value |
| --- | --- |
| Visit | `hatter2` (default `hatter2_start1`, #71 at 5664 -1088 0, yaw 90) · ROUTE 24 · next `jlair1$jlair1_start1` |
| Status | **BLOCKED.** The drawbridge `script_object` (#52) is the only floor over the moat, and `trigger_fall` #57 (x 4728..6600, y -1184..1280, z -192..-128, no difficulty flags) lies under both the moat and the arena. **Walking north from the start is fatal today.** The lift discs `end_plat1`/`2` and `testpend1` are **patch-only** (`Collider::model` is brush-only). The Eye Staff blade altar #11 is not a pickup. There is no exit |
| Size / milestone | L · M5 |
| Depends on | W1 (**drawbridge solid from load**, clock hands, closet doors, cage set), W21 (**patch collision for the lifts**), **B4 Mad Hatter**, E15 (adds), W6 (closet func_spawns and pushes), T7 (**the blade quest item, the ExitTest counter, the Watch pickup_thread**), T8 (`help_me` with difficulty variants), W3 (`end_doors1` targeted), W14 (`end_tele`), C1 and C2 (the Gryphon scene with the attach), C4 (`allow_cheshire(0)` from load; `hatter_cat`/`watch_cat`), C6 |
| Reservations | hit-ID 8,400,000 (the Hatter and the closet adds) · `hatter2/` · `hatter2.` · `--hatter2-*` · `hatter2-*` |

**Arrival** (main, `hatter2.scr:1239-1322`):
- `full_stats`, **`allow_cheshire(0)`**, `bosslevel(1)` (the boss HUD shows), `killdemons`;
- the sky pieces; the Cats hidden;
- the cage bars, knobs and chains bound; circles and beams non-solid;
- Setup (765-789): cameras `hatter2_path1`-`8`; the gear ring; the `testpend1` pendulum.
- **The Hatter stands at `tower_up` (5592 -1024 464), about 474 units from the start, within his vision of 500, with AI on.** Decide: faithful early fire, or hold him dormant until the cycle starts (default in How to read).

**Progression and gates:**
1. **Drawbridge.** Trigger #51 (a line at the arena entrance) runs `drawbridgeMove`: the bridge rolls up to 90 degrees over 1 s behind Alice, sealing the arena.
2. **Arena centre** (a clock-face floor; the hands and gear ring lie beneath the floor patches):
   - the blade altar `ambient_eyestaff_blade` #11 (5664 160 16; pickup_thread `ExitTest`: `exit_cnt` 2 to 1);
   - trigger #5 `hatter_cat` (`catz315`); when the line finishes, `ClockMinMove` (a quarter turn every 2 s) and `ClockHrMove` (a quarter every 24 s) start.
3. **Boss cycle, in 24 s stages:**
   - **S1:** gong, then `HatterDown`: AI off, shrink (about 0.5 s), warp to `tower_down` (5664 544 64) 2 s after the shrink starts, grow, AI on, attack the player, trigger `help_me1`.
   - **S2:** gong.
   - **S3:** gong, then `HatterUp` (to `tower_up`, AI off). 2 s later `ClockworkSpawn`: the closet doors open, and `clockwork_east`/`west` spawn and are launched by pushes #54/#175.
   - **S4:** gong; `hatter_splash` (area damage at the centre); after 3 s the clockspawns take 1000 damage.

   Health spawners: on Easy, four corner spawners; on Normal/Hard, only `help_me1` #334, refilled at each `HatterDown`.
4. **Hatter death.** At health 100, `START_DEATH` plays three `death_malfunction`, then `suicide`, then `death_end` (deathshrink). The killthread `EndPlats` clears the boss HUD, stops the clocks, and loops the lifts: `end_plat1` rises 392 and `end_plat2` sinks 392 over 2 s, then 4 s later they swap (an 8 s cycle), so each disc top alternates between z 8 and z 400.
5. **Watch and door.** Ride a disc to 400 and step north onto the ledge (top z 392). The Watch altar #13 (5664 1152 400; pickup_thread `ExitTest`) brings `exit_cnt` to 0: `watch_cat` appears (`cat022`) and `end_doors1` (#2/#3, targeted, wait -1) opens. `testpend1` swings just south of the lifts.
6. **Gryphon exit.** `end_tele` #9 (behind the doors) goes to `end_dest` (1248 1280 0), inside #179, which runs `Hatter2_Gryphon_Cinema1`:
   - `Open_Cage` (chains and cages over 5 s);
   - the Gryphon's flight (`gyphon_hover`, `gryphon_land`); lines `gry004`-`006`, `alcz3020`-`3021`;
   - **Alice attached at `tag_alice`** and riding;
   - the dive (`t137`), the breath attack (smashing `circle`/`smashme`), the flight along `t145`;
   - after 4 s, hints are re-enabled (`allow_cheshire`) and the script changes level to `jlair1$jlair1_start1`.

   The skip `HA1_End` goes straight to the level change (`Skipthread_HA1` kills a misnamed thread, so commit the transition once).

**Exit:** the script `map()` (`hatter2.scr:732`; skip :566) leads to `jlair1$jlair1_start1`. It requires the Hatter dead, the blade and Watch collected, and the doors open. `$h2_changelevel` is not in the data.

**Enemies:** the Mad Hatter (**gating**); `clockwork_east`/`west` func_spawns (2 per S3 stage, killed at S4).

**Boss B4: Mad Hatter** (`hatter` #63; `c_madhatter.tik`, `ai/c_madhatter.st`):
- 2600 hp, vision 500, pain threshold 75, mass 2000.
- Melee inside 150: cane (chance 0.4; 15 damage, knockback 300) or slap (15, knockback 300).
- Ranged beyond 150 (chance 0.4): a teacup saucer (`prj_teacup`, 600 u/s, 10) or syringe rockets (`prj_syringe`, 500 u/s, 20 poison); each may repeat (0.333).
- Runs beyond 400.
- The tower cycle above; the splash (research the Explosion entity's damage, U6).
- Death at health 100.
- Decide how a death while he is up at `tower_up` with AI off is handled (default in How to read).

**Cinematics:** `hatter_cat` (the commit starts the clocks); `watch_cat` (the commit opens the doors); `Hatter2_Gryphon_Cinema1` (the attach ride; watched and skipped both transition once; hints re-enabled).

**Movers:**
- the drawbridge (solid from load; rolls up behind Alice);
- the floor clock (hands and gear ring; below the floor patches);
- the clockwork closets (doors, func_spawn, launch pads that also push Alice if she stands in an open closet);
- the end lifts (patch-only discs; ride only after `EndPlats`);
- the end doors and teleport;
- `testpend1` (patch-only);
- the Gryphon cage set (scene only);
- the `help_me` chain.

**NPCs and dialogue:** Cheshire `hatter_cat` (`catz315`) and `watch_cat` (`cat022`); the Gryphon (nodamage, caged at 1248 352 208: `gry004`-`006`); Alice `alcz3020`-`3021`. `allow_cheshire(0)` from load until the Gryphon scene ends.

**Special mechanics:** a clock-driven boss cycle; the tea-splash area damage; two required quest pickups counted through pickup_thread; lifts that work only after the boss dies; a fatal moat and arena fall; hint suppression; the boss HUD from load; the scene exit with the Alice attach.

**Rewards:** **the Deadtime Watch** (#13, slot 9; `campaign.rs:159`); **the Eye Staff blade part** (#11, a quest flag); super essence #47. Both carry into jlair1.

**Entity classes:** `Enemies_MadHatter` (boss), `func_spawn` (closets; difficulty variants), `Enemies_ClockworkAutomaton`, `Emitter_HatterTeaSplash`, `script_object` (including patch-only models), `func_rotatingdoor` (targeted), `trigger_teleport` (gated), `trigger_fall`, `trigger_push`, `Item_WeaponPickup_DeadtimeWatch`, `ambient_eyestaff_blade`, `Characters_CheshireCat`, `Characters_Gryphon`, `func_smashablewall` (scene), `hatter_teapot_huge`.

**PLAYABLE MVP checklist:**
- [ ] Controller: the **drawbridge solid from load**; clock hands and gear ring; `testpend1`; closet doors; `end_plat1`/`2` (W21 patch hulls, or reviewed 112x112 disc boxes, oscillating only after `EndPlats`, with rider carry); `end_doors1` locked until ExitTest; the cage set.
- [ ] B4 boss with the tower cycle tied to the 24 s stages after the `hatter_cat` beat; registry targets; the HUD.
- [ ] E15 closet spawns with launch; the S4 splash and removal; the `help_me` chain with its difficulty variants.
- [ ] T7: the blade as a quest pickup (Stats field with serde default and validation) plus the ExitTest counter over the blade and #13; at 2, the `watch_cat` beat and the doors open; gate `end_tele` behind the doors.
- [ ] The Gryphon scene (C1/C2) with its once-only transition.
- [ ] Hint suppression while `allow_cheshire` is 0; `full_stats`, `killdemons` and the HUD from load.
- [ ] Saves (Hatter down mid-attack; up with adds; splash pending; lifts moving; one quest item collected; mid Gryphon flight), plus entrance restart.
- [ ] Launcher `tools/launchers/Launch-About-Face.cmd`; `docs/HATTER2.md`; `docs/LOADOUTS.md` note on the blade part.

**Route check (`--hatter2-route-check` and skip variant):**
1. North over the drawbridge; assert no fatal fall and the bridge raised afterwards.
2. The centre: assert the blade quest flag and the `hatter_cat` beat.
3. Fight loop (up to about 20,000 ticks): circle-strafe on the arena; attack while the Hatter is down or adds are live; leave the centre before S4 (read the controller's stage clock); take `help_me1` when Sanity is below 50.
4. Assert the Hatter is dead and `EndPlats` started.
5. Board a disc at floor level; `ride_to(400)`; walk to the Watch. Assert `copies(9) == 1`, `exit_cnt == 0`, and the doors open.
6. `end_tele` (teleports == 1); the Gryphon scene.
7. Assert `transition == ("jlair1", Some("jlair1_start1"))`, and that the next spawn is clear.

Contracts:
- the doors and teleport are inert while the Hatter lives or either item is missing;
- the lifts are static before the death and carry a rider between z 8 and 400 after it;
- the boss clock is deterministic and pause-safe;
- a live-boss snapshot rejects impossible states;
- watched and skipped exits are identical.

**Anode checks:**
- [ ] `--hatter2-render-check`: the drawbridge raising; the clock hands; the Hatter's shrink and grow at each tower; cane and slap; syringes and teacups; closets launching; the splash; the malfunction death; the lifts at both heights; `watch_cat`; the cage opening and the Gryphon flight with Alice riding.
- [ ] A real-input fight: HUD at load, health dropping, adds and their removal, lifts after the death, the Watch in the HUD. F5 mid-fight, kill, `--load quick`, F9: the stage clock, Hatter health and adds are restored.
- [ ] After both pickups: doors, teleport, skip the Gryphon scene. jlair1 loads and autosaves; Continue in a fresh process keeps the Watch and the blade flag.

**Fidelity follow-ups:** shrink/grow scale animation; the tea drop and splash particles; syringe and teacup visuals; cane attachment on death; the gong and chain sounds; exact Gryphon splines, riding pose and breath; the falling rings and debris; Hatter idle acting; lip-sync.

**Risks:**
- The patch-only lifts.
- The drawbridge must be solid from load.
- Death while up with AI off.
- Splash damage semantics.
- The Hatter can fire from the tower early.
- The new Stats quest field.
- The scene complexity with the misnamed skip killthread.

**Refs:** `maps/hatter2.scr` (12, 17-39, 496-575, 577-734, 736-763, 765-789, 791-980, 1024-1152, 1239-1322); `ai/c_madhatter.st`; `models/c_madhatter.tik`, `fx_hatterexp.tik`, `prj_syringe.tik`, `prj_teacup.tik`; entities #1-#13, #22, #46, #47, #49-#71, #133, #174-#222, #329-#334.

---

# Jabberwock lair and Wonderland Woods return (M6)

Shared M6 systems:
- the Gryphon attach ride (C2);
- `func_sinkobject` over lava (W10);
- the imp, Fire Snark, Magma, Phantasm and Jabberspawn families at scale (E6, E10-E12, E17), with activation and sleep radii for performance;
- the grounded Jabberwock survival boss (B5);
- the Eye Staff grant (T7) and the EyeBeam damage kind (T1);
- a smashable wall with a weapon filter (W8);
- heavy-weapon route firing (R2);
- the optional Blunderbuss policy (DG-5).

<a id="v26-jlair1"></a>
## 26 · jlair1 (Burning Curiosity)

| Field | Value |
| --- | --- |
| Visit | `jlair1$jlair1_start1` (-4336 -6768 -576, yaw 45; no entry thread) · ROUTE 25 · next `jlair2$jlair2_start1` |
| Status | **IMPLEMENTED AND ROUTE-VERIFIED.** Gryphon arrival, sinking lava platform, native encounters and launched imps, Caterpillar scene, triggered essences and the transition into active Jabberwock survival. Watched and skipped routes pass. See [JLAIR1.md](JLAIR1.md) for current behavior, persistence and evidence; the source audit and planning checklist below are retained for reference. |
| Size / milestone | M · M6 |
| Depends on | W10, W6 (func_spawns plus monster-only leap pads), C1, C2 (**Alice attached to `tag_alice` on a flying spline**), C3, E6, E10 (**43 imps**), E11, E12, E13, E17, R1 (lava-gap jumps, sink timing), the performance budget |
| Reservations | hit-ID 8,500,000 · `jlair1/` · `jlair1.` · `--jlair1-*` · `jlair1-*` |

**Arrival** (main: sky portal `skycamera01`, `setfarplane 5000`, `full_stats`, then `JLair1_Start`):
- Alice is attached to `gryphon_actor1` at `tag_alice` and flies `gryphon_path1` (`t8`..`t12`) for 6 s. Cameras: `gryphon_watch1` on `jlair1_path1`/`path3`; `caterpillar_watch1` on `jlair1_path2`.
- Fade; she stands at `alice_pos1` (-4608 -6656 -536) facing the Gryphon at `gryphon_posx1`; line `gry007`.
- The Gryphon takes off; `gryphon_actor2` leaves along `gryphon_leave1` (5 s).

The skip `J1_End` gives the same placement with both Gryphons removed.

**Progression and gates:**
1. **Gryphon arrival.**
2. **Lava caverns, south to middle.** The trigger chain: `imp-01` #113/#572, `mountainsidelocos` #11, imps 04/05 (#1, #94, #602), `jspawn-01` #114, `boojum-01` #93, `1stcave02` #78, `lavaimps` #43 (func_spawns #734/#737 with monster-only pads #736/#739), `t2` #388 (Magma), `imp-lah` #126, `imps11` #7, `1stcaveexit` #76, `impsrule` #48 (func_spawn #47, pad #50). **Lava channels must be jumped**; the first break is near (-3600, -1900, 592). The Jacks altar #15 is a duplicate (Will refill).
3. **Sink platform** #770 (*40; 239x235x96, top 1632; limit 128, speed 100) bridges a lava pool (surface about 1600) between `impsrule` and `imps09`. It is likely required.
4. **Upper caverns.** `imps09` #10, `imps12` #6, `jabbercorner` #9, `t3` #124, `imp10` #8, `2ndcaveboojum` #23 (a trigger carrying spawnflags 64; meaning unknown), `t4` #123, `impsterpimpster` #44, `t5` #121; `trigger_fall` #134.
5. **Caterpillar scene.** #26 (a wall at x -4504) runs `JLair1_Caterpillar_Cinema1`:
   - Alice at `alice_cater_pos1`; the Cat at `cat_cater_pos1` (fade-in); the Caterpillar revealed;
   - cameras `jlair1_path4`/`5`, `jdm1` (FOV 20 to 120 over 3 s), `jdm2`, `jdm4`, `jdm5`, `path6`, `jdm6`;
   - lines `cpl010`-`016`, `alcz4001`-`4007`, `catz318`.

   Skip `J1_CatEnd`. It repositions only; there is no gate.
6. **Exit.** `t7` #111 (Magma); `magmaman03` #84 (missing target, a no-op); `jabber-07` #80; `metaessences` #3 (func_spawns #4/#908); then #106.

**Exit:** `trigger_changelevel` #106 (-3520 3700 3088) leads to `jlair2$jlair2_start1`. No gate.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Fire Imp (35 hp; immune to lava and firesword) | 40 plus 3 func_spawns | 10 placed; 30 delayed across 15 groups (the two in `t157` are never triggered); leap pads | no |
| Fire Snark (lava swimmer, 100 hp) | 10 | Placed (5 more on Hard) | no |
| Jabberspawn variants (asleep variants wake) | 8 | Delayed | no |
| Boojum | 4 | Delayed | no |
| Phantasm / Magma | 2 / 2 | Delayed | no |

**Cinematics:**
- `JLair1_Start` with `J1_End`: the ride with the Alice puppet attached, the dismount, `gry007`, the departure. The commit: Alice at `alice_pos1`, both Gryphons gone.
- `JLair1_Caterpillar_Cinema1` with `J1_CatEnd`: FOV cuts, 15 beats, the reveal. The commit: Alice at `alice_cater_pos1`, the Cat and Caterpillar gone.

**Movers:** sink #770; the lava imp launchers (spawnflags 20, monster-only); the essence spawn #3.

**NPCs and dialogue:** the Gryphon `gry007`; the Caterpillar `cpl010`-`016`; Alice `alcz4001`-`4007`; the Cat `catz318`. No hint regions.

**Special mechanics:** the arrival ride attached to a flying Gryphon; lava caverns with channel jumps; imps leaping from the lava; the sinking platform; sleeping Jabberspawns; a FOV zoom; a fall pit.

**Rewards:** a Jacks duplicate (Will refill); 2 RageBoxes (#14, #95); the Looking Glass #19; large essences #117/#120 (#29 and #30 are Easy-only).

**Entity classes:** `info_player_start`, `trigger_once`, `trigger_changelevel`, `trigger_push` (monster-only), `trigger_fall`, `func_sinkobject`, `func_spawn`, `func_camera`, `info_splinepath`, `info_pathnode`, `script_object` (sky, excluded), `Characters_Gryphon` x2 (scene-owned flyers; never ground-settled), `Characters_CheshireCat`, `characters_caterpillar`, `Enemies_FireImp`/`FireSnark`/`Boojum`/`Jabberspawn*`/`Phantasmagoria`, `enemies_magmaman`, items.

**PLAYABLE MVP checklist:**
- [ ] Controller owning `gryphon_actor1`/`2`, `cat_actor1` and `caterpillar_actor1`, removed from placements.
- [ ] The arrival ride through the registry `entry_story`: the puppet attached to `tag_alice` on `gryphon_path1`, dismount, beat, departure; the skip commit.
- [ ] The Caterpillar scene.
- [ ] W10 platform #770 (a recovery rule from U1), with its position saved.
- [ ] Encounters: every listed group; the func_spawns and pads through W6. E10 must hold 43 active imps within the performance budget (sleep radii).
- [ ] Saves: scene beats, sink position; rearm #26 if it was consumed as "Pending". Cases: `jlair1-ride`, `jlair1-sinking`, `jlair1-caterpillar`.
- [ ] Launcher `tools/launchers/Launch-Burning-Curiosity.cmd`; `docs/JLAIR1.md`.

**Route check (`--jlair1-route-check` and skip variant):**
1. The ride, watched or skipped; assert feet at `alice_pos1`.
2. The trigger chain in segments with grounded floor goals under each trigger footprint (trigger origins are volume centres). Include reviewed run-jumps over the lava channels.
3. Cross #770 (at -6968 698, top 1632) with a timed run and jump.
4. The upper chain to #26; assert `alice_cater_pos1`.
5. On to the #106 floor.

Assert `transition == ("jlair2", Some("jlair2_start1"))`; the Jacks pickup gives Will 100 without adding a copy; lava never kills; no recovery; the sink follows its rule.

**Anode checks:**
- [ ] `--jlair1-render-check`: ride frames with Alice attached; the landing dialogue; the departure; the Caterpillar scene with the `jdm1` zoom; the sink at rest and sunk.
- [ ] Real input: skip the ride, F5 and reload; the lava channels and the sink with real run and jump keys; F5 **while standing on the sinking platform**, restart (its position restores); the Caterpillar scene; the exit, then the jlair2 autosave in a fresh process.
- [ ] `--perf-sample-check` (R7) smoke run at the peak imp count; full sampling in Phase 4.

**Fidelity follow-ups:** Gryphon spline curvature and riding pose; FOV easing; imp leap arcs; Jabberspawn sleep and wake parity; exact sink recovery; fog density.

**Risks:**
- Sink semantics.
- Lava-gap reachability.
- The Gryphons are grounded by generic placement today.
- The attach-during-flight staging is new.
- 40+ imps affect route tuning and performance.

**Refs:** `maps/jlair1.scr:5-29`; `maps/cinematics/jlair1_cinematics.scr` (1-43, 45-260, 262-349); entities #1, #3-#11, #14, #15, #19, #23, #26, #29, #30, #43, #44, #47-#50, #57-#59, #76, #78, #80, #84, #93-#96, #98, #99, #101, #102, #104-#106, #108, #111, #113, #114, #116, #117, #120, #121, #123, #124, #126, #134, #388, #428-#445, #465, #572, #602, #734-#739, #770, #908.

<a id="v27-jlair2"></a>
## 27 · jlair2 (Jabberwock's Lair)

| Field | Value |
| --- | --- |
| Visit | `jlair2$jlair2_start1` (-848 416 64; no entry thread) · ROUTE 26 · next `wforest$wforest_start2` |
| Status | **BLOCKED: no exit entity.** `end_trigger` #1 covers essentially the whole arena floor, including the spawn. In Rust it fires "Pending" on the first tick and is consumed, which cached visits must re-arm. The boss and its double `j` are statues; the Gryphon is grounded off the arena; the eye altar is not drawn |
| Size / milestone | L · M6 |
| Depends on | **B5**, E17 (waves), W6 (func_spawn `t2` under the arena plus the launch pads #16/#20/#23), T7 (**the eye altar and the `w_eyestaff` grant**), T8 (`help_me` every 8 s), T12, C1, C2 (the Gryphon dive), C3, C6, R1 |
| Reservations | hit-ID 8,600,000 · `jlair2/` · `jlair2.` · `--jlair2-*` · `jlair2-*` |

**Arrival** (main, `jlair2.scr:16-38`; `jlair2_cinematics.scr:228-272`):
- `jabber_actor1` is renamed `JLair2`, which selects the land-only AI variant;
- `end_trigger` is disabled;
- after the player spawns: `full_stats`, `bosslevel(1)`, `killdemons`, then `JLair2_Cinema1`;
- `sky_camera1`; health 99999 with AI off; `j` and `g` hidden;
- `eye_altar` non-solid with `altarhide(1)`; `pickup_eye_trigger` disabled.

**Progression and gates:**
1. **Intro** `JLair2_Cinema1`: a 3 s fade in; cameras `jlair_path1`-`5`; the Jabberwock walks `jabber_pos1` to `jabber_pos2` three times, waits 5 s, then walks to `jabber_pos3`; lines `jbwk001`-`004` and `alcz4008`-`4011`. At the end: AI on, `help_me1` spawned, `JLair2_Jabber_Fight` starts. Skip `JL1a_End`.
2. **90-second survival.** At 30 s, `t2` spawns three Jabberspawns under the arena (z -616, inside `trigger_fall` #15), launched to `t1`/`t3`/`t5` (z 264). At 60 s, the same again. At 90 s, `end_trigger` becomes triggerable if Alice's health is above 0. `help_me1` and `help_me2` alternate, each 8 s after a pickup.
3. **Death scene.** `end_trigger` #1 (x -1536..-168, y -704..1392, z 56..640) fires **the instant it becomes triggerable**. `JLair2_Dead_Jabber`:
   - `bosslevel(0)`; the spawns removed; the boss hidden;
   - the double `j` strikes; Alice is knocked down at `alice_eye`;
   - `g` dives along `p6`, `p7`, `p8`, `t9`, `t10` and strikes; `j` plays `eyeloss`/`pain3`;
   - the altar is revealed; `j` flies off (`t6`..`t8`);
   - `gry008`, with Alice at `alice_gryphon1` and the Gryphon at `gryphon_pos1` (`jlair_path8`);
   - the Gryphon leaves along `jabber_leave1`.

   The end state: Alice at `alice_dead_pos1` (-1056 416 96), `pickup_eye_trigger` enabled, the altar solid. Skip `JL1b_End`, same state.
4. **Eye pickup.** `pickup_eye_trigger` #4 (±14, over the solid ±10 altar #3) runs `pickup_eye`, which grants the complete Eye Staff (`w_eyestaff.tik`), waits 2 s, then changes level to `wforest$wforest_start2`.

**Exit:** the script `map()` in `pickup_eye` (`jlair2_cinematics.scr:41-47`) leads to `wforest$wforest_start2`, after the 90 s survival, the death scene and the pickup.

**Enemies:** the Jabberwock (**gating by survival**; invulnerable); Jabberspawn waves (3 at 30 s and 3 at 60 s; 200 hp; tail 10, claw 20; removed in the death scene).

**Boss B5: the grounded Jabberwock** (`c_jabberwock.tik`, `ai/c_jabberwock.st`, the `JLair2` branch):
- It never takes off; `LAND_IDLE` watches Alice.
- Inside 180, a 20% melee chance: `attack1d` (5 then 20, knockback 400) or a bite (15).
- Beyond 180, on each ready loop, a 1/3 ranged chance:
  - fire breath: `attack4a`, then `attack4b` looped 1-4 s with 7 `prj_jabberwock_breath` per loop (200 u/s, 5 damage), then `attack4c`;
  - the eye beam: `attack5a`/`5b`/`5c` for 1-3 s (damage is native, U6).
- Beyond 600: a jump attack, then a two-punch.
- Pain at 80 or more. `ignore_deadtime` (the Watch does not freeze it). Vision 1000. Health 99999, so it cannot be killed.

**Cinematics:**
- **Intro** with `JL1a_End`: the commit puts the boss at `jabber_pos3` with AI on, `help_me1` spawned, and the fight clock at 0.
- **Death scene** with `JL1b_End`: the commit puts Alice at `alice_dead_pos1`, removes the spawns, the boss and the Gryphons, makes the altar visible and solid, and enables the pickup.
- **`pickup_eye`**: the grant, latched, then the transition after 2 s.

**Movers:** the eye altar (`ambient_eyestaff_eye` plus the trigger; hide and show; solidity); `end_trigger` (arena-wide; enabled at 90 s); the wave spawners and push pads; the `help_me` chain.

**NPCs and dialogue:** the Jabberwock `jbwk001`-`004`; Alice `alcz4008`-`4011`; the Gryphon (`gryphon_actor1`) `gry008`. The double `j` (Jabberwock) and `g` (a Gryphon `script_model`) have no lines.

**Special mechanics:** a survival-timer boss; an arena-wide end trigger; waves spawned below and launched in; beams and breath; the essence 2-cycle; the altar hide and show; the Gryphon strike; a delayed transition after a grant.

**Rewards:** **the complete Jabberwock's Eye Staff** (`w_eyestaff.tik`, slot 7; REWARDS `jlair2`, `campaign.rs:160`); respawning essences.

**Entity classes:** `info_player_start`, `Enemies_Jabberwock` (boss and double), `script_model` (`g`), `Characters_Gryphon`, `trigger_once` (initially disabled), `ambient_eyestaff_eye`, `func_spawn`, `trigger_push`, `trigger_fall`, `info_splinepath`, `info_pathnode`, `func_camera`, `prj_jabberwock_breath`, steam emitters.

**PLAYABLE MVP checklist:**
- [ ] Controller owning `jabber_actor1`, `j`, `g`, `gryphon_actor1`, `eye_altar`, the `t2` spawners and the `help_me` spawners, excluded from placements.
- [ ] Initial facts: `end_trigger` and the pickup disabled; the altar hidden and non-solid; the refill; Dice demons dismissed.
- [ ] The intro, then the fight clock.
- [ ] B5: seeded chances; `ignore_deadtime`; infinite health; a registry target so hits and pain register; the eye-beam damage from research.
- [ ] The survival timeline: waves at 30 s and 60 s (spawn and launch, or a reviewed placement arc); `end_trigger` enabled at 90 s if Alice is alive. The clock is saved and pauses. Define the retry (recommendation and default: restart from the intro end).
- [ ] E17 wave AI, owned and saved; removed in the death scene.
- [ ] The death scene.
- [ ] The eye grant via `Stats::apply(Weapon(7))`, latched, then the transition. Update `campaign.rs:223-241` to assert the controller's grant.
- [ ] The `help_me` chain.
- [ ] Saves: intro beat, clock, waves and spawns, boss action and cues, projectiles, the death-scene beat, grant and exit latches. **Re-arm `end_trigger` and #4 in cached visits** (they were consumed at spawn in old builds). Cases: `jlair2-wave1`, `jlair2-t89`, `jlair2-death-scene`.
- [ ] Launcher `tools/launchers/Launch-Jabberwocks-Lair.cmd`; `docs/JLAIR2.md`.

**Route check (`--jlair2-route-check` and skip variant):**
1. The intro.
2. Survival loop: strafe outside 180 but inside 600 to avoid the jump attack; dodge; throw at the Jabberspawns. Assert 3 spawns at 30 s and 3 at 60 s, `end_trigger` disabled at 89.9 s, and the death scene starting on the 90 s tick with no movement needed.
3. The death scene, watched or skipped. Assert the spawns removed, the altar solid, the pickup enabled, Alice at `alice_dead_pos1`.
4. **Walk into** the altar until `pickup_eye` fires. Assert `copies(7) == 1`, then after 2 s `transition == ("wforest", Some("wforest_start2"))`.

`--jlair2-check` contracts: the boss never dies; Alice's death before 90 s never enables the trigger; pause; 30/60/144 Hz; a mid-wave restore; the old-save re-arm.

**Anode checks:**
- [ ] `--jlair2-render-check`: intro cameras; breath and eye beam; waves arriving through the pads; the Gryphon strike and `eyeloss`; the Gryphon dialogue; the altar reveal.
- [ ] Real input: survive; F5 at about 45 s with wave 1 alive, kill, `--load quick` (the timer, wave count and spawns restore); at 90 s the scene starts automatically; hold Enter; take the eye. The automatic `wforest_start2` load and autosave, with the Eye Staff owned.
- [ ] Negative: die before 90 s and retry; the trigger stays off until the timer completes.

**Fidelity follow-ups:** eye-beam visuals and tick damage; breath and wing emitters; the Gryphon dive choreography; wave launch arcs; pain variety.

**Risks:** the exit exists only through the controller; unknown eye-beam damage; mis-rendered statues today; the consumed `end_trigger` in caches; push physics for actors; retry and recovery must not strand Alice below the arena.

**Refs:** `maps/jlair2.scr:16-38`; `maps/cinematics/jlair2_cinematics.scr` (4-14, 41-47, 49-202, 204-272, 274-382); `ai/c_jabberwock.st` (1-33, 291-529); `models/c_jabberwock.tik`, `prj_jabberwock_breath.tik`; entities #1-#4, #6, #7, #9, #11-#13, #15-#25, #40, #45, #46, #49-#53, #66-#81.

<a id="v28-wforest-return"></a>
## 28 · wforest return (Caterpillar's Plot, return)

| Field | Value |
| --- | --- |
| Visit | `wforest$wforest_start2` (5264 208 288, yaw 90; thread `Setup_LastPass`) · ROUTE 27 · visit key `wforest$return` · next `hedge1$hedge1_start1` |
| Status | **BLOCKED and BYPASS.** Because the chesswall and cavegate are open, the return player can walk back to wchess1 #126. The Humpty secret door is absent (the Blunderbuss is free). `eyestaff_wall` is invisible, non-solid and undamageable (there is no `func_smashablewall` support; Staff hits are `DamageKind::Other`). `Hedge_Maze_Entrance` #83 is pending. Anode input has confirmed the `wforest_start2` entry and its inventory (`docs/VALIDATION.md` ~238) |
| Size / milestone | L · M6 |
| Depends on | W1 (the last-pass table), W8 (**`eyestaff_wall` with its damage filter**), T1 (the EyeBeam kind), R2 (**Eye Staff held fire in the route**), T7 (the `blunder_cat` pickup_thread; the Eye Staff re-grant), T9 and DG-5, W22b (#126 off on the return; #83 gated on the wall), T8 (`get_me1`/`2`), C1, E9 |
| Reservations | hit-ID 8,700,000 (the same `wforest` controller with the return-visit base) · `wforest-return/` · `wforest-return.` · `--wforest-return-*` · `wforest-return-*` |

**Arrival** (`Setup_LastPass`, `wforest.scr:96-160`):
- `humpty_counter = 1`;
- `eyestaff_wall` set to ignore deadtime and hidden (**still solid**);
- `fake_humpty_wall1` hidden (still solid: an invisible slab);
- `secretdoorbutton` shown (solid); `secretdoor_trigger` enabled;
- Humpty shown and solid, looping `Humpty_Loop`;
- `broken_wall1` shown, `broken_wall2` hidden;
- `first_eyestaff` removed; `cat_chess_trigger` off; the Cats removed; the Caterpillar hidden; the scene volumes, `eyestaff_trigger` and `wall_hide_trigger` off;
- the Eye Staff is granted (`w_eyestaff`);
- `blunder_cat` hidden;
- **the chesswall closed, solid and shown; the cavegate solid and shown**;
- `wall_clip` removed; `alice_destory_wall_trigger` enabled; `block_trigger` non-solid;
- then `full_stats`, and `get_me1` triggered (a 2-cycle, 10 s).

**Progression and gates:**
1. **Arrival.**
2. **Optional Blunderbuss.** `secretdoor_trigger` #70 (3650 1094 424, at the button beside Humpty; floor about 368) runs `Open_Humpty_Door`: the button slides 24 units west over 2 s, then the secret door #72 rises 96 over 4 s. The Blunderbuss altar #73 (2814 574 -64) has pickup_thread `blunder_cat`: the Cat fades in, says `cat016`, fades out, and is removed.
3. **Hedge approach.** `hedgeguards` #95. `wall_show_trigger` #67 shows the wall; `cat_wall_trigger` #62 does nothing. `alice_destory_wall_trigger` #25 (a wall at 6444 4896 716) runs `Alice_Destroy_Wall` (unskippable in the data): Alice at `alice_wall_posx1` looking at `alicelookwall` #10; `alice_watch1` #121 on `wforest_jdm2`; line `gry006a` on the fake player; the Eye Staff is granted again; `wall_clip` removed.
4. **Break the Eye Staff wall.** `func_smashablewall` `eyestaff_wall` #82 (6496 5424 1004; 1152x96x808; health 100, spawnflags 15, 15 rubble pieces, killthread `wallkilled`). At 0 health, `wallkilled`:
   - first disables #25, so **breaking the wall first skips the Gryphon scene**;
   - Alice takes no damage for about 1.5 s;
   - `splodeme`/`splodeme2`;
   - `debris1`-`7` fly and are removed;
   - `hedge_door1` turns to -65 and `hedge_door2` to +65 over 4 s.

   **The damage filter (spawnflags 15) needs research (U1); the likely answer is Eye Staff only.**
5. **Enter the maze.** `trigger_once` #83 (inside the wall volume) runs `Hedge_Maze_Entrance`, which changes level to `hedge1$hedge1_start1`.

**Exits:**
- #83, then the script `map()`, leading to `hedge1$hedge1_start1`, after the wall is destroyed.
- #126 to wchess1 **must be unreachable and gated off** on the return visit.

**Enemies:** the same placements as the first visit. The reachable subset lies east of the closed chesswall and cavegate: `hedgeguards` (Heart x2, Diamond), Club and Spade guards, fire imps, `magmaman01`/`02`, and the `tunnelgroup` Phantasm.

**Cinematics:**
- `Alice_Destroy_Wall`: fades plus the `gry006a` beat. The commit: the Eye Staff owned, `wall_clip` removed. It must not replay after `wallkilled`.
- `blunder_cat`: an optional Cat beat.
- `wallkilled`: debris, splodes and doors. The committed state survives saves.
- `Hedge_Maze_Entrance`: the transition.
- `Humpty_Loop`: the ambient animation loop (`idle_hand`, `ash`, `poke`, `smoke`, `cigar`).

**Movers:** the secret button and door (#71, #72; sounds #69/#614); the Eye Staff wall (#82) with debris #3-#9, splodes #2/#734 and hedge doors #84/#85; the return-closed routes (chesswall x4, cavegate x3); the respawning essence (`get_me1` #38, `get_me2` #698).

**NPCs and dialogue:** Humpty (animation loop only); Cheshire `blunder_cat` (`cat016`); `gry006a` played at Alice (relabel the speaker; `story.rs:750-752` only relabels Cat voices).

**Special mechanics:** a return-only setup table; a secret button opening a remote door to an optional altar; a breakable wall with a weapon filter; a transition from a script thread; the hidden-but-solid fake slab.

**Rewards:** the optional **Blunderbuss** #73 (slot 8; REWARDS `wforest_start2`, `campaign.rs:161`; DG-5); the Eye Staff re-grant (already owned); respawning essences.

**Entity classes:** `info_player_start` (thread), `Characters_HumptyDumpty`, `script_object`, `trigger_once` (#70, #25, #83), `trigger_multiple` (`wall_show`), `func_smashablewall`, `SFX_WallSplode`, `sound_speaker`, `Item_WeaponPickup_Blunderbuss` (pickup_thread), `Characters_CheshireCat` (`blunder_cat`), `func_spawn` (essence), `trigger_changelevel` (gated off), `func_camera`, markers.

**PLAYABLE MVP checklist:**
- [ ] The `wforest` controller with `returning = true`: the `Setup_LastPass` table, the Humpty loop, the Eye Staff grant, the refill, `get_me1`.
- [ ] Gate #126 off on the return (plus the closed colliders).
- [ ] The secret route: #70 moves the button and door with sounds and an opening collider; the pickup_thread for #73 plays `blunder_cat`.
- [ ] The `Alice_Destroy_Wall` scene.
- [ ] W8: `eyestaff_wall` as a shot target with health 100 and the researched filter. T1 staff damage kind (the `heavy.rs` beam and comet are tagged `Other` today). On death: disable #25, splodes, debris, wall removal, door rotation, about 1.5 s without damage.
- [ ] #83 transitions to `hedge1$hedge1_start1`, gated on returning and the wall destroyed.
- [ ] R2: Eye Staff held-primary firing through the shared heavy state.
- [ ] Saves under `wforest$return`: button, door, Blunderbuss scene, wall health and destroyed flag, debris and door clocks, essence cycle. Rearm #83/#25/#70 if they were consumed as "Pending"; move obstructed positions to the entrance. Decide DG-5.
- [ ] Launcher `tools/launchers/Launch-Caterpillars-Plot-Return.cmd`; the return section of `docs/WFOREST.md`.

**Route check (`--wforest-return-route-check`):**
- Enter with the loadout `[1,1,1,1,1,1,3,1,0,1]` (`campaign.rs:288-289`) and assert the return setup applied.
- **Variant A:** walk to the floor beside Humpty (about 3650 1094 368) and into #70; assert the button and door move. Walk into the altar at (2814, 574, -64); assert `copies(8) == 1` and the `blunder_cat` beat.
- **Both variants:** navigate to `alice_wall_posx1` (6464 4896 592, inside #25) and let the scene play. Select slot 7 and hold primary on `eyestaff_wall` until its health reaches 0. Assert the debris removed, the wall non-solid, the doors rotated and #25 disabled. Navigate to `alice_hedge1` (6504 5424 784). Assert `transition == ("hedge1", Some("hedge1_start1"))`.

Contracts:
- #126 is unreachable and inert;
- #83 is inert while the wall stands;
- the Blade and Cards do not damage the wall (if research confirms the filter);
- destroying the wall first suppresses the scene.

**Anode checks:**
- [ ] `--wforest-return-render-check`: Humpty's loop; the button and door; the Blunderbuss Cat; the destroy-wall scene; the Eye Staff beam on the wall; splodes and debris; the open hedge doors.
- [ ] Real input: press Humpty's button, take the Blunderbuss; F5, restart (kept, door open). At the hedge, select the Eye Staff and hold primary with the real mouse; the wall breaks. F5, restart (the wall stays broken). Enter the maze; the hedge1 autosave.
- [ ] Negative: the Blade and Cards on the wall (no damage if the filter is confirmed); the way back to wchess1 is blocked.

**Fidelity follow-ups:** Humpty loop timing; wall-splode effects and the 15 rubble pieces; the positional Gryphon voice; `ignore_deadtime` on the wall under the Watch; Cat fade timing.

**Risks:** the unknown filter semantics; the harness cannot fire heavy weapons yet (R2); the Blunderbuss fill-in policy (DG-5); the return player can walk back to wchess1 today; the Staff/Blunderbuss code changed after the survey (that session has finished; re-check it); consumed triggers in old return caches.

**Refs:** `maps/wforest.scr` (71-89, 96-160); `maps/cinematics/wforest_cinematics.scr` (74-149, 151-212, 383-433, 481-500); entities #1-#10, #17, #20, #25-#28, #35, #38-#41, #62, #67-#73, #80, #82-#85, #95, #96, #113, #114, #121, #127, #506, #614, #698, #709, #734.

---

# Hedges and Towers (M7)

This is the most data-driven stretch once the toolkit exists. Shared M7 systems:
- sliding `func_door` (W2);
- trigger respond masks, wait-based re-fire and actor occupancy (W4);
- generic levers (W7), sink objects (W10), static fulcrums (W11), moving liquid (W13), checkpoint teleports (W14);
- hedge3 skies 4-7 (W20); patch hulls (W21);
- the insane-child follower on a pathnode graph (E18);
- the card-guard, Jabberspawn, Phantasm, Boojum, Snark and Clockwork families;
- the Cheshire tower intro template (C4);
- the `lead`, `wait_until` and `ride` route primitives (R1) and the leg planner (R4);
- sealed precache rooms (exclude them from Encounters and NPC drawing): hedge1 #1-#7, tower1 #410, hedge2 #1 and #746-#752.

<a id="v29-hedge1"></a>
## 29 · hedge1 (Majestic Maze)

| Field | Value |
| --- | --- |
| Visit | `hedge1$hedge1_start1` (the only start, #43 at 4806 -100 2056, yaw 90) · ROUTE 28 · next `tower1$tower1_start1` |
| Status | **RESTORED.** The existing child cast drives the plate and solid sliding leaves. The chase child uses a bounded saved trail around corners, and only its held plate contact unlocks physical exit #29. The inactive `SeekCine` is not run. See [HEDGE1.md](HEDGE1.md) for traversal evidence and remaining fidelity limits |
| Size / milestone | L · M7 |
| Depends on | W2 (the plate door #31 and leaves #32/#33), W4 (**bits 4/8 on `trigger_multiple`**; **re-fire every wait while occupied, with actors as occupants**), **E18** (follower and Hold), F5 navigation (a pathnode graph over 141 nodes, or a breadcrumb trail), E9, E12, E13, E17, W5, W6 (hedge-top pads), R1 (`lead`), R4 |
| Reservations | hit-ID 8,800,000 · `hedge1/` · `hedge1.` · `--hedge1-*` · `hedge1-*` |

**Arrival:** from the wforest return's `Hedge_Maze_Entrance` `map()`. Level start:
- `sky_camera1` active (the skies 1-3 switches already work, `sky_sequence.rs:103`);
- `seek_kid` AI off, playing `idle01`;
- `seek_kid_chase` AI off, **hidden and non-solid**;
- the sky puffballs; music `hedge1.mus`.

**Progression and gates:**
1. **Cross the maze** from (4806, -100) north-west to the exit corridor at y about 4288. Ambush volumes (`trigger_multiple`, wait 300/600) spawn func_spawn groups: `spawnfromgate` #12, `cornertrap` #14, `jabcorner` #13, `underbridge` #598, `cardambush` #563, `boojumtrap` #17, `jabbertrap` #16, `slopeambush` #581, `gateguards` #579, `gateboojum` #15, and `tunneljab` #11 (once). The targets of #609, #10 and #580 do not exist. Hedge-top spawns sit beside player-excluded pads (spawnflags 4).
2. **`SeekEnd`.** #25 (x 4288-4352) sends `seek_kid` (Muzzle) to `seek_end` (3136 4096 1944), the centre of the pressure plate.
   - Its body fires the plate `trigger_multiple` #30 (56x56x8; **spawnflags 8 means players and monsters**; wait 0.2; target `seek_exit`).
   - The plate door #31 sinks 4 units. The leaves #32 (+Y) and #33 (-Y) slide about 120 units at 400 u/s and return 0.3 s after the last fire.
   - The Muzzle kid also fires `SeekHold` #21 (harmless; it only renames the still-hidden chase kid).
3. **Courtyard ambush.** `gategoons` #18 (14 card guards across the maze, 5 Hearts not on Easy); `t32` #19 (2 Jabberspawns next to `t30`); #10 targets a missing `t62`.
4. **`SeekStart`.** #527 sends `seek_kid` to `seek_tele_path` and removes it. **The plate empties and the gates close.** `seek_kid_chase` (at 5102 1592 2414) becomes visible and solid, with AI on.
5. **Find the kid.** Optional shortcut: `seek_tele` #28 goes to `t30` (4584 2632 2056) mid-maze (the #530 sky reset). The monster response of teleport bit 8 is unverified (U4).

   Chase AI (`ai/c_insanechild_chase.st`; effective health 10000; untargetable; vision 500; fov 225): idle and twitch until Alice is visible within 300; then approach, walking inside 150 and running between 150 and 300, stopping at 60. It goes idle beyond 300, without line of sight, or when it cannot reach her.
6. **Lead the kid onto the plate** (about 3.2k units straight line). When its body enters `SeekHold` #21 (spawnflags 12, monsters only; wait 5), it is renamed `Hold` for 5 s, refreshed while it touches. **HOLD_POSITION is entered only from the approach states**, and it exits only when not named `Hold` and without an enemy, so in practice it holds. Its body keeps #30 firing, so the gates stay open.
7. **Exit** through the double gate to #29.

**Exit:** `trigger_changelevel` #29 (2584 4288 2052) leads to `tower1$tower1_start1`. It requires the `seek_exit` leaves open, which means the kid holding the plate. The `SeekCine` `map()` at `hedge1.scr:151` sits **inside a block comment (131-153) and is dead**.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| CardGuard-Spade (160) | 22 | func_spawn groups | no |
| CardGuard-Heart (200) | 13 | Groups | no |
| Jabberspawn variants | 11 | 2 placed (#8, #548); 9 func_spawns (hedge-top leaps) | no |
| Boojum | 6 | `gateboojum`, `boojumtrap` | no |
| Phantasmagoria | 3 | `boojumtrap` (spawnflags 256) | no |
| Precache room #1-#7 | 7 | Never; exclude | - |
| Insane-child follower | 1 | After `SeekStart` | **yes** |

**Cinematics:** none active. `SeekCine` is dead. The kid's scripted runs are gameplay staging with no camera takeover, so there is no skip.

**Movers:** the seek exit gate and plate (W2: plate #31 angle -2, lip 4, spawnflags 132; the leaves spawnflags 128, speed 400, wait 0.3); the hide-and-seek escort; the seek portal; the sky puffballs (fidelity).

**NPCs and dialogue:** no voiced scene; the `catz501` hint region (#23) works.

**Special mechanics:** occupancy triggers (respond mask plus wait re-fire); a plate held by an actor; auto-closing gates; a friendly follower escort through a maze; a teleport portal; hedge-top ambush pads; portal skies.

**Rewards:** RageBox (3528 2286 2434); loot.

**Entity classes:** `func_door` (TARGETED, DONT_LINK, angle ±Y and -2, lip, speed, wait), `trigger_multiple` (mask bits 4/8, re-fire), `trigger_once`, `trigger_teleport` plus `func_teleportdest`, `trigger_changelevel`, `trigger_push` (spawnflags 4, monster pads), `func_spawn`, `Characters_InsaneChild_Muzzle`, `Characters_InsaneChild_CHASE`, `info_pathnode`, `Enemies_CardGuard-Spade`/`Heart`, `Enemies_Jabberspawn*`, `Enemies_Phantasmagoria`, `Enemies_Boojum`, `Item_RageBox`, `trigger_catmessage`, sky puffball objects.

**PLAYABLE MVP checklist:**
- [x] Map-owned W2 plate and sliding leaves, saved fractions and shared render/collision poses; blocking without crushing Alice.
- [x] Map-owned W4 occupancy for #21/#30: monsters-only Hold and player/actor plate contact, refreshed at the authored wait intervals. Other maps' trigger semantics are unchanged.
- [x] Saved child handoff, following, holding, occupancy, gate travel and exit retry; consumes all three Seek threads.
- [x] E18 actors:
  - hide the chase kid until `SeekStart`;
  - run `seek_kid` along its legs, then remove it;
  - the follower: sight via `world.sweep`, acquire within 300, stop at 60, walk and run speeds, pathnode or breadcrumb navigation;
  - Hold from `#21` occupancy, entered only from the approach states;
  - untargetable and friendly; saved.

  Only the chase child's own occupancy latches Hold; unrelated enemies cannot strand it elsewhere.
- [x] The native cast retains enemy and ambush ownership. Precache actors #1-#7 are hidden/inactive without changing live hit/loot indices; older precache identities migrate to inactive placements. No duplicate controller-owned combat cast.
- [x] Real Store writes and fresh-process reads for following, holding, moving leaves, demonstration and handoff, with identical continuation; pre-controller identity migration and solid-leaf entrance recovery fixture.
- [x] Launcher `tools/launchers/Launch-Majestic-Maze.cmd`; `docs/HEDGE1.md`.

**Route check (`--hedge1-route-check`):**
1. A leg planner over the pathnodes (legs of at most about 600 units).
2. Assert `SeekEnd` puts the kid on the plate with the gates open, and `SeekStart` removes it, activates the chase kid and closes the gates.
3. **Negative:** Alice alone on #30 sprinting for #29 is blocked.
4. `portal()` into `seek_tele` (teleports + 1).
5. `lead(kid, legs)`: walk a leg, wait until the kid is within 150 with line of sight, repeat to the plate. Assert Hold and the gates open.
6. Walk through.

Assert `transition == ("tower1", Some("tower1_start1"))`, teleports == 1, alive. Mid-escort serialization continues identically at 30/60/144 Hz.

**Anode checks:**
- [x] Full default native input route: child demonstration, handoff and gate closure, `seek_tele`, return escort, Hold, open gates and strict tower1 departure. Normal difficulty; 25,580 ticks; 48.196 Sanity and 98.400 Will retained at arrival; no health assistance or recovery.
- [ ] Escort persistence: lead the kid for two legs, F5, kill, `--load quick`; diff `quick.json` with the live state (kid pose and phase, gates, enemies, Stats); finish on the plate; the tower1 autosave.
- [ ] `--hedge1-render-check`: the gates closed and open, the plate pressed, the kid hidden before `SeekStart`, the chase kid following, skies 1-3.

**Fidelity follow-ups:** respawns after 300/600 s (the MVP activates each group once); Jabberspawn hedge-top leaps; the puffball loops; child clips and sounds; gate and plate sounds; the Watch's interaction with the gate timers.

**Risks:**
- Follower navigation around hedge corners.
- **A monster touching #21 renames the kid to Hold while it is elsewhere, which can strand it off the plate.** Replicate and test that case, or honour Hold only for the kid's own occupancy (a documented deviation).
- The new trigger semantics must not change other maps.
- A Watch time stop could freeze the gates open.
- A long route (7-9k units) with the `gategoons` ambush.
- Save compatibility (the door `Vec`, the encounters signature).

**Refs:** `maps/hedge1.scr` (main, `change_to_sky1`-`3`, `SeekHold`, `SeekEnd`, `SeekStart`, `Quiver_Puff_Ball_Thread`, `Puff_Ball_Thread`, the dead `SeekCine` 131-153); `ai/c_insanechild_chase.st`; `models/c_insanechild_chase.tik`; entities #1-#43, #468-#504, #519, #527, #530, #535-#613.

<a id="v30-tower1"></a>
## 30 · tower1 (Airborne Terror)

| Field | Value |
| --- | --- |
| Visit | `tower1$tower1_start1` (#27 at 224 1344 1796, yaw 0; thread `Tower1_Start`) · ROUTE 29 · next `hedge2$hedge2_start1` |
| Status | **IMPLEMENTED; full Normal watched/skipped routes verified.** Cheshire introduction, four animated and physical face gusts, occupied six-second repeats and ten native Boojum activations. Exit #22 loads Hedge2 alive with exact carried resources. See [TOWER1.md](TOWER1.md). |
| Size / milestone | M · M7 |
| Depends on | A face state machine toggling the shared enable flags (`traversal.rs:218-225` becomes the initial state only), W4 (wait-6 re-fire), controller-drawn TAN props, C4 (the Tower intro template), E13 (10 Boojum func_spawns), W6 (pads #8/#15/#354 for spawned actors), R1 |
| Reservations | hit-ID 8,900,000 · `tower1/` · `tower1.` · `--tower1-*` · `tower1-*` |

**Arrival (`Tower1_Start`):**
1. Alice frozen, AI off.
2. A 2 s fade from black; `alice_watch1` #2 on `cams/tower1_p1.cam`.
3. `cat_actor1` #3 is shown at 0.5 s, waits 4 s, fades in over 2 s with the appear sound, and plays `Cat1_Dialog` (`catz504`; clips `sit_idle1`, `sit_talk3`, `sit_smile_open`), with head-watch on the fake player.
4. It fades out with the disappear sound; a short white fade; control and AI return; the Cat is removed 2 s later.

The skip `Skipthread_T1Start`/`T1Start_End` gives the same outcome. Also: the faces disabled; the missing `face1_blow` (a no-op); music `tower1.mus`; the Cat hidden at `cat_pos1` #368.

**Progression and gates:**
1. **The Cheshire intro.**
2. **Vent and updraft ascent.** 24 vents (64x64 `trigger_hurt` at the default damage, with a 100-unit upward push) under updrafts (speed -1). Kill volumes: #18 (the whole base above the lava floor, 10000) and the pits #17/#333/#334 under the face gauntlet.
3. **Blow faces.** Entering `Face1`-`4Thread` (#1, #406, #407, #409; wait 6): face N closes (0.35 s), holds (0.2 s), **enables `faceN_push` for 2 s** (face1 pushes -Y, faces 2 and 3 +X, face4 -X; default speed 1000), then opens (0.4 s). The trigger re-fires every 6 s while Alice stays inside; the Face2/Face3 zones overlap. Gusts blow Alice into the kill pits.
4. **Boojum ambushes.** Ten trigger_once/func_spawn pairs (`t2` over pad #15; `t11` over #354; `t13` pushed down by #8; `t9`/`t10` adjacent).
5. **Exit** #22 (x 936-952, y 1408-1664, z 2488-2624) from the last updraft tops.

**Exit:** `trigger_changelevel` #22 leads to `hedge2$hedge2_start1`. None.

**Enemies:** Boojums (10 func_spawns, #14 ... #401); precache #410 excluded.

**Cinematic:** `Tower1_Start`, the Tower intro template (C4). The commit: `intro_done`, the Cat removed, AI on, Alice at `tower1_start1`. Old saves count the intro as done.

**Movers:** the blow faces (`tower_face01.tik` TAN: `idle`/`close`/`hold`/`open`, where the hold clip toggles smoke) plus `faceN_push` plus `FaceNThread`; the vent and updraft columns (existing); launch pad #12 (unflagged; angle 315, speed 800; confirm whether it is a route or a hazard).

**NPCs and dialogue:** Cheshire `catz504` (`dialog/tower1.tlk`); the unnamed hint #20.

**Special mechanics:** updraft columns with hurting vents; timed face gusts; kill volumes over lava; Boojum knockback; the Cheshire intro.

**Rewards:** 3 medium essences.

**Entity classes:** `tower_blowface`, `trigger_push` (face toggled; vents; monster pads), `trigger_accelerate`, `trigger_hurt`, `trigger_multiple`, `trigger_once`, `func_spawn`, `Characters_CheshireCat`, `func_camera`, `info_player_start` (thread), `trigger_changelevel`, items, `trigger_catmessage`.

**PLAYABLE MVP checklist:**
- [x] Four saved face cycles gate player and native actor pushes, including six-second occupied re-fire.
- [x] Controller-drawn TAN face clips and attached smoke; deferred props are not duplicated.
- [x] Authored Tower introduction, hidden Cat, speech, camera, fades and shared watch/skip handoff.
- [x] Ten native Boojum activations, damage/death and all-difficulty checks; hidden precache #410; actor pads.
- [x] Full fresh Normal watched/skipped ascent into Hedge2, with no route teleports or combat weakening.
- [x] Saved clocks and old-save continuation; four native Store fixtures including early fade and ending.
- [x] `tools/launchers/Launch-Airborne-Terror.cmd`; [TOWER1.md](TOWER1.md).

**Route check (`--tower1-route-check` and skip variant):** watched and skipped intros give identical state. Climb column by column with an updraft helper. At each face zone, read the phase and cross in the idle window, or trigger it and wait out the 2.55 s. Assert no contact with #17/#18/#333/#334. Assert `transition == ("hedge2", Some("hedge2_start1"))`. Serialize mid-gust and continue identically.

**Anode checks:**
- [ ] The intro watched once (camera start, the Cat visible, the fade) and skipped once with Enter held. F5 mid-intro, restart: it resumes at the saved clock, and never replays once finished.
- [ ] The face gauntlet with real input: capture close, hold, blow and open; get blown into a pit once (fatal, then Enter retry); a clean timed pass. F5 during a gust, restart: the face phase and push state match.
- [ ] A Boojum knockback on a column; the exit, and `auto.json` holds `hedge2$hedge2_start1`.

**Fidelity follow-ups:** sub-frame acting, audible comparison, native actor post-pad velocity decay, vent steam synchronization and the original hurt cadence. Supplied Cat clip lengths and face smoke attachment are implemented.

**Risks:** gusts of 1000 plus knockback make the route flaky (the safe window is about 3.45 s per 6 s); updraft tuning (-0.25 g); the vent damage accumulates while waiting; a double draw of the face props; pad #12's role.

**Refs:** `maps/tower1.scr` (main, `Tower1_Start`, `Cat1_Dialog`, `T1Start_End`, `Skipthread_T1Start`, `Tower1_Cinematics_Init`, `Face1Thread`-`Face4Thread`); `cams/tower1_p1.cam`; entities #1-#3, #8, #10, #12, #14-#37, #59-#63, #83, #103-#166, #333, #334, #341-#356, #368, #400-#410.

<a id="v31-hedge2"></a>
## 31 · hedge2 (Mystifying Madness)

| Field | Value |
| --- | --- |
| Visit | `hedge2$hedge2_start1` (the only start, #70 at -672 -1592 1608, yaw 90) · ROUTE 30 · next `tower2$tower2_start1` |
| Status | **BYPASS.** `water_door` and `end_doors` (`func_door`) are invisible and non-solid, so exit #52 is reachable without the lever and the underwater pipe is always open. Both levers do nothing and are invisible (`lever.tik` is skipped by decorations). The trapdoor and robodoors are missing. 55 func_spawns are absent; the placed enemies are inert |
| Size / milestone | L · M7 |
| Depends on | W7 (**levers #507 and #450**), W5 (the relay `water_door_relays` #576/#577 to the robot func_spawns), W2 (`water_door` #402; `end_doors` #53/#54), W1 (trapdoor #11: +184 at entry, -184 after `OpenRobodoors`; robodoors +144), C1 (2 cutaways on `cam1`), E6 (Snarks), E15 (robots), E9, E12, E13, E17, T6 (the idempotent shell grant), R1 (`swim_to`, `use_at`) |
| Reservations | hit-ID 9,000,000 · `hedge2/` · `hedge2.` · `--hedge2-*` · `hedge2-*` |

**Arrival:** from tower1 #22. Level start: the turtle shell is granted (Rust already has `turtle_air`); the **trapdoor #11 rises 184**; cams `hedge2_watergate` (unused), `hedge2_endgatecam`, `hedge2_tunnelcam`; music `hedge2.mus`.

**Progression and gates** (the critical path must be proven by probes, U9):
1. **Arrive** at (-672, -1592) facing +Y; hint `catz502` #51.
2. **Upper maze** (z about 1840-2200). Ambushes: `trap001`-`006` (wait 300) and `traponce`, `upperthugs`, `clearing`, `nearend`, `welltrap` (once). Pickups: the Jacks altar (1888 1318 1822) and the Looking Glass (223 1381 2171). The unflagged hedge-top pushes #29/#621 also push Alice.
3. **Well, robot room and lever #507.**
   - The well water column (x -1280..-1056, z 680..2072; `welltrap` #18 at the top) leads down to a flooded chamber.
   - The pipe north is blocked by `water_door` #402 (TARGETED; rises 168 over 2 s and stays open).
   - The robot-room corridor (entered under the raised trapdoor at x about -76) holds **lever #507** (-1169 1136 960; count 1). The `t44`/`t45`/`t46` triggers spawn guards inside this corridor, which suggests the room is on the main path.
   - Per the `lever.tik` notes, `stop_thread` `OpenRobodoors` and `triggertarget` `water_door_relays` fire when the lever finishes animating:
     - the robots (clockworks) spawn behind the closed robodoors;
     - the cutaway: `cam1` on `hedge2_tunnelcam` for 8 s; `water_door` opens; 3 s; end;
     - the robodoors rise 144 over 2 s, releasing the robots;
     - **the trapdoor drops 184, closing the entrance behind Alice.**
4. **Underwater tunnel and north lakes.** Tunnel water (x -992..640, z 384..672) leads to the lakes (z 256..832). Snark triggers `t39`-`t43`. A 20 s air budget; the dive from the well top to the tunnel is about 1,400 units deep plus the pipe.
5. **End lever and gate.** `nearswitch` #15, `switch2trap` #24 (wait 240, clockwork #25), `gateguys` #19. **Lever #450** (2239 2190 2080; count 1; move_thread `OpenEnd`): a cutaway on `cam1` along `hedge2_endgatecam`; 1 s; `end_doors` open (#53 +X, #54 -X, 40 units each; TARGETED, wait -1); 1 s.
6. **Exit** #52 just past the gate.

**Exit:** `trigger_changelevel` #52 (2240 2808 1884) leads to `tower2$tower2_start1`, after `end_doors` are opened by lever #450.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Snark (swimmer) | 16 | 11 placed; 5 func_spawns (`t39`-`t43`) | no |
| Card guard, spade model (**#58/#59 are "Diamond" classnames with spade models**) | 19 | 2 placed plus 17 func_spawns | no |
| CardGuard-Heart | 12 | func_spawns | no |
| Jabberspawn | 8 | func_spawns (hedge-top leaps) | no |
| Phantasmagoria | 6 | 2 placed (#21, #22), 4 func_spawns | no |
| Boojum | 6 | func_spawns (targets `t48`/`t53`/`t55`/`t56`/`t63` are missing) | no |
| Clockwork | 3 | `robot1`/`robot2` (relays), `switch2trap` (spawnflags 256) | no |
| Precache #1, #746-#752 | 8 | Never; exclude | - |

**Cinematics:**
- `OpenRobodoors` (the stop_thread of lever #507): freeze; `cam1` on `hedge2_tunnelcam` for 8 s; `water_door` opens; 3 s; release; then the robodoors and trapdoor. The skip commits the door, robodoors, trapdoor and robots.
- `OpenEnd` (the move_thread of #450): 1 s, doors open, 1 s. The skip commits the doors open.

**Movers:** the robot-room lever chain (#507, relays #576/#577, `robot1`/`robot2`, `water_door`, robodoors, trapdoor); the end-gate lever (#450, `end_doors`).

**NPCs and dialogue:** the `catz502` hint only.

**Special mechanics:** levers with `move_thread`, `stop_thread` and `triggertarget`; relay-fired func_spawns; door-opening cutaways; a trapdoor that closes behind Alice; a deep well dive and underwater tunnel; Snarks; hedge-top ambushes (two unflagged pads push Alice).

**Rewards:** a Jacks duplicate; the Looking Glass; essences; the shell grant (already true).

**Entity classes:** `Objects_Lever`, `trigger_relay`, `func_door` (TARGETED, angle -1/0/180, lip, time, wait -1), `script_object` (trapdoor, robodoors), `func_spawn`, `trigger_once`/`multiple` (wait 240/300), `trigger_push`, `func_camera`, `Enemies_Snark`/`CardGuard-Diamond` (spade model)/`Phantasmagoria`, items, `trigger_changelevel`, `trigger_catmessage`, world water.

**PLAYABLE MVP checklist:**
- [ ] W7 lever component (draw `lever.tik`; E prompt; clips; count 1; the event timing per the lever notes; relays through the existing receivers).
- [ ] W2 `water_door` (up 168 over 2 s) and `end_doors` (40 units, wait -1).
- [ ] Movers: the trapdoor (+184 at entry, -184 after `OpenRobodoors`, 2 s) and the robodoors (+144, 2 s). They block rather than crush.
- [ ] Controller: set `turtle_air` on entry; lever #507 fires the relays, then `OpenRobodoors`, then the doors and trapdoor; lever #450 fires `OpenEnd`. Consume both threads.
- [ ] The two cutaways on `cam1` (tracks), with commits identical at any skip moment.
- [ ] Encounters: exclude the precache actors; key by model (so #58/#59 are spades); additive rules.
- [ ] Saves: entrance restart when the feet overlap a now-solid door or trapdoor. Cases: `hedge2-robodoors-mid`, `hedge2-after-robodoors`, `hedge2-after-openend`.
- [ ] Launcher `tools/launchers/Launch-Mystifying-Madness.cmd`; `docs/HEDGE2.md`.

**Route check (`--hedge2-route-check`):**
- Legs over the upper maze (222 pathnodes).
- `swim_to` for the well, pipe and tunnel, asserting remaining air above a margin.
- `use_at` for both levers.
- Watched and skipped variants of both cutaways give the same door, trapdoor and robot state.
- **Negatives:** #52 unreachable before `OpenEnd`; the pipe blocked before lever #507.

Assert `transition == ("tower2", Some("tower2_start1"))`, the shell flag, alive, no recovery.

**Anode checks:**
- [ ] A real E press on lever #507: watch `OpenRobodoors` (tunnel shot, water door rising, robots released, trapdoor closing); a second run skips. F5 during the cutaway, restart: the committed state matches.
- [ ] Swim down the well and through the pipe with the air meter visible; fight Snarks underwater.
- [ ] `OpenEnd`, then through the gate. `auto.json` holds `tower2$tower2_start1`, and a fresh `--load auto` works.
- [ ] `--hedge2-render-check` (levers visible).

**Fidelity follow-ups:** lever clip timing; robot release choreography; the trapdoor sound; camera splines; Snark tongue and spit; Phantasm drain; respawns; topiary props; the unreferenced puffball-style objects `t32`/`t35`/`t38`.

**Risks:** the critical path is unproven (the coarse probe was inconclusive); the trapdoor closing must never strand Alice if a skip or restore misorders things; E-key priority between levers, doors and ropes; the 1,400-unit dive; classname versus model.

**Refs:** `maps/hedge2.scr` (main, `Cine_Init`, `OpenRobodoors`, `OpenEnd`); `models/lever.tik` (the Objects_Lever notes); entities #1, #4-#30, #32-#43, #51-#54, #58-#70, #402, #450, #479, #507, #513, #531-#538, #567, #576, #577, #580, #605-#662, #746-#752.

<a id="v32-tower2"></a>
## 32 · tower2 (Water Logged)

| Field | Value |
| --- | --- |
| Visit | `tower2$tower2_start1` (#36 at 1776 2400 640, yaw 270; thread `Tower2_Start`) · ROUTE 31 · next `hedge3$hedge3_start1` |
| Status | **PLAYABLE:** all three stages, flushers, lids/sway, fans, currents, air emitters, five native Snark waves and the Cheshire introduction are implemented. Standalone native Normal routes pass with the intro watched/skipped and live hazards/combat, ten disk continuations each, through the real hedge3 entry. Full campaign-chain validation remains pending |
| Size / milestone | L · M7 |
| Depends on | **W13 (moving entity water)**, W10 (flushers with `ignore_deadtime`), W1 (fliptops: rotate-to plus sway; fans spinning with 1000 crush damage), C4 (Tower intro), E6 (27 Snarks), R1 (`swim_to`, flusher stepping) |
| Reservations | hit-ID 9,100,000 · `tower2/` · `tower2.` · `--tower2-*` · `tower2-*` |

**Arrival:** water non-solid; fans spinning; flushers exempt from the Watch; the turtle shell granted; music `tower2.mus`; `cat_actor1` hidden at `cat_pos1` #396. `Tower2_Start` is the Tower template with a 2 s wait and line `catz505` (`sit_idle1`, `sit_smile_open`); skippable.

**Progression and gates:**
1. **Intro.**
2. **Layout.** The tank water is one brush (x 128..3392, y -352..2688, z -1920..624). Alice starts on a ledge at z 640. There are three bowls with lids: `fliptop1` at about (2176, 1728), `fliptop2`, and `fliptop3` over the octagonal **exit shaft** at (544, 1344) (walls z 384-1104, rim 1104-1152, player-clip rings).
3. **Flush 1.** Stepping on the `flusher01` tray (752, 800; bottom about 896; limit 64, speed 500) fires `MoveWater1` #37: the water rises 256 over 5 s (to 880); `fliptop1` rolls to +35 degrees over 6 s, then sways. `trap000` #406 spawns 3 Snarks.
4. **Flush 2.** `flusher02` (tray bottom about 1152 at 2104, 2144) fires `MoveWater2` #34: water to 1136; `fliptop1` to 90; `fliptop2` to +35, then sway. Traps `trap001` #14, `trap003` #417, `trap004` #422.
5. **Flush 3.** `flusher03` (tray bottom about 1472 at 2256, 1343) fires `MoveWater3` #43: water to 1392; `fliptop2` to 90; **`fliptop3` to -35 over 5 s, uncovering the shaft**. `trap005` #423. A large essence at (1344, 1344, 1408).
6. **Dive to the exit.** Swim over the shaft and dive about 920 units (from 1392 to 472) to #22 within the 20 s air budget. Avoid the bottom currents (#30, #48, #49, #50; speed 200), which pull toward the fans (dmg 1000).

**Exit:** `trigger_changelevel` #22 (544 1344 472, 176x176x32) at the bottom of the shaft leads to `hedge3$hedge3_start1`. It needs `MoveWater3`: lid 3 open and the water above the rim.

**Enemies:** Snarks: 7 placed, active in the water, plus 20 from traps. Not gating.

**Cinematic:** `Tower2_Start` (Tower template, C4).

**Movers:**
- rising water (+256 per stage over 5 s: 624, 880, 1136, 1392);
- flushers (`func_sinkobject`, limit 64, speed 500, ignore_deadtime, with `MoveWater` triggers just above each tray);
- lids (hinged; roll to set angles, then sway);
- drain fans (`fan01`-`05`: spin 256/-256/256/-256/100 deg/s on various axes; dmg 1000);
- the currents.

**NPCs and dialogue:** Cheshire `catz505`; the unnamed hint #21.

**Special mechanics:** a moving liquid volume; sinking trigger platforms that ignore the Watch; hinged lids; lethal fans with currents; a deep dive needing the shell; Snarks.

**Rewards:** 2 small essences and 1 large essence.

**Entity classes:** `script_object` `water` (dynamic liquid), fliptops, fans, `func_sinkobject`, `trigger_once` (`MoveWater1`-`3`, traps), `trigger_push` (currents), `func_spawn`, `Enemies_Snark`, `Characters_CheshireCat`, `func_camera`, `trigger_changelevel` (underwater), items, `emitter_BubbleEmitter_air`.

**PLAYABLE MVP checklist:**
- [x] W13 moving liquid: live brush height feeds immersion, swimming, air and drowning; rendered surface and underwater fog use the same water state.
- [x] Movers: hinged lids open/close and sway; five spinning fans cause lethal blade-contact damage; flushers sink up to 64 using the authored acceleration parameter 500 and remain active during the Watch.
- [x] Controller: ordered water stages, saved machinery, flush sounds, consumed threads and shell grant. Four saved current cooldowns retain the native 0.2-second trigger wait.
- [x] Tower introduction: original track/acting, runtime dialogue, watched/skipped and mid-scene save handoffs.
- [x] Native Snark swimming/combat and five distinct additive waves, with independent activation/death/restore checks.
- [x] Saves: existing water-only saves retain progress and derive missing machinery; controller-less imports retain campaign recovery. Four separate-process native fixtures and ten disk continuations per full route cover machinery and partial air.
- [x] Launcher `tools/launchers/Launch-Water-Logged.cmd`; `docs/TOWER2.md`.

**Route check (`--tower2-route-check`, both intro variants):** starts from fresh Normal arrival with the native cast and ordinary movement/combat. Reaches all three flushers via the underwater pipes, currents, timed fan gap and beveled outlet climb. Collects the large essence, surfaces and dives through #22 into `hedge3$hedge3_start1`, with carried resources and a collision-clear arrival. Both runs end at 75 sanity and about 13.9 seconds of air, without drowning, teleport, god mode or notarget. Ten disk saves per run check identical live futures and continue the route from the reloaded state. `--tower2-check` separately checks all stage heights/timings and saves at 30/60/144 Hz; `--tower2-cast-check` covers every wave, including optional encounters not cleared by the full route.

Negatives: the lid blocks the dive before `MoveWater3`; the flushers work while the Watch is active; touching a fan is lethal.

**Anode checks:**
- [ ] The intro watched and skipped.
- [ ] Flushes 1-3 with real input: screenshots of the rising surface, the lid rotation (confirm the roll sign so the lids open upward), and an underwater view. F5 mid-rise, restart: the water height, lid angles and flusher depth match.
- [ ] The dive with the air meter; the exit to hedge3; the autosave, and `--load auto`.
- [ ] From a staged save, touch a fan: death, then Enter retry.

**Native automation evidence:** hidden-desktop scene/water captures, the two full routes and four separate-process save fixtures pass. The Anode keyboard/F5/retry checks above remain unclaimed. Audio was muted during verification.

**Fidelity follow-ups:** exact native pendulum sway and air-volume bounds setup; frame-rate resampling of the researched sink law; fan blur. See `docs/TOWER2.md`. The route and final dive are proven on Normal; exhaustive optional paths and other difficulties remain unverified.

**Refs:** `maps/tower2.scr` (main, `Tower2_Start`, `Cat1_Dialog`, `T2Start_End`, `MoveWater1`-`3`, `MoveFan`); entities #5-#9, #14, #21-#23, #27, #30, #34-#37, #43, #47-#52, #55-#58, #396, #406-#423.

<a id="v33-hedge3"></a>
## 33 · hedge3 (Labyrinthine Revenge)

| Field | Value |
| --- | --- |
| Visit | `hedge3$hedge3_start1` (the only start, #102 at 6144 -7168 248, yaw 90) · ROUTE 32 · next `tower3$tower3_start1` |
| Status | Machinery restored: 29 inline script objects, weighted lava platform, 27 door leaves, both bellows and all seven skies. Native Normal traversal and the authored tower3 handoff pass through live checkpoint continuations; see [implementation and limits](HEDGE3.md). Existing native enemy cast remains active |
| Size / milestone | XL · M7 |
| Depends on | F4/W1 mover tables (the largest set), W21 (**patch-only turbines, pistons, `t26`**), W10 (#28), W2 (27 doors: AUTO_OPEN, NOT_MONSTERS, NOT_PLAYERS), a bellows state machine (enable the pushes in stages 3-4), W20 (**skies 4-7**, without changing hedge1), E6, E9, E10, E13, E15, E17 (with difficulty spawnflags, including the 256-flagged triggers #13, #19, #896), R1 (`ride`, `wait_until`), R4 |
| Reservations | hit-ID 9,200,000 · `hedge3/` · `hedge3.` · `--hedge3-*` · `hedge3-*` |

**Arrival:** from tower2 #22 (underwater). `bellow2steam` off; music `hedge3.mus`; `GearMoves` start after the player spawns; `sky_camera1`.

**Progression and gates** (the step order is approximate; prove it with the route probe):
1. **Lava crossing.** Alice starts on a ledge (pathnodes at z 224) above the lava lake (world lava x 4416..6976, surface about -512). Stepping stones:
   - `lavagear` (5440 -6352 -504), a 960x960 turntable at 20 deg/s;
   - `func_sinkobject` #28 (6400 -5568 -456; uses speed-60 acceleration/spring law; implicit 1,000-unit limit).

   Then walkways at z -264/-160. FireImps and a FireSnark (difficulty-gated); `boo_scary` #24 spawns 2 Boojums.
2. **Western machine hall.**
   - `gearobstacle01` (30 deg/s) inside updraft #67 above lava.
   - The bellows chamber (lava floor): `bellow1`/`bellow2` run an 8 s four-stage cycle, with `bellowNpush` (152x32x32, pushing west at 1000) and its steam **on only in stages 3-4**, blowing toward lava and hurt #30 (10000).
   - `droptrap` #19 releases a clockwork through the monster-only auto door `t28` #17.
   - Updraft #66 under `turbine01`/`02` (90 deg/s, dmg 1000; patch-only) backed by hurts #4/#1094 (4000).
3. **Gear climb.**
   - `ridegear01` (3424 -2048 1088; continuous +12 deg/s yaw; 776x776);
   - `gearwithspikeaxle` (rolls every 15 s; covered by the static clip `t30`);
   - `ridegear02`/`03` and `wallgear`.

   **All carry dmg 1000, but the ride gears are meant to be ridden, so dmg is crush-only.** Vent updrafts #68-#71 (tops 1072/936/760/704). Auto double doors #82/#83, #84/#85, #86/#87, #683-#690, singles #1/#1103. Ambushes `t6`, `t8`, `t9`, `t10`, `t14`, `t15`, `t16`, `t25`, `hearttrap`.
4. **Eastern hazards.**
   - The crusher corridor (y about -3708): `pendulum01`/`02` alternately lift 272 over 4 s and slam over 1 s (dmg 1000; an 8 s cycle).
   - Hammer hall: `pendulum03` (6656 -3248 1456) and `pendulum04` (6656 -3144 1456) swing ±45 in pitch (period about 3 s, to verify).
   - Auto doors #6/#7 (NOT_MONSTERS).
5. **Vent column to the top.** Pushes #60/#755/#756 and updrafts #61-#63 up to the top level. Lower-level winds #3/#1095/#1096 (speed 300) push toward pit #16/#1058.
6. **Upper gear bridge and exit.** `t3`/`t4` Boojums. Doors #55/#56 and #1021/#1022. Cross `uppergear01`/`02`/`03` (wheels 128 thick, rolling -30/+30/-30 deg/s) over `trigger_fall` #32; skies 6/7 switch here (#828/#829). Exit #101.

**Exit:** `trigger_changelevel` #101 (3808 -1168 2224) leads to `tower3$tower3_start1`. No script gate.

**Enemies (difficulty flags: 256 = not Easy, 1536 = Easy only):**

| Type | Count | Gates |
| --- | --- | --- |
| FireImp | 4 | no |
| FireSnark (lava) | 1 | no |
| Clockwork | 5 (4 placed plus `droptrap`) | no |
| CardGuard-Spade | 8 | no |
| CardGuard-Heart | 5 | no |
| Jabberspawn | 10 | no |
| Boojum | 9 | no |

**Cinematics:** none (no dialogue, no catmessage, no func_camera).

**Movers:**
- the lava crossing (`lavagear`, #28);
- the bellows (the four-stage cycle toggling the pushes and steam; `bellow1arm` is bound to bellow 2's pumpwheel, which is cosmetic);
- the ride gears and meshes;
- crushers and hammers;
- turbines and the obstacle gear;
- the upper gear bridge;
- decorative pistons and `ambigear`;
- the static clip `t30`;
- 27 auto doors (the NOT_PLAYERS door `t28` #17 drops 240 and stays open).

**Special mechanics:** rideable turntables and rolling wheels; crush-only dmg; timed bellows; a sink over lava; auto sliding doors; a chain rope (#43, already verified by `--traversal-check`); vent columns and wind; skies 1-7; lava enemies.

**Rewards:** 6 essences.

**Entity classes:** `script_object` (including patch-only), `func_sinkobject`, `func_door` (AUTO_OPEN, NOT_MONSTERS, NOT_PLAYERS), `trigger_push` (toggled bellows, vents, wind, monster pads), steam emitters, `func_rope`, `trigger_fall`, `trigger_hurt`, `trigger_accelerate`, `trigger_once`/`func_spawn`, `trigger_multiple` (`change_to_sky1`-`7`), `sky_camera1`-`7`, the enemy classes, items.

**PLAYABLE MVP checklist:**
- [x] Mover tables: spin and continuous 15-second half turns, alternating crushers, pendulum swing, bellows stages, sink, static clip. Drawing; colliders including patch hulls; rider carry (yaw turntables, rolling wheels); **dmg as crush only**.
- [x] Bellows: enable `bellowNpush` in stages 3-4 via the shared enable flags (`traversal.rs:224` stays the initial state only); steam on and off.
- [x] #28 and `lavagear`.
- [x] The 27 doors, including NOT_MONSTERS #6/#7 and NOT_PLAYERS `t28` #17.
- [x] W20: add `change_to_sky4`-`7` for hedge3 (`sky_camera4`-`7` #760-#762, #827) without changing hedge1.
- [ ] Encounters with difficulty (including the triggers #13/#19/#896); additive rules.
- [x] Saves: riding `ridegear01` mid-rotation, bellows mid-gust, crusher mid-slam, the sunk platform.
- [x] Launcher `tools/launchers/Launch-Labyrinthine-Revenge.cmd`; `docs/HEDGE3.md`; correct `docs/SKY_PERFORMANCE.md:9` (hedge3 has 7 skies).

**Route check (`--hedge3-route-check`):** a fresh Normal prefix joined to unedited live checkpoint continuations proves lava/weighted platforms, both bellows, gear rides, vent climbs, crushers, hammers, upper wheels and the real `tower3$tower3_start1` handoff. Ordinary controls and the native cast remain active. The route defeats ambushers, collects their drops and the urn essence, then clears the gear Boojum before its exit jump. Seventy-six disk checks compare identical futures, including identical death if idle input is lethal. Actual route progress uses no god mode, notarget, teleport or resource edits. The final baked input matches the verified segments; this is not a single uninterrupted process. Watch and every sky trigger are covered separately by the asset contracts. See [validation](VALIDATION.md#labyrinthine-revenge-machinery-2026-10-01).

**Anode checks:**
- [x] `--hedge3-render-check`: five machinery views, live bellows steam and all seven sky selections; inspected captures remain private. Exact visual parity across every mechanism cycle remains a follow-up.
- [ ] Real input: the crusher corridor and a bellows gust (one deliberate death, then retry); ride a gear; cross the gear bridge. F5 while riding, restart: the rider pose and mover phase are identical.
- [ ] The exit; `auto.json` holds `tower3$tower3_start1`. Run `tools/test_sky_performance.ps1`.

**Fidelity follow-ups:** exact original pendulum integrator (current bounded three-second sinusoid at the authored amplitude); existing enemy-family and lava-detail limitations. Machinery sounds, steam, piston/ambient/turbine motion and save restoration are implemented.

**Risks:** the largest mover set (performance, rider edge cases on rolling wheels); the move from the stop-not-crush policy (`school.rs:850`) to crush is opt-in per mover; tight platform edges and combat knockback; fire AI; recovery on moving supports (`recovery.rs:159`); the sky extension must leave hedge1 unchanged.

**Refs:** `maps/hedge3.scr` (main, `GearMoves`, `Bellow1Move`/`Bellow2Move`, `SpikeGearsMesh`, `ThreeGearMesh`, `Crushers`, `PistonMove`); entities #1-#19, #21-#24, #28-#32, #36-#43, #47, #49, #51, #54-#63, #65-#89, #91, #96, #97, #101-#115, #204, #683-#690, #731, #750-#762, #818-#829, #856-#896, #916, #1017-#1022, #1051-#1105.

<a id="v34-tower3"></a>
## 34 · tower3 (Machinations)

| Field | Value |
| --- | --- |
| Visit | `tower3$tower3_start1` (#44 at 1232 2512 576, yaw 225; thread `Tower3_Start`) · ROUTE 33 · next `grounds1$grounds1_start1` |
| Status | **PLAYABLE standalone on Normal (2026-10-01).** Fresh watched/skipped routes pass with 46 disk continuations, all three fall returns and actual Royal Rage arrival. See [Machinations](TOWER3.md) for scope and fidelity limits. No enemies are authored here. |
| Size / milestone | L · M7 (the recommended mover proving ground: no enemies) |
| Depends on | F4 objects with **bind hierarchies keyed by entity id** (two entities share the name `gear01`) and `bind_localangles`; W14 (checkpoint gating: exactly one floor teleport enabled); W11 (#21/#22 static); C4 (Tower intro with the watch offset); R1 (`ride_to`, `wait_until`) |
| Reservations | hit-ID 9,300,000 · `tower3/` · `tower3.` · `--tower3-*` · `tower3-*` |

**Arrival:** `Room1Tele` leaves only `room1_tele` active. Setup binds the gears and pedals; `lamp_start` is non-solid; `cat_actor1` is hidden at `cat_pos1` #410. The machine threads start, then `Tower3_Start` (the Cat appears after 0.5 seconds and fades in over two seconds; `Cat1_Dialog` `catz506` with `sit_idle1`, `sit_talk1`, `sit_talk3`; fades out; AI on). The skip removes the Cat; the watched path only fades it, which is equivalent.

**Progression and gates:**
1. **Intro.**
2. **Room 1 (north-west).**
   - Elevator gears: `gear03`/`gear03_2` on their bind (±60 deg/s; 256 units, 4 s legs); `gear06`/`gear06_2` (±30; 256; 10 s); `gearup_01` (1472 2336; -30; 256; 10 s); `gearup_02` (1472 1760; +30; 256; 12 s).
   - Spinning columns `gear01` (two entities) and `gear02`.
   - Grinders `grinder01`/`02` (dmg 1000).
   - Pedal wheels `pedns1_bar`/`pedew1_bar` with plates (dmg 15, ridden, so crush only).
   - A fall into `room1_tele` goes to `t3` (1136 2480 584).
3. **Room 2 (south).**
   - `Room1Tele` #4 re-asserts room 1.
   - `Room2Tele` #428 enables only `room2_tele` (falls go to `t2` at 450 704 1120).
   - Pedals `pedns2`/`pedew2`; the pendulum bar `penew1` ±30; the `gear05` elevator (1600 544; 304 units, 10 s); `arm_bar` (70 degrees of yaw over 10 s, 2 s rests); `big_wheel` (continuous pitch at -12 deg/s, dmg 1000) with `big_cage` (360 degrees every 15 s, dmg 1000); `med_gear` (dmg 15).
4. **Room 3 (east).** `Room2Tele` #429 and `Room3Tele` #430 switch the checkpoints (`room3_tele` falls go to `t1` at 2432 960 1632).
   - `gear04` elevator (2368 1120; 512 units, 12 s); `gear_plat` (2112 1984; 256, 10 s);
   - `big_gear1` (30 deg/s) with its bind; `big_gear2` (continuous roll at 180 degrees per 32 seconds, dmg 1000) and its latch (an 8-degree flick in 0.2 s, back over 3.8 s, dmg 1000);
   - static #28/#29;
   - fulcrums #21/#22 (spawnflags 2).
5. **Exit.** `ladder_drop` #19 runs a thread whose ladder, crank and debris targets are **absent from the BSP**, so it is a silent no-op. Correct `docs/TRAVERSAL.md:15`. Then #17.

**Exit:** `trigger_changelevel` #17 (1616 2512 2192) leads to `grounds1$grounds1_start1`. No script gate; the climb depends on the movers.

**Enemies:** none.

**Cinematic:** `Tower3_Start` (Tower template).

**Movers:** elevator gears on binds (translation plus spin, carrying riders in translation and yaw); pedal wheels with plates (`bind_localangles`); the arm; crushing machinery (crush only); spinning columns; checkpoint teleports; fulcrums (static MVP).

**NPCs and dialogue:** Cheshire `catz506`; the unnamed hint #16.

**Special mechanics:** rotating elevator gears on parent binds; pedal plates; swinging bars; crushers; checkpoint teleports that enable one room at a time; seesaw fulcrums.

**Rewards:** 3 small essences.

**Entity classes:** `script_object` (47; 7 modelless binds), `func_fulcrum`, `trigger_teleport` plus `func_teleportdest` (gated), `trigger_multiple` (`Room*Tele`), `trigger_once` `ladder_drop` (no-op), `trigger_changelevel`, `Characters_CheshireCat`, `func_camera`, items, `trigger_catmessage`.

**PLAYABLE MVP checklist:**
- [x] Mover runtime with bind hierarchies keyed by entity id; the reviewed tower3 table; drawing, colliders, rider carry (translation plus yaw; test the rolling pedal plates); crush only.
- [x] Checkpoints: on entry, disable `room2_tele` #35 and `room3_tele` #426. The `Room1Tele`, `Room2Tele` and `Room3Tele` rules enable exactly one (`Action::Enable`); consume the threads.
- [x] Fulcrums #21/#22 solid and static; `lamp_start` non-solid; #28/#29 static.
- [x] `ladder_drop` as a documented no-op.
- [x] The Tower intro.
- [x] Saves: mover clocks and phases, the active checkpoint. Entrance restart for legacy saves. Cases: riding `gear03` mid-lift, on a pedal plate, after `Room2Tele`.
- [x] Launcher `tools/launchers/Launch-Machinations.cmd`; `docs/TOWER3.md`.

**Route check (`--tower3-route-check` and skip variant):** fresh watched and skipped arrival, the lower columns and lifts, both pedal crossings, pendulum, gear05, cage, upper pedal, middle gear, eastern balcony, gear04, large flat gear, rising platform, vertical gear and ratchet. Collect all three essences, deliberately fall once in each room, then cross #17. Exactly three teleports, no recovery/warps, 93 Sanity at the actual `grounds1$grounds1_start1` handoff. The input driver waits at the safe entry to align machinery timing between intro outcomes. The arm, unused lift alternatives and static fulcrums are covered by machinery contracts, not claimed as route segments.

**Anode checks:**
- [ ] The intro watched and skipped.
- [ ] `--tower3-render-check`: gears at both extremes, pedals, the arm, the big wheel and cage, the grinders, the latch flick; the parts stay joined through their binds.
- [ ] Real input: ride a gear; F5 mid-ride, restart: the same phase and rider pose. Fall into room 1's floor and capture the checkpoint teleport.
- [ ] The exit to grounds1; `auto.json`.

**Fidelity follow-ups:** weight-responsive fulcrum tilt; exact pedal plate orientation and pendulum integration; detailed Cheshire acting and audible sound review. The latch impact cue is restored. Absent ladder/crank targets remain a documented no-op; no replacement geometry is invented.

**Risks:** bind hierarchies and duplicate names; rider support on tilting plates; static fulcrums might block a jump that relied on tilt; the docs imply a ladder that does not exist; many dynamic colliders; the teleport gating must be saved.

**Refs:** `maps/tower3.scr` (main, Setup, `Tower3_Start`, `Room1Tele`-`Room3Tele`, `Gear*_MoveUp`, `Pedal_Move`, `Pendulum_Move`, `Arm_Move`, `CageWheel_Move`, `BigGear_Move`, `BigLatch_Move`, `Grinder_Move`, `MedGear_Move`, `ladder_drop`, `Ladder_Smash`, `Crank_Move`); entities #1-#7, #14, #16, #17, #19-#59, #138, #139, #144, #145, #154-#177, #410, #426-#433.

---

<a id="m8-finale"></a>
# Finale: Grounds, Facade, Keep, Queen and ending (M8)

> **Draft specs.** Sections v35-v40 were written during the plan revision of 2026-09-29 from direct read-only reads of the data (entity lumps and scripts) and the code. They have not had the recipe-13a read-and-refute pass that v09-v34 received. **Do not start any M8 visit, B6-B8 or T10 post-game task until recipe 13a has re-verified these six sections** (`args.mode = 'refresh'`, or `'author'` for any section it finds unusable) **and the user has seen the M8 gate report** (CAMPAIGN_PLAN.md §7 M8).

Shared M8 systems:
- the flying Jabberwock (B6), Queen1 (B7) and Queen2 (B8);
- W1: the grounds1 bridge, planks, drawbridge and clips; the grounds2 collapse floor; the facade lift; the keep lift; the qlair floor and stairs; all solid from load where the data places them;
- W2: AUTO_OPEN, TARGETED, TOGGLE, NOT_MONSTERS and NOT_PLAYERS doors;
- W6: bounded spawner pools (grounds2 `Spawner01`-`08`, the keep guard groups);
- the keep puzzle: W4 (shootable portrait triggers), W7 (the mirror lever), W8 (portrait walls), W14 (loss teleports), W19 (the mirror clue);
- W10 (the keep hub crust);
- W20: sky portal swaps in grounds1 and the keep; the qlair farplane and fog fade;
- W22b: the facade and keep dual exits; the finale event;
- T6 directives (`bosslevel`, `killdemons`, `full_stats`, `inqueen2`, the halo, `allow_cheshire`, scene-local timescale), T8 (essence cycles), T10 (post-game), T11 (the qlair checkpoint save);
- C1-C6: scenes, the Gryphon, Jabberwock and Queen rigs, the Cheshire beats.

<a id="v35-grounds1"></a>
## 35 · grounds1 (Royal Rage)

| Field | Value |
| --- | --- |
| Visit | `grounds1$grounds1_start1` (#52 at -2848 288 16, no angle key, so yaw 0; no entry thread) · ROUTE 34 · next `grounds2$grounds2_start1` |
| Status | **BLOCKED: the chasm.** The rope bridge (`bridge_west` *24 #39, `bridge_east` *23 #38), the planks `plank_west1`/`2` and `plank_east1`/`2` (*8-*11, #18-#21) and the `drawbridge` (*20, #35) are `script_object`s, neither drawn nor solid, over `trigger_fall` #7 (*5). The start thread `grounds1_StartCine` never runs, the Jabberwock is a statue, and the exit side is unreachable |
| Size / milestone | XL · M8 · draft (re-verify with 13a) |
| Depends on | **B6**, W1, W2 (doors #40/#41), W20 (sky portal swaps), T6 (`bosslevel`, `killdemons`), T8 (the 8 s essence cycle), T12, C1, C2 (air-battle flyers, the Gryphon fall), C3 (Jabberwock and Gryphon rigs), C4 (`CatThread`), C6, R1, R2, R9 |
| Reservations | hit-ID 9,400,000 (the Jabberwock inside it) · `grounds1/` · `grounds1.` · `--grounds1-*` · `grounds1-*` |

**Arrival** (`main` and `WorldInit`, `maps/grounds1.scr`; `Cine_Init`, `maps/cinematics/grounds1_cine.scr`):
- `bosslevel(1)`, `killdemons`, music `grounds1.mus`, then `grounds1_StartCine`;
- world init: `gate_push` and `cat_thread_trigger` off; the gnome hidden with AI off; `dead_guard` hidden; `sky_lava` becomes the sky portal; `get_me3` spawned; the Cat hidden (alpha 0, non-solid, AI off);
- cinema init: 29 camera paths; the boss `jabberwock` #45 hidden with AI off and the killthread `grounds1_EndCine`; the scene doubles (`cine_wock` #44, `dive_wock` #16, `j` #413, `g` #414, the Gryphons #14, #34 and #423) hidden or frozen; `jabberjump_clips` (*2, clip) and `wock_clips` (*1, monster clip) non-solid.

**Progression and gates:**
1. **Air-battle intro** `grounds1_StartCine` (about 48 s; 16 camera shots; skip `Skipthread_SC`/`SC_End`): the Gryphon and the Jabberwock take off (`jlaunchpad` *7 rises 250), trade strikes on splines, and the Gryphon is struck down (`fall_plat` *6 drops 1600). The fake player runs across the intact bridge to `gdeath` (#280, -1120 288 32) at about 32 s, and the dying Gryphon crashes. At about 43 s the Jabberwock's breath runs `BreakBridge`: `bridge_west` rotates up to 90 and `bridge_east` down to 270 over 1 s, and the planks tumble and drop 2400 over 5 s. The boss lands, speaks `jbwk005` and attacks. The scene swaps the sky portals (`sky_lava`, `sky_manga`, `sky_manga_side`).
2. **Commit** (watched or `SC_End`): Alice at `gdeath`, east of the broken bridge; the scene doubles removed; `gryphon_dying` shown at `gryphon_dying_pos1` (#462, -855 215 22, yaw 270); the boss shown, AI on, attacking; `jabberjump_clips` and `wock_clips` solid; `bosslevel(1)`; `sky_lava` restored.
3. **Boss B6** (below). The medium essence cycles `get_me3`, `get_me1`, `get_me2` (#283, #281, #282), each 8 s after the previous pickup (the pickup_thread names `Centipede2_ME1`-`3` are reused data).
4. **Defeat.** The killthread `grounds1_EndCine` runs only if Alice is alive: 9 s with control, a 1 s fade, then the boss removed and the dying Gryphon shown; the fake player at `alice_end1` (#388, -888 336 32, yaw 270); lines `gry009`, `alcz5001`, `gry010`, `alcz5002`, `gry011` (cameras `grounds1_end1`, then `grounds1_deathrise`). The skip `Skipthread_EC`/`EC_End` gives the same commit.
5. **Drawbridge.** 3 s after control returns (both paths): `cat_thread_trigger` enabled, `jabberjump_clips` removed (`wock_clips` stays), `bosslevel(0)`, and `drawbridge` (*20 at 688 192 8) rotates down to 270 over 10 s with `dmg 1000` (crush only, the W1 blocked policy) while its gears *21/*22 spin; then a slam and `bridge_quake1`. Confirm the rotation sign and the deck height in Anode.
6. *Optional:* `CatThread` (#3, *3): the Cat fades in with `catz508` and is removed; hints are suspended while it plays.
7. **Gate area.** `GnomeGuard` (#27, *17 at 744 160 200): `dead_guard` (#28) is shown and hurt; `gate_push` (#30, *18, target `t15`) is enabled for about 2 s; the gnome (#32) walks to `gate_ledge`, `gate_back` and `gate_hide`, then hides.
8. **Exit** through the AUTO_OPEN doors #40/#41 (*25/*26 at x 1504).

**Exit:** `trigger_changelevel` #33 (*19 at 1696 192 336) leads to `grounds2$grounds2_start1`. It is authored ungated; the lowered drawbridge is the only way to it. The route must prove there is no path to #33 before the drawbridge is down.

**Enemies:** the flying Jabberwock (**gating**). `dead_guard` (Diamond #28) is a scripted corpse. Hazards: four small `trigger_hurt` volumes at the bridge ends (#22, #405-#407) and `trigger_fall` #7 (the chasm).

**Boss B6: the flying Jabberwock** (`jabberwock` #45; `c_jabberwock.tik` health 2000; `ai/c_jabberwock.st`, the branch not named `JLair2`): flight with breath strafes and a dive; `FLY_DISENGAGE` at health 1000 (`c_jabberwock.st:101`) ends the flight phase for the ground phase. Research the flight and landing states before B6 (U6). The shared `src/enemy/jabberwock.rs` serves B5 and B6.

**Cinematics:**
- `grounds1_StartCine` with `SC_End` (commit above). MVP: the camera shots, the scene flyers `j` and `g` on their splines, the Gryphon fall, the bridge break, the landing and `jbwk005`.
- `grounds1_EndCine` with `EC_End` (commit above), gated on Alice alive.
- `CatThread` (a Cheshire beat) and `GnomeGuard` (gameplay staging, no camera).

**Movers:** the rope-bridge halves and planks (collapse program; one-way); the drawbridge and its gears (rotate-to with crush); `jlaunchpad` and `fall_plat` (scene only); the clip brushes; the sky portal objects; the essence spawners.

**NPCs and dialogue:** the Jabberwock `jbwk005`; the Gryphon `gry009`-`011`; Alice `alcz5001`, `alcz5002`; the Cheshire `catz508`; the gnome (walks only).

**Special mechanics:** an air-battle intro that breaks the only bridge behind Alice; a flying boss that lands below half health; a crush-only drawbridge; a pushed gate ledge; sky portal swaps.

**Rewards:** the respawning medium essences. No toy.

**Entity classes:** `info_player_start`, `script_object` (bridges, planks, drawbridge and gears, clips, launch pad, fall platform, sky objects), `trigger_fall`, `trigger_hurt`, `trigger_push`, `trigger_once` (threads), `trigger_changelevel`, `func_door` (AUTO_OPEN), `func_spawn`, `func_camera`, `func_earthquake`, `info_splinepath`, `info_pathnode`, `info_notnull`, `Enemies_Jabberwock` (boss and scene doubles), `Characters_Gryphon` (scene), `script_model` (`j`, `g`), `Characters_GnomeOld`, `Enemies_CardGuard-Diamond` (scripted), `Characters_CheshireCat`, `func_lavaeffects`, steam emitters.

**PLAYABLE MVP checklist:**
- [ ] Registry controller owning the bridge halves, planks, drawbridge and gears, clips, `jlaunchpad`, `fall_plat`, the sky objects, the scene doubles, `gryphon_dying`, the gnome, `dead_guard` and the Cat, all excluded from generic placement.
- [ ] Bridge and planks solid from load and collapsed only by the intro commit; the drawbridge solid in its raised pose until the defeat commit.
- [ ] The intro through the registry `entry_story`, fresh visits only, with the `SC_End` commit; `bosslevel` and `killdemons` through T6.
- [ ] B6 with registry targets, the landing at health 1000, arena recovery east of the break (T12), and the essence cycle (T8).
- [ ] The end scene gated on Alice alive; the 3 s follow-up; the drawbridge program; the optional Cat beat; `GnomeGuard`.
- [ ] Saves: scene id and clock; bridge and plank state; boss state; drawbridge angle; clip state; essence index and timer. Cases: `grounds1-intro-mid`, `grounds1-boss-flying`, `grounds1-boss-landed`, `grounds1-drawbridge-lowering`.
- [ ] Launcher `tools/launchers/Launch-Royal-Rage.cmd`; `docs/GROUNDS1.md`.

**Route check (`--grounds1-route-check` and `--grounds1-skip-route-check`):**
1. Watch or skip the intro; assert feet near `gdeath`, the bridge broken behind Alice, the clips solid and the boss active.
2. Boss loop (the `duchess_check` pattern, with R2 weapons and essence pickups); assert the landing at health 1000 and the death.
3. Wait out the end scene and the 3 s follow-up; assert `jabberjump_clips` removed and the drawbridge at 270 after 10 s.
4. Cross the drawbridge and the AUTO_OPEN doors; walk into #33.

Assert `transition == ("grounds2", Some("grounds2_start1"))`, alive, teleports == 0. Contracts: #33 is unreachable before the drawbridge lowers; the end scene never starts if Alice is dead; watched and skipped commits are equal.

**Anode checks** (recipe 13c; CAMPAIGN_PLAN.md §12.6):
- [ ] `--grounds1-render-check`: take-off, a strike, the Gryphon fall, the bridge break, the landing with `jbwk005`, the boss in flight and landed, the end-scene two-shot, the drawbridge mid-lowering.
- [ ] The intro watched once and skipped once (hold Enter). From `grounds1-boss-flying`: F5, kill, `--load quick`; the boss phase and health restore.
- [ ] The boss fight through `--route-replay` plus a real-input segment of at least 30 s; a second run on Hard.
- [ ] After the defeat: cross the drawbridge with real input; grounds2 loads and autosaves; `--load auto` in a fresh process.

**Fidelity follow-ups:** the air-battle choreography and sky swaps; spline curvature; the Gryphon crash and death animations; the drawbridge gear steam and loops; the gnome's walk; the Cat's fade timing.

**Risks:** the flying AI and its landing rule are unresearched (U6); the drawbridge crush may trap Alice if she stands under it (crush only when blocked); recovery must keep Alice east of the break; the long intro must commit exactly when skipped.

**Refs:** `maps/grounds1.scr` (`CatThread`, the essence threads, `WorldInit`, `GnomeGuard`, `main`); `maps/cinematics/grounds1_cine.scr` (`Cine_Init`, `BreakPlanks`, `BreakBridge`, `Drawbridge`, `EC_End`, `Skipthread_EC`, `grounds1_EndCine`, `SC_End`, `Skipthread_SC`, `grounds1_StartCine`); `ai/c_jabberwock.st`; `models/c_jabberwock.tik`; entities #1-#3, #7, #11, #13, #14, #16, #18-#45, #52, #271, #280-#283, #336, #388, #402-#407, #410-#423, #454, #455, #461, #462.

<a id="v36-grounds2"></a>
## 36 · grounds2 (Battle Royale)

| Field | Value |
| --- | --- |
| Visit | `grounds2$grounds2_start1` (#18 at -1897 -1985 -648, yaw 90; no entry thread) · ROUTE 35 · next `facade$facade_start1` |
| Status | **BLOCKED: fatal arrival.** The collapse floor `fall1`-`fall6` (*9-*14, #19-#24) under the spawn is neither drawn nor solid, so Alice drops into the lava (`textures/liquid/alicelava`, surface z -1056), and every retry repeats the death. `grounds2Cine` never runs; the spawner threads are pending |
| Size / milestone | L · M8 · draft (re-verify with 13a) |
| Depends on | W1 (the collapse floor and `start_clip`), W6 (**bounded `Spawner01`-`08` pools** with launch pads), W4 and T13 (difficulty on the spawner triggers), E9 (Spade, Heart, Diamond), E11 (Magma Men), C1, C6 (`Start_Cinematic_AI_ON`), R1, R5 (zero lava immersion) |
| Reservations | hit-ID 9,500,000 · `grounds2/` · `grounds2.` · `--grounds2-*` · `grounds2-*` |

**Arrival** (`main`, `maps/grounds2.scr`; `Cine_Init`, `maps/cinematics/grounds2_cine.scr`):
- the airship flies with the riding gnome attached at `tag_gnome`; music `grounds2.mus`;
- `BattleRoyal`: the chess allies `good1`-`4` (#246-#248, #250) and the card guards `bad1`-`4` (#249, #251, #271, #272) at 10000 health, paired against each other with dumb AI;
- `heartguard01`/`02` (#8, #295) get vision 4000;
- `start_clip` (*1, a clip brush) non-solid; 6 camera paths.

**Progression and gates:**
1. **Intro** `grounds2Cine` (about 22 s, AI on; skip `Skipthread_C`/`C_End`): the fake player runs from the spawn to `alicestop` (#265, -2016 -416 -608, yaw 90). At +4 s `CollapseEnter` drops `fall6` to `fall1` in turn (each rotates and falls 2400 over 6 s, 0.2 s apart). At about +10 s `launcher01` (#277) spawns the pawn `spawnfall01`, which push #245 launches toward `t12`. Battle cameras watch the duel. The commit: Alice at `alicestop`, the floor fallen behind her, the duel actors removed, `start_clip` solid. The skip commits the same state, but the watched path has already spawned `spawnfall01` and the skip may not have; decide (default in How to read).
2. **Cross the grounds** north and east through four spawner pairs (below). Each spawned Spade is launched onto the field by an unflagged push (#17, #243, #283, #286, #289, #292, #375, #378), which also pushes Alice.
3. **Magma ambush.** `trigger_once` #9 (*5) fires `launchmagma`: two Magma Men (#13, #279) launched by #14/#280.
4. **Exit guards.** `heartguard01` (#8, not on Easy) and `heartguard02` (#295) near the exit, with vision 4000.
5. **Exit** #7.

**Spawner pairs** (`maps/grounds2.scr`, `Spawner01`-`08` and `SpawnDead01`-`08`): each spawner triggers its func_spawn (`bad01`-`bad08`, `cardguard_spade.tik`) every 1 s while fewer than 2 of its spawns are alive, until its total is reached: 2, 2, 3, 3, 4, 4, 6 and 6 for `Spawner01` to `08`. A spawn's death decrements the live count through its killthread. The not-Easy triggers (#10, #293, #294, #379; spawnflags 256) run the odd spawner, which also starts the even one. The Easy-only triggers (#384-#387; spawnflags 3584) run only the even spawner. So Normal and Hard see 30 Spades and Easy sees 15.

**Exit:** `trigger_changelevel` #7 (*4 at 4992 -1032 -312) leads to `facade$facade_start1`. Ungated.

**Enemies (Normal):**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Spade guards (`Spawner01`-`08`) | 30 (Easy 15) | Spawner pairs; at most 2 alive per spawner | no |
| Heart guards | 2 (1 on Easy) | Placed, vision 4000 | no |
| Diamond guard | 1 | Placed #2 | no |
| Magma Men | 2 | `launchmagma` via #9 | no |
| Duel cast (chess allies and card guards) | 8 | Intro only; removed by the commit | - |

**Cinematic:** `grounds2Cine` with `C_End` (commit above). MVP: the camera track, the Alice puppet run, the collapse, the duel staging, the pawn launch.

**Movers:** the collapse floor (collapse program, one-way); `start_clip`; the spawn launch pads; the airship and gnome (decor).

**NPCs and dialogue:** no voiced lines; the riding gnome is decoration.

**Special mechanics:** a floor that collapses behind the arrival; bounded repeating spawners with launch pads that also push Alice; difficulty-dependent spawner pairs; long-sighted Heart guards; lava below.

**Rewards:** super essences #3, #388, #389; the Looking Glass #6; RageBox #26.

**Entity classes:** `info_player_start`, `script_object` (collapse floor, clip, the objects #5/#380 beside the hurt volumes), `trigger_once` (spawner threads; targets), `trigger_push` (launchers), `trigger_hurt` (#4, #381), `trigger_changelevel`, `func_spawn` (Spades, Magma Men, the pawn), `func_camera`, `info_pathnode`, `info_notnull`, `Enemies_CardGuard-Heart`/`Club`/`Diamond`, `Characters_chess_*` (duel allies), items, lens flares.

**PLAYABLE MVP checklist:**
- [ ] The collapse floor solid and drawn from load; collapsed only by the intro commit.
- [ ] The intro, fresh visits only, with the `C_End` commit and the `spawnfall01` decision.
- [ ] W6 spawner pools with the authored totals, live cap 2 and 1 s cadence; pre-allocated slots; the killthread decrement; saved.
- [ ] Difficulty on the spawner triggers (W4, T13).
- [ ] Encounters: the Spades, Hearts (vision 4000), the Diamond and the Magma Men (E9, E11).
- [ ] Saves: scene clock, floor state, spawner counters and live slots. Cases: `grounds2-intro-mid`, `grounds2-spawners-mid`, `grounds2-magma`.
- [ ] Launcher `tools/launchers/Launch-Battle-Royale.cmd`; `docs/GROUNDS2.md`.

**Route check (`--grounds2-route-check` and skip variant):** the arrival (assert no fall and no lava contact), the intro commit, then legs across the four spawner zones with auto-aim combat, the Magma ambush, the exit guards and #7. Assert `transition == ("facade", Some("facade_start1"))`, zero lava immersion, alive, Sanity above 0; log the spawns per pair and difficulty. An Easy variant asserts 15 Spades.

**Anode checks:**
- [ ] `--grounds2-render-check`: the arrival floor, the collapse mid-fall, the duel, a Spade launched from a pad, the Magma launch.
- [ ] Real input: arrive (no fall), skip the intro, fight through one spawner pair; from `grounds2-spawners-mid`: F5, kill, `--load quick`, and the counters and live spawns restore; reach #7; facade autosaves; `--load auto`.

**Fidelity follow-ups:** the airship and gnome; the duel choreography; launch arcs; spawn cadence; the lens flares; what the objects #5/#380 and hurts #4/#381 represent.

**Risks:** the spawner pools must stay within the save limits; launch pads that also push Alice near lava; the fatal arrival until W1 lands.

**Refs:** `maps/grounds2.scr` (`Spawner01`-`08`, `SpawnDead01`-`08`, `main`); `maps/cinematics/grounds2_cine.scr` (`Cine_Init`, `CollapseEnter`, `Launcher`, `BattleRoyal`, `C_End`, `Skipthread_C`, `grounds2Cine`); entities #1-#26, #241-#295, #373-#389.

<a id="v37-facade"></a>
## 37 · facade (Ascension)

| Field | Value |
| --- | --- |
| Visit | `facade$facade_start1` (#76 at 2240 1456 -2744, yaw 270; no entry thread) · ROUTE 36 · next `keep$keep_start1` |
| Status | **Doors and departure restored locally.** Solid paired gates and lift, original departure camera, saved rider/skip clock, and a single gated transfer into Keep with carried state. See [implementation and verification limits](FACADE.md). The full cave combat route remains a separate acceptance item. |
| Size / milestone | L · M8 · draft (re-verify with 13a) |
| Depends on | W1 (the lift), W2 (the `t15` doors), W5 (the relay #24), W6 (func_spawn groups; the monster-only pad #39), W22b (**one latch for the lift scene and the fallback volume**), E9, E10, E12, E13, E17, C1, R1 (updraft climbs) |
| Reservations | hit-ID 9,600,000 · `facade/` · `facade.` · `--facade-*` · `facade-*` |

**Arrival** (`main` and `Facade_World_Init`, `maps/facade.scr`): the `skycamera` (*21) sky portal; `Facade_Cinematics_Init` loads `facade_path1`-`3`; music `facade.mus`. No scene.

**Progression and gates** (the order is inferred from coordinates; prove it with the route probe, U9):
1. **Lower level near the start** (z about -3000 to -2500): `bridge-ambush` (#50), `cardledge` (#47) and `heart-01` (#27).
2. **West caves:** `jspawncave` (#28), `gangloadoimps` (#29, #447), `jabbrah-cadabrah` (#36), `imp-attack-01`/`02` (#40, #396), `imp-houzin` (#410) and `bottomfeederimps` (#412). Lava (`alicelava`, surfaces up to z -2752) lies below.
3. **Updrafts.** `trigger_accelerate` #74 (*19) and #176 (*20) (speed -1) rise over the hurt planes #53 (*13) and #366 (*22). The relay #24 (delay 0.5) wakes `imp-attack-03`; pad #39 (spawnflags 20) launches monsters only.
4. **Upper level** (z about -1600 to -1000): `doorguards` (#21); the `t15` doors #31/#32 (*6/*7; TARGETED; #32 also NOT_MONSTERS) opened by `trigger_multiple` #19 (*1, wait 3); `guard-02` (#58); `guardgroup-01` (#59); the Tea #61.
5. **The lift.** `trigger_once` #56 (*14 at 320 2976 -1508) runs `Facade_Lift`: AI off, the fake player, `Start_Cinematic`, camera `dialog_watch1` on `facade_path3`. `facade_lift` rises 1500 and turns (yaw up to 1024) over 10 s with the frozen player riding, and 4.5 s after the trigger the script changes level. The skip `Skipthread_FA1`/`FA1_End` fades for 0.5 s, then changes level.

**Exit:** the lift scene's level change to `keep$keep_start1`; the fallback `trigger_changelevel` #67 lies on the lift's path. Both go through one W22b latch. The physical volume stays gated until the scene commits, preventing an early transfer (U10).

**Enemies (Normal; spawnflags-64 actors wake from their triggers):**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Heart guards | 15 (14 placed, 1 func_spawn) | Placed and delayed groups | no |
| Fire Imps | 9 (4 placed, 5 func_spawns) | Cave triggers, relay #24 | no |
| Club guards | 1 placed, up to 3 func_spawns | `bridge-ambush` #50 (confirm what fires `t13`/`t14`) | no |
| Spade guards | 2 | Placed; `guard-02` | no |
| Boojums | 3 (difficulty-flagged) | `jspawncave` | no |
| Phantasmagoria | 1 | `imp-attack-02` | no |
| Jabberspawn (asleep) | 1 | `jspawncave` | no |

**Cinematic:** `Facade_Lift` with `FA1_End`. MVP: the camera, the lift ride with Alice held on the deck, and the transition; the skip commits the same transition. `Facade_Cinema1` is never triggered (dead data).

**Movers:** `facade_lift` (translate plus yaw, rider carry); the `t15` doors; the sky object.

**NPCs and dialogue:** none.

**Special mechanics:** lava caverns with updraft climbs over hurt planes; delayed groups woken by triggers; a monster-only launch pad; a lift ride that ends in a scripted transition, with a physical fallback volume.

**Rewards:** Grasshopper Tea #61; RageBox #45; small, medium and large essences (several difficulty-flagged); a small health and a small mana pickup.

**Entity classes:** `info_player_start`, `script_object` (lift, sky), `script_model` (sky), `trigger_once` (targets; thread), `trigger_multiple`, `trigger_relay`, `trigger_push` (monster-only), `trigger_accelerate`, `trigger_hurt`, `trigger_changelevel`, `func_door`, `func_spawn`, `func_camera`, `info_pathnode`, `info_notnull`, `Enemies_CardGuard-Heart`/`Spade`/`Club`, `Enemies_FireImp`, `Enemies_Boojum`, `Enemies_Phantasmagoria`, `Enemies_Jabberspawn-Asleep3`, items, emitters.

**PLAYABLE MVP checklist:**
- [x] Controller owning the lift and the latch; the lift solid and drawn from load.
- [x] The lift scene with its commit and the single latch shared with #67.
- [ ] W2 doors; the W5 relay; W6 groups and the monster-only pad.
- [ ] Encounters with difficulty flags.
- [ ] Saves: the lift pose and scene clock; the latch; the woken groups. Cases: `facade-lift-mid`, `facade-caves`.
- [x] Launcher `tools/launchers/Launch-Ascension.cmd`; `docs/FACADE.md`.

**Remaining full-route acceptance:** legs through the caves with auto-aim combat; the updraft climbs (assert hurt-plane damage within a budget and zero lava immersion); the `t15` doors; then walk onto the lift into #56. Assert exactly one transition, `("keep", Some("keep_start1"))`, 4.5 s after the trigger (watched) or 0.5 s after the skip, and never through #67 first.

The current `--facade-route-check` and skip variant use isolated updraft, gate and lift fixtures with ordinary movement after initial placement; they do not certify the full cave combat route.

**Anode checks:**
- [ ] `--facade-render-check`: a cave group waking, an updraft column, the `t15` doors, the lift mid-ride from `facade_path3`.
- [ ] Real input: an updraft climb; open the `t15` doors; ride the lift once watched and once skipped; keep autosaves; `--load auto` in a fresh process.

**Fidelity follow-ups:** the unused `Facade_Cinema1`; lift sound and sway; imp leap arcs; lava effects.

**Risks:** the inferred route order; hurt planes under the updrafts; the double transition (U10); legacy saves inside the caves.

**Refs:** `maps/facade.scr` (`Facade_World_Init`, `main`); `maps/cinematics/facade_cinematics.scr` (`Facade_Cinematics_Init`, `FA1_End`, `Skipthread_FA1`, `Facade_Lift`, `Facade_Cinema1`); entities #1-#77, #176, #297-#299, #343-#348, #366-#477.

<a id="v38-keep"></a>
## 38 · keep (Castle Keep)

| Field | Value |
| --- | --- |
| Visit | `keep$keep_start1` (#80 at 512 288 -56, yaw 90; thread `Start_Keep`) · ROUTE 37 · next `qlair$qlair_start1` |
| Status | **Restored locally.** Saved lift, mirror/suit clues, repeatable portrait puzzle, guarded doors and Cheshire death sequence with gated qlair transition. See [implementation and verification](KEEP.md). |
| Size / milestone | XL · M8 · draft (re-verify with 13a) |
| Depends on | W1 (the lift, `keep_monster_clip`, paintings, shafts, mirror), W2 (suit, heart and queen doors), W4 (**9 shootable portrait triggers**), W6 (bounded guard groups), W7 (the mirror lever, reset 1 s), W8 (portrait walls), W10 (the hub crust), W14 (loss teleports), W19 (**the mirror clue**, or a recorded fallback), W20 (sky swaps), W22b (**one latch for the scene exit and the fallback volume**), E9, C1, C4 (Cheshire beats), C6, T6 (`allow_cheshire`, scene-local timescale), DG-8 |
| Reservations | hit-ID 9,700,000 · `keep/` · `keep.` · `--keep-*` · `keep-*` |

**Arrival** (`Start_Keep` in `maps/cinematics/keep_cinematics.scr`; `Keep_World_Init` in `maps/keep.scr`):
- `keep_monster_clip` (*2) non-solid; the fake player; `Start_Cinematic`; camera `heart_watch1` on `keep_introp1`;
- `keep_lift` rises 192 over 5 s; at +3 s the `lift_cat` fades in and says `catz502`, then fades out and leaves. The skip `Skipthread_KI`/`KI_End` does not stop the lift, which finishes its move;
- world init: the `black_sky` portal; the club, spade and diamond paintings hidden; every painting marked `+nomirror`; the mirror (*42) turned to yaw 315.

**Progression and gates:**
1. **Hub.** Slime (`slime_nodraw`, z about -8..88) with the `func_sinkobject` crust #70 (*48, limit 44). The vent #25 (*14, `trigger_accelerate` speed -1) lifts Alice to the ring at z about 512. Placed Hearts, Spades and a Club patrol.
2. *Optional:* `Lever_Cat_Dialog` (#1, *1): the `lever_cat` beat `cat040` (skip `LCD_End`).
3. **Mirror lever** `mirror_lever` #340 (`lever.tik`, move_thread `Keep_Rotate_Mirror`, reset 1 s). Each pull runs a short scene (`keep_path2`) that resets the loss teleports and all door triggers, then turns the mirror to 60 (club), 0 (diamond) or -60 (spade) over 5 s, cycling in that order. The matching hidden painting is shown only in the mirror (`+onlymirror`, the W19 clue), and that suit's door trigger is enabled.
4. **Suit rooms** (club south, spade east, diamond west). The door pairs `door_club1` (#14/#15), `door_spade1` (#12/#13) and `door_diamond1` (#16/#17) (spawnflags 144; lip 96; 0.2 s; wait 2) are opened by `club_door_open` #543, `spade_door_open` #542 and `diamond_door_open` #11 (wait 1.5). The far trigger (`door_club_begin` #29, `door_spade_begin` #30, `door_diamond_begin` #31) runs `Door_<suit>_Close`: a gong, the suit's shaft (*36-*38) hidden, all guard groups removed and the grounds sky shown; after 1 s the room's three portrait triggers are enabled and its portraits shown.
5. **Portrait choice.** Shoot a portrait with a toy (the triggers have health 1 and spawnflags 32):
   - the correct portrait (club: `tweedle_club` via #473; spade: `hatter_spade` via #469; diamond: `jabber_diamond` via #32) wins: the puzzle count rises; the room's `*_door_open_win` trigger is enabled; the portrait wall (`func_smashablewall`) is smashed; the other portraits, triggers and the shaft are removed; `spawn_heart1`/`2` (#10, #547) spawn; the paired paintings are removed; the sky is restored;
   - a wrong portrait loses: a red fade; the room's loss teleport (`club_teleport` #467 *56, `spade_teleport` #466 *55, `diamond_teleport` #36 *24; spawnflags 8) is enabled toward `t9` (#35 at 512 3528 272); five suit guards spawn (`<suit>_spawn1`-`5`); the portraits hide; the shaft shows; the room's begin trigger is re-armed; the door triggers reset;
   - **data defect:** the club and spade Lose threads disable the diamond portrait triggers instead of their own (a copy-paste slip), so that room's own portrait triggers stay live after a loss. Apply the reviewed fix (disable the room's own triggers) as a documented deviation.
6. **Heart doors.** After the third win, `heart_door1` (#8/#9; spawnflags 176 = TARGETED, TOGGLE and NOT_MONSTERS; lip 128; 0.1 s) opens with a 5 s camera shot (`keep_heartdoor1`), `heart_door_trigger` #7 is enabled, and the mirror lever is removed. The heart doors toggle, and both #7 and the Cheshire scene trigger them again, so model the toggle parity explicitly (U4).
7. **Cheshire scene and exit.** `trigger_once` #64 (*45 at 512 4596 528) runs `Keep_Cheshire_Dead`: AI off; the fake player at `alice_pos1`; hints disabled (`allow_cheshire`); skip `Skipthread_K1`/`K1_End`. The three `cheshire_actor` puppets appear in turn with `catz510a`, `catz510b` and `catz510c`; the queen doors #57/#58 (spawnflags 184, which adds NOT_PLAYERS; lip 180; speed 10) open; the Queen's popup (`cat_popup` #550, `c_queen1_popup.tik`) strikes under a **scene-local** timescale of 0.7; Alice kneels and weeps; `alcz4011`; 2 s later the timescale returns to 1 and the script changes level. `K1_End` resets the timescale and changes level after a 0.5 s fade.

**Exit:** the scene's level change to `qlair$qlair_start1`; the fallback `trigger_changelevel` #71 (*49 at 512 5088 512) lies behind the NOT_PLAYERS queen doors. One W22b latch.

**Enemies:**

| Type | Count | Activation | Gates |
| --- | --- | --- | --- |
| Heart guards | 6 placed, plus `spawn_heart1`/`2` | Placed; the hearts spawn after each win | no |
| Spade guards | 2 placed, plus `spade_spawn1`-`5` | The spawns follow a wrong spade portrait | no |
| Club guards | 1 placed, plus `club_spawn1`-`5` | The spawns follow a wrong club portrait | no |
| Diamond guards | `diamond_spawn1`-`5` | The spawns follow a wrong diamond portrait | no |

Every group is removed at each room entry and at each win or loss, so the pools stay bounded.

**Cinematics:** `Start_Keep` (`KI_End`); `Lever_Cat_Dialog` (`LCD_End`); `Keep_Rotate_Mirror` (per pull; commit the mirror pose, the painting and the door trigger); `Heart_Door_Open` (the 5 s shot); `Keep_Cheshire_Dead` (`K1_End`; the popup; the transition). MVP: tracks, the Cat puppets and beats, the popup, and the timescale scoped to the scene (never `FIXED_DT`).

**Movers:** `keep_lift`; `keep_monster_clip`; the mirror and lever; suit, heart and queen doors; shafts; paintings and portrait walls; the hub crust; sky objects (`black_sky`, `grounds_sky`, `default_sky`). `cat_head1` (#60, a `func_fallingrock`) is hidden and never triggered (dead data).

**NPCs and dialogue:** the Cheshire (`lift_cat` `catz502`, `lever_cat` `cat040`, `cheshire_actor1`-`3` `catz510a`-`c`, all at `maxmouthangle 45`); Alice `alcz4011`; the Queen's popup (a prop).

**Special mechanics:** a lift arrival over a fall; a mirror-only clue; shoot-the-right-portrait rooms with loss teleports and guard waves; toggle doors; a scene-local slow motion; a scene exit with a physical fallback; hint suppression.

**Rewards:** none; the map places no pickups.

**Entity classes:** `info_player_start` (thread), `script_object` (lift, clip, mirror, paintings, shafts, skies), `script_model` (popup, waterfall faces), `Objects_Lever`, `portal_surface`, `func_door`, `func_smashablewall`, `trigger_multiple` (doors, shootable portraits), `trigger_once`, `trigger_teleport` plus `func_teleportdest`, `trigger_accelerate`, `trigger_fall`, `trigger_changelevel`, `func_sinkobject`, `func_fallingrock` (unused), `func_spawn`, `func_camera`, `sound_speaker` (gongs), `info_waypoint`, `info_null`, `Characters_CheshireCat`, `Enemies_CardGuard-Heart`/`Spade`/`Club`, `func_lavaeffects`, steam emitters.

**PLAYABLE MVP checklist:**
- [ ] The lift and clip solid from load; the arrival scene with its commit.
- [ ] The mirror lever (W7) and the three-position mirror with the painting clue (W19, or a recorded fallback).
- [ ] Suit doors, begin triggers, portraits, win and loss threads, loss teleports and guard groups, with the reviewed Lose-thread fix; the three-win heart doors with explicit toggle parity.
- [ ] The Cheshire scene with the popup, the scene-local timescale and the exit latch shared with #71.
- [ ] Hint suppression per DG-8.
- [ ] Saves: lift, mirror position, puzzle count, room states, door parity, guard pools, scene clock and timescale, the latch. Cases: `keep-arrival`, `keep-mirror-mid`, `keep-room-lost`, `keep-two-wins`, `keep-cheshire-mid`.
- [ ] Launcher `tools/launchers/Launch-Castle-Keep.cmd`; `docs/KEEP.md`.

**Route check (`--keep-route-check` and skip variant):**
1. The arrival (assert no fall); the vent to the ring.
2. For each suit: pull the lever until the matching mirror position, open the door, cross the begin trigger, shoot the correct portrait with `shoot_trigger`/`aim_at` (never auto-aim), and assert a win.
3. A negative variant shoots a wrong portrait once: assert the loss teleport, the guard spawns and a clean retry.
4. After three wins: the heart doors; walk into #64; watch or skip.

Assert exactly one `transition == ("qlair", Some("qlair_start1"))`; teleports equal the deliberate losses; the timescale is 1 after the transition.

**Anode checks:**
- [ ] `--keep-render-check`: the lift arrival with the Cat; each mirror position with its painting visible only in the reflection; an armed portrait room; a portrait smash; the heart doors opening; the popup strike.
- [ ] Real input: pull the lever (E); shoot a portrait with a toy; from `keep-two-wins`: F5, kill, `--load quick`, and the count, mirror and doors restore; the Cheshire scene watched once and skipped once; qlair autosaves; `--load auto`.
- [ ] Negative: a wrong portrait teleports Alice to `t9` and spawns guards; Cards and Dice seekers never pick a portrait trigger (W4).

**Fidelity follow-ups:** full planar reflections; gong and grind sounds; the sky swaps; the waterfall faces; the Cat's acting; the kneel and weep; the unused `cat_head1` roll.

**Risks:** the mirror renderer (K13, DG-10); toggle-door parity; the Lose-thread defect; seekers or demons firing portrait triggers; the scene timescale leaking into play or saves; the double transition.

**Refs:** `maps/keep.scr` (`Keep_World_Init`, `main`); `maps/cinematics/keep_cinematics.scr` (`Heart_Door_Open`, `Door_Diamond_Close`, `Door_Spade_Close`, `Door_Club_Close`, `Generic_Lose_Thread`, `Keep_Rotate_Mirror`, the `*_Lose` and `*_Win` threads, `Keep_Cinematics_Init`, `Keep_Cheshire_Anims1`-`3`, `K1_End`, `Keep_Cheshire_Dialog`, `Skipthread_K1`, `Keep_Cheshire_Dead`, `Lever_Cat_Dialog`, `LCD_End`, `KI_End`, `Skipthread_KI`, `Start_Keep`); `models/lever.tik`; entities #1-#80, #207, #310-#357, #466-#487, #523-#550.

<a id="v39-qlair"></a>
## 39 · qlair (Heart of Darkness)

| Field | Value |
| --- | --- |
| Visit | `qlair$qlair_start1` (#41 at 0 -1568 80, yaw 90; no entry thread) · ROUTE 38 · next: the ending film (no map) |
| Status | **BLOCKED: the finale is absent.** The `Boss_*` classes are excluded from placement; the floor pieces `brokenfloor1`-`12` (*6-*17, #22-#33) and `queen_stairs` (*19, #37) are neither drawn nor solid; `QLair_Cinema1` (#36) is pending; nothing requests the ending |
| Size / milestone | XL · M8 · draft (re-verify with 13a) |
| Depends on | **B7**, **B8**, W1 (the floor and stairs solid until the collapse), W14 (the secret teleport #13), W20 (farplane 10000, then 3000; the fog fade), W22b (**the finale event**), T6 (`bosslevel`, `killdemons`, `full_stats`, `allow_cheshire`, `inqueen2`, the halo on `tag_gut`, scene-local timescale), T8 (two essence cycles), T10 (post-game), T11 (**the checkpoint autosave**), T12, C1, C2 (Queen puppets, the popups), C3 (Queen rig), C5 (music mood), C6, R1, R2, R9 |
| Reservations | hit-ID 9,800,000 (Queen1, Queen2 and their parts) · `qlair/` · `qlair.` · `--qlair-*` · `qlair-*` |

**Arrival** (`main`, `QLair_Cinema_Setup`, `QLair_Setup` and `Queen2_Birthing_and_Creation` in `maps/qlair.scr`):
- Queen1 non-solid; farplane 10000; 17 camera paths; the five corridor popups and `alice_magic_power` hidden;
- hints off and `full_stats`; Queen1 (`queen` #71) and the big tentacle (`tentacle` #72) with AI off and non-solid, the Queen attached to the tentacle at `tag_queen`, the Queen's killthread `Die_Bitch`, the tentacle moved to the throne;
- Queen2: the halo and `tentacle1`-`4` attached to the body at `tag_halo` and `tag_t01`-`04`; the body's killthread `QLair_Ending`; the sink depth 3000 and the slow mark 512 below the body's top; platform 4 and last position 5 to start;
- after spawn: `bosslevel(1)`, `killdemons`, `full_stats`, hints off; `Popup_Thread` plays the five popups along the corridor, 0.5 s each.

**Progression and gates:**
1. **Throne-room intro** `QLair_Cinema1` (`trigger_once` #36, *18 at 8 -800 624; AI on; **no skip thread**, DG-4): Queen1 and the tentacle become solid; cameras `qlair_path1`/`2`; Alice's line `alcz5003` (authored on the Queen entity; relabel the speaker) and the Queen's `qnr001a`; the throne breaks. Then `floor_sound` and `queen_quakex1`, `QLair_Floor_Break1`/`2` (the twelve floor pieces rotate and fall 2400 over about 10 s) and `QLair_Stairs_Break`. 3 s later Alice is returned to `alice_start_pos1` (#75, 0 -792 0) and `get_me1` spawns. The name `queen_clip` matches no entity, so its solid call is a no-op.
2. **Queen1 fight** (B7, below): the medium essences `get_me1`/`get_me2` alternate, each 10 s after the previous pickup. The grab camera (`queen_grab_cam` #19 on `qlair_grab1`) is driven by `GotoQueenCam`/`GotoPlayerCam`, which the AI calls (U6). `trigger_fall` #17 (*4) lies under the broken floor.
3. **Queen1 death** runs `Die_Bitch` only if Alice is alive, and checks again 4.5 s in. The chain (`Die_Bitch`, `Die_Bitch2`, `QLair_Body_Talk`, `QLair_Body_Dialog`; **no skip thread**, DG-4):
   - the body shown at `bitch2_pos1`; Queen1 and the tentacle frozen, then pulled through and hidden; music to suspense;
   - the power-up (`alice_magic_power`), and on the real player `inqueen2(1)`, `alice_halo.tik` on `tag_gut` and `full_stats`;
   - the fog fades to 5000 over 8 s; the throne breaks under a timescale of 0.8 for Alice's knockdown;
   - the body's lines `qnr002`-`004`; the body moves to `bitch_pos2` (24 4840 8) with its tentacles shown; Alice lands at `alice_fight_queen` (#15, -1080 3928 960, yaw 45); `help_me1` spawns; farplane 3000; the lightning loop starts; the halo strobes;
   - control returns; `GotoMiddle` starts; then the **checkpoint save request (T11: one autosave)**; the body's AI goes on 5 s later.
4. **Queen2 fight** (B8, below). The large essences `help_me1`-`4` cycle, each 10 s after the previous pickup. `trigger_fall` #11 (*2) lies under the platforms. *Optional:* the secret teleport #13 (*3) sends Alice to `q2_secret` (#12, 976 4144 984, yaw 135); the Blunderbuss altar #14 stands beside the teleport.
5. **Queen2 death.** At body health 1000 the AI enters `START_DEATH`, which runs `Queen2_Death`: the fight stops, the lightning flashes, and the body rises if it was down. The body's death then runs `QLair_Ending`: if Alice is alive, wait 10 s, check again, then request the ending film with the main menu and the credits queued after it. **No map change.**

**Exit:** the finale event `ending_ready` (W22b), consumed by v40.

**Boss B7: Queen1** (`c_queen1.tik` health 2500; the puppet rides the invulnerable big tentacle): popups, lasers and beams, telekinesis slam and grab (with the grab camera), iceball; wounded below 1,100 (CAMPAIGN_PLAN.md §6.2 B7; research the Queen1 attacks first, U6).

**Boss B8: Queen2** (`c_q2_body.tik` health 4500, the halo and four tentacles; `ai/c_queen2.st`): part-gated attacks (`!PART_DEAD` conditions) and tentacles immune to the Blunderbuss. The body sinks up to 70 units per 0.05 s step (after a 50-step ramp) and rises 45 units per step, slowing inside the top 512, at `point1`-`5` (`info_player_deathmatch` markers used as waypoints). It follows Alice's platform: 8 `trigger_multiple` planes (wait 1, delay 1) run `MoveQueen1`-`4`, and `GotoMiddle` rises at `point5`, waits 17 s, then follows the platform Alice is on. Missing names (`queen_clip`, `queen_teleport` and the misspelled tentacle names) stay no-ops; the tentacles' authored solidity comes from `Queen2_Birthing_and_Creation`.

**Cinematics:** `QLair_Cinema1`; the `Die_Bitch` chain; `Queen2_Death`; `QLair_Ending`. All are unskippable in the data. DG-4 decides hold-Enter skips with identical commits; the Die_Bitch commit must include the power-up, the halo, `inqueen2`, the arena warp and the checkpoint save, each exactly once.

**Movers:** the floor pieces and stairs (collapse, one-way); `queen_breast_hide`; the Queen2 body rig (sink and rise); the popups; the lightning models; the power-up light.

**NPCs and dialogue:** the Queen `qnr001a` and `qnr002`-`004`; Alice `alcz5003`.

**Special mechanics:** a collapsing throne-room floor; a two-stage final boss, first a puppet on an invulnerable tentacle, then a multipart body that submerges and follows the player's platform; the halo power-up; a scripted checkpoint save; a scene-local slow motion; a finale that ends in a film, not a map.

**Rewards:** the Blunderbuss #14 (secret; slot 8; DG-5); `full_stats` on arrival and in the power-up; the two essence cycles.

**Entity classes:** `info_player_start`, `Boss_Queen1_Puppet`, `Boss_Queen1_Tentacle`, `Boss_Queen2_Body`, `Boss_Queen2_Halo`, `Boss_Queen2_Tentacle1`-`4`, `script_object` (floor, stairs, breast cover), `script_model` (popups, throne, lightning, sky), `script_skyorigin`, `queen_magic_light`, `trigger_once`, `trigger_multiple` (platform planes), `trigger_teleport` plus `func_teleportdest`, `trigger_fall`, `func_spawn`, `func_camera`, `func_earthquake`, `sound_speaker`, `info_player_deathmatch` (waypoints), `info_pathnode`, `info_waypoint`, `Item_WeaponPickup_Blunderbuss`.

**PLAYABLE MVP checklist:**
- [ ] Controller owning every `Boss_*` actor, the throne, popups, floor, stairs and lights; `Boss_*` excluded from generic placement.
- [ ] Floor and stairs solid from load, collapsed only by the intro commit.
- [ ] B7 and B8 per CAMPAIGN_PLAN.md §6.2, with registry targets inside 9,800,000, deterministic seeded choices, and arena recovery that never resets a boss (T12).
- [ ] The Die_Bitch chain with its two alive checks, the T6 directives and the T11 autosave exactly once.
- [ ] The Queen2 platform logic, the essence cycles and the secret teleport.
- [ ] `QLair_Ending`: two alive checks 10 s apart, then `ending_ready` exactly once; never a map change.
- [ ] Saves: the stage (intro, Queen1, death chain, Queen2, ending), boss parts and health, the body's height, platform and position state, essence timers, the power-up flags, the checkpoint latch. Cases: `qlair-queen1`, `qlair-die-chain`, `qlair-queen2-sunk`, `qlair-ending-wait`.
- [ ] Launcher `tools/launchers/Launch-Heart-of-Darkness.cmd`; `docs/QLAIR.md`.

**Route check (`--qlair-route-check` and skip variant):** the corridor; the intro (assert the collapse and the return to `alice_start_pos1`); the Queen1 loop; the death chain (assert `inqueen2`, the halo and one checkpoint save, and Alice at `alice_fight_queen`); the Queen2 loop across the platforms. After the kill, assert `ending_ready` exactly 10 s later and no map transition, and print the `ENDING` flag. Contracts: a Queen death while Alice is dead starts nothing; a boss never resets on recovery; the ending never fires twice.

**Anode checks:**
- [ ] `--qlair-render-check`: the popups, the throne break and floor collapse, Queen1 attacks and the grab camera, the death-chain shots, Queen2 sunk and risen, the halo, the lightning.
- [ ] Queen1 and Queen2 on Normal and Hard through `--route-replay` plus real-input segments of at least 30 s (latency rule).
- [ ] From `qlair-queen2-sunk`: F5, kill, `--load quick`; the checkpoint autosave resumes Queen2 in a fresh process.
- [ ] The ending protocol is v40 (CAMPAIGN_PLAN.md §8.3 S8).

**Fidelity follow-ups:** the Die_Bitch choreography; Queen gibs and blood; the popup presentation; lightning timing; the fog fade; music moods.

**Risks:** the largest boss (B8 is XL); unresearched Queen attacks (U6); long unskippable scenes (DG-4); a scripted save during a scene; the waits in the platform logic; recovery on moving platforms.

**Refs:** `maps/qlair.scr` (`Q2_Lightning`, `QLair1_Alice_Powerup`, `QLair1_Fakeplayer_Powerup`, `QLair_Body_Anims`, `QLair_Body_Dialog`, `Alice_QueenMean_Anims`, `QLair_Body_Talk`, `DownBitch`, `UpBitch`, `GotoMiddle`, `MoveQueen1`-`4`, `QLair_Floor_Break1`/`2`, `QLair_Stairs_Break`, `QLair_Fade_Fog`, `Die_Bitch2`, `Die_Bitch`, `QLair_Cinema1`, `QLair_Cinema_Setup`, `QLair_Setup`, `Queen2_Death`, `QLair_Ending`, `Queen2_Birthing_and_Creation`, `GotoQueenCam`, `GotoPlayerCam`, the essence threads, `Popup_Thread`, `main`); `ai/c_queen1.st`, `ai/c_queen2.st`; `models/c_queen1.tik`, `c_queen1_bigtent.tik`, `c_q2_body.tik`, `c_q2_halo.tik`, `c_q2_t01.tik`-`c_q2_t04.tik`; entities #1-#41, #70-#102, #119, #132-#144.

<a id="v40-ending"></a>
## 40 · Ending film, credits and post-game

| Field | Value |
| --- | --- |
| Trigger | The qlair finale event `ending_ready` (v39) |
| Status | The film decodes and `--movie ending` plays it (Anode). Nothing in gameplay triggers it, and there is no post-game state |
| Size / milestone | M · M8 · T10, T11, DG-14 · draft (re-verify with 13a) |

**Authored flow:** `QLair_Ending` queues the main menu and then the credits menu to open after the film, then plays `ending.roq`. After the film the player sees the main menu with the credits on top.

**Target behaviour:**
1. On `ending_ready`, play `movie::play("ending")` (`video/ending.roq` with its soundtrack; P pauses, holding Enter or A skips). Closing the window during the film quits without an autosave; the v00 double-write must not recur.
2. Then the Credits page (`menu.rs`, `Page::Credits`), with Back going to Main.
3. Post-game state: Main has no "Back to game"/Resume; Escape at Main does nothing; clean quit does not rewrite the autosave; New Game skips the preserve-session autosave (`viewer.rs` ~952-972) and plays the opening film again.
4. Record campaign completion in `save::Campaign` (the `completed` set gains `qlair$first`; add a serde-default completion flag only if DG-14 needs one), and define Continue per DG-14 (recommended: resume the pre-finale checkpoint and show a campaign-complete marker).
5. A headless ending flag: the qlair route check and the chain print `ENDING` once after `ending_ready` and assert that no map transition happened.

**Checks:** `--movie-check` (decode); the chain's ending flag; the qlair contracts; a T10 unit test of the post-game menu state.

**Anode checks** (CAMPAIGN_PLAN.md §8.3 S8):
- [ ] From the `qlair-ending-wait` save: the 10 s wait, the film (screenshot mid-film), Credits, then Back to Main with no "Back to game". A second run holds Enter to skip the film.
- [ ] Quit, then compare the SHA-256 of `auto.json` before and after (unchanged). Relaunch: Continue follows DG-14; New Game plays the opening film.
- [ ] Soundtrack sync is not verified (NoDevice).

**Risks:** the film-window close path; stale post-game menu state; Continue landing in a dead session; the chapter chooser after completion (T10).

**Refs:** `maps/qlair.scr` (`QLair_Ending`); `src/movie.rs:259-397`; `src/menu.rs` (Credits and Resume); `src/viewer.rs` (~952-1035; the clean-quit autosave ~3024); `src/save.rs` (`Campaign`).

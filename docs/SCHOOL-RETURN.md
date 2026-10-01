# First school return — v0.24

This restores `skool1$skool1_start2`, the school-two potion quest's normal exit destination. Run `tools/launchers/Launch-School-Return.cmd` for a fresh standalone return, choosing N if Continue is offered. That fresh visit supplies the earlier campaign weapons, one Demon Die, Lucky Star and Drink Me potion. A real transition retains the player's current resources, selected weapon, owned copies, pickup history and earned quest items; it does not refill health or replace the quest inventory. Existing campaign minimum weapon grants still apply.

## Return world and route

The authored lower return entrance, eight skip platforms, already fallen shelves, open secret passage, removed flying books/recipe and first-visit cast, and return guard/Boojum activations are used. First-visit theatre/book conversations are disabled. The original `mana_skool1` meta-essence appears on the upper theatre landing only on the return; the first-visit Mallet pickup is removed. Previously collected pickup IDs stay collected.

1. Leave the lower entrance, cross the steam area and climb to the theatre's upper corridor. The return-only meta-essence is available near the former Mallet landing.
2. Follow the open library passage. Use the spiral lift and fallen shelves to reach the middle landing and star doors.
3. Approach with the Lucky Star to open the doors and spend the star once. Step fully onto the observatory lift; it rises 632 units in five seconds and carries Alice using collision checks. The upper doors unlock on arrival; open them with E.
4. Cross the observatory's lower room to its western staircase, jump the broken section and climb to the upper walkway above the globe.
5. With the potion still in inventory, entering the original exit contact opens the globe. Alice drinks the potion, shrinks while following the original jump path, then enters `potears1$potears1_start1`. The potion is spent once when drinking begins.

H shows the current objective. Missing rewards leave their gate closed. The separate hidden exit inside the globe is also gated until the shrinking sequence completes, so it cannot bypass the quest. Pause and saves preserve the sequence. Retry/Home after reaching the observatory uses its upper landing so the one-way lift cannot strand the player below it; recovery is briefly disabled during its ascent and the final cinematic.

## Implementation and verification

The implementation reads geometry, models and animation assets from the user's archives and translates the reviewed local school setup, lift and exit behavior into explicit Rust state. It does not execute original scripts or binaries. First and return snapshots remain separate.

- `--school-return-check`: first verifies missing-reward and hidden-exit rejection against the original trigger volumes, then runs continuous ordinary movement/jump/door/combat input from the return entrance to the potion exit. Only the authored final drinking/jump cinematic controls Alice; no traversal teleport, flight, god mode or recovery is used.
- `--school-return-chain-check`: completes school two, retains its actual depleted resources and quest rewards, then walks the return route. Any healing comes from the original map pickup. Checks resource/selection/pickup/reward preservation at entry and inventory preservation at the onward exit.
- `--school-return-render-check`: explicitly staged native captures of the lift, observatory, opening globe, drinking and shrinking. These are visual checks, not traversal proof.
- Save writer and reader run as separate native processes. Return entrance, moving lift, drinking and shrinking fixtures check exact restoration, continued simulation, resource/weapon counts and one-time quest consumption. An actual v0.23 save exercises migration to format 6.

Recorded v0.24 results: school two finished in 20,838 ticks with 18 Sanity; the carried-over return finished in **12,270 ticks**, with **15 jumps, 60 Blade throws, 18 combat damage and 82 Sanity** after collecting the original meta-essence. The onward exit preserved the resulting inventory. The first-school route still finished in 14,822 ticks with 27 Sanity. All 130 unit tests, warning-free static checks, school/shared-event/progression/loadout/story checks and 21 native persistence cases passed.

Re-baselined 2026-09-29: school two now finishes in 25,814 ticks with 100 Sanity and about 4 Will (its fights spend Cards; it was 26,915 ticks before its route moved onto the shared route, `docs/CAMPAIGN.md`). The carried-over return (`--school-return-chain-check`) and the standalone `--school-return-check` both finish in **12,134 ticks, with 15 jumps, 21 Blade throws, 18 combat damage and 82 Sanity**, and the first-school route in 21,033 ticks with 73 Sanity.

A separate native viewer test resumed the drinking save using keyboard input and loaded/autosaved `potears1_start1`. It retained **37 Sanity, selected Mallet, two Dice, pickup history and the cached first-school puzzle**, spent both quest rewards, and allowed normal Will regeneration. Native staged visuals were inspected in Anode. Anode reported no audio output device during this run, so the sound cues were not audibly verified.

## Limits

Camera cuts and the star's presentation are approximations rather than a full original cinematic interpreter. The explicit return state does not reconstruct every optional enemy difficulty variant or original cutscene effect. The next map loads through its authored entrance; completing Pool of Tears and the remaining campaign is separate work. No original game assets are included in the source-review package, and nothing is published.

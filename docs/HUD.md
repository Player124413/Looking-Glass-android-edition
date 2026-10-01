# HUD, resources and inventory — updated for version 0.31

## Boss meters (2026-10-01)

Centipede, Royal Rage's Jabberwock and both Queen forms now show a named meter
at the top of the screen during combat. Centipede loses one sixth per accepted
weak-point reaction. The Queen's second form measures damage remaining to its
authored 1,000-health defeat threshold. The first Jabberwock encounter instead
shows a labelled survival countdown, since that boss is invulnerable.

The new displays use the existing dialogue frame and original font, disappear
outside combat, and read the saved encounter state directly. Pause and restore
need no separate UI clock or save-format change. Existing Duchess, Red King,
Tweedle and Hatter meters remain available. The encounter render checks include
the HUD, with full and partially depleted captures.

Verified on the packaged `A971A442` build: 655 unit tests, all eight boss contract
checks, and the Centipede, both Jabberwock and finale native render suites pass.
The normal launcher and dedicated Jabberwock/finale launchers use this build.
Evidence and the compiled source snapshot are under `private/boss-hud/`.

## Player HUD

The first-school secret now grants a unique 45-second Looking Glass effect. A small remaining-time label appears while active, Alice and her held toy appear faint, and enemy sight cannot acquire her. The timer pauses with gameplay; already-fired projectiles and hazards still cause damage. Recovery clears the effect without respawning the collected item. See [SCHOOL.md](SCHOOL.md).

The second-school quest now tracks Mushroom, Spice Drops, Jumbogrow, Lollipop, Drink Me potion and Lucky Star. **I** lists held quest items above the toy grid; **H** gives the next quest objective. Ingredients are consumed by their transformations, both final rewards gate the return portal, and normal transitions retain those rewards for the current run. See [SCHOOL2.md](SCHOOL2.md).

The Rust viewer now keeps a live player state: Sanity (health), Will, selected toy, ten ownership slots, up to three Demon Dice, and the identities of collected pickups. The compact display reads that state. I opens a paused inventory with original icons; number keys or clicking select owned toys, and the mouse wheel or brackets cycle only through owned slots. H still briefly shows instructions. Menus release the mouse.

## Data and artwork

The two meter frames come from `models/ui/pieces/main/health.ftx` and `skin01.ftx`. Weapon icons come from `models/ui/*.ftx`. These are read from the user's archives at runtime. Version 0.31 uses the authored bar, back and riser TAN meshes and their texture coordinates, with resource levels driving the original riser animation frames. Toy cards use the original folding TAN's open pose. This removes unused opaque texture-sheet areas from the HUD. The fixed projection, static open card pose and base liquid textures do not yet reproduce the original perspective, folding transitions or layered liquid shaders. Placed ordinary resource/weapon pickups still use markers and billboards; power-ups and enemy essence drops have original models.

Inventory, notices, death and air prompts use original frames and fonts. Persistent instruction strips and synthetic meter labels/fills are gone; **H** remains temporary help and **I** shows exact resources and ownership. See [UI_FIDELITY.md](UI_FIDELITY.md).

The equipped-toy and active-power foldouts share their meter's authored origin and scale. Their hinges now meet the upper part of the lower handles, with the handles drawn over the hinges; the cards no longer sit independently at the screen bottom. The toy caption follows its card. Inventory cards retain their separate grid layout.

The order of the ten toy bindings comes from the supplied `default.cfg`. Primary and alternate requirements are read from the `init/server` blocks in `models/w_*.tik`. Small and large health/Will pickups restore 15 and 25 respectively; meta-essence amounts are 15, 25, 50 and 100, read from their TIKI definitions. Only this narrow set of fields is parsed; no original scripts or native code execute.

The original [licensed strategy guide](https://oldgamesdownload.com/manual/american-mcgee-s-alice-windows-strategy-guide-english/) describes red Sanity, blue Will, shared weapon resources, restorative meta-essence and gradual Will recovery on easier difficulties. Later read-only archive/binary research supplied the implemented difficulty, power and essence rules; see [ITEMS.md](ITEMS.md) for exact values and remaining limits.

## Current behaviour

- Exploration begins with earlier campaign weapons and 100 of each resource. Each map/visit has a profile; the default school first visit has Blade and Cards. This assumes full earlier exploration without executing campaign scripts. See [LOADOUTS.md](LOADOUTS.md).
- Sanity and Will are clamped to 0–100. Falling faster than 550 units/second costs `floor((speed - 550) * 0.15)` Sanity. Ordinary jumps and the initial landing after a map start/Home are safe. These thresholds and the 100-point scale are provisional.
- Leaving level bounds or touching an authored falling-death volume ends the attempt and offers retry. Ordinary nonfatal drops retain provisional landing damage. Returning from F4 flight does not grant landing protection.
- Passive Will recovery follows difficulty without a spending delay: Easy restores 0.3/second up to 100, Normal restores 0.1/second only up to 10, and Hard/Nightmare do not regenerate. Sanity does not passively recover. Recovery stops in menus, pause, free flight or at zero Sanity. Weapon selection, busy clicks and unsupported attacks never debit Will. Cards, croquet balls and Demon Dice spend Will when an action is accepted; the owned Pocket Watch spends one Will on accepted activation. See the original-code recovery audit in [ITEMS.md](ITEMS.md).
- Walking near a supported map pickup collects it if it changes player state and a world trace is unobstructed. Full-resource pickups stay available. Weapon pickups unlock/select a new toy and refill Will; Dice ownership caps at three. Weapon refills are a provisional common rule.
- Each collected item is remembered by map name and entity index across normal level exits/revisits. Resources, selection and inventory carry through exits, with missing earlier weapons filled in. The Tab level chooser starts a fresh visit with its baseline inventory and empty pickup history. F5 saves this state; F9 or Continue restores it after restart. See SAVES.md.
- At zero Sanity, movement stops and Enter retries at prior safe footing or the entrance. R provides the same recovery during play; Home returns to the current entrance. All restore resources while retaining ownership and the pickup ledger. See [RECOVERY.md](RECOVERY.md).

## Scope and limits

The version-0.28 item audit finds 338/314/293/293 supported available pickups across the 36 maps on Easy/Normal/Hard/Nightmare. This is a data count, not a claim that every item is reachable without scripts. Difficulty flags, supported enemy drops, power-ups and the Watch are implemented; map-specific scripts and full campaign progression remain incomplete. Persistent saves are supported; see [ITEMS.md](ITEMS.md) and [SAVES.md](SAVES.md).

The inventory displays weapon definition costs; the older three toys still use provisional gameplay tuning described in [COMBAT.md](COMBAT.md). Both Dice buttons use the shared summon action and 40-Will cost. Ice Wand/Eye Staff charging and continuous consumption need their combat logic. Blade, Cards, Mallet and Dice have implemented attacks; the Watch's time freeze/recharge and power-up timers are also active. See [WEAPONS.md](WEAPONS.md) and [ITEMS.md](ITEMS.md).

## Validation and preview

`--hud-check` reads all ten weapon definitions and eight resource definitions, inventories all maps and checks resource transitions without a graphics context. Synthetic tests cover ownership/cycling, Dice limits, resource clamping, death/reset, frame-independent regeneration, ignored TIKI comments/client fields, single-use pickups, obstruction checks and actual physics landing speed feeding damage.

`--hud-preview` explicitly starts with all toys and partial sample resources for visual checks. Normal launches never enable that state. As in normal play, Will regenerates while active; I freezes it for inspection. Captures and original-art research remain local under `private/` and are excluded from source packaging.

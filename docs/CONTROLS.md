# Rebindable controls, controllers and multi-touch

Open **Escape / Start → Settings → Controls**. Select the heading to switch between **Keyboard / Mouse**, **Controller**, and **Touch Controls**. All changes are staged: **Apply** saves them, **Cancel** discards them. **Reset** restores the selected device's defaults. The original parchment, buttons, fonts and menu layouts are retained.

Select an action and press its new input. Keyboard bindings accept letters, numbers, punctuation, modifiers, navigation/numpad keys, F5–F11, left/right/middle mouse and either wheel direction. Controller bindings accept face buttons, triggers, bumpers, stick clicks, D-pad and Back/View. Assigning an occupied input swaps the two actions. Delete clears a controller binding while capturing. Escape cancels capture; Start also cancels controller capture. Escape, console (`~`), F1–F4 and F12 keep their interface/developer functions. Menu navigation always uses its fixed controls, regardless of gameplay bindings.

There are five pages covering 38 actions. Previous/Next, Page Up/Down, mouse wheel and controller LB/RB browse pages. The controller's sixth page adjusts look sensitivity, inverted vertical look and the radial stick deadzone. Left stick movement and right stick look remain analog; stick clicks can be rebound. Smaller movement deflections walk more slowly, including when swimming. Mouse sensitivity and invert remain on the Game page.

Attack bindings support tapping and holding through the shared [weapon action rules](WEAPON_ACTIONS.md). Hold to repeat when ready; release before resuming after menus or a focus/controller interruption. Both-button priority, queued toy switches and Will debits are shared by mouse, keyboard and controller.

## Default controller layout

| Input | Gameplay |
| --- | --- |
| Left stick | Move / swim / swing on a rope |
| Right stick | Look |
| A | Jump, climb, swim up |
| B | Swim down / descend |
| X | Interact, talk, advance dialogue |
| Y | Inventory |
| RT / LT | Primary / alternate attack |
| LB / RB | Previous / next owned toy |
| Left stick click | Run / walk modifier |
| Right stick click | First / third person |
| D-pad up / down | Cheshire hint / help |
| D-pad left / right | Safe footing recovery / pause |
| Back / View | Unbound by default (assign **Choose level** here if you want the chapter chooser) |
| Start / Menu | Main menu |

In menus: D-pad or left stick browses, A selects, B returns, Left/Right adjusts a setting. In inventory, browse owned toys with D-pad, stick or bumpers and press A to return with the selected toy; B also returns. In chapters, Y changes difficulty and A begins a fresh visit. On the startup prompt, Up/Down chooses Continue/New and A confirms. A resumes a paused game, retries after death, or (held) skips a supported cinematic. These contextual menu/retry/skip controls stay fixed even if gameplay A/B are rebound. The existing cinematic release-before-skip safeguard still applies. Start always offers Save/Load; quick save/load have no default pad binding but may be assigned.

The game uses the Windows system [XInput API](https://learn.microsoft.com/en-us/windows/win32/xinput/getting-started-with-xinput). Xbox controllers and devices exposing XInput are supported. Native PlayStation/DirectInput/HID protocols, rumble, motion controls and local multiplayer are not implemented. No controller driver is installed by the game. On non-Windows builds, keyboard and mouse remain available.

Controller layout revision 2 removed the default Back/View → chapter chooser shortcut: starting a visit from there sends Alice back into earlier areas. Existing controller files are updated once, and only that default is changed. You can assign **Choose level** to any button again in Controls.

Controllers can be connected during play. A disconnected active controller pauses the game; keyboard and mouse remain usable. Release buttons and centre the sticks after connecting, returning to the window or leaving an overlay. Held input is suppressed across these boundaries so menus, save loads and reconnections do not fire a weapon or move Alice unexpectedly. Prompts follow the most recently used input and the chosen bindings.

## Controller setup in the Android launcher

On Android, the launcher has a **Controller Setup** button in its top bar. It edits the same controller bindings as **Settings → Controls → Controller**, with a simpler layout for phones and gamepads:

- **Categories** (Move, Combat, Toys, Actions, Other) show the 38 actions, eight per page. Tap a row, or use D-pad Up/Down and LB/RB (tabs) with A to select.
- **Assign**: after selecting an action, press the button on your controller, or tap the button on the right side. Button groups are shown as triggers and bumpers, face buttons, D-pad, and sticks and menu.
- **Swap**: assigning a button that another action already uses swaps the two, the same as the in-game Controls page.
- While the screen waits for a press, the next button you press is assigned. Start or Escape stops waiting.
- **Unbind selected** clears the action's button. **Start** is reserved for the menu and can't be assigned.
- **Default**, **Attack: X** and **Attack: RB** change the draft layout. For example, **Attack: X** puts Primary attack on X. The action that used X receives Primary attack's previous button.
- **Save & Close** writes `private/preferences.json`. **Cancel**, B or Back discards the draft. Nothing is saved until you press Save & Close.

Keyboard and mouse bindings are not changed on this screen. Touch HUD layout stays under **Edit Touch HUD**.

## Multi-touch controls (Android & touch displays)

On Android (or when **Touch Controls** is enabled in **Settings → Controls**), Looking Glass displays a context-sensitive multi-touch HUD:

| Touch Zone / Button | Gameplay Function |
| --- | --- |
| Left thumb zone (floating stick) | Move / walk (inner ring) / run (outer ring `>78%`) / swim / swing on ropes |
| Right half of screen (drag) | Smooth relative camera look and vertical pitch |
| Primary (`Attack`) | Primary toy attack (`Mouse 1`); drag while holding to aim while firing |
| Alternate (`Alt`) | Secondary toy attack (`Mouse 2`); drag while holding to aim while firing |
| Jump / Rise | Jump, climb ropes, or swim upward (`Space`) |
| Dive / Down | Swim downward or descend on ropes (`Ctrl`, shown contextually) |
| Use / Talk / Skip | Interact, grab ropes, talk to NPCs, or advance dialogue (`E`, shown contextually) |
| Top utility bar | `Menu` (`Esc`), `Toys` (`I` inventory), `Prev` / `Next` toy, `Hint` (`C` Cheshire Cat), `Cam` (`V`), and `QSave` (`F5`) |
| Android Back button | Closes Inventory / Chapter Chooser, or opens / backs out of the Main Menu |

In **Settings → Controls → Touch Controls**, you can configure the Touch Overlay mode (`Auto`, `Always On`, `Off`), Touch Sensitivity (`0.2`–`3.0`), Invert Touch Look, Button Size (`0.6x`–`1.6x`), HUD Opacity (`20%`–`100%`), and Left-Handed Layout. See [docs/ANDROID.md](ANDROID.md) for full details.

## Persistence and saves

Controls live in `private/preferences.json`, independently of campaign saves. `LOOKING_GLASS_SETTINGS_DIR` selects another settings folder for testing. Old eight-key preferences migrate, preserving the user's bindings and other options. If a new action's default is already taken by an old custom binding, the new action starts as None and can be assigned in Controls.

The original Load/Save screen now offers four manual slots plus quick save and autosave. Select a thumbnail or its small button, then the large Save or Load button. An empty manual slot creates a save immediately. Occupied slots require overwrite confirmation; unreadable saves cannot be loaded. The autosave slot is reserved for the game and cannot be overwritten manually. Manual slots never get replaced by quick save or autosave. The initial Continue choice includes all six slots when looking for the newest usable save. `--load slot1` through `--load slot4` also work. No save-format change is required.

## Verification

Unit checks cover radial deadzones and analog magnitude, trigger/button edges, menu repeat, connection/focus/menu suppression, preference migration, key/mouse/controller conflicts, serialization, reserved controls and distinct safe slot paths. Native Anode tests use isolated `private/controls-settings` and `private/controls-native` folders, never the player's settings or saves. The test Xbox controller is confined to the Anode session through HidHide. Verification results and remaining limits are recorded in VALIDATION.md.

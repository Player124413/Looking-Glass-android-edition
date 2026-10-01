# Aim pointer

The aiming point is a world-space particle glow, using the local `fx_emitter_target` model and its original sprite material. It follows Alice's aim and stops at the nearest supported actor or solid surface, including moving doors. The glow sits at 90% of the contact distance to keep it in front of the surface; open space uses the weapon's bounded range. Rendering respects depth and fog.

The pointer uses 0.2 of the initial restoration's particle size and 0.1 of its per-particle opacity. This keeps overlapping additive particles from producing a large saturated flare. The adjustment is local to the pointer, including the red variant; other effects keep their existing appearance.

Primary weapon declarations select the marker and its range. The default is blue with a 1000-unit range. Cards and Jacks also use blue, as do the Mallet and Eye Staff. The Blunderbuss uses the red variant. Weapons declaring reticle type 0 omit the marker. These declarations are read from the user's local assets. No assets or original script text are embedded in source.

The marker is hidden during performances, menus, free flight, death and unavailable weapon movement states. Pausing freezes its cosmetic clock. Changing weapons, hiding the marker or moving its endpoint a large distance clears the short particle trail. Its state is cosmetic and does not change saves, weapon contacts, costs, auto-aim or route/event identifiers.

This restores the endpoint indicator. Native target-orbit pairs for certain weapons and the native fade after ten seconds without firing remain fidelity work. The port uses its existing player eye/aim and collision targets; the original's weapon minimum range does not allow this indicator to pass through a nearby obstruction.

`--targeting-render-check` verifies the original blue/red artwork, disabled weapon/context and depth occlusion. Unit fixtures verify nearest actor/world contacts, a target behind a wall, clear-space range, embedded starts and weapon-rule parsing. Evidence is retained under `private/aim-pointer/`.

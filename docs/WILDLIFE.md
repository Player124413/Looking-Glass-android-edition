# Phantasmagoria, Spiders, Jabberspawn and ambient children

This batch adds shared controllers for eleven model variants: Phantasmagoria,
ground/wall Nightmare Spiders, the three awake and three sleeping Jabberspawn
skins, and both Walkrock sizes. Existing scene owners retain their actors.

## Playable behaviour

- Phantasmagoria: swept flight, alert, drain/chain windup, attack and recovery,
  pain, two-part death, ice retirement and large essence drops. Its supplied
  alpha timeline now uses ordered translucency, including the low resting alpha.
  Additional head surfaces follow the attack/death visibility cues.
- Spiders: protected wall idle, checked departure, ground pursuit, impale,
  poison spray, swept pounce and landing contact, short ceiling-web manoeuvre,
  pain, normal/ice death and medium essence drops. Gas remembers each victim
  across its multiple source cues, preventing repeated hits from one spray.
- Jabberspawn: variant health/skins, sleep and wake, pursuit, three distinct
  melee contacts, charged beam, pounce, pain and three death variants. Decapitation
  hides the supplied surfaces and emits the attached blood burst. Pipe smoke,
  mouth spray and death emissions have separate histories for each actor.
- Walkrocks: rise when Alice approaches, flee, then settle. They are harmless,
  cannot be targeted or killed, and do not produce drops.
- Funhouse child: follows the four authored waypoints repeatedly.
- Hedge child: runs to the reviewed handoff, then activates the separate hidden
  chase actor. The chase actor follows within the supplied distance bands, stops
  behind cover, and observes the five-second monster-contact hold. `SeekEnd`
  moves the first actor to its source endpoint. The commented-out exit cinematic
  and disabled exit triggers are not enabled by this controller.

Controllers advance at 120 Hz. Pausing does not advance attacks, routes or
particles. Saved state includes cue/victim bits, sleeping/wall phases, web and
beam state, movement, delays, random choices and escort progress. Old resident
slots remain stable; newly supported delayed placements append as generation 5.
Effects are cleared on restore and on level replacement. Particle histories use
the existing finite particle budgets; attachment histories are bounded to eight.

Hatter1's reviewed spider touch callbacks wake their named actors. Spiders 3/4,
cinematic ground-spider spawning, Funhouse wall-smashing ghost entrances and
Jabberwock boss waves remain with their unfinished scene logic. No recurring
boss wave is converted into an automatic one-time spawn.

## Fidelity limits

The supplied native implementations of `PhantasmAttack`, `SlingWeb` and
`JumpAttack` are absent. Current bounded substitutes are explicit:

- A close ghost attack drains 10 Will once; a chain applies a short pull while
  sight remains clear. The original drain rate, stun rules, controller-bone
  stretching and decorative chain geometry are not reproduced exactly.
- Webbing uses a checked ceiling anchor, a short lift and a swept pounce.
  Jump horizontal speeds come from the data; vertical arcs and steering are
  approximations. Wall spiders with obstructed departure hulls stay hanging.
- Jabber beam damage is a single five-point pulse during its half-second beam
  window. Native repeat cadence and precise beam appearance remain approximate.
- Poison uses the supplied immediate damage; native poison status behaviour is
  not independently reconstructed. Attack volumes interpret source dimensions
  as full dimensions, with world collision checked before victim contact.
- Sprite blood is supported. The decapitation effect's emitted mesh-gib swarm
  and its original bounce simulation remain unsupported.
- Ground spiders now retain the last visible target for three seconds and share
  a supported-hull detour planner with Army Ants. Routes persist across frames
  and saves, preventing repeated sidestep oscillation. Searches are limited to
  96 expansions, 384 candidate nodes and 32 retained waypoints, with a half-second
  retry interval. Walls, doors, unsafe liquids and unsupported gaps remain
  impassable; actors do not teleport. This is bounded local navigation, not the
  original pathfinding network.
  Companion routes use the existing actor collision mask and check centre
  support on narrow, chamfered steps while sweeping the whole body. They retain
  current doors and moving obstacles and never teleport to bypass a blockage.

## Checks and use

`--wildlife-check` checks real assets, placements on all four difficulties,
attacks, pause and restored continuation at 30/60/144 Hz. It also traverses the
two child routes on their original maps. `--wildlife-save-write` followed by
`--wildlife-save-read` checks 78 saved states in separate processes.
`--wildlife-render-check` captures the eleven model variants; `--render-fx-check`
checks ordered alpha, low-alpha visibility and opacity isolation.

Use `tools/launchers/Launch-Creatures.cmd` for the separate NPC candidate and isolated saves.
Its private evidence manifest records the exact executable, successful checks,
captures and remaining native limitations. Captures are staged inspections;
they do not certify a complete campaign playthrough or original-game parity.

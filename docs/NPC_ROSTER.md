# NPC roster completion

Working inventory for the request to cover every NPC, started 30 September 2026.
This distinguishes a working character controller from completion of every map
appearance and story sequence. The older campaign roster contains stale Missing
labels; current source and the family documents take precedence.

| Group | Current implementation | Remaining coverage |
| --- | --- | --- |
| Club/Diamond guards | Opening encounters and shared later-map combat | Remaining authored ambushes/patrols |
| Heart/Spade guards | Shared melee/ranged combat, pain, deaths, freezing, Spade torso cuts and persistence | Scripted recurring/cinematic waves and presentation approximations in CARD_GUARDS.md |
| Boojums | Opening encounters and shared later-map combat | Scripted patrols and unreviewed spawns |
| Ladybugs | Pool encounter and shared later patrols/ambushes | Garden2 cinematic spawns and mixed Garden1 callbacks |
| Army Ants/Corporals | Shared combat and scene-owned Pool guards | Remaining script movement, grab/fling and blind fire |
| Red chess pawn/knight/bishop/rook | Shared combat and reviewed ambushes | Scripted duels and progression gates |
| White chess allies | Scene/decorative actors | Factions, authored duels and scene opt-in damage |
| Fire Imps | Shared combat | Remaining unlinked/script-owned spawns |
| Clockwork Automata | Shared combat and Hatter boss waves | Remaining glass-wall entrances |
| Bloodrose/Evil Mushroom | Shared growth/suction/ranged combat, deaths and persistence | Documented Digest/aim approximations and scene-specific spawns |
| Snark/Bite-only/Fire Snark | Shared swimming, bite, surface tongue/spit, acid/fire shots, pain, deaths and persistence | Local-steering/TongueGrab approximations and unreviewed scene spawns in SNARKS.md |
| Antlion/Larva | Shared Antlion pursuit/burrowing/ambushes; Larva leap/attach/drain, deaths and persistence | Centipede-owned Larva spawning and movement/attachment approximations in ANTLIONS_LARVAE.md |
| Magma Man | Shared three-form combat, extending fire punch, fireballs, rock attacks, pain/deaths, freezing and six gated placements | Cooling/steering/launch approximations and presentation limits in MAGMA_MEN.md |
| Phantasmagoria | Shared flight, Will drain/chain pull, translucency, pain/death and persistence | Native attack/controller-stretch approximations and wall entrances in WILDLIFE.md |
| Nightmare Spider | Shared ground/wall departure, impale/gas, pounce/web, deaths and Hatter wake triggers | Native web/poison approximations, obstructed wall hulls and cinematic spawns in WILDLIFE.md |
| Jabberspawn variants | All six variants: sleep/wake, melee, beam, pounce, pain/deaths and persistence | Native beam cadence, mesh gib particles and boss-owned waves in WILDLIFE.md |
| Walkrocks | Both sizes rise, flee and settle; harmless and untargetable | Original local-steering differences; scene-owned Duchess actors preserved |
| Insane children | Opening support, school circuit, Funhouse runner and Hedge handoff/follow/hold | Remaining story staging and disabled/commented-out Hedge exit logic |
| Duchess | Playable encounter | Existing documented presentation limits |
| Centipede | Missing encounter | Weak point, stages, larvae, defeat and story gate |
| Red King | Playable Checkmate in Red encounter: melee, health-stage magic, counter, pain/deaths and Funhouse departure | Beam rendering, steering, health-condition interpretation and scene choreography approximations in RCHESS1.md |
| Tweedles and minis | Missing encounter | Both bosses, splitting limits and defeat gate |
| Mad Hatter | Playable About Face battle: attacks, clock phases, Clockwork waves, breakdown, Watch platforms and Gryphon escape | Steering, tea radius, platform support and choreography approximations in HATTER2.md; preceding Hatter1 scenes |
| Jabberwock ground/flying | Both major encounters: Lair survival/waves/Eye Staff and Royal Rage flight/ground combat/defeat/farewell/castle exit | Native steering, beam and choreography approximations in JABBERWOCK.md |
| Queen1/Queen2 and parts | Shared main finale verified end to end on Normal/Hard, watched and skipped | Original attack/effect parity findings in QLAIR_AUDIT.md |
| Friendly story characters | Reviewed opening scenes and E bindings | Remaining original conversations/staging by visit |

Implementation order: broaden reusable existing controllers; plant/swimming
families; remaining terrestrial/flying families; ally/escort/ambient behaviour;
bosses and friendly scene gaps. Each family needs original-data validation,
contact/collision checks, pause and save continuity, death/reward checks and
native captures. All source data and captures remain private. A staged native
capture is not a natural campaign route proof.

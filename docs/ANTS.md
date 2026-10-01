# Army Ant enemies

`tools/launchers/Launch-Ants.cmd` opens the tested Ant build in Pool of Tears when that local
build is installed. It uses separate Ant playtest saves. The regular launcher
continues to follow the shared playtest build maintained by the other chats.

Soldiers and Corporals now share one saved combat controller. Ordinary eligible
placements in Pool of Tears, Hollow Hideaway, Garden and Centipede maps acquire
combat through the existing NPC cast. Difficulty exclusions, hidden placements,
script-only spawn flags and scene ownership remain in force. The Pool boulder
pushers still activate at their existing handoff; the two house guards still own
the Bill gate. No extra enemies or campaign gates are invented.
Garden actors and Centipede runners/guards whose scripts disable AI retain that
waiting state. They can take damage, react and die, but their scene-controlled
movement and later activation still await those map sequences.

Ants turn toward visible opponents, approach on supported ground, keep bounded
detours around obstacles, retain a last-seen position for three seconds and
retreat when wounded. Soldiers fire swept musket rounds; Corporals throw bouncing
grenades. Melee and firing contacts use source animation frame timings. Cover
blocks acquisition and damage; melee also checks height, distance and facing.
They participate in the shared weapon target, recoil and summoned-demon systems.
Combat uses bounded 120 Hz steps and stops when its gameplay clock stops.

Both models load three pain and three ordinary death animations. Further damage
does not continually restart the current pain clip. Lethal hits immediately remove
the target, interrupt its pending attack and play one death. Ice preserves a
frozen pose and material. Corpses finish their pose, remain for five seconds, then
shrink over two seconds. Existing projectiles finish their bounded lifetime.
Soldiers drop medium essence and Corporals large essence, matching the model
metadata. The visit reward ledger prevents repeat drops after restoration.

Save format stays at 12. Optional NPC Ant state preserves health, action/event
clock, variant, pursuit memory, recoil, projectiles, explosions and corpse age.
Older decorative Ant records acquire fresh combat at their saved location;
older scene-owned Ant records deserialize with defaults for the added fields.
Authored NPC identities and reserved scene hit ranges are unchanged.

This is a playable combat implementation, with local steering rather than a full
navigation graph. The soldier's grab/fling, blind-fire and dismemberment variants
are still deferred. Corporal grenade launch arc, bounce damping, blast radius and
the shared corpse disappearance interval are approximations. Grenades use the
authored projectile model, explosion sound and a bounded visual flash; the full
nested explosion particle recipe is not implemented here. Antlion is a separate
enemy family and is outside this change.

`--ant-check` audits the six source maps and runs source-animation combat at
30/60/144 Hz. `tools/test_ants.ps1 -Executable <candidate>` runs this plus staged
soldier/Corporal art, independent-process restart checks and both existing scene
owner checks. Captures and results stay under `private/ants/`. The native views
stage poses explicitly; they are not evidence of a complete campaign playthrough.

## Verification — 30 September 2026

The installed Ant candidate (`1767F5EBBD3E1545214EFC059D58754DADC51494E1248A10A2D422DA1FDB7728`)
passes 505 unit tests, strict Clippy, formatting, source/provenance audits, the
39-visit deterministic registry and NPC asset checks. All six Ant checks pass,
including 18 distinct staged pose captures and two independent-process Ant save
futures. The existing Pool boulder and house/Bill scene checks also pass.

Broader native checks pass for billboards, shared rendering effects, the material
corpus, actor visibility, all 78 overlapping level replacements, all 169 campaign
restart cases and the existing actor-placement views. The first visibility
attempt needed its capture directory created; the general restart command was
retried with the correct CLI flags. Neither required a game-code repair.
Native audio was disabled, so audible mixing is not claimed.

The local launcher uses a versioned executable and separate Ant playtest saves.
Evidence and the consolidated result are retained under `private/ants/`; the
shared playtest launcher and the running player's executable are untouched.

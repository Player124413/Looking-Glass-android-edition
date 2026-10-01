# Antlions and Larvae

Antlions now use shared combat in `centipede1` and the reviewed Garden2 touch
ambushes. Surface and underground variants retain their distinct original melee
cues. They pursue, slash, pinch or sting, burrow after attacks, emerge, react to
hits and play normal or frozen deaths. Buried actors cannot be hit. Defeat uses
the existing one-time Medium reward ledger.

Larvae use their original model, jumping/attachment/draining and death clips,
7 health and Small rewards. They leap into contact, follow the contacted host,
drain 2 health per `attack_suck` loop for at most 3 seconds, then detach and die.
A damaging hit follows the source's fatal pain branch. Freezing releases the
host, as do cover, teleportation, removal of a summoned Demon or disabled Alice
targeting. Damage routes to the attached host; it cannot jump to another target.
The original red light follows each live larva.

Use `tools/launchers/Launch-Antlions.cmd` for the campaign's Centipede level and
`tools/launchers/Launch-Larvae.cmd` for an explicitly staged, playable Larva encounter in that
level. The latter disables persistent saves. The ordinary Antlion launcher uses
its own save folder. Existing launchers and player saves are preserved.

The supplied maps contain no placed `c_larva` actors. `c_centipede` spawns them
from animation events. The Centipede encounter and its spawning/progression
sequence remain unfinished; this change does not inject larvae into normal
campaign maps or claim the boss is complete.

## Source facts and approximations

- Antlions: 120 health, 1000 vision, original slash/pincer/sting frame contacts,
  all three pain clips, both death clips and frozen frame 9. Underground slash
  contacts are frames 9/17 instead of 5/10/18; sting is frame 9 instead of 4.
- Movement uses original clip travel speed with local steering. Burrowing follows
  the existing walkable ground and solid-wall checks; it cannot tunnel through
  level geometry or teleport across ledges. Emergence waits for clear supported
  ground. Full authored navigation and original wander randomness are approximated.
- The three source dirt emitters follow animated skeleton tags during the two
  burrow clips. Histories are owned per actor and reset on phase/restore. No dust
  continues while buried or dead. This ends old particles at phase boundaries.
- Underground source sound paths refer to `ant_lion`; the supplied recordings are
  under `antlion`. The absent `anl_painfolding.wav` uses `anl_painsquirm.wav`.
- Larva collision uses source server bounds; its separate model scale remains
  visual. The host offset is retained from swept contact rather than reconstructed
  from Alice's animated `tag_back`; exact back attachment and pre-attachment
  staging are approximated. The authored `attack_death` clip presents detachment.
- Source acid immunity is not separately expressible by the current shared damage
  categories. Gore fragments and actor-to-actor avoidance remain unsupported here.

## Verification

`--resident-check` includes source health/clip/cue/emitter audits, real Antlion
placements on all difficulties, and 30/60/144 Hz combat, pause and restoration.
`tools/test_burrow.ps1` runs native captures and fresh-process saves in an Anode
background desktop. It must not be run on the foreground desktop.

Evidence is recorded with the immutable candidate in `private/burrow/`.
Native captures are staged views of supplied maps, not a natural campaign route
playthrough. Sound paths are checked against supplied assets; audio playback is
not verified in the background desktop.


Verified on 30 September 2026:

- Final candidate `68c67ddf`: 570 unit tests; all-target Clippy with the existing
  repository allowances; all-target shared-main compile; original-data checks,
  34 Antlion placements across difficulties, 84 resident cast audits, and earlier
  Ant/Chess/Imp/Clockwork regressions. The model audit loaded 63 models/453 clips.
- The 39-visit registry rebuilt each visit twice identically. No stored golden
  snapshot was available, so this proves determinism only.
- Native candidate `1f1cb3c9`: 32 staged captures, visible-model retirement checks,
  and 24 fresh-process cases (20 enemy states plus 4 Garden2 trigger branches).
- The final candidate adds preview recovery/reward reset and a closer staged
  Larva camera. The final preview/retry smoke test and this batch's full-level
  replacement check could not run because the shared background desktop stayed
  occupied after repeated lease attempts. These are unverified, not passes.
- Both source-provenance and staged-file audits passed. Audio playback and a
  natural campaign route were not tested. The full evidence manifest records
  both executable hashes. The isolated candidate contains prior NPC batches;
  other concurrent campaign changes remain integrated in the shared main source.

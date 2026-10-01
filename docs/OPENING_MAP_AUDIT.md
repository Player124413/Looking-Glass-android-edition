# Opening-map actor and dialogue audit

Scope: `gvillage` and `pandemonium`, including their cinematic handoffs, automatic
dialogue, optional Talk repeats and saved actor identities. The private opening
research inventory and playtest reports 3 and 6b were checked against the shipped
map/script identifiers and the current Rust controllers.

Confirmed defects and corrections:

- Village `essence_cat` and `bridge_cat` were rendered at their stored placements.
  Both are initially transparent in the original setup. No reachable caller for
  either auxiliary scene was established, so they remain offstage.
- `cat_shrink1` became visible at the start of the third Gnome conversation because
  generic NPC presentation exposed every actor in the entire dialogue sequence.
  The village cinematic now exclusively owns that Cat, the opening/Blade Cats and
  the Rabbit. Its original Cat shot remains visible; no extra placed copy is drawn.
- `climb_cat` and `exit_cat` were shown at their storage coordinates. They now use
  the authored `climb_cat_node` and `exit_cat_node` presentation markers.
- The second, third and fourth Gnomes reverted to their stored positions after a
  scene. All four Gnome handoffs now keep the authored final markers, including
  after skipping or loading an older save.
- An automatic Gnome conversation did not arm the cooldown used by E-initiated
  conversations. Another E press immediately after completion could restart the
  whole exchange. Its existing saved cooldown now stays armed for one second of
  gameplay after dialogue. Intentional later replays display **Talk again** and
  still cannot repeat quest effects, cameras or completion callbacks.

The snapshot's actor list, Spawn identities, story indices and format version12
are unchanged. Offstage ownership is rebuilt from the map; scene markers override
old saved presentation positions. The existing saved NPC cooldown covers reloads.

Pandemonium's warning, return, flight and minecart actors already have one dedicated
owner. Their stored NPC copies remain hidden. Rope/Cards Cats appear only for their
own dialogue. The two village exit hints use different recordings; Pandemonium's
four departure lines and Fortress1's arrival recordings are also distinct. No
authored lines were removed to conceal an unconfirmed duplicate.

`--opening-dialogue-check` checks all twelve opening-map conversations at 30, 60
and 144 Hz: repeated activation, one start per original recording, one completion,
pause and serialized mid-line resume. `--opening-audit-check` adds actual renderer
pixel probes for unwanted/active actors, real scene watch/skip handoffs, old actor
snapshot restoration and immediate versus deliberate Talk. The ordinary village
and watched/skipped Pandemonium route checks remain separate traversal proof.

This is an actor/dialogue audit, not a claim of complete original-game parity.
Auxiliary village scene activation, exact Cat fade/gesture staging for standalone
hints, and the separately reported Pandemonium key aura remain outside these fixes.
The specific airship-repeat report remains unreproduced; the test retains all four
authored departure recordings and checks their individual starts.

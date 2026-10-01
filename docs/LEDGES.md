# Airborne ledge recovery

Jump toward a reachable edge to catch it automatically. Alice holds the edge until another input: release and press forward again, or press Space, to pull up; move sideways to shimmy; press Back or Ctrl to drop. Attacks and their costs are suppressed while hanging. A drop briefly prevents immediate reattachment.

The detector checks two hand-sized probes against static collision, a near-horizontal top, the hand-height interval crossed in the current physics step, fall speed and body clearance. Both hands need support. Pull-up checks the vertical and forward paths and rechecks collision throughout its 2.35-second animation. Sloped rock shelves use the whole standing body to find the landing height farther inland; the hand-contact height alone is insufficient. The original flat-ledge path is retained. Low ceilings and obstructed landings keep Alice hanging. Moving supports are excluded. The existing low-ledge and swimming-bank climb remain available.

The new optional state uses save format 12. Contact, phase, forward-input history and release cooldown are saved and validated against the restored world. Ordinary saves with no ledge fields load normally. No event keys or visit/hit identifiers change.

Water Logged's narrow beveled pipe rim permits each hand probe to continue 8 or
16 units across connected upward-sloping geometry to a flat cap. Missing
support or a vertical face stops the probe. Both hands and the full pull-up
clearance are still required. `--tower2-check` proves the actual jump, catch and
saved mid-pull continuation; the full Tower route climbs this rim normally.

`--ledge-check` exercises six sloping Pool of Tears shelves plus real ledges in fortress1, garden1 and wforest, including saved continuation. `--ledge-render-check` shows actual Fortress and Pool edge catches, held weapon and pull-up, checks attack suppression and the single authored climb cue. The save suite includes hanging, shimmy and mid-pull-up cases in separate writer/reader processes. Geometry fixtures cover missed hand support, excessive fall speed, blocked headroom, fixed-rate continuation, release and pause through the existing fixed-step clock.

Limits: these are staged recovery probes, not proof that every campaign edge is reachable. Current body clearance and the checked pull-up trajectory use the port's collision shape; exact native per-frame root motion, hanging pain, moving supports and corner turning remain unimplemented. The port keeps its usual following camera.

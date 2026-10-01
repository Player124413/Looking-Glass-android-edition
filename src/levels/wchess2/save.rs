use super::*;
pub fn stage(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
    _: &mut Stats,
) -> Result<Story> {
    let mut story = Story::load(a, "wchess2");
    let r = owner(i)?;
    *r = Castle::load(a, map)?;
    *p = Player::new(r.data.point("alice_queen_dest1").translation);
    r.saved.pending = Some(scene::Kind::Queen);
    r.start_scene(p);
    if case == "wchess2-queen-mid" {
        for _ in 0..600 {
            r.advance(crate::movement::FIXED_DT, map, w, p, &[])?;
        }
    } else {
        r.finish_scene(map, w, p, true)?;
        r.saved.pending = Some(scene::Kind::King);
        r.start_scene(p);
        if case == "wchess2-king-mid" {
            for _ in 0..18000 {
                r.advance(crate::movement::FIXED_DT, map, w, p, &[])?;
                if r.prepare_story(&mut story) {
                    story.tick(crate::movement::FIXED_DT, false);
                }
                r.sync_story(&story);
                for n in story.take_completed() {
                    r.dialogue_complete(&n);
                }
                if story.progress(TALK).is_some_and(|(n, t)| n == 3 && t > 0.4) {
                    break;
                }
            }
            ensure!(
                story.progress(TALK).is_some_and(|(n, _)| n == 3),
                "King fixture did not reach fourth line"
            );
        } else if case == "wchess2-exit-open" {
            r.finish_scene(map, w, p, true)?;
        } else {
            anyhow::bail!("Unknown Castling fixture");
        }
    }
    i.sync(w);
    Ok(story)
}

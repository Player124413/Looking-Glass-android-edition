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
    let r = owner(i)?;
    *r = Realm::load(a, map)?;
    r.saved.pending = None;
    r.saved.intro = true;
    r.saved.age = 100.;
    match case {
        "wchess1-intro-mid" => {
            r.saved.intro = false;
            r.saved.pending = Some(scene::Kind::Intro);
            *p = Player::new(r.data.point("wchess1_start1").translation);
            for _ in 0..360 {
                r.advance(crate::movement::FIXED_DT, map, w, p, &[])?;
            }
        }
        "wchess1-bishop-square" | "wchess1-bishop-move" => {
            r.saved.board = Some(Board {
                piece: Piece::Bishop,
                node: 0,
                moving: None,
            });
            *p = Player::new(r.data.square(Piece::Bishop, 1) + Vec3::Z * crate::collision::SKIN);
            if case.ends_with("move") {
                r.direction(0, p);
                r.rebuild(map)?;
                w.set_dynamic(r.colliders());
                for _ in 0..24 {
                    r.advance(crate::movement::FIXED_DT, map, w, p, &[])?;
                }
            }
        }
        "wchess1-knight-gate" => {
            r.saved.bishop_done = true;
            r.saved.knight_gate = true;
            *p = Player::new(vec3(3488., 1728., -127.96875));
        }
        "wchess1-bell-delay" => {
            r.saved.bishop_done = true;
            r.saved.bell = Some(98.);
            r.saved.lever_started[0] = Some(94.);
            *p = Player::new(vec3(320., 1248., 416.03125));
        }
        "wchess1-water-rising" => {
            r.saved.bishop_done = true;
            r.saved.knight_gate = true;
            r.saved.knight_done = true;
            r.saved.water = Some(95.);
            r.saved.lever_started[1] = Some(87.);
            *p = Player::new(vec3(2080., 1440., 256.03125));
        }
        _ => anyhow::bail!("Unknown Pale Realm fixture {case}"),
    }
    r.rebuild(map)?;
    r.player_pose = Transform {
        translation: p.feet,
        rotation: Quat::IDENTITY,
    };
    i.sync(w);
    Ok(Story::load(a, "wchess1"))
}

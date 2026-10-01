use super::*;
pub(super) struct Data {
    pub floor: Vec<(usize, Vec3)>,
    pub decor: Vec<(usize, Vec3)>,
    pub clip: (usize, Vec3),
    pub path: Vec<Vec3>,
    pub support: World,
    pub arena: World,
    pub length: f32,
    pub cameras: Vec<crate::fortress::spline::Spline>,
    pub spawns: Vec<Transform>,
    pub launches: Vec<Vec3>,
    pub duel: Vec<(String, Transform, usize)>,
    pub pawn: Transform,
    pub pawn_velocity: Vec3,
    pub easy: bool,
}
pub(super) fn pose(e: &super::super::Entity) -> Transform {
    Transform {
        translation: e
            .get("origin")
            .and_then(|s| interaction::vector(s))
            .unwrap_or(Vec3::ZERO),
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    }
}
fn brush(map: &Bsp, id: usize) -> Result<(usize, Vec3)> {
    let e = &map.entities[id];
    Ok((
        e.get("model")
            .and_then(|s| s.strip_prefix('*'))
            .context("Missing battlefield brush")?
            .parse()?,
        pose(e).translation,
    ))
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        ensure!(
            map.entities[19]
                .get("targetname")
                .is_some_and(|s| s == "fall6"),
            "Unexpected battlefield layout"
        );
        let floor = (19..=24)
            .map(|i| brush(map, i))
            .collect::<Result<Vec<_>>>()?;
        let mut world = World::from_bsp(map)?;
        world.set_dynamic(
            floor
                .iter()
                .map(|(m, p)| Collider::model(map, *m, *p, Quat::IDENTITY, true))
                .collect::<Result<_>>()?,
        );
        let mut path = Vec::new();
        for id in 257..=265 {
            let origin = pose(&map.entities[id]).translation;
            path.push(
                world
                    .actor_footing(origin, PLAYER_CENTER, PLAYER_HALF, 160.)
                    .with_context(|| format!("Unsupported arrival marker {id} at {origin:?}"))?,
            );
        }
        let length = path.windows(2).map(|p| p[0].distance(p[1])).sum();
        let cameras = [
            "grounds2a1",
            "grounds2a2",
            "grounds2br1",
            "grounds2br2",
            "grounds2bra",
        ]
        .into_iter()
        .map(|n| {
            Ok(crate::fortress::spline::Spline::camera_track(
                crate::cinematic::Track::load(a, n)?.controls().collect(),
            ))
        })
        .collect::<Result<_>>()?;
        let mut spawns = Vec::new();
        let mut launches = Vec::new();
        for (id, target) in [
            (241, 242),
            (15, 16),
            (281, 282),
            (284, 285),
            (290, 291),
            (287, 288),
            (376, 377),
            (373, 374),
        ] {
            let at = pose(&map.entities[id]);
            let goal = pose(&map.entities[target]).translation;
            spawns.push(at);
            launches.push(
                crate::traversal::launch_velocity(at.translation, goal)
                    .context("Invalid guard launch pad")?,
            );
        }
        let mut duel = Vec::new();
        for (id, opponent) in [
            (248, 249),
            (250, 251),
            (247, 272),
            (246, 271),
            (249, 248),
            (251, 250),
            (272, 247),
            (271, 246),
        ] {
            let e = &map.entities[id];
            let mut at = pose(e);
            let d = pose(&map.entities[opponent]).translation - at.translation;
            at.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
            duel.push((e["model"].trim_end_matches(".tik").into(), at, id));
        }
        let pawn = pose(&map.entities[277]);
        Ok(Self {
            arena: World::actor_world(map)?,
            decor: vec![brush(map, 5)?, brush(map, 380)?],
            support: world,
            floor,
            clip: brush(map, 1)?,
            path,
            length,
            cameras,
            spawns,
            launches,
            duel,
            pawn,
            pawn_velocity: crate::traversal::launch_velocity(
                pawn.translation,
                pose(&map.entities[244]).translation,
            )
            .context("Invalid pawn launch")?,
            easy: map.difficulty == crate::powerups::Difficulty::Easy,
        })
    }
}

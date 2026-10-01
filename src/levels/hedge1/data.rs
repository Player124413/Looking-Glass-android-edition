use super::*;
use crate::collision::model_shape::Template;
pub(super) struct Door {
    pub model: usize,
    pub base: Vec3,
    pub slide: Vec3,
    pub duration: f32,
    pub shape: Template,
    pub top: f32,
}
pub(super) struct Data {
    pub doors: Vec<Door>,
    pub plate: Collider,
    pub portal_origin: Vec3,
    pub portal_landing: Vec3,
}
impl Data {
    pub fn load(map: &Bsp) -> Result<Self> {
        ensure!(
            map.entities[21]["spawnflags"] == "12" && map.entities[21]["thread"] == "SeekHold",
            "Maze Hold response changed"
        );
        let mut doors = vec![];
        for id in [31, 32, 33] {
            let e = &map.entities[id];
            ensure!(
                e.get("classname").is_some_and(|s| s == "func_door")
                    && e["targetname"] == "seek_exit",
                "Maze gate binding changed"
            );
            let model = e["model"].trim_start_matches('*').parse::<usize>()?;
            let base = crate::interaction::vector(&e["origin"]).context("Gate origin")?;
            let angle = e["angle"].parse::<f32>()?;
            let dir = if angle == -2. {
                -Vec3::Z
            } else {
                vec3(angle.to_radians().cos(), angle.to_radians().sin(), 0.)
            };
            let bounds = &map.models[model];
            let lip = e
                .get("lip")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(8.);
            let distance = (bounds.max - bounds.min).dot(dir.abs()) - lip;
            let speed = e
                .get("speed")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(100.);
            ensure!(distance > 0. && speed > 0., "Invalid gate travel");
            doors.push(Door {
                model,
                base,
                slide: dir * distance,
                duration: distance / speed,
                shape: Template::model(map, model)?,
                top: base.z + bounds.max.z,
            });
        }
        let e = &map.entities[30];
        ensure!(
            e["spawnflags"] == "8" && e["target"] == "seek_exit",
            "Plate response changed"
        );
        let plate = Collider::model(
            map,
            e["model"].trim_start_matches('*').parse()?,
            crate::interaction::vector(&e["origin"]).context("Plate origin")?,
            Quat::IDENTITY,
            false,
        )?;
        ensure!(
            map.entities[29]["map"] == "tower1$tower1_start1",
            "Maze exit changed"
        );
        let destination = &map.entities[27];
        ensure!(
            destination["classname"] == "func_teleportdest" && destination["targetname"] == "t30",
            "Maze portal binding changed"
        );
        let portal_origin =
            crate::interaction::vector(&destination["origin"]).context("Portal origin")?;
        let yaw = destination["angle"].parse::<f32>()?.to_radians();
        let world = World::from_bsp(map)?;
        // The supplied marker lies in the solid teleporter2_1red patch. Place the
        // player's body just in front of that sheet along the marker's facing.
        // This bounded, map-specific correction preserves the surrounding walls.
        let portal_landing = if world.body_clear(portal_origin) {
            portal_origin
        } else {
            let p = portal_origin + vec3(yaw.cos(), yaw.sin(), 0.) * (PLAYER_HALF.x + 1.);
            ensure!(
                world.body_clear(p),
                "Maze portal no longer has its reviewed landing clearance"
            );
            p
        };
        Ok(Self {
            doors,
            plate,
            portal_origin,
            portal_landing,
        })
    }
}

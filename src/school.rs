//! Explicit first-visit school progression, based on the locally supplied map scripts.
//! This is a bounded gameplay translation, not an interpreter for arbitrary script text.
pub mod cinema;
pub mod cinema_check;
mod door_check;
mod library_check;
pub(crate) mod return_check;
pub(crate) mod return_cinema;
use crate::{
    assets::Assets,
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    interaction::{vector, Events},
    movement::Player,
    skeletal::Transform,
    texture,
    weapons::Prop,
};
use anyhow::{Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

const RETURN_OBJECTS: &[&str] = &[
    "star_door1",
    "star_door2",
    "observatory_lift",
    "observatory_lift_arm_left",
    "observatory_lift_arm_right",
];
const OBJECTS: &[&str] = &[
    "slamming_door1",
    "slamming_door2",
    "glass_secret_panel",
    "secret_face",
    "secret_shelf",
    "secret_shelf_block",
    "shelf_book",
    "bookshelf1",
    "bookshelf2",
    "library_spiral_lift",
    "elevator2",
    "observatory_clip",
    "removeme",
    "flyingsupport1",
    "flyingsupport2",
    "flyingsupport3",
    "flyingsupport4",
    "t182",
    "t183",
    "t184",
    "t185",
];
fn flapping_door(name: &str) -> bool {
    matches!(name, "slamming_door1" | "slamming_door2")
}
fn door_motion(name: &str, base: Pose) -> Option<Motion> {
    let (angle, outward, inward) = match name {
        "slamming_door1" => (32., 0.4, 0.3),
        "slamming_door2" => (-26., 0.2, 0.4),
        _ => return None,
    };
    Some(Motion {
        keys: vec![
            (0., base),
            (
                outward,
                Pose {
                    angles: Vec3::Z * angle,
                    ..base
                },
            ),
            (outward + inward, base),
        ],
        time: 0.,
        repeat: true,
    })
}
pub fn supported_inline(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|s| s == "script_object")
        && e.get("targetname")
            .is_some_and(|s| OBJECTS.contains(&s.as_str()) || RETURN_OBJECTS.contains(&s.as_str()))
}
/// Theatre pictures are initialized scenery, independent of saved puzzle movers.
pub fn theatre_picture(e: &BTreeMap<String, String>) -> bool {
    e.get("classname").is_some_and(|s| s == "script_object")
        && e.get("targetname").is_some_and(|s| {
            matches!(
                s.as_str(),
                "theatre_pic1" | "theatre_pic2" | "theatre_pic3" | "theatre_pic4"
            )
        })
}
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
struct Pose {
    origin: Vec3,
    // Euler components stay unwrapped: the spiral lift turns one and a half times.
    angles: Vec3,
}
impl Pose {
    fn rotation(self) -> Quat {
        Quat::from_rotation_z(self.angles.z.to_radians())
            * Quat::from_rotation_y(self.angles.y.to_radians())
    }
    fn lerp(self, other: Self, f: f32) -> Self {
        Self {
            origin: self.origin.lerp(other.origin, f),
            angles: self.angles.lerp(other.angles, f),
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Motion {
    keys: Vec<(f32, Pose)>,
    time: f32,
    repeat: bool,
}
impl Motion {
    fn sample(&self, time: f32) -> Pose {
        for k in self.keys.windows(2) {
            if time <= k[1].0 {
                return k[0]
                    .1
                    .lerp(k[1].1, ((time - k[0].0) / (k[1].0 - k[0].0)).clamp(0., 1.));
            }
        }
        self.keys.last().unwrap().1
    }
}
struct Object {
    name: String,
    model: usize,
    base: Pose,
    pose: Pose,
    collider: Collider,
    enabled: bool,
    solid: bool,
    motion: Option<Motion>,
}
impl Object {
    fn move_to(&mut self, end: Pose, seconds: f32) {
        self.motion = Some(Motion {
            keys: vec![(0., self.pose), (seconds, end)],
            time: 0.,
            repeat: false,
        });
    }
}
#[derive(Clone, Copy, Default, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Book {
    #[default]
    Waiting,
    Flying,
    Bridge,
    Descending,
    Landing,
}
pub struct School {
    pub first_cinema: Option<cinema::State>,
    cinema_data: Option<cinema::Data>,
    pub(crate) return_data: Option<return_cinema::Data>,
    objects: Vec<Object>,
    pictures: Vec<(usize, Vec3, Collider)>,
    paths: [Vec<Vec3>; 4],
    last_paths: [Vec<Vec3>; 4],
    book_offsets: [Vec3; 4],
    book_yaws: [f32; 4],
    books: [Book; 4],
    book_times: [f32; 4],
    returning: bool,
    pub return_visit: Option<crate::school_return::State>,
    theatre: bool,
    library: bool,
    shelves: [bool; 2],
    recipe: bool,
    recipe_time: f32,
    book_top: Vec3,
    book_bottom: Vec3,
    clock: f32,
    pub secret_open: bool,
    secret_claimed: bool,
    secret_reward: Vec3,
}
fn path(map: &Bsp, start: &str) -> Vec<Vec3> {
    let mut name = start;
    let mut seen = std::collections::BTreeSet::new();
    let mut result = Vec::new();
    while seen.insert(name) && result.len() < 128 {
        let Some(e) = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == name))
        else {
            break;
        };
        if let Some(p) = e.get("origin").and_then(|s| vector(s)) {
            result.push(p);
        }
        let Some(next) = e.get("target") else { break };
        name = next;
    }
    result
}
impl School {
    pub fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        self.return_sound(loops, clocks);
        loops.push(crate::audio::LoopCue {
            id: 1002,
            path: "sound/world/mover/flapping_door_loop.wav",
            origin: self.object("slamming_door1").pose.origin,
            clock: Some(self.clock),
        });
        for (i, name) in ["bookshelf1", "bookshelf2"].into_iter().enumerate() {
            if self.shelves[i] {
                let o = self.object(name);
                clocks.push(crate::audio::world::Clock {
                    key: name,
                    time: o.motion.as_ref().map_or(2., |m| m.time),
                    period: None,
                    origin: o.pose.origin,
                    cues: &[
                        (0., "sound/world/mover/bookcase_fall_1.wav"),
                        (2., "sound/world/mover/bookcase_fall_2.wav"),
                    ],
                });
            }
        }
        // The map's lift1_sound speaker is triggered by the spiral lift movement.
        let o = self.object("library_spiral_lift");
        if let Some(m) = &o.motion {
            let moving = m
                .sample(m.time + 0.01)
                .origin
                .distance_squared(m.sample(m.time).origin)
                > 0.001;
            if moving {
                loops.push(crate::audio::LoopCue {
                    id: 1001,
                    path: "sound/ambience/special/lift_1.wav",
                    origin: o.pose.origin,
                    clock: Some(m.time),
                });
            }
        }
    }
    pub fn load(map: &Bsp, returning: bool) -> Result<Self> {
        let mut objects = Vec::new();
        let mut pictures = Vec::new();
        for e in map.entities.iter().filter(|e| theatre_picture(e)) {
            let offset = match e["targetname"].as_str() {
                "theatre_pic1" => vec3(628., -20., 0.),
                "theatre_pic2" => vec3(628., 36., 0.),
                // Initialization explicitly hides the pendulum-bound third picture.
                "theatre_pic3" => continue,
                _ => Vec3::ZERO,
            };
            let model = e["model"].trim_start_matches('*').parse()?;
            let origin = vector(&e["origin"]).context("Invalid theatre picture")? + offset;
            pictures.push((
                model,
                origin,
                Collider::model(map, model, origin, Quat::IDENTITY, true)?,
            ));
        }
        for e in map.entities.iter().filter(|e| supported_inline(e)) {
            if !returning && RETURN_OBJECTS.contains(&e["targetname"].as_str()) {
                continue;
            }
            let model = e["model"].trim_start_matches('*').parse()?;
            let origin = vector(&e["origin"]).context("School object has invalid origin")?;
            let name = e["targetname"].clone();
            let base = Pose {
                origin,
                angles: Vec3::ZERO,
            };
            let bridge = matches!(name.as_str(), "t182" | "t183" | "t184" | "t185");
            let motion = door_motion(&name, base);
            objects.push(Object {
                name,
                model,
                base,
                pose: base,
                collider: Collider::model(map, model, origin, Quat::IDENTITY, true)?,
                enabled: !bridge,
                solid: true,
                motion,
            });
        }
        for name in OBJECTS {
            anyhow::ensure!(
                objects.iter().any(|o| o.name == *name),
                "Missing school object {name}"
            );
        }
        let entity_origin = |name: &str| -> Result<Vec3> {
            map.entities
                .iter()
                .find(|e| e.get("targetname").is_some_and(|s| s == name))
                .and_then(|e| e.get("origin"))
                .and_then(|s| vector(s))
                .with_context(|| format!("Missing {name}"))
        };
        let mut offsets = [Vec3::ZERO; 4];
        let mut yaws = [0.; 4];
        for i in 0..4 {
            let name = format!("flyingbook{}", i + 1);
            offsets[i] = entity_origin(&name)? - entity_origin(&format!("flyingsupport{}", i + 1))?;
            yaws[i] = map
                .entities
                .iter()
                .find(|e| e.get("targetname") == Some(&name))
                .and_then(|e| e.get("angle"))
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians();
        }
        let mut s = Self {
            first_cinema: None,
            cinema_data: None,
            return_data: None,
            objects,
            pictures,
            paths: ["t46", "t59", "t65", "t72"].map(|n| path(map, n)),
            last_paths: std::array::from_fn(|i| path(map, &format!("book{}lastpath", i + 1))),
            book_offsets: offsets,
            book_yaws: yaws,
            books: [Book::Waiting; 4],
            book_times: [0.; 4],
            returning,
            return_visit: returning.then(Default::default),
            theatre: returning,
            library: returning,
            shelves: [returning; 2],
            recipe: false,
            recipe_time: 0.,
            book_top: entity_origin("bitterbook")? + Vec3::Z * 18.,
            book_bottom: entity_origin("bitterbook2")?,
            clock: 0.,
            secret_open: false,
            secret_claimed: false,
            secret_reward: map
                .entities
                .iter()
                .find(|e| {
                    e.get("classname")
                        .is_some_and(|s| s == "Item_DarkenedLookingGlass")
                })
                .and_then(|e| e.get("origin"))
                .and_then(|s| vector(s))
                .context("Missing school secret reward")?,
        };
        anyhow::ensure!(
            s.paths.iter().chain(&s.last_paths).all(|p| !p.is_empty()),
            "Missing school book path"
        );
        if returning {
            for name in [
                "secret_shelf_block",
                "observatory_clip",
                "removeme",
                "flyingsupport1",
                "flyingsupport2",
                "flyingsupport3",
                "flyingsupport4",
            ] {
                s.object_mut(name).enabled = false;
            }
            for name in ["secret_shelf", "shelf_book"] {
                s.object_mut(name).pose.origin.y += 648.;
            }
            s.object_mut("bookshelf1").pose.angles.y = 45.;
            s.object_mut("bookshelf2").pose.angles.y = -45.;
            for o in &mut s.objects {
                o.collider = Collider::model(map, o.model, o.pose.origin, o.pose.rotation(), true)?;
            }
        }
        s.refresh_bound_book(map)?;
        Ok(s)
    }
    fn object_mut(&mut self, name: &str) -> &mut Object {
        self.objects.iter_mut().find(|o| o.name == name).unwrap()
    }
    pub fn collect_secret(
        &mut self,
        stats: &mut crate::inventory::Stats,
        feet: Vec3,
        world: &World,
    ) -> Events {
        self.secret_claimed = stats.collected.contains("skool1:lookingglass");
        if self.secret_claimed
            || !self.secret_open
            || !stats.alive()
            || (feet - self.secret_reward).truncate().length() > 30.
            || feet.z + 56. < self.secret_reward.z
            || feet.z > self.secret_reward.z + 64.
        {
            return Events::default();
        }
        let t = world.sweep(
            feet + Vec3::Z * 24.,
            self.secret_reward + Vec3::Z * 24.,
            Vec3::ZERO,
        );
        if t.start_solid || t.fraction < 0.99 {
            return Events::default();
        }
        if !stats.powerup(crate::powerups::Kind::Glass) {
            return Events::default();
        }
        self.secret_claimed = true;
        stats.collected.insert("skool1:lookingglass".into());

        Events {
            story: vec!["Cat_Glass_Dialog".into()],
            message: Some(format!(
                "Secret found / Darkened Looking Glass / {:.0} seconds",
                stats.invisible
            )),
            ..Default::default()
        }
    }
    fn object(&self, name: &str) -> &Object {
        self.objects.iter().find(|o| o.name == name).unwrap()
    }
    fn bound_pose(&self, o: &Object, pose: Pose) -> Pose {
        if o.name != "shelf_book" {
            return pose;
        }
        let shelf = self.object("secret_shelf");
        // The saved switch motion remains local to its original shelf position.
        // Older non-cinematic/return saves already include the shelf translation;
        // they have no pressed offset. Deriving the binding also repairs completed
        // saves without replaying a scene or changing the saved object identities.
        Pose {
            origin: o.base.origin + shelf.pose.origin - shelf.base.origin
                + Vec3::Y * (pose.origin.y - o.base.origin.y).clamp(-16., 0.),
            ..pose
        }
    }
    fn refresh_bound_book(&mut self, map: &Bsp) -> Result<()> {
        let o = self.object("shelf_book");
        let pose = self.bound_pose(o, o.pose);
        let collider = Collider::model(map, o.model, pose.origin, pose.rotation(), true)?;
        self.object_mut("shelf_book").collider = collider;
        Ok(())
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| o.enabled && o.solid)
            .map(|o| o.collider.clone())
            .chain(self.pictures.iter().map(|p| p.2.clone()))
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects
            .iter()
            .filter(|o| o.enabled)
            .map(|o| {
                let p = self.bound_pose(o, o.pose);
                (o.model, p.origin, p.rotation())
            })
            .chain(self.pictures.iter().map(|p| (p.0, p.1, Quat::IDENTITY)))
    }
    pub fn event_facts(&self) -> crate::event::Facts {
        let mut f = crate::event::Facts::default();
        for (key, value) in [
            ("school.return", self.returning),
            ("school.theatre", self.theatre),
            ("school.library", self.library),
            ("school.recipe", self.recipe),
            ("school.recipe_ready", self.recipe_ready()),
        ] {
            f.flag(key, value);
        }
        if let Some(r) = &self.return_visit {
            f.extend(r.facts());
        }
        f.count(
            "school.books_bridged",
            self.books.iter().filter(|b| **b == Book::Bridge).count() as u32,
        );
        f
    }
    pub fn trigger_condition(
        name: &str,
        thread: &str,
        exit: Option<&str>,
    ) -> crate::event::Condition {
        use crate::event::Condition as C;
        let first = || C::flag("school.return").not();
        match name {
            "return_booj_trigger"
            | "lastpass_trigger"
            | "olift_activate_trigger"
            | "olift_up_trigger" => return C::flag("school.return"),
            "first_monster_trigger1" | "firstpass_trigger" | "bookcase_goodie_trigger" => {
                return first()
            }
            "elevator_hurt" => return C::Any(vec![]), // Swept supports replace crushing.
            _ => (),
        }
        if let Some(exit) = exit {
            return if exit == "skool2" {
                C::All(vec![first(), C::flag("school.recipe_ready")])
            } else {
                C::flag("school.return")
            };
        }
        match thread {
            "Theatre_Cinematic" => C::All(vec![first(), C::flag("school.theatre").not()]),
            "Skool1_OG_MoveShelf" => C::All(vec![
                C::flag("school.theatre"),
                C::flag("school.library").not(),
            ]),
            "Book_Ingredients_Exit" => C::All(vec![first(), C::flag("school.recipe_ready")]),
            "book_cinematic" => C::All(vec![
                first(),
                C::flag("school.recipe").not(),
                C::AtLeast("school.books_bridged".into(), 4),
            ]),
            "start_book1" | "start_book2" | "start_book3" | "start_book4" => {
                C::All(vec![first(), C::flag("school.library")])
            }
            _ => C::Always,
        }
    }
    pub fn condition(
        &self,
        name: &str,
        thread: &str,
        exit: Option<&str>,
    ) -> crate::event::Condition {
        if self.return_visit.is_some() && exit == Some("potears1") {
            return crate::event::Condition::flag("return.finished");
        }
        self.return_visit
            .as_ref()
            .and_then(|r| r.gate(thread))
            .unwrap_or_else(|| Self::trigger_condition(name, thread, exit))
    }
    pub fn cinematic(&self) -> bool {
        self.scene_id().is_some() || self.return_visit.as_ref().is_some_and(|r| r.cinematic())
    }
    pub fn sync_inventory(&mut self, stats: &mut crate::inventory::Stats) {
        if let Some(r) = &mut self.return_visit {
            r.inventory(&mut stats.school_items);
        }
    }
    pub fn trigger_enabled(&self, name: &str, thread: &str, exit: Option<&str>) -> bool {
        self.condition(name, thread, exit).test(&self.event_facts())
    }
    fn tip(message: impl Into<String>) -> Events {
        Events {
            message: Some(message.into()),
            ..Default::default()
        }
    }
    pub fn event(&mut self, thread: &str) -> Option<Events> {
        use crate::school_return::Phase as R;
        if self.return_visit.is_some() {
            let r = self.return_visit.as_mut().unwrap();
            match thread {
                "Skool1_Setup_OLift" if r.star && r.phase == R::Locked => {
                    r.enter(R::Open);
                    r.star = false;
                    for (name, angle) in [("star_door1", -75.), ("star_door2", 75.)] {
                        let o = self.object_mut(name);
                        o.move_to(
                            Pose {
                                angles: vec3(0., 0., angle),
                                ..o.base
                            },
                            1.,
                        );
                    }
                    return Some(Events {
                        sound: Some("sound/world/door/door wood open 01.wav".into()),
                        message: Some("Lucky Star used / step aboard the observatory lift".into()),
                        ..Default::default()
                    });
                }
                "Skool1_OLift_Up" if r.phase == R::Open && r.time >= 1. => {
                    r.enter(R::Rising);
                    if self.return_data.is_some() {
                        r.scene = Some(return_cinema::State::new(return_cinema::Beat::Star));
                        return Some(Events::default());
                    }
                    let o = self.object_mut("observatory_lift");
                    o.move_to(
                        Pose {
                            origin: o.base.origin + Vec3::Z * 632.,
                            ..o.base
                        },
                        5.,
                    );
                    return Some(Events {
                        sound: Some("sound/ambience/special/observatory_lift.wav".into()),
                        ..Default::default()
                    });
                }
                "observatory_exit_cinematic" if r.phase == R::Observatory && r.potion => {
                    r.enter(R::Opening);
                    if self.return_data.is_some() {
                        r.scene = Some(return_cinema::State::new(return_cinema::Beat::Exit));
                        return Some(Events::default());
                    }
                    return Some(Events {
                        sound: Some("sound/world/machine/globe_open.wav".into()),
                        message: Some(r.objective().into()),
                        ..Default::default()
                    });
                }
                _ => (),
            }
        }
        let was_recipe = self.recipe;
        let event = match thread {
            "Open_Bookcase_Goodie" if !self.returning && !self.secret_open => {
                self.secret_open = true;
                self.object_mut("glass_secret_panel").enabled = false;
                let o = self.object_mut("secret_face");
                o.move_to(
                    Pose {
                        origin: o.base.origin + Vec3::X * 16.,
                        ..o.base
                    },
                    2.,
                );
                Some(Events {
                    message: Some("A hidden panel opens in the library.".into()),
                    sound: Some("sound/ambience/special/slow scrape.wav".into()),
                    ..Default::default()
                })
            }
            "Open_Bookcase_Goodie" => Some(Events::default()),
            "Theatre_Cinematic" if !self.theatre => {
                self.theatre = true;
                if self.cinema_data.is_some() {
                    self.scene_start(cinema::Beat::Theatre);
                }
                Some(Self::tip(
                    "The theatre passage is ready. Return through the doors to open the library.",
                ))
            }
            "Skool1_OG_MoveShelf" if self.theatre && !self.library => {
                if self.cinema_data.is_some() {
                    self.scene_start(cinema::Beat::Shelf);
                    return Some(Events::default());
                }
                self.library = true;
                self.object_mut("secret_shelf_block").enabled = false;
                {
                    let o = self.object_mut("secret_shelf");
                    let end = Pose {
                        origin: o.base.origin + Vec3::Y * 648.,
                        ..o.base
                    };
                    o.move_to(end, 10.);
                }
                Some(Self::tip(
                    "The library passage is opening. Find the four flying books.",
                ))
            }
            "shelf_cinematic" if self.cinema_data.is_some() && !self.returning => {
                if self.scene_id().is_some() || self.shelves.iter().all(|v| *v) {
                    return Some(Events::default());
                }
                self.scene_start(cinema::Beat::Shelves);
                Some(Events::default())
            }
            "push_bookshelf1" | "push_bookshelf2" | "shelf_cinematic" => {
                let mut changed = false;
                for i in 0..2 {
                    if !self.shelves[i]
                        && (thread == "shelf_cinematic" || thread.ends_with(&(i + 1).to_string()))
                    {
                        self.shelves[i] = true;
                        changed = true;
                        let o = self.object_mut(&format!("bookshelf{}", i + 1));
                        o.move_to(
                            Pose {
                                angles: vec3(0., if i == 0 { 45. } else { -45. }, 0.),
                                ..o.base
                            },
                            2.,
                        );
                    }
                }
                if !changed {
                    Some(Events::default())
                } else {
                    Some(Self::tip(
                        "The shelves are tipping. Stand clear, then climb the fallen shelves.",
                    ))
                }
            }
            "Skool1_Lift_Up" | "Skool1_Elevator_Up" => {
                let spiral = thread == "Skool1_Lift_Up";
                let o = self.object_mut(if spiral {
                    "library_spiral_lift"
                } else {
                    "elevator2"
                });
                if o.motion.is_some() {
                    return Some(Events::default());
                }
                let top = Pose {
                    origin: o.base.origin + Vec3::Z * if spiral { 328. } else { 296. },
                    angles: vec3(0., 0., if spiral { -540. } else { 0. }),
                };
                let duration = if spiral { 6. } else { 3. };
                o.motion = Some(Motion {
                    keys: vec![
                        (0., o.base),
                        (1., o.base),
                        (1. + duration, top),
                        (6. + duration, top),
                        (6. + 2. * duration, o.base),
                        (11. + 2. * duration, o.base),
                    ],
                    time: 0.,
                    repeat: true,
                });
                Some(Self::tip(
                    "Lift moving / step aboard; it will return if you miss it.",
                ))
            }
            "start_book1" | "start_book2" | "start_book3" | "start_book4" => {
                let i = thread.as_bytes()[10] as usize - b'1' as usize;
                if self.books[i] != Book::Waiting {
                    return Some(Events::default());
                }
                self.books[i] = Book::Flying;
                self.book_times[i] = 0.;
                let points = self.paths[i].clone();
                self.follow_book(i, &points, [6., 6., 7., 8.][i], 0.55);
                let count = self.books.iter().filter(|&&b| b != Book::Waiting).count();
                Some(Self::tip(format!(
                    "Flying books {count}/4 / {}",
                    if count == 4 {
                        "Cross their bridge to the large recipe book."
                    } else {
                        "Search the other library floors."
                    }
                )))
            }
            "book_cinematic" if !self.recipe => {
                if self.books.iter().any(|b| *b != Book::Bridge) {
                    return Some(Self::tip(
                        "Wake all four flying books before approaching the recipe book.",
                    ));
                }
                self.recipe = true;
                self.recipe_time = 0.;
                if self.cinema_data.is_some() {
                    self.scene_start(cinema::Beat::Book);
                    return Some(Events {
                        story: vec![thread.into()],
                        ..Default::default()
                    });
                }
                self.object_mut("removeme").enabled = false;
                for i in 0..4 {
                    self.books[i] = Book::Descending;
                    self.object_mut(&format!("t{}", 185 - i)).enabled = false;
                    let points = self.last_paths[i].clone();
                    self.follow_book(i, &points, 3., 0.);
                }
                Some(Self::tip(
                    "The recipe book has fallen. Return to it on the library's lowest floor.",
                ))
            }
            "Book_Ingredients_Exit" if self.recipe_ready() && self.cinema_data.is_some() => {
                if self.scene_id().is_some()
                    || self
                        .first_cinema
                        .as_ref()
                        .and_then(|s| s.library.as_ref())
                        .is_some_and(|l| l.exit_ready)
                {
                    return Some(Events::default());
                }
                self.scene_start(cinema::Beat::Recipe);
                Some(Events::default())
            }
            "Book_Ingredients_Exit" if self.recipe_ready() => Some(Events {
                message: Some("Recipe found / continuing to the next part of the school".into()),
                transition: Some(("skool2".into(), Some("skool2_start1".into()))),
                ..Default::default()
            }),
            // These camera-only beats have no navigation side effects in the first visit.
            "Skool1_Book_Win" => Some(Events::default()),
            "mallet_cat" => Some(Events::default()),
            "cat_fluff_cinematic" => Some(Self::tip(
                "Explore the library. The flying books can make a path to the recipe.",
            )),
            _ => None,
        };
        event.map(|mut e| {
            // Publish only accepted story beats. Touching the book before its puzzle
            // is solved must not play the successful book-opening conversation.
            if crate::story::supports("skool1", thread)
                && (thread != "book_cinematic" || (!was_recipe && self.recipe))
            {
                e.story.push(thread.into());
            }
            e
        })
    }
    pub fn theatre_finished(&self) -> bool {
        self.theatre
    }
    fn follow_book(&mut self, i: usize, points: &[Vec3], duration: f32, delay: f32) {
        let o = self.object_mut(&format!("flyingsupport{}", i + 1));
        o.solid = false;
        let mut keys = vec![(0., o.pose)];
        if delay > 0. {
            keys.push((delay, o.pose));
        }
        // Use authored waypoints; spline interpolation/speeds remain a presentation approximation.
        let total = std::iter::once(o.pose.origin)
            .chain(points.iter().copied())
            .collect::<Vec<_>>()
            .windows(2)
            .map(|p| p[0].distance(p[1]))
            .sum::<f32>()
            .max(0.001);
        let mut last = o.pose.origin;
        let mut time = delay;
        for &p in points {
            let distance = last.distance(p);
            last = p;
            if distance < 0.001 {
                continue;
            }
            time += duration * distance / total;
            keys.push((
                time,
                Pose {
                    origin: p,
                    ..o.pose
                },
            ));
        }
        o.motion = Some(Motion {
            keys,
            time: 0.,
            repeat: false,
        });
    }
    pub fn objective(&self) -> String {
        if let Some(r) = &self.return_visit {
            return r.objective().into();
        }
        if !self.theatre {
            return "School: follow the corridor left to the theatre.".into();
        }
        if !self.library {
            return "School: return through the theatre doors to reveal the library passage."
                .into();
        }
        if self.recipe {
            return "School: reach the fallen recipe book on the lowest library floor.".into();
        }
        let n = self.books.iter().filter(|&&b| b != Book::Waiting).count();
        if n < 4 {
            format!(
                "School: wake the four flying books ({n}/4). Use the lifts to explore each floor."
            )
        } else {
            "School: cross the flying-book bridge to the large recipe book.".into()
        }
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        // Bound the swept transforms, including when the renderer has a slow frame.
        let mut remaining = dt.min(0.1);
        while remaining > 0.000001 {
            let step = remaining.min(1. / 120.);
            remaining -= step;
            self.clock += step;
            self.scene_step(step, world, player)?;
            self.return_step(step, player);
            if let Some(r) = self.return_visit.as_mut().filter(|r| r.scene.is_none()) {
                use crate::school_return::Phase as R;
                r.time += step;
                if r.phase == R::Opening && r.time >= 5. {
                    r.enter(R::Drinking);
                    r.potion = false;
                }
                if r.phase == R::Drinking && r.time >= 3.8 {
                    r.enter(R::Shrinking);
                }
                if r.phase == R::Shrinking && r.time >= 2.5 {
                    r.enter(R::Complete);
                }
                if r.cinematic() {
                    player.velocity = Vec3::ZERO;
                    player.grounded = true;
                    if r.phase >= R::Drinking {
                        let drink = map
                            .entities
                            .iter()
                            .find(|e| e.get("targetname").is_some_and(|n| n == "alice_drink_pos"))
                            .and_then(|e| e.get("origin"))
                            .and_then(|s| vector(s))
                            .unwrap();
                        player.feet = drink;
                        if r.phase >= R::Shrinking {
                            let jump = path(map, "alice_jump");
                            if !jump.is_empty() {
                                let t = if r.phase == R::Complete {
                                    1.
                                } else {
                                    (r.time / 2.5).min(1.)
                                };
                                let index = t * (jump.len() - 1) as f32;
                                let a = index.floor() as usize;
                                player.feet =
                                    jump[a].lerp(jump[(a + 1).min(jump.len() - 1)], index.fract());
                            }
                        }
                    }
                    r.position = player.feet;
                }
            }
            if self.recipe && self.scene_id() != Some(cinema::library::BOOK) {
                self.recipe_time += step;
            }
            let last_opening = self.books.contains(&Book::Flying)
                && self.books.iter().all(|b| matches!(b, Book::Flying | Book::Bridge))
                && self.book_times.iter().copied().fold(f32::INFINITY, f32::min) < 0.55;
            for i in 0..4 {
                if self.books[i] != Book::Waiting {
                    self.book_times[i] += step;
                }
            }
            if last_opening && self.book_times.iter().all(|t| *t >= 0.55)
                && self.return_visit.is_none() && self.cinema_data.is_some() && !self.cinematic()
            {
                self.scene_start(cinema::Beat::BookWin);
            }
            for i in 0..self.objects.len() {
                let o = &self.objects[i];
                let Some(motion) = &o.motion else {
                    continue;
                };
                let duration = motion.keys.last().unwrap().0;
                // Preserve the remainder on these short ambient cycles, so their
                // rhythm does not drift with the rendering frame rate.
                let time = if flapping_door(&o.name) {
                    (motion.time + step).rem_euclid(duration)
                } else {
                    (motion.time + step).min(duration)
                };
                let next = motion.sample(time);
                let next_world = self.bound_pose(o, next);
                let old_world = self.bound_pose(o, o.pose);
                let collider =
                    Collider::model(map, o.model, next_world.origin, next_world.rotation(), true)?;
                let ground = o.collider.trace(
                    player.feet + PLAYER_CENTER,
                    player.feet + PLAYER_CENTER - Vec3::Z * 3.,
                    PLAYER_HALF,
                );
                let riding = o.enabled
                    && o.solid
                    && player.velocity.z <= 1.
                    && !ground.start_solid
                    && ground.fraction < 1.
                    && ground.normal.z > 0.65;
                world.set_dynamic(
                    fixed
                        .iter()
                        .cloned()
                        .chain(
                            self.objects
                                .iter()
                                .enumerate()
                                .filter(|(j, o)| *j != i && o.enabled && o.solid)
                                .map(|(_, o)| o.collider.clone()),
                        )
                        .chain(self.pictures.iter().map(|p| p.2.clone()))
                        .collect(),
                );
                let mut carried = next_world.origin
                    + next_world.rotation()
                        * old_world.rotation().inverse()
                        * (player.feet - old_world.origin);
                let mut support_normal = ground.normal;
                if o.enabled && o.solid {
                    if riding {
                        let Some((feet, normal)) = collider.rider_feet(carried) else {
                            continue;
                        };
                        carried = feet;
                        support_normal = normal;
                        let trace = world.body_trace(player.feet, carried);
                        if trace.start_solid || trace.fraction < 1. || !world.body_clear(carried) {
                            continue;
                        }
                    } else if collider.touches(
                        player.feet + PLAYER_CENTER,
                        player.feet + PLAYER_CENTER,
                        PLAYER_HALF,
                    ) {
                        continue; // Stop instead of crushing or passing through Alice.
                    }
                }
                let o = &mut self.objects[i];
                if riding {
                    player.feet = carried;
                    player.grounded = true;
                    player.ground_normal = support_normal;
                }
                o.pose = next;
                o.collider = collider;
                let motion = o.motion.as_mut().unwrap();
                motion.time = time;
                if time >= motion.keys.last().unwrap().0 {
                    if motion.repeat {
                        motion.time = 0.;
                    } else {
                        o.motion = None;
                    }
                }
                if matches!(o.name.as_str(), "secret_shelf" | "shelf_book") {
                    self.refresh_bound_book(map)?;
                }
            }
            if self.return_visit.as_ref().is_some_and(|r| {
                r.phase == crate::school_return::Phase::Rising
                    && r.scene.as_ref().is_none_or(|s| !s.active)
            }) && self.object("observatory_lift").motion.is_none()
            {
                self.return_visit
                    .as_mut()
                    .unwrap()
                    .enter(crate::school_return::Phase::Observatory);
            }
            for i in 0..4 {
                if matches!(self.books[i], Book::Flying | Book::Descending)
                    && self
                        .object(&format!("flyingsupport{}", i + 1))
                        .motion
                        .is_none()
                {
                    if self.books[i] == Book::Flying {
                        let bridge = self.object(&format!("t{}", 185 - i));
                        if bridge.collider.touches(
                            player.feet + PLAYER_CENTER,
                            player.feet + PLAYER_CENTER,
                            PLAYER_HALF,
                        ) {
                            continue;
                        }
                        self.books[i] = Book::Bridge;
                        self.object_mut(&format!("t{}", 185 - i)).enabled = true;
                    } else {
                        let support = self.object(&format!("flyingsupport{}", i + 1));
                        if support.collider.touches(
                            player.feet + PLAYER_CENTER,
                            player.feet + PLAYER_CENTER,
                            PLAYER_HALF,
                        ) {
                            continue;
                        }
                        self.books[i] = Book::Landing;
                        self.object_mut(&format!("flyingsupport{}", i + 1)).solid = true;
                    }
                }
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
}

/// Reuse the checked TAN reader and world material: all book art comes from the user's data.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_cinema: Option<cinema::State>,
    objects: Vec<ObjectSave>,
    books: [Book; 4],
    book_times: [f32; 4],
    returning: bool,
    #[serde(default)]
    pub return_visit: Option<crate::school_return::State>,
    theatre: bool,
    library: bool,
    shelves: [bool; 2],
    recipe: bool,
    recipe_time: f32,
    clock: f32,
    secret_open: bool,
    secret_claimed: bool,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ObjectSave {
    name: String,
    pose: Pose,
    enabled: bool,
    solid: bool,
    motion: Option<Motion>,
}
impl Snapshot {
    pub fn legacy_return(&self) -> bool {
        self.returning && self.return_visit.is_none()
    }
}
impl School {
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            first_cinema: self.first_cinema.clone(),
            objects: self
                .objects
                .iter()
                .map(|o| ObjectSave {
                    name: o.name.clone(),
                    pose: o.pose,
                    enabled: o.enabled,
                    solid: o.solid,
                    motion: o.motion.clone(),
                })
                .collect(),
            books: self.books,
            book_times: self.book_times,
            returning: self.returning,
            return_visit: self.return_visit.clone(),
            theatre: self.theatre,
            library: self.library,
            shelves: self.shelves,
            recipe: self.recipe,
            recipe_time: self.recipe_time,
            clock: self.clock,
            secret_open: self.secret_open,
            secret_claimed: self.secret_claimed,
        }
    }
    pub fn restore(&mut self, s: &Snapshot, map: &Bsp) -> Result<()> {
        if let Some(c) = &s.first_cinema {
            anyhow::ensure!(
                !s.returning && s.theatre,
                "School scene belongs to first visit after theatre contact"
            );
            c.validate()?;
        }
        self.first_cinema = s.first_cinema.clone();
        anyhow::ensure!(
            self.returning == s.returning,
            "Saved school visit does not match map"
        );
        if let Some(r) = &s.return_visit {
            r.validate()?;
            self.return_visit = Some(r.clone());
        }
        let mut seen = std::collections::BTreeSet::new();
        for v in &s.objects {
            anyhow::ensure!(seen.insert(&v.name), "Duplicate saved school object");
            let o = self
                .objects
                .iter_mut()
                .find(|o| o.name == v.name)
                .context("Unknown school object")?;
            anyhow::ensure!(o.name == v.name, "Saved school object does not match map");
            if let Some(m) = &v.motion {
                anyhow::ensure!(
                    m.keys.len() >= 2
                        && m.keys.len() <= 256
                        && m.time >= 0.
                        && m.keys[0].0 == 0.
                        && m.keys.windows(2).all(|k| k[1].0 > k[0].0),
                    "Invalid saved school motion"
                );
            }
            o.pose = v.pose;
            o.enabled = v.enabled;
            o.solid = v.solid;
            o.motion = v.motion.clone();
            o.collider = Collider::model(map, o.model, o.pose.origin, o.pose.rotation(), true)?;
        }
        for o in &mut self.objects {
            if seen.contains(&o.name) {
                continue;
            }
            // Earlier saves predate the ambient doors. Recreate just those
            // leaves; preserve every saved puzzle object and scene commitment.
            if flapping_door(&o.name) {
                o.pose = o.base;
                o.enabled = true;
                o.solid = true;
                o.motion = door_motion(&o.name, o.base);
                o.collider = Collider::model(map, o.model, o.base.origin, o.base.rotation(), true)?;
            } else {
                anyhow::ensure!(
                    s.returning
                        && s.return_visit.is_none()
                        && RETURN_OBJECTS.contains(&o.name.as_str()),
                    "Missing saved school object {}",
                    o.name
                );
            }
        }
        self.books = s.books;
        self.book_times = s.book_times;
        self.theatre = s.theatre;
        self.library = s.library;
        self.shelves = s.shelves;
        self.recipe = s.recipe;
        self.recipe_time = s.recipe_time;
        self.clock = s.clock;
        self.secret_open = s.secret_open;
        self.secret_claimed = s.secret_claimed;
        self.refresh_bound_book(map)?;
        Ok(())
    }
}
pub struct Art {
    pub first: cinema::Art,
    pub(crate) returning: crate::school_return::Art,
    glass: Prop,
    flying: Vec<Prop>,
    recipe: Vec<Prop>,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(
        assets: &mut Assets,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        Ok(Self {
            first: cinema::Art::load(assets, specs)?,
            returning: crate::school_return::Art::load(assets, specs)?,
            glass: Prop::load_animation(assets, "w_lookingglass", "idle", specs)?,
            material: crate::character::skin_material()?,
            flying: ["idle", "open", "flying", "flying2"]
                .iter()
                .map(|a| Prop::load_animation(assets, "c_flyingbook", a, specs))
                .collect::<Result<_>>()?,
            recipe: [
                "idle", "tumble", "idle02", "sway", "open", "paging01", "paging02", "paging03",
            ]
            .iter()
            .map(|a| Prop::load_animation(assets, "obj_bitterbook", a, specs))
            .collect::<Result<_>>()?,
        })
    }
    pub fn draw(
        &mut self,
        school: &School,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        if let Some(r) = &school.return_visit {
            self.material.atmosphere(atmosphere, camera);
            self.material.bind();
            self.returning
                .draw(school, r, fullbright, atmosphere, camera);
            gl_use_default_material();
            return;
        }
        self.first.draw(school, atmosphere, camera, fullbright);
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        if !school.secret_claimed {
            self.glass.draw(
                Transform {
                    translation: school.secret_reward,
                    rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
                },
                1.,
                fullbright,
            );
        }
        for i in 0..4 {
            let (clip, time, looping) = match school.books[i] {
                Book::Waiting => (0, 0., true),
                Book::Flying if school.book_times[i] < 0.55 => (1, school.book_times[i], false),
                Book::Flying | Book::Descending => (2, school.book_times[i], true),
                _ => (3, school.clock, true),
            };
            let support = school.object(&format!("flyingsupport{}", i + 1));
            self.flying[clip].draw_frame(
                Transform {
                    translation: support.pose.origin + school.book_offsets[i],
                    rotation: Quat::from_rotation_z(school.book_yaws[i]),
                },
                if i == 3 { 4.2 } else { 4. },
                fullbright,
                time,
                looping,
            );
        }
        if let Some((pose, time)) = school.falling_book() {
            self.recipe[1].draw_frame(pose, 1., fullbright, time, false);
        }
        if let Some((clip, pose, time)) = school.staged_book() {
            self.recipe[clip].draw_frame(pose, 1., fullbright, time, matches!(clip, 0 | 2 | 3));
            gl_use_default_material();
            return;
        }
        let (clip, pos) = if !school.recipe {
            (0, school.book_top)
        } else if school.recipe_time < 3. {
            (
                1,
                school
                    .book_top
                    .lerp(school.book_bottom, (school.recipe_time / 3.).powi(2)),
            )
        } else {
            (2, school.book_bottom)
        };
        self.recipe[clip].draw_frame(
            Transform {
                translation: pos,
                rotation: Quat::from_rotation_z(90_f32.to_radians()),
            },
            1.,
            fullbright,
            school.recipe_time,
            clip != 1,
        );
        gl_use_default_material();
    }
}

/// Staged real-data regressions for the reported shelf crossing and tipping riders.
/// The separate school-route check covers continuous traversal from the entrance.
pub fn check_footing(assets: &mut Assets) -> Result<()> {
    use crate::movement::{Controls, FixedClock};
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let school = School::load(&map, true)?;
    world.set_dynamic(school.colliders().collect());
    for fps in [30, 60, 144] {
        let mut cases = 0;
        for y in (3220..=3380).step_by(20) {
            for x in (-120..=560).step_by(20) {
                let start = vec3(x as f32, y as f32, 540.);
                if !world.body_clear(start) {
                    continue;
                }
                cases += 1;
                let mut p = Player::new(start);
                let mut clock = FixedClock::default();
                for _ in 0..fps * 3 {
                    clock.advance(1. / fps as f64, &world, &mut p, Controls::default());
                    anyhow::ensure!(
                        world.body_clear(p.feet),
                        "Drop embedded from {start:?} at {fps} Hz: {:?}",
                        p.feet
                    );
                }
                anyhow::ensure!(
                    p.grounded,
                    "Shelf drop never grounded from {start:?} at {fps} Hz: {:?}",
                    p.feet
                );
                let mut farthest = 0_f32;
                for d in 0..8 {
                    let mut walker = p.clone();
                    let mut clock = FixedClock::default();
                    let a = d as f32 * std::f32::consts::FRAC_PI_4;
                    for _ in 0..fps {
                        clock.advance(
                            1. / fps as f64,
                            &world,
                            &mut walker,
                            Controls {
                                wish: vec2(a.cos(), a.sin()),
                                ..Default::default()
                            },
                        );
                        anyhow::ensure!(
                            world.body_clear(walker.feet),
                            "Walk embedded from {start:?}, direction {d}, {fps} Hz: {:?}",
                            walker.feet
                        );
                    }
                    farthest = farthest.max(walker.feet.truncate().distance(p.feet.truncate()));
                }
                anyhow::ensure!(
                    farthest > 40.,
                    "All shelf walking directions blocked from {start:?} at {fps} Hz"
                );
                let old = p.feet;
                let mut clock = FixedClock::default();
                for frame in 0..fps / 5 {
                    clock.advance(
                        1. / fps as f64,
                        &world,
                        &mut p,
                        Controls {
                            jump: frame == 0,
                            ..Default::default()
                        },
                    );
                    anyhow::ensure!(
                        world.body_clear(p.feet),
                        "Jump embedded from {start:?} at {fps} Hz"
                    );
                }
                anyhow::ensure!(
                    p.jumps == 1 && p.feet.z > old.z + 20.,
                    "Jump failed from {start:?} at {fps} Hz: {:?}",
                    p.feet
                );
            }
        }
        anyhow::ensure!(cases == 315, "Shelf test coverage changed: {cases}");
        println!("PASS {fps} Hz: {cases} shelf landings, eight walking directions and a jump from each; no blocked or embedded bodies");
    }
    for name in ["bookshelf1", "bookshelf2"] {
        for fps in [30, 60, 144] {
            let mut s = School::load(&map, false)?;
            world.set_dynamic(s.colliders().collect());
            let o = s.object(name);
            let start =
                o.base.origin + vec3(if name == "bookshelf1" { 24. } else { -24. }, 0., 490.);
            let mut p = Player::new(start);
            for _ in 0..120 {
                p.tick(&world, Controls::default());
            }
            anyhow::ensure!(
                p.grounded && world.body_clear(p.feet),
                "Rider start invalid: {name}"
            );
            s.event(if name == "bookshelf1" {
                "push_bookshelf1"
            } else {
                "push_bookshelf2"
            });
            let mut clock = FixedClock::default();
            for t in 0..fps * 3 {
                s.advance(1. / fps as f32, &map, &mut world, &mut p, &[])?;
                // Check before physics: the movement overlap correction must not hide
                // a mover that pushed Alice inside its own newly rotated surface.
                anyhow::ensure!(
                    world.body_clear(p.feet),
                    "Tipping shelf embedded rider: {name}, frame {t}, {fps} Hz, {:?}",
                    p.feet
                );
                clock.advance(1. / fps as f64, &world, &mut p, Controls::default());
                anyhow::ensure!(
                    world.body_clear(p.feet),
                    "Rider embedded after physics: {name} {fps} Hz"
                );
            }
            anyhow::ensure!(
                (s.object(name).pose.angles.y.abs() - 45.).abs() < 0.001,
                "Shelf stalled with rider: {name} {fps} Hz"
            );
            anyhow::ensure!(p.grounded, "Rider failed to settle: {name} {fps} Hz");
            p.tick(
                &world,
                Controls {
                    jump: true,
                    ..Default::default()
                },
            );
            anyhow::ensure!(p.jumps == 1, "Rider cannot jump: {name} {fps} Hz");
            println!("PASS {name} {fps} Hz: carried through full tilt, body clear before/after physics, jump works");
        }
    }
    Ok(())
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let cinema_map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    door_check::check(assets, &cinema_map)?;
    cinema_check::check(assets, &cinema_map)?;
    use crate::interaction::Interactions;
    use crate::movement::{Controls, FIXED_DT};
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let mut interactions = Interactions::load(&map)?;
    interactions.set_entry(assets, &map, "skool1", None)?;
    interactions.sync(&mut world);
    let mut player = Player::spawn(&world, map.spawn().0).context("School entrance blocked")?;
    let s = interactions.school.as_mut().unwrap();
    // The scene matrix above exercises timing; these fixtures isolate machinery.
    s.cinema_data = None;
    anyhow::ensure!(
        !s.trigger_enabled("toskool2", "", Some("skool2")),
        "Recipe exit open at start"
    );
    anyhow::ensure!(
        !s.trigger_enabled("shelf_trigger", "Skool1_OG_MoveShelf", None),
        "Library opens before theatre"
    );
    anyhow::ensure!(
        !s.trigger_enabled("ig_trigger", "Book_Ingredients_Exit", None),
        "Recipe triggers at start"
    );
    // Probe real trigger volumes, rather than calling script handlers directly.
    let trigger =
        |interactions: &mut Interactions, feet: Vec3| interactions.triggers(FIXED_DT, feet, feet);
    trigger(&mut interactions, vec3(-2416., 2368., -480.));
    trigger(&mut interactions, vec3(204., 2436., 704.));
    interactions.triggers(0., vec3(-2514., 2368., -480.), vec3(-2514., 2368., -480.));
    anyhow::ensure!(
        interactions
            .school
            .as_ref()
            .is_some_and(|s| !s.library && !s.recipe && !s.theatre),
        "Paused or out-of-order contact advanced the school"
    );
    trigger(&mut interactions, vec3(-2514., 2368., -480.));
    anyhow::ensure!(
        interactions.school.as_ref().unwrap().theatre,
        "Theatre contact failed"
    );
    trigger(&mut interactions, vec3(-2416., 2368., -480.));
    anyhow::ensure!(
        interactions.school.as_ref().unwrap().library,
        "Library contact failed"
    );
    for _ in 0..1250 {
        interactions.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
    }
    anyhow::ensure!(
        world.body_clear(vec3(-208., 3124., -256.)),
        "Library passage still blocked"
    );
    println!("PASS theatre -> library gate; early exit rejected");
    // Both lifts must carry an actual physics body through a whole cycle, at renderer rates.
    for (name, start, top) in [
        ("Skool1_Lift_Up", vec3(144., 4360., -263.96), 64.),
        ("Skool1_Elevator_Up", vec3(-276., 4280., 432.04), 696.),
    ] {
        for fps in [30, 60, 144] {
            let mut interactions = Interactions::load(&map)?;
            interactions.set_entry(assets, &map, "skool1", None)?;
            interactions.sync(&mut world);
            let mut rider = Player::new(start);
            rider.grounded = true;
            interactions
                .school
                .as_mut()
                .unwrap()
                .event(name)
                .context("Missing lift event")?;
            let mut highest = rider.feet.z;
            let mut clock = crate::movement::FixedClock::default();
            for _ in 0..fps * 25 {
                interactions.advance_school(1. / fps as f32, &map, &mut world, &mut rider)?;
                clock.advance(1. / fps as f64, &world, &mut rider, Controls::default());
                highest = highest.max(rider.feet.z);
                anyhow::ensure!(
                    world.body_clear(rider.feet),
                    "Lift trapped rider {name} {fps} {:?}",
                    rider.feet
                );
            }
            anyhow::ensure!(
                highest >= top - 1.,
                "Lift failed to reach top: {name} {fps}, high={highest}, end={:?}",
                rider.feet
            );
            println!("PASS rider {name} {fps} Hz reached {highest:.2}");
        }
    }
    // A blocked lift must stop its own clock and resume when clearance returns.
    {
        let mut s = School::load(&map, false)?;
        let mut rider = Player::new(vec3(-276., 4360., 400.04));
        rider.grounded = true;
        s.event("Skool1_Elevator_Up");
        let roof = Collider::model(
            &map,
            s.object("t185").model,
            vec3(-276., 4360., 600.),
            Quat::IDENTITY,
            true,
        )?;
        let before = rider.feet;
        s.advance(
            0.,
            &map,
            &mut world,
            &mut rider,
            std::slice::from_ref(&roof),
        )?;
        anyhow::ensure!(
            rider.feet == before && s.clock == 0.,
            "Paused school mover advanced"
        );
        for _ in 0..1200 {
            s.advance(
                FIXED_DT,
                &map,
                &mut world,
                &mut rider,
                std::slice::from_ref(&roof),
            )?;
            rider.tick(&world, Controls::default());
        }
        anyhow::ensure!(
            rider.feet.z > 490. && rider.feet.z < 513. && world.body_clear(rider.feet),
            "Lift did not stop at obstruction: {:?}",
            rider.feet
        );
        for _ in 0..360 {
            s.advance(FIXED_DT, &map, &mut world, &mut rider, &[])?;
            rider.tick(&world, Controls::default());
        }
        anyhow::ensure!(rider.feet.z > 695., "Blocked lift did not resume");
        println!(
            "PASS pause and blocked lift: no crushing, no timer jump, resumes after clearance"
        );
    }
    for p in [
        vec3(-212., 3820., 704.),
        vec3(-468., 3690., 384.),
        vec3(884., 3960., 64.),
        vec3(940., 3324., -256.),
    ] {
        trigger(&mut interactions, p);
    }
    let entrance = player.feet;
    player.feet = vec3(-150., 3396., 676.);
    for _ in 0..1100 {
        interactions.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
    }
    anyhow::ensure!(
        interactions.school.as_ref().unwrap().books[0] == Book::Flying
            && world.body_clear(player.feet),
        "Book platform appeared inside Alice"
    );
    player.feet = entrance;
    interactions.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
    anyhow::ensure!(
        interactions.school.as_ref().unwrap().books == [Book::Bridge; 4],
        "Four book contacts did not build bridge: {:?}",
        interactions.school.as_ref().unwrap().books
    );
    trigger(&mut interactions, vec3(204., 2436., 704.));
    for _ in 0..400 {
        interactions.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
    }
    let events = trigger(&mut interactions, vec3(200., 2776., -256.));
    anyhow::ensure!(
        events.transition == Some(("skool2".into(), Some("skool2_start1".into()))),
        "Recipe exit not reached"
    );
    println!("PASS four book contacts -> bridge -> recipe -> skool2; this is a state/contact test, not a continuous playthrough");
    interactions.reset_contacts();
    anyhow::ensure!(
        interactions.school.as_ref().unwrap().recipe,
        "Retry cleared school progress"
    );
    interactions.set_entry(assets, &map, "skool1", Some("skool1_start2"))?;
    let s = interactions.school.as_ref().unwrap();
    anyhow::ensure!(
        !s.trigger_enabled("", "", Some("skool2"))
            && !s.trigger_enabled("", "start_book1", None)
            && !s.object("observatory_clip").enabled
            && !s.object("flyingsupport1").enabled
            && s.library,
        "Return visit leaked first-visit puzzle state"
    );
    interactions.set_entry(assets, &map, "skool1", None)?;
    anyhow::ensure!(
        !interactions.school.as_ref().unwrap().library
            && !interactions.school.as_ref().unwrap().recipe,
        "Fresh visit retained progress"
    );
    println!("PASS retry retains progress; return/fresh visits rebuild their own puzzle state");
    Ok(())
}

/// Native renderer diagnostic. These staged contacts are separate from the traversal test.
pub async fn check_render(assets: &mut Assets) -> Result<()> {
    crate::render::check_school_floor(assets).await?;
    if std::env::var_os("LOOKING_GLASS_SCHOOL_FLOOR_CHECK").is_some() { return Ok(()); }
    door_check::render(assets).await?;
    cinema_check::render(assets).await?;
    let mut scene = crate::render::Scene::load(assets, "skool1")?;
    let specs = texture::read_materials(assets)?;
    let mut art = Art::load(assets, &specs)?;
    let mut interactions = crate::interaction::Interactions::load(&scene.map)?;
    interactions.set_entry(assets, &scene.map, "skool1", None)?;
    // The separate scene matrix above exercises cinematics. Stage these older
    // machinery-only views with their original instantaneous commitments.
    interactions.school.as_mut().unwrap().cinema_data = None;
    interactions.sync(&mut scene.world);
    let mut player =
        Player::spawn(&scene.world, scene.map.spawn().0).context("School spawn blocked")?;
    for (stage, eye, target) in [
        (
            "waiting",
            vec3(944., 3440., -170.),
            vec3(1000., 3320., -180.),
        ),
        ("bridge", vec3(-130., 3540., 806.), vec3(200., 2700., 695.)),
        ("recipe", vec3(200., 3100., -160.), vec3(200., 2776., -248.)),
    ] {
        if stage == "bridge" {
            let s = interactions.school.as_mut().unwrap();
            for name in [
                "Theatre_Cinematic",
                "Skool1_OG_MoveShelf",
                "shelf_cinematic",
                "start_book1",
                "start_book2",
                "start_book3",
                "start_book4",
            ] {
                s.event(name);
            }
        }
        if stage == "recipe" {
            interactions
                .school
                .as_mut()
                .unwrap()
                .event("book_cinematic");
        }
        for _ in 0..1250 {
            interactions.advance_school(1. / 120., &scene.map, &mut scene.world, &mut player)?;
        }
        for frame in 0..90 {
            interactions.advance_school(1. / 60., &scene.map, &mut scene.world, &mut player)?;
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: eye,
                target,
                up: Vec3::Z,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            let transforms = interactions.transforms();
            scene.draw(eye, frame as f32 / 60., false, false, &transforms);
            art.draw(
                interactions.school.as_ref().unwrap(),
                false,
                &scene.atmosphere,
                eye,
            );
            crate::render::depth_read_only(|| {
                scene.draw(eye, frame as f32 / 60., false, true, &transforms)
            });
            set_default_camera();
            draw_text(
                &format!("SCHOOL VISUAL CHECK / {stage}"),
                24.,
                35.,
                24.,
                WHITE,
            );
            if frame == 89 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school-{stage}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    println!("PASS school TAN clips and moving brush rendering; inspect the three captures for appearance");
    Ok(())
}

impl School {
    /// Explicit staged fixture, never used by ordinary level entry.
    pub fn return_fixture(
        &mut self,
        phase: &str,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        stats: &mut crate::inventory::Stats,
    ) -> Result<()> {
        stats.school_items.star = true;
        stats.school_items.potion = true;
        self.sync_inventory(stats);
        if phase == "return" {
            return Ok(());
        }
        *player = Player::new(vec3(692., 4370., 384.03125));
        player.grounded = true;
        self.event("Skool1_Setup_OLift");
        for _ in 0..150 {
            self.advance(1. / 120., map, world, player, &[])?;
        }
        self.event("Skool1_OLift_Up");
        let lift_ticks = match phase {
            "return-star-start" => 0,
            "return-star" => 240,
            "return-star-close" => 570,
            "return-star-skipped" => {
                self.skip_return_scene();
                24
            }
            "return-lift" => 1020,
            _ => 1440,
        };
        for _ in 0..lift_ticks {
            self.advance(1. / 120., map, world, player, &[])?;
        }
        if ![
            "return-star-start",
            "return-star",
            "return-star-close",
            "return-star-skipped",
            "return-lift",
            "return-observatory",
        ]
        .contains(&phase)
        {
            *player = Player::new(vec3(340., 4396., 1344.0313));
            self.event("observatory_exit_cinematic");
            let ticks = match phase {
                "return-exit-start" => 0,
                "return-potion" => 1080,
                "return-globe" => 470,
                "return-takeoff" => 1380,
                "return-skipped" => {
                    self.skip_return_scene();
                    24
                }
                "return-complete" => 1800,
                _ => 1570,
            };
            for _ in 0..ticks {
                self.advance(1. / 120., map, world, player, &[])?;
            }
        }
        self.sync_inventory(stats);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spiral_rotation_does_not_take_the_short_quaternion_arc() {
        let m = Motion {
            keys: vec![
                (
                    0.,
                    Pose {
                        origin: Vec3::ZERO,
                        angles: Vec3::ZERO,
                    },
                ),
                (
                    6.,
                    Pose {
                        origin: Vec3::Z * 328.,
                        angles: Vec3::Z * -540.,
                    },
                ),
            ],
            time: 0.,
            repeat: false,
        };
        let half = m.sample(3.);
        assert_eq!(half.origin.z, 164.);
        assert_eq!(half.angles.z, -270.);
        assert!((half.rotation() * Vec3::X - Vec3::Y).length() < 0.001);
    }
}

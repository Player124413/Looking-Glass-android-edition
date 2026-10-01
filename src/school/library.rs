//! Reviewed library staging. Timed presentation never grants an unsolved book puzzle.
use super::*;

pub const SHELVES: &str = "shelf_cinematic";
pub const BOOK: &str = "book_cinematic";
pub const RECIPE: &str = "Book_Ingredients_Exit";
pub const CAT: &[&str] = &[
    "sit_idle1",
    "sit_talk1",
    "sit_talk2",
    "sit_talk3",
    "sit_smile_open",
];
const SOUNDS: &[&str] = &[
    "sound/character/cheshire_cat/appear.wav",
    "sound/character/cheshire_cat/disappear.wav",
    "sound/world/mover/bookcase_fall_1.wav",
    "sound/world/mover/bookcase_fall_2.wav",
    "sound/ambience/special/book_push.wav",
    "sound/ambience/special/book_land.wav",
    "sound/ambience/special/switch.wav",
];

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub phase: u8,
    pub time: f32,
    action_time: f32,
    cues: u32,
    sounds: Vec<usize>,
    pub exit_ready: bool,
    exit_sent: bool,
}
impl State {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.action_time.is_finite()
                && (0. ..=10000.).contains(&self.action_time)
                && self.phase <= 3
                && self.time.is_finite()
                && (0. ..=10000.).contains(&self.time)
                && self.sounds.len() <= 16
                && self.sounds.iter().all(|&i| i < SOUNDS.len()),
            "Invalid library scene save"
        );
        Ok(())
    }
    pub(super) fn shelf_sounds(&mut self, t: f32, d: &Data) {
        self.cue(0, t >= d.walk_end() + 0.5, Some(6));
        self.cue(1, t >= d.shelf_start(), Some(2));
        self.cue(2, t >= d.shelf_end(), Some(3));
    }
    fn enter(&mut self, phase: u8) {
        self.phase = phase;
        self.time = 0.;
    }
    fn cue(&mut self, bit: u32, ready: bool, sound: Option<usize>) -> bool {
        if !ready || self.cues & (1 << bit) != 0 {
            return false;
        }
        self.cues |= 1 << bit;
        if let Some(sound) = sound {
            self.sounds.push(sound);
        }
        true
    }
}
impl School {
    pub fn owns_recipe_exit(&self) -> bool {
        !self.returning && self.cinema_data.is_some()
    }
    pub fn recipe_ready(&self) -> bool {
        self.recipe
            && self.recipe_time >= 3.
            && self.scene_id() != Some(BOOK)
            && self.books.iter().all(|b| *b != Book::Descending)
    }
    pub fn library_events(&mut self) -> Events {
        let mut e = Events::default();
        if let Some(l) = self.first_cinema.as_mut().and_then(|s| s.library.as_mut()) {
            if !l.sounds.is_empty() {
                e.sound = Some(SOUNDS[l.sounds.remove(0)].into());
            }
            if l.exit_ready && !l.exit_sent {
                l.exit_sent = true;
                e.transition = Some(("skool2".into(), Some("skool2_start1".into())));
            }
        }
        e
    }
    /// Old format-12 saves can contain a dialogue-owned exit. Adopt that intent,
    /// but require the new recipe staging before releasing it to the campaign.
    pub fn guard_recipe_exit(&mut self, story: &mut Story) -> bool {
        if self.returning
            || self.cinema_data.is_none()
            || story.pending_exit_map() != Some("skool2")
        {
            return true;
        }
        if self
            .first_cinema
            .as_ref()
            .and_then(|s| s.library.as_ref())
            .is_some_and(|l| l.exit_ready)
        {
            return true;
        }
        story.clear_exit();
        if self.recipe_ready() && self.scene_id().is_none() {
            self.scene_start(Beat::Recipe);
            let already_done = story.has_seen(RECIPE) && !story.sequence_pending(RECIPE);
            story.trigger(RECIPE);
            self.first_cinema.as_mut().unwrap().dialogue_done = already_done;
        }
        false
    }
    pub fn sync_library_story(&mut self, story: &Story) {
        let Some(s) = &mut self.first_cinema else {
            return;
        };
        let Some(l) = &mut s.library else {
            return;
        };
        if s.beat == Some(Beat::Book) {
            if l.phase == 0 && story.progress(BOOK).is_some_and(|p| p.0 == 1) {
                l.enter(1);
            }
            if l.phase == 1
                && l.time >= 3.85
                && story.progress(BOOK).is_some_and(|p| p.0 == 1)
                && story.line_finished()
            {
                l.enter(2);
            }
            if l.phase == 2 && l.time >= 12. && s.dialogue_done {
                l.enter(3);
            }
        } else if s.beat == Some(Beat::Recipe) && l.phase == 0 && s.time >= 0.5 && s.dialogue_done {
            l.enter(1);
        }
    }
    pub(super) fn library_step(&mut self, dt: f32) -> bool {
        let s = self.first_cinema.as_mut().unwrap();
        let beat = s.beat.unwrap();
        let l = s.library.as_mut().unwrap();
        l.time += dt;
        if beat == Beat::Book && l.phase >= 2 {
            l.action_time += dt;
        }
        let t = s.time;
        match beat {
            Beat::Shelves => {
                l.cue(0, t >= 2., Some(0));
                let tip = l.cue(1, t >= 7.5, Some(2));
                l.cue(2, t >= 9.5, Some(1));
                let done = t >= 13. && s.dialogue_done;
                if tip {
                    self.tip_shelves();
                }
                let stopped =
                    (1..=2).all(|n| self.object(&format!("bookshelf{n}")).motion.is_none());
                if stopped && t >= 9.5 {
                    self.first_cinema
                        .as_mut()
                        .unwrap()
                        .library
                        .as_mut()
                        .unwrap()
                        .cue(3, true, Some(3));
                }
                done && stopped
            }
            Beat::Book => {
                l.cue(0, true, Some(0));
                if l.phase == 2 {
                    l.cue(1, l.time >= 3.4, Some(4));
                    l.cue(2, l.time >= 6.1, Some(1));
                    l.cue(3, l.time >= 11.5, Some(5));
                }
                l.phase == 3 && l.time >= 4.5
            }
            Beat::Recipe => l.phase == 1 && l.time >= 4.5,
            _ => false,
        }
    }
    pub(super) fn tip_shelves(&mut self) {
        // Reuse the swept, rider-aware machinery; these handlers are one-shot.
        self.event("push_bookshelf1");
        self.event("push_bookshelf2");
    }
    pub(super) fn finish_book(&mut self) {
        self.recipe_time = 3.;
        self.object_mut("removeme").enabled = false;
        for i in 0..4 {
            if self.books[i] != Book::Bridge {
                continue;
            }
            self.books[i] = Book::Descending;
            self.book_times[i] = 0.;
            self.object_mut(&format!("t{}", 185 - i)).enabled = false;
            let points = self.last_paths[i].clone();
            self.follow_book(i, &points, 3., 0.);
        }
    }
    pub(super) fn library_camera(&self) -> Option<(&'static str, f32)> {
        let s = self.first_cinema.as_ref()?;
        let l = s.library.as_ref()?;
        Some(match s.beat? {
            Beat::Shelves if s.time < 7.5 => ("skool1_path5", (s.time - 1.).max(0.)),
            Beat::Shelves => ("skool1_path6", s.time - 7.5),
            Beat::Book if l.phase < 2 => ("skool1_path2", s.time),
            Beat::Book if l.phase == 2 && l.time < 5.1 => ("skool1_path2", s.time),
            Beat::Book if l.phase == 2 && l.time < 8.8 => ("skool1_path3", l.time - 5.1),
            Beat::Book => ("skool1_path4", l.action_time - 8.8),
            Beat::Recipe => ("skool1_bjbook", (s.time - 0.5).max(0.)),
            _ => return None,
        })
    }
    pub(super) fn library_fade(&self) -> Option<f32> {
        let s = self.first_cinema.as_ref()?;
        let l = s.library.as_ref()?;
        Some(match s.beat? {
            Beat::Shelves => {
                ((s.time - 11.5) / 0.5).clamp(0., 1.) * ((12.5 - s.time) / 0.5).clamp(0., 1.)
            }
            Beat::Book if l.phase == 3 => ((l.time - 4.) / 0.5).clamp(0., 1.),
            Beat::Recipe if l.phase == 1 => {
                (l.time / 0.5).clamp(0., 1.) * ((1. - l.time) / 0.5).clamp(0., 1.)
            }
            Beat::Recipe => (s.time / 0.5).clamp(0., 1.) * ((1. - s.time) / 0.5).clamp(0., 1.),
            _ => return None,
        })
    }
    pub(crate) fn falling_book(&self) -> Option<(Transform, f32)> {
        let l = self.first_cinema.as_ref()?.library.as_ref()?;
        if self.scene_id() != Some(BOOK) || l.phase != 2 || !(11. ..11.5).contains(&l.action_time) {
            return None;
        }
        let t = l.action_time;
        Some((
            Transform {
                translation: self.book_top + vec3(0., 155., 20.)
                    - Vec3::Z * 1000. * ((t - 9.) / 2.5),
                rotation: Quat::from_rotation_z(90_f32.to_radians())
                    * Quat::from_rotation_y(35_f32.to_radians()),
            },
            t - 8.7,
        ))
    }
    pub(crate) fn staged_book(&self) -> Option<(usize, Transform, f32)> {
        let s = self.first_cinema.as_ref()?.library.as_ref()?;
        if self.scene_id() != Some(BOOK) {
            return None;
        }
        let t = if s.phase < 2 { -1. } else { s.action_time };
        let mut pose = Transform {
            translation: self.book_top,
            rotation: Quat::from_rotation_z(90_f32.to_radians()),
        };
        let (clip, at) = if t >= 11. {
            pose.translation = self.book_bottom;
            let at = t - 11.;
            if at < 2.4 {
                (4, at)
            } else if at < 3.9 {
                (5, at - 2.4)
            } else if at < 5.4 {
                (6, at - 3.9)
            } else if at < 6.9 {
                (7, at - 5.4)
            } else {
                (2, at - 6.9)
            }
        } else if t >= 8.7 {
            let start = self.book_top + Vec3::Y * 65.;
            let raised = start + vec3(0., 90., 20.);
            pose.translation = if t < 9. {
                start.lerp(raised, (t - 8.7) / 0.3)
            } else {
                raised - Vec3::Z * 1000. * ((t - 9.) / 2.5).clamp(0., 1.)
            };
            pose.rotation *=
                Quat::from_rotation_y(35_f32.to_radians() * ((t - 8.7) / 0.3).clamp(0., 1.));
            (1, t - 8.7)
        } else {
            pose.translation += Vec3::Y * 65. * ((t - 3.4) / 1.7).clamp(0., 1.);
            if t >= 5.1 {
                (3, t - 5.1)
            } else {
                (0, 0.)
            }
        };
        Some((clip, pose, at))
    }
}

impl Art {
    pub(super) fn draw_library(&mut self, school: &School, bright: bool) {
        let s = school.first_cinema.as_ref().unwrap();
        let l = s.library.as_ref().unwrap();
        let d = school.cinema_data.as_ref().unwrap();
        let home = s.home.unwrap_or(d.points["alicepush"]);
        let (alice, clip, at, cat) = match s.beat.unwrap() {
            Beat::Shelves => (
                home,
                "idle",
                s.time,
                (s.time >= 2. && s.time < 11.5).then_some((
                    "shelf_cat",
                    if s.time >= 7. {
                        "sit_talk2"
                    } else {
                        "sit_idle1"
                    },
                    (s.time - 7.).max(0.),
                    ((s.time - 2.) / 2.).clamp(0., 1.) * ((11.5 - s.time) / 2.).clamp(0., 1.),
                )),
            ),
            Beat::Recipe => (
                d.points["alice_bookwatch_pos1"],
                "idle_stand_rocktoes",
                (s.time - 0.5).max(0.),
                None,
            ),
            Beat::Book => {
                let t = if l.phase < 2 { -1. } else { l.action_time };
                let mut p = travel(
                    d.points["alicepush"],
                    d.points["bookpush"],
                    ((t - 3.) / 2.1).clamp(0., 1.),
                );
                p.rotation = Quat::from_rotation_z(90_f32.to_radians());
                let (clip, at) = if (1.4..3.).contains(&t) {
                    ("idle_shrug_headtilt", t - 1.4)
                } else if (3. ..5.1).contains(&t) {
                    ("push_loop", t - 3.)
                } else if (5.1..6.1).contains(&t) {
                    ("push_end", t - 5.1)
                } else if (8.1..8.8).contains(&t) {
                    ("idle_base_01_play3", t - 8.1)
                } else {
                    ("idle", s.time)
                };
                let (c, cat_at) = if l.phase == 1 {
                    if l.time < 1.85 {
                        ("sit_talk1", l.time)
                    } else if l.time < 3.85 {
                        ("sit_talk3", l.time - 1.85)
                    } else {
                        ("sit_idle1", l.time - 3.85)
                    }
                } else if (5.1..6.1).contains(&t) {
                    ("sit_smile_open", t - 5.1)
                } else {
                    ("sit_idle1", s.time)
                };
                let target = if l.phase < 2 {
                    d.points["book_cat"].translation + Vec3::Z * 25.
                } else {
                    school.book_top + Vec3::Z * 35.
                };
                self.alice.watch(attention(p, target));
                (
                    p,
                    clip,
                    at,
                    (t < 8.1).then_some((
                        "book_cat",
                        c,
                        cat_at,
                        (s.time / 2.).clamp(0., 1.) * ((8.1 - t) / 2.).clamp(0., 1.),
                    )),
                )
            }
            _ => return,
        };
        self.alice.draw(
            clip,
            at,
            !matches!(
                clip,
                "push_end" | "idle_shrug_headtilt" | "idle_base_01_play3"
            ),
            alice,
            1.,
            bright,
        );
        if let Some((name, clip, at, alpha)) = cat {
            let pose = d.points[name];
            self.cat
                .watch(attention(pose, alice.translation + Vec3::Z * 55.));
            self.cat
                .draw_dissolving(clip, at, true, pose, 1., bright, 1. - alpha);
        }
    }
}
fn attention(pose: Transform, target: Vec3) -> crate::facial::Watch {
    let mut watch = crate::facial::Watch::default();
    for _ in 0..30 {
        watch.update(
            1. / 60.,
            Some(pose.rotation.conjugate() * (target - pose.translation - Vec3::Z * 45.)),
        );
    }
    watch
}

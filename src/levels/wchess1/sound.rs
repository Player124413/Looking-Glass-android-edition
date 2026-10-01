use super::*;
impl Realm {
    pub(super) fn sounds(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        for (i, t) in self.saved.lever_started.into_iter().enumerate() {
            if let Some(t) = t {
                clocks.push(crate::audio::world::Clock {
                    key: if i == 0 {
                        "wchess1.bell-pull"
                    } else {
                        "wchess1.water-pull"
                    },
                    time: self.saved.age - t,
                    period: None,
                    origin: self
                        .data
                        .point(if i == 0 { "bell_lever" } else { "water_lever" })
                        .translation,
                    cues: &[(1.05, "sound/world/machine/lever1.wav")],
                });
            }
        }
        if let Some(t) = self.saved.refusal {
            clocks.push(crate::audio::world::Clock {
                key: "wchess1.rook-refusal",
                time: self.saved.age - t,
                period: None,
                origin: self.actor("rook_guard1").unwrap().piece.feet,
                cues: &[(0., "sound/character/chess_piece/rook/twitchc.wav")],
            });
        }
        if let Some(t) = self.saved.water {
            loops.push(crate::audio::LoopCue {
                id: BASE + 90_001,
                clock: Some(self.saved.age - t),
                path: "sound/ambience/special/water_wheel2.wav",
                origin: self.data.point("waterwheel_start").translation,
            });
        }
        if let Some(s) = &self.saved.scene {
            if matches!(s.kind, scene::Kind::Intro | scene::Kind::Bell) {
                clocks.push(crate::audio::world::Clock {
                    key: if s.kind == scene::Kind::Intro {
                        "wchess1.intro-bell"
                    } else {
                        "wchess1.lever-bell"
                    },
                    time: s.clock.time,
                    period: None,
                    origin: self.data.point("bell_sound").translation,
                    cues: if s.kind == scene::Kind::Intro {
                        &[(1., "sound/ui/quit_c.wav")]
                    } else {
                        &[(0.5, "sound/ui/quit_c.wav")]
                    },
                });
            }
            if matches!(s.kind, scene::Kind::Bishop | scene::Kind::Knight) {
                clocks.push(crate::audio::world::Clock {
                    key: if s.kind == scene::Kind::Bishop {
                        "wchess1.bishop-effect"
                    } else {
                        "wchess1.knight-effect"
                    },
                    time: (s.clock.time - self.scene_times(s)[3]).max(0.),
                    period: None,
                    origin: self.alice_scene_pose().unwrap().0.translation,
                    cues: &[(0.001, "sound/character/gnome/elder/vanish.wav")],
                });
            }
        }
        if matches!(self.saved.elevator.phase, 0 | 3) {
            clocks.push(crate::audio::world::Clock {
                key: if self.saved.elevator.phase == 0 {
                    "wchess1.lift-down"
                } else {
                    "wchess1.lift-up"
                },
                time: self.saved.elevator.time,
                period: None,
                origin: self.data.point("elevator").translation
                    + Vec3::Z * self.saved.elevator.height,
                cues: &[(0., "sound/ambience/special/lift_2.wav")],
            });
        }
        const NAMES: [&str; 10] = [
            "bsspike1", "bsspike2", "bsspike3", "bsspike4", "bsspike5", "knspike1", "knspike2",
            "knspike3", "knspike4", "knspike5",
        ];
        const KEYS: [&str; 10] = [
            "wchess1.bs1",
            "wchess1.bs2",
            "wchess1.bs3",
            "wchess1.bs4",
            "wchess1.bs5",
            "wchess1.kn1",
            "wchess1.kn2",
            "wchess1.kn3",
            "wchess1.kn4",
            "wchess1.kn5",
        ];
        for (n, key) in NAMES.into_iter().zip(KEYS) {
            if let Some(o) = self.objects.iter().find(|o| o.name == n) {
                clocks.push(crate::audio::world::Clock {
                    key,
                    time: (self.saved.age - (o.id % 17) as f32 / 34.).max(0.),
                    period: Some(1.),
                    origin: o.pose.translation,
                    cues: &[(0., "sound/world/mover/spike.wav")],
                });
            }
        }
    }
}

use super::*;
impl Funhouse {
    pub fn world_sound(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        use crate::audio::world::Clock;
        if let Some(start) = self.saved.tube {
            let t = self.saved.time - start;
            let origin = self.data.points["tuber_bind"].translation;
            clocks.push(Clock {
                key: "funhouse/tube",
                time: t,
                period: None,
                origin,
                cues: &[
                    (0., "sound/world/mover/tube_start.wav"),
                    (8.25, "sound/world/mover/tube_end.wav"),
                ],
            });
            if (0.25..8.25).contains(&t) {
                loops.push(crate::audio::LoopCue {
                    id: BASE + 99_001,
                    clock: Some(t - 0.25),
                    path: "sound/world/mover/tube_loop.wav",
                    origin,
                });
            }
        }
        for (key, name, cues) in [
            (
                "funhouse/head1",
                "head1_steam1",
                &[(2., "sound/ambience/special/suck_1.wav")][..],
            ),
            (
                "funhouse/head2",
                "head2_steam1",
                &[(2., "sound/ambience/special/suck_2.wav")][..],
            ),
            (
                "funhouse/head3",
                "head3_steam1",
                &[(2., "sound/ambience/special/suck_1.wav")][..],
            ),
        ] {
            if let Some(p) = self.data.points.get(name) {
                clocks.push(Clock {
                    key,
                    time: self.saved.time,
                    period: Some(7.),
                    origin: p.translation,
                    cues,
                });
            }
        }
        if let Some(start) = self.saved.gas_start.filter(|_| !self.saved.gas) {
            clocks.push(Clock {
                key: "funhouse/gas",
                time: self.saved.time - start,
                period: None,
                origin: self.data.points["steam1"].translation,
                cues: &[
                    (2., "sound/ambience/special/steam_blow.wav"),
                    (4., "sound/ambience/special/steam_blow2.wav"),
                    (5., "sound/ambience/special/steam_blow.wav"),
                    (6.5, "sound/ambience/special/steam_blow.wav"),
                    (6.8, "sound/ambience/special/steam_blow3.wav"),
                    (7.1, "sound/ambience/special/steam_blow.wav"),
                    (7.6, "sound/ambience/special/steam_pipe_burst.wav"),
                    (7.7, "sound/ambience/special/thronebreak1.wav"),
                ],
            });
        }
        if let Some(start) = self.saved.floor {
            clocks.push(Clock {
                key: "funhouse/floor",
                time: self.saved.time - start,
                period: None,
                origin: self.data.points["break_plat"].translation,
                cues: &[
                    (0., "sound/ambience/special/roomsplit.wav"),
                    (0.4, "sound/ambience/special/thronebreak3.wav"),
                    (0.7, "sound/ambience/special/rock_falling1.wav"),
                    (1.3, "sound/ambience/special/thronebreak1.wav"),
                    (1.5, "sound/ambience/special/rock_falling1.wav"),
                    (2.1, "sound/ambience/special/rock_falling1.wav"),
                    (2.2, "sound/ambience/special/thronebreak2.wav"),
                ],
            });
        }
        if let Some(s) = &self.saved.scene {
            if s.kind == Kind::Arrival {
                clocks.push(Clock {
                    key: "funhouse/arrival",
                    time: s.clock.time,
                    period: None,
                    origin: self.data.points["cat_pos1"].translation,
                    cues: &[
                        (12., "sound/ambience/special/glassimpact2.wav"),
                        (14.5, "sound/ambience/special/glass_break_4.wav"),
                        (26.1, "sound/character/cheshire_cat/appear.wav"),
                    ],
                });
            }
        }
    }
}

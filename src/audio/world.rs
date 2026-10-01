//! Audio follows the same mover clocks used by collision and saves.
use super::LoopCue;
use macroquad::prelude::Vec3;
use std::collections::BTreeMap;

pub struct Clock {
    pub key: &'static str,
    pub time: f32,
    pub period: Option<f32>,
    pub origin: Vec3,
    pub cues: &'static [(f32, &'static str)],
}
#[derive(Default)]
pub struct State {
    clocks: BTreeMap<&'static str, f32>,
    seeded: bool,
}
impl State {
    pub fn update(&mut self, clocks: Vec<Clock>) -> Vec<(&'static str, Vec3)> {
        let mut out = Vec::new();
        let mut next = BTreeMap::new();
        for clock in clocks {
            let previous = self.clocks.get(clock.key).copied();
            // Load/skip seeks reconstruct loops but never replay historical one-shots.
            let start = previous.unwrap_or(if self.seeded && clock.time < 0.25 {
                -0.001
            } else {
                clock.time
            });
            if clock.time > start {
                for &(at, path) in clock.cues {
                    let crossed = if let Some(period) = clock.period {
                        let cycle = ((start - at) / period).floor() + 1.;
                        let when = at + cycle.max(0.) * period;
                        when > start && when <= clock.time
                    } else {
                        at > start && at <= clock.time
                    };
                    if crossed {
                        out.push((path, clock.origin));
                    }
                }
            }
            next.insert(clock.key, clock.time);
        }
        self.clocks = next;
        self.seeded = true;
        out
    }
}

pub fn collect(
    interactions: &crate::interaction::Interactions,
    loops: &mut Vec<LoopCue>,
) -> Vec<Clock> {
    let mut clocks = Vec::new();
    if let Some(f) = &interactions.fortress {
        f.cinema.sound_state(&f.state.cinema, loops, &mut clocks);
    }
    if let Some(p) = &interactions.pandemonium {
        p.sound_state(loops, &mut clocks);
    }
    if let Some(s) = &interactions.school {
        s.sound_state(loops, &mut clocks);
    }
    if let Some(s) = &interactions.school2 {
        s.potion_sound(loops);
    }
    if let Some(v) = &interactions.village {
        v.sound_state(&mut clocks);
    }
    for s in &interactions.levels {
        s.ctl.sound_state(loops, &mut clocks);
    }
    clocks
}

/// Archive-backed check of actual level clocks, including save restoration.
pub fn check(assets: &mut crate::assets::Assets) -> anyhow::Result<()> {
    use crate::{bsp::Bsp, collision::World, movement::Player};
    use anyhow::ensure;
    let map = Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let mut player = Player::new(map.spawn().0);
    let mut school = crate::school::School::load(&map, false)?;
    let mut observer = State::default();
    observer.update(Vec::new());
    school.event("push_bookshelf1");
    let mut emitted = Vec::new();
    for _ in 0..180 {
        school.advance(1. / 60., &map, &mut world, &mut player, &[])?;
        let mut clocks = Vec::new();
        school.sound_state(&mut Vec::new(), &mut clocks);
        emitted.extend(observer.update(clocks));
    }
    ensure!(
        emitted.len() == 2
            && emitted[0].0.ends_with("fall_1.wav")
            && emitted[1].0.ends_with("fall_2.wav"),
        "Bookcase sound sequence missing/duplicated"
    );
    school.event("Skool1_Lift_Up");
    for _ in 0..120 {
        school.advance(1. / 60., &map, &mut world, &mut player, &[])?;
    }
    let mut loops = Vec::new();
    school.sound_state(&mut loops, &mut Vec::new());
    loops.retain(|l| l.id == 1001);
    ensure!(loops.len() == 1, "Moving school lift is silent");
    let saved = school.snapshot();
    let mut restored = crate::school::School::load(&map, false)?;
    restored.restore(&saved, &map)?;
    let mut after = Vec::new();
    let mut clocks = Vec::new();
    restored.sound_state(&mut after, &mut clocks);
    after.retain(|l| l.id == 1001);
    ensure!(
        after.len() == 1 && after[0].clock == loops[0].clock && after[0].origin == loops[0].origin,
        "Restored lift audio lost its phase/position"
    );
    ensure!(
        State::default().update(clocks).is_empty(),
        "Loading replayed a bookcase impact"
    );

    let map = Bsp::parse(&assets.read("maps/pandemonium.bsp")?)?;
    let mut world = World::from_bsp(&map)?;
    let mut cart = crate::pandemonium::Pandemonium::load(assets, &map)?;
    for (phase, expected) in [
        ("pand-cart", Some("mine_lift1.wav")),
        ("pand-rail", Some("mine_lift3.wav")),
        ("pand-landing", None),
    ] {
        cart.fixture(phase, &map, &mut world, &mut player)?;
        let mut loops = Vec::new();
        let mut clocks = Vec::new();
        cart.sound_state(&mut loops, &mut clocks);
        ensure!(
            loops.first().map(|l| l.path.rsplit('/').next().unwrap()) == expected,
            "Incorrect {phase} sound loop"
        );
        ensure!(
            State::default().update(clocks).is_empty(),
            "Cart load replayed a past cue"
        );
        let saved = cart.snapshot();
        cart.restore(&saved, &map)?;
        let mut after = Vec::new();
        cart.sound_state(&mut after, &mut Vec::new());
        ensure!(
            after.len() == loops.len()
                && after
                    .iter()
                    .zip(loops)
                    .all(|(a, b)| a.path == b.path && a.clock == b.clock && a.origin == b.origin),
            "Cart audio differs after restore"
        );
    }
    cart.state.cart = crate::pandemonium::Cart::Rail;
    cart.state.cinema.beat = None;
    let mut observer = State::default();
    observer.update(Vec::new());
    let mut effects = Vec::new();
    for tick in 0..18000 {
        cart.state.time = tick as f32 / 60.;
        let mut clocks = Vec::new();
        cart.sound_state(&mut Vec::new(), &mut clocks);
        effects.extend(observer.update(clocks));
    }
    for name in ["mine_lift7.wav", "mine_lift4.wav", "mine_lift5.wav"] {
        ensure!(
            effects.iter().filter(|(p, _)| p.ends_with(name)).count() == 2,
            "Each track collapse must emit its {name} cue once"
        );
    }
    let map = Bsp::parse(&assets.read("maps/gvillage.bsp")?)?;
    let mut cinema = crate::village::cinema::Cinema::load(assets, &map)?;
    cinema.state.beat = Some(crate::village::cinema::Beat::Fall);
    let mut observer = State::default();
    observer.update(Vec::new());
    let mut cues = Vec::new();
    for tick in 0..600 {
        cinema.state.time = tick as f32 / 60.;
        let mut clocks = Vec::new();
        cinema.sound_state(&mut clocks);
        cues.extend(observer.update(clocks));
    }
    ensure!(
        cues.len() == 2
            && cues[0].0.ends_with("death_fall.wav")
            && cues[1].0.ends_with("pain_knockdown.wav"),
        "Village fall/landing sound timing differs"
    );
    let mut clocks = Vec::new();
    cinema.sound_state(&mut clocks);
    ensure!(
        State::default().update(clocks).is_empty(),
        "Loading village fall replayed impact"
    );
    for (path, _) in effects.iter().chain(&cues) {
        let bytes = assets.read(path)?;
        let mut wav = hound::WavReader::new(std::io::Cursor::new(bytes))?;
        ensure!(
            wav.duration() > 0 && wav.samples::<i16>().any(|s| s.is_ok_and(|s| s != 0)),
            "Silent or invalid cinematic recording: {path}"
        );
    }
    println!("Audio world check: bookcase/lift/cart loops, both track-collapse cue sequences, village fall/landing recordings, and saved phase restoration passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clock(time: f32) -> Clock {
        Clock {
            key: "roller",
            time,
            period: Some(7.7),
            origin: Vec3::ZERO,
            cues: &[(0., "sound/roll1.wav"), (4.7, "sound/roll2.wav")],
        }
    }
    #[test]
    fn mover_sounds_follow_saved_clock_without_pause_or_restart_replay() {
        let mut state = State::default();
        assert!(state.update(vec![clock(4.8)]).is_empty());
        assert!(state.update(vec![clock(4.8)]).is_empty());
        assert_eq!(state.update(vec![clock(7.8)]).len(), 1);
        assert!(state.update(vec![clock(7.8)]).is_empty());
        assert_eq!(state.update(vec![clock(12.5)]).len(), 1);
        let mut restored = State::default();
        assert!(restored.update(vec![clock(12.5)]).is_empty());
        restored.update(Vec::new());
        assert_eq!(restored.update(vec![clock(0.1)]).len(), 1);
    }
}

//! Bounded, independently implemented obstruction and early reflections.
use crate::collision::World;
use macroquad::prelude::*;
use rodio::Source;
use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct Controls {
    cutoff: AtomicU32,
    wet: AtomicU32,
}
impl Controls {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            cutoff: AtomicU32::new(18000f32.to_bits()),
            wet: AtomicU32::new(0f32.to_bits()),
        })
    }
    pub fn set(&self, obstruction: f32, submerged: bool, room: f32) {
        self.cutoff.store(
            (if submerged {
                650.
            } else {
                18000. - obstruction * 16500.
            })
            .to_bits(),
            Ordering::Relaxed,
        );
        self.wet.store(
            (room * if submerged { 0.04 } else { 0.16 }).to_bits(),
            Ordering::Relaxed,
        );
    }
}
pub fn obstruction(world: &World, ear: Vec3, source: Vec3) -> f32 {
    let delta = source - ear;
    if delta.length() < 24. {
        return 0.;
    }
    // Many speakers are embedded slightly in their wall/vent surface.
    let end = source - delta.normalize() * 16.;
    let hit = world.sweep(ear, end, Vec3::ZERO);
    if hit.start_solid || hit.fraction < 0.99 {
        1.
    } else {
        0.
    }
}
pub fn room(world: &World, ear: Vec3) -> f32 {
    [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
        .into_iter()
        .map(|dir| {
            let trace = world.sweep(ear, ear + dir * 600., Vec3::ZERO);
            if trace.start_solid {
                0.
            } else {
                1. - trace.fraction
            }
        })
        .sum::<f32>()
        / 6.
}
pub struct Filter<S> {
    source: S,
    controls: Arc<Controls>,
    low: [f32; 2],
    delays: [Vec<f32>; 2],
    cursor: usize,
    channel: usize,
    tail: usize,
    rate: u32,
    last_cutoff_bits: u32,
    alpha: f32,
}
impl<S: Source<Item = f32>> Filter<S> {
    pub fn new(source: S, controls: Arc<Controls>) -> Self {
        let rate = source.sample_rate();
        let cutoff_bits = controls.cutoff.load(Ordering::Relaxed);
        let cutoff = f32::from_bits(cutoff_bits);
        let alpha = 1. - (-std::f32::consts::TAU * cutoff / rate as f32).exp();
        Self {
            source,
            controls,
            low: [0.; 2],
            delays: [
                vec![0.; (rate as f32 * 0.047) as usize],
                vec![0.; (rate as f32 * 0.071) as usize],
            ],
            cursor: 0,
            channel: 0,
            tail: 0,
            rate,
            last_cutoff_bits: cutoff_bits,
            alpha,
        }
    }
}
impl<S: Source<Item = f32>> Iterator for Filter<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let dry = match self.source.next() {
            Some(v) => v,
            None => {
                self.tail += 1;
                if self.tail > self.rate as usize {
                    return None;
                }
                0.
            }
        };
        let ch = self.channel;
        let cutoff_bits = self.controls.cutoff.load(Ordering::Relaxed);
        if cutoff_bits != self.last_cutoff_bits {
            self.last_cutoff_bits = cutoff_bits;
            let cutoff = f32::from_bits(cutoff_bits);
            self.alpha = 1. - (-std::f32::consts::TAU * cutoff / self.rate as f32).exp();
        }
        self.low[ch] += self.alpha * (dry - self.low[ch]);
        let wet = f32::from_bits(self.controls.wet.load(Ordering::Relaxed));
        let buffer = &mut self.delays[ch];
        let index = self.cursor % buffer.len();
        let reflection = buffer[index];
        buffer[index] = self.low[ch] + reflection * 0.32;
        let sample = self.low[ch] + reflection * wet;
        self.channel ^= 1;
        if self.channel == 0 {
            self.cursor += 1;
        }
        Some(sample)
    }
}
impl<S: Source<Item = f32>> Source for Filter<S> {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source
            .total_duration()
            .map(|d| d + Duration::from_millis(500))
    }
}

/// Leave quiet signals unchanged; round simultaneous loud transients below full scale.
pub fn limit(sample: f32) -> f32 {
    if !sample.is_finite() {
        return 0.;
    }
    let a = sample.abs();
    if a <= 0.8 {
        sample
    } else {
        sample.signum() * (0.8 + 0.18 * ((a - 0.8) / 0.18).tanh())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wall_obstructs_and_open_space_does_not() {
        let world = World::fixture(&[(vec3(-2., -100., -100.), vec3(2., 100., 100.))]);
        assert_eq!(
            obstruction(&world, vec3(-50., 0., 0.), vec3(50., 0., 0.)),
            1.
        );
        assert_eq!(
            obstruction(&world, vec3(-50., 0., 0.), vec3(-50., 80., 0.)),
            0.
        );
    }
    #[test]
    fn filtering_reduces_high_frequencies_and_reflections_have_a_tail() {
        let render = |blocked, room| {
            let controls = Controls::new();
            controls.set(blocked, false, room);
            let samples: Vec<f32> = (0..2205)
                .flat_map(|i| [if i % 2 == 0 { 0.5 } else { -0.5 }; 2])
                .collect();
            Filter::new(
                rodio::buffer::SamplesBuffer::new(2, 22050, samples),
                controls,
            )
            .collect::<Vec<_>>()
        };
        let clear = render(0., 0.);
        let blocked = render(1., 0.);
        let room = render(0., 1.);
        let energy = |s: &[f32]| s.iter().map(|x| x * x).sum::<f32>();
        assert!(energy(&blocked[..4410]) < energy(&clear[..4410]) * 0.1);
        assert!(energy(&room[4500..]) > 0.01);
        assert!(energy(&clear[4500..]) < 0.00001);
        assert_eq!(limit(0.25), 0.25);
        assert!(limit(10.) < 0.99 && limit(-10.) > -0.99);
    }
}

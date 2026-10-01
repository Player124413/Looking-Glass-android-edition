//! Pool's W9 adapter, saved scene consequences and pusher combat handoff.
use super::*;
use crate::{
    combat::{Feedback, Hit, Target},
    falling_rock,
    level::Combat,
    loot::{Grade, Source},
};
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub version: u8,
    pub rocks: Vec<falling_rock::State>,
    pub ants: [ants::Ant; 2],
    pub released: bool,
    pub clocks: [f32; 4],
    pub damage: f32,
    pub sounds: Vec<String>,
    pub fade_until: f32,
    pub quake: f32,
    pub quake_until: f32,
}
impl Pool {
    pub(super) fn restore_boulders(&mut self) -> Result<()> {
        if let Some(b) = &self.state.boulders {
            ensure!(
                b.clocks
                    .iter()
                    .all(|t| t.is_finite() && *t >= 0. && *t <= self.state.age),
                "Invalid rock clocks"
            );
            ensure!(
                b.version == 1
                    && b.rocks.len() == 4
                    && b.damage.is_finite()
                    && b.damage >= 0.
                    && b.sounds.len() <= 32
                    && b.sounds
                        .iter()
                        .all(|s| s == "sound/ambience/special/quake_step1.wav")
                    && b.quake.is_finite()
                    && (0. ..=1.).contains(&b.quake)
                    && b.quake_until.is_finite()
                    && b.quake_until >= 0.,
                "Invalid Pool boulder state"
            );
            for (spec, s) in self.cinema.rocks.iter().zip(&b.rocks) {
                spec.validate(s)?;
            }
            for a in &b.ants {
                a.validate()?;
                ensure!(a.enabled == b.released, "Inconsistent ant release");
            }
            ensure!(
                !b.released || self.state.cinema.rocks[2].is_some(),
                "Ant release without Boulder3"
            );
            return Ok(());
        }
        let mut rocks = self
            .cinema
            .rocks
            .iter()
            .enumerate()
            .map(|(i, s)| falling_rock::State::new(s, i == 0 || i == 3, i != 2))
            .collect::<Vec<_>>();
        // A timestamp is evidence of consumption. Reconstruct elapsed motion quietly;
        // never clear event keys or replay a previously consumed trigger.
        for (i, s) in rocks.iter_mut().enumerate() {
            if let Some(start) = self.state.cinema.rocks[i] {
                self.cinema.rocks[i].seek(s, self.state.age - start);
            } else if (i < 2 && self.state.cinema.done[i + 1])
                || (i == 3 && self.state.cinema.done[0])
            {
                self.cinema.rocks[i].seek(s, 120.);
                self.state.cinema.rocks[i] = Some(self.state.age);
            }
        }
        if self.state.cinema.done[0] {
            rocks[3].solid = false;
        }
        let released = self.state.cinema.rocks[2].is_some_and(|t| self.state.age - t >= 2.);
        let mut ants = [
            ants::Ant::pusher(self.points["ant_pos1"], 90_f32.to_radians(), 634),
            ants::Ant::pusher(self.points["ant_pusher2"], 180_f32.to_radians(), 727),
        ];
        for a in &mut ants {
            a.enabled = released;
        }
        self.state.boulders = Some(State {
            version: 1,
            rocks,
            ants,
            released,
            clocks: [self.state.age; 4],
            damage: 0.,
            sounds: Vec::new(),
            fade_until: 0.,
            quake: 0.,
            quake_until: 0.,
        });
        Ok(())
    }
    pub(super) fn boulder_alice(&self) -> Transform {
        let position = self.points["alice_boulder_2"];
        let delta = self.points["t317"] - position;
        let feet = self
            .cinema
            .camera_world
            .actor_footing(position, PLAYER_CENTER, PLAYER_HALF, 64.)
            .unwrap_or(position);
        Transform {
            translation: feet,
            rotation: Quat::from_rotation_z(delta.y.atan2(delta.x)),
        }
    }
    pub(super) fn finish_boulder(&mut self, index: usize) {
        let duration = self.cinema.push_time() + if index == 0 { 4. } else { 3. };
        let age = (duration - 1.).max(0.);
        let s = &mut self.state.boulders.as_mut().unwrap().rocks[index];
        // Watch and skip share the same elapsed physical state; a long path is
        // still falling after control returns, just as with the native flags 6.
        if s.elapsed < age {
            self.cinema.rocks[index].seek(s, age - s.elapsed);
        }
        s.solid = true;
        self.state.boulders.as_mut().unwrap().clocks[index] = self.state.age;
        self.state.boulders.as_mut().unwrap().fade_until = self.state.age + 0.5;
        self.state.cinema.rocks[index].get_or_insert((self.state.age - age).max(0.));
    }
    pub(super) fn finish_arrival_rock(&mut self) {
        let s = &mut self.state.boulders.as_mut().unwrap().rocks[3];
        // Arrival commits the stopped final rock and removes its collision.
        self.cinema.rocks[3].seek(s, 120.);
        s.solid = false;
        self.state.boulders.as_mut().unwrap().clocks[3] = self.state.age;
    }
    pub(super) fn rock_colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.state.boulders.iter().flat_map(|b| {
            self.cinema
                .rocks
                .iter()
                .zip(&b.rocks)
                .filter_map(|(d, s)| d.collider(s))
        })
    }
    pub(super) fn advance_boulders(&mut self, dt: f32, player: &mut Player) {
        if dt <= 0. {
            return;
        }
        let c = &self.state.cinema;
        let b = self.state.boulders.as_mut().unwrap();
        let release = c.rocks[2].is_some_and(|start| self.state.age - start + 0.00001 >= 2.);
        if release && !b.released {
            b.released = true;
            for a in &mut b.ants {
                a.enabled = true;
                if let Some(at) = self.cinema.arrival.rabbit_world.actor_footing(
                    a.feet,
                    Vec3::Z * 36.,
                    Vec3::splat(36.),
                    64.,
                ) {
                    a.feet = at;
                }
            }
        }
        let mut bodies = vec![Target {
            id: crate::dice::ALICE,
            center: player.feet + PLAYER_CENTER,
            half: PLAYER_HALF,
        }];
        bodies.extend(
            b.ants
                .iter()
                .enumerate()
                .filter(|(_, a)| a.enabled && a.health > 0.)
                .map(|(i, a)| a.target(ants::IDS[i])),
        );
        for (i, (spec, s)) in self.cinema.rocks.iter().zip(&mut b.rocks).enumerate() {
            if c.beat == Some(cinema::Beat::Boulder2) && i == 1 {
                s.visible = true;
            }
            if !s.started
                && ((i == 0 && c.beat == Some(cinema::Beat::Boulder1))
                    || (i == 1 && c.beat == Some(cinema::Beat::Boulder2)))
            {
                s.solid = false;
            }
            let Some(start) = c.rocks[i] else { continue };
            if !s.started {
                spec.activate(s);
                b.clocks[i] = start.max(self.state.age - dt);
            }
            let elapsed = (self.state.age - b.clocks[i]).max(0.).min(dt);
            b.clocks[i] = self.state.age;
            let out = spec.advance(s, elapsed, &bodies);
            for (id, damage, impulse) in out.hits {
                if id == crate::dice::ALICE {
                    b.damage += damage;
                    player.velocity += impulse;
                } else if let Some(index) = ants::IDS.iter().position(|&n| n == id) {
                    b.ants[index].hit(Hit {
                        id,
                        damage,
                        kind: crate::combat::DamageKind::Other,
                        knockback: impulse / 2.5,
                    });
                }
            }
            for position in out.arrivals {
                if let Some(sound) = &spec.sound {
                    if b.sounds.len() < 32 {
                        b.sounds.insert(0, sound.clone());
                    }
                }
                if spec.flags & 4 != 0 && spec.distance > 0. {
                    let amount = (1. - position.distance(player.feet) / spec.distance).max(0.);
                    b.quake = amount * spec.magnitude;
                    b.quake_until = self.state.age + amount * 0.5;
                }
            }
            // All four reviewed Pool paths have no waypoint threads. Reject any
            // newly authored callback at load instead of silently dropping it.
            debug_assert!(out.threads.is_empty());
        }
    }
    pub fn quake_offset(&self) -> Vec3 {
        let b = self.state.boulders.as_ref().unwrap();
        let strength = b.quake * (b.quake_until - self.state.age).max(0.) * 2.;
        vec3(
            (self.state.age * 97.).sin(),
            (self.state.age * 113.).sin(),
            (self.state.age * 83.).cos(),
        ) * strength
    }
    pub fn targets(&self) -> Vec<Target> {
        self.state
            .boulders
            .as_ref()
            .unwrap()
            .ants
            .iter()
            .enumerate()
            .filter(|(_, a)| a.enabled && a.health > 0.)
            .map(|(i, a)| a.target(ants::IDS[i]))
            .collect()
    }
    pub fn hit(&mut self, hit: Hit) -> Option<Option<&'static str>> {
        let i = ants::IDS.iter().position(|&id| id == hit.id)?;
        Some(self.state.boulders.as_mut().unwrap().ants[i].hit(hit))
    }
    pub fn provoke(&mut self, id: usize) {
        if let Some(i) = ants::IDS.iter().position(|&n| n == id) {
            self.state.boulders.as_mut().unwrap().ants[i]
                .opponents
                .demon = true;
        }
    }
    pub fn combat(&mut self, ctx: &mut Combat<'_>) -> Feedback {
        let mut out = Feedback::default();
        for a in &mut self.state.boulders.as_mut().unwrap().ants {
            a.advance(ctx, &self.cinema, &mut out);
        }
        out
    }
    pub fn loot_sources(&self) -> Vec<Source> {
        self.state
            .boulders
            .as_ref()
            .unwrap()
            .ants
            .iter()
            .enumerate()
            .filter(|(_, a)| a.enabled)
            .map(|(i, a)| Source {
                id: ants::IDS[i],
                feet: a.feet,
                grade: Grade::Medium,
                dead: a.health == 0.,
            })
            .collect()
    }
}

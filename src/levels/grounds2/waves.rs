use super::*;
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Group {
    pub started: bool,
    pub count: usize,
    pub wait: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Flight {
    pub velocity: Vec3,
    pub time: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Actor {
    pub spawned: bool,
    pub body: crate::cards::Guard,
    pub flight: Option<Flight>,
    pub electric: f32,
}
impl Actor {
    pub fn new(p: Transform, seed: usize) -> Self {
        let d = p.rotation * Vec3::X;
        Self {
            spawned: false,
            body: crate::cards::Guard::new(
                crate::cards::Kind::Spade,
                p.translation,
                d.y.atan2(d.x),
                1.,
                seed,
            ),
            flight: None,
            electric: 0.,
        }
    }
    fn launch(&mut self, velocity: Vec3, w: &World) {
        self.spawned = true;
        let feet = self.body.feet;
        self.body.place(w);
        self.body.feet = feet;
        self.body.set(crate::cards::Phase::Run);
        self.flight = Some(Flight { velocity, time: 0. });
    }
    fn hazards(&mut self, w: &World) {
        if self.body.health > 0.
            && (self.body.feet.z < -1080. || w.liquid_at(self.body.feet + Vec3::Z * 8.) & 0x08 != 0)
        {
            self.body.hit(Hit {
                id: 0,
                damage: 10000.,
                kind: crate::combat::DamageKind::Other,
                knockback: Vec3::ZERO,
            });
            self.flight = None;
        }
    }
    fn fly(&mut self, w: &World) {
        let Some(f) = &mut self.flight else {
            return;
        };
        let dt = crate::cards::STEP;
        if self.body.health > 0. {
            self.body.time += dt;
        }
        f.time = (f.time + dt).min(12.);
        f.velocity.z -= crate::movement::GRAVITY * dt;
        // Slide along solid sides while retaining the remaining fraction of the step.
        let mut remaining = dt;
        for _ in 0..4 {
            let b = self.body.target(0);
            let tr = w.sweep(b.center, b.center + f.velocity * remaining, b.half);
            if tr.start_solid {
                break;
            }
            self.body.feet += f.velocity * remaining * tr.fraction;
            if tr.fraction >= 1. {
                break;
            }
            if tr.normal.z > 0.65 && f.velocity.z <= 0. {
                self.flight = None;
                return;
            }
            remaining *= 1. - tr.fraction;
            f.velocity -= tr.normal * f.velocity.dot(tr.normal).min(0.);
        }
        if self.body.feet.z < -1080. || f.time >= 12. {
            self.body.hit(Hit {
                id: 0,
                damage: 10000.,
                kind: crate::combat::DamageKind::Other,
                knockback: Vec3::ZERO,
            });
            self.flight = None;
        }
    }
}
impl Royale {
    pub(super) fn step_waves(
        &mut self,
        c: &mut crate::level::Combat<'_>,
    ) -> crate::combat::Feedback {
        let mut out = crate::combat::Feedback::default();
        if self.scripted() || c.dt <= 0. {
            return out;
        }
        self.data.arena.copy_dynamic_from(c.world);
        self.saved.accumulator += c.dt.min(0.1) as f64;
        while self.saved.accumulator >= crate::cards::STEP as f64 {
            self.saved.accumulator -= crate::cards::STEP as f64;
            let dt = crate::cards::STEP;
            let mut offset = 0;
            for (k, total) in TOTAL.into_iter().enumerate() {
                let g = &mut self.saved.groups[k];
                if g.started && g.count < total {
                    g.wait = (g.wait - dt).max(0.);
                    if g.wait <= 0. {
                        let live = self.saved.actors[offset..offset + total]
                            .iter()
                            .filter(|a| a.spawned && a.body.health > 0.)
                            .count();
                        if live < 2 {
                            self.saved.actors[offset + g.count]
                                .launch(self.data.launches[k], &self.data.arena);
                            g.count += 1;
                            g.wait = 1.1;
                        } else {
                            g.wait = 1.;
                        }
                    }
                }
                offset += total;
            }
            for a in &mut self.saved.actors {
                if !a.spawned {
                    continue;
                }
                a.electric = (a.electric - dt).max(0.);
                a.body.notarget = c.notarget;
                a.body.opponents.summon = c.summon;
                if a.flight.is_some() {
                    a.fly(&self.data.arena);
                    a.hazards(&self.data.arena);
                    continue;
                }
                if !c.notarget {
                    a.body.attack_player(c.player.eye());
                }
                self.rig
                    .step(&mut a.body, &self.data.arena, c.player.eye(), &mut out);
                a.hazards(&self.data.arena);
            }
        }
        out
    }
}

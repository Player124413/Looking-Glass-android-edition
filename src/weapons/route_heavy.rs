//! Graphics-free adapter for input replays. Uses the live Staff/cannon simulation,
//! action release times, resource costs and collision paths.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Heavy(heavy::State);
impl Heavy {
    pub fn validate(&self) -> Result<()> {
        self.0.validate()
    }
    pub fn charging(&self) -> bool {
        self.0.charge.is_some()
    }
    pub fn update(
        &mut self,
        dt: f32,
        events: Events,
        actions: &mut Actions,
        eye: Vec3,
        aim: Vec3,
        owner: combat::Target,
        ctx: &CombatContext<'_>,
        stats: &mut crate::inventory::Stats,
    ) -> Vec<Hit> {
        let pose = Transform {
            translation: eye,
            rotation: Quat::from_rotation_arc(Vec3::X, aim.normalize_or_zero()),
        };
        let mut hits = Vec::new();
        let mut cursor = 0.;
        let mut timeline: Vec<_> = events
            .emissions
            .into_iter()
            .map(|e| (e.at, Some(e)))
            .chain(events.stop_times.into_iter().map(|at| (at, None)))
            .collect();
        timeline.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (at, emission) in timeline {
            hits.extend(
                self.0
                    .advance((at - cursor).max(0.), ctx, owner, pose, false, Some(stats))
                    .hits,
            );
            if let Some(e) = emission {
                if e.fire && matches!(e.play.action, Action::Attack { .. }) {
                    if e.weapon == 7 {
                        self.0.start(e.play.alternate(), pose);
                    }
                    if e.weapon == 8 {
                        self.0.launch(heavy::Kind::Cannon, eye, e.play.aim);
                    }
                }
            } else {
                self.0.release();
            }
            cursor = at;
        }
        if actions.selected != 7
            || actions
                .playing
                .as_ref()
                .is_none_or(|p| !matches!(p.action, Action::Attack { .. }))
        {
            self.0.charge = None;
        }
        hits.extend(
            self.0
                .advance((dt - cursor).max(0.), ctx, owner, pose, false, Some(stats))
                .hits,
        );
        if self.0.charge.is_none()
            && actions.selected == 7
            && actions.playing.as_ref().is_some_and(|p| {
                (p.fired || p.stage == crate::weapon_rules::Stage::Sustain)
                    && matches!(p.action, Action::Attack { .. })
                    && p.stage != crate::weapon_rules::Stage::End
            })
        {
            actions.end_staff();
        }
        hits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn funded_staff_replay_resumes_and_world_blocks_its_beam_and_shot() {
        let run = |resume: bool, wall: bool| {
            let world = crate::collision::World::fixture(&if wall {
                vec![(vec3(150., -100., -100.), vec3(170., 100., 100.))]
            } else {
                vec![]
            });
            let targets = [combat::Target {
                id: 1,
                center: vec3(600., 0., 50.),
                half: Vec3::splat(20.),
            }];
            let ctx = CombatContext {
                world: &world,
                targets: &targets,
            };
            let mut actions = test_actions_with_bones(2);
            let base = actions.pose.clone();
            let mut heavy = Heavy::default();
            let mut stats = crate::inventory::Stats::weapon_preview();
            let eye = vec3(0., 0., 50.);
            let owner = combat::Target {
                id: crate::dice::ALICE,
                center: Vec3::ZERO,
                half: Vec3::splat(15.),
            };
            let mut damage = 0.;
            for tick in 0..840 {
                let e = actions.update_funded(
                    1. / 120.,
                    WeaponInput {
                        dice: 1,
                        selected: 7,
                        click: (tick < 600).then_some(false),
                        aim: Vec3::X,
                        first_person: true,
                    },
                    &base,
                    &[true; 2],
                    true,
                    Some(&mut stats),
                );
                damage += heavy
                    .update(
                        1. / 120.,
                        e,
                        &mut actions,
                        eye,
                        Vec3::X,
                        owner,
                        &ctx,
                        &mut stats,
                    )
                    .iter()
                    .filter(|h| h.id == 1)
                    .map(|h| h.damage)
                    .sum::<f32>();
                actions.finish_frame();
                if resume && tick == 420 {
                    heavy = serde_json::from_value(serde_json::to_value(&heavy).unwrap()).unwrap();
                    heavy.validate().unwrap();
                    let state = actions.snapshot();
                    actions = test_actions_with_bones(2);
                    actions.restore(&state).unwrap();
                }
            }
            (damage, stats.will())
        };
        let live = run(false, false);
        assert!(live.0 > 300. && live.1 < 85., "{live:?}");
        assert_eq!(live, run(true, false));
        let blocked = run(false, true);
        assert_eq!(blocked.0, 0.);
        assert_eq!(blocked.1, live.1);
    }
}

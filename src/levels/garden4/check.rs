use super::*;
use crate::movement::{Controls, FIXED_DT};
use std::collections::BTreeSet;
fn setup(a: &mut Assets, map: &Bsp) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(a, map, "garden4", Some("garden4_start1"))?;
    let mut w = World::from_bsp(map)?;
    i.sync(&mut w);
    let p = Player::spawn(&w, vec3(-1632., 5120., -4768.))
        .context("Caterpillar approach has no support")?;
    Ok((i, w, p, Story::load(a, "garden4")))
}
fn enter(i: &mut Interactions, w: &World, p: &mut Player) -> Result<()> {
    for _ in 0..240 {
        let from = p.feet;
        p.tick(
            w,
            Controls {
                wish: Vec2::Y,
                ..Default::default()
            },
        );
        let e = i.triggers(FIXED_DT, from, p.feet);
        ensure!(e.transition.is_none(), "Exit on encounter approach");
        if i.scripted() {
            return Ok(());
        }
    }
    anyhow::bail!(
        "Ordinary player movement missed Caterpillar trigger at {:?}",
        p.feet
    )
}
fn tick(
    i: &mut Interactions,
    map: &Bsp,
    w: &mut World,
    p: &mut Player,
    s: &mut Story,
    dt: f32,
) -> Result<()> {
    i.advance_school(dt, map, w, p)?;
    if i.prepare_story(s) {
        s.tick(dt, false);
    }
    i.sync_cinematic_story(s);
    for n in s.take_completed() {
        i.completed_dialogue(&n);
    }
    i.sync(w);
    Ok(())
}
fn blocked(i: &mut Interactions) -> Result<()> {
    let old = vec3(-1746., 5498., -4544.) - crate::collision::PLAYER_CENTER;
    let g = owner(i)?;
    let pose = g.portal_pose();
    let moved = pose.translation
        + pose.rotation * (vec3(-1746., 5498., -4544.) - g.data.portal_origin)
        - crate::collision::PLAYER_CENTER;
    for p in [old, moved] {
        ensure!(
            i.triggers(FIXED_DT, p, p).transition.is_none(),
            "Premature portal exit"
        );
    }
    Ok(())
}
pub(super) fn check(a: &mut Assets) -> Result<()> {
    course_check::check(a)?;
    let refs: Vec<_> = [("alice", ALICE), ("c_caterpillar", CATER)]
        .into_iter()
        .flat_map(|(model, names)| {
            names
                .iter()
                .map(move |&requested| crate::story::registry::AnimationRef {
                    model,
                    requested,
                    fallback: None,
                })
        })
        .collect();
    crate::story::registry::validate_animations(a, &refs)?;
    let map = Bsp::parse(&a.read("maps/garden4.bsp")?)?;
    let hints = crate::cheshire::Hints::load(a, &map, "garden4")?;
    let mut endpoint = None;
    for hz in [30, 60, 144] {
        for skip_at in [None, Some(0.2), Some(12.), Some(90.), Some(-1.)] {
            let (mut i, mut w, mut p, mut story) = setup(a, &map)?;
            ensure!(i.levels.len() == 1, "Duplicate Garden4 owner");
            blocked(&mut i)?;
            // An unrelated or stale completion callback cannot open the portal.
            i.completed_dialogue(TALK);
            i.completed_dialogue(END);
            blocked(&mut i)?;
            enter(&mut i, &w, &mut p)?;
            let mut skipped = false;
            let mut lines = BTreeSet::new();
            let mut resume_checked = false;
            for _ in 0..hz * 300 {
                if let Some((n, _)) = story.progress(TALK) {
                    lines.insert(n);
                }
                let time = owner(&mut i)?
                    .saved
                    .scene
                    .as_ref()
                    .map_or(300., |s| s.clock.time);
                let reveal_skip = owner(&mut i)?
                    .saved
                    .scene
                    .as_ref()
                    .is_some_and(|s| s.phase == Phase::Reveal && s.elapsed() >= 7.);
                if !skipped
                    && skip_at.is_some_and(|t| if t < 0. { reveal_skip } else { time >= t })
                    && i.scripted()
                {
                    ensure!(
                        i.skip_cinematic(&map, &mut w, &mut p, &mut story)?,
                        "Scene skip refused"
                    );
                    skipped = true;
                }
                if skipped && i.scripted() {
                    let snapshot = owner(&mut i)?.snapshot();
                    let _ = i.skip_cinematic(&map, &mut w, &mut p, &mut story)?;
                    ensure!(
                        snapshot == owner(&mut i)?.snapshot(),
                        "Held skip restarted closing scene"
                    );
                }
                tick(&mut i, &map, &mut w, &mut p, &mut story, 1. / hz as f32)?;
                if !owner(&mut i)?.ready() {
                    blocked(&mut i)?;
                }
                if !resume_checked && story.progress(TALK).is_some_and(|(n, t)| n == 3 && t > 0.3) {
                    let saved_i = i.snapshot();
                    let saved_story = story.snapshot();
                    let saved_p = p.clone();
                    let (mut ri, mut rw, _, mut rs) = setup(a, &map)?;
                    ri.restore(&saved_i, &map)?;
                    rs.restore(&saved_story, &hints)?;
                    let mut rp = saved_p.clone();
                    let before = owner(&mut i)?.snapshot();
                    for _ in 0..20 {
                        tick(&mut i, &map, &mut w, &mut p, &mut story, 0.)?;
                    }
                    ensure!(
                        owner(&mut i)?.snapshot() == before,
                        "Pause advanced conversation"
                    );
                    for _ in 0..120 {
                        tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?;
                        tick(&mut ri, &map, &mut rw, &mut rp, &mut rs, FIXED_DT)?;
                    }
                    ensure!(
                        owner(&mut i)?.snapshot() == owner(&mut ri)?.snapshot()
                            && serde_json::to_value(story.snapshot())?
                                == serde_json::to_value(rs.snapshot())?,
                        "Restored dialogue future differs"
                    );
                    // Simulate interrupted speech with its durable seen history retained.
                    let cursor = story.progress(TALK).unwrap();
                    let mut broken = serde_json::to_value(story.snapshot())?;
                    broken["current"] = serde_json::Value::Null;
                    broken["queue"] = serde_json::json!([]);
                    story.restore(&serde_json::from_value(broken)?, &hints)?;
                    ensure!(
                        story.trigger(END),
                        "Interruption fixture did not queue speech"
                    );
                    for _ in 0..300 {
                        owner(&mut i)?.advance(0.1, &map, &mut w, &mut p, &[])?;
                        i.prepare_story(&mut story);
                        ensure!(
                            story.sequence_pending(END) && story.progress(TALK).is_none(),
                            "Resuming Caterpillar dialogue displaced queued speech"
                        );
                    }
                    let g = owner(&mut i)?;
                    let s = g.saved.scene.as_ref().unwrap();
                    ensure!(
                        (s.clock.line, s.clock.line_time) == cursor,
                        "Waiting for speech consumed the interrupted line cursor"
                    );
                    story.finish_sequence(END);
                    for n in story.take_completed() {
                        i.completed_dialogue(&n);
                    }
                    i.prepare_story(&mut story);
                    ensure!(
                        story.progress(TALK) == Some(cursor),
                        "Interrupted dialogue restarted or lost its saved cursor"
                    );
                    resume_checked = true;
                }
                if owner(&mut i)?.ready() {
                    break;
                }
            }
            let g = owner(&mut i)?;
            ensure!(
                g.ready() && !g.drugview() && !g.scripted(),
                "Conversation failed to commit"
            );
            if skip_at.is_none() {
                ensure!(
                    lines.len() == 10,
                    "Authored dialogue count differs: {lines:?}"
                );
            }
            ensure!(
                w.body_clear(p.feet)
                    && p.grounded
                    && (p.script_facing - 90_f32.to_radians()).abs() < 0.001,
                "Unsafe final player pose: feet {:?}, grounded {}, yaw {}, clear {}",
                p.feet,
                p.grounded,
                p.script_facing,
                w.body_clear(p.feet)
            );
            let portal = g.portal_pose();
            let final_state = (portal.translation, portal.rotation, p.feet, p.script_facing);
            if let Some(expected) = endpoint {
                ensure!(expected == final_state, "Watch/skip outcomes differ");
            } else {
                endpoint = Some(final_state);
                println!("Portal end {:?}; Alice {:?}", final_state.0, p.feet);
            }
            let before = g.snapshot();
            g.event(START);
            ensure!(
                g.snapshot() == before,
                "Encounter replayed after completion"
            );
            // The editor location must no longer be an exit after the portal moved.
            let old = vec3(-1746., 5498., -4544.) - crate::collision::PLAYER_CENTER;
            ensure!(
                i.triggers(FIXED_DT, old, old).transition.is_none(),
                "Stale editor exit remains active"
            );
            let mut departure = None;
            for _ in 0..240 {
                let from = p.feet;
                p.tick(
                    &w,
                    Controls {
                        wish: Vec2::X,
                        ..Default::default()
                    },
                );
                let e = i.triggers(FIXED_DT, from, p.feet);
                if e.transition.is_some() {
                    departure = e.transition;
                    break;
                }
            }
            ensure!(
                departure.as_ref().is_some_and(|e| EXIT.matches(e)),
                "Walking into bound portal failed at {:?}",
                p.feet
            );
            ensure!(
                owner(&mut i)?.saved.exit.committed,
                "Exit latch was not recorded"
            );
            ensure!(
                i.triggers(FIXED_DT, p.feet, p.feet).transition.is_none(),
                "Portal emitted twice"
            );
            let g = owner(&mut i)?;
            g.transition_failed(&EXIT.destination());
            ensure!(
                g.update(&mut w, &p, Vec3::X, false).transition.is_none(),
                "Failed exit retried without delay"
            );
            g.advance(0.1, &map, &mut w, &mut p, &[])?;
            for _ in 0..10 {
                g.advance(0.1, &map, &mut w, &mut p, &[])?;
            }
            ensure!(
                g.update(&mut w, &p, Vec3::X, false).transition.is_some(),
                "Failed exit cannot retry"
            );
            let snap = g.snapshot();
            g.restore(&snap, &map)?;
            ensure!(
                g.update(&mut w, &p, Vec3::X, false).transition.is_some(),
                "Committed exit lost on reload"
            );
        }
    }
    println!("PASS 15 watched/skipped cases, ten beats, interruption, pause, exact restore, real encounter/portal movement and exit retries");
    for (consumed, old_exit) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut old = Interactions::load(&map)?;
        if old_exit {
            let at = vec3(-1746., 5498., -4544.) - crate::collision::PLAYER_CENTER;
            ensure!(
                old.triggers(FIXED_DT, at, at).transition.is_some(),
                "Legacy fixture did not exercise the previously open exit"
            );
        }
        if consumed {
            let p = vec3(-1508., 5240., -4636.) - crate::collision::PLAYER_CENTER;
            old.triggers(FIXED_DT, p, p);
        }
        let (mut i, mut w, mut p, mut story) = setup(a, &map)?;
        i.restore(&old.snapshot(), &map)?;
        for slot in &mut i.levels {
            slot.ctl.restore_position(&p, &map)?;
        }
        blocked(&mut i)?;
        if !i.scripted() {
            enter(&mut i, &w, &mut p)?;
        }
        ensure!(i.scripted(), "Previously consumed encounter cannot restart");
        tick(&mut i, &map, &mut w, &mut p, &mut story, FIXED_DT)?;
        let g = owner(&mut i)?;
        let good = g.snapshot();
        let mut bad = good.clone();
        bad["complete"] = serde_json::json!(true);
        ensure!(
            g.restore(&bad, &map).is_err() && g.snapshot() == good,
            "Rejected save mutated live scene"
        );
    }
    println!("PASS four generic-save migrations keep portal closed, including previously consumed encounter and exit; malformed states rejected atomically");
    Ok(())
}
pub fn stage(
    case: &str,
    a: &mut Assets,
    map: &Bsp,
    i: &mut Interactions,
    w: &mut World,
    p: &mut Player,
    _: &mut Stats,
) -> Result<Story> {
    if matches!(
        case,
        "garden4-ice-fall" | "garden4-ice-collapse" | "garden4-marble" | "garden4-wall"
    ) {
        return course_check::stage(case, a, map, i, w, p);
    }
    *p = Player::spawn(w, vec3(-1632., 5120., -4768.)).context("Scene fixture approach blocked")?;
    enter(i, w, p)?;
    let mut s = Story::load(a, "garden4");
    let mut skipped = false;
    for _ in 0..120 * 300 {
        if case == "garden4-skip-delay" && !skipped {
            i.skip_cinematic(map, w, p, &mut s)?;
            skipped = true;
        }
        tick(i, map, w, p, &mut s, FIXED_DT)?;
        let g = owner(i)?;
        let found = match case {
            "garden4-talk" => s.progress(TALK).is_some_and(|(n, t)| n == 3 && t > 0.5),
            "garden4-smoke" => g
                .saved
                .scene
                .as_ref()
                .is_some_and(|s| s.phase == Phase::Reveal && s.elapsed() > 4.3),
            "garden4-portal" => g
                .saved
                .scene
                .as_ref()
                .is_some_and(|s| s.phase == Phase::Reveal && s.elapsed() > 9.),
            "garden4-skip-delay" => g.saved.complete && g.saved.enable_delay > 3.,
            "garden4-ready" => g.ready(),
            _ => anyhow::bail!("Unknown Caterpillar fixture"),
        };
        if found {
            return Ok(s);
        }
    }
    anyhow::bail!("Caterpillar fixture timed out: {case}")
}
pub(super) fn render(a: &mut Assets) -> super::super::BoxFuture<'_> {
    Box::pin(async move {
        let mut scene = crate::render::Scene::load(a, "garden4")?;
        let mut art = art::Art::load(a)?;
        for case in REGISTRATION.save_cases {
            let (mut i, mut w, mut p, _) = setup(a, &scene.map)?;
            let mut stats = Stats::for_level("garden4", None);
            let s = stage(case.name, a, &scene.map, &mut i, &mut w, &mut p, &mut stats)?;
            art.story_pose(&s);
            let g = owner(&mut i)?;
            let c = g.camera(&w).unwrap_or_else(|| {
                if case.name == "garden4-ice-fall" {
                    return crate::cinematic::Camera::look(
                        vec3(-448., -512., -4310.),
                        vec3(-600., -1190., -4700.),
                    );
                }
                if case.name == "garden4-ice-collapse" {
                    return crate::cinematic::Camera::look(
                        vec3(-1704., -288., -4470.),
                        vec3(-1540., -1020., -4640.),
                    );
                }
                if case.name == "garden4-wall" {
                    return crate::cinematic::Camera::look(
                        vec3(-2624., 1200., -5200.),
                        vec3(-2624., 1536., -5200.),
                    );
                }
                crate::cinematic::Camera::look(
                    p.feet + vec3(100., -240., 140.),
                    g.portal_pose().translation,
                )
            });
            for frame in 0..3 {
                clear_background(BLACK);
                set_camera(&Camera3D {
                    position: c.eye,
                    target: c.target,
                    up: c.up,
                    fovy: 75_f32.to_radians(),
                    z_near: 2.,
                    z_far: 20000.,
                    ..Default::default()
                });
                scene.draw(c.eye, 0., false, false, &g.transforms());
                art.draw(g, &scene.atmosphere, c.eye, false);
                crate::render::depth_read_only(|| {
                    scene.draw(c.eye, 0., false, true, &g.transforms())
                });
                art.effects(g, c.eye, &scene.atmosphere);
                set_default_camera();
                if frame == 2 {
                    crate::viewer::save_capture(std::path::Path::new(&format!(
                        "private/garden4-work/{}.png",
                        case.name
                    )))?;
                }
                next_frame().await;
            }
        }
        Ok(())
    })
}

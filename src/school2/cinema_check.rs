//! Component fixtures use actual scene/quest callbacks, separately from traversal proof.
use super::cinema::{Beat, MUSHROOM, SPICE};
use super::*;
use crate::cinematic::Camera;
use crate::{interaction::Interactions, story::Story};
use anyhow::ensure;

pub(crate) fn setup(
    assets: &mut Assets,
    map: &Bsp,
    beat: Beat,
) -> Result<(Interactions, World, Player, Story)> {
    if matches!(beat, Beat::Dice | Beat::Bleachers) { return extra_setup(assets, map, beat); }
    if matches!(beat, Beat::Growth | Beat::Final) {
        return super::potion_check::setup(assets, map, beat);
    }
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "skool2", None)?;
    let mut world = World::from_bsp(map)?;
    i.sync(&mut world);
    let mut p = Player::spawn(
        &world,
        if beat == Beat::Spice {
            vec3(168., -864., 280.)
        } else {
            vec3(2020., -3044., 448.)
        },
    )
    .context("Second-school fixture obstructed")?;
    p.script_facing = if beat == Beat::Spice {
        std::f32::consts::PI
    } else {
        -std::f32::consts::FRAC_PI_4
    };
    let mut story = Story::load(assets, "skool2");
    let s = i.school2.as_mut().unwrap();
    if beat == Beat::Mushroom {
        let event = s.event(MUSHROOM).unwrap();
        for id in event.story {
            story.trigger(&id);
        }
    } else {
        s.quest.items.mushroom = true;
        for b in &mut s.boojums[..3] {
            b.active = true;
            b.hit(1000.);
            b.active = false;
        }
        s.quest.enter(if beat == Beat::Warp {
            Stage::Battle
        } else {
            Stage::Rescue
        });
        if beat == Beat::Spice {
            for g in &mut s.guards {
                g.hurt(1000.);
            }
        }
        let (e, _) = s.update(1. / 60., &world, p.feet, false);
        for id in e.story {
            story.trigger(&id);
        }
    }
    ensure!(i.scripted(), "School-two fixture did not start");
    ensure!(
        i.school2.as_ref().unwrap().scene_id().is_some(),
        "School-two skip identity absent"
    );
    Ok((i, world, p, story))
}
pub(crate) fn tick(
    i: &mut Interactions,
    map: &Bsp,
    world: &mut World,
    p: &mut Player,
    story: &mut Story,
    dt: f32,
    advance: bool,
) -> Result<()> {
    i.advance_school(dt, map, world, p)?;
    i.update(dt, map, world, p, Vec3::X, false)?;
    if i.prepare_story(story) {
        story.tick(dt, advance);
    }
    i.sync_cinematic_story(story);
    for id in story.take_completed() {
        i.completed_dialogue(&id);
    }
    let (e, _) = i
        .school2
        .as_mut()
        .unwrap()
        .update(dt, world, p.feet, story.busy());
    for id in e.story {
        story.trigger(&id);
    }
    Ok(())
}
pub fn check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    extra_check(assets, map)?;
    for beat in [Beat::Mushroom, Beat::Warp, Beat::Spice] {
        for (advance, skip) in [
            (false, None),
            (true, None),
            (false, Some(0.)),
            (false, Some(3.)),
            (false, Some(15.)),
        ] {
            let (mut i, mut world, mut p, mut story) = setup(assets, map, beat)?;
            let home = p.feet;
            let hints = crate::cheshire::Hints::load(assets, map, "skool2")?;
            let mut skipped = false;
            for frame in 0..9000 {
                if !skipped && skip.is_some_and(|t| frame as f32 / 60. >= t) {
                    i.skip_cinematic(map, &mut world, &mut p, &mut story)?;
                    skipped = true;
                }
                if !i.scripted() {
                    break;
                }
                if frame % 53 == 0 {
                    let old = serde_json::to_value(i.snapshot())?;
                    let camera = i.school2.as_ref().unwrap().scene_camera().unwrap();
                    i.advance_school(0., map, &mut world, &mut p)?;
                    ensure!(
                        old == serde_json::to_value(i.snapshot())?,
                        "School-two paused scene advanced"
                    );
                    i.restore(&serde_json::from_value(old)?, map)?;
                    i.sync(&mut world);
                    story.restore(
                        &serde_json::from_value(serde_json::to_value(story.snapshot())?)?,
                        &hints,
                    )?;
                    let next = i.school2.as_ref().unwrap().scene_camera().unwrap();
                    ensure!(
                        camera.eye.distance(next.eye) < 0.001
                            && camera.up.distance(next.up) < 0.001,
                        "School-two restored shot changed"
                    );
                }
                let s = i.school2.as_ref().unwrap();
                if beat == Beat::Spice && frame > 180 {
                    ensure!(
                        s.guards
                            .iter()
                            .all(|g| g.time >= s.guard_timing.death - 0.01),
                        "Rescued guards froze during dialogue"
                    );
                }
                ensure!(s.targets().is_empty(), "Scene exposed combat targets");
                ensure!(
                    !s.quest.items.spice
                        && !s.quest.items.jumbogrow
                        && !s.quest.items.potion
                        && !s.quest.items.star,
                    "Scene granted an early reward"
                );
                ensure!(
                    s.boojums[..3].iter().all(|b| !b.active),
                    "Mushroom scene started combat early"
                );
                tick(
                    &mut i,
                    map,
                    &mut world,
                    &mut p,
                    &mut story,
                    1. / 60.,
                    advance,
                )?;
            }
            let s = i.school2.as_ref().unwrap();
            s.cinema.as_ref().unwrap().validate(s.quest.stage)?;
            ensure!(
                !i.scripted() && world.body_clear(p.feet) && p.feet.distance(home) < 1.,
                "School-two scene stuck or unsafe {beat:?}"
            );
            let expected = match beat {
                Beat::Mushroom => Stage::Battle,
                Beat::Warp => Stage::Laboratory,
                Beat::Spice => Stage::Jumbogrow,
                _ => unreachable!(),
            };
            ensure!(s.quest.stage == expected, "Wrong scene commitment");
            if beat == Beat::Mushroom {
                ensure!(
                    s.quest.items.mushroom
                        && s.boojums[..3].iter().all(|b| b.active && b.health > 0.),
                    "Mushroom did not release three live Boojums"
                );
            }
            if beat == Beat::Spice {
                ensure!(
                    s.quest.items.spice && s.guards.iter().all(|g| g.health <= 0.),
                    "Spice reward lost rescue gate"
                );
            }
            if beat == Beat::Warp {
                ensure!(
                    s.guards.iter().all(|g| g.health > 0.)
                        && s.boojums[6..8].iter().all(|b| b.active),
                    "Warp bypassed rescue or lost ambush"
                );
            }
            let saved = serde_json::to_value(i.snapshot())?;
            i.completed_dialogue(MUSHROOM);
            i.completed_dialogue(SPICE);
            ensure!(
                !i.skip_cinematic(map, &mut world, &mut p, &mut story)?,
                "Completed scene replayed"
            );
            ensure!(
                saved == serde_json::to_value(i.snapshot())?,
                "Repeated scene completion changed state"
            );
            println!("PASS school-two {beat:?} advance={advance} skip={skip:?}: gates, repeated save/restore, safe handoff");
        }
    }
    let mut s = School2::load(assets, map)?;
    let mut world = World::from_bsp(map)?;
    world.set_dynamic(s.colliders().collect());
    let feet = vec3(-32., -980., 272.);
    ensure!(
        !s.quest.begin("Kill_The_Gnome"),
        "Rescue before Mushroom/battle"
    );
    s.quest.enter(Stage::Battle);
    for b in &mut s.boojums[..2] {
        b.active = true;
        b.hit(1000.);
        b.active = false;
    }
    s.update(0.1, &world, feet, false);
    ensure!(
        s.quest.stage == Stage::Battle && !s.cinematic(),
        "Two Boojums triggered Gnome warp"
    );
    s.quest.enter(Stage::Rescue);
    s.guards[0].hurt(1000.);
    s.update(0.1, &world, feet, false);
    ensure!(
        s.quest.stage == Stage::Rescue && !s.quest.items.spice && !s.cinematic(),
        "One guard granted Spice"
    );
    // Existing version-12 quest saves without staging continue their current gate.
    for (stage, id, next) in [
        (Stage::MushroomDialogue, MUSHROOM, Stage::Battle),
        (Stage::SpiceDialogue, SPICE, Stage::Jumbogrow),
    ] {
        s.quest.enter(stage);
        let mut saved = serde_json::to_value(s.snapshot())?;
        saved.as_object_mut().unwrap().remove("cinema");
        s.restore(&serde_json::from_value(saved)?, map)?;
        ensure!(
            s.completed_dialogue(id) && s.quest.stage == next && !s.cinematic(),
            "Legacy dialogue save replayed scene"
        );
    }
    println!("PASS distinct combat gates and legacy second-school saves");
    Ok(())
}

fn extra_setup(assets: &mut Assets, map: &Bsp, beat: Beat) -> Result<(Interactions, World, Player, Story)> {
    let mut i = Interactions::load(map)?;
    i.set_entry(assets, map, "skool2", None)?;
    let mut world = World::from_bsp(map)?;
    i.sync(&mut world);
    let mut story = Story::load(assets, "skool2");
    let mut p;
    if beat == Beat::Dice {
        p = Player::spawn(&world, vec3(-906., -3170., -192.)).context("Dice altar blocked")?;
        let mut stats = crate::inventory::Stats::for_level("skool2", None);
        let catalog = crate::inventory::Catalog::load(assets)?;
        let pickups = crate::inventory::pickups(map, "skool2", &catalog);
        let copies = stats.copies(6);
        crate::inventory::collect(&mut stats, &pickups, p.feet, &world);
        ensure!(stats.copies(6) > copies, "Actual Dice pickup not reachable");
        ensure!(story.trigger(cinema::DICE), "Dice pickup dialogue missing");
        i.prepare_story(&mut story);
    } else {
        let e = map.entities.iter().find(|e| e.get("move_thread").is_some_and(|n| n == "extendBleachers")).unwrap();
        let origin = vector(&e["origin"]).unwrap();
        p = Player::spawn(&world, origin + vec3(-45., 0., 48.)).context("Gym lever blocked")?;
        for _ in 0..240 { p.tick(&world, crate::movement::Controls::default()); }
        let yaw = e.get("angle").and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.).to_radians();
        let target = origin + Quat::from_rotation_z(yaw) * vec3(36., 0., 26.);
        let aim = (target - p.eye()).normalize();
        i.update(1. / 60., map, &mut world, &mut p, aim, true)?;
        ensure!(i.gym.as_ref().unwrap().used, "Real lever use did not activate gym");
    }
    ensure!(i.school2.as_ref().unwrap().cinema.as_ref().is_some_and(|s| s.beat == beat && s.active), "Missing extra school scene {beat:?}");
    Ok((i, world, p, story))
}
fn extra_check(assets: &mut Assets, map: &Bsp) -> Result<()> {
    for beat in [Beat::Dice, Beat::Bleachers] {
        let mut expected = None;
        for skip in [None, Some(2.), Some(5.)] {
            let (mut i, mut world, mut p, mut story) = extra_setup(assets, map, beat)?;
            let home = p.feet;
            let hints = crate::cheshire::Hints::load(assets, map, "skool2")?;
            let gym_start = serde_json::to_value(i.gym.as_ref().unwrap().snapshot())?;
            let quest = serde_json::to_value(&i.school2.as_ref().unwrap().quest.items)?;
            if beat == Beat::Bleachers {
                ensure!(!i.skip_cinematic(map, &mut world, &mut p, &mut story)?, "Bleachers skipped before independent movement was armed");
            }
            let mut skipped = false;
            let mut ended = false;
            for frame in 0..3600 {
                if !skipped && !ended && skip.is_some_and(|t| frame as f32 / 60. >= t) {
                    skipped = i.skip_cinematic(map, &mut world, &mut p, &mut story)?;
                }
                if i.scripted() {
                    ensure!(!i.school2.as_ref().unwrap().dice_guards, "Dice ambush activated during shot");
                    if frame % 79 == 0 {
                        let before = serde_json::to_value(i.snapshot())?;
                        i.advance_school(0., map, &mut world, &mut p)?;
                        ensure!(before == serde_json::to_value(i.snapshot())?, "Extra school scene moved while paused");
                        let camera = i.school2.as_ref().unwrap().scene_camera().unwrap();
                        i.restore(&serde_json::from_value(before)?, map)?;
                        i.sync(&mut world);
                        story.restore(&serde_json::from_value(serde_json::to_value(story.snapshot())?)?, &hints)?;
                        let next = i.school2.as_ref().unwrap().scene_camera().unwrap();
                        ensure!(camera.eye.distance(next.eye) < 0.001 && camera.up.distance(next.up) < 0.001, "Extra school camera changed after load");
                    }
                }
                tick(&mut i, map, &mut world, &mut p, &mut story, 1. / 60., false)?;
                i.activate_enemies();
                if beat == Beat::Bleachers && frame < 80 {
                    ensure!(gym_start == serde_json::to_value(i.gym.as_ref().unwrap().snapshot())?, "Bleachers moved before authored delay");
                }
                ended |= !i.scripted();
                if ended && frame >= 1200 { break; }
            }
            ensure!(ended && !story.busy() && world.body_clear(p.feet) && p.feet.distance(home) < 1., "Extra school scene unsafe or unfinished {beat:?}");
            let s = i.school2.as_ref().unwrap();
            ensure!(quest == serde_json::to_value(&s.quest.items)? && s.quest.stage == Stage::Explore, "Presentation changed ingredient quest");
            ensure!(s.dice_guards == (beat == Beat::Dice), "Wrong Dice encounter commitment");
            if beat == Beat::Dice {
                ensure!(s.boojums[5].active && story.completed == 1, "Dice activation or dialogue lost");
            }
            let gym = serde_json::to_value(i.gym.as_ref().unwrap().snapshot())?["steps"].clone();
            if let Some(ref prior) = expected { ensure!(*prior == gym, "Watched/skipped gym geometry differs"); } else { expected = Some(gym); }
            let before = serde_json::to_value(i.snapshot())?;
            ensure!(!i.skip_cinematic(map, &mut world, &mut p, &mut story)?, "Extra school scene replayed");
            i.prepare_story(&mut story);
            if beat == Beat::Dice { i.completed_dialogue(cinema::DICE); }
            ensure!(before == serde_json::to_value(i.snapshot())?, "Repeated extra school completion changed state");
            println!("PASS {beat:?} skip={skip:?}: actual pickup/use binding, delayed movers/encounters, pause, restore, safe one-time handoff");
        }
    }
    Ok(())
}

pub async fn render(assets: &mut Assets) -> Result<()> {
    let mut scene = crate::render::Scene::load(assets, "skool2")?;
    for (beat, seconds) in [
        (Beat::Dice, 3.),
        (Beat::Dice, 6.),
        (Beat::Bleachers, 4.),
        (Beat::Bleachers, 10.),
        (Beat::Bleachers, 12.),
        (Beat::Mushroom, 3.),
        (Beat::Mushroom, 8.),
        (Beat::Mushroom, 17.),
        (Beat::Mushroom, 25.),
        (Beat::Mushroom, 30.),
        (Beat::Warp, 4.5),
        (Beat::Warp, 7.5),
        (Beat::Spice, 4.),
        (Beat::Spice, 14.),
        (Beat::Spice, 22.),
        (Beat::Spice, 31.),
        (Beat::Spice, 39.),
        (Beat::Growth, 1.),
        (Beat::Growth, 3.),
        (Beat::Growth, 5.),
        (Beat::Growth, 8.),
        (Beat::Final, 3.),
        (Beat::Final, 8.),
        (Beat::Final, 14.),
        (Beat::Final, 20.),
        (Beat::Final, 26.),
        (Beat::Final, 32.),
    ] {
        let (mut i, mut world, mut p, mut story) = setup(assets, &scene.map, beat)?;
        for _ in 0..(seconds * 60.) as usize {
            tick(
                &mut i,
                &scene.map,
                &mut world,
                &mut p,
                &mut story,
                1. / 60.,
                false,
            )?;
        }
        let s = i.school2.as_ref().unwrap();
        let c = s
            .scene_camera()
            .unwrap_or(Camera::look(p.eye() - Vec3::X * 150., p.eye()));
        let mut art = super::Art::load(assets, s)?;
        art.story_pose(&story);
        for frame in 0..3 {
            clear_background(BLACK);
            set_camera(&Camera3D {
                position: c.eye,
                target: c.target,
                up: c.up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            });
            crate::render_fx::begin(
                c.eye,
                (c.target - c.eye).normalize_or_zero(),
                seconds,
                &scene.atmosphere,
                false,
            );
            let transforms = i.transforms();
            scene.draw(c.eye, seconds, false, false, &transforms);
            art.draw(s, false, &scene.atmosphere, c.eye);
            crate::render::depth_read_only(|| scene.draw(c.eye, seconds, false, true, &transforms));
            crate::render_fx::finish();
            set_default_camera();
            if frame == 2 {
                crate::viewer::save_capture(std::path::Path::new(&format!(
                    "private/school2-scene-{beat:?}-{seconds}.png"
                )))?;
            }
            next_frame().await;
        }
    }
    Ok(())
}

//! The observatory return quest. Timelines are explicit, saved and independent of frame rate.
use crate::{
    event::{Condition as C, Facts},
    school2_quest::Items,
};
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum Phase {
    #[default]
    Locked,
    Open,
    Rising,
    Observatory,
    Opening,
    Drinking,
    Shrinking,
    Complete,
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub phase: Phase,
    pub time: f32,
    pub star: bool,
    pub potion: bool,
    pub position: macroquad::prelude::Vec3,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<crate::school::return_cinema::State>,
}
impl State {
    pub fn enter(&mut self, phase: Phase) {
        self.phase = phase;
        self.time = 0.;
    }
    pub fn cinematic(&self) -> bool {
        self.phase >= Phase::Opening || self.scene.as_ref().is_some_and(|s| s.active)
    }
    pub fn inventory(&mut self, items: &mut Items) {
        if self.phase >= Phase::Open {
            items.star = false;
        }
        if self.phase >= Phase::Drinking {
            items.potion = false;
        }
        self.star = items.star;
        self.potion = items.potion;
    }
    pub fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("return.star", self.star && self.phase == Phase::Locked);
        f.flag("return.finished", self.phase == Phase::Complete);
        f.flag("return.board", self.phase == Phase::Open && self.time >= 1.);
        f.flag(
            "return.exit",
            self.phase == Phase::Observatory && self.potion,
        );
        f
    }
    pub fn gate(&self, thread: &str) -> Option<C> {
        match thread {
            "shelf_cinematic"
            | "push_bookshelf1"
            | "push_bookshelf2"
            | "cat_fluff_cinematic"
            | "mallet_cat" => Some(C::Any(vec![])),
            "Skool1_Setup_OLift" => Some(C::flag("return.star")),
            "Skool1_OLift_Up" => Some(C::flag("return.board")),
            "observatory_exit_cinematic" => Some(C::flag("return.exit")),
            _ => None,
        }
    }
    pub fn objective(&self) -> &'static str {
        match self.phase {
            Phase::Locked if !self.star => "The observatory needs the Lucky Star from the Elder Gnome.",
            Phase::Locked => "Return: climb the library to the star doors. Your Lucky Star opens the lift.",
            Phase::Open => "Step aboard the observatory lift through the open star doors.",
            Phase::Rising => "Ride the lift, then follow the stairs to the observatory globe.",
            Phase::Observatory if !self.potion => "The observatory exit needs the Drink Me potion from the Elder Gnome.",
            Phase::Observatory => "Climb to the high walkway above the globe to use the Drink Me potion.",
            _ => "The globe opens. Drink the potion and follow the White Rabbit into the Pool of Tears.",
        }
    }
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.position.is_finite()
                && self.time.is_finite()
                && (0. ..=1e8).contains(&self.time)
                && (self.phase < Phase::Open || !self.star)
                && (self.phase < Phase::Drinking || !self.potion),
            "Invalid observatory quest state"
        );
        if let Some(s) = &self.scene {
            s.validate(self.phase)?;
        }
        Ok(())
    }
}

use crate::{npc::Puppet, skeletal::Transform, weapons::Prop};
use macroquad::prelude::*;
pub struct Art {
    globe: Vec<Prop>,
    star: Prop,
    potion: Prop,
    pub(crate) alice: Puppet,
    vapor: crate::particles::Attached,
}
impl Art {
    fn draw_scene(
        &mut self,
        s: &crate::school::return_cinema::State,
        data: &crate::school::return_cinema::Data,
        bright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        use crate::school::return_cinema::Beat;
        let star = s.beat == Beat::Star;
        if star || s.time < 5. {
            let open = !star && s.time >= 3.;
            self.globe[usize::from(open)].draw_frame(
                data.points["globe_puzzle"],
                0.6,
                bright,
                if open { s.time - 3. } else { 0. },
                false,
            );
        }
        if !s.active {
            return;
        }
        let (pose, clip, age, scale) = data.actor(s);
        self.alice.atmosphere(atmosphere, camera);
        let mut watch = crate::facial::Watch::default();
        if star {
            watch.update(
                1.,
                Some(
                    pose.rotation.conjugate()
                        * (data.star(s).translation - pose.translation - Vec3::Z * 50.),
                ),
            );
        }
        self.alice.watch(watch);
        self.alice
            .draw(clip, age, clip == "idle_stand", pose, scale, bright);
        if star && (0.5..6.).contains(&s.time) {
            let alpha = ((s.time - 0.5) / 2.).clamp(0., 1.);
            for mesh in self.star.meshes_at(data.star(s), 1., bright, 0., false) {
                for v in &mut mesh.vertices {
                    v.color[3] = (alpha * 255.) as u8;
                }
                crate::render_fx::effect(mesh, crate::materials::Blend::Alpha);
            }
        }
        if !star && (7.5..11.3).contains(&s.time) {
            if let Some(hand) = self.alice.tag("tag_weapon", clip, age, pose, 1.) {
                self.potion.draw_frame(hand, 1., bright, age, false);
                let alice = &mut self.alice;
                let potion = &self.potion;
                crate::render::depth_read_only(|| {
                    self.vapor.draw(
                        age,
                        1.,
                        |birth, tag| {
                            let hand = alice
                                .tag("tag_weapon", "drink", birth, pose, 1.)
                                .unwrap_or(pose);
                            Transform {
                                translation: tag
                                    .map_or(hand.translation, |tag| potion.point(hand, tag, 1.)),
                                ..hand
                            }
                        },
                        |_, _, enabled| enabled,
                        camera,
                        atmosphere,
                    )
                });
            }
        }
    }
    pub fn load(
        assets: &mut crate::assets::Assets,
        specs: &std::collections::BTreeMap<String, crate::texture::MaterialSpec>,
    ) -> anyhow::Result<Self> {
        let mut alice = Puppet::load(
            assets,
            "alice",
            &["idle_stand", "drink", "jump_long_takeoff", "jump_falling3"],
            specs,
        )?;
        alice.show_attachments(false);
        Ok(Self {
            globe: ["idle", "open"]
                .iter()
                .map(|a| Prop::load_animation(assets, "globepuzzle", a, specs))
                .collect::<anyhow::Result<_>>()?,
            star: Prop::load(assets, "star", specs)?,
            potion: Prop::load_animation(assets, "beakerhand", "beakerhand", specs)?,
            vapor: crate::particles::Attached::load(assets, "beakerhand", specs)?
                .ok_or_else(|| anyhow::anyhow!("Missing held potion emitter"))?,
            alice,
        })
    }
    pub fn draw(
        &mut self,
        school: &crate::school::School,
        r: &State,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        if let (Some(s), Some(data)) = (&r.scene, &school.return_data) {
            self.draw_scene(s, data, fullbright, atmosphere, camera);
            return;
        }
        if r.phase < Phase::Drinking {
            let open = r.phase == Phase::Opening;
            self.globe[usize::from(open)].draw_frame(
                Transform {
                    translation: vec3(209.6, 3801.6, 1152.),
                    rotation: Quat::IDENTITY,
                },
                0.6,
                fullbright,
                if open { (r.time - 3.).max(0.) } else { 0. },
                false,
            );
        }
        if r.phase == Phase::Rising {
            self.star.draw(
                Transform {
                    translation: vec3(727.58, 4337., 464.52)
                        + Vec3::Z
                            * if r.phase == Phase::Rising {
                                r.time * 126.4
                            } else {
                                0.
                            },
                    rotation: Quat::from_rotation_z(r.time * 2.),
                },
                1.,
                fullbright,
            );
        }
        if r.cinematic() {
            let shrinking = r.phase >= Phase::Shrinking;
            let clip = if shrinking { "jump_falling3" } else { "drink" };
            let scale = if shrinking {
                if r.phase == Phase::Complete {
                    0.01
                } else {
                    (1. - r.time / 2.5).max(0.01)
                }
            } else {
                1.
            };
            let transform = Transform {
                translation: r.position,
                rotation: Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
            };
            if r.phase >= Phase::Drinking {
                self.alice
                    .draw(clip, r.time, false, transform, scale, fullbright);
                if !shrinking {
                    if let Some(hand) = self.alice.tag("tag_weapon", clip, r.time, transform, scale)
                    {
                        self.potion.draw(hand, 1., fullbright);
                    }
                }
            }
        }
    }
}
impl State {
    pub fn camera(&self) -> Option<(Vec3, Vec3, bool)> {
        (self.scene.is_none() && self.cinematic()).then(|| {
            if self.phase == Phase::Opening {
                (vec3(208., 4260., 1450.), vec3(208., 3801., 1270.), false)
            } else {
                (
                    vec3(410., 4300., 1420.),
                    self.position + Vec3::Z * 30.,
                    false,
                )
            }
        })
    }
}

pub async fn render_check(assets: &mut crate::assets::Assets) -> anyhow::Result<()> {
    for phase in [
        "return-star",
        "return-star-close",
        "return-star-skipped",
        "return-exit-start",
        "return-takeoff",
        "return-skipped",
        "return-lift",
        "return-observatory",
        "return-globe",
        "return-potion",
        "return-shrink",
    ] {
        let mut scene = crate::render::Scene::load(assets, "skool1")?;
        let mut i = crate::interaction::Interactions::load(&scene.map)?;
        i.set_entry(assets, &scene.map, "skool1", Some("skool1_start2"))?;
        i.sync(&mut scene.world);
        let mut p = crate::movement::Player::new(Vec3::ZERO);
        let mut stats = crate::inventory::Stats::for_level("skool1", Some("skool1_start2"));
        i.school.as_mut().unwrap().return_fixture(
            phase,
            &scene.map,
            &mut scene.world,
            &mut p,
            &mut stats,
        )?;
        i.sync(&mut scene.world);
        let specs = crate::texture::read_materials(assets)?;
        let mut art = crate::school::Art::load(assets, &specs)?;
        let mut alice = crate::character::Character::load(assets)?;
        alice.reset(&p, 0.);
        for frame in 0..4 {
            let s = i.school.as_ref().unwrap();
            let (mut eye, mut target, mut show) = if phase == "return-observatory" {
                (vec3(692., 4120., 1120.), vec3(208., 3800., 1152.), false)
            } else {
                s.return_visit.as_ref().unwrap().camera().unwrap_or((
                    p.feet + vec3(-60., -60., 100.),
                    p.feet + Vec3::Z * 20.,
                    true,
                ))
            };
            let mut up = Vec3::Z;
            if let Some(c) = s.scene_camera() {
                eye = c.eye;
                target = c.target;
                up = c.up;
                show = false;
            }
            clear_background(BLACK);
            let camera = Camera3D {
                position: eye,
                target,
                up,
                fovy: 75_f32.to_radians(),
                z_near: 2.,
                z_far: 30000.,
                ..Default::default()
            };
            set_camera(&camera);
            crate::render_fx::begin_view(
                &camera,
                s.return_visit.as_ref().unwrap().time,
                &scene.atmosphere,
                false,
            );
            scene.draw(eye, 1., false, false, &i.transforms());
            art.draw(s, false, &scene.atmosphere, eye);
            if show {
                alice.atmosphere(&scene.atmosphere, eye);
                alice.draw(p.feet, false);
            }
            crate::render_fx::finish();
            set_default_camera();
            let (color, a) = s.scene_fade();
            draw_rectangle(
                0.,
                0.,
                screen_width(),
                screen_height(),
                Color { a, ..color },
            );
            draw_text(
                &format!("STAGED RETURN CHECK / {phase}"),
                20.,
                28.,
                22.,
                WHITE,
            );
            if frame == 3 {
                get_screen_data().export_png(&format!("private/{phase}.png"));
            }
            next_frame().await;
        }
        alice.check_visible(phase).await?;
    }
    Ok(())
}

/// Negative checks against the original brush volumes, separate from normal traversal.
pub fn check_gates(assets: &mut crate::assets::Assets) -> anyhow::Result<()> {
    let map = crate::bsp::Bsp::parse(&assets.read("maps/skool1.bsp")?)?;
    let mut i = crate::interaction::Interactions::load(&map)?;
    i.set_entry(assets, &map, "skool1", Some("skool1_start2"))?;
    let mut stats = crate::inventory::Stats::default();
    let portal = vec3(208., 3804., 1152.);
    let doors = vec3(692., 4174., 384.);
    let cinema = vec3(340., 4396., 1344.);
    for (star, potion) in [(false, false), (false, true), (true, false), (true, true)] {
        stats.school_items.star = star;
        stats.school_items.potion = potion;
        i.school.as_mut().unwrap().sync_inventory(&mut stats);
        anyhow::ensure!(
            i.triggers(0.01, portal, portal).transition.is_none(),
            "Hidden globe exit bypassed quest"
        );
        i.triggers(0.01, cinema, cinema);
        anyhow::ensure!(
            !i.school.as_ref().unwrap().cinematic(),
            "Potion exit bypassed the lift"
        );
    }
    stats.school_items.star = false;
    i.school.as_mut().unwrap().sync_inventory(&mut stats);
    i.triggers(0.01, doors, doors);
    anyhow::ensure!(
        i.school
            .as_ref()
            .unwrap()
            .return_visit
            .as_ref()
            .unwrap()
            .phase
            == Phase::Locked,
        "Star doors opened without reward"
    );
    stats.school_items.star = true;
    i.school.as_mut().unwrap().sync_inventory(&mut stats);
    i.triggers(0.01, doors, doors);
    anyhow::ensure!(
        i.school
            .as_ref()
            .unwrap()
            .return_visit
            .as_ref()
            .unwrap()
            .phase
            == Phase::Open,
        "Star doors did not open"
    );
    i.school
        .as_mut()
        .unwrap()
        .return_visit
        .as_mut()
        .unwrap()
        .enter(Phase::Observatory);
    stats.school_items.potion = false;
    i.school.as_mut().unwrap().sync_inventory(&mut stats);
    i.triggers(0.01, cinema, cinema);
    anyhow::ensure!(
        !i.school.as_ref().unwrap().cinematic(),
        "Missing potion accepted"
    );
    stats.school_items.potion = true;
    i.school.as_mut().unwrap().sync_inventory(&mut stats);
    anyhow::ensure!(
        i.triggers(0.01, portal, portal).transition.is_none(),
        "Hidden globe exit bypassed drinking"
    );
    i.triggers(0.01, cinema, cinema);
    anyhow::ensure!(
        i.school.as_ref().unwrap().cinematic(),
        "Completed prerequisites rejected"
    );
    println!("PASS actual star/lift/potion gates and hidden globe exit bypass prevention");
    crate::school::return_check::check(assets, &map)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rewards_gate_the_return_and_are_spent_once() {
        let mut s = State::default();
        let mut items = Items::default();
        s.inventory(&mut items);
        assert!(!s.gate("Skool1_Setup_OLift").unwrap().test(&s.facts()));
        items.star = true;
        items.potion = true;
        s.inventory(&mut items);
        assert!(s.gate("Skool1_Setup_OLift").unwrap().test(&s.facts()));
        s.enter(Phase::Open);
        s.inventory(&mut items);
        assert!(!items.star && items.potion);
        assert!(!s.gate("Skool1_OLift_Up").unwrap().test(&s.facts()));
        s.time = 1.;
        assert!(s.gate("Skool1_OLift_Up").unwrap().test(&s.facts()));
        assert!(!s
            .gate("observatory_exit_cinematic")
            .unwrap()
            .test(&s.facts()));
        s.enter(Phase::Observatory);
        assert!(s
            .gate("observatory_exit_cinematic")
            .unwrap()
            .test(&s.facts()));
        items.potion = false;
        s.inventory(&mut items);
        assert!(!s
            .gate("observatory_exit_cinematic")
            .unwrap()
            .test(&s.facts()));
        items.potion = true;
        s.enter(Phase::Drinking);
        s.inventory(&mut items);
        assert!(!items.potion && !items.star);
        for _ in 0..100 {
            s.inventory(&mut items);
        }
        assert_eq!(items, Items::default());
        assert!(s.validate().is_ok());
    }
    #[test]
    fn restart_preserves_return_phase_and_resource_history() {
        let mut stats = crate::inventory::Stats::for_level("skool1", Some("skool1_start2"));
        stats.damage(37.);
        stats.spend_will(21.);
        stats.select(6);
        stats.collected.insert("skool1:original-pickup".into());
        let mut state = State::default();
        state.enter(Phase::Open);
        state.inventory(&mut stats.school_items);
        let before = serde_json::to_value(&stats).unwrap();
        let mut restored: State =
            serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
        stats.ensure_level_weapons("skool1", Some("skool1_start2"));
        restored.inventory(&mut stats.school_items);
        assert_eq!(serde_json::to_value(stats).unwrap(), before);
    }
}

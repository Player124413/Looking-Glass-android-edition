//! Alice's original idle performances and conversation gestures, layered over gameplay.
use crate::{
    assets::Assets,
    audio::events::{Model as Sounds, Span},
    skeletal::{Animation, Definition, Skeleton, Transform},
    texture,
    weapons::{Prop, UNARMED},
};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub const IDLE_FAMILIES: [&str; 11] = [
    "knife_idle",
    "cards_idle",
    "mallet_idle",
    "jbomb_idle",
    "wand_idle",
    "jacks_idle",
    "dice_idle",
    "staff_idle",
    "buss_idle",
    "idle_base_01_play",
    "idle_base_01_play",
];
const EXTRA: [&str; 10] = [
    "ready_2_base01",
    "idle_base_01",
    "talk_01",
    "talk_02",
    "talk_03",
    "talk_04",
    "talk_gnomes_01",
    "talk_gnomes_02",
    "talk_gnomes_03",
    "pain_front",
];

type Commands = crate::animation_events::Model;
use crate::animation_events::{Command, VisualState};

fn numbered(name: &str, family: &str) -> bool {
    name.strip_prefix(family)
        .is_some_and(|tail| !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()))
}
pub fn clip_names(def: &Definition) -> Vec<String> {
    def.animations
        .keys()
        .filter(|n| EXTRA.contains(&n.as_str()) || IDLE_FAMILIES.iter().any(|p| numbered(n, p)))
        .cloned()
        .collect()
}

struct Clip {
    animation: Animation,
}
pub struct Acting {
    clips: BTreeMap<String, Clip>,
    commands: Commands,
    props: BTreeMap<String, Prop>,
    sounds: Sounds,
    cues: Vec<crate::audio::events::Cue>,
    pub state: State,
    dialogue: Option<String>,
    health: Option<f32>,
    hurt: bool,
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    idle: f32,
    cycle: usize,
    weapon: usize,
    current: Option<Play>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Play {
    clip: String,
    time: f32,
}
impl Acting {
    pub fn load(
        assets: &mut Assets,
        def: &Definition,
        skeleton: &Skeleton,
        specs: &BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut clips = BTreeMap::new();
        for name in clip_names(def) {
            let file = &def.animations[&name];
            clips.insert(
                name,
                Clip {
                    animation: Animation::parse(
                        &assets.read(&format!("{}/{file}", def.path))?,
                        skeleton.bones.len(),
                    )?,
                },
            );
        }
        let mut commands =
            Commands::parse(&String::from_utf8_lossy(&assets.read("models/alice.tik")?))?;
        commands.clips.retain(|n, _| clips.contains_key(n));
        let mut props = BTreeMap::new();
        for events in commands.clips.values() {
            for event in events {
                if let Command::Attach { model, tag, .. } = &event.command {
                    ensure!(
                        skeleton.bones.iter().any(|b| &b.name == tag),
                        "Missing idle attachment tag {tag}"
                    );
                    if !props.contains_key(model) {
                        props.insert(model.clone(), Prop::load(assets, model, specs)?);
                    }
                }
            }
        }
        Ok(Self {
            clips,
            commands,
            props,
            sounds: Sounds::load(assets, "models/alice.tik")?,
            cues: vec![],
            state: State::default(),
            dialogue: None,
            health: None,
            hurt: false,
        })
    }
    pub fn restore(&mut self, state: &State) -> Result<()> {
        ensure!(
            state.idle.is_finite() && state.idle >= 0. && state.weapon <= UNARMED,
            "Invalid saved idle performance"
        );
        if let Some(p) = &state.current {
            let clip = self
                .clips
                .get(&p.clip)
                .context("Unknown saved Alice gesture")?;
            ensure!(
                p.time.is_finite() && p.time >= 0. && p.time <= clip.animation.duration(),
                "Invalid saved gesture time"
            );
        }
        self.state = state.clone();
        self.health = None;
        self.hurt = false;
        self.cues.clear();
        Ok(())
    }
    pub fn reset(&mut self) {
        self.state = State::default();
        self.health = None;
        self.hurt = false;
        self.dialogue = None;
        self.cues.clear();
    }
    pub fn health(&mut self, health: f32) {
        if self.health.is_some_and(|old| health > 0. && health < old) {
            self.hurt = true;
        }
        self.health = Some(health);
    }
    pub fn dialogue(&mut self, story: &crate::story::Story) {
        self.dialogue = story
            .line()
            .filter(|l| crate::story::speakers::is_alice(&l.actor))
            .map(|line| {
                let family = if story.cast().iter().any(|name| name.contains("gnome")) {
                    "talk_gnomes_"
                } else {
                    "talk_"
                };
                let count = if family == "talk_" { 4 } else { 3 };
                let hash = line
                    .path
                    .bytes()
                    .fold(0usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize));
                format!("{family}{:02}", hash % count + 1)
            });
    }
    fn start(&mut self, name: String) {
        self.state.current = Some(Play {
            clip: name,
            time: 0.,
        });
    }
    /// Presentation only: it never blocks movement, attacks, resource use or quest events.
    pub fn update(&mut self, dt: f32, idle: bool, allowed: bool, busy: bool, weapon: usize) {
        if dt <= 0. {
            return;
        }
        if !allowed || busy || self.state.weapon != weapon {
            self.state.current = None;
            self.state.idle = 0.;
            self.state.weapon = weapon;
            self.hurt = false;
            return;
        }
        if self.hurt {
            self.hurt = false;
            self.state.idle = 0.;
            // Continuous hazards must allow the reaction to finish, rather than
            // restarting frame zero (and its voice command) every damage tick.
            if self
                .state
                .current
                .as_ref()
                .is_none_or(|p| p.clip != "pain_front")
            {
                self.start("pain_front".into());
            }
        } else if let Some(name) = self.dialogue.clone().filter(|_| idle) {
            self.state.idle = 0.;
            if self.state.current.as_ref().is_none_or(|p| p.clip != name) {
                self.start(name);
            }
        } else if !idle
            || self
                .state
                .current
                .as_ref()
                .is_some_and(|p| p.clip.starts_with("talk_"))
        {
            if self
                .state
                .current
                .as_ref()
                .is_none_or(|p| p.clip != "pain_front")
            {
                self.state.current = None;
            }
            self.state.idle = 0.;
        } else if self.state.current.is_none() {
            let before = self.state.idle;
            self.state.idle += dt;
            if before < 25. && self.state.idle >= 25. {
                self.start("ready_2_base01".into());
            } else if self.state.idle >= 40. {
                let choices: Vec<_> = self
                    .clips
                    .keys()
                    .filter(|n| numbered(n, IDLE_FAMILIES[weapon.min(UNARMED)]))
                    .cloned()
                    .collect();
                if !choices.is_empty() {
                    let name = choices[self.state.cycle % choices.len()].clone();
                    self.state.cycle = self.state.cycle.wrapping_add(1);
                    self.start(name);
                }
                self.state.idle = 25.;
            }
        }
        if let Some(play) = &mut self.state.current {
            let clip = &self.clips[&play.clip];
            let before = play.time;
            play.time = (play.time + dt).min(clip.animation.duration());
            self.cues.extend(self.sounds.between(
                &play.clip,
                Span {
                    start: before,
                    end: play.time,
                    duration: clip.animation.duration(),
                    frame_time: clip.animation.frame_time,
                    looping: false,
                    entered: before == 0.,
                },
            ));
            if before >= clip.animation.duration() {
                self.state.current = None;
            }
        }
    }
    pub fn apply(&self, pose: &mut [Transform], upper: &[bool], idle: bool) {
        let (name, time, looping) = if let Some(play) = &self.state.current {
            (play.clip.as_str(), play.time, false)
        } else if self.state.idle >= 25. && idle {
            ("idle_base_01", self.state.idle - 25., true)
        } else {
            return;
        };
        let clip = &self.clips[name].animation;
        let weight = if looping {
            1.
        } else {
            (time / 0.15).min(1.) * ((clip.duration() - time) / 0.15).clamp(0., 1.)
        };
        for (i, sample) in clip.sample(time, looping).into_iter().enumerate() {
            if idle || upper[i] {
                pose[i] = pose[i].blend(sample, weight);
            }
        }
    }
    fn visual(&self) -> VisualState {
        self.state
            .current
            .as_ref()
            .map_or_else(VisualState::default, |p| {
                let c = &self.clips[&p.clip].animation;
                self.commands
                    .visual(&p.clip, p.time, c.duration(), c.frame_time, false)
            })
    }
    pub fn hide_weapon(&self) -> bool {
        self.visual().hide_weapon
    }
    pub fn draw(
        &mut self,
        skeleton: &Skeleton,
        pose: &[Transform],
        transform: Transform,
        scale: f32,
        fullbright: bool,
    ) {
        if self.state.current.is_none() {
            return;
        }
        for (tag, attachment) in self.visual().attachments {
            let bone = skeleton.bones.iter().position(|b| b.name == tag).unwrap();
            self.props.get_mut(&attachment.model).unwrap().draw_frame(
                Transform {
                    translation: transform.point(pose[bone].translation * scale),
                    rotation: transform.rotation * pose[bone].rotation,
                },
                scale * attachment.scale,
                fullbright,
                attachment.age,
                true,
            );
        }
    }
    pub fn take_audio(&mut self) -> Vec<crate::audio::events::Cue> {
        std::mem::take(&mut self.cues)
    }
    /// Explicitly staged visual evidence; never used by the gameplay controller.
    pub fn stage(&mut self, clip: &str, time: f32, weapon: usize) -> Result<()> {
        let time = time.min(
            self.clips
                .get(clip)
                .context("Missing staged gesture")?
                .animation
                .duration()
                * 0.75,
        );
        self.restore(&State {
            weapon,
            current: Some(Play {
                clip: clip.into(),
                time,
            }),
            ..State::default()
        })
    }
}

pub fn check(assets: &mut Assets) -> Result<()> {
    let def = Definition::alice(assets)?;
    let skeleton = Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let commands = Commands::parse(&String::from_utf8_lossy(&assets.read("models/alice.tik")?))?;
    let names = clip_names(&def);
    let mut frames = 0;
    for name in &names {
        let clip = Animation::parse(
            &assets.read(&format!("{}/{}", def.path, def.animations[name]))?,
            skeleton.bones.len(),
        )?;
        for frame in &clip.frames {
            let pose = skeleton.global_pose(&frame.pose);
            ensure!(
                skeleton
                    .surfaces
                    .iter()
                    .flat_map(|s| &s.vertices)
                    .all(|v| v.position(&pose).is_finite()),
                "Invalid idle skin {name}"
            );
        }
        for event in commands.clips.get(name).into_iter().flatten() {
            if let Command::Attach { model, tag, .. } = &event.command {
                ensure!(
                    skeleton.bones.iter().any(|b| &b.name == tag),
                    "Unknown idle tag {tag}"
                );
                crate::weapons::read_model(assets, model)?;
            }
        }
        frames += clip.frames.len();
    }
    println!(
        "PASS {} Alice idle/talk/reaction clips, {frames} frames and original attachment commands",
        names.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn acting() -> Acting {
        let clips = [
            "idle_base_01",
            "ready_2_base01",
            "knife_idle1",
            "mallet_idle1",
            "pain_front",
            "talk_01",
        ]
        .into_iter()
        .map(|name| {
            (
                name.into(),
                Clip {
                    animation: Animation {
                        frames: (0..20)
                            .map(|_| crate::skeletal::Frame {
                                delta: Vec3::ZERO,
                                pose: vec![Transform {
                                    rotation: Quat::IDENTITY,
                                    translation: Vec3::X,
                                }],
                                min: Vec3::ZERO,
                                max: Vec3::ONE,
                            })
                            .collect(),
                        frame_time: 0.05,
                        distance: 0.,
                    },
                },
            )
        })
        .collect();
        Acting { clips, commands: Commands::parse("animations { mallet_idle1 test.ska { server { 2 hideweapon 1 2 attachmodel toy.tik tag_hand } } }").unwrap(), props: BTreeMap::new(), sounds: Sounds::default(), cues: vec![], state: State::default(), dialogue: None, health: None, hurt: false }
    }
    #[test]
    fn fidgets_pause_restore_and_cancel_on_attack_movement_or_equipment_change() {
        let mut a = acting();
        a.state.weapon = 2;
        a.state.idle = 39.99;
        a.update(0.2, true, true, false, 2);
        assert_eq!(a.state.current.as_ref().unwrap().clip, "mallet_idle1");
        assert!(a.hide_weapon());
        let saved = serde_json::to_string(&a.state).unwrap();
        a.update(0., true, true, false, 2);
        assert_eq!(serde_json::to_string(&a.state).unwrap(), saved);
        let state = serde_json::from_str(&saved).unwrap();
        for (idle, busy, weapon) in [(true, true, 2), (false, false, 2), (true, false, 0)] {
            a.restore(&state).unwrap();
            assert!(a.hide_weapon());
            a.update(0.1, idle, true, busy, weapon);
            assert!(!a.hide_weapon());
            assert!(a.state.current.is_none());
        }
    }
    #[test]
    fn gestures_follow_dialogue_and_damage_without_replacing_moving_legs() {
        let mut a = acting();
        a.dialogue = Some("talk_01".into());
        a.update(0.2, true, true, false, 0);
        assert_eq!(a.state.current.as_ref().unwrap().clip, "talk_01");
        a.dialogue = None;
        a.health(100.);
        a.health(90.);
        a.update(0.2, false, true, false, 0);
        assert_eq!(a.state.current.as_ref().unwrap().clip, "pain_front");
        let mut pose = vec![Transform {
            rotation: Quat::IDENTITY,
            translation: Vec3::ZERO,
        }];
        a.apply(&mut pose, &[false], false);
        assert_eq!(pose[0].translation, Vec3::ZERO);
        a.apply(&mut pose, &[true], false);
        assert_eq!(pose[0].translation, Vec3::X);
    }
    #[test]
    fn continuous_damage_finishes_reaction_and_restore_does_not_restart_it() {
        let mut a = acting();
        a.health(100.);
        for i in 1..=6 {
            a.health(100. - i as f32);
            a.update(0.1, false, true, false, 0);
        }
        assert!((a.state.current.as_ref().unwrap().time - 0.6).abs() < 0.001);
        let saved = a.state.clone();
        a.restore(&saved).unwrap();
        a.health(94.);
        a.update(0., false, true, false, 0);
        assert!((a.state.current.as_ref().unwrap().time - 0.6).abs() < 0.001);
        for i in 7..=11 {
            a.health(100. - i as f32);
            a.update(0.1, false, true, false, 0);
        }
        assert!(a.state.current.is_none());
    }
    #[test]
    fn clip_families_exclude_transitions_and_other_contexts() {
        assert!(numbered("knife_idle1", "knife_idle"));
        assert!(!numbered("idle_base_01_2_base_02", "idle_base_01"));
        assert!(!numbered("talk_goaway", "talk"));
    }
}

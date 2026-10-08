//! Weapon presentation with swept contacts for supported Rust combat actors.
mod blast;
mod bomb;
mod bomb_art;
pub mod check;
mod croquet_check;
pub mod dice_watch_check;
mod heavy;
mod heavy_art;
pub(crate) use heavy_art::EnemyBeam;
pub mod heavy_check;
mod ice;
mod ice_jacks_art;
pub mod ice_jacks_check;
mod jacks;
pub mod mallet_jack_check;
mod projectile;
pub mod route_heavy;
mod skool_check;
mod trail;
use crate::{
    assets::Assets,
    collision::World,
    combat::{self, Context as CombatContext, Hit},
    inventory::WEAPONS,
    skeletal::{Animation, Definition, Transform},
    tan, texture,
};
use anyhow::{ensure, Context, Result};
use macroquad::{
    miniquad::{BlendFactor, BlendState, Equation, ShaderSource},
    prelude::*,
};
pub(crate) use projectile::Projectile;

/// The Vorpal Blade's melee volume (40 units, native +-15 X/Y and 0..24 Z) for
/// headless route drivers, which use the same contact rules as the live weapon.
pub(crate) fn blade_melee(ctx: &CombatContext<'_>, origin: Vec3, aim: Vec3) -> Vec<Hit> {
    projectile::melee(ctx, origin, aim)
}

/// The dry primary stream has no persistent damage state. Replay drivers use
/// this same contact function after the ordinary action has paid for a pulse.
pub(crate) fn ice_primary(ctx: &CombatContext<'_>, origin: Vec3, aim: Vec3) -> Option<Hit> {
    ice::State::default().primary(ctx, origin, aim).0
}

/// The enemy a fired card locks onto: Alice's 90-degree, 2,048-unit acquisition cone.
pub(crate) fn acquire_target(ctx: &CombatContext<'_>, eye: Vec3, aim: Vec3) -> Option<usize> {
    projectile::target(ctx, eye - Vec3::Z * 20., aim)
}

pub const ACTION_CLIPS: [&str; 20] = [
    "wep_switch",
    "knife_att_1_prim_01",
    "knife_att_1_prim_02",
    "knife_att_1_prim_03",
    "knife_att_1_alt",
    "cards_att_prim",
    "cards_att_1_alt",
    "mallet_att_3_prim_01",
    "mallet_att_3_prim_02",
    "mallet_att_3_alt",
    "dice_att",
    "deadtime_watch",
    "wand_att_2_prim",
    "wand_att_2_alt",
    "staff_att_prim_middle",
    "staff_att_prim_end",
    "staff_att_alt_start",
    "staff_att_alt_middle",
    "staff_att_alt_end",
    "buss_att",
];
const EXTRA_MODELS: [&str; 7] = [
    "w_card_loose",
    "croquetball",
    "w_throwndice",
    "prj_knife",
    "w_playingcard",
    "prj_cardmirv",
    "prj_carddrunk",
];
pub fn supported(weapon: usize) -> bool {
    weapon < 10
}
fn one() -> u8 {
    1
}
fn already_paid() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Action {
    Equip(usize),
    Attack { alternate: bool },
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Playing {
    pub action: Action,
    pub clip: usize,
    pub time: f32,
    pub duration: f32,
    pub frame_time: f32,
    pub aim: Vec3,
    fired: bool,
    sounded: bool,
    // Old saves charged at click, before their release. Never debit them again.
    #[serde(default = "already_paid")]
    paid: bool,
    #[serde(default)]
    stage: crate::weapon_rules::Stage,
}
#[derive(Default, Debug)]
pub struct Events {
    pub fire: bool,
    pub sound: bool,
    emissions: Vec<Emission>,
    pub notice: Option<&'static str>,
    pub stops: usize,
    stop_times: Vec<f32>,
    pub blade_return: bool,
}
#[derive(Debug)]
struct Emission {
    weapon: usize,
    play: Playing,
    fire: bool,
    sound: bool,
    at: f32,
    pose: Vec<Transform>,
    anchors: Option<[Transform; 3]>,
}
impl Events {
    pub fn attach_contacts(&mut self, mut anchors: impl FnMut(&[Transform]) -> [Transform; 3]) {
        for e in &mut self.emissions {
            if !e.pose.is_empty() {
                e.anchors = Some(anchors(&e.pose));
            }
        }
    }
}
impl Playing {
    pub fn frame(&self) -> f32 {
        self.time / self.frame_time
    }
    #[cfg(test)]
    fn advance(&mut self, dt: f32) -> Events {
        if dt <= 0. {
            return Events::default();
        }
        self.time = (self.time + dt).min(self.duration);
        let (fire, sound) = match self.clip {
            0 => (5., 4.),
            1..=3 => (7., 5.),
            4 => (8., 6.),
            5 => (0., 0.),
            6 => (7., 7.),
            7 | 8 => (7., 0.),
            9 => (13., 2.),
            10 => (7., 7.),
            11 => (22., 22.),
            _ => unreachable!(),
        };
        let e = Events {
            fire: !self.fired && self.frame() >= fire,
            sound: !self.sounded && self.frame() >= sound,
            ..Default::default()
        };
        self.fired |= e.fire;
        self.sounded |= e.sound;
        e
    }
    pub fn alternate(&self) -> bool {
        matches!(self.action, Action::Attack { alternate: true })
    }
}
pub struct Actions {
    clips: Vec<Animation>,
    pub playing: Option<Playing>,
    pub selected: usize,
    pub pose: Vec<Transform>,
    start: Vec<Transform>,
    variation: usize,
    cooldowns: [f32; 10],
    blade_clock: Option<f32>,
    jack_clock: Option<[f32; 2]>,
    jacks_clock: Option<[f32; 2]>,
    return_pending: bool,
    pub time_stopped: bool,
    // Backend capability gate. Rules for unfinished toys can be exercised by
    // headless checks without enabling pretend attacks in gameplay.
    available: [bool; 10],
}
/// Internal presentation slot, distinct from any weapon/prop in the archives.
pub const UNARMED: usize = 10;
#[derive(Default, Clone)]
pub struct WeaponInput {
    pub dice: u8,
    pub selected: usize,
    /// Current held mode (false primary / true alternate); None is release.
    /// A one-frame Some is a tap, not a request queued through a busy action.
    pub click: Option<bool>,
    pub aim: Vec3,
    pub first_person: bool,
}
impl Actions {
    pub fn movement_locked(&self) -> bool {
        self.playing.as_ref().is_some_and(|p| {
            matches!(p.action, Action::Attack { .. })
                && (self.selected == 8
                    || (self.selected == 7
                        && p.alternate()
                        && p.stage != crate::weapon_rules::Stage::End))
        })
    }
    fn end_staff(&mut self) {
        if self.selected == 7 {
            if let Some(p) = &self.playing {
                if matches!(p.action, Action::Attack { .. })
                    && p.stage != crate::weapon_rules::Stage::End
                {
                    self.begin(
                        if p.alternate() { 18 } else { 15 },
                        p.action,
                        p.aim,
                        crate::weapon_rules::Stage::End,
                    );
                }
            }
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            playing: self.playing.clone(),
            selected: self.selected,
            pose: self.pose.clone(),
            start: self.start.clone(),
            variation: self.variation,
            cooldowns: self.cooldowns,
            blade_clock: self.blade_clock,
            jack_clock: self.jack_clock,
            jacks_clock: self.jacks_clock,
        }
    }
    pub fn restore(&mut self, s: &Snapshot) -> Result<()> {
        ensure!(
            s.selected <= UNARMED
                && s.pose.len() == self.pose.len()
                && s.start.len() == self.start.len(),
            "Invalid saved weapon pose"
        );
        if let Some(p) = &s.playing {
            let clip = self
                .clips
                .get(p.clip)
                .context("Invalid saved weapon action")?;
            ensure!(
                p.time >= 0.
                    && p.time <= p.duration
                    && (p.frame_time - clip.frame_time).abs() < 0.00001
                    && p.aim.is_finite()
                    && (p.duration
                        - action_duration(s.selected, p.stage, clip.duration(), p.alternate()))
                    .abs()
                        < 0.00001,
                "Invalid saved weapon action time"
            );
            if let Action::Equip(i) = p.action {
                ensure!(i <= UNARMED, "Invalid saved equipped toy");
                ensure!(p.clip == 0, "Invalid saved equip clip");
            } else {
                ensure!(
                    s.selected < 10 && self.available[s.selected],
                    "Unsupported saved attack"
                );
                let valid_clip = match p.stage {
                    crate::weapon_rules::Stage::Shot => (0..3).any(|v| {
                        crate::weapon_rules::rule(s.selected, p.alternate(), v).clip == p.clip
                    }),
                    crate::weapon_rules::Stage::Charge => {
                        s.selected == 7 && p.clip == if p.alternate() { 16 } else { 14 }
                    }
                    crate::weapon_rules::Stage::Sustain => {
                        s.selected == 7 && p.clip == if p.alternate() { 17 } else { 14 }
                    }
                    crate::weapon_rules::Stage::End => {
                        s.selected == 7 && p.clip == if p.alternate() { 18 } else { 15 }
                    }
                };
                ensure!(
                    valid_clip && (!p.fired || p.paid),
                    "Invalid saved attack phase"
                );
            }
            ensure!(
                s.selected != UNARMED,
                "An unarmed pose cannot have a weapon action"
            );
        }
        ensure!(
            s.cooldowns
                .iter()
                .all(|t| t.is_finite() && (0.0..=360.).contains(t)),
            "Invalid saved weapon cooldown"
        );
        self.playing = s.playing.clone();
        self.selected = s.selected;
        self.pose = s.pose.clone();
        self.start = s.start.clone();
        self.variation = s.variation;
        ensure!(
            s.blade_clock
                .is_none_or(|t| t.is_finite() && (0. ..=4.8).contains(&t)),
            "Invalid saved Blade recovery"
        );
        self.blade_clock = s.migrated_blade_clock();
        self.return_pending = false;
        ensure!(
            s.jack_clock
                .is_none_or(|[age, cycle]| (cycle == 3. || cycle == 8.)
                    && (0. ..=cycle + 1.3).contains(&age)),
            "Invalid saved Jackbomb recovery"
        );
        self.jack_clock = s.jack_clock;
        self.jacks_clock = s.jacks_clock;
        ensure!(
            s.jacks_clock
                .is_none_or(|[a, c]| (c == 2.5 || c == 6.5) && (0. ..=c + 1.3).contains(&a)),
            "Invalid saved Jacks recovery"
        );
        self.cooldowns = s.cooldowns;
        Ok(())
    }
    pub fn ready_to_attack(&self, selected: usize) -> bool {
        selected < 10
            && self.available[selected]
            && self.selected == selected
            && self.playing.is_none()
            && self.cooldowns[selected] <= 0.00001
    }
    pub fn load(assets: &mut Assets, bones: usize, pose: &[Transform]) -> Result<Self> {
        let clips = ACTION_CLIPS
            .iter()
            .map(|name| Animation::parse(&assets.read(&format!("models/alice/{name}.ska"))?, bones))
            .collect::<Result<_>>()?;
        Ok(Self {
            clips,
            playing: None,
            selected: 0,
            pose: pose.to_vec(),
            start: pose.to_vec(),
            variation: 0,
            cooldowns: [0.; 10],
            blade_clock: None,
            jack_clock: None,
            jacks_clock: None,
            return_pending: false,
            time_stopped: false,
            available: std::array::from_fn(supported),
        })
    }
    pub fn reset(&mut self, pose: &[Transform]) {
        self.playing = None;
        self.pose = pose.to_vec();
        self.start = self.pose.clone();
    }
    pub fn unarm(&mut self, pose: &[Transform]) {
        self.selected = UNARMED;
        self.reset(pose);
    }
    /// Unmetered harness for route/render fixtures. The interactive game uses
    /// update_funded; both paths execute the same scheduler and event ledger.
    pub fn update(
        &mut self,
        dt: f32,
        input: WeaponInput,
        base: &[Transform],
        upper: &[bool],
        full_body: bool,
    ) -> Events {
        self.update_funded(dt, input, base, upper, full_body, None)
    }
    pub fn update_funded(
        &mut self,
        dt: f32,
        input: WeaponInput,
        base: &[Transform],
        upper: &[bool],
        full_body: bool,
        mut stats: Option<&mut crate::inventory::Stats>,
    ) -> Events {
        use crate::weapon_rules::{canonical_mode, rule, Stage};
        let mut events = Events::default();
        if input.selected == UNARMED {
            self.unarm(base);
            return events;
        }
        if !dt.is_finite() || dt <= 0. {
            return events;
        }
        if stats.as_deref().is_some_and(|s| !s.alive()) {
            self.reset(base);
            return events;
        }
        if input.selected >= 10 || !input.aim.is_finite() {
            return events;
        }
        let mut selected = input.selected;
        let mut remaining = dt.min(1.);
        // At most twenty stream ticks per second plus transitions; retain the
        // remaining time across boundaries rather than adding a render-frame gap.
        for _ in 0..128 {
            if self
                .playing
                .as_ref()
                .is_some_and(|p| p.time + 0.000001 >= p.duration)
            {
                let previous = self.playing.take().unwrap();
                if self.selected == 7 && matches!(previous.action, Action::Attack { .. }) {
                    let alt = previous.alternate();
                    if previous.stage == Stage::Charge && input.click == Some(alt) {
                        self.begin(
                            if alt { 17 } else { 14 },
                            previous.action,
                            input.aim,
                            Stage::Sustain,
                        );
                    } else if previous.stage != Stage::End {
                        events.stops += 1;
                        events.stop_times.push(dt.min(1.) - remaining);
                        self.begin(
                            if alt { 18 } else { 15 },
                            previous.action,
                            input.aim,
                            Stage::End,
                        );
                    }
                }
            }
            if self.playing.is_none() {
                if selected != self.selected {
                    let old = self.selected;
                    self.selected = selected;
                    self.begin(0, Action::Equip(old), input.aim, Stage::Shot);
                } else if let Some(held) = input.click {
                    if !self.available[selected]
                        || stats.as_deref().is_some_and(|s| s.copies(selected) == 0)
                    {
                        break;
                    }
                    let wait = self.cooldowns[selected];
                    if wait > 0.000001 {
                        let step = remaining.min(wait);
                        self.tick_cooldowns(step);
                        remaining -= step;
                        if remaining <= 0.000001 {
                            break;
                        }
                        continue;
                    }
                    let alt = canonical_mode(selected, held);
                    let variation =
                        self.variation.wrapping_mul(214013).wrapping_add(2531011) & 0xffff_ffff;
                    let r = rule(selected, alt, (variation >> 16) & 0x7fff);
                    let affordable = stats.as_deref().is_none_or(|s| {
                        s.god
                            || (s.will() >= r.cost
                                && (selected != 7
                                    || if alt { s.will() >= 1. } else { s.will() > 20. }))
                    });
                    if !affordable {
                        events.notice = Some("Not enough Will / switching to Vorpal Blade");
                        if let Some(s) = stats.as_deref_mut() {
                            if s.select(0) {
                                selected = 0;
                                continue;
                            }
                        }
                        break;
                    }
                    if selected == 9 && stats.as_deref().is_some_and(|s| s.powers.recharge > 0.) {
                        break;
                    }
                    self.variation = variation;
                    self.begin(
                        r.clip,
                        Action::Attack { alternate: alt },
                        input.aim,
                        if selected == 7 {
                            Stage::Charge
                        } else {
                            Stage::Shot
                        },
                    );
                } else {
                    break;
                }
            }
            let p = self.playing.as_ref().unwrap();
            let attack = matches!(p.action, Action::Attack { .. });
            // Discrete attacks ignore release and finish before a queued switch.
            // Staff primary can cancel its charge; alternate finishes startup,
            // then enters its authored ending when its button is no longer held.
            if attack
                && self.selected == 7
                && p.stage != Stage::End
                && (input.click != Some(p.alternate())
                    || stats.as_deref().is_some_and(|s| !s.god && s.will() < 0.4))
                && (p.stage == Stage::Sustain || !p.alternate())
            {
                let alt = p.alternate();
                events.stops += 1;
                events.stop_times.push(dt.min(1.) - remaining);
                self.begin(
                    if alt { 18 } else { 15 },
                    Action::Attack { alternate: alt },
                    input.aim,
                    Stage::End,
                );
                continue;
            }
            let p = self.playing.as_ref().unwrap();
            let mut r = if attack {
                rule(self.selected, p.alternate(), 0)
            } else {
                crate::weapon_rules::Rule {
                    clip: 0,
                    release: 0.25,
                    debit: 0.25,
                    sound: 0.20,
                    cost: 0.,
                    cycle: 0.,
                }
            };
            // Staff's custom effect starts once. Repeated middle animation events
            // do not start extra beams or charge another startup price.
            if p.stage == Stage::Sustain || p.stage == Stage::End {
                r.release = f32::INFINITY;
                r.debit = f32::INFINITY;
                r.sound = f32::INFINITY;
            }
            let mut until = p.duration - p.time;
            for (done, time) in [
                (p.paid, r.debit),
                (p.fired, r.release),
                (p.sounded, r.sound),
            ] {
                if !done {
                    until = until.min((time - p.time).max(0.));
                }
            }
            let step = remaining.min(until.max(0.));
            self.tick_cooldowns(step);
            remaining -= step;
            let p = self.playing.as_mut().unwrap();
            p.time += step;
            let mut paid_now = false;
            if !p.paid && p.time + 0.000001 >= r.debit {
                let accepted = stats.as_deref_mut().is_none_or(|s| {
                    if !attack {
                        true
                    } else if self.selected == 9 {
                        s.watch().is_ok()
                    } else {
                        r.cost == 0. || s.spend_will(r.cost)
                    }
                });
                if !accepted {
                    events.notice = Some("Attack cancelled / resource unavailable");
                    self.playing = None;
                    break;
                }
                p.paid = true;
                paid_now = true;
                if attack && self.selected == 9 {
                    events.notice = Some("Pocket Watch / time stopped for 20 seconds");
                }
            }
            let fire = !p.fired && p.paid && p.time + 0.000001 >= r.release;
            let sound = !p.sounded && p.time + 0.000001 >= r.sound;
            if fire {
                p.fired = true;
            }
            if sound {
                p.sounded = true;
            }
            if fire || sound {
                if attack {
                    p.aim = input.aim;
                }
                events.emissions.push(Emission {
                    weapon: self.selected,
                    play: p.clone(),
                    at: dt.min(1.) - remaining,
                    anchors: None,
                    pose: if fire && attack {
                        let sample = self.clips[p.clip].sample(p.time, false);
                        let incoming = if self.selected == 4 && p.clip == 12 {
                            1.
                        } else {
                            (p.time / 0.08).min(1.)
                        };
                        let outgoing = if self.selected == 4 && p.clip == 12 {
                            1.
                        } else {
                            ((p.duration - p.time) / 0.1).clamp(0., 1.)
                        };
                        base.iter()
                            .zip(sample)
                            .enumerate()
                            .map(|(i, (&b, a))| {
                                if full_body
                                    || self.selected == 8
                                    || (self.selected == 7 && p.alternate())
                                    || upper[i]
                                {
                                    b.blend(self.start[i].blend(a, incoming), outgoing)
                                } else {
                                    b
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    fire,
                    sound,
                });
                events.fire |= fire;
                events.sound |= sound;
            }
            if attack && (fire || (paid_now && self.selected == 8)) {
                self.cooldowns[self.selected] = self.cooldowns[self.selected].max(r.cycle);
                if self.selected == 0 && p.alternate() && fire {
                    self.blade_clock = Some(0.);
                }
                if self.selected == 3 && fire {
                    self.jack_clock = Some([0., r.cycle]);
                }
                if self.selected == 5 && fire {
                    self.jacks_clock = Some([0., r.cycle]);
                }
            }
            if self.selected == 7 && attack {
                p.aim = input.aim;
                // A held middle state stays alive; its native effect owns pulses.
                if p.stage == Stage::Sustain {
                    p.time %= p.duration;
                }
            }
            if remaining <= 0.000001 {
                break;
            }
            // A sustain state with no events consumes the rest in one pass.
        }
        self.tick_cooldowns(remaining);
        events.blade_return = std::mem::take(&mut self.return_pending);
        self.pose = base.to_vec();
        if let Some(play) = &self.playing {
            let incoming = if self.selected == 4 && play.clip == 12 {
                1.
            } else {
                (play.time / 0.08).min(1.)
            };
            let outgoing = if self.selected == 4 && play.clip == 12 {
                1.
            } else if play.stage == Stage::Shot || play.stage == Stage::End {
                ((play.duration - play.time) / 0.10).clamp(0., 1.)
            } else {
                1.
            };
            let sample = self.clips[play.clip].sample(
                play.time,
                play.stage == Stage::Sustain || (self.selected == 7 && play.clip == 14),
            );
            for (i, (&b, a)) in base.iter().zip(sample).enumerate() {
                if full_body
                    || self.selected == 8
                    || (self.selected == 7 && play.alternate())
                    || upper[i]
                {
                    self.pose[i] = b.blend(self.start[i].blend(a, incoming), outgoing);
                }
            }
        }
        events
    }
    fn begin(&mut self, clip: usize, action: Action, aim: Vec3, stage: crate::weapon_rules::Stage) {
        self.start.clone_from(&self.pose);
        let alternate = matches!(action, Action::Attack { alternate: true });
        self.playing = Some(Playing {
            action,
            clip,
            time: 0.,
            duration: action_duration(self.selected, stage, self.clips[clip].duration(), alternate),
            frame_time: self.clips[clip].frame_time,
            aim,
            fired: false,
            sounded: false,
            paid: false,
            stage,
        });
    }
    fn tick_cooldowns(&mut self, dt: f32) {
        if let Some([age, cycle]) = &mut self.jack_clock {
            self.return_pending |= *age < *cycle - 0.5 && *age + dt >= *cycle - 0.5;
            *age += dt;
            if *age >= *cycle + 1.3 {
                self.jack_clock = None;
            }
        }
        if let Some([age, cycle]) = &mut self.jacks_clock {
            self.return_pending |= *age < *cycle - 0.5 && *age + dt >= *cycle - 0.5;
            *age += dt;
            if *age >= *cycle + 1.3 {
                self.jacks_clock = None;
            }
        }
        for t in &mut self.cooldowns {
            *t = (*t - dt).max(0.);
        }
        if let Some(t) = &mut self.blade_clock {
            let next = *t + dt;
            self.return_pending |= *t < 3. && next >= 3.;
            *t = next;
            if next >= 4.8 {
                self.blade_clock = None;
            }
        }
    }
    fn return_age(&self) -> Option<f32> {
        if self.selected == 3 || self.selected == 5 {
            (if self.selected == 3 {
                self.jack_clock
            } else {
                self.jacks_clock
            })
            .filter(|[a, c]| *a >= *c - 0.5)
            .map(|[a, c]| a - c + 0.5)
        } else {
            self.blade_clock.filter(|t| *t >= 3.).map(|t| t - 3.)
        }
    }
    // Keep the last action available through effect dispatch on its terminal frame.
    pub fn finish_frame(&mut self) {
        if self.playing.as_ref().is_some_and(|p| {
            p.time >= p.duration
                && matches!(
                    p.stage,
                    crate::weapon_rules::Stage::Shot | crate::weapon_rules::Stage::End
                )
        }) {
            self.playing = None;
        }
    }
    pub fn equipment(&self) -> (usize, f32) {
        let (weapon, scale) = match &self.playing {
            Some(p) if matches!(p.action, Action::Equip(_)) => {
                let Action::Equip(old) = p.action else {
                    unreachable!()
                };
                if p.frame() < 5. {
                    (old, (1. - p.frame() / 5.).max(0.05))
                } else {
                    (self.selected, ((p.frame() - 5.) / 3.).clamp(0.05, 1.))
                }
            }
            Some(p) if [4, 10].contains(&p.clip) && p.fired => (self.selected, 0.),
            Some(p) if [7, 8].contains(&p.clip) => {
                let size = if p.frame() < 8. {
                    ((p.time - 0.2) / 0.25).clamp(0., 1.)
                } else {
                    (1. - (p.time - 0.4) / 0.5).clamp(0., 1.)
                };
                (self.selected, 1. + size * 0.5)
            }
            _ if self.selected == 0 && self.cooldowns[0] > 0. => (self.selected, 0.),
            _ => (self.selected, 1.),
        };
        (
            weapon,
            if [0, 3, 5].contains(&weapon) && self.cooldowns[weapon] > 0. {
                0.
            } else {
                scale
            },
        )
    }
}

fn action_duration(weapon: usize, stage: crate::weapon_rules::Stage, clip: f32, alt: bool) -> f32 {
    if weapon == 7 && stage == crate::weapon_rules::Stage::Charge && !alt {
        2.30
    } else {
        clip
    }
}

pub(crate) struct Prop {
    _material_layers: Vec<std::rc::Rc<crate::render_fx::Surface>>,
    model: tan::Model,
    meshes: Vec<Mesh>,
    scale: f32,
}
fn draw_mesh(mesh: &Mesh) {
    crate::render_fx::effect(mesh, crate::materials::Blend::AlphaAdd);
}
impl Prop {
    pub(crate) fn draw_electric(&self, art: &crate::electric::Art, remaining: f32) {
        for mesh in &self.meshes {
            art.draw(mesh, remaining);
        }
    }
    /// Centre world pickup art independently of the first-person hand anchor.
    pub(crate) fn draw_pickup(&mut self, center: Vec3, angle: f32) {
        let (lo, hi) = self
            .model
            .surfaces
            .iter()
            .flat_map(|s| s.frames[0].iter())
            .fold(
                (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
                |(lo, hi), &p| (lo.min(p), hi.max(p)),
            );
        let size = 32. / (hi - lo).max_element().max(0.001);
        let rotation = Quat::from_rotation_z(angle);
        let attachment = Transform {
            translation: center - rotation * ((lo + hi) * 0.5 * size),
            rotation,
        };
        self.draw(attachment, size / self.scale, true);
    }
    pub(crate) fn load(
        assets: &mut Assets,
        name: &str,
        specs: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let (def, model) = read_model(assets, name)?;
        Self::build(assets, def, model, specs)
    }
    pub(crate) fn load_animation(
        assets: &mut Assets,
        name: &str,
        clip: &str,
        specs: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let (def, model) = read_model_clip(assets, name, Some(clip))?;
        Self::build(assets, def, model, specs)
    }
    pub(crate) fn build(
        assets: &mut Assets,
        def: Definition,
        model: tan::Model,
        specs: &std::collections::BTreeMap<String, texture::MaterialSpec>,
    ) -> Result<Self> {
        let mut meshes = Vec::new();
        let mut material_layers = Vec::new();
        for s in &model.surfaces {
            let skin = def
                .skins
                .get(&s.name)
                .or_else(|| def.skins.get("all"))
                .with_context(|| format!("Weapon surface skin missing: {}/{}", def.path, s.name))?;
            let path = texture::resolve(assets, &format!("{}/{skin}", def.path), specs)
                .or_else(|| texture::resolve(assets, skin, specs))
                .with_context(|| format!("Weapon texture {skin}"))?;
            let tex = if crate::android::is_android() {
                texture::load_gpu(assets, &path, true, true)?
            } else {
                let image = texture::decode(assets, &path)?;
                let tex = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
                tex.set_filter(FilterMode::Linear);
                tex
            };
            material_layers.push(crate::render_fx::register(
                assets,
                &tex,
                &[format!("{}/{skin}", def.path), skin.clone()],
                specs,
            )?);
            meshes.push(Mesh {
                vertices: s
                    .uv
                    .iter()
                    .map(|&uv| Vertex {
                        position: Vec3::ZERO,
                        uv,
                        color: [255; 4],
                        normal: Vec4::ZERO,
                    })
                    .collect(),
                indices: s.indices.clone(),
                texture: Some(tex),
            });
        }
        Ok(Self {
            model,
            _material_layers: material_layers,
            meshes,
            scale: def.scale,
        })
    }
    pub(crate) fn point(&self, attachment: Transform, tag: &str, scale: f32) -> Vec3 {
        attachment.point(self.model.tags.get(tag).map_or(Vec3::ZERO, |v| v[0]) * self.scale * scale)
    }
    fn point_at(&self, attachment: Transform, tag: &str, time: f32) -> Vec3 {
        let position = self.model.tags.get(tag).map_or(Vec3::ZERO, |v| {
            let f = (time.max(0.) / self.model.frame_time).min((v.len() - 1) as f32);
            let a = f as usize;
            v[a].lerp(v[(a + 1).min(v.len() - 1)], f.fract())
        });
        attachment.point(position * self.scale)
    }
    pub(crate) fn draw(&mut self, attachment: Transform, scale: f32, fullbright: bool) {
        self.draw_frame(attachment, scale, fullbright, 0., false);
    }
    pub(crate) fn draw_frame(
        &mut self,
        attachment: Transform,
        scale: f32,
        fullbright: bool,
        time: f32,
        looping: bool,
    ) {
        for mesh in self.meshes_at(attachment, scale, fullbright, time, looping) {
            crate::render_fx::skin(mesh);
        }
    }
    /// Animate once; callers with layered materials may draw the same geometry
    /// several times. Always reset authored UVs before applying per-pass tcMods.
    pub(crate) fn meshes_at(
        &mut self,
        attachment: Transform,
        scale: f32,
        fullbright: bool,
        time: f32,
        looping: bool,
    ) -> &mut [Mesh] {
        if scale <= 0. {
            return &mut self.meshes[..0];
        }
        for (s, m) in self.model.surfaces.iter().zip(&mut self.meshes) {
            let frame = time.max(0.) / self.model.frame_time.max(0.001);
            let frame = if looping {
                frame % s.frames.len() as f32
            } else {
                frame.min((s.frames.len() - 1) as f32)
            };
            let a = frame.floor() as usize;
            let b = if looping {
                (a + 1) % s.frames.len()
            } else {
                (a + 1).min(s.frames.len() - 1)
            };
            for ((v, &p), &q) in m.vertices.iter_mut().zip(&s.frames[a]).zip(&s.frames[b]) {
                let p = p.lerp(q, frame.fract());
                v.position = attachment.point(p * self.scale * scale);
                v.normal = Vec4::ZERO;
            }
            for tri in m.indices.chunks_exact(3) {
                let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                let n = (m.vertices[b].position - m.vertices[a].position)
                    .cross(m.vertices[c].position - m.vertices[a].position);
                for i in [a, b, c] {
                    m.vertices[i].normal += n.extend(0.);
                }
            }
            for (v, &uv) in m.vertices.iter_mut().zip(&s.uv) {
                v.uv = uv;
                let light = if fullbright {
                    1.
                } else {
                    0.65 + 0.35
                        * v.normal
                            .truncate()
                            .normalize_or_zero()
                            .dot(vec3(0.3, -0.5, 1.).normalize())
                            .max(0.)
                };
                v.color = [
                    (light * 255.) as u8,
                    (light * 255.) as u8,
                    (light * 255.) as u8,
                    255,
                ];
            }
        }
        &mut self.meshes
    }
}
pub(crate) fn read_model(assets: &mut Assets, name: &str) -> Result<(Definition, tan::Model)> {
    read_model_clip(assets, name, None)
}
pub(crate) fn read_model_clip(
    assets: &mut Assets,
    name: &str,
    selected: Option<&str>,
) -> Result<(Definition, tan::Model)> {
    let def = Definition::load(assets, &format!("models/{name}.tik"))?;
    let selected = selected
        .map(|clip| {
            def.animations
                .get(clip)
                .or_else(|| {
                    def.animations
                        .iter()
                        .find(|(name, _)| {
                            name.strip_prefix(clip).is_some_and(|tail| {
                                !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit())
                            })
                        })
                        .map(|(_, file)| file)
                })
                .with_context(|| format!("Missing prop animation {name}/{clip}"))
        })
        .transpose()?;
    // Some original attachments (notably the Hatter's cane) use SKB/SKA.
    // Bake their authored frames into the shared animated-prop representation.
    if !def.model.is_empty() {
        let skeleton = crate::skeletal::Skeleton::parse(
            &assets.read(&format!("{}/{}", def.path, def.model))?,
        )?;
        let file = selected
            .or_else(|| {
                def.animations
                    .get("idle")
                    .or_else(|| def.animations.get("notmoving"))
                    .or_else(|| def.animations.values().find(|f| f.ends_with(".ska")))
            })
            .context("Skeletal prop animation missing")?;
        let clip = Animation::parse(
            &assets.read(&format!("{}/{file}", def.path))?,
            skeleton.bones.len(),
        )?;
        let poses: Vec<_> = clip
            .frames
            .iter()
            .map(|f| skeleton.global_pose(&f.pose))
            .collect();
        let model = tan::Model {
            surfaces: skeleton
                .surfaces
                .iter()
                .map(|s| tan::Surface {
                    detail: Vec::new(),
                    name: s.name.clone(),
                    uv: s.vertices.iter().map(|v| v.uv).collect(),
                    indices: s.indices.clone(),
                    frames: poses
                        .iter()
                        .map(|pose| s.vertices.iter().map(|v| v.position(pose)).collect())
                        .collect(),
                })
                .collect(),
            tags: skeleton
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    (
                        b.name.clone(),
                        poses.iter().map(|p| p[i].translation).collect(),
                    )
                })
                .collect(),
            tag_rotations: skeleton
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| {
                    (
                        b.name.clone(),
                        poses.iter().map(|p| p[i].rotation).collect(),
                    )
                })
                .collect(),
            frame_time: clip.frame_time,
        };
        return Ok((def, model));
    }
    let file = selected
        .or_else(|| {
            def.animations
                .get("idle")
                .filter(|file| file.ends_with(".tan"))
                .or_else(|| def.animations.values().find(|file| file.ends_with(".tan")))
        })
        .context("Prop TAN model missing")?;
    let path = format!("{}/{file}", def.path);
    let model = tan::Model::parse(&assets.read(&path)?).with_context(|| path)?;
    Ok((def, model))
}

fn launch_from_eye(
    world: &World,
    eye: Vec3,
    raw: Vec3,
    direction: Vec3,
    first_person: bool,
    radius: f32,
) -> (Vec3, Vec3) {
    let reach = world.sweep(eye, raw, Vec3::splat(radius));
    let origin = eye.lerp(raw, reach.fraction);
    if !first_person {
        return (origin, direction);
    }
    let end = eye + direction * 2000.;
    // Aim at the surface, not the swept projectile's clearance plane. The latter
    // can coincide with the blocked hand and make a close shot run along the wall.
    let sight = world.sweep(eye, end, Vec3::ZERO);
    let target = eye.lerp(end, sight.fraction);
    (
        origin,
        (target - origin).try_normalize().unwrap_or(direction),
    )
}
struct Spark {
    position: Vec3,
    velocity: Vec3,
    life: f32,
    color: Color,
}
struct Trail {
    a: Vec3,
    b: Vec3,
    life: f32,
    color: Color,
    electric: bool,
}
fn append_swing(vertices: &mut Vec<Vertex>, trail: &[Trail], electric: bool) {
    for (i, t) in trail.windows(2).enumerate() {
        if t[0].electric != electric || t[1].electric != electric {
            continue;
        }
        if t[0].a.distance_squared(t[1].a) > 80. * 80. {
            continue;
        }
        let c = Color::new(
            t[1].color.r,
            t[1].color.g,
            t[1].color.b,
            t[1].life / 0.2 * 0.6,
        );
        let u0 = i as f32 / (trail.len() - 1) as f32;
        let u1 = (i + 1) as f32 / (trail.len() - 1) as f32;
        let points = [t[0].a, t[0].b, t[1].b, t[0].a, t[1].b, t[1].a];
        let uv = [
            vec2(u0, 0.),
            vec2(u0, 1.),
            vec2(u1, 1.),
            vec2(u0, 0.),
            vec2(u1, 1.),
            vec2(u1, 0.),
        ];
        for (position, uv) in points.into_iter().zip(uv) {
            vertices.push(Vertex {
                position,
                uv: if electric { uv } else { Vec2::splat(0.5) },
                color: if electric { [255; 4] } else { c.into() },
                normal: Vec4::ZERO,
            });
        }
    }
}
fn projectile_impact_sound(model: usize, flesh: bool) -> &'static str {
    match (model, flesh) {
        (0, true) => "sound/weapon/knife/knife_phit_flesh1.wav",
        (0, false) => "sound/weapon/knife/knife_phit_world1.wav",
        (10, true) => "sound/weapon/cards/cards_hit_flesh1.wav",
        (10, false) => "sound/weapon/cards/cards_hit_world1.wav",
        (_, true) => "sound/weapon/mallet/mallet_ball_flesh1.wav",
        (_, false) => "sound/weapon/mallet/mallet_ball_bounce.wav",
    }
}

pub struct Visuals {
    heavy: heavy::State,
    heavy_art: heavy_art::Art,
    ice: ice::State,
    ice_data: ice::Data,
    jacks: jacks::State,
    ice_art: ice_jacks_art::Art,
    pub ice_refund: f32,
    pub ice_drain: bool,
    pub owner_immersed: bool,
    bombs: bomb::State,
    bomb_data: bomb::Data,
    bomb_art: bomb_art::Art,
    blasts: Vec<blast::Blast>,
    pub owner_velocity: Vec3,
    pub spatial_sounds: Vec<(&'static str, Vec3)>,
    pub time_stopped: bool,
    pub world_ratio: Option<f32>,
    pub dice: crate::dice::State,
    dice_data: crate::dice::Data,
    dice_art: crate::dice::Art,
    pub dice_count: u8,
    props: Vec<Prop>,
    projectiles: Vec<Projectile>,
    sparks: Vec<Spark>,
    trail: Vec<Trail>,
    flight_trails: Vec<Trail>,
    effect_material: Material,
    spark_texture: Texture2D,
    swipe_texture: Texture2D,
    _swipe_surface: std::rc::Rc<crate::render_fx::Surface>,
    return_texture: Texture2D,
    eye: Vec3,
    first_person: bool,
    view_scale: f32,
    combat_anchors: [Transform; 3],
    combat_aim: Vec3,
    pub shots: u64,
    pub impacts: u64,
    pub hits: Vec<Hit>,
}
impl Visuals {
    pub fn croquet_in_flight(&self) -> bool {
        self.projectiles.iter().any(|p| p.model == 11)
    }
    pub fn cancel_staff(&mut self) {
        self.heavy.charge = None;
    }
    pub fn finish_staff(&self, actions: &mut Actions) {
        if self.heavy.charge.is_none()
            && actions.playing.as_ref().is_some_and(|p| {
                (p.fired || p.stage == crate::weapon_rules::Stage::Sustain)
                    && matches!(p.action, Action::Attack { .. })
                    && p.stage != crate::weapon_rules::Stage::End
            })
        {
            actions.end_staff();
        }
    }
    pub fn audio_loops(&self) -> Vec<crate::audio::LoopCue> {
        let ice_loop = self
            .ice
            .pulses
            .last()
            .filter(|p| p.1 + 1e-6 >= self.ice.clock)
            .map(|p| crate::audio::LoopCue {
                id: 0x2_0000_0000usize,
                origin: self.eye,
                path: if p.2 { ice::WALL_FIRE } else { ice::FIRE },
                clock: Some((self.ice.clock - p.0) as f32),
            });
        self.projectiles
            .iter()
            .filter(|p| p.kind != projectile::Kind::Fragment)
            .take(16)
            .map(|p| crate::audio::LoopCue {
                clock: if p.model == 0 {
                    Some(p.age as f32)
                } else {
                    None
                },
                id: p.audio_id,
                origin: p.position,
                path: if p.model == 0 {
                    "sound/weapon/knife/knife_spin.wav"
                } else if p.model == 10 {
                    "sound/weapon/cards/cards_loop1.wav"
                } else {
                    "sound/weapon/mallet/mallet_ball_loop.wav"
                },
            })
            .chain(
                self.bombs
                    .bombs
                    .iter()
                    .filter(|b| b.alternate && b.age >= self.bomb_data.crank + self.bomb_data.open)
                    .map(|b| crate::audio::LoopCue {
                        clock: Some((b.age - self.bomb_data.crank - self.bomb_data.open) as f32),
                        id: 0x1_0000_0000usize + b.id as usize,
                        origin: b.position,
                        path: bomb::BREATH,
                    }),
            )
            .chain(ice_loop)
            .chain(self.heavy.charge.iter().map(|c| crate::audio::LoopCue {
                id: 0x3_0000_0000usize + c.id as usize,
                origin: c.pose.translation,
                path: if !c.alternate && c.age >= 2.15 {
                    heavy::BEAM
                } else {
                    heavy::CHARGE
                },
                clock: Some(
                    (c.age
                        - if !c.alternate && c.age >= 2.15 {
                            2.15
                        } else {
                            0.
                        }) as f32,
                ),
            }))
            .chain(
                self.heavy
                    .shots
                    .iter()
                    .filter(|p| p.kind == heavy::Kind::Cannon)
                    .map(|p| crate::audio::LoopCue {
                        id: 0x4_0000_0000usize + p.id as usize,
                        origin: p.position,
                        path: heavy::BUSS_LOOP,
                        clock: Some(p.age as f32),
                    }),
            )
            .collect()
    }
    pub fn ice_locked(&self) -> bool {
        self.ice.locked()
    }
    pub fn follow_ice(&mut self, feet: Vec3) {
        if let Some(shell) = &mut self.ice.shell {
            shell.origin = feet;
        }
    }
    pub fn stop_ice(&mut self) {
        self.ice.stop();
    }
    pub fn frozen_body(&self, meshes: &[Option<Mesh>]) {
        if self.ice.locked() {
            self.ice_art.frozen_body(meshes);
        }
    }
    pub fn ice_targets(&self) -> Vec<combat::Target> {
        self.ice.targets()
    }
    pub fn hit_ice(&mut self, hit: Hit) -> bool {
        if hit.id < ice::WALL_BASE {
            return false;
        }
        if self.ice.hit(hit) {
            self.spatial_sounds
                .push(("sound/weapon/icewand/icewand_icewall_death.wav", self.eye));
        }
        true
    }
    pub fn threatens(&self, world: &World, target: crate::combat::Target) -> bool {
        self.projectiles.iter().any(|p| p.threatens(world, target))
    }
    pub fn snapshot(&self) -> ProjectileSave {
        ProjectileSave {
            heavy: self.heavy.clone(),
            ice: self.ice.clone(),
            jacks: self.jacks.clone(),
            bombs: self.bombs.clone(),
            blasts: self.blasts.clone(),
            dice: self.dice.clone(),
            dice_count: self.dice_count,
            projectiles: self.projectiles.clone(),
            shots: self.shots,
            impacts: self.impacts,
        }
    }
    pub fn restore(&mut self, s: &ProjectileSave) -> Result<()> {
        s.validate()?;
        let s = s.migrated();
        self.clear();
        self.heavy = s.heavy;
        self.ice = s.ice;
        self.jacks = s.jacks;
        self.dice = s.dice;
        self.bombs = s.bombs;
        self.blasts = s.blasts;
        self.dice_count = s.dice_count;
        self.projectiles = s.projectiles;
        self.shots = s.shots;
        self.impacts = s.impacts;
        Ok(())
    }
    pub fn load(assets: &mut Assets) -> Result<Self> {
        let mut props = Vec::new();
        let specs = texture::read_materials(assets)?;
        for name in WEAPONS
            .iter()
            .map(|(id, _)| format!("w_{id}"))
            .chain(EXTRA_MODELS.iter().map(|s| s.to_string()))
        {
            props.push(Prop::load(assets, &name, &specs)?);
        }
        let effect_material = load_material(
            ShaderSource::Glsl {
                vertex: crate::character::VERTEX,
                fragment: &crate::environment::fragment(EFFECT_FRAGMENT),
            },
            MaterialParams {
                uniforms: crate::environment::uniforms(),
                pipeline_params: crate::render::depth_pipeline(Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(macroquad::miniquad::BlendValue::SourceAlpha),
                    BlendFactor::One,
                ))),
                ..Default::default()
            },
        )
        .map_err(|e| anyhow::anyhow!("Weapon effect shader: {e:?}"))?;
        let mut pixels = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0..32 {
                let d = vec2((x as f32 + 0.5) / 16. - 1., (y as f32 + 0.5) / 16. - 1.).length();
                pixels.extend_from_slice(&[255, 255, 255, ((1. - d).max(0.).powi(3) * 255.) as u8]);
            }
        }
        let spark_texture = Texture2D::from_rgba8(32, 32, &pixels);
        spark_texture.set_filter(FilterMode::Linear);
        let path = texture::resolve(assets, "meta", &specs)
            .context("Missing Blade reappearance sprite")?;
        let image = texture::decode(assets, &path)?;
        let return_texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        return_texture.set_filter(FilterMode::Linear);
        let swipe_name = "powerups/lightningswipe";
        let path = texture::resolve(assets, swipe_name, &specs).context("Missing Mallet swipe")?;
        let image = texture::decode(assets, &path)?;
        let swipe_texture = Texture2D::from_rgba8(image.width, image.height, &image.pixels);
        swipe_texture.set_filter(FilterMode::Linear);
        let swipe_surface =
            crate::render_fx::register(assets, &swipe_texture, &[swipe_name.into()], &specs)?;
        Ok(Self {
            swipe_texture,
            _swipe_surface: swipe_surface,
            time_stopped: false,
            world_ratio: None,
            heavy: Default::default(),
            heavy_art: heavy_art::Art::load(assets, &specs)?,
            ice: Default::default(),
            ice_data: ice::Data::load(assets)?,
            jacks: Default::default(),
            ice_art: ice_jacks_art::Art::load(assets, &specs)?,
            ice_refund: 0.,
            ice_drain: false,
            owner_immersed: false,
            bombs: bomb::State::default(),
            bomb_data: bomb::Data::load(assets)?,
            bomb_art: bomb_art::Art::load(assets, &specs)?,
            blasts: Vec::new(),
            owner_velocity: Vec3::ZERO,
            dice: crate::dice::State::default(),
            dice_data: crate::dice::Data::load(assets)?,
            dice_art: crate::dice::Art::load(assets, &specs)?,
            dice_count: 1,
            props,
            projectiles: Vec::new(),
            sparks: Vec::new(),
            trail: Vec::new(),
            flight_trails: Vec::new(),
            effect_material,
            spark_texture,
            return_texture,
            eye: Vec3::ZERO,
            first_person: false,
            view_scale: 1.,
            combat_aim: Vec3::X,
            combat_anchors: [Transform {
                rotation: Quat::IDENTITY,
                translation: Vec3::ZERO,
            }; 3],
            shots: 0,
            impacts: 0,
            spatial_sounds: Vec::new(),
            hits: Vec::new(),
        })
    }
    pub fn atmosphere(&self, atmosphere: &crate::environment::Atmosphere, camera: Vec3) {
        *self.bomb_art.atmosphere.borrow_mut() = atmosphere.clone();
        *self.ice_art.atmosphere.borrow_mut() = atmosphere.clone();
        *self.heavy_art.atmosphere.borrow_mut() = atmosphere.clone();
        *self.dice_art.atmosphere.borrow_mut() = atmosphere.clone();
        atmosphere.apply(&self.effect_material, camera);
    }
    pub fn lights(&self) -> Vec<crate::lighting::Light> {
        self.jacks
            .pieces
            .iter()
            .filter(|p| !p.silent)
            .map(|p| crate::lighting::Light {
                position: p.position,
                color: if p.kind == jacks::Kind::Ball {
                    vec3(1., 1., 0.)
                } else {
                    vec3(0.25, 0.125, 0.125)
                },
                radius: if p.kind == jacks::Kind::Ball {
                    100.
                } else {
                    150.
                },
                only_models: false,
                flare: false,
            })
            .chain(
                self.projectiles
                    .iter()
                    .filter(|p| p.model == 11)
                    .flat_map(|p| crate::lighting::croquet(p.position))
                    .chain(
                        self.bombs
                            .bombs
                            .iter()
                            .filter(|b| b.age >= self.bomb_data.crank)
                            .map(|b| crate::lighting::Light {
                                position: self.bomb_data.tag(b, "tag_fuse").translation,
                                color: vec3(1., 0.5, 0.),
                                radius: if b.alternate
                                    && b.age >= self.bomb_data.crank + self.bomb_data.open
                                {
                                    450.
                                } else {
                                    250.
                                },
                                only_models: false,
                                flare: false,
                            }),
                    )
                    .chain(
                        self.blasts
                            .iter()
                            .filter(|b| b.kind == blast::Kind::Jack && (0.1..0.6).contains(&b.age))
                            .map(|b| crate::lighting::Light {
                                position: b.origin,
                                color: vec3(1., 0.5, 0.) * (1. - (b.age - 0.1) / 0.5),
                                radius: 300.,
                                only_models: false,
                                flare: false,
                            }),
                    ),
            )
            .chain(
                self.heavy
                    .shots
                    .iter()
                    .filter(|p| p.kind != heavy::Kind::Cannon)
                    .map(|p| crate::lighting::Light {
                        position: p.position,
                        color: if p.kind == heavy::Kind::Spiral {
                            vec3(1., 1., 0.)
                        } else {
                            vec3(0.5, 0.5, 1.)
                        },
                        radius: if p.kind == heavy::Kind::Spiral {
                            200.
                        } else {
                            500.
                        },
                        only_models: false,
                        flare: false,
                    }),
            )
            .chain(self.heavy.charge.iter().map(|c| crate::lighting::Light {
                position: c.pose.translation,
                color: vec3(0.15, 0.15, 0.5),
                radius: 200.,
                only_models: false,
                flare: false,
            }))
            .chain(
                self.heavy
                    .impacts
                    .iter()
                    .filter(|p| p.kind == heavy::Kind::Cannon && p.age < 0.75)
                    .map(|p| crate::lighting::Light {
                        position: p.origin,
                        color: vec3(1., 1., 0.),
                        radius: 300.,
                        only_models: false,
                        flare: false,
                    }),
            )
            .collect()
    }
    pub fn clear(&mut self) {
        self.heavy = Default::default();
        self.heavy_art.clear();
        self.ice = Default::default();
        self.jacks = Default::default();
        self.ice_art.clear();
        self.ice_refund = 0.;
        self.ice_drain = false;
        self.bomb_art.clear();
        self.bombs = bomb::State::default();
        self.blasts.clear();
        self.spatial_sounds.clear();
        self.dice = crate::dice::State::default();
        self.hits.clear();
        self.projectiles.clear();
        self.sparks.clear();
        self.trail.clear();
        self.flight_trails.clear();
    }
    pub fn set_combat_anchors(&mut self, anchors: [Transform; 3], aim: Vec3) {
        self.combat_anchors = anchors;
        self.combat_aim = aim;
    }
    pub fn set_view(&mut self, first_person: bool, scale: f32) {
        if self.first_person != first_person {
            self.trail.clear();
        }
        self.first_person = first_person;
        self.view_scale = scale;
    }
    pub fn update(
        &mut self,
        dt: f32,
        actions: &Actions,
        events: Events,
        anchors: &[Transform; 3],
        eye: Vec3,
        context: &CombatContext<'_>,
    ) -> Vec<&'static str> {
        self.update_funded(dt, actions, events, anchors, eye, context, None)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update_funded(
        &mut self,
        dt: f32,
        actions: &Actions,
        events: Events,
        anchors: &[Transform; 3],
        eye: Vec3,
        context: &CombatContext<'_>,
        mut stats: Option<&mut crate::inventory::Stats>,
    ) -> Vec<&'static str> {
        if dt <= 0. {
            return Vec::new();
        }
        self.eye = eye;
        // Draw the Staff from the visible first-person muzzle while retaining
        // the body-space collision ray and its resolved hit point.
        self.heavy_art.view_muzzle = self.first_person.then(|| {
            self.props[7].point(anchors[0], "tag_barrel", self.view_scale)
        });
        let mut sounds = Vec::new();
        if events.blade_return {
            sounds.push("sound/weapon/shared/weapon_reappear.wav");
        }
        let dice_events = self.dice.advance_clocks(
            dt,
            dt * self
                .world_ratio
                .unwrap_or(if self.time_stopped { 0. } else { 1. }),
            context,
            eye,
            &self.dice_data,
        );
        if dice_events.refund > 0. {
            if let Some(s) = stats.as_deref_mut() {
                s.apply(crate::inventory::PickupKind::Will, dice_events.refund);
            }
        }
        self.hits.extend(dice_events.hits);
        sounds.extend(dice_events.sounds);
        self.spatial_sounds.extend(dice_events.spatial_sounds);
        let mut cursor = 0.;
        let mut timeline: Vec<_> = events
            .emissions
            .into_iter()
            .map(|e| (e.at, Some(e)))
            .chain(events.stop_times.into_iter().map(|at| (at, None)))
            .collect();
        timeline.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (at, emission) in timeline {
            self.step((at - cursor).max(0.), context, stats.as_deref_mut());
            if let Some(emission) = emission {
                self.emit(&emission, anchors, eye, context, &mut sounds);
            } else {
                self.spatial_sounds.extend(self.heavy.release());
            }
            cursor = at;
        }
        // Death/unarming/script interruption cancels a held effect without a shot.
        if actions.selected != 7
            || actions
                .playing
                .as_ref()
                .is_none_or(|p| !matches!(p.action, Action::Attack { .. }))
        {
            self.heavy.charge = None;
        }
        self.step((dt - cursor).max(0.), context, stats);
        if let Some(p) = &actions.playing {
            let (weapon, size) = actions.equipment();
            let swipe = match (weapon, p.alternate()) {
                (0, false) => (3.0..12.01).contains(&p.frame()),
                (2, false) => (3.0..8.0).contains(&p.frame()),
                (2, true) => (10.0..18.0).contains(&p.frame()),
                _ => false,
            };
            if swipe {
                let prop = &self.props[weapon];
                self.trail.push(Trail {
                    a: prop.point(
                        anchors[0],
                        if weapon == 0 {
                            "tag_trail"
                        } else {
                            "tag_swipe"
                        },
                        size * self.view_scale,
                    ),
                    b: prop.point(anchors[0], "tag_barrel", size * self.view_scale),
                    life: 0.2,
                    electric: weapon == 2,
                    color: if weapon == 0 {
                        Color::new(0.8, 0.9, 1., 0.7)
                    } else {
                        Color::new(0.3, 0.55, 1., 0.85)
                    },
                });
                if self.trail.len() > 32 {
                    self.trail.remove(0);
                }
            }
        }
        sounds
    }
    fn emit(
        &mut self,
        emission: &Emission,
        anchors: &[Transform; 3],
        eye: Vec3,
        context: &CombatContext<'_>,
        sounds: &mut Vec<&'static str>,
    ) {
        let combat_anchors = emission.anchors.unwrap_or(self.combat_anchors);
        let weapon = emission.weapon;
        let p = &emission.play;
        let fire = emission.fire;
        let sound = emission.sound;
        let world = context.world;
        if sound && weapon == 4 && p.alternate() {
            self.ice.pulse(
                Transform {
                    translation: self.props[4].point(combat_anchors[0], "tag_barrel", 1.),
                    rotation: Quat::from_rotation_arc(Vec3::X, p.aim.normalize_or_zero()),
                },
                true,
            );
        }
        if sound && weapon != 4 && weapon != 7 {
            sounds.push(
                if weapon == 5 && matches!(p.action, Action::Attack { .. }) {
                    jacks::TOSS
                } else if weapon == 3 {
                    bomb::TOSS
                } else {
                    match p.clip {
                        0 => "sound/character/alice/weapon_switch.wav",
                        1 => "sound/weapon/knife/knife_swing1.wav",
                        2 => "sound/weapon/knife/knife_swing2.wav",
                        3 => "sound/weapon/knife/knife_swing3.wav",
                        4 => "sound/weapon/knife/knife_throw.wav",
                        5 | 6 => "sound/weapon/cards/cards_toss1.wav",
                        7 => "sound/weapon/mallet/mallet_swing1.wav",
                        8 => "sound/weapon/mallet/mallet_swing2.wav",
                        10 => "sound/weapon/dice/dice_toss.wav",
                        11 => "sound/item/watch/use_watch.wav",
                        19 => heavy::BUSS,
                        _ => "sound/weapon/mallet/mallet_ball_swing.wav",
                    }
                },
            );
        }
        if !matches!(p.action, Action::Attack { .. }) {
            return;
        }
        if sound && weapon == 8 {
            self.heavy.ignition = Some((0., combat_anchors[0]));
        }
        if weapon == 9 {
            return;
        }
        if fire {
            self.shots += 1;
            let direction = p.aim.normalize_or_zero();
            if weapon == 7 || weapon == 8 {
                let raw = self.props[weapon].point(combat_anchors[0], "tag_barrel", 1.);
                let (origin, aimed) = launch_from_eye(
                    world,
                    eye,
                    raw,
                    direction,
                    self.first_person,
                    if weapon == 8 { 8. } else { 0. },
                );
                if weapon == 7 {
                    self.heavy.start(
                        p.alternate(),
                        Transform {
                            translation: origin,
                            rotation: Quat::from_rotation_arc(Vec3::X, aimed),
                        },
                    );
                } else {
                    self.heavy.launch(heavy::Kind::Cannon, origin, aimed);
                }
                return;
            }
            if weapon == 4 {
                let raw = self.props[4].point(combat_anchors[0], "tag_barrel", 1.);
                let (origin, aimed) =
                    launch_from_eye(world, eye, raw, direction, self.first_person, 1.);
                if self.owner_immersed {
                    if self.ice.underwater(eye - Vec3::Z * 48., direction) {
                        self.ice_drain = true;
                    }
                    return;
                }
                if p.alternate() {
                    if self.ice.place(context, eye, origin, aimed).is_none() {
                        self.ice_refund += 10.;
                    }
                } else {
                    self.ice.pulse(
                        Transform {
                            translation: origin,
                            rotation: Quat::from_rotation_arc(Vec3::X, aimed),
                        },
                        false,
                    );
                    let (hit, sound) = self.ice.primary(context, origin, aimed);
                    if let Some(hit) = hit {
                        self.hits.push(hit);
                        self.impacts += 1;
                    }
                    if let Some(sound) = sound {
                        self.spatial_sounds.push(sound);
                    }
                }
                return;
            }
            if weapon == 5 {
                let (origin, aimed) = launch_from_eye(
                    world,
                    eye,
                    combat_anchors[0].translation,
                    direction,
                    self.first_person,
                    8.,
                );
                let target = projectile::target(context, eye - Vec3::Z * 20., direction);
                self.jacks.launch(origin, aimed, target, p.alternate());
                return;
            }
            if weapon == 3 {
                let (origin, aimed) = launch_from_eye(
                    world,
                    eye,
                    combat_anchors[0].translation,
                    direction,
                    self.first_person,
                    if p.alternate() { 1. } else { 4. },
                );
                self.bombs
                    .launch(origin, aimed, self.owner_velocity, p.alternate());
                self.spatial_sounds.push((bomb::MUSIC, origin));
                return;
            }
            if weapon == 6 {
                let (origin, aimed) = launch_from_eye(
                    world,
                    eye,
                    anchors[0].translation,
                    direction,
                    self.first_person,
                    8.,
                );
                self.dice.throw(self.dice_count, origin, aimed);
                return;
            }
            if weapon == 1 || p.alternate() {
                let raw = { self.props[weapon].point(combat_anchors[0], "tag_barrel", 1.) };
                // Attachments can reach through a wall; trace from Alice before spawning a visual.
                let (origin, aimed) =
                    launch_from_eye(world, eye, raw, direction, self.first_person, 8.);
                // Converge on a visible enemy under the eye ray, not the wall behind it.
                let aimed = combat::contact(context, eye, eye + direction * 2000., 0.)
                    .and_then(|(_, f)| (eye + direction * 2000. * f - origin).try_normalize())
                    .unwrap_or(aimed);
                let kind = match weapon {
                    0 => projectile::Kind::Blade,
                    1 if p.alternate() => projectile::Kind::Carrier,
                    1 => projectile::Kind::Card,
                    _ => projectile::Kind::Ball,
                };
                let target = (weapon == 1)
                    .then(|| projectile::target(context, eye - Vec3::Z * 20., direction))
                    .flatten();
                let mut projectile =
                    Projectile::new(kind, origin, aimed, target, self.shots as u32);
                if weapon == 2 {
                    projectile.velocity += aimed * self.owner_velocity.dot(aimed).max(0.);
                }
                projectile.audio_id = (0..1000)
                    .find(|id| self.projectiles.iter().all(|p| p.audio_id != *id))
                    .unwrap_or(0);
                self.projectiles.push(projectile);
            } else if weapon == 0 || weapon == 2 {
                let origin = combat_anchors[0].translation;
                let reach = world.sweep(eye, origin, Vec3::ZERO);
                if reach.start_solid || reach.fraction < 1. {
                    return;
                }
                let hits = projectile::melee_toy(context, origin, direction, weapon == 2);
                let reach = if weapon == 2 { 50. } else { 40. };
                if !hits.is_empty() {
                    self.impacts += hits.len() as u64;
                    if weapon == 2 {
                        for (i, hit) in hits.iter().enumerate() {
                            if let Some(target) = context.targets.iter().find(|t| t.id == hit.id) {
                                self.blasts.push(blast::Blast::new(
                                    blast::Kind::ElectricHit,
                                    origin.clamp(
                                        target.center - target.half,
                                        target.center + target.half,
                                    ),
                                    self.impacts as u32 + i as u32,
                                ));
                            }
                        }
                    }
                    self.hits.extend(hits);
                    self.burst(
                        origin + direction * 32.,
                        -direction,
                        Color::new(1., 0.7, 0.3, 1.),
                    );
                    sounds.push(if weapon == 2 {
                        "sound/weapon/mallet/mallet_hit_flesh1.wav"
                    } else {
                        "sound/weapon/knife/knife_hit_flesh1.wav"
                    });
                } else {
                    let wall = world.sweep(origin, origin + direction * reach, Vec3::splat(8.));
                    if !wall.start_solid && wall.fraction < 1. {
                        self.impacts += 1;
                        self.burst(
                            origin + direction * (reach * wall.fraction),
                            wall.normal,
                            WHITE,
                        );
                        sounds.push(if weapon == 2 {
                            "sound/weapon/mallet/mallet_hit_world1.wav"
                        } else {
                            "sound/weapon/knife/knife_hit_world1.wav"
                        });
                    }
                }
            }
        }
    }
    fn burst(&mut self, point: Vec3, normal: Vec3, color: Color) {
        for i in 0..12 {
            let angle = i as f32 * 2.399963;
            let velocity = (vec3(angle.cos(), angle.sin(), (i as f32 * 1.7).sin()) + normal * 0.9)
                * (35. + i as f32 * 4.);
            self.sparks.push(Spark {
                position: point + normal,
                velocity,
                life: 0.28 + i as f32 * 0.018,
                color,
            });
        }
        if self.sparks.len() > 256 {
            self.sparks.drain(..self.sparks.len() - 256);
        }
    }
    fn step(
        &mut self,
        dt: f32,
        context: &CombatContext<'_>,
        stats: Option<&mut crate::inventory::Stats>,
    ) {
        if dt <= 0. {
            return;
        }
        for b in &mut self.blasts {
            b.age += dt;
        }
        self.blasts.retain(|b| b.age < 5.);
        let owner = combat::Target {
            id: crate::dice::ALICE,
            center: self.eye - Vec3::Z * 20.,
            half: vec3(15., 15., 28.),
        };
        let raw = self.props[7].point(self.combat_anchors[0], "tag_barrel", 1.);
        let (origin, aim) = launch_from_eye(
            context.world,
            self.eye,
            raw,
            self.combat_aim,
            self.first_person,
            0.,
        );
        let pose = Transform {
            translation: origin,
            rotation: Quat::from_rotation_arc(Vec3::X, aim),
        };
        let e = self
            .heavy
            .advance(dt, context, owner, pose, self.time_stopped, stats);
        self.hits.extend(e.hits);
        self.spatial_sounds.extend(e.sounds);
        self.impacts += e.impacts;
        self.spatial_sounds.extend(
            self.ice.advance_clocks(
                dt,
                dt * self
                    .world_ratio
                    .unwrap_or(if self.time_stopped { 0. } else { 1. }),
                &self.ice_data,
            ),
        );
        let je = self.jacks.advance(dt, context, owner, self.combat_aim);
        self.hits.extend(je.hits);
        self.spatial_sounds.extend(je.sounds);
        for (position, normal) in je.impacts {
            self.burst(position, normal, SKYBLUE);
            self.impacts += 1;
        }
        let bomb_events = self.bombs.advance(dt, context, owner, &self.bomb_data);
        self.hits.extend(bomb_events.hits);
        self.spatial_sounds.extend(bomb_events.sounds);
        for blast in bomb_events.blasts {
            self.hits.extend(blast.damage(context, owner, None));
            self.blasts.push(blast);
            self.impacts += 1;
        }
        for s in &mut self.sparks {
            s.life -= dt;
            s.position += s.velocity * dt;
            s.velocity.z -= 120. * dt;
        }
        self.sparks.retain(|s| s.life > 0.);
        for t in &mut self.trail {
            t.life -= dt;
        }
        self.trail.retain(|t| t.life > 0.);
        for t in &mut self.flight_trails {
            t.life -= dt;
        }
        self.flight_trails.retain(|t| t.life > 0.);
        for p in &self.projectiles {
            if p.model != 11 {
                self.flight_trails.push(Trail {
                    a: p.position,
                    b: p.position - p.velocity.normalize_or_zero() * 18.,
                    life: 0.18,
                    electric: false,
                    color: if p.model == 11 {
                        SKYBLUE
                    } else {
                        Color::new(0.8, 0.85, 1., 1.)
                    },
                });
            }
        }
        let target = if self
            .projectiles
            .iter()
            .any(|p| p.kind == projectile::Kind::Carrier && p.age < 0.1)
        {
            projectile::target(context, self.eye - Vec3::Z * 20., self.combat_aim)
        } else {
            None
        };
        let impacts = projectile::advance(
            &mut self.projectiles,
            dt,
            self.time_stopped,
            context,
            target,
        );
        if self.flight_trails.len() > 256 {
            self.flight_trails.drain(..self.flight_trails.len() - 256);
        }
        for impact in impacts {
            if impact.blast {
                let blast =
                    blast::Blast::new(blast::Kind::Croquet, impact.position, self.impacts as u32);
                self.hits
                    .extend(blast.damage(context, owner, impact.hit.map(|h| h.id)));
                self.blasts.push(blast);
            }
            if let Some(hit) = impact.hit {
                if impact.model == 11 {
                    self.blasts.push(blast::Blast::new(
                        blast::Kind::ElectricHit,
                        impact.position - impact.normal * 36.,
                        self.impacts as u32,
                    ));
                }
                self.hits.push(hit);
            }
            if !impact.blast || impact.hit.is_some() {
                self.spatial_sounds.push((
                    projectile_impact_sound(impact.model, impact.hit.is_some()),
                    impact.position,
                ));
            }
            self.burst(
                impact.position,
                impact.normal,
                if impact.model == 11 {
                    SKYBLUE
                } else {
                    Color::new(1., 0.8, 0.45, 1.)
                },
            );
            self.impacts += 1;
        }
        if self.blasts.len() > 64 {
            self.blasts.drain(..self.blasts.len() - 64);
        }
    }
    pub fn draw_held(&mut self, actions: &Actions, anchors: &[Transform; 3], fullbright: bool) {
        let (index, scale) = actions.equipment();
        if index == UNARMED {
            return;
        }
        if index == 8
            && actions
                .playing
                .as_ref()
                .is_some_and(|p| p.clip == 19 && p.time >= 0.05 && p.time < 1.10)
        {
            self.heavy_art.cannon(
                anchors[0],
                scale * self.view_scale,
                actions.playing.as_ref().unwrap().time - 0.05,
                fullbright,
            );
        } else if index == 4 && actions.playing.as_ref().is_some_and(|p| p.clip == 12) {
            self.ice_art.wand(
                anchors[0],
                scale * self.view_scale,
                self.ice.clock as f32,
                fullbright,
            );
        } else if index == 9 {
            let time = actions
                .playing
                .as_ref()
                .filter(|p| p.clip == 11)
                .map_or(0., |p| p.time);
            self.props[index].draw_frame(
                anchors[0],
                scale * self.view_scale,
                fullbright,
                time,
                false,
            );
        } else {
            self.props[index].draw(anchors[0], scale * self.view_scale, fullbright);
        }
        if let Some(p) = &actions.playing {
            if p.clip == 9 && (3.0..13.0).contains(&p.frame()) {
                self.props[11].draw(anchors[if p.frame() < 7. { 1 } else { 2 }], 1., fullbright);
            }
            if p.clip == 6 && (3.0..7.0).contains(&p.frame()) {
                self.props[10].draw(anchors[1], 1., fullbright);
            }
        }
        if [0, 3, 5].contains(&index) {
            if let Some(age) = actions.return_age() {
                crate::render::depth_read_only(|| self.draw_return(anchors[0], age));
            }
        }
        if index == 4 {
            self.ice_art.held_mist(
                anchors[0],
                scale * self.view_scale,
                self.ice.clock as f32,
                self.eye,
            );
        }
    }
    fn draw_return(&self, anchor: Transform, age: f32) {
        let forward = (self.eye - anchor.translation)
            .try_normalize()
            .unwrap_or(Vec3::X);
        let right = forward.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::Y);
        let up = right.cross(forward);
        let mut vertices = Vec::new();
        let mut seed = 104729_u32;
        for (burst, (radius, speed)) in [(10., 30.), (13., 40.), (15., 40.), (18., 45.), (20., 50.)]
            .into_iter()
            .enumerate()
        {
            let t = age - burst as f32 * 0.25;
            for _ in 0..40 {
                let mut rand = || {
                    seed = seed.wrapping_mul(214013).wrapping_add(2531011);
                    ((seed >> 16) & 32767) as f32 / 16384. - 1.
                };
                let direction = vec3(rand(), rand(), rand()).normalize_or_zero();
                let offset = vec3(rand(), rand(), rand()) * 7.;
                if !(0. ..0.6).contains(&t) {
                    continue;
                }
                let center =
                    anchor.point((direction * (radius - speed * t) + offset) * self.view_scale);
                let alpha = (t / 0.2).min(1.) * (1. - t / 0.6);
                let color = Color::new(1., 1., 1., alpha).into();
                let corners = [-right - up, right - up, right + up, -right + up];
                let uv = [vec2(0., 1.), vec2(1., 1.), vec2(1., 0.), vec2(0., 0.)];
                for i in [0, 1, 2, 0, 2, 3] {
                    vertices.push(Vertex {
                        position: center + corners[i] * 2. * self.view_scale,
                        uv: uv[i],
                        color,
                        normal: Vec4::ZERO,
                    });
                }
            }
        }
        if !vertices.is_empty() {
            gl_use_material(&self.effect_material);
            draw_mesh(&Mesh {
                indices: (0..vertices.len() as u16).collect(),
                vertices,
                texture: Some(self.return_texture.clone()),
            });
            gl_use_default_material();
        }
    }
    /// Only the local swing ribbon belongs in the weapon pass. World effects stay depth-tested.
    fn draw_electric_trail(&self) {
        let mut vertices = Vec::new();
        append_swing(&mut vertices, &self.trail, true);
        if !vertices.is_empty() {
            let age = self.trail.first().map_or(0., |t| 0.2 - t.life);
            crate::render_fx::skin_effect(
                &Mesh {
                    indices: (0..vertices.len() as u16).collect(),
                    vertices,
                    texture: Some(self.swipe_texture.clone()),
                },
                age,
                1.,
            );
        }
    }
    pub fn draw_view_trail(&self) {
        self.draw_electric_trail();
        gl_use_material(&self.effect_material);
        let mut vertices = Vec::new();
        append_swing(&mut vertices, &self.trail, false);
        if !vertices.is_empty() {
            draw_mesh(&Mesh {
                indices: (0..vertices.len() as u16).collect(),
                vertices,
                texture: Some(self.spark_texture.clone()),
            });
        }
        gl_use_default_material();
    }
    pub fn draw_effects(
        &mut self,
        body_material: &crate::character::SkinMaterial,
        camera: Vec3,
        fullbright: bool,
        show_trail: bool,
    ) {
        body_material.bind();
        self.dice_art.draw(&self.dice, fullbright);
        self.dice_art.effects(&self.dice, camera);
        body_material.bind();
        for d in &self.dice.dice {
            self.props[12].draw(
                Transform {
                    translation: d.position,
                    rotation: Quat::from_rotation_x(d.age * 7.)
                        * Quat::from_rotation_y(d.age * 11.),
                },
                1.,
                fullbright,
            );
        }
        for p in &self.projectiles {
            self.props[p.prop()].draw_frame(p.pose(), 1., fullbright, p.age as f32, true);
        }
        gl_use_material(&self.effect_material);
        let mut vertices = Vec::new();
        let mut triangle = |points: [Vec3; 3], color: Color| {
            for position in points {
                vertices.push(Vertex {
                    position,
                    uv: Vec2::splat(0.5),
                    color: color.into(),
                    normal: Vec4::ZERO,
                });
            }
        };
        for beam in &self.dice.beams {
            let side = (camera - beam.b).cross(beam.b - beam.a).normalize_or_zero()
                * if beam.kind == 1 { 4. } else { 1.8 };
            let color = if beam.kind == 1 {
                Color::new(1., 0.3, 0.06, 0.8)
            } else {
                Color::new(0.55, 0.75, 1., 0.9)
            };
            triangle([beam.a - side, beam.b - side, beam.b + side], color);
            triangle([beam.a - side, beam.b + side, beam.a + side], color);
        }
        for t in &self.flight_trails {
            let right = (camera - t.b).cross(t.b - t.a).normalize_or_zero() * 0.8;
            let c = Color::new(t.color.r, t.color.g, t.color.b, t.life / 0.18 * 0.6);
            triangle([t.a - right, t.b - right, t.b + right], c);
            triangle([t.a - right, t.b + right, t.a + right], c);
        }
        if show_trail {
            self.draw_electric_trail();
            append_swing(&mut vertices, &self.trail, false);
        }
        for s in &self.sparks {
            let right = (camera - s.position).cross(Vec3::Z).normalize_or_zero() * 2.5;
            let up = Vec3::Z * 2.5;
            let proximity = ((camera.distance(s.position) - 8.) / 32.).clamp(0., 1.);
            let c = Color::new(
                s.color.r,
                s.color.g,
                s.color.b,
                (s.life * 3.).min(1.) * proximity,
            );
            let corners = [
                s.position - right - up,
                s.position + right - up,
                s.position + right + up,
                s.position - right + up,
            ];
            let uv = [vec2(0., 0.), vec2(1., 0.), vec2(1., 1.), vec2(0., 1.)];
            for i in [0, 1, 2, 0, 2, 3] {
                vertices.push(Vertex {
                    position: corners[i],
                    uv: uv[i],
                    color: c.into(),
                    normal: Vec4::ZERO,
                });
            }
        }
        if !vertices.is_empty() {
            draw_mesh(&Mesh {
                indices: (0..vertices.len() as u16).collect(),
                vertices,
                texture: Some(self.spark_texture.clone()),
            });
        }
        self.heavy_art.draw(
            &self.heavy,
            camera,
            fullbright,
            self.first_person,
            body_material,
        );
        self.ice_art.draw(
            &self.ice,
            &self.ice_data,
            &self.jacks,
            camera,
            fullbright,
            body_material,
        );
        self.bomb_art.draw(
            (&self.bombs, &self.bomb_data),
            &self.projectiles,
            &self.blasts,
            camera,
            fullbright,
            body_material,
        );
        gl_use_default_material();
    }
}
const EFFECT_FRAGMENT: &str = r#"#version 100
precision mediump float;
uniform sampler2D Texture;
varying highp vec2 uv;
varying lowp vec4 color;
// FOG
void main() { vec4 c=color*texture2D(Texture,uv); gl_FragColor=vec4(fogged(c.rgb,1.0),c.a); }
"#;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    playing: Option<Playing>,
    selected: usize,
    pose: Vec<Transform>,
    start: Vec<Transform>,
    variation: usize,
    #[serde(default)]
    cooldowns: [f32; 10],
    #[serde(default)]
    blade_clock: Option<f32>,
    #[serde(default)]
    jack_clock: Option<[f32; 2]>,
    #[serde(default)]
    jacks_clock: Option<[f32; 2]>,
}
impl Snapshot {
    /// Saves from before the Blade recovery clock only stored the cooldown.
    fn migrated_blade_clock(&self) -> Option<f32> {
        self.blade_clock
            .or_else(|| (self.cooldowns[0] > 0.).then_some(3.5 - self.cooldowns[0]))
    }
    /// The block a restored `Actions` saves again. Legacy blocks are upgraded on load
    /// (never changed by the validation), so a fixture comparison must expect the same.
    pub fn migrated(&self) -> Self {
        Self {
            blade_clock: self.migrated_blade_clock(),
            ..self.clone()
        }
    }
    /// What `Actions::unarm` leaves in a saved block.
    pub fn unarmed(&self, pose: &[Transform]) -> Self {
        Self {
            playing: None,
            selected: UNARMED,
            pose: pose.to_vec(),
            start: pose.to_vec(),
            ..self.clone()
        }
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectileSave {
    #[serde(default)]
    heavy: heavy::State,
    #[serde(default)]
    ice: ice::State,
    #[serde(default)]
    jacks: jacks::State,
    #[serde(default)]
    bombs: bomb::State,
    #[serde(default)]
    blasts: Vec<blast::Blast>,
    #[serde(default)]
    dice: crate::dice::State,
    #[serde(default = "one")]
    dice_count: u8,
    projectiles: Vec<Projectile>,
    shots: u64,
    impacts: u64,
}
impl ProjectileSave {
    /// The bounds `Visuals::restore` enforces before it accepts a saved block.
    pub fn validate(&self) -> Result<()> {
        self.heavy.validate()?;
        self.ice.validate()?;
        self.jacks.validate()?;
        self.dice.validate()?;
        self.bombs.validate()?;
        ensure!(
            self.blasts.len() <= 64 && self.blasts.iter().all(blast::Blast::valid),
            "Invalid saved explosions"
        );
        ensure!(self.dice_count <= 3, "Invalid saved Dice count");
        ensure!(
            self.projectiles.len() <= 256 && self.projectiles.iter().all(Projectile::valid),
            "Invalid saved player projectiles"
        );
        Ok(())
    }
    /// The block a restored `Visuals` saves again: legacy projectiles become their current
    /// kinds (without spawning carriers or replaying a firing debit) and duplicated audio
    /// ids are reassigned. Everything else is kept exactly as saved.
    pub fn migrated(&self) -> Self {
        let mut s = self.clone();
        s.projectiles.iter_mut().for_each(Projectile::migrate);
        let mut ids = std::collections::BTreeSet::new();
        for p in s
            .projectiles
            .iter_mut()
            .filter(|p| p.kind != projectile::Kind::Fragment)
        {
            if ids.contains(&p.audio_id) {
                p.audio_id = (0..1000).find(|id| !ids.contains(id)).unwrap_or(0);
            }
            ids.insert(p.audio_id);
        }
        s
    }
    /// What a dead player's restore leaves of a live summon.
    pub fn dismissed(mut self) -> Self {
        self.dice.dismiss();
        self
    }
}
pub fn check(assets: &mut Assets) -> Result<()> {
    let specs = texture::read_materials(assets)?;
    let mut total = 0;
    for name in WEAPONS
        .iter()
        .map(|(id, _)| format!("w_{id}"))
        .chain(EXTRA_MODELS.iter().map(|s| s.to_string()))
    {
        let (def, model) = read_model(assets, &name)?;
        for s in &model.surfaces {
            let skin = def
                .skins
                .get(&s.name)
                .or_else(|| def.skins.get("all"))
                .context("Missing surface skin")?;
            let path = texture::resolve(assets, &format!("{}/{skin}", def.path), &specs)
                .or_else(|| texture::resolve(assets, skin, &specs))
                .context("Missing prop skin")?;
            texture::decode(assets, &path)?;
            total += s.frames.iter().map(Vec::len).sum::<usize>();
        }
        ensure!(model.frame_time > 0., "Invalid prop timing");
        println!(
            "{name}: {} surfaces, {} tags, scale {}",
            model.surfaces.len(),
            model.tags.len(),
            def.scale
        );
    }
    let alice = Definition::alice(assets)?;
    let skeleton = crate::skeletal::Skeleton::parse(
        &assets.read(&format!("{}/{}", alice.path, alice.model))?,
    )?;
    for tag in ["tag_weapon", "tag_ball_linked", "tag_ball_free"] {
        ensure!(
            skeleton.bones.iter().any(|b| b.name == tag),
            "Missing attachment {tag}"
        );
    }
    for name in ACTION_CLIPS {
        Animation::parse(
            &assets.read(&format!("models/alice/{name}.ska"))?,
            skeleton.bones.len(),
        )?;
    }
    println!(
        "PASS: 17 prop models, {total} decoded vertices, {} action clips and 3 Alice attachments",
        ACTION_CLIPS.len()
    );
    Ok(())
}

/// Uses the supplied original clips and the same funded scheduler as gameplay.
pub fn check_actions(assets: &mut Assets) -> Result<()> {
    let def = Definition::alice(assets)?;
    let skeleton =
        crate::skeletal::Skeleton::parse(&assets.read(&format!("{}/{}", def.path, def.model))?)?;
    let base = vec![
        Transform {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY
        };
        skeleton.bones.len()
    ];
    let upper = vec![true; base.len()];
    for hz in [30, 60, 144] {
        for (weapon, alt, expected, will) in [
            (1, false, 16, 52.),
            (1, true, 5, 0.),
            (2, true, 3, 76.),
            (9, false, 1, 99.),
        ] {
            let mut a = Actions::load(assets, base.len(), &base)?;
            a.selected = weapon;
            let mut stats = crate::inventory::Stats::weapon_preview();
            stats.select(weapon);
            let mut count = 0;
            for _ in 0..hz * 4 {
                let e = a.update_funded(
                    1. / hz as f32,
                    WeaponInput {
                        selected: weapon,
                        click: Some(alt),
                        aim: Vec3::X,
                        ..Default::default()
                    },
                    &base,
                    &upper,
                    true,
                    Some(&mut stats),
                );
                count += e
                    .emissions
                    .iter()
                    .filter(|e| {
                        e.fire
                            && matches!(e.play.action, Action::Attack { .. })
                            && e.weapon == weapon
                    })
                    .count();
                a.finish_frame();
            }
            ensure!(
                count == expected && (stats.will() - will).abs() < 0.001,
                "Weapon input mismatch toy={weapon} alt={alt} Hz={hz}: {count} / {}",
                stats.will()
            );
        }
        println!("PASS real weapon clips at {hz} Hz: held Cards/Mallet, alternate costs, Watch release and no duplicate commits");
    }
    Ok(())
}

/// Window-free `Actions` with the real clip lengths (frame counts of the shipped action clips)
/// and a synthetic rig of any bone count, for restore and fixture tests.
#[cfg(test)]
pub(crate) fn test_actions_with_bones(bones: usize) -> Actions {
    let base = vec![
        Transform {
            rotation: Quat::IDENTITY,
            translation: Vec3::ZERO
        };
        bones
    ];
    let clips = (0..ACTION_CLIPS.len())
        .map(|i| Animation {
            frames: (0..[
                9, 21, 21, 21, 21, 5, 17, 16, 16, 27, 18, 43, 1, 32, 5, 18, 12, 5, 16, 73,
            ][i])
                .map(|_| crate::skeletal::Frame {
                    delta: Vec3::ZERO,
                    pose: vec![
                        Transform {
                            translation: Vec3::X * 10.,
                            ..base[0]
                        };
                        bones
                    ],
                    min: Vec3::ZERO,
                    max: Vec3::ONE * 10.,
                })
                .collect(),
            frame_time: 0.05,
            distance: 0.,
        })
        .collect();
    Actions {
        clips,
        playing: None,
        selected: 0,
        pose: base.clone(),
        start: base.clone(),
        variation: 0,
        cooldowns: [0.; 10],
        blade_clock: None,
        jack_clock: None,
        jacks_clock: None,
        return_pending: false,
        time_stopped: false,
        available: std::array::from_fn(supported),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn funded(
        a: &mut Actions,
        s: &mut crate::inventory::Stats,
        dt: f32,
        selected: usize,
        held: Option<bool>,
    ) -> Events {
        let base = a.pose.clone();
        a.update_funded(
            dt,
            WeaponInput {
                dice: 1,
                selected,
                click: held,
                aim: Vec3::X,
                first_person: false,
            },
            &base,
            &[true; 2],
            true,
            Some(s),
        )
    }
    fn shots(e: &Events) -> usize {
        e.emissions
            .iter()
            .filter(|e| e.fire && matches!(e.play.action, Action::Attack { .. }))
            .count()
    }
    #[test]
    fn switching_all_toys_preserves_watch_and_never_reactivates_or_recharges_it() {
        let mut a = test_actions();
        let mut s = crate::inventory::Stats::weapon_preview();
        s.difficulty = crate::powerups::Difficulty::Hard;
        s.watch().unwrap();
        for weapon in 0..10 {
            s.select(weapon);
            for _ in 0..20 {
                s.update(0.05);
                let e = funded(&mut a, &mut s, 0.05, weapon, None);
                assert_eq!(shots(&e), 0);
                a.finish_frame();
            }
            let saved = a.snapshot();
            a.restore(&saved).unwrap();
            s = serde_json::from_value(serde_json::to_value(&s).unwrap()).unwrap();
            assert_eq!(s.will(), 99.);
            assert!((s.powers.stopped - (19. - weapon as f32)).abs() < 0.002);
            assert!((s.powers.recharge - (359. - weapon as f32)).abs() < 0.01);
        }
        assert!(s.watch().is_err());
    }
    #[test]
    fn blade_reappearance_is_delayed_saved_once_and_continues_during_watch() {
        for hz in [30, 60, 144] {
            let mut a = test_actions();
            let mut s = crate::inventory::Stats::weapon_preview();
            let mut cue = 0;
            let mut fire = 0;
            for frame in 0..hz * 4 {
                let e = funded(
                    &mut a,
                    &mut s,
                    1. / hz as f32,
                    0,
                    (frame == 0).then_some(true),
                );
                fire += shots(&e);
                if e.blade_return {
                    assert!(((frame + 1) as f32 / hz as f32 - 3.4).abs() < 0.04);
                    cue += 1;
                }
                if frame == hz * 2 || frame == hz * 7 / 2 {
                    let saved = a.snapshot();
                    a.restore(&saved).unwrap();
                }
                a.time_stopped = frame >= hz * 2;
                a.finish_frame();
            }
            assert_eq!((fire, cue), (1, 1));
            assert_eq!(a.equipment(), (0, 1.));
            assert!(a.ready_to_attack(0));
            assert_eq!(s.will(), 100.);
        }
    }
    #[test]
    fn swing_uses_aim_at_contact_and_reports_subframe_release_offset() {
        let mut a = test_actions();
        let mut s = crate::inventory::Stats::weapon_preview();
        funded(&mut a, &mut s, 0.3, 0, Some(false));
        let base = a.pose.clone();
        let e = a.update_funded(
            0.1,
            WeaponInput {
                selected: 0,
                aim: Vec3::Y,
                ..Default::default()
            },
            &base,
            &[true; 2],
            true,
            Some(&mut s),
        );
        let shot = e.emissions.iter().find(|e| e.fire).unwrap();
        assert_eq!(shot.play.aim, Vec3::Y);
        assert!((shot.at - 0.05).abs() < 0.00001);
        assert!((shot.play.time - 0.35).abs() < 0.00001);
        assert_eq!(shot.pose.len(), 2);
    }
    #[test]
    fn held_cards_repeat_without_render_frame_gaps_or_duplicate_debits() {
        for hz in [30, 60, 144] {
            let mut a = test_actions();
            a.selected = 1;
            let mut s = crate::inventory::Stats::weapon_preview();
            s.select(1);
            let mut count = 0;
            for _ in 0..hz * 4 {
                count += shots(&funded(&mut a, &mut s, 1. / hz as f32, 1, Some(false)));
                a.finish_frame();
            }
            assert_eq!(count, 16, "{hz} Hz");
            assert_eq!(s.will(), 52., "{hz} Hz");
        }
    }
    #[test]
    fn held_cards_do_not_interrupt_easy_will_recovery() {
        for hz in [30, 60, 144] {
            let mut a = test_actions();
            a.selected = 1;
            let mut s = crate::inventory::Stats::weapon_preview();
            s.difficulty = crate::powerups::Difficulty::Easy;
            s.select(1);
            s.spend_will(10.);
            let mut count = 0;
            for _ in 0..hz * 4 {
                s.update(1. / hz as f32);
                count += shots(&funded(&mut a, &mut s, 1. / hz as f32, 1, Some(false)));
                a.finish_frame();
            }
            assert_eq!(count, 16, "{hz} Hz");
            // 90 starting Will - 16 cards * 3 + four seconds of recovery.
            assert!((s.will() - 43.2).abs() < 0.004, "{hz} Hz: {}", s.will());
        }
    }
    #[test]
    fn tap_completes_attack_release_does_not_queue_extra_shots() {
        let mut a = test_actions();
        a.selected = 2;
        let mut s = crate::inventory::Stats::weapon_preview();
        assert_eq!(shots(&funded(&mut a, &mut s, 0.01, 2, Some(true))), 0);
        assert_eq!(s.will(), 100.);
        let mut count = 0;
        for _ in 0..240 {
            count += shots(&funded(&mut a, &mut s, 1. / 120., 2, None));
            a.finish_frame();
        }
        assert_eq!(count, 1);
        assert_eq!(s.will(), 92.);
    }
    #[test]
    fn switching_queues_behind_committed_animation_and_does_not_rebill() {
        let mut a = test_actions();
        let base = a.pose.clone();
        let mut s = crate::inventory::Stats::weapon_preview();
        funded(&mut a, &mut s, 0.10, 0, Some(false));
        funded(&mut a, &mut s, 0.10, 1, None);
        assert_eq!(a.selected, 0);
        let e = funded(&mut a, &mut s, 0.20, 1, None);
        assert_eq!(shots(&e), 1);
        assert_eq!(e.emissions.last().unwrap().weapon, 0);
        assert_eq!(s.will(), 100.);
        for _ in 0..140 {
            funded(&mut a, &mut s, 1. / 120., 1, None);
            a.finish_frame();
        }
        assert_eq!(a.selected, 1);
        assert!(a.playing.is_none());
        let e = a.update_funded(
            0.01,
            WeaponInput {
                selected: 1,
                click: Some(false),
                aim: Vec3::X,
                ..Default::default()
            },
            &base,
            &[false, true],
            false,
            Some(&mut s),
        );
        assert_eq!(shots(&e), 1);
        assert_eq!(s.will(), 97.);
        assert_eq!(a.pose[0].translation, base[0].translation);
    }
    #[test]
    fn cancelling_before_release_costs_nothing_after_release_keeps_one_cost() {
        for (time, expected) in [(0.2, 100.), (0.7, 92.)] {
            let mut a = test_actions();
            a.selected = 2;
            let base = a.pose.clone();
            let mut s = crate::inventory::Stats::weapon_preview();
            funded(&mut a, &mut s, time, 2, Some(true));
            a.reset(&base);
            assert_eq!(shots(&funded(&mut a, &mut s, 1., 2, None)), 0);
            assert_eq!(s.will(), expected);
        }
    }
    #[test]
    fn pause_and_pre_post_release_save_restore_never_duplicate_charge_or_shot() {
        for before in [0.20, 0.70] {
            let mut a = test_actions();
            a.selected = 2;
            let mut s = crate::inventory::Stats::weapon_preview();
            let mut count = shots(&funded(&mut a, &mut s, before, 2, Some(true)));
            let state = serde_json::to_vec(&a.snapshot()).unwrap();
            let saved_will = s.will();
            assert_eq!(shots(&funded(&mut a, &mut s, 0., 2, Some(true))), 0);
            assert_eq!(serde_json::to_vec(&a.snapshot()).unwrap(), state);
            a.restore(&serde_json::from_slice(&state).unwrap()).unwrap();
            assert_eq!(s.will(), saved_will);
            count += shots(&funded(&mut a, &mut s, 0.8, 2, None));
            assert_eq!(count, 1);
            assert_eq!(s.will(), 92.);
            assert_eq!(shots(&funded(&mut a, &mut s, 0.3, 2, None)), 0);
        }
    }
    #[test]
    fn legacy_prepaid_save_releases_once_without_second_debit() {
        let mut a = test_actions();
        a.selected = 2;
        let mut s = crate::inventory::Stats::weapon_preview();
        funded(&mut a, &mut s, 0.2, 2, Some(true));
        s.spend_will(6.); // v11 prototype charged six at click.
        let mut legacy = serde_json::to_value(a.snapshot()).unwrap();
        legacy["playing"].as_object_mut().unwrap().remove("paid");
        legacy["playing"].as_object_mut().unwrap().remove("stage");
        legacy.as_object_mut().unwrap().remove("cooldowns");
        a.restore(&serde_json::from_value(legacy).unwrap()).unwrap();
        assert_eq!(shots(&funded(&mut a, &mut s, 0.6, 2, None)), 1);
        assert_eq!(s.will(), 94.);
    }
    #[test]
    fn watch_waits_for_frame_22_and_cannot_restart_on_hold() {
        for alt in [false, true] {
            let mut a = test_actions();
            a.selected = 9;
            let mut s = crate::inventory::Stats::weapon_preview();
            funded(&mut a, &mut s, 1., 9, Some(alt));
            assert_eq!(s.will(), 100.);
            assert_eq!(s.powers.stopped, 0.);
            assert_eq!(shots(&funded(&mut a, &mut s, 0.1, 9, Some(alt))), 1);
            assert_eq!(s.will(), 99.);
            assert_eq!(s.powers.stopped, 20.);
            for _ in 0..20 {
                assert_eq!(shots(&funded(&mut a, &mut s, 0.5, 9, Some(alt))), 0);
            }
            assert_eq!(s.will(), 99.);
        }
    }
    #[test]
    fn unsupported_toys_do_not_spend_and_insufficient_will_selects_owned_knife() {
        for toy in [7, 8] {
            let mut a = test_actions();
            a.selected = toy;
            a.available[toy] = false;
            let mut s = crate::inventory::Stats::weapon_preview();
            assert_eq!(shots(&funded(&mut a, &mut s, 1., toy, Some(false))), 0);
            assert_eq!(s.will(), 100.);
            assert!(a.playing.is_none());
        }
        let mut a = test_actions();
        a.selected = 1;
        let mut s = crate::inventory::Stats::weapon_preview();
        s.select(1);
        s.spend_will(99.);
        let e = funded(&mut a, &mut s, 0.1, 1, Some(true));
        assert_eq!(shots(&e), 0);
        assert_eq!(s.will(), 1.);
        assert_eq!(s.selected(), 0);
        assert!(matches!(
            a.playing.as_ref().unwrap().action,
            Action::Equip(1)
        ));
    }
    #[test]
    fn thrown_weapon_recovery_survives_switching_and_serialization() {
        let mut a = test_actions();
        let mut s = crate::inventory::Stats::weapon_preview();
        assert_eq!(shots(&funded(&mut a, &mut s, 0.5, 0, Some(true))), 1);
        for _ in 0..60 {
            funded(&mut a, &mut s, 1. / 60., 1, None);
            a.finish_frame();
        }
        let saved = a.snapshot();
        a.restore(&saved).unwrap();
        let mut n = 0;
        for _ in 0..60 {
            n += shots(&funded(&mut a, &mut s, 1. / 60., 0, Some(true)));
            a.finish_frame();
        }
        assert_eq!(n, 0);
        assert!(a.cooldowns[0] > 0.);
    }
    #[test]
    fn all_toy_rules_have_single_commits_including_delayed_buss_and_channels() {
        for toy in 0..10 {
            for alt in [false, true] {
                let mut a = test_actions();
                a.selected = toy;
                a.available = [true; 10];
                let mut s = crate::inventory::Stats::weapon_preview();
                let r = crate::weapon_rules::rule(toy, alt, 0);
                let mut count = 0;
                for frame in 0..240 {
                    count += shots(&funded(
                        &mut a,
                        &mut s,
                        1. / 120.,
                        toy,
                        (frame == 0 || toy == 7).then_some(alt),
                    ));
                    a.finish_frame();
                }
                assert_eq!(count, 1, "toy={toy} alt={alt}");
                assert!((s.will() - (100. - r.cost)).abs() < 0.001, "toy={toy}");
                if toy == 7 {
                    assert_eq!(funded(&mut a, &mut s, 0.01, toy, None).stops, 1);
                    assert_eq!(funded(&mut a, &mut s, 0.01, toy, None).stops, 0);
                }
            }
        }
        let mut a = test_actions();
        a.selected = 8;
        a.available[8] = true;
        let mut s = crate::inventory::Stats::weapon_preview();
        assert_eq!(shots(&funded(&mut a, &mut s, 0.05, 8, Some(false))), 0);
        assert_eq!(s.will(), 1.);
        assert_eq!(shots(&funded(&mut a, &mut s, 0.70, 8, None)), 1);
        assert_eq!(s.will(), 1.);
    }
    #[test]
    fn staff_charge_sustain_stop_and_queued_switch_have_one_lifecycle() {
        use crate::weapon_rules::Stage;
        let mut a = test_actions();
        a.selected = 7;
        a.available[7] = true;
        let mut s = crate::inventory::Stats::weapon_preview();
        assert_eq!(shots(&funded(&mut a, &mut s, 0.1, 7, Some(false))), 1);
        for _ in 0..2 {
            assert_eq!(shots(&funded(&mut a, &mut s, 1., 7, Some(false))), 0);
        }
        assert_eq!(a.playing.as_ref().unwrap().stage, Stage::Charge);
        assert_eq!(shots(&funded(&mut a, &mut s, 0.4, 7, Some(false))), 0);
        assert_eq!(a.playing.as_ref().unwrap().stage, Stage::Sustain);
        assert_eq!(shots(&funded(&mut a, &mut s, 0.4, 1, Some(false))), 0);
        assert_eq!(a.selected, 7);
        let save = a.snapshot();
        a.restore(&save).unwrap();
        assert_eq!(funded(&mut a, &mut s, 0.1, 1, None).stops, 1);
        assert_eq!(a.playing.as_ref().unwrap().stage, Stage::End);
        assert_eq!(funded(&mut a, &mut s, 0.9, 1, None).stops, 0);
        assert_eq!(a.selected, 1);
        assert!(matches!(
            a.playing.as_ref().unwrap().action,
            Action::Equip(7)
        ));

        a.reset(&a.pose.clone());
        a.selected = 7;
        funded(&mut a, &mut s, 0.1, 7, Some(false));
        s.spend_will(100.);
        assert_eq!(funded(&mut a, &mut s, 0.1, 7, Some(false)).stops, 1);
        assert_eq!(a.playing.as_ref().unwrap().stage, Stage::End);
    }
    #[test]
    fn unarmed_input_never_attacks_and_first_pickup_equips_from_empty_hands() {
        let mut actions = test_actions();
        let base = actions.pose.clone();
        for dt in [0., 0.1, 0.1] {
            let events = actions.update(
                dt,
                WeaponInput {
                    dice: 0,
                    selected: UNARMED,
                    click: Some(false),
                    aim: Vec3::X,
                    first_person: false,
                },
                &base,
                &[true; 2],
                true,
            );
            assert!(!events.fire && !events.sound);
            assert!(!actions.ready_to_attack(0));
            assert_eq!(actions.equipment().0, UNARMED);
        }
        let saved = actions.snapshot();
        actions.restore(&saved).unwrap();
        actions.update(
            0.05,
            WeaponInput {
                dice: 0,
                selected: 0,
                click: None,
                aim: Vec3::X,
                first_person: true,
            },
            &base,
            &[true; 2],
            true,
        );
        assert_eq!(actions.equipment().0, UNARMED);
        assert!(matches!(
            actions.playing.as_ref().unwrap().action,
            Action::Equip(UNARMED)
        ));
        let saved = actions.snapshot();
        actions.restore(&saved).unwrap();
        for _ in 0..10 {
            actions.update(
                0.05,
                WeaponInput {
                    dice: 0,
                    selected: 0,
                    click: None,
                    aim: Vec3::X,
                    first_person: true,
                },
                &base,
                &[true; 2],
                true,
            );
            actions.finish_frame();
        }
        assert_eq!(actions.equipment().0, 0);
        assert!(actions.ready_to_attack(0));
    }
    fn test_actions() -> Actions {
        super::test_actions_with_bones(2)
    }
    #[test]
    fn dice_buttons_share_original_throw_frame_and_cost_with_no_repeat_or_pause_debit() {
        for alternate in [false, true] {
            for hz in [30, 60, 144] {
                let mut a = test_actions();
                a.selected = 6;
                let base = a.pose.clone();
                let mut stats = crate::inventory::Stats::weapon_preview();
                stats.select(6);
                assert!(a.ready_to_attack(6));
                assert!(stats.spend_will(combat::will_cost(6, alternate)));
                let mut fires = 0;
                for tick in 0..hz * 2 {
                    let before = a.playing.as_ref().map(|p| p.time);
                    let paused = a.update(
                        0.,
                        WeaponInput {
                            dice: 1,
                            selected: 6,
                            click: Some(alternate),
                            aim: Vec3::X,
                            first_person: true,
                        },
                        &base,
                        &[true; 2],
                        true,
                    );
                    assert!(!paused.fire);
                    assert_eq!(before, a.playing.as_ref().map(|p| p.time));
                    let e = a.update(
                        1. / hz as f32,
                        WeaponInput {
                            dice: 1,
                            selected: 6,
                            click: (tick == 0).then_some(alternate),
                            aim: Vec3::X,
                            first_person: true,
                        },
                        &base,
                        &[true; 2],
                        true,
                    );
                    fires += usize::from(e.fire);
                    if e.fire {
                        assert_eq!(a.playing.as_ref().unwrap().clip, 10);
                        assert_eq!(a.equipment().1, 0.);
                    }
                    a.finish_frame();
                }
                assert_eq!(fires, 1);
                assert_eq!(stats.will(), 60.);
                stats.spend_will(21.);
                assert!(!stats.spend_will(combat::will_cost(6, alternate)));
                assert_eq!(stats.will(), 39.);
            }
        }
    }

    #[test]
    fn first_person_pose_pauses_and_throw_visibility_follows_release() {
        let mut actions = test_actions();
        let base = actions.pose.clone();
        let mut view = crate::viewmodel::ViewModel::default();
        let camera = crate::viewmodel::camera_frame(Vec3::ZERO, Vec3::X);
        let idle = view.pose(0., 0., &actions, camera);
        let input = || WeaponInput {
            dice: 1,
            first_person: true,
            selected: 0,
            click: Some(true),
            aim: Vec3::X,
        };
        actions.update(0.2, input(), &base, &[true; 2], true);
        let moved = view.pose(0.2, 150., &actions, camera);
        assert!(
            moved.anchors[0]
                .translation
                .distance(idle.anchors[0].translation)
                > 1.
        );
        let paused = view.pose(0., 0., &actions, camera);
        assert_eq!(paused.anchors[0].translation, moved.anchors[0].translation);
        assert_eq!(paused.anchors[0].rotation, moved.anchors[0].rotation);
        assert!(actions.equipment().1 > 0.);
        let events = actions.update(0.25, input(), &base, &[true; 2], true);
        assert!(events.fire);
        assert_eq!(actions.equipment().1, 0.);
        actions.reset(&base);
        view.reset();
        let reset = view.pose(0., 0., &actions, camera);
        assert_eq!(reset.anchors[0].translation, idle.anchors[0].translation);
        assert_eq!(actions.equipment().1, 0.); // Cancelling a pose cannot recall a thrown knife.
        for _ in 0..4 {
            actions.update(
                1.,
                WeaponInput {
                    selected: 0,
                    aim: Vec3::X,
                    ..Default::default()
                },
                &base,
                &[true; 2],
                true,
            );
        }
        assert_eq!(actions.equipment().1, 1.);
    }
    #[test]
    fn first_person_projectiles_aim_at_eye_ray_without_spawning_beyond_a_wall() {
        for distance in [10., 100.] {
            let world = World::fixture(&[(
                vec3(distance, -100., -100.),
                vec3(distance + 2., 100., 100.),
            )]);
            let (origin, direction) =
                launch_from_eye(&world, Vec3::ZERO, vec3(25., -10., -8.), Vec3::X, true, 1.);
            assert!(origin.x < distance);
            assert!(direction.is_finite() && (direction.length() - 1.).abs() < 0.0001);
            let hit = world.sweep(origin, origin + direction * 200., Vec3::splat(1.));
            assert!(
                !hit.start_solid && hit.fraction < 1.,
                "distance={distance}, origin={origin:?}, direction={direction:?}, hit={hit:?}"
            );
            let target =
                origin + direction * ((distance - crate::collision::SKIN - origin.x) / direction.x);
            assert!(target.y.abs() < 0.001 && target.z.abs() < 0.001);
        }
    }

    #[test]
    fn fast_visual_projectiles_hit_thin_walls_and_balls_survive_repeated_bounces() {
        let world = World::fixture(&[(vec3(10., -100., -100.), vec3(12., 100., 100.))]);
        let mut p = Projectile::new(projectile::Kind::Card, Vec3::ZERO, Vec3::X, None, 1);
        assert!(p.advance(0., &world).is_none());
        assert_eq!(p.age, 0.);
        assert!(p.advance(0.05, &world).is_some());
        assert!(p.position.x < 10. && p.age >= 2.5);
        p = Projectile::new(projectile::Kind::Ball, Vec3::ZERO, Vec3::X, None, 1);
        for bounce in 1..=4 {
            p.position = Vec3::ZERO;
            p.velocity = Vec3::X * 1100.;
            assert!(p.advance(0.05, &world).is_some());
            assert!((p.velocity.x + 935.).abs() < 0.01);
            assert_eq!(p.bounces, bounce);
            assert!(p.alive());
        }
        assert!((p.age - 0.2).abs() < 0.00001);
    }
    #[test]
    fn events_cross_frames_once_and_pause_stops_them() {
        for rate in [30, 60, 144] {
            let mut p = Playing {
                action: Action::Attack { alternate: true },
                clip: 9,
                time: 0.,
                duration: 1.35,
                frame_time: 0.05,
                aim: Vec3::X,
                fired: false,
                sounded: false,
                paid: false,
                stage: Default::default(),
            };
            let (mut fires, mut sounds) = (0, 0);
            for _ in 0..rate * 2 {
                let e = p.advance(1. / rate as f32);
                fires += usize::from(e.fire);
                sounds += usize::from(e.sound);
                assert!(!p.advance(0.).fire);
            }
            assert_eq!((fires, sounds), (1, 1));
        }
        let mut p = Playing {
            action: Action::Attack { alternate: false },
            clip: 5,
            time: 0.,
            duration: 0.25,
            frame_time: 0.05,
            aim: Vec3::X,
            fired: false,
            sounded: false,
            paid: false,
            stage: Default::default(),
        };
        assert!(!p.advance(0.).fire);
        assert!(p.advance(0.001).fire);
        assert!(!p.advance(1.).fire);
    }

    fn saved_json(v: &impl serde::Serialize) -> serde_json::Value {
        serde_json::to_value(v).unwrap()
    }
    #[test]
    fn legacy_blade_cooldown_restores_into_the_recovery_clock_the_migrated_block_predicts() {
        let mut a = test_actions_with_bones(4);
        let mut old = saved_json(&a.snapshot());
        old["cooldowns"][0] = 2.0.into();
        old.as_object_mut().unwrap().remove("blade_clock");
        let old: Snapshot = serde_json::from_value(old).unwrap();
        assert!(old.blade_clock.is_none());
        a.restore(&old).unwrap();
        assert_eq!(a.blade_clock, Some(1.5));
        assert_eq!(saved_json(&a.snapshot()), saved_json(&old.migrated()));
        // A block that already carries the clock, or has no cooldown, is not rewritten.
        for mut modern in [a.snapshot(), test_actions_with_bones(4).snapshot()] {
            modern = modern.migrated();
            let mut b = test_actions_with_bones(4);
            b.restore(&modern).unwrap();
            assert_eq!(saved_json(&b.snapshot()), saved_json(&modern));
        }
    }
    #[test]
    fn every_action_the_first_action_system_could_save_still_restores_unchanged() {
        // The first action system (before the Stage and paid fields, cooldown array and
        // recovery clocks) chose these clips and saved the clip's own duration.
        let old_clips = |weapon: usize, alternate: bool| -> Vec<usize> {
            match (weapon, alternate) {
                (0, false) => vec![1, 2, 3],
                (0, true) => vec![4],
                (1, false) => vec![5],
                (1, true) => vec![6],
                (2, false) => vec![7, 8],
                (2, true) => vec![9],
                (6, _) => vec![10],
                (9, _) => vec![11],
                _ => vec![],
            }
        };
        let bones = 4;
        let source = test_actions_with_bones(bones);
        let pose = saved_json(&source.pose);
        let mut restored = 0;
        for weapon in 0..10 {
            for alternate in [false, true] {
                for clip in old_clips(weapon, alternate) {
                    let duration = source.clips[clip].duration();
                    for time in [0., duration * 0.4, duration] {
                        let old: Snapshot = serde_json::from_value(serde_json::json!({
                            "playing": {
                                "action": {"Attack": {"alternate": alternate}},
                                "clip": clip, "time": time, "duration": duration,
                                "frame_time": 0.05, "aim": [1., 0., 0.],
                                "fired": time > duration * 0.5, "sounded": false
                            },
                            "selected": weapon, "pose": pose, "start": pose, "variation": 1
                        }))
                        .unwrap();
                        let mut a = test_actions_with_bones(bones);
                        a.restore(&old)
                            .unwrap_or_else(|e| panic!("toy {weapon} clip {clip}: {e}"));
                        assert_eq!(saved_json(&a.snapshot()), saved_json(&old.migrated()));
                        restored += 1;
                    }
                }
            }
        }
        for weapon in 0..10 {
            let old: Snapshot = serde_json::from_value(serde_json::json!({
                "playing": {
                    "action": {"Equip": (weapon + 3) % 10}, "clip": 0, "time": 0.1,
                    "duration": source.clips[0].duration(), "frame_time": 0.05,
                    "aim": [1., 0., 0.], "fired": false, "sounded": false
                },
                "selected": weapon, "pose": pose, "start": pose, "variation": 0
            }))
            .unwrap();
            let mut a = test_actions_with_bones(bones);
            a.restore(&old).unwrap();
            assert_eq!(saved_json(&a.snapshot()), saved_json(&old.migrated()));
            restored += 1;
        }
        assert_eq!(restored, 3 * (3 + 1 + 1 + 1 + 2 + 1 + 2 + 2) + 10);
    }
    #[test]
    fn unarmed_prediction_matches_what_unarm_leaves_in_a_saved_block() {
        let pose = vec![
            Transform {
                rotation: Quat::IDENTITY,
                translation: Vec3::Y,
            };
            4
        ];
        let mut a = test_actions_with_bones(4);
        a.selected = 2;
        let armed = a.snapshot();
        a.unarm(&pose);
        assert_eq!(saved_json(&a.snapshot()), saved_json(&armed.unarmed(&pose)));
    }
    #[test]
    fn legacy_projectiles_migrate_once_and_the_migrated_block_is_a_fixed_point() {
        let legacy = |model: usize, audio: usize| {
            serde_json::json!({
                "model": model, "position": [1., 2., 3.], "velocity": [500., 0., 0.],
                "age": 0.5, "bounces": 0, "audio_id": audio
            })
        };
        let save: ProjectileSave = serde_json::from_value(serde_json::json!({
            "projectiles": [legacy(0, 3), legacy(10, 3), legacy(11, 3)],
            "shots": 3, "impacts": 1
        }))
        .unwrap();
        save.validate().unwrap();
        let migrated = save.migrated();
        let kinds: Vec<_> = migrated.projectiles.iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            [
                projectile::Kind::Blade,
                projectile::Kind::Card,
                projectile::Kind::Ball
            ]
        );
        let ids: std::collections::BTreeSet<_> =
            migrated.projectiles.iter().map(|p| p.audio_id).collect();
        assert_eq!(ids.len(), 3, "duplicated audio ids are reassigned");
        migrated.validate().unwrap();
        assert_eq!(saved_json(&migrated.migrated()), saved_json(&migrated));
        assert_eq!((migrated.shots, migrated.impacts), (3, 1));
        // The defaults an old block deserializes to are already current-format state.
        let empty: ProjectileSave = serde_json::from_value(
            serde_json::json!({"projectiles": [], "shots": 0, "impacts": 0}),
        )
        .unwrap();
        empty.validate().unwrap();
        assert_eq!(saved_json(&empty.migrated()), saved_json(&empty));
    }
}

//! Declarative TIKI animation events. No script commands are executed.
use crate::{assets::Assets, bsp::tokens};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum When {
    Entry,
    Frame(u32),
    Last,
    Exit,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Sound {
        path: String,
        volume: f32,
    },
    Attach {
        model: String,
        tag: String,
        scale: f32,
        lifetime: Option<f32>,
    },
    Remove(String),
    HideWeapon(bool),
    Surface {
        name: String,
        hidden: bool,
    },
    Emitter {
        name: String,
        enabled: bool,
    },
    Gib {
        offset: [f32; 3],
        cap: String,
        animation: String,
        surfaces: String,
    },
}
#[derive(Clone, Debug)]
pub struct Event {
    pub when: When,
    pub command: Command,
}
#[derive(Default)]
pub struct Model {
    pub clips: BTreeMap<String, Vec<Event>>,
}
#[derive(Clone, Copy)]
pub struct Span {
    pub start: f32,
    pub end: f32,
    pub duration: f32,
    pub frame_time: f32,
    pub looping: bool,
    pub entered: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    pub model: String,
    pub scale: f32,
    pub age: f32,
}
#[derive(Default, Debug, PartialEq)]
pub struct VisualState {
    pub attachments: BTreeMap<String, Attachment>,
    pub surfaces: Vec<(String, bool)>,
    pub removed: BTreeSet<String>,
    pub emitters: BTreeMap<String, bool>,
    pub hide_weapon: bool,
}
pub fn matches(pattern: &str, name: &str) -> bool {
    pattern == "all"
        || pattern
            .strip_suffix('*')
            .map_or(pattern == name, |p| name.starts_with(p))
}
fn trigger(s: &str) -> bool {
    matches!(s, "entry" | "enter" | "first" | "last" | "exit") || s.parse::<u32>().is_ok()
}
fn command_name(s: &str) -> bool {
    matches!(
        s,
        "sound"
            | "attachmodel"
            | "removeattachedmodel"
            | "hideweapon"
            | "surface"
            | "emitteron"
            | "emitteroff"
            | "spawngib"
    )
}
impl When {
    fn time(self, duration: f32, frame_time: f32) -> f32 {
        match self {
            Self::Entry => 0.,
            Self::Frame(n) => n as f32 * frame_time,
            Self::Last => (duration - frame_time).max(0.),
            Self::Exit => duration,
        }
    }
}
impl Model {
    pub fn load(assets: &mut Assets, path: &str) -> Result<Self> {
        Self::parse(&String::from_utf8_lossy(&assets.read(path)?))
    }
    pub fn parse(text: &str) -> Result<Self> {
        let t = tokens(text)?;
        let mut defines = BTreeMap::new();
        for w in t.windows(3).filter(|w| w[0] == "$define") {
            defines.insert(w[1].clone(), w[2].clone());
        }
        let expand = |s: &str| {
            let mut s = s.replace('\\', "/");
            for (key, value) in &defines {
                s = s.replace(&format!("${key}$"), value);
            }
            s
        };
        let mut out = Self::default();
        let (mut depth, mut animations, mut server, mut client) = (0, false, false, false);
        let mut clip = String::new();
        for (i, word) in t.iter().enumerate() {
            match word.as_str() {
                "{" => depth += 1,
                "}" => {
                    depth -= 1;
                    if depth < 3 {
                        server = false;
                        client = false;
                    }
                    if depth == 0 {
                        animations = false;
                    }
                }
                "animations" if depth == 0 => animations = true,
                _ if animations
                    && depth == 1
                    && t.get(i + 1)
                        .is_some_and(|s| s.ends_with(".ska") || s.ends_with(".tan")) =>
                {
                    clip = word.clone();
                }
                "server" if animations && depth == 2 => {
                    server = true;
                    client = false;
                }
                "client" if animations && depth == 2 => {
                    client = true;
                    server = false;
                }
                _ if animations && depth == 3 && (server || client) => {
                    let when = match word.as_str() {
                        "entry" | "enter" => When::Entry,
                        "first" => When::Frame(0),
                        "last" => When::Last,
                        "exit" => When::Exit,
                        _ => {
                            let Ok(n) = word.parse() else { continue };
                            When::Frame(n)
                        }
                    };
                    // Commands have optional arguments. Stop at the next event, even
                    // when an exporter puts several commands on one line.
                    let end = (i + 2..t.len())
                        .find(|&j| {
                            t[j] == "}"
                                || (trigger(&t[j]) && t.get(j + 1).is_some_and(|v| command_name(v)))
                        })
                        .unwrap_or(t.len());
                    let get = |n: usize| if i + n < end { t[i + n].as_str() } else { "" };
                    let number = |n, default| {
                        get(n)
                            .parse::<f32>()
                            .ok()
                            .filter(|n| n.is_finite())
                            .unwrap_or(default)
                    };
                    let command = match get(1) {
                        "sound" if client && expand(get(2)).starts_with("sound/") => {
                            Command::Sound {
                                path: expand(get(2)).to_lowercase(),
                                volume: number(4, 1.).clamp(0., 1.),
                            }
                        }
                        "attachmodel" if server && get(2).ends_with(".tik") => Command::Attach {
                            model: get(2)
                                .trim_start_matches("models/")
                                .trim_end_matches(".tik")
                                .into(),
                            tag: get(3).into(),
                            scale: number(4, 1.).clamp(0.01, 10.),
                            lifetime: (number(7, 0.) > 0.).then(|| number(7, 0.).min(60.)),
                        },
                        "removeattachedmodel" if server => Command::Remove(get(2).into()),
                        "hideweapon" if server && matches!(get(2), "0" | "1") => {
                            Command::HideWeapon(get(2) == "1")
                        }
                        "surface" if server && matches!(get(3), "+nodraw" | "-nodraw") => {
                            Command::Surface {
                                name: get(2).to_lowercase(),
                                hidden: get(3) == "+nodraw",
                            }
                        }
                        "emitteron" | "emitteroff" if client => Command::Emitter {
                            name: get(2).into(),
                            enabled: get(1) == "emitteron",
                        },
                        "spawngib" if server => {
                            let Some(offset) = crate::interaction::vector(get(2)) else {
                                continue;
                            };
                            Command::Gib {
                                offset: offset.to_array(),
                                cap: get(5).into(),
                                animation: get(6).into(),
                                surfaces: get(7).into(),
                            }
                        }
                        _ => continue,
                    };
                    out.clips
                        .entry(clip.clone())
                        .or_default()
                        .push(Event { when, command });
                }
                _ => (),
            }
        }
        Ok(out)
    }
    /// Chronological (start, end] crossings, including frame zero on a fresh play.
    /// Catch-up is bounded relative to the current cycle, even after hours of looping.
    pub fn between(&self, name: &str, span: Span) -> Vec<&Command> {
        if ![span.start, span.end, span.duration, span.frame_time]
            .iter()
            .all(|x| x.is_finite())
            || span.start < 0.
            || span.end <= span.start
            || span.duration <= 0.
            || span.frame_time <= 0.
        {
            return vec![];
        }
        let first = if span.looping {
            (span.start / span.duration).floor() as u64
        } else {
            0
        };
        let last = if span.looping {
            ((span.end / span.duration).floor() as u64).min(first.saturating_add(32))
        } else {
            0
        };
        let mut events = Vec::new();
        for cycle in first..=last {
            for (order, e) in self.clips.get(name).into_iter().flatten().enumerate() {
                if (e.when == When::Entry && (!span.entered || cycle != 0))
                    || (e.when == When::Exit && span.looping)
                {
                    continue;
                }
                let t = e.when.time(span.duration, span.frame_time);
                if t > span.duration || (t == span.duration && e.when != When::Exit) {
                    continue;
                }
                let at = cycle as f32 * span.duration + t;
                if (at > span.start && at <= span.end) || (span.entered && cycle == 0 && t == 0.) {
                    events.push((at, order, &e.command));
                }
            }
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        events.into_iter().map(|(_, _, c)| c).collect()
    }
    /// Reconstruct persistent presentation without replaying sounds or one-shot effects.
    pub fn visual(
        &self,
        name: &str,
        time: f32,
        duration: f32,
        frame_time: f32,
        looping: bool,
    ) -> VisualState {
        let mut out = VisualState::default();
        if !time.is_finite() || duration <= 0. {
            return out;
        }
        let time = time.max(0.);
        let cycle = if looping {
            (time / duration).floor() as u64
        } else {
            0
        };
        // One complete preceding cycle is enough for declarative assignments;
        // retain cycle zero separately because entry events only run once.
        let mut cycles = vec![0];
        if cycle > 1 {
            cycles.push(cycle - 1);
        }
        if cycle > 0 {
            cycles.push(cycle);
        }
        let mut events = Vec::new();
        for n in cycles {
            for e in self.clips.get(name).into_iter().flatten() {
                if (looping && e.when == When::Exit) || (n > 0 && e.when == When::Entry) {
                    continue;
                }
                let local = e.when.time(duration, frame_time);
                if local > duration || (local == duration && e.when != When::Exit) {
                    continue;
                }
                events.push((n as f32 * duration + local, e));
            }
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut expiry = BTreeMap::new();
        for (at, e) in events {
            if at > time {
                continue;
            }
            match &e.command {
                Command::Attach {
                    model,
                    tag,
                    scale,
                    lifetime,
                } => {
                    out.removed.remove(tag);
                    out.removed.remove(model);
                    out.attachments.insert(
                        tag.clone(),
                        Attachment {
                            model: model.clone(),
                            scale: *scale,
                            age: time - at,
                        },
                    );
                    expiry.insert(tag.clone(), lifetime.is_some_and(|life| time - at >= life));
                }
                Command::Remove(name) => {
                    out.removed.insert(name.clone());
                    let model = name.trim_start_matches("models/").trim_end_matches(".tik");
                    out.attachments
                        .retain(|tag, a| tag != name && a.model != model);
                }
                Command::HideWeapon(value) => out.hide_weapon = *value,
                Command::Surface { name, hidden } => {
                    out.surfaces.push((name.clone(), *hidden));
                }
                Command::Emitter { name, enabled } => {
                    out.emitters.insert(name.clone(), *enabled);
                }
                _ => (),
            }
        }
        out.attachments
            .retain(|tag, _| !expiry.get(tag).copied().unwrap_or(false));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enter_alias_enables_distinct_emitters_until_exit() {
        let m = Model::parse("animations { spray test.ska { client { enter emitteron jet_a enter emitteron jet_b exit emitteroff jet_a exit emitteroff jet_b } } }").unwrap();
        for at in [0., 0.5, 0.99] {
            let visual = m.visual("spray", at, 1., 0.05, false);
            assert_eq!(visual.emitters.get("jet_a"), Some(&true));
            assert_eq!(visual.emitters.get("jet_b"), Some(&true));
        }
        let end = m.visual("spray", 1., 1., 0.05, false);
        assert_eq!(end.emitters.get("jet_a"), Some(&false));
        assert_eq!(end.emitters.get("jet_b"), Some(&false));
    }
    #[test]
    fn first_repeats_but_entry_does_not_and_loop_visuals_keep_history() {
        let m=Model::parse("animations { act act.ska { server { entry attachmodel pipe.tik tag_hand 8 removeattachedmodel pipe.tik 9 attachmodel spark.tik tag_tip 1 \"\" 0 .4 } client { first sound sound/loop.wav 0 .5 entry sound sound/start.wav 0 .5 } } }").unwrap();
        let events = m.between(
            "act",
            Span {
                start: 0.,
                end: 2.1,
                duration: 1.,
                frame_time: 0.1,
                looping: true,
                entered: true,
            },
        );
        assert_eq!(
            events
                .iter()
                .filter(|c| matches!(c,Command::Sound{path,..} if path=="sound/loop.wav"))
                .count(),
            3
        );
        assert_eq!(
            events
                .iter()
                .filter(|c| matches!(c,Command::Sound{path,..} if path=="sound/start.wav"))
                .count(),
            1
        );
        let state = m.visual("act", 2.1, 1., 0.1, true);
        assert!(!state.attachments.contains_key("tag_hand"));
        assert!((state.attachments["tag_tip"].age - 0.2).abs() < 0.001);
        assert!(!m
            .visual("act", 2.4, 1., 0.1, true)
            .attachments
            .contains_key("tag_tip"));
    }
    #[test]
    fn optional_arguments_and_ordered_wildcards_do_not_consume_next_event() {
        let m=Model::parse("animations { act act.ska { server { 0 attachmodel pipe.tik tag_hand 5 surface all +nodraw 6 surface top_head -nodraw 7 surface top* +nodraw } } }").unwrap();
        let state = m.visual("act", 0.8, 1., 0.1, false);
        assert_eq!(state.attachments["tag_hand"].scale, 1.);
        let mut hidden = false;
        for (pattern, value) in state.surfaces {
            if matches(&pattern, "top_head") {
                hidden = value;
            }
        }
        assert!(hidden);
        assert!(m
            .visual("act", 5., 1., 0.1, false)
            .attachments
            .contains_key("tag_hand"));
    }
    #[test]
    fn skips_loops_entry_exit_and_restore_keep_order_without_replay() {
        let m=Model::parse("animations { idle idle.ska { client { 8 sound sound/late.wav 0 .8 0 sound sound/zero.wav 0 1 3 sound sound/early.wav 0 .5 } server { entry attachmodel pipe.tik tag_hand exit removeattachedmodel pipe.tik } } }").unwrap();
        let span = Span {
            start: 10.2,
            end: 12.1,
            duration: 1.,
            frame_time: 0.1,
            looping: true,
            entered: false,
        };
        let names: Vec<_> = m
            .between("idle", span)
            .iter()
            .filter_map(|c| {
                if let Command::Sound { path, .. } = c {
                    Some(path.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            names,
            [
                "sound/early.wav",
                "sound/late.wav",
                "sound/zero.wav",
                "sound/early.wav",
                "sound/late.wav",
                "sound/zero.wav"
            ]
        );
        assert!(m.between("idle", Span { end: 10.2, ..span }).is_empty());
        assert!(m
            .between(
                "idle",
                Span {
                    start: 12.1,
                    end: 12.2,
                    ..span
                }
            )
            .is_empty());
        assert!(m
            .visual("idle", 0.5, 1., 0.1, false)
            .attachments
            .contains_key("tag_hand"));
        assert!(m.visual("idle", 1., 1., 0.1, false).attachments.is_empty());
    }
    #[test]
    fn visual_state_restores_removal_by_model_lifetimes_surfaces_and_emitters() {
        let m=Model::parse("animations { act act.ska { server { 0 attachmodel spark.tik tag_tip 1 \"\" 0 .3 1 hideweapon 1 2 surface top* +nodraw 3 attachmodel pipe.tik tag_hand 5 removeattachedmodel pipe.tik } client { 1 emitteron smoke 6 emitteroff smoke } } }").unwrap();
        let s = m.visual("act", 0.4, 1., 0.1, false);
        assert!(
            s.hide_weapon && s.surfaces.contains(&("top*".into(), true)) && s.emitters["smoke"]
        );
        assert!(!s.attachments.contains_key("tag_tip"));
        assert!(s.attachments.contains_key("tag_hand"));
        let restored = m.visual("act", 0.7, 1., 0.1, false);
        assert!(restored.attachments.is_empty() && !restored.emitters["smoke"]);
    }
}

//! Selected dialogue beats read from the user's archives. Never executes game scripts.
use crate::{assets::Assets, audio::Audio, bsp::Bsp};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Cursor;

pub mod registry;
pub mod speakers;
pub use registry::BeatSpec;

pub fn supports(map: &str, event: &str) -> bool {
    registry::for_map(map).iter().any(|b| b.event == event)
}
fn voice_path(path: &str) -> bool {
    path.starts_with("sound/character/")
        && path.ends_with(".wav")
        && !path.contains("..")
        && path.len() < 256
}

/// TLK is a quoted sound path followed by one or more plain-text lines.
/// Windows-1252 and the game's CRCRLF newlines are handled before parsing.
pub(crate) fn subtitles(bytes: &[u8]) -> BTreeMap<String, String> {
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    let mut out = BTreeMap::<String, String>::new();
    let mut key = None;
    for line in text.replace('\r', "\n").lines().map(str::trim) {
        if line.is_empty() || line.starts_with("//") || line == "TALK" {
            continue;
        }
        if let Some(path) = line.strip_prefix('"').and_then(|s| s.split('"').next()) {
            let path = path.replace('\\', "/").to_lowercase();
            key = voice_path(&path).then_some(path);
        } else if let Some(key) = &key {
            let value = out.entry(key.clone()).or_default();
            if value.len() + line.len() > 8192 {
                continue;
            }
            if !value.is_empty() {
                value.push(' ');
            }
            value.push_str(line);
        }
    }
    out
}

/// Small lexical reader: quoted literals, braces and comments, not a script VM.
fn tokens(text: &str) -> Result<Vec<String>> {
    ensure!(text.len() <= 2_000_000, "Story script too large");
    let mut chars = text.chars().peekable();
    let mut out = Vec::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\r' || c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            let mut last = ' ';
            let mut closed = false;
            for c in chars.by_ref() {
                if last == '*' && c == '/' {
                    closed = true;
                    break;
                }
                last = c;
            }
            ensure!(closed, "Unclosed story comment");
            continue;
        }
        if c == '"' || c == '\'' {
            let mut value = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == c {
                    closed = true;
                    break;
                }
                value.push(next);
            }
            ensure!(closed, "Unclosed story literal");
            out.push(value);
        } else if "{}().;,".contains(c) {
            out.push(c.to_string());
        } else {
            let mut value = c.to_string();
            while chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && !"{}().;,\"'".contains(*c))
            {
                value.push(chars.next().unwrap());
            }
            out.push(value);
        }
    }
    Ok(out)
}
// Only unconditional commands in a reviewed linear function are used. A
// controller target is retained across its lines, never inferred from the speaker.
fn dialogue_watches(text: &str, name: &str) -> Result<Vec<BTreeMap<String, String>>> {
    let t = tokens(text)?;
    let start = t
        .windows(5)
        .position(|w| w[0] == "void" && w[1] == name && w[2] == "(" && w[3] == ")" && w[4] == "{")
        .with_context(|| format!("Missing dialogue function {name}"))?
        + 5;
    let mut depth = 1;
    let mut watches = BTreeMap::new();
    let mut result = Vec::new();
    for i in start..t.len() {
        match t[i].as_str() {
            "{" => depth += 1,
            "}" => depth -= 1,
            _ => (),
        }
        if depth == 0 {
            break;
        }
        let Some(w) = t.get(i..i + 6) else {
            continue;
        };
        if w[0].starts_with('$') && w[1] == "." && w[3] == "(" {
            if depth == 1 && w[2] == "headwatch" && w[4].starts_with('$') {
                watches.insert(
                    w[0].trim_start_matches('$').to_owned(),
                    w[4].trim_start_matches('$').to_owned(),
                );
            }
            if w[2] == "playdialog" && w[5] == ")" && voice_path(&w[4]) {
                result.push(watches.clone());
            }
        }
    }
    Ok(result)
}
fn dialogue_calls(text: &str, name: &str) -> Result<Vec<(String, String)>> {
    let t = tokens(text)?;
    let start = t
        .windows(5)
        .position(|w| w[0] == "void" && w[1] == name && w[2] == "(" && w[3] == ")" && w[4] == "{")
        .with_context(|| format!("Missing dialogue function {name}"))?
        + 5;
    let mut depth = 1;
    let mut end = start;
    while end < t.len() {
        match t[end].as_str() {
            "{" => depth += 1,
            "}" => depth -= 1,
            _ => (),
        }
        if depth == 0 {
            break;
        }
        end += 1;
    }
    ensure!(depth == 0, "Unclosed dialogue function");
    let lines = t[start..end]
        .windows(6)
        .filter(|w| {
            w[0].starts_with('$')
                && w[1] == "."
                && w[2] == "playdialog"
                && w[3] == "("
                && w[5] == ")"
                && voice_path(&w[4])
        })
        .map(|w| (w[0].trim_start_matches('$').to_owned(), w[4].clone()))
        .collect::<Vec<_>>();
    ensure!(
        !lines.is_empty() && lines.len() <= 32,
        "Unsupported dialogue list {name}"
    );
    Ok(lines)
}
/// Select only a reviewed list. Conditional branches need named owner gates.
fn reviewed_calls(text: &str, spec: &BeatSpec) -> Result<Vec<(usize, (String, String))>> {
    use registry::Calls;
    let calls = dialogue_calls(text, spec.thread)?;
    ensure!(
        calls.len() == spec.source_lines,
        "Dialogue call count changed: {}/{}",
        spec.map,
        spec.thread
    );
    match spec.calls {
        Calls::Legacy => Ok(calls.into_iter().enumerate().collect()),
        Calls::Linear => {
            let t = tokens(text)?;
            let start = t
                .windows(5)
                .position(|w| {
                    w[0] == "void"
                        && w[1] == spec.thread
                        && w[2] == "("
                        && w[3] == ")"
                        && w[4] == "{"
                })
                .context("Missing dialogue function")?
                + 5;
            let mut depth = 1;
            for word in &t[start..] {
                if word == "{" {
                    depth += 1;
                }
                if word == "}" {
                    depth -= 1;
                }
                if depth == 0 {
                    break;
                }
                ensure!(
                    !["if", "else", "switch", "while", "for"].contains(&word.as_str()),
                    "Conditional dialogue needs an explicit owner gate: {}",
                    spec.thread
                );
            }
            Ok(calls.into_iter().enumerate().collect())
        }
        Calls::Gated { key, indices } => {
            ensure!(
                !key.is_empty() && !indices.is_empty() && indices.windows(2).all(|p| p[0] < p[1]),
                "Invalid dialogue branch selection"
            );
            indices
                .iter()
                .map(|&i| {
                    Ok((
                        i,
                        calls
                            .get(i)
                            .context("Dialogue branch index outside source")?
                            .clone(),
                    ))
                })
                .collect()
        }
    }
}
fn speaker(path: &str) -> &'static str {
    speakers::for_voice(path).map_or("Voice", |s| s.label)
}
#[derive(Clone)]
pub struct Line {
    pub watches: BTreeMap<String, String>,
    pub lips: crate::facial::LipTrack,
    pub actor: String,
    pub path: String,
    pub text: String,
    pub duration: f32,
}
#[derive(Clone)]
struct Sequence {
    repeat: bool,
    id: String,
    lines: Vec<Line>,
}
impl Sequence {
    fn pause_after_line(&self, index: usize) -> f32 {
        if matches!(self.id.as_str(), crate::levels::wchess2::TALK | crate::levels::wchess2::LAST) { return 1.; }
        if self.id == crate::levels::garden4::TALK {
            return [0.3, 0.4, 0.5, 1., 0.4, 1., 0.4, 0.2, 0.3, 0.2][index.min(9)];
        }
        if self.id == crate::levels::garden4::END { return 0.2; }
        if self.id == "Torchgnome3_Dialog_part2" && index < 6 {
            return crate::village::cinema::gnome3::PAUSES[index];
        }
        if self.id == crate::levels::garden1::TURTLE {
            return [0.5, 0.3, 0.4, 0.2, 0.4][index.min(4)];
        }
        if self.id == crate::levels::garden1::RABBIT {
            return [0.4, 0., 0.3, 0.4, 0.2][index.min(4)];
        }
        if self.id == crate::levels::potears2::DIALOGUE {
            return if index == 0 { 1. } else { 0.2 };
        }
        // Reviewed school waits include their authored post-voice delays.
        if self.id == crate::school2::cinema::MUSHROOM {
            return [0.3, 0.4, 0.][index.min(2)];
        }
        if self.id == crate::school2::cinema::FINAL {
            return if index == 0 { 1. } else { 0.2 };
        }
        if matches!(self.id.as_str(), crate::school2::cinema::SPICE | crate::school2::cinema::DICE) {
            return 1.;
        }
        if self.id == crate::school::cinema::THEATRE
            || self.id == "Book_Ingredients_Exit"
            || (self.id == "book_cinematic" && index < 2)
        {
            1.
        } else if self.id == "book_cinematic" {
            0.
        } else {
            0.35
        }
    }
}
struct Playing {
    sequence: Sequence,
    index: usize,
    elapsed: f32,
    started: bool,
}
#[derive(Default)]
pub struct Story {
    /// A scene can hold the next line until its actor movement has finished.
    /// Recomputed from the saved scene every frame, not a second saved clock.
    pub line_limit: Option<usize>,
    map: String,
    available: BTreeMap<String, Sequence>,
    seen: BTreeSet<String>,
    queue: VecDeque<Sequence>,
    current: Option<Playing>,
    exit: Option<(String, Option<String>)>,
    pub completed: u64,
    finished: Vec<String>,
}
impl Story {
    pub fn has_seen(&self, event: &str) -> bool {
        self.seen.contains(event)
    }
    pub fn snapshot(&self) -> Snapshot {
        let reference = |seq: &Sequence| SequenceSave {
            id: seq.id.clone(),
            hint: (seq.id == "cheshire_hint").then(|| seq.lines[0].path.clone()),
            repeat: seq.repeat,
        };
        Snapshot {
            seen: self.seen.clone(),
            queue: self.queue.iter().map(reference).collect(),
            current: self
                .current
                .as_ref()
                .map(|p| (reference(&p.sequence), p.index, p.elapsed)),
            exit: self.exit.clone(),
            completed: self.completed,
            finished: self.finished.clone(),
        }
    }
    pub fn restore(&mut self, s: &Snapshot, hints: &crate::cheshire::Hints) -> Result<()> {
        ensure!(
            s.seen
                .iter()
                .chain(&s.finished)
                .all(|id| self.available.contains_key(id)),
            "Unknown saved dialogue event"
        );
        let sequence = |r: &SequenceSave| -> Result<Sequence> {
            if r.id == "cheshire_hint" {
                let line = hints
                    .saved_line(r.hint.as_deref().context("Missing saved hint")?)
                    .context("Unknown saved hint")?;
                Ok(Sequence {
                    repeat: false,
                    id: r.id.clone(),
                    lines: vec![line],
                })
            } else {
                ensure!(
                    !r.repeat || (self.repeatable(&r.id) && s.seen.contains(&r.id)),
                    "Invalid repeated conversation"
                );
                let mut sequence = self
                    .available
                    .get(&r.id)
                    .cloned()
                    .context("Unknown saved dialogue")?;
                sequence.repeat = r.repeat;
                if r.repeat && self.map == "gvillage" && r.id == "Torchgnome2_Dialog" {
                    sequence.lines.truncate(4);
                }
                Ok(sequence)
            }
        };
        let queue = s
            .queue
            .iter()
            .map(sequence)
            .collect::<Result<VecDeque<_>>>()?;
        let current = if let Some((r, index, elapsed)) = &s.current {
            let sequence = sequence(r)?;
            ensure!(
                *index < sequence.lines.len()
                    && *elapsed >= 0.
                    && *elapsed
                        <= sequence.lines[*index].duration
                            + sequence.pause_after_line(*index).max(0.35)
                            + 0.25,
                "Invalid saved dialogue position"
            );
            Some(Playing {
                sequence,
                index: *index,
                elapsed: *elapsed,
                started: false,
            })
        } else {
            None
        };
        if let Some((map, entry)) = &s.exit {
            ensure!(
                crate::interaction::destination(&format!(
                    "{map}{}",
                    entry.as_ref().map_or(String::new(), |e| format!("${e}"))
                ))
                .is_some(),
                "Invalid saved story exit"
            );
        }
        self.seen = s.seen.clone();
        self.queue = queue;
        self.current = current;
        self.exit = s.exit.clone();
        self.completed = s.completed;
        self.finished = s.finished.clone();
        Ok(())
    }
    pub fn load(assets: &mut Assets, map: &str) -> Self {
        let mut s = Self {
            map: map.into(),
            ..Self::default()
        };
        let table = assets
            .read(&format!("dialog/{map}.tlk"))
            .map(|b| subtitles(&b))
            .unwrap_or_default();
        let mut failed = BTreeSet::new();
        for spec in registry::for_map(map) {
            let (event, script, function) = (spec.event, spec.script, spec.thread);
            let path = if script.starts_with("../") {
                format!("maps/{}.scr", script.trim_start_matches("../"))
            } else {
                format!("maps/cinematics/{script}.scr")
            };
            let result = (|| -> Result<Vec<Line>> {
                let bytes = assets.read(&path)?;
                let text = String::from_utf8_lossy(&bytes);
                let watches = dialogue_watches(&text, function)?;
                ensure!(
                    watches.len() == spec.source_lines,
                    "Headwatch/dialogue count differs"
                );
                reviewed_calls(&text, spec)?
                    .into_iter()
                    .map(|(index, call)| (call, watches[index].clone()))
                    .map(|((actor, path), watches)| {
                        let mut line = load_line(assets, &table, actor, path)?;
                        if !matches!(spec.calls, registry::Calls::Legacy) {
                            ensure!(
                                speakers::for_voice(&line.path).is_some(),
                                "Unregistered speaker {}",
                                line.path
                            );
                            // New beats use the actual recording length, including subsecond
                            // answers. Retain the shipped floor for legacy saved clocks.
                            let wav = hound::WavReader::new(Cursor::new(assets.read(&line.path)?))?;
                            line.duration = wav.duration() as f32 / wav.spec().sample_rate as f32;
                            ensure!(
                                line.duration.is_finite()
                                    && line.duration > 0.
                                    && line.duration <= 120.,
                                "Invalid dialogue duration"
                            );
                        }
                        line.watches = watches;
                        Ok(line)
                    })
                    .collect()
            })();
            match result {
                Ok(lines) => s
                    .available
                    .entry(event.into())
                    .or_insert_with(|| Sequence {
                        repeat: false,
                        id: event.into(),
                        lines: Vec::new(),
                    })
                    .lines
                    .extend(lines),
                Err(e) => {
                    failed.insert(event);
                    eprintln!("Dialogue unavailable {map}/{event}: {e:#}");
                }
            }
        }
        // A multi-function event is atomic: never publish a truncated call list.
        for event in failed {
            s.available.remove(event);
        }
        // These are authored spatial Cat hints, not arbitrary code entry points.
        if map == "skool2" {
            if let Ok(bsp) = assets.read("maps/skool2.bsp").and_then(|b| Bsp::parse(&b)) {
                for e in &bsp.entities {
                    if e.get("classname")
                        .is_some_and(|c| c == "trigger_catmessage")
                    {
                        if let Some(path) = e.get("target").filter(|p| voice_path(p)) {
                            if let Ok(line) = load_line(assets, &table, String::new(), path.clone())
                            {
                                s.available.insert(
                                    path.clone(),
                                    Sequence {
                                        repeat: false,
                                        id: path.clone(),
                                        lines: vec![line],
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }
        println!(
            "Story {map}: {} conversations, {} voiced lines",
            s.available.len(),
            s.available.values().map(|v| v.lines.len()).sum::<usize>()
        );
        s
    }
    pub fn trigger(&mut self, event: &str) -> bool {
        if registry::for_map(&self.map)
            .iter()
            .any(|b| b.event == event && b.gate().is_some())
        {
            return false;
        }
        self.enqueue(event, false)
    }
    /// Owner evaluates the named quest condition before selecting a fixed branch.
    /// The selected event and its stable indices are already saved by Story.
    pub fn trigger_gated(&mut self, event: &str, gate: &str, allowed: bool) -> bool {
        let specs = registry::for_map(&self.map);
        let matching: Vec<_> = specs.iter().filter(|b| b.event == event).collect();
        allowed
            && !matching.is_empty()
            && matching.iter().any(|b| b.gate() == Some(gate))
            && matching
                .iter()
                .all(|b| b.gate().is_none_or(|key| key == gate))
            && self.enqueue(event, false)
    }
    fn enqueue(&mut self, event: &str, replay: bool) -> bool {
        let Some(sequence) = self.available.get(event) else {
            return false;
        };
        if self.seen.contains(event) && !replay {
            return true;
        }
        if self.queue.len() >= 8 {
            return false;
        }
        self.seen.insert(event.into());
        let mut sequence = sequence.clone();
        sequence.repeat = replay;
        if replay && self.map == "gvillage" && event == "Torchgnome2_Dialog" {
            // Repeatable Gnome talk does not replay the one-time Cat cutaway.
            sequence.lines.truncate(4);
        }
        self.queue.push_back(sequence);
        true
    }
    fn repeatable(&self, event: &str) -> bool {
        self.map == "gvillage"
            && matches!(
                event,
                "Torchgnome1_Dialog"
                    | "Torchgnome2_Dialog"
                    | "Torchgnome3_Dialog_part2"
                    | "Torchgnome4_Dialog"
            )
    }
    /// A replay is speech only: it cannot publish completion/reward callbacks.
    /// The interaction owner additionally checks the original scene has finished.
    pub fn repeat_conversation(&mut self, event: &str) -> bool {
        !self.busy()
            && self.repeatable(event)
            && self.seen.contains(event)
            && self.enqueue(event, true)
    }
    pub fn progress(&self, event: &str) -> Option<(usize, f32)> {
        self.current
            .as_ref()
            .filter(|p| p.sequence.id == event)
            .map(|p| (p.index, p.elapsed))
    }
    pub fn sequence_pending(&self, event: &str) -> bool {
        self.current
            .as_ref()
            .is_some_and(|p| p.sequence.id == event)
            || self.queue.iter().any(|s| s.id == event)
    }
    /// A scene retains its own line cursor when speech is interrupted. Restore only
    /// that registered sequence, without clearing other dialogue or emitting rewards.
    pub fn resume_scene(&mut self, event: &str, index: usize, elapsed: f32) -> bool {
        if self.progress(event).is_some() {
            return true;
        }
        if self.current.is_some() || self.queue.iter().any(|s| s.id != event) {
            return false;
        }
        let Some(sequence) = self.available.get(event).cloned() else {
            return false;
        };
        let Some(line) = sequence.lines.get(index) else {
            return false;
        };
        if !elapsed.is_finite()
            || elapsed < 0.
            || elapsed > line.duration + sequence.pause_after_line(index).max(0.35) + 0.25
        {
            return false;
        }
        self.queue.retain(|s| s.id != event);
        self.finished.retain(|s| s != event);
        self.seen.insert(event.into());
        self.current = Some(Playing {
            sequence,
            index,
            elapsed,
            started: false,
        });
        true
    }
    pub fn line_finished(&self) -> bool {
        self.current.as_ref().is_some_and(|p| {
            p.elapsed >= p.sequence.lines[p.index].duration + p.sequence.pause_after_line(p.index)
        })
    }
    /// Finish only the requested scene, preserving other queued conversations and their order.
    /// Completion is emitted once through the normal callback path, even before its first line.
    pub fn finish_sequence(&mut self, event: &str) -> bool {
        let playing = self
            .current
            .as_ref()
            .is_some_and(|p| p.sequence.id == event);
        let queued = self.queue.iter().any(|p| p.id == event);
        if !playing && !queued {
            return false;
        }
        let commits = self
            .current
            .as_ref()
            .is_some_and(|p| p.sequence.id == event && !p.sequence.repeat)
            || self.queue.iter().any(|p| p.id == event && !p.repeat);
        if playing {
            self.current = None;
        }
        self.queue.retain(|p| p.id != event);
        if commits && event != "cheshire_hint" {
            self.completed += 1;
            self.finished.push(event.into());
        }
        true
    }
    pub fn busy(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty()
    }
    /// User-requested hints never enter the quest's seen/completion registry.
    pub fn summon_hint(&mut self, line: Line) -> bool {
        if self.busy() {
            return false;
        }
        self.queue.push_back(Sequence {
            repeat: false,
            id: "cheshire_hint".into(),
            lines: vec![line],
        });
        true
    }
    pub fn hint_active(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|p| p.sequence.id == "cheshire_hint")
            || self.queue.iter().any(|s| s.id == "cheshire_hint")
    }
    pub fn defer_exit(&mut self, exit: (String, Option<String>)) {
        self.exit = Some(exit);
    }
    pub fn pending_exit_map(&self) -> Option<&str> {
        self.exit.as_ref().map(|e| e.0.as_str())
    }
    pub fn clear_exit(&mut self) {
        self.exit = None;
    }
    pub fn take_exit(&mut self) -> Option<(String, Option<String>)> {
        if self.busy() {
            None
        } else {
            self.exit.take()
        }
    }
    pub fn line(&self) -> Option<&Line> {
        self.current.as_ref().map(|p| &p.sequence.lines[p.index])
    }
    /// Sample the exact saved dialogue clock; no separate mouth timer can drift.
    pub fn mouth(&self, actors: &[&str]) -> f32 {
        self.current.as_ref().map_or(0., |p| {
            let line = &p.sequence.lines[p.index];
            if actors
                .iter()
                .any(|actor| speakers::same_actor(actor, &line.actor))
            {
                line.lips.sample(p.elapsed)
            } else {
                0.
            }
        })
    }
    pub fn cast(&self) -> Vec<&str> {
        self.current
            .as_ref()
            .map(|p| p.sequence.lines.iter().map(|l| l.actor.as_str()).collect())
            .unwrap_or_default()
    }
    // Clock is shared with gameplay. Audio has its own pausable sink; unavailable audio
    // retains the same subtitle timeline. Advance only skips the currently visible line.
    pub(crate) fn tick(&mut self, dt: f32, advance: bool) -> bool {
        if dt <= 0. || !dt.is_finite() {
            return false;
        }
        if let Some(p) = &mut self.current {
            let end = p.sequence.lines[p.index].duration + p.sequence.pause_after_line(p.index);
            p.elapsed = (p.elapsed + dt.min(0.25)).min(end);
            if advance && p.started {
                p.elapsed = end;
            }
            if p.started && p.elapsed >= end && self.line_limit.is_none_or(|limit| p.index < limit)
            {
                p.index += 1;
                p.elapsed = 0.;
                p.started = false;
                if p.index >= p.sequence.lines.len() {
                    println!("Story completed: {}", p.sequence.id);
                    if !p.sequence.repeat && p.sequence.id != "cheshire_hint" {
                        self.completed += 1;
                        self.finished.push(p.sequence.id.clone());
                    }
                    self.current = None;
                }
            }
        }
        if self.current.is_none() {
            self.current = self.queue.pop_front().map(|sequence| Playing {
                sequence,
                index: 0,
                elapsed: 0.,
                started: false,
            });
        }
        if let Some(p) = &mut self.current {
            if !p.started {
                p.started = true;
                return true;
            }
        }
        false
    }
    pub fn update(&mut self, dt: f32, advance: bool, assets: &mut Assets, audio: &mut Audio) {
        let had_line = self.line().is_some();
        if self.tick(dt, advance) {
            if let Some(line) = self.line() {
                audio.speak_from(
                    assets,
                    &line.path,
                    self.current.as_ref().map_or(0., |p| p.elapsed),
                );
            }
        } else if had_line && self.line().is_none() {
            audio.stop_voice();
        }
    }
    pub fn take_completed(&mut self) -> Vec<String> {
        std::mem::take(&mut self.finished)
    }
    /// On recovery replay an interrupted beat from the start, retaining finished history.
    /// In particular discard a pending exit so retry never transports Alice from the entrance.
    pub fn recover(&mut self, audio: &mut Audio) {
        self.exit = None;
        if let Some(p) = self.current.take() {
            if p.sequence.id != "cheshire_hint" {
                self.queue.push_front(p.sequence);
            }
        }
        self.queue.retain(|s| s.id != "cheshire_hint");
        audio.stop_voice();
    }
    pub fn objective(&self) -> Option<&str> {
        match self.map.as_str() {
            "skool2" if !self.seen.contains("Old_Gnome_Mushroom") => {
                Some("Explore the east gymnasium. E uses its lever; the Elder Gnome waits above.")
            }
            "skool2" => Some("Gather the ingredients and help the Elder Gnome prepare the potion."),
            "gvillage" => Some("Explore the village; approach its gnomes and press E to talk."),
            _ => None,
        }
    }
    pub fn draw(&self, use_key: &str, ui: &crate::ui::Ui) {
        let Some(line) = self.line() else {
            return;
        };
        draw_subtitle(&line.text, speaker(&line.path), use_key,
            self.current.as_ref().unwrap().elapsed, line.duration, ui);
    }
}

pub(crate) fn draw_subtitle(text: &str, speaker_name: &str, use_key: &str, elapsed: f32, duration: f32, ui: &crate::ui::Ui) {
        let s = crate::ui::overlay_scale();
        let width = (screen_width() - 180. * s).clamp(220. * s, 850. * s);
        let font = (screen_height() / (32. * s)).clamp(16., 23.) * s;
        let rows = wrap(text, width - 36. * s, |s| ui.body.width(s, font));
        // Paginate unusually long lines instead of obscuring the play area.
        let pages = rows.len().div_ceil(3).max(1);
        let page = ((elapsed / duration.max(1.) * pages as f32) as usize).min(pages - 1);
        let rows = &rows[(page * 3).min(rows.len())..((page + 1) * 3).min(rows.len())];
        let height = 46. * s + rows.len() as f32 * (font + 4. * s);
        let x = (screen_width() - width) / 2.;
        let y = screen_height() - 95. * s - height;
        ui.dialog(Rect::new(x, y, width, height));
        ui.label(
            speaker_name,
            x + 18. * s,
            y + 25. * s,
            20. * s,
            Color::from_hex(0xf3d6a0),
        );
        let hint = format!("{use_key}  next line");
        ui.body.left(
            &hint,
            x + width - ui.body.width(&hint, 16. * s) - 18. * s,
            y + 24. * s,
            16. * s,
            WHITE,
        );
        for (i, row) in rows.iter().enumerate() {
            ui.body.left(
                row,
                x + 18. * s,
                y + 49. * s + i as f32 * (font + 4. * s),
                font,
                WHITE,
            );
        }
}
pub(crate) fn load_line(
    assets: &mut Assets,
    table: &BTreeMap<String, String>,
    actor: String,
    path: String,
) -> Result<Line> {
    let text = table.get(&path).context("No matching subtitle")?.clone();
    let duration = assets
        .read(&path)
        .and_then(|b| {
            let wav = hound::WavReader::new(Cursor::new(b))?;
            Ok(wav.duration() as f32 / wav.spec().sample_rate as f32)
        })
        .unwrap_or_else(|e| {
            eprintln!("Voice unavailable {path}: {e:#}; using subtitles");
            (text.split_whitespace().count() as f32 / 2.6).max(3.)
        })
        .clamp(1., 120.);
    // Some source lines assign the Cat's voice to fakeplayer. Label and animate the
    // actual speaker, never Alice speaking with the Cat's voice.
    let actor = speakers::bind_actor(&actor, &path).to_owned();
    Ok(Line {
        watches: BTreeMap::new(),
        lips: crate::facial::LipTrack::load(assets, &path),
        actor,
        path,
        text,
        duration,
    })
}
pub(crate) fn wrap(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut rows = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        let next = if row.is_empty() {
            word.to_owned()
        } else {
            format!("{row} {word}")
        };
        if !row.is_empty() && measure(&next) > width {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct SequenceSave {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    repeat: bool,
    id: String,
    hint: Option<String>,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    seen: BTreeSet<String>,
    queue: Vec<SequenceSave>,
    current: Option<(SequenceSave, usize, f32)>,
    exit: Option<(String, Option<String>)>,
    completed: u64,
    finished: Vec<String>,
}
pub fn check_faces(assets: &mut Assets) -> Result<()> {
    let mut count = 0;
    for map in registry::maps() {
        let story = Story::load(assets, map);
        ensure!(!story.available.is_empty(), "Missing dialogue for {map}");
        for seq in story.available.values() {
            for line in &seq.lines {
                let track = crate::facial::LipTrack::checked(assets, &line.path)?;
                for i in 0..(line.duration * 40.) as usize {
                    let sample = track.sample(i as f32 / 40.);
                    ensure!(
                        sample == line.lips.sample(i as f32 / 40.),
                        "Lip binding differs from source {}",
                        line.path
                    );
                    ensure!((0. ..=1.).contains(&sample), "Invalid lip envelope");
                }
                ensure!(
                    line.lips.sample(line.duration + 0.35) == 0.,
                    "Mouth not closed after {}",
                    line.path
                );
                count += 1;
            }
        }
    }
    let voice = "sound/character/cheshire_cat/vo/catz306.wav";
    let table = subtitles(&assets.read("dialog/hatter1.tlk")?);
    let line = load_line(assets, &table, "cat".into(), voice.into())?;
    let track = crate::facial::LipTrack::checked(assets, voice)?;
    ensure!(
        !assets.contains(&format!("{}.lip", voice.trim_end_matches(".wav"))),
        "Neutral-mouth fixture changed"
    );
    ensure!(
        (0..(line.duration * 40.) as usize)
            .all(|i| track.sample(i as f32 / 40.) == 0. && line.lips.sample(i as f32 / 40.) == 0.),
        "Missing lip track moved the mouth"
    );
    println!("PASS {count} registered story lines bind supplied lip envelopes; catz306 missing-track fixture remains neutral");
    Ok(())
}

pub fn check(assets: &mut Assets) -> Result<()> {
    use rodio::Source;
    let mut count = 0;
    for map in registry::maps() {
        let mut s = Story::load(assets, map);
        let mut expected = registry::events(map).len();
        if map == "skool2" {
            let bsp = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
            expected += bsp
                .entities
                .iter()
                .filter(|e| {
                    e.get("classname")
                        .is_some_and(|c| c == "trigger_catmessage")
                })
                .filter_map(|e| e.get("target").filter(|p| voice_path(p)))
                .collect::<BTreeSet<_>>()
                .len();
        }
        ensure!(s.available.len() == expected, "Missing {map} story beats");
        for event in registry::events(map) {
            let expected: usize = registry::for_map(map)
                .iter()
                .filter(|b| b.event == event)
                .map(|b| b.line_count())
                .sum();
            ensure!(
                s.available[event].lines.len() == expected,
                "Wrong line count {map}/{event}"
            );
        }
        for seq in s.available.values() {
            for line in &seq.lines {
                ensure!(
                    speakers::for_voice(&line.path).is_some(),
                    "Unregistered voice {}",
                    line.path
                );
                let wav = hound::WavReader::new(Cursor::new(assets.read(&line.path)?))?;
                ensure!(
                    wav.duration() > 0 && !line.text.is_empty(),
                    "Empty dialogue asset"
                );
                let decoded = rodio::Decoder::new(Cursor::new(assets.read(&line.path)?))?;
                let rate = decoded.sample_rate();
                let channels = decoded.channels();
                let mut samples = 0usize;
                let mut peak = 0f32;
                for sample in decoded.convert_samples::<f32>() {
                    ensure!(sample.is_finite(), "Nonfinite decoded voice");
                    samples += 1;
                    peak = peak.max(sample.abs());
                }
                let seconds = samples as f32 / rate as f32 / channels as f32;
                ensure!(
                    peak > 0.001 && (seconds - line.duration).abs() < 0.01,
                    "Voice decode/timing mismatch: {} (decoded {seconds}, scheduled {})",
                    line.path,
                    line.duration
                );
                count += 1;
            }
        }
        // Registered owners may gate speech behind combat, movement or a shot.
        // Validate their real contact/lifecycle contracts instead of demanding
        // immediate dialogue from a bare touch in an unsolved fresh visit.
        let owners: Vec<_> = crate::levels::for_map(map).collect();
        for owner in &owners {
            let flag = format!("--{}-check", owner.id);
            let check = owner.checks.iter().find(|c| c.flag == flag)
                .with_context(|| format!("Missing scene owner contract: {flag}"))?;
            let crate::levels::Run::Headless(run) = check.run else {
                anyhow::bail!("Scene owner contract needs a window: {flag}");
            };
            println!("Story {map}: checking owned gates and scene clocks through {flag}");
            run(assets)?;
        }
        if owners.is_empty() && !matches!(map, "skool1" | "potears3") {
            use crate::{
                collision::World,
                interaction::{self, Interactions},
                movement::{Controls, Player},
            };
            let bsp = Bsp::parse(&assets.read(&format!("maps/{map}.bsp"))?)?;
            let mut world = World::from_bsp(&bsp)?;
            let mut events = Interactions::load(&bsp)?;
            events.set_entry(assets, &bsp, map, None)?;
            events.sync(&mut world);
            let mut player = Player::spawn(&world, interaction::spawn(&bsp, None).0)
                .context("Blocked story map spawn")?;
            for _ in 0..240 {
                player.tick(&world, Controls::default());
            }
            ensure!(
                world.body_clear(player.feet) && player.grounded,
                "Story map arrival does not reach solid ground"
            );
            println!("Story {map}: arrival settles safely at {:?}", player.feet);
            for e in &bsp.entities {
                if !bsp.difficulty.allows(
                    e.get("spawnflags")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0),
                ) {
                    continue;
                }
                let expected = e.get("thread").filter(|s| supports(map, s)).or_else(|| {
                    e.get("target").filter(|_| {
                        map == "skool2"
                            && e.get("classname")
                                .is_some_and(|s| s == "trigger_catmessage")
                    })
                });
                if let Some(expected) = expected {
                    if map == "fortress1" && expected == crate::fortress::cinema::ARRIVAL {
                        ensure!(
                            e.get("classname").is_some_and(|s| s == "info_player_start"),
                            "Fortress arrival is no longer attached to the level entrance"
                        );
                        events.entry_story(&mut s);
                        ensure!(
                            events.scripted() && s.queue.iter().any(|seq| seq.id == *expected),
                            "Fortress entry did not queue its arrival scene"
                        );
                        continue;
                    }
                    if (map == "skool2" && expected == "Skool2_LastGnome_Cinema")
                        || (map == "pandemonium" && expected == "Pand_End_Ship")
                        || (map == "fortress1" && expected == "skool_cat")
                    {
                        continue;
                    } // Progression-gated conversations are covered by their map and continuous route checks.

                    let feet = e
                        .get("origin")
                        .and_then(|s| interaction::vector(s))
                        .context("Story trigger lacks origin")?
                        - crate::collision::PLAYER_CENTER;
                    events.reset_contacts();
                    ensure!(
                        events.triggers(0., feet, feet).story.is_empty(),
                        "Paused story trigger fired"
                    );
                    let fired = events.triggers(0.01, feet, feet);
                    ensure!(
                        fired.story.contains(expected),
                        "Story trigger missing: {map}/{expected}"
                    );
                    ensure!(
                        events.triggers(0.01, feet, feet).story.is_empty(),
                        "Story trigger repeats while occupied"
                    );
                }
                if map == "skool2"
                    && e.get("classname")
                        .is_some_and(|s| s == "trigger_changelevel")
                {
                    let feet = e
                        .get("origin")
                        .and_then(|s| interaction::vector(s))
                        .unwrap();
                    ensure!(
                        events.triggers(0.01, feet, feet).transition.is_none(),
                        "Premature school exit"
                    );
                }
            }
        }
    }
    println!("Story check passed: {count} original voice/subtitle pairs across registered maps");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_conditional_speech_requires_an_explicit_stable_branch() {
        let text = "void dialog() { $fakeplayer.playdialog(\"sound/character/fixture_a.wav\"); if (ready) { $bill.playdialog(\"sound/character/fixture_b.wav\"); } }";
        let mut spec = BeatSpec::linear("test", "event", "test", "dialog", 2);
        assert!(reviewed_calls(text, &spec).is_err());
        spec.calls = registry::Calls::Gated {
            key: "test.ready",
            indices: &[1],
        };
        let calls = reviewed_calls(text, &spec).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 1);
        assert_eq!(calls[0].1 .0, "bill");
        spec.calls = registry::Calls::Gated {
            key: "test.ready",
            indices: &[1, 0],
        };
        assert!(reviewed_calls(text, &spec).is_err());
        spec.calls = registry::Calls::Gated {
            key: "test.ready",
            indices: &[2],
        };
        assert!(reviewed_calls(text, &spec).is_err());
    }
    #[test]
    fn registered_owner_gate_cannot_be_bypassed_by_generic_trigger() {
        let mut s = story();
        s.map = "potears2".into();
        let event = crate::levels::potears2::DIALOGUE;
        let mut sequence = s.available.remove("a").unwrap();
        sequence.id = event.into();
        s.available.insert(event.into(), sequence);
        assert!(!s.trigger(event));
        assert!(!s.trigger_gated(event, "wrong", true));
        assert!(!s.trigger_gated(event, "potears2.bill_dialogue_ready", false));
        assert!(!s.busy() && !s.has_seen(event));
        assert!(s.trigger_gated(event, "potears2.bill_dialogue_ready", true));
        let bytes = serde_json::to_vec(&s.snapshot()).unwrap();
        s.restore(
            &serde_json::from_slice(&bytes).unwrap(),
            &crate::cheshire::Hints::default(),
        )
        .unwrap();
        assert!(s.sequence_pending(event));
        assert!(!String::from_utf8_lossy(&bytes).contains("gate"));
        s.finish_sequence(event);
        assert_eq!(s.take_completed(), vec![event]);
        assert!(s.trigger_gated(event, "potears2.bill_dialogue_ready", true));
        assert!(!s.busy());
    }
    #[test]
    fn headwatch_uses_only_explicit_targets_before_the_current_line() {
        let text = "void dialog() { // $cat.headwatch($wrong);\n $cat.headwatch($player,2); $cat.playdialog(\"sound/character/cat.wav\"); if (condition) { $cat.headwatch($branch); } $cat.headwatch($gnome,2); $alice.playdialog(\"sound/character/alice.wav\"); }";
        let w = dialogue_watches(text, "dialog").unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!(w[0]["cat"], "player");
        assert_eq!(w[1]["cat"], "gnome");
        assert!(!w[1].contains_key("alice"));
    }
    fn story() -> Story {
        let line = Line {
            watches: BTreeMap::new(),
            lips: Default::default(),
            actor: "gnome".into(),
            path: "sound/character/test.wav".into(),
            text: "Synthetic test line".into(),
            duration: 1.,
        };
        Story {
            available: BTreeMap::from([(
                "a".into(),
                Sequence {
                    repeat: false,
                    id: "a".into(),
                    lines: vec![line.clone(), line],
                },
            )]),
            ..Default::default()
        }
    }
    #[test]
    fn conversation_repeats_restore_and_finish_without_quest_callbacks() {
        for skip in [false, true] {
            for queued in [false, true] {
                let mut s = story();
                s.map = "gvillage".into();
                let id = "Torchgnome3_Dialog_part2";
                let mut sequence = s.available.remove("a").unwrap();
                sequence.id = id.into();
                s.available.insert(id.into(), sequence);
                assert!(!s.repeat_conversation(id));
                assert!(s.trigger(id));
                assert!(!serde_json::to_string(&s.snapshot())
                    .unwrap()
                    .contains("repeat"));
                s.finish_sequence(id);
                assert_eq!(s.take_completed(), vec![id]);
                assert_eq!(s.completed, 1);
                assert!(s.repeat_conversation(id));
                assert!(!s.repeat_conversation(id));
                if !queued {
                    s.tick(0.01, false);
                }
                let saved = serde_json::to_vec(&s.snapshot()).unwrap();
                s.restore(
                    &serde_json::from_slice(&saved).unwrap(),
                    &crate::cheshire::Hints::default(),
                )
                .unwrap();
                if skip {
                    assert!(s.finish_sequence(id));
                } else {
                    for _ in 0..30 {
                        s.tick(0.25, true);
                    }
                }
                assert!(!s.busy());
                assert!(s.take_completed().is_empty());
                assert_eq!(s.completed, 1);
                assert!(s.take_exit().is_none());
            }
        }
    }
    #[test]
    fn old_sequence_saves_default_to_authored_completion() {
        let mut s = story();
        s.trigger("a");
        let bytes = serde_json::to_vec(&s.snapshot()).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("repeat"));
        s.restore(
            &serde_json::from_slice(&bytes).unwrap(),
            &crate::cheshire::Hints::default(),
        )
        .unwrap();
        s.finish_sequence("a");
        assert_eq!(s.take_completed(), vec!["a"]);
        assert!(!s.repeat_conversation("a"));
    }
    #[test]
    fn theatre_pause_survives_save_without_replaying_voice_or_completing_early() {
        let mut s = story();
        let mut sequence = s.available.remove("a").unwrap();
        sequence.id = crate::school::cinema::THEATRE.into();
        s.available.insert(sequence.id.clone(), sequence);
        let id = crate::school::cinema::THEATRE;
        s.trigger(id);
        s.tick(0.01, false);
        for _ in 0..7 {
            s.tick(0.25, false);
        }
        assert_eq!(s.progress(id), Some((0, 1.75)));
        assert!(!s.line_finished());
        let saved = s.snapshot();
        s.restore(&saved, &crate::cheshire::Hints::default())
            .unwrap();
        s.tick(0.01, false);
        assert_eq!(s.progress(id).unwrap().0, 0);
        s.tick(0.25, false);
        assert_eq!(s.progress(id), Some((1, 0.)));
        assert_eq!(s.completed, 0);
        assert!(s.finish_sequence(id));
        assert!(!s.finish_sequence(id));
        assert_eq!(s.completed, 1);
    }
    #[test]
    fn staged_movement_holds_next_line_and_keeps_save_clock_bounded() {
        let mut s = story();
        s.trigger("a");
        s.line_limit = Some(0);
        for _ in 0..100 {
            s.tick(0.1, true);
        }
        assert_eq!(s.progress("a"), Some((0, 1.35)));
        assert!(s.line_finished());
        let snapshot = s.snapshot();
        s.restore(&snapshot, &crate::cheshire::Hints::default())
            .unwrap();
        s.line_limit = None;
        s.tick(0.01, false);
        s.tick(0.01, false);
        assert_eq!(s.progress("a").unwrap().0, 1);
    }
    #[test]
    fn mouth_follows_speaker_pause_skip_and_saved_dialogue_clock() {
        let mut s = story();
        let lines = &mut s.available.get_mut("a").unwrap().lines;
        lines[0].lips = crate::facial::LipTrack::parse(b"1 4 0 255 0 0").unwrap();
        lines[1].actor = "alice".into();
        s.trigger("a");
        s.tick(0.01, false);
        s.tick(0.05, false);
        assert_eq!(s.mouth(&["gnome"]), 1.);
        assert_eq!(s.mouth(&["alice"]), 0.);
        let saved = s.snapshot();
        s.tick(0., true);
        assert_eq!(s.mouth(&["gnome"]), 1.);
        s.tick(0.01, true);
        assert_eq!(s.mouth(&["gnome"]), 0.);
        s.restore(&saved, &crate::cheshire::Hints::default())
            .unwrap();
        assert_eq!(s.mouth(&["gnome"]), 1.);
        s.finish_sequence("a");
        assert_eq!(s.mouth(&["gnome"]), 0.);
    }
    #[test]
    fn reads_calls_without_comments_prototypes_or_other_functions() {
        let t = "void test(); void other(){ $bad.playdialog(\"sound/character/wrong.wav\"); } void test() { /* $bad.playdialog(\"sound/character/wrong.wav\"); */ // ignored\n $good.playdialog(\"sound/character/right.wav\"); }";
        assert_eq!(
            dialogue_calls(t, "test").unwrap(),
            vec![("good".into(), "sound/character/right.wav".into())]
        );
        assert!(dialogue_calls(t, "absent").is_err());
    }
    #[test]
    fn tlk_multiline_crcrlf_encoding_and_paths() {
        let t = b"TALK\r\r\n// comment\n\"sound/character/test.wav\"\r\r\nA test\r\nwith \x93quotes\x94.\n\"../bad.wav\"\nIgnore";
        let parsed = subtitles(t);
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed["sound/character/test.wav"],
            "A test with \u{201c}quotes\u{201d}."
        );
    }
    #[test]
    fn pause_skip_once_queue_and_deferred_exit() {
        let mut s = story();
        assert!(s.trigger("a"));
        assert!(s.trigger("a"));
        assert_eq!(s.queue.len(), 1);
        s.defer_exit(("next".into(), None));
        assert!(!s.tick(0., true));
        assert!(s.line().is_none());
        assert!(s.tick(0.01, false));
        assert!(!s.tick(0., true));
        assert_eq!(s.current.as_ref().unwrap().index, 0);
        assert!(s.tick(0.01, true));
        assert_eq!(s.current.as_ref().unwrap().index, 1);
        assert!(s.take_exit().is_none());
        assert!(!s.tick(0.01, true));
        assert!(!s.busy());
        assert_eq!(s.take_exit().unwrap().0, "next");
        assert!(s.take_exit().is_none());
        assert_eq!(s.completed, 1);
    }
    #[test]
    fn wraps_by_measured_width_without_losing_words() {
        let rows = wrap("one two three four", 8., |s| s.len() as f32);
        assert_eq!(rows, vec!["one two", "three", "four"]);
        assert_eq!(rows.join(" "), "one two three four");
    }
    #[test]
    fn recovery_replays_interrupted_dialogue_but_cancels_exit() {
        let mut s = story();
        let mut audio = Audio::new(true, None);
        s.trigger("a");
        s.tick(0.01, false);
        s.tick(0.01, true);
        s.defer_exit(("next".into(), None));
        s.recover(&mut audio);
        assert!(s.tick(0.01, false));
        assert_eq!(s.current.as_ref().unwrap().index, 0);
        s.tick(0.01, true);
        s.tick(0.01, true);
        assert!(s.take_exit().is_none());
        assert!(s.trigger("a"));
        assert!(!s.busy());
    }
    #[test]
    fn whole_scene_skip_finishes_once_and_retains_unrelated_queue() {
        for start in [false, true] {
            let mut s = story();
            let mut other = s.available["a"].clone();
            other.id = "b".into();
            s.available.insert("b".into(), other);
            s.trigger("a");
            s.trigger("b");
            if start {
                s.tick(0.01, false);
            }
            assert!(s.finish_sequence("a"));
            assert!(!s.finish_sequence("a"));
            assert_eq!(s.take_completed(), ["a"]);
            assert_eq!(s.completed, 1);
            assert!(s.tick(0.01, false));
            assert!(s.progress("b").is_some());
            s.trigger("a");
            assert_eq!(s.queue.len(), 0);
        }
    }
}

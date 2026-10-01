//! Audio from the user's archives. No original DLL, executable, or script execution.
mod acoustics;
pub mod events;
mod reaction;
pub mod regression;
pub mod world;
use crate::{assets::Assets, bsp::Bsp, collision::World};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::{vec3, Vec3};
use rodio::{dynamic_mixer, Decoder, OutputStream, Sink, Source};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
    path::PathBuf,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

const EFFECTS: [&str; 22] = [
    "sound/character/alice/footstep1.wav",
    "sound/character/alice/footstep2.wav",
    "sound/character/alice/jump.wav",
    "sound/character/alice/small_land.wav",
    "sound/character/alice/weapon_switch.wav",
    "sound/weapon/knife/knife_swing1.wav",
    "sound/weapon/knife/knife_swing2.wav",
    "sound/weapon/knife/knife_swing3.wav",
    "sound/weapon/knife/knife_throw.wav",
    "sound/weapon/knife/knife_hit_world1.wav",
    "sound/weapon/cards/cards_toss1.wav",
    "sound/weapon/cards/cards_hit_world1.wav",
    "sound/weapon/mallet/mallet_swing1.wav",
    "sound/weapon/mallet/mallet_swing2.wav",
    "sound/weapon/mallet/mallet_ball_swing.wav",
    "sound/weapon/jackbomb/jackbomb_toss.wav",
    "sound/weapon/mallet/mallet_hit_world1.wav",
    "sound/weapon/mallet/mallet_ball_bounce.wav",
    "sound/character/alice/splash1.wav",
    "sound/character/alice/leavewater.wav",
    "sound/character/alice/swim1.wav",
    "sound/character/alice/swim2.wav",
];
const WORLD_EFFECTS: &[&str] = &[
    "sound/character/alice/drink.wav",
    "sound/world/machine/globe_open.wav",
    "sound/ambience/special/observatory_lift.wav",
    "sound/world/door/door wood open 01.wav",
    "sound/weapon/jackbomb/jackbomb_music.wav",
    "sound/weapon/jackbomb/jackbomb_pop.wav",
    "sound/weapon/jackbomb/jackbomb_breath.wav",
    "sound/weapon/jackbomb/jackbomb_explode.wav",
    "sound/item/ragebox/ragebox_power.wav",
    "sound/item/ragebox/ragebox_pickup.wav",
    "sound/item/tea/powerup.wav",
    "sound/item/glass/powerup.wav",
    "sound/item/watch/use_watch.wav",
    "sound/item/essence/small/mtespu.wav",
    "sound/character/alice/choke1.wav",
    "sound/character/cheshire_cat/appear.wav",
    "sound/character/cheshire_cat/disappear.wav",
    "sound/world/machine/lever1.wav",
    "sound/world/mover/bleacher.wav",
    "sound/character/boojum/attack.wav",
    "sound/character/boojum/pain01.wav",
    "sound/character/boojum/death.wav",
    "sound/character/cardguard/club/alert1.wav",
    "sound/character/cardguard/club/attack1.wav",
    "sound/character/cardguard/club/pain1.wav",
    "sound/character/cardguard/club/death1.wav",
    "sound/character/cardguard/diamond/alert1.wav",
    "sound/character/cardguard/diamond/stand_attack.wav",
    "sound/character/cardguard/diamond/pain1.wav",
    "sound/character/cardguard/diamond/death1.wav",
    "sound/character/cardguard/diamond/prj_hit_flesh1.wav",
    "sound/ambience/special/slow scrape.wav",
    "sound/character/gnome/elder/vanish.wav",
    "sound/character/gnome/elder/mixing.wav",
    "sound/ambience/special/lollipop_grow.wav",
    "sound/world/machine/condenser.wav",
    "sound/ambience/special/door_flip.wav",
    "sound/item/pickup.wav",
    "sound/weapon/knife/knife_hit_flesh1.wav",
    "sound/world/door/door wood open 02.wav",
];
const EXTRA_EFFECTS: &[&str] = &[
    "sound/character/cardguard/club/death_3a.wav",
    "sound/character/cardguard/club/death_3b.wav",
    "sound/character/cardguard/club/death_3c.wav",
    "sound/world/machine/mine_lift1.wav",
    "sound/world/machine/mine_lift2.wav",
    "sound/world/machine/mine_lift3.wav",
    "sound/ambience/special/splash_1.wav",
    "sound/ambience/special/lift_1.wav",
    "sound/world/mover/bookcase_fall_1.wav",
    "sound/world/mover/bookcase_fall_2.wav",
    "sound/world/mover/roll_1.wav",
    "sound/world/mover/roll_2.wav",
    "sound/weapon/cards/cards_loop1.wav",
    "sound/weapon/mallet/mallet_ball_loop.wav",
    "sound/ambience/special/roomsplit.wav",
    "sound/weapon/mallet/mallet_hit_flesh1.wav",
    "sound/weapon/mallet/mallet_ball_flesh1.wav",
    "sound/weapon/cards/cards_hit_flesh1.wav",
    "sound/character/alice/pain1.wav",
    "sound/character/alice/pain_strong1.wav",
    "sound/character/alice/pain_water1.wav",
    "sound/character/alice/submerge.wav",
    "sound/character/alice/gasp1.wav",
    "sound/character/alice/swimloop.wav",
    "sound/character/duchess/attack_1.wav",
    "sound/character/duchess/attack_2.wav",
    "sound/character/duchess/attack_3.wav",
    "sound/character/duchess/attack_4.wav",
    "sound/character/duchess/pain1.wav",
    "sound/character/duchess/death.wav",
    "sound/character/duchess/pig_explode.wav",
    "sound/character/duchess/sneeze1.wav",
    "sound/character/duchess/sneeze2.wav",
    "sound/character/duchess/sneeze3.wav",
    "sound/character/duchess/sneeze4.wav",
    "sound/character/duchess/deathfall1.wav",
    "sound/character/duchess/deathfall2.wav",
];
fn settings_path() -> PathBuf {
    crate::preferences::path("audio-settings.txt")
}
const MAX_PCM: usize = 8_000_000; // 32 MB per decoded effect, with a separate level cache budget.

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub music: f32,
    pub effects: f32,
    pub muted: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            music: 0.5,
            effects: 0.7,
            muted: false,
        }
    }
}
impl Settings {
    fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                if key == "muted" {
                    s.muted = value == "true";
                }
                if let Ok(n) = value.parse::<f32>() {
                    if n.is_finite() {
                        match key {
                            "music" => s.music = n.clamp(0., 1.),
                            "effects" => s.effects = n.clamp(0., 1.),
                            _ => {}
                        }
                    }
                }
            }
        }
        s
    }
    pub fn save(self) -> Result<()> {
        std::fs::create_dir_all(settings_path().parent().unwrap())?;
        std::fs::write(
            settings_path(),
            format!(
                "music={}\neffects={}\nmuted={}\n",
                self.music, self.effects, self.muted
            ),
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct Music {
    path: String,
    volume: f32,
    fade: f32,
    looping: bool,
}
#[derive(Clone, Debug)]
struct Speaker {
    path: String,
    origin: Vec3,
    volume: f32,
    radius: f32,
    random: bool,
    delay: (f32, f32),
    chance: f32,
}
#[derive(Default)]
struct LevelAudio {
    music: BTreeMap<String, Music>,
    speakers: Vec<Speaker>,
}

// Small data-only lexer: quoted paths may contain spaces; // starts a comment outside quotes.
fn words(line: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut present = false;
    let mut chars = line.trim_end_matches('\0').chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            quoted = !quoted;
            present = true;
        } else if !quoted && c == '/' && chars.peek() == Some(&'/') {
            break;
        } else if !quoted && c.is_whitespace() {
            if present {
                out.push(std::mem::take(&mut word));
                present = false;
            }
        } else {
            word.push(c);
            present = true;
        }
    }
    ensure!(!quoted, "Unclosed audio-data quote");
    if present {
        out.push(word);
    }
    Ok(out)
}
fn finite(s: &str) -> Result<f32> {
    let n: f32 = s.parse()?;
    ensure!(n.is_finite(), "Non-finite audio value");
    Ok(n)
}
fn music_cues(text: &str) -> Result<BTreeMap<String, Music>> {
    let mut base = String::new();
    let mut cues = BTreeMap::<String, Music>::new();
    // Definitions can precede their options; resolve paths only after all rows.
    for line in text.lines() {
        let w = words(line)?;
        if w.len() == 2 && w[0] == "path" {
            base = w[1].replace('\\', "/");
        } else if w.len() == 2 && w[1].to_lowercase().ends_with(".mp3") {
            cues.entry(w[0].clone()).or_insert(Music {
                path: w[1].clone(),
                volume: 1.,
                fade: 0.,
                looping: false,
            });
        }
    }
    for line in text.lines() {
        let w = words(line)?;
        if let Some(m) = w
            .first()
            .and_then(|s| s.strip_prefix('!'))
            .and_then(|s| cues.get_mut(s))
        {
            match w.get(1).map(String::as_str) {
                Some("loop") => m.looping = true,
                Some("volume") if w.len() == 3 => m.volume = finite(&w[2])?.clamp(0., 1.),
                Some("fadetime") if w.len() == 3 => m.fade = finite(&w[2])?.clamp(0., 30.),
                _ => (),
            }
        }
    }
    for m in cues.values_mut() {
        m.path = format!("{}/{}", base.trim_end_matches('/'), m.path).to_lowercase();
    }
    Ok(cues)
}
#[cfg(test)]
fn music_spec(text: &str) -> Result<Option<Music>> {
    Ok(music_cues(text)?.remove("normal"))
}
fn number(p: &BTreeMap<String, String>, key: &str, default: f32) -> Result<f32> {
    p.get(key).map(|s| finite(s)).unwrap_or(Ok(default))
}
fn speaker_spec(
    p: &BTreeMap<String, String>,
    random: bool,
    sound_key: &str,
) -> Result<Option<Speaker>> {
    let flags = p
        .get("spawnflags")
        .map(|s| s.parse::<u32>())
        .transpose()?
        .unwrap_or(0);
    // Untoggled trigger-only speakers must remain silent without their level scripts.
    if !random && flags & 1 == 0 {
        return Ok(None);
    }
    let Some(path) = p.get(sound_key) else {
        return Ok(None);
    };
    if path.is_empty() {
        return Ok(None);
    }
    let origin = p
        .get("origin")
        .context("Speaker missing origin")?
        .split_whitespace()
        .map(finite)
        .collect::<Result<Vec<_>>>()?;
    ensure!(origin.len() == 3, "Invalid speaker origin");
    let min = number(p, "mindelay", 5.)?.clamp(0.1, 3600.);
    let max = number(p, "maxdelay", 15.)?.clamp(min, 3600.);
    Ok(Some(Speaker {
        path: path.replace('\\', "/").to_lowercase(),
        origin: vec3(origin[0], origin[1], origin[2]),
        volume: number(p, "volume", 1.)?.clamp(0., 1.),
        radius: number(p, "min_dist", 256.)?.clamp(1., 16384.),
        random,
        delay: (min, max),
        chance: number(p, "chance", 1.)?.clamp(0., 1.),
    }))
}
fn sound_specs(text: &str) -> Result<Vec<Speaker>> {
    let mut result = Vec::new();
    for line in text.lines() {
        let w = words(line)?;
        if w.first().map(String::as_str) != Some("spawn") {
            continue;
        }
        ensure!(
            w.len() >= 2 && w.len() % 2 == 0,
            "Invalid sound manager row"
        );
        if !matches!(w[1].as_str(), "TriggerSpeaker" | "RandomSpeaker") {
            continue;
        }
        let p = w[2..]
            .chunks_exact(2)
            .map(|v| (v[0].clone(), v[1].clone()))
            .collect();
        if let Some(s) = speaker_spec(&p, w[1] == "RandomSpeaker", "sound")? {
            result.push(s);
        }
    }
    ensure!(result.len() <= 512, "Too many sound emitters");
    Ok(result)
}
fn relocate_speaker(assets: &Assets, speaker: &mut Speaker) -> bool {
    if assets.contains(&speaker.path) {
        return false;
    }
    let relocated = match speaker.path.as_str() {
        "sound/050800transfer/ambience/ambience_airelement.wav" => {
            Some("sound/ambience/background/ambience_airelement.wav")
        }
        "sound/050800transfer/ambience/ambience_forcefield.wav" => {
            Some("sound/ambience/background/ambience_forcefield.wav")
        }
        _ => None,
    };
    if let Some(path) = relocated.filter(|path| assets.contains(path)) {
        speaker.path = path.into();
        true
    } else {
        false
    }
}
fn add_map_speaker(speakers: &mut Vec<Speaker>, speaker: Speaker, relocated: bool) {
    // The village's obsolete forcefield BSP reference and its sound-manager
    // replacement are 113 units apart. Prefer the authored .snd position/gain.
    // Only migrated legacy references allow this tolerance; separate ordinary
    // emitters (even nearby ones using the same file) must remain independent.
    if !speakers.iter().any(|existing| {
        existing.path == speaker.path
            && existing.random == speaker.random
            && existing.origin.distance(speaker.origin) < if relocated { 128. } else { 1. }
    }) {
        speakers.push(speaker);
    }
}
fn level_spec(assets: &mut Assets, name: &str, map: &Bsp) -> Result<LevelAudio> {
    let mut level = LevelAudio::default();
    // tower3 explicitly reuses hedge1 in the authored map soundtrack command.
    let soundtrack = if name == "tower3" { "hedge1" } else { name };
    let mus = format!("sound/music/{soundtrack}.mus");
    if assets.contains(&mus) {
        level.music = music_cues(std::str::from_utf8(&assets.read(&mus)?)?)?;
    }
    let snd = format!("maps/{name}.snd");
    if assets.contains(&snd) {
        level.speakers = sound_specs(std::str::from_utf8(&assets.read(&snd)?)?)?;
    }
    for speaker in &mut level.speakers {
        relocate_speaker(assets, speaker);
    }
    // .snd is a separate sound-manager layer, alongside map-entity speakers.
    for p in &map.entities {
        let class = p.get("classname").map(String::as_str).unwrap_or("");
        if matches!(class, "sound_speaker" | "sound_randomspeaker") {
            if let Some(mut s) = speaker_spec(p, class == "sound_randomspeaker", "noise")? {
                let relocated = relocate_speaker(assets, &mut s);
                add_map_speaker(&mut level.speakers, s, relocated);
            }
        }
    }
    ensure!(level.speakers.len() <= 512, "Too many sound emitters");
    Ok(level)
}

#[derive(Clone)]
struct Clip {
    samples: Arc<[f32]>,
    rate: u32,
}
fn decode_clip(data: Vec<u8>) -> Result<Clip> {
    let decoder = Decoder::new(Cursor::new(data))?;
    let channels = decoder.channels() as usize;
    let rate = decoder.sample_rate();
    ensure!(
        (1..=2).contains(&channels) && (8000..=96000).contains(&rate),
        "Unsupported audio layout"
    );
    let pcm = decoder
        .convert_samples::<f32>()
        .take(MAX_PCM + 1)
        .collect::<Vec<_>>();
    ensure!(
        !pcm.is_empty() && pcm.len() <= MAX_PCM && pcm.len() % channels == 0,
        "Empty or oversized sound"
    );
    ensure!(
        pcm.iter().all(|s| s.is_finite()),
        "Non-finite decoded sound"
    );
    let mono = pcm
        .chunks_exact(channels)
        .map(|s| s.iter().sum::<f32>() / channels as f32)
        .collect::<Vec<_>>();
    Ok(Clip {
        samples: mono.into(),
        rate,
    })
}
struct Pan {
    left: AtomicU32,
    right: AtomicU32,
}
impl Pan {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            left: AtomicU32::new(1f32.to_bits()),
            right: AtomicU32::new(1f32.to_bits()),
        })
    }
    fn set(&self, left: f32, right: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }
}
struct Pcm {
    clip: Clip,
    index: usize,
    looping: bool,
    pan: Arc<Pan>,
}
impl Iterator for Pcm {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.index / 2 >= self.clip.samples.len() {
            if self.looping {
                self.index = 0;
            } else {
                return None;
            }
        }
        let gain = if self.index % 2 == 0 {
            &self.pan.left
        } else {
            &self.pan.right
        };
        let sample =
            self.clip.samples[self.index / 2] * f32::from_bits(gain.load(Ordering::Relaxed));
        self.index += 1;
        Some(sample)
    }
}
impl Source for Pcm {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.clip.rate
    }
    fn total_duration(&self) -> Option<Duration> {
        if self.looping {
            None
        } else {
            Some(Duration::from_secs_f64(
                self.clip.samples.len() as f64 / self.clip.rate as f64,
            ))
        }
    }
}

// Monitor only our application's mixed samples, never the microphone/system loopback.
#[derive(Default)]
struct MonitorData {
    samples: Mutex<Vec<f32>>,
    count: AtomicU64,
    peak: AtomicU32,
}
struct Monitor<S> {
    source: S,
    data: Arc<MonitorData>,
    block: Vec<f32>,
    record: bool,
}
impl<S> Monitor<S> {
    fn flush(&mut self) {
        self.data
            .count
            .fetch_add(self.block.len() as u64, Ordering::Relaxed);
        let peak = self.block.iter().fold(0f32, |p, s| p.max(s.abs()));
        self.data.peak.fetch_max(peak.to_bits(), Ordering::Relaxed);
        if self.record {
            let mut target = self.data.samples.lock().unwrap();
            let n = self
                .block
                .len()
                .min((44100usize * 2 * 30).saturating_sub(target.len()));
            target.extend_from_slice(&self.block[..n]);
        }
        self.block.clear();
    }
}
impl<S: Source<Item = f32>> Iterator for Monitor<S> {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let sample = acoustics::limit(self.source.next().unwrap_or(0.));
        self.block.push(sample);
        if self.block.len() == 1024 {
            self.flush();
        }
        Some(sample)
    }
}
impl<S: Source<Item = f32>> Source for Monitor<S> {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        44100
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
impl<S> Drop for Monitor<S> {
    fn drop(&mut self) {
        self.flush();
    }
}
struct Playback {
    _stream: OutputStream,
    mixer: Arc<dynamic_mixer::DynamicMixerController<f32>>,
}
impl Playback {
    fn open(data: Arc<MonitorData>, record: bool) -> Result<Self> {
        let (stream, handle) = OutputStream::try_default()?;
        let (mixer, source) = dynamic_mixer::mixer(2, 44100);
        handle.play_raw(Monitor {
            source,
            data,
            block: Vec::with_capacity(1024),
            record,
        })?;
        Ok(Self {
            _stream: stream,
            mixer,
        })
    }
    fn sink(&self) -> Sink {
        let (sink, queue) = Sink::new_idle();
        self.mixer.add(queue);
        sink
    }
}
struct Emitter {
    spec: Speaker,
    clip: Clip,
    pan: Arc<Pan>,
    sink: Sink,
    remaining: f32,
    acoustics: Arc<acoustics::Controls>,
    obstruction: f32,
}

struct Effect {
    sink: Sink,
    pan: Arc<Pan>,
    acoustics: Arc<acoustics::Controls>,
    origin: Option<Vec3>,
    volume: f32,
    obstruction: f32,
}
pub struct LoopCue {
    pub clock: Option<f32>,
    pub id: usize,
    pub path: &'static str,
    pub origin: Vec3,
}
struct Loop {
    clock: Option<f32>,
    path: &'static str,
    effect: Effect,
}
struct Request {
    path: String,
    origin: Option<Vec3>,
    volume: f32,
}
struct FadingMusic {
    sink: Sink,
    volume: f32,
    elapsed: f32,
    duration: f32,
}

pub struct Audio {
    playback: Option<Playback>,
    voice: Option<Sink>,
    music: Option<(Sink, Music)>,
    emitters: Vec<Emitter>,
    effects: Vec<Effect>,
    pending: Vec<Request>,
    missing: BTreeSet<String>,
    cache_samples: usize,
    retiring: Vec<FadingMusic>,
    cues: BTreeMap<String, Music>,
    room: f32,
    acoustic_clock: f32,
    submerged: bool,
    last_sanity: Option<f32>,
    reaction_gate: reaction::Gate,
    reaction: Option<Effect>,
    swim_bed: Option<Sink>,
    loops: BTreeMap<usize, Loop>,
    world_sounds: world::State,
    resolved: u64,
    clips: Vec<Option<Clip>>,
    world_clips: BTreeMap<String, Clip>,
    pub settings: Settings,
    pub status: String,
    fade_elapsed: f32,
    stride: f32,
    foot: usize,
    swim_clock: f32,
    rng: u32,
    monitor: Arc<MonitorData>,
    capture: Option<PathBuf>,
    steps: u64,
    jumps: u64,
    landings: u64,
}
impl Audio {
    pub fn new(disabled: bool, capture: Option<PathBuf>) -> Self {
        let monitor = Arc::new(MonitorData::default());
        if capture.is_some() {
            monitor.samples.lock().unwrap().reserve(44100 * 2 * 30);
        }
        let (playback, status) = if disabled {
            (None, "Sound disabled by launch option".into())
        } else {
            match Playback::open(monitor.clone(), capture.is_some()) {
                Ok(p) => (Some(p), "Sound ready".into()),
                Err(e) => {
                    eprintln!("Audio output unavailable: {e:#}");
                    (None, "No audio output available".into())
                }
            }
        };
        Self {
            playback,
            voice: None,
            music: None,
            emitters: Vec::new(),
            effects: Vec::new(),
            pending: Vec::new(),
            missing: BTreeSet::new(),
            cache_samples: 0,
            retiring: Vec::new(),
            cues: BTreeMap::new(),
            room: 0.,
            acoustic_clock: 0.,
            submerged: false,
            last_sanity: None,
            reaction_gate: reaction::Gate::default(),
            reaction: None,
            swim_bed: None,
            loops: BTreeMap::new(),
            world_sounds: world::State::default(),
            resolved: 0,
            clips: vec![None; 4],
            world_clips: BTreeMap::new(),
            settings: std::fs::read_to_string(settings_path())
                .map(|s| Settings::parse(&s))
                .unwrap_or_default(),
            status,
            fade_elapsed: 0.,
            stride: 0.,
            foot: 0,
            swim_clock: 0.,
            rng: 0x137bc459,
            monitor,
            capture,
            steps: 0,
            jumps: 0,
            landings: 0,
        }
    }
    pub fn load(&mut self, assets: &mut Assets, name: &str, map: &Bsp) {
        self.stop_voice();
        self.retire_music();
        self.world_sounds = world::State::default();
        self.pending.clear();
        self.missing.clear();
        self.cache_samples = 0;
        self.last_sanity = None;
        self.reaction_gate = reaction::Gate::default();
        self.reaction = None;
        self.swim_bed = None;
        self.loops.clear();
        self.acoustic_clock = 0.;
        self.room = 0.;
        self.emitters.clear();
        self.effects.clear();
        self.clips = vec![None; 4];
        self.world_clips.clear();
        self.stride = 0.;
        self.fade_elapsed = 0.;
        self.swim_clock = 0.;
        if self.playback.is_none() {
            return;
        }
        if let Err(e) = self.load_inner(assets, name, map) {
            eprintln!("Audio level error in {name}: {e:#}");
            self.status = "Some level audio could not be loaded".into();
        }
    }
    fn load_inner(&mut self, assets: &mut Assets, name: &str, map: &Bsp) -> Result<()> {
        let spec = level_spec(assets, name, map)?;
        let mut failures = 0;
        self.cues = spec.music;
        self.mood(assets, "normal")?;
        let playback = self.playback.as_ref().unwrap();
        let mut cache: BTreeMap<String, Option<Clip>> = BTreeMap::new();
        let mut cache_samples = 0;
        let mut alice_sounds = events::Model::load(assets, "models/alice.tik")?;
        alice_sounds.retain(&crate::character::CLIPS.map(crate::character::sound_alias));
        for path in spec
            .speakers
            .iter()
            .map(|s| s.path.as_str())
            .chain(EFFECTS)
            .chain(alice_sounds.paths())
            .chain(EXTRA_EFFECTS.iter().copied())
            .chain(WORLD_EFFECTS.iter().copied())
            .chain(crate::dice::SOUNDS)
            .chain(crate::beyond::SOUNDS)
            .chain(crate::ladybug::SOUNDS)
            .chain(
                map.entities
                    .iter()
                    .filter_map(|e| e.get("sound_move").map(String::as_str)),
            )
        {
            if cache.contains_key(path) {
                continue;
            }
            let result = assets.read(path).and_then(decode_clip).and_then(|clip| {
                ensure!(
                    cache_samples + clip.samples.len() <= 32_000_000,
                    "Level audio cache limit reached"
                );
                cache_samples += clip.samples.len();
                Ok(clip)
            });
            match result {
                Ok(clip) => {
                    cache.insert(path.into(), Some(clip));
                }
                Err(e) => {
                    failures += 1;
                    eprintln!("Sound unavailable: {path}: {e:#}");
                    cache.insert(path.into(), None);
                }
            }
        }
        self.cache_samples = cache_samples;
        self.world_clips = cache
            .iter()
            .filter_map(|(path, clip)| clip.clone().map(|c| (path.clone(), c)))
            .collect();
        for speaker in spec.speakers {
            if let Some(clip) = cache.get(&speaker.path).and_then(Clone::clone) {
                let sink = playback.sink();
                sink.set_volume(0.);
                let pan = Pan::new();
                let acoustics = acoustics::Controls::new();
                if !speaker.random {
                    sink.append(acoustics::Filter::new(
                        Pcm {
                            clip: clip.clone(),
                            index: 0,
                            looping: true,
                            pan: pan.clone(),
                        },
                        acoustics.clone(),
                    ));
                }
                let remaining =
                    speaker.delay.0 + random(&mut self.rng) * (speaker.delay.1 - speaker.delay.0);
                self.emitters.push(Emitter {
                    spec: speaker,
                    clip,
                    pan,
                    sink,
                    remaining,
                    acoustics,
                    obstruction: 0.,
                });
            }
        }
        self.clips = EFFECTS
            .iter()
            .map(|p| cache.get(*p).and_then(Clone::clone))
            .collect();
        self.status = if failures == 0 {
            "Sound ready".into()
        } else {
            format!("{failures} sound files unavailable")
        };
        println!(
            "Audio {name}: {} ambient emitters, {} player effects, {failures} unavailable files",
            self.emitters.len(),
            self.clips.iter().flatten().count()
        );
        Ok(())
    }
    pub fn update(&mut self, dt: f32, listener: Vec3, yaw: f32, paused: bool, voice_paused: bool) {
        self.update_clocks(dt, dt, listener, yaw, paused, voice_paused);
    }
    pub fn update_clocks(
        &mut self,
        dt: f32,
        world_dt: f32,
        listener: Vec3,
        yaw: f32,
        paused: bool,
        voice_paused: bool,
    ) {
        let dt = if paused { 0. } else { dt.clamp(0., 0.25) };
        let world_dt = world_dt.clamp(0., dt);
        self.fade_elapsed += dt;
        self.reaction_gate.tick(dt);
        let master = if self.settings.muted { 0. } else { 1. };
        if let Some((sink, music)) = &self.music {
            set_paused(sink, paused);
            let fade = if music.fade > 0. {
                (self.fade_elapsed / music.fade).min(1.)
            } else {
                1.
            };
            let duck = if self.voice.as_ref().is_some_and(|s| !s.empty()) {
                0.4
            } else {
                1.
            };
            sink.set_volume(master * self.settings.music * music.volume * fade * duck);
        }
        for old in &mut self.retiring {
            old.elapsed += dt;
            set_paused(&old.sink, paused);
            old.sink.set_volume(
                master
                    * self.settings.music
                    * old.volume
                    * (1. - old.elapsed / old.duration).max(0.)
                    * if self.voice.as_ref().is_some_and(|s| !s.empty()) {
                        0.4
                    } else {
                        1.
                    },
            );
        }
        self.retiring
            .retain(|old| old.elapsed < old.duration && !old.sink.empty());
        if let Some(sink) = &self.swim_bed {
            sink.set_volume(master * self.settings.effects * 0.24);
            set_paused(sink, paused);
        }
        if let Some(sink) = &self.voice {
            sink.set_volume(master * self.settings.effects);
            set_paused(sink, paused || voice_paused);
        }
        for e in &mut self.emitters {
            let (left, right, attenuation) = spatial(e.spec.origin - listener, yaw, e.spec.radius);
            e.pan.set(left, right);
            e.sink.set_volume(
                master
                    * self.settings.effects
                    * e.spec.volume
                    * attenuation
                    * (1. - 0.7 * e.obstruction)
                    * 0.5,
            );
            set_paused(&e.sink, paused);
            if e.spec.random && !paused {
                // RandomSpeaker schedules server events. Existing PCM, music,
                // speech and loops keep playing; the next emission waits.
                e.remaining -= world_dt;
                if e.remaining <= 0. {
                    e.remaining =
                        e.spec.delay.0 + random(&mut self.rng) * (e.spec.delay.1 - e.spec.delay.0);
                    if random(&mut self.rng) < e.spec.chance
                        && e.sink.empty()
                        && attenuation > 0.001
                    {
                        e.sink.append(acoustics::Filter::new(
                            Pcm {
                                clip: e.clip.clone(),
                                index: 0,
                                looping: false,
                                pan: e.pan.clone(),
                            },
                            e.acoustics.clone(),
                        ));
                    }
                }
            }
        }
        self.effects.retain(|e| !e.sink.empty());
        if self.reaction.as_ref().is_some_and(|e| e.sink.empty()) {
            self.reaction = None;
        }
        for e in self
            .effects
            .iter()
            .chain(self.reaction.iter())
            .chain(self.loops.values().map(|l| &l.effect))
        {
            let (l, r, attenuation) = e.origin.map_or((0.70710677, 0.70710677, 1.), |p| {
                spatial(p - listener, yaw, 384.)
            });
            e.pan.set(l, r);
            e.sink.set_volume(
                master
                    * self.settings.effects
                    * e.volume
                    * attenuation
                    * (1. - 0.7 * e.obstruction)
                    * 0.7,
            );
            set_paused(&e.sink, paused);
        }
    }
    pub fn movement(&mut self, distance: f32, grounded: bool, jumped: bool, landed: bool) {
        if jumped {
            self.play_effect(2);
            self.jumps += 1;
            self.stride = 0.;
        }
        if landed {
            self.play_effect(3);
            self.landings += 1;
            self.stride = 0.;
        }
        // Distance is actual accepted movement, not a held key; walls/flying/teleports stay quiet.
        if grounded && distance < 32. {
            self.stride += distance;
            if self.stride >= 72. {
                self.stride %= 72.;
                self.play_effect(self.foot);
                self.foot ^= 1;
                self.steps += 1;
            }
        } else {
            self.stride = 0.;
        }
    }
    pub fn water(&mut self, _dt: f32, before: u8, now: u8, _speed: f32) {
        if before == 0 && now > 0 {
            self.play_effect(17);
        }
        if before > 0 && now == 0 {
            self.play_effect(18);
        }
        if before < 3 && now == 3 {
            self.world_effect("sound/character/alice/submerge.wav");
        }
        if before == 3 && now < 3 {
            self.world_effect("sound/character/alice/gasp1.wav");
        }
    }
    pub fn weapon_effect(&mut self, path: &str) {
        self.world_effect(path);
    }
    pub fn world_effect(&mut self, path: &str) {
        self.queue(path, None, 1.);
    }
    pub fn feedback(&mut self, feedback: &crate::combat::Feedback) {
        for (path, origin) in &feedback.cue_sounds {
            self.world_effect_at(path, *origin);
        }
        for path in &feedback.sounds {
            self.world_effect(path);
        }
        for (path, origin) in &feedback.spatial_sounds {
            self.world_effect_at(path, *origin);
        }
    }
    pub fn world_effect_at(&mut self, path: &str, origin: Vec3) {
        self.queue(path, Some(origin), 1.);
    }
    pub fn animation(&mut self, cue: events::Cue) {
        // Health changes choose the context-appropriate cry even when Alice is
        // swimming, in first person, or attacking without a pain animation.
        if reaction::kind(&cue.path) != Some(reaction::Kind::Pain) {
            self.queue(&cue.path, None, cue.volume);
        }
    }
    fn queue(&mut self, path: &str, origin: Option<Vec3>, volume: f32) {
        let volume = volume
            * match path {
                "sound/weapon/jackbomb/jackbomb_music.wav" => 0.1,
                "sound/weapon/jackbomb/jackbomb_pop.wav" => 0.2,
                "sound/weapon/jackbomb/jackbomb_explode.wav" => 0.8,
                "sound/weapon/jackbomb/jackbomb_toss.wav" => 0.3,
                "sound/weapon/blunderbuss/bb_fire.wav" => 0.3,
                "sound/weapon/staff/charge_off.wav" => 0.6,
                "sound/weapon/staff/liftoff.wav" | "sound/weapon/staff/explode1.wav" => 0.5,
                _ => 1.,
            };
        if self.playback.is_some() && self.pending.len() < 128 && !path.is_empty() {
            self.pending.push(Request {
                path: path.replace('\\', "/").to_lowercase(),
                origin,
                volume,
            });
        }
    }
    /// Resolve every requested sound, including level-specific sounds outside the preload list.
    /// At the cache limit, transient clips remain owned only by their bounded playback sink.
    pub fn prepare(
        &mut self,
        assets: &mut Assets,
        world: &World,
        listener: Vec3,
        submerged: bool,
        dt: f32,
    ) {
        if self.playback.is_none() {
            self.pending.clear();
            return;
        }
        self.submerged = submerged;
        for request in std::mem::take(&mut self.pending) {
            if !self.world_clips.contains_key(&request.path)
                && !self.missing.contains(&request.path)
            {
                match assets.read(&request.path).and_then(decode_clip) {
                    Ok(clip) => {
                        if self.cache_samples + clip.samples.len() > 32_000_000 {
                            self.resolved += u64::from(self.play_clip(clip, &request));
                            continue;
                        }
                        self.cache_samples += clip.samples.len();
                        self.world_clips.insert(request.path.clone(), clip);
                    }
                    Err(error) => {
                        eprintln!("Requested audio unavailable: {}: {error:#}", request.path);
                        self.missing.insert(request.path.clone());
                        self.status =
                            format!("{} requested sounds unavailable", self.missing.len());
                    }
                }
            }
            if let Some(clip) = self.world_clips.get(&request.path).cloned() {
                self.resolved += u64::from(self.play_clip(clip, &request));
            }
        }
        self.acoustic_clock -= dt;
        if self.acoustic_clock <= 0. {
            self.acoustic_clock = 0.1;
            self.room = acoustics::room(world, listener);
            for e in &mut self.emitters {
                let target = if e.spec.origin.distance(listener) < e.spec.radius * 4. {
                    acoustics::obstruction(world, listener, e.spec.origin)
                } else {
                    0.
                };
                e.obstruction += (target - e.obstruction) * 0.6;
            }
        }
        for e in &self.emitters {
            e.acoustics.set(e.obstruction, submerged, self.room);
        }
        for e in self
            .effects
            .iter_mut()
            .chain(self.reaction.iter_mut())
            .chain(self.loops.values_mut().map(|l| &mut l.effect))
        {
            let target = e
                .origin
                .map_or(0., |p| acoustics::obstruction(world, listener, p));
            e.obstruction += (target - e.obstruction) * (1. - (-dt * 12.).exp());
            e.acoustics.set(e.obstruction, submerged, self.room);
        }
    }
    pub fn world(
        &mut self,
        assets: &mut Assets,
        interactions: &crate::interaction::Interactions,
        mut loops: Vec<LoopCue>,
    ) {
        let clocks = world::collect(interactions, &mut loops);
        if let Some(mood) = interactions.levels.iter().find_map(|s| s.ctl.music_mood()) {
            if let Err(e) = self.mood(assets, mood) { eprintln!("Scene music unavailable: {e:#}"); }
        }
        for (path, origin) in self.world_sounds.update(clocks) {
            // The village introduction declares this cry level-wide. Its camera
            // is high above the playable landing, outside ordinary falloff.
            if path == "sound/character/alice/death_fall.wav" {
                self.world_effect(path);
            } else {
                self.world_effect_at(path, origin);
            }
        }
        self.loops(assets, loops);
    }
    pub fn loops(&mut self, assets: &mut Assets, cues: Vec<LoopCue>) {
        self.loops
            .retain(|id, l| cues.iter().any(|c| c.id == *id && c.path == l.path));
        let Some(playback) = &self.playback else {
            return;
        };
        for cue in cues.into_iter().take(32) {
            let volume = if cue.path == "sound/weapon/staff/beam_loop.wav" {
                0.8
            } else if matches!(
                cue.path,
                "sound/weapon/blunderbuss/bb_loop.wav"
                    | "sound/weapon/jackbomb/jackbomb_breath.wav"
            ) {
                0.5
            } else if cue.id < 1000 {
                0.28
            } else {
                0.6
            };
            if let Some(l) = self.loops.get_mut(&cue.id) {
                l.effect.volume = if cue.clock.is_some() && l.clock == cue.clock {
                    0.
                } else {
                    volume
                };
                l.clock = cue.clock;
                l.effect.origin = Some(cue.origin);
                continue;
            }
            let clip = if let Some(clip) = self.world_clips.get(cue.path) {
                Some(clip.clone())
            } else {
                assets.read(cue.path).and_then(decode_clip).ok()
            };
            if let Some(clip) = clip {
                let index = cue
                    .clock
                    .map_or(0, |clock| (clock.max(0.) * clip.rate as f32) as usize * 2)
                    % (clip.samples.len().max(1) * 2);
                let sink = playback.sink();
                sink.set_volume(0.);
                let pan = Pan::new();
                let acoustics = acoustics::Controls::new();
                sink.append(acoustics::Filter::new(
                    Pcm {
                        clip,
                        index,
                        looping: true,
                        pan: pan.clone(),
                    },
                    acoustics.clone(),
                ));
                self.loops.insert(
                    cue.id,
                    Loop {
                        clock: cue.clock,
                        path: cue.path,
                        effect: Effect {
                            sink,
                            pan,
                            acoustics,
                            origin: Some(cue.origin),
                            volume,
                            obstruction: 0.,
                        },
                    },
                );
            }
        }
    }
    pub fn player_state(
        &mut self,
        assets: &mut Assets,
        sanity: f32,
        swimming: bool,
        underwater: bool,
        drowning: bool,
    ) {
        if let Some(previous) = self.last_sanity {
            if sanity > 0. && previous <= 0. {
                self.reaction = None;
                self.reaction_gate = reaction::Gate::default();
            }
            // Drowning already supplies choke events at its damage contacts.
            // Do not insert a normal pain cry between those contacts.
            if sanity > 0. && sanity < previous && !drowning {
                let path = if underwater {
                    "sound/character/alice/pain_water1.wav"
                } else if previous - sanity >= 20. {
                    "sound/character/alice/pain_strong1.wav"
                } else {
                    "sound/character/alice/pain1.wav"
                };
                self.queue(path, None, 0.7);
            }
        }
        self.last_sanity = Some(sanity);
        if swimming && sanity > 0. {
            if self.swim_bed.is_none() {
                if let Some(playback) = &self.playback {
                    if let Ok(clip) = assets
                        .read("sound/character/alice/swimloop.wav")
                        .and_then(decode_clip)
                    {
                        let sink = playback.sink();
                        sink.set_volume(0.);
                        sink.append(Pcm {
                            clip,
                            index: 0,
                            looping: true,
                            pan: Pan::new(),
                        });
                        self.swim_bed = Some(sink);
                    }
                }
            }
        } else {
            self.swim_bed = None;
        }
    }
    fn retire_music(&mut self) {
        if let Some((sink, cue)) = self.music.take() {
            let fraction = if cue.fade > 0. {
                (self.fade_elapsed / cue.fade).min(1.)
            } else {
                1.
            };
            self.retiring.push(FadingMusic {
                sink,
                volume: cue.volume * fraction,
                elapsed: 0.,
                duration: cue.fade.max(0.25),
            });
            if self.retiring.len() > 2 {
                self.retiring.remove(0);
            }
        }
    }
    fn mood(&mut self, assets: &mut Assets, name: &str) -> Result<()> {
        let Some(cue) = self.cues.get(name).cloned() else {
            return Ok(());
        };
        if self
            .music
            .as_ref()
            .is_some_and(|(_, old)| old.path == cue.path)
        {
            return Ok(());
        }
        let Some(playback) = &self.playback else {
            return Ok(());
        };
        let bytes = assets.read(&cue.path)?;
        let sink = playback.sink();
        sink.set_volume(0.);
        if cue.looping {
            sink.append(Decoder::new_looped(Cursor::new(bytes))?);
        } else {
            sink.append(Decoder::new(Cursor::new(bytes))?);
        }
        self.retire_music();
        self.fade_elapsed = 0.;
        println!(
            "Music {name}: {} (loop={}, fade={}s)",
            cue.path, cue.looping, cue.fade
        );
        self.music = Some((sink, cue));
        Ok(())
    }
    pub fn stop_voice(&mut self) {
        if let Some(sink) = self.voice.take() {
            sink.stop();
        }
    }
    pub fn movie_sound(&self, assets: &mut Assets, path: &str) -> Result<Option<Sink>> {
        let Some(playback) = &self.playback else {
            return Ok(None);
        };
        let sink = playback.sink();
        sink.pause();
        sink.set_volume(if self.settings.muted {
            0.
        } else {
            self.settings.effects
        });
        sink.append(Decoder::new(Cursor::new(assets.read(path)?))?);
        Ok(Some(sink))
    }
    /// Menu cues use the existing output/mixer but continue while gameplay is paused.
    pub fn menu_sound(&self, bytes: Arc<[u8]>) -> Result<Option<Sink>> {
        let Some(playback) = &self.playback else {
            return Ok(None);
        };
        let sink = playback.sink();
        sink.set_volume(if self.settings.muted {
            0.
        } else {
            self.settings.effects
        });
        sink.append(Decoder::new(Cursor::new(bytes))?);
        Ok(Some(sink))
    }
    pub fn speak_from(&mut self, assets: &mut Assets, path: &str, seconds: f32) {
        self.stop_voice();
        let Some(playback) = &self.playback else {
            return;
        };
        match assets
            .read(path)
            .and_then(|b| Ok(Decoder::new(Cursor::new(b))?))
        {
            Ok(clip) => {
                let sink = playback.sink();
                sink.set_volume(if self.settings.muted {
                    0.
                } else {
                    self.settings.effects
                });
                sink.append(clip.skip_duration(Duration::from_secs_f32(seconds.max(0.))));
                self.voice = Some(sink);
                println!("Dialogue voice: {path}");
            }
            Err(e) => eprintln!("Dialogue voice unavailable: {e:#}"),
        }
    }
    fn play_effect(&mut self, index: usize) {
        if let Some(path) = EFFECTS.get(index) {
            self.world_effect(path);
        }
    }
    fn play_clip(&mut self, clip: Clip, request: &Request) -> bool {
        let Some(playback) = &self.playback else {
            return false;
        };
        let reaction = request
            .origin
            .is_none()
            .then(|| reaction::kind(&request.path))
            .flatten();
        if let Some(kind) = reaction {
            if !self
                .reaction_gate
                .accept(kind, clip.samples.len() as f32 / clip.rate as f32)
            {
                return false;
            }
            // Drop/stop the prior reaction before adding its replacement to the
            // mixer; ordinary combat effects remain free to overlap.
            self.reaction = None;
        } else if self.effects.len() >= 32 {
            self.effects.remove(0);
        }
        let sink = playback.sink();
        sink.set_volume(0.); // update applies mute/pause/spatial gain before the first audible sample.
        let pan = Pan::new();
        let acoustics = acoustics::Controls::new();
        acoustics.set(0., self.submerged, self.room);
        sink.append(acoustics::Filter::new(
            Pcm {
                clip,
                index: 0,
                looping: false,
                pan: pan.clone(),
            },
            acoustics.clone(),
        ));
        let effect = Effect {
            sink,
            pan,
            acoustics,
            origin: request.origin,
            volume: request.volume,
            obstruction: 0.,
        };
        if reaction.is_some() {
            self.reaction = Some(effect);
        } else {
            self.effects.push(effect);
        }
        true
    }
    pub fn finish(&mut self) -> Result<()> {
        self.stop_voice();
        self.music = None;
        self.retiring.clear();
        self.swim_bed = None;
        self.loops.clear();
        self.emitters.clear();
        self.effects.clear();
        self.reaction = None;
        self.playback = None;
        let count = self.monitor.count.load(Ordering::Relaxed);
        let peak = f32::from_bits(self.monitor.peak.load(Ordering::Relaxed));
        println!("Audio output: {} stereo frames consumed, peak {:.4}; {} footsteps, {} jumps, {} landings",count/2,peak,self.steps,self.jumps,self.landings);
        if let Some(path) = &self.capture {
            let samples = self.monitor.samples.lock().unwrap();
            ensure!(
                !samples.is_empty(),
                "No audio output was available to capture"
            );
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
            let mut writer = hound::WavWriter::create(
                path,
                hound::WavSpec {
                    channels: 2,
                    sample_rate: 44100,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )?;
            for s in samples.iter() {
                writer.write_sample((s * 32767.) as i16)?;
            }
            writer.finalize()?;
            println!(
                "App audio capture: {} ({:.2}s)",
                path.display(),
                samples.len() as f32 / 88200.
            );
        }
        Ok(())
    }
}
fn set_paused(sink: &Sink, paused: bool) {
    if paused {
        sink.pause()
    } else {
        sink.play()
    }
}
fn random(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    (*state >> 8) as f32 / 16777216.
}
fn spatial(delta: Vec3, yaw: f32, radius: f32) -> (f32, f32, f32) {
    let distance = delta.length();
    let attenuation = (1. - (distance - radius) / (radius * 3.)).clamp(0., 1.);
    let pan = if distance > 0.001 {
        (delta.dot(vec3(yaw.sin(), -yaw.cos(), 0.)) / distance).clamp(-1., 1.)
    } else {
        0.
    };
    (
        (0.5 * (1. - pan)).sqrt(),
        (0.5 * (1. + pan)).sqrt(),
        attenuation,
    )
}

pub fn check(assets: &mut Assets) -> Result<()> {
    regression::check(assets)?;
    world::check(assets)?;
    let mut refs = BTreeSet::new();
    let mut speakers = 0;
    let mut tracks = 0;
    for name in assets.maps() {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let spec =
            level_spec(assets, &name, &map).with_context(|| format!("Audio metadata in {name}"))?;
        for m in spec.music.into_values() {
            ensure!(assets.contains(&m.path), "Missing track {}", m.path);
            refs.insert(m.path);
            tracks += 1;
        }
        for s in spec.speakers {
            refs.insert(s.path);
            speakers += 1;
        }
    }
    refs.extend(EFFECTS.map(String::from));
    refs.extend(
        events::Model::load(assets, "models/alice.tik")?
            .paths()
            .map(String::from),
    );
    refs.extend(EXTRA_EFFECTS.iter().map(|s| s.to_string()));
    refs.extend(crate::beyond::SOUNDS.map(String::from));
    refs.extend(WORLD_EFFECTS.iter().map(|s| s.to_string()));
    refs.extend(crate::dice::SOUNDS.map(String::from));
    refs.extend(crate::ladybug::SOUNDS.map(String::from));
    let mut missing = Vec::new();
    let mut decoded = 0;
    for path in refs {
        if !assets.contains(&path) {
            missing.push(path);
            continue;
        }
        let d = Decoder::new(Cursor::new(assets.read(&path)?)).with_context(|| path.clone())?;
        let rate = d.sample_rate() as usize;
        let channels = d.channels() as usize;
        let limit = rate * channels * 600;
        let mut count = 0;
        let mut peak = 0f32;
        for sample in d.convert_samples::<f32>().take(limit + 1) {
            ensure!(sample.is_finite(), "Invalid sample in {path}");
            count += 1;
            peak = peak.max(sample.abs());
        }
        ensure!(
            count > 0 && count <= limit && peak > 0.,
            "Empty, silent or oversized audio: {path}"
        );
        decoded += 1;
    }
    for path in &missing {
        println!("Missing original sound reference: {path}");
    }
    println!("Audio check: {tracks} level music cues, {speakers} ambient emitters, {decoded} unique referenced files fully decoded, {} missing original references",missing.len());
    Ok(())
}

/// Short audio-only output test: no window, microphone, or system audio capture.
pub fn test_output(assets: &mut Assets, name: &str) -> Result<()> {
    let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
    let mut audio = Audio::new(false, Some(PathBuf::from("private/audio-output-check.wav")));
    ensure!(
        audio.playback.is_some(),
        "This session has no usable audio output device"
    );
    audio.settings = Settings::default(); // Does not save/alter the user's preferences.
    audio.load(assets, name, &map);
    let (eye, yaw) = map.spawn();
    let world = World::from_bsp(&map)?;
    let model = events::Model::load(assets, "models/alice.tik")?;
    let mut marks = Vec::<(&str, usize)>::new();
    for tick in 0..440 {
        let mark = match tick {
            0 => Some("level"),
            20 => Some("movement"),
            56 => Some("combat"),
            80 => Some("mute"),
            100 => Some("resume"),
            120 => Some("pause"),
            140 => Some("dialogue"),
            160 => Some("crossfade"),
            200 => Some("underwater"),
            240 => Some("climb-death"),
            280 => Some("alternate-music"),
            320 => Some("normal-music"),
            360 => Some("loud-combat"),
            400 => Some("release"),
            _ => None,
        };
        if let Some(label) = mark {
            let frame = audio.monitor.count.load(Ordering::Relaxed) as usize / 2;
            marks.push((label, frame));
            println!("Audio probe {label} at frame {frame}");
        }
        if tick == 60 {
            audio.world_effect("sound/world/mover/bookcase_fall_2.wav");
        }
        if tick == 56 {
            audio.world_effect("sound/world/mover/bookcase_fall_1.wav");
            audio.world_effect("sound/world/mover/roll_1.wav");
            for (i, path) in [
                "sound/weapon/knife/knife_swing1.wav",
                "sound/weapon/cards/cards_hit_flesh1.wav",
                "sound/weapon/mallet/mallet_hit_flesh1.wav",
                "sound/character/boojum/attack.wav",
                "sound/character/duchess/attack_1.wav",
            ]
            .into_iter()
            .enumerate()
            {
                audio.world_effect_at(
                    path,
                    eye + vec3(0., if i % 2 == 0 { -100. } else { 100. }, 0.),
                );
            }
        }
        if tick == 100 {
            audio.loops(
                assets,
                vec![LoopCue {
                    id: 1000,
                    clock: None,
                    path: "sound/world/machine/mine_lift1.wav",
                    origin: eye,
                }],
            );
        }
        if tick == 140 {
            audio.loops(assets, Vec::new());
            audio.speak_from(assets, "sound/character/cheshire_cat/vo/cat001.wav", 0.);
        }
        if tick == 200 {
            audio.water(0.05, 0, 3, 100.);
            audio.player_state(assets, 100., true, true, false);
            audio.loops(
                assets,
                vec![LoopCue {
                    id: 0,
                    clock: None,
                    path: "sound/weapon/mallet/mallet_ball_loop.wav",
                    origin: eye + Vec3::Y * 100.,
                }],
            );
        }
        if tick == 220 {
            audio.player_state(assets, 80., true, true, false);
        }
        if tick == 240 {
            audio.water(0.05, 3, 0, 0.);
            audio.player_state(assets, 80., false, false, false);
            audio.loops(assets, Vec::new());
            for cue in model.between(
                "climb_48",
                events::Span {
                    start: 0.,
                    end: 2.,
                    duration: 2.,
                    frame_time: 0.05,
                    looping: false,
                    entered: true,
                },
            ) {
                audio.animation(cue);
            }
        }
        if tick == 260 {
            for cue in model.between(
                "death_faint",
                events::Span {
                    start: 0.,
                    end: 0.1,
                    duration: 2.,
                    frame_time: 0.05,
                    looping: false,
                    entered: true,
                },
            ) {
                audio.animation(cue);
            }
        }
        if tick == 280 {
            audio.cues = music_cues(std::str::from_utf8(
                &assets.read("sound/music/funhouse.mus")?,
            )?)?;
            audio.mood(assets, "suspense")?;
            ensure!(
                !audio.retiring.is_empty(),
                "No outgoing track during crossfade"
            );
        }
        if tick == 320 {
            audio.mood(assets, "normal")?;
        }
        if (360..380).contains(&tick) && tick % 4 == 0 {
            for _ in 0..12 {
                audio.world_effect("sound/weapon/mallet/mallet_hit_world1.wav");
            }
        }

        if (20..40).contains(&tick) {
            audio.movement(8., true, false, false);
        }
        if tick == 40 {
            audio.movement(0., false, true, false);
        }
        if tick == 52 {
            audio.movement(0., true, false, true);
        }
        if tick == 80 || tick == 100 {
            audio.settings.muted = tick == 80;
            println!(
                "Output test mute={} at frame {}",
                audio.settings.muted,
                audio.monitor.count.load(Ordering::Relaxed) / 2
            );
        }
        if tick == 120 || tick == 140 {
            println!(
                "Output test pause={} at frame {}",
                tick == 120,
                audio.monitor.count.load(Ordering::Relaxed) / 2
            );
        }
        if tick == 160 && name == "skool1" {
            let next = Bsp::parse(&assets.read("maps/skool2.bsp")?)?;
            audio.load(assets, "skool2", &next);
        }
        audio.prepare(assets, &world, eye, (200..240).contains(&tick), 0.05);
        audio.update(0.05, eye, yaw, (120..140).contains(&tick), false);
        std::thread::sleep(Duration::from_millis(50));
    }
    marks.push((
        "end",
        audio.monitor.count.load(Ordering::Relaxed) as usize / 2,
    ));
    audio.finish()?;
    let samples = audio.monitor.samples.lock().unwrap();
    let mut report = Vec::new();
    for pair in marks.windows(2) {
        let start = (pair[0].1 + 11025).min(samples.len() / 2);
        let end = pair[1].1.saturating_sub(2205).min(samples.len() / 2);
        ensure!(end > start, "Empty capture interval {}", pair[0].0);
        let slice = &samples[start * 2..end * 2];
        let rms = (slice.iter().map(|s| s * s).sum::<f32>() / slice.len() as f32).sqrt();
        let peak = slice.iter().fold(0f32, |p, s| p.max(s.abs()));
        let silent = matches!(pair[0].0, "mute" | "pause");
        ensure!(
            if silent {
                peak < 0.00001
            } else {
                rms > 0.00001
            },
            "Unexpected output in {}: rms={rms} peak={peak}",
            pair[0].0
        );
        ensure!(peak < 0.99, "Clipped mix in {}", pair[0].0);
        report.push(serde_json::json!({"phase": pair[0].0, "start_frame": start, "end_frame": end, "rms": rms, "peak": peak}));
    }
    std::fs::write(
        "private/gameplay-audio-mixing.json",
        serde_json::to_string_pretty(&report)?,
    )?;
    ensure!(
        audio.missing.is_empty(),
        "Some gameplay requests were silent: {:?}",
        audio.missing
    );
    ensure!(
        audio.resolved >= 70,
        "Too few combat/animation events resolved: {}",
        audio.resolved
    );
    println!(
        "Gameplay audio: {} sound requests resolved; {} measured playback intervals passed",
        audio.resolved,
        report.len()
    );
    ensure!(
        audio.steps == 2 && audio.jumps == 1 && audio.landings == 1,
        "Movement sound events did not fire"
    );
    ensure!(
        audio.monitor.count.load(Ordering::Relaxed) > 88200
            && audio.monitor.peak.load(Ordering::Relaxed) > 0,
        "No audible samples consumed by output device"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn watch_keeps_pcm_playing_but_defers_random_speaker_events() {
        let mut audio = Audio::new(true, None);
        let (sink, _output) = Sink::new_idle();
        sink.append(rodio::buffer::SamplesBuffer::new(
            1,
            22050,
            vec![0.2f32; 22050],
        ));
        audio.emitters.push(Emitter {
            spec: Speaker {
                path: "fixture.wav".into(),
                origin: Vec3::ZERO,
                volume: 1.,
                radius: 500.,
                random: true,
                delay: (1., 1.),
                chance: 1.,
            },
            clip: Clip {
                samples: vec![0.2; 22050].into(),
                rate: 22050,
            },
            pan: Pan::new(),
            sink,
            remaining: 0.1,
            acoustics: acoustics::Controls::new(),
            obstruction: 0.,
        });
        audio.update_clocks(0.2, 0., Vec3::ZERO, 0., false, false);
        assert_eq!(audio.emitters[0].remaining, 0.1);
        assert!(!audio.emitters[0].sink.is_paused());
        audio.update_clocks(0.2, 0.05, Vec3::ZERO, 0., false, false);
        assert!((audio.emitters[0].remaining - 0.05).abs() < 1e-6);
        audio.update_clocks(0.2, 0.2, Vec3::ZERO, 0., true, false);
        assert!((audio.emitters[0].remaining - 0.05).abs() < 1e-6);
        assert!(audio.emitters[0].sink.is_paused());
        audio.update_clocks(0.2, 0.1, Vec3::ZERO, 0., false, false);
        assert_eq!(audio.emitters[0].remaining, 1.);
    }
    #[test]
    fn alternate_music_preserves_authored_loop_and_fade_settings() -> Result<()> {
        let cues = music_cues("path sound/music/final\nnormal queen.mp3\n!normal loop\n!normal fadetime 1\nsuspense finale.mp3\n!suspense volume .7\n!suspense fadetime 2")?;
        assert!(cues["normal"].looping);
        assert!(!cues["suspense"].looping);
        assert_eq!(cues["suspense"].fade, 2.);
        assert_eq!(cues["suspense"].volume, 0.7);
        Ok(())
    }
    #[test]
    fn outgoing_music_obeys_crossfade_pause_mute_and_cleanup() {
        let mut audio = Audio::new(true, None);
        audio.settings = Settings::default();
        let (sink, _queue) = Sink::new_idle();
        sink.append(rodio::buffer::SamplesBuffer::new(
            1,
            22050,
            vec![0.2f32; 22050],
        ));
        audio.music = Some((
            sink,
            Music {
                path: String::new(),
                volume: 1.,
                fade: 1.,
                looping: true,
            },
        ));
        audio.fade_elapsed = 1.;
        audio.retire_music();
        audio.update(0.25, Vec3::ZERO, 0., false, false);
        assert!((audio.retiring[0].sink.volume() - 0.375).abs() < 0.001);
        audio.update(0.25, Vec3::ZERO, 0., true, false);
        assert_eq!(audio.retiring[0].elapsed, 0.25);
        assert!(audio.retiring[0].sink.is_paused());
        audio.settings.muted = true;
        audio.update(0.25, Vec3::ZERO, 0., false, false);
        assert_eq!(audio.retiring[0].sink.volume(), 0.);
        audio.update(0.25, Vec3::ZERO, 0., false, false);
        audio.update(0.25, Vec3::ZERO, 0., false, false);
        assert!(audio.retiring.is_empty());
    }
    #[test]
    fn dialogue_pauses_independently_mutes_and_releases_music_ducking() {
        let mut audio = Audio::new(true, None);
        audio.settings = Settings::default();
        let (voice, _voice_output) = Sink::new_idle();
        voice.append(rodio::buffer::SamplesBuffer::new(
            1,
            22050,
            vec![0.25f32; 22050],
        ));
        let (music, _music_output) = Sink::new_idle();
        music.append(rodio::buffer::SamplesBuffer::new(
            1,
            22050,
            vec![0.25f32; 22050],
        ));
        audio.voice = Some(voice);
        audio.music = Some((
            music,
            Music {
                path: String::new(),
                volume: 1.,
                fade: 0.,
                looping: false,
            },
        ));
        audio.update(0.05, Vec3::ZERO, 0., false, true);
        assert!(audio.voice.as_ref().unwrap().is_paused());
        assert!(!audio.music.as_ref().unwrap().0.is_paused());
        assert!((audio.music.as_ref().unwrap().0.volume() - 0.2).abs() < 0.001);
        audio.settings.muted = true;
        audio.update(0.05, Vec3::ZERO, 0., false, false);
        assert!(!audio.voice.as_ref().unwrap().is_paused());
        assert_eq!(audio.voice.as_ref().unwrap().volume(), 0.);
        audio.settings.muted = false;
        audio.update(0.05, Vec3::ZERO, 0., true, false);
        assert!(audio.voice.as_ref().unwrap().is_paused());
        assert!(audio.music.as_ref().unwrap().0.is_paused());
        audio.stop_voice();
        audio.update(0.05, Vec3::ZERO, 0., false, false);
        assert!(audio.voice.is_none());
        assert!((audio.music.as_ref().unwrap().0.volume() - 0.5).abs() < 0.001);
    }
    #[test]
    fn parses_music_and_paths_with_spaces() -> Result<()> {
        let m = music_spec("path sound/music\r\nnormal song.mp3\n!normal loop\n!normal volume .8\n!normal fadetime 5")?.unwrap();
        assert_eq!(m.path, "sound/music/song.mp3");
        assert!(m.looping);
        assert_eq!(m.volume, 0.8);
        let s = sound_specs("spawn TriggerSpeaker origin \"1 2 3\" spawnflags 1 sound \"sound/a b.wav\" // trailing\nspawn TriggerSpeaker origin \"0 0 0\" sound off.wav")?;
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].path, "sound/a b.wav");
        assert!(sound_specs("spawn RandomSpeaker origin \"NaN 0 0\" sound a.wav").is_err());
        assert!(words("\"unterminated").is_err());
        Ok(())
    }
    #[test]
    fn migrated_speaker_reuses_sound_manager_but_keeps_distinct_sources() -> Result<()> {
        let make = |x| {
            sound_specs(&format!(
                "spawn TriggerSpeaker origin \"{x} 0 0\" spawnflags 1 sound sound/ambient.wav"
            ))
            .map(|mut s| s.remove(0))
        };
        let mut speakers = vec![make(0)?];
        add_map_speaker(&mut speakers, make(113)?, true);
        assert_eq!(speakers.len(), 1);
        add_map_speaker(&mut speakers, make(113)?, false);
        assert_eq!(speakers.len(), 2);
        add_map_speaker(&mut speakers, make(600)?, true);
        assert_eq!(speakers.len(), 3);
        add_map_speaker(&mut speakers, make(600)?, true);
        assert_eq!(speakers.len(), 3);
        Ok(())
    }
    #[test]
    fn settings_reject_nan_and_clamp() {
        let s = Settings::parse("music=NaN\neffects=4\nmuted=true");
        assert_eq!(s.music, 0.5);
        assert_eq!(s.effects, 1.);
        assert!(s.muted);
    }
    #[test]
    fn empty_speaker_reference_does_not_shift_key_value_pairs() -> Result<()> {
        assert_eq!(words("sound \"\" next 1")?, vec!["sound", "", "next", "1"]);
        assert!(sound_specs(
            "spawn TriggerSpeaker origin \"0 0 0\" spawnflags 1 sound \"\" _addtosoundmanager 0"
        )?
        .is_empty());
        Ok(())
    }
    #[test]
    fn sink_controls_silence_and_resume_without_resetting() {
        let (sink, mut output) = Sink::new_idle();
        sink.append(Pcm {
            clip: Clip {
                samples: Arc::from([0.25; 64]),
                rate: 22050,
            },
            index: 0,
            looping: true,
            pan: Pan::new(),
        });
        assert!(output.by_ref().take(4000).any(|s| s > 0.));
        sink.pause();
        output.by_ref().take(2000).for_each(drop);
        let paused = sink.get_pos();
        assert!(output.by_ref().take(4000).all(|s| s == 0.));
        assert_eq!(sink.get_pos(), paused);
        sink.set_volume(0.);
        sink.play();
        output.by_ref().take(2000).for_each(drop);
        assert!(output.by_ref().take(4000).all(|s| s == 0.));
        assert!(sink.get_pos() > paused);
        sink.set_volume(1.);
        output.by_ref().take(2000).for_each(drop);
        assert!(output.by_ref().take(4000).all(|s| s > 0.));
    }
    #[test]
    fn panning_rotates_and_falls_off_with_distance() {
        let (l, r, a) = spatial(vec3(0., -10., 0.), 0., 100.);
        assert_eq!(l, 0.);
        assert_eq!(r, 1.);
        assert_eq!(a, 1.);
        let (l, r, _) = spatial(vec3(0., -10., 0.), std::f32::consts::PI, 100.);
        assert!(l > 0.99 && r < 0.01);
        assert_eq!(spatial(vec3(500., 0., 0.), 0., 100.).2, 0.);
    }
    #[test]
    fn pcm_loops_and_preserves_stereo_pairing() {
        let pan = Pan::new();
        pan.set(1., 0.5);
        let clip = Clip {
            samples: Arc::from([0.25, 0.5]),
            rate: 22050,
        };
        let mut p = Pcm {
            clip,
            index: 0,
            looping: true,
            pan,
        };
        assert_eq!(
            p.by_ref().take(6).collect::<Vec<_>>(),
            vec![0.25, 0.125, 0.5, 0.25, 0.25, 0.125]
        );
        p.looping = false;
        assert_eq!(p.count(), 2);
    }
    #[test]
    fn wav_decode_and_bounded_monitor() -> Result<()> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut w = hound::WavWriter::new(
                &mut bytes,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 22050,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )?;
            for s in [8192i16, -16384] {
                w.write_sample(s)?;
            }
            w.finalize()?;
        }
        let clip = decode_clip(bytes.into_inner())?;
        assert_eq!(clip.samples.len(), 2);
        assert!((clip.samples[0] - 0.25).abs() < 0.001);
        assert!(decode_clip(vec![0; 44]).is_err());
        let data = Arc::new(MonitorData::default());
        {
            let src = rodio::buffer::SamplesBuffer::new(2, 44100, vec![2f32, -2., 0.25]);
            let mut m = Monitor {
                source: src,
                data: data.clone(),
                block: Vec::new(),
                record: true,
            };
            assert_eq!(
                m.by_ref().take(4).collect::<Vec<_>>(),
                vec![acoustics::limit(2.), acoustics::limit(-2.), 0.25, 0.]
            );
        }
        assert_eq!(data.count.load(Ordering::Relaxed), 4);
        assert_eq!(data.samples.lock().unwrap().len(), 4);
        Ok(())
    }
}

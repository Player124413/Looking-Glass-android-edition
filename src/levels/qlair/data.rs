use super::*;
use crate::{
    cinematic::Track,
    skeletal::{Animation, Definition, Skeleton, Transform},
};
use std::collections::BTreeMap;

pub(super) const MODELS: &[&str] = &[
    "c_queen1",
    "c_queen1_bigtent",
    "c_q2_body",
    "c_q2_halo",
    "c_q2_t01",
    "c_q2_t02",
    "c_q2_t03",
    "c_q2_t04",
    "alice",
];
pub(super) struct Rig {
    pub def: Definition,
    pub skeleton: Skeleton,
    pub clips: BTreeMap<String, Animation>,
    pub attacks: BTreeMap<String, Vec<(f32, Vec<String>)>>,
    pub events: crate::animation_events::Model,
    pub sounds: BTreeMap<String, &'static str>,
    pub lights: Vec<(String, Option<String>, bool, crate::lighting::Light)>,
    scales: BTreeMap<String, Vec<(f32, usize, f32)>>,
}
impl Rig {
    fn load(a: &mut Assets, name: &str) -> Result<Self> {
        let def = Definition::load(a, &format!("models/{name}.tik"))?;
        let skeleton = Skeleton::parse(&a.read(&format!("{}/{}", def.path, def.model))?)?;
        let mut clips = BTreeMap::new();
        for (clip_name, file) in &def.animations {
            if file.ends_with(".ska")
                && (name != "alice"
                    || [
                        "ready",
                        "walk",
                        "idle_base_02",
                        "use_recharger",
                        "pain_knockdown",
                        "idle_base_02_kneel",
                        "kneel_idle",
                        "kneel_shakeno",
                        "kneel_2_weep",
                        "weep_loop",
                        "weep_sobbing",
                        "weep_2_kneel",
                        "kneel_2_base_02",
                    ]
                    .contains(&clip_name.as_str()))
            {
                let (clip, _) = Animation::parse_tags(
                    &a.read(&format!("{}/{file}", def.path))?,
                    &skeleton.unskinned_tags(),
                )?;
                clips.insert(clip_name.clone(), clip);
            }
        }
        // Read only the reviewed, declarative attack events. Never execute source scripts.
        let text = String::from_utf8_lossy(&a.read(&format!("models/{name}.tik"))?).into_owned();
        let mut attacks: BTreeMap<String, Vec<(f32, Vec<String>)>> = BTreeMap::new();
        let mut scales: BTreeMap<String, Vec<(f32, usize, f32)>> = BTreeMap::new();
        let (mut depth, mut server, mut clip) = (0, false, String::new());
        for row in crate::materials::lines(&text) {
            if row.len() > 1 && row[1].ends_with(".ska") {
                clip = row[0].clone();
            }
            if row[0] == "server" {
                server = true;
            }
            if name == "c_q2_t02" && depth == 3 && row.len() >= 4 && row[1] == "setcontrollerscale"
            {
                if let (Ok(frame), Ok(controller), Ok(value), Some(anim)) = (
                    row[0].parse::<f32>(),
                    row[2].parse::<usize>(),
                    row[3].parse::<f32>(),
                    clips.get(&clip),
                ) {
                    if (1..=3).contains(&controller) && (0. ..=1.).contains(&value) {
                        scales.entry(clip.clone()).or_default().push((
                            frame * anim.frame_time,
                            controller - 1,
                            value,
                        ));
                    }
                }
            }
            if server
                && depth == 3
                && row.len() > 2
                && matches!(row[1].as_str(), "proj" | "q2_proj" | "beamattack" | "melee")
            {
                if let Some(anim) = clips.get(&clip) {
                    if row[0] == "every" {
                        let mut f = 0.;
                        while f < anim.duration() {
                            attacks
                                .entry(clip.clone())
                                .or_default()
                                .push((f, row[1..].to_vec()));
                            f += anim.frame_time;
                        }
                    } else if let Ok(frame) = row[0].parse::<f32>() {
                        attacks
                            .entry(clip.clone())
                            .or_default()
                            .push((frame * anim.frame_time, row[1..].to_vec()));
                    }
                }
            }
            for word in &row {
                if word == "{" {
                    depth += 1;
                } else if word == "}" {
                    depth -= 1;
                    if depth < 3 {
                        server = false;
                    }
                }
            }
        }
        let events = crate::animation_events::Model::parse(&text)?;
        let mut sounds = BTreeMap::new();
        for event in events
            .clips
            .iter()
            .filter(|(clip, _)| clips.contains_key(*clip))
            .flat_map(|(_, events)| events)
        {
            if let crate::animation_events::Command::Sound { path, .. } = &event.command {
                // The supplied Queen definition references a nonexistent wounded
                // preparation recording. Use the existing normal preparation cue.
                let resolved = if path == "sound/character/queen/wounded_2_laser.wav" {
                    "sound/character/queen/ready_2_laser.wav"
                } else {
                    path
                };
                ensure!(a.contains(resolved), "Missing finale sound {resolved}");
                sounds.insert(path.clone(), sound_name(resolved));
            }
        }
        Ok(Self {
            def,
            skeleton,
            clips,
            attacks,
            events,
            sounds,
            lights: crate::particles::attached_lights(a, name)?,
            scales,
        })
    }
    pub fn duration(&self, clip: &str) -> f32 {
        self.clips.get(clip).map_or(1., Animation::duration)
    }
    pub fn scales(&self, clip: &str, time: f32) -> Vec<(&'static str, f32)> {
        let Some(events) = self.scales.get(clip) else {
            return Vec::new();
        };
        let mut scales = [1.; 3];
        for &(at, controller, scale) in events {
            if at <= time {
                scales[controller] = scale;
            }
        }
        ["bip01 spine1", "bip01 l upperarm", "bip01 r upperarm"]
            .into_iter()
            .zip(scales)
            .collect()
    }
    pub fn tag(&self, tag: &str, clip: &str, t: f32, pose: Transform, looping: bool) -> Transform {
        self.tag_watched(tag, clip, t, pose, looping, &Default::default())
    }
    pub fn tag_watched(
        &self,
        tag: &str,
        clip: &str,
        t: f32,
        pose: Transform,
        looping: bool,
        watch: &crate::facial::Watch,
    ) -> Transform {
        let Some(bone) = self
            .skeleton
            .bones
            .iter()
            .position(|b| b.name.eq_ignore_ascii_case(tag))
        else {
            return pose;
        };
        let Some(anim) = self.clips.get(clip) else {
            return pose;
        };
        let mut local = anim.sample(t, looping);
        watch.apply(&self.skeleton, &mut local);
        let mut global = self.skeleton.global_pose(&local);
        let roots = self
            .scales(clip, t)
            .into_iter()
            .filter_map(|(name, scale)| {
                self.skeleton
                    .bones
                    .iter()
                    .position(|b| b.name.eq_ignore_ascii_case(name))
                    .map(|n| (n, scale))
            })
            .collect::<Vec<_>>();
        crate::skeletal::scale_pose(&self.skeleton, &mut global, &roots);
        Transform {
            translation: pose.point(global[bone].translation * self.def.scale),
            rotation: pose.rotation * global[bone].rotation,
        }
    }
}
// Feedback uses static sound keys; intern each validated asset path only once.
fn sound_name(path: &str) -> &'static str {
    static NAMES: std::sync::OnceLock<std::sync::Mutex<BTreeMap<String, &'static str>>> =
        std::sync::OnceLock::new();
    let mut names = NAMES.get_or_init(Default::default).lock().unwrap();
    *names
        .entry(path.into())
        .or_insert_with(|| Box::leak(path.to_owned().into_boxed_str()))
}
pub(super) fn origin(e: &super::super::Entity) -> Vec3 {
    e.get("origin")
        .and_then(|s| crate::interaction::vector(s))
        .unwrap_or_default()
}
pub(super) fn pose(e: &super::super::Entity) -> Transform {
    Transform {
        translation: origin(e),
        rotation: Quat::from_rotation_z(
            e.get("angle")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(0.)
                .to_radians(),
        ),
    }
}
pub(super) struct Brush {
    pub name: String,
    pub model: usize,
    pub base: Transform,
}
pub(super) struct Data {
    pub rigs: BTreeMap<String, Rig>,
    pub points: BTreeMap<String, Transform>,
    pub cameras: BTreeMap<String, Track>,
    pub brushes: Vec<Brush>,
    pub emitters: Vec<(crate::entity::Id, String)>,
    pub intro_line: f32,
    pub speech: f32,
    pub projectiles: BTreeMap<String, projectile::Spec>,
    pub popup_frame: f32,
    pub popup_duration: f32,
    pub head_strobe: Vec<Vec3>,
}
impl Data {
    pub fn load(a: &mut Assets, map: &Bsp) -> Result<Self> {
        let popup = Definition::load(a, "models/c_queen1_popup.tik")?;
        let popup = crate::tan::Model::parse(
            &a.read(&format!("{}/{}", popup.path, popup.animations["popup"]))?,
        )?;
        let style = crate::texture::decode(a, "gfx/light/LS_strobe.tga")?;
        let head_strobe = style
            .pixels
            .chunks_exact(4)
            .take(style.width as usize)
            .map(|p| vec3(p[0] as f32, p[1] as f32, p[2] as f32) / 255.)
            .collect();
        let points = map
            .entities
            .iter()
            .filter_map(|e| e.get("targetname").map(|n| (n.clone(), pose(e))))
            .collect::<BTreeMap<_, _>>();
        for n in [
            "throne",
            "alice_start_pos1",
            "alice_fight_queen",
            "bitch2_pos1",
            "bitch_pos2",
            "point1",
            "point2",
            "point3",
            "point4",
            "point5",
        ] {
            ensure!(points.contains_key(n), "Missing finale marker {n}");
        }
        let mut rigs = BTreeMap::new();
        for &name in MODELS {
            rigs.insert(name.into(), Rig::load(a, name)?);
        }
        let mut cameras = BTreeMap::new();
        for n in [
            "path1",
            "path2",
            "q1dead",
            "grab1",
            "q1deadpx1",
            "bodybounce",
            "getpower",
            "slitherp1",
            "alicefallp1",
            "bodytx1",
            "bodytx2",
            "bodytx3",
            "bodytx4",
            "bodyt2",
            "showq2",
        ] {
            cameras.insert(n.into(), Track::load(a, &format!("qlair_{n}"))?);
        }
        let mut brushes = Vec::new();
        for e in &map.entities {
            if e.get("classname").is_some_and(|s| s == "script_object") {
                brushes.push(Brush {
                    name: e["targetname"].clone(),
                    model: e["model"].trim_start_matches('*').parse()?,
                    base: pose(e),
                });
            }
        }
        let wav = |a: &mut Assets, n: &str| -> Result<f32> {
            let b = a.read(n)?;
            let r = hound::WavReader::new(std::io::Cursor::new(b))?;
            Ok(r.duration() as f32 / r.spec().sample_rate as f32)
        };
        let intro_line = wav(a, "sound/character/alice/vo/alcz5003.wav")? + 1.;
        let mut speech = 0.;
        for n in ["qnr002", "qnr003", "qnr004"] {
            speech += wav(a, &format!("sound/character/queen/vo/{n}.wav"))? + 0.3;
        }
        let mut projectiles = BTreeMap::new();
        for events in rigs.values().flat_map(|r| r.attacks.values()).flatten() {
            let row = &events.1;
            if matches!(row[0].as_str(), "proj" | "q2_proj")
                && row.len() >= 3
                && !projectiles.contains_key(&row[2])
            {
                projectiles.insert(row[2].clone(), projectile::Spec::load(a, &row[2])?);
            }
        }
        let emitters = map
            .entities
            .iter()
            .enumerate()
            .filter_map(|(id, e)| {
                e.get("targetname")
                    .filter(|n| {
                        ["lightning_front", "lightning_back", "alice_magic_power"]
                            .contains(&n.as_str())
                    })
                    .map(|n| (crate::entity::Id(id), n.clone()))
            })
            .collect();
        Ok(Self {
            rigs,
            points,
            cameras,
            brushes,
            emitters,
            intro_line,
            speech,
            projectiles,
            popup_frame: popup.frame_time,
            head_strobe,
            popup_duration: popup
                .surfaces
                .first()
                .map_or(1., |s| s.frames.len() as f32 * popup.frame_time),
        })
    }
    pub fn intro_break(&self) -> f32 {
        6. + self.intro_line.max(2.) + 5.
    }
    pub fn intro_end(&self) -> f32 {
        self.intro_break() + 3.5
    }
    pub fn pull_end(&self) -> f32 {
        23.5 + self.rigs["c_queen1_bigtent"].duration("death_pullthrough") + 1.
    }
    pub fn speech_start(&self) -> f32 {
        self.pull_end() + 0.6 + self.rigs["alice"].duration("pain_knockdown") / 0.8 + 9.5
    }
    pub fn reveal(&self) -> f32 {
        self.speech_start()
            + self.speech
            + 0.5
            + self.rigs["alice"].duration("kneel_2_base_02")
            + 8.7
    }
    pub fn birth_end(&self) -> f32 {
        self.reveal() + 10.5
    }
}

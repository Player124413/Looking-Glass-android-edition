//! Reviewed presentation commands from original map scripts. No script execution.
use crate::{assets::Assets, bsp::Bsp};
use anyhow::{ensure, Context, Result};
use macroquad::prelude::*;
use std::collections::BTreeMap;

pub fn body(text: &str, name: &str) -> String {
    let clean = text
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(open) = clean
        .match_indices(&format!("void {name}("))
        .find_map(|(start, _)| {
            let tail = &clean[start..];
            let open = tail.find('{')?;
            (!tail[..open].contains(';')).then_some(start + open)
        })
    else {
        return String::new();
    };
    let mut depth = 1;
    for (i, c) in clean[open + 1..].char_indices() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
        }
        if depth == 0 {
            return clean[open + 1..open + 1 + i].to_owned();
        }
    }
    String::new()
}
fn script(assets: &mut Assets, name: &str) -> Result<String> {
    let path = format!("maps/{name}.scr");
    Ok(if assets.contains(&path) {
        String::from_utf8_lossy(&assets.read(&path)?).replace('\r', "")
    } else {
        String::new()
    })
}
fn numbers(s: &str) -> Vec<f32> {
    s.split(|c: char| c.is_whitespace() || "(),'\";".contains(c))
        .filter_map(|v| v.parse::<f32>().ok())
        .collect()
}
pub fn sky_target(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (target, args) = l.trim().strip_prefix('$')?.split_once(".rendereffects")?;
        args.contains("+skyorigin").then(|| target.to_owned())
    })
}
fn fog(text: &str, command: &str) -> Option<(Vec4, f32)> {
    text.lines().find_map(|l| {
        let rest = l.trim().strip_prefix(command)?;
        let n = numbers(rest);
        if n.len() != if command == "fadefog" { 5 } else { 4 } || n.iter().any(|v| !v.is_finite()) {
            return None;
        }
        Some((vec4(n[0], n[1], n[2], n[3]), *n.get(4).unwrap_or(&0.)))
    })
}
#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    gates: BTreeMap<String, bool>,
    pub sky: Option<String>,
    pub from: [f32; 4],
    pub to: [f32; 4],
    pub elapsed: f32,
    pub duration: f32,
}
#[derive(Default)]
pub struct Controller {
    pub state: State,
    skies: BTreeMap<String, Vec3>,
    events: BTreeMap<String, Action>,
    fog_owned: bool,
    initial_sky_upgrade: bool,
}
struct Action {
    gates: BTreeMap<String, bool>,
    sky: Option<String>,
    fog: Option<(Vec4, f32)>,
}
impl Controller {
    pub fn load(assets: &mut Assets, map: &Bsp, name: &str) -> Result<Self> {
        let text = script(assets, name)?;
        let mut c = Self::default();
        if name == "grounds1" {
            for (event, name) in [
                ("JabberSkyLava", "sky_lava"),
                ("JabberSkyManga", "sky_manga"),
                ("JabberSkySide", "sky_manga_side"),
            ] {
                let p = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|n| n == name))
                    .and_then(|e| e.get("origin"))
                    .and_then(|s| crate::interaction::vector(s))
                    .context("Missing Royal Rage sky viewpoint")?;
                c.skies.insert(name.into(), p);
                c.events.insert(
                    event.into(),
                    Action {
                        gates: BTreeMap::new(),
                        sky: Some(name.into()),
                        fog: None,
                    },
                );
            }
            c.state.sky = Some("sky_lava".into());
            c.initial_sky_upgrade = true;
        }
        if matches!(name, "hedge1" | "hedge3") {
            for e in &map.entities {
                if let (Some(n), Some(p)) = (
                    e.get("targetname"),
                    e.get("origin").and_then(|p| crate::interaction::vector(p)),
                ) {
                    if n.starts_with("sky_camera") {
                        c.skies.insert(n.clone(), p);
                    }
                }
            }
            c.state.sky = sky_target(&body(&text, "main"));
            for index in 1..=if name == "hedge3" { 7 } else { 3 } {
                let event = format!("change_to_sky{index}");
                if let Some(sky) = sky_target(&body(&text, &event)) {
                    ensure!(c.skies.contains_key(&sky), "Missing sky target {sky}");
                    c.events.insert(
                        event.into(),
                        Action {
                            gates: BTreeMap::new(),
                            sky: Some(sky),
                            fog: None,
                        },
                    );
                }
            }
            ensure!(
                c.state
                    .sky
                    .as_ref()
                    .is_some_and(|s| c.skies.contains_key(s)),
                "Missing initial sky in {name}"
            );
        }
        let (init, events): (&str, &[(&str, &str)]) = match name {
            "garden2" => (
                "Garden2_World_Init",
                &[
                    ("Garden2_Fog1", "Garden2_Fog1"),
                    ("Garden2_Reset_Fog1", "Garden2_Reset_Fog1"),
                ],
            ),
            "garden4" => (
                "Garden4_World_Init",
                &[("Garden4_Fade_Fog", "Garden4_Fade_Fog2")],
            ),
            _ => ("", &[]),
        };
        if !init.is_empty() {
            let (initial, _) =
                fog(&body(&text, init), "setfarplane").context("Missing reviewed initial fog")?;
            c.state.from = initial.to_array();
            c.state.to = initial.to_array();
            c.fog_owned = true;
            for &(event, function) in events {
                let commands = body(&text, function);
                let f = fog(&commands, "fadefog").context("Missing reviewed fog fade")?;
                let gates = commands
                    .lines()
                    .filter_map(|l| {
                        let (name, command) = l.trim().strip_prefix('$')?.split_once('.')?;
                        let enabled = if command.starts_with("nottriggerable(") {
                            false
                        } else if command.starts_with("triggerable(") {
                            true
                        } else {
                            return None;
                        };
                        Some((name.to_owned(), enabled))
                    })
                    .collect();
                c.events.insert(
                    event.into(),
                    Action {
                        gates,
                        sky: None,
                        fog: Some(f),
                    },
                );
            }
        }
        Ok(c)
    }
    pub fn event(&mut self, event: &str) -> bool {
        let Some(a) = self.events.get(event) else {
            return false;
        };
        if let Some(sky) = &a.sky {
            self.state.sky = Some(sky.clone());
        }
        self.state.gates.extend(a.gates.clone());
        if let Some((to, duration)) = a.fog {
            // Multiple adjoining original volumes may request the same fade.
            if self.state.to != to.to_array() {
                self.state.from = self.distance().to_array();
                self.state.to = to.to_array();
                self.state.duration = duration;
                self.state.elapsed = 0.;
            }
        }
        true
    }
    pub fn update(&mut self, dt: f32) {
        if dt.is_finite() && dt > 0. {
            self.state.elapsed = (self.state.elapsed + dt).min(self.state.duration);
        }
    }
    pub fn enabled(&self, name: &str) -> bool {
        self.state.gates.get(name).copied().unwrap_or(true)
    }
    fn distance(&self) -> Vec4 {
        Vec4::from_array(self.state.from).lerp(
            Vec4::from_array(self.state.to),
            if self.state.duration > 0. {
                (self.state.elapsed / self.state.duration).clamp(0., 1.)
            } else {
                1.
            },
        )
    }
    pub fn apply(&self, scene: &mut crate::render::Scene) {
        if self.fog_owned {
            scene.atmosphere.distance = self.distance();
        }
        if let Some(p) = self.state.sky.as_ref().and_then(|s| self.skies.get(s)) {
            scene.set_sky_origin(*p);
        }
    }
    pub fn restore(&mut self, state: &State) -> Result<()> {
        if self.initial_sky_upgrade && state.sky.is_none() {
            let mut upgraded = state.clone();
            upgraded.sky = self.state.sky.clone();
            return self.restore(&upgraded);
        }
        ensure!(
            state
                .gates
                .keys()
                .all(|k| self.events.values().any(|a| a.gates.contains_key(k))),
            "Invalid saved fog trigger gate"
        );
        ensure!(
            state
                .sky
                .as_ref()
                .is_none_or(|s| self.skies.contains_key(s))
                && state.sky.is_some() == self.state.sky.is_some(),
            "Invalid saved sky target"
        );
        for values in [state.from, state.to] {
            ensure!(
                values.iter().all(|v| v.is_finite())
                    && values[..3].iter().all(|v| (0. ..=1.).contains(v))
                    && (0. ..=100000.).contains(&values[3]),
                "Invalid saved fog"
            );
        }
        ensure!(
            state.elapsed.is_finite()
                && state.duration.is_finite()
                && (0. ..=120.).contains(&state.duration)
                && (0. ..=state.duration).contains(&state.elapsed),
            "Invalid saved fog timing"
        );
        self.state = state.clone();
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct Motion {
    pivot: Vec3,
    rotation: Vec3,
    scale: f32,
    delay: f32,
    path: Vec<(Vec3, f32)>,
    shrink: Option<f32>,
}
impl Motion {
    pub fn transform(&self, time: f32) -> (Vec3, Quat, f32) {
        let a = self.rotation * time;
        let r = Quat::from_rotation_z(a.y.to_radians())
            * Quat::from_rotation_y(a.x.to_radians())
            * Quat::from_rotation_x(a.z.to_radians());
        let mut position = self.pivot;
        let mut scale = self.scale;
        if time >= self.delay && !self.path.is_empty() {
            let duration = self.path.iter().map(|p| p.1).sum::<f32>();
            if let Some(start) = self.shrink.filter(|s| time >= self.delay + s) {
                let age = (time - self.delay - start).rem_euclid(duration);
                // Reviewed Shrink_Object: 1.2 -> 0 -> 1.2 at .01/.1 s.
                scale *= if age < 12. {
                    1.2 - age * 0.1
                } else {
                    (age - 12.).min(12.) * 0.1
                } / 1.5;
            }
            let mut t = (time - self.delay).rem_euclid(duration);
            for (i, &(p, d)) in self.path.iter().enumerate() {
                if t <= d {
                    // Interpolating spline through the authored nodes; original
                    // executable spline tension is not available in the assets.
                    let n = self.path.len();
                    let u = t / d;
                    let p0 = self.path[(i + n - 1) % n].0;
                    let p2 = self.path[(i + 1) % n].0;
                    let p3 = self.path[(i + 2) % n].0;
                    position = 0.5
                        * ((2. * p)
                            + (-p0 + p2) * u
                            + (2. * p0 - 5. * p + 4. * p2 - p3) * u * u
                            + (-p0 + 3. * p - 3. * p2 + p3) * u * u * u);
                    break;
                }
                t -= d;
            }
        }
        (position, r, scale)
    }
    pub fn point(&self, p: Vec3, time: f32) -> Vec3 {
        let (o, r, s) = self.transform(time);
        o + r * (p - self.pivot) * s
    }
    pub fn normal(&self, p: Vec3, time: f32) -> Vec3 {
        self.transform(time).1 * p
    }
}
pub fn motions(assets: &mut Assets, map: &Bsp, name: &str) -> Result<BTreeMap<String, Motion>> {
    let mut result = BTreeMap::<String, Motion>::new();
    if !matches!(name, "hatter1" | "hatter2") {
        return Ok(result);
    }
    let source = script(assets, name)?;
    let text = body(&source, "Move_Sky_Pieces");
    let entity = |name: &str| {
        map.entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == name))
    };
    let mut delay = 0.;
    for l in text.lines().map(str::trim) {
        if let Some(rest) = l.strip_prefix("wait(") {
            delay += numbers(rest).first().copied().unwrap_or(0.);
            continue;
        }
        let Some((target, command)) = l.strip_prefix('$').and_then(|l| l.split_once('.')) else {
            continue;
        };
        let e = entity(target).with_context(|| format!("Missing moving sky target {target}"))?;
        let motion = result.entry(target.into()).or_insert_with(|| Motion {
            pivot: e
                .get("origin")
                .and_then(|p| crate::interaction::vector(p))
                .unwrap_or(Vec3::ZERO),
            scale: 1.,
            ..Default::default()
        });
        for (i, c) in ["rotateX(", "rotateY(", "rotateZ("].iter().enumerate() {
            if let Some(rest) = command.strip_prefix(c) {
                motion.rotation[i] = numbers(rest)
                    .first()
                    .copied()
                    .context("Invalid sky rotation")?;
            }
        }
        if let Some(rest) = command.strip_prefix("scale(") {
            motion.scale = numbers(rest)
                .first()
                .copied()
                .context("Invalid sky scale")?
                / e.get("scale")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.);
        }
        if let Some(rest) = command.strip_prefix("followpath(") {
            let first = rest
                .trim()
                .strip_prefix('$')
                .and_then(|s| s.split([',', ' ']).next())
                .context("Invalid sky path")?;
            let mut next = first.to_owned();
            motion.delay = delay;
            for _ in 0..256 {
                let node = entity(&next).context("Missing sky path node")?;
                let p = node
                    .get("origin")
                    .and_then(|s| crate::interaction::vector(s))
                    .context("Invalid sky path position")?;
                let speed = node
                    .get("speed")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(1.);
                ensure!(
                    speed.is_finite() && speed > 0. && speed <= 100.,
                    "Invalid sky path speed"
                );
                if let Some(thread) = node.get("thread") {
                    let callback = body(&source, thread);
                    if callback.contains(&format!("Shrink_Object( \"{target}\"")) {
                        ensure!(motion.shrink.is_none(), "Multiple sky shrink callbacks");
                        motion.shrink = Some(motion.path.iter().map(|p| p.1).sum());
                    }
                }
                motion.path.push((p, 1. / speed));
                next = node.get("target").context("Open sky path")?.clone();
                if next == first {
                    break;
                }
            }
            ensure!(
                next == first && motion.path.len() >= 2,
                "Unbounded sky path"
            );
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sky_path_delay_loop_and_shrink_are_bounded_and_repeatable() {
        let m = Motion {
            pivot: Vec3::X * 4.,
            rotation: vec3(4., 11., 0.),
            scale: 1.5,
            delay: 2.,
            path: vec![
                (Vec3::ZERO, 10.),
                (Vec3::X, 10.),
                (Vec3::X + Vec3::Y, 10.),
                (Vec3::Y, 10.),
            ],
            shrink: Some(30.),
        };
        assert_eq!(m.transform(1.).0, m.pivot);
        assert_eq!(m.transform(2.).0, Vec3::ZERO);
        assert!((m.transform(44.).2).abs() < 0.00001);
        assert!((m.transform(56.).2 - 1.2).abs() < 0.00001);
        assert_eq!(m.transform(12.).0, m.transform(52.).0);
        for time in [0., 1., 2., 8., 32., 44., 56., 72., 84., 10000.] {
            let (p, r, s) = m.transform(time);
            assert!(p.is_finite() && r.is_finite() && (0. ..=1.5).contains(&s));
            assert_eq!(m.point(Vec3::ONE, time), m.point(Vec3::ONE, time));
        }
    }
    #[test]
    fn fog_interruption_pause_and_restore() {
        let mut c = Controller {
            fog_owned: true,
            ..Default::default()
        };
        c.events.insert(
            "fade".into(),
            Action {
                gates: BTreeMap::from([("underfog1".into(), false)]),
                sky: None,
                fog: Some((vec4(0.5, 0.3, 0.1, 4000.), 4.)),
            },
        );
        assert!(c.event("fade"));
        c.update(2.);
        let half = c.distance();
        c.update(0.);
        assert_eq!(half, c.distance());
        assert_eq!(half.w, 2000.);
        let saved = serde_json::from_slice(&serde_json::to_vec(&c.state).unwrap()).unwrap();
        c.update(2.);
        assert_eq!(c.distance().w, 4000.);
        c.restore(&saved).unwrap();
        assert!(!c.enabled("underfog1"));
        assert_eq!(half, c.distance());
        c.event("fade");
        assert_eq!(half, c.distance());
        let mut invalid = saved;
        invalid.duration = f32::NAN;
        assert!(c.restore(&invalid).is_err());
    }
    #[test]
    fn bounded_function_does_not_take_other_sky_targets() {
        let s="void main();\nvoid other() {}\nvoid main() {\n $one.rendereffects(\"+skyorigin\");\n}\nvoid change() {\n $two.rendereffects(\"+skyorigin\");\n}";
        assert_eq!(sky_target(&body(s, "main")).as_deref(), Some("one"));
    }
}

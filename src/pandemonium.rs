//! Reviewed Pandemonium traversal. Asset declarations are data, never executed scripts.
pub mod cinema;
mod machinery;
use crate::{
    bsp::Bsp,
    collision::{Collider, World, PLAYER_CENTER, PLAYER_HALF},
    event::{Condition, Facts},
    interaction::{vector, Events},
    movement::{Controls, Player},
    skeletal::Transform,
};
use anyhow::{ensure, Context, Result};
use cinema::Beat;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn supported(e: &BTreeMap<String, String>) -> bool {
    matches!(
        e.get("classname").map(String::as_str),
        Some("script_object" | "func_rope")
    ) && e.get("model").is_some_and(|s| s.starts_with('*'))
}
fn point(map: &Bsp, name: &str) -> Result<Vec3> {
    map.entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|s| s == name))
        .and_then(|e| e.get("origin"))
        .and_then(|s| vector(s))
        .with_context(|| format!("Missing Pandemonium marker {name}"))
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum Cart {
    Waiting,
    Boarding,
    Lift,
    Rail,
    Finished,
}
#[derive(Clone, Serialize, Deserialize)]
struct Grip {
    index: usize,
    length: f32,
    velocity: Vec3,
    hand: Vec3,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Snapshot {
    #[serde(default)]
    pub cinema: cinema::State,
    pub cart: Cart,
    pub time: f32,
    age: f32,
    pub key: bool,
    key_time: f32,
    pub returned: bool,
    return_time: f32,
    pub leaving: bool,
    pub departure: bool,
    flight: f32,
    pub dialogue_done: bool,
    exit_sent: bool,
    board_from: Vec3,
    grip: Option<Grip>,
    pub rides: u32,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            cinema: cinema::State::fresh(),
            cart: Cart::Waiting,
            time: 0.,
            age: 0.,
            key: false,
            key_time: 0.,
            returned: false,
            return_time: 0.,
            leaving: false,
            departure: false,
            flight: 0.,
            dialogue_done: false,
            exit_sent: false,
            board_from: Vec3::ZERO,
            grip: None,
            rides: 0,
        }
    }
}
struct Object {
    name: String,
    model: usize,
    base: Vec3,
    origin: Vec3,
    rotation: Quat,
    collider: Collider,
    solid: bool,
    visible: bool,
}
pub struct Pandemonium {
    cinema: cinema::Data,
    machinery: machinery::Machinery,
    rig: crate::skeletal::Skeleton,
    fly: crate::skeletal::Animation,
    hover: crate::skeletal::Animation,
    ship_scale: f32,
    alice_tag: usize,
    gnome_tag: usize,
    pub state: Snapshot,
    objects: Vec<Object>,
    rail: Vec<Vec3>,
    rail_times: Vec<f32>,
    air: Vec<Vec3>,
    air_times: Vec<f32>,
    ropes: Vec<(usize, Vec3, f32)>,
    key: Vec3,
    cart_base: Vec3,
    end: Vec3,
    ship: Vec3,
}
fn path(map: &Bsp, name: &str) -> Result<Vec<(Vec3, f32)>> {
    let mut next = Some(name);
    let mut seen = std::collections::BTreeSet::new();
    let mut points = Vec::new();
    while let Some(name) = next {
        ensure!(
            seen.insert(name) && points.len() < 256,
            "Invalid transport path"
        );
        let e = map
            .entities
            .iter()
            .find(|e| e.get("targetname").is_some_and(|s| s == name))
            .context("Missing transport node")?;
        points.push((
            vector(&e["origin"]).context("Invalid transport point")?,
            e.get("speed")
                .and_then(|s| s.parse::<f32>().ok())
                .unwrap_or(1.)
                .clamp(0.1, 8.),
        ));
        next = e.get("target").map(String::as_str);
    }
    ensure!(points.len() > 1, "Short transport path");
    Ok(points)
}
fn fraction(time: f32, duration: f32) -> f32 {
    (time / duration).clamp(0., 1.)
}
fn lerp_path(points: &[Vec3], times: &[f32], t: f32) -> (Vec3, Vec3) {
    for i in 1..points.len() {
        if t <= times[i] {
            return (
                points[i - 1].lerp(
                    points[i],
                    fraction(t - times[i - 1], times[i] - times[i - 1]),
                ),
                (points[i] - points[i - 1]).normalize_or_zero(),
            );
        }
    }
    (
        *points.last().unwrap(),
        (points[points.len() - 1] - points[points.len() - 2]).normalize_or_zero(),
    )
}
/// Seconds the departure flight spends on each authored node of the airship path.
const FLIGHT_NODE_TIME: f32 = 3.;
/// Half width, in seconds, of the blend that turns the airship from one segment heading to the next.
const YAW_BLEND: f32 = 0.5;
fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0., 1.);
    x * x * (3. - 2. * x)
}
/// Horizontal travel direction of segment `i` (point `i` to point `i + 1`). A segment with no
/// horizontal travel borrows the nearest one that has some (the earlier one first), so a vertical
/// hop never spins the ship.
fn segment_heading(points: &[Vec3], i: usize) -> Vec2 {
    let last = points.len() - 2;
    let heading = |k: usize| (points[k + 1] - points[k]).truncate().normalize_or_zero();
    for d in 0..=last {
        for k in [i.checked_sub(d), Some(i + d).filter(|&k| k <= last)]
            .into_iter()
            .flatten()
        {
            let h = heading(k);
            if h != Vec2::ZERO {
                return h;
            }
        }
    }
    Vec2::X
}
/// Travel heading at time `t` along a piecewise-linear path (the same segment choice as
/// `lerp_path`). The raw segment headings jump at every interior node, so the two adjacent
/// headings are blended with a smoothstep over `YAW_BLEND` seconds either side of the node and
/// renormalised. That approximates the tangent of the native smooth spline. Only the horizontal
/// direction is used: pitch and bank are not reproduced.
fn flight_heading(points: &[Vec3], times: &[f32], t: f32) -> Vec2 {
    let segments = points.len() - 1;
    let index = (1..points.len())
        .find(|&i| t <= times[i])
        .map_or(segments - 1, |i| i - 1);
    for node in [index, index + 1] {
        if node < 1 || node >= segments {
            continue; // Path ends keep their own segment heading.
        }
        let half = YAW_BLEND
            .min(0.5 * (times[node] - times[node - 1]))
            .min(0.5 * (times[node + 1] - times[node]));
        if half > 0. && (t - times[node]).abs() < half {
            let before = segment_heading(points, node - 1);
            let after = segment_heading(points, node);
            let blend = before.lerp(after, smoothstep((t - times[node] + half) / (2. * half)));
            return if blend.length_squared() > 1e-6 {
                blend.normalize()
            } else {
                after
            };
        }
    }
    segment_heading(points, index)
}
/// Yaw of the airship model about +Z, in radians. The model's authored nose is +X (its propeller
/// and rudders sit at -X, the passenger seat at +X), so yaw 0 is the authored rest pose.
fn flight_yaw(points: &[Vec3], times: &[f32], t: f32) -> f32 {
    let h = flight_heading(points, times, t);
    h.y.atan2(h.x)
}
/// The ship flies its path from the moment the departure starts, except during the boarding beat,
/// when it still hovers at its rest waypoint.
fn is_flying(departure: bool, beat: Option<Beat>) -> bool {
    departure && beat != Some(Beat::BoardShip)
}
/// Rest pose and boarding keep yaw exactly 0 as authored; flight follows the path heading.
fn ship_yaw_of(flying: bool, points: &[Vec3], times: &[f32], flight: f32) -> f32 {
    if flying {
        flight_yaw(points, times, flight)
    } else {
        0.
    }
}
/// Transform of a ship attachment tag: the tag offset turns with the ship, and the tag's own
/// rotation is composed after the ship's.
fn attach(position: Vec3, yaw: Quat, tag: Transform, scale: f32) -> Transform {
    Transform {
        translation: position + yaw * (tag.translation * scale),
        rotation: yaw * tag.rotation,
    }
}
impl Pandemonium {
    pub fn load(assets: &mut crate::assets::Assets, map: &Bsp) -> Result<Self> {
        let def = crate::skeletal::Definition::load(assets, "models/gnome_airship.tik")?;
        let rig = crate::skeletal::Skeleton::parse(
            &assets.read(&format!("{}/{}", def.path, def.model))?,
        )?;
        let fly = crate::skeletal::Animation::parse(
            &assets.read(&format!("{}/{}", def.path, def.animations["fly"]))?,
            rig.bones.len(),
        )?;
        let hover = crate::skeletal::Animation::parse(
            &assets.read(&format!("{}/{}", def.path, def.animations["hover"]))?,
            rig.bones.len(),
        )?;
        let alice_tag = rig
            .bones
            .iter()
            .position(|b| b.name == "tag_alice")
            .context("Airship Alice attachment missing")?;
        let gnome_tag = rig
            .bones
            .iter()
            .position(|b| b.name == "tag_gnome")
            .context("Airship Gnome attachment missing")?;
        let mut objects = Vec::new();
        let mut ropes = Vec::new();
        for e in map.entities.iter().filter(|e| supported(e)) {
            let model = e["model"][1..].parse::<usize>()?;
            let base = vector(&e["origin"]).context("Invalid mover origin")?;
            let name = e
                .get("targetname")
                .cloned()
                .unwrap_or_else(|| format!("rope/{model}"));
            if e["classname"] == "func_rope" {
                let m = &map.models[model];
                ropes.push((objects.len(), base + Vec3::Z * m.max.z, m.max.z - m.min.z));
            }
            objects.push(Object {
                name,
                model,
                base,
                origin: base,
                rotation: Quat::IDENTITY,
                collider: Collider::model(map, model, base, Quat::IDENTITY, true)?,
                solid: true,
                visible: true,
            });
        }
        let data = path(map, "minecart_spline")?;
        let mut rail_times = vec![0.];
        for p in data.windows(2) {
            rail_times.push(rail_times.last().unwrap() + 1. / p[0].1);
        }
        let cinema = cinema::Data::load(assets, map, &rail_times)?;
        let air = path(map, "airship_spline")?
            .iter()
            .map(|p| p.0)
            .collect::<Vec<_>>();
        let air_times = (0..air.len())
            .map(|i| i as f32 * FLIGHT_NODE_TIME)
            .collect();
        let mut p = Self {
            cinema,
            machinery: machinery::Machinery::load(map)?,
            rig,
            fly,
            hover,
            ship_scale: def.scale,
            alice_tag,
            gnome_tag,
            state: Snapshot::default(),
            objects,
            rail: data.iter().map(|p| p.0).collect(),
            rail_times,
            air,
            air_times,
            ropes,
            key: point(map, "gnome_key")?,
            cart_base: point(map, "minecart")?,
            end: point(map, "minecart_endnode2")?,
            ship: point(map, "airship_start_waypoint")?,
        };
        p.rebuild(map)?;
        Ok(p)
    }
    /// Staged restart fixtures; not used by the normal-input route.
    pub fn fixture(
        &mut self,
        phase: &str,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
    ) -> Result<()> {
        self.state = Snapshot::default();
        if phase == "pand-warning" || phase == "pand-vanish" {
            self.state.cinema.start(if phase == "pand-warning" {
                Beat::Warning
            } else {
                Beat::Vanish
            });
            self.state.cinema.time = if phase == "pand-warning" { 8. } else { 2. };
            player.feet = self.cinema.at("elder_gnome1_player_node2");
        } else if phase == "pand-boarding" {
            self.state.cart = Cart::Boarding;
            self.state.time = 0.;
            player.feet = vec3(-5024., 1368., 32.);
        } else if phase == "pand-cart" {
            self.state.cart = Cart::Lift;
            self.state.time = 2.;
        } else {
            self.state.cart = Cart::Finished;
            self.state.time = self.rail_times[90];
            self.state.rides = 1;
            self.state.cinema.warning_done = true;
            self.state.key = true;
            player.feet = self.end;
            if matches!(
                phase,
                "pand-return"
                    | "pand-flight"
                    | "pand-flight-start"
                    | "pand-flight-turn"
                    | "pand-flight-end"
                    | "pand-board"
                    | "pand-leave"
            ) {
                self.state.returned = true;
                self.state.leaving = true;
                if !self.state.cinema.return_done {
                    self.state.cinema.start(Beat::Return);
                }
            }
            if phase == "pand-leave" {
                self.state.cinema.start(Beat::Return);
                self.state.cinema.time = 7.;
                player.feet = vec3(-3600., 800., -264.);
            }
            if matches!(phase, "pand-rail" | "pand-rail-start") {
                self.state.cart = Cart::Rail;
                self.state.time = if phase == "pand-rail" { 10. } else { 0. };
                self.state.rides = 0;
                self.state.key = false;
            }
            if phase == "pand-landing" {
                self.state.cart = Cart::Rail;
                self.state.rides = 0;
                self.state.key = false;
                self.state.cinema.start(Beat::Landing);
                self.state.cinema.time = 7.;
            }
            if phase == "pand-board" {
                self.state.departure = true;
                self.state.cinema.return_done = true;
                self.state.cinema.start(Beat::BoardShip);
                self.state.cinema.time = 3.;
            }
            // Staged flight clocks. Each render capture lands 1.25 s later: on the first path
            // segment, just after the second node (mid-turn) and on the last segment.
            let flight = match phase {
                "pand-flight" => Some(4.),
                "pand-flight-start" => Some(0.25),
                "pand-flight-turn" => Some(5.),
                "pand-flight-end" => Some(13.),
                _ => None,
            };
            if let Some(flight) = flight {
                self.state.cinema.return_done = true;
                self.state.cinema.start(Beat::Flight);
                self.state.departure = true;
                self.state.flight = flight;
            }
        }
        self.advance(1. / 120., map, world, player, &[])
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.clone()
    }
    pub fn restore(&mut self, s: &Snapshot, map: &Bsp) -> Result<()> {
        s.cinema.validate()?;
        ensure!(
            (!matches!(s.cinema.beat, Some(Beat::Return)) || s.returned)
                && (!matches!(s.cinema.beat, Some(Beat::BoardShip | Beat::Flight)) || s.departure)
                && (s.cinema.beat != Some(Beat::Landing) || s.cart == Cart::Rail)
                && [s.time, s.age, s.key_time, s.return_time, s.flight]
                    .iter()
                    .all(|t| t.is_finite() && (0.0..=1e8).contains(t))
                && s.board_from.is_finite()
                && s.rides <= 1
                && s.key_time <= 1.4
                && s.return_time <= 2.
                && (!s.key || s.cart == Cart::Finished)
                && (!s.returned || s.key)
                && (s.cart == Cart::Finished) == (s.rides == 1)
                && (s.cart != Cart::Waiting || s.time == 0.)
                && (s.cart != Cart::Boarding || s.time <= 1.1)
                && (s.cart != Cart::Lift || s.time <= 6.1)
                && s.time <= self.rail_times[90] + 0.1
                && (s.key || s.key_time == 0.)
                && (s.returned || s.return_time == 0.)
                && (s.departure || s.flight == 0.)
                && (!s.leaving || s.returned)
                && (!s.dialogue_done || s.departure)
                && (!s.departure || s.returned)
                && (!s.exit_sent || (s.departure && s.dialogue_done))
                && s.grip.as_ref().is_none_or(|g| g.index < self.ropes.len()
                    && g.length.is_finite()
                    && g.length >= 32.
                    && g.length <= self.ropes[g.index].2
                    && g.hand.is_finite()
                    && (g.hand.distance(self.ropes[g.index].1) - g.length).abs() < 2.
                    && g.velocity.is_finite()
                    && g.velocity.length() <= 900.),
            "Invalid Pandemonium save"
        );
        self.state = s.clone();
        self.rebuild(map)
    }
    pub fn validate_player(&self, player: &Player) -> Result<()> {
        let expected = if self.state.cinema.beat == Some(Beat::BoardShip) {
            None // Authored boarding walk is still on the deck, before the seat attachment.
        } else if self.state.cinema.beat == Some(Beat::Landing) {
            Some(self.end)
        } else if self.state.departure && self.state.flight > 0. {
            // A save written before the ship yawed put Alice at the unturned seat. It stays
            // loadable: the next tick moves her to the turned seat.
            let seat = self.ship_seat();
            let legacy = self.legacy_ship_seat();
            Some(
                if seat.distance(player.feet) <= legacy.distance(player.feet) {
                    seat
                } else {
                    legacy
                },
            )
        } else {
            match self.state.cart {
                Cart::Boarding if self.state.time > 0. => Some(self.state.board_from.lerp(
                    self.cart_base + Vec3::Z * 16.,
                    fraction(self.state.time, 1.),
                )),
                Cart::Lift => {
                    Some(self.cart_base + Vec3::Z * (16. + 640. * fraction(self.state.time, 6.)))
                }
                Cart::Rail => Some(self.rail_pose().0 + Vec3::Z * 16.),
                _ => None,
            }
        };
        ensure!(
            expected.is_none_or(|p| p.distance(player.feet) < 0.25),
            "Saved player detached from transport"
        );
        ensure!(
            self.state
                .grip
                .as_ref()
                .is_none_or(|g| g.hand.distance(player.feet + Vec3::Z * 40.) < 0.25),
            "Saved player detached from rope"
        );
        ensure!(
            self.controlled() || player.script_motion == 0,
            "Saved scripted pose without transport"
        );
        Ok(())
    }
    pub fn facts(&self) -> Facts {
        let mut f = Facts::default();
        f.flag("pand.key", self.state.key);
        f.flag("pand.cart_done", self.state.cart == Cart::Finished);
        f.flag("pand.returned", self.state.returned);
        f
    }
    pub fn gate(thread: &str, teleport: bool) -> Condition {
        if teleport || thread == "alice_return_trigger1" {
            Condition::All(vec![
                Condition::flag("pand.key"),
                Condition::flag("pand.cart_done"),
            ])
        } else if matches!(thread, "alice_leave" | "Pand_End_Ship") {
            Condition::flag("pand.returned")
        } else {
            Condition::Always
        }
    }
    pub fn event(&mut self, thread: &str) -> Option<Events> {
        let mut e = Events::default();
        match thread {
            cinema::WARNING => {
                if !self.state.cinema.warning_done {
                    self.state.cinema.start(Beat::Warning);
                    self.state.grip = None;
                    e.story.push(thread.into());
                }
            }
            "Minecart_Thread" => {
                if self.state.cart == Cart::Waiting {
                    self.state.cart = Cart::Boarding;
                    self.state.time = 0.;
                    self.state.grip = None;
                    e.message = Some("Minecart / hold on".into());
                    e.sound = Some("sound/world/machine/mine_lift2.wav".into());
                }
            }
            "alice_return_trigger1" => {
                self.state.returned = true;
                e.message = Some("Key secured / return to the Gnome's house".into());
            }
            "alice_leave" => {
                self.state.leaving = true;
                e.message = Some("The Gnome is waiting / enter his house".into());
            }
            "Pand_End_Ship" => {
                self.state.departure = true;
                self.state.cinema.start(Beat::BoardShip);
                self.state.flight = 0.;
                self.state.grip = None;
                e.story.push(thread.into());
                e.message = Some("Airship / departing for the fortress".into());
            }
            "Airship_CardGuards_Thread" => {}
            _ if crate::story::supports("pandemonium", thread) => e.story.push(thread.into()),
            _ => return None,
        }
        Some(e)
    }
    pub fn objective(&self) -> String {
        if self.state.departure {"Ride the airship to the fortress."}
        else if self.state.returned {"Return through the village streets to the Gnome's house."}
        else if self.state.key {"The key opened the upper door. Climb the hall and enter the return portal."}
        else if self.state.cart==Cart::Finished {"Collect the Cards, pass the guards and find the Gnome's key."}
        else if self.state.cart!=Cart::Waiting {"Ride the minecart. Its transport ends on the far ledge."}
        else {"Descend to the Elder Gnome, then follow the path to the minecart. E grabs/releases ropes; Space/Ctrl climbs; WASD swings."}.into()
    }
    pub fn cinematic(&self) -> bool {
        self.state.cinema.beat.is_some()
            || matches!(self.state.cart, Cart::Boarding | Cart::Lift | Cart::Rail)
            || self.state.departure
    }
    pub fn recovery_entry(&self, normal: (Vec3, f32)) -> (Vec3, f32) {
        if self.state.cart == Cart::Finished && !self.state.returned {
            (self.end, -std::f32::consts::FRAC_PI_2)
        } else {
            normal
        }
    }
    pub fn release_rope(&mut self) {
        self.state.grip = None;
    }
    pub fn controlled(&self) -> bool {
        self.cinematic() || self.state.grip.is_some()
    }
    pub fn prompt(&self, world: &World, eye: Vec3) -> Option<&'static str> {
        if self.cinematic() {
            None
        } else if self.state.grip.is_some() {
            Some("E release rope / Space up / Ctrl down / WASD swing")
        } else {
            self.near_rope(world, eye).map(|_| "E grab rope")
        }
    }
    fn near_rope(&self, world: &World, eye: Vec3) -> Option<usize> {
        self.ropes
            .iter()
            .enumerate()
            .find_map(|(i, (_, top, length))| {
                let hand = vec3(top.x, top.y, eye.z.clamp(top.z - length, top.z - 24.));
                let t = world.sweep(eye, hand, Vec3::splat(0.5));
                (eye.distance(hand) < 76. && !t.start_solid && t.fraction >= 1.).then_some(i)
            })
    }
    pub fn rope_hand(&self) -> Option<(usize, Vec3)> {
        self.state
            .grip
            .as_ref()
            .map(|g| (self.objects[self.ropes[g.index].0].model, g.hand))
    }
    pub fn update(&mut self, world: &World, player: &Player, use_pressed: bool) -> Events {
        let mut e = Events::default();
        if self.state.cart == Cart::Boarding && self.state.time == 0. {
            self.state.board_from = player.feet;
        }
        if use_pressed
            && !self.cinematic()
            && self.state.grip.take().is_none()
            && player.knockback_time <= 0.
        {
            if let Some(index) = self.near_rope(world, player.eye()) {
                self.state.grip = Some(Grip {
                    index,
                    length: self.ropes[index]
                        .1
                        .distance(player.feet + Vec3::Z * 40.)
                        .clamp(32., self.ropes[index].2),
                    velocity: Vec3::ZERO,
                    hand: player.feet + Vec3::Z * 40.,
                });
            }
        }
        let delta = self.key - player.eye();
        if !self.state.key
            && self.state.cart == Cart::Finished
            && delta.truncate().length() < 48.
            && delta.z.abs() < 64.
        {
            let t = world.sweep(player.eye(), self.key, Vec3::splat(0.5));
            if !t.start_solid && t.fraction >= 1. {
                self.state.key = true;
                e.message = Some("Collected the Gnome's key / the upper door opens".into());
                e.sound = Some("sound/item/pickup.wav".into());
            }
        }
        if self.state.departure
            && self.state.dialogue_done
            && self.state.flight >= 15.
            && !self.state.exit_sent
        {
            self.state.exit_sent = true;
            e.transition = Some(("fortress1".into(), Some("fortress1_start1".into())));
        }
        e
    }
    pub fn control(
        &mut self,
        dt: f32,
        world: &World,
        player: &mut Player,
        input: Controls,
    ) -> bool {
        if self.cinematic() {
            return true;
        }
        if player.knockback_time > 0. && self.state.grip.is_some() {
            self.release_rope();
            player.script_motion = 0;
            return false;
        }
        let Some(grip) = &mut self.state.grip else {
            return false;
        };
        let (_, anchor, max) = self.ropes[grip.index];
        let before_length = grip.length;
        let mut remaining = dt.min(0.1);
        while remaining > 0.000001 {
            let step = remaining.min(1. / 120.);
            remaining -= step;
            crate::rope::step_grip(
                world,
                anchor,
                max,
                &mut player.feet,
                &mut grip.length,
                &mut grip.velocity,
                input,
                step,
            );
        }
        grip.hand = player.feet + Vec3::Z * 40.;
        player.script_rope_rise = if (before_length - grip.length).abs() > 0.001 {
            (before_length - grip.length).signum()
        } else {
            0.
        };
        player.script_rope_length = Some(grip.length);
        player.script_facing =
            crate::rope::grip_facing(anchor, player.feet, player.script_facing, dt);
        player.velocity = grip.velocity;
        player.grounded = false;
        player.script_motion = 1;
        true
    }
    fn rail_pose(&self) -> (Vec3, Quat) {
        let t = self.state.time
            + if self.state.cinema.beat == Some(Beat::Landing) {
                self.state.cinema.time
            } else {
                0.
            };
        self.rail_pose_at(t)
    }
    fn rail_pose_at(&self, t: f32) -> (Vec3, Quat) {
        let (p, fallback) = lerp_path(&self.rail, &self.rail_times, t);
        // Segment tangents jump at every authored node. A short, symmetric look
        // ahead/back follows the same rail while turning the cart/camera continuously.
        let before = lerp_path(&self.rail, &self.rail_times, (t - 0.25).max(0.)).0;
        let after = lerp_path(&self.rail, &self.rail_times, t + 0.25).0;
        let dir = if (after - before).truncate().length_squared() > 0.001 {
            after - before
        } else {
            fallback
        };
        (p, Quat::from_rotation_z(dir.y.atan2(dir.x)))
    }
    pub fn advance(
        &mut self,
        dt: f32,
        map: &Bsp,
        world: &mut World,
        player: &mut Player,
        fixed: &[Collider],
    ) -> Result<()> {
        if dt <= 0. {
            return Ok(());
        }
        let old_key_time = self.state.key_time;
        let old_return_time = self.state.return_time;
        let dt = dt.min(0.1);
        self.state.age += dt;
        if self.state.key {
            self.state.key_time = (self.state.key_time + dt).min(1.4);
        }
        if self.state.returned {
            self.state.return_time = (self.state.return_time + dt).min(2.);
        }
        self.advance_cinema(dt, player);
        match self.state.cart {
            Cart::Boarding => {
                if self.state.time == 0. {
                    self.state.board_from = player.feet;
                }
                self.state.time += dt;
                player.feet = self.state.board_from.lerp(
                    self.cart_base + Vec3::Z * 16.,
                    fraction(self.state.time, 1.),
                );
                if self.state.time >= 1. {
                    self.state.cart = Cart::Lift;
                    self.state.time = 0.;
                }
            }
            Cart::Lift => {
                self.state.time += dt;
                player.feet =
                    self.cart_base + Vec3::Z * (16. + 640. * fraction(self.state.time, 6.));
                if self.state.time >= 6. {
                    self.state.cart = Cart::Rail;
                    self.state.time = 0.;
                    player.feet = self.rail_pose().0 + Vec3::Z * 16.;
                }
            }
            Cart::Rail if self.state.cinema.beat != Some(Beat::Landing) => {
                self.state.time += dt;
                player.feet = self.rail_pose().0 + Vec3::Z * 16.;
                // The source cinematic jumps Alice to this landing as the cart plunges away.
                if self.state.time >= self.rail_times[90] {
                    ensure!(world.body_clear(self.end), "Minecart landing is obstructed");
                    player.feet = self.end;
                    player.velocity = Vec3::ZERO;
                    player.grounded = true;
                    self.state.time = self.rail_times[90];
                    self.state.cinema.start(Beat::Landing);
                }
            }
            _ => {}
        }
        if self.state.departure && self.state.cinema.beat != Some(Beat::BoardShip) {
            self.state.flight += dt;
            player.feet = self.ship_seat();
        }
        player.script_motion = if self.state.departure {
            3
        } else if matches!(self.state.cart, Cart::Boarding | Cart::Lift | Cart::Rail) {
            2
        } else if self.state.grip.is_some() {
            1
        } else {
            0
        };
        if self.cinematic() {
            player.velocity = Vec3::ZERO;
            player.grounded = true;
            player.script_facing = if self.state.departure {
                self.ship_yaw()
            } else if self.state.cart == Cart::Rail {
                let (_, q) = self.rail_pose();
                let d = q * Vec3::X;
                d.y.atan2(d.x)
            } else {
                -std::f32::consts::FRAC_PI_2
            };
        }
        self.rebuild(map)?;
        if !self.cinematic() {
            let center = player.feet + PLAYER_CENTER;
            let obstructed = |o: &Object| {
                o.visible && o.solid && o.collider.trace(center, center, PLAYER_HALF).start_solid
            };
            let key_blocked = self
                .objects
                .iter()
                .any(|o| o.name == "guard_last_door" && obstructed(o));
            let return_blocked = self
                .objects
                .iter()
                .any(|o| matches!(o.name.as_str(), "return_gate1" | "t136") && obstructed(o));
            if key_blocked {
                self.state.key_time = old_key_time;
            }
            if return_blocked {
                self.state.return_time = old_return_time;
            }
            if key_blocked || return_blocked {
                self.rebuild(map)?;
            }
        }
        world.set_dynamic(fixed.iter().cloned().chain(self.colliders()).collect());
        Ok(())
    }
    fn rebuild(&mut self, map: &Bsp) -> Result<()> {
        let rail = self.rail_pose();
        let state = &self.state;
        let mut cart_origin = self.cart_base;
        let mut cart_rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
        if state.cart == Cart::Lift {
            cart_origin.z += 640. * fraction(state.time, 6.);
        } else if state.cart == Cart::Rail {
            (cart_origin, cart_rotation) = rail;
        }
        let mut cart_visible = state.cart != Cart::Finished;
        if state.cinema.beat == Some(Beat::Landing) {
            let sink = (state.time + state.cinema.time - self.rail_times.last().unwrap()).max(0.);
            cart_origin.z -= 128. * fraction(sink, 1.);
            // Pitch the entire assembly down around its transverse axle.
            cart_rotation *=
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 * fraction(sink, 1.));
            cart_visible &= sink < 2.;
        }
        for o in &mut self.objects {
            let n = o.name.as_str();
            let mut p = o.base;
            let mut q = Quat::IDENTITY;
            let mut visible = true;
            let mut solid = true;
            match n {
                "minecart" | "minecart_wheels1" | "minecart_wheels2" => {
                    // BSP wheel origins are axle pivots in the cart's authored frame.
                    // Bind their offsets for every phase, including lift and final tilt.
                    p = cart_origin + cart_rotation * (o.base - self.cart_base);
                    q = cart_rotation;
                    visible = cart_visible;
                    if n.starts_with("minecart_wheels") && matches!(state.cart, Cart::Rail) {
                        // Source rotateX is pitch (our local Y); the two discs lie
                        // along Y. Rotating around X instead orbits them through the bed.
                        q *= Quat::from_rotation_y(
                            (state.time + state.cinema.time) * std::f32::consts::TAU,
                        );
                    }
                    solid = false;
                }
                "minecart_lift" => {
                    if state.cart == Cart::Lift {
                        p.z += 640. * fraction(state.time, 6.);
                    } else if matches!(state.cart, Cart::Rail | Cart::Finished) {
                        p.z += 640.;
                    }
                }
                "minecart_fake" => {
                    visible = state.cart == Cart::Finished;
                    solid = visible;
                }
                "mine_end_door" | "mine_end_rocks" => {
                    visible = state.cart == Cart::Finished;
                    solid = visible;
                }
                "mine_block" => {
                    visible = state.returned;
                    solid = visible;
                }
                "guard_last_door" => {
                    q = Quat::from_rotation_z(75_f32.to_radians() * fraction(state.key_time, 1.4))
                }
                "return_gate1" => p.z += 128. * fraction(state.return_time, 2.),
                "t136" => {
                    q = Quat::from_rotation_z(
                        -65_f32.to_radians() * fraction(state.return_time, 2.),
                    );
                }
                _ if n.starts_with("rope/") => {
                    solid = false;
                    if let Some(g) = &state.grip {
                        let (_, anchor, _) = self.ropes[g.index];
                        if o.base.x == anchor.x && o.base.y == anchor.y {
                            q = Quat::from_rotation_arc(
                                -Vec3::Z,
                                (g.hand - anchor).normalize_or_zero(),
                            );
                            p = anchor + q * (o.base - anchor);
                        }
                    }
                }
                _ if n.starts_with("badtrack") => {
                    let index = usize::from(n.starts_with("badtrack2"));
                    let clock = if state.cart == Cart::Finished {
                        1000.
                    } else if state.cart == Cart::Rail {
                        state.time
                            + if state.cinema.beat == Some(Beat::Landing) {
                                state.cinema.time
                            } else {
                                0.
                            }
                    } else {
                        -1000.
                    };
                    let t = clock - self.cinema.collapse_times[index];
                    let transform = cinema::debris(n, o.base, t);
                    p = transform.translation;
                    q = transform.rotation;
                    if n.contains("rock") || t >= 1. {
                        solid = false;
                    }
                }
                _ => {
                    if let Some(t) = self.machinery.sample(n, o.base, state.age) {
                        p = t.translation;
                        q = t.rotation;
                        solid = false;
                    }
                }
            }
            if solid && (!o.solid || o.origin != p || o.rotation != q) {
                o.collider = Collider::model(map, o.model, p, q, true)?;
            }
            o.origin = p;
            o.rotation = q;
            o.visible = visible;
            o.solid = solid;
        }
        Ok(())
    }
    pub fn sound_state(
        &self,
        loops: &mut Vec<crate::audio::LoopCue>,
        clocks: &mut Vec<crate::audio::world::Clock>,
    ) {
        let origin = self
            .objects
            .iter()
            .find(|o| o.name == "minecart")
            .map_or(self.cart_base, |o| o.origin);
        if self.state.cart == Cart::Rail {
            let clock = self.state.time
                + if self.state.cinema.beat == Some(Beat::Landing) {
                    self.state.cinema.time
                } else {
                    0.
                };
            for index in 0..2 {
                let t = clock - self.cinema.collapse_times[index];
                if t >= 0. {
                    clocks.push(crate::audio::world::Clock {
                        key: if index == 0 {
                            "cart-collapse-1"
                        } else {
                            "cart-collapse-2"
                        },
                        time: t,
                        period: None,
                        origin: self.cinema.at(if index == 0 {
                            "mine_boulder_sound1"
                        } else {
                            "mine_boulder_sound2"
                        }),
                        cues: if index == 0 {
                            &[
                                (0., "sound/world/machine/mine_lift7.wav"),
                                (1., "sound/world/machine/mine_lift4.wav"),
                                (4., "sound/world/machine/mine_lift5.wav"),
                            ]
                        } else {
                            &[
                                (0., "sound/world/machine/mine_lift7.wav"),
                                (1.2, "sound/world/machine/mine_lift4.wav"),
                                (2.2, "sound/world/machine/mine_lift5.wav"),
                            ]
                        },
                    });
                }
            }
            clocks.push(crate::audio::world::Clock {
                key: "cart-lift-stop",
                time: self.state.time,
                period: None,
                origin,
                cues: &[(0., "sound/world/machine/mine_lift2.wav")],
            });
        }
        match self.state.cart {
            Cart::Lift => {
                loops.push(crate::audio::LoopCue {
                    id: 1000,
                    path: "sound/world/machine/mine_lift1.wav",
                    origin,
                    clock: Some(self.state.time),
                });
                clocks.push(crate::audio::world::Clock {
                    key: "cart-lift",
                    time: self.state.time,
                    period: None,
                    origin,
                    cues: &[(0., "sound/world/machine/mine_lift2.wav")],
                });
            }
            Cart::Rail => {
                if self.state.cinema.beat != Some(Beat::Landing) {
                    loops.push(crate::audio::LoopCue {
                        id: 1000,
                        path: "sound/world/machine/mine_lift3.wav",
                        origin,
                        clock: Some(self.state.time),
                    });
                }
                // This last cue follows the cinematic's cart-drop time, not a skip to its endpoint.
                if self.state.cinema.beat == Some(Beat::Landing) {
                    clocks.push(crate::audio::world::Clock {
                        key: "cart-splash",
                        time: self.state.cinema.time,
                        period: None,
                        origin,
                        cues: &[(0., "sound/ambience/special/splash_1.wav")],
                    });
                }
            }
            _ => (),
        }
    }
    pub fn gate_particles(&self, steam: &mut crate::particles::Steam) {
        steam.gate(&self.machinery.emissions(self.state.age));
    }
    pub fn colliders(&self) -> impl Iterator<Item = Collider> + '_ {
        self.objects
            .iter()
            .filter(|o| o.visible && o.solid)
            .map(|o| o.collider.clone())
    }
    pub fn transforms(&self) -> impl Iterator<Item = (usize, Vec3, Quat)> + '_ {
        self.objects.iter().map(|o| {
            (
                o.model,
                if o.visible {
                    o.origin
                } else {
                    Vec3::splat(-100000.)
                },
                o.rotation,
            )
        })
    }
    pub fn ship_seat(&self) -> Vec3 {
        self.ship_attachment(self.alice_tag).translation
    }
    fn flying(&self) -> bool {
        is_flying(self.state.departure, self.state.cinema.beat)
    }
    /// Yaw of the airship about +Z in radians, derived only from the flight clock: 0 at rest and
    /// during the boarding beat, the smoothed travel heading during the flight.
    pub fn ship_yaw(&self) -> f32 {
        ship_yaw_of(self.flying(), &self.air, &self.air_times, self.state.flight)
    }
    /// The airship's orientation (yaw only) that the ship, its Gnome and Alice are drawn with.
    pub fn ship_rotation(&self) -> Quat {
        if self.flying() {
            Quat::from_rotation_z(self.ship_yaw())
        } else {
            Quat::IDENTITY
        }
    }
    /// Where Alice sat during the flight before the ship yawed: the tag offset was never turned.
    fn legacy_ship_seat(&self) -> Vec3 {
        self.ship_attachment_turned(self.alice_tag, Quat::IDENTITY)
            .translation
    }
    fn ship_attachment(&self, tag: usize) -> Transform {
        self.ship_attachment_turned(tag, self.ship_rotation())
    }
    fn ship_attachment_turned(&self, tag: usize, yaw: Quat) -> Transform {
        let a = if self.flying() {
            &self.fly
        } else {
            &self.hover
        };
        let clock = if self.flying() {
            self.state.flight
        } else {
            self.state.age
        };
        let pose = self.rig.global_pose(&a.sample(clock, true))[tag];
        attach(self.ship_position(), yaw, pose, self.ship_scale)
    }
    pub fn ship_position(&self) -> Vec3 {
        if !self.flying() {
            return self.ship;
        }
        lerp_path(&self.air, &self.air_times, self.state.flight).0
    }
}
pub struct Art {
    pipe_pose: Option<(String, f32, bool, Transform, f32)>,
    burst: crate::particles::PickupBurst,
    pub(crate) alice: crate::npc::Puppet,
    torch: crate::npc::Puppet,
    gnome: crate::npc::Puppet,
    ship: crate::npc::Puppet,
    key: crate::weapons::Prop,
    material: crate::character::SkinMaterial,
}
impl Art {
    pub fn load(assets: &mut crate::assets::Assets) -> Result<Self> {
        let specs = crate::texture::read_materials(assets)?;
        let mut alice = crate::npc::Puppet::load(assets, "alice", cinema::ALICE_CLIPS, &specs)?;
        alice.show_attachments(false);
        Ok(Self {
            pipe_pose: None,
            burst: crate::particles::PickupBurst::load(assets)?,
            alice,
            torch: crate::npc::Puppet::load(
                assets,
                "c_torchgnome",
                &["idle", "run", "alert1"],
                &specs,
            )?,
            gnome: crate::npc::Puppet::load(assets, "c_gnomeold", cinema::GNOME_CLIPS, &specs)?,
            ship: crate::npc::Puppet::load(assets, "gnome_airship", &["hover", "fly"], &specs)?,
            key: crate::weapons::Prop::load(assets, "key_skeleton", &specs)?,
            material: crate::character::skin_material()?,
        })
    }
    pub fn story_pose(&mut self, story: &crate::story::Story) {
        self.alice
            .mouth(story.mouth(&["fakeplayer", "player", "alice"]));
        self.gnome
            .mouth(story.mouth(&["elder_gnome1", "leave_gnome", "airship_gnome"]));
    }
    pub fn draw(
        &mut self,
        p: &Pandemonium,
        fullbright: bool,
        atmosphere: &crate::environment::Atmosphere,
        camera: Vec3,
    ) {
        self.material.atmosphere(atmosphere, camera);
        self.material.bind();
        self.ship.draw(
            if p.flying() { "fly" } else { "hover" },
            if p.flying() {
                p.state.flight
            } else {
                p.state.age
            },
            true,
            Transform {
                translation: p.ship_position(),
                rotation: p.ship_rotation(),
            },
            1.,
            fullbright,
        );
        if p.state.departure {
            self.gnome.draw(
                "ride",
                if p.state.cinema.beat == Some(Beat::BoardShip) {
                    0.4
                } else {
                    p.state.flight
                },
                p.state.cinema.beat != Some(Beat::BoardShip),
                Transform {
                    translation: p.ship_attachment(p.gnome_tag).translation,
                    rotation: p.ship_rotation(),
                },
                1.,
                fullbright,
            );
        }
        self.draw_cast(p, fullbright);
        if !p.state.key {
            self.key.draw_frame(
                Transform {
                    translation: p.key,
                    rotation: Quat::from_rotation_z(p.state.age),
                },
                1.,
                fullbright,
                p.state.age,
                true,
            );
        }
        gl_use_default_material();
    }
}

fn check_wheel_geometry(p: &Pandemonium, map: &Bsp) -> Result<()> {
    let body = p.objects.iter().find(|o| o.name == "minecart").unwrap();
    for wheel in p
        .objects
        .iter()
        .filter(|o| o.name.starts_with("minecart_wheels"))
    {
        let inverse = body.rotation.conjugate();
        let mount = inverse * (wheel.origin - body.origin);
        ensure!(
            mount.distance(wheel.base - body.base) < 0.01,
            "{} detached from cart in {:?}",
            wheel.name,
            p.state.cart
        );
        ensure!(
            wheel.visible == body.visible,
            "Detached visible wheels after cart exit"
        );
        ensure!(
            (inverse * wheel.rotation * Vec3::Y).distance(Vec3::Y) < 0.001,
            "{} spins around the wrong axle",
            wheel.name
        );
        // Test the actual BSP vertices, not just a synthetic axle. Both discs
        // must keep their transverse positions and radial distance at every angle.
        for si in map.models[wheel.model].surfaces.clone() {
            let surface = &map.surfaces[si];
            for vertex in
                &map.vertices[surface.first_vertex..surface.first_vertex + surface.vertex_count]
            {
                let before = vertex.position;
                let after = inverse * (wheel.rotation * before);
                ensure!(
                    (after.y - before.y).abs() < 0.01
                        && (after.x.hypot(after.z) - before.x.hypot(before.z)).abs() < 0.01,
                    "Wheel geometry orbits through the cart bed"
                );
            }
        }
    }
    Ok(())
}

pub fn check(assets: &mut crate::assets::Assets) -> Result<()> {
    use crate::{interaction::Interactions, movement::FIXED_DT};
    let map = Bsp::parse(&assets.read("maps/pandemonium.bsp")?)?;
    let fresh = |assets: &mut crate::assets::Assets| -> Result<(Interactions, World)> {
        let mut i = Interactions::load(&map)?;
        i.set_entry(assets, &map, "pandemonium", None)?;
        let mut w = World::from_bsp(&map)?;
        i.sync(&mut w);
        Ok((i, w))
    };
    let (mut i, mut world) = fresh(assets)?;
    machinery::check(i.pandemonium.as_mut().unwrap(), &map)?;
    check_wheel_geometry(i.pandemonium.as_ref().unwrap(), &map)?;
    for at in [
        vec3(-3674., -2276., 700.),
        vec3(-3968., 2718., -232.),
        vec3(-3716., -2264., 712.),
    ] {
        let e = i.triggers(FIXED_DT, at, at);
        ensure!(
            e.teleport.is_none()
                && e.transition.is_none()
                && !i.pandemonium.as_ref().unwrap().state.departure,
            "Early gate consumed or exited"
        );
    }
    let mut player = Player::new(vec3(-4312., -2484., 536.));
    i.update(FIXED_DT, &map, &mut world, &player, Vec3::Y, false)?;
    ensure!(
        !i.pandemonium.as_ref().unwrap().state.key,
        "Key available before minecart"
    );
    for hz in [30, 60, 144] {
        let (mut i, mut world) = fresh(assets)?;
        let mut player = Player::new(vec3(-5024., 1368., 20.));
        i.triggers(FIXED_DT, player.feet, player.feet);
        ensure!(
            i.pandemonium.as_ref().unwrap().state.cart == Cart::Boarding,
            "Cart contact did not board"
        );
        let saved = serde_json::to_value(i.snapshot())?;
        i.advance_school(0., &map, &mut world, &mut player)?;
        ensure!(
            serde_json::to_value(i.snapshot())? == saved,
            "Pause advanced machinery"
        );
        for _ in 0..hz * 65 {
            i.advance_school(1. / hz as f32, &map, &mut world, &mut player)?;
            check_wheel_geometry(i.pandemonium.as_ref().unwrap(), &map)?;
        }
        let p = i.pandemonium.as_ref().unwrap();
        ensure!(
            p.state.cart == Cart::Finished
                && p.state.rides == 1
                && player.feet.distance(p.end) < 0.1
                && world.body_clear(player.feet),
            "Cart landing at {hz} Hz"
        );
        i.triggers(FIXED_DT, player.feet, player.feet);
        i.activate_enemies();
        ensure!(
            i.encounters
                .as_ref()
                .unwrap()
                .actors
                .iter()
                .any(|a| a.name == "airship_cardguard1" && a.active),
            "Cart guard not activated"
        );
        println!("PASS Pandemonium cart lift/path/landing at {hz} Hz, wheel pivots and axle geometry stay bound throughout");
    }
    i.pandemonium
        .as_mut()
        .unwrap()
        .fixture("pand-key", &map, &mut world, &mut player)?;
    i.sync(&mut world);
    // Exercise the real key pickup after undoing only the staged key flag.
    i.pandemonium.as_mut().unwrap().state.key = false;
    player = Player::new(vec3(-4312., -2484., 536.));
    i.update(FIXED_DT, &map, &mut world, &player, Vec3::Y, false)?;
    ensure!(
        i.pandemonium.as_ref().unwrap().state.key,
        "Accessible key contact failed"
    );
    i.advance_school(0.1, &map, &mut world, &mut player)?;
    let p = i.pandemonium.as_mut().unwrap();
    let before = p.snapshot();
    let mut invalid = before.clone();
    invalid.key = false;
    invalid.returned = true;
    ensure!(
        p.restore(&invalid, &map).is_err(),
        "Invalid saved gate combination accepted"
    );
    ensure!(
        serde_json::to_value(p.snapshot())? == serde_json::to_value(before)?,
        "Failed restore mutated state"
    );
    // Opening solid doors stop instead of advancing through Alice.
    {
        let (mut trial, mut w) = fresh(assets)?;
        let p = trial.pandemonium.as_mut().unwrap();
        let mut rider = Player::new(p.end);
        p.fixture("pand-key", &map, &mut w, &mut rider)?;
        p.state.key_time = 0.6;
        p.rebuild(&map)?;
        w.set_dynamic(p.colliders().collect());
        let before = p.snapshot();
        p.state.key_time = 0.7;
        p.rebuild(&map)?;
        let door = &p
            .objects
            .iter()
            .find(|o| o.name == "guard_last_door")
            .unwrap()
            .collider;
        let mut obstruction = None;
        'scan: for x in (-3940..-3680).step_by(4) {
            for y in (-2380..-2150).step_by(4) {
                let feet = vec3(x as f32, y as f32, 664.05);
                let center = feet + PLAYER_CENTER;
                if w.body_clear(feet) && door.trace(center, center, PLAYER_HALF).start_solid {
                    obstruction = Some(feet);
                    break 'scan;
                }
            }
        }
        let feet = obstruction.context("No swept-door obstruction fixture")?;
        p.restore(&before, &map)?;
        rider = Player::new(feet);
        p.advance(0.1, &map, &mut w, &mut rider, &[])?;
        ensure!(
            p.state.key_time == before.key_time && w.body_clear(rider.feet),
            "Key door crushed its blocker"
        );
        rider = Player::new(p.end);
        p.advance(0.1, &map, &mut w, &mut rider, &[])?;
        ensure!(
            p.state.key_time > before.key_time,
            "Key door did not resume after obstruction cleared"
        );
    }
    // A saved rope grip must continue with the same constrained movement and detach on E.
    {
        let (mut trial, mut w) = fresh(assets)?;
        let mut p = Player::new(vec3(-3498.4, 2082.8, -32.7));
        trial.update(FIXED_DT, &map, &mut w, &p, Vec3::Y, true)?;
        for _ in 0..80 {
            trial.advance_school(FIXED_DT, &map, &mut w, &mut p)?;
            trial.pandemonium.as_mut().unwrap().control(
                FIXED_DT,
                &w,
                &mut p,
                Controls {
                    rise: -1.,
                    ..Default::default()
                },
            );
        }
        let saved = trial.snapshot();
        let (mut restored, mut other_world) = fresh(assets)?;
        restored.restore(&saved, &map)?;
        restored.sync(&mut other_world);
        let mut other = p.clone();
        restored.pandemonium.as_ref().unwrap().validate_player(&p)?;
        for _ in 0..120 {
            let input = Controls {
                wish: Vec2::X,
                rise: 0.5,
                ..Default::default()
            };
            trial
                .pandemonium
                .as_mut()
                .unwrap()
                .control(FIXED_DT, &w, &mut p, input);
            restored.pandemonium.as_mut().unwrap().control(
                FIXED_DT,
                &other_world,
                &mut other,
                input,
            );
            ensure!(w.body_clear(p.feet), "Rope swing crossed solid geometry");
        }
        ensure!(p.feet == other.feet, "Saved rope diverged");
        other.knockback(Vec3::X * 80.);
        let pushed = other.velocity;
        ensure!(
            !restored.pandemonium.as_mut().unwrap().control(
                FIXED_DT,
                &other_world,
                &mut other,
                Controls::default()
            ) && !restored.pandemonium.as_ref().unwrap().controlled()
                && other.script_motion == 0
                && other.velocity == pushed,
            "Pandemonium grip swallowed the enemy impulse"
        );
        trial.update(FIXED_DT, &map, &mut w, &p, Vec3::Y, true)?;
        ensure!(
            !trial.pandemonium.as_ref().unwrap().controlled(),
            "E did not release rope"
        );
    }
    for phase in [
        "pand-cart",
        "pand-rail",
        "pand-key",
        "pand-return",
        "pand-flight",
        "pand-flight-start",
        "pand-flight-turn",
        "pand-flight-end",
    ] {
        let (mut i, mut world) = fresh(assets)?;
        let mut player = Player::new(vec3(-3378., 2460., 48.));
        i.pandemonium
            .as_mut()
            .unwrap()
            .fixture(phase, &map, &mut world, &mut player)?;
        let save = serde_json::from_slice(&serde_json::to_vec(&i.snapshot())?)?;
        let (mut restored, mut other_world) = fresh(assets)?;
        restored.restore(&save, &map)?;
        check_wheel_geometry(restored.pandemonium.as_ref().unwrap(), &map)?;
        ensure!(
            i.pandemonium
                .as_ref()
                .unwrap()
                .transforms()
                .collect::<Vec<_>>()
                == restored
                    .pandemonium
                    .as_ref()
                    .unwrap()
                    .transforms()
                    .collect::<Vec<_>>(),
            "Restored machinery pose differs for {phase}"
        );
        let mut other = player.clone();
        for _ in 0..60 {
            i.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
            restored.advance_school(FIXED_DT, &map, &mut other_world, &mut other)?;
        }
        ensure!(
            serde_json::to_value(i.snapshot())? == serde_json::to_value(restored.snapshot())?
                && player.feet == other.feet,
            "Restart diverged for {phase}"
        );
    }
    let portal = vec3(-3674., -2276., 700.);
    let e = i.triggers(FIXED_DT, portal, portal);
    ensure!(e.teleport.is_some(), "Key did not unlock return portal");
    // Return trigger precedes the portal in normal movement.
    i.triggers(
        FIXED_DT,
        vec3(-3716., -2264., 712.),
        vec3(-3716., -2264., 712.),
    );
    i.activate_enemies();
    ensure!(
        i.pandemonium.as_ref().unwrap().state.returned,
        "Return contact failed"
    );
    let cast = i.encounters.as_ref().unwrap();
    for name in ["return_di1", "return_di2", "club_spawn1"] {
        ensure!(
            cast.actors.iter().any(|a| a.name == name && a.active),
            "Missing return guard {name}"
        );
    }
    let end = vec3(-3968., 2718., -232.);
    i.triggers(FIXED_DT, end, end);
    ensure!(
        i.pandemonium.as_ref().unwrap().state.departure,
        "Departure blocked after key/return"
    );
    for _ in 0..2400 {
        i.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
    }
    ensure!(
        i.update(FIXED_DT, &map, &mut world, &player, Vec3::Y, false)?
            .transition
            .is_none(),
        "Exit skipped unfinished dialogue"
    );
    i.completed_dialogue("Pand_End_Ship");
    for _ in 0..1900 {
        i.advance_school(FIXED_DT, &map, &mut world, &mut player)?;
    }
    let e = i.update(FIXED_DT, &map, &mut world, &player, Vec3::Y, false)?;
    ensure!(
        e.transition == Some(("fortress1".into(), Some("fortress1_start1".into()))),
        "Wrong campaign exit"
    );
    ensure!(
        i.update(FIXED_DT, &map, &mut world, &player, Vec3::Y, false)?
            .transition
            .is_none(),
        "Duplicate campaign exit"
    );
    println!(
        "PASS Pandemonium gates, pickups, activations, pause, restart phases and one-shot exit"
    );
    check_ship_yaw(assets, &map)?;
    Ok(())
}

/// Smallest signed angle between two yaws, in radians.
fn yaw_delta(a: f32, b: f32) -> f32 {
    let d = (b - a).rem_euclid(std::f32::consts::TAU);
    if d > std::f32::consts::PI {
        d - std::f32::consts::TAU
    } else {
        d
    }
}

/// The airship turns along its flight path. Presentation only: the yaw is derived from the saved
/// flight clock, so the saved state and the flight timing are untouched.
fn check_ship_yaw(assets: &mut crate::assets::Assets, map: &Bsp) -> Result<()> {
    use crate::interaction::Interactions;
    let fresh = |assets: &mut crate::assets::Assets| -> Result<(Interactions, World)> {
        let mut i = Interactions::load(map)?;
        i.set_entry(assets, map, "pandemonium", None)?;
        let mut w = World::from_bsp(map)?;
        i.sync(&mut w);
        Ok((i, w))
    };
    // Rest pose: yaw exactly 0, no rotation, the ship at its authored waypoint.
    let (mut i, mut world) = fresh(assets)?;
    let p = i.pandemonium.as_ref().unwrap();
    ensure!(
        p.ship_yaw().to_bits() == 0
            && p.ship_rotation() == Quat::IDENTITY
            && p.ship_position() == p.ship,
        "Airship rest pose is not the authored one"
    );
    let segments = p.air.len() - 1;
    ensure!(
        segments == 5 && p.air_times.last() == Some(&15.),
        "Unexpected airship path shape"
    );
    let heading = |a: Vec3, b: Vec3| (b - a).y.atan2((b - a).x);
    let first = heading(p.air[0], p.air[1]);
    let last = heading(p.air[segments - 1], p.air[segments]);
    // Boarding beat: still the authored yaw 0 at the rest waypoint.
    let mut player = Player::new(vec3(-3378., 2460., 48.));
    let p = i.pandemonium.as_mut().unwrap();
    p.fixture("pand-board", map, &mut world, &mut player)?;
    ensure!(
        p.state.departure
            && p.state.cinema.beat == Some(Beat::BoardShip)
            && p.ship_yaw().to_bits() == 0
            && p.ship_rotation() == Quat::IDENTITY
            && p.ship_position() == p.ship,
        "Airship boarding pose is not the authored one"
    );
    let boarding_seat = p.ship_seat();
    let mut runs: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut report = String::new();
    for hz in [30u32, 60, 144, 60] {
        let (mut i, mut world) = fresh(assets)?;
        let mut player = Player::new(vec3(-3378., 2460., 48.));
        i.pandemonium
            .as_mut()
            .unwrap()
            .fixture("pand-board", map, &mut world, &mut player)?;
        i.completed_dialogue("Pand_End_Ship");
        let dt = 1. / hz as f32;
        {
            let p = i.pandemonium.as_ref().unwrap();
            ensure!(
                p.state.cinema.beat == Some(Beat::Flight) && p.state.flight == 0.,
                "Flight did not start at hz {hz}"
            );
            ensure!(
                (p.ship_yaw() - first).abs() < 1e-5,
                "Yaw at flight start {} is not the first segment heading {first}",
                p.ship_yaw()
            );
            // The rest-to-flight cut happens on the scene's black frame: both the position and the
            // yaw change at once, which is authored.
            ensure!(
                p.ship_position() == p.air[0] && boarding_seat.distance(p.ship_seat()) > 1.,
                "Flight does not start at the spline start"
            );
        }
        let mut yaws = Vec::new();
        let mut previous = i.pandemonium.as_ref().unwrap().ship_yaw();
        let mut worst = 0_f32;
        let mut closest = f32::MAX;
        let mut widest = 0_f32;
        let mut lowest = f32::MAX;
        let mut ticks = 0;
        let mut saved = None;
        while i.pandemonium.as_ref().unwrap().state.flight < 15. + 0.5 && ticks < hz * 20 {
            let before = player.feet;
            i.advance_school(dt, map, &mut world, &mut player)?;
            let e = i.triggers(dt, before, player.feet);
            ensure!(
                e.teleport.is_none() && e.transition.is_none(),
                "Flight seat fired a trigger at hz {hz}"
            );
            let p = i.pandemonium.as_ref().unwrap();
            let yaw = p.ship_yaw();
            yaws.push(yaw.to_bits());
            // At most about 100 degrees per second: the raw headings jump 54 degrees at a node.
            let step = yaw_delta(previous, yaw).abs();
            ensure!(
                step <= (100_f32).to_radians() * dt + 1e-4,
                "Yaw jumped {step} rad in one {hz} Hz tick at flight {}",
                p.state.flight
            );
            worst = worst.max(step / dt);
            previous = yaw;
            p.validate_player(&player)?;
            if saved.is_none() && p.state.flight >= 7.3 {
                saved = Some((p.snapshot(), player.clone(), yaw, p.ship_seat()));
            }
            ensure!(
                player.feet == p.ship_seat()
                    && player.script_facing == yaw
                    && world.body_clear(player.feet),
                "Rotated seat left the transport or entered solid geometry at flight {}",
                p.state.flight
            );
            // The seat rides the ship's rotation: its horizontal distance from the hull origin
            // never changes with yaw.
            let offset = (p.ship_seat() - p.ship_position()).truncate().length();
            closest = closest.min(offset);
            widest = widest.max(offset);
            // The chase camera keeps its authored world-space bearing and distance to the seat.
            if ticks % (hz / 10) == 0 {
                let c = p.camera().context("Flight camera missing")?;
                let d = c.eye.distance(c.target);
                let hull = p.ship_position() + Vec3::Z * 60.;
                let view = (c.target - c.eye).normalize();
                let ship = (hull - c.eye).normalize();
                ensure!(
                    (250. ..300.).contains(&d)
                        && view.dot(ship).acos().to_degrees() < 30.
                        && c.eye.truncate().distance(hull.truncate()) > 160.
                        && world.sweep(c.target, c.eye, Vec3::splat(2.)).fraction >= 1.,
                    "Chase camera does not frame the ship at flight {}",
                    p.state.flight
                );
                lowest = lowest.min(c.eye.z);
            }
            ticks += 1;
        }
        let p = i.pandemonium.as_ref().unwrap();
        ensure!(
            p.state.flight >= 15. && (p.ship_yaw() - last).abs() < 1e-5,
            "Yaw at flight end {} is not the last segment heading {last}",
            p.ship_yaw()
        );
        if let Some((snap, rider, yaw, seat)) = saved {
            // A mid-flight save restores the same yaw, seat and pose from the flight clock alone.
            let (mut j, mut other_world) = fresh(assets)?;
            j.pandemonium.as_mut().unwrap().restore(&snap, map)?;
            j.sync(&mut other_world);
            let q = j.pandemonium.as_ref().unwrap();
            ensure!(
                q.ship_yaw().to_bits() == yaw.to_bits()
                    && q.ship_seat() == seat
                    && q.validate_player(&rider).is_ok(),
                "Restored flight differs from the live flight at {hz} Hz"
            );
            let mut off = rider.clone();
            off.feet.x += 1.;
            ensure!(
                q.validate_player(&off).is_err(),
                "validate_player accepted a seat 1 unit off the ship"
            );
            // A flight save written before the ship yawed seats Alice at the unturned position.
            // It still loads, and the next tick puts her on the turned seat.
            let mut old = rider.clone();
            old.feet = q.legacy_ship_seat();
            ensure!(
                q.legacy_ship_seat().distance(seat) > 1. && q.validate_player(&old).is_ok(),
                "A pre-yaw flight save no longer loads at {hz} Hz"
            );
            j.advance_school(dt, map, &mut other_world, &mut old)?;
            ensure!(
                old.feet == j.pandemonium.as_ref().unwrap().ship_seat()
                    && j.pandemonium
                        .as_ref()
                        .unwrap()
                        .validate_player(&old)
                        .is_ok(),
                "A pre-yaw flight save did not move to the turned seat at {hz} Hz"
            );
        }
        report.push_str(&format!(
            " {hz} Hz max {:.1} deg/s, seat radius {:.1}-{:.1}, camera z >= {:.0};",
            worst.to_degrees(),
            closest,
            widest,
            lowest
        ));
        runs.push((hz, yaws));
    }
    let a = runs.iter().find(|r| r.0 == 60).unwrap();
    let b = runs.iter().rfind(|r| r.0 == 60).unwrap();
    ensure!(a.1 == b.1, "Airship yaw is not deterministic");
    println!(
        "PASS Pandemonium airship yaw: rest/boarding 0 exactly, {first:.4} to {last:.4} rad along the path, continuous at 30/60/144 Hz,{report} seat, validate_player, camera and restore consistent"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The six nodes of the departure path (map coordinates), three seconds apart.
    fn path() -> (Vec<Vec3>, Vec<f32>) {
        let points = vec![
            vec3(-2800., -1128., 816.),
            vec3(-2616., -1008., 832.),
            vec3(-2600., -656., 816.),
            vec3(-2728., -352., 840.),
            vec3(-2760., -136., 864.),
            vec3(-2936., 136., 912.),
        ];
        let times = (0..points.len())
            .map(|i| i as f32 * FLIGHT_NODE_TIME)
            .collect();
        (points, times)
    }
    fn segment_yaw(points: &[Vec3], i: usize) -> f32 {
        let d = points[i + 1] - points[i];
        d.y.atan2(d.x)
    }
    fn flight_ticks(hz: u32) -> impl Iterator<Item = f32> {
        let dt = 1. / hz as f32;
        let mut t = 0_f32;
        std::iter::from_fn(move || {
            let now = t;
            t += dt;
            (now < 16.).then_some(now)
        })
    }

    #[test]
    fn rest_and_boarding_yaw_are_exactly_zero() {
        let (points, times) = path();
        assert!(!is_flying(false, None));
        assert!(!is_flying(false, Some(Beat::Flight)));
        assert!(!is_flying(true, Some(Beat::BoardShip)));
        assert!(is_flying(true, Some(Beat::Flight)));
        assert!(is_flying(true, None));
        for flight in [0., 1.5, 7., 15., 99.] {
            assert_eq!(ship_yaw_of(false, &points, &times, flight).to_bits(), 0);
        }
        assert_ne!(ship_yaw_of(true, &points, &times, 0.).to_bits(), 0);
    }

    #[test]
    fn start_and_end_match_the_first_and_last_segments() {
        let (points, times) = path();
        assert!((flight_yaw(&points, &times, 0.) - segment_yaw(&points, 0)).abs() < 1e-6);
        let last = segment_yaw(&points, points.len() - 2);
        for t in [15., 15.5, 1e6] {
            assert!((flight_yaw(&points, &times, t) - last).abs() < 1e-6);
        }
        assert!((flight_yaw(&points, &times, 0.).to_degrees() - 33.1).abs() < 0.1);
        assert!((last.to_degrees() - 122.9).abs() < 0.1);
    }

    #[test]
    fn yaw_follows_each_segment_between_the_blend_windows() {
        let (points, times) = path();
        for (i, t) in [(0, 1.), (1, 4.5), (2, 7.5), (3, 10.5), (4, 13.5)] {
            assert!((flight_yaw(&points, &times, t) - segment_yaw(&points, i)).abs() < 1e-6);
        }
        // The blend window is 0.5 s each side of a node: its edges are the plain headings.
        for node in 1..points.len() - 1 {
            let at = times[node];
            assert!(
                (flight_yaw(&points, &times, at - YAW_BLEND) - segment_yaw(&points, node - 1))
                    .abs()
                    < 1e-6
            );
            assert!(
                (flight_yaw(&points, &times, at + YAW_BLEND) - segment_yaw(&points, node)).abs()
                    < 1e-6
            );
        }
    }

    #[test]
    fn node_heading_is_the_normalised_mean_of_its_neighbours() {
        let (points, times) = path();
        for node in 1..points.len() - 1 {
            let a = segment_yaw(&points, node - 1);
            let b = segment_yaw(&points, node);
            let mean = (Vec2::from_angle(a) + Vec2::from_angle(b)).normalize();
            let yaw = flight_yaw(&points, &times, times[node]);
            assert!((yaw - mean.y.atan2(mean.x)).abs() < 1e-5, "node {node}");
        }
    }

    #[test]
    fn yaw_is_continuous_at_every_tick_rate() {
        let (points, times) = path();
        for hz in [30u32, 60, 144] {
            let dt = 1. / hz as f32;
            let mut worst = 0_f32;
            let mut previous = flight_yaw(&points, &times, 0.);
            for t in flight_ticks(hz) {
                let yaw = flight_yaw(&points, &times, t);
                let step = (yaw - previous).abs();
                assert!(step < std::f32::consts::PI);
                worst = worst.max(step / dt);
                previous = yaw;
            }
            // The raw headings jump 54, 26, 15 and 25 degrees at the nodes; the blend spreads each
            // of them over a second, so the turn rate stays under 100 degrees per second.
            assert!(worst.to_degrees() < 100., "{hz} Hz turned {worst} rad/s");
            assert!(
                worst.to_degrees() > 10.,
                "the blend must still turn the ship"
            );
        }
    }

    #[test]
    fn yaw_is_deterministic() {
        let (points, times) = path();
        let run = || {
            flight_ticks(60)
                .map(|t| flight_yaw(&points, &times, t).to_bits())
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn a_vertical_hop_borrows_the_neighbouring_heading() {
        let points = vec![
            Vec3::ZERO,
            Vec3::X * 100.,
            Vec3::new(100., 0., 50.),
            Vec3::new(100., 100., 50.),
        ];
        let times = vec![0., 3., 6., 9.];
        // Segment 1 climbs straight up: it keeps the heading of a horizontal neighbour.
        assert_eq!(segment_heading(&points, 1), Vec2::X);
        for t in [0., 2., 4.5, 5.9, 6.4, 8., 9., 20.] {
            assert!(flight_yaw(&points, &times, t).is_finite());
        }
        assert!(flight_yaw(&points, &times, 4.5).abs() < 1e-6);
        assert!((flight_yaw(&points, &times, 8.) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        // A path with no horizontal travel at all falls back to the authored nose direction.
        let up = vec![Vec3::ZERO, Vec3::Z * 10.];
        assert_eq!(flight_yaw(&up, &[0., 3.], 1.), 0.);
    }

    #[test]
    fn a_reversal_never_produces_a_non_finite_yaw() {
        let points = vec![Vec3::ZERO, Vec3::X * 100., Vec3::ZERO];
        let times = vec![0., 3., 6.];
        for t in flight_ticks(144) {
            assert!(flight_yaw(&points, &times, t).is_finite());
        }
    }

    #[test]
    fn a_single_segment_path_holds_one_heading() {
        let points = vec![Vec3::ZERO, Vec3::new(0., 100., 0.)];
        let times = vec![0., 3.];
        for t in [0., 1.5, 3., 10.] {
            assert!((flight_yaw(&points, &times, t) - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        }
    }

    #[test]
    fn attachment_is_unchanged_at_rest_and_turns_with_the_ship() {
        let tag = Transform {
            translation: vec3(38., 0., -28.),
            rotation: Quat::from_rotation_y(0.3),
        };
        let position = vec3(-2736., -1128., 816.);
        let rest = attach(position, Quat::IDENTITY, tag, 1.19);
        assert_eq!(rest.translation, position + tag.translation * 1.19);
        assert_eq!(rest.rotation, tag.rotation);
        let quarter = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let turned = attach(position, quarter, tag, 1.);
        assert!(turned.translation.distance(position + vec3(0., 38., -28.)) < 1e-4);
        assert!(turned.rotation.dot(quarter * tag.rotation).abs() > 0.999_999);
        // Whatever the yaw, the seat keeps its height and its distance from the hull origin, so
        // the rotated seat cannot wander away from the ship.
        for k in 0..72 {
            let yaw = Quat::from_rotation_z((k as f32 * 5.).to_radians());
            let seat = attach(position, yaw, tag, 1.);
            let d = seat.translation - position;
            assert!((d.z + 28.).abs() < 1e-4);
            assert!((d.truncate().length() - 38.).abs() < 1e-3);
        }
    }
}

//! Reviewed ambient/escort movement only; no inferred quest rewards or exit unlocks.
use super::*;
use crate::collision::Collider;
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub(super) enum Role {
    Runner,
    Prelude,
    Chase,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct State {
    pub role: Role,
    stage: u8,
    node: usize,
    debt: f64,
    time: f32,
    hold: f32,
    inside_hold: bool,
    moving: bool,
    running: bool,
    grounded: bool,
    /// A short visible trail lets the follower round a corner it just saw Alice cross.
    #[serde(default)]
    trail: Vec<Vec3>,
    #[serde(default)]
    path_retry: f32,
}
impl State {
    pub fn new(map: &str, spawn: &Spawn) -> Option<Self> {
        let role = match (map, spawn.name.as_str(), spawn.model.as_str()) {
            ("funhouse", _, "c_insanechild-runner") => Role::Runner,
            ("hedge1", "seek_kid", "c_insanechild_muzzle") => Role::Prelude,
            ("hedge1", "seek_kid_chase", "c_insanechild_chase") => Role::Chase,
            _ => return None,
        };
        Some(Self {
            role,
            stage: 0,
            node: 0,
            debt: 0.,
            time: 0.,
            hold: 0.,
            inside_hold: false,
            moving: false,
            running: false,
            grounded: false,
            trail: vec![],
            path_retry: 0.,
        })
    }
    pub fn valid(&self, spawn: &Spawn) -> bool {
        let identity = match self.role {
            Role::Runner => spawn.model == "c_insanechild-runner",
            Role::Prelude => spawn.name == "seek_kid" && spawn.model == "c_insanechild_muzzle",
            Role::Chase => spawn.name == "seek_kid_chase" && spawn.model == "c_insanechild_chase",
        };
        identity
            && self.stage <= 3
            && self.node < 4
            && (0. ..0.009).contains(&self.debt)
            && (0. ..=3600.).contains(&self.time)
            && (0. ..=5.).contains(&self.hold)
            && (self.role != Role::Chase || self.stage <= 1)
            && self.trail.len() <= 32
            && self
                .trail
                .iter()
                .all(|p| p.is_finite() && p.abs().max_element() < 1e6)
            && (self.role == Role::Chase || self.trail.is_empty())
            && (0. ..=0.25).contains(&self.path_retry)
    }
    pub fn visible(&self) -> bool {
        match self.role {
            Role::Prelude => self.stage != 3,
            Role::Chase => self.stage == 1,
            Role::Runner => true,
        }
    }
    pub fn holding(&self) -> bool {
        self.role == Role::Chase && self.visible() && self.hold > 0.
    }
    pub fn clip(&self) -> &'static str {
        if self.moving {
            if self.running || self.role == Role::Runner {
                "run_panic"
            } else {
                "walk"
            }
        } else if self.role == Role::Prelude {
            "idle01"
        } else {
            "idle"
        }
    }
}
#[derive(Default)]
pub(super) struct Routes {
    actor_world: Option<World>,
    runner: Vec<Vec3>,
    transfer: Vec<Vec3>,
    end: Vec<Vec3>,
    start_trigger: Option<Collider>,
    end_trigger: Option<Collider>,
    hold_trigger: Option<Collider>,
}
fn point(map: &Bsp, name: &str) -> Result<Vec3> {
    map.entities
        .iter()
        .find(|e| e.get("targetname").is_some_and(|s| s == name))
        .and_then(|e| e.get("origin"))
        .and_then(|s| vector(s))
        .with_context(|| format!("Missing companion point {name}"))
}
fn trigger(map: &Bsp, name: &str) -> Result<Option<Collider>> {
    let Some(e) = map.entities.iter().find(|e| {
        e.get("thread")
            .is_some_and(|s| s.trim_end_matches("()") == name)
    }) else {
        return Ok(None);
    };
    let id = e
        .get("model")
        .and_then(|s| s.strip_prefix('*'))
        .and_then(|s| s.parse::<usize>().ok())
        .context("Companion trigger model missing")?;
    Ok(Some(Collider::model(
        map,
        id,
        e.get("origin")
            .and_then(|s| vector(s))
            .unwrap_or(Vec3::ZERO),
        Quat::IDENTITY,
        false,
    )?))
}
impl Routes {
    pub fn load(map: &Bsp, name: &str) -> Result<Self> {
        if name == "funhouse" {
            let names = ["t191", "t192", "t189", "t190"];
            let mut runner = Vec::new();
            for (i, name) in names.iter().enumerate() {
                let e = map
                    .entities
                    .iter()
                    .find(|e| e.get("targetname").is_some_and(|s| s == name))
                    .context("Runner path missing")?;
                ensure!(
                    e.get("target").is_some_and(|s| s == names[(i + 1) % 4]),
                    "Runner path changed"
                );
                runner.push(point(map, name)?);
            }
            return Ok(Self {
                runner,
                actor_world: Some(World::actor_world(map)?),
                ..Default::default()
            });
        }
        if name != "hedge1" {
            return Ok(Self::default());
        }
        Ok(Self {
            actor_world: Some(World::actor_world(map)?),
            transfer: vec![point(map, "seek_middle")?, point(map, "seek_tele_path")?],
            end: vec![point(map, "seek_end")?],
            start_trigger: trigger(map, "SeekStart")?,
            end_trigger: trigger(map, "SeekEnd")?,
            hold_trigger: trigger(map, "SeekHold")?,
            ..Default::default()
        })
    }
    pub fn update(
        &mut self,
        actors: &mut [Actor],
        dt: f32,
        world: &World,
        eye: Vec3,
        speed_for: impl Fn(usize, &str) -> f32,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let world = if let Some(actors) = &mut self.actor_world {
            actors.copy_dynamic_from(world);
            &*actors
        } else {
            world
        };
        let touch = |c: &Option<Collider>, p: Vec3, half: Vec3| {
            c.as_ref().is_some_and(|c| c.trace(p, p, half).start_solid)
        };
        let start = touch(
            &self.start_trigger,
            eye - Vec3::Z * 20.,
            crate::collision::PLAYER_HALF,
        );
        let end = touch(
            &self.end_trigger,
            eye - Vec3::Z * 20.,
            crate::collision::PLAYER_HALF,
        );
        let mut handoff = actors.iter().any(|a| {
            a.guide
                .as_ref()
                .is_some_and(|g| g.role == Role::Prelude && g.stage == 3)
        });
        for actor in actors.iter_mut() {
            let Some(mut s) = actor.guide.take() else {
                continue;
            };
            if s.role == Role::Prelude && s.stage < 3 {
                if start && s.stage != 1 {
                    s.stage = 1;
                    s.node = 0;
                    s.time = 0.;
                } else if end && s.stage == 0 {
                    s.stage = 2;
                    s.node = 0;
                    s.time = 0.;
                }
            }
            if s.role == Role::Chase && handoff {
                s.stage = 1;
            }
            s.debt += dt.min(0.1) as f64;
            while s.debt + 1e-9 >= 1. / 120. {
                s.debt = (s.debt - 1. / 120.).max(0.);
                if !s.visible() {
                    continue;
                }
                let half = vec3(16., 16., 30.) * actor.spawn.scale;
                if !s.grounded {
                    actor.footing = world
                        .actor_footing(actor.position(), Vec3::Z * half.z, half, 1024.)
                        .or(Some(actor.position()));
                    s.grounded = true;
                }
                let from = actor.position();
                let inside = touch(&self.hold_trigger, from + Vec3::Z * half.z, half);
                s.inside_hold = inside;
                s.hold = (s.hold - 1. / 120.).max(0.);
                s.path_retry = (s.path_retry - 1. / 120.).max(0.);
                let route: &[Vec3] = match s.role {
                    Role::Runner => &self.runner,
                    Role::Prelude if s.stage == 1 => &self.transfer,
                    Role::Prelude if s.stage == 2 => &self.end,
                    _ => &self.end[..0],
                };
                let target = if s.role == Role::Chase {
                    Some(eye - Vec3::Z * 48.)
                } else {
                    route.get(s.node).copied()
                };
                let mut delta = target.unwrap_or(from) - from;
                let distance = delta.truncate().length();
                let sight = world.sweep(from + Vec3::Z * 45., eye, Vec3::splat(0.5));
                let visible = !sight.start_solid && sight.fraction >= 1.;
                if s.role == Role::Chase {
                    if distance > 300. || distance < 60. {
                        s.trail.clear();
                    } else if visible {
                        let goal = target.unwrap();
                        if s.trail.len() < 32
                            && s.trail
                                .last()
                                .is_none_or(|p| p.truncate().distance(goal.truncate()) >= 16.)
                        {
                            s.trail.push(goal);
                        }
                    }
                    while s
                        .trail
                        .first()
                        .is_some_and(|p| p.truncate().distance(from.truncate()) < 6.)
                    {
                        s.trail.remove(0);
                    }
                    if let Some(p) = s.trail.first() {
                        delta = *p - from;
                    }
                }
                // The authored monsters-only volume refreshes Hold while occupied.
                // Only this child's own contact can latch it; distant enemy contacts cannot strand it.
                let approaching =
                    (60. ..=300.).contains(&distance) && (visible || !s.trail.is_empty());
                if s.role == Role::Chase && inside && (s.hold > 0. || approaching) {
                    s.hold = 5.;
                }
                let move_now = target.is_some()
                    && s.hold == 0.
                    && !actor.speaking
                    && actor.greeting <= 0.
                    && (s.role != Role::Chase
                        || (60. ..=300.).contains(&distance) && (visible || !s.trail.is_empty()));
                s.running = s.role != Role::Chase || distance >= 150.;
                s.moving = move_now;
                if move_now {
                    let speed = speed_for(actor.model, s.clip()).clamp(20., 320.);
                    let step = delta.with_z(0.).normalize_or_zero()
                        * (speed / 120.).min(delta.truncate().length());
                    let to = walk_route(world, from, step, half);
                    s.moving = to.distance_squared(from) > 0.00001;
                    if !s.moving && s.role == Role::Chase && s.path_retry == 0. {
                        s.path_retry = 0.25;
                        // Alice may double back during combat. Rejoin only a later
                        // remembered point whose complete supported path is clear.
                        if let Some(index) = s.trail.iter().enumerate().rev().find_map(|(i, p)| {
                            (i > 0 && route_clear(world, from, *p, half)).then_some(i)
                        }) {
                            s.trail.drain(..index);
                        }
                    }
                    if s.moving {
                        actor.footing = Some(to);
                        actor.yaw = delta.y.atan2(delta.x);
                    }
                    if s.role != Role::Chase
                        && to.truncate().distance(target.unwrap().truncate()) <= 2.
                    {
                        s.node += 1;
                        if s.node >= route.len() {
                            s.node = 0;
                            if s.role == Role::Prelude {
                                if s.stage == 1 {
                                    s.stage = 3;
                                    handoff = true;
                                } else {
                                    s.stage = 0;
                                }
                                s.moving = false;
                            }
                        }
                    }
                }
                s.time = (s.time + 1. / 120.).rem_euclid(3600.);
                actor.time = s.time;
            }
            actor.guide = Some(s);
        }
        if handoff {
            for a in actors {
                if let Some(s) = &mut a.guide {
                    if s.role == Role::Chase {
                        s.stage = 1;
                    }
                }
            }
        }
    }
}

/// Scripted waypoints cross narrow, chamfered steps. Require support under the
/// centre and sweep the whole body; requiring all four corners rejects those steps.
fn walk_route(world: &World, feet: Vec3, delta: Vec3, half: Vec3) -> Vec3 {
    let center = feet + Vec3::Z * (half.z + 0.1);
    let up = world.sweep(center, center + Vec3::Z * 16., half);
    if up.start_solid {
        return feet;
    }
    let raised = center + Vec3::Z * 16. * up.fraction;
    let across = world.sweep(raised, raised + delta, half);
    if across.start_solid || across.fraction < 1. {
        return feet;
    }
    let down = world.sweep(raised + delta, raised + delta - Vec3::Z * 34., half);
    if down.start_solid || down.fraction >= 1. || down.normal.z < 0.65 {
        return feet;
    }
    let next = raised + delta - Vec3::Z * (34. * down.fraction + half.z);
    let support = world.sweep(next + Vec3::Z * 2., next - Vec3::Z * 17., Vec3::ZERO);
    if support.start_solid
        || support.fraction >= 1.
        || support.normal.z < 0.65
        || world.liquid_at(next + Vec3::Z * 4.) != 0
    {
        return feet;
    }
    next
}

fn route_clear(world: &World, from: Vec3, to: Vec3, half: Vec3) -> bool {
    let delta = (to - from).with_z(0.);
    if delta.length() > 350. {
        return false;
    }
    let steps = (delta.length() / 12.).ceil().max(1.) as usize;
    let step = delta / steps as f32;
    let mut at = from;
    for _ in 0..steps {
        let next = walk_route(world, at, step, half);
        if next.truncate().distance((at + step).truncate()) > 0.1 {
            return false;
        }
        at = next;
    }
    (at.z - to.z).abs() < 18.
}

fn fixture(spawn: Spawn, map: &str) -> Actor {
    Actor {
        guide: State::new(map, &spawn),
        walk: None,
        watch: Default::default(),
        mouth: 0.,
        story_visible: false,
        speaking: false,
        ending_talk: false,
        model: 0,
        yaw: spawn.yaw,
        time: 0.,
        greeting: 0.,
        cooldown: 0.,
        guard: None,
        ant: None,
        chess: None,
        imp: None,
        clock: None,
        resident: None,
        electric: 0.,
        footing: None,
        falling: 0.,
        spawn,
    }
}
pub(super) fn check(assets: &mut Assets) -> Result<()> {
    for name in ["funhouse", "hedge1"] {
        let map = Bsp::parse(&assets.read(&format!("maps/{name}.bsp"))?)?;
        let world = World::from_bsp(&map)?;
        let mut routes = Routes::load(&map, name)?;
        let mut initial = Vec::new();
        let mut data = Vec::new();
        for s in placements(&map, name, None, false) {
            if State::new(name, &s).is_some() {
                let mut a = fixture(s, name);
                a.model = data.len();
                data.push(Data::load(
                    assets,
                    &a.spawn.model,
                    &[
                        "idle".into(),
                        "idle01".into(),
                        "walk".into(),
                        "run_panic".into(),
                    ],
                )?);
                initial.push(a);
            }
        }
        ensure!(
            initial.len() == if name == "funhouse" { 1 } else { 2 },
            "Companion cast changed"
        );
        let eye = if name == "hedge1" {
            let e = map
                .entities
                .iter()
                .find(|e| e.get("thread").is_some_and(|s| s == "SeekStart"))
                .context("Missing escort trigger")?;
            let i = e["model"].trim_start_matches('*').parse::<usize>()?;
            vector(&e["origin"]).unwrap()
                + (map.models[i].min + map.models[i].max) * 0.5
                + Vec3::Z * 20.
        } else {
            Vec3::splat(10000.)
        };
        let speed = |i: usize, c: &str| {
            data[i].clips[c].distance * data[i].def.scale / data[i].clips[c].duration()
        };
        let mut ends = Vec::new();
        for hz in [30, 60, 144] {
            let mut actors = initial.clone();
            for _ in 0..hz * 30 {
                routes.update(&mut actors, 1. / hz as f32, &world, eye, speed);
            }
            for a in &actors {
                a.validate_save()?;
            }
            if name == "hedge1" {
                ensure!(
                    actors.iter().any(|a| a
                        .guide
                        .as_ref()
                        .is_some_and(|g| g.role == Role::Prelude && g.stage == 3)),
                    "Hedge child did not reach handoff: {:?}",
                    actors
                        .iter()
                        .map(|a| (
                            a.spawn.name.as_str(),
                            a.position(),
                            a.guide.as_ref().map(|g| (g.stage, g.node))
                        ))
                        .collect::<Vec<_>>()
                );
                ensure!(
                    actors.iter().any(|a| a
                        .guide
                        .as_ref()
                        .is_some_and(|g| g.role == Role::Chase && g.visible())),
                    "Chase child not activated"
                );
            } else {
                ensure!(
                    actors[0].position().distance(initial[0].position()) > 100.,
                    "Runner stayed still"
                );
            }
            let saved = serde_json::to_value(&actors)?;
            routes.update(&mut actors, 0., &world, eye, speed);
            ensure!(
                saved == serde_json::to_value(&actors)?,
                "Companion moved while paused"
            );
            let mut restored: Vec<Actor> = serde_json::from_value(saved)?;
            for _ in 0..240 {
                routes.update(&mut actors, 1. / 120., &world, eye, speed);
                routes.update(&mut restored, 1. / 120., &world, eye, speed);
            }
            ensure!(
                serde_json::to_value(&actors)? == serde_json::to_value(restored)?,
                "Companion save diverged"
            );
            ends.push(actors[0].position());
        }
        ensure!(
            ends.iter().all(|p| p.distance(ends[0]) < 0.1),
            "Companion frame-rate drift"
        );
        println!(
            "PASS {name} child route: real map, source clips, 30/60/144 Hz, pause and continuation"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn child(role: Role) -> Actor {
        let mut a = super::super::tests::actor();
        a.greeting = 0.;
        a.spawn.model = match role {
            Role::Runner => "c_insanechild-runner",
            Role::Prelude => "c_insanechild_muzzle",
            Role::Chase => "c_insanechild_chase",
        }
        .into();
        a.spawn.name = match role {
            Role::Prelude => "seek_kid",
            Role::Chase => "seek_kid_chase",
            _ => "",
        }
        .into();
        a.guide = State::new(
            if role == Role::Runner {
                "funhouse"
            } else {
                "hedge1"
            },
            &a.spawn,
        );
        a
    }
    fn floor() -> World {
        World::fixture(&[(vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.))])
    }
    #[test]
    fn hidden_escort_activates_only_after_checked_handoff_and_keeps_state_on_restore() {
        let mut routes = Routes {
            transfer: vec![vec3(80., 0., 0.)],
            start_trigger: Some(Collider::box_bounds(
                vec3(300., -20., 0.),
                vec3(340., 20., 100.),
            )),
            ..Default::default()
        };
        let mut actors = vec![child(Role::Prelude), child(Role::Chase)];
        routes.update(
            &mut actors,
            1. / 60.,
            &floor(),
            vec3(200., 0., 48.),
            |_, _| 100.,
        );
        assert!(!actors[1].guide.as_ref().unwrap().visible());
        for _ in 0..120 {
            routes.update(
                &mut actors,
                1. / 60.,
                &floor(),
                vec3(320., 0., 48.),
                |_, _| 100.,
            );
        }
        assert!(!actors[0].guide.as_ref().unwrap().visible());
        assert!(actors[1].guide.as_ref().unwrap().visible());
        let mut restored: Vec<Actor> =
            serde_json::from_value(serde_json::to_value(&actors).unwrap()).unwrap();
        for _ in 0..240 {
            routes.update(
                &mut actors,
                1. / 120.,
                &floor(),
                vec3(200., 0., 48.),
                |_, _| 100.,
            );
            routes.update(
                &mut restored,
                1. / 120.,
                &floor(),
                vec3(200., 0., 48.),
                |_, _| 100.,
            );
        }
        assert_eq!(
            serde_json::to_value(actors).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
    }
    #[test]
    fn chase_stops_for_cover_distance_and_five_second_hold() {
        let mut a = child(Role::Chase);
        a.guide.as_mut().unwrap().stage = 1;
        let mut routes = Routes {
            hold_trigger: Some(Collider::box_bounds(
                vec3(-20., -20., 0.),
                vec3(20., 20., 100.),
            )),
            ..Default::default()
        };
        let mut actors = vec![a];
        for _ in 0..240 {
            routes.update(
                &mut actors,
                1. / 60.,
                &floor(),
                vec3(200., 0., 48.),
                |_, _| 100.,
            );
        }
        assert!(actors[0].position().x.abs() < 0.1);
        for _ in 0..180 {
            routes.update(
                &mut actors,
                1. / 60.,
                &floor(),
                vec3(200., 0., 48.),
                |_, _| 100.,
            );
        }
        assert!(
            actors[0].position().x.abs() < 0.1,
            "Occupied plate must keep refreshing Hold"
        );
        actors[0].footing = Some(vec3(50., 0., 0.));
        for _ in 0..360 {
            routes.update(
                &mut actors,
                1. / 60.,
                &floor(),
                vec3(200., 0., 48.),
                |_, _| 100.,
            );
        }
        assert!(actors[0].position().x > 100.);
        let before = actors[0].position();
        routes.update(&mut actors, 0., &floor(), vec3(200., 0., 48.), |_, _| 100.);
        assert_eq!(before, actors[0].position());
        for _ in 0..120 {
            routes.update(
                &mut actors,
                1. / 60.,
                &floor(),
                vec3(900., 0., 48.),
                |_, _| 100.,
            );
        }
        assert_eq!(before, actors[0].position());
    }

    #[test]
    fn chase_remembers_visible_corners_with_bounded_saved_trail() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.)),
            (vec3(40., -200., 0.), vec3(80., 80., 200.)),
        ]);
        let mut a = child(Role::Chase);
        a.footing = Some(vec3(-100., 0., 0.1));
        a.guide.as_mut().unwrap().stage = 1;
        let mut actors = vec![a];
        let mut routes = Routes::default();
        let mut eye = vec3(0., 0., 48.);
        for target in [
            vec3(0., 130., 48.),
            vec3(150., 130., 48.),
            vec3(150., 0., 48.),
        ] {
            while eye.distance(target) > 0.1 {
                eye += (target - eye).normalize_or_zero() * eye.distance(target).min(0.8);
                routes.update(&mut actors, 1. / 120., &world, eye, |_, _| 180.);
                assert!(actors[0].guide.as_ref().unwrap().trail.len() <= 32);
                assert!(
                    !world
                        .sweep(
                            actors[0].position() + Vec3::Z * 30.1,
                            actors[0].position() + Vec3::Z * 30.1,
                            vec3(16., 16., 30.)
                        )
                        .start_solid
                );
            }
        }
        let saved = serde_json::to_value(&actors).unwrap();
        routes.update(&mut actors, 0., &world, eye, |_, _| 180.);
        assert_eq!(saved, serde_json::to_value(&actors).unwrap());
        let mut restored: Vec<Actor> = serde_json::from_value(saved).unwrap();
        for _ in 0..600 {
            routes.update(&mut actors, 1. / 120., &world, eye, |_, _| 180.);
            routes.update(&mut restored, 1. / 120., &world, eye, |_, _| 180.);
        }
        assert_eq!(
            serde_json::to_value(&actors).unwrap(),
            serde_json::to_value(restored).unwrap()
        );
        assert!(
            actors[0].position().truncate().distance(eye.truncate()) < 65.,
            "Child stalled at {:?}",
            actors[0].position()
        );
        assert!(actors[0].guide.as_ref().unwrap().valid(&actors[0].spawn));
    }

    #[test]
    fn chase_rejoins_a_doubled_back_trail_without_crossing_walls() {
        let world = World::fixture(&[
            (vec3(-1000., -1000., -20.), vec3(1000., 1000., 0.)),
            (vec3(40., -200., 0.), vec3(80., 80., 200.)),
        ]);
        let mut a = child(Role::Chase);
        let s = a.guide.as_mut().unwrap();
        s.stage = 1;
        s.trail = vec![vec3(150., 0., 0.), vec3(0., 130., 0.), vec3(150., 130., 0.)];
        let mut actors = vec![a];
        let mut routes = Routes::default();
        for _ in 0..1200 {
            routes.update(
                &mut actors,
                1. / 120.,
                &world,
                vec3(150., 130., 48.),
                |_, _| 180.,
            );
            let at = actors[0].position() + Vec3::Z * 30.1;
            assert!(!world.sweep(at, at, vec3(16., 16., 30.)).start_solid);
        }
        assert!(actors[0].position().truncate().distance(vec2(150., 130.)) < 65.);
    }
}

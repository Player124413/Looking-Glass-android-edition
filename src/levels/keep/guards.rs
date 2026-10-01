use super::*;
#[derive(Clone, Serialize, Deserialize)]
pub(super) enum Body {
    Basic(Guard),
    Card(crate::cards::Guard),
}
impl Body {
    pub fn identity(&self) -> (&'static str, f32) {
        match self {
            Self::Basic(g) => (if g.ranged { "diamond" } else { "club" }, g.scale),
            Self::Card(g) => (g.kind.model(), g.scale),
        }
    }
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Basic(g) => g.validate_save(),
            Self::Card(g) => g.validate(),
        }
    }
    pub fn health(&self) -> f32 {
        match self {
            Self::Basic(g) => g.health,
            Self::Card(g) => g.health,
        }
    }
    pub fn feet(&self) -> Vec3 {
        match self {
            Self::Basic(g) => g.feet,
            Self::Card(g) => g.feet,
        }
    }
    pub fn hit(&mut self, h: Hit) -> Option<&'static str> {
        match self {
            Self::Basic(g) => g.hit(h),
            Self::Card(g) => g.hit(h),
        }
    }
    pub fn target(&self, id: usize) -> Target {
        match self {
            Self::Basic(g) => g.target(id),
            Self::Card(g) => g.target(id),
        }
    }
}
impl Keep {
    pub(super) fn combat_guards(
        &mut self,
        c: &mut crate::level::Combat<'_>,
    ) -> crate::combat::Feedback {
        let mut out = crate::combat::Feedback::default();
        if self.scripted() || c.dt <= 0. {
            return out;
        }
        for a in &mut self.saved.spawned {
            a.electric = (a.electric - c.dt).max(0.);
            let mut f = crate::combat::Feedback::default();
            match &mut a.guard {
                Body::Basic(g) => {
                    g.notarget = c.notarget;
                    g.opponents.summon = c.summon;
                    f = g.advance(
                        c.dt,
                        c.world,
                        c.player.eye(),
                        if g.ranged {
                            self.data.diamond
                        } else {
                            self.data.guard
                        },
                    );
                }
                Body::Card(g) => {
                    g.notarget = c.notarget;
                    g.opponents.summon = c.summon;
                    a.accumulator += c.dt.min(0.1) as f64;
                    while a.accumulator >= crate::cards::STEP as f64 {
                        a.accumulator -= crate::cards::STEP as f64;
                        self.data.cards[usize::from(g.kind == crate::cards::Kind::Spade)].step(
                            g,
                            c.world,
                            c.player.eye(),
                            &mut f,
                        );
                    }
                }
            }
            out.will_drain += f.will_drain;
            out.cue_sounds.extend(f.cue_sounds);
            out.damage += f.damage;
            out.impulse += f.impulse;
            out.summon_hits.extend(f.summon_hits);
            out.spatial_sounds.extend(f.spatial_sounds);
            out.spatial_sounds
                .extend(f.sounds.into_iter().map(|s| (s, a.guard.feet())));
        }
        out
    }
}

//! Only reviewed named regions are enabled. Unnamed regions retain their legacy ordering.
use crate::interaction::Interactions;

pub struct HintSpec {
    pub map: &'static str,
    pub name: &'static str,
    pub entities: &'static [usize],
    pub enabled: fn(&Interactions) -> bool,
}

pub const NAMED: &[HintSpec] = &[
    HintSpec {
        map: "centipede2",
        name: "catmessage",
        entities: &[18],
        enabled: |i| i.levels.iter().any(|s| crate::event::Condition::flag("centipede2.climb").test(&s.ctl.facts())),
    },
    HintSpec {
        map: "potears1",
        name: "cat_after_talk",
        entities: &[46],
        enabled: |i| i.pool.as_ref().is_some_and(|p| p.state.talking),
    },
    HintSpec {
        map: "potears1",
        name: "cat_before_talk",
        entities: &[47, 728],
        enabled: |i| i.pool.as_ref().is_some_and(|p| !p.state.talking),
    },
];

pub fn named(map: &str, name: &str) -> Option<&'static HintSpec> {
    NAMED.iter().find(|s| s.map == map && s.name == name)
}

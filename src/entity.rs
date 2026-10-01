//! Stable identities within one BSP/visit. Names are groups, never unique identities.
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Id(pub usize);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Definition {
    pub id: Id,
    pub class: String,
    pub names: Vec<String>,
    pub model: String,
}

#[derive(Clone, Default, Serialize)]
pub struct Registry {
    pub definitions: Vec<Definition>,
    names: BTreeMap<String, Vec<Id>>,
}
impl Registry {
    pub fn new(entities: &[BTreeMap<String, String>]) -> Self {
        let mut registry = Self::default();
        for (index, e) in entities.iter().enumerate() {
            let id = Id(index);
            let mut names = Vec::new();
            for key in ["targetname", "spawntargetname"] {
                if let Some(name) = e.get(key).filter(|n| !n.is_empty()) {
                    if !names.contains(name) {
                        registry.names.entry(name.clone()).or_default().push(id);
                        names.push(name.clone());
                    }
                }
            }
            registry.definitions.push(Definition {
                id,
                names,
                class: e.get("classname").cloned().unwrap_or_default(),
                model: e.get("model").cloned().unwrap_or_default(),
            });
        }
        registry
    }
    pub fn contains(&self, id: Id) -> bool {
        self.definitions.get(id.0).is_some_and(|d| d.id == id)
    }
    pub fn named(&self, name: &str) -> &[Id] {
        self.names.get(name).map(Vec::as_slice).unwrap_or(&[])
    }
    pub fn validate(&self, ids: impl IntoIterator<Item = Id>) -> Result<()> {
        for id in ids {
            ensure!(self.contains(id), "Unknown entity {}", id.0);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_names_and_spawn_aliases_fan_out_in_map_order() {
        let r = Registry::new(&[
            BTreeMap::new(),
            BTreeMap::from([("targetname".into(), "guards".into())]),
            BTreeMap::from([
                ("targetname".into(), "guards".into()),
                ("spawntargetname".into(), "guards".into()),
            ]),
            BTreeMap::from([("spawntargetname".into(), "guards".into())]),
        ]);
        assert_eq!(r.named("guards"), &[Id(1), Id(2), Id(3)]);
        assert!(r.contains(Id(0))); // Unnamed entities retain identity too.
        assert!(!r.contains(Id(4)));
    }
}

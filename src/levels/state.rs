//! The saved-state pattern of a registered controller (F2).
//!
//! Every controller that keeps state across a save writes it under its own key in
//! `Snapshot.levels` and reads it back through this module, so the same rules hold for all 39
//! visits and a reviewer checks one template instead of thirty-nine. The pattern:
//!
//! * a serde struct with a `version: u8` first field, implementing [`State`];
//! * `#[serde(default)]` on **every field added after the first release**, so a save written by
//!   an older build of the same controller still loads (the field takes its default), and a
//!   bump of `State::VERSION` only when a default cannot express the migration;
//! * exhaustive validation in [`State::validate`]: clocks finite and bounded ([`clock`]),
//!   flags that must agree with each other checked as pairs, counters within their maxima, and
//!   every entry-dependent flag equal to the visit being restored ([`Visit`]);
//! * `snapshot()` calls [`save`] and `restore()` calls [`load`], nothing else. `load` builds a
//!   fresh value and only then replaces the live state, so a rejected save changes nothing.
//!
//! ```ignore
//! #[derive(Serialize, Deserialize)]
//! struct Saved {
//!     version: u8,
//!     returning: bool,       // entry-dependent: must equal the current visit
//!     clock: f32,            // a mover clock: finite and bounded
//!     open: bool,
//!     opened_at: Option<f32>, // consistent with `open`
//!     #[serde(default)]
//!     later: u32,            // added after release: defaults for older saves
//! }
//! impl State for Saved {
//!     const VERSION: u8 = 2;
//!     fn version(&self) -> u8 { self.version }
//!     fn validate(&self, visit: Visit) -> Result<()> {
//!         ensure!(self.returning == visit.returning, "Saved visit differs");
//!         clock("mover", self.clock, 600.)?;
//!         ensure!(self.open == self.opened_at.is_some(), "Inconsistent lever");
//!         Ok(())
//!     }
//! }
//! // in the controller:
//! fn snapshot(&self) -> serde_json::Value { state::save(&self.state) }
//! fn restore(&mut self, saved: &serde_json::Value, _: &Bsp) -> Result<()> {
//!     self.state = state::load(saved, self.visit)?;
//!     Ok(())
//! }
//! ```
//!
//! The file-level bounds (numbers finite and at most 1e12, strings at most 1 KiB, arrays at most
//! 10,000 elements, 8 MiB in all) are enforced by the save reader before a controller sees its
//! state; this module adds the per-controller ones.
use anyhow::{ensure, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

/// The visit a controller serves. Flags that depend on the entrance are validated against it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visit {
    /// The named return entrance (`wforest_start2`) rather than the first arrival.
    pub returning: bool,
}
impl Visit {
    /// The visit for the entrance a controller was loaded with. It follows the same rule as
    /// `save::visit_key`.
    pub fn of(map: &str, entry: Option<&str>) -> Self {
        Self {
            returning: super::returning(map, entry),
        }
    }
}

/// A controller's saved state.
pub trait State: Serialize + DeserializeOwned {
    /// The newest version this build writes and understands.
    const VERSION: u8;
    fn version(&self) -> u8;
    /// Check everything a saved value can get wrong. Called on every load.
    fn validate(&self, visit: Visit) -> Result<()>;
}

/// The value for `Controller::snapshot`.
pub fn save<T: State>(state: &T) -> Value {
    debug_assert_eq!(
        state.version(),
        T::VERSION,
        "state is written in the newest version"
    );
    serde_json::to_value(state).expect("controller state serializes")
}
/// Rebuild and validate a saved state for `Controller::restore`. Null (a stateless controller's
/// value), a missing or unknown version, a malformed body and any failed check are all errors.
pub fn load<T: State>(saved: &Value, visit: Visit) -> Result<T> {
    let version = saved
        .get("version")
        .and_then(Value::as_u64)
        .context("Saved level state has no version")?;
    ensure!(
        (1..=u64::from(T::VERSION)).contains(&version),
        "Saved level state has unsupported version {version}"
    );
    let state: T =
        serde_json::from_value(saved.clone()).context("Saved level state is malformed")?;
    ensure!(
        u64::from(state.version()) == version,
        "Saved level state version changed while loading"
    );
    state.validate(visit)?;
    Ok(state)
}

/// A clock or timer: finite, not negative and at most `max` seconds.
pub fn clock(name: &str, value: f32, max: f32) -> Result<()> {
    ensure!(
        value.is_finite() && (0. ..=max).contains(&value),
        "Invalid saved {name} clock"
    );
    Ok(())
}
/// A progress or blend value in `0..=1`.
pub fn fraction(name: &str, value: f32) -> Result<()> {
    clock(name, value, 1.)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    /// The template of the module documentation, in its second version.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Saved {
        version: u8,
        returning: bool,
        clock: f32,
        open: bool,
        opened_at: Option<f32>,
        #[serde(default)]
        later: u32,
    }
    impl State for Saved {
        const VERSION: u8 = 2;
        fn version(&self) -> u8 {
            self.version
        }
        fn validate(&self, visit: Visit) -> Result<()> {
            ensure!(self.returning == visit.returning, "Saved visit differs");
            clock("mover", self.clock, 600.)?;
            ensure!(self.open == self.opened_at.is_some(), "Inconsistent lever");
            if let Some(at) = self.opened_at {
                clock("lever", at, self.clock)?;
            }
            ensure!(self.later <= 1000, "Invalid saved counter");
            Ok(())
        }
    }
    const FIRST: Visit = Visit { returning: false };
    const RETURN: Visit = Visit { returning: true };
    fn good() -> Saved {
        Saved {
            version: 2,
            returning: false,
            clock: 12.5,
            open: true,
            opened_at: Some(4.),
            later: 7,
        }
    }

    #[test]
    fn a_state_round_trips_through_save_and_load() {
        let value = save(&good());
        assert_eq!(value["version"], 2);
        assert_eq!(load::<Saved>(&value, FIRST).unwrap(), good());
    }
    #[test]
    fn a_field_added_after_release_takes_its_default_from_an_older_save() {
        let older = json!({
            "version": 1, "returning": false, "clock": 3., "open": false, "opened_at": null
        });
        let loaded = load::<Saved>(&older, FIRST).unwrap();
        assert_eq!((loaded.version, loaded.later, loaded.open), (1, 0, false));
    }
    #[test]
    fn versions_from_the_future_the_past_or_nowhere_are_refused() {
        let mut value = save(&good());
        for version in [
            json!(3),
            json!(0),
            json!(255),
            json!(-1),
            json!(1.5),
            json!("2"),
        ] {
            value["version"] = version.clone();
            assert!(load::<Saved>(&value, FIRST).is_err(), "{version}");
        }
        assert!(load::<Saved>(&json!({ "clock": 1. }), FIRST).is_err());
        // A stateless controller saves null; a stateful one must never accept it.
        assert!(load::<Saved>(&Value::Null, FIRST).is_err());
        assert!(load::<Saved>(&json!([1, 2]), FIRST).is_err());
    }
    #[test]
    fn a_malformed_body_is_refused() {
        let mut value = save(&good());
        value["clock"] = json!("soon");
        assert!(load::<Saved>(&value, FIRST).is_err());
        let mut value = save(&good());
        value.as_object_mut().unwrap().remove("open");
        assert!(load::<Saved>(&value, FIRST).is_err());
    }
    #[test]
    fn clocks_must_be_finite_non_negative_and_bounded() {
        for bad in [-0.5, 600.5, 1e12, f64::NAN, f64::INFINITY] {
            let mut value = save(&good());
            // JSON has no NaN or infinity: serde_json writes them as null, which is refused too.
            value["clock"] = serde_json::Number::from_f64(bad).map_or(Value::Null, Value::Number);
            assert!(load::<Saved>(&value, FIRST).is_err(), "clock {bad}");
        }
        for ok in [0., 600.] {
            let mut value = save(&good());
            value["clock"] = json!(ok);
            value["opened_at"] = json!(0.);
            assert!(load::<Saved>(&value, FIRST).is_ok(), "clock {ok}");
        }
        assert!(clock("x", f32::NAN, 1.).is_err());
        assert!(clock("x", f32::INFINITY, f32::INFINITY).is_err());
        assert!(fraction("x", 1.01).is_err() && fraction("x", 1.).is_ok());
    }
    #[test]
    fn flags_that_disagree_or_leave_their_range_are_refused() {
        let mut value = save(&good());
        value["opened_at"] = Value::Null;
        assert!(load::<Saved>(&value, FIRST).is_err(), "open without a time");
        let mut value = save(&good());
        value["open"] = json!(false);
        assert!(load::<Saved>(&value, FIRST).is_err(), "a time without open");
        let mut value = save(&good());
        value["opened_at"] = json!(99.);
        assert!(load::<Saved>(&value, FIRST).is_err(), "opened after now");
        let mut value = save(&good());
        value["later"] = json!(1001);
        assert!(load::<Saved>(&value, FIRST).is_err());
    }
    #[test]
    fn an_entry_dependent_flag_must_equal_the_visit_being_restored() {
        let value = save(&good());
        assert!(load::<Saved>(&value, FIRST).is_ok());
        assert!(load::<Saved>(&value, RETURN).is_err());
        let mut returning = good();
        returning.returning = true;
        assert!(load::<Saved>(&save(&returning), RETURN).is_ok());
        assert!(load::<Saved>(&save(&returning), FIRST).is_err());
    }
    #[test]
    fn the_visit_follows_the_same_rule_as_the_save_key() {
        assert_eq!(Visit::of("wforest", None), FIRST);
        assert_eq!(Visit::of("wforest", Some("wforest_start1")), FIRST);
        assert_eq!(Visit::of("wforest", Some("wforest_start2")), RETURN);
        assert_eq!(Visit::of("skool1", Some("skool1_start2")), RETURN);
        // A map without a return entrance never returns, whatever its entrance is called.
        assert_eq!(Visit::of("garden1", Some("garden1_start2")), FIRST);
        for map in ["wforest", "skool1", "fortress1", "garden1"] {
            for entry in [
                None,
                Some("x"),
                Some("skool1_start2"),
                Some("wforest_start2"),
            ] {
                assert_eq!(
                    Visit::of(map, entry).returning,
                    crate::save::visit_key(map, entry).ends_with("$return"),
                    "{map} {entry:?}"
                );
            }
        }
    }
}

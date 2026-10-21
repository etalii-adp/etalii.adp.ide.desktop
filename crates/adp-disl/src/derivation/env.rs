//! What a derivation's expressions read as `env` (DISL section 12.3; the standalone host's `DislEnv`): whether the
//! diagram is read-only, the active viewpoint, and the moment of the gesture where the host gives one.

use adp_cel::{Map, Value};

/// The `env` of one derivation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DislEnv {
    /// Whether nothing may be edited: `env.readOnly`, and the reason a form row gives when no other applies.
    pub read_only: bool,
    /// The active viewpoint's id, `env.viewpoint`; empty when the specification has none.
    pub viewpoint: String,
    /// The moment of the gesture (DISL 0.4: `env.now`, an ISO 8601 text).
    pub now: Option<String>,
}

impl DislEnv {
    /// The value `env` is bound to.
    pub fn to_cel(&self) -> Map {
        let mut map = Map::default();
        map.insert("readOnly", Value::Bool(self.read_only));
        map.insert("viewpoint", Value::String(self.viewpoint.clone()));
        if let Some(now) = &self.now {
            map.insert("now", Value::String(now.clone()));
        }
        map
    }
}

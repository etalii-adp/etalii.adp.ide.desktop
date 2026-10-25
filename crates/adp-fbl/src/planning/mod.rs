//! Planning a model change as one edit (FBL section 6.4; the standalone host's `Planning`): the change a DISL
//! transaction makes, the splices it is planned as, or the reason it cannot be made. Planning writes nothing;
//! [`crate::history::OpenBody`] applies what it plans.

pub(crate) mod new_text;

/// .NET's invariant text of a double, which DISL reads a binding's numbers through too (DISL section 4.2).
pub use new_text::dotnet_double;
pub(crate) mod plan;
pub(crate) mod planner;

use adp_cel::Map;

use crate::splice::Splice;

/// One model change, as a DISL transaction makes it and as a conformance fixture's step names it (FBL sections 6.4
/// and 15.3). Planning turns it into one [`PlannedEdit`] or a refusal.
#[derive(Debug, Clone)]
pub enum ModelChange {
    /// Adds an element or relation; a relation's ends are the attributes `source` and `target`, by id. `index` is
    /// where among the parent's children it goes, which only a plugin can honour; `slot` is the containment slot
    /// of the parent, when the type can be written in several.
    Add {
        type_name: String,
        id: Option<String>,
        attributes: Map,
        parent_id: Option<String>,
        index: Option<i64>,
        slot: Option<String>,
    },
    /// Moves an element under `new_parent_id` (the top level when none), before the child now at `index` there,
    /// or after the last when the index is negative or past them. The index counts the new parent's children as
    /// they are before the move, the element itself included.
    Move {
        id: String,
        new_parent_id: Option<String>,
        index: i64,
    },
    /// Changes an element's type (DISL section 9.5 `retype`).
    Retype {
        id: String,
        type_name: String,
        attributes: Map,
    },
    /// Sets attributes of one element or relation; an empty string, an empty list or null empties an attribute.
    Set {
        id: String,
        attributes: Map,
    },
    Remove {
        id: String,
    },
    /// Places an element on the canvas: an edit of the registration, not of the body (FBL section 8.3).
    Place {
        id: String,
        x: f64,
        y: f64,
    },
    /// Stores the id of the element with a natural key in the registration's `identities` (FBL section 8.6).
    Identify {
        key: String,
        id: String,
    },
    /// Writes an id into an entry of the body that keeps its id in the body and has none yet.
    StoreId {
        id: String,
        new_id: String,
    },
    /// Changes an entry of a block of the registration that the binding declares (FBL 0.7 sections 8.1, 8.4, 15.3).
    Setting(Setting),
    /// Saves without a change: no splice (FBL section 15.3).
    Save,
}

/// A change to an entry of a declared block (FBL 0.7 sections 8.1, 8.4, 15.3): gives the entry `key` of `block` a new
/// `value`, moves it to `position` in a `kept` block, renames it to `rename`, or removes it. A setting with a position
/// and no value moves the entry; one with a value and no entry of that key adds it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Setting {
    pub block: String,
    pub key: String,
    pub value: Option<String>,
    pub position: Option<SettingPosition>,
    pub rename: Option<String>,
    pub remove_entry: bool,
}

/// Where an entry of a `kept` block goes (FBL 0.7 section 8.4, 15.3): at `index` among the block's entries as they are
/// without it, or directly after or before the entry with the key `after` or `before`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingPosition {
    pub index: Option<i64>,
    pub after: Option<String>,
    pub before: Option<String>,
}

/// One edit: the splices one model change was planned as, in body order, applied together, recorded together and
/// undone together (FBL section 6.4). Offsets refer to the body before the edit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlannedEdit {
    pub splices: Vec<Splice>,
    pub snapshot: bool,
}

/// What planning a change gives: an edit to apply, or the reason it cannot be made (FBL section 6.4). `Invalid` is
/// a change that names what the reading does not have, which the standalone host throws as an argument error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanResult {
    Planned(PlannedEdit),
    Refused(String),
    Invalid(String),
}

#[cfg(test)]
mod tests;

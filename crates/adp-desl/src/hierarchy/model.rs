//! The read model of a hierarchy surface (DESL section 7.7; the standalone host's `HierarchySurfaceModel` and its
//! members at a7fdf03, field for field): everything a page of nested boxes shows, sent whole on open and again after
//! every change. It carries no focus, narrowing, sort or selection: those are the view's and never written to a file.

use std::collections::BTreeMap;

/// What a [`SurfaceModel`] follows, so the view knows whether the file changed under an open field
/// (`HierarchySurfaceCauses`).
pub mod causes {
    /// The document was opened.
    pub const OPEN: &str = "open";
    /// A gesture of this connection's person was written.
    pub const EDIT: &str = "edit";
    /// The file was changed by another program, or by another connection.
    pub const OUTSIDE: &str = "outside";
}

/// The role a node plays (DESL section 7.7.1; `HierarchySurfaceRoles`): a level holds further nodes, an entry none.
pub mod roles {
    /// A type the surface's `levels` names, drawn as a box with what it holds to its right.
    pub const LEVEL: &str = "level";
    /// A type the surface's `entries` names, drawn inside the box of its level.
    pub const ENTRY: &str = "entry";
}

/// The read model of one document on a hierarchy surface.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceModel {
    /// The document's title, for the trail.
    pub title: String,
    /// Why nothing can be edited, or empty when the document can be.
    pub read_only_reason: String,
    /// What this model follows: one of [`causes`].
    pub cause: String,
    /// Every level and entry, in the file's order, each naming its parent.
    pub nodes: Vec<SurfaceNode>,
    /// The members of each label's list.
    pub option_lists: Vec<OptionList>,
    /// The stored filters, in their order.
    pub filters: Vec<SurfaceFilter>,
    /// Every note, open and resolved, in the file's order.
    pub notes: Vec<Note>,
    /// Everything reading reported, whatever the view narrows to.
    pub findings: Vec<SurfaceFinding>,
    /// The widths the user set of the columns of levels, by depth; a column not listed has the host's.
    pub widths: Vec<ColumnWidth>,
}

/// A level or an entry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceNode {
    /// What a gesture names it by: its number when that is usable, otherwise `line/<n>`, the line of its bullet,
    /// valid for this model only.
    pub id: String,
    /// Its number as written, a placeholder included.
    pub number: String,
    /// One of [`roles`].
    pub role: String,
    /// The specification's type.
    pub type_name: String,
    /// The id of the level it stands in, or empty at the top.
    pub parent_id: String,
    pub title: String,
    /// Its lines joined by LF, the indentation taken off.
    pub description: String,
    /// The values of each label it carries.
    pub labels: Vec<LabelValues>,
    /// The numbers its description names.
    pub references: Vec<Reference>,
    /// Whether the registration keeps it collapsed.
    pub collapsed: bool,
}

/// The values a node carries of one label, in the order of its line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LabelValues {
    /// The label's attribute in the specification.
    pub label: String,
    pub values: Vec<String>,
}

/// A number a text names, where it stands in that text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reference {
    /// Its offset, in UTF-16 code units, as the standalone host counts it.
    pub start: usize,
    /// The length of the bracketed number, brackets included.
    pub length: usize,
    /// The node it names, or empty when it names none.
    pub target_id: String,
}

/// The members of one label's list: the stored ones in their order, then those only a node carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionList {
    /// The label's attribute in the specification.
    pub label: String,
    pub options: Vec<SurfaceOption>,
}

/// A member of a label's list.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceOption {
    /// What a gesture names it by, the specification's id of the member.
    pub id: String,
    pub name: String,
    /// One of the ten colour names: default, gray, brown, orange, yellow, green, blue, purple, pink, red.
    pub colour: String,
    /// Whether a node carries it and the list does not hold it.
    pub implicit: bool,
}

/// A stored filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceFilter {
    /// What a gesture names it by.
    pub id: String,
    pub name: String,
    /// Its words, in DESL section 7.7.3's notation.
    pub query: String,
}

/// The width the user set of one column of levels (DESL section 7.7.1).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColumnWidth {
    /// What a gesture names it by.
    pub id: String,
    /// The depth of the levels the column holds, 1 for the top ones.
    pub level: u32,
    /// Its width in pixels.
    pub width: u32,
}

/// A note about one node (DESL section 7.7.1, `notes`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Note {
    /// What a gesture names it by: its place among the notes, valid for this model only.
    pub id: String,
    /// Its kind, as the specification names it.
    pub kind: String,
    /// The number it is about, as written.
    pub about: String,
    /// The node it is about, or empty when it names none.
    pub about_id: String,
    pub text: String,
    /// The lines under it, empty when there are none.
    pub detail: String,
    pub resolved: bool,
    /// The numbers its text names, then those its detail names, offset after the text and one LF.
    pub references: Vec<Reference>,
}

/// What reading reported at one place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceFinding {
    /// Its code: `std.duplicateId`, `std.mentions`, `fbl.unknown-block` and so on.
    pub code: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    pub message: String,
    /// The node it stands at, or empty when it stands at none.
    pub node_id: String,
    /// The 1-based line of the body, or of the registration for a finding of the registration.
    pub line: usize,
}

/// One gesture of the person: an operation of the designer's specification, what it is for, and its params. The
/// host passes it through and knows none of the operations; the module maps it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gesture {
    /// The view's id of the gesture; its outcome names it.
    pub edit_id: String,
    /// The operation's name in the specification, `toggleCollapsed` say.
    pub operation: String,
    /// The id of what it is for, from the model it was made in; empty for one that is for the document.
    pub target_id: String,
    /// Its params by their names in the specification, in their written form.
    pub params: BTreeMap<String, String>,
}

/// An action a selected node offers besides the page's own gestures, with its key: the host's ribbon shows it with that
/// key, and the key or the ribbon sends its operation for the node, as a gesture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    /// The key it is run by with Ctrl or Cmd held, by the standalone host's names (`Enter`), and whether Shift is too.
    pub key: String,
    pub shift: bool,
    /// Empty when it can run; otherwise the sentence that says why not.
    pub unavailable: String,
    /// The operation it sends for the node.
    pub operation: String,
}

/// What became of an accepted gesture.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditOutcome {
    pub edit_id: String,
    pub written: bool,
    /// Why not, when it was not.
    pub error: String,
}

/// What a session queues: the whole model after a change, or what became of a gesture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceChange {
    Model(Box<SurfaceModel>),
    Outcome(EditOutcome),
}

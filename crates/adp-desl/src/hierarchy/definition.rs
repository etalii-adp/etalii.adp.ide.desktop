//! What a designer declares for the hierarchy surface to draw it (DESL section 7.7, spec 029 R9), as a table designer
//! declares its table: the labels its nodes carry, the icons of its notes, and which operation of its specification each
//! gesture sends. A port of the standalone host's `hierarchySurfaceDefinition.ts` at a7fdf03, as far as the page drawn
//! so far reads it (etalii.adp spec 036). The surface names none of these itself.

/// How a label's values look on the page, so labels side by side tell apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelLook {
    /// A rounded tag written with `#`.
    Tag,
    /// A square badge.
    Badge,
    /// A rounded status with a dot.
    Status,
}

/// One label: an attribute its nodes carry, drawn as tags below the title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelDefinition {
    /// The attribute's name in the specification.
    pub label: String,
    /// The word that introduces it in a query: `#`, or a name and a colon.
    pub prefix: String,
    /// Whether a node holds many values of it, or one.
    pub many: bool,
    /// How its values look; none, a label of many values is a tag and one of one value a badge.
    pub look: Option<LabelLook>,
    /// The types that carry it.
    pub on: Vec<String>,
    /// How its values and its list are changed on the page, for a label of many values; none for one only shown.
    pub edit: Option<LabelEdit>,
    /// How its value and its list are changed on the page, for a label of one value; none for one only shown.
    pub pick: Option<LabelPick>,
}

impl LabelDefinition {
    /// What the page calls a value of it: the picked value's own name, or the label's.
    pub fn value_word(&self) -> &str {
        self.pick
            .as_ref()
            .map_or(self.label.as_str(), |p| p.words.value.as_str())
    }

    /// The look its values are drawn in: its own, or a tag for a label of many values and a badge for one of one.
    pub fn look(&self) -> LabelLook {
        self.look
            .unwrap_or(if self.many { LabelLook::Tag } else { LabelLook::Badge })
    }
}

/// The target an operation on the document itself names, as the definition's `for: "document"` has it.
pub const DOCUMENT_TARGET: &str = "document";

/// The names of the params of a list's gestures: a member's name, its colour, and the member a moved one goes after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListParams {
    pub name: String,
    pub colour: String,
    pub after: String,
}

/// When deleting a member of a label's list asks first, and in which words: the specification's `confirm` of its
/// delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDeletion {
    /// It asks when this many nodes carry the member, or more.
    pub threshold: usize,
    /// What it asks, with `{name}` for the member's name and `{count}` for the nodes that carry it.
    pub message: String,
}

impl ListDeletion {
    /// The question for the member `name`, which `count` nodes carry.
    pub fn question(&self, name: &str, count: usize) -> String {
        self.message
            .replace("{name}", name)
            .replace("{count}", &count.to_string())
    }
}

/// How a label of many values is changed: a field below the title that searches the label's list, picks and creates,
/// and the list's editor, where a member is renamed, recoloured, put in its place and deleted. Each gesture sends one
/// operation of the specification; an add or a remove names the node, the others the member of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelEdit {
    pub operations: LabelEditOperations,
    pub params: ListParams,
    pub words: LabelEditWords,
    pub deletion: ListDeletion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelEditOperations {
    /// A value the node carries from now on, picked or created by its name; its param is the name.
    pub add: String,
    /// A value the node no longer carries; its param is the name.
    pub remove: String,
    /// A member renamed, and every node that carries it with it; its param is the name.
    pub rename: String,
    /// A member's colour; its param is the colour.
    pub recolour: String,
    /// A member put after another, or first without one; its param is the member it goes after.
    pub reorder: String,
    /// A member deleted, and taken from every node that carries it, asked first as `deletion` says.
    pub delete: String,
}

/// What the field and the list say: the list's name, and the words of an add and a remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelEditWords {
    pub list: String,
    pub add: String,
    pub remove: String,
}

/// How a label of one value is changed: its value, or the way to give one, opens the label's list, where a member is
/// picked or the value cleared, and from where the list is edited, a new member added there too. A pick and a clear
/// name the node; an add names the document; the others the member of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelPick {
    pub operations: LabelPickOperations,
    pub params: PickParams,
    pub words: PickWords,
    pub deletion: ListDeletion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelPickOperations {
    /// The node's value from now on, a member picked by its name; its param is `params.value`.
    pub set: String,
    /// The node's value taken off.
    pub clear: String,
    /// A new member of the list; its params are the name and the colour.
    pub add: String,
    pub rename: String,
    pub recolour: String,
    pub reorder: String,
    /// A member deleted, and cleared from every node that carries it, asked first as `deletion` says.
    pub delete: String,
}

/// The names of the params: the picked value, and a member's name, colour and the member a moved one goes after.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickParams {
    pub value: String,
    pub list: ListParams,
}

/// What the control and the list say: the list's name, the value's own name, and the words of a clear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickWords {
    pub list: String,
    pub value: String,
    pub clear: String,
}

/// One key `sort:` orders the entries by that is not a label, and what the menu calls it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortKey {
    pub key: String,
    pub label: String,
}

/// What the page's filters are stored, renamed and removed by (FR-048): a store names the document, the others the
/// stored filter; and the keys a sort offers besides the labels of one value, in the query's notation (DESL section
/// 7.7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filters {
    pub operations: FilterOperations,
    pub params: FilterParams,
    /// The keys of `sort:` that are not labels: `number`, `title`.
    pub sort_keys: Vec<SortKey>,
    /// What the bar's count calls the entries, in the plural.
    pub entries: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterOperations {
    pub store: String,
    pub rename: String,
    pub remove: String,
}

/// The names of the params: a filter's name and its query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterParams {
    pub name: String,
    pub query: String,
}

/// A control that adds a node: the operation it sends, with no params, what the control says and the icon it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddDefinition {
    pub operation: String,
    pub label: String,
    /// The control is this icon alone, its label the tooltip.
    pub icon: String,
}

/// Which operation of the specification each gesture sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operations {
    /// The control to the left of a title.
    pub toggle_collapsed: String,
    /// Committing a title typed in place; its param is `title`.
    pub set_title: String,
    /// Committing a description typed in place; its param is `description`.
    pub set_description: String,
    /// The control at the end of the page: a level at the top.
    pub add_top_level: AddDefinition,
    /// The controls at the end of a level's content: an entry, and a level inside it.
    pub add_entry: AddDefinition,
    pub add_level: AddDefinition,
    /// Dropping a dragged level, with everything in it, or a dragged entry.
    pub move_level: String,
    pub move_entry: String,
    /// Delete on a selected node, with everything in it.
    pub delete: String,
}

/// The params of a move: the level it goes into (empty for the top), and the sibling it goes after (empty for none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveParams {
    pub into: String,
    pub after: String,
}

/// How the widths of the columns of levels are kept (DESL section 7.7.1): the operations, their params, and the range a
/// width is held to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Widths {
    /// The operation that gives a column its first width, the one that changes a kept width and the one that drops it.
    pub add: String,
    pub set: String,
    pub reset: String,
    /// The names of the params: the column's depth and its width.
    pub level_param: String,
    pub width_param: String,
    pub min: u32,
    pub max: u32,
}

/// What the notes of a node are made, changed and taken away by: the operation each gesture of the list sends, the
/// params they carry, and the words of the list. An add names the node, the others the note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotesDefinition {
    pub operations: NoteOperations,
    pub params: NoteParams,
    pub words: NoteWords,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteOperations {
    /// A new open note about the node; its params are the kind and the text.
    pub add: String,
    /// A note's kind, text and detail written anew.
    pub change: String,
    /// A note resolved, or reopened when it is resolved.
    pub resolve: String,
    pub remove: String,
}

/// The names of the params: the kind, as the model names it, the one-line text and the lines below it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteParams {
    pub kind: String,
    pub text: String,
    pub detail: String,
}

/// What the list and its controls say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteWords {
    pub list: String,
    pub add: String,
    pub change: String,
    pub resolve: String,
    pub reopen: String,
    pub remove: String,
}

/// When a delete asks first, and in which words: the specification's `confirm` of its delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deletion {
    /// It asks when this many nodes, notes or texts concern the node, or more.
    pub threshold: usize,
    /// What it asks, with `{number}` for the node's number and `{count}` for that count.
    pub message: String,
}

impl Deletion {
    /// The question for the node numbered `number`, which `count` things concern.
    pub fn question(&self, number: &str, count: usize) -> String {
        self.message
            .replace("{number}", number)
            .replace("{count}", &count.to_string())
    }
}

/// What a designer declares for the hierarchy surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceDefinition {
    /// The type the surface draws as boxes holding further nodes.
    pub level_type: String,
    /// The type the surface draws inside the box of its level.
    pub entry_type: String,
    pub labels: Vec<LabelDefinition>,
    /// The icon of each note kind, by the kind as the model names it.
    pub note_icons: Vec<(String, String)>,
    /// The icon that stands for several open notes.
    pub several_notes_icon: String,
    /// The notes' list, opened from the icon at a box's top right (FR-015, FR-016).
    pub notes: NotesDefinition,
    pub operations: Operations,
    pub move_params: MoveParams,
    pub deletion: Deletion,
    /// The filter bar's stored filters and sort keys; none, the page offers no filter bar.
    pub filters: Option<Filters>,
    /// How the columns' widths are kept; none when the page's own widths are not changed.
    pub widths: Option<Widths>,
    /// The keys a selected node sends to the host as a shortcut, with Ctrl or Cmd held.
    pub shortcut_keys: Vec<String>,
}

/// The icon of a level whose deeper levels do not fit the page (spec 029 R10).
pub const HIDDEN_LEVELS_ICON: &str = "mdi-chevron-double-right";
/// The icon at a box's top right while no open note is about it: the way to add the first.
pub const ADD_NOTE_ICON: &str = "mdi-comment-plus-outline";
/// The icon of the control that makes a level the left edge of the page.
pub const FOCUS_ICON: &str = "mdi-target";
/// The icon of the control at a selected box's bottom right that deletes its node.
pub const DELETE_ICON: &str = "mdi-delete-outline";
/// The icon of the grip a node is dragged by.
pub const GRIP_ICON: &str = "mdi-drag-vertical";
/// The icon of the control on a selected box with no description that opens one for typing.
pub const ADD_DESCRIPTION_ICON: &str = "mdi-text-box-plus-outline";

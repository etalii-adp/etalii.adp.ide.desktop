//! Selection, actions, prompts and property rows (etalii.adp spec 030, FR-015, FR-017, task T016).
//!
//! The data the standalone host's context system carries at `8b6ccfc` (`src/api/context.proto` and
//! `context-contract.proto`), as plain Rust: the shell renders it and knows nothing more, exactly as the standalone
//! client renders what the backend sends. One description of an action feeds the ribbon, the context menu and the
//! keyboard.

/// Where something selected or acted on lives: an explorer entry, an element of a document, the project, or the
/// errors and warnings panel (the standalone host's `ContextSource`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reference {
    /// An explorer entry, by its project-relative path.
    Entry(String),
    /// An element of an open document, by the document's id and the element's id.
    Element {
        document: String,
        element: String,
    },
    Project,
    Problems,
}

/// Which part of the window a selection was made in (the standalone host's `ContextSelectionSource`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionSource {
    Explorer,
    Canvas,
    Search,
    Problems,
    Toolbox,
    Ribbon,
    PropertyGrid,
}

/// How a new selection combines with the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectMode {
    /// The new references replace the selection.
    Replace,
    /// Ctrl+click: a reference already selected is removed, any other is added.
    Toggle,
    /// Box selection with a modifier: the references are added.
    Add,
}

/// One selection per window: ordered, with the most recently added reference as its primary one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    references: Vec<Reference>,
    source: Option<SelectionSource>,
}

impl Selection {
    pub fn references(&self) -> &[Reference] {
        &self.references
    }

    /// The reference the property grid and single-element actions act on.
    pub fn primary(&self) -> Option<&Reference> {
        self.references.last()
    }

    pub fn source(&self) -> Option<SelectionSource> {
        self.source
    }

    pub fn is_empty(&self) -> bool {
        self.references.is_empty()
    }

    pub fn contains(&self, reference: &Reference) -> bool {
        self.references.contains(reference)
    }

    /// Changes the selection; returns whether anything changed, so an unchanged selection raises no event.
    pub fn select(&mut self, references: Vec<Reference>, mode: SelectMode, source: SelectionSource) -> bool {
        let before = self.clone();
        match mode {
            SelectMode::Replace => {
                self.references.clear();
                for reference in references {
                    if !self.references.contains(&reference) {
                        self.references.push(reference);
                    }
                }
            }
            SelectMode::Toggle => {
                for reference in references {
                    if let Some(at) = self.references.iter().position(|r| *r == reference) {
                        self.references.remove(at);
                    } else {
                        self.references.push(reference);
                    }
                }
            }
            SelectMode::Add => {
                for reference in references {
                    if !self.references.contains(&reference) {
                        self.references.push(reference);
                    }
                }
            }
        }
        self.source = Some(source);
        *self != before
    }

    pub fn clear(&mut self) -> bool {
        let changed = !self.references.is_empty();
        self.references.clear();
        changed
    }
}

/// A keyboard shortcut. `primary` is Ctrl on Windows and Linux and Cmd on macOS, where the standalone host says Ctrl.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// The key, by the standalone host's names ("F2", "Delete", "ArrowUp", "z").
    pub key: String,
    pub primary: bool,
    pub shift: bool,
    pub alt: bool,
}

impl Shortcut {
    pub fn key(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            primary: false,
            shift: false,
            alt: false,
        }
    }

    pub fn primary(key: impl Into<String>) -> Self {
        Self {
            primary: true,
            ..Self::key(key)
        }
    }

    pub fn with_shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn with_alt(mut self) -> Self {
        self.alt = true;
        self
    }
}

/// One action as data (the standalone host's `ContextAction`): what it is called, its icon, its shortcut, whether it
/// can run now and, when not, why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// As in the standalone host: `hierarchy.add`, `hierarchy.rename`, `problems.validate`, ….
    pub id: String,
    pub label: String,
    /// A Material Design Icons name, as the standalone host gives it ("mdi-pencil-outline"); empty for none.
    pub icon: String,
    pub shortcut: Option<Shortcut>,
    /// Empty when the action can run; otherwise the sentence shown on the disabled item.
    pub unavailable_reason: String,
    /// Non-empty: a submenu instead of an action.
    pub items: Vec<ActionGroup>,
}

impl Action {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: String::new(),
            shortcut: None,
            unavailable_reason: String::new(),
            items: Vec::new(),
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self
    }

    pub fn with_shortcut(mut self, shortcut: Shortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    pub fn unavailable(mut self, reason: impl Into<String>) -> Self {
        self.unavailable_reason = reason.into();
        self
    }

    pub fn is_available(&self) -> bool {
        self.unavailable_reason.is_empty()
    }
}

/// Actions shown together, between separators.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionGroup {
    pub actions: Vec<Action>,
}

/// Finds the available action a shortcut runs, depth first through submenus; an unavailable action's shortcut is inert.
pub fn action_for_shortcut<'a>(groups: &'a [ActionGroup], pressed: &Shortcut) -> Option<&'a Action> {
    for group in groups {
        for action in &group.actions {
            if action.shortcut.as_ref() == Some(pressed) && action.is_available() && action.items.is_empty() {
                return Some(action);
            }
            if let Some(found) = action_for_shortcut(&action.items, pressed) {
                return Some(found);
            }
        }
    }
    None
}

/// The editor a property row is changed with (the standalone host's `ContextPropertyEditor`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyEditor {
    /// A single line: a name, a technology.
    Line,
    /// Several lines: a description, a note.
    Text,
    /// Yes or no; the value is "true" or "false".
    Toggle,
    /// One of the row's candidates.
    Choice,
    /// One of the row's candidates, in their order, on a slider.
    Slider,
    /// A set of tags.
    Tags,
}

/// One row of the property grid (the standalone host's `ContextProperty`). The value is always text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRow {
    /// Stable within its provider, prefixed like an action id.
    pub id: String,
    pub label: String,
    pub value: String,
    pub editor: PropertyEditor,
    /// Empty when the row can be edited; otherwise why it cannot be, shown beside the value.
    pub read_only_reason: String,
    /// An optional heading; empty for none.
    pub group: String,
    /// What a choice or a slider may be set to, in the order shown; empty for the other editors.
    pub candidates: Vec<String>,
}

impl PropertyRow {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        editor: PropertyEditor,
        value: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: value.into(),
            editor,
            read_only_reason: String::new(),
            group: String::new(),
            candidates: Vec::new(),
        }
    }

    pub fn is_read_only(&self) -> bool {
        !self.read_only_reason.is_empty()
    }

    /// Checks a new value against the row before a provider is asked to apply it: read-only rows refuse with their
    /// reason, toggles take only "true" or "false", choices and sliders only a candidate.
    pub fn validate(&self, value: &str) -> Result<(), String> {
        if self.is_read_only() {
            return Err(self.read_only_reason.clone());
        }
        match self.editor {
            PropertyEditor::Toggle if value != "true" && value != "false" => {
                Err(format!("{} is yes or no.", self.label))
            }
            PropertyEditor::Choice | PropertyEditor::Slider if !self.candidates.iter().any(|c| c == value) => {
                Err(format!("{} cannot be {value}.", self.label))
            }
            PropertyEditor::Line if value.contains('\n') => Err(format!("{} is a single line.", self.label)),
            _ => Ok(()),
        }
    }
}

/// One option of a choice or file dialog (the standalone host's `ContextOption`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptOption {
    /// For a selectable option, exactly the value an answer carries; for a group, informational.
    pub id: String,
    pub label: String,
    /// False for a group heading, which is never an answer.
    pub selectable: bool,
    pub children: Vec<PromptOption>,
    /// What the dialog's name field takes when this option is picked and nothing was typed.
    pub suggested_value: String,
    pub icon: String,
    pub description: String,
    /// Non-empty: this option wants no name, and the sentence replaces the name field.
    pub name_suppressed_reason: String,
    /// Non-empty: shown but not choosable, with this reason.
    pub unavailable_reason: String,
}

/// A text field beside a dialog's main answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    pub label: String,
    pub initial_value: String,
}

/// What the core asks the person (the standalone host's `ContextPrompt`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prompt {
    Input {
        title: String,
        icon: String,
        label: String,
        initial_value: String,
        confirm_label: String,
    },
    Confirm {
        title: String,
        icon: String,
        message: String,
        confirm_label: String,
        danger: bool,
    },
    Choice {
        title: String,
        icon: String,
        confirm_label: String,
        options: Vec<PromptOption>,
        empty_message: String,
        name_field: Option<TextField>,
    },
    File {
        title: String,
        icon: String,
        confirm_label: String,
        files: Vec<PromptOption>,
        pinned: Vec<PromptOption>,
        empty_message: String,
        name_field: Option<TextField>,
    },
    /// Closed by the core, with what to tell the person.
    Closed { message: String },
}

/// Finds a selectable, choosable option by the value an answer carries, depth first.
pub fn find_answer<'a>(options: &'a [PromptOption], value: &str) -> Option<&'a PromptOption> {
    for option in options {
        if option.selectable && option.unavailable_reason.is_empty() && option.id == value {
            return Some(option);
        }
        if let Some(found) = find_answer(&option.children, value) {
            return Some(found);
        }
    }
    None
}

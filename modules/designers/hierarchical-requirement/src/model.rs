//! A requirement document as the designer holds it (the standalone host's `_Model` at a7fdf03): the marks of its
//! counters, its entries and assessments in reading order, the members of its lists, its stored filters and the
//! widths of its columns, and what reading it reported.

/// A requirement document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Document {
    /// The mark of the item counter on line 2, `ITEM-0007`, or none when the file has none.
    pub last_item: Option<String>,
    /// The mark of the requirement counter, or none when the file has none.
    pub last_requirement: Option<String>,
    /// The items and requirements in reading order.
    pub entries: Vec<Entry>,
    /// In the order of the Assessments section.
    pub assessments: Vec<Assessment>,
    pub tags: Vec<Member>,
    pub categories: Vec<Member>,
    pub states: Vec<Member>,
    pub filters: Vec<Filter>,
    pub columns: Vec<Column>,
    pub findings: Vec<Finding>,
    /// Why the document cannot be changed here, or empty when it can.
    pub read_only_reason: String,
}

/// What an entry is: an item, which holds requirements and further items, or a requirement, which holds nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryKind {
    Item,
    Requirement {
        /// The category's name as written, or none when it has none.
        category: Option<String>,
        /// The state's name as written, or none when it has none.
        state: Option<String>,
    },
}

/// An entry of the hierarchy: an item or a requirement, a bullet under the heading Requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Its number, `ITEM-0004` or `REQ-0031`. A placeholder, `ITEM-NEW` or `REQ-NEW`, has the number it is given in
    /// memory, written with the next edit (DESL 0.2 section 5.3.5); a duplicate keeps the number written.
    pub id: String,
    /// The item it is nested in, by its number, or none at the top level.
    pub parent_id: Option<String>,
    /// The text between the number and the trailing words; empty when nothing stands there.
    pub title: String,
    /// The lines indented under the bullet, empty when there are none.
    pub description: String,
    /// The `#name` words, in their order.
    pub tags: Vec<String>,
    /// Whether the registration keeps it collapsed.
    pub collapsed: bool,
    /// The line of its bullet, from 1.
    pub line: usize,
    /// Whether the file holds a placeholder where its number goes.
    pub placeholder: bool,
    pub kind: EntryKind,
}

impl Entry {
    pub fn is_item(&self) -> bool {
        self.kind == EntryKind::Item
    }

    /// The specification's type of the entry.
    pub fn type_name(&self) -> &'static str {
        if self.is_item() { "Item" } else { "Requirement" }
    }

    pub fn category(&self) -> Option<&str> {
        match &self.kind {
            EntryKind::Requirement { category, .. } => category.as_deref(),
            EntryKind::Item => None,
        }
    }

    pub fn state(&self) -> Option<&str> {
        match &self.kind {
            EntryKind::Requirement { state, .. } => state.as_deref(),
            EntryKind::Item => None,
        }
    }
}

/// What an assessment is, written in the file as Question, Remark, Warning or Idea in any case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssessmentKind {
    Question,
    Remark,
    Warning,
    Idea,
}

impl AssessmentKind {
    pub const ALL: [AssessmentKind; 4] = [
        AssessmentKind::Question,
        AssessmentKind::Remark,
        AssessmentKind::Warning,
        AssessmentKind::Idea,
    ];

    /// The kind of this name, ignoring case.
    pub fn parse(name: &str) -> Option<AssessmentKind> {
        AssessmentKind::ALL
            .into_iter()
            .find(|kind| kind.name().eq_ignore_ascii_case(name))
    }

    /// Its name as the specification's enumeration writes it, capitalised.
    pub fn name(self) -> &'static str {
        match self {
            AssessmentKind::Question => "Question",
            AssessmentKind::Remark => "Remark",
            AssessmentKind::Warning => "Warning",
            AssessmentKind::Idea => "Idea",
        }
    }
}

/// A question, remark, warning or idea about one entry, a bullet under the heading Assessments. It has no number:
/// its id is its place among the document's assessments, `assessment/0` onwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assessment {
    pub id: String,
    pub kind: AssessmentKind,
    /// The number of the entry it is about, as written.
    pub about: String,
    pub text: String,
    /// The lines indented under it, empty when there are none.
    pub detail: String,
    /// Whether its checkbox is ticked; one without a checkbox is open.
    pub resolved: bool,
    pub line: usize,
}

/// A member of the tag, category or state list: its name and its colour, `default` when the list gives none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub colour: String,
}

impl Member {
    pub fn new(name: impl Into<String>, colour: impl Into<String>) -> Self {
        Member {
            name: name.into(),
            colour: colour.into(),
        }
    }
}

/// A stored filter: its name and its query (DESL 0.2 section 7.7.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub name: String,
    pub query: String,
}

/// The width the user set of one column of items: its depth, 1 for the top items, and its width in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub level: u32,
    pub width: u32,
}

/// The narrowest and the widest a column can be set to (`setColumnWidth`'s `width`, 120 to 2000).
pub const MINIMUM_WIDTH: u32 = 120;
pub const MAXIMUM_WIDTH: u32 = 2000;

/// What reading a requirement document reports at one line (the companion's "What reading reports").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The finding's code: `std.missingId`, `std.containment`, `fbl.unknown-block` and so on.
    pub code: String,
    pub message: String,
    /// The line it stands at, from 1, in the file that holds it.
    pub line: usize,
    /// Whether it is a finding of the registration rather than of the Markdown file.
    pub in_registration: bool,
}

impl Finding {
    pub fn new(code: impl Into<String>, message: impl Into<String>, line: usize) -> Self {
        Finding {
            code: code.into(),
            message: message.into(),
            line,
            in_registration: false,
        }
    }
}

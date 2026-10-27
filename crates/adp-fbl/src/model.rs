//! What reading a body gives (FBL sections 5 and 7.4; the standalone host's `FblModel` and `Finding`): its elements
//! and relations in document order, each with its attributes and where it is, and the findings the reading made.
//! Reading never fails on content: whatever it cannot read becomes a finding. The caller's settings for one body,
//! [`FblOptions`], carry what FBL leaves to the host: the DISL id strategy and the attribute types.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

use adp_cel::{Map, Value};

use crate::text::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
}

/// DISL's source location: the file relative to the subject, and the line, column and length of the entry's span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// A problem found while reading a body or a registration (FBL section 7.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub code: String,
    pub severity: FindingSeverity,
    pub message: String,
    pub location: Option<SourceLocation>,
    /// What the finding is about, by name, where its code defines it: `attribute` and `text` for
    /// [`codes::UNCONVERTED_VALUE`].
    pub detail: Option<Vec<(String, String)>>,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(location) => write!(f, "{} at {location}: {}", self.code, self.message),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}

/// The finding codes FBL defines (FBL section 7.4) and the DISL ones it uses (DISL section 8.6).
pub mod codes {
    pub const UNBOUND_STATEMENT: &str = "fbl.unbound-statement";
    pub const UNBOUND_KEY: &str = "fbl.unbound-key";
    pub const DANGLING_REFERENCE: &str = "fbl.dangling-reference";
    pub const HEADER_MISMATCH: &str = "fbl.header-mismatch";
    pub const DUPLICATE_KEY: &str = "fbl.duplicate-key";
    pub const MISSING_BODY: &str = "fbl.missing-body";
    pub const STALE_VIEW_DATA: &str = "fbl.stale-view-data";
    pub const UNKNOWN_HEADER: &str = "fbl.unknown-header";
    pub const UNKNOWN_BLOCK: &str = "fbl.unknown-block";
    pub const REGEX_TIMEOUT: &str = "fbl.regex-timeout";
    pub const UNCONVERTED_VALUE: &str = "fbl.unconverted-value";
    pub const UNPARSEABLE: &str = "std.unparseable";
    pub const UNREADABLE_ENTRY: &str = "std.unreadableEntry";
    /// *(FBL 0.9)* Text of a `grammar` body read in part that does not parse (FBL section 4.9).
    pub const UNPARSED_TEXT: &str = "fbl.unparsed-text";
    pub const MISSING_ID: &str = "std.missingId";
    pub const DUPLICATE_ID: &str = "std.duplicateId";
    pub const PLUGIN_MISSING: &str = "std.pluginMissing";
}

/// An element or a relation read from a body (FBL section 5). `id_is_stored` is false when the id is not kept in
/// the body: derived by the caller's DISL id strategy, or a place-based address.
#[derive(Debug, Clone)]
pub struct FblElement {
    pub id: String,
    pub id_is_stored: bool,
    pub type_name: String,
    pub rule: String,
    pub is_relation: bool,
    pub attributes: Map,
    pub parent_id: Option<String>,
    pub parent_slot: Option<String>,
    pub source: Option<String>,
    pub target: Option<String>,
    pub own_span: Span,
    pub line: usize,
    /// The file the element is in, relative to a folder subject and `/`-separated, as the persistence plugin that
    /// read it says (FBL section 11.2); none for an element of a file body, whose file is the body's.
    pub file: Option<String>,
}

/// A view a block of the body defines (FBL section 4.7): its name, the block rule that matched it, and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FblView {
    pub name: String,
    pub block: String,
    pub span: Span,
    pub line: usize,
}

/// What reading a body gives: its elements and relations in document order, and its findings.
#[derive(Debug, Clone, Default)]
pub struct FblModel {
    pub elements: Vec<FblElement>,
    pub findings: Vec<Finding>,
    pub unreadable: bool,
    /// The views a blocks body defines (FBL section 4.7), in document order.
    pub views: Vec<FblView>,
    /// The resources a body holds (FBL section 8.2): the values of the binding's resource capture.
    pub resources: Vec<String>,
}

impl FblModel {
    pub fn find(&self, id: &str) -> Option<&FblElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// The view a registration's `view` header selects (FBL section 9.3): the one of that name, ignoring case;
    /// without the header, the first in document order.
    pub fn select_view(&self, header: Option<&str>) -> Option<&FblView> {
        match header {
            None => self.views.first(),
            Some(h) => self.views.iter().find(|v| v.name.eq_ignore_ascii_case(h)),
        }
    }
}

/// What a DISL id strategy is asked when a rule stores no id (FBL section 5.3, DISL section 11.5).
#[derive(Debug, Clone)]
pub struct IdRequest<'a> {
    pub rule: &'a str,
    pub type_name: &'a str,
    pub attributes: &'a Map,
    pub source: Option<&'a str>,
    pub target: Option<&'a str>,
    pub line: usize,
    /// The id of the element that contains this one (FBL section 5.5), or none at the top level. Known before this
    /// element's id is asked for: an element is read after the one that contains it.
    pub parent_id: Option<&'a str>,
    /// The containment slot of the parent this element is in, or none at the top level.
    pub parent_slot: Option<&'a str>,
    /// The element's zero-based place among the elements of the same parent and slot, in document order.
    pub position_in_slot: usize,
}

/// The DISL id strategy: the derived id, or none to address the element by its place.
pub type DeriveId = Rc<dyn Fn(&IdRequest<'_>) -> Option<String>>;

/// The type a tool type's specification gives an attribute of an element type, by the two names.
pub type AttributeType = Rc<dyn Fn(&str, &str) -> Option<String>>;

/// Whether a tool type's specification gives an attribute of an element type `many`, by the two names.
pub type Many = Rc<dyn Fn(&str, &str) -> bool>;

/// The caller's settings for reading and writing one body.
#[derive(Clone)]
pub struct FblOptions {
    /// The body's file name, relative to the subject, for findings.
    pub file_name: String,
    /// A body larger than this is unreadable rather than read in part (FBL section 16).
    pub max_body_bytes: usize,
    /// A body with more entries than this is unreadable rather than read in part (FBL section 16).
    pub max_entries: usize,
    /// The DISL id strategy for rules that store no id. FBL defines no derivation of its own (FBL section 5.3).
    pub derive_id: Option<DeriveId>,
    /// The type the tool type's specification gives an attribute; with it, an entry holding a value that does not
    /// convert is an unreadable entry (FBL section 7.4).
    pub attribute_type: Option<AttributeType>,
    /// Whether the tool type's specification gives an attribute `many`. A word slot of such an attribute holds every
    /// word of its group that matches, as a list (FBL 0.7 section 4.6); without this, it holds the first. A binding
    /// does not carry it.
    pub many: Option<Many>,
    /// The binding types the tool type's specification maps as unreadable (DESL section 5.2): an entry a rule of such
    /// a type takes is an unreadable entry, reported and kept, and nothing nested in it is looked into (FBL 0.7
    /// sections 4.8, 7.4).
    pub unreadable_types: HashSet<String>,
    /// The attributes whose type is a date or a date-time, each as `rule.attribute`, which FBL section 6.3 needs
    /// to write a value that reads as a timestamp plain or quoted.
    pub time_attributes: HashSet<String>,
    /// Whether a key of a read entry that no rule reads is reported as [`codes::UNBOUND_KEY`].
    pub report_unbound_keys: bool,
    /// The registration's headers, visible to CEL as `registration`.
    pub registration_headers: Vec<(String, String)>,
    /// The registration's `resource` header (FBL section 8.2).
    pub resource: Option<String>,
    /// The registration's `identities` block, for rules with `id.sidecar`.
    pub identities: HashMap<String, String>,
    /// The entries a host adds beside a folder subject's selected files for its persistence plugin, each a path
    /// relative to the folder, given the folder's full path (the standalone host's `SupplementaryEntries`; outside
    /// FBL 0.2). None adds nothing.
    pub supplementary_entries: Option<SupplementaryEntries>,
}

/// What [`FblOptions::supplementary_entries`] gives for a folder: paths relative to it, with no bytes.
pub type SupplementaryEntries = Rc<dyn Fn(&std::path::Path) -> Vec<String>>;

impl Default for FblOptions {
    fn default() -> Self {
        FblOptions {
            file_name: "body".into(),
            max_body_bytes: 32 * 1024 * 1024,
            max_entries: 500_000,
            derive_id: None,
            attribute_type: None,
            many: None,
            unreadable_types: HashSet::new(),
            time_attributes: HashSet::new(),
            report_unbound_keys: false,
            registration_headers: Vec::new(),
            resource: None,
            identities: HashMap::new(),
            supplementary_entries: None,
        }
    }
}

impl fmt::Debug for FblOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FblOptions")
            .field("file_name", &self.file_name)
            .finish_non_exhaustive()
    }
}

impl FblOptions {
    pub fn attribute_type(&self, element: &str, attribute: &str) -> Option<String> {
        self.attribute_type.as_ref().and_then(|f| f(element, attribute))
    }

    pub fn many(&self, element: &str, attribute: &str) -> bool {
        self.many.as_ref().is_some_and(|f| f(element, attribute))
    }
}

/// A value of an attribute as a number, when it is one.
pub fn number_of(value: &Value) -> Option<f64> {
    value.as_number()
}

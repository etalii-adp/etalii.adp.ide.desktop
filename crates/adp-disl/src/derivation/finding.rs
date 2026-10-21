//! One finding (DISL section 8.6; the standalone host's `DislFinding`, `DislReaderFinding` and
//! `DislConstraintOptions`) as the runtime gives it to a host, which maps it onto its own type: the code, the rule
//! that raised it, the severity, the message, the elements it is about, where in the files it is and what it is
//! about when that is not drawn; the reader's findings (section 8.7) a host hands in; and what the constraint
//! evaluator is given beside the specification and the diagram.

use std::rc::Rc;

use adp_cel::{Map, Value};

use super::env::DislEnv;
use crate::model::element::DislElement;

/// One finding as a host shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislFinding {
    /// The rule's `code`, else its id.
    pub code: String,
    /// The id of the declared rule, or of the built-in (`std.endpoints`).
    pub constraint_id: String,
    /// `error`, `warning`, `info` or `hint`.
    pub severity: String,
    /// The rule's Message, evaluated.
    pub message: String,
    /// The ids of what the finding is about, as the host writes them.
    pub element_ids: Vec<String>,
    /// The 1-based line it is located at, when it has one.
    pub line: Option<i32>,
    /// The file it is located in, relative to the diagram's subject (section 8.6), when it has one.
    pub file: Option<String>,
    /// The rule's `subject` (section 8.2): a display string naming what is not drawn, never an id.
    pub subject: Option<String>,
}

/// A finding the reader raised (section 8.7): `std.unparseable` or `std.unreadableEntry`, with the `detail` its
/// message is evaluated with and its line.
#[derive(Debug, Clone)]
pub struct DislReaderFinding {
    /// The built-in it is reported as.
    pub built_in: String,
    /// Bound as `detail` in the built-in's message: at least `reason`.
    pub detail: Map,
    /// The 1-based line, when the reader knows it.
    pub line: Option<i32>,
    /// The file it is located in, relative to the subject (section 8.6); none for the primary file. A
    /// `std.unparseable` naming a file other than the diagram's own silences only the findings located in that
    /// file, where one naming no file stops every constraint.
    pub file: Option<String>,
}

pub const UNPARSEABLE: &str = "std.unparseable";
const UNREADABLE_ENTRY: &str = "std.unreadableEntry";

impl DislReaderFinding {
    fn with_reason(built_in: &str, reason: &str, line: Option<i32>, file: Option<&str>) -> Self {
        let mut detail = Map::default();
        detail.insert("reason", Value::from(reason));
        DislReaderFinding {
            built_in: built_in.to_owned(),
            detail,
            line,
            file: file.map(str::to_owned),
        }
    }

    /// An entry the reader could not make an element of, for `reason`.
    pub fn unreadable(reason: &str, line: Option<i32>, file: Option<&str>) -> Self {
        DislReaderFinding::with_reason(UNREADABLE_ENTRY, reason, line, file)
    }

    /// A file the reader could not parse, for `reason`.
    pub fn not_parsed(reason: &str, line: Option<i32>, file: Option<&str>) -> Self {
        DislReaderFinding::with_reason(UNPARSEABLE, reason, line, file)
    }
}

/// The id an element is written with, which a finding names and `self.id` reads in its message.
pub type WrittenId = Rc<dyn Fn(&DislElement) -> String>;

/// What the constraint evaluator is given beside the specification and the diagram.
#[derive(Clone, Default)]
pub struct DislConstraintOptions {
    /// What `env` reads.
    pub env: Option<DislEnv>,
    /// The reader's findings, in reading order.
    pub reader_findings: Vec<DislReaderFinding>,
    /// The id an element is written with; none for its model id. A later holder of a written id has an ephemeral id
    /// in the model (section 11.5.4), while its file still names it by what is written.
    pub written_id: Option<WrittenId>,
}

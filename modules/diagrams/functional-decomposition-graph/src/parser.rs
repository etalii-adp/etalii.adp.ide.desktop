//! Reads a `.fdg` into an [`FdgModel`], recording which lines declare what (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host's `src/diagrams/functional-decomposition-graph/backend/.../FdgParser.cs` at
//! `8b6ccfc`. The parser never fails: a text that is not YAML gives an empty model carrying the reason and its line,
//! so the document still opens and says what is wrong, and nothing is edited until it reads; an unknown key, an
//! unknown type or a malformed entry is kept with whatever could be read and reported; a coordinate that will not
//! read is zero, and reported. Nothing is ever written through the reader: every write is a splice.

use crate::lines::{LineDocument, LineRange};
use crate::model::{FdgConnection, FdgElement, FdgModel, ParseProblem, connection_types, element_types};
use crate::yaml::{self, LineIndex, Node};

const HEADER_KEY: &str = "functional-decomposition-graph";
const ELEMENTS_KEY: &str = "elements";
const CONNECTIONS_KEY: &str = "connections";

/// The keys an element entry may carry; anything else survives and is reported.
const ELEMENT_KEYS: [&str; 9] = ["id", "type", "name", "description", "text", "x", "y", "width", "height"];

/// The keys a connection entry may carry.
const CONNECTION_KEYS: [&str; 6] = ["id", "type", "from", "to", "name", "description"];

/// The text of a new, empty `.fdg`, with `line_ending`: the header and the two sections written flow-empty, since a
/// bare `elements:` reads as null rather than an empty sequence.
pub fn empty_document(line_ending: &str) -> String {
    [
        format!("{HEADER_KEY}: {}", FdgModel::CURRENT_VERSION),
        "elements: []".to_owned(),
        "connections: []".to_owned(),
    ]
    .join(line_ending)
        + line_ending
}

/// What the store keeps of a document: its model, and, for a text that is not YAML, why, so that nothing is edited
/// or written until it reads (`ParseOutcome`).
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutcome {
    pub model: FdgModel,
    pub unreadable: Option<String>,
}

/// Reads the document. Never fails.
pub fn parse(document: &LineDocument) -> FdgModel {
    outcome(document).model
}

/// Reads the document, and says why when its text is not YAML. Never fails.
pub fn outcome(document: &LineDocument) -> ParseOutcome {
    let text = document.text();
    let index = LineIndex::of(document);
    let root = match yaml::read(&text) {
        Ok(root) => root,
        Err(error) => {
            let problem = ParseProblem {
                line: index.line_of(error.at),
                message: format!("The document could not be read as YAML: {}", error.message),
            };
            let reason = problem.message.clone();
            return ParseOutcome {
                model: FdgModel {
                    problems: vec![problem],
                    ..FdgModel::default()
                },
                unreadable: Some(reason),
            };
        }
    };
    let Some(root @ Node::Mapping { .. }) = root else {
        // An empty file, or a root that is not a mapping, declares nothing; whether that is worth reporting is the
        // rule set's to say.
        return ParseOutcome {
            model: FdgModel::default(),
            unreadable: None,
        };
    };
    let mut problems = Vec::new();
    let reader = Reader {
        document,
        index: &index,
    };
    let elements = reader.elements(&root, &mut problems);
    let connections = reader.connections(&root, &mut problems);
    let version = reader.version(&root, &mut problems);
    ParseOutcome {
        model: FdgModel {
            elements,
            connections,
            problems,
            version,
        },
        unreadable: None,
    }
}

struct Reader<'a> {
    document: &'a LineDocument,
    index: &'a LineIndex,
}

impl Reader<'_> {
    fn problem(&self, node: &Node, message: String) -> ParseProblem {
        ParseProblem {
            line: self.index.start_line(node),
            message,
        }
    }

    fn range(&self, node: &Node) -> LineRange {
        self.index.range(node, self.document)
    }

    /// The header's version: a missing or unreadable header opens the document and is reported.
    fn version(&self, root: &Node, problems: &mut Vec<ParseProblem>) -> Option<i64> {
        let Some(scalar @ Node::Scalar { value, .. }) = root.get(HEADER_KEY) else {
            problems.push(ParseProblem {
                line: 0,
                message: format!(
                    "The document does not begin with `{HEADER_KEY}: {}`.",
                    FdgModel::CURRENT_VERSION
                ),
            });
            return None;
        };
        let Ok(version) = value.trim().parse::<i64>() else {
            problems.push(self.problem(scalar, format!("The version `{value}` is not a number.")));
            return None;
        };
        if version != FdgModel::CURRENT_VERSION {
            problems.push(self.problem(
                scalar,
                format!(
                    "This document states version {version}; this module reads version {}.",
                    FdgModel::CURRENT_VERSION
                ),
            ));
        }
        Some(version)
    }

    fn elements(&self, root: &Node, problems: &mut Vec<ParseProblem>) -> Vec<FdgElement> {
        let mut elements = Vec::new();
        for node in root.sequence(ELEMENTS_KEY) {
            if !matches!(node, Node::Mapping { .. }) {
                problems.push(self.problem(
                    node,
                    "An element entry is not a mapping and was passed over.".to_owned(),
                ));
                continue;
            }
            let element_type = node.scalar("type").unwrap_or_default().to_owned();
            let is_comment = element_type == element_types::COMMENT;
            self.report_unknown_keys(node, &ELEMENT_KEYS, problems, "element");
            if !element_types::is_known(&element_type) {
                problems.push(self.problem(
                    node,
                    format!("`{element_type}` is not an element type this module knows."),
                ));
            }
            if is_comment && node.scalar("name").is_some() {
                problems.push(self.problem(
                    node,
                    "A Comment has no name; the `name` here is kept and ignored.".to_owned(),
                ));
            }
            let text = |key: &str| node.scalar(key).unwrap_or_default().to_owned();
            elements.push(FdgElement {
                id: text("id"),
                name: text("name"),
                description: text("description"),
                text: text("text"),
                x: self.number(node, "x", problems),
                y: self.number(node, "y", problems),
                width: self.number(node, "width", problems),
                height: is_comment.then(|| self.number(node, "height", problems)),
                range: self.range(node),
                element_type,
            });
        }
        elements
    }

    fn connections(&self, root: &Node, problems: &mut Vec<ParseProblem>) -> Vec<FdgConnection> {
        let mut connections = Vec::new();
        for node in root.sequence(CONNECTIONS_KEY) {
            if !matches!(node, Node::Mapping { .. }) {
                problems.push(self.problem(
                    node,
                    "A connection entry is not a mapping and was passed over.".to_owned(),
                ));
                continue;
            }
            self.report_unknown_keys(node, &CONNECTION_KEYS, problems, "connection");
            let connection_type = node.scalar("type").unwrap_or_default().to_owned();
            if !connection_types::is_known(&connection_type) {
                problems.push(self.problem(
                    node,
                    format!("`{connection_type}` is not a relation this module knows."),
                ));
            }
            let text = |key: &str| node.scalar(key).unwrap_or_default().to_owned();
            connections.push(FdgConnection {
                id: text("id"),
                from: text("from"),
                to: text("to"),
                name: text("name"),
                description: text("description"),
                range: self.range(node),
                connection_type,
            });
        }
        connections
    }

    fn report_unknown_keys(&self, mapping: &Node, known: &[&str], problems: &mut Vec<ParseProblem>, what: &str) {
        let Node::Mapping { entries, .. } = mapping else {
            return;
        };
        for (key, _) in entries {
            if let Node::Scalar { value, .. } = key
                && !known.contains(&value.as_str())
            {
                problems.push(self.problem(
                    key,
                    format!("`{value}` is not a key this module reads on a {what}; the line is kept."),
                ));
            }
        }
    }

    /// A number from the entry, or zero; one that will not read is zero and a problem, never silently the origin.
    fn number(&self, mapping: &Node, key: &str, problems: &mut Vec<ParseProblem>) -> f64 {
        let Some(scalar @ Node::Scalar { value, .. }) = mapping.get(key) else {
            return 0.0;
        };
        match parse_number(value) {
            Some(number) => number,
            None => {
                problems.push(self.problem(scalar, format!("`{key}: {value}` is not a number; zero was used.")));
                0.0
            }
        }
    }
}

/// A number as .NET's `double.TryParse` with `NumberStyles.Float` and the invariant culture reads it: surrounding
/// whitespace, a sign, a decimal point and an exponent.
pub fn parse_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    match trimmed {
        "Infinity" | "+Infinity" | "∞" => return Some(f64::INFINITY),
        "-Infinity" | "-∞" => return Some(f64::NEG_INFINITY),
        "NaN" => return Some(f64::NAN),
        _ => {}
    }
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'))
    {
        return None;
    }
    trimmed.parse().ok()
}

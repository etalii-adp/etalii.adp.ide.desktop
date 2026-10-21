//! Reads a `.skv` document into a [`SankeyModel`], recording which lines declare what (etalii.adp spec 030, task
//! T077).
//!
//! Ported from the standalone host's `SankeyParser.cs` at `8b6ccfc`. The parser never fails: a document that is not
//! YAML opens empty and says why; an unknown key or a number that will not read is kept, passed over and reported. A
//! half-typed document must open and keep every byte its author wrote.

use crate::lines::{LineDocument, parse_int, parse_number};
use crate::model::{
    CURRENT_VERSION, FROM_SOURCE, FROM_TARGET, ParseProblem, SankeyFlow, SankeyModel, SankeyNode, SankeySettings,
    geometry,
};
use crate::yaml::{self, YamlNode};

pub const HEADER_KEY: &str = "sankey";
const FORMAT_KEY: &str = "format";
const FLOW_COLOR_KEY: &str = "flow-color";
pub const THICKNESS_KEY: &str = "thickness";
pub const NODES_KEY: &str = "nodes";
pub const FLOWS_KEY: &str = "flows";

/// The text of a new, empty `.skv` document: the header and the two sections written flow-empty, because a bare
/// `nodes:` reads as a null rather than an empty sequence; CRLF, because a new file has no style of its own to keep.
pub const EMPTY_DOCUMENT: &str = "sankey: 1\r\nnodes: []\r\nflows: []\r\n";

const ROOT_KEYS: [&str; 6] = [
    HEADER_KEY,
    FORMAT_KEY,
    FLOW_COLOR_KEY,
    THICKNESS_KEY,
    NODES_KEY,
    FLOWS_KEY,
];
const NODE_KEYS: [&str; 7] = ["id", "name", "color", "note", "format", "column", "description"];
const FLOW_KEYS: [&str; 7] = ["id", "from", "to", "value", "step", "color", "description"];

/// What reading a document came to: the model, and why the text could not be read as YAML at all, so that nothing is
/// edited or written until it can (`ParseOutcome`).
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub model: SankeyModel,
    pub unreadable: Option<String>,
}

/// Reads `document`. Never fails.
pub fn parse(document: &LineDocument) -> SankeyModel {
    outcome(document).model
}

/// Reads `document`, with why it is unreadable when it is not YAML.
pub fn outcome(document: &LineDocument) -> Outcome {
    let root = match yaml::parse(&document.text()) {
        Ok(root) => root,
        Err(error) => {
            let message = format!("The document could not be read as YAML: {}", error.message);
            return Outcome {
                model: SankeyModel {
                    problems: vec![ParseProblem {
                        line: error.line,
                        message: message.clone(),
                    }],
                    ..SankeyModel::default()
                },
                unreadable: Some(message),
            };
        }
    };
    let Some(root @ YamlNode::Mapping { .. }) = root else {
        return Outcome {
            model: SankeyModel::default(),
            unreadable: None,
        };
    };
    let mut problems = Vec::new();
    report_unknown_keys(&root, &ROOT_KEYS, document, &mut problems, "diagram");
    let settings = read_settings(&root, document, &mut problems);
    let nodes = read_nodes(&root, document, &mut problems);
    let flows = read_flows(&root, document, &mut problems);
    let version = read_version(&root, document, &mut problems);
    Outcome {
        model: SankeyModel {
            settings,
            nodes,
            flows,
            problems,
            version,
        },
        unreadable: None,
    }
}

fn problem(problems: &mut Vec<ParseProblem>, line: usize, message: String) {
    problems.push(ParseProblem { line, message });
}

fn read_version(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Option<i32> {
    let Some(scalar @ YamlNode::Scalar { value, .. }) = root.get(HEADER_KEY) else {
        problem(
            problems,
            0,
            format!("The document does not begin with `{HEADER_KEY}: {CURRENT_VERSION}`."),
        );
        return None;
    };
    let line = yaml::line_of(scalar, document);
    let Some(version) = parse_int(value) else {
        problem(problems, line, format!("The version `{value}` is not a number."));
        return None;
    };
    if version != CURRENT_VERSION {
        problem(
            problems,
            line,
            format!("This document states version {version}; this module reads version {CURRENT_VERSION}."),
        );
    }
    Some(version)
}

fn read_settings(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> SankeySettings {
    let mut settings = SankeySettings::default();
    if let Some(format @ YamlNode::Scalar { value, .. }) = root.get(FORMAT_KEY) {
        settings.format = value.clone();
        settings.format_line = Some(yaml::line_of(format, document));
    }
    if let Some(flow_color @ YamlNode::Scalar { value, .. }) = root.get(FLOW_COLOR_KEY) {
        let line = yaml::line_of(flow_color, document);
        if value == FROM_SOURCE || value == FROM_TARGET {
            settings.flow_color = value.clone();
        } else {
            problem(
                problems,
                line,
                format!(
                    "`{FLOW_COLOR_KEY}: {value}` is neither `{FROM_SOURCE}` nor `{FROM_TARGET}`; flows take their target's colour."
                ),
            );
        }
        settings.flow_color_line = Some(line);
    }
    if let Some(thickness @ YamlNode::Scalar { value, .. }) = root.get(THICKNESS_KEY) {
        let line = yaml::line_of(thickness, document);
        match parse_number(value) {
            Some(number) if number.is_finite() && number > 0.0 => {
                settings.thickness = number.clamp(geometry::MINIMUM_SCALE, geometry::MAXIMUM_SCALE);
            }
            _ => problem(
                problems,
                line,
                format!("`{THICKNESS_KEY}: {value}` is not a number above zero and was ignored."),
            ),
        }
        settings.thickness_line = Some(line);
    }
    settings
}

fn text(mapping: &YamlNode, key: &str) -> String {
    mapping.scalar(key).unwrap_or_default().to_owned()
}

fn read_nodes(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Vec<SankeyNode> {
    let mut nodes = Vec::new();
    for mapping in entries(root, NODES_KEY, "node", document, problems) {
        report_unknown_keys(mapping, &NODE_KEYS, document, problems, "node");
        let mut column = None;
        if let Some(stated) = number(mapping, "column", document, problems) {
            if stated >= 1.0 && (stated - stated.floor()).abs() < TOLERANCE {
                column = Some(stated.min(1000.0) as i32);
            } else {
                problem(
                    problems,
                    yaml::line_of(mapping, document),
                    format!(
                        "`column: {}` is not a whole number from 1; the flows place this node.",
                        round_trip(stated)
                    ),
                );
            }
        }
        nodes.push(SankeyNode {
            id: text(mapping, "id"),
            name: text(mapping, "name"),
            color: text(mapping, "color"),
            note: text(mapping, "note"),
            format: text(mapping, "format"),
            column,
            description: text(mapping, "description"),
            range: yaml::range_of(mapping, document),
        });
    }
    nodes
}

fn read_flows(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Vec<SankeyFlow> {
    let mut flows = Vec::new();
    for mapping in entries(root, FLOWS_KEY, "flow", document, problems) {
        report_unknown_keys(mapping, &FLOW_KEYS, document, problems, "flow");
        let value = number(mapping, "value", document, problems);
        let step = number(mapping, "step", document, problems);
        flows.push(SankeyFlow {
            explicit_id: text(mapping, "id"),
            from: text(mapping, "from"),
            to: text(mapping, "to"),
            value,
            step,
            color: text(mapping, "color"),
            description: text(mapping, "description"),
            range: yaml::range_of(mapping, document),
        });
    }
    flows
}

/// The standalone host's `double.Tolerance`.
const TOLERANCE: f64 = 0.000_001_f32 as f64;

/// A number as .NET's invariant `ToString()` writes it: the shortest text that reads back as it.
fn round_trip(value: f64) -> String {
    format!("{value}")
}

/// The mappings under a section, reporting every entry that is not one.
fn entries<'a>(
    root: &'a YamlNode,
    key: &str,
    what: &str,
    document: &LineDocument,
    problems: &mut Vec<ParseProblem>,
) -> Vec<&'a YamlNode> {
    let Some(YamlNode::Sequence { items, .. }) = root.get(key) else {
        return Vec::new();
    };
    let mut mappings = Vec::new();
    for node in items {
        if matches!(node, YamlNode::Mapping { .. }) {
            mappings.push(node);
        } else {
            problem(
                problems,
                yaml::line_of(node, document),
                format!("A {what} entry is not a mapping and was passed over."),
            );
        }
    }
    mappings
}

fn report_unknown_keys(
    mapping: &YamlNode,
    known: &[&str],
    document: &LineDocument,
    problems: &mut Vec<ParseProblem>,
    what: &str,
) {
    for (name, key) in mapping.keys() {
        if !known.contains(&name) {
            problem(
                problems,
                yaml::line_of(key, document),
                format!("`{name}` is not a key this module reads on a {what}; the line is kept."),
            );
        }
    }
}

/// A number from the entry, or none when the key is absent or will not read: the second is reported, because a
/// silently missing value reads as a lost edit.
fn number(mapping: &YamlNode, key: &str, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Option<f64> {
    let scalar @ YamlNode::Scalar { value, .. } = mapping.get(key)? else {
        return None;
    };
    match parse_number(value) {
        Some(number) if number.is_finite() => Some(number),
        _ => {
            problem(
                problems,
                yaml::line_of(scalar, document),
                format!("`{key}: {value}` is not a number and was ignored."),
            );
            None
        }
    }
}

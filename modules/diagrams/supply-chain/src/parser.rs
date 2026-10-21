//! Reads a `.supply` document into a [`SupplyChainModel`], recording which lines declare what (etalii.adp spec 030,
//! task T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainParser.cs` at `8b6ccfc`. The parser never fails: a document that is not YAML opens empty and says why;
//! an unknown key, an unknown stage or a number that will not read is kept, passed over and reported, so a half-typed
//! document opens and keeps every byte its author wrote.

use crate::lines::{LineDocument, parse_number};
use crate::model::{
    CURRENT_VERSION, ParseProblem, SupplyChainFlow, SupplyChainGroup, SupplyChainModel, SupplyChainNode, stages,
};
use crate::yaml::{self, YamlNode};

pub const HEADER_KEY: &str = "supply-chain";
pub const GROUPS_KEY: &str = "groups";
pub const NODES_KEY: &str = "nodes";
pub const FLOWS_KEY: &str = "flows";

/// The text of a new, empty `.supply` document: the header and the three sections written flow-empty, because a bare
/// `nodes:` reads as null rather than an empty list; CRLF, since a new file has no line ending of its own to keep.
pub const EMPTY_DOCUMENT: &str = "supply-chain: 1\r\ngroups: []\r\nnodes: []\r\nflows: []\r\n";

const GROUP_KEYS: [&str; 5] = ["id", "name", "description", "x", "y"];
const NODE_KEYS: [&str; 10] = [
    "id",
    "type",
    "name",
    "description",
    "group",
    "quantity",
    "unit",
    "step",
    "x",
    "y",
];
const FLOW_KEYS: [&str; 8] = ["id", "from", "to", "product", "description", "volume", "unit", "step"];

/// What reading a document came to: its model, and for a text that is not YAML, why, so that nothing is edited or
/// written until it is (`ParseOutcome`).
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutcome {
    pub model: SupplyChainModel,
    pub unreadable: Option<String>,
}

/// Reads the document.
pub fn parse(document: &LineDocument) -> SupplyChainModel {
    outcome(document).model
}

/// Reads the document, keeping why it could not be read.
pub fn outcome(document: &LineDocument) -> ParseOutcome {
    let root = match yaml::parse(&document.text()) {
        Ok(root) => root,
        Err(error) => {
            let message = format!("The document could not be read as YAML: {}", error.message);
            return ParseOutcome {
                model: SupplyChainModel {
                    problems: vec![ParseProblem {
                        line: error.line.saturating_sub(1),
                        message: message.clone(),
                    }],
                    ..SupplyChainModel::default()
                },
                unreadable: Some(message),
            };
        }
    };
    let Some(root @ YamlNode::Mapping { .. }) = root else {
        return ParseOutcome {
            model: SupplyChainModel::default(),
            unreadable: None,
        };
    };
    let mut problems = Vec::new();
    let groups = read_groups(&root, document, &mut problems);
    let nodes = read_nodes(&root, document, &mut problems);
    let flows = read_flows(&root, document, &mut problems);
    let version = read_version(&root, document, &mut problems);
    ParseOutcome {
        model: SupplyChainModel {
            groups,
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

fn read_version(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Option<i64> {
    let Some(scalar @ YamlNode::Scalar { value, .. }) = root.get(HEADER_KEY) else {
        problem(
            problems,
            0,
            format!("The document does not begin with `{HEADER_KEY}: {CURRENT_VERSION}`."),
        );
        return None;
    };
    let Some(version) = parse_int(value) else {
        problem(
            problems,
            yaml::line_of(scalar, document),
            format!("The version `{value}` is not a number."),
        );
        return None;
    };
    if version != CURRENT_VERSION {
        problem(
            problems,
            yaml::line_of(scalar, document),
            format!("This document states version {version}; this module reads version {CURRENT_VERSION}."),
        );
    }
    Some(version)
}

/// `int.TryParse` with `NumberStyles.Integer`: white space around, a sign, digits.
fn parse_int(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    let digits = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: i64 = trimmed.trim_start_matches('+').parse().ok()?;
    i32::try_from(value).ok().map(i64::from)
}

fn read_groups(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Vec<SupplyChainGroup> {
    entries(root, GROUPS_KEY, "group", document, problems)
        .into_iter()
        .map(|mapping| {
            report_unknown_keys(mapping, &GROUP_KEYS, document, problems, "group");
            SupplyChainGroup {
                id: text(mapping, "id"),
                name: text(mapping, "name"),
                description: text(mapping, "description"),
                range: yaml::range_of(mapping, document),
                x: number(mapping, "x", document, problems),
                y: number(mapping, "y", document, problems),
            }
        })
        .collect()
}

fn read_nodes(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Vec<SupplyChainNode> {
    entries(root, NODES_KEY, "node", document, problems)
        .into_iter()
        .map(|mapping| {
            report_unknown_keys(mapping, &NODE_KEYS, document, problems, "node");
            // A legacy stage word reads as the stage it became; the line keeps its word until edited.
            let node_type = stages::normalize(&text(mapping, "type")).to_owned();
            if !stages::is_known(&node_type) {
                problem(
                    problems,
                    yaml::line_of(mapping, document),
                    format!("`{node_type}` is not a stage this module knows."),
                );
            }
            SupplyChainNode {
                id: text(mapping, "id"),
                node_type,
                name: text(mapping, "name"),
                description: text(mapping, "description"),
                group: text(mapping, "group"),
                quantity: number(mapping, "quantity", document, problems),
                unit: text(mapping, "unit"),
                step: number(mapping, "step", document, problems),
                x: number(mapping, "x", document, problems),
                y: number(mapping, "y", document, problems),
                range: yaml::range_of(mapping, document),
            }
        })
        .collect()
}

fn read_flows(root: &YamlNode, document: &LineDocument, problems: &mut Vec<ParseProblem>) -> Vec<SupplyChainFlow> {
    entries(root, FLOWS_KEY, "flow", document, problems)
        .into_iter()
        .map(|mapping| {
            report_unknown_keys(mapping, &FLOW_KEYS, document, problems, "flow");
            SupplyChainFlow {
                id: text(mapping, "id"),
                from: text(mapping, "from"),
                to: text(mapping, "to"),
                product: text(mapping, "product"),
                description: text(mapping, "description"),
                volume: number(mapping, "volume", document, problems),
                unit: text(mapping, "unit"),
                step: number(mapping, "step", document, problems),
                range: yaml::range_of(mapping, document),
            }
        })
        .collect()
}

fn text(mapping: &YamlNode, key: &str) -> String {
    mapping.scalar(key).unwrap_or_default().to_owned()
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
    let YamlNode::Mapping { entries, .. } = mapping else {
        return;
    };
    for (key, _) in entries {
        if let YamlNode::Scalar { value, .. } = key
            && !known.contains(&value.as_str())
        {
            problem(
                problems,
                yaml::line_of(key, document),
                format!("`{value}` is not a key this module reads on a {what}; the line is kept."),
            );
        }
    }
}

/// A number from the entry, or none when the key is absent or will not read - the second of which is reported,
/// because a silently missing value reads as a lost edit.
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

//! The designer's model onto the host's hierarchy surface (DESL section 7.7; the standalone host's
//! `HierarchicalRequirementSurfaceMapper` at a7fdf03): the items as its levels, the requirements as its entries, the
//! tags, category and state as its labels and the assessments as its notes. The surface knows none of these words;
//! this is where they are said.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::LazyLock;

use adp_desl::hierarchy::model::{
    ColumnWidth, LabelValues, Note, OptionList, Reference, SurfaceFilter, SurfaceFinding, SurfaceModel, SurfaceNode,
    SurfaceOption, roles,
};
use regex::Regex;

use crate::body::lower_ascii;
use crate::definition;
use crate::model::{Assessment, Document, Entry};

/// What a gesture names a column of levels by: this, then its depth.
pub const COLUMN_PREFIX: &str = "column/";

/// The document's first heading of level one.
static TITLE_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^#[ \t]+(?<title>[^\r\n]+)").expect("the pattern compiles"));

/// The id of a node without a usable number: the line of its bullet, valid for one model.
fn line_id(line: usize) -> String {
    format!("line/{line}")
}

/// The document as the surface shows it, following `cause`. `stored` gives the names each list's block holds, by
/// block, or none for a block the registration does not hold.
pub fn map(
    document: &Document,
    title: &str,
    cause: &str,
    stored: &dyn Fn(&str) -> Option<Vec<String>>,
) -> SurfaceModel {
    let ids = node_ids(&document.entries);
    let mut by_number: HashMap<&str, &str> = HashMap::new();
    for (entry, id) in document.entries.iter().zip(&ids) {
        by_number.entry(entry.id.as_str()).or_insert(id.as_str());
    }
    let nodes: Vec<SurfaceNode> = document
        .entries
        .iter()
        .zip(&ids)
        .map(|(entry, id)| node(entry, id, &by_number))
        .collect();
    // A finding stands at the entry whose bullet or description holds its line, so selecting it shows that box.
    let mut node_at: HashMap<usize, &str> = HashMap::new();
    for (entry, node) in document.entries.iter().zip(&nodes) {
        for line in entry.line..=entry.line + description_lines(entry) {
            node_at.entry(line).or_insert(node.id.as_str());
        }
    }
    let option_lists = vec![
        options("tags", "tag", "tags", &document.tags, &nodes, stored),
        options(
            "category",
            "category",
            "categories",
            &document.categories,
            &nodes,
            stored,
        ),
        options("state", "state", "states", &document.states, &nodes, stored),
    ];
    SurfaceModel {
        title: title.to_owned(),
        read_only_reason: document.read_only_reason.clone(),
        cause: cause.to_owned(),
        option_lists,
        filters: document
            .filters
            .iter()
            .map(|filter| SurfaceFilter {
                id: format!("filter/{}", filter.name),
                name: filter.name.clone(),
                query: filter.query.clone(),
            })
            .collect(),
        notes: document
            .assessments
            .iter()
            .map(|assessment| note(assessment, &by_number))
            .collect(),
        findings: document
            .findings
            .iter()
            .map(|finding| SurfaceFinding {
                code: finding.code.clone(),
                severity: severity(&finding.code).to_owned(),
                message: finding.message.clone(),
                node_id: if finding.in_registration {
                    String::new()
                } else {
                    node_at
                        .get(&finding.line)
                        .map(|id| (*id).to_owned())
                        .unwrap_or_default()
                },
                line: finding.line,
            })
            .collect(),
        widths: document
            .columns
            .iter()
            .map(|column| ColumnWidth {
                id: format!("{COLUMN_PREFIX}{}", column.level),
                level: column.level,
                width: column.width,
            })
            .collect(),
        nodes,
    }
}

/// The lines below an entry's bullet that its description takes.
fn description_lines(entry: &Entry) -> usize {
    if entry.description.is_empty() {
        0
    } else {
        entry.description.split('\n').count()
    }
}

/// The id of each entry: its number when that is usable, else the line of its bullet. A placeholder's number is
/// given in memory only, and a later duplicate's is the earlier entry's, so neither can be named by it.
pub fn node_ids(entries: &[Entry]) -> Vec<String> {
    let mut seen = HashSet::new();
    entries
        .iter()
        .map(|entry| {
            if !entry.placeholder && seen.insert(entry.id.as_str()) {
                entry.id.clone()
            } else {
                line_id(entry.line)
            }
        })
        .collect()
}

fn node(entry: &Entry, id: &str, by_number: &HashMap<&str, &str>) -> SurfaceNode {
    let mut labels = vec![LabelValues {
        label: "tags".into(),
        values: entry.tags.clone(),
    }];
    if let Some(category) = entry.category() {
        labels.push(LabelValues {
            label: "category".into(),
            values: vec![category.to_owned()],
        });
    }
    if let Some(state) = entry.state() {
        labels.push(LabelValues {
            label: "state".into(),
            values: vec![state.to_owned()],
        });
    }
    let number = if entry.placeholder {
        format!("{}NEW", if entry.is_item() { "ITEM-" } else { "REQ-" })
    } else {
        entry.id.clone()
    };
    SurfaceNode {
        id: id.to_owned(),
        number,
        role: if entry.is_item() { roles::LEVEL } else { roles::ENTRY }.to_owned(),
        type_name: entry.type_name().to_owned(),
        parent_id: entry
            .parent_id
            .as_deref()
            .and_then(|parent| by_number.get(parent))
            .map(|id| (*id).to_owned())
            .unwrap_or_default(),
        title: entry.title.clone(),
        description: entry.description.clone(),
        labels,
        references: references(entry.type_name(), "description", &entry.description, 0, by_number),
        collapsed: entry.collapsed && id == entry.id,
    }
}

fn note(assessment: &Assessment, by_number: &HashMap<&str, &str>) -> Note {
    let mut references = references("Assessment", "text", &assessment.text, 0, by_number);
    references.extend(references_after(assessment, by_number));
    Note {
        id: assessment.id.clone(),
        kind: assessment.kind.name().to_owned(),
        about: assessment.about.clone(),
        about_id: by_number
            .get(assessment.about.as_str())
            .map(|id| (*id).to_owned())
            .unwrap_or_default(),
        text: assessment.text.clone(),
        detail: assessment.detail.clone(),
        resolved: assessment.resolved,
        references,
    }
}

/// The detail's references, offset after the text and one LF.
fn references_after(assessment: &Assessment, by_number: &HashMap<&str, &str>) -> Vec<Reference> {
    let offset = utf16_len(&assessment.text) + 1;
    references("Assessment", "detail", &assessment.detail, offset, by_number)
}

/// The length of `text` in UTF-16 code units, as the standalone host counts an offset.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// The bracketed numbers `text` names, as the specification's mentions find them; a link is none.
fn references(
    type_name: &str,
    attribute: &str,
    text: &str,
    offset: usize,
    by_number: &HashMap<&str, &str>,
) -> Vec<Reference> {
    if text.is_empty() {
        return Vec::new();
    }
    let Some(pattern) = definition::mentions(type_name, attribute) else {
        return Vec::new();
    };
    pattern
        .captures_iter(text)
        .filter_map(|mention| {
            let id = mention.name("id")?;
            let whole = mention.get(0)?;
            Some(Reference {
                start: offset + utf16_len(&text[..whole.start()]),
                length: utf16_len(whole.as_str()),
                target_id: by_number
                    .get(id.as_str())
                    .map(|id| (*id).to_owned())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// One label's list: its members in their order, then any a node carries that the list does not hold. A member is
/// implicit when the list does not hold it: neither its block nor, while there is none, its initial members.
fn options(
    label: &str,
    id_prefix: &str,
    block: &str,
    members: &[crate::model::Member],
    nodes: &[SurfaceNode],
    stored: &dyn Fn(&str) -> Option<Vec<String>>,
) -> OptionList {
    let held: HashSet<String> = stored(block)
        .unwrap_or_else(|| definition::initial(block).into_iter().map(|(name, _)| name).collect())
        .iter()
        .map(|name| lower_ascii(name))
        .collect();
    let carried = nodes
        .iter()
        .flat_map(|node| &node.labels)
        .filter(|values| values.label == label)
        .flat_map(|values| &values.values)
        .map(|name| (name.as_str(), "default"));
    let mut seen = HashSet::new();
    let mut options = Vec::new();
    for (name, colour) in members
        .iter()
        .map(|m| (m.name.as_str(), m.colour.as_str()))
        .chain(carried)
    {
        let key = lower_ascii(name);
        if seen.insert(key.clone()) {
            options.push(SurfaceOption {
                id: format!("{id_prefix}/{key}"),
                name: name.to_owned(),
                colour: colour.to_owned(),
                implicit: !held.contains(&key),
            });
        }
    }
    OptionList {
        label: label.to_owned(),
        options,
    }
}

/// How much a finding weighs, as the companion's table "What reading reports" says.
fn severity(code: &str) -> &'static str {
    match code {
        "std.missingId" | "std.mentions" => "warning",
        "fbl.unknown-block" => "info",
        _ => "error",
    }
}

/// The document's title: its first heading of level one, or the file's base name when it has none.
pub fn title(text: &str, file_name: &str) -> String {
    match TITLE_LINE.captures(text) {
        Some(c) => c["title"].trim().to_owned(),
        None => Path::new(file_name)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    }
}

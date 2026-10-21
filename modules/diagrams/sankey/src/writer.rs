//! Every edit as a splice of the lines it affects, and nothing else (etalii.adp spec 030, task T077).
//!
//! Ported from the standalone host's `SankeyWriter.cs` at `8b6ccfc`. Nothing serialises a model back to a file:
//! every write splices a range the parser recorded, so an unknown key, a comment and the author's own indentation
//! all survive by construction. A write invalidates the model that located it: line numbers move after a splice, so
//! a caller making several edits applies them bottom up or reads the document again between them.

use crate::lines::{self, LineDocument, LineRange, dsl_number, quote_text};
use crate::model::{SankeyFlow, SankeyModel, SankeyNode};
use crate::parser::{FLOWS_KEY, HEADER_KEY, NODES_KEY};

/// Writes a text key, or removes it when `remove_when_empty` and the value is blank.
pub fn set_text(document: &mut LineDocument, range: LineRange, key: &str, value: &str, remove_when_empty: bool) {
    if remove_when_empty && value.trim().is_empty() {
        lines::remove_key(document, range, key);
    } else {
        lines::set_key(document, range, key, &quote_text(value));
    }
}

/// Writes a number key, or removes it for none.
pub fn set_number(document: &mut LineDocument, range: LineRange, key: &str, value: Option<f64>) {
    match value {
        None => lines::remove_key(document, range, key),
        Some(value) => lines::set_key(document, range, key, &dsl_number(value)),
    }
}

/// Writes one of the document-wide keys on its own line, or adds it under the header when the document does not
/// state it yet.
pub fn set_root_key(document: &mut LineDocument, line: Option<usize>, key: &str, value: &str) {
    match line {
        Some(line) if line < document.lines().len() => {
            document.replace(LineRange::new(line, line), &[format!("{key}: {value}")]);
        }
        _ => {
            let header = lines::find_section(document, &format!("{HEADER_KEY}:"));
            document.insert(header.map_or(0, |at| at + 1), &[format!("{key}: {value}")]);
        }
    }
}

/// Removes a node and every flow to or from it, bottom up.
pub fn remove_node(document: &mut LineDocument, model: &SankeyModel, node: &SankeyNode) {
    let mut ranges: Vec<LineRange> = model
        .flows
        .iter()
        .filter(|flow| flow.from == node.id || flow.to == node.id)
        .map(|flow| flow.range)
        .chain(std::iter::once(node.range))
        .collect();
    // Bottom up, two ranges at one start in the order they came, as a stable descending sort leaves them.
    ranges.sort_by_key(|range| std::cmp::Reverse(range.start));
    for range in ranges {
        document.remove(range);
    }
}

/// Removes one entry: a flow, typically.
pub fn remove(document: &mut LineDocument, range: LineRange) {
    document.remove(range);
}

/// Moves a node's entry so it comes directly before `anchor`, or directly after it when `after`: how a node changes
/// its place in its column, since a column is drawn in the order its nodes are written. The entry's own lines move,
/// comments inside it included, and nothing else is touched.
pub fn move_node(
    document: &mut LineDocument,
    node: &SankeyNode,
    anchor: &SankeyNode,
    after: bool,
) -> Result<(), String> {
    if node.id == anchor.id {
        return Err("A node cannot be moved next to itself.".to_owned());
    }
    let moved: Vec<String> = (node.range.start..=node.range.end)
        .map(|index| document.line(index).to_owned())
        .collect();
    let mut at = if after {
        anchor.range.end + 1
    } else {
        anchor.range.start
    };
    if at > node.range.start {
        at -= node.range.len();
    }
    document.remove(node.range);
    document.insert(at, &moved);
    Ok(())
}

/// Adds a node entry in the indentation the document already uses: directly before `before` when one is given, so it
/// lands at its place in its column, and after the last node otherwise.
pub fn add_node(document: &mut LineDocument, model: &SankeyModel, node: &SankeyNode, before: Option<&SankeyNode>) {
    let mut keys = vec![("name", quote_text(&node.name))];
    if !node.color.is_empty() {
        keys.push(("color", quote_text(&node.color)));
    }
    if let Some(column) = node.column {
        keys.push(("column", column.to_string()));
    }
    let ranges: Vec<LineRange> = model.nodes.iter().map(|existing| existing.range).collect();
    match before {
        None => append(document, &ranges, &format!("{NODES_KEY}:"), &node.id, &keys),
        Some(before) => {
            let (item_indent, dash_gap, key_indent) = lines::indent_of_entries(document, &ranges);
            document.insert(
                before.range.start,
                &entry(&item_indent, &dash_gap, &key_indent, &node.id, &keys),
            );
        }
    }
}

/// Appends a flow entry: its two ends and its value, and no id, since its ends are its name.
pub fn add_flow(document: &mut LineDocument, model: &SankeyModel, flow: &SankeyFlow) {
    let section = format!("{FLOWS_KEY}:");
    let ranges: Vec<LineRange> = model.flows.iter().map(|existing| existing.range).collect();
    let at = match lines::insertion_point_for(document, &ranges, &section) {
        Some(at) => at,
        None => {
            // A hand-written document may leave the section out; the entry brings its section with it.
            let end = document.lines().len();
            document.insert(end, std::slice::from_ref(&section));
            document.lines().len()
        }
    };
    let (item_indent, dash_gap, key_indent) = lines::indent_of_entries(document, &ranges);
    let mut added = vec![
        format!("{item_indent}-{dash_gap}from: {}", quote_text(&flow.from)),
        format!("{key_indent}to: {}", quote_text(&flow.to)),
    ];
    if let Some(value) = flow.value {
        added.push(format!("{key_indent}value: {}", dsl_number(value)));
    }
    document.insert(at, &added);
}

fn append(document: &mut LineDocument, ranges: &[LineRange], section: &str, id: &str, keys: &[(&str, String)]) {
    let at = match lines::insertion_point_for(document, ranges, section) {
        Some(at) => at,
        None => {
            let end = document.lines().len();
            document.insert(end, &[section.to_owned()]);
            document.lines().len()
        }
    };
    let (item_indent, dash_gap, key_indent) = lines::indent_of_entries(document, ranges);
    document.insert(at, &entry(&item_indent, &dash_gap, &key_indent, id, keys));
}

fn entry(item_indent: &str, dash_gap: &str, key_indent: &str, id: &str, keys: &[(&str, String)]) -> Vec<String> {
    std::iter::once(format!("{item_indent}-{dash_gap}id: {}", quote_text(id)))
        .chain(keys.iter().map(|(key, value)| format!("{key_indent}{key}: {value}")))
        .collect()
}

//! Turns an edit into a splice of the lines that edit affects, and nothing else (etalii.adp spec 030, task T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainWriter.cs` at `8b6ccfc`. Nothing serialises a model back to a file: every write splices a range the
//! parser recorded, so an unknown key, a comment and the author's own indentation all survive by construction. A
//! write invalidates the model that located it, since lines move after a splice: several edits are made bottom-up, or
//! the document is read again between them.

use indexmap::IndexMap;

use crate::lines::{self, LineDocument, LineRange, format_number, quote_text, round};
use crate::model::{SupplyChainFlow, SupplyChainGroup, SupplyChainModel, SupplyChainNode};
use crate::parser::{FLOWS_KEY, GROUPS_KEY, NODES_KEY};

/// Writes a text key, removing it instead when `remove_when_empty` and the value is blank.
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
        Some(value) => lines::set_key(document, range, key, &format_number(value)),
    }
}

/// Places several nodes at once, bottom-up so each splice leaves the ranges above it valid; `y` first, so a newly
/// placed entry reads `x` before `y` (both are inserted right after the entry's first line).
pub fn place(
    document: &mut LineDocument,
    model: &SupplyChainModel,
    places: &IndexMap<String, (f64, f64)>,
) -> Result<(), String> {
    let mut targets: Vec<&SupplyChainNode> = Vec::new();
    for node in &model.nodes {
        if places.contains_key(&node.id) && !targets.iter().any(|t| t.id == node.id) {
            targets.push(node);
        }
    }
    if targets.is_empty() {
        return Err("There is nothing here to place.".to_owned());
    }
    // A stable sort by descending start, as `OrderByDescending` sorts.
    targets.sort_by_key(|n| std::cmp::Reverse(n.range.start));
    for node in targets {
        let (x, y) = places[&node.id];
        place_range(document, node.range, x, y);
    }
    Ok(())
}

fn place_range(document: &mut LineDocument, range: LineRange, x: f64, y: f64) {
    let grows = usize::from(lines::find_key(document, range, "y").is_none());
    lines::set_key(document, range, "y", &format_number(round(y)));
    lines::set_key(document, range.extended_by(grows), "x", &format_number(round(x)));
}

/// Places a group's own frame: what it is drawn at while it has no members.
pub fn place_group(document: &mut LineDocument, group: &SupplyChainGroup, x: f64, y: f64) {
    place_range(document, group.range, x, y);
}

/// Removes a node and every flow to or from it, bottom-up.
pub fn remove_node(document: &mut LineDocument, model: &SupplyChainModel, node: &SupplyChainNode) {
    let mut ranges: Vec<LineRange> = model
        .flows
        .iter()
        .filter(|f| f.from == node.id || f.to == node.id)
        .map(|f| f.range)
        .collect();
    ranges.push(node.range);
    remove_bottom_up(document, ranges);
}

/// Removes a group, and its id from every node that was inside it.
pub fn remove_group(document: &mut LineDocument, model: &SupplyChainModel, group: &SupplyChainGroup) {
    enum Edit {
        Ungroup(LineRange),
        Remove(LineRange),
    }
    let mut edits: Vec<(usize, Edit)> = model
        .nodes
        .iter()
        .filter(|n| n.group == group.id)
        .map(|n| (n.range.start, Edit::Ungroup(n.range)))
        .collect();
    edits.push((group.range.start, Edit::Remove(group.range)));
    edits.sort_by_key(|e| std::cmp::Reverse(e.0));
    for (_, edit) in edits {
        match edit {
            Edit::Ungroup(range) => lines::remove_key(document, range, "group"),
            Edit::Remove(range) => document.remove(range),
        }
    }
}

/// Removes one entry: a flow, typically.
pub fn remove(document: &mut LineDocument, range: LineRange) {
    document.remove(range);
}

/// Appends a node entry, matching whatever indentation the document already uses.
pub fn add_node(document: &mut LineDocument, model: &SupplyChainModel, node: &SupplyChainNode) {
    let mut keys = vec![("type", node.node_type.clone()), ("name", quote_text(&node.name))];
    if !node.group.is_empty() {
        keys.push(("group", quote_text(&node.group)));
    }
    if let Some(quantity) = node.quantity {
        keys.push(("quantity", format_number(quantity)));
    }
    if !node.unit.is_empty() {
        keys.push(("unit", quote_text(&node.unit)));
    }
    if let (Some(x), Some(y)) = (node.x, node.y) {
        keys.push(("x", format_number(round(x))));
        keys.push(("y", format_number(round(y))));
    }
    let ranges: Vec<LineRange> = model.nodes.iter().map(|n| n.range).collect();
    append(document, &ranges, NODES_KEY, &node.id, &keys);
}

/// Appends a group entry.
pub fn add_group(document: &mut LineDocument, model: &SupplyChainModel, group: &SupplyChainGroup) {
    let mut keys = vec![("name", quote_text(&group.name))];
    if let (Some(x), Some(y)) = (group.x, group.y) {
        keys.push(("x", format_number(round(x))));
        keys.push(("y", format_number(round(y))));
    }
    let ranges: Vec<LineRange> = model.groups.iter().map(|g| g.range).collect();
    append(document, &ranges, GROUPS_KEY, &group.id, &keys);
}

/// Appends a flow entry.
pub fn add_flow(document: &mut LineDocument, model: &SupplyChainModel, flow: &SupplyChainFlow) {
    let mut keys = vec![("from", quote_text(&flow.from)), ("to", quote_text(&flow.to))];
    if !flow.product.is_empty() {
        keys.push(("product", quote_text(&flow.product)));
    }
    if let Some(volume) = flow.volume {
        keys.push(("volume", format_number(volume)));
    }
    if !flow.unit.is_empty() {
        keys.push(("unit", quote_text(&flow.unit)));
    }
    let ranges: Vec<LineRange> = model.flows.iter().map(|f| f.range).collect();
    append(document, &ranges, FLOWS_KEY, &flow.id, &keys);
}

fn append(document: &mut LineDocument, ranges: &[LineRange], section: &str, id: &str, keys: &[(&str, String)]) {
    let section = format!("{section}:");
    let at = match lines::insertion_point(document, ranges, &section) {
        Some(at) => at,
        None => {
            // A hand-written document may leave a section out; the entry brings its section with it.
            let end = document.lines().len();
            document.insert(end, std::slice::from_ref(&section));
            document.lines().len()
        }
    };
    let (item_indent, dash_gap, key_indent) = lines::indent_from(document, ranges);
    let mut written = vec![format!("{item_indent}-{dash_gap}id: {}", quote_text(id))];
    written.extend(keys.iter().map(|(key, value)| format!("{key_indent}{key}: {value}")));
    document.insert(at, &written);
}

fn remove_bottom_up(document: &mut LineDocument, mut ranges: Vec<LineRange>) {
    ranges.sort_by_key(|r| std::cmp::Reverse(r.start));
    for range in ranges {
        document.remove(range);
    }
}

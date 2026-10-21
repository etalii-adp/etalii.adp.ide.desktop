//! Every edit of a `.fdg` as a splice of the lines it affects, and nothing else (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host's `src/diagrams/functional-decomposition-graph/backend/.../FdgWriter.cs` at
//! `8b6ccfc`. Nothing serialises a model back to a file, which is what makes "only the affected lines change" true by
//! construction, and what lets an unknown key survive: a line nobody splices is a line nobody can damage. A write
//! invalidates the model that located it, so a caller reads the document again rather than adjusting ranges.

use crate::lines::{self, LineDocument, LineRange};
use crate::model::{FdgConnection, FdgElement, FdgModel, geometry};

const ELEMENTS_SECTION: &str = "elements:";
const CONNECTIONS_SECTION: &str = "connections:";

/// What an edit came to: made, or refused with the sentence that says why.
pub type EditOutcome = Result<(), String>;

/// Rewrites an element's name; a Comment has none, and says so.
pub fn set_name(document: &mut LineDocument, element: &FdgElement, name: &str) -> EditOutcome {
    if element.is_comment() {
        return Err("A Comment has no name; its text is what it says.".to_owned());
    }
    lines::set_key(document, element.range, "name", &lines::quote(name))
}

/// Rewrites an element's description; an empty one removes the key.
pub fn set_description(document: &mut LineDocument, element: &FdgElement, description: &str) -> EditOutcome {
    set_or_remove(document, element.range, "description", description)
}

/// Rewrites a connection's description, the same way an element's is.
pub fn set_connection_description(
    document: &mut LineDocument,
    connection: &FdgConnection,
    description: &str,
) -> EditOutcome {
    set_or_remove(document, connection.range, "description", description)
}

fn set_or_remove(document: &mut LineDocument, range: LineRange, key: &str, value: &str) -> EditOutcome {
    if value.trim().is_empty() {
        lines::remove_key(document, range, key)
    } else {
        lines::set_key(document, range, key, &lines::quote(value))
    }
}

/// The lines of a Comment's text as a `|-` block scalar under `indent`, so its newlines survive the round trip
/// without a trailing newline the author did not write.
fn block(indent: &str, text: &str) -> Vec<String> {
    let mut lines = vec![format!("{indent}text: |-")];
    lines.extend(
        text.split('\n')
            .map(|line| format!("{indent}  {}", line.trim_end_matches('\r'))),
    );
    lines
}

/// Rewrites a Comment's text as a block scalar; only a Comment carries text.
pub fn set_text(document: &mut LineDocument, element: &FdgElement, text: &str) -> EditOutcome {
    if !element.is_comment() {
        return Err("Only a Comment carries text.".to_owned());
    }
    let indent = lines::key_indent_within(document, element.range);
    replace_key_block(document, element.range, "text", &block(&indent, text))
}

/// Moves an element by rewriting its top-left corner.
pub fn move_to(document: &mut LineDocument, element: &FdgElement, x: f64, y: f64) -> EditOutcome {
    lines::set_key(document, element.range, "x", &lines::format_number(x))?;
    lines::set_key(document, element.range, "y", &lines::format_number(y))
}

/// Resizes an element: the width clamped to the minimum rather than refused, and a height written for a Comment
/// only. The width is written before a height is refused, so a caller edits a copy.
pub fn resize(document: &mut LineDocument, element: &FdgElement, width: f64, height: Option<f64>) -> EditOutcome {
    lines::set_key(
        document,
        element.range,
        "width",
        &lines::format_number(width.max(geometry::MINIMUM_WIDTH)),
    )?;
    let Some(height) = height else {
        return Ok(());
    };
    if !element.is_comment() {
        return Err("Only a Comment has its own height; the others share one.".to_owned());
    }
    lines::set_key(
        document,
        element.range,
        "height",
        &lines::format_number(height.max(geometry::MINIMUM_COMMENT_HEIGHT)),
    )
}

/// Rewrites a connection's name.
pub fn set_connection_name(document: &mut LineDocument, connection: &FdgConnection, name: &str) -> EditOutcome {
    lines::set_key(document, connection.range, "name", &lines::quote(name))
}

/// Removes an element and every connection touching it, bottom-up, since each removal moves every line after it.
pub fn remove_element(document: &mut LineDocument, model: &FdgModel, element: &FdgElement) -> EditOutcome {
    let mut ranges: Vec<LineRange> = model
        .connections
        .iter()
        .filter(|c| c.from == element.id || c.to == element.id)
        .map(|c| c.range)
        .chain(std::iter::once(element.range))
        .collect();
    // A stable sort, descending by start, as LINQ's OrderByDescending.
    ranges.sort_by_key(|r| std::cmp::Reverse(r.start));
    for range in ranges {
        document.remove(range)?;
    }
    Ok(())
}

/// Removes one connection.
pub fn disconnect(document: &mut LineDocument, connection: &FdgConnection) -> EditOutcome {
    document.remove(connection.range)
}

/// Appends an element entry, matching whatever indentation the document already uses.
pub fn add_element(document: &mut LineDocument, model: &FdgModel, element: &FdgElement) -> EditOutcome {
    let ranges: Vec<LineRange> = model.elements.iter().map(|e| e.range).collect();
    let Some(at) = lines::insertion_point_for(document, &ranges, ELEMENTS_SECTION) else {
        return Err(format!("The document has no `{ELEMENTS_SECTION}` section to add to."));
    };
    let (item_indent, dash_gap, key_indent) = lines::indent_of_entries(document, &ranges);
    let mut entry = vec![
        format!("{item_indent}-{dash_gap}id: {}", lines::quote(&element.id)),
        format!("{key_indent}type: {}", element.element_type),
    ];
    if element.is_comment() {
        entry.extend(block(&key_indent, &element.text));
    } else {
        entry.push(format!("{key_indent}name: {}", lines::quote(&element.name)));
    }
    entry.push(format!("{key_indent}x: {}", lines::format_number(element.x)));
    entry.push(format!("{key_indent}y: {}", lines::format_number(element.y)));
    entry.push(format!(
        "{key_indent}width: {}",
        lines::format_number(element.width.max(geometry::MINIMUM_WIDTH))
    ));
    if element.is_comment() {
        entry.push(format!(
            "{key_indent}height: {}",
            lines::format_number(element.drawn_height().max(geometry::MINIMUM_COMMENT_HEIGHT))
        ));
    }
    document.insert(at, &entry)
}

/// Appends a connection entry.
pub fn connect(document: &mut LineDocument, model: &FdgModel, connection: &FdgConnection) -> EditOutcome {
    let ranges: Vec<LineRange> = model.connections.iter().map(|c| c.range).collect();
    let Some(at) = lines::insertion_point_for(document, &ranges, CONNECTIONS_SECTION) else {
        return Err(format!(
            "The document has no `{CONNECTIONS_SECTION}` section to add to."
        ));
    };
    let (item_indent, dash_gap, key_indent) = lines::indent_of_entries(document, &ranges);
    let mut entry = vec![
        format!("{item_indent}-{dash_gap}id: {}", lines::quote(&connection.id)),
        format!("{key_indent}type: {}", connection.connection_type),
        format!("{key_indent}from: {}", lines::quote(&connection.from)),
        format!("{key_indent}to: {}", lines::quote(&connection.to)),
    ];
    if !connection.name.is_empty() {
        entry.push(format!("{key_indent}name: {}", lines::quote(&connection.name)));
    }
    document.insert(at, &entry)
}

/// Replaces a multi-line key's block, the key's line and every more deeply indented line after it within the range,
/// or writes it after the range when the entry has none.
fn replace_key_block(document: &mut LineDocument, range: LineRange, key: &str, new_lines: &[String]) -> EditOutcome {
    let Some(at) = lines::find_key(document, range, key) else {
        return document.insert(range.end + 1, new_lines);
    };
    let key_indent = lines::indent_of(&document.lines()[at].text).len();
    let mut end = at;
    while end < range.end && lines::indent_of(&document.lines()[end + 1].text).len() > key_indent {
        end += 1;
    }
    document.replace(LineRange::new(at, end), new_lines)
}

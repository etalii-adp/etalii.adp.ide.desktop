//! The property rows of a selected element or connection, and the command an edited row becomes (etalii.adp spec
//! 030, task T072).
//!
//! Ported from the standalone host at `8b6ccfc`: the module's `FdgContextPropertyProvider.cs`, with the ids the
//! client states once in `client/fdgIds.ts`. A Description for all five types and every connection; a Name for the
//! four named types and a connection; a Comment's Text instead of a Name, since a Comment has none. Width, and a
//! Comment's Height, are rows because the canvas resizes through them; a size that is not a positive number is
//! refused before any command exists, and every size is shown as the file holds it, to four decimals.

use std::path::Path;

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::{FdgCommand, FdgEdit};
use crate::lines::format_number;
use crate::model::FdgModel;
use crate::parser::parse_number;

/// An element's name, for the four named types.
pub const NAME: &str = "fdg.name";
/// A Comment's text.
pub const TEXT: &str = "fdg.text";
/// The description of any element or connection.
pub const DESCRIPTION: &str = "fdg.description";
/// An element's width.
pub const WIDTH: &str = "fdg.width";
/// A Comment's own height.
pub const HEIGHT: &str = "fdg.height";
/// A connection's name.
pub const CONNECTION_NAME: &str = "fdg.connection-name";

const IDENTITY: &str = "Identity";
const SIZE: &str = "Size";

fn row(id: &str, label: &str, value: &str, editor: PropertyEditor, group: &str) -> PropertyRow {
    PropertyRow {
        group: group.to_owned(),
        ..PropertyRow::new(id, label, editor, value)
    }
}

/// The rows of `id`, or none when it names nothing.
pub fn describe(model: &FdgModel, id: &str) -> Vec<PropertyRow> {
    if let Some(element) = model.element_of(id) {
        let mut rows = vec![if element.is_comment() {
            row(TEXT, "Text", &element.text, PropertyEditor::Text, IDENTITY)
        } else {
            row(NAME, "Name", &element.name, PropertyEditor::Line, IDENTITY)
        }];
        rows.push(row(
            DESCRIPTION,
            "Description",
            &element.description,
            PropertyEditor::Text,
            IDENTITY,
        ));
        rows.push(row(
            WIDTH,
            "Width",
            &format_number(element.width),
            PropertyEditor::Line,
            SIZE,
        ));
        if element.is_comment() {
            rows.push(row(
                HEIGHT,
                "Height",
                &format_number(element.drawn_height()),
                PropertyEditor::Line,
                SIZE,
            ));
        }
        return rows;
    }
    if let Some(connection) = model.connection_of(id) {
        return vec![
            row(
                CONNECTION_NAME,
                "Name",
                &connection.name,
                PropertyEditor::Line,
                IDENTITY,
            ),
            row(
                DESCRIPTION,
                "Description",
                &connection.description,
                PropertyEditor::Text,
                IDENTITY,
            ),
        ];
    }
    Vec::new()
}

/// The command setting property `property_id` of `id` to `value` becomes, or why it is refused.
pub fn change(
    model: &FdgModel,
    body_path: &Path,
    id: &str,
    property_id: &str,
    value: &str,
) -> Result<FdgCommand, String> {
    let element = model.element_of(id);
    let mut size = 0.0;
    if property_id == WIDTH || property_id == HEIGHT {
        match parse_number(value) {
            Some(parsed) if parsed > 0.0 => size = parsed,
            _ => return Err(format!("'{value}' is not a size this graph can draw.")),
        }
    }
    let edit = match (property_id, element) {
        (NAME | TEXT, Some(_)) => FdgEdit::Rename {
            element_id: id.to_owned(),
            value: value.to_owned(),
        },
        (DESCRIPTION, _) => FdgEdit::Describe {
            id: id.to_owned(),
            description: value.to_owned(),
        },
        (WIDTH, Some(_)) => FdgEdit::Size {
            element_id: id.to_owned(),
            width: size,
            height: None,
        },
        (HEIGHT, Some(element)) => FdgEdit::Size {
            element_id: id.to_owned(),
            width: element.width,
            height: Some(size),
        },
        (CONNECTION_NAME, _) => FdgEdit::RenameConnection {
            connection_id: id.to_owned(),
            name: value.to_owned(),
        },
        _ => return Err(format!("'{property_id}' cannot be edited on this selection.")),
    };
    Ok(FdgCommand::new(body_path, edit))
}

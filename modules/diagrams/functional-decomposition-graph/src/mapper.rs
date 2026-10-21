//! A `.fdg` model as the elements the shell draws, and the palette (etalii.adp spec 030, task T072).
//!
//! Ported from the standalone host at `8b6ccfc`: the module's `FdgElementMapper.cs` and `FdgToolboxProvider.cs`. What
//! is sent is the type, the element's centre (the document holds the top-left), and its name or text, width and
//! height; a description is never sent, so a change to one alone changes nothing drawn. What a rule would refuse is
//! still drawn; what cannot be drawn is left out: an entry of a type this notation does not have, a connection whose
//! end is not an element, and every later entry reusing an id already drawn. A view of a viewport holds every
//! element overlapping it and every connection whose span does, with that connection's two ends.

use std::collections::HashSet;

use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind, ToolboxItem};

use crate::actions::add_action_id;
use crate::lines::format_number;
use crate::model::{FdgConnection, FdgElement, FdgModel, connection_types, element_types};

/// What every element or connection type is prefixed with on the wire.
pub const PREFIX: &str = "etalii/functional-decomposition-graph+";

/// The wire type of an element or relation type.
pub fn wire_type(type_name: &str) -> String {
    format!("{PREFIX}{type_name}")
}

/// The document's own type word of a wire type, or none for a type this module does not send.
pub fn type_of_wire(wire: &str) -> Option<&str> {
    wire.strip_prefix(PREFIX)
}

fn rect_of(element: &FdgElement) -> Rect {
    Rect::new(element.x, element.y, element.width, element.drawn_height())
}

fn hull(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
}

/// The elements and connections that can be drawn: the first entry with an id, of a known type, with both ends.
fn drawable(model: &FdgModel) -> (Vec<&FdgElement>, Vec<&FdgConnection>) {
    let mut taken: HashSet<&str> = HashSet::new();
    let elements: Vec<&FdgElement> = model
        .elements
        .iter()
        .filter(|e| !e.id.is_empty() && element_types::is_known(&e.element_type) && taken.insert(&e.id))
        .collect();
    let ids: HashSet<&str> = elements.iter().map(|e| e.id.as_str()).collect();
    let connections = model
        .connections
        .iter()
        .filter(|c| {
            !c.id.is_empty()
                && connection_types::is_known(&c.connection_type)
                && ids.contains(c.from.as_str())
                && ids.contains(c.to.as_str())
                && taken.insert(&c.id)
        })
        .collect();
    (elements, connections)
}

/// The elements and connections a view of `viewport` holds, elements first.
pub fn visible(model: &FdgModel, viewport: DiagramViewport) -> Vec<DiagramElement> {
    let (elements, connections) = drawable(model);
    let by_id = |id: &str| elements.iter().find(|e| e.id == id).copied();
    let mut shown: HashSet<&str> = elements
        .iter()
        .filter(|e| viewport.intersects(rect_of(e)))
        .map(|e| e.id.as_str())
        .collect();
    let shown_connections: Vec<&FdgConnection> = connections
        .into_iter()
        .filter(|c| match (by_id(&c.from), by_id(&c.to)) {
            (Some(from), Some(to)) => viewport.intersects(hull(rect_of(from), rect_of(to))),
            _ => false,
        })
        .collect();
    for connection in &shown_connections {
        shown.insert(&connection.from);
        shown.insert(&connection.to);
    }
    elements
        .iter()
        .filter(|e| shown.contains(e.id.as_str()))
        .map(|e| element(e))
        .chain(shown_connections.iter().map(|c| connection(c)))
        .collect()
}

/// One element as drawn: at its centre, with its name or text and its size.
pub fn element(element: &FdgElement) -> DiagramElement {
    let height = element.drawn_height();
    DiagramElement {
        x: element.x + element.width / 2.0,
        y: element.y + height / 2.0,
        placed: true,
        attributes: vec![
            (
                "name".to_owned(),
                if element.is_comment() {
                    String::new()
                } else {
                    element.name.clone()
                },
            ),
            (
                "text".to_owned(),
                if element.is_comment() {
                    element.text.clone()
                } else {
                    String::new()
                },
            ),
            ("width".to_owned(), format_number(element.width)),
            ("height".to_owned(), format_number(height)),
        ],
        ..DiagramElement::node(element.id.clone(), wire_type(&element.element_type))
    }
}

/// One connection as drawn: it has no position of its own and follows its ends.
pub fn connection(connection: &FdgConnection) -> DiagramElement {
    DiagramElement {
        kind: ElementKind::Relation,
        source: Some(connection.from.clone()),
        target: Some(connection.to.clone()),
        attributes: vec![("name".to_owned(), connection.name.clone())],
        ..DiagramElement::node(connection.id.clone(), wire_type(&connection.connection_type))
    }
}

/// The palette: the five element types, each dropping its add action.
pub fn toolbox() -> Vec<ToolboxItem> {
    [
        (
            element_types::UI_ELEMENT,
            "UI element",
            "mdi-application-outline",
            "A screen, or a part of one. Drop it where it belongs; its parent is a connection drawn to it.",
        ),
        (
            element_types::ACTION,
            "Action",
            "mdi-gesture-tap",
            "Something the user does on a screen. A screen owns it.",
        ),
        (
            element_types::DATA_ELEMENT,
            "Data element",
            "mdi-database-outline",
            "Something the application keeps. A screen, an action or other data owns it.",
        ),
        (
            element_types::FUNCTION,
            "Function",
            "mdi-cog-outline",
            "Something that runs behind the screens. Anything but a Comment may own it.",
        ),
        (
            element_types::COMMENT,
            "Comment",
            "mdi-note-text-outline",
            "A note on the graph. It connects to nothing.",
        ),
    ]
    .into_iter()
    .map(|(element_type, label, icon, description)| ToolboxItem {
        id: format!("fdg.toolbox.{element_type}"),
        label: label.to_owned(),
        icon: icon.to_owned(),
        description: description.to_owned(),
        drop_action_id: add_action_id(element_type),
    })
    .collect()
}

//! ADP's default C4 palette, and a document's tag styles over it (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4Theme.cs` at `8b6ccfc`. C4 is
//! notation independent: the familiar blue and grey is a default a document may replace, keyed by tag, the kind's
//! implicit tag (`Person`, `Container`) included.

use crate::model::{C4Element, C4ElementKind, C4ElementStyle, same};

pub const PERSON_BACKGROUND: &str = "#08427b";
pub const SOFTWARE_SYSTEM_BACKGROUND: &str = "#1168bd";
const CONTAINER_BACKGROUND: &str = "#438dd5";
pub const COMPONENT_BACKGROUND: &str = "#85bbf0";
pub const EXTERNAL_BACKGROUND: &str = "#999999";
const DEPLOYMENT_NODE_BACKGROUND: &str = "#ffffff";
const LIGHT_TEXT: &str = "#ffffff";
pub const DARK_TEXT: &str = "#000000";

/// How one element is painted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct C4Style {
    pub background: String,
    pub color: String,
    pub shape: String,
    pub border: String,
}

/// The default style of `element`: muted when it is external, else its kind's colours.
pub fn style_for(element: &C4Element) -> C4Style {
    if element.is_external() {
        return C4Style {
            background: EXTERNAL_BACKGROUND.to_owned(),
            color: LIGHT_TEXT.to_owned(),
            shape: shape_of(element),
            border: String::new(),
        };
    }
    let (background, color) = match element.kind {
        C4ElementKind::Person => (PERSON_BACKGROUND, LIGHT_TEXT),
        C4ElementKind::SoftwareSystem | C4ElementKind::SoftwareSystemInstance => {
            (SOFTWARE_SYSTEM_BACKGROUND, LIGHT_TEXT)
        }
        C4ElementKind::Container | C4ElementKind::ContainerInstance => (CONTAINER_BACKGROUND, LIGHT_TEXT),
        C4ElementKind::Component => (COMPONENT_BACKGROUND, DARK_TEXT),
        C4ElementKind::DeploymentNode | C4ElementKind::InfrastructureNode => (DEPLOYMENT_NODE_BACKGROUND, DARK_TEXT),
    };
    C4Style {
        background: background.to_owned(),
        color: color.to_owned(),
        shape: shape_of(element),
        border: String::new(),
    }
}

/// The shape an element is drawn as: a person as a person, anything tagged `Database` as a cylinder, a node as a box,
/// the rest as a rounded box.
pub fn shape_of(element: &C4Element) -> String {
    if element.kind == C4ElementKind::Person {
        return "Person".to_owned();
    }
    if element.tags.iter().any(|tag| same(tag, "Database")) {
        return "Cylinder".to_owned();
    }
    match element.kind {
        C4ElementKind::DeploymentNode | C4ElementKind::InfrastructureNode => "Box".to_owned(),
        _ => "RoundedBox".to_owned(),
    }
}

/// `style` with every document style that names one of `element`'s tags, or its kind's tag, laid over it in the
/// document's order.
pub fn apply(style: &C4Style, element: &C4Element, document_styles: &[C4ElementStyle]) -> C4Style {
    let mut tags: Vec<&str> = element.tags.iter().map(String::as_str).collect();
    tags.push(kind_tag(element.kind));
    let mut result = style.clone();
    for candidate in document_styles
        .iter()
        .filter(|s| tags.iter().any(|tag| same(tag, &s.tag)))
    {
        let set = |slot: &mut String, value: &Option<String>| {
            if let Some(value) = value.as_ref().filter(|v| !v.is_empty()) {
                *slot = value.clone();
            }
        };
        set(&mut result.background, &candidate.background);
        set(&mut result.color, &candidate.color);
        set(&mut result.shape, &candidate.shape);
        set(&mut result.border, &candidate.border);
    }
    result
}

/// The tag every element of a kind carries implicitly, as the DSL's `element "Person" { ... }` reaches every person.
pub fn kind_tag(kind: C4ElementKind) -> &'static str {
    match kind {
        C4ElementKind::Person => "Person",
        C4ElementKind::SoftwareSystem | C4ElementKind::SoftwareSystemInstance => "Software System",
        C4ElementKind::Container | C4ElementKind::ContainerInstance => "Container",
        C4ElementKind::Component => "Component",
        C4ElementKind::DeploymentNode => "Deployment Node",
        C4ElementKind::InfrastructureNode => "Infrastructure Node",
    }
}

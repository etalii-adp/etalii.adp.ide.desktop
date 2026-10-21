//! What the property grid shows for a selected C4 element or relationship, and the command an edited row becomes
//! (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4ContextPropertyProvider.cs` at
//! `8b6ccfc`. The same commands the context actions use, reached a shorter way: "Rename…" and the Name row are one
//! edit and one undo entry. Only what C4 gives a technology to has a technology row; the kind, the identifier, the
//! tags and a relationship's ends are shown and not edited here.

use std::path::Path;

use adp_context::{PropertyEditor, PropertyRow};
use adp_diagram::DiagramDocuments;
use adp_history::Command;

use crate::actions::{GONE, resolve};
use crate::commands::{
    SetElementDescription, SetElementName, SetElementTechnology, SetRelationshipDescription, SetRelationshipTechnology,
};
use crate::model::{C4Element, C4ElementKind, C4Relationship, C4Workspace};

pub const NAME: &str = "c4.name";
pub const DESCRIPTION: &str = "c4.description";
pub const TECHNOLOGY: &str = "c4.technology";
pub const TAGS: &str = "c4.tags";
pub const KIND: &str = "c4.kind";
pub const IDENTIFIER: &str = "c4.identifier";
pub const RELATIONSHIP_DESCRIPTION: &str = "c4.relationship.description";
pub const RELATIONSHIP_TECHNOLOGY: &str = "c4.relationship.technology";
pub const RELATIONSHIP_SOURCE: &str = "c4.relationship.source";
pub const RELATIONSHIP_DESTINATION: &str = "c4.relationship.destination";

/// Why the structural facts are shown without being editable.
pub const STRUCTURE_REASON: &str = "Changing this changes the shape of the model rather than a value, so it is done from the diagram rather than here.";

/// Why a relationship's ends are shown without being editable.
pub const ENDPOINT_REASON: &str =
    "The ends of a relationship are what it is. Draw a different relationship to connect something else.";

/// Why the tags are shown without being editable.
pub const TAGS_REASON: &str = "Tags are edited in the document until an action exists for them.";

fn row(id: &str, label: &str, value: &str, editor: PropertyEditor) -> PropertyRow {
    PropertyRow::new(id, label, editor, value)
}

fn read_only(id: &str, label: &str, value: &str, reason: &str, group: &str) -> PropertyRow {
    PropertyRow {
        read_only_reason: reason.to_owned(),
        group: group.to_owned(),
        ..row(id, label, value, PropertyEditor::Line)
    }
}

/// The rows of `element_id`, or none when it names nothing.
pub fn describe(workspace: &C4Workspace, element_id: &str) -> Vec<PropertyRow> {
    match resolve(workspace, element_id) {
        (Some(element), _) => describe_element(element),
        (_, Some(relationship)) => describe_relationship(workspace, relationship),
        _ => Vec::new(),
    }
}

fn describe_element(element: &C4Element) -> Vec<PropertyRow> {
    let mut rows = vec![
        row(NAME, "Name", &element.name, PropertyEditor::Line),
        row(DESCRIPTION, "Description", &element.description, PropertyEditor::Text),
    ];
    if element.kind.has_technology() {
        rows.push(row(TECHNOLOGY, "Technology", &element.technology, PropertyEditor::Line));
    }
    rows.push(read_only(TAGS, "Tags", &element.tags.join(", "), TAGS_REASON, ""));
    rows.push(read_only(KIND, "Kind", &spell(element.kind), STRUCTURE_REASON, "Model"));
    rows.push(read_only(
        IDENTIFIER,
        "Identifier",
        &element.id,
        STRUCTURE_REASON,
        "Model",
    ));
    rows
}

fn describe_relationship(workspace: &C4Workspace, relationship: &C4Relationship) -> Vec<PropertyRow> {
    vec![
        row(
            RELATIONSHIP_DESCRIPTION,
            "Description",
            &relationship.description,
            PropertyEditor::Line,
        ),
        row(
            RELATIONSHIP_TECHNOLOGY,
            "Technology",
            &relationship.technology,
            PropertyEditor::Line,
        ),
        read_only(
            RELATIONSHIP_SOURCE,
            "From",
            &name_of(workspace, &relationship.source_id),
            ENDPOINT_REASON,
            "Model",
        ),
        read_only(
            RELATIONSHIP_DESTINATION,
            "To",
            &name_of(workspace, &relationship.destination_id),
            ENDPOINT_REASON,
            "Model",
        ),
    ]
}

/// The command a new `value` of row `property_id` of `element_id` becomes, or why it cannot be set.
pub fn change(
    workspace: &C4Workspace,
    body_path: &Path,
    element_id: &str,
    property_id: &str,
    value: &str,
) -> Result<Box<dyn Command<DiagramDocuments>>, String> {
    let (element, relationship) = resolve(workspace, element_id);
    if element.is_none() && relationship.is_none() {
        return Err(GONE.to_owned());
    }
    let body_path = body_path.to_path_buf();
    let value = value.to_owned();
    Ok(match (property_id, element, relationship) {
        (NAME, Some(element), _) => Box::new(SetElementName {
            body_path,
            element_id: element.id.clone(),
            name: value,
        }),
        (DESCRIPTION, Some(element), _) => Box::new(SetElementDescription {
            body_path,
            element_id: element.id.clone(),
            description: value,
        }),
        (TECHNOLOGY, Some(element), _) => Box::new(SetElementTechnology {
            body_path,
            element_id: element.id.clone(),
            technology: value,
        }),
        (RELATIONSHIP_DESCRIPTION, _, Some(relationship)) => Box::new(SetRelationshipDescription {
            body_path,
            relationship_id: relationship.id(),
            description: value,
        }),
        (RELATIONSHIP_TECHNOLOGY, _, Some(relationship)) => Box::new(SetRelationshipTechnology {
            body_path,
            relationship_id: relationship.id(),
            technology: value,
        }),
        _ => return Err(format!("'{property_id}' is not a property of what is selected.")),
    })
}

/// The element's name where the model has one, and the identifier where it does not.
fn name_of(workspace: &C4Workspace, id: &str) -> String {
    match workspace.find(id) {
        Some(found) if !found.name.is_empty() => found.name.clone(),
        _ => id.to_owned(),
    }
}

fn spell(kind: C4ElementKind) -> String {
    match kind {
        C4ElementKind::SoftwareSystem => "Software system".to_owned(),
        C4ElementKind::DeploymentNode => "Deployment node".to_owned(),
        C4ElementKind::InfrastructureNode => "Infrastructure node".to_owned(),
        C4ElementKind::ContainerInstance => "Container instance".to_owned(),
        C4ElementKind::SoftwareSystemInstance => "Software system instance".to_owned(),
        _ => kind.name().to_owned(),
    }
}

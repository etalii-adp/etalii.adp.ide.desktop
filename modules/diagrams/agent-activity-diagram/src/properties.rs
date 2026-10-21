//! The property grid of an element or a row: its type's attributes, each written through a command (etalii.adp spec
//! 030, task T075). A relation shows nothing; it is its two ends.
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadProviders.cs` (`AadContextPropertyProvider`) at `8b6ccfc`, with the rows of `AadDefinition.Rows`
//! ([`crate::definition::AadDefinition::rows`]).

use std::path::Path;

use adp_context::PropertyRow;

use crate::commands::{AadCommand, AadEdit};
use crate::definition::{self, AadDefinition};
use crate::store::AadDocument;

/// The rows of `element_id`, or none for a relation or an id the file does not have.
pub fn describe(document: &AadDocument, element_id: &str) -> Vec<PropertyRow> {
    match AadDefinition::element_of(&document.body.typed().diagram, element_id) {
        Some(element) if !element.is_relation() => definition::get().rows(&element, !document.is_usable()),
        _ => Vec::new(),
    }
}

/// The command that sets row `property_id` of `element_id` to `value`, or why the row cannot be set.
pub fn change(
    document: &AadDocument,
    body_path: &Path,
    element_id: &str,
    property_id: &str,
    value: &str,
) -> Result<AadCommand, String> {
    if let Some(row) = describe(document, element_id).iter().find(|r| r.id == property_id)
        && row.is_read_only()
    {
        return Err(row.read_only_reason.clone());
    }
    Ok(AadCommand::new(
        body_path,
        AadEdit::SetAttribute {
            element_id: element_id.to_owned(),
            attribute: property_id.to_owned(),
            value: value.to_owned(),
        },
    ))
}

//! The property rows of a selected element of the family (etalii.adp spec 030, task T083), ported from the standalone
//! host's `RdfContextPropertyProvider.cs` at `8b6ccfc`. STUB: filled in by the family port.

use adp_context::PropertyRow;

use crate::commands::RdfEdit;
use crate::family::Target;
use crate::store::RdfDocument;

/// The rows of `target` (`DescribeAsync`).
pub fn describe(document: &RdfDocument, target: &Target) -> Vec<PropertyRow> {
    let _ = (document, target);
    Vec::new()
}

/// The edit setting `property_id` of `target` to `value` makes, or why none (`SetAsync`).
pub fn command_for(document: &RdfDocument, target: &Target, property_id: &str, value: &str) -> Result<RdfEdit, String> {
    let _ = (document, target, value);
    Err(format!("'{property_id}' cannot be edited on this selection."))
}

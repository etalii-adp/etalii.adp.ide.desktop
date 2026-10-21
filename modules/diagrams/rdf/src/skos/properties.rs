//! The skos reading's property rows. STUB: filled in by the skos port.

use adp_context::PropertyRow;

use crate::commands::RdfEdit;
use crate::family::Target;
use crate::store::RdfDocument;

/// The grid of `target` when this reading claims it, none otherwise (`Describe`).
pub fn describe(document: &RdfDocument, target: &Target) -> Option<Vec<PropertyRow>> {
    let _ = (document, target);
    None
}

/// The edit setting `property_id` makes when the row is this reading's, none otherwise (`CommandFor`).
pub fn command_for(document: &RdfDocument, target: &Target, property_id: &str, value: &str) -> Option<RdfEdit> {
    let _ = (document, target, property_id, value);
    None
}

//! What can be done to a selected element of the family (etalii.adp spec 030, task T083), ported from the standalone
//! host's `RdfContextActionProvider.cs` at `8b6ccfc`. STUB: filled in by the family port.

use adp_context::ActionGroup;

use crate::commands::RdfEdit;
use crate::family::{Run, Target};
use crate::model::RdfModel;
use crate::store::RdfDocument;

/// Rename a resource, rewriting every reference with it.
pub const RENAME_RESOURCE: &str = "rdf.rename-resource";
/// Remove a resource and every triple touching it, saying how many first.
pub const REMOVE_RESOURCE: &str = "rdf.remove-resource";
/// Remove a selected edge's triple.
pub const REMOVE_EDGE: &str = "rdf.remove-edge";
/// The relation gesture: relate two drawn resources, asking for the predicate.
pub const CONNECT: &str = "rdf.connect";
/// Add a resource at a placement, asking for its name.
pub const ADD_RESOURCE: &str = "rdf.add-resource";
/// Declare a prefix, asking for `prefix: iri` in one line.
pub const ADD_PREFIX: &str = "rdf.add-prefix";
/// The subclass gesture: one `rdfs:subClassOf` splice, a duplicate refused first.
pub const SUBCLASS: &str = "owl.subclass";
/// Add a class at a placement: one `a owl:Class` declaration.
pub const ADD_CLASS: &str = "owl.add-class";
/// Add an object property at a placement.
pub const ADD_OBJECT_PROPERTY: &str = "owl.add-object-property";
/// Add a datatype property at a placement.
pub const ADD_DATATYPE_PROPERTY: &str = "owl.add-datatype-property";
/// Add a named individual at a placement.
pub const ADD_INDIVIDUAL: &str = "owl.add-individual";

/// The text an element id resolves to (`RdfContextSourceResolver.Describe`).
pub fn describe(model: &RdfModel, element_id: &str) -> Option<String> {
    let _ = (model, element_id);
    None
}

/// The menu of `target` (`DiscoverAsync`).
pub fn discover(document: &RdfDocument, target: &Target) -> Vec<ActionGroup> {
    let _ = (document, target);
    Vec::new()
}

/// Runs `action_id` on `target` (`ExecuteAsync`).
pub fn execute(document: &RdfDocument, target: &Target, action_id: &str) -> Run {
    let _ = (document, target, action_id);
    Run::Done
}

/// Whether `value` may answer the prompt of `action_id` (`ValidateAsync`).
pub fn validate(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Result<(), String> {
    let _ = (document, target, action_id, value);
    Ok(())
}

/// The edit an answer `value` to the prompt of `action_id` makes (`CommitAsync`).
pub fn commit(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Result<RdfEdit, String> {
    let _ = (document, target, value);
    Err(format!("'{action_id}' does not apply to this selection."))
}

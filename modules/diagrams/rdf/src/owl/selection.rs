//! What the ontology reading's element ids name (`OwlSelection.cs` at `8b6ccfc`). STUB: filled in by the OWL port.

use crate::model::RdfModel;

/// What a subclass gesture between two classes already related answers (`OwlSelection.DuplicateSubclassRefusal`).
pub const DUPLICATE_SUBCLASS_REFUSAL: &str = "";

/// Why an expression node reads rather than edits (`OwlSelection.ExpressionRefusal`).
pub const EXPRESSION_REFUSAL: &str = "";

/// The IRI of the named individual an `ind:` id names, when the model states it (`IndividualIriOf`).
pub fn individual_iri_of(model: &RdfModel, element_id: &str) -> Option<String> {
    let _ = (model, element_id);
    None
}

/// The text an ontology element id resolves to (`OwlSelection.Describe`).
pub fn describe(model: &RdfModel, element_id: &str) -> Option<String> {
    let _ = (model, element_id);
    None
}

/// The full, uncapped Manchester rendering of the expression an `expr:` id names (`ExpressionOf` rendered by
/// `ExpressionRenderer.Render`).
pub fn expression_text(model: &RdfModel, element_id: &str) -> Option<String> {
    let _ = (model, element_id);
    None
}

/// The uncapped Manchester rendering of the expression rooted at the blank node `root` (`ExpressionRenderer.Render`).
pub fn render_expression(model: &RdfModel, root: &crate::model::BlankTerm) -> String {
    let _ = (model, root);
    String::new()
}

/// The property IRI a derived property edge's id names (`PropertyEdgeIriOf`).
pub fn property_edge_iri_of(model: &RdfModel, element_id: &str) -> Option<String> {
    let _ = (model, element_id);
    None
}

/// Whether the model states `iri` to be a class (`IsClass`).
pub fn is_class(model: &RdfModel, iri: &str) -> bool {
    let _ = (model, iri);
    false
}

/// Whether the document carries the `owl:Ontology` marker (`IsOntologyDocument`).
pub fn is_ontology_document(model: &RdfModel) -> bool {
    let _ = model;
    false
}

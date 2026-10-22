//! What one reading of the family draws of a document, and what it refuses to arrange (etalii.adp spec 030, task
//! T083): the part of a view that differs between the data graph, the ontology, the scheme and the shapes reading.
//! [`crate::view::RdfView`] is the rest, written once.
//!
//! Ported from the standalone host's `IRdfReading.cs`, `RdfReadings.cs` and `RdfViewport.cs` at `8b6ccfc`, with the
//! core's `RegistrationLayout.Apply`. A reading also answers here for its palette and its findings, which the
//! standalone host registered beside it per origin (`IDiagramToolboxProvider`, `IDiagramValidator`).

use std::collections::BTreeMap;
use std::path::Path;

use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramProblem, DiagramViewport, ToolboxItem};

use crate::model::RdfModel;
use crate::store::RdfDocument;

/// Where every element sits, by its id: the top-left corner of its box in canvas units (`RegistrationPosition`).
pub type Positions = BTreeMap<String, (f64, f64)>;

/// One reading of the family (`IRdfReading<TProjection>`).
pub trait Reading {
    /// What the reading projects a document's model into.
    type Projection;

    /// The tool type it is (`w3c/rdf`, `w3c/owl`, `w3c/skos`, `w3c/shacl`).
    fn origin(&self) -> &'static str;

    /// What a drag onto another parent answers: in this reading's words, why nothing nests.
    fn parent_refusal(&self) -> &'static str;

    /// The whole of `model`, projected without a budget, so everything the reader can pan to has a place.
    fn project(&self, model: &RdfModel) -> Self::Projection;

    /// Where every element of `projection` sits: the reading's layout, with the registration's `stored` positions on
    /// top (see [`apply`]).
    fn positions(&self, projection: &Self::Projection, stored: &Positions) -> Positions;

    /// What `viewport` admits of `projection`: the elements it intersects, the edges both of whose ends survived, and
    /// the budget applied to that set. The counts stay the document's.
    fn in_view(
        &self,
        projection: &Self::Projection,
        positions: &Positions,
        viewport: DiagramViewport,
    ) -> Self::Projection;

    /// The elements the canvas is sent for `projection`, in the order the standalone host sent them.
    fn elements(&self, projection: &Self::Projection, positions: &Positions) -> Vec<DiagramElement>;

    /// Why `element_id` cannot be given a stored position, or none when it can.
    fn refusal(&self, element_id: &str) -> Option<String>;

    /// The palette (`IDiagramToolboxProvider` of this origin).
    fn toolbox(&self) -> Vec<ToolboxItem>;

    /// The findings of `document` (`IDiagramValidator` of this origin), given the registration it was opened with.
    fn problems(&self, document: &RdfDocument, registration: Option<&Path>) -> Vec<DiagramProblem>;

    /// The action a connect gesture drawn on this reading's canvas runs on its `rel:` target (the canvas's
    /// `onConnectionDrawn`); none for a reading whose canvas draws no connection.
    fn connect_action(&self) -> Option<&'static str> {
        None
    }
}

/// The computed positions with the stored ones on top, element by element; a stored position of an element the
/// projection does not draw is ignored (`RegistrationLayout.Apply`).
pub fn apply(computed: Positions, stored: &Positions) -> Positions {
    let mut merged = computed;
    for (id, position) in stored {
        if let Some(slot) = merged.get_mut(id) {
            *slot = *position;
        }
    }
    merged
}

/// The element `reading` draws for `element_id` in `model`, or none when it draws none: its type and payload are what
/// the canvas was sent, though not its place, which no stored position moves here (`RdfReadings.Drawn`).
pub fn drawn<R: Reading>(reading: &R, model: &RdfModel, element_id: &str) -> Option<DiagramElement> {
    let projection = reading.project(model);
    let positions = reading.positions(&projection, &Positions::new());
    reading
        .elements(&projection, &positions)
        .into_iter()
        .find(|element| element.id == element_id)
}

/// Whether the node `id`, in a cell of `width` by `height` at its position, is on screen; a node nothing placed is
/// admitted, since no position is no evidence it is off screen (`RdfViewport.Admits`).
pub fn admits(viewport: DiagramViewport, positions: &Positions, id: &str, width: f64, height: f64) -> bool {
    positions
        .get(id)
        .is_none_or(|&(x, y)| viewport.intersects(Rect { x, y, width, height }))
}

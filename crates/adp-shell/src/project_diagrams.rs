//! The project's diagrams as the knowledge designer's relations see them (etalii.adp spec 092, FR-025): which of them a
//! relation may point at, each with its named elements and the kinds of element its *Create* may add, and the body a
//! diagram has with one added - read and planned through the diagram tool types the canvas opens them with, so an
//! element added from a table is the one the diagram's own tool would add.

use std::cell::RefCell;
use std::path::Path;

use adp_designer_knowledge::diagrams::{CreatedElement, ElementKind, KnowledgeDiagram, KnowledgeDiagrams};
use adp_diagram::DiagramTools;
use adp_diagram::element_creation;

use crate::canvas::{DiagramHost, vendor_folder};

/// The diagram tool types of the vendored catalogue, loaded once for the tables of one tab.
pub struct ProjectDiagrams {
    tools: RefCell<DiagramTools>,
}

impl ProjectDiagrams {
    /// The diagram tool types the canvas opens, from the vendored catalogue at `vendor`.
    pub fn with_vendor(vendor: &Path) -> Self {
        ProjectDiagrams {
            tools: RefCell::new(DiagramHost::with_vendor(vendor).tools),
        }
    }

    /// [`ProjectDiagrams::with_vendor`] of the program's own vendored catalogue.
    pub fn new() -> Self {
        ProjectDiagrams::with_vendor(&vendor_folder())
    }
}

impl Default for ProjectDiagrams {
    fn default() -> Self {
        ProjectDiagrams::new()
    }
}

impl KnowledgeDiagrams for ProjectDiagrams {
    fn diagrams(&self, folder: &Path) -> Vec<KnowledgeDiagram> {
        element_creation::diagrams_under(&mut self.tools.borrow_mut(), folder)
            .into_iter()
            .map(|diagram| KnowledgeDiagram {
                path: diagram.path,
                indexed: diagram.indexed,
                kinds: diagram
                    .kinds
                    .into_iter()
                    .map(|k| ElementKind::new(k.type_name, k.label))
                    .collect(),
            })
            .collect()
    }

    fn create(&self, path: &Path, kind: &str, name: &str) -> Result<CreatedElement, String> {
        element_creation::create_in(&mut self.tools.borrow_mut(), path, kind, name).map(|created| CreatedElement {
            id: created.id,
            body: created.body,
        })
    }
}

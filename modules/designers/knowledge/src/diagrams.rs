//! A knowledge relation whose target is a diagram (etalii.adp spec 092, FR-025; `knowledge.md`, "A relation to a
//! diagram"): its target may be a document of any tool type whose definition declares a linkable element type - a
//! timeline, a dependency graph, a mind map - as well as a knowledge file. Its cells then hold links to that document's
//! elements, named by the name property their type declares, and the picker searches those elements by name.
//!
//! This crate knows no diagram tool type: what the workspace's diagrams are, and the body a diagram has with an element
//! added by name, are asked of [`KnowledgeDiagrams`], which the host gives the session. A diagram is then seen as a
//! table of one title property, named after the diagram's name property, whose rows are its named elements: the
//! picker, the labels of the cells and the links they are written as are a knowledge target's.

use std::path::{Path, PathBuf};

use adp_desl::table::content::{ContentCell, ContentRow};
use adp_links::IndexedDocument;

use crate::model::{self, KnowledgeTable};

/// The setting of a `setCell` that asks for an element named so to be added to the relation's target diagram and
/// related in the cell, in the same step (*Create*).
pub const NEW_ELEMENT: &str = "newElement";

/// The setting of that `setCell` that names the kind of element, by its type, where the diagram offers several.
pub const ELEMENT_KIND: &str = "elementKind";

/// Why *Create* adds nothing yet: the diagram offers several kinds and none was picked.
pub const PICK_A_KIND: &str = "Pick which kind of element to add.";

/// The id of the one property a diagram is seen with: what names its elements.
pub const NAME_PROPERTY_ID: &str = "name";

/// Why a relation to a diagram is not made two-way: a diagram has no column to show the rows that link to it in.
pub const ONE_WAY_ONLY: &str = "A relation to a diagram is shown in this table only: a diagram has no column to show it in. Its elements list the rows that link to them under Linked from.";

/// Why *Create* adds nothing to a relation's target: it is a diagram its tool gives no element by name.
pub const NOTHING_TO_CREATE: &str = "This diagram is given no element from a table.";

/// Why *Create* adds nothing: the relation does not point at a diagram that can be read.
pub const NO_DIAGRAM: &str = "This relation does not point at a diagram that can be read.";

/// A kind of element *Create* may add to a diagram: the type it creates and the word a person picks it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementKind {
    pub type_name: String,
    pub label: String,
}

impl ElementKind {
    pub fn new(type_name: impl Into<String>, label: impl Into<String>) -> Self {
        ElementKind {
            type_name: type_name.into(),
            label: label.into(),
        }
    }
}

/// A diagram of the workspace as a relation's target sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeDiagram {
    pub path: PathBuf,
    /// The diagram as the workspace's index holds it (FBL section 17.3), its elements those of a linkable type that
    /// hold no link of their own: a linked element stands for another document's element and is not offered.
    pub indexed: IndexedDocument,
    /// What *Create* may add to it, in its tool's order; none for a diagram given no element from elsewhere.
    pub kinds: Vec<ElementKind>,
}

/// An element added to a diagram by name: the id it was given and the diagram's body that holds it, not yet written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedElement {
    pub id: String,
    pub body: Vec<u8>,
}

/// What the host knows of the workspace's diagrams, for a knowledge relation whose target is one.
pub trait KnowledgeDiagrams {
    /// The diagrams under `folder` whose tool type declares a linkable element type.
    fn diagrams(&self, folder: &Path) -> Vec<KnowledgeDiagram>;

    /// The body of the diagram at `path` with an element of the type `kind` names added, named `name`, where its tool
    /// places an element added without a position; or why it is not.
    fn create(&self, path: &Path, kind: &str, name: &str) -> Result<CreatedElement, String>;
}

/// The kinds a relation column's `creates` setting names ([`crate::mapper::CREATES`]), in its order: none for a
/// column whose *Create* adds nothing to another file.
pub fn kinds_of(setting: &str) -> Vec<ElementKind> {
    setting
        .split(',')
        .filter_map(|kind| kind.split_once('='))
        .filter(|(type_name, _)| !type_name.trim().is_empty())
        .map(|(type_name, label)| ElementKind::new(type_name.trim(), label.trim()))
        .collect()
}

/// The name an element of `diagram` is offered and linked by.
fn name_of(diagram: &KnowledgeDiagram, element: &adp_links::IndexedElement) -> String {
    match &diagram.indexed.name_property {
        Some(property) => element.attribute(property).unwrap_or_default().to_owned(),
        None => element.attributes.first().map(|(_, v)| v.clone()).unwrap_or_default(),
    }
}

/// A diagram as a table: one title property, named after the diagram's name property, and a row per named element.
pub fn table_of(diagram: &KnowledgeDiagram) -> KnowledgeTable {
    let name_property = diagram
        .indexed
        .name_property
        .clone()
        .unwrap_or_else(|| NAME_PROPERTY_ID.to_owned());
    KnowledgeTable {
        document_id: diagram.indexed.document_id.clone().unwrap_or_default(),
        name: file_name(&diagram.path),
        columns: vec![model::property(
            NAME_PROPERTY_ID,
            &name_property,
            "text",
            true,
            Vec::new(),
            "",
            "",
            "",
            false,
            false,
        )],
        rows: diagram
            .indexed
            .elements
            .iter()
            .map(|element| ContentRow::new(element.id.clone(), vec![named(&name_of(diagram, element))]))
            .filter(|row| row.cells.iter().any(|c| c.values.iter().any(|v| !v.is_empty())))
            .collect(),
        ..KnowledgeTable::default()
    }
}

/// `table` with a row for an element just added by name, so that it is offered and linked before the diagram is read
/// again.
pub fn with_row(mut table: KnowledgeTable, id: &str, name: &str) -> KnowledgeTable {
    table.rows.push(ContentRow::new(id, vec![named(name)]));
    table
}

fn named(name: &str) -> ContentCell {
    ContentCell::new(NAME_PROPERTY_ID, [name], "text")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

//! Links between documents for every diagram type whose specification declares them (etalii.adp spec 089; DISL
//! section 4.12, FBL section 17): what a diagram adds to the workspace's index, the one edit its load-time validation
//! makes, the findings of the links that do not resolve, and what a linked element's `self.linked` reads.
//!
//! Nothing here names a tool type: an element type is linkable, stands for a linked element or holds links as its
//! specification says, and the document id is where its binding, or its plugin, keeps it.

use std::path::Path;

use adp_cel::{Map, Value};
use adp_disl::{DislChange, DislElement, DislSpecification};
use adp_fbl::ModelChange;
use adp_links::{HeldLink, IndexedDocument, IndexedElement, Link, Resolution, Workspace, codes};

use crate::document::{DiagramChange, DiagramDocument, text_of};
use crate::problems::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

/// The label of the edit load-time validation makes (FBL section 17.4).
pub const CHECK_LINKS: &str = "Check links";

/// The rule `persistence.documentId` gives a new document id by (DISL section 11.5): its alphabet and length; none
/// for a diagram type whose documents hold no id, and so no links.
pub fn document_id_rule(specification: &DislSpecification) -> Option<(String, usize)> {
    let rule = specification.root().get("persistence")?.get("documentId")?;
    let alphabet = match rule.get("alphabet").and_then(|a| a.as_str()) {
        None | Some("base36") => adp_links::BASE36.to_owned(),
        Some("alphanumeric") => "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz".to_owned(),
        Some(other) => other.to_owned(),
    };
    let length = rule.get("length").and_then(|l| l.as_u64()).unwrap_or(8) as usize;
    Some((alphabet, length))
}

/// Whether documents of this diagram type take part in links (DISL section 4.12: only a type whose persistence
/// declares `documentId`).
pub fn takes_part(document: &DiagramDocument) -> bool {
    document_id_rule(document.tool().definition.specification()).is_some()
}

/// A new document id for `specification`'s documents.
pub fn new_document_id(specification: &DislSpecification) -> Option<String> {
    let (alphabet, length) = document_id_rule(specification)?;
    Some(adp_links::nanoid(&alphabet, length))
}

/// The id the file writes for `element`: its specification's `persistence.writtenId` host attribute, else its id.
pub(crate) fn written_id(document: &DiagramDocument, element: &DislElement) -> String {
    let written = document.tool().definition.specification().root()["persistence"]["writtenId"]
        .as_str()
        .and_then(|attribute| match element.host_attributes().get(attribute) {
            Some(Value::String(written)) if !written.is_empty() => Some(written.clone()),
            _ => None,
        });
    written.unwrap_or_else(|| {
        let id = element.id();
        document
            .fbl()
            .written_ids
            .iter()
            .find(|(addressed, _)| *addressed == id)
            .map_or(id, |(_, written)| written.clone())
    })
}

/// The links `element` holds, each with its attribute: every attribute of type `link` with a value.
fn links_of(element: &DislElement) -> Vec<(String, String)> {
    let diagram = element.diagram();
    let specification = diagram.specification();
    specification
        .metamodel()
        .link_attributes(&element.type_name())
        .into_iter()
        .filter_map(|attribute| {
            let value = element.attributes().get(&attribute).cloned()?;
            let text = text_of(&value);
            (!value.is_null() && !text.is_empty()).then_some((attribute, text))
        })
        .collect()
}

/// The diagram as the workspace's index holds it (FBL section 17.3): its path and document id, each element of a
/// linkable type by the id its file writes with its text attributes, the name property first, and every link it
/// holds; none for a diagram type that takes no part in links.
pub fn indexed(document: &DiagramDocument) -> Option<IndexedDocument> {
    if !takes_part(document) {
        return None;
    }
    let diagram = &document.model().diagram;
    let specification = document.tool().definition.specification();
    let metamodel = specification.metamodel();
    let mut indexed = IndexedDocument {
        path: document.body_path().to_path_buf(),
        document_id: document.fbl().document_id.clone(),
        ..IndexedDocument::default()
    };
    for element in diagram.nodes() {
        let type_name = element.type_name();
        if let Some(name_property) = metamodel.linkable(&type_name) {
            if indexed.name_property.is_none() && !name_property.is_empty() {
                indexed.name_property = Some(name_property.clone());
            }
            let attributes = element.attributes();
            let mut texts: Vec<(String, String)> = Vec::new();
            if let Some(value) = attributes.get(&name_property) {
                texts.push((name_property.clone(), text_of(value)));
            }
            if let Some(known) = metamodel.type_of(&type_name) {
                for attribute in known.attributes.values() {
                    if attribute.name == name_property
                        || attribute.many
                        || !matches!(attribute.type_name.as_str(), "string" | "text")
                    {
                        continue;
                    }
                    if let Some(value) = attributes.get(&attribute.name).filter(|v| !v.is_null()) {
                        texts.push((attribute.name.clone(), text_of(value)));
                    }
                }
            }
            indexed
                .elements
                .push(IndexedElement::new(written_id(document, &element), texts));
        }
        for (_, link) in links_of(&element) {
            indexed.links.push(HeldLink::new(element.id(), link));
        }
    }
    Some(indexed)
}

/// What load-time validation changes in `document` (FBL section 17.4), as one edit: a document id when its body
/// holds none and its binding can keep one, and every link that resolved and reads otherwise than resolution found.
/// Empty when nothing differs.
pub fn load_changes(document: &DiagramDocument, index: &Workspace) -> Vec<DiagramChange> {
    let specification = document.tool().definition.specification();
    let Some(new_id) = new_document_id(specification) else {
        return Vec::new();
    };
    let mut changes = Vec::new();
    let binding = &document.tool().binding;
    if document.fbl().document_id.is_none() && (binding.document_id.is_some() || binding.plugin.is_some()) {
        changes.push(DiagramChange::Body(ModelChange::SetDocumentId { id: new_id }));
    }
    for element in document.model().diagram.nodes() {
        let mut attributes = Map::default();
        for (attribute, link) in links_of(&element) {
            if let Resolution::Resolved(resolved) = index.resolve(&link) {
                let refreshed = resolved.refreshed.text();
                if refreshed != link {
                    attributes.insert(attribute, Value::String(refreshed));
                }
            }
        }
        if !attributes.is_empty() {
            changes.push(DiagramChange::Disl(DislChange::Set {
                element_id: element.id(),
                type_name: element.type_name(),
                attributes,
            }));
        }
    }
    changes
}

/// The findings of the links `document` holds that do not resolve (FBL section 17.6), each at the element that holds
/// it.
pub fn problems(document: &DiagramDocument, index: &Workspace) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    for element in document.model().diagram.nodes() {
        for (_, link) in links_of(&element) {
            if let Resolution::Finding(finding) = index.resolve(&link) {
                problems.push(DiagramProblem {
                    severity: if finding.code == codes::MALFORMED {
                        DiagramProblemSeverity::Error
                    } else {
                        DiagramProblemSeverity::Warning
                    },
                    message: finding.message,
                    rule_id: finding.code.to_owned(),
                    location: Some(DiagramProblemLocation::Element(element.id())),
                });
            }
        }
    }
    problems
}

/// Binds `self.linked` of every element of a `linked` type in `document` (DISL section 4.12): the target's `shows`
/// values while its link resolves, nothing while it does not or when there is no index to resolve it in.
pub fn bind_linked(document: &DiagramDocument, index: Option<&Workspace>) {
    let diagram = &document.model().diagram;
    let metamodel = document.tool().definition.specification().metamodel();
    for element in diagram.nodes() {
        let Some((attribute, shows)) = metamodel.linked(&element.type_name()) else {
            continue;
        };
        let link = element.attributes().get(&attribute).map(text_of).unwrap_or_default();
        let resolved = index.and_then(|index| match index.resolve(&link) {
            Resolution::Resolved(resolved) => Some(resolved),
            Resolution::Finding(_) => None,
        });
        // Its label is the target's current name, else the name its link caches (DISL section 4.12).
        let name = resolved
            .as_ref()
            .and_then(|r| r.refreshed.cached_name().map(str::to_owned))
            .or_else(|| Link::parse(&link).and_then(|l| l.cached_name().map(str::to_owned)));
        let property = metamodel.linkable(&element.type_name());
        element.set_linked_name(property.zip(name));
        element.set_linked(resolved.map(|resolved| {
            let mut values = Map::default();
            for shown in &shows {
                if let Some(value) = resolved.element.as_ref().and_then(|e| e.attribute(shown)) {
                    values.insert(shown.clone(), Value::String(value.to_owned()));
                }
            }
            values
        }));
    }
}

/// Whether `path` names a file the index may hold: none under a folder whose name starts with a dot, or that holds
/// what a build or a package manager writes.
pub fn is_indexed_folder(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_none_or(|n| !n.starts_with('.') && !matches!(n, "node_modules" | "target" | "bin" | "obj"))
}

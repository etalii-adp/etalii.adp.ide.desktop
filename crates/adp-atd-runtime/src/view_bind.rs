//! View data kept in the body (DISL 0.4, section 11.6, `persistence.view.bind`): a pinned element's position as an
//! entry of the type `bind.pinned` names, and a change of it planned as the add, set or remove of that entry (FBL
//! section 6.4), in the same edit as the gesture. An entry for an element that does not exist, or a second entry for
//! the same element, is ignored.

use std::collections::HashMap;

use adp_cel::{Map, Value};
use adp_diagram::DiagramDocument;
use adp_fbl::model::FblElement;
use adp_fbl::planning::ModelChange;
use serde_json::Value as Json;

/// Where a pinned position is kept: the binding type of its entries, and the attributes naming the element and
/// holding its place.
#[derive(Debug, Clone)]
pub struct PinnedBind {
    type_name: String,
    element: String,
    x: String,
    y: String,
}

impl PinnedBind {
    /// The `pinned` bind of `specification`'s persistence, when it keeps pinned positions in the body.
    pub fn of(specification: &Json) -> Option<Self> {
        let bind = specification.pointer("/persistence/view/bind/pinned")?;
        let text = |key: &str| bind[key].as_str().map(str::to_owned);
        Some(PinnedBind {
            type_name: text("type")?,
            element: text("element")?,
            x: text("x")?,
            y: text("y")?,
        })
    }

    /// The element an entry names, as its reference holds it.
    fn named<'e>(&self, entry: &'e FblElement) -> Option<&'e str> {
        match entry.attributes.get(&self.element) {
            Some(Value::String(id)) if !id.is_empty() => Some(id),
            _ => entry
                .referenced
                .iter()
                .find(|(attribute, _)| *attribute == self.element)
                .map(|(_, id)| id.as_str()),
        }
    }

    /// The first entry naming each element, by element: the entry's id and the stored position.
    fn entries<'d>(&self, document: &'d DiagramDocument) -> HashMap<String, (&'d FblElement, f64, f64)> {
        let mut entries = HashMap::new();
        for entry in document.fbl().elements.iter().filter(|e| e.type_name == self.type_name) {
            let Some(id) = self.named(entry) else { continue };
            if document.element(id).is_none() || entries.contains_key(id) {
                continue;
            }
            let number = |name: &str| entry.attributes.get(name).and_then(Value::as_number).unwrap_or(0.0);
            entries.insert(id.to_owned(), (entry, number(&self.x), number(&self.y)));
        }
        entries
    }

    /// The stored position of every pinned element, by element id.
    pub fn positions(&self, document: &DiagramDocument) -> HashMap<String, (f64, f64)> {
        self.entries(document)
            .into_iter()
            .map(|(id, (_, x, y))| (id, (x, y)))
            .collect()
    }

    /// The change that pins `element` at `(x, y)`: its entry's place set, else a new entry.
    pub fn pin(&self, document: &DiagramDocument, element: &str, x: f64, y: f64) -> ModelChange {
        let mut place = Map::default();
        place.insert(self.x.clone(), Value::Double(x));
        place.insert(self.y.clone(), Value::Double(y));
        if let Some((entry, _, _)) = self.entries(document).remove(element) {
            return ModelChange::Set {
                id: entry.id.clone(),
                attributes: place,
            };
        }
        let mut attributes = Map::default();
        attributes.insert(self.element.clone(), Value::String(element.to_owned()));
        for (name, value) in place.iter() {
            attributes.insert(name.clone(), value.clone());
        }
        ModelChange::Add {
            type_name: self.type_name.clone(),
            id: None,
            attributes,
            parent_id: None,
            index: None,
            slot: None,
        }
    }

    /// The changes that unpin `elements`: each one's entry removed; all pinned elements for none.
    pub fn unpin(&self, document: &DiagramDocument, elements: Option<&[String]>) -> Vec<ModelChange> {
        let mut entries: Vec<(String, String)> = self
            .entries(document)
            .into_iter()
            .filter(|(id, _)| elements.is_none_or(|elements| elements.contains(id)))
            .map(|(id, (entry, _, _))| (id, entry.id.clone()))
            .collect();
        entries.sort();
        entries.into_iter().map(|(_, id)| ModelChange::Remove { id }).collect()
    }
}

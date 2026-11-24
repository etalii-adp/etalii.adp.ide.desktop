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

    /// The changes that unpin `elements`: each one's entry removed; all pinned elements for none. The last entry of
    /// the file goes first, so that an entry named by its place is still where its id says when its turn comes.
    pub fn unpin(&self, document: &DiagramDocument, elements: Option<&[String]>) -> Vec<ModelChange> {
        let mut entries: Vec<(usize, String)> = self
            .entries(document)
            .into_iter()
            .filter(|(id, _)| elements.is_none_or(|elements| elements.contains(id)))
            .map(|(_, (entry, _, _))| (entry.line, entry.id.clone()))
            .collect();
        entries.sort_by(|a, b| b.cmp(a));
        entries.into_iter().map(|(_, id)| ModelChange::Remove { id }).collect()
    }
}

/// Where a group's folded state is kept (DISL 0.11, `view.bind.groups`): an entry of the binding type naming the
/// element, the group's key and whether it is folded, kept only while the state differs from the group's default.
#[derive(Debug, Clone)]
pub struct GroupsBind {
    type_name: String,
    element: String,
    group: String,
    collapsed: String,
}

impl GroupsBind {
    /// The `groups` bind of `specification`'s persistence, when it keeps group states in the body.
    pub fn of(specification: &Json) -> Option<Self> {
        let bind = specification.pointer("/persistence/view/bind/groups")?;
        let text = |key: &str| bind[key].as_str().map(str::to_owned);
        Some(GroupsBind {
            type_name: text("type")?,
            element: text("element")?,
            group: text("group")?,
            collapsed: text("collapsed")?,
        })
    }

    fn text<'e>(entry: &'e FblElement, name: &str) -> Option<&'e str> {
        match entry.attributes.get(name) {
            Some(Value::String(text)) if !text.is_empty() => Some(text),
            _ => entry
                .referenced
                .iter()
                .find(|(attribute, _)| attribute == name)
                .map(|(_, id)| id.as_str()),
        }
    }

    /// The first entry for each group of each element, by element and key: the entry and whether it folds the group.
    fn entries<'d>(&self, document: &'d DiagramDocument) -> HashMap<(String, String), (&'d FblElement, bool)> {
        let mut entries = HashMap::new();
        for entry in document.fbl().elements.iter().filter(|e| e.type_name == self.type_name) {
            let (Some(id), Some(key)) = (Self::text(entry, &self.element), Self::text(entry, &self.group)) else {
                continue;
            };
            // A boolean as the binding types it, or as the text the file writes.
            let folded = match entry.attributes.get(&self.collapsed) {
                Some(Value::Bool(folded)) => *folded,
                Some(Value::String(text)) if text == "true" || text == "false" => text == "true",
                _ => continue,
            };
            let at = (id.to_owned(), key.to_owned());
            if document.element(id).is_none() || entries.contains_key(&at) {
                continue;
            }
            entries.insert(at, (entry, folded));
        }
        entries
    }

    /// The stored states, by element id: each group's key to whether it is folded.
    pub fn states(&self, document: &DiagramDocument) -> HashMap<String, Map> {
        let mut states: HashMap<String, Map> = HashMap::new();
        for ((id, key), (_, folded)) in self.entries(document) {
            states.entry(id).or_default().insert(key, Value::Bool(folded));
        }
        states
    }

    /// The changes that fold (or unfold) the group `key` of `element`, whose default is `default`: its entry removed
    /// where the state is the default, else its entry set, else a new entry.
    pub fn fold(
        &self,
        document: &DiagramDocument,
        element: &str,
        key: &str,
        folded: bool,
        default: bool,
    ) -> Vec<ModelChange> {
        let entry = self.entries(document).remove(&(element.to_owned(), key.to_owned()));
        match entry {
            Some((entry, _)) if folded == default => vec![ModelChange::Remove { id: entry.id.clone() }],
            Some((entry, stored)) if stored != folded => {
                let mut attributes = Map::default();
                attributes.insert(self.collapsed.clone(), Value::Bool(folded));
                vec![ModelChange::Set {
                    id: entry.id.clone(),
                    attributes,
                }]
            }
            Some(_) => Vec::new(),
            None if folded == default => Vec::new(),
            None => {
                let mut attributes = Map::default();
                attributes.insert(self.element.clone(), Value::String(element.to_owned()));
                attributes.insert(self.group.clone(), Value::String(key.to_owned()));
                attributes.insert(self.collapsed.clone(), Value::Bool(folded));
                vec![ModelChange::Add {
                    type_name: self.type_name.clone(),
                    id: None,
                    attributes,
                    parent_id: None,
                    index: None,
                    slot: None,
                }]
            }
        }
    }
}

/// Where a kept filter's value is kept (DISL 0.4, `view.bind.filters`): an attribute of the one entry of the binding
/// type, by filter id.
#[derive(Debug, Clone)]
pub struct FiltersBind {
    type_name: String,
    values: Vec<(String, String)>,
}

impl FiltersBind {
    /// The `filters` bind of `specification`'s persistence, when it keeps filter values in the body.
    pub fn of(specification: &Json) -> Option<Self> {
        let bind = specification.pointer("/persistence/view/bind/filters")?;
        let values = bind["values"]
            .as_object()?
            .iter()
            .filter_map(|(filter, attribute)| Some((filter.clone(), attribute.as_str()?.to_owned())))
            .collect();
        Some(FiltersBind {
            type_name: bind["type"].as_str()?.to_owned(),
            values,
        })
    }

    /// The change that sets the filter `filter` to `value`: none when the description keeps no such filter, or the
    /// body has no entry to keep it in.
    pub fn set(&self, document: &DiagramDocument, filter: &str, value: Value) -> Option<ModelChange> {
        let (_, attribute) = self.values.iter().find(|(id, _)| id == filter)?;
        let entry = document.fbl().elements.iter().find(|e| e.type_name == self.type_name)?;
        let mut attributes = Map::default();
        attributes.insert(attribute.clone(), value);
        Some(ModelChange::Set {
            id: entry.id.clone(),
            attributes,
        })
    }
}

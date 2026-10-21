//! How the family's protobuf payloads (`api/rdf.proto`, `api/skos.proto`, `api/shacl.proto` at `8b6ccfc`) are
//! carried on a [`DiagramElement`] (etalii.adp spec 030, task T083): as its attributes, one per field, named as the
//! payload's JSON names them (`iri`, `typeBadges`, `fromElementId`). Every field is written, defaults included, in
//! the message's field order. A repeated field is one attribute per item, `{name}.{index}`; an item of a repeated
//! message is one attribute per field, `{name}.{index}.{field}`. Booleans are `true` or `false`, integers in decimal.

use adp_diagram::DiagramElement;

/// The attributes of one payload, in the order its fields are added.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Payload {
    attributes: Vec<(String, String)>,
}

impl Payload {
    pub fn new() -> Self {
        Payload::default()
    }

    /// A text field.
    pub fn text(mut self, name: &str, value: impl Into<String>) -> Self {
        self.attributes.push((name.to_owned(), value.into()));
        self
    }

    /// A boolean field.
    pub fn flag(self, name: &str, value: bool) -> Self {
        self.text(name, if value { "true" } else { "false" })
    }

    /// An integer field.
    pub fn number(self, name: &str, value: i64) -> Self {
        self.text(name, value.to_string())
    }

    /// A repeated text field.
    pub fn list<S: AsRef<str>>(mut self, name: &str, items: &[S]) -> Self {
        for (index, item) in items.iter().enumerate() {
            self.attributes
                .push((format!("{name}.{index}"), item.as_ref().to_owned()));
        }
        self
    }

    /// A repeated message field, each item its fields in order.
    pub fn records(mut self, name: &str, items: &[Vec<(&str, String)>]) -> Self {
        for (index, fields) in items.iter().enumerate() {
            for (field, value) in fields {
                self.attributes.push((format!("{name}.{index}.{field}"), value.clone()));
            }
        }
        self
    }

    pub fn into_attributes(self) -> Vec<(String, String)> {
        self.attributes
    }
}

/// The text field `name` of `element`, empty when it has none.
pub fn text<'a>(element: &'a DiagramElement, name: &str) -> &'a str {
    element
        .attributes
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value.as_str())
}

/// The boolean field `name` of `element`.
pub fn flag(element: &DiagramElement, name: &str) -> bool {
    text(element, name) == "true"
}

/// The integer field `name` of `element`, `0` when it has none.
pub fn number(element: &DiagramElement, name: &str) -> i64 {
    text(element, name).parse().unwrap_or(0)
}

/// The items of the repeated text field `name` of `element`, in order.
pub fn list(element: &DiagramElement, name: &str) -> Vec<String> {
    let mut items = Vec::new();
    loop {
        let key = format!("{name}.{}", items.len());
        match element.attributes.iter().find(|(k, _)| *k == key) {
            Some((_, value)) => items.push(value.clone()),
            None => return items,
        }
    }
}

/// The items of the repeated message field `name` of `element`, each as its fields by name, in order.
pub fn records(element: &DiagramElement, name: &str) -> Vec<Vec<(String, String)>> {
    let mut items = Vec::new();
    loop {
        let prefix = format!("{name}.{}.", items.len());
        let fields: Vec<(String, String)> = element
            .attributes
            .iter()
            .filter_map(|(key, value)| key.strip_prefix(&prefix).map(|field| (field.to_owned(), value.clone())))
            .collect();
        if fields.is_empty() {
            return items;
        }
        items.push(fields);
    }
}

/// The field `name` of one record of [`records`], empty when it has none.
pub fn field<'a>(record: &'a [(String, String)], name: &str) -> &'a str {
    record
        .iter()
        .find(|(key, _)| key == name)
        .map_or("", |(_, value)| value.as_str())
}

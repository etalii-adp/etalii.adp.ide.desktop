//! The property rows of an element (DISL section 7.5; the standalone host's `FormDerivation`): the items of the first
//! form `for` its type whose `usage` includes `inspector` (the default), sections flattened into groups, each item
//! or section left out while its `visible` fails.
//!
//! A row's value is its `display` when it has one (DISL 0.3 section 7.5), else a type item's `optionLabel` for the
//! element's own type, else a computed item's `value`, else the attribute's text: a stored value as CEL writes it (a
//! `yearMonth` as `uuuu-MM`, a list joined by `, `), an unset one as its default when it has one, else empty. A
//! row's read-only reason is the first of its `readOnlyReasons` that applies, else, on a read-only diagram,
//! `behavior.messages`' `std.readOnly`. A row's id is the map's for the item's `id`, else its attribute, else its
//! label. A retype row is an item of kind `type`: its candidates are the types its `options` give, each named by
//! its `optionLabel` with `item` bound, and the row says it retypes so a host offers a choice.

use adp_cel::{Map, Value};

use super::env::DislEnv;
use super::evaluation::{self, Vars};
use super::wire_ids::WireIdMap;
use crate::expressions::{contexts, year_month};
use crate::json::{self, Json};
use crate::loading::metamodel::DislAttribute;
use crate::loading::specification::DislSpecification;
use crate::model::budgets;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;
use crate::model::values;

/// One row of a property inspector as a host shows it, its id mapped by a [`WireIdMap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedRow {
    /// The row id.
    pub id: String,
    pub label: String,
    /// The value as text.
    pub value: String,
    /// The item's `widget`, or none when it states none; the host chooses its editor from it.
    pub widget: Option<String>,
    /// Why it cannot be edited; empty when it can.
    pub read_only_reason: String,
    /// The title of the section (or group, or tab) it is in; empty outside one.
    pub group: String,
    /// What the value may be chosen from; none for none.
    pub candidates: Option<Vec<String>>,
    /// Whether the row changes the element's type: an item of kind `type` (DISL 0.3 section 7.5).
    pub retypes: bool,
}

/// The rows of `element`'s form for `usage` (`inspector` by default).
pub fn derive(
    specification: &DislSpecification,
    element: &DislElement,
    env: &DislEnv,
    ids: &WireIdMap,
    usage: &str,
) -> Vec<DerivedRow> {
    let mut rows = Vec::new();
    let Some((form, pointer)) = form_of(specification, element, usage) else {
        return rows;
    };
    let walk = Walk {
        specification,
        subject: Subject::Element(element),
        env,
        cel: env.to_cel(),
        ids,
    };
    walk.items(form, &json::pointer(&pointer, "items"), "", &mut rows);
    rows
}

/// The rows of the form `for` the diagram itself whose `usage` includes `usage` (DISL section 7.5), read as an element's
/// are, with `self` the diagram and its attributes those of `metamodel.diagram`; none when no form is for the diagram.
pub fn derive_diagram(
    specification: &DislSpecification,
    diagram: &DislDiagram,
    env: &DislEnv,
    ids: &WireIdMap,
    usage: &str,
) -> Vec<DerivedRow> {
    let mut rows = Vec::new();
    let Some((form, pointer)) = diagram_form_of(specification, usage) else {
        return rows;
    };
    let walk = Walk {
        specification,
        subject: Subject::Diagram(diagram),
        env,
        cel: env.to_cel(),
        ids,
    };
    walk.items(form, &json::pointer(&pointer, "items"), "", &mut rows);
    rows
}

/// The form `for` the diagram itself whose `usage` includes `usage`, the default being `inspector`.
pub(crate) fn diagram_form_of<'a>(specification: &'a DislSpecification, usage: &str) -> Option<(&'a Json, String)> {
    for (name, form) in json::members(specification.root(), "forms") {
        let usages = json::strings_of(form, "usage");
        let skip = if usages.is_empty() {
            usage != "inspector"
        } else {
            !usages.iter().any(|u| u == usage)
        };
        if !skip && json::strings_of(form, "for").iter().any(|t| t == "diagram") {
            return Some((form, json::pointer("/forms", name)));
        }
    }
    None
}

/// The form `for` the element's type whose `usage` includes `usage`, the default being `inspector`.
pub(crate) fn form_of<'a>(
    specification: &'a DislSpecification,
    element: &DislElement,
    usage: &str,
) -> Option<(&'a Json, String)> {
    for (name, form) in json::members(specification.root(), "forms") {
        let usages = json::strings_of(form, "usage");
        let skip = if usages.is_empty() {
            usage != "inspector"
        } else {
            !usages.iter().any(|u| u == usage)
        };
        if skip {
            continue;
        }
        if json::strings_of(form, "for").iter().any(|t| element.is_a(t)) {
            return Some((form, json::pointer("/forms", name)));
        }
    }
    None
}

/// The kind of a form item: its `kind`, else `field` for an item with an attribute.
pub(crate) fn kind_of(item: &Json) -> &str {
    json::string_of(item, "kind").unwrap_or(if json::has(item, "attribute") { "field" } else { "" })
}

/// What a form is read for: an element or the diagram, as `self`.
#[derive(Clone, Copy)]
enum Subject<'a> {
    Element(&'a DislElement),
    Diagram(&'a DislDiagram),
}

impl Subject<'_> {
    fn to_value(self) -> Value {
        match self {
            Subject::Element(element) => element.to_value(),
            Subject::Diagram(diagram) => diagram.to_value(),
        }
    }

    fn diagram_value(self) -> Value {
        match self {
            Subject::Element(element) => element.diagram().to_value(),
            Subject::Diagram(diagram) => diagram.to_value(),
        }
    }

    /// The attribute `name` the subject declares.
    fn declared(self, specification: &DislSpecification, name: &str) -> Option<DislAttribute> {
        match self {
            Subject::Element(element) => element.element_type().attributes.get(name).cloned(),
            Subject::Diagram(_) => specification.metamodel().diagram_attributes.get(name).cloned(),
        }
    }

    fn value_of(self, name: &str) -> Value {
        match self {
            Subject::Element(element) => element.value_of(name),
            Subject::Diagram(diagram) => diagram.value_of(name),
        }
    }

    fn stores(self, name: &str) -> bool {
        match self {
            Subject::Element(element) => element.attributes().contains_key(name),
            Subject::Diagram(diagram) => diagram.attributes().contains_key(name),
        }
    }

    /// The name of the subject's type, which a type item shows.
    fn type_name(self) -> String {
        match self {
            Subject::Element(element) => element.element_type().name.clone(),
            Subject::Diagram(_) => "diagram".to_owned(),
        }
    }
}

struct Walk<'a> {
    specification: &'a DislSpecification,
    subject: Subject<'a>,
    env: &'a DislEnv,
    cel: Map,
    ids: &'a WireIdMap,
}

impl Walk<'_> {
    fn items(&self, owner: &Json, pointer: &str, group: &str, rows: &mut Vec<DerivedRow>) {
        let Some(items) = json::items(owner, "items") else {
            return;
        };
        for (index, item) in items.iter().enumerate() {
            self.item(item, &json::pointer_at(pointer, index), group, rows);
        }
    }

    fn variables(&self, value: Value) -> Vars {
        evaluation::vars([
            ("self", self.subject.to_value()),
            ("value", value),
            ("diagram", self.subject.diagram_value()),
            ("env", Value::map(self.cel.clone())),
        ])
    }

    fn item(&self, item: &Json, pointer: &str, group: &str, rows: &mut Vec<DerivedRow>) {
        let specification = self.specification;
        let kind = kind_of(item);
        let attribute = json::string_of(item, "attribute");
        let value = match attribute {
            Some(a) if self.subject.declared(specification, a).is_some() => self.subject.value_of(a),
            _ => Value::Null,
        };
        let variables = self.variables(value);
        if let Some(visible) = json::get(item, "visible")
            && !evaluation::holds(
                specification,
                visible,
                &json::pointer(pointer, "visible"),
                contexts::FORM,
                &variables,
            )
        {
            return;
        }
        match kind {
            "section" | "group" => {
                let title = json::get(item, "label")
                    .map(|t| (t, "label"))
                    .or_else(|| json::get(item, "title").map(|t| (t, "title")));
                let name = match title {
                    Some((title, key)) => evaluation::message(
                        specification,
                        title,
                        &json::pointer(pointer, key),
                        contexts::FORM,
                        &variables,
                        group,
                    ),
                    None => group.to_owned(),
                };
                self.items(item, &json::pointer(pointer, "items"), &name, rows);
            }
            "row" => self.items(item, &json::pointer(pointer, "items"), group, rows),
            "tabs" => {
                let Some(tabs) = json::items(item, "tabs") else {
                    return;
                };
                for (index, tab) in tabs.iter().enumerate() {
                    let at = json::pointer_at(&json::pointer(pointer, "tabs"), index);
                    let title = match json::get(tab, "title") {
                        Some(text) => evaluation::message(
                            specification,
                            text,
                            &json::pointer(&at, "title"),
                            contexts::FORM,
                            &variables,
                            group,
                        ),
                        None => group.to_owned(),
                    };
                    self.items(tab, &json::pointer(&at, "items"), &title, rows);
                }
            }
            "field" | "computed" | "type" => rows.push(self.row(item, pointer, group, kind, attribute, &variables)),
            _ => {}
        }
    }

    fn row(
        &self,
        item: &Json,
        pointer: &str,
        group: &str,
        kind: &str,
        attribute: Option<&str>,
        variables: &Vars,
    ) -> DerivedRow {
        let specification = self.specification;
        let declared = attribute.and_then(|a| self.subject.declared(specification, a));
        let default_label = match &declared {
            None => attribute.unwrap_or("").to_owned(),
            Some(d) => json::string_of(&d.json, "label").unwrap_or(&d.name).to_owned(),
        };
        let label = match json::get(item, "label") {
            Some(message) => evaluation::message(
                specification,
                message,
                &json::pointer(pointer, "label"),
                contexts::FORM,
                variables,
                &default_label,
            ),
            None => default_label,
        };

        let value = if let Some(display) = json::get(item, "display") {
            text(&evaluation::expression(
                specification,
                display,
                &json::pointer(pointer, "display"),
                contexts::FORM,
                variables,
            ))
        } else if kind == "type" {
            self.option_label(item, pointer, variables, Value::String(self.subject.type_name()))
        } else if kind == "computed" {
            match json::get(item, "value") {
                Some(Json::String(s)) => s.clone(),
                Some(computed) => text(&evaluation::expression(
                    specification,
                    computed,
                    &json::pointer(pointer, "value"),
                    contexts::FORM,
                    variables,
                )),
                None => String::new(),
            }
        } else {
            declared.as_ref().map(|d| self.attribute_text(d)).unwrap_or_default()
        };

        let mut reason = json::get(item, "readOnlyReasons").and_then(|reasons| {
            evaluation::first_reason(
                specification,
                reasons,
                &json::pointer(pointer, "readOnlyReasons"),
                contexts::FORM,
                variables,
            )
        });
        if reason.is_none()
            && let Some(diagram) = variables.get("diagram").and_then(DislDiagram::from_value)
        {
            reason = budgets::withheld_reason(&diagram, false);
        }
        if reason.is_none() && self.env.read_only {
            reason = Some(evaluation::standard_message(
                specification,
                "std.readOnly",
                variables,
                "This diagram is read-only.",
            ));
        }

        let key = json::string_of(item, "id")
            .map(str::to_owned)
            .or_else(|| attribute.map(str::to_owned))
            .unwrap_or_else(|| label.clone());
        let retypes = kind == "type";
        DerivedRow {
            id: self.ids.property_id(&key),
            label,
            value,
            widget: json::string_of(item, "widget").map(str::to_owned),
            read_only_reason: reason.unwrap_or_default(),
            group: group.to_owned(),
            candidates: if retypes {
                Some(self.types(item, pointer, variables))
            } else {
                self.candidates(item, pointer, variables)
            },
            retypes,
        }
    }

    /// The types a type item offers, each named by its `optionLabel` with `item` bound, else as itself.
    fn types(&self, item: &Json, pointer: &str, variables: &Vars) -> Vec<String> {
        let Some(options) = json::get(item, "options") else {
            return Vec::new();
        };
        match evaluation::expression(
            self.specification,
            options,
            &json::pointer(pointer, "options"),
            contexts::FORM,
            variables,
        ) {
            Ok(Value::List(types)) => types
                .iter()
                .map(|t| self.option_label(item, pointer, variables, t.clone()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// A type item's name for `type_value`: its `optionLabel` with `item` bound, else the type as text.
    fn option_label(&self, item: &Json, pointer: &str, variables: &Vars, type_value: Value) -> String {
        match json::get(item, "optionLabel") {
            Some(option_label) => {
                let mut bound = variables.clone();
                bound.insert("item".into(), type_value);
                text(&evaluation::expression(
                    self.specification,
                    option_label,
                    &json::pointer(pointer, "optionLabel"),
                    contexts::FORM,
                    &bound,
                ))
            }
            None => text(&Ok(type_value)),
        }
    }

    fn candidates(&self, item: &Json, pointer: &str, variables: &Vars) -> Option<Vec<String>> {
        if let Some(options) = json::get(item, "options") {
            match options {
                Json::Array(list) => {
                    return Some(
                        list.iter()
                            .map(|o| match o {
                                Json::String(s) => s.clone(),
                                other => option_text(&values::to_value(other)),
                            })
                            .collect(),
                    );
                }
                Json::Object(_) if json::string_of(options, "enum").is_some() => {
                    let name = json::string_of(options, "enum").unwrap_or("");
                    return Some(match self.specification.metamodel().enums.get(name) {
                        Some(enumeration) => enumeration
                            .values
                            .iter()
                            .map(|v| v.label.clone().unwrap_or_else(|| v.key.clone()))
                            .collect(),
                        None => Vec::new(),
                    });
                }
                Json::Object(_) | Json::String(_) => {
                    return Some(
                        match evaluation::expression(
                            self.specification,
                            options,
                            &json::pointer(pointer, "options"),
                            contexts::FORM,
                            variables,
                        ) {
                            Ok(Value::List(list)) => list.iter().map(option_text).collect(),
                            _ => Vec::new(),
                        },
                    );
                }
                _ => {}
            }
        }
        if let Some(marks) = json::items(json::member(item, "widgetOptions"), "marks") {
            return Some(
                marks
                    .iter()
                    .map(|mark| match json::string_of(mark, "label") {
                        Some(label) => label.to_owned(),
                        None => {
                            evaluation::text_of(&json::get(mark, "value").map(values::to_value).unwrap_or(Value::Null))
                        }
                    })
                    .collect(),
            );
        }
        None
    }

    /// An attribute's text: derived, else stored, else its default when it declares one, else empty.
    fn attribute_text(&self, attribute: &DislAttribute) -> String {
        if !self.subject.stores(&attribute.name) && attribute.default.is_none() && !attribute.is_derived() {
            return String::new();
        }
        format(&self.subject.value_of(&attribute.name), &attribute.type_name)
    }
}

/// An option as it is offered: a `{value, label}` map by its label, else by its value; any other value as text.
fn option_text(option: &Value) -> String {
    if let Some(map) = option.as_map() {
        if let Some(Value::String(label)) = map.get("label") {
            return label.clone();
        }
        if let Some(value) = map.get("value") {
            return text(&Ok(value.clone()));
        }
    }
    text(&Ok(option.clone()))
}

fn format(value: &Value, type_name: &str) -> String {
    match value {
        Value::Int(month) if type_name == "yearMonth" => year_month::format(*month, "uuuu-MM").unwrap_or_default(),
        Value::List(list) => list.iter().map(|i| format(i, type_name)).collect::<Vec<_>>().join(", "),
        other => evaluation::text_of(other),
    }
}

fn text(value: &Result<Value, String>) -> String {
    match value {
        Err(_) => String::new(),
        Ok(Value::List(list)) => list.iter().map(|i| text(&Ok(i.clone()))).collect::<Vec<_>>().join(", "),
        Ok(other) => evaluation::text_of(other),
    }
}

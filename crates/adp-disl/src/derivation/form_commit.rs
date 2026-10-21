//! A form item at commit (DISL 0.3 section 7.5; the standalone host's `FormCommit`): what an inspector row does with
//! the value typed into it, what a params form asks for and refuses, and a label's inline edit (section 6.12).
//!
//! A row's commit runs in the order section 7.5 gives: the row's read-only reasons, or on a read-only diagram
//! `std.readOnly`; then each `validate` of severity `error`, whatever its timing; then the `parse`, whose `accepts`
//! refuses with its `refusal` and whose `write` makes the transaction. A row without a `parse` sets its attribute to
//! the value, typed by the attribute; a computed row without one cannot be committed. A choice is made by its label:
//! a row whose `options` are `{value, label}` maps receives the label a host shows, and the option of that label
//! gives the value `validate` sees and the row writes. The element's diagram is the transaction's working state: a
//! `write` is run on it as an operation's actions are.

use adp_cel::{Map, Value};

use super::env::DislEnv;
use super::evaluation::{self, Vars};
use super::forms::{form_of, kind_of};
use crate::behavior::actions::ActionRunner;
use crate::behavior::change::{DislChange, DislTransaction};
use crate::behavior::ids;
use crate::behavior::write;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;
use crate::model::values;

/// A dialog a params form asks a host to show (DISL 0.3 section 7.5): its title, icon, confirming label and fields.
#[derive(Debug, Clone)]
pub struct FormRequest {
    /// The form's `label`.
    pub label: String,
    pub icon: Option<String>,
    /// The form's `submitLabel`, or none for the runtime's own.
    pub submit_label: Option<String>,
    pub fields: Vec<FormRequestField>,
}

/// One field of a [`FormRequest`]: the parameter it answers, its label, its widget and the value it opens with.
#[derive(Debug, Clone)]
pub struct FormRequestField {
    pub attribute: String,
    pub label: String,
    pub widget: Option<String>,
    pub initial: Value,
}

fn variables(element: Option<&DislElement>, value: Value, env: &Map, diagram: Option<&DislDiagram>) -> Vars {
    let diagram = diagram.cloned().or_else(|| element.map(DislElement::diagram));
    evaluation::vars([
        ("self", element.map(DislElement::to_value).unwrap_or(Value::Null)),
        ("value", value),
        ("diagram", diagram.map(|d| d.to_value()).unwrap_or(Value::Null)),
        ("env", Value::map(env.clone())),
    ])
}

/// What committing `value` to the row `row` of `element`'s form does: refused with the sentence the definition
/// gives, or the transaction to write. The row is named as the form derivation names it: by its item's `id`, else
/// its attribute, else its label.
pub fn row(
    specification: &DislSpecification,
    element: &DislElement,
    row: &str,
    value: &str,
    env: &DislEnv,
    usage: &str,
) -> DislTransaction {
    let found = form_of(specification, element, usage)
        .and_then(|(form, pointer)| find(form, &json::pointer(&pointer, "items"), row));
    let Some((item, pointer)) = found else {
        return DislTransaction::refused(format!("'{row}' is not a row of this element's form."));
    };
    let element_type = element.element_type();
    let declared = json::string_of(item, "attribute").and_then(|a| element_type.attributes.get(a));
    let cel = env.to_cel();
    let current = variables(
        Some(element),
        declared.map(|d| element.value_of(&d.name)).unwrap_or(Value::Null),
        &cel,
        None,
    );

    if let Some(reasons) = json::get(item, "readOnlyReasons")
        && let Some(reason) = evaluation::first_reason(
            specification,
            reasons,
            &json::pointer(&pointer, "readOnlyReasons"),
            contexts::FORM,
            &current,
        )
    {
        return DislTransaction::refused(reason);
    }
    if env.read_only {
        return DislTransaction::refused(evaluation::standard_message(
            specification,
            "std.readOnly",
            &current,
            "This diagram is read-only.",
        ));
    }

    let chosen = chosen(specification, item, &pointer, &current, value);
    if let Some(invalid) = invalid(
        specification,
        item,
        &pointer,
        &variables(Some(element), chosen.clone(), &cel, None),
    ) {
        return DislTransaction::refused(invalid);
    }

    if let Some(parse) = json::get(item, "parse") {
        let mut text = variables(Some(element), Value::from(value), &cel, None);
        let parse_pointer = json::pointer(&pointer, "parse");
        if let Some(refused) = refused_by(specification, parse, &parse_pointer, contexts::FORM, &text, value) {
            return DislTransaction::refused(refused);
        }
        let diagram = element.diagram();
        let ids = ids::fixed(&[]);
        let mut runner = ActionRunner::new(specification, &diagram, &ids, contexts::FORM);
        if let Some(write) = json::get(parse, "write") {
            runner.run(write, &json::pointer(&parse_pointer, "write"), &mut text);
        }
        return runner.transaction();
    }

    let Some(declared) = declared.filter(|_| json::string_of(item, "kind") != Some("computed")) else {
        return DislTransaction::refused(format!("'{row}' cannot be edited here."));
    };
    set(specification, element, declared, &chosen, value)
}

/// The transaction that sets attribute `declared` of `element` to `chosen`, a text typed as the attribute is; refused
/// when the text is not a value of it. A text typed for a `many` attribute is its items as a row shows them, separated
/// by commas, each trimmed, blank ones left out.
fn set(
    specification: &DislSpecification,
    element: &DislElement,
    declared: &crate::DislAttribute,
    chosen: &Value,
    typed: &str,
) -> DislTransaction {
    let written = match chosen {
        Value::String(text) if declared.many => {
            let items: Vec<Value> = text
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(Value::from)
                .collect();
            match values::from_read(&Value::list(items), declared, specification.metamodel()) {
                Some(value) => value,
                None => return DislTransaction::refused(format!("'{typed}' is not a value of {}.", declared.name)),
            }
        }
        Value::String(text) => {
            match values::from_read(&Value::String(text.clone()), declared, specification.metamodel()) {
                Some(value) => value,
                None => return DislTransaction::refused(format!("'{typed}' is not a value of {}.", declared.name)),
            }
        }
        other => other.clone(),
    };
    let element_type = element.element_type();
    let mut attributes = Map::default();
    attributes.insert(
        declared.name.clone(),
        write::stored_form(specification, &element_type.name, &declared.name, &written),
    );
    DislTransaction {
        changes: vec![DislChange::Set {
            element_id: element.id(),
            type_name: element_type.name.clone(),
            attributes,
        }],
        host_actions: Vec::new(),
        refusal: None,
    }
}

/// What writing `text` to attribute `attribute` of `element` does, as an inline edit of a label bound to it does
/// (section 6.12): on a read-only diagram `std.readOnly`, else the attribute set to the text, typed as it is.
pub fn attribute(
    specification: &DislSpecification,
    element: &DislElement,
    attribute: &str,
    text: &str,
    env: &DislEnv,
) -> DislTransaction {
    if env.read_only {
        let variables = variables(Some(element), Value::from(text), &env.to_cel(), None);
        return DislTransaction::refused(evaluation::standard_message(
            specification,
            "std.readOnly",
            &variables,
            "This diagram is read-only.",
        ));
    }
    let element_type = element.element_type();
    let Some(declared) = element_type.attributes.get(attribute) else {
        return DislTransaction::refused(format!("'{attribute}' cannot be edited here."));
    };
    set(specification, element, declared, &Value::from(text), text)
}

/// The refusal of a parse's `accepts` that does not hold, when it does not.
fn refused_by(
    specification: &DislSpecification,
    parse: &Json,
    parse_pointer: &str,
    context: &str,
    variables: &Vars,
    text: &str,
) -> Option<String> {
    let accepts = json::get(parse, "accepts")?;
    if evaluation::holds(
        specification,
        accepts,
        &json::pointer(parse_pointer, "accepts"),
        context,
        variables,
    ) {
        return None;
    }
    let fallback = format!("'{text}' cannot be written here.");
    Some(match json::get(parse, "refusal") {
        Some(refusal) => evaluation::message(
            specification,
            refusal,
            &json::pointer(parse_pointer, "refusal"),
            context,
            variables,
            &fallback,
        ),
        None => fallback,
    })
}

fn form_json<'a>(specification: &'a DislSpecification, form: &str) -> Option<&'a Json> {
    json::get(json::member(specification.root(), "forms"), form)
}

/// The dialog the params form `form` asks a host to show for `element` (none for a form on the diagram, whose
/// `diagram` is then given): its label, icon and `submitLabel`, and each item's label, widget and `initial` value,
/// evaluated once; none when there is no such form.
pub fn request(
    specification: &DislSpecification,
    form: &str,
    element: Option<&DislElement>,
    env: &DislEnv,
    diagram: Option<&DislDiagram>,
) -> Option<FormRequest> {
    let json = form_json(specification, form)?;
    let pointer = json::pointer("/forms", form);
    let variables = variables(element, Value::Null, &env.to_cel(), diagram);
    let mut fields = Vec::new();
    for (index, item) in json::items(json, "items").into_iter().flatten().enumerate() {
        let at = json::pointer_at(&json::pointer(&pointer, "items"), index);
        let attribute = json::string_of(item, "attribute").unwrap_or("").to_owned();
        let label = match json::get(item, "label") {
            Some(text) => evaluation::message(
                specification,
                text,
                &json::pointer(&at, "label"),
                contexts::FORM,
                &variables,
                &attribute,
            ),
            None => attribute.clone(),
        };
        let initial = match json::get(item, "initial") {
            Some(expression) => evaluation::expression(
                specification,
                expression,
                &json::pointer(&at, "initial"),
                contexts::FORM,
                &variables,
            )
            .unwrap_or(Value::Null),
            None => Value::Null,
        };
        fields.push(FormRequestField {
            attribute,
            label,
            widget: json::string_of(item, "widget").map(str::to_owned),
            initial,
        });
    }
    let text = |key: &str, fallback: &str| {
        json::get(json, key).map(|t| {
            evaluation::message(
                specification,
                t,
                &json::pointer(&pointer, key),
                contexts::FORM,
                &variables,
                fallback,
            )
        })
    };
    Some(FormRequest {
        label: text("label", form).unwrap_or_else(|| form.to_owned()),
        icon: json::string_of(json, "icon").map(str::to_owned),
        submit_label: text("submitLabel", ""),
        fields,
    })
}

/// Why the params form `form` refuses `value` for its item `attribute`: the message of the first of its validations
/// of severity `error` that fails; none when the value is accepted. For a form on the diagram, `diagram` is given.
pub fn refusal(
    specification: &DislSpecification,
    form: &str,
    attribute: &str,
    value: Value,
    element: Option<&DislElement>,
    env: &DislEnv,
    diagram: Option<&DislDiagram>,
) -> Option<String> {
    let items = json::items(form_json(specification, form)?, "items")?;
    for (index, item) in items.iter().enumerate() {
        let at = json::pointer_at(&json::pointer(&json::pointer("/forms", form), "items"), index);
        if json::string_of(item, "attribute") != Some(attribute) {
            continue;
        }
        return invalid(
            specification,
            item,
            &at,
            &variables(element, value, &env.to_cel(), diagram),
        );
    }
    None
}

/// What committing the inline edit `text` of `element`'s label `label` does (DISL 0.3 section 6.12): on a read-only
/// diagram `std.readOnly`; then the label's `parse`, whose `accepts` refuses with its `refusal` and whose `write`
/// makes the transaction; without a `parse`, the text written to the attribute its `editText` binds, else to the one
/// its `text` binds, since an attribute binding makes a label editable. A label that has none of these, or no label
/// of that id, cannot be edited. The refusal is the label's editor's alone: a form item on the same attribute keeps
/// its own validation.
pub fn label(
    specification: &DislSpecification,
    element: &DislElement,
    label: &str,
    text: &str,
    env: &DislEnv,
) -> DislTransaction {
    let mut variables = variables(Some(element), Value::from(text), &env.to_cel(), None);
    if env.read_only {
        return DislTransaction::refused(evaluation::standard_message(
            specification,
            "std.readOnly",
            &variables,
            "This diagram is read-only.",
        ));
    }
    let Some((found, pointer)) = label_of(specification, element, label) else {
        return DislTransaction::refused(format!("'{label}' cannot be edited here."));
    };
    let Some(parse) = json::get(found, "parse") else {
        let bound = ["editText", "text"]
            .iter()
            .find_map(|key| json::string_of(json::get(found, key)?, "attribute"));
        return match bound {
            Some(bound) => attribute(specification, element, bound, text, env),
            None => DislTransaction::refused(format!("'{label}' cannot be edited here.")),
        };
    };
    let parse_pointer = json::pointer(&pointer, "parse");
    if let Some(refused) = refused_by(
        specification,
        parse,
        &parse_pointer,
        contexts::LABEL_PARSE,
        &variables,
        text,
    ) {
        return DislTransaction::refused(refused);
    }
    let diagram = element.diagram();
    let ids = ids::fixed(&[]);
    let mut runner = ActionRunner::new(specification, &diagram, &ids, contexts::LABEL_PARSE);
    if let Some(write) = json::get(parse, "write") {
        runner.run(write, &json::pointer(&parse_pointer, "write"), &mut variables);
    }
    runner.transaction()
}

/// The text the inline editor of `element`'s label `label` opens on (section 6.12): its `editText`, else its `text`,
/// the value of the attribute it binds or the text its expression gives; none when there is no such label or it does
/// not evaluate to text.
pub fn edit_text(
    specification: &DislSpecification,
    element: &DislElement,
    label: &str,
    env: &DislEnv,
) -> Option<String> {
    let (found, pointer) = label_of(specification, element, label)?;
    let key = if json::has(found, "editText") {
        "editText"
    } else {
        "text"
    };
    let json = json::get(found, key)?;
    if let Some(attribute) = json::string_of(json, "attribute") {
        return match element.value_of(attribute) {
            Value::Null => Some(String::new()),
            value => Some(evaluation::text_of(&value)),
        };
    }
    match evaluation::expression(
        specification,
        json,
        &json::pointer(&pointer, key),
        contexts::ELEMENT,
        &variables(Some(element), Value::Null, &env.to_cel(), None),
    ) {
        Ok(Value::String(text)) => Some(text),
        _ => None,
    }
}

/// The label `id` of the notation of the element's type: a node's, else a relation's.
fn label_of<'a>(specification: &'a DislSpecification, element: &DislElement, id: &str) -> Option<(&'a Json, String)> {
    let notation = json::get(specification.root(), "notation")?;
    let type_name = element.type_name();
    for kind in ["nodes", "edges"] {
        for (name, declared) in json::members(notation, kind) {
            if *name != type_name {
                continue;
            }
            let Some(labels) = json::items(declared, "labels") else {
                continue;
            };
            for (index, label) in labels.iter().enumerate() {
                let at = json::pointer_at(
                    &json::pointer(&json::pointer(&json::pointer("/notation", kind), name), "labels"),
                    index,
                );
                if json::string_of(label, "id") == Some(id) {
                    return Some((label, at));
                }
            }
        }
    }
    None
}

/// The message of the first validation of severity `error` of `item` that fails, or none.
fn invalid(specification: &DislSpecification, item: &Json, pointer: &str, variables: &Vars) -> Option<String> {
    let validations = json::items(item, "validate")?;
    for (index, validation) in validations.iter().enumerate() {
        let at = json::pointer_at(&json::pointer(pointer, "validate"), index);
        if json::string_of(validation, "severity").unwrap_or("error") != "error" {
            continue;
        }
        let Some(rule) = json::get(validation, "rule") else {
            continue;
        };
        if evaluation::holds(
            specification,
            rule,
            &json::pointer(&at, "rule"),
            contexts::FORM,
            variables,
        ) {
            continue;
        }
        let fallback = format!(
            "'{}' is not accepted here.",
            evaluation::text_of(variables.get("value").unwrap_or(&Value::Null))
        );
        return Some(match json::get(validation, "message") {
            Some(message) => evaluation::message(
                specification,
                message,
                &json::pointer(&at, "message"),
                contexts::FORM,
                variables,
                &fallback,
            ),
            None => fallback,
        });
    }
    None
}

/// The value of the option labelled `text` in `item`'s `options`, else the text itself.
fn chosen(specification: &DislSpecification, item: &Json, pointer: &str, variables: &Vars, text: &str) -> Value {
    let Some(options) = json::get(item, "options") else {
        return Value::from(text);
    };
    let list: Vec<Value> = match options {
        Json::Array(items) => items.iter().map(values::to_value).collect(),
        Json::Object(_) if json::has(options, "enum") => Vec::new(),
        Json::Object(_) | Json::String(_) => match evaluation::expression(
            specification,
            options,
            &json::pointer(pointer, "options"),
            contexts::FORM,
            variables,
        ) {
            Ok(Value::List(list)) => list.to_vec(),
            _ => Vec::new(),
        },
        _ => Vec::new(),
    };
    for option in list {
        if let Some(map) = option.as_map()
            && map.get("label").and_then(Value::as_str) == Some(text)
        {
            return map.get("value").cloned().unwrap_or_else(|| Value::from(text));
        }
    }
    Value::from(text)
}

/// The item of `row`: by its `id`, else its attribute, else its literal label, sections, rows and tabs searched.
fn find<'a>(owner: &'a Json, pointer: &str, row: &str) -> Option<(&'a Json, String)> {
    let items = json::items(owner, "items")?;
    for (index, item) in items.iter().enumerate() {
        let at = json::pointer_at(pointer, index);
        match kind_of(item) {
            "section" | "group" | "row" => {
                if let Some(nested) = find(item, &json::pointer(&at, "items"), row) {
                    return Some(nested);
                }
            }
            "tabs" if json::items(item, "tabs").is_some() => {
                for (tab, each) in json::items(item, "tabs").into_iter().flatten().enumerate() {
                    let tab_at = json::pointer_at(&json::pointer(&at, "tabs"), tab);
                    if let Some(in_tab) = find(each, &json::pointer(&tab_at, "items"), row) {
                        return Some(in_tab);
                    }
                }
            }
            _ => {
                let key = json::string_of(item, "id")
                    .or_else(|| json::string_of(item, "attribute"))
                    .or_else(|| json::string_of(item, "label"));
                if key == Some(row) {
                    return Some((item, at));
                }
            }
        }
    }
    None
}

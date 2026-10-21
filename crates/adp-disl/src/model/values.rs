//! Attribute values as CEL sees them (DISL sections 4.2 and 12.2; the standalone host's `DislValues`): `int` and
//! `yearMonth` as ints, `number` as a double, the text types and enum keys as strings, `many` as a list, a data type
//! as a map, a reference as the id it stores. This CEL has no `timestamp`, `duration` or `bytes` values, so `date`,
//! `datetime`, `time`, `duration` and `binary` keep their JSON text and read as null when unset.

use adp_cel::{Map, Value};

use crate::expressions::year_month;
use crate::json::Json;
use crate::loading::metamodel::{DislAttribute, DislMetamodel};

/// What an unset attribute reads as: its default, else its type's zero value (section 4.3).
pub(crate) fn unset(attribute: &DislAttribute, metamodel: &DislMetamodel) -> Value {
    if let Some(literal) = &attribute.default
        && let Some(value) = from_json(literal, attribute, metamodel)
    {
        return value;
    }
    if attribute.many {
        return Value::list(Vec::new());
    }
    match attribute.type_name.as_str() {
        "string" | "text" | "color" | "uri" | "expression" => Value::from(""),
        "int" | "yearMonth" => Value::Int(0),
        "number" => Value::Double(0.0),
        "bool" => Value::Bool(false),
        t if metamodel.enums.contains_key(t) => Value::from(""),
        t if metamodel.is_data_type(t) => Value::map(Map::default()),
        _ => Value::Null,
    }
}

/// A JSON value of `attribute`'s type as CEL sees it; none when it is not of that type.
pub(crate) fn from_json(json: &Json, attribute: &DislAttribute, metamodel: &DislMetamodel) -> Option<Value> {
    if attribute.many {
        let items = json.as_array()?;
        let mut values = Vec::with_capacity(items.len());
        for item in items {
            values.push(one(item, &attribute.type_name, metamodel)?);
        }
        return Some(Value::list(values));
    }
    one(json, &attribute.type_name, metamodel)
}

fn one(json: &Json, type_name: &str, metamodel: &DislMetamodel) -> Option<Value> {
    let value = match (type_name, json) {
        ("int", Json::Number(n)) => Value::Int(n.as_i64()?),
        ("number", Json::Number(n)) => Value::Double(n.as_f64()?),
        ("bool", Json::Bool(b)) => Value::Bool(*b),
        ("yearMonth", Json::String(s)) => Value::Int(year_month::parse(s)?),
        ("json", _) => to_value(json),
        (t, Json::Object(_)) if metamodel.is_data_type(t) => to_value(json),
        ("int" | "number" | "bool" | "yearMonth", _) => return None,
        (_, Json::String(s)) => Value::String(s.clone()),
        _ => return None,
    };
    (!value.is_null()).then_some(value)
}

/// A value a format binding read (text, an int, a double, a bool, a list, or null for an empty scalar) as
/// `attribute`'s type has it in CEL; none when it does not fit, so the attribute stays unset and reads as its
/// default. A scalar is read through its text, as written; an enum maps a stored form to its key, and a form the
/// enum lacks is kept as written (section 4.5); a `many` attribute takes a list and keeps the items that fit.
pub(crate) fn from_read(raw: &Value, attribute: &DislAttribute, metamodel: &DislMetamodel) -> Option<Value> {
    if attribute.many {
        let items = raw.as_list()?;
        return Some(Value::list(
            items
                .iter()
                .filter_map(|item| one_from_read(item, &attribute.type_name, metamodel))
                .collect(),
        ));
    }
    one_from_read(raw, &attribute.type_name, metamodel)
}

fn one_from_read(raw: &Value, type_name: &str, metamodel: &DislMetamodel) -> Option<Value> {
    if type_name == "json" || metamodel.is_data_type(type_name) {
        return Some(raw.clone());
    }
    let text = match raw {
        Value::List(_) | Value::Map(_) => return None,
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_owned(),
        Value::Int(i) => i.to_string(),
        Value::Double(d) => adp_fbl::planning::dotnet_double(*d),
        other => other.to_string(),
    };
    Some(match type_name {
        "int" => Value::Int(text.trim().parse().ok()?),
        "number" => Value::Double(text.trim().parse().ok()?),
        "bool" => match text.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => return None,
        },
        "yearMonth" => Value::Int(year_month::parse(&text)?),
        "date" | "datetime" | "time" | "duration" | "binary" => Value::String(text),
        t => match metamodel.enums.get(t) {
            Some(enumeration) => Value::String(enumeration.stored_as(&text).map(|v| v.key.clone()).unwrap_or(text)),
            None => Value::String(text),
        },
    })
}

/// Any JSON as CEL values: objects as maps, arrays as lists, integral numbers as `int`.
pub(crate) fn to_value(json: &Json) -> Value {
    match json {
        Json::Object(members) => Value::map(members.iter().map(|(k, v)| (k.clone(), to_value(v))).collect()),
        Json::Array(items) => Value::list(items.iter().map(to_value).collect()),
        Json::String(s) => Value::String(s.clone()),
        Json::Number(n) => match n.as_i64() {
            Some(i) => Value::Int(i),
            None => Value::Double(n.as_f64().unwrap_or(0.0)),
        },
        Json::Bool(b) => Value::Bool(*b),
        Json::Null => Value::Null,
    }
}

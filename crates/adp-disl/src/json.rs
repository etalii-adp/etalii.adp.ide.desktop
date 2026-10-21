//! Small readings of a specification's JSON that every part of the runtime shares (the standalone host's
//! `DislJson`), and the parse of DISL section 2.1 that keeps every member in the order it is written and reports a
//! key repeated in one object at its second occurrence, which a JSON reader that keeps the last value of a key
//! would hide.

use std::cell::RefCell;
use std::fmt;

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};

/// A specification's JSON, as it was read: objects keep their members in document order.
pub type Json = serde_json::Value;

/// The members of a JSON object, in document order.
pub type JsonObject = serde_json::Map<String, Json>;

/// What an absent value reads as: nothing, so that every reading of it finds nothing.
pub(crate) static NOTHING: Json = Json::Null;

/// Parses `bytes` as JSON; besides the value, the pointers of every key that repeats in its object, in document
/// order, at the occurrence that repeats it.
pub(crate) fn parse(bytes: &[u8]) -> Result<(Json, Vec<(String, String)>), String> {
    let duplicates = RefCell::new(Vec::new());
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let value = Seed {
        pointer: String::new(),
        duplicates: &duplicates,
    }
    .deserialize(&mut deserializer)
    .map_err(|e| e.to_string())?;
    deserializer.end().map_err(|e| e.to_string())?;
    Ok((value, duplicates.into_inner()))
}

struct Seed<'a> {
    pointer: String,
    duplicates: &'a RefCell<Vec<(String, String)>>,
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Json;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Json, D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Json, E> {
        Ok(Json::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Json, E> {
        Ok(Json::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Json, E> {
        Ok(Json::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Json, E> {
        Ok(serde_json::Number::from_f64(v).map(Json::Number).unwrap_or(Json::Null))
    }

    fn visit_str<E>(self, v: &str) -> Result<Json, E> {
        Ok(Json::String(v.to_owned()))
    }

    fn visit_string<E>(self, v: String) -> Result<Json, E> {
        Ok(Json::String(v))
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_none<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        loop {
            let seed = Seed {
                pointer: pointer_at(&self.pointer, items.len()),
                duplicates: self.duplicates,
            };
            match seq.next_element_seed(seed)? {
                Some(item) => items.push(item),
                None => return Ok(Json::Array(items)),
            }
        }
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut members = JsonObject::new();
        while let Some(key) = map.next_key::<String>()? {
            let child = pointer(&self.pointer, &key);
            if members.contains_key(&key) {
                self.duplicates.borrow_mut().push((child.clone(), key.clone()));
            }
            let value = map.next_value_seed(Seed {
                pointer: child,
                duplicates: self.duplicates,
            })?;
            members.insert(key, value);
        }
        Ok(Json::Object(members))
    }
}

/// The member `name` of `element`, when it is an object that has one.
pub(crate) fn get<'a>(element: &'a Json, name: &str) -> Option<&'a Json> {
    element.as_object().and_then(|o| o.get(name))
}

/// The member `name` of `element`, or nothing.
pub(crate) fn member<'a>(element: &'a Json, name: &str) -> &'a Json {
    get(element, name).unwrap_or(&NOTHING)
}

/// Whether `element` has the member `name`.
pub(crate) fn has(element: &Json, name: &str) -> bool {
    get(element, name).is_some()
}

/// The `x-` properties of `element` (DISL section 2.8), in order; none for a value that is not an object.
pub(crate) fn extensions(element: &Json) -> Vec<(String, Json)> {
    element
        .as_object()
        .map(|o| {
            o.iter()
                .filter(|(k, _)| k.starts_with("x-"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Whether the member `name` of `element` is `true`.
pub(crate) fn bool_of(element: &Json, name: &str) -> bool {
    matches!(get(element, name), Some(Json::Bool(true)))
}

/// The member `name` of `element` when it is a string.
pub(crate) fn string_of<'a>(element: &'a Json, name: &str) -> Option<&'a str> {
    get(element, name).and_then(Json::as_str)
}

/// A property that is one string or a list of them (a TypeRef or TypeRef[], DISL section 2.7); empty when absent.
pub(crate) fn strings_of(element: &Json, name: &str) -> Vec<String> {
    get(element, name).map(strings).unwrap_or_default()
}

/// A value that is one string or a list of them, the strings of a list kept and the rest skipped.
pub(crate) fn strings(value: &Json) -> Vec<String> {
    match value {
        Json::String(s) => vec![s.clone()],
        Json::Array(items) => items.iter().filter_map(|i| i.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

/// The members of an object property, in order; none when it is absent or not an object.
pub(crate) fn members<'a>(element: &'a Json, name: &str) -> impl Iterator<Item = (&'a String, &'a Json)> {
    get(element, name)
        .and_then(Json::as_object)
        .into_iter()
        .flat_map(|o| o.iter())
}

/// The items of an array property; none when it is absent or not an array.
pub(crate) fn items<'a>(element: &'a Json, name: &str) -> Option<&'a Vec<Json>> {
    get(element, name).and_then(Json::as_array)
}

/// The member `name` as an int, when it is an integral JSON number that fits (`TryGetInt64`).
pub(crate) fn int_of(element: &Json, name: &str) -> Option<i64> {
    get(element, name).and_then(Json::as_i64)
}

/// One reference token of a JSON Pointer (RFC 6901): `~` and `/` escaped.
fn token(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

pub(crate) fn pointer(parent: &str, name: &str) -> String {
    format!("{parent}/{}", token(name))
}

pub(crate) fn pointer_at(parent: &str, index: usize) -> String {
    format!("{parent}/{index}")
}

//! The designer's definition as vendored from etalii.adp (the standalone host's `KnowledgeDefinition` at 8b6ccfc):
//! the bindings a knowledge file is read and written with (`knowledge.fbl`), and what the specification
//! (`knowledge.des`, loaded through `adp-desl`) says that a binding does not carry - the type of each attribute, the
//! value types a column may have, and how each element's id is derived when the file does not store one.
//!
//! Loaded once. A definition that does not load is a broken build of this module, not a state a user can be in, so
//! it panics where it is first asked for rather than being reported.

use std::rc::Rc;
use std::sync::OnceLock;

use adp_desl::table::content::ValueKind;
use adp_desl::{DeslLoader, DeslSpecification};
use adp_fbl::{FblBinding, IdRequest};

use crate::values;

const SPECIFICATION: &str = include_str!("../../../../vendor/etalii.adp/definitions/designers/knowledge.des");
const BINDINGS: &[u8] = include_bytes!("../../../../vendor/etalii.adp/definitions/designers/knowledge.fbl");

/// The designer type's origin: the first line of a knowledge file's registration.
pub const ORIGIN: &str = "etalii/knowledge";

/// The specification, loaded through the DESL runtime.
pub fn specification() -> &'static DeslSpecification {
    static SPECIFICATION_LOADED: OnceLock<DeslSpecification> = OnceLock::new();
    SPECIFICATION_LOADED.get_or_init(|| {
        let result = DeslLoader::load(SPECIFICATION);
        result
            .specification
            .unwrap_or_else(|| panic!("knowledge.des does not load: {:?}", result.diagnostics))
    })
}

thread_local! {
    /// The three bindings, by name: `yaml`, `json` and `xml`.
    static LOADED: Vec<(String, Rc<FblBinding>)> = {
        let (problems, document) = adp_fbl::documents::load(BINDINGS);
        let document = document.unwrap_or_else(|| panic!("knowledge.fbl does not load: {problems:?}"));
        document.bindings.into_iter().map(|(name, binding)| (name, Rc::new(binding))).collect()
    };
}

/// The binding a body of this extension is read and written with, or none for an extension no binding has.
pub fn binding_for(extension: &str) -> Option<Rc<FblBinding>> {
    let name = match extension.to_ascii_lowercase().as_str() {
        ".yaml" | ".yml" => "yaml",
        ".json" => "json",
        ".xml" => "xml",
        _ => return None,
    };
    LOADED.with(|bindings| bindings.iter().find(|(n, _)| n == name).map(|(_, b)| b.clone()))
}

/// Whether a body of this extension gets a new entry after the last entry its own rule reads, rather than at the
/// end of the list: the YAML family does, the JSON and XML families do not. Only a list two rules read shows the
/// difference - a filter's conditions and groups - and the designer evens it out there.
pub fn adds_beside_its_rule(extension: &str) -> bool {
    matches!(extension.to_ascii_lowercase().as_str(), ".yaml" | ".yml")
}

/// The type the specification gives an attribute of an element type, or none where it gives none.
pub fn attribute_type(type_name: &str, attribute: &str) -> Option<String> {
    specification().attribute_type(type_name, attribute).map(str::to_owned)
}

/// The name the specification gives a value type, or none for a type it does not have.
pub fn value_type_label(value_type: &str) -> Option<&'static str> {
    specification().enum_label("ValueType", value_type)
}

/// How many attributes the specification types: none would mean it was not read.
pub fn typed_attribute_count() -> usize {
    specification().typed_attribute_count()
}

/// What a value type says of its values: the DESL value type with the key a knowledge file's cell holds it under,
/// or, for a type the specification does not have, text under the type's own name.
pub fn kind_of(value_type: &str) -> ValueKind {
    match specification().value_type(value_type) {
        Some(declared) => ValueKind::of(declared, crate::vocabulary::key_of(value_type)),
        None => ValueKind::unknown(value_type),
    }
}

/// The id of an element whose rule stores none, as `persistence.ids` of the specification derives it: a column and
/// a sort from their view and property, a cell from its row and property, a cell's item from its cell and what it
/// names, a group setting from its view, slot and key, and a filter's conditions and groups from their place.
pub fn derive_id(request: &IdRequest<'_>) -> Option<String> {
    let text = |name: &str| values::text(request.attributes.get(name));
    let parent = request.parent_id.unwrap_or_default();
    Some(match request.rule {
        "table" => "table".into(),
        "column" => format!("{parent}/columns/{}", text("property")),
        "sort" => format!("{parent}/sorts/{}", text("property")),
        "condition1" | "filterGroup1" => format!("{parent}/filter/{}", request.position_in_slot),
        "condition2" | "filterGroup2" | "condition3" => format!("{parent}/{}", request.position_in_slot),
        "groupOrder" | "hiddenGroup" | "collapsedGroup" => {
            format!("{parent}/{}/{}", request.parent_slot.unwrap_or_default(), text("key"))
        }
        "cell" => format!("{parent}/{}", text("property")),
        "cellOption" => format!("{parent}/{}", text("option")),
        "cellRow" => format!("{parent}/{}", text("row")),
        _ => return None,
    })
}

/// A new id: a ShortGuid, a version 4 GUID written as 25 characters of base 36, never reused and never derived
/// from anything that could change (`knowledge.des`, `persistence.ids`).
pub fn new_id() -> String {
    let mut value = uuid::Uuid::new_v4().as_u128();
    let mut digits = Vec::with_capacity(25);
    while value > 0 {
        let digit = (value % 36) as u8;
        digits.push(if digit < 10 { b'0' + digit } else { b'a' + digit - 10 });
        value /= 36;
    }
    while digits.len() < 25 {
        digits.push(b'0');
    }
    digits.reverse();
    String::from_utf8(digits).expect("base 36 is ASCII")
}

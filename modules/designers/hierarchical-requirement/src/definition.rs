//! The designer's definition as vendored from etalii.adp (the standalone host's `HierarchicalRequirementDefinition` at
//! a7fdf03): the binding a requirement document is read and written with (`hierarchical-requirement.fbl`, its one
//! binding `markdown`), and what the specification (`hierarchical-requirement.des`) says that a binding does not
//! carry: the type of each attribute and which hold many values, the binding types it maps as unreadable, the
//! mentions an attribute's text holds, and the initial members of each list.
//!
//! Loaded once. A definition that does not load is a broken build of this module, not a state a user can be in, so
//! it panics where it is first asked for rather than being reported.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::OnceLock;

use adp_fbl::{FblBinding, FblOptions};
use regex::Regex;
use serde_json::Value as Json;

const SPECIFICATION: &[u8] =
    include_bytes!("../../../../vendor/etalii.adp/definitions/designers/hierarchical-requirement.des");
const BINDINGS: &[u8] =
    include_bytes!("../../../../vendor/etalii.adp/definitions/designers/hierarchical-requirement.fbl");

/// The designer type's origin, as the envelope on a document's line 1 and its registration name it.
pub const ORIGIN: &str = "etalii/hierarchical-requirement";

/// What is read from the specification, by element type and attribute.
#[derive(Default)]
struct Specification {
    types: HashMap<(String, String), String>,
    many: HashSet<(String, String)>,
    mentions: HashMap<(String, String), Regex>,
    unreadable: HashSet<String>,
    initial: HashMap<String, Vec<(String, String)>>,
}

fn specification() -> &'static Specification {
    static LOADED: OnceLock<Specification> = OnceLock::new();
    LOADED.get_or_init(load)
}

fn load() -> Specification {
    let text = std::str::from_utf8(SPECIFICATION).expect("hierarchical-requirement.des is UTF-8");
    let root: Json = serde_json::from_str(text.trim_start_matches('\u{feff}'))
        .unwrap_or_else(|e| panic!("hierarchical-requirement.des does not load: {e}"));
    let mut specification = Specification::default();
    for (type_name, declared) in root["metamodel"]["types"].as_object().into_iter().flatten() {
        for (attribute, value) in declared["attributes"].as_object().into_iter().flatten() {
            let key = (type_name.clone(), attribute.clone());
            if let Some(type_of) = value["type"].as_str() {
                specification.types.insert(key.clone(), type_of.to_owned());
            }
            if value["many"].as_bool() == Some(true) {
                specification.many.insert(key.clone());
            }
            if let Some(pattern) = value["mentions"]["pattern"].as_str() {
                let compiled = Regex::new(pattern)
                    .unwrap_or_else(|e| panic!("the mentions of {type_name}.{attribute} do not compile: {e}"));
                specification.mentions.insert(key, compiled);
            }
        }
    }
    let persistence = &root["persistence"];
    for (name, entry) in persistence["typeMap"].as_object().into_iter().flatten() {
        if entry["as"].as_str() == Some("unreadable") {
            specification.unreadable.insert(name.clone());
        }
    }
    for (name, setting) in persistence["settings"].as_object().into_iter().flatten() {
        let Some(initial) = setting["initial"].as_array() else {
            continue;
        };
        let key = setting["key"].as_str().unwrap_or_default();
        let value = setting["value"].as_str().unwrap_or_default();
        let members = initial
            .iter()
            .map(|member| {
                (
                    member[key].as_str().unwrap_or_default().to_owned(),
                    member[value].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        specification.initial.insert(name.clone(), members);
    }
    specification
}

thread_local! {
    /// The binding of the Markdown file.
    static BINDING: Rc<FblBinding> = {
        let (problems, document) = adp_fbl::documents::load(BINDINGS);
        let mut document = document.unwrap_or_else(|| panic!("hierarchical-requirement.fbl does not load: {problems:?}"));
        let at = document
            .bindings
            .iter()
            .position(|(name, _)| name == "markdown")
            .expect("hierarchical-requirement.fbl has its binding markdown");
        Rc::new(document.bindings.swap_remove(at).1)
    };
}

/// The binding of the Markdown file.
pub fn binding() -> Rc<FblBinding> {
    BINDING.with(Rc::clone)
}

/// The reading's options for a file named `file_name`: the specification's types, its many attributes and its
/// unreadable types.
pub fn options(file_name: &str) -> Rc<FblOptions> {
    Rc::new(FblOptions {
        file_name: file_name.to_owned(),
        attribute_type: Some(Rc::new(|t: &str, a: &str| attribute_type(t, a))),
        many: Some(Rc::new(is_many)),
        unreadable_types: specification().unreadable.clone(),
        ..FblOptions::default()
    })
}

/// The type the specification gives an attribute of an element type, or none where it gives none.
pub fn attribute_type(type_name: &str, attribute: &str) -> Option<String> {
    specification()
        .types
        .get(&(type_name.to_owned(), attribute.to_owned()))
        .cloned()
}

/// Whether an attribute holds many values (DESL 0.2 section 4.2), as an item's and a requirement's tags do.
pub fn is_many(type_name: &str, attribute: &str) -> bool {
    specification()
        .many
        .contains(&(type_name.to_owned(), attribute.to_owned()))
}

/// The expression of the mentions an attribute's text holds (DESL 0.2 section 4.3), or none for one that holds none.
pub fn mentions(type_name: &str, attribute: &str) -> Option<&'static Regex> {
    specification()
        .mentions
        .get(&(type_name.to_owned(), attribute.to_owned()))
}

/// The initial members of the list `setting` keeps, as name and colour, in their order.
pub fn initial(setting: &str) -> Vec<(String, String)> {
    specification().initial.get(setting).cloned().unwrap_or_default()
}

/// How many attribute types the specification gives: none would mean it was not read.
pub fn typed_attribute_count() -> usize {
    specification().types.len()
}

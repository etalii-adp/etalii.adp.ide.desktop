//! What a slot of a yaml or json body reads from a scalar (FBL 0.5 section 5.2; the standalone host's
//! `Rules/ReadMode`): the written text, the typed value converted to the attribute's type, or, where neither the
//! binding nor the caller says, the value as the family types it, as before FBL 0.5.

use crate::documents::types::{AttributeBinding, Rule};
use crate::model::FblOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadMode {
    /// Neither the binding nor the caller says: the value as the family types it, as before FBL 0.5.
    Legacy,
    /// The written text (FBL 0.5 section 4.1.5).
    Text,
    /// The typed value, converted to the attribute's type.
    Typed,
}

/// The binding's `read` first; else text for an attribute the caller types as a string, or for a reference; else
/// typed when the caller gives a type; else legacy.
fn of_binding(binding: &AttributeBinding, type_name: Option<&str>) -> ReadMode {
    match binding.read.as_deref() {
        Some("text") => ReadMode::Text,
        Some("typed") => ReadMode::Typed,
        _ if type_name == Some("string") || binding.reference.is_some() => ReadMode::Text,
        _ if type_name.is_some() => ReadMode::Typed,
        _ => ReadMode::Legacy,
    }
}

/// The mode of an attribute of `rule`, with the type the caller gives it.
pub(crate) fn of(rule: &Rule, attribute: &str, binding: &AttributeBinding, options: &FblOptions) -> ReadMode {
    of_binding(binding, options.attribute_type(&rule.type_name, attribute).as_deref())
}

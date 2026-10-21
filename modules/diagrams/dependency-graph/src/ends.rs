//! The CEL function of the dependency graph specification's `net.etalii.adp.generic.dependencyEnds` plugin (DISL
//! section 13.1.1; the standalone host's `DependencyGraphFindings.cs`, `Plugins` and `WrittenEnd`, at `8b6ccfc`).
//!
//! `writtenEnd(relation, end)` is the id a dependency's `source` or `target` end is written with in the file (`from`
//! or `to`), also when it names no node and the end is none in the model: the binding reads each end a second time as
//! written, into the host attributes `writtenFrom` and `writtenTo`, which CEL does not see. The specification's
//! fallback reads only an end that reaches a node, so a dangling end would read as empty and go unreported; the
//! module always supplies the function.

use adp_cel::Value;
use adp_disl::{DislElement, DislPluginFunctions};

/// What a call that is not over a dependency and one of its two ends answers.
pub const MISUSED: &str = "writtenEnd() takes a dependency of this graph and 'source' or 'target'.";

/// The implementations, for loading the specification.
pub fn plugin_functions() -> DislPluginFunctions {
    DislPluginFunctions::none().add("writtenEnd", |arguments| {
        let (Some(relation), Some(end)) = (
            arguments.first().and_then(DislElement::from_value),
            arguments.get(1).and_then(Value::as_str),
        ) else {
            return Err(MISUSED.to_owned());
        };
        written_end(&relation, end).map(Value::String)
    })
}

/// The id `relation`'s `end` (`source` or `target`) is written with: its host attribute `writtenFrom` or
/// `writtenTo`, the empty text when the key is absent or empty.
pub fn written_end(relation: &DislElement, end: &str) -> Result<String, String> {
    let attribute = match end {
        "source" => "writtenFrom",
        "target" => "writtenTo",
        _ => return Err(MISUSED.to_owned()),
    };
    if !relation.is_relation() {
        return Err(MISUSED.to_owned());
    }
    Ok(relation
        .host_attributes()
        .get(attribute)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned())
}

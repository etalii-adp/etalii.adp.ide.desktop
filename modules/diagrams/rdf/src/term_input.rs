//! A dialog's term input read the way an author writes terms (etalii.adp spec 030, task T083): a full IRI, a bracketed
//! IRI, or a prefixed name under the document's own declarations; an undeclared prefix is refused by name rather than
//! a declaration invented. Ported from the standalone host's `RdfTermInput.cs` at `8b6ccfc`.

use crate::model::RdfModel;

/// The full IRI `value` means under `model`'s declarations, or the refusal saying why not (`RdfTermInput.Resolve`).
pub fn resolve(model: &RdfModel, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("A term needs a name: a full IRI, or a prefixed name like ex:thing.".to_owned());
    }
    if trimmed.contains(' ') {
        return Err("An IRI cannot contain spaces.".to_owned());
    }
    // Brackets say "this is the IRI itself" - the way out for scheme-only IRIs like urn:...
    if trimmed.len() >= 2 && trimmed.starts_with('<') && trimmed.ends_with('>') {
        let inner = trimmed[1..trimmed.len() - 1].trim();
        return if inner.is_empty() {
            Err("The brackets are empty.".to_owned())
        } else {
            Ok(inner.to_owned())
        };
    }
    if trimmed.contains("://") {
        return Ok(trimmed.to_owned());
    }
    let Some(colon) = trimmed.find(':') else {
        return Err(format!(
            "'{trimmed}' is neither a full IRI nor a prefixed name. Write ex:{trimmed}, or a full IRI."
        ));
    };
    let prefix = &trimmed[..colon];
    match model.expansion(prefix) {
        Some(expansion) => Ok(format!("{expansion}{}", &trimmed[colon + 1..])),
        None => Err(format!(
            "The prefix '{prefix}:' is not declared in this file. Declare it first, write the full IRI, or bracket a scheme IRI as <{trimmed}>."
        )),
    }
}

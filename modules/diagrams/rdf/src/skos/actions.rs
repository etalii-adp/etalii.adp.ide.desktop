//! The scheme reading's gestures (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Skos/SkosActions.cs` at `8b6ccfc`. Everything is keyed off the file's own assertions - both ends typed
//! `skos:Concept`, a `skos:ConceptScheme` present - never off the registration: an element is an element whichever
//! reading selected it.

use adp_context::{Action, ActionGroup, Shortcut};

use crate::commands::{ObjectValue, RdfEdit};
use crate::family::{self, Run, Target};
use crate::model::RdfModel;
use crate::selection::{is_placement, relation_of};
use crate::skos::labels::DEFAULT_LANGUAGE;
use crate::skos::projection::SkosEdgeKind;
use crate::skos::selection::{concept_of, first_scheme, has_scheme, pair_of};
use crate::store::RdfDocument;
use crate::vocabulary::skos;

/// File one concept under another: one `skos:broader` on the narrower end, no inverse written.
pub const FILE_UNDER: &str = "skos.file-under";
/// Cross-link two concepts: one `skos:related`.
pub const RELATE: &str = "skos.relate";
/// Create a concept at a placement: type, preferred label and scheme as one command.
pub const ADD_CONCEPT: &str = "skos.add-concept";
/// Remove every asserted triple of a hierarchy or related pair.
pub const DISCONNECT: &str = "skos.disconnect";

fn group(actions: Vec<Action>) -> Vec<ActionGroup> {
    vec![ActionGroup { actions }]
}

/// The skos entries for a discovery pass, or none where the selection is not this reading's business (`Discover`).
pub fn discover(document: &RdfDocument, target: &Target) -> Vec<ActionGroup> {
    let model = &document.model;
    if let Some((from, to)) = relation_of(target.element_id)
        && concept_of(model, &from).is_some()
        && concept_of(model, &to).is_some()
    {
        return group(vec![
            Action::new(FILE_UNDER, "File under (broader)").with_icon("mdi-file-tree"),
            Action::new(RELATE, "Relate (skos:related)").with_icon("mdi-swap-horizontal"),
        ]);
    }
    if is_placement(target.element_id) && has_scheme(model) {
        return group(vec![
            Action::new(ADD_CONCEPT, "Add concept here…").with_icon("mdi-tag-plus-outline"),
        ]);
    }
    if let Some(pair) = pair_of(model, target.element_id) {
        let label = match pair.kind {
            SkosEdgeKind::Hierarchy if pair.triples.len() > 1 => "Disconnect (both directions)",
            SkosEdgeKind::Hierarchy => "Disconnect",
            _ => "Remove related link",
        };
        return group(vec![
            Action::new(DISCONNECT, label)
                .with_icon("mdi-link-off")
                .with_shortcut(Shortcut::key("Delete")),
        ]);
    }
    Vec::new()
}

/// Executes a skos action, or none when `action_id` is not this reading's (`ExecuteAsync`).
pub fn execute(document: &RdfDocument, target: &Target, action_id: &str) -> Option<Run> {
    let model = &document.model;
    match action_id {
        FILE_UNDER | RELATE => {
            let (from, to) = relation_of(target.element_id)?;
            let (Some(from_iri), Some(to_iri)) = (concept_of(model, &from), concept_of(model, &to)) else {
                return Some(Run::Refused(
                    "Both ends of this gesture must be concepts the file types as skos:Concept.".to_owned(),
                ));
            };
            if from_iri == to_iri {
                return Some(Run::Refused(
                    "A concept cannot be filed under or related to itself.".to_owned(),
                ));
            }
            let predicate = if action_id == FILE_UNDER {
                skos::BROADER
            } else {
                skos::RELATED
            };
            Some(Run::Edit(RdfEdit::AddTriple {
                subject: from_iri,
                predicate: predicate.to_owned(),
                object: ObjectValue::iri(to_iri),
            }))
        }
        ADD_CONCEPT => Some(Run::Ask(family::input(
            "Add concept",
            "mdi-tag-plus-outline",
            "Preferred label",
            "",
            "Add",
        ))),
        DISCONNECT => {
            let pair = pair_of(model, target.element_id)?;
            if pair.triples.len() > 1 {
                return Some(Run::Ask(family::confirm(
                    "Disconnect",
                    "mdi-link-off",
                    "This pair is asserted in both directions; disconnecting removes both statements as one undo.",
                    "Disconnect",
                    false,
                )));
            }
            Some(Run::Edit(RdfEdit::DisconnectSkosPair {
                a: pair.a_iri,
                b: pair.b_iri,
                hierarchy: pair.kind == SkosEdgeKind::Hierarchy,
            }))
        }
        _ => None,
    }
}

/// Validates a skos dialog value, or none when the action is not this reading's (`Validate`).
pub fn validate(document: &RdfDocument, action_id: &str, value: &str) -> Option<Result<(), String>> {
    if action_id != ADD_CONCEPT {
        return None;
    }
    if value.trim().is_empty() {
        return Some(Err("A concept needs a preferred label.".to_owned()));
    }
    let model = &document.model;
    let iri = match mint_iri(model, value) {
        Ok(iri) => iri,
        Err(error) => return Some(Err(error)),
    };
    if model.names(&iri) {
        return Some(Err(format!(
            "{iri} already names something in this document. Choose a different label, or rename the existing resource first."
        )));
    }
    Some(Ok(()))
}

/// The edit a skos dialog or a confirmed disconnect commits, or none when the action is not this reading's
/// (`CommandFor`).
pub fn command_for(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Option<RdfEdit> {
    let model = &document.model;
    match action_id {
        ADD_CONCEPT => {
            let iri = mint_iri(model, value).ok()?;
            Some(RdfEdit::AddSkosConcept {
                concept: iri,
                label: value.trim().to_owned(),
                language: DEFAULT_LANGUAGE.to_owned(),
                scheme: first_scheme(model).unwrap_or_default(),
            })
        }
        DISCONNECT => {
            let pair = pair_of(model, target.element_id)?;
            Some(RdfEdit::DisconnectSkosPair {
                a: pair.a_iri,
                b: pair.b_iri,
                hierarchy: pair.kind == SkosEdgeKind::Hierarchy,
            })
        }
        _ => None,
    }
}

/// The IRI a labeled concept starts life under: the namespace of the prefix the file's first prefixed subject is
/// written under, else the first declared prefix, plus the label slugged (`MintIri`).
pub fn mint_iri(model: &RdfModel, label: &str) -> Result<String, String> {
    let subject_prefix = model
        .triples
        .iter()
        .filter_map(|t| t.subject.as_iri_term())
        .map(|s| s.as_written.as_str())
        .find(|written| !written.starts_with('<') && written.contains(':'))
        .map(|written| &written[..written.find(':').expect("contains a colon")]);
    let namespace = subject_prefix
        .and_then(|prefix| model.expansion(prefix))
        .or_else(|| model.prefixes.first().map(|p| p.iri.as_str()));
    let Some(namespace) = namespace else {
        return Err(
            "This file declares no prefix to mint the concept's IRI under. Declare one first (the family's prefix action)."
                .to_owned(),
        );
    };
    let mut slug = String::with_capacity(label.len());
    for character in label.trim().chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            slug.push(character);
        } else {
            // One per UTF-16 unit, as the standalone host walks a string's chars.
            for _ in 0..character.len_utf16() {
                slug.push('_');
            }
        }
    }
    let local = slug.trim_matches('_');
    if local.is_empty() {
        Err("The label leaves nothing usable for an IRI once slugged; use at least one letter or digit.".to_owned())
    } else {
        Ok(format!("{namespace}{local}"))
    }
}

//! The property grid of an element the file asserts to be a concept, scheme or collection (etalii.adp spec 030, task
//! T083), ported from the standalone host's `Skos/SkosProperties.cs` at `8b6ccfc`: identity and notation, every label
//! by kind and language, the documentation properties as editable rows, memberships, and out-of-file mappings
//! read-only with their reason. Label and documentation edits are single-literal rewrites.

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::{ObjectValue, RdfEdit};
use crate::family::Target;
use crate::model::{LiteralTerm, RdfModel};
use crate::projection::{display, local_part};
use crate::selection::{TRUNCATED_REFUSAL, is_truncated};
use crate::skos::selection::{concept_of, has_xl_labels, scheme_or_collection_of};
use crate::store::RdfDocument;
use crate::vocabulary::skos;

const IDENTITY_GROUP: &str = "Identity";
const LABELS_GROUP: &str = "Labels";
const DOCUMENTATION_GROUP: &str = "Documentation";
const MEMBERSHIP_GROUP: &str = "Membership";
const MAPPINGS_GROUP: &str = "Mappings";

const RENAME_VIA_MENU: &str = "Rename through the context menu, so every reference follows the name.";
const MEMBERSHIP_IS_GESTURE: &str = "Membership is stated by triples; file and unfile concepts on the canvas.";
/// Why an XL-labeled resource's labels read rather than edit.
pub const XL_BOUNDARY: &str = "This concept's labels are stated through SKOS-XL, which this reading does not resolve (Requirement 3.6). Edit them as triples in the graph reading.";
const OUT_OF_FILE_MAPPING: &str =
    "The mapped concept lives outside this file, so the mapping shows here rather than drawing.";

fn row(id: String, label: String, value: String, read_only_reason: &str, group: &str) -> PropertyRow {
    let mut row = PropertyRow::new(id, label, PropertyEditor::Line, value);
    row.read_only_reason = read_only_reason.to_owned();
    row.group = group.to_owned();
    row
}

fn literals<'a>(model: &'a RdfModel, iri: &'a str, predicate: &'a str) -> impl Iterator<Item = &'a LiteralTerm> + 'a {
    model
        .triples
        .iter()
        .filter(move |t| t.subject.is_iri(iri) && t.predicate.iri == predicate)
        .filter_map(|t| t.object.as_literal())
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The skos rows for the selection, or none when the element is not this reading's business (`Describe`).
pub fn describe(document: &RdfDocument, target: &Target) -> Option<Vec<PropertyRow>> {
    let model = &document.model;
    let concept_iri = concept_of(model, target.element_id);
    let container_iri = concept_iri
        .clone()
        .or_else(|| scheme_or_collection_of(model, target.element_id))?;

    let truncated = is_truncated(model);
    let xl = has_xl_labels(model, &container_iri);
    let mut rows = vec![row(
        "skos.iri".to_owned(),
        "IRI".to_owned(),
        container_iri.clone(),
        RENAME_VIA_MENU,
        IDENTITY_GROUP,
    )];

    let notations: Vec<&str> = literals(model, &container_iri, skos::NOTATION)
        .map(|l| l.lexical.as_str())
        .collect();
    if !notations.is_empty() {
        rows.push(row(
            "skos.notation".to_owned(),
            "Notation".to_owned(),
            notations.join(", "),
            "Notations are codes other systems key off; edit them as triples.",
            IDENTITY_GROUP,
        ));
    }

    for (source, name) in [
        (skos::PREF_LABEL, "Preferred"),
        (skos::ALT_LABEL, "Alternate"),
        (skos::HIDDEN_LABEL, "Hidden"),
    ] {
        for literal in literals(model, &container_iri, source) {
            let language = literal.language.clone().unwrap_or_default();
            let tag = if language.is_empty() {
                String::new()
            } else {
                format!(" @{language}")
            };
            let editable = source == skos::PREF_LABEL && !truncated && !xl;
            let reason = if editable {
                ""
            } else if xl {
                XL_BOUNDARY
            } else if truncated {
                TRUNCATED_REFUSAL
            } else {
                "Alternate and hidden labels are edited as triples."
            };
            rows.push(row(
                format!("skos.label:{}:{language}", local_part(source)),
                format!("{name}{tag}"),
                literal.lexical.clone(),
                reason,
                LABELS_GROUP,
            ));
        }
    }

    for documentation in skos::DOCUMENTATION {
        let value = literals(model, &container_iri, documentation)
            .next()
            .map(|l| l.lexical.clone())
            .unwrap_or_default();
        rows.push(row(
            format!("skos.doc:{}", local_part(documentation)),
            capitalize(&local_part(documentation)),
            value,
            if truncated { TRUNCATED_REFUSAL } else { "" },
            DOCUMENTATION_GROUP,
        ));
    }

    if let Some(concept_iri) = concept_iri {
        let mut schemes: Vec<String> = Vec::new();
        for triple in &model.triples {
            if triple.subject.is_iri(&concept_iri)
                && matches!(triple.predicate.iri.as_str(), skos::IN_SCHEME | skos::TOP_CONCEPT_OF)
                && let Some(object) = triple.object.as_iri()
            {
                let shown = display(model, object);
                if !schemes.contains(&shown) {
                    schemes.push(shown);
                }
            }
        }
        rows.push(row(
            "skos.schemes".to_owned(),
            "In schemes".to_owned(),
            if schemes.is_empty() {
                "(unfiled)".to_owned()
            } else {
                schemes.join(", ")
            },
            MEMBERSHIP_IS_GESTURE,
            MEMBERSHIP_GROUP,
        ));

        for mapping in &model.triples {
            let Some(object) = mapping.object.as_iri() else {
                continue;
            };
            if mapping.subject.is_iri(&concept_iri)
                && skos::MAPPINGS.contains(&mapping.predicate.iri.as_str())
                && concept_of(model, &format!("res:{object}")).is_none()
            {
                let fragment = local_part(&mapping.predicate.iri);
                rows.push(row(
                    format!("skos.mapping:{fragment}:{object}"),
                    capitalize(&fragment),
                    object.to_owned(),
                    OUT_OF_FILE_MAPPING,
                    MAPPINGS_GROUP,
                ));
            }
        }
    }
    Some(rows)
}

/// The edit a skos row edit makes, or none when the row is not this reading's or not editable (`CommandFor`).
pub fn command_for(document: &RdfDocument, target: &Target, property_id: &str, value: &str) -> Option<RdfEdit> {
    let model = &document.model;
    let iri = concept_of(model, target.element_id).or_else(|| scheme_or_collection_of(model, target.element_id))?;

    if let Some(language) = property_id.strip_prefix("skos.label:prefLabel:") {
        if has_xl_labels(model, &iri) {
            return None;
        }
        let existing =
            literals(model, &iri, skos::PREF_LABEL).find(|l| l.language.as_deref().unwrap_or_default() == language);
        return Some(match existing {
            None => RdfEdit::AddTriple {
                subject: iri,
                predicate: skos::PREF_LABEL.to_owned(),
                object: ObjectValue::literal(value, language, ""),
            },
            Some(existing) => replace(&iri, skos::PREF_LABEL, existing, value),
        });
    }

    if let Some(fragment) = property_id.strip_prefix("skos.doc:") {
        let predicate = skos::DOCUMENTATION.into_iter().find(|d| local_part(d) == fragment)?;
        let existing = literals(model, &iri, predicate).next();
        return Some(match existing {
            None => RdfEdit::AddTriple {
                subject: iri,
                predicate: predicate.to_owned(),
                object: ObjectValue::literal(value, "", ""),
            },
            Some(existing) => replace(&iri, predicate, existing, value),
        });
    }
    None
}

/// The literal rewritten in place, its language and datatype kept.
fn replace(iri: &str, predicate: &str, existing: &LiteralTerm, value: &str) -> RdfEdit {
    let language = existing.language.clone().unwrap_or_default();
    let datatype = existing.datatype.clone().unwrap_or_default();
    RdfEdit::ReplaceLiteral {
        subject: iri.to_owned(),
        predicate: predicate.to_owned(),
        old: ObjectValue::literal(existing.lexical.clone(), language.clone(), datatype.clone()),
        new_lexical: value.to_owned(),
        new_language: language,
        new_datatype: datatype,
    }
}

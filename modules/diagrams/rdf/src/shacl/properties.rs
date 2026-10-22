//! The property grid of a shape card (etalii.adp spec 030, task T083), ported from the standalone host's
//! `Shacl/ShaclProperties.cs` at `8b6ccfc`: identity, the shape's own annotations, its targets and every constraint row
//! with its values. Blank-rooted content is fully readable here though nothing about it is editable, and almost
//! everything is read-only on purpose - targets, rows, activation, rename and removal are gestures on the menu - so only
//! `sh:name` and `sh:description` are written here; each read-only row says which path to take instead.

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::RdfEdit;
use crate::family::Target;
use crate::model::RdfModel;
use crate::selection;
use crate::shacl::projection::{self, ShaclCard, ShaclTargetKind};
use crate::shacl::{ORIGIN, gate, refusals};
use crate::store::RdfDocument;
use crate::vocabulary::shacl;

const IDENTITY_GROUP: &str = "Identity";
const SHAPE_GROUP: &str = "Shape";
const TARGETS_GROUP: &str = "Targets";
const CONSTRAINTS_GROUP: &str = "Constraints";

const RENAME_VIA_MENU: &str = "Rename through the context menu, so every reference follows the name.";
const TARGETS_ARE_GESTURES: &str = "Targets are declared and withdrawn through the context menu.";
const ACTIVATION_IS_GESTURE: &str = "Switch the shape off and on through the context menu.";
const ROWS_ARE_GESTURES: &str = "Property rows are added through the context menu, and edited as triples.";
const EDITED_AS_TRIPLES: &str = "This is edited as triples in the graph reading.";
const NOT_EXECUTED: &str = "Shown as written, never parsed beyond extraction and never executed - this reading draws constraints, it does not run them.";

fn row(
    id: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
    reason: &str,
    group: &str,
) -> PropertyRow {
    let mut row = PropertyRow::new(id, label, PropertyEditor::Line, value);
    row.read_only_reason = reason.to_owned();
    row.group = group.to_owned();
    row
}

/// The grid of `target` when this reading claims it, none otherwise (`Describe`).
pub fn describe(document: &RdfDocument, target: &Target) -> Option<Vec<PropertyRow>> {
    if target.origin != ORIGIN || target.element_id.is_empty() {
        return None;
    }
    let model = &document.model;
    let projected = projection::project(model, projection::DEFAULT_BUDGET);
    let card = projected.cards.iter().find(|c| c.id == target.element_id)?;

    let truncated = selection::is_truncated(model);
    let frozen = if card.blank {
        refusals::BLANK_ROOTED
    } else if truncated {
        selection::TRUNCATED_REFUSAL
    } else {
        ""
    };
    let mut rows = vec![
        row(
            "shacl.iri",
            "IRI",
            if card.blank { "(anonymous)" } else { card.iri.as_str() },
            if card.blank {
                refusals::BLANK_ROOTED
            } else {
                RENAME_VIA_MENU
            },
            IDENTITY_GROUP,
        ),
        row(
            "shacl.display",
            "Name",
            card.display.clone(),
            RENAME_VIA_MENU,
            IDENTITY_GROUP,
        ),
        row("shacl.name", "sh:name", card.name.clone(), frozen, SHAPE_GROUP),
        row(
            "shacl.description",
            "sh:description",
            card.description.clone(),
            frozen,
            SHAPE_GROUP,
        ),
    ];
    let messages = literals(model, card, shacl::MESSAGE);
    if !messages.is_empty() {
        rows.push(row(
            "shacl.message",
            "sh:message",
            messages.join(" / "),
            EDITED_AS_TRIPLES,
            SHAPE_GROUP,
        ));
    }
    rows.push(row(
        "shacl.deactivated",
        "Deactivated",
        if card.deactivated { "true" } else { "false" },
        ACTIVATION_IS_GESTURE,
        SHAPE_GROUP,
    ));
    if card.closed {
        rows.push(row("shacl.closed", "Closed", "true", EDITED_AS_TRIPLES, SHAPE_GROUP));
    }
    if !card.severity.is_empty() {
        rows.push(row(
            "shacl.severity",
            "Severity",
            card.severity.clone(),
            EDITED_AS_TRIPLES,
            SHAPE_GROUP,
        ));
    }
    add_targets(&mut rows, card);
    add_constraint_rows(&mut rows, card);
    add_sparql_queries(&mut rows, model, card);
    Some(rows)
}

/// The edit setting `property_id` makes when the row is this reading's, none otherwise (`CommandFor`).
pub fn command_for(document: &RdfDocument, target: &Target, property_id: &str, value: &str) -> Option<RdfEdit> {
    if target.origin != ORIGIN {
        return None;
    }
    let predicate = match property_id {
        "shacl.name" => shacl::NAME,
        "shacl.description" => shacl::DESCRIPTION,
        _ => return None,
    };
    let model = &document.model;
    let decision = gate(model, target.element_id, selection::is_truncated(model));
    (decision.applies && decision.available && !decision.shape_iri.is_empty()).then(|| RdfEdit::SetShaclLiteral {
        shape: decision.shape_iri,
        predicate: predicate.to_owned(),
        value: value.to_owned(),
    })
}

fn kind_label(kind: ShaclTargetKind) -> &'static str {
    match kind {
        ShaclTargetKind::Class => "Target class",
        ShaclTargetKind::Node => "Target node",
        ShaclTargetKind::SubjectsOf => "Target subjects of",
        ShaclTargetKind::ObjectsOf => "Target objects of",
        ShaclTargetKind::ImplicitClass => "Target (implicit)",
    }
}

fn add_targets(rows: &mut Vec<PropertyRow>, card: &ShaclCard) {
    for (ordinal, chip) in card.targets.iter().enumerate() {
        let absent = if chip.described_in_file {
            ""
        } else {
            " (not described in this file)"
        };
        rows.push(row(
            format!("shacl.target:{ordinal}"),
            kind_label(chip.kind),
            format!("{}{absent}", chip.term_display),
            if chip.predicate_iri.is_empty() {
                refusals::IMPLICIT_TARGET
            } else {
                TARGETS_ARE_GESTURES
            },
            TARGETS_GROUP,
        ));
    }
}

fn add_constraint_rows(rows: &mut Vec<PropertyRow>, card: &ShaclCard) {
    for (ordinal, constraint) in card.rows.iter().enumerate() {
        let label = if !constraint.path.is_empty() {
            constraint.path.as_str()
        } else if constraint.sparql {
            "SPARQL"
        } else {
            "constraint"
        };
        let value = if constraint.cardinality.is_empty() {
            constraint.summary.clone()
        } else {
            format!("{} {}", constraint.summary, constraint.cardinality)
        };
        rows.push(row(
            format!("shacl.row:{ordinal}"),
            if constraint.name.is_empty() {
                label.to_owned()
            } else {
                format!("{label} - {}", constraint.name)
            },
            value.trim(),
            if constraint.blank {
                refusals::BLANK_ROOTED
            } else {
                ROWS_ARE_GESTURES
            },
            CONSTRAINTS_GROUP,
        ));
    }
}

/// The query text itself, readable, on a row that says it is not run.
fn add_sparql_queries(rows: &mut Vec<PropertyRow>, model: &RdfModel, card: &ShaclCard) {
    if card.iri.is_empty() {
        return;
    }
    let mut ordinal = 0;
    for constraint in model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(&card.iri) && t.predicate.iri == shacl::SPARQL)
        .map(|t| &t.object)
    {
        let text = model
            .triples
            .iter()
            .find(|t| t.subject == *constraint && t.predicate.iri == shacl::SELECT)
            .map(|t| t.object.as_literal().map(|l| l.lexical.clone()).unwrap_or_default())
            .unwrap_or_default();
        if !text.is_empty() {
            let mut query = row(
                format!("shacl.sparql:{ordinal}"),
                "sh:select",
                text,
                NOT_EXECUTED,
                CONSTRAINTS_GROUP,
            );
            query.editor = PropertyEditor::Text;
            rows.push(query);
            ordinal += 1;
        }
    }
}

fn literals(model: &RdfModel, card: &ShaclCard, predicate: &str) -> Vec<String> {
    if card.iri.is_empty() {
        return Vec::new();
    }
    model
        .triples
        .iter()
        .filter(|t| t.subject.is_iri(&card.iri) && t.predicate.iri == predicate)
        .filter_map(|t| t.object.as_literal().map(|l| l.lexical.clone()))
        .collect()
}

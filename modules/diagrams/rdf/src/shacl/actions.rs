//! The shapes reading's part of the family's context menu (etalii.adp spec 030, task T083), ported from the
//! standalone host's `Shacl/ShaclActions.cs` at `8b6ccfc`.
//!
//! Scoped by origin, deliberately: every reading of the family shares one selection vocabulary, so an element id
//! cannot say which reading is looking at it, and these cases answer only for a target opened as `w3c/shacl`.
//! Availability is the [gate](super::gate)'s, which is also the provider half of the double refusal: it answers without
//! consulting the writer, so a blank-rooted selection is marked unavailable before anything runs.

use adp_context::{Action, ActionGroup, Shortcut};

use crate::commands::RdfEdit;
use crate::family::{self, Run, Target};
use crate::selection;
use crate::shacl::projection::{self, ShaclTargetKind};
use crate::shacl::writer::{ShaclPropertyShapeOptions, count_shape_removal};
use crate::shacl::{ORIGIN, gate, refusals};
use crate::store::RdfDocument;
use crate::term_input;
use crate::vocabulary::shacl;

/// Declare a class target on a shape.
pub const ADD_TARGET_CLASS: &str = "shacl.add-target-class";
/// Declare a node target on a shape.
pub const ADD_TARGET_NODE: &str = "shacl.add-target-node";
/// Add a property row to a shape.
pub const ADD_PROPERTY_ROW: &str = "shacl.add-property-row";
/// Switch a shape off.
pub const DEACTIVATE: &str = "shacl.deactivate";
/// Switch a shape back on.
pub const REACTIVATE: &str = "shacl.reactivate";
/// Remove a shape and the blank subtrees only it reaches.
pub const REMOVE_SHAPE: &str = "shacl.remove-shape";
/// Withdraw one target declaration; the chip's address rides the id after this prefix, `{predicate}|{term}`.
pub const REMOVE_TARGET_PREFIX: &str = "shacl.remove-target|";
/// Create a node shape at a placement.
pub const ADD_NODE_SHAPE: &str = "shacl.add-node-shape";

/// Whether this reading owns the target: its origin, and nothing else's.
fn answers_for(target: &Target) -> bool {
    target.origin == ORIGIN
}

fn offered(id: &str, label: &str, icon: &str, available: bool, reason: &str) -> Action {
    let action = Action::new(id, label).with_icon(icon);
    if available { action } else { action.unavailable(reason) }
}

fn kind_word(kind: ShaclTargetKind) -> &'static str {
    match kind {
        ShaclTargetKind::Class => "class",
        ShaclTargetKind::Node => "node",
        ShaclTargetKind::SubjectsOf => "subjects of",
        ShaclTargetKind::ObjectsOf => "objects of",
        ShaclTargetKind::ImplicitClass => "implicit class",
    }
}

/// The groups this reading leads the menu of `target` with (`Discover`).
pub fn discover(document: &RdfDocument, target: &Target) -> Vec<ActionGroup> {
    if !answers_for(target) {
        return Vec::new();
    }
    if selection::is_placement(target.element_id) {
        return vec![ActionGroup {
            actions: vec![Action::new(ADD_NODE_SHAPE, "Add node shape here…").with_icon("mdi-check-decagram-outline")],
        }];
    }
    let model = &document.model;
    let decision = gate(model, target.element_id, selection::is_truncated(model));
    if !decision.applies {
        return Vec::new();
    }
    let (available, reason) = (decision.available, decision.reason.as_str());
    let projected = projection::project(model, projection::DEFAULT_BUDGET);
    let card = projected.cards.iter().find(|c| c.id == target.element_id);
    let mut actions = vec![
        offered(
            ADD_TARGET_CLASS,
            "Add target class…",
            "mdi-crosshairs",
            available,
            reason,
        ),
        offered(
            ADD_TARGET_NODE,
            "Add target node…",
            "mdi-crosshairs-gps",
            available,
            reason,
        ),
        offered(
            ADD_PROPERTY_ROW,
            "Add property row…",
            "mdi-table-row-plus-after",
            available,
            reason,
        ),
    ];
    if let Some(card) = card {
        actions.push(if card.deactivated {
            offered(
                REACTIVATE,
                "Reactivate shape",
                "mdi-play-circle-outline",
                available,
                reason,
            )
        } else {
            offered(
                DEACTIVATE,
                "Deactivate shape",
                "mdi-pause-circle-outline",
                available,
                reason,
            )
        });
        for chip in card
            .targets
            .iter()
            .filter(|c| !c.predicate_iri.is_empty() && !c.term_iri.is_empty())
        {
            actions.push(offered(
                &format!("{REMOVE_TARGET_PREFIX}{}|{}", chip.predicate_iri, chip.term_iri),
                &format!("Remove target: {} {}", kind_word(chip.kind), chip.term_display),
                "mdi-crosshairs-off",
                available,
                reason,
            ));
        }
        let count = if decision.shape_iri.is_empty() {
            0
        } else {
            count_shape_removal(model, &decision.shape_iri)
        };
        let label = if count > 1 {
            format!("Remove shape (with {count} statements)")
        } else {
            "Remove shape".to_owned()
        };
        actions.push(
            offered(REMOVE_SHAPE, &label, "mdi-delete-outline", available, reason)
                .with_shortcut(Shortcut::key("Delete")),
        );
    }
    vec![ActionGroup { actions }]
}

fn ask(title: &str, icon: &str, placeholder: &str) -> Run {
    Run::Ask(family::input(title, icon, placeholder, "", "Add"))
}

/// Runs `action_id` when it is this reading's, none otherwise (`ExecuteAsync`).
pub fn execute(document: &RdfDocument, target: &Target, action_id: &str) -> Option<Run> {
    if !action_id.starts_with("shacl.") || !answers_for(target) {
        return None;
    }
    let model = &document.model;
    let decision = gate(model, target.element_id, selection::is_truncated(model));
    if decision.applies && !decision.available {
        return Some(Run::Refused(decision.reason));
    }
    let shape = decision.shape_iri;
    Some(match action_id {
        ADD_TARGET_CLASS => ask("Add target class", "mdi-crosshairs", "Class IRI or prefixed name"),
        ADD_TARGET_NODE => ask("Add target node", "mdi-crosshairs-gps", "Node IRI or prefixed name"),
        ADD_PROPERTY_ROW => ask(
            "Add property row",
            "mdi-table-row-plus-after",
            "Path IRI or prefixed name",
        ),
        ADD_NODE_SHAPE => ask(
            "Add node shape",
            "mdi-check-decagram-outline",
            "Shape IRI or prefixed name",
        ),
        DEACTIVATE | REACTIVATE if shape.is_empty() => Run::Refused(refusals::BLANK_ROOTED.to_owned()),
        DEACTIVATE | REACTIVATE => Run::Edit(RdfEdit::SetShaclDeactivated {
            shape,
            deactivated: action_id == DEACTIVATE,
        }),
        REMOVE_SHAPE if shape.is_empty() => Run::Refused(refusals::BLANK_ROOTED.to_owned()),
        REMOVE_SHAPE => Run::Edit(RdfEdit::RemoveShaclShape { shape }),
        _ => {
            let address = action_id.strip_prefix(REMOVE_TARGET_PREFIX)?;
            let parts: Vec<&str> = address.split('|').collect();
            if parts.len() != 2 || shape.is_empty() {
                return Some(Run::Refused(refusals::NO_SUCH_TARGET.to_owned()));
            }
            Run::Edit(RdfEdit::RemoveShaclTarget {
                shape,
                predicate: parts[0].to_owned(),
                term: parts[1].to_owned(),
            })
        }
    })
}

fn asks_for_a_term(action_id: &str) -> bool {
    matches!(
        action_id,
        ADD_TARGET_CLASS | ADD_TARGET_NODE | ADD_PROPERTY_ROW | ADD_NODE_SHAPE
    )
}

/// Judges an answer to this reading's prompt, none for another reading's action (`Validate`).
pub fn validate(document: &RdfDocument, action_id: &str, value: &str) -> Option<Result<(), String>> {
    if !asks_for_a_term(action_id) {
        return None;
    }
    Some(term_input::resolve(&document.model, value).map(|_| ()))
}

/// The edit an answer to this reading's prompt makes, none for another reading's action or an answer that does not
/// resolve (`CommandFor`).
pub fn command_for(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Option<RdfEdit> {
    if !asks_for_a_term(action_id) {
        return None;
    }
    let model = &document.model;
    let iri = term_input::resolve(model, value).ok()?;
    if action_id == ADD_NODE_SHAPE {
        return Some(RdfEdit::CreateShaclNodeShape { shape: iri });
    }
    let shape = gate(model, target.element_id, false).shape_iri;
    if shape.is_empty() {
        return None;
    }
    match action_id {
        ADD_TARGET_CLASS => Some(RdfEdit::AddShaclTarget {
            shape,
            predicate: shacl::TARGET_CLASS.to_owned(),
            term: iri,
        }),
        ADD_TARGET_NODE => Some(RdfEdit::AddShaclTarget {
            shape,
            predicate: shacl::TARGET_NODE.to_owned(),
            term: iri,
        }),
        ADD_PROPERTY_ROW => Some(RdfEdit::AddShaclPropertyRow {
            shape,
            path: iri,
            options: ShaclPropertyShapeOptions::default(),
        }),
        _ => None,
    }
}

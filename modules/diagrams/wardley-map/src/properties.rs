//! What the property grid shows for a selected Wardley element, and what an edit of one of its rows does (etalii.adp
//! spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyContextPropertyProvider.cs` at `8b6ccfc`. Typing `0.42` into Maturity and
//! dragging the component there are the same command and the same undo entry. An absent property has no row at all,
//! so a row that is not here means the document does not say it; inertia, a word whose absence is false, is the one
//! row always shown. The rows that are an action's (the name, the evolution target, inertia) make their command
//! through that action, so a row and a menu entry cannot mean two things. Every read-only row carries its reason as a
//! full sentence, and a write to it is refused with that sentence. On a read-only map every row is shown and none is
//! editable.

use adp_context::{PropertyEditor, PropertyRow};
use adp_diagram::DiagramDocuments;
use adp_history::Command;

use crate::actions::{self, WardleySubject};
use crate::axis;
use crate::commands::{GONE, MoveWardleyElement, SetWardleyDecorators, SetWardleyEvolve, SetWardleyLink};
use crate::identities::{WardleyIdentityEntry, keys, kind};
use crate::lines::{format_number, parse_number};
use crate::model::{
    WardleyElementKind, WardleyLink, WardleyMap, decorator_vocabulary, join_decorators, parse_decorators,
};

pub const NAME: &str = "wardley.name";
pub const KIND: &str = "wardley.kind";
pub const VISIBILITY: &str = "wardley.visibility";
pub const MATURITY: &str = "wardley.maturity";
pub const STAGE: &str = "wardley.stage";
pub const LABEL_OFFSET: &str = "wardley.label-offset";
pub const EVOLVE: &str = "wardley.evolve";
pub const EVOLVE_NAME: &str = "wardley.evolve-name";
pub const INERTIA: &str = "wardley.inertia";
pub const DECORATORS: &str = "wardley.decorators";
const URL: &str = "wardley.url";
pub const LINK_SOURCE: &str = "wardley.link.source";
pub const LINK_TARGET: &str = "wardley.link.target";
pub const LINK_KIND: &str = "wardley.link.kind";
pub const LINK_CONTEXT: &str = "wardley.link.context";
pub const PIPELINE_CHILDREN: &str = "wardley.pipeline.children";

const IDENTITY: &str = "Identity";
const POSITION: &str = "Position";
const STRATEGY: &str = "Strategy";
const LINKS: &str = "Links";

pub const STAGE_REASON: &str =
    "The evolution stage is derived from the component's maturity, so it is not set directly. Change Maturity instead.";
pub const KIND_REASON: &str = "The kind is which statement declares the element, so changing it is a different statement rather than a different value. Remove it and add the kind you want.";
pub const LABEL_OFFSET_REASON: &str = "A label offset is in pixels rather than on the map's scale, which is a property of the format ADP reproduces rather than edits. Change it where the map was written.";
const CHILD_NAME_REASON: &str = "A pipeline component's name is also the only handle the map has on it, and nothing renames one yet. Remove it from the pipeline and add it under the name you want.";
pub const LINK_END_REASON: &str = "The ends of a link are what it is. Remove this link and draw the one you meant.";
pub const LINK_KIND_REASON: &str = "A dependency and a flow are different claims, written with different arrows. Remove this link and draw the other kind.";
const URL_REASON: &str = "A url points at a definition elsewhere in the document, and nothing edits those here yet. Change it where the map was written.";
const CHILDREN_REASON: &str =
    "A pipeline's components are listed here for reading; each one is edited by selecting it.";

fn child_visibility_reason(parent: &str) -> String {
    format!("A pipeline child takes its visibility from its parent, so it is not its own. Change it on '{parent}'.")
}

fn row(id: &str, label: &str, value: impl Into<String>, group: &str) -> PropertyRow {
    let mut row = PropertyRow::new(id, label, PropertyEditor::Line, value);
    row.group = group.to_owned();
    row
}

fn read_only(id: &str, label: &str, value: impl Into<String>, reason: impl Into<String>, group: &str) -> PropertyRow {
    let mut row = row(id, label, value, group);
    row.read_only_reason = reason.into();
    row
}

/// The rows of the element `subject` names; none for the map itself or an element that is gone.
pub fn describe(subject: &WardleySubject) -> Vec<PropertyRow> {
    let Some(entry) = subject.entry() else {
        return Vec::new();
    };
    let map = subject.map;
    let rows = match entry.kind.as_str() {
        kind::COMPONENT => for_component(map, entry),
        kind::PIPELINE_CHILD => for_pipeline_child(map, entry),
        kind::LINK => for_link(map, entry),
        kind::PIPELINE => for_pipeline(map, entry),
        _ => Vec::new(),
    };
    if actions::editable(subject.body_path) {
        rows
    } else {
        rows.into_iter()
            .map(|mut row| {
                row.read_only_reason = actions::READ_ONLY.to_owned();
                row
            })
            .collect()
    }
}

fn spell(kind: WardleyElementKind) -> &'static str {
    kind.keyword()
}

fn for_component(map: &WardleyMap, entry: &WardleyIdentityEntry) -> Vec<PropertyRow> {
    let Some(component) = map.components.iter().find(|c| keys::component(c) == entry.key) else {
        return Vec::new();
    };
    let stage = axis::stage_of(component.position.maturity);
    let evolve = map.evolves.iter().find(|e| e.name == component.name);
    let mut rows = vec![
        row(NAME, "Name", component.name.clone(), IDENTITY),
        read_only(KIND, "Kind", spell(component.kind), KIND_REASON, IDENTITY),
        row(
            VISIBILITY,
            "Visibility",
            format_number(component.position.visibility),
            POSITION,
        ),
        row(
            MATURITY,
            "Maturity",
            format_number(component.position.maturity),
            POSITION,
        ),
        read_only(STAGE, "Evolution stage", stage.label, STAGE_REASON, POSITION),
    ];
    if let Some(offset) = component.label_offset {
        rows.push(read_only(
            LABEL_OFFSET,
            "Label offset",
            format!("{}, {}", format_number(offset.x), format_number(offset.y)),
            LABEL_OFFSET_REASON,
            POSITION,
        ));
    }
    if let Some(evolve) = evolve {
        rows.push(row(EVOLVE, "Evolves to", format_number(evolve.maturity), STRATEGY));
        if !evolve.override_name.is_empty() {
            rows.push(row(EVOLVE_NAME, "Becomes", evolve.override_name.clone(), STRATEGY));
        }
    }
    let mut inertia = row(
        INERTIA,
        "Inertia",
        if component.inertia { "true" } else { "false" },
        STRATEGY,
    );
    inertia.editor = PropertyEditor::Toggle;
    rows.push(inertia);
    rows.push(row(
        DECORATORS,
        "Decorators",
        join_decorators(&component.decorators),
        STRATEGY,
    ));
    if !component.url.is_empty() {
        let address = map.urls.iter().find(|u| u.name == component.url);
        rows.push(read_only(
            URL,
            if component.kind == WardleyElementKind::Submap {
                "Opens"
            } else {
                "Url"
            },
            match address {
                Some(address) => format!("{} ({})", component.url, address.address),
                None => component.url.clone(),
            },
            URL_REASON,
            LINKS,
        ));
    }
    rows
}

fn for_pipeline_child(map: &WardleyMap, entry: &WardleyIdentityEntry) -> Vec<PropertyRow> {
    let Some((pipeline, child)) = map.pipelines.iter().find_map(|pipeline| {
        pipeline
            .children
            .iter()
            .find(|child| keys::pipeline_child(pipeline, child) == entry.key)
            .map(|child| (pipeline, child))
    }) else {
        return Vec::new();
    };
    let parent = map.components.iter().find(|c| c.name == pipeline.parent);
    vec![
        read_only(NAME, "Name", child.name.clone(), CHILD_NAME_REASON, IDENTITY),
        read_only(KIND, "Kind", "pipeline component", KIND_REASON, IDENTITY),
        read_only(
            VISIBILITY,
            "Visibility",
            parent.map(|p| format_number(p.position.visibility)).unwrap_or_default(),
            child_visibility_reason(&pipeline.parent),
            POSITION,
        ),
        row(MATURITY, "Maturity", format_number(child.maturity), POSITION),
        read_only(
            STAGE,
            "Evolution stage",
            axis::stage_of(child.maturity).label,
            STAGE_REASON,
            POSITION,
        ),
    ]
}

fn for_link(map: &WardleyMap, entry: &WardleyIdentityEntry) -> Vec<PropertyRow> {
    let Some(link) = map.links.iter().find(|l| keys::link(l) == entry.key) else {
        return Vec::new();
    };
    let mut rows = vec![
        read_only(LINK_SOURCE, "From", link.source.clone(), LINK_END_REASON, IDENTITY),
        read_only(LINK_TARGET, "To", link.target.clone(), LINK_END_REASON, IDENTITY),
        read_only(
            LINK_KIND,
            "Kind",
            if link.kind == crate::model::WardleyLinkKind::Flow {
                "flow"
            } else {
                "dependency"
            },
            LINK_KIND_REASON,
            IDENTITY,
        ),
    ];
    // Only when the document gives one, and then editable: the author's own words about why the link is there.
    if !link.context.is_empty() {
        rows.push(row(LINK_CONTEXT, "Context", link.context.clone(), IDENTITY));
    }
    rows
}

fn for_pipeline(map: &WardleyMap, entry: &WardleyIdentityEntry) -> Vec<PropertyRow> {
    let Some(pipeline) = map.pipelines.iter().find(|p| keys::pipeline(p) == entry.key) else {
        return Vec::new();
    };
    let parent = map.components.iter().find(|c| c.name == pipeline.parent);
    let mut rows = vec![read_only(NAME, "Name", pipeline.parent.clone(), KIND_REASON, IDENTITY)];
    if let Some(parent) = parent {
        rows.push(read_only(
            VISIBILITY,
            "Visibility",
            format_number(parent.position.visibility),
            child_visibility_reason(&pipeline.parent),
            POSITION,
        ));
    }
    let mut children = read_only(
        PIPELINE_CHILDREN,
        "Components",
        pipeline
            .children
            .iter()
            .map(|c| format!("{} ({})", c.name, format_number(c.maturity)))
            .collect::<Vec<_>>()
            .join(", "),
        CHILDREN_REASON,
        POSITION,
    );
    children.editor = PropertyEditor::Text;
    rows.push(children);
    rows
}

/// A coordinate the person typed: refused rather than clamped, unlike a drag.
fn coordinate(value: &str) -> Result<f64, String> {
    let number = parse_number(value).ok_or_else(|| {
        "This is a number between 0 and 1, where 0 is the invisible end and 1 the user need.".to_owned()
    })?;
    if !(0.0..=1.0).contains(&number) {
        return Err(
            "Both of a Wardley map's axes run from 0 to 1, so a position outside that is off the map.".to_owned(),
        );
    }
    Ok(number)
}

/// The change action `action_id` makes of `value`, refused as the action refuses it.
fn through(
    subject: &WardleySubject,
    action_id: &str,
    value: &str,
) -> Result<Box<dyn Command<DiagramDocuments>>, String> {
    if !actions::applies(subject, action_id) {
        return Err(GONE.to_owned());
    }
    actions::validate(action_id, value)?;
    actions::command(subject, action_id, value).ok_or_else(|| GONE.to_owned())
}

/// Why a write to a row this module does not accept is refused: the sentence the row carries.
fn reason_for(property_id: &str) -> String {
    match property_id {
        STAGE => STAGE_REASON.to_owned(),
        KIND => KIND_REASON.to_owned(),
        LABEL_OFFSET => LABEL_OFFSET_REASON.to_owned(),
        LINK_SOURCE | LINK_TARGET => LINK_END_REASON.to_owned(),
        LINK_KIND => LINK_KIND_REASON.to_owned(),
        URL => URL_REASON.to_owned(),
        PIPELINE_CHILDREN => CHILDREN_REASON.to_owned(),
        VISIBILITY => {
            "A pipeline child takes its visibility from its parent, so it is not its own. Change it on the parent."
                .to_owned()
        }
        _ => format!("'{property_id}' is not a property of this element."),
    }
}

/// The command a write of `value` to row `property_id` of `subject` makes, or why it is refused.
pub fn change(
    subject: &WardleySubject,
    property_id: &str,
    value: &str,
) -> Result<Box<dyn Command<DiagramDocuments>>, String> {
    if !actions::editable(subject.body_path) {
        return Err(actions::READ_ONLY.to_owned());
    }
    let entry = subject.entry().ok_or_else(|| GONE.to_owned())?;
    let body_path = subject.body_path.to_path_buf();
    let component = subject.component();
    let child = subject.child();
    let moved = |visibility: f64, maturity: f64| -> Box<dyn Command<DiagramDocuments>> {
        Box::new(MoveWardleyElement {
            body_path: body_path.clone(),
            element_id: entry.id.clone(),
            visibility,
            maturity,
        })
    };
    match (property_id, component, child) {
        (NAME, Some(_), _) => through(subject, actions::RENAME, value),
        (VISIBILITY, Some(component), _) => Ok(moved(coordinate(value)?, component.position.maturity)),
        (MATURITY, Some(component), _) => Ok(moved(component.position.visibility, coordinate(value)?)),
        // A child's visibility is its parent's, so only its maturity moves.
        (MATURITY, None, Some(_)) => Ok(moved(0.0, coordinate(value)?)),
        // Emptying the row clears the statement.
        (EVOLVE, Some(_), _) if value.trim().is_empty() => through(subject, actions::CLEAR_EVOLVE, ""),
        (EVOLVE, Some(_), _) => through(subject, actions::SET_EVOLVE, value),
        (EVOLVE_NAME, Some(component), _) => match subject.evolve() {
            Some(evolve) => Ok(Box::new(SetWardleyEvolve {
                body_path: body_path.clone(),
                element_id: entry.id.clone(),
                present: true,
                maturity: evolve.maturity,
                override_name: value.trim().to_owned(),
            })),
            None => Err(format!(
                "'{}' is not evolving, so it has no name to arrive under.",
                component.name
            )),
        },
        (INERTIA, Some(_), _) => through(
            subject,
            actions::TOGGLE_INERTIA,
            if value.is_empty() { "false" } else { value },
        ),
        (DECORATORS, Some(_), _) => match parse_decorators(value) {
            Ok(decorators) => Ok(Box::new(SetWardleyDecorators {
                body_path: body_path.clone(),
                element_id: entry.id.clone(),
                decorators,
            })),
            Err(unknown) => Err(format!(
                "'{unknown}' is not one of this notation's decorators. They are: {}.",
                decorator_vocabulary()
            )),
        },
        (LINK_CONTEXT, _, _) => {
            let link: Option<&WardleyLink> = subject.map.links.iter().find(|l| keys::link(l) == entry.key);
            match link {
                Some(link) => Ok(Box::new(SetWardleyLink {
                    body_path: body_path.clone(),
                    source_element_id: subject.id_of(&link.source),
                    target_element_id: subject.id_of(&link.target),
                    kind: link.kind,
                    present: true,
                    context: value.trim().to_owned(),
                })),
                None => Err(GONE.to_owned()),
            }
        }
        _ => Err(reason_for(property_id)),
    }
}

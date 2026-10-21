//! What the standalone module's commands wrote where the specification's generic edit writes otherwise (the
//! standalone host's `GhgContextActionProvider.cs`, `GhgContextPropertyProvider.cs`, `GhgWriter.cs` and
//! `Commands/*` at `8b6ccfc`).
//!
//! - A finished connect gesture (`ghg.connect.influence`) draws an influence after the checks the canvas makes, from
//!   the source's last visible phase, bottom, halfway, to the target's Peak, top, halfway; one from a trigger states
//!   no from end (`AddGhgInfluenceCommandHandler`).
//! - A transaction's changes are written as the commands wrote them ([`written`]): a trend's span scales its dragged
//!   boundaries and keeps every visible phase a month long, a boundary set is clamped likewise, a phase count is
//!   refused when the span is too short for it, tags are trimmed and each kept once, a note's size may carry its new
//!   place, and an influence end is written as its phase, edge and fraction.

use adp_cel::{Map, Value};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{DiagramChange, DiagramSession, DiagramWorkspace};
use adp_disl::{DislChange, DislDiagram, DislElement, IdSource};

use crate::phases::{self, NAMES};
use crate::placement;
use crate::reading::{self, BOUNDARY_ATTRIBUTES, Trend};
use crate::scale;

/// The action that draws an influence for a finished connect gesture.
pub const CONNECT: &str = "ghg.connect.influence";

/// What an edit of something no longer in the graph answers (`GhgRefusals.Gone`).
pub const GONE: &str = "That is no longer in this graph.";

/// Runs the actions the module answers itself; none for the rest, which the specification's behaviour runs.
pub fn execute(
    session: &DiagramSession,
    workspace: &mut DiagramWorkspace,
    target: &ActionTarget,
    action_id: &str,
    ids: &dyn IdSource,
) -> Option<ActionOutcome> {
    if action_id != CONNECT {
        return None;
    }
    let outcome = match target {
        ActionTarget::Connection {
            source: Some(source),
            target: Some(target),
        } => connect(session, workspace, source, target, ids),
        _ => Err("An influence is drawn from one trend to another.".to_owned()),
    };
    Some(match outcome {
        Ok(()) => ActionOutcome::Done,
        Err(refusal) => ActionOutcome::Refused(refusal),
    })
}

/// Writes `changes` to the session's document as one command labelled `label`.
fn write(
    session: &DiagramSession,
    workspace: &mut DiagramWorkspace,
    label: &str,
    changes: Vec<DislChange>,
) -> Result<(), String> {
    let document = workspace
        .documents()
        .get(session.body_path())
        .ok_or_else(|| GONE.to_owned())?;
    if let Some(refusal) = document.refusal() {
        return Err(refusal);
    }
    let changes: Vec<DiagramChange> = changes.into_iter().map(DiagramChange::Disl).collect();
    let edit = document.plan(label, &changes)?;
    workspace.execute(edit)
}

/// Draws an influence from `source` to `target`, checked against the document as it is now.
fn connect(
    session: &DiagramSession,
    workspace: &mut DiagramWorkspace,
    source: &str,
    target: &str,
    ids: &dyn IdSource,
) -> Result<(), String> {
    let document = workspace
        .documents()
        .get(session.body_path())
        .ok_or_else(|| GONE.to_owned())?;
    let diagram = &document.model().diagram;
    let find = |id: &str, type_name: &str| {
        diagram
            .nodes_of_type(type_name)
            .into_iter()
            .find(|n| reading::written_id(n) == id)
    };
    let from_trend = find(source, "Trend");
    let from_trigger = find(source, "Trigger");
    let to_trend = find(target, "Trend");
    if find(target, "Trigger").is_some() && to_trend.is_none() {
        return Err("An influence cannot end at a trigger.".to_owned());
    }
    let Some(to) = to_trend else {
        return Err("An influence is drawn from one trend to another.".to_owned());
    };
    let Some(from) = from_trend.clone().or(from_trigger) else {
        return Err("An influence is drawn from one trend to another.".to_owned());
    };
    if source == target {
        return Err("A trend cannot influence itself.".to_owned());
    }
    let repeated = diagram.relations_of_type("Influence").iter().any(|influence| {
        influence.source_id().as_deref() == Some(source) && influence.target_id().as_deref() == Some(target)
    });
    if repeated {
        return Err(
            "This trend already influences that one; a trend influences another once in each direction.".to_owned(),
        );
    }
    let id = ids.next("Influence")?;
    if diagram.element_by_id(&id).is_some() {
        return Err("That id is already used in this graph.".to_owned());
    }
    let mut attributes = Map::default();
    if let Some(trend) = &from_trend {
        let phases = Trend::of(trend).visible_phases();
        attributes.insert("fromPhase", Value::from(NAMES[(phases - 1) as usize]));
        attributes.insert("fromEdge", Value::from("bottom"));
        attributes.insert("fromAt", Value::Double(0.5));
    }
    attributes.insert("toPhase", Value::from(NAMES[0]));
    attributes.insert("toEdge", Value::from("top"));
    attributes.insert("toAt", Value::Double(0.5));
    let change = DislChange::Connect {
        type_name: "Influence".to_owned(),
        id,
        source_id: from.id(),
        target_id: to.id(),
        attributes,
    };
    write(session, workspace, "Influence", vec![change])
}

/// The changes of a transaction on `diagram` as the standalone module's commands wrote them, or why they may not be.
pub fn written(diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
    changes
        .into_iter()
        .map(|change| match change {
            DislChange::Set {
                element_id,
                type_name,
                attributes,
            } => {
                let element = diagram.element_by_id(&element_id);
                let attributes = match &element {
                    Some(element) => set(element, attributes)?,
                    None => attributes,
                };
                Ok(DislChange::Set {
                    element_id,
                    type_name,
                    attributes,
                })
            }
            other => Ok(other),
        })
        .collect()
}

/// A set of `attributes` on `element`, as the command of the row or gesture that sets them wrote it.
fn set(element: &DislElement, mut attributes: Map) -> Result<Map, String> {
    if let Some(Value::String(name)) = attributes.get("name")
        && (element.is_a("Trend") || element.is_a("Trigger"))
    {
        // A name is trimmed, and none is refused (`GhgWriter.SetName`).
        let name = name.trim().to_owned();
        if name.is_empty() {
            let what = if element.is_a("Trigger") { "trigger" } else { "trend" };
            return Err(format!("A {what} needs a name."));
        }
        attributes.insert("name", Value::String(name));
    }
    if let Some(Value::String(text)) = attributes.get("text")
        && element.is_a("Note")
    {
        // Line breaks as LF, trailing ones dropped: FBL writes the body's own ending (`GhgWriter.Normalised`).
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        attributes.insert("text", Value::String(text.trim_end_matches('\n').to_owned()));
    }
    if element.is_a("Trend") {
        return set_trend(element, attributes);
    }
    if element.is_a("Trigger") {
        return Ok(with_tags_once(attributes));
    }
    Ok(attributes)
}

/// A set on a trend: a span scales the dragged boundaries, a boundary is clamped with the others, a phase count is
/// refused when the span is too short for it, and tags are kept once each.
fn set_trend(element: &DislElement, attributes: Map) -> Result<Map, String> {
    let trend = Trend::of(element);
    let mut attributes = with_tags_once(attributes);
    let boundaries: Vec<usize> = BOUNDARY_ATTRIBUTES
        .iter()
        .enumerate()
        .filter(|(_, name)| attributes.contains_key(name))
        .map(|(index, _)| index)
        .collect();
    let month = |name: &str| match attributes.get(name) {
        Some(Value::Int(month)) => Some(*month),
        Some(Value::String(text)) => scale::parse_month(text),
        _ => None,
    };
    let (start, stop) = (month("start"), month("stop"));
    if (start.is_some() || stop.is_some()) && boundaries.is_empty() {
        // A date typed in the grid: the span, with every dragged boundary scaled to it (`SetGhgSpanCommandHandler`).
        let Some((was_start, was_stop)) = trend.span() else {
            return Err(placement::NO_SPAN.to_owned());
        };
        let spanned = placement::span(&trend, start.unwrap_or(was_start), stop.unwrap_or(was_stop))?;
        for (name, value) in spanned.iter() {
            attributes.insert(name.clone(), value.clone());
        }
        return Ok(attributes);
    }
    if let [index] = boundaries.as_slice()
        && start.is_none()
        && stop.is_none()
        && let Some(month) = month(BOUNDARY_ATTRIBUTES[*index])
    {
        // One boundary typed or dragged: clamped so every phase stays a month long (`SetGhgBoundaryCommandHandler`).
        let Some((start, stop)) = trend.span() else {
            return Err(placement::NO_SPAN.to_owned());
        };
        if *index as i64 >= trend.visible_phases() - 1 {
            return Err("Only a boundary between two visible phases can be moved.".to_owned());
        }
        let dragged = phases::with_boundary(&trend.dragged, *index, month, start, stop, trend.visible_phases());
        placement::boundaries_into(&mut attributes, &dragged);
        return Ok(attributes);
    }
    let count = match attributes.get("phases") {
        Some(Value::Int(count)) => Some(*count),
        Some(Value::String(text)) => text.trim().parse().ok(),
        _ => None,
    };
    if let Some(count) = count
        && trend.span().is_some()
        && let Some(refusal) = phases::too_short(trend.months(), count)
    {
        return Err(refusal);
    }
    Ok(attributes)
}

/// `attributes` with its tags, when it sets them, each kept once, in order (`SetGhgTagsCommand.Parsed`).
fn with_tags_once(mut attributes: Map) -> Map {
    if let Some(Value::List(tags)) = attributes.get("tags") {
        let mut seen = Vec::new();
        for tag in tags.iter() {
            if !seen.contains(tag) {
                seen.push(tag.clone());
            }
        }
        attributes.insert("tags", Value::list(seen));
    }
    attributes
}

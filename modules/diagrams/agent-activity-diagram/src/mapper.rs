//! What the canvas draws of an activity file (etalii.adp spec 030, task T075): one element per project,
//! specification, agent, location and environment, one per relation, and one that carries what is set for the diagram
//! as a whole; each element where it is locked or where the radiating layout puts it.
//!
//! Ported from the standalone host's `src/diagrams/agent-activity-diagram/backend/EtAlii.Adp.Diagram.AgentActivityDiagram/
//! AadElementMapper.cs` and the client's `AadCanvas.tsx` at `8b6ccfc`. The standalone host sent only a locked element's
//! position and left the others to its canvas's `tieredForceLayout.ts`; the desktop host's core has that layout
//! (`adp_layout::TieredForceLayout`, task T054), so every element is handed over at its centre, which keeps the canvas a
//! renderer. **An archived specification is left out unless the switch is on**, with the relations at either end of it
//! and nothing else. **A relation whose end is of the wrong type is still a relation here**, flagged as a breach so the
//! canvas draws it marked. A card's size is the definition's before its lists, grown by the rows it shows.

use std::collections::HashMap;
use std::rc::Rc;

use adp_diagram::{DiagramElement, ElementKind};
use adp_layout::{
    LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, Placements,
    TieredForceLayout,
};
use serde_json::Value as Json;

use crate::actions::VIEW_ID;
use crate::definition::{self, PULL_REQUESTS_GROUP};
use crate::model::{AadElement, AadKind, AadModel, AadRow};
use crate::store::AadDocument;

/// What precedes an element's or a relation's type on the wire: the tool type's origin.
pub const PREFIX: &str = "etalii/agent-activity-diagram+";
pub const RELATION_TYPE: &str = "etalii/agent-activity-diagram+relation";
pub const VIEW_TYPE: &str = "etalii/agent-activity-diagram+view";

/// The member name of the status that hides a specification.
const ARCHIVED: &str = "archived";

/// The height of a list's heading, and of one of its rows.
pub const HEADING_HEIGHT: f64 = 22.0;
pub const ROW_HEIGHT: f64 = 18.0;

/// The wire type of a kind of element.
pub fn type_of(kind: AadKind) -> String {
    format!("{PREFIX}{}", kind.wire())
}

/// The groups of `element`'s rows as the card lists them: a specification's tasks by status, in the definition's
/// order, a location's pull requests as one group; each group's rows most recently updated first, those without a
/// moment after in file order; a group with no row is left out.
pub fn groups(element: &AadElement) -> Vec<(String, Vec<&AadRow>)> {
    fn ordered(mut rows: Vec<&AadRow>) -> Vec<&AadRow> {
        rows.sort_by(|a, b| match (a.updated.is_empty(), b.updated.is_empty()) {
            (false, false) => b.updated.cmp(&a.updated),
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (true, true) => std::cmp::Ordering::Equal,
        });
        rows
    }
    match element.kind {
        AadKind::Specification => definition::get()
            .task_status()
            .members
            .iter()
            .map(|status| {
                (
                    status.name.clone(),
                    ordered(element.rows.iter().filter(|r| r.status == status.name).collect()),
                )
            })
            .filter(|(_, rows)| !rows.is_empty())
            .collect(),
        AadKind::Location if !element.rows.is_empty() => {
            vec![(PULL_REQUESTS_GROUP.to_owned(), ordered(element.rows.iter().collect()))]
        }
        _ => Vec::new(),
    }
}

/// The size a card is drawn at: the definition's, grown by the folder line and the rows of each unfolded group, to
/// the most the definition allows.
fn size_of(element: &AadElement, model: &AadModel) -> (f64, f64) {
    let definition = definition::get();
    let (width, height) = definition.size_of(element.kind.type_name());
    let collapsed = model.collapsed.get(&element.id);
    let mut grown = height;
    if element.kind == AadKind::Location {
        grown += ROW_HEIGHT;
    }
    for (group, rows) in groups(element) {
        grown += HEADING_HEIGHT;
        if !collapsed.is_some_and(|c| c.contains(&group)) {
            grown += ROW_HEIGHT * rows.len() as f64;
        }
    }
    let most = definition
        .specification()
        .root()
        .pointer(&format!("/notation/nodes/{}/size/max/1", element.kind.type_name()))
        .and_then(Json::as_f64)
        .unwrap_or(grown);
    (width, grown.min(most))
}

/// The elements shown: an archived specification only while the switch is on.
fn shown(model: &AadModel) -> Vec<&AadElement> {
    model
        .elements
        .iter()
        .filter(|e| model.show_archived || e.kind != AadKind::Specification || e.status != ARCHIVED)
        .collect()
}

/// The rings of the definition's `force` layout, from the centre outwards.
fn tiers() -> Vec<Vec<String>> {
    let definition = definition::get();
    let root = definition.specification().root();
    let default = root
        .pointer("/layout/default")
        .and_then(Json::as_str)
        .unwrap_or("rings");
    root.pointer(&format!("/layout/algorithms/{default}/force/tiers"))
        .and_then(Json::as_array)
        .map(|tiers| {
            tiers
                .iter()
                .map(|t| match t {
                    Json::Array(types) => types.iter().filter_map(Json::as_str).map(str::to_owned).collect(),
                    other => other.as_str().map(|s| vec![s.to_owned()]).unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Where the radiating layout puts every element nobody locked, starting from where it last placed them.
fn placements(model: &AadModel, previous: Option<&Placements>) -> Placements {
    let elements = shown(model);
    let ids: std::collections::HashSet<&str> = elements.iter().map(|e| e.id.as_str()).collect();
    let input = LayoutInput {
        elements: elements
            .iter()
            .map(|e| {
                let (width, height) = size_of(e, model);
                let pinned = model.placements.get(&e.id);
                LayoutElement {
                    element_type: Some(e.kind.type_name().to_owned()),
                    pinned: pinned.is_some(),
                    ..LayoutElement::new(
                        e.id.clone(),
                        pinned.map_or(0.0, |p| p.x),
                        pinned.map_or(0.0, |p| p.y),
                        width,
                        height,
                    )
                }
            })
            .collect(),
        connections: model
            .relations
            .iter()
            .filter(|r| ids.contains(r.from.as_str()) && ids.contains(r.to.as_str()))
            .map(|r| LayoutConnection::new(r.from.clone(), r.to.clone()))
            .collect(),
    };
    let definition = LayoutDefinition {
        tiers: tiers(),
        ..LayoutDefinition::with_modes(&[LayoutMode::TieredForce])
    };
    TieredForceLayout
        .place(&input, &definition, previous)
        .unwrap_or_default()
}

/// The placements of `document`'s model, computed once per reading and warm-started from the last.
fn placed(document: &AadDocument) -> Rc<Placements> {
    let mut cache = document.placed.borrow_mut();
    if let Some((model, placements)) = cache.as_ref()
        && Rc::ptr_eq(model, &document.model)
    {
        return placements.clone();
    }
    let previous = cache.as_ref().map(|(_, p)| p.clone());
    let placements = Rc::new(placements(&document.model, previous.as_deref()));
    *cache = Some((document.model.clone(), placements.clone()));
    placements
}

/// Every element the canvas draws of `document`, at its centre.
pub fn visible(document: &AadDocument) -> Vec<DiagramElement> {
    let model = &document.model;
    let placements = placed(document);
    let definition = definition::get();
    let elements = shown(model);
    let kinds: HashMap<&str, AadKind> = elements.iter().map(|e| (e.id.as_str(), e.kind)).collect();

    let mut drawn = Vec::with_capacity(elements.len() + model.relations.len() + 1);
    let mut view = DiagramElement::node(VIEW_ID, VIEW_TYPE);
    view.attributes
        .push(("showArchived".to_owned(), model.show_archived.to_string()));
    drawn.push(view);

    for element in &elements {
        let pinned = model.placements.get(&element.id);
        let (x, y) = match pinned {
            Some(p) => (p.x, p.y),
            None => placements.get(&element.id).map_or((0.0, 0.0), |p| (p.x, p.y)),
        };
        let (width, height) = size_of(element, model);
        let collapsed: Vec<&str> = model
            .collapsed
            .get(&element.id)
            .map(|c| c.iter().map(String::as_str).collect())
            .unwrap_or_default();
        let rows: Vec<String> = groups(element)
            .into_iter()
            .flat_map(|(group, rows)| {
                rows.into_iter().map(move |r| {
                    [group.as_str(), &r.id, &r.title, &r.updated, &r.link]
                        .map(|f| f.replace(['\t', '\n'], " "))
                        .join("\t")
                })
            })
            .collect();
        let mut node = DiagramElement::node(element.id.clone(), type_of(element.kind));
        node.x = x;
        node.y = y;
        node.placed = pinned.is_some();
        node.attributes = vec![
            ("name".to_owned(), element.name.clone()),
            ("status".to_owned(), element.status.clone()),
            ("statusLabel".to_owned(), element.status_label.clone()),
            ("link".to_owned(), element.link.clone()),
            ("folder".to_owned(), element.folder.clone()),
            ("branchLink".to_owned(), element.branch_link.clone()),
            ("folderLink".to_owned(), element.folder_link.clone()),
            ("kindLabel".to_owned(), element.kind_label.clone()),
            ("pinned".to_owned(), pinned.is_some().to_string()),
            ("width".to_owned(), width.to_string()),
            ("height".to_owned(), height.to_string()),
            ("collapsed".to_owned(), collapsed.join(",")),
            ("rows".to_owned(), rows.join("\n")),
        ];
        drawn.push(node);
    }

    for relation in &model.relations {
        let (Some(from), Some(to)) = (kinds.get(relation.from.as_str()), kinds.get(relation.to.as_str())) else {
            continue;
        };
        let mut line = DiagramElement::node(relation.id.clone(), RELATION_TYPE);
        line.kind = ElementKind::Relation;
        line.source = Some(relation.from.clone());
        line.target = Some(relation.to.clone());
        // A key that names an element of another type than its relation joins is kept, drawn and marked.
        let breach = !definition.joins(&relation.type_name, from.type_name(), to.type_name());
        line.attributes = vec![
            ("type".to_owned(), relation.type_name.clone()),
            ("breach".to_owned(), breach.to_string()),
        ];
        drawn.push(line);
    }
    drawn
}

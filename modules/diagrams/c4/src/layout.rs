//! Where a view's elements go (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4Layout.cs` at `8b6ccfc`. The
//! arrangement itself is the capability `layout.rankedBoundaries`, moved to `adp-atd-runtime` (etalii.adp spec 067,
//! T026); what stays here is what is the C4 model's: which elements a view ranks along which relationships, how big a
//! card is, what a boundary is labelled with, and that a document's `autoLayout` sets the direction and separations and
//! makes stored positions ignored. A deployment view nests every node around what it hosts.

use adp_atd_runtime::capabilities::ranked_boundaries::{self, RankedArguments, RankedNode};
use adp_layout::rect::Rect;
use adp_layout::text_metric;

use crate::metrics::C4Metrics;
use crate::model::{C4Element, C4ElementKind, C4Relationship, C4View, C4ViewKind, C4Workspace, folded, same};
use crate::rules;
use crate::sidecar::Positions;

pub use adp_atd_runtime::capabilities::ranked_boundaries::Boxes;

/// A boundary drawn around the elements inside one scope.
#[derive(Debug, Clone, PartialEq)]
pub struct C4Boundary {
    /// A synthetic id, `boundary:<scope>`, since a boundary is drawn from an element rather than declared.
    pub id: String,
    /// The scope element's name.
    pub name: String,
    /// What the boundary encloses, for its label: "Software System", "Container" or a deployment node's kind.
    pub kind: String,
    pub rect: Rect,
}

/// Where everything on one view goes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct C4Layout {
    pub boxes: Boxes,
    pub boundaries: Vec<C4Boundary>,
    /// Whether the view was arranged by hand but the document declares `autoLayout`, which won.
    pub authored_positions_ignored: bool,
}

/// Computes where `view`'s elements go; `authored` positions, from the sidecar, win over the computed ones unless the
/// document declares `autoLayout`.
pub fn compute(workspace: &C4Workspace, view: &C4View, metrics: &C4Metrics, authored: Option<&Positions>) -> C4Layout {
    let members = rules::members_of(workspace, view);
    if members.is_empty() {
        return C4Layout::default();
    }
    let auto = view.auto_layout.as_ref();
    let arguments = RankedArguments {
        nest: view.kind == C4ViewKind::Deployment,
        direction: auto.map_or("tb".to_owned(), |a| a.direction.clone()),
        rank_separation: f64::from(
            auto.and_then(|a| a.rank_separation)
                .unwrap_or(metrics.rank_separation as i32),
        ),
        node_separation: f64::from(
            auto.and_then(|a| a.node_separation)
                .unwrap_or(metrics.node_separation as i32),
        ),
        pins_ignored: auto.is_some(),
        padding: metrics.boundary_padding,
        label_height: metrics.font_size * metrics.line_height,
        clearance: metrics.node_separation,
    };
    let (nodes, edges, labels) = if arguments.nest {
        nested_input(workspace, &members, metrics, authored)
    } else {
        enclosed_input(workspace, view, &members, metrics, authored)
    };
    let placed = ranked_boundaries::place_nodes(&nodes, &edges, &arguments);
    let has_authored = authored.is_some_and(|a| !a.is_empty() && a.keys().any(|id| placed.boxes.contains(id)));
    let boundaries = placed
        .boundaries
        .iter()
        .filter_map(|(id, rect)| {
            let (name, kind) = labels
                .iter()
                .find(|(boundary, _, _)| same(boundary, id))
                .map(|(_, n, k)| (n, k))?;
            Some(C4Boundary {
                id: id.clone(),
                name: name.clone(),
                kind: kind.clone(),
                rect: *rect,
            })
        })
        .collect();
    C4Layout {
        boxes: placed.boxes,
        boundaries,
        authored_positions_ignored: has_authored && auto.is_some(),
    }
}

/// What a set of boundaries is drawn with, by its node's id: its name and its kind.
type BoundaryLabels = Vec<(String, String, String)>;

/// The nodes of a view other than a deployment's: its members, and the boundary a container or component view draws
/// around its scope's children; and the relationships it ranks along (the interactions, on a dynamic view).
fn enclosed_input(
    workspace: &C4Workspace,
    view: &C4View,
    members: &[&C4Element],
    metrics: &C4Metrics,
    authored: Option<&Positions>,
) -> (Vec<RankedNode>, Vec<(String, String)>, BoundaryLabels) {
    let scope = view
        .scope_id
        .as_deref()
        .filter(|_| matches!(view.kind, C4ViewKind::Container | C4ViewKind::Component))
        .and_then(|id| workspace.find(id))
        .filter(|scope| {
            members
                .iter()
                .any(|e| e.parent_id.as_deref().is_some_and(|p| same(p, &scope.id)))
        });
    let boundary = scope.map(|scope| format!("boundary:{}", scope.id));
    let mut nodes: Vec<RankedNode> = members
        .iter()
        .map(|element| {
            let size = measure_of(workspace, element, metrics);
            let inside = scope.is_some_and(|scope| element.parent_id.as_deref().is_some_and(|p| same(p, &scope.id)));
            RankedNode {
                id: element.id.clone(),
                width: size.width,
                height: size.height,
                parent: boundary.clone().filter(|_| inside),
                boundary: false,
                pinned: authored.and_then(|a| a.get(&element.id)),
            }
        })
        .collect();
    let mut labels = Vec::new();
    if let (Some(scope), Some(boundary)) = (scope, boundary) {
        nodes.push(RankedNode {
            id: boundary.clone(),
            width: 0.0,
            height: 0.0,
            parent: None,
            boundary: true,
            pinned: None,
        });
        let kind = if view.kind == C4ViewKind::Container {
            "Software System"
        } else {
            "Container"
        };
        labels.push((boundary, scope.name.clone(), kind.to_owned()));
    }
    let edges = if view.kind == C4ViewKind::Dynamic {
        view.interactions
            .iter()
            .map(|i| (i.source_id.clone(), i.destination_id.clone()))
            .collect()
    } else {
        workspace
            .relationships
            .iter()
            .map(|r| (r.source_id.clone(), r.destination_id.clone()))
            .collect()
    };
    (nodes, edges, labels)
}

/// The nodes of a deployment view: every node that hosts something on the view a boundary around what it hosts, to any
/// depth; and the relationships it ranks along. A position the user arranged is honoured only for an element no node
/// encloses.
fn nested_input(
    workspace: &C4Workspace,
    members: &[&C4Element],
    metrics: &C4Metrics,
    authored: Option<&Positions>,
) -> (Vec<RankedNode>, Vec<(String, String)>, BoundaryLabels) {
    let by_id = |id: &str| members.iter().copied().find(|e| same(&e.id, id));
    // The nearest ancestor that is on the view.
    let host_of = |element: &C4Element| -> Option<String> {
        let mut parent = element.parent_id.clone();
        while let Some(id) = parent {
            if let Some(found) = by_id(&id) {
                return Some(found.id.clone());
            }
            parent = workspace.find(&id).and_then(|e| e.parent_id.clone());
        }
        None
    };
    let hosts: Vec<Option<String>> = members.iter().map(|e| host_of(e)).collect();
    let hosting = |element: &C4Element| hosts.iter().flatten().any(|h| same(h, &element.id));
    let mut labels = Vec::new();
    let nodes = members
        .iter()
        .zip(&hosts)
        .map(|(element, host)| {
            let parent = host.as_ref().map(|h| format!("boundary:{h}"));
            if hosting(element) {
                let id = format!("boundary:{}", element.id);
                labels.push((id.clone(), element.name.clone(), boundary_kind_of(element)));
                let label_width = text_metric::width_of(&boundary_label_of(element), metrics.font_size)
                    + 2.0 * metrics.horizontal_padding;
                return RankedNode {
                    id,
                    width: label_width,
                    height: 0.0,
                    parent,
                    boundary: true,
                    pinned: None,
                };
            }
            let size = measure_of(workspace, element, metrics);
            RankedNode {
                id: element.id.clone(),
                width: size.width,
                height: size.height,
                parent,
                boundary: false,
                pinned: authored.and_then(|a| a.get(&element.id)),
            }
        })
        .collect();
    let edges = deployment_relationships_of(workspace, members)
        .into_iter()
        .map(|r| (r.source_id, r.destination_id))
        .collect();
    (nodes, edges, labels)
}

/// The element as it is drawn: an instance shows the name, description and technology of what it instantiates.
pub fn displayed(workspace: &C4Workspace, element: &C4Element) -> C4Element {
    let referenced = element
        .referenced_id
        .as_deref()
        .filter(|_| element.kind.is_instance())
        .and_then(|id| workspace.find(id));
    let Some(referenced) = referenced else {
        return element.clone();
    };
    let or = |own: &str, theirs: &str| {
        if own.is_empty() {
            theirs.to_owned()
        } else {
            own.to_owned()
        }
    };
    C4Element {
        name: or(&element.name, &referenced.name),
        description: or(&element.description, &referenced.description),
        technology: or(&element.technology, &referenced.technology),
        ..element.clone()
    }
}

/// The relationships a deployment view draws between `members`: the ones declared between deployment elements, and
/// the ones implied by instances, from every instance of a relationship's source to every instance of its
/// destination, each pair once.
pub fn deployment_relationships_of(workspace: &C4Workspace, members: &[&C4Element]) -> Vec<C4Relationship> {
    let ids: Vec<String> = members.iter().map(|e| folded(&e.id)).collect();
    let is_member = |id: &str| ids.contains(&folded(id));
    let instances_of = |id: &str| -> Vec<&str> {
        members
            .iter()
            .filter(|e| e.referenced_id.as_deref().is_some_and(|r| same(r, id)))
            .map(|e| e.id.as_str())
            .collect()
    };
    let mut drawn: Vec<(String, String)> = Vec::new();
    let mut result = Vec::new();
    for relationship in &workspace.relationships {
        if is_member(&relationship.source_id) && is_member(&relationship.destination_id) {
            let pair = (relationship.source_id.clone(), relationship.destination_id.clone());
            if !drawn.contains(&pair) {
                drawn.push(pair);
                result.push(relationship.clone());
            }
            continue;
        }
        for source in instances_of(&relationship.source_id) {
            for destination in instances_of(&relationship.destination_id) {
                let pair = (source.to_owned(), destination.to_owned());
                if !same(source, destination) && !drawn.contains(&pair) {
                    drawn.push(pair);
                    result.push(C4Relationship {
                        source_id: source.to_owned(),
                        destination_id: destination.to_owned(),
                        ..relationship.clone()
                    });
                }
            }
        }
    }
    result
}

/// The bracketed line under an element's name: its type, and its technology where it has one.
pub fn type_line_of(element: &C4Element) -> String {
    let kind = match element.kind {
        C4ElementKind::Person => "Person",
        C4ElementKind::SoftwareSystem | C4ElementKind::SoftwareSystemInstance => "Software System",
        C4ElementKind::Container | C4ElementKind::ContainerInstance => "Container",
        C4ElementKind::Component => "Component",
        C4ElementKind::DeploymentNode => "Deployment Node",
        C4ElementKind::InfrastructureNode => "Infrastructure Node",
    };
    if element.technology.is_empty() {
        format!("[{kind}]")
    } else {
        format!("[{kind}: {}]", element.technology)
    }
}

/// The box an element is drawn in, before it is placed: its displayed name, type line and description measured.
pub fn measure_of(workspace: &C4Workspace, element: &C4Element, metrics: &C4Metrics) -> Rect {
    let shown = displayed(workspace, element);
    metrics.measure(&shown.name, &type_line_of(&shown), &shown.description)
}

/// What a deployment node's boundary says after its name, in its brackets.
fn boundary_kind_of(element: &C4Element) -> String {
    if element.technology.is_empty() {
        "Deployment Node".to_owned()
    } else {
        format!("Deployment Node: {}", element.technology)
    }
}

/// What a deployment node's boundary is labelled with: its name and, in brackets, its kind.
pub fn boundary_label_of(element: &C4Element) -> String {
    format!("{} [{}]", element.name, boundary_kind_of(element))
}

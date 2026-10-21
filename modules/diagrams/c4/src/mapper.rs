//! Turns a laid-out view into the elements a session hands the shell (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4ElementMapper.cs` at
//! `8b6ccfc`. Elements, relationships, boundaries and the view's own furniture (its title and legend) all travel as
//! [`DiagramElement`]s; what the standalone host put in a protobuf payload (`src/diagrams/c4/api/c4.proto`) is here an
//! element's attributes, as text. A node sits at the centre of its box. A node whose box the viewport touches is in
//! view, and brings the far end of each of its relationships with it; relationships are elevated to the level the
//! view shows, a deployment view draws what its instances imply, and a dynamic view draws its interactions.

use std::path::Path;

use adp_diagram::element::Rect as ViewportRect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};
use adp_fbl::planning::dotnet_double;
use adp_layout::rect::Rect;

use crate::layout::{self, C4Boundary, C4Layout};
use crate::metrics::C4Metrics;
use crate::model::{C4Element, C4Relationship, C4View, C4ViewKind, C4Workspace, folded, same};
use crate::rules;
use crate::sidecar;
use crate::theme;

/// The element types C4 puts on the wire.
pub const NODE_TYPE: &str = "c4/model+node";
pub const RELATIONSHIP_TYPE: &str = "c4/model+relationship";
pub const BOUNDARY_TYPE: &str = "c4/model+boundary";
pub const VIEW_TYPE: &str = "c4/model+view";

/// The id of the element carrying the view's title and legend.
pub const VIEW_ELEMENT_ID: &str = "c4:view";

/// The layout of `view`, with the positions arranged by hand in the sidecar beside `body_path` laid over it.
pub fn layout_of(workspace: &C4Workspace, view: &C4View, metrics: &C4Metrics, body_path: Option<&Path>) -> C4Layout {
    let authored = body_path.map(|path| sidecar::read(path, &view.key));
    layout::compute(workspace, view, metrics, authored.as_ref())
}

/// Everything a connection sees of `view` inside `viewport`.
pub fn visible(
    workspace: &C4Workspace,
    view: &C4View,
    viewport: DiagramViewport,
    metrics: &C4Metrics,
    body_path: Option<&Path>,
) -> Vec<DiagramElement> {
    let layout = layout_of(workspace, view, metrics, body_path);
    let members = rules::members_of(workspace, view);
    let shown: Vec<&C4Element> = members
        .into_iter()
        .filter(|element| layout.boxes.contains(&element.id))
        .collect();
    let in_view = |element: &C4Element| {
        let rect = layout.boxes.get(&element.id).unwrap_or_default();
        viewport.intersects(ViewportRect::new(rect.x, rect.y, rect.width, rect.height))
    };
    let relationships = relationships_of(workspace, view, &layout);

    // One hop: whatever an on-screen element's line reaches.
    let mut delivered: Vec<String> = shown
        .iter()
        .filter(|element| in_view(element))
        .map(|element| folded(&element.id))
        .collect();
    for (relationship, _, _, _) in &relationships {
        let (source, destination) = (folded(&relationship.source_id), folded(&relationship.destination_id));
        if delivered.contains(&source) || delivered.contains(&destination) {
            if !delivered.contains(&source) {
                delivered.push(source);
            }
            if !delivered.contains(&destination) {
                delivered.push(destination);
            }
        }
    }
    let is_delivered = |id: &str| delivered.contains(&folded(id));

    let mut elements = Vec::new();
    for element in shown.iter().filter(|element| is_delivered(&element.id)) {
        let rect = layout.boxes.get(&element.id).unwrap_or_default();
        elements.push(node(&layout::displayed(workspace, element), rect, workspace));
    }
    for (relationship, source, destination, order) in &relationships {
        if is_delivered(&relationship.source_id) && is_delivered(&relationship.destination_id) {
            elements.push(line(relationship, *source, *destination, order));
        }
    }
    for boundary in &layout.boundaries {
        elements.push(boundary_element(boundary));
    }
    elements.push(view_element(workspace, view));
    elements
}

fn number(value: f64) -> String {
    dotnet_double(value)
}

fn attributes(pairs: &[(&str, String)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect()
}

fn node(element: &C4Element, rect: Rect, workspace: &C4Workspace) -> DiagramElement {
    let style = theme::apply(&theme::style_for(element), element, &workspace.styles);
    DiagramElement {
        id: element.id.clone(),
        x: rect.center_x(),
        y: rect.center_y(),
        placed: true,
        element_type: NODE_TYPE.to_owned(),
        kind: ElementKind::Node,
        parent: element.parent_id.clone(),
        source: None,
        target: None,
        attributes: attributes(&[
            ("kind", element.kind.name().to_owned()),
            ("name", element.name.clone()),
            ("description", element.description.clone()),
            ("technology", element.technology.clone()),
            ("typeLine", layout::type_line_of(element)),
            ("external", element.is_external().to_string()),
            ("parentId", element.parent_id.clone().unwrap_or_default()),
            ("width", number(rect.width)),
            ("height", number(rect.height)),
            ("background", style.background),
            ("color", style.color),
            ("shape", style.shape),
            ("border", style.border),
        ]),
    }
}

fn line(relationship: &C4Relationship, source: Rect, destination: Rect, order: &str) -> DiagramElement {
    DiagramElement {
        id: relationship.id(),
        x: (source.center_x() + destination.center_x()) / 2.0,
        y: (source.center_y() + destination.center_y()) / 2.0,
        placed: true,
        element_type: RELATIONSHIP_TYPE.to_owned(),
        kind: ElementKind::Relation,
        parent: None,
        source: Some(relationship.source_id.clone()),
        target: Some(relationship.destination_id.clone()),
        attributes: attributes(&[
            ("description", relationship.description.clone()),
            ("technology", relationship.technology.clone()),
            ("interactionOrder", order.to_owned()),
            ("sourceX", number(source.center_x())),
            ("sourceY", number(source.center_y())),
            ("sourceWidth", number(source.width)),
            ("sourceHeight", number(source.height)),
            ("destinationX", number(destination.center_x())),
            ("destinationY", number(destination.center_y())),
            ("destinationWidth", number(destination.width)),
            ("destinationHeight", number(destination.height)),
        ]),
    }
}

fn boundary_element(boundary: &C4Boundary) -> DiagramElement {
    DiagramElement {
        x: boundary.rect.center_x(),
        y: boundary.rect.center_y(),
        placed: true,
        attributes: attributes(&[
            ("name", boundary.name.clone()),
            ("kind", boundary.kind.clone()),
            ("width", number(boundary.rect.width)),
            ("height", number(boundary.rect.height)),
        ]),
        ..DiagramElement::node(boundary.id.clone(), BOUNDARY_TYPE)
    }
}

/// The view's own element: its title and the key to the notation in use.
fn view_element(workspace: &C4Workspace, view: &C4View) -> DiagramElement {
    let mut pairs = vec![
        ("title".to_owned(), title_of(workspace, view)),
        ("viewKind".to_owned(), view.kind.name().to_owned()),
        ("viewKey".to_owned(), view.key.clone()),
    ];
    for (index, (label, style)) in legend_for(workspace, view).into_iter().enumerate() {
        pairs.push((format!("legend.{index}.label"), label));
        pairs.push((format!("legend.{index}.background"), style.background));
        pairs.push((format!("legend.{index}.color"), style.color));
        pairs.push((format!("legend.{index}.shape"), style.shape));
    }
    DiagramElement {
        attributes: pairs,
        ..DiagramElement::node(VIEW_ELEMENT_ID, VIEW_TYPE)
    }
}

/// The title C4 requires on every diagram, worded "<type> diagram for <scope>"; a title the document declares wins.
pub fn title_of(workspace: &C4Workspace, view: &C4View) -> String {
    if let Some(declared) = view.title.as_ref().filter(|t| !t.is_empty()) {
        return declared.clone();
    }
    let kind = match view.kind {
        C4ViewKind::SystemLandscape => "System Landscape",
        C4ViewKind::SystemContext => "System Context",
        C4ViewKind::Container => "Container",
        C4ViewKind::Component => "Component",
        C4ViewKind::Dynamic => "Dynamic",
        C4ViewKind::Deployment => "Deployment",
    };
    let scope = view
        .scope_id
        .as_deref()
        .and_then(|id| workspace.find(id))
        .map(|e| e.name.clone())
        .filter(|name| !name.is_empty());
    if view.kind == C4ViewKind::SystemLandscape {
        return if workspace.name.is_empty() {
            "System Landscape diagram".to_owned()
        } else {
            format!("System Landscape diagram for {}", workspace.name)
        };
    }
    if view.kind == C4ViewKind::Deployment
        && let Some(environment) = view.environment.as_ref().filter(|e| !e.is_empty())
    {
        return match scope {
            Some(scope) => format!("Deployment diagram for {scope} - {environment}"),
            None => format!("Deployment diagram - {environment}"),
        };
    }
    match scope {
        Some(scope) => format!("{kind} diagram for {scope}"),
        None => format!("{kind} diagram"),
    }
}

/// The key, built from the notation in use: one entry per kind (and per kind drawn external), painted from its first
/// member.
pub fn legend_for(workspace: &C4Workspace, view: &C4View) -> Vec<(String, theme::C4Style)> {
    let mut entries: Vec<(String, theme::C4Style)> = Vec::new();
    for element in rules::members_of(workspace, view) {
        let style = theme::apply(&theme::style_for(element), element, &workspace.styles);
        let label = if element.is_external() {
            format!("{} (external)", theme::kind_tag(element.kind))
        } else {
            theme::kind_tag(element.kind).to_owned()
        };
        if !entries.iter().any(|(seen, _)| *seen == label) {
            entries.push((label, style));
        }
    }
    entries
}

/// The relationships this view draws, with the boxes at their two ends and, on a dynamic view, the interaction's
/// number.
pub fn relationships_of(
    workspace: &C4Workspace,
    view: &C4View,
    layout: &C4Layout,
) -> Vec<(C4Relationship, Rect, Rect, String)> {
    let mut result = Vec::new();
    if view.kind == C4ViewKind::Dynamic {
        for interaction in &view.interactions {
            if let (Some(from), Some(to)) = (
                layout.boxes.get(&interaction.source_id),
                layout.boxes.get(&interaction.destination_id),
            ) {
                result.push((
                    C4Relationship {
                        source_id: interaction.source_id.clone(),
                        destination_id: interaction.destination_id.clone(),
                        description: interaction.description.clone(),
                        technology: String::new(),
                        line: interaction.line,
                    },
                    from,
                    to,
                    interaction.order.clone(),
                ));
            }
        }
        return result;
    }
    if view.kind == C4ViewKind::Deployment {
        let members = rules::members_of(workspace, view);
        for relationship in layout::deployment_relationships_of(workspace, &members) {
            if let (Some(from), Some(to)) = (
                layout.boxes.get(&relationship.source_id),
                layout.boxes.get(&relationship.destination_id),
            ) {
                result.push((relationship, from, to, String::new()));
            }
        }
        return result;
    }
    // Elevated to the level the view shows: a container talking to an external system is, at the system level, that
    // system talking to it. Several elevated relationships collapsing onto one pair draw one line.
    let mut drawn: Vec<(String, String)> = Vec::new();
    for relationship in &workspace.relationships {
        let (Some(source), Some(destination)) = (
            nearest_shown(workspace, layout, &relationship.source_id),
            nearest_shown(workspace, layout, &relationship.destination_id),
        ) else {
            continue;
        };
        if same(&source, &destination) {
            continue;
        }
        let pair = (source.clone(), destination.clone());
        if drawn.contains(&pair) {
            continue;
        }
        drawn.push(pair);
        let elevated = if same(&source, &relationship.source_id) && same(&destination, &relationship.destination_id) {
            relationship.clone()
        } else {
            C4Relationship {
                source_id: source.clone(),
                destination_id: destination.clone(),
                ..relationship.clone()
            }
        };
        let (from, to) = (
            layout.boxes.get(&source).unwrap_or_default(),
            layout.boxes.get(&destination).unwrap_or_default(),
        );
        result.push((elevated, from, to, String::new()));
    }
    result
}

/// The element the view draws for `id`: itself when it is on the view, else the nearest ancestor that is.
fn nearest_shown(workspace: &C4Workspace, layout: &C4Layout, id: &str) -> Option<String> {
    let mut current = workspace.find(id);
    while let Some(element) = current {
        if layout.boxes.contains(&element.id) {
            return Some(element.id.clone());
        }
        current = element.parent_id.as_deref().and_then(|p| workspace.find(p));
    }
    None
}

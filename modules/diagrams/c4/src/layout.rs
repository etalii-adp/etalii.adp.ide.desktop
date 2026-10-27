//! Where a view's elements go (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4Layout.cs` at `8b6ccfc`: a
//! layered arrangement along the view's flow, elements ranked by the longest path to them over the relationships the
//! view shows, each rank centred on the others; a document's `autoLayout` sets the direction and separations and
//! makes stored positions ignored; a container or component view draws a boundary around its scope's children and
//! moves anything else out of it; a deployment view nests every node around what it hosts. Every coordinate is
//! rounded as .NET's `Math.Round(value, 2)` rounds it, so the boxes are the standalone host's to the last bit.

use std::collections::HashMap;

use adp_layout::rect::Rect;
use adp_layout::text_metric;

use crate::metrics::{C4Metrics, round2};
use crate::model::{C4Element, C4ElementKind, C4Relationship, C4View, C4ViewKind, C4Workspace, folded, same};
use crate::rules;
use crate::sidecar::Positions;

/// `0.000001f`, the single-precision literal the standalone host compares with, as a double.
const TOLERANCE: f64 = 0.000001_f32 as f64;

/// Boxes by element id, compared without regard to case, in the order they were first placed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Boxes {
    entries: Vec<(String, Rect)>,
    index: HashMap<String, usize>,
}

impl Boxes {
    /// Places `id` at `rect`; an id already placed keeps its place in the order.
    pub fn insert(&mut self, id: &str, rect: Rect) {
        let key = folded(id);
        match self.index.get(&key) {
            Some(at) => self.entries[*at].1 = rect,
            None => {
                self.index.insert(key, self.entries.len());
                self.entries.push((id.to_owned(), rect));
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<Rect> {
        self.index.get(&folded(id)).map(|at| self.entries[*at].1)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.index.contains_key(&folded(id))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every box, in the order it was first placed.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Rect)> {
        self.entries.iter().map(|(id, rect)| (id.as_str(), *rect))
    }

    fn of(&self, id: &str) -> Rect {
        self.get(id).unwrap_or_default()
    }
}

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
    let direction = view
        .auto_layout
        .as_ref()
        .map_or("tb".to_owned(), |a| a.direction.clone());
    let rank_separation = view
        .auto_layout
        .as_ref()
        .and_then(|a| a.rank_separation)
        .unwrap_or(metrics.rank_separation as i32);
    let node_separation = view
        .auto_layout
        .as_ref()
        .and_then(|a| a.node_separation)
        .unwrap_or(metrics.node_separation as i32);

    if view.kind == C4ViewKind::Deployment {
        return compute_nested(
            workspace,
            view,
            &members,
            metrics,
            authored,
            &direction,
            rank_separation,
            node_separation,
        );
    }

    let mut sizes = Boxes::default();
    for element in &members {
        sizes.insert(&element.id, measure_of(workspace, element, metrics));
    }
    let ranks = rank_by(workspace, view, &members);
    let mut boxes = place(
        &members,
        &sizes,
        &ranks,
        &direction,
        rank_separation,
        f64::from(node_separation),
    );

    let has_authored = authored.is_some_and(|a| !a.is_empty() && a.keys().any(|id| sizes.contains(id)));
    let authored_ignored = has_authored && view.auto_layout.is_some();
    if has_authored
        && !authored_ignored
        && let Some(authored) = authored
    {
        for (id, x, y) in authored.iter() {
            if let Some(size) = sizes.get(id) {
                boxes.insert(id, Rect::new(round2(x), round2(y), size.width, size.height));
            }
        }
    }

    let boundaries = boundaries_for(workspace, view, &members, &boxes, metrics);
    let boundaries = push_outsiders(workspace, view, &members, &mut boxes, boundaries, metrics);
    C4Layout {
        boxes,
        boundaries,
        authored_positions_ignored: authored_ignored,
    }
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

/// A deployment view: every node that hosts something on the view drawn as a boundary around what it hosts, to any
/// depth, the siblings inside each node ranked along the relationships between them. A position the user arranged is
/// honoured only for an element no node encloses.
#[allow(clippy::too_many_arguments)]
fn compute_nested(
    workspace: &C4Workspace,
    view: &C4View,
    members: &[&C4Element],
    metrics: &C4Metrics,
    authored: Option<&Positions>,
    direction: &str,
    rank_separation: i32,
    node_separation: i32,
) -> C4Layout {
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
    let hosts: HashMap<String, Option<String>> = members.iter().map(|e| (folded(&e.id), host_of(e))).collect();
    let host = |id: &str| hosts.get(&folded(id)).cloned().flatten();
    let edges: Vec<(String, String)> = deployment_relationships_of(workspace, members)
        .into_iter()
        .map(|r| (r.source_id, r.destination_id))
        .collect();

    let nested = Nested {
        workspace,
        members,
        metrics,
        direction,
        rank_separation,
        node_separation,
        edges: &edges,
        host: &host,
    };
    let top: Vec<&C4Element> = members.iter().copied().filter(|e| host(&e.id).is_none()).collect();
    let (top_boxes, top_boundaries, _, _) = nested.arrange(&top);
    let mut boxes = Boxes::default();
    for (id, rect) in top_boxes.iter() {
        boxes.insert(id, rect);
    }

    let has_authored = authored.is_some_and(|a| !a.is_empty() && a.keys().any(|id| boxes.contains(id)));
    let authored_ignored = has_authored && view.auto_layout.is_some();
    if has_authored
        && !authored_ignored
        && let Some(authored) = authored
    {
        for (id, x, y) in authored.iter() {
            if let (Some(rect), Some(element)) = (boxes.get(id), by_id(id))
                && host(&element.id).is_none()
            {
                boxes.insert(id, Rect::new(round2(x), round2(y), rect.width, rect.height));
            }
        }
    }
    C4Layout {
        boxes,
        boundaries: top_boundaries,
        authored_positions_ignored: authored_ignored,
    }
}

/// What a deployment view's nested arrangement reads.
struct Nested<'a> {
    workspace: &'a C4Workspace,
    members: &'a [&'a C4Element],
    metrics: &'a C4Metrics,
    direction: &'a str,
    rank_separation: i32,
    node_separation: i32,
    edges: &'a [(String, String)],
    host: &'a dyn Fn(&str) -> Option<String>,
}

impl Nested<'_> {
    /// Lays out one set of siblings with its top-left corner at the origin: every box and boundary inside it, relative
    /// to that corner, and the extent it takes up.
    fn arrange(&self, siblings: &[&C4Element]) -> (Boxes, Vec<C4Boundary>, f64, f64) {
        let padding = self.metrics.boundary_padding;
        let mut sizes = Boxes::default();
        let mut inner: HashMap<String, (Boxes, Vec<C4Boundary>)> = HashMap::new();
        for sibling in siblings {
            let children: Vec<&C4Element> = self
                .members
                .iter()
                .copied()
                .filter(|e| (self.host)(&e.id).is_some_and(|h| same(&h, &sibling.id)))
                .collect();
            if children.is_empty() {
                sizes.insert(&sibling.id, measure_of(self.workspace, sibling, self.metrics));
                continue;
            }
            let (child_boxes, child_boundaries, width, height) = self.arrange(&children);
            inner.insert(folded(&sibling.id), (child_boxes, child_boundaries));
            let label_width = text_metric::width_of(&boundary_label_of(sibling), self.metrics.font_size)
                + 2.0 * self.metrics.horizontal_padding;
            sizes.insert(
                &sibling.id,
                Rect::new(
                    0.0,
                    0.0,
                    round2((width + 2.0 * padding).max(label_width)),
                    round2(height + 2.0 * padding + self.metrics.font_size * self.metrics.line_height),
                ),
            );
        }

        // Rank the siblings along the relationships between them, each end standing in for the sibling that hosts
        // it.
        let sibling_of = |id: &str| -> Option<String> {
            let mut current = Some(id.to_owned());
            while let Some(candidate) = current {
                if siblings.iter().any(|s| same(&s.id, &candidate)) {
                    return Some(candidate);
                }
                current = (self.host)(&candidate);
            }
            None
        };
        let sibling_edges: Vec<(String, String)> = self
            .edges
            .iter()
            .filter_map(|(source, destination)| {
                let (source, destination) = (sibling_of(source)?, sibling_of(destination)?);
                (!same(&source, &destination)).then_some((source, destination))
            })
            .collect();
        let ranks = rank(siblings, &sibling_edges);
        let placed = place(
            siblings,
            &sizes,
            &ranks,
            self.direction,
            self.rank_separation,
            f64::from(self.node_separation),
        );
        let left = placed.iter().map(|(_, r)| r.x).fold(f64::INFINITY, f64::min);
        let top = placed.iter().map(|(_, r)| r.y).fold(f64::INFINITY, f64::min);

        let mut result_boxes = Boxes::default();
        let mut result_boundaries = Vec::new();
        for sibling in siblings {
            let at = placed.of(&sibling.id);
            let rect = Rect {
                x: round2(at.x - left),
                y: round2(at.y - top),
                ..at
            };
            let Some((contents, boundaries)) = inner.get(&folded(&sibling.id)) else {
                result_boxes.insert(&sibling.id, rect);
                continue;
            };
            // Outermost first, so a canvas drawing them in order puts the inner ones on top.
            result_boundaries.push(C4Boundary {
                id: format!("boundary:{}", sibling.id),
                name: sibling.name.clone(),
                kind: boundary_kind_of(sibling),
                rect,
            });
            let dx = rect.x + padding;
            let dy = rect.y + padding;
            for (id, child) in contents.iter() {
                result_boxes.insert(id, offset(child, dx, dy));
            }
            result_boundaries.extend(boundaries.iter().map(|b| C4Boundary {
                rect: offset(b.rect, dx, dy),
                ..b.clone()
            }));
        }
        let extent_width = placed.iter().map(|(_, r)| r.right()).fold(f64::NEG_INFINITY, f64::max) - left;
        let extent_height = placed.iter().map(|(_, r)| r.bottom()).fold(f64::NEG_INFINITY, f64::max) - top;
        (result_boxes, result_boundaries, extent_width, extent_height)
    }
}

fn offset(rect: Rect, dx: f64, dy: f64) -> Rect {
    Rect {
        x: round2(rect.x + dx),
        y: round2(rect.y + dy),
        ..rect
    }
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

/// How far each element sits from a source, over the relationships of the model (the interactions, on a dynamic
/// view).
fn rank_by(workspace: &C4Workspace, view: &C4View, members: &[&C4Element]) -> HashMap<String, i32> {
    let edges: Vec<(String, String)> = if view.kind == C4ViewKind::Dynamic {
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
    rank(members, &edges)
}

/// Longest-path ranks of `members` over `edges`, by folded id; an edge with an end outside the members is ignored, and
/// the ranks are relaxed at most once per member, so a cycle stops deepening rather than looping for ever.
fn rank(members: &[&C4Element], edges: &[(String, String)]) -> HashMap<String, i32> {
    let ids: Vec<String> = members.iter().map(|e| folded(&e.id)).collect();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    for (source, destination) in edges {
        let (source, destination) = (folded(source), folded(destination));
        if ids.contains(&source) && ids.contains(&destination) {
            outgoing.entry(source).or_default().push(destination);
        }
    }
    let mut ranks: HashMap<String, i32> = ids.iter().map(|id| (id.clone(), 0)).collect();
    for _ in members {
        let mut moved = false;
        for id in &ids {
            let Some(destinations) = outgoing.get(id) else {
                continue;
            };
            for destination in destinations {
                let next = ranks[id] + 1;
                if ranks[destination] < next {
                    ranks.insert(destination.clone(), next);
                    moved = true;
                }
            }
        }
        if !moved {
            break;
        }
    }
    ranks
}

/// Elements laid out rank by rank: each rank a row (a column when the flow is horizontal), centred on the others,
/// each element centred within its rank's thickness; `rl` and `bt` are the same layout read from the other end.
fn place(
    members: &[&C4Element],
    sizes: &Boxes,
    ranks: &HashMap<String, i32>,
    direction: &str,
    rank_separation: i32,
    node_separation: f64,
) -> Boxes {
    let horizontal = direction == "lr" || direction == "rl";
    let reversed = direction == "rl" || direction == "bt";
    let mut keys: Vec<i32> = Vec::new();
    for element in members {
        let rank = ranks[&folded(&element.id)];
        if !keys.contains(&rank) {
            keys.push(rank);
        }
    }
    keys.sort();

    let mut boxes = Boxes::default();
    let mut rank_offset = 0.0;
    for key in keys {
        let elements: Vec<&C4Element> = members
            .iter()
            .copied()
            .filter(|e| ranks[&folded(&e.id)] == key)
            .collect();
        let thickness = elements
            .iter()
            .map(|e| {
                let size = sizes.of(&e.id);
                if horizontal { size.width } else { size.height }
            })
            .fold(f64::NEG_INFINITY, f64::max);
        let mut along = 0.0;
        for element in &elements {
            let size = sizes.of(&element.id);
            along += if horizontal { size.height } else { size.width };
        }
        let along = along + node_separation * (elements.len() as f64 - 1.0);
        let mut along_offset = -along / 2.0;
        for element in &elements {
            let size = sizes.of(&element.id);
            let across_size = if horizontal { size.width } else { size.height };
            let along_size = if horizontal { size.height } else { size.width };
            let across = rank_offset + (thickness - across_size) / 2.0;
            let rect = if horizontal {
                Rect::new(round2(across), round2(along_offset), size.width, size.height)
            } else {
                Rect::new(round2(along_offset), round2(across), size.width, size.height)
            };
            boxes.insert(&element.id, rect);
            along_offset += along_size + node_separation;
        }
        rank_offset += thickness + f64::from(rank_separation);
    }
    if !reversed {
        return boxes;
    }
    let extent = rank_offset - f64::from(rank_separation);
    let mut flipped = Boxes::default();
    for (id, rect) in boxes.iter() {
        let rect = if horizontal {
            Rect {
                x: round2(extent - rect.right()),
                ..rect
            }
        } else {
            Rect {
                y: round2(extent - rect.bottom()),
                ..rect
            }
        };
        flipped.insert(id, rect);
    }
    flipped
}

/// Moves anything not inside the boundary out of it by its shortest route, separates what landed on each other, and
/// sizes the boundary again.
fn push_outsiders(
    workspace: &C4Workspace,
    view: &C4View,
    members: &[&C4Element],
    boxes: &mut Boxes,
    boundaries: Vec<C4Boundary>,
    metrics: &C4Metrics,
) -> Vec<C4Boundary> {
    let Some(scope) = view.scope_id.as_deref() else {
        return boundaries;
    };
    let Some(first) = boundaries.first() else {
        return boundaries;
    };
    let boundary = first.rect;
    let inside = |element: &C4Element| element.parent_id.as_deref().is_some_and(|p| same(p, scope));
    let outsiders: Vec<String> = members.iter().filter(|e| !inside(e)).map(|e| e.id.clone()).collect();
    let mut pushed_vertically: Vec<(String, bool)> = Vec::new();
    let separation = metrics.node_separation;
    for id in &outsiders {
        let rect = boxes.of(id);
        if !rect.overlaps(&boundary) {
            continue;
        }
        let left = rect.right() - boundary.x + separation;
        let right = boundary.right() - rect.x + separation;
        let up = rect.bottom() - boundary.y + separation;
        let down = boundary.bottom() - rect.y + separation;
        let shortest = left.min(right).min(up.min(down));
        let moved = if (shortest - left).abs() < TOLERANCE {
            Rect {
                x: round2(rect.x - left),
                ..rect
            }
        } else if (shortest - right).abs() < TOLERANCE {
            Rect {
                x: round2(rect.x + right),
                ..rect
            }
        } else if (shortest - up).abs() < TOLERANCE {
            Rect {
                y: round2(rect.y - up),
                ..rect
            }
        } else {
            Rect {
                y: round2(rect.y + down),
                ..rect
            }
        };
        boxes.insert(id, moved);
        let vertical = (shortest - up).abs() < TOLERANCE || (shortest - down).abs() < TOLERANCE;
        match pushed_vertically.iter_mut().find(|(other, _)| same(other, id)) {
            Some(entry) => entry.1 = vertical,
            None => pushed_vertically.push((id.clone(), vertical)),
        }
    }
    spread(boxes, &outsiders, &pushed_vertically, separation);
    boundaries_for(workspace, view, members, boxes, metrics)
}

/// The boundary a container or component view draws around its scope's children, sized to them with a margin and
/// room at the foot for its own label.
fn boundaries_for(
    workspace: &C4Workspace,
    view: &C4View,
    members: &[&C4Element],
    boxes: &Boxes,
    metrics: &C4Metrics,
) -> Vec<C4Boundary> {
    if !matches!(view.kind, C4ViewKind::Container | C4ViewKind::Component) {
        return Vec::new();
    }
    let Some(scope) = view.scope_id.as_deref().and_then(|id| workspace.find(id)) else {
        return Vec::new();
    };
    let inside: Vec<Rect> = members
        .iter()
        .filter(|e| e.parent_id.as_deref().is_some_and(|p| same(p, &scope.id)))
        .map(|e| boxes.of(&e.id))
        .collect();
    if inside.is_empty() {
        return Vec::new();
    }
    let padding = metrics.boundary_padding;
    let x = inside.iter().map(|r| r.x).fold(f64::INFINITY, f64::min) - padding;
    let y = inside.iter().map(|r| r.y).fold(f64::INFINITY, f64::min) - padding;
    let right = inside.iter().map(|r| r.right()).fold(f64::NEG_INFINITY, f64::max) + padding;
    let bottom = inside.iter().map(|r| r.bottom()).fold(f64::NEG_INFINITY, f64::max)
        + padding
        + metrics.font_size * metrics.line_height;
    let kind = if view.kind == C4ViewKind::Container {
        "Software System"
    } else {
        "Container"
    };
    vec![C4Boundary {
        id: format!("boundary:{}", scope.id),
        name: scope.name.clone(),
        kind: kind.to_owned(),
        rect: Rect::new(round2(x), round2(y), round2(right - x), round2(bottom - y)),
    }]
}

/// Separates outsiders that landed on each other on the way out of a boundary: first a sweep along the axis
/// perpendicular to the one each was pushed off, then a bounded pairwise nudge for what the sweeps cannot see.
fn spread(boxes: &mut Boxes, outsiders: &[String], pushed_vertically: &[(String, bool)], separation: f64) {
    if pushed_vertically.is_empty() {
        return;
    }
    let vertical: Vec<String> = pushed_vertically
        .iter()
        .filter(|(_, v)| *v)
        .map(|(id, _)| id.clone())
        .collect();
    let horizontal: Vec<String> = pushed_vertically
        .iter()
        .filter(|(_, v)| !*v)
        .map(|(id, _)| id.clone())
        .collect();
    sweep_apart(boxes, &vertical, true, separation);
    sweep_apart(boxes, &horizontal, false, separation);

    for _ in outsiders {
        let mut moved = false;
        for (id, vertically) in pushed_vertically {
            for other in outsiders {
                if same(other, id) || !boxes.of(id).overlaps(&boxes.of(other)) {
                    continue;
                }
                let rect = boxes.of(id);
                let blocker = boxes.of(other);
                // A tie is answered by the ordinally smaller id going first, never the same answer for both.
                let goes_first = id.encode_utf16().lt(other.encode_utf16());
                let rect = if *vertically {
                    let to_left = rect.right() - blocker.x + separation;
                    let to_right = blocker.right() - rect.x + separation;
                    if to_left < to_right || ((to_left - to_right).abs() < TOLERANCE && goes_first) {
                        Rect {
                            x: round2(rect.x - to_left),
                            ..rect
                        }
                    } else {
                        Rect {
                            x: round2(rect.x + to_right),
                            ..rect
                        }
                    }
                } else {
                    let up = rect.bottom() - blocker.y + separation;
                    let down = blocker.bottom() - rect.y + separation;
                    if up < down || ((up - down).abs() < TOLERANCE && goes_first) {
                        Rect {
                            y: round2(rect.y - up),
                            ..rect
                        }
                    } else {
                        Rect {
                            y: round2(rect.y + down),
                            ..rect
                        }
                    }
                };
                boxes.insert(id, rect);
                moved = true;
            }
        }
        if !moved {
            return;
        }
    }
}

/// Lays `ids` out in a row (or a column) so that no two of them overlap on that axis, in the order of their position
/// and then of their id; an element only ever moves forwards.
fn sweep_apart(boxes: &mut Boxes, ids: &[String], horizontally: bool, separation: f64) {
    let start_of = |rect: Rect| if horizontally { rect.x } else { rect.y };
    let mut ordered: Vec<&String> = ids.iter().collect();
    ordered.sort_by(|a, b| {
        start_of(boxes.of(a))
            .partial_cmp(&start_of(boxes.of(b)))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.encode_utf16().cmp(b.encode_utf16()))
    });
    let mut cursor: Option<f64> = None;
    for id in ordered {
        let mut rect = boxes.of(id);
        let mut start = start_of(rect);
        let size = if horizontally { rect.width } else { rect.height };
        if let Some(from) = cursor
            && start < from
        {
            if horizontally {
                rect.x = round2(from);
            } else {
                rect.y = round2(from);
            }
            boxes.insert(id, rect);
            start = from;
        }
        cursor = Some(start + size + separation);
    }
}

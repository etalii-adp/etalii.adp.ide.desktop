//! `layout.rankedBoundaries` (ATD §13.3; its contract and 115 fixtures are in `specifications/atd/capabilities/`): ranks
//! along the edges, and boundary nodes drawn around the nodes whose `parent` they are. Moved (not rewritten) from the C4
//! module's layout, which calls it here (etalii.adp spec 067, T026), itself ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4Layout.cs` at `8b6ccfc`: a layered arrangement along the flow, nodes
//! ranked by the longest path to them, each rank centred on the others; mode `enclose` draws one boundary around the
//! nodes it holds and moves anything else out of it; mode `nest` nests every boundary around what it holds. Every
//! coordinate is rounded as .NET's `Math.Round(value, 2)` rounds it, so the boxes are the standalone host's to the last
//! bit.

use std::collections::HashMap;

use adp_layout::rect::Rect;
use serde_json::Value as Json;

use super::LayoutCapability;

/// `0.000001f`, the single-precision literal the standalone host compares with, as a double.
const TOLERANCE: f64 = 0.000001_f32 as f64;

/// .NET's `Math.Round(value, 2)`: to two decimals, a midpoint to the even neighbour.
pub fn round2(value: f64) -> f64 {
    if value.abs() < 1e16 {
        (value * 100.0).round_ties_even() / 100.0
    } else {
        value
    }
}

/// An id's folded form, by which ids are compared.
fn folded(id: &str) -> String {
    id.chars().flat_map(char::to_uppercase).collect()
}

fn same(left: &str, right: &str) -> bool {
    left.chars()
        .flat_map(char::to_uppercase)
        .eq(right.chars().flat_map(char::to_uppercase))
}

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

/// A node of `$defs/LayoutInput`.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedNode {
    pub id: String,
    pub width: f64,
    pub height: f64,
    /// The boundary node that holds it.
    pub parent: Option<String>,
    /// Whether it is drawn around the nodes whose `parent` it is; its `width` is then the least its label needs.
    pub boundary: bool,
    /// Its pin, in the canvas's domain units.
    pub pinned: Option<(f64, f64)>,
}

/// The capability's arguments, evaluated.
#[derive(Debug, Clone, PartialEq)]
pub struct RankedArguments {
    /// Whether boundaries nest to any depth (`nest`), or one encloses its nodes (`enclose`).
    pub nest: bool,
    pub direction: String,
    pub rank_separation: f64,
    pub node_separation: f64,
    pub pins_ignored: bool,
    pub padding: f64,
    pub label_height: f64,
    pub clearance: f64,
}

impl Default for RankedArguments {
    fn default() -> Self {
        RankedArguments {
            nest: false,
            direction: "tb".to_owned(),
            rank_separation: 120.0,
            node_separation: 60.0,
            pins_ignored: false,
            padding: 32.0,
            label_height: 19.6,
            clearance: 60.0,
        }
    }
}

/// Where everything goes: the boxes of the nodes that are not drawn as boundaries, and the boundaries, outermost first.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RankedPlacement {
    pub boxes: Boxes,
    pub boundaries: Vec<(String, Rect)>,
}

/// Places `nodes` along `edges`, each edge its two ends' ids.
pub fn place_nodes(nodes: &[RankedNode], edges: &[(String, String)], arguments: &RankedArguments) -> RankedPlacement {
    if nodes.iter().all(|node| node.boundary) && !arguments.nest {
        return RankedPlacement::default();
    }
    if arguments.nest {
        nest(nodes, edges, arguments)
    } else {
        enclose(nodes, edges, arguments)
    }
}

/// Mode `enclose`: the nodes ranked and placed together, the pins laid over them, and one boundary around the nodes it
/// holds, anything else moved out of it.
fn enclose(nodes: &[RankedNode], edges: &[(String, String)], arguments: &RankedArguments) -> RankedPlacement {
    let members: Vec<&RankedNode> = nodes.iter().filter(|node| !node.boundary).collect();
    let mut sizes = Boxes::default();
    for node in &members {
        sizes.insert(&node.id, Rect::new(0.0, 0.0, node.width, node.height));
    }
    let ranks = rank(&members, edges);
    let mut boxes = place(
        &members,
        &sizes,
        &ranks,
        &arguments.direction,
        arguments.rank_separation,
        arguments.node_separation,
    );
    if !arguments.pins_ignored {
        for node in &members {
            if let (Some((x, y)), Some(size)) = (node.pinned, sizes.get(&node.id)) {
                boxes.insert(&node.id, Rect::new(round2(x), round2(y), size.width, size.height));
            }
        }
    }
    let boundary = nodes.iter().find(|node| node.boundary);
    let mut boundaries = boundary
        .and_then(|boundary| boundary_around(boundary, &members, &boxes, arguments))
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(boundary) = boundary
        && let Some((_, rect)) = boundaries.first().cloned()
    {
        push_outsiders(boundary, &members, &mut boxes, rect, arguments);
        boundaries = boundary_around(boundary, &members, &boxes, arguments)
            .into_iter()
            .collect();
    }
    RankedPlacement { boxes, boundaries }
}

/// Mode `nest`: the nodes with no parent arranged as one set, every boundary around what it holds, to any depth. Unless
/// the pins are ignored, a pinned node that has no parent and encloses nothing takes its pin.
fn nest(nodes: &[RankedNode], edges: &[(String, String)], arguments: &RankedArguments) -> RankedPlacement {
    // An edge names an element by its own id, also when the element is drawn as the boundary `boundary:<id>`, as the
    // fixtures record it.
    let end = |id: &String| {
        let boundary = format!("boundary:{id}");
        match nodes
            .iter()
            .find(|node| same(&node.id, id) || same(&node.id, &boundary))
        {
            Some(node) => node.id.clone(),
            None => id.clone(),
        }
    };
    let edges: Vec<(String, String)> = edges.iter().map(|(from, to)| (end(from), end(to))).collect();
    let nested = Nested {
        nodes,
        edges: &edges,
        arguments,
    };
    let top: Vec<&RankedNode> = nodes.iter().filter(|node| node.parent.is_none()).collect();
    let (top_boxes, boundaries, _, _) = nested.arrange(&top);
    let mut boxes = Boxes::default();
    for (id, rect) in top_boxes.iter() {
        boxes.insert(id, rect);
    }
    if !arguments.pins_ignored {
        for node in &top {
            if let (Some((x, y)), Some(rect)) = (node.pinned, boxes.get(&node.id)) {
                boxes.insert(&node.id, Rect::new(round2(x), round2(y), rect.width, rect.height));
            }
        }
    }
    RankedPlacement { boxes, boundaries }
}

/// What the nested arrangement reads.
struct Nested<'a> {
    nodes: &'a [RankedNode],
    edges: &'a [(String, String)],
    arguments: &'a RankedArguments,
}

impl Nested<'_> {
    fn parent_of(&self, id: &str) -> Option<String> {
        self.nodes
            .iter()
            .find(|node| same(&node.id, id))
            .and_then(|node| node.parent.clone())
    }

    /// Lays out one set of siblings with its top-left corner at the origin: every box and boundary inside it, relative
    /// to that corner, and the extent it takes up.
    fn arrange(&self, siblings: &[&RankedNode]) -> (Boxes, Vec<(String, Rect)>, f64, f64) {
        let padding = self.arguments.padding;
        let mut sizes = Boxes::default();
        let mut inner: HashMap<String, (Boxes, Vec<(String, Rect)>)> = HashMap::new();
        for sibling in siblings {
            let children: Vec<&RankedNode> = self
                .nodes
                .iter()
                .filter(|node| node.parent.as_deref().is_some_and(|parent| same(parent, &sibling.id)))
                .collect();
            if children.is_empty() {
                sizes.insert(&sibling.id, Rect::new(0.0, 0.0, sibling.width, sibling.height));
                continue;
            }
            let (child_boxes, child_boundaries, width, height) = self.arrange(&children);
            inner.insert(folded(&sibling.id), (child_boxes, child_boundaries));
            sizes.insert(
                &sibling.id,
                Rect::new(
                    0.0,
                    0.0,
                    round2((width + 2.0 * padding).max(sibling.width)),
                    round2(height + 2.0 * padding + self.arguments.label_height),
                ),
            );
        }

        // Rank the siblings along the edges between them, each end standing in for the sibling that encloses it.
        let sibling_of = |id: &str| -> Option<String> {
            let mut current = Some(id.to_owned());
            while let Some(candidate) = current {
                if siblings.iter().any(|s| same(&s.id, &candidate)) {
                    return Some(candidate);
                }
                current = self.parent_of(&candidate);
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
            &self.arguments.direction,
            self.arguments.rank_separation,
            self.arguments.node_separation,
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
            result_boundaries.push((sibling.id.clone(), rect));
            let dx = rect.x + padding;
            let dy = rect.y + padding;
            for (id, child) in contents.iter() {
                result_boxes.insert(id, offset(child, dx, dy));
            }
            result_boundaries.extend(boundaries.iter().map(|(id, b)| (id.clone(), offset(*b, dx, dy))));
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

/// Longest-path ranks of `members` over `edges`, by folded id; an edge with an end outside the members is ignored, and
/// the ranks are relaxed at most once per member, so a cycle stops deepening rather than looping for ever.
fn rank(members: &[&RankedNode], edges: &[(String, String)]) -> HashMap<String, i32> {
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

/// Nodes laid out rank by rank: each rank a row (a column when the flow is horizontal), centred on the others,
/// each node centred within its rank's thickness; `rl` and `bt` are the same layout read from the other end.
fn place(
    members: &[&RankedNode],
    sizes: &Boxes,
    ranks: &HashMap<String, i32>,
    direction: &str,
    rank_separation: f64,
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
        let elements: Vec<&RankedNode> = members
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
        rank_offset += thickness + rank_separation;
    }
    if !reversed {
        return boxes;
    }
    let extent = rank_offset - rank_separation;
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

/// Moves anything not inside the boundary out of it by its shortest route, and separates what landed on each other.
fn push_outsiders(
    scope: &RankedNode,
    members: &[&RankedNode],
    boxes: &mut Boxes,
    boundary: Rect,
    arguments: &RankedArguments,
) {
    let inside = |node: &RankedNode| node.parent.as_deref().is_some_and(|p| same(p, &scope.id));
    let outsiders: Vec<String> = members.iter().filter(|e| !inside(e)).map(|e| e.id.clone()).collect();
    let mut pushed_vertically: Vec<(String, bool)> = Vec::new();
    let separation = arguments.clearance;
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
}

/// The boundary around the nodes `scope` holds, sized to them with a margin and room at the foot for its own label; none
/// when it holds nothing.
fn boundary_around(
    scope: &RankedNode,
    members: &[&RankedNode],
    boxes: &Boxes,
    arguments: &RankedArguments,
) -> Option<(String, Rect)> {
    let inside: Vec<Rect> = members
        .iter()
        .filter(|e| e.parent.as_deref().is_some_and(|p| same(p, &scope.id)))
        .map(|e| boxes.of(&e.id))
        .collect();
    if inside.is_empty() {
        return None;
    }
    let padding = arguments.padding;
    let x = inside.iter().map(|r| r.x).fold(f64::INFINITY, f64::min) - padding;
    let y = inside.iter().map(|r| r.y).fold(f64::INFINITY, f64::min) - padding;
    let right = inside.iter().map(|r| r.right()).fold(f64::NEG_INFINITY, f64::max) + padding;
    let bottom = inside.iter().map(|r| r.bottom()).fold(f64::NEG_INFINITY, f64::max) + padding + arguments.label_height;
    let rect = Rect::new(round2(x), round2(y), round2(right - x), round2(bottom - y));
    Some((scope.id.clone(), rect))
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

/// The capability over `$defs/LayoutInput`, its query arguments each given as a CEL constant.
pub struct RankedBoundaries;

impl LayoutCapability for RankedBoundaries {
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"]
            .as_array()
            .ok_or("The input has no `nodes`.")?
            .iter()
            .map(|node| RankedNode {
                id: node["id"].as_str().unwrap_or_default().to_owned(),
                width: node["width"].as_f64().unwrap_or(0.0),
                height: node["height"].as_f64().unwrap_or(0.0),
                parent: node["parent"].as_str().map(str::to_owned),
                boundary: node["attributes"]["boundary"].as_bool() == Some(true),
                pinned: node["pinned"]["x"].as_f64().zip(node["pinned"]["y"].as_f64()),
            })
            .collect::<Vec<_>>();
        let edges: Vec<(String, String)> = input["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|edge| {
                let end = |name: &str| edge[name].as_str().unwrap_or_default().to_owned();
                (end("source"), end("target"))
            })
            .collect();
        let placed = place_nodes(&nodes, &edges, &arguments_of(args));
        let mut positions: Vec<(String, f64, f64)> = placed
            .boxes
            .iter()
            .map(|(id, rect)| (id.to_owned(), rect.x, rect.y))
            .collect();
        positions.extend(placed.boundaries.iter().map(|(id, rect)| (id.clone(), rect.x, rect.y)));
        Ok(positions)
    }

    fn query_arguments(&self) -> &'static [&'static str] {
        &["direction", "rankSeparation", "nodeSeparation", "pinsIgnored"]
    }
}

/// The arguments of `args`, the defaults filling what it leaves out. A query is given as the text of a CEL constant, a
/// number or a Boolean as itself.
fn arguments_of(args: &Json) -> RankedArguments {
    let defaults = RankedArguments::default();
    let number = |name: &str, default: f64| match &args[name] {
        Json::String(text) => text.parse().unwrap_or(default),
        value => value.as_f64().unwrap_or(default),
    };
    let direction = args["direction"]
        .as_str()
        .map(|text| text.trim_matches(['\'', '"']).to_owned());
    let pins_ignored = match &args["pinsIgnored"] {
        Json::String(text) => text == "true",
        value => value.as_bool().unwrap_or(defaults.pins_ignored),
    };
    RankedArguments {
        nest: args["mode"].as_str() == Some("nest"),
        direction: direction.unwrap_or(defaults.direction),
        rank_separation: number("rankSeparation", defaults.rank_separation),
        node_separation: number("nodeSeparation", defaults.node_separation),
        pins_ignored,
        padding: number("padding", defaults.padding),
        label_height: number("labelHeight", defaults.label_height),
        clearance: number("clearance", defaults.clearance),
    }
}

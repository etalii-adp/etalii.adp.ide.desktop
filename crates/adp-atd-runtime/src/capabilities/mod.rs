//! The capability registry (ATD §13, research R14): the capabilities of `specifications/atd/capabilities/` whose code
//! the desktop host has, each moved (not rewritten) from the module that did it, and registered only once its
//! conformance fixtures pass (`tests/capabilities.rs`). A capability is named by its contract, never by a tool type;
//! one this host does not provide is absent, and the gestures that need it are refused.

pub mod forward_bezier;
pub mod two_sided_tree;

use std::collections::BTreeMap;
use std::rc::Rc;

use adp_fbl::plugins::PersistencePlugin;
use adp_scene::scene::{RouteEnds, RoutePlugins};
use adp_scene::{Path, Point};
use serde_json::{Map, Value as Json, json};

/// A capability at the `layout` extension point: nodes with their sizes and pins (`$defs/LayoutInput`) to positions
/// (`$defs/LayoutResult`).
pub trait LayoutCapability {
    /// The top-left corner of every node placed, by id, in the order they are placed, as the algorithm computes them:
    /// not rounded, so that a host that places a node by its centre places it where the code it was moved from did.
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String>;

    /// `$defs/LayoutResult` for `input` and the capability's `args`, the defaults filling what `args` leaves out: the
    /// corners rounded to 0.01, as the contract states.
    fn layout(&self, input: &Json, args: &Json) -> Result<Json, String> {
        let positions: Map<String, Json> = self
            .place(input, args)?
            .into_iter()
            .map(|(id, x, y)| {
                (
                    id,
                    json!({"x": two_sided_tree::round2(x), "y": two_sided_tree::round2(y)}),
                )
            })
            .collect();
        Ok(json!({ "positions": positions }))
    }
}

/// A capability at the `routing` extension point: an edge's two end boxes to its route (`$defs/RoutingInput` to
/// `$defs/RoutingResult`).
pub trait RoutingCapability {
    /// The route from `from` to `to` between the end boxes, none where the capability does not route the edge.
    fn route(&self, from: Point, to: Point, ends: &RouteEnds) -> Option<Path>;

    /// `$defs/RoutingResult` for `input` and the capability's `args`, the points rounded to 0.01.
    fn routing(&self, input: &Json, args: &Json) -> Result<Json, String>;
}

/// The capabilities this host provides, by name: layouts, routings, and readers (a reader is a persistence plugin, FBL
/// §11, named by its capability as a binding's family).
#[derive(Clone, Default)]
pub struct Capabilities {
    layouts: BTreeMap<&'static str, Rc<dyn LayoutCapability>>,
    routings: BTreeMap<&'static str, Rc<dyn RoutingCapability>>,
    readers: BTreeMap<&'static str, Rc<dyn PersistencePlugin>>,
}

impl Capabilities {
    /// Every capability of the desktop host.
    pub fn desktop() -> Self {
        let mut capabilities = Capabilities::default();
        capabilities
            .layouts
            .insert("layout.twoSidedTree", Rc::new(TwoSidedTree));
        capabilities
            .routings
            .insert("routing.forwardBezier", Rc::new(forward_bezier::ForwardBezier));
        capabilities
    }

    /// These capabilities with `reader` as the reader capability `name`.
    pub fn with_reader(mut self, name: &'static str, reader: Rc<dyn PersistencePlugin>) -> Self {
        self.readers.insert(name, reader);
        self
    }

    pub fn layout(&self, name: &str) -> Option<Rc<dyn LayoutCapability>> {
        self.layouts.get(name).cloned()
    }

    pub fn routing(&self, name: &str) -> Option<Rc<dyn RoutingCapability>> {
        self.routings.get(name).cloned()
    }

    pub fn reader(&self, name: &str) -> Option<Rc<dyn PersistencePlugin>> {
        self.readers.get(name).cloned()
    }

    /// The names of every capability, in order.
    pub fn names(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = self
            .layouts
            .keys()
            .chain(self.routings.keys())
            .chain(self.readers.keys())
            .copied()
            .collect();
        names.sort_unstable();
        names
    }

    /// The output of capability `name` for a fixture's `input` and `args` (`$defs/CapabilityFixture`).
    pub fn run(&self, name: &str, fixture: &Json) -> Result<Json, String> {
        let empty = json!({});
        let args = fixture.get("args").unwrap_or(&empty);
        if let Some(layout) = self.layout(name) {
            return layout.layout(&fixture["input"], args);
        }
        match self.routing(name) {
            Some(routing) => routing.routing(&fixture["input"], args),
            None => Err(format!("This host provides no capability `{name}`.")),
        }
    }
}

/// The routings of these capabilities, as a scene draws a `routing: { plugin }` (DISL §6.10) by its capability's name.
impl RoutePlugins for Capabilities {
    fn route(&self, name: &str, from: Point, to: Point, ends: &RouteEnds) -> Option<Path> {
        self.routings.get(name)?.route(from, to, ends)
    }
}

/// `layout.twoSidedTree` over `$defs/LayoutInput`: the tree is the nodes' `parent` and `order`, from the first of
/// `roots` (else the node without a parent); a node's `side` attribute fixes a first-level node to `right` or `left`,
/// and a node whose `folded` or `collapsed` attribute is true hides its descendants.
struct TwoSidedTree;

impl LayoutCapability for TwoSidedTree {
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        use two_sided_tree::{Gaps, Side, TreeNode, layout};

        let defaults = Gaps::default();
        let number = |name: &str, default: f64| args.get(name).and_then(Json::as_f64).unwrap_or(default);
        let gaps = Gaps {
            spacing: number("spacing", defaults.spacing),
            level_gap: number("levelGap", defaults.level_gap),
            minimum_gap_ratio: number("minimumGapRatio", defaults.minimum_gap_ratio),
        };

        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut children: BTreeMap<&str, Vec<(i64, usize)>> = BTreeMap::new();
        let mut collapsed = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            if let Some(parent) = node["parent"].as_str() {
                let order = node["order"].as_i64().unwrap_or(index as i64);
                children.entry(parent).or_default().push((order, index));
            }
            let attributes = &node["attributes"];
            if [&attributes["folded"], &attributes["collapsed"]]
                .iter()
                .any(|v| v.as_bool() == Some(true))
            {
                collapsed.push(node["id"].as_str().unwrap_or_default().to_owned());
            }
        }
        for list in children.values_mut() {
            list.sort();
        }
        let root = input["roots"]
            .as_array()
            .and_then(|r| r.first())
            .and_then(Json::as_str)
            .and_then(|id| nodes.iter().position(|n| n["id"].as_str() == Some(id)))
            .or_else(|| nodes.iter().position(|n| n["parent"].is_null()))
            .ok_or("The input has no root.")?;

        fn tree(nodes: &[Json], index: usize, children: &BTreeMap<&str, Vec<(i64, usize)>>, depth: usize) -> TreeNode {
            let node = &nodes[index];
            let id = node["id"].as_str().unwrap_or_default();
            let side = match node["attributes"]["side"].as_str().map(str::to_lowercase).as_deref() {
                Some("left") => Some(Side::Left),
                Some("right") => Some(Side::Right),
                _ => None,
            };
            TreeNode {
                id: id.to_owned(),
                width: node["width"].as_f64().unwrap_or(0.0),
                height: node["height"].as_f64().unwrap_or(0.0),
                side,
                children: children
                    .get(id)
                    .into_iter()
                    .flatten()
                    .filter(|_| depth < nodes.len())
                    .map(|(_, child)| tree(nodes, *child, children, depth + 1))
                    .collect(),
            }
        }

        let boxes = layout(&tree(nodes, root, &children, 0), &gaps, &|id| {
            collapsed.iter().any(|c| c == id)
        });
        Ok(boxes.into_iter().map(|(id, rect)| (id, rect.x, rect.y)).collect())
    }
}

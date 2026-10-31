//! The capability registry (ATD §13, research R14): the capabilities of `specifications/atd/capabilities/` whose code
//! the desktop host has, each moved (not rewritten) from the module that did it, and registered only once its
//! conformance fixtures pass (`tests/capabilities.rs`). A capability is named by its contract, never by a tool type;
//! one this host does not provide is absent, and the gestures that need it are refused.

pub mod bands;
pub mod forward_bezier;
pub mod layered_columns;
pub mod ranked_boundaries;
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

    /// The arguments its contract types `query`: each evaluated once over the diagram, and given to [`place`] as the
    /// text of a CEL constant, as its fixtures state them.
    ///
    /// [`place`]: LayoutCapability::place
    fn query_arguments(&self) -> &'static [&'static str] {
        &[]
    }

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

/// A number among a capability's arguments, the default where it is left out.
fn number_argument(args: &Json, name: &str, default: f64) -> f64 {
    args[name].as_f64().unwrap_or(default)
}

/// The text of a query argument, as a fixture writes it or a description's `(…)` lowers it.
fn query_text(value: &Json) -> Option<&str> {
    value.as_str().or_else(|| value["cel"].as_str())
}

/// A node of `$defs/LayoutInput` as a query reads it: `self.id`, `self.type` and its attributes by name, the text
/// `true` or `false` as a Boolean, since an input's attributes are text.
fn self_of(node: &Json) -> adp_cel::Value {
    let mut entries: Vec<(String, adp_cel::Value)> = Vec::new();
    for key in ["id", "type"] {
        if let Some(text) = node[key].as_str() {
            entries.push((key.to_owned(), adp_cel::Value::String(text.to_owned())));
        }
    }
    for (key, value) in node["attributes"].as_object().into_iter().flatten() {
        let read = match value {
            Json::Bool(flag) => adp_cel::Value::Bool(*flag),
            Json::Number(number) => number.as_f64().map_or(adp_cel::Value::Null, adp_cel::Value::Double),
            Json::String(text) if text == "true" => adp_cel::Value::Bool(true),
            Json::String(text) if text == "false" => adp_cel::Value::Bool(false),
            Json::String(text) => adp_cel::Value::String(text.clone()),
            _ => adp_cel::Value::Null,
        };
        entries.push((key.clone(), read));
    }
    entries.into_iter().collect::<adp_cel::Map>().into()
}

/// The answer of the query argument `name` for `node`: evaluated here where `args` holds its text, as a fixture states
/// it, else what the host evaluated for the node and gave as the attribute of that name, as a runtime does.
fn query_of(node: &Json, args: &Json, name: &str) -> Result<Option<adp_cel::Value>, String> {
    if let Some(source) = query_text(&args[name]) {
        let program = adp_cel::Environment::standard(&["self"])
            .compile(source)
            .map_err(|e| format!("`{name}`: {e}"))?;
        let variables = std::collections::HashMap::from([("self".to_owned(), self_of(node))]);
        // A query that cannot be answered for a node says nothing of it.
        return Ok(program.evaluate(&variables).ok());
    }
    Ok(match node["attributes"][name].as_str() {
        Some("true") => Some(adp_cel::Value::Bool(true)),
        Some("false") => Some(adp_cel::Value::Bool(false)),
        Some(text) => text.parse().ok().map(adp_cel::Value::Double),
        None => None,
    })
}

/// `placed` with every pinned node of `nodes` at its pin: the law `pinnedStay`, which moves no other node.
fn pinned_over(placed: Vec<(String, f64, f64)>, nodes: &[Json]) -> Vec<(String, f64, f64)> {
    placed
        .into_iter()
        .map(|(id, x, y)| {
            let pin = nodes
                .iter()
                .find(|node| node["id"].as_str() == Some(id.as_str()))
                .and_then(|node| node["pinned"]["x"].as_f64().zip(node["pinned"]["y"].as_f64()));
            match pin {
                Some((x, y)) => (id, x, y),
                None => (id, x, y),
            }
        })
        .collect()
}

/// A capability's version: major, minor and patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u64, pub u64, pub u64);

impl Version {
    /// Whether this version is in `range`, read as npm's semver ranges are (ATD §13.2): comparators separated by
    /// spaces, all of which hold (`>=1.0 <2`), each `^`, `~`, `>=`, `>`, `<=`, `<`, `=` or none before a version whose
    /// minor and patch may be left out.
    pub fn satisfies(self, range: &str) -> bool {
        let comparators: Vec<&str> = range.split_whitespace().collect();
        !comparators.is_empty() && comparators.into_iter().all(|c| self.holds(c))
    }

    fn holds(self, comparator: &str) -> bool {
        let (operator, rest) = ["^", "~", ">=", "<=", ">", "<", "="]
            .iter()
            .find_map(|op| comparator.strip_prefix(op).map(|rest| (*op, rest)))
            .unwrap_or(("", comparator));
        let parts: Vec<Option<u64>> = rest
            .trim_start_matches('v')
            .split('.')
            .map(|p| p.parse().ok())
            .collect();
        if parts.is_empty() || parts.len() > 3 || parts.iter().any(Option::is_none) {
            return false;
        }
        let given = parts.len();
        let part = |i: usize| parts.get(i).copied().flatten().unwrap_or(0);
        let low = Version(part(0), part(1), part(2));
        // The first version past what a partial version names: 1 → 2.0.0, 1.2 → 1.3.0.
        let next = match given {
            1 => Version(low.0 + 1, 0, 0),
            2 => Version(low.0, low.1 + 1, 0),
            _ => Version(low.0, low.1, low.2 + 1),
        };
        match operator {
            "^" => {
                let high = if low.0 > 0 || given == 1 {
                    Version(low.0 + 1, 0, 0)
                } else if low.1 > 0 || given == 2 {
                    Version(0, low.1 + 1, 0)
                } else {
                    Version(0, 0, low.2 + 1)
                };
                low <= self && self < high
            }
            "~" => {
                let high = if given == 1 {
                    Version(low.0 + 1, 0, 0)
                } else {
                    Version(low.0, low.1 + 1, 0)
                };
                low <= self && self < high
            }
            ">=" => self >= low,
            ">" => self >= next,
            "<=" => self < next,
            "<" => self < low,
            _ => low <= self && self < next,
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
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
            .layouts
            .insert("layout.rankedBoundaries", Rc::new(ranked_boundaries::RankedBoundaries));
        capabilities
            .layouts
            .insert("layout.layeredColumns", Rc::new(layered_columns::LayeredColumns));
        capabilities.layouts.insert("layout.bands", Rc::new(bands::Bands));
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

    /// The version of capability `name` this host provides, none when it provides none. Every capability is at the
    /// version its contract in the catalogue states, 1.0.0 for all of them today.
    pub fn version(&self, name: &str) -> Option<Version> {
        let provided =
            self.layouts.contains_key(name) || self.routings.contains_key(name) || self.readers.contains_key(name);
        provided.then_some(Version(1, 0, 0))
    }

    /// Why this host cannot run a description that requires capability `name` in `range` (ATD §13.2, §13.4): it
    /// provides no such capability, or none of the versions the range allows; none when it can.
    pub fn lacks(&self, name: &str, range: &str) -> Option<String> {
        match self.version(name) {
            None => Some(format!("This host does not provide the capability `{name}`.")),
            Some(version) if !version.satisfies(range) => Some(format!(
                "This host provides the capability `{name}` at {version}, which `{range}` does not allow."
            )),
            Some(_) => None,
        }
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

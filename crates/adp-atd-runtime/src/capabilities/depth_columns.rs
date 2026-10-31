//! `layout.depthColumns` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): classes in
//! columns by their depth in a subclass hierarchy, each with its satellites in a lane beside it, and two bands of grid
//! beneath. Moved (not rewritten) from the RDF module's layout of the ontology reading and its hierarchy, which call it
//! here (etalii.adp spec 067, T040), themselves ported from the standalone host's `Owl/OwlLayout.cs` and
//! `Owl/OwlClassHierarchy.cs` at `8b6ccfc`.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_layout::cycle_policy::CyclePolicy;
use adp_layout::layering;
use indexmap::IndexMap;
use serde_json::Value as Json;

use super::{LayoutCapability, bool_query, list_query, number_argument, pinned_over, text_query};

/// Class to its asserted superclasses, in the order the classes were met.
pub type Supers = IndexMap<String, Vec<String>>;

/// What a node is to the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthRole {
    Header,
    Class,
    Individual,
    Satellite,
}

/// A node to place, in input order.
#[derive(Debug, Clone, PartialEq)]
pub struct DepthNode {
    pub id: String,
    pub width: f64,
    pub height: f64,
    pub role: DepthRole,
    /// A class's asserted superclasses.
    pub supers: Vec<String>,
    /// The node a satellite belongs to, or ''.
    pub owner: String,
}

/// The capability's arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthArguments {
    pub column_width: f64,
    pub lane_offset: f64,
    pub slot_height: f64,
    pub satellite_height: f64,
    pub satellite_offset: f64,
    pub header_gap: f64,
    pub band_gap: f64,
    pub band_columns: usize,
    pub band_column_gap: f64,
    pub band_row_gap: f64,
}

impl Default for DepthArguments {
    fn default() -> Self {
        DepthArguments {
            column_width: 530.0,
            lane_offset: 250.0,
            slot_height: 130.0,
            satellite_height: 110.0,
            satellite_offset: 10.0,
            header_gap: 80.0,
            band_gap: 140.0,
            band_columns: 5,
            band_column_gap: 60.0,
            band_row_gap: 50.0,
        }
    }
}

/// The top-left corner of every node, joined by `edges` from source to target.
pub fn place_depth_columns(
    nodes: &[DepthNode],
    edges: &[(String, String)],
    arguments: &DepthArguments,
) -> Vec<(String, f64, f64)> {
    let mut positions: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    let mut header_height = 0.0;
    if let Some(header) = nodes.iter().find(|n| n.role == DepthRole::Header) {
        positions.insert(header.id.clone(), (0.0, 0.0));
        header_height = header.height + arguments.header_gap;
    }

    let order: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id.as_str(), i)).collect();

    let class_ids: Vec<&str> = nodes
        .iter()
        .filter(|n| n.role == DepthRole::Class)
        .map(|n| n.id.as_str())
        .collect();
    let class_set: HashSet<&str> = class_ids.iter().copied().collect();
    let mut supers: Supers = class_ids.iter().map(|id| ((*id).to_owned(), Vec::new())).collect();
    for node in nodes.iter().filter(|n| n.role == DepthRole::Class) {
        for parent in &node.supers {
            if class_set.contains(parent.as_str()) {
                supers[node.id.as_str()].push(parent.clone());
            }
        }
    }
    let depths = depths(&supers);

    let mut satellites: HashMap<&str, Vec<&str>> = class_ids.iter().map(|id| (*id, Vec::new())).collect();
    for node in nodes {
        if positions.contains_key(&node.id)
            || class_set.contains(node.id.as_str())
            || node.role == DepthRole::Individual
        {
            continue;
        }
        let host = if !node.owner.is_empty() && satellites.contains_key(node.owner.as_str()) {
            Some(node.owner.as_str())
        } else {
            edges
                .iter()
                .filter(|(from, to)| *from == node.id || *to == node.id)
                .map(|(from, to)| if *from == node.id { to.as_str() } else { from.as_str() })
                .find(|other| satellites.contains_key(other))
        };
        if let Some(host) = host
            && let Some(list) = satellites.get_mut(host)
        {
            list.push(node.id.as_str());
        }
    }

    let mut columns: BTreeMap<i32, Vec<&str>> = BTreeMap::new();
    for id in &class_ids {
        let depth = depths.get(*id).copied().unwrap_or(0);
        columns.entry(depth).or_default().push(id);
    }

    let mut row_of: HashMap<&str, f64> = HashMap::new();
    let class_top = header_height;
    let mut max_y = class_top;
    for (depth, members) in &columns {
        let mut arranged = members.clone();
        if *depth == 0 {
            arranged.sort_by_key(|id| order[id]);
        } else {
            let key = |id: &&str| -> f64 {
                let parents: Vec<f64> = supers[*id]
                    .iter()
                    .filter_map(|parent| row_of.get(parent.as_str()).copied())
                    .collect();
                if parents.is_empty() {
                    f64::MAX
                } else {
                    parents.iter().sum::<f64>() / parents.len() as f64
                }
            };
            let mut keyed: Vec<(f64, usize, &str)> = arranged.iter().map(|id| (key(id), order[id], *id)).collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            arranged = keyed.into_iter().map(|(_, _, id)| id).collect();
        }

        let mut y = class_top;
        for id in arranged {
            let attached = &satellites[id];
            let x = f64::from(*depth) * arguments.column_width;
            positions.insert(id.to_owned(), (x, y));
            row_of.insert(id, y);
            for (i, satellite) in attached.iter().enumerate() {
                positions.insert(
                    (*satellite).to_owned(),
                    (
                        x + arguments.lane_offset,
                        y + arguments.satellite_offset + i as f64 * arguments.satellite_height,
                    ),
                );
            }
            let slot = arguments
                .slot_height
                .max(attached.len() as f64 * arguments.satellite_height + arguments.slot_height / 2.0);
            y += slot;
            max_y = max_y.max(y);
        }
    }

    let individuals: Vec<&DepthNode> = nodes
        .iter()
        .filter(|n| n.role == DepthRole::Individual && !positions.contains_key(&n.id))
        .collect();
    max_y = band(&mut positions, &individuals, max_y, arguments);
    let rest: Vec<&DepthNode> = nodes.iter().filter(|n| !positions.contains_key(&n.id)).collect();
    band(&mut positions, &rest, max_y, arguments);
    positions.into_iter().map(|(id, (x, y))| (id, x, y)).collect()
}

fn band(
    positions: &mut BTreeMap<String, (f64, f64)>,
    banded: &[&DepthNode],
    top: f64,
    arguments: &DepthArguments,
) -> f64 {
    if banded.is_empty() {
        return top;
    }
    let column_pitch = banded.iter().map(|n| n.width).fold(f64::MIN, f64::max) + arguments.band_column_gap;
    let row_pitch = banded.iter().map(|n| n.height).fold(f64::MIN, f64::max) + arguments.band_row_gap;
    let y = top + arguments.band_gap;
    let mut bottom = top;
    for (placed, node) in banded.iter().enumerate() {
        let (x, down) = adp_layout::grid_packer::cell(placed, arguments.band_columns, column_pitch, row_pitch);
        positions.insert(node.id.clone(), (x, y + down));
        bottom = bottom.max(y + down + row_pitch);
    }
    bottom
}

/// The capability over `$defs/LayoutInput`.
pub struct DepthColumns;

impl LayoutCapability for DepthColumns {
    fn draws_its_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let given = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut nodes = Vec::with_capacity(given.len());
        let mut pins = Vec::new();
        for node in given {
            let role = match text_query(node, args, "role")?.as_deref() {
                Some("header") => DepthRole::Header,
                Some("class") => DepthRole::Class,
                Some("individual") => DepthRole::Individual,
                _ => DepthRole::Satellite,
            };
            nodes.push(DepthNode {
                id: node["id"].as_str().unwrap_or_default().to_owned(),
                width: node["width"].as_f64().unwrap_or(0.0),
                height: node["height"].as_f64().unwrap_or(0.0),
                role,
                supers: list_query(node, args, "supers")?,
                owner: text_query(node, args, "owner")?.unwrap_or_default(),
            });
            if bool_query(node, args, "pinnable")?.unwrap_or(true) {
                pins.push(node.clone());
            }
        }
        let edges: Vec<(String, String)> = input["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|edge| {
                (
                    edge["source"].as_str().unwrap_or_default().to_owned(),
                    edge["target"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let arguments = DepthArguments {
            column_width: number_argument(args, "columnWidth", 530.0),
            lane_offset: number_argument(args, "laneOffset", 250.0),
            slot_height: number_argument(args, "slotHeight", 130.0),
            satellite_height: number_argument(args, "satelliteHeight", 110.0),
            satellite_offset: number_argument(args, "satelliteOffset", 10.0),
            header_gap: number_argument(args, "headerGap", 80.0),
            band_gap: number_argument(args, "bandGap", 140.0),
            band_columns: number_argument(args, "bandColumns", 5.0) as usize,
            band_column_gap: number_argument(args, "bandColumnGap", 60.0),
            band_row_gap: number_argument(args, "bandRowGap", 50.0),
        };
        Ok(pinned_over(place_depth_columns(&nodes, &edges, &arguments), &pins))
    }
}

/// The cycles: every strongly connected component with more than one member, plus every self-loop.
pub fn cycles(supers: &Supers) -> Vec<Vec<String>> {
    components(supers)
        .into_iter()
        .filter(|component| {
            component.len() > 1 || supers.get(&component[0]).is_some_and(|own| own.contains(&component[0]))
        })
        .collect()
}

/// Column depth per class: a root at 0, every other class one past its deepest asserted superclass, a cycle's members
/// sharing one depth.
pub fn depths(supers: &Supers) -> HashMap<String, i32> {
    let components = components(supers);
    let mut component_of: HashMap<&str, usize> = HashMap::new();
    for (i, component) in components.iter().enumerate() {
        for member in component {
            component_of.insert(member, i);
        }
    }
    let mut edges: Vec<(String, String)> = Vec::new();
    for (child, parents) in supers {
        for parent in parents {
            if let Some(&parent_component) = component_of.get(parent.as_str())
                && parent_component != component_of[child.as_str()]
            {
                edges.push((
                    components[parent_component][0].clone(),
                    components[component_of[child.as_str()]][0].clone(),
                ));
            }
        }
    }
    let component_depth = layering::longest_path(
        components.iter().map(|c| c[0].clone()),
        edges,
        CyclePolicy::BackEdgeIsZero,
    );
    component_of
        .iter()
        .map(|(member, &i)| ((*member).to_owned(), component_depth[&components[i][0]]))
        .collect()
}

/// Tarjan's strongly connected components over the adjacency, deterministic in key order.
fn components(supers: &Supers) -> Vec<Vec<String>> {
    struct Walk<'a> {
        supers: &'a Supers,
        index_of: HashMap<String, usize>,
        low_of: HashMap<String, usize>,
        on_stack: HashSet<String>,
        stack: Vec<String>,
        components: Vec<Vec<String>>,
        counter: usize,
    }

    impl Walk<'_> {
        fn connect(&mut self, node: &str) {
            self.index_of.insert(node.to_owned(), self.counter);
            self.low_of.insert(node.to_owned(), self.counter);
            self.counter += 1;
            self.stack.push(node.to_owned());
            self.on_stack.insert(node.to_owned());

            if let Some(parents) = self.supers.get(node) {
                for parent in parents {
                    if !self.supers.contains_key(parent) && !self.index_of.contains_key(parent) {
                        continue; // a superclass with no own subclass entry cannot be in a cycle
                    }
                    match self.index_of.get(parent).copied() {
                        None => {
                            self.connect(parent);
                            let low = self.low_of[node].min(self.low_of[parent]);
                            self.low_of.insert(node.to_owned(), low);
                        }
                        Some(parent_index) if self.on_stack.contains(parent) => {
                            let low = self.low_of[node].min(parent_index);
                            self.low_of.insert(node.to_owned(), low);
                        }
                        Some(_) => {}
                    }
                }
            }

            if self.low_of[node] == self.index_of[node] {
                let mut component = Vec::new();
                loop {
                    let member = self.stack.pop().expect("the node itself is on the stack");
                    self.on_stack.remove(&member);
                    let done = member == node;
                    component.push(member);
                    if done {
                        break;
                    }
                }
                component.reverse();
                self.components.push(component);
            }
        }
    }

    let mut walk = Walk {
        supers,
        index_of: HashMap::new(),
        low_of: HashMap::new(),
        on_stack: HashSet::new(),
        stack: Vec::new(),
        components: Vec::new(),
        counter: 0,
    };
    for node in supers.keys() {
        if !walk.index_of.contains_key(node) {
            walk.connect(node);
        }
    }
    walk.components
}

//! `layout.layeredRegions` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): regions
//! stacked top to bottom, each a layered hierarchy whose layers wrap onto centred rows, and a trailing grid. Moved (not
//! rewritten) from the RDF module's layout of the scheme reading, which calls it here (etalii.adp spec 067, T040), itself
//! ported from the standalone host's `Skos/SkosLayout.cs` at `8b6ccfc`. Cycles are detected here and nowhere else: layering
//! runs on an acyclic subset made by excluding, per knot, the edge whose (source, target) pair sorts lowest, still drawn,
//! and the module's validator reads the knots, so the picture and the report cannot disagree.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_layout::cycle_policy::CyclePolicy;
use adp_layout::{grid_packer, layering};
use serde_json::Value as Json;

use super::{LayoutCapability, list_query, number_argument, pinned_over, text_query};

/// A region: its node and its key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub id: String,
    pub key: String,
}

/// A member: its node and the keys of the regions it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub id: String,
    pub regions: Vec<String>,
}

/// An edge from a broader node to a narrower one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionEdge {
    pub id: String,
    pub from: String,
    pub to: String,
}

/// One knot in the edges: its members sorted, the edge left out of the layering, and the edges on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Knot {
    pub members: Vec<String>,
    pub excluded: String,
    pub edges: Vec<String>,
}

/// The capability's arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionArguments {
    pub column_width: f64,
    pub row_height: f64,
    pub region_gap: f64,
    pub region_header: f64,
    pub layer_gap: f64,
    pub minimum_columns: usize,
}

impl Default for RegionArguments {
    fn default() -> Self {
        RegionArguments {
            column_width: 240.0,
            row_height: 110.0,
            region_gap: 90.0,
            region_header: 60.0,
            layer_gap: 50.0,
            minimum_columns: 8,
        }
    }
}

/// The widest a row may be, in columns: about the square root of twice the elements to place.
fn columns_for(count: usize, arguments: &RegionArguments) -> usize {
    arguments
        .minimum_columns
        .max((2.0 * count as f64).sqrt().ceil() as usize)
}

/// The top-left corner of every node placed, and the knots found.
pub fn place_regions(
    regions: &[Region],
    members: &[Member],
    trailing: &[Region],
    edges: &[RegionEdge],
    arguments: &RegionArguments,
) -> (BTreeMap<String, (f64, f64)>, Vec<Knot>) {
    let (acyclic, knots) = break_knots(edges.iter().collect());

    let mut broaders_of: HashMap<&str, Vec<&str>> = HashMap::new();
    for edge in &acyclic {
        broaders_of
            .entry(edge.to.as_str())
            .or_default()
            .push(edge.from.as_str());
    }

    let mut band = Band {
        acyclic: &acyclic,
        broaders_of: &broaders_of,
        positions: BTreeMap::new(),
        placed: HashSet::new(),
        arguments,
    };
    let mut y = 0.0;

    let mut sorted: Vec<&Region> = regions.iter().collect();
    sorted.sort_by(|a, b| a.key.cmp(&b.key));
    for region in sorted {
        band.positions.insert(region.id.clone(), (0.0, y));
        let held: HashSet<String> = members
            .iter()
            .filter(|m| m.regions.contains(&region.key) && !band.placed.contains(&m.id))
            .map(|m| m.id.clone())
            .collect();
        y = band.place(&held, y + arguments.region_header) + arguments.region_gap;
    }

    let unfiled: HashSet<String> = members
        .iter()
        .filter(|m| !band.placed.contains(&m.id))
        .map(|m| m.id.clone())
        .collect();
    if !unfiled.is_empty() {
        y = band.place(&unfiled, y) + arguments.region_gap;
    }

    let mut last: Vec<&Region> = trailing.iter().collect();
    last.sort_by(|a, b| a.key.cmp(&b.key));
    let last_columns = columns_for(last.len(), arguments);
    for (index, node) in last.iter().enumerate() {
        let (x, down) = grid_packer::cell(index, last_columns, arguments.column_width, arguments.row_height);
        band.positions.insert(node.id.clone(), (x, y + down));
    }

    (band.positions, knots)
}

struct Band<'a> {
    acyclic: &'a [&'a RegionEdge],
    broaders_of: &'a HashMap<&'a str, Vec<&'a str>>,
    positions: BTreeMap<String, (f64, f64)>,
    placed: HashSet<String>,
    arguments: &'a RegionArguments,
}

impl Band<'_> {
    /// Layers a band's members from `top`, and answers the y just past the band.
    fn place(&mut self, members: &HashSet<String>, top: f64) -> f64 {
        let mut nodes: Vec<&String> = members.iter().collect();
        nodes.sort();
        let depth = layering::longest_path(
            nodes,
            self.acyclic.iter().map(|e| (e.from.as_str(), e.to.as_str())),
            CyclePolicy::Kahn,
        );
        let mut by_depth: Vec<(i32, Vec<String>)> = Vec::new();
        for (id, level) in depth {
            match by_depth.iter_mut().find(|(d, _)| *d == level) {
                Some((_, ids)) => ids.push(id),
                None => by_depth.push((level, vec![id])),
            }
        }
        by_depth.sort_by_key(|(d, _)| *d);

        let mut sequence: HashMap<String, usize> = HashMap::new();
        let mut ordered: Vec<Vec<String>> = Vec::new();
        for (_, mut level) in by_depth {
            let keys: HashMap<String, f64> = level
                .iter()
                .map(|id| (id.clone(), self.broader_position(id, &sequence)))
                .collect();
            level.sort_by(|a, b| keys[a].total_cmp(&keys[b]).then_with(|| a.cmp(b)));
            for id in &level {
                let next = sequence.len();
                sequence.insert(id.clone(), next);
            }
            ordered.push(level);
        }

        let columns = columns_for(members.len(), self.arguments);
        let band_columns = ordered
            .iter()
            .map(Vec::len)
            .max()
            .map_or(0, |widest| columns.min(widest));
        let mut row_top = top;
        for level in &ordered {
            for row in level.chunks(columns) {
                let indent = (band_columns as f64 - row.len() as f64) / 2.0;
                for (x, id) in row.iter().enumerate() {
                    self.positions
                        .insert(id.clone(), ((indent + x as f64) * self.arguments.column_width, row_top));
                    self.placed.insert(id.clone());
                }
                row_top += self.arguments.row_height;
            }
            row_top += self.arguments.layer_gap;
        }
        if ordered.is_empty() {
            top
        } else {
            row_top - self.arguments.layer_gap
        }
    }

    /// The mean place of a concept's already-placed broader concepts, or the largest number when it has none.
    fn broader_position(&self, id: &str, sequence: &HashMap<String, usize>) -> f64 {
        let places: Vec<f64> = self
            .broaders_of
            .get(id)
            .map(|broaders| {
                broaders
                    .iter()
                    .filter_map(|b| sequence.get(*b).map(|s| *s as f64))
                    .collect()
            })
            .unwrap_or_default();
        if places.is_empty() {
            f64::MAX
        } else {
            places.iter().sum::<f64>() / places.len() as f64
        }
    }
}

/// The acyclic layering subset and the knots found: per remaining knot, the edge whose (source, target) pair sorts lowest
/// is excluded and detection reruns, until nothing knots.
fn break_knots(edges: Vec<&RegionEdge>) -> (Vec<&RegionEdge>, Vec<Knot>) {
    let mut included = edges;
    let mut knots = Vec::new();
    loop {
        let Some(knot) = find_knot(&included) else {
            return (included, knots);
        };
        let knot_edges: Vec<&RegionEdge> = included
            .iter()
            .copied()
            .filter(|e| knot.contains(&e.from) && knot.contains(&e.to))
            .collect();
        let excluded = knot_edges
            .iter()
            .copied()
            .min_by(|a, b| a.from.cmp(&b.from).then_with(|| a.to.cmp(&b.to)))
            .expect("a knot has edges");
        let at = included
            .iter()
            .position(|e| std::ptr::eq(*e, excluded))
            .expect("the excluded edge is included");
        included.remove(at);
        let mut members: Vec<String> = knot.into_iter().collect();
        members.sort();
        knots.push(Knot {
            members,
            excluded: excluded.id.clone(),
            edges: knot_edges.iter().map(|e| e.id.clone()).collect(),
        });
    }
}

/// One strongly connected set of two or more nodes (or a self-loop), or none when none is left: Tarjan's algorithm,
/// iteratively, from the sources in ordinal order.
fn find_knot(edges: &[&RegionEdge]) -> Option<HashSet<String>> {
    if let Some(edge) = edges.iter().find(|e| e.from == e.to) {
        return Some(HashSet::from([edge.from.clone()]));
    }

    let mut next: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut starts: Vec<&str> = Vec::new();
    for edge in edges {
        if !next.contains_key(edge.from.as_str()) {
            starts.push(edge.from.as_str());
        }
        next.entry(edge.from.as_str()).or_default().push(edge.to.as_str());
    }
    starts.sort();

    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut low: HashMap<&str, usize> = HashMap::new();
    let mut on_stack: HashSet<&str> = HashSet::new();
    let mut stack: Vec<&str> = Vec::new();
    let mut counter = 0;
    let empty: Vec<&str> = Vec::new();

    for start in starts {
        if index.contains_key(start) {
            continue;
        }
        let mut work: Vec<(&str, usize)> = vec![(start, 0)];
        while let Some((node, child)) = work.pop() {
            if child == 0 {
                index.insert(node, counter);
                low.insert(node, counter);
                counter += 1;
                stack.push(node);
                on_stack.insert(node);
            }
            let children = next.get(node).unwrap_or(&empty);
            if child < children.len() {
                work.push((node, child + 1));
                let target = children[child];
                match index.get(target) {
                    None => work.push((target, 0)),
                    Some(&target_index) if on_stack.contains(target) => {
                        let lowest = low[node].min(target_index);
                        low.insert(node, lowest);
                    }
                    Some(_) => {}
                }
            } else {
                if low[node] == index[node] {
                    let mut component = HashSet::new();
                    loop {
                        let popped = stack.pop().expect("the node is on the stack");
                        on_stack.remove(popped);
                        component.insert(popped.to_owned());
                        if popped == node {
                            break;
                        }
                    }
                    if component.len() > 1 {
                        return Some(component);
                    }
                }
                if let Some(&(parent, _)) = work.last() {
                    let lowest = low[parent].min(low[node]);
                    low.insert(parent, lowest);
                }
            }
        }
    }
    None
}

/// The capability over `$defs/LayoutInput`.
pub struct LayeredRegions;

impl LayoutCapability for LayeredRegions {
    fn draws_its_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let (mut regions, mut members, mut trailing) = (Vec::new(), Vec::new(), Vec::new());
        let mut order = Vec::with_capacity(nodes.len());
        for node in nodes {
            let id = node["id"].as_str().unwrap_or_default().to_owned();
            order.push(id.clone());
            let key = || -> Result<String, String> { Ok(text_query(node, args, "key")?.unwrap_or_default()) };
            match text_query(node, args, "role")?.as_deref() {
                Some("region") => regions.push(Region { id, key: key()? }),
                Some("member") => members.push(Member {
                    id,
                    regions: list_query(node, args, "regions")?,
                }),
                _ => trailing.push(Region { id, key: key()? }),
            }
        }
        let edges: Vec<RegionEdge> = input["edges"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|edge| RegionEdge {
                id: edge["id"].as_str().unwrap_or_default().to_owned(),
                from: edge["source"].as_str().unwrap_or_default().to_owned(),
                to: edge["target"].as_str().unwrap_or_default().to_owned(),
            })
            .collect();
        let arguments = RegionArguments {
            column_width: number_argument(args, "columnWidth", 240.0),
            row_height: number_argument(args, "rowHeight", 110.0),
            region_gap: number_argument(args, "regionGap", 90.0),
            region_header: number_argument(args, "regionHeader", 60.0),
            layer_gap: number_argument(args, "layerGap", 50.0),
            minimum_columns: number_argument(args, "minimumColumns", 8.0) as usize,
        };
        let (positions, _) = place_regions(&regions, &members, &trailing, &edges, &arguments);
        let placed = order
            .into_iter()
            .filter_map(|id| positions.get(&id).map(|(x, y)| (id.clone(), *x, *y)))
            .collect();
        Ok(pinned_over(placed, nodes))
    }
}

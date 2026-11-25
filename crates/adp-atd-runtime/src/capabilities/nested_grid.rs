//! `layout.nestedGrid` (ATD §13.3; its contract is in `specifications/atd/capabilities/`): a containment laid out as
//! nested grids, bottom up. Each container lays out its own nodes first and then its child containers, in rows of
//! `columns` cells, a row as tall as its tallest member; a child container is one cell the size of its own grid, plus
//! `padding` on every side and `labelBand` above its contents, and never smaller than its own size in the input. The
//! roots are laid out the same way from `origin`, and each container is given its size. A node the input marks a container (its `container` attribute) is one
//! even when it holds nothing, its empty grid then `emptyGrid` in size. A pinned container moves as a frame, its contents following it; a
//! pinned node stays at its pin. Moved from the SPARQL module's scope grid (`modules/diagrams/sparql/src/layout.rs`),
//! itself ported from the standalone host's `SparqlLayout.cs` at `8b6ccfc`.

use std::collections::HashMap;

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument};

/// The capability's arguments.
#[derive(Debug, Clone, PartialEq)]
struct Arguments {
    columns: usize,
    column_gap: f64,
    row_gap: f64,
    padding: f64,
    label_band: f64,
    origin: (f64, f64),
    empty_grid: (f64, f64),
}

impl Arguments {
    fn of(args: &Json) -> Self {
        let pair = |name: &str, i: usize, default: f64| {
            args[name]
                .as_array()
                .and_then(|o| o.get(i))
                .and_then(Json::as_f64)
                .unwrap_or(default)
        };
        Arguments {
            columns: (number_argument(args, "columns", 3.0) as usize).max(1),
            column_gap: number_argument(args, "columnGap", 60.0),
            row_gap: number_argument(args, "rowGap", 48.0),
            padding: number_argument(args, "padding", 36.0),
            label_band: number_argument(args, "labelBand", 30.0),
            origin: (pair("origin", 0, 40.0), pair("origin", 1, 90.0)),
            empty_grid: (pair("emptyGrid", 0, 170.0), pair("emptyGrid", 1, 32.0)),
        }
    }
}

struct Node<'a> {
    id: &'a str,
    size: (f64, f64),
    pinned: Option<(f64, f64)>,
    /// Marked a container by the input, whether or not it holds anything.
    container: bool,
}

struct Nesting<'a> {
    nodes: Vec<Node<'a>>,
    /// The members of each container by index, its own nodes first and then its containers, each in input order; the
    /// roots under `None`.
    members: HashMap<Option<usize>, Vec<usize>>,
    arguments: Arguments,
}

impl<'a> Nesting<'a> {
    fn of(input: &'a [Json], arguments: Arguments) -> Self {
        let nodes: Vec<Node<'a>> = input
            .iter()
            .map(|node| Node {
                id: node["id"].as_str().unwrap_or_default(),
                size: (
                    node["width"].as_f64().unwrap_or(0.0),
                    node["height"].as_f64().unwrap_or(0.0),
                ),
                pinned: node["pinned"]["x"].as_f64().zip(node["pinned"]["y"].as_f64()),
                container: matches!(&node["attributes"]["container"], Json::Bool(true))
                    || node["attributes"]["container"].as_str() == Some("true"),
            })
            .collect();
        let index: HashMap<&str, usize> = nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let parent_of: Vec<Option<usize>> = input
            .iter()
            .map(|node| node["parent"].as_str().and_then(|p| index.get(p).copied()))
            .collect();
        let mut members: HashMap<Option<usize>, Vec<usize>> = HashMap::new();
        for (i, parent) in parent_of.iter().enumerate() {
            members.entry(*parent).or_default().push(i);
        }
        let holds = |i: &usize| nodes[*i].container || members.contains_key(&Some(*i));
        let ordered: HashMap<Option<usize>, Vec<usize>> = members
            .iter()
            .map(|(parent, list)| {
                let (own, held): (Vec<usize>, Vec<usize>) = list.iter().partition(|i| !holds(i));
                (*parent, own.into_iter().chain(held).collect())
            })
            .collect();
        Nesting {
            nodes,
            members: ordered,
            arguments,
        }
    }

    fn is_container(&self, i: usize) -> bool {
        self.nodes[i].container || self.members.contains_key(&Some(i))
    }

    /// The members of `parent`, each with its size: a container's is its own grid's.
    fn sized(&self, parent: Option<usize>) -> Vec<(usize, (f64, f64))> {
        self.members
            .get(&parent)
            .into_iter()
            .flatten()
            .map(|&i| (i, self.size_of(i)))
            .collect()
    }

    fn size_of(&self, i: usize) -> (f64, f64) {
        if !self.is_container(i) {
            return self.nodes[i].size;
        }
        let members = self.sized(Some(i));
        let sizes: Vec<(f64, f64)> = members.iter().map(|(_, size)| *size).collect();
        let a = &self.arguments;
        let (width, height) = if sizes.is_empty() {
            a.empty_grid
        } else {
            let grid = adp_layout::grid_packer::pack(&sizes, a.columns, a.column_gap, a.row_gap);
            (grid.width, grid.height)
        };
        let (min_width, min_height) = self.nodes[i].size;
        (
            (width + 2.0 * a.padding).max(min_width),
            (height + 2.0 * a.padding + a.label_band).max(min_height),
        )
    }

    /// Places the members of `parent` from `(x, y)`, and each container's own members inside it.
    fn place(&self, parent: Option<usize>, x: f64, y: f64, placed: &mut Vec<(usize, f64, f64)>) {
        let members = self.sized(parent);
        let sizes: Vec<(f64, f64)> = members.iter().map(|(_, size)| *size).collect();
        let a = &self.arguments;
        let grid = adp_layout::grid_packer::pack(&sizes, a.columns, a.column_gap, a.row_gap);
        for ((i, _), cell) in members.iter().zip(grid.cells.iter()) {
            let (cx, cy) = (x + cell.x, y + cell.y);
            placed.push((*i, cx, cy));
            if self.is_container(*i) {
                self.place(Some(*i), cx + a.padding, cy + a.label_band + a.padding, placed);
            }
        }
    }

    /// The members of `i`, at any depth.
    fn within(&self, i: usize, out: &mut Vec<usize>) {
        for &member in self.members.get(&Some(i)).into_iter().flatten() {
            out.push(member);
            self.within(member, out);
        }
    }
}

/// The capability over `$defs/LayoutInput`.
pub struct NestedGrid;

impl LayoutCapability for NestedGrid {
    /// Every container's size, its own grid's with padding and label band, in the order they are placed.
    fn sizes(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let arguments = Arguments::of(args);
        let (x, y) = arguments.origin;
        let nesting = Nesting::of(nodes, arguments);
        let mut placed = Vec::new();
        nesting.place(None, x, y, &mut placed);
        Ok(placed
            .iter()
            .filter(|(i, ..)| nesting.is_container(*i))
            .map(|(i, ..)| {
                let (width, height) = nesting.size_of(*i);
                (nesting.nodes[*i].id.to_owned(), width, height)
            })
            .collect())
    }

    /// A node is drawn at the size it was placed with, its notation's fixed one.
    fn draws_its_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let arguments = Arguments::of(args);
        let (x, y) = arguments.origin;
        let nesting = Nesting::of(nodes, arguments);
        let mut placed = Vec::new();
        nesting.place(None, x, y, &mut placed);
        let mut at: HashMap<usize, (f64, f64)> = placed.iter().map(|(i, x, y)| (*i, (*x, *y))).collect();
        // A pinned container is a frame its computed contents follow, outermost first.
        for &(i, ..) in &placed {
            let (Some((px, py)), true) = (nesting.nodes[i].pinned, nesting.is_container(i)) else {
                continue;
            };
            let (x, y) = at[&i];
            let (dx, dy) = (px - x, py - y);
            let mut moved = vec![i];
            nesting.within(i, &mut moved);
            for member in moved {
                if let Some(position) = at.get_mut(&member) {
                    *position = (position.0 + dx, position.1 + dy);
                }
            }
        }
        // A pinned node stays at its pin.
        for &(i, ..) in &placed {
            if let (Some(pin), false) = (nesting.nodes[i].pinned, nesting.is_container(i)) {
                at.insert(i, pin);
            }
        }
        Ok(placed
            .iter()
            .map(|(i, ..)| {
                let (x, y) = at[i];
                (nesting.nodes[*i].id.to_owned(), x, y)
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn place(input: Json) -> HashMap<String, (f64, f64)> {
        NestedGrid
            .place(&input, &json!({}))
            .unwrap()
            .into_iter()
            .map(|(id, x, y)| (id, (x, y)))
            .collect()
    }

    #[test]
    fn a_container_is_one_cell_the_size_of_its_grid_after_the_nodes() {
        let at = place(json!({ "nodes": [
            { "id": "r", "width": 170, "height": 94 },
            { "id": "a", "width": 170, "height": 64 },
            { "id": "b", "width": 170, "height": 64, "parent": "r" },
            { "id": "c", "width": 170, "height": 64 }
        ], "edges": [] }));
        // Roots: a, c, then the container r, three to a row from the origin.
        assert_eq!(at["a"], (40.0, 90.0));
        assert_eq!(at["c"], (270.0, 90.0));
        assert_eq!(at["r"], (500.0, 90.0));
        // r's contents: padding inside its edge, below its label band.
        assert_eq!(at["b"], (536.0, 156.0));
    }

    #[test]
    fn a_pinned_container_moves_its_contents_with_it() {
        let at = place(json!({ "nodes": [
            { "id": "r", "width": 170, "height": 94, "pinned": { "x": 0, "y": 0 } },
            { "id": "b", "width": 170, "height": 64, "parent": "r" }
        ], "edges": [] }));
        assert_eq!(at["r"], (0.0, 0.0));
        assert_eq!(at["b"], (36.0, 66.0));
    }
}

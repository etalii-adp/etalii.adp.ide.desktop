//! `layout.grid` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): a grid filled row by
//! row in input order, each row below the tallest node of the row above. What the RDF module's layout of the shapes
//! reading does, which calls it here (etalii.adp spec 067, T040), itself ported from the standalone host's
//! `ShaclLayout.cs` at `8b6ccfc`.

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, pinned_over};

/// The top-left corner of every node, given with its width and its height, in input order.
pub fn place_grid(
    nodes: &[(String, f64, f64)],
    columns: usize,
    column_gap: f64,
    row_gap: f64,
) -> Vec<(String, f64, f64)> {
    let sizes: Vec<(f64, f64)> = nodes.iter().map(|(_, width, height)| (*width, *height)).collect();
    let grid = adp_layout::grid_packer::pack(&sizes, columns, column_gap, row_gap);
    nodes
        .iter()
        .zip(grid.cells.iter())
        .map(|((id, ..), cell)| (id.clone(), cell.x, cell.y))
        .collect()
}

/// The capability over `$defs/LayoutInput`.
pub struct Grid;

impl LayoutCapability for Grid {
    fn draws_its_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let sized: Vec<(String, f64, f64)> = nodes
            .iter()
            .map(|node| {
                (
                    node["id"].as_str().unwrap_or_default().to_owned(),
                    node["width"].as_f64().unwrap_or(0.0),
                    node["height"].as_f64().unwrap_or(0.0),
                )
            })
            .collect();
        let placed = place_grid(
            &sized,
            number_argument(args, "columns", 3.0) as usize,
            number_argument(args, "columnGap", 0.0),
            number_argument(args, "rowGap", 48.0),
        );
        Ok(pinned_over(placed, nodes))
    }
}

//! `layout.bandedGrid` (ATD §13.3; its contract and fixtures are in `specifications/atd/capabilities/`): bands of
//! nodes stacked top to bottom, each band a grid filled row by row. Moved (not rewritten) from the RDF module's layout of
//! the data graph, which calls it here (etalii.adp spec 067, T040), itself ported from the standalone host's
//! `RdfLayout.cs` at `8b6ccfc`.

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, pinned_over, text_query};

/// The capability's arguments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandedGridArguments {
    pub columns: usize,
    pub column_width: f64,
    pub row_height: f64,
    pub band_gap: f64,
}

/// The top-left corner of every node, each given as its id and its band's key: the bands ordered by their keys' UTF-8
/// bytes, the empty key last, and the nodes of a band in input order.
pub fn place_banded(nodes: &[(String, String)], arguments: &BandedGridArguments) -> Vec<(String, f64, f64)> {
    let mut bands: Vec<(&str, Vec<&str>)> = Vec::new();
    for (id, key) in nodes {
        match bands.iter_mut().find(|(k, _)| k == key) {
            Some((_, members)) => members.push(id),
            None => bands.push((key, vec![id])),
        }
    }
    bands.sort_by(|(a, _), (b, _)| {
        a.is_empty()
            .cmp(&b.is_empty())
            .then_with(|| a.as_bytes().cmp(b.as_bytes()))
    });
    let mut placed = Vec::with_capacity(nodes.len());
    let mut y = 0.0;
    for (_, members) in &bands {
        for (index, id) in members.iter().enumerate() {
            let (x, down) =
                adp_layout::grid_packer::cell(index, arguments.columns, arguments.column_width, arguments.row_height);
            placed.push(((*id).to_owned(), x, y + down));
        }
        y += adp_layout::grid_packer::rows(members.len(), arguments.columns) as f64 * arguments.row_height
            + arguments.band_gap;
    }
    placed
}

/// The capability over `$defs/LayoutInput`.
pub struct BandedGrid;

impl LayoutCapability for BandedGrid {
    fn draws_its_sizes(&self) -> bool {
        true
    }

    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let mut keyed = Vec::with_capacity(nodes.len());
        for node in nodes {
            let key = text_query(node, args, "band")?.unwrap_or_default();
            keyed.push((node["id"].as_str().unwrap_or_default().to_owned(), key));
        }
        let arguments = BandedGridArguments {
            columns: number_argument(args, "columns", 4.0) as usize,
            column_width: number_argument(args, "columnWidth", 280.0),
            row_height: number_argument(args, "rowHeight", 160.0),
            band_gap: number_argument(args, "bandGap", 80.0),
        };
        Ok(pinned_over(place_banded(&keyed, &arguments), nodes))
    }
}

//! `layout.bands` (ATD §13.3; its contract and 7 fixtures are in `specifications/atd/capabilities/`): horizontal bands at
//! fixed heights, each band's members side by side. Moved (not rewritten) from the Databricks module's layout of the
//! bundle reading, which calls it here (etalii.adp spec 067, T027), itself ported from the standalone host's
//! `DatabricksBundleLayout.cs` at `8b6ccfc`.

use serde_json::Value as Json;

use super::{LayoutCapability, number_argument, query_text, self_of};

/// One band, from top to bottom: its top edge and its members, in input order.
#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    pub y: f64,
    pub members: Vec<String>,
}

/// The top-left corner of every member of `bands`, band by band: a node two bands hold comes twice, the later place the
/// one that counts.
pub fn place_bands(bands: &[Band], column_width: f64) -> Vec<(String, f64, f64)> {
    bands
        .iter()
        .flat_map(|band| {
            band.members
                .iter()
                .enumerate()
                .map(move |(index, id)| (id.clone(), index as f64 * column_width, band.y))
        })
        .collect()
}

/// The capability over `$defs/LayoutInput`.
pub struct Bands;

impl LayoutCapability for Bands {
    fn place(&self, input: &Json, args: &Json) -> Result<Vec<(String, f64, f64)>, String> {
        let nodes = input["nodes"].as_array().ok_or("The input has no `nodes`.")?;
        let given = args["bands"].as_array().ok_or("The arguments have no `bands`.")?;
        let mut bands = Vec::with_capacity(given.len());
        for band in given {
            let source = query_text(&band["members"]).ok_or("A band has no `members`.")?;
            let program = adp_cel::Environment::standard(&["self"])
                .compile(source)
                .map_err(|e| format!("A band's `members`: {e}"))?;
            let mut members = Vec::new();
            for node in nodes {
                let variables = std::collections::HashMap::from([("self".to_owned(), self_of(node))]);
                if program
                    .evaluate(&variables)
                    .map_err(|e| format!("A band's `members`: {e}"))?
                    == adp_cel::Value::Bool(true)
                {
                    members.push(node["id"].as_str().unwrap_or_default().to_owned());
                }
            }
            bands.push(Band {
                y: band["y"].as_f64().unwrap_or(0.0),
                members,
            });
        }
        let width = number_argument(args, "columnWidth", 260.0);
        Ok(super::pinned_over(place_bands(&bands, width), nodes))
    }
}

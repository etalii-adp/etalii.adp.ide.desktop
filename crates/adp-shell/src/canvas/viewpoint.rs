//! What the canvas draws beside a specification's diagram where the standalone host's library derived it from the
//! specification (etalii.adp spec 030, task T073; `compileNotation.ts` and `definition/chrome.ts` at `8b6ccfc`): a
//! viewpoint the person toggles, its notation drawn in place of the default's, and the legend of an enumeration's
//! colours.
//!
//! - **A toggled viewpoint** (DISL section 11, `variantOf` with a `toggle`): the hype cycle graph's Compact. The
//!   toggle is the viewer's: it is never saved, and every diagram opens in its default viewpoint. Its notation is the
//!   default's with the viewpoint's node notations merged over it, compiled once, so a node type drawn otherwise there
//!   (a trend at 24 units a phase with even phases) is drawn so, and one its placement does not let move or resize
//!   does neither. Where the viewpoint's layout places the elements is the tool type's: the hype cycle graph's
//!   row-packed layout is `adp_diagram_hype_cycle::compact`.
//! - **The legend** (`notation.canvas.legend`): one swatch per value of each enumeration it names, painted with the
//!   value's colour token in the theme's mode, under its label, at the top left of the canvas.

use adp_disl::{DislLoader, DislPluginFunctions, DislSpecification};
use adp_scene::{DiagramDefinition, Theme as NotationTheme, compile_notation};
use serde_json::Value as Json;

/// A viewpoint the person toggles: its id and the label of its switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toggle {
    pub id: String,
    pub label: String,
}

/// The first viewpoint of `specification` a switch toggles (`toggle`), when it varies another (`variantOf`).
pub fn toggle_of(specification: &DislSpecification) -> Option<Toggle> {
    let viewpoints = specification.root().get("viewpoints")?.as_object()?;
    viewpoints.iter().find_map(|(id, viewpoint)| {
        viewpoint.get("variantOf")?;
        let toggle = viewpoint.get("toggle")?;
        let label = toggle
            .get("label")
            .or_else(|| viewpoint.get("label"))
            .and_then(Json::as_str)
            .unwrap_or(id);
        Some(Toggle {
            id: id.clone(),
            label: label.to_owned(),
        })
    })
}

/// `into` with `over` merged into it: an object key by key, anything else replaced.
fn merge(into: &mut Json, over: &Json) {
    match (into, over) {
        (Json::Object(into), Json::Object(over)) => {
            for (key, value) in over {
                match into.get_mut(key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        into.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (into, over) => *into = over.clone(),
    }
}

/// The specification as viewpoint `id` draws it: its node notations merged over the default's.
pub fn variant_specification(
    specification: &DislSpecification,
    id: &str,
    plugins: &DislPluginFunctions,
) -> Result<DislSpecification, String> {
    let mut root = specification.root().clone();
    let notation = root
        .get("viewpoints")
        .and_then(|v| v.get(id))
        .and_then(|v| v.get("notation"))
        .cloned()
        .ok_or_else(|| format!("The viewpoint '{id}' draws nothing of its own."))?;
    if let Some(nodes) = notation.get("nodes")
        && let Some(target) = root.get_mut("notation").and_then(|n| n.get_mut("nodes"))
    {
        merge(target, nodes);
    }
    if let Some(edges) = notation.get("edges")
        && let Some(target) = root.get_mut("notation").and_then(|n| n.get_mut("edges"))
    {
        merge(target, edges);
    }
    let text = serde_json::to_string(&root).map_err(|e| e.to_string())?;
    let loaded = DislLoader::load_bytes(text.as_bytes(), plugins);
    let refusal = match loaded.errors().next() {
        Some(error) => format!("The viewpoint '{id}' does not load: {}", error.message),
        None => format!("The viewpoint '{id}' does not load."),
    };
    loaded.specification.ok_or(refusal)
}

/// The notation viewpoint `id` of `specification` is drawn with, compiled.
pub fn variant_definition(
    specification: &DislSpecification,
    id: &str,
    plugins: &DislPluginFunctions,
) -> Result<DiagramDefinition, String> {
    compile_notation(&variant_specification(specification, id, plugins)?).map_err(|e| e.to_string())
}

/// One entry of the legend: its label and its colour as the notation's theme writes it, in `mode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegendEntry {
    pub label: String,
    pub colour: Option<String>,
}

/// The legend of `specification` (`notation.canvas.legend`), in the theme's `mode`: every value of each enumeration
/// its `entries` name, in order; none when it shows none.
pub fn legend_of(specification: &DislSpecification, mode: Option<&str>) -> Vec<LegendEntry> {
    let root = specification.root();
    let Some(legend) = root
        .get("notation")
        .and_then(|n| n.get("canvas"))
        .and_then(|c| c.get("legend"))
    else {
        return Vec::new();
    };
    if legend.get("visible").and_then(Json::as_bool) == Some(false) {
        return Vec::new();
    }
    let theme = NotationTheme::of(specification, mode);
    let enums = root.get("metamodel").and_then(|m| m.get("enums"));
    legend
        .get("entries")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
        .filter_map(Json::as_str)
        .filter_map(|name| enums?.get(name)?.get("values")?.as_object())
        .flat_map(|values| values.iter())
        .map(|(key, value)| LegendEntry {
            label: value.get("label").and_then(Json::as_str).unwrap_or(key).to_owned(),
            colour: value
                .get("color")
                .and_then(|c| c.get("token"))
                .and_then(Json::as_str)
                .and_then(|token| theme.text_of(token)),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn hype_cycle() -> DislSpecification {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/etalii.adp/definitions/diagrams/gartner-hype-cycle-graph.dis");
        DislLoader::load(&std::fs::read_to_string(path).unwrap())
            .specification
            .unwrap()
    }

    #[test]
    fn the_hype_cycle_graph_toggles_compact() {
        assert_eq!(
            toggle_of(&hype_cycle()),
            Some(Toggle {
                id: "compact".into(),
                label: "Compact".into()
            })
        );
    }

    #[test]
    fn compact_draws_a_trend_that_neither_moves_nor_resizes() {
        let definition = variant_definition(&hype_cycle(), "compact", &DislPluginFunctions::none()).unwrap();
        let trend = definition.element_type("Trend").unwrap();
        assert!(!trend.draggable);
        let note = definition.element_type("Note").unwrap();
        assert!(!note.draggable);
        let default = compile_notation(&hype_cycle()).unwrap();
        assert!(default.element_type("Trend").unwrap().draggable);
    }

    #[test]
    fn the_legend_is_one_swatch_per_phase_in_the_themes_colours() {
        let light = legend_of(&hype_cycle(), None);
        assert_eq!(
            light.iter().map(|e| e.label.as_str()).collect::<Vec<_>>(),
            ["Peak", "Trough", "Slope", "Plateau"]
        );
        assert_eq!(light[0].colour.as_deref(), Some("#f7e7a1"));
        assert_eq!(
            legend_of(&hype_cycle(), Some("dark"))[3].colour.as_deref(),
            Some("#3f6212")
        );
    }
}

//! The recording painter (etalii.adp spec 035, T051) on two scenes their modules draw by hand: the Sankey diagram's
//! bars and bands and the Wardley map's components on its axes, recorded headlessly as `atd record` records them.

use std::collections::HashSet;

use adp_scene::scene::{Owner, Primitive};
use adp_theme::Kind;
use serde_json::Value;

use super::{Harness, vendor};
use crate::canvas::recording::drawing;

/// ATD's definition of a drawing (`$defs/Drawing`).
fn drawing_schema() -> jsonschema::Validator {
    let path = vendor().join("etalii.adp/specifications/atd/atd.schema.json");
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    // The definition stands on its own: it refers to nothing outside it.
    jsonschema::validator_for(&schema["$defs"]["Drawing"]).unwrap()
}

/// Records the harness's diagram, checks the drawing against the schema and against the scene the canvas drew, and
/// that recording it again gives the same drawing.
fn recorded(harness: &Harness, open: &str) -> Value {
    let path = harness.root.join(open);
    let first = drawing(&vendor(), &path, &harness.root, Kind::Light).unwrap();
    let errors: Vec<String> = drawing_schema()
        .iter_errors(&first)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(
        drawing(&vendor(), &path, &harness.root, Kind::Light).unwrap(),
        first,
        "recording is deterministic"
    );

    // Every element and relation the canvas drew is in the drawing, and nothing else.
    let scene = harness.tab.scene().unwrap();
    let (mut elements, mut relations) = (HashSet::new(), HashSet::new());
    for primitive in &scene.primitives {
        match (primitive, primitive.owner()) {
            (Primitive::Hook(_), _) => {}
            (_, Owner::Element(id)) => {
                elements.insert(id.clone());
            }
            (_, Owner::Relation(id)) => {
                relations.insert(id.clone());
            }
            _ => {}
        }
    }
    let ids = |key: &str| -> HashSet<String> {
        first[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["id"].as_str().unwrap().to_owned())
            .collect()
    };
    assert_eq!(ids("elements"), elements);
    assert_eq!(ids("edges"), relations);
    let order: Vec<&str> = first["order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o.as_str().unwrap())
        .collect();
    assert_eq!(
        order.len(),
        order.iter().collect::<HashSet<_>>().len(),
        "each id once in the order"
    );
    first
}

#[test]
fn the_sankey_scene_records_its_bars_with_their_labels_and_its_bands_with_their_ends() {
    let harness = super::sankey::company();
    let drawn = recorded(&harness, "brightwater-coffee.adp");
    let bars = drawn["elements"].as_array().unwrap();
    let bands = drawn["edges"].as_array().unwrap();
    assert!(
        bars.len() >= 4 && bands.len() >= 3,
        "{} bars, {} bands",
        bars.len(),
        bands.len()
    );
    assert!(
        bars.iter()
            .all(|b| b["type"] == "etalii/sankey+node" && b["bounds"]["h"].as_f64().unwrap() > 0.0)
    );
    assert!(bars.iter().any(|b| b["labels"][0]["text"] == "Cafés"), "{bars:?}");
    let ids: HashSet<&str> = bars.iter().map(|b| b["id"].as_str().unwrap()).collect();
    for band in bands {
        assert!(ids.contains(band["source"].as_str().unwrap()) && ids.contains(band["target"].as_str().unwrap()));
        assert!(band["style"]["fill"].is_string(), "a band is a filled shape: {band}");
        assert!(band["route"].as_array().unwrap().len() >= 2);
    }
}

#[test]
fn the_wardley_scene_records_its_components_and_its_axes_as_chrome() {
    let harness = super::wardley_map::strategy_vocabulary();
    let drawn = recorded(&harness, "strategy-vocabulary.adp");
    let components = drawn["elements"].as_array().unwrap();
    assert!(
        components.iter().any(|c| c["labels"][0]["text"] == "Payment"),
        "{components:?}"
    );
    let chrome = drawn["chrome"].as_array().unwrap();
    assert!(
        chrome.iter().any(|c| c["kind"] == "legend" && c["text"] == "Genesis"),
        "the evolution axis: {chrome:?}"
    );
    assert!(drawn["edges"].as_array().is_some_and(|e| !e.is_empty()));
    let written = serde_json::to_string_pretty(&drawn).unwrap();
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/recordings");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("wardley.drawing.json"), written).unwrap();
}

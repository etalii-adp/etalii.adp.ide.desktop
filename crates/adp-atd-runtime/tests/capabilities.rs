//! The capability registry (etalii.adp spec 035, task T125, research R14): a capability is registered only once its
//! conformance fixtures pass, so every registered capability has fixtures in the vendored catalogue and gives every
//! fixture's output exactly.

use std::fs;
use std::path::{Path, PathBuf};

use adp_atd_runtime::Capabilities;
use serde_json::Value as Json;

fn catalogue() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/specifications/atd/capabilities")
}

fn fixtures(name: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(catalogue().join(name).join("fixtures"))
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "json"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

#[test]
fn every_registered_capability_gives_its_fixtures_outputs() {
    let capabilities = Capabilities::desktop();
    assert!(!capabilities.names().is_empty());
    for name in capabilities.names() {
        let contract: Json = serde_json::from_str(
            &fs::read_to_string(catalogue().join(name).join("contract.json"))
                .expect("the catalogue has the capability"),
        )
        .unwrap();
        assert_eq!(contract["name"], name);
        let files = fixtures(name);
        assert!(!files.is_empty(), "`{name}` is registered without fixtures");
        for file in files {
            let fixture: Json = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            let output = capabilities
                .run(name, &fixture)
                .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            assert_eq!(output, fixture["output"], "{}", file.display());
        }
    }
}

#[test]
fn a_capability_the_host_does_not_provide_is_absent() {
    let capabilities = Capabilities::desktop();
    assert!(capabilities.layout("layout.noSuchThing").is_none());
    let refused = capabilities.run("layout.noSuchThing", &serde_json::json!({ "input": {} }));
    assert_eq!(
        refused.unwrap_err(),
        "This host provides no capability `layout.noSuchThing`."
    );
}

#[test]
fn a_fixed_side_and_a_collapsed_node_are_honoured() {
    let input = serde_json::json!({
        "input": {
            "nodes": [
                { "id": "r", "width": 40, "height": 20, "parent": null },
                { "id": "a", "width": 30, "height": 20, "parent": "r", "order": 0, "attributes": { "side": "left" } },
                { "id": "b", "width": 30, "height": 20, "parent": "r", "order": 1, "attributes": { "collapsed": true } },
                { "id": "c", "width": 30, "height": 20, "parent": "b", "order": 0 }
            ],
            "edges": []
        }
    });
    let output = Capabilities::desktop().run("layout.twoSidedTree", &input).unwrap();
    let positions = output["positions"].as_object().unwrap();
    assert!(positions["a"]["x"].as_f64().unwrap() < 0.0, "a is fixed to the left");
    // b is the first child without a side, so it goes right; c is beneath a collapsed node and takes no room.
    assert!(positions["b"]["x"].as_f64().unwrap() > 0.0);
    assert!(!positions.contains_key("c"));
    assert_eq!(positions["r"], serde_json::json!({ "x": -20.0, "y": -10.0 }));
}

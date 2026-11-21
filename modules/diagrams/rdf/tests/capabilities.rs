//! The capabilities the RDF module contributes to the registry (etalii.adp spec 035, T125, research R14): each is
//! registered only because every fixture of it in the vendored catalogue gives its output exactly.

use std::fs;
use std::path::{Path, PathBuf};

use adp_atd_runtime::Capabilities;
use serde_json::Value as Json;

fn fixtures(name: &str) -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../vendor/etalii.adp/specifications/atd/capabilities")
        .join(name)
        .join("fixtures");
    let mut files: Vec<PathBuf> = fs::read_dir(folder)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    files
}

#[test]
fn every_contributed_capability_gives_its_fixtures_outputs() {
    let capabilities = adp_diagram_rdf::capabilities::contribute(Capabilities::desktop());
    for name in ["reader.turtle", "render.descriptionLogic"] {
        assert!(capabilities.names().contains(&name), "{name} is not contributed");
        let files = fixtures(name);
        assert!(!files.is_empty(), "`{name}` has no fixtures");
        for file in files {
            let fixture: Json = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            let output = capabilities
                .run(name, &fixture)
                .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            assert_eq!(output, fixture["output"], "{}", file.display());
        }
    }
}

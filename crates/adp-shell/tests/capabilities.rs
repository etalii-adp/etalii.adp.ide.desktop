//! The capabilities as the shell assembles them (etalii.adp spec 035, T125, T132, research R14): every one, the
//! registry's own and those the modules contribute, gives every fixture of it in the vendored catalogue exactly.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value as Json;

fn fixtures(name: &str) -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/etalii.adp/specifications/atd/capabilities")
        .join(name)
        .join("fixtures");
    let mut files: Vec<PathBuf> = fs::read_dir(folder)
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
fn every_capability_the_shell_assembles_gives_its_fixtures_outputs() {
    let capabilities = adp_shell::capabilities();
    for name in capabilities.names() {
        let files = fixtures(name);
        assert!(!files.is_empty(), "`{name}` is assembled without fixtures");
        for file in files {
            let fixture: Json = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
            let output = capabilities
                .run(name, &fixture)
                .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            assert_eq!(output, fixture["output"], "{}", file.display());
        }
    }
}

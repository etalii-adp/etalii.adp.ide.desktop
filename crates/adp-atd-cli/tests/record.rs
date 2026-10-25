//! `atd record` (etalii.adp spec 035, T049): recording the mind map's cases from its parity scripts keeps the bytes of
//! `parity/goldens/freeplane/mindmap/` unchanged, draws what the mind map's `insta` scene snapshot holds, writes cases
//! valid against `$defs/ConformanceFixture`, and gives the same files when recorded twice.

#![cfg(feature = "record")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_dir() -> PathBuf {
    repository().join("vendor/etalii.adp/specifications/atd")
}

/// The other specifications' schemas, beside ATD's, and nothing from the network.
struct Vendored;

impl jsonschema::Retrieve for Vendored {
    fn retrieve(&self, uri: &jsonschema::Uri<String>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let name = uri.as_str().rsplit('/').next().unwrap_or("").trim_end_matches('#');
        let folder = spec_dir().parent().unwrap().join(name.trim_end_matches(".schema.json"));
        Ok(serde_json::from_str(&std::fs::read_to_string(folder.join(name))?)?)
    }
}

fn validator(name: &str) -> jsonschema::Validator {
    let mut schema: Value =
        serde_json::from_str(&std::fs::read_to_string(spec_dir().join("atd.schema.json")).unwrap()).unwrap();
    schema["$ref"] = Value::String(format!("#/$defs/{name}"));
    jsonschema::options().with_retriever(Vendored).build(&schema).unwrap()
}

/// Records the mind map's cases into a new folder; the folder.
fn record() -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_atd"))
        .args(["record", "freeplane/mindmap", "--cases"])
        .arg(folder.path())
        .env("ATD_RECORDED_WITH", "0000000")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    folder
}

/// Every file under a folder, by its path in it.
fn files(folder: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(next) = folders.pop() {
        for entry in std::fs::read_dir(&next).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let name = path.strip_prefix(folder).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(name, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn json(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// A number of the snapshot, as a scene keeps it.
fn numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|s| !s.is_empty() && *s != "-")
        .map(|s| s.parse().unwrap())
        .collect()
}

#[test]
fn recording_the_mind_map_keeps_its_golden_bytes_and_draws_its_scene_snapshot() {
    let cases = record();
    let case = cases.path().join("example-1");
    let fixture = json(&case.join("conformance.json"));
    let errors: Vec<String> = validator("ConformanceFixture")
        .iter_errors(&fixture)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{errors:?}");

    // The inputs are the fixture's documents, byte for byte.
    let source = repository().join("parity/fixtures/freeplane/mindmap/example-1");
    for (_, name) in fixture["inputs"].as_object().unwrap() {
        let name = name.as_str().unwrap();
        assert_eq!(
            std::fs::read(case.join(name)).unwrap(),
            std::fs::read(source.join(name)).unwrap(),
            "{name}"
        );
    }
    // The bytes the last step leaves are the standalone host's golden, unchanged.
    let steps = fixture["steps"].as_array().unwrap();
    let last = steps.iter().rev().find_map(|s| s["expectFile"].as_str()).unwrap();
    let golden = repository().join("parity/goldens/freeplane/mindmap/example-1/mindmap.mm");
    assert_eq!(std::fs::read(case.join(last)).unwrap(), std::fs::read(golden).unwrap());
    for step in steps {
        if let Some(name) = step["expectFile"].as_str() {
            assert!(case.join(name).is_file(), "{name}");
        }
        if let Some(name) = step["drawing"]["$ref"].as_str() {
            let errors: Vec<String> = validator("Drawing")
                .iter_errors(&json(&case.join(name)))
                .map(|e| e.to_string())
                .collect();
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
    }
    assert_eq!(fixture["provenance"]["bytes"]["host"], "standalone");
    assert_eq!(fixture["provenance"]["drawing"]["recordedWith"], "0000000");

    // The drawing of the reading is the scene the snapshot holds: each node's body, its stroke inside the box
    // the snapshot measures, and its label where the snapshot sets it; and as many lines as it draws.
    let drawing = json(&case.join("drawing.read.json"));
    let snapshot =
        std::fs::read_to_string(repository().join("crates/adp-disl/tests/scenes/snapshots/scenes__mindmap_drawn.snap"))
            .unwrap();
    let elements: BTreeMap<&str, &Value> = drawing["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["id"].as_str().unwrap(), e))
        .collect();
    let mut bodies = 0;
    for line in snapshot.lines() {
        let Some(rest) = line.strip_prefix("shape Element(\"") else {
            continue;
        };
        let (id, rest) = rest.split_once("\")").unwrap();
        if !rest.starts_with(" Body ") {
            continue;
        }
        bodies += 1;
        let element = elements.get(id).unwrap_or_else(|| panic!("{id} is drawn"));
        let half = element["style"]["strokeWidth"].as_f64().unwrap_or(0.0) / 2.0;
        let bounds = &element["bounds"];
        let drawn = [
            bounds["x"].as_f64().unwrap() - half,
            bounds["y"].as_f64().unwrap() - half,
            bounds["w"].as_f64().unwrap() + 2.0 * half,
            bounds["h"].as_f64().unwrap() + 2.0 * half,
        ];
        let expected = numbers(rest);
        for (d, e) in drawn.iter().zip(&expected) {
            assert!(
                (d - e).abs() < 0.011,
                "{id}: drawn {drawn:?}, the snapshot {expected:?}"
            );
        }
    }
    assert_eq!(bodies, elements.len());
    for line in snapshot.lines().filter(|l| l.starts_with("text Element(\"")) {
        let id = &line["text Element(\"".len()..line.find("\")").unwrap()];
        let text = line.split('"').nth(5).unwrap();
        let at = numbers(line.rsplit_once(" at ").unwrap().1);
        let label = &elements[id]["labels"][0];
        assert_eq!(label["text"], text, "{id}");
        assert_eq!(
            [
                label["position"]["x"].as_f64().unwrap(),
                label["position"]["y"].as_f64().unwrap()
            ],
            [at[0], at[1]],
            "{id}"
        );
    }
    let lines = snapshot.lines().filter(|l| l.starts_with("edge Relation(")).count();
    assert_eq!(drawing["edges"].as_array().unwrap().len(), lines);

    // Recording again gives the same files.
    let again = record();
    assert_eq!(files(cases.path()), files(again.path()));
}

#[test]
fn recording_a_capability_writes_valid_fixtures_from_the_code_that_does_it() {
    let record = || {
        let folder = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_atd"))
            .args(["record", "--capability", "layout.twoSidedTree", "--cases"])
            .arg(folder.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        folder
    };
    let fixtures = record();
    let validator = validator("CapabilityFixture");
    let written = files(fixtures.path());
    assert!(written.contains_key("example-1-read.json") && written.contains_key("example-1-golden.json"));
    for (name, bytes) in &written {
        let fixture: Value = serde_json::from_slice(bytes).unwrap();
        let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        let nodes = fixture["input"]["nodes"].as_array().unwrap();
        let positions = fixture["output"]["positions"].as_object().unwrap();
        assert!(nodes.iter().all(|n| n["width"].as_f64().unwrap() > 0.0));
        // Every node is placed but what a folded branch hides.
        let hidden = nodes.len() - positions.len();
        let folded = nodes.iter().any(|n| n["attributes"]["folded"] == true);
        assert!(hidden == 0 || folded, "{name}");
        assert!(
            nodes
                .iter()
                .filter(|n| n["parent"].is_null())
                .all(|n| positions.contains_key(n["id"].as_str().unwrap()))
        );
    }
    assert_eq!(
        files(record().path()),
        written,
        "recording again gives the same fixtures"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_atd"))
        .args(["record", "--capability", "render.markdown", "--cases", "unused"])
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "a capability without named code is refused"
    );
}

/// Records capability `name`'s fixtures into a new folder; the folder.
fn record_capability(name: &str) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_atd"))
        .args(["record", "--capability", name, "--cases"])
        .arg(folder.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    folder
}

#[test]
fn recording_a_reader_writes_what_each_document_of_its_cases_reads_to() {
    let fixtures = record_capability("reader.databricksBundle");
    let validator = validator("CapabilityFixture");
    let written = files(fixtures.path());
    for expected in [
        "job-nightly-ingest.json",
        "bundle-lakehouse.json",
        "pipeline-settings.json",
    ] {
        assert!(written.contains_key(expected), "{expected} in {:?}", written.keys());
    }
    for (name, bytes) in &written {
        let fixture: Value = serde_json::from_slice(bytes).unwrap();
        let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        assert!(
            !fixture["input"]["body"].as_str().unwrap().is_empty() || name.starts_with("bundle-empty"),
            "{name}"
        );
        assert!(
            fixture["output"]["elements"].is_array() && fixture["output"]["findings"].is_array(),
            "{name}"
        );
    }
    let job: Value = serde_json::from_slice(&written["job-nightly-ingest.json"]).unwrap();
    assert_eq!(job["args"]["reading"], "job");
    assert!(
        job["output"]["elements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == "task:land_raw")
    );
    assert_eq!(files(record_capability("reader.databricksBundle").path()), written);
}

#[test]
fn recording_a_layout_places_every_node_of_each_reading_and_keeps_the_pinned_ones() {
    let validator = validator("CapabilityFixture");
    for (name, expected) in [
        ("layout.layeredColumns", "job-nightly-ingest.json"),
        ("layout.bands", "bundle-lakehouse.json"),
    ] {
        let written = files(record_capability(name).path());
        assert!(written.contains_key(expected), "{name}: {:?}", written.keys());
        for (file, bytes) in &written {
            let fixture: Value = serde_json::from_slice(bytes).unwrap();
            let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "{file}: {errors:?}");
            let positions = fixture["output"]["positions"].as_object().unwrap();
            for node in fixture["input"]["nodes"].as_array().unwrap() {
                let id = node["id"].as_str().unwrap();
                assert!(positions.contains_key(id), "{name} {file}: {id} is placed");
                if let Some(pinned) = node.get("pinned") {
                    assert_eq!(&positions[id], pinned, "{name} {file}: {id} stays where it is pinned");
                }
            }
        }
    }
}

/// Records the Hierarchical Requirement Designer's cases into a new folder (etalii.adp spec 035, T167); the folder.
fn record_requirements() -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_atd"))
        .args(["record", "etalii/hierarchical-requirement", "--cases"])
        .arg(folder.path())
        .env("ATD_RECORDED_WITH", "0000000")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    folder
}

#[test]
fn recording_the_requirement_designer_keeps_its_golden_bytes_and_states_its_page_gestures() {
    let cases = record_requirements();
    let validator = validator("ConformanceFixture");
    let scripts = repository().join("parity/scripts/etalii/hierarchical-requirement");
    let mut recorded = 0;
    for script in std::fs::read_dir(&scripts).unwrap().flatten() {
        let path = script.path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let case = cases.path().join(&name);
        let fixture = json(&case.join("conformance.json"));
        let errors: Vec<String> = validator
            .iter_errors(&fixture)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        recorded += 1;

        // A designer's case states no drawing; its reading and findings are the desktop host's at the commit that
        // records them, its bytes the standalone host's golden.
        assert!(fixture.get("drawing").is_none(), "{name}");
        assert_eq!(fixture["provenance"]["reading"]["commit"], "0000000", "{name}");
        assert_eq!(fixture["provenance"]["bytes"]["host"], "standalone", "{name}");
        assert!(!fixture["read"]["elements"].as_array().unwrap().is_empty(), "{name}");

        // The bytes the last step leaves are the golden's body, unchanged.
        let body = fixture["inputs"]["body"].as_str().unwrap();
        let steps = fixture["steps"].as_array().unwrap();
        if let Some(last) = steps.iter().rev().find_map(|s| s["expectFile"].as_str()) {
            let golden = repository()
                .join("parity/goldens/etalii/hierarchical-requirement")
                .join(&name)
                .join(body);
            if golden.is_file() {
                assert_eq!(
                    std::fs::read(case.join(last)).unwrap(),
                    std::fs::read(golden).unwrap(),
                    "{name}"
                );
            }
        }
        for step in steps {
            if let Some(file) = step["expectFile"].as_str() {
                assert!(case.join(file).is_file(), "{name}: {file}");
            }
        }
    }
    assert!(recorded > 0);

    // A page gesture is the operation on its target; an edit by another writer is `outside`.
    let tags = json(&cases.path().join("tags/conformance.json"));
    assert_eq!(tags["steps"][0]["action"], "addTag");
    assert_eq!(tags["steps"][0]["select"], serde_json::json!(["REQ-0009"]));
    assert!(
        tags["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.get("refused").is_some())
    );
    let writers = json(&cases.path().join("two-writers/conformance.json"));
    assert!(
        writers["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s.get("outside").is_some())
    );

    // Recording again gives the same files.
    assert_eq!(files(cases.path()), files(record_requirements().path()));
}

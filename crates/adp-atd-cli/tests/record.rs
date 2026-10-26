//! `atd record` (etalii.adp spec 035, T049): recording the mind map's cases from its parity scripts keeps the bytes of
//! `parity/goldens/freeplane/mindmap/` unchanged, draws what the mind map's `insta` scene snapshot holds, writes cases
//! valid against `$defs/ConformanceFixture`, and gives the same files when recorded twice. The editors' cases (T066)
//! keep their examples byte for byte and say where each finding stands.

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
        .args(["record", "--capability", "reader.turtle", "--cases", "unused"])
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

/// The fixtures of capability `name`, recorded twice and found the same both times and valid against
/// `$defs/CapabilityFixture`, by file name.
fn fixtures_of(name: &str) -> BTreeMap<String, Value> {
    let written = files(record_capability(name).path());
    assert_eq!(
        files(record_capability(name).path()),
        written,
        "{name}: recording again gives the same fixtures"
    );
    let validator = validator("CapabilityFixture");
    written
        .into_iter()
        .map(|(file, bytes)| {
            let fixture: Value = serde_json::from_slice(&bytes).unwrap();
            let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "{name} {file}: {errors:?}");
            (file, fixture)
        })
        .collect()
}

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap()
}

#[test]
fn recording_row_packing_puts_no_two_items_closer_than_the_gap_on_a_row() {
    let fixtures = fixtures_of("layout.rowPacking");
    for tool in ["timeline-roadmap", "timeline-add", "hypecycle-graph-coal-technologies"] {
        assert!(fixtures.contains_key(&format!("{tool}-read.json")), "{tool}");
    }
    for (file, fixture) in &fixtures {
        let gap = number(&fixture["args"]["gap"]);
        let positions = &fixture["output"]["positions"];
        let nodes = fixture["input"]["nodes"].as_array().unwrap();
        assert_eq!(
            positions.as_object().unwrap().len(),
            nodes.len(),
            "{file}: every item gets a row"
        );
        // Each item's rows, and where it runs across.
        let placed: Vec<(i64, i64, f64, f64)> = nodes
            .iter()
            .map(|node| {
                let at = &positions[node["id"].as_str().unwrap()];
                let left = number(&node["attributes"]["left"]);
                assert_eq!(number(&at["x"]), left, "{file}: only rows change");
                let top = at["y"].as_i64().unwrap();
                (
                    top,
                    top + node["height"].as_i64().unwrap(),
                    left,
                    left + number(&node["width"]),
                )
            })
            .collect();
        for (index, first) in placed.iter().enumerate() {
            for second in &placed[index + 1..] {
                let share_a_row = first.0 < second.1 && second.0 < first.1;
                let clear = first.3 + gap <= second.2 + 0.02 || second.3 + gap <= first.2 + 0.02;
                assert!(!share_a_row || clear, "{file}: {first:?} and {second:?} crowd one row");
            }
        }
        // A pair kept apart never shares a row.
        for edge in fixture["input"]["edges"].as_array().unwrap() {
            if edge["attributes"]["apart"] == true {
                let row = |end: &str| positions[edge[end].as_str().unwrap()]["y"].clone();
                assert_ne!(row("source"), row("target"), "{file}: {edge}");
            }
        }
    }
}

#[test]
fn recording_row_packed_keeps_every_row_and_packs_it_left_to_right() {
    let fixtures = fixtures_of("layout.rowPacked");
    assert!(fixtures.contains_key("hypecycle-graph-coal-technologies-read.json"));
    for (file, fixture) in &fixtures {
        let gap = number(&fixture["args"]["gap"]);
        let step = number(&fixture["args"]["rowStep"]);
        let output = &fixture["output"];
        let mut rows: BTreeMap<i64, Vec<(f64, f64, f64)>> = BTreeMap::new();
        for node in fixture["input"]["nodes"].as_array().unwrap() {
            let id = node["id"].as_str().unwrap();
            let at = &output["positions"][id];
            assert_eq!(at["y"], node["position"]["y"], "{file}: {id} keeps its row");
            let width = output["sizes"][id]["width"]
                .as_f64()
                .unwrap_or_else(|| number(&node["width"]));
            if let Some(packed) = node["attributes"]["packedWidth"].as_f64() {
                assert_eq!(width, packed, "{file}: {id} is drawn at its packed width");
            }
            let (top, height) = (number(&node["position"]["y"]), number(&node["height"]));
            let leading = node["attributes"]["leading"].as_f64().unwrap_or(0.0);
            let (first, last) = (
                (top / step).floor() as i64,
                ((top + height - 1.0) / step).floor() as i64,
            );
            for row in first..=last.max(first) {
                rows.entry(row)
                    .or_default()
                    .push((number(&at["x"]), number(&at["x"]) + width, leading));
            }
        }
        for (row, mut placed) in rows {
            placed.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in placed.windows(2) {
                assert!(
                    pair[0].1 + gap + pair[1].2 <= pair[1].0 + 0.02,
                    "{file}: row {row} crowds {pair:?}"
                );
            }
        }
    }
}

#[test]
fn recording_the_tidy_tree_hangs_every_row_one_layer_below_its_parent() {
    let fixtures = fixtures_of("layout.tidyTree");
    assert!(fixtures.contains_key("agent-behavior-modelling-research-assistant-read.json"));
    for (file, fixture) in &fixtures {
        let layer = number(&fixture["args"]["layerGap"]);
        let gap = number(&fixture["args"]["nodeGap"]);
        let positions = &fixture["output"]["positions"];
        let nodes = fixture["input"]["nodes"].as_array().unwrap();
        assert_eq!(positions.as_object().unwrap().len(), nodes.len(), "{file}");
        let minimum = nodes
            .iter()
            .map(|n| number(&positions[n["id"].as_str().unwrap()]["x"]))
            .fold(f64::INFINITY, f64::min);
        assert_eq!(minimum, 0.0, "{file}: the leftmost node is at zero");
        for node in nodes {
            let at = &positions[node["id"].as_str().unwrap()];
            let Some(parent) = node["parent"].as_str() else {
                assert_eq!(number(&at["y"]), 0.0, "{file}: a root is on the first row");
                continue;
            };
            let above = &positions[parent];
            assert_eq!(
                number(&at["y"]),
                number(&above["y"]) + number(&node["height"]) + layer,
                "{file}"
            );
        }
        // Siblings in their order, left to right, at least the gap apart.
        for node in nodes {
            for other in nodes {
                if node["parent"] == other["parent"] && node["order"].as_i64() < other["order"].as_i64() {
                    let (left, right) = (
                        &positions[node["id"].as_str().unwrap()],
                        &positions[other["id"].as_str().unwrap()],
                    );
                    assert!(
                        number(&left["x"]) + number(&node["width"]) + gap <= number(&right["x"]) + 0.02,
                        "{file}"
                    );
                }
            }
        }
    }
}

#[test]
fn recording_the_self_organising_arrangement_moves_every_element_clear_of_the_others_or_refuses() {
    let fixtures = fixtures_of("arrange.selfOrganising");
    let mut refused = 0;
    for (file, fixture) in &fixtures {
        let elements = fixture["input"]["elements"].as_array().unwrap();
        let separation = number(&fixture["args"]["separation"]);
        if let Some(findings) = fixture["output"]["findings"].as_array() {
            assert!(
                elements.len() > fixture["args"]["budget"].as_u64().unwrap() as usize,
                "{file}"
            );
            assert!(findings[0]["message"].as_str().unwrap().contains("more than"), "{file}");
            refused += 1;
            continue;
        }
        let actions = fixture["output"]["actions"].as_array().unwrap();
        assert_eq!(actions.len(), elements.len(), "{file}: every element is moved");
        let boxes: Vec<(f64, f64, f64, f64)> = elements
            .iter()
            .zip(actions)
            .map(|(element, action)| {
                assert_eq!(action["kind"], "move");
                assert_eq!(action["target"], element["id"], "{file}: in the elements' order");
                (
                    number(&action["x"]),
                    number(&action["y"]),
                    number(&element["width"]),
                    number(&element["height"]),
                )
            })
            .collect();
        for (index, a) in boxes.iter().enumerate() {
            for b in &boxes[index + 1..] {
                let apart_x = (a.0 - b.0).abs() >= (a.2 + b.2) / 2.0 + separation - 0.05;
                let apart_y = (a.1 - b.1).abs() >= (a.3 + b.3) / 2.0 + separation - 0.05;
                assert!(apart_x || apart_y, "{file}: {a:?} and {b:?} overlap");
            }
        }
    }
    assert_eq!(refused, 1, "one fixture records the refusal past the budget");
}

#[test]
fn recording_the_forward_bezier_leaves_each_dependent_rightward_and_enters_each_dependency_from_its_left() {
    let fixtures = fixtures_of("routing.forwardBezier");
    let mut loops = 0;
    for (file, fixture) in &fixtures {
        let routes = &fixture["output"]["routes"];
        for edge in fixture["input"]["edges"].as_array().unwrap() {
            let (source, target) = (&edge["source"], &edge["target"]);
            let route = routes[edge["id"].as_str().unwrap()].as_array().unwrap();
            assert_eq!(route.len(), 2, "{file}: a move and one curve");
            let (start, curve) = (&route[0]["points"][0], route[1]["points"].as_array().unwrap());
            assert_eq!(route[1]["op"], "cubic");
            let middle = |b: &Value| number(&b["y"]) + number(&b["height"]) / 2.0;
            let near = |a: f64, b: f64| (a - b).abs() < 0.011;
            assert!(
                near(number(&start["x"]), number(&source["x"]) + number(&source["width"])),
                "{file}"
            );
            assert!(near(number(&start["y"]), middle(source)), "{file}");
            assert!(near(number(&curve[2]["x"]), number(&target["x"])), "{file}");
            assert!(near(number(&curve[2]["y"]), middle(target)), "{file}");
            if number(&target["x"]) < number(&source["x"]) + number(&source["width"]) {
                assert!(
                    number(&curve[0]["x"]) > number(&start["x"]),
                    "{file}: it leaves rightward"
                );
                assert!(
                    number(&curve[1]["x"]) < number(&curve[2]["x"]),
                    "{file}: it arrives from the left"
                );
                loops += 1;
            }
        }
    }
    assert!(loops > 0, "a dependency sitting behind its dependent loops round");
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

/// Records the cases of editor `origin` into a new folder (etalii.adp spec 035, T066); the folder.
fn record_editor(origin: &str) -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_atd"))
        .args(["record", origin, "--cases"])
        .arg(folder.path())
        .env("ATD_RECORDED_WITH", "0000000")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    folder
}

#[test]
fn recording_an_editor_keeps_its_examples_and_places_its_findings() {
    let examples = repository().join("crates/adp-conformance/tests/round_trip/editors");
    let validator = validator("ConformanceFixture");
    for (origin, folder, case) in [
        ("editor/plain", "plain", "crlf-notes-txt"),
        ("editor/markdown", "markdown", "guide-md"),
        ("editor/json", "json", "service-broken-json"),
        ("editor/xml", "xml", "library-broken-xml"),
        ("editor/yaml", "yaml", "service-broken-yaml"),
    ] {
        let cases = record_editor(origin);
        let written = files(cases.path());
        let fixture = json(&cases.path().join(case).join("conformance.json"));
        let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{origin} {case}: {errors:?}");
        assert_eq!(fixture["tool"], origin);
        // Every file of the examples is an input, byte for byte, so that a schema a document names is there.
        let source = files(&examples.join(folder).join("examples"));
        for (name, bytes) in &source {
            assert_eq!(
                written.get(&format!("{case}/{name}")),
                Some(bytes),
                "{origin} {case}: {name}"
            );
        }
        assert_eq!(
            files(record_editor(origin).path()),
            written,
            "{origin}: recording again gives the same cases"
        );
    }

    // The Markdown editor reads the headings outside fenced code; the `#` line inside the fence is not one.
    let markdown = record_editor("editor/markdown");
    let guide = json(&markdown.path().join("guide-md/conformance.json"));
    let lines: Vec<u64> = guide["read"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["attributes"]["line"].as_u64().unwrap())
        .collect();
    assert_eq!(lines, [1, 6, 10, 14, 21]);

    // A finding stands at its line and column, from 1, columns in characters: the empty name of service-broken.json
    // is on line 3 after two spaces.
    let json_cases = record_editor("editor/json");
    let broken = json(&json_cases.path().join("service-broken-json/conformance.json"));
    let first = &broken["read"]["findings"][0];
    assert_eq!(
        (first["line"].as_u64(), first["column"].as_u64()),
        (Some(3), Some(3)),
        "{first}"
    );
    assert_eq!(first["severity"], "error");
}

#[test]
fn recording_the_markdown_capabilities_writes_valid_fixtures() {
    let validator = validator("CapabilityFixture");
    for name in ["reader.markdown", "render.markdown"] {
        let folder = tempfile::tempdir().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_atd"))
            .args(["record", "--capability", name, "--cases"])
            .arg(folder.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let written = files(folder.path());
        assert!(written.contains_key("guide-md.json"), "{name}: {:?}", written.keys());
        for (file, bytes) in &written {
            let fixture: Value = serde_json::from_slice(bytes).unwrap();
            let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
            assert!(errors.is_empty(), "{name} {file}: {errors:?}");
        }
        let guide: Value = serde_json::from_slice(&written["guide-md.json"]).unwrap();
        if name == "render.markdown" {
            let runs = guide["output"]["runs"].as_array().unwrap();
            assert!(
                runs.iter()
                    .any(|r| r["block"] == "code" && r["style"]["language"] == "bash")
            );
            assert!(runs.iter().any(|r| r["style"]["strong"] == true));
        } else {
            assert_eq!(guide["output"]["elements"].as_array().unwrap().len(), 5);
        }
    }
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

#[test]
fn recording_the_structurizr_reader_names_the_kind_of_view_each_case_reads() {
    let written = files(record_capability("reader.structurizr").path());
    let validator = validator("CapabilityFixture");
    for (name, view) in [
        ("container-courier.json", "container"),
        ("context-operations.json", "systemContext"),
        ("system-landscape-operations.json", "systemLandscape"),
    ] {
        let fixture: Value = serde_json::from_slice(&written[name]).unwrap();
        let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{name}: {errors:?}");
        assert_eq!(fixture["args"]["view"], view, "{name}");
        assert!(
            fixture["output"]["elements"].as_array().is_some_and(|e| !e.is_empty()),
            "{name}"
        );
    }
}

#[test]
fn recording_ranked_boundaries_keeps_the_pins_a_view_does_not_ignore_and_encloses_each_boundary_s_nodes() {
    let written = files(record_capability("layout.rankedBoundaries").path());
    let validator = validator("CapabilityFixture");
    assert!(
        written.contains_key("deployment-operations.json"),
        "{:?}",
        written.keys()
    );
    let mut pins_kept = 0;
    for (file, bytes) in &written {
        let fixture: Value = serde_json::from_slice(bytes).unwrap();
        let errors: Vec<String> = validator.iter_errors(&fixture).map(|e| e.to_string()).collect();
        assert!(errors.is_empty(), "{file}: {errors:?}");
        let positions = fixture["output"]["positions"].as_object().unwrap();
        let ignored = fixture["args"]["pinsIgnored"] == "true";
        let padding = fixture["args"]["padding"].as_f64().unwrap();
        let nodes = fixture["input"]["nodes"].as_array().unwrap();
        for node in nodes {
            let id = node["id"].as_str().unwrap();
            assert!(positions.contains_key(id), "{file}: {id} is placed");
            let encloses = nodes.iter().any(|n| n["parent"] == node["id"]);
            if let Some(pinned) = node.get("pinned")
                && !ignored
                && node.get("parent").is_none()
                && !encloses
            {
                assert_eq!(&positions[id], pinned, "{file}: {id} stays where it is pinned");
                pins_kept += 1;
            }
            if let Some(parent) = node["parent"].as_str() {
                let (inner, outer) = (&positions[id], &positions[parent]);
                for axis in ["x", "y"] {
                    assert!(
                        inner[axis].as_f64().unwrap() >= outer[axis].as_f64().unwrap() + padding - 0.005,
                        "{file}: {id} is inside {parent}"
                    );
                }
            }
        }
    }
    assert!(pins_kept > 0, "some view keeps its stored positions");
}

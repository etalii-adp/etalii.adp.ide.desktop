//! A diagram's description as the tool `adp-diagram` opens documents with (etalii.adp spec 035, tasks T120 and T126):
//! its binding lowered onto FBL, its documents opened from memory, read as the case records the module read them,
//! and laid out by the capability it names, with no code for the tool type.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::capabilities::two_sided_tree::round2;
use adp_atd_runtime::{RuntimeHost, diagram_tool};
use adp_diagram::{DiagramDocuments, ElementKind};
use serde_json::{Value as Json, json};

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn read_json(path: &Path) -> Json {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// A specification without its documentation.
fn undocumented(json: &Json) -> Json {
    match json {
        Json::Object(map) => Json::Object(
            map.iter()
                .filter(|(k, _)| *k != "doc")
                .map(|(k, v)| (k.clone(), undocumented(v)))
                .collect(),
        ),
        Json::Array(items) => Json::Array(items.iter().map(undocumented).collect()),
        other => other.clone(),
    }
}

fn differences(lowered: &Json, written: &Json, path: &str, out: &mut BTreeSet<String>) {
    match (lowered, written) {
        (Json::Object(a), Json::Object(b)) => {
            for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
                let at = format!("{path}/{key}");
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => differences(x, y, &at, out),
                    (Some(_), None) => {
                        out.insert(format!("+ {at}"));
                    }
                    _ => {
                        out.insert(format!("- {at}"));
                    }
                }
            }
        }
        (Json::Array(a), Json::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                differences(x, y, &format!("{path}/{i}"), out);
            }
        }
        (a, b) if a != b => {
            out.insert(format!("~ {path}"));
        }
        _ => {}
    }
}

#[test]
fn the_mind_maps_binding_is_fbls_example_but_for_what_the_description_leaves_out() {
    let checked = check(&FsFiles, &vendor().join("definitions/diagrams/mindmap.atd"));
    let members: Vec<&Json> = checked.description.members().collect();
    let lowered = adp_atd_runtime::lowering::fbl::lower(&members).expect("the mind map has a binding");
    let written = read_json(&vendor().join("specifications/fbl/mindmap.fbl"));
    let (lowered, written) = (
        undocumented(&lowered["bindings"]["freeplane"]),
        undocumented(&written["bindings"]["freeplane"]),
    );
    let keys = |b: &Json| b.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(&lowered), keys(&written), "FBL's order");
    let mut found = BTreeSet::new();
    differences(&lowered, &written, "", &mut found);
    let expected: BTreeSet<String> = [
        // What a new note is written as, which the description states in prose only.
        "- /elements/0/attributes/notes/create",
        // The reason a read-only side gives, worded otherwise.
        "~ /elements/0/attributes/position/readOnly",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(found, expected);
}

#[test]
fn a_case_opens_from_memory_and_reads_as_the_module_read_it() {
    let checked = check(&FsFiles, &vendor().join("definitions/diagrams/mindmap.atd"));
    let symbols = checked.symbols.as_ref().expect("symbols");
    let runtime = diagram_tool(&checked.description, symbols, &RuntimeHost::default()).unwrap();
    assert_eq!(runtime.tool.origin, "freeplane/mindmap");
    assert_eq!(runtime.tool.extensions, [".mm"]);

    let case = vendor().join("definitions/diagrams/conformance/mindmap/example-1");
    let fixture = read_json(&case.join("conformance.json"));
    let body = std::fs::read(case.join("mindmap.mm")).unwrap();
    let registration = std::fs::read(case.join("mindmap.adp")).unwrap();
    // The paths name the files; nothing is read from them.
    let mut documents = DiagramDocuments::default();
    let document = documents.open_bytes(
        runtime.tool.clone(),
        Path::new("/nowhere/mindmap.mm"),
        Some(Path::new("/nowhere/mindmap.adp")),
        body.clone(),
        Some(registration),
    );
    assert!(!document.fbl().unreadable, "{:?}", document.fbl().findings);
    assert_eq!(document.body_bytes(), &body[..]);

    // The nodes, their parents and attributes as the case's reading has them.
    let nodes: Vec<Json> = document
        .elements()
        .iter()
        .filter(|e| e.kind == ElementKind::Node)
        .map(|e| {
            let attributes: serde_json::Map<String, Json> = e
                .attributes
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| (k.clone(), json!(v)))
                .collect();
            json!({"id": e.id, "type": e.element_type, "parent": e.parent, "attributes": attributes})
        })
        .collect();
    // The module read a node without `FOLDED` as not folded; the description's binding states no value for an absent
    // `FOLDED` (FBL's `absent`), so it reads none. That is the one difference, which validation reports.
    let mut expected = fixture["read"]["elements"].clone();
    for node in expected.as_array_mut().unwrap() {
        let attributes = node["attributes"].as_object_mut().unwrap();
        if attributes.get("folded") == Some(&json!("false")) {
            attributes.shift_remove("folded");
        }
    }
    assert_eq!(Json::Array(nodes), expected);

    // The layout is the capability's: the boxes its fixture of this map places, as centres.
    let positions = document.laid_out().expect("the description names a layout capability");
    let recorded =
        read_json(&vendor().join("specifications/atd/capabilities/layout.twoSidedTree/fixtures/example-1-read.json"));
    let sizes: std::collections::HashMap<String, (f64, f64)> = recorded["input"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["id"].as_str().unwrap().to_owned(),
                (n["width"].as_f64().unwrap(), n["height"].as_f64().unwrap()),
            )
        })
        .collect();
    let expected = recorded["output"]["positions"].as_object().unwrap();
    assert_eq!(positions.len(), expected.len());
    for (id, at) in expected {
        let (width, height) = sizes[id];
        let (x, y) = positions[id];
        assert_eq!(
            (round2(x - width / 2.0), round2(y - height / 2.0)),
            (at["x"].as_f64().unwrap(), at["y"].as_f64().unwrap()),
            "{id}"
        );
    }
}

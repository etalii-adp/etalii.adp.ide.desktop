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

/// The etalii.adp the C4 descriptions are read from: the one named by `ETALII_ADP`, else the vendored one.
fn etalii() -> PathBuf {
    std::env::var_os("ETALII_ADP").map_or_else(vendor, PathBuf::from)
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
        // What an absent FOLDED reads as, which the cases state and FBL's example leaves unread.
        "+ /elements/0/attributes/folded/default",
        // The reason a read-only side gives, worded otherwise.
        "~ /elements/0/attributes/position/readOnly",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(found, expected);
}

/// One binding reads the six C4 tool types (research R6): the lowered binding part of each is `structurizr.fbl`, its
/// grammar a copy of `structurizr.lark`, but for what the cases, recorded from the module, ask of it. `ETALII_ADP` runs it on
/// another clone.
#[test]
fn the_six_c4_bindings_are_structurizr_fbl_but_for_what_the_cases_ask() {
    let root = etalii();
    let written = read_json(&root.join("specifications/fbl/structurizr.fbl"));
    let written = undocumented(&written["bindings"]["workspace"]);
    let keys = |b: &Json| b.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    let expected: BTreeSet<String> = C4_DIFFERENCES.iter().map(|d| (*d).to_owned()).collect();
    assert_eq!(
        std::fs::read_to_string(root.join("definitions/diagrams/structurizr.lark")).unwrap(),
        std::fs::read_to_string(root.join("specifications/fbl/structurizr.lark")).unwrap(),
        "the grammar beside the descriptions is FBL's example's"
    );
    let mut first = None;
    for origin in [
        "context",
        "container",
        "component",
        "deployment",
        "dynamic",
        "system-landscape",
    ] {
        let checked = check(&FsFiles, &root.join(format!("definitions/diagrams/c4-{origin}.atd")));
        assert!(
            !checked.description.members().any(|m| m["kind"] == "requires"
                && m["capability"]["text"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("reader."))),
            "{origin} requires no reader"
        );
        let members: Vec<&Json> = checked.description.members().collect();
        let lowered = adp_atd_runtime::lowering::fbl::lower(&members).expect("a binding");
        let mut lowered = undocumented(&lowered["bindings"]["structurizr"]);
        assert_eq!(keys(&lowered), keys(&written), "FBL's order");
        // The rules FBL's example leaves unbound come after every rule it has.
        let elements = lowered["elements"].as_array_mut().unwrap();
        assert_eq!(elements.pop().unwrap()["name"], "includedFile");
        // So does the rule that reads the kind of the selected view, which the module goes by.
        assert_eq!(elements.pop().unwrap()["name"], "viewKind");
        let mut found = BTreeSet::new();
        differences(&lowered, &written, "", &mut found);
        assert_eq!(found, expected, "{origin}");
        match &first {
            None => first = Some(lowered),
            Some(first) => assert_eq!(&lowered, first, "{origin} reads as the first does"),
        }
    }
}

/// One binding reads Turtle for the four W3C tool types (spec 067, T037, T039): the lowered binding part of each is the
/// binding `triples` of `w3c-turtle-grammar.fbl`, its grammar a copy of `w3c-turtle.lark`, but for its origins and its
/// label, and none requires a reader. `ETALII_ADP` runs it on another clone.
#[test]
fn the_four_turtle_bindings_are_fbls_triples() {
    let root = etalii();
    let written = read_json(&root.join("specifications/fbl/w3c-turtle-grammar.fbl"));
    let written = undocumented(&written["bindings"]["triples"]);
    assert_eq!(
        std::fs::read_to_string(root.join("definitions/diagrams/w3c-turtle.lark")).unwrap(),
        std::fs::read_to_string(root.join("specifications/fbl/w3c-turtle.lark")).unwrap(),
        "the grammar beside the descriptions is FBL's example's"
    );
    for origin in ["rdf", "owl", "shacl", "skos"] {
        let checked = check(&FsFiles, &root.join(format!("definitions/diagrams/w3c-{origin}.atd")));
        assert!(
            !checked.description.members().any(|m| m["kind"] == "requires"
                && m["capability"]["text"]
                    .as_str()
                    .is_some_and(|c| c.starts_with("reader."))),
            "{origin} requires no reader"
        );
        let members: Vec<&Json> = checked.description.members().collect();
        let lowered = adp_atd_runtime::lowering::fbl::lower(&members).expect("a binding");
        let lowered = undocumented(&lowered["bindings"]["turtle"]);
        for part in ["elements", "text"] {
            assert_eq!(lowered[part], written[part], "{origin}'s {part}");
        }
        for part in ["start", "order", "prefixes", "ordinals"] {
            assert_eq!(lowered["body"][part], written["body"][part], "{origin}'s body's {part}");
        }
    }
}

/// How the C4 binding differs from `structurizr.fbl`, by what the cases, recorded from the module, ask of it.
const C4_DIFFERENCES: &[&str] = &[
    // The key of `natural` ids, which the model holds and a nested relationship's source names.
    "+ /elements/1/attributes/identifier",
    "+ /elements/2/attributes/identifier",
    "+ /elements/3/attributes/identifier",
    "+ /elements/4/attributes/identifier",
    "+ /elements/5/attributes/identifier",
    "+ /elements/6/attributes/identifier",
    "+ /elements/7/attributes/identifier",
    "+ /elements/8/attributes/identifier",
    // A relationship and an interaction read their ends as written, which their ids and sentences state (gap 14).
    "+ /elements/9/attributes/sourceWritten",
    "+ /elements/9/attributes/targetWritten",
    "+ /elements/16/attributes/sourceWritten",
    "+ /elements/16/attributes/targetWritten",
    // A relationship written inside an element's block has that element as its source, as FBL's example says in words.
    "+ /elements/9/attributes/from/secondary",
    // An element's parent is the element whose block it is in, of any kind, and `misplacedElement` says so.
    "~ /elements/3/parent/rules",
    "~ /elements/4/parent/rules",
    "~ /elements/5/parent/rules",
    "~ /elements/6/parent/rules",
    "~ /elements/7/parent/rules",
    "~ /elements/8/parent/rules",
];

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
    let expected = fixture["read"]["elements"].clone();
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

/// A layout capability's query arguments are evaluated over the diagram, its edges are the stored relations, and a
/// position the registration authors is a pin (`layout.rankedBoundaries`).
#[test]
fn a_ranked_layout_reads_its_arguments_its_edges_and_its_pins_from_the_document() {
    let text = r#"atd 0.1

diagram example/ranks version "0.1.0" {
  label "Ranks"
  id: "example.ranks"
  defaultLocale: "en"
  requires capability layout.rankedBoundaries@^1

  model {
    document {
      attribute flow: string readonly default "lr"
    }
    element Item {
      attribute name: string default "" { key: true }
      labelAttribute: "name"
    }
    relation Next from Item to Item {
      directed: true
    }
  }

  notation {
    node Item {
      shape rect
      label @name as name
      size: { min: [100, 40] }
    }
  }

  layout {
    default ranks
    algorithm ranks capability layout.rankedBoundaries {
      direction: (diagram.flow)
      rankSeparation: (10 + 20)
    }
  }

  persistence {
    format: fbl
    ids: { strategy: natural, stable: true }
  }

  binding items family lines {
    claims { extensions: [".txt"] }
    element Item as item line "^item (?<name>.+)$" {
      attribute name from group "name"
    }
    relation Next as next line "^(?<from>.+) -> (?<to>.+)$" {
      source from group "from"
      target from group "to"
    }
  }
}
"#;
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("ranks.atd");
    std::fs::write(&path, text).unwrap();
    let checked = check(&FsFiles, &path);
    let errors: Vec<String> = checked.findings.iter().map(adp_atd_check::Finding::to_line).collect();
    assert!(
        checked
            .findings
            .iter()
            .all(|f| f.severity != adp_atd_check::Severity::Error),
        "{errors:?}"
    );
    let symbols = checked.symbols.as_ref().expect("symbols");
    let runtime = diagram_tool(&checked.description, symbols, &RuntimeHost::default()).unwrap();
    assert!(runtime.findings.is_empty(), "{:?}", runtime.findings);
    let mut documents = DiagramDocuments::default();
    let document = documents.open_bytes(
        runtime.tool.clone(),
        Path::new("/nowhere/ranks.txt"),
        Some(Path::new("/nowhere/ranks.adp")),
        b"item a\nitem b\nitem c\nb -> a\n".to_vec(),
        Some(b"example/ranks\nbody: ranks.txt\nlayout:\n  c: 500 600\n".to_vec()),
    );
    assert!(!document.fbl().unreadable, "{:?}", document.fbl().findings);
    let positions = document.laid_out().expect("the description names a layout capability");
    // Centres: `b` ranks before `a` along the edge, left to right, 30 apart.
    assert_eq!(positions.len(), 3, "{positions:?} of {:?}", document.fbl().elements);
    let (ax, ay) = positions["a"];
    let (bx, by) = positions["b"];
    assert_eq!(
        ax - bx,
        130.0,
        "a rank to the right of b, by its width and the separation"
    );
    assert_eq!((ay, by), (0.0, -50.0), "a alone on its rank, b beside c on its own");
    assert_eq!(
        positions["c"],
        (550.0, 620.0),
        "the corner the registration pins, as a centre"
    );
}

#[test]
fn a_node_on_an_axis_with_a_scale_is_placed_in_its_canvas_units() {
    let checked = check(&FsFiles, &vendor().join("definitions/diagrams/dependency-graph.atd"));
    let symbols = checked.symbols.as_ref().expect("symbols");
    let runtime = diagram_tool(&checked.description, symbols, &RuntimeHost::default()).unwrap();
    let case = vendor().join("definitions/diagrams/conformance/dependency-graph/services");
    let mut documents = DiagramDocuments::default();
    let document = documents.open_bytes(
        runtime.tool.clone(),
        Path::new("/nowhere/services.dgr"),
        None,
        std::fs::read(case.join("services.dgr")).unwrap(),
        None,
    );
    // Row 4 of an axis of 60 canvas units a row; `x`, on an axis without a scale, as it is written.
    let (x, y) = document.positions()["api-gateway"];
    assert_eq!((round2(x), y), (270.43, 240.0));

    let element = document.element("api-gateway").expect("the node");
    let module = runtime.tool.module.as_ref().expect("the description's module");
    let placed = module.placement(&element, 100.5, 305.0).unwrap().unwrap();
    assert_eq!(placed.attributes.get("x"), Some(&adp_cel::Value::Double(100.5)));
    assert_eq!(
        placed.attributes.get("row"),
        Some(&adp_cel::Value::Int(5)),
        "the nearest row"
    );
}

/// The Wardley map's binding part is `wardley-map.fbl` in ATD, its grammar a copy of `wardley-map.lark`, but for the form
/// of a pipeline, which the model reads and FBL's example leaves unread (spec 067, T032). `ETALII_ADP` runs it on another
/// clone.
#[test]
fn the_wardley_binding_is_fbls_example_but_for_a_pipelines_form() {
    let root = etalii();
    assert_eq!(
        std::fs::read_to_string(root.join("definitions/diagrams/wardley-map.lark")).unwrap(),
        std::fs::read_to_string(root.join("specifications/fbl/wardley-map.lark")).unwrap(),
        "the grammar beside the description is FBL's example's"
    );
    let checked = check(&FsFiles, &root.join("definitions/diagrams/wardley-map.atd"));
    let members: Vec<&Json> = checked.description.members().collect();
    let lowered = adp_atd_runtime::lowering::fbl::lower(&members).expect("a binding");
    let written = read_json(&root.join("specifications/fbl/wardley-map.fbl"));
    let mut found = BTreeSet::new();
    differences(
        &undocumented(&lowered["bindings"]["owm"]),
        &undocumented(&written["bindings"]["map"]),
        "",
        &mut found,
    );
    let expected: BTreeSet<String> = ["+ /elements/8/attributes/form", "+ /elements/9/attributes/form"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(found, expected);
}

/// Each Databricks binding part holds its reading's rules of `databricks.fbl`, where they read what FBL's example reads
/// (spec 067, T030); what the cases ask beyond them is said in the parts' own documentation.
#[test]
fn the_databricks_bindings_hold_their_readings_rules_of_fbls_example() {
    let root = etalii();
    let written = read_json(&root.join("specifications/fbl/databricks.fbl"));
    let written = &written["bindings"]["configuration"];
    for reading in ["bundle", "job", "pipeline"] {
        let checked = check(
            &FsFiles,
            &root.join(format!("definitions/diagrams/databricks-{reading}.atd")),
        );
        let members: Vec<&Json> = checked.description.members().collect();
        let lowered = adp_atd_runtime::lowering::fbl::lower(&members).expect("a binding");
        let lowered = &lowered["bindings"]["bundle"];
        assert_eq!(lowered["reader"], "declared", "{reading} needs no reader");
        let rules_of = |binding: &Json| -> Vec<Json> {
            ["elements", "relations"]
                .iter()
                .flat_map(|list| binding[*list].as_array().cloned().unwrap_or_default())
                .collect()
        };
        let own = rules_of(lowered);
        let origin = format!("databricks/{reading}");
        for rule in rules_of(written).iter().filter(|rule| {
            rule["origins"]
                .as_array()
                .is_some_and(|o| o.iter().any(|x| *x == origin.as_str()))
        }) {
            // The job's dependency is read by a rule of its own and added by the relation rule of FBL's name's kind.
            let found = own
                .iter()
                .find(|r| r["name"] == rule["name"])
                .unwrap_or_else(|| panic!("{reading} has {}", rule["name"]));
            assert_eq!(found["at"], rule["at"], "{reading} {}", rule["name"]);
            assert_eq!(found["origins"], rule["origins"], "{reading} {}", rule["name"]);
        }
    }
}

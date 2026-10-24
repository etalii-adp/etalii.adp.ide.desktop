//! Lowering a diagram's description onto DISL (etalii.adp spec 035, task T126, research R16): the mind map's
//! description is the specification `mindmap.dis` is, but for what the description states more completely, and its
//! operations run through DISL's actions with the script's functions; an operation whose statements are the script's
//! runs as an `extension` action in DISL's transaction.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{Files, FsFiles, Severity, check};
use adp_atd_runtime::{LoadedDiagram, load_diagram, lower_diagram};
use adp_cel::{Map, Value};
use adp_disl::behavior::{ids, operations};
use adp_disl::{DislChange, DislDiagram, DislPluginFunctions, DislTransaction, HostAction};
use serde_json::Value as Json;

fn definitions() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

fn load(files: &dyn Files, path: &Path) -> LoadedDiagram {
    let checked = check(files, path);
    let errors: Vec<String> = checked
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.to_line())
        .collect();
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    let symbols = checked.symbols.expect("symbols");
    match load_diagram(&checked.description, &symbols, &DislPluginFunctions::none()) {
        Ok(loaded) => loaded,
        Err(diagnostics) => panic!(
            "{}",
            diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

fn mindmap() -> LoadedDiagram {
    load(&FsFiles, &definitions().join("mindmap.atd"))
}

/// A specification without its documentation, an expression in either of DISL's forms written as its CEL, and
/// CEL's two string quotes made one.
fn normal(json: &Json) -> Json {
    match json {
        Json::Object(map) if map.len() == 1 && map.get("cel").is_some_and(Json::is_string) => normal(&map["cel"]),
        Json::Object(map) => Json::Object(
            map.iter()
                .filter(|(k, _)| *k != "doc")
                .map(|(k, v)| (k.clone(), normal(v)))
                .collect(),
        ),
        Json::Array(items) => Json::Array(items.iter().map(normal).collect()),
        Json::String(s) => Json::String(s.replace('"', "'")),
        other => other.clone(),
    }
}

/// Every path at which two specifications differ.
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
fn the_mind_maps_description_is_its_dis_but_for_what_it_states_more_completely() {
    let lowered = lower_diagram(&check(&FsFiles, &definitions().join("mindmap.atd")).description).unwrap();
    let written: Json =
        serde_json::from_str(&std::fs::read_to_string(definitions().join("mindmap.dis")).unwrap()).unwrap();
    let mut found = BTreeSet::new();
    differences(&normal(&lowered.specification), &normal(&written), "", &mut found);
    let expected: BTreeSet<String> = [
        // The new sibling's place, which the module placed in code.
        "+ /behavior/operations/addSibling/actions/0/create/after",
        // One type in a list, as the description writes it.
        "~ /constraints/rules/2/scope",
        // The form's sections by name.
        "+ /forms/nodeProperties/items/3/id",
        "+ /forms/nodeProperties/items/4/id",
        // The functions are the script's: `siblingName` in full rather than the module's stub.
        "- /functions",
        // The layout is the capability, with its arguments.
        "~ /layout/algorithms/mindmap/algorithm",
        "- /layout/algorithms/mindmap/options",
        "- /layout/algorithms/mindmap/spacing",
        "+ /layout/algorithms/mindmap/x-arguments",
        // A node's size from the declared text metric, as the module measured it.
        "- /notation/nodes/Node/size/autoSize",
        "+ /notation/nodes/Node/size/height",
        "+ /notation/nodes/Node/size/width",
        "+ /notation/textMetric",
        // The binding is the description's own, lowered onto FBL apart from DISL.
        "- /persistence/binding",
        // The tool drops on the node it adds a child to.
        "+ /toolbox/groups/0/tools/0/mode",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(found, expected);
    assert!(
        lowered.scripts.is_empty(),
        "every operation of the mind map is DISL's own actions"
    );
}

/// A central topic with children named `names`.
fn map_with(loaded: &LoadedDiagram, names: &[&str]) -> DislDiagram {
    let diagram = DislDiagram::new(&loaded.specification);
    let text = |t: &str| Map::from_iter([("text", Value::String(t.into()))]);
    let root = diagram.add_node("Node", "root", text("Plan")).unwrap();
    for (i, name) in names.iter().enumerate() {
        diagram
            .add_child("Node", &format!("c{i}"), text(name), &root, None)
            .unwrap();
    }
    diagram
}

fn created(transaction: &DislTransaction) -> (String, Option<i64>) {
    assert!(transaction.refusal.is_none(), "{:?}", transaction.refusal);
    match &transaction.changes[0] {
        DislChange::Create { attributes, index, .. } => (attributes.get("text").unwrap().to_string(), *index),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_mind_maps_operations_run_through_disl_with_the_scripts_functions() {
    let loaded = mindmap();
    let diagram = map_with(&loaded, &["Phase 1", "Phase 3", "Idea"]);
    let root = diagram.element_by_id("root").unwrap();
    let first = diagram.element_by_id("c0").unwrap();
    let run = |name: &str, on| {
        operations::run(
            &loaded.specification,
            name,
            &diagram,
            Some(on),
            &ids::fixed(&["n1", "n2"]),
            None,
            None,
        )
    };

    // `siblingName` is the script's: the stem most siblings share, from one past its highest number.
    let child = run("addChild", &root);
    assert_eq!(created(&child), ("Phase 4".to_owned(), None));
    assert!(matches!(
        &child.host_actions[..],
        [HostAction::Select { .. }, HostAction::EditLabel { .. }]
    ));

    // A sibling goes right after the node it is added beside.
    let sibling = run("addSibling", &first);
    assert_eq!(created(&sibling), ("Phase 5".to_owned(), Some(1)));
    let order: Vec<String> = root.children().iter().map(|c| c.value_of("text").to_string()).collect();
    assert_eq!(order, ["Phase 1", "Phase 5", "Phase 3", "Idea", "Phase 4"]);

    // The central topic has no sibling.
    assert!(run("addSibling", &root).refusal.is_some());
}

/// Files held in memory.
struct Memory(BTreeMap<PathBuf, String>);

impl Files for Memory {
    fn read(&self, path: &Path) -> Option<String> {
        self.0.get(path).cloned()
    }

    fn list(&self, folder: &Path) -> Vec<PathBuf> {
        self.0.keys().filter(|p| p.parent() == Some(folder)).cloned().collect()
    }
}

#[test]
fn an_operation_of_the_script_runs_as_an_extension_in_disls_transaction() {
    let text = r#"atd 0.1
diagram test/numbered version "0.1.0" {
  label "Numbered"
  id: "org.example.numbered"
  model {
    element Item {
      attribute name: string
      contains Item ordered
    }
  }

  /// Adds `count` children numbered from one, and refuses more than ten.
  operation number "Number" for Item {
    param count: int
    var i = 1
    while i <= p.count limit 20 {
      if i > 10 {
        fail "At most ten."
      }
      create Item { name: "Item " + string(i) } in self as added
      i = i + 1
    }
    select self
  }
}
"#;
    let path = PathBuf::from("/test/numbered.atd");
    let loaded = load(&Memory(BTreeMap::from([(path.clone(), text.to_owned())])), &path);
    assert_eq!(
        loaded.json.pointer("/behavior/operations/number/actions"),
        Some(&serde_json::json!([{ "extension": "operation:number" }]))
    );

    let diagram = DislDiagram::new(&loaded.specification);
    let root = diagram.add_node("Item", "r", Map::default()).unwrap();
    let invoke = |count: i64| {
        let invocation = adp_disl::DislInvocation {
            parameters: Some(Map::from_iter([("count", Value::Int(count))])),
            ..Default::default()
        };
        let ids = ids::fixed(&["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"]);
        operations::run(
            &loaded.specification,
            "number",
            &diagram,
            Some(&root),
            &ids,
            Some(&invocation),
            None,
        )
    };

    let transaction = invoke(3);
    assert!(transaction.refusal.is_none(), "{:?}", transaction.refusal);
    let names: Vec<String> = transaction
        .changes
        .iter()
        .map(|c| match c {
            DislChange::Create { attributes, .. } => attributes.get("name").unwrap().to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(names, ["Item 1", "Item 2", "Item 3"]);
    assert!(matches!(
        &transaction.host_actions[..],
        [HostAction::Select { element_ids }] if element_ids == &["r".to_owned()]
    ));

    // A `fail` refuses the gesture with its message, and nothing of it is kept.
    let refused = invoke(12);
    assert_eq!(refused.refusal.as_deref(), Some("At most ten."));
    assert!(refused.changes.is_empty());
}

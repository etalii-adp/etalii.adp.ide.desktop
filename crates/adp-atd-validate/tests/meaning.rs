//! A case recorded from a module that opens its own documents, compared by what it means (etalii.adp spec 067, FR-012,
//! ATD §2.3, §2.4): the module's own type names, wire attributes, shapes and painted chrome are not differences; a real
//! difference in an id, a parent, an end, a value or where an element is still is; and a case from a declared tool stays
//! compared exactly.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::{RuntimeHost, RuntimeTool, diagram_tool};
use adp_atd_validate::meaning::{Reading, drawing_difference, drawn_ids, reading_differences, stated_by};
use adp_atd_validate::{Case, Difference, validate};
use serde_json::{Value as Json, json};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// The mind map's tool, a description whose element type `Node` declares the attribute `text`.
fn mind_map() -> (RuntimeTool, adp_atd_check::Description) {
    let checked = check(&FsFiles, &diagrams().join("mindmap.atd"));
    let tool = diagram_tool(
        &checked.description,
        checked.symbols.as_ref().unwrap(),
        &RuntimeHost::default(),
    )
    .unwrap();
    (tool, checked.description)
}

/// What differs when a case states `expected` and the description read `actual`, the case's drawing drawing `drawn` and
/// the description's drawing `shown`.
fn unlike(expected: Json, actual: Json, drawn: &[&str], shown: &[&str]) -> Vec<(String, String)> {
    let (tool, _) = mind_map();
    let case_draws: BTreeSet<String> = drawn.iter().map(|d| (*d).to_owned()).collect();
    let description_draws: BTreeSet<String> = shown.iter().map(|d| (*d).to_owned()).collect();
    let read = json!({"elements": expected});
    let stated = stated_by(&read);
    let reading = Reading {
        expected: &read["elements"],
        actual: &actual,
        case_draws: Some(&case_draws),
        description_draws: Some(&description_draws),
        stated: &stated,
    };
    reading_differences(tool.tool.definition.specification(), &reading)
        .into_iter()
        .map(|(want, got, _)| (want, got))
        .collect()
}

fn topic(id: &str, parent: Option<&str>, text: &str) -> Json {
    json!({"id": id, "type": "Node", "parent": parent, "attributes": {"text": text}})
}

/// The same element as a module states it: its own type, and wire attributes the description has no word for.
fn wire(id: &str, parent: Option<&str>, text: &str) -> Json {
    json!({"id": id, "type": "mine/model+node", "parent": parent,
           "attributes": {"text": text, "width": "160", "background": "#fff", "shape": "RoundedBox"}})
}

#[test]
fn a_modules_type_names_and_wire_attributes_are_not_differences() {
    let differences = unlike(
        json!([wire("a", None, "A"), wire("b", Some("a"), "B")]),
        json!([topic("a", None, "A"), topic("b", Some("a"), "B")]),
        &["a", "b"],
        &["a", "b"],
    );
    assert_eq!(differences, vec![]);
}

#[test]
fn a_different_parent_is_a_difference() {
    let differences = unlike(
        json!([wire("b", Some("a"), "B")]),
        json!([topic("b", Some("c"), "B")]),
        &["b"],
        &["b"],
    );
    assert_eq!(differences.len(), 1, "{differences:?}");
    assert!(differences[0].0.contains("parent"), "{differences:?}");
}

#[test]
fn a_different_end_is_a_difference() {
    let relation = |target: &str| json!({"id": "r", "type": "Link", "source": "a", "target": target, "attributes": {}});
    let differences = unlike(json!([relation("b")]), json!([relation("c")]), &["r"], &["r"]);
    assert_eq!(differences.len(), 1, "{differences:?}");
    assert!(differences[0].0.contains("target"), "{differences:?}");
}

#[test]
fn a_different_value_is_a_difference() {
    let differences = unlike(
        json!([wire("a", None, "A"), wire("b", None, "B")]),
        json!([topic("a", None, "Other"), topic("b", None, "B")]),
        &["a", "b"],
        &["a", "b"],
    );
    assert_eq!(differences.len(), 1, "{differences:?}");
    assert!(differences[0].0.contains("text"), "{differences:?}");
}

/// An attribute both can state that one side leaves out reads as empty on it, so losing a value is noticed.
#[test]
fn a_value_one_side_left_out_is_a_difference() {
    let lost = json!({"id": "a", "type": "Node", "parent": null, "attributes": {}});
    let differences = unlike(
        json!([wire("a", None, "A"), wire("b", None, "B")]),
        json!([lost, topic("b", None, "B")]),
        &["a", "b"],
        &["a", "b"],
    );
    assert_eq!(differences.len(), 1, "{differences:?}");
    let added = unlike(
        json!([wire("b", None, "B"), {"id": "a", "type": "mine/model+node", "parent": null, "attributes": {}}]),
        json!([topic("a", None, "A"), topic("b", None, "B")]),
        &["a", "b"],
        &["a", "b"],
    );
    assert_eq!(added.len(), 1, "{added:?}");
}

/// A case whose attributes the description has no word for has nothing to compare, and that is not a pass.
#[test]
fn a_case_with_no_attribute_in_common_is_a_difference() {
    let differences = unlike(
        json!([{"id": "a", "type": "mine/model+node", "parent": null, "attributes": {"caption": "A"}}]),
        json!([topic("a", None, "A")]),
        &["a"],
        &["a"],
    );
    assert_eq!(differences.len(), 1, "{differences:?}");
    assert!(differences[0].0.contains("caption"), "{differences:?}");
}

/// Reserved names and painted values are no statement a description could answer (ATD §2.3), alone or together.
#[test]
fn a_case_stating_only_reserved_names_and_painted_values_has_nothing_to_have_in_common() {
    for attributes in [
        json!({"kind": "depends"}),
        json!({"source": "a", "_own": "1", "$own": "2"}),
        json!({"role": "task", "label": "A", "badges": "serverless", "width": "200", "height": "56"}),
        json!({"kind": "notebook", "label": "A"}),
    ] {
        let differences = unlike(
            json!([{"id": "a", "type": "mine/model+node", "parent": null, "attributes": attributes}]),
            json!([topic("a", None, "A")]),
            &["a"],
            &["a"],
        );
        assert_eq!(differences, vec![], "{attributes}");
    }
}

/// Another attribute beside them is still a statement, and the difference names it alone.
#[test]
fn another_attribute_beside_them_with_nothing_in_common_is_a_difference() {
    let differences = unlike(
        json!([{"id": "a", "type": "mine/model+node", "parent": null,
                "attributes": {"kind": "notebook", "label": "A", "caption": "A"}}]),
        json!([topic("a", None, "A")]),
        &["a"],
        &["a"],
    );
    assert_eq!(
        differences,
        vec![("attributes in common with caption".to_owned(), "none".to_owned())]
    );
}

/// A painted value a description does declare is compared like any other.
#[test]
fn a_painted_value_the_description_declares_is_still_compared() {
    let spec = adp_disl::DislLoader::load(
        r#"{ "disl": "0.7", "language": { "id": "t", "name": "T", "version": "1.0.0" },
             "metamodel": { "types": { "Node": { "attributes": { "label": { "type": "string" } } } } } }"#,
    )
    .specification
    .expect("it loads");
    let read = json!({"elements": [{"id": "a", "type": "m", "parent": null, "attributes": {"label": "A"}}]});
    let actual = json!([{"id": "a", "type": "Node", "parent": null, "attributes": {"label": "B"}}]);
    let stated = stated_by(&read);
    let differences = reading_differences(
        &spec,
        &Reading {
            expected: &read["elements"],
            actual: &actual,
            case_draws: None,
            description_draws: None,
            stated: &stated,
        },
    );
    assert_eq!(differences.len(), 1, "{differences:?}");
    assert!(differences[0].0.contains("label"), "{differences:?}");
}

#[test]
fn an_element_the_case_draws_and_the_description_lacks_is_a_difference() {
    let differences = unlike(json!([wire("a", None, "A")]), json!([]), &["a"], &[]);
    assert_eq!(differences, vec![("a".to_owned(), "nothing".to_owned())]);
}

/// What the module presents beside what it reads, and draws no element for, is its own: what stands for the view.
#[test]
fn what_the_case_does_not_draw_and_the_description_lacks_is_the_modules_own() {
    let differences = unlike(
        json!([wire("a", None, "A"), wire("mine:view", None, "View")]),
        json!([topic("a", None, "A")]),
        &["a"],
        &["a"],
    );
    assert_eq!(differences, vec![]);
}

/// A module reads the view it shows, a description the whole body: what the view does not show is no difference, but
/// what the description draws and the case does not state is.
#[test]
fn what_the_description_reads_beyond_the_view_counts_only_when_it_draws_it() {
    let expected = json!([wire("a", None, "A")]);
    let actual = json!([topic("a", None, "A"), topic("hidden", None, "H")]);
    assert_eq!(unlike(expected.clone(), actual.clone(), &["a"], &["a"]), vec![]);
    let differences = unlike(expected, actual, &["a"], &["a", "hidden"]);
    assert_eq!(differences, vec![("nothing".to_owned(), "hidden".to_owned())]);
}

fn drawing(x: f64, target: &str) -> Json {
    json!({
        "elements": [{"id": "a", "type": "t", "shape": "rect", "bounds": {"x": x, "y": 0, "w": 10, "h": 10},
                      "labels": [{"id": "name", "text": "A"}], "style": {"fill": "#000"}}],
        "edges": [{"id": "r", "type": "t", "source": "a", "target": target, "route": [{"x": 0, "y": 0}]}],
        "chrome": [{"id": "chrome:1", "kind": "band"}],
        "order": ["r", "a", "chrome:1"],
    })
}

#[test]
fn a_drawing_differs_in_which_elements_appear_and_where() {
    assert_eq!(drawing_difference(&drawing(0.0, "b"), &drawing(0.0, "b")), None);
    let moved = drawing_difference(&drawing(0.0, "b"), &drawing(5.0, "b")).expect("a moved element");
    assert_eq!(moved.2, "a");
    let other_end = drawing_difference(&drawing(0.0, "b"), &drawing(0.0, "c")).expect("another end");
    assert_eq!(other_end.2, "r");
    let mut absent = drawing(0.0, "b");
    absent["elements"] = json!([]);
    assert_eq!(
        drawing_difference(&drawing(0.0, "b"), &absent).expect("no element").2,
        "a"
    );
    let mut resized = drawing(0.0, "b");
    resized["elements"][0]["bounds"]["w"] = json!(20);
    assert_eq!(drawing_difference(&drawing(0.0, "b"), &resized).expect("a size").2, "a");
    assert_eq!(
        drawn_ids(&drawing(0.0, "b")),
        BTreeSet::from(["a".to_owned(), "r".to_owned()])
    );
}

#[test]
fn a_drawings_type_shape_labels_style_chrome_and_order_are_the_modules_painting() {
    let mut painted = drawing(0.0, "b");
    painted["elements"][0]["type"] = json!("mine/model+node");
    painted["elements"][0]["shape"] = json!("path");
    painted["elements"][0]["labels"] = json!([]);
    painted["elements"][0]["style"] = json!({"fill": "#fff"});
    painted["edges"][0]["type"] = json!("mine/model+relationship");
    painted["edges"][0]["route"] = json!([{"x": 9, "y": 9}]);
    painted["chrome"] = json!([]);
    painted["order"] = json!(["a", "r"]);
    assert_eq!(drawing_difference(&drawing(0.0, "b"), &painted), None);
}

/// The mind map's case `example-1`, read only.
fn read_only() -> Case {
    let dir = diagrams().join("conformance/mindmap/example-1");
    let files = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    let mut case = adp_atd_validate::case_of("example-1".to_owned(), files).unwrap();
    case.fixture["steps"] = json!([]);
    case
}

fn parts(differences: &[Difference]) -> BTreeSet<&'static str> {
    differences.iter().map(|d| d.part).collect()
}

/// Wire attributes and a module's type name on every element it states.
fn in_wire_terms(case: &mut Case) {
    for element in case.fixture["read"]["elements"].as_array_mut().unwrap() {
        element["type"] = json!("mine/model+node");
        element["attributes"]["background"] = json!("#fff");
    }
}

#[test]
fn a_case_says_it_comes_from_a_module_and_is_compared_by_what_it_means() {
    let (tool, description) = mind_map();
    let mut case = read_only();
    in_wire_terms(&mut case);
    // Unsaid, the case comes from a declared tool and is compared exactly.
    assert!(parts(&validate(&tool, &description, &case)).contains("reading"));
    case.fixture["recordedFrom"] = json!("declared");
    assert!(parts(&validate(&tool, &description, &case)).contains("reading"));
    case.fixture["recordedFrom"] = json!("module");
    let found = validate(&tool, &description, &case);
    assert!(!parts(&found).contains("reading"), "{found:#?}");
}

#[test]
fn a_real_difference_in_a_case_from_a_module_is_still_reported() {
    let (tool, description) = mind_map();
    let mut case = read_only();
    in_wire_terms(&mut case);
    case.fixture["recordedFrom"] = json!("module");
    let first_with_parent = case.fixture["read"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .position(|e| !e["parent"].is_null())
        .unwrap();

    let mut other_parent = case.clone();
    other_parent.fixture["read"]["elements"][first_with_parent]["parent"] = json!("somewhere");
    assert!(parts(&validate(&tool, &description, &other_parent)).contains("reading"));

    let mut other_value = case.clone();
    other_value.fixture["read"]["elements"][0]["attributes"]["text"] = json!("Something else");
    assert!(parts(&validate(&tool, &description, &other_value)).contains("reading"));
}

/// Findings stay exact in a case from a module, and so do bytes.
#[test]
fn findings_are_compared_exactly_in_a_case_from_a_module() {
    let (tool, description) = mind_map();
    let mut case = read_only();
    case.fixture["recordedFrom"] = json!("module");
    case.fixture["read"]["findings"] = json!([{"rule": "x", "severity": "warning", "message": "m"}]);
    assert!(parts(&validate(&tool, &description, &case)).contains("findings"));

    let mut with_step = read_only();
    with_step.fixture["recordedFrom"] = json!("module");
    with_step.fixture["steps"] = json!([{"undo": true, "expect": "not the bytes", "refused": {"reason": "No."}}]);
    let found = validate(&tool, &description, &with_step);
    assert!(parts(&found).contains("bytes"), "{found:#?}");
}

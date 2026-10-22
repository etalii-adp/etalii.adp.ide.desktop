//! Budgets (DISL section 3.2.1; the standalone host's `BudgetsTests` at `c9af7c5`): what a budget counts, which
//! candidates a truncating one keeps - by `order`, ties in persistence order, whole `unit`s while they fit - and what
//! it withholds, read in CEL with `budget(id)` and reported as a finding. The standalone host's notice is drawn by its
//! canvas chrome, which this runtime does not derive.

mod common;

use adp_cel::{Map, Value};
use adp_disl::behavior::ids;
use adp_disl::behavior::operations;
use adp_disl::derivation::{constraints, context_menu, forms, gestures};
use adp_disl::model::budgets;
use adp_disl::{DislConstraintOptions, DislDiagram, DislEnv, DislMenuTarget, DislSpecification, WireIdMap, contexts};
use common::{attrs, loaded, vars, with};

const METAMODEL: &str = r#""metamodel": {
  "types": { "Item": { "attributes": { "name": { "type": "string" }, "rank": { "type": "int" }, "group": { "type": "string" } } } },
  "relations": { "Link": { "source": "Item", "target": "Item" } } }"#;

fn specification(budgets: &str, extra: &str) -> DislSpecification {
    loaded(&with(&format!(
        r#"{METAMODEL}, "language": {{ "id": "org.example.test", "version": "1.0.0", "limits": {{ "budgets": {budgets} }} }}{extra}"#
    )))
}

fn diagram(specification: &DislSpecification, items: &[(&str, i64, &str)]) -> DislDiagram {
    let diagram = DislDiagram::new(specification);
    for (id, rank, group) in items {
        diagram
            .add_node(
                "Item",
                id,
                attrs([
                    ("name", Value::from(*id)),
                    ("rank", Value::Int(*rank)),
                    ("group", Value::from(*group)),
                ]),
            )
            .unwrap();
    }
    diagram
}

fn drawn(diagram: &DislDiagram) -> Vec<String> {
    diagram
        .nodes()
        .iter()
        .filter(|node| diagram.is_drawn(node))
        .map(|node| node.id())
        .collect()
}

fn cel(diagram: &DislDiagram, expression: &str) -> Value {
    diagram
        .specification()
        .environment(contexts::ELEMENT, &[])
        .compile(expression)
        .unwrap()
        .evaluate(&vars([
            ("self", diagram.to_value()),
            ("diagram", diagram.to_value()),
            ("env", Value::map(DislEnv::default().to_cel())),
        ]))
        .unwrap()
}

fn state(shown: i64, total: i64, truncated: bool, withheld: bool) -> Value {
    let mut map = Map::default();
    map.insert("shown", Value::Int(shown));
    map.insert("total", Value::Int(total));
    map.insert("truncated", Value::Bool(truncated));
    map.insert("withheld", Value::Bool(withheld));
    Value::map(map)
}

#[test]
fn a_count_budget_keeps_the_first_max_in_persistence_order_and_cel_reads_its_state() {
    let specification = specification(r#"{ "items": { "measure": { "count": ["Item"] }, "max": 2 } }"#, "");
    let diagram = diagram(&specification, &[("a", 3, "x"), ("b", 1, "x"), ("c", 2, "y")]);

    let state_of = diagram.budget("items").unwrap();

    assert_eq!(
        (2, 3, true, true, true),
        (
            state_of.shown,
            state_of.total,
            state_of.exceeded,
            state_of.truncated,
            state_of.withheld
        )
    );
    assert_eq!(vec!["a", "b"], drawn(&diagram));
    assert_eq!(state(2, 3, true, true), cel(&diagram, "budget('items')"));
}

#[test]
fn a_budget_is_computed_again_after_the_diagram_changes() {
    let specification = specification(r#"{ "items": { "measure": { "count": ["Item"] }, "max": 2 } }"#, "");
    let diagram = diagram(&specification, &[("a", 0, "x"), ("b", 0, "x")]);
    assert!(!diagram.budget("items").unwrap().exceeded);

    diagram.add_node("Item", "c", Map::default()).unwrap();

    assert!(diagram.budget("items").unwrap().exceeded);
}

#[test]
fn an_order_sorts_the_candidates_list_keys_member_by_member_ties_in_persistence_order() {
    let specification = specification(
        r#"{ "items": { "measure": { "count": ["Item"] }, "max": 2, "order": "[self.group, self.rank]" } }"#,
        "",
    );
    let diagram = diagram(
        &specification,
        &[("a", 3, "y"), ("b", 2, "x"), ("c", 2, "x"), ("d", 1, "y")],
    );

    assert_eq!(vec!["b", "c"], drawn(&diagram));
}

#[test]
fn a_unit_is_kept_whole_and_the_first_unit_that_does_not_fit_ends_the_cut() {
    let specification = specification(
        r#"{ "items": { "measure": { "count": ["Item"] }, "max": 3, "unit": "self.group" } }"#,
        "",
    );
    let diagram = diagram(
        &specification,
        &[
            ("a", 0, "x"),
            ("b", 0, "y"),
            ("c", 0, "x"),
            ("d", 0, "y"),
            ("e", 0, "z"),
        ],
    );

    assert_eq!(vec!["a", "c"], drawn(&diagram));
    assert_eq!(2, diagram.budget("items").unwrap().shown);
}

#[test]
fn a_relation_is_drawn_only_when_both_its_ends_are() {
    let specification = specification(r#"{ "items": { "measure": { "count": ["Item"] }, "max": 2 } }"#, "");
    let diagram = diagram(&specification, &[("a", 0, "x"), ("b", 0, "x"), ("c", 0, "x")]);
    let nodes = diagram.nodes();
    let kept = diagram
        .add_relation("Link", "ab", Some(&nodes[0]), Some(&nodes[1]), Map::default())
        .unwrap();
    let cut = diagram
        .add_relation("Link", "bc", Some(&nodes[1]), Some(&nodes[2]), Map::default())
        .unwrap();

    assert!(diagram.is_drawn(&kept));
    assert!(!diagram.is_drawn(&cut));
}

#[test]
fn a_cel_measure_is_exceeded_without_cutting_anything() {
    let specification = specification(
        r#"{ "size": { "measure": { "cel": "diagram.nodesOfType('Item').size() * 10" }, "max": 15 } }"#,
        "",
    );
    let diagram = diagram(&specification, &[("a", 0, "x"), ("b", 0, "x")]);

    let size = diagram.budget("size").unwrap();

    assert_eq!(
        (20, 20, true, false, true),
        (size.shown, size.total, size.exceeded, size.truncated, size.withheld)
    );
    assert_eq!(vec!["a", "b"], drawn(&diagram));
}

#[test]
fn budget_without_an_id_aggregates_every_budget() {
    let specification = specification(
        r#"{ "items": { "measure": { "count": ["Item"] }, "max": 5 }, "size": { "measure": { "cel": "7" }, "max": 3, "withhold": { "edits": "none" } } }"#,
        "",
    );
    let diagram = diagram(&specification, &[("a", 0, "x")]);

    assert_eq!(state(8, 8, false, false), cel(&diagram, "budget()"));
}

const EDITING: &str = r#", "forms": { "inspector": { "for": ["Item"], "usage": ["inspector"], "items": [ { "attribute": "name", "id": "name", "widget": "text" } ] } },
"toolbox": { "contextMenus": [ { "for": ["Item"], "tools": [
  { "kind": "operation", "operation": "rename", "label": "Rename" },
  { "kind": "operation", "operation": "focus", "label": "Focus" },
  { "kind": "delete", "label": "Delete" } ] } ] },
"behavior": { "operations": {
  "rename": { "for": ["Item"], "actions": [ { "set": { "name": "'renamed'" } } ] },
  "focus": { "for": ["Item"], "actions": [ { "select": "self" } ] } } }"#;

fn withholding(edits: &str) -> DislSpecification {
    specification(
        &format!(
            r#"{{ "items": {{ "measure": {{ "count": ["Item"] }}, "max": 1, "withhold": {{ "edits": "{edits}", "reason": "Too many items to edit them here." }} }} }}"#
        ),
        EDITING,
    )
}

#[test]
fn a_withheld_edit_is_the_reason_of_every_model_changing_entry_and_of_every_row() {
    let specification = withholding("model");
    let diagram = diagram(&specification, &[("a", 0, "x"), ("b", 0, "x")]);
    let item = diagram.nodes()[0].clone();

    let entries: Vec<String> = context_menu::derive(
        &specification,
        &DislMenuTarget::Element(item.clone()),
        &DislEnv::default(),
        &WireIdMap::default(),
    )
    .into_iter()
    .flat_map(|group| group.entries)
    .map(|entry| format!("{}: {}", entry.label, entry.unavailable_reason))
    .collect();
    let rows: Vec<String> = forms::derive(
        &specification,
        &item,
        &DislEnv::default(),
        &WireIdMap::default(),
        "inspector",
    )
    .into_iter()
    .map(|row| row.read_only_reason)
    .collect();

    assert_eq!(
        vec![
            "Rename: Too many items to edit them here.",
            "Focus: ",
            "Delete: Too many items to edit them here."
        ],
        entries
    );
    assert_eq!(vec!["Too many items to edit them here."], rows);
}

#[test]
fn a_withheld_edit_refuses_the_operation_and_a_move_only_under_all() {
    let model = withholding("model");
    let all = withholding("all");
    let model_diagram = diagram(&model, &[("a", 0, "x"), ("b", 0, "x")]);
    let all_diagram = diagram(&all, &[("a", 0, "x"), ("b", 0, "x")]);
    let bounds = || attrs([("x", Value::Double(0.0)), ("y", Value::Double(0.0))]);

    let renamed = operations::run(
        &model,
        "rename",
        &model_diagram,
        Some(&model_diagram.nodes()[0]),
        &ids::fixed(&[]),
        None,
        None,
    );
    let moved_under_model = gestures::placement(
        &model,
        &model_diagram.nodes()[0],
        "move",
        bounds(),
        bounds(),
        None,
        None,
    );
    let moved_under_all = gestures::placement(&all, &all_diagram.nodes()[0], "move", bounds(), bounds(), None, None);
    let changed = gestures::change(&model, &model_diagram.nodes()[0], "name", Value::from("other"), None);

    assert_eq!(Some("Too many items to edit them here."), renamed.refusal.as_deref());
    assert!(moved_under_model.is_empty());
    assert_eq!(
        vec![budgets::WITHHELD],
        moved_under_all.iter().map(|f| f.code.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["Too many items to edit them here."],
        changed.iter().map(|f| f.message.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn an_exceeded_budget_reports_its_finding() {
    let specification = specification(
        r#"{ "items": { "measure": { "count": ["Item"] }, "max": 1, "finding": { "severity": "warning", "message": { "cel": "'There are ' + string(budget('items').total) + ' items.'" } } } }"#,
        "",
    );
    let small = diagram(&specification, &[("a", 0, "x")]);
    let large = diagram(&specification, &[("a", 0, "x"), ("b", 0, "x")]);
    let options = DislConstraintOptions::default();

    assert!(constraints::evaluate(&specification, &small, &options).is_empty());
    let found = constraints::evaluate(&specification, &large, &options);
    assert_eq!(1, found.len());
    assert_eq!(
        ("budget:items", "warning", "There are 2 items."),
        (
            found[0].code.as_str(),
            found[0].severity.as_str(),
            found[0].message.as_str()
        )
    );
}

#[test]
fn a_count_measure_names_types_and_its_texts_compile_in_their_own_contexts() {
    let specification = specification(
        r#"{ "items": { "measure": { "count": ["Item", "Link"] }, "max": 1, "order": "self.rank", "unit": "self.group",
            "notice": { "text": { "cel": "string(budget('items').shown)" } },
            "withhold": { "reason": { "cel": "'Too many: ' + string(budget('items').total)" } },
            "finding": { "message": { "cel": "string(budget('items').total)" } } } }"#,
        "",
    );

    let at = |pointer: &str| specification.expression_at(pointer).map(|e| e.context.clone());

    assert_eq!(None, at("/language/limits/budgets/items/measure/count/0"));
    assert_eq!(
        Some(contexts::BUDGET.to_owned()),
        at("/language/limits/budgets/items/order")
    );
    assert_eq!(
        Some(contexts::BUDGET.to_owned()),
        at("/language/limits/budgets/items/unit")
    );
    assert_eq!(
        Some(contexts::CHROME.to_owned()),
        at("/language/limits/budgets/items/notice/text/cel")
    );
    assert_eq!(
        Some(contexts::ELEMENT.to_owned()),
        at("/language/limits/budgets/items/withhold/reason/cel")
    );
    assert_eq!(
        Some(contexts::CONSTRAINT.to_owned()),
        at("/language/limits/budgets/items/finding/message/cel")
    );
}

//! A session over the company example: what it delivers, and that dragging a node up or down its column moves its
//! entry and nothing else. Ported from the standalone host's `EtAlii.Adp.Diagram.Sankey.Tests/SankeySession.Tests.cs`
//! and `SankeyContextSourceResolver.Tests.cs` at `8b6ccfc`. A session there subscribed to the store's change event; a
//! desktop session is told of a change by the shell (`DiagramSession::on_document_changed`), so not ported:
//! `AReadOnlySession_RefusesAMove` (every desktop session edits through the project's history) and
//! `ADisposedSession_HearsNoMoreEdits` (a desktop session subscribes to nothing).

mod common;

use adp_diagram::change_handler::DocumentChange;
use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_sankey::layout::SankeyLayout;
use adp_diagram_sankey::lines::LineDocument;
use adp_diagram_sankey::mapper::{FLOW_TYPE, LEFT_SIDE, NODE_TYPE, RIGHT_SIDE};
use adp_diagram_sankey::parser;
use adp_diagram_sankey::view::{NOT_MOVABLE, PARENT_REFUSAL, place_of};
use common::{Diagram, brightwater_coffee, open};

fn company() -> Diagram {
    open(&brightwater_coffee(), "brightwater-coffee")
}

fn delivered(diagram: &Diagram) -> Vec<DiagramElement> {
    diagram.session.render(diagram.workspace.documents()).unwrap()
}

fn find<'a>(elements: &'a [DiagramElement], id: &str) -> &'a DiagramElement {
    elements.iter().find(|e| e.id == id).unwrap()
}

fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap()
}

#[test]
fn a_fresh_session_delivers_every_node_and_flow_nodes_first() {
    let diagram = company();
    let model = diagram.folder.model();
    let elements = delivered(&diagram);
    let ids: Vec<String> = elements.iter().map(|e| e.id.clone()).collect();
    let expected: Vec<String> = model
        .nodes
        .iter()
        .map(|n| n.id.clone())
        .chain(model.flows.iter().map(|f| f.id()))
        .collect();
    assert_eq!(ids, expected);
    assert!(
        elements[..model.nodes.len()]
            .iter()
            .all(|e| e.element_type == NODE_TYPE)
    );
    assert!(
        elements[model.nodes.len()..]
            .iter()
            .all(|e| e.element_type == FLOW_TYPE && e.kind == ElementKind::Relation)
    );
}

#[test]
fn a_node_arrives_with_its_value_in_its_format_and_its_labels_on_the_right_side() {
    let diagram = company();
    let elements = delivered(&diagram);
    let cafes = find(&elements, "cafes");
    assert_eq!(
        (
            cafes.attribute("name"),
            number(cafes, "value"),
            cafes.attribute("displayValue"),
            cafes.attribute("note"),
            cafes.attribute("color"),
            cafes.attribute("side")
        ),
        ("Cafés", 1840.0, "€1,840M", "+9% Y/Y", "blue", LEFT_SIDE)
    );
    let tax = find(&elements, "tax");
    assert_eq!(
        (
            tax.attribute("displayValue"),
            tax.attribute("color"),
            tax.attribute("side"),
            tax.attribute("column")
        ),
        ("(€140M)", "red", RIGHT_SIDE, "4")
    );
    assert_eq!(number(find(&elements, "revenue"), "value"), 3500.0);
}

#[test]
fn a_flow_takes_its_targets_colour_and_is_as_thick_as_its_value() {
    let diagram = company();
    let elements = delivered(&diagram);
    let cost = find(&elements, "revenue->cost-of-revenue");
    let profit = find(&elements, "revenue->gross-profit");
    assert_eq!((cost.attribute("color"), profit.attribute("color")), ("red", "green"));
    let ratio = number(cost, "thickness") / number(profit, "thickness");
    assert!((ratio - 1890.0 / 1610.0).abs() < 0.005, "{ratio}");
    let sum = number(cost, "thickness") + number(profit, "thickness");
    assert!(
        (number(find(&elements, "revenue"), "height") - sum).abs() < 0.05,
        "{sum}"
    );
    assert_eq!(cost.source.as_deref(), Some("revenue"));
    assert_eq!(cost.target.as_deref(), Some("cost-of-revenue"));
}

#[test]
fn dragging_a_node_below_its_neighbour_moves_its_entry_after_it() {
    let mut diagram = company();
    let layout = SankeyLayout::of(&diagram.folder.model());
    let cafes = layout.boxes["cafes"];
    let packaged = layout.boxes["packaged"];
    // The cafés dropped with their middle just below the packaged coffee's.
    diagram
        .session
        .move_elements(
            &mut diagram.workspace,
            &[(
                "cafes".to_owned(),
                cafes.x + 40.0,
                packaged.centre_y() + 1.0 - cafes.height / 2.0,
            )],
        )
        .unwrap();
    // The order changed, the column did not.
    let after = SankeyLayout::of(&diagram.folder.model());
    assert_eq!(after.column_order[0], ["packaged", "cafes", "wholesale", "licensing"]);
    assert_eq!(after.boxes["cafes"].x, cafes.x);
}

#[test]
fn dropping_a_node_back_in_its_place_writes_nothing() {
    let mut diagram = company();
    let original = diagram.folder.bytes();
    let bar = SankeyLayout::of(&diagram.folder.model()).boxes["wholesale"];
    diagram
        .session
        .move_elements(
            &mut diagram.workspace,
            &[("wholesale".to_owned(), bar.x + 200.0, bar.y + 5.0)],
        )
        .unwrap();
    assert_eq!(diagram.folder.bytes(), original);
    assert!(!diagram.workspace.history().availability().can_undo);
}

#[test]
fn a_place_is_before_the_first_lower_node_or_after_the_last() {
    // A small column of three.
    let layout = SankeyLayout::of(&parser::parse(&LineDocument::parse(
        "sankey: 1\nnodes:\n  - id: a\n  - id: m\n  - id: b\n  - id: z\n    column: 2\nflows:\n  - from: a\n    to: z\n    value: 1\n  - from: m\n    to: z\n    value: 1\n  - from: b\n    to: z\n    value: 1\n",
    )));
    assert_eq!(place_of(&layout, "m", -1000.0), Some(("a".to_owned(), false)));
    assert_eq!(place_of(&layout, "m", 1000.0), Some(("b".to_owned(), true)));
}

#[test]
fn dragging_something_that_is_not_a_node_is_refused() {
    let mut diagram = company();
    let refusal = diagram.session.move_elements(
        &mut diagram.workspace,
        &[("revenue->gross-profit".to_owned(), 0.0, 0.0)],
    );
    assert_eq!(refusal, Err(NOT_MOVABLE.to_owned()));
    let reparent = diagram
        .session
        .move_element(&mut diagram.workspace, "cafes", Some("revenue"), -1);
    assert_eq!(reparent, Err(PARENT_REFUSAL.to_owned()));
}

#[test]
fn an_edit_reaches_the_session_as_deltas() {
    let mut diagram = company();
    let layout = SankeyLayout::of(&diagram.folder.model());
    let cafes = layout.boxes["cafes"];
    diagram
        .session
        .move_elements(
            &mut diagram.workspace,
            &[(
                "cafes".to_owned(),
                cafes.x,
                layout.boxes["packaged"].centre_y() + 1.0 - cafes.height / 2.0,
            )],
        )
        .unwrap();
    let body = diagram.folder.body.clone();
    let change = diagram
        .session
        .on_document_changed(diagram.workspace.documents(), &body);
    assert!(matches!(change, DocumentChange::Changed(deltas) if !deltas.is_empty()));
}

/// `SankeyContextSourceResolver`: a node is selected by its label, a flow by its ends, and nothing else resolves.
#[test]
fn a_selection_reads_as_a_nodes_label_or_a_flows_ends() {
    let diagram = company();
    let documents = diagram.workspace.documents();
    let cafes = diagram.session.select(documents, "cafes").unwrap();
    assert_eq!((cafes.path, cafes.text.as_str()), (vec!["Cafés".to_owned()], "Cafés"));
    assert_eq!(cafes.element.element_type, NODE_TYPE);
    let flow = diagram.session.select(documents, "revenue->gross-profit").unwrap();
    assert_eq!(flow.path.len(), 1);
    assert!(flow.path[0].contains(" → "), "{:?}", flow.path);
    assert_eq!(flow.element.element_type, FLOW_TYPE);
    assert!(diagram.session.select(documents, "no-such-node").is_none());
}

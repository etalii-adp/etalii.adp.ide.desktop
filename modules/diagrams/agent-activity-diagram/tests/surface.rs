//! What the shell is offered for an activity file (agent-activity-diagram Requirements 3.7, 6.2, 9.1, 9.4 and 9.6):
//! the palette, an element's menu, a gesture's action, the property grid's rows, a selection, and a drag. Ported from
//! the standalone host's `AadSurface.Tests.cs`, `AadContextSourceResolver.Tests.cs`, `AadSession.Tests.cs` and
//! `ServiceCollectionAddAgentActivityDiagram.Tests.cs` at `8b6ccfc`.

mod common;

use std::fs;
use std::path::Path;

use adp_context::{Prompt, PropertyEditor};
use adp_diagram::{ActionOutcome, ActionTarget, DiagramWorkspace};
use adp_diagram_agent_activity::view::PARENT_REFUSAL;
use common::{element, fixtures, ids_of, open_one_day, tools};

#[test]
fn the_palette_has_one_tool_per_kind_of_element_each_dropping_its_own_add() {
    let diagram = open_one_day();
    assert_eq!(
        vec![
            "project:add-project",
            "specification:add-specification",
            "agent:add-agent",
            "location:add-location",
            "environment:add-environment"
        ],
        diagram
            .session
            .toolbox(diagram.workspace.documents())
            .iter()
            .map(|t| format!("{}:{}", t.id, t.drop_action_id))
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_elements_menu_is_the_definitions_less_what_this_host_does_elsewhere() {
    let diagram = open_one_day();
    let menu = |id: &str| diagram.session.actions(diagram.workspace.documents(), &element(id));
    let locked = menu("s-knowledge");
    let free = menu("a-dev2");
    let task = menu("t-k2");
    // No Open link (its symbol opens it); Unlock position where there is a lock and Lock position where there is none.
    assert_eq!(vec!["editLabel", "create-child", "unpin", "delete"], ids_of(&locked));
    assert!(ids_of(&free).contains(&"pin".to_owned()));
    assert!(!ids_of(&free).contains(&"unpin".to_owned()));
    let pin = free.iter().flat_map(|g| &g.actions).find(|a| a.id == "pin").unwrap();
    assert_eq!("Lock position", pin.label);
    assert!(pin.is_available());
    // A task is not placed, and its status is chosen from a submenu of the definition's statuses, in its order.
    assert_eq!(vec!["editLabel", "setTaskStatus", "delete"], ids_of(&task));
    let status = task
        .iter()
        .flat_map(|g| &g.actions)
        .find(|a| a.id == "setTaskStatus")
        .unwrap();
    assert_eq!(
        vec![
            "setTaskStatus:progressing=Progressing",
            "setTaskStatus:pending=Pending",
            "setTaskStatus:inputRequired=Input Required",
            "setTaskStatus:finished=Finished"
        ],
        status
            .items
            .iter()
            .flat_map(|g| &g.actions)
            .map(|a| format!("{}={}", a.id, a.label))
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_gestures_target_discovers_what_the_gesture_executes_and_executing_it_writes_the_file() {
    let mut diagram = open_one_day();
    let documents = diagram.workspace.documents();
    assert_eq!(
        vec![
            "add-project",
            "add-specification",
            "add-agent",
            "add-location",
            "add-environment",
            "unpinAll"
        ],
        ids_of(
            &diagram
                .session
                .actions(documents, &ActionTarget::Canvas { x: 10.0, y: 20.0 })
        )
    );
    let relation = ActionTarget::Connection {
        source: Some("s-rider".into()),
        target: Some("a-dev2".into()),
    };
    assert_eq!(vec!["connect"], ids_of(&diagram.session.actions(documents, &relation)));
    assert_eq!(
        vec!["show-archived", "hide-archived"],
        ids_of(&diagram.session.actions(documents, &element("diagram view")))
    );
    assert_eq!(
        vec!["collapse-group", "expand-group"],
        ids_of(&diagram.session.actions(documents, &element("s-knowledge#finished")))
    );

    // A fold, the switch, and a line the rules refuse.
    let folded = diagram.execute(&element("s-knowledge#progressing"), "collapse-group");
    let shown = diagram.execute(&element("diagram view"), "show-archived");
    let refused = diagram.execute(&relation, "connect");
    assert_eq!(ActionOutcome::Done, folded);
    assert_eq!(ActionOutcome::Done, shown);
    let model = diagram.model();
    assert!(model.collapsed["s-knowledge"].contains("progressing"));
    assert!(model.show_archived);
    match refused {
        ActionOutcome::Refused(reason) => assert!(reason.contains("already joined"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_rename_is_asked_for_an_element_and_for_a_row_and_an_empty_name_is_refused() {
    let mut diagram = open_one_day();
    let initial = |outcome: ActionOutcome| match outcome {
        ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) => initial_value,
        other => panic!("{other:?}"),
    };
    let location = initial(diagram.execute(&element("l-kd"), "editLabel"));
    let row = initial(diagram.execute(&element("pr-150"), "editLabel"));
    let empty = diagram.answer(&element("l-kd"), "editLabel", "  ");
    let committed = diagram.answer(&element("l-kd"), "editLabel", "features/knowledge");
    // A location is named by its branch, and that is what a rename writes.
    assert_eq!("features/knowledge-designer", location);
    assert_eq!("150: Knowledge designer, tasks 2 and 6", row);
    assert_eq!(ActionOutcome::Refused("It needs a name.".into()), empty);
    assert_eq!(ActionOutcome::Done, committed);
    assert!(diagram.text().contains("    branch: features/knowledge\r\n"));
}

#[test]
fn the_property_grid_lists_a_types_attributes_a_status_as_a_choice_and_a_relation_by_name() {
    let mut diagram = open_one_day();
    let rows = diagram
        .session
        .properties(diagram.workspace.documents(), &["s-knowledge"]);
    assert_eq!(
        vec![
            "name=Knowledge designer",
            "status=Progressing",
            "project=etalii.adp.ide.standalone",
            "link=.spec-workflow/specs/knowledge-designer/requirements.md"
        ],
        rows.iter().map(|r| format!("{}={}", r.id, r.value)).collect::<Vec<_>>()
    );
    assert_eq!(
        vec!["title", "status", "updated", "link"],
        diagram
            .session
            .properties(diagram.workspace.documents(), &["t-k2"])
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>()
    );
    let status = rows.iter().find(|r| r.id == "status").unwrap();
    assert_eq!(PropertyEditor::Choice, status.editor);
    assert_eq!(
        vec!["Pending", "Progressing", "Input Required", "Finished", "Archived"],
        status.candidates
    );
    assert!(rows.iter().find(|r| r.id == "project").unwrap().is_read_only());
    diagram
        .session
        .set_property(&mut diagram.workspace, &["s-knowledge"], "status", "Finished")
        .unwrap();
    assert!(diagram.text().contains("    status: finished\r\n    tasks:"));
}

#[test]
fn a_selection_reads_as_the_element_the_row_the_relation_the_switch_or_the_group_it_names() {
    let diagram = open_one_day();
    let documents = diagram.workspace.documents();
    let text = |id: &str| diagram.session.select(documents, id).map(|s| (s.path, s.text));
    assert_eq!(
        Some((vec!["Knowledge designer".to_owned()], "Knowledge designer".to_owned())),
        text("s-knowledge")
    );
    assert_eq!(Some("Trial the three bindings".to_owned()), text("t-k2").map(|t| t.1));
    assert_eq!(
        Some("Knowledge designer → Developer 2".to_owned()),
        text("specification:a-dev2").map(|t| t.1)
    );
    assert_eq!(Some(vec!["Diagram".to_owned()]), text("diagram view").map(|t| t.0));
    assert_eq!(
        Some(vec!["Group".to_owned()]),
        text("s-knowledge#finished").map(|t| t.0)
    );
    // A group of an element that is gone resolves to nothing.
    assert_eq!(None, text("s-gone#finished"));
}

#[test]
fn a_drop_locks_the_element_where_it_was_let_go() {
    let mut diagram = open_one_day();
    diagram
        .session
        .move_elements(&mut diagram.workspace, &[("a-dev2".into(), 120.0, 40.0)])
        .unwrap();
    assert!(
        diagram
            .text()
            .contains("    - element: a-dev2\r\n      x: 120\r\n      y: 40\r\n")
    );
}

#[test]
fn a_drag_onto_another_element_is_refused_in_this_diagrams_words() {
    let mut diagram = open_one_day();
    let refusal = diagram
        .session
        .move_element(&mut diagram.workspace, "a-dev2", Some("s-knowledge"), 0);
    assert_eq!(Err(PARENT_REFUSAL.to_owned()), refusal);
}

#[test]
fn lock_position_locks_an_element_where_the_layout_drew_it() {
    let mut diagram = open_one_day();
    let drawn = diagram
        .session
        .render(diagram.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == "a-dev2")
        .unwrap();
    assert_eq!(ActionOutcome::Done, diagram.execute(&element("a-dev2"), "pin"));
    assert_eq!(
        Some(&adp_diagram_agent_activity::model::AadPlacement {
            x: drawn.x.round_ties_even(),
            y: drawn.y.round_ties_even()
        }),
        diagram.model().placements.get("a-dev2")
    );
    // Locked where it was, it stays there.
    let again = diagram
        .session
        .render(diagram.workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == "a-dev2")
        .unwrap();
    assert!((again.x - drawn.x).abs() <= 0.5 && (again.y - drawn.y).abs() <= 0.5);
}

#[test]
fn an_example_opens_through_its_registration_and_every_element_is_laid_out_apart() {
    let folder = tempfile::tempdir().unwrap();
    for name in ["two-projects.adp", "two-projects.aad"] {
        fs::copy(
            fixtures().join("examples/two-projects").join(name),
            folder.path().join(name),
        )
        .unwrap();
    }
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, session) = workspace.open(&mut tools(), Path::new("two-projects.adp")).unwrap();
    assert_eq!("etalii/agent-activity-diagram", opened.origin);
    assert_eq!(folder.path().join("two-projects.aad"), opened.body_path);
    let elements = session.render(workspace.documents()).unwrap();
    let cards: Vec<_> = elements
        .iter()
        .filter(|e| e.kind == adp_diagram::ElementKind::Node && e.id != "diagram view")
        .collect();
    assert_eq!(13, cards.len());
    // Projects in the middle, environments outermost.
    let distance = |id: &str| {
        let e = cards.iter().find(|e| e.id == id).unwrap();
        (e.x * e.x + e.y * e.y).sqrt()
    };
    assert!(
        distance("p-ide") < distance("e-fractal"),
        "{} {}",
        distance("p-ide"),
        distance("e-fractal")
    );
    // No two cards overlap.
    for (i, a) in cards.iter().enumerate() {
        for b in &cards[i + 1..] {
            let (aw, ah) = (common_number(a, "width"), common_number(a, "height"));
            let (bw, bh) = (common_number(b, "width"), common_number(b, "height"));
            let apart = (a.x - b.x).abs() >= (aw + bw) / 2.0 - 0.5 || (a.y - b.y).abs() >= (ah + bh) / 2.0 - 0.5;
            assert!(apart, "{} and {} overlap", a.id, b.id);
        }
    }
    // The same file gives the same picture.
    let again = session.render(workspace.documents()).unwrap();
    assert_eq!(elements, again);
}

fn common_number(element: &adp_diagram::DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap()
}

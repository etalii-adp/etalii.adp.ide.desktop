//! The commands that write an activity file (agent-activity-diagram Requirements 3, 5, 6, 8 and 9), each run through
//! the project's history against a file on disk, and each judged by the file's text afterwards: what a command must
//! write, and every line it must leave alone. Ported from the standalone host's `AadCommands.Tests.cs` and
//! `AadCommandHandlers.Tests.cs` at `8b6ccfc`.

mod common;

use std::collections::BTreeSet;

use adp_diagram::ActionTarget;
use adp_diagram_agent_activity::commands::AadEdit;
use adp_diagram_agent_activity::mapper;
use common::{Diagram, lines, one_day, open, open_one_day};

fn new_file() -> Diagram {
    open("agent-activity-diagram: 1\r\n")
}

fn drop(diagram: &mut Diagram, tool: &str, x: f64, y: f64, id: &str) {
    diagram
        .run(AadEdit::AddElement {
            tool: tool.into(),
            x,
            y,
            id: id.into(),
        })
        .unwrap();
}

fn is_short_guid(id: &str) -> bool {
    id.len() == 25 && id.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
}

fn is_moment(text: &str) -> bool {
    // yyyy-MM-ddTHH:mm:ss+hh:mm
    let bytes = text.as_bytes();
    text.len() == 25
        && bytes[4] == b'-'
        && bytes[10] == b'T'
        && (bytes[19] == b'+' || bytes[19] == b'-')
        && bytes[22] == b':'
}

/// The lines of `after` that `before` does not have, in order.
fn added(before: &str, after: &str) -> Vec<String> {
    let before = lines(before);
    lines(after).into_iter().filter(|l| !before.contains(l)).collect()
}

#[test]
fn a_dropped_tool_adds_its_element_locked_where_it_was_dropped() {
    let mut diagram = new_file();
    let outcome = diagram.execute(&ActionTarget::Canvas { x: 120.4, y: -40.6 }, "add-project");
    assert_eq!(adp_diagram::ActionOutcome::Done, outcome);
    let lines = lines(&diagram.text());
    assert_eq!("projects:", lines[1]);
    let id = lines[2].strip_prefix("  - id: ").unwrap().to_owned();
    assert!(is_short_guid(&id), "{id}");
    assert_eq!(
        vec![
            "    name: New project".to_owned(),
            "view:".to_owned(),
            "  placements:".to_owned(),
            format!("    - element: {id}"),
            "      x: 120".to_owned(),
            "      y: -41".to_owned(),
            String::new()
        ],
        lines[3..].to_vec()
    );
}

#[test]
fn a_redone_add_is_the_element_it_was_so_that_what_was_joined_to_it_comes_back_too() {
    let mut diagram = new_file();
    drop(&mut diagram, "project", 0.0, 0.0, "p1");
    drop(&mut diagram, "specification", 100.0, 0.0, "s1");
    diagram
        .run(AadEdit::Connect {
            from: "p1".into(),
            to: "s1".into(),
        })
        .unwrap();
    let joined = diagram.text();
    for _ in 0..3 {
        diagram.undo().unwrap();
    }
    let undone = diagram.text();
    for _ in 0..3 {
        diagram.redo().unwrap();
    }
    assert_eq!("agent-activity-diagram: 1\r\n", undone);
    assert_eq!(joined, diagram.text());
}

#[test]
fn an_added_task_goes_last_in_its_specification_stamped_now() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    diagram.execute(&common::element("s-knowledge"), "create-child");
    let after = diagram.text();
    let added = added(&before, &after);
    assert_eq!(4, added.len(), "{added:?}");
    assert!(
        is_short_guid(added[0].strip_prefix("      - id: ").unwrap()),
        "{}",
        added[0]
    );
    assert_eq!(
        vec!["        title: New task", "        status: pending"],
        added[1..3].to_vec()
    );
    assert!(
        is_moment(added[3].strip_prefix("        updated: ").unwrap()),
        "{}",
        added[3]
    );
    assert!(lines(&before).iter().all(|l| lines(&after).contains(l)));
    assert_eq!(3, diagram.model().element("s-knowledge").unwrap().rows.len());
}

#[test]
fn a_renamed_task_is_stamped_and_a_renamed_element_is_not() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    diagram
        .run(AadEdit::SetAttribute {
            element_id: "s-knowledge".into(),
            attribute: "name".into(),
            value: "Knowledge base designer".into(),
        })
        .unwrap();
    diagram
        .run(AadEdit::SetAttribute {
            element_id: "t-k2".into(),
            attribute: "title".into(),
            value: "Trial two bindings".into(),
        })
        .unwrap();
    let changed = added(&before, &diagram.text());
    assert_eq!(3, changed.len(), "{changed:?}");
    assert_eq!("    name: Knowledge base designer", changed[0]);
    assert_eq!("        title: Trial two bindings", changed[1]);
    assert!(changed[2].starts_with("        updated: "));
}

#[test]
fn a_status_chosen_by_its_label_is_written_in_its_stored_word() {
    let mut diagram = open_one_day();
    diagram
        .run(AadEdit::SetAttribute {
            element_id: "s-knowledge".into(),
            attribute: "status".into(),
            value: "Input Required".into(),
        })
        .unwrap();
    // The menu gives the member's name.
    let by_menu = diagram.execute(&common::element("t-k2"), "setTaskStatus:inputRequired");
    let refused = diagram.run(AadEdit::SetAttribute {
        element_id: "s-knowledge".into(),
        attribute: "status".into(),
        value: "Paused".into(),
    });
    assert_eq!(adp_diagram::ActionOutcome::Done, by_menu);
    let model = diagram.model();
    let specification = model.element("s-knowledge").unwrap();
    assert_eq!("inputRequired", specification.status);
    assert_eq!(
        "inputRequired",
        specification.rows.iter().find(|r| r.id == "t-k2").unwrap().status
    );
    // The specification, the task, and the task the file already had so.
    assert_eq!(
        3,
        lines(&diagram.text())
            .iter()
            .filter(|l| l.trim_start() == "status: input-required")
            .count()
    );
    assert_eq!(
        Err("'Paused' is not one of the values this can have.".to_owned()),
        refused
    );
}

#[test]
fn a_drawn_line_is_the_key_on_the_element_that_holds_it_whichever_way_it_was_drawn() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    diagram
        .run(AadEdit::Disconnect {
            relation_id: "project:s-rider".into(),
        })
        .unwrap();
    assert!(
        !diagram
            .text()
            .contains("    project: p-standalone\r\n    name: Rider warnings cleanup")
    );
    // Drawn from the specification to the project, the reverse of how the file reads it.
    let outcome = diagram.execute(
        &ActionTarget::Connection {
            source: Some("s-rider".into()),
            target: Some("p-adp".into()),
        },
        "connect",
    );
    assert_eq!(adp_diagram::ActionOutcome::Done, outcome);
    assert_eq!(
        before.replace(
            "    project: p-standalone\r\n    name: Rider warnings cleanup",
            "    project: p-adp\r\n    name: Rider warnings cleanup"
        ),
        diagram.text()
    );
}

#[test]
fn a_second_specification_for_an_agent_is_refused_and_so_is_a_pair_no_relation_joins() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    let second = diagram.run(AadEdit::Connect {
        from: "s-rider".into(),
        to: "a-dev2".into(),
    });
    let unrelated = diagram.run(AadEdit::Connect {
        from: "p-adp".into(),
        to: "e-fractal".into(),
    });
    let second = second.unwrap_err();
    assert!(second.contains("\"Developer 2\""), "{second}");
    assert!(second.contains("\"Knowledge designer\""), "{second}");
    assert_eq!(
        Err("A project cannot be joined to an environment. A project is joined to a specification.".to_owned()),
        unrelated
    );
    assert_eq!(before, diagram.text());
}

#[test]
fn a_removed_specification_takes_its_tasks_its_lock_and_its_groups_and_clears_the_key_that_named_it() {
    let mut diagram = open_one_day();
    diagram
        .run(AadEdit::Remove {
            element_id: "s-knowledge".into(),
        })
        .unwrap();
    let after = diagram.text();
    assert!(!after.contains("s-knowledge"), "{after}");
    assert!(!after.contains("t-k2"), "{after}");
    assert!(after.contains("  - id: a-dev2\r\n    name: Developer 2\r\n    link: https://claude.ai/code\r\n"));
    assert!(after.ends_with("view:\r\n  showArchived: false\r\n"), "{after}");
}

#[test]
fn a_drag_moves_the_lock_an_element_has_and_gives_one_to_an_element_without() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    diagram
        .session
        .move_elements(&mut diagram.workspace, &[("s-knowledge".into(), 300.0, -120.5)])
        .unwrap();
    diagram
        .session
        .move_elements(&mut diagram.workspace, &[("l-kd".into(), 520.0, 40.0)])
        .unwrap();
    let row = diagram
        .session
        .move_elements(&mut diagram.workspace, &[("t-k2".into(), 0.0, 0.0)]);
    // A row is not placed. -120.5 rounds half to even, as the standalone host's `Math.Round` does.
    assert_eq!(
        before.replace(
            "      x: 240\r\n      y: -80\r\n",
            "      x: 300\r\n      y: -120\r\n    - element: l-kd\r\n      x: 520\r\n      y: 40\r\n"
        ),
        diagram.text()
    );
    assert!(row.is_err());
}

#[test]
fn unlocking_removes_the_lock_and_the_last_one_takes_its_list_with_it() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    let none = diagram.run(AadEdit::Unpin {
        element_id: Some("l-kd".into()),
    });
    let all = diagram.run(AadEdit::Unpin { element_id: None });
    assert_eq!(Err("Its position is not locked.".to_owned()), none);
    assert_eq!(Ok(()), all);
    assert_eq!(
        before.replace(
            "  placements:\r\n    - element: s-knowledge\r\n      x: 240\r\n      y: -80\r\n",
            ""
        ),
        diagram.text()
    );
    assert!(diagram.model().placements.is_empty());
}

#[test]
fn a_groups_state_is_kept_only_while_it_differs_from_the_groups_default() {
    let mut diagram = open_one_day();
    let before = diagram.text();
    // Progressing comes unfolded, so folding it is kept...
    diagram.execute(&common::element("s-knowledge#progressing"), "collapse-group");
    let kept = diagram.text();
    // ...and putting both back as they come leaves no entry, and no list.
    diagram.execute(&common::element("s-knowledge#progressing"), "expand-group");
    diagram.execute(&common::element("s-knowledge#pending"), "collapse-group");
    assert!(
        kept.ends_with(
            "      collapsed: false\r\n    - element: s-knowledge\r\n      group: progressing\r\n      collapsed: true\r\n"
        ),
        "{kept}"
    );
    assert_eq!(
        before.replace(
            "  groups:\r\n    - element: s-knowledge\r\n      group: pending\r\n      collapsed: false\r\n",
            ""
        ),
        diagram.text()
    );
    assert_eq!(
        BTreeSet::from(["finished".to_owned(), "pending".to_owned()]),
        diagram.model().collapsed["s-knowledge"]
    );
}

#[test]
fn the_archived_switch_is_the_first_key_of_the_readers_part_and_shows_the_archived_specification() {
    let mut diagram = open(&one_day().replace("  showArchived: false\r\n", ""));
    let shown = |diagram: &Diagram| mapper::visible(diagram.document()).iter().any(|e| e.id == "s-rider");
    assert!(!shown(&diagram));
    let outcome = diagram.execute(&common::element("diagram view"), "show-archived");
    assert_eq!(adp_diagram::ActionOutcome::Done, outcome);
    assert!(
        diagram
            .text()
            .contains("view:\r\n  showArchived: true\r\n  placements:\r\n")
    );
    assert!(shown(&diagram));
}

#[test]
fn an_edit_is_applied_to_what_an_agent_wrote_since_the_file_was_read_and_undone_only_while_nobody_wrote_after_it() {
    let mut diagram = open_one_day();
    let agents = one_day().replace("    name: etalii.adp\r\n", "    name: etalii.adp (languages)\r\n");
    std::fs::write(&diagram.body, &agents).unwrap();
    let rename = || AadEdit::SetAttribute {
        element_id: "a-dev2".into(),
        attribute: "name".into(),
        value: "Developer two".into(),
    };
    // An edit made in the diagram, which has not seen the agent's write: both changes are in the file.
    diagram.run(rename()).unwrap();
    let both = agents.replace("    name: Developer 2\r\n", "    name: Developer two\r\n");
    assert_eq!(both, diagram.text());
    // Undo restores what the edit was applied to: the agent's text, not the diagram's older reading.
    diagram.undo().unwrap();
    assert_eq!(agents, diagram.text());
    // And once an agent has written after an edit, that edit is no longer undone over the agent's write.
    diagram.run(rename()).unwrap();
    let later = both.replace("    name: Fractal\r\n", "    name: Fractal 2\r\n");
    std::fs::write(&diagram.body, &later).unwrap();
    assert!(diagram.undo().is_err());
    assert_eq!(later, diagram.text());
}

#[test]
fn an_edit_of_an_element_that_is_gone_is_refused() {
    let mut diagram = open_one_day();
    let refused = diagram.run(AadEdit::SetAttribute {
        element_id: "p-gone".into(),
        attribute: "name".into(),
        value: "The standalone host".into(),
    });
    assert_eq!(Err("That is no longer in this diagram.".to_owned()), refused);
    assert_eq!(one_day(), diagram.text());
}

//! Every edit of a trigger or a note (the standalone host's `GhgTriggersAndNotesEditing.Tests.cs` at `8b6ccfc`,
//! ghg-triggers-and-notes task 6): its undo gives back the original bytes, a refusal writes nothing, and the toolbox,
//! menus and grid answer for both types, on the triggers-and-notes fixture, a diagram drawn in years.

mod common;

use adp_diagram::ResizedEdge;
use adp_diagram_hype_cycle::reading::{Note, Trigger};
use adp_diagram_hype_cycle::scale::{self, TimeUnit};
use common::{Graph, canvas, connection, element, fixture, m, open};

fn graph() -> Graph {
    open(&fixture("triggers-and-notes"))
}

/// The x of the start of `year`: the fixture is drawn in years.
fn x(year: i64) -> f64 {
    common::x(year, TimeUnit::Year)
}

fn edit(graph: &mut Graph, name: &str) -> Result<(), String> {
    match name {
        "add a trigger" => graph.run(
            &canvas(x(1970) + 1.0, scale::top_of(6) + 10.0),
            "ghg.add.trigger",
            &["t2"],
        ),
        "add a note" => graph.run(&canvas(x(1970) + 1.0, scale::top_of(6) + 10.0), "ghg.add.note", &["n3"]),
        // The desktop host's canvas hands a trigger's centre.
        "move a trigger" => graph.move_to("transistor-invented", x(1949), scale::top_of(2) + 16.0),
        "move a note" => graph.move_to("note-2", x(1965), scale::top_of(6)),
        "rename a trigger" => graph.run_answered(
            &element("transistor-invented"),
            "ghg.rename",
            "Point-contact transistor",
        ),
        "edit a note's text" => graph.run_answered(&element("note-2"), "ghg.rename", "Now\non two lines"),
        "resize a note" => graph.set("note-2", "ghg.size", "200 x 48"),
        "resize a note from the left" => {
            graph.resize("note-2", ResizedEdge::Left, x(1950), scale::top_of(5), 160.0, 40.0)
        }
        "resize a note from the top" => {
            graph.resize("note-2", ResizedEdge::Top, x(1960), scale::top_of(4), 120.0, 96.0)
        }
        "date a trigger" => graph.set("transistor-invented", "ghg.date", "1947-11"),
        "tag a trigger" => graph.set("transistor-invented", "ghg.tags", "electronics, physics"),
        "describe a trigger" => graph.set("transistor-invented", "ghg.description", ""),
        "remove a trigger" => graph.run(&element("transistor-invented"), "ghg.remove", &[]),
        "remove a note" => graph.run(&element("note-1"), "ghg.remove", &[]),
        "influence from a trigger" => graph.run(
            &connection("transistor-invented", "radio"),
            "ghg.connect.influence",
            &["i-new"],
        ),
        _ => panic!("no such edit: {name}"),
    }
}

#[test]
fn every_edit_then_its_undo_gives_the_original_bytes() {
    for name in [
        "add a trigger",
        "add a note",
        "move a trigger",
        "move a note",
        "rename a trigger",
        "edit a note's text",
        "resize a note",
        "resize a note from the left",
        "resize a note from the top",
        "date a trigger",
        "tag a trigger",
        "describe a trigger",
        "remove a trigger",
        "remove a note",
        "influence from a trigger",
    ] {
        let mut graph = graph();
        let original = graph.bytes();
        edit(&mut graph, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_ne!(graph.bytes(), original, "{name} edited nothing");
        assert!(
            graph.session.problems(graph.workspace.documents()).is_empty(),
            "{name}: {:?}",
            graph.session.problems(graph.workspace.documents())
        );
        graph.undo();
        assert_eq!(graph.bytes(), original, "{name}");
    }
}

fn refusal(graph: &mut Graph, name: &str) -> Result<(), String> {
    match name {
        "influence into a trigger" => graph.run(
            &connection("transistors", "transistor-invented"),
            "ghg.connect.influence",
            &["x"],
        ),
        "influence a trigger into a trigger" => graph.run(
            &connection("transistor-invented", "transistor-invented"),
            "ghg.connect.influence",
            &["x"],
        ),
        "a second influence from a trigger" => graph.run(
            &connection("transistor-invented", "transistors"),
            "ghg.connect.influence",
            &["x"],
        ),
        "a trigger date that is not one" => graph.set("transistor-invented", "ghg.date", "soon"),
        "a size that is not one" => graph.set("note-1", "ghg.size", "wide"),
        "rename a trigger to nothing" => graph.run_answered(&element("transistor-invented"), "ghg.rename", " "),
        "remove a note that is not there" => graph.run(&element("nothing-here"), "ghg.remove", &[]),
        _ => panic!("no such refusal: {name}"),
    }
}

#[test]
fn every_refusal_writes_nothing_and_says_why() {
    for (name, saying) in [
        ("influence into a trigger", "An influence cannot end at a trigger."),
        (
            "influence a trigger into a trigger",
            "An influence cannot end at a trigger.",
        ),
        ("a second influence from a trigger", "already influences that one"),
        ("a trigger date that is not one", "is not a date"),
        ("a size that is not one", "is not a size"),
        ("rename a trigger to nothing", "needs a name"),
        ("remove a note that is not there", ""),
    ] {
        let mut graph = graph();
        let original = graph.bytes();
        let refused = refusal(&mut graph, name).expect_err(name);
        assert!(refused.contains(saying), "{name}: {refused}");
        assert_eq!(graph.bytes(), original, "{name}");
    }
}

/// A trigger takes its influences with it, and one undo restores them.
#[test]
fn removing_a_trigger_takes_its_influences_and_one_undo_restores_them() {
    let mut graph = graph();
    let from_it = |graph: &Graph| {
        graph
            .diagram()
            .relations_of_type("Influence")
            .iter()
            .any(|i| i.source_id().as_deref() == Some("transistor-invented"))
    };
    assert!(from_it(&graph));
    graph.run(&element("transistor-invented"), "ghg.remove", &[]).unwrap();
    assert!(!from_it(&graph));
    graph.undo();
    assert!(from_it(&graph));
}

/// A trigger's centre lands on a step line and a row's middle.
#[test]
fn a_moved_trigger_lands_with_its_centre_on_a_step_line_and_a_rows_middle() {
    let mut graph = graph();
    // A little right of 1949's line and a little below row 2's middle.
    graph
        .move_to("transistor-invented", x(1949) + 1.0, scale::top_of(2) + 16.0 + 3.0)
        .unwrap();
    let trigger = Trigger::of(&graph.diagram().element_by_id("transistor-invented").unwrap());
    assert_eq!((trigger.date, trigger.row), (Some(m(1949, 1)), 2));
}

#[test]
fn a_dropped_note_is_empty_and_default_sized_with_the_drop_inside_it() {
    let mut graph = graph();
    graph
        .run(&canvas(x(1970) + 1.0, scale::top_of(6) + 10.0), "ghg.add.note", &["n3"])
        .unwrap();
    let element = graph.diagram().element_by_id("n3").unwrap();
    let note = Note::of(&element);
    assert_eq!(
        (note.at, note.row, note.width, note.height),
        (Some(m(1970, 1)), 6, Some(160.0), Some(64.0))
    );
    assert_eq!(
        adp_diagram_hype_cycle::reading::text(&element, "text").as_deref(),
        Some("")
    );
}

/// The fixture's trigger is drawn at its centre, its date's start and its row's middle, and a note at its top-left.
#[test]
fn a_trigger_is_drawn_at_its_centre_and_a_note_at_its_top_left() {
    let graph = graph();
    assert_eq!(
        graph.position("transistor-invented"),
        (
            x(1947) + scale::width_of(11, TimeUnit::Year),
            scale::top_of(1) + scale::TREND_HEIGHT / 2.0
        )
    );
    assert_eq!(graph.position("note-2"), (x(1960), scale::top_of(5)));
}

#[test]
fn the_toolbox_offers_a_trend_a_trigger_and_a_note() {
    let graph = graph();
    let toolbox: Vec<String> = graph
        .session
        .toolbox(graph.workspace.documents())
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert_eq!(toolbox, ["Trend", "Trigger", "Note"]);
}

#[test]
fn a_triggers_rows_are_its_name_date_tags_and_description_and_a_notes_its_text_and_size() {
    let graph = graph();
    let trigger: Vec<String> = graph
        .session
        .properties(graph.workspace.documents(), &["transistor-invented"])
        .into_iter()
        .map(|row| row.id)
        .collect();
    assert_eq!(trigger, ["ghg.name", "ghg.description", "ghg.tags", "ghg.date"]);
    assert_eq!(graph.row("transistor-invented", "ghg.date").as_deref(), Some("1947-12"));
    assert_eq!(
        graph.row("note-1", "ghg.text").as_deref(),
        Some("Dates are illustrative.\n\nSee the readme.")
    );
    assert_eq!(graph.row("note-1", "ghg.size").as_deref(), Some("160 x 64"));
}

#[test]
fn a_triggers_date_set_in_the_grid_is_one_undo_away() {
    let mut graph = graph();
    let original = graph.bytes();
    graph.set("transistor-invented", "ghg.date", "1948-06").unwrap();
    let trigger = Trigger::of(&graph.diagram().element_by_id("transistor-invented").unwrap());
    assert_eq!(trigger.date, Some(m(1948, 6)));
    graph.undo();
    assert_eq!(graph.bytes(), original);
}

#[test]
fn a_trigger_and_a_note_offer_rename_and_remove_and_the_canvas_offers_all_three_adds() {
    let graph = graph();
    let ids = |target| -> Vec<String> {
        graph
            .session
            .actions(graph.workspace.documents(), &target)
            .into_iter()
            .flat_map(|group| group.actions)
            .map(|action| action.id)
            .collect()
    };
    assert_eq!(
        ids(element("transistor-invented")),
        ["ghg.rename", "ghg.remove", "ghg.arrange"]
    );
    assert_eq!(ids(element("note-1")), ["ghg.rename", "ghg.remove", "ghg.arrange"]);
    assert_eq!(
        ids(canvas(10.0, 10.0)),
        ["ghg.add.trend", "ghg.add.trigger", "ghg.add.note", "ghg.arrange"]
    );
}

/// An influence from a trigger states no from end: a trigger has no phases.
#[test]
fn an_influence_from_a_trigger_states_no_from_end() {
    let mut graph = graph();
    graph
        .run(
            &connection("transistor-invented", "radio"),
            "ghg.connect.influence",
            &["i-new"],
        )
        .unwrap();
    assert!(
        graph.text().ends_with("  - id: i-new\n    from: transistor-invented\n    to: radio\n    to-phase: peak\n    to-edge: top\n    to-at: 0.5\n"),
        "{}",
        graph.text()
    );
}

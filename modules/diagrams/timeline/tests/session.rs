//! A timeline edited through the desktop session with the timeline's module: the standalone host's
//! `TimelineCommandsTests`, `TimelineContextActionProviderTests` (with `TimelineActionRealityTests` and
//! `TimelineRelationGestureTests`), `TimelineContextPropertyProviderTests`, `TimelineSessionTests` and
//! `TimelineElementMapperTests` at `8b6ccfc`, where they apply: the standalone module ran its own commands, the
//! desktop host runs the specification's menus, forms and operations through `adp-diagram`, so each test here asks
//! the session what the standalone test asked the module's provider or command, with the same expectations.
//!
//! Not ported: what the session does for every tool type alike and `adp-diagram` tests (the store, the reload, the
//! viewport deltas, the payloads of the gRPC contract), and a relation gesture released on empty canvas, which the
//! desktop session's `ActionTarget::Connection` carries no point for.

mod common;

use adp_context::Prompt;
use adp_diagram::{ActionOutcome, ActionTarget};
use adp_diagram_timeline::arrangement;
use adp_diagram_timeline::instants;
use adp_diagram_timeline::reading::TimelineElement;
use adp_diagram_timeline::scale;
use common::{Timeline, element, open};

const TIMELINE: &str = "timeline: 1\r\n# A comment the commands must never disturb.\r\nelements:\r\n  - id: aaa\r\n    label: Period\r\n    begin: 2026-01-05\r\n    end: 2026-02-13\r\n    row: 0\r\n  - id: bbb\r\n    label: Moment\r\n    begin: 2026-02-16T14:00:00\r\n    row: 2\r\nconnections:\r\n  - id: ccc\r\n    from: aaa\r\n    to: bbb\r\n    label: gates\r\n";

/// The relation gestures' timeline: two elements and no `connections:` key.
const UNRELATED: &str = "timeline: 1\r\nelements:\r\n  - id: aaa\r\n    label: First\r\n    begin: 2026-01-01\r\n    end: 2026-01-20\r\n    row: 0\r\n  - id: bbb\r\n    label: Second\r\n    begin: 2026-02-01\r\n    row: 1\r\n";

fn elements(timeline: &Timeline) -> Vec<TimelineElement> {
    arrangement::read(
        &timeline
            .workspace
            .documents()
            .get(&timeline.body)
            .unwrap()
            .model()
            .diagram,
    )
    .0
}

fn links(timeline: &Timeline) -> Vec<(String, String)> {
    arrangement::read(
        &timeline
            .workspace
            .documents()
            .get(&timeline.body)
            .unwrap()
            .model()
            .diagram,
    )
    .1
}

fn find(timeline: &Timeline, id: &str) -> TimelineElement {
    elements(timeline).into_iter().find(|e| e.id == id).unwrap()
}

fn action_ids(timeline: &Timeline, target: &ActionTarget) -> Vec<String> {
    timeline
        .session
        .actions(timeline.workspace.documents(), target)
        .into_iter()
        .flat_map(|group| group.actions.into_iter().map(|a| a.id))
        .collect()
}

fn refused(outcome: &ActionOutcome, part: &str) -> bool {
    matches!(outcome, ActionOutcome::Refused(reason) if reason.contains(part))
}

fn seconds(date: &str) -> f64 {
    scale::to_seconds(instants::parse(date).unwrap())
}

// ---- commands ----------------------------------------------------------------------------------------------------

#[test]
fn add_then_undo_is_byte_identical() {
    let mut timeline = open(TIMELINE);

    let outcome = timeline.execute(
        &ActionTarget::Canvas {
            x: seconds("2026-03-01"),
            y: scale::to_y(4),
        },
        "timeline.add-element",
        &["new00001"],
    );
    let added = find(&timeline, "new00001");
    timeline.undo();

    assert_eq!(ActionOutcome::Done, outcome);
    assert_eq!(
        ("2026-03-01", Some("2026-03-15"), 4),
        (added.begin.as_str(), added.end.as_deref(), added.row)
    );
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn adding_a_moment_writes_no_end_key() {
    let mut timeline = open(TIMELINE);

    let outcome = timeline.execute(
        &ActionTarget::Canvas {
            x: seconds("2026-06-01"),
            y: scale::to_y(1),
        },
        "timeline.add-moment",
        &["mmm00001"],
    );

    assert_eq!(ActionOutcome::Done, outcome);
    assert_eq!(None, find(&timeline, "mmm00001").end);
}

#[test]
fn remove_takes_the_connections_and_undo_puts_everything_back_byte_for_byte() {
    let mut timeline = open(TIMELINE);

    let asked = timeline.execute(&element("aaa"), "timeline.remove", &[]);
    let removed = timeline.answer(&element("aaa"), "timeline.remove", "");
    let (after_elements, after_links) = (elements(&timeline), links(&timeline));
    timeline.undo();

    assert!(
        matches!(&asked, ActionOutcome::Prompt(Prompt::Confirm { message, .. }) if message.contains("1 relation")),
        "{asked:?}"
    );
    assert_eq!(ActionOutcome::Done, removed);
    assert!(after_links.is_empty());
    assert_eq!(1, after_elements.len());
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn remove_undo_redo_removes_it_again() {
    let mut timeline = open(TIMELINE);
    timeline.answer(&element("aaa"), "timeline.remove", "");

    timeline.undo();
    timeline.redo();

    assert!(elements(&timeline).iter().all(|e| e.id != "aaa"));
}

#[test]
fn removing_the_last_element_of_a_file_with_no_trailing_newline_undoes_byte_for_byte() {
    let unterminated = "timeline: 1\r\nelements:\r\n  - id: last\r\n    begin: 2026-01-01\r\n    row: 0";
    let mut timeline = open(unterminated);

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&element("last"), "timeline.remove", &[])
    );
    timeline.undo();

    assert_eq!(unterminated, timeline.text());
}

#[test]
fn rename_changes_one_line_leaves_the_id_and_undoes() {
    let mut timeline = open(TIMELINE);

    let asked = timeline.execute(&element("aaa"), "timeline.rename", &[]);
    let renamed = timeline.answer(&element("aaa"), "timeline.rename", "Renamed");
    let text = timeline.text();
    timeline.undo();

    assert!(
        matches!(&asked, ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) if initial_value == "Period"),
        "{asked:?}"
    );
    assert_eq!(ActionOutcome::Done, renamed);
    assert_eq!(TIMELINE.replace("label: Period", "label: Renamed"), text);
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn a_placement_that_would_invert_is_refused() {
    let mut timeline = open(TIMELINE);

    let refused = timeline.set("aaa", "timeline.begin", "2026-03-01");

    assert_eq!(Err("An element cannot end before it begins.".to_owned()), refused);
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn a_placement_never_gives_a_moment_an_end() {
    let mut timeline = open(TIMELINE);

    let moved = timeline.move_to("bbb", seconds("2026-05-01T09:00:00"), scale::to_y(2));

    assert_eq!(Ok(()), moved);
    let bbb = find(&timeline, "bbb");
    assert_eq!(("2026-05-01T09:00:00", None), (bbb.begin.as_str(), bbb.end.as_deref()));
}

#[test]
fn connect_then_undo_is_byte_identical() {
    let mut timeline = open(TIMELINE);
    let gesture = ActionTarget::Connection {
        source: Some("bbb".into()),
        target: Some("aaa".into()),
    };

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&gesture, "timeline.connect", &["ddd"])
    );
    timeline.undo();

    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn a_self_connection_is_refused_with_the_reason() {
    let mut timeline = open(TIMELINE);
    let gesture = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: Some("aaa".into()),
    };

    let outcome = timeline.execute(&gesture, "timeline.connect", &["ddd"]);

    assert!(refused(&outcome, "cannot be connected to itself"), "{outcome:?}");
}

#[test]
fn a_second_connection_between_the_same_pair_is_permitted() {
    let mut timeline = open(TIMELINE);
    let gesture = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: Some("bbb".into()),
    };

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&gesture, "timeline.connect", &["ddd"])
    );

    assert_eq!(2, links(&timeline).len());
}

#[test]
fn a_connection_to_an_absent_element_is_refused() {
    let mut timeline = open(TIMELINE);
    let gesture = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: Some("ghost".into()),
    };

    let outcome = timeline.execute(&gesture, "timeline.connect", &["ddd"]);

    assert!(matches!(outcome, ActionOutcome::Refused(_)), "{outcome:?}");
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn disconnect_then_undo_brings_the_connection_back() {
    let mut timeline = open(TIMELINE);

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&element("ccc"), "timeline.disconnect", &[])
    );
    let after = links(&timeline).len();
    timeline.undo();

    assert_eq!(0, after);
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn relabel_then_undo_is_byte_identical() {
    let mut timeline = open(TIMELINE);

    assert_eq!(
        ActionOutcome::Done,
        timeline.answer(&element("ccc"), "timeline.relabel", "renamed")
    );
    let text = timeline.text();
    timeline.undo();

    assert_eq!(TIMELINE.replace("label: gates", "label: renamed"), text);
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn no_command_touches_the_comment() {
    let mut timeline = open(TIMELINE);

    timeline.execute(
        &ActionTarget::Canvas {
            x: seconds("2026-03-01"),
            y: scale::to_y(4),
        },
        "timeline.add-moment",
        &["new00001"],
    );
    timeline.answer(&element("aaa"), "timeline.rename", "Renamed");
    timeline.execute(
        &ActionTarget::Connection {
            source: Some("new00001".into()),
            target: Some("aaa".into()),
        },
        "timeline.connect",
        &["ddd"],
    );
    timeline.answer(&element("new00001"), "timeline.remove", "");

    assert!(
        timeline
            .text()
            .contains("# A comment the commands must never disturb.\r\n")
    );
}

#[test]
fn a_given_end_lands_after_begin() {
    let mut timeline = open(TIMELINE);

    let given = timeline.answer(&element("bbb"), "timeline.give-end", "2026-03-01T09:00:00");

    assert_eq!(ActionOutcome::Done, given);
    assert_eq!(
        TIMELINE.replace(
            "    begin: 2026-02-16T14:00:00\r\n",
            "    begin: 2026-02-16T14:00:00\r\n    end: 2026-03-01T09:00:00\r\n"
        ),
        timeline.text()
    );
}

#[test]
fn the_first_relation_creates_the_connections_key_at_the_end_of_the_document_and_its_undo_removes_it() {
    let mut timeline = open(UNRELATED);
    let gesture = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: Some("bbb".into()),
    };

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&gesture, "timeline.connect", &["ddd"])
    );
    let related = timeline.text();
    timeline.undo();

    assert_eq!(
        format!("{UNRELATED}connections:\r\n  - id: ddd\r\n    from: aaa\r\n    to: bbb\r\n"),
        related
    );
    assert_eq!(UNRELATED, timeline.text());
}

#[test]
fn the_first_element_of_an_empty_timeline_opens_its_empty_sequence() {
    let mut timeline = open("timeline: 1\r\nelements: []\r\n");

    let outcome = timeline.execute(
        &ActionTarget::Canvas {
            x: seconds("2026-07-01"),
            y: scale::to_y(0),
        },
        "timeline.add-moment",
        &["first"],
    );

    assert_eq!(ActionOutcome::Done, outcome);
    assert_eq!(
        "timeline: 1\r\nelements:\r\n  - id: first\r\n    label: New moment\r\n    begin: 2026-07-01\r\n    row: 0\r\n",
        timeline.text()
    );
}

// ---- actions -----------------------------------------------------------------------------------------------------

#[test]
fn an_element_with_an_end_offers_its_edits_and_its_additions() {
    let timeline = open(TIMELINE);

    assert_eq!(
        vec![
            "timeline.rename",
            "timeline.remove-end",
            "timeline.remove",
            "timeline.add-after",
            "timeline.add-below",
            "timeline.arrange"
        ],
        action_ids(&timeline, &element("aaa"))
    );
}

#[test]
fn a_moment_offers_give_end_instead_of_remove_end() {
    let timeline = open(TIMELINE);

    let ids = action_ids(&timeline, &element("bbb"));

    assert!(ids.iter().any(|id| id == "timeline.give-end"));
    assert!(!ids.iter().any(|id| id == "timeline.remove-end"));
}

#[test]
fn a_connection_offers_relabel_and_disconnect() {
    let timeline = open(TIMELINE);

    assert_eq!(
        vec!["timeline.relabel", "timeline.disconnect", "timeline.arrange"],
        action_ids(&timeline, &element("ccc"))
    );
}

#[test]
fn give_end_is_refused_as_typed_on_the_handlers_own_terms() {
    let mut timeline = open(TIMELINE);

    let inverted = timeline.answer(&element("bbb"), "timeline.give-end", "2026-01-01");
    let unreadable = timeline.answer(&element("bbb"), "timeline.give-end", "sideways");

    assert!(refused(&inverted, "cannot end before it begins"), "{inverted:?}");
    assert!(refused(&unreadable, "'sideways' is not a time"), "{unreadable:?}");
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn give_end_committed_makes_the_moment_a_period_and_undo_makes_it_a_moment_again() {
    let mut timeline = open(TIMELINE);

    let asked = timeline.execute(&element("bbb"), "timeline.give-end", &[]);
    assert_eq!(
        ActionOutcome::Done,
        timeline.answer(&element("bbb"), "timeline.give-end", "2026-03-01")
    );
    let after = find(&timeline, "bbb").end;
    timeline.undo();

    assert!(
        matches!(&asked, ActionOutcome::Prompt(Prompt::Input { initial_value, label, .. })
            if initial_value == "2026-02-16T14:00:00" && label == "End"),
        "{asked:?}"
    );
    assert_eq!(Some("2026-03-01".to_owned()), after);
    assert_eq!(None, find(&timeline, "bbb").end);
}

#[test]
fn a_drop_on_a_placement_lands_there_at_once() {
    let mut timeline = open(TIMELINE);

    let outcome = timeline.execute(
        &ActionTarget::Canvas {
            x: seconds("2026-07-01"),
            y: scale::to_y(5),
        },
        "timeline.add-moment",
        &["dropped"],
    );

    assert_eq!(ActionOutcome::Done, outcome);
    let added = find(&timeline, "dropped");
    assert_eq!(
        ("New moment", "2026-07-01", None, 5),
        (
            added.label.as_str(),
            added.begin.as_str(),
            added.end.as_deref(),
            added.row
        )
    );
}

#[test]
fn an_action_on_the_wrong_selection_reports_it_does_not_apply() {
    let mut timeline = open(TIMELINE);

    let outcome = timeline.answer(&element("ccc"), "timeline.give-end", "2026-03-01");

    assert!(refused(&outcome, "does not apply"), "{outcome:?}");
}

#[test]
fn a_placement_discovers_its_actions_or_no_drop_could_ever_resolve() {
    let timeline = open(TIMELINE);

    let ids = action_ids(&timeline, &ActionTarget::Canvas { x: 0.0, y: 0.0 });

    for id in ["timeline.add-element", "timeline.add-moment", "timeline.arrange"] {
        assert!(ids.iter().any(|i| i == id), "{id} in {ids:?}");
    }
}

#[test]
fn arrange_from_the_background_menu_moves_rows() {
    let lone = "timeline: 1\r\nelements:\r\n  - id: lone\r\n    label: Lone\r\n    begin: 2026-01-05\r\n    end: 2026-02-13\r\n    row: 3\r\n";
    let mut timeline = open(lone);

    let outcome = timeline.execute(&ActionTarget::Canvas { x: 0.0, y: 0.0 }, "timeline.arrange", &[]);

    assert_eq!(ActionOutcome::Done, outcome);
    assert_eq!(vec![0], elements(&timeline).iter().map(|e| e.row).collect::<Vec<_>>());
}

#[test]
fn removing_a_relation_free_element_happens_at_once() {
    let lone = "timeline: 1\r\nelements:\r\n  - id: lone\r\n    label: Lone\r\n    begin: 2026-01-05\r\n    end: 2026-02-13\r\n    row: 0\r\n";
    let mut timeline = open(lone);

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&element("lone"), "timeline.remove", &[])
    );

    assert!(elements(&timeline).is_empty());
}

#[test]
fn removing_an_end_happens_at_once() {
    let lone = "timeline: 1\r\nelements:\r\n  - id: lone\r\n    label: Lone\r\n    begin: 2026-01-05\r\n    end: 2026-02-13\r\n    row: 0\r\n";
    let mut timeline = open(lone);

    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&element("lone"), "timeline.remove-end", &[])
    );

    assert_eq!(None, find(&timeline, "lone").end);
}

#[test]
fn one_call_relates_two_elements_in_the_stated_direction() {
    let mut timeline = open(UNRELATED);
    let gesture = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: Some("bbb".into()),
    };

    assert!(
        action_ids(&timeline, &gesture)
            .iter()
            .any(|id| id == "timeline.connect")
    );
    assert_eq!(
        ActionOutcome::Done,
        timeline.execute(&gesture, "timeline.connect", &["ddd"])
    );

    assert_eq!(vec![("aaa".to_owned(), "bbb".to_owned())], links(&timeline));
}

#[test]
fn tab_grows_a_related_element_six_days_after_two_weeks_long() {
    let mut timeline = open(UNRELATED);

    let outcome = timeline.execute(&element("aaa"), "timeline.add-after", &["grown", "link"]);

    assert_eq!(ActionOutcome::Done, outcome);
    let added = find(&timeline, "grown");
    assert_eq!(
        ("New element", "2026-01-26", Some("2026-02-09"), 0),
        (
            added.label.as_str(),
            added.begin.as_str(),
            added.end.as_deref(),
            added.row
        )
    );
    assert_eq!(vec![("aaa".to_owned(), "grown".to_owned())], links(&timeline));
}

#[test]
fn enter_grows_a_related_element_on_the_next_row() {
    let mut timeline = open(UNRELATED);

    let outcome = timeline.execute(&element("aaa"), "timeline.add-below", &["grown", "link"]);

    assert_eq!(ActionOutcome::Done, outcome);
    let added = find(&timeline, "grown");
    assert_eq!(("2026-01-01", 1), (added.begin.as_str(), added.row));
    assert_eq!(vec![("aaa".to_owned(), "grown".to_owned())], links(&timeline));
}

// ---- properties --------------------------------------------------------------------------------------------------

fn row_ids(timeline: &Timeline, id: &str) -> Vec<String> {
    timeline
        .session
        .properties(timeline.workspace.documents(), &[id])
        .into_iter()
        .map(|r| r.id)
        .collect()
}

#[test]
fn a_period_contributes_label_begin_end_and_row() {
    let timeline = open(TIMELINE);

    let rows = timeline.session.properties(timeline.workspace.documents(), &["aaa"]);

    assert_eq!(
        vec!["timeline.label", "timeline.begin", "timeline.end", "timeline.row"],
        rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
    );
    assert!(rows.iter().all(|r| r.read_only_reason.is_empty()));
}

#[test]
fn a_moment_contributes_no_end_row_at_all() {
    let timeline = open(TIMELINE);

    assert!(!row_ids(&timeline, "bbb").iter().any(|id| id == "timeline.end"));
}

#[test]
fn a_connection_shows_its_endpoints_read_only_with_the_reason() {
    let timeline = open(TIMELINE);

    let rows = timeline.session.properties(timeline.workspace.documents(), &["ccc"]);

    let from = rows.iter().find(|r| r.id == "timeline.from").unwrap();
    let to = rows.iter().find(|r| r.id == "timeline.to").unwrap();
    assert_eq!(("aaa", "bbb"), (from.value.as_str(), to.value.as_str()));
    assert!(from.read_only_reason.contains("canvas"), "{}", from.read_only_reason);
    assert!(!to.read_only_reason.is_empty());
    assert!(
        rows.iter()
            .find(|r| r.id == "timeline.label")
            .unwrap()
            .read_only_reason
            .is_empty()
    );
}

#[test]
fn an_edit_is_one_undo_away() {
    let mut timeline = open(TIMELINE);

    assert_eq!(Ok(()), timeline.set("aaa", "timeline.begin", "2026-01-10"));
    timeline.undo();

    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn a_malformed_time_is_refused_naming_the_value() {
    let mut timeline = open(TIMELINE);

    let refused = timeline.set("aaa", "timeline.begin", "sideways").unwrap_err();

    assert!(refused.contains("sideways"), "{refused}");
}

#[test]
fn mixing_precision_within_one_element_is_refused() {
    let mut timeline = open(TIMELINE);

    let refused = timeline
        .set("aaa", "timeline.begin", "2026-01-05T09:00:00")
        .unwrap_err();

    assert!(refused.contains("both"), "{refused}");
}

#[test]
fn editing_the_row_moves_the_element() {
    let mut timeline = open(TIMELINE);

    assert_eq!(Ok(()), timeline.set("aaa", "timeline.row", "7"));

    assert_eq!(7, find(&timeline, "aaa").row);
    assert_eq!((seconds("2026-01-05"), scale::to_y(7)), timeline.position("aaa"));
}

#[test]
fn an_unknown_property_is_refused() {
    let mut timeline = open(TIMELINE);

    assert!(timeline.set("aaa", "timeline.nonsense", "x").is_err());
}

// ---- the session and the mapper ----------------------------------------------------------------------------------

#[test]
fn an_elements_position_is_its_begin_and_its_row() {
    let timeline = open(TIMELINE);

    assert_eq!((seconds("2026-01-05"), 0.0), timeline.position("aaa"));
    assert_eq!((seconds("2026-02-16T14:00:00"), 120.0), timeline.position("bbb"));
}

#[test]
fn an_element_with_an_unreadable_begin_is_still_delivered_at_the_epoch() {
    let timeline = open("timeline: 1\r\nelements:\r\n  - id: odd\r\n    begin: someday\r\n    row: 1\r\n");

    assert_eq!((0.0, 60.0), timeline.position("odd"));
}

#[test]
fn the_reparenting_move_is_refused_with_a_reason() {
    let mut timeline = open(TIMELINE);

    let refused = timeline
        .session
        .move_element(&mut timeline.workspace, "aaa", Some("bbb"), 0);

    assert!(refused.is_err());
    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn a_positional_move_rewrites_the_element_and_preserves_its_duration() {
    let mut timeline = open(TIMELINE);

    assert_eq!(Ok(()), timeline.move_to("aaa", seconds("2026-03-01"), scale::to_y(4)));

    let text = timeline.text();
    assert!(text.contains("begin: 2026-03-01"), "{text}");
    assert!(text.contains("end: 2026-04-09"), "{text}");
    assert!(text.contains("row: 4"), "{text}");
}

#[test]
fn a_move_is_one_undo_away() {
    let mut timeline = open(TIMELINE);

    timeline.move_to("aaa", seconds("2026-03-01"), scale::to_y(4)).unwrap();
    timeline.undo();

    assert_eq!(TIMELINE, timeline.text());
}

#[test]
fn moving_a_connection_is_refused() {
    let mut timeline = open(TIMELINE);

    assert!(timeline.move_to("ccc", 0.0, 0.0).is_err());
}

#[test]
fn a_dragged_date_only_element_lands_on_a_date() {
    let mut timeline = open(TIMELINE);

    timeline
        .move_to("aaa", seconds("2026-03-01") + 3.7 * 3600.0, scale::to_y(0))
        .unwrap();

    let aaa = find(&timeline, "aaa");
    assert_eq!(
        ("2026-03-01", Some("2026-04-09")),
        (aaa.begin.as_str(), aaa.end.as_deref())
    );
}

#[test]
fn a_dragged_period_with_an_unreadable_end_moves_only_its_begin() {
    let mut timeline =
        open("timeline: 1\r\nelements:\r\n  - id: odd\r\n    begin: 2026-01-01\r\n    end: later\r\n    row: 0\r\n");

    timeline.move_to("odd", seconds("2026-02-01"), 0.0).unwrap();

    assert_eq!(
        "timeline: 1\r\nelements:\r\n  - id: odd\r\n    begin: 2026-02-01\r\n    end: later\r\n    row: 0\r\n",
        timeline.text()
    );
}

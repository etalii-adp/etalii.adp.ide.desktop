//! The context's behaviour (etalii.adp spec 030, FR-015, FR-017, task T016).

use adp_context::*;

fn entry(path: &str) -> Reference {
    Reference::Entry(path.into())
}

#[test]
fn replace_selects_exactly_the_references_once_each() {
    let mut selection = Selection::default();
    assert!(selection.select(
        vec![entry("a"), entry("b"), entry("a")],
        SelectMode::Replace,
        SelectionSource::Explorer
    ));
    assert_eq!(selection.references(), [entry("a"), entry("b")]);
    assert_eq!(selection.primary(), Some(&entry("b")));
    assert_eq!(selection.source(), Some(SelectionSource::Explorer));
}

#[test]
fn toggle_adds_what_is_not_selected_and_removes_what_is() {
    let mut selection = Selection::default();
    selection.select(
        vec![entry("a"), entry("b")],
        SelectMode::Replace,
        SelectionSource::Canvas,
    );
    selection.select(
        vec![entry("a"), entry("c")],
        SelectMode::Toggle,
        SelectionSource::Canvas,
    );
    assert_eq!(selection.references(), [entry("b"), entry("c")]);
}

#[test]
fn an_unchanged_selection_reports_no_change() {
    let mut selection = Selection::default();
    selection.select(vec![entry("a")], SelectMode::Replace, SelectionSource::Explorer);
    assert!(!selection.select(vec![entry("a")], SelectMode::Add, SelectionSource::Explorer));
    assert!(selection.clear());
    assert!(!selection.clear());
}

#[test]
fn a_shortcut_runs_only_an_available_action_including_in_submenus() {
    let rename = Action::new("hierarchy.rename", "Rename").with_shortcut(Shortcut::key("F2"));
    let delete = Action::new("hierarchy.delete", "Delete")
        .with_shortcut(Shortcut::key("Delete"))
        .unavailable("The project folder cannot be deleted.");
    let mut add = Action::new("hierarchy.add", "Add…");
    add.items = vec![ActionGroup {
        actions: vec![Action::new("hierarchy.add-folder", "Folder").with_shortcut(Shortcut::primary("n").with_shift())],
    }];
    let groups = [ActionGroup {
        actions: vec![rename, delete, add],
    }];
    assert_eq!(
        action_for_shortcut(&groups, &Shortcut::key("F2")).map(|a| a.id.as_str()),
        Some("hierarchy.rename")
    );
    assert_eq!(action_for_shortcut(&groups, &Shortcut::key("Delete")), None);
    assert_eq!(
        action_for_shortcut(&groups, &Shortcut::primary("n").with_shift()).map(|a| a.id.as_str()),
        Some("hierarchy.add-folder")
    );
}

#[test]
fn a_property_row_checks_a_value_by_its_editor() {
    let mut toggle = PropertyRow::new("element.folded", "Folded", PropertyEditor::Toggle, "false");
    assert!(toggle.validate("true").is_ok());
    assert!(toggle.validate("yes").is_err());
    toggle.read_only_reason = "Folding is set in the file.".into();
    assert_eq!(toggle.validate("true"), Err("Folding is set in the file.".into()));

    let mut phase = PropertyRow::new("trend.phase", "Phase", PropertyEditor::Slider, "Trigger");
    phase.candidates = vec!["Trigger".into(), "Peak".into(), "Trough".into()];
    assert!(phase.validate("Peak").is_ok());
    assert_eq!(phase.validate("Plateau"), Err("Phase cannot be Plateau.".into()));

    let name = PropertyRow::new("element.text", "Text", PropertyEditor::Line, "Idea");
    assert!(name.validate("two\nlines").is_err());
}

#[test]
fn an_answer_is_a_selectable_choosable_option() {
    let options = vec![PromptOption {
        id: "freeplane".into(),
        label: "Freeplane".into(),
        selectable: false,
        children: vec![
            PromptOption {
                id: "freeplane/mindmap".into(),
                label: "Mind map".into(),
                selectable: true,
                ..Default::default()
            },
            PromptOption {
                id: "freeplane/concept".into(),
                label: "Concept map".into(),
                selectable: true,
                unavailable_reason: "Not built yet.".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }];
    assert!(find_answer(&options, "freeplane/mindmap").is_some());
    assert!(find_answer(&options, "freeplane").is_none());
    assert!(find_answer(&options, "freeplane/concept").is_none());
}

//! The designer tab, headless (spec 030, Story 4, T065): the standalone host's Knowledge designer examples opened as
//! tables, scrolled through a window, edited in place by kind, written behind and undone, and a change on disk shown.

use std::path::{Path, PathBuf};

use adp_desl::table::model::TableGesture;
use eframe::egui;

use super::{EditorKind, TableTab};
use crate::theme::{self, Palette};

const EXAMPLES: &str = "../../modules/designers/knowledge/tests/examples";

/// A copy of an example, with the table its relation points at beside it.
fn example(name: &str, folder: &Path) -> PathBuf {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join(EXAMPLES);
    for file in [name, "provinces.yaml"] {
        std::fs::copy(examples.join(file), folder.join(file)).expect("the example is copied");
    }
    folder.join(name)
}

/// One headless frame of the tab, at a window's size.
fn frame(ctx: &egui::Context, tab: &mut TableTab, events: Vec<egui::Event>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        events,
        ..Default::default()
    };
    let theme = adp_theme::Theme::dark();
    let palette = Palette::of(&theme);
    ctx.run_ui(input, |ui| super::show(ui, tab, &theme, &palette))
        .textures_delta
        .clear();
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx
}

fn key(key: egui::Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

/// The visible column of a property.
fn column(tab: &TableTab, id: &str) -> usize {
    tab.visible_columns()
        .iter()
        .position(|c| c.id == id)
        .unwrap_or_else(|| panic!("{id} is shown"))
}

/// The line of a row in the view.
fn line_of(tab: &TableTab, row: &str) -> usize {
    *tab.held
        .iter()
        .find(|(_, r)| r.id == row)
        .unwrap_or_else(|| panic!("{row} is in the window"))
        .0
}

/// A table of `rows` rows, for what scrolls.
fn generated(rows: usize, folder: &Path) -> PathBuf {
    let mut text = String::from(
        "ded: \"0.1\"\ndesigner: etalii/knowledge\nname: Places\nproperties:\n  - id: p1\n    name: Name\n    type: text\n    title: true\n  - id: p2\n    name: People\n    type: number\nviews:\n  - id: v1\n    name: All\nrows:\n",
    );
    for index in 0..rows {
        text += &format!(
            "  - id: r{index}\n    cells:\n      - property: p1\n        text: Place {index}\n      - property: p2\n        number: {index}\n"
        );
    }
    let path = folder.join("places.yaml");
    std::fs::write(&path, text).expect("written");
    path
}

#[test]
fn each_of_the_examples_opens_as_its_table_with_its_views_and_rows() {
    for name in ["cities.yaml", "cities.json", "cities.xml"] {
        let folder = tempfile::tempdir().unwrap();
        let path = example(name, folder.path());
        assert!(super::claims(&path), "{name} is a knowledge file");
        let mut tab = TableTab::open(&path, folder.path()).expect("opens");
        let ctx = context();
        frame(&ctx, &mut tab, Vec::new());
        assert_eq!(tab.title, "Cities", "{name}");
        let views: Vec<&str> = tab.views.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(views, ["All cities", "Map"], "{name}");
        assert!(tab.editable(), "{}", tab.read_only_reason);
        assert!(!tab.held.is_empty(), "{name}: the lines in sight were given");
    }
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("notes.yaml"), "just: notes\n").unwrap();
    assert!(!super::claims(&folder.path().join("notes.yaml")));
}

#[test]
fn the_session_is_asked_only_for_the_lines_in_sight_and_a_margin() {
    let folder = tempfile::tempdir().unwrap();
    let path = generated(2_000, folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    frame(&ctx, &mut tab, Vec::new());
    let (first, count) = tab.window.expect("a window was asked for");
    assert_eq!(first, 0);
    assert!(count < 60, "{count} lines for one screen");
    assert_eq!(tab.held.len(), count, "only the window is held");
    assert_eq!(tab.row_count, 2_001, "every row and the line a row is added at");

    // Scrolled down: other lines, and still only a window of them.
    tab.scroll_to = Some(1_000.0 * super::ROW_HEIGHT);
    frame(&ctx, &mut tab, Vec::new());
    frame(&ctx, &mut tab, Vec::new());
    let (first, count) = tab.window.expect("a window was asked for");
    assert!((980..=1_000).contains(&first), "{first}");
    assert_eq!(tab.held.len(), count);
    assert!(tab.held.keys().all(|i| (first..first + count).contains(i)));
    assert_eq!(tab.line(1_000).map(|r| r.id.as_str()), Some("r1000"));
}

#[test]
fn a_text_cell_is_edited_in_place_written_behind_and_undone() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.show_view("v2");
    frame(&ctx, &mut tab, Vec::new());
    let before = std::fs::read(&path).unwrap();
    let (line, name) = (line_of(&tab, "r1"), column(&tab, "p1"));

    tab.start_edit(line, name, None);
    frame(&ctx, &mut tab, Vec::new());
    let editing = tab.editing.as_mut().expect("the editor is open");
    assert_eq!((editing.editor, editing.text.as_str()), (EditorKind::Text, "Amsterdam"));
    editing.text = "Mokum".into();
    // Enter commits: shown at once, and written behind by the next frame.
    frame(&ctx, &mut tab, vec![key(egui::Key::Enter)]);
    assert!(tab.editing.is_none(), "the editor closed");
    let cell = tab.line(line).and_then(|r| r.cell("p1")).cloned().expect("the cell");
    assert_eq!(cell.values, ["Mokum"]);
    frame(&ctx, &mut tab, Vec::new());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("text: Mokum"), "written");
    assert!(!tab.session.has_pending_writes());

    tab.undo().expect("undone");
    assert_eq!(std::fs::read(&path).unwrap(), before, "undo puts the bytes back");
    assert_eq!(
        tab.line(line).and_then(|r| r.cell("p1")).map(|c| c.values.clone()),
        Some(vec!["Amsterdam".to_owned()])
    );
    tab.redo().expect("redone");
    assert!(std::fs::read_to_string(&path).unwrap().contains("text: Mokum"));
}

#[test]
fn a_value_its_kind_cannot_hold_keeps_the_editor_open_with_the_reason() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.json", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.show_view("v2");
    frame(&ctx, &mut tab, Vec::new());
    let before = std::fs::read(&path).unwrap();
    let (line, population) = (line_of(&tab, "r1"), column(&tab, "p3"));

    tab.start_edit(line, population, None);
    tab.editing.as_mut().unwrap().text = "a great many".into();
    assert!(!tab.commit_edit(), "refused");
    let editing = tab.editing.as_ref().expect("still open");
    assert_eq!(editing.reason, adp_designer_knowledge::edits::NUMBER_EXPECTED);
    frame(&ctx, &mut tab, Vec::new());
    frame(&ctx, &mut tab, Vec::new());
    assert_eq!(std::fs::read(&path).unwrap(), before, "nothing is written");
}

#[test]
fn a_tick_a_date_an_option_and_options_are_edited_by_their_kind() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.xml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.show_view("v2");
    frame(&ctx, &mut tab, Vec::new());
    let line = line_of(&tab, "r1");
    let value = |tab: &TableTab, id: &str| {
        tab.line(line)
            .and_then(|r| r.cell(id))
            .map(|c| c.values.clone())
            .unwrap_or_default()
    };

    // A tick has no editor: the gesture is the edit.
    let ticked = value(&tab, "p6");
    tab.start_edit(line, column(&tab, "p6"), None);
    assert!(tab.editing.is_none());
    assert_ne!(value(&tab, "p6"), ticked);

    // A date is typed, as year-month-day.
    tab.start_edit(line, column(&tab, "p7"), None);
    assert_eq!(tab.editing.as_ref().map(|e| e.editor), Some(EditorKind::Date));
    tab.editing.as_mut().unwrap().text = "1300-01-01".into();
    assert!(tab.commit_edit(), "{:?}", tab.editing);
    assert_eq!(value(&tab, "p7"), ["1300-01-01"]);

    // An option is picked from its list, and replaces the one there was.
    tab.start_edit(line, column(&tab, "p2"), None);
    assert_eq!(tab.editing.as_ref().map(|e| e.editor), Some(EditorKind::Option));
    frame(&ctx, &mut tab, Vec::new());
    tab.pick("o2");
    assert!(tab.editing.is_none(), "one choice closes the list");
    assert_eq!(value(&tab, "p2"), ["o2"]);

    // Several options: each pick adds or takes away, and the list stays open; a new one is made from its text.
    tab.start_edit(line, column(&tab, "p4"), None);
    let before = value(&tab, "p4");
    let first = before.first().cloned().expect("the row has a tag");
    tab.pick(&first);
    assert!(tab.editing.is_some());
    assert!(!value(&tab, "p4").contains(&first));
    tab.editing.as_mut().unwrap().text = "Harbour".into();
    tab.create_option();
    let made = tab
        .columns
        .iter()
        .find(|c| c.id == "p4")
        .and_then(|c| c.options.iter().find(|o| o.name == "Harbour"))
        .map(|o| o.id.clone())
        .expect("the option is made");
    assert!(value(&tab, "p4").contains(&made));

    // Written behind, in order, and every one of them in the file.
    frame(&ctx, &mut tab, Vec::new());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("1300-01-01") && text.contains("Harbour"), "{text}");
}

#[test]
fn a_row_is_added_at_the_new_row_line_and_a_view_change_is_written_and_undone() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let before = std::fs::read(&path).unwrap();
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    frame(&ctx, &mut tab, Vec::new());

    // The file was left in its plain view. Switching is the connection's own, and the file remembers it.
    assert_eq!(tab.settings.view_id, "v2");
    tab.show_view("v1");
    frame(&ctx, &mut tab, Vec::new());
    assert_eq!(tab.settings.view_id, "v1");
    assert!(std::fs::read_to_string(&path).unwrap().contains("activeView: v1"));

    // A row added at the bottom of a group: one more row in the view.
    let lines = tab.row_count;
    let group = tab
        .held
        .values()
        .find(|r| r.is_new_row)
        .map(|r| r.id.clone())
        .expect("a new-row line");
    tab.add_row(&group);
    assert_eq!(tab.row_count, lines + 1, "{}", tab.refusal);
    // A sort more, from the view bar's gesture.
    let sorts = tab.settings.sorts.len();
    tab.raise(
        TableGesture::new("addSort")
            .column("p1")
            .setting("direction", "descending"),
    );
    assert_eq!(tab.settings.sorts.len(), sorts + 1, "{}", tab.refusal);
    frame(&ctx, &mut tab, Vec::new());

    // Three undos: the sort, the row and the view.
    for _ in 0..3 {
        tab.undo().expect("undone");
    }
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn the_keyboard_moves_the_active_cell_and_enter_opens_its_editor() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.show_view("v2");
    frame(&ctx, &mut tab, Vec::new());
    tab.active = Some((0, 0));
    ctx.memory_mut(|m| m.request_focus(tab.grid_id()));
    frame(&ctx, &mut tab, Vec::new());
    frame(&ctx, &mut tab, vec![key(egui::Key::ArrowDown)]);
    assert_eq!(tab.active, Some((1, 0)));
    assert_eq!(tab.selected.as_deref(), tab.line(1).map(|r| r.id.as_str()));
    frame(&ctx, &mut tab, vec![key(egui::Key::Enter)]);
    assert!(tab.editing.is_some(), "Enter opens the editor");
}

#[test]
fn a_change_on_disk_is_shown_without_reopening() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.show_view("v2");
    frame(&ctx, &mut tab, Vec::new());
    let line = line_of(&tab, "r1");

    let text = std::fs::read_to_string(&path).unwrap();
    std::fs::write(&path, text.replace("Amsterdam", "Mokum")).unwrap();
    tab.changed_on_disk(&path);
    frame(&ctx, &mut tab, Vec::new());
    assert_eq!(
        tab.line(line).and_then(|r| r.cell("p1")).map(|c| c.values.clone()),
        Some(vec!["Mokum".to_owned()])
    );
}

#[test]
fn the_selected_rows_values_are_the_property_grids_rows_and_are_edited_there() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.select_row(Some("r1".into()));
    assert_eq!(tab.selected_title().as_deref(), Some("Amsterdam"));
    let rows = tab.property_rows();
    assert_eq!(
        rows.iter().find(|r| r.id == "p2").map(|r| r.value.as_str()),
        Some("Netherlands")
    );
    tab.set_property("p2", "Belgium");
    assert_eq!(tab.refusal, "");
    let rows = tab.property_rows();
    assert_eq!(
        rows.iter().find(|r| r.id == "p2").map(|r| r.value.as_str()),
        Some("Belgium")
    );
    frame(&ctx, &mut tab, Vec::new());
    tab.set_property("p2", "Atlantis");
    assert_eq!(tab.refusal, "'Atlantis' is not one of the options of Country.");
}

//! The designer tab, headless (spec 030, Story 4, T065): the standalone host's Knowledge designer examples opened as
//! tables, scrolled through a window, edited in place by kind, written behind and undone, and a change on disk shown.

use std::path::{Path, PathBuf};

use adp_desl::table::model::TableGesture;
use eframe::egui;

use crate::table::{self as table, EditorKind, TableTab};
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
    ctx.run_ui(input, |ui| table::show(ui, tab, &theme, &palette))
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
        assert!(table::claims(&path), "{name} is a knowledge file");
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
    assert!(!table::claims(&folder.path().join("notes.yaml")));
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
    tab.scroll_to = Some(1_000.0 * table::ROW_HEIGHT);
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

#[test]
fn closing_the_tab_writes_the_edits_still_waiting() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    tab.raise(TableGesture::new("setCell").row("r1").column("p1").values(["Mokum"]));
    assert!(tab.session.has_pending_writes(), "shown, and not written yet");
    drop(tab);
    assert!(std::fs::read_to_string(&path).unwrap().contains("text: Mokum"));
}

#[test]
fn the_table_is_drawn_in_the_standalone_hosts_layout_and_vs_codes_colours() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    let theme = adp_theme::Theme::dark();
    let palette = Palette::of(&theme);
    let mut textures = std::collections::HashMap::new();
    let mut shapes = Vec::new();
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| table::show(ui, &mut tab, &theme, &palette));
        crate::screenshot::take_textures(&mut textures, &mut output);
        shapes = output.shapes;
    }
    // The view's headers and a row's values are among what was painted.
    let mut texts = Vec::new();
    for clipped in &shapes {
        if let egui::Shape::Text(text) = &clipped.shape {
            texts.push(text.galley.text().to_owned());
        }
    }
    assert!(texts.iter().any(|t| t.contains("Country")), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Amsterdam"), "{texts:?}");
    assert!(texts.iter().any(|t| t == "Map"), "the view tabs: {texts:?}");
    let screenshots = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
    crate::screenshot::write(
        &ctx,
        &shapes,
        &textures,
        egui::vec2(1280.0, 800.0),
        palette.editor_background,
        &screenshots.join("knowledge-cities.png"),
    );

    // The other view groups, sorts and filters: its group headings and the line a row is added at under each.
    tab.show_view("v1");
    tab.panel = Some(table::ViewPanel::Filter);
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| table::show(ui, &mut tab, &theme, &palette));
        crate::screenshot::take_textures(&mut textures, &mut output);
        shapes = output.shapes;
    }
    assert!(tab.held.values().any(|r| r.is_group), "a group heading is in sight");
    crate::screenshot::write(
        &ctx,
        &shapes,
        &textures,
        egui::vec2(1280.0, 800.0),
        palette.editor_background,
        &screenshots.join("knowledge-cities-grouped.png"),
    );

    // A selection's list, open over its cell.
    let line = line_of(&tab, "r1");
    tab.start_edit(line, column(&tab, "p2"), None);
    for _ in 0..3 {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| table::show(ui, &mut tab, &theme, &palette));
        crate::screenshot::take_textures(&mut textures, &mut output);
        shapes = output.shapes;
    }
    assert!(tab.editing.is_some(), "the list stays open");
    crate::screenshot::write(
        &ctx,
        &shapes,
        &textures,
        egui::vec2(1280.0, 800.0),
        palette.editor_background,
        &screenshots.join("knowledge-cities-option-list.png"),
    );
}

// ---- Walk 4's findings (K1 to K9), headless ----

/// One headless frame at a window's size, its shapes kept.
fn shapes(ctx: &egui::Context, tab: &mut TableTab, events: Vec<egui::Event>) -> Vec<egui::epaint::ClippedShape> {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
        events,
        ..Default::default()
    };
    let theme = adp_theme::Theme::dark();
    let palette = Palette::of(&theme);
    let mut output = ctx.run_ui(input, |ui| table::show(ui, tab, &theme, &palette));
    output.textures_delta.clear();
    output.shapes
}

/// Where each text was painted, in the order it was painted.
fn texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, egui::Rect)> {
    shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some((
                text.galley.text().to_owned(),
                text.galley.rect.translate(text.pos.to_vec2()),
            )),
            _ => None,
        })
        .collect()
}

/// Where a text was last painted.
fn text_at(shapes: &[egui::epaint::ClippedShape], wanted: &str) -> egui::Rect {
    texts(shapes)
        .into_iter()
        .rev()
        .find(|(text, _)| text == wanted)
        .map(|(_, rect)| rect)
        .unwrap_or_else(|| {
            let all: Vec<String> = texts(shapes).into_iter().map(|t| t.0).collect();
            panic!("'{wanted}' is drawn: {all:?}")
        })
}

fn moved(at: egui::Pos2) -> egui::Event {
    egui::Event::PointerMoved(at)
}

fn button(at: egui::Pos2, pressed: bool) -> egui::Event {
    egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    }
}

/// A press and release at one place, each in its own frame, and a frame after.
fn click(ctx: &egui::Context, tab: &mut TableTab, at: egui::Pos2) -> Vec<egui::epaint::ClippedShape> {
    shapes(ctx, tab, vec![moved(at)]);
    shapes(ctx, tab, vec![button(at, true)]);
    shapes(ctx, tab, vec![button(at, false)]);
    shapes(ctx, tab, Vec::new())
}

/// A drag from one place to another, in steps, let go at the end.
fn drag(ctx: &egui::Context, tab: &mut TableTab, from: egui::Pos2, to: egui::Pos2) {
    shapes(ctx, tab, vec![moved(from)]);
    shapes(ctx, tab, vec![button(from, true)]);
    for step in 1..=4 {
        let at = from + (to - from) * (step as f32 / 4.0);
        shapes(ctx, tab, vec![moved(at)]);
    }
    shapes(ctx, tab, vec![button(to, false)]);
    shapes(ctx, tab, Vec::new());
}

/// The header cell of a column, as the last frame drew it.
fn header_of(tab: &TableTab, id: &str) -> egui::Rect {
    tab.drawn
        .header
        .iter()
        .find(|(c, _)| c == id)
        .map(|(_, r)| *r)
        .unwrap_or_else(|| panic!("{id} has a header"))
}

/// The cities example in its *Map* view, drawn.
fn map_view(folder: &Path, name: &str) -> (PathBuf, TableTab, egui::Context) {
    let path = example(name, folder);
    let mut tab = TableTab::open(&path, folder).expect("opens");
    let ctx = context();
    tab.show_view("v2");
    shapes(&ctx, &mut tab, Vec::new());
    shapes(&ctx, &mut tab, Vec::new());
    (path, tab, ctx)
}

/// The centre of a row's line, as the last frame drew it.
fn line_y(tab: &TableTab, row: &str) -> f32 {
    tab.drawn.origin.y + (line_of(tab, row) as f32 + 0.5) * table::ROW_HEIGHT
}

#[test]
fn k1_a_columns_border_dragged_resizes_it_once_on_release_and_never_under_sixty() {
    let folder = tempfile::tempdir().unwrap();
    let (path, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    let before = std::fs::read(&path).unwrap();
    let header = header_of(&tab, "p3");
    let border = egui::pos2(header.right(), header.center().y);
    let width = header.width();

    // While the border is dragged the column follows, and nothing is written.
    shapes(&ctx, &mut tab, vec![moved(border)]);
    shapes(&ctx, &mut tab, vec![button(border, true)]);
    for dx in [10.0, 40.0, 80.0, 100.0] {
        shapes(&ctx, &mut tab, vec![moved(border + egui::vec2(dx, 0.0))]);
    }
    shapes(&ctx, &mut tab, Vec::new());
    assert!(
        (header_of(&tab, "p3").width() - (width + 100.0)).abs() < 0.5,
        "the column follows the pointer"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "nothing is written while the border is dragged"
    );
    // Let go: one resizeColumn, written behind.
    shapes(&ctx, &mut tab, vec![button(border + egui::vec2(100.0, 0.0), false)]);
    shapes(&ctx, &mut tab, Vec::new());
    shapes(&ctx, &mut tab, Vec::new());
    let wanted = format!("width: {}", (width + 100.0) as i64);
    assert!(
        std::fs::read_to_string(&path).unwrap().contains(&wanted),
        "{wanted} is written"
    );
    assert_eq!(
        tab.columns.iter().find(|c| c.id == "p3").map(|c| c.width),
        Some((width + 100.0) as i64)
    );

    // Never narrower than 60.
    let header = header_of(&tab, "p3");
    let border = egui::pos2(header.right(), header.center().y);
    drag(&ctx, &mut tab, border, border - egui::vec2(1_000.0, 0.0));
    assert_eq!(tab.columns.iter().find(|c| c.id == "p3").map(|c| c.width), Some(60));
}

#[test]
fn k1_a_columns_header_dragged_moves_the_column() {
    let folder = tempfile::tempdir().unwrap();
    let (path, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    let order = |tab: &TableTab| tab.visible_columns().iter().map(|c| c.id.clone()).collect::<Vec<_>>();
    assert_eq!(order(&tab)[..4], ["p2", "p1", "p3", "p4"]);
    // Population dragged past the middle of Tags: it lands after it.
    let from = header_of(&tab, "p3");
    let past = header_of(&tab, "p4");
    drag(
        &ctx,
        &mut tab,
        from.center(),
        egui::pos2(past.center().x + 20.0, from.center().y),
    );
    assert_eq!(order(&tab)[..4], ["p2", "p1", "p4", "p3"], "{}", tab.refusal);
    shapes(&ctx, &mut tab, Vec::new());
    let text = std::fs::read_to_string(&path).unwrap();
    let map = &text[text.find("name: Map").expect("the view")..];
    let tags = map.find("- property: p4").expect("Tags listed");
    let population = map.find("- property: p3").expect("Population listed");
    assert!(tags < population, "the view lists Tags before Population now");
}

#[test]
fn k2_a_view_is_renamed_in_its_tab_by_enter_or_a_press_elsewhere() {
    let folder = tempfile::tempdir().unwrap();
    let (path, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    tab.start_view_rename("v2");
    shapes(&ctx, &mut tab, Vec::new());
    shapes(&ctx, &mut tab, Vec::new());
    tab.renaming.as_mut().expect("the field is open").name = "Recent".into();
    shapes(&ctx, &mut tab, vec![key(egui::Key::Enter)]);
    shapes(&ctx, &mut tab, Vec::new());
    assert!(tab.renaming.is_none(), "Enter closes the field");
    assert!(tab.views.iter().any(|v| v.name == "Recent"), "{:?}", tab.views);
    shapes(&ctx, &mut tab, Vec::new());
    assert!(std::fs::read_to_string(&path).unwrap().contains("name: Recent"));

    // A press elsewhere keeps the name too.
    tab.start_view_rename("v2");
    shapes(&ctx, &mut tab, Vec::new());
    shapes(&ctx, &mut tab, Vec::new());
    tab.renaming.as_mut().unwrap().name = "Places".into();
    click(&ctx, &mut tab, egui::pos2(900.0, 700.0));
    assert!(tab.renaming.is_none(), "a press elsewhere closes the field");
    assert!(tab.views.iter().any(|v| v.name == "Places"), "{:?}", tab.views);
}

#[test]
fn k3_escape_leaves_the_rename_field_and_gives_the_keyboard_back_to_the_table() {
    let folder = tempfile::tempdir().unwrap();
    let (_, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    tab.start_view_rename("v2");
    shapes(&ctx, &mut tab, Vec::new());
    shapes(&ctx, &mut tab, Vec::new());
    assert!(
        ctx.memory(|m| m.focused()).is_some_and(|f| f != tab.grid_id()),
        "the field has the keyboard"
    );
    shapes(&ctx, &mut tab, vec![key(egui::Key::Escape)]);
    shapes(&ctx, &mut tab, Vec::new());
    shapes(&ctx, &mut tab, Vec::new());
    assert!(tab.renaming.is_none());
    // The shell undoes only while nothing but the grid has the keyboard (`Shell::show`).
    assert_eq!(
        ctx.memory(|m| m.focused()),
        Some(tab.grid_id()),
        "the table has the keyboard again"
    );
}

#[test]
fn k4_the_first_column_is_drawn_over_the_cells_that_scroll_under_it() {
    let folder = tempfile::tempdir().unwrap();
    let (_, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    // Scrolled sideways by 40 points, so Name's values slide under Country, the first column.
    let middle = egui::pos2(640.0, 400.0);
    shapes(
        &ctx,
        &mut tab,
        vec![
            moved(middle),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(-40.0, 0.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
                source: egui::MouseWheelSource::Wheel,
            },
        ],
    );
    let mut painted = Vec::new();
    for _ in 0..90 {
        painted = shapes(&ctx, &mut tab, vec![moved(middle)]);
    }
    let scrolled = tab.drawn.clip.left() - tab.drawn.origin.x;
    assert!((12.0..78.0).contains(&scrolled), "scrolled sideways by {scrolled}");
    let first = header_of(&tab, "p2");
    assert!(
        (first.left() - tab.drawn.clip.left()).abs() < 0.5,
        "the first header stays at the left"
    );
    let at = egui::pos2(tab.drawn.clip.left() + first.width() - 4.0, line_y(&tab, "r1"));
    assert!(
        text_at(&painted, "Amsterdam").contains(at),
        "Name's value runs under the first column here"
    );
    // What is painted last there is the first column's own cell, not Name's value.
    let last = painted
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::Shape::Text(text) => Some((
                text.galley.rect.translate(text.pos.to_vec2()),
                Some(text.galley.text().to_owned()),
            )),
            egui::Shape::Rect(rect) if rect.fill.a() == 255 => Some((rect.rect, None)),
            _ => None,
        })
        .rfind(|(rect, _)| rect.contains(at))
        .expect("something is painted there");
    assert_eq!(last.1, None, "the first column's cell is drawn over Name's value");
    assert!((last.0.left() - tab.drawn.clip.left()).abs() < 0.5 && (last.0.width() - first.width()).abs() < 0.5);
}

#[test]
fn k5_the_question_of_a_delete_stands_after_the_rows() {
    let folder = tempfile::tempdir().unwrap();
    example("cities.yaml", folder.path());
    let provinces = folder.path().join("provinces.yaml");
    let mut tab = TableTab::open(&provinces, folder.path()).expect("opens");
    let ctx = context();
    shapes(&ctx, &mut tab, Vec::new());
    tab.delete_rows(&["nh".to_owned()]);
    let question = tab
        .asking
        .as_ref()
        .map(|a| a.1.clone())
        .expect("a row of cities.yaml relates to Noord-Holland");
    let painted = shapes(&ctx, &mut tab, Vec::new());
    let asking = tab.drawn.asking.expect("the question is drawn");
    let rows_end = tab.drawn.origin.y + tab.row_count as f32 * table::ROW_HEIGHT;
    assert!(
        asking.top() >= rows_end - 0.5,
        "after the rows: {asking:?}, the rows end at {rows_end}"
    );
    assert!(
        asking.top() <= rows_end + 2.0 * table::ROW_HEIGHT,
        "straight after them and the line of calculations"
    );
    assert!(text_at(&painted, &question).intersects(asking));
    // Keep answers it, and deletes nothing.
    let rows = tab.row_count;
    click(&ctx, &mut tab, text_at(&painted, "Keep").center());
    assert!(tab.asking.is_none());
    assert_eq!(tab.row_count, rows);
}

#[test]
fn k6_a_columns_menu_offers_the_knowledge_designers_column_actions() {
    use crate::table::header::{action_label, menu_groups};
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let tab = TableTab::open(&path, folder.path()).expect("opens");
    let labels = |id: &str| {
        let column = tab.columns.iter().find(|c| c.id == id).unwrap();
        menu_groups(column)
            .into_iter()
            .map(|g| {
                g.into_iter()
                    .map(|a| action_label(a, column, false))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels("p3"),
        [
            vec!["Rename", "Change type"],
            vec!["Filter", "Sort ascending", "Sort descending", "Group"],
            vec!["Hide in view", "Wrap text", "Freeze up to column"],
            vec!["Insert left", "Insert right", "Duplicate", "Delete"],
        ]
    );
    assert_eq!(labels("p2")[0], ["Rename", "Change type", "Edit options"]);
    assert_eq!(labels("p5")[0], ["Rename", "Change type", "Edit relation"]);
}

#[test]
fn k6_pressing_a_header_opens_its_menu_and_a_column_is_renamed_in_its_header() {
    let folder = tempfile::tempdir().unwrap();
    let (path, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    let at = header_of(&tab, "p3").center();
    let painted = click(&ctx, &mut tab, at);
    for entry in [
        "Rename",
        "Change type",
        "Filter",
        "Hide in view",
        "Wrap text",
        "Freeze up to column",
        "Insert left",
        "Duplicate",
        "Delete",
    ] {
        text_at(&painted, entry);
    }
    click(&ctx, &mut tab, text_at(&painted, "Rename").center());
    assert_eq!(tab.column_renaming.as_ref().map(|r| r.id.as_str()), Some("p3"));
    shapes(&ctx, &mut tab, Vec::new());
    tab.column_renaming.as_mut().unwrap().name = "Inhabitants".into();
    shapes(&ctx, &mut tab, vec![key(egui::Key::Enter)]);
    shapes(&ctx, &mut tab, Vec::new());
    assert!(tab.column_renaming.is_none());
    shapes(&ctx, &mut tab, Vec::new());
    assert!(std::fs::read_to_string(&path).unwrap().contains("name: Inhabitants"));

    // Freeze, from the menu: the columns up to Name stay in sight.
    let at = header_of(&tab, "p1").center();
    let painted = click(&ctx, &mut tab, at);
    click(&ctx, &mut tab, text_at(&painted, "Freeze up to column").center());
    assert_eq!(tab.frozen_up_to(), Some(1), "{}", tab.refusal);
}

#[test]
fn k6_a_hidden_column_is_shown_again_from_the_views_properties() {
    let folder = tempfile::tempdir().unwrap();
    let (_, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    tab.raise(TableGesture::new("hideColumn").column("p6"));
    assert!(!tab.visible_columns().iter().any(|c| c.id == "p6"));
    tab.panel = Some(table::ViewPanel::Properties);
    shapes(&ctx, &mut tab, Vec::new());
    let painted = shapes(&ctx, &mut tab, Vec::new());
    click(&ctx, &mut tab, text_at(&painted, "Visited").center());
    assert!(tab.visible_columns().iter().any(|c| c.id == "p6"), "{}", tab.refusal);
}

#[test]
fn k6_groups_are_hidden_one_by_one_and_shown_again() {
    let folder = tempfile::tempdir().unwrap();
    let path = example("cities.yaml", folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    tab.show_view("v1");
    tab.panel = Some(table::ViewPanel::Group);
    shapes(&ctx, &mut tab, Vec::new());
    let painted = shapes(&ctx, &mut tab, Vec::new());
    let first = texts(&painted)
        .into_iter()
        .find(|(t, _)| t == "Hide")
        .expect("a group's Hide")
        .1;
    click(&ctx, &mut tab, first.center());
    assert_eq!(
        tab.settings.settings.get("hiddenGroups").map(String::as_str),
        Some("1"),
        "{}",
        tab.refusal
    );
    shapes(&ctx, &mut tab, Vec::new());
    assert!(std::fs::read_to_string(&path).unwrap().contains("hiddenGroups"));
    let painted = shapes(&ctx, &mut tab, Vec::new());
    click(&ctx, &mut tab, text_at(&painted, "Show 1 hidden group").center());
    assert_eq!(tab.settings.settings.get("hiddenGroups"), None);
}

#[test]
fn k6_rows_are_ticked_and_acted_on_together() {
    let folder = tempfile::tempdir().unwrap();
    let (path, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    let rows_before = tab.row_count;
    let first = header_of(&tab, "p2");
    for row in ["r1", "r2"] {
        let y = line_y(&tab, row);
        click(&ctx, &mut tab, egui::pos2(first.left() + 8.0 + 7.0, y));
    }
    assert_eq!(tab.checked, ["r1", "r2"]);
    let painted = shapes(&ctx, &mut tab, Vec::new());
    text_at(&painted, "2 rows selected");
    // A property set for both at once.
    tab.bulk = Some("p6".into());
    let painted = shapes(&ctx, &mut tab, Vec::new());
    click(&ctx, &mut tab, text_at(&painted, "Check").center());
    let ticked = |tab: &TableTab, row: &str| {
        tab.line(line_of(tab, row))
            .and_then(|r| r.cell("p6"))
            .is_some_and(|c| c.values.first().is_some_and(|v| v == "true"))
    };
    assert!(ticked(&tab, "r1") && ticked(&tab, "r2"), "{}", tab.refusal);
    // Both deleted at once.
    let painted = shapes(&ctx, &mut tab, Vec::new());
    click(&ctx, &mut tab, text_at(&painted, "Delete 2 rows").center());
    assert_eq!(tab.row_count, rows_before - 2, "{}", tab.refusal);
    assert!(tab.checked.is_empty());
    shapes(&ctx, &mut tab, Vec::new());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("text: Amsterdam") && !text.contains("text: Antwerp"));
}

#[test]
fn k7_the_line_of_calculations_shows_and_offers_each_columns_calculations() {
    let folder = tempfile::tempdir().unwrap();
    let (path, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    let line = tab.drawn.calculations.expect("the line of calculations is drawn");
    let rows_end = tab.drawn.origin.y + tab.row_count as f32 * table::ROW_HEIGHT;
    assert!((line.top() - rows_end).abs() < 0.5, "under the rows");
    // Pressing Population's place there offers its calculations; Sum is written to the view.
    let at = egui::pos2(header_of(&tab, "p3").center().x, line.center().y);
    let painted = click(&ctx, &mut tab, at);
    for offered in ["None", "Count all", "Count unique values", "Sum", "Average", "Median"] {
        text_at(&painted, offered);
    }
    click(&ctx, &mut tab, text_at(&painted, "Sum").center());
    shapes(&ctx, &mut tab, Vec::new());
    assert!(std::fs::read_to_string(&path).unwrap().contains("calculate: sum"));
    let painted = shapes(&ctx, &mut tab, Vec::new());
    text_at(&painted, "SUM");
}

#[test]
fn k9_a_notch_of_the_wheel_scrolls_the_browsers_hundred_points() {
    let folder = tempfile::tempdir().unwrap();
    let path = generated(2_000, folder.path());
    let mut tab = TableTab::open(&path, folder.path()).expect("opens");
    let ctx = context();
    let middle = egui::pos2(640.0, 400.0);
    shapes(&ctx, &mut tab, vec![moved(middle)]);
    let top = tab.drawn.origin.y;
    shapes(
        &ctx,
        &mut tab,
        vec![
            moved(middle),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -3.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
                source: egui::MouseWheelSource::Wheel,
            },
        ],
    );
    for _ in 0..90 {
        shapes(&ctx, &mut tab, vec![moved(middle)]);
    }
    let scrolled = top - tab.drawn.origin.y;
    assert!(
        (scrolled - 300.0).abs() < 2.0,
        "three notches scrolled {scrolled} points"
    );
}

#[test]
fn a_relation_is_drawn_as_text_not_as_a_link() {
    let folder = tempfile::tempdir().unwrap();
    let (_, mut tab, ctx) = map_view(folder.path(), "cities.yaml");
    let painted = shapes(&ctx, &mut tab, Vec::new());
    let label = text_at(&painted, "Noord-Holland");
    let underlined = painted.iter().any(|clipped| match &clipped.shape {
        egui::Shape::LineSegment { points, .. } => {
            (points[0].y - points[1].y).abs() < 0.1
                && (points[0].y - label.bottom()).abs() < 2.0
                && points[0].x.min(points[1].x) >= label.left() - 1.0
                && points[0].x.max(points[1].x) <= label.right() + 1.0
        }
        _ => false,
    });
    assert!(
        !underlined,
        "the Knowledge designer opens no related row, so its title is not a link"
    );
}

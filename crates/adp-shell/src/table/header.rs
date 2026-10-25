//! The table's header (the standalone host's `header/ColumnHeader.tsx`, `header/columnActions.ts` and
//! `header/RelationEditor.tsx` at 8b6ccfc): one cell per visible column with its kind's mark, its name and its sort.
//! Pressing a header opens its menu - the Knowledge designer's `columnActions` - dragging it moves the column, and
//! dragging its right border resizes it, sent once on release; a column is renamed in its own cell; its options
//! ([`super::options`]) and its relation are edited in a panel under it.

use adp_desl::table::model::{TableColumn, TableGesture};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

use super::options::{OptionsEditing, options_editor, table_question};
use super::view::{Asked, CELL_PADDING, Colours, arrow, check_box, kind_mark};
use super::{ColumnPanel, HeaderDrag, ROW_HEIGHT, TableTab, kind_label};

/// A column's narrowest width, in points (`MINIMUM_COLUMN_WIDTH`).
pub const MINIMUM_COLUMN_WIDTH: f32 = 60.0;
/// The width of the strip over a header's right edge that is dragged to resize its column (`table-header-resize`).
const RESIZE_STRIP: f32 = 7.0;
/// The width of the header's button that adds a property.
pub const ADD_COLUMN: f32 = 40.0;
/// The setting a column names the other side of its two-way relation under (`OTHER_SIDE`).
const OTHER_SIDE: &str = "otherSide";

/// A width a drag may leave a column with: whole points, and never narrower than the minimum (`resizedWidth`).
pub fn resized_width(start: f32, dx: f32) -> f32 {
    MINIMUM_COLUMN_WIDTH.max((start + dx).round())
}

/// Where a dragged column lands: after every other column whose middle the pointer has passed (`dropIndex`).
pub fn drop_index(from: usize, pointer_x: f32, columns: &[(f32, f32)]) -> usize {
    columns
        .iter()
        .enumerate()
        .filter(|(index, (left, right))| *index != from && (left + right) / 2.0 < pointer_x)
        .count()
}

/// One entry of a column's menu (`ColumnAction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAction {
    Rename,
    ChangeType,
    Options,
    Relation,
    Filter,
    SortAscending,
    SortDescending,
    Group,
    Nest,
    Hide,
    Wrap,
    Freeze,
    InsertLeft,
    InsertRight,
    Duplicate,
    Delete,
}

/// The order the menu shows its entries in, and where a line separates them (`MENU_ORDER`). The Knowledge designer
/// offers every one of them (`knowledgeDefinition.ts`'s `columnActions`).
const MENU_ORDER: [&[ColumnAction]; 4] = [
    &[
        ColumnAction::Rename,
        ColumnAction::ChangeType,
        ColumnAction::Options,
        ColumnAction::Relation,
    ],
    &[
        ColumnAction::Filter,
        ColumnAction::SortAscending,
        ColumnAction::SortDescending,
        ColumnAction::Group,
        ColumnAction::Nest,
    ],
    &[ColumnAction::Hide, ColumnAction::Wrap, ColumnAction::Freeze],
    &[
        ColumnAction::InsertLeft,
        ColumnAction::InsertRight,
        ColumnAction::Duplicate,
        ColumnAction::Delete,
    ],
];

/// A column's menu, grouped (`columnMenuGroups`): only a selection has options to edit, only a relation that holds its
/// own values has a relation to edit, and only a column that says its rows can be nested by it offers to.
pub fn menu_groups(column: &TableColumn) -> Vec<Vec<ColumnAction>> {
    let setting = |name: &str| column.settings.get(name).map(String::as_str).unwrap_or_default();
    let offered = |action: &ColumnAction| match action {
        ColumnAction::Options => matches!(column.kind.as_str(), "selection" | "multipleSelection"),
        ColumnAction::Relation => column.kind == "relation" && setting("computed") != "true",
        ColumnAction::Nest => setting("nestable") == "true",
        _ => true,
    };
    MENU_ORDER
        .iter()
        .map(|group| group.iter().copied().filter(offered).collect::<Vec<_>>())
        .filter(|group| !group.is_empty())
        .collect()
}

/// What an entry is called for a column: some say what they would undo.
pub fn action_label(action: ColumnAction, column: &TableColumn, frozen_up_to_this: bool) -> String {
    let parent = column.settings.get("parent").is_some_and(|p| p == "true");
    match action {
        ColumnAction::Rename => "Rename",
        ColumnAction::ChangeType => "Change type",
        ColumnAction::Options => "Edit options",
        ColumnAction::Relation => "Edit relation",
        ColumnAction::Filter => "Filter",
        ColumnAction::SortAscending => "Sort ascending",
        ColumnAction::SortDescending => "Sort descending",
        ColumnAction::Group => "Group",
        ColumnAction::Nest if parent => "Do not nest rows by this",
        ColumnAction::Nest => "Nest rows by this",
        ColumnAction::Hide => "Hide in view",
        ColumnAction::Wrap if column.wraps => "Do not wrap",
        ColumnAction::Wrap => "Wrap text",
        ColumnAction::Freeze if frozen_up_to_this => "Unfreeze columns",
        ColumnAction::Freeze => "Freeze up to column",
        ColumnAction::InsertLeft => "Insert left",
        ColumnAction::InsertRight => "Insert right",
        ColumnAction::Duplicate => "Duplicate",
        ColumnAction::Delete => "Delete",
    }
    .to_owned()
}

/// The kinds a column may be given or changed to: every kind but a relation, which is given to a new property only,
/// where the table it relates to is asked for.
fn changeable_kinds() -> impl Iterator<Item = &'static str> {
    adp_designer_knowledge::vocabulary::VALUE_TYPES
        .iter()
        .copied()
        .filter(|kind| *kind != "relation")
}

const TITLE_STAYS: &str = "The column that names a row is always there.";

/// The header: one cell per visible column, the frozen ones over those that scroll under them, the borders that
/// resize, and the + that adds a property.
#[allow(clippy::too_many_arguments)]
pub(super) fn header_row(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    font: &FontId,
    rect: Rect,
    offset: f32,
    columns: &[TableColumn],
    widths: &[f32],
    frozen: &[Option<f32>],
    asked: &mut Asked,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, colours.surface);
    painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, colours.border));
    let editable = tab.editable();
    let frozen_up_to = tab.frozen_up_to();
    // Where each column stands unscrolled, and where it is drawn: a frozen one is kept in sight.
    let mut natural = Vec::with_capacity(columns.len());
    let mut x = rect.left() - offset;
    for width in widths {
        natural.push((x, x + width));
        x += width;
    }
    let cells: Vec<Rect> = natural
        .iter()
        .zip(frozen)
        .map(|(&(left, right), at)| {
            let shown = match at {
                Some(at) => left.max(rect.left() + at),
                None => left,
            };
            Rect::from_min_size(Pos2::new(shown, rect.top()), Vec2::new(right - left, ROW_HEIGHT))
        })
        .collect();
    tab.drawn.header = columns
        .iter()
        .map(|c| c.id.clone())
        .zip(cells.iter().copied())
        .collect();
    // The columns that scroll first, the frozen ones over them.
    let order: Vec<usize> = (0..columns.len())
        .filter(|i| frozen[*i].is_none())
        .chain((0..columns.len()).filter(|i| frozen[*i].is_some()))
        .collect();
    for &index in &order {
        let column = &columns[index];
        let cell = cells[index];
        if cell.right() < rect.left() || cell.left() > rect.right() {
            continue;
        }
        let shown = cell.intersect(rect);
        let painter = ui.painter_at(shown);
        painter.rect_filled(cell, 0.0, colours.surface);
        let edge = frozen_up_to == Some(index) && index > 0;
        painter.vline(
            cell.right() - if edge { 1.0 } else { 0.5 },
            cell.y_range(),
            Stroke::new(if edge { 2.0 } else { 1.0 }, colours.border),
        );
        painter.hline(cell.x_range(), cell.bottom() - 0.5, Stroke::new(1.0, colours.border));
        if tab.column_renaming.as_ref().is_some_and(|r| r.id == column.id) {
            rename_field(ui, tab, colours, font, cell, column, asked);
            continue;
        }
        let mut left = cell.left() + CELL_PADDING;
        if index == 0 && editable && tab.row_count > 0 {
            // The first header's checkbox, which ticks every row the table holds, where a row's own stands.
            let rows: Vec<String> = tab
                .held
                .values()
                .filter(|r| !r.is_group && !r.is_new_row)
                .map(|r| r.id.clone())
                .collect();
            let all = !rows.is_empty() && rows.iter().all(|r| tab.checked.contains(r));
            let mark = Rect::from_center_size(Pos2::new(left + 7.0, cell.center().y), Vec2::splat(14.0));
            let response = ui.interact(mark.expand(2.0), tab.id().with("check-all"), Sense::click());
            if all || response.hovered() || ui.rect_contains_pointer(rect) {
                check_box(&painter, mark, all, colours);
            }
            if response.on_hover_text("Select all rows").clicked() {
                if all {
                    tab.checked.clear();
                    tab.bulk = None;
                } else {
                    for row in rows {
                        if !tab.checked.contains(&row) {
                            tab.checked.push(row);
                        }
                    }
                }
            }
            left += 18.0;
        }
        let kind = kind_label(&column.kind);
        let mark = Rect::from_center_size(Pos2::new(left + 7.0, cell.center().y), Vec2::splat(14.0));
        kind_mark(&painter, mark, &column.kind, colours.muted, font);
        let name = painter.text(
            Pos2::new(mark.right() + 6.0, cell.center().y),
            Align2::LEFT_CENTER,
            &column.name,
            font.clone(),
            colours.muted,
        );
        if let Some(sort) = tab.settings.sorts.iter().find(|s| s.column_id == column.id) {
            arrow(
                &painter,
                Pos2::new(name.right() + 8.0, cell.center().y),
                sort.descending,
                colours.muted,
            );
        }
        let sense = if editable {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let response = ui
            .interact(shown, tab.id().with(("header", &column.id)), sense)
            .on_hover_text(format!("{} · {kind}", column.name));
        if !editable {
            continue;
        }
        // Dragging the header moves its column: one gesture, where it is let go.
        let travelled = ui.input(|i| match (i.pointer.press_origin(), i.pointer.interact_pos()) {
            (Some(from), Some(to)) => to.x - from.x,
            _ => 0.0,
        });
        if response.drag_started() {
            tab.header_drag = Some(HeaderDrag::Move {
                column_id: column.id.clone(),
                from: index,
                dx: 0.0,
            });
        }
        if response.dragged()
            && let Some(HeaderDrag::Move { dx, .. }) = &mut tab.header_drag
        {
            *dx = travelled;
        }
        if response.drag_stopped()
            && let Some(HeaderDrag::Move { from, dx, .. }) = tab.header_drag.take()
        {
            let (left, right) = natural[from];
            let to = drop_index(from, (left + right) / 2.0 + dx, &natural);
            if to != from {
                asked
                    .gestures
                    .push(TableGesture::new("moveColumn").column(&column.id).index(to as i64));
            }
        }
        if tab
            .header_drag
            .as_ref()
            .is_some_and(|d| matches!(d, HeaderDrag::Move { column_id, .. } if *column_id == column.id))
        {
            // Where it would land, as a line between the columns.
            if let Some(HeaderDrag::Move { from, dx, .. }) = &tab.header_drag {
                let (left, right) = natural[*from];
                let to = drop_index(*from, (left + right) / 2.0 + dx, &natural);
                let others: Vec<&(f32, f32)> = natural
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i != from)
                    .map(|(_, n)| n)
                    .collect();
                let at = others
                    .get(to)
                    .map_or_else(|| others.last().map_or(left, |n| n.1), |n| n.0);
                ui.painter().vline(at, rect.y_range(), Stroke::new(2.0, colours.focus));
            }
        }
        // Pressing the header opens its menu, and so does the right button.
        let open = if response.clicked() {
            Some(egui::SetOpenCommand::Toggle)
        } else if response.secondary_clicked() {
            Some(egui::SetOpenCommand::Bool(true))
        } else {
            None
        };
        egui::Popup::menu(&response)
            .open_memory(open)
            .show(|ui| column_menu(ui, tab, column, frozen_up_to == Some(index), asked));
    }
    // The borders that resize, over every header so each is grabbed whole.
    if editable {
        for &index in &order {
            let column = &columns[index];
            let cell = cells[index];
            let strip = Rect::from_min_max(
                Pos2::new(cell.right() - RESIZE_STRIP / 2.0, cell.top()),
                Pos2::new(cell.right() + RESIZE_STRIP / 2.0, cell.bottom()),
            );
            if strip.right() < rect.left() || strip.left() > rect.right() {
                continue;
            }
            let response = ui
                .interact(
                    strip.intersect(rect),
                    tab.id().with(("resize", &column.id)),
                    Sense::drag(),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            let travelled = ui.input(|i| match (i.pointer.press_origin(), i.pointer.interact_pos()) {
                (Some(from), Some(to)) => to.x - from.x,
                _ => 0.0,
            });
            if response.drag_started() {
                tab.header_drag = Some(HeaderDrag::Resize {
                    column_id: column.id.clone(),
                    grabbed: widths[index],
                    width: resized_width(widths[index], travelled),
                });
            }
            if response.dragged()
                && let Some(HeaderDrag::Resize { grabbed, width, .. }) = &mut tab.header_drag
            {
                *width = resized_width(*grabbed, travelled);
            }
            // The width is sent once, on release: a resize written on every move would be a write per point.
            // The pointer's travel is known no more once it is let go: the width is the one the drag last gave.
            if response.drag_stopped()
                && let Some(HeaderDrag::Resize {
                    grabbed, width: next, ..
                }) = tab.header_drag.take()
                && next != grabbed
            {
                asked.gestures.push(
                    TableGesture::new("resizeColumn")
                        .column(&column.id)
                        .setting("width", (next as i64).to_string()),
                );
            }
        }
        let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(ADD_COLUMN, ROW_HEIGHT));
        if cell.left() < rect.right() {
            ui.scope_builder(egui::UiBuilder::new().max_rect(cell.intersect(rect)), |ui| {
                ui.menu_button(RichText::new("+").color(colours.muted), |ui| {
                    for kind in changeable_kinds() {
                        if ui.button(kind_label(kind)).clicked() {
                            asked
                                .gestures
                                .push(TableGesture::new("addColumn").setting("type", kind));
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text("Add a property");
            });
        }
    }
    column_panel(ui, tab, colours);
}

/// A column's menu: the Knowledge designer's `columnActions`, grouped, each bound to its gesture.
fn column_menu(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    column: &TableColumn,
    frozen_up_to_this: bool,
    asked: &mut Asked,
) {
    let groups = menu_groups(column);
    for (number, group) in groups.iter().enumerate() {
        if number > 0 {
            ui.separator();
        }
        for &action in group {
            let label = action_label(action, column, frozen_up_to_this);
            match action {
                ColumnAction::ChangeType => {
                    ui.menu_button(label, |ui| {
                        for kind in changeable_kinds() {
                            let same = kind == column.kind;
                            let response = ui
                                .add_enabled(!same, egui::Button::new(kind_label(kind)))
                                .on_disabled_hover_text("It is this type already.");
                            if response.clicked() {
                                asked.gestures.push(
                                    TableGesture::new("setColumnType")
                                        .column(&column.id)
                                        .setting("type", kind),
                                );
                                ui.close();
                            }
                        }
                    });
                }
                ColumnAction::InsertLeft | ColumnAction::InsertRight => {
                    let side = if action == ColumnAction::InsertLeft {
                        "left"
                    } else {
                        "right"
                    };
                    ui.menu_button(label, |ui| {
                        for kind in changeable_kinds() {
                            if ui.button(kind_label(kind)).clicked() {
                                // The property is named at once, as a column just added is.
                                tab.naming_new = Some(tab.columns.iter().map(|c| c.id.clone()).collect());
                                asked.gestures.push(
                                    TableGesture::new("addColumn")
                                        .target(&column.id)
                                        .setting("side", side)
                                        .setting("type", kind),
                                );
                                ui.close();
                            }
                        }
                    });
                }
                ColumnAction::Delete if column.settings.get(OTHER_SIDE).is_some_and(|o| !o.is_empty()) => {
                    // One side of a two-way relation is not deleted on one press: its author chooses what becomes of
                    // the other side.
                    let other = column.settings[OTHER_SIDE].clone();
                    ui.menu_button(label, |ui| {
                        for (choice, text) in [
                            ("delete", format!("Delete '{other}' too")),
                            ("keep", format!("Keep '{other}' as a one-way relation")),
                        ] {
                            if ui.button(text).clicked() {
                                asked.gestures.push(
                                    TableGesture::new("deleteColumn")
                                        .column(&column.id)
                                        .setting(OTHER_SIDE, choice),
                                );
                                ui.close();
                            }
                        }
                    });
                }
                _ => {
                    let title_stays = column.is_title && matches!(action, ColumnAction::Hide | ColumnAction::Delete);
                    let response = ui
                        .add_enabled(!title_stays, egui::Button::new(label))
                        .on_disabled_hover_text(TITLE_STAYS);
                    if response.clicked() {
                        act(tab, column, action, frozen_up_to_this, asked);
                        ui.close();
                    }
                }
            }
        }
    }
}

/// What an entry of a column's menu does.
fn act(tab: &mut TableTab, column: &TableColumn, action: ColumnAction, frozen_up_to_this: bool, asked: &mut Asked) {
    let gesture = |kind: &str| TableGesture::new(kind).column(&column.id);
    let flip = |on: bool| if on { "false" } else { "true" };
    let push = match action {
        ColumnAction::Rename => {
            tab.start_column_rename(&column.id);
            None
        }
        ColumnAction::Options => {
            tab.options_editing = OptionsEditing::default();
            tab.panel_refusal.clear();
            tab.column_panel = Some(ColumnPanel::Options {
                column_id: column.id.clone(),
            });
            None
        }
        ColumnAction::Relation => {
            tab.panel_text.clear();
            tab.panel_refusal.clear();
            tab.column_panel = Some(ColumnPanel::Relation {
                column_id: column.id.clone(),
            });
            None
        }
        ColumnAction::Filter => Some(gesture("addFilter")),
        ColumnAction::SortAscending => Some(gesture("addSort").setting("direction", "ascending")),
        ColumnAction::SortDescending => Some(gesture("addSort").setting("direction", "descending")),
        ColumnAction::Group => Some(gesture("groupBy")),
        ColumnAction::Nest => {
            let parent = column.settings.get("parent").is_some_and(|p| p == "true");
            Some(gesture("setColumnParent").setting("parent", flip(parent)))
        }
        ColumnAction::Hide => Some(gesture("hideColumn")),
        ColumnAction::Wrap => Some(gesture("setColumnWrap").setting("wrap", flip(column.wraps))),
        ColumnAction::Freeze if frozen_up_to_this => Some(TableGesture::new("freezeColumns")),
        ColumnAction::Freeze => Some(gesture("freezeColumns")),
        ColumnAction::Duplicate => Some(gesture("duplicateColumn")),
        ColumnAction::Delete => Some(gesture("deleteColumn")),
        ColumnAction::ChangeType | ColumnAction::InsertLeft | ColumnAction::InsertRight => None,
    };
    if let Some(gesture) = push {
        asked.gestures.push(gesture);
    }
}

/// The column's name as a field in its header's place (`RenameField`): Enter keeps the name, Escape drops it, and so
/// does nothing typed. A refused name is said under the field, which stays open; leaving it then keeps the old name.
fn rename_field(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    font: &FontId,
    cell: Rect,
    column: &TableColumn,
    asked: &mut Asked,
) {
    let id = tab.id().with(("rename-column", &column.id));
    let Some(renaming) = tab.column_renaming.as_mut() else {
        return;
    };
    ui.painter().rect_filled(cell, 0.0, colours.surface);
    let response = ui.put(
        cell.shrink2(Vec2::new(CELL_PADDING, 6.0)),
        egui::TextEdit::singleline(&mut renaming.name)
            .id(id)
            .frame(egui::Frame::NONE)
            .font(font.clone()),
    );
    ui.painter().rect_stroke(
        cell.shrink(1.0),
        0.0,
        Stroke::new(2.0, colours.focus),
        StrokeKind::Inside,
    );
    if !renaming.focused {
        response.request_focus();
        renaming.focused = true;
    }
    if response.changed() {
        renaming.refusal.clear();
    }
    if !renaming.refusal.is_empty() {
        let at = cell.left_bottom();
        let reason = renaming.refusal.clone();
        egui::Area::new(id.with("refusal"))
            .order(egui::Order::Foreground)
            .fade_in(false)
            .fixed_pos(at)
            .show(ui.ctx(), |ui| {
                egui::Frame::NONE
                    .fill(colours.danger_bg)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| ui.label(RichText::new(reason).color(colours.text)));
            });
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        asked.column_rename = Some(None);
    } else if response.lost_focus() {
        let leaving = !ui.input(|i| i.key_pressed(egui::Key::Enter));
        asked.column_rename = Some(Some((column.id.clone(), renaming.name.trim().to_owned(), leaving)));
    }
}

/// Carries out a rename of a column ended in its field: `None` when it was dropped.
pub(super) fn finish_rename(tab: &mut TableTab, ended: Option<(String, String, bool)>) -> bool {
    let Some((column_id, name, leaving)) = ended else {
        tab.column_renaming = None;
        return true;
    };
    let refused_before = tab.column_renaming.as_ref().is_some_and(|r| !r.refusal.is_empty());
    let current = tab.columns.iter().find(|c| c.id == column_id).map(|c| c.name.clone());
    // A name of nothing is no name, and a field left after a refusal is given up.
    if name.is_empty() || (leaving && refused_before) || current.as_deref() == Some(name.as_str()) {
        tab.column_renaming = None;
        return true;
    }
    let answer = tab.edit(TableGesture::new("renameColumn").column(&column_id).values([name]));
    if answer.is_empty() {
        tab.column_renaming = None;
        true
    } else {
        if let Some(open) = &mut tab.column_renaming {
            open.refusal = answer;
            open.focused = false;
        }
        false
    }
}

/// The panel of a column's options or relation, under its header. A press anywhere else, or Escape, closes it.
fn column_panel(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours) {
    let Some(panel) = tab.column_panel.clone() else {
        return;
    };
    let column_id = match &panel {
        ColumnPanel::Options { column_id } | ColumnPanel::Relation { column_id } => column_id.clone(),
    };
    let Some(column) = tab.columns.iter().find(|c| c.id == column_id).cloned() else {
        tab.column_panel = None;
        return;
    };
    let at = tab
        .drawn
        .header
        .iter()
        .find(|(id, _)| *id == column_id)
        .map(|(_, r)| r.left_bottom())
        .unwrap_or_else(|| ui.max_rect().left_top());
    let at = Pos2::new(at.x.max(ui.max_rect().left() + 8.0), at.y);
    let id = tab.id().with(("column-panel", &column_id));
    let mut gestures: Vec<TableGesture> = Vec::new();
    let mut asked_relation: Option<TableGesture> = None;
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .fixed_pos(at)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).fill(colours.surface).show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
                match panel {
                    ColumnPanel::Options { .. } => options_panel(ui, tab, colours, &column, &mut gestures),
                    ColumnPanel::Relation { .. } => relation_panel(ui, tab, colours, &column, &mut asked_relation),
                }
            });
        });
    let outside = ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p))
    });
    for gesture in gestures {
        tab.raise(gesture);
    }
    if let Some(gesture) = asked_relation {
        tab.panel_refusal = tab.edit(gesture);
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || outside {
        tab.column_panel = None;
        tab.options_editing = OptionsEditing::default();
    }
}

/// A column's options, edited in [`super::options`], every change one gesture made as it is chosen.
fn options_panel(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    column: &TableColumn,
    gestures: &mut Vec<TableGesture>,
) {
    let id = tab.id().with(("options", &column.id));
    gestures.extend(options_editor(
        ui,
        &mut tab.options_editing,
        id,
        column,
        colours,
        true,
        &table_question,
    ));
}

/// A relation's own settings: the table it relates to, said; how many rows a cell holds; and whether the other
/// table shows it too (`RelationEditor.tsx`). A change that is refused is said here.
fn relation_panel(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    column: &TableColumn,
    asked: &mut Option<TableGesture>,
) {
    let setting = |name: &str| column.settings.get(name).cloned().unwrap_or_default();
    let target = setting("targetFile");
    ui.horizontal(|ui| {
        ui.label(RichText::new("Related to").color(colours.muted));
        ui.label(
            RichText::new(if target.is_empty() || target == "." {
                "This table"
            } else {
                &target
            })
            .strong(),
        );
    });
    let one = setting("limit") == "one";
    ui.horizontal(|ui| {
        ui.label("Limit");
        egui::ComboBox::from_id_salt(tab.id().with(("relation-limit", &column.id)))
            .selected_text(if one { "1 row" } else { "No limit" })
            .show_ui(ui, |ui| {
                for (value, label) in [("none", "No limit"), ("one", "1 row")] {
                    if ui.selectable_label((value == "one") == one, label).clicked() && (value == "one") != one {
                        *asked = Some(
                            TableGesture::new("setRelationLimit")
                                .column(&column.id)
                                .setting("limit", value),
                        );
                    }
                }
            });
    });
    let other_side = setting(OTHER_SIDE);
    let mut two_way = !other_side.is_empty();
    let other_name = tab.panel_text.entry("other".to_owned()).or_default();
    if ui.checkbox(&mut two_way, "Two-way relation").changed() {
        let gesture = TableGesture::new("setRelationOtherSide").column(&column.id);
        *asked = Some(if two_way {
            gesture
                .setting("twoWay", "true")
                .setting("counterpart", other_name.trim())
        } else {
            gesture.setting("twoWay", "false")
        });
    }
    if other_side.is_empty() {
        ui.add(
            egui::TextEdit::singleline(other_name)
                .hint_text("Name in the other table")
                .desired_width(200.0),
        );
    } else {
        ui.label(format!("Shown in the other table as {other_side}."));
    }
    if !tab.panel_refusal.is_empty() {
        ui.colored_label(colours.danger, &tab.panel_refusal);
    }
}

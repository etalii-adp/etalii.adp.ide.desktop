//! Drawing a [`TableTab`]: the read-only note, the view tabs and the view bar above the table, the sticky header, the
//! window of lines with `ScrollArea::show_rows`, the cell editors, and the table's own line for a refusal or a
//! question (the standalone host's `TableSurface.tsx`, `ViewTabs.tsx`, `ViewBar.tsx`, `CellEditor.tsx`,
//! `GroupHeading.tsx`, `NewRow.tsx` and `OptionTag.tsx` at 8b6ccfc, in `table.css`'s layout and VS Code's colours).

use adp_desl::table::model::{TableColumn, TableFilterItem, TableGesture, TableRow};
use adp_theme::{Kind, Theme};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

use super::{EditorKind, ROW_HEIGHT, TableTab, ViewPanel, groupable, kind_label};
use crate::theme::{self, Palette};

/// A cell's padding, and the step a nested row is set in by (`CELL_PADDING`, `INDENT`).
const CELL_PADDING: f32 = 8.0;
const INDENT: f32 = 20.0;
/// The width of the header's button that adds a property.
const ADD_COLUMN: f32 = 40.0;

/// The colours the table draws with: VS Code's, by role.
struct Colours {
    surface: Color32,
    text: Color32,
    muted: Color32,
    border: Color32,
    raised: Color32,
    selected: Color32,
    focus: Color32,
    danger: Color32,
    danger_bg: Color32,
    warning: Color32,
    warning_bg: Color32,
    dark: bool,
}

impl Colours {
    fn of(theme: &Theme, palette: &Palette) -> Colours {
        let c = |ids: &[&str], fallback: Color32| theme::color(theme, ids, fallback);
        Colours {
            surface: palette.editor_background,
            text: palette.foreground,
            muted: c(&["descriptionForeground"], palette.foreground),
            border: c(
                &["tree.tableColumnsBorder", "editorGroup.border", "panel.border"],
                Color32::from_gray(80),
            ),
            raised: c(
                &["sideBarSectionHeader.background", "list.hoverBackground"],
                palette.side_bar_background,
            ),
            selected: c(
                &["list.inactiveSelectionBackground", "list.hoverBackground"],
                palette.side_bar_background,
            ),
            focus: palette.focus,
            danger: c(&["errorForeground"], Color32::RED),
            danger_bg: c(&["inputValidation.errorBackground"], palette.notification_background),
            warning: c(&["editorWarning.foreground"], Color32::YELLOW),
            warning_bg: c(&["inputValidation.warningBackground"], palette.notification_background),
            dark: theme.kind == Kind::Dark,
        }
    }

    /// An option's colour by its theme name: the standalone host's own ten (`--color-table-option-*`), which VS Code
    /// has no colours for.
    fn option(&self, name: &str) -> Color32 {
        let hex = |rgb: u32| Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
        let (light, dark) = match name {
            "gray" => (0xe2e8f0, 0x475569),
            "brown" => (0xeedfd3, 0x603b2c),
            "orange" => (0xfde3c8, 0x7a4317),
            "yellow" => (0xfdecc8, 0x6b4d14),
            "green" => (0xdbeddb, 0x2b593f),
            "blue" => (0xd3e5ef, 0x28456c),
            "purple" => (0xe8deee, 0x492f64),
            "pink" => (0xf5e0e9, 0x69314c),
            "red" => (0xffe2dd, 0x6e3630),
            _ => (0xf1f5f9, 0x334155),
        };
        hex(if self.dark { dark } else { light })
    }
}

/// How a finding's severity ranks, least serious first.
fn rank(severity: adp_desl::table::model::TableFindingSeverity) -> u8 {
    severity as u8
}

/// What a cell shows while it is not being edited, as one line (`cellText`).
fn cell_text(row: &TableRow, column_id: &str) -> String {
    row.cell(column_id)
        .map(|cell| {
            cell.values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    cell.labels
                        .get(i)
                        .filter(|l| !l.is_empty())
                        .cloned()
                        .unwrap_or_else(|| v.clone())
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// A comparison as it reads between a property and a value (`knowledgeDefinition.ts`).
fn comparison_label(id: &str) -> &str {
    match id {
        "isEmpty" | "is-empty" => "is empty",
        "isNotEmpty" | "is-not-empty" => "is not empty",
        "is-not" => "is not",
        "does-not-contain" => "does not contain",
        "starts-with" => "starts with",
        "ends-with" => "ends with",
        "does-not-equal" => "does not equal",
        "greater-than" => "is greater than",
        "less-than" => "is less than",
        "at-least" => "is at least",
        "at-most" => "is at most",
        "is-checked" => "is checked",
        "is-not-checked" => "is not checked",
        "is-before" => "is before",
        "is-after" => "is after",
        "is-on-or-before" => "is on or before",
        "is-on-or-after" => "is on or after",
        other => other,
    }
}

/// Whether a comparison is complete without a value.
fn takes_value(id: &str) -> bool {
    !matches!(
        id,
        "isEmpty" | "isNotEmpty" | "is-empty" | "is-not-empty" | "is-checked" | "is-not-checked"
    )
}

/// Draws the tab: everything above the table, the table, and its own line.
pub fn show(ui: &mut egui::Ui, tab: &mut TableTab, theme: &Theme, palette: &Palette) {
    // What the frames before this one showed is written now, one edit at a time.
    tab.write_behind();
    let colours = Colours::of(theme, palette);
    let font = FontId::proportional(theme.sizes.workbench_font);
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

    if !tab.read_only_reason.is_empty() {
        egui::Frame::NONE
            .fill(colours.warning_bg)
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(&tab.read_only_reason).color(colours.text));
            });
    }
    if !tab.views.is_empty() {
        view_tabs(ui, tab, &colours, &font);
        view_bar(ui, tab, &colours);
    }
    let footer = !tab.refusal.is_empty() || tab.asking.is_some();
    let footer_height = if footer { ROW_HEIGHT } else { 0.0 };
    grid(ui, tab, &colours, &font, footer_height);
    if footer {
        footer_line(ui, tab, &colours);
    }
    let pending = tab.session.pending_edits();
    if tab.editable() && pending > 0 {
        // Over the table in the frame's corner, never above its rows: a line that came and went moved every row.
        let text = if pending == 1 {
            "1 change is being saved…".to_owned()
        } else {
            format!("{pending} changes are being saved…")
        };
        let corner = ui.max_rect().right_bottom() - Vec2::new(12.0, 12.0);
        let galley = ui.painter().layout_no_wrap(text, font.clone(), colours.muted);
        let rect = Rect::from_min_max(corner - galley.size() - Vec2::new(20.0, 8.0), corner);
        let painter = ui
            .ctx()
            .layer_painter(egui::LayerId::new(egui::Order::Foreground, tab.id().with("saving")));
        painter.rect(
            rect,
            12.0,
            colours.raised,
            Stroke::new(1.0, colours.border),
            StrokeKind::Inside,
        );
        painter.galley(rect.min + Vec2::new(10.0, 4.0), galley, colours.muted);
        ui.ctx().request_repaint();
    }
}

/// The table's views as tabs above it: pressing a tab shows that view, a right press on the one shown renames,
/// duplicates or deletes it, and a + adds one.
fn view_tabs(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours, font: &FontId) {
    let editable = tab.editable();
    let mut chosen = None;
    let mut gesture = None;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let views = tab.views.clone();
        for view in &views {
            if let Some((renaming, name)) = &mut tab.renaming
                && *renaming == view.id
            {
                let response = ui.add(egui::TextEdit::singleline(name).desired_width(140.0));
                response.request_focus();
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    tab.renaming = None;
                } else if response.lost_focus() {
                    let name = name.trim().to_owned();
                    if !name.is_empty() && name != view.name {
                        gesture = Some(TableGesture::new("renameView").view(&view.id).values([name]));
                    }
                    tab.renaming = None;
                }
                continue;
            }
            let active = view.id == tab.settings.view_id;
            let colour = if active { colours.text } else { colours.muted };
            let galley = ui.painter().layout_no_wrap(view.name.clone(), font.clone(), colour);
            let size = galley.size() + Vec2::new(20.0, 12.0);
            let (rect, response) = ui.allocate_exact_size(size, Sense::click());
            ui.painter().galley(rect.min + Vec2::new(10.0, 6.0), galley, colour);
            if active {
                ui.painter().line_segment(
                    [
                        rect.left_bottom() - Vec2::new(0.0, 1.0),
                        rect.right_bottom() - Vec2::new(0.0, 1.0),
                    ],
                    Stroke::new(2.0, colours.text),
                );
            }
            if response.clicked() && !active {
                chosen = Some(view.id.clone());
            }
            if editable && active {
                response.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        tab.renaming = Some((view.id.clone(), view.name.clone()));
                        ui.close();
                    }
                    if ui.button("Duplicate").clicked() {
                        gesture = Some(TableGesture::new("duplicateView").view(&view.id));
                        ui.close();
                    }
                    let last = views.len() <= 1;
                    let delete = ui
                        .add_enabled(!last, egui::Button::new("Delete"))
                        .on_disabled_hover_text("A table keeps at least one view.");
                    if delete.clicked() {
                        gesture = Some(TableGesture::new("deleteView").view(&view.id));
                        ui.close();
                    }
                });
            }
        }
        if editable
            && ui
                .add(egui::Button::new(RichText::new("+").color(colours.muted)).frame(false))
                .on_hover_text("Add a view")
                .clicked()
        {
            gesture = Some(TableGesture::new("addView"));
        }
    });
    let line = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), line, Stroke::new(1.0, colours.border));
    if let Some(view) = chosen {
        tab.show_view(&view);
    }
    if let Some(gesture) = gesture {
        tab.raise(gesture);
    }
}

/// What the view does - its sorts, its filter and its grouping - as three controls, each opening its panel.
fn view_bar(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours) {
    let editable = tab.editable();
    let settings = tab.settings.clone();
    let name_of = |id: &str| {
        tab.columns
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.name.clone())
            .unwrap_or_else(|| id.to_owned())
    };
    let filters = settings.filter.as_ref().map_or(0, |f| f.items.len());
    let labels = [
        (
            ViewPanel::Sort,
            if settings.sorts.is_empty() {
                "Sort".to_owned()
            } else {
                format!("Sort ({})", settings.sorts.len())
            },
        ),
        (
            ViewPanel::Filter,
            if filters == 0 {
                "Filter".to_owned()
            } else {
                format!("Filter ({filters})")
            },
        ),
        (
            ViewPanel::Group,
            if settings.group_by.is_empty() {
                "Group".to_owned()
            } else {
                format!("Group: {}", name_of(&settings.group_by))
            },
        ),
    ];
    let mut gesture: Option<TableGesture> = None;
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        for (panel, label) in labels {
            let open = tab.panel == Some(panel);
            let text = RichText::new(label).color(if open { colours.text } else { colours.muted });
            if ui.add(egui::Button::new(text).frame(open)).clicked() {
                tab.panel = if open { None } else { Some(panel) };
            }
        }
    });
    if let Some(panel) = tab.panel {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                ui.add_enabled_ui(editable, |ui| match panel {
                    ViewPanel::Sort => gesture = sort_panel(ui, tab, colours),
                    ViewPanel::Filter => gesture = filter_panel(ui, tab, colours),
                    ViewPanel::Group => gesture = group_panel(ui, tab),
                });
            });
    }
    ui.add_space(4.0);
    let line = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), line, Stroke::new(1.0, colours.border));
    if let Some(gesture) = gesture {
        tab.raise(gesture);
    }
}

fn sort_panel(ui: &mut egui::Ui, tab: &TableTab, colours: &Colours) -> Option<TableGesture> {
    let mut gesture = None;
    for sort in &tab.settings.sorts {
        let name = tab
            .columns
            .iter()
            .find(|c| c.id == sort.column_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label(name);
            let direction = if sort.descending { "Descending" } else { "Ascending" };
            if ui.button(direction).clicked() {
                gesture = Some(
                    TableGesture::new("setSort")
                        .column(&sort.column_id)
                        .setting("direction", if sort.descending { "ascending" } else { "descending" }),
                );
            }
            if ui
                .add(egui::Button::new(RichText::new("×").color(colours.muted)).frame(false))
                .on_hover_text("Remove this sort")
                .clicked()
            {
                gesture = Some(TableGesture::new("removeSort").column(&sort.column_id));
            }
        });
    }
    egui::ComboBox::from_id_salt(tab.id().with("add-sort"))
        .selected_text("Add a sort…")
        .show_ui(ui, |ui| {
            for column in &tab.columns {
                if tab.settings.sorts.iter().any(|s| s.column_id == column.id) {
                    continue;
                }
                if ui.selectable_label(false, &column.name).clicked() {
                    gesture = Some(TableGesture::new("addSort").column(&column.id));
                }
            }
        });
    gesture
}

fn filter_panel(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours) -> Option<TableGesture> {
    let mut gesture = None;
    let items = tab
        .settings
        .filter
        .as_ref()
        .map(|f| f.items.clone())
        .unwrap_or_default();
    for (index, item) in items.iter().enumerate() {
        ui.horizontal(|ui| {
            match item {
                TableFilterItem::Condition(condition) => {
                    let column = tab.columns.iter().find(|c| c.id == condition.column_id).cloned();
                    let kind = column.as_ref().map(|c| c.kind.clone()).unwrap_or_default();
                    ui.label(column.as_ref().map(|c| c.name.clone()).unwrap_or_default());
                    let mut comparisons: Vec<String> = adp_designer_knowledge::vocabulary::comparisons(&kind)
                        .unwrap_or_default()
                        .iter()
                        .map(|c| (*c).to_owned())
                        .collect();
                    comparisons.extend(["isEmpty".to_owned(), "isNotEmpty".to_owned()]);
                    egui::ComboBox::from_id_salt(tab.id().with(("comparison", index)))
                        .selected_text(comparison_label(&condition.comparison))
                        .show_ui(ui, |ui| {
                            for comparison in &comparisons {
                                if ui
                                    .selectable_label(*comparison == condition.comparison, comparison_label(comparison))
                                    .clicked()
                                    && *comparison != condition.comparison
                                {
                                    gesture = Some(
                                        TableGesture::new("setFilter")
                                            .target(index.to_string())
                                            .values(condition.values.clone())
                                            .setting("comparison", comparison.clone()),
                                    );
                                }
                            }
                        });
                    if takes_value(&condition.comparison) {
                        let shown = condition.values.first().cloned().unwrap_or_default();
                        let text = tab.filter_text.entry(index).or_insert_with(|| shown.clone());
                        let response = ui.add(egui::TextEdit::singleline(text).desired_width(140.0));
                        if response.lost_focus() {
                            let value = text.trim().to_owned();
                            tab.filter_text.remove(&index);
                            if value != shown {
                                gesture = Some(
                                    TableGesture::new("setFilter")
                                        .target(index.to_string())
                                        .values(if value.is_empty() { Vec::new() } else { vec![value] }),
                                );
                            }
                        } else if !response.has_focus() {
                            *text = shown;
                        }
                    }
                }
                TableFilterItem::Group(group) => {
                    ui.label(
                        RichText::new(format!(
                            "{} of {} conditions",
                            if group.any { "Any" } else { "All" },
                            group.items.len()
                        ))
                        .color(colours.muted),
                    );
                }
            }
            if ui
                .add(egui::Button::new(RichText::new("×").color(colours.muted)).frame(false))
                .on_hover_text("Remove from the filter")
                .clicked()
            {
                gesture = Some(TableGesture::new("removeFilter").target(index.to_string()));
            }
        });
    }
    egui::ComboBox::from_id_salt(tab.id().with("add-filter"))
        .selected_text("Add a filter…")
        .show_ui(ui, |ui| {
            for column in &tab.columns {
                if ui.selectable_label(false, &column.name).clicked() {
                    gesture = Some(TableGesture::new("addFilter").column(&column.id));
                }
            }
        });
    gesture
}

fn group_panel(ui: &mut egui::Ui, tab: &TableTab) -> Option<TableGesture> {
    let mut gesture = None;
    let current = tab
        .columns
        .iter()
        .find(|c| c.id == tab.settings.group_by)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "None".to_owned());
    ui.horizontal(|ui| {
        ui.label("Group by");
        egui::ComboBox::from_id_salt(tab.id().with("group-by"))
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui.selectable_label(tab.settings.group_by.is_empty(), "None").clicked()
                    && !tab.settings.group_by.is_empty()
                {
                    gesture = Some(TableGesture::new("groupBy"));
                }
                for column in tab.columns.iter().filter(|c| groupable(&c.kind)) {
                    let on = column.id == tab.settings.group_by;
                    if ui.selectable_label(on, &column.name).clicked() && !on {
                        gesture = Some(TableGesture::new("groupBy").column(&column.id));
                    }
                }
            });
        if !tab.settings.group_by.is_empty() {
            let mut hides = tab.settings.hides_empty_groups;
            if ui.checkbox(&mut hides, "Hide empty groups").changed() {
                gesture = Some(TableGesture::new("setHideEmptyGroups").setting("hide", hides.to_string()));
            }
        }
    });
    gesture
}

/// What a frame of the grid asked for, done once the rows are drawn.
#[derive(Default)]
struct Asked {
    gestures: Vec<TableGesture>,
    edit: Option<(usize, usize)>,
    click: Option<(usize, usize, String)>,
    commit: bool,
    cancel: bool,
    pick: Option<String>,
    create: bool,
    add_row: Option<String>,
}

/// The header and the window of lines: the header stays at the top while the rows scroll under it, and scrolls
/// sideways with them.
fn grid(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours, font: &FontId, footer: f32) {
    let columns: Vec<TableColumn> = tab.visible_columns().into_iter().cloned().collect();
    let widths: Vec<f32> = columns.iter().map(TableTab::width_of).collect();
    let editable = tab.editable();
    let total: f32 = widths.iter().sum::<f32>() + if editable { ADD_COLUMN } else { 0.0 };
    let (header, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let grid_id = tab.grid_id();
    let mut asked = Asked::default();

    let body_height = (ui.available_height() - footer).max(ROW_HEIGHT);
    let mut area = egui::ScrollArea::both()
        .id_salt(tab.id().with("rows"))
        .auto_shrink(false)
        .max_height(body_height);
    if let Some(top) = tab.scroll_to.take() {
        area = area.vertical_scroll_offset(top);
    }
    let output = area.show_rows(ui, ROW_HEIGHT, tab.row_count, |ui, range| {
        ui.set_min_width(total);
        let last = range.end.saturating_sub(1).max(range.start);
        tab.want(range.start, last);
        let body = ui.clip_rect();
        let focus = ui.interact(body, grid_id, Sense::focusable_noninteractive());
        let _ = focus;
        for index in range {
            let (rect, _) = ui.allocate_exact_size(Vec2::new(total, ROW_HEIGHT), Sense::hover());
            line(ui, tab, colours, font, index, rect, &columns, &widths, &mut asked);
        }
    });
    let offset = output.state.clamped_offset().x;
    header_row(ui, tab, colours, font, header, offset, &columns, &widths, &mut asked);
    if ui.memory(|m| m.has_focus(grid_id)) && tab.editing.is_none() {
        keyboard(ui, tab, columns.len(), &mut asked);
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                grid_id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: false,
                },
            )
        });
    }
    // The active cell is kept in sight.
    if let Some((line, _)) = tab.active
        && asked.edit.is_none()
        && ui.memory(|m| m.has_focus(grid_id))
    {
        let top = line as f32 * ROW_HEIGHT;
        let shown = output.state.clamped_offset().y;
        let height = output.inner_rect.height();
        if top < shown {
            tab.scroll_to = Some(top);
        } else if top + ROW_HEIGHT > shown + height {
            tab.scroll_to = Some(top + ROW_HEIGHT - height);
        }
    }
    apply(ui, tab, asked);
}

/// Carries out what the grid's frame asked for.
fn apply(ui: &mut egui::Ui, tab: &mut TableTab, asked: Asked) {
    if let Some((line, column, row)) = asked.click {
        tab.active = Some((line, column));
        tab.select_row(Some(row));
        ui.memory_mut(|m| m.request_focus(tab.grid_id()));
    }
    if asked.cancel {
        tab.editing = None;
        ui.memory_mut(|m| m.request_focus(tab.grid_id()));
    } else if asked.commit && tab.commit_edit() {
        ui.memory_mut(|m| m.request_focus(tab.grid_id()));
    }
    if let Some(id) = asked.pick {
        tab.pick(&id);
    }
    if asked.create {
        tab.create_option();
    }
    if let Some((line, column)) = asked.edit {
        tab.start_edit(line, column, None);
    }
    if let Some(group) = asked.add_row {
        tab.add_row(&group);
    }
    for gesture in asked.gestures {
        tab.raise(gesture);
    }
}

/// The header: one cell per visible column - its kind's mark and its name, its sort - with the menu of what can
/// be done to it, and the + that adds a property.
#[allow(clippy::too_many_arguments)]
fn header_row(
    ui: &mut egui::Ui,
    tab: &TableTab,
    colours: &Colours,
    font: &FontId,
    rect: Rect,
    offset: f32,
    columns: &[TableColumn],
    widths: &[f32],
    asked: &mut Asked,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, colours.surface);
    painter.hline(rect.x_range(), rect.bottom() - 0.5, Stroke::new(1.0, colours.border));
    let mut x = rect.left() - offset;
    let editable = tab.editable();
    for (index, (column, width)) in columns.iter().zip(widths).enumerate() {
        // The first column stays in sight while the table scrolls sideways: it names a row.
        let left = if index == 0 { rect.left() } else { x };
        let cell = Rect::from_min_size(Pos2::new(left, rect.top()), Vec2::new(*width, ROW_HEIGHT));
        x += width;
        if cell.right() < rect.left() || cell.left() > rect.right() {
            continue;
        }
        let painter = ui.painter_at(cell.intersect(rect));
        painter.rect_filled(cell, 0.0, colours.surface);
        painter.vline(cell.right() - 0.5, cell.y_range(), Stroke::new(1.0, colours.border));
        let (kind, mark) = kind_label(&column.kind);
        let sort = tab
            .settings
            .sorts
            .iter()
            .find(|s| s.column_id == column.id)
            .map(|s| if s.descending { " ↓" } else { " ↑" })
            .unwrap_or_default();
        painter.text(
            Pos2::new(cell.left() + CELL_PADDING, cell.center().y),
            Align2::LEFT_CENTER,
            format!("{mark}  {}{sort}", column.name),
            font.clone(),
            colours.muted,
        );
        let response = ui
            .interact(
                cell.intersect(rect),
                tab.id().with(("header", &column.id)),
                Sense::click(),
            )
            .on_hover_text(format!("{} · {kind}", column.name));
        response.context_menu(|ui| {
            if ui.button("Sort ascending").clicked() {
                asked.gestures.push(sort_gesture(tab, column, false));
                ui.close();
            }
            if ui.button("Sort descending").clicked() {
                asked.gestures.push(sort_gesture(tab, column, true));
                ui.close();
            }
            if groupable(&column.kind) && ui.button("Group by this").clicked() {
                asked.gestures.push(TableGesture::new("groupBy").column(&column.id));
                ui.close();
            }
            if editable && !column.is_title && ui.button("Hide").clicked() {
                asked.gestures.push(TableGesture::new("hideColumn").column(&column.id));
                ui.close();
            }
            if editable && !column.is_title && ui.button("Delete").clicked() {
                asked
                    .gestures
                    .push(TableGesture::new("deleteColumn").column(&column.id));
                ui.close();
            }
        });
    }
    if editable {
        let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(ADD_COLUMN, ROW_HEIGHT));
        if cell.left() < rect.right() {
            ui.scope_builder(egui::UiBuilder::new().max_rect(cell.intersect(rect)), |ui| {
                ui.menu_button(RichText::new("+").color(colours.muted), |ui| {
                    for kind in adp_designer_knowledge::vocabulary::VALUE_TYPES {
                        // A relation is added by choosing the table it relates to, which the explorer asks for.
                        if kind == "relation" {
                            continue;
                        }
                        if ui.button(kind_label(kind).0).clicked() {
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
}

/// A sort by one column alone, or that column's direction changed when the view sorts by it already.
fn sort_gesture(tab: &TableTab, column: &TableColumn, descending: bool) -> TableGesture {
    let direction = if descending { "descending" } else { "ascending" };
    let kind = if tab.settings.sorts.iter().any(|s| s.column_id == column.id) {
        "setSort"
    } else {
        "addSort"
    };
    TableGesture::new(kind)
        .column(&column.id)
        .setting("direction", direction)
}

/// One line of the view: a row, a group's heading, the line a row is added at, or a line not given yet.
#[allow(clippy::too_many_arguments)]
fn line(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    font: &FontId,
    index: usize,
    rect: Rect,
    columns: &[TableColumn],
    widths: &[f32],
    asked: &mut Asked,
) {
    let painter = ui.painter().clone();
    let Some(row) = tab.line(index).cloned() else {
        // A line the session has not given yet: an empty row of the same height, so the scroll bar never jumps.
        painter.rect_filled(rect, 0.0, colours.raised);
        return;
    };
    let visible = ui.clip_rect();
    let border = Stroke::new(1.0, colours.border);
    if row.is_group {
        painter.rect_filled(rect, 0.0, colours.raised);
        painter.hline(rect.x_range(), rect.bottom() - 0.5, border);
        let left = visible.left().max(rect.left()) + CELL_PADDING;
        let toggle = Rect::from_min_size(Pos2::new(left, rect.top()), Vec2::new(16.0, ROW_HEIGHT));
        painter.text(
            toggle.center(),
            Align2::CENTER_CENTER,
            if row.collapsed { "▸" } else { "▾" },
            font.clone(),
            colours.muted,
        );
        let label = painter.text(
            Pos2::new(toggle.right() + 6.0, rect.center().y),
            Align2::LEFT_CENTER,
            &row.label,
            FontId::proportional(font.size),
            colours.text,
        );
        painter.text(
            Pos2::new(label.right() + 8.0, rect.center().y),
            Align2::LEFT_CENTER,
            row.count.to_string(),
            font.clone(),
            colours.muted,
        );
        let response = ui.interact(toggle, tab.id().with(("group", index)), Sense::click());
        if tab.editable() && response.clicked() {
            asked.gestures.push(
                TableGesture::new("toggleGroup")
                    .target(&row.id)
                    .setting("collapsed", if row.collapsed { "false" } else { "true" }),
            );
        }
        return;
    }
    if row.is_new_row {
        painter.hline(rect.x_range(), rect.bottom() - 0.5, border);
        if tab.editable() {
            let left = visible.left().max(rect.left()) + CELL_PADDING;
            let shown = painter.text(
                Pos2::new(left, rect.center().y),
                Align2::LEFT_CENTER,
                "+  New",
                font.clone(),
                colours.muted,
            );
            let response = ui
                .interact(shown.expand(4.0), tab.id().with(("new", index)), Sense::click())
                .on_hover_text("Add a row");
            if response.clicked() {
                asked.add_row = Some(row.id.clone());
            }
        }
        return;
    }
    let selected = tab.selected.as_deref() == Some(row.id.as_str());
    if selected {
        painter.rect_filled(rect, 0.0, colours.selected);
    }
    painter.hline(rect.x_range(), rect.bottom() - 0.5, border);
    let mut x = rect.left();
    for (c, (column, width)) in columns.iter().zip(widths).enumerate() {
        let mut cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(*width, ROW_HEIGHT));
        x += width;
        if c == 0 && cell.left() < visible.left() {
            // The first column stays in sight, drawn over what scrolls under it.
            cell = cell.translate(Vec2::new(visible.left() - cell.left(), 0.0));
        }
        if cell.right() < visible.left() || cell.left() > visible.right() {
            continue;
        }
        let editing = tab.editing.as_ref().is_some_and(|e| e.line == index && e.column == c);
        cell_view(
            ui, tab, colours, font, index, c, cell, &row, column, selected, editing, asked,
        );
    }
}

/// One cell: its value, its finding, the active cell's outline, its editor while it is edited.
#[allow(clippy::too_many_arguments)]
fn cell_view(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    font: &FontId,
    index: usize,
    c: usize,
    cell: Rect,
    row: &TableRow,
    column: &TableColumn,
    selected: bool,
    editing: bool,
    asked: &mut Asked,
) {
    let painter = ui.painter_at(cell);
    if c == 0 {
        painter.rect_filled(cell, 0.0, if selected { colours.selected } else { colours.surface });
        painter.hline(cell.x_range(), cell.bottom() - 0.5, Stroke::new(1.0, colours.border));
    }
    painter.vline(cell.right() - 0.5, cell.y_range(), Stroke::new(1.0, colours.border));
    let id = tab.id().with(("cell", index, c));
    if editing {
        editor(ui, tab, colours, font, cell, row, column, asked);
        return;
    }
    let response = ui.interact(cell, id, Sense::click());
    let mut left = cell.left() + CELL_PADDING;
    if column.is_title && row.depth > 0 {
        left += row.depth as f32 * INDENT;
    }
    if column.is_title && row.has_children {
        let toggle = Rect::from_min_size(Pos2::new(left, cell.top()), Vec2::new(14.0, ROW_HEIGHT));
        painter.text(
            toggle.center(),
            Align2::CENTER_CENTER,
            if row.collapsed { "▸" } else { "▾" },
            font.clone(),
            colours.muted,
        );
        if tab.editable() && ui.interact(toggle, id.with("toggle"), Sense::click()).clicked() {
            asked.gestures.push(
                TableGesture::new("toggleRow")
                    .row(&row.id)
                    .setting("collapsed", if row.collapsed { "false" } else { "true" }),
            );
        }
        left += 18.0;
    }
    let value = row.cell(&column.id);
    let pending = value.is_some_and(|v| v.pending);
    let text_colour = if pending { colours.muted } else { colours.text };
    let text_font = if pending {
        FontId::new(font.size, egui::FontFamily::Proportional)
    } else {
        font.clone()
    };
    match EditorKind::of(&column.kind) {
        Some(EditorKind::Checkbox) => {
            let ticked = value.is_some_and(|v| v.values.first().is_some_and(|t| t == "true"));
            let mark = Rect::from_center_size(Pos2::new(left + 7.0, cell.center().y), Vec2::splat(14.0));
            painter.rect_stroke(mark, 2.0, Stroke::new(1.0, colours.muted), StrokeKind::Inside);
            if ticked {
                painter.line(
                    vec![
                        mark.left_center() + Vec2::new(3.0, 0.0),
                        mark.center_bottom() + Vec2::new(-1.0, -3.0),
                        mark.right_top() + Vec2::new(-3.0, 3.0),
                    ],
                    Stroke::new(2.0, text_colour),
                );
            }
        }
        Some(EditorKind::Option | EditorKind::Options) => {
            if let Some(value) = value {
                for (i, id) in value.values.iter().enumerate() {
                    // An option the column no longer has is still shown, by its label or its id.
                    let option = column.options.iter().find(|o| &o.id == id);
                    let name = option
                        .map(|o| o.name.clone())
                        .or_else(|| value.labels.get(i).filter(|l| !l.is_empty()).cloned())
                        .unwrap_or_else(|| id.clone());
                    let fill = colours.option(option.map(|o| o.color.as_str()).unwrap_or_default());
                    let galley = painter.layout_no_wrap(name, text_font.clone(), text_colour);
                    let tag = Rect::from_min_size(
                        Pos2::new(left, cell.center().y - galley.size().y / 2.0 - 2.0),
                        galley.size() + Vec2::new(16.0, 4.0),
                    );
                    painter.rect_filled(tag, 10.0, fill);
                    painter.galley(tag.min + Vec2::new(8.0, 2.0), galley, text_colour);
                    left = tag.right() + 4.0;
                }
            }
        }
        Some(EditorKind::Rows) => {
            if let Some(value) = value {
                for (i, id) in value.values.iter().enumerate() {
                    let label = value
                        .labels
                        .get(i)
                        .filter(|l| !l.is_empty())
                        .cloned()
                        .unwrap_or_else(|| id.clone());
                    let shown = painter.text(
                        Pos2::new(left, cell.center().y),
                        Align2::LEFT_CENTER,
                        label,
                        text_font.clone(),
                        text_colour,
                    );
                    painter.hline(shown.x_range(), shown.bottom(), Stroke::new(1.0, text_colour));
                    left = shown.right() + 6.0;
                }
            }
        }
        _ => {
            painter.text(
                Pos2::new(left, cell.center().y),
                Align2::LEFT_CENTER,
                cell_text(row, &column.id),
                text_font,
                text_colour,
            );
        }
    }
    // What the table is told about this cell: a line at its foot in the colour of the most serious, said on hover.
    let told: Vec<&adp_desl::table::model::TableFinding> = tab
        .findings
        .iter()
        .filter(|f| f.row_id == row.id && f.column_id == column.id)
        .collect();
    let response = if let Some(worst) = told.iter().max_by_key(|f| rank(f.severity)) {
        let colour = match worst.severity {
            adp_desl::table::model::TableFindingSeverity::Error => colours.danger,
            adp_desl::table::model::TableFindingSeverity::Warning => colours.warning,
            adp_desl::table::model::TableFindingSeverity::Info => colours.muted,
        };
        painter.hline(cell.x_range(), cell.bottom() - 1.0, Stroke::new(2.0, colour));
        let text = told.iter().map(|f| f.message.as_str()).collect::<Vec<_>>().join("\n");
        response.on_hover_text(text)
    } else {
        response
    };
    if tab.active == Some((index, c)) {
        painter.rect_stroke(
            cell.shrink(1.0),
            0.0,
            Stroke::new(2.0, colours.focus),
            StrokeKind::Inside,
        );
    }
    if response.double_clicked() {
        asked.edit = Some((index, c));
    } else if response.clicked() {
        asked.click = Some((index, c, row.id.clone()));
    }
    if tab.editable() {
        response.context_menu(|ui| {
            if ui.button("Delete row").clicked() {
                asked.gestures.push(TableGesture::new("deleteRows").row(&row.id));
                ui.close();
            }
        });
    }
}

/// The editor of the cell being edited, in place over it: a field for a typed value, a searchable list for a chosen
/// one. A refusal is shown under it, and it stays open.
#[allow(clippy::too_many_arguments)]
fn editor(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    font: &FontId,
    cell: Rect,
    row: &TableRow,
    column: &TableColumn,
    asked: &mut Asked,
) {
    let Some(editing) = tab.editing.as_mut() else {
        return;
    };
    let id = tab.id.with(("editor", &editing.row_id, &editing.column_id));
    let reason_at = cell.left_bottom();
    if !editing.editor.is_list() {
        ui.painter().rect_filled(cell, 0.0, colours.surface);
        let response = ui.put(
            cell.shrink2(Vec2::new(CELL_PADDING, 6.0)),
            egui::TextEdit::singleline(&mut editing.text)
                .id(id)
                .frame(egui::Frame::NONE)
                .font(font.clone())
                .hint_text(editing.editor.hint()),
        );
        if !editing.focused {
            response.request_focus();
            editing.focused = true;
        }
        ui.painter().rect_stroke(
            cell.shrink(1.0),
            0.0,
            Stroke::new(2.0, colours.focus),
            StrokeKind::Inside,
        );
        if response.changed() {
            editing.reason.clear();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            asked.cancel = true;
        } else if response.lost_focus() {
            asked.commit = true;
        }
        if !editing.reason.is_empty() {
            refusal_popup(ui, tab.id.with("refusal"), reason_at, &editing.reason.clone(), colours);
        }
        return;
    }
    // A list: the column's options - for a relation, the rows that can be related - and what the cell holds that
    // the list does not.
    let chosen: Vec<String> = row.cell(&column.id).map(|c| c.values.clone()).unwrap_or_default();
    let mut items: Vec<(String, String, String)> = column
        .options
        .iter()
        .map(|o| (o.id.clone(), o.name.clone(), o.color.clone()))
        .collect();
    if let Some(value) = row.cell(&column.id) {
        for (i, v) in value.values.iter().enumerate() {
            if !items.iter().any(|(id, _, _)| id == v) {
                let label = value
                    .labels
                    .get(i)
                    .filter(|l| !l.is_empty())
                    .cloned()
                    .unwrap_or_else(|| v.clone());
                items.push((v.clone(), label, String::new()));
            }
        }
    }
    let creates = editing.editor != EditorKind::Rows;
    let reason = editing.reason.clone();
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(cell.left_top())
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(cell.width().max(220.0));
                let field = ui.add(
                    egui::TextEdit::singleline(&mut editing.text)
                        .id(id.with("search"))
                        .hint_text(format!("Search {}", column.name)),
                );
                if !editing.focused {
                    field.request_focus();
                    editing.focused = true;
                }
                let search = editing.text.trim().to_lowercase();
                egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                    for (option, name, colour) in &items {
                        if !search.is_empty() && !name.to_lowercase().contains(&search) {
                            continue;
                        }
                        let on = chosen.contains(option);
                        let label = if on { format!("✔ {name}") } else { name.clone() };
                        let mut text = RichText::new(label).color(colours.text);
                        if !colour.is_empty() {
                            text = text.background_color(colours.option(colour));
                        }
                        if ui.selectable_label(on, text).clicked() {
                            asked.pick = Some(option.clone());
                        }
                    }
                });
                let exact = items.iter().any(|(_, n, _)| n.to_lowercase() == search);
                if creates
                    && !search.is_empty()
                    && !exact
                    && ui.button(format!("Create '{}'", editing.text.trim())).clicked()
                {
                    asked.create = true;
                }
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && creates && !exact {
                    asked.create = true;
                }
                if !reason.is_empty() {
                    ui.colored_label(colours.danger, &reason);
                }
            });
        });
    // Escape, or a press anywhere outside the list, leaves it: what was picked is already sent.
    let outside = ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p) && !cell.contains(p))
    });
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) || outside {
        asked.cancel = true;
    }
}

fn refusal_popup(ui: &egui::Ui, id: egui::Id, at: Pos2, reason: &str, colours: &Colours) {
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(at)
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(colours.danger_bg)
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.label(RichText::new(reason).color(colours.danger));
                });
        });
}

/// The keys of the grid while no editor is open (`cells/keyboard.ts`): the arrows and Tab move, Enter and F2 open the
/// editor, a character typed opens it with that character, Delete clears the cell, and Ctrl+Enter adds a row.
fn keyboard(ui: &mut egui::Ui, tab: &mut TableTab, columns: usize, asked: &mut Asked) {
    if tab.row_count == 0 || columns == 0 {
        return;
    }
    let (line, column) = tab.active.unwrap_or((tab.window.map_or(0, |w| w.0), 0));
    let rows = tab.row_count;
    let none = egui::Modifiers::NONE;
    let (up, down, left, right, enter, f2, tab_key, back_tab, delete, new_row, typed) = ui.input_mut(|i| {
        let typed = i.events.iter().find_map(|e| match e {
            egui::Event::Text(t) if !t.trim().is_empty() => Some(t.clone()),
            _ => None,
        });
        (
            i.consume_key(none, egui::Key::ArrowUp),
            i.consume_key(none, egui::Key::ArrowDown),
            i.consume_key(none, egui::Key::ArrowLeft),
            i.consume_key(none, egui::Key::ArrowRight),
            i.consume_key(none, egui::Key::Enter),
            i.consume_key(none, egui::Key::F2),
            i.consume_key(none, egui::Key::Tab),
            i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab),
            i.consume_key(none, egui::Key::Delete) || i.consume_key(none, egui::Key::Backspace),
            i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter),
            typed,
        )
    });
    let mut to = (line, column);
    if up {
        to.0 = line.saturating_sub(1);
    }
    if down {
        to.0 = (line + 1).min(rows - 1);
    }
    if left {
        to.1 = column.saturating_sub(1);
    }
    if right {
        to.1 = (column + 1).min(columns - 1);
    }
    if tab_key {
        to = if column + 1 < columns {
            (line, column + 1)
        } else if line + 1 < rows {
            (line + 1, 0)
        } else {
            to
        };
    }
    if back_tab {
        to = if column > 0 {
            (line, column - 1)
        } else if line > 0 {
            (line - 1, columns - 1)
        } else {
            to
        };
    }
    if to != (line, column) || tab.active.is_none() && (up || down || left || right) {
        tab.active = Some(to);
        if let Some(row) = tab.line(to.0).filter(|r| !r.is_group && !r.is_new_row) {
            tab.selected = Some(row.id.clone());
        }
        return;
    }
    if enter || f2 {
        asked.edit = Some((line, column));
    } else if new_row {
        let after = tab.line(line).map(|r| r.id.clone()).unwrap_or_default();
        asked.gestures.push(TableGesture::new("addRow").target(after));
    } else if delete {
        if let Some((row, col, editor)) = tab.editable_at(line, column)
            && row.cell(&col.id).is_some_and(|c| !c.values.is_empty())
        {
            let values: Vec<&str> = if editor == EditorKind::Checkbox {
                vec!["false"]
            } else {
                Vec::new()
            };
            asked
                .gestures
                .push(TableGesture::new("setCell").row(&row.id).column(&col.id).values(values));
        }
    } else if let Some(text) = typed
        && tab
            .editable_at(line, column)
            .is_some_and(|(_, _, e)| !e.is_list() && e != EditorKind::Checkbox)
    {
        tab.start_edit(line, column, Some(text));
    }
}

/// The table's own line: why a gesture was refused, or the question it was answered with and its two answers.
fn footer_line(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours) {
    let mut answer = None;
    egui::Frame::NONE
        .fill(colours.danger_bg)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some((_, question)) = &tab.asking {
                    ui.label(RichText::new(question).color(colours.text));
                    if ui.button("Delete").clicked() {
                        answer = Some(true);
                    }
                    if ui.button("Keep").clicked() {
                        answer = Some(false);
                    }
                } else {
                    ui.label(RichText::new(&tab.refusal).color(colours.danger));
                }
            });
        });
    match answer {
        Some(true) => tab.confirm(),
        Some(false) => tab.asking = None,
        None => {}
    }
}

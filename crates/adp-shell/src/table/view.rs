//! Drawing a [`TableTab`]: the read-only note, the view tabs and the view bar above the table, the bar of the ticked
//! rows, the sticky header, the window of lines in a `ScrollArea::show_viewport`, the cell editors, and at the table's
//! foot the line of calculations, a refusal and a question (the standalone host's `TableSurface.tsx`, `ViewTabs.tsx`,
//! `ViewBar.tsx`, `GroupEditor.tsx`, `CellEditor.tsx`, `CalculationRow.tsx`, `GroupHeading.tsx`, `NewRow.tsx` and
//! `OptionTag.tsx` at 8b6ccfc, in `table.css`'s layout and VS Code's colours).

use adp_desl::table::model::{TableColumn, TableFilterItem, TableGesture, TableRow};
use adp_theme::{Kind, Theme};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};

use super::header::{self, ADD_COLUMN};
use super::{EditorKind, ROW_HEIGHT, TableTab, ViewPanel, groupable};
use crate::theme::{self, Palette};

/// A cell's padding, and the step a nested row is set in by (`CELL_PADDING`, `INDENT`).
pub(super) const CELL_PADDING: f32 = 8.0;
const INDENT: f32 = 20.0;
/// The room a row's checkbox takes before its first value.
const CHECK_ROOM: f32 = 18.0;

/// The colours the table draws with: VS Code's, by role.
pub(super) struct Colours {
    pub(super) surface: Color32,
    pub(super) text: Color32,
    pub(super) muted: Color32,
    pub(super) border: Color32,
    pub(super) raised: Color32,
    pub(super) selected: Color32,
    pub(super) focus: Color32,
    pub(super) danger: Color32,
    pub(super) danger_bg: Color32,
    pub(super) warning: Color32,
    pub(super) warning_bg: Color32,
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
    pub(super) fn option(&self, name: &str) -> Color32 {
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

/// What a calculation is called, in its menu and beside its figure: Notion's names (`CalculationRow.tsx`).
pub fn calculation_label(calculation: &str) -> &str {
    match calculation {
        "countAll" => "Count all",
        "countValues" => "Count values",
        "countUnique" => "Count unique values",
        "countEmpty" => "Count empty",
        "countNotEmpty" => "Count not empty",
        "percentEmpty" => "Percent empty",
        "percentNotEmpty" => "Percent not empty",
        "sum" => "Sum",
        "average" => "Average",
        "median" => "Median",
        "min" => "Min",
        "max" => "Max",
        "range" => "Range",
        "earliest" => "Earliest date",
        "latest" => "Latest date",
        "dateRange" => "Date range",
        other => other,
    }
}

/// The calculations a column says it can be given (`calculationsOf`).
pub fn calculations_of(column: &TableColumn) -> Vec<String> {
    column
        .settings
        .get("calculations")
        .map(|c| c.split(',').filter(|c| !c.is_empty()).map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Whether the table has a line of calculations: some column has one, or can be given one in a table that can be
/// edited (`showsCalculations`).
pub fn shows_calculations(columns: &[TableColumn], editable: bool) -> bool {
    columns
        .iter()
        .any(|c| c.settings.get("calculate").is_some_and(|c| !c.is_empty()))
        || (editable && columns.iter().any(|c| !calculations_of(c).is_empty()))
}

/// Draws the tab: everything above the table, and the table with its foot.
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
    if tab.editable() && !tab.checked.is_empty() {
        selection_bar(ui, tab, &colours);
    }
    grid(ui, tab, &colours, &font);
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
/// duplicates or deletes it, and a + adds one. A view is renamed in its tab: Enter or a press elsewhere keeps the
/// name, Escape drops it, and either gives the keyboard back to the table.
fn view_tabs(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours, font: &FontId) {
    let editable = tab.editable();
    let mut chosen = None;
    let mut gesture = None;
    let mut give_back = false;
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        let views = tab.views.clone();
        for view in &views {
            let field = tab.id().with(("rename-view", &view.id));
            if let Some(renaming) = &mut tab.renaming
                && renaming.id == view.id
            {
                let response = ui.add(
                    egui::TextEdit::singleline(&mut renaming.name)
                        .id(field)
                        .desired_width(140.0),
                );
                // The focus is asked for once: asked for again each frame, it would never be lost to Enter.
                if !renaming.focused {
                    response.request_focus();
                    renaming.focused = true;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    tab.renaming = None;
                    give_back = true;
                } else if response.lost_focus() {
                    let name = renaming.name.trim().to_owned();
                    if !name.is_empty() && name != view.name {
                        gesture = Some(TableGesture::new("renameView").view(&view.id).values([name]));
                    }
                    tab.renaming = None;
                    give_back = true;
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
                        tab.start_view_rename(&view.id);
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
    if give_back {
        // The field that was left holds the keyboard no more: undo and the arrows are the table's again. The key that
        // left it was the field's, and opens no cell's editor.
        ui.input_mut(|i| {
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter);
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
        });
        ui.memory_mut(|m| m.request_focus(tab.grid_id()));
    }
    if let Some(view) = chosen {
        tab.show_view(&view);
    }
    if let Some(gesture) = gesture {
        tab.raise(gesture);
    }
}

/// What the view does - which properties it shows, its sorts, its filter and its grouping - as four controls, each
/// opening its panel.
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
    let hidden = tab.columns.iter().filter(|c| !c.visible).count();
    let labels = [
        (
            ViewPanel::Properties,
            if hidden == 0 {
                "Properties".to_owned()
            } else {
                format!("Properties ({hidden} hidden)")
            },
        ),
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
                    ViewPanel::Properties => gesture = properties_panel(ui, tab),
                    ViewPanel::Sort => gesture = sort_panel(ui, tab, colours),
                    ViewPanel::Filter => gesture = filter_panel(ui, tab, colours),
                    ViewPanel::Group => gesture = group_panel(ui, tab, colours),
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

/// Which properties the view shows: a tick per property, the one that names a row always ticked
/// (`PropertyVisibility`).
fn properties_panel(ui: &mut egui::Ui, tab: &TableTab) -> Option<TableGesture> {
    let mut gesture = None;
    ui.horizontal_wrapped(|ui| {
        for column in &tab.columns {
            let mut shown = column.visible;
            let response = ui.add_enabled(!column.is_title, egui::Checkbox::new(&mut shown, &column.name));
            if response.changed() {
                let kind = if shown { "showColumn" } else { "hideColumn" };
                gesture = Some(TableGesture::new(kind).column(&column.id));
            }
        }
    });
    gesture
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
                    let options = column.as_ref().map(|c| c.options.clone()).unwrap_or_default();
                    if takes_value(&condition.comparison) && matches!(kind.as_str(), "selection" | "multipleSelection")
                    {
                        // An option is chosen by its name; the filter holds its id.
                        let shown = condition.values.first().cloned().unwrap_or_default();
                        let name = options
                            .iter()
                            .find(|o| o.id == shown)
                            .map(|o| o.name.clone())
                            .unwrap_or_else(|| shown.clone());
                        egui::ComboBox::from_id_salt(tab.id().with(("filter-option", index)))
                            .selected_text(name)
                            .show_ui(ui, |ui| {
                                for option in &options {
                                    if ui.selectable_label(option.id == shown, &option.name).clicked()
                                        && option.id != shown
                                    {
                                        gesture = Some(
                                            TableGesture::new("setFilter")
                                                .target(index.to_string())
                                                .values([option.id.clone()]),
                                        );
                                    }
                                }
                            });
                    } else if takes_value(&condition.comparison) {
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

/// One group of a grouped view, as the view lists it (`ListedGroup`).
struct ListedGroup {
    key: String,
    label: String,
    hidden: bool,
}

/// The groups a view says it has, in its order; none when it says nothing that reads as a list of them
/// (`listedGroups`).
fn listed_groups(settings: &adp_desl::table::model::TableViewSettings) -> Vec<ListedGroup> {
    let parsed: Option<serde_json::Value> = settings
        .settings
        .get(adp_desl::table::mapper::GROUPS)
        .and_then(|g| serde_json::from_str(g).ok());
    parsed
        .as_ref()
        .and_then(serde_json::Value::as_array)
        .map(|groups| {
            groups
                .iter()
                .filter_map(|group| {
                    Some(ListedGroup {
                        key: group.get("key")?.as_str()?.to_owned(),
                        label: group.get("label")?.as_str()?.to_owned(),
                        hidden: group
                            .get("hidden")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The view's grouping: what it groups by, whether a group without rows is left out, and its groups one by one, each
/// shown or hidden (`GroupEditor.tsx`).
fn group_panel(ui: &mut egui::Ui, tab: &TableTab, colours: &Colours) -> Option<TableGesture> {
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
    let groups = listed_groups(&tab.settings);
    if !groups.is_empty() {
        ui.horizontal_wrapped(|ui| {
            for group in &groups {
                let colour = if group.hidden { colours.muted } else { colours.text };
                ui.label(RichText::new(&group.label).color(colour));
                let (kind, label) = if group.hidden {
                    ("showGroup", "Show")
                } else {
                    ("hideGroup", "Hide")
                };
                if ui
                    .small_button(label)
                    .on_hover_text(format!("{label} {}", group.label))
                    .clicked()
                {
                    gesture = Some(TableGesture::new(kind).target(&group.key));
                }
                ui.add_space(8.0);
            }
        });
    }
    let hidden: usize = tab
        .settings
        .settings
        .get(adp_desl::table::mapper::HIDDEN_GROUPS)
        .and_then(|h| h.parse().ok())
        .unwrap_or(0);
    if hidden > 0 {
        // Groups hidden one by one are brought back together.
        let label = if hidden == 1 {
            "Show 1 hidden group".to_owned()
        } else {
            format!("Show {hidden} hidden groups")
        };
        if ui.button(label).clicked() {
            gesture = Some(TableGesture::new("showGroups"));
        }
    }
    gesture
}

/// What is done to the ticked rows together: a property set for all of them, or all of them deleted
/// (`table-selection-bar`).
fn selection_bar(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours) {
    enum Done {
        Delete,
        Clear,
        Set(String, Vec<String>),
    }
    let mut done = None;
    let count = tab.checked.len();
    let editable_columns: Vec<TableColumn> = tab
        .columns
        .iter()
        .filter(|c| EditorKind::of(&c.kind).is_some() && c.settings.get("computed").is_none_or(|v| v != "true"))
        .cloned()
        .collect();
    egui::Frame::NONE
        .fill(colours.raised)
        .inner_margin(egui::Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
            ui.horizontal(|ui| {
                ui.label(if count == 1 {
                    "1 row selected".to_owned()
                } else {
                    format!("{count} rows selected")
                });
                let chosen = tab
                    .bulk
                    .as_ref()
                    .and_then(|b| editable_columns.iter().find(|c| c.id == *b))
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "Edit a property…".to_owned());
                egui::ComboBox::from_id_salt(tab.id().with("bulk"))
                    .selected_text(chosen)
                    .show_ui(ui, |ui| {
                        for column in &editable_columns {
                            let on = tab.bulk.as_deref() == Some(column.id.as_str());
                            if ui.selectable_label(on, &column.name).clicked() && !on {
                                tab.bulk = Some(column.id.clone());
                                tab.bulk_text.clear();
                            }
                        }
                    });
                let delete = if count == 1 {
                    "Delete row".to_owned()
                } else {
                    format!("Delete {count} rows")
                };
                if ui.button(delete).clicked() {
                    done = Some(Done::Delete);
                }
                if ui.button("Clear selection").clicked() {
                    done = Some(Done::Clear);
                }
            });
            let Some(column) = tab
                .bulk
                .as_ref()
                .and_then(|b| editable_columns.iter().find(|c| c.id == *b))
                .cloned()
            else {
                return;
            };
            let Some(editor) = EditorKind::of(&column.kind) else {
                return;
            };
            ui.horizontal_wrapped(|ui| {
                if editor == EditorKind::Checkbox {
                    if ui.button("Check").clicked() {
                        done = Some(Done::Set(column.id.clone(), vec!["true".to_owned()]));
                    }
                    if ui.button("Uncheck").clicked() {
                        done = Some(Done::Set(column.id.clone(), Vec::new()));
                    }
                } else if editor.is_list() {
                    for option in &column.options {
                        let mut text = RichText::new(&option.name).color(colours.text);
                        if !option.color.is_empty() {
                            text = text.background_color(colours.option(&option.color));
                        }
                        if ui.selectable_label(false, text).clicked() {
                            done = Some(Done::Set(column.id.clone(), vec![option.id.clone()]));
                        }
                    }
                    if ui.button("Clear").clicked() {
                        done = Some(Done::Set(column.id.clone(), Vec::new()));
                    }
                } else {
                    let field = ui.add(
                        egui::TextEdit::singleline(&mut tab.bulk_text)
                            .hint_text(editor.hint())
                            .desired_width(200.0),
                    );
                    if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let value = tab.bulk_text.trim().to_owned();
                        let values = if value.is_empty() { Vec::new() } else { vec![value] };
                        done = Some(Done::Set(column.id.clone(), values));
                    }
                }
            });
        });
    match done {
        Some(Done::Delete) => {
            let rows = tab.checked.clone();
            tab.delete_rows(&rows);
        }
        Some(Done::Clear) => {
            tab.checked.clear();
            tab.bulk = None;
        }
        Some(Done::Set(column, values)) => tab.set_checked(&column, values),
        None => {}
    }
}

/// What a frame of the grid asked for, done once the rows are drawn.
#[derive(Default)]
pub(super) struct Asked {
    pub(super) gestures: Vec<TableGesture>,
    edit: Option<(usize, usize)>,
    click: Option<(usize, usize, String)>,
    commit: bool,
    cancel: bool,
    pick: Option<String>,
    create: bool,
    add_row: Option<String>,
    /// A rename of a column ended in its header: `Some(None)` when dropped, else the column, the name and whether
    /// the field was left rather than Enter pressed.
    pub(super) column_rename: Option<Option<(String, String, bool)>>,
    /// The question a gesture was answered with, answered.
    answer: Option<bool>,
    /// What lies where on the lines drawn, the later over the earlier: the grid is one widget, and a press is found
    /// on it by place, so a line makes no widget of its own - egui would keep an id per line ever drawn.
    hits: Vec<(Rect, Hit)>,
}

/// What a press on the grid is on.
#[derive(Debug, Clone)]
enum Hit {
    Cell {
        line: usize,
        column: usize,
        row_id: String,
        /// What the table is told about the cell, said when it is pointed at.
        told: String,
    },
    Group {
        key: String,
        collapsed: bool,
    },
    NewRow {
        group: String,
    },
    RowToggle {
        row_id: String,
        collapsed: bool,
    },
    /// A row's checkbox, which ticks it to be acted on with others.
    RowCheck {
        row_id: String,
    },
    /// The lines at the table's foot, over the rows: a press there is none of theirs.
    Foot,
}

/// A line at the table's foot, after the rows: it stays in sight at the bottom while the rows scroll under it
/// (`table-calculation-row`, `table-refusal` and `table-ask`, `position: sticky; bottom: 0`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Foot {
    Calculations,
    Refusal,
    Asking,
}

/// The header and the window of lines, and the lines at the table's foot: the header stays at the top while the rows
/// scroll under it and scrolls sideways with them, and the first column and those the view freezes stay in sight.
fn grid(ui: &mut egui::Ui, tab: &mut TableTab, colours: &Colours, font: &FontId) {
    let columns: Vec<TableColumn> = tab.visible_columns().into_iter().cloned().collect();
    let widths: Vec<f32> = columns.iter().map(|c| tab.drawn_width(c)).collect();
    let editable = tab.editable();
    let total: f32 = widths.iter().sum::<f32>() + if editable { ADD_COLUMN } else { 0.0 };
    // How far from the left each frozen column stays: the first always, and those up to the one the view names.
    let frozen_up_to = tab.frozen_up_to();
    let frozen: Vec<Option<f32>> = (0..columns.len())
        .map(|i| (i == 0 || frozen_up_to.is_some_and(|up| i <= up)).then(|| widths[..i].iter().sum()))
        .collect();
    let mut feet = Vec::new();
    if shows_calculations(&columns, editable) {
        feet.push(Foot::Calculations);
    }
    if !tab.refusal.is_empty() {
        feet.push(Foot::Refusal);
    }
    if tab.asking.is_some() {
        feet.push(Foot::Asking);
    }
    let (header, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::hover());
    let grid_id = tab.grid_id();
    let mut asked = Asked::default();
    tab.drawn.calculations = None;
    tab.drawn.refusal = None;
    tab.drawn.asking = None;

    let rows = tab.row_count;
    let height = (rows + feet.len()) as f32 * ROW_HEIGHT;
    let mut area = egui::ScrollArea::both()
        .id_salt(tab.id().with("rows"))
        .auto_shrink(false);
    if let Some(top) = tab.scroll_to.take() {
        area = area.vertical_scroll_offset(top);
    }
    let output = area.show_viewport(ui, |ui, viewport| {
        ui.set_min_size(Vec2::new(total, height));
        // Each line's place follows from its index; nothing is allocated per line, so a line makes no id.
        let origin = ui.max_rect().left_top();
        let clip = ui.clip_rect();
        tab.drawn.origin = origin;
        tab.drawn.clip = clip;
        let first = (viewport.top() / ROW_HEIGHT).floor().max(0.0) as usize;
        let end = ((viewport.bottom() / ROW_HEIGHT).ceil().max(0.0) as usize).min(rows);
        if rows > 0 {
            let first = first.min(rows - 1);
            tab.want(first, end.saturating_sub(1).max(first));
        }
        // The grid is one widget under every line, so what is drawn over it - an editor - takes its own presses.
        let response = ui.interact(clip, grid_id, Sense::click());
        let offsets: Vec<f32> = widths
            .iter()
            .scan(0.0, |x, w| {
                let at = *x;
                *x += w;
                Some(at)
            })
            .collect();
        let layout = Layout {
            columns: &columns,
            widths: &widths,
            offsets: &offsets,
            frozen: &frozen,
            frozen_up_to,
        };
        for index in first..end {
            let top = origin + Vec2::new(0.0, index as f32 * ROW_HEIGHT);
            let rect = Rect::from_min_size(top, Vec2::new(total, ROW_HEIGHT));
            line(ui, tab, colours, font, index, rect, &layout, &mut asked);
        }
        // The lines at the foot: after the rows, and at the bottom of what is in sight while the rows go on under it.
        let count = feet.len();
        for (k, foot) in feet.iter().enumerate() {
            let natural = origin.y + (rows + k) as f32 * ROW_HEIGHT;
            let stuck = clip.bottom() - (count - k) as f32 * ROW_HEIGHT;
            let top = natural.min(stuck);
            let rect = Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(total.max(clip.width()), ROW_HEIGHT));
            asked.hits.push((rect.intersect(clip), Hit::Foot));
            match foot {
                Foot::Calculations => {
                    tab.drawn.calculations = Some(rect);
                    calculation_line(ui, tab, colours, font, rect, &layout, &mut asked);
                }
                Foot::Refusal | Foot::Asking => {
                    // Kept at the left of what is in sight, as wide as it.
                    let line = Rect::from_min_size(Pos2::new(clip.left(), top), Vec2::new(clip.width(), ROW_HEIGHT));
                    if *foot == Foot::Refusal {
                        tab.drawn.refusal = Some(line);
                    } else {
                        tab.drawn.asking = Some(line);
                    }
                    foot_line(ui, tab, colours, line, *foot, &mut asked);
                }
            }
        }
        pointer(tab, response, &mut asked);
    });
    let offset = output.state.clamped_offset().x;
    header::header_row(
        ui, tab, colours, font, header, offset, &columns, &widths, &frozen, &mut asked,
    );
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
        let height = output.inner_rect.height() - feet.len() as f32 * ROW_HEIGHT;
        if top < shown {
            tab.scroll_to = Some(top);
        } else if top + ROW_HEIGHT > shown + height {
            tab.scroll_to = Some(top + ROW_HEIGHT - height);
        }
    }
    apply(ui, tab, asked);
}

/// Where the columns stand on a line: their widths, how far each is from the first, and how far from the left each
/// frozen one stays.
struct Layout<'a> {
    columns: &'a [TableColumn],
    widths: &'a [f32],
    offsets: &'a [f32],
    frozen: &'a [Option<f32>],
    frozen_up_to: Option<usize>,
}

impl Layout<'_> {
    /// The columns in the order they are drawn: those that scroll, then the frozen ones over them.
    fn order(&self) -> impl Iterator<Item = usize> + '_ {
        let n = self.columns.len();
        (0..n)
            .filter(|i| self.frozen[*i].is_none())
            .chain((0..n).filter(|i| self.frozen[*i].is_some()))
    }

    /// A column's cell on a line drawn at `line`, with what is in sight at `visible`.
    fn cell(&self, index: usize, line: Rect, visible: Rect) -> Rect {
        let natural = line.left() + self.offsets[index];
        let left = match self.frozen[index] {
            Some(at) => natural.max(visible.left() + at),
            None => natural,
        };
        Rect::from_min_size(
            Pos2::new(left, line.top()),
            Vec2::new(self.widths[index], line.height()),
        )
    }
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
    if let Some(ended) = asked.column_rename
        && header::finish_rename(tab, ended)
    {
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
    match asked.answer {
        Some(true) => tab.confirm(),
        Some(false) => tab.asking = None,
        None => {}
    }
    for gesture in asked.gestures {
        tab.raise(gesture);
    }
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
    layout: &Layout<'_>,
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
        chevron(&painter, toggle.center(), !row.collapsed, colours.muted);
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
        asked.hits.push((
            toggle,
            Hit::Group {
                key: row.id.clone(),
                collapsed: row.collapsed,
            },
        ));
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
            asked
                .hits
                .push((shown.expand(4.0), Hit::NewRow { group: row.id.clone() }));
        }
        return;
    }
    let selected = tab.selected.as_deref() == Some(row.id.as_str()) || tab.checked.contains(&row.id);
    if selected {
        painter.rect_filled(rect, 0.0, colours.selected);
    }
    painter.hline(rect.x_range(), rect.bottom() - 0.5, border);
    let hovered = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| rect.contains(p) && visible.contains(p));
    for c in layout.order() {
        let cell = layout.cell(c, rect, visible);
        if cell.right() < visible.left() || cell.left() > visible.right() {
            continue;
        }
        let editing = tab.editing.as_ref().is_some_and(|e| e.line == index && e.column == c);
        let place = Place {
            index,
            column: c,
            cell,
            frozen: layout.frozen[c].is_some(),
            edge: layout.frozen_up_to == Some(c) && c > 0,
            selected,
            hovered,
        };
        cell_view(ui, tab, colours, font, &place, &row, &layout.columns[c], editing, asked);
    }
}

/// Where a cell is drawn, and what of its row and column it is drawn as.
struct Place {
    index: usize,
    column: usize,
    cell: Rect,
    /// Kept in sight while the table scrolls sideways: drawn opaque, over what scrolls under it.
    frozen: bool,
    /// The last of the frozen columns, drawn with a heavier line at its right.
    edge: bool,
    selected: bool,
    hovered: bool,
}

/// One cell: its value, its finding, the active cell's outline, its editor while it is edited.
#[allow(clippy::too_many_arguments)]
fn cell_view(
    ui: &mut egui::Ui,
    tab: &mut TableTab,
    colours: &Colours,
    font: &FontId,
    place: &Place,
    row: &TableRow,
    column: &TableColumn,
    editing: bool,
    asked: &mut Asked,
) {
    let Place {
        index, column: c, cell, ..
    } = *place;
    let painter = ui.painter_at(cell);
    if place.frozen {
        painter.rect_filled(
            cell,
            0.0,
            if place.selected {
                colours.selected
            } else {
                colours.surface
            },
        );
        painter.hline(cell.x_range(), cell.bottom() - 0.5, Stroke::new(1.0, colours.border));
    }
    if place.edge {
        painter.vline(cell.right() - 1.0, cell.y_range(), Stroke::new(2.0, colours.border));
    } else {
        painter.vline(cell.right() - 0.5, cell.y_range(), Stroke::new(1.0, colours.border));
    }
    if editing {
        editor(ui, tab, colours, font, cell, row, column, asked);
        return;
    }
    let hit = asked.hits.len();
    asked.hits.push((
        cell,
        Hit::Cell {
            line: index,
            column: c,
            row_id: row.id.clone(),
            told: String::new(),
        },
    ));
    let mut left = cell.left() + CELL_PADDING;
    if c == 0 && tab.editable() {
        // As in Notion: a checkbox that ticks the row, seen where the pointer is or once it is ticked.
        let checked = tab.checked.contains(&row.id);
        let mark = Rect::from_center_size(Pos2::new(left + 7.0, cell.center().y), Vec2::splat(14.0));
        if checked || place.hovered {
            check_box(&painter, mark, checked, colours);
        }
        asked
            .hits
            .push((mark.expand(2.0), Hit::RowCheck { row_id: row.id.clone() }));
        left += CHECK_ROOM;
    }
    if column.is_title && row.depth > 0 {
        left += row.depth as f32 * INDENT;
    }
    if column.is_title && row.has_children {
        let toggle = Rect::from_min_size(Pos2::new(left, cell.top()), Vec2::new(14.0, ROW_HEIGHT));
        chevron(&painter, toggle.center(), !row.collapsed, colours.muted);
        asked.hits.push((
            toggle,
            Hit::RowToggle {
                row_id: row.id.clone(),
                collapsed: row.collapsed,
            },
        ));
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
        // Every other kind is its text - a relation's rows too, by their titles: the Knowledge designer passes no
        // way to open a related row, so they are not drawn as links (`CellValue`'s "text").
        _ => {
            let text = cell_text(row, &column.id);
            if column.wraps {
                let width = (cell.right() - CELL_PADDING - left).max(1.0);
                let galley = painter.layout(text, text_font, text_colour, width);
                let top = if galley.size().y < cell.height() {
                    cell.center().y - galley.size().y / 2.0
                } else {
                    cell.top() + 2.0
                };
                painter.galley(Pos2::new(left, top), galley, text_colour);
            } else {
                painter.text(
                    Pos2::new(left, cell.center().y),
                    Align2::LEFT_CENTER,
                    text,
                    text_font,
                    text_colour,
                );
            }
        }
    }
    // What the table is told about this cell: a line at its foot in the colour of the most serious, said on hover.
    let told: Vec<&adp_desl::table::model::TableFinding> = tab
        .findings
        .iter()
        .filter(|f| f.row_id == row.id && f.column_id == column.id)
        .collect();
    if let Some(worst) = told.iter().max_by_key(|f| rank(f.severity)) {
        let colour = match worst.severity {
            adp_desl::table::model::TableFindingSeverity::Error => colours.danger,
            adp_desl::table::model::TableFindingSeverity::Warning => colours.warning,
            adp_desl::table::model::TableFindingSeverity::Info => colours.muted,
        };
        painter.hline(cell.x_range(), cell.bottom() - 1.0, Stroke::new(2.0, colour));
        let text = told.iter().map(|f| f.message.as_str()).collect::<Vec<_>>().join("\n");
        if let Some((_, Hit::Cell { told, .. })) = asked.hits.get_mut(hit) {
            *told = text;
        }
    }
    if tab.active == Some((index, c)) {
        painter.rect_stroke(
            cell.shrink(1.0),
            0.0,
            Stroke::new(2.0, colours.focus),
            StrokeKind::Inside,
        );
    }
}

/// A checkbox's box, ticked or not, in the table's colours.
pub(super) fn check_box(painter: &egui::Painter, mark: Rect, ticked: bool, colours: &Colours) {
    if ticked {
        painter.rect_filled(mark, 2.0, colours.focus);
        painter.line(
            vec![
                mark.left_center() + Vec2::new(3.0, 0.0),
                mark.center_bottom() + Vec2::new(-1.0, -3.0),
                mark.right_top() + Vec2::new(-3.0, 3.0),
            ],
            Stroke::new(2.0, colours.surface),
        );
    } else {
        painter.rect(
            mark,
            2.0,
            colours.surface,
            Stroke::new(1.0, colours.muted),
            StrokeKind::Inside,
        );
    }
}

/// The line under the columns, as in Notion: per column, what it calculates over the rows and what that comes to.
/// Pressing a column's place opens the list of the calculations its type has (`CalculationRow.tsx`).
#[allow(clippy::too_many_arguments)]
fn calculation_line(
    ui: &mut egui::Ui,
    tab: &TableTab,
    colours: &Colours,
    font: &FontId,
    rect: Rect,
    layout: &Layout<'_>,
    asked: &mut Asked,
) {
    let visible = ui.clip_rect();
    let painter = ui.painter_at(visible);
    painter.rect_filled(rect.intersect(visible), 0.0, colours.surface);
    painter.hline(rect.x_range(), rect.top() + 0.5, Stroke::new(1.0, colours.border));
    let small = FontId::proportional((font.size - 2.0).max(8.0));
    let editable = tab.editable();
    for c in layout.order() {
        let column = &layout.columns[c];
        let cell = layout.cell(c, rect, visible);
        if cell.right() < visible.left() || cell.left() > visible.right() {
            continue;
        }
        let painter = ui.painter_at(cell.intersect(visible));
        if layout.frozen[c].is_some() {
            painter.rect_filled(cell, 0.0, colours.surface);
            painter.hline(cell.x_range(), cell.top() + 0.5, Stroke::new(1.0, colours.border));
        }
        let calculate = column.settings.get("calculate").cloned().unwrap_or_default();
        let offered = calculations_of(column);
        let response = (editable && !offered.is_empty()).then(|| {
            ui.interact(
                cell.intersect(visible),
                tab.id().with(("calculate", &column.id)),
                Sense::click(),
            )
        });
        let at = Pos2::new(
            cell.left() + CELL_PADDING + if c == 0 && editable { CHECK_ROOM } else { 0.0 },
            cell.center().y,
        );
        if calculate.is_empty() {
            // A column that calculates nothing says so only where the pointer is.
            if response.as_ref().is_some_and(|r| r.hovered()) {
                painter.text(at, Align2::LEFT_CENTER, "Calculate", small.clone(), colours.muted);
            }
        } else {
            let label = painter.text(
                at,
                Align2::LEFT_CENTER,
                calculation_label(&calculate).to_uppercase(),
                small.clone(),
                colours.muted,
            );
            let figure = column.settings.get("calculation").cloned().unwrap_or_default();
            painter.text(
                Pos2::new(label.right() + 6.0, cell.center().y),
                Align2::LEFT_CENTER,
                figure,
                font.clone(),
                colours.text,
            );
        }
        if let Some(response) = response {
            let response = response.on_hover_text(format!("Calculate {}", column.name));
            egui::Popup::menu(&response).show(|ui| {
                if ui.selectable_label(calculate.is_empty(), "None").clicked() {
                    asked.gestures.push(
                        TableGesture::new("setColumnCalculation")
                            .column(&column.id)
                            .setting("calculate", ""),
                    );
                    ui.close();
                }
                ui.separator();
                for calculation in &offered {
                    if ui
                        .selectable_label(*calculation == calculate, calculation_label(calculation))
                        .clicked()
                    {
                        asked.gestures.push(
                            TableGesture::new("setColumnCalculation")
                                .column(&column.id)
                                .setting("calculate", calculation.clone()),
                        );
                        ui.close();
                    }
                }
            });
        }
    }
}

/// The table's own line at its foot: why a gesture was refused, or the question it was answered with and its two
/// answers.
fn foot_line(ui: &mut egui::Ui, tab: &TableTab, colours: &Colours, rect: Rect, foot: Foot, asked: &mut Asked) {
    ui.painter().rect_filled(rect, 0.0, colours.danger_bg);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(8.0, 4.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
            match (&tab.asking, foot) {
                (Some((_, question)), Foot::Asking) => {
                    ui.label(RichText::new(question).color(colours.text));
                    if ui.button("Delete").clicked() {
                        asked.answer = Some(true);
                    }
                    if ui.button("Keep").clicked() {
                        asked.answer = Some(false);
                    }
                }
                _ => {
                    ui.label(RichText::new(&tab.refusal).color(colours.danger));
                }
            }
        },
    );
}

/// What a press on the grid does, by what lies where it was pressed: a cell becomes the active one and its row the
/// selection, a double press opens its editor, a right press its row's menu; a row's checkbox ticks it, a group's
/// toggle folds it, and the line a row is added at adds one.
fn pointer(tab: &mut TableTab, response: egui::Response, asked: &mut Asked) {
    let at = |hits: &[(Rect, Hit)], place: Option<Pos2>| {
        place.and_then(|p| hits.iter().rev().find(|(r, _)| r.contains(p)).map(|(_, h)| h.clone()))
    };
    let response = match at(&asked.hits, response.hover_pos()) {
        Some(Hit::Cell { told, .. }) if !told.is_empty() => response.on_hover_text(told),
        Some(Hit::NewRow { .. }) => response.on_hover_text("Add a row"),
        Some(Hit::RowCheck { .. }) => response.on_hover_text("Select this row"),
        _ => response,
    };
    let pressed = at(&asked.hits, response.interact_pointer_pos());
    let editable = tab.editable();
    let flip = |collapsed: bool| if collapsed { "false" } else { "true" };
    if response.double_clicked() {
        if let Some(Hit::Cell { line, column, .. }) = &pressed {
            asked.edit = Some((*line, *column));
        }
    } else if response.clicked() {
        match &pressed {
            Some(Hit::Cell {
                line, column, row_id, ..
            }) => asked.click = Some((*line, *column, row_id.clone())),
            Some(Hit::RowCheck { row_id }) if editable => tab.toggle_checked(row_id),
            Some(Hit::Group { key, collapsed }) if editable => asked.gestures.push(
                TableGesture::new("toggleGroup")
                    .target(key)
                    .setting("collapsed", flip(*collapsed)),
            ),
            Some(Hit::RowToggle { row_id, collapsed }) if editable => asked.gestures.push(
                TableGesture::new("toggleRow")
                    .row(row_id)
                    .setting("collapsed", flip(*collapsed)),
            ),
            Some(Hit::NewRow { group }) if editable => asked.add_row = Some(group.clone()),
            _ => {}
        }
    }
    if response.secondary_clicked() {
        tab.menu_row = match &pressed {
            Some(Hit::Cell { row_id, .. } | Hit::RowCheck { row_id }) => Some(row_id.clone()),
            _ => None,
        };
    }
    if editable && let Some(row) = tab.menu_row.clone() {
        response.context_menu(|ui| {
            // Several ticked rows go together when the menu is one of theirs.
            let rows = tab.rows_of(&row);
            let label = if rows.len() > 1 {
                format!("Delete {} rows", rows.len())
            } else {
                "Delete row".to_owned()
            };
            if ui.button(label).clicked() {
                tab.delete_rows(&rows);
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
        .fade_in(false)
        .fixed_pos(cell.left_top())
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).fill(colours.surface).show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
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
                        // What the cell holds is the selected entry: a list of several shows each as selected.
                        let mut text = RichText::new(name).color(colours.text);
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
        .fade_in(false)
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

/// A chevron, pointing down when what it folds is open and right when it is folded, drawn as the explorer draws
/// its own rather than taken from a font.
fn chevron(painter: &egui::Painter, centre: Pos2, open: bool, colour: Color32) {
    let points = if open {
        vec![
            centre + Vec2::new(-4.0, -2.0),
            centre + Vec2::new(4.0, -2.0),
            centre + Vec2::new(0.0, 3.0),
        ]
    } else {
        vec![
            centre + Vec2::new(-2.0, -4.0),
            centre + Vec2::new(3.0, 0.0),
            centre + Vec2::new(-2.0, 4.0),
        ]
    };
    painter.add(egui::Shape::convex_polygon(points, colour, Stroke::NONE));
}

/// A sort's direction beside its column's name: an arrow down for descending, up for ascending.
pub(super) fn arrow(painter: &egui::Painter, at: Pos2, descending: bool, colour: Color32) {
    let (tip, tail) = if descending { (5.0, -5.0) } else { (-5.0, 5.0) };
    let stroke = Stroke::new(1.5, colour);
    painter.line_segment([at + Vec2::new(0.0, tail), at + Vec2::new(0.0, tip)], stroke);
    let head = at + Vec2::new(0.0, tip);
    let back = if descending { -3.5 } else { 3.5 };
    painter.line_segment([head, head + Vec2::new(-3.5, back)], stroke);
    painter.line_segment([head, head + Vec2::new(3.5, back)], stroke);
}

/// A kind's mark in its column's header, in `rect`: the standalone host's Material Design icons
/// (`knowledgeDefinition.ts`) drawn in lines - a text's letters, a number's sign, a tick, a calendar, a clock, a
/// selection's circled chevron, a list, and a relation's arrow out.
pub(super) fn kind_mark(painter: &egui::Painter, rect: Rect, kind: &str, colour: Color32, font: &FontId) {
    let stroke = Stroke::new(1.2, colour);
    let small = FontId::proportional(font.size - 2.0);
    let c = rect.center();
    match kind {
        "text" => {
            painter.text(c, Align2::CENTER_CENTER, "Aa", small, colour);
        }
        "number" => {
            painter.text(c, Align2::CENTER_CENTER, "#", font.clone(), colour);
        }
        "checkbox" => {
            let r = rect.shrink(2.0);
            painter.rect_stroke(r, 2.0, stroke, StrokeKind::Inside);
            painter.line(
                vec![
                    r.left_center() + Vec2::new(2.5, 0.0),
                    r.center_bottom() + Vec2::new(-1.0, -3.0),
                    r.right_top() + Vec2::new(-2.5, 3.0),
                ],
                stroke,
            );
        }
        "date" | "dateTime" => {
            let r = rect.shrink(2.0);
            painter.rect_stroke(r, 1.5, stroke, StrokeKind::Inside);
            painter.hline(r.x_range(), r.top() + 3.5, stroke);
            if kind == "dateTime" {
                painter.circle_filled(r.center() + Vec2::new(0.0, 2.0), 1.5, colour);
            }
        }
        "time" => {
            painter.circle_stroke(c, 5.5, stroke);
            painter.line(vec![c + Vec2::new(0.0, -3.5), c, c + Vec2::new(2.5, 1.5)], stroke);
        }
        "selection" => {
            painter.circle_stroke(c, 5.5, stroke);
            painter.line(
                vec![
                    c + Vec2::new(-2.5, -1.0),
                    c + Vec2::new(0.0, 1.5),
                    c + Vec2::new(2.5, -1.0),
                ],
                stroke,
            );
        }
        "multipleSelection" => {
            for dy in [-3.5, 0.0, 3.5] {
                painter.circle_filled(Pos2::new(rect.left() + 3.0, c.y + dy), 1.0, colour);
                painter.hline(rect.left() + 5.5..=rect.right() - 1.5, c.y + dy, stroke);
            }
        }
        "relation" => {
            let (from, to) = (c + Vec2::new(-4.0, 4.0), c + Vec2::new(4.0, -4.0));
            painter.line_segment([from, to], stroke);
            painter.line(vec![to + Vec2::new(-5.0, 0.0), to, to + Vec2::new(0.0, 5.0)], stroke);
        }
        _ => {
            painter.text(c, Align2::CENTER_CENTER, "?", small, colour);
        }
    }
}

//! Drawing the hierarchy tab with egui (etalii.adp spec 036, T028, research R8): the trail, the read-only reason, the
//! findings and the page, ported from the standalone host's `HierarchySurfaceTrail.tsx`, `FindingList.tsx`,
//! `HierarchySurface.tsx` (`NodeView`), `HierarchySurfaceBox.tsx` and `CollapseToggle.tsx` at a7fdf03, in the sizes of
//! `hierarchySurface.css` and VS Code's colours.
//!
//! - A level's own box is as high as what stands to its right: its entries, then its sub-levels, each a box sharing its
//!   borders with its neighbours. As many levels as fit the page's width are shown; a level whose deeper levels do not
//!   fit carries a mark that shows them.
//! - A box's first line is the collapse control, the number and the title. The title wraps under them, its next lines
//!   as far from the box's left as the labels and the description (the hanging indent of research R8): the first line
//!   is laid out at the box's 6 point edge after the control and the number, the rest again at the 12 point edge.
//! - The box's controls are one list: in a row at the end of the first line when the whole line fits beside them, in a
//!   column at the box's right otherwise, from the top down.
//! - Everything a frame is asked is collected while the page is drawn and carried out after it.

use adp_desl::hierarchy::definition::{FOCUS_ICON, HIDDEN_LEVELS_ICON, LabelLook};
use adp_desl::hierarchy::model::{SurfaceNode, roles};
use adp_theme::Theme;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2,
    accesskit::Role, text::LayoutJob,
};

use super::HierarchyTab;
use super::layout::{
    BOX_SPACING, ENTRY_MIN_WIDTH, HEADER_GAP, HierarchyIndex, ICON_SIZE, IconsLayout, LINE_HEIGHT, PAGE_MARGIN,
    header_width, icons_layout,
};
use crate::a11y;
use crate::icons;
use crate::table::view::Colours;
use crate::theme::Palette;

/// The space between the labels' row and the title, and between the tags in it.
const TAG_GAP: f32 = 4.0;
/// A tag's height, its side padding and its font size (`.hs-look .table-option-tag`).
const TAG_HEIGHT: f32 = 18.0;
const TAG_PADDING: f32 = 7.0;
const TAG_FONT: f32 = 11.0;
/// The space above a description and below it.
const DESCRIPTION_TOP: f32 = 4.0;
const DESCRIPTION_BOTTOM: f32 = 6.0;

/// What a frame of the page asked for, carried out once the page is drawn.
enum Act {
    Toggle(String),
    Select(String),
    Focus(String),
    ShowHidden(String),
    Reveal(String),
}

/// Draws the tab: the trail, the read-only reason, a refusal, the findings and the page.
pub fn show(ui: &mut egui::Ui, tab: &mut HierarchyTab, theme: &Theme, palette: &Palette) {
    tab.drain();
    let colours = Colours::of(theme, palette);
    let font = FontId::proportional(theme.sizes.workbench_font);
    let mut acts = Vec::new();
    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

    trail(ui, tab, &colours, &font, &mut acts);
    if tab.read_only() {
        bar(ui, &colours, |ui| {
            ui.label(
                RichText::new(icons::text("mdi-lock-outline"))
                    .font(icons::font(font.size))
                    .color(colours.muted),
            );
            ui.label(RichText::new(&tab.model.read_only_reason).color(colours.muted));
        });
    }
    if !tab.refusal.is_empty() {
        bar(ui, &colours, |ui| {
            ui.label(RichText::new(&tab.refusal).color(colours.danger));
        });
    }
    findings(ui, tab, &colours, &font, &mut acts);
    page(ui, tab, &colours, &font, &mut acts);

    for act in acts {
        match act {
            Act::Toggle(id) => tab.toggle(&id),
            Act::Select(id) => tab.select(&id),
            Act::Focus(id) => tab.set_focus(&id),
            Act::ShowHidden(id) => tab.show_hidden(&id),
            Act::Reveal(id) => tab.reveal(&id),
        }
    }
}

/// A line above the page, padded as the trail is and with a border under it.
fn bar(ui: &mut egui::Ui, colours: &Colours, content: impl FnOnce(&mut egui::Ui)) {
    let response = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(PAGE_MARGIN as i8, BOX_SPACING as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(content);
        });
    let rect = response.response.rect;
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, colours.border));
}

/// The trail above the page, always shown: the document's title, then the levels down to the selected node, or to the
/// focused level while nothing is selected. Each is a link that makes it the page's left edge, the title the whole
/// page, except the one that already is.
fn trail(ui: &mut egui::Ui, tab: &HierarchyTab, colours: &Colours, font: &FontId, acts: &mut Vec<Act>) {
    let current = tab.trail_current();
    let title = if tab.model.title.is_empty() {
        "Document".to_owned()
    } else {
        tab.model.title.clone()
    };
    let path: Vec<(String, String)> = tab
        .trail()
        .iter()
        .map(|n| (n.id.clone(), format!("{} {}", n.number, n.title)))
        .collect();
    bar(ui, colours, |ui| {
        trail_step(ui, &title, current == Some(""), "", colours, acts);
        for (id, text) in &path {
            ui.label(
                RichText::new(icons::text("mdi-chevron-right"))
                    .font(icons::font(font.size))
                    .color(colours.muted),
            );
            trail_step(ui, text, current == Some(id.as_str()), id, colours, acts);
        }
    });
}

fn trail_step(ui: &mut egui::Ui, text: &str, current: bool, id: &str, colours: &Colours, acts: &mut Vec<Act>) {
    if current {
        ui.label(RichText::new(text).strong().color(colours.text));
    } else if ui.link(RichText::new(text).color(colours.text)).clicked() {
        acts.push(Act::Focus(id.to_owned()));
    }
}

/// What reading the document reported, above the page: each finding at its line, and a link to the node it stands at
/// when it names one.
fn findings(ui: &mut egui::Ui, tab: &HierarchyTab, colours: &Colours, font: &FontId, acts: &mut Vec<Act>) {
    if tab.model.findings.is_empty() {
        return;
    }
    let response = egui::Frame::NONE
        .inner_margin(egui::Margin::symmetric(PAGE_MARGIN as i8, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            for finding in &tab.model.findings {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = BOX_SPACING;
                    let (icon, colour) = match finding.severity.as_str() {
                        "error" => ("mdi-alert-circle-outline", colours.danger),
                        "warning" => ("mdi-alert-outline", colours.warning),
                        _ => ("mdi-information-outline", colours.muted),
                    };
                    let mark = ui.label(
                        RichText::new(icons::text(icon))
                            .font(icons::font(font.size))
                            .color(colour),
                    );
                    a11y::describe(&mark, Role::Image, &finding.severity);
                    if finding.node_id.is_empty() {
                        ui.label(RichText::new(&finding.message).color(colours.text));
                    } else if ui.link(RichText::new(&finding.message).color(colours.text)).clicked() {
                        acts.push(Act::Reveal(finding.node_id.clone()));
                    }
                    if finding.line > 0 {
                        ui.label(RichText::new(format!("line {}", finding.line)).color(colours.muted));
                    }
                });
                ui.add_space(2.0);
            }
        });
    let rect = response.response.rect;
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, colours.border));
}

/// The page: one area that scrolls, the boxes laid out from its top left.
fn page(ui: &mut egui::Ui, tab: &mut HierarchyTab, colours: &Colours, font: &FontId, acts: &mut Vec<Act>) {
    let mut measured = tab.width;
    egui::ScrollArea::both()
        .id_salt(tab.id().with("page"))
        .auto_shrink(false)
        .show(ui, |ui| {
            measured = (ui.available_width() - 2.0 * PAGE_MARGIN).max(0.0);
            let index = HierarchyIndex::of(&tab.model.nodes);
            let mut page = Page {
                tab,
                index: &index,
                colours,
                font,
                fit: tab.fit(),
                root_level: tab.root_level(),
                acts,
                right: 0.0,
            };
            let origin = ui.max_rect().min + Vec2::splat(PAGE_MARGIN);
            let width = measured.max(ENTRY_MIN_WIDTH);
            let mut y = origin.y;
            for root in tab.roots() {
                let height = page.node(ui, root, 0, Pos2::new(origin.x, y), width);
                y += height - 1.0;
            }
            let size = Vec2::new(
                page.right.max(origin.x + width) - ui.max_rect().min.x,
                y - ui.max_rect().min.y,
            ) + Vec2::splat(PAGE_MARGIN);
            ui.allocate_rect(Rect::from_min_size(ui.max_rect().min, size), Sense::hover());
        });
    tab.width = measured;
}

/// One frame's drawing of the page.
struct Page<'a, 'm> {
    tab: &'a HierarchyTab,
    index: &'a HierarchyIndex<'m>,
    colours: &'a Colours,
    font: &'a FontId,
    fit: usize,
    root_level: u32,
    acts: &'a mut Vec<Act>,
    /// The rightmost edge drawn, for the page's width.
    right: f32,
}

impl Page<'_, '_> {
    /// Draws `node` at `at`, `width` wide, with what stands to its right; its height.
    fn node(&mut self, ui: &mut egui::Ui, node: &SurfaceNode, depth: usize, at: Pos2, width: f32) -> f32 {
        if node.role != roles::LEVEL {
            return self.boxed(ui, node, Rect::from_min_size(at, Vec2::new(width, 0.0)), 0);
        }
        let collapsed = self.tab.is_collapsed(node);
        let level_width = self.tab.column_width(self.root_level + depth as u32);
        let shows_sub_levels = depth + 1 < self.fit;
        let hidden = if shows_sub_levels {
            0
        } else {
            self.index.levels_below(&node.id)
        };
        let children = self.index.children_of(&node.id);
        let mut content: Vec<&SurfaceNode> = Vec::new();
        if !collapsed {
            content.extend(children.iter().filter(|c| c.role != roles::LEVEL));
            if shows_sub_levels {
                content.extend(children.iter().filter(|c| c.role == roles::LEVEL));
            }
        }
        // The content keeps the entries' least width, and the page scrolls sideways when the window is narrower.
        let content_width = (width - level_width + 1.0).max(ENTRY_MIN_WIDTH);
        let content_x = at.x + level_width - 1.0;
        let mut y = at.y;
        for child in &content {
            let child_depth = if child.role == roles::LEVEL { depth + 1 } else { depth };
            let height = self.node(ui, child, child_depth, Pos2::new(content_x, y), content_width);
            y += height - 1.0;
        }
        let content_height = if content.is_empty() { 0.0 } else { y - at.y + 1.0 };
        let own = Rect::from_min_size(at, Vec2::new(level_width, content_height));
        self.boxed(ui, node, own, hidden).max(content_height)
    }

    /// Draws one node's own box in `rect`'s width, at least as high as `rect`; its height.
    fn boxed(&mut self, ui: &mut egui::Ui, node: &SurfaceNode, rect: Rect, hidden: usize) -> f32 {
        let tab = self.tab;
        let colours = self.colours;
        let level = node.role == roles::LEVEL;
        let collapsed = tab.is_collapsed(node);
        let selected = tab.selected == node.id;
        let painter = ui.painter().clone();
        let background = painter.add(egui::Shape::Noop);
        let id = tab.id().with(("box", &node.id));
        // The box takes the clicks its controls do not.
        // Each control's icon, its name, its tooltip and what it asks for.
        let mut controls: Vec<(&'static str, String, String, Act)> = Vec::new();
        if hidden > 0 {
            let levels = if hidden == 1 { "level" } else { "levels" };
            controls.push((
                HIDDEN_LEVELS_ICON,
                format!(
                    "{hidden} more {levels}: show the levels of {} {}",
                    node.number, node.title
                ),
                format!("{hidden} more {levels}"),
                Act::ShowHidden(node.id.clone()),
            ));
        }
        if level {
            let name = format!("Focus on {} {}", node.number, node.title);
            controls.push((FOCUS_ICON, name.clone(), name, Act::Focus(node.id.clone())));
        }

        let number = painter.layout_no_wrap(node.number.clone(), self.font.clone(), colours.muted);
        let title_width = painter
            .layout_no_wrap(node.title.clone(), self.font.clone(), colours.text)
            .size()
            .x;
        let header = header_width(false, number.size().x, title_width);
        let layout = icons_layout(Some(rect.width()), Some(header), controls.len());
        let side = layout == IconsLayout::Column && !controls.is_empty();
        let left = rect.left() + BOX_SPACING;
        let text_right = rect.right() - BOX_SPACING - if side { ICON_SIZE + BOX_SPACING } else { 0.0 };
        let text_width = (text_right - left).max(ICON_SIZE);
        let top = rect.top() + BOX_SPACING;

        // The first line: the collapse control, the number and as much of the title as fits after them.
        let toggle = Rect::from_min_size(Pos2::new(left, top), Vec2::splat(ICON_SIZE));
        let number_at = Pos2::new(toggle.right() + HEADER_GAP, top + (LINE_HEIGHT - number.size().y) / 2.0);
        let lead = ICON_SIZE + HEADER_GAP + number.size().x + HEADER_GAP;
        let first = painter.layout_job(self.title_job(&node.title, lead, text_width, Some(1)));
        let shown: usize = first.rows.first().map_or(0, |r| r.char_count_excluding_newline().0);
        let rest: String = node
            .title
            .chars()
            .skip(shown)
            .collect::<String>()
            .trim_start()
            .to_owned();
        painter.galley(Pos2::new(left, top), first, colours.text);
        painter.galley(number_at, number, colours.muted);
        let mut y = top + LINE_HEIGHT;
        if !rest.is_empty() {
            let more = painter.layout_job(self.title_job(&rest, 0.0, text_width - BOX_SPACING, None));
            let height = more.size().y;
            painter.galley(Pos2::new(left + BOX_SPACING, y), more, colours.text);
            y += height;
        }

        y = self.labels(&painter, node, Pos2::new(left + BOX_SPACING, y), text_right);
        if !collapsed && !node.description.is_empty() {
            let mut job = LayoutJob::simple(
                node.description.clone(),
                self.font.clone(),
                colours.text,
                text_width - BOX_SPACING,
            );
            job.sections[0].format.line_height = Some(LINE_HEIGHT);
            let galley = painter.layout_job(job);
            let height = galley.size().y;
            painter.galley(Pos2::new(left + BOX_SPACING, y + DESCRIPTION_TOP), galley, colours.text);
            y += DESCRIPTION_TOP + height + DESCRIPTION_BOTTOM - BOX_SPACING;
        }
        let controls_height = if side { controls.len() as f32 * ICON_SIZE } else { 0.0 };
        let height = (y + BOX_SPACING - rect.top())
            .max(controls_height + 2.0 * BOX_SPACING)
            .max(rect.height());
        let outline = Rect::from_min_size(rect.min, Vec2::new(rect.width(), height));
        self.right = self.right.max(outline.right());

        let fill = if level { colours.raised } else { colours.surface };
        painter.set(background, egui::Shape::rect_filled(outline, CornerRadius::ZERO, fill));
        painter.rect_stroke(outline, 0.0, Stroke::new(1.0, colours.border), StrokeKind::Inside);
        if selected {
            painter.rect_stroke(outline, 0.0, Stroke::new(2.0, colours.focus), StrokeKind::Inside);
        }
        let response = ui.interact(outline, id, Sense::click());
        a11y::tree_item(
            &response,
            &format!("{} {}", node.number, node.title),
            self.index.ancestors_of(&node.id).len() + 1,
            selected,
            Some(!collapsed),
        );
        if response.clicked() {
            self.acts.push(Act::Select(node.id.clone()));
        }

        // The collapse control, and the box's controls: shown under the pointer and on the selected box.
        // The pointer over the box, its controls included: over a control the box itself is not hovered.
        let hovered = ui.rect_contains_pointer(outline) || selected;
        let chevron = if collapsed {
            "mdi-chevron-right"
        } else {
            "mdi-chevron-down"
        };
        let verb = if collapsed { "Expand" } else { "Collapse" };
        let name = format!("{verb} {} {}", node.number, node.title);
        if self.icon_button(ui, toggle, chevron, &name, &name, id.with("toggle"), true) {
            self.acts.push(Act::Toggle(node.id.clone()));
        }
        for (i, (icon, label, tooltip, act)) in controls.into_iter().enumerate() {
            let at = match layout {
                IconsLayout::Column => Pos2::new(rect.right() - BOX_SPACING - ICON_SIZE, top + i as f32 * ICON_SIZE),
                IconsLayout::Row => Pos2::new(rect.right() - BOX_SPACING - (i + 1) as f32 * ICON_SIZE, top),
            };
            let place = Rect::from_min_size(at, Vec2::splat(ICON_SIZE));
            // The hidden levels' mark is always shown; focus only where a person is looking, kept in its place.
            let visible = icon == HIDDEN_LEVELS_ICON || hovered;
            if self.icon_button(ui, place, icon, &label, &tooltip, id.with(icon), visible) {
                self.acts.push(act);
            }
        }
        height
    }

    /// A title's layout: `lead` points of the first row left for what stands before it, `rows` at most.
    fn title_job(&self, text: &str, lead: f32, width: f32, rows: Option<usize>) -> LayoutJob {
        let mut job = LayoutJob::default();
        let mut format = egui::TextFormat::simple(self.font.clone(), self.colours.text);
        format.line_height = Some(LINE_HEIGHT);
        job.append(text, lead, format);
        job.wrap.max_width = width.max(1.0);
        if let Some(rows) = rows {
            job.wrap.max_rows = rows;
            job.wrap.overflow_character = None;
        }
        job
    }

    /// The labels' values as tags below the title, in the definition's order and each in its look; the bottom of the
    /// row, or `at`'s top when the node carries none.
    fn labels(&self, painter: &egui::Painter, node: &SurfaceNode, at: Pos2, right: f32) -> f32 {
        let colours = self.colours;
        let font = FontId::proportional(TAG_FONT);
        let mut tags: Vec<(LabelLook, String, Color32)> = Vec::new();
        for label in self
            .tab
            .definition
            .labels
            .iter()
            .filter(|l| l.on.contains(&node.type_name))
        {
            let Some(values) = node.labels.iter().find(|v| v.label == label.label) else {
                continue;
            };
            for value in &values.values {
                tags.push((
                    label.look(),
                    value.clone(),
                    colours.option(&self.colour_of(&label.label, value)),
                ));
            }
        }
        if tags.is_empty() {
            return at.y;
        }
        let mut x = at.x;
        let mut y = at.y + TAG_GAP;
        for (look, value, fill) in tags {
            let text = if look == LabelLook::Tag {
                format!("#{value}")
            } else {
                value
            };
            let galley = painter.layout_no_wrap(text, font.clone(), colours.text);
            let dot = if look == LabelLook::Status { 6.0 + 5.0 } else { 0.0 };
            let width = galley.size().x + 2.0 * TAG_PADDING + dot;
            if x > at.x && x + width > right {
                x = at.x;
                y += TAG_HEIGHT + TAG_GAP;
            }
            let tag = Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, TAG_HEIGHT));
            let radius = if look == LabelLook::Badge {
                3.0
            } else {
                TAG_HEIGHT / 2.0
            };
            painter.rect_filled(tag, radius, fill);
            if dot > 0.0 {
                painter.circle_filled(
                    Pos2::new(tag.left() + TAG_PADDING + 3.0, tag.center().y),
                    3.0,
                    colours.text.gamma_multiply(0.6),
                );
            }
            painter.galley(
                Pos2::new(tag.left() + TAG_PADDING + dot, tag.center().y - galley.size().y / 2.0),
                galley,
                colours.text,
            );
            x = tag.right() + TAG_GAP;
        }
        y + TAG_HEIGHT
    }

    /// The colour the option list of `label` gives `value`, matched by name or id as a person types it.
    fn colour_of(&self, label: &str, value: &str) -> String {
        let wanted = value.to_lowercase();
        self.tab
            .model
            .option_lists
            .iter()
            .find(|l| l.label == label)
            .and_then(|l| {
                l.options
                    .iter()
                    .find(|o| o.name.to_lowercase() == wanted || o.id.to_lowercase() == wanted)
            })
            .map(|o| o.colour.clone())
            .unwrap_or_else(|| "default".to_owned())
    }

    /// One of a box's controls: its icon in `place`, named `label` for a screen reader and showing `tooltip` under the
    /// pointer; whether it was pressed. A control not `visible` keeps its place and takes no press.
    #[allow(clippy::too_many_arguments)]
    fn icon_button(
        &self,
        ui: &mut egui::Ui,
        place: Rect,
        icon: &str,
        label: &str,
        tooltip: &str,
        id: egui::Id,
        visible: bool,
    ) -> bool {
        if !visible {
            return false;
        }
        let response = ui.interact(place, id, Sense::click());
        a11y::describe(&response, Role::Button, label);
        let colour = if response.hovered() {
            self.colours.text
        } else {
            self.colours.muted
        };
        ui.painter().text(
            place.center(),
            Align2::CENTER_CENTER,
            icons::text(icon),
            icons::font(self.font.size),
            colour,
        );
        response.on_hover_text(tooltip).clicked()
    }
}

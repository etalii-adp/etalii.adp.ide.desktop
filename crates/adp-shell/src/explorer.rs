//! The explorer (etalii.adp spec 030, Story 1, FR-013, task T034): the workspace tree in VS Code's look, folders
//! first, each registration under the file it registers, kept in step with the disk. A click selects, a double
//! click or Enter opens, the arrows move, F2 renames and Delete deletes; the context menu offers the standalone
//! host's actions. The explorer only says what was asked; the shell does it.

use std::collections::HashSet;

use adp_add::{AddCatalogue, AddTarget, Offer};
use adp_hierarchy::{Hierarchy, Kind, Node};
use adp_theme::Theme;
use eframe::egui::{self, Color32, Key, RichText, Sense};

use crate::theme::color;

/// How far right of a row's indent its chevron reaches: a press left of that is a press on the chevron.
const CHEVRON_WIDTH: f32 = 12.0;

/// What the person asked of the explorer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Open(String),
    OpenAsText(String),
    NewFolder(String),
    Rename(String),
    Delete(String),
    Validate(String),
    ValidateAll,
    /// The knowledge file's entry that adds a relation to another table (T064).
    Relate(String),
    /// Add… on a folder, or Add as diagram… on a file a tool type reads (T068).
    Add(String),
}

#[derive(Default)]
pub struct Explorer {
    pub expanded: HashSet<String>,
    pub selected: Option<String>,
    /// The project's folder, which the entries' paths are relative to: what a context menu asks a module about.
    pub root: std::path::PathBuf,
    /// What the add dialog offers, loaded when a context menu first asks.
    adds: Option<std::rc::Rc<AddCatalogue>>,
    id: Option<egui::Id>,
}

impl Explorer {
    /// What the add dialog offers.
    pub fn adds(&mut self) -> std::rc::Rc<AddCatalogue> {
        self.adds
            .get_or_insert_with(|| std::rc::Rc::new(AddCatalogue::standalone()))
            .clone()
    }

    /// The add on the entry at `path`, a folder or a file, when it has one.
    fn add_offer(&mut self, path: &str, kind: Kind) -> Option<Offer> {
        let full = self.root.join(path);
        let target = match kind {
            Kind::Folder => AddTarget::Folder(full),
            Kind::File => AddTarget::File(full),
            Kind::Registration => return None,
        };
        self.adds().offer(&target)
    }

    pub fn show(&mut self, ui: &mut egui::Ui, tree: &Hierarchy, theme: &Theme) -> Vec<Request> {
        let id = *self.id.get_or_insert_with(|| egui::Id::unique("explorer"));
        let mut requests = Vec::new();
        let mut rows = Vec::new();
        flatten(&tree.children, 0, &self.expanded, &mut rows);
        let focused = ui.memory(|m| m.has_focus(id));
        if focused {
            self.keyboard(ui, &rows, &mut requests);
        }
        let row_height = theme.sizes.row_height;
        let selection = color(theme, &["list.activeSelectionBackground"], Color32::DARK_BLUE);
        let inactive_selection = color(theme, &["list.inactiveSelectionBackground"], selection);
        let hover = color(theme, &["list.hoverBackground"], Color32::from_gray(40));
        let foreground = color(theme, &["sideBar.foreground", "foreground"], Color32::GRAY);
        let muted = color(theme, &["descriptionForeground"], foreground);
        let focus = color(theme, &["list.focusOutline", "focusBorder"], Color32::LIGHT_BLUE);
        let error = color(theme, &["errorForeground"], Color32::RED);

        egui::ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .show_rows(ui, row_height, rows.len(), |ui, range| {
                for row in &rows[range] {
                    let width = ui.available_width();
                    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, row_height), Sense::click());
                    let response = response.on_hover_text(&row.node.path);
                    let selected = self.selected.as_deref() == Some(row.node.path.as_str());
                    if selected {
                        ui.painter()
                            .rect_filled(rect, 0.0, if focused { selection } else { inactive_selection });
                        if focused {
                            ui.painter().rect_stroke(
                                rect.shrink(0.5),
                                0.0,
                                egui::Stroke::new(theme.sizes.focus_outline_width, focus),
                                egui::StrokeKind::Inside,
                            );
                        }
                    } else if response.hovered() {
                        ui.painter().rect_filled(rect, 0.0, hover);
                    }
                    let indent = 8.0 + row.depth as f32 * 8.0;
                    let has_children = !row.node.children.is_empty();
                    let open = self.expanded.contains(&row.node.path);
                    let text_colour = match (row.node.kind, row.node.orphan) {
                        (_, true) => error,
                        (Kind::Registration, false) => muted,
                        _ => foreground,
                    };
                    // VS Code's chevron: pointing right when closed, down when open, drawn rather than taken from a font.
                    if row.node.kind == Kind::Folder || has_children {
                        let centre = egui::pos2(rect.left() + indent + 4.0, rect.center().y);
                        let points = if open {
                            vec![
                                centre + egui::vec2(-4.0, -2.0),
                                centre + egui::vec2(4.0, -2.0),
                                centre + egui::vec2(0.0, 3.0),
                            ]
                        } else {
                            vec![
                                centre + egui::vec2(-2.0, -4.0),
                                centre + egui::vec2(3.0, 0.0),
                                centre + egui::vec2(-2.0, 4.0),
                            ]
                        };
                        ui.painter()
                            .add(egui::Shape::convex_polygon(points, foreground, egui::Stroke::NONE));
                    }
                    ui.painter().text(
                        egui::pos2(rect.left() + indent + 14.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &row.node.name,
                        egui::FontId::proportional(theme.sizes.workbench_font),
                        text_colour,
                    );
                    if response.clicked() {
                        ui.memory_mut(|m| m.request_focus(id));
                        self.selected = Some(row.node.path.clone());
                        // A press on the chevron opens or closes the row and does nothing else, as VS Code's twistie;
                        // a press on the name of a file opens the file.
                        let on_chevron = response
                            .interact_pointer_pos()
                            .is_some_and(|p| p.x < rect.left() + indent + CHEVRON_WIDTH);
                        if row.node.kind == Kind::Folder || has_children {
                            self.toggle(&row.node.path);
                        }
                        if row.node.kind != Kind::Folder && !(has_children && on_chevron) {
                            requests.push(Request::Open(row.node.path.clone()));
                        }
                    }
                    response.context_menu(|ui| {
                        self.selected = Some(row.node.path.clone());
                        let path = row.node.path.clone();
                        let folder = if row.node.kind == Kind::Folder {
                            path.clone()
                        } else {
                            path.rsplit_once('/').map(|(p, _)| p.to_owned()).unwrap_or_default()
                        };
                        // The standalone host's Add… (`AddDiagramContextActionProvider`), offered first.
                        if let Some(offer) = self.add_offer(&path, row.node.kind) {
                            let button = egui::Button::new(RichText::new(offer.label)).shortcut_text("Insert");
                            let response = ui.add_enabled(offer.unavailable_reason.is_none(), button);
                            let response = match offer.unavailable_reason {
                                Some(reason) => response.on_disabled_hover_text(reason),
                                None => response,
                            };
                            if response.clicked() {
                                requests.push(Request::Add(path.clone()));
                                ui.close();
                            }
                        }
                        let mut item = |ui: &mut egui::Ui, label: &str, shortcut: &str, request: Request| {
                            let button = egui::Button::new(RichText::new(label)).shortcut_text(shortcut);
                            if ui.add(button).clicked() {
                                requests.push(request);
                                ui.close();
                            }
                        };
                        item(ui, "New Folder…", "", Request::NewFolder(folder));
                        if row.node.kind != Kind::Folder {
                            item(ui, "Open as Text", "", Request::OpenAsText(path.clone()));
                        }
                        // What a module offers on the entry: a knowledge file adds a relation.
                        for group in adp_designer_knowledge::context_actions::discover(
                            &self.root.join(&path),
                            row.node.kind == Kind::Folder,
                        ) {
                            for action in group.actions {
                                item(ui, &action.label, "", Request::Relate(path.clone()));
                            }
                        }
                        ui.separator();
                        item(ui, "Rename…", "F2", Request::Rename(path.clone()));
                        item(ui, "Delete", "Delete", Request::Delete(path.clone()));
                        ui.separator();
                        item(ui, "Validate", "", Request::Validate(path.clone()));
                        item(ui, "Validate All", "", Request::ValidateAll);
                    });
                }
            });
        requests
    }

    fn toggle(&mut self, path: &str) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_owned());
        }
    }

    fn keyboard(&mut self, ui: &egui::Ui, rows: &[Row], requests: &mut Vec<Request>) {
        let index = self
            .selected
            .as_ref()
            .and_then(|s| rows.iter().position(|r| &r.node.path == s));
        let pressed = |key: Key| ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key));
        if pressed(Key::ArrowDown) {
            let next = index.map_or(0, |i| (i + 1).min(rows.len().saturating_sub(1)));
            self.selected = rows.get(next).map(|r| r.node.path.clone());
        }
        if pressed(Key::ArrowUp) {
            let next = index.map_or(0, |i| i.saturating_sub(1));
            self.selected = rows.get(next).map(|r| r.node.path.clone());
        }
        let Some(row) = index.and_then(|i| rows.get(i)) else {
            return;
        };
        let path = row.node.path.clone();
        let expandable = row.node.kind == Kind::Folder || !row.node.children.is_empty();
        if pressed(Key::ArrowRight) && expandable {
            self.expanded.insert(path.clone());
        }
        if pressed(Key::ArrowLeft) && expandable {
            self.expanded.remove(&path);
        }
        if pressed(Key::Enter) {
            if row.node.kind == Kind::Folder {
                self.toggle(&path);
            } else {
                requests.push(Request::Open(path.clone()));
            }
        }
        if pressed(Key::F2) {
            requests.push(Request::Rename(path.clone()));
        }
        if pressed(Key::Insert)
            && self
                .add_offer(&path, row.node.kind)
                .is_some_and(|o| o.unavailable_reason.is_none())
        {
            requests.push(Request::Add(path.clone()));
        }
        if pressed(Key::Delete) {
            requests.push(Request::Delete(path));
        }
    }
}

struct Row<'n> {
    node: &'n Node,
    depth: usize,
}

fn flatten<'n>(nodes: &'n [Node], depth: usize, expanded: &HashSet<String>, rows: &mut Vec<Row<'n>>) {
    for node in nodes {
        rows.push(Row { node, depth });
        if expanded.contains(&node.path) {
            flatten(&node.children, depth + 1, expanded, rows);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, kind: Kind, children: Vec<Node>) -> Node {
        Node {
            name: name.to_owned(),
            path: name.to_owned(),
            kind,
            orphan: false,
            children,
        }
    }

    /// One frame of the explorer over `tree`, with `events`; what it was asked.
    fn frame(ctx: &egui::Context, explorer: &mut Explorer, tree: &Hierarchy, events: Vec<egui::Event>) -> Vec<Request> {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0))),
            events,
            ..Default::default()
        };
        let mut requests = Vec::new();
        let theme = Theme::dark();
        ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| requests = explorer.show(ui, tree, &theme));
        })
        .textures_delta
        .clear();
        requests
    }

    fn click(ctx: &egui::Context, explorer: &mut Explorer, tree: &Hierarchy, at: egui::Pos2) -> Vec<Request> {
        let press = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut requests = frame(ctx, explorer, tree, vec![egui::Event::PointerMoved(at), press(true)]);
        requests.extend(frame(ctx, explorer, tree, vec![press(false)]));
        requests
    }

    #[test]
    fn a_press_on_the_chevron_of_a_file_with_registrations_expands_it_without_opening_it() {
        let tree = Hierarchy {
            root: std::path::PathBuf::new(),
            children: vec![node(
                "courier.dsl",
                Kind::File,
                vec![node("courier.containers.adp", Kind::Registration, Vec::new())],
            )],
        };
        let ctx = egui::Context::default();
        let mut explorer = Explorer::default();
        frame(&ctx, &mut explorer, &tree, Vec::new());
        let row = theme_row_height() / 2.0;
        // The first row's chevron sits 8 in from the left, its name 22 in.
        let chevron = egui::pos2(8.0 + 12.0, 8.0 + row);
        assert!(
            click(&ctx, &mut explorer, &tree, chevron).is_empty(),
            "nothing is opened"
        );
        assert!(explorer.expanded.contains("courier.dsl"), "the row is expanded");

        let name = egui::pos2(8.0 + 60.0, 8.0 + row);
        assert_eq!(
            click(&ctx, &mut explorer, &tree, name),
            vec![Request::Open("courier.dsl".into())],
            "a press on the name opens the file"
        );
    }

    fn theme_row_height() -> f32 {
        Theme::dark().sizes.row_height
    }
}

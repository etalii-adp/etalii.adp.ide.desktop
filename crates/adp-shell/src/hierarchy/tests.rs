//! The hierarchy tab's tests: the layout rules (the standalone host's `iconsLayout.test.ts` and the fit test of
//! `ColumnEdge.test.tsx` at a7fdf03; etalii.adp spec 036, T023), and the page drawn headless and pressed through its
//! accessibility tree (`HierarchySurface.test.tsx`; T026 to T029), and changed there (`editing/AddControl.test.tsx`; T038).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use adp_desl::hierarchy::definition::{
    AddDefinition, Deletion, FOCUS_ICON, HIDDEN_LEVELS_ICON, LabelDefinition, MoveParams, NoteOperations, NoteParams,
    NoteWords, NotesDefinition, Operations, SurfaceDefinition,
};
use adp_desl::hierarchy::model::{
    EditOutcome, Gesture, LabelValues, Note, OptionList, Reference, SurfaceAction, SurfaceChange, SurfaceFinding,
    SurfaceModel, SurfaceNode, SurfaceOption, causes, roles,
};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use eframe::egui::{
    self,
    accesskit::{Node, NodeId, Role},
};

use super::HierarchyTab;
use super::editing::{self, Attribute, DropHint, DropKind, EntryDrop};
use super::layout::*;
use super::notes;
use crate::icons;
use crate::theme::{self, Palette};

#[test]
fn the_levels_that_fit_are_counted_by_their_widths_from_the_left_leaving_the_entries_their_least_width() {
    let widths = [400.0, 240.0, 240.0];
    let fit = |width: f32| levels_that_fit(width, |column| widths.get(column).copied().unwrap_or(LEVEL_WIDTH));
    // 280 for the entries, then 400, 240 and 240.
    assert_eq!(
        [fit(679.0), fit(680.0), fit(919.0), fit(920.0), fit(1160.0), fit(100.0)],
        [1, 1, 1, 2, 3, 1]
    );
    assert_eq!(levels_that_fit(1000.0, |_| LEVEL_WIDTH), 3);
    assert_eq!(
        levels_that_fit(0.0, |_| LEVEL_WIDTH),
        usize::MAX,
        "an unmeasured page shows them all"
    );
}

#[test]
fn the_controls_stand_in_a_column_while_the_box_or_its_first_line_is_not_measured() {
    assert_eq!(icons_layout(None, Some(100.0), 2), IconsLayout::Column);
    assert_eq!(icons_layout(Some(400.0), None, 2), IconsLayout::Column);
}

#[test]
fn the_controls_stand_in_a_row_when_the_first_line_and_every_control_fit_side_by_side_with_the_boxs_space_around_them()
{
    // A box 300 wide holds 288 inside its padding; three controls and the space before them take 66.
    let width = 300.0;
    let fits = width - 2.0 * BOX_SPACING - BOX_SPACING - 3.0 * ICON_SIZE;
    assert_eq!(icons_layout(Some(width), Some(fits), 3), IconsLayout::Row);
    assert_eq!(icons_layout(Some(width), Some(fits + 1.0), 3), IconsLayout::Column);
    // One control fewer leaves room for a longer first line.
    assert_eq!(icons_layout(Some(width), Some(fits + ICON_SIZE), 2), IconsLayout::Row);
}

#[test]
fn the_first_line_is_the_grip_the_control_the_number_and_the_title_with_their_gaps() {
    assert_eq!(
        header_width(false, 60.0, 100.0),
        ICON_SIZE + HEADER_GAP + 60.0 + HEADER_GAP + 100.0
    );
    assert_eq!(
        header_width(true, 60.0, 100.0),
        GRIP_WIDTH + HEADER_GAP + header_width(false, 60.0, 100.0)
    );
}

#[test]
fn the_spec_sizes_are_the_standalone_hosts() {
    // Story 7, scenarios 4 to 6: 6 points inside a box and 12 around the page.
    assert_eq!((BOX_SPACING, PAGE_MARGIN), (6.0, 12.0));
    assert_eq!(
        (LEVEL_WIDTH, ENTRY_MIN_WIDTH, ICON_SIZE, LINE_HEIGHT),
        (240.0, 280.0, 20.0, 20.0)
    );
}

// ---- The page (the standalone host's `HierarchySurface.test.tsx` at a7fdf03; T026 to T029) ----

/// A session that keeps one model and records the gestures it is sent, each written at once: the model a test queued
/// next, or the one it has, comes back, or the refusal a test set.
struct Recorder {
    model: SurfaceModel,
    gestures: Rc<RefCell<Vec<Gesture>>>,
    script: Rc<RefCell<Script>>,
    changes: Vec<SurfaceChange>,
}

/// What a test has the session answer.
#[derive(Default)]
struct Script {
    /// The models the next gestures bring, in order.
    next: Vec<SurfaceModel>,
    /// The sentence the next gestures are not written with.
    refusal: String,
    /// The model the watcher's next news of a change brings, as another program left the file.
    outside: Option<SurfaceModel>,
    /// The sentence the next gestures are refused with at once, before anything is written.
    at_once: String,
    /// Why the action inside a node cannot run, when it cannot.
    unavailable: String,
}

impl HierarchySurfaceSession for Recorder {
    fn model(&self) -> SurfaceModel {
        self.model.clone()
    }

    fn gesture(&mut self, gesture: Gesture) -> Result<(), String> {
        let mut script = self.script.borrow_mut();
        if !script.at_once.is_empty() {
            self.gestures.borrow_mut().push(gesture);
            return Err(script.at_once.clone());
        }
        if !script.next.is_empty() {
            self.model = script.next.remove(0);
        }
        self.changes.push(SurfaceChange::Model(Box::new(self.model.clone())));
        self.changes.push(SurfaceChange::Outcome(EditOutcome {
            edit_id: gesture.edit_id.clone(),
            written: script.refusal.is_empty(),
            error: script.refusal.clone(),
        }));
        self.gestures.borrow_mut().push(gesture);
        Ok(())
    }

    /// Every node offers Ctrl+Enter, below it, and Ctrl+Shift+Enter, inside it.
    fn actions(&self, _node_id: &str) -> Vec<SurfaceAction> {
        let action = |id: &str, shift: bool, operation: &str| SurfaceAction {
            id: id.into(),
            label: id.into(),
            key: "Enter".into(),
            shift,
            operation: operation.into(),
            ..SurfaceAction::default()
        };
        let mut inside = action("inside", true, "jot-inside");
        inside.unavailable = self.script.borrow().unavailable.clone();
        vec![action("below", false, "jot-below"), inside]
    }

    fn take_changes(&mut self) -> Vec<SurfaceChange> {
        std::mem::take(&mut self.changes)
    }

    fn undo(&mut self) -> Result<(), String> {
        Err("There is nothing to undo.".into())
    }

    fn redo(&mut self) -> Result<(), String> {
        Err("There is nothing to redo.".into())
    }

    fn file_changed(&mut self, _path: &Path) {
        if let Some(model) = self.script.borrow_mut().outside.take() {
            self.model = model;
            self.changes.push(SurfaceChange::Model(Box::new(self.model.clone())));
        }
    }
}

fn label(label: &str, prefix: &str, many: bool, on: &[&str]) -> LabelDefinition {
    LabelDefinition {
        label: label.into(),
        prefix: prefix.into(),
        many,
        look: None,
        on: on.iter().map(|t| (*t).to_owned()).collect(),
        edit: None,
        pick: None,
    }
}

fn add(operation: &str, label: &str, icon: &str) -> AddDefinition {
    AddDefinition {
        operation: operation.into(),
        label: label.into(),
        icon: icon.into(),
    }
}

/// The standalone test's definition: a surface of sections and points that names none of the designer's words.
fn definition() -> SurfaceDefinition {
    SurfaceDefinition {
        level_type: "Section".into(),
        entry_type: "Point".into(),
        labels: vec![
            label("marks", "#", true, &["Section", "Point"]),
            label("phase", "phase:", false, &["Point"]),
        ],
        note_icons: vec![
            ("Doubt".into(), "mdi-help-circle-outline".into()),
            ("Aside".into(), "mdi-comment-text-outline".into()),
        ],
        several_notes_icon: "mdi-comment-multiple-outline".into(),
        notes: NotesDefinition {
            operations: NoteOperations {
                add: "remark".into(),
                change: "reword".into(),
                resolve: "settle".into(),
                remove: "strike".into(),
            },
            params: NoteParams {
                kind: "sort".into(),
                text: "line".into(),
                detail: "more".into(),
            },
            words: NoteWords {
                list: "Notes".into(),
                add: "New note".into(),
                change: "Reword".into(),
                resolve: "Settle".into(),
                reopen: "Unsettle".into(),
                remove: "Strike".into(),
            },
        },
        operations: Operations {
            toggle_collapsed: "fold".into(),
            set_title: "retitle".into(),
            set_description: "redescribe".into(),
            add_top_level: add("open", "New section", "mdi-plus"),
            add_entry: add("jot", "New point", "mdi-note-plus-outline"),
            add_level: add("nest", "New section", "mdi-folder-plus-outline"),
            move_level: "shift".into(),
            move_entry: "slide".into(),
            delete: "drop".into(),
        },
        move_params: MoveParams {
            into: "section".into(),
            after: "behind".into(),
        },
        deletion: Deletion {
            threshold: 1,
            message: "Drop {number}? {count} things concern it.".into(),
        },
        filters: None,
        widths: None,
        shortcut_keys: vec!["Enter".into()],
    }
}

fn node(id: &str, role: &str, type_name: &str, parent: &str, title: &str) -> SurfaceNode {
    SurfaceNode {
        id: id.into(),
        number: format!("[{id}]"),
        role: role.into(),
        type_name: type_name.into(),
        parent_id: parent.into(),
        title: title.into(),
        ..SurfaceNode::default()
    }
}

fn level(id: &str, parent: &str, title: &str) -> SurfaceNode {
    node(id, roles::LEVEL, "Section", parent, title)
}

fn entry(id: &str, parent: &str, title: &str) -> SurfaceNode {
    node(id, roles::ENTRY, "Point", parent, title)
}

/// Three levels deep: 1 holds the point 2 and the section 3, which holds the point 4 and the section 5, which holds
/// the point 6. A second top section, 7, stands beside 1.
fn three_levels() -> SurfaceModel {
    let mut take = entry("2", "1", "Take an order");
    take.description = "Any open order can be taken.".into();
    take.labels = vec![
        LabelValues {
            label: "marks".into(),
            values: vec!["core".into()],
        },
        LabelValues {
            label: "phase".into(),
            values: vec!["draft".into()],
        },
    ];
    SurfaceModel {
        title: "Requirements".into(),
        cause: causes::OPEN.into(),
        nodes: vec![
            level("1", "", "Orders"),
            take,
            level("3", "1", "Payment"),
            entry("4", "3", "Pay by card"),
            level("5", "3", "Refunds"),
            entry("6", "5", "Refund in full"),
            level("7", "", "Shipping"),
        ],
        option_lists: vec![OptionList {
            label: "marks".into(),
            options: vec![SurfaceOption {
                id: "marks/core".into(),
                name: "core".into(),
                colour: "green".into(),
                implicit: false,
            }],
        }],
        ..SurfaceModel::default()
    }
}

/// A gesture as a test reads it: its operation, its target and its params.
type Sent = (String, String, Vec<(String, String)>);

/// One page in a headless window, its frames' texts and accessibility tree kept.
struct Surface {
    ctx: egui::Context,
    tab: HierarchyTab,
    gestures: Rc<RefCell<Vec<Gesture>>>,
    script: Rc<RefCell<Script>>,
    /// The window's width; the page is narrower by its margins.
    window: f32,
    time: f64,
    texts: Vec<String>,
    /// Each run of the last frame's texts, with the family of the font it is drawn in.
    runs: Vec<(String, egui::FontFamily)>,
    /// The rectangles of the last frame that are filled with a colour that lets what is under it show.
    veils: Vec<egui::Rect>,
    nodes: HashMap<NodeId, Node>,
    /// Whether the properties pane is drawn right of the page, as the shell draws it.
    pane: bool,
}

impl Surface {
    fn of(model: SurfaceModel) -> Surface {
        Surface::at(model, 1280.0)
    }

    /// A page `width` points wide, as `pageWidth` gives the standalone test's.
    fn at(model: SurfaceModel, width: f32) -> Surface {
        Surface::with(model, definition(), width)
    }

    /// A page `width` points wide drawn by `definition`.
    fn with(model: SurfaceModel, definition: SurfaceDefinition, width: f32) -> Surface {
        let gestures = Rc::new(RefCell::new(Vec::new()));
        let script = Rc::new(RefCell::new(Script::default()));
        let session = Recorder {
            model,
            gestures: gestures.clone(),
            script: script.clone(),
            changes: Vec::new(),
        };
        let path = Path::new("requirements.md");
        let tab = HierarchyTab::with_session(path, path, Box::new(session), definition);
        let ctx = egui::Context::default();
        theme::install(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        ctx.enable_accesskit();
        let mut surface = Surface {
            ctx,
            tab,
            gestures,
            script,
            window: width + 2.0 * PAGE_MARGIN,
            time: 0.0,
            texts: Vec::new(),
            veils: Vec::new(),
            runs: Vec::new(),
            nodes: HashMap::new(),
            pane: false,
        };
        // The page measures itself in its first frame and lays out by that in the next; the window is widened by
        // whatever the page's frame takes besides its margins.
        surface.frame(Vec::new());
        surface.window += width - surface.tab.width;
        surface.frame(Vec::new());
        surface.frame(Vec::new());
        assert_eq!(surface.tab.width, width, "the page is as wide as asked");
        surface
    }

    fn frame(&mut self, events: Vec<egui::Event>) {
        self.time += 0.15;
        let input = egui::RawInput {
            time: Some(self.time),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.window, 800.0),
            )),
            events,
            ..Default::default()
        };
        let theme = adp_theme::Theme::dark();
        let palette = Palette::of(&theme);
        let tab = &mut self.tab;
        let pane = self.pane;
        let mut output = self.ctx.run_ui(input, |ui| {
            if pane {
                egui::Panel::right("pane")
                    .default_size(280.0)
                    .show(ui, |ui| super::properties::show(ui, tab, &theme, &palette));
            }
            super::show(ui, tab, &theme, &palette)
        });
        output.textures_delta.clear();
        self.runs = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.job.clone()),
                _ => None,
            })
            .flat_map(|job| {
                job.sections
                    .iter()
                    .map(|s| {
                        (
                            job.text[s.byte_range.start.0..s.byte_range.end.0].to_owned(),
                            s.format.font_id.family.clone(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        self.veils = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect) if rect.fill.a() > 0 && rect.fill.a() < 255 => Some(rect.rect),
                _ => None,
            })
            .collect();
        self.texts = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.text().to_owned()),
                _ => None,
            })
            .collect();
        self.nodes = output
            .platform_output
            .accesskit_update
            .expect("AccessKit is on")
            .nodes
            .into_iter()
            .collect();
    }

    /// Whether a text was painted in the last frame.
    fn drawn(&self, text: &str) -> bool {
        self.texts.iter().any(|t| t == text)
    }

    /// What a screen reader would read of the last frame that is an icon's glyph: every name and value, of a widget it
    /// is not told to pass over, holding a character of the private-use planes the icons' font draws from.
    fn glyphs_read(&self) -> Vec<String> {
        let private = |c: char| ('\u{E000}'..='\u{F8FF}').contains(&c) || c >= '\u{F0000}';
        self.nodes
            .values()
            .filter(|n| !n.is_hidden())
            .flat_map(|n| [n.label(), n.value()])
            .flatten()
            .filter(|text| text.chars().any(private))
            .map(|text| text.escape_unicode().to_string())
            .collect()
    }

    /// The names of the boxes drawn, in the order they were.
    fn boxes(&self) -> Vec<String> {
        let mut boxes: Vec<(f64, f64, String)> = self
            .nodes
            .values()
            .filter(|n| n.role() == Role::TreeItem)
            .map(|n| {
                let at = n.bounds().unwrap_or_default();
                (at.y0, at.x0, n.label().unwrap_or_default().to_owned())
            })
            .collect();
        boxes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        boxes.into_iter().map(|b| b.2).collect()
    }

    /// The widget of `role` named `name`, as the last frame described it.
    fn find(&self, role: Role, name: &str) -> Option<&Node> {
        self.nodes
            .values()
            .find(|n| n.role() == role && n.label() == Some(name))
    }

    fn get(&self, role: Role, name: &str) -> &Node {
        self.find(role, name).unwrap_or_else(|| {
            let all: Vec<String> = self
                .nodes
                .values()
                .filter_map(|n| n.label().map(|l| format!("{:?} {l}", n.role())))
                .collect();
            panic!("{role:?} '{name}' is drawn: {all:?}")
        })
    }

    /// The role and name of every node the widget of `role` named `name` stands in, nearest first.
    fn ancestors(&self, role: Role, name: &str) -> Vec<(Role, String)> {
        let mut at = *self
            .nodes
            .iter()
            .find(|(_, n)| n.role() == role && n.label() == Some(name))
            .unwrap_or_else(|| panic!("{role:?} '{name}' is drawn"))
            .0;
        let mut found = Vec::new();
        while let Some((id, parent)) = self.nodes.iter().find(|(_, n)| n.children().contains(&at)) {
            found.push((parent.role(), parent.label().unwrap_or_default().to_owned()));
            at = *id;
        }
        found
    }

    /// Whether the widget of `role` named `name` stands in one of `within`'s role named `group`.
    fn stands_in(&self, role: Role, name: &str, within: Role, group: &str) -> bool {
        self.ancestors(role, name)
            .iter()
            .any(|(r, n)| *r == within && n == group)
    }

    /// The centre of the widget of `role` named `name`.
    fn centre(&self, role: Role, name: &str) -> egui::Pos2 {
        let at = self.get(role, name).bounds().expect("it has bounds");
        egui::pos2(((at.x0 + at.x1) / 2.0) as f32, ((at.y0 + at.y1) / 2.0) as f32)
    }

    /// Presses and lets go at `at`, each in its own frame, the pointer moved there first, and lets the page settle.
    fn click_at(&mut self, at: egui::Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    /// Clicks the widget of `role` named `name`, the pointer moved over it first so that a control shown under the
    /// pointer is drawn.
    fn click(&mut self, role: Role, name: &str) {
        let at = self.centre(role, name);
        self.frame(vec![egui::Event::PointerMoved(at)]);
        let at = self.centre(role, name);
        self.click_at(at);
    }

    /// Clicks a box away from its controls: at its bottom left, inside its padding.
    fn click_box(&mut self, name: &str) {
        let at = self.get(Role::TreeItem, name).bounds().expect("it has bounds");
        self.click_at(egui::pos2(at.x0 as f32 + 3.0, at.y1 as f32 - 3.0));
    }

    /// The trail's steps as they read: the document's title, then the levels down to the selection or the focus.
    fn trail(&self) -> Vec<String> {
        let mut steps = vec![self.tab.model.title.clone()];
        steps.extend(self.tab.trail().iter().map(|n| format!("{} {}", n.number, n.title)));
        for step in &steps {
            assert!(self.drawn(step), "the trail shows '{step}'");
        }
        steps
    }

    fn gestures(&self) -> Vec<Gesture> {
        self.gestures.borrow().clone()
    }

    /// The gestures sent, each as its operation, its target and its params.
    fn sent(&self) -> Vec<Sent> {
        self.gestures()
            .into_iter()
            .map(|g| (g.operation, g.target_id, g.params.into_iter().collect()))
            .collect()
    }

    /// Presses `key` with `modifiers` and lets the page settle.
    fn key(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers,
        };
        self.frame(vec![event(true), event(false)]);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    /// Types `text` into what has the keyboard.
    fn type_text(&mut self, text: &str) {
        self.frame(vec![egui::Event::Text(text.to_owned())]);
        self.frame(Vec::new());
    }

    /// Clicks twice at `at`, quickly, and lets the page settle.
    fn double_click_at(&mut self, at: egui::Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(at)]);
        self.frame(vec![button(true), button(false), button(true), button(false)]);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    /// Presses the grip of the box numbered `number` and moves to `to`, frame by frame, without letting go.
    fn drag_to(&mut self, number: &str, to: egui::Pos2) {
        let from = self.centre(Role::Button, &format!("Drag {number}"));
        let press = egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(from)]);
        self.frame(vec![press]);
        // In one move, as the standalone test puts the box under the pointer: a place shown on the way would move it.
        self.frame(vec![egui::Event::PointerMoved(to)]);
        self.frame(Vec::new());
    }

    /// Lets go of the pointer where it is, and lets the page settle.
    fn release(&mut self) {
        let at = self
            .ctx
            .input(|i| i.pointer.latest_pos())
            .expect("the pointer is on the page");
        self.frame(vec![egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    /// Drags the box numbered `number` by its grip to `to`, and drops it there.
    fn drag_and_drop(&mut self, number: &str, to: egui::Pos2) {
        self.drag_to(number, to);
        self.release();
    }

    /// A point in the lower or the upper half of the box named `name`, away from its controls.
    fn half_of(&self, name: &str, lower: bool) -> egui::Pos2 {
        let at = self.get(Role::TreeItem, name).bounds().unwrap();
        let y = if lower { at.y1 - 4.0 } else { at.y0 + 4.0 };
        egui::pos2(((at.x0 + at.x1) / 2.0) as f32, y as f32)
    }

    /// The height of the box named `name`.
    fn height_of(&self, name: &str) -> f64 {
        let at = self.get(Role::TreeItem, name).bounds().unwrap();
        at.y1 - at.y0
    }

    /// Another program leaves the file as `model` says, and the watcher tells the tab.
    fn outside(&mut self, mut model: SurfaceModel, cause: &str) {
        model.cause = cause.into();
        self.script.borrow_mut().outside = Some(model);
        self.tab.changed_on_disk(Path::new("requirements.md"));
        self.frame(Vec::new());
        self.frame(Vec::new());
    }

    /// The field open on the page, by its name: its value, and whether it has the keyboard.
    fn field(&self, name: &str) -> Option<(String, bool)> {
        let (id, node) = self
            .nodes
            .iter()
            .find(|(_, n)| matches!(n.role(), Role::TextInput | Role::MultilineTextInput) && n.label() == Some(name))?;
        let focused = self
            .ctx
            .memory(|m| m.focused())
            .is_some_and(|f| f.accesskit_id() == *id);
        Some((node.value().unwrap_or_default().to_owned(), focused))
    }
}

const ALL: [&str; 7] = [
    "[1] Orders",
    "[2] Take an order",
    "[3] Payment",
    "[4] Pay by card",
    "[5] Refunds",
    "[6] Refund in full",
    "[7] Shipping",
];

#[test]
fn the_whole_document_is_drawn_as_one_page_with_the_trail_above_it() {
    // Act.
    let surface = Surface::of(three_levels());

    // Assert: every node once, as a box, and the trail the document's title alone.
    assert_eq!(surface.boxes().len(), 7);
    for name in ALL {
        assert!(surface.find(Role::TreeItem, name).is_some(), "{name} is a box");
    }
    for title in [
        "Orders",
        "Take an order",
        "Payment",
        "Pay by card",
        "Refunds",
        "Refund in full",
        "Shipping",
    ] {
        assert_eq!(
            surface.texts.iter().filter(|t| *t == title).count(),
            1,
            "{title} is drawn once"
        );
    }
    assert_eq!(surface.trail(), ["Requirements"]);
}

#[test]
fn a_levels_entries_stand_to_its_right_above_its_sub_levels() {
    // Act.
    let surface = Surface::of(three_levels());

    // Assert: Orders' box, then to its right the point and below it the section, all from the box's top.
    let bounds = |name: &str| surface.get(Role::TreeItem, name).bounds().unwrap();
    let (orders, take, payment) = (bounds("[1] Orders"), bounds("[2] Take an order"), bounds("[3] Payment"));
    assert_eq!(take.y0, orders.y0);
    assert_eq!(take.x0, orders.x1 - 1.0, "the boxes share a border");
    assert_eq!(payment.x0, take.x0);
    assert_eq!(payment.y0, take.y1 - 1.0);
    assert!(
        orders.y1 >= bounds("[6] Refund in full").y1,
        "a level is as high as what stands to its right"
    );
}

#[test]
fn the_number_stands_beside_the_title_the_tags_below_it_and_the_description_under_them() {
    // Act.
    let surface = Surface::of(three_levels());

    // Assert: in the order they are painted, the number after the title on its line, then the tags, then the
    // description; a label of many values is a tag written with #, one of one value a badge.
    let at = |text: &str| {
        surface
            .texts
            .iter()
            .position(|t| t == text)
            .unwrap_or_else(|| panic!("{text}"))
    };
    assert!(at("Take an order") < at("[2]"));
    assert!(at("[2]") < at("#core"));
    assert!(at("#core") < at("draft"));
    assert!(at("draft") < at("Any open order can be taken."));
}

#[test]
fn as_many_levels_are_shown_as_fit_and_a_level_whose_deeper_levels_do_not_carries_a_mark() {
    // Act: room for two levels beside the entries' least width.
    let surface = Surface::at(three_levels(), 760.0);

    // Assert: the third level is not drawn; the second carries the mark, the first does not.
    assert!(surface.find(Role::TreeItem, "[5] Refunds").is_none());
    assert!(surface.find(Role::TreeItem, "[4] Pay by card").is_some());
    let mark = surface.get(Role::Button, "1 more level: show the levels of [3] Payment");
    let payment = surface.get(Role::TreeItem, "[3] Payment").bounds().unwrap();
    let inside = mark.bounds().unwrap();
    assert!(inside.x0 >= payment.x0 && inside.x1 <= payment.x1 && inside.y0 >= payment.y0);
    assert!(surface.drawn(&icons::text(HIDDEN_LEVELS_ICON)));
    assert!(
        !surface
            .nodes
            .values()
            .any(|n| n.label().is_some_and(|l| l.ends_with("show the levels of [1] Orders")))
    );
}

#[test]
fn a_level_is_focused_by_its_mark_with_a_trail_leading_back_and_nothing_is_sent() {
    // Arrange.
    let mut surface = Surface::at(three_levels(), 760.0);

    // Act.
    surface.click(Role::Button, "1 more level: show the levels of [3] Payment");

    // Assert: the page starts at the level, its hidden level is drawn, and the trail names what is above it.
    assert!(!surface.drawn("Orders"));
    assert!(!surface.drawn("Shipping"));
    assert!(surface.drawn("Refund in full"));
    assert_eq!(surface.trail(), ["Requirements", "[1] Orders", "[3] Payment"]);
    assert_eq!(surface.tab.trail_current(), Some("3"));

    // Act: back through the trail, one level and then the whole page.
    surface.click(Role::Link, "[1] Orders");
    assert!(surface.drawn("Orders"));
    assert!(!surface.drawn("Shipping"));
    surface.click(Role::Link, "Requirements");

    // Assert.
    assert!(surface.drawn("Shipping"));
    assert_eq!(surface.trail(), ["Requirements"]);
    assert!(surface.gestures().is_empty());
}

#[test]
fn a_screen_reader_reads_no_icon_as_its_glyph_in_the_bars_the_trail_or_the_page() {
    // Arrange: a read-only document, so that its lock shows, with a finding and a level focused, so that the trail
    // has a step after its first.
    let mut model = three_levels();
    model.read_only_reason = "Read-only.".into();
    model.findings = vec![SurfaceFinding {
        code: "std.missingId".into(),
        severity: "warning".into(),
        message: "Refund in full has no number.".into(),
        node_id: "6".into(),
        line: 12,
    }];
    let mut surface = Surface::at(model, 760.0);

    // Act.
    surface.click(Role::Button, "1 more level: show the levels of [3] Payment");

    // Assert.
    assert_eq!(surface.glyphs_read(), Vec::<String>::new());
}

#[test]
fn the_mark_shows_the_hidden_levels_at_every_level_also_where_only_one_level_fits() {
    // Arrange: room for one level beside the entries' least width.
    let mut surface = Surface::at(three_levels(), 600.0);
    assert!(!surface.drawn("Payment"));

    // Act: the mark of the top level; then the mark of the level it brought.
    surface.click(Role::Button, "2 more levels: show the levels of [1] Orders");

    // Assert: its sub-levels stand in its place, since beside it they cannot fit.
    assert!(surface.find(Role::TreeItem, "[3] Payment").is_some());
    assert!(surface.drawn("Pay by card"));
    assert!(!surface.drawn("Orders"));

    // Act.
    surface.click(Role::Button, "1 more level: show the levels of [3] Payment");

    // Assert: and so on, a level deeper; the trail leads back to the level.
    assert!(surface.drawn("Refund in full"));
    assert!(!surface.drawn("Pay by card"));
    surface.click(Role::Link, "[3] Payment");
    assert!(surface.drawn("Pay by card"));
}

#[test]
fn the_trail_always_shows_the_levels_down_to_the_one_the_selected_node_stands_in() {
    // Arrange.
    let mut surface = Surface::of(three_levels());

    // Act and assert.
    surface.click_box("[4] Pay by card");
    assert_eq!(surface.trail(), ["Requirements", "[1] Orders", "[3] Payment"]);
    assert_eq!(surface.get(Role::TreeItem, "[4] Pay by card").is_selected(), Some(true));
    surface.click_box("[5] Refunds");
    assert_eq!(
        surface.trail(),
        ["Requirements", "[1] Orders", "[3] Payment", "[5] Refunds"]
    );
    surface.click_box("[7] Shipping");
    assert_eq!(surface.trail(), ["Requirements", "[7] Shipping"]);
    assert!(surface.gestures().is_empty());
}

#[test]
fn a_level_is_focused_by_the_control_in_its_header() {
    // Arrange.
    let mut surface = Surface::of(three_levels());

    // Act: the control shows under the pointer.
    let refunds = surface.centre(Role::TreeItem, "[5] Refunds");
    surface.frame(vec![egui::Event::PointerMoved(refunds)]);
    assert!(surface.drawn(&icons::text(FOCUS_ICON)));
    surface.click(Role::Button, "Focus on [5] Refunds");

    // Assert.
    assert!(!surface.drawn("Pay by card"));
    assert!(surface.drawn("Refund in full"));
    assert_eq!(
        surface.trail(),
        ["Requirements", "[1] Orders", "[3] Payment", "[5] Refunds"]
    );
    assert!(surface.gestures().is_empty());
}

#[test]
fn an_entry_offers_no_focus() {
    // Arrange.
    let mut surface = Surface::of(three_levels());

    // Act.
    surface.click_box("[2] Take an order");

    // Assert: the selected entry shows its controls, and focus is not among them.
    assert!(surface.find(Role::Button, "Focus on [2] Take an order").is_none());
    assert!(surface.find(Role::Button, "Collapse [2] Take an order").is_some());
}

#[test]
fn a_collapsed_entry_shows_its_number_title_and_tags_and_not_its_description() {
    // Arrange.
    let mut model = three_levels();
    model.nodes[1].collapsed = true;

    // Act.
    let surface = Surface::of(model);

    // Assert.
    assert!(surface.drawn("[2]"));
    assert!(surface.drawn("#core"));
    assert!(!surface.drawn("Any open order can be taken."));
    assert_eq!(
        surface.get(Role::TreeItem, "[2] Take an order").is_expanded(),
        Some(false)
    );
    assert!(surface.find(Role::Button, "Expand [2] Take an order").is_some());
}

#[test]
fn a_collapsed_level_hides_what_it_holds() {
    // Arrange.
    let mut model = three_levels();
    model.nodes[2].collapsed = true;

    // Act.
    let surface = Surface::of(model);

    // Assert.
    assert!(!surface.drawn("Pay by card"));
    assert!(!surface.drawn("Refunds"));
    assert!(surface.drawn("Payment"));
}

#[test]
fn collapsing_a_numbered_node_sends_the_definitions_operation_and_the_page_waits_for_the_model() {
    // Arrange.
    let mut surface = Surface::of(three_levels());

    // Act.
    surface.click(Role::Button, "Collapse [3] Payment");

    // Assert: the document keeps it, so the page waits for the model that says so.
    let gestures = surface.gestures();
    assert_eq!(gestures.len(), 1);
    assert_eq!(
        (
            gestures[0].operation.as_str(),
            gestures[0].target_id.as_str(),
            gestures[0].params.len()
        ),
        ("fold", "3", 0)
    );
    assert!(surface.drawn("Pay by card"));
}

#[test]
fn a_node_with_no_number_of_its_own_is_collapsed_on_the_page_alone() {
    // Arrange.
    let mut model = three_levels();
    model.nodes[1].id = "line/4".into();
    model.nodes[1].number = "[?]".into();
    model.nodes[1].description = "Unnumbered.".into();
    let mut surface = Surface::of(model);
    assert!(surface.drawn("Unnumbered."));

    // Act.
    surface.click(Role::Button, "Collapse [?] Take an order");

    // Assert.
    assert!(surface.gestures().is_empty());
    assert!(!surface.drawn("Unnumbered."));
}

#[test]
fn the_findings_are_listed_and_a_read_only_document_says_why_it_cannot_be_edited() {
    // Arrange.
    let mut model = three_levels();
    model.read_only_reason = "This document was written by a later version.".into();
    model.findings = vec![SurfaceFinding {
        code: "std.missingId".into(),
        severity: "warning".into(),
        message: "Refund in full has no number.".into(),
        node_id: "6".into(),
        line: 12,
    }];
    let mut surface = Surface::at(model, 760.0);
    assert!(surface.find(Role::TreeItem, "[6] Refund in full").is_none());

    // Act.
    surface.click(Role::Link, "Refund in full has no number.");

    // Assert: the reason and the finding's line are shown, and the node it names is brought into sight, selected.
    assert!(surface.drawn("This document was written by a later version."));
    assert!(surface.drawn("line 12"));
    assert_eq!(
        surface.get(Role::TreeItem, "[6] Refund in full").is_selected(),
        Some(true)
    );
}

#[test]
fn a_finding_reveals_its_node_inside_a_collapsed_level_by_expanding_it() {
    // Arrange.
    let mut model = three_levels();
    model.nodes[4].collapsed = true;
    model.findings = vec![SurfaceFinding {
        code: "std.missingId".into(),
        severity: "warning".into(),
        message: "Refund in full has no number.".into(),
        node_id: "6".into(),
        line: 12,
    }];
    let mut surface = Surface::of(model);
    assert!(!surface.drawn("Refund in full"));

    // Act.
    surface.click(Role::Link, "Refund in full has no number.");

    // Assert: the level is expanded by the definition's operation, as the file keeps it.
    let gestures = surface.gestures();
    assert_eq!(gestures.len(), 1);
    assert_eq!(
        (gestures[0].operation.as_str(), gestures[0].target_id.as_str()),
        ("fold", "5")
    );
}

#[test]
fn a_read_only_document_collapses_on_the_page_alone() {
    // Arrange.
    let mut model = three_levels();
    model.read_only_reason = "Read-only.".into();
    let mut surface = Surface::of(model);

    // Act.
    surface.click(Role::Button, "Collapse [3] Payment");

    // Assert.
    assert!(surface.gestures().is_empty());
    assert!(!surface.drawn("Pay by card"));
}

// ---- Changing the page (the standalone host's `editing/AddControl.test.tsx` at a7fdf03; T038) ----

/// 1 holds the points 2 and 3 and the section 4, which holds the point 5; 6 stands at the top beside 1.
fn plan(nodes: Option<Vec<SurfaceNode>>) -> SurfaceModel {
    SurfaceModel {
        title: "Plan".into(),
        cause: causes::OPEN.into(),
        nodes: nodes.unwrap_or_else(|| {
            vec![
                level("1", "", "Orders"),
                entry("2", "1", "Take"),
                entry("3", "1", "Keep"),
                level("4", "1", "Payment"),
                entry("5", "4", "Pay"),
                level("6", "", "Shipping"),
            ]
        }),
        ..SurfaceModel::default()
    }
}

fn sent(operation: &str, target: &str, params: &[(&str, &str)]) -> Sent {
    (
        operation.into(),
        target.into(),
        params.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect(),
    )
}

#[test]
fn the_add_controls_stand_at_the_end_of_every_level_and_of_the_page_and_add_without_a_dialog() {
    // Arrange.
    let mut surface = Surface::of(plan(None));

    // Act.
    surface.click(Role::Button, "New point in [4]");
    surface.click(Role::Button, "New section in [1]");
    surface.click(Role::Button, "New section");

    // Assert: every level has both, the page one, each sent at once, and nothing asked.
    let points = surface
        .nodes
        .values()
        .filter(|n| n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("New point in ")))
        .count();
    assert_eq!(points, 3);
    assert_eq!(
        surface.sent(),
        [sent("jot", "4", &[]), sent("nest", "1", &[]), sent("open", "", &[])]
    );
    assert!(surface.tab.deleting.is_none());
    // Each add is its icon alone, below what the level holds.
    assert!(surface.drawn(&icons::text("mdi-note-plus-outline")));
    assert!(surface.drawn(&icons::text("mdi-folder-plus-outline")));
    let pay = surface.get(Role::TreeItem, "[5] Pay").bounds().unwrap();
    let control = surface.get(Role::Button, "New point in [4]").bounds().unwrap();
    assert!(control.y0 > pay.y1 && control.x0 > pay.x0, "under what [4] holds");
}

#[test]
fn the_added_nodes_title_opens_for_typing_when_it_arrives_and_it_is_selected() {
    // Arrange: the add's model brings 7 at the end of 4.
    let mut surface = Surface::of(plan(None));
    let mut after = plan(None).nodes;
    after.insert(5, entry("7", "4", "New point"));
    surface.script.borrow_mut().next.push(plan(Some(after)));

    // Act.
    surface.click(Role::Button, "New point in [4]");

    // Assert.
    assert_eq!(surface.sent(), [sent("jot", "4", &[])]);
    assert_eq!(surface.field("Title of [7]"), Some(("New point".to_owned(), true)));
    assert_eq!(surface.tab.selected, "7");
}

#[test]
fn nothing_opens_when_the_add_was_refused() {
    // Arrange.
    let mut surface = Surface::of(plan(None));
    let mut after = plan(None).nodes;
    after.push(entry("7", "6", "Written by another program"));
    surface.script.borrow_mut().next.push(plan(Some(after)));
    surface.script.borrow_mut().refusal = "No more points here.".into();

    // Act.
    surface.click(Role::Button, "New point in [4]");

    // Assert.
    assert!(surface.tab.field.is_none());
    assert!(surface.drawn("No more points here."));
}

#[test]
fn ctrl_enter_and_ctrl_shift_enter_on_a_selected_node_run_its_actions_and_open_what_was_added() {
    // Arrange.
    let mut surface = Surface::of(plan(None));
    surface.click_box("[3] Keep");

    // Act: inside it, plain Enter, then below it with the node the action brings.
    surface.key(egui::Key::Enter, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT);
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);
    let mut after = plan(None).nodes;
    after.insert(3, entry("8", "1", "New point"));
    surface.script.borrow_mut().next.push(plan(Some(after)));
    surface.key(egui::Key::Enter, egui::Modifiers::COMMAND);

    // Assert: each its action's operation for the node; plain Enter is no key of the page's.
    assert_eq!(
        surface.sent(),
        [sent("jot-inside", "3", &[]), sent("jot-below", "3", &[])]
    );
    assert_eq!(surface.field("Title of [8]"), Some(("New point".to_owned(), true)));
}

#[test]
fn the_added_node_is_found_though_an_edit_numbered_a_placeholder_named_by_its_line() {
    // Arrange: a placeholder, line/9, is given a number by the edit that adds 13.
    let before = [level("1", "", "Orders"), entry("line/9", "1", "Not numbered yet")];
    let after = [
        level("1", "", "Orders"),
        entry("12", "1", "Not numbered yet"),
        entry("13", "1", "New point"),
    ];

    // Act and assert.
    assert_eq!(editing::added_node(&before, &after).map(|n| n.id.as_str()), Some("13"));
    assert_eq!(editing::added_node(&after, &after), None);
}

#[test]
fn a_title_is_typed_in_place_and_kept_by_enter_and_a_description_by_ctrl_enter() {
    // Arrange.
    let mut surface = Surface::of(three_levels());

    // Act: the title opened by a double click on it, typed over and kept.
    let title = surface.half_of("[2] Take an order", false);
    surface.double_click_at(title);
    assert_eq!(surface.field("Title of [2]"), Some(("Take an order".to_owned(), true)));
    surface.type_text("Take any order");
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);

    // Act: the description opened, a line added, Enter a line break, and Ctrl+Enter keeps it.
    let description = surface.half_of("[2] Take an order", true);
    surface.double_click_at(description + egui::vec2(0.0, -10.0));
    assert!(surface.field("Description of [2]").is_some_and(|(_, focused)| focused));
    surface.key(egui::Key::End, egui::Modifiers::COMMAND);
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);
    surface.type_text("Or a closed one.");
    surface.key(egui::Key::Enter, egui::Modifiers::COMMAND);

    // Assert.
    assert_eq!(
        surface.sent(),
        [
            sent("retitle", "2", &[("title", "Take any order")]),
            sent(
                "redescribe",
                "2",
                &[("description", "Any open order can be taken.\nOr a closed one.")]
            ),
        ]
    );
    assert!(surface.tab.field.is_none());
}

#[test]
fn escape_drops_what_was_typed_and_an_unchanged_text_is_no_edit() {
    // Arrange.
    let mut surface = Surface::of(three_levels());
    let title = surface.half_of("[4] Pay by card", false);

    // Act: typed over and dropped; opened again and left as it was.
    surface.double_click_at(title);
    surface.type_text("Pay by cash");
    surface.key(egui::Key::Escape, egui::Modifiers::NONE);
    surface.double_click_at(title);
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);

    // Assert.
    assert!(surface.gestures().is_empty(), "{:?}", surface.sent());
    assert!(surface.tab.field.is_none());
}

#[test]
fn a_box_is_as_high_unselected_selected_and_with_its_title_or_description_open() {
    // Arrange: an entry with a description, one without, which offers to add one, and a level whose title leaves its
    // controls a column, with the place of its delete below them.
    let mut model = three_levels();
    model.nodes[6].title = "Shipping to every country we deliver to".into();
    let mut surface = Surface::of(model);
    let heights = |surface: &Surface| {
        (
            surface.height_of("[2] Take an order"),
            surface.height_of("[4] Pay by card"),
            surface.height_of("[7] Shipping to every country we deliver to"),
        )
    };
    let unselected = heights(&surface);

    // Act and assert: Story 7, scenario 7.
    surface.click_box("[7] Shipping to every country we deliver to");
    assert_eq!(heights(&surface), unselected, "a level selected");
    surface.click_box("[2] Take an order");
    surface.click_box("[4] Pay by card");
    assert_eq!(heights(&surface), unselected, "selected");
    surface.tab.open_field("2", Attribute::Title);
    surface.frame(Vec::new());
    assert_eq!(heights(&surface), unselected, "its title open");
    surface.tab.open_field("2", Attribute::Description);
    surface.frame(Vec::new());
    assert_eq!(heights(&surface), unselected, "its description open");
}

#[test]
fn a_description_stands_4_below_the_tags_and_12_above_the_boxs_bottom_edge() {
    // Found by the walk of T054: the standalone host's description has 4 above it and 6 below it inside the box's
    // padding of 6 (`.hs-description`); the page had 4 above it and none below it.
    let mut model = three_levels();
    model.nodes[1].description = String::new();
    let without = Surface::of(model).height_of("[2] Take an order");

    let with = Surface::of(three_levels()).height_of("[2] Take an order");

    // One line of 20, 4 above it and 6 below it.
    assert_eq!(with - without, f64::from(4.0 + LINE_HEIGHT + 6.0));
}

#[test]
fn a_level_is_moved_with_everything_in_it_after_a_level_from_its_lower_half_and_into_it_from_its_upper_one() {
    // Arrange.
    let mut surface = Surface::of(plan(None));

    // Act.
    let lower = surface.half_of("[6] Shipping", true);
    surface.drag_and_drop("[4]", lower);
    let upper = surface.half_of("[6] Shipping", false);
    surface.drag_and_drop("[4]", upper);

    // Assert: one move each, naming the level; what it holds goes with it.
    assert_eq!(
        surface.sent(),
        [
            sent("shift", "4", &[("behind", "6"), ("section", "")]),
            sent("shift", "4", &[("behind", ""), ("section", "6")]),
        ]
    );
}

#[test]
fn an_entry_is_moved_alone_after_or_before_another_entry_or_into_a_level() {
    // Arrange.
    let mut surface = Surface::of(plan(None));

    // Act.
    let take = surface.half_of("[2] Take", true);
    surface.drag_and_drop("[5]", take);
    let keep = surface.half_of("[3] Keep", false);
    surface.drag_and_drop("[5]", keep);
    let shipping = surface.half_of("[6] Shipping", true);
    surface.drag_and_drop("[2]", shipping);

    // Assert.
    assert_eq!(
        surface.sent(),
        [
            sent("slide", "5", &[("behind", "2"), ("section", "1")]),
            sent("slide", "5", &[("behind", "2"), ("section", "1")]),
            sent("slide", "2", &[("behind", ""), ("section", "6")]),
        ]
    );
}

#[test]
fn the_dragged_node_is_shown_where_it_would_stand_and_dropped_there_from_that_place() {
    // Arrange.
    let mut surface = Surface::of(plan(None));
    let take = surface.half_of("[2] Take", true);

    // Act: dragged over the lower half of Take, not yet dropped.
    surface.drag_to("[5]", take);

    // Assert: its place drawn after Take with its number and title, and the node itself still in its own.
    assert!(surface.drawn("[5] Pay"));
    let slot = surface.tab.hint.clone().expect("a drop is shown");
    assert_eq!((slot.node_id.as_str(), slot.kind), ("2", DropKind::After));
    assert!(surface.find(Role::TreeItem, "[5] Pay").is_some());
    assert!(surface.gestures().is_empty());
    let keep = surface.get(Role::TreeItem, "[3] Keep").bounds().unwrap();
    let take_box = surface.get(Role::TreeItem, "[2] Take").bounds().unwrap();
    assert!(keep.y0 > take_box.y1 + 10.0, "the place stands between Take and Keep");

    // Act: the place moved Keep away from under the pointer; over the place the drop stays, and is made there.
    let over = egui::pos2(take.x, (take_box.y1 + keep.y0) as f32 / 2.0);
    surface.frame(vec![egui::Event::PointerMoved(over)]);
    assert!(surface.tab.hint.is_some());
    surface.release();

    // Assert.
    assert_eq!(
        surface.sent(),
        [sent("slide", "5", &[("behind", "2"), ("section", "1")])]
    );
    assert!(surface.tab.hint.is_none() && !surface.drawn("[5] Pay"));
}

#[test]
fn a_dragged_level_is_faded_with_everything_it_carries() {
    // Found by the walk of T059: the standalone host fades the level and its content (`.hs-dragging`), the page only
    // the level's own box.
    let mut surface = Surface::of(three_levels());
    let shipping = surface.half_of("[7] Shipping", true);

    surface.drag_to("[3]", shipping);

    let faded = |surface: &Surface, name: &str| {
        let at = surface.get(Role::TreeItem, name).bounds().unwrap();
        surface.veils.iter().any(|v| {
            (f64::from(v.min.x) - at.x0).abs() < 0.5
                && (f64::from(v.min.y) - at.y0).abs() < 0.5
                && (f64::from(v.max.x) - at.x1).abs() < 0.5
        })
    };
    assert!(faded(&surface, "[3] Payment"));
    assert!(faded(&surface, "[4] Pay by card"));
    assert!(faded(&surface, "[6] Refund in full"));
    assert!(!faded(&surface, "[2] Take an order"));
}

#[test]
fn a_drop_without_a_sibling_is_shown_where_the_move_puts_it() {
    // Arrange: 1 shows 2, 3 and 4.
    let model = plan(None);
    let index = HierarchyIndex::of(&model.nodes);
    let shown = index.children_of("1").to_vec();
    let hint = |id: &str, into: &str| {
        let over = index.by_id[into];
        DropHint::of(
            index.by_id[id],
            over,
            EntryDrop {
                into: into.into(),
                after: String::new(),
            },
        )
    };

    // Act and assert: an entry of its own level to the front, one from another after the last entry; a level of its
    // own level before the first level, one from another at the end.
    assert_eq!(editing::slot_index(&shown, &hint("3", "1")), Some(0));
    assert_eq!(editing::slot_index(&shown, &hint("5", "1")), Some(2));
    assert_eq!(editing::slot_index(&shown, &hint("4", "1")), Some(2));
    assert_eq!(editing::slot_index(&shown, &hint("6", "1")), Some(3));
}

#[test]
fn a_level_is_not_dropped_into_itself_or_into_what_it_holds() {
    let model = plan(None);
    let index = HierarchyIndex::of(&model.nodes);
    let node = |id: &str| index.by_id[id];
    assert_eq!(editing::drop_of(&index, node("1"), node("1"), false), None);
    assert_eq!(editing::drop_of(&index, node("1"), node("4"), false), None);
    assert_eq!(editing::drop_of(&index, node("1"), node("5"), true), None);
    assert_eq!(editing::drop_of(&index, node("2"), node("2"), true), None);
}

#[test]
fn delete_deletes_at_once_a_node_nothing_concerns() {
    // Arrange.
    let mut surface = Surface::of(plan(None));
    surface.click_box("[3] Keep");

    // Act.
    surface.key(egui::Key::Delete, egui::Modifiers::NONE);

    // Assert.
    assert_eq!(surface.sent(), [sent("drop", "3", &[])]);
    assert!(surface.tab.deleting.is_none());
}

fn note(id: &str, about: &str, names: Option<&str>) -> Note {
    Note {
        id: id.into(),
        kind: "Doubt".into(),
        about: format!("[{about}]"),
        about_id: about.into(),
        references: names
            .map(|target| {
                vec![Reference {
                    start: 0,
                    length: 3,
                    target_id: target.into(),
                }]
            })
            .unwrap_or_default(),
        ..Note::default()
    }
}

#[test]
fn delete_asks_first_in_the_definitions_words_when_the_node_holds_anything_or_is_named() {
    // Arrange: 4 holds 5; a note is about it, another names it, and 2's description names it.
    let mut model = plan(None);
    model.nodes[1].references = vec![Reference {
        start: 0,
        length: 3,
        target_id: "4".into(),
    }];
    model.notes = vec![note("0", "4", None), note("1", "6", Some("4")), note("2", "6", None)];
    let mut surface = Surface::of(model);
    surface.click_box("[4] Payment");

    // Act: asked, cancelled, asked again and confirmed.
    surface.key(egui::Key::Delete, egui::Modifiers::NONE);
    assert!(surface.drawn("Drop [4]? 4 things concern it."));
    surface.click(Role::Button, "Cancel");
    assert!(surface.gestures().is_empty() && surface.tab.deleting.is_none());
    surface.key(egui::Key::Delete, egui::Modifiers::NONE);
    surface.click(Role::Button, "Delete");

    // Assert.
    assert_eq!(surface.sent(), [sent("drop", "4", &[])]);
    assert!(!surface.drawn("Drop [4]? 4 things concern it."));
}

#[test]
fn a_selected_boxs_delete_stands_at_its_bottom_right_and_always_asks_first() {
    // Arrange.
    let mut surface = Surface::of(plan(None));
    assert!(surface.find(Role::Button, "Delete [3] Keep").is_none());
    surface.click_box("[3] Keep");

    // Assert: 6 points from the box's right and bottom.
    let keep = surface.get(Role::TreeItem, "[3] Keep").bounds().unwrap();
    let control = surface.get(Role::Button, "Delete [3] Keep").bounds().unwrap();
    assert_eq!((keep.x1 - control.x1, keep.y1 - control.y1), (6.0, 6.0));

    // Act: nothing concerns it, and it still asks.
    surface.click(Role::Button, "Delete [3] Keep");
    assert!(surface.drawn("Delete [3] Keep? Nothing else concerns it."));
    assert!(surface.gestures().is_empty());

    // Act: confirmed; then a level that holds something asks in the definition's words.
    surface.click(Role::Button, "Delete");
    surface.click_box("[4] Payment");
    surface.click(Role::Button, "Delete [4] Payment");

    // Assert.
    assert!(surface.drawn("Drop [4]? 1 things concern it."));
    assert_eq!(surface.sent(), [sent("drop", "3", &[])]);
}

#[test]
fn a_document_that_cannot_change_offers_no_add_no_grip_and_no_delete() {
    // Arrange.
    let mut model = plan(None);
    model.read_only_reason = "Read only.".into();
    let mut surface = Surface::of(model);

    // Act.
    surface.click_box("[3] Keep");
    surface.key(egui::Key::Delete, egui::Modifiers::NONE);

    // Assert.
    assert!(surface.find(Role::Button, "Delete [3] Keep").is_none());
    assert!(surface.find(Role::Button, "Drag [3]").is_none());
    assert!(surface.find(Role::Button, "New section").is_none());
    assert!(surface.gestures().is_empty());
}

#[test]
fn an_open_field_keeps_delete_and_enter_for_itself() {
    // Arrange.
    let mut surface = Surface::of(plan(None));
    surface.click_box("[3] Keep");
    let title = surface.half_of("[3] Keep", false);
    surface.double_click_at(title);
    assert!(surface.field("Title of [3]").is_some());

    // Act.
    surface.key(egui::Key::Delete, egui::Modifiers::NONE);
    surface.key(egui::Key::Enter, egui::Modifiers::COMMAND);

    // Assert: the Enter kept the field, whose text Delete changed, and nothing else was sent.
    assert!(surface.tab.field.is_none());
    assert!(surface.sent().iter().all(|(operation, _, _)| operation == "retitle"));
}

// ---- A change from outside while a field is open (the standalone host's `ConflictBar.test.tsx` at a7fdf03; T033) ----

const CONFLICT: &str = "This file was changed outside the designer.";

/// The page as another program left it: the first point renamed, and with `without` gone.
fn renamed(without: Option<&str>) -> SurfaceModel {
    let mut model = three_levels();
    model.nodes.retain(|n| Some(n.id.as_str()) != without);
    model.nodes.iter_mut().find(|n| n.id == "2").unwrap().title = "Take an order by phone".into();
    model
}

/// Opens the title of the point `[4] Pay by card` and types over it.
fn type_a_title(surface: &mut Surface) {
    let title = surface.half_of("[4] Pay by card", false);
    surface.double_click_at(title);
    surface.type_text("Pay any card");
}

#[test]
fn a_change_from_outside_while_a_field_is_open_shows_the_file_around_it_and_asks_what_to_do_with_the_text() {
    // Arrange.
    let mut surface = Surface::of(three_levels());
    type_a_title(&mut surface);

    // Act: another program renames the first point.
    surface.outside(renamed(None), causes::OUTSIDE);

    // Assert: the page is the file's, the text is still there, and the bar offers both sides.
    assert!(surface.drawn(CONFLICT));
    assert!(
        surface.find(Role::Alert, CONFLICT).is_some(),
        "the sentence is an alert"
    );
    assert!(surface.find(Role::TreeItem, "[2] Take an order by phone").is_some());
    assert_eq!(
        surface.field("Title of [4]").map(|f| f.0),
        Some("Pay any card".to_owned())
    );
    surface.get(Role::Button, "Show the file");
    surface.get(Role::Button, "Keep my text");
    assert!(surface.gestures().is_empty());
}

#[test]
fn show_the_file_drops_the_text_and_sends_nothing_though_the_focus_left_for_the_bar() {
    // Arrange.
    let mut surface = Surface::of(three_levels());
    type_a_title(&mut surface);
    surface.outside(renamed(None), causes::OUTSIDE);

    // Act: the press on the button takes the focus from the field, which keeps no text by leaving for the bar.
    surface.click(Role::Button, "Show the file");

    // Assert.
    assert!(!surface.drawn(CONFLICT));
    assert!(surface.tab.field.is_none());
    assert!(surface.find(Role::TreeItem, "[4] Pay by card").is_some());
    assert!(surface.gestures().is_empty(), "{:?}", surface.sent());
}

#[test]
fn keep_my_text_keeps_the_field_and_commits_it_as_an_edit_of_the_file_as_it_now_is() {
    // Arrange.
    let mut surface = Surface::of(three_levels());
    type_a_title(&mut surface);
    surface.outside(renamed(None), causes::OUTSIDE);

    // Act.
    surface.click(Role::Button, "Keep my text");
    assert!(!surface.drawn(CONFLICT));
    assert_eq!(surface.field("Title of [4]"), Some(("Pay any card".to_owned(), true)));
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);

    // Assert.
    assert_eq!(surface.sent(), [sent("retitle", "4", &[("title", "Pay any card")])]);
}

#[test]
fn the_text_of_a_number_the_file_no_longer_has_is_still_sent_for_the_file_to_refuse() {
    // Arrange: another program removed the point being typed.
    let mut surface = Surface::of(three_levels());
    type_a_title(&mut surface);
    surface.outside(renamed(Some("4")), causes::OUTSIDE);

    // Act.
    surface.click(Role::Button, "Keep my text");
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);

    // Assert: the page cannot know the number is gone for good; the session refuses it with the sentence of R10.
    assert_eq!(surface.sent(), [sent("retitle", "4", &[("title", "Pay any card")])]);
}

#[test]
fn nothing_is_asked_for_the_pages_own_edit_nor_while_no_field_is_open() {
    // Arrange.
    let mut surface = Surface::of(three_levels());
    type_a_title(&mut surface);

    // Act: the page's own edit of another point arrives while the field is open.
    surface.outside(renamed(None), causes::EDIT);

    // Assert.
    assert!(!surface.drawn(CONFLICT));
    assert_eq!(
        surface.field("Title of [4]").map(|f| f.0),
        Some("Pay any card".to_owned())
    );

    // Act: the field closed, a change from outside.
    surface.key(egui::Key::Escape, egui::Modifiers::NONE);
    surface.outside(renamed(None), causes::OUTSIDE);

    // Assert.
    assert!(!surface.drawn(CONFLICT));
}

// ---- A box's notes (the standalone host's `notes/NoteIcon.test.tsx` at a7fdf03; T042) ----

/// The section 1 holds the points 2 and 3 and the section 4, which holds the point 5.
fn noted(notes: Vec<Note>) -> SurfaceModel {
    SurfaceModel {
        title: "Notes".into(),
        cause: causes::OPEN.into(),
        nodes: vec![
            level("1", "", "Orders"),
            entry("2", "1", "Take an order"),
            entry("3", "1", "Cancel an order"),
            level("4", "1", "Payment"),
            entry("5", "4", "Pay by card"),
        ],
        notes,
        ..SurfaceModel::default()
    }
}

fn assessment(id: &str, about: &str, kind: &str) -> Note {
    Note {
        id: id.into(),
        kind: kind.into(),
        about: format!("[{about}]"),
        about_id: about.into(),
        text: format!("note {id}"),
        ..Note::default()
    }
}

fn resolved(mut note: Note) -> Note {
    note.resolved = true;
    note
}

fn collapsed_at(id: &str, mut model: SurfaceModel) -> SurfaceModel {
    model.nodes.iter_mut().find(|n| n.id == id).unwrap().collapsed = true;
    model
}

impl Surface {
    /// The widget of `role` named `name` whose bounds lie within those of `outer`.
    fn inside(&self, outer: &Node, role: Role, name: &str) -> Option<&Node> {
        let o = outer.bounds()?;
        self.nodes.values().find(|n| {
            n.role() == role
                && n.label() == Some(name)
                && n.bounds().is_some_and(|b| {
                    b.x0 >= o.x0 - 0.5 && b.x1 <= o.x1 + 0.5 && b.y0 >= o.y0 - 0.5 && b.y1 <= o.y1 + 0.5
                })
        })
    }

    /// The names of the widgets of `role`, top to bottom and left to right.
    fn named(&self, role: Role) -> Vec<String> {
        let mut found: Vec<(f64, f64, String)> = self
            .nodes
            .values()
            .filter(|n| n.role() == role)
            .map(|n| {
                let at = n.bounds().unwrap_or_default();
                (at.y0, at.x0, n.label().unwrap_or_default().to_owned())
            })
            .collect();
        found.sort_by(|a, b| a.partial_cmp(b).unwrap());
        found.into_iter().map(|f| f.2).collect()
    }

    /// The icon at the top right of the box named `name`, under the pointer so that the way to add is shown.
    fn note_icon(&mut self, name: &str) -> Option<String> {
        let at = self.half_of(name, false);
        self.frame(vec![egui::Event::PointerMoved(at)]);
        let outer = self.get(Role::TreeItem, name).bounds()?;
        self.nodes
            .values()
            .filter(|n| n.role() == Role::Button)
            .filter_map(|n| Some((n.bounds()?, n.label()?.to_owned())))
            .filter(|(b, _)| {
                b.y0 >= outer.y0 && b.y1 <= outer.y0 + f64::from(2.0 * BOX_SPACING + ICON_SIZE) && b.x1 <= outer.x1
            })
            .max_by(|a, b| a.0.x1.partial_cmp(&b.0.x1).unwrap())
            .map(|(_, label)| label)
    }

    /// Opens the list of the box named `name` from its icon.
    fn open_notes(&mut self, name: &str) {
        let label = self.note_icon(name).expect("the box has its notes' icon");
        self.click(Role::Button, &label);
    }

    fn dialog(&self) -> Option<&Node> {
        self.nodes.values().find(|n| n.role() == Role::Dialog)
    }

    /// Picks a kind in the list's choice named `label`.
    fn choose_kind(&mut self, label: &str, kind: &str) {
        self.click(Role::ComboBox, label);
        self.click(Role::ListBoxOption, kind);
    }
}

#[test]
fn every_box_offers_the_way_to_add_the_first_note_first_of_its_controls_at_its_top_right() {
    let mut surface = Surface::of(noted(Vec::new()));

    for (name, number) in [
        ("[1] Orders", "[1]"),
        ("[2] Take an order", "[2]"),
        ("[4] Payment", "[4]"),
        ("[5] Pay by card", "[5]"),
    ] {
        assert_eq!(surface.note_icon(name), Some(format!("New note on {number}")), "{name}");
        let icon = surface
            .get(Role::Button, &format!("New note on {number}"))
            .bounds()
            .unwrap();
        let outer = surface.get(Role::TreeItem, name).bounds().unwrap();
        assert!(
            (outer.x1 - BOX_SPACING as f64 - icon.x1).abs() < 0.5,
            "{name}: the icon stands at the corner"
        );
    }
}

#[test]
fn a_selected_box_of_one_line_has_its_delete_under_its_notes_icon_not_over_it() {
    let mut surface = Surface::of(noted(vec![assessment("n1", "2", "Doubt")]));
    surface.click_box("[2] Take an order");
    surface.frame(Vec::new());

    let icon = surface.get(Role::Button, "Doubt: note n1").bounds().unwrap();
    let delete = surface.get(Role::Button, "Delete [2] Take an order").bounds().unwrap();
    assert!(delete.y0 >= icon.y1, "{icon:?} {delete:?}");
    assert!((delete.x1 - icon.x1).abs() < 0.5, "in the same column");
}

#[test]
fn a_document_that_cannot_change_offers_no_way_to_add_a_note() {
    let mut model = noted(Vec::new());
    model.read_only_reason = "Read only.".into();
    let mut surface = Surface::of(model);

    assert_ne!(
        surface.note_icon("[2] Take an order").as_deref(),
        Some("New note on [2]")
    );
    assert!(surface.find(Role::Button, "New note on [2]").is_none());
}

#[test]
fn one_open_note_shows_by_its_kind_and_several_by_their_count_in_the_words_of_the_list() {
    let notes = vec![
        assessment("n1", "2", "Aside"),
        assessment("n2", "3", "Doubt"),
        assessment("n3", "3", "Aside"),
        resolved(assessment("n4", "3", "Doubt")),
    ];
    let definition = definition();
    let ids = |id: &str| HashSet::from([id.to_owned()]);

    // The kind's icon for one, the icon for several with their count; the resolved note is not counted.
    let one = notes::mark_of(&notes, &ids("2"), &definition).unwrap();
    assert_eq!((one.icon.as_str(), one.count), ("mdi-comment-text-outline", 1));
    let two = notes::mark_of(&notes, &ids("3"), &definition).unwrap();
    assert_eq!((two.icon.as_str(), two.count), ("mdi-comment-multiple-outline", 2));
    assert_eq!(notes::mark_of(&notes, &ids("5"), &definition), None);

    // On the page, by what they say.
    let mut surface = Surface::of(noted(notes));
    assert_eq!(
        surface.note_icon("[2] Take an order").as_deref(),
        Some("Aside: note n1")
    );
    assert_eq!(
        surface.note_icon("[3] Cancel an order").as_deref(),
        Some("2 open notes")
    );
    assert!(surface.drawn("2"), "the count is drawn beside the icon");
}

#[test]
fn a_collapsed_section_counts_every_open_note_inside_it_and_only_its_own_while_open() {
    let notes = vec![
        assessment("n1", "2", "Aside"),
        assessment("n2", "5", "Doubt"),
        assessment("n3", "4", "Aside"),
    ];

    let mut surface = Surface::of(collapsed_at("1", noted(notes.clone())));
    assert_eq!(surface.note_icon("[1] Orders").as_deref(), Some("3 open notes"));

    let mut surface = Surface::of(noted(notes));
    assert_eq!(surface.note_icon("[1] Orders").as_deref(), Some("New note on [1]"));
}

#[test]
fn the_list_opens_from_the_icon_and_names_each_notes_entry_kind_text_and_detail_resolved_ones_too() {
    // Arrange: a collapsed section lists everything inside it, in the file's order.
    let mut aside = assessment("n1", "2", "Aside");
    aside.detail = "Ask the shop.".into();
    let notes = vec![
        aside,
        resolved(assessment("n2", "5", "Doubt")),
        assessment("n3", "3", "Doubt"),
    ];
    let mut surface = Surface::of(collapsed_at("1", noted(notes)));

    // Act.
    surface.open_notes("[1] Orders");

    // Assert.
    let dialog = surface.dialog().expect("the list is open").clone();
    assert_eq!(dialog.label(), Some("Notes of [1]"));
    assert_eq!(
        surface.named(Role::ListItem),
        ["[2] Aside: note n1", "[5] Doubt: note n2", "[3] Doubt: note n3"]
    );
    assert!(surface.drawn("Ask the shop."));
    assert!(surface.inside(&dialog, Role::Image, "Aside").is_some());
    let row = |s: &Surface, name: &str| s.get(Role::ListItem, name).clone();
    let second = row(&surface, "[5] Doubt: note n2");
    assert_eq!(second.description(), Some("resolved"));
    assert!(surface.inside(&second, Role::Button, "Unsettle").is_some());
    let third = row(&surface, "[3] Doubt: note n3");
    assert!(surface.inside(&third, Role::Button, "Settle").is_some());
    assert_eq!(surface.get(Role::Button, "2 open notes").is_expanded(), Some(true));
    assert_eq!(surface.glyphs_read(), Vec::<String>::new());
}

#[test]
fn a_note_is_added_about_the_boxs_node_with_the_kind_chosen_and_the_text_typed_and_none_without_a_text() {
    // Arrange.
    let mut surface = Surface::of(noted(Vec::new()));
    surface.open_notes("[2] Take an order");

    // Act: nothing typed but spaces, then Add.
    surface.click(Role::MultilineTextInput, "New note about [2]");
    surface.type_text("  ");
    surface.click(Role::Button, "Add");
    surface.click(Role::MultilineTextInput, "New note about [2]");
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);
    assert!(surface.gestures().is_empty());

    // Act: the kind chosen, a text typed and Enter.
    surface.choose_kind("Kind of the new one about [2]", "Aside");
    surface.click(Role::MultilineTextInput, "New note about [2]");
    surface.key(egui::Key::A, egui::Modifiers::COMMAND);
    surface.type_text("Which shop?");
    surface.key(egui::Key::Enter, egui::Modifiers::NONE);

    // Assert: one gesture, and the box neither selected nor its title opened by it.
    assert_eq!(
        surface.sent(),
        [sent("remark", "2", &[("line", "Which shop?"), ("sort", "Aside")])]
    );
    assert!(surface.tab.field.is_none());
}

#[test]
fn a_note_is_resolved_removed_and_changed_each_as_the_definitions_operation() {
    // Arrange.
    let mut surface = Surface::of(noted(vec![assessment("assessment/0", "2", "Doubt")]));
    surface.open_notes("[2] Take an order");

    // Act.
    surface.click(Role::Button, "Settle");
    surface.click(Role::Button, "Strike");
    surface.click(Role::Button, "Reword");
    assert_eq!(
        surface.field("Text about [2]").map(|f| f.1),
        Some(true),
        "the text is ready for typing"
    );
    surface.choose_kind("Kind of [2]", "Aside");
    surface.click(Role::MultilineTextInput, "Text about [2]");
    surface.key(egui::Key::A, egui::Modifiers::COMMAND);
    surface.type_text("Settled by the shop");
    surface.click(Role::MultilineTextInput, "Detail about [2]");
    surface.type_text("See the call.");
    surface.click(Role::Button, "Reword");

    // Assert.
    assert_eq!(
        surface.sent(),
        [
            sent("settle", "assessment/0", &[]),
            sent("strike", "assessment/0", &[]),
            sent(
                "reword",
                "assessment/0",
                &[
                    ("line", "Settled by the shop"),
                    ("more", "See the call."),
                    ("sort", "Aside")
                ]
            ),
        ]
    );
}

#[test]
fn the_list_keeps_its_keys_from_the_page_and_closes_on_escape_and_on_a_press_outside() {
    // Arrange.
    let mut surface = Surface::of(noted(vec![assessment("n1", "2", "Doubt")]));
    surface.click_box("[2] Take an order");
    surface.open_notes("[2] Take an order");

    // Act: Delete and Ctrl+Enter, which are the page's delete and add of the selected node.
    surface.key(egui::Key::Delete, egui::Modifiers::NONE);
    surface.key(egui::Key::Enter, egui::Modifiers::COMMAND);

    // Assert.
    assert!(surface.gestures().is_empty(), "{:?}", surface.sent());
    assert!(surface.dialog().is_some());

    // Act and assert: Escape, wherever the focus is, closes it.
    surface.key(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(surface.dialog().is_none());

    // Act and assert: a press outside closes it too, here on the section above it.
    surface.open_notes("[2] Take an order");
    assert!(surface.dialog().is_some());
    surface.click_box("[1] Orders");
    assert!(surface.dialog().is_none());
}

#[test]
fn escape_drops_a_change_and_keeps_the_list() {
    let mut surface = Surface::of(noted(vec![assessment("n1", "2", "Doubt")]));
    surface.open_notes("[2] Take an order");
    surface.click(Role::Button, "Reword");
    assert!(surface.field("Text about [2]").is_some());

    surface.key(egui::Key::Escape, egui::Modifiers::NONE);

    assert!(surface.field("Text about [2]").is_none());
    assert!(surface.dialog().is_some());
    assert!(surface.gestures().is_empty());
}

#[test]
fn the_list_stands_above_every_pane_under_the_boxs_icons() {
    let mut surface = Surface::of(noted(vec![assessment("n1", "2", "Doubt")]));
    surface.open_notes("[2] Take an order");

    let dialog = surface.dialog().unwrap().bounds().unwrap();
    let icon = surface.get(Role::Button, "Doubt: note n1").bounds().unwrap();
    let centre = egui::pos2(
        ((dialog.x0 + dialog.x1) / 2.0) as f32,
        ((dialog.y0 + dialog.y1) / 2.0) as f32,
    );
    assert_eq!(
        surface.ctx.layer_id_at(centre).map(|l| l.order),
        Some(egui::Order::Foreground)
    );
    assert!(
        (dialog.x1 - icon.x1).abs() < 1.0,
        "its right edge on the icons': {dialog:?} {icon:?}"
    );
    assert!(dialog.y0 >= icon.y1, "under them");
}

#[test]
fn a_new_note_is_its_kind_then_its_text_over_three_lines_then_add_below_on_the_right() {
    let mut surface = Surface::of(noted(Vec::new()));
    surface.open_notes("[2] Take an order");

    // Each kind with its icon, in the definition's order.
    surface.click(Role::ComboBox, "Kind of the new one about [2]");
    assert_eq!(surface.named(Role::ListBoxOption), ["Doubt", "Aside"]);
    surface.key(egui::Key::Escape, egui::Modifiers::NONE);
    assert!(surface.dialog().is_some(), "Escape closes the kinds, not the list");

    let kind = surface
        .get(Role::ComboBox, "Kind of the new one about [2]")
        .bounds()
        .unwrap();
    let text = surface
        .get(Role::MultilineTextInput, "New note about [2]")
        .bounds()
        .unwrap();
    let add = surface.get(Role::Button, "Add").bounds().unwrap();
    let dialog = surface.dialog().unwrap().bounds().unwrap();
    assert!(kind.y1 <= text.y0 && text.y1 <= add.y0, "one below the other");
    assert!(
        text.y1 - text.y0 >= 3.0 * f64::from(LINE_HEIGHT) - 6.0,
        "three lines high"
    );
    assert!(dialog.x1 - add.x1 < 12.0, "Add at the right");
}

#[test]
fn a_notes_resolve_change_and_remove_are_icons_side_by_side_under_a_note_of_one_line() {
    let mut surface = Surface::of(noted(vec![assessment("n1", "2", "Doubt")]));
    surface.open_notes("[2] Take an order");

    let row = surface.get(Role::ListItem, "[2] Doubt: note n1").clone();
    let buttons: Vec<(f64, f64, String)> = ["Settle", "Reword", "Strike"]
        .iter()
        .map(|name| {
            let b = surface.inside(&row, Role::Button, name).expect(name).bounds().unwrap();
            assert!((b.x1 - b.x0 - 22.0).abs() < 0.5, "{name} is an icon");
            (b.y0, b.x0, (*name).to_owned())
        })
        .collect();
    assert!(
        buttons.windows(2).all(|w| w[0].1 < w[1].1 && w[0].0 == w[1].0),
        "{buttons:?}"
    );
}

#[test]
fn a_document_that_cannot_change_only_shows_its_notes() {
    let mut model = noted(vec![assessment("n1", "2", "Doubt")]);
    model.read_only_reason = "Read only.".into();
    let mut surface = Surface::of(model);

    surface.open_notes("[2] Take an order");

    let dialog = surface.dialog().expect("the list is open").clone();
    assert!(surface.drawn("note n1"));
    assert!(
        surface
            .nodes
            .values()
            .filter(|n| n.role() == Role::Button || n.role() == Role::MultilineTextInput)
            .filter(|n| surface
                .inside(&dialog, n.role(), n.label().unwrap_or_default())
                .is_some())
            .count()
            == 0
    );
}

// ---- Revealing a node (the standalone host's `references/reveal.test.ts` at a7fdf03; T044) ----

/// 1 holds 2, which holds 3, which holds 4, which holds the entry 5; 6 stands beside 1 and holds the entry 7. 2 and 4
/// are collapsed.
fn deep() -> Vec<SurfaceNode> {
    let mut nodes = vec![
        level("1", "", "1"),
        level("2", "1", "2"),
        level("3", "2", "3"),
        level("4", "3", "4"),
        entry("5", "4", "5"),
        level("6", "", "6"),
        entry("7", "6", "7"),
    ];
    nodes[1].collapsed = true;
    nodes[3].collapsed = true;
    nodes
}

fn revealed(target: &str, focused: &str, fit: usize) -> Option<(String, Vec<String>)> {
    let nodes = deep();
    let index = HierarchyIndex::of(&nodes);
    index
        .reveal_of(target, focused, fit, |n| n.collapsed)
        .map(|plan| (plan.focus, plan.expand))
}

#[test]
fn a_reveal_keeps_the_focus_when_the_node_is_inside_it_and_fits_and_expands_every_collapsed_level_above_it() {
    // 2 is the focus and collapsed, so it is expanded too.
    assert_eq!(
        revealed("5", "2", usize::MAX),
        Some(("2".into(), vec!["2".into(), "4".into()]))
    );
}

#[test]
fn a_reveal_moves_the_focus_to_the_deepest_level_the_node_shares_with_it_and_to_the_whole_page_when_they_share_none() {
    let focus = |target, focused| revealed(target, focused, usize::MAX).unwrap().0;
    assert_eq!(focus("5", "6"), "");
    assert_eq!(focus("7", "4"), "");
    assert_eq!(focus("3", "4"), "3");
}

#[test]
fn a_reveal_moves_the_focus_down_only_as_far_as_the_nodes_level_needs_to_fit_the_page() {
    // 5 stands in 4, the fourth level; two columns show 3 and 4, one column 4 alone.
    let focus = |target, focused, fit| revealed(target, focused, fit).unwrap().0;
    assert_eq!(focus("5", "", 2), "3");
    assert_eq!(focus("5", "1", 1), "4");
    assert_eq!(focus("5", "2", 3), "2");
    assert_eq!(focus("1", "", 1), "");
}

#[test]
fn a_reveal_asks_for_nothing_to_be_expanded_when_nothing_hides_the_node_and_shows_no_node_the_document_lacks() {
    assert_eq!(revealed("7", "", usize::MAX), Some((String::new(), Vec::new())));
    assert_eq!(revealed("99", "", usize::MAX), None);
}

// ---- The query (the standalone host's `narrowing/filterQuery.test.ts` at a7fdf03; T044, T048) ----

mod query {
    use super::*;
    use crate::hierarchy::narrowing::*;

    /// Made-up names throughout: the query knows only the prefixes the definition gives its labels.
    fn labels() -> Vec<LabelDefinition> {
        vec![
            label("marks", "~", true, &["Section", "Point"]),
            label("k", "k", false, &["Point"]),
            label("kind", "kind:", false, &["Point"]),
            label("phase", "phase:", false, &["Point"]),
        ]
    }

    fn values(label: &str, values: &[&str]) -> LabelValues {
        LabelValues {
            label: label.into(),
            values: values.iter().map(|v| (*v).to_owned()).collect(),
        }
    }

    fn point(id: &str, parent: &str, role: &str, title: &str, marks: &[&str], kind: &str, phase: &str) -> SurfaceNode {
        let mut node = node(id, role, "", parent, title);
        node.number = if role == roles::LEVEL {
            format!("S-{id}")
        } else {
            format!("P-{id}")
        };
        let one = |v: &&str| if v.is_empty() { 0 } else { 1 };
        node.labels = vec![
            values("marks", marks),
            values("kind", &[kind][..one(&kind)]),
            values("phase", &[phase][..one(&phase)]),
        ];
        node
    }

    /// Two top sections, Backend (1) and Frontend (5); Backend holds the points 2 and 3 and the section Billing (4)
    /// with the point 9; Frontend, which carries safe, holds the points 6 and 10. The numbers sort as 2, 3, 9, 10, not
    /// as text.
    fn model() -> SurfaceModel {
        let (l, e) = (roles::LEVEL, roles::ENTRY);
        SurfaceModel {
            nodes: vec![
                point("1", "", l, "Backend", &[], "", ""),
                point("3", "1", e, "Store an order", &["Fast"], "rule", "done"),
                point("2", "1", e, "audit an order", &["safe"], "goal", ""),
                point("4", "1", l, "Billing", &[], "", ""),
                point("9", "4", e, "Charge a card", &["safe", "fast"], "", "draft"),
                point("5", "", l, "Frontend", &["safe"], "", ""),
                point("10", "5", e, "Show an order", &["fast"], "goal", "draft"),
                point("6", "5", e, "Book a slot", &[], "rule", "done"),
            ],
            option_lists: vec![OptionList {
                label: "phase".into(),
                options: ["draft", "done"]
                    .iter()
                    .map(|name| SurfaceOption {
                        id: format!("phase/{name}"),
                        name: (*name).into(),
                        colour: "gray".into(),
                        implicit: false,
                    })
                    .collect(),
            }],
            ..SurfaceModel::default()
        }
    }

    /// The titles the query leaves on the page, in the file's order.
    fn shown_in(page: &SurfaceModel, query: &str) -> Vec<String> {
        let index = HierarchyIndex::of(&page.nodes);
        let narrowing = Narrowing::of(page, &index, &parse_query(query, &labels()));
        page.nodes
            .iter()
            .filter(|n| narrowing.shows(n))
            .map(|n| n.title.clone())
            .collect()
    }

    fn shown(query: &str) -> Vec<String> {
        shown_in(&model(), query)
    }

    /// The titles of the entries of `level` in the query's order.
    fn ordered_in(page: &SurfaceModel, query: &str, level: &str) -> Vec<String> {
        let index = HierarchyIndex::of(&page.nodes);
        let narrowing = Narrowing::of(page, &index, &parse_query(query, &labels()));
        let entries = page
            .nodes
            .iter()
            .filter(|n| n.parent_id == level && n.role == roles::ENTRY)
            .collect();
        narrowing.order(entries).iter().map(|n| n.title.clone()).collect()
    }

    fn ordered(query: &str, level: &str) -> Vec<String> {
        ordered_in(&model(), query, level)
    }

    #[test]
    fn a_query_reads_each_word_by_the_longest_prefix_that_fits_it_and_knows_the_item_and_sort_words_in_any_case() {
        let words = parse_query("  ~fast kind:rule kx ITEM:back* Sort:Phase plain ", &labels());

        // kind: is not the k label followed by ind:rule; a prefix alone is no word of its label.
        let read: Vec<String> = words
            .iter()
            .map(|w| format!("{:?} {} {}", w.kind, w.label, w.value))
            .collect();
        assert_eq!(
            read,
            [
                "Label marks fast",
                "Label kind rule",
                "Label k x",
                "Item  back*",
                "Sort  phase",
                "Unknown  plain"
            ]
        );
        let alone: Vec<WordKind> = parse_query("~ item:", &labels()).iter().map(|w| w.kind).collect();
        assert_eq!(alone, [WordKind::Unknown, WordKind::Unknown]);
        assert_eq!(query_text(&words), "~fast kind:rule kx ITEM:back* Sort:Phase plain");
    }

    #[test]
    fn a_query_counts_the_first_sort_word_and_changes_the_sort_in_its_place_or_adds_it_last_or_takes_every_one_out() {
        let words = parse_query("~fast sort:title kind:goal sort:number", &labels());

        assert_eq!(sort_of(&words), "title");
        assert_eq!(query_text(&with_sort(&words, "phase")), "~fast sort:phase kind:goal");
        assert_eq!(
            query_text(&with_sort(&parse_query("~fast", &labels()), "number")),
            "~fast sort:number"
        );
        assert_eq!(query_text(&with_sort(&words, "")), "~fast kind:goal");
    }

    #[test]
    fn narrowing_leaves_everything_when_the_query_is_empty_or_only_sorts() {
        assert_eq!(shown("").len(), 8);
        assert_eq!(shown("sort:title").len(), 8);
        let page = model();
        let index = HierarchyIndex::of(&page.nodes);
        assert!(!Narrowing::of(&page, &index, &parse_query("sort:title", &labels())).narrows);
    }

    #[test]
    fn narrowing_leaves_the_entries_that_carry_a_tag_in_any_case_with_the_levels_that_contain_them() {
        assert_eq!(
            shown("~FAST"),
            [
                "Backend",
                "Store an order",
                "Billing",
                "Charge a card",
                "Frontend",
                "Show an order"
            ]
        );
    }

    #[test]
    fn narrowing_takes_the_words_of_one_label_as_alternatives_and_combines_the_labels() {
        // Safe or fast, and goal; then draft, and the kind of 9 is none, so it goes.
        assert_eq!(
            shown("~safe ~fast kind:goal"),
            ["Backend", "audit an order", "Frontend", "Show an order"]
        );
        assert_eq!(shown("kind:rule kind:goal phase:draft"), ["Frontend", "Show an order"]);
    }

    #[test]
    fn narrowing_narrows_by_state_and_by_category_alone() {
        let expected = ["Backend", "Store an order", "Frontend", "Book a slot"];
        assert_eq!(shown("phase:done"), expected);
        assert_eq!(shown("kind:Rule"), expected);
    }

    #[test]
    fn narrowing_selects_levels_by_a_title_pattern_of_star_and_question_mark_in_any_case_with_everything_in_them_and_keeps_the_levels_above()
     {
        // Billing stays with Backend above it; b?ck* is Backend alone.
        assert_eq!(shown("item:bill*"), ["Backend", "Billing", "Charge a card"]);
        assert_eq!(
            shown("item:B?CK*"),
            [
                "Backend",
                "Store an order",
                "audit an order",
                "Billing",
                "Charge a card"
            ]
        );
        assert!(shown("item:end").is_empty());
        assert!(shown("item:?lling").is_empty());
    }

    #[test]
    fn narrowing_combines_a_pattern_with_the_labels_the_entries_that_carry_them_in_the_levels_it_selects() {
        assert_eq!(
            shown("item:back* ~fast"),
            ["Backend", "Store an order", "Billing", "Charge a card"]
        );
        assert_eq!(
            shown("item:billing item:frontend phase:draft"),
            ["Backend", "Billing", "Charge a card", "Frontend", "Show an order"]
        );
    }

    #[test]
    fn narrowing_leaves_nothing_for_a_value_no_entry_carries() {
        assert!(shown("~none").is_empty());
    }

    #[test]
    fn sorting_keeps_the_files_order_without_a_sort_and_sorts_by_number_as_numbers_and_by_title_in_any_case() {
        assert_eq!(ordered("", "5"), ["Show an order", "Book a slot"]);
        assert_eq!(ordered("sort:number", "5"), ["Book a slot", "Show an order"]);
        assert_eq!(ordered("sort:title", "1"), ["audit an order", "Store an order"]);
    }

    #[test]
    fn sorting_by_a_label_of_one_value_follows_its_lists_order_those_without_a_value_last_and_keeps_the_files_order_of_equals()
     {
        // A third Frontend point without a phase, and a fourth that is done.
        let mut page = model();
        page.nodes
            .push(point("11", "5", roles::ENTRY, "Undo a slot", &[], "", ""));
        page.nodes
            .push(point("12", "5", roles::ENTRY, "Rebook a slot", &[], "", "done"));

        assert_eq!(
            ordered_in(&page, "sort:phase", "5"),
            ["Show an order", "Book a slot", "Rebook a slot", "Undo a slot"]
        );
    }

    #[test]
    fn sorting_keeps_the_files_order_for_a_key_it_does_not_know() {
        assert_eq!(ordered("sort:colour", "5"), ["Show an order", "Book a slot"]);
    }

    fn unhidden(query: &str, node: &str) -> String {
        let page = model();
        let index = HierarchyIndex::of(&page.nodes);
        query_text(&unhiding(&page, &index, &parse_query(query, &labels()), node))
    }

    #[test]
    fn unhiding_keeps_a_query_that_already_shows_the_node() {
        assert_eq!(unhidden("~fast sort:title", "3"), "~fast sort:title");
        assert_eq!(unhidden("~fast", "1"), "~fast");
    }

    #[test]
    fn unhiding_drops_only_the_words_that_hide_it_the_labels_it_does_not_carry_and_the_patterns_it_stands_outside() {
        // 6 is a rule, done, without marks, in Frontend.
        assert_eq!(unhidden("~fast kind:rule sort:number", "6"), "kind:rule sort:number");
        assert_eq!(unhidden("item:back* phase:done", "6"), "phase:done");
        assert_eq!(unhidden("item:front* ~fast ~safe", "2"), "~fast ~safe");
    }

    #[test]
    fn unhiding_drops_every_label_word_for_a_level_which_holds_no_entry_that_stays() {
        assert_eq!(unhidden("item:front* ~none sort:title", "1"), "sort:title");
        assert_eq!(unhidden("item:back* ~none", "4"), "item:back*");
        assert_eq!(unhidden("~safe", "5"), "");
    }

    fn sort_keys() -> Vec<adp_desl::hierarchy::definition::SortKey> {
        vec![adp_desl::hierarchy::definition::SortKey {
            key: "number".into(),
            label: "By number".into(),
        }]
    }

    fn offered(typed: &str) -> Vec<String> {
        completions_of(typed, &labels(), &model().option_lists, &sort_keys())
            .into_iter()
            .map(|c| c.word)
            .collect()
    }

    #[test]
    fn completing_finds_the_word_the_caret_stands_in_and_what_of_it_is_typed() {
        assert_eq!(
            word_at("~fast pha sort:number", 8),
            WordAt {
                start: 6,
                end: 9,
                typed: "ph".into()
            }
        );
    }

    #[test]
    fn completing_offers_the_members_of_a_labels_list_with_its_prefix_in_any_case_then_the_prefixes_and_the_sort_keys_that_fit()
     {
        assert_eq!(offered("PHASE:D"), ["phase:draft", "phase:done"]);
        assert_eq!(offered("so"), ["sort:", "sort:number"]);
    }

    #[test]
    fn completing_writes_the_whole_word_the_caret_stands_in_a_space_after_a_word_and_none_after_a_prefix() {
        let labels = labels();
        let at = |text: &str, caret, completion| completed(text, &word_at(text, caret), completion, &labels);

        // The caret inside a word takes the whole word; what follows is parted from a prefix by one space.
        assert_eq!(at("~fxst", 2, "~fast"), ("~fast ".into(), 6));
        assert_eq!(at("ph   item:x", 2, "phase:"), ("phase: item:x".into(), 6));
        assert_eq!(at("~safe so", 8, "sort:"), ("~safe sort:".into(), 11));
    }

    #[test]
    fn completing_offers_the_prefixes_for_nothing_typed_and_nothing_again_for_a_word_written_in_full() {
        assert_eq!(offered(""), ["~", "k", "kind:", "phase:", "item:", "sort:"]);
        assert!(offered("phase:done").is_empty());
    }
}

// ---- The filter bar (the standalone host's `narrowing/FilterBar.test.tsx` at a7fdf03; T044, T048) ----

mod filter_bar {
    use super::*;
    use adp_desl::hierarchy::definition::{FilterOperations, FilterParams, Filters, SortKey};
    use adp_desl::hierarchy::model::SurfaceFilter;

    /// The page's definition with a filter bar; made-up names throughout: the bar knows only what the definition
    /// tells it.
    fn filtered() -> SurfaceDefinition {
        SurfaceDefinition {
            labels: vec![
                label("marks", "~", true, &["Section", "Point"]),
                label("phase", "phase:", false, &["Point"]),
            ],
            filters: Some(Filters {
                operations: FilterOperations {
                    store: "keep".into(),
                    rename: "relabel".into(),
                    remove: "forget".into(),
                },
                params: FilterParams {
                    name: "called".into(),
                    query: "words".into(),
                },
                sort_keys: vec![SortKey {
                    key: "number".into(),
                    label: "By number".into(),
                }],
                entries: "points".into(),
            }),
            ..definition()
        }
    }

    fn point(id: &str, parent: &str, role: &str, title: &str, marks: &[&str], phase: &str) -> SurfaceNode {
        let mut node = node(id, role, "", parent, title);
        let phases: &[&str] = if phase.is_empty() {
            &[]
        } else {
            std::slice::from_ref(&phase)
        };
        node.labels = [("marks", marks), ("phase", phases)]
            .iter()
            .map(|(label, values)| LabelValues {
                label: (*label).into(),
                values: values.iter().map(|v| (*v).to_owned()).collect(),
            })
            .collect();
        node
    }

    /// Orders (1) holds 3, 2 and Refunds (4) with 5; Stock (6) holds 7. Two of four points carry fast; one filter is
    /// stored.
    fn model() -> SurfaceModel {
        let (l, e) = (roles::LEVEL, roles::ENTRY);
        let phase = |name: &str, colour: &str| SurfaceOption {
            id: format!("phase/{name}"),
            name: name.into(),
            colour: colour.into(),
            implicit: false,
        };
        SurfaceModel {
            title: "Points".into(),
            cause: causes::OPEN.into(),
            nodes: vec![
                point("1", "", l, "Orders", &[], ""),
                point("3", "1", e, "Take an order", &["fast"], "done"),
                point("2", "1", e, "Cancel an order", &[], "draft"),
                point("4", "1", l, "Refunds", &[], ""),
                point("5", "4", e, "Refund in full", &["fast"], ""),
                point("6", "", l, "Stock", &[], ""),
                point("7", "6", e, "Count the stock", &[], "draft"),
            ],
            option_lists: vec![OptionList {
                label: "phase".into(),
                options: vec![phase("draft", "gray"), phase("done", "green")],
            }],
            filters: vec![SurfaceFilter {
                id: "filter/Quick ones".into(),
                name: "Quick ones".into(),
                query: "~fast".into(),
            }],
            ..SurfaceModel::default()
        }
    }

    fn page(model: SurfaceModel) -> Surface {
        Surface::with(model, filtered(), 1280.0)
    }

    impl Surface {
        /// The titles of the boxes drawn, in the order they were.
        fn titles(&self) -> Vec<String> {
            self.boxes()
                .iter()
                .map(|b| b.split_once(' ').map_or(b.clone(), |(_, title)| title.to_owned()))
                .collect()
        }

        /// What the bar's count says.
        fn count(&self) -> String {
            let status = self
                .nodes
                .values()
                .find(|n| n.role() == Role::Status)
                .expect("the bar counts the entries");
            status.label().unwrap_or_default().to_owned()
        }

        /// Gives the query field the keyboard, the caret at the end of what it holds.
        fn focus_query(&mut self) {
            self.click(Role::SearchInput, "Filter");
            self.key(egui::Key::End, egui::Modifiers::NONE);
        }

        /// Types `text` at the end of the query.
        fn type_query(&mut self, text: &str) {
            self.focus_query();
            self.type_text(text);
            self.frame(Vec::new());
        }

        /// The words offered under the field, in their order, each as its word and its hint: the selectable rows of the
        /// list, one of them marked.
        fn offered(&self) -> Vec<String> {
            let mut offered: Vec<(f64, String)> = self
                .nodes
                .values()
                .filter(|n| n.role() == Role::Button && n.toggled().is_some())
                .map(|n| (n.bounds().unwrap().y0, n.label().unwrap_or_default().to_owned()))
                .collect();
            offered.sort_by(|a, b| a.partial_cmp(b).unwrap());
            offered.into_iter().map(|o| o.1).collect()
        }

        /// Picks `option` in the choice named `choice`.
        fn choose(&mut self, choice: &str, option: &str) {
            self.click(Role::ComboBox, choice);
            self.click(Role::ListBoxOption, option);
        }
    }

    #[test]
    fn the_bar_is_offered_only_by_a_definition_that_declares_filters() {
        let surface = Surface::of(model());
        assert!(surface.find(Role::SearchInput, "Filter").is_none());
    }

    #[test]
    fn the_bar_narrows_the_page_as_the_query_is_typed_keeping_the_levels_that_contain_what_stays_and_counts_the_entries()
     {
        let mut surface = page(model());
        let before = surface.count();

        surface.type_query("~FAST");

        // Narrowing writes nothing (FR-049).
        assert_eq!(before, "4 points");
        assert_eq!(
            surface.titles(),
            ["Orders", "Take an order", "Refunds", "Refund in full"]
        );
        assert_eq!(surface.count(), "2 of 4 points");
        assert!(surface.gestures().is_empty());
    }

    #[test]
    fn the_bar_selects_levels_by_a_title_pattern_with_everything_in_them() {
        let mut surface = page(model());

        surface.type_query("item:REF*");

        assert_eq!(surface.titles(), ["Orders", "Refunds", "Refund in full"]);
    }

    #[test]
    fn the_sort_menu_sorts_the_entries_within_each_level_writing_the_sort_into_the_query_and_nothing_to_the_file() {
        let mut surface = page(model());
        surface.click(Role::ComboBox, "Sort");
        let offered = surface.named(Role::ListBoxOption);
        surface.click(Role::ListBoxOption, "By number");
        let by_number = surface.titles();
        let sort_word = surface.tab.query.clone();

        surface.choose("Sort", "phase");

        // The levels keep their place, the entries within Orders sort; a point without a phase comes last.
        assert_eq!(offered, ["File order", "By number", "phase"]);
        assert_eq!(sort_word, "sort:number");
        assert_eq!(
            by_number,
            [
                "Orders",
                "Cancel an order",
                "Take an order",
                "Refunds",
                "Refund in full",
                "Stock",
                "Count the stock"
            ]
        );
        assert_eq!(surface.tab.query, "sort:phase");
        assert_eq!(surface.titles()[..3], ["Orders", "Cancel an order", "Take an order"]);
        assert!(surface.gestures().is_empty());
    }

    #[test]
    fn the_sort_menu_keeps_the_querys_other_words_and_takes_the_sort_out_for_the_files_order() {
        let mut surface = page(model());
        surface.type_query("~fast sort:phase item:ord*");

        surface.choose("Sort", "By number");
        let changed = surface.tab.query.clone();
        surface.choose("Sort", "File order");

        assert_eq!(changed, "~fast sort:number item:ord*");
        assert_eq!(surface.tab.query, "~fast item:ord*");
    }

    #[test]
    fn a_stored_filter_is_applied_by_picking_it_and_the_query_is_cleared() {
        let mut surface = page(model());

        surface.choose("Stored filters", "Quick ones");
        let applied = surface.titles();
        surface.click(Role::Button, "Clear the filter");

        assert_eq!(applied, ["Orders", "Take an order", "Refunds", "Refund in full"]);
        assert_eq!(surface.titles().len(), 7);
    }

    #[test]
    fn the_applied_query_is_stored_under_a_name_as_one_gesture_on_the_document() {
        let mut surface = page(model());
        surface.type_query(" phase:draft sort:number ");

        surface.click(Role::Button, "Store");
        surface.type_text(" Drafts ");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        assert_eq!(
            surface.sent(),
            [sent(
                "keep",
                "document",
                &[("called", "Drafts"), ("words", "phase:draft sort:number")]
            )]
        );
    }

    #[test]
    fn the_stored_filter_that_is_applied_is_renamed_and_removed_by_naming_it() {
        let mut surface = page(model());
        surface.type_query("~fast");
        let store_offered = surface.find(Role::Button, "Store").is_some();

        surface.click(Role::Button, "Rename Quick ones");
        let name = surface.field("New name of Quick ones");
        surface.key(egui::Key::A, egui::Modifiers::COMMAND);
        surface.type_text("Fast ones");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);
        surface.click(Role::Button, "Remove Quick ones");

        // A filter already stored is not offered for storing again.
        assert!(!store_offered);
        assert_eq!(
            name,
            Some(("Quick ones".into(), true)),
            "the field opens on the name, for typing"
        );
        assert_eq!(
            surface.sent(),
            [
                sent("relabel", "filter/Quick ones", &[("called", "Fast ones")]),
                sent("forget", "filter/Quick ones", &[]),
            ]
        );
    }

    #[test]
    fn a_rename_to_the_name_it_has_sends_nothing() {
        let mut surface = page(model());
        surface.type_query("~fast");

        surface.click(Role::Button, "Rename Quick ones");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        assert!(surface.field("New name of Quick ones").is_none(), "the field closes");
        assert!(surface.gestures().is_empty());
    }

    #[test]
    fn escape_drops_a_name_being_typed_and_sends_nothing() {
        let mut surface = page(model());
        surface.type_query("~fast sort:title");

        surface.click(Role::Button, "Store");
        surface.type_text("Later");
        surface.key(egui::Key::Escape, egui::Modifiers::NONE);

        assert!(surface.field("Name of the filter").is_none());
        assert!(surface.gestures().is_empty(), "{:?}", surface.sent());
        assert_eq!(surface.tab.query, "~fast sort:title", "the query stays");
    }

    #[test]
    fn a_document_that_cannot_change_offers_nothing_to_store_rename_or_remove_and_still_narrows() {
        let mut read_only = model();
        read_only.read_only_reason = "Read only.".into();
        let mut surface = page(read_only);

        surface.type_query("~fast");

        for name in ["Store", "Rename Quick ones", "Remove Quick ones"] {
            assert!(surface.find(Role::Button, name).is_none(), "{name}");
        }
        assert_eq!(surface.titles().len(), 4);
    }

    fn with_marks() -> SurfaceModel {
        let mut page = model();
        page.option_lists.push(OptionList {
            label: "marks".into(),
            options: vec![SurfaceOption {
                id: "marks/fast".into(),
                name: "fast".into(),
                colour: "default".into(),
                implicit: true,
            }],
        });
        page
    }

    #[test]
    fn completing_lists_the_members_of_a_labels_list_with_its_prefix_under_the_field_and_tab_takes_the_one_marked() {
        let mut surface = page(with_marks());
        surface.type_query("~f");
        let offered = surface.offered();

        surface.key(egui::Key::Tab, egui::Modifiers::NONE);

        assert_eq!(offered, ["~fast marks"]);
        assert_eq!(surface.tab.query, "~fast ");
        assert_eq!(
            surface.titles(),
            ["Orders", "Take an order", "Refunds", "Refund in full"]
        );
    }

    #[test]
    fn completing_keeps_a_prefix_open_for_its_name_then_offers_the_members_and_a_press_takes_one() {
        let mut surface = page(model());
        surface.type_query("~fast ph");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);
        let after_prefix = surface.tab.query.clone();
        let offered = surface.offered();

        let done = surface
            .nodes
            .values()
            .find(|n| n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("phase:done")))
            .and_then(|n| n.bounds())
            .expect("phase:done is offered");
        surface.click_at(egui::pos2(
            ((done.x0 + done.x1) / 2.0) as f32,
            ((done.y0 + done.y1) / 2.0) as f32,
        ));

        assert_eq!(after_prefix, "~fast phase:");
        assert_eq!(offered, ["phase:draft phase", "phase:done phase"]);
        assert_eq!(surface.tab.query, "~fast phase:done ");
        assert_eq!(surface.titles(), ["Orders", "Take an order"]);
    }

    #[test]
    fn the_field_takes_what_the_bar_leaves_beside_the_sort_the_stored_filters_and_the_count() {
        // Found by the walk of T054: the field kept a width of 260 where the standalone host's fills the bar.
        let surface = page(model());

        let field = surface.get(Role::SearchInput, "Filter").bounds().unwrap();
        let sort = surface.get(Role::ComboBox, "Sort").bounds().unwrap();
        let count = surface
            .nodes
            .values()
            .find(|n| n.role() == Role::Status && n.label().is_some_and(|l| l.ends_with("points")))
            .and_then(|n| n.bounds())
            .unwrap();
        assert!(field.x1 - field.x0 > 600.0, "{field:?}");
        assert!(sort.x0 > field.x1);
        assert!(
            count.x1 <= f64::from(surface.window) && count.y0 < field.y1,
            "the count stays on the field's line"
        );
    }

    #[test]
    fn the_field_gives_way_when_a_stored_filter_brings_its_controls_and_nothing_leaves_the_window() {
        // Found while fixing the one above: an overflowing line widened the bar it was measured against, so the field
        // kept its width and pushed Clear and the count out of the window.
        let mut surface = page(model());
        let wide = surface.get(Role::SearchInput, "Filter").bounds().unwrap();

        surface.choose("Stored filters", "Quick ones");

        let field = surface.get(Role::SearchInput, "Filter").bounds().unwrap();
        let clear = surface.get(Role::Button, "Clear the filter").bounds().unwrap();
        assert!(field.x1 - field.x0 < wide.x1 - wide.x0, "{field:?} {wide:?}");
        assert!(
            clear.x1 <= f64::from(surface.window) && clear.y0 < field.y1,
            "{clear:?}"
        );
    }

    #[test]
    fn the_store_control_draws_its_icon_in_the_icon_font_and_its_word_in_the_pages() {
        // Found by the walk of T059: the icon was written into the button's text, whose font has no such glyph.
        let mut surface = page(model());
        surface.type_query("phase:draft");

        let family = |text: &str| surface.runs.iter().find(|r| r.0.trim() == text).map(|r| r.1.clone());
        assert_eq!(family("Store"), Some(egui::FontFamily::Proportional));
        assert_eq!(
            family(&icons::text("mdi-bookmark-plus-outline")),
            Some(icons::font(1.0).family)
        );
        // A screen reader reads the button as its word, not its icon's glyph.
        assert_eq!(surface.glyphs_read(), Vec::<String>::new());
    }

    #[test]
    fn the_list_of_completions_stands_under_the_field() {
        // Found by the walk of T059: the list stood over the page's title, where the standalone host's stands under
        // the field.
        let mut surface = page(with_marks());
        surface.type_query("~f");

        let field = surface.get(Role::SearchInput, "Filter").bounds().unwrap();
        let row = surface
            .nodes
            .values()
            .find(|n| n.role() == Role::Button && n.toggled().is_some())
            .and_then(|n| n.bounds())
            .expect("a word is offered");
        assert!(row.y0 >= field.y1, "{row:?} {field:?}");
    }

    #[test]
    fn the_sort_menu_shows_its_choice_alone_its_icon_before_it_in_the_icon_font() {
        // Found by the walk of T054: the icon was written into the menu's text, whose font has no such glyph.
        let surface = page(model());

        assert!(surface.drawn("File order"));
        assert!(surface.drawn(&icons::text("mdi-sort")));
    }

    #[test]
    fn the_bar_is_a_search_landmark_holding_the_filter_the_sort_and_the_count() {
        let surface = page(model());

        assert!(surface.stands_in(Role::SearchInput, "Filter", Role::Search, "Narrow and sort"));
        assert!(surface.stands_in(Role::ComboBox, "Sort", Role::Search, "Narrow and sort"));
    }

    #[test]
    fn completing_offers_nothing_after_escape_nor_while_the_field_does_not_have_the_keyboard() {
        let mut surface = page(model());
        surface.type_query("ph");
        let typed = surface.offered();

        surface.key(egui::Key::Escape, egui::Modifiers::NONE);
        let after_escape = surface.offered();
        surface.type_text("a");
        let after_typing = surface.offered();
        surface.click_box("[7] Count the stock");

        assert_eq!(typed, ["phase: phase", "phase:draft phase", "phase:done phase"]);
        assert!(after_escape.is_empty());
        assert_eq!(after_typing, ["phase: phase", "phase:draft phase", "phase:done phase"]);
        assert!(surface.find(Role::SearchInput, "Filter").is_some());
        assert!(surface.offered().is_empty());
    }

    #[test]
    fn a_finding_the_filter_hides_stays_listed_and_selecting_it_shows_its_entry_by_taking_out_the_words_that_hide_it() {
        // Count the stock carries no fast.
        let mut findings = model();
        findings.findings = vec![SurfaceFinding {
            code: "std.duplicate".into(),
            severity: "warning".into(),
            message: "Count the stock twice.".into(),
            node_id: "7".into(),
            line: 9,
        }];
        let mut surface = page(findings);
        surface.type_query("~fast sort:number");
        let hidden = !surface.titles().contains(&"Count the stock".to_owned());

        surface.click(Role::Link, "Count the stock twice.");

        // The sort stays; the entry is shown and selected.
        assert!(hidden);
        assert_eq!(surface.tab.query, "sort:number");
        assert!(surface.titles().contains(&"Count the stock".to_owned()));
        assert_eq!(surface.tab.selected, "7");
    }
}

// ---- References (the standalone host's `references/References.test.tsx` at a7fdf03; T044, T046) ----

mod references {
    use super::*;
    use crate::hierarchy::references::{Part, parts};

    fn numbered(id: &str, parent: &str, role: &str, title: &str) -> SurfaceNode {
        let mut node = node(id, role, "", parent, title);
        node.number = format!("N-{id}");
        node
    }

    fn reference(start: usize, length: usize, target: &str) -> Reference {
        Reference {
            start,
            length,
            target_id: target.into(),
        }
    }

    /// Orders (1) holds Take an order (2) and Payment (3), which holds Refunds (4) with Refund in full (5); Payment
    /// and Refunds collapsed when `collapsed`. Shipping (6) holds Ship it (7), whose description refers to 5, and a
    /// point with no number of its own. Take an order refers to a number nothing has.
    fn model(collapsed: bool) -> SurfaceModel {
        let (l, e) = (roles::LEVEL, roles::ENTRY);
        let mut take = numbered("2", "1", e, "Take an order");
        take.description = "Was [N-99] once.".into();
        take.references = vec![reference(4, 6, "")];
        let mut ship = numbered("7", "6", e, "Ship it");
        ship.description = "After [N-5] at the latest.".into();
        ship.references = vec![reference(6, 5, "5")];
        let mut payment = numbered("3", "1", l, "Payment");
        payment.collapsed = collapsed;
        let mut refunds = numbered("4", "3", l, "Refunds");
        refunds.collapsed = collapsed;
        let mut unnumbered = numbered("line/40", "6", e, "Unnumbered");
        unnumbered.number = "[?]".into();
        SurfaceModel {
            title: "Shop".into(),
            cause: causes::OPEN.into(),
            nodes: vec![
                numbered("1", "", l, "Orders"),
                take,
                payment,
                refunds,
                numbered("5", "4", e, "Refund in full"),
                numbered("6", "", l, "Shipping"),
                ship,
                unnumbered,
            ],
            ..SurfaceModel::default()
        }
    }

    impl Surface {
        /// Opens the description of the box named `name` for typing, by a double click at its start, the caret then
        /// put at its end.
        fn open_description(&mut self, name: &str) {
            let at = self.get(Role::TreeItem, name).bounds().unwrap();
            self.double_click_at(egui::pos2(at.x0 as f32 + 16.0, at.y1 as f32 - 14.0));
            self.key(egui::Key::End, egui::Modifiers::COMMAND);
        }

        /// The rows of the lookup.
        fn lookup_rows(&self) -> Vec<String> {
            let mut rows: Vec<(f64, String)> = self
                .nodes
                .values()
                .filter(|n| n.role() == Role::ListBoxOption)
                .map(|n| (n.bounds().unwrap().y0, n.label().unwrap_or_default().to_owned()))
                .collect();
            rows.sort_by(|a, b| a.partial_cmp(b).unwrap());
            rows.into_iter().map(|r| r.1).collect()
        }

        /// Whether the widget of `role` named `name` has the keyboard.
        fn has_keyboard(&self, role: Role, name: &str) -> bool {
            let Some((id, _)) = self
                .nodes
                .iter()
                .find(|(_, n)| n.role() == role && n.label() == Some(name))
            else {
                return false;
            };
            self.ctx
                .memory(|m| m.focused())
                .is_some_and(|f| f.accesskit_id() == *id)
        }

        /// Where the caret stands in the open field on the node `id`, in characters.
        fn caret(&self, id: &str) -> Option<usize> {
            let field = self.tab.id().with(("field", id));
            egui::text_edit::TextEditState::load(&self.ctx, field)
                .and_then(|s| s.cursor.char_range())
                .map(|r| r.primary.index.0)
        }

        /// The widget of `role` named `name` asks for the keyboard, as a screen reader asks.
        fn focus(&mut self, role: Role, name: &str) {
            let (id, _) = self
                .nodes
                .iter()
                .find(|(_, n)| n.role() == role && n.label() == Some(name))
                .expect("it is drawn");
            let request = egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: *id,
                data: None,
            };
            self.frame(vec![egui::Event::AccessKitActionRequest(request)]);
            self.frame(Vec::new());
        }
    }

    #[test]
    fn a_texts_references_are_its_parts_by_their_utf16_offsets_and_one_that_does_not_fit_stays_text() {
        let text = "Na\u{1F600}h [N-5] and [N-6].";
        // The emoji is two code units, one character.
        // One runs past the text's end, and one starts inside another.
        let found = parts(
            text,
            &[
                reference(6, 5, "5"),
                reference(16, 5, ""),
                reference(15, 9, "x"),
                reference(8, 4, "y"),
            ],
        );

        assert_eq!(
            found,
            [
                Part::Text("Na\u{1F600}h ".into()),
                Part::Reference {
                    written: "[N-5]".into(),
                    target_id: "5".into()
                },
                Part::Text(" and ".into()),
                Part::Reference {
                    written: "[N-6]".into(),
                    target_id: String::new()
                },
                Part::Text(".".into()),
            ]
        );
    }

    #[test]
    fn a_lookup_opens_for_one_hash_typed_and_nothing_else_changed() {
        use crate::hierarchy::references::hash_typed;
        assert_eq!(hash_typed("ab", "a#b"), Some(1));
        assert_eq!(hash_typed("ab", "ab#"), Some(2));
        assert_eq!(hash_typed("ab", "#ba"), None, "the rest changed too");
        assert_eq!(hash_typed("ab", "a#xb"), None, "more than the hash");
        assert_eq!(hash_typed("ab", "axb"), None);
    }

    #[test]
    fn the_arrows_mark_another_row_of_the_lookup_and_enter_takes_it() {
        let mut surface = Surface::of(model(false));
        surface.open_description("N-7 Ship it");
        surface.type_text("#");
        surface.type_text("refund");

        surface.key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        surface.key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        surface.key(egui::Key::ArrowUp, egui::Modifiers::NONE);
        surface.key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        // Down past the last row stays on it.
        let text = "After [N-5] at the latest.[N-5]";
        assert_eq!(surface.field("Description of N-7"), Some((text.to_owned(), true)));
    }

    #[test]
    fn a_hash_opens_the_lookup_of_every_numbered_level_and_entry_narrowed_by_number_or_title() {
        let mut surface = Surface::of(model(false));
        surface.open_description("N-7 Ship it");

        surface.type_text("#");
        // The lookup opens in the frame after the `#`, and its area is drawn unseen in its first.
        surface.frame(Vec::new());

        // The lookup has the keyboard, and lists every node with a number, by number and title.
        assert!(surface.has_keyboard(Role::ComboBox, "Refer to"));
        assert_eq!(
            surface.lookup_rows(),
            [
                "N-1 Orders",
                "N-2 Take an order",
                "N-3 Payment",
                "N-4 Refunds",
                "N-5 Refund in full",
                "N-6 Shipping",
                "N-7 Ship it"
            ]
        );
        surface.type_text("refund");
        assert_eq!(surface.lookup_rows(), ["N-4 Refunds", "N-5 Refund in full"]);
        for _ in 0..6 {
            surface.key(egui::Key::Backspace, egui::Modifiers::NONE);
        }
        surface.type_text("n-2");
        assert_eq!(surface.lookup_rows(), ["N-2 Take an order"]);
    }

    #[test]
    fn the_pick_is_written_as_its_bracketed_number_in_place_of_the_hash_and_the_field_stays_open_until_it_is_committed()
    {
        let mut surface = Surface::of(model(false));
        surface.open_description("N-7 Ship it");
        surface.type_text(" ");
        surface.type_text("#");

        surface.type_text("refunds");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        // The lookup is gone, the text holds the reference, and nothing was sent yet.
        assert!(surface.find(Role::ComboBox, "Refer to").is_none());
        let text = "After [N-5] at the latest. [N-4]";
        assert_eq!(surface.field("Description of N-7"), Some((text.to_owned(), true)));
        assert_eq!(surface.caret("7"), Some(text.chars().count()));
        assert!(surface.gestures().is_empty());

        surface.key(egui::Key::Enter, egui::Modifiers::COMMAND);

        assert_eq!(surface.sent(), [sent("redescribe", "7", &[("description", text)])]);
    }

    #[test]
    fn escape_in_the_lookup_goes_back_to_the_text_with_the_hash_kept_and_nothing_committed() {
        let mut surface = Surface::of(model(false));
        surface.open_description("N-7 Ship it");
        surface.type_text("#");

        surface.key(egui::Key::Escape, egui::Modifiers::NONE);

        assert!(surface.find(Role::ComboBox, "Refer to").is_none());
        let text = "After [N-5] at the latest.#";
        assert_eq!(surface.field("Description of N-7"), Some((text.to_owned(), true)));
        assert_eq!(surface.caret("7"), Some(text.chars().count()));
        assert!(surface.gestures().is_empty());
    }

    #[test]
    fn leaving_the_lookup_for_the_page_commits_the_text_and_a_title_opens_no_lookup() {
        let mut surface = Surface::of(model(false));
        surface.open_description("N-7 Ship it");
        surface.type_text("#");
        assert!(
            surface.gestures().is_empty(),
            "going from the field to the lookup is no leaving"
        );

        surface.click_box("N-1 Orders");

        assert_eq!(
            surface.sent(),
            [sent(
                "redescribe",
                "7",
                &[("description", "After [N-5] at the latest.#")]
            )]
        );
        let title = surface.half_of("N-6 Shipping", false);
        surface.double_click_at(title);
        surface.key(egui::Key::End, egui::Modifiers::NONE);
        surface.type_text("#");
        surface.frame(Vec::new());
        surface.frame(Vec::new());
        assert!(surface.find(Role::ComboBox, "Refer to").is_none());
        assert!(
            surface
                .field("Title of N-6")
                .is_some_and(|(text, _)| text == "Shipping#")
        );
    }

    #[test]
    fn a_reference_stands_as_a_box_with_its_targets_number_and_current_title_and_one_that_names_nothing_as_written_marked()
     {
        let surface = Surface::of(model(false));

        let ship = surface.get(Role::TreeItem, "N-7 Ship it").bounds().unwrap();
        let reference = surface.get(Role::Button, "N-5 Refund in full").bounds().unwrap();
        assert!(
            ship.x0 <= reference.x0 && reference.x1 <= ship.x1 && ship.y0 <= reference.y0 && reference.y1 <= ship.y1
        );
        assert!(surface.drawn("After N-5 Refund in full at the latest."));
        assert!(
            surface
                .find(Role::Image, "[N-99] names nothing in this document")
                .is_some()
        );
        let broken = surface
            .texts
            .iter()
            .find(|t| t.starts_with("Was "))
            .expect("the description is drawn");
        assert!(broken.ends_with("[N-99] once."), "as written: {broken}");
    }

    #[test]
    fn selecting_a_reference_shows_its_target_by_the_focus_that_keeps_it_in_sight_expanding_its_collapsed_levels() {
        // The page focused on Shipping, so Orders and all in it are out of sight.
        let mut surface = Surface::of(model(true));
        let shipping = surface.centre(Role::TreeItem, "N-6 Shipping");
        surface.frame(vec![egui::Event::PointerMoved(shipping)]);
        surface.click(Role::Button, "Focus on N-6 Shipping");
        assert!(!surface.drawn("Orders"));

        surface.click(Role::Button, "N-5 Refund in full");

        // The whole page again, and only the collapse of the two levels that hide the target is sent.
        assert_eq!(surface.tab.trail_current(), Some(""));
        assert_eq!(surface.sent(), [sent("fold", "3", &[]), sent("fold", "4", &[])]);
        assert_eq!(surface.tab.selected, "5");
    }

    #[test]
    fn a_reference_shows_its_target_from_the_keyboard_too_by_enter_on_its_box() {
        let mut surface = Surface::of(model(false));

        surface.focus(Role::Button, "N-5 Refund in full");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        assert_eq!(surface.tab.selected, "5");
    }

    #[test]
    fn a_double_click_on_a_reference_does_not_open_the_description_for_typing() {
        let mut surface = Surface::of(model(false));

        let at = surface.centre(Role::Button, "N-5 Refund in full");
        surface.double_click_at(at);

        assert!(surface.tab.field.is_none());
    }
}

/// A label's field and picker, and its list edited (the standalone host's `labels/TagField.test.tsx` and
/// `classification/ClassificationPicker.test.tsx` at a7fdf03; etalii.adp spec 036, T047).
mod labels {
    use super::*;
    use adp_desl::hierarchy::definition::{
        LabelEdit, LabelEditOperations, LabelEditWords, LabelPick, LabelPickOperations, ListDeletion, ListParams,
        PickParams, PickWords,
    };

    /// Made-up names throughout: the field, the picker and their lists know only what the definition tells them.
    fn labelled() -> SurfaceDefinition {
        let mut definition = definition();
        let mut marks = label("marks", "~", true, &["Section", "Point"]);
        marks.edit = Some(LabelEdit {
            operations: LabelEditOperations {
                add: "mark".into(),
                remove: "unmark".into(),
                rename: "rebrand".into(),
                recolour: "tint".into(),
                reorder: "shuffle".into(),
                delete: "erase".into(),
            },
            params: ListParams {
                name: "word".into(),
                colour: "hue".into(),
                after: "behind".into(),
            },
            words: LabelEditWords {
                list: "Marks".into(),
                add: "Add mark".into(),
                remove: "Unmark".into(),
            },
            deletion: ListDeletion {
                threshold: 1,
                message: "{count} things carry ~{name}. Erase it?".into(),
            },
        });
        let mut phase = label("phase", "phase:", false, &["Point"]);
        phase.pick = Some(LabelPick {
            operations: LabelPickOperations {
                set: "stage".into(),
                clear: "unstage".into(),
                add: "coin".into(),
                rename: "rebrand".into(),
                recolour: "tint".into(),
                reorder: "shuffle".into(),
                delete: "erase".into(),
            },
            params: PickParams {
                value: "phase".into(),
                list: ListParams {
                    name: "word".into(),
                    colour: "hue".into(),
                    after: "behind".into(),
                },
            },
            words: PickWords {
                list: "Phases".into(),
                value: "Phase".into(),
                clear: "No phase".into(),
            },
            deletion: ListDeletion {
                threshold: 1,
                message: "{count} points are {name}. Erase it?".into(),
            },
        });
        definition.labels = vec![marks, phase];
        definition
    }

    fn option(id: &str, name: &str, colour: &str) -> SurfaceOption {
        SurfaceOption {
            id: id.into(),
            name: name.into(),
            colour: colour.into(),
            implicit: false,
        }
    }

    fn carrying(mut node: SurfaceNode, marks: &[&str], phase: &[&str]) -> SurfaceNode {
        node.labels = vec![
            LabelValues {
                label: "marks".into(),
                values: marks.iter().map(|m| (*m).to_owned()).collect(),
            },
            LabelValues {
                label: "phase".into(),
                values: phase.iter().map(|p| (*p).to_owned()).collect(),
            },
        ];
        node
    }

    /// The section 1 holds the points 2, 3 and 4; 1 carries fast, 2 carries fast and cheap, spelt as the list does
    /// not, and is DONE, spelt as the list does not either; 3 carries nothing, and 4 is done.
    fn model(read_only: bool) -> SurfaceModel {
        SurfaceModel {
            title: "Marks".into(),
            cause: causes::OPEN.into(),
            read_only_reason: if read_only { "Read only.".into() } else { String::new() },
            nodes: vec![
                carrying(level("1", "", "Orders"), &["fast"], &[]),
                carrying(entry("2", "1", "Take an order"), &["fast", "cheap"], &["DONE"]),
                carrying(entry("3", "1", "Cancel an order"), &[], &[]),
                carrying(entry("4", "1", "Refund an order"), &[], &["done"]),
            ],
            option_lists: vec![
                OptionList {
                    label: "marks".into(),
                    options: vec![
                        option("mark/fast", "fast", "green"),
                        option("mark/cheap", "Cheap", "blue"),
                        option("mark/slow", "slow", "default"),
                    ],
                },
                OptionList {
                    label: "phase".into(),
                    options: vec![
                        option("phase/draft", "draft", "gray"),
                        option("phase/done", "done", "green"),
                        option("phase/gone", "gone", "red"),
                    ],
                },
            ],
            ..SurfaceModel::default()
        }
    }

    fn surface(read_only: bool) -> Surface {
        Surface::with(model(read_only), labelled(), 1100.0)
    }

    impl Surface {
        /// Whether the widget of `role` named `name` has the keyboard.
        fn keyboard_in(&self, role: Role, name: &str) -> bool {
            self.nodes
                .iter()
                .find(|(_, n)| n.role() == role && n.label() == Some(name))
                .is_some_and(|(id, _)| {
                    self.ctx
                        .memory(|m| m.focused())
                        .is_some_and(|f| f.accesskit_id() == *id)
                })
        }

        /// The widget of `role` named `name` asks for the keyboard, as a screen reader asks.
        fn ask_focus(&mut self, role: Role, name: &str) {
            let (id, _) = self
                .nodes
                .iter()
                .find(|(_, n)| n.role() == role && n.label() == Some(name))
                .expect("it is drawn");
            let request = egui::accesskit::ActionRequest {
                action: egui::accesskit::Action::Focus,
                target_tree: egui::accesskit::TreeId::ROOT,
                target_node: *id,
                data: None,
            };
            self.frame(vec![egui::Event::AccessKitActionRequest(request)]);
            self.frame(Vec::new());
        }

        /// The rows of the open list, in their order, each with whether it is chosen.
        fn rows(&self) -> Vec<(String, bool)> {
            let mut rows: Vec<(f64, String, bool)> = self
                .nodes
                .values()
                .filter(|n| n.role() == Role::ListBoxOption)
                .map(|n| {
                    (
                        n.bounds().map_or(0.0, |b| b.y0),
                        n.label().unwrap_or_default().to_owned(),
                        n.is_selected() == Some(true),
                    )
                })
                .collect();
            rows.sort_by(|a, b| a.partial_cmp(b).unwrap());
            rows.into_iter().map(|(_, name, chosen)| (name, chosen)).collect()
        }

        /// Opens the field's list of the box numbered `number` from the control among its icons.
        fn open_list(&mut self, number: &str) {
            let name = self
                .nodes
                .values()
                .filter(|n| n.role() == Role::TreeItem)
                .filter_map(|n| n.label())
                .find(|l| l.starts_with(&format!("{number} ")))
                .expect("the box is drawn")
                .to_owned();
            // The control shows under the pointer.
            let over = self.half_of(&name, false);
            self.frame(vec![egui::Event::PointerMoved(over)]);
            self.click(Role::Button, &format!("Add mark to {number}"));
            self.frame(Vec::new());
            self.get(Role::Dialog, &format!("Marks of {number}"));
        }

        /// Opens the picker of the box named `name` from its value, selecting the box first.
        fn open_picker(&mut self, name: &str, number: &str, value: &str) {
            self.click_box(name);
            self.click(Role::Button, &format!("Phase of {number}: {value}"));
            self.frame(Vec::new());
            self.get(Role::Dialog, &format!("Phase of {number}"));
        }

        /// Opens the editor of the list from the open field's list or picker.
        fn open_editor(&mut self, list: &str, options: &str) {
            self.click(Role::Button, &format!("Edit {list}"));
            self.frame(Vec::new());
            self.get(Role::Dialog, &format!("Options of {options}"));
        }

        /// Types `text` in the field named `name` in place of what it holds, and Enter.
        fn retype(&mut self, name: &str, text: &str) {
            self.click(Role::TextInput, name);
            self.key(egui::Key::A, egui::Modifiers::COMMAND);
            self.type_text(text);
            self.key(egui::Key::Enter, egui::Modifiers::NONE);
        }

        /// Presses the page away from every box and list.
        fn press_outside(&mut self) {
            self.click_at(egui::pos2(self.window - 20.0, 780.0));
        }

        fn top_of(&self, role: Role, name: &str) -> (f64, f64) {
            let at = self.get(role, name).bounds().expect("it has bounds");
            (at.x0, at.y0)
        }
    }

    #[test]
    fn the_field_shows_the_values_a_node_carries_first_below_the_title_and_the_picker_after_it() {
        let surface = surface(false);

        let field = surface.get(Role::Button, "Marks of [2]: fast, cheap").bounds().unwrap();
        let picker = surface.get(Role::Button, "Phase of [2]: DONE").bounds().unwrap();
        let shown: Vec<&str> = surface
            .texts
            .iter()
            .map(String::as_str)
            .filter(|t| ["#cheap", "DONE"].contains(t))
            .collect();

        // The carried spelling, the field first and the picker after it in the same row.
        assert_eq!(shown, ["#cheap", "DONE"]);
        assert!(field.x1 < picker.x0 && (field.y0 - picker.y0).abs() < 1.0);
        let title = surface.get(Role::TreeItem, "[2] Take an order").bounds().unwrap();
        assert!(field.y0 > title.y0 + LINE_HEIGHT as f64 - 1.0, "below the title");
    }

    #[test]
    fn the_way_to_add_a_value_is_among_the_boxs_controls_and_nothing_changes_a_document_that_cannot_be_changed() {
        let mut surface = surface(false);
        let at = surface.half_of("[3] Cancel an order", false);
        surface.frame(vec![egui::Event::PointerMoved(at)]);
        let control = surface.get(Role::Button, "Add mark to [3]").bounds().unwrap();
        let notes = surface.get(Role::Button, "New note on [3]").bounds().unwrap();
        assert!(
            (control.y0 - notes.y0).abs() < 1.0 || control.x0 == notes.x0,
            "beside the box's other icons"
        );

        let mut surface = surface_read_only();
        surface.frame(vec![egui::Event::PointerMoved(at)]);
        surface.click_box("[3] Cancel an order");
        assert!(surface.find(Role::Button, "Add mark to [3]").is_none());
        assert!(surface.find(Role::Button, "Marks of [2]: fast, cheap").is_none());
        assert!(surface.drawn("#fast"));
    }

    fn surface_read_only() -> Surface {
        surface(true)
    }

    #[test]
    fn the_list_has_the_keyboard_searches_and_picks_a_member_with_the_lists_spelling_and_opens_from_the_values_too() {
        let mut surface = surface(false);

        surface.open_list("[3]");
        let selected = surface.tab.selected.clone();
        let focused = surface.keyboard_in(Role::ComboBox, "Add mark to [3]");
        surface.type_text("CHE");
        let offered = surface.rows();
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);
        surface.ask_focus(Role::Button, "Marks of [2]: fast, cheap");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);
        surface.frame(Vec::new());

        assert_eq!(
            selected, "3",
            "opening the list selects its box, as any press on it does"
        );
        assert!(focused);
        assert_eq!(
            offered,
            [("Cheap".to_owned(), false), ("Create “CHE”".to_owned(), false)]
        );
        assert_eq!(surface.sent(), [sent("mark", "3", &[("word", "Cheap")])]);
        surface.get(Role::Dialog, "Marks of [2]");
        assert!(surface.keyboard_in(Role::ComboBox, "Add mark to [2]"));
    }

    #[test]
    fn a_value_is_created_from_text_that_names_no_member_as_typed() {
        let mut surface = surface(false);

        surface.open_list("[3]");
        surface.type_text("SLOW");
        let exact = surface.rows();
        surface.key(egui::Key::A, egui::Modifiers::COMMAND);
        surface.type_text("brand new");
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        // Text that names a member in another case is that member, and nothing to create.
        assert_eq!(exact, [("slow".to_owned(), false)]);
        assert_eq!(surface.sent(), [sent("mark", "3", &[("word", "brand new")])]);
    }

    #[test]
    fn a_value_is_taken_off_by_its_cross_and_by_picking_it_again_with_the_spelling_the_node_carries() {
        let mut surface = surface(false);

        surface.open_list("[2]");
        let chosen: Vec<String> = surface.rows().into_iter().filter(|r| r.1).map(|r| r.0).collect();
        surface.click(Role::Button, "Unmark fast");
        surface.click(Role::ListBoxOption, "Cheap");

        assert_eq!(chosen, ["fast", "Cheap"]);
        assert!(
            surface.keyboard_in(Role::ComboBox, "Add mark to [2]"),
            "the search keeps the keyboard"
        );
        assert_eq!(
            surface.sent(),
            [
                sent("unmark", "2", &[("word", "fast")]),
                sent("unmark", "2", &[("word", "cheap")])
            ]
        );
    }

    #[test]
    fn the_list_keeps_its_keys_from_the_page_and_closes_on_escape_and_on_a_press_outside() {
        let mut surface = surface(false);

        // Delete in the list is not the page's delete of the selected box; Escape closes; so does a press outside.
        surface.open_list("[2]");
        surface.ask_focus(Role::Button, "Unmark fast");
        surface.key(egui::Key::Delete, egui::Modifiers::NONE);
        // Nothing in the list has the keyboard once its empty part is pressed; the keys are still its own.
        let corner = surface.get(Role::Dialog, "Marks of [2]").bounds().unwrap();
        surface.click_at(egui::pos2(corner.x0 as f32 + 3.0, corner.y0 as f32 + 3.0));
        surface.key(egui::Key::Delete, egui::Modifiers::NONE);
        surface.key(egui::Key::Escape, egui::Modifiers::NONE);
        let after_escape = surface.find(Role::Dialog, "Marks of [2]").is_some();
        surface.open_list("[2]");
        surface.press_outside();

        assert!(surface.gestures().is_empty());
        assert!(!after_escape);
        assert!(surface.find(Role::Dialog, "Marks of [2]").is_none());
    }

    #[test]
    fn the_lists_editor_opens_where_the_list_was_its_names_in_one_column() {
        let mut surface = surface(false);

        surface.open_list("[2]");
        let list = surface.top_of(Role::Dialog, "Marks of [2]");
        surface.open_editor("marks", "Marks");

        let editor = surface.top_of(Role::Dialog, "Options of Marks");
        assert!((list.0 - editor.0).abs() < 1.0 && (list.1 - editor.1).abs() < 1.0);
        let names: Vec<f64> = ["fast", "Cheap", "slow"]
            .iter()
            .map(|o| surface.top_of(Role::TextInput, &format!("Name of {o}")).0)
            .collect();
        assert!(
            names.iter().all(|x| (x - names[0]).abs() < 0.5),
            "every tag as wide as the widest: {names:?}"
        );
        let handle = surface.get(Role::Image, "Move option fast").bounds().unwrap().x1;
        let widest = ["fast", "Cheap", "slow"]
            .iter()
            .map(|o| {
                surface.ctx.fonts_mut(|f| {
                    f.layout_no_wrap((*o).to_owned(), egui::FontId::proportional(12.0), egui::Color32::WHITE)
                        .size()
                        .x
                })
            })
            .fold(0.0, f32::max);
        assert!(
            names[0] - handle >= f64::from(widest + 14.0) - 0.5,
            "the widest tag fits its column"
        );
        assert!(surface.find(Role::Dialog, "Marks of [2]").is_none());
    }

    #[test]
    fn a_member_is_renamed_and_recoloured_each_one_gesture_naming_the_member_and_none_is_added_there() {
        let mut surface = surface(false);

        surface.open_list("[2]");
        surface.open_editor("marks", "Marks");
        surface.retype("Name of fast", "quick");
        surface.click(Role::ComboBox, "Colour of Cheap");
        surface.click(Role::Button, "Blue");
        surface.click(Role::ComboBox, "Colour of Cheap");
        surface.click(Role::Button, "Red");

        assert_eq!(
            surface.sent(),
            [
                sent("rebrand", "mark/fast", &[("word", "quick")]),
                sent("tint", "mark/cheap", &[("hue", "red")]),
            ]
        );
        assert!(surface.find(Role::TextInput, "New option").is_none());
    }

    #[test]
    fn a_member_is_put_in_its_place_as_the_member_it_goes_after_or_first_with_none() {
        let mut surface = surface(false);

        surface.open_list("[2]");
        surface.open_editor("marks", "Marks");
        surface.click(Role::Button, "Move fast down");
        surface.click(Role::Button, "Move slow up");
        surface.click(Role::Button, "Move Cheap up");

        assert_eq!(
            surface.sent(),
            [
                sent("shuffle", "mark/fast", &[("behind", "mark/cheap")]),
                sent("shuffle", "mark/slow", &[("behind", "mark/fast")]),
                sent("shuffle", "mark/cheap", &[("behind", "")]),
            ]
        );
    }

    #[test]
    fn a_member_is_dragged_into_its_place_the_list_standing_as_it_would_once_it_is_dropped() {
        let mut surface = surface(false);
        surface.open_list("[2]");
        surface.open_editor("marks", "Marks");
        let from = surface.centre(Role::Image, "Move option fast");
        let step =
            surface.top_of(Role::TextInput, "Name of Cheap").1 - surface.top_of(Role::TextInput, "Name of fast").1;

        let press = |pressed, at| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let to = from + egui::vec2(0.0, 2.0 * step as f32);
        surface.frame(vec![egui::Event::PointerMoved(from)]);
        surface.frame(vec![press(true, from)]);
        surface.frame(vec![egui::Event::PointerMoved(from + egui::vec2(0.0, 6.0))]);
        surface.frame(vec![egui::Event::PointerMoved(to)]);
        surface.frame(Vec::new());
        let mut order: Vec<(f64, &str)> = ["fast", "Cheap", "slow"]
            .iter()
            .map(|o| (surface.top_of(Role::TextInput, &format!("Name of {o}")).1, *o))
            .collect();
        order.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let sent_while_dragged = surface.gestures().len();
        surface.frame(vec![press(false, to)]);
        surface.frame(Vec::new());

        assert_eq!(order.iter().map(|o| o.1).collect::<Vec<_>>(), ["Cheap", "slow", "fast"]);
        assert_eq!(sent_while_dragged, 0);
        assert_eq!(
            surface.sent(),
            [sent("shuffle", "mark/fast", &[("behind", "mark/slow")])]
        );
    }

    #[test]
    fn a_member_nothing_carries_is_deleted_at_once_and_one_carried_asks_first_in_the_definitions_words() {
        let mut surface = surface(false);

        surface.open_list("[2]");
        surface.open_editor("marks", "Marks");
        surface.click(Role::Button, "Delete slow");
        let sent_at_once = surface.gestures().len();
        surface.click(Role::Button, "Delete fast");
        surface.get(Role::AlertDialog, "Delete fast?");
        let asked = surface.drawn("2 things carry ~fast. Erase it?");
        let before = surface.gestures().len();
        surface.click(Role::Button, "Delete");

        // Two boxes carry fast.
        assert_eq!(sent_at_once, 1);
        assert!(asked);
        assert_eq!(before, 1);
        assert_eq!(
            surface.sent(),
            [sent("erase", "mark/slow", &[]), sent("erase", "mark/fast", &[])]
        );
    }

    #[test]
    fn a_label_of_one_value_shows_it_after_the_others_and_the_way_to_give_one_only_on_the_selected_box() {
        let mut surface = surface(false);
        let unselected = surface.find(Role::Button, "Phase of [3]: none").is_some();

        surface.click_box("[3] Cancel an order");

        assert!(!unselected);
        surface.get(Role::Button, "Phase of [3]: none");
        assert!(surface.drawn("Phase"));
        assert!(surface.drawn("DONE"));
    }

    #[test]
    fn the_way_to_give_a_value_keeps_its_place_on_a_box_that_is_not_selected_so_selecting_it_keeps_its_height() {
        // Found by the walk of T059: a box with no labels grew by the row of its pickers when it was selected.
        let mut surface = surface(false);
        let unselected = surface.height_of("[3] Cancel an order");

        surface.click_box("[3] Cancel an order");

        assert_eq!(surface.height_of("[3] Cancel an order"), unselected);
    }

    #[test]
    fn a_document_that_cannot_change_offers_no_picker_and_still_shows_the_value() {
        let mut surface = surface(true);
        surface.click_box("[3] Cancel an order");

        assert!(surface.named(Role::Button).iter().all(|n| !n.starts_with("Phase of")));
        assert!(surface.drawn("DONE"));
    }

    #[test]
    fn a_member_is_picked_from_the_searched_list_as_one_gesture_naming_the_node_and_nothing_for_the_value_it_has() {
        let mut surface = surface(false);

        surface.open_picker("[2] Take an order", "[2]", "DONE");
        let focused = surface.keyboard_in(Role::ComboBox, "Phase of [2]");
        let chosen: Vec<String> = surface.rows().into_iter().filter(|r| r.1).map(|r| r.0).collect();
        surface.click(Role::ListBoxOption, "done");
        let closed = surface.find(Role::Dialog, "Phase of [2]").is_none();
        surface.open_picker("[2] Take an order", "[2]", "DONE");
        surface.type_text("dr");
        let offered = surface.rows();
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        // No offer to create a member here, which the list's editor does.
        assert!(focused);
        assert_eq!(chosen, ["done"]);
        assert!(closed);
        assert_eq!(offered, [("draft".to_owned(), false)]);
        assert_eq!(surface.sent(), [sent("stage", "2", &[("phase", "draft")])]);
    }

    #[test]
    fn the_arrows_mark_another_member_and_enter_picks_it() {
        let mut surface = surface(false);

        surface.open_picker("[3] Cancel an order", "[3]", "none");
        surface.key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        surface.key(egui::Key::ArrowDown, egui::Modifiers::NONE);
        surface.key(egui::Key::ArrowUp, egui::Modifiers::NONE);
        surface.key(egui::Key::Enter, egui::Modifiers::NONE);

        assert_eq!(surface.sent(), [sent("stage", "3", &[("phase", "done")])]);
    }

    #[test]
    fn an_open_list_closes_when_the_document_can_no_longer_be_changed() {
        let mut surface = surface(false);

        surface.open_list("[2]");
        surface.outside(model(true), causes::OUTSIDE);

        assert!(surface.find(Role::Dialog, "Marks of [2]").is_none());
    }

    #[test]
    fn the_value_is_cleared_and_no_clear_is_offered_without_one() {
        let mut surface = surface(false);

        surface.open_picker("[2] Take an order", "[2]", "DONE");
        surface.click(Role::Button, "No phase");
        surface.open_picker("[3] Cancel an order", "[3]", "none");

        assert_eq!(surface.sent(), [sent("unstage", "2", &[])]);
        assert!(surface.find(Role::Button, "No phase").is_none());
    }

    #[test]
    fn the_picker_keeps_its_keys_from_the_page_and_closes_on_a_press_outside_and_on_its_value() {
        let mut surface = surface(false);

        surface.open_picker("[2] Take an order", "[2]", "DONE");
        surface.ask_focus(Role::Button, "No phase");
        surface.key(egui::Key::Delete, egui::Modifiers::NONE);
        surface.press_outside();
        let outside = surface.find(Role::Dialog, "Phase of [2]").is_none();
        surface.open_picker("[2] Take an order", "[2]", "DONE");
        surface.click(Role::Button, "Phase of [2]: DONE");

        assert!(surface.gestures().is_empty());
        assert!(outside);
        assert!(surface.find(Role::Dialog, "Phase of [2]").is_none());
    }

    #[test]
    fn a_member_is_added_on_the_document_and_renamed_recoloured_and_moved_each_one_gesture_naming_it() {
        let mut surface = surface(false);

        surface.open_picker("[2] Take an order", "[2]", "DONE");
        surface.open_editor("phases", "Phases");
        surface.retype("New option", "review");
        surface.retype("Name of draft", "sketch");
        surface.click(Role::ComboBox, "Colour of done");
        surface.click(Role::Button, "Blue");
        surface.click(Role::Button, "Move gone up");

        assert_eq!(
            surface.sent(),
            [
                sent("coin", "document", &[("word", "review")]),
                sent("rebrand", "phase/draft", &[("word", "sketch")]),
                sent("tint", "phase/done", &[("hue", "blue")]),
                sent("shuffle", "phase/gone", &[("behind", "phase/draft")]),
            ]
        );
    }

    #[test]
    fn a_member_carried_in_any_case_asks_first_before_it_is_deleted() {
        let mut surface = surface(false);

        surface.open_picker("[2] Take an order", "[2]", "DONE");
        surface.open_editor("phases", "Phases");
        surface.click(Role::Button, "Delete gone");
        surface.click(Role::Button, "Delete done");
        surface.get(Role::AlertDialog, "Delete done?");
        let asked = surface.drawn("2 points are done. Erase it?");
        let before = surface.gestures().len();
        surface.click(Role::Button, "Delete");

        // DONE and done are two points.
        assert!(asked);
        assert_eq!(before, 1);
        assert_eq!(
            surface.sent(),
            [sent("erase", "phase/gone", &[]), sent("erase", "phase/done", &[])]
        );
    }
}

/// The widths of the columns (the standalone host's `ColumnEdge.test.tsx` at a7fdf03; etalii.adp spec 036, T050 and
/// T052): each column's right edge dragged, stepped and reset, one gesture each, and the levels that fit counted with
/// the widths.
mod columns {
    use super::*;
    use adp_desl::hierarchy::definition::Widths;
    use adp_desl::hierarchy::model::ColumnWidth;

    /// The standalone test's definition, keeping widths from 150 to 900.
    fn resizing() -> SurfaceDefinition {
        SurfaceDefinition {
            widths: Some(Widths {
                add: "widen".into(),
                set: "rewiden".into(),
                reset: "unwiden".into(),
                level_param: "depth".into(),
                width_param: "pixels".into(),
                min: 150,
                max: 900,
            }),
            ..definition()
        }
    }

    fn width(id: &str, level: u32, width: u32) -> ColumnWidth {
        ColumnWidth {
            id: id.into(),
            level,
            width,
        }
    }

    /// Orders (1) holds Payment (2), which holds Cards (3); Stock (4) stands beside Orders. The second column is kept
    /// at 400, or the columns at `widths`.
    fn page(widths: Vec<ColumnWidth>) -> SurfaceModel {
        SurfaceModel {
            title: "Plan".into(),
            cause: causes::OPEN.into(),
            nodes: vec![
                level("1", "", "Orders"),
                level("2", "1", "Payment"),
                level("3", "2", "Cards"),
                level("4", "", "Stock"),
            ],
            widths,
            ..SurfaceModel::default()
        }
    }

    fn kept() -> Vec<ColumnWidth> {
        vec![width("width/2", 2, 400)]
    }

    impl Surface {
        /// The width of the box named `name`.
        fn width_of(&self, name: &str) -> f64 {
            let at = self.get(Role::TreeItem, name).bounds().unwrap();
            at.x1 - at.x0
        }

        /// A point on the right edge of the box named `name`, near its top.
        fn edge_of(&self, name: &str) -> egui::Pos2 {
            let at = self.get(Role::TreeItem, name).bounds().unwrap();
            egui::pos2(at.x1 as f32, at.y0 as f32 + 10.0)
        }

        /// Presses the right edge of the box named `name` and moves `by` to the right, letting go when `release`.
        fn drag_edge(&mut self, name: &str, by: f32, release: bool) {
            let from = self.edge_of(name);
            let to = from + egui::vec2(by, 0.0);
            self.frame(vec![egui::Event::PointerMoved(from)]);
            self.frame(vec![egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }]);
            self.frame(vec![egui::Event::PointerMoved(from + egui::vec2(by / 2.0, 0.0))]);
            self.frame(vec![egui::Event::PointerMoved(to)]);
            self.frame(Vec::new());
            if release {
                self.release();
            }
        }

        /// The edges drawn, each as its name and its value, with the range it is held to.
        fn edges(&self) -> Vec<(String, f64, f64, f64)> {
            let mut edges: Vec<(String, f64, f64, f64)> = self
                .nodes
                .values()
                .filter(|n| n.role() == Role::Splitter)
                .map(|n| {
                    (
                        n.label().unwrap_or_default().to_owned(),
                        n.numeric_value().unwrap_or_default(),
                        n.min_numeric_value().unwrap_or_default(),
                        n.max_numeric_value().unwrap_or_default(),
                    )
                })
                .collect();
            edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
            edges
        }
    }

    #[test]
    fn each_levels_box_is_drawn_at_its_columns_kept_width_and_the_pages_own_where_none_is_kept() {
        // Act.
        let surface = Surface::with(page(kept()), resizing(), 1280.0);

        // Assert.
        let widths: Vec<f64> = ["[1] Orders", "[4] Stock", "[2] Payment", "[3] Cards"]
            .iter()
            .map(|name| surface.width_of(name))
            .collect();
        assert_eq!(widths, [240.0, 240.0, 400.0, 240.0]);
    }

    #[test]
    fn every_box_of_a_column_follows_a_drag_of_its_right_edge_and_the_width_is_kept_when_let_go_in_one_gesture() {
        // Arrange.
        let mut surface = Surface::with(page(kept()), resizing(), 1280.0);
        let mut answered = kept();
        answered.push(width("width/1", 1, 300));
        surface.script.borrow_mut().next.push(page(answered));

        // Act.
        surface.drag_edge("[1] Orders", 60.0, false);
        let during = [surface.width_of("[1] Orders"), surface.width_of("[4] Stock")];
        let sent_during = surface.sent().len();
        surface.release();

        // Assert: a first width names the column's depth.
        assert_eq!(during, [300.0, 300.0]);
        assert_eq!(sent_during, 0);
        assert_eq!(
            surface.sent(),
            [sent("widen", "document", &[("depth", "1"), ("pixels", "300")])]
        );
        assert_eq!(surface.width_of("[1] Orders"), 300.0);
        assert!(surface.tab.selected.is_empty(), "pressing an edge selects no box");
    }

    #[test]
    fn a_kept_width_is_changed_by_its_columns_id_a_width_is_held_to_the_range_and_a_press_that_moved_nothing_sends_nothing()
     {
        // Arrange.
        let mut surface = Surface::with(page(kept()), resizing(), 1280.0);

        // Act.
        surface.drag_edge("[2] Payment", 600.0, true);
        surface.click_at(surface.edge_of("[1] Orders"));
        surface.drag_edge("[1] Orders", -500.0, true);

        // Assert.
        assert_eq!(
            surface.sent(),
            [
                sent("rewiden", "width/2", &[("pixels", "900")]),
                sent("widen", "document", &[("depth", "1"), ("pixels", "150")]),
            ]
        );
    }

    #[test]
    fn a_double_click_gives_a_kept_column_the_pages_own_width_again_and_the_keys_step_or_reset_a_focused_edge() {
        // Arrange.
        let mut surface = Surface::with(page(kept()), resizing(), 1280.0);

        // Act: the first column keeps no width, so a double click on it sends nothing.
        surface.double_click_at(surface.edge_of("[1] Orders"));
        surface.double_click_at(surface.edge_of("[2] Payment"));
        surface.click_at(surface.edge_of("[1] Orders"));
        surface.key(egui::Key::ArrowRight, egui::Modifiers::NONE);
        surface.key(egui::Key::ArrowLeft, egui::Modifiers::NONE);
        surface.click_at(surface.edge_of("[2] Payment"));
        surface.key(egui::Key::Home, egui::Modifiers::NONE);

        // Assert.
        assert_eq!(
            surface.sent(),
            [
                sent("unwiden", "width/2", &[]),
                sent("widen", "document", &[("depth", "1"), ("pixels", "256")]),
                sent("widen", "document", &[("depth", "1"), ("pixels", "224")]),
                sent("unwiden", "width/2", &[]),
            ]
        );
        assert!(
            surface.tab.selected.is_empty(),
            "the page's keys are the edge's while it has the keyboard"
        );
    }

    #[test]
    fn a_column_goes_back_when_its_gesture_is_refused_and_shows_the_width_the_next_model_brings() {
        // Arrange.
        let mut surface = Surface::with(page(kept()), resizing(), 1280.0);
        surface.script.borrow_mut().refusal = "No.".into();

        // Act.
        surface.drag_edge("[1] Orders", 100.0, true);
        let refused = surface.width_of("[1] Orders");
        surface.outside(page(vec![width("width/1", 1, 500)]), causes::OUTSIDE);

        // Assert.
        assert_eq!(refused, 240.0);
        assert!(surface.drawn("No."));
        assert_eq!(surface.width_of("[1] Orders"), 500.0);
    }

    #[test]
    fn a_column_refused_at_once_goes_back_to_the_width_it_keeps() {
        // Arrange.
        let mut surface = Surface::with(page(kept()), resizing(), 1280.0);
        surface.script.borrow_mut().at_once = "Not now.".into();

        // Act.
        surface.drag_edge("[1] Orders", 100.0, true);

        // Assert.
        assert_eq!(surface.sent().len(), 1);
        assert_eq!(surface.width_of("[1] Orders"), 240.0);
        assert!(surface.drawn("Not now."));
    }

    #[test]
    fn a_drag_let_go_where_it_started_and_a_step_past_the_range_send_nothing() {
        // Arrange: the first column kept at the narrowest, the second at the widest.
        let mut surface = Surface::with(
            page(vec![width("width/1", 1, 150), width("width/2", 2, 900)]),
            resizing(),
            1600.0,
        );

        // Act.
        let from = surface.edge_of("[1] Orders");
        surface.drag_edge("[1] Orders", 60.0, false);
        surface.frame(vec![egui::Event::PointerMoved(from)]);
        surface.frame(Vec::new());
        surface.release();
        surface.click_at(surface.edge_of("[1] Orders"));
        surface.key(egui::Key::ArrowLeft, egui::Modifiers::NONE);
        surface.click_at(surface.edge_of("[2] Payment"));
        surface.key(egui::Key::ArrowRight, egui::Modifiers::NONE);

        // Assert.
        assert_eq!(surface.sent(), []);
    }

    #[test]
    fn the_edges_are_named_by_their_column_and_say_their_width_and_range() {
        // Act.
        let surface = Surface::with(page(kept()), resizing(), 1280.0);

        // Assert: one edge on each level's box.
        assert_eq!(
            surface.edges(),
            [
                ("Width of column 1".to_owned(), 240.0, 150.0, 900.0),
                ("Width of column 1".to_owned(), 240.0, 150.0, 900.0),
                ("Width of column 2".to_owned(), 400.0, 150.0, 900.0),
                ("Width of column 3".to_owned(), 240.0, 150.0, 900.0),
            ]
        );
    }

    #[test]
    fn no_edge_is_offered_on_a_document_that_cannot_be_changed_nor_by_a_definition_that_keeps_no_widths() {
        // Act.
        let mut read_only = page(kept());
        read_only.read_only_reason = "Read only.".into();
        let read_only = Surface::with(read_only, resizing(), 1280.0);
        let unkept = Surface::with(page(kept()), definition(), 1280.0);

        // Assert.
        assert!(read_only.edges().is_empty());
        assert!(unkept.edges().is_empty());
    }

    #[test]
    fn the_levels_that_fit_are_counted_with_the_kept_widths_and_with_the_one_being_dragged() {
        // Act: 280 for the entries, then 240 and 240 fit 800; 400 for the first leaves no room for a second.
        let wide = Surface::with(page(vec![]), resizing(), 800.0);
        let kept_wide = Surface::with(page(vec![width("width/1", 1, 400)]), resizing(), 800.0);
        let mut dragged = Surface::with(page(vec![]), resizing(), 800.0);
        dragged.drag_edge("[1] Orders", 60.0, false);

        // Assert.
        assert!(wide.find(Role::TreeItem, "[2] Payment").is_some());
        assert!(kept_wide.find(Role::TreeItem, "[2] Payment").is_none());
        assert!(dragged.find(Role::TreeItem, "[2] Payment").is_none());
    }
    #[test]
    fn a_column_dragged_wider_than_the_page_has_room_for_stays_in_sight_until_it_is_let_go() {
        // Arrange: 280 for the entries, 240 for the first column and 900 for the second do not fit 1280.
        let mut surface = Surface::with(page(kept()), resizing(), 1280.0);

        // Act.
        surface.drag_edge("[2] Payment", 600.0, false);
        let during = surface
            .find(Role::TreeItem, "[2] Payment")
            .map(|n| n.bounds().unwrap().width());
        surface.release();

        // Assert: once let go, the page shows what fits again, the session having answered with the width it had.
        assert_eq!(during, Some(900.0));
        assert_eq!(surface.sent(), [sent("rewiden", "width/2", &[("pixels", "900")])]);
        assert!(surface.find(Role::TreeItem, "[2] Payment").is_some());
    }
}

// ---- The properties pane (etalii.adp spec 036, T057) ----

mod properties {
    use super::*;

    #[test]
    fn the_pane_names_the_selected_box_under_its_file_with_its_kind_and_where_it_stands() {
        // Arrange.
        let mut surface = Surface::of(three_levels());
        surface.pane = true;
        surface.frame(Vec::new());
        assert!(surface.drawn("Select a box to see its properties."));

        // Act.
        surface.click_box("[4] Pay by card");

        // Assert.
        assert!(surface.drawn("requirements.md › [4] Pay by card"));
        assert!(surface.drawn("Point"));
        assert!(surface.drawn("[3] Payment"));

        // A top level stands in the document.
        surface.click_box("[1] Orders");
        assert!(surface.drawn("Section"));
        assert!(surface.drawn("Requirements"));
    }

    #[test]
    fn each_action_is_a_button_with_its_key_that_runs_it_and_one_that_cannot_run_says_why() {
        // Arrange.
        let mut surface = Surface::of(three_levels());
        surface.pane = true;
        surface.script.borrow_mut().unavailable = "Nothing goes in here.".into();
        surface.click_box("[2] Take an order");

        // Assert: both offered, each with the key that does the same.
        let below = surface.get(Role::Button, "below");
        assert_eq!(below.keyboard_shortcut(), Some("Ctrl+Enter"));
        assert!(!below.is_disabled());
        let inside = surface.get(Role::Button, "inside");
        assert_eq!(inside.keyboard_shortcut(), Some("Ctrl+Shift+Enter"));
        assert!(inside.is_disabled());
        assert_eq!(inside.description(), Some("Nothing goes in here."));

        // Act: the one that can run is pressed.
        surface.click(Role::Button, "below");

        // Assert.
        assert_eq!(surface.sent(), [sent("jot-below", "2", &[])]);
    }
}

// ---- What a screen reader is told (etalii.adp spec 036, FR-011, T055; the standalone host's ARIA at a7fdf03) ----

mod accessibility {
    use super::*;

    #[test]
    fn the_page_is_a_tree_named_by_the_document_its_boxes_its_items_at_their_depth() {
        // Arrange.
        let surface = Surface::of(three_levels());

        // Assert.
        assert!(surface.stands_in(Role::TreeItem, "[1] Orders", Role::Tree, "Requirements"));
        assert!(surface.stands_in(Role::TreeItem, "[6] Refund in full", Role::Tree, "Requirements"));
        let deep = surface.get(Role::TreeItem, "[6] Refund in full");
        assert_eq!((deep.level(), deep.is_expanded()), (Some(4), Some(true)));
        assert!(!surface.get(Role::Tree, "Requirements").is_busy());
    }

    #[test]
    fn the_page_is_busy_while_an_edit_waits_on_its_answer() {
        // Arrange.
        let mut surface = Surface::of(three_levels());

        // Act.
        surface.tab.pending = 1;
        surface.frame(Vec::new());

        // Assert.
        assert!(surface.get(Role::Tree, "Requirements").is_busy());
    }

    #[test]
    fn the_trail_is_a_navigation_whose_step_at_the_pages_left_edge_is_the_current_one() {
        // Arrange.
        let mut surface = Surface::of(three_levels());
        let current = |surface: &Surface| {
            surface
                .nodes
                .values()
                .filter(|n| n.aria_current() == Some(egui::accesskit::AriaCurrent::Page))
                .map(|n| n.value().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        };

        // Act.
        surface.click_box("[5] Refunds");

        // Assert: the levels down to Refunds are links in the trail, and the title, the whole page, is the current step.
        for step in ["[1] Orders", "[3] Payment", "[5] Refunds"] {
            assert!(surface.stands_in(Role::Link, step, Role::Navigation, "Trail"), "{step}");
        }
        assert_eq!(current(&surface), ["Requirements"]);

        // Act: Payment made the page's left edge.
        surface.click(Role::Link, "[3] Payment");

        // Assert.
        assert_eq!(current(&surface), ["[3] Payment"]);
        assert!(surface.find(Role::Link, "[3] Payment").is_none());
        assert!(surface.stands_in(Role::Link, "Requirements", Role::Navigation, "Trail"));
    }

    #[test]
    fn the_findings_are_a_list_of_items_each_named_by_its_message() {
        // Arrange.
        let mut model = three_levels();
        model.findings = vec![SurfaceFinding {
            code: "std.missingId".into(),
            severity: "warning".into(),
            message: "Refund in full has no number.".into(),
            node_id: "6".into(),
            line: 12,
        }];

        // Act.
        let surface = Surface::of(model);

        // Assert.
        assert!(surface.stands_in(Role::ListItem, "Refund in full has no number.", Role::List, "Findings"));
        assert!(surface.stands_in(
            Role::Link,
            "Refund in full has no number.",
            Role::ListItem,
            "Refund in full has no number."
        ));
        assert!(surface.find(Role::Image, "warning").is_some());
    }

    #[test]
    fn the_read_only_reason_is_a_status_and_a_refusal_an_alert() {
        // Arrange.
        let mut model = three_levels();
        model.read_only_reason = "This document was written by a later version.".into();
        let read_only = Surface::of(model);
        let mut refused = Surface::of(three_levels());

        // Act.
        refused.tab.refusal = "No more points here.".into();
        refused.frame(Vec::new());

        // Assert.
        assert!(
            read_only
                .find(Role::Status, "This document was written by a later version.")
                .is_some()
        );
        assert!(refused.find(Role::Alert, "No more points here.").is_some());
    }
}

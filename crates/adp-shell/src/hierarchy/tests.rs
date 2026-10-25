//! The hierarchy tab's tests: the layout rules (the standalone host's `iconsLayout.test.ts` and the fit test of
//! `ColumnEdge.test.tsx` at a7fdf03; etalii.adp spec 036, T023), and the page drawn headless and pressed through its
//! accessibility tree (`HierarchySurface.test.tsx`; T026 to T029), and changed there (`editing/AddControl.test.tsx`; T038).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use adp_desl::hierarchy::definition::{
    AddDefinition, Deletion, FOCUS_ICON, HIDDEN_LEVELS_ICON, LabelDefinition, MoveParams, Operations, SurfaceDefinition,
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
}

impl HierarchySurfaceSession for Recorder {
    fn model(&self) -> SurfaceModel {
        self.model.clone()
    }

    fn gesture(&mut self, gesture: Gesture) -> Result<(), String> {
        let mut script = self.script.borrow_mut();
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
        vec![
            action("below", false, "jot-below"),
            action("inside", true, "jot-inside"),
        ]
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
    nodes: HashMap<NodeId, Node>,
}

impl Surface {
    fn of(model: SurfaceModel) -> Surface {
        Surface::at(model, 1280.0)
    }

    /// A page `width` points wide, as `pageWidth` gives the standalone test's.
    fn at(model: SurfaceModel, width: f32) -> Surface {
        let gestures = Rc::new(RefCell::new(Vec::new()));
        let script = Rc::new(RefCell::new(Script::default()));
        let session = Recorder {
            model,
            gestures: gestures.clone(),
            script: script.clone(),
            changes: Vec::new(),
        };
        let path = Path::new("requirements.md");
        let tab = HierarchyTab::with_session(path, path, Box::new(session), definition());
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
            nodes: HashMap::new(),
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
        let mut output = self.ctx.run_ui(input, |ui| super::show(ui, tab, &theme, &palette));
        output.textures_delta.clear();
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

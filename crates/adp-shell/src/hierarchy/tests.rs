//! The hierarchy tab's tests: the layout rules (the standalone host's `iconsLayout.test.ts` and the fit test of
//! `ColumnEdge.test.tsx` at a7fdf03; etalii.adp spec 036, T023), and the page drawn headless and pressed through its
//! accessibility tree (`HierarchySurface.test.tsx`; T026 to T029).

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use adp_desl::hierarchy::definition::{
    AddDefinition, FOCUS_ICON, HIDDEN_LEVELS_ICON, LabelDefinition, MoveParams, Operations, SurfaceDefinition,
};
use adp_desl::hierarchy::model::{
    EditOutcome, Gesture, LabelValues, OptionList, SurfaceChange, SurfaceFinding, SurfaceModel, SurfaceNode,
    SurfaceOption, causes, roles,
};
use adp_desl::hierarchy::session::HierarchySurfaceSession;
use eframe::egui::{
    self,
    accesskit::{Node, NodeId, Role},
};

use super::HierarchyTab;
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

/// A session that keeps one model and records the gestures it is sent, each written at once.
struct Recorder {
    model: SurfaceModel,
    gestures: Rc<RefCell<Vec<Gesture>>>,
    changes: Vec<SurfaceChange>,
}

impl HierarchySurfaceSession for Recorder {
    fn model(&self) -> SurfaceModel {
        self.model.clone()
    }

    fn gesture(&mut self, gesture: Gesture) -> Result<(), String> {
        self.changes.push(SurfaceChange::Model(Box::new(self.model.clone())));
        self.changes.push(SurfaceChange::Outcome(EditOutcome {
            edit_id: gesture.edit_id.clone(),
            written: true,
            error: String::new(),
        }));
        self.gestures.borrow_mut().push(gesture);
        Ok(())
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

    fn file_changed(&mut self, _path: &Path) {}
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

fn add(operation: &str, label: &str) -> AddDefinition {
    AddDefinition {
        operation: operation.into(),
        label: label.into(),
        icon: "mdi-plus".into(),
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
            add_top_level: add("open", "New section"),
            add_entry: add("jot", "New point"),
            add_level: add("nest", "New section"),
            move_level: "shift".into(),
            move_entry: "slide".into(),
            delete: "drop".into(),
        },
        move_params: MoveParams {
            into: "section".into(),
            after: "behind".into(),
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

/// One page in a headless window, its frames' texts and accessibility tree kept.
struct Surface {
    ctx: egui::Context,
    tab: HierarchyTab,
    gestures: Rc<RefCell<Vec<Gesture>>>,
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
        let session = Recorder {
            model,
            gestures: gestures.clone(),
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

//! The canvas in headless egui frames (etalii.adp spec 030, Story 2 scenarios 3 and 4; tasks T055 to T057): the
//! standalone host's dependency graph and causal loop examples and its hype cycle fixture with notes, vendored under
//! `adp-diagram`'s test fixtures, opened on a canvas, drawn, clicked, dragged, resized, connected, renamed, asked and
//! edited in the property grid, with every write checked against the bytes on disk and undone through the project's
//! history.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use adp_context::SelectMode;
use adp_disl::behavior::ids;
use adp_scene::Point;
use adp_scene::scene::{Owner, Primitive, Role};
use adp_theme::Theme;
use eframe::egui::{self, Event, Key, Modifiers, PointerButton, Pos2, RawInput};

use super::gestures::Intent;
use super::{DiagramHost, DiagramTab};
use crate::theme::{self, Palette};

const SCREEN: egui::Vec2 = egui::vec2(1280.0, 800.0);

fn fixtures(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../adp-diagram/tests/fixtures")
        .join(name)
}

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor")
}

/// A canvas over a copy of a fixture, drawn in headless frames.
struct Harness {
    ctx: egui::Context,
    host: DiagramHost,
    tab: DiagramTab,
    root: PathBuf,
    body: PathBuf,
    original: String,
    /// Every text the last frame painted, with where.
    texts: Vec<(String, egui::Rect)>,
    /// The textures every frame so far set, and the shapes the last one painted: what a screenshot rasterises.
    textures: HashMap<egui::TextureId, egui::ColorImage>,
    shapes: Vec<egui::epaint::ClippedShape>,
    /// The window's size.
    screen: egui::Vec2,
    /// Whether the property grid is drawn beside the canvas, as the shell draws it.
    grid: bool,
    _folder: tempfile::TempDir,
}

impl Harness {
    fn open(fixture: &str, files: &[&str], open: &str, body: &str) -> Harness {
        Harness::open_from(&fixtures(fixture), files, open, body)
    }

    fn open_from(source: &Path, files: &[&str], open: &str, body: &str) -> Harness {
        let folder = tempfile::tempdir().unwrap();
        for name in files {
            // A solution's or a folder subject's files sit in folders of their own.
            if let Some(parent) = folder.path().join(name).parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::copy(source.join(name), folder.path().join(name)).unwrap();
        }
        let root = folder.path().to_path_buf();
        let mut host = DiagramHost::with_vendor(&vendor());
        assert!(host.claims(&root.join(open)), "a usable tool type claims {open}");
        let tab = host.open(&root.join(open), &root).unwrap();
        let ctx = egui::Context::default();
        theme::install(&ctx);
        ctx.set_theme(egui::Theme::Dark);
        let body = root.join(body);
        let mut harness = Harness {
            original: fs::read_to_string(&body).unwrap(),
            ctx,
            host,
            tab,
            root,
            body,
            texts: Vec::new(),
            textures: HashMap::new(),
            shapes: Vec::new(),
            screen: SCREEN,
            grid: false,
            _folder: folder,
        };
        harness.frame(Vec::new());
        harness
    }

    fn dependency_graph() -> Harness {
        Harness::open(
            "dependency-graph/example-1",
            &["services.adp", "services.dgr"],
            "services.dgr",
            "services.dgr",
        )
    }

    /// One frame with `events`; the number of shapes it painted.
    fn frame(&mut self, events: Vec<Event>) -> usize {
        self.frame_with(events, Modifiers::NONE)
    }

    fn frame_with(&mut self, mut events: Vec<Event>, modifiers: Modifiers) -> usize {
        events.insert(0, Event::ModifiersChanged(modifiers));
        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, self.screen)),
            events,
            ..Default::default()
        };
        let theme = Theme::dark();
        let palette = Palette::of(&theme);
        let (host, tab, root, grid) = (&mut self.host, &mut self.tab, &self.root, self.grid);
        let mut output = self.ctx.run_ui(input, |ui| {
            let workspace = host.workspace(root);
            if grid {
                egui::Panel::right("grid")
                    .exact_size(320.0)
                    .show(ui, |ui| crate::properties::show(ui, tab, workspace, &theme, &palette));
            }
            egui::CentralPanel::default().show(ui, |ui| tab.show(ui, workspace, &theme, &palette));
        });
        crate::screenshot::take_textures(&mut self.textures, &mut output);
        self.shapes = output.shapes.clone();
        fn texts(shape: &egui::Shape, into: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => into.push((text.galley.text().to_owned(), text.visual_bounding_rect())),
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| texts(s, into)),
                _ => {}
            }
        }
        self.texts.clear();
        for clipped in &output.shapes {
            texts(&clipped.shape, &mut self.texts);
        }
        output.shapes.len()
    }

    /// Frames enough for a new window to be measured and drawn, and for a section to finish opening.
    fn settle(&mut self) {
        for _ in 0..30 {
            self.frame(Vec::new());
        }
    }

    /// Where the last frame painted `text`, the last of them when it painted it more than once.
    fn text(&self, text: &str) -> Option<Pos2> {
        self.texts
            .iter()
            .rev()
            .find(|(painted, _)| painted == text)
            .map(|(_, rect)| rect.center())
    }

    fn workspace(&mut self) -> &mut adp_diagram::DiagramWorkspace {
        self.host.workspace(&self.root)
    }

    /// Where an element's centre is on the screen.
    fn screen_of(&self, id: &str) -> Pos2 {
        let bounds = self.tab.bounds_of(id).unwrap_or_else(|| panic!("{id} is drawn"));
        self.tab.view.to_screen(self.tab.surface, bounds.centre())
    }

    fn position(&self, id: &str) -> (f64, f64) {
        self.host.workspaces[&self.root]
            .documents()
            .get(&self.body)
            .unwrap()
            .positions()[id]
    }

    fn written(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    fn press(&mut self, at: Pos2, button: PointerButton, modifiers: Modifiers) {
        self.frame_with(
            vec![
                Event::PointerMoved(at),
                Event::PointerButton {
                    pos: at,
                    button,
                    pressed: true,
                    modifiers,
                },
            ],
            modifiers,
        );
    }

    fn release(&mut self, at: Pos2, button: PointerButton, modifiers: Modifiers) {
        self.frame_with(
            vec![Event::PointerButton {
                pos: at,
                button,
                pressed: false,
                modifiers,
            }],
            modifiers,
        );
    }

    fn click(&mut self, at: Pos2, modifiers: Modifiers) {
        self.press(at, PointerButton::Primary, modifiers);
        self.release(at, PointerButton::Primary, modifiers);
    }

    /// A drag in steps from `from` to `to`, released there unless `release` is false.
    fn drag(&mut self, from: Pos2, to: Pos2, button: PointerButton, release: bool) {
        self.press(from, button, Modifiers::NONE);
        for step in 1..=4 {
            let t = step as f32 / 4.0;
            self.frame(vec![Event::PointerMoved(from + (to - from) * t)]);
        }
        if release {
            self.release(to, button, Modifiers::NONE);
        }
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame_with(
            vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            modifiers,
        );
    }
}

fn bodies(scene: &adp_scene::Scene) -> usize {
    scene
        .primitives
        .iter()
        .filter(|p| matches!(p, Primitive::Shape(s) if s.role == Role::Body && matches!(s.owner, Owner::Element(_))))
        .count()
}

#[test]
fn a_dependency_graph_is_drawn_from_its_scene_fitted_to_the_view_and_culled_to_it() {
    let mut harness = Harness::dependency_graph();
    let scene = harness.tab.scene().expect("a scene");
    assert_eq!(bodies(scene), 10, "every node has its body");
    assert!(
        scene.primitives.iter().any(|p| matches!(p, Primitive::Edge(_))),
        "and its lines"
    );
    let all = harness.frame(Vec::new());
    assert!(all > 9 * 2, "bodies, labels and lines are painted: {all}");
    // Every node is in view after fitting.
    let surface = harness.tab.surface;
    for id in ["checkout-web", "payments-db", "notification-service"] {
        assert!(surface.contains(harness.screen_of(id)), "{id} is in view");
    }
    // The dependency graph places a node by its top-left corner, x and row × 60.
    let gateway = harness.tab.bounds_of("api-gateway").unwrap();
    assert_eq!(
        (gateway.x, gateway.y, gateway.width),
        (270.42873351852575, 240.0, 160.0)
    );

    // Zoomed far in on one corner, most of it is culled away.
    harness.tab.zoom_by(8.0, Some(Point::new(gateway.x, gateway.y)));
    let zoomed = harness.frame(Vec::new());
    assert!(zoomed < all, "{zoomed} shapes in view of {all}");
    harness.tab.fit();
    assert_eq!(harness.frame(Vec::new()), all, "fit to view shows it all again");
}

#[test]
fn the_scene_is_rebuilt_only_when_what_it_shows_changes() {
    let mut harness = Harness::dependency_graph();
    let before = harness.tab.scene().unwrap() as *const _;
    harness.frame(Vec::new());
    assert_eq!(
        before,
        harness.tab.scene().unwrap() as *const _,
        "an idle frame keeps the scene"
    );
    let at = harness.screen_of("api-gateway");
    harness.click(at, Modifiers::NONE);
    assert_eq!(harness.tab.selection, ["api-gateway"]);
    let selected = harness.tab.scene().unwrap();
    let outline = selected
        .of(&Owner::Element("api-gateway".into()))
        .find_map(|p| match p {
            Primitive::Shape(s) if s.role == Role::Body => s.stroke.clone(),
            _ => None,
        })
        .unwrap();
    assert_eq!(outline.width, 3.0, "the selection is drawn as the scene's highlight");
}

#[test]
fn a_click_selects_ctrl_toggles_and_a_click_on_empty_canvas_clears() {
    let mut harness = Harness::dependency_graph();
    let gateway = harness.screen_of("api-gateway");
    let orders = harness.screen_of("orders-db");
    harness.click(gateway, Modifiers::NONE);
    assert_eq!(harness.tab.selection, ["api-gateway"]);
    harness.click(orders, Modifiers::COMMAND);
    assert_eq!(harness.tab.selection, ["api-gateway", "orders-db"]);
    harness.click(gateway, Modifiers::COMMAND);
    assert_eq!(harness.tab.selection, ["orders-db"]);
    let empty = harness.tab.surface.min + egui::vec2(3.0, 3.0);
    harness.click(empty, Modifiers::NONE);
    assert!(harness.tab.selection.is_empty());
}

#[test]
fn a_left_drag_on_empty_canvas_pans_keeps_the_selection_and_escape_puts_the_view_back() {
    let mut harness = Harness::dependency_graph();
    let gateway = harness.screen_of("api-gateway");
    let orders = harness.screen_of("orders-db");
    harness.click(gateway, Modifiers::NONE);
    harness.click(orders, Modifiers::COMMAND);
    let empty = harness.tab.surface.min + egui::vec2(3.0, 3.0);
    let origin = harness.tab.view.origin;
    harness.drag(empty, empty + egui::vec2(100.0, 50.0), PointerButton::Primary, true);
    let zoom = harness.tab.view.zoom;
    let panned = harness.tab.view.origin;
    assert!(
        (panned.x - (origin.x - 100.0 / zoom)).abs() < 1e-6 && (panned.y - (origin.y - 50.0 / zoom)).abs() < 1e-6,
        "{panned:?} from {origin:?}"
    );
    assert_eq!(
        harness.tab.selection,
        ["api-gateway", "orders-db"],
        "a pan draws no selection box and clears nothing"
    );
    harness.drag(empty, empty + egui::vec2(100.0, 50.0), PointerButton::Primary, false);
    assert_ne!(harness.tab.view.origin, panned, "the second drag pans too");
    harness.key(Key::Escape, Modifiers::NONE);
    assert_eq!(harness.tab.view.origin, panned);
    harness.release(empty + egui::vec2(100.0, 50.0), PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.tab.view.origin, panned, "the release after Escape is nothing");
    assert_eq!(harness.written(), harness.original);
}

#[test]
fn a_drag_moves_a_node_writes_its_line_and_one_undo_puts_the_bytes_back() {
    let mut harness = Harness::dependency_graph();
    let (x, row) = harness.position("api-gateway");
    let from = harness.screen_of("api-gateway");
    let zoom = harness.tab.view.zoom;
    harness.drag(from, from + egui::vec2(40.0, 0.0), PointerButton::Primary, true);
    let (moved_x, moved_row) = harness.position("api-gateway");
    assert!((moved_x - (x + 40.0 / zoom)).abs() < 1e-3, "{moved_x} from {x}");
    assert_eq!(moved_row, row, "a row is snapped to");
    assert_ne!(harness.written(), harness.original);
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

#[test]
fn a_drag_of_two_selected_nodes_moves_both_as_one_undo_and_a_vertical_drag_lands_on_a_row() {
    let mut harness = Harness::dependency_graph();
    let gateway = harness.screen_of("api-gateway");
    let orders = harness.screen_of("orders-db");
    harness.click(gateway, Modifiers::NONE);
    harness.click(orders, Modifiers::COMMAND);
    let zoom = harness.tab.view.zoom;
    let rows_down = egui::vec2(0.0, (2.0 * 60.0 * zoom) as f32);
    harness.drag(gateway, gateway + rows_down, PointerButton::Primary, true);
    // The dependency graph's module places a node in its own coordinates, 60 canvas units a row (`DependencyGraphRows`).
    assert_eq!(harness.position("api-gateway").1, 6.0 * 60.0);
    assert_eq!(harness.position("orders-db").1, 4.0 * 60.0);
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original, "one undo reverts both");
}

#[test]
fn escape_cancels_a_drag_and_nothing_is_written() {
    let mut harness = Harness::dependency_graph();
    let from = harness.screen_of("api-gateway");
    harness.drag(from, from + egui::vec2(80.0, 40.0), PointerButton::Primary, false);
    assert!(harness.tab.arbiter.moving().is_some(), "the drag is in flight");
    let dragged = harness.tab.scene().unwrap().clone();
    harness.key(Key::Escape, Modifiers::NONE);
    assert!(harness.tab.arbiter.is_idle());
    harness.release(from + egui::vec2(80.0, 40.0), PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.written(), harness.original);
    assert_ne!(&dragged, harness.tab.scene().unwrap(), "the preview is gone");
}

#[test]
fn a_right_click_opens_the_menu_and_a_right_drag_does_nothing_where_the_specification_draws_no_relation_with_it() {
    let mut harness = Harness::dependency_graph();
    assert!(!harness.tab.arbiter.connects_on_right_drag);
    let empty = harness.tab.surface.min + egui::vec2(3.0, 3.0);
    let origin = harness.tab.view.origin;
    harness.drag(empty, empty + egui::vec2(100.0, 50.0), PointerButton::Secondary, true);
    assert_eq!(harness.tab.view.origin, origin, "a right drag does not pan");
    assert!(harness.tab.menu.is_none(), "a right drag opens no menu");

    let gateway = harness.screen_of("api-gateway");
    let orders = harness.screen_of("orders-db");
    harness.drag(gateway, orders, PointerButton::Secondary, true);
    assert!(harness.tab.arbiter.is_idle());
    assert!(harness.tab.menu.is_none());
    assert!(harness.tab.selection.is_empty(), "nor selects");
    assert_eq!(harness.written(), harness.original, "nor connects");

    harness.press(gateway, PointerButton::Secondary, Modifiers::NONE);
    harness.release(gateway, PointerButton::Secondary, Modifiers::NONE);
    assert!(harness.tab.menu.is_some(), "a right click opens the menu");
    assert_eq!(harness.tab.selection, ["api-gateway"]);
}

#[test]
fn ctrl_wheel_zooms_about_the_pointer_and_the_keyboard_zooms_and_fits() {
    let mut harness = Harness::dependency_graph();
    let fitted = harness.tab.view;
    let at = harness.screen_of("api-gateway");
    let under = harness.tab.view.to_canvas(harness.tab.surface, at);
    harness.frame_with(
        vec![
            Event::PointerMoved(at),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, 1.0),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Wheel,
                modifiers: Modifiers::COMMAND,
            },
        ],
        Modifiers::COMMAND,
    );
    // A wheel's zoom is spread over a few frames.
    for _ in 0..30 {
        harness.frame(Vec::new());
    }
    assert!(harness.tab.view.zoom > fitted.zoom, "Ctrl+wheel up zooms in");
    let still = harness.tab.view.to_canvas(harness.tab.surface, at);
    assert!(
        (still.x - under.x).abs() < 1e-3 && (still.y - under.y).abs() < 1e-3,
        "about the pointer"
    );
    harness.key(Key::Num0, Modifiers::NONE);
    assert_eq!(harness.tab.view, fitted, "0 fits to view");
    harness.key(Key::Minus, Modifiers::NONE);
    assert!(
        (harness.tab.view.zoom - fitted.zoom / super::ZOOM_STEP).abs() < 1e-9,
        "{} {}",
        harness.tab.view.zoom,
        fitted.zoom
    );
    assert!((harness.tab.view.zoom - fitted.zoom / super::ZOOM_STEP).abs() < 1e-9);
}

#[test]
fn a_connection_from_an_anchor_to_a_node_is_one_edit() {
    let mut harness = Harness::dependency_graph();
    harness.tab.ids = Some(Box::new(ids::fixed(&["r-web-db"])));
    let anchor = harness
        .tab
        .scene()
        .unwrap()
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::Hook(h)
                if h.owner == Owner::Element("checkout-web".into())
                    && matches!(&h.kind, adp_scene::scene::HookKind::Anchor { name, .. } if name == "right") =>
            {
                Some(h.area.centre())
            }
            _ => None,
        })
        .expect("the right anchor");
    let from = harness.tab.view.to_screen(harness.tab.surface, anchor);
    let to = harness.screen_of("orders-db");
    harness.drag(from, to, PointerButton::Primary, true);
    let written = harness.written();
    assert!(written.contains("id: r-web-db"), "{written}");
    assert!(written.contains("from: checkout-web") && written.contains("to: orders-db"));
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

/// The dependency graph through its registration, with its module (etalii.adp spec 030, T071): a dependency dragged
/// out of a node's right anchor and released on empty canvas makes the node it depends on there, on the nearest row,
/// as one edit, and a vertical drag lands on a row (`DependencyGraphRows`, `AddConnectedDependencyGraphElementCommand`).
#[test]
fn a_dependency_graph_opens_through_its_registration_and_a_dependency_released_on_empty_canvas_makes_its_node() {
    let mut harness = Harness::open(
        "dependency-graph/example-1",
        &["services.adp", "services.dgr"],
        "services.adp",
        "services.dgr",
    );
    assert_eq!(bodies(harness.tab.scene().expect("a scene")), 10, "every node is drawn");
    let identity = harness.tab.bounds_of("identity-service").unwrap();
    assert_eq!(
        (identity.x, identity.y),
        (797.5845690363473, 240.0),
        "x, and row 4 at 60 a row"
    );

    harness.tab.ids = Some(Box::new(ids::fixed(&["cache", "web-to-cache"])));
    let anchor = harness
        .tab
        .scene()
        .unwrap()
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::Hook(h)
                if h.owner == Owner::Element("checkout-web".into())
                    && matches!(&h.kind, adp_scene::scene::HookKind::Anchor { name, .. } if name == "right") =>
            {
                Some(h.area.centre())
            }
            _ => None,
        })
        .expect("the right anchor");
    let from = harness.tab.view.to_screen(harness.tab.surface, anchor);
    let to = harness
        .tab
        .view
        .to_screen(harness.tab.surface, Point::new(1000.0, 170.0));
    assert!(harness.tab.surface.contains(to), "the release is in view");
    harness.drag(from, to, PointerButton::Primary, true);
    let written = harness.written();
    assert!(
        written.contains("  - id: cache\r\n    label: New node\r\n"),
        "{written}"
    );
    assert!(
        written.contains("    row: 3\r\nrelations:\r\n"),
        "the nearest row to 170"
    );
    assert!(written.contains("  - id: web-to-cache\r\n    from: checkout-web\r\n    to: cache\r\n"));
    assert_eq!(harness.position("cache").1, 3.0 * 60.0);
    assert!(harness.workspace().undo().is_done());
    assert_eq!(
        harness.written(),
        harness.original,
        "one undo takes the node and its dependency"
    );

    let gateway = harness.screen_of("api-gateway");
    let zoom = harness.tab.view.zoom;
    harness.drag(
        gateway,
        gateway + egui::vec2(0.0, (70.0 * zoom) as f32),
        PointerButton::Primary,
        true,
    );
    assert!(
        harness
            .written()
            .contains("  - id: api-gateway\r\n    label: API gateway\r\n    x: 270.42873351852575\r\n    row: 5\r\n")
    );
}

#[test]
fn a_toolbox_entry_dropped_on_the_canvas_lands_on_the_nearest_row_and_is_selected() {
    let mut harness = Harness::dependency_graph();
    harness.tab.ids = Some(Box::new(ids::fixed(&["dropped"])));
    let items = harness
        .tab
        .session
        .toolbox(harness.host.workspaces[&harness.root].documents());
    let node = items.first().expect("a toolbox entry").id.clone();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.drop_tool(host.workspace(root), &node, Point::new(40.0, 185.0));
    assert_eq!(harness.position("dropped"), (40.0, 3.0 * 60.0));
    assert_eq!(harness.tab.selection, ["dropped"]);
}

#[test]
fn tab_adds_a_node_by_the_specifications_shortcut_and_f2_renames_inline() {
    let mut harness = Harness::dependency_graph();
    harness.tab.ids = Some(Box::new(ids::fixed(&["right-of-gateway", "gateway-to-right"])));
    let gateway = harness.screen_of("api-gateway");
    harness.click(gateway, Modifiers::NONE);
    harness.key(Key::Tab, Modifiers::NONE);
    let written = harness.written();
    assert!(
        written.contains("id: right-of-gateway"),
        "{:?} {:?}",
        harness.tab.notice,
        harness.ctx.memory(|m| m.focused())
    );
    assert_eq!(harness.tab.selection, ["right-of-gateway"], "the new node is selected");

    harness.key(Key::F2, Modifiers::NONE);
    let editor = harness.tab.editor.as_mut().expect("the inline editor is open");
    editor.text = "Edge router".into();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.commit_editor(host.workspace(root));
    assert!(harness.written().contains("label: Edge router"));
    assert!(harness.workspace().undo().is_done());
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

#[test]
fn a_property_of_several_elements_changes_all_of_them_as_one_undo() {
    let mut harness = Harness::dependency_graph();
    harness
        .tab
        .select(vec!["api-gateway".into(), "orders-db".into()], SelectMode::Replace);
    let rows = crate::properties::rows(&harness.tab, &harness.host.workspaces[&harness.root]);
    let label = rows.iter().find(|r| r.label == "Label").expect("a shared label row");
    assert_eq!(label.value, "", "they differ, so the shared value is empty");
    let id = label.id.clone();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    crate::properties::set(tab, host.workspace(root), &id, "Shared");
    let written = harness.written();
    assert_eq!(written.matches("label: Shared").count(), 2, "{written}");
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);

    // One element's grid draws its rows, read-only ones with their reason.
    harness.tab.select(vec!["api-gateway".into()], SelectMode::Replace);
    let rows = crate::properties::rows(&harness.tab, &harness.host.workspaces[&harness.root]);
    assert!(rows.iter().any(|r| r.label == "Label" && r.value == "API gateway"));
}

#[test]
fn the_property_grid_and_the_toolbox_draw_for_a_selection() {
    let mut harness = Harness::dependency_graph();
    harness.tab.select(vec!["api-gateway".into()], SelectMode::Replace);
    let theme = Theme::dark();
    let palette = Palette::of(&theme);
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    let mut output = harness.ctx.run_ui(RawInput::default(), |ui| {
        let workspace = host.workspace(root);
        egui::CentralPanel::default().show(ui, |ui| {
            crate::properties::show(ui, tab, workspace, &theme, &palette);
            super::toolbox::show(ui, tab, workspace, &theme, &palette);
        });
    });
    output.textures_delta.clear();
    assert!(output.shapes.len() > 4);
}

#[test]
fn a_causal_loop_diagram_opens_through_its_registration_and_moves_by_its_layout() {
    let mut harness = Harness::open(
        "causal-loop-diagram/business-philosophy",
        &["business-philosophy.adp", "business-philosophy.cld"],
        "business-philosophy.adp",
        "business-philosophy.cld",
    );
    let scene = harness.tab.scene().expect("a scene");
    assert_eq!(bodies(scene), 18, "every variable has its pill");
    // A registration's position is the variable's centre.
    let insight = harness.tab.bounds_of("variable:insight").expect("insight is drawn");
    assert_eq!(insight.centre(), Point::new(20.0, 120.0));
    let registration = harness.root.join("business-philosophy.adp");
    let original = fs::read_to_string(&registration).unwrap();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.apply(
        Intent::Move {
            ids: vec!["variable:insight".into()],
            delta: (10.0, -5.0),
        },
        host.workspace(root),
    );
    let moved = fs::read_to_string(&registration).unwrap();
    assert!(moved.contains("variable:insight: 30 115"), "{moved}");
    assert!(harness.workspace().undo().is_done());
    assert_eq!(fs::read_to_string(&registration).unwrap(), original);
}

#[test]
fn a_body_changed_on_disk_is_drawn_again() {
    let mut harness = Harness::dependency_graph();
    let changed = harness.original.replace("label: API gateway", "label: Front door");
    fs::write(&harness.body, &changed).unwrap();
    let bodies = harness.host.reconcile(&harness.body);
    assert!(bodies.iter().any(|b| harness.tab.shows(b)), "{bodies:?}");
    harness.tab.changed();
    harness.frame(Vec::new());
    let texts: Vec<&str> = harness
        .tab
        .scene()
        .unwrap()
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"Front door"), "{texts:?}");
}

impl Harness {
    /// The C4 container view of the parity corpus's courier model, through its registration.
    fn c4_containers() -> Harness {
        let courier = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/c4/container/courier");
        Harness::open_from(
            &courier,
            &["courier.dsl", "courier.containers.adp", "courier.layout.json"],
            "courier.containers.adp",
            "courier.dsl",
        )
    }

    /// The last frame as a PNG at `path` (`crate::screenshot`).
    fn screenshot(&self, path: &Path) {
        let background = Palette::of(&Theme::dark()).editor_background;
        crate::screenshot::write(&self.ctx, &self.shapes, &self.textures, SCREEN, background, path);
    }
}

fn texts_of(scene: &adp_scene::Scene) -> Vec<&str> {
    scene
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_c4_view_is_drawn_from_its_modules_layout_and_selected_moved_and_edited_through_the_session() {
    let mut harness = Harness::c4_containers();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // What the module laid out: the cards, the boundary around the system's containers, the lines, the title and
    // the key.
    let scene = harness.tab.scene().expect("the view is drawn");
    let texts = texts_of(scene);
    for expected in [
        "API Application",
        "[Container: Java and Spring Boot]",
        "Courier Tracking [Software System]",
        "Calls [JSON/HTTPS]",
        "Container diagram for Courier Tracking",
        "Key",
        "Person",
    ] {
        assert!(texts.contains(&expected), "{expected} is drawn: {texts:?}");
    }
    assert!(harness.text("API Application").is_some(), "the name is painted");
    // Every card at the centre the module gave it: spa's at (840, 0), where the parity script's move starts from.
    assert_eq!(harness.tab.bounds_of("spa").unwrap().centre(), Point::new(840.0, 0.0));

    // A click selects a card, and the property grid shows its rows through the session.
    let api = harness.screen_of("api");
    harness.click(api, Modifiers::NONE);
    assert_eq!(harness.tab.selection, vec!["api".to_owned()]);
    let rows = harness
        .tab
        .session
        .properties(harness.host.workspace(&harness.root).documents(), &["api"]);
    assert_eq!(rows[0].id, "c4.name");
    assert_eq!(rows[0].value, "API Application");

    // A drag records where the card went in the layout sidecar, one undo away; the body is not touched.
    let sidecar = harness.root.join("courier.layout.json");
    let before = fs::read_to_string(&sidecar).unwrap();
    harness.drag(api, api + egui::vec2(60.0, 30.0), PointerButton::Primary, true);
    let moved = fs::read_to_string(&sidecar).unwrap();
    assert!(moved.contains("\"api\""), "{moved}");
    assert_eq!(harness.written(), harness.original);
    assert!(harness.workspace().undo().is_done());
    assert!(!fs::read_to_string(&sidecar).unwrap().contains("\"api\""));
    assert_eq!(
        fs::read_to_string(&sidecar).unwrap().replace("\r\n", "\n"),
        before.replace("\r\n", "\n")
    );

    // A property edit through the session rewrites the one line, and the canvas draws the new name.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.session
        .set_property(host.workspace(root), &["api"], "c4.name", "Tracking API")
        .unwrap();
    assert!(harness.written().contains("api = container \"Tracking API\""));
    harness.tab.changed();
    harness.settle();
    assert!(texts_of(harness.tab.scene().unwrap()).contains(&"Tracking API"));

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/c4-container.png"));
}

impl Harness {
    fn hype_cycle() -> Harness {
        Harness::open(
            "gartner-hype-cycle-graph",
            &["triggers-and-notes.ghg"],
            "triggers-and-notes.ghg",
            "triggers-and-notes.ghg",
        )
    }

    /// The width of an element as the document keeps it.
    fn stored_width(&self, id: &str) -> Option<adp_diagram::Length> {
        self.host.workspaces[&self.root]
            .documents()
            .get(&self.body)
            .unwrap()
            .extents()
            .get(id)
            .and_then(|e| e.width)
    }

    /// The middle of a selected element's right edge, on the screen.
    fn right_edge_of(&self, id: &str) -> Pos2 {
        let bounds = self.tab.bounds_of(id).unwrap();
        self.tab
            .view
            .to_screen(self.tab.surface, Point::new(bounds.right(), bounds.centre().y))
    }
}

#[test]
fn a_note_is_drawn_at_the_size_it_keeps_and_a_drag_of_its_edge_resizes_it_as_one_undo() {
    let mut harness = Harness::hype_cycle();
    let note = harness.tab.bounds_of("note-1").expect("the note is drawn");
    assert_eq!(
        (note.width, note.height),
        (160.0, 64.0),
        "drawn at the width and height it keeps"
    );
    assert_eq!(harness.tab.bounds_of("note-2").unwrap().width, 120.0);

    // A press on the edge of an element that is not selected is a press on the element: it has no handles yet.
    let centre = harness.screen_of("note-1");
    harness.click(centre, Modifiers::NONE);
    assert_eq!(harness.tab.selection, ["note-1"]);
    let edge = harness.right_edge_of("note-1");
    assert!(matches!(
        harness.tab.hit(harness.tab.view.to_canvas(harness.tab.surface, edge)),
        super::gestures::Hit::Resize { .. }
    ));

    let zoom = harness.tab.view.zoom;
    harness.drag(edge, edge + egui::vec2(40.0, 25.0), PointerButton::Primary, false);
    assert!(harness.tab.arbiter.resizing().is_some(), "the resize is in flight");
    let handle = harness
        .tab
        .scene()
        .unwrap()
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::Hook(hook)
                if hook.kind == adp_scene::scene::HookKind::ResizeHandle(adp_scene::connectors::Edge::Right)
                    && matches!(&hook.owner, Owner::Element(id) if id == "note-1") =>
            {
                Some(hook.area)
            }
            _ => None,
        })
        .expect("the selected note keeps its handles");
    assert!(
        (handle.centre().x - (note.right() + 40.0 / zoom)).abs() < 0.5,
        "the preview carries the edge"
    );
    assert_eq!(
        harness.written(),
        harness.original,
        "nothing is written before the release"
    );
    harness.release(edge + egui::vec2(40.0, 25.0), PointerButton::Primary, Modifiers::NONE);

    let expected = 160.0 + 40.0 / zoom;
    match harness.stored_width("note-1") {
        Some(adp_diagram::Length::Canvas(width)) => {
            assert!((width - expected).abs() < 0.01, "{width}, not {expected}")
        }
        other => panic!("a width in canvas units, not {other:?}"),
    }
    let written = harness.written();
    let changed: Vec<(&str, &str)> = harness
        .original
        .lines()
        .zip(written.lines())
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(changed.len(), 1, "one line is written: {changed:?}");
    assert_eq!(changed[0].0, "    width: 160");
    assert!(changed[0].1.starts_with("    width: 1"), "{changed:?}");
    assert_eq!(
        harness.tab.bounds_of("note-1").unwrap().x,
        note.x,
        "the left edge stays put"
    );

    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original, "one undo puts every byte back");
}

#[test]
fn escape_cancels_a_resize_and_an_edge_stops_at_the_minimum_before_the_far_one() {
    let mut harness = Harness::hype_cycle();
    let centre = harness.screen_of("note-2");
    harness.click(centre, Modifiers::NONE);
    let edge = harness.right_edge_of("note-2");
    let before = harness.tab.bounds_of("note-2").unwrap();

    harness.drag(edge, edge + egui::vec2(60.0, 0.0), PointerButton::Primary, false);
    harness.key(Key::Escape, Modifiers::NONE);
    assert!(harness.tab.arbiter.is_idle());
    harness.release(edge + egui::vec2(60.0, 0.0), PointerButton::Primary, Modifiers::NONE);
    assert_eq!(harness.written(), harness.original, "Escape writes nothing");
    assert_eq!(
        harness.tab.bounds_of("note-2").unwrap(),
        before,
        "and the preview is gone"
    );

    // Dragged far past its left edge, the right edge stops one unit from it: a note states no minimum.
    let far = edge - egui::vec2(before.width as f32 * harness.tab.view.zoom as f32 * 3.0, 0.0);
    harness.drag(edge, far, PointerButton::Primary, true);
    assert_eq!(harness.stored_width("note-2"), Some(adp_diagram::Length::Canvas(1.0)));
    assert_eq!(harness.tab.bounds_of("note-2").unwrap().x, before.x);
}

#[test]
fn a_drag_of_the_left_edge_moves_the_note_and_sizes_it_in_one_edit() {
    let mut harness = Harness::hype_cycle();
    let centre = harness.screen_of("note-1");
    harness.click(centre, Modifiers::NONE);
    let before = harness.tab.bounds_of("note-1").unwrap();
    let left = harness
        .tab
        .view
        .to_screen(harness.tab.surface, Point::new(before.x, before.centre().y));
    let zoom = harness.tab.view.zoom;
    // A year to the left: the hype cycle's x axis is four canvas units a step of its unit, a year in this fixture.
    harness.drag(
        left,
        left - egui::vec2((4.0 * zoom) as f32, 0.0),
        PointerButton::Primary,
        true,
    );
    let written = harness.written();
    assert!(written.contains("    at: 1949-01\n"), "{written}");
    assert!(written.contains("    width: 164\n"), "{written}");
    let after = harness.tab.bounds_of("note-1").unwrap();
    assert!(
        (after.right() - before.right()).abs() < 1e-6,
        "the right edge stays put"
    );
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

fn option(id: &str, label: &str) -> adp_context::PromptOption {
    adp_context::PromptOption {
        id: id.into(),
        label: label.into(),
        selectable: true,
        ..Default::default()
    }
}

#[test]
fn a_choice_is_answered_on_the_canvas_with_an_option_that_may_be_chosen() {
    let mut harness = Harness::dependency_graph();
    let kinds = adp_context::PromptOption {
        id: "kinds".into(),
        label: "Kinds".into(),
        selectable: false,
        children: vec![
            option("first", "First"),
            adp_context::PromptOption {
                unavailable_reason: "Not now.".into(),
                ..option("second", "Second")
            },
        ],
        ..Default::default()
    };
    harness.tab.prompt = Some(super::PendingPrompt {
        target: adp_diagram::ActionTarget::Element("api-gateway".into()),
        action_id: "test.pick".into(),
        prompt: adp_context::Prompt::Choice {
            title: "Pick one".into(),
            icon: String::new(),
            confirm_label: "Pick".into(),
            options: vec![kinds, option("third", "Third")],
            empty_message: "Nothing to pick.".into(),
            name_field: None,
        },
        value: String::new(),
    });
    harness.settle();
    assert!(harness.text("Pick one").is_some(), "the question is asked");
    assert!(harness.text("Kinds").is_some(), "a group is a heading over its options");
    assert!(
        harness
            .text("This question cannot be answered on the canvas yet.")
            .is_none()
    );

    let pick = harness.text("Pick").unwrap();
    harness.click(pick, Modifiers::NONE);
    assert!(
        harness.tab.prompt.is_some(),
        "nothing is chosen yet, so Pick does nothing"
    );
    let second = harness.text("Second").unwrap();
    harness.click(second, Modifiers::NONE);
    assert_eq!(
        harness.tab.prompt.as_ref().unwrap().value,
        "",
        "an option that may not be chosen is not"
    );
    let third = harness.text("Third").unwrap();
    harness.click(third, Modifiers::NONE);
    assert_eq!(harness.tab.prompt.as_ref().unwrap().value, "third");
    let pick = harness.text("Pick").unwrap();
    harness.click(pick, Modifiers::NONE);
    assert!(harness.tab.prompt.is_none(), "the answer is handed to the session");
    assert_eq!(
        harness.tab.notice.as_deref(),
        Some("'test.pick' does not apply to this selection."),
        "which answers this made-up action with its refusal"
    );
}

#[test]
fn a_file_question_with_no_files_says_so_and_escape_dismisses_it() {
    let mut harness = Harness::dependency_graph();
    harness.tab.prompt = Some(super::PendingPrompt {
        target: adp_diagram::ActionTarget::Element("api-gateway".into()),
        action_id: "test.file".into(),
        prompt: adp_context::Prompt::File {
            title: "Open which file?".into(),
            icon: String::new(),
            confirm_label: "Open".into(),
            files: Vec::new(),
            pinned: Vec::new(),
            empty_message: "There are no files to open.".into(),
            name_field: None,
        },
        value: String::new(),
    });
    harness.settle();
    assert!(harness.text("There are no files to open.").is_some());
    harness.key(Key::Escape, Modifiers::NONE);
    assert!(harness.tab.prompt.is_none());
    assert_eq!(harness.written(), harness.original);
}

#[test]
fn a_submenu_of_the_context_menu_opens_in_place_and_runs_its_action() {
    let mut harness = Harness::dependency_graph();
    let target = adp_diagram::ActionTarget::Element("api-gateway".into());
    let documents = harness.host.workspaces[&harness.root].documents();
    let leaf = harness
        .tab
        .session
        .actions(documents, &target)
        .into_iter()
        .flat_map(|g| g.actions)
        .find(|a| a.items.is_empty() && a.is_available())
        .expect("the gateway has an action");
    let mut more = adp_context::Action::new("test.more", "More actions");
    more.items = vec![adp_context::ActionGroup {
        actions: vec![leaf.clone()],
    }];
    harness.tab.menu = Some(super::Menu {
        target,
        at: harness.tab.surface.center(),
        groups: vec![adp_context::ActionGroup { actions: vec![more] }],
        area: None,
    });
    harness.settle();
    assert!(harness.text(&leaf.label).is_none(), "a submenu starts closed");
    let heading = harness.text("More actions").expect("the submenu is drawn");
    harness.click(heading, Modifiers::NONE);
    assert!(
        harness.tab.menu.is_some(),
        "a press on the submenu is a press on the menu"
    );
    harness.settle();
    let entry = harness
        .text(&leaf.label)
        .expect("the submenu opened beneath its heading");
    harness.click(entry, Modifiers::NONE);
    assert!(harness.tab.menu.is_none(), "running an action closes the menu");
    assert!(
        harness.written() != harness.original
            || harness.tab.prompt.is_some()
            || harness.tab.editor.is_some()
            || harness.tab.notice.is_some(),
        "and {} ran",
        leaf.id
    );
}

impl Harness {
    /// The standalone host's coal-technologies example, as the parity script opens it: its registration and body.
    fn coal_technologies() -> Harness {
        Harness::open_from(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../parity/fixtures/gartner/hypecycle-graph/coal-technologies"),
            &["coal-technologies.adp", "coal-technologies.ghg"],
            "coal-technologies.adp",
            "coal-technologies.ghg",
        )
    }
}

/// A hype cycle graph opens, draws every trend on its month axis with the phase legend and the ruler, and edits
/// (etalii.adp spec 030, T073): a drag moves a trend by whole months, one undo puts the bytes back; Compact packs the
/// trends 24 units a phase, moves nothing and adds nothing on empty canvas, and switching it off draws true time again.
#[test]
fn a_hype_cycle_graph_draws_its_trends_legend_and_ruler_edits_and_toggles_compact() {
    let mut harness = Harness::coal_technologies();
    let scene = harness.tab.scene().expect("a scene");
    assert!(
        bodies(scene) > 34 + 9,
        "every trend, trigger and note has its body: {}",
        bodies(scene)
    );
    // Four canvas units a month from 1900-01, rows 56 apart: the Newcomen engine, 1712-01 to 1800-01 on row 0.
    let newcomen = harness.tab.bounds_of("newcomen-engine").unwrap();
    assert_eq!(
        (newcomen.x, newcomen.y, newcomen.width, newcomen.height),
        (-9024.0, 0.0, 4224.0, 32.0)
    );
    // A trigger by its centre: Rainhill, 1829-10 on row 13.
    let rainhill = harness.tab.bounds_of("rainhill-trials").unwrap();
    assert_eq!(rainhill.centre(), Point::new(-3372.0, 13.0 * 56.0 + 16.0));
    for phase in ["Peak", "Trough", "Slope", "Plateau"] {
        assert!(harness.text(phase).is_some(), "the legend shows {phase}");
    }
    assert!(
        harness
            .texts
            .iter()
            .any(|(text, _)| text.len() == 4 && text.parse::<i32>().is_ok_and(|y| (1700..=2100).contains(&y))),
        "the ruler labels the years in view"
    );

    // A drag of a trend by a year's width writes a year later, its length kept; one undo puts every byte back.
    let zoom = harness.tab.view.zoom;
    harness
        .tab
        .zoom_by(16.0 / zoom, Some(Point::new(newcomen.x + 100.0, newcomen.y + 16.0)));
    harness.frame(Vec::new());
    let zoom = harness.tab.view.zoom;
    let at = harness
        .tab
        .view
        .to_screen(harness.tab.surface, Point::new(newcomen.x + 100.0, newcomen.y + 16.0));
    harness.drag(
        at,
        at + egui::vec2((48.0 * zoom) as f32, 0.0),
        PointerButton::Primary,
        true,
    );
    let written = harness.written();
    assert!(
        written.contains("    start: 1713-01\n    stop: 1801-01\n    row: 0\n"),
        "{}",
        &written[..400]
    );
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
    harness.tab.changed();
    harness.frame(Vec::new());

    // Compact: 24 units a visible phase, packed; nothing moves, and empty canvas offers no addition.
    let documents = harness.host.workspaces[&harness.root].documents();
    harness.tab.set_toggled(documents, true);
    harness.tab.fit();
    harness.settle();
    let packed = harness.tab.bounds_of("newcomen-engine").unwrap();
    assert_eq!((packed.width, packed.height), (96.0, 32.0));
    let gigafactory_like = harness.tab.bounds_of("autonomous-mining").unwrap();
    assert_eq!(gigafactory_like.width, 48.0, "two visible phases");
    assert!(harness.text("Peak").is_some(), "the legend stays");
    let at = harness.screen_of("newcomen-engine");
    harness.drag(at, at + egui::vec2(80.0, 40.0), PointerButton::Primary, true);
    assert_eq!(harness.written(), harness.original, "nothing moves in Compact");
    let documents = harness.host.workspaces[&harness.root].documents();
    assert!(
        harness
            .tab
            .session
            .actions(documents, &adp_diagram::ActionTarget::Canvas { x: 0.0, y: 0.0 })
            .iter()
            .all(|group| group.actions.is_empty()),
        "Compact has no background menu"
    );
    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/hype-cycle-compact.png"));

    let documents = harness.host.workspaces[&harness.root].documents();
    harness.tab.set_toggled(documents, false);
    harness.settle();
    assert_eq!(
        harness.tab.bounds_of("newcomen-engine").unwrap().width,
        4224.0,
        "true time again"
    );
    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/hype-cycle.png"));
}

/// What the Story 2 walk found (`parity/walks.md`, F1 to F14), each held to the standalone host's behaviour.
#[path = "walk_tests.rs"]
mod walk;

// The causal loop diagram with its module (T069), in a file of its own.
mod causal_loop;

// The supply chain diagram with its module (T076), in a file of its own.
mod supply_chain;
// The Ansible structure diagram with its module (T079), in a file of its own.
mod ansible;
// The .NET dependency graph with its module (T082), in a file of its own.
mod dotnet;
// The Sankey diagram with its module (T077), in a file of its own.
mod sankey;
// The Helm chart diagram with its module (T080), in a file of its own.
mod helm;
// The SPARQL query diagram with its module (T084), in a file of its own.
mod sparql;
// The functional decomposition graph with its module (T072), in a file of its own.
mod fdg;
// The Azure DevOps pipeline with its module (T078), in a file of its own.
mod pipeline;
// The agent behavior model with its module (T074), in a file of its own.
mod abm;
// The agent activity diagram with its module (T075), in a file of its own.
mod agent_activity;
// The Databricks job with its module (T081), in a file of its own.
mod databricks;
// The Wardley map with its module (T070), in a file of its own.
mod wardley_map;

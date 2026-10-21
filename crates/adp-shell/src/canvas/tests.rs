//! The canvas in headless egui frames (etalii.adp spec 030, Story 2 scenarios 3 and 4; tasks T055 to T057): the
//! standalone host's dependency graph and causal loop examples, vendored under `adp-diagram`'s test fixtures, opened
//! on a canvas, drawn, clicked, dragged, connected, renamed and edited in the property grid, with every write checked
//! against the bytes on disk and undone through the project's history.

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
    _folder: tempfile::TempDir,
}

impl Harness {
    fn open(fixture: &str, files: &[&str], open: &str, body: &str) -> Harness {
        let folder = tempfile::tempdir().unwrap();
        for name in files {
            fs::copy(fixtures(fixture).join(name), folder.path().join(name)).unwrap();
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
            screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, SCREEN)),
            events,
            ..Default::default()
        };
        let theme = Theme::dark();
        let palette = Palette::of(&theme);
        let (host, tab, root) = (&mut self.host, &mut self.tab, &self.root);
        let mut output = self.ctx.run_ui(input, |ui| {
            let workspace = host.workspace(root);
            egui::CentralPanel::default().show(ui, |ui| tab.show(ui, workspace, &theme, &palette));
        });
        output.textures_delta.clear();
        output.shapes.len()
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
fn a_box_on_empty_canvas_selects_what_lies_inside_it() {
    let mut harness = Harness::dependency_graph();
    let surface = harness.tab.surface;
    harness.drag(
        surface.min + egui::vec2(2.0, 2.0),
        surface.max - egui::vec2(2.0, 2.0),
        PointerButton::Primary,
        true,
    );
    assert_eq!(harness.tab.selection.len(), 10, "{:?}", harness.tab.selection);
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
    assert_eq!(harness.position("api-gateway").1, 6.0);
    assert_eq!(harness.position("orders-db").1, 4.0);
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
fn a_right_drag_pans_and_escape_puts_the_view_back() {
    let mut harness = Harness::dependency_graph();
    let empty = harness.tab.surface.min + egui::vec2(3.0, 3.0);
    let origin = harness.tab.view.origin;
    harness.drag(empty, empty + egui::vec2(100.0, 50.0), PointerButton::Secondary, true);
    let zoom = harness.tab.view.zoom;
    let panned = harness.tab.view.origin;
    assert!(
        (panned.x - (origin.x - 100.0 / zoom)).abs() < 1e-6,
        "{panned:?} from {origin:?}"
    );
    harness.drag(empty, empty + egui::vec2(100.0, 50.0), PointerButton::Secondary, false);
    harness.key(Key::Escape, Modifiers::NONE);
    assert_eq!(harness.tab.view.origin, panned);
    assert_eq!(harness.written(), harness.original);
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
    assert_eq!(harness.position("dropped"), (40.0, 3.0));
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

#[test]
fn a_c4_registration_is_claimed_and_opened_by_its_module_whose_canvas_is_not_drawn_here_yet() {
    let folder = tempfile::tempdir().unwrap();
    let courier = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/c4/container/courier");
    for name in ["courier.dsl", "courier.containers.adp"] {
        fs::copy(courier.join(name), folder.path().join(name)).unwrap();
    }
    let root = folder.path().to_path_buf();
    let mut host = DiagramHost::with_vendor(&vendor());
    let registration = root.join("courier.containers.adp");
    assert!(host.claims(&registration), "the C4 module opens its registrations");
    let mut tab = host.open(&registration, &root).unwrap();
    assert!(tab.session.module_view().is_some());
    let documents = host.workspace(&root).documents();
    let elements = tab.session.render(documents).unwrap();
    assert!(elements.iter().any(|e| e.id == "api"), "{elements:?}");
    tab.ensure_definition(documents);
    assert_eq!(
        tab.definition.as_ref().err().map(String::as_str),
        Some(super::MODULE_CANVAS)
    );
}

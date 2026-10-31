//! The window as a screen reader reads it (etalii.adp spec 030, FR-041, task T088): every interactive element of the
//! explorer, its context menu, the dialogs, the tabs and the errors panel is in the AccessKit tree a frame hands the
//! platform, with its role, its name and its state.
//!
//! Each test opens a project in a headless window with AccessKit switched on, as the platform switches it on when a
//! screen reader starts, and reads the tree each frame produces. It finds what to press in that tree, by role and
//! name, and presses it where the tree says it is, so what is tested is what a screen reader, or a person, reaches.

use std::collections::HashMap;
use std::path::Path;

use adp_core::state::StateFolder;
use adp_shell::{shell::Shell, theme};
use eframe::egui::{
    self,
    accesskit::{Node, NodeId, Role, TreeUpdate},
};

/// The roles a person operates: each of these must carry a name.
const CONTROLS: &[Role] = &[
    Role::Button,
    Role::CheckBox,
    Role::Link,
    Role::ListItem,
    Role::MenuItem,
    Role::RadioButton,
    Role::Tab,
    Role::TextInput,
    Role::TreeItem,
];

/// A headless window over a project, and the accessibility tree of its last frame.
struct Window {
    ctx: egui::Context,
    shell: Shell,
    nodes: HashMap<NodeId, Node>,
    root: NodeId,
    focus: NodeId,
    time: f64,
    _folder: tempfile::TempDir,
}

impl Window {
    /// A window with the project of `files` open, kept in the project list so that its findings are kept.
    fn open(files: &[(&str, &str)]) -> Window {
        let folder = tempfile::tempdir().unwrap();
        let project = folder.path().join("courier");
        for (path, text) in files {
            let path = project.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        std::fs::create_dir_all(&project).unwrap();
        let mut shell = Shell::with_state(StateFolder::at(folder.path().join("state")));
        shell.open_path(&project);
        let ctx = egui::Context::default();
        theme::install(&ctx);
        ctx.enable_accesskit();
        let mut window = Window {
            ctx,
            shell,
            nodes: HashMap::new(),
            root: NodeId(0),
            focus: NodeId(0),
            time: 0.0,
            _folder: folder,
        };
        window.frames(2);
        window
    }

    /// One frame with `events`; its tree replaces the last one's.
    fn frame(&mut self, events: Vec<egui::Event>) {
        // A press and its release a frame apart are a click; two presses three frames apart are not a double click.
        self.time += 0.15;
        let input = egui::RawInput {
            time: Some(self.time),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events,
            ..Default::default()
        };
        let shell = &mut self.shell;
        let mut output = self.ctx.run_ui(input, |ui| shell.show(ui));
        output.textures_delta.clear();
        let TreeUpdate { nodes, tree, focus, .. } = output.platform_output.accesskit_update.expect("AccessKit is on");
        self.root = tree.expect("every frame sends the whole tree").root;
        self.nodes = nodes.into_iter().collect();
        self.focus = focus;
        self.assert_controls_are_named();
    }

    /// The cursor the window shows with the pointer at `at`, after one frame.
    fn cursor_at(&mut self, at: egui::Pos2) -> egui::CursorIcon {
        self.time += 0.15;
        let input = egui::RawInput {
            time: Some(self.time),
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))),
            events: vec![egui::Event::PointerMoved(at)],
            ..Default::default()
        };
        let shell = &mut self.shell;
        let mut output = self.ctx.run_ui(input, |ui| shell.show(ui));
        output.textures_delta.clear();
        output.platform_output.cursor_icon
    }

    /// Drags with the primary button from `from` to `to`, over a few frames, then lets the window settle.
    fn drag(&mut self, from: egui::Pos2, to: egui::Pos2) {
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(from), button(from, true)]);
        for step in 1..=4 {
            let at = from + (to - from) * (step as f32 / 4.0);
            self.frame(vec![egui::Event::PointerMoved(at)]);
        }
        self.frame(vec![button(to, false)]);
        self.frames(2);
    }

    /// Where a pane's edge is along the line from `from` to `to`: the first point there that shows a resize cursor.
    fn edge(&mut self, from: egui::Pos2, to: egui::Pos2) -> egui::Pos2 {
        let steps = (to - from).length().ceil() as usize;
        for step in 0..=steps {
            // Whole pixels: the grab of a pane's edge is narrow.
            let at = (from + (to - from) * (step as f32 / steps.max(1) as f32)).round();
            let cursor = self.cursor_at(at);
            if matches!(
                cursor,
                egui::CursorIcon::ResizeHorizontal
                    | egui::CursorIcon::ResizeVertical
                    | egui::CursorIcon::ResizeColumn
                    | egui::CursorIcon::ResizeRow
                    | egui::CursorIcon::ResizeEast
                    | egui::CursorIcon::ResizeWest
                    | egui::CursorIcon::ResizeNorth
                    | egui::CursorIcon::ResizeSouth
            ) {
                return at;
            }
        }
        panic!("no pane edge between {from:?} and {to:?}");
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.frame(Vec::new());
        }
    }

    /// The name a screen reader reads for `node`: its label, else what labels it, else its text.
    fn name(&self, node: &Node) -> String {
        if let Some(label) = node.label().filter(|l| !l.is_empty()) {
            return label.to_owned();
        }
        let by: Vec<String> = node
            .labelled_by()
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .map(|n| self.name(n))
            .collect();
        if !by.is_empty() {
            return by.join(" ");
        }
        node.value().unwrap_or_default().to_owned()
    }

    /// Every control in the tree has a name.
    fn assert_controls_are_named(&self) {
        let unnamed: Vec<String> = self
            .nodes
            .values()
            .filter(|n| CONTROLS.contains(&n.role()) && self.name(n).trim().is_empty())
            .map(|n| format!("{:?} at {:?}", n.role(), n.bounds()))
            .collect();
        assert!(unnamed.is_empty(), "controls without a name: {unnamed:?}");
    }

    /// The nodes of `role` named `name`, in the tree reached from the root.
    fn all(&self, role: Role, name: &str) -> Vec<(NodeId, &Node)> {
        self.descendants(self.root)
            .into_iter()
            .filter(|(_, n)| n.role() == role && self.name(n) == name)
            .collect()
    }

    /// The one node of `role` named `name`.
    fn find(&self, role: Role, name: &str) -> (NodeId, &Node) {
        let found = self.all(role, name);
        assert_eq!(
            found.len(),
            1,
            "one {role:?} named {name:?}; the tree has: {}",
            self.outline()
        );
        found[0]
    }

    /// The node a screen reader is told has the keyboard: the focused node, or its active descendant when it names
    /// one, as AccessKit resolves it.
    fn focused(&self) -> (Role, String) {
        let node = &self.nodes[&self.focus];
        let node = node
            .active_descendant()
            .and_then(|id| self.nodes.get(&id))
            .unwrap_or(node);
        (node.role(), self.name(node))
    }

    fn key(&mut self, key: egui::Key) {
        let event = |pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![event(true), event(false)]);
        self.frames(1);
    }

    fn has(&self, role: Role, name: &str) -> bool {
        !self.all(role, name).is_empty()
    }

    /// The nodes under `id`, depth first.
    fn descendants(&self, id: NodeId) -> Vec<(NodeId, &Node)> {
        let mut found = Vec::new();
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else { continue };
            found.push((id, node));
            stack.extend(node.children().iter().rev());
        }
        found
    }

    /// The controls under `id`, by role and name.
    fn controls_under(&self, id: NodeId) -> Vec<(Role, String)> {
        self.descendants(id)
            .into_iter()
            .skip(1)
            .filter(|(_, n)| CONTROLS.contains(&n.role()))
            .map(|(_, n)| (n.role(), self.name(n)))
            .collect()
    }

    /// The tree as text, for a failure to show.
    fn outline(&self) -> String {
        fn walk(window: &Window, id: NodeId, depth: usize, out: &mut String) {
            let Some(node) = window.nodes.get(&id) else { return };
            out.push_str(&format!(
                "\n{}{:?} {:?}",
                "  ".repeat(depth),
                node.role(),
                window.name(node)
            ));
            for child in node.children() {
                walk(window, *child, depth + 1, out);
            }
        }
        let mut out = String::new();
        walk(self, self.root, 0, &mut out);
        out
    }

    /// Where the tree says `node` is drawn.
    fn centre(node: &Node) -> egui::Pos2 {
        let bounds = node.bounds().expect("a control has bounds");
        egui::pos2(
            ((bounds.x0 + bounds.x1) / 2.0) as f32,
            ((bounds.y0 + bounds.y1) / 2.0) as f32,
        )
    }

    /// Presses and lets go of `button` on the node of `role` named `name`, then lets the window settle.
    fn press(&mut self, role: Role, name: &str, button: egui::PointerButton) {
        let at = Window::centre(self.find(role, name).1);
        let event = |pressed| egui::Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.frame(vec![egui::Event::PointerMoved(at), event(true)]);
        self.frame(vec![event(false)]);
        self.frames(2);
    }

    fn click(&mut self, role: Role, name: &str) {
        self.press(role, name, egui::PointerButton::Primary);
    }

    fn right_click(&mut self, role: Role, name: &str) {
        self.press(role, name, egui::PointerButton::Secondary);
    }
}

const NOTES: (&str, &str) = ("notes.md", "# Notes\n");
const ROADMAP: (&str, &str) = ("docs/roadmap.md", "# Roadmap\n");
const BROKEN: (&str, &str) = ("service.json", "{ \"name\": }\n");

#[test]
fn the_explorer_is_a_tree_of_its_entries_with_their_level_selection_and_expansion() {
    let mut window = Window::open(&[NOTES, ROADMAP]);
    let (tree, _) = window.find(Role::Tree, "Explorer");
    let items: Vec<(Role, String)> = window.controls_under(tree);
    assert_eq!(
        items,
        vec![
            (Role::TreeItem, "docs".to_owned()),
            (Role::TreeItem, "notes.md".to_owned())
        ],
        "folders first, each entry by its name"
    );
    let (_, docs) = window.find(Role::TreeItem, "docs");
    assert_eq!(docs.level(), Some(1));
    assert_eq!(docs.is_expanded(), Some(false), "a closed folder says so");
    assert_eq!(docs.is_selected(), Some(false));
    let (_, notes) = window.find(Role::TreeItem, "notes.md");
    assert_eq!(
        notes.is_expanded(),
        None,
        "a file without registrations neither opens nor closes"
    );

    window.click(Role::TreeItem, "docs");
    let (_, docs) = window.find(Role::TreeItem, "docs");
    assert_eq!(docs.is_expanded(), Some(true), "an open folder says so");
    assert_eq!(docs.is_selected(), Some(true), "the entry pressed is the selected one");
    let (_, roadmap) = window.find(Role::TreeItem, "roadmap.md");
    assert_eq!(roadmap.level(), Some(2), "an entry inside a folder is a level deeper");

    // The keyboard moves through the rows, and a screen reader is told which row has it.
    assert_eq!(
        window.focused(),
        (Role::TreeItem, "docs".to_owned()),
        "the row pressed has the keyboard"
    );
    window.key(egui::Key::ArrowDown);
    assert_eq!(window.focused(), (Role::TreeItem, "roadmap.md".to_owned()));
    assert_eq!(window.find(Role::TreeItem, "roadmap.md").1.is_selected(), Some(true));
    window.key(egui::Key::ArrowDown);
    assert_eq!(window.focused(), (Role::TreeItem, "notes.md".to_owned()));
    window.key(egui::Key::ArrowUp);
    window.key(egui::Key::ArrowUp);
    assert_eq!(window.focused(), (Role::TreeItem, "docs".to_owned()));
    window.key(egui::Key::ArrowLeft);
    assert_eq!(window.find(Role::TreeItem, "docs").1.is_expanded(), Some(false));
    assert_eq!(
        window.focused(),
        (Role::TreeItem, "docs".to_owned()),
        "closing a folder keeps the keyboard on it"
    );

    let (_, back) = window.find(Role::Button, "Back to projects");
    assert!(!back.is_disabled());
}

#[test]
fn the_explorers_context_menu_is_a_menu_of_named_items_with_their_keys() {
    let mut window = Window::open(&[NOTES, ROADMAP]);
    window.right_click(Role::TreeItem, "notes.md");
    let (menu, _) = window.find(Role::Menu, "notes.md");
    assert_eq!(
        window.controls_under(menu),
        [
            "Add as diagram…",
            "New Folder…",
            "Open as Text",
            "Rename…",
            "Delete",
            "Validate",
            "Validate All"
        ]
        .iter()
        .map(|name| (Role::MenuItem, (*name).to_owned()))
        .filter(|(_, name)| name != "Add as diagram…" || window.has(Role::MenuItem, "Add as diagram…"))
        .collect::<Vec<_>>(),
        "every entry is a menu item, by its label"
    );
    let (_, rename) = window.find(Role::MenuItem, "Rename…");
    assert_eq!(rename.keyboard_shortcut(), Some("F2"));
    let (_, delete) = window.find(Role::MenuItem, "Delete");
    assert_eq!(delete.keyboard_shortcut(), Some("Delete"));

    // A folder's menu offers Add… first.
    window.frame(vec![egui::Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    window.frames(1);
    window.right_click(Role::TreeItem, "docs");
    let (menu, _) = window.find(Role::Menu, "docs");
    assert_eq!(
        window.controls_under(menu).first(),
        Some(&(Role::MenuItem, "Add…".to_owned()))
    );
    let (_, add) = window.find(Role::MenuItem, "Add…");
    assert_eq!(add.keyboard_shortcut(), Some("Insert"));
}

#[test]
fn the_rename_new_folder_and_delete_questions_are_dialogs_named_by_their_title() {
    let mut window = Window::open(&[NOTES, ROADMAP]);
    window.right_click(Role::TreeItem, "notes.md");
    window.click(Role::MenuItem, "Rename…");
    let (dialog, _) = window.find(Role::Dialog, "Rename");
    assert_eq!(
        window.controls_under(dialog),
        vec![
            (Role::TextInput, "New name".to_owned()),
            (Role::Button, "OK".to_owned()),
            (Role::Button, "Cancel".to_owned()),
        ]
    );
    let (_, field) = window.find(Role::TextInput, "New name");
    assert_eq!(
        field.value(),
        Some("notes.md"),
        "the field holds the name it starts from"
    );
    window.click(Role::Button, "Cancel");
    assert!(!window.has(Role::Dialog, "Rename"), "Cancel closes it");

    window.right_click(Role::TreeItem, "notes.md");
    window.click(Role::MenuItem, "New Folder…");
    let (dialog, _) = window.find(Role::Dialog, "New folder");
    assert_eq!(
        window.controls_under(dialog).first(),
        Some(&(Role::TextInput, "Folder name".to_owned()))
    );
    window.click(Role::Button, "Cancel");

    window.right_click(Role::TreeItem, "notes.md");
    window.click(Role::MenuItem, "Delete");
    let (dialog, _) = window.find(Role::Dialog, "Delete");
    let text: Vec<String> = window
        .descendants(dialog)
        .into_iter()
        .filter(|(_, n)| n.role() == Role::Label)
        .map(|(_, n)| window.name(n))
        .collect();
    assert!(
        text.iter()
            .any(|t| t == "Delete 'notes.md' from disk? This cannot be undone."),
        "the question is read out: {text:?}"
    );
    assert_eq!(
        window.controls_under(dialog),
        vec![(Role::Button, "Delete".to_owned()), (Role::Button, "Cancel".to_owned())]
    );
}

#[test]
fn the_add_dialog_is_a_dialog_with_a_tree_of_tool_types_a_name_field_and_a_disabled_confirm() {
    let mut window = Window::open(&[NOTES, ROADMAP]);
    window.right_click(Role::TreeItem, "docs");
    window.click(Role::MenuItem, "Add…");
    let (dialog, _) = window.find(Role::Dialog, "Add diagram");
    let (tree, _) = window.find(Role::Tree, "Add diagram");
    assert!(
        window.descendants(dialog).iter().any(|(id, _)| *id == tree),
        "the tree is the dialog's"
    );
    let vendors: Vec<(NodeId, &Node)> = window
        .descendants(tree)
        .into_iter()
        .filter(|(_, n)| n.role() == Role::TreeItem)
        .collect();
    assert!(vendors.len() > 1, "a group per vendor: {}", window.outline());
    for (_, vendor) in &vendors {
        assert_eq!(vendor.level(), Some(1));
        assert_eq!(vendor.is_expanded(), Some(false), "the vendors read as a list first");
    }
    let (_, name) = window.find(Role::TextInput, "Name");
    assert!(!name.is_disabled());
    let (_, add) = window.find(Role::Button, "Add");
    assert!(add.is_disabled(), "nothing is chosen yet, so Add cannot be pressed");

    // The keyboard is on the tree when the dialog opens, on its first row, and the arrows move it.
    let second = window.name(vendors[1].1);
    let first = window.name(vendors[0].1);
    assert_eq!(window.focused(), (Role::TreeItem, first.clone()));
    window.key(egui::Key::ArrowDown);
    assert_eq!(window.focused(), (Role::TreeItem, second));
    window.key(egui::Key::ArrowUp);
    assert_eq!(window.focused(), (Role::TreeItem, first.clone()));

    window.click(Role::TreeItem, &first);
    let (_, vendor) = window.find(Role::TreeItem, &first);
    assert_eq!(vendor.is_expanded(), Some(true));
    let choices: Vec<&Node> = window
        .descendants(tree)
        .into_iter()
        .map(|(_, n)| n)
        .filter(|n| n.role() == Role::TreeItem && n.level() == Some(2))
        .collect();
    assert!(!choices.is_empty(), "an open vendor shows its tool types");
    for choice in choices {
        assert_eq!(choice.is_selected(), Some(false));
    }
}

#[test]
fn the_tabs_are_a_tab_list_saying_which_one_shows_and_each_closes_by_name() {
    let mut window = Window::open(&[NOTES, ROADMAP]);
    window.click(Role::TreeItem, "notes.md");
    window.click(Role::TreeItem, "docs");
    window.click(Role::TreeItem, "roadmap.md");
    let (list, _) = window.find(Role::TabList, "Open editors");
    assert_eq!(
        window.controls_under(list),
        vec![
            (Role::Tab, "notes.md".to_owned()),
            (Role::Button, "Close notes.md".to_owned()),
            (Role::Tab, "roadmap.md".to_owned()),
            (Role::Button, "Close roadmap.md".to_owned()),
        ]
    );
    assert_eq!(window.find(Role::Tab, "notes.md").1.is_selected(), Some(false));
    assert_eq!(window.find(Role::Tab, "roadmap.md").1.is_selected(), Some(true));

    window.click(Role::Tab, "notes.md");
    assert_eq!(window.find(Role::Tab, "notes.md").1.is_selected(), Some(true));
    window.click(Role::Button, "Close notes.md");
    assert!(!window.has(Role::Tab, "notes.md"), "the tab is closed");
}

#[test]
fn the_errors_panel_is_a_named_list_of_its_findings() {
    let mut window = Window::open(&[NOTES, BROKEN]);
    let (panel, _) = window.find(Role::List, "Problems");
    assert!(window.controls_under(panel).is_empty(), "nothing is validated yet");

    window.right_click(Role::TreeItem, "notes.md");
    window.click(Role::MenuItem, "Validate All");
    let (panel, _) = window.find(Role::List, "Problems");
    let findings = window.controls_under(panel);
    assert!(
        !findings.is_empty(),
        "the broken file has findings: {}",
        window.outline()
    );
    for (role, name) in &findings {
        assert_eq!(*role, Role::ListItem);
        assert!(
            name.starts_with("Error: ") && name.contains("service.json"),
            "a finding is read with its severity and its file: {name}"
        );
    }
}

#[test]
fn a_diagram_canvas_is_one_named_node_beside_its_toolbar_and_its_panes() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../adp-diagram/tests/fixtures/dependency-graph/example-1");
    let services = |name: &str| std::fs::read_to_string(fixture.join(name)).unwrap();
    let (registration, body) = (services("services.adp"), services("services.dgr"));
    let mut window = Window::open(&[("services.adp", &registration), ("services.dgr", &body)]);
    window.click(Role::TreeItem, "services.dgr");
    let (_, canvas) = window.find(Role::Canvas, "services.dgr");
    assert!(
        canvas.description().is_some_and(|d| !d.is_empty()),
        "the canvas says which tool type draws it"
    );
    for button in ["Zoom In", "Zoom Out", "Fit to View", "Undo", "Redo"] {
        window.find(Role::Button, button);
    }
    let (panes, _) = window.find(Role::TabList, "Diagram panes");
    assert_eq!(
        window.controls_under(panes),
        vec![(Role::Tab, "Toolbox".to_owned()), (Role::Tab, "Properties".to_owned())]
    );
    assert_eq!(window.find(Role::Tab, "Toolbox").1.is_selected(), Some(true));
    window.click(Role::Tab, "Properties");
    assert_eq!(window.find(Role::Tab, "Properties").1.is_selected(), Some(true));

    // The canvas's context menu is a menu of named items too.
    window.right_click(Role::Canvas, "services.dgr");
    let menus: Vec<NodeId> = window
        .descendants(window.root)
        .into_iter()
        .filter(|(_, n)| n.role() == Role::Menu)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(menus.len(), 1, "the menu is open: {}", window.outline());
    let entries = window.controls_under(menus[0]);
    assert!(!entries.is_empty(), "{}", window.outline());
    assert!(entries.iter().all(|(role, _)| *role == Role::MenuItem), "{entries:?}");
}

/// The ribbon's export button is a named button that says whether it can run and, when it cannot, why (etalii.adp spec 074,
/// FR-006).
#[test]
fn the_export_button_is_a_named_button_with_its_state() {
    let body = "causal-loop 1\r\nvariable a \"A\"\r\n";
    let mut window = Window::open(&[("loop.adp", "systems/causal-loop-diagram\r\n"), ("loop.cld", body)]);
    let (_, disabled) = window.find(Role::Button, "Export flow page");
    assert!(disabled.is_disabled(), "no diagram is open");
    assert_eq!(
        disabled.description(),
        Some("Open a causal loop diagram to export its flow page.")
    );
    window.click(Role::TreeItem, "loop.cld");
    let (_, enabled) = window.find(Role::Button, "Export flow page");
    assert!(!enabled.is_disabled(), "{}", window.outline());
}

/// The activity bar switches the side bar between the explorer and the search view, as the standalone host's Hierarchy
/// and Search tabs do (P-015, T101); each is a tab that says whether it is the one shown, and the search view says what
/// it is for, as the standalone host's placeholder does.
#[test]
fn the_activity_bar_switches_between_the_explorer_and_the_search_view() {
    let mut window = Window::open(&[NOTES, ROADMAP]);
    assert_eq!(window.find(Role::Tab, "Explorer").1.is_selected(), Some(true));
    assert_eq!(window.find(Role::Tab, "Search").1.is_selected(), Some(false));
    assert!(window.has(Role::Tree, "Explorer"));

    window.click(Role::Tab, "Search");
    assert_eq!(window.find(Role::Tab, "Search").1.is_selected(), Some(true));
    assert!(
        !window.has(Role::Tree, "Explorer"),
        "the explorer gives way to the search view"
    );
    assert!(window.has(Role::Label, adp_shell::shell::SEARCH_DESCRIPTION));

    window.click(Role::Tab, "Explorer");
    assert!(window.has(Role::Tree, "Explorer"));
    assert!(!window.has(Role::Label, adp_shell::shell::SEARCH_DESCRIPTION));
}

/// The standalone host's panes resize (P-009, T102): the side bar with the explorer or the search view, the errors and
/// warnings panel, and the toolbox and properties beside a diagram, each by a drag on its edge, which the tabs between
/// them give way to.
#[test]
fn the_side_bar_the_problems_panel_and_the_diagram_panes_resize() {
    const MAP: (&str, &str) = (
        "ideas.mm",
        "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Ideas\" ID=\"ID_1\">\n<node TEXT=\"One\" ID=\"ID_2\"/>\n</node>\n</map>\n",
    );
    let mut window = Window::open(&[NOTES, MAP]);
    window.click(Role::TreeItem, "ideas.mm");
    window.frames(2);

    // Each edge is found where the window shows a resize cursor, dragged, and found again where it went.
    let row = window.find(Role::TreeItem, "notes.md").1.bounds().unwrap();
    let side_bar = |window: &mut Window| {
        let y = ((row.y0 + row.y1) / 2.0) as f32;
        window.edge(egui::pos2(row.x1 as f32 - 4.0, y), egui::pos2(row.x1 as f32 + 200.0, y))
    };
    let problems = |window: &mut Window| window.edge(egui::pos2(640.0, 790.0), egui::pos2(640.0, 300.0));
    let panes = |window: &mut Window| window.edge(egui::pos2(1275.0, 400.0), egui::pos2(700.0, 400.0));

    // The side bar, by its right edge.
    let edge = side_bar(&mut window);
    window.drag(edge, edge + egui::vec2(80.0, 0.0));
    let moved = side_bar(&mut window);
    assert!(
        moved.x > edge.x + 60.0,
        "the side bar widens: its edge from {edge:?} to {moved:?}"
    );

    // The problems panel, by its top edge.
    let edge = problems(&mut window);
    window.drag(edge, edge - egui::vec2(0.0, 80.0));
    let moved = problems(&mut window);
    assert!(
        moved.y < edge.y - 60.0,
        "the problems panel grows: its edge from {edge:?} to {moved:?}"
    );

    // The toolbox and properties, by their left edge.
    let edge = panes(&mut window);
    window.drag(edge, edge - egui::vec2(80.0, 0.0));
    let moved = panes(&mut window);
    assert!(
        moved.x < edge.x - 60.0,
        "the diagram panes widen: their edge from {edge:?} to {moved:?}"
    );
}

const DESCRIPTION: (&str, &str) = (
    "notes.atd",
    include_str!("../../../vendor/etalii.adp/specifications/atd/notes.atd"),
);

/// A `.atd` file's tab (etalii.adp spec 035, T105): its outline is a tree named Outline of the description's symbols,
/// its suggestions a list box named Suggestions, and its findings are in the errors panel like any editor's.
#[test]
fn an_atd_tab_has_a_named_outline_and_suggestion_list_and_its_findings_in_the_errors_panel() {
    let broken = (
        "broken.atd",
        "atd 0.1\n\ndiagram x/broken \"Broken\" {\n  model {\n    element Node extends Missing {}\n  }\n}\n",
    );
    let mut window = Window::open(&[DESCRIPTION, broken]);
    window.click(Role::TreeItem, "notes.atd");
    // The server answers on its thread; the outline comes with the first colouring.
    for _ in 0..40 {
        if window.has(Role::Tree, "Outline")
            && window
                .all(Role::Tree, "Outline")
                .first()
                .is_some_and(|(id, _)| !window.controls_under(*id).is_empty())
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        window.frames(1);
    }
    let (outline, _) = window.find(Role::Tree, "Outline");
    let symbols = window.controls_under(outline);
    assert!(
        !symbols.is_empty(),
        "the outline lists the symbols: {}",
        window.outline()
    );
    assert!(symbols.iter().all(|(role, _)| *role == Role::Button));

    // Ctrl+Space in the editor opens the suggestions.
    window.frame(vec![
        egui::Event::PointerMoved(egui::pos2(900.0, 300.0)),
        egui::Event::PointerButton {
            pos: egui::pos2(900.0, 300.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        },
    ]);
    window.frame(vec![egui::Event::PointerButton {
        pos: egui::pos2(900.0, 300.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    }]);
    let space = |pressed| egui::Event::Key {
        key: egui::Key::Space,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: egui::Modifiers::CTRL,
    };
    // On a line of its own, where a member of the element can start.
    window.key(egui::Key::Enter);
    window.frame(vec![space(true), space(false)]);
    window.frames(2);
    let (list, _) = window.find(Role::ListBox, "Suggestions");
    assert!(!window.controls_under(list).is_empty(), "{}", window.outline());

    window.right_click(Role::TreeItem, "notes.atd");
    window.click(Role::MenuItem, "Validate All");
    let (panel, _) = window.find(Role::List, "Problems");
    let findings = window.controls_under(panel);
    assert!(
        findings
            .iter()
            .any(|(_, name)| name.contains("broken.atd") && name.contains("ATD")),
        "the broken description's findings: {findings:?}"
    );
}

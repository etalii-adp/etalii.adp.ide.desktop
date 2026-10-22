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
        let TreeUpdate { nodes, tree, .. } = output.platform_output.accesskit_update.expect("AccessKit is on");
        self.root = tree.expect("every frame sends the whole tree").root;
        self.nodes = nodes.into_iter().collect();
        self.assert_controls_are_named();
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

    let first = window.name(vendors[0].1);
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

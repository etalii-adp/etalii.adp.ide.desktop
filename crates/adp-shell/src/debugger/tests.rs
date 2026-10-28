//! The debugger panel's tests (etalii.adp spec 035, T141): the mind map's description opened in the validation view
//! with the debugger showing, drawn headless and read through its accessibility tree. A breakpoint set in the gutter
//! stops the case where the description's function runs, with its call stack and scopes; every control has its role,
//! name and key; a stop ends the run, and nothing of the description or the case is written.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use adp_atd_script::debug::{DebugControl, PauseReason, ScopeKind};
use eframe::egui::{
    self,
    accesskit::{Node, NodeId, Role},
};

use super::RunState;
use crate::theme::{self, Palette};
use crate::validation::ValidationTab;

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// A copy of the mind map's description and its first case in a folder of its own.
fn copied() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    for name in ["mindmap.atd", "mindmap.notation.atd", "mindmap.freeplane.atd"] {
        std::fs::copy(diagrams().join(name), folder.path().join(name)).unwrap();
    }
    let case = folder.path().join("conformance/mindmap/example-1");
    std::fs::create_dir_all(&case).unwrap();
    for file in std::fs::read_dir(diagrams().join("conformance/mindmap/example-1"))
        .unwrap()
        .flatten()
    {
        std::fs::copy(file.path(), case.join(file.file_name())).unwrap();
    }
    let root = folder.path().join("mindmap.atd");
    (folder, root)
}

fn contents(folder: &Path) -> HashMap<PathBuf, Vec<u8>> {
    let mut found = HashMap::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(f) = folders.pop() {
        for entry in std::fs::read_dir(&f).unwrap().flatten() {
            if entry.path().is_dir() {
                folders.push(entry.path());
            } else {
                found.insert(entry.path(), std::fs::read(entry.path()).unwrap());
            }
        }
    }
    found
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.enable_accesskit();
    ctx
}

/// One frame of the view; the accessibility tree it hands the platform.
fn frame(ctx: &egui::Context, tab: &mut ValidationTab) -> HashMap<NodeId, Node> {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))),
        ..Default::default()
    };
    let theme = adp_theme::Theme::dark();
    let palette = Palette::of(&theme);
    let mut output = ctx.run_ui(input, |ui| {
        crate::validation::show(ui, tab, &theme, &palette);
    });
    output.textures_delta.clear();
    output
        .platform_output
        .accesskit_update
        .expect("AccessKit is on")
        .nodes
        .into_iter()
        .collect()
}

/// Frames until the panel pauses, or the run ends.
fn until_paused(ctx: &egui::Context, tab: &mut ValidationTab) -> HashMap<NodeId, Node> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let nodes = frame(ctx, tab);
        if matches!(tab.debugger.state, RunState::Paused { .. }) || !tab.debugger.is_running() {
            return frame(ctx, tab);
        }
        assert!(Instant::now() < deadline, "the case did not pause");
        drop(nodes);
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn nodes_with(nodes: &HashMap<NodeId, Node>, role: Role) -> Vec<&Node> {
    nodes.values().filter(|n| n.role() == role).collect()
}

fn labels(nodes: &HashMap<NodeId, Node>, role: Role) -> Vec<String> {
    nodes_with(nodes, role)
        .into_iter()
        .filter_map(|n| n.label().map(str::to_owned))
        .collect()
}

/// The line of the mind map's description `needle` is on.
fn line_of(root: &Path, needle: &str) -> usize {
    let text = std::fs::read_to_string(root).unwrap();
    text.lines().position(|l| l.contains(needle)).expect("the line") + 1
}

#[test]
fn a_breakpoint_in_the_gutter_stops_the_case_in_the_descriptions_function() {
    let (folder, root) = copied();
    let before = contents(folder.path());
    let mut tab = ValidationTab::open(&root).expect("the mind map has a case");
    tab.debugging = true;
    let ctx = context();
    frame(&ctx, &mut tab);
    let nodes = frame(&ctx, &mut tab);

    // The toolbar: each control a button with its name and key; only Debug Case is enabled before the run.
    let buttons: HashMap<String, &Node> = nodes_with(&nodes, Role::Button)
        .into_iter()
        .filter_map(|n| Some((n.label()?.to_owned(), n)))
        .collect();
    for name in ["Debug Case", "Continue", "Step Over", "Step Into", "Step Out", "Stop"] {
        let button = buttons
            .get(name)
            .unwrap_or_else(|| panic!("no `{name}` in {:?}", buttons.keys()));
        assert!(button.keyboard_shortcut().is_some(), "`{name}` has a key");
    }
    assert!(buttons["Continue"].is_disabled());
    assert!(!buttons["Debug Case"].is_disabled());
    assert!(labels(&nodes, Role::Status).contains(&"Not running".to_owned()));

    // A breakpoint set in the gutter, on the line where siblingName returns.
    let line = line_of(&root, "return unique(name, taken)");
    let file = tab.debugger.files[0].name.clone();
    tab.debugger.toggle(&file, line);
    assert!(tab.debugger.breakpoints[0].verified, "the line has a statement");
    tab.debugger.toggle(&file, line - 1);
    assert!(!tab.debugger.breakpoints[1].verified, "the line before has none");
    tab.debugger.toggle(&file, line - 1);

    tab.debugger.start(&root, &tab.cases[tab.case].1.clone());
    let nodes = until_paused(&ctx, &mut tab);
    // The text scrolled to where it paused, the breakpoint in its gutter.
    assert!(
        labels(&nodes, Role::Button).contains(&format!("Line {line}, breakpoint")),
        "the gutter shows it: {:?}",
        labels(&nodes, Role::Button)
    );
    let RunState::Paused { reason, frames, scopes } = tab.debugger.state.clone() else {
        panic!("not paused: {:?}", tab.debugger.output);
    };
    assert_eq!(reason, PauseReason::Breakpoint);
    assert_eq!(frames[0].name, "siblingName");
    assert_eq!(frames[0].line, line);
    let locals: Vec<&str> = scopes
        .iter()
        .find(|s| s.frame == 0 && s.kind == ScopeKind::Locals)
        .unwrap()
        .variables
        .iter()
        .map(|v| v.name.as_str())
        .collect();
    assert!(locals.contains(&"name") && locals.contains(&"taken"), "{locals:?}");

    // What the panes read.
    let items = labels(&nodes, Role::ListItem);
    assert!(items.iter().any(|i| i.starts_with("siblingName")), "{items:?}");
    assert!(labels(&nodes, Role::TreeItem).iter().any(|i| i.starts_with("names: ")));
    assert!(labels(&nodes, Role::Status).contains(&"Paused on a breakpoint".to_owned()));
    let buttons: HashMap<String, &Node> = nodes_with(&nodes, Role::Button)
        .into_iter()
        .filter_map(|n| Some((n.label()?.to_owned(), n)))
        .collect();
    assert!(!buttons["Continue"].is_disabled());

    // Stop ends the run; nothing was written.
    tab.debugger.control(DebugControl::Stop);
    assert_eq!(tab.debugger.state, RunState::Idle);
    assert!(!tab.debugger.is_running());
    frame(&ctx, &mut tab);
    assert_eq!(contents(folder.path()), before);
}

#[test]
fn continuing_runs_the_case_to_its_end() {
    let (_folder, root) = copied();
    let mut tab = ValidationTab::open(&root).expect("the mind map has a case");
    tab.debugging = true;
    let ctx = context();
    frame(&ctx, &mut tab);
    let line = line_of(&root, "return unique(name, taken)");
    let file = tab.debugger.files[0].name.clone();
    tab.debugger.toggle(&file, line);
    tab.debugger.start(&root, &tab.cases[tab.case].1.clone());
    let mut stops = 0;
    loop {
        until_paused(&ctx, &mut tab);
        if !tab.debugger.is_running() {
            break;
        }
        stops += 1;
        assert!(stops < 100, "the case keeps stopping");
        tab.debugger.control(DebugControl::Continue);
    }
    assert!(stops >= 1, "the case adds children, which names them");
    assert!(
        tab.debugger
            .output
            .iter()
            .any(|o| o.starts_with("The case 'example-1'")),
        "{:?}",
        tab.debugger.output
    );
}

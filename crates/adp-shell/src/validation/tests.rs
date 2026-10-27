//! The validation view's tests (etalii.adp spec 035, T129, T122): the mind map's description opened with its cases,
//! drawn headless and read through its accessibility tree; nothing of the description or the case is written; a save
//! that breaks the description keeps the last result and shows the findings, and one that mends it runs the case
//! again (FR-044).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui::{
    self,
    accesskit::{Node, NodeId, Role},
};

use super::{Place, ValidationTab};
use crate::theme::{self, Palette};

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// A copy of the mind map's description and its cases in a folder of its own.
fn copied() -> (tempfile::TempDir, PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    for name in ["mindmap.atd", "mindmap.notation.atd", "mindmap.freeplane.atd"] {
        std::fs::copy(diagrams().join(name), folder.path().join(name)).unwrap();
    }
    let cases = folder.path().join("conformance/mindmap");
    std::fs::create_dir_all(&cases).unwrap();
    for entry in std::fs::read_dir(diagrams().join("conformance/mindmap"))
        .unwrap()
        .flatten()
    {
        let into = cases.join(entry.file_name());
        std::fs::create_dir_all(&into).unwrap();
        for file in std::fs::read_dir(entry.path()).unwrap().flatten() {
            std::fs::copy(file.path(), into.join(file.file_name())).unwrap();
        }
    }
    let root = folder.path().join("mindmap.atd");
    (folder, root)
}

/// Every file under `folder` with its bytes.
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

/// One frame of the view, 1400 points wide; the accessibility tree it hands the platform, and where it asked to go.
fn frame(ctx: &egui::Context, tab: &mut ValidationTab) -> (HashMap<NodeId, Node>, Option<Place>) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))),
        ..Default::default()
    };
    let theme = adp_theme::Theme::dark();
    let palette = Palette::of(&theme);
    let mut go_to = None;
    let mut output = ctx.run_ui(input, |ui| go_to = super::show(ui, tab, &theme, &palette));
    output.textures_delta.clear();
    let nodes = output
        .platform_output
        .accesskit_update
        .expect("AccessKit is on")
        .nodes
        .into_iter()
        .collect();
    (nodes, go_to)
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    theme::install(&ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.enable_accesskit();
    ctx
}

fn labels(nodes: &HashMap<NodeId, Node>, role: Role) -> Vec<String> {
    nodes
        .values()
        .filter(|n| n.role() == role)
        .filter_map(|n| n.label().map(str::to_owned))
        .collect()
}

#[test]
fn the_view_lists_the_cases_and_each_difference_at_its_construct() {
    let (_folder, root) = copied();

    let mut tab = ValidationTab::open(&root).expect("the mind map has cases");

    let names: Vec<&str> = tab.cases.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"example-1") && names.contains(&"fold"), "{names:?}");
    let result = tab.result.clone().expect("the first case ran");
    assert!(
        !result.differences.is_empty(),
        "the mind map's cases still differ where known"
    );
    let ctx = context();
    frame(&ctx, &mut tab);
    let (nodes, _) = frame(&ctx, &mut tab);
    let items = labels(&nodes, Role::ListItem);
    let first = &result.differences[0];
    let at = first.at.as_ref().expect("a difference is attributed");
    assert!(
        items.iter().any(|i| i.contains(&format!("at {}:{}", at.file, at.line))),
        "{items:?}"
    );
    assert!(labels(&nodes, Role::ComboBox).iter().any(|c| c.starts_with("Case: ")));
    assert!(labels(&nodes, Role::Tab).contains(&"Toolbox".to_owned()));
}

#[test]
fn going_to_a_difference_names_its_file_and_line() {
    let (_folder, root) = copied();
    let tab = ValidationTab::open(&root).unwrap();
    let result = tab.result.clone().unwrap();
    let at = result.differences.iter().find_map(|d| d.at.clone()).unwrap();

    let place = tab.place(&at.file, at.line);

    assert_eq!(place.line, at.line);
    assert!(place.file.ends_with(&at.file), "{place:?}");
    assert!(place.file.exists(), "{place:?}");
}

#[test]
fn validating_writes_no_file_of_the_description_or_its_cases() {
    let (folder, root) = copied();
    let before = contents(folder.path());

    let mut tab = ValidationTab::open(&root).unwrap();
    for case in 0..tab.cases.len() {
        tab.choose(case);
    }
    let ctx = context();
    frame(&ctx, &mut tab);

    assert_eq!(contents(folder.path()), before);
}

#[test]
fn a_save_that_breaks_the_description_keeps_the_last_result_and_one_that_mends_it_runs_again() {
    let (_folder, root) = copied();
    let mut tab = ValidationTab::open(&root).unwrap();
    let kept = tab.result.clone().unwrap();
    let runs = tab.runs;
    let text = std::fs::read_to_string(&root).unwrap();

    // A save that does not check: the findings show, the last result stays, nothing runs.
    std::fs::write(&root, text.replacen("model {", "model {{", 1)).unwrap();
    tab.changed_on_disk(&root);
    assert!(!tab.findings.is_empty());
    assert_eq!(tab.result.as_ref().map(|r| &r.differences), Some(&kept.differences));
    assert_eq!(tab.runs, runs);
    let ctx = context();
    frame(&ctx, &mut tab);
    let (nodes, _) = frame(&ctx, &mut tab);
    assert!(
        labels(&nodes, Role::ListItem).iter().any(|i| i.contains("ATD1")),
        "the syntax error is listed"
    );

    // A save that checks clean again: the case runs again, with the findings gone.
    std::fs::write(&root, &text).unwrap();
    tab.changed_on_disk(&root);
    assert!(tab.findings.is_empty());
    assert_eq!(tab.runs, runs + 1);
    assert_eq!(tab.result.as_ref().map(|r| &r.differences), Some(&kept.differences));
}

#[test]
fn a_description_without_cases_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().join("mindmap.atd");
    std::fs::copy(diagrams().join("mindmap.atd"), &root).unwrap();

    let refused = ValidationTab::open(&root).err().expect("refused");

    assert!(refused.contains("no conformance cases"), "{refused}");
}

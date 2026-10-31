//! Scene snapshots of the examples of the first three diagram types (etalii.adp spec 030, T053): the mind map
//! (`freeplane/mindmap`), the timeline (`generic/timeline`) and the C4 container view (`c4/container`), each the
//! example its parity script edits (`parity/fixtures/`). Each is opened through its registration as the shell opens
//! it, and two scenes are kept for it:
//!
//! - what its session delivers for a viewport (contracts/core-shell.md, `SceneChanged`): every element in view with
//!   its type, its position, its parent or ends and its attributes, once for the whole diagram and once for a
//!   viewport over the left half of it (the left tenth for the C4 view, whose module also delivers what a line from
//!   an element in view reaches), which leaves out the nodes outside and the relations not both of whose ends are in;
//! - for a type its notation draws, the primitives the notation draws it as, each element at the position its session
//!   delivers in its type's default size, measured with the standalone host's text estimate.
//!
//! A change to a binding, a specification, a layout or the notation that moves, adds or drops anything shows as a
//! change to a snapshot under `snapshots/`, to be reviewed with `cargo insta review`.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramElement, DiagramSession, DiagramViewport, ElementKind};
use adp_disl::{DislEnv, DislPluginFunctions};
use adp_scene::definition::compartments::Folding;
use adp_scene::scene::{Primitive, StandardRoutePlugins, default_size};
use adp_scene::{EstimatedMetrics, Evaluator, Rect, SceneOptions, Theme, build_scene, compile_notation};

const REPOSITORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// The three types as the shell composes them.
fn tools() -> DiagramTools {
    let tools = adp_diagram_mindmap::register(DiagramTools::new(
        Catalogue::load(&Path::new(REPOSITORY).join("vendor")),
        |_| DislPluginFunctions::none(),
    ))
    .with_module(adp_diagram_timeline::ORIGIN, adp_diagram_timeline::module());
    adp_diagram_c4::register(tools)
}

fn copy_folder(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).expect("the example's folder reads").flatten() {
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            fs::create_dir_all(&target).expect("the folder is made");
            copy_folder(&path, &target);
        } else {
            fs::copy(&path, &target).expect("the file is copied");
        }
    }
}

/// A number as a scene keeps it: to two decimals, without a negative zero.
fn n(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    format!("{}", if rounded == 0.0 { 0.0 } else { rounded })
}

fn rect(r: Rect) -> String {
    format!("({}, {}, {} x {})", n(r.x), n(r.y), n(r.width), n(r.height))
}

/// The elements a session delivers, one per line, in the order delivered.
fn delivered(elements: &[DiagramElement]) -> String {
    let mut out = String::new();
    for e in elements {
        match e.kind {
            ElementKind::Node => {
                let placed = if e.placed { "placed" } else { "laid out" };
                let _ = write!(
                    out,
                    "node {} {} at ({}, {}) {placed}",
                    e.element_type,
                    e.id,
                    n(e.x),
                    n(e.y)
                );
                if let Some(parent) = &e.parent {
                    let _ = write!(out, " in {parent}");
                }
            }
            ElementKind::Relation => {
                let _ = write!(
                    out,
                    "relation {} {} from {} to {}",
                    e.element_type,
                    e.id,
                    e.source.as_deref().unwrap_or("-"),
                    e.target.as_deref().unwrap_or("-")
                );
            }
        }
        out.push('\n');
        for (name, value) in e.attributes.iter().filter(|(_, v)| !v.is_empty()) {
            let _ = writeln!(out, "    {name}: {}", value.replace('\n', "\\n"));
        }
    }
    out
}

/// A viewport over the left `part` of what `elements` places, top to bottom.
fn left_part(elements: &[DiagramElement], part: f64) -> DiagramViewport {
    let nodes = || elements.iter().filter(|e| e.kind == ElementKind::Node);
    let min_x = nodes().map(|e| e.x).fold(f64::INFINITY, f64::min);
    let max_x = nodes().map(|e| e.x).fold(f64::NEG_INFINITY, f64::max);
    DiagramViewport {
        min_x,
        min_y: f64::NEG_INFINITY,
        max_x: min_x + (max_x - min_x) * part,
        max_y: f64::INFINITY,
    }
}

/// The primitives of the scene the notation of the document's tool type draws, one per line.
fn drawn(workspace: &DiagramWorkspace, body: &Path, elements: &[DiagramElement]) -> String {
    let document = workspace.documents().get(body).expect("the document is open");
    let specification = document.tool().definition.specification();
    let definition = compile_notation(specification).expect("the notation compiles");
    let evaluator = Evaluator::new(
        &definition.specification,
        Theme::of(&definition.specification, None),
        DislEnv::default(),
    );
    let positions: HashMap<String, Rect> = elements
        .iter()
        .filter(|e| e.kind == ElementKind::Node)
        .filter_map(|e| {
            let (width, height) = default_size(definition.element_type(&e.element_type)?);
            Some((e.id.clone(), Rect::new(e.x, e.y, width, height)))
        })
        .collect();
    let folding = Folding::default();
    let options = SceneOptions {
        selection: &[],
        folding: &folding,
        metrics: &EstimatedMetrics,
        plugins: &StandardRoutePlugins,
        shown: None,
    };
    let scene = build_scene(&definition, &evaluator, &document.model().diagram, &positions, &options);
    let mut out = String::new();
    for primitive in &scene.primitives {
        let bounds = primitive
            .bounds(&EstimatedMetrics)
            .map(rect)
            .unwrap_or_else(|| "-".to_owned());
        let _ = match primitive {
            Primitive::Shape(s) => writeln!(out, "shape {:?} {:?} {bounds}", s.owner, s.role),
            Primitive::Text(t) => writeln!(
                out,
                "text {:?} {:?} {:?} at ({}, {}) {:?}",
                t.owner,
                t.role,
                t.text,
                n(t.position.x),
                n(t.position.y),
                t.anchor
            ),
            Primitive::Edge(e) => writeln!(
                out,
                "edge {:?} {bounds} {:?} -> {:?}",
                e.owner,
                e.start.as_ref().map(|m| m.kind),
                e.end.as_ref().map(|m| m.kind)
            ),
            Primitive::Hook(h) => writeln!(out, "hook {:?} {:?} {bounds}", h.owner, h.kind),
        };
    }
    if let Some(empty) = &scene.empty_text {
        let _ = writeln!(out, "empty: {empty}");
    }
    out
}

/// The example under `parity/fixtures/`, opened in a copy of its folder through `registration`.
fn opened(example: &str, registration: &str) -> (tempfile::TempDir, DiagramWorkspace, DiagramSession, PathBuf) {
    let folder = tempfile::tempdir().expect("a temporary folder");
    copy_folder(
        &Path::new(REPOSITORY).join("parity/fixtures").join(example),
        folder.path(),
    );
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, mut session) = workspace
        .open(&mut tools, Path::new(registration))
        .unwrap_or_else(|reason| panic!("{example}/{registration} opens: {reason}"));
    session
        .baseline(workspace.documents())
        .expect("the baseline is delivered");
    (folder, workspace, session, opened.body_path)
}

/// The session's scenes of the example, for the whole diagram and for the left `part` of it.
fn viewport_scenes(
    workspace: &DiagramWorkspace,
    session: &mut DiagramSession,
    part: f64,
) -> (Vec<DiagramElement>, String, String) {
    let whole = session.render(workspace.documents()).expect("the diagram renders");
    assert!(!whole.is_empty(), "the example draws something");
    session
        .update_view(workspace.documents(), left_part(&whole, part))
        .expect("the viewport is delivered");
    let half = session.render(workspace.documents()).expect("the viewport renders");
    assert!(
        half.len() < whole.len(),
        "the left part leaves something out: {}",
        delivered(&half)
    );
    (whole.clone(), delivered(&whole), delivered(&half))
}

#[test]
fn the_mind_map_example() {
    let (_folder, workspace, mut session, body) = opened("freeplane/mindmap/example-1", "mindmap.adp");
    let (whole, all, half) = viewport_scenes(&workspace, &mut session, 0.5);
    insta::assert_snapshot!("mindmap_delivered", all);
    insta::assert_snapshot!("mindmap_delivered_left_half", half);
    insta::assert_snapshot!("mindmap_drawn", drawn(&workspace, &body, &whole));
}

#[test]
fn the_timeline_example() {
    let (_folder, workspace, mut session, body) = opened("generic/timeline/roadmap", "roadmap.adp");
    let (whole, all, half) = viewport_scenes(&workspace, &mut session, 0.5);
    insta::assert_snapshot!("timeline_delivered", all);
    insta::assert_snapshot!("timeline_delivered_left_half", half);
    insta::assert_snapshot!("timeline_drawn", drawn(&workspace, &body, &whole));
}

/// The C4 module draws its views itself, so its scene is what its session delivers.
#[test]
fn the_c4_container_view_example() {
    let (_folder, workspace, mut session, _body) = opened("c4/container/courier", "courier.containers.adp");
    assert!(session.module_view().is_some(), "the C4 module draws its views itself");
    let (_, all, half) = viewport_scenes(&workspace, &mut session, 0.1);
    insta::assert_snapshot!("c4_container_delivered", all);
    insta::assert_snapshot!("c4_container_delivered_left_half", half);
}

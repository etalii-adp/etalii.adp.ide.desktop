//! The module's whole integration surface (etalii.adp spec 030, task T070), after the standalone host's
//! `AddWardleyMap.Tests.cs` at `8b6ccfc`: what was eight seams registered by `AddWardleyMap` is one module registered
//! for `wardley/map`, which opens its documents itself, answers its persistence plugin's name, and keeps one reading
//! of a map for every view of it.

mod common;

use std::fs;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::{DiagramModule, DiagramTools, DiagramWorkspace};
use adp_diagram_wardley_map::{EXTENSIONS, ORIGIN, PLUGIN_ID, WardleyModule, register};
use adp_disl::DislPluginFunctions;
use common::vendor;

fn tools() -> DiagramTools {
    register(DiagramTools::new(Catalogue::load(&vendor()), |_| {
        DislPluginFunctions::none()
    }))
}

#[test]
fn the_module_opens_its_documents_itself_for_its_origin() {
    let tools = tools();
    assert!(tools.opened_by_module(ORIGIN));
    let module = WardleyModule::new();
    assert!(module.opens_documents());
    assert_eq!(module.extensions(), EXTENSIONS);
    assert_eq!(EXTENSIONS, [".owm", ".wm"]);
}

#[test]
fn the_specification_names_this_module_as_its_persistence_format() {
    let specification = fs::read_to_string(vendor().join("etalii.adp/definitions/diagrams/wardley-map.dis")).unwrap();
    assert!(specification.contains(&format!("\"format\": \"plugin:{PLUGIN_ID}\"")));
}

#[test]
fn a_registration_opens_the_owm_beside_it() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("tea.adp"), "wardley/map\r\n").unwrap();
    fs::write(folder.path().join("tea.owm"), "title Tea\ncomponent Cup [0.7, 0.6]\n").unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, session) = workspace.open(&mut tools, Path::new("tea.adp")).unwrap();
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.body_path, folder.path().join("tea.owm"));
    assert_eq!(opened.registration_path, Some(folder.path().join("tea.adp")));
    assert!(session.module_view().is_some());
}

#[test]
fn a_registration_follows_its_body_header_within_the_project() {
    let folder = tempfile::tempdir().unwrap();
    fs::create_dir(folder.path().join("maps")).unwrap();
    fs::write(
        folder.path().join("strategy.adp"),
        "wardley/map\r\nbody: maps/strategy.wm\r\n",
    )
    .unwrap();
    fs::write(folder.path().join("maps/strategy.wm"), "title Strategy\n").unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, _) = workspace.open(&mut tools, Path::new("strategy.adp")).unwrap();
    assert_eq!(
        opened.body_path.canonicalize().unwrap(),
        folder.path().join("maps/strategy.wm").canonicalize().unwrap()
    );

    fs::write(
        folder.path().join("escape.adp"),
        "wardley/map\r\nbody: ../outside.owm\r\n",
    )
    .unwrap();
    let Err(refusal) = workspace.open(&mut tools, Path::new("escape.adp")) else {
        panic!("a body outside the project was followed");
    };
    assert!(refusal.contains("outside the project"), "{refusal}");
}

#[test]
fn a_registration_whose_body_is_missing_opens_an_empty_map_written_on_its_first_edit() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("new.adp"), "wardley/map\r\n").unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (opened, session) = workspace.open(&mut tools, Path::new("new.adp")).unwrap();
    let elements = session.render(workspace.documents()).unwrap();
    assert_eq!(elements.len(), 1, "the axes alone");
    let prompt = session.execute(
        &mut workspace,
        &adp_diagram::ActionTarget::Canvas { x: 0.5, y: 0.5 },
        adp_diagram_wardley_map::actions::ADD_ANCHOR,
        &adp_disl::behavior::ids::fixed(&[]),
    );
    assert!(matches!(prompt, adp_diagram::ActionOutcome::Prompt(_)));
    let answered = session.answer(
        &mut workspace,
        &adp_diagram::ActionTarget::Canvas { x: 0.5, y: 0.5 },
        adp_diagram_wardley_map::actions::ADD_ANCHOR,
        "Customer",
    );
    assert_eq!(answered, adp_diagram::ActionOutcome::Done);
    assert_eq!(
        fs::read_to_string(&opened.body_path).unwrap(),
        "anchor Customer [0.5, 0.5]\r\n"
    );
}

#[test]
fn one_reading_serves_every_view_of_a_map() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("map.owm"), "component Alpha [0.5, 0.5]\n").unwrap();
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, first) = workspace.open(&mut tools, Path::new("map.owm")).unwrap();
    let (_, second) = workspace.open(&mut tools, Path::new("map.owm")).unwrap();
    let alpha = first.render(workspace.documents()).unwrap()[1].id.clone();
    first.move_element_to(&mut workspace, &alpha, 0.2, 0.3).unwrap();
    let seen = second
        .render(workspace.documents())
        .unwrap()
        .into_iter()
        .find(|e| e.id == alpha)
        .unwrap();
    assert!((seen.x - 0.2).abs() < 1e-9 && (seen.y - 0.3).abs() < 1e-9, "{seen:?}");
}

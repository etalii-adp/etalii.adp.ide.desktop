//! The seven C4 notations the module declares (etalii.adp spec 030, task T061): their titles, which of them keep their
//! model in a `.dsl` beside their registration, and their shared extension.
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/Diagram.Tests.cs` at
//! `8b6ccfc`, whose `Diagram.Definitions` are here the module's `toolbox::TYPES` with the catalogue's display names
//! and the extensions the module claims for each origin. Not ported: `EveryTypeDescribesItself_WithoutRestatingItsTitle`,
//! because the desktop catalogue (`crates/adp-core/catalogue/tools.json`) carries no description of a tool type.

use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::DiagramTools;
use adp_diagram_c4::toolbox;
use adp_disl::DislPluginFunctions;

const EVERY_TYPE: [(&str, &str, bool); 7] = [
    ("c4/context", "C4 system context diagram", true),
    ("c4/container", "C4 container diagram", true),
    ("c4/component", "C4 component diagram", true),
    // The one type with no document of its own, which never competes for a bare `.dsl`.
    ("c4/code", "C4 code diagram", false),
    ("c4/system-landscape", "C4 system landscape diagram", true),
    ("c4/dynamic", "C4 dynamic diagram", true),
    ("c4/deployment", "C4 deployment diagram", true),
];

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

/// The extension the module claims for `origin`, or empty when it claims none.
fn extension_of(tools: &DiagramTools, origin: &str) -> String {
    tools.module(origin).unwrap().extensions().join(",")
}

#[test]
fn every_cataloged_type_is_declared_with_its_title_and_its_bodys_home() {
    let tools = tools();
    for (key, title, has_sibling) in EVERY_TYPE {
        assert!(toolbox::TYPES.iter().any(|(origin, _)| *origin == key), "{key}");
        assert_eq!(tools.catalogue().by_origin(key).unwrap().display_name, title);
        assert_eq!(toolbox::view_kind_of(key).is_some(), has_sibling, "{key}");
        assert_eq!(
            extension_of(&tools, key),
            if has_sibling { ".dsl" } else { "" },
            "{key}"
        );
    }
}

#[test]
fn the_seven_types_are_the_whole_of_what_this_module_declares() {
    let mut expected: Vec<&str> = EVERY_TYPE.iter().map(|(key, _, _)| *key).collect();
    let mut declared: Vec<&str> = toolbox::TYPES.iter().map(|(origin, _)| *origin).collect();
    expected.sort();
    declared.sort();
    assert_eq!(expected, declared);
}

#[test]
fn every_type_is_vendored_to_c4_with_no_subtype() {
    for (origin, _) in toolbox::TYPES {
        let (vendor, name) = origin.split_once('/').unwrap();
        assert_eq!(vendor, "c4");
        assert!(!name.contains('/') && !name.contains('+'), "{origin}");
    }
}

#[test]
fn the_six_types_with_a_canvas_all_share_the_one_document_extension() {
    let tools = tools();
    let with_bodies: Vec<&str> = toolbox::TYPES
        .iter()
        .filter(|(_, kind)| kind.is_some())
        .map(|(origin, _)| *origin)
        .collect();
    assert_eq!(with_bodies.len(), 6);
    for origin in with_bodies {
        assert_eq!(extension_of(&tools, origin), adp_diagram_c4::EXTENSION);
    }
    let mut claiming = tools.for_file("model.dsl");
    claiming.sort();
    assert!(!claiming.contains(&"c4/code".to_owned()), "{claiming:?}");
}

//! The `.owm` writer's splices (etalii.adp spec 030, task T070), ported from the standalone host's
//! `WardleyWriter.Tests.cs` at `8b6ccfc`.

mod common;

use adp_diagram_wardley_map::lines::LineDocument;
use adp_diagram_wardley_map::model::{WardleyCoordinate, WardleyMap};
use adp_diagram_wardley_map::{parser, writer};
use common::{corpus, fixture};

/// The document to splice and the map that says where.
fn open(text: &str) -> (LineDocument, WardleyMap) {
    let document = LineDocument::parse(text);
    let map = parser::parse(&document);
    (document, map)
}

/// The lines that differ between two texts, 1-based.
fn changed_lines(before: &str, after: &str) -> Vec<usize> {
    let left = LineDocument::parse(before);
    let right = LineDocument::parse(after);
    (0..left.len().max(right.len()))
        .filter(|i| left.lines().get(*i) != right.lines().get(*i))
        .map(|i| i + 1)
        .collect()
}

#[test]
fn set_position_changes_exactly_one_line_in_a_map_full_of_comments() {
    let before = fixture("comments-everywhere.owm");
    let (mut document, map) = open(&before);
    let component = map.components.iter().find(|c| c.name == "Cup of Tea").unwrap();
    assert!(writer::set_position(
        &mut document,
        component,
        WardleyCoordinate::new(0.55, 0.42)
    ));
    assert_eq!(changed_lines(&before, &document.text()), [component.line as usize]);
}

#[test]
fn set_position_keeps_the_trailing_comment_on_the_line_it_edits() {
    let (mut document, map) = open("component Cup of Tea [0.79, 0.61] // why it sits here\n");
    writer::set_position(&mut document, &map.components[0], WardleyCoordinate::new(0.55, 0.42));
    assert_eq!(
        document.text(),
        "component Cup of Tea [0.55, 0.42] // why it sits here\n"
    );
}

#[test]
fn set_position_keeps_decorators_and_label_offsets() {
    let (mut document, map) = open("component Payment [0.70, 0.72] (buy) inertia label [-27, 20]\n");
    writer::set_position(&mut document, &map.components[0], WardleyCoordinate::new(0.5, 0.5));
    assert_eq!(
        document.text(),
        "component Payment [0.5, 0.5] (buy) inertia label [-27, 20]\n"
    );
}

#[test]
fn set_position_keeps_the_authors_spacing_between_the_two_numbers() {
    let (mut document, map) = open("component Alpha [0.80,0.40]\n");
    writer::set_position(&mut document, &map.components[0], WardleyCoordinate::new(0.1, 0.2));
    assert_eq!(document.text(), "component Alpha [0.1,0.2]\n");
}

#[test]
fn set_position_keeps_the_documents_line_endings() {
    let before = fixture("crlf-line-endings.owm");
    let (mut document, map) = open(&before);
    writer::set_position(&mut document, &map.components[0], WardleyCoordinate::new(0.5, 0.5));
    let after = document.text();
    assert!(after.contains("\r\n"));
    assert_eq!(changed_lines(&before, &after).len(), 1);
}

#[test]
fn set_position_writes_numbers_without_a_trailing_zero() {
    let (mut document, map) = open("component Alpha [0.80, 0.40]\n");
    writer::set_position(&mut document, &map.components[0], WardleyCoordinate::new(1.0, 0.0));
    assert_eq!(document.text(), "component Alpha [1, 0]\n");
}

#[test]
fn set_pipeline_child_maturity_changes_the_childs_single_number() {
    let before = fixture("pipelines-both-forms.owm");
    let (mut document, map) = open(&before);
    let child = map
        .pipelines
        .iter()
        .flat_map(|p| &p.children)
        .find(|c| c.name == "Electric Kettle")
        .unwrap();
    assert!(writer::set_pipeline_child_maturity(&mut document, child, 0.9));
    assert_eq!(changed_lines(&before, &document.text()), [child.line as usize]);
    assert!(document.text().contains("component Electric Kettle [0.9]"));
}

#[test]
fn set_pipeline_extent_rewrites_the_legacy_forms_own_coordinates() {
    let (mut document, map) = open("pipeline Power [0.30, 0.85]\n");
    assert!(writer::set_pipeline_extent(
        &mut document,
        &map.pipelines[0],
        WardleyCoordinate::new(0.2, 0.9)
    ));
    assert_eq!(document.text(), "pipeline Power [0.2, 0.9]\n");
}

#[test]
fn set_pipeline_extent_refuses_the_nested_form_which_has_no_coordinates_of_its_own() {
    let (mut document, map) = open("component Kettle [0.4, 0.4]\npipeline Kettle\n{\n}\n");
    let before = document.text();
    assert!(!writer::set_pipeline_extent(
        &mut document,
        &map.pipelines[0],
        WardleyCoordinate::new(0.2, 0.9)
    ));
    assert_eq!(document.text(), before);
}

#[test]
fn rename_rewrites_the_declaration_and_every_reference_in_one_pass() {
    let (mut document, map) = open(
        "component Kettle [0.43, 0.35]\ncomponent Power [0.10, 0.70]\nCup of Tea->Kettle\nKettle->Power\nevolve Kettle 0.62\npipeline Kettle\n{\n  component Electric Kettle [0.63]\n}",
    );
    assert_eq!(writer::rename(&mut document, &map, "Kettle", "Boiler"), 5);
    let after = document.text();
    for expected in [
        "component Boiler [0.43, 0.35]",
        "Cup of Tea->Boiler",
        "Boiler->Power",
        "evolve Boiler 0.62",
        "pipeline Boiler",
    ] {
        assert!(after.contains(expected), "{expected} in {after}");
    }
}

#[test]
fn rename_does_not_touch_a_name_that_merely_contains_the_old_one() {
    let (mut document, map) = open("component Tea [0.63, 0.81]\ncomponent Cup of Tea [0.79, 0.61]\nCup of Tea->Tea");
    writer::rename(&mut document, &map, "Tea", "Leaves");
    let after = document.text();
    assert!(after.contains("component Leaves [0.63, 0.81]"));
    assert!(after.contains("component Cup of Tea [0.79, 0.61]"));
    assert!(after.contains("Cup of Tea->Leaves"));
}

#[test]
fn rename_keeps_a_links_context_text() {
    let (mut document, map) = open("Order Portal->Partner Plugins; via the plugin SDK\n");
    writer::rename(&mut document, &map, "Partner Plugins", "Marketplace");
    assert_eq!(document.text(), "Order Portal->Marketplace; via the plugin SDK\n");
}

#[test]
fn rename_handles_both_ends_of_a_link_to_itself() {
    let (mut document, map) = open("Alpha->Alpha\n");
    writer::rename(&mut document, &map, "Alpha", "Beta");
    assert_eq!(document.text(), "Beta->Beta\n");
}

#[test]
fn rename_leaves_the_document_alone_when_the_name_is_unchanged() {
    let before = fixture("tea-shop.owm");
    let (mut document, map) = open(&before);
    assert_eq!(writer::rename(&mut document, &map, "Kettle", "Kettle"), 0);
    assert_eq!(document.text(), before);
}

#[test]
fn rename_touches_no_line_that_does_not_mention_the_name() {
    let before = fixture("tea-shop.owm");
    let (mut document, map) = open(&before);
    writer::rename(&mut document, &map, "Kettle", "Boiler");
    assert_eq!(changed_lines(&before, &document.text()).len(), 3);
}

#[test]
fn rename_keeps_the_evolve_rename_target() {
    let (mut document, map) = open("evolve Datacentre->Cloud Hosting 0.83\n");
    writer::rename(&mut document, &map, "Datacentre", "Server Room");
    assert_eq!(document.text(), "evolve Server Room->Cloud Hosting 0.83\n");
}

#[test]
fn every_fixture_round_trips_unchanged_when_nothing_is_edited() {
    let names = corpus();
    assert!(names.len() >= 5, "only {} fixtures were found", names.len());
    for name in names {
        let text = fixture(&name);
        let (document, _) = open(&text);
        assert_eq!(document.text(), text, "{name}");
    }
}

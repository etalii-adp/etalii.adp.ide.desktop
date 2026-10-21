//! A parsed map as the elements the shell draws (etalii.adp spec 030, task T070), ported from the standalone host's
//! `WardleyElementMapper.Tests.cs` at `8b6ccfc`. What the standalone host carried in a protobuf payload is carried
//! here as the element's attributes, read by the same names.

mod common;

use adp_diagram::element::diff;
use adp_diagram::{DiagramDelta, DiagramElement, DiagramViewport, ElementKind};
use adp_diagram_wardley_map::axis;
use adp_diagram_wardley_map::identities::{self, ShortGuids, WardleyIdentityEntry};
use adp_diagram_wardley_map::mapper::{self, ANNOTATION, ELEMENT, EVOLUTION_AXIS, LINK};
use adp_diagram_wardley_map::model::WardleyMap;
use common::{corpus, parse_fixture, parse_text};

fn reconcile(map: &WardleyMap, recorded: &[WardleyIdentityEntry]) -> Vec<WardleyIdentityEntry> {
    identities::reconcile(map, recorded, &ShortGuids)
}

fn elements(map: &WardleyMap) -> Vec<DiagramElement> {
    mapper::elements(map, &reconcile(map, &[]))
}

fn named<'a>(elements: &'a [DiagramElement], name: &str) -> &'a DiagramElement {
    elements
        .iter()
        .find(|e| e.element_type == ELEMENT && e.attribute("name") == name)
        .unwrap_or_else(|| panic!("no element '{name}'"))
}

#[test]
fn elements_puts_a_component_where_the_document_said_through_the_axis_and_no_layout() {
    let all = elements(&parse_text("component Alpha [0.90, 0.10]\n"));
    let element = named(&all, "Alpha");
    assert_eq!(element.x, 0.10);
    assert!((element.y - 0.10).abs() < 1e-10);
    assert!(element.placed);
}

#[test]
fn elements_carries_the_documents_own_numbers_alongside_the_canvas_position() {
    let all = elements(&parse_text("component Alpha [0.90, 0.10]\n"));
    let element = named(&all, "Alpha");
    assert_eq!(element.attribute("visibility"), "0.9");
    assert_eq!(element.attribute("maturity"), "0.1");
}

#[test]
fn elements_resolves_the_evolution_stage_backend_side() {
    let all = elements(&parse_text("component Alpha [0.5, 0.75]\n"));
    assert_eq!(named(&all, "Alpha").attribute("stage"), "Commodity (+utility)");
}

#[test]
fn elements_carries_all_five_decorators_as_a_set() {
    let all = elements(&parse_text("component Payment [0.7, 0.72] (buy) (market)\n"));
    assert_eq!(named(&all, "Payment").attribute("decorators"), "buy, market");
}

#[test]
fn elements_attaches_an_evolve_to_the_component_it_names_with_both_stages() {
    let all = elements(&parse_text(
        "component Datacentre [0.15, 0.55]\nevolve Datacentre->Cloud Hosting 0.83\n",
    ));
    let element = named(&all, "Datacentre");
    assert_eq!(element.attribute("evolveMaturity"), "0.83");
    assert_eq!(element.attribute("evolveName"), "Cloud Hosting");
    assert_eq!(element.attribute("evolveStage"), "Commodity (+utility)");
    assert_eq!(element.attribute("stage"), "Product (+rental)");
}

#[test]
fn elements_resolves_link_endpoints_to_ids() {
    let all = elements(&parse_text("component A [0.9, 0.1]\ncomponent B [0.5, 0.5]\nA->B\n"));
    let link = all.iter().find(|e| e.element_type == LINK).unwrap();
    assert_eq!(link.kind, ElementKind::Relation);
    assert_eq!(link.source.as_deref(), Some(named(&all, "A").id.as_str()));
    assert_eq!(link.target.as_deref(), Some(named(&all, "B").id.as_str()));
}

#[test]
fn elements_still_sends_a_link_whose_endpoint_does_not_exist_carrying_the_name_as_written() {
    let all = elements(&parse_text("component A [0.9, 0.1]\nA->Nowhere\n"));
    let link = all.iter().find(|e| e.element_type == LINK).unwrap();
    assert_eq!(link.target, None);
    assert_eq!(link.attribute("targetName"), "Nowhere");
}

#[test]
fn elements_gives_a_pipeline_child_its_parents_visibility() {
    let all = elements(&parse_text(
        "component Kettle [0.43, 0.35]\npipeline Kettle\n{\n  component Electric [0.63]\n}",
    ));
    let child = named(&all, "Electric");
    assert_eq!(child.attribute("visibility"), "0.43");
    assert_eq!(child.attribute("maturity"), "0.63");
    assert_eq!(child.attribute("pipelineParentId"), named(&all, "Kettle").id);
}

#[test]
fn elements_sends_every_occurrence_of_a_multi_position_annotation() {
    let all = elements(&parse_text("annotation 1 [[0.43,0.49],[0.08,0.79]] Two places\n"));
    let annotation = all.iter().find(|e| e.element_type == ANNOTATION).unwrap();
    assert_eq!(annotation.x, 0.49);
    let occurrences: Vec<&str> = annotation.attribute("occurrences").split(';').collect();
    assert_eq!(occurrences.len(), 2);
    assert!(occurrences[0].starts_with("0.49 "));
    assert!(occurrences[1].starts_with("0.79 "));
}

#[test]
fn elements_always_includes_exactly_one_evolution_axis() {
    let all = elements(&parse_fixture("tea-shop.owm"));
    let axes: Vec<_> = all.iter().filter(|e| e.element_type == EVOLUTION_AXIS).collect();
    assert_eq!(axes.len(), 1);
    let axis_element = axes[0];
    let stages: Vec<&str> = (0..4).map(|i| axis_element.attribute(&format!("stage.{i}"))).collect();
    assert!(stages.iter().all(|s| !s.is_empty()));
    let first: Vec<&str> = stages[0].split('|').collect();
    assert_eq!(first[0], "Genesis");
    assert_eq!(first[2].parse::<f64>().unwrap(), axis::CUSTOM_BUILT);
    assert_eq!(axis_element.attribute("title"), "Tea shop");
}

#[test]
fn elements_includes_the_axis_even_for_an_empty_map() {
    let all = elements(&WardleyMap::default());
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].element_type, EVOLUTION_AXIS);
}

#[test]
fn elements_resolves_a_submaps_url_to_its_address() {
    let all = elements(&parse_fixture("submaps-and-urls.owm"));
    let fulfilment = named(&all, "Fulfilment");
    assert_eq!(fulfilment.attribute("url"), "fulfilmentMap");
    assert_eq!(
        fulfilment.attribute("urlAddress"),
        "https://onlinewardleymaps.com/#clone:example-fulfilment"
    );
}

#[test]
fn elements_covers_every_fixture_without_throwing() {
    let names = corpus();
    assert!(names.len() >= 5);
    for name in names {
        assert!(!elements(&parse_fixture(&name)).is_empty(), "{name}");
    }
}

#[test]
fn diff_reports_nothing_for_two_identical_renderings() {
    let map = parse_fixture("tea-shop.owm");
    let identities = reconcile(&map, &[]);
    assert!(
        diff(
            &mapper::elements(&map, &identities),
            &mapper::elements(&map, &identities)
        )
        .is_empty()
    );
}

#[test]
fn diff_expresses_a_move_as_an_add_not_a_remove_and_add() {
    let before = parse_text("component Alpha [0.9, 0.1]\n");
    let identities = reconcile(&before, &[]);
    let after = parse_text("component Alpha [0.5, 0.5]\n");
    let deltas = diff(
        &mapper::elements(&before, &identities),
        &mapper::elements(&after, &identities),
    );
    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        DiagramDelta::Add(added) => assert_eq!(added.len(), 1),
        other => panic!("{other:?}"),
    }
}

#[test]
fn diff_reports_a_remove_for_an_element_that_is_gone() {
    let before = parse_text("component Alpha [0.9, 0.1]\ncomponent Beta [0.5, 0.5]\n");
    let identities = reconcile(&before, &[]);
    let after = parse_text("component Alpha [0.9, 0.1]\n");
    let deltas = diff(
        &mapper::elements(&before, &identities),
        &mapper::elements(&after, &reconcile(&after, &identities)),
    );
    let removed: Vec<_> = deltas
        .iter()
        .filter_map(|d| match d {
            DiagramDelta::Remove(ids) => Some(ids.len()),
            _ => None,
        })
        .collect();
    assert_eq!(removed, [1]);
}

#[test]
fn group_expresses_a_pipeline_as_the_contracts_grouping() {
    let map = parse_text(
        "component Kettle [0.43, 0.35]\npipeline Kettle\n{\n  component Campfire [0.15]\n  component Electric [0.63]\n}",
    );
    let deltas = mapper::group(&map, &reconcile(&map, &[]));
    assert_eq!(deltas.len(), 1);
    match &deltas[0] {
        DiagramDelta::Group {
            source_element_ids,
            group_element,
        } => {
            assert_eq!(source_element_ids.len(), 2);
            assert_eq!(group_element.attribute("name"), "Kettle");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn group_skips_a_pipeline_whose_parent_does_not_exist() {
    let map = parse_text("pipeline Missing\n{\n  component Child [0.5]\n}\n");
    assert!(mapper::group(&map, &reconcile(&map, &[])).is_empty());
}

#[test]
fn elements_gives_every_element_a_non_empty_id() {
    let all = elements(&parse_fixture("strategy-vocabulary.owm"));
    assert!(!all.is_empty());
    assert!(all.iter().all(|e| !e.id.is_empty()));
}

// ---- the viewport (`WardleyElementMapper.Visible`) ------------------------------------------------------------------

#[test]
fn visible_keeps_the_axis_a_component_in_view_and_the_components_it_links_to() {
    let map = parse_text(
        "component In [0.9, 0.1]\ncomponent Partner [0.1, 0.9]\ncomponent Far [0.2, 0.8]\nIn->Partner\nnote Here [0.85, 0.15]\nnote There [0.1, 0.9]",
    );
    let identities = reconcile(&map, &[]);
    // The top left corner of the map: In at (0.1, 0.1), Partner and Far at the bottom right.
    let shown = mapper::visible(&map, &identities, DiagramViewport::new(0.0, 0.0, 0.3, 0.3));
    let names: Vec<&str> = shown
        .iter()
        .map(|e| {
            if e.element_type == EVOLUTION_AXIS {
                "axis"
            } else if e.element_type == LINK {
                "link"
            } else if !e.attribute("name").is_empty() {
                e.attribute("name")
            } else {
                e.attribute("text")
            }
        })
        .collect();
    assert_eq!(names, ["axis", "In", "Partner", "link", "Here"]);
    assert_eq!(
        mapper::visible(&map, &identities, DiagramViewport::UNBOUNDED).len(),
        mapper::elements(&map, &identities).len()
    );
}

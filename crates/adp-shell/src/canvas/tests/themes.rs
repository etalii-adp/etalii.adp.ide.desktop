//! Every tool's canvas in the colours of the active theme, and again in the other when the theme switches
//! (etalii.adp spec 030, FR-045, task T087): each tool's parity fixture drawn in the dark theme, switched to the light
//! one and back, with the colours its scene paints compared each time.

use std::collections::BTreeSet;
use std::path::Path;

use adp_scene::scene::Primitive;
use adp_scene::style::Colour;
use adp_theme::Kind;

use super::Harness;

/// Every colour the scene paints with, as written.
fn colours(harness: &Harness) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut add = |colour: &Option<Colour>| {
        if let Some(value) = colour.as_ref().and_then(|c| c.value.clone()) {
            found.insert(value.to_ascii_lowercase());
        }
    };
    for primitive in &harness.tab.scene().expect("a scene").primitives {
        match primitive {
            Primitive::Shape(shape) => {
                add(&shape.fill);
                add(&shape.stroke.as_ref().and_then(|s| s.colour.clone()));
            }
            Primitive::Text(text) => add(&text.font.colour),
            Primitive::Edge(edge) => add(&edge.stroke.colour),
            Primitive::Hook(_) => {}
        }
    }
    found
}

/// The first element the scene draws a box for.
fn first_element(harness: &Harness) -> String {
    harness
        .tab
        .cache
        .as_ref()
        .and_then(|c| c.boxes.first())
        .map(|(id, _)| id.clone())
        .expect("the scene draws an element")
}

/// Draws `harness` in the dark theme, then the light, then the dark again, and checks that the scene's colours follow
/// the theme: different in the two where the tool has colours of its own, the same again when it switches back, and
/// a selection outlined in each theme's own selection colour.
fn follows_the_theme(mut harness: Harness, tool: &str, own_colours: bool) {
    harness.settle();
    let dark = colours(&harness);
    assert!(!dark.is_empty(), "{tool} paints with colours");
    assert!(
        harness.tab.cache.as_ref().unwrap().surface.is_none(),
        "{tool} lays no light surface of its own under the dark canvas"
    );
    harness.switch_theme(Kind::Light);
    let light = colours(&harness);
    if own_colours {
        assert_ne!(dark, light, "{tool} draws in other colours in the light theme");
    }
    harness.switch_theme(Kind::Dark);
    assert_eq!(colours(&harness), dark, "{tool} draws in the dark colours again");

    // A selection, in each theme's selection colour.
    let id = first_element(&harness);
    harness.tab.selection = vec![id];
    harness.frame(Vec::new());
    let selected_dark = colours(&harness);
    harness.switch_theme(Kind::Light);
    let selected_light = colours(&harness);
    assert_ne!(selected_dark, selected_light, "{tool} outlines a selection per theme");
}

#[test]
fn every_tools_canvas_follows_the_active_theme() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures");
    let rdf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/diagrams/rdf/tests/examples/w3c-turtle");
    let tools: Vec<(&str, Harness, bool)> = vec![
        ("the dependency graph", Harness::dependency_graph(), true),
        ("the causal loop diagram", super::causal_loop::reference(), true),
        ("the hype cycle", Harness::coal_technologies(), true),
        (
            "the timeline",
            Harness::open_from(
                &fixtures.join("generic/timeline/roadmap"),
                &["roadmap.adp", "roadmap.tml"],
                "roadmap.adp",
                "roadmap.tml",
            ),
            true,
        ),
        // A C4 element is painted in its model's colours, the same in both themes, as Structurizr paints it.
        ("the C4 container view", Harness::c4_containers(), false),
        ("the agent behavior model", super::abm::research_assistant(), true),
        (
            "the agent activity diagram",
            super::agent_activity::two_projects(),
            true,
        ),
        ("the Ansible structure", super::ansible::infrastructure(), true),
        ("the Databricks job", super::databricks::nightly_ingest(), true),
        ("the .NET dependency graph", super::dotnet::pipeline_toolkit(), true),
        ("the functional decomposition graph", super::fdg::field_service(), true),
        ("the Helm chart", super::helm::nginx(), true),
        ("the Azure DevOps pipeline", super::pipeline::multi_stage(), true),
        (
            "the RDF data graph",
            Harness::open_from(
                &rdf,
                &["example-1.adp", "example-1.ttl"],
                "example-1.adp",
                "example-1.adp",
            ),
            true,
        ),
        ("the Sankey diagram", super::sankey::company(), true),
        ("the SPARQL query", super::sparql::groups(), true),
        ("the supply chain", super::supply_chain::automotive(), true),
        ("the Wardley map", super::wardley_map::strategy_vocabulary(), true),
    ];
    for (tool, harness, own_colours) in tools {
        follows_the_theme(harness, tool, own_colours);
    }
}

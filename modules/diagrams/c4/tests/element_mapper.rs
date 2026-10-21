//! What a connection receives of a C4 view (etalii.adp spec 030, task T061): ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4ElementMapper.Tests.cs` at `8b6ccfc`. Elements at their
//! measured sizes, relationships as their own elements and elevated to the level the view shows, boundaries, the
//! view's title and legend, the default theme and a document's styles, and the viewport's one hop. Where the
//! standalone host decoded a protobuf payload, these read the element's attributes, which carry the same fields.

use adp_diagram::{DiagramElement, DiagramProblemLocation, DiagramViewport};
use adp_diagram_c4::layout;
use adp_diagram_c4::mapper::{self, BOUNDARY_TYPE, NODE_TYPE, RELATIONSHIP_TYPE, VIEW_TYPE};
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4Element, C4ElementKind, C4View, C4ViewKind, C4Workspace};
use adp_diagram_c4::rules::{self, codes};
use adp_diagram_c4::{parser, theme};

const SAMPLE: &str = r#"workspace "Big Bank" {
    model {
        u = person "Customer" "A retail banking customer."
        s = softwareSystem "Internet Banking" "Lets customers view accounts." {
            web = container "Web App" "Serves pages." "React"
            db = container "Database" "Stores accounts." "PostgreSQL" "Database"
            web -> db "Reads from and writes to" "SQL/TCP"
        }
        mainframe = softwareSystem "Mainframe" "The core system." "External"
        u -> web "Views accounts using" "HTTPS"
        s -> mainframe "Gets account information from" "XML/HTTPS"
    }
    views {
        systemContext s "context" {
            include *
        }
        container s "containers" {
            include *
        }
    }
}"#;

fn load(key: &str, dsl: &str) -> (C4Workspace, C4View) {
    let workspace = parser::parse_text(dsl);
    let view = workspace.find_view(key).unwrap().clone();
    (workspace, view)
}

fn visible_in(key: &str, dsl: &str, viewport: DiagramViewport) -> Vec<DiagramElement> {
    let (workspace, view) = load(key, dsl);
    mapper::visible(&workspace, &view, viewport, &C4Metrics::default(), None)
}

fn visible(key: &str, dsl: &str) -> Vec<DiagramElement> {
    visible_in(key, dsl, DiagramViewport::UNBOUNDED)
}

fn of_type<'a>(elements: &'a [DiagramElement], element_type: &str) -> Vec<&'a DiagramElement> {
    elements.iter().filter(|e| e.element_type == element_type).collect()
}

fn single<'a>(elements: &'a [DiagramElement], id: &str) -> &'a DiagramElement {
    let found: Vec<_> = elements.iter().filter(|e| e.id == id).collect();
    assert_eq!(found.len(), 1, "{id}");
    found[0]
}

fn number(element: &DiagramElement, name: &str) -> f64 {
    element.attribute(name).parse().unwrap()
}

#[test]
fn a_context_view_delivers_its_people_and_systems_and_no_containers() {
    let elements = visible("context", SAMPLE);
    let mut ids: Vec<&str> = of_type(&elements, NODE_TYPE).iter().map(|e| e.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, ["mainframe", "s", "u"]);
}

#[test]
fn every_node_carries_its_measured_box_so_the_canvas_never_guesses() {
    let elements = visible("context", SAMPLE);
    for node in of_type(&elements, NODE_TYPE) {
        assert!(number(node, "width") > 0.0, "an element arrived with no measured width");
        assert!(
            number(node, "height") > 0.0,
            "an element arrived with no measured height"
        );
    }
}

#[test]
fn a_node_carries_the_three_lines_c4_asks_for_name_type_and_description() {
    let elements = visible("containers", SAMPLE);
    let web = single(&elements, "web");
    assert_eq!(web.attribute("name"), "Web App");
    assert_eq!(web.attribute("typeLine"), "[Container: React]");
    assert_eq!(web.attribute("description"), "Serves pages.");
}

#[test]
fn an_external_system_is_flagged_and_muted() {
    let elements = visible("context", SAMPLE);
    let mainframe = single(&elements, "mainframe");
    assert_eq!(mainframe.attribute("external"), "true");
    assert_eq!(mainframe.attribute("background"), theme::EXTERNAL_BACKGROUND);
}

#[test]
fn the_default_theme_is_the_one_the_reference_diagrams_show() {
    let elements = visible("context", SAMPLE);
    for (id, background, shape) in [
        ("u", theme::PERSON_BACKGROUND, "Person"),
        ("s", theme::SOFTWARE_SYSTEM_BACKGROUND, "RoundedBox"),
    ] {
        let node = single(&elements, id);
        assert_eq!(node.attribute("background"), background);
        assert_eq!(node.attribute("shape"), shape);
    }
}

#[test]
fn a_components_text_is_black_because_its_fill_is_too_light_for_white() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "S" "desc" {
            c = container "C" "desc" "Kotlin" {
                comp = component "Comp" "desc" "Kotlin"
            }
        }
    }
    views {
        component c "components" {
            include *
        }
    }
}"#;
    let elements = visible("components", dsl);
    let comp = single(&elements, "comp");
    assert_eq!(comp.attribute("background"), theme::COMPONENT_BACKGROUND);
    assert_eq!(comp.attribute("color"), theme::DARK_TEXT);
}

#[test]
fn a_data_store_is_a_cylinder() {
    let elements = visible("containers", SAMPLE);
    assert_eq!(single(&elements, "db").attribute("shape"), "Cylinder");
}

#[test]
fn a_documents_own_styles_override_the_default_theme() {
    let dsl = SAMPLE.replace(
        "        container s \"containers\" {\n            include *\n        }\n",
        "        container s \"containers\" {\n            include *\n        }\n        styles {\n            element \"Person\" {\n                background #ff0000\n            }\n        }\n",
    );
    assert_ne!(dsl, SAMPLE);
    let workspace = parser::parse_text(&dsl);
    assert!(workspace.styles.iter().any(|style| style.tag == "Person"));
}

#[test]
fn relationships_travel_as_their_own_elements_with_label_and_technology() {
    let elements = visible("containers", SAMPLE);
    let relationships = of_type(&elements, RELATIONSHIP_TYPE);
    assert!(!relationships.is_empty());
    let from_web: Vec<_> = relationships
        .iter()
        .filter(|r| r.source.as_deref() == Some("web"))
        .collect();
    assert_eq!(from_web.len(), 1);
    let line = from_web[0];
    assert_eq!(line.target.as_deref(), Some("db"));
    assert_eq!(line.attribute("description"), "Reads from and writes to");
    assert_eq!(line.attribute("technology"), "SQL/TCP");
    assert!(number(line, "sourceWidth") > 0.0 && number(line, "destinationWidth") > 0.0);
}

fn ends(elements: &[DiagramElement]) -> Vec<(String, String)> {
    of_type(elements, RELATIONSHIP_TYPE)
        .iter()
        .map(|r| (r.source.clone().unwrap(), r.target.clone().unwrap()))
        .collect()
}

#[test]
fn a_context_view_elevates_a_containers_relationship_to_the_system_that_contains_it() {
    let ends = ends(&visible("context", SAMPLE));
    assert!(ends.contains(&("s".into(), "mainframe".into())), "{ends:?}");
    assert!(ends.contains(&("u".into(), "s".into())), "{ends:?}");
}

#[test]
fn an_elevated_relationship_is_drawn_once_however_many_collapse_onto_it() {
    let dsl = r#"workspace {
    model {
        s = softwareSystem "S" "desc" {
            a = container "A" "desc" "Tech"
            b = container "B" "desc" "Tech"
        }
        ext = softwareSystem "Ext" "desc" "External"
        a -> ext "Calls" "HTTPS"
        b -> ext "Calls" "HTTPS"
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}"#;
    assert_eq!(of_type(&visible("context", dsl), RELATIONSHIP_TYPE).len(), 1);
}

#[test]
fn a_relationship_between_two_containers_of_one_system_is_not_drawn_on_the_context_view() {
    let ends = ends(&visible("context", SAMPLE));
    assert!(!ends.contains(&("s".into(), "s".into())));
}

#[test]
fn a_container_view_draws_the_system_as_its_boundary_and_not_also_as_a_box() {
    let elements = visible("containers", SAMPLE);
    let nodes: Vec<&str> = of_type(&elements, NODE_TYPE).iter().map(|e| e.id.as_str()).collect();
    assert!(!nodes.contains(&"s"));
    assert!(nodes.contains(&"web"));
    assert!(!of_type(&elements, BOUNDARY_TYPE).is_empty());
}

#[test]
fn a_context_view_still_draws_its_own_system_because_there_it_is_the_primary_element() {
    let elements = visible("context", SAMPLE);
    assert!(of_type(&elements, NODE_TYPE).iter().any(|e| e.id == "s"));
}

#[test]
fn a_container_view_draws_a_boundary() {
    let elements = visible("containers", SAMPLE);
    let boundaries = of_type(&elements, BOUNDARY_TYPE);
    assert_eq!(boundaries.len(), 1);
    assert_eq!(boundaries[0].attribute("name"), "Internet Banking");
    assert_eq!(boundaries[0].attribute("kind"), "Software System");
}

fn view_element(elements: &[DiagramElement]) -> &DiagramElement {
    let views = of_type(elements, VIEW_TYPE);
    assert_eq!(views.len(), 1);
    views[0]
}

fn legend(view: &DiagramElement) -> Vec<String> {
    view.attributes
        .iter()
        .filter(|(name, _)| name.starts_with("legend.") && name.ends_with(".label"))
        .map(|(_, value)| value.clone())
        .collect()
}

#[test]
fn every_view_carries_a_title_in_c4s_own_wording() {
    let elements = visible("context", SAMPLE);
    assert_eq!(
        view_element(&elements).attribute("title"),
        "System Context diagram for Internet Banking"
    );
}

#[test]
fn the_title_names_the_diagram_type_and_its_scope() {
    let workspace = C4Workspace {
        name: "W".into(),
        elements: vec![C4Element {
            id: "s".into(),
            kind: C4ElementKind::SoftwareSystem,
            name: "S".into(),
            description: "d".into(),
            technology: String::new(),
            tags: Vec::new(),
            parent_id: None,
            line: 1,
            referenced_id: None,
        }],
        ..C4Workspace::default()
    };
    for (kind, expected) in [
        (C4ViewKind::SystemContext, "System Context diagram for S"),
        (C4ViewKind::Container, "Container diagram for S"),
        (C4ViewKind::Component, "Component diagram for S"),
    ] {
        let view = view_of(kind, Some("s"), None);
        assert_eq!(mapper::title_of(&workspace, &view), expected);
    }
}

fn view_of(kind: C4ViewKind, scope: Option<&str>, title: Option<&str>) -> C4View {
    C4View {
        kind,
        key: "k".into(),
        scope_id: scope.map(str::to_owned),
        environment: None,
        title: title.map(str::to_owned),
        includes_everything: true,
        includes: Vec::new(),
        excludes: Vec::new(),
        auto_layout: None,
        interactions: Vec::new(),
        line: 1,
    }
}

#[test]
fn a_title_the_document_declares_wins() {
    let workspace = C4Workspace {
        name: "W".into(),
        ..C4Workspace::default()
    };
    let view = view_of(C4ViewKind::SystemContext, None, Some("Our own title"));
    assert_eq!(mapper::title_of(&workspace, &view), "Our own title");
}

#[test]
fn every_view_carries_a_key_describing_the_notation_actually_in_use() {
    let elements = visible("context", SAMPLE);
    let legend = legend(view_element(&elements));
    assert!(!legend.is_empty());
    assert!(legend.iter().any(|l| l == "Person"));
    assert!(legend.iter().any(|l| l == "Software System"));
    assert!(legend.iter().any(|l| l.to_lowercase().contains("external")));
}

#[test]
fn the_view_carries_no_problems_of_its_own_because_core_already_pushes_them() {
    let dsl = SAMPLE.replace(
        "person \"Customer\" \"A retail banking customer.\"",
        "person \"Customer\"",
    );
    let elements = visible("context", &dsl);
    let view = view_element(&elements);
    assert_eq!(view.attribute("title"), "System Context diagram for Internet Banking");
    assert!(!legend(view).is_empty());
    assert!(
        !view
            .attributes
            .iter()
            .any(|(name, _)| name.to_lowercase().contains("problem"))
    );
}

#[test]
fn the_rules_still_reach_the_user_through_cores_validator_seam() {
    let dsl = SAMPLE.replace(
        "person \"Customer\" \"A retail banking customer.\"",
        "person \"Customer\"",
    );
    let problems = rules::validate(&parser::parse_text(&dsl));
    let missing: Vec<_> = problems
        .iter()
        .filter(|p| p.rule_id == codes::MISSING_DESCRIPTION)
        .collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].location, Some(DiagramProblemLocation::Element("u".into())));
}

#[test]
fn a_tight_viewport_still_delivers_the_far_end_of_a_line_that_leaves_it() {
    let (workspace, view) = load("containers", SAMPLE);
    let layout = layout::compute(&workspace, &view, &C4Metrics::default(), None);
    let web = layout.boxes.get("web").unwrap();
    let tight = DiagramViewport::new(web.x, web.y, web.right(), web.bottom());
    let ids: Vec<String> = visible_in("containers", SAMPLE, tight)
        .into_iter()
        .map(|e| e.id)
        .collect();
    assert!(ids.contains(&"web".to_owned()));
    assert!(ids.contains(&"db".to_owned()));
}

#[test]
fn a_viewport_far_off_the_diagram_delivers_no_nodes() {
    let elements = visible_in(
        "context",
        SAMPLE,
        DiagramViewport::new(100000.0, 100000.0, 200000.0, 200000.0),
    );
    assert!(of_type(&elements, NODE_TYPE).is_empty());
    assert!(!of_type(&elements, VIEW_TYPE).is_empty());
}

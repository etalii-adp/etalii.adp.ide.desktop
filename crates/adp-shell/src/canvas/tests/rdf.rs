//! The RDF family on the canvas with its module (etalii.adp spec 030, task T083): the data graph of the module's
//! `w3c-turtle` example opened through its registration and as a bare file, its cards drawn where the module laid
//! them out with their badges and rows, a card dragged by its top-left into the registration's `layout:` block and
//! the drag undone; and each reading's scene built from elements as its mapper sends them (`RdfCanvas.test.tsx`,
//! `OwlCanvas.test.tsx`, `SkosCanvas.test.tsx`, `ShaclCanvas.test.tsx`).

use std::path::{Path, PathBuf};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_rdf::payload::Payload;
use adp_scene::Rect;
use adp_scene::scene::{HookKind, Owner, Primitive, Role};

use super::super::rdf_scene::{self, CARD_WIDTH, CONCEPT_HEIGHT, CONCEPT_WIDTH, REGION_WIDTH, SHAPE_CARD_WIDTH};
use super::{Harness, texts_of};

fn example(folder: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/diagrams/rdf/tests/examples")
        .join(folder)
}

fn node(id: &str, element_type: &str, (x, y): (f64, f64), payload: Payload) -> DiagramElement {
    DiagramElement {
        id: id.to_owned(),
        x,
        y,
        placed: true,
        element_type: element_type.to_owned(),
        kind: ElementKind::Node,
        parent: None,
        source: None,
        target: None,
        attributes: payload.into_attributes(),
    }
}

fn edge(id: &str, element_type: &str, from: &str, to: &str, payload: Payload) -> DiagramElement {
    DiagramElement {
        id: id.to_owned(),
        x: 0.0,
        y: 0.0,
        placed: false,
        element_type: element_type.to_owned(),
        kind: ElementKind::Relation,
        parent: None,
        source: Some(from.to_owned()),
        target: Some(to.to_owned()),
        attributes: payload
            .text("fromElementId", from)
            .text("toElementId", to)
            .into_attributes(),
    }
}

fn truncation(element_type: &str, shown: i64, total: i64) -> DiagramElement {
    node(
        "truncation",
        element_type,
        (0.0, 0.0),
        Payload::new().number("shown", shown).number("total", total),
    )
}

/// The outline of the element `id`'s body.
fn body_of<'a>(scene: &'a adp_scene::Scene, id: &str) -> &'a adp_scene::scene::ShapePrimitive {
    scene
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::Shape(s) if s.owner == Owner::Element(id.to_owned()) && s.role == Role::Body => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{id} has a body"))
}

fn anchors_of(built: &super::super::module_scene::ModuleScene, id: &str) -> Vec<String> {
    built
        .anchors
        .iter()
        .filter(|(e, _, _)| e == id)
        .map(|(_, name, _)| name.clone())
        .collect()
}

fn edge_arrowed(scene: &adp_scene::Scene, id: &str) -> (bool, Vec<f64>) {
    scene
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::Edge(e) if e.owner == Owner::Relation(id.to_owned()) => {
                Some((e.end.is_some(), e.stroke.dash.clone()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("{id} is drawn"))
}

#[test]
fn a_registered_data_graph_is_drawn_from_its_modules_layout_and_a_card_is_dragged_into_its_registration() {
    let mut harness = Harness::open_from(
        &example("w3c-turtle"),
        &["example-1.adp", "example-1.ttl"],
        "example-1.adp",
        "example-1.adp",
    );
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    let scene = harness.tab.scene().expect("a scene");
    let texts = texts_of(scene);
    for label in [
        "green-goblin",
        "spiderman",
        "foaf:Person",
        "foaf:name: Green Goblin",
        "foaf:name: Spiderman",
        "foaf:name: Человек-паук @ru",
        "rel:enemyOf",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // One band of foaf:Person, two cells of 280 apart; each card 220 wide and as tall as its content.
    let goblin = "res:http://example.org/#green-goblin";
    let spiderman = "res:http://example.org/#spiderman";
    let bounds = harness.tab.bounds_of(goblin).unwrap();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (0.0, 0.0, 220.0, 72.0)
    );
    let bounds = harness.tab.bounds_of(spiderman).unwrap();
    assert_eq!(
        (bounds.x, bounds.y, bounds.width, bounds.height),
        (280.0, 0.0, 220.0, 88.0)
    );

    // A drag writes where the card's top-left went, into the registration and nowhere else.
    let body = harness.root.join("example-1.ttl");
    let ttl = std::fs::read(&body).unwrap();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &[goblin.to_owned()], (40.0, 20.0));
    let moved = harness.written();
    assert!(
        moved.ends_with(&format!("layout:\r\n  {goblin}: 40 20\r\n")),
        "{moved:?}"
    );
    assert_eq!(
        std::fs::read(&body).unwrap(),
        ttl,
        "the RDF file is never touched by a drag"
    );
    harness.tab.changed();
    harness.settle();
    let after = harness.tab.bounds_of(goblin).unwrap();
    assert_eq!((after.x, after.y), (40.0, 20.0));

    // One undo takes the drag back.
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);

    harness.screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/rdf.png"));
}

#[test]
fn a_bare_turtle_file_opens_as_a_data_graph() {
    let mut harness = Harness::open_from(
        &example("w3c-turtle"),
        &["example-1.ttl"],
        "example-1.ttl",
        "example-1.ttl",
    );
    assert_eq!(harness.tab.opened.origin, "w3c/rdf");
    harness.settle();
    let scene = harness.tab.scene().expect("a scene");
    assert!(texts_of(scene).contains(&"spiderman"));
}

#[test]
fn a_data_graph_card_wears_its_badges_and_rows_and_a_blank_card_is_dashed_without_anchors() {
    let elements = vec![
        node(
            "res:a",
            rdf_scene::RDF_RESOURCE,
            (0.0, 0.0),
            Payload::new()
                .text("iri", "http://example.org/a")
                .text("display", "ex:a")
                .list("typeBadges", &["ex:Person", "foaf:Agent"])
                .records(
                    "rows",
                    &[vec![
                        ("predicate", "rdfs:label".to_owned()),
                        ("value", "Ada".to_owned()),
                        ("annotation", "@en".to_owned()),
                    ]],
                )
                .flag("blank", false),
        ),
        node(
            "blank:1",
            rdf_scene::RDF_RESOURCE,
            (280.0, 0.0),
            Payload::new().text("display", "_:b1").flag("blank", true),
        ),
        edge(
            "edge:res:a|http://example.org/knows|blank:1",
            rdf_scene::RDF_EDGE,
            "res:a",
            "blank:1",
            Payload::new().text("predicate", "ex:knows"),
        ),
        truncation(rdf_scene::RDF_TRUNCATION, 1000, 1200),
    ];
    let built = rdf_scene::build("w3c/rdf", &elements, &[], None, false);
    let texts = texts_of(&built.scene);
    for label in [
        "ex:a",
        "ex:Person · foaf:Agent",
        "rdfs:label: Ada @en",
        "_:b1",
        "ex:knows",
        "Showing 1000 of 1200 resources — edits are withheld on this truncated view",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // Title, badges and one row: 30 + 16 + 16 + 10.
    assert_eq!(built.rects["res:a"], Rect::new(0.0, 0.0, CARD_WIDTH, 72.0));
    assert_eq!(built.rects["blank:1"], Rect::new(280.0, 0.0, CARD_WIDTH, 40.0));
    let blank = body_of(&built.scene, "blank:1");
    assert_eq!(blank.stroke.as_ref().unwrap().dash, vec![5.0, 3.0]);
    assert_eq!(blank.opacity, 0.75);
    assert_eq!(anchors_of(&built, "res:a"), ["w", "e"]);
    assert!(
        anchors_of(&built, "blank:1").is_empty(),
        "a statement never starts at a blank node"
    );
    assert_eq!(
        edge_arrowed(&built.scene, "edge:res:a|http://example.org/knows|blank:1"),
        (true, vec![])
    );
    let mut top_left = built.top_left.clone();
    top_left.sort();
    assert_eq!(top_left, ["blank:1", "res:a"]);
}

#[test]
fn an_ontology_draws_rounds_for_classes_boxes_for_datatypes_and_each_axiom_with_its_own_line() {
    let class = |id: &str, display: &str, at: (f64, f64)| {
        node(
            id,
            rdf_scene::OWL_NODE,
            at,
            Payload::new()
                .text("kind", "class")
                .text("iri", "")
                .text("display", display),
        )
    };
    let elements = vec![
        class("res:Pizza", "Pizza", (0.0, 0.0)),
        class("res:Food", "Food", (300.0, 0.0)),
        class("res:Meal", "Meal", (600.0, 0.0)),
        node(
            "thing:1",
            rdf_scene::OWL_NODE,
            (0.0, 200.0),
            Payload::new().text("kind", "thing").text("display", "Thing"),
        ),
        node(
            "dt:xsd:string",
            rdf_scene::OWL_NODE,
            (300.0, 200.0),
            Payload::new()
                .text("kind", "datatype")
                .text("display", "xsd:string")
                .flag("external", true),
        ),
        node(
            "ind:margherita",
            rdf_scene::OWL_NODE,
            (600.0, 200.0),
            Payload::new()
                .text("kind", "individual")
                .text("display", "margherita")
                .list("badges", &["Pizza"]),
        ),
        node(
            "expr:1",
            rdf_scene::OWL_EXPRESSION,
            (0.0, 400.0),
            Payload::new()
                .text("label", "hasTopping some Cheese")
                .flag("elided", true)
                .flag("malformed", true)
                .text("ownerElementId", "res:Pizza")
                .text("kind", "restriction"),
        ),
        edge(
            "edge:subclass",
            rdf_scene::OWL_EDGE,
            "res:Pizza",
            "res:Food",
            Payload::new().text("kind", "subclass"),
        ),
        edge(
            "edge:equivalent",
            rdf_scene::OWL_EDGE,
            "res:Food",
            "res:Meal",
            Payload::new().text("kind", "equivalent"),
        ),
        edge(
            "edge:property",
            rdf_scene::OWL_EDGE,
            "res:Pizza",
            "dt:xsd:string",
            Payload::new()
                .text("kind", "datatype-property")
                .text("label", "hasName (functional)"),
        ),
        edge(
            "edge:expression",
            rdf_scene::OWL_EDGE,
            "res:Pizza",
            "expr:1",
            Payload::new().text("kind", "expression"),
        ),
        truncation(rdf_scene::OWL_TRUNCATION, 3, 9),
    ];
    let built = rdf_scene::build("w3c/owl", &elements, &[], None, true);
    let texts = texts_of(&built.scene);
    for label in [
        "Pizza",
        "Thing",
        "xsd:string",
        "margherita",
        "hasTopping some Cheese …",
        "hasName (functional)",
        "Showing 3 of 9 elements — edits are withheld on this truncated view",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    assert_eq!(built.rects["res:Pizza"], Rect::new(0.0, 0.0, 190.0, 70.0));
    assert_eq!(built.rects["thing:1"].width, 190.0 * 0.55);
    assert_eq!(built.rects["ind:margherita"].height, 30.0 + 16.0 + 10.0);
    assert_eq!(
        body_of(&built.scene, "thing:1").stroke.as_ref().unwrap().dash,
        vec![4.0, 3.0]
    );
    assert_eq!(body_of(&built.scene, "dt:xsd:string").opacity, 0.55);
    // Equivalence doubles both its ends' rings, and only theirs.
    let rings = |id: &str| {
        built
            .scene
            .primitives
            .iter()
            .filter(|p| matches!(p, Primitive::Shape(s) if s.owner == Owner::Element(id.to_owned()) && s.role == Role::BodyPart))
            .count()
    };
    assert_eq!((rings("res:Food"), rings("res:Meal"), rings("res:Pizza")), (1, 1, 0));
    assert_eq!(edge_arrowed(&built.scene, "edge:subclass"), (true, vec![6.0, 4.0]));
    assert_eq!(edge_arrowed(&built.scene, "edge:equivalent"), (false, vec![]));
    assert_eq!(edge_arrowed(&built.scene, "edge:property"), (true, vec![]));
    assert_eq!(edge_arrowed(&built.scene, "edge:expression"), (false, vec![2.0, 3.0]));
    assert_eq!(anchors_of(&built, "res:Pizza"), ["axiom-left", "axiom-right"]);
    assert!(anchors_of(&built, "expr:1").is_empty(), "an expression starts no axiom");
    let malformed = body_of(&built.scene, "expr:1");
    assert_eq!(malformed.stroke.as_ref().unwrap().dash, vec![3.0, 2.0]);
}

#[test]
fn a_scheme_draws_its_regions_under_its_concepts_and_its_three_edge_languages() {
    let concept = |id: &str, label: &str, kind: i64, at: (f64, f64)| {
        node(
            id,
            rdf_scene::SKOS_CONCEPT,
            at,
            Payload::new()
                .text("iri", "")
                .text("label", label)
                .text("languageTag", "en")
                .number("labelKind", kind)
                .text("notation", if kind == 0 { "A1" } else { "" })
                .flag("blank", false)
                .flag("languageChip", kind == 0),
        )
    };
    let elements = vec![
        concept("res:animals", "Animals", 0, (0.0, 60.0)),
        concept("res:cats", "Cats", 1, (0.0, 160.0)),
        concept("res:dogs", "http://example.org/dogs", 2, (240.0, 160.0)),
        node(
            "res:scheme",
            rdf_scene::SKOS_SCHEME,
            (0.0, 0.0),
            Payload::new().text("label", "Zoo").number("memberCount", 3),
        ),
        node(
            "res:list",
            rdf_scene::SKOS_COLLECTION,
            (300.0, 0.0),
            Payload::new().text("label", "Pets").flag("ordered", true),
        ),
        edge(
            "edge:h",
            rdf_scene::SKOS_EDGE,
            "res:cats",
            "res:animals",
            Payload::new().number("kind", 0),
        ),
        edge(
            "edge:r",
            rdf_scene::SKOS_EDGE,
            "res:cats",
            "res:dogs",
            Payload::new().number("kind", 1),
        ),
        edge(
            "edge:m",
            rdf_scene::SKOS_EDGE,
            "res:dogs",
            "res:animals",
            Payload::new().number("kind", 2).text("predicate", "skos:closeMatch"),
        ),
        truncation(rdf_scene::SKOS_TRUNCATION, 5, 7),
    ];
    let built = rdf_scene::build("w3c/skos", &elements, &[], None, false);
    let texts = texts_of(&built.scene);
    for label in [
        "Animals",
        "A1",
        "en",
        "Zoo (3)",
        "Pets",
        "ordered",
        "skos:closeMatch",
        "Showing 5 of 7 terms — edits are withheld on this truncated view",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    assert_eq!(
        built.rects["res:animals"],
        Rect::new(0.0, 60.0, CONCEPT_WIDTH, CONCEPT_HEIGHT)
    );
    assert_eq!(built.rects["res:scheme"].width, REGION_WIDTH);
    // The regions are painted before the concepts, so a concept is picked over the region it sits in.
    let order: Vec<&str> = built.boxes.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(order, ["res:scheme", "res:list", "res:animals", "res:cats", "res:dogs"]);
    assert_eq!(edge_arrowed(&built.scene, "edge:h"), (false, vec![]));
    assert_eq!(edge_arrowed(&built.scene, "edge:r"), (false, vec![6.0, 4.0]));
    assert_eq!(edge_arrowed(&built.scene, "edge:m"), (true, vec![2.0, 3.0]));
    assert_eq!(anchors_of(&built, "res:cats"), ["file", "relate"]);
    // An alternate label stands in italic; the IRI's own form is monospaced.
    let font_of = |content: &str| {
        built
            .scene
            .primitives
            .iter()
            .find_map(|p| match p {
                Primitive::Text(t) if t.text == content => Some(t.font.clone()),
                _ => None,
            })
            .unwrap()
    };
    assert!(font_of("Cats").italic);
    assert!(font_of("http://example.org/dogs").family.is_some());
}

#[test]
fn a_shapes_card_lists_its_targets_and_rows_and_a_deactivated_shape_is_faded() {
    let row = |path: &str, summary: &str, cardinality: &str, sparql: bool| {
        vec![
            ("path", path.to_owned()),
            ("name", String::new()),
            ("summary", summary.to_owned()),
            ("cardinality", cardinality.to_owned()),
            ("sparql", sparql.to_string()),
            ("blank", "true".to_owned()),
            ("severity", String::new()),
        ]
    };
    let chip = |kind: &str, term: &str| {
        vec![
            ("kind", kind.to_owned()),
            ("termDisplay", term.to_owned()),
            ("termIri", String::new()),
            ("describedInFile", "false".to_owned()),
            ("shapeIri", String::new()),
            ("predicateIri", String::new()),
        ]
    };
    let elements = vec![
        node(
            "res:PersonShape",
            rdf_scene::SHACL_SHAPE,
            (0.0, 0.0),
            Payload::new()
                .text("iri", "http://example.org/PersonShape")
                .text("display", "ex:PersonShape")
                .flag("blank", false)
                .flag("deactivated", true)
                .text("severity", "sh:Warning")
                .flag("closed", true)
                .text("name", "Person")
                .text("description", "")
                .records(
                    "targets",
                    &[chip("SHACL_TARGET_KIND_PROTO_CLASS", "ex:Person"), chip("5", "")],
                )
                .records(
                    "rows",
                    &[
                        row("ex:name", "xsd:string", "1..1", false),
                        row("", "SELECT $this ...", "", true),
                    ],
                ),
        ),
        node(
            "blank:3",
            rdf_scene::SHACL_SHAPE,
            (320.0, 0.0),
            Payload::new().text("display", "_:b3").flag("blank", true),
        ),
        edge(
            "shacl-edge:1",
            rdf_scene::SHACL_EDGE,
            "res:PersonShape",
            "blank:3",
            Payload::new().text("kind", "node").text("label", "sh:node"),
        ),
        truncation(rdf_scene::SHACL_TRUNCATION, 1, 2),
    ];
    let built = rdf_scene::build("w3c/shacl", &elements, &[], None, false);
    let texts = texts_of(&built.scene);
    for label in [
        "ex:PersonShape — Person",
        "deactivated · closed · sh:Warning",
        "targets class ex:Person",
        "targets its own instances",
        "ex:name",
        "xsd:string",
        "1..1",
        "SPARQL constraint",
        "sh:node",
        "Showing 1 of 2 shapes. Edits are withheld while the view is partial.",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    // 44, two targets and two rows of 22, and 12 below.
    assert_eq!(
        built.rects["res:PersonShape"],
        Rect::new(0.0, 0.0, SHAPE_CARD_WIDTH, 44.0 + 4.0 * 22.0 + 12.0)
    );
    assert_eq!(body_of(&built.scene, "res:PersonShape").opacity, 0.55);
    assert_eq!(
        body_of(&built.scene, "blank:3").stroke.as_ref().unwrap().dash,
        vec![4.0, 3.0]
    );
    assert!(built.anchors.is_empty(), "nothing is connected on the shapes canvas");
    assert_eq!(edge_arrowed(&built.scene, "shacl-edge:1"), (true, vec![]));
}

#[test]
fn a_selected_card_is_outlined_and_hooked_and_a_moving_one_is_carried() {
    let elements = vec![node(
        "res:a",
        rdf_scene::RDF_RESOURCE,
        (10.0, 20.0),
        Payload::new().text("display", "ex:a"),
    )];
    let ids = ["res:a".to_owned()];
    let built = rdf_scene::build("w3c/rdf", &elements, &ids, Some((&ids, (5.0, 5.0))), false);
    let hooked = built
        .scene
        .primitives
        .iter()
        .any(|p| matches!(p, Primitive::Hook(h) if h.kind == HookKind::Selection && h.area.x == 15.0));
    assert!(hooked);
    assert_eq!(
        built.rects["res:a"].x, 10.0,
        "the placed box stays where the module put it"
    );
    assert_eq!(built.boxes[0].1.x, 15.0, "the drawn one follows the drag");
}

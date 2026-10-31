//! Scenes of the vendored diagram types, built from their example bodies: the mind map (`freeplane/mindmap`) and the
//! timeline (`generic/timeline`) read through their FBL bindings from the fixtures under
//! `vendor/etalii.adp/specifications/fbl/fixtures/`, and the dependency graph built by hand, since its one fixture is
//! an empty graph. Positions are what the host's layout would give; the scene draws what the notation says at them.

mod common;

use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_disl::DislDiagram;
use adp_disl::model::builder;
use adp_fbl::{FblOptions, OpenBody};
use adp_scene::connectors::forward_bezier_path;
use adp_scene::definition::ShapeKind;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::geometry::PathCommand;
use adp_scene::geometry::{Path, Point, Rect};
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Owner, Primitive, Role};
use adp_scene::shapes::BuiltInShape;
use adp_scene::{DiagramDefinition, compile_notation};
use common::{element, relation, scene, texts, vendored, with_role};

/// A path's SVG with its coordinates to two decimals, where a fraction of the chord is not exact in binary.
fn round_svg(path: &Path) -> String {
    let r = |p: Point| Point::new((p.x * 100.0).round() / 100.0, (p.y * 100.0).round() / 100.0);
    Path {
        commands: path
            .commands
            .iter()
            .map(|c| match *c {
                PathCommand::MoveTo(p) => PathCommand::MoveTo(r(p)),
                PathCommand::LineTo(p) => PathCommand::LineTo(r(p)),
                PathCommand::CubicTo { first, second, to } => PathCommand::CubicTo {
                    first: r(first),
                    second: r(second),
                    to: r(to),
                },
                other => other,
            })
            .collect(),
    }
    .to_svg()
}

const VENDOR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../vendor/etalii.adp");

/// The diagram a fixture's body reads as, through the named binding of an FBL file.
fn read(definition: &DiagramDefinition, fbl: &str, binding: &str, body: &str) -> DislDiagram {
    let (problems, document) = adp_fbl::documents::load(
        &std::fs::read(format!("{VENDOR}/specifications/fbl/{fbl}")).expect("the binding is there"),
    );
    assert!(problems.is_empty(), "{problems:?}");
    let binding = Rc::new(
        document
            .expect("the binding loads")
            .binding(binding)
            .expect("the binding is named")
            .clone(),
    );
    let bytes = std::fs::read(format!("{VENDOR}/specifications/fbl/fixtures/{body}")).expect("the body is there");
    let reading = OpenBody::open(bytes, binding, Rc::new(FblOptions::default())).model();
    builder::from(&reading, &definition.specification).diagram
}

fn line<'a>(scene: &'a adp_scene::Scene, id: &str) -> &'a EdgePrimitive {
    scene
        .of(&relation(id))
        .find_map(|p| match p {
            Primitive::Edge(e) => Some(e),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no line {id}"))
}

fn body<'a>(scene: &'a adp_scene::Scene, owner: &Owner) -> &'a adp_scene::scene::ShapePrimitive {
    match with_role(scene, owner, &Role::Body) {
        Some(Primitive::Shape(s)) => s,
        other => panic!("no body for {owner:?}: {other:?}"),
    }
}

fn colour(c: &Option<adp_scene::style::Colour>) -> (Option<&str>, Option<&str>) {
    let c = c.as_ref().expect("a colour");
    (c.token.as_deref(), c.value.as_deref())
}

// ---- the mind map --------------------------------------------------------------------------------------------------

#[test]
fn the_mind_map_draws_its_topic_boxes_their_texts_and_its_branches_from_side_to_side() {
    let definition = compile_notation(&vendored("mindmap.dis")).unwrap_or_else(|e| panic!("{e}"));
    let diagram = read(&definition, "mindmap.fbl", "freeplane", "mindmap-nodes/plan.mm");
    assert_eq!(
        vec!["ID_1", "ID_2", "ID_3", "ID_4"],
        diagram.nodes().iter().map(|n| n.id()).collect::<Vec<_>>()
    );
    // Risks is folded for this viewer: its corner glyph says so.
    let mut view = Map::default();
    view.insert("collapsed", Value::Bool(true));
    diagram.element_by_id("ID_3").unwrap().set_view(Some(view));

    let root = Rect::new(-30.0, -16.0, 60.0, 32.0);
    let scope = Rect::new(120.0, -56.0, 60.0, 32.0);
    let risks = Rect::new(-180.0, 24.0, 60.0, 32.0);
    let budget = Rect::new(-330.0, 24.0, 70.0, 32.0);
    let scene = scene(
        &definition,
        &diagram,
        &[("ID_1", root), ("ID_2", scope), ("ID_3", risks), ("ID_4", budget)],
    );

    // The canvas's background, the theme's surface.
    let Primitive::Shape(background) = &scene.primitives[0] else {
        panic!("no background");
    };
    assert_eq!(
        (Owner::Canvas, Role::Background),
        (background.owner.clone(), background.role.clone())
    );
    assert_eq!((Some("color.surface"), Some("#FFFFFF")), colour(&background.fill));

    // A topic box is the notation's own custom shape, its corners rounded by 6.
    assert!(matches!(
        definition.element_type("Node").unwrap().shape,
        ShapeKind::Custom(_)
    ));
    let plan = body(&scene, &element("ID_1"));
    assert_eq!(
        "M -24 -16 L 24 -16 A 6 6 0 0 1 30 -10 L 30 10 A 6 6 0 0 1 24 16 L -24 16 A 6 6 0 0 1 -30 10 L -30 -10 A 6 6 0 0 1 -24 -16 Z",
        plan.path.to_svg()
    );
    assert_eq!((Some("color.surface"), Some("#FFFFFF")), colour(&plan.fill));
    let stroke = plan.stroke.as_ref().unwrap();
    assert_eq!(
        ((Some("color.border"), Some("#E2E8F0")), 1.5),
        (colour(&stroke.colour), stroke.width)
    );

    // Its text, centred and editable, in the style's font.
    let text = &texts(&scene, &element("ID_1"))[0];
    assert_eq!(
        ("Plan", Point::new(0.0, 4.0), TextAnchor::Middle, true),
        (text.text.as_str(), text.position, text.anchor, text.editable)
    );
    assert_eq!(
        (12.0, (Some("color.text"), Some("#0F172A"))),
        (text.font.size, colour(&text.font.colour))
    );
    assert_eq!(Some("system-ui, sans-serif"), text.font.family.as_deref());

    // The corner glyph of a folded branch, right-aligned 4 inside the right edge, 12 below the top; none elsewhere.
    let glyph = texts(&scene, &element("ID_3"))
        .into_iter()
        .find(|t| {
            t.role
                == Role::Label {
                    id: "indicators".to_owned(),
                    line: 0,
                }
        })
        .expect("the folded node's glyph");
    assert_eq!(
        (
            "\u{2295}",
            Point::new(risks.right() - 4.0, risks.y + 12.0),
            TextAnchor::End
        ),
        (glyph.text.as_str(), glyph.position, glyph.anchor)
    );
    assert_eq!(1, texts(&scene, &element("ID_2")).len());

    // A branch leaves the facing side of the parent and reaches the facing side of the child: a horizontal bezier,
    // with no arrowheads, unselectable, in the branch style.
    let branch = line(&scene, "Branch:ID_1->ID_2");
    assert_eq!("M 30 0 C 75 0, 75 -40, 120 -40", branch.path.to_svg());
    assert!(branch.start.is_none() && branch.end.is_none() && !branch.selectable);
    assert_eq!(
        ((Some("color.border"), Some("#E2E8F0")), 1.5),
        (colour(&branch.stroke.colour), branch.stroke.width)
    );
    assert_eq!(
        "M -30 0 C -75 0, -75 40, -120 40",
        line(&scene, "Branch:ID_1->ID_3").path.to_svg()
    );
    assert_eq!(
        "M -180 40 C -220 40, -220 40, -260 40",
        line(&scene, "Branch:ID_3->ID_4").path.to_svg()
    );

    // Lines are painted under the boxes.
    let last_line = scene
        .primitives
        .iter()
        .rposition(|p| matches!(p, Primitive::Edge(_)))
        .unwrap();
    let first_body = scene
        .primitives
        .iter()
        .position(|p| p.role() == Some(&Role::Body))
        .unwrap();
    assert!(last_line < first_body);
}

#[test]
fn the_mind_maps_scene_culls_to_a_viewport_round_the_root() {
    let definition = compile_notation(&vendored("mindmap.dis")).unwrap();
    let diagram = read(&definition, "mindmap.fbl", "freeplane", "mindmap-nodes/plan.mm");
    let positions = [
        ("ID_1", Rect::new(-30.0, -16.0, 60.0, 32.0)),
        ("ID_2", Rect::new(120.0, -56.0, 60.0, 32.0)),
        ("ID_3", Rect::new(-180.0, 24.0, 60.0, 32.0)),
        ("ID_4", Rect::new(-330.0, 24.0, 70.0, 32.0)),
    ];
    let whole = scene(&definition, &diagram, &positions);
    let culled = whole.cull(Rect::new(-40.0, -20.0, 80.0, 40.0), &adp_scene::EstimatedMetrics);
    let owners: Vec<&Owner> = culled.primitives.iter().map(Primitive::owner).collect();
    assert!(owners.contains(&&element("ID_1")));
    assert!(owners.contains(&&relation("Branch:ID_1->ID_2")));
    assert!(!owners.contains(&&element("ID_4")));
    assert!(!owners.contains(&&relation("Branch:ID_3->ID_4")));
}

// ---- the timeline --------------------------------------------------------------------------------------------------

#[test]
fn the_timeline_draws_a_period_a_moment_and_a_relation_from_one_end_to_the_others_begin() {
    let definition = compile_notation(&vendored("timeline.dis")).unwrap_or_else(|e| panic!("{e}"));
    let diagram = read(&definition, "timeline.fbl", "timeline", "timeline-edits/roadmap.tml");
    assert_eq!(
        vec!["discovery", "launch"],
        diagram.nodes().iter().map(|n| n.id()).collect::<Vec<_>>()
    );
    let discovery = diagram.element_by_id("discovery").unwrap();
    let launch = diagram.element_by_id("launch").unwrap();
    let mut attributes = Map::default();
    attributes.insert("label", Value::from("then"));
    diagram
        .add_relation("Connection", "c1", Some(&discovery), Some(&launch), attributes)
        .unwrap();

    // Rows are 60 apart; the moment sits 0.3 of a row lower, its left at its begin.
    let period = Rect::new(0.0, 660.0, 200.0, 36.0);
    let moment = Rect::new(400.0, 498.0, 18.0, 18.0);
    let scene = scene(&definition, &diagram, &[("discovery", period), ("launch", moment)]);

    // A period is DISL's rounded rectangle, at the style's corner radius 4, in the standard surface and border.
    assert_eq!(
        ShapeKind::BuiltIn(BuiltInShape::RoundedRectangle),
        definition.element_type("Period").unwrap().shape
    );
    let box_ = body(&scene, &element("discovery"));
    assert_eq!(Path::rounded_rect(period, 4.0), box_.path);
    assert_eq!((Some("color.surface"), Some("#FFFFFF")), colour(&box_.fill));
    assert_eq!(1.0, box_.stroke.as_ref().unwrap().width);
    let label = &texts(&scene, &element("discovery"))[0];
    assert_eq!(
        ("Discovery", Point::new(100.0, 682.0), TextAnchor::Middle),
        (label.text.as_str(), label.position, label.anchor)
    );

    // A moment is a diamond in the accent, its label beside it.
    let diamond = body(&scene, &element("launch"));
    assert_eq!(
        Path::polygon(&[
            Point::new(409.0, 498.0),
            Point::new(418.0, 507.0),
            Point::new(409.0, 516.0),
            Point::new(400.0, 507.0)
        ]),
        diamond.path
    );
    assert_eq!(Some("color.accent"), colour(&diamond.fill).0);
    let beside = &texts(&scene, &element("launch"))[0];
    assert_eq!(
        ("Launch", Point::new(424.0, 511.0), TextAnchor::Start),
        (beside.text.as_str(), beside.position, beside.anchor)
    );

    // The relation leaves the period's end and reaches the moment's begin, a horizontal bezier ending in a filled
    // arrowhead, with its label above its middle.
    let connection = line(&scene, "c1");
    assert_eq!("M 200 678 C 300 678, 300 507, 400 507", connection.path.to_svg());
    let head = connection.end.as_ref().unwrap();
    assert_eq!(
        (MarkerKind::Arrow, Point::new(400.0, 507.0), 0.0),
        (head.kind, head.tip, head.angle)
    );
    assert_eq!(1.5, connection.stroke.width);
    let then = &texts(&scene, &relation("c1"))[0];
    assert_eq!(("then", Point::new(300.0, 586.5)), (then.text.as_str(), then.position));

    // Drawn the other way round, it loops forward out of the end and back into the begin.
    let mut back = Map::default();
    back.insert("label", Value::from(""));
    diagram
        .add_relation("Connection", "c2", Some(&launch), Some(&discovery), back)
        .unwrap();
    let scene = scene_of(&definition, &diagram, period, moment);
    assert_eq!(
        forward_bezier_path(Point::new(418.0, 507.0), Point::new(0.0, 678.0)),
        line(&scene, "c2").path
    );
    assert_eq!(
        "M 418 507 C 668.8 507, -250.8 678, 0 678",
        round_svg(&line(&scene, "c2").path)
    );
    assert!(texts(&scene, &relation("c2")).is_empty());
}

fn scene_of(definition: &DiagramDefinition, diagram: &DislDiagram, period: Rect, moment: Rect) -> adp_scene::Scene {
    scene(definition, diagram, &[("discovery", period), ("launch", moment)])
}

// ---- the dependency graph ------------------------------------------------------------------------------------------

#[test]
fn the_dependency_graph_draws_its_nodes_and_its_curve_through_the_routing_plugin() {
    let definition = compile_notation(&vendored("dependency-graph.dis")).unwrap_or_else(|e| panic!("{e}"));
    let diagram = DislDiagram::new(&definition.specification);
    let core = diagram
        .add_node("Node", "core", [("label", Value::from("Core"))].into_iter().collect())
        .unwrap();
    let app = diagram.add_node("Node", "app", Map::default()).unwrap();
    let ahead = diagram.add_node("Node", "ahead", Map::default()).unwrap();
    diagram
        .add_relation(
            "DependsOn",
            "d1",
            Some(&app),
            Some(&core),
            [("label", Value::from("uses"))].into_iter().collect(),
        )
        .unwrap();
    diagram
        .add_relation("DependsOn", "d2", Some(&ahead), Some(&app), Map::default())
        .unwrap();

    let app_at = Rect::new(0.0, 0.0, 160.0, 36.0);
    let core_at = Rect::new(300.0, 60.0, 160.0, 36.0);
    let ahead_at = Rect::new(250.0, 120.0, 160.0, 36.0);
    let scene = scene(
        &definition,
        &diagram,
        &[("core", core_at), ("app", app_at), ("ahead", ahead_at)],
    );

    // A node is a rounded rectangle of the radius its shape reference states, labelled by its label or its id.
    assert_eq!(Path::rounded_rect(core_at, 6.0), body(&scene, &element("core")).path);
    assert_eq!("Core", texts(&scene, &element("core"))[0].text);
    assert_eq!("app", texts(&scene, &element("app"))[0].text);

    // The dependency curve: a horizontal bezier between the facing sides while the dependency sits clear to the
    // right, and a forward loop out of the dependent's right into the dependency's left when it sits behind.
    assert_eq!("M 160 18 C 230 18, 230 78, 300 78", line(&scene, "d1").path.to_svg());
    assert_eq!(
        forward_bezier_path(Point::new(410.0, 138.0), Point::new(0.0, 18.0)),
        line(&scene, "d2").path
    );
    assert_eq!(
        "M 410 138 C 656 138, -246 18, 0 18",
        round_svg(&line(&scene, "d2").path)
    );
    let head = line(&scene, "d1").end.as_ref().unwrap();
    assert_eq!((MarkerKind::Arrow, Point::new(300.0, 78.0)), (head.kind, head.tip));
    assert_eq!("uses", texts(&scene, &relation("d1"))[0].text);

    // Without the plugin, its declared fallback draws: a bezier between the anchors nearest each other.
    let positions = [
        ("core".to_owned(), core_at),
        ("app".to_owned(), app_at),
        ("ahead".to_owned(), ahead_at),
    ]
    .into_iter()
    .collect();
    let folding = Default::default();
    let options = adp_scene::SceneOptions {
        selection: &[],
        folding: &folding,
        metrics: &adp_scene::EstimatedMetrics,
        plugins: &(),
        shown: None,
    };
    let fallback = adp_scene::build_scene(
        &definition,
        &common::evaluator(&definition),
        &diagram,
        &positions,
        &options,
    );
    assert_eq!("M 160 18 C 230 18, 230 78, 300 78", line(&fallback, "d1").path.to_svg());
    assert_eq!(
        "M 250 138 C 205 138, 205 18, 160 18",
        line(&fallback, "d2").path.to_svg()
    );
}

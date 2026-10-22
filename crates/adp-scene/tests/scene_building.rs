//! Scenes built from small notations: the drawing half of the standalone host's `compileNotation.curved.test.tsx`,
//! `labels.test.ts` and `compartments.test.ts`, and of `DiagramCanvas`'s own tests where they ask what is drawn, with
//! the same expectations read from the primitives rather than from markup.

mod common;

use adp_cel::{Map, Value};
use adp_disl::{DislDiagram, NewElement};
use adp_scene::connectors::Edge;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::geometry::{Path, PathCommand, Point, Rect};
use adp_scene::notation::bowed_arc::{arc_between, normal_along, point_along};
use adp_scene::scene::{HookKind, Owner, Primitive, Role};
use adp_scene::text::{EstimatedMetrics, LABEL_FONT_SIZE, width_of};
use adp_scene::{DiagramDefinition, compile_notation};
use common::{at, element, loaded, relation, scene, scene_selecting, texts, with_role};
use serde_json::{Value as Json, json};

fn attrs(pairs: &[(&str, Value)]) -> Map {
    pairs.iter().cloned().collect()
}

fn definition_of(spec: &Json) -> DiagramDefinition {
    compile_notation(&loaded(&spec.to_string())).unwrap_or_else(|e| panic!("{e}"))
}

fn edge_of<'a>(scene: &'a adp_scene::Scene, id: &str) -> &'a adp_scene::scene::EdgePrimitive {
    let owner = relation(id);
    scene
        .of(&owner)
        .find_map(|p| match p {
            Primitive::Edge(e) => Some(e),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no line for {id}"))
}

fn path_of<'a>(scene: &'a adp_scene::Scene, owner: &Owner, role: &Role) -> &'a Path {
    match with_role(scene, owner, role) {
        Some(Primitive::Shape(s)) => &s.path,
        other => panic!("no shape for {owner:?} {role:?}: {other:?}"),
    }
}

// ---- curved lines and a body-less node -----------------------------------------------------------------------------

fn curved() -> (DiagramDefinition, DislDiagram) {
    let definition = definition_of(&common::curved_spec());
    let diagram = DislDiagram::new(&definition.specification);
    diagram.add_node("Node", "a", attrs(&[("name", "A".into())])).unwrap();
    diagram.add_node("Node", "b", attrs(&[("name", "B".into())])).unwrap();
    (definition, diagram)
}

fn arrow(diagram: &DislDiagram, from: &str, to: &str, attributes: &[(&str, Value)]) {
    let mut new = NewElement::new("Arrow", &format!("{from}-{to}"));
    new.source = diagram.element_by_id(from);
    new.target = diagram.element_by_id(to);
    new.attributes = attrs(attributes);
    diagram.add(new).unwrap();
}

const A: Rect = Rect::new(-50.0, -20.0, 100.0, 40.0);
const B: Rect = Rect::new(250.0, 80.0, 100.0, 40.0);

#[test]
fn a_curved_line_is_an_arc_bowed_off_the_chord_to_the_left_of_travel_and_the_other_way_where_its_variant_holds() {
    let (definition, diagram) = curved();
    arrow(&diagram, "a", "b", &[]);
    arrow(&diagram, "b", "a", &[("flipped", Value::Bool(true))]);
    arrow(&diagram, "b", "b", &[]);
    let scene = scene(&definition, &diagram, &[("a", A), ("b", B)]);
    assert_eq!(arc_between(A, B, 0.2, 26.0).path, edge_of(&scene, "a-b").path);
    assert_eq!(arc_between(B, A, -0.2, 26.0).path, edge_of(&scene, "b-a").path);
    assert_eq!(arc_between(B, B, 0.2, 30.0).path, edge_of(&scene, "b-b").path);
    // The arrowhead sits on the arc's end, on the target's outline.
    let end = edge_of(&scene, "a-b").end.as_ref().unwrap();
    assert_eq!(arc_between(A, B, 0.2, 26.0).finish, end.tip);
}

#[test]
fn the_bars_cross_the_line_where_their_condition_holds_square_to_the_arc_at_its_apex() {
    let (definition, diagram) = curved();
    arrow(&diagram, "a", "b", &[("delayed", Value::Bool(true))]);
    arrow(&diagram, "b", "a", &[("delayed", Value::Bool(false))]);
    let scene = scene(&definition, &diagram, &[("a", A), ("b", B)]);
    let arc = arc_between(A, B, 0.2, 26.0);
    let mut expected = Path::new();
    for t in [0.44, 0.56] {
        let p = point_along(&arc, t);
        expected = expected
            .move_to(Point::new(p.x - arc.apex_normal.x * 8.0, p.y - arc.apex_normal.y * 8.0))
            .line_to(Point::new(p.x + arc.apex_normal.x * 8.0, p.y + arc.apex_normal.y * 8.0));
    }
    assert_eq!(&expected, path_of(&scene, &relation("a-b"), &Role::Bars));
    assert!(with_role(&scene, &relation("b-a"), &Role::Bars).is_none());
}

#[test]
fn a_mark_is_written_beside_the_line_at_a_fraction_along_it_and_nothing_where_the_mark_is_empty() {
    let (definition, diagram) = curved();
    arrow(&diagram, "a", "b", &[("sign", "up".into())]);
    arrow(&diagram, "b", "a", &[("sign", "down".into())]);
    let scene = scene(&definition, &diagram, &[("a", A), ("b", B)]);
    let arc = arc_between(A, B, 0.2, 26.0);
    let marks: Vec<_> = texts(&scene, &relation("a-b"))
        .into_iter()
        .filter(|t| t.role == Role::Mark)
        .collect();
    assert_eq!(1, marks.len());
    assert_eq!("+", marks[0].text);
    assert_eq!(
        Point::new(
            point_along(&arc, 0.86).x + normal_along(&arc, 0.86).x * 11.0,
            point_along(&arc, 0.86).y + normal_along(&arc, 0.86).y * 11.0
        ),
        marks[0].position
    );
    assert!(texts(&scene, &relation("b-a")).iter().all(|t| t.role != Role::Mark));
}

#[test]
fn the_stroke_width_is_the_one_the_lines_cel_chooses() {
    let (definition, diagram) = curved();
    arrow(&diagram, "a", "b", &[("weight", Value::Double(3.0))]);
    arrow(&diagram, "b", "a", &[("weight", Value::Double(0.2))]);
    arrow(&diagram, "a", "a", &[]);
    arrow(&diagram, "b", "b", &[("weight", Value::Double(1.0))]);
    let scene = scene(&definition, &diagram, &[("a", A), ("b", B)]);
    let widths: Vec<f64> = ["a-b", "b-a", "a-a", "b-b"]
        .iter()
        .map(|id| edge_of(&scene, id).stroke.width)
        .collect();
    assert_eq!(vec![2.5, 1.0, 1.5, 1.5], widths);
}

#[test]
fn a_body_less_node_stands_at_the_centre_of_its_members_its_icon_above_that_point_and_its_line_on_it() {
    let (definition, diagram) = curved();
    for (id, turn) in [("h1", "on"), ("h2", "back")] {
        diagram
            .add_node(
                "Hub",
                id,
                attrs(&[
                    ("caption", id.into()),
                    ("members", Value::list(vec!["a".into(), "b".into()])),
                    ("turn", turn.into()),
                ]),
            )
            .unwrap();
    }
    let a = at(0.0, 0.0, 100.0, 40.0);
    let b = at(300.0, 100.0, 100.0, 40.0);
    let scene = scene(&definition, &diagram, &[("a", a), ("b", b)]);
    // Nothing of the body is drawn; the caption sits where a centred line at (150, 50) sits.
    assert!(with_role(&scene, &element("h1"), &Role::Body).is_none());
    let caption = texts(&scene, &element("h1"))
        .into_iter()
        .find(|t| t.text == "h1")
        .unwrap();
    assert_eq!(
        (Point::new(150.0, 54.0), TextAnchor::Middle),
        (caption.position, caption.anchor)
    );
    let sweep = |clockwise: bool| {
        adp_scene::definition::decorations::circular_arrow_path(Point::new(150.0, 35.0), 13.0, clockwise)
    };
    let icon = |id: &str| path_of(&scene, &element(id), &Role::Decoration("mdi-sync".to_owned())).clone();
    assert_eq!(sweep(true), icon("h1"));
    assert_eq!(sweep(false), icon("h2"));
}

// ---- bodies, styles and hooks ----------------------------------------------------------------------------------------

fn cards_spec() -> Json {
    json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "types": { "Card": { "attributes": { "name": { "type": "string" }, "note": { "type": "string" }, "hot": { "type": "bool" } } } },
            "relations": { "Link": { "source": "Card", "target": { "types": ["Card"], "optional": true }, "attributes": { "label": { "type": "string" } } } }
        },
        "notation": {
            "theme": { "tokens": { "color.surface": "#FAFAFA", "color.border": "#333333", "color.text": "#111111", "color.danger": "#FF0000" },
                       "modes": { "dark": { "color.surface": "#000000" } } },
            "styles": { "card": { "fill": { "token": "color.surface" }, "stroke": { "color": { "token": "color.border" }, "width": 1.5 }, "font": { "size": 13 } } },
            "nodes": {
                "Card": {
                    "shape": "rect",
                    "style": "card",
                    "conditions": [{ "when": "self.hot", "style": { "stroke": { "color": { "token": "color.danger" } } } }],
                    "size": { "default": [160, 40], "resizable": "horizontal" },
                    "labels": [
                        { "id": "name", "text": { "attribute": "name" }, "position": "center", "overflow": "ellipsis", "editable": "inline" },
                        { "id": "note", "text": { "cel": "self.note" }, "visible": { "cel": "self.note != ''" }, "position": "outside-right" }
                    ],
                    "anchors": { "mode": "fixed", "points": [{ "id": "in", "x": 0, "y": 0.5 }, { "id": "out", "x": 1, "y": 0.5 }] }
                }
            },
            "edges": {
                "Link": {
                    "line": { "routing": "straight", "stroke": { "width": 2 } },
                    "anchoring": { "source": { "mode": "fixed", "points": [{ "id": "out", "x": 1, "y": 0.5 }] }, "target": { "mode": "fixed", "points": [{ "id": "in", "x": 0, "y": 0.5 }] } },
                    "labels": [{ "id": "label", "text": { "attribute": "label" }, "at": "middle", "side": "above", "distance": 6 }],
                    "stub": { "length": 48, "side": "right", "style": { "stroke": { "dash": [4, 2], "dashScale": "absolute" } },
                              "label": { "id": "missing", "text": { "cel": "self.label + ' (missing)'" }, "side": "above" } }
                }
            },
            "canvas": { "background": { "token": "color.surface" } }
        }
    })
}

fn cards() -> (DiagramDefinition, DislDiagram) {
    let definition = definition_of(&cards_spec());
    let diagram = DislDiagram::new(&definition.specification);
    diagram
        .add_node("Card", "c1", attrs(&[("name", "First".into()), ("note", "".into())]))
        .unwrap();
    diagram
        .add_node(
            "Card",
            "c2",
            attrs(&[
                ("name", "A name far too long for its box".into()),
                ("note", "noted".into()),
                ("hot", Value::Bool(true)),
            ]),
        )
        .unwrap();
    let mut link = NewElement::new("Link", "l1");
    link.source = diagram.element_by_id("c1");
    link.target = diagram.element_by_id("c2");
    link.attributes = attrs(&[("label", "uses".into())]);
    diagram.add(link).unwrap();
    let mut open = NewElement::new("Link", "l2");
    open.source = diagram.element_by_id("c2");
    open.target_id = Some("nowhere".to_owned());
    open.attributes = attrs(&[("label", "nowhere".into())]);
    diagram.add(open).unwrap();
    (definition, diagram)
}

const C1: Rect = Rect::new(0.0, 0.0, 160.0, 40.0);
const C2: Rect = Rect::new(300.0, 100.0, 160.0, 40.0);

#[test]
fn a_body_is_painted_in_its_styles_tokens_with_a_condition_that_holds_merged_over_it() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1), ("c2", C2)]);
    let body = |id: &str| match with_role(&scene, &element(id), &Role::Body) {
        Some(Primitive::Shape(s)) => s.clone(),
        other => panic!("{other:?}"),
    };
    let first = body("c1");
    // DISL's rect is the library's box, rounded by 4 where no radius is stated.
    assert_eq!(Path::rounded_rect(C1, 4.0), first.path);
    assert_eq!(
        (Some("color.surface"), Some("#FAFAFA")),
        (
            first.fill.as_ref().unwrap().token.as_deref(),
            first.fill.as_ref().unwrap().value.as_deref()
        )
    );
    let stroke = first.stroke.unwrap();
    assert_eq!(
        (Some("#333333"), 1.5),
        (stroke.colour.unwrap().value.as_deref(), stroke.width)
    );
    assert_eq!(
        Some("#FF0000"),
        body("c2").stroke.unwrap().colour.unwrap().value.as_deref()
    );
}

#[test]
fn a_theme_mode_repaints_by_token() {
    let (definition, diagram) = cards();
    let evaluator = adp_scene::Evaluator::new(
        &definition.specification,
        adp_scene::Theme::of(&definition.specification, Some("dark")),
        adp_disl::DislEnv::default(),
    );
    let positions = [("c1".to_owned(), C1)].into_iter().collect();
    let folding = Default::default();
    let options = adp_scene::SceneOptions {
        selection: &[],
        folding: &folding,
        metrics: &EstimatedMetrics,
        plugins: &(),
    };
    let scene = adp_scene::build_scene(&definition, &evaluator, &diagram, &positions, &options);
    let Some(Primitive::Shape(body)) = with_role(&scene, &element("c1"), &Role::Body) else {
        panic!("no body");
    };
    assert_eq!(Some("#000000"), body.fill.as_ref().unwrap().value.as_deref());
}

#[test]
fn labels_are_centred_trimmed_to_the_box_and_drawn_only_while_visible() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1), ("c2", C2)]);
    let first = texts(&scene, &element("c1"));
    assert_eq!(1, first.len());
    assert_eq!(
        ("First", Point::new(80.0, 24.0), TextAnchor::Middle, 13.0),
        (
            first[0].text.as_str(),
            first[0].position,
            first[0].anchor,
            first[0].font.size
        )
    );
    assert!(first[0].editable);
    assert_eq!(Some("#111111"), first[0].font.colour.as_ref().unwrap().value.as_deref());
    let second = texts(&scene, &element("c2"));
    let name = second
        .iter()
        .find(|t| {
            t.role
                == Role::Label {
                    id: "name".to_owned(),
                    line: 0,
                }
        })
        .unwrap();
    assert!(name.text.ends_with('…'));
    assert!(width_of(&name.text, 13.0) <= 160.0 - 8.0);
    let note = second
        .iter()
        .find(|t| {
            t.role
                == Role::Label {
                    id: "note".to_owned(),
                    line: 0,
                }
        })
        .unwrap();
    assert_eq!(
        ("noted", Point::new(466.0, 124.0), TextAnchor::Start),
        (note.text.as_str(), note.position, note.anchor)
    );
}

#[test]
fn a_line_runs_between_its_named_anchors_with_its_arrowhead_and_its_label_above_its_middle() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1), ("c2", C2)]);
    let line = edge_of(&scene, "l1");
    assert_eq!("M 160 20 L 300 120", line.path.to_svg());
    assert_eq!(2.0, line.stroke.width);
    let end = line.end.as_ref().unwrap();
    assert_eq!((Point::new(300.0, 120.0), true), (end.tip, end.filled));
    assert!(line.start.is_none());
    let label = texts(&scene, &relation("l1"));
    assert_eq!(
        ("uses", Point::new(230.0, 64.0), 11.0),
        (label[0].text.as_str(), label[0].position, label[0].font.size)
    );
}

#[test]
fn a_relation_without_a_target_is_drawn_as_its_stub_in_its_own_style() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1), ("c2", C2)]);
    let stub = edge_of(&scene, "l2");
    assert_eq!("M 460 120 L 508 120", stub.path.to_svg());
    assert_eq!(vec![4.0, 2.0], stub.stroke.dash);
    assert!(stub.end.is_none());
    let label = texts(&scene, &relation("l2"));
    assert_eq!(
        ("nowhere (missing)", Point::new(468.0, 114.0), TextAnchor::Start),
        (label[0].text.as_str(), label[0].position, label[0].anchor)
    );
}

#[test]
fn the_background_comes_first_lines_before_elements_and_hooks_after_each_elements_drawing() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1), ("c2", C2)]);
    assert_eq!(Owner::Canvas, *scene.primitives[0].owner());
    let first_element = scene
        .primitives
        .iter()
        .position(|p| matches!(p.owner(), Owner::Element(_)))
        .unwrap();
    let last_line = scene
        .primitives
        .iter()
        .rposition(|p| matches!(p.owner(), Owner::Relation(_)))
        .unwrap();
    assert!(last_line < first_element);
    let anchors: Vec<String> = scene
        .of(&element("c1"))
        .filter_map(|p| match p {
            Primitive::Hook(h) => match &h.kind {
                HookKind::Anchor { name, .. } => Some(name.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(vec!["in", "out"], anchors);
}

#[test]
fn a_selection_is_outlined_in_the_selected_colour_heavier_than_the_notation_and_offers_its_resize_handles() {
    let (definition, diagram) = cards();
    let scene = scene_selecting(&definition, &diagram, &[("c1", C1), ("c2", C2)], &["c1", "l1"]);
    let Some(Primitive::Shape(body)) = with_role(&scene, &element("c1"), &Role::Body) else {
        panic!("no body");
    };
    let stroke = body.stroke.as_ref().unwrap();
    assert_eq!(
        (Some("color.selected"), 3.0),
        (stroke.colour.as_ref().unwrap().token.as_deref(), stroke.width)
    );
    assert_eq!(3.0, edge_of(&scene, "l1").stroke.width);
    let hooks: Vec<HookKind> = scene
        .of(&element("c1"))
        .filter_map(|p| match p {
            Primitive::Hook(h) => Some(h.kind.clone()),
            _ => None,
        })
        .filter(|k| !matches!(k, HookKind::Anchor { .. }))
        .collect();
    assert_eq!(
        vec![
            HookKind::Selection,
            HookKind::ResizeHandle(Edge::Left),
            HookKind::ResizeHandle(Edge::Right)
        ],
        hooks
    );
}

#[test]
fn culling_keeps_what_meets_the_viewport_in_order_and_always_the_background() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1), ("c2", C2)]);
    let culled = scene.cull(Rect::new(-10.0, -10.0, 100.0, 40.0), &EstimatedMetrics);
    assert_eq!(Owner::Canvas, *culled.primitives[0].owner());
    assert!(culled.primitives.iter().any(|p| p.owner() == &element("c1")));
    assert!(culled.primitives.iter().all(|p| p.owner() != &element("c2")));
    // The line from c1 to c2 crosses the viewport's corner region only by its hit width; it is kept where its bounds
    // meet the viewport.
    let line_kept = culled.primitives.iter().any(|p| p.owner() == &relation("l1"));
    assert_eq!(
        edge_of(&scene, "l1")
            .path
            .bounds()
            .unwrap()
            .inflate(7.0)
            .intersects(&Rect::new(-10.0, -10.0, 100.0, 40.0)),
        line_kept
    );
    let everything = scene.cull(Rect::new(-1000.0, -1000.0, 3000.0, 3000.0), &EstimatedMetrics);
    assert_eq!(scene.primitives.len(), everything.primitives.len());
    let nothing = scene.cull(Rect::new(5000.0, 5000.0, 10.0, 10.0), &EstimatedMetrics);
    assert_eq!(1, nothing.primitives.len());
}

#[test]
fn an_element_without_a_position_is_not_drawn_nor_are_its_lines() {
    let (definition, diagram) = cards();
    let scene = scene(&definition, &diagram, &[("c1", C1)]);
    assert!(scene.of(&element("c2")).next().is_none());
    assert!(scene.of(&relation("l1")).next().is_none());
}

// ---- labels (`labels.test.ts`) -------------------------------------------------------------------------------------

fn labelled(labels: Json) -> (DiagramDefinition, DislDiagram) {
    let spec = json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": { "types": { "Card": { "attributes": { "name": { "type": "string" }, "rows": { "type": "string", "many": true } } } } },
        "notation": { "nodes": { "Card": { "shape": "rect", "labels": labels } } }
    });
    let definition = definition_of(&spec);
    let diagram = DislDiagram::new(&definition.specification);
    diagram
        .add_node(
            "Card",
            "c",
            attrs(&[
                ("name", "Pipeline.Core".into()),
                ("rows", Value::list(vec!["one".into(), "two".into()])),
            ]),
        )
        .unwrap();
    (definition, diagram)
}

const BOX: Rect = Rect::new(-60.0, -20.0, 120.0, 40.0);

fn lines(labels: Json) -> Vec<adp_scene::scene::TextRun> {
    let (definition, diagram) = labelled(labels);
    let scene = scene(&definition, &diagram, &[("c", BOX)]);
    texts(&scene, &element("c")).into_iter().cloned().collect()
}

#[test]
fn one_centred_line_is_drawn_from_an_attribute_and_nothing_where_it_gives_nothing() {
    let out = lines(json!([{ "id": "n", "text": { "attribute": "name" } }]));
    assert_eq!(1, out.len());
    assert_eq!(
        ("Pipeline.Core", 0.0, TextAnchor::Middle),
        (out[0].text.as_str(), out[0].position.x, out[0].anchor)
    );
    assert!(lines(json!([{ "id": "n", "text": { "cel": "null" } }])).is_empty());
}

#[test]
fn labels_are_placed_above_below_beside_and_before_the_shape() {
    let one =
        |position: &str| lines(json!([{ "id": "n", "text": { "attribute": "name" }, "position": position }])).remove(0);
    let before = one("outside-left");
    assert_eq!(
        (TextAnchor::End, BOX.x - 8.0, BOX.y + BOX.height / 2.0 + 4.0),
        (before.anchor, before.position.x, before.position.y)
    );
    let beside = one("outside-right");
    assert!(beside.anchor == TextAnchor::Start && beside.position.x > BOX.right());
}

#[test]
fn a_list_label_is_one_line_per_entry_and_no_line_of_it_is_editable() {
    let out = lines(json!([{ "id": "n", "text": { "cel": "self.rows" }, "editable": "inline" }]));
    assert_eq!(
        vec!["one", "two"],
        out.iter().map(|l| l.text.as_str()).collect::<Vec<_>>()
    );
    assert!(out.iter().all(|l| !l.editable));
    assert!(lines(json!([{ "id": "n", "text": { "attribute": "name" }, "editable": true }]))[0].editable);
}

#[test]
fn a_label_is_trimmed_only_when_asked() {
    let long = "x".repeat(200);
    let spec_label =
        |overflow: &str| json!([{ "id": "n", "text": { "cel": format!("'{long}'") }, "overflow": overflow }]);
    assert_eq!(200, lines(spec_label("visible"))[0].text.chars().count());
    let cut = lines(spec_label("ellipsis")).remove(0);
    assert!(cut.text.ends_with('…') && cut.text.chars().count() < 40);
}

#[test]
fn an_aligned_label_sits_at_its_inset_and_an_offset_is_measured_from_the_edge_it_names() {
    let right = lines(json!([{ "id": "n", "text": { "attribute": "name" }, "position": { "anchor": [1, 0], "offset": [-8, 18], "align": "end" } }])).remove(0);
    assert_eq!(
        (TextAnchor::End, BOX.right() - 8.0, BOX.y + 18.0),
        (right.anchor, right.position.x, right.position.y)
    );
    let left = lines(json!([{ "id": "n", "text": { "attribute": "name" }, "position": { "anchor": [0, 0], "offset": [12, 18], "align": "start" } }])).remove(0);
    assert_eq!(
        (TextAnchor::Start, BOX.x + 12.0, BOX.y + 18.0),
        (left.anchor, left.position.x, left.position.y)
    );
    let centred = lines(json!([{ "id": "n", "text": { "attribute": "name" }, "position": { "anchor": [0.5, 0.5], "offset": [0, 22] } }])).remove(0);
    assert_eq!(
        (TextAnchor::Middle, 0.0, 22.0),
        (centred.anchor, centred.position.x, centred.position.y)
    );
    let footer = lines(
        json!([{ "id": "n", "text": { "attribute": "name" }, "position": { "anchor": [0.5, 1], "offset": [0, -6] } }]),
    )
    .remove(0);
    assert_eq!(BOX.bottom() - 6.0, footer.position.y);
}

#[test]
fn a_wrapped_label_breaks_into_lines_inside_the_shapes_text_region_and_says_where_it_ran_out() {
    let label = json!([{ "id": "n", "text": { "cel": "'one two three four five six seven'" }, "wrap": "word" }]);
    let (definition, diagram) = labelled(label.clone());
    let scene = scene(&definition, &diagram, &[("c", Rect::new(0.0, 0.0, 120.0, 80.0))]);
    let out = texts(&scene, &element("c"));
    assert_eq!(
        vec!["one two three", "four five six", "seven"],
        out.iter().map(|l| l.text.as_str()).collect::<Vec<_>>()
    );
    let region = out[0].wrap.unwrap();
    for line in &out {
        assert!(
            width_of(&line.text, LABEL_FONT_SIZE) <= region.width + 1e-9,
            "{}",
            line.text
        );
    }
    // The lines are round(12 * 1.4) = 17 apart.
    assert_eq!(
        vec![17.0, 17.0],
        out.windows(2)
            .map(|w| w[1].position.y - w[0].position.y)
            .collect::<Vec<_>>()
    );

    // In the 40-high box only one line fits: it ends in an ellipsis, and the whole text is its tooltip.
    let short = lines(label);
    assert_eq!(1, short.len());
    assert!(short[0].text.ends_with('…'));
    assert_eq!(Some("one two three four five six seven"), short[0].tooltip.as_deref());
    assert_eq!(Rect::new(BOX.x + 6.0, BOX.y + 6.0, 108.0, 28.0), short[0].wrap.unwrap());
}

// ---- lists (`compartments.test.ts`) --------------------------------------------------------------------------------

fn listed() -> (DiagramDefinition, DislDiagram) {
    let spec = json!({
        "disl": "0.4",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "enums": { "State": { "values": { "open": { "label": "Open" }, "done": { "label": "Done" } } } },
            "types": {
                "Card": { "attributes": { "name": { "type": "string" }, "note": { "type": "string" } }, "children": { "allowed": ["Item"] } },
                "Item": { "attributes": { "title": { "type": "string" }, "state": { "type": "State" }, "updated": { "type": "string" } } }
            }
        },
        "notation": {
            "nodes": {
                "Card": {
                    "shape": "rect",
                    "labels": [{ "id": "name", "text": { "attribute": "name" }, "position": "top" }],
                    "compartments": [
                        { "id": "note", "items": { "cel": "[self.note]" }, "itemText": { "cel": "item" } },
                        { "id": "items", "items": { "children": ["Item"], "slot": "items" }, "itemText": { "cel": "item.title" },
                          "itemOrder": { "by": "item.updated", "direction": "descending" }, "groupBy": { "attribute": "state", "collapsed": { "done": true } } }
                    ]
                },
                "Item": { "shape": "rect" }
            }
        }
    });
    let definition = definition_of(&spec);
    let diagram = DislDiagram::new(&definition.specification);
    let card = diagram
        .add_node(
            "Card",
            "c",
            attrs(&[("name", "Card".into()), ("note", "A note".into())]),
        )
        .unwrap();
    for (id, title, state, updated) in [
        ("i1", "Older", "open", "2026-01-01"),
        ("i2", "Newer", "open", "2026-02-01"),
        ("i3", "Shipped", "done", "2026-03-01"),
    ] {
        diagram
            .add_child(
                "Item",
                id,
                attrs(&[
                    ("title", title.into()),
                    ("state", state.into()),
                    ("updated", updated.into()),
                ]),
                &card,
                Some("items"),
            )
            .unwrap();
    }
    (definition, diagram)
}

#[test]
fn a_nodes_lists_are_drawn_beneath_its_labels_grouped_ordered_and_folded_as_declared() {
    let (definition, diagram) = listed();
    let scene = scene(&definition, &diagram, &[("c", Rect::new(0.0, 0.0, 200.0, 40.0))]);
    let drawn: Vec<(String, f64)> = texts(&scene, &element("c"))
        .into_iter()
        .filter(|t| !matches!(t.role, Role::Label { .. }))
        .map(|t| (t.text.clone(), t.position.y))
        .collect();
    // The headless note at 32 (beneath the top line at 20, a gap of 12), then the groups: Open with its rows newest
    // first, and Done folded. Every heading is drawn before the rows, as the standalone draws them.
    assert_eq!(
        vec![
            ("\u{25BE} Open (2)".to_owned(), 50.0 + 10.0 + 4.0),
            ("\u{25B8} Done (1)".to_owned(), 106.0 + 10.0 + 4.0),
            ("A note".to_owned(), 32.0 + 9.0 + 4.0),
            ("Newer".to_owned(), 70.0 + 9.0 + 4.0),
            ("Older".to_owned(), 88.0 + 9.0 + 4.0),
        ],
        drawn
    );
    // The element is as tall as its lists need: the last heading ends at 126, and the list's bottom inset is 8.
    let Some(Primitive::Shape(body)) = with_role(&scene, &element("c"), &Role::Body) else {
        panic!("no body");
    };
    assert_eq!(134.0, body.path.bounds().unwrap().height);
    // The items are not drawn as nodes of their own: they have no position.
    assert!(scene.of(&element("i1")).next().is_none());
}

#[test]
fn a_heading_the_reader_opened_shows_its_rows() {
    let (definition, diagram) = listed();
    let mut folding = adp_scene::definition::compartments::Folding::default();
    folding.opened.insert(("items".to_owned(), "done".to_owned()));
    let positions = [("c".to_owned(), Rect::new(0.0, 0.0, 200.0, 40.0))]
        .into_iter()
        .collect();
    let options = adp_scene::SceneOptions {
        selection: &[],
        folding: &folding,
        metrics: &EstimatedMetrics,
        plugins: &(),
    };
    let scene = adp_scene::build_scene(
        &definition,
        &common::evaluator(&definition),
        &diagram,
        &positions,
        &options,
    );
    assert!(texts(&scene, &element("c")).iter().any(|t| t.text == "Shipped"));
    assert!(
        texts(&scene, &element("c"))
            .iter()
            .any(|t| t.text == "\u{25BE} Done (1)")
    );
}

#[test]
fn a_custom_shape_is_drawn_from_its_own_geometry() {
    let spec = json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": { "types": { "Flag": {} } },
        "notation": {
            "shapes": { "pennant": { "params": { "depth": { "type": "number", "default": 10 } },
                "path": { "segments": [{ "op": "M", "x": 0, "y": 0 }, { "op": "L", "x": "w", "y": "h / 2.0" }, { "op": "L", "x": 0, "y": "h" }, { "op": "L", "x": "p.depth", "y": "50%" }, { "op": "Z" }] } } },
            "nodes": { "Flag": { "shape": { "type": "pennant", "params": { "depth": 20 } } } }
        }
    });
    let definition = definition_of(&spec);
    let diagram = DislDiagram::new(&definition.specification);
    diagram.add_node("Flag", "f", Map::default()).unwrap();
    let scene = scene(&definition, &diagram, &[("f", Rect::new(10.0, 10.0, 100.0, 40.0))]);
    assert_eq!(
        "M 10 10 L 110 30 L 10 50 L 30 30 Z",
        path_of(&scene, &element("f"), &Role::Body).to_svg()
    );
    assert!(matches!(
        path_of(&scene, &element("f"), &Role::Body).commands[0],
        PathCommand::MoveTo(_)
    ));
}

// ---- a scene drawn again from its pieces -------------------------------------------------------------------------------

/// The scene of `diagram` at `before` and then at `after`, the second drawn again from the first's pieces, with the
/// numbers of the second's pieces; and the scene `after` draws when drawn from nothing.
fn drawn_twice(
    definition: &DiagramDefinition,
    diagram: &DislDiagram,
    before: &[(&str, Rect)],
    after: &[(&str, Rect)],
    selection: &[&str],
) -> (Vec<u64>, Vec<u64>, adp_scene::Scene, adp_scene::Scene) {
    let evaluator = common::evaluator(definition);
    let folding = Default::default();
    let selection: Vec<String> = selection.iter().map(|s| (*s).to_owned()).collect();
    let options = adp_scene::SceneOptions {
        selection: &selection,
        folding: &folding,
        metrics: &EstimatedMetrics,
        plugins: &adp_scene::scene::StandardRoutePlugins,
    };
    let positions = |at: &[(&str, Rect)]| at.iter().map(|(id, r)| ((*id).to_owned(), *r)).collect();
    let first =
        adp_scene::scene::build_scene_pieces(definition, &evaluator, diagram, &positions(before), &options, None);
    let numbers = |pieces: &adp_scene::scene::ScenePieces| pieces.iter().map(|(n, _)| n).collect::<Vec<_>>();
    let earlier = numbers(&first);
    let second = adp_scene::scene::build_scene_pieces(
        definition,
        &evaluator,
        diagram,
        &positions(after),
        &options,
        Some(first),
    );
    let redrawn = second.scene(definition, &evaluator, diagram, &EstimatedMetrics);
    let fresh = adp_scene::build_scene(definition, &evaluator, diagram, &positions(after), &options);
    (earlier, numbers(&second), redrawn, fresh)
}

#[test]
fn a_scene_drawn_again_with_an_element_moved_draws_again_only_it_and_its_lines_and_is_the_scene_drawn_from_nothing() {
    // The cards: a background round everything, a line between the two, a stub from the second, the first selected.
    let (definition, diagram) = cards();
    let moved = Rect::new(320.0, 180.0, 160.0, 40.0);
    let (earlier, later, redrawn, fresh) = drawn_twice(
        &definition,
        &diagram,
        &[("c1", C1), ("c2", C2)],
        &[("c1", C1), ("c2", moved)],
        &["c1"],
    );
    assert_eq!(fresh, redrawn);
    // The pieces: l1, l2, c1, c2. The first card is taken over; both lines and the second card are drawn again.
    assert_eq!(4, later.len());
    assert_eq!(
        vec![false, false, true, false],
        earlier.iter().zip(&later).map(|(a, b)| a == b).collect::<Vec<_>>()
    );

    // Nothing moved: every piece is taken over.
    let (earlier, later, redrawn, fresh) = drawn_twice(
        &definition,
        &diagram,
        &[("c1", C1), ("c2", C2)],
        &[("c1", C1), ("c2", C2)],
        &[],
    );
    assert_eq!(fresh, redrawn);
    assert_eq!(earlier, later);
}

#[test]
fn a_node_at_the_centre_of_its_members_is_drawn_again_with_them() {
    let (definition, diagram) = curved();
    diagram
        .add_node(
            "Hub",
            "h1",
            attrs(&[
                ("caption", "h1".into()),
                ("members", Value::list(vec!["a".into(), "b".into()])),
                ("turn", "on".into()),
            ]),
        )
        .unwrap();
    arrow(&diagram, "a", "b", &[]);
    let (earlier, later, redrawn, fresh) =
        drawn_twice(&definition, &diagram, &[("a", A), ("b", B)], &[("a", A), ("b", A)], &[]);
    assert_eq!(fresh, redrawn);
    // The pieces: the arrow, a, b and the hub; only a is taken over.
    assert_eq!(
        vec![false, true, false, false],
        earlier.iter().zip(&later).map(|(a, b)| a == b).collect::<Vec<_>>()
    );
}

// ---- budgets --------------------------------------------------------------------------------------------------------

/// What a truncating budget cuts is not drawn (DISL section 3.2.1): a node it leaves out, and a line one of whose ends
/// it leaves out; what it keeps is drawn as before.
#[test]
fn a_truncating_budget_leaves_out_what_it_cuts_and_the_lines_to_it() {
    let mut spec = common::curved_spec();
    spec["language"]["limits"] = json!({ "budgets": { "nodes": { "measure": { "count": ["Node"] }, "max": 2 } } });
    let definition = definition_of(&spec);
    let diagram = DislDiagram::new(&definition.specification);
    for id in ["a", "b", "c"] {
        diagram.add_node("Node", id, attrs(&[("name", id.into())])).unwrap();
    }
    arrow(&diagram, "a", "b", &[]);
    arrow(&diagram, "b", "c", &[]);
    let c = Rect::new(500.0, 0.0, 100.0, 40.0);
    let scene = scene(&definition, &diagram, &[("a", A), ("b", B), ("c", c)]);
    assert!(scene.of(&element("a")).next().is_some());
    assert!(scene.of(&element("b")).next().is_some());
    assert!(scene.of(&element("c")).next().is_none(), "c is cut");
    assert!(scene.of(&relation("a-b")).next().is_some());
    assert!(
        scene.of(&relation("b-c")).next().is_none(),
        "a line to what is cut is not drawn"
    );
}

//! The compiler's own rules (the standalone host's `compileNotation.test.ts`, `compileNotation.lists.test.ts` and
//! the compiling half of `compileNotation.curved.test.tsx`), on specifications small enough to read in one go: what
//! each mapping gives and what it refuses. The expectations are the standalone's, with a binding kept as the
//! specification's attribute or CEL where the standalone states a payload path; the standalone's refusals of CEL a
//! module's tables do not name have no counterpart, since the scene evaluates CEL as written.

mod common;

use adp_scene::connectors::Edge;
use adp_scene::definition::compartments::CompartmentItems;
use adp_scene::definition::decorations::{DecorationOrigin, Glyph};
use adp_scene::definition::labels::{AnchorTo, LabelPlacement, TextAnchor};
use adp_scene::definition::{
    AnchorKind, Binding, EdgeSides, EndpointAnchors, PerPair, Resize, Route, RouteLabelPlacement, ShapeKind,
    SideFraction, Sizing,
};
use adp_scene::geometry::Point;
use adp_scene::notation::catalog::{BuiltInRoute, MarkerKind};
use adp_scene::shapes::BuiltInShape;
use common::{compiled, refusal};
use serde_json::{Value, json};

/// `base` with `pointer` set to `value`, creating objects on the way.
fn with(mut base: Value, pointer: &str, value: Value) -> Value {
    let mut at = &mut base;
    for key in pointer.trim_start_matches('/').split('/') {
        if at.get(key).is_none() {
            at[key] = json!({});
        }
        at = at.get_mut(key).unwrap();
    }
    *at = value;
    base
}

fn spec() -> Value {
    json!({
        "disl": "0.2",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "types": {
                "Card": { "attributes": { "name": { "type": "string" }, "when": { "type": "string" } } },
                "Box": { "children": { "allowed": ["Card"] } }
            },
            "relations": { "Link": { "source": ["Card", "Box"], "target": "Card", "directed": true, "allowSelfLoops": false, "allowParallel": false } }
        },
        "notation": {
            "nodes": {
                "Card": {
                    "shape": "rect",
                    "labels": [{ "id": "name", "text": { "cel": "self.name + ' (' + self.when + ')'" }, "position": "outside-left", "editable": "inline" }],
                    "tooltip": { "attribute": "name" },
                    "anchors": { "mode": "sides", "sides": ["left", "right"] },
                    "size": { "default": [80, 40], "resizable": true }
                },
                "Box": { "shape": { "type": "hexagon", "params": { "inset": 0.25 } }, "size": { "fixed": [40, 40] }, "connectable": false, "placement": { "movable": false } }
            },
            "edges": { "Link": { "line": { "routing": "orthogonal" }, "targetMarker": "arrow" } }
        }
    })
}

fn text(json: &Value) -> String {
    json.to_string()
}

#[test]
fn nodes_labels_anchors_sizes_and_placement_are_mapped() {
    let definition = compiled(&text(&spec()));
    let card = &definition.element_types[0];
    assert_eq!("Card", card.name);
    assert_eq!(ShapeKind::BuiltIn(BuiltInShape::Box), card.shape);
    assert_eq!(1, card.labels.len());
    let label = &card.labels[0];
    assert_eq!(
        Binding::Cel("self.name + ' (' + self.when + ')'".to_owned()),
        label.text
    );
    assert_eq!(LabelPlacement::Before, label.placement);
    assert!(label.editable);
    assert_eq!(Some(Binding::Attribute("name".to_owned())), card.tooltip);
    assert_eq!(AnchorKind::Edge, card.anchors.kind);
    assert_eq!(EdgeSides::Horizontal, card.anchors.edge_sides);
    assert_eq!((Sizing::User, Resize::Both), (card.sizing, card.resize));
    assert!(card.draggable);

    let box_ = &definition.element_types[1];
    assert_eq!(ShapeKind::BuiltIn(BuiltInShape::Hexagon), box_.shape);
    assert_eq!(
        (AnchorKind::Edge, false, false),
        (box_.anchors.kind.clone(), box_.anchors.enabled, box_.anchors.visible)
    );
    assert_eq!(Sizing::Model, box_.sizing);
    assert!(!box_.draggable);
    assert_eq!(Some((40.0, 40.0)), box_.size.fixed);
}

#[test]
fn the_relation_maps_its_route_disls_open_arrow_to_the_librarys_and_one_per_direction() {
    let link = &compiled(&text(&spec())).relation_types[0];
    assert_eq!(Route::BuiltIn(BuiltInRoute::Orthogonal), link.route);
    assert_eq!(
        (MarkerKind::None, MarkerKind::OpenArrow),
        (link.start_marker, link.end_marker)
    );
    assert_eq!(vec!["Card", "Box"], link.endpoints.source.element_types);
    assert_eq!(vec!["Card"], link.endpoints.target.element_types);
    assert!(!link.endpoints.allow_self);
    assert_eq!(Some(PerPair::Ordered), link.endpoints.per_pair);
    assert!(link.selectable);
}

#[test]
fn a_shape_it_cannot_draw_and_a_marker_it_has_none_for_are_refused() {
    assert!(
        refusal(&text(&with(spec(), "/notation/nodes/Box/shape", json!("cloud"))))
            .contains("\"cloud\", which is no built-in")
    );
    assert!(
        refusal(&text(&with(
            spec(),
            "/notation/nodes/Box/shape",
            json!({ "type": "hexagon", "params": { "inset": 0.3 } })
        )))
        .contains("inset 0.25")
    );
    assert!(
        refusal(&text(&with(
            spec(),
            "/notation/edges/Link/targetMarker",
            json!("erMany")
        )))
        .contains("no marker")
    );
}

/// The spec with Card's one label replaced by `label`.
fn with_label(label: Value) -> String {
    let mut whole = json!({ "id": "name", "text": { "attribute": "name" } });
    for (key, value) in label.as_object().unwrap() {
        whole[key] = value.clone();
    }
    text(&with(spec(), "/notation/nodes/Card/labels", json!([whole])))
}

#[test]
fn a_label_position_without_a_placement_is_refused_and_one_outside_on_the_right_sits_beside() {
    assert!(refusal(&with_label(json!({ "position": "outside-top" }))).contains("\"outside-top\""));
    let beside = &compiled(&with_label(json!({ "position": "outside-right" }))).element_types[0].labels[0];
    assert_eq!(LabelPlacement::Beside, beside.placement);
    assert_eq!(Binding::Attribute("name".to_owned()), beside.text);
    assert!(refusal(&with_label(json!({ "position": "outside-right", "distance": 4 }))).contains("sits 4 beside"));
}

#[test]
fn a_label_anchored_at_an_edge_is_an_aligned_line_inset_by_the_offset_inward() {
    let right = &compiled(&with_label(
        json!({ "position": { "anchor": [1, 0], "offset": [-4, 12], "align": "end" } }),
    ))
    .element_types[0]
        .labels[0];
    assert_eq!(
        (
            AnchorTo::Top,
            Some(TextAnchor::End),
            Some(4.0),
            Some(Point::new(0.0, 12.0))
        ),
        (right.anchor_to, right.align, right.inset_x, right.offset)
    );
    let left = &compiled(&with_label(
        json!({ "position": { "anchor": [0, 1], "offset": [3, -2], "align": "start" } }),
    ))
    .element_types[0]
        .labels[0];
    assert_eq!(
        (
            AnchorTo::Bottom,
            Some(TextAnchor::Start),
            Some(3.0),
            Some(Point::new(0.0, -2.0))
        ),
        (left.anchor_to, left.align, left.inset_x, left.offset)
    );
}

#[test]
fn a_label_at_an_edge_aligned_other_than_toward_it_or_anchored_elsewhere_is_refused() {
    assert!(
        refusal(&with_label(
            json!({ "position": { "anchor": [1, 0], "align": "start" } })
        ))
        .contains("right edge aligned \"start\"")
    );
    assert!(refusal(&with_label(json!({ "position": { "anchor": [1, 0] } }))).contains("right edge with no align"));
    assert!(refusal(&with_label(json!({ "position": { "anchor": [0.25, 0] } }))).contains("anchors at [0.25,0]"));
}

#[test]
fn a_labels_visibility_is_its_condition_as_written() {
    let drawn = &compiled(&with_label(json!({ "visible": { "cel": "self.when != ''" } }))).element_types[0].labels[0];
    assert_eq!(Some("self.when != ''".to_owned()), drawn.when);
    let bare = &compiled(&with_label(json!({ "visible": "self.when != ''" }))).element_types[0].labels[0];
    assert_eq!(Some("self.when != ''".to_owned()), bare.when);
    assert_eq!(
        None,
        compiled(&with_label(json!({ "visible": true }))).element_types[0].labels[0].when
    );
    assert_eq!(
        Some("false".to_owned()),
        compiled(&with_label(json!({ "visible": false }))).element_types[0].labels[0].when
    );
}

#[test]
fn a_relation_is_unselectable_where_the_notation_says_so() {
    let unselectable = compiled(&text(&with(spec(), "/notation/edges/Link/selectable", json!(false))));
    assert!(!unselectable.relation_types[0].selectable);
}

/// A spec drawn as the timeline is: two types with named begin and end anchors on their sides, one relation started
/// from either, labelled above its middle, created on an empty release by its tool.
fn span_spec() -> Value {
    let anchors =
        json!({ "mode": "fixed", "points": [{ "id": "begin", "x": 0, "y": 0.5 }, { "id": "end", "x": 1, "y": 0.5 }] });
    json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "types": { "Item": { "abstract": true }, "Span": { "extends": "Item" }, "Point": { "extends": "Item" } },
            "relations": { "Next": { "source": { "types": ["Item"], "role": "earlier" }, "target": { "types": ["Item"] }, "directed": true, "allowSelfLoops": false, "allowParallel": true, "attributes": { "label": { "type": "string" } } } }
        },
        "notation": {
            "nodes": {
                "Span": { "shape": "roundedRect", "anchors": anchors, "placement": { "movable": { "x": true, "y": true } } },
                "Point": { "shape": "diamond", "anchors": anchors, "size": { "fixed": [18, 18] } }
            },
            "edges": {
                "Next": {
                    "line": { "routing": "bezier" },
                    "targetMarker": "none",
                    "connect": { "from": { "end": "source", "begin": "target" }, "tool": "relate" },
                    "labels": [{ "id": "label", "text": { "attribute": "label" }, "at": "middle", "side": "above", "distance": 6, "editable": "inline" }]
                }
            }
        },
        "toolbox": { "tools": { "relate": { "id": "relate", "creates": "Next", "mode": "drag", "createTarget": { "type": "Span" } } } }
    })
}

#[test]
fn fixed_anchors_are_named_on_the_sides_they_lie_on_and_narrow_the_other_end_to_those_sides() {
    let span = &compiled(&text(&span_spec())).element_types[0];
    assert_eq!(
        AnchorKind::Sides(vec![
            SideFraction {
                side: Edge::Left,
                at: 0.5,
                name: Some("begin".to_owned())
            },
            SideFraction {
                side: Edge::Right,
                at: 0.5,
                name: Some("end".to_owned())
            },
        ]),
        span.anchors.kind
    );
    assert_eq!(EdgeSides::Horizontal, span.anchors.edge_sides);
    let off = with(
        span_spec(),
        "/notation/nodes/Span/anchors",
        json!({ "mode": "fixed", "points": [{ "id": "mid", "x": 0.5, "y": 0.5 }] }),
    );
    assert!(refusal(&text(&off)).contains("\"mid\" at (0.5, 0.5) lies on no side"));
}

#[test]
fn disls_rounded_rectangle_and_diamond_are_the_librarys_own() {
    let definition = compiled(&text(&span_spec()));
    assert_eq!(
        vec![
            ShapeKind::BuiltIn(BuiltInShape::RoundedRectangle),
            ShapeKind::BuiltIn(BuiltInShape::Diamond)
        ],
        definition
            .element_types
            .iter()
            .map(|t| t.shape.clone())
            .collect::<Vec<_>>()
    );
    // The abstract Item is not drawn.
    assert_eq!(
        vec!["Span", "Point"],
        definition
            .element_types
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn an_edges_label_its_starting_anchors_and_its_tools_release_on_empty_canvas_are_mapped() {
    let next = &compiled(&text(&span_spec())).relation_types[0];
    assert_eq!(Route::BuiltIn(BuiltInRoute::CubicBezier), next.route);
    assert_eq!(MarkerKind::None, next.end_marker);
    let label = next.label.as_ref().unwrap();
    assert_eq!(
        (RouteLabelPlacement::Midpoint, Some(-6.0), true),
        (label.placement, label.offset, label.editable)
    );
    assert_eq!(vec!["Span", "Point"], next.endpoints.source.element_types);
    assert_eq!(
        EndpointAnchors::Named(vec!["begin".to_owned(), "end".to_owned()]),
        next.endpoints.source.anchors
    );
    assert_eq!(EndpointAnchors::Any, next.endpoints.target.anchors);
    assert!(next.empty_release);

    let plain_tool = with(
        span_spec(),
        "/toolbox/tools/relate",
        json!({ "id": "relate", "creates": "Next" }),
    );
    assert!(!compiled(&text(&plain_tool)).relation_types[0].empty_release);
    assert!(
        refusal(&text(&with(
            span_spec(),
            "/notation/edges/Next/connect",
            json!({ "tool": "absent" })
        )))
        .contains("\"absent\", which the toolbox does not declare")
    );
    let at_end = with(
        span_spec(),
        "/notation/edges/Next/labels",
        json!([{ "id": "l", "text": { "attribute": "label" }, "at": "end", "side": "below", "distance": 4 }]),
    );
    let label = compiled(&text(&at_end)).relation_types[0].label.clone().unwrap();
    assert_eq!(
        (RouteLabelPlacement::Target, Some(4.0), false),
        (label.placement, label.offset, label.editable)
    );
    let fraction = with(
        span_spec(),
        "/notation/edges/Next/labels",
        json!([{ "id": "l", "text": { "attribute": "label" }, "at": 0.25 }]),
    );
    assert!(refusal(&text(&fraction)).contains("labels at a fraction along a \"bezier\" line"));
}

/// A relation without a target (DISL 4.9) and the stub it is drawn as (6.10 *Stubs*).
fn open_spec(stub: Option<Value>) -> String {
    let mut spec = json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "types": {
                "File": { "label": "File", "labelAttribute": "path", "attributes": { "path": { "type": "string" } } },
                "Part": { "label": "Part", "extends": "File" }
            },
            "relations": { "Uses": { "source": "File", "target": { "types": ["Part"], "optional": true }, "attributes": { "label": { "type": "string" } } } }
        },
        "notation": {
            "styles": { "missing": { "stroke": { "width": 1.5, "dash": [3, 3] } }, "missingDependency": { "extends": "missing", "stroke": { "width": 2, "dash": [2, 6] } } },
            "nodes": { "File": { "shape": "rect", "connectable": false }, "Part": { "shape": "rect", "connectable": false } },
            "edges": { "Uses": { "line": { "routing": "straight" }, "targetMarker": "arrowFilled", "labels": [{ "id": "label", "text": { "attribute": "label" }, "at": "middle", "side": "above", "distance": 6 }] } }
        }
    });
    if let Some(stub) = stub {
        spec = with(spec, "/notation/edges/Uses/stub", stub);
    }
    text(&spec)
}

fn stub() -> Value {
    json!({ "length": 48, "side": "right", "style": "missing", "label": { "id": "asWritten", "text": { "cel": "self.label + ' (missing)'" }, "side": "above" } })
}

#[test]
fn a_stub_is_read_with_its_length_its_side_its_style_and_its_label() {
    let uses = &compiled(&open_spec(Some(stub()))).relation_types[0];
    let stub = uses.stub.as_ref().unwrap();
    assert_eq!((48.0, Edge::Right), (stub.length, stub.side));
    assert_eq!(Some(json!({ "stroke": { "width": 1.5, "dash": [3, 3] } })), stub.style);
    let label = stub.label.as_ref().unwrap();
    assert_eq!(
        (Binding::Cel("self.label + ' (missing)'".to_owned()), false),
        (label.text.clone(), label.below)
    );

    let stated = &compiled(&open_spec(Some(json!({ "length": 30, "side": "left", "style": "missingDependency", "label": { "id": "n", "text": { "attribute": "label" }, "side": "below", "distance": 4 } })))).relation_types[0];
    let stub = stated.stub.as_ref().unwrap();
    assert_eq!((30.0, Edge::Left), (stub.length, stub.side));
    assert_eq!(Some(json!({ "stroke": { "width": 2, "dash": [2, 6] } })), stub.style);
    assert_eq!(
        (true, Some(4.0)),
        (
            stub.label.as_ref().unwrap().below,
            stub.label.as_ref().unwrap().distance
        )
    );
    assert!(compiled(&open_spec(None)).relation_types[0].stub.is_none());
}

#[test]
fn a_stub_side_it_cannot_place_and_a_stub_label_beside_it_are_refused() {
    let mut auto = stub();
    auto["side"] = json!("auto");
    assert!(refusal(&open_spec(Some(auto))).contains("stub leaves \"auto\""));
    let mut on = stub();
    on["label"]["side"] = json!("on");
    assert!(refusal(&open_spec(Some(on))).contains("sits \"on\""));
}

#[test]
fn a_node_that_states_its_role_and_no_name_is_named_as_disl_names_it() {
    let spec: Value = serde_json::from_str(&open_spec(None)).unwrap();
    let spec = with(spec, "/notation/nodes/File/accessibility", json!({ "role": "button" }));
    let spec = with(
        spec,
        "/notation/nodes/Part/accessibility",
        json!({ "role": "link", "name": { "attribute": "path" } }),
    );
    let definition = compiled(&text(&spec));
    let file = definition.element_types[0].accessibility.as_ref().unwrap();
    assert_eq!(Some("button"), file.role.as_deref());
    assert_eq!(Some(Binding::Cel("'File ' + string(self.path)".to_owned())), file.name);
    let part = definition.element_types[1].accessibility.as_ref().unwrap();
    assert_eq!(Some(Binding::Attribute("path".to_owned())), part.name);
}

/// A node type and a relation type with notation of their own on an abstract supertype, and subtypes stating a part.
fn inheriting_spec() -> String {
    text(&json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "types": {
                "Thing": { "abstract": true, "attributes": { "name": { "type": "string" } } },
                "Card": { "extends": "Thing", "label": "Card" },
                "Note": { "extends": "Thing", "label": "Note" }
            },
            "relations": {
                "Link": { "abstract": true, "source": ["Thing"], "target": { "types": ["Thing"], "optional": true } },
                "Uses": { "extends": "Link", "source": ["Card"], "target": { "types": ["Note"], "optional": true } },
                "Owns": { "extends": "Link", "source": ["Card"], "target": ["Card"] }
            }
        },
        "notation": {
            "nodes": {
                "Thing": { "shape": "rect", "labels": [{ "id": "name", "text": { "attribute": "name" }, "position": "center" }], "size": { "default": [72, 32] }, "deletable": false },
                "Card": { "tooltip": { "attribute": "name" } },
                "Note": { "shape": "ellipse" }
            },
            "edges": {
                "Link": { "line": { "routing": "straight", "stroke": { "width": 1.5 } }, "targetMarker": "arrowFilled" },
                "Owns": { "line": { "stroke": { "width": 2 } }, "targetMarker": "none" }
            }
        }
    }))
}

#[test]
fn no_abstract_type_is_drawn_and_each_subtype_has_its_supertypes_notation_under_its_own() {
    let definition = compiled(&inheriting_spec());
    assert_eq!(
        vec!["Card", "Note"],
        definition
            .element_types
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
    );
    let (card, note) = (&definition.element_types[0], &definition.element_types[1]);
    assert_eq!(ShapeKind::BuiltIn(BuiltInShape::Box), card.shape);
    assert_eq!(Binding::Attribute("name".to_owned()), card.labels[0].text);
    assert_eq!(Some(Binding::Attribute("name".to_owned())), card.tooltip);
    assert!(!card.deletable);
    assert_eq!(ShapeKind::BuiltIn(BuiltInShape::Ellipse), note.shape);
    assert_eq!(Binding::Attribute("name".to_owned()), note.labels[0].text);
    assert!(!note.deletable && note.tooltip.is_none());
    assert_eq!((Some(72.0), Some(32.0)), note.size.default);
}

#[test]
fn a_relation_with_no_notation_of_its_own_is_drawn_as_its_supertypes_and_the_one_it_states_is_merged() {
    let definition = compiled(&inheriting_spec());
    assert_eq!(
        vec!["Uses", "Owns"],
        definition
            .relation_types
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
    );
    let (uses, owns) = (&definition.relation_types[0], &definition.relation_types[1]);
    assert_eq!(MarkerKind::Arrow, uses.end_marker);
    assert_eq!(
        (vec!["Card".to_owned()], vec!["Note".to_owned()]),
        (
            uses.endpoints.source.element_types.clone(),
            uses.endpoints.target.element_types.clone()
        )
    );
    assert_eq!(MarkerKind::None, owns.end_marker);
    assert_eq!(uses.route, owns.route);
    assert_eq!(Some(&json!(2)), owns.style.pointer("/stroke/width"));
    assert_eq!(Some(&json!(1.5)), uses.style.pointer("/stroke/width"));
}

#[test]
fn a_lines_dash_is_kept_as_written_and_a_dash_it_cannot_read_is_refused() {
    let dashed = |dash: Value| -> String {
        let spec: Value = serde_json::from_str(&inheriting_spec()).unwrap();
        text(&with(spec, "/notation/edges/Link/line/stroke/dash", dash))
    };
    assert_eq!(
        Some(&json!([1, 4])),
        compiled(&dashed(json!([1, 4]))).relation_types[0]
            .style
            .pointer("/stroke/dash")
    );
    assert_eq!(
        Some(&json!({ "cel": "self.name != '' ? [5.0, 4.0] : []" })),
        compiled(&dashed(json!({ "cel": "self.name != '' ? [5.0, 4.0] : []" }))).relation_types[0]
            .style
            .pointer("/stroke/dash")
    );
    assert!(refusal(&dashed(json!(true))).contains("dash"));
}

#[test]
fn the_canvass_empty_sentence_is_stated_and_one_with_a_brace_is_refused() {
    let empty = |sentence: &str| {
        let spec: Value = serde_json::from_str(&inheriting_spec()).unwrap();
        text(&with(spec, "/notation/canvas/empty", json!({ "text": sentence })))
    };
    assert_eq!(
        Some("Nothing to draw here.".to_owned()),
        compiled(&empty("Nothing to draw here.")).empty_text
    );
    assert_eq!(None, compiled(&inheriting_spec()).empty_text);
    assert!(refusal(&empty("Nothing in {folder}.")).contains("{folder}"));
}

/// DISL 0.4: the lists inside a node and its badges.
fn lists_spec() -> Value {
    json!({
        "disl": "0.4",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "enums": { "State": { "values": { "open": { "label": "Open" }, "done": { "label": "Done" } } } },
            "types": {
                "Board": { "attributes": { "name": { "type": "string" } } },
                "Card": { "attributes": { "name": { "type": "string" }, "board": { "type": "Board" }, "note": { "type": "string" }, "link": { "type": "string" }, "noteLink": { "type": "string" } }, "children": { "allowed": ["Item"] } },
                "Item": { "attributes": { "title": { "type": "string" }, "state": { "type": "State" }, "link": { "type": "string" }, "updated": { "type": "string" } } }
            },
            "relations": {
                "BoardCard": { "source": "Board", "target": "Card", "allowSelfLoops": false, "derived": { "source": "item.board", "target": "item", "edits": { "connect": "setBoard" } } }
            }
        },
        "notation": {
            "nodes": {
                "Board": { "shape": "rect", "labels": [{ "id": "name", "text": { "attribute": "name" }, "position": "center" }] },
                "Card": {
                    "shape": "rect",
                    "labels": [
                        { "id": "name", "text": { "attribute": "name" }, "position": "top" },
                        { "id": "kind", "text": { "attribute": "name" }, "position": "bottom" },
                        { "id": "state", "text": { "attribute": "name" }, "position": { "anchor": [0.5, 0], "offset": [0, 36] } }
                    ],
                    "compartments": [
                        { "id": "note", "items": { "cel": "[self.note]" }, "itemText": { "cel": "item" }, "itemLink": { "cel": "self.noteLink" } },
                        { "id": "items", "items": { "children": ["Item"], "slot": "items" }, "itemText": { "cel": "item.title" }, "itemLink": { "cel": "item.link" },
                          "itemOrder": { "by": "item.updated", "direction": "descending" }, "groupBy": { "attribute": "state", "collapsed": { "done": true } } },
                        { "id": "more", "title": "More", "items": { "children": ["Item"], "slot": "more" }, "itemText": { "cel": "item.title" }, "collapsible": true, "collapsed": true }
                    ],
                    "badges": [
                        { "id": "link", "position": "top-right", "visible": { "cel": "self.link != ''" }, "onClick": "openLink" },
                        { "id": "pinned", "position": "top-left", "visible": { "cel": "self.name == 'pinned'" }, "tooltip": "Locked. Unlock it from its menu." }
                    ]
                }
            },
            "edges": { "BoardCard": { "sourceMarker": "none", "targetMarker": "none" } }
        },
        "behavior": { "operations": { "openLink": { "actions": [{ "open": "self.link" }] }, "setBoard": { "actions": [] }, "shout": { "actions": [{ "set": {} }] } } }
    })
}

#[test]
fn a_label_sits_on_the_first_or_the_last_line_inside_its_node_and_lines_attach_anywhere_where_no_anchors_are_stated() {
    let card = compiled(&text(&lists_spec())).element_types[1].clone();
    assert_eq!(
        vec![
            (AnchorTo::Top, Some(Point::new(0.0, 20.0))),
            (AnchorTo::Bottom, Some(Point::new(0.0, -10.0))),
            (AnchorTo::Top, Some(Point::new(0.0, 36.0))),
        ],
        card.labels.iter().map(|l| (l.anchor_to, l.offset)).collect::<Vec<_>>()
    );
    assert_eq!(
        (AnchorKind::Edge, false),
        (card.anchors.kind.clone(), card.anchors.visible)
    );
}

#[test]
fn a_nodes_lists_are_compiled_beneath_its_lowest_label() {
    let card = compiled(&text(&lists_spec())).element_types[1].clone();
    let (note, items, more) = (&card.compartments[0], &card.compartments[1], &card.compartments[2]);
    assert_eq!(CompartmentItems::Cel("[self.note]".to_owned()), note.items);
    assert_eq!(
        ("item", Some("self.noteLink"), true),
        (note.text.as_str(), note.link.as_deref(), note.headless)
    );
    assert_eq!(
        (48.0, 20.0, 18.0, 8.0, 10.0, 0.0),
        (
            note.top,
            note.heading_height,
            note.row_height,
            note.bottom,
            note.inset_x,
            note.row_indent
        )
    );
    assert_eq!(
        CompartmentItems::Children {
            types: vec!["Item".to_owned()],
            slot: "items".to_owned()
        },
        items.items
    );
    assert_eq!(Some(("item.updated".to_owned(), false)), items.order_by);
    let grouping = items.group_by.as_ref().unwrap();
    assert_eq!(
        (
            vec![
                ("open".to_owned(), "Open".to_owned()),
                ("done".to_owned(), "Done".to_owned())
            ],
            vec!["done".to_owned()]
        ),
        (grouping.groups.clone(), grouping.collapsed.clone())
    );
    assert_eq!((12.0, false), (items.row_indent, items.headless));
    assert_eq!(
        (Some("More"), 12.0, true, false),
        (more.title.as_deref(), more.row_indent, more.collapsed, more.headless)
    );
}

#[test]
fn a_group_that_is_no_enum_of_the_items_is_refused() {
    let spec = with(
        lists_spec(),
        "/notation/nodes/Card/compartments",
        json!([{ "id": "x", "items": { "children": ["Item"], "slot": "items" }, "itemText": { "cel": "item.title" }, "groupBy": { "attribute": "title" } }]),
    );
    assert!(refusal(&text(&spec)).contains("groups by \"title\", which is no enum attribute of its items"));
}

#[test]
fn a_badge_that_opens_an_attribute_is_its_link_symbol_and_one_that_runs_nothing_a_mark() {
    let card = compiled(&text(&lists_spec())).element_types[1].clone();
    assert_eq!(1, card.links.len());
    assert_eq!(
        ("link", "link", 20.0, 5.0),
        (
            card.links[0].id.as_str(),
            card.links[0].attribute.as_str(),
            card.links[0].right,
            card.links[0].top
        )
    );
    assert_eq!(1, card.decorations.len());
    let mark = &card.decorations[0];
    assert_eq!(Glyph::Circle, mark.glyph);
    assert_eq!(
        DecorationOrigin::Corner {
            right: false,
            inset: 9.0
        },
        mark.origin
    );
    assert_eq!(3.5, mark.radius);
    assert_eq!(
        Some(Binding::Literal("Locked. Unlock it from its menu.".to_owned())),
        mark.tooltip
    );
    assert_eq!(Some("Locked"), mark.label.as_deref());
    assert_eq!(Some("self.name == 'pinned'"), mark.when.as_deref());
}

#[test]
fn a_badge_that_runs_something_else_or_sits_elsewhere_is_refused() {
    let badged = |badge: Value| text(&with(lists_spec(), "/notation/nodes/Card/badges", json!([badge])));
    assert!(
        refusal(&badged(
            json!({ "id": "b", "position": "top-right", "onClick": "shout" })
        ))
        .contains("runs \"shout\"")
    );
    assert!(refusal(&badged(json!({ "id": "b", "position": "bottom-left" }))).contains("sits \"bottom-left\""));
}

#[test]
fn a_derived_relation_a_gesture_can_draw_is_one_per_element_at_the_end_that_holds_its_key() {
    let definition = compiled(&text(&lists_spec()));
    assert_eq!(
        (None, Some(1)),
        (
            definition.relation_types[0].endpoints.max_from_source,
            definition.relation_types[0].endpoints.max_into_target
        )
    );
    let flipped = with(
        lists_spec(),
        "/metamodel/relations/BoardCard/derived",
        json!({ "source": "item", "target": "item.board", "edits": { "connect": "setBoard" } }),
    );
    assert_eq!(
        Some(1),
        compiled(&text(&flipped)).relation_types[0].endpoints.max_from_source
    );
}

#[test]
fn a_curved_line_is_a_bowed_arc_with_its_variants_its_loop_its_bars_and_its_marks() {
    let definition = compiled(&text(&common::curved_spec()));
    assert_eq!(
        vec![
            ("Node", ShapeKind::BuiltIn(BuiltInShape::Pill)),
            ("Hub", ShapeKind::BuiltIn(BuiltInShape::None))
        ],
        definition
            .element_types
            .iter()
            .map(|t| (t.name.as_str(), t.shape.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(AnchorKind::Edge, definition.element_types[0].anchors.kind);
    assert_eq!(
        EndpointAnchors::Edge,
        definition.relation_types[0].endpoints.source.anchors
    );
    assert_eq!(
        EndpointAnchors::Edge,
        definition.relation_types[0].endpoints.target.anchors
    );
    let Route::BowedArc(arc) = &definition.relation_types[0].route else {
        panic!("not a bowed arc");
    };
    assert_eq!(0.2, arc.bow);
    assert_eq!(vec![("self.flipped".to_owned(), -0.2)], arc.variants);
    assert_eq!(Some(30.0), arc.self_loop_radius);
    assert_eq!(1, arc.bars.len());
    assert_eq!(
        (vec![0.44, 0.56], 16.0, Some("self.delayed")),
        (arc.bars[0].at.clone(), arc.bars[0].length, arc.bars[0].when.as_deref())
    );
    assert_eq!(
        (0.86, true, 11.0),
        (arc.marks[0].at, arc.marks[0].left, arc.marks[0].distance)
    );
    let hub = &definition.element_types[1];
    assert_eq!(Some("members"), hub.centre_of.as_deref());
    assert_eq!(Glyph::CircularArrow, hub.decorations[0].glyph);
    assert_eq!(
        DecorationOrigin::Centre { dx: 0.0, dy: -15.0 },
        hub.decorations[0].origin
    );
    assert_eq!(
        Some("(self.turn == 'back' ? 'horizontal' : 'none') == 'horizontal'"),
        hub.decorations[0].mirrored.as_deref()
    );
}

#[test]
fn what_it_cannot_draw_along_a_line_an_icon_it_has_none_of_and_another_placement_are_refused() {
    let straight = with(
        common::curved_spec(),
        "/notation/edges/Arrow/line/routing",
        json!("straight"),
    );
    assert!(refusal(&text(&straight)).contains("along a \"straight\" line"));
    let cog = with(
        common::curved_spec(),
        "/notation/nodes/Hub/icon",
        json!({ "icon": "mdi-cog", "position": "top", "size": 26 }),
    );
    assert!(refusal(&text(&cog)).contains("mdi-cog"));
    let placed = with(
        common::curved_spec(),
        "/notation/nodes/Hub/placement",
        json!({ "x": { "cel": "1.0" }, "y": { "cel": "2.0" } }),
    );
    assert!(refusal(&text(&placed)).contains("placed at"));
    let attribute = with(
        common::curved_spec(),
        "/notation/nodes/Hub/placement",
        json!({ "x": { "attribute": "caption" } }),
    );
    assert!(compiled(&text(&attribute)).element_types[1].centre_of.is_none());
}

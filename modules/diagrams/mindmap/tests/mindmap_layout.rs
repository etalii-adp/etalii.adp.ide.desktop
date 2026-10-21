//! The two-sided layout, tested on positions alone (mindmap Requirement 5.5), and the metric it sizes nodes with.
//! Ported from the standalone host's `MindmapLayout.Tests.cs` and `MindmapMetrics.Tests.cs` at `8b6ccfc`.

mod common;

use std::path::Path;

use adp_diagram_mindmap::document::MindmapDocument;
use adp_diagram_mindmap::layout::{self, LayoutNode, MindmapMetrics, round2};
use adp_layout::Rect;
use indexmap::IndexMap;

fn layout_of(document: &MindmapDocument, is_folded: &dyn Fn(&str) -> bool) -> IndexMap<String, Rect> {
    layout::compute(&LayoutNode::of(document), &MindmapMetrics::default(), is_folded)
}

fn nothing(_: &str) -> bool {
    false
}

fn corpus() -> MindmapDocument {
    common::load(true)
}

const LONG_SIBLINGS: &str = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"r\" ID=\"root\">\n<node TEXT=\"a considerably longer sibling label than most\" ID=\"a\" POSITION=\"right\"/>\n<node TEXT=\"another considerably longer sibling label\" ID=\"b\" POSITION=\"right\"/>\n</node>\n</map>\n";

#[test]
fn compute_is_deterministic() {
    let document = corpus();
    let first = layout_of(&document, &nothing);
    let second = layout_of(&document, &nothing);
    assert!(!first.is_empty(), "the corpus laid out to no boxes");
    assert_eq!(first, second);
}

#[test]
fn compute_centres_the_root_on_the_origin() {
    let document = corpus();
    let root = layout_of(&document, &nothing)[&document.id(document.root())];
    assert!(root.center_x().abs() < 1e-3);
    assert!(root.center_y().abs() < 1e-3);
}

#[test]
fn compute_places_every_visible_node_and_no_others() {
    let document = corpus();
    assert_eq!(layout_of(&document, &nothing).len(), document.nodes().len());
}

#[test]
fn compute_honours_freeplanes_position_attribute() {
    let boxes = layout_of(&corpus(), &nothing);
    assert!(boxes["ID_411002937"].x > 0.0, "Backend is POSITION=right");
    assert!(boxes["ID_411002941"].right() < 0.0, "Diagram types is POSITION=left");
}

#[test]
fn compute_alternates_sides_for_unpositioned_branches() {
    let document = MindmapDocument::parse(
        "<map version=\"freeplane 1.11.5\"><node TEXT=\"r\" ID=\"r\"><node TEXT=\"a\" ID=\"a\"/><node TEXT=\"b\" ID=\"b\"/><node TEXT=\"c\" ID=\"c\"/></node></map>",
        true,
    )
    .unwrap();
    let boxes = layout_of(&document, &nothing);
    assert!(boxes["a"].x > 0.0);
    assert!(boxes["b"].right() < 0.0);
    assert!(boxes["c"].x > 0.0);
}

#[test]
fn compute_siblings_do_not_overlap() {
    let document = corpus();
    let boxes = layout_of(&document, &nothing);
    let mut compared = 0;
    for node in document.nodes() {
        for side in [true, false] {
            let column: Vec<String> = document
                .children(node)
                .into_iter()
                .map(|c| document.id(c))
                .filter(|id| (boxes[id].x >= 0.0) == side)
                .collect();
            for pair in column.windows(2) {
                let (above, below) = (boxes[&pair[0]], boxes[&pair[1]]);
                assert!(below.y >= above.bottom(), "'{}' overlaps '{}'", pair[1], pair[0]);
                compared += 1;
            }
        }
    }
    assert!(compared > 0, "no sibling pair was compared");
}

#[test]
fn compute_puts_children_beyond_their_parent_on_the_parents_side() {
    let document = corpus();
    let boxes = layout_of(&document, &nothing);
    let metrics = MindmapMetrics::default();
    let backend = boxes["ID_411002937"];
    let children = document.children(document.find("ID_411002937").unwrap());
    assert!(!children.is_empty());
    for child in children {
        assert!(boxes[&document.id(child)].x >= backend.right() + metrics.horizontal_gap - 0.01);
    }
    let diagram_types = boxes["ID_411002941"];
    let children = document.children(document.find("ID_411002941").unwrap());
    assert!(!children.is_empty());
    for child in children {
        assert!(boxes[&document.id(child)].right() <= diagram_types.x - metrics.horizontal_gap + 0.01);
    }
}

#[test]
fn compute_a_folded_branch_hides_its_descendants_and_takes_no_room_for_them() {
    let document = corpus();
    let unfolded = layout_of(&document, &nothing);
    let folded = layout_of(&document, &|id| id == "ID_88117425");
    assert!(unfolded.contains_key("ID_88117426"));
    assert!(
        !folded.contains_key("ID_88117426"),
        "a descendant of a folded node was laid out"
    );
    assert!(
        folded.contains_key("ID_88117425"),
        "the folded node itself stays visible"
    );
    assert!(
        folded["ID_88117425"].bottom() - folded["ID_88117420"].y
            < unfolded["ID_88117425"].bottom() - unfolded["ID_88117420"].y
    );
}

#[test]
fn compute_a_longer_text_widens_the_node() {
    let mut document = corpus();
    let node = document.find("ID_88117422").unwrap();
    let before = layout_of(&document, &nothing)["ID_88117422"].width;
    let longer = format!("{} with a considerably longer label", document.text(node));
    document.set_text(node, &longer);
    assert!(layout_of(&document, &nothing)["ID_88117422"].width > before);
}

#[test]
fn compute_an_edit_in_one_branch_leaves_the_other_side_where_it_was() {
    let mut document = corpus();
    let before = layout_of(&document, &nothing);
    let node = document.find("ID_88117422").unwrap();
    document.add_child(node, "new");
    let after = layout_of(&document, &nothing);
    for id in ["ID_411002941", "ID_411002942", "ID_411002945", "ID_411002949"] {
        assert_eq!(before[id], after[id], "{id} moved");
    }
}

#[test]
fn measure_gives_an_empty_node_a_minimum_width() {
    let metrics = MindmapMetrics::default();
    assert_eq!(metrics.measure("").0, metrics.minimum_width);
}

#[test]
fn compute_keeps_at_least_the_configured_share_of_a_nodes_width_around_it() {
    let document = MindmapDocument::parse(LONG_SIBLINGS, true).unwrap();
    let metrics = MindmapMetrics {
        minimum_gap_ratio: 0.1,
        ..MindmapMetrics::default()
    };
    let boxes = layout::compute(&LayoutNode::of(&document), &metrics, &nothing);
    let (a, b) = (boxes["a"], boxes["b"]);
    let widest = a.width.max(b.width);
    let distance = (b.y - a.bottom()).abs().min((a.y - b.bottom()).abs());
    assert!(distance >= widest * 0.1 - 0.01, "siblings are {distance} apart");
    let root = boxes["root"];
    assert!(a.x - root.right() >= root.width * 0.1 - 0.01);
}

#[test]
fn compute_children_of_one_parent_share_the_same_horizontal_distance_from_it() {
    let document = corpus();
    let boxes = layout_of(&document, &nothing);
    let round6 = |v: f64| (v * 1e6).round() / 1e6;
    let mut checked = 0;
    for parent in document.nodes().into_iter().filter(|n| document.has_children(*n)) {
        let Some(parent_box) = boxes.get(&document.id(parent)) else {
            continue;
        };
        let children: Vec<Rect> = document
            .children(parent)
            .into_iter()
            .filter_map(|c| boxes.get(&document.id(c)).copied())
            .collect();
        if children.len() < 2 {
            continue;
        }
        checked += 1;
        for side in [true, false] {
            let mut edges: Vec<f64> = children
                .iter()
                .filter(|b| (b.center_x() > parent_box.center_x()) == side)
                .map(|b| round6(if side { b.x } else { b.right() }))
                .collect();
            edges.dedup();
            assert!(
                edges.len() <= 1,
                "children of {} do not align: {edges:?}",
                document.id(parent)
            );
        }
    }
    assert!(checked > 0, "no parent had two placed children");
}

#[test]
fn compute_no_two_visible_nodes_overlap() {
    let boxes: Vec<Rect> = layout_of(&corpus(), &nothing).values().copied().collect();
    assert!(boxes.len() >= 2);
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            let apart = a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y;
            assert!(apart, "boxes at ({},{}) and ({},{}) overlap", a.x, a.y, b.x, b.y);
        }
    }
}

#[test]
fn compute_a_larger_configured_ratio_spreads_the_siblings_further() {
    let document = MindmapDocument::parse(LONG_SIBLINGS, true).unwrap();
    let with = |ratio: f64| {
        let metrics = MindmapMetrics {
            minimum_gap_ratio: ratio,
            ..MindmapMetrics::default()
        };
        let boxes = layout::compute(&LayoutNode::of(&document), &metrics, &nothing);
        boxes["b"].y - boxes["a"].bottom()
    };
    assert!(
        with(0.5) > with(0.1),
        "a larger ratio pushes the siblings further apart"
    );
}

/// The shared text metric's golden fixture, with this module's own sizing around each width.
#[test]
fn a_texts_width_is_the_shared_metrics_width_with_this_modules_own_sizing_around_it() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../crates/adp-layout/tests/fixtures/text-metric.json");
    let fixture: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let metrics = MindmapMetrics::default();
    let cases: Vec<(String, f64)> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| (c["fontSize"].as_f64().unwrap() - metrics.font_size).abs() < f64::EPSILON)
        .map(|c| (c["text"].as_str().unwrap().to_owned(), c["width"].as_f64().unwrap()))
        .collect();
    assert!(!cases.is_empty());
    let mut failures = Vec::new();
    for (text, width) in cases {
        let expected = round2(metrics.minimum_width.max(width + 2.0 * metrics.horizontal_padding));
        let actual = metrics.measure(&text).0;
        if (actual - expected).abs() > 0.005 {
            failures.push(format!("\"{text}\": sized {actual}, expected {expected}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

//! Where a view's elements go, computed in the backend (etalii.adp spec 030, task T061): ported from the standalone
//! host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Layout.Tests.cs` at `8b6ccfc`. The geometry
//! guarantees matter as much as the arrangement: nothing overlaps, a boundary contains what it encloses and nothing
//! else, a declared `autoLayout` direction is honoured, and a box says its type and technology.

use std::path::Path;

use adp_diagram_c4::layout::{self, C4Layout};
use adp_diagram_c4::metrics::C4Metrics;
use adp_diagram_c4::model::{C4Element, C4ElementKind, C4View, C4Workspace};
use adp_diagram_c4::parser;

fn load(dsl: &str) -> (C4Workspace, C4View) {
    let workspace = parser::parse_text(dsl);
    let view = workspace.views[0].clone();
    (workspace, view)
}

fn fixture(name: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)).unwrap()
}

fn compute(dsl: &str) -> C4Layout {
    let (workspace, view) = load(dsl);
    layout::compute(&workspace, &view, &C4Metrics::default(), None)
}

fn assert_no_overlaps(layout: &C4Layout, what: &str) {
    let boxes: Vec<_> = layout.boxes.iter().map(|(_, r)| r).collect();
    for i in 0..boxes.len() {
        for j in i + 1..boxes.len() {
            assert!(!boxes[i].overlaps(&boxes[j]), "{what}: boxes {i} and {j} overlap");
        }
    }
}

const CHAIN: &str = r#"workspace {
    model {
        u = person "User" "Uses it."
        s = softwareSystem "System" "Does it."
        e = softwareSystem "Email" "Sends it."
        u -> s "Uses" "HTTPS"
        s -> e "Sends via" "SMTP"
    }
    views {
        systemContext s "context" {
            include *
        }
    }
}"#;

const OUTSIDERS: &str = r#"workspace {
    model {
        u = person "User" "Uses it."
        s = softwareSystem "System" "Does it." {
            web = container "Web" "Serves." "React"
            api = container "API" "Answers." "Kotlin"
            web -> api "Calls" "JSON"
        }
        ext = softwareSystem "Mainframe" "The old one." "External"
        u -> web "Visits" "HTTPS"
        api -> ext "Calls" "XML/HTTPS"
    }
    views {
        container s "containers" {
            include *
        }
    }
}"#;

#[test]
fn every_member_of_the_view_gets_a_box() {
    let layout = compute(CHAIN);
    assert_eq!(layout.boxes.len(), 3);
    assert!(layout.boxes.iter().all(|(_, b)| b.width > 0.0 && b.height > 0.0));
}

#[test]
fn compute_is_deterministic() {
    let sorted = |layout: C4Layout| {
        let mut boxes: Vec<_> = layout.boxes.iter().map(|(id, r)| (id.to_owned(), r)).collect();
        boxes.sort_by(|a, b| a.0.cmp(&b.0));
        boxes
    };
    assert_eq!(sorted(compute(CHAIN)), sorted(compute(CHAIN)));
}

#[test]
fn what_points_at_something_comes_before_it() {
    let layout = compute(CHAIN);
    let y = |id: &str| layout.boxes.get(id).unwrap().y;
    assert!(y("u") < y("s"), "the user should rank before the system");
    assert!(y("s") < y("e"), "the system should rank before the email system");
}

#[test]
fn no_two_elements_overlap() {
    assert_no_overlaps(&compute(CHAIN), "chain");
}

#[test]
fn no_two_elements_overlap_on_any_fixture() {
    for name in [
        "minimal.dsl",
        "comments-everywhere.dsl",
        "deployment-nested.dsl",
        "dynamic-interactions.dsl",
    ] {
        assert_no_overlaps(&compute(&fixture(name)), name);
    }
}

#[test]
fn a_declared_auto_layout_direction_is_honoured() {
    let top_to_bottom = compute(CHAIN);
    let left_to_right = compute(&CHAIN.replace("include *", "include *\n            autoLayout lr"));
    let at = |layout: &C4Layout, id: &str| layout.boxes.get(id).unwrap();
    assert!(at(&top_to_bottom, "u").y < at(&top_to_bottom, "s").y);
    assert!(
        at(&left_to_right, "u").x < at(&left_to_right, "s").x,
        "lr should rank along x"
    );
    assert_eq!(at(&left_to_right, "u").y.round(), at(&left_to_right, "s").y.round());
}

#[test]
fn a_reversed_direction_reads_the_same_layout_from_the_other_end() {
    let right_to_left = compute(&CHAIN.replace("include *", "include *\n            autoLayout rl"));
    let x = |id: &str| right_to_left.boxes.get(id).unwrap().x;
    assert!(x("u") > x("s"), "rl should put the source on the right");
}

#[test]
fn a_container_view_draws_a_boundary_that_contains_its_containers() {
    let dsl = r#"workspace {
    model {
        u = person "User" "Uses it."
        s = softwareSystem "System" "Does it." {
            web = container "Web" "Serves." "React"
            db = container "Data" "Stores." "PostgreSQL"
            web -> db "Reads" "SQL"
        }
        u -> web "Visits" "HTTPS"
    }
    views {
        container s "containers" {
            include *
        }
    }
}"#;
    let layout = compute(dsl);
    assert_eq!(layout.boundaries.len(), 1);
    let boundary = &layout.boundaries[0];
    assert_eq!(boundary.name, "System");
    assert_eq!(boundary.kind, "Software System");
    for id in ["web", "db"] {
        let b = layout.boxes.get(id).unwrap();
        assert!(b.x > boundary.rect.x, "{id} escapes the boundary on the left");
        assert!(b.right() < boundary.rect.right(), "{id} escapes on the right");
        assert!(b.y > boundary.rect.y, "{id} escapes above");
        assert!(b.bottom() < boundary.rect.bottom(), "{id} escapes below");
    }
    let u = layout.boxes.get("u").unwrap();
    assert!(
        !(u.overlaps(&boundary.rect) && u.y > boundary.rect.y),
        "the external person was enclosed"
    );
}

#[test]
fn what_is_not_inside_the_system_is_not_drawn_inside_its_boundary() {
    let layout = compute(OUTSIDERS);
    assert_eq!(layout.boundaries.len(), 1);
    let boundary = layout.boundaries[0].rect;
    for id in ["u", "ext"] {
        assert!(
            !layout.boxes.get(id).unwrap().overlaps(&boundary),
            "'{id}' is outside the system but was drawn inside its boundary"
        );
    }
    assert!(layout.boxes.get("web").unwrap().overlaps(&boundary));
    assert!(layout.boxes.get("api").unwrap().overlaps(&boundary));
}

#[test]
fn pushing_an_outsider_out_does_not_make_it_overlap_something_else() {
    assert_no_overlaps(&compute(OUTSIDERS), "pushing an outsider out of the boundary");
}

#[test]
fn a_context_view_draws_no_boundary() {
    assert!(compute(CHAIN).boundaries.is_empty());
}

#[test]
fn a_cycle_in_the_relationships_does_not_hang_the_layout() {
    let dsl = r#"workspace {
    model {
        a = softwareSystem "A" "desc"
        b = softwareSystem "B" "desc"
        a -> b "Calls" "HTTPS"
        b -> a "Calls back" "HTTPS"
    }
    views {
        systemLandscape "all" {
            include *
        }
    }
}"#;
    assert_eq!(compute(dsl).boxes.len(), 2);
}

#[test]
fn an_empty_view_lays_out_nothing_rather_than_throwing() {
    let dsl = "workspace {\n  model {\n  }\n  views {\n    systemLandscape \"all\" {\n    }\n  }\n}\n";
    let layout = compute(dsl);
    assert!(layout.boxes.is_empty());
    assert!(layout.boundaries.is_empty());
}

#[test]
fn the_bracketed_line_names_the_type_and_technology() {
    for (kind, technology, expected) in [
        (C4ElementKind::Person, "", "[Person]"),
        (C4ElementKind::SoftwareSystem, "", "[Software System]"),
        (C4ElementKind::Container, "React", "[Container: React]"),
        (C4ElementKind::Component, "Kotlin", "[Component: Kotlin]"),
        (C4ElementKind::DeploymentNode, "ECS", "[Deployment Node: ECS]"),
        (C4ElementKind::Container, "", "[Container]"),
    ] {
        let element = C4Element {
            id: "id".into(),
            kind,
            name: "Name".into(),
            description: "Description".into(),
            technology: technology.into(),
            tags: Vec::new(),
            parent_id: None,
            line: 1,
            referenced_id: None,
        };
        assert_eq!(layout::type_line_of(&element), expected);
    }
}

#[test]
fn a_long_description_widens_and_heightens_the_box_but_not_without_limit() {
    let metrics = C4Metrics::default();
    let small = metrics.measure("A", "[Person]", "Short.");
    let large = metrics.measure("A", "[Person]", &"x".repeat(400));
    assert!(
        large.height > small.height,
        "a wrapping description should make the box taller"
    );
    assert!(
        large.width <= metrics.maximum_width,
        "no box may grow past the maximum width"
    );
}

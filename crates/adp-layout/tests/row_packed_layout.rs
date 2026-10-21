//! The `row-packed` layout mode (etalii.adp spec 030, FR-018, task T054), ported from the standalone host's
//! `src/client/src/canvas/library/layout/rowPackedLayout.test.ts` at `8b6ccfc`.

use adp_layout::{
    LayoutAlgorithm, LayoutConnection, LayoutDefinition, LayoutElement, LayoutInput, LayoutMode, LayoutPlacement,
    Placements, Point, RowPackedDeclaration, RowPackedLayout, RowPackedWidth,
};

const WIDTH: f64 = 48.0;
const GAP: f64 = 4.0;
const ROW: f64 = 56.0;

fn definition() -> LayoutDefinition {
    LayoutDefinition {
        row_packed: Some(RowPackedDeclaration::new(WIDTH, GAP)),
        ..LayoutDefinition::with_modes(&[LayoutMode::Manual, LayoutMode::RowPacked])
    }
}

/// An element by its manual left edge and span, on a row; the layout reads centres, as the canvas does.
fn trend(id: &str, left: f64, span: f64, row: usize) -> LayoutElement {
    LayoutElement::new(id, left + span / 2.0, row as f64 * ROW + 16.0, span, 32.0)
}

fn input_of(elements: &[LayoutElement]) -> LayoutInput {
    LayoutInput {
        elements: elements.to_vec(),
        connections: vec![],
    }
}

fn placed(elements: &[LayoutElement]) -> Placements {
    RowPackedLayout.place(&input_of(elements), &definition(), None).unwrap()
}

fn left_of(placement: &LayoutPlacement) -> f64 {
    placement.x - WIDTH / 2.0
}

fn manual_left(element: &LayoutElement) -> f64 {
    element.x - element.width / 2.0
}

/// A deterministic shuffle, so a failure reproduces; in floating point, as the standalone host's test computes it.
fn shuffled<T: Clone>(items: &[T], seed: u32) -> Vec<T> {
    let mut copy = items.to_vec();
    let mut state = f64::from(seed);
    for index in (1..copy.len()).rev() {
        state = (state * 1103515245.0 + 12345.0) % 2147483648.0;
        let other = (state % (index as f64 + 1.0)) as usize;
        copy.swap(index, other);
    }
    copy
}

/// Fifty trends over five rows, with repeated starts and wildly different spans.
fn fifty() -> Vec<LayoutElement> {
    (0..50usize)
        .map(|index| {
            let left = ((index * 37) % 400 - ((index * 37) % 400) % 8) as f64;
            trend(
                &format!("t{index:02}"),
                left,
                (4 + (index * 53) % 900) as f64,
                index % 5,
            )
        })
        .collect()
}

#[test]
fn keeps_time_order_an_earlier_start_is_never_placed_right_of_a_later_one() {
    let all = fifty();
    let positions = placed(&shuffled(&all, 7));

    for a in &all {
        for b in &all {
            if manual_left(a) < manual_left(b) {
                assert!(
                    left_of(&positions[&a.id]) <= left_of(&positions[&b.id]),
                    "{} and {}",
                    a.id,
                    b.id
                );
            }
        }
    }
}

#[test]
fn never_lets_two_elements_on_one_row_come_closer_than_the_gap() {
    let all = fifty();
    let positions = placed(&all);

    for row in 0..5 {
        let y = row as f64 * ROW + 16.0;
        let mut lefts: Vec<f64> = all
            .iter()
            .filter(|element| element.y == y)
            .map(|element| left_of(&positions[&element.id]))
            .collect();
        lefts.sort_by(f64::total_cmp);
        for pair in lefts.windows(2) {
            assert!(pair[1] - pair[0] >= WIDTH + GAP);
        }
    }
}

#[test]
fn places_each_element_as_far_left_as_time_order_and_its_row_allow() {
    // a and b share a row; c starts after a on another row, so only a's position bounds it.
    let positions = placed(&[
        trend("a", 0.0, 400.0, 0),
        trend("b", 100.0, 8.0, 0),
        trend("c", 50.0, 8.0, 1),
    ]);

    assert_eq!(left_of(&positions["a"]), 0.0);
    assert_eq!(left_of(&positions["c"]), 0.0);
    assert_eq!(left_of(&positions["b"]), WIDTH + GAP);
}

#[test]
fn gives_the_same_placement_for_every_order_the_elements_arrive_in() {
    let all = fifty();
    let reference = placed(&all);

    for seed in [1, 2, 3, 99] {
        assert_eq!(placed(&shuffled(&all, seed)), reference, "seed {seed}");
    }
}

#[test]
fn lets_equal_starts_on_different_rows_share_an_x_whichever_way_their_ids_sort() {
    // z is pushed by its crowded row; a starts with it on an empty row and must still share its x.
    let elements = [
        trend("z", 20.0, 8.0, 0),
        trend("a", 20.0, 8.0, 1),
        trend("crowd", 0.0, 8.0, 0),
    ];

    let positions = placed(&elements);

    assert_eq!(left_of(&positions["a"]), left_of(&positions["z"]));
}

#[test]
fn draws_every_element_at_the_declared_width_on_the_row_it_already_sits_on() {
    let all = fifty();
    let positions = placed(&all);

    for element in &all {
        assert_eq!(positions[&element.id].width, Some(WIDTH));
        assert_eq!(positions[&element.id].y, element.y);
    }
}

#[test]
fn keeps_the_room_an_element_needs_before_it_clear_of_the_element_before_it_on_its_row() {
    let named = LayoutElement {
        leading: Some(100.0),
        ..trend("b", 100.0, 8.0, 0)
    };
    let positions = placed(&[trend("a", 0.0, 8.0, 0), named]);

    assert_eq!(left_of(&positions["b"]), WIDTH + GAP + 100.0);
}

#[test]
fn lays_out_as_manual_where_the_definition_declares_no_row_packed() {
    let definition = LayoutDefinition::with_modes(&[LayoutMode::RowPacked]);

    assert!(RowPackedLayout.place(&input_of(&fifty()), &definition, None).is_none());
}

// ---- row-packed inverse ---------------------------------------------------------------------------------------------

/// One row, so the two are placed apart and a point can fall between them.
fn pair() -> [LayoutElement; 2] {
    [trend("early", 0.0, 40.0, 0), trend("late", 400.0, 40.0, 0)]
}

fn inverse(point: Point, elements: &[LayoutElement], definition: &LayoutDefinition) -> Point {
    RowPackedLayout.inverse(point, &input_of(elements), definition).unwrap()
}

#[test]
fn inverse_interpolates_between_the_two_placed_neighbours_of_a_point() {
    let positions = placed(&pair());
    let early = left_of(&positions["early"]);
    let late = left_of(&positions["late"]);

    let manual = inverse(
        Point {
            x: (early + late) / 2.0,
            y: 100.0,
        },
        &pair(),
        &definition(),
    );

    assert_eq!(manual, Point { x: 200.0, y: 100.0 });
}

#[test]
fn inverse_extrapolates_beyond_either_end_at_the_same_distance_from_the_nearest_element() {
    let positions = placed(&pair());
    let late = left_of(&positions["late"]);

    assert_eq!(inverse(Point { x: -30.0, y: 0.0 }, &pair(), &definition()).x, -30.0);
    assert_eq!(
        inverse(Point { x: late + 10.0, y: 0.0 }, &pair(), &definition()).x,
        410.0
    );
}

#[test]
fn inverse_leaves_the_point_unchanged_with_nothing_placed() {
    assert_eq!(
        inverse(Point { x: 123.0, y: 45.0 }, &[], &definition()),
        Point { x: 123.0, y: 45.0 }
    );
}

#[test]
fn inverse_maps_a_drop_between_two_trends_on_different_rows_to_a_start_that_is_placed_between_them() {
    let positions = placed(&pair());
    let early = left_of(&positions["early"]);
    let late = left_of(&positions["late"]);
    let drop = Point {
        x: (early + late) / 2.0,
        y: 2.0 * ROW + 16.0,
    };

    let manual = inverse(drop, &pair(), &definition());
    let dropped = trend("new", manual.x, 40.0, 2);
    let mut all = pair().to_vec();
    all.push(dropped);
    let after = placed(&all);

    assert!(manual.x > 0.0);
    assert!(manual.x < 400.0);
    assert!(left_of(&after["new"]) >= left_of(&after["early"]));
    assert!(left_of(&after["new"]) <= left_of(&after["late"]));
}

// ---- row-packed placement with a width per type and elements across rows ---------------------------------------------
//
// A population mixing types: trends take the declared width, while dots and notes keep their own, the notes two rows
// tall. Rows are 56 apart, and an element covers every row line its span crosses, so the properties above are proven
// again per row line, not per centre.

fn mixed() -> LayoutDefinition {
    LayoutDefinition {
        row_packed: Some(RowPackedDeclaration {
            types: Some(vec!["trend".to_string()]),
            row_step: Some(ROW),
            ..RowPackedDeclaration::new(WIDTH, GAP)
        }),
        ..LayoutDefinition::with_modes(&[LayoutMode::Manual, LayoutMode::RowPacked])
    }
}

/// An element by type, manual left edge, width, top and height; the layout reads centres.
fn shaped(id: &str, kind: &str, left: f64, width: f64, top: f64, height: f64) -> LayoutElement {
    LayoutElement {
        element_type: Some(kind.to_string()),
        ..LayoutElement::new(id, left + width / 2.0, top + height / 2.0, width, height)
    }
}

fn trend_at(id: &str, left: f64, span: f64, row: usize) -> LayoutElement {
    shaped(id, "trend", left, span, row as f64 * ROW, 32.0)
}

fn dot_at(id: &str, left: f64, row: usize) -> LayoutElement {
    shaped(id, "dot", left - 8.0, 16.0, row as f64 * ROW + 8.0, 16.0)
}

fn note_at(id: &str, left: f64, row: usize) -> LayoutElement {
    shaped(id, "note", left, 100.0, row as f64 * ROW, 100.0)
}

fn mixed_population() -> Vec<LayoutElement> {
    (0..45usize)
        .map(|index| {
            let left = ((index * 37) % 400 - ((index * 37) % 400) % 8) as f64;
            let row = index % 5;
            let id = format!("e{index}");
            match index % 3 {
                0 => trend_at(&id, left, (4 + (index * 53) % 900) as f64, row),
                1 => dot_at(&id, left, row),
                _ => note_at(&id, left, row),
            }
        })
        .collect()
}

fn place_mixed(elements: &[LayoutElement], definition: &LayoutDefinition) -> Placements {
    RowPackedLayout.place(&input_of(elements), definition, None).unwrap()
}

/// The placed left and right edges of an element.
fn span(positions: &Placements, element: &LayoutElement) -> (f64, f64) {
    let at = positions[&element.id];
    let width = at.width.unwrap_or(element.width);
    (at.x - width / 2.0, at.x + width / 2.0)
}

/// Every row line an element's span crosses.
fn row_lines(element: &LayoutElement) -> Vec<i64> {
    let first = ((element.y - element.height / 2.0) / ROW).floor() as i64;
    let last = ((element.y + element.height / 2.0 - 1.0) / ROW).floor() as i64;
    (first..=last).collect()
}

#[test]
fn mixed_keeps_time_order_across_types() {
    let all = mixed_population();
    let positions = place_mixed(&shuffled(&all, 5), &mixed());

    for a in &all {
        for b in &all {
            if manual_left(a) < manual_left(b) {
                assert!(span(&positions, a).0 <= span(&positions, b).0, "{} and {}", a.id, b.id);
            }
        }
    }
}

#[test]
fn mixed_never_lets_two_elements_covering_one_row_line_come_closer_than_the_gap() {
    let all = mixed_population();
    let positions = place_mixed(&all, &mixed());

    let mut lines: std::collections::BTreeMap<i64, Vec<(f64, f64)>> = std::collections::BTreeMap::new();
    for element in &all {
        for line in row_lines(element) {
            lines.entry(line).or_default().push(span(&positions, element));
        }
    }
    for spans in lines.values_mut() {
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in spans.windows(2) {
            assert!(pair[1].0 - pair[0].1 >= GAP);
        }
    }
}

#[test]
fn mixed_keeps_nothing_on_the_first_row_of_a_two_row_note_from_overlapping_it() {
    // The note covers rows 0 and 1, its centre on row line 0's far side at y 50; a trend on row 0 starts after it and
    // must clear it. Keyed by the centre alone, row 0 would look empty.
    let note = note_at("note", 0.0, 0);
    let later = trend_at("later", 10.0, 40.0, 0);
    let below = trend_at("below", 20.0, 40.0, 1);

    let positions = place_mixed(&[note, later.clone(), below.clone()], &mixed());

    assert!(
        span(&positions, &later).0 >= 100.0 + GAP,
        "the trend on the note's first row overlaps it"
    );
    assert!(
        span(&positions, &below).0 >= 100.0 + GAP,
        "the trend on the note's second row overlaps it"
    );
}

#[test]
fn mixed_places_each_element_as_far_left_as_time_order_and_its_rows_allow() {
    let positions = place_mixed(
        &[
            trend_at("a", 0.0, 400.0, 0),
            dot_at("d", 100.0, 0),
            trend_at("c", 50.0, 8.0, 1),
        ],
        &mixed(),
    );

    assert_eq!(span(&positions, &trend_at("a", 0.0, 400.0, 0)).0, 0.0);
    assert_eq!(span(&positions, &trend_at("c", 50.0, 8.0, 1)).0, 0.0);
    assert_eq!(span(&positions, &dot_at("d", 100.0, 0)).0, WIDTH + GAP);
}

#[test]
fn mixed_gives_the_same_placement_for_every_order_the_elements_arrive_in() {
    let all = mixed_population();
    let reference = place_mixed(&all, &mixed());

    for seed in [1, 2, 3, 99] {
        assert_eq!(place_mixed(&shuffled(&all, seed), &mixed()), reference, "seed {seed}");
    }
}

#[test]
fn mixed_draws_the_listed_type_at_the_declared_width_and_leaves_every_other_its_own() {
    let all = mixed_population();
    let positions = place_mixed(&all, &mixed());

    for element in &all {
        let expected = (element.element_type.as_deref() == Some("trend")).then_some(WIDTH);
        assert_eq!(positions[&element.id].width, expected, "{}", element.id);
        assert_eq!(positions[&element.id].y, element.y);
    }
}

#[test]
fn mixed_gives_every_element_the_declared_width_when_no_types_are_listed() {
    let all = mixed_population();
    let definition = LayoutDefinition {
        row_packed: Some(RowPackedDeclaration::new(WIDTH, GAP)),
        ..mixed()
    };
    let positions = place_mixed(&all, &definition);

    for element in &all {
        assert_eq!(positions[&element.id].width, Some(WIDTH));
    }
}

#[test]
fn mixed_inverts_a_drop_between_a_listed_and_an_unlisted_element_to_a_manual_x_between_theirs() {
    let pair = [trend_at("early", 0.0, 40.0, 0), dot_at("late", 400.0, 0)];
    let positions = place_mixed(&pair, &mixed());
    let early = span(&positions, &pair[0]).0;
    let late = span(&positions, &pair[1]).0;

    let manual = inverse(
        Point {
            x: (early + late) / 2.0,
            y: 3.0 * ROW + 16.0,
        },
        &pair,
        &mixed(),
    );

    assert!(manual.x > 0.0);
    assert!(manual.x < 400.0 - 8.0);
}

// ---- row-packed placement with a width per element and connections followed -----------------------------------------

fn base() -> LayoutDefinition {
    LayoutDefinition {
        row_packed: Some(RowPackedDeclaration {
            width: RowPackedWidth::Bound,
            row_step: Some(ROW),
            ..RowPackedDeclaration::new(0.0, GAP)
        }),
        ..LayoutDefinition::with_modes(&[LayoutMode::Manual, LayoutMode::RowPacked])
    }
}

fn at(id: &str, left: f64, row: usize, packed_width: Option<f64>) -> LayoutElement {
    LayoutElement {
        packed_width,
        ..LayoutElement::new(id, left + 20.0, row as f64 * ROW + 16.0, 40.0, 32.0)
    }
}

fn placed_left(positions: &Placements, element: &LayoutElement) -> f64 {
    let placed = positions[&element.id];
    placed.x - placed.width.unwrap_or(element.width) / 2.0
}

#[test]
fn bound_draws_each_element_at_its_own_resolved_width_and_at_its_own_width_where_the_binding_gave_none() {
    let wide = at("wide", 0.0, 0, Some(96.0));
    let half = at("half", 100.0, 1, Some(48.0));
    let unbound = at("unbound", 200.0, 2, None);

    let positions = RowPackedLayout
        .place(
            &LayoutInput {
                elements: vec![wide, half, unbound],
                connections: vec![],
            },
            &base(),
            None,
        )
        .unwrap();

    assert_eq!(
        [
            positions["wide"].width,
            positions["half"].width,
            positions["unbound"].width
        ],
        [Some(96.0), Some(48.0), Some(40.0)]
    );
}

#[test]
fn bound_starts_an_effect_after_the_middle_of_its_earlier_cause_on_another_row_only_when_connections_are_followed() {
    let cause = at("cause", 0.0, 0, Some(96.0));
    let effect = at("effect", 10.0, 1, Some(96.0));
    let input = LayoutInput {
        elements: vec![cause, effect.clone()],
        connections: vec![LayoutConnection::new("cause", "effect")],
    };
    let mut following = base();
    if let Some(declaration) = following.row_packed.as_mut() {
        declaration.follow_connections = true;
    }

    let followed = RowPackedLayout.place(&input, &following, None).unwrap();
    let ignored = RowPackedLayout.place(&input, &base(), None).unwrap();

    assert_eq!(placed_left(&followed, &effect), 48.0 + GAP);
    assert_eq!(placed_left(&ignored, &effect), 0.0);
}

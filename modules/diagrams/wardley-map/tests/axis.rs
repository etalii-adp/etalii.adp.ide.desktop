//! The one conversion from the document's axes to the canvas, and the four evolution stages (etalii.adp spec 030,
//! task T070), ported from the standalone host's `WardleyAxis.Tests.cs` and `WardleyEvolution.Tests.cs` at `8b6ccfc`.

use adp_diagram_wardley_map::axis;
use adp_diagram_wardley_map::model::WardleyCoordinate;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

#[test]
fn to_point_puts_a_highly_visible_genesis_component_at_top_left() {
    let (x, y) = axis::to_point(WardleyCoordinate::new(0.9, 0.1));
    assert_eq!(x, 0.1);
    assert!(close(y, 0.09999999999999998), "{y}");
}

#[test]
fn to_point_puts_an_invisible_commodity_component_at_bottom_right() {
    let (x, y) = axis::to_point(WardleyCoordinate::new(0.05, 0.95));
    assert_eq!(x, 0.95);
    assert!(close(y, 0.95), "{y}");
}

#[test]
fn to_point_puts_the_anchor_at_the_top() {
    let (_, y) = axis::to_point(WardleyCoordinate::new(1.0, 0.5));
    assert_eq!(y, 0.0);
}

#[test]
fn to_coordinate_inverts_to_point_exactly() {
    for (visibility, maturity) in [(0.0, 0.0), (1.0, 1.0), (0.79, 0.61), (0.5, 0.5), (0.175, 0.7)] {
        let original = WardleyCoordinate::new(visibility, maturity);
        let (x, y) = axis::to_point(original);
        let round = axis::to_coordinate(x, y);
        assert!(close(original.visibility, round.visibility), "{visibility}");
        assert!(close(original.maturity, round.maturity), "{maturity}");
    }
}

#[test]
fn clamp_holds_a_coordinate_inside_the_bounded_space() {
    let clamped = axis::clamp(WardleyCoordinate::new(1.4, -0.3));
    assert_eq!(clamped.visibility, 1.0);
    assert_eq!(clamped.maturity, 0.0);
}

#[test]
fn clamp_leaves_a_coordinate_that_is_already_inside_alone() {
    let coordinate = WardleyCoordinate::new(0.79, 0.61);
    assert_eq!(axis::clamp(coordinate), coordinate);
}

#[test]
fn to_point_and_the_evolution_bands_agree_on_where_genesis_ends() {
    let (x, _) = axis::to_point(WardleyCoordinate::new(0.5, axis::CUSTOM_BUILT));
    assert_eq!(x, axis::CUSTOM_BUILT);
    assert_eq!(axis::stage_of(x).label, "Custom Built");
}

// ---- WardleyEvolution.Tests.cs --------------------------------------------------------------------------------------

#[test]
fn boundaries_are_the_reference_renderers_offsets_divided_by_the_axis_width() {
    let axis_width = 20.0;
    assert_eq!(axis::CUSTOM_BUILT, 3.5 / axis_width);
    assert_eq!(axis::PRODUCT, 8.0 / axis_width);
    assert_eq!(axis::COMMODITY, 14.0 / axis_width);
}

#[test]
fn boundaries_are_the_values_the_spec_states() {
    assert_eq!(axis::CUSTOM_BUILT, 0.175);
    assert_eq!(axis::PRODUCT, 0.400);
    assert_eq!(axis::COMMODITY, 0.700);
}

#[test]
fn stages_are_four_and_cover_the_whole_axis_without_gaps_or_overlaps() {
    let stages = axis::STAGES;
    assert_eq!(stages.len(), 4);
    assert_eq!(stages[0].start, 0.0);
    assert_eq!(stages[3].end, 1.0);
    for index in 1..stages.len() {
        assert_eq!(stages[index - 1].end, stages[index].start);
    }
}

#[test]
fn stages_carry_the_notations_own_labels_parentheticals_included() {
    let labels: Vec<&str> = axis::STAGES.iter().map(|s| s.label).collect();
    assert_eq!(
        labels,
        ["Genesis", "Custom Built", "Product (+rental)", "Commodity (+utility)"]
    );
}

#[test]
fn stage_of_reads_a_maturity_against_the_bands() {
    for (maturity, expected) in [
        (0.0, "Genesis"),
        (0.1, "Genesis"),
        (0.174, "Genesis"),
        (0.175, "Custom Built"),
        (0.3, "Custom Built"),
        (0.399, "Custom Built"),
        (0.4, "Product (+rental)"),
        (0.5, "Product (+rental)"),
        (0.699, "Product (+rental)"),
        (0.7, "Commodity (+utility)"),
        (0.9, "Commodity (+utility)"),
        (1.0, "Commodity (+utility)"),
    ] {
        assert_eq!(axis::stage_of(maturity).label, expected, "{maturity}");
    }
}

#[test]
fn stage_of_clamps_rather_than_throwing_for_a_coordinate_outside_the_axis() {
    assert_eq!(axis::stage_of(-0.5).label, "Genesis");
    assert_eq!(axis::stage_of(1.5).label, "Commodity (+utility)");
}

#[test]
fn stage_of_agrees_with_the_stage_ranges_it_is_reading_against() {
    let mut maturity = 0.0;
    while maturity <= 1.0 {
        let stage = axis::stage_of(maturity);
        assert!(
            maturity >= stage.start && (maturity < stage.end || (stage.end - 1.0).abs() < 0.00001),
            "{maturity} was read as {}, whose range is {}..{}.",
            stage.label,
            stage.start,
            stage.end
        );
        maturity += 0.001;
    }
}

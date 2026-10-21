//! The text width metric (etalii.adp spec 030, FR-018, task T054), ported from the standalone host's
//! `EtAlii.Adp.Layout.Tests/TextMetric.Tests.cs` at `8b6ccfc` and checked against its golden fixture,
//! `src/fixtures/cross-tier/text-metric.json`, copied to `tests/fixtures/` (backend-centralization R10.1, R10.3).
//!
//! The fixture is the specification; these tests only apply it, so a case added to it is checked here without
//! editing this file. Padding, minimum and maximum are each module's own (R10.2), so no case includes them. The
//! standalone host's `ANullText_IsRefused` has no counterpart: a Rust `&str` cannot be null.

use std::fs;
use std::path::Path;

use adp_layout::text_metric::{AVERAGE_ADVANCE, DEFAULT_FONT_SIZE, width_at_default_size, width_of};
use serde_json::Value;

struct Fixture {
    reason: String,
    average_advance: f64,
    default_font_size: f64,
    tolerance: f64,
    cases: Vec<Case>,
}

struct Case {
    text: String,
    font_size: f64,
    width: f64,
    why: String,
}

fn number(value: &Value, name: &str) -> f64 {
    value[name].as_f64().unwrap_or_else(|| panic!("no number {name}"))
}

fn text(value: &Value, name: &str) -> String {
    value[name]
        .as_str()
        .unwrap_or_else(|| panic!("no text {name}"))
        .to_string()
}

fn fixture() -> Fixture {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text-metric.json");
    let json: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    Fixture {
        reason: text(&json, "reason"),
        average_advance: number(&json, "averageAdvance"),
        default_font_size: number(&json, "defaultFontSize"),
        tolerance: number(&json, "tolerance"),
        cases: json["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|case| Case {
                text: text(case, "text"),
                font_size: number(case, "fontSize"),
                width: number(case, "width"),
                why: text(case, "why"),
            })
            .collect(),
    }
}

/// A text's length as the standalone host counts it: in UTF-16 code units.
fn length(text: &str) -> usize {
    text.encode_utf16().count()
}

#[test]
fn the_fixture_loaded_and_carries_the_cases_the_design_names() {
    let fixture = fixture();

    // The case test loops over the fixture, so a fixture that parsed to an empty list would let it pass by checking
    // nothing. This is the test that cannot pass that way.
    assert!(!fixture.reason.is_empty());
    assert!(fixture.tolerance > 0.0, "the fixture names no tolerance");

    // The cases the design's cross-tier table names by kind, so none can be dropped unnoticed: an empty string, one
    // character, a long string and a string with spaces, at two font sizes.
    assert!(fixture.cases.iter().any(|case| length(&case.text) == 0));
    assert!(fixture.cases.iter().any(|case| length(&case.text) == 1));
    assert!(fixture.cases.iter().any(|case| length(&case.text) > 50));
    assert!(fixture.cases.iter().any(|case| case.text.contains(' ')));
    let mut sizes: Vec<f64> = fixture.cases.iter().map(|case| case.font_size).collect();
    sizes.sort_by(f64::total_cmp);
    sizes.dedup();
    assert!(sizes.len() >= 2, "the fixture has fewer than two font sizes");
}

#[test]
fn the_shared_values_are_the_ones_the_fixture_states() {
    let fixture = fixture();

    // The standalone host's client reads these two numbers from the fixture rather than from the metric, so a change
    // here that the fixture does not carry would leave the hosts measuring differently.
    assert_eq!(AVERAGE_ADVANCE, fixture.average_advance);
    assert_eq!(DEFAULT_FONT_SIZE, fixture.default_font_size);
}

#[test]
fn every_case_measures_to_its_width() {
    let fixture = fixture();
    let failures: Vec<String> = fixture
        .cases
        .iter()
        .filter_map(|case| {
            let width = width_of(&case.text, case.font_size);
            ((width - case.width).abs() > fixture.tolerance).then(|| {
                format!(
                    "\"{}\" at {}: measured {width}, expected {} ({})",
                    case.text, case.font_size, case.width, case.why
                )
            })
        })
        .collect();

    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn the_default_font_size_is_the_one_the_fixture_measures_at() {
    let fixture = fixture();
    let failures: Vec<String> = fixture
        .cases
        .iter()
        .filter(|case| (case.font_size - fixture.default_font_size).abs() < f64::EPSILON)
        .filter(|case| (width_at_default_size(&case.text) - case.width).abs() > fixture.tolerance)
        .map(|case| {
            format!(
                "\"{}\": measured {}, expected {}",
                case.text,
                width_at_default_size(&case.text),
                case.width
            )
        })
        .collect();

    assert!(failures.is_empty(), "{failures:#?}");
}

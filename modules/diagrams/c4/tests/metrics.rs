//! C4's text sizing measured against the shared text metric's golden fixture (backend-centralization R10.1,
//! R10.2): each case's width is the fixture's text width with this module's own padding, minimum, maximum and rounding
//! around it. Ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4Metrics.Tests.cs` at `8b6ccfc`; the fixture is the standalone
//! host's `src/fixtures/cross-tier/text-metric.json`, vendored in `crates/adp-layout/tests/fixtures/`.

use std::fs;
use std::path::Path;

use adp_diagram_c4::metrics::{C4Metrics, round2};

#[test]
fn a_texts_width_is_the_shared_metrics_width_with_this_modules_own_sizing_around_it() {
    let metrics = C4Metrics::default();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../crates/adp-layout/tests/fixtures/text-metric.json");
    let fixture: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
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
        let expected =
            round2((width + 2.0 * metrics.horizontal_padding).clamp(metrics.minimum_width, metrics.maximum_width));
        let actual = metrics.measure(&text, "", "").width;
        if (actual - expected).abs() > 0.005 {
            failures.push(format!("{text:?}: sized {actual}, expected {expected}"));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
}

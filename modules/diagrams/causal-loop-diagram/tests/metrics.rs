//! The causal loop diagram's text sizing against the shared text metric's golden fixture: the standalone host's
//! `CausalLoopMetricsTests` at `8b6ccfc`. The fixture is the standalone host's `src/fixtures/cross-tier/text-metric.json`,
//! as `adp-layout` keeps it.

use std::fs;
use std::path::Path;

use adp_diagram_causal_loop::metrics::Metrics;

#[test]
fn a_texts_width_is_the_shared_metrics_width_with_this_modules_own_sizing_around_it() {
    let metrics = Metrics::default();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../crates/adp-layout/tests/fixtures/text-metric.json");
    let fixture: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let cases: Vec<(String, f64)> = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| (c["fontSize"].as_f64().unwrap() - metrics.font_size).abs() < 1e-6)
        .map(|c| (c["text"].as_str().unwrap().to_owned(), c["width"].as_f64().unwrap()))
        .collect();
    assert!(!cases.is_empty());
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|(text, width)| {
            let expected = metrics.minimum_width.max(width + 2.0 * metrics.horizontal_padding);
            let actual = metrics.width_of(text);
            ((actual - expected).abs() > 1e-9).then(|| format!("\"{text}\": sized {actual}, expected {expected}"))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn every_variable_is_one_line_of_text_tall() {
    assert!((Metrics::default().height() - 39.6).abs() < 1e-9);
}

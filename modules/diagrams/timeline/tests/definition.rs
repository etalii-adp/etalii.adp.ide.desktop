//! The timeline's specification loads with the module's plugin functions, and an expression calls them rather than
//! the specification's fallbacks (the standalone host's `BundledDefinitionTests` at `8b6ccfc`; the bundling and
//! provenance checks are `tools/sync-etalii-adp.py`'s here).

mod common;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use adp_cel::Value;
use adp_disl::{DislLoader, DislSpecification, contexts};

fn specification_bytes() -> Vec<u8> {
    fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor/etalii.adp/definitions/diagrams/timeline.dis"))
        .unwrap()
}

fn specification() -> DislSpecification {
    DislLoader::load_bytes(&specification_bytes(), &adp_diagram_timeline::times::plugin_functions())
        .specification
        .expect("the timeline's specification loads")
}

#[test]
fn the_specification_is_this_modules_language() {
    let specification = specification();

    assert_eq!("net.etalii.adp.generic.timeline", specification.language_id());
    assert_eq!(
        Some(adp_diagram_timeline::ORIGIN),
        specification
            .root()
            .get("language")
            .and_then(|l| l.get("origin"))
            .and_then(|o| o.as_str())
    );
}

#[test]
fn the_specification_loads_without_diagnostics() {
    let loaded = DislLoader::load_bytes(&specification_bytes(), &adp_diagram_timeline::times::plugin_functions());

    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
}

#[test]
fn the_modules_plugin_functions_are_the_ones_called() {
    let specification = specification();
    let cases: [(&str, Value); 5] = [
        ("readableTime('July 5, 2026')", Value::Bool(true)),
        ("readableTime(' 2026-07-05 ')", Value::Bool(true)),
        ("readableTime('2026-02-30')", Value::Bool(false)),
        (
            "compareTimes('2026-07-05T00:00:00+02:00', '2026-07-04T23:00:00')",
            Value::Int(-1),
        ),
        ("dateAfter('2026-01-05T22:00:00', 6)", Value::from("2026-01-11")),
    ];
    for (expression, expected) in cases {
        let program = specification
            .environment(contexts::ELEMENT, &[])
            .compile(expression)
            .unwrap();

        let value = program.evaluate(&HashMap::<String, Value>::new()).unwrap();

        assert_eq!(expected, value, "{expression}");
    }
}

#[test]
fn a_time_that_does_not_read_is_an_error_where_one_is_needed() {
    let specification = specification();
    let program = specification
        .environment(contexts::ELEMENT, &[])
        .compile("dateAfter('someday', 1)")
        .unwrap();

    let error = program.evaluate(&HashMap::<String, Value>::new()).unwrap_err();

    assert!(
        error.contains("'someday' is not a time this timeline can read."),
        "{error}"
    );
}

#[test]
fn the_catalogue_opens_a_timeline_with_the_module() {
    let mut tools = common::tools();

    let tool = tools.get(adp_diagram_timeline::ORIGIN).unwrap();

    assert!(tool.module.is_some());
    assert!(
        tool.definition.diagnostics().is_empty(),
        "{:?}",
        tool.definition.diagnostics()
    );
}

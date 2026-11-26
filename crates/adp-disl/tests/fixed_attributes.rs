//! A fixed attribute (DISL section 4.3): it always has its value, which a stored one does not change.

mod common;

use adp_cel::Value;
use adp_disl::DislDiagram;
use common::{attrs, loaded, with};

#[test]
fn a_fixed_attribute_reads_its_value_whatever_is_stored() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Card": { "attributes": {
            "height": { "type": "number", "fixed": 48 },
            "width": { "type": "number" } } } } },
        "notation": { "nodes": { "Card": { "shape": "rect" } } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let plain = diagram
        .add_node("Card", "a", attrs([("width", Value::Double(140.0))]))
        .unwrap();
    let stored = diagram
        .add_node("Card", "b", attrs([("height", Value::Double(100.0))]))
        .unwrap();
    assert_eq!(Value::Double(48.0), plain.value_of("height"));
    assert_eq!(Value::Double(48.0), stored.value_of("height"));
    assert_eq!(Value::Double(140.0), plain.value_of("width"));
}

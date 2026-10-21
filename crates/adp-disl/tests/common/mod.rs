//! What the ported tests share (the standalone host's `Specifications`, `Resources` and `Derivations`): small
//! specifications, the two bundled definitions and their wire-id maps, and the diagrams the derivation tests run
//! against.

#![allow(dead_code)]

use adp_cel::{Map, Value};
use adp_disl::bundling::BundledDefinition;
use adp_disl::{DislDiagram, DislElement, DislLoader, DislPluginFunctions, DislSpecification, WireIdMap};

pub const HYPE_CYCLE: &str = "gartner-hype-cycle-graph.dis";
pub const BEHAVIOR_MODEL: &str = "agent-behavior-modelling.dis";

/// The bytes of a bundled definition and of its provenance, as a module embeds them.
pub fn resource(logical_name: &str) -> (&'static [u8], &'static [u8]) {
    match logical_name {
        HYPE_CYCLE => (
            include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/gartner-hype-cycle-graph.dis"),
            include_bytes!("../fixtures/gartner-hype-cycle-graph.provenance.json"),
        ),
        BEHAVIOR_MODEL => (
            include_bytes!("../../../../vendor/etalii.adp/definitions/diagrams/agent-behavior-modelling.dis"),
            include_bytes!("../fixtures/agent-behavior-modelling.provenance.json"),
        ),
        other => panic!("No resource {other}."),
    }
}

pub fn text(logical_name: &str) -> String {
    String::from_utf8(resource(logical_name).0.to_vec()).expect("a definition is UTF-8")
}

pub fn bundled(logical_name: &str) -> BundledDefinition {
    let (bytes, provenance) = resource(logical_name);
    BundledDefinition::load(logical_name, bytes, provenance, &DislPluginFunctions::none()).expect("the definition loads")
}

thread_local! {
    static HYPE: DislSpecification = bundled(HYPE_CYCLE).specification;
    static BEHAVIOR: DislSpecification = bundled(BEHAVIOR_MODEL).specification;
}

pub fn hype_cycle() -> DislSpecification {
    HYPE.with(Clone::clone)
}

pub fn behavior_model() -> DislSpecification {
    BEHAVIOR.with(Clone::clone)
}

pub fn hype_cycle_ids() -> WireIdMap {
    WireIdMap::of(&hype_cycle(), "x-ghg")
}

pub fn behavior_model_ids() -> WireIdMap {
    WireIdMap::of(&behavior_model(), "x-abm")
}

/// A minimal valid specification with `parts` (JSON object members) added or replacing its own.
pub fn with(parts: &str) -> String {
    let mut root: serde_json::Value = serde_json::from_str(
        r#"{
          "disl": "0.2",
          "language": { "id": "org.example.test", "version": "1.0.0" },
          "metamodel": { "types": { "Thing": { "attributes": { "name": { "type": "string" } } } } }
        }"#,
    )
    .expect("the base specification is JSON");
    if !parts.trim().is_empty() {
        let added: serde_json::Value =
            serde_json::from_str(&format!("{{{parts}}}")).expect("the parts are JSON object members");
        let object = root.as_object_mut().expect("an object");
        for (key, value) in added.as_object().expect("an object") {
            object.insert(key.clone(), value.clone());
        }
    }
    root.to_string()
}

/// Loads `json` and gives the specification, failing with the diagnostics when it does not load.
pub fn loaded(json: &str) -> DislSpecification {
    let result = DislLoader::load(json);
    match result.specification {
        Some(specification) => specification,
        None => panic!(
            "The specification does not load: {}",
            result.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
        ),
    }
}

/// The diagnostics of `json` as text a test can compare.
pub fn diagnostics(json: &str) -> Vec<String> {
    DislLoader::load(json).diagnostics.iter().map(ToString::to_string).collect()
}

/// Attributes from pairs.
pub fn attrs<const N: usize>(pairs: [(&str, Value); N]) -> Map {
    pairs.into_iter().collect()
}

pub fn strings(items: &[&str]) -> Value {
    Value::list(items.iter().map(|s| Value::from(*s)).collect())
}

/// Variables from pairs.
pub fn vars<const N: usize>(pairs: [(&str, Value); N]) -> std::collections::HashMap<String, Value> {
    pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect()
}

/// A hype cycle graph: two trends (the second with a dragged peak end), a trigger, a note and three influences, one
/// from a hidden phase.
pub fn hype_cycle_diagram() -> DislDiagram {
    let diagram = DislDiagram::new(&hype_cycle());
    let steam = diagram
        .add_node(
            "Trend",
            "steam",
            attrs([
                ("name", "Steam engine".into()),
                ("start", Value::Int(1800 * 12)),
                ("stop", Value::Int(1850 * 12)),
                ("phases", Value::Int(4)),
                ("tags", strings(&["energy", "industry"])),
            ]),
        )
        .unwrap();
    let rail = diagram
        .add_node(
            "Trend",
            "rail",
            attrs([
                ("name", "Railways".into()),
                ("start", Value::Int(1820 * 12)),
                ("stop", Value::Int(1900 * 12)),
                ("phases", Value::Int(2)),
                ("peakEnd", Value::Int(1830 * 12)),
                ("tags", strings(&["transport"])),
            ]),
        )
        .unwrap();
    let watt = diagram
        .add_node(
            "Trigger",
            "watt",
            attrs([
                ("name", "Watt's patent".into()),
                ("date", Value::Int(1769 * 12)),
                ("tags", strings(&["industry", "law"])),
            ]),
        )
        .unwrap();
    diagram
        .add_node(
            "Note",
            "remark",
            attrs([
                ("text", "A remark".into()),
                ("at", Value::Int(1810 * 12)),
                ("width", Value::Double(160.0)),
                ("height", Value::Double(64.5)),
            ]),
        )
        .unwrap();
    diagram
        .add_relation(
            "Influence",
            "i1",
            Some(&steam),
            Some(&rail),
            attrs([
                ("fromPhase", "plateau".into()),
                ("fromEdge", "top".into()),
                ("fromAt", Value::Double(0.5)),
                ("toPhase", "trough".into()),
                ("toEdge", "bottom".into()),
                ("toAt", Value::Double(0.25)),
            ]),
        )
        .unwrap();
    diagram
        .add_relation(
            "Influence",
            "i2",
            Some(&watt),
            Some(&steam),
            attrs([
                ("toPhase", "peak".into()),
                ("toEdge", "top".into()),
                ("toAt", Value::Double(1.0)),
            ]),
        )
        .unwrap();
    diagram
        .add_relation(
            "Influence",
            "i3",
            Some(&rail),
            Some(&steam),
            attrs([
                ("fromPhase", "slope".into()),
                ("fromEdge", "bottom".into()),
                ("fromAt", Value::Double(0.75)),
                ("toPhase", "peak".into()),
                ("toEdge", "bottom".into()),
                ("toAt", Value::Double(0.0)),
            ]),
        )
        .unwrap();
    diagram
}

/// A behavior model: a sequence with a check and a retry under it, the retry still without a child.
pub fn behavior_diagram() -> DislDiagram {
    let diagram = DislDiagram::new(&behavior_model());
    let root = diagram.add_node("Sequence", "1", attrs([("label", "Answer".into())])).unwrap();
    diagram
        .add_child("Check", "1.1", attrs([("label", "Is it known?".into())]), &root, None)
        .unwrap();
    diagram
        .add_child(
            "Retry",
            "1.2",
            attrs([("label", "".into()), ("attempts", Value::Int(3))]),
            &root,
            None,
        )
        .unwrap();
    diagram
}

pub fn element(diagram: &DislDiagram, id: &str) -> DislElement {
    diagram.element_by_id(id).unwrap_or_else(|| panic!("No element {id}."))
}

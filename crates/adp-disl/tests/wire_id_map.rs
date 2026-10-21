//! The host's wire ids, read from a definition's `x-<prefix>` block (the standalone host's `WireIdMapTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::WireIdMap;
use common::{behavior_model_ids, hype_cycle, hype_cycle_ids};

#[test]
fn a_tool_has_its_palette_id_and_its_drop() {
    let ids = hype_cycle_ids();

    assert_eq!("ghg.toolbox.trend", ids.tool_id("trend"));
    assert_eq!(Some("ghg.add.note".to_owned()), ids.drop_action_id("note"));
}

#[test]
fn an_action_is_keyed_by_operation_or_kind_and_a_type_overrides_it() {
    for (type_name, kind, operation, expected) in [
        ("Trend", "editLabel", None, "ghg.rename"),
        ("Trend", "delete", None, "ghg.remove"),
        ("Influence", "delete", None, "ghg.disconnect"),
        ("Trend", "operation", Some("evenPhases"), "ghg.even-phases"),
        ("diagram", "operation", Some("addTriggerHere"), "ghg.add.trigger"),
        ("connection", "connect", None, "ghg.connect.influence"),
    ] {
        assert_eq!(
            expected,
            hype_cycle_ids().action_id(&[type_name.to_owned()], kind, operation, &Map::default())
        );
    }
}

#[test]
fn a_name_in_an_id_is_the_binding_mapped_through_types() {
    let mut bindings = Map::default();
    bindings.insert("kind", Value::from("RepeatUntil"));

    let id = behavior_model_ids().action_id(
        &["Sequence".into(), "Composite".into(), "BehaviorNode".into()],
        "operation",
        Some("addChild"),
        &bindings,
    );

    assert_eq!("abm.add.repeat", id);
}

#[test]
fn a_property_is_keyed_by_its_items_key() {
    for (key, expected) in [
        ("peakInfluences", "ghg.peak-influences"),
        ("troughEnd", "ghg.trough-end"),
        ("From attachment", "ghg.from-attachment"),
        ("unmapped", "unmapped"),
    ] {
        assert_eq!(expected, hype_cycle_ids().property_id(key));
    }
}

#[test]
fn without_a_block_every_name_is_its_own_id() {
    let ids = WireIdMap::of(&hype_cycle(), "x-none");

    assert!(ids.is_none());
    assert_eq!("trend", ids.tool_id("trend"));
    assert_eq!(None, ids.drop_action_id("trend"));
    assert_eq!(
        "evenPhases",
        ids.action_id(&["Trend".into()], "operation", Some("evenPhases"), &Map::default())
    );
    assert_eq!(
        "delete",
        ids.action_id(&["Influence".into()], "delete", None, &Map::default())
    );
}

//! The palette of each bundled definition (DISL sections 7.1 and 7.2) (the standalone host's
//! `ToolboxDerivationTests`).

mod common;

use adp_disl::derivation::toolbox;
use adp_disl::{DerivedTool, WireIdMap};
use common::{behavior_model, behavior_model_ids, hype_cycle, hype_cycle_ids, loaded, with};

fn line(tool: &DerivedTool) -> String {
    format!(
        "{}/{} {} | {} | {} | {} | drops {} | {}",
        tool.group,
        tool.name,
        tool.id,
        tool.creates.as_deref().unwrap_or(""),
        tool.label,
        tool.icon,
        tool.drop_action_id.as_deref().unwrap_or(""),
        tool.description
    )
}

#[test]
fn the_hype_cycle_has_a_trend_a_trigger_and_a_note() {
    let tools = toolbox::derive(&hype_cycle(), &hype_cycle_ids());

    assert_eq!(
        vec![
            "hypeCycle/trend ghg.toolbox.trend | Trend | Trend | mdi-arrow-right-bold-box-outline | drops ghg.add.trend | A trend through the hype cycle. Drop it where it starts; it is a year long with all four phases.",
            "hypeCycle/trigger ghg.toolbox.trigger | Trigger | Trigger | mdi-circle-slice-8 | drops ghg.add.trigger | A moment in time that set trends off - an invention, a political moment, a disaster. Drop it where it happened; draw influences from it.",
            "hypeCycle/note ghg.toolbox.note | Note | Note | mdi-note-text-outline | drops ghg.add.note | A remark of your own, placed where it applies. Drop it and start typing.",
        ],
        tools.iter().map(line).collect::<Vec<_>>()
    );
}

#[test]
fn the_behavior_model_has_one_tool_per_kind() {
    let tools = toolbox::derive(&behavior_model(), &behavior_model_ids());

    assert_eq!(11, tools.len());
    assert_eq!(
        "nodes/sequence abm.toolbox.sequence | Sequence | Do in order | mdi-arrow-right-bold-outline | drops abm.add.sequence | Runs its children one after another, and fails as soon as one fails. Drop it below the node it belongs under.",
        line(&tools[0])
    );
    assert_eq!(
        "nodes/delegate abm.toolbox.delegate | Delegate | Delegate | mdi-account-arrow-right-outline | drops abm.add.delegate | Hands its task to a sub-agent, or follows the behavior file it links to. Drop it below the node it belongs under.",
        line(&tools[10])
    );
}

#[test]
fn without_a_map_the_ids_are_the_specifications_and_nothing_drops() {
    let tools = toolbox::derive(&hype_cycle(), &WireIdMap::none());

    assert_eq!(
        vec!["trend", "trigger", "note"],
        tools.iter().map(|t| t.id.as_str()).collect::<Vec<_>>()
    );
    assert!(tools.iter().all(|t| t.drop_action_id.is_none()));
}

#[test]
fn a_labelless_tool_is_labelled_as_the_type_it_creates_and_a_library_tool_is_found() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Thing": { "label": "A thing", "icon": "mdi-cube", "attributes": { "name": { "type": "string" } } } } },
        "toolbox": {
          "tools": { "shared": { "creates": "Thing", "doc": { "summary": "Shared." } } },
          "groups": [ { "id": "outer", "tools": [ "shared" ], "groups": [ { "id": "inner", "tools": [ { "id": "own", "creates": "Thing", "label": { "cel": "'Own ' + string(size(diagram.nodes))" } } ] } ] } ]
        }"#,
    ));

    let tools = toolbox::derive(&specification, &WireIdMap::none());

    assert_eq!(
        vec![
            "outer/shared shared | Thing | A thing | mdi-cube | drops  | Shared.",
            "inner/own own | Thing | Own 0 | mdi-cube | drops  | "
        ],
        tools.iter().map(line).collect::<Vec<_>>()
    );
}

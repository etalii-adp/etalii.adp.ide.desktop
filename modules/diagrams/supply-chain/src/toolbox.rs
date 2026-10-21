//! The palette: the seven stages and a group, each naming the add action its drop commits (etalii.adp spec 030, task
//! T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainToolboxProvider.cs` at `8b6ccfc`. A stage's icon is its own everywhere: the palette, and the "Add … here"
//! menu on empty canvas, both read [`crate::actions::stage_icon`].

use adp_diagram::ToolboxItem;

use crate::actions::{ADD_GROUP, GROUP_ICON, add_action_id, stage_icon};
use crate::model::stages;

const DESCRIPTIONS: [(&str, &str); 7] = [
    (
        stages::SOURCE,
        "Where something enters the chain: a mine, a well, a farm, a data feed.",
    ),
    (
        stages::PROCESSOR,
        "Turns what it receives into material others can use.",
    ),
    (
        stages::PRODUCER,
        "Makes the parts or components something is built from.",
    ),
    (stages::INTEGRATOR, "Combines parts into the finished whole."),
    (
        stages::HUB,
        "Holds and moves what is finished on to where it is wanted.",
    ),
    (
        stages::OUTLET,
        "Where the finished whole is offered to those who use it.",
    ),
    (stages::CONSUMER, "Where the chain ends: whoever uses what it delivers."),
];

/// The palette, in the order goods pass through the stages, the group last.
pub fn items() -> Vec<ToolboxItem> {
    let mut items: Vec<ToolboxItem> = DESCRIPTIONS
        .iter()
        .map(|(stage, description)| ToolboxItem {
            id: format!("supply-chain.toolbox.{stage}"),
            label: stages::display(stage),
            icon: stage_icon(stage).to_owned(),
            description: (*description).to_owned(),
            drop_action_id: add_action_id(stage),
        })
        .collect();
    items.push(ToolboxItem {
        id: "supply-chain.toolbox.group".to_owned(),
        label: "Group".to_owned(),
        icon: GROUP_ICON.to_owned(),
        description: "A frame for a region, a company or a tier; drag nodes into it.".to_owned(),
        drop_action_id: ADD_GROUP.to_owned(),
    });
    items
}

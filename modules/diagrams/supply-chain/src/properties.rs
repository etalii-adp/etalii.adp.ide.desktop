//! The property rows of a selected node, flow or group, and the command an edited row becomes (etalii.adp spec 030,
//! task T076).
//!
//! Ported from the standalone host's `src/diagrams/supply-chain/backend/EtAlii.Adp.Diagram.SupplyChain/
//! SupplyChainContextPropertyProvider.cs` at `8b6ccfc`. Every row edits through the one property command, so a value
//! typed in the grid is checked by the same rules as one stepped on the canvas. The group row offers names and writes
//! the id: a person picks "Taiwan", the document keeps `taiwan`, so a renamed group keeps its members.

use std::path::Path;

use adp_context::{PropertyEditor, PropertyRow};

use crate::commands::{SupplyChainCommand, SupplyChainEdit, keys};
use crate::lines::format_optional;
use crate::model::{SupplyChainGroup, SupplyChainModel, geometry, stages};

pub const NAME: &str = "supply-chain.name";
pub const STAGE: &str = "supply-chain.stage";
pub const GROUP: &str = "supply-chain.group";
pub const QUANTITY: &str = "supply-chain.quantity";
pub const PRODUCT: &str = "supply-chain.product";
pub const VOLUME: &str = "supply-chain.volume";
pub const UNIT: &str = "supply-chain.unit";
pub const STEP: &str = "supply-chain.step";
pub const DESCRIPTION: &str = "supply-chain.description";

/// The choice that takes a node out of every group.
pub const NO_GROUP: &str = "(none)";

const IDENTITY: &str = "Identity";
const AMOUNT: &str = "Amount";

fn row(id: &str, label: &str, value: &str, editor: PropertyEditor, group: &str) -> PropertyRow {
    PropertyRow {
        group: group.to_owned(),
        ..PropertyRow::new(id, label, editor, value)
    }
}

fn choice(id: &str, label: &str, value: &str, candidates: Vec<String>) -> PropertyRow {
    PropertyRow {
        candidates,
        ..row(id, label, value, PropertyEditor::Choice, IDENTITY)
    }
}

fn name_of(group: &SupplyChainGroup) -> &str {
    if group.name.is_empty() { &group.id } else { &group.name }
}

/// The rows of `element_id`, or none when it names nothing.
pub fn describe(model: &SupplyChainModel, element_id: &str) -> Vec<PropertyRow> {
    if let Some(node) = model.node_of(element_id) {
        let group = model.group_of(&node.group);
        return vec![
            row(NAME, "Name", &node.name, PropertyEditor::Line, IDENTITY),
            choice(
                STAGE,
                "Stage",
                &node.node_type,
                stages::ALL.iter().map(|s| (*s).to_owned()).collect(),
            ),
            choice(GROUP, "Group", group.map_or(NO_GROUP, name_of), group_choices(model)),
            row(
                DESCRIPTION,
                "Description",
                &node.description,
                PropertyEditor::Text,
                IDENTITY,
            ),
            row(
                QUANTITY,
                "Quantity",
                &format_optional(node.quantity),
                PropertyEditor::Line,
                AMOUNT,
            ),
            row(UNIT, "Unit", &node.unit, PropertyEditor::Line, AMOUNT),
            row(
                STEP,
                "Step",
                &format_optional(Some(node.step.unwrap_or(geometry::DEFAULT_STEP))),
                PropertyEditor::Line,
                AMOUNT,
            ),
        ];
    }
    if let Some(flow) = model.flow_of(element_id) {
        return vec![
            row(PRODUCT, "Product", &flow.product, PropertyEditor::Line, IDENTITY),
            row(
                DESCRIPTION,
                "Description",
                &flow.description,
                PropertyEditor::Text,
                IDENTITY,
            ),
            row(
                VOLUME,
                "Volume",
                &format_optional(flow.volume),
                PropertyEditor::Line,
                AMOUNT,
            ),
            row(UNIT, "Unit", &flow.unit, PropertyEditor::Line, AMOUNT),
            row(
                STEP,
                "Step",
                &format_optional(Some(flow.step.unwrap_or(geometry::DEFAULT_STEP))),
                PropertyEditor::Line,
                AMOUNT,
            ),
        ];
    }
    if let Some(frame) = model.group_of(element_id) {
        return vec![
            row(NAME, "Name", &frame.name, PropertyEditor::Line, IDENTITY),
            row(
                DESCRIPTION,
                "Description",
                &frame.description,
                PropertyEditor::Text,
                IDENTITY,
            ),
        ];
    }
    Vec::new()
}

/// `(none)`, then the name of every group with an id, each once.
fn group_choices(model: &SupplyChainModel) -> Vec<String> {
    let mut choices = vec![NO_GROUP.to_owned()];
    for group in model.groups.iter().filter(|g| !g.id.is_empty()) {
        let name = name_of(group).to_owned();
        if !choices[1..].contains(&name) {
            choices.push(name);
        }
    }
    choices
}

/// The command a row's new value becomes, or why there is none.
pub fn change(
    model: &SupplyChainModel,
    body_path: &Path,
    element_id: &str,
    property_id: &str,
    value: &str,
) -> Result<SupplyChainCommand, String> {
    let key = match property_id {
        NAME => keys::NAME,
        STAGE => keys::TYPE,
        GROUP => keys::GROUP,
        QUANTITY => keys::QUANTITY,
        PRODUCT => keys::PRODUCT,
        VOLUME => keys::VOLUME,
        UNIT => keys::UNIT,
        STEP => keys::STEP,
        DESCRIPTION => keys::DESCRIPTION,
        _ => return Err(format!("'{property_id}' cannot be edited on this selection.")),
    };
    let value = if key == keys::GROUP {
        // The grid offers names; the document keeps ids.
        if value == NO_GROUP || value.is_empty() {
            String::new()
        } else {
            model
                .groups
                .iter()
                .find(|g| name_of(g) == value)
                .map_or_else(|| value.to_owned(), |g| g.id.clone())
        }
    } else {
        value.to_owned()
    };
    Ok(SupplyChainCommand::new(
        body_path,
        SupplyChainEdit::SetProperty {
            entry_id: element_id.to_owned(),
            key: key.to_owned(),
            value,
        },
    ))
}

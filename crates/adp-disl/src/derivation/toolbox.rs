//! The palette of a specification (DISL sections 7.1 and 7.2; the standalone host's `ToolboxDerivation`): every tool
//! of every group, nested groups depth first, in declaration order, a tool referenced by id taken from the toolbox
//! library. A palette reads no diagram, so a tool's label is evaluated with `self` bound to an empty one; a
//! `visible` or `enabled` condition is the host's to evaluate where it has a diagram.

use adp_cel::Value;

use super::env::DislEnv;
use super::evaluation::{self, Vars};
use super::wire_ids::WireIdMap;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::diagram::DislDiagram;

/// One palette tool as a host shows it (section 7.2), its ids mapped by a [`WireIdMap`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedTool {
    /// The tool's id in the specification.
    pub name: String,
    /// The id of the tool group it is in.
    pub group: String,
    /// The palette id.
    pub id: String,
    /// The type it creates.
    pub creates: Option<String>,
    /// Its text: its own label, else the created type's.
    pub label: String,
    /// Its icon: its own, else the created type's; empty when neither has one.
    pub icon: String,
    /// Its `doc`, the tooltip.
    pub description: String,
    /// The action a drop runs, or none when the map names none.
    pub drop_action_id: Option<String>,
}

/// The palette of `specification`.
pub fn derive(specification: &DislSpecification, ids: &WireIdMap) -> Vec<DerivedTool> {
    let mut tools = Vec::new();
    let Some(toolbox) = json::get(specification.root(), "toolbox").filter(|t| t.is_object()) else {
        return tools;
    };
    let empty = DislDiagram::new(specification);
    let variables = evaluation::vars([
        ("self", empty.to_value()),
        ("diagram", empty.to_value()),
        ("env", Value::map(DislEnv::default().to_cel())),
    ]);
    if let Some(groups) = json::items(toolbox, "groups") {
        walk(specification, ids, toolbox, groups, "/toolbox/groups", &variables, &mut tools);
    }
    tools
}

fn walk(
    specification: &DislSpecification,
    ids: &WireIdMap,
    toolbox: &Json,
    groups: &[Json],
    pointer: &str,
    variables: &Vars,
    tools: &mut Vec<DerivedTool>,
) {
    for (index, group) in groups.iter().enumerate() {
        let at = json::pointer_at(pointer, index);
        let id = json::string_of(group, "id").unwrap_or("");
        if let Some(entries) = json::items(group, "tools") {
            for (position, entry) in entries.iter().enumerate() {
                let entry_at = json::pointer_at(&json::pointer(&at, "tools"), position);
                match entry {
                    Json::String(name) => {
                        if let Some(shared) = json::get(json::member(toolbox, "tools"), name) {
                            tools.push(tool(
                                specification,
                                ids,
                                shared,
                                &json::pointer("/toolbox/tools", name),
                                name,
                                id,
                                variables,
                            ));
                        }
                    }
                    Json::Object(_) => tools.push(tool(
                        specification,
                        ids,
                        entry,
                        &entry_at,
                        json::string_of(entry, "id").unwrap_or(""),
                        id,
                        variables,
                    )),
                    _ => {}
                }
            }
        }
        if let Some(nested) = json::items(group, "groups") {
            walk(specification, ids, toolbox, nested, &json::pointer(&at, "groups"), variables, tools);
        }
    }
}

fn tool(
    specification: &DislSpecification,
    ids: &WireIdMap,
    tool: &Json,
    pointer: &str,
    name: &str,
    group: &str,
    variables: &Vars,
) -> DerivedTool {
    let creates = json::string_of(tool, "creates");
    let created = creates.and_then(|c| specification.metamodel().type_of(c));
    let type_label = match created {
        None => name.to_owned(),
        Some(t) => json::string_of(&t.json, "label").unwrap_or(&t.name).to_owned(),
    };
    let label = match json::get(tool, "label") {
        Some(message) => evaluation::message(
            specification,
            message,
            &json::pointer(pointer, "label"),
            contexts::ELEMENT,
            variables,
            &type_label,
        ),
        None => type_label,
    };
    let icon = json::string_of(tool, "icon")
        .or_else(|| created.and_then(|t| json::string_of(&t.json, "icon")))
        .unwrap_or("")
        .to_owned();
    DerivedTool {
        name: name.to_owned(),
        group: group.to_owned(),
        id: ids.tool_id(name),
        creates: creates.map(str::to_owned),
        label,
        icon,
        description: doc(tool),
        drop_action_id: ids.drop_action_id(name),
    }
}

/// A `doc` (section 2.4) as one text: a string, or an object's summary.
fn doc(owner: &Json) -> String {
    match json::get(owner, "doc") {
        Some(Json::String(s)) => s.clone(),
        Some(doc) => json::string_of(doc, "summary").unwrap_or("").to_owned(),
        None => String::new(),
    }
}

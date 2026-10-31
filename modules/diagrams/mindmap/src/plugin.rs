//! The persistence plugin `net.etalii.adp.freeplane.mm` of the binding `mindmap.fbl#mindmap` (etalii.adp spec 030,
//! FR-031 and FR-032; FBL section 11), ported from the standalone host's `MindmapFreeplanePlugin.cs` at `8b6ccfc`, and,
//! for `plan`, from the mind map's commands (`Commands/*.cs`) and `MindmapDocumentStore.Save`.
//!
//! `read` reads a body with [`MindmapDocument`], so the DISL model sees exactly the nodes, texts, notes and links the
//! commands see: a `<node>` is a `Node` with `text`, `notes`, `link` (only when the file has a `LINK`, empty or not,
//! so `has(self.link)` means the file has one), `folded`, `position` (only when the file has one) and `storedId`, the
//! `ID` the file writes, under its parent in the slot `children`. An element's id is its `ID` for the first node the
//! file gives it; a later holder of the same `ID`, and a node with none, is given an ephemeral id that no `ID` can equal
//! (it holds a NUL, which XML forbids), so every element has an id of its own while findings name each node by what
//! the file writes.
//!
//! `plan` is where the desktop host departs from the standalone host's plugin, which refused every change because the
//! module's commands edited the XML in place and kept their own undo. Here the host's history applies, undoes and
//! redoes splices for every tool type, so the plugin plans one: it reads the body as the standalone host's store held
//! it (every missing `ID` assigned in memory), makes the change as the command did, writes the map as the store's save
//! did, and returns the one splice that turns the body into that text. The bytes are the standalone host's, and an
//! undo puts back exactly the bytes the edit replaced.

use std::collections::HashMap;

use adp_cel::Value;
use adp_fbl::ModelChange;
use adp_fbl::model::{FblElement, Finding, FindingSeverity, codes};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginSplice,
    PluginTemplateRequest,
};
use adp_fbl::splice::{Operation, Splice};
use adp_fbl::text::Span;

use crate::document::MindmapDocument;
use crate::xml::NodeId;

/// The plugin's id, as the binding's `reader.plugin` names it.
pub const PLUGIN_ID: &str = "net.etalii.adp.freeplane.mm";

/// The binding's element type, which `persistence.typeMap` maps to the metamodel's.
pub const NODE_TYPE: &str = "Node";

/// The attribute holding the `ID` the file writes, kept beside the metamodel's.
pub const STORED_ID_ATTRIBUTE: &str = "storedId";

/// What the refusals of a change name when the node they address is gone.
pub const NODE_GONE: &str = "The node no longer exists.";

/// The `.mm` reader.
#[derive(Debug, Clone, Copy, Default)]
pub struct MindmapFreeplanePlugin;

impl MindmapFreeplanePlugin {
    /// The elements of `document` as `read` gives them, with the model id of each node.
    pub fn elements(document: &MindmapDocument) -> Vec<FblElement> {
        let ids = model_ids(document);
        document
            .nodes()
            .into_iter()
            .map(|node| {
                let written = document.id(node);
                let id = ids[&node].clone();
                let mut attributes = adp_cel::Map::default();
                attributes.insert("text", Value::String(document.text(node)));
                attributes.insert("notes", Value::String(document.notes(node)));
                attributes.insert("folded", Value::Bool(document.folded(node)));
                attributes.insert(STORED_ID_ATTRIBUTE, Value::String(written.clone()));
                if let Some(link) = document.link(node) {
                    attributes.insert("link", Value::String(link.to_owned()));
                }
                if let Some(position) = document.position(node) {
                    attributes.insert("position", Value::String(position.to_owned()));
                }
                let parent = document.parent(node).map(|p| ids[&p].clone());
                FblElement {
                    id_is_stored: !written.is_empty() && id == written,
                    id,
                    type_name: NODE_TYPE.into(),
                    rule: "node".into(),
                    is_relation: false,
                    attributes,
                    parent_slot: parent.as_ref().map(|_| "children".to_owned()),
                    parent_id: parent,
                    source: None,
                    target: None,
                    referenced: Vec::new(),
                    own_span: Span::new(0, 0),
                    line: document.xml().line(node),
                    file: None,
                }
            })
            .collect()
    }
}

/// The model id of every node: its `ID` for the first holder, `ID` and NUL and the count for a later one, and NUL and
/// its place in document order (from 1) for a node with none.
pub fn model_ids(document: &MindmapDocument) -> HashMap<NodeId, String> {
    let mut ids = HashMap::new();
    let mut holders: HashMap<String, usize> = HashMap::new();
    for (index, node) in document.nodes().into_iter().enumerate() {
        let written = document.id(node);
        let id = if written.is_empty() {
            format!("\0{}", index + 1)
        } else {
            let seen = holders.entry(written.clone()).or_default();
            *seen += 1;
            if *seen == 1 {
                written
            } else {
                format!("{written}\0{seen}")
            }
        };
        ids.insert(node, id);
    }
    ids
}

/// The node a model id names in `document`.
pub fn node_of(document: &MindmapDocument, model_id: &str) -> Option<NodeId> {
    model_ids(document)
        .into_iter()
        .find(|(_, id)| id == model_id)
        .map(|(node, _)| node)
}

impl PersistencePlugin for MindmapFreeplanePlugin {
    fn id(&self) -> &str {
        PLUGIN_ID
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let Some(body) = request.files.iter().find(|f| f.relative_path.is_empty()) else {
            return PluginReadResult::new(Vec::new(), Vec::new(), true);
        };
        // As the validator always judged it: no id is assigned, so the model is the file exactly as written.
        let outcome = MindmapDocument::outcome(&String::from_utf8_lossy(&body.bytes), false);
        match outcome.unreadable {
            Some(reason) => PluginReadResult::new(
                Vec::new(),
                vec![Finding {
                    code: codes::UNPARSEABLE.into(),
                    severity: FindingSeverity::Error,
                    message: reason,
                    location: None,
                    detail: None,
                }],
                true,
            ),
            None => PluginReadResult::new(Self::elements(&outcome.document), Vec::new(), false),
        }
    }

    fn plan(&self, request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        let Some(body) = request.files.iter().find(|f| f.relative_path.is_empty()) else {
            return PluginPlanResult::Refused("The request holds no body.".into());
        };
        let text = String::from_utf8_lossy(&body.bytes);
        // The nodes are addressed as the last reading named them, before any id is assigned; then every missing id
        // is assigned, as the standalone host's store held the map, so the save writes them.
        let mut document = match MindmapDocument::parse(&text, false) {
            Ok(document) => document,
            Err(error) => return PluginPlanResult::Refused(error.message),
        };
        let ids = model_ids(&document);
        document.assign_missing_ids();
        let node = |id: &str| ids.iter().find(|(_, m)| m.as_str() == id).map(|(n, _)| *n);
        let operation = match apply(&mut document, request.change, &node) {
            Ok(operation) => operation,
            Err(refusal) => return PluginPlanResult::Refused(refusal),
        };
        let after = document.to_text();
        PluginPlanResult::Planned(
            difference(&body.bytes, after.as_bytes(), operation)
                .into_iter()
                .collect(),
        )
    }

    fn template(&self, request: &PluginTemplateRequest) -> Vec<u8> {
        MindmapDocument::empty_document(&request.name).into_bytes()
    }
}

/// Makes `change` in `document` as the mind map's commands made it; the splice's operation, or the refusal.
fn apply(
    document: &mut MindmapDocument,
    change: &ModelChange,
    node: &dyn Fn(&str) -> Option<NodeId>,
) -> Result<Operation, String> {
    match change {
        ModelChange::Add {
            id,
            attributes,
            parent_id,
            index,
            ..
        } => {
            let parent = parent_id
                .as_deref()
                .and_then(node)
                .ok_or_else(|| NODE_GONE.to_owned())?;
            let id = id.clone().ok_or("A new node needs an id.")?;
            if document.find(&id).is_some() {
                return Err("A node with that id already exists.".into());
            }
            let text = match attributes.get("text") {
                Some(Value::String(text)) => text.clone(),
                Some(Value::Null) | None => String::new(),
                Some(other) => other.to_string(),
            };
            // An index past the first is a sibling's: right after the child before it (`AddSiblingNodeCommand`);
            // otherwise the last child (`AddChildNodeCommand`), or the first.
            let children = document.children(parent);
            let added = match index.and_then(|i| usize::try_from(i).ok()) {
                Some(i) if i > 0 && i <= children.len() => document.add_sibling(children[i - 1], &text)?,
                Some(0) if !children.is_empty() => {
                    let added = document.add_child(parent, &text);
                    document.move_node(added, parent, 0)?;
                    added
                }
                _ => document.add_child(parent, &text),
            };
            document.set_id(added, &id);
            for (name, value) in attributes.iter().filter(|(n, _)| n.as_str() != "text") {
                set(document, added, name, value)?;
            }
            Ok(Operation::InsertEntry)
        }
        ModelChange::Set { id, attributes } => {
            let target = node(id).ok_or_else(|| NODE_GONE.to_owned())?;
            for (name, value) in attributes.iter() {
                set(document, target, name, value)?;
            }
            Ok(Operation::ReplaceValue)
        }
        ModelChange::Remove { id } => {
            let target = node(id).ok_or_else(|| NODE_GONE.to_owned())?;
            if document.is_root(target) {
                return Err("The root node cannot be removed.".into());
            }
            document.remove(target)?;
            Ok(Operation::RemoveEntry)
        }
        ModelChange::Move {
            id,
            new_parent_id,
            index,
        } => {
            let target = node(id).ok_or_else(|| NODE_GONE.to_owned())?;
            if document.is_root(target) {
                return Err("The root node cannot be moved.".into());
            }
            let parent = new_parent_id
                .as_deref()
                .and_then(node)
                .ok_or("The destination node no longer exists.")?;
            if document.is_within(parent, target) {
                return Err("A node cannot be moved into its own branch.".into());
            }
            document.move_node(target, parent, *index)?;
            Ok(Operation::InsertEntry)
        }
        ModelChange::Save => Ok(Operation::ReplaceValue),
        _ => Err("A mind map cannot be changed that way.".into()),
    }
}

/// Sets one attribute of a node as its command did: the text as plain `TEXT`, the notes (empty removes them), the
/// link (empty or none unlinks, as an emptied Link row does).
fn set(document: &mut MindmapDocument, node: NodeId, name: &str, value: &Value) -> Result<(), String> {
    let text = match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        other => Some(other.to_string()),
    };
    match name {
        "text" => document.set_text(node, text.as_deref().unwrap_or_default()),
        "notes" => document.set_notes(node, text.as_deref().unwrap_or_default()),
        "link" => document.set_link(node, text.as_deref().filter(|t| !t.is_empty())),
        "folded" => return Err("Collapsing is a view setting rather than part of the document.".into()),
        _ => return Err(format!("'{name}' is not ADP's to write.")),
    }
    Ok(())
}

/// The one splice that turns `before` into `after`: what lies between their common start and their common end.
fn difference(before: &[u8], after: &[u8], operation: Operation) -> Option<PluginSplice> {
    if before == after {
        return None;
    }
    let boundary = |bytes: &[u8], at: usize| at == bytes.len() || (bytes[at] & 0xC0) != 0x80;
    let mut prefix = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    while prefix > 0 && !(boundary(before, prefix) && boundary(after, prefix)) {
        prefix -= 1;
    }
    let most = before.len().min(after.len()) - prefix;
    let mut suffix = before
        .iter()
        .rev()
        .zip(after.iter().rev())
        .take(most)
        .take_while(|(a, b)| a == b)
        .count();
    while suffix > 0 && !(boundary(before, before.len() - suffix) && boundary(after, after.len() - suffix)) {
        suffix -= 1;
    }
    let text = String::from_utf8_lossy(&after[prefix..after.len() - suffix]).into_owned();
    Some(PluginSplice {
        file: String::new(),
        splice: Splice {
            operation,
            start: prefix,
            end: before.len() - suffix,
            text,
        },
    })
}

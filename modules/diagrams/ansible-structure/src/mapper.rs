//! Turns the definition's model of a folder and its layout into what the shell draws (etalii.adp spec 030, task T079).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/ansible-structure/backend/
//! EtAlii.Adp.Diagram.AnsibleStructure/AnsibleElementMapper.cs`, whose protobuf payload (`AnsibleElementPayload`) is
//! carried here as the element's attributes. It reads the DISL model and nothing else, nodes and relations in the
//! model's order (finding B6). The layout's boxes are overlaid with the positions the registration stores: a stored
//! node sits where the person put it and keeps its size; a stored id the folder no longer produces draws nothing. A
//! node is placed by its top-left corner, as the standalone host's stream placed it. The viewport rule is the mind
//! map's: the nodes it intersects, plus one hop of graph partners, and every relation whose ends are both delivered;
//! a relation with no target still draws, from its source to nothing.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_cel::Value;
use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};
use adp_disl::{DislDiagram, DislElement};

use crate::definition::{AnsibleDefinition, text};
use crate::layout::{self, AnsibleMetrics};

/// The element kinds this tool type draws (`AnsibleElementMapper`'s wire types).
pub const PLAYBOOK_TYPE: &str = "ansible/structure+playbook";
pub const PLAY_TYPE: &str = "ansible/structure+play";
pub const ROLE_TYPE: &str = "ansible/structure+role";
pub const TASK_FILE_TYPE: &str = "ansible/structure+taskfile";
pub const INVENTORY_TYPE: &str = "ansible/structure+inventory";
pub const VARIABLE_FOLDER_TYPE: &str = "ansible/structure+vars";
pub const EDGE_TYPE: &str = "ansible/structure+edge";

/// The computed boxes with the stored positions overlaid: only the origin moves.
pub fn arranged(diagram: &DislDiagram, stored: &BTreeMap<String, (f64, f64)>) -> HashMap<String, Rect> {
    let mut boxes = layout::compute(diagram, &AnsibleMetrics::default());
    for (id, rect) in &mut boxes {
        if let Some(&(x, y)) = stored.get(id) {
            rect.x = x;
            rect.y = y;
        }
    }
    boxes
}

/// Every element a view with `viewport` shows: the nodes it intersects, their one-hop partners, and every relation
/// whose two ends are both delivered.
pub fn visible(
    diagram: &DislDiagram,
    viewport: DiagramViewport,
    stored: &BTreeMap<String, (f64, f64)>,
) -> Vec<DiagramElement> {
    let definition = AnsibleDefinition::get();
    let boxes = arranged(diagram, stored);
    let nodes = diagram.nodes();
    let relations = diagram.relations();

    let mut delivered: HashSet<String> = nodes
        .iter()
        .map(DislElement::id)
        .filter(|id| boxes.get(id).is_some_and(|b| viewport.intersects(*b)))
        .collect();
    for relation in &relations {
        let source = source_id_of(relation);
        let target = target_id_of(relation);
        if delivered.contains(&source) && !target.is_empty() {
            delivered.insert(target);
        } else if !target.is_empty() && delivered.contains(&target) {
            delivered.insert(source);
        }
    }

    let mut elements = Vec::new();
    for node in nodes.iter().filter(|n| delivered.contains(&n.id())) {
        let rect = boxes.get(&node.id()).copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        elements.push(node_element(node, rect, stored.contains_key(&node.id())));
    }
    for relation in &relations {
        let source = source_id_of(relation);
        let target = target_id_of(relation);
        if delivered.contains(&source) && (target.is_empty() || delivered.contains(&target)) {
            let anchor = boxes.get(&source).copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
            elements.push(edge_element(&definition, relation, source, target, anchor));
        }
    }
    elements
}

fn source_id_of(relation: &DislElement) -> String {
    relation
        .source()
        .map(|s| s.id())
        .or_else(|| relation.source_id())
        .unwrap_or_default()
}

fn target_id_of(relation: &DislElement) -> String {
    relation.target().map(|t| t.id()).unwrap_or_default()
}

/// The wire type of a node of DISL type `type_name`.
pub fn type_of(type_name: &str) -> &'static str {
    match type_name {
        "Playbook" => PLAYBOOK_TYPE,
        "Play" => PLAY_TYPE,
        "Role" => ROLE_TYPE,
        "TaskFile" => TASK_FILE_TYPE,
        "Inventory" => INVENTORY_TYPE,
        "VariableFolder" => VARIABLE_FOLDER_TYPE,
        _ => EDGE_TYPE,
    }
}

fn number(value: &Value) -> i64 {
    match value {
        Value::Int(n) => *n,
        _ => 0,
    }
}

fn items(value: &Value) -> Vec<Value> {
    match value {
        Value::List(items) => items.as_ref().clone(),
        _ => Vec::new(),
    }
}

fn field(item: &Value, name: &str) -> Value {
    match item {
        Value::Map(map) => map.get(name).cloned().unwrap_or_default(),
        _ => Value::Null,
    }
}

fn node_element(node: &DislElement, rect: Rect, placed: bool) -> DiagramElement {
    let type_name = node.type_name();
    let mut attributes = vec![
        ("name".to_owned(), text(&node.value_of("name"))),
        ("path".to_owned(), text(&node.value_of("path"))),
        ("kind".to_owned(), type_name.clone()),
        ("width".to_owned(), rect.width.to_string()),
        ("height".to_owned(), rect.height.to_string()),
        (
            "playIndex".to_owned(),
            if type_name == "Play" {
                number(&node.value_of("playIndex")).to_string()
            } else {
                "-1".to_owned()
            },
        ),
    ];
    match type_name.as_str() {
        "Playbook" | "Play" => {
            attributes.push(("hosts".to_owned(), text(&node.value_of("hosts"))));
            let annotations: Vec<String> = items(&node.value_of("annotations")).iter().map(text).collect();
            attributes.push(("annotations".to_owned(), annotations.join(", ")));
        }
        "Role" if node.attributes().get("contents").is_some() => {
            let contents = node.value_of("contents");
            for name in ["taskFiles", "handlers", "templates", "files", "defaults", "vars"] {
                attributes.push((format!("contents.{name}"), number(&field(&contents, name)).to_string()));
            }
            attributes.push((
                "hollow".to_owned(),
                (node.value_of("hollow") == Value::Bool(true)).to_string(),
            ));
        }
        "Inventory" => {
            let groups: Vec<String> = items(&node.value_of("groups"))
                .iter()
                .map(|g| format!("{} ({})", text(&field(g, "name")), number(&field(g, "hostCount"))))
                .collect();
            attributes.push(("groups".to_owned(), groups.join(", ")));
        }
        "VariableFolder" => {
            attributes.push(("fileCount".to_owned(), number(&node.value_of("fileCount")).to_string()));
        }
        _ => {}
    }
    DiagramElement {
        id: node.id(),
        x: rect.x,
        y: rect.y,
        placed,
        element_type: type_of(&type_name).to_owned(),
        kind: ElementKind::Node,
        parent: None,
        source: None,
        target: None,
        attributes,
    }
}

fn edge_element(
    definition: &AnsibleDefinition,
    relation: &DislElement,
    source: String,
    target: String,
    anchor: Rect,
) -> DiagramElement {
    let resolution = text(&relation.value_of("resolution"));
    let attributes = vec![
        ("relation".to_owned(), relation.type_name()),
        ("directive".to_owned(), definition.label(relation, "directive")),
        (
            "targetAsWritten".to_owned(),
            text(&relation.value_of("targetAsWritten")),
        ),
        ("condition".to_owned(), text(&relation.value_of("condition"))),
        ("declaredIn".to_owned(), text(&relation.value_of("declaredIn"))),
        (
            "declaredAtLine".to_owned(),
            number(&relation.value_of("declaredAtLine")).to_string(),
        ),
        (
            "dynamic".to_owned(),
            (relation.value_of("dynamic") == Value::Bool(true)).to_string(),
        ),
        ("resolution".to_owned(), resolution),
    ];
    // An edge is positioned at its source; the canvas routes it between the two boxes.
    DiagramElement {
        id: relation.id(),
        x: anchor.x,
        y: anchor.y,
        placed: false,
        element_type: EDGE_TYPE.to_owned(),
        kind: ElementKind::Relation,
        parent: None,
        source: Some(source),
        target: (!target.is_empty()).then_some(target),
        attributes,
    }
}

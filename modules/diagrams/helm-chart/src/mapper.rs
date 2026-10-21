//! Turns the definition's model of a chart and its boxes into what the shell draws (etalii.adp spec 030, task T080).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/helm-chart/backend/EtAlii.Adp.Diagram.HelmChart/
//! HelmElementMapper.cs`, whose protobuf payload (`HelmElementPayload`) is carried here as the element's attributes,
//! one per field, named as the payload's JSON names them (`name`, `kind`, `chart.version`, `edge.openEnd`, ...), a list
//! as its items joined by a line feed. Every node and its payload are read from the model's attributes, derived ones
//! included; the relations are delivered in the tool's order ([`relations`]), because DISL appends the derived
//! relations after the stored ones. Two dependencies of one effective name share an id, and each is drawn with the
//! facts of the first of them. A node is placed by its top-left corner; an edge at its source's. The viewport rule is
//! the Ansible module's: the nodes it touches, one hop outwards, and every relation whose ends are both delivered; a
//! relation with no target still draws, from its source to nothing.

use std::collections::{BTreeMap, HashMap, HashSet};

use adp_cel::Value;
use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};
use adp_disl::{DislDiagram, DislElement};

use crate::definition::text;
use crate::layout;

/// The element kinds this tool type puts on the wire.
pub const CHART_TYPE: &str = "helm/chart+chart";
pub const VALUES_TYPE: &str = "helm/chart+values";
pub const SCHEMA_TYPE: &str = "helm/chart+schema";
pub const TEMPLATE_TYPE: &str = "helm/chart+template";
pub const PARTIAL_TYPE: &str = "helm/chart+partial";
pub const CRDS_TYPE: &str = "helm/chart+crds";
pub const DEPENDENCY_TYPE: &str = "helm/chart+dependency";
pub const SUBCHART_TYPE: &str = "helm/chart+subchart";
pub const ARCHIVE_TYPE: &str = "helm/chart+archive";
pub const LOCK_TYPE: &str = "helm/chart+lock";
pub const EDGE_TYPE: &str = "helm/chart+edge";

/// The separator of a list attribute's items.
pub const LIST_SEPARATOR: char = '\n';

/// The path a node stands for, relative to the chart folder; empty for a dependency, which is a declaration and no file
/// of its own.
pub fn path_of(node: &DislElement) -> String {
    if node.is_a("ChartFile") {
        text(&node.value_of("path"))
    } else {
        String::new()
    }
}

fn file_name(relative: &str) -> &str {
    relative.rsplit_once('/').map_or(relative, |(_, name)| name)
}

/// The name a node is drawn with: the chart's name, else `(chart)`; a template's file name; a dependency's effective
/// name; an entry of `charts/` by its own name; every other node by its path.
pub fn name_of(node: &DislElement) -> String {
    match node.type_name().as_str() {
        "Chart" => match node.attributes().get("name") {
            Some(Value::String(name)) => name.clone(),
            _ => "(chart)".to_owned(),
        },
        "Template" | "Partial" => file_name(&path_of(node)).to_owned(),
        "Dependency" => {
            let alias = text(&node.value_of("alias"));
            if alias.is_empty() {
                text(&node.value_of("name"))
            } else {
                alias
            }
        }
        "Subchart" | "Archive" => text(&node.value_of("entryName")),
        _ => path_of(node),
    }
}

/// The id of the node a relation runs from.
pub fn source_of(relation: &DislElement) -> String {
    relation
        .source()
        .map(|s| s.id())
        .or_else(|| relation.source_id())
        .unwrap_or_default()
}

/// The id of the node a relation lands on; empty for an open end.
pub fn target_of(relation: &DislElement) -> String {
    relation.target().map(|t| t.id()).unwrap_or_default()
}

/// The relations of `diagram` in the order they are delivered, which is the tool's: every `Overrides`, then every
/// `Includes`, each in model order, then per dependency in model order its `Declares`, its `Resolves` and its
/// `Configures`. A relation of a dependency is placed by the dependency it is about; the sort is stable.
pub fn relations(diagram: &DislDiagram) -> Vec<DislElement> {
    let nodes = diagram.nodes();
    let place_of = |node: Option<DislElement>| -> usize {
        node.and_then(|n| nodes.iter().position(|m| *m == n))
            .unwrap_or(usize::MAX)
    };
    let mut keyed: Vec<((u8, usize, u8), DislElement)> = diagram
        .relations()
        .into_iter()
        .map(|relation| {
            let key = match relation.type_name().as_str() {
                "Overrides" => (0, 0, 0),
                "Includes" => (1, 0, 0),
                "Declares" => (2, place_of(relation.target()), 0),
                "Resolves" => (2, place_of(relation.source()), 1),
                "Configures" => (2, place_of(relation.target()), 2),
                _ => (3, 0, 0),
            };
            (key, relation)
        })
        .collect();
    keyed.sort_by_key(|(key, _)| *key);
    keyed.into_iter().map(|(_, relation)| relation).collect()
}

/// Every element a view with `viewport` shows over the computed layout with the `stored` positions overlaid: the
/// nodes whose boxes it touches, one hop of graph partners, and every relation whose ends are both delivered.
pub fn visible(
    diagram: &DislDiagram,
    viewport: DiagramViewport,
    stored: &BTreeMap<String, (f64, f64)>,
) -> Vec<DiagramElement> {
    let boxes = layout::arranged(diagram, stored);
    let nodes = diagram.nodes();
    let relations = relations(diagram);

    let mut delivered: HashSet<String> = nodes
        .iter()
        .map(DislElement::id)
        .filter(|id| boxes.get(id).is_some_and(|b| viewport.intersects(*b)))
        .collect();
    // One hop, exactly.
    for relation in &relations {
        let source = source_of(relation);
        let target = target_of(relation);
        if delivered.contains(&source) && !target.is_empty() {
            delivered.insert(target);
        } else if !target.is_empty() && delivered.contains(&target) {
            delivered.insert(source);
        }
    }

    let mut elements = Vec::new();
    for node in nodes.iter().filter(|n| delivered.contains(&n.id())) {
        elements.push(node_element(diagram, node, &boxes, stored));
    }
    for relation in &relations {
        let source = source_of(relation);
        let target = target_of(relation);
        if delivered.contains(&source) && (target.is_empty() || delivered.contains(&target)) {
            elements.push(edge_element(relation, &boxes));
        }
    }
    elements
}

/// Every element of the diagram, as a view that sees all of it is handed them.
pub fn elements(diagram: &DislDiagram, stored: &BTreeMap<String, (f64, f64)>) -> Vec<DiagramElement> {
    visible(diagram, DiagramViewport::UNBOUNDED, stored)
}

fn rect_of(boxes: &HashMap<String, Rect>, id: &str) -> Rect {
    boxes.get(id).copied().unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
}

fn items(element: &DislElement, attribute: &str) -> Vec<Value> {
    match element.value_of(attribute) {
        Value::List(items) => items.as_ref().clone(),
        _ => Vec::new(),
    }
}

fn joined(element: &DislElement, attribute: &str) -> String {
    items(element, attribute)
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(&LIST_SEPARATOR.to_string())
}

fn flag(element: &DislElement, attribute: &str) -> String {
    (element.value_of(attribute) == Value::Bool(true)).to_string()
}

fn count(element: &DislElement, attribute: &str) -> String {
    match element.value_of(attribute) {
        Value::Int(n) => n.to_string(),
        _ => "0".to_owned(),
    }
}

/// The wire type of a node of DISL type `type_name`.
pub fn type_of(type_name: &str) -> &'static str {
    match type_name {
        "Chart" => CHART_TYPE,
        "Values" => VALUES_TYPE,
        "Schema" => SCHEMA_TYPE,
        "Partial" => PARTIAL_TYPE,
        "Template" => TEMPLATE_TYPE,
        "Crds" => CRDS_TYPE,
        "Dependency" => DEPENDENCY_TYPE,
        "Subchart" => SUBCHART_TYPE,
        "Archive" => ARCHIVE_TYPE,
        "Lock" => LOCK_TYPE,
        _ => EDGE_TYPE,
    }
}

/// A node's type as the payload's element kind; a partial travels as a template, with its own wire type.
fn kind_of(type_name: &str) -> &'static str {
    match type_name {
        "Chart" => "CHART",
        "Values" => "VALUES",
        "Schema" => "SCHEMA",
        "Template" | "Partial" => "TEMPLATE",
        "Crds" => "CRDS",
        "Dependency" => "DEPENDENCY",
        "Subchart" => "SUBCHART",
        "Archive" => "ARCHIVE",
        "Lock" => "LOCK",
        _ => "HELM_ELEMENT_KIND_UNSPECIFIED",
    }
}

fn role_of(role: &str) -> &'static str {
    match role {
        "manifest" => "MANIFEST",
        "partial" => "PARTIAL",
        "notes" => "NOTES",
        "test" => "TEST",
        _ => "HELM_TEMPLATE_ROLE_UNSPECIFIED",
    }
}

fn condition_of(state: &str) -> &'static str {
    match state {
        "none" => "CONDITION_NONE",
        "on" => "CONDITION_ON",
        "off" => "CONDITION_OFF",
        "missing" => "CONDITION_MISSING",
        "unknown" => "CONDITION_UNKNOWN",
        _ => "HELM_CONDITION_STATE_UNSPECIFIED",
    }
}

fn edge_kind_of(type_name: &str) -> &'static str {
    match type_name {
        "Declares" => "DECLARES",
        "Resolves" => "RESOLVES",
        "Overrides" => "OVERRIDES",
        "Configures" => "CONFIGURES",
        "Includes" => "INCLUDES",
        _ => "HELM_EDGE_KIND_UNSPECIFIED",
    }
}

/// Marks the payload unreadable with the parser's words and line, when the stored `failure` is the definition's
/// `YamlFailure` of a file that did not parse.
fn mark(attributes: &mut Vec<(String, String)>, facts: &DislElement) {
    let Some(Value::Map(failure)) = facts.attributes().get("failure").cloned() else {
        return;
    };
    attributes.push(("unreadable".to_owned(), "true".to_owned()));
    attributes.push((
        "failureMessage".to_owned(),
        failure.get("message").map(text).unwrap_or_default(),
    ));
    attributes.push((
        "failureLine".to_owned(),
        match failure.get("line") {
            Some(Value::Int(line)) => line.to_string(),
            _ => "0".to_owned(),
        },
    ));
}

fn node_element(
    diagram: &DislDiagram,
    node: &DislElement,
    boxes: &HashMap<String, Rect>,
    stored: &BTreeMap<String, (f64, f64)>,
) -> DiagramElement {
    let id = node.id();
    let rect = rect_of(boxes, &id);
    // Two nodes may share an id (two dependencies of one effective name): both carry the facts of the one it names.
    let facts = diagram.element_by_id(&id).unwrap_or_else(|| node.clone());
    let type_name = node.type_name();
    let mut attributes = vec![
        ("name".to_owned(), name_of(&facts)),
        ("kind".to_owned(), kind_of(&type_name).to_owned()),
        ("chartRelativePath".to_owned(), path_of(node)),
        ("width".to_owned(), rect.width.to_string()),
        ("height".to_owned(), rect.height.to_string()),
    ];
    let mut push = |name: &str, value: String| attributes.push((name.to_owned(), value));
    match type_name.as_str() {
        "Chart" => {
            push("chart.version", text(&facts.value_of("version")));
            push("chart.appVersion", text(&facts.value_of("appVersion")));
            push("chart.apiVersion", text(&facts.value_of("apiVersion")));
            push("chart.chartType", text(&facts.value_of("chartType")));
            push("chart.description", text(&facts.value_of("description")));
            push("chart.deprecated", flag(&facts, "deprecated"));
            push("chart.legacy", flag(&facts, "legacy"));
            mark(&mut attributes, &facts);
        }
        "Values" => {
            push("values.isDefault", flag(&facts, "isDefault"));
            push("values.hasGlobal", flag(&facts, "hasGlobal"));
            push("values.keyCount", items(&facts, "keys").len().to_string());
            mark(&mut attributes, &facts);
        }
        "Template" | "Partial" => {
            push("template.role", role_of(&text(&facts.value_of("role"))).to_owned());
            push("template.kinds", joined(&facts, "kinds"));
            push("template.kindsUndetermined", flag(&facts, "undetermined"));
            push("template.defines", joined(&facts, "defines"));
        }
        "Dependency" => {
            push("dependency.chartName", text(&facts.value_of("name")));
            push("dependency.alias", text(&facts.value_of("alias")));
            push("dependency.constraint", text(&facts.value_of("constraint")));
            push("dependency.repository", text(&facts.value_of("repository")));
            push("dependency.condition", text(&facts.value_of("condition")));
            push(
                "dependency.conditionState",
                condition_of(&text(&facts.value_of("conditionState"))).to_owned(),
            );
            push("dependency.pinnedVersion", text(&facts.value_of("pinnedVersion")));
            push("dependency.resolved", flag(&facts, "resolved"));
        }
        "Subchart" | "Archive" => {
            let subchart = facts.type_name() == "Subchart";
            push("vendored.sealed", (!subchart).to_string());
            push("vendored.declared", flag(&facts, "declared"));
            push("vendored.chartType", text(&facts.value_of("chartType")));
            push("vendored.chartVersion", text(&facts.value_of("chartVersion")));
            push(
                "vendored.templateCount",
                if subchart {
                    count(&facts, "templates")
                } else {
                    "0".to_owned()
                },
            );
            push(
                "vendored.deeperCount",
                if subchart {
                    count(&facts, "deeper")
                } else {
                    "0".to_owned()
                },
            );
            mark(&mut attributes, &facts);
        }
        "Crds" => push("crdCount", count(&facts, "files")),
        "Lock" => {
            push("lockEntryCount", items(&facts, "entries").len().to_string());
            mark(&mut attributes, &facts);
        }
        _ => {}
    }
    DiagramElement {
        id: id.clone(),
        x: rect.x,
        y: rect.y,
        placed: stored.contains_key(&id),
        element_type: type_of(&type_name).to_owned(),
        kind: ElementKind::Node,
        parent: None,
        source: None,
        target: None,
        attributes,
    }
}

fn edge_element(relation: &DislElement, boxes: &HashMap<String, Rect>) -> DiagramElement {
    let source = source_of(relation);
    let target = target_of(relation);
    let label = text(&relation.value_of("label"));
    let anchor = rect_of(boxes, &source);
    let attributes = vec![
        ("name".to_owned(), label.clone()),
        ("kind".to_owned(), "EDGE".to_owned()),
        ("edge.sourceId".to_owned(), source.clone()),
        ("edge.targetId".to_owned(), target.clone()),
        ("edge.kind".to_owned(), edge_kind_of(&relation.type_name()).to_owned()),
        ("edge.label".to_owned(), label),
        ("edge.openEnd".to_owned(), relation.target().is_none().to_string()),
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

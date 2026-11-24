//! The conformance fixture of FBL section 17, `specifications/fbl/fixtures/links-workspace/` of etalii.adp: its
//! folder is one workspace, each document is read through its tool type's own binding (the mind map's through the
//! attributes its plugin contract names, `mindmap.md` section 3.8), and every link it holds resolves, or is reported,
//! and is rewritten by load-time validation as `conformance.json` says.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::Value;
use adp_fbl::{FblModel, FblOptions, IdRequest, OpenBody};
use adp_links::{HeldLink, IndexedDocument, IndexedElement, Resolution, Workspace};
use serde_json::Value as Json;

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn folder() -> PathBuf {
    vendor().join("specifications/fbl/fixtures/links-workspace")
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(s)) => s.clone(),
        None | Some(Value::Null) => String::new(),
        Some(other) => other.to_string(),
    }
}

/// The ids the knowledge designer's rules that store none are given (`knowledge.des`, `persistence.ids`): only a
/// cell's and its items' are read here.
fn knowledge_id(request: &IdRequest<'_>) -> Option<String> {
    let parent = request.parent_id.unwrap_or_default();
    Some(match request.rule {
        "table" => "table".into(),
        "cell" => format!("{parent}/{}", text(request.attributes.get("property"))),
        "cellOption" => format!("{parent}/{}", text(request.attributes.get("option"))),
        "cellRow" => format!("{parent}/{}", request.position_in_slot),
        _ => format!(
            "{parent}/{}/{}",
            request.parent_slot.unwrap_or_default(),
            request.position_in_slot
        ),
    })
}

/// The id an element was written with: a duplicate is addressed by its place, and is still what a link to the id it
/// was written with names.
fn written_id(model: &FblModel, id: &str) -> String {
    model
        .written_ids
        .iter()
        .find(|(addressed, _)| addressed == id)
        .map_or(id, |(_, written)| written)
        .to_owned()
}

fn read(path: &Path, binding_file: &Path, binding: &str, derive: bool) -> FblModel {
    let (problems, bindings) = adp_fbl::documents::load_file(binding_file);
    assert!(
        !problems
            .iter()
            .any(|p| p.severity == adp_fbl::documents::ProblemSeverity::Error),
        "{problems:?}"
    );
    let binding = bindings
        .expect("the binding file reads")
        .binding(binding)
        .expect("the binding")
        .clone();
    let options = FblOptions {
        file_name: path.file_name().unwrap().to_string_lossy().into_owned(),
        derive_id: derive.then(|| Rc::new(knowledge_id) as adp_fbl::model::DeriveId),
        ..FblOptions::default()
    };
    OpenBody::open(fs::read(path).unwrap(), Rc::new(binding), Rc::new(options)).model()
}

/// A timeline or a dependency graph: its elements named by `label`, and the `link` of a linked one.
fn diagram(path: &Path, binding_file: &str, binding: &str) -> IndexedDocument {
    let model = read(path, &vendor().join(binding_file), binding, false);
    assert!(!model.unreadable, "{}: {:?}", path.display(), model.findings);
    let mut document = IndexedDocument {
        path: path.to_path_buf(),
        document_id: model.document_id.clone(),
        name_property: Some("label".into()),
        ..IndexedDocument::default()
    };
    for element in model.elements.iter().filter(|e| e.parent_id.is_none()) {
        if element.type_name == "Dependency" || element.type_name == "Relation" {
            continue;
        }
        let label = text(element.attributes.get("label"));
        document.elements.push(IndexedElement::new(
            written_id(&model, &element.id),
            [("label".to_owned(), label)],
        ));
        if let Some(link) = element.attributes.get("link").filter(|v| !v.is_null()) {
            document.links.push(HeldLink::new(&element.id, text(Some(link))));
        }
    }
    document
}

/// A knowledge file: its rows named by the title property, and the links its relations and their cells hold.
fn knowledge(path: &Path) -> IndexedDocument {
    let model = read(
        path,
        &vendor().join("definitions/designers/knowledge.fbl"),
        "yaml",
        true,
    );
    assert!(!model.unreadable, "{}: {:?}", path.display(), model.findings);
    let properties: Vec<_> = model.elements.iter().filter(|e| e.type_name == "Property").collect();
    let name_of = |id: &str| {
        properties
            .iter()
            .find(|p| p.id == id)
            .map(|p| text(p.attributes.get("name")))
    };
    let title = properties
        .iter()
        .find(|p| matches!(p.attributes.get("title"), Some(Value::Bool(true))))
        .map(|p| text(p.attributes.get("name")));
    let mut document = IndexedDocument {
        path: path.to_path_buf(),
        document_id: model.document_id.clone(),
        name_property: title,
        ..IndexedDocument::default()
    };
    for row in model.elements.iter().filter(|e| e.type_name == "Row") {
        let cells: Vec<_> = model
            .elements
            .iter()
            .filter(|c| c.type_name == "Cell" && c.parent_id.as_deref() == Some(row.id.as_str()))
            .collect();
        let attributes = cells.iter().filter_map(|cell| {
            let value = cell.attributes.get("text").filter(|v| !v.is_null())?;
            Some((name_of(&text(cell.attributes.get("property")))?, text(Some(value))))
        });
        document
            .elements
            .push(IndexedElement::new(written_id(&model, &row.id), attributes));
        for cell in cells {
            let property = text(cell.attributes.get("property"));
            for item in model
                .elements
                .iter()
                .filter(|i| i.type_name == "CellItem" && i.parent_id.as_deref() == Some(cell.id.as_str()))
            {
                let value = text(item.attributes.get("row"));
                if value.starts_with('$') {
                    document
                        .links
                        .push(HeldLink::new(format!("{}/{property}", row.id), value));
                }
            }
        }
    }
    for property in &properties {
        let target = text(property.attributes.get("targetFile"));
        if target.starts_with('$') {
            document.links.push(HeldLink::new(&property.id, target));
        }
    }
    document
}

/// A mind map: the central topic's `adp-id`, every node by its `TEXT`, and each node's `adp-link`.
fn mindmap(path: &Path) -> IndexedDocument {
    let source = fs::read_to_string(path).unwrap();
    let tree = roxmltree::Document::parse(&source).expect("the map reads");
    let attribute = |node: roxmltree::Node<'_, '_>, name: &str| {
        node.children()
            .filter(|c| c.has_tag_name("attribute") && c.attribute("NAME") == Some(name))
            .find_map(|c| c.attribute("VALUE"))
            .map(str::to_owned)
    };
    let central = tree
        .root_element()
        .children()
        .find(|c| c.has_tag_name("node"))
        .expect("a central topic");
    let mut document = IndexedDocument {
        path: path.to_path_buf(),
        document_id: attribute(central, "adp-id"),
        name_property: Some("text".into()),
        ..IndexedDocument::default()
    };
    for node in tree.descendants().filter(|n| n.has_tag_name("node")) {
        let id = node.attribute("ID").unwrap_or_default();
        document.elements.push(IndexedElement::new(
            id,
            [("text".to_owned(), node.attribute("TEXT").unwrap_or_default().to_owned())],
        ));
        if let Some(link) = attribute(node, "adp-link") {
            document.links.push(HeldLink::new(id, link));
        }
    }
    document
}

fn files(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(files(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn the_workspace() -> Workspace {
    let root = folder();
    let documents = files(&root)
        .into_iter()
        .filter_map(|path| match path.extension().and_then(|e| e.to_str()) {
            Some("tml") => Some(diagram(&path, "specifications/fbl/timeline.fbl", "timeline")),
            Some("dgr") => Some(diagram(&path, "specifications/fbl/dependency-graph.fbl", "dgr")),
            Some("yaml") => Some(knowledge(&path)),
            Some("mm") => Some(mindmap(&path)),
            _ => None,
        });
    Workspace::new(root, documents)
}

fn conformance() -> Json {
    serde_json::from_str(&fs::read_to_string(folder().join("conformance.json")).unwrap()).unwrap()
}

#[test]
fn the_documents_without_an_id_are_those_the_fixture_lists() {
    let workspace = the_workspace();
    let without: BTreeSet<String> = workspace
        .documents()
        .iter()
        .filter(|d| d.document_id.is_none())
        .map(|d| workspace.display(&d.path))
        .collect();
    let listed: BTreeSet<String> = conformance()["withoutId"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(listed, without);
}

#[test]
fn every_link_the_documents_hold_is_listed_and_no_other() {
    let workspace = the_workspace();
    let held: BTreeSet<(String, String, String)> = workspace
        .documents()
        .iter()
        .flat_map(|d| {
            d.links
                .iter()
                .map(|l| (workspace.display(&d.path), l.holder.clone(), l.text.clone()))
        })
        .collect();
    let listed: BTreeSet<(String, String, String)> = conformance()["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            (
                case["in"].as_str().unwrap().to_owned(),
                case["at"].as_str().unwrap().to_owned(),
                case["link"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        Vec::<&(String, String, String)>::new(),
        held.difference(&listed).collect::<Vec<_>>(),
        "held and not listed"
    );
    assert_eq!(
        Vec::<&(String, String, String)>::new(),
        listed.difference(&held).collect::<Vec<_>>(),
        "listed and not held"
    );
}

#[test]
fn every_link_resolves_and_is_rewritten_as_the_fixture_says() {
    let workspace = the_workspace();
    let mut failures = Vec::new();
    for case in conformance()["links"].as_array().unwrap() {
        let link = case["link"].as_str().unwrap();
        let place = format!("{} at {} ({})", case["in"], case["at"], case["case"]);
        match (workspace.resolve(link), case.get("finding").and_then(Json::as_str)) {
            (Resolution::Finding(finding), Some(expected)) => {
                if finding.code != expected {
                    failures.push(format!("{place}: {}, where the fixture says {expected}", finding.code));
                }
                if let Some(candidates) = case.get("candidates").and_then(Json::as_array) {
                    let got: Vec<String> = finding.candidates.iter().map(|p| workspace.display(p)).collect();
                    let expected: Vec<String> = candidates.iter().map(|c| c.as_str().unwrap().to_owned()).collect();
                    if got != expected {
                        failures.push(format!(
                            "{place}: candidates {got:?}, where the fixture says {expected:?}"
                        ));
                    }
                }
            }
            (Resolution::Finding(finding), None) => {
                failures.push(format!(
                    "{place}: {} ({}), where it resolves",
                    finding.code, finding.message
                ));
            }
            (Resolution::Resolved(resolved), Some(expected)) => {
                failures.push(format!(
                    "{place}: resolves to {}, where the fixture says {expected}",
                    workspace.display(&resolved.path)
                ));
            }
            (Resolution::Resolved(resolved), None) => {
                let target = workspace.display(&resolved.path);
                let element = resolved.link.element_id().map(str::to_owned);
                if target != case["target"].as_str().unwrap()
                    || element.as_deref() != case.get("element").and_then(Json::as_str)
                {
                    failures.push(format!("{place}: resolves to {target}#{element:?}"));
                } else if resolved.refreshed.text() != case["after"].as_str().unwrap() {
                    failures.push(format!(
                        "{place}: after load it is {:?}, where the fixture says {}",
                        resolved.refreshed.text(),
                        case["after"]
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_rows_backlinks_come_from_every_kind_of_document() {
    let workspace = the_workspace();
    let provinces = folder().join("knowledge/provinces.yaml");
    let from: BTreeSet<String> = workspace
        .backlinks(&provinces, "nh")
        .iter()
        .map(|b| workspace.display(&b.path))
        .collect();
    // A timeline element (renamed target), a dependency graph node, and rows of another knowledge file.
    assert!(from.contains("plan/release.tml"), "{from:?}");
    assert!(from.contains("teams/services.dgr"), "{from:?}");
    assert!(from.iter().any(|p| p.starts_with("knowledge/")), "{from:?}");
}

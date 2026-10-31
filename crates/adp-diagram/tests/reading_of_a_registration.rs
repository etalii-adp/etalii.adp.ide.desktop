//! A body read through a declared binding as its registration says (etalii.adp spec 067, T045; FBL sections 8.2, 8.7 and
//! 9.3): the reading's origin selects its rules, the registration's `view:` the view it reads inside and its `resource:`
//! the resource, and a binding's legacy layout and legacy identities are read and written beside the body in one edit.
//! The bindings are FBL's own examples, the specifications the catalogue's; no module is added.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::Value;
use adp_diagram::disl_definition::DislDiagramDefinition;
use adp_diagram::document::{DiagramChange, DiagramDocuments, DocumentEdit, EditedFile, text_of};
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTool;
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::behavior::ids;
use adp_disl::{DislChange, DislPluginFunctions};
use adp_fbl::ModelChange;
use adp_fbl::documents;

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn tool(origin: &str, specification: &str, binding: &str, name: &str) -> Rc<DiagramTool> {
    let bytes = fs::read(vendor().join("definitions/diagrams").join(specification)).unwrap();
    let definition = DislDiagramDefinition::load(&bytes, None, &DislPluginFunctions::none()).unwrap();
    let (problems, document) = documents::load_file(&vendor().join("specifications/fbl").join(binding));
    let document = document.unwrap_or_else(|| panic!("{problems:?}"));
    Rc::new(DiagramTool {
        origin: origin.to_owned(),
        display_name: origin.to_owned(),
        definition,
        binding: Rc::new(document.binding(name).unwrap().clone()),
        extensions: Vec::new(),
        module: None,
    })
}

fn structurizr(origin: &str, specification: &str) -> Rc<DiagramTool> {
    tool(origin, specification, "structurizr.fbl", "workspace")
}

/// `name`, in the case's in-memory files, as what a body beside it would find.
fn files(entries: &[(&str, &str)]) -> impl Fn(&Path) -> Option<Vec<u8>> + use<> {
    let entries: BTreeMap<String, Vec<u8>> = entries
        .iter()
        .map(|(n, b)| ((*n).to_owned(), b.as_bytes().to_vec()))
        .collect();
    move |path: &Path| entries.get(&*path.file_name()?.to_string_lossy()).cloned()
}

const WORKSPACE: &str = "workspace \"W\" {
    model {
        a = softwareSystem \"A\" \"First.\"
        b = softwareSystem \"B\" \"Second.\" {
            c = container \"C\" \"Third.\" \"Rust\"
        }
    }
    views {
        systemLandscape \"Landscape\" {
            include a
            title \"The landscape\"
        }
        container b \"First\" {
            include c
            title \"The first\"
        }
        container b \"Second\" {
            include a
            title \"The second\"
        }
    }
}
";

fn titles(documents: &DiagramDocuments, body: &Path) -> Vec<String> {
    let document = documents.get(body).unwrap();
    document
        .fbl()
        .elements
        .iter()
        .filter(|e| e.rule == "viewTitle")
        .map(|e| e.attributes.get("title").map(text_of).unwrap_or_default())
        .collect()
}

fn open(
    tool: &Rc<DiagramTool>,
    documents: &mut DiagramDocuments,
    body: &str,
    registration: &str,
    beside: &[(&str, &str)],
) -> PathBuf {
    let path = PathBuf::from("/case/w.dsl");
    documents.open_bytes_beside(
        tool.clone(),
        &path,
        Some(Path::new("/case/w.adp")),
        body.as_bytes().to_vec(),
        Some(registration.as_bytes().to_vec()),
        &files(beside),
    );
    path
}

#[test]
fn the_registrations_view_selects_what_selected_view_rules_read() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let body = open(
        &tool,
        &mut documents,
        WORKSPACE,
        "c4/container\nbody: w.dsl\nview: second\n",
        &[],
    );
    assert_eq!(titles(&documents, &body), ["The second"]);
}

/// A reading shows a view of any kind, as the module does (spec 067, FR-015): `structurizr.fbl`'s view blocks have no
/// `origins`, so the first view of the body is the one a reading without a header shows.
#[test]
fn a_reading_without_a_view_header_reads_the_first_view_of_the_body() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let body = open(&tool, &mut documents, WORKSPACE, "c4/container\nbody: w.dsl\n", &[]);
    let document = documents.get(&body).unwrap();
    let views: Vec<&str> = document.fbl().views.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(views, ["Landscape", "First", "Second"]);
    assert_eq!(titles(&documents, &body), ["The landscape"]);
}

#[test]
fn the_specifications_ephemeral_ids_read_an_element_without_an_id_without_a_finding() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let body = open(
        &tool,
        &mut documents,
        "workspace {\n    model {\n        softwareSystem \"Unnamed\"\n    }\n}\n",
        "c4/container\nbody: w.dsl\n",
        &[],
    );
    let document = documents.get(&body).unwrap();
    assert!(
        !document.fbl().findings.iter().any(|f| f.code == "std.missingId"),
        "{:?}",
        document.fbl().findings
    );
}

const LAYOUT: &str = "{\n  \"First\": {\n    \"C\": { \"x\": 10, \"y\": 20 }\n  },\n  \"Second\": {\n    \"A\": { \"x\": 1, \"y\": 2 }\n  }\n}\n";

#[test]
fn a_legacy_layout_keeps_the_positions_of_the_selected_view_and_a_place_writes_it() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let registration = "c4/container\nbody: w.dsl\nview: First\n";
    let body = open(
        &tool,
        &mut documents,
        WORKSPACE,
        registration,
        &[("w.layout.json", LAYOUT)],
    );
    let document = documents.get(&body).unwrap();
    assert_eq!(
        document.positions().get("c"),
        Some(&(10.0, 20.0)),
        "its key is matched ignoring case"
    );
    assert_eq!(document.positions().get("a"), None, "of another view");

    let edit = document
        .plan(
            "Move",
            &[DiagramChange::Place {
                id: "c".into(),
                x: 12.5,
                y: -40.0,
            }],
        )
        .unwrap();
    assert!(edit.writes(EditedFile::LegacyLayout));
    assert!(
        !edit.writes(EditedFile::Registration),
        "a registration without `layout:` is not written"
    );
    let undo: DocumentEdit = documents.apply(&edit).unwrap();

    let document = documents.get(&body).unwrap();
    assert_eq!(document.positions().get("c"), Some(&(12.5, -40.0)));
    assert_eq!(document.registration_bytes(), Some(registration.as_bytes()));
    assert!(document.legacy_layout_bytes().is_some_and(|b| b != LAYOUT.as_bytes()));

    documents.apply(&undo).unwrap();
    let document = documents.get(&body).unwrap();
    assert_eq!(document.legacy_layout_bytes(), Some(LAYOUT.as_bytes()));
    assert_eq!(document.positions().get("c"), Some(&(10.0, 20.0)));
}

#[test]
fn a_registration_with_a_layout_block_keeps_its_positions_there() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let registration = "c4/container\nbody: w.dsl\nview: First\nlayout:\n  c: 7 8\n";
    let body = open(
        &tool,
        &mut documents,
        WORKSPACE,
        registration,
        &[("w.layout.json", LAYOUT)],
    );
    let document = documents.get(&body).unwrap();
    assert_eq!(document.positions().get("c"), Some(&(7.0, 8.0)));
    let edit = document
        .plan(
            "Move",
            &[DiagramChange::Place {
                id: "c".into(),
                x: 1.0,
                y: 2.0,
            }],
        )
        .unwrap();
    assert!(edit.writes(EditedFile::Registration));
    assert!(!edit.writes(EditedFile::LegacyLayout));
}

#[test]
fn a_change_of_an_element_whose_id_is_derived_reaches_the_entry_it_was_read_from() {
    // The specification's `Relationship` ids are `derived` from its ends and its line: an id the binding does not read.
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let source = "workspace {\n    model {\n        a = person \"A\"\n        b = person \"B\"\n        a -> b \"Uses\"\n    }\n}\n";
    let body = open(&tool, &mut documents, source, "c4/container\nbody: w.dsl\n", &[]);
    let document = documents.get(&body).unwrap();
    let change = DislChange::Set {
        element_id: "a->b@5".into(),
        type_name: "Relationship".into(),
        attributes: [("description", Value::from("Calls"))].into_iter().collect(),
    };
    let edit = document.plan("Relabel", &[DiagramChange::Disl(change)]).unwrap();
    documents.apply(&edit).unwrap();
    let document = documents.get(&body).unwrap();
    assert_eq!(
        String::from_utf8_lossy(document.body_bytes()),
        source.replace("\"Uses\"", "\"Calls\"")
    );
}

#[test]
fn a_relations_end_is_found_ignoring_case_where_the_specification_compares_ids_so() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let source = "workspace {\n    model {\n        a = person \"A\"\n        b = person \"B\"\n        A -> B \"Uses\"\n    }\n}\n";
    let body = open(&tool, &mut documents, source, "c4/container\nbody: w.dsl\n", &[]);
    let document = documents.get(&body).unwrap();
    let relation = document.model().diagram.relations().into_iter().next().unwrap();
    assert_eq!(relation.source().map(|s| s.id()).as_deref(), Some("a"));
    assert_eq!(relation.target().map(|t| t.id()).as_deref(), Some("b"));
}

#[test]
fn a_created_element_is_written_under_its_natural_id() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut documents = DiagramDocuments::in_memory();
    let source = "workspace {\n    model {\n        a = person \"A\"\n    }\n}\n";
    let body = open(&tool, &mut documents, source, "c4/container\nbody: w.dsl\n", &[]);
    let document = documents.get(&body).unwrap();
    let change = DislChange::Create {
        id: "Person:answered-1".into(),
        type_name: "Person".into(),
        attributes: [
            ("name", Value::from("Dispatcher")),
            ("description", Value::from("Sends.")),
        ]
        .into_iter()
        .collect(),
        parent_id: None,
        index: None,
        at: Value::Null,
    };
    let edit = document.plan("Add", &[DiagramChange::Disl(change)]).unwrap();
    documents.apply(&edit).unwrap();
    let written = String::from_utf8_lossy(documents.get(&body).unwrap().body_bytes()).into_owned();
    // The catalogue's specification keys a person by its name; the id it was created with is no id it is read back with.
    assert!(
        written.contains("Dispatcher = person \"Dispatcher\" \"Sends.\""),
        "{written}"
    );
}

#[test]
fn an_operation_a_menu_does_not_offer_runs_where_it_applies_and_says_why_where_it_does_not() {
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut workspace = DiagramWorkspace::in_memory("/case");
    let body = Path::new("/case/w.dsl");
    let session = workspace.open_bytes_beside(
        tool,
        body,
        Some(Path::new("/case/w.adp")),
        "workspace {\n    model {\n        p = person \"P\"\n        s = softwareSystem \"S\"\n    }\n}\n"
            .as_bytes()
            .to_vec(),
        Some(b"c4/container\nbody: w.dsl\n".to_vec()),
        &|_| None,
    );
    // A person's menu holds `setTechnology` behind its `visible`, and the operation says why it does not apply.
    let person = ActionTarget::Element("p".into());
    let outcome = session.execute(&mut workspace, &person, "setTechnology", &ids::fixed(&[]));
    assert!(
        matches!(&outcome, ActionOutcome::Refused(reason) if reason.starts_with("A person has no technology")),
        "{outcome:?}"
    );
    // `addContainer` is in no menu of a system; the toolbox drops it, and it runs on what its `for` admits.
    let system = ActionTarget::Element("s".into());
    let outcome = session.execute(&mut workspace, &system, "addContainer", &ids::fixed(&[]));
    assert!(matches!(outcome, ActionOutcome::Prompt(_)), "{outcome:?}");
    let outcome = session.execute(&mut workspace, &person, "addContainer", &ids::fixed(&[]));
    assert!(matches!(outcome, ActionOutcome::Refused(_)), "{outcome:?}");
}

#[test]
fn a_move_leaves_what_contains_an_element_as_it_is() {
    // `dragDoesNotReparent` compares `newParent` with the element's parent.
    let tool = structurizr("c4/container", "c4-container.dis");
    let mut workspace = DiagramWorkspace::in_memory("/case");
    let session = workspace.open_bytes_beside(
        tool,
        Path::new("/case/w.dsl"),
        Some(Path::new("/case/w.adp")),
        WORKSPACE.as_bytes().to_vec(),
        Some(b"c4/container\nbody: w.dsl\nview: First\n".to_vec()),
        &files(&[("w.layout.json", LAYOUT)]),
    );
    session.move_element_to(&mut workspace, "c", 5.0, 6.0).unwrap();
    let document = workspace.documents().get(Path::new("/case/w.dsl")).unwrap();
    assert_eq!(document.positions().get("c"), Some(&(5.0, 6.0)));
}

#[test]
fn a_rename_writes_the_body_and_its_legacy_identities_in_one_edit() {
    let case = vendor().join("specifications/fbl/fixtures/wardley-rename-identity");
    let read = |name: &str| fs::read_to_string(case.join(name)).unwrap();
    let (body, registration, identities) = (
        read("annotations-and-labels.owm"),
        read("annotations-and-labels.adp"),
        read("annotations-and-labels.identities.json"),
    );
    // The change is in the binding's terms, so any specification carries it; the Wardley map's own needs the module's
    // functions to load.
    let tool = tool("wardley/map", "c4-container.dis", "wardley-map.fbl", "map");
    let path = PathBuf::from("/case/annotations-and-labels.owm");
    let mut documents = DiagramDocuments::in_memory();
    documents.open_bytes_beside(
        tool,
        &path,
        Some(Path::new("/case/annotations-and-labels.adp")),
        body.clone().into_bytes(),
        Some(registration.into_bytes()),
        &files(&[("annotations-and-labels.identities.json", &identities)]),
    );
    let document = documents.get(&path).unwrap();
    let element = document
        .fbl()
        .elements
        .iter()
        .find(|e| e.type_name == "Component")
        .unwrap();
    let change = ModelChange::Set {
        id: element.id.clone(),
        attributes: [("name", Value::from("Renamed"))].into_iter().collect(),
    };
    let edit = document.plan("Rename", &[DiagramChange::Body(change)]).unwrap();
    assert!(edit.writes(EditedFile::Body));
    assert!(edit.writes(EditedFile::LegacyIdentities));
    let undo = documents.apply(&edit).unwrap();
    let document = documents.get(&path).unwrap();
    assert!(
        document
            .legacy_identities_bytes()
            .is_some_and(|b| b != identities.as_bytes())
    );
    documents.apply(&undo).unwrap();
    let document = documents.get(&path).unwrap();
    assert_eq!(document.body_bytes(), body.as_bytes());
    assert_eq!(document.legacy_identities_bytes(), Some(identities.as_bytes()));
}

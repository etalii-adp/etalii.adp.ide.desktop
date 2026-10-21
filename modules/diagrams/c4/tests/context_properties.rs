//! What the property grid shows for a C4 selection, and what changing one of its values does (etalii.adp spec 030,
//! task T061): a property edit is the same edit as the equivalent context action, on the same history.
//!
//! Ported from the standalone host's
//! `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4ContextPropertyProvider.Tests.cs` at `8b6ccfc`, through the
//! session of a `c4/container` registration.

use std::fs;
use std::path::{Path, PathBuf};

use adp_context::{PropertyEditor, PropertyRow};
use adp_core::Catalogue;
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_c4::properties::{
    DESCRIPTION, IDENTIFIER, KIND, NAME, RELATIONSHIP_DESCRIPTION, RELATIONSHIP_DESTINATION, RELATIONSHIP_SOURCE,
    RELATIONSHIP_TECHNOLOGY, TECHNOLOGY,
};
use adp_disl::DislPluginFunctions;

const MODEL: &str = r#"workspace "Bank" {
    model {
        u = person "Customer" "A customer."
        s = softwareSystem "Banking" "Does banking." {
            web = container "Web" "Serves pages." "React"
            batch = container "Batch" "Runs overnight."
        }
        u -> web "Uses" "HTTPS"
    }
    views {
        container s "containers" {
            include *
        }
    }
}"#;

struct Project {
    _folder: tempfile::TempDir,
    body: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    fn open() -> Project {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        let body = root.join("model.dsl");
        fs::write(&body, MODEL).unwrap();
        fs::write(
            root.join("model.adp"),
            "c4/container\nbody: model.dsl\nview: containers\n",
        )
        .unwrap();
        let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
        let mut tools = adp_diagram_c4::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
            DislPluginFunctions::none()
        }));
        let mut workspace = DiagramWorkspace::new(&root);
        let (_, session) = workspace.open(&mut tools, Path::new("model.adp")).unwrap();
        Project {
            _folder: folder,
            body,
            workspace,
            session,
        }
    }

    fn describe(&self, element_id: &str) -> Vec<PropertyRow> {
        self.session.properties(self.workspace.documents(), &[element_id])
    }

    fn set(&mut self, element_id: &str, property: &str, value: &str) -> Result<(), String> {
        self.session
            .set_property(&mut self.workspace, &[element_id], property, value)
    }

    fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }
}

fn row<'a>(rows: &'a [PropertyRow], id: &str) -> &'a PropertyRow {
    rows.iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
}

#[test]
fn a_container_offers_its_name_description_and_technology() {
    let project = Project::open();
    let rows = project.describe("web");
    assert_eq!(row(&rows, NAME).value, "Web");
    assert_eq!(row(&rows, DESCRIPTION).value, "Serves pages.");
    assert_eq!(row(&rows, TECHNOLOGY).value, "React");
}

#[test]
fn a_description_is_a_multi_line_editor_and_a_name_is_not() {
    let project = Project::open();
    let rows = project.describe("web");
    assert_eq!(row(&rows, DESCRIPTION).editor, PropertyEditor::Text);
    assert_eq!(row(&rows, NAME).editor, PropertyEditor::Line);
}

#[test]
fn a_container_with_no_technology_written_still_offers_the_row_empty() {
    let project = Project::open();
    assert_eq!(row(&project.describe("batch"), TECHNOLOGY).value, "");
}

#[test]
fn a_person_is_offered_no_technology_because_c4_gives_it_none() {
    let project = Project::open();
    let rows = project.describe("u");
    assert!(!rows.iter().any(|r| r.id == TECHNOLOGY));
    assert_eq!(row(&rows, NAME).value, "Customer");
}

#[test]
fn the_kind_and_identifier_are_shown_with_a_reason_rather_than_hidden() {
    let project = Project::open();
    let rows = project.describe("web");
    let kind = row(&rows, KIND);
    assert_eq!(kind.value, "Container");
    assert!(!kind.read_only_reason.is_empty(), "the kind is not editable");
    assert_eq!(kind.group, "Model");
    assert!(!row(&rows, IDENTIFIER).read_only_reason.is_empty());
}

#[test]
fn a_relationship_offers_its_description_and_technology_and_names_its_ends() {
    let project = Project::open();
    let relationship = adp_diagram_c4::parser::parse_text(MODEL).relationships[0].id();
    let rows = project.describe(&relationship);
    assert_eq!(row(&rows, RELATIONSHIP_DESCRIPTION).value, "Uses");
    assert_eq!(row(&rows, RELATIONSHIP_TECHNOLOGY).value, "HTTPS");
    assert_eq!(row(&rows, RELATIONSHIP_SOURCE).value, "Customer");
    assert!(!row(&rows, RELATIONSHIP_DESTINATION).read_only_reason.is_empty());
}

#[test]
fn setting_a_name_changes_the_document_and_is_one_undo_away() {
    let mut project = Project::open();
    let before = project.text();
    project.set("web", NAME, "Web Application").unwrap();
    assert!(project.text().contains("container \"Web Application\""));
    assert!(project.workspace.undo().is_done());
    assert_eq!(project.text(), before);
}

#[test]
fn setting_a_technology_changes_only_the_line_it_touches() {
    let mut project = Project::open();
    let before: Vec<String> = project.text().split('\n').map(str::to_owned).collect();
    project.set("web", TECHNOLOGY, "TypeScript and React").unwrap();
    let after: Vec<String> = project.text().split('\n').map(str::to_owned).collect();
    assert_eq!(before.len(), after.len());
    assert_eq!(before.iter().zip(&after).filter(|(a, b)| a != b).count(), 1);
}

#[test]
fn setting_a_property_of_something_gone_is_refused_rather_than_thrown() {
    let mut project = Project::open();
    let refusal = project.set("ghost", NAME, "Anything").unwrap_err();
    assert!(!refusal.is_empty());
}

#[test]
fn setting_a_property_that_is_not_offered_is_refused() {
    let mut project = Project::open();
    let refusal = project.set("web", "c4.invented", "Anything").unwrap_err();
    assert!(refusal.contains("c4.invented"), "{refusal}");
}

#[test]
fn nothing_is_described_for_a_selection_that_is_not_a_diagram_element() {
    let project = Project::open();
    // The document itself, selected in the explorer, names no element.
    assert!(project.describe("").is_empty());
}

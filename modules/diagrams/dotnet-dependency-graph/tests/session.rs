//! A registered solution opened in a project, and what is asked of it (etalii.adp spec 030, task T082): the standalone
//! host's `DotNetDependencyGraphSession.Tests.cs`, `DotNetContextPropertyProvider.Tests.cs`,
//! `DotNetContextSourceResolver.Tests.cs` and `DotNetDependencyGraphFlow.Tests.cs` at `8b6ccfc`, with the same
//! expectations. The standalone host's sessions heard the files through a watcher; here the project's reconciliation
//! hands a changed path to the module.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use adp_core::catalogue::Catalogue;
use adp_diagram::{
    DiagramDelta, DiagramElement, DiagramSession, DiagramTools, DiagramWorkspace, DiskChange, ElementKind,
};
use adp_diagram_dotnet_dependency_graph::view::{EDGE_REFUSAL, REPARENT_REFUSAL};
use adp_diagram_dotnet_dependency_graph::{mapper, registration_layout};
use adp_disl::DislPluginFunctions;
use common::{copy_tree, example, snapshot};

const CORE: &str = "project:src/Pipeline.Core/Pipeline.Core.csproj";
const STORAGE: &str = "project:src/Pipeline.Storage/Pipeline.Storage.csproj";
const SERILOG: &str = "package:Serilog";
const READ_ONLY: &str = "A .NET dependency graph shows the solution as it is and changes nothing in it. Edit the project file this value comes from, and the diagram will follow.";

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_dotnet_dependency_graph::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Opened {
    _folder: tempfile::TempDir,
    root: PathBuf,
    registration: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

fn open_at(folder: tempfile::TempDir, registration: &str) -> Result<Opened, String> {
    let root = folder.path().to_path_buf();
    let mut workspace = DiagramWorkspace::new(&root);
    let (opened, session) = workspace.open(&mut tools(), Path::new(registration))?;
    assert_eq!(opened.origin, "dotnet/dependency-graph");
    Ok(Opened {
        registration: root.join(registration),
        _folder: folder,
        root,
        workspace,
        session,
    })
}

fn open() -> Opened {
    let folder = tempfile::tempdir().unwrap();
    copy_tree(&example(), folder.path());
    open_at(folder, "PipelineToolkit.adp").unwrap()
}

impl Opened {
    fn delivered(&mut self) -> Vec<DiagramElement> {
        match self
            .session
            .baseline(self.workspace.documents())
            .unwrap()
            .into_iter()
            .next()
        {
            Some(DiagramDelta::Add(elements)) => elements,
            other => panic!("{other:?}"),
        }
    }

    fn rendered(&self) -> Vec<DiagramElement> {
        self.session.render(self.workspace.documents()).unwrap()
    }

    fn at(&self, id: &str) -> (f64, f64) {
        let element = self.rendered().into_iter().find(|e| e.id == id).unwrap();
        (element.x, element.y)
    }

    fn registration_text(&self) -> String {
        fs::read_to_string(&self.registration).unwrap()
    }

    fn stored(&self) -> BTreeMap<String, (f64, f64)> {
        registration_layout::read(&self.registration_text())
    }
}

// ---- the subject is the solution the registration names ----------------------------------------------------------------

#[test]
fn the_registration_routes_to_the_solution_its_body_header_names() {
    let mut opened = open();
    let elements = opened.delivered();
    let count = |t: &str| elements.iter().filter(|e| e.element_type == t).count();
    assert_eq!(count(mapper::PROJECT_TYPE), 4);
    assert_eq!(count(mapper::PACKAGE_TYPE), 3);
    assert_eq!(count(mapper::EDGE_TYPE), 9);
    assert!(opened.session.module_view().is_some());
    assert_eq!(opened.session.body_path(), opened.root.join("PipelineToolkit.slnx"));
}

#[test]
fn a_registration_without_a_body_header_routes_to_its_sibling_sln() {
    let folder = tempfile::tempdir().unwrap();
    copy_tree(&common::fixture("solution"), folder.path());
    fs::write(folder.path().join("Flat.adp"), "dotnet/dependency-graph\r\n").unwrap();
    let mut opened = open_at(folder, "Flat.adp").unwrap();
    assert_eq!(opened.session.body_path(), opened.root.join("Flat.sln"));
    assert_eq!(opened.delivered().len(), 2);
}

#[test]
fn a_body_header_outside_the_project_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(
        folder.path().join("Out.adp"),
        "dotnet/dependency-graph\r\nbody: ../Elsewhere.slnx\r\n",
    )
    .unwrap();
    let refusal = open_at(folder, "Out.adp").err().unwrap();
    assert!(refusal.contains("outside the project"), "{refusal}");
}

#[test]
fn every_element_carries_what_the_canvas_draws() {
    let mut opened = open();
    let elements = opened.delivered();
    let core = elements.iter().find(|e| e.id == CORE).unwrap();
    assert_eq!(core.kind, ElementKind::Node);
    assert_eq!((core.x, core.y), (0.0, 0.0));
    assert!(!core.placed);
    assert_eq!(core.attribute("name"), "Pipeline.Core");
    assert_eq!(
        core.attribute("projectRelativePath"),
        "src/Pipeline.Core/Pipeline.Core.csproj"
    );
    assert_eq!(core.attribute("targetFrameworks"), "net10.0");
    assert_eq!(core.attribute("width"), "220");
    assert_eq!(core.attribute("height"), "56");
    let storage = elements.iter().find(|e| e.id == STORAGE).unwrap();
    assert_eq!(storage.attribute("targetFrameworks"), "net10.0, netstandard2.0");
    let serilog = elements.iter().find(|e| e.id == SERILOG).unwrap();
    assert_eq!(serilog.element_type, mapper::PACKAGE_TYPE);
    assert_eq!((serilog.x, serilog.y), (1160.0, 0.0));
    assert_eq!(serilog.attribute("versions"), "3.1.1, 4.4.0");
    assert_eq!(serilog.attribute("hasVersionConflict"), "true");
    assert_eq!(serilog.attribute("dependentProjectCount"), "3");
    assert_eq!(serilog.attribute("isAmbient"), "false");
    let reference = elements
        .iter()
        .find(|e| e.id == format!("depends:{STORAGE}->{CORE}"))
        .unwrap();
    assert_eq!(reference.kind, ElementKind::Relation);
    assert_eq!(reference.source.as_deref(), Some(STORAGE));
    assert_eq!(reference.target.as_deref(), Some(CORE));
    assert_eq!(reference.attribute("kind"), "ProjectReference");
}

#[test]
fn a_selection_reads_as_its_file_its_package_or_its_reference_kind() {
    let opened = open();
    let documents = opened.workspace.documents();
    let core = opened.session.select(documents, CORE).unwrap();
    assert_eq!(core.path, ["src", "Pipeline.Core", "Pipeline.Core.csproj"]);
    assert_eq!(core.text, "Pipeline.Core");
    let serilog = opened.session.select(documents, SERILOG).unwrap();
    assert_eq!(serilog.path, ["Serilog"]);
    assert_eq!(serilog.text, "Serilog");
    let id = format!("depends:{CORE}->{SERILOG}");
    let reference = opened.session.select(documents, &id).unwrap();
    assert_eq!(reference.path, std::slice::from_ref(&id));
    assert_eq!(reference.text, "PackageReference");
    assert!(opened.session.select(documents, "parity:unknown").is_none());
}

#[test]
fn the_diagram_offers_no_toolbox_no_actions_and_no_problems() {
    let opened = open();
    let documents = opened.workspace.documents();
    assert!(opened.session.toolbox(documents).is_empty());
    assert!(opened.session.problems(documents).is_empty());
}

// ---- every row read-only ------------------------------------------------------------------------------------------------

#[test]
fn every_row_is_read_only_and_says_where_its_value_lives() {
    let opened = open();
    let rows = opened.session.properties(opened.workspace.documents(), &[CORE]);
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "dotnet.name",
            "dotnet.path",
            "dotnet.target-frameworks",
            "dotnet.dotnet-version"
        ]
    );
    assert!(
        rows.iter()
            .all(|r| r.read_only_reason
                == "Defined in src/Pipeline.Core/Pipeline.Core.csproj; edit it in a text editor.")
    );
    let package = opened.session.properties(opened.workspace.documents(), &[SERILOG]);
    assert_eq!(
        package[2].value,
        "This solution's projects reference this package at more than one version."
    );
}

#[test]
fn a_write_is_refused_with_the_rows_reason_and_writes_nothing() {
    let mut opened = open();
    let before = snapshot(&opened.root);
    let refusal = opened
        .session
        .set_property(&mut opened.workspace, &[CORE], "dotnet.name", "Renamed")
        .unwrap_err();
    assert_eq!(
        refusal,
        "Defined in src/Pipeline.Core/Pipeline.Core.csproj; edit it in a text editor."
    );
    // A row the element does not have is refused with the definition's sentence.
    let refusal = opened
        .session
        .set_property(&mut opened.workspace, &[CORE], "dotnet.package-id", "Renamed")
        .unwrap_err();
    assert_eq!(refusal, READ_ONLY);
    assert_eq!(snapshot(&opened.root), before);
}

// ---- the one edit: a position in the registration ------------------------------------------------------------------------

#[test]
fn a_move_stores_the_position_in_the_registration_and_nothing_else() {
    let mut opened = open();
    let before = snapshot(&opened.root);
    opened
        .session
        .move_element_to(&mut opened.workspace, CORE, 40.0, 20.5)
        .unwrap();
    assert_eq!(
        opened.registration_text(),
        // The block is written in CRLF whatever the registration's own line ending (`AdpFileWriter.NewLine`).
        "dotnet/dependency-graph\nbody: PipelineToolkit.slnx\nlayout:\r\n  project:src/Pipeline.Core/Pipeline.Core.csproj: 40 20.5\r\n"
    );
    assert_eq!(opened.at(CORE), (40.0, 20.5));
    assert!(opened.rendered().iter().find(|e| e.id == CORE).unwrap().placed);
    // Every file but the registration is as it was.
    let after = snapshot(&opened.root);
    let unchanged = |files: &[(PathBuf, Vec<u8>)]| {
        files
            .iter()
            .filter(|(p, _)| *p != opened.registration)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(unchanged(&after), unchanged(&before));
}

#[test]
fn a_move_is_one_undo_away_and_the_first_one_undone_leaves_no_entry() {
    let mut opened = open();
    let original = opened.registration_text();
    opened
        .session
        .move_element_to(&mut opened.workspace, SERILOG, 1000.0, 300.0)
        .unwrap();
    opened
        .session
        .move_element_to(&mut opened.workspace, SERILOG, 1200.0, 310.0)
        .unwrap();
    opened.workspace.undo();
    assert_eq!(opened.stored()[SERILOG], (1000.0, 300.0));
    opened.workspace.undo();
    assert_eq!(opened.registration_text(), original);
    assert_eq!(opened.at(SERILOG), (1160.0, 0.0));
    opened.workspace.redo();
    assert_eq!(opened.at(SERILOG), (1000.0, 300.0));
}

#[test]
fn a_reference_cannot_be_moved_it_follows_its_ends() {
    let mut opened = open();
    let id = format!("depends:{CORE}->{SERILOG}");
    let refusal = opened
        .session
        .move_element_to(&mut opened.workspace, &id, 1.0, 1.0)
        .unwrap_err();
    assert_eq!(refusal, EDGE_REFUSAL);
    assert!(opened.stored().is_empty());
}

#[test]
fn nothing_can_be_reparented() {
    let mut opened = open();
    let refusal = opened
        .session
        .move_element(&mut opened.workspace, CORE, Some(STORAGE), 0)
        .unwrap_err();
    assert_eq!(refusal, REPARENT_REFUSAL);
}

#[test]
fn a_stored_position_for_an_id_that_is_gone_is_ignored() {
    let folder = tempfile::tempdir().unwrap();
    copy_tree(&example(), folder.path());
    fs::write(
        folder.path().join("PipelineToolkit.adp"),
        "dotnet/dependency-graph\r\nbody: PipelineToolkit.slnx\r\nlayout:\r\n  project:gone.csproj: 1 1\r\n  package:Serilog: 5 6\r\n",
    )
    .unwrap();
    let mut opened = open_at(folder, "PipelineToolkit.adp").unwrap();
    let elements = opened.delivered();
    assert_eq!(elements.len(), 16);
    assert!(!elements.iter().any(|e| e.id == "project:gone.csproj"));
    assert_eq!(opened.at(SERILOG), (5.0, 6.0));
}

// ---- a change on disk ---------------------------------------------------------------------------------------------------

#[test]
fn a_reference_added_to_a_project_file_is_read_again() {
    let mut opened = open();
    opened.delivered();
    let project = opened.root.join("src/Pipeline.Core/Pipeline.Core.csproj");
    fs::write(
        &project,
        "<Project Sdk=\"Microsoft.NET.Sdk\">\r\n  <PropertyGroup>\r\n    <TargetFramework>net10.0</TargetFramework>\r\n  </PropertyGroup>\r\n  <ItemGroup>\r\n    <PackageReference Include=\"Serilog\" />\r\n    <PackageReference Include=\"Polly\" Version=\"8.0.0\" />\r\n  </ItemGroup>\r\n</Project>\r\n",
    )
    .unwrap();
    let change = opened.workspace.reconcile(&project);
    assert!(matches!(change, DiskChange::Reloaded(_)), "{change:?}");
    assert!(opened.rendered().iter().any(|e| e.id == "package:Polly"));
}

#[test]
fn a_version_bump_in_the_central_props_file_is_read_again() {
    let mut opened = open();
    let props = opened.root.join("Directory.Packages.props");
    let text = fs::read_to_string(&props).unwrap().replace("10.0.0", "10.0.1");
    fs::write(&props, text).unwrap();
    assert!(matches!(opened.workspace.reconcile(&props), DiskChange::Reloaded(_)));
    let json = opened
        .rendered()
        .into_iter()
        .find(|e| e.id == "package:System.Text.Json")
        .unwrap();
    assert_eq!(json.attribute("versions"), "10.0.1");
}

#[test]
fn a_file_the_graph_does_not_read_is_not_tracked() {
    let mut opened = open();
    let other = opened.root.join("notes.txt");
    fs::write(&other, "hello").unwrap();
    assert!(matches!(opened.workspace.reconcile(&other), DiskChange::NotTracked));
}

#[test]
fn the_registration_changed_by_hand_is_drawn_from() {
    let mut opened = open();
    fs::write(
        &opened.registration,
        "dotnet/dependency-graph\r\nbody: PipelineToolkit.slnx\r\nlayout:\r\n  package:Serilog: 7 8\r\n",
    )
    .unwrap();
    let registration = opened.registration.clone();
    assert!(matches!(
        opened.workspace.reconcile(&registration),
        DiskChange::Reloaded(_)
    ));
    assert_eq!(opened.at(SERILOG), (7.0, 8.0));
}

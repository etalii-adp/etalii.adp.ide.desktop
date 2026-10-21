//! End to end through a DISL and FBL tool type (etalii.adp spec 030, Story 2 scenarios 1, 4 and 5; tasks T056 to
//! T058): the standalone host's `business-philosophy` causal loop diagram, opened through its registration, its
//! positions read from the registration's `layout:`, edited by a drag, a property change of two variables at once and
//! a context-menu operation, one edit undone, and every write checked byte for byte; then a change on disk
//! reconciled with the history's barrier while the store's own writes are ignored.
//!
//! The example is `src/diagrams/causal-loop-diagram/examples/business-philosophy/` of etalii.adp.ide.standalone at
//! `8b6ccfc`, copied unchanged to `tests/fixtures/causal-loop-diagram/business-philosophy/`.

use std::fs;
use std::path::{Path, PathBuf};

use adp_core::Catalogue;
use adp_diagram::document::DiskChange;
use adp_diagram::element::{DiagramDelta, ElementKind};
use adp_diagram::session::{ActionOutcome, ActionTarget, DiagramSession};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;
use adp_history::{History, Outcome};

const ORIGIN: &str = "systems/causal-loop-diagram";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/causal-loop-diagram/business-philosophy")
}

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
}

struct Project {
    _folder: tempfile::TempDir,
    root: PathBuf,
    body: PathBuf,
    registration: PathBuf,
    original_body: String,
    original_registration: String,
}

fn project() -> Project {
    let folder = tempfile::tempdir().unwrap();
    let root = folder.path().to_path_buf();
    for name in ["business-philosophy.adp", "business-philosophy.cld"] {
        fs::copy(fixtures().join(name), root.join(name)).unwrap();
    }
    Project {
        body: root.join("business-philosophy.cld"),
        registration: root.join("business-philosophy.adp"),
        original_body: fs::read_to_string(fixtures().join("business-philosophy.cld")).unwrap(),
        original_registration: fs::read_to_string(fixtures().join("business-philosophy.adp")).unwrap(),
        root,
        _folder: folder,
    }
}

fn open(project: &Project) -> (DiagramWorkspace, DiagramSession, DiagramTools) {
    let mut tools = tools();
    let mut workspace = DiagramWorkspace::new(&project.root);
    let (opened, session) = workspace
        .open(&mut tools, Path::new("business-philosophy.adp"))
        .expect("the diagram opens");
    assert_eq!(opened.origin, ORIGIN);
    assert_eq!(opened.body_path, project.body);
    assert_eq!(
        opened.registration_path.as_deref(),
        Some(project.registration.as_path())
    );
    (workspace, session, tools)
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn a_registration_opens_its_body_with_the_positions_it_stores() {
    let project = project();
    let (workspace, mut session, _) = open(&project);

    let baseline = session.baseline(workspace.documents()).unwrap();
    let [DiagramDelta::Add(elements)] = baseline.as_slice() else {
        panic!("a baseline is one add, not {baseline:?}");
    };
    let variables: Vec<_> = elements.iter().filter(|e| e.element_type == "Variable").collect();
    assert_eq!(variables.len(), 18);
    let insight = elements.iter().find(|e| e.id == "variable:insight").unwrap();
    assert_eq!((insight.x, insight.y, insight.placed), (20.0, 120.0, true));
    assert_eq!(insight.attribute("label"), "Depth of shared insight");
    let link = elements.iter().find(|e| e.id == "link:insight|applicability").unwrap();
    assert_eq!(link.kind, ElementKind::Relation);
    assert_eq!(
        (link.source.as_deref(), link.target.as_deref()),
        (Some("variable:insight"), Some("variable:applicability"))
    );
}

#[test]
fn edits_are_written_as_splices_and_one_undo_takes_back_a_change_to_two_elements() {
    let project = project();
    let (mut workspace, mut session, _) = open(&project);
    session.baseline(workspace.documents()).unwrap();

    // A drag: the position goes to the registration, and the body is not touched.
    session
        .move_element_to(&mut workspace, "variable:insight", 25.5, 130.0)
        .unwrap();
    let moved_registration = project
        .original_registration
        .replace("  variable:insight: 20 120", "  variable:insight: 25.5 130");
    assert_ne!(moved_registration, project.original_registration);
    assert_eq!(read(&project.registration), moved_registration);
    assert_eq!(read(&project.body), project.original_body);

    // The view hears the edit as the one element that changed.
    let changed = workspace.take_changed();
    assert_eq!(changed, std::slice::from_ref(&project.body));
    let change = session.on_document_changed(workspace.documents(), &project.body);
    let adp_diagram::change_handler::DocumentChange::Changed(deltas) = change else {
        panic!("a change, not {change:?}");
    };
    let [DiagramDelta::Add(moved)] = deltas.as_slice() else {
        panic!("one add, not {deltas:?}");
    };
    assert_eq!(moved.len(), 1);
    assert_eq!(
        (moved[0].id.as_str(), moved[0].x, moved[0].y),
        ("variable:insight", 25.5, 130.0)
    );

    // A property edit of two variables at once: one command.
    let rows = session.properties(workspace.documents(), &["variable:insight", "variable:trust"]);
    let label = rows
        .iter()
        .find(|r| r.label == "Label")
        .expect("a label row both variables share");
    assert_eq!(label.value, "", "the two labels differ, so the shared row shows none");
    session
        .set_property(
            &mut workspace,
            &["variable:insight", "variable:trust"],
            &label.id,
            "Shared label",
        )
        .unwrap();
    let relabelled = project
        .original_body
        .replace(
            "variable insight \"Depth of shared insight\"",
            "variable insight \"Shared label\"",
        )
        .replace(
            "variable trust \"Trust and openness\"",
            "variable trust \"Shared label\"",
        );
    assert_ne!(relabelled, project.original_body);
    assert_eq!(read(&project.body), relabelled);

    // An operation from a link's context menu.
    let link = ActionTarget::Element("link:insight|applicability".into());
    let menu = session.actions(workspace.documents(), &link);
    let negative = menu
        .iter()
        .flat_map(|g| &g.actions)
        .find(|a| a.id == "causal-loop.make-negative")
        .expect("the link offers the opposite direction");
    assert!(negative.is_available());
    assert_eq!(
        session.execute(&mut workspace, &link, "causal-loop.make-negative", &ids::fixed(&[])),
        ActionOutcome::Done
    );
    let negated = relabelled.replace(
        "link insight -> applicability + weight=1.2",
        "link insight -> applicability - weight=1.2",
    );
    assert_eq!(read(&project.body), negated);

    // Undo the operation, then the change to two elements, in one step each.
    assert!(workspace.undo().is_done());
    assert_eq!(read(&project.body), relabelled);
    assert!(workspace.undo().is_done());
    assert_eq!(read(&project.body), project.original_body);
    assert_eq!(read(&project.registration), moved_registration);

    // Redo puts the same bytes back.
    assert!(workspace.redo().is_done());
    assert_eq!(read(&project.body), relabelled);

    // And the drag is one undo further.
    assert!(workspace.undo().is_done());
    assert!(workspace.undo().is_done());
    assert_eq!(read(&project.registration), project.original_registration);
    assert_eq!(read(&project.body), project.original_body);
    assert!(!workspace.history().availability().can_undo);
}

#[test]
fn a_body_changed_on_disk_is_read_again_and_undo_stops_there_while_own_writes_are_ignored() {
    let project = project();
    let (mut workspace, mut session, _) = open(&project);
    session.baseline(workspace.documents()).unwrap();
    session
        .move_element_to(&mut workspace, "variable:trust", -600.0, -150.0)
        .unwrap();
    session.on_document_changed(workspace.documents(), &project.body);

    // The watcher reports the store's own write: nothing happens.
    assert_eq!(workspace.reconcile(&project.registration), DiskChange::OwnWrite);
    assert!(workspace.history().availability().can_undo);

    // Another program adds a variable.
    let external = project.original_body.replace(
        "variable quickFixPressure \"Pressure for quick solutions\"",
        "variable quickFixPressure \"Pressure for quick solutions\"\nvariable outsider \"Written elsewhere\"",
    );
    fs::write(&project.body, &external).unwrap();
    assert_eq!(
        workspace.reconcile(&project.body),
        DiskChange::Reloaded(project.body.clone())
    );
    let change = session.on_document_changed(workspace.documents(), &project.body);
    let adp_diagram::change_handler::DocumentChange::Changed(deltas) = change else {
        panic!("a change, not {change:?}");
    };
    assert!(
        deltas
            .iter()
            .any(|d| matches!(d, DiagramDelta::Add(added) if added.iter().any(|e| e.id == "variable:outsider")))
    );

    // Undo stops at the change from outside, and writes nothing.
    assert_eq!(
        workspace.undo(),
        Outcome::Refused(adp_history::Refusal::new(History::<()>::CHANGED_OUTSIDE))
    );
    assert_eq!(read(&project.body), external);

    // Editing goes on from the new text.
    session
        .set_property(
            &mut workspace,
            &["variable:outsider"],
            "causal-loop.variable-label",
            "Read again",
        )
        .unwrap();
    assert_eq!(
        read(&project.body),
        external.replace("\"Written elsewhere\"", "\"Read again\"")
    );
    assert_eq!(workspace.reconcile(&project.body), DiskChange::OwnWrite);
}

#[test]
fn a_deleted_body_empties_the_diagram_and_refuses_edits() {
    let project = project();
    let (mut workspace, mut session, _) = open(&project);
    session.baseline(workspace.documents()).unwrap();
    fs::remove_file(&project.body).unwrap();
    assert_eq!(
        workspace.reconcile(&project.body),
        DiskChange::Deleted(project.body.clone())
    );
    let refusal = session
        .set_property(&mut workspace, &["variable:insight"], "causal-loop.variable-label", "x")
        .unwrap_err();
    assert!(!refusal.is_empty());
}

#[test]
fn a_viewport_narrows_the_view_to_the_nodes_it_touches_and_the_relations_between_them() {
    let project = project();
    let (workspace, mut session, _) = open(&project);
    session.baseline(workspace.documents()).unwrap();
    let all = session.delivered().len();

    // Around insight (20, 120) and narratives (-270, 120), and nothing else.
    let viewport = adp_diagram::DiagramViewport::new(-280.0, 110.0, 30.0, 130.0);
    let deltas = session.update_view(workspace.documents(), viewport).unwrap();
    let [DiagramDelta::Remove(removed)] = deltas.as_slice() else {
        panic!("only removals, not {deltas:?}");
    };
    let mut kept: Vec<&str> = session.delivered().iter().map(|e| e.id.as_str()).collect();
    kept.sort_unstable();
    assert_eq!(removed.len() + kept.len(), all);
    assert!(
        kept.contains(&"variable:insight") && kept.contains(&"variable:narratives"),
        "{kept:?}"
    );
    assert!(
        session
            .delivered()
            .iter()
            .filter(|e| e.kind == ElementKind::Relation)
            .all(|e| kept.contains(&e.source.as_deref().unwrap()) && kept.contains(&e.target.as_deref().unwrap()))
    );

    // Widening it again brings the rest back as one add.
    let deltas = session
        .update_view(workspace.documents(), adp_diagram::DiagramViewport::UNBOUNDED)
        .unwrap();
    let [DiagramDelta::Add(added)] = deltas.as_slice() else {
        panic!("one add, not {deltas:?}");
    };
    assert_eq!(added.len(), removed.len());
}

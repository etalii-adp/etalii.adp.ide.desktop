//! The C4 commands held to the contract every module's edit handlers are, applied as the standalone host's
//! dispatcher applied them (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4.Tests/C4CommandHandlers.Tests.cs`
//! and the contract it derives from, `src/backend/EtAlii.Adp.TestSupport/EditHandlerContract.cs`, at `8b6ccfc`, over
//! the reference example `courier.dsl`: an edit lands with an inverse, an edit of a missing element is refused and
//! writes nothing, and adding and removing a view keep its registration in step. The contract's warning of a save is
//! carried by a store decorator the standalone tests injected; this store's save has no warning to carry, so those
//! halves are not ported.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram::DiagramDocuments;
use adp_diagram_c4::commands::{AddC4View, SetElementName};
use adp_diagram_c4::model::C4ViewKind;
use adp_diagram_c4::store;
use adp_history::Command;

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/examples/reference/architecture/courier.dsl")
}

fn copy(folder: &Path) -> PathBuf {
    let body = folder.join("courier.dsl");
    fs::copy(example(), &body).unwrap();
    body
}

fn opened(body: &Path) -> DiagramDocuments {
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, body);
    documents
}

#[test]
fn an_edit_lands_and_can_be_undone() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy(folder.path());
    let before = fs::read(&body).unwrap();
    let mut documents = opened(&body);
    let applied = SetElementName {
        body_path: body.clone(),
        element_id: "tracking".into(),
        name: "Parcel Tracking".into(),
    }
    .apply(&mut documents)
    .unwrap();
    assert_ne!(fs::read(&body).unwrap(), before);
    assert_eq!(applied.warning, "");
    applied.inverse.expect("an inverse").apply(&mut documents).unwrap();
    assert_eq!(fs::read(&body).unwrap(), before);
}

#[test]
fn an_edit_of_a_missing_element_is_refused_and_writes_nothing() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy(folder.path());
    let before = fs::read(&body).unwrap();
    let mut documents = opened(&body);
    let refused = SetElementName {
        body_path: body.clone(),
        element_id: "no-such-element".into(),
        name: "Parcel Tracking".into(),
    }
    .apply(&mut documents);
    let refusal = refused.err().expect("refused");
    assert_ne!(refusal.0, "");
    assert_eq!(fs::read(&body).unwrap(), before);
}

#[test]
fn adding_and_removing_a_view_keep_the_registration_in_step() {
    let folder = tempfile::tempdir().unwrap();
    let body = copy(folder.path());
    let registration = folder.path().join("landscape.adp");
    let mut documents = opened(&body);
    let added = AddC4View {
        body_path: body.clone(),
        registration_path: registration.clone(),
        origin: "c4/system-landscape".into(),
        view_kind: C4ViewKind::SystemLandscape,
        view_key: "added-landscape".into(),
        body_relative_path: "courier.dsl".into(),
    }
    .apply(&mut documents)
    .unwrap();
    let registered = registration.exists();
    added.inverse.expect("an inverse").apply(&mut documents).unwrap();
    assert!(registered);
    assert!(!registration.exists());
}

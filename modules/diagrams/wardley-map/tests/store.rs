//! The one owner of the open maps and the identities held beside each (etalii.adp spec 030, task T070), ported from
//! the standalone host's `WardleyDocumentStore.Tests.cs`, `WardleyDocumentStore.Identities.Tests.cs`,
//! `WardleyDocumentStore.Sharing.Tests.cs`, `WardleyDocumentStore.SharedRead.Tests.cs`,
//! `WardleyCommandHandlers.Tests.cs`, `WardleyEmptyDocument.Tests.cs` and `Diagram.Tests.cs` at `8b6ccfc`. The shared
//! store and reloader contracts are the project's documents' and `adp-diagram`'s; what is the module's is here. Without
//! a counterpart: `Touch` (nothing in the desktop host asks a map to redeliver unchanged), the case-insensitive path
//! (the project's documents key a body by its path), and the null and blank paths and names.

mod common;

use std::fs;

use adp_diagram::{DiagramDocuments, DiskChange, ModuleDocuments};
use adp_diagram_wardley_map::commands::MoveWardleyElement;
use adp_diagram_wardley_map::lines::LineDocument;
use adp_diagram_wardley_map::store::{self, STATE_KEY, WardleyStore};
use adp_diagram_wardley_map::{EXTENSION, ORIGIN, empty_document, identities, writer};
use common::{Project, fixture};

/// A copy of the `minimal.owm` fixture in a temporary folder, as `tea.owm`.
fn minimal() -> (tempfile::TempDir, std::path::PathBuf) {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("tea.owm");
    fs::write(&body, fixture("minimal.owm")).unwrap();
    (folder, body)
}

#[test]
fn an_entry_carries_the_lines_its_parser_passed_over() {
    let mut documents = DiagramDocuments::default();
    let body = common::fixtures().join("unmodelled-statements.owm");
    let document = store::get_or_load(&mut documents, &body);
    assert!(document.is_usable());
    assert_eq!(document.problems.len(), 1);
    assert_eq!(document.problems[0].line, 13);
}

#[test]
fn a_missing_body_keeps_its_identities_on_reload_and_loses_them_only_to_the_watchers_delete() {
    let (_folder, body) = minimal();
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, &body);
    let held = store::identities_of(&documents, &body).to_vec();
    assert!(!held.is_empty());
    fs::remove_file(&body).unwrap();

    // A reload that finds no file keeps the last good document, and with it its ids.
    store::reload(&mut documents, &body);
    let kept = store::identities_of(&documents, &body).to_vec();
    // The watcher's evidence that it is gone.
    let change = documents.module_state::<WardleyStore>(STATE_KEY).reconcile(&body);

    assert_eq!(kept, held);
    assert_eq!(change, DiskChange::Deleted(body.clone()));
    assert!(store::identities_of(&documents, &body).is_empty());
}

#[test]
fn an_external_edit_keeps_the_ids_of_what_survived_and_gives_the_newcomer_one() {
    let (_folder, body) = minimal();
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, &body);
    let before = store::identities_of(&documents, &body).to_vec();
    let mut text = fs::read_to_string(&body).unwrap();
    text.push_str("component Kettle [0.45, 0.57]\n");
    fs::write(&body, text).unwrap();

    store::reload(&mut documents, &body);
    let after = store::identities_of(&documents, &body).to_vec();

    // A `git pull` does not reset a selection on what it left alone.
    assert!(before.iter().all(|e| after.contains(e)));
    let newcomers: Vec<_> = after.iter().filter(|e| !before.contains(e)).collect();
    assert_eq!(newcomers.len(), 1);
    assert_eq!(newcomers[0].key, "Kettle");
}

#[test]
fn a_forgotten_document_forgets_its_identities_and_reads_the_sidecar_afresh() {
    let (_folder, body) = minimal();
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, &body);
    let minted: Vec<String> = store::identities_of(&documents, &body)
        .iter()
        .map(|e| e.id.clone())
        .collect();
    documents.module_state::<WardleyStore>(STATE_KEY).close(&body);
    store::get_or_load(&mut documents, &body);
    let again: Vec<String> = store::identities_of(&documents, &body)
        .iter()
        .map(|e| e.id.clone())
        .collect();
    // No sidecar was written, so the next open mints its own; the old ones are not held.
    assert_ne!(minted, again);
}

#[test]
fn a_save_whose_identities_cannot_be_kept_lands_the_map_and_says_so() {
    let (_folder, body) = minimal();
    // A folder where the identity sidecar goes, which no platform lets a file replace.
    fs::create_dir_all(identities::path_for(&body)).unwrap();
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &body).lines();
    writer::replace_line(&mut lines, 1, "title Kept anyway");
    let warning = store::save(&mut documents, &body, &lines).unwrap();
    assert!(fs::read_to_string(&body).unwrap().contains("title Kept anyway"));
    assert!(warning.contains("new ids"), "{warning}");
}

#[test]
fn get_or_load_reads_a_document_an_editor_is_still_writing() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    let text = "title Shared\ncomponent A [0.5, 0.5]\n";
    fs::write(&body, text).unwrap();
    // The handle a save holds: write access.
    let _editor = fs::OpenOptions::new().write(true).open(&body).unwrap();
    let mut documents = DiagramDocuments::default();
    assert_eq!(store::get_or_load(&mut documents, &body).text, text);
}

#[test]
fn get_or_load_returns_the_same_document_so_two_sessions_share_one() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(&body, "title Shared\n").unwrap();
    let mut documents = DiagramDocuments::default();
    let first = store::get_or_load(&mut documents, &body).map.clone();
    let second = store::get_or_load(&mut documents, &body).map.clone();
    assert!(std::rc::Rc::ptr_eq(&first, &second));
}

#[test]
fn get_or_load_opens_a_missing_file_as_an_empty_document_rather_than_throwing() {
    let folder = tempfile::tempdir().unwrap();
    let mut documents = DiagramDocuments::default();
    let document = store::get_or_load(&mut documents, &folder.path().join("absent.owm"));
    assert!(document.is_usable());
    assert_eq!(document.text, "");
}

#[test]
fn save_leaves_no_temporary_file_behind() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(&body, "title Before\n").unwrap();
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &body).lines();
    writer::replace_line(&mut lines, 1, "title After");
    store::save(&mut documents, &body, &lines).unwrap();
    // The map has no element, so it has no identities to keep beside it either.
    let names: Vec<String> = fs::read_dir(folder.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["map.owm"]);
}

#[test]
fn save_preserves_the_documents_bytes_including_crlf_and_a_missing_final_newline() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    let text = "title Kept\r\nanchor A [0.9, 0.5]";
    fs::write(&body, text).unwrap();
    let mut documents = DiagramDocuments::default();
    let lines = store::get_or_load(&mut documents, &body).lines();
    store::save(&mut documents, &body, &lines).unwrap();
    assert_eq!(fs::read_to_string(&body).unwrap(), text);
}

#[test]
fn save_creates_the_file_when_the_sibling_did_not_exist_yet() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("new.owm");
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &body).lines();
    writer::insert_line(&mut lines, 1, "title Created");
    store::save(&mut documents, &body, &lines).unwrap();
    // CRLF, and terminated: a document with no line breaks to learn from takes the core line document's ending.
    assert_eq!(fs::read_to_string(&body).unwrap(), "title Created\r\n");
}

#[test]
fn a_byte_order_mark_is_kept_through_an_edit() {
    let folder = tempfile::tempdir().unwrap();
    let body = folder.path().join("map.owm");
    fs::write(&body, "\u{feff}title Marked\n").unwrap();
    let mut documents = DiagramDocuments::default();
    let mut lines = store::get_or_load(&mut documents, &body).lines();
    assert_eq!(
        lines.text(),
        "title Marked\n",
        "the mark is not part of the text the parser reads"
    );
    writer::append(&mut lines, "anchor A [0.9, 0.5]");
    store::save(&mut documents, &body, &lines).unwrap();
    assert_eq!(
        fs::read_to_string(&body).unwrap(),
        "\u{feff}title Marked\nanchor A [0.9, 0.5]\n"
    );
}

// ---- WardleyCommandHandlers.Tests.cs --------------------------------------------------------------------------------

#[test]
fn a_move_and_its_line_restore_carry_the_save_warning() {
    let mut project = Project::new(&fixture("minimal.owm"));
    let original = project.read();
    // A folder where the identity sidecar goes, so every save warns that the ids were not kept.
    fs::create_dir_all(identities::path_for(&project.body)).unwrap();
    let command = MoveWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Cup of Tea"),
        visibility: 0.5,
        maturity: 0.5,
    };
    let warning = project.execute(&command).unwrap();
    assert!(warning.contains("new ids"), "{warning}");
    match project.workspace.undo() {
        adp_history::Outcome::Done { warning, .. } => assert!(warning.contains("new ids"), "{warning}"),
        adp_history::Outcome::Refused(refusal) => panic!("{}", refusal.0),
    }
    assert_eq!(project.read(), original);
}

#[test]
fn an_edit_of_a_map_that_could_not_be_read_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    // A folder where the body should be: it exists, and it cannot be read.
    let body = folder.path().join("map.owm");
    fs::create_dir_all(&body).unwrap();
    let mut documents = DiagramDocuments::default();
    assert!(!store::get_or_load(&mut documents, &body).is_usable());
    let lines = LineDocument::parse("title Over it\n");
    let refusal = store::save(&mut documents, &body, &lines).unwrap_err();
    assert!(refusal.contains("could not be read"), "{refusal}");
    assert!(body.is_dir());
}

// ---- WardleyEmptyDocument.Tests.cs and Diagram.Tests.cs -------------------------------------------------------------

#[test]
fn create_empty_document_carries_the_file_base_name_as_the_title() {
    assert_eq!(empty_document("map"), "title map\n");
}

#[test]
fn create_empty_document_uses_the_name_given_rather_than_a_fixed_one() {
    assert_eq!(empty_document("supply-chain"), "title supply-chain\n");
}

#[test]
fn create_empty_document_ends_with_a_line_feed_and_no_carriage_return() {
    let document = empty_document("map");
    assert!(document.ends_with('\n'));
    assert!(!document.contains('\r'));
}

#[test]
fn create_empty_document_is_a_valid_map_rather_than_an_empty_file() {
    let document = empty_document("map");
    assert!(!document.is_empty());
    assert!(document.starts_with("title "));
}

#[test]
fn definition_extension_is_the_online_wardley_maps_extension() {
    assert_eq!(EXTENSION, ".owm");
    let definitions = adp_core::definitions::standalone();
    let definition = definitions
        .iter()
        .find(|d| d.origin == ORIGIN)
        .expect("the catalogue defines the Wardley map");
    assert_eq!(definition.extension, EXTENSION);
}

#[test]
fn definition_has_document_sibling_because_the_body_lives_in_the_owm_file() {
    let definitions = adp_core::definitions::standalone();
    let definition = definitions.iter().find(|d| d.origin == ORIGIN).unwrap();
    assert!(definition.has_document_sibling());
}

#[test]
fn definition_mime_type_is_the_first_line_of_an_adp_registration() {
    let definitions = adp_core::definitions::standalone();
    let definition = definitions.iter().find(|d| d.origin == ORIGIN).unwrap();
    assert_eq!(definition.mime_type(), "wardley/map");
}

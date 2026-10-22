//! The lines a document is made of, and the family's one store: load-once, byte-identical saves of untouched
//! documents, errors carried rather than thrown, broken files refused on save, a reload that replaces the entry and
//! one that keeps the last good document. Ported from the standalone host's `RdfDocument.Tests.cs`,
//! `RdfDocumentStore.Tests.cs`, `RdfDocumentStore.LostEdit.Tests.cs`, `RdfDocumentReloader.Tests.cs` and
//! `RdfEmptyDocument.Tests.cs` at `8b6ccfc`. The generic store and reloader contracts (`DocumentStoreContract`,
//! `DocumentReloaderContract`) are the core's and are held by `adp-diagram`'s own tests of `DiagramDocuments`.

mod common;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use adp_diagram::{DiagramDocuments, DiskChange};
use adp_diagram_rdf::lines::{LineDocument, LineRange};
use adp_diagram_rdf::{parser, store, validator};

#[test]
fn parse_and_text_round_trip_byte_identically() {
    for text in [
        "one\r\ntwo\r\n",
        "one\ntwo\n",
        "one\r\ntwo",
        "one\ntwo\r\nthree",
        "",
        "\n",
        "just one line, no newline",
    ] {
        assert_eq!(LineDocument::parse(text).text(), text);
    }
}

#[test]
fn inserting_at_an_unterminated_end_moves_the_missing_terminator_and_is_reversible() {
    let mut document = LineDocument::parse("a\r\nb");

    document.insert(2, &["c".to_owned()]);
    assert_eq!(document.text(), "a\r\nb\r\nc");

    document.remove(LineRange::new(2, 2));
    assert_eq!(document.text(), "a\r\nb");
}

#[test]
fn removing_the_last_line_passes_its_ending_to_the_line_before() {
    let mut document = LineDocument::parse("a\r\nb\r\nc");

    document.remove(LineRange::new(2, 2));

    assert_eq!(document.text(), "a\r\nb");
}

#[test]
fn replacing_a_middle_line_touches_no_other_line() {
    let mut document = LineDocument::parse("a\r\nb\nc\r\n");

    document.replace(LineRange::new(1, 1), &["B".to_owned()]);

    assert_eq!(document.text(), "a\r\nB\nc\r\n");
}

#[test]
fn a_tied_file_gives_an_inserted_line_crlf() {
    let text = common::fixture("tied-line-endings.ttl");
    let mut document = LineDocument::parse(&text);
    let lf = document.lines().iter().filter(|l| l.ending == "\n").count();
    let crlf = document.lines().iter().filter(|l| l.ending == "\r\n").count();
    assert_eq!(lf, crlf);

    document.insert(2, &["ex:g ex:h ex:i .".to_owned()]);

    assert_eq!(document.lines()[2].ending, "\r\n");
    document.remove(LineRange::new(2, 2));
    assert_eq!(document.text(), text);
}

fn copy_fixture(folder: &tempfile::TempDir, name: &str) -> PathBuf {
    let destination = folder.path().join(name);
    fs::copy(common::fixture_path(name), &destination).unwrap();
    destination
}

#[test]
fn saving_an_untouched_document_leaves_the_file_byte_identical() {
    for name in [
        "constructs.ttl",
        "crlf-line-endings.ttl",
        "lf-line-endings.ttl",
        "no-trailing-newline.ttl",
        "tied-line-endings.ttl",
        "simple.nt",
    ] {
        let folder = tempfile::tempdir().unwrap();
        let path = copy_fixture(&folder, name);
        let original = fs::read(&path).unwrap();
        let mut documents = DiagramDocuments::default();
        let text = store::get_or_load(&mut documents, &path).lines.text();

        assert_eq!(store::save(&mut documents, &path, &text), Ok(()), "{name}");
        assert_eq!(fs::read(&path).unwrap(), original, "{name}");
    }
}

#[test]
fn a_missing_file_opens_as_an_empty_usable_document() {
    let folder = tempfile::tempdir().unwrap();
    let mut documents = DiagramDocuments::default();

    let document = store::get_or_load(&mut documents, &folder.path().join("not-yet.ttl"));

    assert!(document.is_usable());
    assert!(document.lines.is_empty());
    assert!(document.model.triples.is_empty());
}

#[test]
fn a_file_that_is_not_turtle_carries_the_error_and_its_line_and_save_refuses() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "broken.ttl");
    let original = fs::read(&path).unwrap();
    let mut documents = DiagramDocuments::default();

    let document = store::get_or_load(&mut documents, &path).clone();
    let refusal = store::save(&mut documents, &path, &document.text).unwrap_err();

    assert!(!document.is_usable());
    let unreadable = document.unreadable.clone().unwrap();
    assert!(unreadable.contains("at line"), "{unreadable}");
    let (line, message) = document.problem.clone().unwrap();
    assert_eq!(unreadable, format!("it does not parse at line {}. {message}", line + 1));
    assert!(refusal.contains("broken.ttl"), "{refusal}");
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn one_document_is_loaded_once_and_shared_by_every_reading() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "constructs.ttl");
    let mut documents = DiagramDocuments::default();

    let first = store::get_or_load(&mut documents, &path).model.clone();
    let second = store::get_or_load(&mut documents, &path).model.clone();

    assert!(Rc::ptr_eq(&first, &second));
}

#[test]
fn reload_after_an_external_change_replaces_the_entry_and_tells_the_sessions() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "crlf-line-endings.ttl");
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, &path);
    fs::write(&path, "@prefix ex: <http://example.org/> .\r\nex:x ex:y ex:z .\r\n").unwrap();

    let change = documents.reconcile(&path);

    assert_eq!(change, DiskChange::Reloaded(path.clone()));
    assert!(documents.take_changed().contains(&path));
    let model = store::get_or_load(&mut documents, &path).model.clone();
    assert_eq!(model.triples.len(), 1);
    assert_eq!(model.triples[0].subject.as_iri(), Some("http://example.org/x"));
}

#[test]
fn forgetting_a_document_makes_the_next_open_read_afresh() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "crlf-line-endings.ttl");
    let mut documents = DiagramDocuments::default();
    let before = store::get_or_load(&mut documents, &path).model.clone();
    fs::write(&path, "@prefix ex: <http://example.org/> .\r\nex:x ex:y ex:z .\r\n").unwrap();

    adp_diagram::ModuleDocuments::close(store::store(&mut documents), &path);
    let after = store::get_or_load(&mut documents, &path).model.clone();

    assert!(!Rc::ptr_eq(&before, &after));
    assert_eq!(after.triples.len(), 1);
    assert_eq!(after.triples[0].subject.as_iri(), Some("http://example.org/x"));
}

#[test]
fn a_save_after_a_reload_writes_the_edit_rather_than_the_file_it_just_reread() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "constructs.ttl");
    let mut documents = DiagramDocuments::default();
    let mut edited = LineDocument::parse(&store::get_or_load(&mut documents, &path).text);
    edited.insert(0, &["# lost-edit marker".to_owned()]);
    assert!(edited.text().contains("# lost-edit marker"));

    // The notification of an earlier write of this same file lands now.
    documents.reconcile(&path);
    assert_eq!(store::save(&mut documents, &path, &edited.text()), Ok(()));

    assert!(fs::read_to_string(&path).unwrap().contains("# lost-edit marker"));
}

#[cfg(unix)]
#[test]
fn an_unreadable_body_is_kept_on_reload_and_tells_nobody() {
    use std::os::unix::fs::PermissionsExt;

    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "constructs.ttl");
    let mut documents = DiagramDocuments::default();
    assert!(!store::get_or_load(&mut documents, &path).model.triples.is_empty());
    documents.take_changed();

    // Another program holds the file so that it cannot be read.
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read(&path).is_ok();
    let change = documents.reconcile(&path);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    if readable {
        // Running with privileges that read anything: the case cannot be arranged.
        return;
    }

    assert_eq!(change, DiskChange::Unchanged);
    assert!(documents.take_changed().is_empty());
    assert!(!store::get(&documents, &path).unwrap().model.triples.is_empty());
}

#[test]
fn every_reading_of_the_family_reloads_through_the_one_store() {
    let folder = tempfile::tempdir().unwrap();
    let path = copy_fixture(&folder, "constructs.ttl");
    let mut documents = DiagramDocuments::default();
    store::get_or_load(&mut documents, &path);

    for reading in ["rdf", "owl", "skos", "shacl"] {
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str(&format!(
            "<http://example.org/{reading}> <http://example.org/y> <http://example.org/z> .\r\n"
        ));
        fs::write(&path, &text).unwrap();
        documents.take_changed();

        assert_eq!(documents.reconcile(&path), DiskChange::Reloaded(path.clone()));
        assert_eq!(documents.take_changed(), std::slice::from_ref(&path));
        let model = store::get(&documents, &path).unwrap().model.clone();
        assert!(model.names(&format!("http://example.org/{reading}")));
    }
}

#[test]
fn a_new_document_parses_and_validates_clean() {
    let text = adp_diagram_rdf::empty_rdf_document("My Graph");
    let model = parser::parse(&LineDocument::parse(&text)).unwrap();

    assert!(!model.triples.is_empty());
    assert!(validator::judge(&model).is_empty());
    assert!(
        model
            .triples
            .iter()
            .any(|t| t.object.as_literal().is_some_and(|l| l.lexical == "My Graph"))
    );
}

#[test]
fn a_hostile_base_name_folds_to_a_clean_local_name() {
    let text = adp_diagram_rdf::empty_rdf_document("weird / name?!");
    let model = parser::parse(&LineDocument::parse(&text)).unwrap();

    assert!(
        model
            .triples
            .iter()
            .any(|t| t.subject.is_iri("http://example.org/weird___name"))
    );
}

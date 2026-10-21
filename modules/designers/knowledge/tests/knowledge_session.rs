//! One open knowledge file on one connection (the standalone host's `KnowledgeSession.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirements 10.5, 2.3 and 8.1): the table model it gives, that opening writes nothing, and
//! that it stays true to the file. The standalone host's tests of its own file watcher are not ported - the session
//! here is told of a change by the core, which owns the one watcher - and a change from outside is given to it
//! through `reload`, as the watcher would ask.

mod common;

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_designer_knowledge::body::ANOTHER_VERSION;
use adp_designer_knowledge::designer::{self, KnowledgeSessionFactory};
use adp_designer_knowledge::store::{self, KnowledgeRead};
use adp_designer_knowledge::{KnowledgeBody, KnowledgeSession};
use adp_desl::table::model::{TableChange, TableGesture};
use adp_desl::table::session::DesignerSession;
use adp_desl::{DesignerFormat, DesignerSessionFactory};
use common::{EXTENSIONS, RELATED, copy_example, copy_plain};

// ---- the table model ----

#[test]
fn the_example_opens_as_the_expected_table_model() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let path = copy_example(&format!("cities{extension}"), folder.path());
        let baseline = KnowledgeSession::new(&path).baseline();

        // The table, editable, with nothing to report.
        assert_eq!("Cities", baseline.title);
        assert_eq!("", baseline.read_only_reason);
        assert!(baseline.findings.is_empty(), "{extension}: {:?}", baseline.findings);

        // Every property is a column, of its type's kind, the one that names a row marked and shown.
        let mut first: Vec<_> = baseline
            .columns
            .iter()
            .filter(|c| ["p1", "p2", "p3", "p4", "p5", "p6"].contains(&c.id.as_str()))
            .collect();
        first.sort_by_key(|c| c.id.clone());
        assert_eq!(
            vec![
                "Name:text",
                "Country:selection",
                "Population:number",
                "Tags:multipleSelection",
                "Province:relation",
                "Visited:checkbox"
            ],
            first
                .iter()
                .map(|c| format!("{}:{}", c.name, c.kind))
                .collect::<Vec<_>>()
        );
        let titles: Vec<_> = baseline.columns.iter().filter(|c| c.is_title).collect();
        assert_eq!(1, titles.len());
        assert_eq!("p1", titles[0].id);
        assert!(titles[0].visible);

        // A selection carries its options and their colours; a relation says where it points.
        let country = baseline.columns.iter().find(|c| c.id == "p2").expect("p2");
        assert_eq!(
            vec![
                "Netherlands:blue",
                "Belgium:yellow",
                "Germany:default",
                "France:default",
                "Finland:default",
                "Italy:default"
            ],
            country
                .options
                .iter()
                .map(|o| format!("{}:{}", o.name, o.color))
                .collect::<Vec<_>>()
        );
        let province = baseline.columns.iter().find(|c| c.id == "p5").expect("p5");
        assert_eq!(
            ("provinces.yaml", "one"),
            (
                province.settings["targetFile"].as_str(),
                province.settings["limit"].as_str()
            )
        );

        // The views, and the settings of the one the file was left in.
        assert!(!baseline.views.is_empty());
        assert_eq!("v2", baseline.settings.view_id);

        // The lines of that view, which filters, sorts and groups nothing: the eleven cities, and the line a new row
        // is added at.
        assert_eq!("", baseline.settings.group_by);
        assert_eq!(11 + 1, baseline.row_count);
    }
}

fn rows_of(change: &TableChange) -> (usize, &Vec<adp_desl::table::model::TableRow>, usize) {
    match change {
        TableChange::Rows { first, rows, row_count } => (*first, rows, *row_count),
        other => panic!("not rows: {other:?}"),
    }
}

#[test]
fn the_window_is_the_lines_asked_for_with_the_new_row_line_at_the_bottom() {
    let folder = common::folder();
    let path = copy_plain("cities.yaml", folder.path());
    let mut session = KnowledgeSession::new(&path);
    let total = session.baseline().row_count;

    // Everything, then two lines from the second.
    session.set_window(0, 1000);
    session.set_window(1, 2);
    let pushed = session.take_changes();

    let (first, all, count) = rows_of(&pushed[0]);
    assert_eq!((0, total, total), (first, all.len(), count));
    assert!(all.iter().take(total - 1).all(|r| !r.is_new_row));
    assert!(all[total - 1].is_new_row);
    assert_eq!("", all[total - 1].id);

    // A row's cells are the ones that hold a value, by column; several values stay several.
    assert_eq!(vec!["Amsterdam"], all[0].cell("p1").expect("a name").values);
    assert!(all.iter().any(|r| r.cell("p4").is_some_and(|c| c.values.len() > 1)));

    let (first, part, count) = rows_of(&pushed[1]);
    assert_eq!((1, 2, total), (first, part.len(), count));
    assert_eq!(
        all.iter().skip(1).take(2).map(|r| &r.id).collect::<Vec<_>>(),
        part.iter().map(|r| &r.id).collect::<Vec<_>>()
    );
}

#[test]
fn a_window_past_the_end_is_empty_and_never_fails() {
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let mut session = KnowledgeSession::new(&path);
    session.set_window(5000, 50);
    session.set_window(usize::MAX, usize::MAX);
    let pushed = session.take_changes();
    assert!(rows_of(&pushed[0]).1.is_empty());
    assert!(rows_of(&pushed[1]).1.is_empty());
}

#[test]
fn another_view_is_this_connections_own() {
    // Two connections on one file.
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let mut one = KnowledgeSession::new(&path);
    let other = KnowledgeSession::new(&path);
    let views = one.baseline().views;
    assert!(views.len() > 1, "The example needs a second view for this test.");
    let left = one.baseline().settings.view_id;
    let another = views.iter().find(|v| v.id != left).expect("another view").id.clone();

    one.set_active_view(&another);
    one.set_active_view("no-such-view");

    // The structure for that view, once; the other connection is where it was, in the view the file was left in.
    let pushed = one.take_changes();
    let [TableChange::Structure(structure)] = pushed.as_slice() else {
        panic!("{pushed:?}")
    };
    assert_eq!(another, structure.settings.view_id);
    assert_eq!(another, one.baseline().settings.view_id);
    assert_eq!(left, other.baseline().settings.view_id);
}

// ---- opening writes nothing ----

#[test]
fn opening_and_looking_writes_nothing() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let name = format!("cities{extension}");
        let path = copy_example(&name, folder.path());
        let before = std::fs::read(&path).expect("read");
        let stamp = std::fs::metadata(&path).and_then(|m| m.modified()).expect("a time");

        // Everything a connection does without editing.
        {
            let mut session = KnowledgeSession::new(&path);
            session.baseline();
            session.set_window(0, 100);
            let last = session.baseline().views.last().expect("a view").id.clone();
            session.set_active_view(&last);
            session.reload();
        }

        // The same bytes, not rewritten, and nothing beside the file but the table its relation names.
        assert_eq!(before, std::fs::read(&path).expect("read"));
        assert_eq!(
            stamp,
            std::fs::metadata(&path).and_then(|m| m.modified()).expect("a time")
        );
        let mut entries: Vec<String> = std::fs::read_dir(folder.path())
            .expect("the folder")
            .map(|e| e.expect("an entry").file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        assert_eq!(vec![name, RELATED.to_owned()], entries);
    }
}

// ---- staying true to the file ----

fn titles(changes: &[TableChange]) -> Vec<String> {
    changes
        .iter()
        .filter_map(|c| match c {
            TableChange::Structure(s) => Some(s.title.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_change_made_outside_adp_is_shown_without_reopening() {
    let folder = common::folder();
    let path = copy_plain("cities.yaml", folder.path());
    let mut session = KnowledgeSession::new(&path);
    session.set_window(0, 100);
    session.take_changes();
    let text = std::fs::read_to_string(&path).expect("read");

    // Another program renames the table and the first city.
    std::fs::write(
        &path,
        text.replace("name: Cities", "name: Towns")
            .replace("text: Amsterdam", "text: Mokum"),
    )
    .expect("written");
    session.reload();

    // The structure, the findings and the window's rows, from the file as it is now.
    let pushed = session.take_changes();
    assert_eq!(vec!["Towns"], titles(&pushed));
    let rows = pushed.iter().rev().find_map(|c| match c {
        TableChange::Rows { rows, .. } => Some(rows),
        _ => None,
    });
    assert_eq!(vec!["Mokum"], rows.expect("rows")[0].cell("p1").expect("a name").values);
    assert!(pushed.iter().any(|c| matches!(c, TableChange::Findings(_))));
}

#[test]
fn a_write_that_changes_nothing_pushes_nothing() {
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let mut session = KnowledgeSession::new(&path);
    session.set_window(0, 100);
    session.take_changes();

    // Read again, as the watcher would ask after a save of the same bytes.
    session.reload();
    assert!(session.take_changes().is_empty());
}

#[test]
fn a_deletion_is_learned() {
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let mut session = KnowledgeSession::new(&path);
    assert_eq!("", session.refusal());

    std::fs::remove_file(&path).expect("deleted");
    session.reload();

    // The session stops claiming the file opens and says why, and keeps the last good table.
    assert_eq!("The file no longer exists.", session.refusal());
    assert_eq!("Cities", session.baseline().title);
}

#[test]
fn a_change_lost_while_nothing_watched_is_learned_by_reading_the_file_again() {
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let mut session = KnowledgeSession::new(&path);
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(&path, text.replace("name: Cities", "name: Towns")).expect("written");

    // What the core does when its watcher says events were lost.
    session.reload();
    assert_eq!(vec!["Towns"], titles(&session.take_changes()));
    assert_eq!("Towns", session.baseline().title);
}

// ---- the read that is refused ----

/// A read that refuses as many times as it is told to, as a file mid-replace does, and counts how often it was
/// asked. Only the table itself is scripted: the table its relation points at is read as it is.
struct ScriptedRead {
    refusal: &'static str,
    to_refuse: Cell<usize>,
    reads: Cell<usize>,
}

fn scripted(path: &Path, script: &Rc<ScriptedRead>) -> KnowledgeSession {
    let reader = script.clone();
    KnowledgeSession::with(
        path,
        Box::new(move |at: &Path| {
            if at.file_name().is_some_and(|n| n == RELATED) {
                return store::read(at);
            }
            reader.reads.set(reader.reads.get() + 1);
            if reader.to_refuse.get() > 0 {
                reader.to_refuse.set(reader.to_refuse.get() - 1);
                return KnowledgeRead {
                    body: None,
                    refusal: reader.refusal.into(),
                };
            }
            store::read(at)
        }),
        None,
        PathBuf::new(),
    )
}

#[test]
fn a_refused_re_read_is_tried_again_so_the_change_is_not_lost() {
    for refusals in [1, 2] {
        let folder = common::folder();
        let path = copy_example("cities.yaml", folder.path());
        let script = Rc::new(ScriptedRead {
            refusal: "The file no longer exists.",
            to_refuse: Cell::new(0),
            reads: Cell::new(0),
        });
        let mut session = scripted(&path, &script);
        session.take_changes();
        let text = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, text.replace("name: Cities", "name: Towns")).expect("written");
        script.reads.set(0);

        script.to_refuse.set(refusals);
        session.reload();

        // Read until it opened, and the change arrived.
        assert_eq!(refusals + 1, script.reads.get());
        assert_eq!("", session.refusal());
        assert_eq!(vec!["Towns"], titles(&session.take_changes()));
    }
}

#[test]
fn a_read_that_keeps_being_refused_is_given_up_and_the_last_table_stays() {
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let script = Rc::new(ScriptedRead {
        refusal: "The file cannot be read: it is in use.",
        to_refuse: Cell::new(0),
        reads: Cell::new(0),
    });
    let mut session = scripted(&path, &script);
    session.take_changes();
    script.reads.set(0);

    script.to_refuse.set(usize::MAX);
    session.reload();

    // Three attempts, then the refusal is kept and nothing is given over the last good table.
    assert_eq!(3, script.reads.get());
    assert_eq!("The file cannot be read: it is in use.", session.refusal());
    assert!(session.take_changes().is_empty());
    assert_eq!("Cities", session.baseline().title);
}

// ---- what cannot be edited ----

#[test]
fn a_file_that_cannot_be_read_is_its_reason_refuses_every_edit_and_is_not_written() {
    let folder = common::folder();
    let path = folder.path().join("broken.yaml");
    std::fs::write(&path, "ded: \"0.1\"\ndesigner: etalii/knowledge\nname: [unclosed\n").expect("written");
    let before = std::fs::read(&path).expect("read");
    let mut session = KnowledgeSession::open_for_editing(&path, PathBuf::new());

    let baseline = session.baseline();
    let answers: Vec<String> = ["setCell", "addRow", "renameColumn", "addView", "deleteColumn"]
        .iter()
        .map(|kind| session.edit("e", &TableGesture::new(*kind)))
        .collect();

    // Shown as its reason with its position, no line to add a row at, and every edit refused with that reason.
    assert!(
        baseline.read_only_reason.starts_with("This file cannot be read: "),
        "{}",
        baseline.read_only_reason
    );
    assert!(baseline.columns.is_empty());
    assert_eq!(0, baseline.row_count);
    assert!(answers.iter().all(|a| *a == baseline.read_only_reason), "{answers:?}");
    assert_eq!(before, std::fs::read(&path).expect("read"));
}

#[test]
fn a_file_of_another_version_refuses_every_edit_and_is_not_written() {
    let folder = common::folder();
    let path = copy_example("cities.yaml", folder.path());
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(&path, text.replace("ded: \"0.1\"", "ded: \"9.0\"")).expect("written");
    let before = std::fs::read(&path).expect("read");
    let mut session = KnowledgeSession::open_for_editing(&path, PathBuf::new());

    let answers: Vec<String> = ["setCell", "addRow", "renameColumn"]
        .iter()
        .map(|kind| session.edit("e", &TableGesture::new(*kind)))
        .collect();

    assert_eq!(ANOTHER_VERSION, session.baseline().read_only_reason);
    assert!(answers.iter().all(|a| a == ANOTHER_VERSION));
    assert_eq!(before, std::fs::read(&path).expect("read"));
}

#[test]
fn a_file_that_is_not_there_is_its_refusal() {
    let folder = common::folder();
    let mut session = KnowledgeSession::new(folder.path().join("missing.yaml"));
    assert_eq!("The file no longer exists.", session.baseline().read_only_reason);
    assert_eq!(
        "The file no longer exists.",
        session.edit("e", &TableGesture::new("addRow"))
    );
}

// ---- the module's registrations ----

#[test]
fn a_new_file_is_one_title_property_one_view_and_no_rows_with_ids_of_its_own() {
    let short_guid = regex::Regex::new("^[0-9a-z]{25}$").expect("the pattern compiles");
    for extension in EXTENSIONS {
        let format = designer::definition()
            .formats
            .into_iter()
            .find(|f| f.extension == extension)
            .expect("the format");
        let name = format!("Suppliers{extension}");

        // Twice, because an id is never reused.
        let first = KnowledgeBody::read(
            designer::new_document(&format, &name).expect("made").into_bytes(),
            &name,
        );
        let second = KnowledgeBody::read(
            designer::new_document(&format, &name).expect("made").into_bytes(),
            &name,
        );

        assert_eq!("", first.unreadable(), "{extension}");
        assert!(
            first.model().findings.is_empty(),
            "{extension}: {:?}",
            first.model().findings
        );
        assert_eq!("Suppliers", first.table().name);
        let [property] = first.table().columns.as_slice() else {
            panic!("one property")
        };
        assert!(property.is_title);
        assert_eq!("text", property.value_type);
        let [view] = first.table().views.as_slice() else {
            panic!("one view")
        };
        assert!(first.table().rows.is_empty());

        // The ids are ShortGuids: twenty-five characters of base 36, and different every time.
        assert!(short_guid.is_match(&property.id), "{}", property.id);
        assert!(short_guid.is_match(&view.id), "{}", view.id);
        assert_ne!(property.id, view.id);
        assert_ne!(property.id, second.table().columns[0].id);
    }
}

#[test]
fn the_definition_offers_the_three_formats_and_serves_its_own_origin() {
    let definition = designer::definition();
    assert_eq!("etalii/knowledge", definition.origin);
    assert_eq!(
        vec![".yaml", ".json", ".xml"],
        definition
            .formats
            .iter()
            .map(|f| f.extension.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(definition.origin, KnowledgeSessionFactory.origin());
    assert_eq!(
        None,
        designer::new_document(&DesignerFormat::new("TOML", ".toml"), "x.toml")
    );
}

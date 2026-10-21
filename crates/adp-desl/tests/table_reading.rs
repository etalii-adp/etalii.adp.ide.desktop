//! A `table` surface read by its DESL roles alone, and shown through a windowed session: DESL's minimal table and the
//! vendored Knowledge examples, opened through FBL with the bindings their specifications name.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_desl::table::content::{ContentView, FilterGroup, FilterItem, ViewSort};
use adp_desl::table::engine::{self, LineOptions};
use adp_desl::table::model::{TableChange, TableGesture};
use adp_desl::table::reading;
use adp_desl::table::session::{ContentSession, DesignerSession, NO_WRITER};
use adp_desl::{DeslLoader, DeslSpecification, Editor};
use adp_fbl::{FblOptions, OpenBody};

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn specification(relative: &str) -> (DeslSpecification, PathBuf) {
    let path = vendor().join(relative);
    let loaded = DeslLoader::load_file(&path);
    (loaded.specification.expect("it loads"), path)
}

/// A body read through the binding of the specification that `reference` names.
fn open(specification: &Path, reference: &str, bytes: &[u8], name: &str) -> adp_fbl::FblModel {
    let (binding, _) = adp_fbl::documents::resolve_reference(reference, specification).expect("the binding");
    let options = FblOptions {
        file_name: name.into(),
        derive_id: Some(Rc::new(reading::structural_id)),
        ..FblOptions::default()
    };
    OpenBody::open(bytes.to_vec(), Rc::new(binding), Rc::new(options)).model()
}

const READING_LIST: &str = "ded: \"0.1\"
designer: example/reading-list
fields:
  - id: f1
    name: Title
    type: text
  - id: f2
    name: Pages
    type: number
views:
  - id: v1
    name: All books
books:
  - id: b1
    values:
      - field: f1
        text: Thinking in Systems
      - field: f2
        number: 240
  - id: b2
    values:
      - field: f1
        text: The Structure of Scientific Revolutions
      - field: f2
        number: 9
  - id: b3
    values:
      - field: f1
        text: Gödel, Escher, Bach
";

#[test]
fn the_minimal_table_is_read_by_its_roles() {
    let (specification, path) = specification("specifications/desl/minimal-table.des");
    let model = open(&path, "minimal-table.fbl#yaml", READING_LIST.as_bytes(), "books.yaml");
    assert!(model.findings.is_empty(), "{:?}", model.findings);
    let content = reading::read(&specification, &model).expect("a table");

    let columns: Vec<String> = content
        .columns
        .iter()
        .map(|c| format!("{}:{}", c.name, c.value_type))
        .collect();
    assert_eq!(vec!["Title:text", "Pages:number"], columns);
    assert_eq!(Editor::Number, content.columns[1].kind.editor);
    assert_eq!(
        vec!["All books"],
        content.views.iter().map(|v| v.name.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(3, content.rows.len());
    assert_eq!(vec!["240"], content.rows[0].cell("f2").expect("pages").values);
    assert!(content.rows[2].cell("f2").is_none());
}

#[test]
fn a_view_of_the_minimal_table_sorts_by_number_as_the_value_type_says() {
    let (specification, path) = specification("specifications/desl/minimal-table.des");
    let model = open(&path, "minimal-table.fbl#yaml", READING_LIST.as_bytes(), "books.yaml");
    let content = reading::read(&specification, &model).expect("a table");
    let view = ContentView {
        sorts: vec![ViewSort {
            column_id: "f2".into(),
            descending: false,
        }],
        filter: FilterGroup::all(vec![FilterItem::Condition(adp_desl::table::content::Condition::new(
            "f1", "contains", "s",
        ))]),
        ..ContentView::new("v1", "All books")
    };
    let lines = engine::lines(&content, Some(&view), &LineOptions::default());
    // 9 before 240 as numbers, the book without pages last; every title has an s.
    assert_eq!(
        vec!["b2", "b1", "b3"],
        lines.iter().map(|l| l.id.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn a_session_over_content_answers_its_window_searches_and_refuses_to_write() {
    let (specification, path) = specification("specifications/desl/minimal-table.des");
    let model = open(&path, "minimal-table.fbl#yaml", READING_LIST.as_bytes(), "books.yaml");
    let content = reading::read(&specification, &model).expect("a table");
    let mut session = ContentSession::new(content, Vec::new(), "");

    let baseline = session.baseline();
    assert_eq!(3, baseline.row_count);
    assert_eq!("v1", baseline.settings.view_id);
    session.set_window(1, 5);
    let changes = session.take_changes();
    let [TableChange::Rows { first, rows, row_count }] = changes.as_slice() else {
        panic!("{changes:?}")
    };
    assert_eq!((1, 2, 3), (*first, rows.len(), *row_count));

    assert_eq!("", session.edit("e1", &TableGesture::new("search").values(["godel"])));
    assert_eq!(0, session.baseline().row_count, "a search ignores case, not accents");
    assert_eq!("", session.edit("e2", &TableGesture::new("search").values(["gödel"])));
    assert_eq!(1, session.baseline().row_count);
    assert_eq!(NO_WRITER, session.edit("e3", &TableGesture::new("setCell").row("b1")));
}

#[test]
fn the_vendored_knowledge_examples_are_read_alike_by_the_roles_alone() {
    let (specification, path) = specification("definitions/designers/knowledge.des");
    let mut shapes = Vec::new();
    for (name, binding) in [
        ("cities.yaml", "knowledge.fbl#yaml"),
        ("cities.json", "knowledge.fbl#json"),
        ("cities.xml", "knowledge.fbl#xml"),
    ] {
        let bytes = std::fs::read(vendor().join("definitions/designers/examples").join(name)).expect("vendored");
        let model = open(&path, binding, &bytes, name);
        let content = reading::read(&specification, &model).expect("a table");
        assert_eq!("Cities", content.name, "{name}");
        assert_eq!("v1", content.active_view_id);
        assert_eq!(9, content.columns.len());
        assert_eq!(6, content.columns[1].options.len());
        assert_eq!(9, content.rows.len());
        // Several values are items: the first city's tags.
        assert_eq!(vec!["o3", "o4"], content.rows[0].cell("p4").expect("tags").values);
        let shape: Vec<String> = content
            .rows
            .iter()
            .map(|r| {
                r.cells
                    .iter()
                    .map(|c| format!("{}={}:{}", c.column_id, c.key, c.values.join("|")))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        shapes.push(shape);
    }
    assert_eq!(shapes[0], shapes[1]);
    assert_eq!(shapes[0], shapes[2]);
}

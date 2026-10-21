//! Loading DESL specifications (DESL section 10.1): every vendored specification loads, a later major version is
//! refused with the reason and a later minor read with a warning, and what the schema and the names refuse is
//! reported at its pointer.

use std::path::{Path, PathBuf};

use adp_desl::{DeslLoader, DeslSeverity, DeslSurface, Editor, SortKind};

fn vendor() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp")
}

fn specifications() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for folder in ["specifications/desl", "definitions/designers"] {
        for entry in std::fs::read_dir(vendor().join(folder)).expect("the folder is vendored") {
            let path = entry.expect("an entry").path();
            if path.extension().is_some_and(|e| e == "des") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn text(relative: &str) -> String {
    std::fs::read_to_string(vendor().join(relative)).expect("the file is vendored")
}

/// The minimal table with its JSON edited.
fn minimal_with(edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(&text("specifications/desl/minimal-table.des")).expect("JSON");
    edit(&mut value);
    serde_json::to_string(&value).expect("JSON")
}

#[test]
fn every_vendored_specification_loads() {
    let all = specifications();
    assert_eq!(4, all.len(), "{all:?}");
    for path in all {
        let result = DeslLoader::load_file(&path);
        assert_eq!(
            0,
            result.errors().count(),
            "{}: {:#?}",
            path.display(),
            result.diagnostics
        );
        assert!(result.specification.is_some());
    }
}

#[test]
fn the_knowledge_specification_and_the_minimal_table_load_with_nothing_to_report() {
    for relative in [
        "definitions/designers/knowledge.des",
        "specifications/desl/minimal-table.des",
    ] {
        let result = DeslLoader::load_file(&vendor().join(relative));
        assert!(result.diagnostics.is_empty(), "{relative}: {:#?}", result.diagnostics);
    }
}

#[test]
fn a_binding_the_fbl_runtime_cannot_read_is_a_warning_and_the_specification_loads() {
    // The Hierarchical Requirement Designer's binding is of FBL 0.7, whose bullets family this FBL runtime lacks.
    let result = DeslLoader::load_file(&vendor().join("definitions/designers/hierarchical-requirement.des"));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|d| d.severity == DeslSeverity::Warning && d.pointer == "/persistence/bindings/0"),
        "{:#?}",
        result.diagnostics
    );
    assert!(matches!(
        result.specification.expect("loaded").surface,
        DeslSurface::Hierarchy(_)
    ));
}

#[test]
fn the_knowledge_specification_is_read_into_its_table() {
    let specification = DeslLoader::load_file(&vendor().join("definitions/designers/knowledge.des"))
        .specification
        .expect("it loads");
    assert_eq!("etalii/knowledge", specification.language.origin);
    assert_eq!("Knowledge", specification.language.label);
    assert_eq!("0.1", specification.desl_version());
    let table = specification.table().expect("a table");
    assert_eq!(
        ("Property", "Row", "Cell", "View"),
        (
            table.columns_type.as_str(),
            table.rows_type.as_str(),
            table.cells_type.as_str(),
            table.views_type.as_str()
        )
    );
    assert_eq!(Some("CellItem"), table.cell_items.as_deref());
    assert_eq!(9, table.value_types.len());
    let relation = specification.value_type("relation").expect("relation");
    assert_eq!(
        (Editor::Rows, SortKind::Title, true, "row"),
        (relation.editor, relation.sort, relation.many, relation.key.as_str())
    );
    assert_eq!(Some("number"), specification.attribute_type("Cell", "number"));
    assert!(specification.typed_attribute_count() > 20);
    assert_eq!(
        Some("Multiple selection"),
        specification.enum_label("ValueType", "multipleSelection")
    );
    assert_eq!("derived", specification.id_rule("Cell").strategy);
    assert_eq!("uuid-v4", specification.id_rule("Row").strategy);
}

#[test]
fn a_later_major_version_is_refused_with_the_reason() {
    let result = DeslLoader::load(&minimal_with(|v| v["desl"] = "1.0".into()));
    assert!(result.specification.is_none());
    let error = result.errors().next().expect("an error");
    assert_eq!("/desl", error.pointer);
    assert_eq!(
        "This runtime reads DESL 0.x; DESL 1.0 is refused (DESL §2.7).",
        error.message
    );
}

#[test]
fn a_later_minor_version_is_read_with_a_warning() {
    let result = DeslLoader::load(&minimal_with(|v| v["desl"] = "0.9".into()));
    assert!(result.specification.is_some(), "{:#?}", result.diagnostics);
    let [warning] = result.diagnostics.as_slice() else {
        panic!("{:#?}", result.diagnostics)
    };
    assert_eq!(
        (DeslSeverity::Warning, "/desl"),
        (warning.severity, warning.pointer.as_str())
    );
}

#[test]
fn a_version_not_written_as_major_minor_is_refused() {
    for version in ["0", "0.1.0", "zero.one", ""] {
        let result = DeslLoader::load(&minimal_with(|v| v["desl"] = version.into()));
        assert!(result.specification.is_none(), "{version}");
        assert_eq!("/desl", result.errors().next().expect("an error").pointer);
    }
}

#[test]
fn a_byte_order_mark_and_a_repeated_key_are_refused() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text("specifications/desl/minimal-table.des").as_bytes());
    assert!(DeslLoader::load_bytes(&bytes, None).specification.is_none());

    let repeated = text("specifications/desl/minimal-table.des").replacen(
        "\"desl\": \"0.1\",",
        "\"desl\": \"0.1\", \"desl\": \"0.1\",",
        1,
    );
    let result = DeslLoader::load(&repeated);
    assert!(result.specification.is_none());
    assert!(
        result
            .errors()
            .any(|e| e.pointer == "/desl" && e.message.contains("more than once"))
    );
}

#[test]
fn what_the_schema_refuses_is_reported_at_its_pointer() {
    let result = DeslLoader::load(&minimal_with(|v| {
        v["surface"]["valueTypes"]["text"]["editor"] = "spreadsheet".into();
    }));
    assert!(result.specification.is_none());
    assert!(
        result
            .errors()
            .any(|e| e.pointer.starts_with("/surface") && e.message.ends_with("(desl.schema.json)")),
        "{:#?}",
        result.diagnostics
    );
}

#[test]
fn a_name_the_surface_uses_that_the_metamodel_lacks_is_refused() {
    let result = DeslLoader::load(&minimal_with(|v| v["surface"]["columns"]["name"] = "title".into()));
    assert!(result.specification.is_none());
    let error = result.errors().next().expect("an error");
    assert_eq!("/surface/columns/name", error.pointer);
    assert!(error.message.contains("'title'"), "{}", error.message);

    let result = DeslLoader::load(&minimal_with(|v| {
        v["surface"]["valueTypes"]["number"]["key"] = "amount".into()
    }));
    assert_eq!(
        "/surface/valueTypes/number/key",
        result.errors().next().expect("an error").pointer
    );
}

#[test]
fn a_reserved_attribute_name_is_refused() {
    let result = DeslLoader::load(&minimal_with(|v| {
        v["metamodel"]["types"]["Book"]["attributes"] = serde_json::json!({ "parent": { "type": "string" } });
    }));
    assert!(result.specification.is_none());
    assert!(
        result
            .errors()
            .any(|e| e.pointer.starts_with("/metamodel/types/Book/attributes")),
        "{:#?}",
        result.diagnostics
    );
}

#[test]
fn a_binding_that_is_not_there_is_an_error() {
    let folder = vendor().join("specifications/desl");
    let json = minimal_with(|v| v["persistence"]["bindings"] = serde_json::json!(["nowhere.fbl#yaml"]));
    let result = DeslLoader::load_bytes(json.as_bytes(), Some(&folder.join("minimal-table.des")));
    assert!(result.specification.is_none());
    assert_eq!(
        "/persistence/bindings/0",
        result.errors().next().expect("an error").pointer
    );
}

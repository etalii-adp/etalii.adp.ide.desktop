//! The blocks of a registration a binding declares, and those it does not (FBL 0.7 sections 8.1 and 8.4), and the
//! registration created beside a body on the first placement or setting (FBL section 8.4, 0.7 section 8.4): the
//! standalone host's `Registration/DeclaredBlocks.Tests.cs` and `Registration/RegistrationCreation.Tests.cs`, over
//! the binding of the `registration-blocks` fixture.

use std::path::{Path, PathBuf};

use super::{OpenRegistration, create_registration};
use crate::documents::types::RegistrationBlockDeclaration;
use crate::documents::{self, FblBinding, ProblemSeverity};
use crate::history::UndoResult;
use crate::model::{FindingSeverity, codes};
use crate::planning::{ModelChange, PlanResult, Setting};
use crate::splice::{Operation, Splice};

fn vendor(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../vendor/etalii.adp")
        .join(path)
}

/// The binding of the `registration-blocks` fixture: `tags` kept, `collapsed` ordinal by ids outside the history,
/// `filters` kept; `createOnFirstPlacement` is true.
fn tasks() -> FblBinding {
    let fixture = vendor("specifications/fbl/fixtures/registration-blocks/fixture.json");
    let (binding, problems) = documents::resolve_reference("tasks.fbl#tasks", &fixture).unwrap();
    assert!(
        !problems.iter().any(|p| p.severity == ProblemSeverity::Error),
        "{problems:?}"
    );
    binding
}

fn open(text: &str) -> OpenRegistration {
    OpenRegistration::open_with(text.as_bytes().to_vec(), &tasks())
}

fn setting(block: &str, key: &str) -> Setting {
    Setting {
        block: block.into(),
        key: key.into(),
        ..Setting::default()
    }
}

fn with_value(block: &str, key: &str, value: &str) -> ModelChange {
    ModelChange::Setting(Setting {
        value: Some(value.into()),
        ..setting(block, key)
    })
}

fn declaration(name: &str, kept: bool, keyed_by_ids: bool, in_history: bool) -> RegistrationBlockDeclaration {
    RegistrationBlockDeclaration {
        name: name.into(),
        kept,
        keyed_by_ids,
        in_history,
    }
}

#[test]
fn the_bindings_blocks_are_read() {
    assert_eq!(
        tasks().registration.blocks,
        vec![
            declaration("tags", true, false, true),
            declaration("collapsed", false, true, false),
            declaration("filters", true, false, true),
        ]
    );
}

#[test]
fn reading_goes_on_past_an_undeclared_block_keeps_its_bytes_and_reports_it_once_at_information() {
    // notes is declared by no one, tags after it is the binding's.
    let mut registration = open("example/tasks\r\nnotes:\r\n  first: keep me\r\ntags:\r\n  security: red\r\n");

    let findings = registration.findings("plan.adp");

    // The declared block after it is read.
    let tags = registration.document().block("tags").expect("the tags block");
    assert_eq!(tags.entries.len(), 1);
    assert_eq!(tags.entries[0].value, "red");
    let unknown: Vec<_> = findings.iter().filter(|f| f.code == codes::UNKNOWN_BLOCK).collect();
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].severity, FindingSeverity::Info);
    let location = unknown[0].location.as_ref().unwrap();
    assert_eq!(
        (location.file.as_str(), location.line, location.column),
        ("plan.adp", 2, 1)
    );

    // A setting written after it keeps the undeclared block's bytes.
    let result = registration.change(&with_value("tags", "core", "default"));
    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert_eq!(
        registration.bytes(),
        b"example/tasks\r\nnotes:\r\n  first: keep me\r\ntags:\r\n  security: red\r\n  core: default\r\n"
    );
}

#[test]
fn a_value_holding_a_colon_and_a_space_is_refused() {
    let text = "example/tasks\r\ntags:\r\n  security: red\r\n";
    let mut registration = open(text);

    let result = registration.change(&with_value("tags", "legal", "see: policy"));

    assert_eq!(
        result,
        PlanResult::Refused("A setting's value cannot hold ': '.".into())
    );
    assert_eq!(registration.bytes(), text.as_bytes());
}

#[test]
fn a_rename_to_a_key_the_block_has_is_refused() {
    let text = "example/tasks\r\ntags:\r\n  security: red\r\n  core: default\r\n";
    let mut registration = open(text);

    let result = registration.change(&ModelChange::Setting(Setting {
        rename: Some("security".into()),
        ..setting("tags", "core")
    }));

    let PlanResult::Refused(reason) = result else {
        panic!("planned: {result:?}");
    };
    assert!(reason.contains("'security'"), "{reason}");
    assert_eq!(registration.bytes(), text.as_bytes());
}

#[test]
fn a_rename_is_a_replace_value_of_the_key() {
    let mut registration = open("example/tasks\r\ntags:\r\n  security: red\r\n");

    let result = registration.change(&ModelChange::Setting(Setting {
        rename: Some("safety".into()),
        ..setting("tags", "security")
    }));

    let PlanResult::Planned(edit) = result else {
        panic!("refused: {result:?}");
    };
    assert_eq!(
        edit.splices,
        vec![Splice {
            operation: Operation::ReplaceValue,
            start: 24,
            end: 32,
            text: "safety".into(),
        }]
    );
    assert_eq!(registration.bytes(), b"example/tasks\r\ntags:\r\n  safety: red\r\n");
}

#[test]
fn removing_the_last_entry_removes_the_blocks_line_with_it() {
    let mut registration = open("example/tasks\r\ntags:\r\n  security: red\r\n");

    let result = registration.change(&ModelChange::Setting(Setting {
        remove_entry: true,
        ..setting("tags", "security")
    }));

    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert_eq!(registration.bytes(), b"example/tasks\r\n");
}

#[test]
fn a_setting_of_a_block_the_binding_does_not_declare_is_refused() {
    let mut registration = open("example/tasks\r\n");

    let result = registration.change(&with_value("notes", "first", "keep me"));

    assert!(matches!(result, PlanResult::Refused(_)), "{result:?}");
    assert_eq!(registration.bytes(), b"example/tasks\r\n");
}

#[test]
fn the_first_change_of_an_absent_block_whose_list_has_members_writes_them_all() {
    let mut registration = open("example/tasks\r\nbody: week.tasks\r\n");
    registration.absent_members = Some(std::rc::Rc::new(|block: &str| {
        (block == "tags").then(|| vec![("core".into(), "default".into()), ("ux".into(), "green".into())])
    }));

    let result = registration.change(&with_value("tags", "ux", "blue"));

    assert!(matches!(result, PlanResult::Planned(_)), "{result:?}");
    assert_eq!(
        registration.bytes(),
        b"example/tasks\r\nbody: week.tasks\r\ntags:\r\n  core: default\r\n  ux: blue\r\n"
    );
}

#[test]
fn a_registration_whose_base_name_differs_names_its_body_in_the_bodys_line_ending() {
    let created = create_registration(
        &tasks(),
        "example/tasks",
        "plan.adp",
        "week.tasks",
        b"task t1\r\ntask t2\r\n",
        None,
    );

    assert_eq!(created.unwrap(), b"example/tasks\r\nbody: week.tasks\r\n");
}

#[test]
fn a_registration_with_the_bodys_base_name_is_the_origin_line_alone() {
    let created = create_registration(
        &tasks(),
        "example/tasks",
        "week.adp",
        "week.tasks",
        b"task t1\ntask t2\r\ntask t3\n",
        None,
    );

    // LF, the body's dominant line ending.
    assert_eq!(created.unwrap(), b"example/tasks\n");
}

#[test]
fn the_created_registration_is_no_step_of_the_history() {
    // Created before the edit is planned.
    let created = create_registration(
        &tasks(),
        "example/tasks",
        "plan.adp",
        "week.tasks",
        b"task t1\r\n",
        None,
    )
    .unwrap();
    let mut registration = OpenRegistration::open_with(created.clone(), &tasks());
    assert!(matches!(
        registration.change(&with_value("tags", "security", "red")),
        PlanResult::Planned(_)
    ));

    let undone = registration.undo();

    // The undo takes the setting back and leaves the file as it was created.
    assert!(matches!(undone, UndoResult::Done(_)), "{undone:?}");
    assert_eq!(registration.bytes(), created.as_slice());
    assert!(!registration.can_undo());
}

#[test]
fn a_different_file_of_that_name_is_refused_with_a_reason() {
    let created = create_registration(
        &tasks(),
        "example/tasks",
        "plan.adp",
        "week.tasks",
        b"task t1\r\n",
        Some(b"my own notes\r\n"),
    );

    let reason = created.unwrap_err();
    assert!(reason.contains("plan.adp"), "{reason}");
}

#[test]
fn a_binding_that_does_not_create_on_first_placement_is_refused_with_a_reason() {
    let (problems, loaded) = documents::load_file(&vendor("specifications/fbl/timeline.fbl"));
    assert!(!problems.iter().any(|p| p.severity == ProblemSeverity::Error));
    let timeline = loaded.unwrap().binding("timeline").unwrap().clone();

    let created = create_registration(
        &timeline,
        "generic/timeline",
        "plan.adp",
        "plan.tml",
        b"elements: []\n",
        None,
    );

    assert!(!created.unwrap_err().trim().is_empty());
}

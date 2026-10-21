//! Adding a relation from the explorer (the standalone host's `KnowledgeContextActions.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirement 5.1): the action is offered on a knowledge file and on nothing else, asks for the
//! table to relate to in the file dialog - which accepts knowledge files only and offers this table first - and makes
//! the relation as one undoable step.
//!
//! The standalone host's last test opens a session beside the provider and waits for its watched file; here the
//! shell sends the gesture `context_actions::commit` gives through the open table's session, which is what the last
//! test does.

mod common;

use std::path::{Path, PathBuf};

use adp_context::Prompt;
use adp_designer_knowledge::context_actions::{self, Commit, MANY, ONE, ONE_WAY, RELATE, TWO_WAY};
use adp_designer_knowledge::{KnowledgeSession, store};
use adp_desl::table::session::DesignerSession;

/// The example and the table it relates to, each with the registration that makes it a document of the project.
fn registered(root: &Path) -> PathBuf {
    common::copy_example("cities.yaml", root);
    std::fs::write(root.join("cities.adp"), "etalii/knowledge\r\nbody: cities.yaml\r\n").expect("written");
    std::fs::write(root.join("provinces.adp"), "etalii/knowledge\r\n").expect("written");
    std::fs::write(root.join("notes.yaml"), "just: notes\r\n").expect("written");
    std::fs::write(root.join("diagram.adp"), "generic/timeline\r\n").expect("written");
    root.join("cities.adp")
}

fn leaves(path: &Path, container: bool) -> Vec<String> {
    context_actions::discover(path, container)
        .iter()
        .flat_map(|g| g.actions.iter().map(|a| a.id.clone()))
        .collect()
}

/// The three answers of the entry, the last carried out on the files: what puts them back.
fn answered(
    root: &Path,
    entry: &Path,
    file: &str,
    holds: &str,
    two_way: bool,
    name: &str,
) -> Result<Option<adp_designer_knowledge::command::KnowledgeRestore>, String> {
    let mut action = RELATE.to_owned();
    let side = if two_way { TWO_WAY } else { ONE_WAY };
    for (value, text) in [(file, ""), (holds, ""), (side, name)] {
        match context_actions::commit(root, entry, &action, value, text) {
            Commit::Asks { next_action_id, .. } => action = next_action_id,
            Commit::Make { body_path, gesture } => return context_actions::make(&body_path, &gesture),
            Commit::Failed(reason) => return Err(reason),
        }
    }
    Err("the third answer made nothing".into())
}

#[test]
fn a_knowledge_file_offers_a_relation_as_one_entry_from_its_registration_and_from_its_data_file() {
    let folder = common::folder();
    let registration = registered(folder.path());

    let groups = context_actions::discover(&registration, false);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].actions[0].label, "Add relation…");
    assert_eq!(leaves(&registration, false), [RELATE]);
    assert_eq!(leaves(&folder.path().join("cities.yaml"), false), [RELATE]);

    // A registration that names no file is the registration of the file of its own name.
    assert_eq!(leaves(&folder.path().join("provinces.adp"), false), [RELATE]);
}

#[test]
fn nothing_else_offers_a_relation() {
    let folder = common::folder();
    registered(folder.path());

    assert!(leaves(folder.path(), true).is_empty());
    assert!(leaves(&folder.path().join("notes.yaml"), false).is_empty());
    assert!(leaves(&folder.path().join("diagram.adp"), false).is_empty());
    assert!(leaves(&folder.path().join("gone.adp"), false).is_empty());
}

#[test]
fn the_table_to_relate_to_is_asked_for_in_the_file_dialog_which_takes_knowledge_files_only_this_one_first() {
    let folder = common::folder();
    let root = folder.path();
    let registration = registered(root);

    let Ok(Prompt::File {
        title, pinned, files, ..
    }) = context_actions::execute(root, &registration)
    else {
        panic!("a file dialog");
    };
    assert_eq!(title, "Relate to");
    assert_eq!(pinned.len(), 1);
    assert_eq!(
        (pinned[0].label.as_str(), pinned[0].id.as_str()),
        ("This table", "cities.yaml")
    );
    let offered: Vec<&str> = files.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(offered, [common::RELATED]);
    assert!(context_actions::accepts(&root.join(common::RELATED)));
    assert!(context_actions::accepts(&root.join("cities.yaml")));
    assert!(!context_actions::accepts(&root.join("notes.yaml")));
    assert!(!context_actions::accepts(&registration));

    // What is chosen is judged by the same rule, named as the dialog names it: by its path from the project's root.
    assert!(context_actions::validate(root, RELATE, common::RELATED).is_ok());
    assert!(context_actions::validate(root, RELATE, "notes.yaml").is_err());
}

#[test]
fn the_relation_is_made_as_its_answers_say_in_one_undoable_step() {
    for (holds, limit, two_way) in [
        (MANY, "none", false),
        (ONE, "one", false),
        (MANY, "none", true),
        (ONE, "one", true),
    ] {
        let folder = common::folder();
        let root = folder.path();
        let registration = registered(root);
        let cities = root.join("cities.yaml");
        let provinces = root.join(common::RELATED);
        let (ours, theirs) = (std::fs::read(&cities).unwrap(), std::fs::read(&provinces).unwrap());

        let restore = answered(root, &registration, common::RELATED, holds, two_way, "").expect("made");

        // A relation to the other table under its name, holding what was answered.
        let table = store::read(&cities).body.unwrap().table().clone();
        let relation = table
            .columns
            .iter()
            .find(|p| p.name == "Provinces")
            .expect("the relation");
        assert_eq!(
            (
                relation.value_type.as_str(),
                relation.target_file.as_str(),
                relation.limit.as_str()
            ),
            ("relation", common::RELATED, limit)
        );
        // Two-way, the other table shows it under this table's name; one-way, the other table is as it was.
        let others = store::read(&provinces).body.unwrap().table().clone();
        if two_way {
            let other = others
                .columns
                .iter()
                .find(|p| p.name == "Cities")
                .expect("the other side");
            assert_eq!(
                (
                    other.counterpart.as_str(),
                    other.is_computed,
                    other.target_file.as_str()
                ),
                (relation.id.as_str(), true, "cities.yaml")
            );
            assert_eq!(other.id, relation.counterpart);
        } else {
            assert_eq!(std::fs::read(&provinces).unwrap(), theirs);
            assert_eq!(relation.counterpart, "");
        }

        // One undo puts both back.
        restore.expect("a byte changed").undo().expect("undone");
        assert_eq!(std::fs::read(&cities).unwrap(), ours);
        assert_eq!(std::fs::read(&provinces).unwrap(), theirs);
    }
}

#[test]
fn a_relation_is_asked_for_in_notions_order_the_table_how_many_rows_and_whether_both_tables_show_it() {
    let folder = common::folder();
    let root = folder.path();
    let registration = registered(root);
    let cities = root.join("cities.yaml");
    let provinces = root.join(common::RELATED);
    let (ours, theirs) = (std::fs::read(&cities).unwrap(), std::fs::read(&provinces).unwrap());

    // The table is answered: followed by how many rows a cell holds; nothing is written yet.
    let Commit::Asks {
        prompt: Prompt::Choice {
            options, name_field, ..
        },
        next_action_id: first,
    } = context_actions::commit(root, &registration, RELATE, common::RELATED, "")
    else {
        panic!("a choice");
    };
    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, [MANY, ONE]);
    assert_eq!(name_field, None);
    assert_eq!(std::fs::read(&cities).unwrap(), ours);

    // One row is answered: followed by where it is shown, with a name for the other table only when it is shown there.
    let Commit::Asks {
        prompt: Prompt::Choice {
            options, name_field, ..
        },
        next_action_id: second,
    } = context_actions::commit(root, &registration, &first, ONE, "")
    else {
        panic!("a choice");
    };
    let ids: Vec<&str> = options.iter().map(|o| o.id.as_str()).collect();
    assert_eq!(ids, [ONE_WAY, TWO_WAY]);
    let field = name_field.expect("a name field");
    assert_eq!(
        (field.label.as_str(), field.initial_value.as_str()),
        (context_actions::OTHER_SIDE_LABEL, "Cities")
    );
    assert_ne!(options[0].name_suppressed_reason, "");
    assert_eq!(
        (
            options[1].name_suppressed_reason.as_str(),
            options[1].suggested_value.as_str()
        ),
        ("", "Cities")
    );
    assert_eq!(std::fs::read(&provinces).unwrap(), theirs);

    // What is typed there is judged as a name.
    assert!(context_actions::validate(root, &second, "Seats").is_ok());
    assert_eq!(
        context_actions::validate(root, &second, "  ").unwrap_err(),
        context_actions::OTHER_SIDE_NEEDS_NAME
    );

    // Both tables, under a name: made, with nothing more to ask.
    let Commit::Make { body_path, gesture } = context_actions::commit(root, &registration, &second, TWO_WAY, " Seats ")
    else {
        panic!("made");
    };
    context_actions::make(&body_path, &gesture).expect("made");
    let table = store::read(&cities).body.unwrap().table().clone();
    assert!(table.columns.iter().any(|p| p.value_type == "relation"
        && p.limit == "one"
        && p.target_file == common::RELATED
        && !p.counterpart.is_empty()));
    let others = store::read(&provinces).body.unwrap().table().clone();
    assert!(others.columns.iter().any(|p| p.name == "Seats" && p.is_computed));

    // An answer that is none of a step's is refused.
    assert!(matches!(
        context_actions::commit(root, &registration, &first, "several", ""),
        Commit::Failed(_)
    ));
}

#[test]
fn a_relation_to_the_table_itself_is_made_by_choosing_this_table() {
    let folder = common::folder();
    let root = folder.path();
    let registration = registered(root);

    answered(root, &registration, "cities.yaml", ONE, false, "").expect("made");

    let table = store::read(&root.join("cities.yaml")).body.unwrap().table().clone();
    let relation = table
        .columns
        .iter()
        .find(|p| p.name == "Relation")
        .expect("the relation");
    assert_eq!((relation.target_file.as_str(), relation.limit.as_str()), (".", "one"));
}

#[test]
fn the_chosen_file_is_named_from_the_projects_root_also_from_a_folder_below_it() {
    let folder = common::folder();
    let root = folder.path();
    std::fs::create_dir(root.join("here")).unwrap();
    std::fs::create_dir(root.join("there")).unwrap();
    common::copy_example("cities.yaml", &root.join("here"));
    std::fs::copy(
        root.join("here").join(common::RELATED),
        root.join("there").join("regions.yaml"),
    )
    .unwrap();
    let table = root.join("here").join("cities.yaml");

    answered(root, &table, "there/regions.yaml", MANY, false, "").expect("made");

    let read = store::read(&table).body.unwrap().table().clone();
    let relation = read
        .columns
        .iter()
        .find(|p| p.name == "Provinces 2" || p.name == "Provinces")
        .expect("the relation");
    assert_eq!(relation.target_file, "../there/regions.yaml");
}

#[test]
fn an_open_table_shows_a_relation_made_from_the_explorer_at_once() {
    let folder = common::folder();
    let root = folder.path();
    let registration = registered(root);
    let cities = root.join("cities.yaml");
    let mut session = KnowledgeSession::open_for_editing(&cities, root);
    let has_relation = |path: &Path| {
        store::read(path)
            .body
            .unwrap()
            .table()
            .columns
            .iter()
            .any(|p| p.name == "Provinces")
    };

    // The last answer's gesture goes through the open table's session, as the shell sends it.
    let mut action = RELATE.to_owned();
    for (value, text) in [(common::RELATED, ""), (MANY, ""), (ONE_WAY, "")] {
        match context_actions::commit(root, &registration, &action, value, text) {
            Commit::Asks { next_action_id, .. } => action = next_action_id,
            Commit::Make { gesture, .. } => assert_eq!(session.edit("relate", &gesture), ""),
            Commit::Failed(reason) => panic!("{reason}"),
        }
    }

    // Shown before it is written, in the file once it is, and undone as the table's own edit.
    assert!(
        session
            .baseline()
            .columns
            .iter()
            .any(|c| c.name == "Provinces" && c.kind == "relation")
    );
    session.write_pending();
    assert!(has_relation(&cities));
    session.undo().expect("undone");
    assert!(!has_relation(&cities));
}

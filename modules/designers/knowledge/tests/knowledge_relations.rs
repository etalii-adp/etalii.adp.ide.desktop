//! Relations between knowledge files (the standalone host's `KnowledgeRelations.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirement 5): a relation names its target by a path and its values by the target's row ids,
//! shows them by their titles, and keeps them when the target or a row is not there; a two-way relation is made in
//! both files in one step and shown from both sides; and the parent relation never makes a row its own ancestor.
//!
//! Two tables open in one application hear of each other's writes through the core; here a test tells the one table
//! of what the other wrote (`told`), where the standalone host's tests share a `KnowledgeDocuments`.

mod common;

use std::path::{Path, PathBuf};

use adp_designer_knowledge::edits::{NOT_A_RELATION, OTHER_SIDE_CHOICE, PARENT_CYCLE, PARENT_HOLDS_ONE};
use adp_designer_knowledge::relations::SELF;
use adp_designer_knowledge::{KnowledgeProperty, KnowledgeTable, store, target_rename, validator};
use adp_desl::table::model::{Settings, TableCell, TableFindingSeverity, TableGesture, TableRow};
use common::{EXTENSIONS, EditingTable, RELATED, copy_plain, describe};

fn provinces(folder: &Path) -> PathBuf {
    folder.join(RELATED)
}

fn open(extension: &str, folder: &Path) -> EditingTable {
    EditingTable::new(copy_plain(&format!("cities{extension}"), folder))
}

fn cell<'l>(lines: &'l [TableRow], row: &str, column: &str) -> Option<&'l TableCell> {
    lines.iter().find(|l| l.id == row).expect("the row").cell(column)
}

fn property(table: &EditingTable, name: &str) -> KnowledgeProperty {
    table
        .on_disk()
        .columns
        .into_iter()
        .find(|p| p.name == name)
        .expect("the property")
}

fn column_settings(table: &EditingTable, id: &str) -> Settings {
    table
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.id == id)
        .expect("the column")
        .settings
}

fn read_table(path: &Path) -> KnowledgeTable {
    store::read(path).body.expect("it reads").table().clone()
}

fn relation(name: &str, target: &str, counterpart: &str) -> TableGesture {
    let mut gesture = TableGesture::new("addRelation")
        .values([name])
        .setting("target", target);
    if !counterpart.is_empty() {
        gesture = gesture.setting("counterpart", counterpart);
    }
    gesture
}

// ---- what a relation shows ----

#[test]
fn a_related_row_is_stored_by_its_id_and_shown_by_its_title() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let baseline = table.baseline();
        let lines = table.lines();
        let shown = cell(&lines, "r1", "p5").expect("a cell");

        // The id in the file, the title on the screen, and the target's rows to choose from.
        assert_eq!(vec!["nh"], shown.values);
        assert_eq!(vec!["Noord-Holland"], shown.labels, "{extension}");
        let column = baseline.columns.iter().find(|c| c.id == "p5").expect("p5");
        assert_eq!(
            vec![
                "nh:Noord-Holland",
                "zh:Zuid-Holland",
                "an:Antwerpen",
                "9wvrdvjn4ob2f2nw421bouq7y:Limburg",
                "cdns3i0ehcr45q7zh0z15iqzt:Overijssel"
            ],
            column
                .options
                .iter()
                .map(|o| format!("{}:{}", o.id, o.name))
                .collect::<Vec<_>>()
        );
        assert!(!column.settings.contains_key("unresolved"));
        assert!(baseline.findings.is_empty(), "{:?}", baseline.findings);
    }
}

#[test]
fn a_relation_whose_target_is_not_there_keeps_its_values_and_says_so() {
    let folder = common::folder();
    let path = copy_plain("cities.yaml", folder.path());
    std::fs::remove_file(provinces(folder.path())).expect("deleted");
    let before = std::fs::read(&path).expect("read");
    let mut table = EditingTable::new(path.clone());

    let baseline = table.baseline();
    let lines = table.lines();

    // Reported on the property, the value still there and shown as what it is, the file untouched.
    let [finding] = baseline.findings.as_slice() else {
        panic!("{:?}", baseline.findings)
    };
    assert_eq!(
        (validator::UNRESOLVED_TARGET, TableFindingSeverity::Warning, "p5"),
        (finding.code.as_str(), finding.severity, finding.column_id.as_str())
    );
    let shown = cell(&lines, "r1", "p5").expect("a cell");
    assert_eq!(vec!["nh"], shown.values);
    assert_eq!(vec![""], shown.labels);
    assert_eq!(
        "true",
        baseline.columns.iter().find(|c| c.id == "p5").expect("p5").settings["unresolved"]
    );
    assert_eq!(before, std::fs::read(&path).expect("read"));
}

#[test]
fn a_value_naming_a_row_that_is_not_in_the_target_is_kept_and_reported_on_its_cell() {
    let folder = common::folder();
    let path = copy_plain("cities.yaml", folder.path());
    let text = std::fs::read_to_string(&path).expect("read");
    std::fs::write(&path, text.replace("- row: nh", "- row: atlantis")).expect("written");
    let mut table = EditingTable::new(path);
    let findings = table.baseline().findings;
    let [finding] = findings.as_slice() else {
        panic!("{findings:?}")
    };
    assert_eq!(
        (validator::UNRESOLVED_ROW, "r1", "p5"),
        (
            finding.code.as_str(),
            finding.row_id.as_str(),
            finding.column_id.as_str()
        )
    );
    assert_eq!(
        vec!["atlantis"],
        cell(&table.lines(), "r1", "p5").expect("a cell").values
    );
}

#[test]
fn a_related_row_is_a_row_of_the_target_and_no_more_than_the_relation_holds() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let before = table.bytes();
    let (_, stranger) = table.begin(TableGesture::new("setCell").row("r2").column("p5").values(["atlantis"]));
    let (_, several) = table.begin(TableGesture::new("setCell").row("r2").column("p5").values(["nh", "zh"]));
    assert_eq!("That row is not in Provinces.", stranger);
    assert_eq!("This relation holds one row.", several);
    assert_eq!(before, table.bytes());

    table.edit(TableGesture::new("setCell").row("r2").column("p5").values(["an"]));
    assert_eq!(
        vec!["an"],
        table.on_disk().row("r2").expect("r2").cell("p5").expect("p5").values
    );
    assert_eq!(
        vec!["Antwerpen"],
        cell(&table.lines(), "r2", "p5").expect("a cell").labels
    );
}

#[test]
fn a_renamed_row_over_there_is_shown_under_its_new_name_here_at_once() {
    // Both tables open, as two tabs of one application are.
    let folder = common::folder();
    let mut cities = open(".yaml", folder.path());
    let mut provinces = EditingTable::new(provinces(folder.path()));
    assert_eq!(
        vec!["Noord-Holland"],
        cell(&cities.lines(), "r1", "p5").expect("a cell").labels
    );

    // The province is renamed in its own table.
    provinces.edit(
        TableGesture::new("setCell")
            .row("nh")
            .column("n1")
            .values(["North Holland"]),
    );
    cities.told(&[provinces.path.as_path()]);

    // Nothing in the cities' file names the province by its name, so nothing there changed - and it shows the new one.
    assert_eq!(
        vec!["North Holland"],
        cell(&cities.lines(), "r1", "p5").expect("a cell").labels
    );
    assert_eq!(
        vec!["nh"],
        cities.on_disk().row("r1").expect("r1").cell("p5").expect("p5").values
    );
}

// ---- making a relation ----

#[test]
fn a_one_way_relation_is_a_property_of_this_file_and_the_target_is_not_touched() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let target = std::fs::read(provinces(folder.path())).expect("read");
        table.edit(relation("Twinned with", RELATED, "").setting("limit", "none"));

        // The target by its path from this file, any number of rows, and no other side.
        let made = property(&table, "Twinned with");
        assert_eq!(
            ("relation", RELATED, "none", "", false),
            (
                made.value_type.as_str(),
                made.target_file.as_str(),
                made.limit.as_str(),
                made.counterpart.as_str(),
                made.is_computed
            ),
            "{extension}"
        );
        assert_eq!(target, std::fs::read(provinces(folder.path())).expect("read"));
        assert!(table.baseline().findings.is_empty());
    }
}

#[test]
fn a_two_way_relation_is_made_in_both_files_in_one_step_and_undone_in_one() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let (ours, theirs) = (table.bytes(), std::fs::read(provinces(folder.path())).expect("read"));

        table.edit(relation("Seat of", RELATED, "Seats"));

        // Each side names the other; the other side is computed, has no cells, and points back at this file.
        let here = property(&table, "Seat of");
        let there_table = read_table(&provinces(folder.path()));
        let there = there_table.columns.iter().find(|p| p.name == "Seats").expect("Seats");
        assert_eq!(
            (there.id.as_str(), false),
            (here.counterpart.as_str(), here.is_computed)
        );
        assert_eq!(
            (here.id.as_str(), true, "relation", format!("cities{extension}")),
            (
                there.counterpart.as_str(),
                there.is_computed,
                there.value_type.as_str(),
                there.target_file.clone()
            )
        );
        assert!(there_table.rows.iter().all(|r| r.cell(&there.id).is_none()));

        // One undo: both files as they were. And one redo.
        table.session.undo().expect("undone");
        assert_eq!(ours, table.bytes(), "{extension}");
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
        table.session.redo().expect("redone");
        assert!(
            read_table(&provinces(folder.path()))
                .columns
                .iter()
                .any(|p| p.name == "Seats")
        );
        assert!(table.on_disk().columns.iter().any(|p| p.name == "Seat of"));
    }
}

#[test]
fn the_other_side_shows_the_rows_that_name_it_and_is_not_filled_in_from_there() {
    // A two-way relation, and both tables open.
    let folder = common::folder();
    let mut cities = open(".yaml", folder.path());
    cities.edit(relation("Near", RELATED, "Cities nearby"));
    let near = property(&cities, "Near").id;
    let mut provinces = EditingTable::new(provinces(folder.path()));
    let nearby = provinces
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.name == "Cities nearby")
        .expect("the other side");

    // Values are given on the side that holds them.
    cities.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(near.clone())
            .values(["nh", "zh"]),
    );
    cities.edit(TableGesture::new("setCell").row("r2").column(near).values(["zh"]));
    provinces.told(&[cities.path.as_path()]);

    // The other side shows them at once, by title, and has nothing of its own in its file.
    let lines = provinces.lines();
    assert_eq!(vec!["r1"], cell(&lines, "nh", &nearby.id).expect("a cell").values);
    let zh = cell(&lines, "zh", &nearby.id).expect("a cell");
    assert_eq!(vec!["r1", "r2"], zh.values);
    assert_eq!(vec!["Amsterdam", "Antwerp"], zh.labels);
    assert!(cell(&lines, "an", &nearby.id).is_none());
    assert_eq!("true", nearby.settings["computed"]);
    assert!(provinces.on_disk().rows.iter().all(|r| r.cell(&nearby.id).is_none()));

    // It is not filled in from here.
    let (_, answer) = provinces.begin(TableGesture::new("setCell").row("an").column(nearby.id).values(["r2"]));
    assert_eq!("This side of the relation is filled in from the other table.", answer);
}

#[test]
fn deleting_one_side_of_a_two_way_relation_asks_what_becomes_of_the_other_side() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(relation("Near", RELATED, "Cities nearby"));
    let near = property(&table, "Near").id;
    let (ours, theirs) = (table.bytes(), std::fs::read(provinces(folder.path())).expect("read"));

    let (_, answer) = table.begin(TableGesture::new("deleteColumn").column(near.clone()));

    // Asked, nothing written, and the column names its other side so that the question can be put.
    assert_eq!(OTHER_SIDE_CHOICE, answer);
    assert_eq!(ours, table.bytes());
    assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
    assert_eq!("Cities nearby", column_settings(&table, &near)["otherSide"]);
    assert!(!column_settings(&table, "p5").contains_key("otherSide"));
}

#[test]
fn deleting_one_side_and_the_other_side_too_takes_both_out_in_one_step() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let (ours, theirs) = (table.bytes(), std::fs::read(provinces(folder.path())).expect("read"));
        table.edit(relation("Near", RELATED, "Cities nearby"));
        let near = property(&table, "Near").id;
        table.edit(
            TableGesture::new("setCell")
                .row("r1")
                .column(near.clone())
                .values(["nh", "zh"]),
        );
        let (ours_with, theirs_with) = (table.bytes(), std::fs::read(provinces(folder.path())).expect("read"));

        table.edit(
            TableGesture::new("deleteColumn")
                .column(near)
                .setting("otherSide", "delete"),
        );

        // Both files as they were before the relation.
        assert_eq!(ours, table.bytes(), "{extension}");
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));

        // One undo: both sides are back, values and all.
        table.session.undo().expect("undone");
        assert_eq!(ours_with, table.bytes());
        assert_eq!(theirs_with, std::fs::read(provinces(folder.path())).expect("read"));
    }
}

#[test]
fn deleting_the_side_that_holds_the_values_and_keeping_the_other_gives_the_other_those_values() {
    for extension in EXTENSIONS {
        // Amsterdam is near two provinces, Antwerp near one of them.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(relation("Near", RELATED, "Cities nearby"));
        let near = property(&table, "Near").id;
        table.edit(
            TableGesture::new("setCell")
                .row("r1")
                .column(near.clone())
                .values(["nh", "zh"]),
        );
        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column(near.clone())
                .values(["zh"]),
        );

        table.edit(
            TableGesture::new("deleteColumn")
                .column(near.clone())
                .setting("otherSide", "keep"),
        );

        // This side is gone; the other is a relation of its own, holding what it showed.
        assert!(table.on_disk().column(&near).is_none());
        let provinces = read_table(&provinces(folder.path()));
        let nearby = provinces
            .columns
            .iter()
            .find(|p| p.name == "Cities nearby")
            .expect("the other side");
        assert_eq!(
            ("", false, "relation", format!("cities{extension}")),
            (
                nearby.counterpart.as_str(),
                nearby.is_computed,
                nearby.value_type.as_str(),
                nearby.target_file.clone()
            ),
            "{extension}"
        );
        let held = |row: &str| {
            provinces
                .row(row)
                .expect("the row")
                .cell(&nearby.id)
                .map(|c| c.values.clone())
        };
        assert_eq!(Some(vec!["r1".to_owned()]), held("nh"));
        assert_eq!(Some(vec!["r1".to_owned(), "r2".to_owned()]), held("zh"));
        assert_eq!(None, held("an"));
    }
}

#[test]
fn deleting_the_side_that_shows_the_values_and_keeping_the_other_leaves_the_other_its_values() {
    let folder = common::folder();
    let mut cities = open(".yaml", folder.path());
    cities.edit(relation("Near", RELATED, "Cities nearby"));
    let near = property(&cities, "Near").id;
    cities.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(near.clone())
            .values(["nh"]),
    );
    let mut provinces = EditingTable::new(provinces(folder.path()));
    let nearby = provinces
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.name == "Cities nearby")
        .expect("the other side");
    assert_eq!("Near", nearby.settings["otherSide"]);

    // The computed side is deleted from its own table.
    provinces.edit(
        TableGesture::new("deleteColumn")
            .column(nearby.id.clone())
            .setting("otherSide", "keep"),
    );

    // The side that holds the values is a one-way relation, with its values.
    assert!(provinces.on_disk().column(&nearby.id).is_none());
    let kept = read_table(&cities.path);
    let held = kept.column(&near).expect("near");
    assert_eq!(("", false), (held.counterpart.as_str(), held.is_computed));
    assert_eq!(
        vec!["nh"],
        kept.row("r1").expect("r1").cell(&near).expect("near").values
    );
}

#[test]
fn deleting_one_side_of_a_relation_to_the_table_itself_keeps_the_other_in_the_same_file() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(relation("Rival of", SELF, "Rivalled by"));
    let (rival, rivalled) = (property(&table, "Rival of").id, property(&table, "Rivalled by").id);
    table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(rival.clone())
            .values(["r2"]),
    );

    table.edit(
        TableGesture::new("deleteColumn")
            .column(rival.clone())
            .setting("otherSide", "keep"),
    );

    // What the other side showed is now what it holds, and the table shows it as before.
    let kept = table.on_disk();
    assert!(kept.column(&rival).is_none());
    let other = kept.column(&rivalled).expect("rivalled");
    assert_eq!(("", false), (other.counterpart.as_str(), other.is_computed));
    assert_eq!(
        vec!["r1"],
        kept.row("r2").expect("r2").cell(&rivalled).expect("a cell").values
    );
    assert_eq!(
        vec!["Amsterdam"],
        cell(&table.lines(), "r2", &rivalled).expect("a cell").labels
    );
    assert!(!column_settings(&table, &rivalled).contains_key("otherSide"));
}

#[test]
fn when_the_target_is_renamed_the_relation_names_it_by_its_new_name_and_by_its_old_one_when_that_is_undone() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let cities = copy_plain(&format!("cities{extension}"), folder.path());
        let before = std::fs::read(&cities).expect("read");
        let regions = folder.path().join("regions.yaml");

        // The target is renamed, as the explorer does it.
        std::fs::rename(provinces(folder.path()), &regions).expect("renamed");
        target_rename::renamed(folder.path(), &provinces(folder.path()), &regions);

        // The relation names the file where it is now, and resolves; the renamed file is untouched.
        assert_eq!(
            "regions.yaml",
            read_table(&cities).column("p5").expect("p5").target_file,
            "{extension}"
        );
        {
            let mut open = EditingTable::new(cities.clone());
            assert_eq!(
                vec!["Noord-Holland"],
                cell(&open.lines(), "r1", "p5").expect("a cell").labels
            );
            assert!(open.baseline().findings.is_empty());
        }

        // The rename is undone, which is a rename back: byte for byte what it was.
        std::fs::rename(&regions, provinces(folder.path())).expect("renamed back");
        target_rename::renamed(folder.path(), &regions, &provinces(folder.path()));
        assert_eq!(before, std::fs::read(&cities).expect("read"));
    }
}

#[test]
fn when_a_folder_is_renamed_relations_into_it_follow_and_relations_inside_it_stay() {
    // Cities relates to a table in a folder; a table in that folder relates to its neighbour there.
    let folder = common::folder();
    let cities = copy_plain("cities.yaml", folder.path());
    let (old, renamed) = (folder.path().join("data"), folder.path().join("reference"));
    std::fs::create_dir(&old).expect("made");
    std::fs::rename(provinces(folder.path()), old.join(RELATED)).expect("moved");
    let text = std::fs::read_to_string(&cities).expect("read");
    std::fs::write(
        &cities,
        text.replace("target: provinces.yaml", "target: data/provinces.yaml"),
    )
    .expect("written");
    std::fs::write(old.join("inside.yaml"), &text).expect("written");
    assert_eq!(
        "data/provinces.yaml",
        read_table(&cities).column("p5").expect("p5").target_file
    );

    std::fs::rename(&old, &renamed).expect("renamed");
    target_rename::renamed(folder.path(), &old, &renamed);

    assert_eq!(
        "reference/provinces.yaml",
        read_table(&cities).column("p5").expect("p5").target_file
    );
    assert_eq!(
        text,
        std::fs::read_to_string(renamed.join("inside.yaml")).expect("read")
    );
}

#[test]
fn deleting_a_row_other_tables_relate_to_tells_how_many_first_and_deletes_when_confirmed() {
    // In the example, Amsterdam relates to the province nh; a second city is made to as well.
    let folder = common::folder();
    let cities = copy_plain("cities.yaml", folder.path());
    EditingTable::new(cities).edit(TableGesture::new("setCell").row("r2").column("p5").values(["nh"]));
    let mut provinces = EditingTable::under(provinces(folder.path()), folder.path().to_path_buf());
    let before = provinces.bytes();

    // The row is deleted without more: a question that says how many, and nothing deleted.
    let (_, asked) = provinces.begin(TableGesture::new("deleteRows").row("nh"));
    assert_eq!(
        "confirm:2 rows in other tables relate to this row. Deleting it leaves those relations pointing at nothing.",
        asked
    );
    assert_eq!(before, provinces.bytes());

    // A row nobody relates to is deleted at once; the other one when confirmed.
    provinces.edit(TableGesture::new("deleteRows").row("an"));
    provinces.edit(TableGesture::new("deleteRows").row("nh").setting("confirmed", "true"));
    assert_eq!(
        vec!["zh", "9wvrdvjn4ob2f2nw421bouq7y", "cdns3i0ehcr45q7zh0z15iqzt"],
        provinces
            .on_disk()
            .rows
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn when_the_other_file_refuses_neither_file_is_written() {
    // The edit is accepted and shown, and waits to be written.
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let ours = table.bytes();
    let (edit, answer) = table.begin(relation("Seat of", RELATED, "Seats"));
    assert_eq!("", answer);

    // Before it is, the other file becomes one this application does not change.
    let text = std::fs::read_to_string(provinces(folder.path())).expect("read");
    std::fs::write(provinces(folder.path()), text.replace("ded: \"0.1\"", "ded: \"9.0\"")).expect("written");
    let theirs = std::fs::read(provinces(folder.path())).expect("read");
    let settled = table.settled(&edit);

    // Refused with the other file's name, taken back, and neither file written.
    assert!(!settled.written);
    assert!(
        settled.error.starts_with(&format!("{RELATED} cannot be changed: ")),
        "{}",
        settled.error
    );
    assert_eq!(ours, table.bytes());
    assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
    assert!(!table.baseline().columns.iter().any(|c| c.name == "Seat of"));
}

#[test]
fn a_relation_is_a_property_asked_for_by_name_with_its_table_chosen_from_the_projects_tables() {
    for extension in EXTENSIONS {
        // The table knows the project it is in.
        let folder = common::folder();
        let mut table = EditingTable::under(
            copy_plain(&format!("cities{extension}"), folder.path()),
            folder.path().to_path_buf(),
        );
        let (ours, theirs) = (table.bytes(), std::fs::read(provinces(folder.path())).expect("read"));

        // Asked for by its name alone: nothing is made; the answer lists this table first, then the project's other.
        let (_, asked) = table.begin(TableGesture::new("addRelation").values(["Region"]));
        assert_eq!(
            r#"choose:[{"id":".","label":"This table"},{"id":"provinces.yaml","label":"Provinces"}]"#, asked,
            "{extension}"
        );
        assert_eq!(ours, table.bytes());

        // The table chosen, two-way with no name for the other side, left of the population.
        table.edit(
            TableGesture::new("addRelation")
                .target("p3")
                .values(["Region"])
                .setting("target", RELATED)
                .setting("limit", "one")
                .setting("twoWay", "true")
                .setting("side", "left"),
        );

        // The property is there under its name, where it was put; the other side is called after this table.
        let properties = table.on_disk().columns;
        let at = properties.iter().position(|p| p.name == "Region").expect("Region");
        let region = &properties[at];
        assert_eq!(
            ("relation", RELATED, "one", false),
            (
                region.value_type.as_str(),
                region.target_file.as_str(),
                region.limit.as_str(),
                region.is_computed
            )
        );
        assert_eq!("p3", properties[at + 1].id);
        let others = read_table(&provinces(folder.path()));
        let other = others.column(&region.counterpart).expect("the other side");
        assert_eq!(
            ("Cities", true, region.id.as_str()),
            (other.name.as_str(), other.is_computed, other.counterpart.as_str())
        );
        assert_ne!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
    }
}

#[test]
fn a_relations_limit_and_its_other_side_are_set_on_the_property_and_no_row_changes() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let province = |table: &EditingTable| table.on_disk().column("p5").expect("p5").clone();
        assert_eq!(
            ("one", ""),
            (province(&table).limit.as_str(), province(&table).counterpart.as_str())
        );
        let rows_only = |t: KnowledgeTable| {
            describe(&KnowledgeTable {
                columns: Vec::new(),
                views: Vec::new(),
                ..t
            })
        };
        let rows = rows_only(table.on_disk());
        let theirs = describe(&read_table(&provinces(folder.path())));

        // Any number of rows, then one again; saying what it is already changes nothing.
        table.edit(
            TableGesture::new("setRelationLimit")
                .column("p5")
                .setting("limit", "none"),
        );
        assert_eq!("none", province(&table).limit, "{extension}");
        assert_eq!("none", column_settings(&table, "p5")["limit"]);
        let bytes = table.bytes();
        let (_, same) = table.begin(
            TableGesture::new("setRelationLimit")
                .column("p5")
                .setting("limit", "none"),
        );
        assert_eq!("", same);
        assert_eq!(bytes, table.bytes());
        table.edit(
            TableGesture::new("setRelationLimit")
                .column("p5")
                .setting("limit", "one"),
        );
        assert_eq!("one", province(&table).limit);

        // Shown in the provinces too, under a name.
        table.edit(
            TableGesture::new("setRelationOtherSide")
                .column("p5")
                .setting("twoWay", "true")
                .setting("counterpart", "Towns"),
        );
        let others = read_table(&provinces(folder.path()));
        let other = others.columns.iter().find(|p| p.name == "Towns").expect("Towns");
        assert_eq!(
            (true, "p5", "relation"),
            (other.is_computed, other.counterpart.as_str(), other.value_type.as_str())
        );
        assert_eq!(other.id, province(&table).counterpart);
        assert_eq!("Towns", column_settings(&table, "p5")["otherSide"]);

        // In this table only again: the other side is gone, this side names none, the provinces are what they were.
        table.edit(
            TableGesture::new("setRelationOtherSide")
                .column("p5")
                .setting("twoWay", "false"),
        );
        assert_eq!("", province(&table).counterpart);
        assert!(
            !read_table(&provinces(folder.path()))
                .columns
                .iter()
                .any(|p| p.name == "Towns")
        );
        assert_eq!(theirs, describe(&read_table(&provinces(folder.path()))));

        // Through all of it no row changed.
        assert_eq!(rows, rows_only(table.on_disk()));

        // Neither is set on a property that is not a relation, and the parent relation keeps its one row.
        let bytes = table.bytes();
        let (_, number) = table.begin(
            TableGesture::new("setRelationLimit")
                .column("p3")
                .setting("limit", "one"),
        );
        let (_, side) = table.begin(
            TableGesture::new("setRelationOtherSide")
                .column("p3")
                .setting("twoWay", "true"),
        );
        assert_eq!((NOT_A_RELATION, NOT_A_RELATION), (number.as_str(), side.as_str()));
        assert_eq!(bytes, table.bytes());
        table.edit(relation("Part of", SELF, "").setting("limit", "one"));
        let part_of = property(&table, "Part of").id;
        table.edit(
            TableGesture::new("setColumnParent")
                .column(part_of.clone())
                .setting("parent", "true"),
        );
        let (_, parent) = table.begin(
            TableGesture::new("setRelationLimit")
                .column(part_of)
                .setting("limit", "none"),
        );
        assert_eq!(PARENT_HOLDS_ONE, parent);
    }
}

#[test]
fn a_relation_needs_a_table_to_relate_to() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let before = table.bytes();
    let (_, none) = table.begin(TableGesture::new("addRelation"));
    let (_, missing) = table.begin(TableGesture::new("addRelation").setting("target", "nowhere.yaml"));

    // Asked for without a table, it is answered with what there is to choose from - here, where no project is
    // known, the table itself.
    assert_eq!(r#"choose:[{"id":".","label":"This table"}]"#, none);
    assert_eq!("'nowhere.yaml' is not there.", missing);
    assert_eq!(before, table.bytes());
}

// ---- the table itself, and the parent relation ----

#[test]
fn a_two_way_relation_to_the_table_itself_has_both_sides_in_the_one_file() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(relation("Rival of", SELF, "Rivalled by"));
    let (rival, rivalled) = (property(&table, "Rival of"), property(&table, "Rivalled by"));
    table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(rival.id.clone())
            .values(["r2"]),
    );

    // One side holds the value, the other shows it from the other end.
    assert_eq!(
        (SELF, rivalled.id.as_str()),
        (rival.target_file.as_str(), rival.counterpart.as_str())
    );
    assert_eq!(
        (SELF, rival.id.as_str(), true),
        (
            rivalled.target_file.as_str(),
            rivalled.counterpart.as_str(),
            rivalled.is_computed
        )
    );
    let lines = table.lines();
    assert_eq!(vec!["Antwerp"], cell(&lines, "r1", &rival.id).expect("a cell").labels);
    assert_eq!(vec!["r1"], cell(&lines, "r2", &rivalled.id).expect("a cell").values);
    assert_eq!(
        vec!["Amsterdam"],
        cell(&lines, "r2", &rivalled.id).expect("a cell").labels
    );
}

#[test]
fn only_a_relation_that_can_be_the_parent_relation_says_its_rows_can_be_nested_by_it() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(relation("Part of", SELF, "").setting("limit", "one"));
    table.edit(relation("Twinned with", SELF, ""));
    let of = |table: &EditingTable, name: &str| -> Settings {
        table
            .baseline()
            .columns
            .into_iter()
            .find(|c| c.name == name)
            .expect("the column")
            .settings
    };
    assert_eq!("true", of(&table, "Part of")["nestable"]);
    assert!(!of(&table, "Twinned with").contains_key("nestable"));
    assert!(!column_settings(&table, "p5").contains_key("nestable"));
    assert!(!of(&table, "Part of").contains_key("parent"));

    // Made the parent relation, as the column's menu does.
    let part_of = property(&table, "Part of").id;
    table.edit(
        TableGesture::new("setColumnParent")
            .column(part_of)
            .setting("parent", "true"),
    );
    assert_eq!("true", of(&table, "Part of")["parent"]);
}

#[test]
fn a_relation_of_a_table_to_itself_shows_both_directions_in_its_one_property() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(relation("Twinned with", SELF, ""));
        let twinned = property(&table, "Twinned with").id;
        let held = |table: &EditingTable, row: &str| {
            table
                .on_disk()
                .row(row)
                .expect("the row")
                .cell(&twinned)
                .map(|c| c.values.clone())
        };

        // Amsterdam is twinned with Antwerp: one of the two holds the pair, and both show it.
        table.edit(
            TableGesture::new("setCell")
                .row("r1")
                .column(twinned.clone())
                .values(["r2"]),
        );
        assert_eq!(Some(vec!["r2".to_owned()]), held(&table, "r1"), "{extension}");
        assert_eq!(None, held(&table, "r2"));
        let lines = table.lines();
        assert_eq!(vec!["r2"], cell(&lines, "r1", &twinned).expect("a cell").values);
        assert_eq!(vec!["r1"], cell(&lines, "r2", &twinned).expect("a cell").values);
        assert_eq!(vec!["Amsterdam"], cell(&lines, "r2", &twinned).expect("a cell").labels);

        // Confirmed from the other end, which changes nothing; then taken away there.
        let bytes = table.bytes();
        let (_, same) = table.begin(
            TableGesture::new("setCell")
                .row("r2")
                .column(twinned.clone())
                .values(["r1"]),
        );
        assert_eq!("", same);
        assert_eq!(bytes, table.bytes());
        table.edit(TableGesture::new("setCell").row("r2").column(twinned.clone()));

        // Gone from the end that held it, and from both rows on the screen.
        assert_eq!(None, held(&table, "r1"));
        assert_eq!(None, held(&table, "r2"));
        assert!(cell(&table.lines(), "r1", &twinned).is_none());

        // The parent relation keeps its direction: a row under another does not put that one under it.
        table.edit(relation("Part of", SELF, "").setting("limit", "one"));
        let part_of = property(&table, "Part of").id;
        table.edit(
            TableGesture::new("setColumnParent")
                .column(part_of.clone())
                .setting("parent", "true"),
        );
        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column(part_of.clone())
                .values(["r1"]),
        );
        assert!(cell(&table.lines(), "r1", &part_of).is_none());
    }
}

#[test]
fn the_parent_relation_nests_rows_and_never_makes_a_row_its_own_ancestor() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(relation("Part of", SELF, "").setting("limit", "one"));
    let part = property(&table, "Part of").id;
    table.edit(TableGesture::new("addRow"));
    let third = table.on_disk().rows.last().expect("a row").id.clone();

    // It becomes the parent relation, and the rows get their parents.
    table.edit(
        TableGesture::new("setColumnParent")
            .column(part.clone())
            .setting("parent", "true"),
    );
    table.edit(
        TableGesture::new("setCell")
            .row("r2")
            .column(part.clone())
            .values(["r1"]),
    );
    table.edit(
        TableGesture::new("setCell")
            .row(third.clone())
            .column(part.clone())
            .values(["r2"]),
    );

    // A row is not its own parent, and not the parent of anything above it.
    assert_eq!(
        PARENT_CYCLE,
        table
            .begin(
                TableGesture::new("setCell")
                    .row("r1")
                    .column(part.clone())
                    .values(["r1"])
            )
            .1
    );
    assert_eq!(
        PARENT_CYCLE,
        table
            .begin(
                TableGesture::new("setCell")
                    .row("r1")
                    .column(part.clone())
                    .values([third.clone()])
            )
            .1
    );
    assert!(property(&table, "Part of").is_parent);

    // A view grouped by it: each row under its parent, a level deeper, and the rows that have none at the top.
    table.edit(TableGesture::new("groupBy").column(part).view("v2"));
    let mut expected = vec!["r1:0".to_owned(), "r2:1".to_owned(), format!("{third}:2")];
    expected.extend(
        [
            "45ako65f2m2wasyxq3mam8ikt",
            "a8q7ddqb6zyubcyf5vuky6iwb",
            "8heh3sdyy0a63fyq7gicize5a",
            "7pa30fmuw51bv1vyl5jnmf5jh",
            "8rj09k224141g98x6rguz7f5h",
            "e1v5f01oi4bx1syl7x6ty3zeo",
            "1dytemy0l8cen9xxkpcxzlgwq",
            "9jh7yjm443uvm99io0nc6ekpi",
            "9cwydn1h89znwl1wlxblxselj",
        ]
        .iter()
        .map(|id| format!("{id}:0")),
    );
    let shown: Vec<String> = table
        .lines()
        .into_iter()
        .filter(|l| !l.is_new_row)
        .map(|l| format!("{}:{}", l.id, l.depth))
        .collect();
    assert_eq!(expected, shown);
    assert!(table.baseline().findings.is_empty(), "{:?}", table.baseline().findings);
}

#[test]
fn only_a_relation_to_the_table_itself_that_holds_one_row_can_be_the_parent_relation() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let (_, elsewhere) = table.begin(
        TableGesture::new("setColumnParent")
            .column("p5")
            .setting("parent", "true"),
    );
    let (_, text) = table.begin(
        TableGesture::new("setColumnParent")
            .column("p1")
            .setting("parent", "true"),
    );
    let refusal = "Only a relation to this table that holds one row can be the parent relation.";
    assert_eq!(refusal, elsewhere);
    assert_eq!(refusal, text);
}

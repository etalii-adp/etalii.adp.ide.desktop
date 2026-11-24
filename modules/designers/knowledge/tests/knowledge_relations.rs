//! Relations between knowledge files (the standalone host's `KnowledgeRelations.Tests.cs` at 8b6ccfc;
//! knowledge-designer Requirement 5; etalii.adp spec 089): a relation names its target by a link to its document and
//! its values by links to the target's rows, shows them by their titles, and keeps them when the target or a row is
//! not there; a two-way relation is one property whose `backlinkName` shows its backlinks in the target, computed
//! and stored in neither file; a file made before links names its target by a path and is converted when it is
//! loaded; and the parent relation never makes a row its own ancestor.
//!
//! Two tables open in one application hear of each other's writes through the core; here a test tells the one table
//! of what the other wrote (`told`), where the standalone host's tests share a `KnowledgeDocuments`.

mod common;

use std::path::{Path, PathBuf};

use adp_designer_knowledge::edits::{BACKLINK_COLUMN, NOT_A_RELATION, PARENT_CYCLE, PARENT_HOLDS_ONE};
use adp_designer_knowledge::relations::SELF;
use adp_designer_knowledge::{KnowledgeProperty, KnowledgeTable, store, target_rename};
use adp_desl::table::model::{Settings, TableCell, TableFindingSeverity, TableGesture, TableRow};
use common::{EXTENSIONS, EditingTable, RELATED, copy_before_links, copy_plain, describe};

/// How the cities' relation names the provinces: a link to their document (etalii.adp spec 089).
const PROVINCES_LINK: &str = "$p7v3k9qd@provinces.yaml";

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
    // The link names a document the workspace does not hold (FBL section 17.6).
    assert_eq!(
        (adp_links::codes::BROKEN_DOCUMENT, TableFindingSeverity::Warning, "p5"),
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
    std::fs::write(&path, text.replace("#nh.Name=Noord-Holland", "#atlantis.Name=Atlantis")).expect("written");
    let mut table = EditingTable::new(path);
    let findings = table.baseline().findings;
    let [finding] = findings.as_slice() else {
        panic!("{findings:?}")
    };
    // The link names a row the target does not hold (FBL section 17.6).
    assert_eq!(
        (adp_links::codes::BROKEN_ELEMENT, "r1", "p5"),
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
            ("relation", PROVINCES_LINK, "none", "", "", false),
            (
                made.value_type.as_str(),
                made.target_file.as_str(),
                made.limit.as_str(),
                made.counterpart.as_str(),
                made.backlink_name.as_str(),
                made.is_computed
            ),
            "{extension}"
        );
        assert_eq!(target, std::fs::read(provinces(folder.path())).expect("read"));
        assert!(table.baseline().findings.is_empty());
    }
}

#[test]
fn a_two_way_relation_is_one_property_whose_backlinks_show_in_the_target_which_is_not_written() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let (ours, theirs) = (table.bytes(), std::fs::read(provinces(folder.path())).expect("read"));

        table.edit(relation("Seat of", RELATED, "Seats"));

        // This side names the target by a link and its backlink column by name; nothing is written over there.
        let here = property(&table, "Seat of");
        assert_eq!(
            (PROVINCES_LINK, "Seats", "", false),
            (
                here.target_file.as_str(),
                here.backlink_name.as_str(),
                here.counterpart.as_str(),
                here.is_computed
            ),
            "{extension}"
        );
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));

        // The target shows the backlink column, computed, and its file has no such property.
        let other = EditingTable::new(provinces(folder.path()));
        let seats = other
            .baseline()
            .columns
            .into_iter()
            .find(|c| c.name == "Seats")
            .expect("the backlink column");
        assert_eq!("true", seats.settings["computed"]);
        assert!(other.on_disk().columns.iter().all(|p| p.name != "Seats"));

        // One undo: this file as it was. And one redo.
        table.session.undo().expect("undone");
        assert_eq!(ours, table.bytes(), "{extension}");
        table.session.redo().expect("redone");
        assert!(table.on_disk().columns.iter().any(|p| p.name == "Seat of"));
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
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
fn deleting_a_two_way_relation_asks_nothing_and_its_backlink_column_goes_with_it() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let before = table.bytes();
    table.edit(relation("Near", RELATED, "Cities nearby"));
    let near = property(&table, "Near").id;
    let theirs = std::fs::read(provinces(folder.path())).expect("read");

    // The column says what its backlink column is called, and there is no other side to ask about.
    assert_eq!("Cities nearby", column_settings(&table, &near)["backlinkName"]);
    assert!(!column_settings(&table, &near).contains_key("otherSide"));
    let (_, answer) = table.begin(TableGesture::new("deleteColumn").column(near.clone()));
    assert_eq!("", answer);
    table.session.write_pending();

    // This file as before the relation; the target untouched, and no longer showing the backlink column.
    assert_eq!(before, table.bytes());
    assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
    let other = EditingTable::new(provinces(folder.path()));
    assert!(other.baseline().columns.iter().all(|c| c.name != "Cities nearby"));
}

#[test]
fn deleting_a_two_way_relation_with_its_values_is_one_step_of_its_own_file() {
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
        let ours_with = table.bytes();

        table.edit(TableGesture::new("deleteColumn").column(near));

        // This file as it was before the relation; the target was never written.
        assert_eq!(ours, table.bytes(), "{extension}");
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));

        // One undo: the relation is back, values and all.
        table.session.undo().expect("undone");
        assert_eq!(ours_with, table.bytes());
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
    }
}

#[test]
fn a_backlink_column_is_changed_with_its_relation_and_not_from_the_table_it_shows_in() {
    let folder = common::folder();
    let mut cities = open(".yaml", folder.path());
    cities.edit(relation("Near", RELATED, "Cities nearby"));
    let mut provinces = EditingTable::new(provinces(folder.path()));
    let before = provinces.bytes();
    let nearby = provinces
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.name == "Cities nearby")
        .expect("the backlink column");

    // It names the relation it shows; it is neither deleted nor renamed here, and nothing is written.
    assert_eq!("Near", nearby.settings["otherSide"]);
    let (_, deleted) = provinces.begin(TableGesture::new("deleteColumn").column(nearby.id.clone()));
    let (_, renamed) = provinces.begin(
        TableGesture::new("renameColumn")
            .column(nearby.id.clone())
            .values(["Towns"]),
    );
    assert_eq!((BACKLINK_COLUMN, BACKLINK_COLUMN), (deleted.as_str(), renamed.as_str()));
    assert_eq!(before, provinces.bytes());
}

#[test]
fn a_relation_written_after_the_target_is_open_shows_there_when_it_is_told() {
    let folder = common::folder();
    let cities_path = copy_plain("cities.yaml", folder.path());
    let mut provinces = EditingTable::new(provinces(folder.path()));
    assert!(provinces.baseline().columns.iter().all(|c| c.name != "Cities nearby"));

    // The relation is made and filled in in the cities, and the provinces are told of the write.
    let mut cities = EditingTable::new(cities_path);
    cities.edit(relation("Near", RELATED, "Cities nearby"));
    let near = property(&cities, "Near").id;
    cities.edit(TableGesture::new("setCell").row("r1").column(near).values(["nh"]));
    provinces.told(&[cities.path.as_path()]);

    let nearby = provinces
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.name == "Cities nearby")
        .expect("the backlink column");
    let lines = provinces.lines();
    assert_eq!(
        vec!["Amsterdam"],
        cell(&lines, "nh", &nearby.id).expect("a cell").labels
    );
}

#[test]
fn deleting_a_two_way_relation_to_the_table_itself_takes_its_backlink_column_with_it() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let before = table.bytes();
    table.edit(relation("Rival of", SELF, "Rivalled by"));
    let rival = property(&table, "Rival of").id;
    table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(rival.clone())
            .values(["r2"]),
    );
    assert!(table.baseline().columns.iter().any(|c| c.name == "Rivalled by"));

    table.edit(TableGesture::new("deleteColumn").column(rival));

    // The file as before the relation, and the backlink column gone from what is shown.
    assert_eq!(before, table.bytes());
    assert!(table.baseline().columns.iter().all(|c| c.name != "Rivalled by"));
}

#[test]
fn when_the_target_of_a_file_made_before_links_is_renamed_the_relation_names_its_new_path_and_its_old_one_when_that_is_undone()
 {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let cities = copy_before_links(&format!("cities{extension}"), folder.path());
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
    // Cities, made before links, relates to a table in a folder; a table in that folder relates to its neighbour.
    let folder = common::folder();
    let cities = copy_before_links("cities.yaml", folder.path());
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
fn when_a_target_named_by_a_link_is_renamed_nothing_is_written_and_the_next_load_names_it_anew() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let cities = copy_plain(&format!("cities{extension}"), folder.path());
        let before = std::fs::read(&cities).expect("read");
        let regions = folder.path().join("regions.yaml");

        // The rename needs no step: the link resolves by the target's document id.
        std::fs::rename(provinces(folder.path()), &regions).expect("renamed");
        assert!(target_rename::renamed(folder.path(), &provinces(folder.path()), &regions).is_empty());
        assert_eq!(before, std::fs::read(&cities).expect("read"));

        // Loaded, it resolves and shows the titles; the link is named anew, and written with the next edit.
        let mut open = EditingTable::new(cities.clone());
        assert_eq!(
            vec!["Noord-Holland"],
            cell(&open.lines(), "r1", "p5").expect("a cell").labels
        );
        assert!(open.baseline().findings.is_empty(), "{:?}", open.baseline().findings);
        assert!(open.session.load_changes().is_some());
        assert_eq!(before, std::fs::read(&cities).expect("read"));
        open.edit(TableGesture::new("setCell").row("r2").column("p5").values(["zh"]));
        let text = String::from_utf8(open.bytes()).expect("UTF-8");
        assert!(
            text.contains("$p7v3k9qd@regions.yaml#nh.Name=Noord-Holland"),
            "{extension}: {text}"
        );
        assert!(
            text.contains("$p7v3k9qd@regions.yaml#zh.Name=Zuid-Holland"),
            "{extension}: {text}"
        );
        assert!(!text.contains("@provinces.yaml"), "{extension}: {text}");
        assert_eq!(
            "$p7v3k9qd@regions.yaml",
            read_table(&cities).column("p5").expect("p5").target_file
        );
    }
}

#[test]
fn a_file_made_before_links_is_converted_when_it_is_loaded_and_written_with_the_next_edit() {
    for extension in EXTENSIONS {
        // The provinces have been given their id already, as loading them would; the cities are as they were.
        let folder = common::folder();
        let cities = copy_before_links(&format!("cities{extension}"), folder.path());
        std::fs::copy(common::examples_folder().join(RELATED), provinces(folder.path())).expect("copied");
        let before = std::fs::read(&cities).expect("read");

        // Loaded for editing: shown converted, nothing written yet.
        let mut table = EditingTable::new(cities.clone());
        assert!(table.session.load_changes().is_some(), "{extension}");
        assert_eq!(before, std::fs::read(&cities).expect("read"));
        assert_eq!(
            vec!["Noord-Holland"],
            cell(&table.lines(), "r1", "p5").expect("a cell").labels
        );

        // The next edit writes the conversion with it: an id, the version that holds links, and links.
        table.edit(TableGesture::new("setCell").row("r2").column("p5").values(["zh"]));
        let converted = read_table(&cities);
        assert_eq!(8, converted.document_id.len(), "{extension}");
        let text = String::from_utf8(table.bytes()).expect("UTF-8");
        assert!(text.contains("0.3"), "{extension}: {text}");
        assert!(
            text.contains("$p7v3k9qd@provinces.yaml#nh.Name=Noord-Holland"),
            "{extension}: {text}"
        );
        assert!(
            text.contains("$p7v3k9qd@provinces.yaml#zh.Name=Zuid-Holland"),
            "{extension}: {text}"
        );
        assert_eq!(PROVINCES_LINK, converted.column("p5").expect("p5").target_file);
        // No related row is lost, and the edit and the conversion are one step.
        assert_eq!(
            vec!["nh"],
            converted.row("r1").expect("r1").cell("p5").expect("p5").values
        );
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes());
    }
}

#[test]
fn a_file_made_before_links_whose_target_has_no_id_yet_keeps_the_relation_as_it_is_and_says_so() {
    let folder = common::folder();
    let cities = copy_before_links("cities.yaml", folder.path());
    let mut table = EditingTable::new(cities.clone());

    // Given its own id; the relation still names a path, shows its titles, and nothing is reported of it.
    table.edit(TableGesture::new("setCell").row("r2").column("p5").values(["zh"]));
    let after = read_table(&cities);
    assert_eq!(8, after.document_id.len());
    assert_eq!(RELATED, after.column("p5").expect("p5").target_file);
    assert_eq!(vec!["zh"], after.row("r2").expect("r2").cell("p5").expect("p5").values);
    assert_eq!(
        vec!["Zuid-Holland"],
        cell(&table.lines(), "r2", "p5").expect("a cell").labels
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
    // A file made before links, whose target has no id: its two-way relation writes both. The edit is accepted and
    // shown, and waits to be written.
    let folder = common::folder();
    let mut table = EditingTable::new(copy_before_links("cities.yaml", folder.path()));
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
            ("relation", PROVINCES_LINK, "one", false, "Cities"),
            (
                region.value_type.as_str(),
                region.target_file.as_str(),
                region.limit.as_str(),
                region.is_computed,
                region.backlink_name.as_str()
            )
        );
        assert_eq!("p3", properties[at + 1].id);
        // Its backlinks show in the provinces, whose file is not written.
        assert_eq!(theirs, std::fs::read(provinces(folder.path())).expect("read"));
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

        // Shown in the provinces too, under a name: its backlinks, which the provinces' file does not hold.
        table.edit(
            TableGesture::new("setRelationOtherSide")
                .column("p5")
                .setting("twoWay", "true")
                .setting("counterpart", "Towns"),
        );
        assert_eq!(
            ("Towns", ""),
            (
                province(&table).backlink_name.as_str(),
                province(&table).counterpart.as_str()
            )
        );
        assert_eq!("Towns", column_settings(&table, "p5")["backlinkName"]);
        let shown = |folder: &Path| {
            EditingTable::new(provinces(folder))
                .baseline()
                .columns
                .into_iter()
                .any(|c| c.name == "Towns" && c.settings.get("computed").is_some_and(|v| v == "true"))
        };
        assert!(shown(folder.path()), "{extension}");

        // In this table only again: no backlinks, and the provinces are what they were throughout.
        table.edit(
            TableGesture::new("setRelationOtherSide")
                .column("p5")
                .setting("twoWay", "false"),
        );
        assert_eq!("", province(&table).backlink_name);
        assert!(!shown(folder.path()));
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
fn a_two_way_relation_to_the_table_itself_shows_its_backlinks_in_the_same_table() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    table.edit(relation("Rival of", SELF, "Rivalled by"));
    let rival = property(&table, "Rival of");
    table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(rival.id.clone())
            .values(["r2"]),
    );

    // One property in the file, a link to the file's own document; its backlinks are a column of their own.
    assert_eq!(
        ("$c4m8x2tr@cities.yaml", "Rivalled by", ""),
        (
            rival.target_file.as_str(),
            rival.backlink_name.as_str(),
            rival.counterpart.as_str()
        )
    );
    assert!(table.on_disk().columns.iter().all(|p| p.name != "Rivalled by"));
    let rivalled = table
        .baseline()
        .columns
        .into_iter()
        .find(|c| c.name == "Rivalled by")
        .expect("the backlink column");
    let lines = table.lines();
    assert_eq!(vec!["Antwerp"], cell(&lines, "r1", &rival.id).expect("a cell").labels);
    assert_eq!(vec!["r1"], cell(&lines, "r2", &rivalled.id).expect("a cell").values);
    assert_eq!(
        vec!["Amsterdam"],
        cell(&lines, "r2", &rivalled.id).expect("a cell").labels
    );
    // The cell holds a link to the row, named by its title.
    let text = String::from_utf8(table.bytes()).expect("UTF-8");
    assert!(text.contains("row: $c4m8x2tr@cities.yaml#r2.Name=Antwerp"), "{text}");
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

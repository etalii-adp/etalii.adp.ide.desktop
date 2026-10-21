//! A selection's options (the standalone host's `KnowledgeOptions.Tests.cs` at 8b6ccfc; knowledge-designer
//! Requirements 3.5 and 3.6): added, renamed, recoloured and reordered without touching any row - rows hold an
//! option's id, never its name - and deleted together with every value that names it, in one step.

mod common;

use std::path::Path;

use adp_designer_knowledge::edits::{OPTION_NAME_NEEDED, colour_for};
use adp_designer_knowledge::{store, validator, values, vocabulary};
use adp_desl::table::content::ContentOption;
use adp_desl::table::model::{Settings, TableGesture};
use common::{EXTENSIONS, EditingTable, copy_plain};

fn open(extension: &str, folder: &Path) -> EditingTable {
    EditingTable::new(copy_plain(&format!("cities{extension}"), folder))
}

fn option(
    kind: &str,
    property: &str,
    option: &str,
    name: Option<&str>,
    colour: Option<&str>,
    index: i64,
) -> TableGesture {
    let mut gesture = TableGesture::new(kind).column(property).target(option).index(index);
    if let Some(name) = name {
        gesture = gesture.values([name]);
    }
    if let Some(colour) = colour {
        gesture = gesture.setting("colour", colour);
    }
    gesture
}

fn options(table: &EditingTable, property: &str) -> Vec<ContentOption> {
    table.on_disk().column(property).expect("the property").options.clone()
}

fn rows_as_text(table: &EditingTable) -> Vec<String> {
    table
        .on_disk()
        .rows
        .iter()
        .map(|r| {
            r.cells
                .iter()
                .map(|c| format!("{}={}", c.column_id, c.values.join(",")))
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect()
}

#[test]
fn a_renamed_option_changes_one_place_in_the_file_and_no_row() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let before = String::from_utf8(table.bytes()).expect("UTF-8");
        let rows = rows_as_text(&table);

        table.edit(option("renameOption", "p2", "o1", Some("The Netherlands"), None, 0));

        // The name where it was, and nothing else.
        assert_eq!(
            before.replace("Netherlands", "The Netherlands"),
            String::from_utf8(table.bytes()).expect("UTF-8"),
            "{extension}"
        );
        assert_eq!(rows, rows_as_text(&table));
    }
}

#[test]
fn options_are_added_recoloured_and_reordered() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());

        table.edit(option("addOption", "p2", "", Some("Luxembourg"), None, 0));
        let added = options(&table, "p2")[6].clone();
        table.edit(option("recolourOption", "p2", &added.id, None, Some("green"), 0));
        table.edit(option("recolourOption", "p2", "o1", None, Some("default"), 0));
        table.edit(option("moveOption", "p2", &added.id, None, None, 0));

        // An id of its own, first of them all, green; and blue given up for the default.
        assert!(!["o1", "o2", "o3", "o4"].contains(&added.id.as_str()));
        assert_eq!(
            vec![
                "Luxembourg:green",
                "Netherlands:default",
                "Belgium:yellow",
                "Germany:default",
                "France:default",
                "Finland:default",
                "Italy:default"
            ],
            options(&table, "p2")
                .iter()
                .map(|o| format!("{}:{}", o.name, o.colour))
                .collect::<Vec<_>>(),
            "{extension}"
        );
        assert!(
            store::read(&table.path)
                .body
                .expect("it reads")
                .model()
                .findings
                .is_empty()
        );
    }
}

#[test]
fn a_deleted_option_takes_every_value_that_names_it_with_it_in_one_step() {
    for extension in EXTENSIONS {
        // Port is a tag of three rows, and v1 filters on Netherlands and folds its group.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        table.edit(
            TableGesture::new("toggleGroup")
                .view("v1")
                .target("o1")
                .setting("collapsed", "true"),
        );
        let before = table.bytes();

        table.edit(option("deleteOption", "p4", "o3", None, None, 0));
        table.edit(option("deleteOption", "p2", "o1", None, None, 0));

        // No option, no value, no condition and no group setting names either any more, and nothing is reported.
        let body = store::read(&table.path).body.expect("it reads");
        let names = |id: &str| -> Vec<String> {
            body.table()
                .column(id)
                .expect("the property")
                .options
                .iter()
                .map(|o| o.name.clone())
                .collect()
        };
        assert_eq!(vec!["Capital"], names("p4"));
        assert_eq!(vec!["Belgium", "Germany", "France", "Finland", "Italy"], names("p2"));
        assert!(!body.model().elements.iter().any(|e| {
            e.attributes
                .values()
                .any(|v| matches!(values::text(Some(v)).as_str(), "o1" | "o3"))
        }));
        let view = body.table().view("v1").expect("v1");
        assert!(view.filter.items.is_empty());
        assert!(view.collapsed.is_empty());
        assert!(body.model().findings.is_empty());
        assert!(validator::validate(body.table(), body.model(), None).is_empty());

        // The other tag of the first row is still its tag.
        assert_eq!(
            vec!["o4"],
            body.table().row("r1").expect("r1").cell("p4").expect("p4").values
        );

        // Each delete is one step.
        table.session.undo().expect("undone");
        table.session.undo().expect("undone");
        assert_eq!(before, table.bytes(), "{extension}");
    }
}

#[test]
fn an_option_some_rows_have_says_how_many_so_its_author_is_told_before_deleting_it() {
    for extension in EXTENSIONS {
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let settings = |table: &EditingTable, id: &str| -> Settings {
            table
                .baseline()
                .columns
                .into_iter()
                .find(|c| c.id == id)
                .expect("the column")
                .settings
        };

        // Port is a tag of three rows, Capital of one; Netherlands is four rows' country, Belgium one's.
        let tags = settings(&table, "p4");
        assert_eq!(("3", "1"), (tags["uses:o3"].as_str(), tags["uses:o4"].as_str()));
        let countries = settings(&table, "p2");
        assert_eq!(
            ("4", "1"),
            (countries["uses:o1"].as_str(), countries["uses:o2"].as_str())
        );

        // An option nobody has says nothing, and the count follows the rows.
        table.edit(option("addOption", "p2", "", Some("Luxembourg"), None, 0));
        table.edit(TableGesture::new("setCell").row("r1").column("p4").values(["o4"]));
        assert_eq!(
            6,
            settings(&table, "p2").keys().filter(|k| k.starts_with("uses:")).count()
        );
        assert_eq!("2", settings(&table, "p4")["uses:o3"]);
    }
}

#[test]
fn a_new_option_starts_with_a_colour_of_the_palette_that_its_property_does_not_have_yet() {
    for extension in EXTENSIONS {
        // Country has a blue and a yellow option, and others without a colour.
        let folder = common::folder();
        let mut table = open(extension, folder.path());
        let taken: Vec<String> = options(&table, "p2")
            .into_iter()
            .map(|o| o.colour)
            .filter(|c| c != "default")
            .collect();

        // One made in the options panel, and one made while filling in a cell.
        table.edit(option("addOption", "p2", "", Some("Luxembourg"), None, 0));
        table.edit(
            TableGesture::new("setCell")
                .row("r2")
                .column("p2")
                .setting("newOption", "Denmark"),
        );

        // Each has a colour of the palette that is not the default and that no option had; and the two differ.
        let made: Vec<ContentOption> = options(&table, "p2")
            .into_iter()
            .filter(|o| o.name == "Luxembourg" || o.name == "Denmark")
            .collect();
        assert_eq!(2, made.len());
        for o in &made {
            assert!(vocabulary::COLOURS.contains(&o.colour.as_str()));
            assert_ne!("default", o.colour);
            assert!(!taken.contains(&o.colour));
        }
        assert_ne!(made[0].colour, made[1].colour);

        // The colour follows from the name: the same option made anywhere starts the same.
        let tags = table.on_disk().column("p4").expect("p4").clone();
        assert_eq!(colour_for(&tags, "Harbour"), colour_for(&tags, "harbour"));
    }
}

#[test]
fn an_option_needs_a_name_that_no_other_option_of_its_property_has_and_a_colour_there_is() {
    let folder = common::folder();
    let mut table = open(".yaml", folder.path());
    let before = table.bytes();
    let mut answer = |gesture: TableGesture| table.begin(gesture).1;

    assert_eq!(
        OPTION_NAME_NEEDED,
        answer(option("addOption", "p2", "", Some("  "), None, 0))
    );
    assert_eq!(
        "'Country' already has an option called 'belgium'.",
        answer(option("addOption", "p2", "", Some("belgium"), None, 0))
    );
    assert_eq!(
        "'Country' already has an option called 'Belgium'.",
        answer(option("renameOption", "p2", "o1", Some("Belgium"), None, 0))
    );
    assert_eq!(
        "'mauve' is not a colour an option can have.",
        answer(option("recolourOption", "p2", "o1", None, Some("mauve"), 0))
    );
    assert_eq!(
        "Only a selection has options.",
        answer(option("addOption", "p3", "", Some("Many"), None, 0))
    );
    assert_eq!(
        "That option is no longer one of this property's.",
        answer(option("renameOption", "p2", "o3", Some("Harbour"), None, 0))
    );
    assert_eq!(before, table.bytes());
    assert!(!table.session.can_undo());
}

//! What a knowledge relation's *Create* adds to a timeline (etalii.adp spec 092, FR-025): the timeline offers a moment
//! and a period, and adds the one asked for, named as typed, beginning today on the first row as an add from a menu
//! without a placement does, a period a week long, after the elements already there and with the rest of the file as
//! it was. An element that holds a link stands for another document's and is not offered.

mod common;

use adp_diagram::element_creation::{NOTHING_TO_CREATE, create_in, diagrams_under};
use adp_diagram_timeline::times;

const PLAIN: &str =
    "timeline: 1\nid: r6d3w8ne\nelements:\n  - id: launch\n    label: Launch\n    begin: 2027-01-04\n    row: 0\n";

fn written(text: &str) -> std::path::PathBuf {
    let folder = tempfile::tempdir().unwrap().keep();
    let path = folder.join("plan.tml");
    std::fs::write(&path, text).unwrap();
    path
}

fn is_new_id(id: &str) -> bool {
    id.len() == 8 && id.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
}

#[test]
fn the_timeline_offers_a_moment_and_a_period_and_no_element_that_holds_a_link() {
    let linked = format!(
        "{PLAIN}  - id: elsewhere\n    link: $s2v5b9hu@services.dgr#pay=Payments\n    begin: 2027-02-01\n    row: 1\n"
    );
    let path = written(&linked);
    let found = diagrams_under(&mut common::tools(), path.parent().unwrap());
    let [timeline] = found.as_slice() else {
        panic!("one timeline: {found:?}")
    };
    let kinds: Vec<(&str, &str)> = timeline
        .kinds
        .iter()
        .map(|k| (k.type_name.as_str(), k.label.as_str()))
        .collect();
    assert_eq!(kinds, [("Moment", "moment"), ("Period", "period")]);
    let ids: Vec<&str> = timeline.indexed.elements.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, ["launch"]);
    assert_eq!(timeline.indexed.document_id.as_deref(), Some("r6d3w8ne"));
    assert_eq!(timeline.indexed.name_property.as_deref(), Some("label"));
}

#[test]
fn a_moment_is_added_today_on_the_first_row_after_the_rest() {
    let created = create_in(&mut common::tools(), &written(PLAIN), "Moment", "Retro").expect("added");
    assert!(is_new_id(&created.id), "{}", created.id);
    assert_eq!(
        String::from_utf8(created.body).unwrap(),
        format!(
            "{PLAIN}  - id: {}\n    label: Retro\n    begin: {}\n    row: 0\n",
            created.id,
            times::today()
        )
    );
}

#[test]
fn a_period_is_added_today_on_the_first_row_a_week_long() {
    let created = create_in(&mut common::tools(), &written(PLAIN), "Period", "Sprint").expect("added");
    assert_eq!(
        String::from_utf8(created.body).unwrap(),
        format!(
            "{PLAIN}  - id: {}\n    label: Sprint\n    begin: {}\n    end: {}\n    row: 0\n",
            created.id,
            times::today(),
            times::days_from_today(7)
        )
    );
}

#[test]
fn a_kind_the_timeline_does_not_offer_adds_nothing() {
    let path = written(PLAIN);
    assert_eq!(
        create_in(&mut common::tools(), &path, "Node", "Retro"),
        Err(NOTHING_TO_CREATE.to_owned())
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), PLAIN);
}

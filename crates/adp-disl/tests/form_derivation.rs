//! The property rows of each bundled definition's inspector forms (DISL section 7.5) (the standalone host's
//! `FormDerivationTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::derivation::forms;
use adp_disl::{DerivedRow, DislDiagram, DislEnv, WireIdMap};
use common::{
    attrs, behavior_diagram, behavior_model, behavior_model_ids, element, hype_cycle, hype_cycle_diagram,
    hype_cycle_ids, loaded, with,
};

fn readable() -> DislEnv {
    DislEnv {
        read_only: false,
        viewpoint: "trueTime".into(),
        now: None,
    }
}

/// One line per row, as the parity transcripts write rows, the candidates inline.
fn lines(rows: &[DerivedRow]) -> Vec<String> {
    rows.iter()
        .map(|row| {
            format!(
                "{} | {} | {} | {} | {} | group {}{}",
                row.id,
                row.label,
                row.value.replace('\n', "\\n"),
                row.widget.as_deref().unwrap_or("-"),
                if row.read_only_reason.is_empty() {
                    "editable".to_owned()
                } else {
                    format!("read-only ({})", row.read_only_reason)
                },
                row.group,
                match &row.candidates {
                    Some(candidates) => format!(" | [{}]", candidates.join(", ")),
                    None => String::new(),
                }
            )
        })
        .collect()
}

fn hype(id: &str, env: Option<DislEnv>) -> Vec<String> {
    let diagram = hype_cycle_diagram();
    lines(&forms::derive(
        &hype_cycle(),
        &element(&diagram, id),
        &env.unwrap_or_else(readable),
        &hype_cycle_ids(),
        "inspector",
    ))
}

#[test]
fn a_trend_shows_its_fields_and_each_drawn_phases_influences() {
    assert_eq!(
        vec![
            "ghg.name | Name | Steam engine | - | editable | group Identity",
            "ghg.description | Description |  | textarea | editable | group Identity",
            "ghg.tags | Tags | energy, industry | tags | editable | group Identity | [energy, industry, transport, law]",
            "ghg.start | Start | 1800-01 | text | editable | group Time",
            "ghg.stop | Stop | 1850-01 | text | editable | group Time",
            "ghg.phases | Phases | All four | slider | editable | group Phases | [Peak, Peak and Trough, Peak, Trough and Slope, All four]",
            "ghg.peak-end | Peak ends | 1812-07 | text | editable | group Phases",
            "ghg.trough-end | Trough ends | 1825-01 | text | editable | group Phases",
            "ghg.slope-end | Slope ends | 1837-07 | text | editable | group Phases",
            "ghg.peak-influences | Influence | None | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Peak",
            "ghg.peak-influenced-by | Influenced by | Watt's patent\\nRailways · Slope | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Peak",
            "ghg.trough-influences | Influence | None | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Trough",
            "ghg.trough-influenced-by | Influenced by | None | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Trough",
            "ghg.slope-influences | Influence | None | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Slope",
            "ghg.slope-influenced-by | Influenced by | None | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Slope",
            "ghg.plateau-influences | Influence | Railways · Trough | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Plateau",
            "ghg.plateau-influenced-by | Influenced by | None | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Plateau",
        ],
        hype("steam", None)
    );
}

#[test]
fn a_hidden_phase_is_listed_only_while_an_influence_attaches_to_it() {
    let rows = hype("rail", None);

    assert!(rows.contains(&"ghg.peak-end | Peak ends | 1830-01 | text | editable | group Phases".to_owned()));
    assert!(!rows.iter().any(|r| r.starts_with("ghg.trough-end")));
    assert!(rows.contains(&"ghg.trough-influenced-by | Influenced by | Steam engine · Plateau | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Trough".to_owned()));
    assert!(rows.contains(&"ghg.slope-influences | Influence | Steam engine · Peak | textarea | read-only (Draw, reattach or delete an influence on the canvas.) | group Slope (hidden)".to_owned()));
    assert!(!rows.iter().any(|r| r.contains("group Plateau")));
}

#[test]
fn a_note_shows_its_text_and_its_size() {
    assert_eq!(
        vec![
            "ghg.text | Text | A remark | textarea | editable | group Identity",
            "ghg.size | Size | 160 x 64.5 | - | editable | group Identity",
        ],
        hype("remark", None)
    );
}

#[test]
fn an_influence_shows_its_ends() {
    assert_eq!(
        vec![
            "ghg.description | Description |  | textarea | editable | group Identity",
            "ghg.from | From | Steam engine · Plateau | - | read-only (Where it is attached; drag the end on the canvas to move it.) | group Ends",
            "ghg.to | To | Railways · Trough | - | read-only (Where it is attached; drag the end on the canvas to move it.) | group Ends",
            "ghg.from-attachment | From attachment | plateau/top/0.5 | - | editable | group Ends",
            "ghg.to-attachment | To attachment | trough/bottom/0.25 | - | editable | group Ends",
        ],
        hype("i1", None)
    );
}

#[test]
fn read_only_gives_every_row_without_a_reason_of_its_own_the_standard_one() {
    let rows = hype(
        "i1",
        Some(DislEnv {
            read_only: true,
            ..readable()
        }),
    );

    assert_eq!(
        "ghg.description | Description |  | textarea | read-only (The graph could not be read, so it cannot be edited.) | group Identity",
        rows[0]
    );
    assert_eq!(
        "ghg.from | From | Steam engine · Plateau | - | read-only (Where it is attached; drag the end on the canvas to move it.) | group Ends",
        rows[1]
    );
}

#[test]
fn a_behavior_node_shows_kind_label_notes_and_place_and_a_retry_its_attempts() {
    let diagram = behavior_diagram();
    let derive = |id: &str| {
        lines(&forms::derive(
            &behavior_model(),
            &element(&diagram, id),
            &DislEnv::default(),
            &behavior_model_ids(),
            "inspector",
        ))
    };

    let check = derive("1.1");
    let retry = derive("1.2");

    assert_eq!(
        vec![
            "abm.kind | Kind | Check | - | editable | group Node | [Do in order, Try in order, Do together, Retry, Repeat until, Only while, Ask approval before, Check, Do, Ask the user, Delegate]",
            "abm.label | Label | Is it known? | - | editable | group Node",
            "abm.notes | Notes |  | textarea | editable | group Node",
            "abm.place | Place | 1.1 | - | read-only (A node's place follows from where it sits in the tree.) | group Node",
        ],
        check
    );
    assert_eq!("abm.attempts | Attempts | 3 | - | editable | group Node", retry[2]);
}

#[test]
fn a_retype_item_offers_the_types_its_options_give_named_by_its_option_label() {
    // Arrange.
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Box": { "attributes": { "name": { "type": "string" } } }, "Crate": { "attributes": { "name": { "type": "string" } } } } },
        "forms": { "thing": { "for": ["Box", "Crate"], "items": [
          { "kind": "type", "label": "Type", "display": { "cel": "self.type" },
            "options": "['Box', 'Crate'].filter(t, t != self.type || self.name == 'keep')", "optionLabel": "item + ' (' + self.name + ')'" },
          { "kind": "computed", "label": "Plain", "value": { "cel": "self.type" } } ] } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let a_box = diagram.add_node("Box", "a", attrs([("name", "A".into())])).unwrap();
    let kept = diagram
        .add_node("Crate", "b", attrs([("name", "keep".into())]))
        .unwrap();

    // Act.
    let rows = forms::derive(
        &specification,
        &a_box,
        &DislEnv::default(),
        &WireIdMap::none(),
        "inspector",
    );
    let kept_rows = forms::derive(
        &specification,
        &kept,
        &DislEnv::default(),
        &WireIdMap::none(),
        "inspector",
    );

    // Assert.
    assert_eq!(
        ("Type", "Box", true, "Crate (A)".to_owned()),
        (
            rows[0].label.as_str(),
            rows[0].value.as_str(),
            rows[0].retypes,
            rows[0].candidates.clone().unwrap().join(", ")
        )
    );
    assert_eq!(
        "Box (keep), Crate (keep)",
        kept_rows[0].candidates.clone().unwrap().join(", ")
    );
    assert_eq!((false, None), (rows[1].retypes, rows[1].candidates.clone()));
}

#[test]
fn an_unset_attribute_shows_its_default_or_nothing() {
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Thing": { "attributes": {
          "name": { "type": "string" }, "size": { "type": "int", "default": 3 }, "when": { "type": "yearMonth" }, "weight": { "type": "number" } } } } },
        "forms": { "thing": { "for": "Thing", "items": [
          { "attribute": "name" }, { "attribute": "size", "label": "Size" }, { "attribute": "when" }, { "attribute": "weight" } ] } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let empty = diagram.add_node("Thing", "a", Map::default()).unwrap();
    let full = diagram
        .add_node(
            "Thing",
            "b",
            attrs([
                ("name", "B".into()),
                ("size", Value::Int(7)),
                ("when", Value::Int(-1)),
                ("weight", Value::Double(2.5)),
            ]),
        )
        .unwrap();
    let derive = |e| {
        lines(&forms::derive(
            &specification,
            e,
            &DislEnv::default(),
            &WireIdMap::none(),
            "inspector",
        ))
    };

    assert_eq!(
        vec![
            "name | name |  | - | editable | group ",
            "size | Size | 3 | - | editable | group ",
            "when | when |  | - | editable | group ",
            "weight | weight |  | - | editable | group "
        ],
        derive(&empty)
    );
    assert_eq!(
        vec![
            "name | name | B | - | editable | group ",
            "size | Size | 7 | - | editable | group ",
            "when | when | -0001-12 | - | editable | group ",
            "weight | weight | 2.5 | - | editable | group "
        ],
        derive(&full)
    );
}

#[test]
fn options_given_as_value_and_label_maps_offer_their_labels_else_their_values() {
    // Arrange: options are values or {value, label, icon, doc} maps (DISL 7.5), from CEL or written out.
    let specification = loaded(&with(
        r#""metamodel": { "types": { "Thing": { "attributes": { "size": { "type": "int" }, "flavour": { "type": "string" } } } } },
        "forms": { "thing": { "for": "Thing", "items": [
          { "attribute": "size", "widget": "select", "options": { "cel": "[{'value': 1, 'label': 'One'}, {'value': 2}, 3]" } },
          { "attribute": "flavour", "widget": "select", "options": [ { "value": "a", "label": "Alpha" }, { "value": "b" }, "c" ] } ] } }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let thing = diagram.add_node("Thing", "a", Map::default()).unwrap();

    // Act.
    let rows = forms::derive(
        &specification,
        &thing,
        &DislEnv::default(),
        &WireIdMap::none(),
        "inspector",
    );

    // Assert.
    assert_eq!(Some(vec!["One".to_owned(), "2".into(), "3".into()]), rows[0].candidates);
    assert_eq!(
        Some(vec!["Alpha".to_owned(), "b".into(), "c".into()]),
        rows[1].candidates
    );
}

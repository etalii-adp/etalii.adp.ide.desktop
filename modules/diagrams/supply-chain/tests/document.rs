//! The format: what the parser reads, what it reports instead of failing, and that every write is a splice which
//! leaves every line it did not mean to touch exactly as it was (the standalone host's `SupplyChainDocumentTests`).

mod common;

use adp_diagram_supply_chain::lines::LineDocument;
use adp_diagram_supply_chain::model::{SupplyChainNode, stages};
use adp_diagram_supply_chain::{parser, writer};
use common::{fixture_path, load, read};
use indexmap::IndexMap;

const LINE_ENDINGS: [&str; 3] = [
    "crlf-line-endings.supply",
    "lf-line-endings.supply",
    "no-trailing-newline.supply",
];

#[test]
fn an_unchanged_document_comes_back_byte_identical() {
    for name in LINE_ENDINGS {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let document = load(name);
        parser::parse(&document);
        assert_eq!(bytes, document.text().into_bytes(), "{name}");
    }
}

#[test]
fn the_small_chain_reads_every_entry() {
    for name in LINE_ENDINGS {
        let model = read(name);
        assert!(model.problems.is_empty(), "{name}: {:?}", model.problems);
        assert_eq!(Some(1), model.version);
        assert_eq!(
            vec!["north"],
            model.groups.iter().map(|g| g.id.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["mine", "plant", "shop"],
            model.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(
            vec!["ore", "goods"],
            model.flows.iter().map(|f| f.id.as_str()).collect::<Vec<_>>()
        );

        let mine = &model.nodes[0];
        assert_eq!(stages::SOURCE, mine.node_type);
        assert_eq!("Mine", mine.name);
        assert_eq!("north", mine.group);
        assert_eq!(Some(10.0), mine.quantity);
        assert_eq!("t", mine.unit);
        assert_eq!(None, mine.step);
        assert!(!mine.is_placed());

        assert_eq!(Some(0.5), model.nodes[1].step);
        assert!(model.nodes[2].is_placed());
        assert_eq!(Some(800.0), model.nodes[2].x);

        let ore = &model.flows[0];
        assert_eq!(
            ("mine", "plant", "Ore", Some(6.0), "t"),
            (
                ore.from.as_str(),
                ore.to.as_str(),
                ore.product.as_str(),
                ore.volume,
                ore.unit.as_str()
            )
        );
    }
}

#[test]
fn every_entry_knows_its_own_lines() {
    let model = read("lf-line-endings.supply");
    let ranges: Vec<(usize, usize)> = model.nodes.iter().map(|n| (n.range.start, n.range.end)).collect();
    assert_eq!(vec![(7, 12), (13, 18), (19, 23)], ranges);
    assert_eq!((3, 5), (model.groups[0].range.start, model.groups[0].range.end));
    assert_eq!((31, 35), (model.flows[1].range.start, model.flows[1].range.end));
}

#[test]
fn malformed_entries_are_reported_and_the_rest_still_reads() {
    let model = read("malformed-entries.supply");
    let said = |text: &str| model.problems.iter().any(|p| p.message.contains(text));
    assert!(said("`quarry` is not a stage"), "{:?}", model.problems);
    assert!(said("`colour`"));
    assert!(said("`quantity: lots`"));
    assert!(said("not a mapping"));
    assert_eq!(
        vec!["mine", "plant"],
        model.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()
    );
    assert_eq!(None, model.nodes[1].quantity);
    assert_eq!(1, model.flows.len());
}

#[test]
fn text_that_is_not_yaml_is_one_problem_and_an_empty_model() {
    let model = read("not-yaml.supply");
    assert_eq!(1, model.problems.len());
    assert!(model.problems[0].message.contains("could not be read as YAML"));
    assert!(model.nodes.is_empty());
}

#[test]
fn a_missing_header_is_reported() {
    let model = parser::parse(&LineDocument::parse("nodes: []\n"));
    assert!(model.problems.iter().any(|p| p.message.contains("does not begin with")));
}

#[test]
fn setting_a_number_changes_that_line_only() {
    let mut document = load("lf-line-endings.supply");
    let before: Vec<String> = document.lines().iter().map(|l| l.text.clone()).collect();
    let mine = parser::parse(&document).nodes[0].clone();

    writer::set_number(&mut document, mine.range, "quantity", Some(12.5));

    let changed: Vec<usize> = (0..before.len()).filter(|i| before[*i] != document.line(*i)).collect();
    assert_eq!(1, changed.len());
    assert_eq!("    quantity: 12.5", document.line(changed[0]));
    assert_eq!(Some(12.5), parser::parse(&document).nodes[0].quantity);
}

#[test]
fn placing_an_unplaced_node_writes_x_before_y_and_keeps_every_other_line() {
    let mut document = load("lf-line-endings.supply");
    let model = parser::parse(&document);
    let places: IndexMap<String, (f64, f64)> =
        [("mine".to_owned(), (100.4, 200.6)), ("shop".to_owned(), (900.0, 50.0))]
            .into_iter()
            .collect();

    writer::place(&mut document, &model, &places).unwrap();

    let reread = parser::parse(&document);
    assert!(reread.problems.is_empty());
    assert_eq!((Some(100.0), Some(201.0)), (reread.nodes[0].x, reread.nodes[0].y));
    assert_eq!((Some(900.0), Some(50.0)), (reread.nodes[2].x, reread.nodes[2].y));
    let x = (0..document.lines().len())
        .find(|i| document.line(*i) == "    x: 100")
        .expect("x is written");
    assert!(x > 0);
    assert_eq!("    y: 201", document.line(x + 1));
    assert!(
        document
            .lines()
            .iter()
            .any(|l| l.text.contains("# a comment the writer must keep"))
    );
}

#[test]
fn adding_a_node_to_a_document_without_the_section_brings_the_section() {
    let mut document = LineDocument::parse("supply-chain: 1\n");
    let model = parser::parse(&document);
    let node = SupplyChainNode {
        id: "n1".into(),
        node_type: stages::PROCESSOR.into(),
        name: "Acme".into(),
        description: String::new(),
        group: String::new(),
        quantity: Some(3.0),
        unit: String::new(),
        step: None,
        x: Some(10.0),
        y: Some(20.0),
        range: Default::default(),
    };

    writer::add_node(&mut document, &model, &node);

    let reread = parser::parse(&document);
    assert!(reread.problems.is_empty(), "{:?}", reread.problems);
    assert_eq!(1, reread.nodes.len());
    let added = &reread.nodes[0];
    assert_eq!(
        ("n1", "Acme", Some(3.0), Some(10.0), Some(20.0)),
        (added.id.as_str(), added.name.as_str(), added.quantity, added.x, added.y)
    );
}

#[test]
fn a_name_that_yaml_would_read_as_something_else_reads_back_as_written() {
    for name in ["[EU]", "yes", "12", "Ore: crushed", "# not a comment", "null"] {
        let mut document = load("lf-line-endings.supply");
        let mine = parser::parse(&document).nodes[0].clone();

        writer::set_text(&mut document, mine.range, "name", name, false);

        let reread = parser::parse(&document);
        assert!(reread.problems.is_empty(), "{name}: {:?}", reread.problems);
        assert_eq!(name, reread.nodes[0].name);
    }
}

#[test]
fn removing_a_group_ungroups_its_members() {
    let mut document = load("lf-line-endings.supply");
    let model = parser::parse(&document);

    writer::remove_group(&mut document, &model, &model.groups[0]);

    let reread = parser::parse(&document);
    assert!(reread.problems.is_empty());
    assert!(reread.groups.is_empty());
    assert!(reread.nodes.iter().all(|n| n.group.is_empty()));
}

#[test]
fn removing_a_node_removes_its_flows() {
    let mut document = load("lf-line-endings.supply");
    let model = parser::parse(&document);

    writer::remove_node(&mut document, &model, &model.nodes[1]);

    let reread = parser::parse(&document);
    assert_eq!(
        vec!["mine", "shop"],
        reread.nodes.iter().map(|n| n.id.as_str()).collect::<Vec<_>>()
    );
    assert!(reread.flows.is_empty());
}

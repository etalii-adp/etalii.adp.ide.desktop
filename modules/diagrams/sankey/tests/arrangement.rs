//! "Arrange diagram" reorders the nodes within their columns so bands cross as little as their values allow, through
//! the store on a real file, and its undo gives back the original bytes. Ported from the standalone host's
//! `EtAlii.Adp.Diagram.Sankey.Tests/SankeyArrangement.Tests.cs` at `8b6ccfc`.

mod common;

use std::fs;

use adp_diagram_sankey::arrangement;
use adp_diagram_sankey::commands::{SankeyCommand, SankeyEdit};
use common::{Dispatcher, TestFolder, fixture, load};

/// Two sources whose bands cross on their way to two ends listed the other way round.
const CROSSING: &str = "sankey: 1\nnodes:\n  - id: a\n    name: Source A\n  - id: b\n    name: Source B\n  - id: x\n    name: End X\n  - id: y\n    name: End Y\nflows:\n  - from: a\n    to: y\n    value: 6\n  - from: b\n    to: x\n    value: 4\n";

fn crossing() -> TestFolder {
    let folder = TestFolder::new(&fixture("lf-line-endings.skv"), "crossing.skv");
    fs::write(&folder.body, CROSSING).unwrap();
    folder
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

#[test]
fn arranging_two_crossing_bands_uncrosses_them_and_the_undo_gives_the_original_bytes() {
    let folder = crossing();
    let mut dispatcher = Dispatcher::default();
    let original = folder.bytes();
    assert!(arrangement::crossings_of(&folder.model()) > 0.0);

    let undo = dispatcher
        .dispatch(&SankeyCommand::new(&folder.body, SankeyEdit::Arrange))
        .unwrap();

    // Nothing crosses, and nothing but the order changed.
    let model = folder.model();
    assert!(model.problems.is_empty());
    assert_eq!(arrangement::crossings_of(&model), 0.0);
    assert_eq!(
        sorted(model.nodes.iter().map(|n| n.id.clone()).collect()),
        ["a", "b", "x", "y"]
    );
    let flows: Vec<String> = model.flows.iter().map(|f| f.id()).collect();
    assert_eq!(flows, ["a->y", "b->x"]);

    dispatcher.dispatch(undo.as_ref()).unwrap();
    assert_eq!(folder.bytes(), original);
}

#[test]
fn arranging_an_arranged_diagram_is_refused_and_writes_nothing() {
    let folder = crossing();
    let mut dispatcher = Dispatcher::default();
    dispatcher.run(&folder.body, SankeyEdit::Arrange);
    let arranged = folder.bytes();

    let second = dispatcher.dispatch(&SankeyCommand::new(&folder.body, SankeyEdit::Arrange));

    assert!(second.err().unwrap().to_lowercase().contains("already arranged"));
    assert_eq!(folder.bytes(), arranged);
}

#[test]
fn arranging_an_example_never_adds_crossings_and_keeps_every_node_and_flow() {
    for example in common::every_example() {
        let folder = TestFolder::new(&example, example.file_name().unwrap().to_str().unwrap());
        let before = load(&folder.body);
        // An example already free of crossings is refused, which is also an arrangement that adds none.
        let _ = Dispatcher::default().dispatch(&SankeyCommand::new(&folder.body, SankeyEdit::Arrange));
        let after = load(&folder.body);
        assert!(after.problems.is_empty());
        assert!(arrangement::crossings_of(&after) <= arrangement::crossings_of(&before));
        assert_eq!(
            sorted(before.nodes.iter().map(|n| n.id.clone()).collect()),
            sorted(after.nodes.iter().map(|n| n.id.clone()).collect())
        );
        let flows = |model: &adp_diagram_sankey::model::SankeyModel| -> Vec<(String, Option<f64>)> {
            model.flows.iter().map(|f| (f.id(), f.value)).collect()
        };
        assert_eq!(flows(&before), flows(&after));
    }
}

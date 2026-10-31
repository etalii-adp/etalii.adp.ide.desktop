//! The flow properties of a causal loop diagram (etalii.adp spec 074, User Story 8): the diagram's own rows with nothing
//! selected, a variable's two rows and a link's weight, each edit written as `contracts/cld-flow-statements.md` gives it,
//! one step of undo, and a file that states only what differs from the defaults.

mod common;

use common::{Diagram, open};

const CORPUS: &str = "causal-loop 1\r\n\r\nvariable a \"A\"\r\nvariable b B\r\n\r\n# Reinforcing.\r\nlink a -> b + \"a note\"\r\nlink b -> a + delayed\r\n\r\nloop R1 \"Round\" a b\r\n";

fn set(diagram: &mut Diagram, id: &str, value: &str) -> Result<(), String> {
    diagram.session.set_diagram_property(&mut diagram.workspace, id, value)
}

fn set_on(diagram: &mut Diagram, element: &str, id: &str, value: &str) -> Result<(), String> {
    diagram
        .session
        .set_property(&mut diagram.workspace, &[element], id, value)
}

fn row(diagram: &Diagram, id: &str) -> String {
    diagram
        .session
        .diagram_properties(diagram.workspace.documents())
        .into_iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
        .value
}

fn element_row(diagram: &Diagram, element: &str, id: &str) -> String {
    diagram
        .session
        .properties(diagram.workspace.documents(), &[element])
        .into_iter()
        .find(|r| r.id == id)
        .unwrap_or_else(|| panic!("no row {id}"))
        .value
}

fn problems_of(diagram: &Diagram) -> Vec<String> {
    diagram
        .session
        .problems(diagram.workspace.documents())
        .into_iter()
        .map(|p| p.message)
        .collect()
}

#[test]
fn the_diagrams_rows_show_their_defaults_where_the_file_states_none() {
    let diagram = open(CORPUS);
    let rows = diagram.session.diagram_properties(diagram.workspace.documents());
    let shown: Vec<(&str, &str, &str, &str)> = rows
        .iter()
        .map(|r| (r.group.as_str(), r.label.as_str(), r.value.as_str(), r.id.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            ("Identity", "Title", "", "causal-loop.diagram-title"),
            ("Flow page", "Link strength", "3", "causal-loop.flow-strength"),
            ("Flow page", "Delay length", "5", "causal-loop.flow-delay"),
            ("Flow page", "Inertia", "1.2", "causal-loop.flow-inertia"),
            ("Flow page", "Simulation speed", "1", "causal-loop.flow-speed"),
            ("Flow page", "Dot density", "3", "causal-loop.flow-density"),
            ("Flow page", "Heatmap reading", "Backlog", "causal-loop.flow-heatmap"),
        ]
    );
}

#[test]
fn a_variable_shows_its_starting_level_and_outside_push() {
    let diagram = open(CORPUS);
    assert_eq!(element_row(&diagram, "variable:a", "causal-loop.variable-start"), "0.5");
    assert_eq!(element_row(&diagram, "variable:a", "causal-loop.variable-push"), "0");
}

#[test]
fn a_title_goes_right_after_the_header_and_a_flow_statement_after_the_title() {
    let mut diagram = open(CORPUS);
    set(&mut diagram, "causal-loop.flow-strength", "4").unwrap();
    assert_eq!(
        diagram.text(),
        CORPUS.replace("causal-loop 1\r\n", "causal-loop 1\r\nflow strength=4\r\n")
    );
    set(&mut diagram, "causal-loop.diagram-title", "Round and round").unwrap();
    assert_eq!(
        diagram.text(),
        CORPUS.replace(
            "causal-loop 1\r\n",
            "causal-loop 1\r\ntitle \"Round and round\"\r\nflow strength=4\r\n"
        )
    );
    assert_eq!(row(&diagram, "causal-loop.diagram-title"), "Round and round");
    assert_eq!(row(&diagram, "causal-loop.flow-strength"), "4");
}

#[test]
fn each_edit_is_one_step_of_undo() {
    let mut diagram = open(CORPUS);
    set(&mut diagram, "causal-loop.flow-delay", "6.5").unwrap();
    assert!(diagram.workspace.undo().is_done());
    assert_eq!(diagram.text(), CORPUS);
    assert_eq!(row(&diagram, "causal-loop.flow-delay"), "5");
}

#[test]
fn every_word_of_the_flow_statement_is_kept_when_another_is_set() {
    let mut diagram = open(CORPUS);
    set(&mut diagram, "causal-loop.flow-heatmap", "Pressure").unwrap();
    set(&mut diagram, "causal-loop.flow-strength", "4").unwrap();
    set(&mut diagram, "causal-loop.flow-delay", "6.5").unwrap();
    set(&mut diagram, "causal-loop.flow-inertia", "2").unwrap();
    set(&mut diagram, "causal-loop.flow-speed", "0.5").unwrap();
    set(&mut diagram, "causal-loop.flow-density", "1.5").unwrap();
    assert_eq!(
        diagram.text(),
        CORPUS.replace(
            "causal-loop 1\r\n",
            "causal-loop 1\r\nflow strength=4 delay=6.5 inertia=2 speed=0.5 density=1.5 heatmap=pressure\r\n"
        )
    );
    assert_eq!(row(&diagram, "causal-loop.flow-heatmap"), "Pressure");
}

#[test]
fn a_property_set_back_to_its_default_leaves_the_file_and_an_empty_statement_goes_with_it() {
    let mut diagram = open(CORPUS);
    set(&mut diagram, "causal-loop.diagram-title", "T").unwrap();
    set(&mut diagram, "causal-loop.flow-strength", "4").unwrap();
    set(&mut diagram, "causal-loop.flow-delay", "6.5").unwrap();
    set(&mut diagram, "causal-loop.flow-strength", "3").unwrap();
    assert_eq!(
        diagram.text(),
        CORPUS.replace(
            "causal-loop 1\r\n",
            "causal-loop 1\r\ntitle \"T\"\r\nflow delay=6.5\r\n"
        )
    );
    set(&mut diagram, "causal-loop.flow-delay", "").unwrap();
    assert_eq!(
        diagram.text(),
        CORPUS.replace("causal-loop 1\r\n", "causal-loop 1\r\ntitle \"T\"\r\n")
    );
    set(&mut diagram, "causal-loop.diagram-title", "").unwrap();
    assert_eq!(diagram.text(), CORPUS);
}

#[test]
fn a_variables_words_are_written_after_its_label_and_nothing_else_moves() {
    let mut diagram = open(CORPUS);
    set_on(&mut diagram, "variable:a", "causal-loop.variable-push", "-1.5").unwrap();
    set_on(&mut diagram, "variable:a", "causal-loop.variable-start", "0.7").unwrap();
    set_on(&mut diagram, "variable:b", "causal-loop.variable-start", "0.3").unwrap();
    assert_eq!(
        diagram.text(),
        CORPUS
            .replace("variable a \"A\"", "variable a \"A\" start=0.7 push=-1.5")
            .replace("variable b B", "variable b B start=0.3")
    );
    set_on(&mut diagram, "variable:a", "causal-loop.variable-start", "0.5").unwrap();
    set_on(&mut diagram, "variable:a", "causal-loop.variable-push", "0").unwrap();
    set_on(&mut diagram, "variable:b", "causal-loop.variable-start", "").unwrap();
    assert_eq!(diagram.text(), CORPUS);
}

#[test]
fn a_variable_without_a_label_keeps_its_words_when_it_gets_one() {
    let mut diagram = open("causal-loop 1\r\nvariable a start=0.7\r\n");
    assert_eq!(element_row(&diagram, "variable:a", "causal-loop.variable-label"), "");
    assert_eq!(element_row(&diagram, "variable:a", "causal-loop.variable-start"), "0.7");
    set_on(&mut diagram, "variable:a", "causal-loop.variable-label", "A").unwrap();
    assert_eq!(diagram.text(), "causal-loop 1\r\nvariable a \"A\" start=0.7\r\n");
}

#[test]
fn a_value_out_of_range_is_refused_with_the_range_and_writes_nothing() {
    let mut diagram = open(CORPUS);
    assert_eq!(
        set(&mut diagram, "causal-loop.flow-strength", "9"),
        Err("Link strength is 9; it must be between 0.5 and 8.".to_owned())
    );
    assert_eq!(
        set_on(&mut diagram, "variable:a", "causal-loop.variable-start", "2"),
        Err("Starting level is 2; it must be between 0 and 1.".to_owned())
    );
    assert_eq!(
        set(&mut diagram, "causal-loop.flow-delay", "wide"),
        Err("Delay length is wide; it must be between 1 and 12.".to_owned())
    );
    assert!(set(&mut diagram, "causal-loop.diagram-title", "say \"so\"").is_err());
    assert_eq!(diagram.text(), CORPUS);
}

#[test]
fn a_file_holding_a_value_out_of_range_opens_with_findings_and_keeps_its_bytes() {
    let text = "causal-loop 1\r\nflow strength=9\r\nvariable a \"A\" start=2\r\n";
    let diagram = open(text);
    let problems = problems_of(&diagram);
    assert!(
        problems.contains(&"Link strength is 9; it must be between 0.5 and 8.".to_owned()),
        "{problems:?}"
    );
    assert!(
        problems.contains(&"Starting level is 2; it must be between 0 and 1.".to_owned()),
        "{problems:?}"
    );
    assert_eq!(diagram.text(), text);
}

#[test]
fn a_flow_statement_with_a_word_it_cannot_read_is_kept_and_reported() {
    let text = "causal-loop 1\r\nflow strength=4 sparkle=2\r\nflow heatmap=hot\r\nvariable a\r\n";
    let diagram = open(text);
    let problems = problems_of(&diagram);
    assert_eq!(
        problems
            .iter()
            .filter(|p| *p == "'flow' is not a statement this reading knows.")
            .count(),
        2,
        "{problems:?}"
    );
    assert_eq!(row(&diagram, "causal-loop.flow-strength"), "3");
    assert_eq!(diagram.text(), text);
}

#[test]
fn of_two_flow_statements_the_last_that_states_a_key_wins_and_an_edit_goes_there() {
    let text = "causal-loop 1\r\nflow strength=4 delay=2\r\nflow strength=5\r\n";
    let mut diagram = open(text);
    assert_eq!(row(&diagram, "causal-loop.flow-strength"), "5");
    assert_eq!(row(&diagram, "causal-loop.flow-delay"), "2");
    set(&mut diagram, "causal-loop.flow-strength", "6").unwrap();
    assert_eq!(
        diagram.text(),
        "causal-loop 1\r\nflow strength=4 delay=2\r\nflow strength=6\r\n"
    );
    set(&mut diagram, "causal-loop.flow-delay", "5").unwrap();
    assert_eq!(
        diagram.text(),
        "causal-loop 1\r\nflow strength=4\r\nflow strength=6\r\n"
    );
    set(&mut diagram, "causal-loop.flow-strength", "3").unwrap();
    assert_eq!(diagram.text(), "causal-loop 1\r\n");
}

#[test]
fn a_file_without_a_header_takes_the_new_statement_first() {
    let mut diagram = open("variable a\r\n");
    set(&mut diagram, "causal-loop.flow-strength", "4").unwrap();
    assert_eq!(diagram.text(), "flow strength=4\r\nvariable a\r\n");
}

#[test]
fn a_header_without_a_line_ending_is_given_one_before_the_new_statement() {
    let mut diagram = open("causal-loop 1");
    set(&mut diagram, "causal-loop.diagram-title", "T").unwrap();
    assert_eq!(diagram.text(), "causal-loop 1\r\ntitle \"T\"");
}

#[test]
fn every_corpus_file_opens_and_writes_nothing_on_a_save() {
    for name in common::corpus() {
        let text = common::fixture(&name);
        let diagram = open(&text);
        assert_eq!(diagram.text(), text, "{name}");
    }
}

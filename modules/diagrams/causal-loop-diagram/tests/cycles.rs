//! The cycles the links form, in the tool's order: the cycle half of the standalone host's `LoopArithmeticTests`, and
//! `CausalLoopCyclesAgainstTheLibraryTests`, at `8b6ccfc`, with the same expectations. DISL 12.4's `diagram.cycles`
//! is `adp-disl`'s; the tool's finder is the module's `cycles::find`, which the plugin function `causalCycles` runs.

mod common;

use adp_diagram_causal_loop::cycles::{self, DEFAULT_BOUND};
use adp_diagram_causal_loop::reading::Model;
use adp_disl::DislElement;
use common::{corpus, fixture, model, model_of, open};

fn find(model: &Model) -> cycles::Cycles {
    cycles::find(model, DEFAULT_BOUND)
}

/// A ring of variables, each linking to the next and the last back to the first, with `polarities` in order.
fn ring(polarities: &[&str]) -> Model {
    let names: Vec<String> = (0..polarities.len()).map(|i| format!("v{i}")).collect();
    let mut statements: Vec<String> = names.iter().map(|n| format!("variable {n}")).collect();
    for (index, polarity) in polarities.iter().enumerate() {
        statements.push(
            format!(
                "link {} -> {} {polarity}",
                names[index],
                names[(index + 1) % names.len()]
            )
            .trim_end()
            .to_owned(),
        );
    }
    model_of(&statements)
}

#[test]
fn a_graph_with_no_cycle_finds_none() {
    let result = find(&model(&["variable a", "variable b", "link a -> b +"]));
    assert!(result.cycles.is_empty());
    assert!(!result.truncated);
}

#[test]
fn a_two_variable_ring_is_one_cycle() {
    assert_eq!(find(&ring(&["+", "+"])).cycles, [["v0", "v1"]]);
}

#[test]
fn a_self_link_is_a_cycle_of_one() {
    assert_eq!(find(&model(&["variable a", "link a -> a +"])).cycles, [["a"]]);
}

#[test]
fn overlapping_cycles_are_found_separately_and_each_only_once() {
    let result = find(&model(&[
        "variable a",
        "variable b",
        "variable c",
        "variable d",
        "link a -> b +",
        "link b -> c +",
        "link c -> a +",
        "link b -> d +",
        "link d -> a +",
    ]));
    assert_eq!(result.cycles.len(), 2);
    assert!(result.cycles.iter().all(|c| c.len() == 3));
    let mut distinct: Vec<String> = result.cycles.iter().map(|c| c.join(",")).collect();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), 2);
}

#[test]
fn nested_cycles_are_both_found() {
    let result = find(&model(&[
        "variable a",
        "variable b",
        "variable c",
        "variable d",
        "link a -> b +",
        "link b -> c +",
        "link c -> d +",
        "link d -> a +",
        "link c -> a +",
    ]));
    let mut lengths: Vec<usize> = result.cycles.iter().map(Vec::len).collect();
    lengths.sort_unstable();
    assert_eq!(lengths, [3, 4]);
}

#[test]
fn a_link_to_an_undeclared_variable_is_not_walked() {
    let result = find(&model(&["variable a", "link a -> nowhere +", "link nowhere -> a +"]));
    assert!(result.cycles.is_empty());
}

#[test]
fn the_search_is_stable_so_the_same_document_always_reports_the_same_loops() {
    let model = model(&[
        "variable a",
        "variable b",
        "variable c",
        "link a -> b +",
        "link b -> c +",
        "link c -> a +",
        "link b -> a +",
    ]);
    assert_eq!(find(&model), find(&model));
}

#[test]
fn the_bound_stops_the_search_and_says_so_rather_than_truncating_silently() {
    let names: Vec<String> = (0..5).map(|i| format!("v{i}")).collect();
    let mut statements: Vec<String> = names.iter().map(|n| format!("variable {n}")).collect();
    for a in &names {
        for b in &names {
            if a != b {
                statements.push(format!("link {a} -> {b} +"));
            }
        }
    }
    let model = model_of(&statements);
    let bounded = cycles::find(&model, 2);
    assert!(bounded.truncated);
    assert_eq!(bounded.cycles.len(), 2);
    assert!(find(&model).cycles.len() > 2);
}

#[test]
fn a_cycle_is_one_signature_however_it_is_rotated() {
    let signature = |c: &[&str]| cycles::canonical_signature(&c.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>());
    assert_eq!(signature(&["b", "c", "a"]), signature(&["a", "b", "c"]));
    assert_ne!(signature(&["a", "c", "b"]), signature(&["a", "b", "c"]));
}

// ---- against the library ---------------------------------------------------------------------------------------------

/// The documents of the corpus on which the two orders differ.
const ORDER_DIFFERS: [&str; 2] = ["lenient-spellings.cld", "weight-forms.cld"];

fn names(cycle: &[DislElement]) -> String {
    cycle
        .iter()
        .map(|v| v.value_of("name").as_str().unwrap_or_default().to_owned())
        .collect::<Vec<_>>()
        .join(" \u{2192} ")
}

#[test]
fn the_library_finds_the_tools_cycles_and_their_order_differs_only_where_named() {
    for relative in corpus() {
        let diagram = open(&fixture(&relative));
        let disl = &diagram
            .workspace
            .documents()
            .get(&diagram.body)
            .unwrap()
            .model()
            .diagram;
        let (found, truncated) = disl.cycles("CausalLink", DEFAULT_BOUND as i64);
        let library: Vec<String> = found.iter().map(|c| names(c)).collect();
        let tool: Vec<String> = find(&diagram.model())
            .cycles
            .iter()
            .map(|c| c.join(" \u{2192} "))
            .collect();
        assert!(!truncated, "{relative}");
        let (mut sorted_tool, mut sorted_library) = (tool.clone(), library.clone());
        sorted_tool.sort();
        sorted_library.sort();
        assert_eq!(sorted_tool, sorted_library, "{relative}");
        let file = relative.rsplit('/').next().unwrap();
        assert_eq!(ORDER_DIFFERS.contains(&file), tool != library, "{relative}");
    }
}

#[test]
fn on_weight_forms_the_library_sorts_where_the_tool_reports_as_it_finds() {
    let diagram = open(&fixture("weight-forms.cld"));
    let disl = &diagram
        .workspace
        .documents()
        .get(&diagram.body)
        .unwrap()
        .model()
        .diagram;
    let library: Vec<String> = disl.cycles("CausalLink", 1000).0.iter().map(|c| names(c)).collect();
    assert_eq!(
        library,
        [
            "a → b",
            "a → b → c",
            "a → b → c → d",
            "a → b → d",
            "a → c",
            "a → c → d",
            "a → c → d → b",
            "b → c → d",
            "b → d"
        ]
    );
    // The tool's order, as `causal-loop-diagram.md` records it.
    let tool: Vec<String> = find(&diagram.model()).cycles.iter().map(|c| c.join(" → ")).collect();
    assert_eq!(
        tool,
        [
            "a → b → c → d",
            "a → b → c",
            "a → b → d",
            "a → b",
            "a → c → d",
            "a → c → d → b",
            "a → c",
            "b → c → d",
            "b → d"
        ]
    );
}

#[test]
fn the_plugin_function_answers_the_specifications_feedback_cycles_in_the_tools_order() {
    // `feedbackCycles()` feeds the unlabelled-loop findings: over lenient-spellings.cld the tool reports a → b before
    // a → indented, where DISL 12.4 gives the reverse, so the first unlabelled cycle reported names b.
    let diagram = open(&fixture("lenient-spellings.cld"));
    let tool: Vec<String> = find(&diagram.model()).cycles.iter().map(|c| c.join(" → ")).collect();
    assert_eq!(tool, ["a → b", "a → indented"]);
}

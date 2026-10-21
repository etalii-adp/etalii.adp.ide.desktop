//! A causal loop diagram's findings as the problems panel shows them: the standalone host's `CausalLoopValidatorTests`
//! at `8b6ccfc`, with the same expectations, asked of the desktop session, whose findings are the specification's with
//! the module's cycle order and its sentences for unread statements; and the four sentences of `unreadable-lines.cld`
//! the standalone parity transcript records (`causal-loop-diagram.md`, gap 1), with `CausalLoopUnreadable`'s choice
//! between them.
//!
//! Not ported: `TheValidatorIsRegistered_ForThisOrigin`, a check of the standalone host's dependency injection; the
//! module is registered by `adp-parity` and the shell as every module is.

mod common;

use adp_diagram::{DiagramProblem, DiagramProblemLocation};
use adp_diagram_causal_loop::unreadable;
use common::{body, fixture, open};

fn judge(statements: &[&str]) -> Vec<DiagramProblem> {
    open(&body(statements)).problems()
}

fn of<'a>(problems: &'a [DiagramProblem], rule: &str) -> Vec<&'a DiagramProblem> {
    problems.iter().filter(|p| p.rule_id == rule).collect()
}

fn single<'a>(problems: &'a [DiagramProblem], rule: &str) -> &'a DiagramProblem {
    let found = of(problems, rule);
    assert_eq!(found.len(), 1, "one {rule} in {problems:?}");
    found[0]
}

#[test]
fn a_label_disagreeing_with_its_arrows_is_reported_with_both_labels() {
    let problems = judge(&[
        "variable a",
        "variable b",
        "link a -> b +",
        "link b -> a -",
        "loop R1 \"claims to reinforce\" a b",
    ]);
    let finding = single(&problems, "causal-loop.label-disagrees");
    for part in [
        "labelled reinforcing",
        "make it balancing",
        "1 negative link",
        "even count",
        "does not assume which",
    ] {
        assert!(finding.message.contains(part), "{part}: {}", finding.message);
    }
}

#[test]
fn an_agreeing_label_is_not_reported() {
    let problems = judge(&[
        "variable a",
        "variable b",
        "link a -> b +",
        "link b -> a -",
        "loop B1 \"self-correcting\" a b",
    ]);
    assert!(of(&problems, "causal-loop.label-disagrees").is_empty());
}

#[test]
fn zero_negative_links_labelled_reinforcing_is_not_reported() {
    let problems = judge(&[
        "variable a",
        "variable b",
        "link a -> b +",
        "link b -> a +",
        "loop R1 \"all positive\" a b",
    ]);
    assert!(of(&problems, "causal-loop.label-disagrees").is_empty());
}

#[test]
fn a_cycle_no_loop_statement_names_is_reported() {
    let problems = judge(&["variable a", "variable b", "link a -> b +", "link b -> a +"]);
    let finding = single(&problems, "causal-loop.unlabelled-loop");
    assert!(finding.message.contains("a \u{2192} b"), "{}", finding.message);
    assert!(finding.message.contains("reinforcing"), "{}", finding.message);
}

#[test]
fn a_cycle_the_document_names_is_not_reported_as_unlabelled_however_it_is_rotated() {
    let problems = judge(&[
        "variable a",
        "variable b",
        "variable c",
        "link a -> b +",
        "link b -> c +",
        "link c -> a +",
        "loop R1 \"named from the middle\" b c a",
    ]);
    assert!(of(&problems, "causal-loop.unlabelled-loop").is_empty());
}

#[test]
fn an_undecidable_loop_says_what_would_decide_it() {
    let problems = judge(&[
        "variable a",
        "variable b",
        "link a -> b +",
        "link b -> a",
        "loop R1 \"unmarked somewhere\" a b",
    ]);
    let finding = single(&problems, "causal-loop.undecidable-loop");
    assert!(finding.message.contains("no stated polarity"), "{}", finding.message);
    assert!(finding.message.contains("+ or -"), "{}", finding.message);
    assert!(of(&problems, "causal-loop.label-disagrees").is_empty());
}

#[test]
fn a_loop_naming_a_path_the_arrows_do_not_close_is_reported() {
    let problems = judge(&["variable a", "variable b", "link a -> b +", "loop R1 \"wishful\" a b"]);
    let finding = single(&problems, "causal-loop.loop-is-not-a-cycle");
    assert!(
        finding.message.contains("do not close into a cycle"),
        "{}",
        finding.message
    );
}

#[test]
fn a_link_to_an_undeclared_variable_is_reported_and_not_drawn() {
    let problems = judge(&["variable a", "link a -> nowhere +"]);
    let finding = single(&problems, "causal-loop.dangling-link");
    assert!(finding.message.contains("'nowhere'"), "{}", finding.message);
    assert!(finding.message.contains("not drawn"), "{}", finding.message);
}

#[test]
fn an_unreadable_line_is_reported_at_its_line() {
    let problems = judge(&["causal-loop 1", "variable a", "nonsense"]);
    let finding = single(&problems, "causal-loop.unreadable-line");
    assert_eq!(finding.location, Some(DiagramProblemLocation::Line(3)));
    assert_eq!(finding.message, "'nonsense' is not a statement this reading knows.");
}

#[test]
fn the_starter_document_judges_clean() {
    // The binding's template, `causal-loop-diagram.fbl#cld`.
    let starter = "causal-loop 1\r\n\r\nvariable population \"Population\"\r\nvariable births \"Births\"\r\n\r\nlink population -> births +\r\nlink births -> population +\r\n\r\nloop R1 \"Births beget births\" population births\r\n";
    assert_eq!(open(starter).problems(), []);
}

#[test]
fn nothing_is_ever_rewritten() {
    let text = "variable a\r\nvariable b\r\nlink a -> b +\r\nlink b -> a -\r\nloop R1 \"wrong\" a b\r\n";
    let diagram = open(text);
    assert!(!diagram.problems().is_empty());
    assert_eq!(diagram.text(), text);
}

// ---- the tool's sentences for what the binding does not read ---------------------------------------------------------

#[test]
fn each_unread_statement_is_worded_as_the_tool_words_it_on_its_line() {
    let problems = open(&fixture("unreadable-lines.cld")).problems();
    let unread: Vec<(Option<DiagramProblemLocation>, &str)> = of(&problems, "causal-loop.unreadable-line")
        .into_iter()
        .map(|p| (p.location.clone(), p.message.as_str()))
        .collect();
    assert_eq!(
        unread,
        [
            (
                Some(DiagramProblemLocation::Line(9)),
                "'bogus' is not a statement this reading knows."
            ),
            (Some(DiagramProblemLocation::Line(10)), unreadable::LINK_SHAPE),
            (
                Some(DiagramProblemLocation::Line(11)),
                "'weight=heavy' does not read as a weight."
            ),
            (
                Some(DiagramProblemLocation::Line(12)),
                "A variable needs an id of at least one character."
            ),
        ]
    );
}

#[test]
fn a_statement_is_worded_by_what_it_starts_like() {
    assert_eq!(
        unreadable::sentence_for("bogus statement"),
        "'bogus' is not a statement this reading knows."
    );
    assert_eq!(unreadable::sentence_for("link a b"), unreadable::LINK_SHAPE);
    assert_eq!(unreadable::sentence_for("link"), unreadable::LINK_SHAPE);
    assert_eq!(
        unreadable::sentence_for("link a -> b weight=heavy"),
        "'weight=heavy' does not read as a weight."
    );
    assert_eq!(
        unreadable::sentence_for("link a -> b weight=1e3 junk"),
        unreadable::LINK_SHAPE,
        "a weight that reads leaves the link's shape to blame"
    );
    assert_eq!(
        unreadable::sentence_for("variable \"\""),
        "A variable needs an id of at least one character."
    );
    assert_eq!(
        unreadable::sentence_for("\"unclosed quote"),
        "'unclosed quote' is not a statement this reading knows."
    );
}

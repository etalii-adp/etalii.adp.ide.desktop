//! What a node's menu, its rows and a parent line do (etalii.adp spec 030, task T074), ported from the standalone
//! host's `AbmCommands.Tests.cs`, `AbmContextActionProvider.Tests.cs` and `AbmValidator.Tests.cs` at `8b6ccfc`, on the
//! pull-request-reviewer example through `adp-diagram`'s session: every edit edits and its undo gives back the original
//! bytes, every refusal writes nothing and says why, a drop goes under the nearest node above it with room, and every
//! rule reports its breach.

mod common;

use adp_context::Prompt;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{DiagramProblemSeverity, DiagramSession};
use adp_diagram_agent_behavior_modelling::actions::{
    ADD_PREFIX, CONNECT_CHILD, EDIT_NOTES, MOVE_EARLIER, MOVE_LATER, REMOVE, RENAME,
};
use adp_diagram_agent_behavior_modelling::layout::{self, NODE_HEIGHT, NODE_WIDTH};
use adp_diagram_agent_behavior_modelling::model::rule_ids;
use common::TestProject;

fn reviewer() -> TestProject {
    TestProject::example("pull-request-reviewer")
}

fn add(kind: &str) -> String {
    format!("{ADD_PREFIX}{kind}")
}

/// The example: 1 Try in order > 1.1 Do in order (skip) > 1.1.1 Check, 1.1.2 Do; 1.2 Do in order (review) >
/// 1.2.1 Do together > 1.2.1.1-3 Do; 1.2.2 Retry > 1.2.2.1 Do; 1.2.3 Do; 1.2.4 approval > 1.2.4.1 Do.
fn edit(project: &mut TestProject, name: &str) -> Result<(), String> {
    let done = |outcome: ActionOutcome| match outcome {
        ActionOutcome::Done => Ok(()),
        ActionOutcome::Prompt(_) => Ok(()),
        ActionOutcome::Refused(refusal) => Err(refusal),
    };
    match name {
        "add a child" => done(project.on("1.2", &add("check"))),
        "add a root" => done(project.execute(ActionTarget::Canvas { x: 0.0, y: 10_000.0 }, &add("action"))),
        "remove a subtree" => {
            match project.on("1.2.1", REMOVE) {
                ActionOutcome::Prompt(Prompt::Confirm { .. }) => {}
                other => return Err(format!("{other:?}")),
            }
            done(project.answer(ActionTarget::Element("1.2.1".into()), REMOVE, ""))
        }
        "rename" => done(project.answer(
            ActionTarget::Element("1.2.3".into()),
            RENAME,
            "Write the review comments",
        )),
        "change the kind" => project.property("1.2.1", "abm.kind", "Do in order"),
        "set a retry's attempts" => project.property("1.2.2", "abm.attempts", "5"),
        "add notes" => project.property("1.1.1", "abm.notes", "Generated files are listed in .gitattributes."),
        "remove notes" => project.property("1.2.3", "abm.notes", ""),
        "move earlier" => done(project.on("1.2", MOVE_EARLIER)),
        "move later" => done(project.on("1.1", MOVE_LATER)),
        "move under another parent" => done(project.execute(
            ActionTarget::Connection {
                source: Some("1.2.1".into()),
                target: Some("1.2.3".into()),
            },
            CONNECT_CHILD,
        )),
        "edit notes" => done(project.answer(ActionTarget::Element("1.2".into()), EDIT_NOTES, "Be kind.")),
        other => panic!("no such edit: {other}"),
    }
}

#[test]
fn every_edit_then_its_undo_gives_the_original_bytes() {
    for name in [
        "add a child",
        "remove a subtree",
        "rename",
        "change the kind",
        "set a retry's attempts",
        "add notes",
        "remove notes",
        "move earlier",
        "move later",
        "move under another parent",
        "edit notes",
    ] {
        let mut project = reviewer();
        let original = project.body();
        edit(&mut project, name).unwrap_or_else(|refusal| panic!("{name}: {refusal}"));
        assert_ne!(project.body(), original, "{name} edited");
        project.undo();
        assert_eq!(project.body(), original, "{name} undone");
    }
}

#[test]
fn every_refusal_writes_nothing_and_says_why() {
    let cases: [(&str, ActionTarget, &str, &str); 4] = [
        (
            "add a child under a leaf",
            ActionTarget::Element("1.1.1".into()),
            "abm.add.action",
            "not available",
        ),
        (
            "add a second child under a wrapper",
            ActionTarget::Element("1.2.2".into()),
            "abm.add.action",
            "not available",
        ),
        (
            "move a node beneath itself",
            ActionTarget::Connection {
                source: Some("1.2.1".into()),
                target: Some("1.2".into()),
            },
            CONNECT_CHILD,
            "beneath itself",
        ),
        (
            "move the first node earlier",
            ActionTarget::Element("1.1".into()),
            MOVE_EARLIER,
            "not available",
        ),
    ];
    for (name, target, action, saying) in cases {
        let mut project = reviewer();
        let original = project.body();
        match project.execute(target, action) {
            ActionOutcome::Refused(refusal) => assert!(refusal.contains(saying), "{name}: {refusal}"),
            other => panic!("{name}: {other:?}"),
        }
        assert_eq!(project.body(), original, "{name}");
    }
    let mut project = reviewer();
    let original = project.body();
    let refusal = project.property("1.2", "abm.kind", "Check").unwrap_err();
    assert!(
        refusal.contains("holds no children") || refusal.contains("cannot become"),
        "{refusal}"
    );
    let refusal = project.property("1.2.2", "abm.attempts", "0").unwrap_err();
    assert_eq!(refusal, "'0' is not a number of attempts; a Retry allows at least 1.");
    assert_eq!(project.body(), original);
}

#[test]
fn an_added_node_is_renamed_by_the_answer_to_its_question() {
    let mut project = reviewer();
    match project.on("1.2", &add("delegate")) {
        ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) => assert_eq!(initial_value, "New task"),
        other => panic!("{other:?}"),
    }
    assert!(project.body().contains("  - **Delegate:** New task\n"));
    assert_eq!(
        project.answer(
            ActionTarget::Element("1.2".into()),
            &add("delegate"),
            "Ask the security reviewer"
        ),
        ActionOutcome::Done
    );
    let model = project.model();
    assert_eq!(model.node("1.2.5").unwrap().label, "Ask the security reviewer");
    // Two edits: the add, and its name.
    project.undo();
    assert_eq!(project.model().node("1.2.5").unwrap().label, "New task");
}

#[test]
fn a_drop_goes_under_the_nearest_node_above_it_that_takes_another_child() {
    let mut project = reviewer();
    let drawn = project.drawn();
    // Just below the review's Do in order.
    let (x, y) = layout::centre(drawn["1.2"]);
    let (x, y) = (x, y + NODE_HEIGHT);
    let (parent, index) = layout::place_drop(&project.model(), &Default::default(), x, y).unwrap();
    assert_eq!(parent.as_deref(), Some("1.2"));
    let outcome = project.execute(ActionTarget::Canvas { x, y }, &add("check"));
    assert!(matches!(outcome, ActionOutcome::Prompt(_)), "{outcome:?}");
    let model = project.model();
    let added = model.node(&format!("1.2.{}", index + 1)).unwrap();
    assert_eq!((added.kind.as_str(), added.label.as_str()), ("check", "New question"));

    // Above every node there is nowhere for it to go.
    let refused = project.execute(ActionTarget::Canvas { x: 0.0, y: -1000.0 }, &add("check"));
    assert!(
        matches!(refused, ActionOutcome::Refused(ref r) if r.starts_with("Drop the node below")),
        "{refused:?}"
    );
}

#[test]
fn the_rows_are_the_definitions_with_the_module_s_ids() {
    let project = reviewer();
    let rows = project.session.properties(project.workspace.documents(), &["1.2.2"]);
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    for id in ["abm.kind", "abm.label", "abm.attempts", "abm.notes", "abm.place"] {
        assert!(ids.contains(&id), "{id} in {ids:?}");
    }
    let menu: Vec<String> = project
        .session
        .actions(project.workspace.documents(), &ActionTarget::Element("1.2".into()))
        .into_iter()
        .flat_map(|g| g.actions)
        .flat_map(|a| std::iter::once(a.id.clone()).chain(a.items.into_iter().flat_map(|g| g.actions).map(|a| a.id)))
        .collect();
    for id in [
        RENAME,
        REMOVE,
        MOVE_EARLIER,
        MOVE_LATER,
        EDIT_NOTES,
        "abm.add.sequence",
        "abm.arrange",
    ] {
        assert!(menu.iter().any(|m| m == id), "{id} in {menu:?}");
    }
}

fn problems_of(text: &str) -> Vec<(String, DiagramProblemSeverity)> {
    let project = TestProject::with(text, "etalii/agent-behavior-modelling\r\n");
    let session: &DiagramSession = &project.session;
    session
        .problems(project.workspace.documents())
        .into_iter()
        .map(|p| (p.rule_id, p.severity))
        .collect()
}

#[test]
fn each_rule_reports_its_breach() {
    for (text, rule, severity) in [
        (
            "## Behavior\n- **Do:** A\n  - **Do:** B\n",
            rule_ids::LEAF_WITH_CHILDREN,
            DiagramProblemSeverity::Error,
        ),
        (
            "## Behavior\n- **Only while:** Ready\n",
            rule_ids::DECORATOR_CHILDREN,
            DiagramProblemSeverity::Error,
        ),
        (
            "## Behavior\n- **Ask approval before:** Post\n  - **Do:** A\n  - **Do:** B\n",
            rule_ids::DECORATOR_CHILDREN,
            DiagramProblemSeverity::Error,
        ),
        (
            "## Behavior\n- **Retry up to 0 times:** Test\n  - **Do:** A\n",
            rule_ids::NO_ATTEMPTS,
            DiagramProblemSeverity::Error,
        ),
        (
            "## Behavior\n- **Do in order:** Nothing\n",
            rule_ids::EMPTY_COMPOSITE,
            DiagramProblemSeverity::Warning,
        ),
        (
            "## Behavior\n- **Do:** A\n- **Do:** B\n",
            rule_ids::SEVERAL_ROOTS,
            DiagramProblemSeverity::Warning,
        ),
        (
            "## Behavior\n- A\n",
            rule_ids::NO_KEYWORD,
            DiagramProblemSeverity::Warning,
        ),
    ] {
        assert_eq!(problems_of(text), [(rule.to_owned(), severity)], "{text:?}");
    }
}

#[test]
fn a_file_with_no_tree_is_information_not_a_problem() {
    for text in ["# Notes\n", "## Behavior\n\nNot a list.\n"] {
        assert_eq!(
            problems_of(text),
            [(rule_ids::NO_BEHAVIOR.to_owned(), DiagramProblemSeverity::Info)],
            "{text:?}"
        );
    }
}

#[test]
fn a_drag_without_a_registration_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    std::fs::write(folder.path().join("agent.md"), common::example("bug-fixer")).unwrap();
    let mut tools = common::tools();
    let mut workspace = adp_diagram::DiagramWorkspace::new(folder.path());
    // A behavior model shares `.md` with every Markdown file, so its body opens only through its registration.
    assert!(workspace.open(&mut tools, std::path::Path::new("agent.md")).is_err());
    let mut project = TestProject::with(&common::example("bug-fixer"), "etalii/agent-behavior-modelling\r\n");
    let (x, y) = project.drawn()["1"];
    project
        .drop_at("1", x, y + NODE_HEIGHT + NODE_WIDTH)
        .expect("with a registration a row is dragged");
}

//! Add from another document on the canvas (etalii.adp spec 092, T024 and SC-003): the mind map's, the timeline's and
//! the dependency graph's lookup, picked and answered through the tab as the shell answers it, and a dependency drawn
//! onto what it added, write the standalone host's bytes, the new ids aside, which take the shape the definitions
//! declare (`persistence.ids`: `nanoid` of 8 characters of `0-9a-z`, after `ID_` in a mind map).

use std::fs;

use adp_context::Prompt;
use adp_diagram::ActionTarget;
use adp_scene::Point;

use super::super::PendingPrompt;
use super::Harness;

const IDEAS: &str = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Shop relaunch\" ID=\"ID_1\">\n<attribute NAME=\"adp-id\" VALUE=\"p4r8t1zc\"/>\n</node>\n</map>\n";
const ROADMAP: &str =
    "timeline: 1\nid: r6d3w8ne\nelements:\n  - id: launch\n    label: Launch\n    begin: 2027-01-04\n    row: 0\n";
const SERVICES: &str =
    "dependencies: 1\nid: s2v5b9hu\nelements:\n  - id: pay\n    label: Payments\n    x: 0\n    row: 0\n";

/// The three documents of one workspace, `open` opened on a canvas.
fn workspace(open: &str) -> Harness {
    let source = tempfile::tempdir().unwrap();
    for (name, text) in [
        ("ideas.mm", IDEAS),
        ("roadmap.tml", ROADMAP),
        ("services.dgr", SERVICES),
    ] {
        fs::write(source.path().join(name), text).unwrap();
    }
    Harness::open_from(source.path(), &["ideas.mm", "roadmap.tml", "services.dgr"], open, open)
}

/// Runs `action` on `target` and picks `link` in the lookup it opens, as the shell's prompt answers it.
fn add_linked(harness: &mut Harness, target: ActionTarget, action: &str, link: &str) {
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(host.workspace(root), target, action);
    let Some(PendingPrompt {
        prompt: Prompt::Choice { options, .. },
        ..
    }) = &harness.tab.prompt
    else {
        panic!("the lookup opens: {:?}", harness.tab.notice);
    };
    let offered: Vec<&str> = options
        .iter()
        .flat_map(|o| o.children.iter().map(|c| c.id.as_str()))
        .collect();
    assert!(offered.contains(&link), "{link} is offered: {offered:?}");
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.answer(host.workspace(root), Some(link));
    assert_eq!(harness.tab.notice, None, "the pick is added");
}

fn body(harness: &Harness, name: &str) -> String {
    fs::read_to_string(harness.root.join(name)).unwrap()
}

/// The id written after `before` and before `after` in `text`.
fn written_id<'a>(text: &'a str, before: &str, after: &str) -> &'a str {
    let start = text.find(before).unwrap_or_else(|| panic!("{before:?} in {text}")) + before.len();
    let length = text[start..]
        .find(after)
        .unwrap_or_else(|| panic!("{after:?} in {text}"));
    &text[start..start + length]
}

fn is_base36(id: &str, length: usize) -> bool {
    id.len() == length && id.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
}

#[test]
fn a_mind_map_adds_a_linked_child_as_the_standalone_host_writes_it() {
    let mut harness = workspace("ideas.mm");
    add_linked(
        &mut harness,
        ActionTarget::Element("ID_1".into()),
        "mindmap.add-linked",
        "$r6d3w8ne@roadmap.tml#launch=Launch",
    );
    let written = body(&harness, "ideas.mm");
    let node = written_id(&written, "<node TEXT=\"Launch\" ID=\"", "\"");
    assert!(
        node.strip_prefix("ID_").is_some_and(|rest| is_base36(rest, 8)),
        "{node}"
    );
    assert_eq!(
        written.replace(node, "{node}"),
        "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"Shop relaunch\" ID=\"ID_1\">\n<attribute NAME=\"adp-id\" VALUE=\"p4r8t1zc\"/>\n<node TEXT=\"Launch\" ID=\"{node}\">\n<attribute NAME=\"adp-link\" VALUE=\"$r6d3w8ne@roadmap.tml#launch=Launch\"/>\n</node>\n</node>\n</map>\n"
    );
}

#[test]
fn a_timeline_adds_a_linked_moment_where_the_canvas_was_clicked_as_the_standalone_host_writes_it() {
    let mut harness = workspace("roadmap.tml");
    // 2027-02-01 15:30 UTC, in seconds since 1970, on row 3.
    let at = ActionTarget::Canvas {
        x: 1_801_495_800.0,
        y: 3.0 * 60.0,
    };
    add_linked(
        &mut harness,
        at,
        "timeline.add-linked",
        "$s2v5b9hu@services.dgr#pay=Payments",
    );
    let written = body(&harness, "roadmap.tml");
    let moment = written_id(&written, "row: 0\n  - id: ", "\n");
    assert!(is_base36(moment, 8), "{moment}");
    assert_eq!(
        written.replace(moment, "{moment}"),
        format!(
            "{ROADMAP}  - id: {{moment}}\n    link: $s2v5b9hu@services.dgr#pay=Payments\n    begin: 2027-02-01\n    row: 3\n"
        )
    );
}

#[test]
fn a_dependency_graph_adds_a_linked_node_and_a_dependency_onto_it_as_the_standalone_host_writes_them() {
    let mut harness = workspace("services.dgr");
    let at = ActionTarget::Canvas {
        x: 240.0,
        y: 3.0 * 60.0,
    };
    add_linked(
        &mut harness,
        at,
        "dependencies.add-linked",
        "$p4r8t1zc@ideas.mm#ID_1=Shop relaunch",
    );
    let node = written_id(&body(&harness, "services.dgr"), "row: 0\n  - id: ", "\n").to_owned();
    assert!(is_base36(&node, 8), "{node}");
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.connect(host.workspace(root), "pay", Some(&node), Point::new(240.0, 180.0));
    assert_eq!(harness.tab.notice, None, "the dependency is drawn");
    let written = body(&harness, "services.dgr");
    let dependency = written_id(&written, "relations:\n  - id: ", "\n");
    assert!(is_base36(dependency, 8), "{dependency}");
    assert_eq!(
        written.replace(dependency, "{dependency}").replace(&node, "{node}"),
        format!(
            "{SERVICES}  - id: {{node}}\n    link: $p4r8t1zc@ideas.mm#ID_1=Shop relaunch\n    x: 240\n    row: 3\nrelations:\n  - id: {{dependency}}\n    from: pay\n    to: {{node}}\n"
        )
    );
}

//! `adp-atd-validate` (etalii.adp spec 035, tasks T121 and T127, FR-041): a case replayed through a description's
//! runtime in memory, every part compared, each difference reported at its step and part with what was expected and
//! what was given.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use adp_atd_check::{Description, FsFiles, Overlay, check};
use adp_atd_runtime::{RuntimeHost, RuntimeTool, diagram_tool};
use adp_atd_validate::{Case, Difference, validate};
use serde_json::json;

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// The mind map's runtime and its description, its root `.atd` replaced by `text` when one is given.
fn runtime(text: Option<String>) -> (RuntimeTool, Description) {
    runtime_of(&diagrams().join("mindmap.atd"), text)
}

/// The runtime and the description of the root `.atd` at `path`, replaced by `text` when one is given.
fn runtime_of(path: &Path, text: Option<String>) -> (RuntimeTool, Description) {
    let path = path.to_path_buf();
    let files = Overlay {
        open: text.map(|t| BTreeMap::from([(path.clone(), t)])).unwrap_or_default(),
        under: &FsFiles,
    };
    let checked = check(&files, &path);
    let tool = diagram_tool(
        &checked.description,
        checked.symbols.as_ref().unwrap(),
        &RuntimeHost::default(),
    )
    .unwrap();
    (tool, checked.description)
}

/// `case` validated against the mind map, its root `.atd` replaced by `text` when one is given.
fn run(text: Option<String>, case: &Case) -> Vec<Difference> {
    let (tool, description) = runtime(text);
    validate(&tool, &description, case)
}

/// A vendored case, read into memory.
fn case(name: &str) -> Case {
    case_in(&diagrams().join("conformance/mindmap").join(name))
}

/// The case in `dir`, read into memory.
fn case_in(dir: &Path) -> Case {
    let name = dir.file_name().unwrap().to_string_lossy().into_owned();
    let fixture = serde_json::from_str(&std::fs::read_to_string(dir.join("conformance.json")).unwrap()).unwrap();
    let files = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| {
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    Case { name, fixture, files }
}

/// Where a difference was attributed, as `file:line` with the file's name alone.
fn short(difference: &Difference) -> String {
    difference.at.as_ref().map_or("nowhere".to_owned(), |at| {
        let file = Path::new(&at.file).file_name().unwrap().to_string_lossy();
        format!("{file}:{}", at.line)
    })
}

/// Each difference as `step:part`, the reading's as `read:part`.
fn places(differences: &[Difference]) -> BTreeSet<String> {
    differences
        .iter()
        .map(|d| match d.step {
            Some(step) => format!("{step}:{}", d.part),
            None => format!("read:{}", d.part),
        })
        .collect()
}

#[test]
fn a_gesture_the_runtime_makes_as_the_reference_did_has_no_difference() {
    // The fold case collapses and expands the central topic (view state, nothing written) and the runtime does so.
    let differences = run(None, &case("fold"));
    let places = places(&differences);
    assert!(
        !places.contains("1:refused") && !places.contains("1:bytes"),
        "{differences:#?}"
    );
    assert!(
        !places.contains("2:refused") && !places.contains("2:bytes"),
        "{differences:#?}"
    );
}

#[test]
fn a_seeded_difference_is_reported_at_its_part_with_what_was_expected_and_given() {
    let text = std::fs::read_to_string(diagrams().join("mindmap.atd")).unwrap();
    let seeded = text.replacen("has an unnamed central topic.", "has a nameless central topic.", 1);
    assert_ne!(seeded, text);
    let differences = run(Some(seeded), &case("unnamed-root"));
    let findings: Vec<&Difference> = differences.iter().filter(|d| d.part == "findings").collect();
    assert_eq!(findings.len(), 1, "{differences:#?}");
    assert_eq!(findings[0].step, None);
    assert!(
        findings[0].expected.contains("has an unnamed central topic."),
        "{}",
        findings[0].expected
    );
    assert!(
        findings[0].actual.contains("has a nameless central topic."),
        "{}",
        findings[0].actual
    );
    // The constraint `unnamedCentralTopic`, whose message the finding is.
    assert_eq!(short(findings[0]), "mindmap.atd:237");
}

/// Each difference names the construct of the description that produced its value (ATD §2.3, `ATD9001`).
#[test]
fn each_difference_is_attributed_to_the_construct_that_produced_it() {
    let (tool, description) = runtime(None);
    let mut attributed = BTreeMap::new();
    // The host's sentence that a deletion is not offered counts as any other: a reason of the description's own shows
    // where a refusal is attributed.
    let mut refusals = case("refusals");
    refusals.fixture["steps"][0]["refused"]["reason"] = json!("The central topic stays.");
    let cases = ["duplicate-id", "example-1", "fold", "not-a-map", "unnamed-root"]
        .map(case)
        .into_iter()
        .chain([refusals]);
    for held in cases {
        let name = held.name.clone();
        for difference in validate(&tool, &description, &held) {
            let place = places(std::slice::from_ref(&difference)).pop_first().unwrap();
            attributed.insert(format!("{name} {place}"), short(&difference));
        }
    }
    let unattributed: Vec<&String> = attributed
        .iter()
        .filter(|(_, at)| *at == "nowhere")
        .map(|(p, _)| p)
        .collect();
    assert!(unattributed.is_empty(), "{unattributed:?}");
    for (place, at) in [
        // The reading, to the binding's rule for the type of the first element that differs.
        ("duplicate-id read:reading", "mindmap.freeplane.atd:67"),
        // A finding, to the constraint that has its code: here the built-in `std.unparseable`.
        ("not-a-map read:findings", "mindmap.atd:226"),
        // A refusal, to the constraint of the gesture's kind: `keepCentralTopic` for a deletion,
        // `reparentWithinTree` for a move.
        ("refusals 1:refused", "mindmap.atd:261"),
        ("refusals 3:refused", "mindmap.atd:269"),
        // A gesture, to its operation (`link`).
        ("example-1 20:refused", "mindmap.atd:306"),
        ("example-1 20:bytes", "mindmap.atd:306"),
        // A drawing, to the notation's entry for the type of the element drawn differently.
        ("fold 1:drawing", "mindmap.notation.atd:58"),
    ] {
        assert_eq!(attributed.get(place).map(String::as_str), Some(at), "{place}");
    }
}

/// Where the mind map's description and the runtime still differ from the cases, case by case, until T132 corrects
/// the description and the engines catch up; a regression or a correction shows here.
#[test]
fn the_mind_maps_cases_differ_only_where_known() {
    let known: [(&str, &[&str]); 6] = [
        // A duplicate id: the module read the later holder under an ephemeral id and reported it, and draws it so.
        ("duplicate-id", &["read:drawing", "read:findings", "read:reading"]),
        // The link picker, a plugin of the host's that this runtime has not, and so the map behind the reference's from
        // step 20 on; and the root's ↗, which the reference did not draw again after an undo.
        (
            "example-1",
            &[
                "11:drawing",
                "14:drawing",
                "18:drawing",
                "19:drawing",
                "20:bytes",
                "20:drawing",
                "20:refused",
                "21:bytes",
                "21:drawing",
                "22:bytes",
                "22:drawing",
            ],
        ),
        // The folded node's ⊕, which the canvas draws from the bound view and the reference
        // recorder, binding no view, did not.
        ("fold", &["1:drawing"]),
        // Two roots: the module refused to read the map, and the runtime still draws the first.
        ("not-a-map", &["read:drawing", "read:findings", "read:reading"]),
        // Moves only a plugin writes: the reasons are the description's own, and it words them differently.
        ("refusals", &["3:refused", "4:refused"]),
        ("unnamed-root", &[]),
    ];
    let (tool, description) = runtime(None);
    let mut unexpected = Vec::new();
    for (name, expected) in known {
        let differences = validate(&tool, &description, &case(name));
        let found = places(&differences);
        let expected: BTreeSet<String> = expected.iter().map(|p| (*p).to_owned()).collect();
        if found != expected {
            unexpected.push(format!("{name}: {found:?}\n  expected {expected:?}"));
        }
    }
    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

#[test]
fn a_case_is_validated_in_memory_and_nothing_is_written() {
    // The case's files are named under a root that does not exist; the runtime reads none of them from disk and
    // writes none, or the replay would have failed to open the body.
    let mut held = case("fold");
    let body = held.files.get("plan.mm").cloned().unwrap();
    held.files.insert("plan.mm".into(), body.clone());
    let _ = run(None, &held);
    assert!(!Path::new("/case").exists());
    assert_eq!(held.files["plan.mm"], body);
}

/// A step's splices are compared with the case's, each edit's in order, its offsets into the body as that edit found
/// it; a difference is attributed to the gesture's operation.
#[test]
fn a_steps_splices_are_compared_and_attributed_to_its_gesture() {
    let (tool, description) = runtime(None);
    let mut one_step = case("example-1");
    let steps = one_step.fixture["steps"].as_array_mut().unwrap();
    steps.truncate(1);
    let with = |splices: serde_json::Value| {
        let mut held = one_step.clone();
        held.fixture["steps"][0]["splices"] = splices;
        validate(&tool, &description, &held)
            .into_iter()
            .filter(|d| d.part == "splices")
            .collect::<Vec<_>>()
    };

    // What the runtime made: the new node, then its text typed into the inline editor.
    let made = with(serde_json::json!([]));
    assert_eq!(made.len(), 1);
    let splices: Vec<serde_json::Value> = serde_json::from_str(&made[0].actual).unwrap();
    assert_eq!(splices.len(), 2, "{splices:?}");
    let mut body = one_step.files["mindmap.mm"].clone();
    for splice in &splices {
        let (start, end) = (
            splice["start"].as_u64().unwrap() as usize,
            splice["end"].as_u64().unwrap() as usize,
        );
        body.splice(start..end, splice["text"].as_str().unwrap().bytes());
    }
    assert_eq!(body, one_step.files["after-1.mm"]);

    assert!(with(serde_json::Value::Array(splices.clone())).is_empty());
    let mut shifted = splices;
    shifted[0]["start"] = serde_json::json!(shifted[0]["start"].as_u64().unwrap() + 1);
    let differences = with(serde_json::Value::Array(shifted));
    assert_eq!(differences.len(), 1);
    assert_eq!(differences[0].step, Some(1));
    // The operation `addChild`.
    assert_eq!(short(&differences[0]), "mindmap.atd:280");
}

/// A refusal is compared exactly, unless both sides are one of the host's own "not offered" sentences (ATD §2.3, spec
/// 067, FR-011): the differences of `refused` when a step made as the case states is expected to give `expected`.
fn refused_when_expecting(expected: &str) -> Vec<Difference> {
    let mut held = case("refusals");
    held.fixture["steps"].as_array_mut().unwrap().truncate(1);
    held.fixture["steps"][0]["refused"]["reason"] = json!(expected);
    run(None, &held).into_iter().filter(|d| d.part == "refused").collect()
}

#[test]
fn the_hosts_sentences_that_a_gesture_is_not_offered_count_as_one_refusal() {
    // The runtime refuses this step with "'mindmap.delete' does not apply to this selection."
    for expected in [
        "Unknown action 'mindmap.delete'.",
        "Unknown action 'another.action:with-more'.",
        "'mindmap.delete' does not apply to this selection.",
        "'another.action' does not apply to this selection.",
        "That action is not available for this item.",
    ] {
        assert_eq!(refused_when_expecting(expected), vec![], "{expected}");
    }
}

#[test]
fn any_other_reason_is_compared_exactly() {
    for expected in [
        "The central topic stays.",
        // Near the host's sentences, and not one of them.
        "Unknown action.",
        "Unknown action 'two words'.",
        "'' does not apply to this selection.",
        "It says 'x' does not apply to this selection.",
        "that action is not available for this item.",
        "That action is not available for this item",
    ] {
        let differences = refused_when_expecting(expected);
        assert_eq!(differences.len(), 1, "{expected}");
        assert!(differences[0].expected.contains(expected));
    }
}

#[test]
fn a_descriptions_own_reason_against_the_hosts_sentence_is_a_difference() {
    // Step 3 is refused by the description's own constraint; a host's sentence is not the same refusal.
    let mut held = case("refusals");
    held.fixture["steps"].as_array_mut().unwrap().truncate(3);
    held.fixture["steps"][2]["refused"]["reason"] = json!("Unknown action 'mindmap.move'.");
    let differences = run(None, &held);
    assert!(places(&differences).contains("3:refused"), "{differences:#?}");
}

/// The dependency graph's runtime and description.
fn dependency_graph() -> (RuntimeTool, Description) {
    runtime_of(&diagrams().join("dependency-graph.atd"), None)
}

/// The dependency graph's `services` case up to its step `last`, its steps from 1 replaced by `gestures`, each keeping
/// what the case states it leaves.
fn services(last: usize, gestures: &[(usize, serde_json::Value)]) -> Case {
    let mut held = case_in(&diagrams().join("conformance/dependency-graph/services"));
    let steps = held.fixture["steps"].as_array_mut().unwrap();
    steps.truncate(last);
    for (number, gesture) in gestures {
        let step = steps[number - 1].as_object_mut().unwrap();
        for key in ["action", "args", "select", "create", "connect", "delete"] {
            step.remove(key);
        }
        step.extend(gesture.as_object().unwrap().clone());
    }
    held
}

/// Steps 13, 16 and 17 of `services`: a connect, a create, and a connect released on empty canvas.
fn gestures_of_services(connect: &str, create: &str) -> Vec<(usize, serde_json::Value)> {
    vec![
        (
            13,
            json!({"connect": {"type": connect, "source": "checkout-web", "target": "identity-service"}}),
        ),
        (16, json!({"create": {"type": create, "at": {"x": 300, "y": 0}}})),
        (
            17,
            json!({
                "connect": {"type": connect, "source": "payments-db", "target": "7yxwlqfi4fcv63bmdakhufj59"},
                "create": {"type": "Node", "at": {"x": 1400, "y": 0}},
            }),
        ),
    ]
}

/// A case's steps replay through the session on the description's own tool (T021): a `create` and a `connect` named by
/// what `atd record` writes, the toolbox entry and the connection's entry, leave the bytes the module left.
#[test]
fn create_and_connect_replay_through_the_descriptions_own_tool() {
    let (tool, description) = dependency_graph();
    let held = services(
        17,
        &gestures_of_services("dependencies.connect", "dependencies.toolbox.node"),
    );
    assert_eq!(validate(&tool, &description, &held), vec![]);
}

/// So do a `create` and a `connect` named by the element and relation types they make.
#[test]
fn create_and_connect_also_take_the_types_they_make() {
    let (tool, description) = dependency_graph();
    let held = services(17, &gestures_of_services("DependsOn", "Node"));
    assert_eq!(validate(&tool, &description, &held), vec![]);
}

#[test]
fn a_create_of_nothing_the_toolbox_has_is_refused_as_not_offered() {
    let (tool, description) = dependency_graph();
    let mut gestures = gestures_of_services("dependencies.connect", "dependencies.toolbox.node");
    gestures[1] = (16, json!({"create": {"type": "Nothing", "at": {"x": 0, "y": 0}}}));
    let held = services(16, &gestures[..2]);
    let differences = validate(&tool, &description, &held);
    let refused = differences.iter().find(|d| d.part == "refused").unwrap();
    assert_eq!(refused.step, Some(16));
    assert_eq!(refused.actual, "refused: Unknown action 'Nothing'.");
}

/// An action on the canvas with no toolbox entry of its own says where in its `args`.
#[test]
fn an_actions_own_place_is_the_canvas_there() {
    let (tool, description) = dependency_graph();
    let mut gestures = gestures_of_services("dependencies.connect", "dependencies.toolbox.node");
    gestures[1] = (
        16,
        json!({"action": "dependencies.add-element", "args": {"at": {"x": 300, "y": 0}}}),
    );
    let held = services(16, &gestures[..2]);
    assert_eq!(validate(&tool, &description, &held), vec![]);
}

#[test]
fn a_delete_removes_what_it_names_and_refuses_what_it_may_not() {
    let (tool, description) = dependency_graph();
    // Step 14 of `services` removes the dependency step 13 made.
    let mut gestures = gestures_of_services("dependencies.connect", "dependencies.toolbox.node");
    gestures.push((14, json!({"delete": ["f2l95kayzmapdgsrek57ftky9"]})));
    let held = services(17, &gestures);
    assert_eq!(validate(&tool, &description, &held), vec![]);

    // The mind map's central topic has no deletion to offer; it is named as what cannot be deleted.
    let mut held = case("refusals");
    held.fixture["steps"].as_array_mut().unwrap().truncate(1);
    let step = held.fixture["steps"][0].as_object_mut().unwrap();
    let root = step["select"][0].clone();
    for key in ["action", "args", "select"] {
        step.remove(key);
    }
    step.insert("delete".into(), json!([root]));
    let differences = run(None, &held);
    assert_eq!(places(&differences), BTreeSet::from(["1:refused".to_owned()]));
    assert!(differences[0].actual.contains("cannot be deleted"), "{differences:#?}");
}

/// A difference at a `create` or a `connect` goes to the operation its entry runs, as an `action`'s does.
#[test]
fn a_create_and_a_connect_are_attributed_to_what_they_run() {
    let (tool, description) = dependency_graph();
    let mut held = services(
        16,
        &gestures_of_services("dependencies.connect", "dependencies.toolbox.node")[..2],
    );
    held.fixture["steps"][15]["expectFile"] = held.fixture["inputs"]["body"].clone();
    let differences = validate(&tool, &description, &held);
    let bytes = differences.iter().find(|d| d.part == "bytes").unwrap();
    // The operation `addNodeHere`.
    assert_eq!(short(bytes), "dependency-graph.atd:201");
}

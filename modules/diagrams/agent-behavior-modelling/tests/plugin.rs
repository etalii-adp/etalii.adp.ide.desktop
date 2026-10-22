//! The persistence plugin `net.etalii.adp.etalii.abmMarkdown` (etalii.adp spec 030, task T074), ported from the
//! standalone host's `Parity/AbmPluginParity.Tests.cs` at `8b6ccfc`: for every corpus document, in its own line
//! endings, in LF, in CRLF, without its final newline and with a byte order mark, and for every edit a node allows,
//! the bytes the plugin plans are the bytes the writer gives on the same text, a refusal is the writer's sentence, and
//! reading the result again gives the parser's model.

mod common;

use adp_cel::{Map, Value};
use adp_diagram_agent_behavior_modelling::kinds;
use adp_diagram_agent_behavior_modelling::lines::LineDocument;
use adp_diagram_agent_behavior_modelling::model::Model;
use adp_diagram_agent_behavior_modelling::plugin::{AbmMarkdownPlugin, GONE};
use adp_diagram_agent_behavior_modelling::{parser, writer};
use adp_fbl::ModelChange;
use adp_fbl::history::apply_in_order;
use adp_fbl::model::FindingSeverity;
use adp_fbl::plugins::{PersistencePlugin, PluginFile, PluginPlanRequest, PluginPlanResult, PluginReadRequest};
use adp_fbl::splice::Operation;
use common::{EXAMPLES, example};

/// Three roots and nesting four deep, with notes, a blank line and a keyword-less item.
const FOREST: &str = "# Agent\n\n## Behavior\n\n- **Do:** A\n  Note on A.\n- **Do in order:** B\n  - **Do:** B1\n\n    Two notes.\n  - **Try in order:** B2\n    - **Do:** B2a\n    - **Retry up to 2 times:** B2b\n      - **Ask the user:** B2b1\n  - **Check:** B3\n- plain item C\n\nAfter the tree.\n";

/// The corpus (`AbmCorpus.cs`): the parser's and the writer's inline documents, the four examples and the forest.
fn corpus() -> Vec<(String, String)> {
    let lines = |lines: &[&str]| lines.join("\n") + "\n";
    let mut corpus: Vec<(String, String)> = vec![
        (
            "inline/parser/reads-the-tree".into(),
            lines(&[
                "# Agent",
                "",
                "## Behavior",
                "",
                "- **Try in order:** Help",
                "  - **Check:** The user asked",
                "  - **Do in order:** Work",
                "    - **Do:** Act",
                "    - **Retry up to 2 times:** Test",
                "      - **Ask the user:** Which test?",
            ]),
        ),
        (
            "inline/parser/reads-notes".into(),
            lines(&[
                "## Behavior",
                "- **Do in order:** Work",
                "  First line of the notes.",
                "",
                "    Indented further.",
                "  - **Do:** Act",
            ]),
        ),
        (
            "inline/parser/no-keyword".into(),
            lines(&["## Behavior", "- Just do the thing"]),
        ),
        (
            "inline/parser/tab-indented-child".into(),
            lines(&["## Behavior", "- **Do in order:** Work", "\t- **Do:** Act"]),
        ),
        (
            "inline/parser/no-section".into(),
            lines(&["# Notes", "- **Do:** Something"]),
        ),
        (
            "inline/writer/lf-tree".into(),
            "## Behavior\n- **Do in order:** Work\n  - **Do:** Act\n".into(),
        ),
        (
            "inline/writer/blank-lines-no-final-newline".into(),
            "## Behavior\n\n- **Do in order:** Work\n\n  - **Do:** Act".into(),
        ),
        (
            "inline/writer/no-tree-yet".into(),
            "# Agent\r\n\r\nNo tree yet.\r\n".into(),
        ),
        (
            "inline/writer/empty-section".into(),
            "## Behavior\n\nNothing yet.\n".into(),
        ),
        ("inline/forest".into(), FOREST.into()),
    ];
    corpus.extend(EXAMPLES.iter().map(|name| (format!("examples/{name}"), example(name))));
    corpus
}

fn variant(text: &str, variant: &str) -> Vec<u8> {
    let lf = text.replace("\r\n", "\n");
    match variant {
        "lf" => lf.into_bytes(),
        "crlf" => lf.replace('\n', "\r\n").into_bytes(),
        "no-final-newline" => text.trim_end_matches(['\r', '\n']).as_bytes().to_vec(),
        "bom" => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        _ => text.as_bytes().to_vec(),
    }
}

type Edit = Box<dyn Fn(&mut LineDocument, &Model) -> Result<(), String>>;

fn set(id: &str, name: &str, value: Value) -> ModelChange {
    ModelChange::Set {
        id: id.to_owned(),
        attributes: [(name, value)].into_iter().collect(),
    }
}

fn attempts(attempts: i64) -> Map {
    [("attempts", Value::Int(attempts))].into_iter().collect()
}

/// Every edit a node allows, as the change the plugin is given and the writer call it stands for.
fn edits(model: &Model) -> Vec<(String, ModelChange, Edit)> {
    let mut edits: Vec<(String, ModelChange, Edit)> = Vec::new();
    let parents: Vec<Option<String>> = std::iter::once(None)
        .chain(model.nodes.iter().map(|n| Some(n.id.clone())))
        .collect();
    let siblings_of = |parent: &Option<String>| match parent {
        None => model.roots().len() as i64,
        Some(id) => model.node(id).unwrap().child_ids.len() as i64,
    };
    for node in &model.nodes {
        let id = node.id.clone();
        let label = "Renamed \u{fc}\nwith a break";
        let i = id.clone();
        edits.push((
            format!("rename {id}"),
            set(&id, "label", Value::String(label.into())),
            Box::new(move |d, m| writer::set_label(d, m.node(&i).unwrap(), label)),
        ));
        let i = id.clone();
        edits.push((
            format!("notes {id}"),
            set(&id, "notes", Value::String("First\n\n  Second".into())),
            Box::new(move |d, m| writer::set_notes(d, m.node(&i).unwrap(), "First\n\n  Second")),
        ));
        let i = id.clone();
        edits.push((
            format!("clear notes {id}"),
            set(&id, "notes", Value::String(String::new())),
            Box::new(move |d, m| writer::set_notes(d, m.node(&i).unwrap(), "")),
        ));
        if node.kind == kinds::RETRY {
            let i = id.clone();
            edits.push((
                format!("attempts {id}"),
                set(&id, "attempts", Value::Int(5)),
                Box::new(move |d, m| writer::set_kind(d, m.node(&i).unwrap(), kinds::RETRY, 5)),
            ));
        }
        let i = id.clone();
        edits.push((
            format!("remove {id}"),
            ModelChange::Remove { id: id.clone() },
            Box::new(move |d, m| writer::remove(d, m.node(&i).unwrap())),
        ));
        for kind in kinds::ALL {
            let count = if kind.id == kinds::RETRY { 3 } else { 0 };
            let i = id.clone();
            edits.push((
                format!("retype {id} {}", kind.id),
                ModelChange::Retype {
                    id: id.clone(),
                    type_name: kind.type_name.into(),
                    attributes: attempts(i64::from(count)),
                },
                Box::new(move |d, m| writer::set_kind(d, m.node(&i).unwrap(), kind.id, count)),
            ));
        }
        let i = id.clone();
        edits.push((
            format!("retype {id} retry 0"),
            ModelChange::Retype {
                id: id.clone(),
                type_name: "Retry".into(),
                attributes: attempts(0),
            },
            Box::new(move |d, m| writer::set_kind(d, m.node(&i).unwrap(), kinds::RETRY, 0)),
        ));
        for parent in &parents {
            for index in -1..=siblings_of(parent) + 1 {
                let (i, p) = (id.clone(), parent.clone());
                edits.push((
                    format!("move {id} to {} at {index}", parent.as_deref().unwrap_or("root")),
                    ModelChange::Move {
                        id: id.clone(),
                        new_parent_id: parent.clone(),
                        index,
                    },
                    Box::new(move |d, m| {
                        writer::move_node(d, m, m.node(&i).unwrap(), p.as_deref().and_then(|p| m.node(p)), index)
                    }),
                ));
            }
        }
    }
    for parent in &parents {
        for index in -1..=siblings_of(parent) + 1 {
            let kinds_here: Vec<&kinds::Kind> = if index <= 0 {
                kinds::ALL.iter().collect()
            } else {
                vec![kinds::of(kinds::ACTION)]
            };
            for kind in kinds_here {
                let label = format!("New {}", kind.id);
                let p = parent.clone();
                let written = label.clone();
                edits.push((
                    format!(
                        "add {} under {} at {index}",
                        kind.id,
                        parent.as_deref().unwrap_or("root")
                    ),
                    ModelChange::Add {
                        type_name: kind.type_name.into(),
                        id: None,
                        attributes: [("label", Value::String(label))].into_iter().collect(),
                        parent_id: parent.clone(),
                        index: Some(index),
                        slot: None,
                    },
                    Box::new(move |d, m| {
                        writer::add(d, m, p.as_deref().and_then(|p| m.node(p)), index, kind.id, &written, 0).map(|_| ())
                    }),
                ));
            }
        }
    }
    edits.push((
        "add gone".into(),
        ModelChange::Add {
            type_name: "Do".into(),
            id: None,
            attributes: Map::default(),
            parent_id: Some("9.9".into()),
            index: None,
            slot: None,
        },
        Box::new(|_, _| Err(GONE.to_owned())),
    ));
    edits.push((
        "remove gone".into(),
        ModelChange::Remove { id: "9.9".into() },
        Box::new(|_, _| Err(GONE.to_owned())),
    ));
    edits
}

/// The writer's path: the text decoded, the writer's edit on its lines, and their bytes.
fn writer_bytes(bytes: &[u8], edit: &Edit) -> (Result<(), String>, Vec<u8>) {
    let mut document = LineDocument::parse(&String::from_utf8_lossy(bytes));
    let model = parser::parse(&document);
    let outcome = edit(&mut document, &model);
    let after = if outcome.is_ok() {
        document.text().into_bytes()
    } else {
        bytes.to_vec()
    };
    (outcome, after)
}

/// The parser's model of `bytes` as the plugin should read it, one line per node.
fn parsed(bytes: &[u8]) -> String {
    common::parse(&String::from_utf8_lossy(bytes))
        .nodes
        .iter()
        .map(|n| {
            let mut line = format!(
                "{} {} {} line {} label={} notes={}",
                n.id,
                kinds::of(&n.kind).type_name,
                n.parent_id.clone().unwrap_or_default(),
                n.line + 1,
                n.label,
                n.notes
            );
            if n.kind == kinds::RETRY {
                line += &format!(" attempts={}", n.retry_count);
            }
            if n.kind == kinds::ACTION {
                line += &format!(" implicit={}", !n.has_keyword);
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// What the plugin reads from `bytes`, in the same shape.
fn reread(bytes: &[u8]) -> String {
    let read = AbmMarkdownPlugin.read(&PluginReadRequest {
        files: vec![PluginFile::new("", bytes.to_vec())],
        ..PluginReadRequest::default()
    });
    read.elements
        .iter()
        .map(|e| {
            let text = |name: &str| match e.attributes.get(name) {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            let mut line = format!(
                "{} {} {} line {} label={} notes={}",
                e.id,
                e.type_name,
                e.parent_id.clone().unwrap_or_default(),
                e.line,
                text("label"),
                text("notes")
            );
            if let Some(Value::Int(attempts)) = e.attributes.get("attempts") {
                line += &format!(" attempts={attempts}");
            }
            if let Some(Value::Bool(implicit)) = e.attributes.get("implicit") {
                line += &format!(" implicit={implicit}");
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The bytes the plugin plans for `change` on `bytes`, or its refusal.
fn planned(bytes: &[u8], change: &ModelChange) -> Result<Vec<u8>, String> {
    let files = [PluginFile::new("", bytes.to_vec())];
    let last = AbmMarkdownPlugin.read(&PluginReadRequest {
        files: files.to_vec(),
        ..PluginReadRequest::default()
    });
    match AbmMarkdownPlugin.plan(&PluginPlanRequest {
        files: &files,
        last: &last,
        change,
    }) {
        PluginPlanResult::Planned(splices) => {
            let splices: Vec<_> = splices.into_iter().map(|s| s.splice).collect();
            apply_in_order(bytes, &splices)
        }
        PluginPlanResult::Refused(refusal) => Err(refusal),
    }
}

#[test]
fn every_edit_writes_the_writers_bytes_refuses_in_its_words_and_rereads_as_the_parser_reads() {
    for (name, text) in corpus() {
        for kind in ["as-is", "lf", "crlf", "no-final-newline", "bom"] {
            let bytes = variant(&text, kind);
            let model = common::parse(&String::from_utf8_lossy(&bytes));
            let mut differences = Vec::new();
            let all = edits(&model);
            for (label, change, edit) in &all {
                let (expected, expected_bytes) = writer_bytes(&bytes, edit);
                match (planned(&bytes, change), expected) {
                    (Ok(after), Ok(())) if after != expected_bytes => {
                        differences.push(format!("{label}: bytes differ"))
                    }
                    (Ok(after), Ok(())) if reread(&after) != parsed(&expected_bytes) => {
                        differences.push(format!("{label}: the reading differs from the parser's"))
                    }
                    (Ok(_), Ok(())) => {}
                    (Err(refusal), Err(said)) if refusal == said => {}
                    (outcome, said) => differences.push(format!("{label}: {outcome:?} where the writer said {said:?}")),
                }
            }
            assert!(all.len() > 20, "{name} {kind}: only {} edits were tried", all.len());
            assert!(
                differences.is_empty(),
                "{name} {kind}: {} of {} edits differ:\n{}",
                differences.len(),
                all.len(),
                differences.iter().take(5).cloned().collect::<Vec<_>>().join("\n")
            );
        }
    }
}

#[test]
fn the_reading_is_the_parsers_model_with_ids_the_definition_derives() {
    for (name, text) in corpus() {
        for kind in ["as-is", "lf", "crlf", "no-final-newline", "bom"] {
            let bytes = variant(&text, kind);
            assert_eq!(reread(&bytes), parsed(&bytes), "{name} {kind}");
            let read = AbmMarkdownPlugin.read(&PluginReadRequest {
                files: vec![PluginFile::new("", bytes.clone())],
                ..PluginReadRequest::default()
            });
            assert!(read.elements.iter().all(|e| !e.id_is_stored), "{name} {kind}");
        }
    }
}

#[test]
fn every_example_is_a_tree_of_its_own() {
    for name in EXAMPLES {
        let size = common::parse(&example(name)).nodes.len();
        assert!(size >= 10, "{name} reads {size} nodes");
    }
}

#[test]
fn an_elements_span_is_its_subtrees_bytes() {
    let bytes = FOREST.replace('\n', "\r\n").into_bytes();
    let read = AbmMarkdownPlugin.read(&PluginReadRequest {
        files: vec![PluginFile::new("", bytes.clone())],
        ..PluginReadRequest::default()
    });
    let b2 = read.find("2.2").unwrap();
    assert_eq!(
        String::from_utf8_lossy(&bytes[b2.own_span.start..b2.own_span.end]),
        "  - **Try in order:** B2\r\n    - **Do:** B2a\r\n    - **Retry up to 2 times:** B2b\r\n      - **Ask the user:** B2b1\r\n"
    );
    assert_eq!(b2.line, 11);
    assert_eq!(
        (
            b2.type_name.as_str(),
            b2.parent_id.as_deref(),
            b2.parent_slot.as_deref()
        ),
        ("Fallback", Some("2"), Some("children"))
    );
}

#[test]
fn a_move_is_planned_as_the_subtree_taken_out_and_put_in_again() {
    let files = [PluginFile::new("", FOREST.as_bytes().to_vec())];
    let last = AbmMarkdownPlugin.read(&PluginReadRequest {
        files: files.to_vec(),
        ..PluginReadRequest::default()
    });
    let plan = AbmMarkdownPlugin.plan(&PluginPlanRequest {
        files: &files,
        last: &last,
        change: &ModelChange::Move {
            id: "2.2".into(),
            new_parent_id: None,
            index: 0,
        },
    });
    let PluginPlanResult::Planned(splices) = plan else {
        panic!("{plan:?}")
    };
    let operations: Vec<Operation> = splices.iter().map(|s| s.splice.operation).collect();
    assert_eq!(operations, [Operation::InsertEntry, Operation::RemoveEntry]);
}

#[test]
fn a_no_behavior_file_is_reported_on_its_first_line_and_an_empty_section_on_its_heading() {
    let findings = |text: &str| {
        AbmMarkdownPlugin
            .read(&PluginReadRequest {
                files: vec![PluginFile::new("", text.as_bytes().to_vec())],
                ..PluginReadRequest::default()
            })
            .findings
            .iter()
            .map(|f| {
                format!(
                    "{} {} {}",
                    f.code,
                    if f.severity == FindingSeverity::Info {
                        "Info"
                    } else {
                        "?"
                    },
                    f.location.as_ref().unwrap().line
                )
            })
            .collect::<Vec<_>>()
            .join(";")
    };
    assert_eq!(findings("# Notes\n\nNothing here.\n"), "abm.no-behavior Info 1");
    assert_eq!(
        findings("# Agent\n\n## Behavior\n\nNo list yet.\n"),
        "abm.no-behavior Info 3"
    );
}

#[test]
fn a_new_body_is_the_module_s_empty_document() {
    let bytes = AbmMarkdownPlugin.template(&adp_fbl::plugins::PluginTemplateRequest {
        name: "Agent".into(),
        placeholders: Default::default(),
    });
    assert_eq!(bytes, writer::empty_document("Agent", "\r\n").into_bytes());
}

#[test]
fn the_kinds_are_the_types_the_specification_maps_them_to() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../vendor/etalii.adp/definitions/diagrams/agent-behavior-modelling.dis");
    let specification: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let types = specification["x-abm"]["types"].as_object().unwrap();
    for kind in kinds::ALL {
        assert_eq!(types[kind.type_name].as_str(), Some(kind.id), "{}", kind.type_name);
    }
}

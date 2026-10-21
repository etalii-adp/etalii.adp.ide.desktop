//! What a session asks a tool type's own code, and what it does for every tool type (etalii.adp spec 030, FR-030 and
//! FR-031; task T060): an operation that asks for its parameter before it runs (DISL section 9.3), a finished connect
//! gesture's menu entry and its connect constraints (sections 7.3 and 8.4), a label bound to an attribute renamed
//! through it (section 6.12), and the module hook's coordinates, positions, plugin layout, label attribute and
//! written changes.
//!
//! The tool type is the vendored timeline (`definitions/diagrams/timeline.dis`), whose plugin functions this test does
//! not implement: its fallbacks read ISO 8601 times, which is all the body below holds. The timeline's real module is
//! `modules/diagrams/timeline`; the probe here stands in for any module.

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_context::Prompt;
use adp_core::Catalogue;
use adp_diagram::{
    ActionOutcome, ActionTarget, DiagramModule, DiagramSession, DiagramTools, DiagramWorkspace, ModulePlacement,
};
use adp_disl::behavior::ids;
use adp_disl::{DislChange, DislDiagram, DislElement, DislPluginFunctions, DislSpecification};

const ORIGIN: &str = "generic/timeline";

const BODY: &str = "timeline: 1\r\n# A comment no edit touches.\r\nelements:\r\n  - id: aaa\r\n    label: Period\r\n    begin: 2026-01-05\r\n    end: 2026-02-13\r\n    row: 0\r\n  - id: bbb\r\n    label: Moment\r\n    begin: 2026-02-16T14:00:00\r\n    row: 2\r\nconnections:\r\n  - id: ccc\r\n    from: aaa\r\n    to: bbb\r\n    label: gates\r\n";

/// A module that places a node 100 units across per row, writes a move's x back as the row, reads a canvas point as
/// a date and a row, arranges by putting `aaa` on row 7, renames a computed label through `label`, and leaves a
/// retype out of what is written.
#[derive(Debug)]
struct Probe;

impl DiagramModule for Probe {
    fn position(&self, element: &DislElement) -> Option<(f64, f64)> {
        let row = element.value_of("row").as_number().unwrap_or_default();
        Some((row * 100.0, 7.0))
    }

    fn placement(&self, element: &DislElement, x: f64, _y: f64) -> Option<Result<ModulePlacement, String>> {
        if x < 0.0 {
            return Some(Err(format!("{} cannot go before the first row.", element.id())));
        }
        let mut attributes = Map::default();
        attributes.insert("row", Value::Int((x / 100.0) as i64));
        // The timeline's placement rule reads newBounds.x2 (`noInversionByGesture`).
        let bounds: Map = [("x", Value::Double(x)), ("x2", Value::Double(x))]
            .into_iter()
            .collect();
        Some(Ok(ModulePlacement {
            attributes,
            old_bounds: bounds.clone(),
            new_bounds: bounds,
        }))
    }

    fn position_at(&self, _x: f64, y: f64) -> Option<Map> {
        Some(
            [("x", Value::from("2026-05-01")), ("y", Value::Int(y as i64))]
                .into_iter()
                .collect(),
        )
    }

    fn layout(
        &self,
        _specification: &DislSpecification,
        plugin: &str,
        diagram: &DislDiagram,
    ) -> Option<Result<Vec<DislChange>, String>> {
        (plugin == "net.etalii.adp.generic.timelineRowArrange").then(|| {
            let aaa = diagram.element_by_id("aaa").ok_or("aaa is gone")?;
            let mut attributes = Map::default();
            attributes.insert("row", Value::Int(7));
            Ok(vec![DislChange::Set {
                element_id: aaa.id(),
                type_name: aaa.type_name(),
                attributes,
            }])
        })
    }

    fn label_attribute(&self, _element: &DislElement, label: &str) -> Option<String> {
        (label == "label").then(|| "label".to_owned())
    }

    fn written(&self, _diagram: &DislDiagram, changes: Vec<DislChange>) -> Result<Vec<DislChange>, String> {
        Ok(changes
            .into_iter()
            .filter(|c| !matches!(c, DislChange::Retype { .. }))
            .collect())
    }
}

struct Open {
    _folder: tempfile::TempDir,
    body: PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

fn open(module: Option<Rc<dyn DiagramModule>>) -> Open {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("plan.tml"), BODY).unwrap();
    fs::write(folder.path().join("plan.adp"), "generic/timeline\r\n").unwrap();
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    let mut tools = DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none());
    if let Some(module) = module {
        tools = tools.with_module(ORIGIN, module);
    }
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools, Path::new("plan.adp")).unwrap();
    session.baseline(workspace.documents()).unwrap();
    Open {
        body: folder.path().join("plan.tml"),
        _folder: folder,
        workspace,
        session,
    }
}

fn probe() -> Option<Rc<dyn DiagramModule>> {
    Some(Rc::new(Probe))
}

fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

impl Open {
    fn text(&self) -> String {
        fs::read_to_string(&self.body).unwrap()
    }

    fn execute(&mut self, target: &ActionTarget, action: &str, ids: &[&str]) -> ActionOutcome {
        self.session
            .execute(&mut self.workspace, target, action, &ids::fixed(ids))
    }

    fn answer(&mut self, target: &ActionTarget, action: &str, value: &str) -> ActionOutcome {
        self.session.answer(&mut self.workspace, target, action, value)
    }
}

#[test]
fn an_operation_that_asks_prompts_for_its_parameter_opened_on_its_default() {
    let mut open = open(probe());

    let prompt = open.execute(&element("bbb"), "timeline.give-end", &[]);

    let ActionOutcome::Prompt(Prompt::Input {
        title,
        label,
        initial_value,
        ..
    }) = prompt
    else {
        panic!("not asked: {prompt:?}");
    };
    assert_eq!(
        ("Give it an end", "End", "2026-02-16T14:00:00"),
        (title.as_str(), label.as_str(), initial_value.as_str())
    );
    assert_eq!(BODY, open.text(), "nothing is written before it is answered");
}

#[test]
fn an_answered_operation_runs_with_its_parameter_and_is_one_undo() {
    let mut open = open(probe());

    let refused = open.answer(&element("bbb"), "timeline.give-end", "later");
    let answered = open.answer(&element("bbb"), "timeline.give-end", "2026-03-01T09:00:00");

    assert_eq!(
        ActionOutcome::Refused("'later' is not a time this timeline can read.".into()),
        refused
    );
    assert_eq!(ActionOutcome::Done, answered);
    assert_eq!(
        BODY.replace(
            "    begin: 2026-02-16T14:00:00\r\n",
            "    begin: 2026-02-16T14:00:00\r\n    end: 2026-03-01T09:00:00\r\n"
        ),
        open.text()
    );
    assert!(open.workspace.undo().is_done());
    assert_eq!(BODY, open.text());
}

#[test]
fn a_retype_is_written_as_the_binding_writes_it_unless_the_module_leaves_it_out() {
    let mut generic = open(None);

    let refused = generic.answer(&element("bbb"), "timeline.give-end", "2026-03-01T09:00:00");

    assert_eq!(
        ActionOutcome::Refused(
            "This file's binding cannot change an element's type; only a persistence plugin can.".into()
        ),
        refused
    );
    let mut probed = open(probe());
    assert_eq!(
        ActionOutcome::Done,
        probed.execute(&element("aaa"), "timeline.remove-end", &[])
    );
    assert_eq!(BODY.replace("    end: 2026-02-13\r\n", ""), probed.text());
}

#[test]
fn a_finished_connect_gestures_entry_draws_the_relation_its_menu_names() {
    let mut open = open(None);
    let gesture = ActionTarget::Connection {
        source: Some("bbb".into()),
        target: Some("aaa".into()),
    };

    let outcome = open.execute(&gesture, "timeline.connect", &["ddd"]);

    assert_eq!(ActionOutcome::Done, outcome);
    assert_eq!(
        format!("{BODY}  - id: ddd\r\n    from: bbb\r\n    to: aaa\r\n"),
        open.text()
    );
}

#[test]
fn a_connect_gesture_is_refused_by_the_specifications_connect_constraints() {
    let mut open = open(None);
    let gesture = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: Some("aaa".into()),
    };
    let unfinished = ActionTarget::Connection {
        source: Some("aaa".into()),
        target: None,
    };

    let refused = open.execute(&gesture, "timeline.connect", &["ddd"]);
    let unfinished = open.execute(&unfinished, "timeline.connect", &["ddd"]);

    assert_eq!(
        ActionOutcome::Refused("An element cannot be connected to itself.".into()),
        refused
    );
    assert!(matches!(unfinished, ActionOutcome::Refused(_)), "{unfinished:?}");
    assert_eq!(BODY, open.text());
}

#[test]
fn a_relations_label_bound_to_an_attribute_is_renamed_through_it() {
    let mut open = open(None);

    let prompt = open.execute(&element("ccc"), "timeline.relabel", &[]);
    let answered = open.answer(&element("ccc"), "timeline.relabel", "informs");

    assert!(
        matches!(&prompt, ActionOutcome::Prompt(Prompt::Input { initial_value, .. }) if initial_value == "gates"),
        "{prompt:?}"
    );
    assert_eq!(ActionOutcome::Done, answered);
    assert_eq!(BODY.replace("label: gates", "label: informs"), open.text());
}

#[test]
fn a_computed_label_is_renamed_through_the_attribute_the_module_names() {
    let mut generic = open(None);
    let mut probed = open(probe());

    let refused = generic.answer(&element("aaa"), "timeline.rename", "Renamed");
    let renamed = probed.answer(&element("aaa"), "timeline.rename", "Renamed");

    assert_eq!(ActionOutcome::Refused("'label' cannot be edited here.".into()), refused);
    assert_eq!(ActionOutcome::Done, renamed);
    assert_eq!(BODY.replace("label: Period", "label: Renamed"), probed.text());
}

#[test]
fn a_modules_coordinates_say_where_a_node_is_and_what_a_move_writes() {
    let mut open = open(probe());

    let elements = open.session.render(open.workspace.documents()).unwrap();
    let moved = open
        .session
        .move_elements(&mut open.workspace, &[("aaa".into(), 300.0, 7.0)]);
    let refused = open
        .session
        .move_elements(&mut open.workspace, &[("bbb".into(), -1.0, 7.0)]);

    let bbb = elements.iter().find(|e| e.id == "bbb").unwrap();
    assert_eq!((200.0, 7.0), (bbb.x, bbb.y));
    assert_eq!(Ok(()), moved);
    assert_eq!(Err("bbb cannot go before the first row.".into()), refused);
    assert_eq!(BODY.replacen("    row: 0\r\n", "    row: 3\r\n", 1), open.text());
}

#[test]
fn an_operation_at_a_canvas_point_reads_the_position_the_module_gives() {
    let mut open = open(probe());

    let outcome = open.execute(
        &ActionTarget::Canvas { x: 0.0, y: 3.0 },
        "timeline.add-moment",
        &["mmm"],
    );

    assert_eq!(ActionOutcome::Done, outcome);
    assert!(
        open.text()
            .contains("  - id: mmm\r\n    label: New moment\r\n    begin: 2026-05-01\r\n    row: 3\r\n"),
        "{}",
        open.text()
    );
}

#[test]
fn a_plugin_layout_is_run_by_the_module_and_left_alone_without_one() {
    let mut generic = open(None);
    let mut probed = open(probe());
    let canvas = ActionTarget::Canvas { x: 0.0, y: 0.0 };

    assert_eq!(ActionOutcome::Done, generic.execute(&canvas, "timeline.arrange", &[]));
    assert_eq!(ActionOutcome::Done, probed.execute(&canvas, "timeline.arrange", &[]));

    assert_eq!(BODY, generic.text());
    assert_eq!(BODY.replacen("    row: 0\r\n", "    row: 7\r\n", 1), probed.text());
}

//! A breakpoint on a binding rule (etalii.adp spec 035, task T139, FR-070): it stops when the rule reads an entry of a
//! document the description's tool opens, once for each entry, with the entry's line and the attributes read so far
//! as its locals; stepping over goes through the attributes as they are read.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::{RuntimeHost, diagram_tool};
use adp_atd_script::debug::{
    Breakpoint, DebugControl, DebugEvent, PauseReason, ScopeKind, Session, statement_lines, verify,
};
use adp_diagram::DiagramDocuments;

fn diagrams() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams")
}

/// The next pause or end.
fn next(session: &Session) -> DebugEvent {
    loop {
        let event = session.next_event(Duration::from_secs(60)).expect("an event");
        if matches!(
            event,
            DebugEvent::Paused { .. } | DebugEvent::Finished { .. } | DebugEvent::Stopped { .. }
        ) {
            return event;
        }
    }
}

/// A local of the innermost frame, as shown.
fn local(event: &DebugEvent, name: &str) -> Option<String> {
    let DebugEvent::Paused { scopes, .. } = event else {
        panic!("not paused: {event:?}");
    };
    scopes
        .iter()
        .find(|s| s.frame == 0 && s.kind == ScopeKind::Locals)?
        .variables
        .iter()
        .find(|v| v.name == name)
        .map(|v| v.value.clone())
}

#[test]
fn a_breakpoint_on_a_binding_rule_stops_as_it_reads_each_entry() {
    let opened = Arc::new(Mutex::new(false));
    let done = Arc::clone(&opened);
    let session = Session::start(Vec::new(), false, move |debugger| {
        let root = diagrams().join("mindmap.atd");
        let checked = check(&FsFiles, &root);
        let symbols = checked.symbols.as_ref().expect("symbols");
        // The line of `element Node as node at "/map/**/node"` in the binding, the third entry only.
        let unit = checked
            .description
            .files
            .iter()
            .find(|f| f.name.ends_with("mindmap.freeplane.atd"))
            .expect("the binding's file");
        let line = unit
            .text
            .lines()
            .position(|l| l.contains("element Node as node"))
            .unwrap()
            + 1;
        let mut breakpoints = [Breakpoint::new(unit.name.clone(), line).when("line == 3")];
        let lines = checked.description.members().flat_map(statement_lines).collect();
        verify(&mut breakpoints, &lines);
        assert!(breakpoints[0].verified, "a binding rule's line takes a breakpoint");
        let host = RuntimeHost {
            debugger: Some(Rc::new(debugger.with_breakpoints(breakpoints.to_vec()))),
            ..RuntimeHost::default()
        };
        let runtime = diagram_tool(&checked.description, symbols, &host).unwrap();
        let case = diagrams().join("conformance/mindmap/example-1");
        let mut documents = DiagramDocuments::default();
        documents.open_bytes(
            runtime.tool.clone(),
            Path::new("/nowhere/mindmap.mm"),
            None,
            std::fs::read(case.join("mindmap.mm")).unwrap(),
            None,
        );
        *done.lock().unwrap() = true;
        Ok(())
    });

    let paused = next(&session);
    let DebugEvent::Paused { reason, frames, .. } = &paused else {
        panic!("not paused: {paused:?}");
    };
    assert_eq!(*reason, PauseReason::Breakpoint);
    assert_eq!(frames[0].name, "node (binding rule)");
    assert!(frames[0].file.ends_with("mindmap.freeplane.atd"));
    assert_eq!(local(&paused, "line").as_deref(), Some("3"));
    assert_eq!(local(&paused, "type").as_deref(), Some("Node"));
    // As the rule takes the entry, no attribute is read yet.
    assert_eq!(local(&paused, "text"), None);

    // Stepping over reads the attributes one by one: the text is read before the step after it.
    let mut text = None;
    for _ in 0..10 {
        session.control(DebugControl::StepOver);
        let event = next(&session);
        if !matches!(event, DebugEvent::Paused { .. }) {
            break;
        }
        if let Some(read) = local(&event, "text") {
            assert_eq!(local(&event, "line").as_deref(), Some("3"));
            text = Some(read);
            break;
        }
    }
    assert_eq!(text.as_deref(), Some("Scene graph"));

    session.control(DebugControl::Continue);
    // The condition holds for one entry only: the reading runs to its end.
    assert_eq!(next(&session), DebugEvent::Finished { committed: true });
    assert!(*opened.lock().unwrap());
}

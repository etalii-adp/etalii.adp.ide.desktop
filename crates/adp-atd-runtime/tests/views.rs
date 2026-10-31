//! `diagram.views` (DISL section 12.2; etalii.adp spec 067, FR-013), on a description of its own so that no tool type is
//! named: every view of the body with its members, whichever view the reading shows, each view's statements as data for
//! it. And what a rule reads through it, a layout's boundary and what a description states of a style, a shape and a
//! script function, which the C4 descriptions were the first to ask for.

use std::path::Path;

use adp_atd_check::{FsFiles, Severity, check};
use adp_atd_runtime::{RuntimeHost, RuntimeTool, diagram_tool};
use adp_diagram::DiagramDocuments;
use adp_diagram::workspace::DiagramWorkspace;
use adp_disl::DislElement;

const GRAMMAR: &str = r#"start: unit*
?unit: person
     | shelf
     | wall
     | _NL
person: "person" NAME _NL
shelf: "shelf" NAME [NAME] "{" _NL? (show_statement _NL)* "}"
wall: "wall" "{" _NL? (show_statement _NL)* "}"
show_statement: "show" NAME
NAME: /[a-z*]+/
_NL: /\n+/
WS: /[ \t]+/
%ignore WS
"#;

const DESCRIPTION: &str = r##"atd 0.1

diagram example/rooms version "0.1.0" {
  label "Rooms"
  id: "example.rooms"
  defaultLocale: "en"

  model {
    document {
      attribute shows: string many readonly default []
      attribute kindOfView: string readonly default ""
      attribute viewpoint: string readonly default ""
    }
    element Person {
      attribute name: string default "" { key: true }
      labelAttribute: "name"
    }
    element Show {
      attribute member: string readonly default ""
    }
  }

  fn onView(e: dyn) -> bool {
    // uses: ["diagram"]
    return cel.bind(here, diagram.shows, e.kind == 'node' && (e.name in here || '*' in here))
  }

  constraints {
    defaults: { severity: warning, timing: [live, save] }
    blockSaveOn: never
    constraint unseen for Person {
      label "Unseen"
      rule diagram.views.exists(v, v.members.exists(m, m == self))
      message ("'" + self.name + "' is on no view.")
      code: "example.unseen"
      over: model
    }
    constraint emptyView for diagram {
      label "Empty"
      message ("The '" + item.id + "' view of kind '" + item.viewpoint + "' is empty.")
      code: "example.empty"
      over: model
      forEach: (diagram.views.filter(v, !v.members.exists(m, m.kind == 'node')))
    }
  }

  notation {
    style base { font: { size: 10, lineHeight: 2 }, padding: [1, 2] }
    style card extends base { fill: "#000000" }
    node Person {
      shape person { radius: 4 }
      label (self.name) as name { wrap: none }
      style: "card"
      size: { min: [10, 5], max: [200, null], autoSize: fitContent, resizable: false }
    }
  }

  viewpoint shelfView "Shelf" {
    include: ["Person"]
    members: (onView(self))
    default: true
  }

  persistence {
    format: fbl
    ids: { strategy: natural, stable: true }
    typeMap: {
      Show: { as: diagram, attributes: { member: "shows" } }
    }
  }

  binding rooms family grammar {
    claims { extensions: [".rooms"], origins: ["example/rooms", "example/other"] }
    body { kind: file }
    grammar from "rooms.lark"
    block shelf {
      at: "/shelf"
      origins: ["example/rooms"]
      view: { node: "NAME" }
    }
    block wall {
      at: "/wall"
      origins: ["example/other"]
      view: { value: "'wall'" }
    }
    element Person as person at "/person" {
      id from node "NAME"
      attribute name from node "NAME" { readOnly: true }
    }
    element Show as show at "/*/show_statement" {
      selectedView: true
      attribute member from node "NAME" { readOnly: true }
    }
  }
}
"##;

fn rooms() -> RuntimeTool {
    let folder = tempfile::tempdir().unwrap().keep();
    let path = folder.join("rooms.atd");
    std::fs::write(&path, DESCRIPTION).unwrap();
    std::fs::write(folder.join("rooms.lark"), GRAMMAR).unwrap();
    let checked = check(&FsFiles, &path);
    let errors: Vec<String> = checked
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .map(adp_atd_check::Finding::to_line)
        .collect();
    assert!(errors.is_empty(), "{errors:#?}");
    diagram_tool(
        &checked.description,
        checked.symbols.as_ref().unwrap(),
        &RuntimeHost::default(),
    )
    .unwrap()
}

fn views_of(body: &str) -> Vec<(String, Vec<String>)> {
    let tool = rooms();
    let mut documents = DiagramDocuments::default();
    let document = documents.open_bytes(
        tool.tool.clone(),
        Path::new("/nowhere/x.rooms"),
        None,
        body.as_bytes().to_vec(),
        None,
    );
    let program = tool
        .tool
        .definition
        .specification()
        .environment("element", &[])
        .compile("diagram.views.map(v, [v.id, v.members.map(m, m.id)])")
        .unwrap();
    let variables = std::collections::HashMap::from([("diagram".to_owned(), document.model().diagram.to_value())]);
    let value = program.evaluate(&variables).unwrap();
    value
        .as_list()
        .unwrap()
        .iter()
        .map(|view| {
            let view = view.as_list().unwrap();
            let members = view[1].as_list().unwrap().iter().map(|m| m.to_string()).collect();
            (view[0].to_string(), members)
        })
        .collect()
}

const BODY: &str = "person ann\nperson bob\nperson cid\nshelf one {\nshow ann\n}\nshelf two {\nshow bob\nshow cid\n}\nwall {\nshow cid\n}\n";

#[test]
fn every_view_of_the_body_is_listed_with_its_members_whichever_the_reading_shows() {
    assert_eq!(
        views_of(BODY),
        vec![
            ("one".to_owned(), vec!["ann".to_owned()]),
            ("two".to_owned(), vec!["bob".to_owned(), "cid".to_owned()]),
            // A view of another reading's block is listed too, its own statements selecting its members.
            ("wall".to_owned(), vec!["cid".to_owned()]),
        ]
    );
}

#[test]
fn a_body_without_views_has_none() {
    assert_eq!(views_of("person ann\n"), vec![]);
}

/// Rules about the model read every view: nothing is said of `ann` or `bob`, whichever view is shown.
#[test]
fn a_rule_about_the_model_sees_every_view() {
    let tool = rooms();
    let body =
        "person ann\nperson bob\nperson zed\nshelf one {\nshow ann\n}\nshelf two {\nshow bob\n}\nshelf empty {\n}\n";
    let mut workspace = DiagramWorkspace::in_memory("/nowhere");
    let path = Path::new("/nowhere/x.rooms");
    let session = workspace.open_bytes_beside(tool.tool.clone(), path, None, body.as_bytes().to_vec(), None, &|_| None);
    let findings = session.problems(workspace.documents());
    let messages: Vec<&str> = findings.iter().map(|f| f.message.as_str()).collect();
    assert!(messages.contains(&"'zed' is on no view."), "{messages:?}");
    assert!(
        !messages.iter().any(|m| m.contains("'ann'") || m.contains("'bob'")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|m| m.starts_with("The 'empty' view of kind 'shelfView'")),
        "{messages:?}"
    );
}

/// A node that fits its content is measured in its style's font and padding, through what the style extends.
#[test]
fn a_node_that_fits_its_content_is_measured_in_its_style() {
    let tool = rooms();
    let mut documents = DiagramDocuments::default();
    let document = documents.open_bytes(
        tool.tool.clone(),
        Path::new("/nowhere/x.rooms"),
        None,
        b"person ann\nshelf one {\nshow ann\n}\n".to_vec(),
        None,
    );
    let element: DislElement = document.element("ann").expect("the person");
    let module = tool.tool.module.as_ref().expect("the description's module");
    // Three characters at 0.55 of 10, and 2 either side; one line of 10 by 2, and 1 above and below.
    assert_eq!(module.size_of(document, &element), Some((20.5, 22.0)));
    let shape = &tool.tool.definition.specification().root()["notation"]["nodes"]["Person"]["shape"];
    assert_eq!(shape["type"], "person", "{shape}");
    assert_eq!(shape["params"]["radius"], 4, "{shape}");
    assert_eq!(
        tool.tool.definition.specification().root()["notation"]["styles"]["card"]["extends"],
        serde_json::json!(["base"])
    );
}

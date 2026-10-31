//! The flow page (etalii.adp spec 074, FR-007 to FR-010, FR-017, FR-027, FR-035): the exact prototype at FR-035's places
//! apart, holding the diagram as the canvas does, with its text safe and the same bytes every time.

mod common;

use std::fs;

use adp_diagram_causal_loop::flow_page::{TEMPLATE, generate};
use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::positions;
use common::{Diagram, fixture, open, open_with};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn page(diagram: &Diagram) -> String {
    let document = diagram.workspace.documents().get(&diagram.body).expect("open");
    generate(document, &Metrics::default(), "feedback")
}

fn data(page: &str) -> Value {
    let line = page
        .lines()
        .find(|l| l.starts_with("const DATA = "))
        .expect("the data line");
    let json = line.strip_prefix("const DATA = ").unwrap().strip_suffix(';').unwrap();
    serde_json::from_str(json).expect("the data is JSON")
}

fn data_of(text: &str) -> Value {
    data(&page(&open(text)))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn the_template_is_the_reference_page_apart_from_fr_035s_places() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let patch: Value =
        serde_json::from_str(&fs::read_to_string(folder.join("flow-page-reference.json")).unwrap()).unwrap();
    let lines: Vec<&str> = TEMPLATE.split('\n').collect();
    let mut restored: Vec<&str> = Vec::new();
    let mut at = 0;
    for hunk in patch["hunks"].as_array().unwrap() {
        let place = hunk["place"].as_str().unwrap();
        assert!(
            ["a", "b", "c", "d", "e", "f"].contains(&place),
            "{place} is no place of FR-035"
        );
        let line = hunk["line"].as_u64().unwrap() as usize - 1;
        restored.extend(&lines[at..line]);
        restored.extend(hunk["original"].as_array().unwrap().iter().map(|l| l.as_str().unwrap()));
        at = line + hunk["count"].as_u64().unwrap() as usize;
    }
    restored.extend(&lines[at..]);
    assert_eq!(
        hex(&Sha256::digest(restored.join("\n").as_bytes())),
        patch["sha256"].as_str().unwrap(),
        "the template differs from the reference page at a place FR-035 does not name"
    );
}

#[test]
fn every_example_is_carried_whole_where_the_canvas_places_it() {
    for name in ["on-call", "reference", "business-philosophy"] {
        let path = format!("examples/{name}/{name}.cld");
        let registration = fs::read_to_string(common::fixtures().join(format!("examples/{name}/{name}.adp")))
            .unwrap_or_else(|_| "systems/causal-loop-diagram\r\n".to_owned());
        let diagram = open_with(&fixture(&path), &registration);
        let document = diagram.workspace.documents().get(&diagram.body).unwrap();
        let model = adp_diagram_causal_loop::reading::Model::of(&document.model().diagram);
        let page = page(&diagram);
        let data = data(&page);
        let variables = data["v"].as_array().unwrap();
        assert_eq!(variables.len(), model.distinct_variables().len(), "{name}");
        assert_eq!(data["l"].as_array().unwrap().len(), model.links.len(), "{name}");
        assert_eq!(
            data["o"].as_array().unwrap().len(),
            document.model().diagram.nodes_of_type("Loop").len(),
            "{name}"
        );
        let places = positions(document, &Metrics::default());
        for variable in variables {
            let place = places[&format!("variable:{}", variable[0].as_str().unwrap())];
            assert_eq!(
                (variable[2].as_f64().unwrap(), variable[3].as_f64().unwrap()),
                place,
                "{name}"
            );
        }
    }
}

#[test]
fn the_page_holds_what_the_canvas_holds_edits_included_and_touches_no_file() {
    let mut diagram = open("causal-loop 1\r\nvariable a \"A\"\r\nvariable b \"B\"\r\nlink a -> b +\r\n");
    diagram
        .session
        .set_property(
            &mut diagram.workspace,
            &["variable:a"],
            "causal-loop.variable-label",
            "Edited",
        )
        .unwrap();
    let before = (diagram.text(), diagram.registration_text());
    let data = data(&page(&diagram));
    assert_eq!(data["v"][0][1], "Edited");
    assert_eq!((diagram.text(), diagram.registration_text()), before);
}

#[test]
fn the_same_diagram_gives_the_same_bytes_and_nothing_leaves_the_file() {
    let diagram = open(&fixture("examples/on-call/on-call.cld"));
    let page = page(&diagram);
    assert_eq!(page, self::page(&diagram));
    for reaching_out in ["src=", "href=", "url(", "http://", "https://", "@import"] {
        assert!(!page.contains(reaching_out), "{reaching_out}");
    }
    assert!(!page.contains("/*FLOW_PAGE_"));
}

#[test]
fn markup_in_a_label_a_note_or_a_title_arrives_as_text() {
    let script = "</script><script>alert(1)</script>";
    let text = format!(
        "causal-loop 1\r\ntitle \"{script} &amp;\"\r\nvariable a \"{script}\"\r\nvariable b \"&amp;<b>\"\r\nlink a -> b + \"{script}\"\r\n"
    );
    let page = page(&open(&text));
    assert_eq!(page.matches("</script>").count(), 1, "the page's own closing tag alone");
    assert!(page.contains("<title>&lt;/script&gt;&lt;script&gt;alert(1)&lt;/script&gt; &amp;amp;</title>"));
    let data = data(&page);
    assert_eq!(data["v"][0][1], script);
    assert_eq!(data["v"][1][1], "&amp;<b>");
    assert_eq!(data["l"][0][4], script);
    // Everywhere the page writes a label or a note as HTML, it escapes it.
    for raw in [
        "${n.label}",
        "${A.label}",
        "${B.label}",
        "${l.note}",
        "${hover.label}",
        "${hover.note}",
        "${lp.name}",
        "${lp.id}",
        "${nodes[s.i].label}",
    ] {
        assert!(!TEMPLATE.contains(raw), "{raw}");
    }
}

#[test]
fn declared_loops_are_carried_and_one_the_links_do_not_close_is_left_out() {
    let data = data_of(
        "causal-loop 1\r\nvariable a\r\nvariable b\r\nlink a -> b +\r\nlink b -> a -\r\nloop B1 \"Check\" a b\r\nloop R9 \"Open\" a b a\r\n",
    );
    let loops = data["o"].as_array().unwrap();
    assert_eq!(loops.len(), 1, "{loops:?}");
    assert_eq!(loops[0], serde_json::json!(["B1", "Check", ["a", "b"]]));
}

#[test]
fn a_diagram_without_loops_carries_the_cycles_the_host_finds() {
    let data = data_of("causal-loop 1\r\nvariable a \"A\"\r\nvariable b \"B\"\r\nlink a -> b +\r\nlink b -> a -\r\n");
    assert_eq!(data["o"], serde_json::json!([["B1", "A → B", ["a", "b"]]]));
}

#[test]
fn the_flow_properties_are_the_datas_limited_to_their_ranges() {
    let data = data_of(
        "causal-loop 1\r\ntitle \"T\"\r\nflow strength=99 delay=6.5 inertia=0.01 speed=2 density=1.5 heatmap=level\r\nvariable a \"A\" start=0.8 push=-9\r\nvariable b start=7\r\nlink a -> b + weight=1.5\r\n",
    );
    assert_eq!(
        data["p"],
        serde_json::json!({"strength": 8, "delayLength": 6.5, "inertia": 0.2, "speed": 2, "density": 1.5, "heatmap": "level"})
    );
    assert_eq!(data["v"][0][4], 0.8);
    assert_eq!(data["v"][0][5], -4);
    assert_eq!(data["v"][1][4], 1);
    assert_eq!(data["l"][0][5], 1.5);
}

#[test]
fn the_defaults_stand_where_the_file_states_none() {
    let data = data_of("causal-loop 1\r\nvariable a\r\nvariable b\r\nlink a -> b\r\n");
    assert_eq!(
        data["p"],
        serde_json::json!({"strength": 3, "delayLength": 5, "inertia": 1.2, "speed": 1, "density": 3, "heatmap": "backlog"})
    );
    assert_eq!(data["v"][0][4], 0.5);
    assert_eq!(data["v"][0][5], 0);
    assert_eq!(data["l"][0][2], 1, "a link of unstated polarity counts as positive");
    assert_eq!(data["l"][0][5], 1, "a link without a weight starts at 1");
}

#[test]
fn the_page_takes_the_file_name_when_the_diagram_has_no_title() {
    assert!(page(&open("causal-loop 1\r\nvariable a\r\n")).contains("<h1>feedback</h1>"));
    assert!(page(&open("causal-loop 1\r\ntitle \"Mine\"\r\nvariable a\r\n")).contains("<title>Mine</title>"));
}

#[test]
fn the_edge_cases_give_a_page() {
    // No links; a self-link; two opposite links; a link naming no variable; weights of 0, -3 and 1e21.
    assert_eq!(data_of("causal-loop 1\r\nvariable a\r\n")["l"], serde_json::json!([]));
    assert_eq!(
        data_of("causal-loop 1\r\nvariable a\r\nlink a -> a +\r\n")["l"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        data_of("causal-loop 1\r\nvariable a\r\nvariable b\r\nlink a -> b +\r\nlink b -> a +\r\n")["l"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        data_of("causal-loop 1\r\nvariable a\r\nlink a -> nowhere +\r\n")["l"],
        serde_json::json!([])
    );
    let weights = data_of(
        "causal-loop 1\r\nvariable a\r\nvariable b\r\nvariable c\r\nlink a -> b + weight=0\r\nlink b -> c + weight=-3\r\nlink c -> a + weight=1e21\r\n",
    );
    assert_eq!(weights["l"][0][5], 0);
    assert_eq!(weights["l"][1][5], 3);
    assert_eq!(weights["l"][2][5], 3);
}

#[test]
fn the_chart_follows_the_seven_variables_in_the_most_loops() {
    let mut text = String::from("causal-loop 1\r\n");
    for n in 0..9 {
        text.push_str(&format!("variable v{n}\r\n"));
    }
    text.push_str("link v8 -> v7 +\r\nlink v7 -> v8 +\r\nloop R1 \"One\" v8 v7\r\n");
    let data = data_of(&text);
    assert_eq!(data["c"], serde_json::json!(["v7", "v8", "v0", "v1", "v2", "v3", "v4"]));
}

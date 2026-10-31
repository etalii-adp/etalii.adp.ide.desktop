//! The flow page of a causal loop diagram (etalii.adp spec 074): one self-contained web page that animates the diagram as
//! flows of influence. The page is `flow_page.html`, which is the prototype the maintainer accepted changed only at
//! FR-035's places, with the diagram's data written at its markers. The generator reads the diagram as the host holds it,
//! unsaved edits and current positions included, and writes nothing; the same diagram gives the same bytes.
//!
//! `DATA` holds what the page reads: `v`, each variable as `[id, label, x, y, start, push]`; `l`, each link as
//! `[from, to, sign, delayed, note, weight]`; `o`, each loop as `[id, name, [member ids]]`; `c`, the ids of the chart's
//! series; and `p`, the dynamics and the heatmap reading. Every number is limited to the range of its control. A link
//! that names no variable and a loop whose links do not close it are left out, since the page has no way to draw either.

use std::collections::{HashMap, HashSet};

use adp_cel::Value;
use adp_diagram::DiagramDocument;
use adp_disl::DislElement;

use crate::metrics::Metrics;
use crate::reading::{Model, text};
use crate::{cycles, positions};

/// The page, with its markers.
pub const TEMPLATE: &str = include_str!("flow_page.html");

/// Where the page's title and heading are written, HTML-escaped.
const TITLE_MARKER: &str = "/*FLOW_PAGE_TITLE*/";

/// Where the page's data is written.
const DATA_MARKER: &str = "/*FLOW_PAGE_DATA*/";

/// How many series the chart follows.
const SERIES: usize = 7;

/// How many cycles a diagram without declared loops lists at most, as the definition's cycle search is bounded.
const CYCLE_BOUND: usize = 1000;

/// The flow page of `document`, whose file name without its extension is `file_stem`.
pub fn generate(document: &DiagramDocument, metrics: &Metrics, file_stem: &str) -> String {
    let diagram = &document.model().diagram;
    let title = {
        let stated = text_value(&diagram.value_of("title"));
        if stated.is_empty() {
            file_stem.to_owned()
        } else {
            stated
        }
    };
    let data = data(document, metrics);
    let mut page = String::with_capacity(TEMPLATE.len() + data.len());
    let mut rest = TEMPLATE;
    while let Some(at) = next_marker(rest) {
        page.push_str(&rest[..at]);
        rest = &rest[at..];
        if let Some(after) = rest.strip_prefix(TITLE_MARKER) {
            page.push_str(&html_escaped(&title));
            rest = after;
        } else if let Some(after) = rest.strip_prefix(DATA_MARKER) {
            page.push_str(&data);
            rest = after;
        }
    }
    page.push_str(rest);
    page
}

fn next_marker(text: &str) -> Option<usize> {
    [TITLE_MARKER, DATA_MARKER].iter().filter_map(|m| text.find(m)).min()
}

/// The page's `DATA`, as one line of JSON whose keys are in one order.
fn data(document: &DiagramDocument, metrics: &Metrics) -> String {
    let diagram = &document.model().diagram;
    let model = Model::of(diagram);
    let declared: HashSet<&str> = model.variables.iter().map(|v| v.name.as_str()).collect();
    let places = positions(document, metrics);

    let mut seen = HashSet::new();
    let variables: Vec<String> = diagram
        .nodes_of_type("Variable")
        .iter()
        .filter(|v| seen.insert(text(v, "name")))
        .map(|v| {
            let name = text(v, "name");
            let label = text(v, "label");
            let (x, y) = places.get(&format!("variable:{name}")).copied().unwrap_or((0.0, 0.0));
            format!(
                "[{},{},{},{},{},{}]",
                json_text(&name),
                json_text(if label.is_empty() { &name } else { &label }),
                x,
                y,
                limited(number(&v.value_of("start")).unwrap_or(0.5), 0.0, 1.0),
                limited(number(&v.value_of("push")).unwrap_or(0.0), -4.0, 4.0),
            )
        })
        .collect();

    let drawn: Vec<DislElement> = diagram
        .relations_of_type("CausalLink")
        .into_iter()
        .filter(|l| {
            declared.contains(text(l, "causeName").as_str()) && declared.contains(text(l, "effectName").as_str())
        })
        .collect();
    let links: Vec<String> = drawn
        .iter()
        .map(|l| {
            format!(
                "[{},{},{},{},{},{}]",
                json_text(&text(l, "causeName")),
                json_text(&text(l, "effectName")),
                sign(l),
                u8::from(matches!(l.value_of("delayed"), Value::Bool(true))),
                json_text(&text(l, "note")),
                starting_strength(l),
            )
        })
        .collect();

    let signs: HashMap<(String, String), i32> = drawn
        .iter()
        .rev()
        .map(|l| ((text(l, "causeName"), text(l, "effectName")), sign(l)))
        .collect();
    let closes = |members: &[String]| {
        !members.is_empty()
            && (0..members.len())
                .all(|i| signs.contains_key(&(members[i].clone(), members[(i + 1) % members.len()].clone())))
    };

    let stated = diagram.nodes_of_type("Loop");
    let mut loops: Vec<(String, String, Vec<String>)> = stated
        .iter()
        .filter_map(|l| {
            let members = member_names(l)?;
            closes(&members).then(|| (text(l, "identifier"), text(l, "title"), members))
        })
        .collect();
    if stated.is_empty() {
        loops = found_cycles(&model, &signs);
    }

    let mut in_loops: HashMap<&str, usize> = HashMap::new();
    for (_, _, members) in &loops {
        for member in members {
            *in_loops.entry(member.as_str()).or_default() += 1;
        }
    }
    let mut order: Vec<String> = model.distinct_variables().into_iter().map(|v| v.name).collect();
    order.sort_by_key(|name| std::cmp::Reverse(in_loops.get(name.as_str()).copied().unwrap_or(0)));
    let chart: Vec<String> = order.iter().take(SERIES).map(|name| json_text(name)).collect();

    let loop_texts: Vec<String> = loops
        .iter()
        .map(|(id, name, members)| {
            let members: Vec<String> = members.iter().map(|m| json_text(m)).collect();
            format!("[{},{},[{}]]", json_text(id), json_text(name), members.join(","))
        })
        .collect();

    let setting = |name: &str, default: f64, low: f64, high: f64| {
        limited(number(&diagram.value_of(name)).unwrap_or(default), low, high)
    };
    let heatmap = match text_value(&diagram.value_of("heatmap")).as_str() {
        reading @ ("pressure" | "throughput" | "level") => reading.to_owned(),
        _ => "backlog".to_owned(),
    };
    format!(
        "{{\"c\":[{}],\"l\":[{}],\"o\":[{}],\"p\":{{\"delayLength\":{},\"density\":{},\"heatmap\":{},\"inertia\":{},\"speed\":{},\"strength\":{}}},\"v\":[{}]}}",
        chart.join(","),
        links.join(","),
        loop_texts.join(","),
        setting("delayLength", 5.0, 1.0, 12.0),
        setting("density", 3.0, 0.5, 8.0),
        json_text(&heatmap),
        setting("inertia", 1.2, 0.2, 5.0),
        setting("speed", 1.0, 0.1, 4.0),
        setting("strength", 3.0, 0.5, 8.0),
        variables.join(","),
    )
}

/// The cycles the host finds in a diagram that declares no loops, named by their kind's letter and counted per kind, as
/// the host claims a loop, with the path of their variables as their name.
fn found_cycles(model: &Model, signs: &HashMap<(String, String), i32>) -> Vec<(String, String, Vec<String>)> {
    let labels: HashMap<&str, &str> = model
        .variables
        .iter()
        .rev()
        .map(|v| (v.name.as_str(), v.display()))
        .collect();
    let (mut reinforcing, mut balancing) = (0, 0);
    cycles::find(model, CYCLE_BOUND)
        .cycles
        .into_iter()
        .map(|cycle| {
            let negative = (0..cycle.len())
                .filter(|i| {
                    signs
                        .get(&(cycle[*i].clone(), cycle[(*i + 1) % cycle.len()].clone()))
                        .is_some_and(|s| *s < 0)
                })
                .count();
            let id = if negative % 2 == 0 {
                reinforcing += 1;
                format!("R{reinforcing}")
            } else {
                balancing += 1;
                format!("B{balancing}")
            };
            let path: Vec<&str> = cycle
                .iter()
                .map(|m| labels.get(m.as_str()).copied().unwrap_or(m))
                .collect();
            (id, path.join(" → "), cycle)
        })
        .collect()
}

/// The names of the variables loop `element` runs through; none when one of them is no variable.
fn member_names(element: &DislElement) -> Option<Vec<String>> {
    let Value::List(members) = element.value_of("members") else {
        return None;
    };
    members
        .iter()
        .map(|m| DislElement::from_value(m).map(|v| text(&v, "name")))
        .collect()
}

/// A link's sign: -1 for a negative link, else 1, a link of unstated polarity included.
fn sign(link: &DislElement) -> i32 {
    if text_value(&link.value_of("polarity")) == "negative" {
        -1
    } else {
        1
    }
}

/// A link's starting strength (FR-027): its weight's size limited to 0 to 3; 1 for a link without one.
fn starting_strength(link: &DislElement) -> f64 {
    match link.attributes().get("weight").and_then(number) {
        Some(weight) => limited(weight.abs(), 0.0, 3.0),
        None => 1.0,
    }
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Double(number) if number.is_finite() => Some(*number),
        Value::Int(number) => Some(*number as f64),
        _ => None,
    }
}

fn text_value(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn limited(value: f64, low: f64, high: f64) -> f64 {
    value.clamp(low, high)
}

/// `text` as a JSON string that is safe inside a script element: `<`, `>`, `&` and the two line separators JavaScript
/// reads as line endings are written as `\uXXXX`.
fn json_text(text: &str) -> String {
    let mut written = String::with_capacity(text.len() + 2);
    written.push('"');
    for c in text.chars() {
        match c {
            '"' => written.push_str("\\\""),
            '\\' => written.push_str("\\\\"),
            '<' | '>' | '&' | '\u{2028}' | '\u{2029}' => written.push_str(&format!("\\u{:04x}", c as u32)),
            c if (c as u32) < 0x20 => written.push_str(&format!("\\u{:04x}", c as u32)),
            c => written.push(c),
        }
    }
    written.push('"');
    written
}

/// `text` as HTML text, for the page's title and heading.
fn html_escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

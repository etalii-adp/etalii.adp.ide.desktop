//! What is wrong with a map, as the problems panel shows it (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyRuleSet.cs` and `WardleyValidator.cs` at `8b6ccfc`. [`validate`] is a pure
//! function from a parsed map to problems, in document order where they have one; a map with a problem still opens
//! and draws, and a dangling link is reported and kept. [`validate_document`] adds what a model cannot answer: a
//! `url` whose address looks like a file of the project and is not there, and every line the parser passed over.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::identities::keys;
use crate::lines::{LineDocument, format_number};
use crate::model::{WardleyCoordinate, WardleyElementKind, WardleyMap};
use crate::parser;

pub const LINK_TARGET_MISSING: &str = "wardley.link-target-missing";
pub const EVOLVE_TARGET_MISSING: &str = "wardley.evolve-target-missing";
pub const PIPELINE_PARENT_MISSING: &str = "wardley.pipeline-parent-missing";
pub const COORDINATE_OUT_OF_RANGE: &str = "wardley.coordinate-out-of-range";
pub const DUPLICATE_NAME: &str = "wardley.duplicate-name";
pub const ANCHOR_MISSING: &str = "wardley.anchor-missing";
pub const URL_UNDEFINED: &str = "wardley.url-undefined";
pub const SUBMAP_MISSING: &str = "wardley.submap-missing";
/// A line the parser passed over, at that line.
pub const LINE_PASSED_OVER: &str = "wardley.line-passed-over";

fn problem(
    severity: DiagramProblemSeverity,
    message: String,
    rule_id: &str,
    location: Option<DiagramProblemLocation>,
) -> DiagramProblem {
    DiagramProblem {
        severity,
        message,
        rule_id: rule_id.to_owned(),
        location,
    }
}

/// Every problem in `map`.
pub fn validate(map: &WardleyMap) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    let names: HashSet<&str> = map.components.iter().map(|c| c.name.as_str()).collect();
    duplicate_names(map, &mut problems);
    dangling_references(map, &names, &mut problems);
    coordinates(map, &mut problems);
    urls(map, &mut problems);
    anchor(map, &mut problems);
    problems
}

/// Two components sharing a name: reported at the element, since identity is keyed by name and the two reconcile to
/// one element.
fn duplicate_names(map: &WardleyMap, problems: &mut Vec<DiagramProblem>) {
    let mut seen: Vec<&str> = Vec::new();
    for component in &map.components {
        if seen.contains(&component.name.as_str()) {
            continue;
        }
        seen.push(&component.name);
        let group: Vec<_> = map.components.iter().filter(|c| c.name == component.name).collect();
        if group.len() < 2 {
            continue;
        }
        let lines = group.iter().map(|c| c.line.to_string()).collect::<Vec<_>>().join(", ");
        problems.push(problem(
            DiagramProblemSeverity::Error,
            format!(
                "'{}' is declared more than once (lines {lines}), so every link, evolve and pipeline that names it is ambiguous.",
                component.name
            ),
            DUPLICATE_NAME,
            Some(DiagramProblemLocation::Element(keys::component(group[0]))),
        ));
    }
}

/// A link, an `evolve` or a pipeline naming a component that does not exist, at the line it was written on.
fn dangling_references(map: &WardleyMap, names: &HashSet<&str>, problems: &mut Vec<DiagramProblem>) {
    for link in &map.links {
        for end in [&link.source, &link.target] {
            if !names.contains(end.as_str()) {
                problems.push(problem(
                    DiagramProblemSeverity::Error,
                    format!(
                        "'{end}' is not on this map, but '{}{}{}' links to it.",
                        link.source,
                        link.kind.arrow(),
                        link.target
                    ),
                    LINK_TARGET_MISSING,
                    Some(DiagramProblemLocation::Line(link.line)),
                ));
            }
        }
    }
    for evolve in map.evolves.iter().filter(|e| !names.contains(e.name.as_str())) {
        problems.push(problem(
            DiagramProblemSeverity::Error,
            format!(
                "'{}' is not on this map, but 'evolve {}' says where it is heading.",
                evolve.name, evolve.name
            ),
            EVOLVE_TARGET_MISSING,
            Some(DiagramProblemLocation::Line(evolve.line)),
        ));
    }
    for pipeline in map.pipelines.iter().filter(|p| !names.contains(p.parent.as_str())) {
        problems.push(problem(
            DiagramProblemSeverity::Error,
            format!(
                "'{}' is not on this map, but a pipeline is declared for it.",
                pipeline.parent
            ),
            PIPELINE_PARENT_MISSING,
            Some(DiagramProblemLocation::Line(pipeline.line)),
        ));
    }
}

fn outside(value: f64) -> bool {
    !(0.0..=1.0).contains(&value)
}

fn report(problems: &mut Vec<DiagramProblem>, line: u32, what: &str, value: f64, axis: &str) {
    problems.push(problem(
        DiagramProblemSeverity::Error,
        format!(
            "The {axis} of {what} is {}, and a Wardley map's axes run from 0 to 1.",
            format_number(value)
        ),
        COORDINATE_OUT_OF_RANGE,
        Some(DiagramProblemLocation::Line(line)),
    ));
}

fn check(problems: &mut Vec<DiagramProblem>, position: WardleyCoordinate, line: u32, what: &str) {
    if outside(position.visibility) {
        report(problems, line, what, position.visibility, "visibility");
    }
    if outside(position.maturity) {
        report(problems, line, what, position.maturity, "maturity");
    }
}

/// Any coordinate outside 0..1, on anything the format positions: the canvas would clamp it, and the element would sit
/// somewhere the document does not claim.
fn coordinates(map: &WardleyMap, problems: &mut Vec<DiagramProblem>) {
    for component in &map.components {
        check(
            problems,
            component.position,
            component.line,
            &format!("'{}'", component.name),
        );
    }
    for pipeline in &map.pipelines {
        if let Some(extent) = pipeline.legacy_extent {
            check(
                problems,
                extent,
                pipeline.line,
                &format!("the pipeline on '{}'", pipeline.parent),
            );
        }
        for child in pipeline.children.iter().filter(|c| outside(c.maturity)) {
            report(
                problems,
                child.line,
                &format!("'{}'", child.name),
                child.maturity,
                "maturity",
            );
        }
    }
    for evolve in map.evolves.iter().filter(|e| outside(e.maturity)) {
        report(
            problems,
            evolve.line,
            &format!("the evolution target for '{}'", evolve.name),
            evolve.maturity,
            "maturity",
        );
    }
    for note in &map.notes {
        check(problems, note.position, note.line, "a note");
    }
    for accelerator in &map.accelerators {
        check(
            problems,
            accelerator.position,
            accelerator.line,
            &format!("'{}'", accelerator.name),
        );
    }
    for attitude in &map.attitudes {
        let what = format!("a {} region", attitude.kind.name());
        check(problems, attitude.from, attitude.line, &what);
        check(problems, attitude.to, attitude.line, &what);
    }
    for annotation in &map.annotations {
        for occurrence in &annotation.occurrences {
            check(
                problems,
                *occurrence,
                annotation.line,
                &format!("annotation {}", annotation.number),
            );
        }
    }
}

/// A `url(...)` naming no `url` definition.
fn urls(map: &WardleyMap, problems: &mut Vec<DiagramProblem>) {
    let defined: HashSet<&str> = map.urls.iter().map(|u| u.name.as_str()).collect();
    for component in map
        .components
        .iter()
        .filter(|c| !c.url.is_empty() && !defined.contains(c.url.as_str()))
    {
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            format!(
                "'{}' points at a url called '{}', which this map does not define.",
                component.name, component.url
            ),
            URL_UNDEFINED,
            Some(DiagramProblemLocation::Line(component.line)),
        ));
    }
}

/// A map with components and no `anchor`: a warning against the file.
fn anchor(map: &WardleyMap, problems: &mut Vec<DiagramProblem>) {
    if !map.components.is_empty() && map.components.iter().all(|c| c.kind != WardleyElementKind::Anchor) {
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            "This map has no anchor, so nothing says whose need the value chain hangs from.".to_owned(),
            ANCHOR_MISSING,
            None,
        ));
    }
}

/// Every problem of the document `text` in the project at `root`: the rules, a `url` whose address looks like a file
/// of the project and is not there, and each line the parser passed over, as a warning at that line.
pub fn validate_document(text: &str, root: &Path) -> Vec<DiagramProblem> {
    let (map, passed_over) = parser::outcome(&LineDocument::parse(text));
    let mut problems = validate(&map);
    problems.extend(unresolved_references(&map, root));
    problems.extend(passed_over.into_iter().map(|p| {
        problem(
            DiagramProblemSeverity::Warning,
            p.message,
            LINE_PASSED_OVER,
            Some(DiagramProblemLocation::Line(p.line as u32 + 1)),
        )
    }));
    problems
}

/// Whether an address is a path in the project rather than somewhere on the internet: anything with a scheme is left
/// alone.
fn looks_like_a_file(address: &str) -> bool {
    !address.is_empty() && !address.contains("://") && !address.contains(':')
}

/// The path with `.` and `..` resolved without touching the file system.
fn normalise(path: &Path) -> PathBuf {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let mut out = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// A `url` whose address looks like a file in the project, and is not; a `..` walking out of the project is not
/// looked for.
fn unresolved_references(map: &WardleyMap, root: &Path) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    let root = normalise(root);
    for url in map.urls.iter().filter(|u| looks_like_a_file(&u.address)) {
        let full = normalise(&root.join(url.address.replace('/', std::path::MAIN_SEPARATOR_STR)));
        if !full.starts_with(&root) || full.exists() {
            continue;
        }
        let named = match map.components.iter().find(|c| c.url == url.name) {
            Some(referrer) => format!("'{}' points at", referrer.name),
            None => "This map defines".to_owned(),
        };
        problems.push(problem(
            DiagramProblemSeverity::Warning,
            format!("{named} '{}', which is not in this project.", url.address),
            SUBMAP_MISSING,
            Some(DiagramProblemLocation::Line(url.line)),
        ));
    }
    problems
}

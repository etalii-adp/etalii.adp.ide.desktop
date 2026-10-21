//! Reads an `.owm` document into a [`WardleyMap`] (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `src/diagrams/wardley-map/backend/EtAlii.Adp.Diagram.WardleyMap/WardleyParser.cs`
//! at `8b6ccfc`. Line-oriented, because the DSL is: every statement is one line, with one nested construct
//! (`pipeline Parent { ... }`) one level deep. It never fails: a line it does not recognise is passed over, the
//! document keeps it, and [`outcome`] reports it; a statement the DSL knows and this module does not model (`y_axis`,
//! `x_axis`, `evolution`, `presentation`) is skipped by design and is no problem. Coordinates stay in the document's
//! `[visibility, maturity]` order.

use std::sync::LazyLock;

use regex::Regex;

use crate::lines::{LineDocument, parse_number, strip_comment};
use crate::model::{
    ParseProblem, WardleyAccelerator, WardleyAnnotation, WardleyAttitude, WardleyAttitudeKind, WardleyComponent,
    WardleyCoordinate, WardleyDecorator, WardleyElementKind, WardleyEvolve, WardleyLabelOffset, WardleyLink,
    WardleyLinkKind, WardleyMap, WardleyMapSize, WardleyNote, WardleyPipeline, WardleyPipelineChild,
    WardleyPipelineForm, WardleyUrlDefinition,
};

fn compile(pattern: &str) -> Regex {
    Regex::new(pattern).expect("a valid expression")
}

static COORDINATE: LazyLock<Regex> = LazyLock::new(|| compile(r"\[\s*(?<values>-?[\d.]+\s*,\s*-?[\d.]+)\s*\]"));
static SINGLE_COORDINATE: LazyLock<Regex> = LazyLock::new(|| compile(r"\[\s*(?<value>-?[\d.]+)\s*\]"));
static LINK: LazyLock<Regex> =
    LazyLock::new(|| compile(r"^(?<source>[^-+]+?)\s*(?<arrow>->|\+>)\s*(?<target>[^;]+?)\s*(?:;\s*(?<context>.*))?$"));
static DECORATOR: LazyLock<Regex> = LazyLock::new(|| compile(r"\(\s*(?<name>[A-Za-z]+)\s*\)"));
static LABEL: LazyLock<Regex> = LazyLock::new(|| compile(r"(?i)label\s*\[\s*(?<values>-?[\d.]+\s*,\s*-?[\d.]+)\s*\]"));
static URL: LazyLock<Regex> = LazyLock::new(|| compile(r"(?i)url\s*\(\s*(?<name>[^)]+)\s*\)"));
static EVOLVE: LazyLock<Regex> =
    LazyLock::new(|| compile(r"^(?<name>.+?)\s*(?:->\s*(?<override>[^\d]+?)\s*)?(?<maturity>-?[\d.]+)\s*$"));
static ATTITUDE: LazyLock<Regex> =
    LazyLock::new(|| compile(r"\[\s*(?<values>-?[\d.]+\s*,\s*-?[\d.]+\s*,\s*-?[\d.]+\s*,\s*-?[\d.]+)\s*\]"));
static URL_DEFINITION: LazyLock<Regex> =
    LazyLock::new(|| compile(r"^(?<name>\S+)\s*\[\s*(?<address>[^\]]+?)\s*\]\s*$"));
static ANNOTATION: LazyLock<Regex> =
    LazyLock::new(|| compile(r"^(?<number>\d+)\s*(?<positions>\[.*\])\s*(?<text>.*)$"));

/// Every keyword the DSL starts a statement with, whether or not this module models it: a statement that opens with
/// one is that statement, never a link, even when it holds an arrow (`y_axis Value chain->Invisible->Visible`).
const KNOWN_KEYWORDS: [&str; 21] = [
    "component",
    "anchor",
    "submap",
    "pipeline",
    "title",
    "size",
    "style",
    "evolve",
    "note",
    "annotation",
    "annotations",
    "url",
    "pioneers",
    "settlers",
    "townplanners",
    "accelerator",
    "deaccelerator",
    "y_axis",
    "x_axis",
    "evolution",
    "presentation",
];

/// The keywords this module skips by design, so a skip of one is no problem.
const UNMODELLED_KEYWORDS: [&str; 4] = ["y_axis", "x_axis", "evolution", "presentation"];

fn is_known(keyword: &str) -> bool {
    KNOWN_KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(keyword))
}

fn is_unmodelled(keyword: &str) -> bool {
    UNMODELLED_KEYWORDS.iter().any(|k| k.eq_ignore_ascii_case(keyword))
}

/// What `document` states.
pub fn parse(document: &LineDocument) -> WardleyMap {
    outcome(document).0
}

/// What `text` states.
pub fn parse_text(text: &str) -> WardleyMap {
    parse(&LineDocument::parse(text))
}

/// What `document` states, with a problem for every line it passed over: a statement whose shape it does not
/// recognise, or a line that is no statement at all. Always usable: a map with a bad line still draws every good one.
pub fn outcome(document: &LineDocument) -> (WardleyMap, Vec<ParseProblem>) {
    let mut problems = Vec::new();
    let mut map = WardleyMap::default();
    // The pipeline being filled and its children; none at the top level.
    let mut open: Option<WardleyPipeline> = None;
    let mut children: Vec<WardleyPipelineChild> = Vec::new();

    for (index, line) in document.lines().iter().enumerate() {
        let number = index as u32 + 1;
        let mut skipped = |message: String| problems.push(ParseProblem { line: index, message });
        let text = strip_comment(&line.text).trim();
        if text.is_empty() {
            continue;
        }

        if let Some(pipeline) = open.as_mut() {
            if text.starts_with('}') {
                pipeline.children = std::mem::take(&mut children);
                map.pipelines.push(open.take().expect("open"));
                continue;
            }
            if let Some(child) = parse_pipeline_child(text, number) {
                children.push(child);
            } else if !text.starts_with('{') {
                skipped("This line inside a pipeline is not a component of it, so it was passed over.".to_owned());
            }
            continue;
        }

        // An opening brace on its own line belongs to a pipeline above it, which has already opened.
        if text.starts_with('{') {
            continue;
        }
        if let Some(rest) = keyword(text, "title") {
            map.title = rest.to_owned();
            continue;
        }
        if let Some(rest) = keyword(text, "style") {
            map.style = rest.to_owned();
            continue;
        }
        if let Some(rest) = keyword(text, "size") {
            match parse_size(rest) {
                Some(size) => map.size = Some(size),
                None => skipped(unread("size")),
            }
            continue;
        }
        // Before `annotation`: `annotations [0.6, 0.02]` would otherwise be tried as a numbered annotation.
        if let Some(rest) = keyword(text, "annotations") {
            match coordinate_in(rest) {
                Some(position) => map.annotations_position = Some(position),
                None => skipped(unread("annotations")),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "annotation") {
            match parse_annotation(rest, number) {
                Some(annotation) => map.annotations.push(annotation),
                None => skipped(unread("annotation")),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "evolve") {
            match parse_evolve(rest, number) {
                Some(evolve) => map.evolves.push(evolve),
                None => skipped(unread("evolve")),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "note") {
            match name_and_position(rest) {
                Some((text, position)) => map.notes.push(WardleyNote {
                    text,
                    position,
                    line: number,
                }),
                None => skipped(unread("note")),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "url") {
            match parse_url_definition(rest, number) {
                Some(url) => map.urls.push(url),
                None => skipped(unread("url")),
            }
            continue;
        }
        if let Some((kind, rest)) = attitude_of(text) {
            match parse_attitude(kind, rest, number) {
                Some(attitude) => map.attitudes.push(attitude),
                None => skipped(unread(first_token(text))),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "accelerator") {
            match name_and_position(rest) {
                Some((name, position)) => map.accelerators.push(WardleyAccelerator {
                    name,
                    position,
                    line: number,
                    is_deaccelerator: false,
                }),
                None => skipped(unread("accelerator")),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "deaccelerator") {
            match name_and_position(rest) {
                Some((name, position)) => map.accelerators.push(WardleyAccelerator {
                    name,
                    position,
                    line: number,
                    is_deaccelerator: true,
                }),
                None => skipped(unread("deaccelerator")),
            }
            continue;
        }
        if let Some(rest) = keyword(text, "pipeline") {
            match parse_pipeline_header(rest, number) {
                // The legacy form is complete on its own line: no block follows.
                Some(pipeline) if pipeline.form == WardleyPipelineForm::Legacy => map.pipelines.push(pipeline),
                Some(pipeline) => {
                    open = Some(pipeline);
                    children = Vec::new();
                }
                None => skipped(unread("pipeline")),
            }
            continue;
        }
        if let Some(component) = parse_component(text, number) {
            map.components.push(component);
            continue;
        }
        if let Some(link) = parse_link(text, number) {
            map.links.push(link);
            continue;
        }
        let first = first_token(text);
        if !is_unmodelled(first) {
            skipped(if is_known(first) {
                unread(first)
            } else {
                "This line is not a statement of a Wardley map, so it was passed over.".to_owned()
            });
        }
    }

    // A pipeline that opens and never closes still yields the children it gathered.
    if let Some(mut pipeline) = open {
        pipeline.children = children;
        map.pipelines.push(pipeline);
    }
    (map, problems)
}

fn unread(keyword: &str) -> String {
    format!(
        "This {} statement could not be read, so it was passed over.",
        keyword.to_lowercase()
    )
}

fn first_token(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or_default()
}

/// `keyword rest`: the rest of `text` after `keyword` and the whitespace that must follow it, trimmed, so `titles` is
/// not `title`; the keyword alone gives an empty rest.
pub(crate) fn keyword<'a>(text: &'a str, keyword: &str) -> Option<&'a str> {
    let head = text.get(..keyword.len())?;
    if !head.eq_ignore_ascii_case(keyword) {
        return None;
    }
    let after = &text[keyword.len()..];
    let Some(next) = after.chars().next() else {
        return Some("");
    };
    if !next.is_whitespace() {
        return None;
    }
    Some(after[next.len_utf8()..].trim())
}

fn parse_evolve(rest: &str, line: u32) -> Option<WardleyEvolve> {
    let captures = EVOLVE.captures(rest)?;
    let name = captures["name"].trim();
    let maturity = parse_number(&captures["maturity"])?;
    (!name.is_empty()).then(|| WardleyEvolve {
        name: name.to_owned(),
        maturity,
        line,
        override_name: captures
            .name("override")
            .map(|m| m.as_str().trim().to_owned())
            .unwrap_or_default(),
    })
}

fn attitude_of(text: &str) -> Option<(WardleyAttitudeKind, &str)> {
    if let Some(rest) = keyword(text, "pioneers") {
        return Some((WardleyAttitudeKind::Pioneers, rest));
    }
    if let Some(rest) = keyword(text, "settlers") {
        return Some((WardleyAttitudeKind::Settlers, rest));
    }
    keyword(text, "townplanners").map(|rest| (WardleyAttitudeKind::TownPlanners, rest))
}

/// `[visibility1, maturity1, visibility2, maturity2]`, visibility first.
fn parse_attitude(kind: WardleyAttitudeKind, rest: &str, line: u32) -> Option<WardleyAttitude> {
    let captures = ATTITUDE.captures(rest)?;
    let values: Vec<&str> = captures["values"].split(',').collect();
    if values.len() != 4 {
        return None;
    }
    Some(WardleyAttitude {
        kind,
        from: WardleyCoordinate::new(parse_number(values[0])?, parse_number(values[1])?),
        to: WardleyCoordinate::new(parse_number(values[2])?, parse_number(values[3])?),
        line,
    })
}

/// `url name [address]`: the address is a URL, not a coordinate pair.
fn parse_url_definition(rest: &str, line: u32) -> Option<WardleyUrlDefinition> {
    let captures = URL_DEFINITION.captures(rest)?;
    let name = captures["name"].trim();
    let address = captures["address"].trim();
    (!name.is_empty() && !address.is_empty()).then(|| WardleyUrlDefinition {
        name: name.to_owned(),
        address: address.to_owned(),
        line,
    })
}

/// `annotation N [x, y] text` or `annotation N [[x, y], [x, y]] text`.
fn parse_annotation(rest: &str, line: u32) -> Option<WardleyAnnotation> {
    let captures = ANNOTATION.captures(rest)?;
    let number: i32 = captures["number"].parse().ok()?;
    let occurrences: Vec<WardleyCoordinate> = COORDINATE
        .captures_iter(&captures["positions"])
        .filter_map(|pair| coordinate(&pair["values"]))
        .collect();
    (!occurrences.is_empty()).then(|| WardleyAnnotation {
        number,
        occurrences,
        text: captures["text"].trim().to_owned(),
        line,
    })
}

/// The shape `accelerator`, `deaccelerator` and `note` share: a name, then a position.
fn name_and_position(rest: &str) -> Option<(String, WardleyCoordinate)> {
    let found = COORDINATE.captures(rest)?;
    let whole = found.get(0)?;
    let name = rest[..whole.start()].trim();
    let position = coordinate(&found["values"])?;
    (!name.is_empty()).then(|| (name.to_owned(), position))
}

fn coordinate_in(text: &str) -> Option<WardleyCoordinate> {
    coordinate(&COORDINATE.captures(text)?["values"])
}

fn parse_component(text: &str, line: u32) -> Option<WardleyComponent> {
    let (kind, rest) = kind_of(text)?;
    let found = COORDINATE.captures(rest)?;
    let whole = found.get(0)?;
    let position = coordinate(&found["values"])?;
    let name = rest[..whole.start()].trim();
    if name.is_empty() {
        return None;
    }
    let trailer = &rest[whole.end()..];
    Some(WardleyComponent {
        name: name.to_owned(),
        kind,
        position,
        line,
        decorators: decorators_in(trailer),
        inertia: has_inertia(trailer),
        label_offset: label_offset_in(trailer),
        url: URL
            .captures(trailer)
            .map(|c| c["name"].trim().to_owned())
            .unwrap_or_default(),
    })
}

/// Which of the three statement kinds the line declares, and the rest after the keyword.
fn kind_of(text: &str) -> Option<(WardleyElementKind, &str)> {
    for kind in [
        WardleyElementKind::Component,
        WardleyElementKind::Anchor,
        WardleyElementKind::Submap,
    ] {
        if let Some(rest) = keyword(text, kind.keyword()) {
            return Some((kind, rest));
        }
    }
    None
}

fn parse_link(text: &str, line: u32) -> Option<WardleyLink> {
    if is_known(first_token(text)) {
        return None;
    }
    let captures = LINK.captures(text)?;
    let source = captures["source"].trim();
    let target = captures["target"].trim();
    if source.is_empty() || target.is_empty() {
        return None;
    }
    Some(WardleyLink {
        source: source.to_owned(),
        target: target.to_owned(),
        kind: if &captures["arrow"] == "+>" {
            WardleyLinkKind::Flow
        } else {
            WardleyLinkKind::Dependency
        },
        line,
        context: captures
            .name("context")
            .map(|m| m.as_str().trim().to_owned())
            .unwrap_or_default(),
    })
}

fn parse_pipeline_header(rest: &str, line: u32) -> Option<WardleyPipeline> {
    if rest.is_empty() {
        return None;
    }
    let Some(found) = COORDINATE.captures(rest) else {
        let nested = rest.trim_end_matches('{').trim();
        return (!nested.is_empty()).then(|| WardleyPipeline {
            parent: nested.to_owned(),
            children: Vec::new(),
            line,
            form: WardleyPipelineForm::Nested,
            legacy_extent: None,
        });
    };
    // Two coordinates on the pipeline's own line is the legacy form.
    let whole = found.get(0)?;
    let name = rest[..whole.start()].trim();
    let extent = coordinate(&found["values"])?;
    (!name.is_empty()).then(|| WardleyPipeline {
        parent: name.to_owned(),
        children: Vec::new(),
        line,
        form: WardleyPipelineForm::Legacy,
        legacy_extent: Some(extent),
    })
}

/// A child carries one number, its evolution position; its visibility is the parent's.
fn parse_pipeline_child(text: &str, line: u32) -> Option<WardleyPipelineChild> {
    let rest = keyword(text, "component")?;
    let found = SINGLE_COORDINATE.captures(rest)?;
    let whole = found.get(0)?;
    let name = rest[..whole.start()].trim();
    let maturity = parse_number(&found["value"])?;
    (!name.is_empty()).then(|| WardleyPipelineChild {
        name: name.to_owned(),
        maturity,
        line,
    })
}

fn coordinate(values: &str) -> Option<WardleyCoordinate> {
    let parts: Vec<&str> = values.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    Some(WardleyCoordinate::new(parse_number(parts[0])?, parse_number(parts[1])?))
}

fn parse_size(rest: &str) -> Option<WardleyMapSize> {
    let found = COORDINATE.captures(rest)?;
    let parts: Vec<&str> = found["values"].split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    Some(WardleyMapSize {
        width: parse_number(parts[0])?,
        height: parse_number(parts[1])?,
    })
}

/// The decorators of a component's trailer, each once, in written order; an unknown one is ignored, as the reference
/// parser ignores one from a newer DSL, and the document keeps it.
fn decorators_in(trailer: &str) -> Vec<WardleyDecorator> {
    let mut found = Vec::new();
    for captures in DECORATOR.captures_iter(trailer) {
        if let Some(decorator) = WardleyDecorator::of(&captures["name"])
            && !found.contains(&decorator)
        {
            found.push(decorator);
        }
    }
    found
}

/// Where the bare word `inertia` stands in `text`, in any case, with no letter on either side; none when it does not.
pub(crate) fn inertia_word(text: &str) -> Option<(usize, usize)> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut from = 0;
    while let Some(at) = lower[from..].find("inertia") {
        let start = from + at;
        let end = start + "inertia".len();
        let before = start == 0 || !bytes[start - 1].is_ascii_alphabetic();
        let after = end == bytes.len() || !bytes[end].is_ascii_alphabetic();
        if before && after {
            return Some((start, end));
        }
        from = start + 1;
    }
    None
}

/// `inertia` is a bare word rather than a parenthesised decorator.
fn has_inertia(trailer: &str) -> bool {
    inertia_word(trailer).is_some()
}

fn label_offset_in(trailer: &str) -> Option<WardleyLabelOffset> {
    let found = LABEL.captures(trailer)?;
    let parts: Vec<&str> = found["values"].split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    Some(WardleyLabelOffset {
        x: parse_number(parts[0])?,
        y: parse_number(parts[1])?,
    })
}

//! `atd record --capability <name> --cases <dir>` (etalii.adp spec 035, T050, research R17): a capability's fixtures,
//! recorded from the code that does it in the desktop host today, one per document of the parity scripts that use it,
//! as `$defs/CapabilityFixture` (ATD §2.6). Only the capabilities whose code is named here can be recorded.
//!
//! A reader's and a layout's fixtures of a diagram module are taken from what the module shows once a case's document
//! opens (`adp-parity`): a reader's output is the reading and the findings a conformance case records, a layout's
//! input the nodes and relations of that reading, each node pinned where the registration placed it, and its output
//! where the module drew every node.

use std::fs;
use std::path::{Path, PathBuf};

use adp_atd_validate::parts::{findings, reading};
use adp_diagram::ElementKind;
use adp_diagram_mindmap::document::MindmapDocument;
use adp_diagram_mindmap::layout::{LayoutNode, MindmapMetrics, compute, round2};
use adp_diagram_mindmap::xml::NodeId;
use serde_json::{Map, Value as Json, json};

use super::write_json;

/// Records the fixtures of capability `name` into `cases`; the files written.
pub fn record(name: &str, cases: &Path) -> Result<Vec<PathBuf>, String> {
    match name {
        "layout.twoSidedTree" => two_sided_tree(cases),
        "reader.databricksBundle" => of_modules(name, DATABRICKS, Part::Reader, cases),
        "layout.layeredColumns" => of_modules(name, &["databricks/job", "databricks/pipeline"], Part::Layout, cases),
        "layout.bands" => of_modules(name, &["databricks/bundle"], Part::Layout, cases),
        "reader.structurizr" => of_modules(name, C4, Part::Reader, cases),
        "layout.rankedBoundaries" => super::c4::ranked_boundaries(cases),
        "reader.markdown" => markdown(cases, "reader.markdown", reader_markdown),
        "render.markdown" => markdown(cases, "render.markdown", render_markdown),
        other => Err(format!("no code of the desktop host is named to record `{other}` from")),
    }
}

/// `layout.twoSidedTree` from the mind map's layout (`modules/diagrams/mindmap/src/layout.rs`), on every map of the mind
/// map's parity fixtures and goldens: the nodes with the sizes the layout measures them at, and the top-left corners
/// it places them at.
fn two_sided_tree(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let parity = adp_parity::repository_root().join("parity");
    let mut maps: Vec<(String, PathBuf)> = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("freeplane/mindmap/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let short = name.rsplit('/').next().unwrap_or(&name).to_owned();
        maps.push((
            format!("{short}-read"),
            parity.join("fixtures").join(&case.script.fixture),
        ));
        for changed in case.recorded.changed.iter().filter(|c| c.ends_with(".mm")) {
            maps.push((format!("{short}-golden"), case.golden.join(changed)));
        }
    }
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let metrics = MindmapMetrics::default();
    let mut written = Vec::new();
    for (case, path) in maps {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let document = MindmapDocument::parse(&text, false).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let mut nodes = Vec::new();
        let mut folded = Vec::new();
        walk(&document, document.root(), None, 0, &metrics, &mut nodes, &mut folded);
        let boxes = compute(&LayoutNode::of(&document), &metrics, &|id| {
            folded.iter().any(|f| f == id)
        });
        let positions: Map<String, Json> = boxes
            .iter()
            .map(|(id, rect)| (id.clone(), json!({"x": round2(rect.x), "y": round2(rect.y)})))
            .collect();
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability layout.twoSidedTree` from modules/diagrams/mindmap/src/layout.rs of etalii.adp.ide.desktop, on {}; never edited by hand. A folded node's descendants are not placed.",
                path.strip_prefix(&parity).unwrap_or(&path).display()
            ),
            "input": {"nodes": nodes, "edges": [], "roots": [document.id(document.root())]},
            "args": {},
            "output": {"positions": positions},
        });
        let file = cases.join(format!("{case}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// The nodes of a map, depth first, with their measured sizes, parents, order and the attributes the layout reads.
fn walk(
    document: &MindmapDocument,
    node: NodeId,
    parent: Option<String>,
    order: usize,
    metrics: &MindmapMetrics,
    nodes: &mut Vec<Json>,
    folded: &mut Vec<String>,
) {
    let id = document.id(node);
    let text = document.text(node);
    let (width, height) = metrics.measure(&text);
    let mut attributes = Map::new();
    attributes.insert("text".into(), json!(text));
    if let Some(side) = document.position(node) {
        attributes.insert("side".into(), json!(side));
    }
    if document.folded(node) {
        attributes.insert("folded".into(), json!(true));
        folded.push(id.clone());
    }
    nodes.push(json!({
        "id": id, "width": round2(width), "height": round2(height), "parent": parent, "order": order,
        "attributes": attributes,
    }));
    for (index, child) in document.children(node).into_iter().enumerate() {
        walk(document, child, Some(id.clone()), index, metrics, nodes, folded);
    }
}

const DATABRICKS: &[&str] = &["databricks/bundle", "databricks/job", "databricks/pipeline"];
const C4: &[&str] = &[
    "c4/context",
    "c4/container",
    "c4/component",
    "c4/deployment",
    "c4/dynamic",
    "c4/system-landscape",
];

/// Which part of an opened document a capability's fixture is taken from.
#[derive(Clone, Copy)]
enum Part {
    Reader,
    Layout,
}

/// The arguments the tool description of `origin` passes capability `name`, which are the module's own constants.
fn args_of(name: &str, origin: &str) -> Json {
    match (name, origin) {
        ("reader.databricksBundle", _) => json!({"reading": origin.rsplit('/').next().unwrap_or(origin)}),
        ("reader.structurizr", _) => json!({"view": match origin {
            "c4/context" => "systemContext",
            "c4/system-landscape" => "systemLandscape",
            other => other.rsplit('/').next().unwrap_or(other),
        }}),
        ("layout.layeredColumns", "databricks/job") => json!({
            "columnWidth": 260, "rowHeight": 120, "bandGap": 80, "align": "top",
            "band": "self.id.startsWith('cluster:')", "placeholder": "self.unresolved",
        }),
        ("layout.layeredColumns", _) => json!({
            "columnWidth": 320, "rowHeight": 120, "bandGap": 80, "align": "middle",
            "band": "self.id == 'compute' || self.id == 'notifications'",
            "column": "self.id.startsWith('library:') ? 0 : self.id == 'pipeline' ? 1 : 2",
        }),
        _ => json!({
            "columnWidth": 260,
            "bands": [
                {"members": "self.id == 'bundle'", "y": 0},
                {"members": "self.id.startsWith('resource:') || self.id.startsWith('unknown:')", "y": 180},
                {"members": "self.id.startsWith('target:')", "y": 400},
            ],
        }),
    }
}

/// The fixtures of capability `name` from the cases of `origins`, parity scripts and minted cases alike: one per case
/// whose document opens, named `<reading>-<case>.json`.
fn of_modules(name: &str, origins: &[&str], part: Part, cases: &Path) -> Result<Vec<PathBuf>, String> {
    let of_origins = |n: &String| n.rsplit_once('/').is_some_and(|(tool, _)| origins.contains(&tool));
    let scripts = adp_parity::script_names()
        .into_iter()
        .filter(of_origins)
        .map(|n| adp_parity::Case::load(&n));
    let minted = adp_parity::case_names()
        .into_iter()
        .filter(of_origins)
        .map(|n| adp_parity::Case::load_minted(&n));
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for case in scripts.chain(minted) {
        let case = case?;
        let origin = case.script.tool.clone();
        let mut opened: Option<Json> = None;
        adp_parity::replay_observed(&case, &mut |observed: &adp_parity::Observed| {
            if observed.step.is_some() || opened.is_some() {
                return;
            }
            let elements = observed.session.render(observed.documents).unwrap_or_default();
            opened = Some(match part {
                Part::Reader => {
                    let (nodes, relations) = reading(&elements);
                    json!({"elements": nodes, "relations": relations, "findings": findings(observed.session, observed.documents)})
                }
                Part::Layout => layout_of(&elements),
            });
        })?;
        let Some(opened) = opened else { continue };
        let (input, output) = match part {
            Part::Reader => (reader_input(&case)?, opened),
            Part::Layout => (opened["input"].clone(), opened["output"].clone()),
        };
        let reading_name = origin.rsplit('/').next().unwrap_or(&origin);
        let short = case.name.rsplit('/').next().unwrap_or(&case.name);
        let source = if case.minted { "parity/cases" } else { "parity/scripts" };
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability {name}` from what the desktop host's module shows when {source}/{}.json of etalii.adp.ide.desktop opens its document; never edited by hand.",
                case.name
            ),
            "input": input,
            "args": args_of(name, &origin),
            "output": output,
        });
        let file = cases.join(format!("{reading_name}-{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// A reader's input for a case: the body's text, and the registration's by its file name.
fn reader_input(case: &adp_parity::Case) -> Result<Json, String> {
    let fixtures = adp_parity::repository_root().join(if case.minted { "parity/cases" } else { "parity/fixtures" });
    let read = |path: &str| {
        let path = fixtures.join(path);
        fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
    };
    let mut files = Map::new();
    if let Some(registration) = &case.script.registration {
        let file = Path::new(registration)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned());
        files.insert(file.unwrap_or_default(), json!(read(registration)?));
    }
    Ok(json!({"body": read(&case.script.fixture)?, "files": files}))
}

/// A layout's input and output from what a module drew: the nodes with their sizes, parents and attributes, pinned
/// where their position was authored, the relations as edges, and every node's position.
fn layout_of(elements: &[adp_diagram::DiagramElement]) -> Json {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut positions = Map::new();
    for element in elements {
        if element.kind == ElementKind::Relation {
            edges.push(json!({
                "id": element.id, "source": element.source.clone().unwrap_or_default(),
                "target": element.target.clone().unwrap_or_default(),
            }));
            continue;
        }
        let number = |name: &str| {
            element
                .attributes
                .iter()
                .find(|(n, _)| n == name)
                .and_then(|(_, v)| v.parse::<f64>().ok())
                .unwrap_or(0.0)
        };
        let attributes: Map<String, Json> = element
            .attributes
            .iter()
            .filter(|(n, _)| n != "width" && n != "height")
            .map(|(n, v)| (n.clone(), json!(v)))
            .collect();
        let at = json!({"x": round2(element.x), "y": round2(element.y)});
        let mut node = json!({
            "id": element.id, "type": element.element_type, "width": number("width"), "height": number("height"),
            "attributes": attributes,
        });
        if let Some(parent) = &element.parent {
            node["parent"] = json!(parent);
        }
        if element.placed {
            node["pinned"] = at.clone();
        }
        nodes.push(node);
        positions.insert(element.id.clone(), at);
    }
    json!({"input": {"nodes": nodes, "edges": edges}, "output": {"positions": positions}})
}

/// A capability of Markdown, recorded from the Markdown editor's module (`modules/editors/markdown/src/lib.rs`) on every
/// Markdown document of the editor's examples (`atd record editor/markdown`'s documents): one fixture per document,
/// named after it.
fn markdown(cases: &Path, name: &str, output: fn(&str) -> Json) -> Result<Vec<PathBuf>, String> {
    let source = super::editor::examples().join("markdown/examples");
    let mut files = Vec::new();
    super::collect_files(&source, &source, &mut files);
    files.sort();
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for document in files.iter().filter(|f| f.ends_with(".md") || f.ends_with(".markdown")) {
        let path = source.join(document);
        let file = adp_documents::TextFile::open(&path)?;
        let text = file.content();
        let input = if name.starts_with("reader.") {
            json!({"body": text})
        } else {
            json!({"value": text, "format": "markdown"})
        };
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability {name}` from modules/editors/markdown/src/lib.rs of etalii.adp.ide.desktop, on crates/adp-conformance/tests/round_trip/editors/markdown/examples/{document}; never edited by hand."
            ),
            "input": input,
            "args": {},
            "output": output(text),
        });
        let file = cases.join(format!("{}.json", document.replace('.', "-")));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `reader.markdown`: the ATX headings outside fenced code, each an element `heading` with its level, its title and its
/// line, as the Markdown editor's outline reads them (`headings_of`).
fn reader_markdown(text: &str) -> Json {
    let elements: Vec<Json> = adp_editor_markdown::headings_of(text)
        .into_iter()
        .map(|h| {
            json!({
                "id": format!("heading-{}", h.line), "type": "heading",
                "attributes": {"level": h.level, "title": h.title, "line": h.line},
            })
        })
        .collect();
    json!({"elements": elements, "relations": [], "findings": []})
}

/// `render.markdown`: the Markdown editor's preview (`preview`), each block's runs with the block they stand in
/// (`heading-<level>`, `paragraph`, `item`, `code`, `rule`) and how each is set.
fn render_markdown(text: &str) -> Json {
    use adp_editor_markdown::{Block, Span};
    let span_runs = |spans: &[Span], block: &str, extra: &Map<String, Json>| -> Vec<Json> {
        spans
            .iter()
            .map(|span| {
                let mut style = extra.clone();
                for (key, on) in [
                    ("strong", span.strong),
                    ("emphasis", span.emphasis),
                    ("code", span.code),
                ] {
                    if on {
                        style.insert(key.into(), json!(true));
                    }
                }
                if let Some(link) = &span.link {
                    style.insert("link".into(), json!(link));
                }
                json!({"text": span.text, "style": style, "block": block})
            })
            .collect()
    };
    let mut runs = Vec::new();
    for block in adp_editor_markdown::preview(text) {
        match block {
            Block::Heading { level, spans } => runs.extend(span_runs(&spans, &format!("heading-{level}"), &Map::new())),
            Block::Paragraph { spans, quote_depth } => {
                let mut extra = Map::new();
                if quote_depth > 0 {
                    extra.insert("quoteDepth".into(), json!(quote_depth));
                }
                runs.extend(span_runs(&spans, "paragraph", &extra));
            }
            Block::Item { depth, number, spans } => {
                let mut extra = Map::new();
                extra.insert("depth".into(), json!(depth));
                if let Some(number) = number {
                    extra.insert("number".into(), json!(number));
                }
                runs.extend(span_runs(&spans, "item", &extra));
            }
            Block::Code { language, text } => {
                let mut style = Map::new();
                if !language.is_empty() {
                    style.insert("language".into(), json!(language));
                }
                runs.push(json!({"text": text, "style": style, "block": "code"}));
            }
            Block::Rule => runs.push(json!({"text": "", "style": {}, "block": "rule"})),
        }
    }
    json!({"runs": runs})
}

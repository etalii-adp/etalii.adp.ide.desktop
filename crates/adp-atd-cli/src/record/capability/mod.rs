//! `atd record --capability <name> --cases <dir>` (etalii.adp spec 035, T050, research R17): a capability's fixtures,
//! recorded from the code that does it in the desktop host today, one per document of the parity scripts that use it,
//! as `$defs/CapabilityFixture` (ATD §2.6). Only the capabilities whose code is named here can be recorded.

mod documents;
mod forward_bezier;
mod row_packed;
mod row_packing;
mod self_organising;
mod tidy_tree;

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_mindmap::document::MindmapDocument;
use adp_diagram_mindmap::layout::{LayoutNode, MindmapMetrics, compute, round2};
use adp_diagram_mindmap::xml::NodeId;
use serde_json::{Map, Value as Json, json};

use super::write_json;

/// Where a fixture was recorded from: the capability, the code that does it, and the document it ran on.
struct Recorded<'a> {
    capability: &'a str,
    code: &'a str,
    source: &'a str,
    /// A sentence the fixture's `doc` ends with, or nothing.
    note: &'a str,
}

/// Writes one fixture as `<case>.json` in `cases`: what it was recorded from, its input, its arguments and its output.
fn write_fixture(
    cases: &Path,
    case: &str,
    from: &Recorded,
    input: Json,
    args: Json,
    output: Json,
) -> Result<PathBuf, String> {
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let fixture = json!({
        "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
        "doc": format!(
            "Recorded by `atd record --capability {}` from {} of etalii.adp.ide.desktop, on {}; never edited by hand.{}",
            from.capability,
            from.code,
            from.source,
            if from.note.is_empty() { String::new() } else { format!(" {}", from.note) }
        ),
        "input": input,
        "args": args,
        "output": output,
    });
    let file = cases.join(format!("{case}.json"));
    write_json(&file, &fixture)?;
    Ok(file)
}

/// Records the fixtures of capability `name` into `cases`; the files written.
pub fn record(name: &str, cases: &Path) -> Result<Vec<PathBuf>, String> {
    match name {
        "layout.twoSidedTree" => two_sided_tree(cases),
        "layout.rowPacking" => row_packing::record(cases),
        "layout.rowPacked" => row_packed::record(cases),
        "layout.tidyTree" => tidy_tree::record(cases),
        "arrange.selfOrganising" => self_organising::record(cases),
        "routing.forwardBezier" => forward_bezier::record(cases),
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

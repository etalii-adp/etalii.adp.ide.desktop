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

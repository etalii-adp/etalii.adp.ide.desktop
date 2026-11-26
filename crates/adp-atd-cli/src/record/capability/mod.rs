//! `atd record --capability <name> --cases <dir>` (etalii.adp spec 035, T050, research R17): a capability's fixtures,
//! recorded from the code that does it in the desktop host today, one per document of the parity scripts that use it,
//! as `$defs/CapabilityFixture` (ATD §2.6). Only the capabilities whose code is named here can be recorded.
//!
//! A reader's and a layout's fixtures of a diagram module are taken from what the module shows once a case's document
//! opens (`adp-parity`): a reader's output is the reading and the findings a conformance case records, a layout's
//! input the nodes and relations of that reading, each node pinned where the registration placed it, and its output
//! where the module drew every node.

mod documents;
mod forward_bezier;
mod ring;
mod row_packed;
mod row_packing;
mod self_organising;
mod tidy_tree;

use std::fs;
use std::path::{Path, PathBuf};

use adp_atd_validate::parts::{findings, reading};
use adp_diagram::ElementKind;
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
        "reader.databricksBundle" => of_modules(name, DATABRICKS, Part::Reader, cases),
        "layout.layeredColumns" => of_modules(name, &["databricks/job", "databricks/pipeline"], Part::Layout, cases),
        "layout.bands" => of_modules(name, &["databricks/bundle"], Part::Layout, cases),
        "reader.structurizr" => of_modules(name, C4, Part::Reader, cases),
        "layout.rankedBoundaries" => super::c4::ranked_boundaries(cases),
        "layout.rowPacking" => row_packing::record(cases),
        "layout.rowPacked" => row_packed::record(cases),
        "layout.tidyTree" => tidy_tree::record(cases),
        "layout.ring" => ring::record(cases),
        "arrange.selfOrganising" => self_organising::record(cases),
        "routing.forwardBezier" => forward_bezier::record(cases),
        "reader.markdown" => markdown(cases, "reader.markdown", reader_markdown),
        "reader.helmChart" => helm_chart(cases),
        "reader.ansibleProject" => ansible_project(cases),
        "layout.rankedColumns" => ranked_columns(cases),
        "reader.sparql" => sparql(cases),
        "layout.nestedGrid" => nested_grid(cases),
        "layout.bandedLayers" => banded_layers(cases),
        "layout.flowRelaxation" => flow_relaxation(cases),
        "render.markdown" => markdown(cases, "render.markdown", render_markdown),
        "reader.turtle" => super::w3c::turtle(cases),
        "render.descriptionLogic" => super::w3c::description_logic(cases),
        "layout.bandedGrid" | "layout.grid" | "layout.depthColumns" | "layout.layeredRegions" => {
            super::w3c::layout(name, cases)
        }
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

/// `reader.helmChart` from the Helm chart module's folder reader (`modules/diagrams/helm-chart/src/plugin.rs`), on the
/// chart folder of every Helm chart parity script: the registration as the body, the files the binding's file rules
/// select as `files`, and what the reader reads them to. One fixture per script, named after it.
fn helm_chart(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_helm_chart::definition::HelmDefinition;
    use adp_diagram_helm_chart::plugin::HelmChartFolderPlugin;

    let binding = HelmDefinition::get().binding;
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let fixtures = adp_parity::repository_root().join("parity/fixtures");
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("helm/chart/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let registration = fixtures.join(&case.script.fixture);
        let folder = registration
            .parent()
            .ok_or("The chart's registration is in no folder.")?;
        let mut names = Vec::new();
        super::collect_files(folder, folder, &mut names);
        names.sort();
        let mut files = Map::new();
        for file in adp_fbl::routing::held_files(&binding, Path::new(""), &names, Some(false)) {
            let path = folder.join(&file.relative_path);
            let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.insert(file.relative_path, json!(text));
        }
        let body = fs::read_to_string(&registration).map_err(|e| format!("{}: {e}", registration.display()))?;
        let input = json!({"body": body, "files": files});
        let output = adp_atd_runtime::capabilities::reader_result(&HelmChartFolderPlugin, &input)?;
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability reader.helmChart` from modules/diagrams/helm-chart/src/plugin.rs of etalii.adp.ide.desktop, on the chart folder parity/scripts/{name}.json opens: the registration as the body, the files the binding's file rules select as `files`; never edited by hand."
            ),
            "input": input,
            "args": {},
            "output": output,
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `layout.rankedColumns` from the Helm chart module's anatomy (`modules/diagrams/helm-chart/src/layout.rs`), on the
/// chart of every Helm chart parity script: each node with its size, its column as `rank` and its place in the column
/// as `order`, and the top-left corner the anatomy gives it. One fixture per script, named after it.
fn ranked_columns(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_helm_chart::layout::{columns, compute, size_of};
    use adp_diagram_helm_chart::store::HelmFolder;

    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let fixtures = adp_parity::repository_root().join("parity/fixtures");
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("helm/chart/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let chart = HelmFolder::open(&fixtures.join(&case.script.fixture));
        let diagram = &chart.model.diagram;
        let mut nodes = Vec::new();
        for (rank, column) in columns(diagram).iter().enumerate() {
            for (order, node) in column.iter().enumerate() {
                let (width, height) = size_of(&node.type_name());
                nodes.push(json!({
                    "id": node.id(), "type": node.type_name(), "width": width, "height": height,
                    "attributes": {"rank": rank, "order": order},
                }));
            }
        }
        let positions: Map<String, Json> = compute(diagram)
            .into_iter()
            .map(|(id, rect)| (id, json!({"x": round2(rect.x), "y": round2(rect.y)})))
            .collect();
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability layout.rankedColumns` from modules/diagrams/helm-chart/src/layout.rs of etalii.adp.ide.desktop, on the chart parity/scripts/{name}.json opens: each node's column as `rank` and its place in it as `order`; never edited by hand."
            ),
            "input": {"nodes": nodes, "edges": []},
            "args": {"rank": "self.rank", "order": "self.order", "origin": [40, 40], "columnGap": 60, "rowGap": 24},
            "output": {"positions": positions},
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `reader.sparql` from the SPARQL module's query reader (`modules/diagrams/sparql/src/plugin.rs`), on the query of
/// every SPARQL query parity script: the query as the body, the registration by its name, and the records it reads.
/// One fixture per script, named after it.
fn sparql(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_sparql::plugin::SparqlQueryPlugin;

    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("w3c/sparql/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let input = reader_input(&case)?;
        let output = adp_atd_runtime::capabilities::reader_result(&SparqlQueryPlugin, &input)?;
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability reader.sparql` from modules/diagrams/sparql/src/plugin.rs of etalii.adp.ide.desktop, on the query parity/scripts/{name}.json opens; never edited by hand."
            ),
            "input": input,
            "args": {},
            "output": output,
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `layout.nestedGrid` from the SPARQL module's scope grid (`modules/diagrams/sparql/src/layout.rs`), on the query of
/// every SPARQL parity script: the module's nodes and regions as the containment it lays out, each node in the region of
/// its scope and each region in its parent's, in the order the module places them, a region marked a container; and the
/// top-left corner of every node and every region's frame.
fn nested_grid(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_sparql::{layout, parser, projection};

    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("w3c/sparql/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let text = reader_input(&case)?["body"].as_str().unwrap_or_default().to_owned();
        let Ok(model) = parser::parse(&text) else {
            continue;
        };
        let projected = projection::project(&model);
        let placed = layout::compute(&projected, &Default::default());
        let region_of = |scope: &str| {
            projected
                .regions
                .iter()
                .find(|r| r.scope_path == scope)
                .map(|r| r.id.clone())
        };
        let parent_of = |region: &projection::SparqlRegion| {
            (!region.parent_region_id.is_empty()).then(|| region.parent_region_id.clone())
        };
        let mut nodes = Vec::new();
        for node in &projected.nodes {
            nodes.push(json!({
                "id": node.id, "width": layout::NODE_WIDTH, "height": layout::NODE_HEIGHT,
                "parent": region_of(&node.scope_path),
            }));
        }
        // The template region is laid out last among the roots, beside the where clause's items.
        let (template, others): (Vec<_>, Vec<_>) = projected.regions.iter().partition(|r| r.scope_path == "template");
        for region in others.into_iter().chain(template) {
            nodes.push(json!({
                "id": region.id, "width": layout::NODE_WIDTH, "height": layout::NODE_HEIGHT + 30.0,
                "parent": parent_of(region), "attributes": { "container": "true" },
            }));
        }
        let mut positions = Map::new();
        for node in &nodes {
            let id = node["id"].as_str().unwrap_or_default();
            let at = placed
                .node_positions
                .get(id)
                .copied()
                .or_else(|| placed.region_bounds.get(id).map(|r| (r.x, r.y)));
            if let Some((x, y)) = at {
                positions.insert(id.to_owned(), json!({ "x": round2(x), "y": round2(y) }));
            }
        }
        let mut sizes = Map::new();
        for node in &nodes {
            let id = node["id"].as_str().unwrap_or_default();
            if let Some(frame) = placed.region_bounds.get(id) {
                sizes.insert(
                    id.to_owned(),
                    json!({ "width": round2(frame.width), "height": round2(frame.height) }),
                );
            }
        }
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability layout.nestedGrid` from modules/diagrams/sparql/src/layout.rs of etalii.adp.ide.desktop, on the query parity/scripts/{name}.json opens; never edited by hand."
            ),
            "input": { "nodes": nodes, "edges": [] },
            "args": { "columns": 3, "columnGap": 60, "rowGap": 48, "padding": 36, "labelBand": 30, "origin": [40, 90] },
            "output": if sizes.is_empty() { json!({ "positions": positions }) } else { json!({ "positions": positions, "sizes": sizes }) },
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `layout.bandedLayers` from the supply chain module's layout (`modules/diagrams/supply-chain/src/layout.rs`), on the
/// document of every supply chain parity script: the drawable nodes, each with its stage's column as `minLayer` and
/// its group as `band`, pinned where the document places it; the groups, each its band's frame, pinned where it states
/// a place; the flows as edges; and every node's top-left, every drawn frame's place and size and every lane's route.
fn banded_layers(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_supply_chain::layout::SupplyChainLayout;
    use adp_diagram_supply_chain::model::{geometry, stages};
    use adp_diagram_supply_chain::{lines::LineDocument, parser};

    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("etalii/supply-chain/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let text = reader_input(&case)?["body"].as_str().unwrap_or_default().to_owned();
        let model = parser::parse(&LineDocument::parse(&text));
        let layout = SupplyChainLayout::of(&model);
        let group_ids: std::collections::HashSet<&str> = layout.groups.iter().map(|g| g.id.as_str()).collect();
        let mut nodes = Vec::new();
        for node in &layout.nodes {
            let mut entry = json!({
                "id": node.id, "width": geometry::NODE_WIDTH, "height": geometry::NODE_HEIGHT,
                "attributes": { "minLayer": stages::rank_of(&node.node_type).to_string() },
            });
            if group_ids.contains(node.group.as_str()) {
                entry["attributes"]["band"] = json!(node.group);
            }
            if let (Some(x), Some(y)) = (node.x, node.y) {
                entry["pinned"] = json!({ "x": round2(x), "y": round2(y) });
            }
            nodes.push(entry);
        }
        for group in &layout.groups {
            // A frame names its own band, so that it is one while the band has no member.
            let mut entry = json!({
                "id": group.id, "width": geometry::EMPTY_GROUP_WIDTH, "height": geometry::EMPTY_GROUP_HEIGHT,
                "attributes": { "band": group.id },
            });
            if let (Some(x), Some(y)) = (group.x, group.y) {
                entry["pinned"] = json!({ "x": round2(x), "y": round2(y) });
            }
            nodes.push(entry);
        }
        let edges: Vec<Json> = layout
            .flows
            .iter()
            .map(|flow| json!({ "id": flow.id, "source": flow.from, "target": flow.to }))
            .collect();
        let mut positions = Map::new();
        for (id, frame) in &layout.node_boxes {
            positions.insert(id.clone(), json!({ "x": round2(frame.x), "y": round2(frame.y) }));
        }
        let mut sizes = Map::new();
        for (id, frame) in &layout.group_boxes {
            positions.insert(id.clone(), json!({ "x": round2(frame.x), "y": round2(frame.y) }));
            let members = layout.nodes.iter().any(|n| &n.group == id);
            if members {
                sizes.insert(
                    id.clone(),
                    json!({ "width": round2(frame.width), "height": round2(frame.height) }),
                );
            }
        }
        let routes: Map<String, Json> = layout
            .lanes
            .iter()
            .map(|(id, points)| {
                let points: Vec<Json> = points
                    .iter()
                    .map(|(x, y)| json!({ "x": round2(*x), "y": round2(*y) }))
                    .collect();
                (id.clone(), Json::Array(points))
            })
            .collect();
        let mut output = Map::new();
        output.insert("positions".into(), Json::Object(positions));
        if !sizes.is_empty() {
            output.insert("sizes".into(), Json::Object(sizes));
        }
        if !routes.is_empty() {
            output.insert("routes".into(), Json::Object(routes));
        }
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability layout.bandedLayers` from modules/diagrams/supply-chain/src/layout.rs of etalii.adp.ide.desktop, on the document parity/scripts/{name}.json opens; never edited by hand."
            ),
            "input": { "nodes": nodes, "edges": edges },
            "args": { "layerGap": 128, "rowGap": 28, "bandTitle": 40, "bandBottom": 20, "bandGap": 32, "passes": 8, "lanes": true },
            "output": output,
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `layout.flowRelaxation` from the Sankey module's layout (`modules/diagrams/sankey/src/layout.rs`), on the document of
/// every Sankey parity script: the drawable nodes with their column, 24 wide and as tall as the module measures them;
/// the drawable flows with their weight and the thickness the module stacks them at; and every node's top-left, and
/// the values the contract names: each node's `top`, each edge's `sourceAt` and `targetAt`.
fn flow_relaxation(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_sankey::layout::SankeyLayout;
    use adp_diagram_sankey::model::geometry;
    use adp_diagram_sankey::{lines::LineDocument, parser};

    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("etalii/sankey/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let text = reader_input(&case)?["body"].as_str().unwrap_or_default().to_owned();
        let model = parser::parse(&LineDocument::parse(&text));
        let layout = SankeyLayout::of(&model);
        let height = |id: &str| geometry::MINIMUM_NODE_HEIGHT.max(layout.values[id] * layout.scale);
        let nodes: Vec<Json> = layout
            .nodes
            .iter()
            .map(|node| {
                json!({
                    "id": node.id, "width": geometry::NODE_WIDTH, "height": height(&node.id),
                    "attributes": { "column": layout.columns[&node.id].to_string() },
                })
            })
            .collect();
        let edges: Vec<Json> = layout
            .flows
            .iter()
            .map(|flow| {
                json!({
                    "id": flow.id(), "source": flow.from, "target": flow.to,
                    "attributes": {
                        "weight": adp_fbl::planning::dotnet_double(flow.amount()),
                        "thickness": adp_fbl::planning::dotnet_double(flow.amount() * layout.scale),
                    },
                })
            })
            .collect();
        let mut positions = Map::new();
        let mut values = Map::new();
        for node in &layout.nodes {
            let bar = layout.boxes[&node.id];
            positions.insert(node.id.clone(), json!({ "x": round2(bar.x), "y": round2(bar.y) }));
            values.insert(node.id.clone(), json!({ "top": round2(bar.y) }));
        }
        for flow in &layout.flows {
            let band = layout.bands[&flow.id()];
            values.insert(
                flow.id(),
                json!({ "sourceAt": band.source_at, "targetAt": band.target_at }),
            );
        }
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability layout.flowRelaxation` from modules/diagrams/sankey/src/layout.rs of etalii.adp.ide.desktop, on the document parity/scripts/{name}.json opens; never edited by hand."
            ),
            "input": { "nodes": nodes, "edges": edges },
            "args": { "columnGap": 320, "nodeGap": 36, "passes": 24, "decay": 0.99 },
            "output": { "positions": positions, "values": values },
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// `reader.ansibleProject` from the Ansible structure module's folder reader (`modules/diagrams/ansible-structure/src/
/// plugin.rs`), on the project folder of every Ansible parity script: the registration as the body, the files the
/// binding's file rules select as `files`, and every other folder and file of the folder as its `listing` (FBL 0.26).
fn ansible_project(cases: &Path) -> Result<Vec<PathBuf>, String> {
    use adp_diagram_ansible_structure::definition::AnsibleDefinition;
    use adp_diagram_ansible_structure::plugin::AnsibleFolderPlugin;

    let binding = AnsibleDefinition::get().binding;
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let fixtures = adp_parity::repository_root().join("parity/fixtures");
    let mut written = Vec::new();
    for name in adp_parity::script_names()
        .into_iter()
        .filter(|n| n.starts_with("ansible/structure/"))
    {
        let case = adp_parity::Case::load(&name)?;
        let registration = fixtures.join(&case.script.fixture);
        let folder = registration
            .parent()
            .ok_or("The project's registration is in no folder.")?;
        let mut files = Map::new();
        let selected = adp_fbl::routing::folder_files(&binding, folder, Some(false));
        for file in &selected {
            let text = fs::read_to_string(&file.full_path).map_err(|e| format!("{}: {e}", file.full_path.display()))?;
            files.insert(file.relative_path.clone(), json!(text));
        }
        let registration_name = registration.file_name().map(|n| n.to_string_lossy().into_owned());
        let listing: Vec<String> = adp_fbl::routing::listing(&binding, folder)
            .into_iter()
            .filter(|entry| !files.contains_key(entry) && Some(entry) != registration_name.as_ref())
            .collect();
        let body = fs::read_to_string(&registration).map_err(|e| format!("{}: {e}", registration.display()))?;
        let input = json!({"body": body, "files": files, "listing": listing});
        let output = adp_atd_runtime::capabilities::reader_result(&AnsibleFolderPlugin::handed(), &input)?;
        let short = name.rsplit('/').next().unwrap_or(&name);
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability reader.ansibleProject` from modules/diagrams/ansible-structure/src/plugin.rs of etalii.adp.ide.desktop, on the project folder parity/scripts/{name}.json opens: the registration as the body, the files the binding's file rules select as `files`, every other folder and file as the `listing`; never edited by hand."
            ),
            "input": input,
            "args": {},
            "output": output,
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
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
/// line, as the capability reads them.
fn reader_markdown(text: &str) -> Json {
    adp_atd_runtime::capabilities::reader_result(
        &adp_atd_runtime::capabilities::markdown::MarkdownReader,
        &json!({ "body": text }),
    )
    .unwrap_or_default()
}

/// `render.markdown`: the Markdown editor's preview, as the capability renders it.
fn render_markdown(text: &str) -> Json {
    adp_atd_runtime::capabilities::markdown::runs(text)
}

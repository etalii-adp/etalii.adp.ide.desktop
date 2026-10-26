//! The fixtures of `layout.rankedBoundaries`, recorded from the C4 module's layout (etalii.adp spec 035, T074, T075).
//!
//! A drawn C4 view marks every box as placed, so what it draws cannot say which positions were stored and which were
//! computed; and the edges it draws are not the ones its ranks follow. The input is therefore built from what the
//! module's layout reads: the view's members measured, the boundary nodes that enclose them, the relationships (or a
//! dynamic view's interactions) the ranks follow, and the positions the view's `.layout.json` stores.

use std::fs;
use std::path::{Path, PathBuf};

use adp_diagram_c4::layout::{self, boundary_label_of, deployment_relationships_of, measure_of};
use adp_diagram_c4::metrics::{C4Metrics, round2};
use adp_diagram_c4::model::{C4Element, C4View, C4ViewKind, C4Workspace, same};
use adp_diagram_c4::{parser, rules, sidecar};
use serde_json::{Map, Value as Json, json};

use super::write_json;

const C4: &[&str] = &[
    "c4/context",
    "c4/container",
    "c4/component",
    "c4/deployment",
    "c4/dynamic",
    "c4/system-landscape",
];

/// One fixture per C4 case whose document declares the view it opens, named `<reading>-<case>.json`.
pub fn ranked_boundaries(cases: &Path) -> Result<Vec<PathBuf>, String> {
    let of_origins = |n: &String| n.rsplit_once('/').is_some_and(|(tool, _)| C4.contains(&tool));
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
        let fixtures = adp_parity::repository_root().join(if case.minted { "parity/cases" } else { "parity/fixtures" });
        let body_path = fixtures.join(&case.script.fixture);
        let text = fs::read_to_string(&body_path).map_err(|e| format!("{}: {e}", body_path.display()))?;
        let key = match &case.script.registration {
            Some(registration) => view_key_of(&fixtures.join(registration))?,
            None => None,
        };
        let workspace = parser::parse_text(&text);
        let view = match &key {
            Some(key) => workspace.find_view(key),
            None => workspace.views.first(),
        };
        let Some(view) = view else { continue };
        let members = rules::members_of(&workspace, view);
        if members.is_empty() {
            continue;
        }
        let metrics = C4Metrics::default();
        let stored = sidecar::read(&body_path, &view.key);
        let computed = layout::compute(&workspace, view, &metrics, Some(&stored));
        let input = input_of(&workspace, view, &members, &metrics, &stored);
        let mut positions = Map::new();
        for (id, rect) in computed.boxes.iter() {
            positions.insert(id.to_owned(), json!({"x": rect.x, "y": rect.y}));
        }
        for boundary in &computed.boundaries {
            positions.insert(boundary.id.clone(), json!({"x": boundary.rect.x, "y": boundary.rect.y}));
        }
        let origin = &case.script.tool;
        let reading_name = origin.rsplit('/').next().unwrap_or(origin);
        let short = case.name.rsplit('/').next().unwrap_or(&case.name);
        let source = if case.minted { "parity/cases" } else { "parity/scripts" };
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability layout.rankedBoundaries` from the C4 module's layout of the view that {source}/{}.json of etalii.adp.ide.desktop opens; never edited by hand.",
                case.name
            ),
            "input": input,
            "args": args_of(view, &metrics),
            "output": {"positions": positions},
        });
        let file = cases.join(format!("{reading_name}-{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// The key a registration's `view:` header names, or none.
fn view_key_of(registration: &Path) -> Result<Option<String>, String> {
    let text = fs::read_to_string(registration).map_err(|e| format!("{}: {e}", registration.display()))?;
    Ok(text
        .lines()
        .find_map(|line| line.trim().strip_prefix("view:"))
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty()))
}

/// The arguments the C4 descriptions pass for `view`, each a CEL constant of the value the view's statements give.
fn args_of(view: &C4View, metrics: &C4Metrics) -> Json {
    let auto = view.auto_layout.as_ref();
    let direction = auto.map_or("tb", |a| a.direction.as_str());
    let rank = auto
        .and_then(|a| a.rank_separation)
        .unwrap_or(metrics.rank_separation as i32);
    let node = auto
        .and_then(|a| a.node_separation)
        .unwrap_or(metrics.node_separation as i32);
    json!({
        "mode": if view.kind == C4ViewKind::Deployment { "nest" } else { "enclose" },
        "direction": format!("'{direction}'"),
        "rankSeparation": rank.to_string(),
        "nodeSeparation": node.to_string(),
        "pinsIgnored": auto.is_some().to_string(),
        "padding": metrics.boundary_padding,
        "labelHeight": round2(metrics.font_size * metrics.line_height),
        "clearance": metrics.node_separation,
    })
}

/// The nodes and edges the layout reads: every member, a boundary node for each element drawn as a boundary around
/// what it encloses, and the relations the ranks follow.
fn input_of(
    workspace: &C4Workspace,
    view: &C4View,
    members: &[&C4Element],
    metrics: &C4Metrics,
    stored: &sidecar::Positions,
) -> Json {
    let mut nodes = Vec::new();
    let pin = |id: &str| stored.get(id).map(|(x, y)| json!({"x": round2(x), "y": round2(y)}));
    if view.kind == C4ViewKind::Deployment {
        let host_of = |element: &C4Element| -> Option<String> {
            let mut parent = element.parent_id.clone();
            while let Some(id) = parent {
                if let Some(found) = members.iter().find(|e| same(&e.id, &id)) {
                    return Some(found.id.clone());
                }
                parent = workspace.find(&id).and_then(|e| e.parent_id.clone());
            }
            None
        };
        for element in members {
            let host = host_of(element).map(|h| format!("boundary:{h}"));
            let hosts = members
                .iter()
                .any(|e| host_of(e).is_some_and(|h| same(&h, &element.id)));
            let mut node = if hosts {
                let label = metrics.text_width(&boundary_label_of(element)) + 2.0 * metrics.horizontal_padding;
                json!({
                    "id": format!("boundary:{}", element.id), "type": kind_of(element), "width": label, "height": 0.0,
                    "attributes": {"boundary": true},
                })
            } else {
                let size = measure_of(workspace, element, metrics);
                json!({"id": element.id, "type": kind_of(element), "width": size.width, "height": size.height})
            };
            if let Some(host) = host {
                node["parent"] = json!(host);
            }
            if !hosts && let Some(at) = pin(&element.id) {
                node["pinned"] = at;
            }
            nodes.push(node);
        }
        let edges = deployment_relationships_of(workspace, members)
            .into_iter()
            .map(|r| (r.source_id, r.destination_id));
        return json!({"nodes": nodes, "edges": edges_of(edges)});
    }

    let scope = view
        .scope_id
        .as_deref()
        .filter(|_| matches!(view.kind, C4ViewKind::Container | C4ViewKind::Component))
        .and_then(|id| workspace.find(id))
        .filter(|scope| {
            members
                .iter()
                .any(|e| e.parent_id.as_deref().is_some_and(|p| same(p, &scope.id)))
        });
    for element in members {
        let size = measure_of(workspace, element, metrics);
        let mut node = json!({"id": element.id, "type": kind_of(element), "width": size.width, "height": size.height});
        if let Some(scope) = scope
            && element.parent_id.as_deref().is_some_and(|p| same(p, &scope.id))
        {
            node["parent"] = json!(format!("boundary:{}", scope.id));
        }
        if let Some(at) = pin(&element.id) {
            node["pinned"] = at;
        }
        nodes.push(node);
    }
    if let Some(scope) = scope {
        nodes.push(json!({
            "id": format!("boundary:{}", scope.id), "type": kind_of(scope), "width": 0.0, "height": 0.0,
            "attributes": {"boundary": true},
        }));
    }
    let edges: Vec<(String, String)> = if view.kind == C4ViewKind::Dynamic {
        view.interactions
            .iter()
            .map(|i| (i.source_id.clone(), i.destination_id.clone()))
            .collect()
    } else {
        workspace
            .relationships
            .iter()
            .map(|r| (r.source_id.clone(), r.destination_id.clone()))
            .collect()
    };
    json!({"nodes": nodes, "edges": edges_of(edges)})
}

/// The kind of an element, as a sentence spells it.
fn kind_of(element: &C4Element) -> String {
    rules::spell(element.kind)
}

/// Edges in their order, each id its ends and its position among them.
fn edges_of(edges: impl IntoIterator<Item = (String, String)>) -> Vec<Json> {
    edges
        .into_iter()
        .enumerate()
        .map(|(n, (source, target))| json!({"id": format!("{source}->{target}#{}", n + 1), "source": source, "target": target}))
        .collect()
}

//! Turns a projection and its geometry into what the shell draws (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlElementMapper.cs`, whose protobuf payloads (`api/sparql.proto`) are carried here as the element's attributes.
//! Regions come first so a frame is behind its contents, then nodes, edges, annotations, the header band, and the
//! banner when the sanity bound cut. A region and a node are placed by their top-left; an edge, an annotation, the
//! header and the banner sit at the origin. The viewport admits every region whose frame it touches, every node whose
//! box it touches, one hop outwards along the edges, every edge whose two ends are delivered, and always the header
//! and the banner.

use std::collections::HashSet;

use adp_diagram::element::Rect;
use adp_diagram::{DiagramElement, DiagramViewport, ElementKind};

use crate::layout::{NODE_HEIGHT, NODE_WIDTH, SparqlLayoutResult};
use crate::projection::{SparqlNodeKind, SparqlProjectionResult};

/// The kinds the canvas switches on.
pub const VARIABLE_TYPE: &str = "w3c/sparql+variable";
pub const TERM_TYPE: &str = "w3c/sparql+term";
pub const EDGE_TYPE: &str = "w3c/sparql+edge";
pub const REGION_TYPE: &str = "w3c/sparql+region";
pub const ANNOTATION_TYPE: &str = "w3c/sparql+annotation";
pub const HEADER_TYPE: &str = "w3c/sparql+header";
pub const TRUNCATION_TYPE: &str = "w3c/sparql+truncation";

/// The header band's element id: one per diagram, always present.
pub const HEADER_ID: &str = "header:query";

/// The truncation banner's element id: only when the sanity bound cut.
pub const TRUNCATION_ID: &str = "truncation";

fn element(id: &str, (x, y): (f64, f64), element_type: &str, attributes: Vec<(&str, String)>) -> DiagramElement {
    DiagramElement {
        id: id.to_owned(),
        x,
        y,
        placed: false,
        element_type: element_type.to_owned(),
        kind: ElementKind::Node,
        parent: None,
        source: None,
        target: None,
        attributes: attributes.into_iter().map(|(n, v)| (n.to_owned(), v)).collect(),
    }
}

/// Every element of `projection`, in the order it is drawn.
pub fn elements(projection: &SparqlProjectionResult, layout: &SparqlLayoutResult) -> Vec<DiagramElement> {
    let mut elements = Vec::new();
    for region in &projection.regions {
        let frame = layout
            .region_bounds
            .get(&region.id)
            .copied()
            .unwrap_or(adp_layout::Rect::new(0.0, 0.0, 0.0, 0.0));
        let mut drawn = element(
            &region.id,
            (frame.x, frame.y),
            REGION_TYPE,
            vec![
                ("kind", region.kind.clone()),
                ("label", region.label.clone()),
                ("parentRegionId", region.parent_region_id.clone()),
                ("width", frame.width.to_string()),
                ("height", frame.height.to_string()),
            ],
        );
        drawn.parent = (!region.parent_region_id.is_empty()).then(|| region.parent_region_id.clone());
        elements.push(drawn);
    }
    for node in &projection.nodes {
        let at = layout.node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
        let drawn = match node.kind {
            SparqlNodeKind::Variable | SparqlNodeKind::Anonymous => element(
                &node.id,
                at,
                VARIABLE_TYPE,
                vec![
                    ("name", node.display.clone()),
                    ("projected", node.projected.to_string()),
                    ("joinCount", node.join_count.to_string()),
                    ("definingExpression", node.defining_expression.clone()),
                    ("anonymous", (node.kind == SparqlNodeKind::Anonymous).to_string()),
                ],
            ),
            // A collapsed subquery travels as a term whose display is its projection.
            SparqlNodeKind::SubSelect => element(
                &node.id,
                at,
                TERM_TYPE,
                vec![
                    ("display", node.display.clone()),
                    ("full", node.full.clone()),
                    ("annotation", "subquery".to_owned()),
                    ("kind", "unspecified".to_owned()),
                ],
            ),
            SparqlNodeKind::Iri | SparqlNodeKind::Literal => element(
                &node.id,
                at,
                TERM_TYPE,
                vec![
                    ("display", node.display.clone()),
                    ("full", node.full.clone()),
                    ("annotation", node.annotation.clone()),
                    (
                        "kind",
                        if node.kind == SparqlNodeKind::Iri {
                            "iri"
                        } else {
                            "literal"
                        }
                        .to_owned(),
                    ),
                ],
            ),
        };
        elements.push(drawn);
    }
    for edge in &projection.edges {
        let mut drawn = element(
            &edge.id,
            (0.0, 0.0),
            EDGE_TYPE,
            vec![
                ("fromElementId", edge.from_id.clone()),
                ("toElementId", edge.to_id.clone()),
                ("label", edge.label.clone()),
                ("isPath", edge.is_path.to_string()),
            ],
        );
        drawn.kind = ElementKind::Relation;
        drawn.source = Some(edge.from_id.clone());
        drawn.target = (!edge.to_id.is_empty()).then(|| edge.to_id.clone());
        elements.push(drawn);
    }
    for annotation in &projection.annotations {
        elements.push(element(
            &annotation.id,
            (0.0, 0.0),
            ANNOTATION_TYPE,
            vec![
                ("kind", annotation.kind.clone()),
                ("text", annotation.text.clone()),
                ("attachedTo", annotation.attached_to_id.clone()),
            ],
        ));
    }
    let mut header = element(
        HEADER_ID,
        (0.0, 0.0),
        HEADER_TYPE,
        vec![
            ("form", projection.header_form.clone()),
            ("modifierRows", projection.header_rows.len().to_string()),
        ],
    );
    for (i, row) in projection.header_rows.iter().enumerate() {
        header.attributes.push((format!("modifierRow.{i}"), row.clone()));
    }
    elements.push(header);
    if projection.truncated() {
        elements.push(element(
            TRUNCATION_ID,
            (0.0, 0.0),
            TRUNCATION_TYPE,
            vec![
                ("shown", projection.shown.to_string()),
                ("total", projection.total.to_string()),
            ],
        ));
    }
    elements
}

/// The header's modifier rows, as [`elements`] carries them.
pub fn modifier_rows(header: &DiagramElement) -> Vec<String> {
    let count: usize = header.attribute("modifierRows").parse().unwrap_or(0);
    (0..count)
        .map(|i| header.attribute(&format!("modifierRow.{i}")).to_owned())
        .collect()
}

/// The elements `viewport` admits (`SparqlElementMapper.Visible`).
pub fn visible(
    projection: &SparqlProjectionResult,
    layout: &SparqlLayoutResult,
    viewport: DiagramViewport,
) -> Vec<DiagramElement> {
    let mut delivered: HashSet<String> = HashSet::new();
    for region in &projection.regions {
        if let Some(frame) = layout.region_bounds.get(&region.id)
            && viewport.intersects(Rect::new(frame.x, frame.y, frame.width, frame.height))
        {
            delivered.insert(region.id.clone());
        }
    }
    for node in &projection.nodes {
        let (x, y) = layout.node_positions.get(&node.id).copied().unwrap_or((0.0, 0.0));
        if viewport.intersects(Rect::new(x, y, NODE_WIDTH, NODE_HEIGHT)) {
            delivered.insert(node.id.clone());
        }
    }
    // One hop, exactly: an edge with one end in view brings the other end with it.
    for edge in &projection.edges {
        if delivered.contains(&edge.from_id) && !edge.to_id.is_empty() {
            delivered.insert(edge.to_id.clone());
        } else if !edge.to_id.is_empty() && delivered.contains(&edge.to_id) {
            delivered.insert(edge.from_id.clone());
        }
    }
    for edge in &projection.edges {
        if delivered.contains(&edge.from_id) && (edge.to_id.is_empty() || delivered.contains(&edge.to_id)) {
            delivered.insert(edge.id.clone());
        }
    }
    // Anything that is neither region, node nor edge has no position to test, and travels always.
    let positioned: HashSet<&str> = projection
        .regions
        .iter()
        .map(|r| r.id.as_str())
        .chain(projection.nodes.iter().map(|n| n.id.as_str()))
        .chain(projection.edges.iter().map(|e| e.id.as_str()))
        .collect();
    elements(projection, layout)
        .into_iter()
        .filter(|e| !positioned.contains(e.id.as_str()) || delivered.contains(&e.id))
        .collect()
}

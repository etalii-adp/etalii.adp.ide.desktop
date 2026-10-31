//! The data-graph reading, `w3c/rdf` (etalii.adp spec 030, task T083): what a model draws, and where.
//!
//! Ported from the standalone host's `RdfProjection.cs`, `RdfLayout.cs` and `_Model` (`RdfProjectionNode`,
//! `RdfProjectionEdge`, `RdfProjectionRow`, `RdfProjectionResult`) at `8b6ccfc`. IRI subjects and objects draw as cards;
//! literals fold into their subject's rows; `rdf:type` triples become badges; collections draw as ordered member edges
//! while their cons pairs stay in the file; blank nodes draw, marked. The budget counts drawn nodes in document order,
//! so the same file always truncates the same way.

use std::collections::{BTreeMap, HashMap};

use adp_atd_runtime::capabilities::banded_grid::{BandedGridArguments, place_banded};

use crate::model::{BlankTerm, LiteralTerm, RdfModel, RdfTerm};
use crate::vocabulary::rdf;
use crate::writer;

/// The drawn-node budget the family's specifications name.
pub const DEFAULT_BUDGET: usize = 1000;

/// One literal property row inside its subject's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfRow {
    pub predicate: String,
    pub value: String,
    /// A language tag with its `@`, a compressed datatype, or empty for a plain string.
    pub annotation: String,
}

/// One drawn resource: a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfNode {
    /// `res:{iri}`, or `blank:{ordinal}`.
    pub id: String,
    /// Empty for a blank node.
    pub iri: String,
    pub display: String,
    pub types: Vec<String>,
    pub rows: Vec<RdfRow>,
    pub blank: bool,
}

/// One drawn edge: a triple between two drawn terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfEdge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub predicate: String,
    pub predicate_iri: String,
}

/// What the data-graph reading draws, and the honest count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdfProjection {
    pub nodes: Vec<RdfNode>,
    pub edges: Vec<RdfEdge>,
    pub shown: usize,
    pub total: usize,
}

impl RdfProjection {
    /// Whether the budget cut the graph down.
    pub fn truncated(&self) -> bool {
        self.shown < self.total
    }
}

/// The cons cells of `model`'s collections: each blank subject's `rdf:first` and `rdf:rest`.
pub fn cons_cells(model: &RdfModel) -> (HashMap<BlankTerm, RdfTerm>, HashMap<BlankTerm, RdfTerm>) {
    let mut firsts = HashMap::new();
    let mut rests = HashMap::new();
    for triple in &model.triples {
        let RdfTerm::Blank(cell) = &triple.subject else {
            continue;
        };
        match triple.predicate.iri.as_str() {
            rdf::FIRST => {
                firsts.insert(cell.clone(), triple.object.clone());
            }
            rdf::REST => {
                rests.insert(cell.clone(), triple.object.clone());
            }
            _ => {}
        }
    }
    (firsts, rests)
}

struct Builder<'a> {
    model: &'a RdfModel,
    firsts: HashMap<BlankTerm, RdfTerm>,
    nodes: HashMap<String, RdfNode>,
    order: Vec<String>,
    edges: Vec<RdfEdge>,
    edge_counts: HashMap<String, usize>,
}

impl Builder<'_> {
    fn ensure(&mut self, term: &RdfTerm) -> Option<String> {
        match term {
            RdfTerm::Iri(iri) => {
                let id = format!("res:{}", iri.iri);
                if !self.nodes.contains_key(&id) {
                    self.nodes.insert(
                        id.clone(),
                        RdfNode {
                            id: id.clone(),
                            iri: iri.iri.clone(),
                            display: display(self.model, &iri.iri),
                            types: Vec::new(),
                            rows: Vec::new(),
                            blank: false,
                        },
                    );
                    self.order.push(id.clone());
                }
                Some(id)
            }
            RdfTerm::Blank(blank) if !self.firsts.contains_key(blank) => {
                let id = format!("blank:{}", blank.ordinal);
                if !self.nodes.contains_key(&id) {
                    self.nodes.insert(
                        id.clone(),
                        RdfNode {
                            id: id.clone(),
                            iri: String::new(),
                            display: blank_display(blank),
                            types: Vec::new(),
                            rows: Vec::new(),
                            blank: true,
                        },
                    );
                    self.order.push(id.clone());
                }
                Some(id)
            }
            _ => None,
        }
    }

    fn edge(&mut self, from_id: &str, to_id: &str, predicate: &str, predicate_iri: &str) {
        let key = format!("{from_id}|{predicate_iri}|{to_id}");
        let count = self.edge_counts.entry(key.clone()).or_insert(0);
        let id = if *count == 0 {
            format!("edge:{key}")
        } else {
            format!("edge:{key}|{count}")
        };
        *count += 1;
        self.edges.push(RdfEdge {
            id,
            from_id: from_id.to_owned(),
            to_id: to_id.to_owned(),
            predicate: predicate.to_owned(),
            predicate_iri: predicate_iri.to_owned(),
        });
    }

    fn row(&mut self, subject_id: &str, predicate: String, literal: &LiteralTerm) {
        let annotation = annotation(self.model, literal);
        if let Some(node) = self.nodes.get_mut(subject_id) {
            node.rows.push(RdfRow {
                predicate,
                value: literal.lexical.clone(),
                annotation,
            });
        }
    }
}

/// A blank node's display form: `_:label`, or `_:b{ordinal}`.
pub fn blank_display(blank: &BlankTerm) -> String {
    match blank.label.as_deref().filter(|l| !l.is_empty()) {
        Some(label) => format!("_:{label}"),
        None => format!("_:b{}", blank.ordinal),
    }
}

/// What `model` draws, cut to `budget` nodes (`RdfProjection.Project`).
pub fn project(model: &RdfModel, budget: usize) -> RdfProjection {
    let (firsts, rests) = cons_cells(model);
    let mut builder = Builder {
        model,
        firsts,
        nodes: HashMap::new(),
        order: Vec::new(),
        edges: Vec::new(),
        edge_counts: HashMap::new(),
    };
    for triple in &model.triples {
        let is_plumbing = matches!(triple.predicate.iri.as_str(), rdf::FIRST | rdf::REST)
            && triple
                .subject
                .as_blank()
                .is_some_and(|b| builder.firsts.contains_key(b));
        if is_plumbing {
            continue;
        }
        let Some(subject_id) = builder.ensure(&triple.subject) else {
            continue;
        };
        if triple.predicate.iri == rdf::TYPE
            && let RdfTerm::Iri(type_iri) = &triple.object
        {
            let badge = display(model, &type_iri.iri);
            if let Some(node) = builder.nodes.get_mut(&subject_id) {
                node.types.push(badge);
            }
            continue;
        }
        let predicate = display(model, &triple.predicate.iri);
        match &triple.object {
            RdfTerm::Literal(literal) => builder.row(&subject_id, predicate, literal),
            RdfTerm::Blank(head) if builder.firsts.contains_key(head) => {
                let mut index = 1;
                let mut cursor = RdfTerm::Blank(head.clone());
                while let RdfTerm::Blank(cell) = &cursor {
                    let Some(member) = builder.firsts.get(cell).cloned() else {
                        break;
                    };
                    let label = format!("{predicate} [{index}]");
                    match &member {
                        RdfTerm::Literal(literal) => builder.row(&subject_id, label, literal),
                        _ => {
                            if let Some(member_id) = builder.ensure(&member) {
                                builder.edge(&subject_id, &member_id, &label, &triple.predicate.iri);
                            }
                        }
                    }
                    index += 1;
                    cursor = rests
                        .get(cell)
                        .cloned()
                        .unwrap_or_else(|| RdfTerm::iri(rdf::NIL, "rdf:nil"));
                }
            }
            _ => {
                if let Some(object_id) = builder.ensure(&triple.object) {
                    builder.edge(&subject_id, &object_id, &predicate, &triple.predicate.iri);
                }
            }
        }
    }
    let total = builder.order.len();
    let kept: Vec<String> = builder.order.iter().take(budget).cloned().collect();
    let kept_set: std::collections::HashSet<&String> = kept.iter().collect();
    let edges = builder
        .edges
        .into_iter()
        .filter(|e| kept_set.contains(&e.from_id) && kept_set.contains(&e.to_id))
        .collect();
    let mut nodes_by_id = builder.nodes;
    let nodes: Vec<RdfNode> = kept.iter().filter_map(|id| nodes_by_id.remove(id)).collect();
    RdfProjection {
        shown: nodes.len(),
        nodes,
        edges,
        total,
    }
}

/// A term's display form: its prefixed name under the document's own declarations, else the IRI's local name, else
/// the full IRI (`RdfProjection.Display`).
pub fn display(model: &RdfModel, iri: &str) -> String {
    let compressed = writer::compress(model, iri);
    if !compressed.starts_with('<') {
        return compressed;
    }
    local_part(iri)
}

/// What follows the last `#` or `/` of `iri`, or `iri` itself when nothing does.
pub fn local_part(iri: &str) -> String {
    match iri.rfind(['#', '/']) {
        Some(cut) if cut < iri.len() - 1 => iri[cut + 1..].to_owned(),
        _ => iri.to_owned(),
    }
}

/// A literal's annotation: its language tag with `@`, a compressed datatype other than `xsd:string`, or nothing.
pub fn annotation(model: &RdfModel, literal: &LiteralTerm) -> String {
    if let Some(language) = literal.language.as_deref().filter(|l| !l.is_empty()) {
        return format!("@{language}");
    }
    match literal.datatype.as_deref() {
        Some(datatype) if !datatype.is_empty() && datatype != rdf::XSD_STRING => writer::compress(model, datatype),
        _ => String::new(),
    }
}

/// The cell the data-graph layout reserves for one node, and the box a viewport intersects it by.
pub const CELL_WIDTH: f64 = 280.0;
pub const CELL_HEIGHT: f64 = 160.0;
const BAND_GAP: f64 = 80.0;
const COLUMNS: usize = 4;

/// The computed layout: bands by first type badge, ordered by name with the untyped band last, grid-placed within a
/// band in projection order (`RdfLayout.Positions`): the capability `layout.bandedGrid`.
pub fn positions(result: &RdfProjection) -> BTreeMap<String, (f64, f64)> {
    let nodes: Vec<(String, String)> = result
        .nodes
        .iter()
        .map(|node| (node.id.clone(), node.types.first().cloned().unwrap_or_default()))
        .collect();
    let arguments = BandedGridArguments {
        columns: COLUMNS,
        column_width: CELL_WIDTH,
        row_height: CELL_HEIGHT,
        band_gap: BAND_GAP,
    };
    place_banded(&nodes, &arguments)
        .into_iter()
        .map(|(id, x, y)| (id, (x, y)))
        .collect()
}

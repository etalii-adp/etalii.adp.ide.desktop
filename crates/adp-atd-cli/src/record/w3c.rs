//! The fixtures of the W3C readings' layouts, recorded from the RDF module (etalii.adp spec 035, T078, T079).
//!
//! A drawn W3C reading marks every box as placed and sizes none, so what it draws cannot say which positions were
//! stored and how big a box is. The input is therefore built from what each reading's layout reads: its projection's
//! nodes with their sizes and the attributes the layout groups them by, the edges it follows, and the positions the
//! registration's `layout:` block stores.

use std::fs;
use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use adp_diagram::problems::DiagramProblemLocation;
use adp_diagram_rdf::graph::RdfReading;
use adp_diagram_rdf::model::{RdfModel, RdfTerm};
use adp_diagram_rdf::owl::OwlReading;
use adp_diagram_rdf::owl::expression;
use adp_diagram_rdf::owl::index::OwlModelIndex;
use adp_diagram_rdf::owl::layout::size_of;
use adp_diagram_rdf::owl::projection::{OwlEdgeKind, OwlNodeKind};
use adp_diagram_rdf::projection::{CELL_HEIGHT, CELL_WIDTH};
use adp_diagram_rdf::reading::{Positions, Reading};
use adp_diagram_rdf::shacl::{ShaclReading, card_height};
use adp_diagram_rdf::skos::projection::SkosEdgeKind;
use adp_diagram_rdf::skos::{SkosReading, registration_language};
use adp_diagram_rdf::store::RdfDocument;
use adp_diagram_rdf::validator;
use adp_diagram_rdf::vocabulary::owl;
use adp_diagram_rdf::{registration_layout, shacl};
use serde_json::{Map, Value as Json, json};

use super::write_json;

/// The layout capability each W3C reading is laid out by.
pub fn layout(name: &str, cases: &Path) -> Result<Vec<PathBuf>, String> {
    let origin = match name {
        "layout.bandedGrid" => "w3c/rdf",
        "layout.grid" => "w3c/shacl",
        "layout.depthColumns" => "w3c/owl",
        _ => "w3c/skos",
    };
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for opened in opened_of(origin)? {
        let Opened {
            case,
            document,
            registration,
            stored,
        } = opened;
        let (input, positions) = match name {
            "layout.bandedGrid" => banded_grid(&document, &stored),
            "layout.grid" => grid(&document, &stored),
            "layout.depthColumns" => depth_columns(&document, &stored),
            _ => layered_regions(&document, registration.as_deref(), &stored),
        };
        if input["nodes"].as_array().is_none_or(Vec::is_empty) {
            continue;
        }
        let mut output = Map::new();
        for (id, (x, y)) in &positions {
            output.insert(id.clone(), json!({"x": x, "y": y}));
        }
        let short = case.name.rsplit('/').next().unwrap_or(&case.name);
        let source = if case.minted { "parity/cases" } else { "parity/scripts" };
        let fixture = json!({
            "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
            "doc": format!(
                "Recorded by `atd record --capability {name}` from the RDF module's layout of the reading that {source}/{}.json of etalii.adp.ide.desktop opens; never edited by hand.",
                case.name
            ),
            "input": input,
            "args": args_of(name),
            "output": {"positions": output},
        });
        let file = cases.join(format!("{short}.json"));
        write_json(&file, &fixture)?;
        written.push(file);
    }
    Ok(written)
}

/// One fixture per body the W3C cases open, whichever reading opens it, named after the first case that opens it: the
/// stored triples, prefix declarations and base the body states, and the findings every reading of it shares.
pub fn turtle(cases: &Path) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    let mut bodies: Vec<String> = Vec::new();
    for origin in ["w3c/rdf", "w3c/owl", "w3c/shacl", "w3c/skos"] {
        for Opened {
            case,
            document,
            registration,
            ..
        } in opened_of(origin)?
        {
            let fixtures =
                adp_parity::repository_root().join(if case.minted { "parity/cases" } else { "parity/fixtures" });
            let body_path = fixtures.join(&case.script.fixture);
            let body = fs::read(&body_path).map_err(|e| format!("{}: {e}", body_path.display()))?;
            let body = String::from_utf8_lossy(&body).into_owned();
            let mut files = Map::new();
            if let Some(path) = &registration {
                let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                files.insert(name, json!(text));
            }
            let key = format!("{body}\u{0}{}", document_of(registration.as_deref()));
            if bodies.contains(&key) {
                continue;
            }
            bodies.push(key);
            let short = case.name.rsplit('/').next().unwrap_or(&case.name);
            let source = if case.minted { "parity/cases" } else { "parity/scripts" };
            let findings: Vec<Json> = validator::validate(&document.text)
                .into_iter()
                .map(|p| {
                    let line = match p.location {
                        Some(DiagramProblemLocation::Line(line)) => json!(line),
                        _ => Json::Null,
                    };
                    json!({"rule": p.rule_id, "severity": format!("{:?}", p.severity).to_lowercase(), "line": line, "message": p.message})
                })
                .collect();
            let mut elements = vec![document_of(registration.as_deref())];
            elements.extend(stored_of(&document.model));
            let mut input = json!({"body": body});
            if !files.is_empty() {
                input["files"] = Json::Object(files);
            }
            let fixture = json!({
                "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
                "doc": format!(
                    "Recorded by `atd record --capability reader.turtle` from the RDF module's parser and family findings on the body that {source}/{}.json of etalii.adp.ide.desktop opens; never edited by hand.",
                    case.name
                ),
                "input": input,
                "args": {"prefixes": "keep"},
                "output": {"elements": elements, "relations": [], "findings": findings},
            });
            let file = cases.join(format!("{short}.json"));
            write_json(&file, &fixture)?;
            written.push(file);
        }
    }
    Ok(written)
}

/// The element `document`, which carries what the registration's headers say: `language`, the display language, when
/// one is honoured, and `languageLine`, the line of a `language:` header that is ignored for standing above a core
/// header, 0 when none is.
fn document_of(registration: Option<&Path>) -> Json {
    let (language, misplaced) = registration_language::read(registration);
    let mut attributes = json!({"languageLine": misplaced});
    if let Some(language) = language {
        attributes["language"] = json!(language);
    }
    json!({"id": "document", "type": "document", "parent": null, "attributes": attributes})
}

/// The base, the prefix declarations and the triples of `model` in document order, as stored elements with the ids
/// the W3C descriptions give them: `base`, `prefix:<name>` and `triple:<s>|<p>|<o>`, `|<n>` added for the n-th repeat.
fn stored_of(model: &RdfModel) -> Vec<Json> {
    let mut elements = Vec::new();
    if let Some(iri) = &model.base_iri {
        elements.push(json!({"id": "base", "type": "Base", "parent": null, "attributes": {"iri": iri}}));
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for declaration in &model.prefixes {
        let id = repeated(&mut seen, format!("prefix:{}", declaration.prefix));
        elements.push(json!({
            "id": id, "type": "Prefix", "parent": null,
            "attributes": {"name": declaration.prefix, "iri": declaration.iri, "line": declaration.line + 1},
        }));
    }
    for triple in &model.triples {
        let (s, s_kind, s_label, s_written) = stored_term(&triple.subject);
        let (o, o_kind, o_label, _) = stored_term(&triple.object);
        let literal = triple.object.as_literal();
        let id = repeated(&mut seen, format!("triple:{s}|{}|{o}", triple.predicate.iri));
        elements.push(json!({
            "id": id, "type": "Triple", "parent": null,
            "attributes": {
                "s": s, "sKind": s_kind, "sLabel": s_label, "sWritten": s_written,
                "p": triple.predicate.iri,
                "o": o, "oKind": o_kind, "oLabel": o_label,
                "language": literal.and_then(|l| l.language.clone()).unwrap_or_default(),
                "datatype": literal.and_then(|l| l.datatype.clone()).unwrap_or_default(),
                "line": triple.span.start_line + 1,
            },
        }));
    }
    elements
}

/// `id`, with `|<n>` added when it is the n-th repeat of an id already given.
fn repeated(seen: &mut BTreeMap<String, usize>, id: String) -> String {
    let count = seen.entry(id.clone()).or_insert(0);
    let out = if *count == 0 {
        id.clone()
    } else {
        format!("{id}|{count}")
    };
    *count += 1;
    out
}

/// A term as a stored triple holds it: its value (an IRI, a blank node's ordinal, a literal's lexical form), its kind,
/// a blank node's label as written, and an IRI's spelling as written.
fn stored_term(term: &RdfTerm) -> (String, &'static str, String, String) {
    match term {
        RdfTerm::Iri(iri) => (iri.iri.clone(), "iri", String::new(), iri.as_written.clone()),
        RdfTerm::Blank(blank) => (
            blank.ordinal.to_string(),
            "blank",
            blank.label.clone().unwrap_or_default(),
            String::new(),
        ),
        RdfTerm::Literal(literal) => (
            literal.lexical.clone(),
            "literal",
            String::new(),
            literal.as_written.clone(),
        ),
    }
}

/// One fixture per class expression an OWL case's ontology states as a blank node, rendered at the canvas's depth
/// cap and uncapped, named `<case>-<n>-canvas.json` and `<case>-<n>-full.json`.
pub fn description_logic(cases: &Path) -> Result<Vec<PathBuf>, String> {
    fs::create_dir_all(cases).map_err(|e| format!("{}: {e}", cases.display()))?;
    let mut written = Vec::new();
    for Opened { case, document, .. } in opened_of("w3c/owl")? {
        let model = &document.model;
        let index = OwlModelIndex::build(model);
        let mut roots: Vec<RdfTerm> = Vec::new();
        for triple in &model.triples {
            let axiom =
                [owl::SUB_CLASS_OF, owl::EQUIVALENT_CLASS, owl::DISJOINT_WITH].contains(&triple.predicate.iri.as_str());
            if let RdfTerm::Blank(blank) = &triple.object
                && axiom
                && !roots
                    .iter()
                    .any(|r| r.as_blank().is_some_and(|b| b.ordinal == blank.ordinal))
            {
                roots.push(triple.object.clone());
            }
        }
        let short = case.name.rsplit('/').next().unwrap_or(&case.name);
        let source = if case.minted { "parity/cases" } else { "parity/scripts" };
        for (n, root) in roots.iter().enumerate() {
            let value = expression_of(root, model, &index);
            for (cap, suffix) in [(Some(expression::CANVAS_DEPTH), "canvas"), (None, "full")] {
                let text = expression::render_with(root, &index, model, cap);
                let fixture = json!({
                    "$schema": "https://etalii.net/adp/atd/schema/0.1/atd.schema.json#/$defs/CapabilityFixture",
                    "doc": format!(
                        "Recorded by `atd record --capability render.descriptionLogic` from the RDF module's rendering of a class expression of the ontology that {source}/{}.json of etalii.adp.ide.desktop opens; never edited by hand.",
                        case.name
                    ),
                    "input": {"value": value, "format": "rdf"},
                    "args": {"maxDepth": cap.unwrap_or(0)},
                    "output": {"runs": [{"text": text.text, "style": {"elided": text.elided, "cyclic": text.cyclic}}]},
                });
                let file = cases.join(format!("{short}-{}-{suffix}.json", n + 1));
                write_json(&file, &fixture)?;
                written.push(file);
            }
        }
    }
    Ok(written)
}

/// A W3C case opened as its reading opens it: the document, its registration and the positions that stores.
struct Opened {
    case: adp_parity::Case,
    document: RdfDocument,
    registration: Option<PathBuf>,
    stored: Positions,
}

/// Every case of `origin`, scripted and minted, opened.
fn opened_of(origin: &str) -> Result<Vec<Opened>, String> {
    let of_origin = |n: &String| n.rsplit_once('/').is_some_and(|(tool, _)| tool == origin);
    let scripts = adp_parity::script_names()
        .into_iter()
        .filter(of_origin)
        .map(|n| adp_parity::Case::load(&n));
    let minted = adp_parity::case_names()
        .into_iter()
        .filter(of_origin)
        .map(|n| adp_parity::Case::load_minted(&n));
    let mut opened = Vec::new();
    for case in scripts.chain(minted) {
        let case = case?;
        let fixtures = adp_parity::repository_root().join(if case.minted { "parity/cases" } else { "parity/fixtures" });
        let body_path = fixtures.join(&case.script.fixture);
        let bytes = fs::read(&body_path).map_err(|e| format!("{}: {e}", body_path.display()))?;
        let document = RdfDocument::read(&body_path, &String::from_utf8_lossy(&bytes));
        let registration = case.script.registration.as_ref().map(|r| fixtures.join(r));
        let stored = match &registration {
            Some(path) => {
                registration_layout::read(&fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
            }
            None => Positions::new(),
        };
        opened.push(Opened {
            case,
            document,
            registration,
            stored,
        });
    }
    Ok(opened)
}

/// A class expression as the capability reads it: the root, the triples of every blank node it reaches, and the
/// display of every IRI those triples name, which is the term's `rdfs:label` or its compressed form.
fn expression_of(root: &RdfTerm, model: &RdfModel, index: &OwlModelIndex) -> Json {
    let mut triples = Vec::new();
    let mut names = BTreeMap::new();
    let mut seen = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(term) = pending.pop() {
        let Some(blank) = term.as_blank() else { continue };
        if seen.contains(&blank.ordinal) {
            continue;
        }
        seen.push(blank.ordinal);
        let about = model
            .triples
            .iter()
            .filter(|t| t.subject.as_blank().is_some_and(|b| b.ordinal == blank.ordinal));
        for triple in about {
            if let Some(iri) = triple.object.as_iri() {
                names.insert(iri.to_owned(), expression::display(index, model, iri));
            }
            triples.push(json!({
                "s": term_of(&triple.subject),
                "p": triple.predicate.iri,
                "o": term_of(&triple.object),
                "oKind": kind_of(&triple.object),
            }));
            pending.push(triple.object.clone());
        }
    }
    json!({"root": term_of(root), "triples": triples, "names": names})
}

/// A term as the capability's input writes it: an IRI in full, a blank node as `_:b<n>`, a literal's lexical form.
fn term_of(term: &RdfTerm) -> String {
    match term {
        RdfTerm::Iri(iri) => iri.iri.clone(),
        RdfTerm::Blank(blank) => format!("_:b{}", blank.ordinal),
        RdfTerm::Literal(literal) => literal.lexical.clone(),
    }
}

/// Which of the three kinds a term is.
fn kind_of(term: &RdfTerm) -> &'static str {
    match term {
        RdfTerm::Iri(_) => "iri",
        RdfTerm::Blank(_) => "blank",
        RdfTerm::Literal(_) => "literal",
    }
}

/// The arguments the W3C descriptions pass each layout, which are the module's own constants.
fn args_of(name: &str) -> Json {
    match name {
        "layout.bandedGrid" => json!({
            "columns": 4, "columnWidth": CELL_WIDTH, "rowHeight": CELL_HEIGHT, "bandGap": 80,
            "band": "size(self.types) > 0 ? self.types[0] : ''",
        }),
        "layout.grid" => json!({"columns": 3, "columnGap": 0, "rowGap": 48}),
        "layout.depthColumns" => json!({
            "columnWidth": 530, "laneOffset": 250, "slotHeight": 130, "satelliteHeight": 110, "satelliteOffset": 10,
            "headerGap": 80, "bandGap": 140, "bandColumns": 5, "bandColumnGap": 60, "bandRowGap": 50,
            "role": "self.role", "supers": "self.supers", "owner": "self.owner",
            "pinnable": "self.id.startsWith('res:') || self.id.startsWith('ind:')",
        }),
        _ => json!({
            "columnWidth": 240, "rowHeight": 110, "regionGap": 90, "regionHeader": 60, "layerGap": 50,
            "minimumColumns": 8, "role": "self.role", "key": "self.key", "regions": "self.regions",
        }),
    }
}

/// A node of a layout's input, pinned where the registration stores its position.
fn node(id: &str, width: f64, height: f64, attributes: Json, stored: &Positions, pinnable: bool) -> Json {
    let mut node = json!({"id": id, "width": width, "height": height, "attributes": attributes});
    if pinnable && let Some((x, y)) = stored.get(id) {
        node["pinned"] = json!({"x": x, "y": y});
    }
    node
}

/// The data graph: every card in projection order, with the types it is banded by.
fn banded_grid(document: &RdfDocument, stored: &Positions) -> (Json, Positions) {
    let projection = RdfReading.project(&document.model);
    let nodes: Vec<Json> = projection
        .nodes
        .iter()
        .map(|n| node(&n.id, CELL_WIDTH, CELL_HEIGHT, json!({"types": n.types}), stored, true))
        .collect();
    let positions = RdfReading.positions(&projection, stored);
    (json!({"nodes": nodes, "edges": []}), positions)
}

/// The shapes: every card in discovery order, as tall as its targets and rows.
fn grid(document: &RdfDocument, stored: &Positions) -> (Json, Positions) {
    let projection = ShaclReading.project(&document.model);
    let nodes: Vec<Json> = projection
        .cards
        .iter()
        .map(|c| node(&c.id, shacl::CARD_WIDTH, card_height(c), json!({}), stored, true))
        .collect();
    let positions = ShaclReading.positions(&projection, stored);
    (json!({"nodes": nodes, "edges": []}), positions)
}

/// The ontology: every node with its size, its role, the node that owns it and its asserted superclasses, and every
/// edge. Every stored position is a pin; the description's `pinnable` keeps those of the IRI-derived nodes.
fn depth_columns(document: &RdfDocument, stored: &Positions) -> (Json, Positions) {
    let graph = OwlReading.project(&document.model);
    let nodes: Vec<Json> = graph
        .nodes
        .iter()
        .map(|n| {
            let role = match n.kind {
                OwlNodeKind::OntologyHeader => "header",
                OwlNodeKind::Individual => "individual",
                OwlNodeKind::Class | OwlNodeKind::Thing if n.id.starts_with("res:") => "class",
                _ => "satellite",
            };
            let supers: Vec<&str> = graph
                .edges
                .iter()
                .filter(|e| e.kind == OwlEdgeKind::Subclass && e.from_id == n.id)
                .map(|e| e.to_id.as_str())
                .collect();
            let (width, height) = size_of(n);
            let attributes = json!({"role": role, "owner": n.owner_id, "supers": supers});
            node(&n.id, width, height, attributes, stored, true)
        })
        .collect();
    let edges: Vec<Json> = graph
        .edges
        .iter()
        .map(|e| json!({"id": e.id, "source": e.from_id, "target": e.to_id}))
        .collect();
    let positions = OwlReading.positions(&graph, stored);
    (json!({"nodes": nodes, "edges": edges}), positions)
}

/// The schemes as regions keyed by IRI, the concepts as their members, the collections as the trailing grid, and the
/// hierarchy from broader to narrower.
fn layered_regions(document: &RdfDocument, registration: Option<&Path>, stored: &Positions) -> (Json, Positions) {
    let reading = SkosReading::for_registration(registration);
    let projection = reading.project(&document.model);
    let mut nodes = Vec::new();
    for scheme in &projection.schemes {
        let attributes = json!({"role": "region", "key": scheme.iri});
        nodes.push(node(&scheme.id, 240.0, 110.0, attributes, stored, true));
    }
    for concept in &projection.concepts {
        let attributes = json!({"role": "member", "regions": concept.scheme_iris});
        nodes.push(node(&concept.id, 240.0, 110.0, attributes, stored, true));
    }
    for collection in &projection.collections {
        let attributes = json!({"role": "trailing", "key": collection.iri});
        nodes.push(node(&collection.id, 240.0, 110.0, attributes, stored, true));
    }
    let edges: Vec<Json> = projection
        .edges
        .iter()
        .filter(|e| e.kind == SkosEdgeKind::Hierarchy)
        .map(|e| json!({"id": e.id, "source": e.from_id, "target": e.to_id}))
        .collect();
    let positions = reading.positions(&projection, stored);
    (json!({"nodes": nodes, "edges": edges}), positions)
}

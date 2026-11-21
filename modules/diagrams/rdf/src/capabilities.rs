//! The capabilities the RDF module contributes to the capability registry (etalii.adp spec 035, T125, research R14):
//! `reader.turtle`, the module's parser and family findings as a persistence plugin, and `render.descriptionLogic`,
//! its rendering of a class expression. Their code stays here, where the readings call it; the host that assembles
//! the modules adds them to [`Capabilities`] with [`contribute`].

use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

use adp_atd_runtime::Capabilities;
use adp_atd_runtime::capabilities::RenderCapability;
use adp_cel::{Map, Value};
use adp_diagram::problems::{DiagramProblemLocation, DiagramProblemSeverity};
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use serde_json::{Value as Json, json};

use crate::lines::LineRange;
use crate::model::{BlankTerm, IriTerm, LiteralTerm, RdfModel, RdfTerm, RdfTriple, SourceSpan};
use crate::owl::expression;
use crate::owl::index::OwlModelIndex;
use crate::skos::registration_language;
use crate::store::RdfDocument;
use crate::validator;

/// These capabilities with the RDF module's: `reader.turtle` and `render.descriptionLogic`.
pub fn contribute(capabilities: Capabilities) -> Capabilities {
    capabilities
        .with_reader("reader.turtle", Rc::new(TurtleReader))
        .with_renderer("render.descriptionLogic", Rc::new(DescriptionLogic))
}

/// `reader.turtle`: the stored model every reading of a Turtle or N-Triples body shares, as the elements `document`,
/// `Base`, `Prefix` and `Triple`, and the findings of the RDF family. Its edits are planned by the descriptions' grammar
/// family binding (spec 035, T149), so it plans none of its own.
pub struct TurtleReader;

impl PersistencePlugin for TurtleReader {
    fn id(&self) -> &str {
        "reader.turtle"
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let body = request
            .files
            .iter()
            .find(|f| f.relative_path.is_empty())
            .map(|f| String::from_utf8_lossy(&f.bytes).into_owned())
            .unwrap_or_default();
        // The registration, handed beside the body by its name.
        let registration = request
            .files
            .iter()
            .find(|f| f.relative_path.ends_with(".adp"))
            .map(|f| String::from_utf8_lossy(&f.bytes).into_owned());
        let body_name = registration
            .as_deref()
            .and_then(|text| {
                text.lines()
                    .find_map(|line| line.trim().strip_prefix("body:").map(|b| b.trim().to_owned()))
            })
            .unwrap_or_else(|| "body.ttl".to_owned());
        let document = RdfDocument::read(Path::new(&body_name), &body);
        let mut elements = vec![document_of(registration.as_deref())];
        elements.extend(stored_of(&document.model));
        let findings = validator::validate(&document.text)
            .into_iter()
            .map(|problem| Finding {
                code: problem.rule_id,
                severity: match problem.severity {
                    DiagramProblemSeverity::Error => FindingSeverity::Error,
                    DiagramProblemSeverity::Warning => FindingSeverity::Warning,
                    DiagramProblemSeverity::Info => FindingSeverity::Info,
                },
                message: problem.message,
                location: match problem.location {
                    Some(DiagramProblemLocation::Line(line)) => Some(SourceLocation {
                        file: String::new(),
                        line: line as usize,
                        column: 0,
                        length: 0,
                    }),
                    _ => None,
                },
                detail: None,
            })
            .collect();
        PluginReadResult::new(elements, findings, false)
    }

    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused("A Turtle body is edited through its grammar family binding.".into())
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

/// A stored element of the reading: at the top, its attributes as given.
fn element(id: String, type_name: &str, attributes: Vec<(&str, Value)>) -> FblElement {
    let mut map = Map::default();
    for (name, value) in attributes {
        map.insert(name, value);
    }
    FblElement {
        id,
        id_is_stored: false,
        type_name: type_name.to_owned(),
        rule: type_name.to_owned(),
        is_relation: false,
        attributes: map,
        parent_id: None,
        parent_slot: None,
        source: None,
        target: None,
        referenced: Vec::new(),
        own_span: adp_fbl::text::Span::new(0, 0),
        line: 0,
        file: None,
    }
}

fn text(value: impl Into<String>) -> Value {
    Value::String(value.into())
}

fn number(value: usize) -> Value {
    Value::Int(value as i64)
}

/// The element `document`, which carries what the registration's headers say: `language`, the display language, when
/// one is honoured, and `languageLine`, the line of a `language:` header that is ignored for standing above a core
/// header, 0 when none is.
fn document_of(registration: Option<&str>) -> FblElement {
    let (language, misplaced) = registration.map_or((None, 0), registration_language::read_text);
    let mut attributes = vec![("languageLine", number(misplaced))];
    if let Some(language) = language {
        attributes.push(("language", text(language)));
    }
    element("document".to_owned(), "document", attributes)
}

/// The base, the prefix declarations and the triples of `model` in document order, as stored elements with the ids
/// the W3C descriptions give them: `base`, `prefix:<name>` and `triple:<s>|<p>|<o>`, `|<n>` added for the n-th repeat.
fn stored_of(model: &RdfModel) -> Vec<FblElement> {
    let mut elements = Vec::new();
    if let Some(iri) = &model.base_iri {
        elements.push(element("base".to_owned(), "Base", vec![("iri", text(iri))]));
    }
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for declaration in &model.prefixes {
        let id = repeated(&mut seen, format!("prefix:{}", declaration.prefix));
        elements.push(element(
            id,
            "Prefix",
            vec![
                ("name", text(&declaration.prefix)),
                ("iri", text(&declaration.iri)),
                ("line", number(declaration.line + 1)),
            ],
        ));
    }
    for triple in &model.triples {
        let (s, s_kind, s_label, s_written) = stored_term(&triple.subject);
        let (o, o_kind, o_label, _) = stored_term(&triple.object);
        let literal = triple.object.as_literal();
        let id = repeated(&mut seen, format!("triple:{s}|{}|{o}", triple.predicate.iri));
        elements.push(element(
            id,
            "Triple",
            vec![
                ("s", text(s)),
                ("sKind", text(s_kind)),
                ("sLabel", text(s_label)),
                ("sWritten", text(s_written)),
                ("p", text(&triple.predicate.iri)),
                ("o", text(o)),
                ("oKind", text(o_kind)),
                ("oLabel", text(o_label)),
                (
                    "language",
                    text(literal.and_then(|l| l.language.clone()).unwrap_or_default()),
                ),
                (
                    "datatype",
                    text(literal.and_then(|l| l.datatype.clone()).unwrap_or_default()),
                ),
                ("line", number(triple.span.start_line + 1)),
            ],
        ));
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

/// `render.descriptionLogic`: a class expression, given as its root, the triples of every blank node it reaches and the
/// display of every IRI they name, rendered by [`expression::render_with`] as one run.
pub struct DescriptionLogic;

impl RenderCapability for DescriptionLogic {
    fn render(&self, input: &Json, args: &Json) -> Result<Json, String> {
        let value = &input["value"];
        let term = |written: &str, kind: &str| -> RdfTerm {
            match kind {
                "blank" => RdfTerm::Blank(blank_of(written)),
                "literal" => RdfTerm::Literal(LiteralTerm::new(written, None, None)),
                _ => RdfTerm::Iri(IriTerm::new(written, written)),
            }
        };
        let triples = value["triples"]
            .as_array()
            .ok_or("The value has no `triples`.")?
            .iter()
            .map(|t| {
                let subject = t["s"].as_str().unwrap_or_default();
                RdfTriple {
                    subject: term(subject, if subject.starts_with("_:") { "blank" } else { "iri" }),
                    predicate: IriTerm::new(t["p"].as_str().unwrap_or_default(), ""),
                    object: term(
                        t["o"].as_str().unwrap_or_default(),
                        t["oKind"].as_str().unwrap_or("iri"),
                    ),
                    span: SourceSpan {
                        start_line: 0,
                        end_line: 0,
                        fragment_start: None,
                        fragment_end: None,
                    },
                    statement: LineRange::default(),
                    span_start: 0,
                    span_end: 0,
                    object_start: 0,
                    object_end: 0,
                    terminator_offset: 0,
                }
            })
            .collect();
        let model = RdfModel {
            triples,
            ..RdfModel::default()
        };
        let mut index = OwlModelIndex::build(&model);
        for (iri, name) in value["names"].as_object().into_iter().flatten() {
            index
                .labels
                .insert(iri.clone(), name.as_str().unwrap_or_default().to_owned());
        }
        let root = value["root"].as_str().ok_or("The value has no `root`.")?;
        let root = term(root, if root.starts_with("_:") { "blank" } else { "iri" });
        let depth = args["maxDepth"].as_u64().filter(|d| *d > 0).map(|d| d as usize);
        let rendered = expression::render_with(&root, &index, &model, depth);
        Ok(json!({"runs": [{"text": rendered.text, "style": {"elided": rendered.elided, "cyclic": rendered.cyclic}}]}))
    }
}

/// The blank node `_:b<n>`, by its ordinal.
fn blank_of(written: &str) -> BlankTerm {
    BlankTerm {
        label: None,
        ordinal: written.trim_start_matches("_:b").parse().unwrap_or(usize::MAX),
    }
}

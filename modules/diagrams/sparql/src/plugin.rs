//! The persistence plugin of the SPARQL query's binding (FBL §11; `reader.sparql`, etalii.adp spec 035, T132): it reads
//! a query with the module's tokenizer and parser and delivers its stored records with their spans, and it plans
//! nothing.
//!
//! Ported from the standalone host's `SparqlQueryPlugin.cs` and `SparqlRecords.cs` at `25cd26e`. The records and their
//! attributes are the definition's stored types: `Query` is mapped onto the diagram's attributes and `Constraint` onto
//! `Annotation` by its `persistence.typeMap`; `Prefix`, `Scope`, `Pattern` and `SubQuery` are stored as they are named.
//! A scope's record is a child of the scope it is written in, and so is a subquery's; the where clause itself is the
//! open canvas and has no record. Three things are decided here because DISL cannot decide them, each named in the
//! definition's companion: whether a prefix is used, asked of the source text (finding A7); a term's key, display, full
//! form and annotation, a literal's key percent-encoded (gap 6); and a pattern's `repeat`, how many patterns before it
//! in the order the tool walks them have the same subject, predicate as written and object.

use std::collections::HashMap;

use adp_cel::{Map, Value};
use adp_fbl::model::{FblElement, Finding, FindingSeverity, SourceLocation, codes};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use adp_fbl::text::Span;

use crate::model::{GroupScope, SparqlQueryForm, SparqlQueryModel, SparqlSpan, SparqlTerm, TriplePattern};
use crate::parser;

/// The plugin's id, as the binding's `reader.plugin` names it.
pub const PLUGIN_ID: &str = "net.etalii.adp.w3c.sparqlQuery";

/// The binding's `readOnly` sentence, the plugin's answer to every plan.
pub const READ_ONLY: &str = "A query diagram offers no edits: it reads the .rq file, which is edited in a text editor.";

/// The plugin: reading never fails on content, and every plan is refused.
pub struct SparqlQueryPlugin;

impl PersistencePlugin for SparqlQueryPlugin {
    fn id(&self) -> &str {
        PLUGIN_ID
    }

    /// A text the tokenizer or the parser cannot read is no records, unreadable, and one `std.unparseable` carrying the
    /// parser's sentence at the line and column it stopped at.
    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let text = request
            .files
            .first()
            .map(|file| String::from_utf8_lossy(&file.bytes).into_owned())
            .unwrap_or_default();
        match parser::parse(&text) {
            Ok(model) => PluginReadResult::new(records(&model, &text), Vec::new(), false),
            Err(error) => PluginReadResult::new(
                Vec::new(),
                vec![Finding {
                    code: codes::UNPARSEABLE.into(),
                    severity: FindingSeverity::Error,
                    message: error.message,
                    location: Some(SourceLocation {
                        file: String::new(),
                        line: error.line.max(1),
                        column: error.column.max(1),
                        length: 0,
                    }),
                    detail: None,
                }],
                true,
            ),
        }
    }

    /// Refused with the binding's own sentence, `Save` included: the second of two independent stops.
    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused(READ_ONLY.into())
    }

    /// Never asked, because the binding has `template.text`; nothing is written.
    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

/// The read id of the one `Query` record.
const QUERY_ID: &str = "query";

/// The where clause's scope path: the open canvas, which has no record of its own.
const WHERE_PATH: &str = "where";

/// The records of `model`, read from `text`, in document order.
pub fn records(model: &SparqlQueryModel, text: &str) -> Vec<FblElement> {
    Reader::new(model, text).read()
}

struct Reader<'a> {
    model: &'a SparqlQueryModel,
    text: &'a str,
    /// The byte offset of every character offset, FBL's spans counting UTF-8 bytes (FBL 6.1).
    bytes: Vec<usize>,
    records: Vec<FblElement>,
    repeats: HashMap<*const TriplePattern, usize>,
    patterns: usize,
    prefixes: usize,
}

impl<'a> Reader<'a> {
    fn new(model: &'a SparqlQueryModel, text: &'a str) -> Self {
        let mut bytes: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
        bytes.push(text.len());
        Reader {
            model,
            text,
            bytes,
            records: Vec::new(),
            repeats: HashMap::new(),
            patterns: 0,
            prefixes: 0,
        }
    }

    fn byte_span(&self, span: SparqlSpan) -> Span {
        let at = |offset: usize| self.bytes.get(offset).copied().unwrap_or(self.text.len());
        Span::new(at(span.start), at(span.end))
    }

    fn read(mut self) -> Vec<FblElement> {
        let repeats_where = self.repeats_where();
        self.count_repeats(repeats_where);

        let attributes = self.query(repeats_where);
        let whole = Span::new(0, self.text.len());
        self.records.push(record(QUERY_ID, "Query", attributes, None, whole, 1));
        for prefix in &self.model.prefixes {
            let id = format!("prefix-{}", self.prefixes);
            self.prefixes += 1;
            let attributes = map(vec![
                ("prefix", text(&prefix.prefix)),
                ("iri", text(&prefix.iri)),
                ("used", Value::Bool(self.is_used(&prefix.prefix))),
            ]);
            let span = self.byte_span(prefix.span);
            self.records
                .push(record(&id, "Prefix", attributes, None, span, prefix.line));
        }
        // The template of CONSTRUCT is written before the where clause; the short form's shares the where clause's
        // text and states no entry of its own.
        if let Some(template) = &self.model.template {
            self.scope(template, None, !repeats_where);
        }
        self.entries(&self.model.where_clause);
        self.records
    }

    /// The diagram's attributes: the query's form, its projection, its targets and its modifiers.
    fn query(&self, repeats_where: bool) -> Map {
        let model = self.model;
        let form = match model.form {
            SparqlQueryForm::Select => "select",
            SparqlQueryForm::Construct => "construct",
            SparqlQueryForm::Ask => "ask",
            SparqlQueryForm::Describe => "describe",
        };
        map(vec![
            ("form", text(form)),
            ("distinct", Value::Bool(model.distinct)),
            ("reduced", Value::Bool(model.reduced)),
            ("projectsAll", Value::Bool(model.projects_all)),
            (
                "projection",
                texts(model.projection.iter().map(|item| item.name.as_str())),
            ),
            (
                "projectionExpressions",
                texts(model.projection.iter().map(|item| item.expression_text.as_str())),
            ),
            (
                "describeTargets",
                texts(model.describe_targets.iter().map(String::as_str)),
            ),
            (
                "describeTerms",
                Value::list(model.describe_terms.iter().map(describe_term).collect()),
            ),
            (
                "datasetClauses",
                texts(model.dataset_clauses.iter().map(String::as_str)),
            ),
            ("modifierRows", texts(model.modifier_rows.iter().map(String::as_str))),
            ("templateRepeatsWhere", Value::Bool(repeats_where)),
            ("baseIri", text(&model.base_iri)),
        ])
    }

    /// A group's record, then its entries.
    fn scope(&mut self, scope: &GroupScope, parent: Option<&str>, own_entries: bool) {
        let attributes = map(vec![
            ("path", text(&scope.path)),
            ("scopeKind", text(scope.kind.name())),
            ("term", text(&scope.label)),
        ]);
        let span = self.byte_span(scope.span);
        self.records
            .push(record(&scope.path, "Scope", attributes, parent, span, scope.span.line));
        if own_entries {
            self.entries(scope);
        }
    }

    /// The entries `scope` states, in the order they are written: its patterns, constraints, subqueries and groups
    /// merged by where each starts, each kind keeping the order the parser gave it. A subquery and a group are
    /// children of the group's record; the where clause has none.
    fn entries(&mut self, scope: &GroupScope) {
        let parent = (scope.path != WHERE_PATH).then_some(scope.path.as_str());
        let mut next = [0usize; 4];
        loop {
            let starts = [
                scope.patterns.get(next[0]).map(|p| p.span.start),
                scope.constraints.get(next[1]).map(|c| c.span.start),
                scope.sub_selects.get(next[2]).map(|s| s.span.start),
                scope.children.get(next[3]).map(|c| c.span.start),
            ];
            // The earliest start; among equal starts, the first kind, as a queue picked by its least head does.
            let Some(kind) = (0..4).filter(|&k| starts[k].is_some()).min_by_key(|&k| (starts[k], k)) else {
                return;
            };
            let index = next[kind];
            next[kind] += 1;
            match kind {
                0 => self.pattern(&scope.patterns[index], &scope.path),
                1 => self.constraint(scope, index),
                2 => self.sub_query(scope, index, parent),
                _ => self.scope(&scope.children[index], parent, true),
            }
        }
    }

    fn pattern(&mut self, pattern: &TriplePattern, scope: &str) {
        let repeat = self.repeats.get(&(pattern as *const _)).copied().unwrap_or(0);
        let mut attributes: Vec<(String, Value)> = vec![
            ("scope".into(), text(scope)),
            ("predicate".into(), text(&pattern.predicate.as_written())),
            (
                "isPath".into(),
                Value::Bool(matches!(pattern.predicate, SparqlTerm::Path { .. })),
            ),
            (
                "predicateVariable".into(),
                text(match &pattern.predicate {
                    SparqlTerm::Variable { name, .. } => name,
                    _ => "",
                }),
            ),
            ("repeat".into(), Value::Int(repeat as i64)),
        ];
        end(&mut attributes, "s", &pattern.subject);
        end(&mut attributes, "o", &pattern.object);
        let id = format!("pattern-{}", self.patterns);
        self.patterns += 1;
        let attributes_map = map(attributes);
        let span = self.byte_span(pattern.span);
        self.records
            .push(record(&id, "Pattern", attributes_map, None, span, pattern.span.line));
    }

    fn constraint(&mut self, scope: &GroupScope, index: usize) {
        let constraint = &scope.constraints[index];
        let ordinal = scope.constraints[..index]
            .iter()
            .filter(|earlier| earlier.kind == constraint.kind)
            .count();
        let attributes = map(vec![
            ("constraintKind", text(constraint.kind.name())),
            ("text", text(&constraint.text)),
            ("scope", text(&scope.path)),
            ("ordinal", Value::Int(ordinal as i64)),
            ("definedVariable", text(&constraint.defined_variable)),
            (
                "referencedVariables",
                texts(constraint.referenced_variables.iter().map(String::as_str)),
            ),
        ]);
        let id = format!("{}/constraint.{index}", scope.path);
        let span = self.byte_span(constraint.span);
        self.records
            .push(record(&id, "Constraint", attributes, None, span, constraint.span.line));
    }

    fn sub_query(&mut self, scope: &GroupScope, index: usize, parent: Option<&str>) {
        let sub_select = &scope.sub_selects[index];
        let attributes = map(vec![
            ("scope", text(&scope.path)),
            ("ordinal", Value::Int(sub_select.ordinal as i64)),
            ("display", text(&sub_select.projection_text)),
            ("text", text(&sub_select.text)),
            (
                "projectedNames",
                texts(sub_select.projected_names.iter().map(String::as_str)),
            ),
        ]);
        let id = format!("{}/subquery.{}", scope.path, sub_select.ordinal);
        let span = self.byte_span(sub_select.span);
        self.records
            .push(record(&id, "SubQuery", attributes, parent, span, sub_select.span.line));
    }

    /// Whether the template of `CONSTRUCT WHERE` is the where clause's own patterns, stated once.
    fn repeats_where(&self) -> bool {
        self.model.template.as_ref().is_some_and(|template| {
            !template.patterns.is_empty() && template.patterns == self.model.where_clause.patterns
        })
    }

    /// Each pattern's `repeat`: how many patterns before it, in the order the tool walks them - the where clause and
    /// its groups depth first, then the template - have its subject, its predicate as written and its object.
    fn count_repeats(&mut self, repeats_where: bool) {
        let mut seen = HashMap::new();
        let mut repeats = HashMap::new();
        walk_repeats(&self.model.where_clause, &mut seen, &mut repeats);
        if let Some(template) = &self.model.template
            && !repeats_where
        {
            walk_repeats(template, &mut seen, &mut repeats);
        }
        self.repeats = repeats;
    }

    /// Whether the query uses `prefix` anywhere outside its own declarations, asked of the source text: an occurrence
    /// on a line that starts with `PREFIX` is a declaration, not a use.
    fn is_used(&self, prefix: &str) -> bool {
        let needle = format!("{prefix}:");
        let mut from = 0;
        while let Some(found) = self.text[from..].find(&needle) {
            let at = from + found;
            from = at + needle.len();
            let line_start = self.text[..at].rfind('\n').map_or(0, |n| n + 1);
            let before = self.text[line_start..at].trim_start();
            if before.len() >= 6 && before[..6].eq_ignore_ascii_case("PREFIX") {
                continue;
            }
            return true;
        }
        false
    }
}

/// Counts the repeats of `scope`'s patterns, then of its groups', depth first.
fn walk_repeats(
    scope: &GroupScope,
    seen: &mut HashMap<(String, String, String), usize>,
    repeats: &mut HashMap<*const TriplePattern, usize>,
) {
    for pattern in &scope.patterns {
        let key = (
            key(&pattern.subject),
            pattern.predicate.as_written(),
            key(&pattern.object),
        );
        let count = seen.get(&key).copied().unwrap_or(0);
        repeats.insert(pattern as *const _, count);
        seen.insert(key, count + 1);
    }
    for child in &scope.children {
        walk_repeats(child, seen, repeats);
    }
}

/// One end of a pattern, as the attributes `{end}Kind`, `{end}Key`, `{end}Display`, `{end}Full` and
/// `{end}Annotation`.
fn end(attributes: &mut Vec<(String, Value)>, end: &str, term: &SparqlTerm) {
    attributes.push((format!("{end}Kind"), text(kind(term))));
    attributes.push((format!("{end}Key"), text(&key(term))));
    attributes.push((format!("{end}Display"), text(&display(term))));
    attributes.push((format!("{end}Full"), text(&full(term))));
    attributes.push((format!("{end}Annotation"), text(&annotation(term))));
}

/// A `DESCRIBE` target, as the definition's `DescribeTerm`.
fn describe_term(term: &SparqlTerm) -> Value {
    Value::map(map(vec![
        ("kind", text(kind(term))),
        ("key", text(&key(term))),
        ("display", text(&display(term))),
        ("full", text(&full(term))),
        ("annotation", text(&annotation(term))),
    ]))
}

fn kind(term: &SparqlTerm) -> &'static str {
    match term {
        SparqlTerm::Variable { .. } => "var",
        SparqlTerm::Anonymous { .. } => "anon",
        SparqlTerm::Iri { .. } => "iri",
        _ => "lit",
    }
}

/// The key a term is told apart by, which is also its node's id: a literal's lexical form and datatype
/// percent-encoded (gap 6).
fn key(term: &SparqlTerm) -> String {
    match term {
        SparqlTerm::Variable { name, .. } => format!("var:{name}"),
        SparqlTerm::Anonymous { ordinal, .. } => format!("anon:{ordinal}"),
        SparqlTerm::Iri { iri, .. } => format!("iri:{iri}"),
        SparqlTerm::Literal {
            lexical,
            datatype,
            language,
            ..
        } => {
            let mut key = format!("lit:{}", escape_data(lexical));
            if !language.is_empty() {
                key.push_str(&format!("@{language}"));
            }
            if !datatype.is_empty() {
                key.push_str(&format!("^{}", escape_data(&short_datatype(datatype))));
            }
            key
        }
        SparqlTerm::Path { .. } => String::new(),
    }
}

/// What a term's node shows: a prefixed name or an IRI as written, a literal's lexical form, a blank node as written.
fn display(term: &SparqlTerm) -> String {
    match term {
        SparqlTerm::Literal { lexical, .. } => lexical.clone(),
        other => other.as_written(),
    }
}

/// An IRI's full form, or a literal as written.
fn full(term: &SparqlTerm) -> String {
    match term {
        SparqlTerm::Iri { iri, .. } => iri.clone(),
        SparqlTerm::Literal { written, .. } => written.clone(),
        _ => String::new(),
    }
}

/// A literal's language tag or the local name of its datatype.
fn annotation(term: &SparqlTerm) -> String {
    match term {
        SparqlTerm::Literal { language, .. } if !language.is_empty() => format!("@{language}"),
        SparqlTerm::Literal { datatype, .. } if !datatype.is_empty() => short_datatype(datatype),
        _ => String::new(),
    }
}

fn short_datatype(iri: &str) -> String {
    if let Some(hash) = iri.rfind('#')
        && hash < iri.len() - 1
    {
        return iri[hash + 1..].to_owned();
    }
    match iri.rfind('/') {
        Some(slash) if slash < iri.len() - 1 => iri[slash + 1..].to_owned(),
        _ => iri.to_owned(),
    }
}

/// .NET's `Uri.EscapeDataString`: every byte of the UTF-8 encoding outside RFC 3986's unreserved characters as `%XX`.
fn escape_data(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            escaped.push(byte as char);
        } else {
            escaped.push_str(&format!("%{byte:02X}"));
        }
    }
    escaped
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn texts<'s>(values: impl Iterator<Item = &'s str>) -> Value {
    Value::list(values.map(text).collect())
}

fn map<N: Into<String>>(entries: Vec<(N, Value)>) -> Map {
    let mut map = Map::default();
    for (name, value) in entries {
        map.insert(name, value);
    }
    map
}

fn record(id: &str, type_name: &str, attributes: Map, parent: Option<&str>, span: Span, line: usize) -> FblElement {
    FblElement {
        id: id.to_owned(),
        id_is_stored: false,
        type_name: type_name.to_owned(),
        rule: type_name.to_owned(),
        is_relation: false,
        attributes,
        parent_id: parent.map(str::to_owned),
        parent_slot: parent.map(|_| "children".to_owned()),
        source: None,
        target: None,
        referenced: Vec::new(),
        own_span: span,
        line,
        file: None,
    }
}

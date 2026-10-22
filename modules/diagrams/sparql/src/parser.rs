//! A hand-rolled recursive-descent parser over the SPARQL 1.1 Query grammar (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlParser.cs` and `SparqlParseException.cs`, with its three tiers. **Parsed structurally**: the prologue, the
//! four query forms and their clause shapes, the group-pattern tree, triple patterns with their lists and blank-node
//! and collection syntax, solution modifiers and `VALUES`. **Recognized but kept as written**: every expression and
//! every property path, sliced from the source with the variables inside found, never built into a tree.
//! **Refused by name**: a SPARQL Update document, because an update is a write instruction rather than a question.

use std::collections::HashMap;

use crate::model::{
    GroupScope, GroupScopeKind, SparqlConstraint, SparqlConstraintKind, SparqlPrefixDeclaration, SparqlProjectionItem,
    SparqlQueryForm, SparqlQueryModel, SparqlSubSelect, SparqlTerm, SparqlVariableUsage, TriplePattern,
};
use crate::terminals;
use crate::tokenizer::{Token, TokenKind, Tokenizer};

/// What the keyword `a` means in verb position.
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// A collection cell's head.
pub const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
/// A collection cell's tail.
pub const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
/// The empty collection.
pub const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
/// The datatype of `true` and `false`.
pub const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

const UPDATE_VERBS: [&str; 10] = [
    "INSERT", "DELETE", "LOAD", "CLEAR", "CREATE", "DROP", "COPY", "MOVE", "ADD", "WITH",
];

/// Why a document is not a SPARQL 1.1 query, and the 1-based line the parser stopped at (`SparqlParseException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
}

/// What `text` asks, or why it could not be read.
pub fn parse(text: &str) -> Result<SparqlQueryModel, ParseError> {
    let characters: Vec<char> = text.chars().collect();
    let mut tokenizer = Tokenizer::new(&characters);
    let tokens = tokenizer.tokenize()?;
    Parser {
        text: &characters,
        tokenizer: &tokenizer,
        tokens,
        index: 0,
        prefixes: Vec::new(),
        prefix_map: HashMap::new(),
        labeled_anonymous: HashMap::new(),
        base_iri: String::new(),
        anonymous_ordinal: 0,
        seen_counter: 0,
    }
    .parse_query()
}

struct Parser<'a> {
    text: &'a [char],
    tokenizer: &'a Tokenizer<'a>,
    tokens: Vec<Token>,
    index: usize,
    prefixes: Vec<SparqlPrefixDeclaration>,
    prefix_map: HashMap<String, String>,
    labeled_anonymous: HashMap<String, SparqlTerm>,
    base_iri: String,
    anonymous_ordinal: usize,
    seen_counter: usize,
}

type Parsed<T> = Result<T, ParseError>;

struct UsageBuilder {
    name: String,
    pattern_occurrences: usize,
    projected: bool,
    defining_expression: String,
    scope_paths: Vec<String>,
    first_seen: usize,
}

fn without_sigil(value: &str) -> String {
    value.chars().skip(1).collect()
}

fn is_open(token: &Token) -> bool {
    token.kind == TokenKind::Punct && matches!(token.value.as_str(), "(" | "{" | "[")
}

fn is_close(token: &Token) -> bool {
    token.kind == TokenKind::Punct && matches!(token.value.as_str(), ")" | "}" | "]")
}

fn unbracket(bracketed: &str) -> String {
    let characters: Vec<char> = bracketed.chars().collect();
    characters[1..characters.len() - 1].iter().collect()
}

fn shown(token: &Token) -> &str {
    if token.kind == TokenKind::EndOfFile {
        "the end of the file"
    } else {
        &token.value
    }
}

impl Parser<'_> {
    fn parse_query(mut self) -> Parsed<SparqlQueryModel> {
        self.parse_prologue()?;
        let first = self.current().clone();
        if first.kind == TokenKind::EndOfFile {
            return Err(self.error("The file holds no query: it ends after the prologue.", &first));
        }
        for verb in UPDATE_VERBS {
            if first.is_keyword(verb) {
                return Err(self.error(
                    &format!(
                        "This is a SPARQL Update document ({verb} ...), and SPARQL Update is out of scope: an update is a write instruction, not a question, and this diagram draws questions. Update documents conventionally use the .ru extension."
                    ),
                    &first,
                ));
            }
        }
        let model = if first.is_keyword("SELECT") {
            self.parse_select_query()?
        } else if first.is_keyword("CONSTRUCT") {
            self.parse_construct_query()?
        } else if first.is_keyword("ASK") {
            self.parse_ask_query()?
        } else if first.is_keyword("DESCRIBE") {
            self.parse_describe_query()?
        } else {
            return Err(self.error(
                &format!(
                    "Expected a query form (SELECT, CONSTRUCT, ASK or DESCRIBE), found '{}'.",
                    first.value
                ),
                &first,
            ));
        };
        let current = self.current().clone();
        if current.kind != TokenKind::EndOfFile {
            return Err(self.error(
                &format!("The query is over, but '{}' follows it.", current.value),
                &current,
            ));
        }
        Ok(model)
    }

    // -- The prologue ------------------------------------------------------------------------------------------

    fn parse_prologue(&mut self) -> Parsed<()> {
        loop {
            if self.current().is_keyword("PREFIX") {
                let line = self.line_of(self.current());
                self.advance();
                let name = self.expect(TokenKind::PrefixedName, "a prefix name ending in ':'")?;
                let colon = name.value.find(':').unwrap_or_default();
                if colon != name.value.len() - 1 {
                    return Err(self.error(
                        &format!(
                            "A PREFIX declaration names the prefix alone, ending in ':' - found '{}'.",
                            name.value
                        ),
                        &name,
                    ));
                }
                let iri = self.expect(TokenKind::Iri, "the prefix's <IRI>")?;
                let prefix = name.value[..colon].to_owned();
                let expanded = unbracket(&iri.value);
                self.prefix_map.insert(prefix.clone(), expanded.clone());
                self.prefixes.push(SparqlPrefixDeclaration {
                    prefix,
                    iri: expanded,
                    line,
                });
            } else if self.current().is_keyword("BASE") {
                self.advance();
                let iri = self.expect(TokenKind::Iri, "the base <IRI>")?;
                self.base_iri = unbracket(&iri.value);
            } else {
                return Ok(());
            }
        }
    }

    // -- The four forms ----------------------------------------------------------------------------------------

    fn parse_select_query(&mut self) -> Parsed<SparqlQueryModel> {
        self.advance();
        let mut distinct = false;
        let mut reduced = false;
        if self.current().is_keyword("DISTINCT") {
            distinct = true;
            self.advance();
        } else if self.current().is_keyword("REDUCED") {
            reduced = true;
            self.advance();
        }
        let (projects_all, projection) = self.parse_projection()?;
        let datasets = self.parse_dataset_clauses()?;
        let mut where_clause = self.parse_where()?;
        let (modifiers, values) = self.parse_solution_modifiers()?;
        if let Some(values) = values {
            where_clause.constraints.push(values);
        }
        Ok(self.finish(SparqlQueryModel {
            form: SparqlQueryForm::Select,
            distinct,
            reduced,
            projects_all,
            projection,
            dataset_clauses: datasets,
            where_clause,
            modifier_rows: modifiers,
            ..SparqlQueryModel::default()
        }))
    }

    fn parse_construct_query(&mut self) -> Parsed<SparqlQueryModel> {
        self.advance();
        let mut template = None;
        if self.current().is_punct("{") {
            template = Some(self.parse_group_graph_pattern(GroupScopeKind::Template, 0, "")?);
        }
        let datasets = self.parse_dataset_clauses()?;
        let mut where_clause = self.parse_where()?;
        let template = template.unwrap_or_else(|| {
            // The short form, CONSTRUCT WHERE { ... }: the template is the pattern itself.
            let mut template = GroupScope::new(GroupScopeKind::Template, 0, "");
            template.patterns.extend(where_clause.patterns.iter().cloned());
            template
        });
        let (modifiers, values) = self.parse_solution_modifiers()?;
        if let Some(values) = values {
            where_clause.constraints.push(values);
        }
        Ok(self.finish(SparqlQueryModel {
            form: SparqlQueryForm::Construct,
            dataset_clauses: datasets,
            template: Some(template),
            where_clause,
            modifier_rows: modifiers,
            ..SparqlQueryModel::default()
        }))
    }

    fn parse_ask_query(&mut self) -> Parsed<SparqlQueryModel> {
        self.advance();
        let datasets = self.parse_dataset_clauses()?;
        let mut where_clause = self.parse_where()?;
        let (modifiers, values) = self.parse_solution_modifiers()?;
        if let Some(values) = values {
            where_clause.constraints.push(values);
        }
        Ok(self.finish(SparqlQueryModel {
            form: SparqlQueryForm::Ask,
            dataset_clauses: datasets,
            where_clause,
            modifier_rows: modifiers,
            ..SparqlQueryModel::default()
        }))
    }

    fn parse_describe_query(&mut self) -> Parsed<SparqlQueryModel> {
        self.advance();
        let mut targets = Vec::new();
        let mut terms = Vec::new();
        let mut projection = Vec::new();
        if self.current().is_punct("*") {
            targets.push("*".to_owned());
            self.advance();
        } else {
            while matches!(
                self.current().kind,
                TokenKind::Variable | TokenKind::Iri | TokenKind::PrefixedName
            ) {
                let token = self.current().clone();
                targets.push(token.value.clone());
                match token.kind {
                    TokenKind::Variable => {
                        let name = without_sigil(&token.value);
                        projection.push(SparqlProjectionItem {
                            name: name.clone(),
                            expression_text: String::new(),
                        });
                        terms.push(SparqlTerm::Variable {
                            name,
                            written: token.value.clone(),
                        });
                    }
                    TokenKind::Iri => terms.push(SparqlTerm::Iri {
                        iri: self.resolve_iri(&token),
                        written: token.value.clone(),
                    }),
                    _ => terms.push(SparqlTerm::Iri {
                        iri: self.expand_prefixed(&token)?,
                        written: token.value.clone(),
                    }),
                }
                self.advance();
            }
            if targets.is_empty() {
                let current = self.current().clone();
                return Err(self.error("DESCRIBE names at least one variable or IRI, or '*'.", &current));
            }
        }
        let datasets = self.parse_dataset_clauses()?;
        let mut where_clause = GroupScope::new(GroupScopeKind::Group, 0, "");
        if self.current().is_keyword("WHERE") || self.current().is_punct("{") {
            where_clause = self.parse_where()?;
        }
        let (modifiers, values) = self.parse_solution_modifiers()?;
        if let Some(values) = values {
            where_clause.constraints.push(values);
        }
        Ok(self.finish(SparqlQueryModel {
            form: SparqlQueryForm::Describe,
            describe_targets: targets,
            describe_terms: terms,
            projection,
            dataset_clauses: datasets,
            where_clause,
            modifier_rows: modifiers,
            ..SparqlQueryModel::default()
        }))
    }

    fn parse_projection(&mut self) -> Parsed<(bool, Vec<SparqlProjectionItem>)> {
        if self.current().is_punct("*") {
            self.advance();
            return Ok((true, Vec::new()));
        }
        let mut items = Vec::new();
        loop {
            if self.current().kind == TokenKind::Variable {
                items.push(SparqlProjectionItem {
                    name: without_sigil(&self.current().value),
                    expression_text: String::new(),
                });
                self.advance();
            } else if self.current().is_punct("(") {
                // (expression AS ?name) - the expression stays as written, tier two.
                let open = self.index;
                let close = self.find_balanced_end(open)?;
                let Some(as_index) = self.find_top_level_keyword(open + 1, close, "AS") else {
                    let current = self.current().clone();
                    return Err(self.error(
                        "A parenthesized projection item is written '(expression AS ?name)'.",
                        &current,
                    ));
                };
                let variable = self.tokens[as_index + 1].clone();
                if variable.kind != TokenKind::Variable {
                    return Err(self.error("AS names the variable the expression defines.", &variable));
                }
                let expression = self.slice_tokens(open + 1, as_index - 1);
                items.push(SparqlProjectionItem {
                    name: without_sigil(&variable.value),
                    expression_text: expression,
                });
                self.index = close + 1;
            } else {
                break;
            }
        }
        if items.is_empty() {
            let current = self.current().clone();
            return Err(self.error("SELECT projects at least one variable, or '*'.", &current));
        }
        Ok((false, items))
    }

    fn parse_dataset_clauses(&mut self) -> Parsed<Vec<String>> {
        let mut rows = Vec::new();
        while self.current().is_keyword("FROM") {
            let start = self.index;
            self.advance();
            if self.current().is_keyword("NAMED") {
                self.advance();
            }
            if !matches!(self.current().kind, TokenKind::Iri | TokenKind::PrefixedName) {
                let current = self.current().clone();
                return Err(self.error("FROM names a graph by IRI.", &current));
            }
            self.advance();
            rows.push(self.slice_tokens(start, self.index - 1));
        }
        Ok(rows)
    }

    fn parse_where(&mut self) -> Parsed<GroupScope> {
        if self.current().is_keyword("WHERE") {
            self.advance();
        }
        self.parse_group_graph_pattern(GroupScopeKind::Group, 0, "")
    }

    // -- The group-pattern tree --------------------------------------------------------------------------------

    fn parse_group_graph_pattern(&mut self, kind: GroupScopeKind, ordinal: usize, label: &str) -> Parsed<GroupScope> {
        self.expect_punct("{")?;
        let mut scope = GroupScope::new(kind, ordinal, label);
        loop {
            let token = self.current().clone();
            if token.kind == TokenKind::EndOfFile {
                return Err(self.error("The group that starts here is never closed with '}'.", &token));
            }
            if token.is_punct("}") {
                self.advance();
                return Ok(scope);
            }
            if token.is_punct(".") {
                self.advance();
                continue;
            }
            if token.is_punct("{") {
                if self.next().is_keyword("SELECT") {
                    self.parse_sub_select(&mut scope)?;
                } else {
                    self.parse_group_or_union(&mut scope)?;
                }
                continue;
            }
            if token.is_keyword("OPTIONAL") {
                self.advance();
                let ordinal = next_ordinal(&scope, GroupScopeKind::Optional);
                let child = self.parse_group_graph_pattern(GroupScopeKind::Optional, ordinal, "")?;
                scope.children.push(child);
                continue;
            }
            if token.is_keyword("MINUS") {
                self.advance();
                let ordinal = next_ordinal(&scope, GroupScopeKind::Minus);
                let child = self.parse_group_graph_pattern(GroupScopeKind::Minus, ordinal, "")?;
                scope.children.push(child);
                continue;
            }
            if token.is_keyword("GRAPH") {
                self.advance();
                let term = self.current().clone();
                if !matches!(
                    term.kind,
                    TokenKind::Variable | TokenKind::Iri | TokenKind::PrefixedName
                ) {
                    return Err(self.error("GRAPH names its graph by variable or IRI.", &term));
                }
                self.advance();
                let ordinal = next_ordinal(&scope, GroupScopeKind::Graph);
                let child = self.parse_group_graph_pattern(GroupScopeKind::Graph, ordinal, &term.value)?;
                scope.children.push(child);
                continue;
            }
            if token.is_keyword("SERVICE") {
                self.advance();
                let silent = self.current().is_keyword("SILENT");
                if silent {
                    self.advance();
                }
                let term = self.current().clone();
                if !matches!(
                    term.kind,
                    TokenKind::Variable | TokenKind::Iri | TokenKind::PrefixedName
                ) {
                    return Err(self.error("SERVICE names its endpoint by variable or IRI.", &term));
                }
                self.advance();
                let label = if silent {
                    format!("SILENT {}", term.value)
                } else {
                    term.value.clone()
                };
                let ordinal = next_ordinal(&scope, GroupScopeKind::Service);
                let child = self.parse_group_graph_pattern(GroupScopeKind::Service, ordinal, &label)?;
                scope.children.push(child);
                continue;
            }
            if token.is_keyword("FILTER") {
                let filter = self.parse_filter()?;
                scope.constraints.push(filter);
                continue;
            }
            if token.is_keyword("BIND") {
                let bind = self.parse_bind()?;
                scope.constraints.push(bind);
                continue;
            }
            if token.is_keyword("VALUES") {
                let values = self.parse_values()?;
                scope.constraints.push(values);
                continue;
            }
            self.parse_triples_block(&mut scope)?;
        }
    }

    fn parse_group_or_union(&mut self, parent: &mut GroupScope) -> Parsed<()> {
        let first_ordinal = next_ordinal(parent, GroupScopeKind::Group);
        let first = self.parse_group_graph_pattern(GroupScopeKind::Group, first_ordinal, "")?;
        if !self.current().is_keyword("UNION") {
            parent.children.push(first);
            return Ok(());
        }
        // The branches join under one union scope; the first group parsed becomes branch 0.
        let mut union = GroupScope::new(GroupScopeKind::Union, next_ordinal(parent, GroupScopeKind::Union), "");
        let mut rebranded = GroupScope::new(GroupScopeKind::Branch, 0, &first.label);
        rebranded.patterns = first.patterns;
        rebranded.children = first.children;
        rebranded.constraints = first.constraints;
        rebranded.sub_selects = first.sub_selects;
        union.children.push(rebranded);
        while self.current().is_keyword("UNION") {
            self.advance();
            let ordinal = union.children.len();
            let branch = self.parse_group_graph_pattern(GroupScopeKind::Branch, ordinal, "")?;
            union.children.push(branch);
        }
        parent.children.push(union);
        Ok(())
    }

    fn parse_sub_select(&mut self, parent: &mut GroupScope) -> Parsed<()> {
        let open = self.index;
        let close = self.find_balanced_end(open)?;
        let select = open + 1;
        let mut projection_end = select + 1;
        let mut projected_names: Vec<String> = Vec::new();
        let mut projects_all = false;
        for i in select + 1..close {
            let token = &self.tokens[i];
            if token.is_keyword("WHERE") || token.is_keyword("FROM") || token.is_punct("{") {
                projection_end = i - 1;
                break;
            }
            if token.kind == TokenKind::Variable {
                let name = without_sigil(&token.value);
                // Inside '(expr AS ?n)' only the aliased name is projected; a plain item is its own name.
                if (i > select + 1 && self.tokens[i - 1].is_keyword("AS")) || self.depth_between(select + 1, i) == 0 {
                    projected_names.push(name);
                }
            } else if token.is_punct("*") && i == select + 1 {
                projects_all = true;
            }
        }
        if projects_all {
            // SELECT * projects the subquery's own in-scope names: every variable its text mentions.
            for i in select + 1..close {
                if self.tokens[i].kind == TokenKind::Variable {
                    let name = without_sigil(&self.tokens[i].value);
                    if !projected_names.contains(&name) {
                        projected_names.push(name);
                    }
                }
            }
        }
        let projection_text = self.slice_tokens(select, select.max(projection_end));
        let text = self.slice_tokens(open, close);
        let ordinal = parent.sub_selects.len();
        parent.sub_selects.push(SparqlSubSelect {
            projection_text,
            text,
            projected_names,
            projects_all,
            ordinal,
        });
        self.index = close + 1;
        Ok(())
    }

    // -- Constraints: tier two, sliced as written --------------------------------------------------------------

    fn parse_filter(&mut self) -> Parsed<SparqlConstraint> {
        let start = self.index;
        self.advance();
        if self.current().is_keyword("EXISTS") {
            self.advance();
            self.skip_balanced("{")?;
        } else if self.current().is_keyword("NOT") {
            self.advance();
            if !self.current().is_keyword("EXISTS") {
                let current = self.current().clone();
                return Err(self.error("After FILTER NOT comes EXISTS.", &current));
            }
            self.advance();
            self.skip_balanced("{")?;
        } else if self.current().is_punct("(") {
            self.skip_balanced("(")?;
        } else if matches!(
            self.current().kind,
            TokenKind::Name | TokenKind::Iri | TokenKind::PrefixedName
        ) {
            // A built-in or named function call: REGEX(...), ex:matches(...).
            self.advance();
            self.skip_balanced("(")?;
        } else {
            let current = self.current().clone();
            return Err(self.error(
                "FILTER takes a bracketed expression, a function call, or EXISTS { ... }.",
                &current,
            ));
        }
        let end = self.index - 1;
        Ok(SparqlConstraint {
            kind: SparqlConstraintKind::Filter,
            text: self.slice_tokens(start, end),
            defined_variable: String::new(),
            referenced_variables: self.variables_between(start, end),
        })
    }

    fn parse_bind(&mut self) -> Parsed<SparqlConstraint> {
        let start = self.index;
        self.advance();
        if !self.current().is_punct("(") {
            let current = self.current().clone();
            return Err(self.error("BIND is written 'BIND(expression AS ?name)'.", &current));
        }
        let open = self.index;
        let close = self.find_balanced_end(open)?;
        let as_index = self
            .find_top_level_keyword(open + 1, close, "AS")
            .filter(|&at| self.tokens[at + 1].kind == TokenKind::Variable);
        let Some(as_index) = as_index else {
            let token = self.tokens[open].clone();
            return Err(self.error("BIND is written 'BIND(expression AS ?name)'.", &token));
        };
        let defined = without_sigil(&self.tokens[as_index + 1].value);
        self.index = close + 1;
        Ok(SparqlConstraint {
            kind: SparqlConstraintKind::Bind,
            text: self.slice_tokens(start, close),
            defined_variable: defined,
            referenced_variables: self.variables_between(open + 1, as_index - 1),
        })
    }

    fn parse_values(&mut self) -> Parsed<SparqlConstraint> {
        let start = self.index;
        self.advance();
        let mut referenced = Vec::new();
        if self.current().kind == TokenKind::Variable {
            referenced.push(without_sigil(&self.current().value));
            self.advance();
        } else if self.current().is_punct("(") {
            let close = self.find_balanced_end(self.index)?;
            referenced.extend(self.variables_between(self.index + 1, close - 1));
            self.index = close + 1;
        } else {
            let current = self.current().clone();
            return Err(self.error("VALUES names its variables before its data block.", &current));
        }
        if !self.current().is_punct("{") {
            let current = self.current().clone();
            return Err(self.error("VALUES takes its rows in a { ... } block.", &current));
        }
        self.skip_balanced("{")?;
        Ok(SparqlConstraint {
            kind: SparqlConstraintKind::Values,
            text: self.slice_tokens(start, self.index - 1),
            defined_variable: String::new(),
            referenced_variables: referenced,
        })
    }

    // -- Triple patterns ---------------------------------------------------------------------------------------

    fn parse_triples_block(&mut self, scope: &mut GroupScope) -> Parsed<()> {
        let subject = self.parse_term_or_blank(scope, true)?;
        self.parse_property_list(scope, &subject)?;
        if self.current().is_punct(".") {
            self.advance();
        }
        Ok(())
    }

    fn parse_property_list(&mut self, scope: &mut GroupScope, subject: &SparqlTerm) -> Parsed<()> {
        loop {
            let predicate = self.parse_verb()?;
            self.parse_object_list(scope, subject, &predicate)?;
            if self.current().is_punct(";") {
                self.advance();
                // A trailing ';' before the block ends is legal and empty.
                let current = self.current();
                if current.is_punct(".") || current.is_punct("}") || current.is_punct("]") || current.is_punct(";") {
                    while self.current().is_punct(";") {
                        self.advance();
                    }
                    if !starts_verb(self.current()) {
                        return Ok(());
                    }
                }
                continue;
            }
            return Ok(());
        }
    }

    fn parse_object_list(
        &mut self,
        scope: &mut GroupScope,
        subject: &SparqlTerm,
        predicate: &SparqlTerm,
    ) -> Parsed<()> {
        loop {
            let object = self.parse_term_or_blank(scope, false)?;
            scope.patterns.push(TriplePattern {
                subject: subject.clone(),
                predicate: predicate.clone(),
                object,
            });
            if self.current().is_punct(",") {
                self.advance();
                continue;
            }
            return Ok(());
        }
    }

    /// The verb: a variable, `a`, a single IRI, or a property path consumed by shape and kept as written.
    fn parse_verb(&mut self) -> Parsed<SparqlTerm> {
        let token = self.current().clone();
        if token.kind == TokenKind::Variable {
            self.advance();
            return Ok(SparqlTerm::Variable {
                name: without_sigil(&token.value),
                written: token.value,
            });
        }
        let start = self.index;
        self.consume_path_expression()?;
        let end = self.index - 1;
        if end == start {
            let only = self.tokens[start].clone();
            if only.is_keyword("a") {
                return Ok(SparqlTerm::iri(RDF_TYPE, "a"));
            }
            if only.kind == TokenKind::Iri {
                return Ok(SparqlTerm::Iri {
                    iri: self.resolve_iri(&only),
                    written: only.value,
                });
            }
            if only.kind == TokenKind::PrefixedName {
                return Ok(SparqlTerm::Iri {
                    iri: self.expand_prefixed(&only)?,
                    written: only.value,
                });
            }
        }
        Ok(SparqlTerm::Path {
            written: self.slice_tokens(start, end),
        })
    }

    fn consume_path_expression(&mut self) -> Parsed<()> {
        self.consume_path_segment()?;
        while self.current().is_punct("|") || self.current().is_punct("/") {
            self.advance();
            self.consume_path_segment()?;
        }
        Ok(())
    }

    fn consume_path_segment(&mut self) -> Parsed<()> {
        if self.current().is_punct("^") {
            self.advance();
        }
        if self.current().is_punct("!") {
            self.advance();
            if self.current().is_punct("(") {
                self.skip_balanced("(")?;
                self.consume_path_postfix();
                return Ok(());
            }
        }
        if self.current().is_punct("(") {
            let open = self.index;
            self.advance();
            self.consume_path_expression()?;
            if !self.current().is_punct(")") {
                let token = self.tokens[open].clone();
                return Err(self.error("The path group that starts here is never closed with ')'.", &token));
            }
            self.advance();
            self.consume_path_postfix();
            return Ok(());
        }
        if matches!(self.current().kind, TokenKind::Iri | TokenKind::PrefixedName) || self.current().is_keyword("a") {
            self.advance();
            self.consume_path_postfix();
            return Ok(());
        }
        let current = self.current().clone();
        Err(self.error(
            &format!("Expected a predicate or property path, found '{}'.", current.value),
            &current,
        ))
    }

    fn consume_path_postfix(&mut self) {
        if self.current().is_punct("?") || self.current().is_punct("*") || self.current().is_punct("+") {
            self.advance();
        }
    }

    fn parse_term_or_blank(&mut self, scope: &mut GroupScope, subject_position: bool) -> Parsed<SparqlTerm> {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Variable => {
                self.advance();
                return Ok(SparqlTerm::Variable {
                    name: without_sigil(&token.value),
                    written: token.value,
                });
            }
            TokenKind::Iri => {
                self.advance();
                return Ok(SparqlTerm::Iri {
                    iri: self.resolve_iri(&token),
                    written: token.value,
                });
            }
            TokenKind::PrefixedName => {
                self.advance();
                return Ok(SparqlTerm::Iri {
                    iri: self.expand_prefixed(&token)?,
                    written: token.value,
                });
            }
            TokenKind::BlankNodeLabel => {
                self.advance();
                if let Some(labeled) = self.labeled_anonymous.get(&token.value) {
                    return Ok(labeled.clone());
                }
                let labeled = SparqlTerm::Anonymous {
                    ordinal: self.next_anonymous(),
                    label: token.value.clone(),
                };
                self.labeled_anonymous.insert(token.value, labeled.clone());
                return Ok(labeled);
            }
            TokenKind::String => return self.parse_literal(&token),
            TokenKind::Number => {
                self.advance();
                return Ok(SparqlTerm::Literal {
                    lexical: token.value.clone(),
                    datatype: terminals::number_datatype(&token.value).to_owned(),
                    language: String::new(),
                    written: token.value,
                });
            }
            TokenKind::Boolean => {
                self.advance();
                return Ok(SparqlTerm::Literal {
                    lexical: token.value.clone(),
                    datatype: XSD_BOOLEAN.to_owned(),
                    language: String::new(),
                    written: token.value,
                });
            }
            _ => {}
        }
        if token.is_punct("[") {
            let anonymous = SparqlTerm::Anonymous {
                ordinal: self.next_anonymous(),
                label: String::new(),
            };
            self.advance();
            if self.current().is_punct("]") {
                self.advance();
                return Ok(anonymous);
            }
            self.parse_property_list(scope, &anonymous)?;
            self.expect_punct("]")?;
            return Ok(anonymous);
        }
        if token.is_punct("(") {
            return self.parse_collection(scope);
        }
        let position = if subject_position { "subject" } else { "object" };
        Err(self.error(&format!("Expected a {position} term, found '{}'.", token.value), &token))
    }

    fn next_anonymous(&mut self) -> usize {
        let ordinal = self.anonymous_ordinal;
        self.anonymous_ordinal += 1;
        ordinal
    }

    fn parse_collection(&mut self, scope: &mut GroupScope) -> Parsed<SparqlTerm> {
        self.advance();
        if self.current().is_punct(")") {
            self.advance();
            return Ok(SparqlTerm::iri(RDF_NIL, "()"));
        }
        // ( e1 e2 ... ) expands to its cons cells, each an anonymous variable.
        let head = SparqlTerm::Anonymous {
            ordinal: self.next_anonymous(),
            label: String::new(),
        };
        let mut current = head.clone();
        let first = SparqlTerm::iri(RDF_FIRST, "rdf:first");
        let rest = SparqlTerm::iri(RDF_REST, "rdf:rest");
        loop {
            let item = self.parse_term_or_blank(scope, false)?;
            scope.patterns.push(TriplePattern {
                subject: current.clone(),
                predicate: first.clone(),
                object: item,
            });
            if self.current().is_punct(")") {
                self.advance();
                scope.patterns.push(TriplePattern {
                    subject: current,
                    predicate: rest,
                    object: SparqlTerm::iri(RDF_NIL, "rdf:nil"),
                });
                return Ok(head);
            }
            let next = SparqlTerm::Anonymous {
                ordinal: self.next_anonymous(),
                label: String::new(),
            };
            scope.patterns.push(TriplePattern {
                subject: current,
                predicate: rest.clone(),
                object: next.clone(),
            });
            current = next;
        }
    }

    fn parse_literal(&mut self, string_token: &Token) -> Parsed<SparqlTerm> {
        self.advance();
        let lexical = self.decode_string(string_token)?;
        if self.current().kind == TokenKind::LanguageTag {
            let tag = without_sigil(&self.current().value);
            self.advance();
            return Ok(SparqlTerm::Literal {
                lexical,
                datatype: String::new(),
                language: tag,
                written: self.slice_tokens(self.index - 2, self.index - 1),
            });
        }
        if self.current().is_punct("^^") {
            self.advance();
            let datatype = self.current().clone();
            if datatype.kind == TokenKind::Iri {
                self.advance();
                return Ok(SparqlTerm::Literal {
                    lexical,
                    datatype: self.resolve_iri(&datatype),
                    language: String::new(),
                    written: self.slice_tokens(self.index - 3, self.index - 1),
                });
            }
            if datatype.kind == TokenKind::PrefixedName {
                self.advance();
                return Ok(SparqlTerm::Literal {
                    lexical,
                    datatype: self.expand_prefixed(&datatype)?,
                    language: String::new(),
                    written: self.slice_tokens(self.index - 3, self.index - 1),
                });
            }
            return Err(self.error("After '^^' comes the datatype IRI.", &datatype));
        }
        Ok(SparqlTerm::Literal {
            lexical,
            datatype: String::new(),
            language: String::new(),
            written: string_token.value.clone(),
        })
    }

    // -- Solution modifiers: frame, kept as written ------------------------------------------------------------

    fn parse_solution_modifiers(&mut self) -> Parsed<(Vec<String>, Option<SparqlConstraint>)> {
        let mut rows = Vec::new();
        let mut trailing_values = None;
        loop {
            if self.current().is_keyword("GROUP") {
                rows.push(self.slice_modifier(&["HAVING", "ORDER", "LIMIT", "OFFSET", "VALUES"])?);
            } else if self.current().is_keyword("HAVING") {
                rows.push(self.slice_modifier(&["ORDER", "LIMIT", "OFFSET", "VALUES"])?);
            } else if self.current().is_keyword("ORDER") {
                rows.push(self.slice_modifier(&["LIMIT", "OFFSET", "VALUES"])?);
            } else if self.current().is_keyword("LIMIT") || self.current().is_keyword("OFFSET") {
                let start = self.index;
                self.advance();
                self.expect(TokenKind::Number, "a whole number")?;
                rows.push(self.slice_tokens(start, self.index - 1));
            } else if self.current().is_keyword("VALUES") {
                trailing_values = Some(self.parse_values()?);
            } else {
                return Ok((rows, trailing_values));
            }
        }
    }

    fn slice_modifier(&mut self, stop_keywords: &[&str]) -> Parsed<String> {
        let start = self.index;
        self.advance();
        while self.current().kind != TokenKind::EndOfFile {
            if self.current().kind == TokenKind::Name && stop_keywords.iter().any(|k| self.current().is_keyword(k)) {
                break;
            }
            if self.current().is_punct("(") {
                self.skip_balanced("(")?;
                continue;
            }
            self.advance();
        }
        Ok(self.slice_tokens(start, self.index - 1))
    }

    // -- Finishing: paths and the variable index ---------------------------------------------------------------

    fn finish(&mut self, mut model: SparqlQueryModel) -> SparqlQueryModel {
        model.where_clause.path = "where".to_owned();
        assign_paths(&mut model.where_clause);
        if let Some(template) = &mut model.template {
            template.path = "template".to_owned();
            assign_paths(template);
        }
        let mut usages: Vec<UsageBuilder> = Vec::new();
        self.index_scope(&model.where_clause, &mut usages);
        if let Some(template) = &model.template {
            self.index_scope(template, &mut usages);
        }
        let mut projection = model.projection.clone();
        if model.projects_all && model.form == SparqlQueryForm::Select {
            let mut ordered: Vec<&UsageBuilder> = usages.iter().collect();
            ordered.sort_by_key(|u| u.first_seen);
            projection = ordered
                .iter()
                .map(|u| SparqlProjectionItem {
                    name: u.name.clone(),
                    expression_text: String::new(),
                })
                .collect();
        }
        for item in &projection {
            let at = self.get_or_add(&mut usages, &item.name);
            usages[at].projected = true;
            if !item.expression_text.is_empty() {
                usages[at].defining_expression = item.expression_text.clone();
            }
        }
        model.base_iri = self.base_iri.clone();
        model.prefixes = self.prefixes.clone();
        model.projection = projection;
        model.variables = usages
            .into_iter()
            .map(|u| SparqlVariableUsage {
                name: u.name,
                pattern_occurrences: u.pattern_occurrences,
                projected: u.projected,
                defining_expression: u.defining_expression,
                scope_paths: u.scope_paths,
            })
            .collect();
        model
    }

    fn get_or_add(&mut self, usages: &mut Vec<UsageBuilder>, name: &str) -> usize {
        if let Some(at) = usages.iter().position(|u| u.name == name) {
            return at;
        }
        usages.push(UsageBuilder {
            name: name.to_owned(),
            pattern_occurrences: 0,
            projected: false,
            defining_expression: String::new(),
            scope_paths: Vec::new(),
            first_seen: self.seen_counter,
        });
        self.seen_counter += 1;
        usages.len() - 1
    }

    fn touch(&mut self, usages: &mut Vec<UsageBuilder>, name: &str, pattern_position: bool, scope_path: &str) {
        let at = self.get_or_add(usages, name);
        let builder = &mut usages[at];
        if pattern_position {
            builder.pattern_occurrences += 1;
        }
        if !builder.scope_paths.iter().any(|p| p == scope_path) {
            builder.scope_paths.push(scope_path.to_owned());
        }
    }

    fn index_scope(&mut self, scope: &GroupScope, usages: &mut Vec<UsageBuilder>) {
        for pattern in &scope.patterns {
            for term in [&pattern.subject, &pattern.predicate, &pattern.object] {
                if let SparqlTerm::Variable { name, .. } = term {
                    self.touch(usages, name, true, &scope.path);
                }
            }
        }
        for constraint in &scope.constraints {
            for name in &constraint.referenced_variables {
                self.touch(usages, name, false, &scope.path);
            }
            if !constraint.defined_variable.is_empty() {
                let at = self.get_or_add(usages, &constraint.defined_variable);
                usages[at].defining_expression = constraint.text.clone();
                self.touch(usages, &constraint.defined_variable, false, &scope.path);
            }
        }
        for sub_select in &scope.sub_selects {
            for name in &sub_select.projected_names {
                self.touch(usages, name, false, &scope.path);
            }
        }
        for child in &scope.children {
            self.index_scope(child, usages);
        }
    }

    // -- Names, IRIs and strings -------------------------------------------------------------------------------

    fn resolve_iri(&self, token: &Token) -> String {
        let value = unbracket(&token.value);
        if value.contains(':') || self.base_iri.is_empty() {
            value
        } else {
            format!("{}{value}", self.base_iri)
        }
    }

    fn expand_prefixed(&self, token: &Token) -> Parsed<String> {
        let colon = token.value.find(':').unwrap_or_default();
        let prefix = &token.value[..colon];
        let local = &token.value[colon + 1..];
        let Some(iri) = self.prefix_map.get(prefix) else {
            let shown = if prefix.is_empty() {
                "(the default prefix)"
            } else {
                prefix
            };
            return Err(self.error(
                &format!(
                    "The prefix '{shown}' is used here but never declared - SPARQL requires every prefix to be declared with PREFIX."
                ),
                token,
            ));
        };
        Ok(format!("{iri}{}", terminals::decode_local(local)))
    }

    fn decode_string(&self, token: &Token) -> Parsed<String> {
        let raw = &self.text[token.start..token.end()];
        // An escape SPARQL does not know is refused at its own line.
        if let Some((offset, message)) = terminals::escape_refusal(raw) {
            return Err(ParseError {
                message,
                line: self.tokenizer.line_of(token.start + offset) + 1,
            });
        }
        Ok(terminals::decode_string(raw))
    }

    // -- Token plumbing ----------------------------------------------------------------------------------------

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn next(&self) -> &Token {
        self.tokens
            .get(self.index + 1)
            .unwrap_or_else(|| &self.tokens[self.tokens.len() - 1])
    }

    fn advance(&mut self) {
        self.index = (self.index + 1).min(self.tokens.len() - 1);
    }

    fn expect(&mut self, kind: TokenKind, described: &str) -> Parsed<Token> {
        let token = self.current().clone();
        if token.kind != kind {
            return Err(self.error(&format!("Expected {described}, found '{}'.", shown(&token)), &token));
        }
        self.advance();
        Ok(token)
    }

    fn expect_punct(&mut self, punctuation: &str) -> Parsed<()> {
        let token = self.current().clone();
        if !token.is_punct(punctuation) {
            return Err(self.error(&format!("Expected '{punctuation}', found '{}'.", shown(&token)), &token));
        }
        self.advance();
        Ok(())
    }

    /// The index of the token closing the bracket at `open_index`, `(`, `{` and `[` counted together.
    fn find_balanced_end(&self, open_index: usize) -> Parsed<usize> {
        let mut depth = 0_i64;
        for (i, token) in self.tokens.iter().enumerate().skip(open_index) {
            if is_open(token) {
                depth += 1;
            } else if is_close(token) {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
        }
        Err(self.error("The bracket that opens here is never closed.", &self.tokens[open_index]))
    }

    /// Consumes from the current opening bracket through its balanced close.
    fn skip_balanced(&mut self, expected_open: &str) -> Parsed<()> {
        let current = self.current().clone();
        if !current.is_punct(expected_open) {
            return Err(self.error(
                &format!("Expected '{expected_open}', found '{}'.", current.value),
                &current,
            ));
        }
        self.index = self.find_balanced_end(self.index)? + 1;
        Ok(())
    }

    /// The first `keyword` between the bounds at bracket depth zero relative to `start`.
    fn find_top_level_keyword(&self, start: usize, end_exclusive: usize, keyword: &str) -> Option<usize> {
        let mut depth = 0_i64;
        for i in start..end_exclusive {
            let token = &self.tokens[i];
            if token.kind == TokenKind::Punct {
                if is_open(token) {
                    depth += 1;
                } else if is_close(token) {
                    depth -= 1;
                }
            } else if depth == 0 && token.is_keyword(keyword) {
                return Some(i);
            }
        }
        None
    }

    fn depth_between(&self, start: usize, end_exclusive: usize) -> i64 {
        let mut depth = 0_i64;
        for token in &self.tokens[start..end_exclusive] {
            if is_open(token) {
                depth += 1;
            } else if is_close(token) {
                depth -= 1;
            }
        }
        depth
    }

    fn variables_between(&self, start: usize, end_inclusive: usize) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        let mut i = start;
        while i <= end_inclusive && i < self.tokens.len() {
            if self.tokens[i].kind == TokenKind::Variable {
                let name = without_sigil(&self.tokens[i].value);
                if !names.contains(&name) {
                    names.push(name);
                }
            }
            i += 1;
        }
        names
    }

    /// The raw source text from the first token through the last, exactly as written.
    fn slice_tokens(&self, first_index: usize, last_index: usize) -> String {
        let first = &self.tokens[first_index];
        let last = &self.tokens[first_index.max(last_index)];
        self.text[first.start..last.end()].iter().collect()
    }

    fn line_of(&self, token: &Token) -> usize {
        self.tokenizer.line_of(token.start) + 1
    }

    fn error(&self, message: &str, at: &Token) -> ParseError {
        ParseError {
            message: message.to_owned(),
            line: self.line_of(at),
        }
    }
}

fn starts_verb(token: &Token) -> bool {
    matches!(
        token.kind,
        TokenKind::Variable | TokenKind::Iri | TokenKind::PrefixedName
    ) || token.is_keyword("a")
        || token.is_punct("^")
        || token.is_punct("!")
        || token.is_punct("(")
}

/// The ordinal a new child of `kind` takes under `parent`: its count among same-kind siblings so far.
fn next_ordinal(parent: &GroupScope, kind: GroupScopeKind) -> usize {
    parent.children.iter().filter(|c| c.kind == kind).count()
}

fn assign_paths(scope: &mut GroupScope) {
    let path = scope.path.clone();
    for child in &mut scope.children {
        child.path = format!("{path}/{}.{}", child.kind.name(), child.ordinal);
        assign_paths(child);
    }
}

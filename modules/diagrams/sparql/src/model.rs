//! What a query file says (etalii.adp spec 030, task T084): the prologue, the form and its frame, the scope tree of
//! graph patterns, and the query-wide variable index.
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/_Model/`
//! (`SparqlTerm.cs`, `TriplePattern.cs`, `GroupScope.cs`, `GroupScopeKind.cs`, `SparqlConstraint.cs`,
//! `SparqlSubSelect.cs` and `SparqlQueryModel.cs`). This is the module's own model, deliberately not the RDF family's:
//! a query is not a serialization of a graph.

/// One position of a triple pattern: a variable, an anonymous variable (blank-node syntax, which in a query means a
/// variable), a concrete IRI, a literal, or, in predicate position only, a property path kept as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SparqlTerm {
    /// A named variable: `name` without its `?` or `$` sigil, `written` with it.
    Variable { name: String, written: String },
    /// An anonymous variable, with its document-order `ordinal` and the written label of a `_:label` form (empty for
    /// the bracketed forms).
    Anonymous { ordinal: usize, label: String },
    /// A concrete IRI, expanded against the prologue, and as written (prefixed, bracketed, or the keyword `a`).
    Iri { iri: String, written: String },
    /// A literal: its decoded lexical form, its datatype IRI (empty for a plain or tagged literal), its language tag
    /// without its `@`, and the literal exactly as written.
    Literal {
        lexical: String,
        datatype: String,
        language: String,
        written: String,
    },
    /// A property path in predicate position, exactly as written.
    Path { written: String },
}

impl SparqlTerm {
    /// A variable named `name`, written with a `?`.
    pub fn variable(name: &str) -> SparqlTerm {
        SparqlTerm::Variable {
            name: name.to_owned(),
            written: format!("?{name}"),
        }
    }

    /// An IRI written `written`.
    pub fn iri(iri: &str, written: &str) -> SparqlTerm {
        SparqlTerm::Iri {
            iri: iri.to_owned(),
            written: written.to_owned(),
        }
    }

    /// The term exactly as the query wrote it, for every surface a human reads.
    pub fn as_written(&self) -> String {
        match self {
            SparqlTerm::Variable { written, .. }
            | SparqlTerm::Iri { written, .. }
            | SparqlTerm::Literal { written, .. }
            | SparqlTerm::Path { written } => written.clone(),
            SparqlTerm::Anonymous { label, .. } => {
                if label.is_empty() {
                    "[]".to_owned()
                } else {
                    label.clone()
                }
            }
        }
    }
}

/// One triple pattern of the where clause or of a `CONSTRUCT` template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriplePattern {
    pub subject: SparqlTerm,
    pub predicate: SparqlTerm,
    pub object: SparqlTerm,
}

/// The kinds of group a where clause is built of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupScopeKind {
    /// A plain group: the root where clause, or a braced group inside it.
    Group,
    Optional,
    /// A `UNION`, whose alternatives are its children, each a [`GroupScopeKind::Branch`].
    Union,
    Branch,
    Minus,
    Graph,
    Service,
    /// A `CONSTRUCT` template.
    Template,
}

impl GroupScopeKind {
    /// The kind in lower case, as paths and region kinds write it.
    pub fn name(self) -> &'static str {
        match self {
            GroupScopeKind::Group => "group",
            GroupScopeKind::Optional => "optional",
            GroupScopeKind::Union => "union",
            GroupScopeKind::Branch => "branch",
            GroupScopeKind::Minus => "minus",
            GroupScopeKind::Graph => "graph",
            GroupScopeKind::Service => "service",
            GroupScopeKind::Template => "template",
        }
    }
}

/// The expression-shaped constructs, which annotate rather than draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlConstraintKind {
    Filter,
    Bind,
    Values,
}

impl SparqlConstraintKind {
    /// The kind in lower case.
    pub fn name(self) -> &'static str {
        match self {
            SparqlConstraintKind::Filter => "filter",
            SparqlConstraintKind::Bind => "bind",
            SparqlConstraintKind::Values => "values",
        }
    }
}

/// One expression-shaped constraint, kept exactly as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlConstraint {
    pub kind: SparqlConstraintKind,
    /// The whole construct as written, keyword included.
    pub text: String,
    /// The variable a `BIND` defines, without its sigil; empty otherwise.
    pub defined_variable: String,
    /// The variable names referenced inside, without sigils, in order of first mention.
    pub referenced_variables: Vec<String>,
}

/// A subquery, collapsed to one node stating its projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlSubSelect {
    /// The projection as written, e.g. `SELECT ?x (COUNT(?y) AS ?n)`.
    pub projection_text: String,
    /// The whole subquery as written, braces included.
    pub text: String,
    /// The names it projects into the outer scope, without sigils.
    pub projected_names: Vec<String>,
    pub projects_all: bool,
    /// Its position among the parent scope's subqueries.
    pub ordinal: usize,
}

/// One node of the scope tree: a group of triple patterns, the groups nested inside it, and its constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupScope {
    pub kind: GroupScopeKind,
    /// Its position among same-kind siblings, in source order.
    pub ordinal: usize,
    /// The graph term of a `GRAPH`, the endpoint of a `SERVICE` (with `SILENT` when written); empty otherwise.
    pub label: String,
    pub patterns: Vec<TriplePattern>,
    pub children: Vec<GroupScope>,
    pub constraints: Vec<SparqlConstraint>,
    pub sub_selects: Vec<SparqlSubSelect>,
    /// The path naming this scope from the root: `where`, `where/optional.0`, `template`.
    pub path: String,
}

impl GroupScope {
    /// An empty scope of `kind`.
    pub fn new(kind: GroupScopeKind, ordinal: usize, label: &str) -> GroupScope {
        GroupScope {
            kind,
            ordinal,
            label: label.to_owned(),
            patterns: Vec::new(),
            children: Vec::new(),
            constraints: Vec::new(),
            sub_selects: Vec::new(),
            path: String::new(),
        }
    }

    /// This scope and every scope nested in it, depth first.
    pub fn scopes(&self) -> Vec<&GroupScope> {
        let mut all = vec![self];
        for child in &self.children {
            all.extend(child.scopes());
        }
        all
    }
}

/// The four query forms of SPARQL 1.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlQueryForm {
    Select,
    Construct,
    Ask,
    Describe,
}

/// One projected item of a `SELECT` clause: the variable's name, and the defining expression of an
/// `(expr AS ?name)` item as written (empty for a plain variable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlProjectionItem {
    pub name: String,
    pub expression_text: String,
}

/// One `PREFIX` declaration: the prefix without its colon, the IRI it expands to, and its 1-based line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlPrefixDeclaration {
    pub prefix: String,
    pub iri: String,
    pub line: usize,
}

/// How one variable name is used across the whole query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlVariableUsage {
    pub name: String,
    /// How many triple-pattern positions mention it: its join count.
    pub pattern_occurrences: usize,
    /// Whether the query's own projection carries it outward.
    pub projected: bool,
    /// The `BIND` or projection alias that defines it, as written; empty for an ordinary variable.
    pub defining_expression: String,
    /// The paths of every scope that mentions it, in first-mention order.
    pub scope_paths: Vec<String>,
}

/// What a query file says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlQueryModel {
    pub base_iri: String,
    pub prefixes: Vec<SparqlPrefixDeclaration>,
    pub form: SparqlQueryForm,
    pub distinct: bool,
    pub reduced: bool,
    pub projects_all: bool,
    /// The projected items; for `SELECT *`, the in-scope names it resolves to.
    pub projection: Vec<SparqlProjectionItem>,
    /// A `DESCRIBE` query's targets, as written.
    pub describe_targets: Vec<String>,
    /// The same targets as terms, IRIs resolved.
    pub describe_terms: Vec<SparqlTerm>,
    /// The `FROM` and `FROM NAMED` clauses, as written.
    pub dataset_clauses: Vec<String>,
    pub template: Option<GroupScope>,
    pub where_clause: GroupScope,
    /// The solution modifiers as written rows.
    pub modifier_rows: Vec<String>,
    /// The query-wide variable index, in the order names were first seen.
    pub variables: Vec<SparqlVariableUsage>,
}

impl Default for SparqlQueryModel {
    /// The model a file that could not be parsed carries: nothing.
    fn default() -> Self {
        let mut where_clause = GroupScope::new(GroupScopeKind::Group, 0, "");
        where_clause.path = "where".to_owned();
        SparqlQueryModel {
            base_iri: String::new(),
            prefixes: Vec::new(),
            form: SparqlQueryForm::Select,
            distinct: false,
            reduced: false,
            projects_all: false,
            projection: Vec::new(),
            describe_targets: Vec::new(),
            describe_terms: Vec::new(),
            dataset_clauses: Vec::new(),
            template: None,
            where_clause,
            modifier_rows: Vec::new(),
            variables: Vec::new(),
        }
    }
}

impl SparqlQueryModel {
    /// The usage of the variable `name`.
    pub fn variable(&self, name: &str) -> Option<&SparqlVariableUsage> {
        self.variables.iter().find(|v| v.name == name)
    }
}

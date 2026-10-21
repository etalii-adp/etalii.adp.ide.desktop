//! Model in, drawn elements out: the variable-as-node projection (etalii.adp spec 030, task T084).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/sparql/backend/EtAlii.Adp.Diagram.Sparql/
//! SparqlProjection.cs` and `_Model/SparqlProjectionResult.cs`. One node per variable name however many patterns
//! mention it, so a node's degree on the canvas is its join count; one node per distinct concrete term form; and one
//! placement rule for all of them: every node lives at the shallowest scope that references it, and region edges reach
//! out to it. Pure and deterministic: the same model always yields the same elements in the same order.

use std::collections::{HashMap, HashSet};

use crate::model::{GroupScope, GroupScopeKind, SparqlConstraintKind, SparqlQueryForm, SparqlQueryModel, SparqlTerm};

/// The local sanity bound on drawn elements, so a generated monster degrades honestly.
pub const SANITY_BOUND: usize = 500;

/// What kind of node a drawn node is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparqlNodeKind {
    Variable,
    Anonymous,
    Iri,
    Literal,
    SubSelect,
}

/// One drawn node, placed at the shallowest scope that references it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlNode {
    /// `var:{name}`, `anon:{ordinal}`, `iri:{full}`, `lit:{escaped}` or `sub:{path}.{ordinal}`.
    pub id: String,
    pub kind: SparqlNodeKind,
    /// `?name`, a term as written, a lexical form, or a subquery's projection.
    pub display: String,
    /// The scope whose region frames it; `where` is the open canvas.
    pub scope_path: String,
    /// An IRI's full form, a literal's written form, or a subquery's whole text.
    pub full: String,
    /// A literal's datatype or language annotation.
    pub annotation: String,
    pub projected: bool,
    pub join_count: usize,
    pub defining_expression: String,
}

impl SparqlNode {
    fn new(id: &str, kind: SparqlNodeKind, display: &str, scope_path: &str) -> SparqlNode {
        SparqlNode {
            id: id.to_owned(),
            kind,
            display: display.to_owned(),
            scope_path: scope_path.to_owned(),
            full: String::new(),
            annotation: String::new(),
            projected: false,
            join_count: 0,
            defining_expression: String::new(),
        }
    }
}

/// One drawn edge: a triple pattern, or a subquery's projected-name join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlEdge {
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    /// The predicate or property path, as written.
    pub label: String,
    pub is_path: bool,
    /// The scope whose group states the pattern.
    pub scope_path: String,
}

/// One containment region: a non-root scope, or the `CONSTRUCT` template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlRegion {
    /// `region:{scope path}`.
    pub id: String,
    /// The construct kind, lower case.
    pub kind: String,
    pub label: String,
    /// The region this one nests inside; empty at the top level.
    pub parent_region_id: String,
    pub scope_path: String,
}

/// One annotation badge: a `FILTER`, `BIND` or `VALUES` as written, anchored to what it constrains, defines or feeds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlAnnotation {
    /// `note:{scope path}/{kind}.{ordinal}`.
    pub id: String,
    pub kind: String,
    pub text: String,
    /// A region, a variable node, or empty for the root scope's own filters.
    pub attached_to_id: String,
    pub scope_path: String,
}

/// Everything the canvas draws, in deterministic order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SparqlProjectionResult {
    pub nodes: Vec<SparqlNode>,
    pub edges: Vec<SparqlEdge>,
    pub regions: Vec<SparqlRegion>,
    pub annotations: Vec<SparqlAnnotation>,
    /// The query form line: `SELECT DISTINCT`, `ASK`, ...
    pub header_form: String,
    /// The dataset clauses and solution modifiers, as written rows.
    pub header_rows: Vec<String>,
    pub shown: usize,
    pub total: usize,
}

impl SparqlProjectionResult {
    /// Whether the sanity bound cut this projection down.
    pub fn truncated(&self) -> bool {
        self.shown < self.total
    }
}

/// What `model` draws.
pub fn project(model: &SparqlQueryModel) -> SparqlProjectionResult {
    Builder {
        model,
        nodes: Vec::new(),
        edges: Vec::new(),
        regions: Vec::new(),
        annotations: Vec::new(),
        node_ids: HashSet::new(),
        term_scopes: HashMap::new(),
        edge_ordinals: HashMap::new(),
    }
    .build()
}

struct Builder<'a> {
    model: &'a SparqlQueryModel,
    nodes: Vec<SparqlNode>,
    edges: Vec<SparqlEdge>,
    regions: Vec<SparqlRegion>,
    annotations: Vec<SparqlAnnotation>,
    node_ids: HashSet<String>,
    term_scopes: HashMap<String, Vec<String>>,
    edge_ordinals: HashMap<String, usize>,
}

/// The id of a region framing the scope at `scope_path`.
pub fn region_id(scope_path: &str) -> String {
    format!("region:{scope_path}")
}

/// The shallowest scope that references a term: the longest common ancestor of every mentioning scope, lifted out of
/// a `UNION`.
fn place_at(scope_paths: &[String]) -> String {
    let Some(first) = scope_paths.first() else {
        return "where".to_owned();
    };
    let mut common: Vec<&str> = first.split('/').collect();
    for path in &scope_paths[1..] {
        let segments: Vec<&str> = path.split('/').collect();
        let mut length = 0;
        while length < common.len() && length < segments.len() && common[length] == segments[length] {
            length += 1;
        }
        common.truncate(length);
    }
    // A union scope holds branches, not nodes: a node landing on the union itself lifts to the union's parent.
    while common.len() > 1 && common[common.len() - 1].starts_with("union.") {
        common.pop();
    }
    if common.is_empty() {
        "where".to_owned()
    } else {
        common.join("/")
    }
}

fn region_label(scope: &GroupScope) -> String {
    match scope.kind {
        GroupScopeKind::Optional => "OPTIONAL".to_owned(),
        GroupScopeKind::Union => "UNION".to_owned(),
        GroupScopeKind::Minus => "MINUS".to_owned(),
        GroupScopeKind::Graph => format!("GRAPH {}", scope.label),
        GroupScopeKind::Service => format!("SERVICE {}", scope.label),
        GroupScopeKind::Template => "CONSTRUCT".to_owned(),
        GroupScopeKind::Branch | GroupScopeKind::Group => String::new(),
    }
}

fn short_datatype(iri: &str) -> String {
    if let Some(hash) = iri.rfind('#')
        && hash + 1 < iri.len()
    {
        return iri[hash + 1..].to_owned();
    }
    match iri.rfind('/') {
        Some(slash) if slash + 1 < iri.len() => iri[slash + 1..].to_owned(),
        _ => iri.to_owned(),
    }
}

/// `value` as .NET's `Uri.EscapeDataString` writes it: every byte of its UTF-8 but the unreserved characters
/// percent-encoded in upper case.
pub fn escape_data_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            escaped.push(char::from(byte));
        } else {
            escaped.push_str(&format!("%{byte:02X}"));
        }
    }
    escaped
}

/// The stable, space-free element id a term keys layout and selection by.
pub fn node_id(term: &SparqlTerm) -> String {
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
            let mut id = format!("lit:{}", escape_data_string(lexical));
            if !language.is_empty() {
                id.push('@');
                id.push_str(language);
            }
            if !datatype.is_empty() {
                id.push('^');
                id.push_str(&escape_data_string(&short_datatype(datatype)));
            }
            id
        }
        SparqlTerm::Path { .. } => String::new(),
    }
}

impl Builder<'_> {
    fn build(mut self) -> SparqlProjectionResult {
        let model = self.model;
        // First pass: where every term is mentioned, so placement can find the shallowest scope.
        self.collect_term_scopes(&model.where_clause);
        if let Some(template) = &model.template {
            self.collect_term_scopes(template);
        }
        // Regions frame the tree; they come first so a region always precedes its contents.
        self.collect_regions(&model.where_clause, "");
        if let Some(template) = &model.template {
            self.regions.push(SparqlRegion {
                id: region_id(&template.path),
                kind: "template".to_owned(),
                label: "CONSTRUCT".to_owned(),
                parent_region_id: String::new(),
                scope_path: template.path.clone(),
            });
            self.collect_regions(template, &region_id(&template.path));
        }
        // Every named variable gets its one node, ordered by first scope mention, then name.
        let mut usages: Vec<_> = model.variables.iter().collect();
        usages.sort_by(|a, b| {
            let first = |u: &crate::model::SparqlVariableUsage| u.scope_paths.first().cloned().unwrap_or_default();
            first(a).cmp(&first(b)).then_with(|| a.name.cmp(&b.name))
        });
        for usage in usages {
            let mut node = SparqlNode::new(
                &format!("var:{}", usage.name),
                SparqlNodeKind::Variable,
                &format!("?{}", usage.name),
                &place_at(&usage.scope_paths),
            );
            node.projected = usage.projected;
            node.join_count = usage.pattern_occurrences;
            node.defining_expression = usage.defining_expression.clone();
            self.add_node(node);
        }
        // A DESCRIBE's targets draw as nodes like any other term.
        for term in &model.describe_terms {
            self.ensure_term_node(term);
        }
        self.collect_patterns(&model.where_clause);
        if let Some(template) = &model.template {
            self.collect_patterns(template);
        }
        self.collect_sub_selects(&model.where_clause);
        self.collect_annotations(&model.where_clause);
        let header_form = self.header_form();
        let header_rows: Vec<String> = model
            .dataset_clauses
            .iter()
            .chain(&model.modifier_rows)
            .cloned()
            .collect();
        self.bound(header_form, header_rows)
    }

    fn collect_term_scopes(&mut self, scope: &GroupScope) {
        for pattern in &scope.patterns {
            for term in [&pattern.subject, &pattern.object] {
                let id = node_id(term);
                if id.is_empty() {
                    continue;
                }
                let scopes = self.term_scopes.entry(id).or_default();
                if !scopes.contains(&scope.path) {
                    scopes.push(scope.path.clone());
                }
            }
        }
        for child in &scope.children {
            self.collect_term_scopes(child);
        }
    }

    /// Every non-root scope is a region; the root where group is the open canvas.
    fn collect_regions(&mut self, scope: &GroupScope, parent_region_id: &str) {
        for child in &scope.children {
            let id = region_id(&child.path);
            self.regions.push(SparqlRegion {
                id: id.clone(),
                kind: child.kind.name().to_owned(),
                label: region_label(child),
                parent_region_id: parent_region_id.to_owned(),
                scope_path: child.path.clone(),
            });
            self.collect_regions(child, &id);
        }
    }

    fn collect_patterns(&mut self, scope: &GroupScope) {
        for pattern in &scope.patterns {
            let from_id = self.ensure_term_node(&pattern.subject);
            let to_id = self.ensure_term_node(&pattern.object);
            let label = pattern.predicate.as_written();
            let is_path = matches!(pattern.predicate, SparqlTerm::Path { .. });
            let key = format!("{from_id}|{label}|{to_id}");
            let ordinal = self.edge_ordinals.get(&key).copied().unwrap_or(0);
            self.edge_ordinals.insert(key, ordinal + 1);
            self.edges.push(SparqlEdge {
                id: format!("edge:{from_id}|{label}|{to_id}|{ordinal}"),
                from_id,
                to_id,
                label,
                is_path,
                scope_path: scope.path.clone(),
            });
        }
        for child in &scope.children {
            self.collect_patterns(child);
        }
    }

    fn collect_sub_selects(&mut self, scope: &GroupScope) {
        for sub_select in &scope.sub_selects {
            let id = format!("sub:{}.{}", scope.path, sub_select.ordinal);
            let mut node = SparqlNode::new(&id, SparqlNodeKind::SubSelect, &sub_select.projection_text, &scope.path);
            node.full = sub_select.text.clone();
            self.add_node(node);
            // The join surface: one labeled edge per projected name.
            for name in &sub_select.projected_names {
                self.edges.push(SparqlEdge {
                    id: format!("edge:{id}|{name}|var:{name}|0"),
                    from_id: id.clone(),
                    to_id: format!("var:{name}"),
                    label: format!("?{name}"),
                    is_path: false,
                    scope_path: scope.path.clone(),
                });
            }
        }
        for child in &scope.children {
            self.collect_sub_selects(child);
        }
    }

    fn collect_annotations(&mut self, scope: &GroupScope) {
        let mut kind_counts: HashMap<&str, usize> = HashMap::new();
        for constraint in &scope.constraints {
            let kind = constraint.kind.name();
            let ordinal = kind_counts.get(kind).copied().unwrap_or(0);
            kind_counts.insert(kind, ordinal + 1);
            let attached_to = match constraint.kind {
                SparqlConstraintKind::Bind => format!("var:{}", constraint.defined_variable),
                SparqlConstraintKind::Values if !constraint.referenced_variables.is_empty() => {
                    format!("var:{}", constraint.referenced_variables[0])
                }
                _ if scope.path.contains('/') => region_id(&scope.path),
                _ => String::new(),
            };
            self.annotations.push(SparqlAnnotation {
                id: format!("note:{}/{kind}.{ordinal}", scope.path),
                kind: kind.to_owned(),
                text: constraint.text.clone(),
                attached_to_id: attached_to,
                scope_path: scope.path.clone(),
            });
        }
        for child in &scope.children {
            self.collect_annotations(child);
        }
    }

    fn add_node(&mut self, node: SparqlNode) {
        if self.node_ids.insert(node.id.clone()) {
            self.nodes.push(node);
        }
    }

    /// The node id for a subject or object term, creating the node on first mention.
    fn ensure_term_node(&mut self, term: &SparqlTerm) -> String {
        let id = node_id(term);
        if self.node_ids.contains(&id) {
            return id;
        }
        let node = match term {
            SparqlTerm::Variable { name, .. } => {
                SparqlNode::new(&id, SparqlNodeKind::Variable, &format!("?{name}"), "where")
            }
            SparqlTerm::Anonymous { .. } => {
                SparqlNode::new(&id, SparqlNodeKind::Anonymous, &term.as_written(), &self.place_for_term(&id))
            }
            SparqlTerm::Iri { iri, written } => {
                let mut node = SparqlNode::new(&id, SparqlNodeKind::Iri, written, &self.place_for_term(&id));
                node.full = iri.clone();
                node
            }
            SparqlTerm::Literal {
                lexical,
                datatype,
                language,
                written,
            } => {
                let mut node = SparqlNode::new(&id, SparqlNodeKind::Literal, lexical, &self.place_for_term(&id));
                node.full = written.clone();
                node.annotation = if !language.is_empty() {
                    format!("@{language}")
                } else if !datatype.is_empty() {
                    short_datatype(datatype)
                } else {
                    String::new()
                };
                node
            }
            // A path is never a subject or an object.
            SparqlTerm::Path { .. } => return id,
        };
        self.add_node(node);
        id
    }

    fn place_for_term(&self, id: &str) -> String {
        self.term_scopes
            .get(id)
            .map_or_else(|| "where".to_owned(), |scopes| place_at(scopes))
    }

    fn header_form(&self) -> String {
        let model = self.model;
        let mut form = match model.form {
            SparqlQueryForm::Select => "SELECT".to_owned(),
            SparqlQueryForm::Construct => "CONSTRUCT".to_owned(),
            SparqlQueryForm::Ask => "ASK".to_owned(),
            SparqlQueryForm::Describe => format!("DESCRIBE {}", model.describe_targets.join(" ")),
        };
        if model.distinct {
            form.push_str(" DISTINCT");
        }
        if model.reduced {
            form.push_str(" REDUCED");
        }
        form
    }

    fn bound(self, header_form: String, header_rows: Vec<String>) -> SparqlProjectionResult {
        let total = self.regions.len() + self.nodes.len() + self.edges.len() + self.annotations.len();
        if total <= SANITY_BOUND {
            return SparqlProjectionResult {
                nodes: self.nodes,
                edges: self.edges,
                regions: self.regions,
                annotations: self.annotations,
                header_form,
                header_rows,
                shown: total,
                total,
            };
        }
        // The cut takes regions, then nodes, then edges, then annotations, and drops any edge whose endpoint fell
        // past the bound: an honest first-N, never a hang.
        let mut remaining = SANITY_BOUND;
        let regions = take(self.regions, &mut remaining);
        let nodes = take(self.nodes, &mut remaining);
        let kept: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        let edges: Vec<SparqlEdge> = take(self.edges, &mut remaining)
            .into_iter()
            .filter(|e| kept.contains(e.from_id.as_str()) && kept.contains(e.to_id.as_str()))
            .collect();
        let annotations = take(self.annotations, &mut remaining);
        let shown = regions.len() + nodes.len() + edges.len() + annotations.len();
        SparqlProjectionResult {
            nodes,
            edges,
            regions,
            annotations,
            header_form,
            header_rows,
            shown,
            total,
        }
    }
}

fn take<T>(source: Vec<T>, remaining: &mut usize) -> Vec<T> {
    let taken: Vec<T> = source.into_iter().take(*remaining).collect();
    *remaining -= taken.len();
    taken
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_used_in_scopes_apart_lands_on_their_common_ancestor() {
        let paths = |p: &[&str]| p.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(place_at(&[]), "where");
        assert_eq!(place_at(&paths(&["where/optional.0", "where/optional.0/minus.0"])), "where/optional.0");
        assert_eq!(place_at(&paths(&["where/union.0/branch.0", "where/union.0/branch.1"])), "where");
        assert_eq!(place_at(&paths(&["where", "template"])), "where");
    }

    #[test]
    fn a_literal_id_is_escaped_as_dotnet_escapes_data() {
        assert_eq!(escape_data_string("a b/é~"), "a%20b%2F%C3%A9~");
        assert_eq!(short_datatype("http://www.w3.org/2001/XMLSchema#int"), "int");
    }
}

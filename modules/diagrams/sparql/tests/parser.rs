//! The parser over its three tiers (etalii.adp spec 030, task T084): the standalone host's `SparqlParser.Tests.cs` at
//! `8b6ccfc`, with the same expectations. The structural constructs, expression and path text kept exactly as written,
//! and the by-name refusal of SPARQL Update.

mod common;

use adp_diagram_sparql::model::{GroupScopeKind, SparqlConstraintKind, SparqlQueryForm, SparqlTerm};
use adp_diagram_sparql::parser::{self, RDF_FIRST, RDF_NIL, RDF_REST, RDF_TYPE, XSD_BOOLEAN};
use adp_diagram_sparql::terminals::{XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER};
use common::{parse_fixture, text};

fn iri_of(term: &SparqlTerm) -> &str {
    match term {
        SparqlTerm::Iri { iri, .. } => iri,
        other => panic!("not an IRI: {other:?}"),
    }
}

fn names(items: impl IntoIterator<Item = String>) -> Vec<String> {
    items.into_iter().collect()
}

// -- Tier one: parsed structurally ---------------------------------------------------------------------------------

#[test]
fn the_construct_corpus_parses_with_every_structural_construct_accounted_for() {
    let model = parse_fixture("constructs.rq");
    let patterns = &model.where_clause.patterns;

    // The prologue, in order, and the base.
    assert_eq!(names(model.prefixes.iter().map(|p| p.prefix.clone())), ["foaf", "ex"]);
    assert_eq!(
        names(model.prefixes.iter().map(|p| p.iri.clone())),
        ["http://xmlns.com/foaf/0.1/", "http://example.org/"]
    );
    assert_eq!(model.base_iri, "http://example.org/base/");

    // The form and its frame.
    assert_eq!(model.form, SparqlQueryForm::Select);
    assert!(model.distinct);
    assert_eq!(model.dataset_clauses, ["FROM <http://example.org/graphs/people>"]);

    // 'a' became rdf:type without losing its spelling.
    let typed = patterns
        .iter()
        .find(|p| matches!(&p.predicate, SparqlTerm::Iri { iri, .. } if iri == RDF_TYPE))
        .unwrap();
    assert_eq!(typed.predicate.as_written(), "a");
    assert_eq!(iri_of(&typed.object), "http://xmlns.com/foaf/0.1/Person");

    // The predicate list and object list fanned out from one subject.
    let person = SparqlTerm::variable("person");
    let knows: Vec<_> = patterns
        .iter()
        .filter(|p| {
            p.subject == person && matches!(&p.predicate, SparqlTerm::Iri { iri, .. } if iri == "http://xmlns.com/foaf/0.1/knows")
        })
        .collect();
    assert_eq!(knows.len(), 2);
    assert!(knows.iter().any(|p| matches!(&p.object, SparqlTerm::Iri { iri, .. } if iri == "http://example.org/carol")));

    // Bare numbers, booleans and typed or tagged strings carry their datatypes.
    let literal = |lexical: &str, datatype: &str, language: &str| {
        patterns.iter().any(|p| {
            matches!(&p.object, SparqlTerm::Literal { lexical: l, datatype: d, language: g, .. }
                if l == lexical && d == datatype && g == language)
        })
    };
    assert!(literal("42", XSD_INTEGER, ""));
    assert!(literal("3.14", XSD_DECIMAL, ""));
    assert!(literal("true", XSD_BOOLEAN, ""));
    assert!(literal("hello\nworld", "", "en"));
    assert!(literal("80", "http://www.w3.org/2001/XMLSchema#int", ""));

    // The relative IRI resolved against the base.
    assert!(patterns
        .iter()
        .any(|p| matches!(&p.object, SparqlTerm::Iri { iri, .. } if iri == "http://example.org/base/relative/place")));

    // The labeled blank node is one anonymous term across its uses.
    let labeled: Vec<_> = patterns
        .iter()
        .filter(|p| matches!(&p.subject, SparqlTerm::Anonymous { label, .. } if label == "_:g"))
        .collect();
    assert_eq!(labeled.len(), 2);
    assert_eq!(labeled[0].subject, labeled[1].subject);

    // The anonymous property list stated its triple and became a subject itself.
    let anonymous_with = |predicate: &str| {
        patterns.iter().any(|p| {
            matches!(&p.subject, SparqlTerm::Anonymous { label, .. } if label.is_empty())
                && matches!(&p.predicate, SparqlTerm::Iri { iri, .. } if iri == predicate)
        })
    };
    assert!(anonymous_with("http://example.org/leader"));
    assert!(anonymous_with("http://example.org/size"));

    // The collection expanded to its cons cells, ending at nil.
    assert!(patterns
        .iter()
        .any(|p| matches!(&p.predicate, SparqlTerm::Iri { iri, .. } if iri == RDF_FIRST)));
    assert!(patterns.iter().any(|p| {
        matches!(&p.predicate, SparqlTerm::Iri { iri, .. } if iri == RDF_REST)
            && matches!(&p.object, SparqlTerm::Iri { iri, .. } if iri == RDF_NIL)
    }));

    // The solution modifiers arrived as written rows, in order.
    assert_eq!(
        model.modifier_rows,
        [
            "GROUP BY ?person ?name",
            "HAVING (COUNT(?friend) > 1)",
            "ORDER BY DESC(?friends) ?name",
            "LIMIT 10",
            "OFFSET 5"
        ]
    );
}

#[test]
fn the_group_corpus_builds_the_scope_tree_with_stable_ordinals_and_paths() {
    let model = parse_fixture("groups.rq");
    let root = &model.where_clause;
    assert_eq!(root.path, "where");

    let optionals: Vec<_> = root.children.iter().filter(|c| c.kind == GroupScopeKind::Optional).collect();
    assert_eq!(optionals.len(), 2);
    assert_eq!(optionals[0].path, "where/optional.0");
    assert_eq!(optionals[1].path, "where/optional.1");

    let unions: Vec<_> = root.children.iter().filter(|c| c.kind == GroupScopeKind::Union).collect();
    assert_eq!(unions.len(), 1);
    assert_eq!(unions[0].children.len(), 3);
    assert!(unions[0].children.iter().all(|b| b.kind == GroupScopeKind::Branch));
    assert_eq!(unions[0].children[2].path, "where/union.0/branch.2");

    assert_eq!(root.children.iter().filter(|c| c.kind == GroupScopeKind::Minus).count(), 1);
    let graph = root.children.iter().find(|c| c.kind == GroupScopeKind::Graph).unwrap();
    assert_eq!(graph.label, "?g");
    let service = root.children.iter().find(|c| c.kind == GroupScopeKind::Service).unwrap();
    assert_eq!(service.label, "SILENT <http://example.org/sparql>");
}

#[test]
fn the_four_forms_all_parse() {
    assert_eq!(parse_fixture("ask-form.rq").form, SparqlQueryForm::Ask);

    let describe = parse_fixture("describe-form.rq");
    assert_eq!(describe.form, SparqlQueryForm::Describe);
    assert_eq!(describe.describe_targets, ["?s", "ex:thing"]);
    assert_eq!(describe.modifier_rows, ["LIMIT 5"]);

    let construct = parse_fixture("construct-form.rq");
    assert_eq!(construct.form, SparqlQueryForm::Construct);
    let template = construct.template.as_ref().unwrap();
    assert_eq!(template.kind, GroupScopeKind::Template);
    assert_eq!(template.patterns.len(), 1);
    assert_eq!(iri_of(&template.patterns[0].predicate), "http://example.org/mirrored");

    // The short form: the template is the pattern itself.
    let short = parse_fixture("construct-where-short.rq");
    assert_eq!(
        short.template.as_ref().unwrap().patterns.len(),
        short.where_clause.patterns.len()
    );
}

// -- Tier two: recognized but kept as written ----------------------------------------------------------------------

#[test]
fn a_property_path_stays_an_edge_label_as_written_never_a_parsed_structure() {
    let model = parse_fixture("constructs.rq");
    let paths: Vec<_> = model
        .where_clause
        .patterns
        .iter()
        .filter(|p| matches!(p.predicate, SparqlTerm::Path { .. }))
        .collect();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].predicate.as_written(), "foaf:knows+");
}

#[test]
fn expressions_arrive_as_source_text_with_their_variable_references_never_as_trees() {
    let model = parse_fixture("groups.rq");
    let root = &model.where_clause;

    let optional = root.children.iter().find(|c| c.kind == GroupScopeKind::Optional).unwrap();
    assert_eq!(optional.constraints.len(), 1);
    let filter = &optional.constraints[0];
    assert_eq!(filter.kind, SparqlConstraintKind::Filter);
    assert_eq!(filter.text, "FILTER(?x > 3)");
    assert_eq!(filter.referenced_variables, ["x"]);

    let of_kind = |kind| {
        let found: Vec<_> = root.constraints.iter().filter(|c| c.kind == kind).collect();
        assert_eq!(found.len(), 1, "{kind:?}");
        found[0]
    };
    assert_eq!(
        of_kind(SparqlConstraintKind::Filter).text,
        "FILTER NOT EXISTS { ?s ex:hidden true }"
    );
    let bind = of_kind(SparqlConstraintKind::Bind);
    assert_eq!(bind.text, "BIND(?x * 2 AS ?doubled)");
    assert_eq!(bind.defined_variable, "doubled");
    assert_eq!(bind.referenced_variables, ["x"]);
    let values = of_kind(SparqlConstraintKind::Values);
    assert_eq!(values.text, "VALUES ?x { 1 2 3 }");
    assert_eq!(values.referenced_variables, ["x"]);
}

#[test]
fn a_subquery_collapses_to_its_projection() {
    let model = parse_fixture("groups.rq");
    assert_eq!(model.where_clause.sub_selects.len(), 1);
    let sub_select = &model.where_clause.sub_selects[0];
    assert_eq!(sub_select.projection_text, "SELECT ?s (MAX(?v) AS ?best)");
    assert_eq!(sub_select.projected_names, ["s", "best"]);
    assert!(!sub_select.projects_all);
    assert!(sub_select.text.starts_with('{'));
    assert!(sub_select.text.ends_with('}'));
}

// -- The variable index ----------------------------------------------------------------------------------------------

#[test]
fn the_variable_index_counts_joins_and_tracks_scopes_and_projection() {
    let model = parse_fixture("groups.rq");
    let x = model.variable("x").unwrap();
    // ?x appears in five triple patterns: the root, one OPTIONAL, and all three UNION branches.
    assert_eq!(x.pattern_occurrences, 5);
    assert!(x.projected);
    assert_eq!(x.scope_paths[0], "where");
    for path in [
        "where/optional.0",
        "where/union.0/branch.0",
        "where/union.0/branch.1",
        "where/union.0/branch.2",
    ] {
        assert!(x.scope_paths.iter().any(|p| p == path), "{path}");
    }

    let y = model.variable("y").unwrap();
    assert!(!y.projected);
    assert_eq!(y.scope_paths, ["where/optional.1"]);

    assert_eq!(
        model.variable("doubled").unwrap().defining_expression,
        "BIND(?x * 2 AS ?doubled)"
    );
    assert!(model.variable("best").unwrap().scope_paths.iter().any(|p| p == "where"));
}

#[test]
fn select_star_resolves_to_the_in_scope_names() {
    let model =
        parser::parse("PREFIX ex: <http://example.org/> SELECT * WHERE { ?a ex:p ?b . OPTIONAL { ?a ex:q ?c } }")
            .unwrap();
    assert!(model.projects_all);
    assert_eq!(names(model.projection.iter().map(|i| i.name.clone())), ["a", "b", "c"]);
    assert!(model.variable("c").unwrap().projected);
}

// -- Line endings ---------------------------------------------------------------------------------------------------

#[test]
fn line_ending_variants_parse_to_the_same_query() {
    for name in ["crlf-line-endings.rq", "lf-line-endings.rq", "no-trailing-newline.rq"] {
        let model = parse_fixture(name);
        assert_eq!(model.form, SparqlQueryForm::Select);
        assert_eq!(model.where_clause.patterns.len(), 1, "{name}");
        assert_eq!(iri_of(&model.where_clause.patterns[0].predicate), "http://example.org/p");
        assert_eq!(names(model.projection.iter().map(|i| i.name.clone())), ["s"]);
    }
}

// -- Tier three: refused by name --------------------------------------------------------------------------------------

#[test]
fn a_sparql_update_document_is_refused_naming_sparql_update() {
    let error = parser::parse(&text("update-document.rq")).unwrap_err();
    assert!(error.message.contains("SPARQL Update"));
    assert!(error.message.contains("INSERT"));
    assert_eq!(error.line, 2);
}

#[test]
fn a_file_that_does_not_parse_names_its_line_and_reason() {
    let error = parser::parse(&text("broken.rq")).unwrap_err();
    assert_eq!(error.line, 4);
    assert!(error.message.contains("undeclared"));
}

// -- The terminals SPARQL shares with Turtle --------------------------------------------------------------------------

#[test]
fn a_carriage_return_inside_a_short_string_is_refused_naming_its_line() {
    let error = parser::parse("SELECT * WHERE {\r\n  ?s ?p \"a\rb\" .\r\n}\r\n").unwrap_err();
    assert_eq!(error.line, 2);
    assert!(error.message.contains("not closed on its line"));
}

#[test]
fn a_number_with_an_exponent_marker_and_no_digits_is_one_double() {
    let model = parser::parse("SELECT * WHERE { ?s ?p 5e }").unwrap();
    assert_eq!(model.where_clause.patterns.len(), 1);
    assert_eq!(
        model.where_clause.patterns[0].object,
        SparqlTerm::Literal {
            lexical: "5e".into(),
            datatype: XSD_DOUBLE.into(),
            language: String::new(),
            written: "5e".into()
        }
    );
}

#[test]
fn an_escape_sparql_does_not_know_is_refused_naming_its_line() {
    for (literal, message) in [
        ("\"a\\qb\"", "'\\q' is not a string escape"),
        ("\"\\u12zz\"", "'\\u12zz' is not an escape"),
    ] {
        let error = parser::parse(&format!("SELECT * WHERE {{\n  ?s ?p {literal} .\n}}\n")).unwrap_err();
        assert_eq!(error.line, 2);
        assert!(error.message.starts_with(message), "{}", error.message);
    }
}

#[test]
fn a_prefixed_name_the_grammar_allows_is_one_name() {
    for (name, iri) in [
        ("ex:a.b", "http://example.org/a.b"),
        ("ex:a\\.b", "http://example.org/a.b"),
        ("ex:a%20b", "http://example.org/a%20b"),
        (":a.b", "http://example.org/default/a.b"),
    ] {
        let model = parser::parse(&format!(
            "PREFIX ex: <http://example.org/> PREFIX : <http://example.org/default/> SELECT * WHERE {{ ?s ?p {name} . }}"
        ))
        .unwrap();
        assert_eq!(model.where_clause.patterns.len(), 1);
        assert_eq!(model.where_clause.patterns[0].object, SparqlTerm::iri(iri, name));
    }
}

#[test]
fn a_blank_node_label_with_a_dot_inside_is_one_label() {
    let model = parser::parse("SELECT * WHERE { _:a.b ?p ?o . }").unwrap();
    assert_eq!(model.where_clause.patterns.len(), 1);
    assert!(matches!(&model.where_clause.patterns[0].subject, SparqlTerm::Anonymous { label, .. } if label == "_:a.b"));
}

#[test]
fn an_unclosed_group_names_the_opening_line() {
    let error = parser::parse("SELECT * WHERE {\n  ?s ?p ?o .\n").unwrap_err();
    assert!(error.message.contains("never closed"));
}

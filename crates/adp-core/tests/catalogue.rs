//! The tool-type catalogue (etalii.adp spec 030, FR-021 and FR-035, task T048): the standalone host's tool types in
//! its order and states, joined with the vendored specifications.

use std::path::PathBuf;

use adp_core::Catalogue;
use adp_core::catalogue::{FindingSeverity, ToolKind, ToolState};

fn vendor() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor")
}

#[test]
fn the_catalogue_lists_every_tool_type_of_the_standalone_host_in_its_groups() {
    let catalogue = Catalogue::standalone();
    assert_eq!(catalogue.types.len(), 101);
    let identity = catalogue.types.iter().filter(|t| !t.state.has_module()).count();
    assert_eq!(
        identity, 66,
        "the tool types the standalone host lists without a module"
    );
    let groups = catalogue.groups();
    assert_eq!(groups.first().map(|g| g.0), Some("1. UML — Unified Modeling Language"));
    assert_eq!(groups.last().map(|g| g.0), Some("16. Editors"));
    let mindmap = catalogue.by_origin("freeplane/mindmap").unwrap();
    assert_eq!(
        (mindmap.display_name.as_str(), mindmap.kind),
        ("Mind map", ToolKind::Diagram)
    );
    assert_eq!(catalogue.by_origin("uml/class").unwrap().state, ToolState::Identified);
    assert_eq!(
        catalogue.by_origin("editor/json").unwrap().state,
        ToolState::Implemented
    );
}

#[test]
fn every_vendored_specification_joins_the_tool_type_of_its_origin() {
    let catalogue = Catalogue::load(&vendor());
    let specified: Vec<&str> = catalogue
        .types
        .iter()
        .filter(|t| t.specification.is_some())
        .map(|t| t.origin.as_str())
        .collect();
    assert_eq!(specified.len(), 30, "{specified:?}");
    for origin in [
        "w3c/owl",
        "etalii/sankey",
        "wardley/map",
        "etalii/knowledge",
        "c4/container",
    ] {
        assert!(specified.contains(&origin), "{origin} has its specification");
    }
    assert_eq!(
        catalogue.unlisted.iter().map(|u| u.origin.as_str()).collect::<Vec<_>>(),
        ["etalii/hierarchical-requirement"],
        "a specification the standalone host does not list is kept apart"
    );
}

#[test]
fn a_tool_type_claims_the_extensions_of_its_specification_and_bindings() {
    let catalogue = Catalogue::load(&vendor());
    let mindmap = catalogue.by_origin("freeplane/mindmap").unwrap();
    assert!(
        mindmap.extensions.contains(&".mm".to_owned()),
        "{:?}",
        mindmap.extensions
    );
    assert_eq!(
        mindmap.binding_files.len(),
        1,
        "the binding vendored from the standalone host"
    );
    let knowledge = catalogue.by_origin("etalii/knowledge").unwrap();
    assert_eq!(knowledge.bindings.len(), 3);
    // Since etalii.adp 4aa0522 the RDF graph is bound by the standalone host's declarative Turtle binding, which the
    // standalone host vendored here at 8b6ccfc does not have yet: the desktop host's RDF module reads Turtle itself.
    let rdf = catalogue.by_origin("w3c/rdf").unwrap();
    assert_eq!(
        rdf.bindings,
        [
            "https://raw.githubusercontent.com/etalii-adp/etalii.adp.ide.standalone/develop/src/diagrams/rdf/backend/EtAlii.Adp.Diagram.Rdf/w3c-turtle.fbl#turtle"
        ]
    );
    assert!(rdf.extensions.contains(&".ttl".to_owned()));
    let opening: Vec<&str> = catalogue
        .for_file("ideas.mm")
        .iter()
        .map(|t| t.origin.as_str())
        .collect();
    assert_eq!(opening, ["freeplane/mindmap"]);
}

/// The vendored diagram specifications that do not load here yet, each for a DISL library function or a variable the CEL
/// evaluator ported from the standalone host does not have. The standalone host reads these tool types with its modules'
/// own code; their modules are ported in phase 7. A specification that starts or stops loading changes this list: the
/// Azure DevOps pipeline, the Databricks job, the functional decomposition graph and the OWL ontology load since the
/// graph functions `successors`, `predecessors`, `reachable` and `inCycle` (DISL section 12.4) are `adp-disl`'s; the
/// C4 views since a type map holds names only (section 11.2); and the RDF graph and SKOS thesaurus since budgets
/// (section 3.2.1) and `diagram.knots` are; and the Sankey diagram since etalii.adp `aceb229` declares its functions
/// before the ones that call them (etalii.adp spec 036, research R2).
const NOT_LOADING: [&str; 1] = ["wardley/map"];

#[test]
fn the_vendored_specifications_that_do_not_load_are_listed_with_their_errors() {
    let catalogue = Catalogue::load(&vendor());
    let mut refused: Vec<&str> = catalogue
        .types
        .iter()
        .filter(|t| t.specification.is_some() && !t.is_usable())
        .map(|t| t.origin.as_str())
        .collect();
    refused.sort_unstable();
    let mut expected = NOT_LOADING.to_vec();
    expected.sort_unstable();
    assert_eq!(refused, expected);
    for origin in [
        "freeplane/mindmap",
        "generic/timeline",
        "generic/dependencies",
        "c4/container",
        "w3c/rdf",
        "etalii/sankey",
    ] {
        let tool_type = catalogue.by_origin(origin).unwrap();
        assert!(tool_type.is_usable(), "{origin}: {:?}", tool_type.findings);
    }
    let wardley = catalogue.by_origin("wardley/map").unwrap();
    assert!(
        wardley.findings.iter().any(|f| f.severity == FindingSeverity::Error),
        "{:?}",
        wardley.findings
    );
}

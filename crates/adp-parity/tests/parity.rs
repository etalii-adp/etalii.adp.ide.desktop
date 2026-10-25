//! The parity check (etalii.adp spec 030, SC-004, task T050; contracts/parity.md, "The check"): one test per edit script
//! under `parity/scripts/`, which replays it headless in the desktop core and compares every file its golden lists byte
//! for byte with what the desktop host wrote.
//!
//! A script whose tool type cannot open yet in the desktop host, or whose bytes do not match yet, stays a test, ignored
//! with the concrete reason; `parity/README.md` lists each script's state. `cargo test -p adp-parity -- --ignored`
//! replays them all and prints, per script, every step that went otherwise than in the standalone host and the first
//! differing offset of each file with both lines.

use std::fs;

use adp_parity::{check, repository_root, script_names};

#[test]
fn ansible_structure_infrastructure() {
    check("ansible/structure/infrastructure");
}

#[test]
fn c4_container_courier() {
    check("c4/container/courier");
}

#[test]
fn databricks_job_nightly_ingest() {
    check("databricks/job/nightly-ingest");
}

#[test]
fn databricks_bundle_lakehouse() {
    check("databricks/bundle/lakehouse");
}

#[test]
fn databricks_pipeline_lakehouse() {
    check("databricks/pipeline/lakehouse");
}

#[test]
fn databricks_pipeline_settings() {
    check("databricks/pipeline/settings");
}

#[test]
fn dotnet_dependency_graph_pipeline_toolkit() {
    check("dotnet/dependency-graph/pipeline-toolkit");
}

#[test]
fn etalii_agent_activity_diagram_two_projects() {
    check("etalii/agent-activity-diagram/two-projects");
}

#[test]
fn etalii_agent_behavior_modelling_research_assistant() {
    check("etalii/agent-behavior-modelling/research-assistant");
}

#[test]
fn etalii_hierarchical_requirement_collapse() {
    check("etalii/hierarchical-requirement/collapse");
}

#[test]
fn etalii_hierarchical_requirement_structure() {
    check("etalii/hierarchical-requirement/structure");
}

#[test]
fn etalii_hierarchical_requirement_structure_lf() {
    check("etalii/hierarchical-requirement/structure-lf");
}

#[test]
fn etalii_hierarchical_requirement_text() {
    check("etalii/hierarchical-requirement/text");
}

#[test]
fn etalii_hierarchical_requirement_two_writers() {
    check("etalii/hierarchical-requirement/two-writers");
}

#[test]
fn etalii_hierarchical_requirement_assessments() {
    check("etalii/hierarchical-requirement/assessments");
}

#[test]
fn etalii_hierarchical_requirement_tags() {
    check("etalii/hierarchical-requirement/tags");
}

#[test]
fn etalii_hierarchical_requirement_classification() {
    check("etalii/hierarchical-requirement/classification");
}

#[test]
fn etalii_hierarchical_requirement_filters() {
    check("etalii/hierarchical-requirement/filters");
}

#[test]
fn etalii_hierarchical_requirement_references() {
    check("etalii/hierarchical-requirement/references");
}

#[test]
fn etalii_hierarchical_requirement_columns() {
    check("etalii/hierarchical-requirement/columns");
}

#[test]
fn etalii_hierarchical_requirement_new_document() {
    check("etalii/hierarchical-requirement/new-document");
}

#[test]
fn etalii_knowledge_cities_json() {
    check("etalii/knowledge/cities-json");
}

#[test]
fn etalii_knowledge_cities_xml() {
    check("etalii/knowledge/cities-xml");
}

#[test]
fn etalii_knowledge_cities_yaml() {
    check("etalii/knowledge/cities-yaml");
}

#[test]
fn azure_devops_pipeline_multi_stage() {
    check("azure-devops/pipeline/multi-stage");
}

#[test]
fn etalii_supply_chain_automotive() {
    check("etalii/supply-chain/automotive");
}

#[test]
fn etalii_sankey_brightwater_coffee() {
    check("etalii/sankey/brightwater-coffee");
}

#[test]
fn freeplane_mindmap_example_1() {
    check("freeplane/mindmap/example-1");
}

#[test]
fn gartner_hypecycle_graph_coal_technologies() {
    check("gartner/hypecycle-graph/coal-technologies");
}

#[test]
fn helm_chart_nginx() {
    check("helm/chart/nginx");
}

#[test]
fn generic_timeline_add() {
    check("generic/timeline/add");
}

#[test]
fn generic_dependencies_services() {
    check("generic/dependencies/services");
}

#[test]
fn generic_timeline_roadmap() {
    check("generic/timeline/roadmap");
}

#[test]
fn etalii_functional_decomposition_graph_field_service() {
    check("etalii/functional-decomposition-graph/field-service");
}

#[test]
fn systems_causal_loop_diagram_on_call() {
    check("systems/causal-loop-diagram/on-call");
}

#[test]
fn systems_causal_loop_diagram_reference() {
    check("systems/causal-loop-diagram/reference");
}

#[test]
fn wardley_map_strategy_vocabulary() {
    check("wardley/map/strategy-vocabulary");
}

#[test]
fn w3c_sparql_groups() {
    check("w3c/sparql/groups");
}

#[test]
fn w3c_sparql_proteome_location_of_gene() {
    check("w3c/sparql/proteome-location-of-gene");
}

/// Every script has its test here and its row in `parity/README.md`, so none is skipped silently.
#[test]
fn every_script_is_checked_and_listed() {
    let tests = include_str!("parity.rs");
    let readme = fs::read_to_string(repository_root().join("parity/README.md")).expect("parity/README.md");
    for name in script_names() {
        assert!(
            tests.contains(&format!("check(\"{name}\")")),
            "{name} has no test in crates/adp-parity/tests/parity.rs"
        );
        assert!(
            readme.contains(&format!("`{name}`")),
            "{name} has no row in parity/README.md"
        );
    }
}

#[test]
fn w3c_rdf_nobel_1903() {
    check("w3c/rdf/nobel-1903");
}

#[test]
fn w3c_owl_prov_o() {
    check("w3c/owl/prov-o");
}

#[test]
fn w3c_skos_geographic_names() {
    check("w3c/skos/geographic-names");
}

#[test]
fn w3c_shacl_navigation_shapes() {
    check("w3c/shacl/navigation-shapes");
}

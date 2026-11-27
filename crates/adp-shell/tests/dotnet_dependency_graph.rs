//! The .NET dependency graph description against its case, recorded from the module (etalii.adp spec 035, T132), on
//! the shell's host, whose module contributes `reader.msbuildSolution` and `analysis.msbuildImports`: the solution read
//! with the project files the case holds beside it, handed to the reader as its input's `files`, and a drag written to
//! the registration of the read-only diagram. Every step passes.

use std::path::Path;

use adp_atd_check::{FsFiles, check};
use adp_atd_validate::{read_case, validate_description};

#[test]
fn the_dotnet_dependency_graph_case_passes() {
    let diagrams = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/etalii.adp/definitions/diagrams");
    let checked = check(&FsFiles, &diagrams.join("dotnet-dependency-graph.atd"));
    let case = read_case(&diagrams.join("conformance/dotnet-dependency-graph/pipeline-toolkit")).unwrap();
    let results = validate_description(&checked, &adp_shell::runtime_host(), std::slice::from_ref(&case)).unwrap();
    let differences: Vec<String> = results[0]
        .differences
        .iter()
        .map(|d| format!("{:?} {}: {} | {}", d.step, d.part, d.expected, d.actual))
        .collect();
    assert!(differences.is_empty(), "{differences:#?}");
}

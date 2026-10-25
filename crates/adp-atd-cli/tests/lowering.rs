//! What `atd validate` loads: a description's data types lower onto DISL's `metamodel.dataTypes` (DISL §4.4), so that
//! an attribute typed by one loads (etalii.adp spec 035, found with the Databricks bundle's `Variable`, T077).

use adp_atd_check::{FsFiles, check};
use adp_atd_runtime::lower_diagram;

#[test]
fn a_data_type_lowers_to_the_data_types_of_the_metamodel() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("variables.atd");
    std::fs::write(
        &path,
        "atd 0.1\n\ndiagram example/variables version \"0.1.0\" {\n  label \"Variables\"\n  model {\n    datatype Variable {\n      fields: { name: { type: \"string\", required: true } }\n    }\n    element Holder {\n      attribute variables: Variable many\n    }\n  }\n}\n",
    )
    .unwrap();
    let checked = check(&FsFiles, &path);
    let lowered = lower_diagram(&checked.description).unwrap();
    assert!(
        lowered.specification["metamodel"]["dataTypes"]["Variable"].is_object(),
        "{}",
        lowered.specification["metamodel"]
    );
    assert!(lowered.specification["metamodel"].get("datatypes").is_none());
}

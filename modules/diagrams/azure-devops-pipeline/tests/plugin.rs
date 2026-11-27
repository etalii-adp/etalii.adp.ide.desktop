//! `reader.azurePipelinesYaml` (etalii.adp spec 035, T132): the pipeline read into the draft binding's types, and a
//! model change planned with the module's own edit, as one splice.

use adp_cel::{Map, Value};
use adp_diagram_azure_devops_pipeline::plugin::PipelineYamlPlugin;
use adp_fbl::planning::ModelChange;
use adp_fbl::plugins::{PersistencePlugin, PluginFile, PluginPlanRequest, PluginPlanResult, PluginReadRequest};

const PIPELINE: &str = "stages:\n  - stage: Build\n    jobs:\n      - job: Compile\n        steps:\n          - script: make\n  - stage: Test\n    dependsOn: Build\n    jobs:\n      - job: Unit\n        steps:\n          - script: make test\n";

fn files() -> Vec<PluginFile> {
    vec![PluginFile::new("", PIPELINE.as_bytes().to_vec())]
}

#[test]
fn a_pipeline_reads_as_its_stages_jobs_steps_and_dependencies() {
    let read = PipelineYamlPlugin.read(&PluginReadRequest {
        files: files(),
        supplementary_entries: Vec::new(),
    });
    let ids: Vec<(&str, &str)> = read
        .elements
        .iter()
        .map(|e| (e.id.as_str(), e.type_name.as_str()))
        .collect();
    assert_eq!(
        ids,
        [
            ("pipeline", "Pipeline"),
            ("Build", "Stage"),
            ("Build/Compile", "Job"),
            ("Build/Compile/step-0", "Step"),
            ("Test", "Stage"),
            ("Test/Unit", "Job"),
            ("Test/Unit/step-0", "Step"),
            ("edge:Build->Test", "DependsOn"),
        ]
    );
    let edge = read.find("edge:Build->Test").unwrap();
    assert_eq!(
        (edge.source.as_deref(), edge.target.as_deref()),
        (Some("Build"), Some("Test"))
    );
}

#[test]
fn a_display_name_set_is_the_modules_rename() {
    let read = PipelineYamlPlugin.read(&PluginReadRequest {
        files: files(),
        supplementary_entries: Vec::new(),
    });
    let mut attributes = Map::default();
    attributes.insert("displayName", Value::from("  Build it  "));
    let change = ModelChange::Set {
        id: "Build".into(),
        attributes,
    };
    let PluginPlanResult::Planned(splices) = PipelineYamlPlugin.plan(&PluginPlanRequest {
        files: &files(),
        last: &read,
        change: &change,
    }) else {
        panic!("the rename is planned");
    };
    let mut text = PIPELINE.to_owned();
    for splice in splices.iter().rev() {
        text.replace_range(splice.splice.start..splice.splice.end, &splice.splice.text);
    }
    assert!(text.contains("  - stage: Build\n    displayName: Build it\n"), "{text}");
}

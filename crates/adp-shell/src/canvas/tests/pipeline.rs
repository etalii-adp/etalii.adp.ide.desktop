//! An Azure DevOps pipeline on the canvas with its module (etalii.adp spec 030, task T078): the standalone host's
//! `multi-stage` example, opened through its registration from the parity fixtures, drawn on the module's layered
//! layout, a stage opened on its jobs, a job switched off and given a step from the menu, a drag refused, and every
//! edit undone through the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;

use adp_scene::scene::Primitive;

use super::{Harness, texts_of};

/// The badges drawn, by what each says.
fn badges(harness: &Harness) -> Vec<String> {
    harness
        .tab
        .scene()
        .unwrap()
        .primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Shape(s) => s.tooltip.clone(),
            Primitive::Text(t) => t.tooltip.clone(),
            _ => None,
        })
        .collect()
}

pub(super) fn multi_stage() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../parity/fixtures/azure-devops/pipeline/example-1");
    Harness::open_from(
        &folder,
        &["multi-stage.adp", "multi-stage.yml"],
        "multi-stage.adp",
        "multi-stage.yml",
    )
}

#[test]
fn a_pipeline_opens_on_its_dependency_layout_and_is_opened_switched_off_and_added_to_on_the_canvas() {
    let mut harness = multi_stage();
    assert!(harness.tab.session.module_view().is_some());
    harness.settle();

    // The stages, closed, each with how many jobs it holds, in the columns of what they wait for.
    let texts = texts_of(harness.tab.scene().expect("a scene"));
    for label in [
        "Build the solution",
        "Test",
        "DeployStaging",
        "Notify the team",
        "1 job",
        "2 jobs",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }
    assert!(badges(&harness).contains(&"Manual trigger: this waits to be started by a person.".to_owned()));
    let build = harness.tab.bounds_of("Build").expect("Build is drawn");
    let test = harness.tab.bounds_of("Test").expect("Test is drawn");
    let staging = harness.tab.bounds_of("DeployStaging").expect("DeployStaging is drawn");
    let production = harness
        .tab
        .bounds_of("DeployProduction")
        .expect("DeployProduction is drawn");
    assert!(build.right() < test.x, "{build:?} {test:?}");
    assert_eq!((220.0, 88.0), (build.width, build.height));
    // Both deployments wait for Test, so they share its next column, in declared order.
    assert_eq!(staging.x, production.x);
    assert!(staging.bottom() < production.y);
    assert!(
        harness.tab.bounds_of("Test/Unit").is_none(),
        "a closed stage hides its jobs"
    );

    // Opening a stage shows its jobs inside it, and writes nothing.
    let original = harness.written();
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("Test".into()),
        "azure-pipeline.toggle-stage",
    );
    harness.tab.changed();
    harness.settle();
    assert_eq!(harness.written(), original);
    let opened = harness.tab.bounds_of("Test").unwrap();
    let unit = harness.tab.bounds_of("Test/Unit").expect("the stage's jobs are drawn");
    assert!(
        opened.x < unit.x && unit.right() < opened.right(),
        "{opened:?} {unit:?}"
    );
    assert!(harness.tab.bounds_of("Test/Integration").is_some());

    // A job switched off from its menu writes `enabled: false` under its declaration and wears the badge.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("Test/Integration".into()),
        "azure-pipeline.toggle-enabled",
    );
    assert!(
        harness
            .written()
            .contains("      - job: Integration\n        enabled: false\n        timeoutInMinutes: 30\n"),
        "{}",
        harness.written()
    );
    harness.tab.changed();
    harness.settle();
    assert!(badges(&harness).contains(&"Disabled: this will not run.".to_owned()));

    // A step added from the job's menu.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root),
        ActionTarget::Element("Test/Unit".into()),
        "azure-pipeline.add-step",
    );
    assert!(
        harness
            .written()
            .contains("            displayName: Unit tests\n          - script: echo Add your build steps here\n"),
        "{}",
        harness.written()
    );

    // A pipeline arranges itself: a drag is refused, with why.
    let (host, tab, root) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.move_by(host.workspace(root), &["Build".to_owned()], (40.0, 20.0));
    assert_eq!(
        Some("This diagram cannot be arranged by dragging."),
        harness.tab.notice.as_deref()
    );

    // Two undos take both edits back, byte for byte.
    for _ in 0..2 {
        assert!(harness.workspace().undo().is_done());
    }
    assert_eq!(harness.written(), original);
    assert_eq!(fs::read_to_string(&harness.body).unwrap(), harness.original);
    harness.tab.changed();
    harness.settle();

    harness
        .screenshot(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/azure-devops-pipeline.png"));
}

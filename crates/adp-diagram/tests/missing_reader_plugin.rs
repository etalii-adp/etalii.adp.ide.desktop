//! A body whose binding names a persistence plugin the host lacks (FBL sections 11.3 and 15.1; etalii.adp spec 030,
//! FR-031): it opens unreadable, with the finding that says which plugin is missing, and refuses every edit, rather
//! than opening as an empty diagram whose every edit is refused without a word. The mind map's binding
//! (`mindmap.fbl#mindmap`, read by `net.etalii.adp.freeplane.mm`) is the example; no module is added here.

use std::fs;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::tool::DiagramTools;
use adp_diagram::workspace::DiagramWorkspace;
use adp_diagram::{DiagramDelta, DiagramProblemSeverity};
use adp_disl::DislPluginFunctions;
use adp_disl::behavior::ids;

const MAP: &str = "<map version=\"freeplane 1.11.5\">\n<node TEXT=\"root\" ID=\"ID_1\">\n<node TEXT=\"leaf\" ID=\"ID_2\"/>\n</node>\n</map>\n";

#[test]
fn a_body_whose_reader_plugin_the_host_lacks_is_unreadable_with_a_finding() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("ideas.mm"), MAP).unwrap();
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor");
    let mut tools = DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none());
    let mut workspace = DiagramWorkspace::new(folder.path());

    let (opened, mut session) = workspace.open(&mut tools, Path::new("ideas.mm")).unwrap();
    assert_eq!(opened.origin, "freeplane/mindmap");
    let document = workspace.documents().get(&opened.body_path).unwrap();
    assert!(document.model().is_unreadable);
    let refusal = document.refusal().expect("an unreadable body is not edited");
    assert!(refusal.contains("could not be read"), "{refusal}");

    let baseline = session.baseline(workspace.documents()).unwrap();
    assert!(matches!(baseline.as_slice(), [DiagramDelta::Add(elements)] if elements.is_empty()));

    let problems = session.problems(workspace.documents());
    let missing = problems
        .iter()
        .find(|p| p.rule_id == "std.pluginMissing")
        .unwrap_or_else(|| panic!("no finding names the missing plugin: {problems:?}"));
    assert_eq!(missing.severity, DiagramProblemSeverity::Error);
    assert!(
        missing.message.contains("net.etalii.adp.freeplane.mm"),
        "{}",
        missing.message
    );

    let outcome = session.execute(
        &mut workspace,
        &ActionTarget::Element("ID_2".into()),
        "mindmap.delete",
        &ids::fixed(&[]),
    );
    assert!(matches!(outcome, ActionOutcome::Refused(_)), "{outcome:?}");
    assert_eq!(fs::read_to_string(folder.path().join("ideas.mm")).unwrap(), MAP);
}

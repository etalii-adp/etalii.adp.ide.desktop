//! The standalone module's three examples (`src/diagrams/causal-loop-diagram/examples/` at `8b6ccfc`, copied to
//! `tests/fixtures/examples/`): its `ExamplesTests`, with the same expectations, each example opened through its own
//! registration, its loops read as the specification derives them (`computed`, `disagrees`, `stated`) and judged by the
//! session.
//!
//! Not ported: `EveryExampleFolder_IsOneOfTheOnesThisTestKnowsAbout`, which guarded the standalone module's own folder;
//! these copies are this test's.

mod common;

use std::fs;
use std::path::Path;

use adp_core::Catalogue;
use adp_diagram::{DiagramProblem, DiagramSession, DiagramTools, DiagramWorkspace};
use adp_diagram_causal_loop::metrics::Metrics;
use adp_diagram_causal_loop::reading::Model;
use adp_diagram_causal_loop::self_organizing::{self, DEFAULT_BUDGET, SEPARATION_ROUNDS};
use adp_disl::{DislDiagram, DislPluginFunctions};
use adp_layout::Rect;

const EXAMPLES: [&str; 3] = ["reference", "on-call", "business-philosophy"];

struct Example {
    _folder: tempfile::TempDir,
    body: std::path::PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Example {
    fn diagram(&self) -> &DislDiagram {
        &self.workspace.documents().get(&self.body).unwrap().model().diagram
    }

    fn problems(&self) -> Vec<DiagramProblem> {
        self.session.problems(self.workspace.documents())
    }
}

fn load(name: &str) -> Example {
    let source = common::fixtures().join("examples").join(name);
    let folder = tempfile::tempdir().unwrap();
    for extension in ["adp", "cld"] {
        fs::copy(
            source.join(format!("{name}.{extension}")),
            folder.path().join(format!("{name}.{extension}")),
        )
        .unwrap();
    }
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    let mut tools = DiagramTools::new(Catalogue::load(&vendor), |_| DislPluginFunctions::none())
        .with_module(adp_diagram_causal_loop::ORIGIN, adp_diagram_causal_loop::module());
    let mut workspace = DiagramWorkspace::new(folder.path());
    let (_, mut session) = workspace.open(&mut tools, Path::new(&format!("{name}.adp"))).unwrap();
    session.baseline(workspace.documents()).unwrap();
    let example = Example {
        body: folder.path().join(format!("{name}.cld")),
        _folder: folder,
        workspace,
        session,
    };
    assert!(
        example
            .workspace
            .documents()
            .get(&example.body)
            .unwrap()
            .model()
            .unreadable
            .is_empty(),
        "{name} reads whole"
    );
    example
}

fn text(element: &adp_disl::DislElement, name: &str) -> String {
    adp_diagram_causal_loop::reading::text(element, name)
}

fn members(element: &adp_disl::DislElement) -> Vec<String> {
    match element.value_of("members") {
        adp_cel::Value::List(items) => items
            .iter()
            .map(|item| match adp_disl::DislElement::from_value(item) {
                Some(member) => text(&member, "name"),
                None => item.as_str().unwrap_or_default().to_owned(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

#[test]
fn an_example_sits_beside_its_own_registration() {
    for name in EXAMPLES {
        let folder = common::fixtures().join("examples").join(name);
        let adp = folder.join(format!("{name}.adp"));
        assert!(adp.exists(), "{name}");
        assert!(folder.join(format!("{name}.cld")).exists(), "{name}");
        assert!(
            fs::read_to_string(adp)
                .unwrap()
                .starts_with("systems/causal-loop-diagram")
        );
    }
}

#[test]
fn an_example_parses_and_every_link_and_loop_names_a_declared_variable() {
    for name in EXAMPLES {
        let example = load(name);
        let model = Model::of(example.diagram());
        let loops = example.diagram().nodes_of_type("Loop");
        assert!(
            !model.variables.is_empty() && !model.links.is_empty() && !loops.is_empty(),
            "{name}"
        );
        let declares = |n: &str| model.variables.iter().any(|v| v.name == n);
        for link in &model.links {
            assert!(
                declares(&link.from) && declares(&link.to),
                "{name}: {} -> {}",
                link.from,
                link.to
            );
        }
        for each in &loops {
            for member in members(each) {
                assert!(
                    declares(&member),
                    "{name}: loop {} runs through '{member}'",
                    text(each, "identifier")
                );
            }
        }
    }
}

#[test]
fn the_reference_example_carries_all_five_hard_cases_together() {
    let example = load("reference");
    let diagram = example.diagram();
    let loops = diagram.nodes_of_type("Loop");
    let computed: Vec<String> = loops.iter().map(|l| text(l, "computed")).collect();
    assert!(computed.iter().any(|c| c == "reinforcing"));
    assert!(computed.iter().any(|c| c == "balancing"));
    assert!(
        diagram
            .relations_of_type("CausalLink")
            .iter()
            .any(|l| l.value_of("delayed") == adp_cel::Value::Bool(true))
    );
    let model = Model::of(diagram);
    assert!(
        model
            .variables
            .iter()
            .any(|v| loops.iter().filter(|l| members(l).contains(&v.name)).count() > 1)
    );
    assert!(
        loops
            .iter()
            .any(|l| l.value_of("disagrees") == adp_cel::Value::Bool(true))
    );
}

#[test]
fn the_reference_example_disagrees_on_r3_specifically_and_the_validator_says_so() {
    let example = load("reference");
    let loops = example.diagram().nodes_of_type("Loop");
    let r3 = loops.iter().find(|l| text(l, "identifier") == "R3").unwrap();
    assert_eq!(text(r3, "computed"), "balancing");
    assert_eq!(r3.value_of("disagrees"), adp_cel::Value::Bool(true));
    let problems = example.problems();
    let disagreements: Vec<&DiagramProblem> = problems
        .iter()
        .filter(|p| p.rule_id == "causal-loop.label-disagrees")
        .collect();
    assert_eq!(disagreements.len(), 1, "{problems:?}");
    assert!(disagreements[0].message.contains("R3"));
}

#[test]
fn the_reference_example_ships_an_undecidable_loop() {
    let example = load("reference");
    let diagram = example.diagram();
    assert!(
        diagram
            .relations_of_type("CausalLink")
            .iter()
            .any(|l| text(l, "polarity") == "unstated" || text(l, "polarity").is_empty())
    );
    assert!(
        diagram
            .nodes_of_type("Loop")
            .iter()
            .any(|l| text(l, "computed") == "undecidable")
    );
}

#[test]
fn the_on_call_example_has_no_disagreement_at_all() {
    let example = load("on-call");
    let loops = example.diagram().nodes_of_type("Loop");
    assert!(!loops.is_empty());
    assert!(
        !example
            .problems()
            .iter()
            .any(|p| p.rule_id == "causal-loop.label-disagrees")
    );
    for each in &loops {
        let claims_reinforcing = text(each, "identifier").to_uppercase().starts_with('R');
        assert_eq!(
            claims_reinforcing,
            text(each, "computed") == "reinforcing",
            "{}",
            text(each, "identifier")
        );
    }
}

#[test]
fn the_business_philosophy_example_names_every_cycle_and_opens_with_no_finding() {
    let example = load("business-philosophy");
    assert_eq!(example.problems(), []);
    let loops = example.diagram().nodes_of_type("Loop");
    assert_eq!(loops.len(), 10);
    assert_eq!(loops.iter().filter(|l| text(l, "computed") == "reinforcing").count(), 8);
    assert_eq!(loops.iter().filter(|l| text(l, "computed") == "balancing").count(), 2);
    assert!(
        loops
            .iter()
            .all(|l| l.value_of("disagrees") == adp_cel::Value::Bool(false))
    );
}

#[test]
fn an_example_arranges_without_overlap() {
    for name in EXAMPLES {
        let example = load(name);
        let boxes: Vec<Rect> = self_organizing::compute(
            &Model::of(example.diagram()),
            &Metrics::default(),
            DEFAULT_BUDGET,
            SEPARATION_ROUNDS,
        )
        .unwrap()
        .into_values()
        .collect();
        assert!(!boxes.is_empty());
        for first in 0..boxes.len() {
            for second in first + 1..boxes.len() {
                assert!(!boxes[first].overlaps(&boxes[second]), "{name}");
            }
        }
    }
}

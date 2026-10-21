//! The ontology cases of the family providers (etalii.adp spec 030, task T083), after the standalone host's
//! `OwlProviders.Tests.cs` at `8b6ccfc`: the subclass gesture with its duplicate refusal, the four palette adds, the
//! punned individual role editing like a resource, the expression grid's uncapped Manchester form with the boundary
//! sentence, and the derived property edge's axiom rows. The cases that run the family's action and property
//! providers (`crate::actions`, `crate::properties`) are ported here and ignored until those providers land; the
//! ontology's own answers they rest on are tested directly.

use std::fs;
use std::path::Path;
use std::rc::Rc;

use adp_core::catalogue::Catalogue;
use adp_diagram::session::{ActionOutcome, ActionTarget};
use adp_diagram::{DiagramSession, DiagramTools, DiagramWorkspace, ModuleView};
use adp_diagram_rdf::lines::LineDocument;
use adp_diagram_rdf::model::RdfModel;
use adp_diagram_rdf::owl::selection;
use adp_diagram_rdf::parser;
use adp_diagram_rdf::vocabulary::{owl, rdf};
use adp_disl::DislPluginFunctions;

const NS: &str = "http://example.org/o#";

const CORPUS: &str = concat!(
    "@prefix : <http://example.org/o#> .\r\n",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\r\n",
    "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\r\n",
    "\r\n",
    "<http://example.org/o> a owl:Ontology .\r\n",
    "\r\n",
    ":Pizza a owl:Class , owl:NamedIndividual .\r\n",
    "\r\n",
    ":Topping a owl:Class .\r\n",
    "\r\n",
    ":Vegetarian a owl:Class ;\r\n",
    "    rdfs:subClassOf :Pizza ;\r\n",
    "    rdfs:subClassOf [ a owl:Restriction ; owl:onProperty :hasTopping ; owl:someValuesFrom :Topping ] .\r\n",
    "\r\n",
    ":hasTopping a owl:ObjectProperty , owl:FunctionalProperty ;\r\n",
    "    rdfs:domain :Pizza ;\r\n",
    "    rdfs:range :Topping .\r\n",
);

fn tools() -> DiagramTools {
    let vendor = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../vendor");
    adp_diagram_rdf::register(DiagramTools::new(Catalogue::load(&vendor), |_| {
        DislPluginFunctions::none()
    }))
}

struct Project {
    _folder: tempfile::TempDir,
    root: std::path::PathBuf,
    workspace: DiagramWorkspace,
    session: DiagramSession,
}

impl Project {
    /// `content` as `ontology.ttl`, registered as an ontology and opened.
    fn new(content: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let root = folder.path().to_path_buf();
        fs::write(root.join("ontology.ttl"), content).unwrap();
        fs::write(root.join("ontology.adp"), "w3c/owl\r\n").unwrap();
        let mut workspace = DiagramWorkspace::new(&root);
        let (_, session) = workspace.open(&mut tools(), Path::new("ontology.adp")).unwrap();
        Project {
            _folder: folder,
            root,
            workspace,
            session,
        }
    }

    fn view(&self) -> Rc<dyn ModuleView> {
        self.session.module_view().unwrap().clone()
    }

    fn body(&self) -> Vec<u8> {
        fs::read(self.root.join("ontology.ttl")).unwrap()
    }

    fn model(&self) -> RdfModel {
        parser::parse(&LineDocument::parse(&String::from_utf8(self.body()).unwrap())).unwrap()
    }

    fn action_ids(&self, target: &ActionTarget) -> Vec<String> {
        self.view()
            .actions(self.workspace.documents(), target)
            .into_iter()
            .flat_map(|g| g.actions)
            .map(|a| a.id)
            .collect()
    }
}

fn element(id: &str) -> ActionTarget {
    ActionTarget::Element(id.to_owned())
}

fn gesture(from: &str, to: &str) -> ActionTarget {
    ActionTarget::Connection {
        source: Some(from.to_owned()),
        target: Some(to.to_owned()),
    }
}

fn corpus_model() -> RdfModel {
    parser::parse(&LineDocument::parse(CORPUS)).unwrap()
}

#[test]
fn the_subclass_gesture_splices_one_triple_one_undo_away() {
    let mut project = Project::new(CORPUS);
    let before = project.body();
    let target = gesture(&format!("res:{NS}Topping"), &format!("res:{NS}Pizza"));
    let offered = project.action_ids(&target);
    assert!(offered.iter().any(|a| a == "owl.subclass"));
    assert!(offered.iter().any(|a| a == "rdf.connect"));
    let view = project.view();
    assert_eq!(
        view.execute(&mut project.workspace, &target, "owl.subclass"),
        ActionOutcome::Done
    );
    assert!(
        project
            .model()
            .triples
            .iter()
            .any(|t| t.subject.is_iri(&format!("{NS}Topping"))
                && t.predicate.iri == owl::SUB_CLASS_OF
                && t.object.is_iri(&format!("{NS}Pizza")))
    );
    project.workspace.undo();
    assert_eq!(project.body(), before);
}

#[test]
fn a_duplicate_subclass_gesture_is_refused_before_any_splice() {
    let mut project = Project::new(CORPUS);
    let before = project.body();
    let target = gesture(&format!("res:{NS}Vegetarian"), &format!("res:{NS}Pizza"));
    let view = project.view();
    assert_eq!(
        view.execute(&mut project.workspace, &target, "owl.subclass"),
        ActionOutcome::Refused(selection::DUPLICATE_SUBCLASS_REFUSAL.to_owned())
    );
    assert_eq!(project.body(), before);
}

#[test]
fn the_palette_adds_appear_on_marked_documents_and_write_declarations() {
    let mut project = Project::new(CORPUS);
    let placement = ActionTarget::Canvas { x: 10.0, y: 20.0 };
    let marked = project.action_ids(&placement);
    assert!(marked.iter().any(|a| a == "owl.add-class"));
    assert!(marked.iter().any(|a| a == "owl.add-individual"));
    let plain = Project::new("@prefix ex: <http://example.org/> .\r\n\r\nex:a ex:knows ex:b .\r\n");
    assert!(
        !plain
            .action_ids(&ActionTarget::Canvas { x: 1.0, y: 2.0 })
            .iter()
            .any(|a| a == "owl.add-class")
    );

    let before = project.body();
    let view = project.view();
    assert_eq!(
        view.answer(&mut project.workspace, &placement, "owl.add-class", ":Dessert"),
        ActionOutcome::Done
    );
    assert!(
        project
            .model()
            .triples
            .iter()
            .any(|t| t.subject.is_iri(&format!("{NS}Dessert"))
                && t.predicate.iri == rdf::TYPE
                && t.object.is_iri(owl::CLASS))
    );
    project.workspace.undo();
    assert_eq!(project.body(), before);
}

#[test]
fn a_punned_individual_role_edits_like_a_resource() {
    let mut project = Project::new(CORPUS);
    let id = format!("ind:{NS}Pizza");
    let offered = project.action_ids(&element(&id));
    assert!(offered.iter().any(|a| a == "rdf.rename-resource"));
    assert!(offered.iter().any(|a| a == "rdf.remove-resource"));
    let before = project.body();
    let view = project.view();
    view.set_property(&mut project.workspace, &[id.as_str()], "rdf.label", "The pizza")
        .unwrap();
    assert!(String::from_utf8(project.body()).unwrap().contains("The pizza"));
    project.workspace.undo();
    assert_eq!(project.body(), before);
}

#[test]
fn the_punned_individual_role_names_its_iri() {
    assert_eq!(
        selection::individual_iri_of(&corpus_model(), &format!("ind:{NS}Pizza")),
        Some(format!("{NS}Pizza"))
    );
    assert_eq!(
        selection::individual_iri_of(&corpus_model(), &format!("ind:{NS}Nowhere")),
        None
    );
}

#[test]
fn an_expression_selection_shows_the_uncapped_form_read_only_with_the_boundary_sentence() {
    let project = Project::new(CORPUS);
    let id = format!("expr:{NS}Vegetarian|{}|1", owl::SUB_CLASS_OF);
    let rows = project.view().properties(project.workspace.documents(), &id);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, "∃ :hasTopping.:Topping");
    assert_eq!(rows[0].read_only_reason, selection::EXPRESSION_REFUSAL);
    assert!(project.action_ids(&element(&id)).is_empty());
}

#[test]
fn an_expression_id_renders_its_uncapped_form() {
    let id = format!("expr:{NS}Vegetarian|{}|1", owl::SUB_CLASS_OF);
    assert_eq!(
        selection::expression_text(&corpus_model(), &id).as_deref(),
        Some("∃ :hasTopping.:Topping")
    );
    assert_eq!(
        selection::describe(&corpus_model(), &id).as_deref(),
        Some("∃ :hasTopping.:Topping")
    );
}

#[test]
fn a_class_selection_carries_its_axiom_rows() {
    let project = Project::new(CORPUS);
    let rows = project
        .view()
        .properties(project.workspace.documents(), &format!("res:{NS}Vegetarian"));
    assert!(rows.iter().any(|r| r.label == "Subclass of" && r.value == ":Pizza"));
    assert!(rows.iter().any(|r| r.label == "Subclass of"
        && r.value == "∃ :hasTopping.:Topping"
        && r.read_only_reason == selection::EXPRESSION_REFUSAL));
}

#[test]
fn a_derived_property_edge_shows_the_propertys_axioms() {
    let project = Project::new(CORPUS);
    let edge_id = format!("edge:res:{NS}Pizza|{NS}hasTopping|res:{NS}Topping");
    let rows = project.view().properties(project.workspace.documents(), &edge_id);
    assert!(rows.iter().any(|r| r.label == "Domain" && r.value == ":Pizza"));
    assert!(rows.iter().any(|r| r.label == "Range" && r.value == ":Topping"));
    assert!(
        rows.iter()
            .any(|r| r.label == "Characteristic" && r.value == "functional")
    );
}

#[test]
fn a_derived_property_edge_names_its_property() {
    let edge_id = format!("edge:res:{NS}Pizza|{NS}hasTopping|res:{NS}Topping");
    assert_eq!(
        selection::property_edge_iri_of(&corpus_model(), &edge_id),
        Some(format!("{NS}hasTopping"))
    );
    assert_eq!(
        selection::property_edge_iri_of(
            &corpus_model(),
            &format!("edge:res:{NS}Pizza|{NS}nothing|res:{NS}Topping")
        ),
        None
    );
    assert_eq!(
        selection::describe(&corpus_model(), &edge_id).as_deref(),
        Some(":Pizza → :Topping")
    );
}

#[test]
fn classes_and_the_ontology_marker_are_recognised() {
    let model = corpus_model();
    assert!(selection::is_class(&model, &format!("{NS}Pizza")));
    assert!(selection::is_class(&model, &format!("{NS}Topping")));
    assert!(!selection::is_class(&model, &format!("{NS}hasTopping")));
    assert!(selection::is_ontology_document(&model));
    assert!(!selection::is_ontology_document(
        &parser::parse(&LineDocument::parse(
            "@prefix ex: <http://example.org/> .\nex:a ex:knows ex:b .\n"
        ))
        .unwrap()
    ));
}

#[test]
fn the_owl_toolbox_offers_the_four_entries_and_no_restriction() {
    let project = Project::new(CORPUS);
    let toolbox = project.session.toolbox(project.workspace.documents());
    assert_eq!(toolbox.len(), 4);
    assert!(!toolbox.iter().any(|i| i.label.to_lowercase().contains("restriction")));
    assert!(toolbox.iter().any(|i| i.drop_action_id == "owl.add-class"));
    assert!(toolbox.iter().any(|i| i.drop_action_id == "owl.add-individual"));
}

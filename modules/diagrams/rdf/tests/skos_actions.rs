//! The thesaurus gestures and the concept grid (etalii.adp spec 030, task T083), ported from the standalone host's
//! `SkosActions.Tests.cs` at `8b6ccfc`. The standalone host's tests went through the family's provider trio; these
//! drive the scheme reading's own part of it ([`skos::actions`], [`skos::properties`]) and run the edits it answers
//! with as commands on the project's documents, undo included. That the family's generic `rdf.connect` stays beneath
//! the thesaurus gestures is the family providers' test.

use std::fs;
use std::path::PathBuf;

use adp_context::{ActionGroup, Prompt};
use adp_diagram::DiagramDocuments;
use adp_diagram_rdf::commands::{RdfCommand, RdfEdit};
use adp_diagram_rdf::family::{Run, Target};
use adp_diagram_rdf::reading::Reading;
use adp_diagram_rdf::skos::{self, SkosReading, actions, properties};
use adp_diagram_rdf::store::{self, RdfDocument};
use adp_diagram_rdf::vocabulary;
use adp_history::Command;

const VOCABULARY: &str = "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\r\n\
@prefix ex: <http://example.org/> .\r\n\
\r\n\
ex:scheme a skos:ConceptScheme ; skos:prefLabel \"Drinks\"@en ; skos:hasTopConcept ex:tea .\r\n\
ex:tea a skos:Concept ; skos:topConceptOf ex:scheme ; skos:prefLabel \"Tea\"@en .\r\n\
ex:milk a skos:Concept ; skos:inScheme ex:scheme ; skos:prefLabel \"Milk\"@en .\r\n";

const TEA: &str = "res:http://example.org/tea";
const MILK: &str = "res:http://example.org/milk";

struct Project {
    _folder: tempfile::TempDir,
    body: PathBuf,
    documents: DiagramDocuments,
}

impl Project {
    fn new(content: &str) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let body = folder.path().join("scheme.ttl");
        fs::write(&body, content).unwrap();
        let mut documents = DiagramDocuments::default();
        store::get_or_load(&mut documents, &body);
        Project {
            _folder: folder,
            body,
            documents,
        }
    }

    fn document(&self) -> RdfDocument {
        store::get(&self.documents, &self.body).unwrap().clone()
    }

    fn target<'a>(&'a self, element_id: &'a str) -> Target<'a> {
        Target {
            element_id,
            body_path: &self.body,
            origin: skos::ORIGIN,
        }
    }

    /// Runs `edit` as a command; its undo.
    fn run(&mut self, edit: RdfEdit) -> Box<dyn Command<DiagramDocuments>> {
        let applied = RdfCommand::new(&self.body, edit).apply(&mut self.documents).unwrap();
        applied.inverse.unwrap()
    }

    fn undo(&mut self, inverse: Box<dyn Command<DiagramDocuments>>) {
        inverse.apply(&mut self.documents).unwrap();
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(&self.body).unwrap()
    }

    fn text(&self) -> String {
        String::from_utf8(self.bytes()).unwrap()
    }
}

fn ids(groups: &[ActionGroup]) -> Vec<String> {
    groups.iter().flat_map(|g| &g.actions).map(|a| a.id.clone()).collect()
}

fn relation(from: &str, to: &str) -> String {
    format!("rel:{from}->{to}")
}

fn edit_of(run: Option<Run>) -> RdfEdit {
    match run {
        Some(Run::Edit(edit)) => edit,
        other => panic!("not an edit: {other:?}"),
    }
}

#[test]
fn a_relation_gesture_between_concepts_leads_with_the_thesaurus_gestures() {
    let project = Project::new(VOCABULARY);
    let gesture = relation(MILK, TEA);
    let found = ids(&actions::discover(&project.document(), &project.target(&gesture)));
    assert_eq!(vec![actions::FILE_UNDER, actions::RELATE], found);
}

#[test]
fn file_under_writes_one_broader_on_the_dragged_end_and_undo_restores_bytes() {
    let mut project = Project::new(VOCABULARY);
    let before = project.bytes();
    let gesture = relation(MILK, TEA);
    let edit = edit_of(actions::execute(
        &project.document(),
        &project.target(&gesture),
        actions::FILE_UNDER,
    ));
    let undo = project.run(edit);
    let text = project.text();
    assert!(text.contains("ex:milk"));
    assert!(text.contains("skos:broader ex:tea"));
    assert!(!text.contains("skos:narrower"));
    project.undo(undo);
    assert_eq!(before, project.bytes());
}

#[test]
fn file_under_on_an_lf_document_keeps_its_line_endings_and_undoes_byte_for_byte() {
    let mut project = Project::new(&VOCABULARY.replace("\r\n", "\n"));
    let before = project.bytes();
    assert!(!project.text().contains("\r\n"));
    let gesture = relation(MILK, TEA);
    let edit = edit_of(actions::execute(
        &project.document(),
        &project.target(&gesture),
        actions::FILE_UNDER,
    ));
    let undo = project.run(edit);
    let after = project.text();
    assert!(after.contains("skos:broader ex:tea"));
    assert!(!after.contains('\r'));
    project.undo(undo);
    assert_eq!(before, project.bytes());
}

#[test]
fn disconnect_takes_both_asserted_directions_as_one_undo() {
    let mut project = Project::new(&format!(
        "{VOCABULARY}ex:milk skos:broader ex:tea .\r\nex:tea skos:narrower ex:milk .\r\n"
    ));
    let before = project.bytes();
    let edge = format!("edge:{MILK}|{}|{TEA}", vocabulary::skos::BROADER);

    let groups = actions::discover(&project.document(), &project.target(&edge));
    let menu: Vec<_> = groups.iter().flat_map(|g| &g.actions).collect();
    assert_eq!(1, menu.len());
    assert!(menu[0].label.contains("both directions"));
    let asked = actions::execute(&project.document(), &project.target(&edge), actions::DISCONNECT);
    assert!(matches!(asked, Some(Run::Ask(Prompt::Confirm { .. }))));
    let edit = actions::command_for(&project.document(), &project.target(&edge), actions::DISCONNECT, "").unwrap();
    let undo = project.run(edit);

    let text = project.text();
    assert!(!text.contains("skos:broader ex:tea"));
    assert!(!text.contains("skos:narrower ex:milk"));
    project.undo(undo);
    assert_eq!(before, project.bytes());
}

#[test]
fn add_concept_is_three_triples_one_command_collision_refused_first() {
    let mut project = Project::new(VOCABULARY);
    let before = project.bytes();
    let placement = "new:120,80";

    assert!(
        ids(&actions::discover(&project.document(), &project.target(placement)))
            .contains(&actions::ADD_CONCEPT.to_owned())
    );
    assert!(matches!(
        actions::validate(&project.document(), actions::ADD_CONCEPT, "tea"),
        Some(Err(_))
    ));

    let edit = actions::command_for(
        &project.document(),
        &project.target(placement),
        actions::ADD_CONCEPT,
        "Green tea",
    )
    .unwrap();
    let undo = project.run(edit);
    let text = project.text();
    assert!(text.contains("ex:Green_tea"));
    assert!(text.contains("\"Green tea\"@en"));
    assert!(text.contains("skos:inScheme ex:scheme"));

    project.undo(undo);
    assert_eq!(before, project.bytes());
}

#[test]
fn the_concept_grid_lists_labels_by_language_and_a_pref_label_edit_keeps_its_tag() {
    let mut project = Project::new(&VOCABULARY.replace(
        "ex:tea a skos:Concept ; skos:topConceptOf ex:scheme ; skos:prefLabel \"Tea\"@en .",
        "ex:tea a skos:Concept ; skos:topConceptOf ex:scheme ; skos:prefLabel \"Tea\"@en ; skos:prefLabel \"Thee\"@nl ; \
         skos:definition \"An infusion.\"@en ; skos:exactMatch <http://elsewhere.org/tea> .",
    ));
    let rows = properties::describe(&project.document(), &project.target(TEA)).unwrap();
    assert!(
        rows.iter()
            .any(|r| r.id == "skos.label:prefLabel:en" && r.value == "Tea" && r.read_only_reason.is_empty())
    );
    assert!(
        rows.iter()
            .any(|r| r.id == "skos.label:prefLabel:nl" && r.value == "Thee")
    );
    assert!(
        rows.iter()
            .any(|r| r.id == "skos.doc:definition" && r.value == "An infusion.")
    );
    assert!(
        rows.iter()
            .any(|r| r.id == "skos.schemes" && !r.read_only_reason.is_empty())
    );
    assert!(
        rows.iter()
            .any(|r| r.id.starts_with("skos.mapping:") && !r.read_only_reason.is_empty())
    );

    let edit = properties::command_for(
        &project.document(),
        &project.target(TEA),
        "skos.label:prefLabel:nl",
        "Zwarte thee",
    )
    .unwrap();
    project.run(edit);
    let text = project.text();
    assert!(text.contains("\"Zwarte thee\"@nl"));
    assert!(text.contains("\"Tea\"@en"));
}

#[test]
fn an_xl_labeled_concept_refuses_label_edits_with_the_boundary_sentence() {
    let project = Project::new(&format!(
        "{VOCABULARY}@prefix skosxl: <http://www.w3.org/2008/05/skos-xl#> .\r\nex:tea skosxl:prefLabel ex:teaLabel .\r\n"
    ));
    let rows = properties::describe(&project.document(), &project.target(TEA)).unwrap();
    assert!(
        rows.iter()
            .any(|r| r.id == "skos.label:prefLabel:en" && r.read_only_reason.contains("SKOS-XL"))
    );
    assert!(
        properties::command_for(
            &project.document(),
            &project.target(TEA),
            "skos.label:prefLabel:en",
            "X"
        )
        .is_none()
    );
}

#[test]
fn the_toolbox_offers_a_concept_under_the_skos_origin() {
    let items = SkosReading::default().toolbox();
    assert_eq!(1, items.len());
    assert_eq!(actions::ADD_CONCEPT, items[0].drop_action_id);
}

#[test]
fn another_readings_action_is_not_the_scheme_readings() {
    let project = Project::new(VOCABULARY);
    let document = project.document();
    assert!(actions::execute(&document, &project.target(TEA), "rdf.rename-resource").is_none());
    assert!(actions::validate(&document, "rdf.add-resource", "x").is_none());
    assert!(actions::discover(&document, &project.target("x")).is_empty());
}

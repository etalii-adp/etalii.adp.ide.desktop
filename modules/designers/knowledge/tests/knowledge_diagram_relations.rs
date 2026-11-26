//! A knowledge relation whose target is a diagram (etalii.adp spec 092, FR-025; `knowledge.md`, "A relation to a
//! diagram"): the diagrams of the project are offered as a new relation's target, a relation to one names it by a link
//! and is one-way, its cells offer the diagram's named elements and hold links to them by their name property, and
//! *Create* adds an element to the diagram and relates it, as one step over both files that one undo takes back.
//!
//! What the diagrams are, and the body one has with an element added, are the host's: here a [`FixedDiagrams`] answers
//! for one timeline as the desktop host's diagram tools would (their own tests, in each diagram module, say what they
//! add and where).

mod common;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_designer_knowledge::KnowledgeSession;
use adp_designer_knowledge::context_actions::{self, Commit, ONE, RELATE};
use adp_designer_knowledge::diagrams::{
    CreatedElement, ELEMENT_KIND, ElementKind, KnowledgeDiagram, KnowledgeDiagrams, NEW_ELEMENT, NOTHING_TO_CREATE,
    ONE_WAY_ONLY, PICK_A_KIND,
};
use adp_designer_knowledge::links;
use adp_designer_knowledge::mapper::{CREATES, DIAGRAM};
use adp_desl::table::model::{Settings, TableGesture, TableOption};
use adp_links::{HeldLink, IndexedDocument, IndexedElement};
use common::{EditingTable, copy_plain, folder};

/// The timeline the cities relate to: its document id, and its body before anything is added to it.
const ROADMAP_ID: &str = "r6d3w8ne";
const ROADMAP: &str = "timeline: 1\nid: r6d3w8ne\nelements:\n  - id: launch\n    label: Launch\n    begin: 2027-01-04\n    row: 0\n  - id: beta\n    label: Beta\n    begin: 2027-02-01\n    row: 1\n";

/// What *Create* adds to the timeline: an element under this id, and the body that holds it.
const CREATED_ID: &str = "n3w0e1em";
const GROWN: &str = "timeline: 1\nid: r6d3w8ne\nelements:\n  - id: launch\n    label: Launch\n    begin: 2027-01-04\n    row: 0\n  - id: beta\n    label: Beta\n    begin: 2027-02-01\n    row: 1\n  - id: n3w0e1em\n    label: Retro\n    begin: 2026-11-26\n    end: 2026-12-03\n    row: 0\n";

/// One timeline the host knows of, read from its file, given the kinds it offers; every *Create* asked of it is kept.
struct FixedDiagrams {
    path: PathBuf,
    kinds: Vec<ElementKind>,
    asked: RefCell<Vec<(PathBuf, String, String)>>,
}

impl KnowledgeDiagrams for FixedDiagrams {
    /// The timeline as its file reads now: with the element *Create* added once that is written.
    fn diagrams(&self, _folder: &Path) -> Vec<KnowledgeDiagram> {
        let mut elements = vec![
            IndexedElement::new("launch", [("label".to_owned(), "Launch".to_owned())]),
            IndexedElement::new("beta", [("label".to_owned(), "Beta".to_owned())]),
        ];
        if std::fs::read_to_string(&self.path).is_ok_and(|text| text.contains(CREATED_ID)) {
            elements.push(IndexedElement::new(
                CREATED_ID,
                [("label".to_owned(), "Retro".to_owned())],
            ));
        }
        vec![KnowledgeDiagram {
            path: self.path.clone(),
            indexed: IndexedDocument {
                path: self.path.clone(),
                document_id: Some(ROADMAP_ID.into()),
                name_property: Some("label".into()),
                elements,
                links: Vec::new(),
            },
            kinds: self.kinds.clone(),
        }]
    }

    fn create(&self, path: &Path, kind: &str, name: &str) -> Result<CreatedElement, String> {
        self.asked
            .borrow_mut()
            .push((path.to_path_buf(), kind.to_owned(), name.to_owned()));
        Ok(CreatedElement {
            id: CREATED_ID.into(),
            body: GROWN.as_bytes().to_vec(),
        })
    }
}

/// The cities, a timeline beside them, and the cities open for editing with the host knowing of the timeline.
struct Project {
    _folder: tempfile::TempDir,
    root: PathBuf,
    roadmap: PathBuf,
    diagrams: Rc<FixedDiagrams>,
    table: EditingTable,
}

impl Project {
    fn new(kinds: Vec<ElementKind>) -> Project {
        let folder = folder();
        let root = folder.path().to_path_buf();
        let cities = copy_plain("cities.yaml", &root);
        let roadmap = root.join("roadmap.tml");
        std::fs::write(&roadmap, ROADMAP).unwrap();
        let diagrams = Rc::new(FixedDiagrams {
            path: roadmap.clone(),
            kinds,
            asked: RefCell::new(Vec::new()),
        });
        let session = KnowledgeSession::open_for_editing_with(&cities, &root, diagrams.clone());
        Project {
            _folder: folder,
            root,
            roadmap,
            diagrams,
            table: EditingTable::with_session(cities, session),
        }
    }

    fn timeline_kinds() -> Vec<ElementKind> {
        vec![
            ElementKind::new("Moment", "moment"),
            ElementKind::new("Period", "period"),
        ]
    }

    /// A relation `Milestones` to the timeline, holding any number of elements, made as the table makes one.
    fn related(kinds: Vec<ElementKind>) -> (Project, String) {
        let mut project = Project::new(kinds);
        project.table.edit(
            TableGesture::new("addRelation")
                .values(["Milestones"])
                .setting("target", "roadmap.tml"),
        );
        let id = project
            .table
            .on_disk()
            .columns
            .into_iter()
            .find(|p| p.name == "Milestones")
            .expect("the relation")
            .id;
        (project, id)
    }

    fn settings(&self, column: &str) -> Settings {
        self.table
            .baseline()
            .columns
            .into_iter()
            .find(|c| c.id == column)
            .expect("the column")
            .settings
    }

    fn options(&self, column: &str) -> Vec<TableOption> {
        self.table
            .baseline()
            .columns
            .into_iter()
            .find(|c| c.id == column)
            .expect("the column")
            .options
    }

    /// The links the cell of the first row of the relation holds, as written.
    fn written(&self, column: &str) -> String {
        let text = std::fs::read_to_string(&self.table.path).unwrap();
        let property = format!("property: {column}");
        let at = text
            .find(&property)
            .unwrap_or_else(|| panic!("no cell of {column}:\n{text}"));
        text[at..].lines().take(4).collect::<Vec<_>>().join("\n")
    }
}

// ---- the target ----

#[test]
fn a_relation_asked_for_without_a_target_offers_the_projects_diagrams_too() {
    let mut project = Project::new(Project::timeline_kinds());
    let (_, answer) = project
        .table
        .begin(TableGesture::new("addRelation").values(["Milestones"]));
    assert!(answer.starts_with("choose:"), "{answer}");
    assert!(answer.contains("\"roadmap.tml\""), "{answer}");
}

#[test]
fn a_relation_to_a_diagram_names_it_by_a_link_and_is_one_way_whatever_was_asked() {
    let mut project = Project::new(Project::timeline_kinds());
    project.table.edit(
        TableGesture::new("addRelation")
            .values(["Milestones"])
            .setting("target", "roadmap.tml")
            .setting("twoWay", "true")
            .setting("counterpart", "Cities"),
    );
    let relation = project
        .table
        .on_disk()
        .columns
        .into_iter()
        .find(|p| p.name == "Milestones")
        .expect("the relation");
    assert_eq!(relation.target_file, format!("${ROADMAP_ID}@roadmap.tml"));
    assert_eq!(
        relation.backlink_name, "",
        "a diagram has no column to show the other side in"
    );
    assert_eq!(relation.counterpart, "");
    assert_eq!(
        std::fs::read_to_string(&project.roadmap).unwrap(),
        ROADMAP,
        "the diagram is not written"
    );
    assert_eq!(
        project.settings(&relation.id).get(DIAGRAM).map(String::as_str),
        Some("true")
    );
}

#[test]
fn making_a_relation_to_a_diagram_two_way_is_refused_with_why() {
    let (mut project, relation) = Project::related(Project::timeline_kinds());
    let (_, answer) = project.table.begin(
        TableGesture::new("setRelationOtherSide")
            .column(&relation)
            .setting("twoWay", "true"),
    );
    assert_eq!(answer, ONE_WAY_ONLY);
}

// ---- its cells ----

#[test]
fn its_cells_offer_the_diagrams_named_elements_and_hold_links_to_them_by_name() {
    let (mut project, relation) = Project::related(Project::timeline_kinds());
    let offered: Vec<(String, String)> = project.options(&relation).into_iter().map(|o| (o.id, o.name)).collect();
    assert_eq!(
        offered,
        [
            ("launch".to_owned(), "Launch".to_owned()),
            ("beta".to_owned(), "Beta".to_owned())
        ]
    );
    project.table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(&relation)
            .values(["launch"]),
    );
    let written = project.written(&relation);
    assert!(
        written.contains(&format!("${ROADMAP_ID}@roadmap.tml#launch.label=Launch")),
        "{written}"
    );
    // The diagram's elements count the row among their backlinks, as every link's target does.
    let held = links::held_links(&project.table.on_disk());
    assert!(
        held.contains(&HeldLink::new("r1", format!("${ROADMAP_ID}@roadmap.tml#launch"))),
        "{held:?}"
    );
}

// ---- Create ----

#[test]
fn create_adds_the_element_to_the_diagram_and_relates_it_in_one_step_that_one_undo_takes_back() {
    let (mut project, relation) = Project::related(Project::timeline_kinds());
    assert_eq!(
        project.settings(&relation).get(CREATES).map(String::as_str),
        Some("Moment=moment,Period=period")
    );
    let cities_before = project.table.bytes();
    project.table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(&relation)
            .setting(NEW_ELEMENT, " Retro ")
            .setting(ELEMENT_KIND, "Period"),
    );
    assert_eq!(
        *project.diagrams.asked.borrow(),
        [(project.roadmap.clone(), "Period".to_owned(), "Retro".to_owned())]
    );
    assert_eq!(std::fs::read_to_string(&project.roadmap).unwrap(), GROWN);
    let written = project.written(&relation);
    assert!(
        written.contains(&format!("${ROADMAP_ID}@roadmap.tml#{CREATED_ID}.label=Retro")),
        "{written}"
    );
    let offered: Vec<String> = project.options(&relation).into_iter().map(|o| o.id).collect();
    assert_eq!(offered, ["launch", "beta", CREATED_ID], "offered at once");
    project.table.session.undo().expect("one undo");
    assert_eq!(std::fs::read_to_string(&project.roadmap).unwrap(), ROADMAP);
    assert_eq!(project.table.bytes(), cities_before);
    // The diagram is read again: the element the step added is gone from what is offered.
    let offered: Vec<String> = project.options(&relation).into_iter().map(|o| o.id).collect();
    assert_eq!(offered, ["launch", "beta"]);
}

#[test]
fn create_asks_which_kind_where_the_diagram_offers_several_and_takes_the_one_it_offers_alone() {
    let (mut project, relation) = Project::related(Project::timeline_kinds());
    let (_, answer) = project.table.begin(
        TableGesture::new("setCell")
            .row("r1")
            .column(&relation)
            .setting(NEW_ELEMENT, "Retro"),
    );
    assert_eq!(answer, PICK_A_KIND);
    assert!(project.diagrams.asked.borrow().is_empty());

    let (mut project, relation) = Project::related(vec![ElementKind::new("Node", "node")]);
    project.table.edit(
        TableGesture::new("setCell")
            .row("r1")
            .column(&relation)
            .setting(NEW_ELEMENT, "Retro"),
    );
    assert_eq!(project.diagrams.asked.borrow()[0].1, "Node");
}

#[test]
fn a_diagram_given_no_element_by_name_has_no_create() {
    let (mut project, relation) = Project::related(Vec::new());
    assert_eq!(project.settings(&relation).get(CREATES), None);
    let (_, answer) = project.table.begin(
        TableGesture::new("setCell")
            .row("r1")
            .column(&relation)
            .setting(NEW_ELEMENT, "Retro"),
    );
    assert_eq!(answer, NOTHING_TO_CREATE);
    assert_eq!(std::fs::read_to_string(&project.roadmap).unwrap(), ROADMAP);
}

// ---- the explorer's entry ----

#[test]
fn the_explorers_entry_offers_the_diagrams_and_asks_no_other_side_for_one() {
    let project = Project::new(Project::timeline_kinds());
    let root = project.root.as_path();
    let diagrams: &dyn KnowledgeDiagrams = project.diagrams.as_ref();
    let Ok(adp_context::Prompt::File { files, .. }) =
        context_actions::execute(root, &project.table.path, Some(diagrams))
    else {
        panic!("the file dialog");
    };
    assert!(files.iter().any(|f| f.id == "roadmap.tml"), "{files:?}");
    assert!(context_actions::validate(root, RELATE, "roadmap.tml", Some(diagrams)).is_ok());
    assert!(context_actions::validate(root, RELATE, "roadmap.tml", None).is_err());
    let Commit::Asks { next_action_id, .. } =
        context_actions::commit(root, &project.table.path, RELATE, "roadmap.tml", "", Some(diagrams))
    else {
        panic!("how many rows a cell holds is asked");
    };
    let Commit::Make { gesture, .. } =
        context_actions::commit(root, &project.table.path, &next_action_id, ONE, "", Some(diagrams))
    else {
        panic!("the relation is made without asking for its other side");
    };
    assert_eq!(gesture.setting_of("target"), "roadmap.tml");
    assert_eq!(gesture.setting_of("limit"), "one");
    assert_eq!(gesture.setting_of("counterpart"), "");
}

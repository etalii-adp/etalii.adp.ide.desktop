//! The add's tests: the tree the standalone host shows, the names it suggests and refuses, and the bytes it writes,
//! against the standalone host's sources at 8b6ccfc.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use adp_core::ToolState;

use super::*;

fn read(path: &Path) -> String {
    String::from_utf8(fs::read(path).unwrap()).unwrap()
}

fn folder_prompt(adds: &AddCatalogue, folder: &Path) -> AddPrompt {
    adds.execute(&AddTarget::Folder(folder.to_path_buf())).unwrap()
}

#[test]
fn the_tree_groups_the_standalone_hosts_tool_types_by_vendor_with_their_titles_and_states() {
    let folder = tempfile::tempdir().unwrap();
    let adds = AddCatalogue::standalone();
    let prompt = folder_prompt(&adds, folder.path());
    assert_eq!(prompt.title, "Add diagram");
    assert_eq!(prompt.confirm_label, "Add");
    assert_eq!(prompt.name_field.as_deref(), Some("Name"));

    let vendors: Vec<&str> = prompt.options.iter().map(|o| o.id.as_str()).collect();
    let mut sorted = vendors.clone();
    sorted.sort();
    assert_eq!(vendors, sorted, "the vendors are in ordinal order");
    assert!(prompt.options.iter().all(|o| !o.selectable && o.label == o.id));
    assert_eq!(vendors.first(), Some(&"ansible"));

    let c4 = prompt.options.iter().find(|o| o.id == "c4").unwrap();
    let labels: Vec<&str> = c4.children.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "C4 code diagram",
            "C4 component diagram",
            "C4 container diagram",
            "C4 deployment diagram",
            "C4 dynamic diagram",
            "C4 system context diagram",
            "C4 system landscape diagram"
        ]
    );
    let code = &c4.children[0];
    assert_eq!(code.state, Some(ToolState::Specified));
    assert_eq!(code.suggested_value, "code");
    assert_eq!(c4.children[1].state, Some(ToolState::Prototype));

    // An identity-only tool type is listed, choosable, with its state, as the standalone host lists it.
    let uml = prompt.options.iter().find(|o| o.id == "uml").unwrap();
    let class = uml.children.iter().find(|o| o.id == "uml/class").unwrap();
    assert!(class.selectable);
    assert_eq!(class.state, Some(ToolState::Identified));
    assert_eq!(class.icon, "mdi-cube-outline");
    assert_eq!(
        class.description,
        "Classes, their attributes and operations, and the associations between them."
    );

    // The designer is a group under its vendor, sorted by label among the diagrams, with a choice per format.
    let etalii = prompt.options.iter().find(|o| o.id == "etalii").unwrap();
    let labels: Vec<&str> = etalii.children.iter().map(|o| o.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Agent Behavior Modelling",
            "Agent activity diagram",
            "Functional decomposition graph",
            "Knowledge",
            "Sankey diagram",
            "Supply chain diagram"
        ]
    );
    let knowledge = &etalii.children[3];
    assert!(!knowledge.selectable);
    assert_eq!(knowledge.state, Some(ToolState::Prototype));
    let formats: Vec<(&str, &str)> = knowledge
        .children
        .iter()
        .map(|o| (o.id.as_str(), o.label.as_str()))
        .collect();
    assert_eq!(
        formats,
        [
            ("etalii/knowledge@.yaml", "YAML"),
            ("etalii/knowledge@.json", "JSON"),
            ("etalii/knowledge@.xml", "XML")
        ]
    );
    assert_eq!(knowledge.children[0].suggested_value, "knowledge");

    // The 78 diagram definitions and the designer's three formats; editors are not added here.
    fn leaves(options: &[AddOption]) -> usize {
        options
            .iter()
            .map(|o| usize::from(o.selectable) + leaves(&o.children))
            .sum()
    }
    assert_eq!(leaves(&prompt.options), 78 + 3);
    assert!(find(&prompt.options, "editor/json").is_none());
    assert!(
        find(&prompt.options, "airflow/dag").is_none(),
        "a tool type without a module is not offered"
    );
}

#[test]
fn a_suggested_name_is_free_and_a_folder_subject_takes_none() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join("timeline.adp"), "generic/timeline\r\n").unwrap();
    fs::create_dir(folder.path().join("timeline-2.adp")).unwrap();
    let adds = AddCatalogue::standalone();
    let prompt = folder_prompt(&adds, folder.path());
    assert_eq!(
        find(&prompt.options, "generic/timeline").unwrap().suggested_value,
        "timeline-3"
    );
    let helm = find(&prompt.options, "helm/chart").unwrap();
    assert_eq!(helm.suggested_value, "");
    assert_eq!(helm.name_suppressed_reason, FOLDER_SUBJECT_SUPPRESSION);
    assert_eq!(helm.unavailable_reason, "");
    assert_eq!(suggest("hypecycle-graph", folder.path()), "hypecycle-graph");
    assert_eq!(sanitise("a b.c//d"), "a-b-c-d");
    assert_eq!(sanitise("..."), "diagram");
    assert_eq!(with_extension(" plan.ADP "), "plan.adp");
}

#[test]
fn a_diagram_is_its_registration_and_its_body_byte_for_byte() {
    let folder = tempfile::tempdir().unwrap();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let adds = AddCatalogue::standalone();

    let created = adds.commit(&target, "generic/timeline", "plan").unwrap();
    assert_eq!(created, folder.path().join("plan.adp"));
    assert_eq!(read(&created), "generic/timeline\r\n");
    assert_eq!(read(&folder.path().join("plan.tml")), "timeline: 1\r\nelements: []\r\n");

    adds.commit(&target, "c4/container", "payments.adp").unwrap();
    assert_eq!(read(&folder.path().join("payments.adp")), "c4/container\r\n");
    assert_eq!(
        read(&folder.path().join("payments.dsl")),
        adp_diagram_c4::toolbox::empty_document(adp_diagram_c4::model::C4ViewKind::Container, "payments")
    );

    adds.commit(&target, "freeplane/mindmap", "ideas").unwrap();
    assert!(read(&folder.path().join("ideas.mm")).contains("TEXT=\"ideas\""));

    // An identity-only tool type, or one without a body file, is its registration alone.
    adds.commit(&target, "uml/class", "domain").unwrap();
    assert_eq!(read(&folder.path().join("domain.adp")), "uml/class\r\n");
    adds.commit(&target, "c4/code", "classes").unwrap();
    assert_eq!(read(&folder.path().join("classes.adp")), "c4/code\r\n");
    let names: HashSet<String> = fs::read_dir(folder.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        names,
        HashSet::from(
            [
                "plan.adp",
                "plan.tml",
                "payments.adp",
                "payments.dsl",
                "ideas.adp",
                "ideas.mm",
                "domain.adp",
                "classes.adp"
            ]
            .map(str::to_owned)
        )
    );
}

#[test]
fn a_designer_document_names_its_body_in_its_registration() {
    let folder = tempfile::tempdir().unwrap();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let adds = AddCatalogue::standalone();
    let created = adds.commit(&target, "etalii/knowledge@.json", "cities").unwrap();
    assert_eq!(read(&created), "etalii/knowledge\r\nbody: cities.json\r\n");
    assert!(read(&folder.path().join("cities.json")).starts_with('{'));
    assert_eq!(
        adds.commit(&target, "etalii/knowledge@.yaml", "cities"),
        Err("An item named 'cities.adp' already exists in this folder.".into())
    );
    fs::write(folder.path().join("towns.yaml"), "").unwrap();
    assert_eq!(
        adds.commit(&target, "etalii/knowledge@.yaml", "towns"),
        Err("An item named 'towns.yaml' already exists in this folder.".into())
    );
}

#[test]
fn a_folder_subject_registers_the_folder_once() {
    let folder = tempfile::tempdir().unwrap();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let adds = AddCatalogue::standalone();
    let created = adds.commit(&target, "helm/chart", "ignored").unwrap();
    assert_eq!(created, folder.path().join(".adp"));
    assert_eq!(read(&created), "helm/chart\r\n");

    let prompt = folder_prompt(&adds, folder.path());
    assert_eq!(
        find(&prompt.options, "ansible/structure").unwrap().unavailable_reason,
        "This folder is already registered by `.adp`"
    );
    assert_eq!(
        adds.commit(&target, "ansible/structure", ""),
        Err("This folder is already registered by '.adp'.".into())
    );
}

#[test]
fn what_the_standalone_host_refuses_is_refused_with_its_words() {
    let folder = tempfile::tempdir().unwrap();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let adds = AddCatalogue::standalone();
    assert_eq!(adds.validate(&target, " .adp"), Err("Enter a name.".into()));
    assert_eq!(
        adds.validate(&target, "a/b"),
        Err("A name cannot contain a path or any of \\ / : * ? \" < > |".into())
    );
    fs::write(folder.path().join("plan.tml"), "").unwrap();
    assert_eq!(
        adds.commit(&target, "generic/timeline", "plan"),
        Err("An item named 'plan.tml' already exists in this folder.".into())
    );
    assert!(!folder.path().join("plan.adp").exists(), "nothing is written");
    fs::write(folder.path().join("map.adp"), "").unwrap();
    assert_eq!(
        adds.commit(&target, "generic/timeline", "map"),
        Err("An item named 'map.adp' already exists in this folder.".into())
    );
    assert_eq!(adds.commit(&target, "wardley/map", "strategy"), Err(INCOMPLETE.into()));
    assert_eq!(
        adds.commit(&target, "nobody/nothing", "x"),
        Err("That diagram type is not available.".into())
    );
    assert_eq!(
        adds.commit(&AddTarget::Folder(folder.path().join("gone")), "generic/timeline", "x"),
        Err("The folder no longer exists.".into())
    );
}

#[test]
fn a_file_a_tool_type_reads_is_registered_beside_itself() {
    let folder = tempfile::tempdir().unwrap();
    let adds = AddCatalogue::standalone();
    let plain = folder.path().join("people.ttl");
    fs::write(&plain, "<a> <b> <c> .\n").unwrap();
    let ontology = folder.path().join("terms.ttl");
    fs::write(&ontology, "<> a owl:Ontology .\n").unwrap();
    let other = folder.path().join("notes.txt");
    fs::write(&other, "").unwrap();

    assert_eq!(adds.offer(&AddTarget::File(other.clone())), None);
    assert_eq!(
        adds.offer(&AddTarget::File(plain.clone())).map(|o| o.label),
        Some("Add as diagram…")
    );
    let ids = |file: &Path| -> Vec<String> {
        let prompt = adds.execute(&AddTarget::File(file.to_path_buf())).unwrap();
        prompt
            .options
            .iter()
            .flat_map(|v| v.children.iter().map(|o| o.id.clone()))
            .collect()
    };
    assert_eq!(ids(&plain), ["w3c/rdf"]);
    assert_eq!(ids(&ontology), ["w3c/owl", "w3c/rdf"]);
    let prompt = adds.execute(&AddTarget::File(ontology.clone())).unwrap();
    assert_eq!(prompt.title, "Open terms.ttl as");
    assert_eq!(prompt.confirm_label, "Register");
    assert_eq!(prompt.name_field, None);

    let created = adds.commit(&AddTarget::File(ontology.clone()), "w3c/owl", "").unwrap();
    assert_eq!(created, folder.path().join("terms.adp"));
    assert_eq!(read(&created), "w3c/owl\r\n");
    assert_eq!(read(&ontology), "<> a owl:Ontology .\n", "the file is left alone");
    assert_eq!(
        adds.offer(&AddTarget::File(ontology)),
        None,
        "a registered file is opened, not registered again"
    );
    assert_eq!(
        adds.commit(&AddTarget::File(plain), "w3c/owl", ""),
        Err("That diagram type is not available for this file.".into())
    );
}

#[test]
fn a_causal_loop_diagram_is_written_with_its_bindings_template() {
    let folder = tempfile::tempdir().unwrap();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let created = AddCatalogue::standalone()
        .commit(&target, "systems/causal-loop-diagram", "feedback")
        .unwrap();
    assert_eq!(created, folder.path().join("feedback.adp"));
    assert_eq!(read(&created), "systems/causal-loop-diagram\r\n");
    // The standalone host's `CausalLoopBody.Template`: the starter `causal-loop-diagram.fbl#cld` writes.
    assert_eq!(
        read(&folder.path().join("feedback.cld")),
        "causal-loop 1\r\n\r\nvariable population \"Population\"\r\nvariable births \"Births\"\r\n\r\nlink population -> births +\r\nlink births -> population +\r\n\r\nloop R1 \"Births beget births\" population births\r\n"
    );
}

#[test]
fn a_hype_cycle_graph_is_written_as_the_standalone_host_writes_an_empty_one() {
    let folder = tempfile::tempdir().unwrap();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let created = AddCatalogue::standalone()
        .commit(&target, "gartner/hypecycle-graph", "trends")
        .unwrap();
    assert_eq!(created, folder.path().join("trends.adp"));
    assert_eq!(read(&created), "gartner/hypecycle-graph\r\n");
    // The standalone host's `GhgParser.EmptyDocument("\r\n")`: the header and both sections flow-empty.
    assert_eq!(
        read(&folder.path().join("trends.ghg")),
        "gartner-hypecycle-graph: 1\r\ntrends: []\r\ninfluences: []\r\n"
    );
}

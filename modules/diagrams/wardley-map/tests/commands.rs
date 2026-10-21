//! Every edit of a Wardley map is a command with an inverse, run through the project's history (etalii.adp spec 030,
//! task T070), ported from the standalone host's `WardleyCommands.Tests.cs` at `8b6ccfc`. The last two lock a file,
//! which only Windows refuses to write, so what only they use is unused elsewhere.
#![cfg_attr(not(windows), allow(unused_imports, dead_code))]

mod common;

use std::fs;

use adp_diagram::DiagramDocuments;
use adp_diagram_wardley_map::commands::{
    AddWardleyElement, MoveWardleyElement, RemoveWardleyElement, RenameWardleyElement, SetWardleyDecorator,
    SetWardleyEvolve, SetWardleyInertia, SetWardleyLink, SetWardleyPipelineMembership,
};
use adp_diagram_wardley_map::identities;
use adp_diagram_wardley_map::model::{WardleyDecorator, WardleyLinkKind};
use adp_history::Command;
use common::Project;

fn add(project: &Project, kind: &str, name: &str, visibility: f64, maturity: f64) -> AddWardleyElement {
    AddWardleyElement {
        body_path: project.body.clone(),
        kind: kind.to_owned(),
        name: name.to_owned(),
        visibility,
        maturity,
        decorator: None,
    }
}

fn link(
    project: &Project,
    source: &str,
    target: &str,
    kind: WardleyLinkKind,
    present: bool,
    context: &str,
) -> SetWardleyLink {
    SetWardleyLink {
        body_path: project.body.clone(),
        source_element_id: source.to_owned(),
        target_element_id: target.to_owned(),
        kind,
        present,
        context: context.to_owned(),
    }
}

fn evolve(project: &Project, id: &str, present: bool, maturity: f64, override_name: &str) -> SetWardleyEvolve {
    SetWardleyEvolve {
        body_path: project.body.clone(),
        element_id: id.to_owned(),
        present,
        maturity,
        override_name: override_name.to_owned(),
    }
}

fn membership(project: &Project, id: &str, child: &str, member: bool, maturity: f64) -> SetWardleyPipelineMembership {
    SetWardleyPipelineMembership {
        body_path: project.body.clone(),
        parent_element_id: id.to_owned(),
        child_name: child.to_owned(),
        member,
        maturity,
    }
}

fn decorator(project: &Project, id: &str, decorator: WardleyDecorator, present: bool) -> SetWardleyDecorator {
    SetWardleyDecorator {
        body_path: project.body.clone(),
        element_id: id.to_owned(),
        decorator,
        present,
    }
}

#[test]
fn add_appends_a_statement_and_leaves_the_rest_alone() {
    let mut project = Project::new("// kept\ntitle Tea\ncomponent Alpha [0.5, 0.5]\n");
    let command = add(&project, "component", "Beta", 0.8, 0.2);
    assert!(project.execute(&command).is_ok());
    assert_eq!(
        project.read(),
        "// kept\ntitle Tea\ncomponent Alpha [0.5, 0.5]\ncomponent Beta [0.8, 0.2]\n"
    );
}

#[test]
fn add_refuses_a_duplicate_name() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let command = add(&project, "component", "Alpha", 0.1, 0.1);
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("already has an element"), "{refusal}");
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\n");
}

#[test]
fn add_refuses_a_statement_kind_the_format_does_not_have() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let command = add(&project, "market", "Beta", 0.5, 0.5);
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("no 'market' statement"), "{refusal}");
}

#[test]
fn add_clamps_a_position_outside_the_map() {
    let mut project = Project::new("title Empty\n");
    let command = add(&project, "component", "Alpha", 1.8, -0.3);
    project.execute(&command).unwrap();
    assert!(project.read().contains("component Alpha [1, 0]"));
}

#[test]
fn add_is_undone_exactly() {
    let before = "// kept\ncomponent Alpha [0.50, 0.50]\n";
    let mut project = Project::new(before);
    let command = add(&project, "component", "Beta", 0.1, 0.1);
    project.execute(&command).unwrap();
    project.undo();
    assert_eq!(project.read(), before);
}

#[test]
fn remove_takes_every_statement_that_named_the_element() {
    let mut project = Project::new(
        "component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha->Beta\nBeta->Alpha\nevolve Alpha 0.9",
    );
    let command = RemoveWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Alpha"),
    };
    project.execute(&command).unwrap();
    let after = project.read();
    assert!(!after.contains("Alpha"), "{after}");
    assert!(after.contains("component Beta [0.2, 0.2]"));
}

#[test]
fn remove_is_undone_exactly() {
    let before = "component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha->Beta\n";
    let mut project = Project::new(before);
    let command = RemoveWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Alpha"),
    };
    project.execute(&command).unwrap();
    project.undo();
    assert_eq!(project.read(), before);
}

#[test]
fn rename_rewrites_the_declaration_and_every_reference() {
    let mut project = Project::new("component Kettle [0.43, 0.35]\nCup of Tea->Kettle\nevolve Kettle 0.62\n");
    let command = RenameWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Kettle"),
        new_name: "Boiler".to_owned(),
    };
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Boiler [0.43, 0.35]\nCup of Tea->Boiler\nevolve Boiler 0.62\n"
    );
}

#[test]
fn rename_keeps_the_elements_identity() {
    let mut project = Project::new("component Kettle [0.43, 0.35]\n");
    let before = project.id_of("Kettle");
    let command = RenameWardleyElement {
        body_path: project.body.clone(),
        element_id: before.clone(),
        new_name: "Boiler".to_owned(),
    };
    project.execute(&command).unwrap();
    assert_eq!(project.id_of("Boiler"), before);
}

#[test]
fn rename_is_undone_exactly() {
    let before = "component Kettle [0.43, 0.35]\nCup of Tea->Kettle\nevolve Kettle 0.62\n";
    let mut project = Project::new(before);
    let command = RenameWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Kettle"),
        new_name: "Boiler".to_owned(),
    };
    project.execute(&command).unwrap();
    project.undo();
    assert_eq!(project.read(), before);
}

#[test]
fn rename_refuses_a_name_already_taken() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\n");
    let command = RenameWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Alpha"),
        new_name: "Beta".to_owned(),
    };
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("already has an element"), "{refusal}");
}

#[test]
fn set_inertia_adds_and_removes_the_bare_word() {
    let mut project = Project::new("component Datacentre [0.15, 0.55]\n");
    let id = project.id_of("Datacentre");
    let set = |inertia| SetWardleyInertia {
        body_path: project.body.clone(),
        element_id: id.clone(),
        inertia,
    };
    let (on, off) = (set(true), set(false));
    project.execute(&on).unwrap();
    assert_eq!(project.read(), "component Datacentre [0.15, 0.55] inertia\n");
    project.execute(&off).unwrap();
    assert_eq!(project.read(), "component Datacentre [0.15, 0.55]\n");
}

#[test]
fn set_decorator_adds_one_without_disturbing_another() {
    let mut project = Project::new("component Payment [0.70, 0.72] (buy)\n");
    let command = decorator(&project, &project.id_of("Payment"), WardleyDecorator::Market, true);
    project.execute(&command).unwrap();
    assert_eq!(project.read(), "component Payment [0.70, 0.72] (buy) (market)\n");
}

#[test]
fn set_decorator_removes_one_and_takes_its_spacing_with_it() {
    let mut project = Project::new("component Payment [0.70, 0.72] (buy)\n");
    let command = decorator(&project, &project.id_of("Payment"), WardleyDecorator::Buy, false);
    project.execute(&command).unwrap();
    assert_eq!(project.read(), "component Payment [0.70, 0.72]\n");
}

#[test]
fn set_decorator_keeps_a_trailing_comment() {
    let mut project = Project::new("component Payment [0.70, 0.72] // why\n");
    let command = decorator(&project, &project.id_of("Payment"), WardleyDecorator::Buy, true);
    project.execute(&command).unwrap();
    assert_eq!(project.read(), "component Payment [0.70, 0.72] (buy) // why\n");
}

#[test]
fn set_decorator_is_idempotent_without_writing_an_empty_undo_entry() {
    let mut project = Project::new("component Payment [0.70, 0.72] (buy)\n");
    let command = decorator(&project, &project.id_of("Payment"), WardleyDecorator::Buy, true);
    assert!(project.execute(&command).is_ok());
    assert_eq!(project.read(), "component Payment [0.70, 0.72] (buy)\n");
}

// ---- links ----------------------------------------------------------------------------------------------------------

#[test]
fn set_link_appends_the_arrow_it_was_asked_for() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\n");
    let command = link(
        &project,
        &project.id_of("Alpha"),
        &project.id_of("Beta"),
        WardleyLinkKind::Flow,
        true,
        "cash",
    );
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha+>Beta; cash\n"
    );
}

#[test]
fn set_link_rewrites_an_existing_link_rather_than_adding_a_second() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha->Beta\n");
    let command = link(
        &project,
        &project.id_of("Alpha"),
        &project.id_of("Beta"),
        WardleyLinkKind::Dependency,
        true,
        "needs",
    );
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha->Beta; needs\n"
    );
}

#[test]
fn set_link_clearing_one_kind_leaves_the_other_arrow_between_the_pair_alone() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha->Beta\nAlpha+>Beta\n");
    let command = link(
        &project,
        &project.id_of("Alpha"),
        &project.id_of("Beta"),
        WardleyLinkKind::Dependency,
        false,
        "",
    );
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\nAlpha+>Beta\n"
    );
}

#[test]
fn set_link_refuses_to_link_an_element_to_itself() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let id = project.id_of("Alpha");
    let command = link(&project, &id, &id, WardleyLinkKind::Dependency, true, "");
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("two different elements"), "{refusal}");
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\n");
}

#[test]
fn set_link_refuses_to_clear_a_link_that_is_not_there() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\ncomponent Beta [0.2, 0.2]\n");
    let command = link(
        &project,
        &project.id_of("Alpha"),
        &project.id_of("Beta"),
        WardleyLinkKind::Dependency,
        false,
        "",
    );
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("no such link"), "{refusal}");
}

// ---- evolve ---------------------------------------------------------------------------------------------------------

#[test]
fn set_evolve_appends_the_statement_and_then_rewrites_it_in_place() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let id = project.id_of("Alpha");
    let first = evolve(&project, &id, true, 0.7, "");
    project.execute(&first).unwrap();
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\nevolve Alpha 0.7\n");
    let again = evolve(&project, &id, true, 0.85, "");
    project.execute(&again).unwrap();
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\nevolve Alpha 0.85\n");
}

#[test]
fn set_evolve_writes_the_name_the_component_takes_when_it_arrives() {
    let mut project = Project::new("component Kettle [0.4, 0.3]\n");
    let command = evolve(&project, &project.id_of("Kettle"), true, 0.75, "Electric Kettle");
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Kettle [0.4, 0.3]\nevolve Kettle->Electric Kettle 0.75\n"
    );
}

#[test]
fn set_evolve_clamps_past_the_end_of_the_axis() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let command = evolve(&project, &project.id_of("Alpha"), true, 1.4, "");
    project.execute(&command).unwrap();
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\nevolve Alpha 1\n");
}

#[test]
fn set_evolve_clears_the_statement_and_refuses_when_there_is_none() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\nevolve Alpha 0.7\n");
    let command = evolve(&project, &project.id_of("Alpha"), false, 0.0, "");
    project.execute(&command).unwrap();
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\n");
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("is not evolving"), "{refusal}");
}

// ---- pipeline membership --------------------------------------------------------------------------------------------

#[test]
fn pipeline_membership_gives_the_parent_a_block_when_it_has_none() {
    let mut project = Project::new("component Kettle [0.4, 0.3]\n");
    let command = membership(&project, &project.id_of("Kettle"), "Electric Kettle", true, 0.63);
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Electric Kettle [0.63]\n}\n"
    );
}

#[test]
fn pipeline_membership_inserts_before_the_closing_brace_leaving_what_is_in_the_block_alone() {
    let mut project = Project::new(
        "component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Campfire Kettle [0.15]\n  // the other one\n}\n",
    );
    let command = membership(&project, &project.id_of("Kettle"), "Electric Kettle", true, 0.63);
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Campfire Kettle [0.15]\n  // the other one\n  component Electric Kettle [0.63]\n}\n"
    );
}

#[test]
fn pipeline_membership_removes_the_child_it_names() {
    let mut project = Project::new(
        "component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Campfire Kettle [0.15]\n  component Electric Kettle [0.63]\n}\n",
    );
    let command = membership(&project, &project.id_of("Kettle"), "Campfire Kettle", false, 0.5);
    project.execute(&command).unwrap();
    assert_eq!(
        project.read(),
        "component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Electric Kettle [0.63]\n}\n"
    );
}

#[test]
fn pipeline_membership_refuses_the_legacy_two_coordinate_form() {
    let mut project = Project::new("component Power [0.1, 0.7]\npipeline Power [0.30, 0.85]\n");
    let command = membership(&project, &project.id_of("Power"), "Solar", true, 0.4);
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("older two-coordinate form"), "{refusal}");
    assert_eq!(
        project.read(),
        "component Power [0.1, 0.7]\npipeline Power [0.30, 0.85]\n"
    );
}

#[test]
fn pipeline_membership_refuses_a_duplicate_child_and_a_child_that_is_not_there() {
    let mut project =
        Project::new("component Kettle [0.4, 0.3]\npipeline Kettle\n{\n  component Electric Kettle [0.63]\n}\n");
    let id = project.id_of("Kettle");
    let duplicate = membership(&project, &id, "Electric Kettle", true, 0.4);
    let absent = membership(&project, &id, "Campfire Kettle", false, 0.5);
    let refusal = project.execute(&duplicate).unwrap_err();
    assert!(refusal.contains("already in this pipeline"), "{refusal}");
    let refusal = project.execute(&absent).unwrap_err();
    assert!(refusal.contains("is not in this pipeline"), "{refusal}");
}

#[test]
fn every_edit_refuses_an_element_that_is_not_there() {
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let body = project.body.clone();
    let alpha = project.id_of("Alpha");
    let gone = "gone".to_owned();
    let commands: Vec<Box<dyn Command<DiagramDocuments>>> = vec![
        Box::new(RemoveWardleyElement {
            body_path: body.clone(),
            element_id: gone.clone(),
        }),
        Box::new(RenameWardleyElement {
            body_path: body.clone(),
            element_id: gone.clone(),
            new_name: "Beta".to_owned(),
        }),
        Box::new(SetWardleyInertia {
            body_path: body.clone(),
            element_id: gone.clone(),
            inertia: true,
        }),
        Box::new(decorator(&project, &gone, WardleyDecorator::Buy, true)),
        Box::new(link(&project, &gone, &alpha, WardleyLinkKind::Dependency, true, "")),
        Box::new(evolve(&project, &gone, true, 0.5, "")),
        Box::new(membership(&project, &gone, "Child", true, 0.5)),
    ];
    for command in commands {
        assert_eq!(
            project.execute(command.as_ref()).unwrap_err(),
            "That element is no longer on this map."
        );
    }
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\n");
}

#[test]
fn an_edit_and_its_undo_leave_the_document_byte_identical_across_the_whole_set() {
    let before = "// a comment\ncomponent Alpha [0.50, 0.50] (buy)\ncomponent Beta [0.20, 0.20]\nAlpha->Beta\n";
    type Make = fn(&Project, String, String) -> Box<dyn Command<DiagramDocuments>>;
    let makes: Vec<Make> = vec![
        |p, _, _| Box::new(add(p, "component", "Gamma", 0.3, 0.3)),
        |p, id, _| {
            Box::new(RemoveWardleyElement {
                body_path: p.body.clone(),
                element_id: id,
            })
        },
        |p, id, _| {
            Box::new(RenameWardleyElement {
                body_path: p.body.clone(),
                element_id: id,
                new_name: "Renamed".to_owned(),
            })
        },
        |p, id, _| {
            Box::new(SetWardleyInertia {
                body_path: p.body.clone(),
                element_id: id,
                inertia: true,
            })
        },
        |p, id, _| Box::new(decorator(p, &id, WardleyDecorator::Market, true)),
        |p, id, _| Box::new(decorator(p, &id, WardleyDecorator::Buy, false)),
        |p, id, beta| Box::new(link(p, &id, &beta, WardleyLinkKind::Flow, true, "cash")),
        |p, id, beta| Box::new(link(p, &id, &beta, WardleyLinkKind::Dependency, false, "")),
        |p, id, _| Box::new(evolve(p, &id, true, 0.9, "")),
        |p, id, _| Box::new(membership(p, &id, "Child", true, 0.4)),
    ];
    for make in makes {
        let mut project = Project::new(before);
        let command = make(&project, project.id_of("Alpha"), project.id_of("Beta"));
        project.execute(command.as_ref()).unwrap();
        project.undo();
        assert_eq!(project.read(), before, "{}", command.label());
    }
}

#[cfg(windows)]
#[test]
fn a_write_that_cannot_land_is_reported_rather_than_answered_with_success() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let command = MoveWardleyElement {
        body_path: project.body.clone(),
        element_id: project.id_of("Alpha"),
        visibility: 0.8,
        maturity: 0.2,
    };
    // A holder that shares reading only, which denies the replace a write performs: an editor with the file open.
    let _holder = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1)
        .open(&project.body)
        .unwrap();
    let refusal = project.execute(&command).unwrap_err();
    assert!(refusal.contains("could not be written"), "{refusal}");
    assert_eq!(project.read(), "component Alpha [0.5, 0.5]\n");
}

#[cfg(windows)]
#[test]
fn an_edit_whose_identities_cannot_be_saved_succeeds_but_says_so() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut project = Project::new("component Alpha [0.5, 0.5]\n");
    let beta = add(&project, "component", "Beta", 0.8, 0.2);
    project.execute(&beta).unwrap();
    let sidecar = identities::path_for(&project.body);
    assert!(sidecar.exists());
    let _holder = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x1)
        .open(&sidecar)
        .unwrap();
    let gamma = add(&project, "component", "Gamma", 0.3, 0.7);
    let warning = project.execute(&gamma).unwrap();
    assert!(project.read().contains("component Gamma"));
    assert!(warning.contains("new ids"), "{warning}");
}

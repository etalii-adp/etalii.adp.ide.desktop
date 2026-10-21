//! The add dialog's tests: choosing, naming, and the mouse and the keyboard on the tree, headless.

use std::fs;
use std::path::Path;

use adp_add::{AddCatalogue, AddTarget};

use super::*;

fn folder_prompt(adds: &AddCatalogue, folder: &Path) -> AddPrompt {
    adds.execute(&AddTarget::Folder(folder.to_path_buf())).unwrap()
}

#[test]
fn choosing_names_the_file_until_the_person_types_and_an_unavailable_choice_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    fs::write(folder.path().join(".adp"), "helm/chart\r\n").unwrap();
    let adds = AddCatalogue::standalone();
    let target = AddTarget::Folder(folder.path().to_path_buf());
    let mut dialog = AddDialog::new(target.clone(), folder_prompt(&adds, folder.path()));
    assert_eq!(
        dialog.name, "",
        "the first choosable option, a folder subject, suggests no name"
    );
    assert!(!dialog.can_submit(), "nothing is chosen");
    assert!(dialog.expanded.is_empty(), "the vendors read as a list first");

    dialog.choose("ansible/structure");
    assert_eq!(dialog.selected, None, "an unavailable choice is refused");

    dialog.choose("generic/timeline");
    assert_eq!(dialog.name, "timeline");
    assert!(dialog.can_submit());
    dialog.type_name("a/b", |n| adds.validate(&target, n));
    assert!(!dialog.can_submit());
    assert!(dialog.name_error.is_some());
    dialog.type_name("roadmap", |n| adds.validate(&target, n));
    dialog.choose("c4/container");
    assert_eq!(dialog.name, "roadmap", "a typed name is kept");
    assert_eq!(
        dialog.submission(),
        Some(DialogOutcome::Submit {
            value: "c4/container".into(),
            text: Some("roadmap".into())
        })
    );
}

/// One headless frame of the dialog in a window, with `events`; what the person did.
fn frame(ctx: &egui::Context, dialog: &mut AddDialog, events: Vec<egui::Event>) -> Option<DialogOutcome> {
    // A twentieth of a second a frame, so two presses a frame apart are a double click.
    let time = ctx.input(|i| i.time) + 0.05;
    let input = egui::RawInput {
        time: Some(time),
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 700.0))),
        events,
        ..Default::default()
    };
    let theme = Theme::dark();
    let mut outcome = None;
    ctx.run_ui(input, |ui| {
        egui::CentralPanel::default().show(ui, |ui| outcome = dialog.show(ui, &theme, |_| Ok(())));
    })
    .textures_delta
    .clear();
    outcome
}

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    }
}

#[test]
fn the_keyboard_walks_the_tree_chooses_and_confirms() {
    let folder = tempfile::tempdir().unwrap();
    let adds = AddCatalogue::standalone();
    let mut dialog = AddDialog::new(
        AddTarget::Folder(folder.path().to_path_buf()),
        folder_prompt(&adds, folder.path()),
    );
    let ctx = egui::Context::default();
    // The keyboard is on the tree from the start, and keeps its arrows from the frame after.
    frame(&ctx, &mut dialog, Vec::new());
    frame(&ctx, &mut dialog, Vec::new());
    frame(&ctx, &mut dialog, Vec::new());
    // The first vendor, ansible: open it, step into its one tool type, choose it, confirm it.
    assert_eq!(frame(&ctx, &mut dialog, vec![key(Key::ArrowRight)]), None);
    assert!(dialog.expanded.contains("ansible"));
    frame(&ctx, &mut dialog, vec![key(Key::ArrowRight)]);
    frame(&ctx, &mut dialog, vec![key(Key::Enter)]);
    assert_eq!(dialog.selected.as_deref(), Some("ansible/structure"));
    assert_eq!(
        frame(&ctx, &mut dialog, vec![key(Key::Enter)]),
        Some(DialogOutcome::Submit {
            value: "ansible/structure".into(),
            text: None
        }),
        "a folder subject is submitted without a name"
    );
    frame(&ctx, &mut dialog, vec![key(Key::ArrowLeft)]);
    frame(&ctx, &mut dialog, vec![key(Key::ArrowLeft)]);
    assert!(
        !dialog.expanded.contains("ansible"),
        "left goes to the vendor and closes it"
    );
}

#[test]
fn a_press_on_a_vendor_opens_it_and_a_double_click_on_a_tool_type_adds_it() {
    let folder = tempfile::tempdir().unwrap();
    let adds = AddCatalogue::standalone();
    let mut dialog = AddDialog::new(
        AddTarget::Folder(folder.path().to_path_buf()),
        folder_prompt(&adds, folder.path()),
    );
    let ctx = egui::Context::default();
    frame(&ctx, &mut dialog, Vec::new());
    let at = |dialog: &AddDialog, id: &str| {
        let rect = dialog.drawn.iter().find(|(i, _)| i == id).expect("drawn").1;
        rect.left_center() + egui::vec2(60.0, 0.0)
    };
    let first = at(&dialog, "ansible");
    let press = |pressed| egui::Event::PointerButton {
        pos: first,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&ctx, &mut dialog, vec![egui::Event::PointerMoved(first), press(true)]);
    frame(&ctx, &mut dialog, vec![press(false)]);
    assert!(dialog.expanded.contains("ansible"), "a press on a vendor opens it");
    // Half a second on, so the presses on the tool type are not a triple click with the press on the vendor.
    for _ in 0..10 {
        frame(&ctx, &mut dialog, Vec::new());
    }
    let second = at(&dialog, "ansible/structure");
    let press = |pressed| egui::Event::PointerButton {
        pos: second,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    frame(&ctx, &mut dialog, vec![egui::Event::PointerMoved(second), press(true)]);
    frame(&ctx, &mut dialog, vec![press(false)]);
    assert_eq!(dialog.selected.as_deref(), Some("ansible/structure"));
    frame(&ctx, &mut dialog, vec![press(true)]);
    let outcome = frame(&ctx, &mut dialog, vec![press(false)]);
    assert_eq!(
        outcome,
        Some(DialogOutcome::Submit {
            value: "ansible/structure".into(),
            text: None
        })
    );
}

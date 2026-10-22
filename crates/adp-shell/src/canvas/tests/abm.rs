//! An agent behavior model on the canvas with its module (etalii.adp spec 030, task T074): the standalone host's
//! `research-assistant` example, whose registration stores one dragged row, opened through its registration from the
//! parity fixtures, drawn as its tidy tree, edited from a node's menu, its inline editor and a drag, arranged, and every
//! edit undone through the project's history.

use std::fs;
use std::path::Path;

use adp_diagram::ActionTarget;
use eframe::egui::PointerButton;

use super::{Harness, bodies, texts_of};

fn research_assistant() -> Harness {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../parity/fixtures/etalii/agent-behavior-modelling/research-assistant");
    Harness::open_from(
        &folder,
        &["research-assistant.adp", "research-assistant.md"],
        "research-assistant.adp",
        "research-assistant.md",
    )
}

#[test]
fn a_behavior_model_opens_as_its_tidy_tree_and_is_edited_dragged_and_arranged_on_the_canvas() {
    let mut harness = research_assistant();
    harness.settle();
    let scene = harness.tab.scene().expect("a scene");
    assert_eq!(bodies(scene), 13, "every node has its shape");
    let texts = texts_of(scene);
    for label in [
        "Answer the research question",
        "Settle the question",
        "Search the user's own documents",
    ] {
        assert!(texts.contains(&label), "{label} is drawn: {texts:?}");
    }

    // The tree: the root above its children, the children left to right in the Markdown's order, and the root's row
    // at the height the registration stores for 1.2 (160, a top-left), not the computed 116.
    let centre = |harness: &Harness, id: &str| {
        harness
            .tab
            .bounds_of(id)
            .unwrap_or_else(|| panic!("{id} is drawn"))
            .centre()
    };
    let root = centre(&harness, "1");
    let (first, second, third) = (
        centre(&harness, "1.1"),
        centre(&harness, "1.2"),
        centre(&harness, "1.3"),
    );
    assert!(root.y < first.y && first.x < second.x && second.x < third.x);
    assert!(
        (first.y - 190.0).abs() < 1e-6 && (third.y - 190.0).abs() < 1e-6,
        "{first:?} {third:?}"
    );

    // A child added from a node's menu, its label asked in place over it, and committed.
    let registration = harness.root.join("research-assistant.adp");
    let original_registration = fs::read_to_string(&registration).unwrap();
    let (host, tab, root_folder) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root_folder),
        ActionTarget::Element("1.1".into()),
        "abm.add.check",
    );
    let editor = harness
        .tab
        .editor
        .as_mut()
        .expect("the new node's label is asked in place");
    assert_eq!(editor.element, "1.1.3");
    assert_eq!(editor.text, "New question");
    editor.text = "The sources are known".into();
    let (host, tab, root_folder) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.commit_editor(host.workspace(root_folder));
    assert!(
        harness
            .written()
            .contains("    - **Ask the user:** What is the answer for, and which period should it cover?\n    - **Check:** The sources are known\n  - **Do together:** Search\n")
    );

    // A drag of the first Delegate past the second's middle reorders them in the Markdown, and stores the height of
    // its row for every node in it, as one edit; the parent's own entry is kept as it was.
    harness.tab.changed();
    harness.settle();
    let before_drag = harness.written();
    let from = harness.screen_of("1.2.1");
    let to = harness.screen_of("1.2.2") + eframe::egui::vec2(20.0, 0.0);
    harness.drag(from, to, PointerButton::Primary, true);
    let dragged = harness.written();
    assert!(
        dragged.contains("    - **Delegate:** Search news and official publications\n    - **Delegate:** Search the academic literature\n"),
        "{dragged}"
    );
    let stored = fs::read_to_string(&registration).unwrap();
    assert_eq!(
        stored,
        "etalii/agent-behavior-modelling\nlayout:\r\n  1.2: 360 160\r\n  1.2.1: 684 276\r\n  1.2.2: 912 276\r\n  1.2.3: 1140 276\r\n"
    );

    // Arrange diagram forgets every dragged position and leaves the Markdown alone.
    let (host, tab, root_folder) = (&mut harness.host, &mut harness.tab, &harness.root);
    tab.run_action(
        host.workspace(root_folder),
        ActionTarget::Element("1".into()),
        "abm.arrange",
    );
    assert_eq!(
        fs::read_to_string(&registration).unwrap(),
        "etalii/agent-behavior-modelling\n"
    );
    assert_eq!(harness.written(), dragged);
    harness.tab.changed();
    harness.settle();
    let arranged = harness.tab.bounds_of("1.1").unwrap().centre();
    assert!(
        (arranged.y - 146.0).abs() < 1e-6,
        "the computed row, 56 under the root: {arranged:?}"
    );

    // Four undos take every edit back, byte for byte: the arrangement, the drag, the label and the add.
    assert!(harness.workspace().undo().is_done());
    assert_eq!(fs::read_to_string(&registration).unwrap(), stored);
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), before_drag);
    assert_eq!(fs::read_to_string(&registration).unwrap(), original_registration);
    assert!(harness.workspace().undo().is_done());
    assert!(harness.workspace().undo().is_done());
    assert_eq!(harness.written(), harness.original);
}

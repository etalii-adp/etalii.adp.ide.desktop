//! The context menus of each bundled definition (DISL section 7.3), grouped by `group` (DISL 0.3 section 7.3) (the
//! standalone host's `ContextMenuDerivationTests`).

mod common;

use adp_cel::{Map, Value};
use adp_disl::derivation::context_menu;
use adp_disl::{DerivedMenuGroup, DerivedShortcut, DislDiagram, DislEnv, DislMenuTarget, WireIdMap};
use common::{
    behavior_diagram, behavior_model, behavior_model_ids, element, hype_cycle, hype_cycle_diagram, hype_cycle_ids,
    loaded, with,
};

fn readable() -> DislEnv {
    DislEnv {
        read_only: false,
        viewpoint: "trueTime".into(),
        now: None,
    }
}

/// One line per entry, prefixed with its group's number and name, as the parity transcripts write menus.
fn lines(groups: &[DerivedMenuGroup]) -> Vec<String> {
    groups
        .iter()
        .enumerate()
        .flat_map(|(index, group)| {
            group.entries.iter().map(move |entry| {
                format!(
                    "g{} {} {} | {} | {} | {} | {}{}",
                    index + 1,
                    group.name.as_deref().unwrap_or(""),
                    entry.id,
                    entry.label,
                    entry.icon,
                    entry.shortcut.as_ref().map(|s| s.text.as_str()).unwrap_or("-"),
                    if entry.available { "on" } else { "off" },
                    if entry.unavailable_reason.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", entry.unavailable_reason)
                    }
                )
            })
        })
        .collect()
}

fn hype(target: &DislMenuTarget, env: Option<DislEnv>) -> Vec<DerivedMenuGroup> {
    context_menu::derive(&hype_cycle(), target, &env.unwrap_or_else(readable), &hype_cycle_ids())
}

fn behavior(target: &DislMenuTarget) -> Vec<DerivedMenuGroup> {
    context_menu::derive(&behavior_model(), target, &DislEnv::default(), &behavior_model_ids())
}

#[test]
fn a_trend_offers_its_edits_then_arrange_in_a_group_of_its_own() {
    let diagram = hype_cycle_diagram();

    let groups = hype(&DislMenuTarget::Element(element(&diagram, "steam")), None);

    assert_eq!(
        vec![
            "g1 edit ghg.rename | Rename… | mdi-pencil-outline | F2 | on",
            "g1 edit ghg.remove | Remove | mdi-delete-outline | Delete | on",
            "g2 arrange ghg.arrange | Arrange diagram | mdi-sitemap-outline | - | on",
        ],
        lines(&groups)
    );
}

#[test]
fn a_trend_with_a_dragged_boundary_offers_even_phases() {
    let diagram = hype_cycle_diagram();

    let groups = hype(&DislMenuTarget::Element(element(&diagram, "rail")), None);

    assert_eq!(
        "g1 edit ghg.even-phases | Even phases | mdi-arrow-split-vertical | - | on",
        lines(&groups)[1]
    );
}

#[test]
fn an_influence_is_removed_by_its_own_action() {
    let diagram = hype_cycle_diagram();

    let groups = hype(&DislMenuTarget::Element(element(&diagram, "i1")), None);

    assert_eq!(
        vec![
            "g1 edit ghg.disconnect | Remove influence | mdi-vector-polyline-remove | Delete | on",
            "g2 arrange ghg.arrange | Arrange diagram | mdi-sitemap-outline | - | on",
        ],
        lines(&groups)
    );
}

#[test]
fn read_only_offers_no_groups_at_all() {
    let diagram = hype_cycle_diagram();
    let read_only = || {
        Some(DislEnv {
            read_only: true,
            ..readable()
        })
    };

    assert!(hype(&DislMenuTarget::Element(element(&diagram, "steam")), read_only()).is_empty());
    assert!(hype(&DislMenuTarget::Canvas(diagram.clone()), read_only()).is_empty());
    assert!(
        hype(
            &DislMenuTarget::Connection {
                diagram: diagram.clone(),
                source: None,
                target: None,
                relation_type: None
            },
            read_only()
        )
        .is_empty()
    );
}

#[test]
fn the_empty_canvas_offers_the_adds_and_arrange_unavailable_until_there_is_something() {
    let groups = hype(&DislMenuTarget::Canvas(DislDiagram::new(&hype_cycle())), None);

    assert_eq!(
        vec![
            "g1 add ghg.add.trend | Add trend here | mdi-plus | - | on",
            "g1 add ghg.add.trigger | Add trigger here | mdi-circle-slice-8 | - | on",
            "g1 add ghg.add.note | Add note here | mdi-note-text-outline | - | on",
            "g2 arrange ghg.arrange | Arrange diagram | mdi-sitemap-outline | - | off (There is nothing to arrange until this graph has a trend.)",
        ],
        lines(&groups)
    );
}

#[test]
fn a_sets_when_keeps_the_canvas_menu_to_its_viewpoint() {
    let groups = hype(
        &DislMenuTarget::Canvas(hype_cycle_diagram()),
        Some(DislEnv {
            viewpoint: "compact".into(),
            ..readable()
        }),
    );

    assert!(groups.is_empty());
}

#[test]
fn a_connection_is_drawn_as_an_influence() {
    let diagram = hype_cycle_diagram();

    let groups = hype(
        &DislMenuTarget::Connection {
            diagram: diagram.clone(),
            source: Some(element(&diagram, "steam")),
            target: None,
            relation_type: None,
        },
        None,
    );

    assert_eq!(
        vec!["g1  ghg.connect.influence | Influence | mdi-ray-start-arrow | - | on"],
        lines(&groups)
    );
}

#[test]
fn a_behavior_node_offers_its_edits_one_add_child_per_kind_and_arrange() {
    let diagram = behavior_diagram();

    let lines = lines(&behavior(&DislMenuTarget::Element(element(&diagram, "1.1"))));

    assert_eq!(
        vec![
            "g1 edit abm.rename | Rename… | mdi-pencil-outline | F2 | on",
            "g1 edit abm.edit-notes | Edit notes… | mdi-note-text-outline | - | on",
            "g1 edit abm.move-earlier | Move earlier | mdi-arrow-left | Alt+Up | off (It is already the first of its siblings.)",
            "g1 edit abm.move-later | Move later | mdi-arrow-right | Alt+Down | on",
            "g1 edit abm.remove | Remove | mdi-delete-outline | Delete | on",
            "g2 arrange abm.arrange | Arrange diagram | mdi-sitemap-outline | - | on",
        ],
        lines
    );
}

#[test]
fn a_node_that_takes_a_child_offers_each_kind_under_it_by_the_kinds_menu_name() {
    let diagram = behavior_diagram();

    let groups = behavior(&DislMenuTarget::Element(element(&diagram, "1.2")));

    assert_eq!(
        vec![Some("edit"), Some("add"), Some("arrange")],
        groups.iter().map(|g| g.name.as_deref()).collect::<Vec<_>>()
    );
    let adds = &groups[1].entries;
    assert_eq!(11, adds.len());
    assert_eq!(
        "abm.add.sequence | Add child: Do in order",
        format!("{} | {}", adds[0].id, adds[0].label)
    );
    assert_eq!(
        "abm.add.repeat | Add child: Repeat until",
        format!("{} | {}", adds[4].id, adds[4].label)
    );
    assert_eq!(Some(&Value::from("RepeatUntil")), adds[4].arguments.get("kind"));
    let later = &groups[0].entries[3];
    assert_eq!(
        "abm.move-later | off (It is already the last of its siblings.)",
        format!(
            "{} | {} ({})",
            later.id,
            if later.available { "on" } else { "off" },
            later.unavailable_reason
        )
    );
}

#[test]
fn the_behavior_models_canvas_offers_each_kind_here_and_arrange() {
    let groups = behavior(&DislMenuTarget::Canvas(behavior_diagram()));

    assert_eq!(vec![11, 1], groups.iter().map(|g| g.entries.len()).collect::<Vec<_>>());
    assert_eq!(
        "abm.add.ask | Add Ask the user here",
        format!("{} | {}", groups[0].entries[9].id, groups[0].entries[9].label)
    );
    assert_eq!("abm.arrange", groups[1].entries[0].id);
}

#[test]
fn a_connection_on_a_derived_relation_runs_its_connect_edit() {
    let diagram = behavior_diagram();

    let groups = behavior(&DislMenuTarget::Connection {
        diagram: diagram.clone(),
        source: Some(element(&diagram, "1.2")),
        target: Some(element(&diagram, "1.1")),
        relation_type: Some("Child".into()),
    });

    assert_eq!(
        vec!["g1  abm.connect.child | Move under this node | mdi-file-tree-outline | - | on"],
        lines(&groups)
    );
}

#[test]
fn a_shortcut_is_read_as_modifiers_then_its_key() {
    for (text, key, ctrl, shift, alt, meta) in [
        ("F2", "F2", false, false, false, false),
        ("Alt+Up", "Up", false, false, true, false),
        ("Ctrl+Shift+Z", "Z", true, true, false, false),
        ("Cmd+Plus", "Plus", false, false, false, true),
    ] {
        assert_eq!(
            DerivedShortcut {
                text: text.into(),
                key: key.into(),
                ctrl,
                shift,
                alt,
                meta
            },
            DerivedShortcut::parse(text)
        );
    }
}

#[test]
fn groups_follow_the_declared_entries_and_a_hidden_ones_group_is_dropped() {
    let specification = loaded(&with(
        r#""toolbox": { "contextMenus": [ { "for": [ "Thing" ], "tools": [
          { "kind": "editLabel", "label": "A", "group": "one" },
          { "kind": "delete", "label": "B", "group": "two", "visible": "false" },
          { "kind": "duplicate", "label": "C", "group": "one" },
          { "kind": "duplicate", "label": "D", "group": "one" },
          { "kind": "openForm", "label": "E" } ] } ] }"#,
    ));
    let diagram = DislDiagram::new(&specification);
    let thing = diagram.add_node("Thing", "t", Map::default()).unwrap();

    let groups = context_menu::derive(
        &specification,
        &DislMenuTarget::Element(thing),
        &DislEnv::default(),
        &WireIdMap::none(),
    );

    assert_eq!(
        vec!["one:A", "one:C,D", ":E"],
        groups
            .iter()
            .map(|g| format!(
                "{}:{}",
                g.name.as_deref().unwrap_or(""),
                g.entries.iter().map(|e| e.label.as_str()).collect::<Vec<_>>().join(",")
            ))
            .collect::<Vec<_>>()
    );
}

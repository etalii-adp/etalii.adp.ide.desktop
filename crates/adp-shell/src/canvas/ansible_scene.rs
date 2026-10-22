//! The scene of an Ansible structure diagram, which its module lays out and hands the session as boxes and relations
//! (etalii.adp spec 030, task T079), drawn as `adp-scene` primitives so the one painter, hit test and gesture arbiter
//! serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/ansible-structure/client/AnsibleCanvas.tsx` at `8b6ccfc` drew
//! through the notation of `ansible-structure.dis`, in its theme's colours, light or dark:
//! - every node is a box at the size the module measured, its name in the middle; a playbook's outline is the primary
//!   colour and heavier, a play is tinted with its play's hue (six slots, wrapped) and dashed, a role's corners are
//!   rounder and a hollow role is dashed and faded, a task file is dashed on the plain surface, and an inventory or
//!   variable folder is a band box on the plain surface;
//! - a relation runs out of its source's right side into its target's left side as a horizontal curve, with a filled
//!   arrowhead; a dynamic one is dashed, a dependency is dotted in the primary colour, a play reaching an inventory is
//!   thin and faint; its directive (and its condition) is written above its middle;
//! - a relation whose target is missing or an expression is a 48-unit stub out of its source's right side in the
//!   warning colour, with the target as written above it.
//!
//! Every box is moved by its top-left, which is what the registration stores (`onElementMoved`).

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_ansible_structure::mapper::{
    EDGE_TYPE, INVENTORY_TYPE, PLAY_TYPE, PLAYBOOK_TYPE, ROLE_TYPE, TASK_FILE_TYPE, VARIABLE_FOLDER_TYPE,
};
use adp_diagram_ansible_structure::theme::palette;
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, marker_shape};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};
use adp_theme::Kind;

use super::module_scene::{ModuleScene, SELECTED_WIDTH, font, number, selection_colour, text};

/// What the canvas says of a folder holding nothing recognised (`canvas.empty`).
const EMPTY: &str = "Nothing here is laid out the way Ansible expects, so there is nothing to draw. Playbooks at the folder root or under playbooks/, roles under roles/, inventories under inventories/.";

/// The length of a missing target's stub.
const STUB: f64 = 48.0;

fn stroke(colour: &str, width: f64, dash: &[f64], opacity: f64) -> Stroke {
    Stroke {
        colour: Some(Colour::value(colour)),
        width,
        dash: dash.to_vec(),
        opacity,
    }
}

/// Where a node is drawn: its top-left and the size the module measured.
pub fn rect_of(element: &DiagramElement) -> Rect {
    Rect::new(
        element.x,
        element.y,
        number(element, "width"),
        number(element, "height"),
    )
}

/// The scene of `elements`, an Ansible structure view's rendering, with `selection` outlined, the elements `moving`
/// carried by the drag in flight, in the palette of the active theme, `theme`.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    theme: Kind,
) -> ModuleScene {
    let palette = palette(theme);
    let highlight = selection_colour(theme);
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements.iter().filter(|e| e.kind == ElementKind::Node) {
        rects.insert(element.id.clone(), rect_of(element));
    }
    let mut drawn = rects.clone();
    if let Some((ids, (dx, dy))) = moving {
        for id in ids {
            if let Some(rect) = drawn.get_mut(id) {
                *rect = Rect::new(rect.x + dx, rect.y + dy, rect.width, rect.height);
            }
        }
    }

    let mut primitives = Vec::new();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut labels = Vec::new();

    // The relations first, beneath the boxes they join.
    for relation in elements.iter().filter(|e| e.element_type == EDGE_TYPE) {
        let Some(source) = relation.source.as_deref().and_then(|s| drawn.get(s)) else {
            continue;
        };
        let owner = Owner::Relation(relation.id.clone());
        let is_selected = selected.contains(relation.id.as_str());
        let from = Point::new(source.right(), source.centre().y);
        let target = relation.target.as_deref().and_then(|t| drawn.get(t));
        let Some(target) = target else {
            // A target that is missing or an expression: the stub, with the target's own words above it.
            let to = Point::new(from.x + STUB, from.y);
            let width = if relation.attribute("relation") == "DependsOn" {
                2.0
            } else {
                1.5
            };
            let line = if is_selected {
                stroke(highlight, SELECTED_WIDTH, &[3.0, 3.0], 1.0)
            } else {
                stroke(palette.warning, width, &[3.0, 3.0], 1.0)
            };
            primitives.push(Primitive::Edge(EdgePrimitive {
                owner: owner.clone(),
                path: Path::polyline(&[from, to]),
                end: None,
                stroke: line,
                start: None,
                hit_width: 10.0,
                selectable: true,
                tooltip: None,
            }));
            lines.push((relation.id.clone(), vec![from, to], 5.0));
            let suffix = if relation.attribute("resolution") == "expression" {
                " (expression)"
            } else {
                " (missing)"
            };
            primitives.push(text(
                &owner,
                Role::LineLabel,
                format!("{}{suffix}", relation.attribute("targetAsWritten")),
                Point::new(from.x + 8.0, from.y - 6.0),
                TextAnchor::Start,
                font(10.0, 400, Some(palette.warning)),
            ));
            continue;
        };
        let to = Point::new(target.x, target.centre().y);
        let bend = ((to.x - from.x).abs() / 2.0).max(24.0);
        let path =
            Path::new()
                .move_to(from)
                .cubic_to(Point::new(from.x + bend, from.y), Point::new(to.x - bend, to.y), to);
        let (colour, width, dash, opacity): (&str, f64, &[f64], f64) = match relation.attribute("relation") {
            "DependsOn" => (palette.primary, 2.0, &[1.0, 4.0], 1.0),
            "Targets" => (palette.muted, 1.0, &[], 0.55),
            _ if relation.attribute("dynamic") == "true" => (palette.muted, 1.5, &[5.0, 4.0], 1.0),
            _ => (palette.muted, 1.5, &[], 1.0),
        };
        let line = if is_selected {
            stroke(highlight, SELECTED_WIDTH, dash, 1.0)
        } else {
            stroke(colour, width, dash, opacity)
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path,
            end: marker_shape(MarkerKind::Arrow, to, 0.0, line.width),
            stroke: line,
            start: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: None,
        }));
        let middle = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        lines.push((relation.id.clone(), vec![from, middle, to], 5.0));
        let condition = relation.attribute("condition");
        let label = if condition.is_empty() {
            relation.attribute("directive").to_owned()
        } else {
            format!("{} when {condition}", relation.attribute("directive"))
        };
        if relation.attribute("relation") != "Targets" && !label.is_empty() {
            primitives.push(text(
                &owner,
                Role::LineLabel,
                label,
                Point::new(middle.x, middle.y - 6.0),
                TextAnchor::Middle,
                font(10.0, if is_selected { 600 } else { 400 }, Some(palette.muted)),
            ));
        }
    }

    // The boxes, at the sizes the module measured.
    for node in elements.iter().filter(|e| e.kind == ElementKind::Node) {
        let Some(rect) = drawn.get(&node.id).copied() else {
            continue;
        };
        let owner = Owner::Element(node.id.clone());
        let hollow = node.attribute("hollow") == "true";
        let (fill, outline, radius) = match node.element_type.as_str() {
            PLAYBOOK_TYPE => (palette.raised.to_owned(), stroke(palette.primary, 2.0, &[], 1.0), 4.0),
            PLAY_TYPE => {
                let slot = node.attribute("playIndex").parse::<usize>().unwrap_or(0) % 6;
                let hue = palette.hues[slot];
                (format!("{hue}2e"), stroke(hue, 1.5, &[6.0, 3.0], 1.0), 4.0)
            }
            ROLE_TYPE if hollow => (
                palette.raised.to_owned(),
                stroke(palette.border, 1.5, &[4.0, 4.0], 1.0),
                10.0,
            ),
            ROLE_TYPE => (palette.raised.to_owned(), stroke(palette.border, 1.5, &[], 1.0), 10.0),
            TASK_FILE_TYPE => (
                palette.surface.to_owned(),
                stroke(palette.border, 1.5, &[2.0, 2.0], 1.0),
                4.0,
            ),
            INVENTORY_TYPE | VARIABLE_FOLDER_TYPE => {
                (palette.surface.to_owned(), stroke(palette.muted, 1.5, &[], 1.0), 4.0)
            }
            _ => (palette.raised.to_owned(), stroke(palette.border, 1.5, &[], 1.0), 4.0),
        };
        let outline = if selected.contains(node.id.as_str()) {
            stroke(highlight, SELECTED_WIDTH, &outline.dash, 1.0)
        } else {
            outline
        };
        let moved = moving.is_some_and(|(ids, _)| ids.contains(&node.id));
        let opacity = match (hollow, moved) {
            (true, _) => 0.6,
            (false, true) => 0.75,
            (false, false) => 1.0,
        };
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: Path::rounded_rect(rect, radius),
            fill: Some(Colour::value(&fill)),
            stroke: Some(outline),
            opacity,
            tooltip: Some(tooltip(node)),
        }));
        primitives.push(text(
            &owner,
            Role::Label {
                id: "name".to_owned(),
                line: 0,
            },
            node.attribute("name").to_owned(),
            Point::new(rect.centre().x, rect.centre().y + 4.5),
            TextAnchor::Middle,
            font(13.0, 400, Some(palette.text)),
        ));
        labels.push((node.id.clone(), "name".to_owned(), rect));
        if selected.contains(node.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((node.id.clone(), rect));
    }

    let empty_text = (!elements.iter().any(|e| e.kind == ElementKind::Node)).then(|| EMPTY.to_owned());
    ModuleScene {
        scene: Scene { primitives, empty_text },
        top_left: rects.keys().cloned().collect(),
        rects,
        boxes,
        lines,
        labels,
        anchors: Vec::new(),
    }
}

/// What a node's tooltip says (`notation.nodes.*.tooltip`): its kind and name, and its hosts where it has them.
fn tooltip(node: &DiagramElement) -> String {
    let kind = match node.element_type.as_str() {
        PLAYBOOK_TYPE => "Playbook",
        PLAY_TYPE => "Play",
        ROLE_TYPE => "Role",
        TASK_FILE_TYPE => "Task file",
        INVENTORY_TYPE => "Inventory",
        VARIABLE_FOLDER_TYPE => "Variables",
        _ => "Element",
    };
    let hosts = node.attribute("hosts");
    if hosts.is_empty() {
        format!("{kind} {}", node.attribute("name"))
    } else {
        format!("{kind} {} — hosts: {hosts}", node.attribute("name"))
    }
}

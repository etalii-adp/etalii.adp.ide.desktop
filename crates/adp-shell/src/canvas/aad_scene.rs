//! The scene of an agent activity diagram, which its module lays out on its rings and hands the session as cards and
//! relations (etalii.adp spec 030, task T075), drawn as `adp-scene` primitives so the one painter, hit test and gesture
//! arbiter serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's `src/diagrams/agent-activity-diagram/client/AadCanvas.tsx` and `aad.css` at
//! `8b6ccfc` draw from `agent-activity-diagram.dis`'s notation, in its theme's colours for the light and the dark mode:
//! - a project is a folder, a specification a rounded rectangle outlined in its status's colour with the status in
//!   words under its name, an agent a pill, a location a rectangle split by a line with its folder below it, an
//!   environment a hexagon with its kind under its name; each kind in its own fill;
//! - a specification lists its tasks by status, each group under a heading with its count, a folded group as its
//!   heading alone; a location lists its pull requests the same way;
//! - a locked element carries a lock at its top-left;
//! - a relation is a plain line between the two cards' edges, a breach dashed in the danger colour.
//!
//! Every card is moved by its centre, which is what a locked position is (`onElementMoved`).

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_agent_activity::mapper::{HEADING_HEIGHT, PREFIX, RELATION_TYPE, ROW_HEIGHT, VIEW_TYPE};
use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};

use super::module_scene::{
    ANCHOR_RADIUS, AVERAGE_ADVANCE, ModuleScene, SELECTED, SELECTED_WIDTH, font, number, stroke, text,
};

/// The theme's tokens (`notation.theme`), light first, dark second.
struct Palette {
    text: &'static str,
    muted: &'static str,
    border: &'static str,
    relation: &'static str,
    danger: &'static str,
}

const LIGHT: Palette = Palette {
    text: "#0f172a",
    muted: "#475569",
    border: "#94a3b8",
    relation: "#64748b",
    danger: "#dc2626",
};

const DARK: Palette = Palette {
    text: "#f1f5f9",
    muted: "#cbd5e1",
    border: "#64748b",
    relation: "#94a3b8",
    danger: "#f87171",
};

/// A kind's fill (`aad.<kind>`).
fn fill(kind: &str, dark: bool) -> &'static str {
    match (kind, dark) {
        ("project", false) => "#dbeafe",
        ("project", true) => "#1e3a8a",
        ("specification", false) => "#ffffff",
        ("specification", true) => "#1e293b",
        ("agent", false) => "#ede9fe",
        ("agent", true) => "#4c1d95",
        ("location", false) => "#fef3c7",
        ("location", true) => "#78350f",
        (_, false) => "#dcfce7",
        (_, true) => "#14532d",
    }
}

/// A status's colour (`aad.<status>`).
fn status_colour(status: &str, dark: bool) -> &'static str {
    match (status, dark) {
        ("progressing", false) => "#2563eb",
        ("progressing", true) => "#60a5fa",
        ("inputRequired", false) => "#c2410c",
        ("inputRequired", true) => "#fb923c",
        ("finished", false) => "#15803d",
        ("finished", true) => "#4ade80",
        ("archived", false) => "#94a3b8",
        ("archived", true) => "#64748b",
        (_, false) => "#64748b",
        (_, true) => "#94a3b8",
    }
}

/// A group's heading, as the definition names the status it lists.
fn heading(group: &str) -> &str {
    match group {
        "progressing" => "Progressing",
        "pending" => "Pending",
        "inputRequired" => "Input Required",
        "finished" => "Finished",
        "pullRequests" => "Pull requests",
        other => other,
    }
}

/// `line` whole when it fits `width` at `size`, else cut with an ellipsis (`overflow: ellipsis`).
fn fitted(line: &str, width: f64, size: f64) -> String {
    let capacity = ((width / (size * AVERAGE_ADVANCE)).floor() as usize).max(1);
    let characters: Vec<char> = line.chars().collect();
    if characters.len() <= capacity {
        return line.to_owned();
    }
    let kept: String = characters[..capacity.saturating_sub(1)].iter().collect();
    format!("{kept}…")
}

fn shape(
    owner: &Owner,
    role: Role,
    path: Path,
    fill: Option<&str>,
    outline: Option<Stroke>,
    opacity: f64,
) -> Primitive {
    Primitive::Shape(ShapePrimitive {
        owner: owner.clone(),
        role,
        path,
        fill: fill.map(Colour::value),
        stroke: outline,
        opacity,
        tooltip: None,
    })
}

fn label(id: &str) -> Role {
    Role::Label {
        id: id.to_owned(),
        line: 0,
    }
}

/// The outline of a card of `kind` (`notation.nodes.<Type>.shape`).
fn silhouette(kind: &str, r: Rect) -> Path {
    match kind {
        "project" => {
            // A folder: a tab on the left of its top edge.
            let tab = (r.width * 0.35).min(64.0);
            Path::new()
                .move_to(Point::new(r.x, r.y + 8.0))
                .line_to(Point::new(r.x, r.y))
                .line_to(Point::new(r.x + tab, r.y))
                .line_to(Point::new(r.x + tab + 8.0, r.y + 8.0))
                .line_to(Point::new(r.right(), r.y + 8.0))
                .line_to(Point::new(r.right(), r.bottom()))
                .line_to(Point::new(r.x, r.bottom()))
                .close()
        }
        "specification" => Path::rounded_rect(r, 8.0),
        "agent" => Path::rounded_rect(r, r.height / 2.0),
        "location" => Path::rounded_rect(r, 0.0),
        _ => {
            // A flat hexagon.
            let inset = (r.height / 2.0).min(r.width / 4.0);
            Path::polygon(&[
                Point::new(r.x + inset, r.y),
                Point::new(r.right() - inset, r.y),
                Point::new(r.right(), r.centre().y),
                Point::new(r.right() - inset, r.bottom()),
                Point::new(r.x + inset, r.bottom()),
                Point::new(r.x, r.centre().y),
            ])
        }
    }
}

/// Where the line from `rect`'s centre towards `towards` leaves it.
fn edge_point(rect: Rect, towards: Point) -> Point {
    let centre = rect.centre();
    let (dx, dy) = (towards.x - centre.x, towards.y - centre.y);
    if dx == 0.0 && dy == 0.0 {
        return centre;
    }
    let tx = if dx == 0.0 {
        f64::INFINITY
    } else {
        (rect.width / 2.0) / dx.abs()
    };
    let ty = if dy == 0.0 {
        f64::INFINITY
    } else {
        (rect.height / 2.0) / dy.abs()
    };
    let t = tx.min(ty).min(1.0);
    Point::new(centre.x + dx * t, centre.y + dy * t)
}

fn rect_of(element: &DiagramElement) -> Rect {
    Rect::centred(
        Point::new(element.x, element.y),
        number(element, "width"),
        number(element, "height"),
    )
}

/// The rows of a card, by group: (group, title) in the order the module listed them.
fn rows_of(element: &DiagramElement) -> Vec<(String, String)> {
    element
        .attribute("rows")
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let group = fields.next()?.to_owned();
            let _id = fields.next()?;
            let title = fields.next()?.to_owned();
            Some((group, title))
        })
        .collect()
}

/// The scene of `elements`, an agent activity view's rendering, with the elements `moving` carried by the drag in
/// flight, in the light or the `dark` mode.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    let palette = if dark { &DARK } else { &LIGHT };
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let cards: Vec<&DiagramElement> = elements
        .iter()
        .filter(|e| e.kind == ElementKind::Node && e.element_type.starts_with(PREFIX) && e.element_type != VIEW_TYPE)
        .collect();
    let rects: HashMap<String, Rect> = cards.iter().map(|e| (e.id.clone(), rect_of(e))).collect();
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
    let mut anchors = Vec::new();

    // The relations first, beneath the cards they join.
    for relation in elements.iter().filter(|e| e.element_type == RELATION_TYPE) {
        let (Some(source), Some(target)) = (
            relation.source.as_deref().and_then(|s| drawn.get(s)),
            relation.target.as_deref().and_then(|t| drawn.get(t)),
        ) else {
            continue;
        };
        let start = edge_point(*source, target.centre());
        let end = edge_point(*target, source.centre());
        let breach = relation.attribute("breach") == "true";
        let line = if selected.contains(relation.id.as_str()) {
            stroke(Some(SELECTED), SELECTED_WIDTH, &[])
        } else if breach {
            stroke(Some(palette.danger), 1.5, &[6.0, 4.0])
        } else {
            stroke(Some(palette.relation), 1.5, &[])
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: Owner::Relation(relation.id.clone()),
            path: Path::new().move_to(start).line_to(end),
            stroke: line,
            start: None,
            end: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: breach
                .then(|| "This relation breaks a rule of the diagram. The errors panel says which.".to_owned()),
        }));
        lines.push((relation.id.clone(), vec![start, end], 5.0));
    }

    for card in &cards {
        let rect = drawn[&card.id];
        let owner = Owner::Element(card.id.clone());
        let kind = &card.element_type[PREFIX.len()..];
        let opacity = if moving.is_some_and(|(ids, _)| ids.contains(&card.id)) {
            0.75
        } else {
            1.0
        };
        let status = card.attribute("status");
        let outline = if selected.contains(card.id.as_str()) {
            stroke(Some(SELECTED), SELECTED_WIDTH, &[])
        } else if kind == "specification" {
            stroke(Some(status_colour(status, dark)), 2.0, &[])
        } else {
            stroke(Some(palette.border), 1.5, &[])
        };
        primitives.push(shape(
            &owner,
            Role::Body,
            silhouette(kind, rect),
            Some(fill(kind, dark)),
            Some(outline),
            opacity,
        ));
        let inner = rect.width - 24.0;
        primitives.push(text(
            &owner,
            label("name"),
            fitted(card.attribute("name"), inner, 12.0),
            Point::new(rect.centre().x, rect.y + 20.0),
            TextAnchor::Middle,
            font(12.0, 600, Some(palette.text)),
        ));
        labels.push((
            card.id.clone(),
            if kind == "location" { "branch" } else { "name" }.to_owned(),
            Rect::new(rect.x, rect.y + 6.0, rect.width, 20.0),
        ));
        let mut y = rect.y + 40.0;
        match kind {
            "specification" => {
                primitives.push(text(
                    &owner,
                    label("status"),
                    card.attribute("statusLabel").to_owned(),
                    Point::new(rect.centre().x, rect.y + 40.0),
                    TextAnchor::Middle,
                    font(11.0, 600, Some(status_colour(status, dark))),
                ));
                y = rect.y + 64.0;
            }
            "environment" => {
                primitives.push(text(
                    &owner,
                    label("environmentKind"),
                    card.attribute("kindLabel").to_owned(),
                    Point::new(rect.centre().x, rect.y + 38.0),
                    TextAnchor::Middle,
                    font(11.0, 400, Some(palette.muted)),
                ));
            }
            "location" => {
                primitives.push(shape(
                    &owner,
                    Role::Part("separator".to_owned()),
                    Path::new()
                        .move_to(Point::new(rect.x, rect.y + 30.0))
                        .line_to(Point::new(rect.right(), rect.y + 30.0)),
                    None,
                    Some(stroke(Some(palette.border), 1.0, &[])),
                    opacity,
                ));
                primitives.push(text(
                    &owner,
                    label("folder"),
                    fitted(card.attribute("folder"), inner, 11.0),
                    Point::new(rect.centre().x, rect.y + 46.0),
                    TextAnchor::Middle,
                    font(11.0, 400, Some(palette.muted)),
                ));
                y = rect.y + 64.0 + ROW_HEIGHT;
            }
            _ => {}
        }

        // The lists: each group under its heading with its count, a folded one as its heading alone.
        let collapsed: HashSet<&str> = card
            .attribute("collapsed")
            .split(',')
            .filter(|g| !g.is_empty())
            .collect();
        let rows = rows_of(card);
        let mut groups: Vec<&str> = Vec::new();
        for (group, _) in &rows {
            if !groups.contains(&group.as_str()) {
                groups.push(group);
            }
        }
        for group in groups {
            if y + HEADING_HEIGHT > rect.bottom() + 0.5 {
                break;
            }
            let titles: Vec<&str> = rows
                .iter()
                .filter(|(g, _)| g == group)
                .map(|(_, t)| t.as_str())
                .collect();
            let folded = collapsed.contains(group);
            primitives.push(text(
                &owner,
                label(&format!("heading:{group}")),
                format!(
                    "{} {} ({})",
                    if folded { "▸" } else { "▾" },
                    heading(group),
                    titles.len()
                ),
                Point::new(rect.x + 12.0, y + 15.0),
                TextAnchor::Start,
                font(11.0, 600, Some(palette.text)),
            ));
            y += HEADING_HEIGHT;
            if folded {
                continue;
            }
            for (index, title) in titles.iter().enumerate() {
                if y + ROW_HEIGHT > rect.bottom() + 0.5 {
                    break;
                }
                primitives.push(text(
                    &owner,
                    label(&format!("row:{group}:{index}")),
                    fitted(title, inner - 12.0, 11.0),
                    Point::new(rect.x + 24.0, y + 13.0),
                    TextAnchor::Start,
                    font(11.0, 400, Some(palette.text)),
                ));
                y += ROW_HEIGHT;
            }
        }

        // A locked element's lock, at its top-left.
        if card.attribute("pinned") == "true" {
            let at = Point::new(rect.x + 10.0, rect.y + 10.0);
            primitives.push(shape(
                &owner,
                Role::Decoration("lock".to_owned()),
                Path::rounded_rect(Rect::new(at.x - 4.0, at.y - 1.0, 8.0, 6.0), 1.0),
                Some(palette.text),
                None,
                opacity,
            ));
            primitives.push(shape(
                &owner,
                Role::Decoration("lockShackle".to_owned()),
                Path::new()
                    .move_to(Point::new(at.x - 2.5, at.y - 1.0))
                    .line_to(Point::new(at.x - 2.5, at.y - 3.5))
                    .arc_to(2.5, 2.5, 0.0, false, true, Point::new(at.x + 2.5, at.y - 3.5))
                    .line_to(Point::new(at.x + 2.5, at.y - 1.0)),
                None,
                Some(stroke(Some(palette.text), 1.2, &[])),
                opacity,
            ));
        }

        if selected.contains(card.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        // A relation is drawn from the middle of any edge.
        for (name, at) in [
            ("n", Point::new(rect.centre().x, rect.y)),
            ("e", Point::new(rect.right(), rect.centre().y)),
            ("s", Point::new(rect.centre().x, rect.bottom())),
            ("w", Point::new(rect.x, rect.centre().y)),
        ] {
            let area = Rect::centred(at, 2.0 * ANCHOR_RADIUS, 2.0 * ANCHOR_RADIUS);
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Anchor {
                    name: name.to_owned(),
                    visible: false,
                },
                area,
            }));
            anchors.push((card.id.clone(), name.to_owned(), area));
        }
        boxes.push((card.id.clone(), rect));
    }

    let empty_text = cards.is_empty().then(|| {
        "This activity file has no element yet. Drop one from the toolbox, or let an agent write it.".to_owned()
    });
    ModuleScene {
        scene: Scene { primitives, empty_text },
        top_left: Vec::new(),
        rects,
        boxes,
        lines,
        labels,
        anchors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_leaves_a_card_at_its_edge() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(Point::new(100.0, 25.0), edge_point(rect, Point::new(300.0, 25.0)));
        assert_eq!(Point::new(50.0, 50.0), edge_point(rect, Point::new(50.0, 400.0)));
    }

    #[test]
    fn a_long_name_is_cut_with_an_ellipsis() {
        assert_eq!("short", fitted("short", 200.0, 12.0));
        assert!(fitted(&"x".repeat(80), 66.0, 12.0).ends_with('…'));
    }
}

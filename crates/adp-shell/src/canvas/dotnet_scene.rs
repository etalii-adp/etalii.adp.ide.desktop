//! The scene of a .NET dependency graph, which its module lays out and hands the session as boxes and references
//! (etalii.adp spec 030, task T082), drawn as `adp-scene` primitives so the one painter, hit test and gesture arbiter
//! serve it as they serve every other canvas.
//!
//! Ported from what the standalone host's
//! `src/diagrams/dotnet-dependency-graph/client/DotNetDependencyGraphCanvas.tsx`, `dotnetBindings.ts` and
//! `dotnet-dependency-graph.css` at `8b6ccfc` drew through the notation of `dotnet-dependency-graph.dis`, in its
//! theme's colours, light or dark:
//! - every box is 220 by 56 with rounded corners, its name in the middle in bold and its second line 14 below in a
//!   smaller face: a project's target frameworks, a package's versions; a project is solid, a package lighter and
//!   dashed, and a package the projects disagree about the version of is outlined in the warning colour;
//! - a reference runs out of its source's right side into its target's left side as a horizontal curve, the long way
//!   round when its target lies behind it, with a filled arrowhead; a project reference is solid, a package reference
//!   dashed;
//! - an ambient package is not drawn, nor are the references to it, and the canvas says above the diagram how many it
//!   hid and which (`canvas.filtered`); the "Show them" switch beside that notice is not ported.
//!
//! Every box is moved by its top-left, which is what the registration stores (`onElementMoved`).

use std::collections::{HashMap, HashSet};

use adp_diagram::{DiagramElement, ElementKind};
use adp_diagram_dotnet_dependency_graph::mapper::{EDGE_TYPE, PACKAGE_TYPE};
use adp_scene::definition::labels::TextAnchor;
use adp_scene::notation::catalog::MarkerKind;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive, marker_shape};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};

use super::module_scene::{AVERAGE_ADVANCE, ModuleScene, SELECTED, SELECTED_WIDTH, font, number, text};

/// What the canvas says of a solution with no project it could resolve (`DotNetDependencyGraphCanvas`).
const EMPTY: &str = "This solution has no projects ADP could resolve. Any reason is reported in the problems panel.";

/// The theme's tokens (`notation.theme`).
struct Palette {
    project_fill: &'static str,
    project_stroke: &'static str,
    package_fill: &'static str,
    package_stroke: &'static str,
    label: &'static str,
    sublabel: &'static str,
    edge_project: &'static str,
    edge_package: &'static str,
    warning: &'static str,
}

const LIGHT: Palette = Palette {
    project_fill: "#3b6ea5",
    project_stroke: "#2b537c",
    package_fill: "#cbd5e1",
    package_stroke: "#94a3b8",
    label: "#ffffff",
    sublabel: "#dbe6f2",
    edge_project: "#64748b",
    edge_package: "#94a3b8",
    warning: "#b45309",
};

const DARK: Palette = Palette {
    project_fill: "#3b6ea5",
    project_stroke: "#7fb0e0",
    package_fill: "#4a5568",
    package_stroke: "#6b7280",
    label: "#ffffff",
    sublabel: "#cbd5e1",
    edge_project: "#94a3b8",
    edge_package: "#64748b",
    warning: "#fbbf24",
};

fn stroke(colour: &str, width: f64, dash: &[f64]) -> Stroke {
    Stroke {
        colour: Some(Colour::value(colour)),
        width,
        dash: dash.to_vec(),
        opacity: 1.0,
    }
}

/// Where a box is drawn: its top-left and the size the module gave it.
pub fn rect_of(element: &DiagramElement) -> Rect {
    Rect::new(
        element.x,
        element.y,
        number(element, "width"),
        number(element, "height"),
    )
}

/// `text` whole when it fits `width` at `size`, else cut with an ellipsis (`overflow: ellipsis`, `maxLines: 1`).
fn fitted(text: &str, width: f64, size: f64) -> String {
    let capacity = ((width / (size * AVERAGE_ADVANCE)).floor() as usize).max(1);
    let characters: Vec<char> = text.chars().collect();
    if characters.len() <= capacity {
        return text.to_owned();
    }
    let kept: String = characters[..capacity.saturating_sub(1)].iter().collect();
    format!("{kept}…")
}

fn is_ambient(element: &DiagramElement) -> bool {
    element.element_type == PACKAGE_TYPE && element.attribute("isAmbient") == "true"
}

/// The scene of `elements`, a .NET dependency graph's rendering, with `selection` outlined, the elements `moving`
/// carried by the drag in flight, in the dark palette when `dark`.
pub fn build(
    elements: &[DiagramElement],
    selection: &[String],
    moving: Option<(&[String], (f64, f64))>,
    dark: bool,
) -> ModuleScene {
    let palette = if dark { &DARK } else { &LIGHT };
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let hidden: Vec<&DiagramElement> = elements.iter().filter(|e| is_ambient(e)).collect();
    let hidden_ids: HashSet<&str> = hidden.iter().map(|e| e.id.as_str()).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements
        .iter()
        .filter(|e| e.kind == ElementKind::Node && !hidden_ids.contains(e.id.as_str()))
    {
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

    // The references first, beneath the boxes they join; one to a hidden package goes with it.
    for reference in elements.iter().filter(|e| e.element_type == EDGE_TYPE) {
        let (Some(source), Some(target)) = (
            reference.source.as_deref().and_then(|s| drawn.get(s)),
            reference.target.as_deref().and_then(|t| drawn.get(t)),
        ) else {
            continue;
        };
        let owner = Owner::Relation(reference.id.clone());
        let from = Point::new(source.right(), source.centre().y);
        let to = Point::new(target.x, target.centre().y);
        // A reference pointing back the way it came goes the long way round, or the curve doubles back through its
        // own source (`dependencyRoute`): its control points reach out of the source's right side and into the
        // target's left side (`forwardBezierPath`); otherwise both sit at the horizontal midpoint
        // (`horizontalBezierPath`).
        let loops_back = target.x < source.x + source.width;
        let (out, into) = if loops_back {
            let span = to.x - from.x;
            let reach = (span / 2.0).max(40.0).max(-span * 0.6);
            (from.x + reach, to.x - reach)
        } else {
            let middle = (from.x + to.x) / 2.0;
            (middle, middle)
        };
        let path = Path::new()
            .move_to(from)
            .cubic_to(Point::new(out, from.y), Point::new(into, to.y), to);
        let is_package = reference.attribute("kind") == "PackageReference";
        let (colour, dash): (&str, &[f64]) = if is_package {
            (palette.edge_package, &[5.0, 4.0])
        } else {
            (palette.edge_project, &[])
        };
        let line = if selected.contains(reference.id.as_str()) {
            stroke(SELECTED, SELECTED_WIDTH, dash)
        } else {
            stroke(colour, 1.5, dash)
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner,
            path,
            end: marker_shape(MarkerKind::Arrow, to, 0.0, line.width),
            stroke: line,
            start: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: Some(reference.attribute("kind").to_owned()),
        }));
        let middle = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        lines.push((reference.id.clone(), vec![from, middle, to], 5.0));
    }

    // The boxes.
    for node in elements
        .iter()
        .filter(|e| e.kind == ElementKind::Node && !hidden_ids.contains(e.id.as_str()))
    {
        let Some(rect) = drawn.get(&node.id).copied() else {
            continue;
        };
        let owner = Owner::Element(node.id.clone());
        let is_package = node.element_type == PACKAGE_TYPE;
        let conflict = node.attribute("hasVersionConflict") == "true";
        let (fill, outline) = match (is_package, conflict) {
            (true, true) => (palette.package_fill, stroke(palette.warning, 2.5, &[])),
            (true, false) => (palette.package_fill, stroke(palette.package_stroke, 1.5, &[4.0, 3.0])),
            _ => (palette.project_fill, stroke(palette.project_stroke, 1.5, &[])),
        };
        let outline = if selected.contains(node.id.as_str()) {
            stroke(SELECTED, SELECTED_WIDTH, &outline.dash)
        } else {
            outline
        };
        let moved = moving.is_some_and(|(ids, _)| ids.contains(&node.id));
        primitives.push(Primitive::Shape(ShapePrimitive {
            owner: owner.clone(),
            role: Role::Body,
            path: Path::rounded_rect(rect, 6.0),
            fill: Some(Colour::value(fill)),
            stroke: Some(outline),
            opacity: if moved { 0.75 } else { 1.0 },
            tooltip: Some(title(node)),
        }));
        let subtitle = if is_package {
            node.attribute("versions")
        } else {
            node.attribute("targetFrameworks")
        };
        let name_y = if subtitle.is_empty() {
            rect.centre().y + 4.5
        } else {
            rect.centre().y - 2.0
        };
        primitives.push(text(
            &owner,
            Role::Label {
                id: "name".to_owned(),
                line: 0,
            },
            fitted(node.attribute("name"), rect.width - 16.0, 13.0),
            Point::new(rect.centre().x, name_y),
            TextAnchor::Middle,
            font(13.0, 600, Some(palette.label)),
        ));
        labels.push((node.id.clone(), "name".to_owned(), rect));
        if !subtitle.is_empty() {
            primitives.push(text(
                &owner,
                Role::Label {
                    id: "subtitle".to_owned(),
                    line: 0,
                },
                fitted(subtitle, rect.width - 16.0, 11.0),
                Point::new(rect.centre().x, rect.centre().y + 14.0),
                TextAnchor::Middle,
                font(11.0, 400, Some(palette.sublabel)),
            ));
        }
        if selected.contains(node.id.as_str()) {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((node.id.clone(), rect));
    }

    // Filtering that cannot be seen is a wrong diagram: the canvas says how many it hid, and which.
    if !hidden.is_empty() {
        let top_left = boxes
            .iter()
            .map(|(_, r)| *r)
            .reduce(|a, b| a.union(&b))
            .map_or(Point::new(0.0, 0.0), |r| Point::new(r.x, r.y));
        let names: Vec<&str> = hidden.iter().map(|e| e.attribute("name")).collect();
        let count = if hidden.len() == 1 {
            "1 package is".to_owned()
        } else {
            format!("{} packages are", hidden.len())
        };
        primitives.push(text(
            &Owner::Canvas,
            Role::Label {
                id: "filtered".to_owned(),
                line: 0,
            },
            format!(
                "{count} hidden as ambient — referenced by so many of this solution's projects that the edge tells you nothing: {}.",
                names.join(", ")
            ),
            Point::new(top_left.x, top_left.y - 24.0),
            TextAnchor::Start,
            font(12.0, 400, None),
        ));
    }

    let any_project = elements.iter().any(|e| e.kind == ElementKind::Node);
    ModuleScene {
        scene: Scene {
            primitives,
            empty_text: (!any_project).then(|| EMPTY.to_owned()),
        },
        top_left: rects.keys().cloned().collect(),
        rects,
        boxes,
        lines,
        labels,
        anchors: Vec::new(),
    }
}

/// What a box says about itself, as its tooltip (the definition's `title(self)`).
fn title(node: &DiagramElement) -> String {
    if node.element_type != PACKAGE_TYPE {
        return format!("Project {}", node.attribute("name"));
    }
    let versions = node.attribute("versions");
    let mut title = format!("Package {}", node.attribute("name"));
    if !versions.is_empty() {
        title.push_str(&format!(" ({versions})"));
    }
    if node.attribute("hasVersionConflict") == "true" {
        title.push_str(" - referenced at more than one version");
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(id: &str, ambient: bool) -> DiagramElement {
        let mut element = DiagramElement::node(id, PACKAGE_TYPE);
        element.attributes = vec![
            ("name".to_owned(), id.trim_start_matches("package:").to_owned()),
            ("versions".to_owned(), "1.0.0".to_owned()),
            ("isAmbient".to_owned(), ambient.to_string()),
            ("width".to_owned(), "220".to_owned()),
            ("height".to_owned(), "56".to_owned()),
        ];
        element
    }

    #[test]
    fn an_ambient_package_is_hidden_and_the_canvas_says_so() {
        let built = build(
            &[package("package:xunit.v3", true), package("package:Serilog", false)],
            &[],
            None,
            false,
        );
        assert!(built.rects.contains_key("package:Serilog"));
        assert!(!built.rects.contains_key("package:xunit.v3"));
        let notice = built.scene.primitives.iter().any(|p| {
            matches!(p, Primitive::Text(t) if t.text.starts_with("1 package is hidden as ambient") && t.text.ends_with("xunit.v3."))
        });
        assert!(notice);
    }

    #[test]
    fn a_title_names_the_versions_and_the_conflict() {
        let mut serilog = package("package:Serilog", false);
        serilog
            .attributes
            .push(("hasVersionConflict".to_owned(), "true".to_owned()));
        assert_eq!(
            title(&serilog),
            "Package Serilog (1.0.0) - referenced at more than one version"
        );
    }
}

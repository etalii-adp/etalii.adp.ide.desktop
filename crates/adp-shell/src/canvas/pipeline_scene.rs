//! The scene of an Azure DevOps pipeline, which its module lays out and hands the session as stages, the jobs of the
//! open ones, the steps of the open jobs and the arrows of what waits for what (etalii.adp spec 030, task T078),
//! drawn as `adp-scene` primitives so the one painter, hit test and gesture arbiter serve it as they serve every other
//! canvas.
//!
//! Ported from what the standalone host's `src/diagrams/azure-devops-pipeline/client/PipelineCanvas.tsx`,
//! `pipelineIndicators.ts` and `azure-pipeline.css` at `8b6ccfc` draw:
//! - a stage is a rounded box beneath everything inside it, its name 24 below its top at 14 and semibold, and while
//!   closed its job count 44 below; a job, a step or a template it could not follow is a plain box with its name
//!   inset 10 at 12, a deployment job green-edged and a template dashed;
//! - an element that exists or runs by an expression's say is edged in dashes, one from a template in dots;
//! - the badges each wears run leftwards from its top-right corner: disabled, manual, a condition, continue on
//!   error, how many run, from a template;
//! - an arrow runs from the right of what is waited for to the left of what waits, a bezier with a reach of 30 and an
//!   arrowhead; one nobody wrote down is dashed and fainter.
//!
//! Nothing is dragged: a pipeline arranges itself from its dependencies (`dragging: "disabled"`).

use std::collections::{HashMap, HashSet};

use adp_diagram::DiagramElement;
use adp_diagram_azure_devops_pipeline::mapper::{EDGE_TYPE, JOB_TYPE, STAGE_TYPE, STEP_TYPE, TEMPLATE_TYPE};
use adp_diagram_azure_devops_pipeline::theme::{Palette, palette};
use adp_scene::definition::labels::TextAnchor;
use adp_scene::scene::{EdgePrimitive, Hook, HookKind, Owner, Primitive, Role, ShapePrimitive};
use adp_scene::style::{Colour, Stroke};
use adp_scene::{Path, Point, Rect, Scene};
use adp_theme::Kind;

use super::module_scene::{ModuleScene, SELECTED_WIDTH, font, number, selection_colour, stroke, text};

/// The fixed reach of an arrow's control points.
const EDGE_REACH: f64 = 30.0;

fn rect_of(element: &DiagramElement) -> Rect {
    Rect::centred(
        Point::new(element.x, element.y),
        number(element, "width"),
        number(element, "height"),
    )
}

fn shape(owner: &Owner, role: Role, path: Path, fill: Option<&str>, outline: Option<Stroke>) -> Primitive {
    Primitive::Shape(ShapePrimitive {
        owner: owner.clone(),
        role,
        path,
        fill: fill.map(Colour::value),
        stroke: outline,
        opacity: 1.0,
        tooltip: None,
    })
}

fn label(
    owner: &Owner,
    id: &str,
    value: String,
    at: Point,
    anchor: TextAnchor,
    (size, weight, colour): (f64, u16, &str),
) -> Primitive {
    text(
        owner,
        Role::Label {
            id: id.to_owned(),
            line: 0,
        },
        value,
        at,
        anchor,
        font(size, weight, Some(colour)),
    )
}

/// The outline of an element: selected, a deployment's, or a plain one, dashed where it is uncertain or borrowed.
fn outline_of(element: &DiagramElement, selected: bool, palette: &Palette, highlight: &str) -> Stroke {
    let dash: &[f64] = if element.element_type == TEMPLATE_TYPE {
        &[6.0, 4.0]
    } else if element.attribute("fromTemplate") == "true" {
        &[2.0, 3.0]
    } else if element.attribute("indeterminate") == "true" {
        &[4.0, 3.0]
    } else {
        &[]
    };
    if selected {
        stroke(Some(highlight), SELECTED_WIDTH, dash)
    } else if element.attribute("kind") == "deploymentJob" {
        stroke(Some(palette.deployment_edge), 2.0, dash)
    } else {
        stroke(Some(palette.border), 1.0, dash)
    }
}

/// The badges `element` wears, as the mapper wrote them: `glyph|title`, one per line.
fn badges(element: &DiagramElement) -> Vec<(String, String)> {
    element
        .attribute("indicators")
        .lines()
        .filter_map(|line| line.split_once('|'))
        .map(|(glyph, title)| (glyph.to_owned(), title.to_owned()))
        .collect()
}

/// The glyphs of `pipelineIndicators.ts` the canvas's fonts have no character for, as paths centred on `centre`:
/// disabled (a slashed circle), manual (a triangle pointing right) and from a template (an arrow into a corner).
fn glyph_shape(glyph: &str, centre: Point) -> Option<Path> {
    let (x, y) = (centre.x, centre.y);
    match glyph {
        "⊘" => Some(
            Path::ellipse(Rect::centred(centre, 10.0, 10.0))
                .move_to(Point::new(x + 3.5, y - 3.5))
                .line_to(Point::new(x - 3.5, y + 3.5)),
        ),
        "▶" => Some(Path::polygon(&[
            Point::new(x - 4.0, y - 5.0),
            Point::new(x + 5.0, y),
            Point::new(x - 4.0, y + 5.0),
        ])),
        "⇱" => Some(
            Path::polyline(&[
                Point::new(x - 5.0, y + 1.0),
                Point::new(x - 5.0, y - 5.0),
                Point::new(x + 1.0, y - 5.0),
            ])
            .move_to(Point::new(x - 5.0, y - 5.0))
            .line_to(Point::new(x + 5.0, y + 5.0)),
        ),
        _ => None,
    }
}

/// The scene of `elements`, a pipeline view's rendering, with `selection` outlined, in the palette of the active
/// theme, `theme`.
pub fn build(elements: &[DiagramElement], selection: &[String], theme: Kind) -> ModuleScene {
    let palette = palette(theme);
    let highlight = selection_colour(theme);
    let selected: HashSet<&str> = selection.iter().map(String::as_str).collect();
    let mut rects: HashMap<String, Rect> = HashMap::new();
    for element in elements.iter().filter(|e| e.element_type != EDGE_TYPE) {
        rects.insert(element.id.clone(), rect_of(element));
    }

    let mut primitives = Vec::new();
    let mut boxes = Vec::new();
    let mut lines = Vec::new();
    let mut labels = Vec::new();

    // Stages first, beneath their jobs and the arrows between them.
    let ordered = elements.iter().filter(|e| e.element_type == STAGE_TYPE).chain(
        elements
            .iter()
            .filter(|e| matches!(e.element_type.as_str(), JOB_TYPE | STEP_TYPE | TEMPLATE_TYPE)),
    );
    for element in ordered {
        let rect = rects[&element.id];
        let owner = Owner::Element(element.id.clone());
        let is_stage = element.element_type == STAGE_TYPE;
        let is_selected = selected.contains(element.id.as_str());
        let fill = if element.element_type == TEMPLATE_TYPE {
            None
        } else if is_stage {
            Some(palette.stage_fill)
        } else if element.attribute("kind") == "deploymentJob" {
            Some(palette.deployment_fill)
        } else {
            Some(palette.surface)
        };
        let mut body = shape(
            &owner,
            Role::Body,
            Path::rounded_rect(rect, if is_stage { 6.0 } else { 0.0 }),
            fill,
            Some(outline_of(element, is_selected, palette, highlight)),
        );
        if let Primitive::Shape(s) = &mut body {
            let reason = element.attribute("unresolvedReason");
            s.tooltip = (!reason.is_empty()).then(|| reason.to_owned());
            if element.attribute("fromTemplate") == "true" {
                s.opacity = 0.85;
            }
        }
        primitives.push(body);
        let name = element.attribute("displayName").to_owned();
        if is_stage {
            primitives.push(label(
                &owner,
                "name",
                name,
                Point::new(rect.x + 12.0, rect.y + 24.0),
                TextAnchor::Start,
                (14.0, 600, palette.text),
            ));
            labels.push((
                element.id.clone(),
                "name".to_owned(),
                Rect::new(rect.x, rect.y + 6.0, rect.width, 24.0),
            ));
            if element.attribute("expanded") != "true" {
                let count: usize = element.attribute("jobCount").parse().unwrap_or_default();
                primitives.push(label(
                    &owner,
                    "count",
                    format!("{count} {}", if count == 1 { "job" } else { "jobs" }),
                    Point::new(rect.x + 12.0, rect.y + 44.0),
                    TextAnchor::Start,
                    (12.0, 400, palette.muted),
                ));
            }
        } else {
            // A job open on its steps keeps its name in its header; a closed one, a step or a template centres it.
            let y = if element.attribute("expanded") == "true" {
                rect.y + 24.0
            } else {
                rect.centre().y + 4.0
            };
            primitives.push(label(
                &owner,
                "name",
                name,
                Point::new(rect.x + 10.0, y),
                TextAnchor::Start,
                (12.0, 400, palette.text),
            ));
            labels.push((
                element.id.clone(),
                "name".to_owned(),
                Rect::new(rect.x, y - 16.0, rect.width, 22.0),
            ));
        }
        let (inset, from_top) = if is_stage { (12.0, 20.0) } else { (8.0, 16.0) };
        for (index, (glyph, title)) in badges(element).into_iter().enumerate() {
            let at = Point::new(rect.right() - inset - 16.0 * index as f64, rect.y + from_top);
            let role = format!("indicator-{index}");
            let drawn = match glyph_shape(&glyph, Point::new(at.x - 5.0, at.y - 4.0)) {
                // A glyph the canvas's fonts lack is drawn as the shape it is.
                Some(path) => shape(
                    &owner,
                    Role::Decoration(role),
                    path,
                    None,
                    Some(stroke(Some(palette.muted), 1.2, &[])),
                ),
                None => label(&owner, &role, glyph, at, TextAnchor::End, (11.0, 400, palette.muted)),
            };
            primitives.push(match drawn {
                Primitive::Shape(mut s) => {
                    s.tooltip = Some(title);
                    Primitive::Shape(s)
                }
                Primitive::Text(mut run) => {
                    run.tooltip = Some(title);
                    Primitive::Text(run)
                }
                other => other,
            });
        }
        if is_selected {
            primitives.push(Primitive::Hook(Hook {
                owner: owner.clone(),
                kind: HookKind::Selection,
                area: rect,
            }));
        }
        boxes.push((element.id.clone(), rect));
    }

    // The arrows, over the stages they join.
    for edge in elements.iter().filter(|e| e.element_type == EDGE_TYPE) {
        let (Some(source), Some(target)) = (
            edge.source.as_deref().and_then(|s| rects.get(s)),
            edge.target.as_deref().and_then(|t| rects.get(t)),
        ) else {
            continue;
        };
        let owner = Owner::Relation(edge.id.clone());
        let from = Point::new(source.right(), source.centre().y);
        let to = Point::new(target.x, target.centre().y);
        let (c1, c2) = (
            Point::new(from.x + EDGE_REACH, from.y),
            Point::new(to.x - EDGE_REACH, to.y),
        );
        let path = Path::new().move_to(from).cubic_to(c1, c2, to);
        let implicit = edge.attribute("implicitDependency") == "true";
        let broken = edge.attribute("broken") == "true";
        let colour = if selected.contains(edge.id.as_str()) {
            highlight
        } else if broken {
            palette.broken
        } else {
            palette.edge
        };
        let dash = if broken {
            vec![2.0, 4.0]
        } else if implicit {
            vec![3.0, 3.0]
        } else {
            Vec::new()
        };
        primitives.push(Primitive::Edge(EdgePrimitive {
            owner: owner.clone(),
            path,
            stroke: Stroke {
                colour: Some(Colour::value(colour)),
                width: 1.5,
                dash,
                opacity: if implicit { 0.7 } else { 1.0 },
            },
            start: None,
            end: None,
            hit_width: 10.0,
            selectable: true,
            tooltip: Some(edge.attribute("condition").to_owned()).filter(|c| !c.is_empty()),
        }));
        primitives.push(shape(
            &owner,
            Role::Decoration("head".to_owned()),
            Path::new()
                .move_to(to)
                .line_to(Point::new(to.x - 8.0, to.y - 4.5))
                .line_to(Point::new(to.x - 8.0, to.y + 4.5))
                .close(),
            Some(colour),
            None,
        ));
        let sampled: Vec<Point> = (0..=12)
            .map(|step| {
                let t = f64::from(step) / 12.0;
                let u = 1.0 - t;
                Point::new(
                    u * u * u * from.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * to.x,
                    u * u * u * from.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * to.y,
                )
            })
            .collect();
        lines.push((edge.id.clone(), sampled, 5.0));
    }

    let empty_text = (!elements.iter().any(|e| e.element_type == STAGE_TYPE))
        .then(|| "This pipeline declares nothing to draw.".to_owned());
    ModuleScene {
        scene: Scene { primitives, empty_text },
        rects,
        boxes,
        lines,
        labels,
        anchors: Vec::new(),
        top_left: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges_are_read_from_the_mappers_lines() {
        let mut element = DiagramElement::node("Build", STAGE_TYPE);
        element.attributes.push((
            "indicators".into(),
            "⊘|Disabled: this will not run.\n?|Runs only when: always()".into(),
        ));
        assert_eq!(
            vec![
                ("⊘".to_owned(), "Disabled: this will not run.".to_owned()),
                ("?".to_owned(), "Runs only when: always()".to_owned())
            ],
            badges(&element)
        );
    }
}

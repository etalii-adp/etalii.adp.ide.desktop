//! A parsed map as the elements a session hands the shell to draw (etalii.adp spec 030, task T070).
//!
//! Ported from the standalone host's `WardleyElementMapper.cs` and `WardleyElementTypes.cs` at `8b6ccfc`. This is where
//! names become identities: a link names its ends, and goes out carrying the ids they resolve to and the names as
//! written, so a dangling end is still drawn and the rules report it. Every position is the author's own claim, read
//! from the document through [`crate::axis`], in the map's 0..1 space (`x` maturity, `y` one less visibility); there
//! is no layout. The evolution axis goes out as an element of its own carrying the four stages, so the canvas draws
//! the bands it is told about rather than constants of its own.

use std::collections::HashSet;

use adp_diagram::element::Rect;
use adp_diagram::{DiagramDelta, DiagramElement, DiagramViewport, ElementKind};

use crate::axis;
use crate::identities::{WardleyIdentityEntry, keys, kind};
use crate::lines::format_number;
use crate::model::{WardleyComponent, WardleyMap};

/// A component, anchor or submap, or a pipeline child; which one is in its `kind`.
pub const ELEMENT: &str = "wardley/map+element";
/// A link between two elements.
pub const LINK: &str = "wardley/map+link";
/// Free text at a position.
pub const NOTE: &str = "wardley/map+note";
/// A numbered annotation, carrying every position it is pinned at.
pub const ANNOTATION: &str = "wardley/map+annotation";
/// An accelerator or a deaccelerator.
pub const ACCELERATOR: &str = "wardley/map+accelerator";
/// A pioneers, settlers or town-planners region.
pub const ATTITUDE: &str = "wardley/map+attitude";
/// The map-level element carrying the evolution axis.
pub const EVOLUTION_AXIS: &str = "wardley/map+evolution-axis";
/// The id of the one evolution-axis element.
pub const EVOLUTION_AXIS_ID: &str = "wardley-evolution-axis";

fn element(id: String, (x, y): (f64, f64), element_type: &str, attributes: Vec<(&str, String)>) -> DiagramElement {
    DiagramElement {
        x,
        y,
        placed: true,
        attributes: attributes.into_iter().map(|(n, v)| (n.to_owned(), v)).collect(),
        ..DiagramElement::node(id, element_type)
    }
}

fn flag(value: bool) -> String {
    if value { "true" } else { "false" }.to_owned()
}

/// Every element of `map`, in the order the standalone host sent them: the axis, the components, the pipeline
/// children, the links, the notes, the annotations, the accelerators and the attitudes. An id given twice (two
/// components of one name) is sent once.
pub fn elements(map: &WardleyMap, identities: &[WardleyIdentityEntry]) -> Vec<DiagramElement> {
    let id = |kind: &str, key: &str| crate::identities::id_of(identities, kind, key);
    let mut elements = vec![evolution_axis(map)];
    for component in &map.components {
        elements.push(component_element(
            map,
            component,
            &id(kind::COMPONENT, &keys::component(component)),
        ));
    }
    for pipeline in &map.pipelines {
        let parent = map.components.iter().find(|c| c.name == pipeline.parent);
        for child in &pipeline.children {
            // A child takes its visibility from its parent: the format gives it none of its own.
            let visibility = parent.map_or(0.0, |p| p.position.visibility);
            let parent_id = parent
                .map(|p| id(kind::COMPONENT, &keys::component(p)))
                .unwrap_or_default();
            elements.push(element(
                id(kind::PIPELINE_CHILD, &keys::pipeline_child(pipeline, child)),
                (child.maturity, 1.0 - visibility),
                ELEMENT,
                vec![
                    ("name", child.name.clone()),
                    ("kind", "component".to_owned()),
                    ("visibility", format_number(visibility)),
                    ("maturity", format_number(child.maturity)),
                    ("stage", axis::stage_of(child.maturity).label.to_owned()),
                    ("pipelineParentId", parent_id),
                ],
            ));
        }
    }
    for link in &map.links {
        let source = id(kind::COMPONENT, &link.source);
        let target = id(kind::COMPONENT, &link.target);
        let mut drawn = element(
            id(kind::LINK, &keys::link(link)),
            (0.0, 0.0),
            LINK,
            vec![
                ("isFlow", flag(link.kind == crate::model::WardleyLinkKind::Flow)),
                ("context", link.context.clone()),
                ("sourceName", link.source.clone()),
                ("targetName", link.target.clone()),
            ],
        );
        drawn.kind = ElementKind::Relation;
        drawn.placed = false;
        drawn.source = (!source.is_empty()).then_some(source);
        drawn.target = (!target.is_empty()).then_some(target);
        elements.push(drawn);
    }
    for note in &map.notes {
        elements.push(element(
            id(kind::NOTE, &keys::note(note)),
            axis::to_point(note.position),
            NOTE,
            vec![("text", note.text.clone())],
        ));
    }
    for annotation in &map.annotations {
        let mut attributes = vec![
            ("number", annotation.number.to_string()),
            ("text", annotation.text.clone()),
        ];
        let occurrences: Vec<String> = annotation
            .occurrences
            .iter()
            .map(|o| {
                let (x, y) = axis::to_point(*o);
                format!("{} {}", format_number(x), format_number(y))
            })
            .collect();
        attributes.push(("occurrences", occurrences.join(";")));
        // The element sits at the first occurrence; the rest travel in its attributes.
        elements.push(element(
            id(kind::ANNOTATION, &keys::annotation(annotation)),
            axis::to_point(annotation.occurrences[0]),
            ANNOTATION,
            attributes,
        ));
    }
    for accelerator in &map.accelerators {
        elements.push(element(
            id(kind::ACCELERATOR, &keys::accelerator(accelerator)),
            axis::to_point(accelerator.position),
            ACCELERATOR,
            vec![
                ("name", accelerator.name.clone()),
                ("isDeaccelerator", flag(accelerator.is_deaccelerator)),
            ],
        ));
    }
    for attitude in &map.attitudes {
        let (x2, y2) = axis::to_point(attitude.to);
        elements.push(element(
            id(kind::ATTITUDE, &keys::attitude(attitude)),
            axis::to_point(attitude.from),
            ATTITUDE,
            vec![
                ("kind", attitude.kind.label().to_owned()),
                ("oppositeX", format_number(x2)),
                ("oppositeY", format_number(y2)),
            ],
        ));
    }
    let mut seen = HashSet::new();
    elements.retain(|e| !e.id.is_empty() && seen.insert(e.id.clone()));
    elements
}

fn component_element(map: &WardleyMap, component: &WardleyComponent, id: &str) -> DiagramElement {
    let mut attributes = vec![
        ("name", component.name.clone()),
        ("kind", component.kind.keyword().to_owned()),
        ("visibility", format_number(component.position.visibility)),
        ("maturity", format_number(component.position.maturity)),
        ("stage", axis::stage_of(component.position.maturity).label.to_owned()),
        ("decorators", crate::model::join_decorators(&component.decorators)),
        ("inertia", flag(component.inertia)),
        ("url", component.url.clone()),
        (
            "urlAddress",
            map.urls
                .iter()
                .find(|u| !component.url.is_empty() && u.name == component.url)
                .map(|u| u.address.clone())
                .unwrap_or_default(),
        ),
    ];
    if let Some(offset) = component.label_offset {
        attributes.push(("labelOffsetX", format_number(offset.x)));
        attributes.push(("labelOffsetY", format_number(offset.y)));
    }
    // Only the first `evolve` of a component is used, as the standalone host used it.
    if let Some(evolve) = map.evolves.iter().find(|e| e.name == component.name) {
        attributes.push(("evolveMaturity", format_number(evolve.maturity)));
        attributes.push(("evolveStage", axis::stage_of(evolve.maturity).label.to_owned()));
        attributes.push(("evolveName", evolve.override_name.clone()));
    }
    element(id.to_owned(), axis::to_point(component.position), ELEMENT, attributes)
}

fn evolution_axis(map: &WardleyMap) -> DiagramElement {
    let mut attributes = vec![
        ("title", map.title.clone()),
        ("width", format_number(map.size.map_or(0.0, |s| s.width))),
        ("height", format_number(map.size.map_or(0.0, |s| s.height))),
    ];
    let names = ["stage.0", "stage.1", "stage.2", "stage.3"];
    for (name, stage) in names.iter().zip(axis::STAGES) {
        attributes.push((name, format!("{}|{}|{}", stage.label, stage.start, stage.end)));
    }
    let mut axis = element(EVOLUTION_AXIS_ID.to_owned(), (0.0, 0.0), EVOLUTION_AXIS, attributes);
    axis.placed = false;
    axis
}

fn is_unbounded(viewport: DiagramViewport) -> bool {
    [viewport.min_x, viewport.min_y, viewport.max_x, viewport.max_y]
        .iter()
        .any(|v| v.is_infinite())
}

fn contains(viewport: DiagramViewport, (x, y): (f64, f64)) -> bool {
    viewport.intersects(Rect::new(x, y, 0.0, 0.0))
}

/// The elements `viewport` can see, in the map's 0..1 space (`WardleyElementMapper.Visible`): the axis always; a
/// component in view and the components it links to, so no link dangles; a link whose two ends are kept; an attitude
/// by the rectangle it spans; every other kind by its point.
pub fn visible(
    map: &WardleyMap,
    identities: &[WardleyIdentityEntry],
    viewport: DiagramViewport,
) -> Vec<DiagramElement> {
    let all = elements(map, identities);
    if is_unbounded(viewport) {
        return all;
    }
    let id = |kind: &str, key: &str| crate::identities::id_of(identities, kind, key);
    let mut kept: HashSet<String> = HashSet::from([EVOLUTION_AXIS_ID.to_owned()]);
    let in_view: HashSet<&str> = map
        .components
        .iter()
        .filter(|c| contains(viewport, axis::to_point(c.position)))
        .map(|c| c.name.as_str())
        .collect();
    let mut reachable = in_view.clone();
    for link in map
        .links
        .iter()
        .filter(|l| in_view.contains(l.source.as_str()) || in_view.contains(l.target.as_str()))
    {
        reachable.insert(&link.source);
        reachable.insert(&link.target);
    }
    for component in map.components.iter().filter(|c| reachable.contains(c.name.as_str())) {
        kept.insert(id(kind::COMPONENT, &keys::component(component)));
    }
    for link in map
        .links
        .iter()
        .filter(|l| reachable.contains(l.source.as_str()) && reachable.contains(l.target.as_str()))
    {
        kept.insert(id(kind::LINK, &keys::link(link)));
    }
    for attitude in &map.attitudes {
        let (x1, y1) = axis::to_point(attitude.from);
        let (x2, y2) = axis::to_point(attitude.to);
        let area = Rect::new(x1.min(x2), y1.min(y2), (x1 - x2).abs(), (y1 - y2).abs());
        if viewport.intersects(area) {
            kept.insert(id(kind::ATTITUDE, &keys::attitude(attitude)));
        }
    }
    all.into_iter()
        .filter(|e| {
            kept.contains(&e.id)
                || (e.element_type != LINK && e.element_type != EVOLUTION_AXIS && contains(viewport, (e.x, e.y)))
        })
        .collect()
}

/// A pipeline as the contract's grouping (`WardleyElementMapper.Group`): the parent stands for the children it holds,
/// one group per pipeline with children whose parent is on the map.
pub fn group(map: &WardleyMap, identities: &[WardleyIdentityEntry]) -> Vec<DiagramDelta> {
    let id = |kind: &str, key: &str| crate::identities::id_of(identities, kind, key);
    let mut deltas = Vec::new();
    for pipeline in map.pipelines.iter().filter(|p| !p.children.is_empty()) {
        // A pipeline naming a component that is not there is the rules' to report; there is nothing to group under.
        let Some(parent) = map.components.iter().find(|c| c.name == pipeline.parent) else {
            continue;
        };
        let children: Vec<String> = pipeline
            .children
            .iter()
            .map(|child| id(kind::PIPELINE_CHILD, &keys::pipeline_child(pipeline, child)))
            .filter(|id| !id.is_empty())
            .collect();
        if !children.is_empty() {
            deltas.push(DiagramDelta::Group {
                source_element_ids: children,
                group_element: component_element(map, parent, &id(kind::COMPONENT, &keys::component(parent))),
            });
        }
    }
    deltas
}

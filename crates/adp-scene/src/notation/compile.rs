//! A DISL notation compiled into a [`DiagramDefinition`] (DISL section 6; the standalone host's
//! `canvas/library/disl/compileNotation.ts` at 8b6ccfc): DISL 0.3, and of DISL 0.4 the lists inside a node and its
//! badges.
//!
//! The notation's nodes, edges, anchors, labels, sizes, placements, icons, badges, lists and stubs are mapped onto
//! the definition key by key, after the notation is resolved along the supertype chain (DISL section 4.7). A value it
//! reads and cannot draw (a shape, a marker, a label position, an anchor, an icon) is refused with an error naming
//! it, as the standalone refuses it.
//!
//! Where this compiler and the standalone's differ, the standalone's behaviour is kept and the difference is the
//! runtime's:
//!
//! - **CEL is kept, not translated.** The standalone turns each bound expression into a template over its backend's
//!   payload, through the module's `celPaths` and `celConditions`, and refuses one they do not name; this compiler
//!   keeps the expression and the scene evaluates it ([`crate::evaluation`]). So nothing is refused for being CEL: a
//!   label's text and `visible`, a line's dash, its computed stroke width (the standalone's stroke ladder), a
//!   variant's condition and a colour chosen by index are all evaluated as written.
//! - **No module bindings.** What a standalone module states beside its specification (class names, wire ids, a
//!   library shape for a custom shape, a custom route, a drag hint) has no counterpart: a custom shape is drawn from
//!   its own geometry, a style from the notation, and a `roundedRect` as the library's rounded rectangle (see
//!   [`crate::notation::catalog`]). A routing plugin is the host's to draw ([`crate::scene::RoutePlugins`]), else its
//!   declared fallback.
//! - **Drawing only.** The standalone compiler also derives the canvas's actions and their shortcuts, the snapping,
//!   the rulers, the filter box and the legend, the switches, the background menu, the layout modes and the
//!   viewpoints' variants; none of that is drawn into a scene, and the runtime's own derivations (`adp-disl`'s
//!   context menus and toolbox) state the actions.

use adp_disl::DislSpecification;
use regex::Regex;
use serde_json::{Map, Value as Json};

use super::bowed_arc::ARC_BOW;
use super::catalog::{BuiltInRoute, MarkerKind, library_marker_of, library_route_of, library_shape_of, shape_name_of};
use crate::connectors::Edge;
use crate::definition::binding::visibility;
use crate::definition::compartments::{CompartmentDeclaration, CompartmentItems, Grouping};
use crate::definition::decorations::{DecorationDeclaration, DecorationOrigin, Glyph, LinkDeclaration};
use crate::definition::labels::{AnchorTo, BEFORE_GAP, LabelDeclaration, LabelPlacement, TextAnchor};
use crate::definition::stub::{StubDeclaration, StubLabel};
use crate::definition::{
    Accessibility, AnchorKind, AnchorSet, ArcBars, ArcMark, Binding, BowedArcDeclaration, Compass, CustomShape,
    DiagramDefinition, EdgeSides, ElementType, EndAnchoring, EndpointAnchors, EndpointConstraint, Endpoints, PerPair,
    RelationType, Resize, Route, RouteLabel, RouteLabelPlacement, ShapeKind, SideFraction, SizeRule, Sizing,
};
use crate::geometry::Point;
use crate::shapes::BuiltInShape;
use crate::style::{merged_per_key, style_of};

/// Why a notation does not compile: what it states that cannot be drawn, named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError(pub String);

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "compileNotation: {}", self.0)
    }
}

impl std::error::Error for CompileError {}

type Compiled<T> = Result<T, CompileError>;

fn fail<T>(message: impl Into<String>) -> Compiled<T> {
    Err(CompileError(message.into()))
}

/// Where the first line of text inside a node sits beneath its top, and half of it above its bottom.
const LABEL_LINE: f64 = 20.0;

/// A list's own measures, which DISL leaves to the host.
const LIST_GAP: f64 = 12.0;
const LIST_HEADING: f64 = 20.0;
const LIST_ROW: f64 = 18.0;
const LIST_BOTTOM: f64 = 8.0;
const LIST_INSET: f64 = 10.0;
const LIST_INDENT: f64 = 12.0;

/// A badge's measures: its distance from the corner, a mark's radius, and a link symbol's size and top.
const BADGE_INSET: f64 = 9.0;
const BADGE_RADIUS: f64 = 3.5;
const BADGE_TOP: f64 = 5.0;
const LINK_SIZE: f64 = 11.0;

/// An icon's size where the specification states none, and the gap between a body-less node's icon and its point.
const ICON_SIZE: f64 = 24.0;
const ICON_GAP: f64 = 2.0;

/// The icons the library draws, by their DISL name.
const ICONS: [(&str, Glyph); 1] = [("mdi-sync", Glyph::CircularArrow)];

fn str_of<'a>(value: &'a Json, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Json::as_str)
}

fn num_of(value: &Json, key: &str) -> Option<f64> {
    value.get(key).and_then(Json::as_f64)
}

fn pair_of(value: Option<&Json>) -> (Option<f64>, Option<f64>) {
    match value.and_then(Json::as_array) {
        Some(items) => (
            items.first().and_then(Json::as_f64),
            items.get(1).and_then(Json::as_f64),
        ),
        None => (None, None),
    }
}

/// The compiler's state for one specification.
struct Compiler<'a> {
    specification: &'a DislSpecification,
    root: &'a Json,
    notation: Json,
    nodes: Map<String, Json>,
    edges: Map<String, Json>,
}

/// Notation looked up along the supertype chain (DISL section 4.7): an abstract type is not drawn; a type with notation
/// of its own draws it merged per key over its supertypes'; a concrete type with none draws its nearest supertype's,
/// in that supertype's place.
fn inherited(notation: &Map<String, Json>, specification: &DislSpecification, relations: bool) -> Map<String, Json> {
    let metamodel = specification.metamodel();
    let types = if relations {
        &metamodel.relations
    } else {
        &metamodel.types
    };
    let chain = |name: &str| -> Vec<String> {
        types
            .get(name)
            .map(|t| t.linearisation.clone())
            .unwrap_or_else(|| vec![name.to_owned()])
    };
    let resolved = |name: &str| -> Json {
        chain(name)
            .iter()
            .rev()
            .filter_map(|n| notation.get(n))
            .fold(None::<Json>, |merged, own| {
                Some(match merged {
                    None => own.clone(),
                    Some(merged) => merged_per_key(&merged, own),
                })
            })
            .unwrap_or(Json::Null)
    };
    let nearest = |name: &str| chain(name).into_iter().find(|n| notation.contains_key(n));
    let is_abstract = |name: &str| types.get(name).is_some_and(|t| t.is_abstract());
    let mut result = Map::new();
    for name in notation.keys() {
        if !is_abstract(name) {
            result.insert(name.clone(), resolved(name));
        }
        for other in types.keys() {
            if !is_abstract(other) && !notation.contains_key(other) && nearest(other).as_deref() == Some(name.as_str())
            {
                result.insert(other.clone(), resolved(other));
            }
        }
    }
    result
}

/// A DISL notation as a diagram definition. See this module's opening comment.
pub fn compile_notation(specification: &DislSpecification) -> Result<DiagramDefinition, CompileError> {
    let root = specification.root();
    let notation = root.get("notation").cloned().unwrap_or(Json::Object(Map::new()));
    let nodes = notation
        .get("nodes")
        .and_then(Json::as_object)
        .cloned()
        .unwrap_or_default();
    let edges = notation
        .get("edges")
        .and_then(Json::as_object)
        .cloned()
        .unwrap_or_default();
    let compiler = Compiler {
        specification,
        root,
        nodes: inherited(&nodes, specification, false),
        edges: inherited(&edges, specification, true),
        notation,
    };
    compiler.compile()
}

impl Compiler<'_> {
    fn compile(&self) -> Compiled<DiagramDefinition> {
        let element_types = self
            .nodes
            .iter()
            .map(|(name, node)| self.element_type(name, node))
            .collect::<Compiled<Vec<_>>>()?;
        let relation_types = self
            .edges
            .keys()
            .map(|name| self.relation_type(name))
            .collect::<Compiled<Vec<_>>>()?;
        let canvas = self.notation.get("canvas");
        let empty_text = match canvas.and_then(|c| c.get("empty")).and_then(|e| str_of(e, "text")) {
            Some(text) if text.contains('{') || text.contains('}') => {
                return fail(format!(
                    "the canvas's empty sentence \"{text}\" holds a brace, which the library would read as a {{placeholder}}."
                ));
            }
            other => other.map(str::to_owned),
        };
        Ok(DiagramDefinition {
            specification: self.specification.clone(),
            notation: self.notation.clone(),
            element_types,
            relation_types,
            empty_text,
            background: canvas.and_then(|c| c.get("background")).cloned(),
        })
    }

    // ---- names -----------------------------------------------------------------------------------------------------

    fn node_names(&self) -> impl Iterator<Item = &String> {
        self.nodes.keys()
    }

    fn is_a(&self, name: &str, ancestor: &str) -> bool {
        self.specification.metamodel().is_a(name, ancestor)
    }

    /// The concrete node types a metamodel type name stands for: itself, or its descendants with a notation.
    fn concrete_types(&self, name: &str) -> Vec<String> {
        self.node_names()
            .filter(|node| self.is_a(node, name))
            .cloned()
            .collect()
    }

    /// Whether a type, or a type it extends, may hold children (DISL section 4.6).
    fn holds_children(&self, name: &str) -> bool {
        self.specification
            .metamodel()
            .type_of(name)
            .is_some_and(|t| !t.child_types.is_empty() || t.json.get("children").is_some())
    }

    fn binding(&self, value: &Json, location: &str) -> Compiled<Binding> {
        match value {
            Json::Object(_) => match Binding::of(value) {
                Some(binding) => Ok(binding),
                None => fail(format!(
                    "{location} binds {value}, which is neither an attribute nor CEL."
                )),
            },
            _ => fail(format!(
                "{location} binds {value}, which is neither an attribute nor CEL."
            )),
        }
    }

    // ---- element types ---------------------------------------------------------------------------------------------

    fn element_type(&self, name: &str, node: &Json) -> Compiled<ElementType> {
        let location = format!("node \"{name}\"");
        let shape = self.shape(name, node)?;
        let bodyless = shape == ShapeKind::BuiltIn(BuiltInShape::None);
        let labels = node
            .get("labels")
            .and_then(Json::as_array)
            .map(|labels| {
                labels
                    .iter()
                    .map(|label| self.label(name, label, bodyless))
                    .collect::<Compiled<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let compartments = self.compartments(name, node)?;
        let (links, badges) = self.badges(name, node)?;
        let mut decorations = self.icon(name, node, bodyless)?;
        decorations.extend(badges);
        let (sizing, resize) = self.sizing(node)?;
        let edit_on_drop = self
            .root
            .get("toolbox")
            .and_then(|t| t.get("groups"))
            .and_then(Json::as_array)
            .is_some_and(|groups| {
                groups.iter().any(|group| {
                    group.get("tools").and_then(Json::as_array).is_some_and(|tools| {
                        tools.iter().any(|tool| {
                            str_of(tool, "creates") == Some(name) && str_of(tool, "after") == Some("editLabel")
                        })
                    })
                })
            });
        let style = node
            .get("style")
            .and_then(|s| style_of(&self.notation, s))
            .unwrap_or(Json::Object(Map::new()));
        let conditions = node
            .get("conditions")
            .and_then(Json::as_array)
            .map(|conditions| {
                conditions
                    .iter()
                    .filter_map(|condition| {
                        let when = visibility(condition.get("when"))?;
                        let style = condition.get("style").and_then(|s| style_of(&self.notation, s))?;
                        Some((when, style))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let size = node.get("size");
        let fixed = match pair_of(size.and_then(|s| s.get("fixed"))) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        };
        Ok(ElementType {
            name: name.to_owned(),
            shape,
            style,
            conditions,
            labels,
            decorations,
            compartments,
            links,
            tooltip: node
                .get("tooltip")
                .map(|t| Binding::of(t).ok_or_else(|| CompileError(format!("{location} tooltip binds {t}."))))
                .transpose()?,
            accessibility: node.get("accessibility").map(|a| Accessibility {
                role: str_of(a, "role").map(str::to_owned),
                name: a.get("name").and_then(Binding::of).or_else(|| self.default_name(name)),
            }),
            anchors: self.anchors(name, node)?,
            sizing,
            resize,
            draggable: self.movable(node),
            deletable: node.get("deletable") != Some(&Json::Bool(false)),
            edit_on_drop,
            centre_of: self.centre_of(name, node)?,
            size: SizeRule {
                default: pair_of(size.and_then(|s| s.get("default"))),
                fixed,
                min: pair_of(size.and_then(|s| s.get("min"))),
                max: pair_of(size.and_then(|s| s.get("max"))),
            },
        })
    }

    /// DISL's default accessible name (section 6.15): the type's label and its label attribute.
    fn default_name(&self, name: &str) -> Option<Binding> {
        let declared = self.specification.metamodel().type_of(name)?;
        let label = str_of(&declared.json, "label").unwrap_or(name).replace('\'', "\\'");
        Some(match &declared.label_attribute {
            Some(attribute) => Binding::Cel(format!("'{label} ' + string(self.{attribute})")),
            None => Binding::Literal(label),
        })
    }

    fn shape(&self, name: &str, node: &Json) -> Compiled<ShapeKind> {
        let reference = node.get("shape").cloned().unwrap_or(Json::String("rect".to_owned()));
        let (shape_name, params) = shape_name_of(&reference);
        if let Some(custom) = self.notation.get("shapes").and_then(|s| s.get(&shape_name)) {
            return Ok(ShapeKind::Custom(CustomShape {
                name: shape_name,
                declaration: custom.clone(),
                params,
            }));
        }
        match library_shape_of(&reference) {
            Ok(Some(shape)) => Ok(ShapeKind::BuiltIn(shape)),
            Ok(None) => fail(format!(
                "node \"{name}\" is drawn as \"{shape_name}\", which is no built-in the library draws and no custom shape of the notation."
            )),
            Err(refusal) => fail(refusal),
        }
    }

    fn label(&self, type_name: &str, label: &Json, bodyless: bool) -> Compiled<LabelDeclaration> {
        let id = str_of(label, "id").unwrap_or("");
        let location = format!("node \"{type_name}\" label \"{id}\"");
        let text = label.get("text").unwrap_or(&Json::Null);
        let mut declaration = LabelDeclaration::new(id, self.binding(text, &location)?);
        let position = label.get("position");
        let distance = num_of(label, "distance");
        match position {
            Some(Json::String(p)) if bodyless && p == "bottom" => {
                // A body-less node has no box to write at the bottom of: its last line is on its point.
            }
            Some(Json::String(p)) if p == "outside-left" => {
                if let Some(d) = distance.filter(|d| *d != BEFORE_GAP) {
                    return fail(format!(
                        "{location} sits {d} before its node; the library's before label sits {BEFORE_GAP}."
                    ));
                }
                declaration.placement = LabelPlacement::Before;
            }
            Some(Json::String(p)) if p == "outside-right" => {
                if let Some(d) = distance {
                    return fail(format!(
                        "{location} sits {d} beside its node; the library's beside label sits at its own gap."
                    ));
                }
                declaration.placement = LabelPlacement::Beside;
            }
            Some(Json::Object(p)) => {
                let (ax, ay) = pair_of(p.get("anchor"));
                let (ax, ay) = (ax.unwrap_or(0.5), ay.unwrap_or(0.5));
                let anchor_to = match ay {
                    0.0 => Some(AnchorTo::Top),
                    0.5 => Some(AnchorTo::Centre),
                    1.0 => Some(AnchorTo::Bottom),
                    _ => None,
                };
                let edge = match ax {
                    0.0 => Some(TextAnchor::Start),
                    1.0 => Some(TextAnchor::End),
                    _ => None,
                };
                let Some(anchor_to) = anchor_to.filter(|_| ax == 0.5 || edge.is_some()) else {
                    return fail(format!(
                        "{location} anchors at {}; the library anchors a label at the top, centre or bottom of the middle or of an edge.",
                        p.get("anchor").unwrap_or(&Json::Null)
                    ));
                };
                let align = p.get("align").and_then(Json::as_str);
                if let Some(edge) = edge {
                    let wanted = if edge == TextAnchor::Start { "start" } else { "end" };
                    if align != Some(wanted) {
                        let side = if edge == TextAnchor::Start { "left" } else { "right" };
                        let stated = match align {
                            None => "with no align".to_owned(),
                            Some(a) => format!("aligned \"{a}\""),
                        };
                        return fail(format!(
                            "{location} anchors at its {side} edge {stated}; the library aligns such a line \"{wanted}\"."
                        ));
                    }
                }
                declaration.anchor_to = anchor_to;
                let (ox, oy) = pair_of(p.get("offset"));
                match edge {
                    Some(edge) => {
                        declaration.align = Some(edge);
                        let inset = ox.unwrap_or(0.0);
                        declaration.inset_x = Some(if edge == TextAnchor::Start { inset } else { -inset });
                        declaration.offset = Some(Point::new(0.0, oy.unwrap_or(0.0)));
                    }
                    None if p.contains_key("offset") => {
                        declaration.offset = Some(Point::new(ox.unwrap_or(0.0), oy.unwrap_or(0.0)));
                    }
                    None => {}
                }
            }
            Some(Json::String(p)) if p == "top" || p == "bottom" => {
                // Inside, on the first or the last line of the node (DISL 0.4 section 6.9).
                let top = p == "top";
                declaration.anchor_to = if top { AnchorTo::Top } else { AnchorTo::Bottom };
                declaration.offset = Some(Point::new(0.0, if top { LABEL_LINE } else { -LABEL_LINE / 2.0 }));
            }
            Some(Json::String(p)) if p == "center" => {}
            None => {}
            Some(other) => {
                let shown = other.as_str().map(str::to_owned).unwrap_or_else(|| other.to_string());
                return fail(format!(
                    "{location} is positioned \"{shown}\", which the library has no placement for."
                ));
            }
        }
        declaration.editable = matches!(label.get("editable"), Some(Json::Bool(true)))
            || matches!(
                label.get("editable").and_then(Json::as_str),
                Some("inline" | "multiline")
            );
        if str_of(label, "wrap") == Some("word") {
            declaration.wrap = true;
        } else if str_of(label, "overflow") == Some("ellipsis") {
            declaration.truncate = true;
        }
        declaration.tooltip = label.get("tooltip").and_then(Binding::of);
        declaration.when = visibility(label.get("visible"));
        declaration.style = label.get("style").and_then(|s| style_of(&self.notation, s));
        Ok(declaration)
    }

    /// A node's icon (DISL section 6.14), drawn by the library's own glyph for its name, above a body-less node's
    /// point, mirrored while its `flip` gives `horizontal`.
    fn icon(&self, name: &str, node: &Json, bodyless: bool) -> Compiled<Vec<DecorationDeclaration>> {
        let Some(icon) = node.get("icon") else {
            return Ok(Vec::new());
        };
        let icon_name = str_of(icon, "icon").unwrap_or("");
        let location = format!("node \"{name}\" icon \"{icon_name}\"");
        let Some((_, glyph)) = ICONS.iter().find(|(n, _)| *n == icon_name) else {
            return fail(format!(
                "{location} is no icon the library draws (it draws {}).",
                ICONS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            ));
        };
        let position = str_of(icon, "position").unwrap_or("top");
        if !bodyless || position != "top" {
            return fail(format!(
                "{location} sits \"{position}\" on a node {}; the library draws an icon above a body-less node's point.",
                if bodyless { "" } else { "with a body " }
            ));
        }
        let radius = num_of(icon, "size").unwrap_or(ICON_SIZE) / 2.0;
        let mirrored = match icon.get("flip") {
            None => None,
            Some(Json::String(f)) if f == "none" => None,
            Some(Json::String(f)) if f == "horizontal" => Some("true".to_owned()),
            Some(flip) => match flip.get("cel").and_then(Json::as_str) {
                Some(cel) => Some(format!("({cel}) == 'horizontal'")),
                None => {
                    return fail(format!(
                        "{location} flips by {flip}; the library mirrors an icon left to right while one condition holds."
                    ));
                }
            },
        };
        Ok(vec![DecorationDeclaration {
            id: icon_name.to_owned(),
            glyph: *glyph,
            origin: DecorationOrigin::Centre {
                dx: 0.0,
                dy: -(radius + ICON_GAP),
            },
            radius,
            marker_end: MarkerKind::Arrow,
            mirrored,
            when: None,
            tooltip: None,
            label: None,
        }])
    }

    /// Where a node with no position of its own is drawn (DISL section 6.9 placement): the mean of the centres of the
    /// members one attribute lists, `self.<members>.map(m, m.view.bounds.center()[0]).avg()` and its `[1]`.
    fn centre_of(&self, name: &str, node: &Json) -> Compiled<Option<String>> {
        let placement = node.get("placement");
        let (x, y) = (placement.and_then(|p| p.get("x")), placement.and_then(|p| p.get("y")));
        fn cel(value: Option<&Json>) -> Option<&str> {
            value.and_then(|v| v.get("cel")).and_then(Json::as_str)
        }
        if cel(x).is_none() && cel(y).is_none() {
            return Ok(None);
        }
        let pattern = Regex::new(
            r"^self\.([A-Za-z_][A-Za-z0-9_]*)\.map\(([A-Za-z_][A-Za-z0-9_]*),([A-Za-z_][A-Za-z0-9_]*)\.view\.bounds\.center\(\)\[(\d)\]\)\.avg\(\)$",
        )
        .expect("the members pattern compiles");
        let members = |value: Option<&Json>, index: &str| -> Option<String> {
            let expression: String = cel(value)?.chars().filter(|c| !c.is_whitespace()).collect();
            let found = pattern.captures(&expression)?;
            (found[2] == found[3] && &found[4] == index).then(|| found[1].to_owned())
        };
        match (members(x, "0"), members(y, "1")) {
            (Some(across), Some(down)) if across == down => Ok(Some(across)),
            _ => fail(format!(
                "node \"{name}\" is placed at {}; the library places a node at the centre of the members one attribute lists.",
                serde_json::json!({ "x": x, "y": y })
            )),
        }
    }

    /// Where a line may attach. DISL never draws an anchor as a handle, so compass and along anchors are hidden.
    fn anchors(&self, name: &str, node: &Json) -> Compiled<AnchorSet> {
        if node.get("connectable") == Some(&Json::Bool(false)) {
            return Ok(AnchorSet {
                enabled: false,
                visible: false,
                ..AnchorSet::edge()
            });
        }
        let Some(anchors) = node.get("anchors") else {
            return self.implied_anchors(name);
        };
        match str_of(anchors, "mode") {
            Some("fixed") => {
                let points: Vec<(String, f64, f64)> = anchors
                    .get("points")
                    .and_then(Json::as_array)
                    .map(|points| {
                        points
                            .iter()
                            .map(|p| {
                                (
                                    str_of(p, "id").unwrap_or("").to_owned(),
                                    num_of(p, "x").unwrap_or(0.0),
                                    num_of(p, "y").unwrap_or(0.0),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let at_compass = points
                    .iter()
                    .all(|(id, x, y)| Compass::of(id).is_some_and(|c| c.fraction() == (*x, *y)));
                if !at_compass && let Some((id, x, y)) = points.iter().find(|(id, _, _)| Compass::of(id).is_some()) {
                    return fail(format!(
                        "node \"{name}\" anchor \"{id}\" at ({x}, {y}) is no compass position the library has."
                    ));
                }
                if !at_compass {
                    return self.named_side_anchors(name, &points);
                }
                Ok(AnchorSet {
                    kind: AnchorKind::Compass(points.iter().filter_map(|(id, _, _)| Compass::of(id)).collect()),
                    edge_sides: EdgeSides::All,
                    enabled: true,
                    visible: false,
                    drawn_by_edge: str_of(anchors, "drawnFrom") == Some("outline"),
                })
            }
            Some("outline") => Ok(AnchorSet::edge()),
            Some("sides") => {
                let mut sides: Vec<String> = anchors
                    .get("sides")
                    .and_then(Json::as_array)
                    .map(|s| s.iter().filter_map(Json::as_str).map(str::to_owned).collect())
                    .unwrap_or_else(|| ["top", "right", "bottom", "left"].map(str::to_owned).to_vec());
                sides.sort();
                let edge_sides = match sides.join(",").as_str() {
                    "bottom,top" => EdgeSides::Vertical,
                    "left,right" => EdgeSides::Horizontal,
                    "bottom,left,right,top" => EdgeSides::All,
                    other => {
                        return fail(format!(
                            "node \"{name}\" attaches on {other}, which the library's edge anchors cannot narrow to."
                        ));
                    }
                };
                Ok(AnchorSet {
                    edge_sides,
                    ..AnchorSet::edge()
                })
            }
            other => fail(format!(
                "node \"{name}\" anchors in mode \"{}\", which this compiler does not map.",
                other.unwrap_or("")
            )),
        }
    }

    /// A node with no anchors of its own: where an edge attaching to a part of it says, anywhere along the sides its
    /// `side` attribute may name; else anywhere on the outline.
    fn implied_anchors(&self, name: &str) -> Compiled<AnchorSet> {
        let mut sides: Vec<String> = Vec::new();
        for (edge_name, edge) in &self.edges {
            let relation = self.relation(edge_name)?;
            for end in ["source", "target"] {
                let anchoring = edge.get("anchoring").and_then(|a| a.get(end));
                if anchoring.and_then(|a| str_of(a, "mode")) != Some("part") {
                    continue;
                }
                let types = if end == "source" { &relation.0 } else { &relation.1 };
                if !types.iter().any(|t| self.is_a(name, t)) {
                    continue;
                }
                let Some(attribute) = anchoring
                    .and_then(|a| a.get("side"))
                    .and_then(|s| str_of(s, "attribute"))
                else {
                    return fail(format!(
                        "edge \"{edge_name}\" {end} side must be bound to an attribute to say which sides of \"{name}\" it attaches to."
                    ));
                };
                let metamodel = self.specification.metamodel();
                let values = metamodel
                    .relations
                    .get(edge_name)
                    .and_then(|r| r.attributes.get(attribute))
                    .and_then(|a| metamodel.enums.get(&a.type_name));
                let Some(values) = values else {
                    return fail(format!(
                        "edge \"{edge_name}\" {end} side attribute \"{attribute}\" is no enum of sides."
                    ));
                };
                for value in &values.values {
                    if !sides.contains(&value.key) {
                        sides.push(value.key.clone());
                    }
                }
            }
        }
        if sides.is_empty() {
            return Ok(AnchorSet {
                visible: false,
                ..AnchorSet::edge()
            });
        }
        let edges = sides.iter().filter_map(|side| edge_of(side)).collect();
        Ok(AnchorSet {
            kind: AnchorKind::Along { edges, segments: true },
            edge_sides: EdgeSides::All,
            enabled: true,
            visible: false,
            drawn_by_edge: false,
        })
    }

    /// Fixed anchors named for what they are rather than where: each a named fraction of the side it lies on, a line's
    /// other end attaching only on the sides these lie on.
    fn named_side_anchors(&self, name: &str, points: &[(String, f64, f64)]) -> Compiled<AnchorSet> {
        let mut fractions = Vec::new();
        for (id, x, y) in points {
            let (side, at) = if *x == 0.0 {
                (Edge::Left, 1.0 - y)
            } else if *x == 1.0 {
                (Edge::Right, *y)
            } else if *y == 0.0 {
                (Edge::Top, *x)
            } else if *y == 1.0 {
                (Edge::Bottom, 1.0 - x)
            } else {
                return fail(format!(
                    "node \"{name}\" anchor \"{id}\" at ({x}, {y}) lies on no side of its bounds."
                ));
            };
            fractions.push(SideFraction {
                side,
                at,
                name: Some(id.clone()),
            });
        }
        let horizontal = fractions.iter().all(|f| matches!(f.side, Edge::Left | Edge::Right));
        let vertical = fractions.iter().all(|f| matches!(f.side, Edge::Top | Edge::Bottom));
        Ok(AnchorSet {
            kind: AnchorKind::Sides(fractions),
            edge_sides: if horizontal {
                EdgeSides::Horizontal
            } else if vertical {
                EdgeSides::Vertical
            } else {
                EdgeSides::All
            },
            enabled: true,
            visible: true,
            drawn_by_edge: false,
        })
    }

    /// `User` when the node may be resized (its size says so and its placement does not forbid it), else `Model`;
    /// both ways when its size is resizable both ways.
    fn sizing(&self, node: &Json) -> Compiled<(Sizing, Resize)> {
        let resizable = node
            .get("size")
            .and_then(|s| s.get("resizable"))
            .cloned()
            .unwrap_or(Json::Bool(false));
        if resizable.as_str() == Some("vertical") {
            return fail("a node resizable only vertically has no library equivalent.");
        }
        let placement = node.get("placement").and_then(|p| p.get("resizable"));
        let allows = match placement {
            None | Some(Json::Bool(true)) => true,
            Some(Json::Object(o)) => o.get("x") == Some(&Json::Bool(true)) || o.get("y") == Some(&Json::Bool(true)),
            _ => false,
        };
        let sizing = if resizable != Json::Bool(false) && allows {
            Sizing::User
        } else {
            Sizing::Model
        };
        Ok((
            sizing,
            if resizable == Json::Bool(true) {
                Resize::Both
            } else {
                Resize::Width
            },
        ))
    }

    fn movable(&self, node: &Json) -> bool {
        match node.get("placement").and_then(|p| p.get("movable")) {
            None | Some(Json::Bool(true)) => true,
            Some(Json::Object(o)) => o.get("x") == Some(&Json::Bool(true)) || o.get("y") == Some(&Json::Bool(true)),
            _ => false,
        }
    }

    // ---- lists and badges (DISL 0.4) -------------------------------------------------------------------------------

    /// A CEL term a row binds: `{cel}` as written, `{attribute: a}` as `item.a`.
    fn row_source(&self, value: &Json, location: &str) -> Compiled<String> {
        if let Some(cel) = value.get("cel").and_then(Json::as_str) {
            return Ok(cel.trim().to_owned());
        }
        if let Some(attribute) = value.get("attribute").and_then(Json::as_str) {
            return Ok(format!("item.{attribute}"));
        }
        fail(format!("{location} binds {value}; a row binds CEL over its item."))
    }

    fn compartments(&self, name: &str, node: &Json) -> Compiled<Vec<CompartmentDeclaration>> {
        let Some(declared) = node
            .get("compartments")
            .and_then(Json::as_array)
            .filter(|c| !c.is_empty())
        else {
            return Ok(Vec::new());
        };
        // Beneath the lowest line of text the node's labels write.
        let lowest = node
            .get("labels")
            .and_then(Json::as_array)
            .map(|labels| {
                labels
                    .iter()
                    .map(|label| match label.get("position") {
                        Some(Json::Object(p)) => pair_of(p.get("offset")).1.unwrap_or(0.0),
                        Some(Json::String(p)) if p == "top" => LABEL_LINE,
                        _ => 0.0,
                    })
                    .fold(LABEL_LINE, f64::max)
            })
            .unwrap_or(LABEL_LINE);
        let top = lowest + LIST_GAP;
        declared
            .iter()
            .map(|compartment| {
                let id = str_of(compartment, "id").unwrap_or("").to_owned();
                let location = format!("node \"{name}\" list \"{id}\"");
                let items = compartment.get("items").unwrap_or(&Json::Null);
                let items = match items.get("cel").and_then(Json::as_str) {
                    Some(cel) => CompartmentItems::Cel(cel.to_owned()),
                    None => CompartmentItems::Children {
                        types: items
                            .get("children")
                            .and_then(Json::as_array)
                            .map(|c| c.iter().filter_map(Json::as_str).map(str::to_owned).collect())
                            .unwrap_or_default(),
                        slot: str_of(items, "slot").unwrap_or("").to_owned(),
                    },
                };
                let Some(item_text) = compartment.get("itemText") else {
                    return fail(format!("{location} states no itemText."));
                };
                let order = compartment.get("itemOrder");
                let headless = compartment.get("title").is_none()
                    && compartment.get("groupBy").is_none()
                    && compartment.get("collapsible") != Some(&Json::Bool(true));
                Ok(CompartmentDeclaration {
                    text: self.row_source(item_text, &format!("{location} itemText"))?,
                    link: compartment
                        .get("itemLink")
                        .map(|link| self.row_source(link, &format!("{location} itemLink")))
                        .transpose()?,
                    order_by: order.and_then(|o| {
                        Some((
                            str_of(o, "by")?.to_owned(),
                            str_of(o, "direction").unwrap_or("ascending") == "ascending",
                        ))
                    }),
                    group_by: compartment
                        .get("groupBy")
                        .map(|g| self.groups(&items, g, &location))
                        .transpose()?,
                    title: str_of(compartment, "title").map(str::to_owned),
                    headless,
                    collapsed: compartment.get("collapsed") == Some(&Json::Bool(true)),
                    id,
                    items,
                    top,
                    heading_height: LIST_HEADING,
                    row_height: LIST_ROW,
                    bottom: LIST_BOTTOM,
                    inset_x: LIST_INSET,
                    row_indent: if headless { 0.0 } else { LIST_INDENT },
                })
            })
            .collect()
    }

    /// A list's groups: the values of the enum the items' attribute has, in the enum's order.
    fn groups(&self, items: &CompartmentItems, group_by: &Json, location: &str) -> Compiled<Grouping> {
        let attribute = str_of(group_by, "attribute").unwrap_or("");
        let metamodel = self.specification.metamodel();
        let values = match items {
            CompartmentItems::Children { types, .. } => types
                .first()
                .and_then(|t| metamodel.type_of(t))
                .and_then(|t| t.attributes.get(attribute))
                .and_then(|a| metamodel.enums.get(&a.type_name)),
            CompartmentItems::Cel(_) => None,
        };
        let Some(values) = values else {
            return fail(format!(
                "{location} groups by \"{attribute}\", which is no enum attribute of its items."
            ));
        };
        Ok(Grouping {
            attribute: attribute.to_owned(),
            groups: values
                .values
                .iter()
                .map(|v| (v.key.clone(), v.label.clone().unwrap_or_else(|| v.key.clone())))
                .collect(),
            collapsed: group_by
                .get("collapsed")
                .and_then(Json::as_object)
                .map(|c| {
                    c.iter()
                        .filter(|(_, v)| v.as_bool() == Some(true))
                        .map(|(k, _)| k.clone())
                        .collect()
                })
                .unwrap_or_default(),
            other_title: "Other".to_owned(),
        })
    }

    /// A node's badges (DISL 0.4 section 6.9): one whose press opens a link (its operation's one action is `open` of
    /// an attribute) is a link symbol for that attribute; one that runs nothing is a mark, shown while its condition
    /// holds. Any other badge is refused, naming it.
    fn badges(&self, name: &str, node: &Json) -> Compiled<(Vec<LinkDeclaration>, Vec<DecorationDeclaration>)> {
        let mut links = Vec::new();
        let mut decorations = Vec::new();
        let opened = Regex::new(r"^self\.([A-Za-z_][A-Za-z0-9_]*)$").expect("the attribute pattern compiles");
        for badge in node.get("badges").and_then(Json::as_array).into_iter().flatten() {
            let id = str_of(badge, "id").unwrap_or("");
            let location = format!("node \"{name}\" badge \"{id}\"");
            let position = str_of(badge, "position");
            let right = position == Some("top-right");
            if !right && position != Some("top-left") {
                return fail(format!(
                    "{location} sits \"{}\"; the library places a badge at the top left or the top right.",
                    position.unwrap_or("undefined")
                ));
            }
            if let Some(operation_name) = str_of(badge, "onClick") {
                let operation = self
                    .root
                    .get("behavior")
                    .and_then(|b| b.get("operations"))
                    .and_then(|o| o.get(operation_name));
                let actions = operation.and_then(|o| o.get("actions")).and_then(Json::as_array);
                let attribute = actions
                    .filter(|a| a.len() == 1)
                    .and_then(|a| str_of(&a[0], "open"))
                    .and_then(|open| opened.captures(open.trim()).map(|c| c[1].to_owned()));
                match attribute {
                    Some(attribute) if right => links.push(LinkDeclaration {
                        id: id.to_owned(),
                        attribute,
                        right: BADGE_INSET + LINK_SIZE,
                        top: BADGE_TOP,
                    }),
                    _ => {
                        return fail(format!(
                            "{location} runs \"{operation_name}\"; the library draws a pressed badge only as a link symbol at the top right, opening one attribute."
                        ));
                    }
                }
                continue;
            }
            let tooltip = badge.get("tooltip");
            let said = match tooltip.and_then(Json::as_str) {
                Some(text) => text.split('.').next().unwrap_or(text).to_owned(),
                None => id.to_owned(),
            };
            decorations.push(DecorationDeclaration {
                id: id.to_owned(),
                glyph: Glyph::Circle,
                origin: DecorationOrigin::Corner {
                    right,
                    inset: BADGE_INSET,
                },
                radius: BADGE_RADIUS,
                marker_end: MarkerKind::None,
                mirrored: None,
                when: visibility(badge.get("visible")),
                tooltip: tooltip.and_then(Binding::of),
                label: Some(said),
            });
        }
        Ok((links, decorations))
    }

    // ---- relation types --------------------------------------------------------------------------------------------

    /// A relation's source and target types.
    fn relation(&self, name: &str) -> Compiled<(Vec<String>, Vec<String>)> {
        let Some(relation) = self.specification.metamodel().relations.get(name) else {
            return fail(format!("edge \"{name}\" has no relation in the metamodel."));
        };
        Ok((
            relation.source.as_ref().map(|e| e.types.clone()).unwrap_or_default(),
            relation.target.as_ref().map(|e| e.types.clone()).unwrap_or_default(),
        ))
    }

    fn relation_type(&self, name: &str) -> Compiled<RelationType> {
        let edge = &self.edges[name];
        let (sources, targets) = self.relation(name)?;
        let declared = self.specification.metamodel().relations.get(name).cloned();
        let relation_json = declared.as_ref().map(|r| r.json.clone()).unwrap_or(Json::Null);
        let line = edge.get("line").cloned().unwrap_or(Json::Object(Map::new()));
        // The flags as the metamodel reads them: stated, else inherited, else the default (DISL 0.6, section 4.7).
        let (directed, allow_self, allow_parallel) = declared.as_ref().map_or((true, false, true), |r| {
            (r.directed, r.allow_self_loops, r.allow_parallel)
        });
        let target_marker = str_of(edge, "targetMarker").unwrap_or(if directed { "arrowFilled" } else { "none" });
        let source_marker = str_of(edge, "sourceMarker").unwrap_or("none");
        let route = self.route(name, edge, &line)?;
        let ends = [
            edge.get("anchoring").and_then(|a| a.get("source")),
            edge.get("anchoring").and_then(|a| a.get("target")),
        ];

        // Only a type that may hold children can be the parent end of a relation derived from containment.
        let derived = relation_json.get("derived");
        let from_parent = derived.and_then(|d| str_of(d, "source")) == Some("item.parent");
        let source_types: Vec<String> = sources
            .iter()
            .flat_map(|t| self.concrete_types(t))
            .filter(|t| !from_parent || self.holds_children(t))
            .collect();
        let target_types: Vec<String> = targets.iter().flat_map(|t| self.concrete_types(t)).collect();
        let endpoint = |types: Vec<String>, end: &str, anchoring: Option<&Json>| EndpointConstraint {
            anchors: match self.starting_anchors(edge, &types, end) {
                Some(named) => EndpointAnchors::Named(named),
                None if anchoring.and_then(|a| str_of(a, "mode")) == Some("outline") => EndpointAnchors::Edge,
                None => EndpointAnchors::Any,
            },
            element_types: types,
        };
        let per_pair = (derived.is_none() && !allow_parallel).then_some(if directed {
            PerPair::Ordered
        } else {
            PerPair::Unordered
        });
        let (max_from_source, max_into_target) = self.held_once(&relation_json);

        let mut style = edge
            .get("style")
            .and_then(|s| style_of(&self.notation, s))
            .unwrap_or(Json::Object(Map::new()));
        if let Some(stroke) = line.get("stroke") {
            let merged = match style.get("stroke") {
                Some(existing) => merged_per_key(existing, stroke),
                None => stroke.clone(),
            };
            if let Some(object) = style.as_object_mut() {
                object.insert("stroke".to_owned(), merged);
            }
        }
        if let Some(dash) = line.get("stroke").and_then(|s| s.get("dash")) {
            self.check_dash(name, dash)?;
        }

        Ok(RelationType {
            name: name.to_owned(),
            route,
            start_marker: library_marker_of(source_marker).map_err(CompileError)?,
            end_marker: library_marker_of(target_marker).map_err(CompileError)?,
            style,
            corner_radius: num_of(&line, "cornerRadius"),
            label: self.edge_label(name, edge)?,
            stub: self.stub(name, edge)?,
            selectable: edge.get("selectable") != Some(&Json::Bool(false)),
            movable_ends: ends
                .iter()
                .any(|e| e.and_then(|e| e.get("movable")) == Some(&Json::Bool(true))),
            hide_when_attachment_hidden: ends.iter().any(|e| e.and_then(|e| str_of(e, "mode")) == Some("part")),
            endpoints: Endpoints {
                source: endpoint(source_types, "source", ends[0]),
                target: endpoint(target_types, "target", ends[1]),
                allow_self,
                per_pair,
                max_from_source,
                max_into_target,
            },
            empty_release: self.creates_on_empty_release(name, edge)?,
            source_anchoring: end_anchoring(ends[0]),
            target_anchoring: end_anchoring(ends[1]),
        })
    }

    /// A line's dash (DISL section 6.4 `dash`): a list of numbers, a preset, or CEL; anything else is refused.
    fn check_dash(&self, name: &str, dash: &Json) -> Compiled<()> {
        let fine = match dash {
            Json::Array(items) => items.iter().all(Json::is_number),
            Json::String(_) => true,
            Json::Object(o) => o.contains_key("cel"),
            _ => false,
        };
        if fine {
            Ok(())
        } else {
            fail(format!(
                "edge \"{name}\": its line's dash is {dash}; the library draws a dash from a list of numbers."
            ))
        }
    }

    /// How many of a drawable derived relation one element may have (DISL 0.4 section 4.11.4).
    fn held_once(&self, relation: &Json) -> (Option<usize>, Option<usize>) {
        let Some(derived) = relation.get("derived").filter(|d| d.is_object()) else {
            return (None, None);
        };
        if self.specification.disl_version() != "0.4"
            || derived
                .get("edits")
                .and_then(|e| e.get("connect"))
                .and_then(Json::as_str)
                .is_none()
        {
            return (None, None);
        }
        let reference = Regex::new(r"^item\.[A-Za-z_][A-Za-z0-9_]*$").expect("the reference pattern compiles");
        let (source, target) = (str_of(derived, "source"), str_of(derived, "target"));
        if target == Some("item") && source.is_some_and(|s| reference.is_match(s)) {
            return (None, Some(1));
        }
        if source == Some("item") && target.is_some_and(|t| reference.is_match(t)) {
            return (Some(1), None);
        }
        (None, None)
    }

    fn route(&self, name: &str, edge: &Json, line: &Json) -> Compiled<Route> {
        let routing = line
            .get("routing")
            .cloned()
            .unwrap_or(Json::String("straight".to_owned()));
        let along = self.along_the_line(name, edge, line, &routing)?;
        if let Some(along) = along {
            return Ok(Route::BowedArc(along));
        }
        self.routing(&routing, line)
    }

    fn routing(&self, routing: &Json, line: &Json) -> Compiled<Route> {
        if let Some(plugin) = routing.get("plugin").and_then(Json::as_str) {
            let fallback = self
                .root
                .get("plugins")
                .and_then(|p| p.get(plugin))
                .and_then(|p| p.get("fallback"))
                .and_then(|f| f.get("routing"))
                .cloned()
                .unwrap_or(Json::String("bezier".to_owned()));
            return Ok(Route::Plugin {
                name: plugin.to_owned(),
                fallback: Box::new(self.routing(&fallback, line)?),
            });
        }
        let routing = routing.as_str().unwrap_or("straight");
        if routing == "bezier"
            && str_of(line, "startDirection") == Some("right")
            && str_of(line, "endDirection") == Some("left")
        {
            return Ok(Route::ForwardBezier);
        }
        Ok(Route::BuiltIn(library_route_of(routing).map_err(CompileError)?))
    }

    /// A curved line as a bowed arc, with what the notation rides along it: the variants that bow it otherwise, its
    /// loop to its own node, the bars of its `midMarkers` and the marks of its labels placed at a fraction. A line of
    /// any other routing that states bars or marks is refused.
    fn along_the_line(
        &self,
        name: &str,
        edge: &Json,
        line: &Json,
        routing: &Json,
    ) -> Compiled<Option<BowedArcDeclaration>> {
        let location = format!("edge \"{name}\"");
        let mid_markers = edge
            .get("midMarkers")
            .and_then(Json::as_array)
            .cloned()
            .unwrap_or_default();
        let marks: Vec<Json> = edge
            .get("labels")
            .and_then(Json::as_array)
            .map(|labels| {
                labels
                    .iter()
                    .filter(|l| l.get("at").is_some_and(Json::is_number))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if routing.as_str() != Some("curved") {
            if !mid_markers.is_empty() || !marks.is_empty() {
                let what = if !mid_markers.is_empty() {
                    "markers"
                } else {
                    "labels at a fraction"
                };
                let shown = routing
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| routing.to_string());
                return fail(format!(
                    "{location} draws {what} along a \"{shown}\" line; the library draws them along a curved one only."
                ));
            }
            return Ok(None);
        }
        let bow = num_of(line, "curvature").unwrap_or(ARC_BOW);
        let variants = edge
            .get("variants")
            .and_then(Json::as_array)
            .map(|variants| {
                variants
                    .iter()
                    .enumerate()
                    .map(|(index, variant)| self.bow_variant(variant, &format!("{location} variant {}", index + 1)))
                    .collect::<Compiled<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();
        let self_loop = edge.get("selfLoop");
        if let Some(self_loop) = self_loop
            && (str_of(self_loop, "shape").unwrap_or("arc") != "arc"
                || str_of(self_loop, "side").unwrap_or("top") != "top")
        {
            return fail(format!(
                "{location} loops to its own node as {self_loop}; the library draws that loop as an arc above the node."
            ));
        }
        let bars = self.bars(name, &mid_markers)?;
        let marks = marks
            .iter()
            .map(|label| self.mark(name, label))
            .collect::<Compiled<Vec<_>>>()?;
        Ok(Some(BowedArcDeclaration {
            bow,
            variants,
            self_loop_radius: self_loop.and_then(|s| num_of(s, "size")),
            bars,
            marks,
        }))
    }

    /// A variant that bows a curved line otherwise while its condition holds; any other change is refused.
    fn bow_variant(&self, variant: &Json, location: &str) -> Compiled<(String, f64)> {
        let empty = Map::new();
        let shape = variant.as_object().unwrap_or(&empty);
        let line = shape.get("line").and_then(Json::as_object).unwrap_or(&empty);
        let others = shape
            .keys()
            .filter(|k| !["when", "doc", "line"].contains(&k.as_str()))
            .count()
            + line
                .keys()
                .filter(|k| !["routing", "curvature", "bendpoints"].contains(&k.as_str()))
                .count();
        let when = shape.get("when").and_then(Json::as_str);
        let curvature = line.get("curvature").and_then(Json::as_f64);
        let curved = line.get("routing").and_then(Json::as_str).unwrap_or("curved") == "curved";
        match (when, curvature) {
            (Some(when), Some(curvature)) if curved && others == 0 => Ok((when.trim().to_owned(), curvature)),
            _ => fail(format!(
                "{location} changes {variant}; the library draws a variant of a curved line as the same curve bowed otherwise."
            )),
        }
    }

    /// The bars of a line's `midMarkers`: each a `bar` of a width at a fraction, oriented by the line; neighbours of
    /// one width shown under one condition are one group.
    fn bars(&self, name: &str, markers: &[Json]) -> Compiled<Vec<ArcBars>> {
        let mut groups: Vec<ArcBars> = Vec::new();
        let mut key: Option<(String, Option<String>)> = None;
        for marker in markers {
            let declared = marker.get("marker").unwrap_or(&Json::Null);
            let kind = declared.as_str().or_else(|| str_of(declared, "type"));
            let width = declared.get("params").and_then(|p| num_of(p, "width"));
            let orient = str_of(marker, "orient").unwrap_or("auto");
            let (Some("bar"), Some(width), "auto") = (kind, width, orient) else {
                return fail(format!(
                    "edge \"{name}\" draws the marker {declared} oriented \"{orient}\"; the library draws a bar of a width across the line."
                ));
            };
            let when = visibility(marker.get("visible"));
            let next = (width.to_string(), when.clone());
            if key.as_ref() != Some(&next) {
                groups.push(ArcBars {
                    at: Vec::new(),
                    length: width,
                    when,
                });
                key = Some(next);
            }
            if let Some(group) = groups.last_mut() {
                group.at.push(num_of(marker, "at").unwrap_or(0.5));
            }
        }
        Ok(groups)
    }

    /// A label at a fraction of a curved line, beside it on the left or the right of travel.
    fn mark(&self, name: &str, label: &Json) -> Compiled<ArcMark> {
        let id = str_of(label, "id").unwrap_or("");
        let location = format!("edge \"{name}\" label \"{id}\"");
        let side = str_of(label, "side").unwrap_or("left");
        let editable = label.get("editable").is_some_and(|e| e != &Json::Bool(false));
        if (side != "left" && side != "right")
            || str_of(label, "orientation").unwrap_or("horizontal") != "horizontal"
            || editable
        {
            return fail(format!(
                "{location} sits {}; the library writes a mark beside a curved line, upright, on its left or right, and never edits one.",
                serde_json::json!({ "side": label.get("side"), "orientation": label.get("orientation"), "editable": label.get("editable") })
            ));
        }
        Ok(ArcMark {
            text: self.binding(label.get("text").unwrap_or(&Json::Null), &location)?,
            at: num_of(label, "at").unwrap_or(0.5),
            left: side == "left",
            distance: num_of(label, "distance").unwrap_or(0.0),
        })
    }

    /// The edge's label along its route: at its start, middle or end, above, on or below the line.
    fn edge_label(&self, name: &str, edge: &Json) -> Compiled<Option<RouteLabel>> {
        let labels: Vec<&Json> = edge
            .get("labels")
            .and_then(Json::as_array)
            .map(|labels| {
                labels
                    .iter()
                    .filter(|l| !l.get("at").is_some_and(Json::is_number))
                    .collect()
            })
            .unwrap_or_default();
        let location = format!("edge \"{name}\" label");
        if labels.len() > 1 {
            return fail(format!(
                "{location}s: the library draws one label along a line; the specification declares {}.",
                labels.len()
            ));
        }
        let Some(label) = labels.first() else {
            return Ok(None);
        };
        let id = str_of(label, "id").unwrap_or("");
        let text = self.binding(
            label.get("text").unwrap_or(&Json::Null),
            &format!("{location} \"{id}\""),
        )?;
        let placement = match label.get("at").and_then(Json::as_str).unwrap_or("middle") {
            "start" => RouteLabelPlacement::Source,
            "middle" => RouteLabelPlacement::Midpoint,
            "end" => RouteLabelPlacement::Target,
            other => {
                return fail(format!(
                    "{location} \"{id}\" sits at \"{other}\"; the library places a line's label at its start, middle or end."
                ));
            }
        };
        let distance = num_of(label, "distance").unwrap_or(0.0);
        let side = str_of(label, "side").unwrap_or("on");
        let offset = match side {
            "above" => -distance,
            "below" => distance,
            "on" if distance == 0.0 => 0.0,
            _ => {
                return fail(format!(
                    "{location} \"{id}\" sits {distance} {side} its line, which the library cannot place."
                ));
            }
        };
        let editable = matches!(label.get("editable"), Some(Json::Bool(true)))
            || matches!(
                label.get("editable").and_then(Json::as_str),
                Some("inline" | "multiline")
            );
        Ok(Some(RouteLabel {
            text,
            placement,
            offset: (offset != 0.0).then_some(offset),
            editable,
        }))
    }

    /// How a relation without a target is drawn (DISL section 6.10 *Stubs*); `auto`, which leaves the side to the
    /// runtime, is refused, as the standalone refuses it.
    fn stub(&self, name: &str, edge: &Json) -> Compiled<Option<StubDeclaration>> {
        let Some(stub) = edge.get("stub") else {
            return Ok(None);
        };
        let location = format!("edge \"{name}\"");
        let side_name = str_of(stub, "side").unwrap_or("undefined");
        let Some(side) = edge_of(side_name) else {
            return fail(format!(
                "{location}: its stub leaves \"{side_name}\"; the library draws a stub out of the right, left, top or bottom."
            ));
        };
        let length = num_of(stub, "length").filter(|l| *l > 0.0);
        let Some(length) = length else {
            return fail(format!(
                "{location}: its stub is {} long; the library draws a stub of a positive length.",
                stub.get("length").unwrap_or(&Json::Null)
            ));
        };
        let label = match stub.get("label") {
            None => None,
            Some(label) => {
                let side = str_of(label, "side").unwrap_or("above");
                if side != "above" && side != "below" {
                    return fail(format!(
                        "{location}: its stub label \"{}\" sits \"{side}\"; the library writes a stub's label above or below it.",
                        str_of(label, "id").unwrap_or("")
                    ));
                }
                Some(StubLabel {
                    text: self.binding(
                        label.get("text").unwrap_or(&Json::Null),
                        &format!("{location} stub label"),
                    )?,
                    below: side == "below",
                    distance: num_of(label, "distance"),
                    inset: None,
                })
            }
        };
        Ok(Some(StubDeclaration {
            length,
            side,
            style: stub.get("style").and_then(|s| style_of(&self.notation, s)),
            label,
        }))
    }

    /// The anchors a drawn connection may start from at the source end: the named anchors the edge's `connect.from`
    /// maps, in the order the end's node lists them.
    fn starting_anchors(&self, edge: &Json, types: &[String], end: &str) -> Option<Vec<String>> {
        let from = edge
            .get("connect")
            .and_then(|c| c.get("from"))
            .and_then(Json::as_object)?;
        if end != "source" {
            return None;
        }
        let mut anchors: Vec<String> = Vec::new();
        for node in types.iter().filter_map(|t| self.nodes.get(t)) {
            for point in node
                .get("anchors")
                .and_then(|a| a.get("points"))
                .and_then(Json::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(id) = str_of(point, "id").filter(|id| from.contains_key(*id))
                    && !anchors.iter().any(|a| a == id)
                {
                    anchors.push(id.to_owned());
                }
            }
        }
        for id in from.keys() {
            if !anchors.contains(id) {
                anchors.push(id.clone());
            }
        }
        Some(anchors)
    }

    /// Whether a connect gesture released on empty canvas creates the missing end (DISL 0.3 section 7.2).
    fn creates_on_empty_release(&self, name: &str, edge: &Json) -> Compiled<bool> {
        let toolbox = self.root.get("toolbox");
        let tools = toolbox.and_then(|t| t.get("tools")).and_then(Json::as_object);
        let grouped: Vec<&Json> = toolbox
            .and_then(|t| t.get("groups"))
            .and_then(Json::as_array)
            .into_iter()
            .flatten()
            .flat_map(|g| g.get("tools").and_then(Json::as_array).into_iter().flatten())
            .collect();
        let tool = match edge.get("connect").and_then(|c| str_of(c, "tool")) {
            Some(named) => match tools
                .and_then(|t| t.get(named))
                .or_else(|| grouped.iter().copied().find(|t| str_of(t, "id") == Some(named)))
            {
                Some(tool) => Some(tool),
                None => {
                    return fail(format!(
                        "edge \"{name}\" connects with the tool \"{named}\", which the toolbox does not declare."
                    ));
                }
            },
            None => tools.and_then(|t| t.values().find(|tool| str_of(tool, "creates") == Some(name))),
        };
        Ok(tool.is_some_and(|t| t.get("createTarget").is_some() || t.get("createSource").is_some()))
    }
}

/// An edge by its DISL name.
fn edge_of(name: &str) -> Option<Edge> {
    match name {
        "top" => Some(Edge::Top),
        "bottom" => Some(Edge::Bottom),
        "left" => Some(Edge::Left),
        "right" => Some(Edge::Right),
        _ => None,
    }
}

/// How an end of a drawn line attaches, from the edge's `anchoring` of that end.
fn end_anchoring(anchoring: Option<&Json>) -> EndAnchoring {
    match anchoring.and_then(|a| str_of(a, "mode")) {
        Some("fixed") => EndAnchoring::Fixed(
            anchoring
                .and_then(|a| a.get("points"))
                .and_then(Json::as_array)
                .map(|p| p.iter().filter_map(|p| str_of(p, "id")).map(str::to_owned).collect())
                .unwrap_or_default(),
        ),
        Some("center") => EndAnchoring::Center,
        _ => EndAnchoring::Edge,
    }
}

/// The built-in route a definition's relation is drawn with, where it has one.
pub fn built_in_route(route: &Route) -> Option<BuiltInRoute> {
    match route {
        Route::BuiltIn(route) => Some(*route),
        _ => None,
    }
}

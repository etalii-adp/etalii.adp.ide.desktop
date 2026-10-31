//! What the ported tests share: specifications loaded from JSON, an evaluator over them, and scenes built with the
//! standalone host's text estimate.

#![allow(dead_code)]

use std::collections::HashMap;

use adp_disl::{DislDiagram, DislEnv, DislLoader, DislSpecification};
use adp_scene::definition::compartments::Folding;
use adp_scene::scene::{Owner, Primitive, Role, StandardRoutePlugins, TextRun};
use adp_scene::{
    DiagramDefinition, EstimatedMetrics, Evaluator, Rect, Scene, SceneOptions, Theme, build_scene, compile_notation,
};

/// Loads `json`, failing with the diagnostics when it does not load.
pub fn loaded(json: &str) -> DislSpecification {
    let result = DislLoader::load(json);
    match result.specification {
        Some(specification) => specification,
        None => panic!(
            "The specification does not load: {}",
            result
                .diagnostics
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        ),
    }
}

/// A vendored diagram specification by its file name.
pub fn vendored(name: &str) -> DislSpecification {
    let text = std::fs::read_to_string(format!(
        "{}/../../vendor/etalii.adp/definitions/diagrams/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("the vendored specification is there");
    loaded(&text)
}

/// The definition `json` compiles to, failing with the refusal.
pub fn compiled(json: &str) -> DiagramDefinition {
    compile_notation(&loaded(json)).unwrap_or_else(|e| panic!("{e}"))
}

/// The refusal `json` compiles to.
pub fn refusal(json: &str) -> String {
    match compile_notation(&loaded(json)) {
        Ok(_) => panic!("The notation compiles."),
        Err(e) => e.to_string(),
    }
}

/// An evaluator over the definition's specification, in its default theme mode.
pub fn evaluator(definition: &DiagramDefinition) -> Evaluator {
    Evaluator::new(
        &definition.specification,
        Theme::of(&definition.specification, None),
        DislEnv::default(),
    )
}

/// The scene of `diagram` with its elements at `positions`, nothing selected.
pub fn scene(definition: &DiagramDefinition, diagram: &DislDiagram, positions: &[(&str, Rect)]) -> Scene {
    scene_selecting(definition, diagram, positions, &[])
}

pub fn scene_selecting(
    definition: &DiagramDefinition,
    diagram: &DislDiagram,
    positions: &[(&str, Rect)],
    selection: &[&str],
) -> Scene {
    let positions: HashMap<String, Rect> = positions.iter().map(|(id, rect)| ((*id).to_owned(), *rect)).collect();
    let selection: Vec<String> = selection.iter().map(|s| (*s).to_owned()).collect();
    let folding = Folding::default();
    let options = SceneOptions {
        selection: &selection,
        folding: &folding,
        metrics: &EstimatedMetrics,
        plugins: &StandardRoutePlugins,
        shown: None,
    };
    build_scene(definition, &evaluator(definition), diagram, &positions, &options)
}

/// The texts an owner draws, in order.
pub fn texts<'a>(scene: &'a Scene, owner: &Owner) -> Vec<&'a TextRun> {
    scene
        .of(owner)
        .filter_map(|p| match p {
            Primitive::Text(t) => Some(t),
            _ => None,
        })
        .collect()
}

/// The first primitive of an owner in a role.
pub fn with_role<'a>(scene: &'a Scene, owner: &Owner, role: &Role) -> Option<&'a Primitive> {
    scene.of(owner).find(|p| p.role() == Some(role))
}

pub fn element(id: &str) -> Owner {
    Owner::Element(id.to_owned())
}

pub fn relation(id: &str) -> Owner {
    Owner::Relation(id.to_owned())
}

/// A rectangle by its centre and size, as the standalone's tests place their elements.
pub fn at(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect::new(x - width / 2.0, y - height / 2.0, width, height)
}

/// Curved lines between bodied nodes, with a body-less node among them.
pub fn curved_spec() -> serde_json::Value {
    serde_json::json!({
        "disl": "0.3",
        "language": { "id": "test", "version": "1.0.0" },
        "metamodel": {
            "types": {
                "Node": { "attributes": { "name": { "type": "string" } } },
                "Hub": { "attributes": { "caption": { "type": "string" }, "members": { "type": "Node", "many": true }, "turn": { "type": "string" } } }
            },
            "relations": { "Arrow": { "source": "Node", "target": "Node", "directed": true, "allowSelfLoops": true,
                "attributes": { "weight": { "type": "number" }, "flipped": { "type": "bool" }, "delayed": { "type": "bool" }, "sign": { "type": "string" } } } }
        },
        "notation": {
            "nodes": {
                "Node": { "shape": "pill", "size": { "default": [100, 40], "resizable": false }, "labels": [{ "id": "name", "text": { "cel": "self.name" }, "position": "center" }], "anchors": { "mode": "outline" } },
                "Hub": {
                    "shape": "none",
                    "size": { "fixed": [60, 40] },
                    "placement": { "x": { "cel": "self.members.map(m, m.view.bounds.center()[0]).avg()" }, "y": { "cel": "self.members.map(m, m.view.bounds.center()[1]).avg()" }, "anchor": "center", "movable": false },
                    "icon": { "icon": "mdi-sync", "position": "top", "size": 26, "flip": { "cel": "self.turn == 'back' ? 'horizontal' : 'none'" } },
                    "labels": [{ "id": "caption", "text": { "cel": "self.caption" }, "position": "bottom" }],
                    "connectable": false
                }
            },
            "edges": {
                "Arrow": {
                    "line": { "routing": "curved", "curvature": 0.2, "stroke": { "width": { "cel": "!has(self.weight) ? 1.5 : (self.weight >= 2.0 ? 2.5 : (self.weight <= 0.5 ? 1.0 : 1.5))" } } },
                    "variants": [{ "when": "self.flipped", "line": { "routing": "curved", "curvature": -0.2 } }],
                    "targetMarker": "arrowFilled",
                    "anchoring": { "source": { "mode": "outline" }, "target": { "mode": "outline" } },
                    "selfLoop": { "shape": "arc", "size": 30, "side": "top" },
                    "midMarkers": [
                        { "marker": { "type": "bar", "params": { "width": 16 } }, "at": 0.44, "orient": "auto", "visible": { "cel": "self.delayed" } },
                        { "marker": { "type": "bar", "params": { "width": 16 } }, "at": 0.56, "orient": "auto", "visible": { "cel": "self.delayed" } }
                    ],
                    "labels": [{ "id": "sign", "text": { "cel": "self.sign == 'up' ? '+' : ''" }, "at": 0.86, "side": "left", "distance": 11, "editable": false }]
                }
            }
        },
        "layout": { "algorithms": { "ring": { "algorithm": "circular" } }, "default": "ring" }
    })
}

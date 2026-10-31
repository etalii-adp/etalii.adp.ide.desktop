//! A parity script's gesture in ATD §2.3's terms (etalii.adp spec 067, T018 to T020, research R7): the target strings
//! of the parity scripts' vocabulary (`adp_parity`, `Replay::target`) do not reach a case. A point on the canvas is a
//! `create` of the toolbox entry whose drop runs the action, or the action's own `at` where no entry does; two ends are
//! a `connect`, with a new end as the `create` of the same step; an element is the `select` of the step. What ATD's
//! steps cannot state is named, and the step is written as the script has it (FR-007).

use std::collections::BTreeMap;

use adp_diagram::{DiagramDocuments, DiagramSession};
use serde_json::{Map, Number, Value as Json, json};

/// What the module says of a step's gesture just before it is made.
#[derive(Debug, Default)]
pub(super) struct Before {
    /// The toolbox entry whose drop runs the step's action.
    tool: Option<String>,
    /// Whether what the step targets can be selected.
    selectable: bool,
}

impl Before {
    pub(super) fn of(step: &Json, session: &DiagramSession, documents: &DiagramDocuments) -> Before {
        let action = step["action"].as_str().unwrap_or_default();
        Before {
            tool: session
                .toolbox(documents)
                .into_iter()
                .find(|item| item.drop_action_id == action)
                .map(|item| item.id),
            selectable: step["args"]["target"]
                .as_str()
                .is_some_and(|target| session.select(documents, target).is_some()),
        }
    }
}

/// How a step's gesture is written.
#[derive(Debug, PartialEq)]
pub(super) enum Written {
    /// In ATD's terms; `select` is the element the step's target named, which its `select` is to hold.
    Gesture {
        fields: Map<String, Json>,
        select: Option<String>,
    },
    /// As the script has it, and why ATD's steps cannot state it.
    Unstated { reason: String, fields: Map<String, Json> },
}

/// A place in a case: `x` and `y`, as a whole number where they are one, as the script wrote them.
fn at(point: &str) -> Option<Json> {
    let (x, y) = point.split_once(',')?;
    let number = |n: &str| -> Option<Json> {
        let trimmed = n.trim();
        match trimmed.parse::<i64>() {
            Ok(whole) => Some(json!(whole)),
            Err(_) => Number::from_f64(trimmed.parse().ok()?).map(Json::Number),
        }
    };
    Some(json!({"x": number(x)?, "y": number(y)?}))
}

/// Writes the action step `step`, its references resolved. `before` is what the module said just before it; `made`,
/// the elements it made, with their types and whether each is a relation.
pub(super) fn write(step: &Json, before: &Before, made: &BTreeMap<String, (String, bool)>) -> Written {
    let action = step["action"].as_str().unwrap_or_default();
    let mut args = step["args"].as_object().cloned().unwrap_or_default();
    let target = args.remove("target");
    let with_args = |mut fields: Map<String, Json>, args: Map<String, Json>| {
        if !args.is_empty() {
            fields.insert("args".into(), Json::Object(args));
        }
        fields
    };
    let unstated = |reason: String| {
        let mut args = args.clone();
        if let Some(target) = &target {
            args.insert("target".into(), target.clone());
        }
        Written::Unstated {
            reason,
            fields: with_args(Map::from_iter([("action".to_owned(), json!(action))]), args),
        }
    };
    let gesture = |fields: Map<String, Json>, select: Option<String>| Written::Gesture { fields, select };

    let Some(target) = target.as_ref().and_then(Json::as_str) else {
        return gesture(
            with_args(Map::from_iter([("action".to_owned(), json!(action))]), args.clone()),
            None,
        );
    };
    let named = format!("'{action}' on '{target}'");

    if let Some(point) = target.strip_prefix("new:") {
        let Some(place) = at(point) else {
            return unstated(format!("{named}: no place"));
        };
        return match &before.tool {
            Some(tool) => {
                let create = json!({"type": tool, "at": place});
                gesture(
                    with_args(Map::from_iter([("create".to_owned(), create)]), args.clone()),
                    None,
                )
            }
            // A gesture on the canvas that no entry of the toolbox makes: the action, told where.
            None => {
                let mut args = args.clone();
                args.insert("at".into(), place);
                gesture(
                    with_args(Map::from_iter([("action".to_owned(), json!(action))]), args),
                    None,
                )
            }
        };
    }

    if let Some(ends) = target.strip_prefix("rel:") {
        let Some((source, end)) = ends.split_once("->") else {
            return unstated(format!("{named}: not two ends"));
        };
        let Some(point) = end.strip_prefix("new:") else {
            let connect = json!({"type": action, "source": source, "target": end});
            return gesture(
                with_args(Map::from_iter([("connect".to_owned(), connect)]), args.clone()),
                None,
            );
        };
        let Some(place) = at(point) else {
            return unstated(format!("{named}: no place"));
        };
        // A new end is the `create` of the same step: what the gesture made, read back.
        let Some((id, (type_name, _))) = made.iter().find(|(_, (_, relation))| !relation) else {
            return unstated(format!("{named}: it made no end to name"));
        };
        let mut fields = Map::new();
        fields.insert(
            "connect".into(),
            json!({"type": action, "source": source, "target": id}),
        );
        fields.insert("create".into(), json!({"type": type_name, "at": place}));
        return gesture(with_args(fields, args.clone()), None);
    }

    if !before.selectable {
        return unstated(format!("{named}: nothing that can be selected has that id"));
    }
    gesture(
        with_args(Map::from_iter([("action".to_owned(), json!(action))]), args.clone()),
        Some(target.to_owned()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(action: &str, target: &str) -> Json {
        json!({"action": action, "args": {"target": target}})
    }

    fn before(tool: Option<&str>, selectable: bool) -> Before {
        Before {
            tool: tool.map(str::to_owned),
            selectable,
        }
    }

    fn fields(value: Json) -> Map<String, Json> {
        value.as_object().unwrap().clone()
    }

    fn nothing() -> BTreeMap<String, (String, bool)> {
        BTreeMap::new()
    }

    #[test]
    fn a_point_with_a_toolbox_entry_is_a_create_of_that_entry() {
        assert_eq!(
            write(
                &step("dependencies.add-element", "new:300,0"),
                &before(Some("dependencies.toolbox.node"), false),
                &nothing()
            ),
            Written::Gesture {
                fields: fields(json!({"create": {"type": "dependencies.toolbox.node", "at": {"x": 300, "y": 0}}})),
                select: None,
            }
        );
    }

    #[test]
    fn a_create_keeps_the_answer_to_its_prompt_and_a_fraction_of_its_place() {
        let step = json!({"action": "c4.add-person", "args": {"target": "new:620.4,-200.6", "value": "Ann"}});
        assert_eq!(
            write(&step, &before(Some("c4.toolbox.person"), false), &nothing()),
            Written::Gesture {
                fields: fields(json!({
                    "create": {"type": "c4.toolbox.person", "at": {"x": 620.4, "y": -200.6}},
                    "args": {"value": "Ann"},
                })),
                select: None,
            }
        );
    }

    #[test]
    fn a_point_without_a_toolbox_entry_is_the_actions_own_argument() {
        assert_eq!(
            write(&step("sankey.arrange", "new:0,0"), &before(None, false), &nothing()),
            Written::Gesture {
                fields: fields(json!({"action": "sankey.arrange", "args": {"at": {"x": 0, "y": 0}}})),
                select: None,
            }
        );
    }

    #[test]
    fn two_ends_are_a_connect_naming_them() {
        assert_eq!(
            write(
                &step("dependencies.connect", "rel:a->b"),
                &before(None, false),
                &nothing()
            ),
            Written::Gesture {
                fields: fields(json!({"connect": {"type": "dependencies.connect", "source": "a", "target": "b"}})),
                select: None,
            }
        );
    }

    #[test]
    fn a_new_end_is_the_create_of_the_same_step() {
        let made = BTreeMap::from([
            ("n".to_owned(), ("Node".to_owned(), false)),
            ("r".to_owned(), ("DependsOn".to_owned(), true)),
        ]);
        assert_eq!(
            write(
                &step("dependencies.connect", "rel:a->new:1400,0"),
                &before(None, false),
                &made
            ),
            Written::Gesture {
                fields: fields(json!({
                    "connect": {"type": "dependencies.connect", "source": "a", "target": "n"},
                    "create": {"type": "Node", "at": {"x": 1400, "y": 0}},
                })),
                select: None,
            }
        );
    }

    #[test]
    fn an_element_or_a_prefixed_target_is_the_selection_of_the_step() {
        for target in ["message-broker", "task:publish", "edge:a->b", "res:http://x#y"] {
            assert_eq!(
                write(&step("x.remove", target), &before(None, true), &nothing()),
                Written::Gesture {
                    fields: fields(json!({"action": "x.remove"})),
                    select: Some(target.to_owned()),
                }
            );
        }
    }

    #[test]
    fn a_step_without_a_target_is_written_as_it_is() {
        let step = json!({"action": "x.remove", "args": {"confirm": true}});
        assert_eq!(
            write(&step, &before(None, false), &nothing()),
            Written::Gesture {
                fields: fields(json!({"action": "x.remove", "args": {"confirm": true}})),
                select: None,
            }
        );
    }

    #[test]
    fn what_cannot_be_stated_is_named_and_stays_as_the_script_has_it() {
        for (target, says) in [
            ("s#group", "nothing that can be selected"),
            ("new:here", "no place"),
            ("rel:a", "not two ends"),
            ("rel:a->new:1,2", "made no end"),
        ] {
            match write(&step("x.go", target), &before(None, false), &nothing()) {
                Written::Unstated { reason, fields: kept } => {
                    assert!(reason.contains(says) && reason.contains(target), "{reason}");
                    assert_eq!(kept, fields(step("x.go", target)));
                }
                written => panic!("{target}: {written:?}"),
            }
        }
    }
}

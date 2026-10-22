//! The refusals of a gesture (DISL section 8.4; the standalone host's `GestureConstraintEvaluator`): every declared
//! gesture constraint of its kind whose scope takes the element, whose enforcement prevents, whose `when` holds and
//! whose `rule` does not, in the order of `rules`, each with its message evaluated in the gesture's context.
//!
//! Declared constraints only: the built-ins that also refuse (section 8.4, step 3) are checked by the host where it
//! has them. A rule that fails to evaluate refuses: a gesture is let through only when its rule is seen to hold. A
//! `when` that fails does not apply the rule.

use adp_cel::{Map, Value};

use super::env::DislEnv;
use super::evaluation::{self, Vars};
use super::finding::DislFinding;
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::specification::DislSpecification;
use crate::model::budgets;
use crate::model::element::DislElement;

/// A change of `attribute` of `element` from its value to `new_value`.
pub fn change(
    specification: &DislSpecification,
    element: &DislElement,
    attribute: &str,
    new_value: Value,
    env: Option<&DislEnv>,
) -> Vec<DislFinding> {
    let old_value = if element.element_type().attributes.contains_key(attribute) {
        element.value_of(attribute)
    } else {
        Value::Null
    };
    refusals(
        specification,
        "change",
        element,
        None,
        evaluation::vars([
            ("attribute", Value::from(attribute)),
            ("oldValue", old_value),
            ("newValue", new_value),
        ]),
        env,
    )
}

/// A placement of `element`: a `gesture` (`move`, `resize`, `reparent`) from `old_bounds` to `new_bounds`, in domain
/// values.
pub fn placement(
    specification: &DislSpecification,
    element: &DislElement,
    gesture: &str,
    old_bounds: Map,
    new_bounds: Map,
    new_parent: Option<&DislElement>,
    env: Option<&DislEnv>,
) -> Vec<DislFinding> {
    refusals(
        specification,
        "placement",
        element,
        None,
        evaluation::vars([
            ("gesture", Value::from(gesture)),
            ("oldBounds", Value::map(old_bounds)),
            ("newBounds", Value::map(new_bounds)),
            (
                "newParent",
                new_parent.map(DislElement::to_value).unwrap_or(Value::Null),
            ),
        ]),
        env,
    )
}

/// A connect gesture drawing a relation of `relation_type` from `source` to `target`, before the relation exists:
/// `self` is the source, and a rule's `scope` names the relation type or one it inherits from.
pub fn connect(
    specification: &DislSpecification,
    relation_type: &str,
    source: &DislElement,
    target: &DislElement,
    env: Option<&DislEnv>,
) -> Vec<DislFinding> {
    let declared = specification.metamodel().type_of(relation_type);
    let in_scope = |name: &str| declared.is_some_and(|t| t.is_a(name));
    refusals(
        specification,
        "connect",
        source,
        Some(&in_scope),
        evaluation::vars([
            ("relationType", Value::from(relation_type)),
            ("source", source.to_value()),
            ("target", target.to_value()),
            ("sourcePort", Value::Null),
            ("targetPort", Value::Null),
        ]),
        env,
    )
}

/// The refusals of a gesture of `kind` on `element`, its own variables in `gesture`; a rule's `scope` is judged by
/// `in_scope` where given, else by the element's type.
fn refusals(
    specification: &DislSpecification,
    kind: &str,
    element: &DislElement,
    in_scope: Option<&dyn Fn(&str) -> bool>,
    gesture: Vars,
    env: Option<&DislEnv>,
) -> Vec<DislFinding> {
    // A withheld edit is refused before every constraint (section 3.2.1); a move or a resize writes view data only.
    let view_data =
        kind == "placement" && matches!(gesture.get("gesture"), Some(Value::String(g)) if g == "move" || g == "resize");
    if let Some(diagram) = element.try_diagram()
        && let Some(reason) = budgets::withheld_reason(&diagram, view_data)
    {
        return vec![DislFinding {
            code: budgets::WITHHELD.to_owned(),
            constraint_id: "budget".to_owned(),
            severity: "error".to_owned(),
            message: reason,
            element_ids: vec![element.id()],
            line: element.line(),
            file: None,
            subject: None,
        }];
    }
    let mut refusals = Vec::new();
    let Some(rules) = json::items(json::member(specification.root(), "constraints"), "rules") else {
        return refusals;
    };
    let context = contexts::of_constraint_kind(Some(kind));
    let mut variables = gesture;
    variables.insert("self".into(), element.to_value());
    variables.insert(
        "diagram".into(),
        element.try_diagram().map(|d| d.to_value()).unwrap_or(Value::Null),
    );
    variables.insert("env".into(), Value::map(env.cloned().unwrap_or_default().to_cel()));
    for (index, rule) in rules.iter().enumerate() {
        let pointer = json::pointer_at("/constraints/rules", index);
        if json::string_of(rule, "kind") != Some(kind) {
            continue;
        }
        if matches!(json::get(rule, "enabled"), Some(Json::Bool(false))) {
            continue;
        }
        if !matches!(
            json::string_of(rule, "enforcement").unwrap_or("prevent"),
            "prevent" | "prevent-and-report"
        ) {
            continue;
        }
        let scope = json::strings_of(rule, "scope");
        let applies = |s: &String| match in_scope {
            Some(in_scope) => in_scope(s),
            None => element.is_a(s),
        };
        if !scope.is_empty() && !scope.iter().any(applies) && !scope.iter().any(|s| s == "*") {
            continue;
        }
        if let Some(when) = json::get(rule, "when")
            && !evaluation::holds(
                specification,
                when,
                &json::pointer(&pointer, "when"),
                context,
                &variables,
            )
        {
            continue;
        }
        if let Some(condition) = json::get(rule, "rule")
            && matches!(
                evaluation::expression(
                    specification,
                    condition,
                    &json::pointer(&pointer, "rule"),
                    context,
                    &variables
                ),
                Ok(Value::Bool(true))
            )
        {
            continue;
        }
        let id = json::string_of(rule, "id")
            .map(str::to_owned)
            .unwrap_or_else(|| pointer.clone());
        let message = match json::get(rule, "message") {
            Some(text) => evaluation::message(
                specification,
                text,
                &json::pointer(&pointer, "message"),
                context,
                &variables,
                &id,
            ),
            None => id.clone(),
        };
        refusals.push(DislFinding {
            code: json::string_of(rule, "code")
                .map(str::to_owned)
                .unwrap_or_else(|| id.clone()),
            constraint_id: id,
            severity: json::string_of(rule, "severity").unwrap_or("error").to_owned(),
            message,
            element_ids: vec![element.id()],
            line: element.line(),
            file: None,
            subject: None,
        });
    }
    refusals
}

//! The findings of a model (DISL sections 8.3, 8.6 and 8.7; the standalone host's `ConstraintEvaluator`): the
//! reader's, the configured built-ins' and every live invariant's, each with its code, severity, message and line.
//!
//! Built-ins: `std.unparseable` and `std.unreadableEntry` as the reader raised them; `std.duplicateId` over the
//! written ids; `std.endpoints` from each relation type's ends, `allowSelfLoops` and `allowParallel`;
//! `std.references` for a relation end that names nothing. Each takes its `code`, `severity`, `message` and `enabled`
//! from `constraints.builtIn` - a `code` that is a `{cel}` object is computed per finding in the message's context
//! (DISL 0.3 section 8.1) - and `oncePerGroup` (`second` or `last`), which reports a group of duplicates once.
//!
//! Order: without `constraints.order`, section 8.6's - the reader's findings, the built-ins, then the rules in
//! declaration order, each by element in model order. With it (a list of codes), findings sort by the position of
//! their code in it, codes it does not list last; then the reader's findings in reading order, then the rest by the
//! line of the element a rule was evaluated for, a group of duplicates by the line of its first member.
//!
//! Where a finding is (sections 8.2 and 8.6): a rule's `location`, else the `location()` of its first target with a
//! line, else of its first target; a built-in's is the element it is about; a reader's is the file and line the
//! reader gave. A file that does not parse has no other finding: with a `std.unparseable` finding for the primary
//! file nothing else is evaluated; with one naming another file of a folder, only the findings located in that file
//! are dropped. An expression that fails proves nothing: a `when` that fails skips the rule, a `rule` that fails
//! raises no finding, a message that fails is the built-in's or the rule's id.

use std::collections::{HashMap, HashSet};

use adp_cel::{Map, Value};

use super::evaluation::{self, Vars};
use super::finding::{DislConstraintOptions, DislFinding, UNPARSEABLE};
use crate::expressions::contexts;
use crate::json::{self, Json};
use crate::loading::metamodel::DislRelationEnd;
use crate::loading::specification::DislSpecification;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;
use crate::model::written::WrittenElement;

/// The findings of `diagram` under `specification`.
pub fn evaluate(
    specification: &DislSpecification,
    diagram: &DislDiagram,
    options: &DislConstraintOptions,
) -> Vec<DislFinding> {
    let mut run = Run {
        specification,
        diagram,
        options,
        found: Vec::new(),
        constraints: json::get(specification.root(), "constraints")
            .filter(|c| c.is_object())
            .cloned()
            .unwrap_or(Json::Null),
        env: options.env.clone().unwrap_or_default().to_cel(),
        unparseable: HashSet::new(),
        primary_unparseable: false,
    };
    run.reader();
    if !run.primary_unparseable {
        run.duplicate_ids();
        run.endpoints();
        run.references();
        run.rules();
    }
    run.ordered()
}

struct Run<'a> {
    specification: &'a DislSpecification,
    diagram: &'a DislDiagram,
    options: &'a DislConstraintOptions,
    /// Each finding, its place in reading order (or -1), and the line it is ordered by.
    found: Vec<(DislFinding, i64, Option<i32>)>,
    constraints: Json,
    env: Map,
    unparseable: HashSet<String>,
    primary_unparseable: bool,
}

/// One built-in's setting (section 8.1), with the runtime's defaults.
struct BuiltIn<'a> {
    specification: &'a DislSpecification,
    id: String,
    json: &'a Json,
    pointer: String,
}

impl BuiltIn<'_> {
    fn enabled(&self) -> bool {
        self.specification.is_built_in_enabled(&self.id)
    }

    fn severity(&self) -> String {
        json::string_of(self.json, "severity")
            .map(str::to_owned)
            .unwrap_or_else(|| match self.id.as_str() {
                "std.unreadableEntry" | "std.duplicateId" | "std.missingId" => "warning".into(),
                _ => "error".into(),
            })
    }

    fn once_per_group(&self) -> Option<&str> {
        json::string_of(self.json, "oncePerGroup")
    }

    fn code_of(&self, variables: &Vars) -> String {
        match json::get(self.json, "code") {
            Some(Json::String(code)) => code.clone(),
            Some(code @ Json::Object(_)) => match evaluation::expression(
                self.specification,
                code,
                &json::pointer(&self.pointer, "code"),
                contexts::BUILT_IN_MESSAGE,
                variables,
            ) {
                Ok(Value::String(computed)) => computed,
                _ => self.id.clone(),
            },
            _ => self.id.clone(),
        }
    }

    fn message_of(&self, variables: &Vars, fallback: &str) -> String {
        match json::get(self.json, "message") {
            Some(message) => evaluation::message(
                self.specification,
                message,
                &json::pointer(&self.pointer, "message"),
                contexts::BUILT_IN_MESSAGE,
                variables,
                fallback,
            ),
            None => fallback.to_owned(),
        }
    }
}

fn flagged<'e>(group: &'e [DislElement], setting: &BuiltIn<'_>) -> Vec<&'e DislElement> {
    match setting.once_per_group() {
        Some("second") => group.get(1).into_iter().collect(),
        Some("last") => group.last().into_iter().collect(),
        _ => group.iter().skip(1).collect(),
    }
}

fn allows(end: Option<&DislRelationEnd>, element: &DislElement) -> bool {
    match end {
        None => true,
        Some(end) => {
            (end.types.is_empty() || end.types.iter().any(|t| element.is_a(t)))
                && !end.exclude.iter().any(|t| element.is_a(t))
        }
    }
}

fn setting_of<'s>(specification: &'s DislSpecification, id: &str) -> BuiltIn<'s> {
    BuiltIn {
        specification,
        id: id.to_owned(),
        json: specification.built_in_setting(id),
        pointer: json::pointer("/constraints/builtIn", id),
    }
}

impl Run<'_> {
    fn variables(&self, self_value: Value, detail: Option<Map>) -> Vars {
        let mut variables = evaluation::vars([
            ("self", self_value),
            ("diagram", self.diagram.to_value()),
            ("env", Value::map(self.env.clone())),
        ]);
        if let Some(detail) = detail {
            variables.insert("detail".into(), Value::map(detail));
        }
        variables
    }

    fn written_id(&self, element: &DislElement) -> String {
        match &self.options.written_id {
            Some(written) => written(element),
            None => element.id(),
        }
    }

    /// An element as its findings name it: by its written id when that is not its model id.
    fn view(&self, element: &DislElement) -> Value {
        let written = self.written_id(element);
        if written != element.id() {
            Value::object(WrittenElement::new(element.clone(), written))
        } else {
            element.to_value()
        }
    }

    fn reader(&mut self) {
        let mut reading = 0;
        for finding in &self.options.reader_findings {
            let setting = setting_of(self.specification, &finding.built_in);
            if !setting.enabled() {
                continue;
            }
            if finding.built_in == UNPARSEABLE {
                if finding.file.is_none() || finding.file == self.diagram.file() {
                    self.primary_unparseable = true;
                } else if let Some(file) = &finding.file {
                    self.unparseable.insert(file.clone());
                }
            }
            let variables = self.variables(self.diagram.to_value(), Some(finding.detail.clone()));
            let fallback = match finding.detail.get("reason") {
                Some(Value::String(reason)) => reason.clone(),
                _ => finding.built_in.clone(),
            };
            let made = DislFinding {
                code: setting.code_of(&variables),
                constraint_id: finding.built_in.clone(),
                severity: setting.severity(),
                message: setting.message_of(&variables, &fallback),
                element_ids: Vec::new(),
                line: finding.line,
                file: finding.file.clone(),
                subject: None,
            };
            self.found.push((made, reading, finding.line));
            reading += 1;
        }
    }

    /// A built-in's finding on `element`, ordered as `position` is in the document.
    fn add(&mut self, built_in: &str, element: &DislElement, detail: Map, fallback: &str, position: &DislElement) {
        let setting = setting_of(self.specification, built_in);
        let variables = self.variables(self.view(element), Some(detail));
        let made = DislFinding {
            code: setting.code_of(&variables),
            constraint_id: built_in.to_owned(),
            severity: setting.severity(),
            message: setting.message_of(&variables, fallback),
            element_ids: vec![self.written_id(element)],
            line: element.line(),
            file: element.location().0,
            subject: None,
        };
        self.found.push((made, -1, position.line()));
    }

    /// `std.duplicateId`: per written id held by several stored elements, each later holder, in reading order.
    fn duplicate_ids(&mut self) {
        let setting = setting_of(self.specification, "std.duplicateId");
        if !setting.enabled() {
            return;
        }
        let mut stored: Vec<DislElement> = self
            .diagram
            .elements()
            .into_iter()
            .filter(|e| !e.is_derived() && !self.written_id(e).is_empty())
            .collect();
        stored.sort_by_key(|e| e.line().unwrap_or(i32::MAX));
        let mut groups: Vec<(String, Vec<DislElement>)> = Vec::new();
        for element in stored {
            let id = self.written_id(&element);
            match groups.iter_mut().find(|(k, _)| *k == id) {
                Some((_, members)) => members.push(element),
                None => groups.push((id, vec![element])),
            }
        }
        for (id, members) in groups.into_iter().filter(|(_, m)| m.len() > 1) {
            let setting = setting_of(self.specification, "std.duplicateId");
            let chosen: Vec<DislElement> = flagged(&members, &setting).into_iter().cloned().collect();
            for element in chosen {
                let mut detail = Map::default();
                detail.insert("id", Value::String(id.clone()));
                detail.insert("first", members[0].to_value());
                detail.insert("count", Value::Int(members.len() as i64));
                self.add(
                    "std.duplicateId",
                    &element,
                    detail,
                    &format!("The id '{id}' is declared more than once."),
                    &members[0],
                );
            }
        }
    }

    /// `std.endpoints`: per stored relation with both ends, an end of a type it does not allow, a loop it does not
    /// allow, a parallel it does not allow.
    fn endpoints(&mut self) {
        if !setting_of(self.specification, "std.endpoints").enabled() {
            return;
        }
        let mut parallels: Vec<((String, usize, usize), Vec<DislElement>)> = Vec::new();
        let mut violations: Vec<(DislElement, &str)> = Vec::new();
        for relation in self.diagram.relations().into_iter().filter(|r| !r.is_derived()) {
            let (Some(source), Some(target)) = (relation.source(), relation.target()) else {
                continue;
            };
            let t = relation.element_type();
            if !allows(t.source.as_ref(), &source) {
                violations.push((relation.clone(), "source"));
            }
            if !allows(t.target.as_ref(), &target) {
                violations.push((relation.clone(), "target"));
            }
            if source == target && !t.allow_self_loops {
                violations.push((relation.clone(), "selfLoop"));
            }
            if !t.allow_parallel {
                let directed = t.directed;
                let (a, b) = (source.key(), target.key());
                let key = if directed || a <= b {
                    (t.name.clone(), a, b)
                } else {
                    (t.name.clone(), b, a)
                };
                let group = match parallels.iter_mut().position(|(k, _)| *k == key) {
                    Some(index) => &mut parallels[index].1,
                    None => {
                        parallels.push((key, Vec::new()));
                        &mut parallels.last_mut().expect("just pushed").1
                    }
                };
                group.push(relation.clone());
                if group.len() > 1 {
                    violations.push((relation.clone(), "parallel"));
                }
            }
        }

        for (relation, violation) in violations {
            let mut position = relation.clone();
            if violation == "parallel" {
                let Some((_, group)) = parallels.iter().find(|(_, members)| members.contains(&relation)) else {
                    continue;
                };
                let setting = setting_of(self.specification, "std.endpoints");
                if !flagged(group, &setting).contains(&&relation) {
                    continue;
                }
                position = group[0].clone();
            }
            let end = if matches!(violation, "source" | "target") {
                violation
            } else {
                "target"
            };
            let type_name = relation.type_name();
            let mut detail = Map::default();
            detail.insert("violation", Value::from(violation));
            detail.insert("end", Value::from(end));
            detail.insert("relationType", Value::String(type_name.clone()));
            let fallback = match violation {
                "selfLoop" => format!("A {type_name} cannot connect an element to itself."),
                "parallel" => format!("A {type_name} already connects these two elements."),
                _ => format!("A {type_name} cannot have this {end}."),
            };
            self.add("std.endpoints", &relation, detail, &fallback, &position);
        }
    }

    /// `std.references`: per stored relation, an end that names nothing, the source before the target.
    fn references(&mut self) {
        if !setting_of(self.specification, "std.references").enabled() {
            return;
        }
        for relation in self.diagram.relations().into_iter().filter(|r| !r.is_derived()) {
            if relation.source().is_none() {
                self.missing(&relation, "source", relation.source_id());
            }
            let optional = relation.element_type().target.as_ref().is_some_and(|t| t.optional);
            if relation.target().is_none() && !(relation.target_id().is_none() && optional) {
                self.missing(&relation, "target", relation.target_id());
            }
        }
    }

    fn missing(&mut self, relation: &DislElement, end: &str, id: Option<String>) {
        let id = id.unwrap_or_default();
        let mut detail = Map::default();
        detail.insert("end", Value::from(end));
        detail.insert("missingId", Value::String(id.clone()));
        detail.insert("attribute", Value::from(end));
        let fallback = format!("The {end} '{id}' of this {} names nothing.", relation.type_name());
        self.add("std.references", relation, detail, &fallback, relation);
    }

    /// The live invariants, in declaration order, each per scope element in model order.
    fn rules(&mut self) {
        let Some(rules) = json::items(&self.constraints, "rules").cloned() else {
            return;
        };
        let defaults = json::member(&self.constraints, "defaults").clone();
        let default_timing = json::strings_of(&defaults, "timing");
        let default_severity = json::string_of(&defaults, "severity").map(str::to_owned);
        for (index, rule) in rules.iter().enumerate() {
            let pointer = json::pointer_at("/constraints/rules", index);
            if json::string_of(rule, "kind").is_some_and(|k| k != "invariant") {
                continue;
            }
            if matches!(json::get(rule, "enabled"), Some(Json::Bool(false))) {
                continue;
            }
            let own = json::strings_of(rule, "timing");
            let timing = if !own.is_empty() {
                own
            } else if !default_timing.is_empty() {
                default_timing.clone()
            } else {
                vec!["live".into(), "save".into()]
            };
            if !timing.iter().any(|t| t == "live") {
                continue;
            }
            for scoped in self.scope(rule) {
                self.rule(rule, &pointer, scoped, default_severity.as_deref());
            }
        }
    }

    fn scope(&self, rule: &Json) -> Vec<Value> {
        let scope = json::strings_of(rule, "scope");
        if scope.is_empty() || scope.iter().any(|s| s == "diagram") {
            return vec![self.diagram.to_value()];
        }
        let has = |word: &str| scope.iter().any(|s| s == word);
        self.diagram
            .elements()
            .iter()
            .filter(|e| {
                has("*")
                    || (has("node") && !e.is_relation())
                    || (has("relation") && e.is_relation())
                    || scope.iter().any(|s| e.is_a(s))
            })
            .map(|e| self.view(e))
            .collect()
    }

    fn rule(&mut self, rule: &Json, pointer: &str, scoped: Value, default_severity: Option<&str>) {
        let variables = self.variables(scoped.clone(), None);
        if let Some(for_each) = json::get(rule, "forEach") {
            let Ok(Value::List(items)) = evaluation::expression(
                self.specification,
                for_each,
                &json::pointer(pointer, "forEach"),
                contexts::CONSTRAINT,
                &variables,
            ) else {
                return;
            };
            for (index, item) in items.iter().enumerate() {
                let mut bound = variables.clone();
                bound.insert("item".into(), item.clone());
                bound.insert("index".into(), Value::Int(index as i64));
                self.one(
                    rule,
                    pointer,
                    &scoped,
                    &bound,
                    DislElement::from_value(item),
                    default_severity,
                );
            }
            return;
        }
        self.one(rule, pointer, &scoped, &variables, None, default_severity);
    }

    fn one(
        &mut self,
        rule: &Json,
        pointer: &str,
        scoped: &Value,
        bound: &Vars,
        item: Option<DislElement>,
        default_severity: Option<&str>,
    ) {
        let specification = self.specification;
        let evaluate = |key: &str| {
            json::get(rule, key).map(|json| {
                evaluation::expression(
                    specification,
                    json,
                    &json::pointer(pointer, key),
                    contexts::CONSTRAINT,
                    bound,
                )
            })
        };
        if let Some(when) = json::get(rule, "when")
            && !evaluation::holds(
                specification,
                when,
                &json::pointer(pointer, "when"),
                contexts::CONSTRAINT,
                bound,
            )
        {
            return;
        }
        let holds = evaluate("rule").unwrap_or(Ok(Value::Bool(false)));
        if !matches!(holds, Ok(Value::Bool(false))) {
            return;
        }

        let id = json::string_of(rule, "id")
            .map(str::to_owned)
            .unwrap_or_else(|| pointer.to_owned());
        let code = json::string_of(rule, "code")
            .map(str::to_owned)
            .unwrap_or_else(|| id.clone());
        let targets = self.targets(rule, pointer, scoped, bound, item.as_ref());
        let severity = match json::get(rule, "severity") {
            Some(Json::String(s)) => s.clone(),
            Some(_) => match evaluate("severity") {
                Some(Ok(Value::String(s))) => s,
                _ => default_severity.unwrap_or("error").to_owned(),
            },
            None => default_severity.unwrap_or("error").to_owned(),
        };
        let message = match json::get(rule, "message") {
            Some(text) => evaluation::message(
                specification,
                text,
                &json::pointer(pointer, "message"),
                contexts::CONSTRAINT,
                bound,
                &id,
            ),
            None => id.clone(),
        };
        let at = targets
            .iter()
            .find(|e| e.line().is_some())
            .or_else(|| targets.first())
            .cloned();
        let at_location = || at.as_ref().map(DislElement::location).unwrap_or((None, None));
        let (file, line) = match evaluate("location") {
            Some(located) => located_at(located, &at_location),
            None => at_location(),
        };
        let subject = match evaluate("subject") {
            Some(Ok(Value::String(s))) => Some(s),
            _ => None,
        };
        let ordered = unwrapped(scoped)
            .or(item)
            .and_then(|e| e.line())
            .or_else(|| at.as_ref().and_then(DislElement::line));
        let element_ids = targets.iter().map(|t| self.written_id(t)).collect();
        self.found.push((
            DislFinding {
                code,
                constraint_id: id,
                severity,
                message,
                element_ids,
                line,
                file,
                subject,
            },
            -1,
            ordered,
        ));
    }

    fn targets(
        &self,
        rule: &Json,
        pointer: &str,
        scoped: &Value,
        bound: &Vars,
        item: Option<&DislElement>,
    ) -> Vec<DislElement> {
        if let Some(target) = json::get(rule, "target")
            && target.as_str() != Some("self")
        {
            return match evaluation::expression(
                self.specification,
                target,
                &json::pointer(pointer, "target"),
                contexts::CONSTRAINT,
                bound,
            ) {
                Ok(Value::List(list)) => list.iter().filter_map(DislElement::from_value).collect(),
                Ok(one) => DislElement::from_value(&one).into_iter().collect(),
                Err(_) => Vec::new(),
            };
        }
        if let Some(item) = item {
            return vec![item.clone()];
        }
        unwrapped(scoped).into_iter().collect()
    }

    /// The findings in the order of `constraints.order`, or as they arose, without those located in a file of the
    /// folder that did not parse (section 8.6), that file's own finding apart.
    fn ordered(self) -> Vec<DislFinding> {
        let unparseable = self.unparseable;
        let kept: Vec<(DislFinding, i64, Option<i32>)> = self
            .found
            .into_iter()
            .filter(|(f, _, _)| {
                unparseable.is_empty()
                    || f.constraint_id == UNPARSEABLE
                    || f.file.as_ref().is_none_or(|file| !unparseable.contains(file))
            })
            .collect();
        let order = json::get(&self.constraints, "order")
            .map(json::strings)
            .unwrap_or_default();
        if order.is_empty() {
            return kept.into_iter().map(|(f, _, _)| f).collect();
        }
        let positions: HashMap<&str, usize> = order.iter().enumerate().rev().map(|(i, c)| (c.as_str(), i)).collect();
        let mut keyed: Vec<_> = kept.into_iter().enumerate().collect();
        keyed.sort_by_key(|(arising, (f, reading, position))| {
            (
                positions.get(f.code.as_str()).copied().unwrap_or(order.len()),
                if *reading >= 0 { 0 } else { 1 },
                if *reading >= 0 {
                    *reading
                } else {
                    i64::from(position.unwrap_or(i32::MAX))
                },
                *arising,
            )
        });
        keyed.into_iter().map(|(_, (f, _, _))| f).collect()
    }
}

/// A rule's evaluated `location` (section 8.2): a SourceLocation map, optional or not, gives its file and line;
/// `null` or `optional.none()` gives none. One that fails, or that names no file, proves nothing, and the finding is
/// located at its target.
fn located_at(
    value: Result<Value, String>,
    target: &dyn Fn() -> (Option<String>, Option<i32>),
) -> (Option<String>, Option<i32>) {
    let value = match value {
        Ok(Value::Optional(None)) => Value::Null,
        Ok(Value::Optional(Some(inner))) => (*inner).clone(),
        Ok(other) => other,
        Err(_) => return target(),
    };
    if value.is_null() {
        return (None, None);
    }
    let Some(map) = value.as_map() else {
        return target();
    };
    let Some(Value::String(file)) = map.get("file") else {
        return target();
    };
    let line = match map.get("line") {
        Some(Value::Int(line)) if *line >= 1 && *line <= i64::from(i32::MAX) => Some(*line as i32),
        _ => None,
    };
    (Some(file.clone()), line)
}

fn unwrapped(value: &Value) -> Option<DislElement> {
    DislElement::from_value(value).or_else(|| value.downcast::<WrittenElement>().map(|w| w.element().clone()))
}

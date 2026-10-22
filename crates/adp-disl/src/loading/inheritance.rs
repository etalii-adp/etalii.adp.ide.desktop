//! Step 6 of loading (DISL section 14.1; the standalone host's `DislInheritance`): the metamodel's types with their
//! supertypes resolved, their C3 linearisations computed, and their attributes, containment and label attribute
//! merged along them (section 4.7), the relation ends resolved (section 4.9), and the relation flags inherited and
//! checked (DISL 0.6, section 4.7).

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use indexmap::IndexMap;

use super::diagnostic::DislDiagnostic;
use super::loader::is_reserved;
use super::metamodel::{DislAttribute, DislEnum, DislEnumValue, DislMetamodel, DislRelationEnd, DislType, PRIMITIVES};
use crate::json::{self, Json, NOTHING};

/// A type's attributes, child types and label attribute, merged along its linearisation.
type Merged = (IndexMap<String, DislAttribute>, Vec<String>, Option<String>);

/// A declaration as it is written, member order included, for telling two apart.
fn raw(json: &Json) -> String {
    serde_json::to_string(json).unwrap_or_default()
}

/// The names an attribute's type may be: a primitive, an enum, a data type, or a node or relation type.
struct TypeNames(HashSet<String>);

impl TypeNames {
    fn contains(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

pub(crate) fn build(metamodel: &Json, diagnostics: &mut Vec<DislDiagnostic>) -> DislMetamodel {
    let enums = enums(metamodel);
    let data_types: Vec<String> = json::members(metamodel, "dataTypes").map(|(n, _)| n.clone()).collect();
    let mut types = declared(metamodel, "types", false);
    let mut relations = declared(metamodel, "relations", true);

    for name in types.keys().filter(|n| relations.contains_key(*n)) {
        diagnostics.push(DislDiagnostic::error(
            json::pointer("/metamodel/relations", name),
            format!("'{name}' names a node type and a relation type; they share one namespace (DISL §2.2)."),
        ));
    }
    for name in enums.keys().chain(data_types.iter()) {
        if types.contains_key(name) || relations.contains_key(name) {
            diagnostics.push(DislDiagnostic::error(
                "/metamodel",
                format!("'{name}' names an enum or data type and a node or relation type (DISL §2.2)."),
            ));
        }
    }

    supertypes(&mut types, "/metamodel/types", "node", diagnostics);
    supertypes(&mut relations, "/metamodel/relations", "relation", diagnostics);
    linearise(&mut types, "/metamodel/types", diagnostics);
    linearise(&mut relations, "/metamodel/relations", diagnostics);
    flags(&mut relations, diagnostics);

    let mut all: IndexMap<String, DislType> = IndexMap::new();
    for (name, t) in types.iter().chain(relations.iter()) {
        all.insert(name.clone(), t.clone());
    }
    let mut known: HashSet<String> = PRIMITIVES.iter().map(|p| (*p).to_owned()).collect();
    known.extend(enums.keys().cloned());
    known.extend(data_types.iter().cloned());
    known.extend(all.keys().cloned());
    let known = TypeNames(known);

    let mut merged: HashMap<String, Merged> = HashMap::new();
    for t in all.values() {
        let section = if t.is_relation {
            "/metamodel/relations"
        } else {
            "/metamodel/types"
        };
        merged.insert(
            t.name.clone(),
            merge(t, &all, &json::pointer(section, &t.name), &known, diagnostics),
        );
    }
    for t in types.values_mut().chain(relations.values_mut()) {
        if let Some((attributes, children, label)) = merged.get(&t.name) {
            t.attributes = attributes.clone();
            t.child_types = children.clone();
            t.label_attribute = label.clone();
        }
    }
    for relation in relations.values_mut() {
        let pointer = json::pointer("/metamodel/relations", &relation.name);
        relation.source = Some(end(relation, "source", &all, &pointer, diagnostics));
        relation.target = Some(end(relation, "target", &all, &pointer, diagnostics));
    }

    let diagram = json::member(metamodel, "diagram");
    let diagram_attributes = attributes(diagram, "diagram", "/metamodel/diagram", &known, Some(diagnostics));
    DislMetamodel {
        diagram_attributes,
        enums: Rc::new(enums),
        types: types.into_iter().map(|(k, v)| (k, Rc::new(v))).collect(),
        relations: relations.into_iter().map(|(k, v)| (k, Rc::new(v))).collect(),
        data_types,
    }
}

fn declared(metamodel: &Json, section: &str, relation: bool) -> IndexMap<String, DislType> {
    json::members(metamodel, section)
        .map(|(name, value)| (name.clone(), DislType::new(name, relation, value.clone())))
        .collect()
}

fn enums(metamodel: &Json) -> IndexMap<String, DislEnum> {
    json::members(metamodel, "enums")
        .map(|(name, value)| {
            let values = json::members(value, "values")
                .map(|(key, v)| DislEnumValue {
                    key: key.clone(),
                    stored: json::string_of(v, "value").unwrap_or(key).to_owned(),
                    label: json::string_of(v, "label").map(str::to_owned),
                })
                .collect();
            (
                name.clone(),
                DislEnum {
                    name: name.clone(),
                    values,
                    ordered: json::bool_of(value, "ordered"),
                },
            )
        })
        .collect()
}

/// Each type's `extends`, every name a type of the same kind (section 4.7).
fn supertypes(
    types: &mut IndexMap<String, DislType>,
    section: &str,
    kind: &str,
    diagnostics: &mut Vec<DislDiagnostic>,
) {
    let names: HashSet<String> = types.keys().cloned().collect();
    for t in types.values_mut() {
        let extends = json::strings_of(&t.json, "extends");
        for parent in extends.iter().filter(|p| !names.contains(*p)) {
            diagnostics.push(DislDiagnostic::error(
                json::pointer(&json::pointer(section, &t.name), "extends"),
                format!(
                    "'{}' extends '{parent}', which is not a {kind} type of this specification (DISL §4.7).",
                    t.name
                ),
            ));
        }
        t.extends = extends.into_iter().filter(|p| names.contains(p)).collect();
    }
}

/// The relation flags (section 4.9) with their defaults.
const RELATION_FLAGS: [(&str, bool); 3] = [("directed", true), ("allowSelfLoops", false), ("allowParallel", true)];

/// Each relation type's `directed`, `allowSelfLoops` and `allowParallel` (DISL 0.6, section 4.7): the value of the
/// nearest type of its linearisation that states it, else the section 4.9 default. A stated flag only narrows every
/// direct supertype's value (`true` to `false`; `directed` never differs), and direct supertypes that differ in a
/// flag the type does not state are an error, as for conflicting inherited attributes.
fn flags(relations: &mut IndexMap<String, DislType>, diagnostics: &mut Vec<DislDiagnostic>) {
    let stated = |t: &DislType, flag: &str| json::get(&t.json, flag).and_then(Json::as_bool);
    let effective = |relations: &IndexMap<String, DislType>, name: &str, flag: &str, default: bool| {
        relations[name]
            .linearisation
            .iter()
            .filter_map(|ancestor| relations.get(ancestor).and_then(|t| stated(t, flag)))
            .next()
            .unwrap_or(default)
    };
    let mut values = Vec::new();
    for (name, relation) in relations.iter() {
        let pointer = json::pointer("/metamodel/relations", name);
        let [directed, allow_self_loops, allow_parallel] = RELATION_FLAGS.map(|(flag, default)| {
            let inherited: Vec<(&String, bool)> = relation
                .extends
                .iter()
                .map(|parent| (parent, effective(relations, parent, flag, default)))
                .collect();
            match stated(relation, flag) {
                Some(own) => {
                    for (parent, value) in &inherited {
                        if flag == "directed" && own != *value {
                            diagnostics.push(DislDiagnostic::error(
                                json::pointer(&pointer, flag),
                                format!("'{name}' states {flag} {own}, which differs from the {value} of its supertype '{parent}' (DISL §4.7)."),
                            ));
                        } else if flag != "directed" && own && !value {
                            diagnostics.push(DislDiagnostic::error(
                                json::pointer(&pointer, flag),
                                format!("'{name}' states {flag} true, which widens the false of its supertype '{parent}'; a subtype may only narrow it (DISL §4.7)."),
                            ));
                        }
                    }
                }
                None if inherited.iter().any(|(_, value)| *value != inherited[0].1) => {
                    let given: Vec<String> = inherited.iter().map(|(parent, value)| format!("{value} from '{parent}'")).collect();
                    diagnostics.push(DislDiagnostic::error(
                        pointer.clone(),
                        format!("'{name}' inherits {flag} differently, {}, and does not redeclare it (DISL §4.7).", given.join(", ")),
                    ));
                }
                None => {}
            }
            effective(relations, name, flag, default)
        });
        values.push((directed, allow_self_loops, allow_parallel));
    }
    for (relation, (directed, allow_self_loops, allow_parallel)) in relations.values_mut().zip(values) {
        relation.directed = directed;
        relation.allow_self_loops = allow_self_loops;
        relation.allow_parallel = allow_parallel;
    }
}

/// The C3 linearisation of every type (section 4.7); a cycle or an inconsistent hierarchy is an error.
fn linearise(types: &mut IndexMap<String, DislType>, section: &str, diagnostics: &mut Vec<DislDiagnostic>) {
    let extends: HashMap<String, Vec<String>> = types.iter().map(|(n, t)| (n.clone(), t.extends.clone())).collect();
    let mut done: HashMap<String, Vec<String>> = HashMap::new();
    let mut visiting: HashSet<String> = HashSet::new();

    fn of(
        name: &str,
        extends: &HashMap<String, Vec<String>>,
        done: &mut HashMap<String, Vec<String>>,
        visiting: &mut HashSet<String>,
        section: &str,
        diagnostics: &mut Vec<DislDiagnostic>,
    ) -> Vec<String> {
        if let Some(known) = done.get(name) {
            return known.clone();
        }
        if !visiting.insert(name.to_owned()) {
            diagnostics.push(DislDiagnostic::error(
                json::pointer(section, name),
                format!("'{name}' is its own supertype; supertypes form a directed acyclic graph (DISL §4.7)."),
            ));
            return vec![name.to_owned()];
        }
        let parents = extends.get(name).cloned().unwrap_or_default();
        let mut sequences: Vec<Vec<String>> = parents
            .iter()
            .map(|p| of(p, extends, done, visiting, section, diagnostics))
            .collect();
        sequences.push(parents.clone());
        let mut result = vec![name.to_owned()];
        while sequences.iter().any(|s| !s.is_empty()) {
            let head = sequences
                .iter()
                .filter(|s| !s.is_empty())
                .map(|s| s[0].clone())
                .find(|candidate| {
                    !sequences
                        .iter()
                        .any(|s| s.iter().position(|x| x == candidate).is_some_and(|p| p > 0))
                });
            let Some(head) = head else {
                diagnostics.push(DislDiagnostic::error(
                    json::pointer(section, name),
                    format!("'{name}' has no consistent C3 linearisation of its supertypes (DISL §4.7)."),
                ));
                let mut rest: Vec<String> = Vec::new();
                for n in sequences.iter().flatten() {
                    if !rest.contains(n) {
                        rest.push(n.clone());
                    }
                }
                for n in rest {
                    if !result.contains(&n) {
                        result.push(n);
                    }
                }
                break;
            };
            for s in sequences.iter_mut() {
                if let Some(p) = s.iter().position(|x| *x == head) {
                    s.remove(p);
                }
            }
            result.push(head);
        }
        visiting.remove(name);
        done.insert(name.to_owned(), result.clone());
        result
    }

    let names: Vec<String> = types.keys().cloned().collect();
    for name in names {
        let lin = of(&name, &extends, &mut done, &mut visiting, section, diagnostics);
        if let Some(t) = types.get_mut(&name) {
            t.linearisation = lin;
        }
    }
}

fn section_of(t: &DislType) -> &'static str {
    if t.is_relation {
        "/metamodel/relations"
    } else {
        "/metamodel/types"
    }
}

/// A type's attributes, containment and label attribute, merged along its linearisation (sections 4.6 to 4.8).
fn merge(
    t: &DislType,
    all: &IndexMap<String, DislType>,
    pointer: &str,
    known: &TypeNames,
    diagnostics: &mut Vec<DislDiagnostic>,
) -> Merged {
    let mut merged: IndexMap<String, DislAttribute> = IndexMap::new();
    for ancestor in t.linearisation.iter().rev() {
        let Some(declarer) = all.get(ancestor) else {
            continue;
        };
        let own = attributes(
            &declarer.json,
            &declarer.name,
            &json::pointer(section_of(declarer), &declarer.name),
            known,
            if *ancestor == t.name {
                Some(&mut *diagnostics)
            } else {
                None
            },
        );
        for attribute in own.into_values() {
            if let Some(inherited) = merged.get(&attribute.name) {
                let narrows = all
                    .get(&attribute.declared_by)
                    .is_some_and(|d| d.linearisation.contains(&inherited.declared_by));
                if !narrows
                    && raw(&attribute.json) != raw(&inherited.json)
                    && !json::members(&t.json, "attributes").any(|(n, _)| *n == attribute.name)
                {
                    diagnostics.push(DislDiagnostic::error(
                        pointer,
                        format!(
                            "'{}' inherits '{}' from both '{}' and '{}', differently, and does not redeclare it (DISL §4.7).",
                            t.name, attribute.name, inherited.declared_by, attribute.declared_by
                        ),
                    ));
                }
                if narrows
                    && attribute.type_name != inherited.type_name
                    && !(all.contains_key(&inherited.type_name)
                        && all
                            .get(&attribute.type_name)
                            .is_some_and(|a| a.linearisation.contains(&inherited.type_name)))
                {
                    diagnostics.push(DislDiagnostic::error(
                        json::pointer(&json::pointer(pointer, "attributes"), &attribute.name),
                        format!(
                            "'{}' redeclares '{}' as {}, changing its type {}; a subtype may only narrow it (DISL §4.7).",
                            attribute.declared_by, attribute.name, attribute.type_name, inherited.type_name
                        ),
                    ));
                }
            }
            merged.insert(attribute.name.clone(), attribute);
        }
    }

    let containing = t
        .linearisation
        .iter()
        .filter_map(|n| all.get(n))
        .map(|d| &d.json)
        .find(|j| json::has(j, "children"));
    let child_types = containing
        .map(|j| json::strings_of(json::member(j, "children"), "allowed"))
        .unwrap_or_default();

    let declared_label = t
        .linearisation
        .iter()
        .filter_map(|n| all.get(n))
        .find_map(|d| json::string_of(&d.json, "labelAttribute").map(str::to_owned));
    if let Some(label) = &declared_label
        && !merged.contains_key(label)
    {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, "labelAttribute"),
            format!(
                "'{}' has no attribute '{label}' to use as its label (DISL §4.6).",
                t.name
            ),
        ));
    }
    let label = declared_label
        .or_else(|| {
            merged
                .values()
                .find(|a| json::bool_of(&a.json, "key"))
                .map(|a| a.name.clone())
        })
        .or_else(|| {
            merged
                .values()
                .find(|a| a.type_name == "string" && json::bool_of(&a.json, "required"))
                .map(|a| a.name.clone())
        })
        .or_else(|| {
            merged
                .values()
                .find(|a| a.type_name == "string")
                .map(|a| a.name.clone())
        });
    (merged, child_types, label)
}

/// The attributes an object declares itself, in order; each name checked against the reserved names and each type
/// resolved when diagnostics are asked for.
fn attributes(
    owner: &Json,
    declared_by: &str,
    pointer: &str,
    known: &TypeNames,
    mut diagnostics: Option<&mut Vec<DislDiagnostic>>,
) -> IndexMap<String, DislAttribute> {
    let mut result = IndexMap::new();
    for (name, value) in json::members(owner, "attributes") {
        let at = json::pointer(&json::pointer(pointer, "attributes"), name);
        let type_name = json::string_of(value, "type");
        if let Some(diagnostics) = diagnostics.as_deref_mut() {
            if is_reserved(name) {
                diagnostics.push(DislDiagnostic::error(
                    at.clone(),
                    format!("'{name}' is a reserved name and cannot be an attribute (DISL §2.2)."),
                ));
            }
            match type_name {
                None => diagnostics.push(DislDiagnostic::error(
                    at.clone(),
                    format!("The attribute '{name}' has no type (DISL §4.3)."),
                )),
                Some(t) if !known.contains(t) => diagnostics.push(DislDiagnostic::error(
                    json::pointer(&at, "type"),
                    format!(
                        "'{t}' is not a primitive, an enum, a data type or a type of this specification (DISL §4.3)."
                    ),
                )),
                _ => {}
            }
        }
        let literal = json::get(value, "default").filter(|d| !(d.is_object() && json::has(d, "cel")));
        result.insert(
            name.clone(),
            DislAttribute {
                name: name.clone(),
                type_name: type_name.unwrap_or("json").to_owned(),
                many: json::bool_of(value, "many"),
                default: literal.cloned(),
                declared_by: declared_by.to_owned(),
                json: value.clone(),
            },
        );
    }
    result
}

/// A relation end (section 4.9): a TypeRef, a TypeRef[] or a RelationEnd, its types resolved.
fn end(
    relation: &DislType,
    side: &str,
    all: &IndexMap<String, DislType>,
    pointer: &str,
    diagnostics: &mut Vec<DislDiagnostic>,
) -> DislRelationEnd {
    // An end is inherited from the nearest supertype that declares it.
    let declaring = relation
        .linearisation
        .iter()
        .filter_map(|n| all.get(n))
        .map(|d| &d.json)
        .find(|j| json::has(j, side));
    let Some(declaring) = declaring else {
        if !relation.is_abstract() && relation.derived().is_none() {
            diagnostics.push(DislDiagnostic::error(
                pointer,
                format!("The relation type '{}' has no {side} end (DISL §4.9).", relation.name),
            ));
        }
        return DislRelationEnd::default();
    };
    let end = json::get(declaring, side).unwrap_or(&NOTHING);
    let (types, exclude) = if end.is_object() {
        (json::strings_of(end, "types"), json::strings_of(end, "exclude"))
    } else {
        (json::strings(end), Vec::new())
    };
    for name in types.iter().chain(exclude.iter()).filter(|n| !all.contains_key(*n)) {
        diagnostics.push(DislDiagnostic::error(
            json::pointer(pointer, side),
            format!("'{name}' is not a type of this specification (DISL §4.9)."),
        ));
    }
    DislRelationEnd {
        types,
        exclude,
        optional: side == "target" && json::bool_of(end, "optional"),
    }
}

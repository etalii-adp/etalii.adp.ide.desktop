//! What can be done to a selected element of the family (etalii.adp spec 030, task T083), offered as data and reached
//! through the one path the context service defines.
//!
//! Ported from the standalone host's `RdfContextActionProvider.cs` and `RdfContextSourceResolver.cs` at `8b6ccfc`. One
//! provider for the whole family, whichever reading a file is opened under: the shapes reading's and the scheme
//! reading's entries lead where they apply, the family's own beneath them. Every edit is one command on the project's
//! history. Term dialogs accept a full IRI or a prefixed name under the document's own declarations, and an undeclared
//! prefix is refused by name. Under truncation every edit is withheld with the stated sentence; blank-node-rooted
//! selections describe but never edit.

use adp_context::{Action, ActionGroup, Shortcut};

use crate::commands::{ObjectValue, RdfEdit};
use crate::family::{self, Run, Target};
use crate::model::RdfModel;
use crate::store::RdfDocument;
use crate::vocabulary::{owl, rdf};
use crate::{iri, owl as owl_reading, selection, shacl, skos, term_input, writer};

/// Rename a resource, rewriting every reference with it.
pub const RENAME_RESOURCE: &str = "rdf.rename-resource";
/// Remove a resource and every triple touching it, saying how many first.
pub const REMOVE_RESOURCE: &str = "rdf.remove-resource";
/// Remove a selected edge's triple.
pub const REMOVE_EDGE: &str = "rdf.remove-edge";
/// The relation gesture: relate two drawn resources, asking for the predicate.
pub const CONNECT: &str = "rdf.connect";
/// Add a resource at a placement, asking for its name.
pub const ADD_RESOURCE: &str = "rdf.add-resource";
/// Declare a prefix, asking for `prefix: iri` in one line.
pub const ADD_PREFIX: &str = "rdf.add-prefix";
/// The subclass gesture: one `rdfs:subClassOf` splice, a duplicate refused first.
pub const SUBCLASS: &str = "owl.subclass";
/// Add a class at a placement: one `a owl:Class` declaration.
pub const ADD_CLASS: &str = "owl.add-class";
/// Add an object property at a placement.
pub const ADD_OBJECT_PROPERTY: &str = "owl.add-object-property";
/// Add a datatype property at a placement.
pub const ADD_DATATYPE_PROPERTY: &str = "owl.add-datatype-property";
/// Add a named individual at a placement.
pub const ADD_INDIVIDUAL: &str = "owl.add-individual";

/// What each ontology add action declares its new term as.
fn owl_declaration_type(action_id: &str) -> Option<&'static str> {
    match action_id {
        ADD_CLASS => Some(owl::CLASS),
        ADD_OBJECT_PROPERTY => Some(owl::OBJECT_PROPERTY),
        ADD_DATATYPE_PROPERTY => Some(owl::DATATYPE_PROPERTY),
        ADD_INDIVIDUAL => Some(owl::NAMED_INDIVIDUAL),
        _ => None,
    }
}

/// The text an element id resolves to: the family's, else the ontology's, else the shapes reading's
/// (`RdfContextSourceResolver.Describe`).
pub fn describe(model: &RdfModel, element_id: &str) -> Option<String> {
    selection::describe(model, element_id)
        .or_else(|| owl_reading::selection::describe(model, element_id))
        .or_else(|| shacl::describe(model, element_id))
}

/// The named resource or individual an id names (`RdfSelection.ResourceOf ?? OwlSelection.IndividualIriOf`).
fn named(model: &RdfModel, element_id: &str) -> Option<String> {
    selection::resource_of(model, element_id).or_else(|| owl_reading::selection::individual_iri_of(model, element_id))
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

/// The groups the shapes and the scheme readings lead a menu with.
fn readings(document: &RdfDocument, target: &Target) -> Vec<ActionGroup> {
    let mut groups = shacl::actions::discover(document, target);
    groups.extend(skos::actions::discover(document, target));
    groups
}

/// The menu of `target` (`DiscoverAsync`).
pub fn discover(document: &RdfDocument, target: &Target) -> Vec<ActionGroup> {
    let model = &document.model;
    if !document.is_usable() || selection::is_truncated(model) {
        // A broken file withholds every edit; so does a truncated view - executing anyway answers with the sentence.
        return Vec::new();
    }
    let element_id = target.element_id;
    if let Some(iri) = named(model, element_id) {
        let touching = selection::touching(model, &iri).len();
        let remove = if touching == 1 {
            "Remove".to_owned()
        } else {
            format!("Remove (with {touching} statements)")
        };
        return vec![ActionGroup {
            actions: vec![
                action(RENAME_RESOURCE, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
                action(REMOVE_RESOURCE, &remove, "mdi-delete-outline").with_shortcut(Shortcut::key("Delete")),
            ],
        }];
    }
    if skos::selection::pair_of(model, element_id).is_some() {
        return readings(document, target);
    }
    if selection::edge_of(model, element_id).is_some() {
        let mut groups = readings(document, target);
        groups.push(ActionGroup {
            actions: vec![
                action(REMOVE_EDGE, "Remove statement", "mdi-vector-polyline-remove")
                    .with_shortcut(Shortcut::key("Delete")),
            ],
        });
        return groups;
    }
    if selection::is_placement(element_id) {
        let mut actions = vec![
            action(ADD_RESOURCE, "Add resource here…", "mdi-card-plus-outline"),
            action(ADD_PREFIX, "Declare prefix…", "mdi-at"),
        ];
        if owl_reading::selection::is_ontology_document(model) {
            actions.push(action(ADD_CLASS, "Add class here…", "mdi-shape-circle-plus"));
            actions.push(action(
                ADD_OBJECT_PROPERTY,
                "Add object property…",
                "mdi-ray-start-arrow",
            ));
            actions.push(action(
                ADD_DATATYPE_PROPERTY,
                "Add datatype property…",
                "mdi-form-textbox",
            ));
            actions.push(action(ADD_INDIVIDUAL, "Add individual…", "mdi-account-outline"));
        }
        let mut groups = readings(document, target);
        groups.push(ActionGroup { actions });
        return groups;
    }
    if let Some((from, to)) = selection::relation_of(element_id) {
        let mut actions = Vec::new();
        let from_iri = selection::resource_of(model, &from);
        let to_iri = selection::resource_of(model, &to);
        if let (Some(from_iri), Some(to_iri)) = (&from_iri, &to_iri)
            && owl_reading::selection::is_class(model, from_iri)
            && owl_reading::selection::is_class(model, to_iri)
        {
            actions.push(action(SUBCLASS, "Subclass of", "mdi-file-tree"));
        }
        actions.push(action(CONNECT, "Relate…", "mdi-ray-start-arrow"));
        let mut groups = readings(document, target);
        groups.push(ActionGroup { actions });
        return groups;
    }
    readings(document, target)
}

/// Runs `action_id` on `target` (`ExecuteAsync`).
pub fn execute(document: &RdfDocument, target: &Target, action_id: &str) -> Run {
    let model = &document.model;
    if selection::is_truncated(model) {
        return Run::Refused(selection::TRUNCATED_REFUSAL.to_owned());
    }
    if let Some(run) = shacl::actions::execute(document, target, action_id) {
        return run;
    }
    if let Some(run) = skos::actions::execute(document, target, action_id) {
        return run;
    }
    let element_id = target.element_id;
    let iri = named(model, element_id);
    match action_id {
        SUBCLASS => {
            let Some((from, to)) = selection::relation_of(element_id) else {
                return Run::Done;
            };
            let (Some(from_iri), Some(to_iri)) =
                (selection::resource_of(model, &from), selection::resource_of(model, &to))
            else {
                return Run::Refused("A subclass relation needs two named classes.".to_owned());
            };
            let already = model.triples.iter().any(|t| {
                t.subject.is_iri(&from_iri) && t.predicate.iri == owl::SUB_CLASS_OF && t.object.is_iri(&to_iri)
            });
            if already {
                return Run::Refused(owl_reading::selection::DUPLICATE_SUBCLASS_REFUSAL.to_owned());
            }
            Run::Edit(RdfEdit::AddTriple {
                subject: from_iri,
                predicate: owl::SUB_CLASS_OF.to_owned(),
                object: ObjectValue::iri(to_iri),
            })
        }
        ADD_CLASS | ADD_OBJECT_PROPERTY | ADD_DATATYPE_PROPERTY | ADD_INDIVIDUAL => {
            let title = format!("Add {}", action_id["owl.add-".len()..].replace('-', " "));
            Run::Ask(family::input(
                &title,
                "mdi-shape-circle-plus",
                "IRI or prefixed name",
                "",
                "Add",
            ))
        }
        RENAME_RESOURCE if iri.is_some() => Run::Ask(family::input(
            "Rename resource",
            "mdi-pencil-outline",
            "New IRI or prefixed name",
            &writer::compress(model, iri.as_deref().unwrap_or_default()),
            "Rename",
        )),
        REMOVE_RESOURCE if iri.is_some() => {
            let iri = iri.unwrap_or_default();
            let touching = selection::touching(model, &iri).len();
            if touching <= 1 {
                return Run::Edit(RdfEdit::RemoveResource { iri });
            }
            Run::Ask(family::confirm(
                "Remove resource",
                "mdi-delete-outline",
                &format!("Removing this resource also removes the {touching} statements it appears in."),
                "Remove",
                true,
            ))
        }
        REMOVE_EDGE => match selection::edge_of(model, element_id) {
            Some(edge) => Run::Edit(RdfEdit::RemoveTriple {
                subject: edge.subject.as_iri().unwrap_or_default().to_owned(),
                predicate: edge.predicate.iri.clone(),
                object: ObjectValue::iri(edge.object.as_iri().unwrap_or_default()),
            }),
            None => Run::Done,
        },
        CONNECT => {
            let Some((from, to)) = selection::relation_of(element_id) else {
                return Run::Done;
            };
            if selection::is_blank(&from) || selection::is_blank(&to) {
                return Run::Refused("A blank node's identity does not survive a reparse, so relations to it cannot land in the file. Name it with an IRI first.".to_owned());
            }
            if selection::is_placement(&from) || selection::is_placement(&to) {
                return Run::Refused("Drop the relation on a resource; a statement needs both of its ends.".to_owned());
            }
            Run::Ask(family::input(
                "Relate",
                "mdi-ray-start-arrow",
                "Predicate (IRI or prefixed name)",
                "",
                "Relate",
            ))
        }
        ADD_RESOURCE => Run::Ask(family::input(
            "Add resource",
            "mdi-card-plus-outline",
            "IRI or prefixed name",
            "",
            "Add",
        )),
        ADD_PREFIX => Run::Ask(family::input(
            "Declare prefix",
            "mdi-at",
            "Declaration, e.g. ex: http://example.org/",
            "",
            "Declare",
        )),
        _ => Run::Done,
    }
}

/// Whether `value` may answer the prompt of `action_id`: the writers refuse on exactly these terms at commit, so a
/// dialog can refuse first (`ValidateAsync`).
pub fn validate(document: &RdfDocument, _target: &Target, action_id: &str, value: &str) -> Result<(), String> {
    if let Some(verdict) = shacl::actions::validate(document, action_id, value) {
        return verdict;
    }
    if let Some(verdict) = skos::actions::validate(document, action_id, value) {
        return verdict;
    }
    let model = &document.model;
    if matches!(action_id, RENAME_RESOURCE | CONNECT | ADD_RESOURCE) || owl_declaration_type(action_id).is_some() {
        let resolved = term_input::resolve(model, value)?;
        if action_id == RENAME_RESOURCE && model.names(&resolved) {
            return Err(format!(
                "{resolved} already names something in this document; renaming onto it would silently merge two resources."
            ));
        }
    }
    if action_id == ADD_PREFIX && prefix_declaration(value).is_none() {
        return Err("Write the declaration as 'prefix: iri', e.g. ex: http://example.org/.".to_owned());
    }
    Ok(())
}

/// The edit an answer `value` to the prompt of `action_id` makes, or why none (`CommitAsync`).
pub fn commit(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Result<RdfEdit, String> {
    if selection::is_truncated(&document.model) {
        return Err(selection::TRUNCATED_REFUSAL.to_owned());
    }
    command_for(document, target, action_id, value.trim())
        .ok_or_else(|| format!("'{action_id}' does not apply to this selection."))
}

/// The edit an answer makes (`CommandFor`).
fn command_for(document: &RdfDocument, target: &Target, action_id: &str, value: &str) -> Option<RdfEdit> {
    if let Some(edit) = shacl::actions::command_for(document, target, action_id, value) {
        return Some(edit);
    }
    if let Some(edit) = skos::actions::command_for(document, target, action_id, value) {
        return Some(edit);
    }
    let model = &document.model;
    let element_id = target.element_id;
    let iri = named(model, element_id);
    if let Some(declaration) = owl_declaration_type(action_id) {
        let declared = term_input::resolve(model, value).ok()?;
        return Some(RdfEdit::AddTriple {
            subject: declared,
            predicate: rdf::TYPE.to_owned(),
            object: ObjectValue::iri(declaration),
        });
    }
    match action_id {
        RENAME_RESOURCE => {
            let resolved = term_input::resolve(model, value).ok()?;
            Some(RdfEdit::RenameTerm {
                old_iri: iri?,
                new_iri: resolved,
            })
        }
        REMOVE_RESOURCE => Some(RdfEdit::RemoveResource { iri: iri? }),
        CONNECT => {
            let (from, to) = selection::relation_of(element_id)?;
            let from_iri = selection::resource_of(model, &from);
            let to_iri = selection::resource_of(model, &to);
            let predicate = term_input::resolve(model, value).ok();
            Some(RdfEdit::AddTriple {
                subject: from_iri?,
                predicate: predicate?,
                object: ObjectValue::iri(to_iri?),
            })
        }
        ADD_RESOURCE => {
            let resolved = term_input::resolve(model, value).ok()?;
            Some(RdfEdit::AddTriple {
                subject: resolved,
                predicate: rdf::TYPE.to_owned(),
                object: ObjectValue::iri(rdf::RESOURCE),
            })
        }
        ADD_PREFIX => {
            let (prefix, iri) = prefix_declaration(value)?;
            Some(RdfEdit::AddPrefix { prefix, iri })
        }
        _ => None,
    }
}

/// One line, two facts: `prefix: iri`, brackets tolerated (`ParsePrefixDeclaration`).
fn prefix_declaration(value: &str) -> Option<(String, String)> {
    let colon = value.find(':').filter(|&c| c > 0)?;
    let prefix = value[..colon].trim();
    let iri = value[colon + 1..]
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    (!prefix.is_empty() && !iri.is_empty() && !prefix.contains(' ') && !iri.contains(' ') && iri::is_absolute(iri))
        .then(|| (prefix.to_owned(), iri.to_owned()))
}

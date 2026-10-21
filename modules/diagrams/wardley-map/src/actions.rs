//! What a person can do to a Wardley map and its elements, and the command each answer becomes (etalii.adp spec 030,
//! task T070).
//!
//! Ported from the standalone host's `WardleyActions.cs`, `WardleyContextActionProvider.cs` and `WardleyEditability.cs`
//! at `8b6ccfc`, over its `DiagramAction` and `DiagramActionProvider`. Every action is one record: whether it applies
//! to a subject, how it is offered there (an action that would fail is not offered: a "Stop evolving" only on
//! something that evolves, an unlink only where there is a link, no pipeline child on a legacy pipeline), what it asks
//! first (a value, a choice, a refusal), what it confirms, and the command it makes from the answer. The menu, the
//! commit of an answer and the property grid's rows for the same edits all make their command here. Nothing is offered
//! on a map whose file is read-only.

use std::path::Path;

use adp_context::{Action, ActionGroup, Prompt, PromptOption, Shortcut};
use adp_diagram::DiagramDocuments;
use adp_history::Command;

use crate::commands::{
    AddWardleyAnnotation, AddWardleyElement, AddWardleyNote, RemoveWardleyElement, RenameWardleyElement,
    SetWardleyDecorator, SetWardleyEvolve, SetWardleyInertia, SetWardleyLink, SetWardleyPipelineMembership,
};
use crate::identities::{SEPARATOR, WardleyIdentityEntry, keys, kind};
use crate::lines::{format_number, parse_number};
use crate::model::{
    WardleyComponent, WardleyDecorator, WardleyEvolve, WardleyLinkKind, WardleyMap, WardleyPipeline,
    WardleyPipelineChild, WardleyPipelineForm,
};

pub const ADD_COMPONENT: &str = "wardley.add-component";
pub const ADD_ANCHOR: &str = "wardley.add-anchor";
pub const ADD_SUBMAP: &str = "wardley.add-submap";
pub const ADD_MARKET: &str = "wardley.add-market";
pub const ADD_ECOSYSTEM: &str = "wardley.add-ecosystem";
pub const ADD_NOTE: &str = "wardley.add-note";
pub const ADD_ANNOTATION: &str = "wardley.add-annotation";
pub const RENAME: &str = "wardley.rename";
pub const REMOVE: &str = "wardley.remove";
pub const SET_EVOLVE: &str = "wardley.set-evolve";
pub const CLEAR_EVOLVE: &str = "wardley.clear-evolve";
pub const TOGGLE_INERTIA: &str = "wardley.toggle-inertia";
pub const LINK: &str = "wardley.link";
pub const FLOW: &str = "wardley.flow";
pub const UNLINK: &str = "wardley.unlink";
pub const ADD_TO_PIPELINE: &str = "wardley.add-to-pipeline";
pub const REMOVE_FROM_PIPELINE: &str = "wardley.remove-from-pipeline";

/// The toggle of one of the five decorators: `wardley.toggle-buy` and its kin.
pub fn decorator_action_id(decorator: WardleyDecorator) -> String {
    format!("wardley.toggle-{}", decorator.spell())
}

fn decorator_of_action(action_id: &str) -> Option<WardleyDecorator> {
    WardleyDecorator::ALL
        .into_iter()
        .find(|d| decorator_action_id(*d) == action_id)
}

/// What an action on an element the map no longer has answers.
pub const GONE: &str = crate::commands::GONE;
const NO_LINK: &str = "This element has no link to remove.";

/// The reason an edit of a read-only map is refused (`WardleyEditability.Reason`).
pub const READ_ONLY: &str = "This map's file is read-only.";

/// Whether the map at `body_path` can be written: a file that does not exist yet can, and one whose attributes cannot
/// be read is offered its edits and the save decides.
pub fn editable(body_path: &Path) -> bool {
    match std::fs::metadata(body_path) {
        Ok(metadata) => !metadata.permissions().readonly(),
        Err(_) => true,
    }
}

/// What an action is run on: the map, the identities held beside it, and the element the selection names.
#[derive(Debug, Clone, Copy)]
pub struct WardleySubject<'a> {
    pub body_path: &'a Path,
    pub map: &'a WardleyMap,
    pub identities: &'a [WardleyIdentityEntry],
    /// The id the selection names; empty for the map itself.
    pub element_id: &'a str,
}

impl<'a> WardleySubject<'a> {
    /// The identity the selection names, or none for the map itself or an element that is gone.
    pub fn entry(&self) -> Option<&'a WardleyIdentityEntry> {
        self.identities.iter().find(|e| e.id == self.element_id)
    }

    /// Whether the selection is the map itself.
    pub fn is_map(&self) -> bool {
        self.element_id.is_empty()
    }

    /// Whether the selection names an element the map no longer has.
    pub fn is_gone(&self) -> bool {
        !self.is_map() && self.entry().is_none()
    }

    /// The component the selection names, or none when it names something else.
    pub fn component(&self) -> Option<&'a WardleyComponent> {
        let entry = self.entry().filter(|e| e.kind == kind::COMPONENT)?;
        self.map.components.iter().find(|c| keys::component(c) == entry.key)
    }

    /// The pipeline child the selection names, with its pipeline.
    pub fn child(&self) -> Option<(&'a WardleyPipeline, &'a WardleyPipelineChild)> {
        let entry = self.entry().filter(|e| e.kind == kind::PIPELINE_CHILD)?;
        self.map.pipelines.iter().find_map(|pipeline| {
            pipeline
                .children
                .iter()
                .find(|child| keys::pipeline_child(pipeline, child) == entry.key)
                .map(|child| (pipeline, child))
        })
    }

    /// The `evolve` statement of the selected component.
    pub fn evolve(&self) -> Option<&'a WardleyEvolve> {
        let component = self.component()?;
        self.map.evolves.iter().find(|e| e.name == component.name)
    }

    /// The id of the component called `name`, or empty.
    pub fn id_of(&self, name: &str) -> String {
        crate::identities::id_of(self.identities, kind::COMPONENT, name)
    }
}

/// One link, as a dialog offers it.
#[derive(Debug, Clone, PartialEq)]
pub struct WardleyLinkChoice {
    pub source: String,
    pub target: String,
    pub kind: WardleyLinkKind,
    pub label: String,
    /// The link's composite key, the handle a chosen answer comes back as.
    pub id: String,
}

/// The links the selection is, or is an end of, each labelled as a person reads it.
pub fn links_of(subject: &WardleySubject) -> Vec<WardleyLinkChoice> {
    let name = subject.component().map(|c| c.name.as_str());
    let key = subject.entry().map(|e| e.key.as_str());
    subject
        .map
        .links
        .iter()
        .filter(|link| match name {
            Some(name) => link.source == name || link.target == name,
            None => Some(keys::link(link).as_str()) == key,
        })
        .map(|link| WardleyLinkChoice {
            source: link.source.clone(),
            target: link.target.clone(),
            kind: link.kind,
            label: format!("{} {} {}", link.source, link.kind.arrow(), link.target),
            id: format!(
                "{}{SEPARATOR}{}{SEPARATOR}{}",
                link.source,
                link.target,
                link.kind.name()
            ),
        })
        .collect()
}

fn pipeline_of<'a>(subject: &WardleySubject<'a>, component: &WardleyComponent) -> Option<&'a WardleyPipeline> {
    subject.map.pipelines.iter().find(|p| p.parent == component.name)
}

/// Every other element on the map, as the options of a "link to" dialog, by name.
fn others(subject: &WardleySubject) -> Vec<PromptOption> {
    let Some(component) = subject.component() else {
        return Vec::new();
    };
    let mut options: Vec<PromptOption> = subject
        .map
        .components
        .iter()
        .filter(|c| c.name != component.name)
        .map(|c| option(subject.id_of(&c.name), c.name.clone()))
        .filter(|o| !o.id.is_empty())
        .collect();
    options.sort_by_key(|o| o.label.to_uppercase());
    options
}

fn option(id: String, label: String) -> PromptOption {
    PromptOption {
        id,
        label,
        selectable: true,
        children: Vec::new(),
        suggested_value: String::new(),
        icon: String::new(),
        description: String::new(),
        name_suppressed_reason: String::new(),
        unavailable_reason: String::new(),
    }
}

fn action(id: &str, label: &str, icon: &str) -> Action {
    Action::new(id, label).with_icon(icon)
}

fn input(title: &str, icon: &str, label: &str, initial: &str, confirm: &str) -> Prompt {
    Prompt::Input {
        title: title.to_owned(),
        icon: icon.to_owned(),
        label: label.to_owned(),
        initial_value: initial.to_owned(),
        confirm_label: confirm.to_owned(),
    }
}

/// The add actions: their id, label, icon and how the dialog names what is added.
const ADDS: [(&str, &str, &str, &str); 7] = [
    (ADD_COMPONENT, "Add component…", "mdi-plus", "component"),
    (ADD_ANCHOR, "Add anchor…", "mdi-anchor", "anchor"),
    (ADD_SUBMAP, "Add submap…", "mdi-map-outline", "submap"),
    (ADD_MARKET, "Add market…", "mdi-store-outline", "market"),
    (ADD_ECOSYSTEM, "Add ecosystem…", "mdi-graph-outline", "ecosystem"),
    (ADD_NOTE, "Add note…", "mdi-note-outline", "note"),
    (
        ADD_ANNOTATION,
        "Add annotation…",
        "mdi-comment-text-outline",
        "annotation",
    ),
];

fn is_add(action_id: &str) -> bool {
    ADDS.iter().any(|(id, ..)| *id == action_id)
}

/// Whether a note or an annotation is added: they carry text rather than a name.
fn adds_text(action_id: &str) -> bool {
    action_id == ADD_NOTE || action_id == ADD_ANNOTATION
}

/// The context menu of `subject`, in its groups: the adds on the map itself; on a component, its identity, its
/// strategy, its decorators, its links and its pipeline; on a link, its removal; on a pipeline child, its removal from
/// the pipeline. Nothing on a map that could not be read or cannot be written.
pub fn offered(subject: &WardleySubject, usable: bool) -> Vec<ActionGroup> {
    if !usable || !editable(subject.body_path) {
        return Vec::new();
    }
    let mut groups: Vec<ActionGroup> = Vec::new();
    let mut push = |actions: Vec<Action>| {
        if !actions.is_empty() {
            groups.push(ActionGroup { actions });
        }
    };
    if subject.is_map() {
        push(
            ADDS.iter()
                .map(|(id, label, icon, _)| action(id, label, icon))
                .collect(),
        );
    }
    let component = subject.component();
    if let Some(component) = component {
        push(vec![
            action(RENAME, "Rename…", "mdi-pencil-outline").with_shortcut(Shortcut::key("F2")),
            action(REMOVE, "Remove", "mdi-trash-can-outline").with_shortcut(Shortcut::key("Delete")),
        ]);
        let evolve = subject.evolve();
        let mut strategy = vec![action(
            SET_EVOLVE,
            if evolve.is_none() {
                "Evolve to…"
            } else {
                "Change evolution target…"
            },
            "mdi-arrow-right-bold-outline",
        )];
        if evolve.is_some() {
            strategy.push(action(CLEAR_EVOLVE, "Stop evolving", "mdi-arrow-right-bold-outline"));
        }
        strategy.push(action(
            TOGGLE_INERTIA,
            if component.inertia {
                "Clear inertia"
            } else {
                "Mark as having inertia"
            },
            "mdi-weight",
        ));
        push(strategy);
        push(
            WardleyDecorator::ALL
                .into_iter()
                .map(|decorator| {
                    let label = if component.decorators.contains(&decorator) {
                        format!("Clear {}", decorator.spell())
                    } else {
                        format!("Mark as {}", decorator.spell())
                    };
                    action(&decorator_action_id(decorator), &label, "mdi-tag-outline")
                })
                .collect(),
        );
    }
    let mut links = Vec::new();
    if component.is_some() {
        links.push(action(LINK, "Link to…", "mdi-arrow-right"));
        links.push(action(FLOW, "Flow to…", "mdi-transfer-right"));
    }
    match subject.entry().map(|e| e.kind.as_str()) {
        Some(kind::COMPONENT) if !links_of(subject).is_empty() => {
            links.push(action(UNLINK, "Remove link…", "mdi-link-variant-off"));
        }
        Some(kind::LINK) if !links_of(subject).is_empty() => {
            links.push(action(UNLINK, "Remove link", "mdi-link-variant-off").with_shortcut(Shortcut::key("Delete")));
        }
        _ => {}
    }
    push(links);
    let mut pipeline = Vec::new();
    if let Some(component) = component {
        let existing = pipeline_of(subject, component);
        if existing.is_none_or(|p| p.form != WardleyPipelineForm::Legacy) {
            pipeline.push(action(
                ADD_TO_PIPELINE,
                if existing.is_none() {
                    "Start a pipeline…"
                } else {
                    "Add to pipeline…"
                },
                "mdi-pipe",
            ));
        }
    }
    if subject.child().is_some() {
        pipeline.push(
            action(REMOVE_FROM_PIPELINE, "Remove from pipeline", "mdi-trash-can-outline")
                .with_shortcut(Shortcut::key("Delete")),
        );
    }
    push(pipeline);
    groups
}

/// Whether action `action_id` applies to `subject` (`DiagramAction.AppliesTo`).
pub(crate) fn applies(subject: &WardleySubject, action_id: &str) -> bool {
    match action_id {
        _ if is_add(action_id) => true,
        RENAME | REMOVE | SET_EVOLVE | CLEAR_EVOLVE | TOGGLE_INERTIA | LINK | FLOW | ADD_TO_PIPELINE => {
            subject.component().is_some()
        }
        _ if decorator_of_action(action_id).is_some() => subject.component().is_some(),
        UNLINK => subject.entry().is_some(),
        REMOVE_FROM_PIPELINE => subject.entry().is_some_and(|e| e.kind == kind::PIPELINE_CHILD),
        _ => false,
    }
}

/// What running an action comes to before anything is written.
pub enum Run {
    /// It asks first.
    Prompt(Prompt),
    /// It makes this command at once.
    Command(Box<dyn Command<DiagramDocuments>>),
    /// It cannot, and why.
    Refused(String),
}

/// What the action asks before it runs, if anything (`DiagramAction.Ask`).
fn ask(subject: &WardleySubject, action_id: &str) -> Option<Result<Prompt, String>> {
    if let Some((_, _, _, spelled)) = ADDS.iter().find(|(id, ..)| *id == action_id) {
        return Some(Ok(input(
            &format!("Add {spelled}"),
            "mdi-plus",
            if adds_text(action_id) { "Text" } else { "Name" },
            "",
            "Add",
        )));
    }
    match action_id {
        RENAME => Some(Ok(input(
            "Rename element",
            "mdi-pencil-outline",
            "Name",
            &subject.component()?.name,
            "Rename",
        ))),
        SET_EVOLVE => {
            let start = subject.evolve().map_or_else(
                || subject.component().map(|c| c.position.maturity),
                |e| Some(e.maturity),
            )?;
            Some(Ok(input(
                "Evolve element",
                "mdi-arrow-right-bold-outline",
                "Target maturity (0 to 1)",
                &format_number(start),
                "Save",
            )))
        }
        LINK | FLOW => Some(Ok(Prompt::Choice {
            title: if action_id == LINK { "Link to" } else { "Flow to" }.to_owned(),
            icon: "mdi-arrow-right".to_owned(),
            confirm_label: if action_id == LINK { "Link" } else { "Flow" }.to_owned(),
            options: others(subject),
            empty_message: "This map has no other element to link to.".to_owned(),
            name_field: None,
        })),
        UNLINK => {
            let links = links_of(subject);
            match links.len() {
                0 => Some(Err(NO_LINK.to_owned())),
                // One link is not a choice.
                1 => None,
                _ => Some(Ok(Prompt::Choice {
                    title: "Remove link".to_owned(),
                    icon: "mdi-link-variant-off".to_owned(),
                    confirm_label: "Remove".to_owned(),
                    options: links.into_iter().map(|l| option(l.id, l.label)).collect(),
                    empty_message: NO_LINK.to_owned(),
                    name_field: None,
                })),
            }
        }
        ADD_TO_PIPELINE => Some(Ok(input(
            &format!("Add to {}'s pipeline", subject.component()?.name),
            "mdi-pipe",
            "Name",
            "",
            "Add",
        ))),
        _ => None,
    }
}

/// Runs action `action_id` on `subject` (`RunActionAsync`): what it asks first, the confirmation of a removal, or the
/// command it makes at once.
pub fn run(subject: &WardleySubject, usable: bool, action_id: &str) -> Run {
    if !usable {
        return Run::Refused(format!(
            "{} could not be read, so it cannot be edited.",
            subject
                .body_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        ));
    }
    if !applies(subject, action_id) {
        return Run::Refused(if subject.is_gone() {
            GONE.to_owned()
        } else {
            format!("'{action_id}' does not apply to this selection.")
        });
    }
    if let Some(asked) = ask(subject, action_id) {
        return match asked {
            Ok(prompt) => Run::Prompt(prompt),
            Err(refusal) => Run::Refused(refusal),
        };
    }
    if action_id == REMOVE
        && let Some(component) = subject.component()
    {
        // Confirmed, because removing an element takes its links and its `evolve` with it.
        return Run::Prompt(Prompt::Confirm {
            title: "Remove element".to_owned(),
            icon: "mdi-delete-outline".to_owned(),
            message: format!(
                "Remove '{}', and every link and evolution that names it?",
                component.name
            ),
            confirm_label: "Remove".to_owned(),
            danger: true,
        });
    }
    match command(subject, action_id, "") {
        Some(command) => Run::Command(command),
        None => Run::Refused(format!("'{action_id}' does not apply to this selection.")),
    }
}

/// Judges a typed value (`DiagramAction.Validate`): a name or a note's text may not be blank, an evolution target is
/// a number between 0 and 1.
pub fn validate(action_id: &str, value: &str) -> Result<(), String> {
    let blank = value.trim().is_empty();
    match action_id {
        ADD_NOTE | ADD_ANNOTATION if blank => Err("A note needs something to say.".to_owned()),
        _ if is_add(action_id) && blank => Err("Every element needs a name.".to_owned()),
        RENAME | ADD_TO_PIPELINE if blank => Err("Every element needs a name.".to_owned()),
        SET_EVOLVE if maturity(value).is_none() => Err("An evolution target is a number between 0 and 1.".to_owned()),
        _ => Ok(()),
    }
}

/// A typed maturity, or none when it is not a number between 0 and 1.
fn maturity(value: &str) -> Option<f64> {
    parse_number(value).filter(|m| (0.0..=1.0).contains(m))
}

/// A toggle's new state: the opposite of `current` from the menu, the given one from the grid.
fn toggled(value: &str, current: bool) -> bool {
    if value.is_empty() {
        !current
    } else {
        value.eq_ignore_ascii_case("true")
    }
}

/// The command action `action_id` makes on `subject` from `value` (`DiagramAction.Command`), or none when it does not
/// apply or the subject no longer holds what it works on.
pub fn command(subject: &WardleySubject, action_id: &str, value: &str) -> Option<Box<dyn Command<DiagramDocuments>>> {
    if !applies(subject, action_id) {
        return None;
    }
    let body_path = subject.body_path.to_path_buf();
    let element_id = subject.element_id.to_owned();
    let trimmed = value.trim().to_owned();
    let add = |kind: &str, decorator: Option<WardleyDecorator>| -> Box<dyn Command<DiagramDocuments>> {
        Box::new(AddWardleyElement {
            body_path: body_path.clone(),
            kind: kind.to_owned(),
            name: trimmed.clone(),
            visibility: 0.5,
            maturity: 0.5,
            decorator,
        })
    };
    let command: Box<dyn Command<DiagramDocuments>> = match action_id {
        ADD_COMPONENT => add("component", None),
        ADD_ANCHOR => add("anchor", None),
        ADD_SUBMAP => add("submap", None),
        ADD_MARKET => add("component", Some(WardleyDecorator::Market)),
        ADD_ECOSYSTEM => add("component", Some(WardleyDecorator::Ecosystem)),
        ADD_NOTE => Box::new(AddWardleyNote {
            body_path,
            text: trimmed,
            visibility: 0.5,
            maturity: 0.5,
        }),
        ADD_ANNOTATION => Box::new(AddWardleyAnnotation {
            body_path,
            text: trimmed,
            visibility: 0.5,
            maturity: 0.5,
        }),
        RENAME => Box::new(RenameWardleyElement {
            body_path,
            element_id,
            new_name: trimmed,
        }),
        REMOVE => Box::new(RemoveWardleyElement { body_path, element_id }),
        SET_EVOLVE => Box::new(SetWardleyEvolve {
            body_path,
            element_id,
            present: true,
            maturity: maturity(value)?,
            override_name: subject.evolve().map(|e| e.override_name.clone()).unwrap_or_default(),
        }),
        CLEAR_EVOLVE => Box::new(SetWardleyEvolve {
            body_path,
            element_id,
            present: false,
            maturity: 0.0,
            override_name: String::new(),
        }),
        TOGGLE_INERTIA => Box::new(SetWardleyInertia {
            body_path,
            element_id,
            inertia: toggled(value, subject.component()?.inertia),
        }),
        LINK | FLOW => Box::new(SetWardleyLink {
            body_path,
            source_element_id: element_id,
            target_element_id: value.to_owned(),
            kind: if action_id == LINK {
                WardleyLinkKind::Dependency
            } else {
                WardleyLinkKind::Flow
            },
            present: true,
            context: String::new(),
        }),
        UNLINK => {
            let links = links_of(subject);
            let chosen = if value.is_empty() && links.len() == 1 {
                links.into_iter().next()?
            } else {
                links.into_iter().find(|l| l.id == value)?
            };
            Box::new(SetWardleyLink {
                body_path,
                source_element_id: subject.id_of(&chosen.source),
                target_element_id: subject.id_of(&chosen.target),
                kind: chosen.kind,
                present: false,
                context: String::new(),
            })
        }
        ADD_TO_PIPELINE => Box::new(SetWardleyPipelineMembership {
            body_path,
            parent_element_id: element_id,
            child_name: trimmed,
            member: true,
            maturity: 0.5,
        }),
        REMOVE_FROM_PIPELINE => {
            let (pipeline, child) = subject.child()?;
            Box::new(SetWardleyPipelineMembership {
                body_path,
                parent_element_id: subject.id_of(&pipeline.parent),
                child_name: child.name.clone(),
                member: false,
                maturity: 0.5,
            })
        }
        _ => {
            let decorator = decorator_of_action(action_id)?;
            Box::new(SetWardleyDecorator {
                body_path,
                element_id,
                decorator,
                present: toggled(value, subject.component()?.decorators.contains(&decorator)),
            })
        }
    };
    Some(command)
}

//! What a diagram's menu offers for links between documents, for every diagram type that takes part in them
//! (etalii.adp spec 089; DISL sections 4.12 and 9.4, FBL section 17.5): the lookup an operation's `lookup` action
//! opens over the linkable elements of the workspace, a linked element's entry that opens its target, and an
//! element's backlinks, listed as "Linked from (N)", each opening the document that holds it.

use std::path::PathBuf;

use adp_context::{Action, ActionGroup, Prompt, PromptOption};
use adp_disl::{DislElement, IdSource};
use adp_links::{Link, Resolution, Workspace};

use crate::document::{DiagramDocument, text_of};
use crate::session::{ActionOutcome, ActionTarget, DiagramSession};
use crate::workspace::DiagramWorkspace;

/// The menu entry that lists an element's backlinks.
pub const LINKED_FROM: &str = "links.linkedFrom";

/// The menu entry of a linked element that opens its target.
pub const OPEN_LINKED: &str = "links.open";

/// What separates the path of a backlink's document from the element that holds it, in the answer that picks it.
const SEPARATOR: char = '\u{1f}';

/// Whether the operation `operation` of `document`'s specification opens the host's lookup (DISL section 9.4): one of
/// its actions is a `lookup`.
fn opens_lookup(document: &DiagramDocument, operation: &str) -> bool {
    document.tool().definition.specification().root()["behavior"]["operations"][operation]["actions"]
        .as_array()
        .is_some_and(|actions| actions.iter().any(|action| action.get("lookup").is_some()))
}

/// The element a backlink's holder names: a diagram's element by its id, a designer's row by the part before `/`.
fn holder_element(holder: &str) -> String {
    holder.split('/').next().unwrap_or(holder).to_owned()
}

/// The id the file writes for `element`, which a link to it names.
fn written_id(document: &DiagramDocument, element: &DislElement) -> String {
    crate::links::written_id(document, element)
}

/// The link a linked element holds, when its type stands for a linked element.
fn link_of(document: &DiagramDocument, element: &DislElement) -> Option<String> {
    let (attribute, _) = document
        .tool()
        .definition
        .specification()
        .metamodel()
        .linked(&element.type_name())?;
    let link = element.attributes().get(&attribute).map(text_of)?;
    (!link.is_empty()).then_some(link)
}

/// The links entries of `target`'s menu: Open linked element for a linked element whose link resolves, and Linked
/// from (N) for an element that has backlinks; none when the diagram takes no part in links.
pub fn menu(document: &DiagramDocument, target: &ActionTarget) -> Option<ActionGroup> {
    let ActionTarget::Element(id) = target else { return None };
    let index = document.link_index()?;
    let element = document.element(id)?;
    let mut actions = Vec::new();
    if let Some(link) = link_of(document, &element)
        && index.resolve(&link).resolved().is_some()
    {
        actions.push(Action::new(OPEN_LINKED, "Open linked element").with_icon("mdi-open-in-new"));
    }
    let backlinks = index.backlinks(document.body_path(), &written_id(document, &element));
    if !backlinks.is_empty() {
        actions
            .push(Action::new(LINKED_FROM, format!("Linked from ({})", backlinks.len())).with_icon("mdi-link-variant"));
    }
    (!actions.is_empty()).then_some(ActionGroup { actions })
}

/// How many links of the workspace a delete of `elements`, with everything beneath them, breaks (FBL section 17.5).
pub fn broken_by_deleting(document: &DiagramDocument, elements: &[DislElement]) -> usize {
    let Some(index) = document.link_index() else { return 0 };
    let mut ids: Vec<String> = Vec::new();
    for element in elements {
        ids.push(written_id(document, element));
        ids.extend(element.descendants().iter().map(|d| written_id(document, d)));
    }
    ids.sort();
    ids.dedup();
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    index.broken_by_deleting(document.body_path(), &ids)
}

/// The sentence a delete's confirmation adds when it breaks links; none when it breaks none.
pub fn breaks_sentence(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 link from another element points here and will be broken.".to_owned()),
        n => Some(format!("{n} links from other elements point here and will be broken.")),
    }
}

/// The options of the lookup over the linkable elements of `index` (DISL section 9.4): a heading per document, by
/// its path in the workspace, with its elements under it by name; each answers with the link to it.
fn lookup_options(index: &Workspace) -> Vec<PromptOption> {
    index
        .documents()
        .iter()
        .filter(|d| d.document_id.is_some() && !d.elements.is_empty())
        .map(|document| {
            let id = document.document_id.clone().unwrap_or_default();
            let children = document
                .elements
                .iter()
                .map(|element| {
                    let name = match document.name_property.as_deref() {
                        Some(property) => element.attribute(property).unwrap_or_default(),
                        None => element.attributes.first().map(|(_, v)| v.as_str()).unwrap_or_default(),
                    };
                    // The element is named by its type's own name property, so the link leaves `name-prop` out
                    // (FBL section 17.1), as the standalone host writes it: `$r6d3w8ne@roadmap.tml#launch=Launch`.
                    let link = Link::to_element(
                        id.clone(),
                        document.file_name(),
                        element.id.clone(),
                        None,
                        Some(name.to_owned()),
                    );
                    PromptOption {
                        id: link.text(),
                        label: if name.is_empty() {
                            element.id.clone()
                        } else {
                            name.to_owned()
                        },
                        selectable: true,
                        ..PromptOption::default()
                    }
                })
                .collect();
            PromptOption {
                id: index.display(&document.path),
                label: index.display(&document.path),
                selectable: false,
                children,
                ..PromptOption::default()
            }
        })
        .collect()
}

impl DiagramSession {
    /// Runs a links entry, or an operation that opens the lookup; none for any other action.
    pub(crate) fn link_action(
        &self,
        workspace: &DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
    ) -> Option<ActionOutcome> {
        let document = self.document(workspace.documents()).ok()?;
        let index = document.link_index()?;
        if action_id != OPEN_LINKED && action_id != LINKED_FROM {
            let entry = self.entry(document, target, action_id)?;
            if !opens_lookup(document, entry.operation.as_deref()?) {
                return None;
            }
            if !entry.available {
                return Some(ActionOutcome::Refused(entry.unavailable_reason));
            }
            return Some(ActionOutcome::Prompt(Prompt::Choice {
                title: entry.label.trim_end_matches(['\u{2026}', '.']).trim_end().to_owned(),
                icon: entry.icon.clone(),
                confirm_label: "Add".into(),
                options: lookup_options(index),
                empty_message: "No document of the workspace has elements to link to.".into(),
                name_field: None,
            }));
        }
        let ActionTarget::Element(id) = target else { return None };
        let element = document.element(id)?;
        match action_id {
            OPEN_LINKED => {
                let link = link_of(document, &element)?;
                Some(match index.resolve(&link) {
                    Resolution::Resolved(resolved) => ActionOutcome::Open {
                        path: resolved.path.clone(),
                        element_id: resolved.element.map(|e| e.id),
                    },
                    Resolution::Finding(finding) => ActionOutcome::Refused(finding.message),
                })
            }
            LINKED_FROM => {
                let backlinks = index.backlinks(document.body_path(), &written_id(document, &element));
                let options = backlinks
                    .iter()
                    .map(|backlink| {
                        let holder = holder_element(&backlink.holder);
                        let name = index
                            .document(&backlink.path)
                            .and_then(|d| {
                                let found = d.elements.iter().find(|e| e.id == holder)?;
                                match d.name_property.as_deref() {
                                    Some(property) => found.attribute(property).map(str::to_owned),
                                    None => found.attributes.first().map(|(_, v)| v.clone()),
                                }
                            })
                            .unwrap_or_else(|| holder.clone());
                        PromptOption {
                            id: format!("{}{SEPARATOR}{holder}", backlink.path.display()),
                            label: format!("{name} \u{2014} {}", index.display(&backlink.path)),
                            selectable: true,
                            ..PromptOption::default()
                        }
                    })
                    .collect();
                Some(ActionOutcome::Prompt(Prompt::Choice {
                    title: "Linked from".into(),
                    icon: "mdi-link-variant".into(),
                    confirm_label: "Open".into(),
                    options,
                    empty_message: "Nothing links here.".into(),
                    name_field: None,
                }))
            }
            _ => None,
        }
    }

    /// Answers a links entry's prompt: the backlink picked is opened, the element picked in a lookup is added as a
    /// linked element; none for any other action.
    pub(crate) fn link_answer(
        &self,
        workspace: &mut DiagramWorkspace,
        target: &ActionTarget,
        action_id: &str,
        value: &str,
        ids: Option<&dyn IdSource>,
    ) -> Option<ActionOutcome> {
        if action_id == LINKED_FROM {
            let ActionTarget::Element(_) = target else { return None };
            let (path, holder) = value.split_once(SEPARATOR)?;
            return Some(ActionOutcome::Open {
                path: PathBuf::from(path),
                element_id: Some(holder.to_owned()),
            });
        }
        let document = self.document(workspace.documents()).ok()?;
        document.link_index()?;
        let entry = self.entry(document, target, action_id)?;
        let operation = entry.operation.clone()?;
        if !opens_lookup(document, &operation) {
            return None;
        }
        if Link::parse(value).is_none() {
            return Some(ActionOutcome::Refused(format!("'{value}' is not a link.")));
        }
        let minted = adp_disl::behavior::ids::of(document.tool().definition.specification());
        let ids: &dyn IdSource = match (ids, &minted) {
            (Some(ids), _) => ids,
            (None, Ok(minted)) => minted,
            (None, Err(refusal)) => return Some(ActionOutcome::Refused(refusal.clone())),
        };
        Some(
            match self.run_operation(workspace, target, &entry, &operation, None, ids, Some(value.to_owned())) {
                Ok(()) => ActionOutcome::Done,
                Err(refusal) => ActionOutcome::Refused(refusal),
            },
        )
    }
}

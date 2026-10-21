//! The C4 rules, as a pure function from a parsed workspace to the problems in it, and what each view shows
//! (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4RuleSet.cs` and `C4Rules.cs`
//! at `8b6ccfc`. Every finding is a warning: a model mid-edit is routinely incomplete, and a tool that refuses an
//! unfinished diagram is worse than one that says what is unfinished. No rule is expressed in terms of colour.

use std::collections::HashSet;

use adp_diagram::{DiagramProblem, DiagramProblemLocation, DiagramProblemSeverity};

use crate::model::{C4Element, C4ElementKind, C4View, C4ViewKind, C4Workspace, folded, same};

/// Every rule's stable id, prefixed with the module's short name.
pub mod codes {
    pub const MISSING_DESCRIPTION: &str = "c4.missing-description";
    pub const MISSING_TECHNOLOGY: &str = "c4.missing-technology";
    pub const UNLABELLED_RELATIONSHIP: &str = "c4.unlabelled-relationship";
    pub const MISSING_PROTOCOL: &str = "c4.missing-protocol";
    pub const DANGLING_RELATIONSHIP: &str = "c4.dangling-relationship";
    pub const KIND_NOT_PERMITTED: &str = "c4.kind-not-permitted-on-view";
    pub const EMPTY_VIEW: &str = "c4.empty-view";
    pub const UNKNOWN_VIEW_SCOPE: &str = "c4.unknown-view-scope";
    pub const MISPLACED_ELEMENT: &str = "c4.misplaced-element";
    pub const INCLUDE_NOT_FOLLOWED: &str = "c4.include-not-followed";
    /// Mirrors Structurizr's `model.deploymentnode.description`.
    pub const MISSING_DEPLOYMENT_DESCRIPTION: &str = "c4.missing-deployment-description";
    /// Mirrors Structurizr's `model.deploymentnode.technology`.
    pub const MISSING_DEPLOYMENT_TECHNOLOGY: &str = "c4.missing-deployment-technology";
    /// Mirrors Structurizr's `model.infrastructurenode.description`.
    pub const MISSING_INFRASTRUCTURE_DESCRIPTION: &str = "c4.missing-infrastructure-description";
    /// Mirrors Structurizr's `model.infrastructurenode.technology`.
    pub const MISSING_INFRASTRUCTURE_TECHNOLOGY: &str = "c4.missing-infrastructure-technology";
    /// Mirrors Structurizr's `model.element.disconnected`.
    pub const DISCONNECTED_ELEMENT: &str = "c4.disconnected-element";
    /// Mirrors Structurizr's `model.element.noview`.
    pub const ELEMENT_NOT_ON_ANY_VIEW: &str = "c4.element-not-on-any-view";

    /// Every id above.
    pub const ALL: [&str; 16] = [
        MISSING_DESCRIPTION,
        MISSING_TECHNOLOGY,
        UNLABELLED_RELATIONSHIP,
        MISSING_PROTOCOL,
        DANGLING_RELATIONSHIP,
        KIND_NOT_PERMITTED,
        EMPTY_VIEW,
        UNKNOWN_VIEW_SCOPE,
        MISPLACED_ELEMENT,
        INCLUDE_NOT_FOLLOWED,
        MISSING_DEPLOYMENT_DESCRIPTION,
        MISSING_DEPLOYMENT_TECHNOLOGY,
        MISSING_INFRASTRUCTURE_DESCRIPTION,
        MISSING_INFRASTRUCTURE_TECHNOLOGY,
        DISCONNECTED_ELEMENT,
        ELEMENT_NOT_ON_ANY_VIEW,
    ];
}

/// What each kind must sit inside, or none when it sits at the top of the model.
fn required_parent_of(kind: C4ElementKind) -> Option<C4ElementKind> {
    match kind {
        C4ElementKind::Container => Some(C4ElementKind::SoftwareSystem),
        C4ElementKind::Component => Some(C4ElementKind::Container),
        _ => None,
    }
}

/// Which element kinds each view kind may show.
pub fn permitted_kinds(view_kind: C4ViewKind) -> &'static [C4ElementKind] {
    use C4ElementKind::*;
    match view_kind {
        C4ViewKind::SystemContext | C4ViewKind::SystemLandscape => &[Person, SoftwareSystem],
        C4ViewKind::Container => &[Person, SoftwareSystem, Container],
        C4ViewKind::Component | C4ViewKind::Dynamic => &[Person, SoftwareSystem, Container, Component],
        C4ViewKind::Deployment => &[
            DeploymentNode,
            InfrastructureNode,
            ContainerInstance,
            SoftwareSystemInstance,
        ],
    }
}

fn warning(message: String, rule_id: &str, location: Option<DiagramProblemLocation>) -> DiagramProblem {
    DiagramProblem {
        severity: DiagramProblemSeverity::Warning,
        message,
        rule_id: rule_id.to_owned(),
        location,
    }
}

fn at_element(element: &C4Element) -> Option<DiagramProblemLocation> {
    Some(DiagramProblemLocation::Element(element.id.clone()))
}

fn at_line(line: u32) -> Option<DiagramProblemLocation> {
    Some(DiagramProblemLocation::Line(line))
}

/// Every problem in `workspace`, across the whole model and all its views: the includes, each element's, each
/// relationship's, each view's, then the coverage.
pub fn validate(workspace: &C4Workspace) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    for include in &workspace.includes {
        problems.push(warning(
            format!(
                "This model includes '{include}', which ADP does not read. Anything declared there is missing from this diagram, and ADP will not edit it."
            ),
            codes::INCLUDE_NOT_FOLLOWED,
            None,
        ));
    }
    problems.extend(validate_elements(workspace));
    problems.extend(validate_relationships(workspace));
    for view in &workspace.views {
        problems.extend(validate_view(workspace, view));
    }
    problems.extend(validate_coverage(workspace));
    problems
}

/// Elements the model declares but no view draws, while the model declares any view.
fn validate_coverage(workspace: &C4Workspace) -> Vec<DiagramProblem> {
    if workspace.views.is_empty() {
        return Vec::new();
    }
    let drawn: HashSet<String> = workspace
        .views
        .iter()
        .flat_map(|view| members_of(workspace, view))
        .map(|element| element.id.clone())
        .collect();
    workspace
        .elements
        .iter()
        .filter(|element| !element.kind.is_instance() && !drawn.contains(&element.id))
        .map(|element| {
            warning(
                format!(
                    "'{}' is on no view. An element the model declares but nothing draws is invisible to every reader of the diagrams.",
                    label(element)
                ),
                codes::ELEMENT_NOT_ON_ANY_VIEW,
                at_element(element),
            )
        })
        .collect()
}

/// Every element some relationship crosses: the ends and their ancestors, less the ancestors both ends share.
fn connected_elements(workspace: &C4Workspace) -> HashSet<String> {
    let mut connected = HashSet::new();
    for relationship in &workspace.relationships {
        let source = ancestors_and_self(workspace, &relationship.source_id);
        let destination = ancestors_and_self(workspace, &relationship.destination_id);
        connected.extend(source.iter().filter(|id| !destination.contains(*id)).cloned());
        connected.extend(destination.iter().filter(|id| !source.contains(*id)).cloned());
    }
    connected
}

/// An element's id and every id above it, or nothing when the id names no element.
fn ancestors_and_self(workspace: &C4Workspace, id: &str) -> HashSet<String> {
    let mut chain = HashSet::new();
    let mut element = workspace.find(id);
    while let Some(current) = element {
        if !chain.insert(current.id.clone()) {
            break;
        }
        element = current.parent_id.as_deref().and_then(|p| workspace.find(p));
    }
    chain
}

fn validate_elements(workspace: &C4Workspace) -> Vec<DiagramProblem> {
    let connected = connected_elements(workspace);
    let model_is_underway = !workspace.views.is_empty();
    let mut problems = Vec::new();
    for element in &workspace.elements {
        if element.kind.is_instance() {
            continue;
        }
        if element.description.is_empty() {
            let rule = match element.kind {
                C4ElementKind::DeploymentNode => codes::MISSING_DEPLOYMENT_DESCRIPTION,
                C4ElementKind::InfrastructureNode => codes::MISSING_INFRASTRUCTURE_DESCRIPTION,
                _ => codes::MISSING_DESCRIPTION,
            };
            problems.push(warning(
                format!(
                    "'{}' has no description. C4 asks for a short description on every element, so a reader can tell at a glance what it is responsible for.",
                    label(element)
                ),
                rule,
                at_element(element),
            ));
        }
        if matches!(element.kind, C4ElementKind::Container | C4ElementKind::Component) && element.technology.is_empty()
        {
            problems.push(warning(
                format!(
                    "'{}' has no technology. C4 asks for one explicitly on every {}.",
                    label(element),
                    element.kind.lower()
                ),
                codes::MISSING_TECHNOLOGY,
                at_element(element),
            ));
        }
        let node_technology_rule = match element.kind {
            C4ElementKind::DeploymentNode => Some(codes::MISSING_DEPLOYMENT_TECHNOLOGY),
            C4ElementKind::InfrastructureNode => Some(codes::MISSING_INFRASTRUCTURE_TECHNOLOGY),
            _ => None,
        };
        if let Some(rule) = node_technology_rule
            && element.technology.is_empty()
        {
            problems.push(warning(
                format!(
                    "'{}' names no technology. A deployment diagram is read to find out what actually runs where, so say what this node is.",
                    label(element)
                ),
                rule,
                at_element(element),
            ));
        }
        let can_be_disconnected = matches!(
            element.kind,
            C4ElementKind::Person | C4ElementKind::SoftwareSystem | C4ElementKind::Container | C4ElementKind::Component
        );
        if model_is_underway && can_be_disconnected && !connected.contains(&element.id) {
            problems.push(warning(
                format!(
                    "'{}' is in no relationship at all. An element nothing reaches and that reaches nothing is either unfinished or left over.",
                    label(element)
                ),
                codes::DISCONNECTED_ELEMENT,
                at_element(element),
            ));
        }
        if let Some(required) = required_parent_of(element.kind) {
            let parent = element.parent_id.as_deref().and_then(|p| workspace.find(p));
            if parent.is_none_or(|parent| parent.kind != required) {
                let message = match parent {
                    None => format!(
                        "'{}' is a {} declared outside any {}. In C4 a {} is part of a {}.",
                        label(element),
                        spell(element.kind),
                        spell(required),
                        spell(element.kind),
                        spell(required)
                    ),
                    Some(parent) => format!(
                        "'{}' is a {} inside a {}. In C4 a {} is part of a {}.",
                        label(element),
                        spell(element.kind),
                        spell(parent.kind),
                        spell(element.kind),
                        spell(required)
                    ),
                };
                problems.push(warning(message, codes::MISPLACED_ELEMENT, at_element(element)));
            }
        }
    }
    problems
}

fn validate_relationships(workspace: &C4Workspace) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    for relationship in &workspace.relationships {
        let source = workspace.find(&relationship.source_id);
        let destination = workspace.find(&relationship.destination_id);
        let (Some(source), Some(destination)) = (source, destination) else {
            let missing = if source.is_none() {
                &relationship.source_id
            } else {
                &relationship.destination_id
            };
            problems.push(warning(
                format!(
                    "The relationship on line {} names '{missing}', which the model does not declare.",
                    relationship.line
                ),
                codes::DANGLING_RELATIONSHIP,
                at_line(relationship.line),
            ));
            continue;
        };
        if relationship.description.is_empty() {
            problems.push(warning(
                format!(
                    "The relationship from '{}' to '{}' has no label. C4 asks what the relationship is for, in the direction of the arrow.",
                    label(source),
                    label(destination)
                ),
                codes::UNLABELLED_RELATIONSHIP,
                at_line(relationship.line),
            ));
        }
        if relationship.technology.is_empty() {
            problems.push(warning(
                format!(
                    "The relationship from '{}' to '{}' names no technology. How two elements communicate - a protocol, a library, a call - is what tells a reader where a boundary really is.",
                    label(source),
                    label(destination)
                ),
                codes::MISSING_PROTOCOL,
                at_line(relationship.line),
            ));
        }
    }
    problems
}

fn validate_view(workspace: &C4Workspace, view: &C4View) -> Vec<DiagramProblem> {
    let mut problems = Vec::new();
    if let Some(scope) = &view.scope_id
        && workspace.find(scope).is_none()
    {
        problems.push(warning(
            format!(
                "The '{}' view is scoped to '{scope}', which the model does not declare.",
                view.key
            ),
            codes::UNKNOWN_VIEW_SCOPE,
            at_line(view.line),
        ));
    }
    let members = members_of(workspace, view);
    let permitted = permitted_kinds(view.kind);
    for element in members.iter().filter(|element| !permitted.contains(&element.kind)) {
        problems.push(warning(
            format!(
                "The '{}' view shows '{}', which is a {}. A {} view shows {}.",
                view.key,
                label(element),
                spell(element.kind),
                spell_view(view.kind),
                permitted.iter().map(|kind| spell(*kind)).collect::<Vec<_>>().join(", ")
            ),
            codes::KIND_NOT_PERMITTED,
            at_element(element),
        ));
    }
    if members.is_empty() {
        problems.push(warning(
            format!("The '{}' view is empty.", view.key),
            codes::EMPTY_VIEW,
            at_line(view.line),
        ));
    }
    problems
}

/// The elements a view shows: what its `include` statements name, in their order, then what `include *` adds within
/// the kinds the view permits, then on a dynamic view what its interactions name; less what it excludes and the
/// element the view is inside of, each once.
pub fn members_of<'a>(workspace: &'a C4Workspace, view: &C4View) -> Vec<&'a C4Element> {
    let permitted = permitted_kinds(view.kind);
    let mut named: Vec<&C4Element> = view.includes.iter().filter_map(|id| workspace.find(id)).collect();
    if view.includes_everything {
        named.extend(workspace.elements.iter().filter(|e| permitted.contains(&e.kind)));
    }
    if view.kind == C4ViewKind::Dynamic {
        named.extend(
            view.interactions
                .iter()
                .flat_map(|i| [i.source_id.as_str(), i.destination_id.as_str()])
                .filter_map(|id| workspace.find(id)),
        );
    }
    let mut seen = HashSet::new();
    named
        .into_iter()
        .filter(|element| !view.excludes.iter().any(|x| same(x, &element.id)))
        .filter(|element| !is_the_boundary_itself(view, element))
        .filter(|element| seen.insert(folded(&element.id)))
        .collect()
}

/// Whether this element is what the view is inside of: the software system of a container view, the container of a
/// component view, drawn as the boundary rather than as a box.
fn is_the_boundary_itself(view: &C4View, element: &C4Element) -> bool {
    matches!(view.kind, C4ViewKind::Container | C4ViewKind::Component)
        && view.scope_id.as_deref().is_some_and(|scope| same(&element.id, scope))
}

fn label(element: &C4Element) -> &str {
    if element.name.is_empty() {
        &element.id
    } else {
        &element.name
    }
}

/// A kind as a sentence spells it: `software system`, `deployment node`.
pub fn spell(kind: C4ElementKind) -> String {
    match kind {
        C4ElementKind::SoftwareSystem => "software system".to_owned(),
        C4ElementKind::DeploymentNode => "deployment node".to_owned(),
        C4ElementKind::InfrastructureNode => "infrastructure node".to_owned(),
        C4ElementKind::ContainerInstance => "container instance".to_owned(),
        C4ElementKind::SoftwareSystemInstance => "software system instance".to_owned(),
        _ => kind.lower(),
    }
}

fn spell_view(kind: C4ViewKind) -> String {
    match kind {
        C4ViewKind::SystemLandscape => "system landscape".to_owned(),
        C4ViewKind::SystemContext => "system context".to_owned(),
        _ => kind.name().to_lowercase(),
    }
}

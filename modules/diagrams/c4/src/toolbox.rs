//! What a C4 view offers in the toolbox, and the empty document a new C4 diagram starts from (etalii.adp spec 030,
//! task T061).
//!
//! Ported from the standalone host at `8b6ccfc`: `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4ToolboxProvider.cs`,
//! the empty documents of `ServiceCollection.AddC4.cs` and the seven types of `Diagram.cs`. The palette offers exactly
//! the kinds the view permits, drawn from the same rule the validator uses, so the two cannot disagree. A new document
//! is not an empty model: a C4 view is a view of something, so it carries one placeholder software system named after
//! the file, which the declared view can be scoped to.

use adp_diagram::ToolboxItem;

use crate::model::{C4ElementKind, C4ViewKind};
use crate::rules;

/// The C4 tool types of the catalogue, in the order C4 introduces the levels, and the view kind each binds; the code
/// level binds none.
pub const TYPES: [(&str, Option<C4ViewKind>); 7] = [
    ("c4/context", Some(C4ViewKind::SystemContext)),
    ("c4/container", Some(C4ViewKind::Container)),
    ("c4/component", Some(C4ViewKind::Component)),
    ("c4/code", None),
    ("c4/system-landscape", Some(C4ViewKind::SystemLandscape)),
    ("c4/dynamic", Some(C4ViewKind::Dynamic)),
    ("c4/deployment", Some(C4ViewKind::Deployment)),
];

/// The view kind the tool type `origin` binds, or none for `c4/code` and an origin that is not C4's.
pub fn view_kind_of(origin: &str) -> Option<C4ViewKind> {
    TYPES.iter().find(|(o, _)| *o == origin).and_then(|(_, kind)| *kind)
}

/// What a `c4/code` diagram answers: the one C4 type that cannot be completed on its own.
pub const CODE_UNAVAILABLE: &str = "C4 code diagrams are not available yet: this level uses UML class or entity-relationship notation, which ADP does not implement yet, and C4 itself recommends generating it from an IDE rather than drawing it by hand. Use a Component diagram to describe what is inside a container.";

/// The palette of a view of `view_kind`.
pub fn items_for(view_kind: C4ViewKind) -> Vec<ToolboxItem> {
    rules::permitted_kinds(view_kind)
        .iter()
        .map(|kind| ToolboxItem {
            id: format!("c4.toolbox.{}", kind.lower()),
            label: label(*kind).to_owned(),
            icon: icon_for(*kind).to_owned(),
            description: description_for(*kind, view_kind),
            drop_action_id: format!("c4.add-{}", kind.lower()),
        })
        .collect()
}

fn label(kind: C4ElementKind) -> &'static str {
    match kind {
        C4ElementKind::SoftwareSystem => "Software System",
        C4ElementKind::DeploymentNode => "Deployment Node",
        C4ElementKind::InfrastructureNode => "Infrastructure Node",
        C4ElementKind::ContainerInstance => "Container Instance",
        C4ElementKind::SoftwareSystemInstance => "Software System Instance",
        _ => kind.name(),
    }
}

fn icon_for(kind: C4ElementKind) -> &'static str {
    match kind {
        C4ElementKind::Person => "mdi-account-outline",
        C4ElementKind::SoftwareSystem | C4ElementKind::SoftwareSystemInstance => "mdi-application-outline",
        C4ElementKind::Container | C4ElementKind::ContainerInstance => "mdi-package-variant-closed",
        C4ElementKind::Component => "mdi-puzzle-outline",
        C4ElementKind::DeploymentNode => "mdi-server",
        C4ElementKind::InfrastructureNode => "mdi-lan",
    }
}

fn description_for(kind: C4ElementKind, view_kind: C4ViewKind) -> String {
    match kind {
        C4ElementKind::Container => "An application or a data store - not a Docker container.".to_owned(),
        C4ElementKind::Component => "A grouping of related functionality inside a container.".to_owned(),
        C4ElementKind::Person => "Someone who uses the software.".to_owned(),
        C4ElementKind::SoftwareSystem if view_kind == C4ViewKind::SystemContext => {
            "Another system this one talks to.".to_owned()
        }
        C4ElementKind::SoftwareSystem => "A software system.".to_owned(),
        C4ElementKind::DeploymentNode => "Where things run: a server, a cluster, an execution environment.".to_owned(),
        C4ElementKind::InfrastructureNode => "Supporting infrastructure: DNS, a load balancer, a firewall.".to_owned(),
        C4ElementKind::ContainerInstance => "A container, deployed onto a node.".to_owned(),
        C4ElementKind::SoftwareSystemInstance => label(kind).to_owned(),
    }
}

/// The empty Structurizr DSL document a new C4 diagram of `view_kind` starts from, its placeholder software system
/// named after `base_name`.
pub fn empty_document(view_kind: C4ViewKind, base_name: &str) -> String {
    let name = if base_name.trim().is_empty() {
        "system".to_owned()
    } else {
        base_name.trim().to_owned()
    };
    let id = identifier_for(&name);
    let mut text = String::new();
    text.push_str(&format!("workspace \"{}\" {{\n\n", escape(&name)));
    text.push_str("    model {\n");
    text.push_str(&format!("        {id} = softwareSystem \"{}\"", escape(&name)));
    // A component view is scoped to a container, so the placeholder reaches one level deeper, with the technology C4
    // asks of every container.
    if view_kind == C4ViewKind::Component {
        text.push_str(" {\n");
        text.push_str("            application = container \"Application\" \"\" \"Technology\"\n");
        text.push_str("        }\n");
    } else {
        text.push('\n');
    }
    // A deployment view names an environment, which exists only through a node deployed into it.
    if view_kind == C4ViewKind::Deployment {
        text.push('\n');
        text.push_str("        deploymentEnvironment \"Production\" {\n");
        text.push_str("            deploymentNode \"Server\" {\n");
        text.push_str("            }\n");
        text.push_str("        }\n");
    }
    text.push_str("    }\n\n");
    text.push_str("    views {\n");
    text.push_str(&view_declaration(view_kind, &id));
    text.push('\n');
    text.push_str("    }\n\n");
    text.push_str("}\n");
    text
}

fn view_declaration(view_kind: C4ViewKind, id: &str) -> String {
    let body = "            include *\n            autoLayout lr\n";
    match view_kind {
        C4ViewKind::SystemLandscape => format!("        systemLandscape \"landscape\" {{\n{body}        }}"),
        C4ViewKind::SystemContext => format!("        systemContext {id} \"context\" {{\n{body}        }}"),
        C4ViewKind::Container => format!("        container {id} \"containers\" {{\n{body}        }}"),
        // Scoped by the bare identifier: the DSL's identifiers are flat unless a document asks otherwise.
        C4ViewKind::Component => format!("        component application \"components\" {{\n{body}        }}"),
        C4ViewKind::Dynamic => format!("        dynamic {id} \"scenario\" {{\n            autoLayout lr\n        }}"),
        C4ViewKind::Deployment => {
            format!("        deployment {id} \"Production\" \"deployment\" {{\n{body}        }}")
        }
    }
}

/// A DSL identifier for a file name: letters, digits and underscores kept, everything else an underscore, trimmed of
/// underscores, a leading digit prefixed, `system` when nothing is left.
fn identifier_for(name: &str) -> String {
    let replaced: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let identifier = replaced.trim_matches('_');
    if identifier.is_empty() {
        return "system".to_owned();
    }
    if identifier.chars().next().is_some_and(char::is_numeric) {
        format!("_{identifier}")
    } else {
        identifier.to_owned()
    }
}

fn escape(value: &str) -> String {
    value.replace('"', "\\\"")
}

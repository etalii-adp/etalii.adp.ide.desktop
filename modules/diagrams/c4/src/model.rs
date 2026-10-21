//! The C4 model as a Structurizr DSL document declares it: its elements, relationships, views and styles (etalii.adp
//! spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/_Model` at `8b6ccfc`
//! (`C4Element.cs`, `C4View.cs`, `C4ViewKind.cs`) and the workspace record of `C4Parser.cs`. An element remembers the
//! line its declaration begins on, which is what makes an edit surgical: a rename replaces that one line and leaves
//! the document otherwise untouched. Identifiers are compared without regard to case, as the DSL compares them.

/// Whether two identifiers name the same thing: .NET's `StringComparison.OrdinalIgnoreCase`.
pub fn same(left: &str, right: &str) -> bool {
    left.chars()
        .flat_map(char::to_uppercase)
        .eq(right.chars().flat_map(char::to_uppercase))
}

/// An identifier in the form two identifiers that are [`same`] share, for keying a map by it.
pub fn folded(id: &str) -> String {
    id.chars().flat_map(char::to_uppercase).collect()
}

/// The C4 abstraction an element is. The hierarchy Person, Software System, Container, Component is what each view's
/// permitted kinds are drawn from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum C4ElementKind {
    Person,
    SoftwareSystem,
    Container,
    Component,
    DeploymentNode,
    InfrastructureNode,
    ContainerInstance,
    SoftwareSystemInstance,
}

impl C4ElementKind {
    /// Every kind, in the order the standalone host's enum declares them.
    pub const ALL: [C4ElementKind; 8] = [
        C4ElementKind::Person,
        C4ElementKind::SoftwareSystem,
        C4ElementKind::Container,
        C4ElementKind::Component,
        C4ElementKind::DeploymentNode,
        C4ElementKind::InfrastructureNode,
        C4ElementKind::ContainerInstance,
        C4ElementKind::SoftwareSystemInstance,
    ];

    /// The kind's name, as .NET's `ToString()` writes the enum member.
    pub fn name(self) -> &'static str {
        match self {
            C4ElementKind::Person => "Person",
            C4ElementKind::SoftwareSystem => "SoftwareSystem",
            C4ElementKind::Container => "Container",
            C4ElementKind::Component => "Component",
            C4ElementKind::DeploymentNode => "DeploymentNode",
            C4ElementKind::InfrastructureNode => "InfrastructureNode",
            C4ElementKind::ContainerInstance => "ContainerInstance",
            C4ElementKind::SoftwareSystemInstance => "SoftwareSystemInstance",
        }
    }

    /// The name in lower case (`softwaresystem`), as ids and messages spell it.
    pub fn lower(self) -> String {
        self.name().to_lowercase()
    }

    /// Whether this is an instance of another element, placed on a deployment node.
    pub fn is_instance(self) -> bool {
        matches!(
            self,
            C4ElementKind::ContainerInstance | C4ElementKind::SoftwareSystemInstance
        )
    }

    /// Whether C4 gives this kind a technology: a container, a component and a deployment or infrastructure node.
    pub fn has_technology(self) -> bool {
        matches!(
            self,
            C4ElementKind::Container
                | C4ElementKind::Component
                | C4ElementKind::DeploymentNode
                | C4ElementKind::InfrastructureNode
        )
    }
}

/// One element of a C4 model, as the document declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4Element {
    /// The DSL identifier, or a generated one when the declaration named none.
    pub id: String,
    pub kind: C4ElementKind,
    /// Its name, as shown on the diagram.
    pub name: String,
    /// Its short description: C4 asks for one on every element.
    pub description: String,
    /// Its technology; C4 asks for one on every container and component.
    pub technology: String,
    /// The tags it declares, which is how styles and "external" are applied.
    pub tags: Vec<String>,
    /// The element it sits inside, or none at the top of the model.
    pub parent_id: Option<String>,
    /// The 1-based line its declaration begins on.
    pub line: u32,
    /// For an instance, the container or system it is an instance of.
    pub referenced_id: Option<String>,
}

impl C4Element {
    /// Whether this element is outside the scope of what the model describes, which C4 draws muted: the tag
    /// `External` or `Existing System`.
    pub fn is_external(&self) -> bool {
        self.tags
            .iter()
            .any(|tag| same(tag, "External") || same(tag, "Existing System"))
    }
}

/// One relationship, which C4 requires to be unidirectional and labelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4Relationship {
    pub source_id: String,
    pub destination_id: String,
    pub description: String,
    pub technology: String,
    pub line: u32,
}

impl C4Relationship {
    /// The id a relationship goes by, since it has no DSL identifier of its own: its ends as written and its line.
    pub fn id(&self) -> String {
        format!("{}->{}@{}", self.source_id, self.destination_id, self.line)
    }
}

/// One ordered step of a dynamic view, whose numbering carries the order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4Interaction {
    pub source_id: String,
    pub destination_id: String,
    pub description: String,
    /// The interaction's number as written (`1`, `2`).
    pub order: String,
    pub line: u32,
}

/// The kind of view one C4 diagram shows. Six of the seven C4 diagram types bind exactly one of these; there is no
/// code view, which the DSL does not declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum C4ViewKind {
    SystemLandscape,
    SystemContext,
    Container,
    Component,
    Dynamic,
    Deployment,
}

impl C4ViewKind {
    /// The kind's name, as .NET's `ToString()` writes the enum member.
    pub fn name(self) -> &'static str {
        match self {
            C4ViewKind::SystemLandscape => "SystemLandscape",
            C4ViewKind::SystemContext => "SystemContext",
            C4ViewKind::Container => "Container",
            C4ViewKind::Component => "Component",
            C4ViewKind::Dynamic => "Dynamic",
            C4ViewKind::Deployment => "Deployment",
        }
    }
}

/// An `autoLayout` declaration: where the document carries one it wins over ADP's own arrangement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4AutoLayout {
    /// `tb`, `bt`, `lr` or `rl`, as written.
    pub direction: String,
    pub rank_separation: Option<i32>,
    pub node_separation: Option<i32>,
}

/// One view declared in the document's `views` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4View {
    pub kind: C4ViewKind,
    /// The view key, which a registration's `view:` header names.
    pub key: String,
    /// The element the view is about, or none for a landscape.
    pub scope_id: Option<String>,
    /// The deployment environment, for a deployment view only.
    pub environment: Option<String>,
    /// The title the document declares, or none to fall back to C4's wording.
    pub title: Option<String>,
    /// Whether the view declares `include *`.
    pub includes_everything: bool,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub auto_layout: Option<C4AutoLayout>,
    /// The ordered steps, for a dynamic view.
    pub interactions: Vec<C4Interaction>,
    /// The 1-based line the view declaration begins on.
    pub line: u32,
}

/// A tag-based element style, which is how a model overrides the default palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C4ElementStyle {
    pub tag: String,
    pub background: Option<String>,
    pub color: Option<String>,
    pub shape: Option<String>,
    pub border: Option<String>,
    pub line: u32,
}

/// What a document parsed to: its model, its relationships, its views, its styles and the files it includes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct C4Workspace {
    /// The workspace name, for a title that wants to say it.
    pub name: String,
    /// Every declared element, in document order.
    pub elements: Vec<C4Element>,
    /// Every declared relationship, in document order.
    pub relationships: Vec<C4Relationship>,
    /// Every declared view, in document order: the first is what a bare document opens.
    pub views: Vec<C4View>,
    pub styles: Vec<C4ElementStyle>,
    /// The files this document pulls in with `!include`, which ADP does not follow.
    pub includes: Vec<String>,
}

impl C4Workspace {
    /// The element with this id, or none; ids are compared without regard to case.
    pub fn find(&self, id: &str) -> Option<&C4Element> {
        self.elements.iter().find(|element| same(&element.id, id))
    }

    /// The view with this key, or none.
    pub fn find_view(&self, key: &str) -> Option<&C4View> {
        self.views.iter().find(|view| same(&view.key, key))
    }

    /// The relationship with this id, or none.
    pub fn relationship(&self, id: &str) -> Option<&C4Relationship> {
        self.relationships.iter().find(|relationship| relationship.id() == id)
    }
}

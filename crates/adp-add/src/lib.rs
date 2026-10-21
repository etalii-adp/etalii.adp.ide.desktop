//! What the add dialog offers and what it writes (etalii.adp spec 030, Story 5 scenario 1, FR-021, task T068),
//! ported from the standalone host at 8b6ccfc: `AddDiagramContextActionProvider`, `DiagramOptionTree`,
//! `DesignerAddOptions`, `DiagramFileName`, and `CreateDiagramFileCommandHandler` with `AdpFileWriter.CreateAll`. The
//! dialog itself is the shell's (`adp-shell`'s `add_dialog`).
//!
//! The add lists the standalone host's diagram definitions ([`adp_core::definitions`]), identity-only ones included,
//! one group per vendor, each tool type under its vendor by title, groups and tool types in ordinal order; a designer
//! type is a group under its vendor with a choice per format. Each tool type carries its state from the standalone
//! host's `docs/tools.md`. A choice suggests a free name; a folder-subject type takes no name and says why, and is
//! unavailable when the folder is already registered. Adding writes the registration, the tool type's MIME type and a
//! CRLF, and the body beside it from the tool type's module, byte for byte what the standalone host writes
//! (`parity/scripts/generic/timeline/add.json`).
//!
//! On a file the add registers the file instead: the tool types whose extension it has and whose test, if any, its
//! text passes, with no name, written as the `.adp` beside it.

use std::collections::HashMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use adp_core::definitions::{self, Definition};
use adp_core::{Catalogue, ToolState};
use adp_desl::{DesignerDefinition, DesignerFormat};
use adp_diagram::definition::{DiagramDocumentFactories, DiagramDocumentFactory, EmptyDocumentFactory};
use adp_hierarchy::validate_name;

/// The extension every registration carries (`DiagramFileName.Extension`).
pub const REGISTRATION_EXTENSION: &str = ".adp";
/// The line ending of everything the add writes (`AdpFileWriter.NewLine`).
const NEW_LINE: &str = "\r\n";
/// Separates a designer type from a format's extension in an option id; no origin holds it.
const FORMAT_SEPARATOR: char = '@';

pub const NO_DIAGRAM_TYPES: &str = "No diagram types are available.";
pub const NOT_REGISTRABLE: &str = "No diagram type reads this kind of file.";
pub const FOLDER_SUBJECT_SUPPRESSION: &str =
    "This type registers the folder itself, so the registration is created as `.adp` inside it.";
pub const INCOMPLETE: &str = "This diagram type cannot be created: its module is incomplete.";

/// What the add is asked on: a folder to create a document in, or a file to register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddTarget {
    Folder(PathBuf),
    File(PathBuf),
}

impl AddTarget {
    fn path(&self) -> &Path {
        match self {
            AddTarget::Folder(path) | AddTarget::File(path) => path,
        }
    }
}

/// The entry the explorer's context menu shows for a target: its label, and why it is unavailable when it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub label: &'static str,
    pub unavailable_reason: Option<&'static str>,
}

/// One row of the dialog's tree (`ContextOptionNode`): a vendor or a designer type, which groups and is not chosen,
/// or a tool type or format, which is.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddOption {
    /// The origin of a tool type, the origin and a format's extension of a designer format, the vendor of a group.
    pub id: String,
    pub label: String,
    pub selectable: bool,
    pub children: Vec<AddOption>,
    /// The name the field takes when this is chosen, free in the folder.
    pub suggested_value: String,
    pub description: String,
    pub icon: String,
    /// Why this takes no name, said where the field was.
    pub name_suppressed_reason: String,
    /// Why this cannot be chosen; shown greyed with the reason.
    pub unavailable_reason: String,
    /// The state the standalone host's `docs/tools.md` gives the tool type.
    pub state: Option<ToolState>,
}

/// What the dialog asks (`ContextChoiceRequest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddPrompt {
    pub title: String,
    pub confirm_label: String,
    pub options: Vec<AddOption>,
    pub empty_message: String,
    /// The label of the name field; none when the choice is the whole answer.
    pub name_field: Option<String>,
}

/// Writes the body of a new document of a designer type in a format, named `body_name`; none for a format its module
/// cannot write.
type DesignerTemplate = Box<dyn Fn(&DesignerFormat, &str) -> Option<String>>;

/// The tool types the add offers, and what writes a new document of each.
pub struct AddCatalogue {
    definitions: Vec<Definition>,
    states: HashMap<String, ToolState>,
    factories: DiagramDocumentFactories,
    designers: Vec<(DesignerDefinition, DesignerTemplate)>,
}

impl AddCatalogue {
    /// The standalone host's tool types, with the new documents of the modules the desktop host has.
    pub fn standalone() -> Self {
        let states = Catalogue::standalone()
            .types
            .into_iter()
            .map(|t| (t.origin, t.state))
            .collect();
        AddCatalogue {
            definitions: definitions::standalone(),
            states,
            factories: document_factories(),
            designers: vec![(
                adp_designer_knowledge::designer::definition(),
                Box::new(adp_designer_knowledge::designer::new_document),
            )],
        }
    }

    /// The designer types a document can be added of: those with a format.
    fn offered_designers(&self) -> impl Iterator<Item = &(DesignerDefinition, DesignerTemplate)> {
        self.designers.iter().filter(|(d, _)| !d.formats.is_empty())
    }

    /// The context menu's entry for `target`: Add… on every folder, Add as diagram… on a file a tool type reads and
    /// nothing on any other file (`DiscoverAsync`).
    pub fn offer(&self, target: &AddTarget) -> Option<Offer> {
        if !target.path().exists() {
            return None;
        }
        match target {
            AddTarget::Folder(_) => {
                let any = !self.definitions.is_empty() || self.offered_designers().next().is_some();
                Some(Offer {
                    label: "Add…",
                    unavailable_reason: (!any).then_some(NO_DIAGRAM_TYPES),
                })
            }
            AddTarget::File(file) => (!self.registrable(file).is_empty()).then_some(Offer {
                label: "Add as diagram…",
                unavailable_reason: None,
            }),
        }
    }

    /// The tool types that could claim `file`: those whose extension it has and whose test its text passes; none
    /// when a registration already sits beside it (`RegistrableTypesFor`).
    fn registrable(&self, file: &Path) -> Vec<&Definition> {
        let extension = file
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        if extension.is_empty() || extension.eq_ignore_ascii_case(REGISTRATION_EXTENSION) {
            return Vec::new();
        }
        if file.with_extension("adp").exists() {
            return Vec::new();
        }
        let text = std::cell::OnceCell::new();
        let read = || {
            text.get_or_init(|| fs::read(file).ok().map(|b| String::from_utf8_lossy(&b).into_owned()))
                .clone()
        };
        self.definitions
            .iter()
            .filter(|d| d.extension.eq_ignore_ascii_case(&extension))
            .filter(|d| d.suggests(read))
            .collect()
    }

    /// What the dialog asks for `target` (`ExecuteAsync`), or why it cannot be asked.
    pub fn execute(&self, target: &AddTarget) -> Result<AddPrompt, String> {
        if !target.path().exists() {
            return Err(match target {
                AddTarget::Folder(_) => "The folder no longer exists.".into(),
                AddTarget::File(_) => "The file no longer exists.".into(),
            });
        }
        match target {
            AddTarget::File(file) => {
                let registrable = self.registrable(file);
                if registrable.is_empty() {
                    return Err(NOT_REGISTRABLE.into());
                }
                Ok(AddPrompt {
                    title: format!("Open {} as", file_name(file)),
                    confirm_label: "Register".into(),
                    options: option_tree(&registrable, |d| {
                        self.leaf(d, |_| (String::new(), String::new(), String::new()))
                    }),
                    empty_message: NOT_REGISTRABLE.into(),
                    name_field: None,
                })
            }
            AddTarget::Folder(folder) => {
                let registered_by = self.existing_folder_registration(folder);
                let definitions: Vec<&Definition> = self.definitions.iter().collect();
                let options = option_tree(&definitions, |d| {
                    self.leaf(d, |d| {
                        if d.has_folder_subject() {
                            (
                                String::new(),
                                FOLDER_SUBJECT_SUPPRESSION.to_owned(),
                                registered_by
                                    .as_ref()
                                    .map(|r| format!("This folder is already registered by `{r}`"))
                                    .unwrap_or_default(),
                            )
                        } else {
                            (suggest(&type_name(d), folder), String::new(), String::new())
                        }
                    })
                });
                Ok(AddPrompt {
                    title: "Add diagram".into(),
                    confirm_label: "Add".into(),
                    options: self.add_designers(options, folder),
                    empty_message: NO_DIAGRAM_TYPES.into(),
                    name_field: Some("Name".into()),
                })
            }
        }
    }

    /// One choosable tool type, with the suggested name, the reason it takes no name and the reason it is
    /// unavailable that `annotate` gives it.
    fn leaf(&self, definition: &Definition, annotate: impl Fn(&Definition) -> (String, String, String)) -> AddOption {
        let (suggested_value, name_suppressed_reason, unavailable_reason) = annotate(definition);
        AddOption {
            id: definition.origin.clone(),
            label: definition.title.clone(),
            selectable: true,
            children: Vec::new(),
            suggested_value,
            description: definition.description.clone(),
            icon: definition.icon.clone(),
            name_suppressed_reason,
            unavailable_reason,
            state: self.states.get(&definition.origin).copied(),
        }
    }

    /// The tree with the designer types added under their vendors, each vendor's entries by label (`AddTo`).
    fn add_designers(&self, tree: Vec<AddOption>, folder: &Path) -> Vec<AddOption> {
        let mut vendors: Vec<AddOption> = tree;
        let mut by_vendor: Vec<(String, Vec<&DesignerDefinition>)> = Vec::new();
        for (designer, _) in self.offered_designers() {
            let vendor = designer.origin.split('/').next().unwrap_or_default().to_owned();
            match by_vendor.iter_mut().find(|(v, _)| *v == vendor) {
                Some((_, members)) => members.push(designer),
                None => by_vendor.push((vendor, vec![designer])),
            }
        }
        for (vendor, designers) in by_vendor {
            let entries = designers.into_iter().map(|d| self.designer_entry(d, folder));
            match vendors.iter_mut().find(|v| v.id == vendor) {
                Some(existing) => {
                    existing.children.extend(entries);
                    existing.children.sort_by(|a, b| a.label.cmp(&b.label));
                }
                None => {
                    let mut children: Vec<AddOption> = entries.collect();
                    children.sort_by(|a, b| a.label.cmp(&b.label));
                    vendors.push(group(&vendor, &vendor, children));
                }
            }
        }
        vendors.sort_by(|a, b| a.id.cmp(&b.id));
        vendors
    }

    /// One designer type: a heading that is not chosen itself, with a choice per format (`Entry`).
    fn designer_entry(&self, designer: &DesignerDefinition, folder: &Path) -> AddOption {
        let type_name = designer
            .origin
            .split_once('/')
            .map(|(_, t)| t.replace('/', "-"))
            .unwrap_or_default();
        let suggested = suggest(&type_name, folder);
        AddOption {
            id: designer.origin.clone(),
            label: designer.title.clone(),
            selectable: false,
            children: designer
                .formats
                .iter()
                .map(|format| AddOption {
                    id: format!("{}{FORMAT_SEPARATOR}{}", designer.origin, format.extension),
                    label: format.title.clone(),
                    selectable: true,
                    suggested_value: suggested.clone(),
                    description: designer.description.clone(),
                    icon: designer.icon.clone(),
                    ..AddOption::default()
                })
                .collect(),
            description: designer.description.clone(),
            icon: designer.icon.clone(),
            state: self.states.get(&designer.origin).copied(),
            ..AddOption::default()
        }
    }

    /// The name of the registration that already registers `folder` as a folder-subject diagram, if any.
    fn existing_folder_registration(&self, folder: &Path) -> Option<String> {
        if !self.definitions.iter().any(Definition::has_folder_subject) {
            return None;
        }
        let mut names: Vec<PathBuf> = fs::read_dir(folder)
            .ok()?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && is_registration(p))
            .collect();
        names.sort();
        names.into_iter().find_map(|path| {
            let origin = adp_fbl::RegistrationDocument::read(fs::read(&path).ok()?).origin;
            self.definitions
                .iter()
                .any(|d| d.has_folder_subject() && d.mime_type() == origin)
                .then(|| file_name(&path))
        })
    }

    /// Judges a name typed into the dialog: the registration it would make must be a free name in the folder
    /// (`ValidateName`).
    pub fn validate(&self, target: &AddTarget, name: &str) -> Result<(), String> {
        let folder = match target {
            AddTarget::Folder(folder) => folder.as_path(),
            AddTarget::File(file) => file.parent().unwrap_or(Path::new("")),
        };
        if strip_extension(name).trim().is_empty() {
            return Err("Enter a name.".into());
        }
        validate_name(&with_extension(name), folder, None)
    }

    /// Creates what the chosen option `value` names, called `text` where it takes a name (`CommitAsync`); the
    /// registration written.
    pub fn commit(&self, target: &AddTarget, value: &str, text: &str) -> Result<PathBuf, String> {
        if !target.path().exists() {
            return Err(match target {
                AddTarget::Folder(_) => "The folder no longer exists.".into(),
                AddTarget::File(_) => "The file no longer exists.".into(),
            });
        }
        let folder = match target {
            AddTarget::File(file) => return self.register(file, value),
            AddTarget::Folder(folder) => folder,
        };
        if let Some((designer, template, format)) = self.resolve_designer(value) {
            self.validate(target, text)?;
            let base_name = strip_extension(text);
            let body_name = format!("{base_name}{}", format.extension);
            if folder.join(&body_name).exists() {
                return Err(format!("An item named '{body_name}' already exists in this folder."));
            }
            let Some(content) = template(format, &body_name) else {
                return Err(format!(
                    "A {} document cannot be created as {}: its module is incomplete.",
                    designer.title, format.title
                ));
            };
            let registration = format!("{}{NEW_LINE}body: {body_name}{NEW_LINE}", designer.origin);
            return create_all(
                folder,
                &[(with_extension(&base_name), registration), (body_name, content)],
            );
        }
        let Some(definition) = self.definitions.iter().find(|d| d.origin == value) else {
            return Err("That diagram type is not available.".into());
        };
        let file_name = if definition.has_folder_subject() {
            if let Some(registered_by) = self.existing_folder_registration(folder) {
                return Err(format!("This folder is already registered by '{registered_by}'."));
            }
            REGISTRATION_EXTENSION.to_owned()
        } else {
            self.validate(target, text)?;
            with_extension(text)
        };
        let mut files = vec![(file_name.clone(), format!("{}{NEW_LINE}", definition.mime_type()))];
        if definition.has_document_sibling() {
            let Some(factory) = self.factories.find(&definition.origin) else {
                return Err(INCOMPLETE.into());
            };
            let base_name = strip_extension(&file_name);
            let sibling = format!("{base_name}{}", definition.extension);
            if folder.join(&sibling).exists() {
                return Err(format!("An item named '{sibling}' already exists in this folder."));
            }
            files.push((sibling, factory.create_empty_document(&base_name)));
        }
        create_all(folder, &files)
    }

    /// Writes the registration beside `file`, naming the tool type `value` and leaving the file alone (`RegisterAsync`).
    fn register(&self, file: &Path, value: &str) -> Result<PathBuf, String> {
        let Some(definition) = self.registrable(file).into_iter().find(|d| d.origin == value) else {
            return Err("That diagram type is not available for this file.".into());
        };
        let folder = file.parent().unwrap_or(Path::new(""));
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        create_all(
            folder,
            &[(with_extension(&stem), format!("{}{NEW_LINE}", definition.mime_type()))],
        )
    }

    /// The designer type and format an option id names (`Resolve`).
    fn resolve_designer(&self, value: &str) -> Option<(&DesignerDefinition, &DesignerTemplate, &DesignerFormat)> {
        let (origin, extension) = value.rsplit_once(FORMAT_SEPARATOR)?;
        let (designer, template) = self.offered_designers().find(|(d, _)| d.origin == origin)?;
        let format = designer.formats.iter().find(|f| f.extension == extension)?;
        Some((designer, template, format))
    }
}

/// What writes the body of a new diagram of each tool type the desktop host has a module for, as the standalone
/// host's modules register it with `AddEmptyDocument`; a tool type with a body and no factory here cannot be added.
pub fn document_factories() -> DiagramDocumentFactories {
    use adp_diagram_c4::model::C4ViewKind;
    let mut factories: Vec<Box<dyn DiagramDocumentFactory>> = [
        C4ViewKind::SystemLandscape,
        C4ViewKind::SystemContext,
        C4ViewKind::Container,
        C4ViewKind::Component,
        C4ViewKind::Dynamic,
        C4ViewKind::Deployment,
    ]
    .into_iter()
    .map(|kind| {
        Box::new(EmptyDocumentFactory::from_base_name(
            adp_diagram_c4::commands::origin_for(kind),
            move |base| adp_diagram_c4::toolbox::empty_document(kind, base),
        )) as Box<dyn DiagramDocumentFactory>
    })
    .collect();
    factories.push(Box::new(EmptyDocumentFactory::from_base_name(
        adp_diagram_mindmap::ORIGIN,
        adp_diagram_mindmap::document::MindmapDocument::empty_document,
    )));
    // `ServiceCollection.AddTimeline.cs`.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_timeline::ORIGIN,
        "timeline: 1\r\nelements: []\r\n",
    )));
    // `ServiceCollection.AddCausalLoop.cs`: the binding's template.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_causal_loop::ORIGIN,
        adp_diagram_causal_loop::TEMPLATE,
    )));
    // `ServiceCollection.AddWardleyMap.cs`: a `title` line carrying the file's base name.
    factories.push(Box::new(EmptyDocumentFactory::from_base_name(
        adp_diagram_wardley_map::ORIGIN,
        adp_diagram_wardley_map::empty_document,
    )));
    // `ServiceCollection.AddSupplyChain.cs`: `SupplyChainParser.EmptyDocument`.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_supply_chain::ORIGIN,
        adp_diagram_supply_chain::TEMPLATE,
    )));
    // `ServiceCollection.AddDatabricks.cs`: `EmptyDocument`, one per type of the family, its keys made from the file's
    // name.
    factories.push(Box::new(EmptyDocumentFactory::from_base_name(
        adp_diagram_databricks::BUNDLE_ORIGIN,
        adp_diagram_databricks::new_bundle,
    )));
    factories.push(Box::new(EmptyDocumentFactory::from_base_name(
        adp_diagram_databricks::JOB_ORIGIN,
        adp_diagram_databricks::new_job,
    )));
    factories.push(Box::new(EmptyDocumentFactory::from_base_name(
        adp_diagram_databricks::PIPELINE_ORIGIN,
        adp_diagram_databricks::new_pipeline,
    )));
    // `ServiceCollection.AddSankey.cs`: `SankeyParser.EmptyDocument`.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_sankey::ORIGIN,
        adp_diagram_sankey::EMPTY_DOCUMENT,
    )));
    // `ServiceCollection.AddFunctionalDecompositionGraph.cs`: the header and two empty sections, in CRLF.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_functional_decomposition_graph::ORIGIN,
        adp_diagram_functional_decomposition_graph::EMPTY_DOCUMENT,
    )));
    // `ServiceCollection.AddGartnerHypeCycleGraph.cs`: `GhgParser.EmptyDocument("\r\n")`.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_hype_cycle::ORIGIN,
        adp_diagram_hype_cycle::EMPTY_DOCUMENT,
    )));
    // `ServiceCollection.AddAgentActivityDiagram.cs`: `AadBody.EmptyDocument("\r\n")`, the header alone.
    factories.push(Box::new(EmptyDocumentFactory::fixed(
        adp_diagram_agent_activity::ORIGIN,
        adp_diagram_agent_activity::TEMPLATE,
    )));
    DiagramDocumentFactories::new(factories)
}

/// The definitions as the dialog's tree (`DiagramOptionTree.Build`): one group per vendor, each tool type under its
/// vendor by title and a subtype under the type it refines, all in ordinal order.
fn option_tree(definitions: &[&Definition], leaf: impl Fn(&Definition) -> AddOption) -> Vec<AddOption> {
    let mut vendors: Vec<&str> = definitions.iter().map(|d| d.vendor()).collect();
    vendors.sort();
    vendors.dedup();
    vendors
        .into_iter()
        .map(|vendor| {
            let of_vendor: Vec<&Definition> = definitions.iter().copied().filter(|d| d.vendor() == vendor).collect();
            let mut types: Vec<&str> = of_vendor.iter().map(|d| d.type_name()).collect();
            types.sort();
            types.dedup();
            let mut nodes: Vec<AddOption> = types
                .into_iter()
                .map(|type_name| {
                    let of_type: Vec<&Definition> = of_vendor
                        .iter()
                        .copied()
                        .filter(|d| d.type_name() == type_name)
                        .collect();
                    let mut subtypes: Vec<&Definition> =
                        of_type.iter().copied().filter(|d| !d.subtype().is_empty()).collect();
                    subtypes.sort_by(|a, b| a.title.cmp(&b.title));
                    let subtypes: Vec<AddOption> = subtypes.into_iter().map(&leaf).collect();
                    match of_type.iter().find(|d| d.subtype().is_empty()) {
                        Some(plain) => AddOption {
                            children: subtypes,
                            ..leaf(plain)
                        },
                        None => group(&format!("{vendor}/{type_name}"), type_name, subtypes),
                    }
                })
                .collect();
            nodes.sort_by(|a, b| a.label.cmp(&b.label));
            group(vendor, vendor, nodes)
        })
        .collect()
}

fn group(id: &str, label: &str, children: Vec<AddOption>) -> AddOption {
    AddOption {
        id: id.to_owned(),
        label: label.to_owned(),
        children,
        ..AddOption::default()
    }
}

/// The name a tool type suggests: its type segment, and its subtype after a hyphen.
fn type_name(definition: &Definition) -> String {
    match definition.subtype() {
        "" => definition.type_name().to_owned(),
        subtype => format!("{}-{subtype}", definition.type_name()),
    }
}

/// A free name in `folder` starting from `type_name`, numbered from 2 when it is taken (`DiagramFileName.Suggest`).
pub fn suggest(type_name: &str, folder: &Path) -> String {
    let base = sanitise(type_name);
    let free = |name: &str| !folder.join(with_extension(name)).exists();
    if free(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|candidate| free(candidate))
        .unwrap_or(base)
}

/// Anything a file name cannot carry, a dot and a space, replaced with a hyphen; hyphens collapsed and trimmed
/// (`DiagramFileName.Sanitise`).
pub fn sanitise(value: &str) -> String {
    let replaced: String = value
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|. ".contains(c) || (c as u32) < 32 {
                '-'
            } else {
                c
            }
        })
        .collect();
    let mut collapsed = replaced;
    while collapsed.contains("--") {
        collapsed = collapsed.replace("--", "-");
    }
    let trimmed = collapsed.trim_matches('-');
    if trimmed.is_empty() {
        "diagram".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `name` trimmed, without one trailing `.adp` (`DiagramFileName.StripExtension`).
pub fn strip_extension(name: &str) -> String {
    let name = name.trim();
    let cut = name.len().saturating_sub(REGISTRATION_EXTENSION.len());
    match name.get(cut..) {
        Some(tail) if tail.eq_ignore_ascii_case(REGISTRATION_EXTENSION) => name[..cut].to_owned(),
        _ => name.to_owned(),
    }
}

/// The registration's file name for a base name: always exactly one `.adp`.
pub fn with_extension(name: &str) -> String {
    format!("{}{REGISTRATION_EXTENSION}", strip_extension(name))
}

/// Whether the file at `path` is a registration: its name ends in `.adp`, `.adp` alone included
/// (`DiagramFilePair.IsRegistrationFile`).
fn is_registration(path: &Path) -> bool {
    file_name(path).to_lowercase().ends_with(REGISTRATION_EXTENSION)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Creates every file in `folder`, in order, each under a name that must be free, as UTF-8 without a byte order mark;
/// all of them or none (`AdpFileWriter.CreateAll` through `CreateDiagramFileCommandHandler`). The first file's path.
fn create_all(folder: &Path, files: &[(String, String)]) -> Result<PathBuf, String> {
    if !folder.is_dir() {
        return Err("The folder no longer exists.".into());
    }
    let mut created: Vec<PathBuf> = Vec::new();
    for (name, content) in files {
        let path = folder.join(name);
        let written = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .and_then(|mut file| file.write_all(content.as_bytes()));
        if let Err(e) = written {
            let taken = e.kind() == std::io::ErrorKind::AlreadyExists;
            if !taken {
                let _ = fs::remove_file(&path);
            }
            for path in created {
                let _ = fs::remove_file(path);
            }
            return Err(if taken {
                format!("An item named '{}' already exists in this folder.", files[0].0)
            } else {
                format!("Could not create the diagram: {e}")
            });
        }
        created.push(path);
    }
    Ok(folder.join(&files[0].0))
}

/// The option `id`, anywhere in `options`.
pub fn find<'o>(options: &'o [AddOption], id: &str) -> Option<&'o AddOption> {
    options
        .iter()
        .find_map(|o| if o.id == id { Some(o) } else { find(&o.children, id) })
}

/// The first selectable option, depth first: what the name field starts out describing.
pub fn first_selectable(options: &[AddOption]) -> Option<&AddOption> {
    options.iter().find_map(|o| {
        if o.selectable {
            Some(o)
        } else {
            first_selectable(&o.children)
        }
    })
}

#[cfg(test)]
mod tests;

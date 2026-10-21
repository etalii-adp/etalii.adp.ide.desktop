//! A diagram tool type of the catalogue, loaded for opening documents: its DISL specification and the FBL binding its
//! bodies are read and written with (etalii.adp spec 030, FR-030 and FR-031; data-model.md, ToolType).
//!
//! The standalone host bundled each module's specification and binding as embedded resources (`BundledDefinition`,
//! `BundledBinding`) and loaded them once per module; the desktop host loads them from the vendored files the
//! catalogue names, once per tool type, through [`DiagramTools`].

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use adp_core::{Catalogue, ToolKind, ToolType};
use adp_disl::DislPluginFunctions;
use adp_disl::model::{attribute_types, natural_ids};
use adp_fbl::documents::{self, FblBinding};
use adp_fbl::{FblOptions, RegistrationDocument};

use crate::definition::DiagramDefinition;
use crate::disl_definition::DislDiagramDefinition;
use crate::module::DiagramModule;

/// A diagram tool type whose specification and binding are loaded.
#[derive(Debug)]
pub struct DiagramTool {
    pub origin: String,
    pub display_name: String,
    pub definition: DislDiagramDefinition,
    pub binding: Rc<FblBinding>,
    /// The extensions it opens, lower case with their dot.
    pub extensions: Vec<String>,
    /// The tool type's own code beside its specification, when it has any.
    pub module: Option<Rc<dyn DiagramModule>>,
}

impl DiagramTool {
    /// Loads `tool_type`'s specification and its first binding, with the host's implementations of the
    /// specification's plugin functions; the reason, in a sentence, when it cannot be opened.
    pub fn load(tool_type: &ToolType, plugins: &DislPluginFunctions) -> Result<DiagramTool, String> {
        if tool_type.kind != ToolKind::Diagram {
            return Err(format!("'{}' is not a diagram.", tool_type.origin));
        }
        let Some(specification) = tool_type.specification.as_deref() else {
            return Err(format!("'{}' diagrams cannot be opened yet.", tool_type.origin));
        };
        let bytes = fs::read(specification).map_err(|e| format!("{}: {e}", specification.display()))?;
        let definition = DislDiagramDefinition::load(&bytes, None, plugins)?;
        let wire_block = DislDiagramDefinition::wire_block_of(definition.specification());
        let definition = DislDiagramDefinition::of(
            definition.specification().clone(),
            definition.diagnostics().to_vec(),
            wire_block.as_deref(),
        );
        let (Some(reference), Some(file)) = (tool_type.bindings.first(), tool_type.binding_files.first()) else {
            return Err(format!(
                "'{}' names no format binding, so its documents cannot be read.",
                tool_type.origin
            ));
        };
        let binding = load_binding(file, reference)?;
        Ok(DiagramTool {
            origin: tool_type.origin.clone(),
            display_name: tool_type.display_name.clone(),
            definition,
            binding: Rc::new(binding),
            extensions: tool_type.extensions.clone(),
            module: None,
        })
    }

    /// The tool type as the host's definition: its origin, its title and the extension of its bodies.
    pub fn diagram_definition(&self) -> DiagramDefinition {
        DiagramDefinition {
            origin: self.origin.clone(),
            title: self.display_name.clone(),
            extension: self.binding.claims.extensions.first().cloned().unwrap_or_default(),
            ..DiagramDefinition::default()
        }
    }

    /// The text of a new, empty body: the binding's template.
    pub fn empty_body(&self) -> Option<String> {
        self.binding.template.as_ref().map(|t| t.text.clone())
    }

    /// The settings for reading a body named `file_name` through the binding: the specification's id strategy and
    /// attribute types, and what the registration it was opened through says.
    pub fn options(&self, file_name: &str, registration: Option<&RegistrationDocument>) -> FblOptions {
        let specification = self.definition.specification();
        let mut options = FblOptions {
            file_name: file_name.to_owned(),
            derive_id: Some(natural_ids::of(specification)),
            attribute_type: Some(attribute_types::of(specification)),
            time_attributes: attribute_types::time_attributes(specification, &self.binding),
            ..FblOptions::default()
        };
        if let Some(registration) = registration {
            options.registration_headers = registration
                .headers
                .iter()
                .map(|h| (h.key.clone(), h.value.clone()))
                .collect();
            options.resource = registration.resource().map(str::to_owned);
            options.identities = registration.identity_map();
        }
        options
    }
}

/// Loads binding `reference` (`file.fbl#name`, or an address ending so) from `file`.
fn load_binding(file: &Path, reference: &str) -> Result<FblBinding, String> {
    let (problems, document) = documents::load_file(file);
    let Some(document) = document else {
        return Err(format!(
            "The binding {} does not load: {}",
            file.display(),
            problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ")
        ));
    };
    let name = reference.rsplit_once('#').map(|(_, name)| name);
    let binding = match name {
        Some(name) => document.binding(name),
        None => document.bindings.first().map(|(_, binding)| binding),
    };
    binding
        .cloned()
        .ok_or_else(|| format!("{} has no binding '{}'.", file.display(), name.unwrap_or_default()))
}

/// The diagram tool types of the catalogue, each loaded the first time a document of it is opened.
pub struct DiagramTools {
    catalogue: Catalogue,
    plugins: Box<dyn Fn(&str) -> DislPluginFunctions>,
    modules: HashMap<String, Rc<dyn DiagramModule>>,
    loaded: HashMap<String, Result<Rc<DiagramTool>, String>>,
}

impl DiagramTools {
    /// The tool types of `catalogue`, whose plugin functions `plugins` gives by origin.
    pub fn new(catalogue: Catalogue, plugins: impl Fn(&str) -> DislPluginFunctions + 'static) -> Self {
        DiagramTools {
            catalogue,
            plugins: Box::new(plugins),
            modules: HashMap::new(),
            loaded: HashMap::new(),
        }
    }

    /// Registers the tool type `origin`'s own code: its plugin functions take the place of what `plugins` gives for
    /// that origin, and its session asks it what the specification cannot state.
    pub fn with_module(mut self, origin: &str, module: Rc<dyn DiagramModule>) -> Self {
        self.loaded.remove(origin);
        self.modules.insert(origin.to_owned(), module);
        self
    }

    pub fn catalogue(&self) -> &Catalogue {
        &self.catalogue
    }

    /// The tool type of `origin`, loaded, or why it cannot open anything.
    pub fn get(&mut self, origin: &str) -> Result<Rc<DiagramTool>, String> {
        if !self.loaded.contains_key(origin) {
            let loaded = match self.catalogue.by_origin(origin) {
                None => Err(format!("No tool type '{origin}' is known.")),
                Some(tool_type) => {
                    let module = self.modules.get(origin).cloned();
                    let plugins = match &module {
                        Some(module) => module.plugin_functions(),
                        None => (self.plugins)(origin),
                    };
                    DiagramTool::load(tool_type, &plugins).map(|mut tool| {
                        tool.module = module;
                        Rc::new(tool)
                    })
                }
            };
            self.loaded.insert(origin.to_owned(), loaded);
        }
        self.loaded[origin].clone()
    }

    /// The origins of the diagram tool types that open a file named `name` by its extension, in catalogue order.
    pub fn for_file(&self, name: &str) -> Vec<String> {
        self.catalogue
            .for_file(name)
            .into_iter()
            .filter(|t| t.kind == ToolKind::Diagram)
            .map(|t| t.origin.clone())
            .collect()
    }
}

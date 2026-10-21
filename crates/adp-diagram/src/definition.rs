//! What a diagram tool type is to the host, how a new document of it is written, and its toolbox entries (etalii.adp
//! spec 030, FR-018 and FR-021); ported from the standalone host's `EtAlii.Adp.Diagram` at `8b6ccfc`:
//! `DiagramDefinition.cs`, `IDiagramDocumentFactory` with `EmptyDocumentFactory.cs` and `DiagramDocumentFactories.cs`,
//! and `_Model/ToolboxItemDefinition.cs`.
//!
//! The standalone host found its definitions by reflection over its module assemblies (`DiagramDefinitionDiscovery`);
//! here the tool-type catalogue (`adp_core::Catalogue`) lists them from the vendored specifications, so discovery is
//! not ported.

use std::collections::HashMap;

/// What a diagram is: its document, or the folder its registration sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiagramSubject {
    #[default]
    Document,
    Folder,
}

/// One diagram tool type as the host knows it: its origin, its title, and where its body lives.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiagramDefinition {
    /// `<vendor>/<type>`.
    pub origin: String,
    pub title: String,
    /// One sentence on what the diagram answers, shown beside the choice in the add dialog.
    pub description: String,
    /// A Material Design Icons class; empty for the generic diagram mark.
    pub icon: String,
    /// The extension, dot included, of the file that holds the body; empty when the registration is the whole
    /// diagram.
    pub extension: String,
    /// Whether the extension is too common to claim on sight (`.yml`): such a body becomes a diagram only when
    /// registered.
    pub shared_extension: bool,
    pub subject: DiagramSubject,
    /// A second serialisation's extension that routes through the same reading (`.nt` beside `.ttl`).
    pub alternate_extension: String,
}

impl DiagramDefinition {
    pub fn new(origin: impl Into<String>, title: impl Into<String>) -> Self {
        DiagramDefinition {
            origin: origin.into(),
            title: title.into(),
            ..DiagramDefinition::default()
        }
    }

    pub fn with_extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = extension.into();
        self
    }

    pub fn with_subject(mut self, subject: DiagramSubject) -> Self {
        self.subject = subject;
        self
    }

    /// Whether this tool type declares `extension`, its own or the alternate, ignoring case.
    pub fn declares_extension(&self, extension: &str) -> bool {
        !self.extension.is_empty()
            && (self.extension.eq_ignore_ascii_case(extension)
                || (!self.alternate_extension.is_empty() && self.alternate_extension.eq_ignore_ascii_case(extension)))
    }

    /// Whether the body lives in a file of its own rather than in the registration.
    pub fn has_document_sibling(&self) -> bool {
        !self.extension.is_empty()
    }

    /// Whether the diagram is a folder rather than a document.
    pub fn has_folder_subject(&self) -> bool {
        self.subject == DiagramSubject::Folder
    }

    /// Whether a file with the extension may be opened as this tool type without a registration beside it.
    pub fn routes_bare_body(&self) -> bool {
        self.has_document_sibling() && !self.shared_extension
    }

    /// Why the definition cannot be believed, when it cannot: a folder subject has no body file to name.
    pub fn contradiction(&self) -> Option<String> {
        (self.has_folder_subject() && self.has_document_sibling()).then(|| {
            format!(
                "{} declares a folder subject and the extension '{}', which cannot both be true",
                self.origin, self.extension
            )
        })
    }
}

/// Writes the body of a new, empty document of one tool type.
pub trait DiagramDocumentFactory {
    fn origin(&self) -> &str;

    /// The text of a new document whose file is called `base_name`.
    fn create_empty_document(&self, base_name: &str) -> String;
}

/// A factory made of what a new document is: a fixed text, or a text written from the file's base name.
pub struct EmptyDocumentFactory {
    origin: String,
    text: Box<dyn Fn(&str) -> String>,
}

impl EmptyDocumentFactory {
    /// A factory whose new document is `text`, whatever the file is called.
    pub fn fixed(origin: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        EmptyDocumentFactory {
            origin: origin.into(),
            text: Box::new(move |_| text.clone()),
        }
    }

    /// A factory whose new document `text` writes from the file's base name.
    pub fn from_base_name(origin: impl Into<String>, text: impl Fn(&str) -> String + 'static) -> Self {
        EmptyDocumentFactory {
            origin: origin.into(),
            text: Box::new(text),
        }
    }
}

impl DiagramDocumentFactory for EmptyDocumentFactory {
    fn origin(&self) -> &str {
        &self.origin
    }

    fn create_empty_document(&self, base_name: &str) -> String {
        (self.text)(base_name)
    }
}

/// The registered factories, by origin, and the start-up check that every definition needing one has one.
#[derive(Default)]
pub struct DiagramDocumentFactories {
    by_origin: HashMap<String, Box<dyn DiagramDocumentFactory>>,
}

impl DiagramDocumentFactories {
    pub fn new(factories: Vec<Box<dyn DiagramDocumentFactory>>) -> Self {
        DiagramDocumentFactories {
            by_origin: factories.into_iter().map(|f| (f.origin().to_owned(), f)).collect(),
        }
    }

    /// The factory for `origin`, or none when the tool type keeps no body file of its own.
    pub fn find(&self, origin: &str) -> Option<&dyn DiagramDocumentFactory> {
        self.by_origin.get(origin).map(|f| f.as_ref())
    }

    /// Every definition that declares an extension but has no factory to write its body: the deployment errors
    /// reported at start, so they cannot surface as a failed add later. Empty means consistent.
    pub fn verify<'a>(&self, definitions: &'a [DiagramDefinition]) -> Vec<&'a DiagramDefinition> {
        definitions
            .iter()
            .filter(|d| d.has_document_sibling() && !self.by_origin.contains_key(&d.origin))
            .collect()
    }
}

/// One toolbox entry a tool type contributes, as data the shell renders without understanding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolboxItem {
    /// Unique within its tool type.
    pub id: String,
    pub label: String,
    pub icon: String,
    /// What dropping it does.
    pub description: String,
    /// The action executed against what the entry is dropped on, so the drop, the menu and the keyboard share one
    /// implementation; empty when it names none.
    pub drop_action_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_keeps_the_origin_and_title_it_was_given() {
        let definition = DiagramDefinition::new("uml/class", "Class diagram");
        assert_eq!(
            (definition.origin.as_str(), definition.title.as_str()),
            ("uml/class", "Class diagram")
        );
        assert_eq!(definition.description, "");
    }

    #[test]
    fn without_an_extension_the_registration_is_the_whole_diagram() {
        let definition = DiagramDefinition::new("uml/class", "Class diagram");
        assert_eq!(definition.extension, "");
        assert!(!definition.has_document_sibling());
    }

    #[test]
    fn with_an_extension_the_body_is_a_file_of_its_own() {
        let definition = DiagramDefinition::new("freeplane/mindmap", "Mind map").with_extension(".mm");
        assert!(definition.has_document_sibling());
        assert!(definition.routes_bare_body());
        assert!(definition.declares_extension(".MM"));
    }

    #[test]
    fn without_a_subject_the_diagram_is_its_document() {
        let definition = DiagramDefinition::new("uml/class", "Class diagram");
        assert_eq!(definition.subject, DiagramSubject::Document);
        assert!(!definition.has_folder_subject());
    }

    #[test]
    fn with_a_folder_subject_the_diagram_is_the_folder_and_an_extension_contradicts_it() {
        let folder = DiagramDefinition::new("ansible/structure", "Ansible project structure")
            .with_subject(DiagramSubject::Folder);
        assert!(folder.has_folder_subject());
        assert!(!folder.has_document_sibling());
        assert!(folder.contradiction().is_none());
        let contradiction = folder.with_extension(".yml").contradiction().expect("a contradiction");
        assert!(contradiction.contains("cannot both be true"), "{contradiction}");
    }

    #[test]
    fn the_factory_registered_for_an_origin_is_found() {
        let factories =
            DiagramDocumentFactories::new(vec![Box::new(EmptyDocumentFactory::fixed("freeplane/mindmap", ""))]);
        assert!(factories.find("freeplane/mindmap").is_some());
        assert!(factories.find("uml/class").is_none());
    }

    #[test]
    fn verify_names_every_definition_with_an_extension_but_no_factory() {
        let factories = DiagramDocumentFactories::default();
        let definitions = [
            DiagramDefinition::new("freeplane/mindmap", "Mind map").with_extension(".mm"),
            DiagramDefinition::new("d2/diagram", "D2").with_extension(".d2"),
            DiagramDefinition::new("uml/class", "Class diagram"),
        ];
        let missing: Vec<&str> = factories
            .verify(&definitions)
            .iter()
            .map(|d| d.origin.as_str())
            .collect();
        assert_eq!(missing, ["freeplane/mindmap", "d2/diagram"]);
    }

    #[test]
    fn an_empty_document_is_its_text_or_written_from_the_base_name() {
        let fixed = EmptyDocumentFactory::fixed("etalii/test", "test: 1\r\n");
        assert_eq!(fixed.create_empty_document("first"), "test: 1\r\n");
        assert_eq!(fixed.create_empty_document("second"), "test: 1\r\n");
        let named = EmptyDocumentFactory::from_base_name("etalii/test", |base| format!("title: {base}\r\n"));
        assert_eq!(named.create_empty_document("first"), "title: first\r\n");
        assert_eq!(named.create_empty_document("second"), "title: second\r\n");
    }
}

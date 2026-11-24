//! The workspace's index of documents, and a link resolved against it (FBL sections 17.3 to 17.6).

use std::path::{Path, PathBuf};

use crate::link::Link;

/// The codes of a link's findings (FBL section 17.6).
pub mod codes {
    pub const MALFORMED: &str = "link.malformed";
    pub const BROKEN_DOCUMENT: &str = "link.broken-document";
    pub const BROKEN_ELEMENT: &str = "link.broken-element";
    pub const AMBIGUOUS_DOCUMENT: &str = "link.ambiguous-document";
}

/// An element of a document that can be linked to: its id, and its attributes of a text type by name, in
/// declaration order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndexedElement {
    pub id: String,
    pub attributes: Vec<(String, String)>,
}

impl IndexedElement {
    pub fn new(id: impl Into<String>, attributes: impl IntoIterator<Item = (String, String)>) -> IndexedElement {
        IndexedElement {
            id: id.into(),
            attributes: attributes.into_iter().collect(),
        }
    }

    /// The value of the attribute `name`, or none when the element has no such attribute.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
    }
}

/// A link a document holds, and the element or value of it that holds it, as its tool type names that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldLink {
    pub holder: String,
    pub text: String,
}

impl HeldLink {
    pub fn new(holder: impl Into<String>, text: impl Into<String>) -> HeldLink {
        HeldLink {
            holder: holder.into(),
            text: text.into(),
        }
    }
}

/// One document of the workspace as the index sees it (FBL section 17.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndexedDocument {
    pub path: PathBuf,
    /// Its document id; none for a body that holds none yet, which nothing resolves to until it is given one.
    pub document_id: Option<String>,
    /// The attribute its tool type names an element by (DISL's and DESL's `linkable`); none to use an element's
    /// first attribute.
    pub name_property: Option<String>,
    /// Its elements that can be linked to, in document order; two with one id are both kept, as a body may hold them.
    pub elements: Vec<IndexedElement>,
    /// The links it holds.
    pub links: Vec<HeldLink>,
}

impl IndexedDocument {
    /// The file's name, with its extension and without its folder.
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// What names `element`: the value of `name_prop` when the element has that attribute, else of the type's name
    /// property, else of its first attribute; and the attribute that was used, none for the type's own.
    fn name_of(&self, element: &IndexedElement, name_prop: Option<&str>) -> (Option<String>, String) {
        if let Some(prop) = name_prop
            && let Some(value) = element.attribute(prop)
        {
            return (Some(prop.to_owned()), value.to_owned());
        }
        let own = match self.name_property.as_deref() {
            Some(property) => element.attribute(property),
            None => element.attributes.first().map(|(_, v)| v.as_str()),
        }
        .unwrap_or_default();
        // A name property that names no attribute is replaced by the one used in its place (FBL section 17.4).
        let used = name_prop.and(self.name_property.clone());
        (used, own.to_owned())
    }
}

/// A link that resolved: its target document and, for a link to an element, that element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub link: Link,
    pub path: PathBuf,
    pub element: Option<IndexedElement>,
    /// The link as load-time validation writes it (FBL section 17.4, step 4); equal to what was read when nothing
    /// differs.
    pub refreshed: Link,
}

/// A link's finding (FBL section 17.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkFinding {
    pub code: &'static str,
    pub message: String,
    /// For `link.ambiguous-document`, the documents to choose from, by path.
    pub candidates: Vec<PathBuf>,
    /// The link as it was read, for every finding but `link.malformed`: it is shown by its cached name.
    pub link: Option<Link>,
}

impl LinkFinding {
    /// Whether the finding is an error (`link.malformed`) rather than a warning.
    pub fn is_error(&self) -> bool {
        self.code == codes::MALFORMED
    }
}

/// What resolving a link gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Resolved(Resolved),
    Finding(LinkFinding),
}

impl Resolution {
    pub fn resolved(&self) -> Option<&Resolved> {
        match self {
            Resolution::Resolved(r) => Some(r),
            Resolution::Finding(_) => None,
        }
    }

    pub fn finding(&self) -> Option<&LinkFinding> {
        match self {
            Resolution::Resolved(_) => None,
            Resolution::Finding(f) => Some(f),
        }
    }
}

/// A link, in some document of the workspace, that resolves to an element (FBL section 17.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlink {
    /// The consuming document.
    pub path: PathBuf,
    /// The element or value of it that holds the link.
    pub holder: String,
    pub link: Link,
}

/// The documents of one workspace root (FBL section 17.3), each as its tool type reads it.
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    root: PathBuf,
    documents: Vec<IndexedDocument>,
}

impl Workspace {
    /// A workspace of `root`, holding `documents`. They are kept in path order, so that candidates are listed alike
    /// whatever order they were found in.
    pub fn new(root: impl Into<PathBuf>, documents: impl IntoIterator<Item = IndexedDocument>) -> Workspace {
        let mut documents: Vec<IndexedDocument> = documents.into_iter().collect();
        documents.sort_by(|a, b| a.path.cmp(&b.path));
        Workspace {
            root: root.into(),
            documents,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn documents(&self) -> &[IndexedDocument] {
        &self.documents
    }

    pub fn document(&self, path: &Path) -> Option<&IndexedDocument> {
        self.documents.iter().find(|d| d.path == path)
    }

    /// Replaces the document at its path, or adds it: a document read again after it changed.
    pub fn put(&mut self, document: IndexedDocument) {
        match self.documents.iter().position(|d| d.path == document.path) {
            Some(at) => self.documents[at] = document,
            None => {
                let at = self.documents.partition_point(|d| d.path < document.path);
                self.documents.insert(at, document);
            }
        }
    }

    /// Takes the document at `path` out of the index: a file deleted, or moved out of the workspace.
    pub fn remove(&mut self, path: &Path) {
        self.documents.retain(|d| d.path != path);
    }

    /// A path as a finding names it: relative to the workspace's root, with forward slashes.
    pub fn display(&self, path: &Path) -> String {
        let relative = path.strip_prefix(&self.root).unwrap_or(path);
        relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Resolves the link `text` (FBL section 17.3): parsed; the target document by its id, else by its file name
    /// among the documents of that id or, when none has it, among all; then the target element by its id.
    pub fn resolve(&self, text: &str) -> Resolution {
        let Some(link) = Link::parse(text) else {
            return Resolution::Finding(LinkFinding {
                code: codes::MALFORMED,
                message: format!("'{text}' is not a link."),
                candidates: Vec::new(),
                link: None,
            });
        };
        let with_id: Vec<&IndexedDocument> = self
            .documents
            .iter()
            .filter(|d| d.document_id.as_deref() == Some(link.document_id.as_str()))
            .collect();
        let target = if with_id.len() == 1 {
            with_id[0]
        } else {
            let pool: Vec<&IndexedDocument> = if with_id.is_empty() {
                self.documents.iter().filter(|d| d.document_id.is_some()).collect()
            } else {
                with_id
            };
            let named: Vec<&IndexedDocument> = pool
                .into_iter()
                .filter(|d| d.file_name() == link.document_name)
                .collect();
            match named.as_slice() {
                [one] => *one,
                [] => {
                    return Resolution::Finding(LinkFinding {
                        code: codes::BROKEN_DOCUMENT,
                        message: format!(
                            "No document '{}' with id '{}' is in the workspace.",
                            link.document_name, link.document_id
                        ),
                        candidates: Vec::new(),
                        link: Some(link),
                    });
                }
                several => {
                    let candidates: Vec<PathBuf> = several.iter().map(|d| d.path.clone()).collect();
                    return Resolution::Finding(LinkFinding {
                        code: codes::AMBIGUOUS_DOCUMENT,
                        message: format!(
                            "{} documents are '{}' with id '{}': {}.",
                            candidates.len(),
                            link.document_name,
                            link.document_id,
                            candidates
                                .iter()
                                .map(|p| self.display(p))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                        candidates,
                        link: Some(link),
                    });
                }
            }
        };
        let Some(part) = &link.element else {
            let refreshed = Link::to_document(target.document_id.clone().unwrap_or_default(), target.file_name());
            return Resolution::Resolved(Resolved {
                path: target.path.clone(),
                element: None,
                refreshed,
                link,
            });
        };
        let holders: Vec<&IndexedElement> = target.elements.iter().filter(|e| e.id == part.id).collect();
        match holders.as_slice() {
            [] => Resolution::Finding(LinkFinding {
                code: codes::BROKEN_ELEMENT,
                message: format!(
                    "'{}' is no longer in '{}'.",
                    part.name.as_deref().unwrap_or(&part.id),
                    target.file_name()
                ),
                candidates: Vec::new(),
                link: Some(link),
            }),
            [element] => {
                let (name_prop, name) = target.name_of(element, part.name_prop.as_deref());
                let refreshed = Link::to_element(
                    target.document_id.clone().unwrap_or_default(),
                    target.file_name(),
                    part.id.clone(),
                    name_prop,
                    Some(name),
                );
                Resolution::Resolved(Resolved {
                    path: target.path.clone(),
                    element: Some((*element).clone()),
                    refreshed,
                    link,
                })
            }
            several => Resolution::Finding(LinkFinding {
                code: codes::AMBIGUOUS_DOCUMENT,
                message: format!(
                    "'{}' holds {} elements with id '{}'.",
                    target.file_name(),
                    several.len(),
                    part.id
                ),
                candidates: vec![target.path.clone()],
                link: Some(link),
            }),
        }
    }

    /// The backlinks of the element `element_id` of the document at `path` (FBL section 17.5): every link of the
    /// workspace that resolves to it, in path order and then in the order each document holds them. Computed, never
    /// stored.
    pub fn backlinks(&self, path: &Path, element_id: &str) -> Vec<Backlink> {
        self.documents
            .iter()
            .flat_map(|document| {
                document.links.iter().filter_map(move |held| {
                    let resolved = self.resolve(&held.text);
                    let resolved = resolved.resolved()?;
                    (resolved.path == path && resolved.link.element_id() == Some(element_id)).then(|| Backlink {
                        path: document.path.clone(),
                        holder: held.holder.clone(),
                        link: resolved.link.clone(),
                    })
                })
            })
            .collect()
    }

    /// How many links of the workspace a delete of these elements of the document at `path` breaks (FBL section
    /// 17.5): the links to them from other documents and from this one.
    pub fn broken_by_deleting(&self, path: &Path, element_ids: &[&str]) -> usize {
        element_ids.iter().map(|id| self.backlinks(path, id).len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(path: &str, id: Option<&str>, elements: &[(&str, &str)]) -> IndexedDocument {
        IndexedDocument {
            path: PathBuf::from("/w").join(path),
            document_id: id.map(str::to_owned),
            name_property: Some("label".into()),
            elements: elements
                .iter()
                .map(|(id, label)| IndexedElement::new(*id, [("label".to_owned(), (*label).to_owned())]))
                .collect(),
            links: Vec::new(),
        }
    }

    fn workspace() -> Workspace {
        Workspace::new(
            "/w",
            [
                document(
                    "a/plan.tml",
                    Some("aaaaaaaa"),
                    &[("x", "Ex"), ("d", "One"), ("d", "Two")],
                ),
                document("b/plan.tml", Some("aaaaaaaa"), &[("x", "Copy")]),
                document("c/other.tml", Some("cccccccc"), &[("y", "Why")]),
                document("c/new.tml", None, &[("z", "Zed")]),
            ],
        )
    }

    #[test]
    fn a_unique_id_resolves_whatever_name_the_link_has() {
        let resolution = workspace().resolve("$cccccccc@old.tml#y.label=Old");
        let resolved = resolution.resolved().expect("resolved");
        assert_eq!(PathBuf::from("/w/c/other.tml"), resolved.path);
        assert_eq!("$cccccccc@other.tml#y.label=Why", resolved.refreshed.text());
    }

    #[test]
    fn copies_with_one_id_are_told_apart_by_name_or_reported() {
        let w = workspace();
        let finding = w.resolve("$aaaaaaaa@plan.tml#x").finding().cloned().expect("a finding");
        assert_eq!(codes::AMBIGUOUS_DOCUMENT, finding.code);
        assert_eq!(
            "2 documents are 'plan.tml' with id 'aaaaaaaa': a/plan.tml, b/plan.tml.",
            finding.message
        );
        assert_eq!(2, finding.candidates.len());
    }

    #[test]
    fn a_document_without_an_id_is_not_a_target() {
        let finding = workspace()
            .resolve("$zzzzzzzz@new.tml#z")
            .finding()
            .cloned()
            .expect("a finding");
        assert_eq!(codes::BROKEN_DOCUMENT, finding.code);
        assert_eq!(
            "No document 'new.tml' with id 'zzzzzzzz' is in the workspace.",
            finding.message
        );
    }

    #[test]
    fn a_missing_and_a_duplicate_element_are_reported() {
        let w = workspace();
        let missing = w.resolve("$cccccccc@other.tml#q=Gone").finding().cloned().unwrap();
        assert_eq!(
            (codes::BROKEN_ELEMENT, "'Gone' is no longer in 'other.tml'."),
            (missing.code, missing.message.as_str())
        );
        let unnamed = w.resolve("$cccccccc@other.tml#q").finding().cloned().unwrap();
        assert_eq!("'q' is no longer in 'other.tml'.", unnamed.message);
        let twice = w.resolve("$aaaaaaaa@missing.tml#d");
        let twice = twice.finding().expect("a finding");
        assert_eq!(codes::BROKEN_DOCUMENT, twice.code);
        let w = Workspace::new(
            "/w",
            [document("a/plan.tml", Some("aaaaaaaa"), &[("d", "One"), ("d", "Two")])],
        );
        let twice = w.resolve("$aaaaaaaa@plan.tml#d").finding().cloned().unwrap();
        assert_eq!(
            (codes::AMBIGUOUS_DOCUMENT, "'plan.tml' holds 2 elements with id 'd'."),
            (twice.code, twice.message.as_str())
        );
    }

    #[test]
    fn a_malformed_link_is_an_error() {
        let finding = workspace().resolve("plan.tml#x").finding().cloned().unwrap();
        assert!(finding.is_error());
        assert_eq!("'plan.tml#x' is not a link.", finding.message);
    }

    #[test]
    fn a_name_property_the_target_has_not_is_replaced_and_a_missing_name_filled_in() {
        let w = workspace();
        let r = w.resolve("$cccccccc@other.tml#y.text=Why");
        assert_eq!(
            "$cccccccc@other.tml#y.label=Why",
            r.resolved().unwrap().refreshed.text()
        );
        let r = w.resolve("$cccccccc@other.tml#y");
        assert_eq!("$cccccccc@other.tml#y=Why", r.resolved().unwrap().refreshed.text());
        let r = w.resolve("$cccccccc@x.tml");
        assert_eq!("$cccccccc@other.tml", r.resolved().unwrap().refreshed.text());
    }

    #[test]
    fn backlinks_are_the_links_that_resolve_to_the_element() {
        let mut w = workspace();
        let mut consumer = document("d/map.tml", Some("dddddddd"), &[("m", "Em")]);
        consumer.links = vec![
            HeldLink::new("m", "$cccccccc@other.tml#y=Why"),
            HeldLink::new("n", "$cccccccc@renamed.tml#y=Why"),
            HeldLink::new("o", "$cccccccc@other.tml#gone"),
        ];
        w.put(consumer);
        let back = w.backlinks(Path::new("/w/c/other.tml"), "y");
        assert_eq!(
            vec!["m", "n"],
            back.iter().map(|b| b.holder.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(2, w.broken_by_deleting(Path::new("/w/c/other.tml"), &["y"]));
        w.remove(Path::new("/w/d/map.tml"));
        assert!(w.backlinks(Path::new("/w/c/other.tml"), "y").is_empty());
    }
}

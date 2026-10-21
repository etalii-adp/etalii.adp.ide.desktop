//! A parsed `.mm` file (etalii.adp spec 030, FR-031 and FR-032), ported from the standalone host's
//! `src/diagrams/mindmap/backend/EtAlii.Adp.Diagram.Mindmap` at `8b6ccfc`: `MindmapDocument.cs`, `_Model/MindmapNode.cs`
//! and `FreeplaneNewline.cs`.
//!
//! The tree of nodes is a view over the XML it came from ([`crate::xml`]): reading goes to the elements and every edit
//! changes them, and the text is the XML written back in Freeplane's own layout, so a map nothing changed comes back
//! byte for byte and a changed one differs only where it was changed (mindmap Requirements 3.2 and 3.3). A node is an
//! element by its arena id, so it stays the same node while its branch is detached and put back.

use crate::xml::{NodeId, XmlDocument, XmlError};

const MAP_ELEMENT: &str = "map";
pub(crate) const NODE_ELEMENT: &str = "node";
const ID_ATTRIBUTE: &str = "ID";
const TEXT_ATTRIBUTE: &str = "TEXT";
const LINK_ATTRIBUTE: &str = "LINK";
const FOLDED_ATTRIBUTE: &str = "FOLDED";
const POSITION_ATTRIBUTE: &str = "POSITION";
const RICH_CONTENT_ELEMENT: &str = "richcontent";
const RICH_CONTENT_TYPE_ATTRIBUTE: &str = "TYPE";
const NOTE_TYPE: &str = "NOTE";
const NODE_TEXT_TYPE: &str = "NODE";

/// The version a map ADP creates declares: the newest Freeplane format the reader was written against.
const FORMAT_VERSION: &str = "freeplane 1.11.5";

/// A text that is not a Freeplane map, with what to tell the person; it never takes more than its one diagram down
/// (mindmap Requirement 3.8). `line` is the 1-based line the XML reader stopped at, for text that is not well-formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MindmapFormatError {
    pub message: String,
    pub line: Option<usize>,
}

impl std::fmt::Display for MindmapFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// What reading a text gives when it must not fail: the map, or an empty one with the reason and the 0-based line.
#[derive(Debug, Clone)]
pub struct MindmapOutcome {
    pub document: MindmapDocument,
    /// Why the text is not a map; none when it is one.
    pub unreadable: Option<String>,
    /// The 0-based line of the problem, with the reason.
    pub problems: Vec<(usize, String)>,
}

impl MindmapOutcome {
    pub fn is_usable(&self) -> bool {
        self.unreadable.is_none()
    }
}

/// A Freeplane map: its nodes, and behind them the XML they were read from.
#[derive(Debug, Clone)]
pub struct MindmapDocument {
    xml: XmlDocument,
    map: NodeId,
    /// What followed `</map>`, exactly as read.
    tail: String,
}

impl MindmapDocument {
    /// Parses `text`. With `assign_missing_ids` every node without an `ID` is given one in memory, written on the next
    /// save and never on open (mindmap Requirement 3.4).
    pub fn parse(text: &str, assign_missing_ids: bool) -> Result<MindmapDocument, MindmapFormatError> {
        let content = text.trim_end_matches(['\r', '\n', ' ', '\t']);
        let tail = text[content.len()..].to_owned();
        // XML obliges a reader to turn "\r\n" into "\n". Escaping the carriage return first keeps it as a literal one
        // in the text it belongs to, so a Windows save (CRLF structure, LF inside its rich content) comes back byte
        // for byte; the escape keeps one '\n' per line break, so the lines are the file's own.
        let xml =
            XmlDocument::parse(&content.replace("\r\n", "&#xD;\n")).map_err(|e: XmlError| MindmapFormatError {
                message: format!("The map is not well-formed XML: {e}"),
                line: Some(e.line),
            })?;
        let map = xml
            .root()
            .filter(|root| xml.local_name(*root) == MAP_ELEMENT)
            .ok_or_else(|| MindmapFormatError {
                message: "The file does not start with a <map> element.".into(),
                line: None,
            })?;
        let roots = xml.elements_named(map, NODE_ELEMENT).count();
        if roots != 1 {
            return Err(MindmapFormatError {
                message: format!("A map has exactly one root node; this one has {roots}."),
                line: None,
            });
        }
        let mut document = MindmapDocument { xml, map, tail };
        if assign_missing_ids {
            document.assign_missing_ids();
        }
        Ok(document)
    }

    /// [`MindmapDocument::parse`] that never fails: a text that is not a map is an empty map, with the reason and the
    /// 0-based line the reader stopped at.
    pub fn outcome(text: &str, assign_missing_ids: bool) -> MindmapOutcome {
        match Self::parse(text, assign_missing_ids) {
            Ok(document) => MindmapOutcome {
                document,
                unreadable: None,
                problems: Vec::new(),
            },
            Err(error) => {
                let line = error.line.map_or(0, |l| l.max(1) - 1);
                let document = Self::parse(&Self::empty_document(""), assign_missing_ids).expect("an empty map parses");
                MindmapOutcome {
                    document,
                    problems: vec![(line, error.message.clone())],
                    unreadable: Some(error.message),
                }
            }
        }
    }

    /// The text of the map a new diagram starts as: one root node named after the file (mindmap Requirement 1.4).
    pub fn empty_document(base_name: &str) -> String {
        let mut xml = XmlDocument::new();
        let root = xml.new_element(
            NODE_ELEMENT,
            vec![
                (TEXT_ATTRIBUTE.into(), base_name.to_owned()),
                (ID_ATTRIBUTE.into(), new_id()),
            ],
        );
        let map = xml.new_element(MAP_ELEMENT, vec![("version".into(), FORMAT_VERSION.into())]);
        let before = xml.new_text("\n");
        let after = xml.new_text("\n");
        xml.append(map, before);
        xml.append(map, root);
        xml.append(map, after);
        xml.push_top(map);
        xml.write()
    }

    /// The document as Freeplane would write it, with what followed `</map>` as it was read.
    pub fn to_text(&self) -> String {
        let mut text = self.xml.write();
        text.push_str(&self.tail);
        text
    }

    pub fn xml(&self) -> &XmlDocument {
        &self.xml
    }

    /// The one root node.
    pub fn root(&self) -> NodeId {
        self.xml
            .elements_named(self.map, NODE_ELEMENT)
            .next()
            .expect("a map has one root node")
    }

    /// Every node, root first, in document order.
    pub fn nodes(&self) -> Vec<NodeId> {
        self.xml.descendants_named(self.map, NODE_ELEMENT)
    }

    /// The first node with `id`.
    pub fn find(&self, id: &str) -> Option<NodeId> {
        self.nodes()
            .into_iter()
            .find(|n| self.xml.attribute(*n, ID_ATTRIBUTE) == Some(id))
    }

    // ---- a node ---------------------------------------------------------------------------------------------------

    /// The node's `ID`; empty when it has none.
    pub fn id(&self, node: NodeId) -> String {
        self.xml.attribute(node, ID_ATTRIBUTE).unwrap_or_default().to_owned()
    }

    /// The node's text: the plain text of a `richcontent TYPE="NODE"`, else its `TEXT`.
    pub fn text(&self, node: NodeId) -> String {
        match self.rich_content(node, NODE_TEXT_TYPE) {
            Some(rich) => self.plain_text_of(rich),
            None => self.xml.attribute(node, TEXT_ATTRIBUTE).unwrap_or_default().to_owned(),
        }
    }

    /// The node's notes, from its `richcontent TYPE="NOTE"`; empty when it has none.
    pub fn notes(&self, node: NodeId) -> String {
        self.rich_content(node, NOTE_TYPE)
            .map(|rich| self.plain_text_of(rich))
            .unwrap_or_default()
    }

    /// Whether the file says the branch starts folded.
    pub fn folded(&self, node: NodeId) -> bool {
        self.xml
            .attribute(node, FOLDED_ATTRIBUTE)
            .is_some_and(|f| f.eq_ignore_ascii_case("true"))
    }

    /// The `LINK` exactly as stored, or none when the node is not linked.
    pub fn link(&self, node: NodeId) -> Option<&str> {
        self.xml.attribute(node, LINK_ATTRIBUTE)
    }

    /// Freeplane's `POSITION` on a first-level node, when the file has one.
    pub fn position(&self, node: NodeId) -> Option<&str> {
        self.xml.attribute(node, POSITION_ATTRIBUTE)
    }

    /// The node's parent node; none for the root.
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.xml
            .parent(node)
            .filter(|p| self.xml.is_element(*p) && self.xml.name(*p) == NODE_ELEMENT)
    }

    pub fn children(&self, node: NodeId) -> Vec<NodeId> {
        self.xml.elements_named(node, NODE_ELEMENT).collect()
    }

    pub fn has_children(&self, node: NodeId) -> bool {
        self.xml.elements_named(node, NODE_ELEMENT).next().is_some()
    }

    pub fn is_root(&self, node: NodeId) -> bool {
        self.parent(node).is_none()
    }

    /// The node's place among its parent's children; 0 for the root.
    pub fn index_in_parent(&self, node: NodeId) -> usize {
        match self.xml.parent(node) {
            Some(parent) => self
                .xml
                .elements_named(parent, NODE_ELEMENT)
                .take_while(|n| *n != node)
                .count(),
            None => 0,
        }
    }

    /// Whether `node` is `ancestor` or lies beneath it.
    pub fn is_within(&self, node: NodeId, ancestor: NodeId) -> bool {
        let mut current = Some(node);
        while let Some(at) = current {
            if at == ancestor {
                return true;
            }
            current = self.xml.parent(at);
        }
        false
    }

    // ---- structural edits -----------------------------------------------------------------------------------------

    /// Appends a new node named `text` as the last child of `parent`.
    pub fn add_child(&mut self, parent: NodeId, text: &str) -> NodeId {
        let element = self.new_node_element(text);
        self.append_child(parent, element);
        element
    }

    /// Puts a new node named `text` right after `sibling`, under the same parent.
    pub fn add_sibling(&mut self, sibling: NodeId, text: &str) -> Result<NodeId, String> {
        if self.is_root(sibling) {
            return Err("The root has no siblings.".into());
        }
        let element = self.new_node_element(text);
        let newline = self.xml.new_text(self.newline());
        self.xml.insert_after(sibling, &[newline, element]);
        Ok(element)
    }

    /// Moves `node` and its branch to be child `index` of `new_parent`, counted once it has left its place; a negative
    /// index or one past the children appends.
    pub fn move_node(&mut self, node: NodeId, new_parent: NodeId, index: i64) -> Result<(), String> {
        if self.is_root(node) {
            return Err("The root cannot be moved.".into());
        }
        if self.is_within(new_parent, node) {
            return Err("A node cannot be moved into its own subtree.".into());
        }
        self.detach(node);
        self.insert_at(new_parent, node, index);
        Ok(())
    }

    /// Removes `node` with its branch; the branch stays in the arena, for [`MindmapDocument::restore`].
    pub fn remove(&mut self, node: NodeId) -> Result<NodeId, String> {
        if self.is_root(node) {
            return Err("The root cannot be removed.".into());
        }
        self.detach(node);
        Ok(node)
    }

    /// Puts a branch [`MindmapDocument::remove`] took back under `parent` at `index`.
    pub fn restore(&mut self, subtree: NodeId, parent: NodeId, index: i64) {
        self.insert_at(parent, subtree, index);
    }

    /// Sets the node's text as plain `TEXT`, dropping formatted text, the empty string included.
    pub fn set_text(&mut self, node: NodeId, text: &str) {
        if let Some(rich) = self.rich_content(node, NODE_TEXT_TYPE) {
            self.xml.remove(rich);
        }
        self.xml.set_attribute(node, TEXT_ATTRIBUTE, Some(text));
    }

    /// Sets the node's notes, one `<p>` per line; empty removes them.
    pub fn set_notes(&mut self, node: NodeId, notes: &str) {
        let existing = self.rich_content(node, NOTE_TYPE);
        if notes.is_empty() {
            if let Some(existing) = existing {
                // With the newline that followed it, so a node whose only content was the note closes itself again.
                if let Some(next) = self.xml.next_sibling(existing).filter(|n| self.xml.is_whitespace(*n)) {
                    self.xml.remove(next);
                }
                self.xml.remove(existing);
                if self.xml.children(node).iter().all(|c| self.xml.is_whitespace(*c)) {
                    self.xml.remove_children(node);
                }
            }
            return;
        }
        let html = self.notes_html(notes);
        match existing {
            Some(existing) => {
                self.xml.remove_children(existing);
                self.xml.append(existing, html);
            }
            None => {
                let content = self.xml.new_element(
                    RICH_CONTENT_ELEMENT,
                    vec![(RICH_CONTENT_TYPE_ATTRIBUTE.into(), NOTE_TYPE.into())],
                );
                self.xml.append(content, html);
                let newline = self.newline();
                let first_child = self.xml.elements_named(node, NODE_ELEMENT).next();
                if let Some(first_child) = first_child {
                    let after = self.xml.new_text(newline);
                    self.xml.insert_before(first_child, &[content, after]);
                } else if self.ends_with_line_break(node) {
                    let after = self.xml.new_text(newline);
                    self.xml.append(node, content);
                    self.xml.append(node, after);
                } else {
                    let before = self.xml.new_text(newline);
                    let after = self.xml.new_text(newline);
                    self.xml.append(node, before);
                    self.xml.append(node, content);
                    self.xml.append(node, after);
                }
            }
        }
    }

    /// Sets the node's `LINK`; none unlinks.
    pub fn set_link(&mut self, node: NodeId, link: Option<&str>) {
        self.xml.set_attribute(node, LINK_ATTRIBUTE, link);
    }

    pub fn set_id(&mut self, node: NodeId, id: &str) {
        self.xml.set_attribute(node, ID_ATTRIBUTE, Some(id));
    }

    // ---- helpers --------------------------------------------------------------------------------------------------

    /// Gives every node without an `ID` a new one.
    pub fn assign_missing_ids(&mut self) {
        for node in self.nodes() {
            if self.id(node).is_empty() {
                self.set_id(node, &new_id());
            }
        }
    }

    fn new_node_element(&mut self, text: &str) -> NodeId {
        self.xml.new_element(
            NODE_ELEMENT,
            vec![
                (TEXT_ATTRIBUTE.into(), text.to_owned()),
                (ID_ATTRIBUTE.into(), new_id()),
            ],
        )
    }

    fn notes_html(&mut self, notes: &str) -> NodeId {
        let body = self.xml.new_element("body", Vec::new());
        for line in notes.split('\n') {
            let paragraph = self.xml.new_element("p", Vec::new());
            let text = self.xml.new_text(line);
            self.xml.append(paragraph, text);
            self.xml.append(body, paragraph);
        }
        let html = self.xml.new_element("html", Vec::new());
        let head = self.xml.new_element("head", Vec::new());
        self.xml.append(html, head);
        self.xml.append(html, body);
        html
    }

    /// Removes an element with the newline that follows it, so the siblings left keep one node per line.
    fn detach(&mut self, element: NodeId) {
        if let Some(next) = self.xml.next_sibling(element).filter(|n| self.xml.is_whitespace(*n)) {
            self.xml.remove(next);
        }
        self.xml.remove(element);
    }

    fn insert_at(&mut self, parent: NodeId, element: NodeId, index: i64) {
        let children = self.children(parent);
        match usize::try_from(index).ok().and_then(|i| children.get(i).copied()) {
            Some(before) => {
                let newline = self.xml.new_text(self.newline());
                self.xml.insert_before(before, &[element, newline]);
            }
            None => self.append_child(parent, element),
        }
    }

    /// Appends a child the way Freeplane lays children out: one per line, each followed by a newline.
    fn append_child(&mut self, parent: NodeId, element: NodeId) {
        let newline = self.newline();
        if self.ends_with_line_break(parent) {
            let after = self.xml.new_text(newline);
            self.xml.append(parent, element);
            self.xml.append(parent, after);
        } else {
            let before = self.xml.new_text(newline);
            let after = self.xml.new_text(newline);
            self.xml.append(parent, before);
            self.xml.append(parent, element);
            self.xml.append(parent, after);
        }
    }

    /// Whether the element's last node is whitespace holding a line break.
    fn ends_with_line_break(&self, element: NodeId) -> bool {
        self.xml
            .last_child(element)
            .and_then(|last| self.xml.text(last))
            .is_some_and(|t| t.chars().all(char::is_whitespace) && t.contains('\n'))
    }

    /// The newline the map uses between its elements: CRLF when the first line break under `<map>` is one, else LF
    /// (`FreeplaneNewline.Of`).
    fn newline(&self) -> &'static str {
        let first = self
            .xml
            .children(self.map)
            .iter()
            .filter_map(|c| self.xml.text(*c))
            .find(|t| t.contains('\n'));
        if first.is_some_and(|t| t.contains("\r\n")) {
            "\r\n"
        } else {
            "\n"
        }
    }

    fn rich_content(&self, node: NodeId, kind: &str) -> Option<NodeId> {
        self.xml.elements_named(node, RICH_CONTENT_ELEMENT).find(|c| {
            self.xml
                .attribute(*c, RICH_CONTENT_TYPE_ATTRIBUTE)
                .is_some_and(|t| t.eq_ignore_ascii_case(kind))
        })
    }

    /// The readable text of an HTML body: one line per paragraph, markup dropped, whitespace tidied.
    fn plain_text_of(&self, rich: NodeId) -> String {
        let body = self
            .xml
            .descendant_elements(rich)
            .into_iter()
            .find(|e| self.xml.local_name(*e) == "body")
            .unwrap_or(rich);
        let paragraphs: Vec<NodeId> = self
            .xml
            .children(body)
            .iter()
            .copied()
            .filter(|c| self.xml.is_element(*c) && self.xml.local_name(*c) == "p")
            .collect();
        if paragraphs.is_empty() {
            return self.collapse(body);
        }
        paragraphs
            .iter()
            .map(|p| self.collapse(*p))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn collapse(&self, element: NodeId) -> String {
        self.xml.value(element).split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// A new node's id: `ID_` and a ShortGuid, a version 4 GUID as 25 characters of base 36 (`persistence.ids`).
pub fn new_id() -> String {
    format!("ID_{}", short_guid())
}

fn short_guid() -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut value = uuid::Uuid::new_v4().as_u128();
    let mut digits = [b'0'; 25];
    for digit in digits.iter_mut().rev() {
        *digit = DIGITS[(value % 36) as usize];
        value /= 36;
    }
    String::from_utf8(digits.to_vec()).expect("base 36 digits")
}

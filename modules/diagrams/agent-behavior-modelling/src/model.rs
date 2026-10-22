//! What the parser reads from one Markdown file: the nodes of the behavior tree, each with the lines it occupies
//! (etalii.adp spec 030, task T074).
//!
//! Ported from the standalone host's `_Model/AbmModel.cs` and `_Model/AbmNode.cs` at `8b6ccfc`. A node's id is its
//! place in the tree, `1` for the first root and `1.2` for that root's second child: a place rather than a name, so a
//! rename keeps the selection and the position it was dragged to, and nothing is written into the Markdown to name it.

use std::collections::HashMap;

use crate::kinds::{self, Category};
use crate::lines::LineRange;

/// The ids of what the reader and the rules report, one per rule (`AbmValidator.cs`, `AbmRuleIds`).
pub mod rule_ids {
    /// An item without a keyword, read as a Do.
    pub const NO_KEYWORD: &str = "abm.no-keyword";
    /// A Do, Check, Ask the user or Delegate with children.
    pub const LEAF_WITH_CHILDREN: &str = "abm.leaf-with-children";
    /// A Retry, Repeat until, Only while or Ask approval before without exactly one child.
    pub const DECORATOR_CHILDREN: &str = "abm.decorator-children";
    /// A Do in order, Try in order or Do together with no children.
    pub const EMPTY_COMPOSITE: &str = "abm.empty-composite";
    /// More than one root.
    pub const SEVERAL_ROOTS: &str = "abm.several-roots";
    /// A Retry that allows no attempt.
    pub const NO_ATTEMPTS: &str = "abm.no-attempts";
    /// A file with no Behavior section, or one with no list in it.
    pub const NO_BEHAVIOR: &str = "abm.no-behavior";
}

/// Something the parser has to say about a document, on a zero-based line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub rule_id: String,
    pub message: String,
    pub line: usize,
}

/// One node of the tree, as it was read from one list item and the lines under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Its place in the tree.
    pub id: String,
    /// One of the kinds' ids.
    pub kind: String,
    /// The text after the keyword.
    pub label: String,
    /// A Retry's attempts; zero for every other kind.
    pub retry_count: i32,
    /// The plain lines under the item, without their indentation; empty when there are none.
    pub notes: String,
    /// Whether the item carried a keyword; an item without one is read as a Do.
    pub has_keyword: bool,
    /// The item's own line.
    pub line: usize,
    /// The notes' lines, or none when there are none.
    pub notes_range: Option<LineRange>,
    /// The last line that belongs to this node: its notes, or its last descendant's.
    pub subtree_end: usize,
    /// The column the item's marker sits at.
    pub indent: usize,
    /// The column the item's text starts at, where its notes are written.
    pub content_indent: usize,
    /// The list marker: `-`, `*` or `+`.
    pub marker: char,
    /// The keyword exactly as written, so a rename keeps the author's own spelling.
    pub keyword_text: String,
    pub parent_id: Option<String>,
    pub child_ids: Vec<String>,
}

impl Node {
    /// What the node may hold beneath it.
    pub fn category(&self) -> Category {
        kinds::of(&self.kind).category
    }

    /// Whether another child may be added beneath it.
    pub fn takes_another_child(&self) -> bool {
        match self.category() {
            Category::Composite => true,
            Category::Decorator => self.child_ids.is_empty(),
            Category::Leaf => false,
        }
    }

    /// Every line that belongs to this node, its own and its descendants'.
    pub fn subtree(&self) -> LineRange {
        LineRange::new(self.line, self.subtree_end)
    }

    /// The keyword as the canvas shows it: the canonical one, with a Retry's count.
    pub fn keyword(&self) -> String {
        kinds::keyword_of(&self.kind, self.retry_count)
    }
}

/// What the parser read from one Markdown file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Model {
    /// Every node, in document order, which is also depth-first order.
    pub nodes: Vec<Node>,
    /// The Behavior heading's line; none when the file has none.
    pub section_line: Option<usize>,
    /// The last line of the Behavior section; meaningless without a heading.
    pub section_end: usize,
    /// The column the tree's root items sit at.
    pub list_indent: usize,
    /// What the parser passed over, with the line it was on.
    pub problems: Vec<Problem>,
    by_id: HashMap<String, usize>,
}

impl Model {
    pub fn new(
        nodes: Vec<Node>,
        section_line: Option<usize>,
        section_end: usize,
        list_indent: usize,
        problems: Vec<Problem>,
    ) -> Model {
        let by_id = nodes.iter().enumerate().map(|(i, n)| (n.id.clone(), i)).collect();
        Model {
            nodes,
            section_line,
            section_end,
            list_indent,
            problems,
            by_id,
        }
    }

    /// The nodes with no parent, in order.
    pub fn roots(&self) -> Vec<&Node> {
        self.nodes.iter().filter(|n| n.parent_id.is_none()).collect()
    }

    /// The node with `id`.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.by_id.get(id).map(|i| &self.nodes[*i])
    }

    /// The children of `node`, in order.
    pub fn children_of(&self, node: &Node) -> Vec<&Node> {
        node.child_ids.iter().filter_map(|id| self.node(id)).collect()
    }

    /// `node`'s siblings, itself included, in order: its parent's children, or the roots.
    pub fn siblings_of(&self, node: &Node) -> Vec<&Node> {
        match node.parent_id.as_deref().and_then(|p| self.node(p)) {
            Some(parent) => self.children_of(parent),
            None => self.roots(),
        }
    }

    /// Whether `candidate` is `node` or lies beneath it.
    pub fn is_within(candidate: &str, node: &str) -> bool {
        candidate == node || candidate.strip_prefix(node).is_some_and(|rest| rest.starts_with('.'))
    }
}

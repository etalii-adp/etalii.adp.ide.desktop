//! One family's lossless reading of a body (FBL section 4) and the splices it writes (FBL section 6), the
//! standalone host's `Rules/FamilyReader` and its five readers. The engine ([`crate::rules::reading`]) does what is
//! the same for every family: rule precedence, ids, references, containment, findings and refusals; the planner
//! does the shape of an edit; a family reads its format into entries and writes their values back.
//!
//! Every entry of a reading lives in one arena, [`Family::entries`], and is named by its index there: a yaml or
//! json member or item, an xml element, or a lines or blocks statement. What a family knows besides is its own
//! kind of [`FamilyKind`].

pub(crate) mod bullets;
pub(crate) mod grammar;
pub(crate) mod json;
pub(crate) mod lines;
pub(crate) mod tree;
pub(crate) mod xml;
pub(crate) mod yaml;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use adp_cel::{BoundedRegex, Value};

use crate::documents::types::{
    AttributeBinding, BlockRule, Family as FamilyName, FblBinding, HeaderSettings, Rule, Slot,
};
use crate::model::{FblOptions, Finding, FindingSeverity, SourceLocation};
use crate::planning::plan::{Plan, Planning, refuse};
use crate::rules::{Candidate, InsertRequest, ReadElement, SlotChange, SlotRead};
use crate::splice::Operation;
use crate::text::{BodyText, Span};

pub(crate) type EntryId = usize;

/// A node a rule can match (FBL section 4.1.1): its own span, its line span when it starts and ends its lines, and
/// the entries it contains.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub own: Span,
    /// The line span (FBL section 4.1.1), leading comments included; none when the entry shares a line.
    pub line_span: Option<Span>,
    pub parent: Option<EntryId>,
    pub children: Vec<EntryId>,
    /// The key of a member, the name of an xml element; none for a sequence item or a statement.
    pub name: Option<String>,
    /// The byte column of the entry's first byte on its line.
    pub indent: usize,
    pub kind: EntryKind,
}

impl Entry {
    /// The span a removal takes: the line span when there is one, else the own span.
    pub fn removal_span(&self) -> Span {
        self.line_span.unwrap_or(self.own)
    }

    pub fn tree(&self) -> &tree::TreeEntry {
        match &self.kind {
            EntryKind::Tree(t) => t,
            _ => panic!("the entry is not a yaml or json entry"),
        }
    }

    pub fn xml(&self) -> &xml::XmlElement {
        match &self.kind {
            EntryKind::Xml(x) => x,
            _ => panic!("the entry is not an xml element"),
        }
    }

    pub fn statement(&self) -> &lines::Statement {
        match &self.kind {
            EntryKind::Statement(s) => s,
            _ => panic!("the entry is not a statement"),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum EntryKind {
    Tree(tree::TreeEntry),
    // Boxed: an xml element or a statement is several times a tree entry, and every entry of a body is kept.
    Xml(Box<xml::XmlElement>),
    Statement(Box<lines::Statement>),
    /// *(FBL 0.9)* A node or token of a `grammar` body's tree.
    Grammar(Box<grammar::GrammarEntry>),
}

/// What each family knows besides its entries.
#[derive(Debug)]
pub(crate) enum FamilyKind {
    Yaml(tree::TreeData),
    Json(tree::TreeData),
    Xml(xml::XmlData),
    Lines(lines::LinesData),
    Grammar(grammar::GrammarData),
}

/// One family's reading of one body.
#[derive(Debug)]
pub(crate) struct Family {
    pub text: BodyText,
    pub binding: Rc<FblBinding>,
    pub options: Rc<FblOptions>,
    /// Every entry, by its id.
    pub entries: Vec<Entry>,
    /// Every entry in document order (FBL section 4.1.3).
    pub order: Vec<EntryId>,
    /// The leaf nodes of the reading (keys, scalars, statements, tags, comments): every byte outside them is
    /// trivia (FBL section 4.1).
    pub leaves: Vec<Span>,
    pub findings: RefCell<Vec<Finding>>,
    /// Where and why the body is unreadable (FBL section 7.5).
    pub unreadable: Option<(usize, String)>,
    regexes: RefCell<HashMap<String, Rc<BoundedRegex>>>,
    pub kind: FamilyKind,
}

impl Family {
    /// The family a binding reads `options.file_name` with: the body's, or one of `alsoRead` its extension names.
    pub fn create(text: BodyText, binding: Rc<FblBinding>, options: Rc<FblOptions>) -> Family {
        let mut family = binding.body.family.unwrap_or(FamilyName::Lines);
        let extension = std::path::Path::new(&options.file_name)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy().to_lowercase()))
            .unwrap_or_default();
        for also in &binding.body.also_read {
            let of_extension = match extension.as_str() {
                ".yml" | ".yaml" => Some(FamilyName::Yaml),
                ".json" => Some(FamilyName::Json),
                ".xml" => Some(FamilyName::Xml),
                _ => None,
            };
            if of_extension == Some(*also) {
                family = *also;
            }
        }
        let kind = match family {
            FamilyName::Lines => FamilyKind::Lines(lines::LinesData::new(false)),
            FamilyName::Blocks => FamilyKind::Lines(lines::LinesData::new(true)),
            FamilyName::Bullets => FamilyKind::Lines(lines::LinesData::bullets()),
            FamilyName::Yaml => FamilyKind::Yaml(tree::TreeData::new(&binding, &options)),
            FamilyName::Json => FamilyKind::Json(tree::TreeData::new(&binding, &options)),
            FamilyName::Xml => FamilyKind::Xml(xml::XmlData::default()),
            FamilyName::Grammar => FamilyKind::Grammar(grammar::GrammarData::default()),
        };
        Family {
            text,
            binding,
            options,
            entries: Vec::new(),
            order: Vec::new(),
            leaves: Vec::new(),
            findings: RefCell::new(Vec::new()),
            unreadable: None,
            regexes: RefCell::new(HashMap::new()),
            kind,
        }
    }

    pub fn family_name(&self) -> &'static str {
        match &self.kind {
            FamilyKind::Yaml(_) => "yaml",
            FamilyKind::Json(_) => "json",
            FamilyKind::Xml(_) => "xml",
            FamilyKind::Grammar(_) => "grammar",
            FamilyKind::Lines(l) if l.bullets.is_some() => "bullets",
            FamilyKind::Lines(l) if l.blocks => "blocks",
            FamilyKind::Lines(_) => "lines",
        }
    }

    pub fn is_tree(&self) -> bool {
        matches!(self.kind, FamilyKind::Yaml(_) | FamilyKind::Json(_))
    }

    pub fn is_yaml(&self) -> bool {
        matches!(self.kind, FamilyKind::Yaml(_))
    }

    pub fn tree_data(&self) -> &tree::TreeData {
        match &self.kind {
            FamilyKind::Yaml(t) | FamilyKind::Json(t) => t,
            _ => panic!("the family is not yaml or json"),
        }
    }

    pub fn rule(&self, index: usize) -> &Rule {
        &self.binding.rules[index]
    }

    /// Builds the lossless reading; sets `unreadable` when the body is not well-formed.
    pub fn parse(&mut self) {
        match &self.kind {
            FamilyKind::Yaml(_) => self.yaml_parse(),
            FamilyKind::Json(_) => self.json_parse(),
            FamilyKind::Xml(_) => self.xml_parse(),
            FamilyKind::Grammar(_) => self.grammar_parse(),
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_parse(),
            FamilyKind::Lines(_) => self.lines_parse(),
        }
    }

    /// Whether the bytes of `gap` are trivia of this family: whitespace, line endings, punctuation.
    fn is_trivia(&self, gap: Span) -> bool {
        match &self.kind {
            FamilyKind::Yaml(_) => self.yaml_is_trivia(gap),
            FamilyKind::Json(_) => (gap.start..gap.end).all(|i| {
                matches!(
                    self.text.bytes()[i],
                    b' ' | b'\t' | b'\r' | b'\n' | b'{' | b'}' | b'[' | b']' | b',' | b':'
                )
            }),
            FamilyKind::Xml(_) => self.is_whitespace(gap),
            // Punctuation the tree leaves out lies between the tokens; what does not parse is a leaf of its own.
            FamilyKind::Grammar(_) => true,
            FamilyKind::Lines(_) => {
                (gap.start..gap.end).all(|i| matches!(self.text.bytes()[i], b' ' | b'\t' | b'\r' | b'\n' | b'}'))
            }
        }
    }

    /// The gaps between leaves that are not trivia: empty when the reading accounts for every byte.
    pub fn unaccounted(&self) -> Vec<Span> {
        let mut gaps = Vec::new();
        let mut position = self.text.bom_length();
        let mut leaves = self.leaves.clone();
        leaves.sort_by_key(|l| l.start);
        for leaf in leaves {
            if leaf.start < position {
                continue;
            }
            if leaf.start > position && !self.is_trivia(Span::new(position, leaf.start)) {
                gaps.push(Span::new(position, leaf.start));
            }
            position = position.max(leaf.end);
        }
        if position < self.text.len() && !self.is_trivia(Span::new(position, self.text.len())) {
            gaps.push(Span::new(position, self.text.len()));
        }
        gaps
    }

    /// Whether a byte range holds only whitespace and line endings.
    pub fn is_whitespace(&self, span: Span) -> bool {
        self.text.bytes()[span.start..span.end]
            .iter()
            .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    }

    pub fn candidates(&self, rule: usize) -> Vec<Candidate> {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => self.tree_candidates(rule),
            FamilyKind::Xml(_) => self.xml_candidates(rule),
            FamilyKind::Grammar(_) => self.grammar_candidates(rule),
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_candidates(rule),
            FamilyKind::Lines(_) => self.lines_candidates(rule),
        }
    }

    pub fn block_candidates(&self, block: usize) -> Vec<Candidate> {
        match &self.kind {
            FamilyKind::Lines(_) => self.lines_block_candidates(block),
            FamilyKind::Grammar(_) => self.grammar_block_candidates(block),
            _ => Vec::new(),
        }
    }

    /// Whether `within` admits `entry`, given what claimed the entries around it (blocks).
    pub fn admits(
        &self,
        within: Option<&[String]>,
        entry: EntryId,
        claimed_by: &dyn Fn(EntryId) -> Option<String>,
    ) -> bool {
        match &self.kind {
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_admits(entry, claimed_by),
            FamilyKind::Lines(l) if l.blocks => {
                let Some(within) = within else { return true };
                let name = match self.entries[entry].parent {
                    None => Some("^".to_owned()),
                    Some(parent) => claimed_by(parent),
                };
                name.is_some_and(|n| within.contains(&n))
            }
            _ => true,
        }
    }

    /// The entries that structurally enclose `entry`, nearest first.
    pub fn enclosing(&self, entry: EntryId) -> Vec<EntryId> {
        let mut result = Vec::new();
        let mut parent = self.entries[entry].parent;
        while let Some(p) = parent {
            match &self.entries[p].kind {
                EntryKind::Tree(t) if t.is_root => break,
                EntryKind::Xml(x) if x.is_root => break,
                EntryKind::Grammar(_) if self.entries[p].parent.is_none() => break,
                _ => {}
            }
            result.push(p);
            parent = self.entries[p].parent;
        }
        result
    }

    /// The entry as CEL sees it (FBL section 4.1.4).
    pub fn cel_value(&self, candidate: &Candidate) -> Value {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => {
                self.tree_cel(self.entries[candidate.entry].tree().value, tree::Reading::Typed)
            }
            FamilyKind::Xml(_) => self.xml_cel_value(candidate.entry),
            FamilyKind::Grammar(_) => self.grammar_cel_value(candidate.entry),
            FamilyKind::Lines(_) => captures_map(candidate),
        }
    }

    /// The entry as CEL's `text` sees it (FBL 0.5 section 2.4); null for a family that has no such variable.
    pub fn cel_text(&self, candidate: &Candidate) -> Value {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => {
                self.tree_cel(self.entries[candidate.entry].tree().value, tree::Reading::Text)
            }
            _ => Value::Null,
        }
    }

    /// The entry as CEL's `style` sees it (FBL 0.5 section 2.4); null for a family that has no such variable.
    pub fn cel_style(&self, candidate: &Candidate) -> Value {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => {
                self.tree_cel(self.entries[candidate.entry].tree().value, tree::Reading::Style)
            }
            _ => Value::Null,
        }
    }

    /// The variable a rule's CEL has besides `entry`, `parent`, `line` and `registration`: `path` or `groups`.
    pub fn cel_extra(&self, candidate: &Candidate) -> (&'static str, Value) {
        match &self.kind {
            FamilyKind::Lines(_) => ("groups", captures_map(candidate)),
            _ => ("path", captures_map(candidate)),
        }
    }

    /// Reads a slot that is the family's own: a key, an attribute, text, a group, a capture.
    pub fn read(&self, candidate: &Candidate, slot: &Slot, html_paragraphs: bool) -> SlotRead {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => self.tree_read(candidate, slot),
            FamilyKind::Xml(_) => self.xml_read(candidate, slot, html_paragraphs),
            FamilyKind::Grammar(_) => self.grammar_read(candidate, slot),
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_read(candidate, slot),
            FamilyKind::Lines(_) => self.lines_read(candidate, slot),
        }
    }

    /// Reads a word slot of an attribute the tool type's specification gives `many` (FBL 0.7 section 4.6): every
    /// word of the group that matches, each an item; a slot of a family without words, or no word slot, as any slot.
    pub fn read_many(&self, candidate: &Candidate, attribute: &AttributeBinding) -> SlotRead {
        let slot = &attribute.slot;
        let many = match &self.kind {
            FamilyKind::Lines(_) if !slot.continuation => self.lines_read_many(candidate, slot),
            FamilyKind::Grammar(_) if slot.node.is_some() => Some(self.grammar_read_many(candidate, attribute)),
            _ => None,
        };
        many.unwrap_or_else(|| self.read(candidate, slot, false))
    }

    /// Reads the slot named `name` of an enclosing entry, for a `parent` slot naming no attribute.
    pub fn read_raw(&self, entry: EntryId, name: &str) -> SlotRead {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => self.tree_read_raw(entry, name),
            FamilyKind::Xml(_) => self.xml_read_raw(entry, name),
            FamilyKind::Grammar(_) => SlotRead::absent(),
            FamilyKind::Lines(_) => self.lines_read_raw(entry, name),
        }
    }

    /// The header check (FBL section 5.6): whether the mark is there with the right value.
    /// Gives back the room the reading's lists grew into and no longer need. Nothing read changes.
    pub fn compact(&mut self) {
        self.entries.shrink_to_fit();
        for entry in &mut self.entries {
            entry.children.shrink_to_fit();
        }
        self.order.shrink_to_fit();
        self.leaves.shrink_to_fit();
        if let FamilyKind::Yaml(tree) | FamilyKind::Json(tree) = &mut self.kind {
            tree.values.shrink_to_fit();
            for value in &mut tree.values {
                value.entries.shrink_to_fit();
            }
        }
    }

    pub fn header_holds(&self, header: &HeaderSettings) -> bool {
        match &self.kind {
            FamilyKind::Yaml(_) | FamilyKind::Json(_) => self.tree_header_holds(header),
            FamilyKind::Xml(_) => self.xml_header_holds(header),
            FamilyKind::Grammar(_) => self.grammar_header_holds(header),
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_header_holds(header),
            FamilyKind::Lines(_) => self.lines_header_holds(header),
        }
    }

    /// Findings about entries no rule claimed, once reading is done.
    pub fn after_read(&self, claimed_by: &dyn Fn(EntryId) -> Option<String>) {
        match &self.kind {
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_after_read(claimed_by),
            FamilyKind::Lines(_) => self.lines_after_read(claimed_by),
            _ => {}
        }
    }

    // ---- writing (FBL section 6) ----

    /// The text that replaces a value's span: the new value in the value's own style when it fits (FBL 6.3).
    pub fn format(&self, read: &SlotRead, binding: Option<&AttributeBinding>, value: &Value) -> String {
        match &self.kind {
            FamilyKind::Yaml(_) => self.yaml_format(read, binding, value),
            FamilyKind::Json(_) => self.json_format(read, binding, value),
            FamilyKind::Xml(_) => self.xml_format(read, binding, value),
            FamilyKind::Grammar(_) => self.grammar_format(binding, value),
            FamilyKind::Lines(_) => self.lines_format(read, binding, value),
        }
    }

    /// Writes `changes` to `element`; the engine has already refused what must be refused.
    pub fn write(&self, plan: &mut Plan<'_>, element: &ReadElement, changes: &[SlotChange<'_>]) -> Planning {
        match &self.kind {
            FamilyKind::Yaml(_) => self.yaml_write(plan, element, changes),
            FamilyKind::Json(_) => self.json_write(plan, element, changes),
            FamilyKind::Xml(_) => self.xml_write(plan, element, changes),
            FamilyKind::Grammar(_) => self.grammar_write(plan, element, changes),
            FamilyKind::Lines(_) => self.lines_write(plan, element, changes),
        }
    }

    pub fn insert(&self, plan: &mut Plan<'_>, request: &InsertRequest) -> Planning {
        match &self.kind {
            FamilyKind::Yaml(_) => self.yaml_insert(plan, request),
            FamilyKind::Json(_) => self.json_insert(plan, request),
            FamilyKind::Xml(_) => self.xml_insert(plan, request),
            FamilyKind::Grammar(_) => self.grammar_insert(plan, request),
            FamilyKind::Lines(l) if l.bullets.is_some() => self.bullets_insert(plan, request),
            FamilyKind::Lines(_) => self.lines_insert(plan, request),
        }
    }

    pub fn remove(&self, plan: &mut Plan<'_>, element: usize, removed: &[usize]) -> Planning {
        match &self.kind {
            FamilyKind::Yaml(_) => self.yaml_remove(plan, element, removed),
            FamilyKind::Json(_) => self.json_remove(plan, element),
            FamilyKind::Xml(_) => self.xml_remove(plan, element),
            FamilyKind::Grammar(_) => self.grammar_remove(plan, element),
            FamilyKind::Lines(_) => {
                let entry = plan.reading.element(element).entry();
                plan.add_span(Operation::RemoveEntry, self.entries[entry].removal_span(), "");
                Ok(())
            }
        }
    }

    /// Whether a new or moved entry takes its place from its position among its siblings, and a move may change its
    /// parent (FBL 0.7 section 6.3): true for a family whose nesting is its indentation, false where the binding's
    /// insert place alone decides.
    pub fn places_by_position(&self) -> bool {
        self.is_bullets()
    }

    /// Moves an element's entry, with everything it contains, to `index` among `parent`'s children as they are
    /// without it (the top level when it is none). Only a family that [`Family::places_by_position`] plans one.
    pub fn move_to_position(
        &self,
        plan: &mut Plan<'_>,
        element: usize,
        parent: Option<usize>,
        index: usize,
    ) -> Planning {
        if self.is_bullets() {
            return self.bullets_move_to(plan, element, parent, index);
        }
        let type_name = &self.rule(plan.reading.element(element).rule).type_name;
        refuse(format!(
            "This file's binding cannot move a {type_name} to another parent; only a persistence plugin can."
        ))
    }

    /// Moves an element's entry among its siblings: a `remove-entry` where it is and an `insert-entry` of the same
    /// bytes before `before`'s entry, or after `last`'s when it goes last (FBL section 5.5). This is the move of a
    /// family whose entries stand on lines of their own; json, which separates its entries with commas, plans its
    /// own.
    pub fn move_entry(&self, plan: &mut Plan<'_>, element: usize, before: Option<usize>, last: usize) -> Planning {
        if let FamilyKind::Json(_) = &self.kind {
            return self.json_move(plan, element, before, last);
        }
        let reading = plan.reading;
        let span = self.entries[reading.element(element).entry()].removal_span();
        let offset = match before {
            Some(b) => self.entries[reading.element(b).entry()].removal_span().start,
            None => self.entries[reading.element(last).entry()].removal_span().end,
        };
        plan.add_span(Operation::RemoveEntry, span, "");
        plan.add(Operation::InsertEntry, offset, offset, self.text.text_of(span));
        Ok(())
    }

    /// Moves an element's entry to another parent: a `remove-entry` of its line span and an `insert-entry` of the
    /// same bytes before `before`'s entry, or after `last`'s, the last of its new siblings (FBL section 5.5). Into a
    /// parent with no such children it goes where a new entry would, which only xml plans here. Entries keep their
    /// bytes, so a family that reads its nesting from indentation moves an entry only to siblings at its own column;
    /// json, which separates its entries with commas, does not move one to another parent.
    pub fn move_to(
        &self,
        plan: &mut Plan<'_>,
        element: usize,
        parent: Option<usize>,
        before: Option<usize>,
        last: Option<usize>,
    ) -> Planning {
        let reading = plan.reading;
        let type_name = &self.rule(reading.element(element).rule).type_name;
        let entry = &self.entries[reading.element(element).entry()];
        let cannot = || refuse(format!("This file's binding cannot move a {type_name} there."));
        let (FamilyKind::Yaml(_) | FamilyKind::Xml(_) | FamilyKind::Lines(_)) = &self.kind else {
            return cannot();
        };
        let Some(span) = entry.line_span else {
            return cannot();
        };
        let Some(sibling) = before.or(last) else {
            return match (&self.kind, parent) {
                (FamilyKind::Xml(_), Some(parent)) => self.xml_move_into(plan, element, parent),
                _ => cannot(),
            };
        };
        let target = &self.entries[reading.element(sibling).entry()];
        let Some(line) = target.line_span else {
            return cannot();
        };
        if target.indent != entry.indent && !matches!(self.kind, FamilyKind::Xml(_)) {
            return cannot();
        }
        let offset = if before.is_some() { line.start } else { line.end };
        plan.add_span(Operation::RemoveEntry, span, "");
        plan.add(
            Operation::InsertEntry,
            offset,
            offset,
            self.lines_at(offset, self.text.text_of(span)),
        );
        Ok(())
    }

    /// The whole lines `text` stands for when inserted at `offset`: itself when `offset` starts a line and `text`
    /// ends one; at the end of a body whose last line has no line ending, `text` written as that ending followed by
    /// lines. None for any other text, such as an entry written after a separator on its sibling's line.
    pub fn as_lines(&self, offset: usize, text: &str) -> Option<String> {
        let line_start = offset == 0 || self.text.text(offset - 1, offset) == "\n";
        if line_start && text.ends_with('\n') {
            return Some(text.to_owned());
        }
        let at_unended_end = offset == self.text.len() && self.text.lines().last().is_some_and(|l| l.ending.is_empty());
        let rest = text.strip_prefix("\r\n").or_else(|| text.strip_prefix('\n'))?;
        (at_unended_end && !rest.is_empty()).then(|| format!("{rest}{}", self.newline_at(offset)))
    }

    /// Whole `lines` as the text of an insertion at `offset`, a line start or the end of the body: at the end of a
    /// body whose last line has no line ending, the ending is written first and none after (FBL section 6.3).
    pub fn lines_at(&self, offset: usize, lines: &str) -> String {
        let body = lines
            .strip_suffix("\r\n")
            .or_else(|| lines.strip_suffix('\n'))
            .unwrap_or(lines);
        let newline = self.newline_at(offset);
        if offset == self.text.len()
            && !self.text.is_empty()
            && self.text.lines().last().is_some_and(|l| l.ending.is_empty())
        {
            return format!("{newline}{body}");
        }
        format!("{body}{newline}")
    }

    // ---- shared helpers ----

    pub fn newline_at(&self, offset: usize) -> String {
        self.text.newline_at(offset, &self.binding.text.newline)
    }

    pub fn regex(&self, expression: &str, case_insensitive: bool) -> Rc<BoundedRegex> {
        let key = format!("{}{expression}", if case_insensitive { "i:" } else { "s:" });
        if let Some(regex) = self.regexes.borrow().get(&key) {
            return regex.clone();
        }
        let regex = Rc::new(BoundedRegex::new(expression, case_insensitive));
        self.regexes.borrow_mut().insert(key, regex.clone());
        regex
    }

    pub fn locate(&self, span: Span) -> SourceLocation {
        let (line, column) = self.text.position(span.start);
        SourceLocation {
            file: self.options.file_name.clone(),
            line,
            column,
            length: self.text.code_points(span.start, span.end),
        }
    }

    pub fn report(&self, code: &str, severity: FindingSeverity, message: impl Into<String>, span: Option<Span>) {
        self.report_with(code, severity, message, span, None);
    }

    pub fn report_with(
        &self,
        code: &str,
        severity: FindingSeverity,
        message: impl Into<String>,
        span: Option<Span>,
        detail: Option<Vec<(String, String)>>,
    ) {
        let finding = Finding {
            code: code.to_owned(),
            severity,
            message: message.into(),
            location: span.map(|s| self.locate(s)),
            detail,
        };
        self.findings.borrow_mut().push(finding);
    }

    /// `columns` of the binding's indentation character.
    pub fn indentation(&self, columns: usize) -> String {
        let c = if self.binding.text.indent == 0 { '\t' } else { ' ' };
        std::iter::repeat_n(c, columns).collect()
    }

    /// The indentation step (FBL section 6.3): the difference between the indentation of the first parent and
    /// child pair in document order, else the binding's `text.indent`.
    pub fn step(&self) -> usize {
        if let FamilyKind::Xml(_) = &self.kind {
            return self.xml_step();
        }
        for &id in &self.order {
            let entry = &self.entries[id];
            if let Some(parent) = entry.parent {
                let parent = &self.entries[parent];
                if entry.indent > parent.indent && entry.line_span.is_some() && parent.line_span.is_some() {
                    return entry.indent - parent.indent;
                }
            }
        }
        self.default_step()
    }

    pub fn default_step(&self) -> usize {
        if self.binding.text.indent == 0 {
            1
        } else {
            self.binding.text.indent
        }
    }

    /// Whether the bytes of the line holding `offset` before it are all whitespace.
    pub fn only_whitespace_before(&self, offset: usize) -> bool {
        let line = self.text.lines()[self.text.line_index_at(offset)];
        let start = line.start.max(self.text.bom_length());
        (start..offset).all(|i| matches!(self.text.bytes()[i], b' ' | b'\t'))
    }

    /// Whether only whitespace (and a comment, when `comment` is given) follows `offset` on its line.
    pub fn only_trivia_after(&self, offset: usize, comment: Option<u8>) -> bool {
        let line = self.text.lines()[self.text.line_index_at(offset)];
        let bytes = self.text.bytes();
        for i in offset..line.content_end {
            let b = bytes[i];
            if let Some(c) = comment
                && b == c
                && (i == offset || matches!(bytes[i - 1], b' ' | b'\t'))
            {
                return true;
            }
            if !matches!(b, b' ' | b'\t') {
                return false;
            }
        }
        true
    }

    /// The index of `binding` among the binding's rules' attribute bindings, when it is one of them.
    pub fn binding_index(&self, binding: &AttributeBinding) -> Option<(usize, usize)> {
        for (r, rule) in self.binding.rules.iter().enumerate() {
            for (a, (_, attribute)) in rule.attributes.iter().enumerate() {
                if std::ptr::eq(attribute, binding) {
                    return Some((r, a));
                }
            }
        }
        None
    }

    pub fn block(&self, index: usize) -> &BlockRule {
        &self.binding.blocks[index]
    }
}

/// The captures of a candidate as a CEL map.
fn captures_map(candidate: &Candidate) -> Value {
    Value::map(
        candidate
            .captures
            .iter()
            .map(|(k, v)| (k.clone(), Value::from(v.as_str())))
            .collect(),
    )
}

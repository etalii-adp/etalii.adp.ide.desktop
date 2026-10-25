//! The filter bar and what its query leaves of the page and in which order (etalii.adp spec 036, T048), ported from
//! the standalone host's `hierarchySurface/library/narrowing/{filterQuery.ts,FilterBar.tsx,SortMenu.tsx}` at a7fdf03:
//! the query in the notation of DESL section 7.7.3 read word by word, the nodes it leaves, the order of a level's
//! entries, the words that hide a node, and the words the field offers to complete the one being typed; and the bar
//! above the page that applies it as it is typed, sorts, and stores it under a name. Narrowing and sorting write
//! nothing (FR-049); storing, renaming and removing a filter are gestures.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

use adp_desl::hierarchy::definition::{DOCUMENT_TARGET, Filters, LabelDefinition, SortKey};
use adp_desl::hierarchy::model::{OptionList, SurfaceFilter, SurfaceModel, SurfaceNode, roles};
use eframe::egui::{self, FontId, RichText, accesskit::Role};

use super::HierarchyTab;
use super::layout::HierarchyIndex;
use crate::a11y;
use crate::icons;
use crate::table::view::Colours;

/// The word that selects levels by their title.
pub const ITEM_PREFIX: &str = "item:";
/// The word that orders the entries within each level.
pub const SORT_PREFIX: &str = "sort:";

/// What one word of a query selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordKind {
    /// A label's value.
    Label,
    /// A level's title pattern.
    Item,
    /// A sort key.
    Sort,
    /// A word the notation does not know, which selects nothing.
    Unknown,
}

/// One word of a query, as typed, and what it selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryWord {
    /// The word as written.
    pub text: String,
    pub kind: WordKind,
    /// The label a label word names.
    pub label: String,
    /// What follows the prefix; a sort key in lower case.
    pub value: String,
}

/// The values of `label` that `node` carries.
pub fn values_of<'n>(node: &'n SurfaceNode, label: &str) -> &'n [String] {
    node.labels
        .iter()
        .find(|v| v.label == label)
        .map(|v| v.values.as_slice())
        .unwrap_or_default()
}

/// A filter's query read word by word: `<prefix><name>` for a label, `item:pattern` for the levels whose title matches,
/// and `sort:key`. The longest prefix that fits a word decides what it is, so `category:` is not mistaken for a label
/// introduced by `c`; a prefix alone is no word of its label.
pub fn parse_query(text: &str, labels: &[LabelDefinition]) -> Vec<QueryWord> {
    let mut prefixes: Vec<&LabelDefinition> = labels.iter().collect();
    prefixes.sort_by_key(|l| std::cmp::Reverse(l.prefix.len()));
    text.split_whitespace()
        .map(|word| {
            // ASCII lower case keeps every byte where it was, so the prefix's length cuts the word as written.
            let lower = word.to_ascii_lowercase();
            let word_of = |kind, label: &str, value: String| QueryWord {
                text: word.to_owned(),
                kind,
                label: label.to_owned(),
                value,
            };
            if lower.starts_with(ITEM_PREFIX) && word.len() > ITEM_PREFIX.len() {
                return word_of(WordKind::Item, "", word[ITEM_PREFIX.len()..].to_owned());
            }
            if lower.starts_with(SORT_PREFIX) && word.len() > SORT_PREFIX.len() {
                return word_of(WordKind::Sort, "", lower[SORT_PREFIX.len()..].to_owned());
            }
            match prefixes
                .iter()
                .find(|l| lower.starts_with(&l.prefix.to_ascii_lowercase()) && word.len() > l.prefix.len())
            {
                Some(label) => word_of(WordKind::Label, &label.label, word[label.prefix.len()..].to_owned()),
                None => word_of(WordKind::Unknown, "", word.to_owned()),
            }
        })
        .collect()
}

/// The query's text again, its words in their order.
pub fn query_text(words: &[QueryWord]) -> String {
    words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
}

/// The sort key of a query: of several `sort:` words the first counts; empty for the file's order.
pub fn sort_of(words: &[QueryWord]) -> &str {
    words
        .iter()
        .find(|w| w.kind == WordKind::Sort)
        .map_or("", |w| w.value.as_str())
}

/// The query with its sort set to `key`, in the place of the first `sort:` word, or last; empty takes every `sort:`
/// word out.
pub fn with_sort(words: &[QueryWord], key: &str) -> Vec<QueryWord> {
    let others = words.iter().filter(|w| w.kind != WordKind::Sort).cloned();
    if key.is_empty() {
        return others.collect();
    }
    let sort = QueryWord {
        text: format!("{SORT_PREFIX}{key}"),
        kind: WordKind::Sort,
        label: String::new(),
        value: key.to_owned(),
    };
    match words.iter().position(|w| w.kind == WordKind::Sort) {
        None => others.chain(std::iter::once(sort)).collect(),
        Some(at) => words
            .iter()
            .enumerate()
            .filter_map(|(place, w)| match (w.kind, place == at) {
                (WordKind::Sort, true) => Some(sort.clone()),
                (WordKind::Sort, false) => None,
                _ => Some(w.clone()),
            })
            .collect(),
    }
}

/// A title pattern as a test: `*` any run of characters, `?` one, the rest as written, in any case, the whole title.
#[derive(Debug, Clone)]
struct Pattern(Vec<char>);

impl Pattern {
    fn of(pattern: &str) -> Pattern {
        Pattern(pattern.chars().flat_map(char::to_lowercase).collect())
    }

    fn matches(&self, title: &str) -> bool {
        let title: Vec<char> = title.chars().flat_map(char::to_lowercase).collect();
        // Whether the pattern from `p` on matches the title from `t` on, kept for every pair: a `*` tries every run.
        let (pattern, rows) = (&self.0, title.len() + 1);
        let mut matched = vec![false; (pattern.len() + 1) * rows];
        matched[pattern.len() * rows + title.len()] = true;
        for p in (0..pattern.len()).rev() {
            for t in (0..=title.len()).rev() {
                matched[p * rows + t] = match pattern[p] {
                    '*' => matched[(p + 1) * rows + t] || (t < title.len() && matched[p * rows + t + 1]),
                    '?' => t < title.len() && matched[(p + 1) * rows + t + 1],
                    c => t < title.len() && title[t] == c && matched[(p + 1) * rows + t + 1],
                };
            }
        }
        matched[0]
    }
}

/// The patterns of a query's `item:` words.
fn patterns_of(words: &[QueryWord]) -> Vec<Pattern> {
    words
        .iter()
        .filter(|w| w.kind == WordKind::Item)
        .map(|w| Pattern::of(&w.value))
        .collect()
}

/// Whether `node` is a level whose title one of `patterns` matches.
fn selected(node: &SurfaceNode, patterns: &[Pattern]) -> bool {
    node.role == roles::LEVEL && patterns.iter().any(|p| p.matches(&node.title))
}

/// Two numbers as numbers where they are digits, as `localeCompare` with `numeric` has them: `P-9` before `P-10`.
fn compare_numbers(left: &str, right: &str) -> Ordering {
    let (mut left, mut right) = (left.chars().peekable(), right.chars().peekable());
    loop {
        match (left.peek().copied(), right.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(l), Some(r)) if l.is_ascii_digit() && r.is_ascii_digit() => {
                let run = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut digits = String::new();
                    while let Some(c) = chars.next_if(char::is_ascii_digit) {
                        digits.push(c);
                    }
                    digits.trim_start_matches('0').to_owned()
                };
                let (l, r) = (run(&mut left), run(&mut right));
                let order = l.len().cmp(&r.len()).then_with(|| l.cmp(&r));
                if order != Ordering::Equal {
                    return order;
                }
            }
            (Some(l), Some(r)) => {
                let order = compare_titles(&l.to_string(), &r.to_string());
                if order != Ordering::Equal {
                    return order;
                }
                left.next();
                right.next();
            }
        }
    }
}

/// Two titles in any case, as `localeCompare` with `sensitivity: "base"` has them.
fn compare_titles(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}

/// What a query leaves of the page, and in which order the entries of a level stand.
#[derive(Debug, Clone, Default)]
pub struct Narrowing {
    /// Whether the query narrows at all; false when it only sorts or is empty.
    pub narrows: bool,
    shown: HashSet<String>,
    key: String,
    /// Each label's list, its members' names in lower case, for a sort by the label.
    lists: HashMap<String, Vec<String>>,
}

impl Narrowing {
    /// The page a query leaves (FR-019, FR-020; DESL section 7.7.3). Words of one kind are alternatives and kinds
    /// combine: an entry stays when, for every label its words name, it carries one of them, and it stands in a level
    /// an `item:` word selects, or below one, or no `item:` word is given. A level stays when it holds an entry that
    /// stays, or, with no word on entries, when it is selected or below one that is; and every level above what stays
    /// stays with it. The sort orders the entries within each level: by number, by title, or by the place of their
    /// value in a label's list, those without one last; the levels keep the file's order.
    pub fn of(model: &SurfaceModel, index: &HierarchyIndex, words: &[QueryWord]) -> Narrowing {
        let mut by_label: Vec<(&str, HashSet<String>)> = Vec::new();
        for word in words.iter().filter(|w| w.kind == WordKind::Label) {
            let value = word.value.to_ascii_lowercase();
            match by_label.iter_mut().find(|(label, _)| *label == word.label) {
                Some((_, names)) => {
                    names.insert(value);
                }
                None => by_label.push((&word.label, HashSet::from([value]))),
            }
        }
        let patterns = patterns_of(words);
        let narrows = !by_label.is_empty() || !patterns.is_empty();
        let mut shown = HashSet::new();
        if narrows {
            for node in &model.nodes {
                let above = index.ancestors_of(&node.id);
                let in_scope =
                    patterns.is_empty() || selected(node, &patterns) || above.iter().any(|l| selected(l, &patterns));
                if !in_scope {
                    continue;
                }
                let carries = |(label, names): &(&str, HashSet<String>)| {
                    values_of(node, label)
                        .iter()
                        .any(|v| names.contains(&v.to_ascii_lowercase()))
                };
                let keep = if node.role == roles::LEVEL {
                    by_label.is_empty()
                } else {
                    by_label.iter().all(carries)
                };
                if keep {
                    shown.insert(node.id.clone());
                    shown.extend(above.iter().map(|l| l.id.clone()));
                }
            }
        }
        Narrowing {
            narrows,
            shown,
            key: sort_of(words).to_owned(),
            lists: model
                .option_lists
                .iter()
                .map(|l| {
                    let names = l.options.iter().map(|o| o.name.to_ascii_lowercase()).collect();
                    (l.label.clone(), names)
                })
                .collect(),
        }
    }

    /// Whether `node` stays on the page.
    pub fn shows(&self, node: &SurfaceNode) -> bool {
        !self.narrows || self.shown.contains(&node.id)
    }

    /// The place of `node`'s value in the sort's list: a value the list does not hold stands after those it does, and
    /// no value after both.
    fn rank(&self, node: &SurfaceNode) -> usize {
        match values_of(node, &self.key).first() {
            None => usize::MAX,
            Some(value) => {
                let value = value.to_ascii_lowercase();
                self.lists
                    .get(&self.key)
                    .and_then(|l| l.iter().position(|n| *n == value))
                    .unwrap_or(usize::MAX - 1)
            }
        }
    }

    /// The entries of one level in the query's order: the file's when it has no sort, or a key it does not know.
    /// The sort is stable, so equal entries keep the file's order.
    pub fn order<'m>(&self, mut entries: Vec<&'m SurfaceNode>) -> Vec<&'m SurfaceNode> {
        match self.key.as_str() {
            "" => {}
            "number" => entries.sort_by(|l, r| compare_numbers(&l.number, &r.number)),
            "title" => entries.sort_by(|l, r| compare_titles(&l.title, &r.title)),
            key if self.lists.contains_key(key) => entries.sort_by_key(|n| self.rank(n)),
            _ => {}
        }
        entries
    }
}

/// The query without the words that hide `node_id`, as far as they hide it (FR-018, FR-021): the `item:` words when
/// the node stands outside every level they select, and the words of each label the node does not carry, or every
/// label's words for a level, which they hide by holding no entry that stays. The sort is kept.
pub fn unhiding(model: &SurfaceModel, index: &HierarchyIndex, words: &[QueryWord], node_id: &str) -> Vec<QueryWord> {
    let Some(node) = index.by_id.get(node_id).copied() else {
        return words.to_vec();
    };
    if Narrowing::of(model, index, words).shows(node) {
        return words.to_vec();
    }
    let patterns = patterns_of(words);
    let outside = !patterns.is_empty()
        && !std::iter::once(node)
            .chain(index.ancestors_of(node_id))
            .any(|l| selected(l, &patterns));
    words
        .iter()
        .filter(|word| match word.kind {
            WordKind::Item => !outside,
            WordKind::Label if node.role == roles::LEVEL => false,
            WordKind::Label => {
                let names: HashSet<String> = words
                    .iter()
                    .filter(|o| o.kind == WordKind::Label && o.label == word.label)
                    .map(|o| o.value.to_ascii_lowercase())
                    .collect();
                values_of(node, &word.label)
                    .iter()
                    .any(|v| names.contains(&v.to_ascii_lowercase()))
            }
            _ => true,
        })
        .cloned()
        .collect()
}

/// A word the filter field offers to complete the one being typed: the word, and what it is, shown beside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub word: String,
    pub hint: String,
}

/// The word the caret stands in or at the end of, in characters: where it starts, where it ends, and what of it is
/// typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordAt {
    pub start: usize,
    pub end: usize,
    pub typed: String,
}

/// The word the caret, `caret` characters into `text`, stands in or at the end of.
pub fn word_at(text: &str, caret: usize) -> WordAt {
    let chars: Vec<char> = text.chars().collect();
    let caret = caret.min(chars.len());
    let start = chars[..caret]
        .iter()
        .rposition(|c| c.is_whitespace())
        .map_or(0, |at| at + 1);
    let end = chars[caret..]
        .iter()
        .position(|c| c.is_whitespace())
        .map_or(chars.len(), |at| caret + at);
    WordAt {
        start,
        end,
        typed: chars[start..caret].iter().collect(),
    }
}

/// What the field offers for `typed`, the start of a word (DESL section 7.7.3): the labels' prefixes, `item:` and
/// `sort:`, then every member of every label's list written with the label's prefix, then `sort:` with each key, those
/// that start with what is typed, in any case. A word written in full is not offered again. Nothing is offered for
/// nothing typed but the prefixes.
pub fn completions_of(
    typed: &str,
    labels: &[LabelDefinition],
    option_lists: &[OptionList],
    sort_keys: &[SortKey],
) -> Vec<Completion> {
    let completion = |word: String, hint: &str| Completion {
        word,
        hint: hint.to_owned(),
    };
    let mut prefixes: Vec<Completion> = labels
        .iter()
        .map(|l| completion(l.prefix.clone(), l.value_word()))
        .collect();
    prefixes.push(completion(ITEM_PREFIX.into(), "title"));
    prefixes.push(completion(SORT_PREFIX.into(), "order"));
    let wanted = typed.to_ascii_lowercase();
    if wanted.is_empty() {
        return prefixes;
    }
    let members = labels.iter().flat_map(|l| {
        option_lists
            .iter()
            .filter(move |list| list.label == l.label)
            .take(1)
            .flat_map(|list| list.options.iter())
            .map(move |o| completion(format!("{}{}", l.prefix, o.name), l.value_word()))
    });
    let sorts = sort_keys
        .iter()
        .map(|k| completion(format!("{SORT_PREFIX}{}", k.key), &k.label));
    prefixes
        .into_iter()
        .chain(members)
        .chain(sorts)
        .filter(|c| {
            let word = c.word.to_ascii_lowercase();
            word.starts_with(&wanted) && word != wanted
        })
        .collect()
}

/// The keys the sort menu offers: the definition's, then each label of one value, by the value's own name.
pub fn sort_keys(labels: &[LabelDefinition], keys: &[SortKey]) -> Vec<SortKey> {
    keys.iter()
        .cloned()
        .chain(labels.iter().filter(|l| !l.many).map(|l| SortKey {
            key: l.label.clone(),
            label: l.value_word().to_owned(),
        }))
        .collect()
}

/// The most completions the field offers at once.
const MOST_COMPLETIONS: usize = 12;

/// The query `text` with the word the caret stands in, `word`, written `completion`: a prefix stays open for its name,
/// parted from what follows it, and a whole word gets a space after it, as the standalone host's `complete` writes it.
/// The text, and where the caret goes, in characters.
pub fn completed(text: &str, word: &WordAt, completion: &str, labels: &[LabelDefinition]) -> (String, usize) {
    let whole =
        !(labels.iter().any(|l| l.prefix == completion) || completion == ITEM_PREFIX || completion.ends_with(':'));
    let inserted = if whole {
        format!("{completion} ")
    } else {
        completion.to_owned()
    };
    let chars: Vec<char> = text.chars().collect();
    let before: String = chars[..word.start].iter().collect();
    let rest: String = chars[word.end.min(chars.len())..].iter().collect();
    let rest = rest.trim_start();
    // A whole word brings its own space; a prefix is parted from what follows it.
    let tail = if rest.is_empty() || whole {
        rest.to_owned()
    } else {
        format!(" {rest}")
    };
    let caret = word.start + inserted.chars().count();
    (format!("{before}{inserted}{tail}"), caret)
}

/// What the bar's name field is open for: storing the applied query, or renaming the stored filter it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Naming {
    Store,
    Rename(SurfaceFilter),
}

/// The bar's own state, between frames: the name field, when it is open, and what is typed in it.
#[derive(Debug, Clone, Default)]
pub struct FilterBar {
    pub naming: Option<Naming>,
    pub name: String,
    /// Whether the name field opened since the last frame, which gives it the keyboard.
    pub opened: bool,
}

impl HierarchyTab {
    /// The bar's query, read word by word.
    pub fn words(&self) -> Vec<QueryWord> {
        parse_query(&self.query, &self.definition.labels)
    }

    /// What the bar's query leaves of the page.
    pub fn narrowing(&self) -> Narrowing {
        let index = HierarchyIndex::of(&self.model.nodes);
        Narrowing::of(&self.model, &index, &self.words())
    }

    /// The sort menu: the query with its sort set to `key`, the file's order for empty. Nothing is sent.
    pub fn set_sort(&mut self, key: &str) {
        self.query = query_text(&with_sort(&self.words(), key));
    }

    /// The stored filter the query is, as typed apart from the spaces around it.
    pub fn applied_filter(&self) -> Option<&SurfaceFilter> {
        let query = self.query.trim();
        self.model.filters.iter().find(|f| f.query == query)
    }

    /// Opens the bar's name field for `naming`: empty for a store, the filter's name for a rename.
    pub fn start_naming(&mut self, naming: Naming) {
        self.filter_bar.name = match &naming {
            Naming::Store => String::new(),
            Naming::Rename(filter) => filter.name.clone(),
        };
        self.filter_bar.naming = Some(naming);
        self.filter_bar.opened = true;
    }

    /// Keeps what the name field holds: the applied query stored under it, or the filter renamed, unless it is empty
    /// or, for a rename, unchanged; the field closes either way.
    pub fn commit_name(&mut self) {
        let Some(naming) = self.filter_bar.naming.take() else {
            return;
        };
        let Some(filters) = self.definition.filters.clone() else {
            return;
        };
        let typed = self.filter_bar.name.trim().to_owned();
        if typed.is_empty() {
            return;
        }
        let (o, p) = (&filters.operations, &filters.params);
        match naming {
            Naming::Store => {
                let params = BTreeMap::from([(p.name.clone(), typed), (p.query.clone(), self.query.trim().to_owned())]);
                self.gesture(&o.store, DOCUMENT_TARGET, params);
            }
            Naming::Rename(filter) if typed != filter.name => {
                self.gesture(&o.rename, &filter.id, BTreeMap::from([(p.name.clone(), typed)]));
            }
            Naming::Rename(_) => {}
        }
    }

    /// Removes the stored filter `id`: a gesture.
    pub fn remove_filter(&mut self, id: &str) {
        if let Some(filters) = self.definition.filters.clone() {
            self.gesture(&filters.operations.remove, id, BTreeMap::new());
        }
    }
}

/// The bar above the page that narrows and sorts it (FR-019, FR-020, FR-048): a query typed in the notation of DESL
/// section 7.7.3, applied as it is typed, the sort menu, and the filters the document stores, each applied by picking
/// it. The applied query is stored under a name, and the stored filter it is renamed or removed. The word being typed
/// is completed from a list under the field: the labels' members with their prefixes, the prefixes, and the sort keys;
/// Tab or Enter takes the one marked, a click any, the arrows mark another and Escape closes it until the word changes.
pub fn filter_bar(ui: &mut egui::Ui, tab: &mut HierarchyTab, filters: &Filters, colours: &Colours, font: &FontId) {
    ui.label(
        RichText::new(icons::text("mdi-filter-variant"))
            .font(icons::font(font.size))
            .color(colours.muted),
    );
    query_field(ui, tab, filters);
    let words = tab.words();
    let keys = sort_keys(&tab.definition.labels, &filters.sort_keys);
    if let Some(key) = sort_menu(ui, tab.id().with("sort"), &keys, sort_of(&words)) {
        tab.set_sort(&key);
    }
    let applied = tab.applied_filter().cloned();
    if !tab.model.filters.is_empty() {
        let shown = applied.as_ref().map_or("Stored filters", |f| f.name.as_str());
        let mut picked = None;
        let choice = egui::ComboBox::from_id_salt(tab.id().with("stored"))
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for filter in &tab.model.filters {
                    let option = ui.selectable_label(applied.as_ref() == Some(filter), &filter.name);
                    a11y::describe(&option, Role::ListBoxOption, &filter.name);
                    if option.clicked() {
                        picked = Some(filter.query.clone());
                    }
                }
            });
        a11y::describe(&choice.response, Role::ComboBox, "Stored filters");
        if let Some(query) = picked {
            tab.query = query;
        }
    }
    if tab.filter_bar.naming.is_some() {
        name_field(ui, tab);
    } else if !tab.read_only() {
        if !tab.query.trim().is_empty() && applied.is_none() {
            let store = ui
                .button(format!("{} Store", icons::text("mdi-bookmark-plus-outline")))
                .on_hover_text("Store this filter under a name");
            a11y::describe(&store, Role::Button, "Store");
            if store.clicked() {
                tab.start_naming(Naming::Store);
            }
        }
        if let Some(filter) = &applied {
            if icon_button(ui, "mdi-pencil-outline", &format!("Rename {}", filter.name), "Rename") {
                tab.start_naming(Naming::Rename(filter.clone()));
            }
            if icon_button(
                ui,
                "mdi-bookmark-remove-outline",
                &format!("Remove {}", filter.name),
                "Remove",
            ) {
                tab.remove_filter(&filter.id);
            }
        }
    }
    if !tab.query.is_empty() && icon_button(ui, "mdi-close", "Clear the filter", "Clear") {
        tab.query.clear();
    }
    let narrowing = tab.narrowing();
    let entries: Vec<&SurfaceNode> = tab.model.nodes.iter().filter(|n| n.role == roles::ENTRY).collect();
    let shown = entries.iter().filter(|n| narrowing.shows(n)).count();
    let count = if shown == entries.len() {
        format!("{} {}", entries.len(), filters.entries)
    } else {
        format!("{shown} of {} {}", entries.len(), filters.entries)
    };
    let status = ui.label(RichText::new(&count).color(colours.muted));
    a11y::describe(&status, Role::Status, &count);
}

/// A control that is its icon alone, `name` for a screen reader and `tooltip` under the pointer; whether it was pressed.
fn icon_button(ui: &mut egui::Ui, icon: &str, name: &str, tooltip: &str) -> bool {
    let button = ui.button(RichText::new(icons::text(icon)).font(icons::font(14.0)));
    a11y::describe(&button, Role::Button, name);
    button.on_hover_text(tooltip).clicked()
}

/// The query, typed in place, and the words that complete the one being typed.
fn query_field(ui: &mut egui::Ui, tab: &mut HierarchyTab, filters: &Filters) {
    let id = tab.id().with("query");
    let placeholder = tab
        .definition
        .labels
        .iter()
        .map(|l| format!("{}name", l.prefix))
        .chain(std::iter::once(format!("{ITEM_PREFIX}back*")))
        .collect::<Vec<_>>()
        .join(" ");
    // Where the caret was, for writing a completion in place of the whole word it stands in.
    let before = tab.query.clone();
    let caret = egui::text_edit::TextEditState::load(ui.ctx(), id)
        .and_then(|s| s.cursor.char_range())
        .map_or(before.chars().count(), |r| r.primary.index.0);
    let labels = tab.definition.labels.clone();
    let lists = tab.model.option_lists.clone();
    let keys = filters.sort_keys.clone();
    let edit = egui::TextEdit::singleline(&mut tab.query)
        .hint_text(placeholder)
        .desired_width(260.0);
    let output = egui::CompletionPopup::new(id)
        .word_boundary(char::is_whitespace)
        .show(ui, edit, |query| {
            completions_of(query.word, &labels, &lists, &keys)
                .into_iter()
                .take(MOST_COMPLETIONS)
                .map(|c| egui::Suggestion::new(c.word.clone()).description(c.hint))
                .collect()
        });
    let response = &output.text_edit.response.response;
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(Role::SearchInput);
        node.set_label("Filter");
    });
    if let Some(accepted) = output.accepted {
        let (text, at) = completed(&before, &word_at(&before, caret), &accepted.insert, &labels);
        tab.query = text;
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
        state.store(ui.ctx(), id);
    }
}

/// The order of the entries within each level (FR-020): the file's, or by one of `keys`, a key the query holds that
/// the menu does not know shown too. The key picked, if one was; it changes the query's `sort:` word, so a stored
/// filter keeps it, and it writes nothing.
fn sort_menu(ui: &mut egui::Ui, id: egui::Id, keys: &[SortKey], value: &str) -> Option<String> {
    let mut options: Vec<(String, String)> = std::iter::once((String::new(), "File order".to_owned()))
        .chain(keys.iter().map(|k| (k.key.clone(), k.label.clone())))
        .collect();
    if !options.iter().any(|(key, _)| key == value) {
        options.push((value.to_owned(), value.to_owned()));
    }
    let shown = options
        .iter()
        .find(|(key, _)| key == value)
        .map(|(_, label)| label.clone());
    let mut picked = None;
    let choice = egui::ComboBox::from_id_salt(id)
        .selected_text(format!("{} {}", icons::text("mdi-sort"), shown.unwrap_or_default()))
        .show_ui(ui, |ui| {
            for (key, label) in &options {
                let option = ui.selectable_label(key == value, label);
                a11y::describe(&option, Role::ListBoxOption, label);
                if option.clicked() {
                    picked = Some(key.clone());
                }
            }
        });
    a11y::describe(&choice.response, Role::ComboBox, "Sort");
    picked
}

/// The field a filter's name is typed in: Enter keeps it, Escape drops it, and leaving it keeps it.
fn name_field(ui: &mut egui::Ui, tab: &mut HierarchyTab) {
    let id = tab.id().with("filter-name");
    let label = match &tab.filter_bar.naming {
        Some(Naming::Rename(filter)) => format!("New name of {}", filter.name),
        _ => "Name of the filter".to_owned(),
    };
    if std::mem::take(&mut tab.filter_bar.opened) {
        ui.memory_mut(|m| m.request_focus(id));
    }
    let focused = ui.memory(|m| m.has_focus(id));
    let (enter, escape) = if focused {
        ui.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        })
    } else {
        (false, false)
    };
    let edit = egui::TextEdit::singleline(&mut tab.filter_bar.name)
        .id(id)
        .hint_text("Name")
        .desired_width(140.0);
    // Escape is egui's own end of a focus, which would read as leaving the field and keep the name; it is kept for
    // the field, which drops the name by it.
    let filter = egui::EventFilter {
        escape: true,
        ..edit.get_event_filter()
    };
    let response = ui.add(edit.event_filter(filter));
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(Role::TextInput);
        node.set_label(label.as_str());
    });
    if escape {
        tab.filter_bar.naming = None;
        ui.memory_mut(|m| m.surrender_focus(id));
    } else if enter || response.lost_focus() {
        tab.commit_name();
        ui.memory_mut(|m| m.surrender_focus(id));
    }
}

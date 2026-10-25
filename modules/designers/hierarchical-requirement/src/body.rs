//! One requirement document read through FBL with the vendored binding, and its registration when it has one, into
//! the designer's model (the standalone host's `HierarchicalRequirementBody` at a7fdf03). The module has no parser of
//! its own: the entries are the library's reading of the binding's rules, and this adds what DESL says of them. A
//! placeholder is numbered in memory only, the mark line is read, and every fault of the companion's table "What
//! reading reports" is reported at its bullet with every other entry usable. Reading writes nothing.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use adp_cel::Value;
use adp_desl::DedEnvelope;
use adp_fbl::model::codes;
use adp_fbl::{FblElement, FblModel, OpenBody, OpenRegistration};
use regex::Regex;

use crate::definition::{self, ORIGIN};
use crate::model::{
    Assessment, AssessmentKind, Column, Document, Entry, EntryKind, Filter, Finding, MAXIMUM_WIDTH, MINIMUM_WIDTH,
    Member,
};

const ITEM: &str = "Item";
const REQUIREMENT: &str = "Requirement";
const ASSESSMENT: &str = "Assessment";

/// The written number on a bullet's line: the first word after its marker.
static WRITTEN_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[ \t]*[-*+][ \t]+(?<id>\S+)").expect("the pattern compiles"));

/// A counter's number, `ITEM-0004` or `REQ-0031`.
static COUNTER_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:ITEM|REQ)-(?<number>\d+)$").expect("the pattern compiles"));

/// A requirement document open through the binding, with its registration beside it when it has one.
pub struct RequirementBody {
    /// The Markdown file, open through the binding.
    pub body: OpenBody,
    /// The registration beside it, or none when there is none.
    pub registration: Option<OpenRegistration>,
    /// What the designer shows.
    pub document: Document,
}

impl RequirementBody {
    /// Reads the Markdown file `bytes`, named `file_name`, and its registration, by its bytes and name, when it has
    /// one.
    pub fn read(bytes: Vec<u8>, file_name: &str, registration: Option<(Vec<u8>, &str)>) -> RequirementBody {
        let binding = definition::binding();
        let body = OpenBody::open(bytes, binding.clone(), definition::options(file_name));
        let (mut registration, registration_name) = match registration {
            Some((bytes, name)) => (Some(OpenRegistration::open_with(bytes, &binding)), name),
            None => (None, ""),
        };
        let model = body.model();
        let document = if model.unreadable {
            let reason = model
                .findings
                .first()
                .map_or("The file could not be read.", |f| f.message.as_str())
                .to_owned();
            Document {
                findings: model.findings.iter().map(of).collect(),
                read_only_reason: format!(
                    "This file cannot be read as a requirement document: {reason} It is left exactly as it is and cannot be changed here."
                ),
                ..Document::default()
            }
        } else {
            Reading::new(body.bytes(), &model, registration.as_mut(), file_name).document(registration_name)
        };
        RequirementBody {
            body,
            registration,
            document,
        }
    }
}

fn of(finding: &adp_fbl::Finding) -> Finding {
    Finding::new(
        finding.code.clone(),
        finding.message.clone(),
        finding.location.as_ref().map_or(0, |l| l.line),
    )
}

fn prefix(type_name: &str) -> &'static str {
    if type_name == ITEM { "ITEM-" } else { "REQ-" }
}

fn counter(type_name: &str, number: u32) -> String {
    format!("{}{number:04}", prefix(type_name))
}

fn number(id: Option<&str>) -> u32 {
    id.and_then(|id| COUNTER_ID.captures(id))
        .and_then(|c| c["number"].parse().ok())
        .unwrap_or(0)
}

/// CEL's `lowerAscii`, which the specification's ids of list members are derived with.
pub(crate) fn lower_ascii(text: &str) -> String {
    text.to_ascii_lowercase()
}

fn text(element: &FblElement, attribute: &str) -> String {
    element
        .attributes
        .get(attribute)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn optional(element: &FblElement, attribute: &str) -> Option<String> {
    element
        .attributes
        .get(attribute)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn words(element: &FblElement, attribute: &str) -> Vec<String> {
    match element.attributes.get(attribute) {
        Some(Value::List(words)) => words.iter().filter_map(|w| w.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

/// One reading: the library's model of the file, made into the designer's.
struct Reading<'a> {
    bytes: &'a [u8],
    model: &'a FblModel,
    registration: Option<&'a mut OpenRegistration>,
    lines: Vec<String>,
    marks: Option<&'a FblElement>,
    file_name: &'a str,
}

impl<'a> Reading<'a> {
    fn new(
        bytes: &'a [u8],
        model: &'a FblModel,
        registration: Option<&'a mut OpenRegistration>,
        file_name: &'a str,
    ) -> Self {
        let text = String::from_utf8_lossy(bytes);
        Reading {
            bytes,
            model,
            registration,
            lines: text.split('\n').map(str::to_owned).collect(),
            marks: model.elements.iter().find(|e| e.type_name == "Marks"),
            file_name,
        }
    }

    fn document(mut self, registration_name: &str) -> Document {
        let entries = self.entries();
        let assessments = self.assessments();
        let mut findings = self.findings(&entries, &assessments);
        if let Some(registration) = self.registration.as_deref_mut() {
            registration.known_ids = Some(entries.iter().map(|e| e.id.clone()).collect());
            findings.extend(registration.findings(registration_name).iter().map(|f| Finding {
                in_registration: true,
                ..of(f)
            }));
        }
        let read_only_reason = DedEnvelope::read(self.bytes, self.file_name)
            .ok()
            .and_then(|envelope| envelope.read_only_reason(|designer| designer == ORIGIN))
            .unwrap_or_default();
        let members = |block: &str, attribute: &str| -> Vec<Member> {
            self.members(block, attribute, &entries)
                .into_iter()
                .map(|(name, colour)| Member::new(name, colour))
                .collect()
        };
        let (tags, categories, states) = (
            members("tags", "tags"),
            members("categories", "category"),
            members("states", "state"),
        );
        let filters = self
            .block("filters")
            .unwrap_or_default()
            .into_iter()
            .map(|(name, query)| Filter { name, query })
            .collect();
        Document {
            last_item: self.mark("lastItem"),
            last_requirement: self.mark("lastRequirement"),
            columns: self.columns(),
            entries,
            assessments,
            tags,
            categories,
            states,
            filters,
            findings,
            read_only_reason,
        }
    }

    /// The widths the user set, by depth: an entry whose depth or width is not a number in the specification's
    /// range is left out, so its column has the host's width.
    fn columns(&self) -> Vec<Column> {
        self.block("widths")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(level, width)| {
                let level: u32 = digits(&level)?;
                let width: u32 = digits(&width)?;
                (level >= 1 && (MINIMUM_WIDTH..=MAXIMUM_WIDTH).contains(&width)).then_some(Column { level, width })
            })
            .collect()
    }

    fn mark(&self, attribute: &str) -> Option<String> {
        self.marks.and_then(|m| optional(m, attribute))
    }

    fn written(&self, element: &FblElement) -> String {
        let line = element
            .line
            .checked_sub(1)
            .and_then(|at| self.lines.get(at))
            .map_or("", String::as_str);
        WRITTEN_ID
            .captures(line)
            .map_or_else(|| element.id.clone(), |c| c["id"].to_owned())
    }

    /// The highest number of a kind: the mark's, or the highest the file holds.
    fn highest(&self, read: &[(&FblElement, String)], type_name: &str, mark: &str) -> u32 {
        read.iter()
            .filter(|(element, _)| element.type_name == type_name)
            .map(|(_, written)| number(Some(written)))
            .max()
            .unwrap_or(0)
            .max(number(self.mark(mark).as_deref()))
    }

    /// The items and requirements in reading order, each by the number written on its bullet. A placeholder is
    /// given the next number of its kind, after the mark and every number the file holds, in reading order (DESL
    /// 0.2 section 5.3.5); a duplicate keeps its written number, where the library gives it a place id.
    fn entries(&self) -> Vec<Entry> {
        let read: Vec<(&FblElement, String)> = self
            .model
            .elements
            .iter()
            .filter(|e| e.type_name == ITEM || e.type_name == REQUIREMENT)
            .map(|e| (e, self.written(e)))
            .collect();
        let mut next_item = self.highest(&read, ITEM, "lastItem");
        let mut next_requirement = self.highest(&read, REQUIREMENT, "lastRequirement");
        let collapsed: HashSet<String> = self
            .block("collapsed")
            .unwrap_or_default()
            .into_iter()
            .filter(|(_, value)| value == "yes")
            .map(|(key, _)| key)
            .collect();
        let mut ids: HashMap<&str, String> = HashMap::new();
        let mut entries = Vec::new();
        for (element, written) in &read {
            let placeholder = *written == format!("{}NEW", prefix(&element.type_name));
            let id = if !placeholder {
                written.clone()
            } else if element.type_name == ITEM {
                next_item += 1;
                counter(ITEM, next_item)
            } else {
                next_requirement += 1;
                counter(REQUIREMENT, next_requirement)
            };
            ids.insert(element.id.as_str(), id.clone());
            let parent_id = element
                .parent_id
                .as_deref()
                .map(|parent| ids.get(parent).cloned().unwrap_or_else(|| parent.to_owned()));
            let kind = if element.type_name == ITEM {
                EntryKind::Item
            } else {
                EntryKind::Requirement {
                    category: optional(element, "category"),
                    state: optional(element, "state"),
                }
            };
            entries.push(Entry {
                collapsed: collapsed.contains(&id),
                id,
                parent_id,
                title: text(element, "title"),
                description: text(element, "description"),
                tags: words(element, "tags"),
                line: element.line,
                placeholder,
                kind,
            });
        }
        entries
    }

    fn assessments(&self) -> Vec<Assessment> {
        self.model
            .elements
            .iter()
            .filter(|e| e.type_name == ASSESSMENT)
            .enumerate()
            .map(|(place, element)| Assessment {
                id: format!("assessment/{place}"),
                kind: AssessmentKind::parse(&text(element, "kind")).unwrap_or(AssessmentKind::Remark),
                about: text(element, "about"),
                text: text(element, "text"),
                detail: text(element, "detail"),
                resolved: matches!(element.attributes.get("resolved"), Some(Value::Bool(true)))
                    || text(element, "resolved") == "true",
                line: element.line,
            })
            .collect()
    }

    /// The findings in DESL 0.2 section 6.3's order: the reader's in reading order, the library's among them, then
    /// `std.required`, `std.containment` and `std.mentions`, each in document order.
    fn findings(&self, entries: &[Entry], assessments: &[Assessment]) -> Vec<Finding> {
        let placeholders: HashSet<usize> = entries.iter().filter(|e| e.placeholder).map(|e| e.line).collect();
        let mut reader: Vec<Finding> = self
            .model
            .findings
            .iter()
            .filter(|f| {
                !(f.code == codes::DUPLICATE_ID && placeholders.contains(&f.location.as_ref().map_or(0, |l| l.line)))
            })
            .map(of)
            .chain(entries.iter().filter(|e| e.placeholder).map(|entry| {
                Finding::new(
                    codes::MISSING_ID,
                    format!(
                        "{} is numbered when the document is next changed: the file holds {}NEW.",
                        entry.id,
                        prefix(entry.type_name())
                    ),
                    entry.line,
                )
            }))
            .collect();
        reader.sort_by_key(|f| f.line);
        let mut required: Vec<Finding> = entries
            .iter()
            .filter(|e| e.title.is_empty())
            .map(|e| Finding::new("std.required", format!("{} has no title.", e.id), e.line))
            .collect();
        required.sort_by_key(|f| f.line);
        let mut by_id: HashMap<&str, &Entry> = HashMap::new();
        for entry in entries {
            by_id.entry(entry.id.as_str()).or_insert(entry);
        }
        let mut containment: Vec<Finding> = entries
            .iter()
            .filter_map(|entry| {
                match &entry.parent_id {
                    None if !entry.is_item() => Some(format!(
                        "{} stands in no item: a requirement stands in an item.",
                        entry.id
                    )),
                    Some(parent) if by_id.get(parent.as_str()).is_some_and(|p| !p.is_item()) => Some(format!(
                        "{} is nested in the requirement {parent}: a requirement holds nothing.",
                        entry.id
                    )),
                    _ => None,
                }
                .map(|message| Finding::new("std.containment", message, entry.line))
            })
            .collect();
        containment.sort_by_key(|f| f.line);
        let mut mentions = Vec::new();
        for entry in entries {
            unresolved(
                entry.type_name(),
                "description",
                &entry.description,
                entry.line + 1,
                &by_id,
                &mut mentions,
            );
        }
        for assessment in assessments {
            unresolved(
                ASSESSMENT,
                "about",
                &assessment.about,
                assessment.line,
                &by_id,
                &mut mentions,
            );
            unresolved(
                ASSESSMENT,
                "text",
                &assessment.text,
                assessment.line,
                &by_id,
                &mut mentions,
            );
            unresolved(
                ASSESSMENT,
                "detail",
                &assessment.detail,
                assessment.line + 1,
                &by_id,
                &mut mentions,
            );
        }
        mentions.sort_by_key(|f| f.line);
        reader
            .into_iter()
            .chain(required)
            .chain(containment)
            .chain(mentions)
            .collect()
    }

    /// The members of a list (DESL 0.2 sections 5.5 and 7.7.2): its block in the registration when there is one,
    /// else its initial members and then those the entries imply, in the order they first appear, spelt as they
    /// first appear.
    fn members(&self, block: &str, attribute: &str, entries: &[Entry]) -> Vec<(String, String)> {
        if let Some(stored) = self.block(block) {
            return stored;
        }
        let mut members = definition::initial(block);
        for entry in entries {
            let names: Vec<&str> = match attribute {
                "tags" => entry.tags.iter().map(String::as_str).collect(),
                "category" => entry.category().into_iter().collect(),
                _ => entry.state().into_iter().collect(),
            };
            for name in names {
                if members
                    .iter()
                    .all(|(member, _)| lower_ascii(member) != lower_ascii(name))
                {
                    members.push((name.to_owned(), "default".into()));
                }
            }
        }
        members
    }

    fn block(&self, name: &str) -> Option<Vec<(String, String)>> {
        self.registration.as_deref()?.block_entries(name)
    }
}

/// A number written in digits alone, as .NET's `NumberStyles.None` reads one.
fn digits(text: &str) -> Option<u32> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// The mentions of `text` that name no entry, each at the line it stands on.
fn unresolved(
    type_name: &str,
    attribute: &str,
    text: &str,
    first_line: usize,
    entries: &HashMap<&str, &Entry>,
    found: &mut Vec<Finding>,
) {
    if text.is_empty() {
        return;
    }
    let Some(pattern) = definition::mentions(type_name, attribute) else {
        return;
    };
    for mention in pattern.captures_iter(text) {
        let Some(id) = mention.name("id") else { continue };
        if entries.contains_key(id.as_str()) {
            continue;
        }
        let at = mention.get(0).map_or(0, |m| m.start());
        let line = first_line + text[..at].matches('\n').count();
        found.push(Finding::new(
            "std.mentions",
            format!("{} names no item or requirement of this document.", id.as_str()),
            line,
        ));
    }
}

//! Counted ids (DESL 0.2 section 5.3.5), offered to a designer's description lowered onto DISL (etalii.adp spec 076,
//! T058): a type's ids are a prefix and a number counted up per document, the last one given kept in a document
//! attribute, the mark; an entry written with the placeholder (`REQ-NEW`) has no number yet, is named `line/<line>`
//! until it gets one (DESL 0.3) and gets one with the next edit that writes the document, in reading order.

use adp_cel::Value;

use crate::json;
use crate::loading::specification::DislSpecification;
use crate::model::diagram::DislDiagram;
use crate::model::element::DislElement;

/// A type's counter: `persistence.ids.types.<type>` with `strategy: counter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counter {
    pub prefix: String,
    pub digits: usize,
    /// The document attribute that holds the last id given.
    pub mark: String,
    pub placeholder: Option<String>,
}

impl Counter {
    /// The counter of `type_name`, or of the nearest type of its linearisation that has one.
    pub fn of(specification: &DislSpecification, type_name: &str) -> Option<Counter> {
        let types = specification.root().pointer("/persistence/ids/types")?;
        let linearisation = specification
            .metamodel()
            .type_of(type_name)
            .map(|t| t.linearisation.clone())
            .unwrap_or_else(|| vec![type_name.to_owned()]);
        let rule = linearisation
            .iter()
            .find_map(|t| json::get(types, t).filter(|r| json::string_of(r, "strategy") == Some("counter")))?;
        Some(Counter {
            prefix: json::string_of(rule, "prefix")?.to_owned(),
            digits: rule.get("digits").and_then(|d| d.as_u64()).unwrap_or(1) as usize,
            mark: json::string_of(rule, "mark")?.to_owned(),
            placeholder: json::string_of(rule, "placeholder").map(str::to_owned),
        })
    }

    /// The id written with the placeholder, `REQ-NEW`.
    pub fn placeholder_id(&self) -> Option<String> {
        self.placeholder.as_ref().map(|p| format!("{}{p}", self.prefix))
    }

    /// The number of `id`, when it is the prefix followed by digits only.
    pub fn number(&self, id: &str) -> Option<u64> {
        let digits = id.strip_prefix(&self.prefix)?;
        (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
            .then(|| digits.parse().ok())
            .flatten()
    }

    /// The id of number `n`, padded to `digits`.
    pub fn id(&self, n: u64) -> String {
        format!("{}{n:0width$}", self.prefix, width = self.digits)
    }

    /// The highest number given so far in `diagram`: the mark's, or of any id with the prefix.
    pub fn highest(&self, diagram: &DislDiagram) -> u64 {
        let mark = match diagram.value_of(&self.mark) {
            Value::String(id) => self.number(&id).unwrap_or(0),
            _ => 0,
        };
        diagram
            .elements()
            .iter()
            .filter_map(|e| self.number(&e.id()))
            .chain(std::iter::once(mark))
            .max()
            .unwrap_or(0)
    }
}

impl Counter {
    /// How many elements of `diagram` wait for a number of this counter.
    pub fn waiting(&self, specification: &DislSpecification, diagram: &DislDiagram) -> u64 {
        diagram
            .elements()
            .iter()
            .filter(|e| is_placeholder(e) && Counter::of(specification, &e.type_name()).as_ref() == Some(self))
            .count() as u64
    }

    /// The id a new element of this counter gets in `diagram`: past the highest given and past every number the
    /// elements waiting for one get first, in reading order.
    pub fn next(&self, specification: &DislSpecification, diagram: &DislDiagram) -> String {
        self.id(self.highest(diagram) + self.waiting(specification, diagram) + 1)
    }
}

/// Whether `element` was read with its counter's placeholder, so that it is named `line/<line>` (DESL 0.3).
pub fn is_placeholder(element: &DislElement) -> bool {
    element.id().starts_with("line/") && element.read_id().is_some()
}

/// The elements read with a placeholder, in reading order, each with the id it gets with the next edit, and the
/// mark of each counter after them.
pub fn numbering(specification: &DislSpecification, diagram: &DislDiagram) -> Vec<(DislElement, String, Counter)> {
    let mut waiting: Vec<DislElement> = diagram.elements().into_iter().filter(is_placeholder).collect();
    waiting.sort_by_key(|e| e.line().unwrap_or(i32::MAX));
    let mut next: Vec<(Counter, u64)> = Vec::new();
    let mut numbered = Vec::new();
    for element in waiting {
        let Some(counter) = Counter::of(specification, &element.type_name()) else {
            continue;
        };
        let at = match next.iter().position(|(c, _)| *c == counter) {
            Some(at) => at,
            None => {
                next.push((counter.clone(), counter.highest(diagram)));
                next.len() - 1
            }
        };
        next[at].1 += 1;
        numbered.push((element, counter.id(next[at].1), counter));
    }
    numbered
}

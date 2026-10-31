//! A case recorded from a module that opens its own documents, compared by what it means (ATD §2.3, §2.4; etalii.adp
//! spec 067, FR-012). Such a case states its reading and its drawing in the module's own terms, so they are compared
//! here rather than exactly: a reading by its ids, parents, ends and values, a drawing by which elements appear and
//! where. Nothing here decides anything for bytes, splices, findings or refusals, nor for a case from a declared tool.

use std::collections::{BTreeMap, BTreeSet};

use adp_disl::DislSpecification;
use serde_json::{Map, Value as Json};

/// A list of elements or relations by its ids.
fn by_id(items: &Json) -> BTreeMap<&str, &Json> {
    items
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| (item["id"].as_str().unwrap_or_default(), item))
        .collect()
}

/// The ids of what a drawing draws: its elements and its edges.
pub fn drawn_ids(drawing: &Json) -> BTreeSet<String> {
    ["elements", "edges"]
        .iter()
        .flat_map(|part| drawing[*part].as_array().into_iter().flatten())
        .filter_map(|item| item["id"].as_str().map(str::to_owned))
        .collect()
}

/// What differs between an element or relation the case states and the one read: `(expected, actual, the id)`.
pub type Unlike = (String, String, String);

/// What is compared of one list of a reading.
pub struct Reading<'a> {
    /// What the case states.
    pub expected: &'a Json,
    /// What the description read.
    pub actual: &'a Json,
    /// What the case's own drawing draws; none when the case states no drawing, and then nothing it states is the
    /// module's own presentation.
    pub case_draws: Option<&'a BTreeSet<String>>,
    /// What the description's drawing draws; none when it draws nothing, which the drawing's own comparison reports.
    pub description_draws: Option<&'a BTreeSet<String>>,
    /// The attributes the case states anywhere in its reading.
    pub stated: &'a BTreeSet<&'a str>,
}

/// The attributes a case states anywhere in its reading: of its elements and its relations.
pub fn stated_by(read: &Json) -> BTreeSet<&str> {
    ["elements", "relations"]
        .iter()
        .flat_map(|part| read[*part].as_array().into_iter().flatten())
        .flat_map(|item| item["attributes"].as_object().into_iter().flat_map(Map::keys))
        .map(String::as_str)
        .collect()
}

/// The attributes both sides can state: those the case states anywhere in its reading that the description's model
/// declares on `type_name`, not derived.
fn comparable(specification: &DislSpecification, type_name: &str, stated: &BTreeSet<&str>) -> BTreeSet<String> {
    let Some(declared) = specification.metamodel().type_of(type_name) else {
        return BTreeSet::new();
    };
    declared
        .attributes
        .values()
        .filter(|attribute| !attribute.is_derived() && stated.contains(attribute.name.as_str()))
        .map(|attribute| attribute.name.clone())
        .collect()
}

/// One attribute as a reading states it: the text, an absent one the empty text.
fn text_of<'a>(attributes: &'a Json, name: &str) -> &'a str {
    attributes[name].as_str().unwrap_or_default()
}

/// Every difference of meaning between what a case states of its reading and what was read.
pub fn reading_differences(specification: &DislSpecification, reading: &Reading) -> Vec<Unlike> {
    let (expected, actual) = (by_id(reading.expected), by_id(reading.actual));
    let mut differences = Vec::new();
    let mut compared_any = false;
    for (id, want) in &expected {
        let Some(got) = actual.get(id) else {
            // What the module presents and draws no element for, such as what stands for the view it shows.
            let presented = reading.case_draws.is_some_and(|drawn| !drawn.contains(*id));
            if !presented {
                differences.push(((*id).to_owned(), "nothing".to_owned(), (*id).to_owned()));
            }
            continue;
        };
        for key in ["parent", "source", "target"] {
            if want.get(key).is_some() && want[key] != got[key] {
                differences.push((
                    format!("{id} {key}: {}", want[key]),
                    format!("{id} {key}: {}", got[key]),
                    (*id).to_owned(),
                ));
            }
        }
        let names = comparable(specification, got["type"].as_str().unwrap_or_default(), reading.stated);
        compared_any |= !names.is_empty();
        for name in names {
            let (w, g) = (text_of(&want["attributes"], &name), text_of(&got["attributes"], &name));
            if w != g {
                differences.push((
                    format!("{id} {name}: {w:?}"),
                    format!("{id} {name}: {g:?}"),
                    (*id).to_owned(),
                ));
            }
        }
    }
    // What the case says of these and the description can state nothing of: nothing would be compared.
    if !compared_any
        && let Some(id) = expected
            .iter()
            .find(|(id, item)| {
                actual.contains_key(*id) && item["attributes"].as_object().is_some_and(|a| !a.is_empty())
            })
            .map(|(id, _)| id)
    {
        differences.push((
            format!(
                "attributes in common with {}",
                reading.stated.iter().copied().collect::<Vec<_>>().join(", ")
            ),
            "none".to_owned(),
            (*id).to_owned(),
        ));
    }
    for (id, _) in actual.iter().filter(|(id, _)| !expected.contains_key(*id)) {
        if reading.description_draws.is_some_and(|drawn| drawn.contains(*id)) {
            differences.push(("nothing".to_owned(), (*id).to_owned(), (*id).to_owned()));
        }
    }
    differences
}

/// The first difference of meaning between two drawings, as ATD §2.4 compares one recorded from a module: which
/// elements appear and where, and which edges appear between which ends; with the id of the element or edge that
/// differs. Nothing else of a drawing is the description's to match.
pub fn drawing_difference(expected: &Json, actual: &Json) -> Option<(String, String, String)> {
    let first = |part: &str, what: &dyn Fn(&Json) -> Json| {
        let keyed = |drawing: &Json| -> BTreeMap<String, Json> {
            drawing[part]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| (item["id"].as_str().unwrap_or_default().to_owned(), what(item)))
                .collect()
        };
        let (want, got) = (keyed(expected), keyed(actual));
        let ids: BTreeSet<&String> = want.keys().chain(got.keys()).collect();
        ids.into_iter().find_map(|id| {
            let (w, g) = (want.get(id), got.get(id));
            let show = |v: Option<&Json>| v.map_or("nothing".to_owned(), Json::to_string);
            (w != g).then(|| {
                (
                    format!("{part} {id}: {}", show(w)),
                    format!("{part} {id}: {}", show(g)),
                    id.clone(),
                )
            })
        })
    };
    first("elements", &|item| item["bounds"].clone()).or_else(|| {
        first(
            "edges",
            &|item| serde_json::json!({"source": item["source"], "target": item["target"]}),
        )
    })
}

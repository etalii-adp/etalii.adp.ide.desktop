//! The operations on stored filters of `hierarchical-requirement.des` (the standalone host's `FilterOperations` at
//! a7fdf03; FR-026, FR-048): `storeFilter` keeps the applied filter with its sorting under a name, `renameFilter` and
//! `removeFilter` change a stored one. They write the registration's `filters` block alone, as one step of the
//! history, byte for byte as the conformance fixture `hierarchical-requirement-filters` states; applying a filter or a
//! sort writes nothing (FR-049).

use std::collections::BTreeMap;

use adp_fbl::Setting;

use super::lists::{ListEdit, ListFiles};
use super::registration;
use crate::body::RequirementBody;
use crate::gestures::DRIFTED;

pub const STORE_FILTER: &str = "storeFilter";
pub const RENAME_FILTER: &str = "renameFilter";
pub const REMOVE_FILTER: &str = "removeFilter";

const FILTERS_BLOCK: &str = "filters";
/// The prefix of a filter's id, `'filter/' + self.name` (hierarchical-requirement.des, ids).
const FILTER_ID_PREFIX: &str = "filter/";

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    [STORE_FILTER, RENAME_FILTER, REMOVE_FILTER].contains(&operation)
}

/// Whether `operation` targets a stored filter rather than the document.
pub fn targets_a_filter(operation: &str) -> bool {
    operation == RENAME_FILTER || operation == REMOVE_FILTER
}

/// Whether `params` can be written by `operation`: the sentence saying why not, or empty.
pub fn refusal(operation: &str, params: &BTreeMap<String, String>) -> String {
    if operation != STORE_FILTER && operation != RENAME_FILTER {
        return String::new();
    }
    let blank = |name: &str| params.get(name).is_none_or(|v| v.trim().is_empty());
    if blank("name") {
        return "A filter needs a name.".into();
    }
    if operation == STORE_FILTER && blank("query") {
        return "There is no filter to store.".into();
    }
    String::new()
}

/// The registration before and after `operation`; or the sentence saying why it cannot be written, in which case
/// nothing changed. The body is never written. A filter the registration no longer holds is refused as a change made
/// against an older file.
pub fn apply(
    files: ListFiles<'_>,
    operation: &str,
    target_id: &str,
    params: &BTreeMap<String, String>,
) -> Result<ListEdit, String> {
    let refused = refusal(operation, params);
    if !refused.is_empty() {
        return Err(refused);
    }
    let document = RequirementBody::read(
        files.body.to_vec(),
        files.body_name,
        files.registration.map(|r| (r.to_vec(), files.registration_name)),
    )
    .document;
    let trimmed = |name: &str| params.get(name).map_or("", String::as_str).trim().to_owned();
    let setting = if operation == STORE_FILTER {
        Setting {
            block: FILTERS_BLOCK.into(),
            key: trimmed("name"),
            value: Some(trimmed("query")),
            ..Setting::default()
        }
    } else {
        let Some(stored) = document
            .filters
            .iter()
            .find(|f| format!("{FILTER_ID_PREFIX}{}", f.name) == target_id)
        else {
            return Err(DRIFTED.into());
        };
        let kept = Setting {
            block: FILTERS_BLOCK.into(),
            key: stored.name.clone(),
            ..Setting::default()
        };
        if operation == RENAME_FILTER {
            Setting {
                rename: Some(trimmed("name")),
                ..kept
            }
        } else {
            Setting {
                remove_entry: true,
                ..kept
            }
        }
    };
    let (before, after) = registration::apply(files, &document, setting)?;
    Ok(ListEdit {
        body: None,
        registration_before: Some(before),
        registration_after: Some(after),
    })
}

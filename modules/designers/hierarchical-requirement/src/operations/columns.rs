//! The operations on the widths of the columns of items of `hierarchical-requirement.des` (the standalone host's
//! `ColumnOperations` at a7fdf03; DESL 0.2 section 7.7.1; etalii.adp spec 036, T051): `addColumnWidth` gives a column
//! its first width, `setColumnWidth` changes it, and `resetColumnWidth` gives it the host's width again. They write the
//! registration's `widths` block alone, as one step of the history, the gesture being the end of a drag of the
//! column's right edge.

use std::collections::BTreeMap;

use adp_fbl::Setting;

use super::lists::{ListEdit, ListFiles};
use super::registration;
use crate::body::RequirementBody;
use crate::gestures::DRIFTED;
use crate::mapper::COLUMN_PREFIX;
use crate::model::{MAXIMUM_WIDTH, MINIMUM_WIDTH};

pub const ADD_COLUMN_WIDTH: &str = "addColumnWidth";
pub const SET_COLUMN_WIDTH: &str = "setColumnWidth";
pub const RESET_COLUMN_WIDTH: &str = "resetColumnWidth";

const WIDTHS_BLOCK: &str = "widths";

/// Whether `operation` is one of these.
pub fn handles(operation: &str) -> bool {
    [ADD_COLUMN_WIDTH, SET_COLUMN_WIDTH, RESET_COLUMN_WIDTH].contains(&operation)
}

/// Whether `operation` targets a stored width rather than the document.
pub fn targets_a_column(operation: &str) -> bool {
    operation == SET_COLUMN_WIDTH || operation == RESET_COLUMN_WIDTH
}

/// The whole number `text` is, digits only, or 0.
fn number(text: Option<&String>) -> u32 {
    text.filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|t| t.parse().ok())
        .unwrap_or(0)
}

/// Whether `params` can be written by `operation`: the sentence saying why not, or empty.
pub fn refusal(operation: &str, params: &BTreeMap<String, String>) -> String {
    if operation != ADD_COLUMN_WIDTH && operation != SET_COLUMN_WIDTH {
        return String::new();
    }
    if operation == ADD_COLUMN_WIDTH && number(params.get("level")) < 1 {
        return "A column is named by its depth, a whole number from 1.".into();
    }
    if (MINIMUM_WIDTH..=MAXIMUM_WIDTH).contains(&number(params.get("width"))) {
        String::new()
    } else {
        format!("A column is {MINIMUM_WIDTH} to {MAXIMUM_WIDTH} pixels wide.")
    }
}

/// The registration before and after `operation`; or the sentence saying why it cannot be written, in which case
/// nothing changed. The body is never written. A width the registration no longer holds is refused as a change made
/// against an older file, and so is a first width for a column that has one.
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
    let width = number(params.get("width")).to_string();
    let setting = if operation == ADD_COLUMN_WIDTH {
        let level = number(params.get("level"));
        if document.columns.iter().any(|c| c.level == level) {
            return Err(DRIFTED.into());
        }
        Setting {
            block: WIDTHS_BLOCK.into(),
            key: level.to_string(),
            value: Some(width),
            ..Setting::default()
        }
    } else {
        let Some(stored) = document
            .columns
            .iter()
            .find(|c| format!("{COLUMN_PREFIX}{}", c.level) == target_id)
        else {
            return Err(DRIFTED.into());
        };
        let kept = Setting {
            block: WIDTHS_BLOCK.into(),
            key: stored.level.to_string(),
            ..Setting::default()
        };
        if operation == SET_COLUMN_WIDTH {
            Setting {
                value: Some(width),
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

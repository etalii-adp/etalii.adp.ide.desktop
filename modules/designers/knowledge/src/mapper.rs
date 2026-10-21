//! A knowledge file onto the table model (the standalone host's `KnowledgeTableMapper` at 8b6ccfc): the DESL
//! runtime's mapper lays out the columns and the view, and this is where the Knowledge designer's own words - a
//! calculation, an option's uses, a relation's target, limit and other side - become a column's settings.

use std::collections::HashMap;

use adp_desl::table::content::{ContentRow, ViewColumn};
use adp_desl::table::engine::LabelOf;
use adp_desl::table::mapper;
use adp_desl::table::model::{Settings, TableFinding, TableFindingSeverity, TableOption, TableStructure};
use adp_fbl::FblModel;
use adp_fbl::model::{FindingSeverity, codes};

use crate::calculations;
use crate::model::{KnowledgeProperty, KnowledgeTable};
use crate::relations::{self, SELF, TargetOf};
use crate::validator;

/// The setting a column says under how many rows have one of its options: what deleting that option would clear.
pub const USES_PREFIX: &str = "uses:";

/// Everything about the table but its rows, for the view a connection shows. `in_sight` is the rows the view has
/// in sight, which the line under a column calculates over; none for every row of the table.
#[allow(clippy::too_many_arguments)]
pub fn structure(
    table: &KnowledgeTable,
    read_only_reason: &str,
    view_id: &str,
    row_count: usize,
    target_of: Option<TargetOf<'_>>,
    in_sight: Option<&[&ContentRow]>,
    label_of: Option<LabelOf<'_>>,
    search: &str,
) -> TableStructure {
    let uses = uses(table);
    let all: Vec<&ContentRow>;
    let in_sight = match in_sight {
        Some(rows) => rows,
        None => {
            all = table.rows.iter().collect();
            &all
        }
    };
    let extras = |property: &KnowledgeProperty, column: Option<&ViewColumn>, settings: &mut Settings| {
        column_settings(table, property, column, settings, target_of, &uses, in_sight)
    };
    mapper::structure(
        table,
        read_only_reason,
        view_id,
        row_count,
        Some(&extras),
        label_of,
        search,
    )
}

/// How many rows have each option of each selection, counted once for the whole table.
fn uses(table: &KnowledgeTable) -> HashMap<(String, String), usize> {
    let mut uses = HashMap::new();
    for cell in table
        .rows
        .iter()
        .flat_map(|r| r.cells.iter())
        .filter(|c| c.key == "option" || c.key == "options")
    {
        let mut seen = Vec::new();
        for value in &cell.values {
            if !seen.contains(&value) {
                seen.push(value);
                *uses.entry((cell.column_id.clone(), value.clone())).or_insert(0) += 1;
            }
        }
    }
    uses
}

fn column_settings(
    table: &KnowledgeTable,
    property: &KnowledgeProperty,
    column: Option<&ViewColumn>,
    settings: &mut Settings,
    target_of: Option<TargetOf<'_>>,
    uses: &HashMap<(String, String), usize>,
    in_sight: &[&ContentRow],
) -> Option<Vec<TableOption>> {
    // The calculations a column of this type can be given, what this one is given, and what it comes to.
    settings.insert(
        "calculations".into(),
        calculations::for_type(&property.value_type).join(","),
    );
    if let Some(column) = column.filter(|c| !c.calculate.is_empty()) {
        settings.insert("calculate".into(), column.calculate.clone());
        settings.insert(
            "calculation".into(),
            calculations::of(property, &column.calculate, in_sight),
        );
    }
    if !property.target_file.is_empty() {
        settings.insert("targetFile".into(), property.target_file.clone());
    }
    // An option some rows have says how many: its author is told before deleting it clears them.
    for option in &property.options {
        if let Some(count) = uses.get(&(property.id.clone(), option.id.clone())).filter(|c| **c > 0) {
            settings.insert(format!("{USES_PREFIX}{}", option.id), count.to_string());
        }
    }
    if property.value_type != "relation" {
        return None;
    }
    settings.insert("limit".into(), property.limit.clone());
    if property.is_computed {
        settings.insert("computed".into(), "true".into());
    }
    if property.is_parent {
        settings.insert("parent".into(), "true".into());
    }
    let itself = property.target_file == SELF || property.target_file.is_empty();
    // A relation to the table itself that holds one row can be the one its rows are nested by.
    if itself && property.limit == "one" && !property.is_computed {
        settings.insert("nestable".into(), "true".into());
    }
    let target = target_of.and_then(|t| t(property));
    // The other side of a two-way relation, by its name: deleting this side asks what becomes of it.
    let related = if itself {
        Some(table)
    } else {
        target.as_ref().and_then(|t| t.table.as_ref())
    };
    if !property.counterpart.is_empty()
        && let Some(other) = related.and_then(|r| r.column(&property.counterpart))
    {
        settings.insert("otherSide".into(), other.name.clone());
    }
    // What a relation offers to choose from is the rows of its target, by their titles.
    match target.as_ref().and_then(|t| t.table.as_ref()) {
        Some(target) => Some(
            target
                .rows
                .iter()
                .map(|row| TableOption {
                    id: row.id.clone(),
                    name: relations::title_of(target, row),
                    color: String::new(),
                })
                .collect(),
        ),
        None => {
            if target_of.is_some() {
                settings.insert("unresolved".into(), "true".into());
            }
            None
        }
    }
}

/// What the body's reading reported, as the table's findings. A key the bindings do not read is the designer's own
/// finding: kept, and said.
pub fn findings(model: &FblModel) -> Vec<TableFinding> {
    model
        .findings
        .iter()
        .map(|finding| TableFinding {
            code: if finding.code == codes::UNBOUND_KEY {
                validator::UNKNOWN_KEY.into()
            } else {
                finding.code.clone()
            },
            severity: match finding.severity {
                FindingSeverity::Error => TableFindingSeverity::Error,
                FindingSeverity::Warning => TableFindingSeverity::Warning,
                FindingSeverity::Info => TableFindingSeverity::Info,
            },
            message: match &finding.location {
                Some(at) => format!("{} (line {})", finding.message, at.line),
                None => finding.message.clone(),
            },
            ..TableFinding::default()
        })
        .collect()
}

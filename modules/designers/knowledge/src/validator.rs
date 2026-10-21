//! What a knowledge file has that the designer cannot accept as it is, each as a finding at its place: one code per
//! condition of the specification's list (`knowledge.md`, *Findings*; the standalone host's `KnowledgeValidator` at
//! 8b6ccfc).
//!
//! - **Reading never fails on content, and reporting never changes it.** Everything readable is shown; what is
//!   reported stays in the file exactly as it is, to be put right by its author.
//! - **Every condition is looked for, every time.** A file with two things wrong reports two.

use std::collections::{HashMap, HashSet};

use adp_desl::table::model::{TableFinding, TableFindingSeverity};
use adp_fbl::FblModel;

use crate::model::{KnowledgeProperty, KnowledgeTable};
use crate::relations::TargetOf;
use crate::vocabulary;

pub const NO_TITLE: &str = "knowledge.no-title";
pub const SEVERAL_TITLES: &str = "knowledge.several-titles";
pub const TITLE_NOT_TEXT: &str = "knowledge.title-not-text";
pub const DUPLICATE_PROPERTY_NAME: &str = "knowledge.duplicate-property-name";
pub const UNKNOWN_PROPERTY: &str = "knowledge.unknown-property";
pub const VALUE_OF_ANOTHER_TYPE: &str = "knowledge.value-of-another-type";
pub const UNKNOWN_OPTION: &str = "knowledge.unknown-option";
pub const RELATION_OVER_LIMIT: &str = "knowledge.relation-over-limit";
pub const VIEW_NAMES_MISSING_PROPERTY: &str = "knowledge.view-names-missing-property";
pub const NO_VIEW: &str = "knowledge.no-view";
pub const UNKNOWN_TYPE: &str = "knowledge.unknown-type";
pub const PARENT_CYCLE: &str = "knowledge.parent-cycle";
pub const MISSING_ID: &str = "knowledge.missing-id";
pub const UNRESOLVED_TARGET: &str = "knowledge.unresolved-target";
pub const UNKNOWN_KEY: &str = "knowledge.unknown-key";
pub const NEWER_VERSION: &str = "knowledge.newer-version";
pub const UNRESOLVED_ROW: &str = "knowledge.unresolved-row";

/// The element types whose entries carry an id of their own in the file.
pub const IDENTIFIED: [&str; 4] = ["Property", "Option", "View", "Row"];

struct Findings(Vec<TableFinding>);

impl Findings {
    fn error(&mut self, code: &str, message: String, row_id: &str, column_id: &str) {
        self.0
            .push(TableFinding::new(code, TableFindingSeverity::Error, message).at(row_id, column_id));
    }

    fn warn(&mut self, code: &str, message: String, row_id: &str, column_id: &str) {
        self.0
            .push(TableFinding::new(code, TableFindingSeverity::Warning, message).at(row_id, column_id));
    }
}

/// What the designer's rules find in `table`, read from `model`; with `target_of`, what its relations point at,
/// and without it nothing is said of them.
pub fn validate(table: &KnowledgeTable, model: &FblModel, target_of: Option<TargetOf<'_>>) -> Vec<TableFinding> {
    let mut findings = Findings(Vec::new());
    let mut properties: HashMap<&str, &KnowledgeProperty> = HashMap::new();
    for property in &table.columns {
        properties.entry(property.id.as_str()).or_insert(property);
    }

    // ---- the table and its properties ----

    let titles: Vec<&KnowledgeProperty> = table.columns.iter().filter(|p| p.is_title).collect();
    if titles.is_empty() && (!table.columns.is_empty() || !table.rows.is_empty()) {
        findings.warn(
            NO_TITLE,
            "The table has no title property, so its rows have no name.".into(),
            "",
            "",
        );
    }
    for extra in titles.iter().skip(1) {
        findings.warn(
            SEVERAL_TITLES,
            format!(
                "'{}' is marked as the title, and so is '{}'; the first is used.",
                extra.name, titles[0].name
            ),
            "",
            &extra.id,
        );
    }
    for title in titles.iter().filter(|t| t.value_type != "text") {
        findings.error(
            TITLE_NOT_TEXT,
            format!(
                "The title property '{}' is a {}; a title is text.",
                title.name, title.value_type
            ),
            "",
            &title.id,
        );
    }
    let mut names = HashSet::new();
    for property in &table.columns {
        if !names.insert(property.name.to_uppercase()) {
            findings.error(
                DUPLICATE_PROPERTY_NAME,
                format!("Another property is already called '{}'.", property.name),
                "",
                &property.id,
            );
        }
        if !vocabulary::VALUE_TYPES.contains(&property.value_type.as_str()) {
            findings.warn(
                UNKNOWN_TYPE,
                format!(
                    "'{}' has the type '{}', which is none this application knows; its values are shown and cannot be changed here.",
                    property.name, property.value_type
                ),
                "",
                &property.id,
            );
        }
    }

    // ---- the cells ----

    for row in &table.rows {
        for cell in &row.cells {
            let Some(property) = properties.get(cell.column_id.as_str()) else {
                findings.warn(
                    UNKNOWN_PROPERTY,
                    format!(
                        "This cell names a property the table does not have ('{}'); it is kept as it is.",
                        cell.column_id
                    ),
                    &row.id,
                    &cell.column_id,
                );
                continue;
            };
            if cell.values.is_empty() || !vocabulary::VALUE_TYPES.contains(&property.value_type.as_str()) {
                continue;
            }
            if cell.key != vocabulary::key_of(&property.value_type) {
                findings.warn(
                    VALUE_OF_ANOTHER_TYPE,
                    format!("This value is not a {}; it is kept as it is.", property.value_type),
                    &row.id,
                    &property.id,
                );
                continue;
            }
            if matches!(property.value_type.as_str(), "selection" | "multipleSelection") {
                for value in cell
                    .values
                    .iter()
                    .filter(|v| property.options.iter().all(|o| o.id != **v))
                {
                    findings.warn(
                        UNKNOWN_OPTION,
                        format!(
                            "This value names an option '{}' does not have ('{value}'); it is kept as it is.",
                            property.name
                        ),
                        &row.id,
                        &property.id,
                    );
                }
            }
            if property.value_type == "relation" && property.limit == "one" && cell.values.len() > 1 {
                findings.warn(
                    RELATION_OVER_LIMIT,
                    format!(
                        "'{}' relates to one row, and this cell names {}; they are kept as they are.",
                        property.name,
                        cell.values.len()
                    ),
                    &row.id,
                    &property.id,
                );
            }
        }
    }

    // ---- relations: a target that is not there, and a row that is not in it. The value stays either way. ----

    if let Some(target_of) = target_of {
        for relation in table.columns.iter().filter(|p| p.value_type == "relation") {
            let target = target_of(relation);
            let Some(related) = target.as_ref().and_then(|t| t.table.as_ref()) else {
                let problem = target
                    .as_ref()
                    .map(|t| t.problem.clone())
                    .unwrap_or_else(|| "it names no file.".into());
                findings.warn(
                    UNRESOLVED_TARGET,
                    format!(
                        "'{}' relates to a table that cannot be shown: {problem} Its values are kept as they are.",
                        relation.name
                    ),
                    "",
                    &relation.id,
                );
                continue;
            };
            if relation.is_computed {
                continue;
            }
            let there: HashSet<&str> = related.rows.iter().map(|r| r.id.as_str()).collect();
            let name = if related.name.is_empty() {
                "the table it relates to"
            } else {
                related.name.as_str()
            };
            for row in &table.rows {
                let Some(cell) = row.cells.iter().find(|c| c.column_id == relation.id && c.key == "rows") else {
                    continue;
                };
                for value in cell.values.iter().filter(|v| !there.contains(v.as_str())) {
                    findings.warn(
                        UNRESOLVED_ROW,
                        format!("This value names a row that is not in {name} ('{value}'); it is kept as it is."),
                        &row.id,
                        &relation.id,
                    );
                }
            }
        }
    }

    // ---- the views ----

    if table.views.is_empty() {
        findings.warn(
            NO_VIEW,
            "The table has no view; it is shown as if it had one that changes nothing.".into(),
            "",
            "",
        );
    }
    for view in &table.views {
        let named = view
            .columns
            .iter()
            .map(|c| c.column_id.as_str())
            .chain(view.sorts.iter().map(|s| s.column_id.as_str()))
            .chain(view.filter.conditions().into_iter().map(|c| c.column_id.as_str()))
            .chain([view.group_by.as_str(), view.freeze.as_str()]);
        let mut said = HashSet::new();
        for id in named.filter(|id| !id.is_empty() && !properties.contains_key(id)) {
            if said.insert(id) {
                findings.warn(
                    VIEW_NAMES_MISSING_PROPERTY,
                    format!(
                        "The view '{}' has a setting for a property the table does not have ('{id}'); the view opens without it.",
                        view.name
                    ),
                    "",
                    "",
                );
            }
        }
    }

    // ---- rows that are their own ancestors ----

    if let Some(parent) = table.parent_column() {
        let mut under: HashMap<&str, &str> = HashMap::new();
        for row in &table.rows {
            if let Some(cell) = row.cells.iter().find(|c| c.column_id == parent.id)
                && cell.key == "rows"
                && let Some(first) = cell.values.first()
            {
                under.entry(row.id.as_str()).or_insert(first.as_str());
            }
        }
        for row in table.rows.iter().filter(|r| under.contains_key(r.id.as_str())) {
            // Walk up from the row; coming back to it within as many steps as there are rows is a circle.
            let mut at = under[row.id.as_str()];
            let mut steps = 0;
            while steps <= under.len() && at != row.id {
                let Some(next) = under.get(at) else { break };
                at = next;
                steps += 1;
            }
            if at == row.id {
                findings.error(
                    PARENT_CYCLE,
                    format!("This row is its own ancestor through '{}'.", parent.name),
                    &row.id,
                    &parent.id,
                );
            }
        }
    }

    // ---- entries without an id ----

    for element in model
        .elements
        .iter()
        .filter(|e| !e.id_is_stored && IDENTIFIED.contains(&e.type_name.as_str()))
    {
        findings.0.push(
            TableFinding::new(
                MISSING_ID,
                TableFindingSeverity::Info,
                format!(
                    "A {} on line {} has no id; it is shown, and anything that names it cannot until it has one.",
                    element.type_name.to_lowercase(),
                    element.line
                ),
            )
            .at(
                if element.type_name == "Row" {
                    element.id.as_str()
                } else {
                    ""
                },
                if element.type_name == "Property" {
                    element.id.as_str()
                } else {
                    ""
                },
            ),
        );
    }
    findings.0
}

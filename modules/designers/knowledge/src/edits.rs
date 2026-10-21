//! The table's gestures as changes of the file (the standalone host's `KnowledgeEdits` at 8b6ccfc): one place that
//! knows what each gesture means for a knowledge file, what it refuses and in which words. It reads a table and
//! writes nothing; the changes it gives are made to the file through the FBL runtime ([`crate::body`]) and to what
//! the table shows through [`crate::projection`], so the two cannot mean different things.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use adp_cel::{Map, Value};
use adp_desl::table::content::{ContentOption, ContentView, FilterGroup, FilterItem};
use adp_desl::table::engine::NO_VALUE;
use adp_desl::table::groups;
use adp_desl::table::model::TableGesture;
use adp_desl::table::values::{self, parse_date, parse_date_time, parse_time};
use adp_fbl::ModelChange;
use regex::Regex;

use crate::calculations;
use crate::definition;
use crate::model::{KnowledgeProperty, KnowledgeTable};
use crate::relations::{self, KnowledgeTarget, SELF};
use crate::vocabulary;

const ROW_GONE: &str = "That row is no longer in this table.";
const PROPERTY_GONE: &str = "That property is no longer in this table.";
const VIEW_GONE: &str = "That view is no longer in this table.";
pub const NAME_NEEDED: &str = "A property needs a name.";
const VIEW_NAME_NEEDED: &str = "A view needs a name.";
pub const TITLE_STAYS: &str = "The title property is always there: it cannot be deleted or hidden.";
pub const LAST_VIEW: &str = "A table keeps at least one view.";
pub const NUMBER_EXPECTED: &str = "A number is expected.";
pub const DATE_EXPECTED: &str = "A date is expected, as year-month-day.";
pub const DATE_TIME_EXPECTED: &str = "A date and a time are expected.";
pub const TIME_EXPECTED: &str = "A time is expected, as hours:minutes.";
const COMPUTED: &str = "This side of the relation is filled in from the other table.";
pub const RELATION_NEEDS_TARGET: &str = "A relation is added by choosing the table it relates to.";
pub const TITLE_IS_TEXT: &str = "The title property is always text.";
pub const OPTION_FIRST: &str = "Make the option in one row first; it can then be given to several.";
pub const SORTED_ROWS_STAY: &str = "Rows are in the order of this view's sort, so they cannot be moved here.";
pub const OTHER_SIDE_CHOICE: &str =
    "This relation has another side: choose whether that is deleted too or kept as a one-way relation.";
pub const PARENT_CYCLE: &str = "A row cannot be its own ancestor.";
pub const NOT_A_RELATION: &str = "That property is not a relation of this table's own.";
pub const PARENT_HOLDS_ONE: &str = "The parent relation holds one row.";
const NOT_A_SELECTION: &str = "Only a selection has options.";
const OPTION_GONE: &str = "That option is no longer one of this property's.";
pub const OPTION_NAME_NEEDED: &str = "An option needs a name.";
const NOT_GROUPED: &str = "This view is not grouped.";
const FILTER_GROUP_GONE: &str = "That group is no longer in this filter.";

/// The id the root of a knowledge file has: its rule stores none, and the designer derives this one.
pub const TABLE_ID: &str = "table";

/// The narrowest a column is, in pixels, as the specification has it.
const MINIMUM_WIDTH: i64 = 32;

/// How deep a filter's groups go: the bindings read a group inside a group and no further.
const FILTER_DEPTH: usize = 2;

/// What an answer starts with when it is a question to confirm and not a refusal: the gesture is made again,
/// confirmed.
const CONFIRM_PREFIX: &str = "confirm:";

/// What an answer starts with when it is a choice to make and not a refusal: the things to choose from follow.
const CHOOSE_PREFIX: &str = "choose:";

/// The changes one edit makes to one other knowledge file: the other side of a two-way relation.
#[derive(Debug, Clone)]
pub struct KnowledgeOtherFile {
    pub body_path: PathBuf,
    pub changes: Vec<ModelChange>,
}

/// What a gesture comes to: the changes to make to the file, in order, or why it is refused.
#[derive(Debug, Clone, Default)]
pub struct KnowledgeEdit {
    pub changes: Vec<ModelChange>,
    /// Why the gesture is refused, as a sentence for the author; empty when it is not.
    pub refusal: String,
    pub new_row_id: String,
    pub new_view_id: String,
    /// What the same step changes in other files: the other side of a two-way relation.
    pub others: Vec<KnowledgeOtherFile>,
}

impl KnowledgeEdit {
    pub fn is_refused(&self) -> bool {
        !self.refusal.is_empty()
    }

    fn refused(reason: impl Into<String>) -> Self {
        KnowledgeEdit {
            refusal: reason.into(),
            ..KnowledgeEdit::default()
        }
    }

    fn nothing() -> Self {
        KnowledgeEdit::default()
    }

    fn of(changes: Vec<ModelChange>) -> Self {
        KnowledgeEdit {
            changes,
            ..KnowledgeEdit::default()
        }
    }
}

/// What an edit may know of the files around the one it edits: where that file is, and what its relations point at.
pub struct KnowledgeSurroundings<'a> {
    pub body_path: &'a Path,
    /// What a relation property of the file points at.
    pub target_of: &'a dyn Fn(&KnowledgeProperty) -> Option<Rc<KnowledgeTarget>>,
    /// What a target named from the file - a relative path, or `.` for the file itself - is.
    pub target_at: &'a dyn Fn(&str) -> Rc<KnowledgeTarget>,
}

fn refused(reason: impl Into<String>) -> KnowledgeEdit {
    KnowledgeEdit::refused(reason)
}

fn flag(gesture: &TableGesture, name: &str) -> bool {
    gesture.setting_of(name) == "true"
}

fn text(value: impl Into<String>) -> Value {
    Value::String(value.into())
}

fn attributes(pairs: Vec<(&str, Value)>) -> Map {
    pairs.into_iter().collect()
}

fn set(id: impl Into<String>, pairs: Vec<(&str, Value)>) -> ModelChange {
    ModelChange::Set {
        id: id.into(),
        attributes: attributes(pairs),
    }
}

fn add(type_name: &str, id: Option<String>, pairs: Vec<(&str, Value)>, parent_id: Option<&str>) -> ModelChange {
    ModelChange::Add {
        type_name: type_name.into(),
        id,
        attributes: attributes(pairs),
        parent_id: parent_id.map(str::to_owned),
        index: None,
        slot: None,
    }
}

fn add_in(type_name: &str, pairs: Vec<(&str, Value)>, parent_id: &str, slot: &str) -> ModelChange {
    ModelChange::Add {
        type_name: type_name.into(),
        id: None,
        attributes: attributes(pairs),
        parent_id: Some(parent_id.to_owned()),
        index: None,
        slot: Some(slot.to_owned()),
    }
}

fn remove(id: impl Into<String>) -> ModelChange {
    ModelChange::Remove { id: id.into() }
}

fn cell_id(row_id: &str, property_id: &str) -> String {
    format!("{row_id}/{property_id}")
}

fn column_id(view: &ContentView, property_id: &str) -> String {
    format!("{}/columns/{property_id}", view.id)
}

fn sort_id(view: &ContentView, property_id: &str) -> String {
    format!("{}/sorts/{property_id}", view.id)
}

fn first_value(gesture: &TableGesture) -> String {
    gesture.values.first().map(|v| v.trim().to_owned()).unwrap_or_default()
}

/// A comparison by the name the file knows it by: the two every type has arrive under the table's names for them.
fn comparison(asked: &str) -> String {
    match asked {
        "isEmpty" => "is-empty".into(),
        "isNotEmpty" => "is-not-empty".into(),
        other => other.into(),
    }
}

/// `name`, or it with the lowest number that no other of `taken` has, ignoring case.
pub fn unique_name<'a>(taken: impl IntoIterator<Item = &'a str>, name: &str) -> String {
    let names: Vec<String> = taken.into_iter().map(str::to_uppercase).collect();
    if !names.contains(&name.to_uppercase()) {
        return name.to_owned();
    }
    (2..)
        .map(|number| format!("{name} {number}"))
        .find(|candidate| !names.contains(&candidate.to_uppercase()))
        .expect("a free number")
}

/// Moves one of `siblings` so that it ends at `to` among them. The runtime's index counts the siblings as they are
/// before the move, the moved one included, so a place further on is one more than where it ends.
fn move_to(id: &str, parent_id: Option<&str>, siblings: &[String], to: i64) -> KnowledgeEdit {
    let Some(from) = siblings.iter().position(|s| s == id) else {
        return KnowledgeEdit::nothing();
    };
    if siblings.is_empty() {
        return KnowledgeEdit::nothing();
    }
    let target = to.clamp(0, siblings.len() as i64 - 1) as usize;
    if from == target {
        return KnowledgeEdit::nothing();
    }
    KnowledgeEdit::of(vec![ModelChange::Move {
        id: id.into(),
        new_parent_id: parent_id.map(str::to_owned),
        index: if target > from {
            target as i64 + 1
        } else {
            target as i64
        },
    }])
}

/// Something added at a place among its siblings. A declared binding adds at the end and nowhere else, so a place
/// is two changes of the one step: added last, then moved to where it belongs.
fn add_at(
    type_name: &str,
    id: Option<String>,
    pairs: Vec<(&str, Value)>,
    parent_id: Option<&str>,
    index: Option<usize>,
    count: usize,
    added_id: &str,
) -> Vec<ModelChange> {
    let mut changes = vec![add(type_name, id, pairs, parent_id)];
    if let Some(at) = index.filter(|at| *at < count) {
        changes.push(ModelChange::Move {
            id: added_id.into(),
            new_parent_id: parent_id.map(str::to_owned),
            index: at as i64,
        });
    }
    changes
}

/// The question put before rows are deleted that rows of other tables relate to.
pub fn ask_first(relating: usize, deleted: usize) -> String {
    format!(
        "{CONFIRM_PREFIX}{} to {}. Deleting {} leaves {} pointing at nothing.",
        if relating == 1 {
            "1 row in another table relates".to_owned()
        } else {
            format!("{relating} rows in other tables relate")
        },
        if deleted == 1 { "this row" } else { "these rows" },
        if deleted == 1 { "it" } else { "them" },
        if relating == 1 {
            "that relation"
        } else {
            "those relations"
        },
    )
}

/// The tables a new relation can relate to, as the answer to a relation asked for without one.
pub fn choose_target(tables: &[(String, String)]) -> String {
    let listed: Vec<serde_json::Value> = tables
        .iter()
        .map(|(id, label)| serde_json::json!({ "id": id, "label": label }))
        .collect();
    format!("{CHOOSE_PREFIX}{}", serde_json::Value::Array(listed))
}

/// Plans one gesture against `table`, as shown now, edits not yet written included. `active_view_id` is the view
/// the connection shows, for a gesture that names none; `new_id` gives an id that was never used; `beside_its_rule`
/// is whether the file's format adds an entry after the last one its rule reads; `files` are the files around this
/// one, for a gesture about a relation.
pub fn plan(
    table: &KnowledgeTable,
    active_view_id: &str,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
    beside_its_rule: bool,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    let named = if gesture.view_id.is_empty() {
        active_view_id
    } else {
        gesture.view_id.as_str()
    };
    let view = table.view(named).or_else(|| table.view_or_default(active_view_id));
    match gesture.kind.as_str() {
        "setCell" => return set_cell(table, gesture, new_id, files),
        "setCells" => return set_cells(table, gesture, new_id, files),
        "addRow" => return add_row(table, view, gesture, new_id),
        "deleteRows" => return delete_rows(table, gesture),
        "addColumn" => return add_column(table, view, gesture, new_id),
        "renameColumn" => return rename_column(table, gesture),
        "duplicateColumn" => return duplicate_column(table, view, gesture, new_id),
        "deleteColumn" => return delete_column(table, gesture, files),
        "setColumnType" => return change_type(table, gesture, new_id),
        "addRelation" => return add_relation(table, view, gesture, new_id, files),
        "setColumnParent" => return set_parent(table, gesture),
        "setRelationLimit" => return set_limit(table, gesture),
        "setRelationOtherSide" => return set_other_side(table, gesture, new_id, files),
        "addOption" => return add_option(table, gesture, new_id),
        "renameOption" => return for_option(table, gesture, |p, o| rename_option(p, o, gesture)),
        "recolourOption" => return for_option(table, gesture, |_, o| recolour_option(o, gesture)),
        "moveOption" => {
            return for_option(table, gesture, |p, o| {
                let ids: Vec<String> = p.options.iter().map(|c| c.id.clone()).collect();
                move_to(&o.id, Some(&p.id), &ids, gesture.index)
            });
        }
        "deleteOption" => return for_option(table, gesture, |p, o| delete_option(table, p, o)),
        "addView" => return add_view(table, new_id),
        "renameView" => return for_view(table, gesture, |v| rename_view(v, gesture)),
        "duplicateView" => return for_view(table, gesture, |v| duplicate_view(table, v, new_id)),
        "deleteView" => return for_view(table, gesture, |v| delete_view(table, v)),
        "moveView" => {
            return for_view(table, gesture, |v| {
                let ids: Vec<String> = table.views.iter().map(|c| c.id.clone()).collect();
                move_to(&v.id, None, &ids, gesture.index)
            });
        }
        _ => {}
    }
    let Some(view) = view else {
        return refused(VIEW_GONE);
    };
    match gesture.kind.as_str() {
        "moveRows" => move_rows(table, view, gesture),
        "moveColumn" => move_column(table, view, gesture),
        "resizeColumn" => match gesture.setting_of("width").trim().parse::<i32>() {
            Ok(width) if width > 0 => column_setting(
                table,
                view,
                &gesture.column_id,
                "width",
                Value::Int(MINIMUM_WIDTH.max(i64::from(width))),
            ),
            _ => refused("A width is a number of pixels."),
        },
        "hideColumn" => show_column(table, view, gesture, false),
        "showColumn" => show_column(table, view, gesture, true),
        "setColumnWrap" => column_setting(
            table,
            view,
            &gesture.column_id,
            "wrap",
            if flag(gesture, "wrap") {
                Value::Bool(true)
            } else {
                Value::Null
            },
        ),
        "setColumnCalculation" => calculate(table, view, gesture),
        "freezeColumns" => freeze(table, view, gesture),
        "addSort" => add_sort(table, view, gesture),
        "setSort" => {
            if view.sorts.iter().any(|s| s.column_id == gesture.column_id) {
                KnowledgeEdit::of(vec![set(
                    sort_id(view, &gesture.column_id),
                    vec![("direction", direction(gesture))],
                )])
            } else {
                KnowledgeEdit::nothing()
            }
        }
        "removeSort" => {
            if view.sorts.iter().any(|s| s.column_id == gesture.column_id) {
                KnowledgeEdit::of(vec![remove(sort_id(view, &gesture.column_id))])
            } else {
                KnowledgeEdit::nothing()
            }
        }
        "moveSort" => {
            if view.sorts.iter().any(|s| s.column_id == gesture.column_id) {
                let ids: Vec<String> = view.sorts.iter().map(|s| sort_id(view, &s.column_id)).collect();
                move_to(&sort_id(view, &gesture.column_id), Some(&view.id), &ids, gesture.index)
            } else {
                KnowledgeEdit::nothing()
            }
        }
        "groupBy" => group_by(table, view, gesture),
        "setHideEmptyGroups" => KnowledgeEdit::of(vec![set(
            view.id.clone(),
            vec![(
                "hideEmptyGroups",
                if flag(gesture, "hide") {
                    Value::Bool(true)
                } else {
                    Value::Null
                },
            )],
        )]),
        "toggleGroup" => collapse(view, &gesture.target_id, flag(gesture, "collapsed")),
        "toggleRow" => collapse(view, &gesture.row_id, flag(gesture, "collapsed")),
        "hideGroup" => {
            if gesture.target_id.is_empty() || view.hidden_groups.contains(&gesture.target_id) {
                KnowledgeEdit::nothing()
            } else {
                KnowledgeEdit::of(vec![add_in(
                    "GroupSetting",
                    vec![("key", text(gesture.target_id.clone()))],
                    &view.id,
                    "hiddenGroups",
                )])
            }
        }
        "showGroups" => KnowledgeEdit::of(
            view.hidden_groups
                .iter()
                .map(|key| remove(format!("{}/hiddenGroups/{key}", view.id)))
                .collect(),
        ),
        "showGroup" => {
            if view.hidden_groups.contains(&gesture.target_id) {
                KnowledgeEdit::of(vec![remove(format!("{}/hiddenGroups/{}", view.id, gesture.target_id))])
            } else {
                KnowledgeEdit::nothing()
            }
        }
        "moveGroup" => move_group(table, view, gesture),
        "sortGroups" => sort_groups(table, view, gesture, files),
        "addFilter" => add_filter(table, view, gesture, beside_its_rule),
        "addFilterGroup" => add_filter_group(view, gesture, beside_its_rule),
        "setFilter" => set_filter(table, view, gesture),
        "setFilterMatch" => set_filter_match(view, gesture),
        "removeFilter" => match filter_item(view, &gesture.target_id) {
            Some((id, _, _)) if !gesture.target_id.is_empty() => KnowledgeEdit::of(vec![remove(id)]),
            _ => KnowledgeEdit::nothing(),
        },
        _ => refused("This table does not know how to do that."),
    }
}

fn direction(gesture: &TableGesture) -> Value {
    if gesture.setting_of("direction") == "descending" {
        text("descending")
    } else {
        Value::Null
    }
}

// ---- cells and rows ----

/// One property set to the same value in several rows, as one step: what Notion does when the property of several
/// selected rows is edited. Any row that refuses the value refuses it for all.
fn set_cells(
    table: &KnowledgeTable,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    if gesture.settings.contains_key("newOption") {
        return refused(OPTION_FIRST);
    }
    let mut rows: Vec<&str> = Vec::new();
    for row in gesture
        .setting_of("rows")
        .split(',')
        .map(str::trim)
        .filter(|r| !r.is_empty())
    {
        if !rows.contains(&row) {
            rows.push(row);
        }
    }
    if rows.is_empty() {
        return refused(ROW_GONE);
    }
    let mut changes = Vec::new();
    for row in rows {
        let one = TableGesture::new("setCell")
            .row(row)
            .column(gesture.column_id.clone())
            .values(gesture.values.clone());
        let edit = set_cell(table, &one, new_id, files);
        if edit.is_refused() {
            return edit;
        }
        changes.extend(edit.changes);
    }
    KnowledgeEdit::of(changes)
}

fn set_cell(
    table: &KnowledgeTable,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    let Some(row) = table.row(&gesture.row_id) else {
        return refused(ROW_GONE);
    };
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    if property.is_computed {
        return refused(COMPUTED);
    }
    let mut changes = Vec::new();
    let mut values: Vec<String> = gesture.values.iter().filter(|v| !v.is_empty()).cloned().collect();
    let choice = matches!(property.value_type.as_str(), "selection" | "multipleSelection");

    // An option made while filling in the cell: made first, and then chosen like any other.
    let option_name = gesture.setting_of("newOption").trim().to_owned();
    if !option_name.is_empty() {
        if !choice {
            return refused(NOT_A_SELECTION);
        }
        let existing = property
            .options
            .iter()
            .find(|o| values::equals_ignoring_case(&o.name, &option_name));
        let option_id = match existing {
            Some(o) => o.id.clone(),
            None => {
                let id = new_id();
                changes.push(add(
                    "Option",
                    Some(id.clone()),
                    vec![
                        ("name", text(option_name.clone())),
                        ("colour", text(colour_for(property, &option_name))),
                    ],
                    Some(&property.id),
                ));
                id
            }
        };
        if property.value_type == "selection" {
            values = vec![option_id];
        } else if !values.contains(&option_id) {
            values.push(option_id);
        }
    } else if choice && values.iter().any(|v| property.options.iter().all(|o| o.id != *v)) {
        return refused(OPTION_GONE);
    }

    let cell_id = cell_id(&row.id, &property.id);
    let cell = row.cell(&property.id);

    if matches!(property.value_type.as_str(), "multipleSelection" | "relation") {
        if property.value_type == "relation" && property.limit == "one" && values.len() > 1 {
            return refused("This relation holds one row.");
        }
        if property.value_type == "relation" {
            // A related row is a row of the target: one that is not there is not related to.
            let target = files.and_then(|f| (f.target_of)(property));
            if let Some(target) = target.as_ref().and_then(|t| t.table.as_ref())
                && values.iter().any(|v| target.row(v).is_none())
            {
                return refused(format!(
                    "That row is not in {}.",
                    if target.name.is_empty() {
                        "the table this relates to"
                    } else {
                        &target.name
                    }
                ));
            }
            if property.is_parent && values.len() == 1 && relations::would_cycle(table, property, &row.id, &values[0]) {
                return refused(PARENT_CYCLE);
            }
        }
        if relations::is_mutual(property) {
            return set_mutual(table, &row.id, property, &values, changes);
        }
        let key = if property.value_type == "relation" {
            "row"
        } else {
            "option"
        };
        let held: Vec<String> = cell.map(|c| c.values.clone()).unwrap_or_default();
        if values.is_empty() {
            if cell.is_none() {
                return KnowledgeEdit::nothing();
            }
            changes.push(remove(cell_id));
            return KnowledgeEdit::of(changes);
        }
        if cell.is_none() {
            changes.push(add(
                "Cell",
                None,
                vec![("property", text(property.id.clone()))],
                Some(&row.id),
            ));
        }
        changes.extend(
            held.iter()
                .filter(|v| !values.contains(v))
                .map(|v| remove(format!("{cell_id}/{v}"))),
        );
        changes.extend(
            values
                .iter()
                .filter(|v| !held.contains(v))
                .map(|v| add("CellItem", None, vec![(key, text(v.clone()))], Some(&cell_id))),
        );
        return KnowledgeEdit::of(changes);
    }

    if values.is_empty() {
        if cell.is_none() {
            return KnowledgeEdit::nothing();
        }
        changes.push(remove(cell_id));
        return KnowledgeEdit::of(changes);
    }
    let (name, written) = match written(&property.value_type, &values[0]) {
        Err(refusal) => return refused(refusal),
        Ok(w) => w,
    };
    if written.is_null() {
        // An unticked checkbox is a cell that is not there.
        if cell.is_none() {
            return KnowledgeEdit::nothing();
        }
        changes.push(remove(cell_id));
        return KnowledgeEdit::of(changes);
    }
    // A value kept from a type the property had before is replaced, not joined: a cell holds one value.
    changes.push(match cell {
        None => add(
            "Cell",
            None,
            vec![("property", text(property.id.clone())), (name.as_str(), written)],
            Some(&row.id),
        ),
        Some(cell) if !cell.key.is_empty() && cell.key != name && cell.key != "options" && cell.key != "rows" => set(
            cell_id,
            vec![(cell.key.as_str(), Value::Null), (name.as_str(), written)],
        ),
        Some(_) => set(cell_id, vec![(name.as_str(), written)]),
    });
    KnowledgeEdit::of(changes)
}

/// Sets what a row is related to in a relation that shows both directions in one property. A pair of rows is
/// related when either names the other, so relating two rows writes one of them - the one being edited - and
/// unrelating them takes the pair out of whichever holds it.
fn set_mutual(
    table: &KnowledgeTable,
    row_id: &str,
    property: &KnowledgeProperty,
    values: &[String],
    mut changes: Vec<ModelChange>,
) -> KnowledgeEdit {
    let held = |of: &str| -> Vec<String> {
        table
            .row(of)
            .and_then(|r| r.cells.iter().find(|c| c.column_id == property.id && c.key == "rows"))
            .map(|c| c.values.clone())
            .unwrap_or_default()
    };
    let own = held(row_id);
    let stays: Vec<&String> = own.iter().filter(|v| values.contains(v)).collect();
    let mut added = Vec::new();
    for other in table.rows.iter().filter(|o| o.id != row_id) {
        let theirs = held(&other.id);
        let names = theirs.iter().any(|v| v == row_id);
        if values.contains(&other.id) {
            if !names && !own.contains(&other.id) {
                added.push(other.id.clone());
            }
        } else if names {
            // The other end holds the pair: it goes there, with the cell when it was all the cell held.
            changes.push(remove(if theirs.len() == 1 {
                cell_id(&other.id, &property.id)
            } else {
                format!("{}/{row_id}", cell_id(&other.id, &property.id))
            }));
        }
    }
    let cell = cell_id(row_id, &property.id);
    if stays.len() + added.len() == 0 {
        if !own.is_empty() {
            changes.push(remove(cell));
        }
        return KnowledgeEdit::of(changes);
    }
    if own.is_empty() {
        changes.push(add(
            "Cell",
            None,
            vec![("property", text(property.id.clone()))],
            Some(row_id),
        ));
    }
    changes.extend(
        own.iter()
            .filter(|v| !values.contains(v))
            .map(|v| remove(format!("{cell}/{v}"))),
    );
    changes.extend(
        added
            .iter()
            .map(|v| add("CellItem", None, vec![("row", text(v.clone()))], Some(&cell))),
    );
    KnowledgeEdit::of(changes)
}

/// The `Z` or offset a date and time ends with.
fn offset() -> Regex {
    Regex::new(r"(Z|[+-]\d\d:\d\d)$").expect("the offset pattern compiles")
}

fn two(n: i64) -> String {
    format!("{n:02}")
}

/// A value as its type writes it: the key it is stored under and what is stored, or why it is not a value of that
/// type. Dates and times are ISO 8601, and a date and time carries its offset: one given without is written as UTC
/// (the standalone host takes it to be the machine's local time, which this port does not read).
fn written(value_type: &str, value: &str) -> Result<(String, Value), String> {
    let key = value_type.to_owned();
    match value_type {
        "text" => Ok((key, text(value))),
        "number" => {
            let trimmed = value.trim();
            let integer = trimmed.strip_prefix(['+', '-']).unwrap_or(trimmed);
            if !integer.is_empty()
                && integer.chars().all(|c| c.is_ascii_digit())
                && let Ok(whole) = trimmed.parse::<i64>()
            {
                return Ok((key, Value::Int(whole)));
            }
            match values::parse_number(value) {
                Some(real) if real.is_finite() => Ok((key, Value::Double(real))),
                _ => Err(NUMBER_EXPECTED.into()),
            }
        }
        "checkbox" => Ok((
            "checked".into(),
            if value == "true" {
                Value::Bool(true)
            } else {
                Value::Null
            },
        )),
        "date" => match parse_date(value) {
            Some(_) => Ok((key, text(value))),
            None => Err(DATE_EXPECTED.into()),
        },
        "dateTime" => {
            if offset().is_match(value) {
                return match parse_date_time(value) {
                    Some(_) => Ok((key, text(value))),
                    None => Err(DATE_TIME_EXPECTED.into()),
                };
            }
            let Some(ticks) = parse_date_time(value) else {
                return Err(DATE_TIME_EXPECTED.into());
            };
            Ok((key, text(format_date_time(ticks))))
        }
        "time" => match parse_time(value) {
            Some(ticks) => {
                let seconds = ticks / 10_000_000;
                let (h, m, s) = (seconds / 3600, (seconds / 60) % 60, seconds % 60);
                Ok((
                    key,
                    text(if s == 0 {
                        format!("{}:{}", two(h), two(m))
                    } else {
                        format!("{}:{}:{}", two(h), two(m), two(s))
                    }),
                ))
            }
            None => Err(TIME_EXPECTED.into()),
        },
        "selection" => Ok(("option".into(), text(value))),
        _ => Err("This value cannot be set here.".into()),
    }
}

/// Ticks since 0001-01-01 as `yyyy-MM-ddTHH:mm:ss+00:00`.
fn format_date_time(ticks: i64) -> String {
    let seconds = ticks.div_euclid(10_000_000);
    let (days, rest) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days, from 0001-01-01.
    let z = days - 719_162 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{}-{}T{}:{}:{}+00:00",
        two(month),
        two(day),
        two(rest / 3600),
        two((rest / 60) % 60),
        two(rest % 60)
    )
}

fn add_row(
    table: &KnowledgeTable,
    view: Option<&ContentView>,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
) -> KnowledgeEdit {
    let id = new_id();
    // After the row the key was pressed in, where there is one; at the end otherwise.
    let after = if gesture.target_id.is_empty() {
        None
    } else {
        table.rows.iter().position(|r| r.id == gesture.target_id)
    };
    let mut changes = add_at(
        "Row",
        Some(id.clone()),
        Vec::new(),
        None,
        after.map(|a| a + 1),
        table.rows.len(),
        &id,
    );

    // A row added in a group is given the value that puts it there.
    let group = gesture.setting_of("group");
    if !group.is_empty()
        && let Some(view) = view
        && let Some(grouped) = table.column(&view.group_by).filter(|g| !g.is_computed)
    {
        let cell = cell_id(&id, &grouped.id);
        let has_option = grouped.options.iter().any(|o| o.id == group);
        let property = || ("property", text(grouped.id.clone()));
        match grouped.value_type.as_str() {
            "selection" if has_option => {
                changes.push(add("Cell", None, vec![property(), ("option", text(group))], Some(&id)));
            }
            "multipleSelection" if has_option => {
                changes.push(add("Cell", None, vec![property()], Some(&id)));
                changes.push(add("CellItem", None, vec![("option", text(group))], Some(&cell)));
            }
            "relation" => {
                changes.push(add("Cell", None, vec![property()], Some(&id)));
                changes.push(add("CellItem", None, vec![("row", text(group))], Some(&cell)));
            }
            "checkbox" if group == "true" => {
                changes.push(add(
                    "Cell",
                    None,
                    vec![property(), ("checked", Value::Bool(true))],
                    Some(&id),
                ));
            }
            _ => {}
        }
    }
    KnowledgeEdit {
        changes,
        new_row_id: id,
        ..KnowledgeEdit::default()
    }
}

fn delete_rows(table: &KnowledgeTable, gesture: &TableGesture) -> KnowledgeEdit {
    let asked: Vec<&String> = if gesture.values.is_empty() {
        vec![&gesture.row_id]
    } else {
        gesture.values.iter().collect()
    };
    let mut ids: Vec<&String> = Vec::new();
    for id in asked {
        if table.row(id).is_some() && !ids.contains(&id) {
            ids.push(id);
        }
    }
    if ids.is_empty() {
        refused(ROW_GONE)
    } else {
        KnowledgeEdit::of(ids.into_iter().map(|id| remove(id.clone())).collect())
    }
}

/// Moves rows to stand before or after another row, together and in the order they were in. The order is the
/// file's, which a view without a sort shows; a sorted view has an order of its own, so nothing is moved from one.
fn move_rows(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    if !view.sorts.is_empty() {
        return refused(SORTED_ROWS_STAY);
    }
    let mut order: Vec<String> = table.rows.iter().map(|r| r.id.clone()).collect();
    let asked: Vec<&String> = if gesture.values.is_empty() {
        vec![&gesture.row_id]
    } else {
        gesture.values.iter().collect()
    };
    let moved: Vec<String> = order.iter().filter(|id| asked.contains(id)).cloned().collect();
    if moved.is_empty() || !order.contains(&gesture.target_id) {
        return refused(ROW_GONE);
    }
    if moved.contains(&gesture.target_id) {
        return KnowledgeEdit::nothing();
    }
    // One row at a time: the first to its place beside the target, each next one right after the one before it.
    let mut changes = Vec::new();
    let mut anchor = gesture.target_id.clone();
    let mut after = gesture.setting_of("place") == "after";
    for id in moved {
        let mut without: Vec<String> = order.iter().filter(|c| **c != id).cloned().collect();
        let to = without.iter().position(|c| *c == anchor).expect("the anchor is a row") + usize::from(after);
        changes.extend(move_to(&id, None, &order, to as i64).changes);
        without.insert(to, id.clone());
        order = without;
        anchor = id;
        after = true;
    }
    KnowledgeEdit::of(changes)
}

// ---- properties ----

fn add_column(
    table: &KnowledgeTable,
    view: Option<&ContentView>,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
) -> KnowledgeEdit {
    let asked = gesture.setting_of("type");
    let type_name = if gesture.settings.contains_key("type") {
        asked
    } else {
        "text"
    };
    if type_name == "relation" {
        return refused(RELATION_NEEDS_TARGET);
    }
    let label = match definition::value_type_label(type_name) {
        Some(label) if vocabulary::VALUE_TYPES.contains(&type_name) => label,
        _ => return refused(format!("'{type_name}' is not a type a property can have.")),
    };
    // A name given with the property is its name, refused like a rename's when another property has it; without
    // one it takes its type's.
    let given = first_value(gesture);
    if !given.is_empty()
        && table
            .columns
            .iter()
            .any(|o| values::equals_ignoring_case(&o.name, &given))
    {
        return refused(format!("Another property is already called '{given}'."));
    }
    let id = new_id();
    let name = if given.is_empty() {
        unique_name(table.columns.iter().map(|p| p.name.as_str()), label)
    } else {
        given
    };
    KnowledgeEdit::of(place(
        table,
        view,
        &id,
        vec![("name", text(name)), ("valueType", text(type_name))],
        &gesture.target_id,
        gesture.setting_of("side") == "left",
    ))
}

/// A new property and where it stands: at the end, or beside another - in the file, so every view that has no
/// order of its own shows it there, and in this view's own order where the view has one that reaches that far.
fn place(
    table: &KnowledgeTable,
    view: Option<&ContentView>,
    id: &str,
    pairs: Vec<(&str, Value)>,
    beside_id: &str,
    left: bool,
) -> Vec<ModelChange> {
    let Some(beside) = table.columns.iter().position(|p| p.id == beside_id) else {
        return vec![add("Property", Some(id.to_owned()), pairs, None)];
    };
    let at = if left { beside } else { beside + 1 };
    let mut changes = add_at(
        "Property",
        Some(id.to_owned()),
        pairs,
        None,
        Some(at),
        table.columns.len(),
        id,
    );
    if let Some(view) = view
        && let Some(listed) = view.columns.iter().position(|c| c.column_id == beside_id)
    {
        changes.extend(add_at(
            "Column",
            None,
            vec![("property", text(id))],
            Some(&view.id),
            Some(if left { listed } else { listed + 1 }),
            view.columns.len(),
            &format!("{}/columns/{id}", view.id),
        ));
    }
    changes
}

fn rename_column(table: &KnowledgeTable, gesture: &TableGesture) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    let name = first_value(gesture);
    if name.is_empty() {
        return refused(NAME_NEEDED);
    }
    if name == property.name {
        return KnowledgeEdit::nothing();
    }
    if table
        .columns
        .iter()
        .any(|o| o.id != property.id && values::equals_ignoring_case(&o.name, &name))
    {
        return refused(format!("Another property is already called '{name}'."));
    }
    KnowledgeEdit::of(vec![set(property.id.clone(), vec![("name", text(name))])])
}

fn duplicate_column(
    table: &KnowledgeTable,
    view: Option<&ContentView>,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    // The copy is a property like the first: its type and its options, each with an id of its own. It is never the
    // title, and never the other side of the first one's relation.
    let id = new_id();
    let mut pairs = vec![
        (
            "name",
            text(unique_name(
                table.columns.iter().map(|p| p.name.as_str()),
                &property.name,
            )),
        ),
        ("valueType", text(property.value_type.clone())),
    ];
    if property.value_type == "relation" && !property.is_computed {
        pairs.push(("targetFile", text(property.target_file.clone())));
        pairs.push((
            "limit",
            if property.limit == "one" {
                text("one")
            } else {
                Value::Null
            },
        ));
    }
    let mut changes = place(table, view, &id, pairs, &property.id, false);
    for option in &property.options {
        changes.push(add(
            "Option",
            Some(new_id()),
            vec![
                ("name", text(option.name.clone())),
                (
                    "colour",
                    if option.colour.is_empty() || option.colour == "default" {
                        Value::Null
                    } else {
                        text(option.colour.clone())
                    },
                ),
            ],
            Some(&id),
        ));
    }
    KnowledgeEdit::of(changes)
}

fn ungroup_settings(view: &ContentView) -> Vec<ModelChange> {
    view.group_order
        .iter()
        .map(|k| format!("{}/groupOrder/{k}", view.id))
        .chain(
            view.hidden_groups
                .iter()
                .map(|k| format!("{}/hiddenGroups/{k}", view.id)),
        )
        .chain(view.collapsed.iter().map(|k| format!("{}/collapsed/{k}", view.id)))
        .map(remove)
        .collect()
}

fn delete_column(
    table: &KnowledgeTable,
    gesture: &TableGesture,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    if property.is_title {
        return refused(TITLE_STAYS);
    }
    // A view grouped by the property is no longer grouped: the grouping, and what it remembered of its groups, goes
    // in the same step. Its cells, columns, sorts and conditions go with the property itself, as the bindings
    // cascade.
    let mut changes = Vec::new();
    for view in table.views.iter().filter(|v| v.group_by == property.id) {
        changes.extend(ungroup_settings(view));
        changes.push(set(view.id.clone(), vec![("groupBy", Value::Null)]));
    }
    // A view frozen up to the property is no longer frozen: it has no column to be frozen up to.
    changes.extend(
        table
            .views
            .iter()
            .filter(|v| v.freeze == property.id)
            .map(|v| set(v.id.clone(), vec![("freeze", Value::Null)])),
    );
    changes.push(remove(property.id.clone()));

    // One side of a two-way relation: what becomes of the other side is its author's choice, never a guess.
    let itself = property.target_file == SELF || property.target_file.is_empty();
    let target = if itself {
        None
    } else {
        files.and_then(|f| (f.target_of)(property))
    };
    let related = if itself {
        Some(table)
    } else {
        target.as_ref().and_then(|t| t.table.as_ref())
    };
    let other = (property.value_type == "relation" && !property.counterpart.is_empty())
        .then(|| related.and_then(|r| r.column(&property.counterpart)))
        .flatten();
    let (Some(other), Some(related)) = (other, related) else {
        return KnowledgeEdit::of(changes);
    };
    let mut theirs = Vec::new();
    match gesture.setting_of("otherSide") {
        "delete" => theirs.push(remove(other.id.clone())),
        // It holds the values already: it only stops naming the side that goes.
        "keep" if !other.is_computed => theirs.push(set(other.id.clone(), vec![("counterpart", Value::Null)])),
        "keep" => {
            // It showed the values of the side that goes: they become its own, row by row.
            theirs.push(set(
                other.id.clone(),
                vec![("counterpart", Value::Null), ("computed", Value::Null)],
            ));
            for row in &related.rows {
                let naming: Vec<&String> = table
                    .rows
                    .iter()
                    .filter(|c| {
                        c.cells.iter().any(|cell| {
                            cell.column_id == property.id && cell.key == "rows" && cell.values.contains(&row.id)
                        })
                    })
                    .map(|c| &c.id)
                    .collect();
                if !naming.is_empty() {
                    theirs.push(add(
                        "Cell",
                        None,
                        vec![("property", text(other.id.clone()))],
                        Some(&row.id),
                    ));
                    let cell = cell_id(&row.id, &other.id);
                    theirs.extend(
                        naming
                            .into_iter()
                            .map(|v| add("CellItem", None, vec![("row", text(v.clone()))], Some(&cell))),
                    );
                }
            }
        }
        _ => return refused(OTHER_SIDE_CHOICE),
    }
    if itself {
        theirs.extend(changes);
        return KnowledgeEdit::of(theirs);
    }
    KnowledgeEdit {
        changes,
        others: vec![KnowledgeOtherFile {
            body_path: target.expect("a target with a table").path.clone(),
            changes: theirs,
        }],
        ..KnowledgeEdit::default()
    }
}

// ---- relations ----

/// What the other side of a two-way relation is called until its author says otherwise: this table's name.
fn other_side_name(table: &KnowledgeTable) -> String {
    if table.name.is_empty() {
        "Relation".into()
    } else {
        table.name.clone()
    }
}

/// A relation to another knowledge file, or to this one. One-way, it is a property of this file. Two-way, the
/// target gets the other side in the same step: a property of its own, computed and without cells, each naming the
/// other. Neither file is written when either refuses.
fn add_relation(
    table: &KnowledgeTable,
    view: Option<&ContentView>,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    let target_name = gesture.setting_of("target").trim();
    let Some(files) = files.filter(|_| !target_name.is_empty()) else {
        return refused(RELATION_NEEDS_TARGET);
    };
    let target = (files.target_at)(target_name);
    let Some(related) = &target.table else {
        return refused(target.problem.clone());
    };
    let itself = relations::same_path(&target.path, files.body_path);
    let asked = first_value(gesture);
    let wanted = if !asked.is_empty() {
        asked
    } else if !related.name.is_empty() && !itself {
        related.name.clone()
    } else {
        "Relation".into()
    };
    let name = unique_name(table.columns.iter().map(|p| p.name.as_str()), &wanted);
    let id = new_id();
    let target_file = if itself {
        SELF.to_owned()
    } else {
        relations::target_name(files.body_path, &target.path)
    };
    let mut pairs = vec![
        ("name", text(name.clone())),
        ("valueType", text("relation")),
        ("targetFile", text(target_file)),
        (
            "limit",
            if gesture.setting_of("limit") == "one" {
                text("one")
            } else {
                Value::Null
            },
        ),
    ];
    // A relation asked for as two-way without a name for its other side is called there after this table.
    let left = gesture.setting_of("side") == "left";
    let mut other_name = gesture.setting_of("counterpart").trim().to_owned();
    if other_name.is_empty() && flag(gesture, "twoWay") {
        other_name = other_side_name(table);
    }
    if other_name.is_empty() {
        return KnowledgeEdit::of(place(table, view, &id, pairs, &gesture.target_id, left));
    }
    // The other side: in the target, computed, naming this one - and this one naming it.
    let other_id = new_id();
    pairs.push(("counterpart", text(other_id.clone())));
    let taken: Vec<&str> = if itself {
        table
            .columns
            .iter()
            .map(|p| p.name.as_str())
            .chain([name.as_str()])
            .collect()
    } else {
        related.columns.iter().map(|p| p.name.as_str()).collect()
    };
    let other = add(
        "Property",
        Some(other_id),
        vec![
            ("name", text(unique_name(taken, &other_name))),
            ("valueType", text("relation")),
            (
                "targetFile",
                text(if itself {
                    SELF.to_owned()
                } else {
                    relations::target_name(&target.path, files.body_path)
                }),
            ),
            ("computed", Value::Bool(true)),
            ("counterpart", text(id.clone())),
        ],
        None,
    );
    let mut changes = place(table, view, &id, pairs, &gesture.target_id, left);
    if itself {
        changes.push(other);
        return KnowledgeEdit::of(changes);
    }
    KnowledgeEdit {
        changes,
        others: vec![KnowledgeOtherFile {
            body_path: target.path.clone(),
            changes: vec![other],
        }],
        ..KnowledgeEdit::default()
    }
}

/// The relation of this table a gesture is about: one that holds its own values, not the shown side of another's.
fn own_relation<'t>(table: &'t KnowledgeTable, property_id: &str) -> Option<&'t KnowledgeProperty> {
    table
        .column(property_id)
        .filter(|p| p.value_type == "relation" && !p.is_computed)
}

/// How many rows a relation's cell may hold: one, or any number. Cells that hold more than a new limit of one are
/// kept as they are, and the table says so of each.
fn set_limit(table: &KnowledgeTable, gesture: &TableGesture) -> KnowledgeEdit {
    if table.column(&gesture.column_id).is_none() {
        return refused(PROPERTY_GONE);
    }
    let Some(property) = own_relation(table, &gesture.column_id) else {
        return refused(NOT_A_RELATION);
    };
    let limit = if gesture.setting_of("limit") == "one" {
        "one"
    } else {
        "none"
    };
    if limit == property.limit {
        return KnowledgeEdit::nothing();
    }
    if property.is_parent {
        return refused(PARENT_HOLDS_ONE);
    }
    KnowledgeEdit::of(vec![set(
        property.id.clone(),
        vec![("limit", if limit == "one" { text("one") } else { Value::Null })],
    )])
}

/// Whether a relation is shown in the table it relates to as well: made two-way, its other side is added there under
/// the name given, or under this table's; made one-way, that side is removed.
fn set_other_side(
    table: &KnowledgeTable,
    gesture: &TableGesture,
    new_id: &mut dyn FnMut() -> String,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    if table.column(&gesture.column_id).is_none() {
        return refused(PROPERTY_GONE);
    }
    let Some(property) = own_relation(table, &gesture.column_id) else {
        return refused(NOT_A_RELATION);
    };
    let itself = property.target_file == SELF || property.target_file.is_empty();
    let target = if itself {
        None
    } else {
        files.and_then(|f| (f.target_of)(property))
    };
    if !itself && target.as_ref().and_then(|t| t.table.as_ref()).is_none() {
        let problem = target.as_ref().map(|t| t.problem.clone()).unwrap_or_default();
        return refused(if problem.is_empty() {
            RELATION_NEEDS_TARGET.to_owned()
        } else {
            problem
        });
    }
    let related = if itself {
        table
    } else {
        target.as_ref().and_then(|t| t.table.as_ref()).expect("a table")
    };
    let other = (!property.counterpart.is_empty())
        .then(|| related.column(&property.counterpart))
        .flatten();
    let two_way = flag(gesture, "twoWay");
    if two_way == other.is_some() {
        return KnowledgeEdit::nothing();
    }
    let (mine, theirs) = match other {
        Some(other) => (
            set(property.id.clone(), vec![("counterpart", Value::Null)]),
            remove(other.id.clone()),
        ),
        None => {
            let other_id = new_id();
            let asked = gesture.setting_of("counterpart").trim().to_owned();
            let name = unique_name(
                related.columns.iter().map(|p| p.name.as_str()),
                &if asked.is_empty() {
                    other_side_name(table)
                } else {
                    asked
                },
            );
            (
                set(property.id.clone(), vec![("counterpart", text(other_id.clone()))]),
                add(
                    "Property",
                    Some(other_id),
                    vec![
                        ("name", text(name)),
                        ("valueType", text("relation")),
                        (
                            "targetFile",
                            text(if itself {
                                SELF.to_owned()
                            } else {
                                relations::target_name(
                                    &target.as_ref().expect("a target").path,
                                    files.expect("the files around a target").body_path,
                                )
                            }),
                        ),
                        ("computed", Value::Bool(true)),
                        ("counterpart", text(property.id.clone())),
                    ],
                    None,
                ),
            )
        }
    };
    if itself {
        return KnowledgeEdit::of(vec![mine, theirs]);
    }
    KnowledgeEdit {
        changes: vec![mine],
        others: vec![KnowledgeOtherFile {
            body_path: target.expect("a target").path.clone(),
            changes: vec![theirs],
        }],
        ..KnowledgeEdit::default()
    }
}

/// Makes a relation the parent relation, or no longer that: the one a view nests its rows by. Only a relation to the
/// table itself that holds one row can be it, and a table has one.
fn set_parent(table: &KnowledgeTable, gesture: &TableGesture) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    let parent = flag(gesture, "parent");
    if parent == property.is_parent {
        return KnowledgeEdit::nothing();
    }
    if !parent {
        return KnowledgeEdit::of(vec![set(property.id.clone(), vec![("isParent", Value::Null)])]);
    }
    let itself = property.target_file == SELF || property.target_file.is_empty();
    if !(property.value_type == "relation" && itself && property.limit == "one" && !property.is_computed) {
        return refused("Only a relation to this table that holds one row can be the parent relation.");
    }
    if let Some(already) = table.parent_column() {
        return refused(format!("'{}' is the parent relation already.", already.name));
    }
    // What is there must not be a circle already: a parent relation is never one.
    let circle = table.rows.iter().any(|row| {
        row.cells
            .iter()
            .find(|c| c.column_id == property.id && c.key == "rows")
            .and_then(|c| c.values.first())
            .is_some_and(|above| relations::would_cycle(table, property, &row.id, above))
    });
    if circle {
        return refused("Some rows of this relation are each other's ancestors, so it cannot be the parent relation.");
    }
    KnowledgeEdit::of(vec![set(property.id.clone(), vec![("isParent", Value::Bool(true))])])
}

// ---- a selection's options ----

/// The selection a gesture names, and the option of it the gesture names, handed to what the gesture does.
fn for_option(
    table: &KnowledgeTable,
    gesture: &TableGesture,
    edit: impl FnOnce(&KnowledgeProperty, &ContentOption) -> KnowledgeEdit,
) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    if !matches!(property.value_type.as_str(), "selection" | "multipleSelection") {
        return refused(NOT_A_SELECTION);
    }
    match property.options.iter().find(|o| o.id == gesture.target_id) {
        Some(option) => edit(property, option),
        None => refused(OPTION_GONE),
    }
}

/// A name for an option of a property: given, and not the name of another of its options.
fn option_name(
    property: &KnowledgeProperty,
    except_id: Option<&str>,
    gesture: &TableGesture,
) -> Result<String, String> {
    let name = first_value(gesture);
    if name.is_empty() {
        return Err(OPTION_NAME_NEEDED.into());
    }
    if property
        .options
        .iter()
        .any(|o| Some(o.id.as_str()) != except_id && values::equals_ignoring_case(&o.name, &name))
    {
        return Err(format!("'{}' already has an option called '{name}'.", property.name));
    }
    Ok(name)
}

/// .NET's `char.ToUpperInvariant`: the one upper-case letter a letter has, else itself.
fn upper_invariant(c: char) -> char {
    let mut upper = c.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(u), None) => u,
        _ => c,
    }
}

/// The colour a new option starts with, as Notion gives one: any of the palette's colours, and one the property's
/// options do not have yet while there is one. Which one follows from the option's name, so the same option made in
/// two files, or made again after an undo, looks the same.
pub fn colour_for(property: &KnowledgeProperty, name: &str) -> String {
    let palette: Vec<&str> = vocabulary::COLOURS.into_iter().filter(|c| *c != "default").collect();
    let free: Vec<&str> = palette
        .iter()
        .copied()
        .filter(|c| property.options.iter().all(|o| o.colour != *c))
        .collect();
    let pool = if free.is_empty() { &palette } else { &free };
    let upper: String = name.chars().map(upper_invariant).collect();
    let sum = upper.encode_utf16().fold(17_i32, |total, letter| {
        total.wrapping_mul(31).wrapping_add(i32::from(letter))
    });
    pool[(sum as u32 % pool.len() as u32) as usize].to_owned()
}

fn add_option(table: &KnowledgeTable, gesture: &TableGesture, new_id: &mut dyn FnMut() -> String) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    if !matches!(property.value_type.as_str(), "selection" | "multipleSelection") {
        return refused(NOT_A_SELECTION);
    }
    match option_name(property, None, gesture) {
        Err(refusal) => refused(refusal),
        Ok(name) => KnowledgeEdit::of(vec![add(
            "Option",
            Some(new_id()),
            vec![
                ("name", text(name.clone())),
                ("colour", text(colour_for(property, &name))),
            ],
            Some(&property.id),
        )]),
    }
}

/// A rename is one place in the file: rows hold an option's id, never its name.
fn rename_option(property: &KnowledgeProperty, option: &ContentOption, gesture: &TableGesture) -> KnowledgeEdit {
    match option_name(property, Some(&option.id), gesture) {
        Err(refusal) => refused(refusal),
        Ok(name) if name == option.name => KnowledgeEdit::nothing(),
        Ok(name) => KnowledgeEdit::of(vec![set(option.id.clone(), vec![("name", text(name))])]),
    }
}

fn recolour_option(option: &ContentOption, gesture: &TableGesture) -> KnowledgeEdit {
    let colour = gesture.setting_of("colour");
    if !vocabulary::COLOURS.contains(&colour) {
        return refused(format!("'{colour}' is not a colour an option can have."));
    }
    // The default colour is what an option has when the file says nothing.
    if colour == option.colour {
        return KnowledgeEdit::nothing();
    }
    KnowledgeEdit::of(vec![set(
        option.id.clone(),
        vec![("colour", if colour == "default" { Value::Null } else { text(colour) })],
    )])
}

/// An option deleted with every value that names it, in one step: the cells, the items and the conditions comparing
/// with it go with the option as the bindings cascade, and what a view grouped by this property remembered of the
/// option's group goes here.
fn delete_option(table: &KnowledgeTable, property: &KnowledgeProperty, option: &ContentOption) -> KnowledgeEdit {
    let mut changes = Vec::new();
    for view in table.views.iter().filter(|v| v.group_by == property.id) {
        for (slot, keys) in [
            ("groupOrder", &view.group_order),
            ("hiddenGroups", &view.hidden_groups),
            ("collapsed", &view.collapsed),
        ] {
            if keys.contains(&option.id) {
                changes.push(remove(format!("{}/{slot}/{}", view.id, option.id)));
            }
        }
    }
    changes.push(remove(option.id.clone()));
    KnowledgeEdit::of(changes)
}

// ---- a property's type ----

/// A change of type, with every value that has a meaning in the new type converted in the same step, as the
/// conversion table of the designer's specification says. A value that has none is not touched: it stays under the
/// key it has, to be reported on its cell and to be there again when the type is changed back.
fn change_type(table: &KnowledgeTable, gesture: &TableGesture, new_id: &mut dyn FnMut() -> String) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    let to = gesture.setting_of("type");
    if to == property.value_type {
        return KnowledgeEdit::nothing();
    }
    if property.is_title {
        return refused(TITLE_IS_TEXT);
    }
    if to == "relation" {
        return refused(RELATION_NEEDS_TARGET);
    }
    if !vocabulary::VALUE_TYPES.contains(&to) {
        return refused(format!("'{to}' is not a type a property can have."));
    }
    let mut changes = vec![set(property.id.clone(), vec![("valueType", text(to))])];
    let from = property.value_type.as_str();
    let held = vocabulary::key_of(from);
    let names: std::collections::HashMap<&str, &str> = property
        .options
        .iter()
        .map(|o| (o.id.as_str(), o.name.as_str()))
        .collect();
    let mut ids: Vec<(String, String)> = Vec::new();
    for option in &property.options {
        if !ids.iter().any(|(n, _)| values::equals_ignoring_case(n, &option.name)) {
            ids.push((option.name.clone(), option.id.clone()));
        }
    }
    // The option of a name, made in this step when the property has none of it.
    let mut option = |name: &str, changes: &mut Vec<ModelChange>| -> String {
        if let Some((_, id)) = ids.iter().find(|(n, _)| values::equals_ignoring_case(n, name)) {
            return id.clone();
        }
        let id = new_id();
        ids.push((name.to_owned(), id.clone()));
        changes.push(add(
            "Option",
            Some(id.clone()),
            vec![("name", text(name))],
            Some(&property.id),
        ));
        id
    };
    for row in &table.rows {
        // Only a value written as the type the property had is converted. One kept from an earlier change of type
        // stays as it is, under the key it has.
        let Some(cell) = row.cell(&property.id).filter(|c| !c.values.is_empty() && c.key == held) else {
            continue;
        };
        let cell_id = cell_id(&row.id, &property.id);
        let value = cell.values[0].as_str();
        let trimmed = value.trim();
        match (from, to) {
            ("text", "checkbox") if matches!(trimmed.to_lowercase().as_str(), "true" | "false") => {
                changes.push(set(
                    cell_id,
                    vec![
                        ("text", Value::Null),
                        ("checked", Value::Bool(trimmed.eq_ignore_ascii_case("true"))),
                    ],
                ));
            }
            ("text", "number" | "date" | "dateTime" | "time") => {
                if let Ok((key, written)) = written(to, trimmed) {
                    changes.push(set(cell_id, vec![("text", Value::Null), (key.as_str(), written)]));
                }
            }
            ("text", "selection") if !trimmed.is_empty() => {
                let id = option(trimmed, &mut changes);
                changes.push(set(cell_id, vec![("text", Value::Null), ("option", text(id))]));
            }
            ("text", "multipleSelection") if !trimmed.is_empty() => {
                let made = option(trimmed, &mut changes);
                changes.push(set(cell_id.clone(), vec![("text", Value::Null)]));
                changes.push(add("CellItem", None, vec![("option", text(made))], Some(&cell_id)));
            }
            ("number" | "checkbox" | "date" | "dateTime" | "time", "text") => {
                changes.push(set(cell_id, vec![(held, Value::Null), ("text", text(value))]));
            }
            ("selection", "text") if names.contains_key(value) => {
                changes.push(set(
                    cell_id,
                    vec![("option", Value::Null), ("text", text(names[value]))],
                ));
            }
            ("selection", "multipleSelection") => {
                changes.push(set(cell_id.clone(), vec![("option", Value::Null)]));
                changes.push(add("CellItem", None, vec![("option", text(value))], Some(&cell_id)));
            }
            ("multipleSelection", "text") if cell.values.iter().all(|v| names.contains_key(v.as_str())) => {
                let joined: Vec<&str> = cell.values.iter().map(|v| names[v.as_str()]).collect();
                changes.push(set(cell_id.clone(), vec![("text", text(joined.join(", ")))]));
                changes.extend(cell.values.iter().map(|v| remove(format!("{cell_id}/{v}"))));
            }
            ("multipleSelection", "selection") if cell.values.len() == 1 => {
                changes.push(remove(format!("{cell_id}/{value}")));
                changes.push(set(cell_id, vec![("option", text(value))]));
            }
            _ => {}
        }
    }
    KnowledgeEdit::of(changes)
}

// ---- a view's columns ----

fn move_column(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    // The order the view shows: what it lists, then every other property in the file's order.
    let mut order: Vec<String> = view
        .columns
        .iter()
        .map(|c| c.column_id.clone())
        .filter(|id| table.column(id).is_some())
        .collect();
    for property in &table.columns {
        if !order.contains(&property.id) {
            order.push(property.id.clone());
        }
    }
    let Some(at) = order.iter().position(|id| *id == gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    // The index is among the columns in sight; the one that will follow the moved column says where that is among
    // all of them.
    let shown = |id: &str| {
        table.column(id).is_some_and(|p| p.is_title)
            || view
                .columns
                .iter()
                .find(|c| c.column_id == id)
                .is_none_or(|c| c.visible)
    };
    let others: Vec<String> = order.iter().filter(|id| **id != gesture.column_id).cloned().collect();
    let in_sight: Vec<&String> = others.iter().filter(|id| shown(id)).collect();
    let to = match usize::try_from(gesture.index).ok().and_then(|i| in_sight.get(i)) {
        Some(next) => others.iter().position(|id| id == *next).expect("among the others"),
        None => others.len(),
    };
    if at == to {
        return KnowledgeEdit::nothing();
    }
    // An order is the view's own, so the view has to list everything up to where the column lands.
    let mut changes = Vec::new();
    let mut listed: Vec<String> = view.columns.iter().map(|c| c.column_id.clone()).collect();
    for id in others.iter().take(to).chain(std::iter::once(&gesture.column_id)) {
        if !listed.contains(id) {
            changes.push(add(
                "Column",
                None,
                vec![("property", text(id.clone()))],
                Some(&view.id),
            ));
            listed.push(id.clone());
        }
    }
    let ids: Vec<String> = listed.iter().map(|id| column_id(view, id)).collect();
    let place = to.min(listed.len() - 1);
    changes.extend(move_to(&column_id(view, &gesture.column_id), Some(&view.id), &ids, place as i64).changes);
    KnowledgeEdit::of(changes)
}

fn show_column(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture, visible: bool) -> KnowledgeEdit {
    if !visible && table.column(&gesture.column_id).is_some_and(|p| p.is_title) {
        return refused(TITLE_STAYS);
    }
    // Shown is what a column is unless the view says otherwise, so only hidden is written.
    column_setting(
        table,
        view,
        &gesture.column_id,
        "visible",
        if visible { Value::Null } else { Value::Bool(false) },
    )
}

/// What the line under a column shows: one of the calculations its property's type has, or none.
fn calculate(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    let calculation = gesture.setting_of("calculate");
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    if !calculation.is_empty() && !calculations::for_type(&property.value_type).contains(&calculation) {
        return refused(if calculations::is_known(calculation) {
            format!("'{}' is not of a type that calculation is for.", property.name)
        } else {
            "That is not a calculation.".into()
        });
    }
    let now = view
        .columns
        .iter()
        .find(|c| c.column_id == property.id)
        .map(|c| c.calculate.as_str())
        .unwrap_or_default();
    if now == calculation {
        return KnowledgeEdit::nothing();
    }
    column_setting(
        table,
        view,
        &property.id,
        "calculate",
        if calculation.is_empty() {
            Value::Null
        } else {
            text(calculation)
        },
    )
}

/// The columns up to and including one stay in sight when the table scrolls sideways; with no column named, none
/// does.
fn freeze(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    if !gesture.column_id.is_empty() && table.column(&gesture.column_id).is_none() {
        return refused(PROPERTY_GONE);
    }
    if gesture.column_id == view.freeze {
        return KnowledgeEdit::nothing();
    }
    KnowledgeEdit::of(vec![set(
        view.id.clone(),
        vec![(
            "freeze",
            if gesture.column_id.is_empty() {
                Value::Null
            } else {
                text(gesture.column_id.clone())
            },
        )],
    )])
}

/// One setting of one column of one view: set on its entry, which is written only when there is something to say. A
/// column the view does not list yet is listed together with the unlisted ones shown before it, each by its property
/// alone, and stays where it was shown.
fn column_setting(
    table: &KnowledgeTable,
    view: &ContentView,
    property_id: &str,
    name: &str,
    value: Value,
) -> KnowledgeEdit {
    if table.column(property_id).is_none() {
        return refused(PROPERTY_GONE);
    }
    if view.columns.iter().any(|c| c.column_id == property_id) {
        return KnowledgeEdit::of(vec![set(column_id(view, property_id), vec![(name, value)])]);
    }
    if value.is_null() {
        return KnowledgeEdit::nothing();
    }
    let listed: Vec<&str> = view.columns.iter().map(|c| c.column_id.as_str()).collect();
    let mut changes: Vec<ModelChange> = table
        .columns
        .iter()
        .map(|p| p.id.as_str())
        .take_while(|id| *id != property_id)
        .filter(|id| !listed.contains(id))
        .map(|id| add("Column", None, vec![("property", text(id))], Some(&view.id)))
        .collect();
    changes.push(add(
        "Column",
        None,
        vec![("property", text(property_id)), (name, value)],
        Some(&view.id),
    ));
    KnowledgeEdit::of(changes)
}

// ---- views ----

fn for_view(
    table: &KnowledgeTable,
    gesture: &TableGesture,
    edit: impl FnOnce(&ContentView) -> KnowledgeEdit,
) -> KnowledgeEdit {
    match table.view(&gesture.view_id) {
        Some(view) => edit(view),
        None => refused(VIEW_GONE),
    }
}

fn add_view(table: &KnowledgeTable, new_id: &mut dyn FnMut() -> String) -> KnowledgeEdit {
    let id = new_id();
    KnowledgeEdit {
        changes: vec![add(
            "View",
            Some(id.clone()),
            vec![(
                "name",
                text(unique_name(table.views.iter().map(|v| v.name.as_str()), "View")),
            )],
            None,
        )],
        new_view_id: id,
        ..KnowledgeEdit::default()
    }
}

fn rename_view(view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    let name = first_value(gesture);
    if name.is_empty() {
        return refused(VIEW_NAME_NEEDED);
    }
    if name == view.name {
        return KnowledgeEdit::nothing();
    }
    KnowledgeEdit::of(vec![set(view.id.clone(), vec![("name", text(name))])])
}

fn or_null(condition: bool, value: Value) -> Value {
    if condition { value } else { Value::Null }
}

fn duplicate_view(table: &KnowledgeTable, view: &ContentView, new_id: &mut dyn FnMut() -> String) -> KnowledgeEdit {
    let id = new_id();
    let at = table.views.iter().position(|v| v.id == view.id).map_or(0, |i| i + 1);
    let mut changes = add_at(
        "View",
        Some(id.clone()),
        vec![
            (
                "name",
                text(unique_name(table.views.iter().map(|v| v.name.as_str()), &view.name)),
            ),
            ("filterMatch", or_null(view.filter.any, text("any"))),
            (
                "groupBy",
                or_null(!view.group_by.is_empty(), text(view.group_by.clone())),
            ),
            ("hideEmptyGroups", or_null(view.hide_empty_groups, Value::Bool(true))),
            ("freeze", or_null(!view.freeze.is_empty(), text(view.freeze.clone()))),
        ],
        None,
        Some(at),
        table.views.len(),
        &id,
    );
    for column in &view.columns {
        changes.push(add(
            "Column",
            None,
            vec![
                ("property", text(column.column_id.clone())),
                ("visible", or_null(!column.visible, Value::Bool(false))),
                ("width", or_null(column.width > 0, Value::Int(column.width))),
                ("wrap", or_null(column.wrap, Value::Bool(true))),
                (
                    "calculate",
                    or_null(!column.calculate.is_empty(), text(column.calculate.clone())),
                ),
            ],
            Some(&id),
        ));
    }
    for sort in &view.sorts {
        changes.push(add(
            "Sort",
            None,
            vec![
                ("property", text(sort.column_id.clone())),
                ("direction", or_null(sort.descending, text("descending"))),
            ],
            Some(&id),
        ));
    }
    copy_filter(table, &view.filter, &id, &format!("{id}/filter"), &mut changes);
    for (slot, keys) in [
        ("groupOrder", &view.group_order),
        ("hiddenGroups", &view.hidden_groups),
        ("collapsed", &view.collapsed),
    ] {
        changes.extend(
            keys.iter()
                .map(|k| add_in("GroupSetting", vec![("key", text(k.clone()))], &id, slot)),
        );
    }
    KnowledgeEdit {
        changes,
        new_view_id: id,
        ..KnowledgeEdit::default()
    }
}

/// A filter's items under a new parent, each addressed by its place as the reading will address it.
fn copy_filter(
    table: &KnowledgeTable,
    group: &FilterGroup,
    parent_id: &str,
    prefix: &str,
    changes: &mut Vec<ModelChange>,
) {
    for (index, item) in group.items.iter().enumerate() {
        let item_id = format!("{prefix}/{index}");
        match item {
            FilterItem::Group(nested) => {
                changes.push(add(
                    "FilterGroup",
                    None,
                    vec![("match", text(if nested.any { "any" } else { "all" }))],
                    Some(parent_id),
                ));
                copy_filter(table, nested, &item_id, &item_id, changes);
            }
            FilterItem::Condition(condition) => {
                let mut pairs = vec![
                    ("property", text(condition.column_id.clone())),
                    ("operator", text(condition.operator.clone())),
                ];
                let value = (!condition.value.is_empty())
                    .then(|| condition_value(table, &condition.column_id, &condition.value).ok())
                    .flatten()
                    .filter(|(_, w)| !w.is_null());
                if let Some((key, written)) = &value {
                    pairs.push((key.as_str(), written.clone()));
                }
                changes.push(add("Condition", None, pairs, Some(parent_id)));
            }
        }
    }
}

fn delete_view(table: &KnowledgeTable, view: &ContentView) -> KnowledgeEdit {
    if table.views.len() <= 1 {
        return refused(LAST_VIEW);
    }
    // The file no longer says it was left in a view that is gone.
    if table.active_view_id == view.id {
        return KnowledgeEdit::of(vec![
            set(TABLE_ID, vec![("activeView", Value::Null)]),
            remove(view.id.clone()),
        ]);
    }
    KnowledgeEdit::of(vec![remove(view.id.clone())])
}

// ---- sorts and groups ----

fn add_sort(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    if table.column(&gesture.column_id).is_none() {
        return refused(PROPERTY_GONE);
    }
    if view.sorts.iter().any(|s| s.column_id == gesture.column_id) {
        return KnowledgeEdit::of(vec![set(
            sort_id(view, &gesture.column_id),
            vec![("direction", direction(gesture))],
        )]);
    }
    KnowledgeEdit::of(vec![add(
        "Sort",
        None,
        vec![
            ("property", text(gesture.column_id.clone())),
            ("direction", direction(gesture)),
        ],
        Some(&view.id),
    )])
}

fn group_by(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    if gesture.column_id == view.group_by {
        return KnowledgeEdit::nothing();
    }
    if !gesture.column_id.is_empty() {
        let Some(property) = table.column(&gesture.column_id) else {
            return refused(PROPERTY_GONE);
        };
        if !matches!(
            property.value_type.as_str(),
            "selection" | "multipleSelection" | "checkbox" | "relation"
        ) {
            return refused("A view is grouped by a selection, a checkbox or a relation.");
        }
    }
    // What the view remembered of its groups was about the old grouping's groups.
    let mut changes = ungroup_settings(view);
    changes.push(set(
        view.id.clone(),
        vec![(
            "groupBy",
            if gesture.column_id.is_empty() {
                Value::Null
            } else {
                text(gesture.column_id.clone())
            },
        )],
    ));
    KnowledgeEdit::of(changes)
}

/// Moves a group: one place up or down among the groups that are shown, or to a place among all of them. The view's
/// order names every group from then on: those it did not name yet are added in the order they stood in.
fn move_group(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    let Some(grouped) = table.column(&view.group_by) else {
        return refused(NOT_GROUPED);
    };
    let order: Vec<String> = groups::of(table, view, grouped, None)
        .into_iter()
        .map(|(k, _)| k)
        .collect();
    let Some(from) = order.iter().position(|k| *k == gesture.target_id) else {
        return KnowledgeEdit::nothing();
    };
    let to = if let Some(direction) = gesture.settings.get("direction") {
        // Beside the next group that is shown: a hidden one is stepped over.
        let shown: Vec<&String> = order.iter().filter(|k| !view.hidden_groups.contains(k)).collect();
        let here = shown
            .iter()
            .position(|k| **k == gesture.target_id)
            .map_or(-1, |i| i as i64);
        let beside = here + if direction == "up" { -1 } else { 1 };
        if beside < 0 || beside >= shown.len() as i64 {
            return KnowledgeEdit::nothing();
        }
        order
            .iter()
            .position(|k| k == shown[beside as usize])
            .expect("a shown group")
    } else {
        gesture.index.clamp(0, order.len() as i64 - 1) as usize
    };
    let mut desired = order.clone();
    let moved = desired.remove(from);
    desired.insert(to, moved);
    reordered(view, &order, &desired)
}

/// Puts the groups in the order of their names, forwards or backwards; the rows without a value stay last.
fn sort_groups(
    table: &KnowledgeTable,
    view: &ContentView,
    gesture: &TableGesture,
    files: Option<&KnowledgeSurroundings<'_>>,
) -> KnowledgeEdit {
    let Some(grouped) = table.column(&view.group_by) else {
        return refused(NOT_GROUPED);
    };
    let target = if grouped.value_type == "relation" {
        files.and_then(|f| (f.target_of)(grouped))
    } else {
        None
    };
    let label = |_: &str, id: &str| relations::label_of(target.as_deref(), id);
    let listed = groups::of(table, view, grouped, target.as_ref().map(|_| &label as _));
    let mut named: Vec<&(String, String)> = listed.iter().filter(|(k, _)| k != NO_VALUE).collect();
    named.sort_by_key(|a| a.1.to_lowercase());
    if gesture.setting_of("order") == "descending" {
        named.reverse();
    }
    let mut desired: Vec<String> = named.into_iter().map(|(k, _)| k.clone()).collect();
    desired.extend(listed.iter().filter(|(k, _)| k == NO_VALUE).map(|(k, _)| k.clone()));
    let order: Vec<String> = listed.into_iter().map(|(k, _)| k).collect();
    reordered(view, &order, &desired)
}

/// The changes that give a view's groups an order: an entry for each group the view does not name yet, in the place
/// it stands in now, and then one move per group that is not where it is wanted. Entries for groups that are no
/// longer there stay behind the others.
fn reordered(view: &ContentView, order: &[String], desired: &[String]) -> KnowledgeEdit {
    let mut named: Vec<String> = Vec::new();
    for key in &view.group_order {
        if !named.contains(key) {
            named.push(key.clone());
        }
    }
    let mut changes: Vec<ModelChange> = order
        .iter()
        .filter(|k| !named.contains(k))
        .map(|k| add_in("GroupSetting", vec![("key", text(k.clone()))], &view.id, "groupOrder"))
        .collect();
    let mut work: Vec<String> = named.clone();
    work.extend(order.iter().filter(|k| !named.contains(k)).cloned());
    let mut wanted: Vec<String> = desired.to_vec();
    wanted.extend(work.iter().filter(|k| !desired.contains(k)).cloned());
    let id = |key: &str| format!("{}/groupOrder/{key}", view.id);
    for (place, key) in wanted.iter().enumerate() {
        let Some(from) = work.iter().position(|k| k == key) else {
            continue;
        };
        if from == place {
            continue;
        }
        let ids: Vec<String> = work.iter().map(|k| id(k)).collect();
        changes.extend(move_to(&id(key), Some(&view.id), &ids, place as i64).changes);
        let moved = work.remove(from);
        work.insert(place, moved);
    }
    KnowledgeEdit::of(changes)
}

fn collapse(view: &ContentView, key: &str, collapsed: bool) -> KnowledgeEdit {
    if key.is_empty() || view.collapsed.iter().any(|k| k == key) == collapsed {
        return KnowledgeEdit::nothing();
    }
    if collapsed {
        KnowledgeEdit::of(vec![add_in(
            "GroupSetting",
            vec![("key", text(key))],
            &view.id,
            "collapsed",
        )])
    } else {
        KnowledgeEdit::of(vec![remove(format!("{}/collapsed/{key}", view.id))])
    }
}

// ---- the filter ----

/// A filter's item, its id and its depth, by its path: empty for the view's own group, else places joined by a
/// slash.
fn filter_item<'v>(view: &'v ContentView, path: &str) -> Option<(String, FilterRef<'v>, usize)> {
    let mut item = FilterRef::Group(&view.filter);
    let mut id = format!("{}/filter", view.id);
    let mut depth = 0;
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        let FilterRef::Group(group) = item else {
            return None;
        };
        if !segment.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let index: usize = segment.parse().ok()?;
        item = match group.items.get(index)? {
            FilterItem::Group(nested) => FilterRef::Group(nested),
            FilterItem::Condition(condition) => FilterRef::Condition(condition),
        };
        id = format!("{id}/{index}");
        depth += 1;
    }
    Some((if depth == 0 { view.id.clone() } else { id }, item, depth))
}

#[derive(Clone, Copy)]
enum FilterRef<'v> {
    Group(&'v FilterGroup),
    Condition(&'v adp_desl::table::content::Condition),
}

/// A condition or a group added as the last item of a group, in every format. A format that adds an entry after the
/// last one its rule reads would put a condition before the groups that follow the conditions; there the item is
/// moved to the end in the same step, so a filter reads the same whichever format the file has.
fn add_to_filter(
    type_name: &str,
    pairs: Vec<(&str, Value)>,
    view: &ContentView,
    group: (String, &FilterGroup, usize),
    beside_its_rule: bool,
) -> KnowledgeEdit {
    let (group_id, filter, depth) = group;
    let added = add(type_name, None, pairs, Some(&group_id));
    if !beside_its_rule {
        return KnowledgeEdit::of(vec![added]);
    }
    let condition = type_name == "Condition";
    let last = filter
        .items
        .iter()
        .rposition(|i| matches!(i, FilterItem::Condition(_)) == condition);
    match last {
        Some(last) if last != filter.items.len() - 1 => {
            let prefix = if depth == 0 {
                format!("{}/filter", view.id)
            } else {
                group_id.clone()
            };
            KnowledgeEdit::of(vec![
                added,
                ModelChange::Move {
                    id: format!("{prefix}/{}", last + 1),
                    new_parent_id: Some(group_id),
                    index: filter.items.len() as i64 + 1,
                },
            ])
        }
        _ => KnowledgeEdit::of(vec![added]),
    }
}

fn add_filter(
    table: &KnowledgeTable,
    view: &ContentView,
    gesture: &TableGesture,
    beside_its_rule: bool,
) -> KnowledgeEdit {
    let Some(property) = table.column(&gesture.column_id) else {
        return refused(PROPERTY_GONE);
    };
    let Some((id, FilterRef::Group(group), depth)) = filter_item(view, &gesture.target_id) else {
        return refused(FILTER_GROUP_GONE);
    };
    let asked = gesture.setting_of("comparison");
    let operator = if asked.is_empty() {
        vocabulary::comparisons(&property.value_type)
            .and_then(|c| c.first())
            .copied()
            .unwrap_or("is-empty")
            .to_owned()
    } else {
        comparison(asked)
    };
    add_to_filter(
        "Condition",
        vec![("property", text(property.id.clone())), ("operator", text(operator))],
        view,
        (id, group, depth),
        beside_its_rule,
    )
}

fn add_filter_group(view: &ContentView, gesture: &TableGesture, beside_its_rule: bool) -> KnowledgeEdit {
    let Some((id, FilterRef::Group(group), depth)) = filter_item(view, &gesture.target_id) else {
        return refused(FILTER_GROUP_GONE);
    };
    if depth >= FILTER_DEPTH {
        return refused("A filter's groups go two deep.");
    }
    add_to_filter(
        "FilterGroup",
        vec![("match", text("all"))],
        view,
        (id, group, depth),
        beside_its_rule,
    )
}

fn set_filter(table: &KnowledgeTable, view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    let Some((id, FilterRef::Condition(condition), _)) = filter_item(view, &gesture.target_id) else {
        return refused("That condition is no longer in this filter.");
    };
    let property_id = if gesture.column_id.is_empty() {
        condition.column_id.clone()
    } else {
        gesture.column_id.clone()
    };
    if table.column(&property_id).is_none() {
        return refused(PROPERTY_GONE);
    }
    // One value under the key its property's type writes it under, and no other value left beside it.
    let asked = gesture.setting_of("comparison");
    let mut changed: Map = attributes(vec![
        ("property", text(property_id.clone())),
        (
            "operator",
            text(if asked.is_empty() {
                condition.operator.clone()
            } else {
                comparison(asked)
            }),
        ),
        ("text", Value::Null),
        ("number", Value::Null),
        ("checked", Value::Null),
        ("date", Value::Null),
        ("dateTime", Value::Null),
        ("time", Value::Null),
        ("option", Value::Null),
    ]);
    let value = gesture.values.first().cloned().unwrap_or_default();
    if !value.is_empty() {
        match condition_value(table, &property_id, &value) {
            Err(refusal) => return refused(refusal),
            Ok((key, written)) => changed.insert(key, written),
        }
    }
    KnowledgeEdit::of(vec![ModelChange::Set {
        id,
        attributes: changed,
    }])
}

/// What a condition is compared with, as its property's type writes it. Several values are compared one at a time.
fn condition_value(table: &KnowledgeTable, property_id: &str, value: &str) -> Result<(String, Value), String> {
    let type_name = table.column(property_id).map_or("text", |p| p.value_type.as_str());
    match type_name {
        "multipleSelection" => Ok(("option".into(), text(value))),
        "relation" => Ok(("text".into(), text(value))),
        other => written(other, value),
    }
}

fn set_filter_match(view: &ContentView, gesture: &TableGesture) -> KnowledgeEdit {
    let Some((id, FilterRef::Group(_), depth)) = filter_item(view, &gesture.target_id) else {
        return refused(FILTER_GROUP_GONE);
    };
    let any = gesture.setting_of("match") == "any";
    if depth == 0 {
        KnowledgeEdit::of(vec![set(
            view.id.clone(),
            vec![("filterMatch", or_null(any, text("any")))],
        )])
    } else {
        KnowledgeEdit::of(vec![set(id, vec![("match", text(if any { "any" } else { "all" }))])])
    }
}

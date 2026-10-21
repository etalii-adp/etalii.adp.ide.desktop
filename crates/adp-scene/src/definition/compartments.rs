//! Lists inside a node (DISL 0.4 section 6.9 `compartments`; the standalone host's
//! `canvas/library/definition/compartments.ts` at 8b6ccfc): a list's rows are the model's own entries, grouped,
//! foldable and tall enough to make the element grow.
//!
//! The standalone reads the rows, their groups and which headings are folded from a payload its backend computed.
//! Here the rows are the node's children in the slot, of the types the compartment names, or the list its CEL gives;
//! a row's text, link and order key are evaluated in the `compartmentItem` context (`self`, `item`, `index`). Which
//! headings are folded is the compartment's and its groups' `collapsed`, unless the host says otherwise
//! ([`Folding`]): the reader's own folding is viewer state the host keeps.

use std::collections::HashSet;

use adp_cel::Value;
use adp_disl::DislElement;
use adp_disl::expressions::contexts;

use super::binding::text_of;
use crate::evaluation::{Evaluator, Vars};
use crate::geometry::Rect;
use crate::text::{LABEL_FONT_SIZE, TextMetrics, fit_to_width};

/// The room a row keeps free for its link symbol.
pub const ROW_LINK_WIDTH: f64 = 16.0;

/// Where a list's rows come from.
#[derive(Debug, Clone, PartialEq)]
pub enum CompartmentItems {
    /// The node's children of these types in the slot.
    Children { types: Vec<String>, slot: String },
    /// A CEL list in the element context.
    Cel(String),
}

/// A list's groups: one per value of the items' enum attribute, in the enum's order.
#[derive(Debug, Clone, PartialEq)]
pub struct Grouping {
    pub attribute: String,
    /// Each value and its title.
    pub groups: Vec<(String, String)>,
    /// The groups that come folded.
    pub collapsed: Vec<String>,
    pub other_title: String,
}

/// One list of a node.
#[derive(Debug, Clone, PartialEq)]
pub struct CompartmentDeclaration {
    pub id: String,
    pub items: CompartmentItems,
    /// A row's text, CEL over its item.
    pub text: String,
    pub link: Option<String>,
    /// The order key, CEL over the item, and whether it ascends.
    pub order_by: Option<(String, bool)>,
    pub group_by: Option<Grouping>,
    pub title: Option<String>,
    /// A list with no title, no groups and nothing to fold it by is its rows alone.
    pub headless: bool,
    /// Whether the list itself comes folded.
    pub collapsed: bool,
    pub top: f64,
    pub heading_height: f64,
    pub row_height: f64,
    pub bottom: f64,
    pub inset_x: f64,
    pub row_indent: f64,
}

/// A heading, laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutHeading {
    pub compartment_id: String,
    pub key: String,
    pub title: String,
    pub count: usize,
    pub collapsed: bool,
    pub bounds: Rect,
}

/// A row, laid out.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutRow {
    pub compartment_id: String,
    pub group_key: String,
    pub id: Option<String>,
    pub text: String,
    pub full_text: String,
    pub link: Option<String>,
    pub bounds: Rect,
    pub text_x: f64,
}

/// A node's lists, laid out, and where they end.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LaidOutCompartments {
    pub headings: Vec<LaidOutHeading>,
    pub rows: Vec<LaidOutRow>,
    pub bottom: f64,
}

/// Which headings the host has folded or opened against their defaults, by compartment and group key.
#[derive(Debug, Clone, Default)]
pub struct Folding {
    pub folded: HashSet<(String, String)>,
    pub opened: HashSet<(String, String)>,
}

struct Group {
    key: String,
    title: String,
    rows: Vec<Row>,
}

struct Row {
    item: Value,
    index: usize,
}

fn item_vars(vars: &Vars, row: &Row) -> Vars {
    let mut vars = vars.clone();
    vars.insert("item".to_owned(), row.item.clone());
    vars.insert("index".to_owned(), Value::Int(row.index as i64));
    vars
}

fn item_text(evaluator: &Evaluator, source: &str, vars: &Vars, row: &Row) -> Option<String> {
    match evaluator.evaluate(contexts::COMPARTMENT_ITEM, source, &item_vars(vars, row)) {
        Ok(Value::Null) | Err(_) => None,
        Ok(value) => {
            let text = text_of(&value);
            (!text.is_empty()).then_some(text)
        }
    }
}

fn items_of(evaluator: &Evaluator, declaration: &CompartmentDeclaration, vars: &Vars) -> Vec<Value> {
    match &declaration.items {
        CompartmentItems::Children { types, slot } => {
            let Some(element) = vars.get("self").and_then(DislElement::from_value) else {
                return Vec::new();
            };
            element
                .children()
                .into_iter()
                .filter(|child| child.slot().as_deref().is_none_or(|s| s == slot))
                .filter(|child| types.is_empty() || types.iter().any(|t| child.is_a(t)))
                .map(|child| child.to_value())
                .collect()
        }
        CompartmentItems::Cel(source) => match evaluator.evaluate(contexts::ELEMENT, source, vars) {
            Ok(Value::List(items)) => items.to_vec(),
            _ => Vec::new(),
        },
    }
}

fn ordered(evaluator: &Evaluator, declaration: &CompartmentDeclaration, vars: &Vars, rows: Vec<Row>) -> Vec<Row> {
    let Some((by, ascending)) = &declaration.order_by else {
        return rows;
    };
    let mut keyed: Vec<(Option<String>, Row)> = rows
        .into_iter()
        .map(|row| (item_text(evaluator, by, vars, &row), row))
        .collect();
    // Stable, so rows that compare equal, and rows with no key, keep the order they came in; rows with no key last.
    keyed.sort_by(|(left, _), (right, _)| match (left, right) {
        (Some(l), Some(r)) => {
            let compared = l.cmp(r);
            if *ascending { compared } else { compared.reverse() }
        }
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
    });
    keyed.into_iter().map(|(_, row)| row).collect()
}

fn attribute_text(item: &Value, attribute: &str) -> Option<String> {
    let value = match DislElement::from_value(item) {
        Some(element) => element.value_of(attribute),
        None => item
            .as_map()
            .and_then(|m| m.get(attribute))
            .cloned()
            .unwrap_or(Value::Null),
    };
    let text = text_of(&value);
    (!text.is_empty()).then_some(text)
}

fn groups_of(evaluator: &Evaluator, declaration: &CompartmentDeclaration, vars: &Vars) -> Vec<Group> {
    let all: Vec<Row> = items_of(evaluator, declaration, vars)
        .into_iter()
        .enumerate()
        .map(|(index, item)| Row { item, index })
        .collect();
    if all.is_empty() {
        return Vec::new();
    }
    let Some(grouping) = &declaration.group_by else {
        return vec![Group {
            key: declaration.id.clone(),
            title: declaration.title.clone().unwrap_or_default(),
            rows: ordered(evaluator, declaration, vars, all),
        }];
    };
    let mut buckets: Vec<Vec<Row>> = grouping.groups.iter().map(|_| Vec::new()).collect();
    let mut others = Vec::new();
    for row in all {
        let value = attribute_text(&row.item, &grouping.attribute);
        match grouping.groups.iter().position(|(v, _)| Some(v) == value.as_ref()) {
            Some(at) => buckets[at].push(row),
            None => others.push(row),
        }
    }
    let mut groups: Vec<Group> = grouping
        .groups
        .iter()
        .zip(buckets)
        .map(|((value, title), rows)| Group {
            key: value.clone(),
            title: title.clone(),
            rows: ordered(evaluator, declaration, vars, rows),
        })
        .collect();
    if !others.is_empty() {
        groups.push(Group {
            key: String::new(),
            title: grouping.other_title.clone(),
            rows: ordered(evaluator, declaration, vars, others),
        });
    }
    groups.into_iter().filter(|g| !g.rows.is_empty()).collect()
}

fn is_folded(declaration: &CompartmentDeclaration, key: &str, folding: &Folding) -> bool {
    let id = (declaration.id.clone(), key.to_owned());
    if folding.folded.contains(&id) {
        return true;
    }
    if folding.opened.contains(&id) {
        return false;
    }
    declaration.collapsed
        || declaration
            .group_by
            .as_ref()
            .is_some_and(|g| g.collapsed.iter().any(|c| c == key))
}

/// What a node's lists are laid out with.
pub struct CompartmentContext<'a> {
    pub evaluator: &'a Evaluator,
    pub vars: &'a Vars,
    pub folding: &'a Folding,
    pub metrics: &'a dyn TextMetrics,
}

/// A node's lists laid out within `bounds` (`layoutCompartments`): one heading per group, its rows beneath it unless
/// it is folded; a list that draws nothing takes no room.
pub fn layout_compartments(
    declarations: &[CompartmentDeclaration],
    context: &CompartmentContext<'_>,
    bounds: Rect,
) -> LaidOutCompartments {
    let mut laid_out = LaidOutCompartments {
        bottom: bounds.y,
        ..LaidOutCompartments::default()
    };
    let mut cursor: Option<f64> = None;
    for declaration in declarations {
        let groups = groups_of(context.evaluator, declaration, context.vars);
        let mut y = cursor.unwrap_or(bounds.y + declaration.top);
        for group in &groups {
            let folded = !declaration.headless && is_folded(declaration, &group.key, context.folding);
            if !declaration.headless {
                laid_out.headings.push(LaidOutHeading {
                    compartment_id: declaration.id.clone(),
                    key: group.key.clone(),
                    title: group.title.clone(),
                    count: group.rows.len(),
                    collapsed: folded,
                    bounds: Rect::new(
                        bounds.x + declaration.inset_x,
                        y,
                        bounds.width - 2.0 * declaration.inset_x,
                        declaration.heading_height,
                    ),
                });
                y += declaration.heading_height;
            }
            if folded {
                continue;
            }
            for row in &group.rows {
                let x = bounds.x + declaration.inset_x + declaration.row_indent;
                let width = (bounds.width - 2.0 * declaration.inset_x - declaration.row_indent).max(0.0);
                let full_text = item_text(context.evaluator, &declaration.text, context.vars, row).unwrap_or_default();
                laid_out.rows.push(LaidOutRow {
                    compartment_id: declaration.id.clone(),
                    group_key: group.key.clone(),
                    id: DislElement::from_value(&row.item)
                        .map(|e| e.id())
                        .or_else(|| attribute_text(&row.item, "id")),
                    text: fit_to_width(
                        context.metrics,
                        &full_text,
                        (width - ROW_LINK_WIDTH).max(0.0),
                        LABEL_FONT_SIZE,
                    ),
                    full_text,
                    link: declaration
                        .link
                        .as_ref()
                        .and_then(|link| item_text(context.evaluator, link, context.vars, row)),
                    bounds: Rect::new(x, y, width, declaration.row_height),
                    text_x: x,
                });
                y += declaration.row_height;
            }
        }
        if !groups.is_empty() {
            cursor = Some(y);
            laid_out.bottom = y + declaration.bottom;
        }
    }
    laid_out
}

/// How tall a node's lists make it: laid out from y = 0, since heights do not depend on the width; none where they
/// draw nothing (`compartmentsHeight`).
pub fn compartments_height(declarations: &[CompartmentDeclaration], context: &CompartmentContext<'_>) -> Option<f64> {
    if declarations.is_empty() {
        return None;
    }
    let laid_out = layout_compartments(declarations, context, Rect::default());
    (!laid_out.headings.is_empty() || !laid_out.rows.is_empty()).then_some(laid_out.bottom)
}

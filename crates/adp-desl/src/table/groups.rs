//! The groups of a view that is grouped by a column (the standalone host's `KnowledgeGroups` at 8b6ccfc): which
//! there are, what each is called, and in which order the view has them. One place, so that what the table draws,
//! what the group menu lists and what a gesture moves are the same groups.

use std::collections::HashSet;

use super::content::{ContentColumn, ContentView, TableContent};
use super::engine::{LabelOf, NO_VALUE, values_of};
use crate::loading::specification::Editor;

/// Every group of the grouping, hidden ones included, in the view's order: the groups its order names first, then
/// the others as they come - an option each in the options' order, unticked and ticked, or a related row each as the
/// rows name them - and last the group of rows without a value, which a checkbox has none of. A related row is
/// called by `label_of`, and by its id where that gives nothing.
pub fn of(
    table: &TableContent,
    view: &ContentView,
    grouped: &ContentColumn,
    label_of: Option<LabelOf<'_>>,
) -> Vec<(String, String)> {
    let mut natural: Vec<(String, String)> = match grouped.kind.editor {
        Editor::Choice | Editor::Choices => grouped.options.iter().map(|o| (o.id.clone(), o.name.clone())).collect(),
        Editor::Checkbox => vec![
            ("false".into(), "Not checked".into()),
            ("true".into(), "Checked".into()),
        ],
        _ => {
            let mut seen = HashSet::new();
            table
                .rows
                .iter()
                .flat_map(|row| values_of(row, grouped).iter())
                .filter(|id| seen.insert(id.as_str()))
                .map(|id| {
                    let title = label_of.map(|l| l(&grouped.id, id)).unwrap_or_default();
                    (id.clone(), if title.is_empty() { id.clone() } else { title })
                })
                .collect()
        }
    };
    if grouped.kind.editor != Editor::Checkbox {
        natural.push((NO_VALUE.into(), format!("No {}", grouped.name)));
    }
    let mut order: Vec<String> = Vec::new();
    for key in &view.group_order {
        if natural.iter().any(|(k, _)| k == key) && !order.contains(key) {
            order.push(key.clone());
        }
    }
    for (key, _) in &natural {
        if !order.contains(key) {
            order.push(key.clone());
        }
    }
    order
        .into_iter()
        .map(|key| {
            let label = natural
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, l)| l.clone())
                .unwrap_or_default();
            (key, label)
        })
        .collect()
}

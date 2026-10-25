//! A node's labels changed on the page (etalii.adp spec 036, T047): the standalone host's `labels/TagField.tsx`,
//! `labels/listEditing.ts` and `classification/ClassificationPicker.tsx` at a7fdf03.
//!
//! - A label of many values with an `edit`, the first of a box's labels, is its field: its values, pressed, open its
//!   list, as does the control among the box's icons. The list holds the values carried, each taken off by its cross,
//!   and the label's members searched by typing, a member picked to carry it, picked again to take it off, or created
//!   from text that names none.
//! - A label of one value with a `pick` is its picker, after the other labels: its value, or on the selected box the
//!   way to give one, opens the label's members, where one is picked or the value cleared.
//! - From either, the label's list is edited in the table's options editor ([`crate::table::options`]), reused as it
//!   is: each member renamed, recoloured, put in its place and deleted, asking first in the definition's words when a
//!   node carries it, and for a picker a new member added there too.
//! - Every change is one gesture, sent as it is made; what the file then says is the page's next model. While a list
//!   is open, the page's keys are its own; Escape or a press outside closes it.
//!
//! It knows no designer type: the labels, their operations, params and words are the definition's.

use std::collections::BTreeMap;

use adp_desl::hierarchy::definition::{DOCUMENT_TARGET, LabelDefinition, ListDeletion, ListParams};
use adp_desl::hierarchy::model::{SurfaceNode, SurfaceOption};
use adp_desl::table::model::{TableColumn, TableGesture, TableOption};
use eframe::egui::{self, Align2, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2, accesskit::Role};

use super::HierarchyTab;
use super::search::{SearchList, Searched, search_list};
use crate::a11y;
use crate::icons;
use crate::table::options::{OptionsEditing, USES_PREFIX, options_editor};
use crate::table::view::Colours;

/// The list's width, the gap between it and what it opened from, and its padding.
const LIST_WIDTH: f32 = 280.0;
const GAP: f32 = 4.0;
const PADDING: f32 = 8.0;

/// CEL's `lowerAscii`, as the specification compares a label's values with its members.
pub fn lower(text: &str) -> String {
    text.to_ascii_lowercase()
}

/// The values of `label` that `node` carries.
pub fn values_of<'n>(node: &'n SurfaceNode, label: &str) -> &'n [String] {
    node.labels
        .iter()
        .find(|v| v.label == label)
        .map_or(&[], |v| v.values.as_slice())
}

/// A label's list as the table's options editor edits a column's: its members in their colours, and under the setting
/// the editor reads, how many nodes carry each, which a delete asks about (`listColumn`).
pub fn list_column(label: &str, name: &str, options: &[SurfaceOption], nodes: &[SurfaceNode]) -> TableColumn {
    let mut column = TableColumn {
        id: label.to_owned(),
        name: name.to_owned(),
        kind: "options".to_owned(),
        options: options
            .iter()
            .map(|o| TableOption {
                id: o.id.clone(),
                name: o.name.clone(),
                color: o.colour.clone(),
            })
            .collect(),
        visible: true,
        ..TableColumn::default()
    };
    for option in options {
        let uses = nodes
            .iter()
            .filter(|n| values_of(n, label).iter().any(|v| lower(v) == lower(&option.name)))
            .count();
        column
            .settings
            .insert(format!("{USES_PREFIX}{}", option.id), uses.to_string());
    }
    column
}

/// What a label's list is edited by: an operation for each change, and their params; `add` is absent when a member is
/// made by carrying it.
pub struct ListOperations<'d> {
    pub rename: &'d str,
    pub recolour: &'d str,
    pub reorder: &'d str,
    pub delete: &'d str,
    pub add: Option<&'d str>,
    pub params: &'d ListParams,
}

/// A gesture to send: its operation, its target and its params.
pub type Sent = (String, String, BTreeMap<String, String>);

/// The gesture a change the options editor raised sends, naming the member it changes (`listRaise`); none for an add
/// where a member is made by carrying it.
pub fn list_gesture(operations: &ListOperations, options: &[SurfaceOption], gesture: &TableGesture) -> Option<Sent> {
    let params = operations.params;
    let param = |name: &str, value: &str| BTreeMap::from([(name.to_owned(), value.to_owned())]);
    let first = gesture.values.first().cloned().unwrap_or_default();
    let target = gesture.target_id.clone();
    match gesture.kind.as_str() {
        "addOption" => operations
            .add
            .map(|add| (add.to_owned(), DOCUMENT_TARGET.to_owned(), param(&params.name, &first))),
        "renameOption" => Some((operations.rename.to_owned(), target, param(&params.name, &first))),
        "recolourOption" => Some((
            operations.recolour.to_owned(),
            target,
            param(&params.colour, gesture.setting_of("colour")),
        )),
        "moveOption" => {
            // A place in the list is the member it goes after, among those that stay where they are.
            let others: Vec<&SurfaceOption> = options.iter().filter(|o| o.id != target).collect();
            let at = usize::try_from(gesture.index).unwrap_or(0);
            let after = match at {
                0 => String::new(),
                _ => others.get(at - 1).map(|o| o.id.clone()).unwrap_or_default(),
            };
            Some((operations.reorder.to_owned(), target, param(&params.after, &after)))
        }
        "deleteOption" => Some((operations.delete.to_owned(), target, BTreeMap::new())),
        _ => None,
    }
}

/// The label of `node` that is its field: the first of many values the page changes, of those its type carries.
pub fn field_of<'d>(labels: &'d [LabelDefinition], node: &SurfaceNode) -> Option<&'d LabelDefinition> {
    labels
        .iter()
        .find(|l| l.many && l.edit.is_some() && l.on.contains(&node.type_name))
}

/// A label's list open on a node: the members searched, or, once Edit is pressed, the list's editor in its place.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelList {
    pub node_id: String,
    pub label: String,
    pub search: SearchList,
    pub editor: Option<OptionsEditing>,
    /// Where the list stood in the last frame, which its editor takes.
    pub at: Option<Pos2>,
}

impl LabelList {
    pub fn on(node_id: &str, label: &str) -> LabelList {
        LabelList {
            node_id: node_id.to_owned(),
            label: label.to_owned(),
            search: SearchList::default(),
            editor: None,
            at: None,
        }
    }
}

impl HierarchyTab {
    /// A label's field or picker pressed: the node selected and its list opened, or closed when it is the one open.
    pub fn toggle_label_list(&mut self, node_id: &str, label: &str, toggles: bool) {
        let open = self
            .label_list
            .as_ref()
            .is_some_and(|l| l.node_id == node_id && l.label == label && l.editor.is_none());
        self.select(node_id);
        if !open {
            self.label_list = Some(LabelList::on(node_id, label));
        } else if toggles {
            self.label_list = None;
        }
    }
}

/// What the list was asked this frame.
enum Asked {
    Gesture(Sent),
    Edit,
    Close,
}

/// Draws the label's list open on a node, under `anchor`, the place of the node's labels; or its editor, where the
/// list stood. Nothing when none is open, or the node or the label is gone.
pub fn show(ui: &mut egui::Ui, tab: &mut HierarchyTab, anchor: Option<Rect>, colours: &Colours, font: &FontId) {
    let Some(list) = tab.label_list.as_ref() else {
        return;
    };
    let node = tab.model.nodes.iter().find(|n| n.id == list.node_id).cloned();
    let definition = tab.definition.labels.iter().find(|l| l.label == list.label).cloned();
    let (Some(anchor), Some(node), Some(definition)) = (anchor, node, definition) else {
        tab.label_list = None;
        return;
    };
    if tab.read_only() || (definition.edit.is_none() && definition.pick.is_none()) {
        tab.label_list = None;
        return;
    }
    let ctx = ui.ctx().clone();
    let popup_open = egui::Popup::is_any_open(&ctx);
    let mut asked = Vec::new();
    if !popup_open && ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        asked.push(Asked::Close);
    }
    let options: Vec<SurfaceOption> = tab
        .model
        .option_lists
        .iter()
        .find(|l| l.label == definition.label)
        .map(|l| l.options.clone())
        .unwrap_or_default();
    let nodes = tab.model.nodes.clone();
    let id = tab.id().with(("labels", &node.id, &definition.label));
    let list = tab.label_list.as_mut().expect("the list is open");
    let screen = ctx.content_rect();
    let at = match (&list.editor, list.at) {
        (Some(_), Some(at)) => at,
        _ => Pos2::new(
            anchor.left().min(screen.right() - LIST_WIDTH - GAP).max(GAP),
            anchor.bottom() + GAP,
        ),
    };
    let (name, area) = if let Some(editing) = list.editor.as_mut() {
        let (words_list, deletion, operations) = list_words(&definition);
        let column = list_column(&definition.label, words_list, &options, &nodes);
        let area = egui::Area::new(id.with("editor"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(colours.surface)
                    .stroke(Stroke::new(1.0, colours.border))
                    .inner_margin(PADDING)
                    .show(ui, |ui| {
                        let question = |option: &TableOption, uses: usize| deletion.question(&option.name, uses);
                        let addable = operations.add.is_some();
                        for gesture in
                            options_editor(ui, editing, id.with("options"), &column, colours, addable, &question)
                        {
                            asked.extend(list_gesture(&operations, &options, &gesture).map(Asked::Gesture));
                        }
                    })
                    .response
            });
        (format!("Options of {words_list}"), area)
    } else {
        let area = egui::Area::new(id.with("list"))
            .order(egui::Order::Foreground)
            .fixed_pos(at)
            .show(&ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .fill(colours.surface)
                    .stroke(Stroke::new(1.0, colours.border))
                    .inner_margin(PADDING)
                    .show(ui, |ui| {
                        ui.set_width(LIST_WIDTH - 2.0 * PADDING);
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                        let drawn = Drawn {
                            node: &node,
                            definition: &definition,
                            options: &options,
                            colours,
                            font,
                            id,
                        };
                        drawn.list(ui, &mut list.search, &mut asked);
                    })
                    .response
            });
        list.at = Some(area.response.rect.left_top());
        (format!("{} of {}", list_name(&definition), node.number), area)
    };
    a11y::describe(&area.inner, Role::Dialog, &name);
    // A press outside it closes it, but not one on the labels it opened from, which open or close it by themselves, nor
    // one in a colour's choice, which stands outside it.
    let pressed = ctx.input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()).flatten());
    if let Some(at) = pressed
        && !popup_open
        && !area.response.rect.contains(at)
        && !anchor.contains(at)
    {
        asked.push(Asked::Close);
    }

    for ask in asked {
        match ask {
            Asked::Gesture((operation, target, params)) => tab.gesture(&operation, &target, params),
            Asked::Edit => {
                if let Some(list) = tab.label_list.as_mut() {
                    list.editor = Some(OptionsEditing::default());
                }
            }
            Asked::Close => tab.label_list = None,
        }
    }
}

/// What the list of `label` is called: the field's list, or the picked value's own word.
fn list_name(label: &LabelDefinition) -> &str {
    match (&label.edit, &label.pick) {
        (Some(edit), _) => &edit.words.list,
        (None, Some(pick)) => &pick.words.value,
        (None, None) => &label.label,
    }
}

/// The name of the list of `label` as its editor calls it, what deleting a member asks, and the operations it sends.
fn list_words(label: &LabelDefinition) -> (&str, &ListDeletion, ListOperations<'_>) {
    match (&label.edit, &label.pick) {
        (Some(edit), _) => (
            &edit.words.list,
            &edit.deletion,
            ListOperations {
                rename: &edit.operations.rename,
                recolour: &edit.operations.recolour,
                reorder: &edit.operations.reorder,
                delete: &edit.operations.delete,
                add: None,
                params: &edit.params,
            },
        ),
        (None, Some(pick)) => (
            &pick.words.list,
            &pick.deletion,
            ListOperations {
                rename: &pick.operations.rename,
                recolour: &pick.operations.recolour,
                reorder: &pick.operations.reorder,
                delete: &pick.operations.delete,
                add: Some(&pick.operations.add),
                params: &pick.params.list,
            },
        ),
        (None, None) => unreachable!("a list opens only for a label the page changes"),
    }
}

/// The list's contents, for one frame.
struct Drawn<'a> {
    node: &'a SurfaceNode,
    definition: &'a LabelDefinition,
    options: &'a [SurfaceOption],
    colours: &'a Colours,
    font: &'a FontId,
    id: egui::Id,
}

impl Drawn<'_> {
    fn list(&self, ui: &mut egui::Ui, search: &mut SearchList, asked: &mut Vec<Asked>) {
        let label = &self.definition.label;
        let values = values_of(self.node, label);
        let member = |value: &str| self.options.iter().find(|o| lower(&o.name) == lower(value));
        let items: Vec<(String, String)> = self.options.iter().map(|o| (o.id.clone(), o.name.clone())).collect();
        let node_id = self.node.id.clone();
        let number = &self.node.number;
        let gesture = |operation: &str, params: &[(&str, &str)]| {
            Asked::Gesture((
                operation.to_owned(),
                node_id.clone(),
                params.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
            ))
        };
        let words_list;
        if let Some(edit) = &self.definition.edit {
            words_list = &edit.words.list;
            let name = &edit.params.name;
            if !values.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    for value in values {
                        let colour = member(value).map_or("default", |o| o.colour.as_str());
                        ui.label(
                            RichText::new(value)
                                .color(self.colours.text)
                                .background_color(self.colours.option(colour)),
                        );
                        let remove = format!("{} {value}", edit.words.remove);
                        if self.icon(ui, "mdi-close", &remove, &edit.words.remove) {
                            asked.push(gesture(&edit.operations.remove, &[(name, value)]));
                        }
                    }
                });
            }
            let chosen: Vec<String> = self
                .options
                .iter()
                .filter(|o| values.iter().any(|v| lower(v) == lower(&o.name)))
                .map(|o| o.id.clone())
                .collect();
            let label = format!("{} to {number}", edit.words.add);
            let searched = search_list(
                ui,
                search,
                self.id.with("search"),
                &label,
                &items,
                &chosen,
                "Search or create…",
                true,
                self.colours,
            );
            match searched {
                Some(Searched::Picked(id)) => {
                    if let Some(option) = self.options.iter().find(|o| o.id == id) {
                        // Taken off with the spelling the node carries, put on with the list's.
                        match values.iter().find(|v| lower(v) == lower(&option.name)) {
                            Some(carried) => asked.push(gesture(&edit.operations.remove, &[(name, carried)])),
                            None => asked.push(gesture(&edit.operations.add, &[(name, &option.name)])),
                        }
                    }
                }
                Some(Searched::Created(text)) => asked.push(gesture(&edit.operations.add, &[(name, &text)])),
                None => {}
            }
        } else if let Some(pick) = &self.definition.pick {
            words_list = &pick.words.list;
            let value = values.first();
            let chosen: Vec<String> = value
                .and_then(|v| member(v))
                .map(|o| vec![o.id.clone()])
                .unwrap_or_default();
            let label = format!("{} of {number}", pick.words.value);
            let searched = search_list(
                ui,
                search,
                self.id.with("search"),
                &label,
                &items,
                &chosen,
                "Search…",
                false,
                self.colours,
            );
            if let Some(Searched::Picked(id)) = searched
                && let Some(option) = self.options.iter().find(|o| o.id == id)
            {
                asked.push(Asked::Close);
                // The value it has is no change.
                if value.is_none_or(|v| lower(v) != lower(&option.name)) {
                    asked.push(gesture(&pick.operations.set, &[(&pick.params.value, &option.name)]));
                }
            }
            if value.is_some() && self.text_button(ui, "mdi-close", &pick.words.clear) {
                asked.push(Asked::Close);
                asked.push(gesture(&pick.operations.clear, &[]));
            }
        } else {
            return;
        }
        if self.text_button(ui, "mdi-pencil-outline", &format!("Edit {}", words_list.to_lowercase())) {
            asked.push(Asked::Edit);
        }
    }

    /// A small control showing `icon`, named `name`; whether it was pressed.
    fn icon(&self, ui: &mut egui::Ui, icon: &str, name: &str, tooltip: &str) -> bool {
        let (place, response) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
        a11y::describe(&response, Role::Button, name);
        let colour = if response.hovered() {
            self.colours.text
        } else {
            self.colours.muted
        };
        ui.painter().text(
            place.center(),
            Align2::CENTER_CENTER,
            icons::text(icon),
            icons::font(14.0),
            colour,
        );
        response.on_hover_text(tooltip).clicked()
    }

    /// A row's button with `icon` before `text`, named by its text; whether it was pressed.
    fn text_button(&self, ui: &mut egui::Ui, icon: &str, text: &str) -> bool {
        let mut job = egui::text::LayoutJob::default();
        job.append(
            &icons::text(icon),
            0.0,
            egui::TextFormat::simple(icons::font(14.0), self.colours.muted),
        );
        job.append(
            text,
            6.0,
            egui::TextFormat::simple(self.font.clone(), self.colours.text),
        );
        let response = ui.add(egui::Button::new(job).frame(false));
        a11y::describe(&response, Role::Button, text);
        response.clicked()
    }
}

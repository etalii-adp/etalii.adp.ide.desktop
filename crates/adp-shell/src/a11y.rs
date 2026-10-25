//! What the shell's own widgets tell the platform's accessibility interface (etalii.adp spec 030, FR-041, task T088).
//!
//! egui names its stock widgets itself: a button by its text, a text field by its value, a checkbox by its label
//! and tick. The shell draws some of its controls by hand - the explorer's rows, the tabs, the add dialog's tree, the
//! activity bar's chevron - and gives others a role egui cannot guess, such as a context menu's entries. These
//! helpers give each the role, the name and the state a screen reader announces, through AccessKit, so that the
//! window reads as VS Code's does: a tree of tree items, a tab list of tabs, a menu of menu items, a dialog by its
//! title.

use eframe::egui::{
    Response, UiBuilder, WidgetInfo,
    accesskit::{AriaCurrent, Role},
};

/// Gives the widget of `response` `role` and `name`, in place of what egui inferred. A widget egui drew carries its
/// bounds and actions already; one the shell drew by hand gets them here.
pub fn describe(response: &Response, role: Role, name: &str) {
    if !response.ctx.has_accesskit_node(response.id) {
        let enabled = response.enabled();
        response.widget_info(|| WidgetInfo::labeled(role, enabled, name));
    }
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_role(role);
        // A label carries its text as its value; a control is named by its label.
        node.clear_value();
        node.set_label(name);
    });
}

/// Leaves the widget of `response` out of what a screen reader reads: an icon that only decorates the text beside it,
/// whose glyph is a private-use character a reader would spell out.
pub fn decoration(response: &Response) {
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.clear_value();
        node.clear_label();
        node.set_hidden();
    });
}

/// Names a field egui drew, keeping its role and its value: what a screen reader reads before the text typed in it.
pub fn label(response: &Response, name: &str) {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_label(name));
}

/// A row of a tree: its name, its depth from one, whether it is selected and, when it has children, whether they show.
pub fn tree_item(response: &Response, name: &str, level: usize, selected: bool, expanded: Option<bool>) {
    describe(response, Role::TreeItem, name);
    response.ctx.accesskit_node_builder(response.id, |node| {
        node.set_level(level);
        node.set_selected(selected);
        match expanded {
            Some(open) => node.set_expanded(open),
            None => node.clear_expanded(),
        }
    });
}

/// A tab of a tab list: its name and whether it is the one showing.
pub fn tab(response: &Response, name: &str, selected: bool) {
    describe(response, Role::Tab, name);
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_selected(selected));
}

/// An entry of a menu, with the key that does the same when it has one.
pub fn menu_item(response: &Response, name: &str, shortcut: &str) {
    describe(response, Role::MenuItem, name);
    response.ctx.accesskit_node_builder(response.id, |node| {
        if shortcut.is_empty() {
            node.clear_keyboard_shortcut();
        } else {
            node.set_keyboard_shortcut(shortcut);
        }
    });
}

/// A button named `name`, with the key that does the same when it has one: an action offered outside a menu.
pub fn button(response: &Response, name: &str, shortcut: &str) {
    describe(response, Role::Button, name);
    response.ctx.accesskit_node_builder(response.id, |node| {
        if shortcut.is_empty() {
            node.clear_keyboard_shortcut();
        } else {
            node.set_keyboard_shortcut(shortcut);
        }
    });
}

/// A part of a page that holds others, such as a landmark, a list or a tree: what a scope drawn with this builder is
/// to a screen reader, its children inside it.
pub fn region(role: Role, name: &str) -> UiBuilder {
    UiBuilder::new().accessibility_role(role).accessibility_label(name)
}

/// Marks the widget of `response` as the current one of the set it stands in, as a trail's last step is.
pub fn current(response: &Response) {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_aria_current(AriaCurrent::Page));
}

/// Says the widget of `response` is busy, or no longer, as a page waiting on an edit's answer is.
pub fn busy(response: &Response, busy: bool) {
    response.ctx.accesskit_node_builder(response.id, |node| {
        if busy {
            node.set_busy();
        } else {
            node.clear_busy();
        }
    });
}

/// Says the widget of `response` is `description` besides its name, such as a tab's unsaved changes.
pub fn describe_state(response: &Response, description: &str) {
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_description(description));
}

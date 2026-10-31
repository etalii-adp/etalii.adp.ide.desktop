//! A tool type's DISL specification, loaded once, and what every diagram derives from it the same way (etalii.adp
//! spec 030, FR-015, FR-017 and FR-018): the palette, the context menus and the property rows mapped onto the host's
//! types with the wire ids of the specification's `x-` block, and a finding's severity as the problems panel counts
//! it. Ported from the standalone host's `EtAlii.Adp.Diagram.Modules/DislDiagramDefinition.cs` and
//! `DislToolboxProvider.cs` at `8b6ccfc`.
//!
//! The derivations are `adp_disl`'s; this maps their results onto `adp_context`. A shortcut is sent as a browser
//! spells the key, with its modifiers, which is how the shell matches a keypress: `Alt+Up` is `ArrowUp` with Alt.
//! The standalone host's "standing operations" (an available entry carrying its first `unavailable` sentence) are
//! not ported: `adp_context::Action` reads a reason as unavailability, and no tool type of this port uses them.

use adp_context::{Action, ActionGroup, PropertyEditor, PropertyRow, Shortcut};
use adp_disl::derivation::{context_menu, forms, toolbox};
use adp_disl::{
    DerivedMenuEntry, DerivedRow, DislDiagnostic, DislElement, DislEnv, DislFinding, DislLoader, DislMenuTarget,
    DislPluginFunctions, DislSeverity, DislSpecification, WireIdMap,
};

use crate::definition::ToolboxItem;
use crate::problems::DiagramProblemSeverity;

/// A tool type's loaded specification, with its wire ids and its palette.
#[derive(Debug, Clone)]
pub struct DislDiagramDefinition {
    specification: DislSpecification,
    diagnostics: Vec<DislDiagnostic>,
    wire_block: Option<String>,
    ids: WireIdMap,
    toolbox: Vec<ToolboxItem>,
}

impl DislDiagramDefinition {
    /// Loads the specification `bytes`, whose wire ids are in the top-level block `wire_block` (`x-cld`), with the
    /// host's implementations of its plugin functions. Without a block, every name is its own id. An error, with the
    /// loader's sentences, when the specification does not load.
    pub fn load(bytes: &[u8], wire_block: Option<&str>, plugins: &DislPluginFunctions) -> Result<Self, String> {
        let loaded = DislLoader::load_bytes(bytes, plugins);
        let Some(specification) = loaded.specification else {
            return Err(loaded
                .diagnostics
                .iter()
                .filter(|d| d.severity == DislSeverity::Error)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "));
        };
        Ok(Self::of(specification, loaded.diagnostics, wire_block))
    }

    /// A definition of a specification already loaded.
    pub fn of(specification: DislSpecification, diagnostics: Vec<DislDiagnostic>, wire_block: Option<&str>) -> Self {
        let ids = wire_block
            .map(|block| WireIdMap::of(&specification, block))
            .unwrap_or_else(WireIdMap::none);
        let toolbox = toolbox::derive(&specification, &ids)
            .into_iter()
            .map(|tool| ToolboxItem {
                id: tool.id,
                label: tool.label,
                icon: tool.icon,
                description: tool.description,
                drop_action_id: tool.drop_action_id.unwrap_or_default(),
            })
            .collect();
        DislDiagramDefinition {
            specification,
            diagnostics,
            wire_block: wire_block.map(str::to_owned),
            ids,
            toolbox,
        }
    }

    /// The wire block a specification declares: its first top-level `x-` member with `tools`, `actions` or
    /// `properties`, as every module's block is.
    pub fn wire_block_of(specification: &DislSpecification) -> Option<String> {
        specification.root().as_object()?.iter().find_map(|(name, block)| {
            (name.starts_with("x-")
                && ["tools", "actions", "properties"]
                    .iter()
                    .any(|member| block.get(member).is_some()))
            .then(|| name.clone())
        })
    }

    pub fn specification(&self) -> &DislSpecification {
        &self.specification
    }

    /// What the runtime had to say about the specification as it loaded it.
    pub fn diagnostics(&self) -> &[DislDiagnostic] {
        &self.diagnostics
    }

    pub fn ids(&self) -> &WireIdMap {
        &self.ids
    }

    /// The palette.
    pub fn toolbox(&self) -> &[ToolboxItem] {
        &self.toolbox
    }

    /// The derived entries of the context menu of `target`, in their groups.
    pub fn menu_entries(&self, target: &DislMenuTarget, env: &DislEnv) -> Vec<Vec<DerivedMenuEntry>> {
        context_menu::derive(&self.specification, target, env, &self.ids)
            .into_iter()
            .map(|group| group.entries)
            .collect()
    }

    /// The entry of the operation `action_id` names, for a gesture that names an action `target`'s menu does not offer.
    pub fn operation_entry(&self, target: &DislMenuTarget, env: &DislEnv, action_id: &str) -> Option<DerivedMenuEntry> {
        context_menu::operation_entry(&self.specification, target, env, &self.ids, action_id)
    }

    /// The context menu of `target`, as the shell shows it.
    pub fn menus(&self, target: &DislMenuTarget, env: &DislEnv) -> Vec<ActionGroup> {
        self.menu_entries(target, env)
            .into_iter()
            .map(|entries| ActionGroup {
                actions: entries.iter().map(Self::action).collect(),
            })
            .collect()
    }

    /// The property rows of `element`'s inspector form.
    pub fn rows(&self, element: &DislElement, env: &DislEnv) -> Vec<PropertyRow> {
        forms::derive(&self.specification, element, env, &self.ids, "inspector")
            .iter()
            .map(Self::row)
            .collect()
    }

    /// The property rows of `diagram`'s own inspector form; none when no form is for the diagram.
    pub fn diagram_rows(&self, diagram: &adp_disl::DislDiagram, env: &DislEnv) -> Vec<PropertyRow> {
        forms::derive_diagram(&self.specification, diagram, env, &self.ids, "inspector")
            .iter()
            .map(Self::row)
            .collect()
    }

    /// Whether a notation draws the element `id`: one none does, as a stored element everything is derived from, is
    /// nowhere the reader can be taken to.
    pub fn draws(&self, diagram: &adp_disl::DislDiagram, id: &str) -> bool {
        let Some(element) = diagram.element_by_id(id) else {
            return true;
        };
        let notation = &self.specification.root()["notation"];
        let type_name = element.type_name();
        let declared = |part: &str| notation[part].get(type_name.as_str()).is_some();
        declared("nodes") || declared("relations") || declared("edges") || notation.is_null()
    }

    /// The form item a row id names: the key the wire block maps to it, or the id itself.
    pub fn row_key(&self, property_id: &str) -> String {
        self.wire_block
            .as_deref()
            .and_then(|block| self.specification.root().get(block))
            .and_then(|block| block.get("properties"))
            .and_then(|properties| properties.as_object())
            .and_then(|properties| {
                properties
                    .iter()
                    .find(|(_, id)| id.as_str() == Some(property_id))
                    .map(|(key, _)| key.clone())
            })
            .unwrap_or_else(|| property_id.to_owned())
    }

    /// A menu entry as the shell shows it, its shortcut as a browser spells the key.
    pub fn action(entry: &DerivedMenuEntry) -> Action {
        let mut action = Action::new(&entry.id, &entry.label).with_icon(&entry.icon);
        if let Some(shortcut) = Self::shortcut_of(entry) {
            action = action.with_shortcut(shortcut);
        }
        if !entry.available {
            action = action.unavailable(&entry.unavailable_reason);
        }
        action
    }

    /// A menu entry's shortcut as a browser spells the key, with its modifiers; none when it has none. Ctrl and Meta
    /// are both the platform's primary modifier.
    pub fn shortcut_of(entry: &DerivedMenuEntry) -> Option<Shortcut> {
        entry.shortcut.as_ref().map(|key| Shortcut {
            key: key.event_key().to_owned(),
            primary: key.ctrl || key.meta,
            shift: key.shift,
            alt: key.alt,
        })
    }

    /// A row as the grid draws it: a retype row and a `select` are a choice among their candidates, a `textarea` the
    /// text editor, `tags`, `slider` and `switch` their own, anything else a line. Only a choice, the tags and the
    /// slider carry candidates.
    pub fn row(row: &DerivedRow) -> PropertyRow {
        let editor = if row.retypes {
            PropertyEditor::Choice
        } else {
            match row.widget.as_deref() {
                Some("select") => PropertyEditor::Choice,
                Some("textarea") => PropertyEditor::Text,
                Some("tags") => PropertyEditor::Tags,
                Some("slider") => PropertyEditor::Slider,
                Some("switch") => PropertyEditor::Toggle,
                _ => PropertyEditor::Line,
            }
        };
        let mut property = PropertyRow::new(&row.id, &row.label, editor, &row.value);
        property.read_only_reason = row.read_only_reason.clone();
        property.group = row.group.clone();
        if matches!(
            editor,
            PropertyEditor::Choice | PropertyEditor::Tags | PropertyEditor::Slider
        ) {
            property.candidates = row.candidates.clone().unwrap_or_default();
        }
        property
    }

    /// A finding's severity as the panel counts it: `error` an error, `info` and `hint` information, anything else a
    /// warning.
    pub fn severity_of(finding: &DislFinding) -> DiagramProblemSeverity {
        match finding.severity.as_str() {
            "error" => DiagramProblemSeverity::Error,
            "info" | "hint" => DiagramProblemSeverity::Info,
            _ => DiagramProblemSeverity::Warning,
        }
    }
}

#[cfg(test)]
mod tests {
    use adp_cel::Map;
    use adp_disl::DerivedShortcut;

    use super::*;

    fn entry(shortcut: Option<DerivedShortcut>, available: bool, reason: &str) -> DerivedMenuEntry {
        DerivedMenuEntry {
            id: "arrange".into(),
            kind: "operation".into(),
            operation: Some("arrange".into()),
            label: "Arrange".into(),
            icon: "mdi-auto-fix".into(),
            shortcut,
            available,
            unavailable_reason: reason.into(),
            arguments: Map::default(),
            via: None,
        }
    }

    fn derived(widget: Option<&str>, retypes: bool) -> DerivedRow {
        DerivedRow {
            id: "p".into(),
            label: "Label".into(),
            value: "value".into(),
            widget: widget.map(str::to_owned),
            read_only_reason: String::new(),
            group: "General".into(),
            candidates: Some(vec!["a".into(), "b".into()]),
            retypes,
        }
    }

    #[test]
    fn a_shortcut_is_sent_as_a_browser_spells_the_key_with_its_modifiers() {
        let action = DislDiagramDefinition::action(&entry(Some(DerivedShortcut::parse("Alt+Up")), true, ""));
        assert_eq!(action.shortcut, Some(Shortcut::key("ArrowUp").with_alt()));
    }

    #[test]
    fn an_entry_without_a_shortcut_is_sent_without_one() {
        assert_eq!(DislDiagramDefinition::action(&entry(None, true, "")).shortcut, None);
    }

    #[test]
    fn an_unavailable_entry_carries_its_reason_and_an_available_one_none() {
        let unavailable = DislDiagramDefinition::action(&entry(None, false, "Nothing to arrange."));
        let available = DislDiagramDefinition::action(&entry(None, true, ""));
        assert_eq!(
            (unavailable.unavailable_reason.as_str(), unavailable.is_available()),
            ("Nothing to arrange.", false)
        );
        assert_eq!(
            (available.unavailable_reason.as_str(), available.is_available()),
            ("", true)
        );
    }

    #[test]
    fn a_row_is_drawn_with_the_editor_of_its_widget() {
        for (widget, retypes, expected) in [
            (None, false, PropertyEditor::Line),
            (Some("text"), false, PropertyEditor::Line),
            (Some("number"), false, PropertyEditor::Line),
            (Some("textarea"), false, PropertyEditor::Text),
            (Some("select"), false, PropertyEditor::Choice),
            (Some("switch"), false, PropertyEditor::Toggle),
            (Some("tags"), false, PropertyEditor::Tags),
            (Some("slider"), false, PropertyEditor::Slider),
            (Some("textarea"), true, PropertyEditor::Choice),
        ] {
            assert_eq!(
                DislDiagramDefinition::row(&derived(widget, retypes)).editor,
                expected,
                "{widget:?}"
            );
        }
    }

    #[test]
    fn only_a_choice_tags_and_a_slider_carry_candidates() {
        for (widget, carries) in [
            ("select", true),
            ("tags", true),
            ("slider", true),
            ("textarea", false),
            ("switch", false),
        ] {
            let row = DislDiagramDefinition::row(&derived(Some(widget), false));
            assert_eq!(!row.candidates.is_empty(), carries, "{widget}");
        }
    }

    #[test]
    fn a_finding_is_counted_at_its_severity() {
        for (severity, expected) in [
            ("error", DiagramProblemSeverity::Error),
            ("warning", DiagramProblemSeverity::Warning),
            ("info", DiagramProblemSeverity::Info),
            ("hint", DiagramProblemSeverity::Info),
        ] {
            let finding = DislFinding {
                code: "x.code".into(),
                constraint_id: "rule".into(),
                severity: severity.into(),
                message: "Message.".into(),
                element_ids: Vec::new(),
                line: Some(1),
                file: None,
                subject: None,
            };
            assert_eq!(DislDiagramDefinition::severity_of(&finding), expected);
        }
    }
}

//! Steps 4 and 5 of DESL section 10.1: the specification read into its types, with every name it uses resolved -
//! the types and attributes the surface names, the options' and items' containment, the value types' keys, the
//! types a `mentions` facet and an id rule name - and the reserved attribute names of DESL section 2.2 refused.

use indexmap::IndexMap;

use super::diagnostic::DeslDiagnostic;
use super::specification::{
    DeslAttribute, DeslContainment, DeslElementType, DeslEnum, DeslEnumValue, DeslIdRule, DeslLanguage, DeslMetamodel,
    DeslOptions, DeslPersistence, DeslSurface, DeslTable, DeslValueType, Editor, SortKind,
};
use crate::json::{self, Json};

/// Attribute names that are members of every element or variables in CEL (DESL section 2.2), and CEL's keywords.
const RESERVED: &[&str] = &[
    "id",
    "type",
    "parent",
    "slot",
    "children",
    "document",
    "self",
    "item",
    "p",
    "count",
    "selection",
    "in",
    "as",
    "break",
    "const",
    "continue",
    "else",
    "for",
    "function",
    "if",
    "import",
    "let",
    "loop",
    "package",
    "namespace",
    "return",
    "var",
    "void",
    "while",
    "true",
    "false",
    "null",
];

/// The primitive types of DESL section 4.2.
const PRIMITIVES: &[&str] = &["string", "text", "int", "number", "bool", "date", "datetime", "time"];

/// Whether `name` may not be the name of an attribute (DESL section 2.2).
pub(crate) fn is_reserved(name: &str) -> bool {
    RESERVED.contains(&name) || name.starts_with('_') || name.starts_with('$')
}

/// A LocalizedText in the default locale: the string itself, or the `en` entry of an object, else its first.
pub(crate) fn localized(value: Option<&Json>) -> String {
    match value {
        Some(Json::String(text)) => text.clone(),
        Some(Json::Object(map)) => map
            .get("en")
            .or_else(|| map.values().next())
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned(),
        _ => String::new(),
    }
}

pub(crate) fn language(root: &Json) -> DeslLanguage {
    let language = json::member(root, "language");
    let origin = json::string_of(language, "origin").unwrap_or_default().to_owned();
    let label = localized(json::get(language, "label"));
    DeslLanguage {
        id: json::string_of(language, "id").unwrap_or_default().to_owned(),
        version: json::string_of(language, "version").unwrap_or_default().to_owned(),
        label: if label.is_empty() { origin.clone() } else { label },
        origin,
        icon: json::string_of(language, "icon").unwrap_or_default().to_owned(),
    }
}

fn attribute(value: &Json) -> DeslAttribute {
    DeslAttribute {
        type_name: json::string_of(value, "type").unwrap_or_default().to_owned(),
        required: json::bool_of(value, "required"),
        many: json::bool_of(value, "many"),
        read_only: json::bool_of(value, "readOnly"),
        default: json::get(value, "default").cloned(),
    }
}

fn containment(value: Option<&Json>) -> Option<DeslContainment> {
    let value = value.filter(|v| v.is_object())?;
    Some(DeslContainment {
        allowed: json::strings_of(value, "allowed"),
        ordered: json::bool_of(value, "ordered"),
        slots: json::members(value, "slots")
            .map(|(name, slot)| (name.clone(), json::strings_of(slot, "allowed")))
            .collect(),
    })
}

fn element_type(value: &Json) -> DeslElementType {
    DeslElementType {
        label: localized(json::get(value, "label")),
        attributes: json::members(value, "attributes")
            .map(|(name, a)| (name.clone(), attribute(a)))
            .collect(),
        children: containment(json::get(value, "children")),
        label_attribute: json::string_of(value, "labelAttribute").map(str::to_owned),
    }
}

pub(crate) fn metamodel(root: &Json, diagnostics: &mut Vec<DeslDiagnostic>) -> DeslMetamodel {
    let metamodel = json::member(root, "metamodel");
    let model = DeslMetamodel {
        document: element_type(json::member(metamodel, "document")),
        types: json::members(metamodel, "types")
            .map(|(name, t)| (name.clone(), element_type(t)))
            .collect(),
        enums: json::members(metamodel, "enums")
            .map(|(name, e)| {
                let values = json::members(e, "values")
                    .map(|(key, v)| DeslEnumValue {
                        key: key.clone(),
                        value: json::string_of(v, "value").unwrap_or(key).to_owned(),
                        label: localized(json::get(v, "label")),
                    })
                    .collect();
                (
                    name.clone(),
                    DeslEnum {
                        values,
                        ordered: json::bool_of(e, "ordered"),
                    },
                )
            })
            .collect(),
    };

    // Element types and enumerations share one namespace (DESL section 2.2).
    for name in model.enums.keys() {
        if model.types.contains_key(name) {
            diagnostics.push(DeslDiagnostic::error(
                json::pointer("/metamodel/enums", name),
                format!("'{name}' is both an element type and an enumeration; they share one namespace (DESL §2.2)."),
            ));
        }
    }

    let known =
        |name: &str| PRIMITIVES.contains(&name) || model.types.contains_key(name) || model.enums.contains_key(name);
    let document_pointer = "/metamodel/document";
    let owners = std::iter::once((document_pointer.to_owned(), &model.document)).chain(
        model
            .types
            .iter()
            .map(|(name, t)| (json::pointer("/metamodel/types", name), t)),
    );
    for (pointer, owner) in owners {
        for (name, a) in &owner.attributes {
            let at = json::pointer(&json::pointer(&pointer, "attributes"), name);
            if is_reserved(name) {
                diagnostics.push(DeslDiagnostic::error(
                    at.clone(),
                    format!("'{name}' is a reserved name and cannot be an attribute (DESL §2.2)."),
                ));
            }
            if !known(&a.type_name) {
                diagnostics.push(DeslDiagnostic::error(
                    json::pointer(&at, "type"),
                    format!(
                        "The attribute '{name}' has the type '{}', which is no primitive, enumeration or element type (DESL §4.3).",
                        a.type_name
                    ),
                ));
            }
        }
        if let Some(children) = &owner.children {
            for allowed in children.all_allowed() {
                if !model.types.contains_key(allowed) {
                    diagnostics.push(DeslDiagnostic::error(
                        json::pointer(&pointer, "children"),
                        format!("The containment allows '{allowed}', which is no element type (DESL §4.6)."),
                    ));
                }
            }
        }
    }
    let mentions = json::members(metamodel, "types").flat_map(|(type_name, t)| {
        json::members(t, "attributes").filter_map(move |(name, a)| {
            json::get(a, "mentions").map(|m| {
                (
                    json::pointer(
                        &json::pointer(&json::pointer("/metamodel/types", type_name), "attributes"),
                        name,
                    ),
                    json::strings_of(m, "types"),
                )
            })
        })
    });
    for (pointer, types) in mentions {
        for named in types {
            if !model.types.contains_key(&named) {
                diagnostics.push(DeslDiagnostic::error(
                    json::pointer(&pointer, "mentions"),
                    format!("A mention may name '{named}', which is no element type (DESL §4.3)."),
                ));
            }
        }
    }
    model
}

fn id_rule(value: &Json, base: Option<&DeslIdRule>) -> DeslIdRule {
    let base = base.cloned().unwrap_or_default();
    DeslIdRule {
        strategy: json::string_of(value, "strategy")
            .map(str::to_owned)
            .unwrap_or(base.strategy),
        expression: json::get(value, "expression")
            .and_then(|e| e.as_str().or_else(|| json::string_of(e, "cel")))
            .map(str::to_owned),
        ephemeral: json::bool_of(value, "ephemeral"),
        prefix: json::string_of(value, "prefix").map(str::to_owned),
        digits: json::int_of(value, "digits"),
        mark: json::string_of(value, "mark").map(str::to_owned),
        placeholder: json::string_of(value, "placeholder").map(str::to_owned),
    }
}

pub(crate) fn persistence(
    root: &Json,
    metamodel: &DeslMetamodel,
    diagnostics: &mut Vec<DeslDiagnostic>,
) -> DeslPersistence {
    let persistence = json::member(root, "persistence");
    let ids = json::member(persistence, "ids");
    let mut top = id_rule(ids, None);
    if top.strategy.is_empty() {
        top.strategy = "uuid-v7".into();
    }
    let id_rules: IndexMap<String, DeslIdRule> = json::members(ids, "types")
        .map(|(name, rule)| (name.clone(), id_rule(rule, Some(&top))))
        .collect();
    for (name, rule) in &id_rules {
        let pointer = json::pointer("/persistence/ids/types", name);
        if !metamodel.types.contains_key(name) {
            diagnostics.push(DeslDiagnostic::error(
                pointer.clone(),
                format!("An id rule is given for '{name}', which is no element type (DESL §5.3)."),
            ));
        }
        if rule.strategy == "derived" && rule.expression.is_none() {
            diagnostics.push(DeslDiagnostic::error(
                pointer.clone(),
                format!("The derived id rule of '{name}' has no expression (DESL §5.3.2)."),
            ));
        }
        if rule.strategy == "counter" {
            let mark = rule.mark.as_deref().unwrap_or_default();
            let is_string = metamodel
                .document
                .attributes
                .get(mark)
                .is_some_and(|a| a.type_name == "string");
            if !is_string {
                diagnostics.push(DeslDiagnostic::error(
                    json::pointer(&pointer, "mark"),
                    format!("The counter of '{name}' keeps its mark in '{mark}', which is no string attribute of the document (DESL §5.3.5)."),
                ));
            }
        }
    }
    let type_map: IndexMap<String, String> = json::members(persistence, "typeMap")
        .map(|(name, entry)| {
            (
                name.clone(),
                json::string_of(entry, "as").unwrap_or_default().to_owned(),
            )
        })
        .collect();
    for (name, target) in &type_map {
        if !matches!(target.as_str(), "document" | "header" | "unreadable") && !metamodel.types.contains_key(target) {
            diagnostics.push(DeslDiagnostic::error(
                json::pointer(&json::pointer("/persistence/typeMap", name), "as"),
                format!("The binding type '{name}' becomes '{target}', which is no element type (DESL §5.2)."),
            ));
        }
    }
    DeslPersistence {
        format: json::string_of(persistence, "format").unwrap_or_default().to_owned(),
        bindings: json::strings_of(persistence, "bindings"),
        missing: json::string_of(ids, "missing").unwrap_or("assign").to_owned(),
        encoding: json::string_of(ids, "encoding").unwrap_or("hex").to_owned(),
        ids: top,
        id_rules,
        type_map,
    }
}

/// Reads the surface and resolves the names it uses (DESL section 10.1, step 4).
pub(crate) fn surface(
    root: &Json,
    metamodel: &DeslMetamodel,
    diagnostics: &mut Vec<DeslDiagnostic>,
) -> Option<DeslSurface> {
    let surface = json::member(root, "surface");
    match json::string_of(surface, "kind") {
        Some("table") => table(surface, metamodel, diagnostics).map(|t| DeslSurface::Table(Box::new(t))),
        Some("hierarchy") => {
            hierarchy(surface, metamodel, diagnostics);
            Some(DeslSurface::Hierarchy(surface.clone()))
        }
        other => {
            diagnostics.push(DeslDiagnostic::error(
                "/surface/kind",
                format!(
                    "The surface's kind is '{}'; this runtime lays out 'table' and 'hierarchy' (DESL §7.1).",
                    other.unwrap_or_default()
                ),
            ));
            None
        }
    }
}

/// Checks names against the metamodel, each failure a diagnostic at its pointer.
struct Names<'a> {
    metamodel: &'a DeslMetamodel,
    diagnostics: &'a mut Vec<DeslDiagnostic>,
}

impl Names<'_> {
    /// The type named at `pointer`, when it is an element type.
    fn type_at(&mut self, value: &Json, pointer: &str) -> Option<String> {
        let name = value.as_str().unwrap_or_default();
        if self.metamodel.types.contains_key(name) {
            return Some(name.to_owned());
        }
        self.diagnostics.push(DeslDiagnostic::error(
            pointer,
            format!("The surface names '{name}', which is no element type (DESL §10.1)."),
        ));
        None
    }

    /// The attribute named at `pointer`, when `type_name` has it.
    fn attribute_at(&mut self, type_name: &str, value: &Json, pointer: &str) -> String {
        let name = value.as_str().unwrap_or_default().to_owned();
        let has = self
            .metamodel
            .types
            .get(type_name)
            .is_some_and(|t| t.attributes.contains_key(&name));
        if !has {
            self.diagnostics.push(DeslDiagnostic::error(
                pointer,
                format!("The surface names the attribute '{name}', which '{type_name}' does not have (DESL §10.1)."),
            ));
        }
        name
    }

    fn contains(&self, parent: &str, child: &str) -> bool {
        self.metamodel
            .types
            .get(parent)
            .and_then(|t| t.children.as_ref())
            .is_some_and(|c| c.all_allowed().any(|a| a == child))
    }
}

fn table(surface: &Json, metamodel: &DeslMetamodel, diagnostics: &mut Vec<DeslDiagnostic>) -> Option<DeslTable> {
    let mut names = Names { metamodel, diagnostics };
    let columns = json::member(surface, "columns");
    let cells = json::member(surface, "cells");
    let views = json::member(surface, "views");
    let columns_type = names.type_at(json::member(columns, "type"), "/surface/columns/type");
    let rows_type = names.type_at(
        json::member(json::member(surface, "rows"), "type"),
        "/surface/rows/type",
    );
    let cells_type = names.type_at(json::member(cells, "type"), "/surface/cells/type");
    let views_type = names.type_at(json::member(views, "type"), "/surface/views/type");

    fn optional(
        names: &mut Names<'_>,
        type_name: &Option<String>,
        owner: &Json,
        key: &str,
        pointer: &str,
    ) -> Option<String> {
        let value = json::get(owner, key)?;
        Some(match type_name {
            Some(t) => names.attribute_at(t, value, pointer),
            None => value.as_str().unwrap_or_default().to_owned(),
        })
    }
    fn required(names: &mut Names<'_>, type_name: &Option<String>, owner: &Json, key: &str, pointer: &str) -> String {
        match type_name {
            Some(t) => names.attribute_at(t, json::member(owner, key), pointer),
            None => json::string_of(owner, key).unwrap_or_default().to_owned(),
        }
    }

    let column_name = required(&mut names, &columns_type, columns, "name", "/surface/columns/name");
    let column_value_type = required(
        &mut names,
        &columns_type,
        columns,
        "valueType",
        "/surface/columns/valueType",
    );
    let column_title = optional(&mut names, &columns_type, columns, "title", "/surface/columns/title");
    let column_parent = optional(&mut names, &columns_type, columns, "parent", "/surface/columns/parent");
    let options = json::get(columns, "options").map(|o| {
        let type_name = names.type_at(json::member(o, "type"), "/surface/columns/options/type");
        if let (Some(t), Some(c)) = (&type_name, &columns_type)
            && !names.contains(c, t)
        {
            names.diagnostics.push(DeslDiagnostic::error(
                "/surface/columns/options/type",
                format!("The options' type '{t}' is not among the children of the columns' type '{c}' (DESL §10.1)."),
            ));
        }
        DeslOptions {
            name: required(&mut names, &type_name, o, "name", "/surface/columns/options/name"),
            colour: optional(&mut names, &type_name, o, "colour", "/surface/columns/options/colour"),
            type_name: type_name.unwrap_or_default(),
        }
    });
    let cell_column = required(&mut names, &cells_type, cells, "column", "/surface/cells/column");
    let cell_items = json::get(cells, "items").and_then(|i| names.type_at(i, "/surface/cells/items"));
    if let (Some(items), Some(c)) = (&cell_items, &cells_type)
        && !names.contains(c, items)
    {
        names.diagnostics.push(DeslDiagnostic::error(
            "/surface/cells/items",
            format!("The items' type '{items}' is not among the children of the cells' type '{c}' (DESL §10.1)."),
        ));
    }
    let view_name = required(&mut names, &views_type, views, "name", "/surface/views/name");
    let view_active = json::string_of(views, "active").map(str::to_owned);
    if let Some(active) = &view_active
        && !metamodel.document.attributes.contains_key(active)
    {
        names.diagnostics.push(DeslDiagnostic::error(
            "/surface/views/active",
            format!("The active view is kept in '{active}', which is no attribute of the document (DESL §7.4)."),
        ));
    }
    let mut view_settings = IndexMap::new();
    for (role, value) in json::members(views, "settings") {
        if let Some(t) = names.type_at(value, &json::pointer("/surface/views/settings", role)) {
            view_settings.insert(role.clone(), t);
        }
    }

    let mut value_types = IndexMap::new();
    for (name, value) in json::members(surface, "valueTypes") {
        let pointer = json::pointer("/surface/valueTypes", name);
        let many = json::bool_of(value, "many");
        let key = json::string_of(value, "key").unwrap_or_default().to_owned();
        // The key is an attribute of the cells' type, or of the items' type with `many` (DESL section 10.1, step 4).
        let holder = if many { &cell_items } else { &cells_type };
        if let Some(holder) = holder {
            names.attribute_at(holder, &Json::String(key.clone()), &json::pointer(&pointer, "key"));
        } else if many {
            names.diagnostics.push(DeslDiagnostic::error(
                json::pointer(&pointer, "many"),
                format!("The value type '{name}' holds several values, and the cells have no items' type (DESL §7.3)."),
            ));
        }
        let editor = json::string_of(value, "editor").and_then(Editor::parse);
        let sort = json::string_of(value, "sort").and_then(SortKind::parse);
        let comparisons = json::strings_of(value, "comparisons");
        for listed in comparisons
            .iter()
            .filter(|c| matches!(c.as_str(), "is-empty" | "is-not-empty"))
        {
            names.diagnostics.push(DeslDiagnostic::error(
                json::pointer(&pointer, "comparisons"),
                format!("'{listed}' is offered for every value type and is not listed (DESL §7.5)."),
            ));
        }
        let converts: IndexMap<String, String> = json::members(value, "converts")
            .map(|(to, how)| (to.clone(), how.as_str().unwrap_or_default().to_owned()))
            .collect();
        match (editor, sort) {
            (Some(editor), Some(sort)) => {
                value_types.insert(
                    name.clone(),
                    DeslValueType {
                        label: localized(json::get(value, "label")),
                        key,
                        many,
                        editor,
                        comparisons,
                        sort,
                        converts,
                    },
                );
            }
            _ => names.diagnostics.push(DeslDiagnostic::error(
                pointer,
                format!("The value type '{name}' has no editor or no sort this runtime knows (DESL §7.5)."),
            )),
        }
    }
    for (name, value) in &value_types {
        for to in value.converts.keys() {
            if !value_types.contains_key(to) {
                names.diagnostics.push(DeslDiagnostic::error(
                    json::pointer(&json::pointer("/surface/valueTypes", name), "converts"),
                    format!("'{name}' converts to '{to}', which is no value type (DESL §7.6)."),
                ));
            }
        }
    }

    Some(DeslTable {
        columns_type: columns_type?,
        column_name,
        column_value_type,
        column_title,
        column_parent,
        options,
        rows_type: rows_type?,
        cells_type: cells_type?,
        cell_column,
        cell_items,
        views_type: views_type?,
        view_name,
        view_active,
        view_settings,
        value_types,
    })
}

/// The names a `hierarchy` surface uses (DESL section 10.1, step 4, DESL 0.2).
fn hierarchy(surface: &Json, metamodel: &DeslMetamodel, diagnostics: &mut Vec<DeslDiagnostic>) {
    let mut names = Names { metamodel, diagnostics };
    fn role(surface: &Json, names: &mut Names<'_>, role: &str, attributes: &[&str]) -> Option<String> {
        let value = json::get(surface, role)?;
        let pointer = json::pointer("/surface", role);
        let type_name = names.type_at(json::member(value, "type"), &json::pointer(&pointer, "type"))?;
        for attribute in attributes {
            if let Some(named) = json::get(value, attribute) {
                names.attribute_at(&type_name, named, &json::pointer(&pointer, attribute));
            }
        }
        Some(type_name)
    }
    let levels = role(surface, &mut names, "levels", &["title", "description"]);
    let entries = role(surface, &mut names, "entries", &["title", "description"]);
    role(
        surface,
        &mut names,
        "notes",
        &["about", "kind", "text", "detail", "resolved"],
    );
    role(surface, &mut names, "filters", &["name", "query"]);
    if let (Some(levels), Some(entries)) = (&levels, &entries) {
        for child in [levels, entries] {
            if !names.contains(levels, child) {
                names.diagnostics.push(DeslDiagnostic::error(
                    "/surface/levels/type",
                    format!("The levels' type '{levels}' does not contain '{child}' (DESL §10.1)."),
                ));
            }
        }
        if let Some(collapsed) = json::get(surface, "collapsed") {
            for owner in [levels, entries] {
                names.attribute_at(owner, collapsed, "/surface/collapsed");
            }
        }
    }
    for (label, value) in json::members(surface, "labels") {
        let pointer = json::pointer("/surface/labels", label);
        for owner in json::strings_of(value, "on") {
            if let Some(owner) = names.type_at(&Json::String(owner), &json::pointer(&pointer, "on")) {
                names.attribute_at(&owner, &Json::String(label.clone()), &pointer);
            }
        }
        if let Some(options) = json::get(value, "options") {
            let options_pointer = json::pointer(&pointer, "options");
            if let Some(t) = names.type_at(json::member(options, "type"), &json::pointer(&options_pointer, "type")) {
                for attribute in ["name", "colour"] {
                    if let Some(named) = json::get(options, attribute) {
                        names.attribute_at(&t, named, &json::pointer(&options_pointer, attribute));
                    }
                }
            }
        }
    }
}

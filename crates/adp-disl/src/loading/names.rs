//! Step 5 of loading (DISL section 14.1; the standalone host's `DislNames`) outside the metamodel: the type,
//! operation and form references of the toolbox, the forms, the constraints, the behavior, the notation, the
//! viewpoints and the id rules each name something the specification declares (section 2.7). The metamodel's own
//! references are resolved while its inheritance is flattened.

use std::collections::HashSet;

use super::diagnostic::DislDiagnostic;
use super::metamodel::DislMetamodel;
use crate::json::{self, Json};

/// The words a context menu's `for`, a constraint's `scope` or an operation's `for` may use besides type names.
const TARGET_WORDS: [&str; 6] = ["diagram", "connection", "node", "relation", "port", "*"];

struct Resolver<'a> {
    metamodel: &'a DislMetamodel,
    diagnostics: &'a mut Vec<DislDiagnostic>,
}

impl Resolver<'_> {
    /// A type name, where any node or relation type will do.
    fn name(&mut self, name: &str, pointer: &str) {
        if self.metamodel.type_of(name).is_none() {
            self.diagnostics.push(DislDiagnostic::error(
                pointer,
                format!("'{name}' is not a type of this specification (DISL §2.7)."),
            ));
        }
    }

    /// A property naming one type: a concrete one where something is created, a relation type where one is connected.
    fn type_ref(&mut self, owner: &Json, property: &str, pointer: &str, concrete: bool, relation: bool) {
        let Some(name) = json::string_of(owner, property) else {
            return;
        };
        let at = json::pointer(pointer, property);
        match self.metamodel.type_of(name) {
            None => self.diagnostics.push(DislDiagnostic::error(
                at,
                format!("'{name}' is not a type of this specification (DISL §2.7)."),
            )),
            Some(t) if concrete && t.is_abstract() => self.diagnostics.push(DislDiagnostic::error(
                at,
                format!("'{name}' is abstract, so nothing can create one (DISL §4.6)."),
            )),
            Some(t) if relation && !t.is_relation => self.diagnostics.push(DislDiagnostic::error(
                at,
                format!("'{name}' is not a relation type (DISL §7.3)."),
            )),
            _ => {}
        }
    }

    /// A property naming what something applies to: types, or one of the words `diagram`, `connection` and the like.
    fn targets(&mut self, owner: &Json, property: &str, pointer: &str) {
        for name in json::strings_of(owner, property) {
            if !TARGET_WORDS.contains(&name.as_str()) {
                self.name(&name, &json::pointer(pointer, property));
            }
        }
    }
}

fn each(owner: &Json, name: &str, pointer: &str, mut visit: impl FnMut(&Json, &str)) {
    let Some(list) = json::items(owner, name) else {
        return;
    };
    for (index, item) in list.iter().enumerate() {
        if item.is_object() {
            visit(item, &json::pointer_at(pointer, index));
        }
    }
}

pub(crate) fn resolve(root: &Json, metamodel: &DislMetamodel, diagnostics: &mut Vec<DislDiagnostic>) {
    let behavior = json::member(root, "behavior");
    let operations: HashSet<String> = json::members(behavior, "operations").map(|(n, _)| n.clone()).collect();
    let forms: HashSet<String> = json::members(root, "forms").map(|(n, _)| n.clone()).collect();
    let mut r = Resolver { metamodel, diagnostics };

    if let Some(toolbox) = json::get(root, "toolbox") {
        each(toolbox, "groups", "/toolbox/groups", |group, group_pointer| {
            each(
                group,
                "tools",
                &json::pointer(group_pointer, "tools"),
                |tool, tool_pointer| {
                    r.type_ref(tool, "creates", tool_pointer, true, false);
                },
            );
        });
        each(
            toolbox,
            "contextMenus",
            "/toolbox/contextMenus",
            |menu, menu_pointer| {
                r.targets(menu, "for", menu_pointer);
                each(
                    menu,
                    "tools",
                    &json::pointer(menu_pointer, "tools"),
                    |tool, tool_pointer| {
                        if let Some(operation) = json::string_of(tool, "operation")
                            && !operations.contains(operation)
                        {
                            r.diagnostics.push(DislDiagnostic::error(
                                json::pointer(tool_pointer, "operation"),
                                format!("'{operation}' is not an operation of behavior.operations (DISL §7.3)."),
                            ));
                        }
                        r.type_ref(tool, "via", tool_pointer, false, true);
                        if let Some(form) = json::string_of(tool, "form")
                            && !forms.contains(form)
                        {
                            r.diagnostics.push(DislDiagnostic::error(
                                json::pointer(tool_pointer, "form"),
                                format!("'{form}' is not a form of this specification (DISL §7.3)."),
                            ));
                        }
                    },
                );
            },
        );
    }

    for (name, form) in json::members(root, "forms") {
        r.targets(form, "for", &json::pointer("/forms", name));
    }

    if let Some(constraints) = json::get(root, "constraints") {
        each(constraints, "rules", "/constraints/rules", |rule, pointer| {
            r.targets(rule, "scope", pointer);
        });
    }

    for (name, operation) in json::members(behavior, "operations") {
        let pointer = json::pointer("/behavior/operations", name);
        r.targets(operation, "for", &pointer);
        if let Some(form) = json::string_of(operation, "paramsForm")
            && !forms.contains(form)
        {
            r.diagnostics.push(DislDiagnostic::error(
                json::pointer(&pointer, "paramsForm"),
                format!("'{form}' is not a form of this specification (DISL §9.3)."),
            ));
        }
    }
    each(behavior, "hooks", "/behavior/hooks", |hook, pointer| {
        r.targets(hook, "for", pointer)
    });
    for (name, _) in json::members(behavior, "deletion") {
        r.name(name, &json::pointer("/behavior/deletion", name));
    }
    for (name, retype) in json::members(behavior, "retype") {
        let pointer = json::pointer("/behavior/retype", name);
        r.name(name, &pointer);
        for target in json::strings_of(retype, "to") {
            r.name(&target, &json::pointer(&pointer, "to"));
        }
    }

    if let Some(notation) = json::get(root, "notation") {
        notation_names(notation, "/notation", &mut r);
    }
    for (name, viewpoint) in json::members(root, "viewpoints") {
        let pointer = json::pointer("/viewpoints", name);
        for key in ["include", "exclude"] {
            for n in json::strings_of(viewpoint, key) {
                if !n.contains('*') {
                    r.name(&n, &json::pointer(&pointer, key));
                }
            }
        }
        if let Some(overrides) = json::get(viewpoint, "notation") {
            notation_names(overrides, &json::pointer(&pointer, "notation"), &mut r);
        }
    }

    if let Some(ids) = json::get(root, "persistence").and_then(|p| json::get(p, "ids")) {
        for (name, _) in json::members(ids, "types") {
            r.name(name, &json::pointer("/persistence/ids/types", name));
        }
    }
}

fn notation_names(notation: &Json, pointer: &str, r: &mut Resolver<'_>) {
    for (name, _) in json::members(notation, "nodes") {
        r.name(name, &json::pointer(&json::pointer(pointer, "nodes"), name));
    }
    // An edge notation names a relation type, or a reference attribute as Type.attribute (section 6.10).
    for (name, _) in json::members(notation, "edges") {
        if !name.contains('.') {
            r.name(name, &json::pointer(&json::pointer(pointer, "edges"), name));
        }
    }
}

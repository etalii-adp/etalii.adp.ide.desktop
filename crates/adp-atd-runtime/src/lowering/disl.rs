//! A diagram's description as a DISL specification (ATD §7, §8; research R16): each construct as the DISL construct
//! it carries, its settings as DISL's properties (ATD §6.6). Operations and hooks whose statements are all DISL's
//! actions are lowered to them; any other is one `extension` action, which the script runs (see `script.rs`). The
//! description's functions are the script's, so the specification has no `functions` of its own.

use serde_json::{Map, Value as Json, json};

use super::actions::{self, Parameters, Written};
use super::values::{expression, names, settings, settings_of, text, type_name, value};

/// The DISL version the lowered specification states.
const DISL: &str = "0.3";

/// What lowering a diagram's description gives: the specification, and the statements each `extension` action runs,
/// by its name.
pub struct LoweredDiagram {
    pub specification: Json,
    pub scripts: Map<String, Json>,
}

/// Lowers the description whose tool construct is `tool` and whose members, from every file, are `members`.
pub fn lower(tool: &Json, members: &[&Json]) -> LoweredDiagram {
    let parameters: Parameters = members
        .iter()
        .filter(|m| m["kind"] == "operation")
        .map(|m| {
            let params = m["params"].as_array().into_iter().flatten();
            (text(&m["name"]), params.map(|p| text(&p["name"])).collect())
        })
        .collect();
    let mut lowering = Lowering {
        root: Map::new(),
        scripts: Map::new(),
        parameters,
    };
    lowering.root.insert(
        "$schema".into(),
        json!("https://etalii.net/adp/disl/schema/0.2/disl.schema.json#/$defs/Specification"),
    );
    lowering.root.insert("disl".into(), json!(DISL));
    lowering.language(tool, members);
    for member in members {
        lowering.member(member);
    }
    LoweredDiagram {
        specification: Json::Object(lowering.root),
        scripts: lowering.scripts,
    }
}

struct Lowering {
    root: Map<String, Json>,
    scripts: Map<String, Json>,
    parameters: Parameters,
}

/// The object at `path` under `object`, made when it is not there.
fn at<'a>(object: &'a mut Map<String, Json>, path: &[&str]) -> &'a mut Map<String, Json> {
    let mut current = object;
    for key in path {
        let entry = current.entry((*key).to_owned()).or_insert_with(|| json!({}));
        if !entry.is_object() {
            *entry = json!({});
        }
        current = entry.as_object_mut().expect("an object");
    }
    current
}

/// The list at `key` under `object`, made when it is not there.
fn list<'a>(object: &'a mut Map<String, Json>, key: &str) -> &'a mut Vec<Json> {
    let entry = object.entry(key.to_owned()).or_insert_with(|| json!([]));
    if !entry.is_array() {
        *entry = json!([]);
    }
    entry.as_array_mut().expect("a list")
}

/// A `for` list as DISL writes it: one type as its name, several as a list.
fn for_types(node: &Json) -> Json {
    let types = names(node);
    match &types[..] {
        [one] if one == "document" => json!("diagram"),
        [one] => json!(one),
        _ => json!(types),
    }
}

impl Lowering {
    /// The tool construct: DISL's `language`, with the `x-` settings at the root (DISL §2.8).
    fn language(&mut self, tool: &Json, members: &[&Json]) {
        let mut language = Map::new();
        for key in ["origin", "version"] {
            if let Some(v) = tool.get(key) {
                language.insert(key.into(), v.clone());
            }
        }
        for member in members {
            match member["kind"].as_str() {
                Some("label") => {
                    language.insert("label".into(), value(&member["value"]));
                }
                Some("doc") => {
                    language.insert("doc".into(), value(&member["value"]));
                }
                Some("setting") => {
                    let key = member["setting"]["key"].as_str().unwrap_or_default();
                    let lowered = value(&member["setting"]["value"]);
                    if key.starts_with("x-") {
                        self.root.insert(key.to_owned(), lowered);
                    } else {
                        language.insert(key.to_owned(), lowered);
                    }
                }
                _ => {}
            }
        }
        // DISL's order: the id first.
        let mut ordered = Map::new();
        if let Some(id) = language.remove("id") {
            ordered.insert("id".into(), id);
        }
        ordered.extend(language);
        self.root.insert("language".into(), Json::Object(ordered));
    }

    fn member(&mut self, member: &Json) {
        match member["kind"].as_str().unwrap_or_default() {
            "model" => self.model(member),
            "constraints" => self.constraints(member),
            "constraint" => self.constraint(member),
            "operation" => self.operation(member),
            "hook" => self.hook(member),
            "reason" => self.reason(member),
            "message" => {
                at(&mut self.root, &["behavior", "messages"]).insert(text(&member["name"]), value(&member["value"]));
            }
            "behavior" => at(&mut self.root, &["behavior"]).extend(settings_of(member)),
            "persistence" => at(&mut self.root, &["persistence"]).extend(settings_of(member)),
            "plugin" => {
                at(&mut self.root, &["plugins"]).insert(text(&member["name"]), Json::Object(settings_of(member)));
            }
            "coordinates" => self.coordinates(member),
            "notation" => self.notation(member),
            "toolbox" => self.toolbox(member),
            "form" => self.form(member),
            "layout" => self.layout(member),
            "viewpoint" => {
                let mut viewpoint = settings_of(member);
                if let Some(title) = member.get("title") {
                    viewpoint.insert("label".into(), title.clone());
                }
                at(&mut self.root, &["viewpoints"]).insert(text(&member["name"]), Json::Object(viewpoint));
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------------------------------------ The model (ATD §7.4)

    fn model(&mut self, model: &Json) {
        let metamodel = at(&mut self.root, &["metamodel"]);
        for item in model["members"].as_array().into_iter().flatten() {
            match item["kind"].as_str().unwrap_or_default() {
                "element" => {
                    let lowered = element(item);
                    at(metamodel, &["types"]).insert(text(&item["name"]), Json::Object(lowered));
                }
                "relation" => {
                    let mut lowered = Map::new();
                    let (from, to) = (names(&item["from"]), names(&item["to"]));
                    let end = |types: Vec<String>| {
                        if types.len() == 1 {
                            json!(types[0])
                        } else {
                            json!(types)
                        }
                    };
                    if !from.is_empty() {
                        lowered.insert("source".into(), end(from));
                    }
                    if !to.is_empty() {
                        lowered.insert("target".into(), end(to));
                    }
                    let mut rest = element(item);
                    // A label first, as DISL writes it.
                    let mut ordered = Map::new();
                    if let Some(label) = rest.remove("label") {
                        ordered.insert("label".into(), label);
                    }
                    ordered.extend(lowered);
                    ordered.extend(rest);
                    at(metamodel, &["relations"]).insert(text(&item["name"]), Json::Object(ordered));
                }
                "enum" => {
                    let mut values = Map::new();
                    for entry in item["values"].as_array().into_iter().flatten() {
                        let name = entry["name"]
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| text(&entry["name"]));
                        values.insert(name, Json::Object(settings_of(entry)));
                    }
                    let mut lowered = settings_of(item);
                    if let Some(title) = item.get("title") {
                        lowered.insert("label".into(), title.clone());
                    }
                    lowered.insert("values".into(), Json::Object(values));
                    at(metamodel, &["enums"]).insert(text(&item["name"]), Json::Object(lowered));
                }
                "datatype" => {
                    let mut lowered = settings_of(item);
                    if let Some(base) = item.get("base") {
                        lowered.insert("base".into(), json!(type_name(base)));
                    }
                    at(metamodel, &["datatypes"]).insert(text(&item["name"]), Json::Object(lowered));
                }
                "document" => {
                    let mut lowered = element(item);
                    lowered.remove("label");
                    at(metamodel, &["diagram"]).extend(lowered);
                }
                "setting" => {
                    let key = item["setting"]["key"].as_str().unwrap_or_default().to_owned();
                    metamodel.insert(key, value(&item["setting"]["value"]));
                }
                _ => {}
            }
        }
    }

    // ---------------------------------------------------------------------------- Constraints (ATD §7.9)

    fn constraints(&mut self, constraints: &Json) {
        for member in constraints["members"].as_array().into_iter().flatten() {
            match member["kind"].as_str().unwrap_or_default() {
                "setting" => {
                    let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                    at(&mut self.root, &["constraints"]).insert(key, value(&member["setting"]["value"]));
                }
                "builtin" => {
                    let lowered = settings(member["settings"].as_array().map(Vec::as_slice).unwrap_or_default());
                    at(&mut self.root, &["constraints", "builtIn"])
                        .insert(text(&member["name"]), Json::Object(lowered));
                }
                "constraint" => self.constraint(member),
                _ => {}
            }
        }
    }

    fn constraint(&mut self, constraint: &Json) {
        let mut rule = Map::new();
        rule.insert("id".into(), json!(text(&constraint["name"])));
        let mut own = settings_of(constraint);
        for key in ["code", "kind"] {
            if let Some(v) = own.remove(key) {
                rule.insert(key.into(), v);
            }
        }
        if let Some(label) = constraint.get("label") {
            rule.insert("label".into(), value(label));
        }
        if !constraint["for"].is_null() {
            rule.insert("scope".into(), json!(names(&constraint["for"])));
        }
        for key in ["when", "rule"] {
            if let Some(node) = constraint.get(key) {
                rule.insert(key.into(), expression(node));
            }
        }
        if let Some(severity) = constraint.get("severity") {
            rule.insert("severity".into(), json!(text(severity)));
        }
        rule.extend(own);
        if let Some(message) = constraint.get("message") {
            rule.insert("message".into(), value(message));
        }
        let fixes: Vec<Json> = constraint["fixes"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|fix| {
                let mut lowered = settings_of(fix);
                lowered.insert("id".into(), json!(text(&fix["name"])));
                if let Some(title) = fix.get("title") {
                    lowered.insert("label".into(), title.clone());
                }
                let name = format!("fix:{}:{}", text(&constraint["name"]), text(&fix["name"]));
                lowered.insert("actions".into(), Json::Array(self.statements(&name, &fix["body"])));
                Json::Object(lowered)
            })
            .collect();
        if !fixes.is_empty() {
            rule.insert("fixes".into(), Json::Array(fixes));
        }
        list(at(&mut self.root, &["constraints"]), "rules").push(Json::Object(rule));
    }

    // --------------------------------------------------------------------- Operations and hooks (ATD §7.10, §7.11)

    /// The actions of a body: DISL's own when every statement is one, else one `extension` action that runs the
    /// script, named `name`.
    fn statements(&mut self, name: &str, body: &Json) -> Vec<Json> {
        let statements = body.as_array().map(Vec::as_slice).unwrap_or_default();
        match actions::block(statements, &self.parameters, &mut Written) {
            Some(actions) => actions,
            None => {
                self.scripts.insert(name.to_owned(), body.clone());
                vec![json!({ "extension": name })]
            }
        }
    }

    fn operation(&mut self, operation: &Json) {
        let name = text(&operation["name"]);
        let mut lowered = Map::new();
        match (operation.get("label"), operation.get("title")) {
            (Some(label), _) => {
                lowered.insert("label".into(), value(label));
            }
            (None, Some(title)) => {
                lowered.insert("label".into(), title.clone());
            }
            _ => {}
        }
        if !operation["for"].is_null() {
            let types = for_types(&operation["for"]);
            lowered.insert(
                "for".into(),
                if types.is_string() && types != "diagram" {
                    json!([types])
                } else {
                    types
                },
            );
        }
        let mut own = settings_of(operation);
        for key in ["icon", "shortcut"] {
            if let Some(v) = own.remove(key) {
                lowered.insert(key.into(), v);
            }
        }
        if let Some(enabled) = operation.get("enabled") {
            lowered.insert("enabled".into(), expression(enabled));
        }
        if let Some(unavailable) = operation.get("unavailable") {
            lowered.insert("unavailable".into(), value(unavailable));
        }
        if let Some(confirm) = operation["confirm"].as_array() {
            lowered.insert("confirm".into(), Json::Object(settings(confirm)));
        }
        let mut params = Map::new();
        for param in operation["params"].as_array().into_iter().flatten() {
            let mut lowered_param = Map::new();
            lowered_param.insert("type".into(), json!(type_name(&param["type"])));
            lowered_param.extend(settings_of(param));
            if let Some(default) = param.get("default") {
                lowered_param.insert("default".into(), value(default));
            }
            params.insert(text(&param["name"]), Json::Object(lowered_param));
        }
        if !params.is_empty() {
            lowered.insert("params".into(), Json::Object(params));
        }
        let body = &operation["body"];
        if body.as_array().is_some_and(|b| !b.is_empty()) {
            let actions = self.statements(&format!("operation:{name}"), body);
            lowered.insert("actions".into(), Json::Array(actions));
        }
        lowered.extend(own);
        at(&mut self.root, &["behavior", "operations"]).insert(name, Json::Object(lowered));
    }

    fn hook(&mut self, hook: &Json) {
        let name = text(&hook["name"]);
        let mut lowered = Map::new();
        lowered.insert("id".into(), json!(name));
        lowered.insert("on".into(), json!(text(&hook["on"])));
        if !hook["for"].is_null() {
            lowered.insert("for".into(), for_types(&hook["for"]));
        }
        if let Some(when) = hook.get("when") {
            lowered.insert("when".into(), expression(when));
        }
        lowered.extend(settings_of(hook));
        let actions = self.statements(&format!("hook:{name}"), &hook["body"]);
        lowered.insert("actions".into(), Json::Array(actions));
        list(at(&mut self.root, &["behavior"]), "hooks").push(Json::Object(lowered));
    }

    fn reason(&mut self, reason: &Json) {
        let mut lowered = settings_of(reason);
        if let Some(when) = reason.get("when") {
            lowered.insert("when".into(), expression(when));
        }
        if let Some(message) = reason.get("message") {
            lowered.insert("message".into(), value(message));
        }
        at(&mut self.root, &["behavior", "reasons"]).insert(text(&reason["name"]), Json::Object(lowered));
    }

    // ----------------------------------------------------------------------- The canvas surface (ATD §8)

    fn coordinates(&mut self, coordinates: &Json) {
        let lowered = at(&mut self.root, &["coordinates"]);
        for member in coordinates["members"].as_array().into_iter().flatten() {
            let kind = member["kind"].as_str().unwrap_or_default();
            match kind {
                "system" | "axis" | "snapping" => {
                    let section = match kind {
                        "system" => "systems",
                        "axis" => "axes",
                        _ => "snapping",
                    };
                    at(lowered, &[section]).insert(text(&member["name"]), Json::Object(settings_of(member)));
                }
                "default" => {
                    lowered.insert("default".into(), json!(text(&member["name"])));
                }
                "setting" => {
                    let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                    lowered.insert(key, value(&member["setting"]["value"]));
                }
                _ => {}
            }
        }
    }

    fn notation(&mut self, notation: &Json) {
        let lowered = at(&mut self.root, &["notation"]);
        for member in notation["members"].as_array().into_iter().flatten() {
            let kind = member["kind"].as_str().unwrap_or_default();
            match kind {
                "theme" => {
                    lowered.insert("theme".into(), Json::Object(settings_of(member)));
                }
                "style" | "shape" | "marker" | "icon" => {
                    let section = match kind {
                        "style" => "styles",
                        "shape" => "shapes",
                        "marker" => "markers",
                        _ => "icons",
                    };
                    let mut definition = settings_of(member);
                    if !member["extends"].is_null() {
                        definition.insert("extends".into(), json!(names(&member["extends"])));
                    }
                    at(lowered, &[section]).insert(text(&member["name"]), Json::Object(definition));
                }
                "canvas" | "defaults" => {
                    lowered.insert(kind.to_owned(), Json::Object(settings_of(member)));
                }
                "node" | "edge" => {
                    let section = if kind == "node" { "nodes" } else { "edges" };
                    let definition = presentation(member);
                    for name in names(&member["names"]) {
                        at(lowered, &[section]).insert(name, Json::Object(definition.clone()));
                    }
                }
                "setting" => {
                    let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                    lowered.insert(key, value(&member["setting"]["value"]));
                }
                _ => {}
            }
        }
    }

    fn toolbox(&mut self, toolbox: &Json) {
        let mut groups = Vec::new();
        let mut menus = Vec::new();
        let mut own = Map::new();
        for member in toolbox["members"].as_array().into_iter().flatten() {
            match member["kind"].as_str().unwrap_or_default() {
                "group" => groups.push(group(member)),
                "tool" => groups.push(tool(member)),
                "context" => {
                    let mut menu = Map::new();
                    if !member["names"].is_null() {
                        menu.insert("for".into(), json!(names(&member["names"])));
                    }
                    if let Some(when) = member.get("when") {
                        menu.insert("when".into(), expression(when));
                    }
                    menu.extend(settings_of(member));
                    let tools: Vec<Json> = member["members"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|m| m["kind"] == "tool")
                        .map(tool)
                        .collect();
                    menu.insert("tools".into(), Json::Array(tools));
                    menus.push(Json::Object(menu));
                }
                "template" => {
                    let mut template = settings_of(member);
                    template.insert("id".into(), json!(text(&member["name"])));
                    list(&mut own, "templates").push(Json::Object(template));
                }
                "setting" => {
                    let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                    own.insert(key, value(&member["setting"]["value"]));
                }
                _ => {}
            }
        }
        let lowered = at(&mut self.root, &["toolbox"]);
        lowered.extend(own);
        if !groups.is_empty() {
            lowered.insert("groups".into(), Json::Array(groups));
        }
        if !menus.is_empty() {
            lowered.insert("contextMenus".into(), Json::Array(menus));
        }
    }

    fn form(&mut self, form: &Json) {
        let mut lowered = Map::new();
        if !form["for"].is_null() {
            lowered.insert("for".into(), for_types(&form["for"]));
        }
        let own = settings_of(form);
        lowered.extend(own);
        if let Some(title) = form.get("title") {
            lowered.insert("label".into(), title.clone());
        }
        lowered.insert("items".into(), Json::Array(form_items(form)));
        at(&mut self.root, &["forms"]).insert(text(&form["name"]), Json::Object(lowered));
    }

    fn layout(&mut self, layout: &Json) {
        let lowered = at(&mut self.root, &["layout"]);
        for member in layout["members"].as_array().into_iter().flatten() {
            match member["kind"].as_str().unwrap_or_default() {
                "default" => {
                    lowered.insert("default".into(), json!(text(&member["name"])));
                }
                "algorithm" => {
                    let mut algorithm = settings_of(member);
                    if let Some(capability) = member.get("capability") {
                        // A capability's arguments stay its own; DISL names the algorithm by the capability.
                        let arguments = std::mem::take(&mut algorithm);
                        algorithm.insert("algorithm".into(), json!(format!("capability:{}", text(capability))));
                        algorithm.insert("x-arguments".into(), Json::Object(arguments));
                    }
                    at(lowered, &["algorithms"]).insert(text(&member["name"]), Json::Object(algorithm));
                }
                "setting" => {
                    let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                    lowered.insert(key, value(&member["setting"]["value"]));
                }
                _ => {}
            }
        }
    }
}

/// An element or relation type: its label, attributes, containment and settings (ATD §7.5 to §7.8).
fn element(item: &Json) -> Map<String, Json> {
    let mut lowered = Map::new();
    let mut attributes = Map::new();
    let mut rest = Map::new();
    for member in item["members"].as_array().into_iter().flatten() {
        match member["kind"].as_str().unwrap_or_default() {
            "label" => {
                lowered.insert("label".into(), value(&member["value"]));
            }
            "attribute" => {
                attributes.insert(text(&member["name"]), Json::Object(attribute(member)));
            }
            "contains" => {
                let mut containment = Map::new();
                containment.insert("allowed".into(), json!(names(&member["types"])));
                if member["ordered"] == true {
                    containment.insert("ordered".into(), json!(true));
                }
                containment.extend(settings_of(member));
                match member.get("as") {
                    Some(slot) => {
                        at(&mut rest, &["slots"]).insert(text(slot), Json::Object(containment));
                    }
                    None => {
                        rest.insert("children".into(), Json::Object(containment));
                    }
                }
            }
            "port" => {
                at(&mut rest, &["ports"]).insert(text(&member["name"]), Json::Object(settings_of(member)));
            }
            "setting" => {
                let key = member["setting"]["key"].as_str().unwrap_or_default().to_owned();
                lowered.insert(key, value(&member["setting"]["value"]));
            }
            _ => {}
        }
    }
    if !item["extends"].is_null() {
        lowered.insert("extends".into(), json!(names(&item["extends"])));
    }
    if !attributes.is_empty() {
        lowered.insert("attributes".into(), Json::Object(attributes));
    }
    lowered.extend(rest);
    lowered
}

/// An attribute (ATD §7.6).
fn attribute(member: &Json) -> Map<String, Json> {
    let mut lowered = Map::new();
    lowered.insert("type".into(), json!(type_name(&member["type"])));
    let mut own = settings_of(member);
    if let Some(label) = own.remove("label") {
        lowered.insert("label".into(), label);
    }
    for (flag, key) in [("required", "required"), ("many", "many"), ("readonly", "readOnly")] {
        if member[flag] == true {
            lowered.insert(key.into(), json!(true));
        }
    }
    if let Some(default) = member.get("default") {
        lowered.insert("default".into(), value(default));
    }
    lowered.extend(own);
    lowered
}

/// A node's or an edge's notation (ATD §8.4, §8.5).
fn presentation(member: &Json) -> Map<String, Json> {
    let mut lowered = Map::new();
    let mut labels = Vec::new();
    for item in member["members"].as_array().into_iter().flatten() {
        match item["kind"].as_str().unwrap_or_default() {
            "shape" => {
                let shape = value(&item["value"]);
                let parameters = settings_of(item);
                if parameters.is_empty() {
                    lowered.insert("shape".into(), shape);
                } else {
                    let mut instance = Map::new();
                    instance.insert("shape".into(), shape);
                    instance.insert("params".into(), Json::Object(parameters));
                    lowered.insert("shape".into(), Json::Object(instance));
                }
            }
            "label" => {
                let mut label = Map::new();
                if let Some(id) = item.get("name") {
                    label.insert("id".into(), json!(text(id)));
                }
                label.insert("text".into(), value(&item["value"]));
                label.extend(settings_of(item));
                labels.push(Json::Object(label));
            }
            "state" => {
                at(&mut lowered, &["states"]).insert(text(&item["name"]), Json::Object(settings_of(item)));
            }
            "variant" => {
                let mut variant = presentation(item);
                variant.insert("id".into(), json!(text(&item["name"])));
                if let Some(when) = item.get("when") {
                    variant.insert("when".into(), expression(when));
                }
                list(&mut lowered, "variants").push(Json::Object(variant));
            }
            "line" => {
                lowered.insert("line".into(), Json::Object(settings_of(item)));
            }
            "marker" => {
                let end = item["end"].as_str().unwrap_or("target");
                let marker = value(&item["value"]);
                let parameters = settings_of(item);
                let lowered_marker = if parameters.is_empty() {
                    marker
                } else {
                    let mut instance = Map::new();
                    instance.insert("marker".into(), marker);
                    instance.extend(parameters);
                    Json::Object(instance)
                };
                lowered.insert(format!("{end}Marker"), lowered_marker);
            }
            "compartment" | "badge" | "port" => {
                let section = match item["kind"].as_str() {
                    Some("compartment") => "compartments",
                    Some("badge") => "badges",
                    _ => "ports",
                };
                let mut lowered_item = settings_of(item);
                lowered_item.insert("id".into(), json!(text(&item["name"])));
                list(&mut lowered, section).push(Json::Object(lowered_item));
            }
            "setting" => {
                let key = item["setting"]["key"].as_str().unwrap_or_default().to_owned();
                lowered.insert(key, value(&item["setting"]["value"]));
            }
            _ => {}
        }
    }
    if !labels.is_empty() {
        lowered.insert("labels".into(), Json::Array(labels));
    }
    lowered
}

/// A toolbox group, with its tools and groups (ATD §8.6).
fn group(member: &Json) -> Json {
    let mut lowered = Map::new();
    lowered.insert("id".into(), json!(text(&member["name"])));
    if let Some(title) = member.get("title") {
        lowered.insert("label".into(), title.clone());
    }
    let mut tools = Vec::new();
    let mut groups = Vec::new();
    for item in member["members"].as_array().into_iter().flatten() {
        match item["kind"].as_str().unwrap_or_default() {
            "tool" => tools.push(tool(item)),
            "group" => groups.push(group(item)),
            "setting" => {
                let key = item["setting"]["key"].as_str().unwrap_or_default().to_owned();
                lowered.insert(key, value(&item["setting"]["value"]));
            }
            _ => {}
        }
    }
    if !tools.is_empty() {
        lowered.insert("tools".into(), Json::Array(tools));
    }
    if !groups.is_empty() {
        lowered.insert("groups".into(), Json::Array(groups));
    }
    Json::Object(lowered)
}

/// A tool or a context tool (ATD §8.6).
fn tool(member: &Json) -> Json {
    let mut lowered = Map::new();
    if let Some(name) = member.get("name") {
        lowered.insert("id".into(), json!(text(name)));
    }
    let mut own = settings_of(member);
    for key in ["kind", "operation"] {
        if let Some(v) = own.remove(key) {
            lowered.insert(key.into(), v);
        }
    }
    if let Some(group) = own.remove("group") {
        lowered.insert("group".into(), group);
    }
    if let Some(title) = member.get("title") {
        lowered.insert("label".into(), title.clone());
    }
    if let Some(Json::Object(visible)) = own.get("visible")
        && let Some(cel) = visible.get("cel")
    {
        let cel = cel.clone();
        own.insert("visible".into(), cel);
    }
    lowered.extend(own);
    Json::Object(lowered)
}

/// A form's fields, sections and items (ATD §8.7).
fn form_items(node: &Json) -> Vec<Json> {
    let mut items = Vec::new();
    for member in node["members"].as_array().into_iter().flatten() {
        match member["kind"].as_str().unwrap_or_default() {
            "field" => {
                let mut field = Map::new();
                field.insert("attribute".into(), json!(text(&member["name"])));
                if let Some(title) = member.get("title") {
                    field.insert("label".into(), title.clone());
                }
                field.extend(settings_of(member));
                items.push(Json::Object(field));
            }
            "section" => {
                let mut section = Map::new();
                section.insert("kind".into(), json!("section"));
                if let Some(name) = member.get("name") {
                    section.insert("id".into(), json!(text(name)));
                }
                if let Some(title) = member.get("title") {
                    section.insert("title".into(), title.clone());
                }
                section.extend(settings_of(member));
                section.insert("items".into(), Json::Array(form_items(member)));
                items.push(Json::Object(section));
            }
            "item" => {
                let mut item = Map::new();
                let mut own = settings_of(member);
                if let Some(kind) = own.remove("kind") {
                    item.insert("kind".into(), kind);
                }
                if let Some(name) = member.get("name") {
                    item.insert("id".into(), json!(text(name)));
                }
                if let Some(title) = member.get("title") {
                    item.insert("label".into(), title.clone());
                }
                if let Some(Json::Object(visible)) = own.get("visible")
                    && let Some(cel) = visible.get("cel")
                {
                    let cel = cel.clone();
                    own.insert("visible".into(), cel);
                }
                item.extend(own);
                items.push(Json::Object(item));
            }
            _ => {}
        }
    }
    items
}

//! Reads the subset of the Structurizr DSL that ADP models (etalii.adp spec 030, task T061).
//!
//! Ported from the standalone host's `src/diagrams/c4/backend/EtAlii.Adp.Diagram.C4/C4Parser.cs`, `C4ParseState.cs`
//! and `C4ParseScope.cs` at `8b6ccfc`. Everything it does not model it simply does not read: the document keeps its
//! own text, so an unrecognised construct survives a round trip untouched. Hand-written and deliberately forgiving:
//! what a document means is the rule set's question, and a mid-edit document still opens. `!include` is noted, not
//! followed, and a `/* ... */` comment is skipped within a line and across lines.

use crate::lines::LineDocument;
use crate::model::{
    C4AutoLayout, C4Element, C4ElementKind, C4ElementStyle, C4Interaction, C4Relationship, C4View, C4ViewKind,
    C4Workspace, same,
};
use crate::tokens;

/// Which construct the parser is inside, so a line is read the right way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    Root,
    Workspace,
    Model,
    Element,
    Views,
    View,
    Styles,
    Style,
    Unknown,
}

/// The parser's running state: the scope stack, the element chain, and what has been read.
struct State {
    stack: Vec<(Scope, Option<String>)>,
    generated: usize,
    workspace_name: String,
    elements: Vec<C4Element>,
    relationships: Vec<C4Relationship>,
    views: Vec<C4View>,
    styles: Vec<C4ElementStyle>,
    includes: Vec<String>,
    current_view: Option<usize>,
}

impl State {
    fn new() -> Self {
        State {
            stack: vec![(Scope::Root, None)],
            generated: 0,
            workspace_name: String::new(),
            elements: Vec::new(),
            relationships: Vec::new(),
            views: Vec::new(),
            styles: Vec::new(),
            includes: Vec::new(),
            current_view: None,
        }
    }

    fn current(&self) -> Scope {
        self.stack.last().map_or(Scope::Root, |(scope, _)| *scope)
    }

    /// The innermost element block, which is what a nested declaration belongs to.
    fn current_element(&self) -> Option<String> {
        self.stack.iter().rev().find_map(|(_, id)| id.clone())
    }

    /// Adds tags to an element already read, for a `tags` line inside its block.
    fn add_tags(&mut self, id: &str, tags: Vec<String>) {
        if let Some(element) = self.elements.iter_mut().find(|element| same(&element.id, id)) {
            element.tags.extend(tags);
        }
    }

    fn push(&mut self, scope: Scope) {
        self.stack.push((scope, None));
    }

    fn push_element(&mut self, id: String) {
        self.stack.push((Scope::Element, Some(id)));
    }

    fn pop(&mut self) {
        if self.stack.len() > 1
            && let Some((scope, _)) = self.stack.pop()
            && scope == Scope::View
        {
            self.current_view = None;
        }
    }

    /// An id for an element the document declared without one, which the canvas needs to select and edit it.
    fn generate_id(&mut self, kind: C4ElementKind, name: &str) -> String {
        self.generated += 1;
        let slug: String = name.chars().filter(|c| c.is_alphanumeric()).collect();
        format!(
            "{}_{}_{}",
            kind.lower(),
            if slug.is_empty() { "unnamed" } else { &slug },
            self.generated
        )
    }
}

/// The kind an element keyword declares.
fn element_keyword(keyword: &str) -> Option<C4ElementKind> {
    Some(match keyword.to_lowercase().as_str() {
        "person" => C4ElementKind::Person,
        "softwaresystem" => C4ElementKind::SoftwareSystem,
        "container" => C4ElementKind::Container,
        "component" => C4ElementKind::Component,
        "deploymentnode" => C4ElementKind::DeploymentNode,
        "infrastructurenode" => C4ElementKind::InfrastructureNode,
        "containerinstance" => C4ElementKind::ContainerInstance,
        "softwaresysteminstance" => C4ElementKind::SoftwareSystemInstance,
        _ => return None,
    })
}

/// The kind a view keyword declares.
fn view_keyword(keyword: &str) -> Option<C4ViewKind> {
    Some(match keyword.to_lowercase().as_str() {
        "systemlandscape" => C4ViewKind::SystemLandscape,
        "systemcontext" => C4ViewKind::SystemContext,
        "container" => C4ViewKind::Container,
        "component" => C4ViewKind::Component,
        "dynamic" => C4ViewKind::Dynamic,
        "deployment" => C4ViewKind::Deployment,
        _ => return None,
    })
}

/// The workspace `text` declares.
pub fn parse_text(text: &str) -> C4Workspace {
    parse(&LineDocument::parse(text))
}

/// The workspace `document` declares.
pub fn parse(document: &LineDocument) -> C4Workspace {
    let mut state = State::new();
    let mut in_block_comment = false;
    for line in document.code_lines() {
        let code = skip_block_comments(line.code(), &mut in_block_comment);
        if code.is_empty() {
            continue;
        }
        let tokens = tokens::split(&code);

        // Noted, not followed: ADP writes only this document, so an edit landing in an included file could not be
        // saved.
        if tokens.len() >= 2 && same(&tokens[0], "!include") {
            state.includes.push(tokens[1].clone());
            continue;
        }
        read_line(&tokens, line.number, &mut state);
    }
    C4Workspace {
        name: state.workspace_name,
        elements: state.elements,
        relationships: state.relationships,
        views: state.views,
        styles: state.styles,
        includes: state.includes,
    }
}

/// `code` with any block-commented span removed, tracking whether the comment continues past this line.
fn skip_block_comments(code: &str, in_block_comment: &mut bool) -> String {
    let mut result = String::new();
    let mut index = 0;
    while index < code.len() {
        if *in_block_comment {
            let Some(close) = code[index..].find("*/") else {
                return result;
            };
            *in_block_comment = false;
            index += close + 2;
            continue;
        }
        let Some(open) = code[index..].find("/*") else {
            result.push_str(&code[index..]);
            return result;
        };
        result.push_str(&code[index..index + open]);
        *in_block_comment = true;
        index += open + 2;
    }
    result
}

fn read_line(tokens: &[String], number: u32, state: &mut State) {
    if tokens.is_empty() {
        return;
    }

    // A closing brace pops whatever the matching opener pushed.
    if tokens[0] == "}" {
        state.pop();
        return;
    }

    let opens_block = tokens.last().is_some_and(|t| t == "{");
    let mut body: &[String] = if opens_block {
        &tokens[..tokens.len() - 1]
    } else {
        tokens
    };
    if body.is_empty() {
        state.push(Scope::Unknown);
        return;
    }

    // `identifier = keyword ...`: the identifier is the model's own handle on the element.
    let mut identifier = None;
    if body.len() >= 3 && body[1] == "=" {
        identifier = Some(body[0].clone());
        body = &body[2..];
    }

    let keyword = body[0].as_str();
    let arguments = &body[1..];

    match state.current() {
        Scope::Views => read_in_views(keyword, arguments, number, opens_block, state),
        Scope::Styles => read_in_styles(keyword, arguments, number, opens_block, state),
        Scope::View => read_inside_a_view(keyword, arguments, body, number, opens_block, state),
        _ => read_in_model(keyword, arguments, body, identifier, number, opens_block, state),
    }
}

fn read_in_model(
    keyword: &str,
    arguments: &[String],
    body: &[String],
    identifier: Option<String>,
    number: u32,
    opens_block: bool,
    state: &mut State,
) {
    let scope = |within: Scope| if opens_block { within } else { Scope::Unknown };
    match keyword.to_lowercase().as_str() {
        "workspace" => {
            state.workspace_name = arguments.first().cloned().unwrap_or_default();
            state.push(scope(Scope::Workspace));
            return;
        }
        "model" => {
            state.push(scope(Scope::Model));
            return;
        }
        "views" => {
            state.push(scope(Scope::Views));
            return;
        }
        // A deployment environment is read like the model; a group is a visual cluster, not an abstraction level,
        // so its children belong to whatever contains it and nothing is pushed onto the parent chain.
        "deploymentenvironment" | "group" => {
            state.push(scope(Scope::Model));
            return;
        }
        _ => {}
    }

    // `a -> b "description" "technology"`
    if body.len() >= 3 && body[1] == "->" {
        state.relationships.push(C4Relationship {
            source_id: body[0].clone(),
            destination_id: body[2].clone(),
            description: body.get(3).cloned().unwrap_or_default(),
            technology: body.get(4).cloned().unwrap_or_default(),
            line: number,
        });
        if opens_block {
            state.push(Scope::Unknown);
        }
        return;
    }

    // A relationship declared inside an element block: `-> destination "description"`.
    if keyword == "->"
        && !arguments.is_empty()
        && let Some(source) = state.current_element()
    {
        state.relationships.push(C4Relationship {
            source_id: source,
            destination_id: arguments[0].clone(),
            description: arguments.get(1).cloned().unwrap_or_default(),
            technology: arguments.get(2).cloned().unwrap_or_default(),
            line: number,
        });
        return;
    }

    // `tags "A" "B"` on a line of its own inside an element's block.
    if same(keyword, "tags")
        && !arguments.is_empty()
        && let Some(tagged) = state.current_element()
    {
        let tags = arguments.iter().flat_map(|a| split_tags(a)).collect();
        state.add_tags(&tagged, tags);
        return;
    }

    let Some(kind) = element_keyword(keyword) else {
        // Not something this parser models (`!docs`, `properties`, `url`): the document keeps the text, and only a
        // block's nesting is tracked so the parent chain stays correct.
        if opens_block {
            state.push(Scope::Unknown);
        }
        return;
    };

    let is_instance = kind.is_instance();
    let (technology, tags) = read_technology_and_tags(kind, arguments);
    let id = match identifier {
        Some(id) => id,
        None => {
            let named = arguments.first().map_or(keyword, String::as_str).to_owned();
            state.generate_id(kind, &named)
        }
    };
    let element = C4Element {
        id,
        kind,
        name: if is_instance {
            String::new()
        } else {
            arguments.first().cloned().unwrap_or_default()
        },
        description: if is_instance {
            String::new()
        } else {
            arguments.get(1).cloned().unwrap_or_default()
        },
        technology,
        tags,
        parent_id: state.current_element(),
        line: number,
        referenced_id: if is_instance { arguments.first().cloned() } else { None },
    };
    let id = element.id.clone();
    state.elements.push(element);
    if opens_block {
        state.push_element(id);
    }
}

fn read_in_views(keyword: &str, arguments: &[String], number: u32, opens_block: bool, state: &mut State) {
    if same(keyword, "styles") {
        state.push(if opens_block { Scope::Styles } else { Scope::Unknown });
        return;
    }
    let Some(kind) = view_keyword(keyword) else {
        // `theme`, `branding`, `terminology`, `properties`: carried by the document, not modelled.
        if opens_block {
            state.push(Scope::Unknown);
        }
        return;
    };

    // systemLandscape "key"; systemContext, container, component and dynamic <scope> "key"; deployment <scope>
    // "environment" "key".
    let (scope, environment, key) = match kind {
        C4ViewKind::SystemLandscape => (
            None,
            None,
            arguments.first().cloned().unwrap_or_else(|| "landscape".to_owned()),
        ),
        C4ViewKind::Deployment => {
            let environment = arguments.get(1).cloned();
            let key = arguments
                .get(2)
                .cloned()
                .or_else(|| environment.clone())
                .unwrap_or_else(|| "deployment".to_owned());
            (arguments.first().cloned(), environment, key)
        }
        _ => (
            arguments.first().cloned(),
            None,
            arguments.get(1).cloned().unwrap_or_else(|| kind.name().to_lowercase()),
        ),
    };
    state.views.push(C4View {
        kind,
        key,
        scope_id: scope,
        environment,
        title: None,
        includes_everything: false,
        includes: Vec::new(),
        excludes: Vec::new(),
        auto_layout: None,
        interactions: Vec::new(),
        line: number,
    });
    state.push(if opens_block { Scope::View } else { Scope::Unknown });
    if opens_block {
        state.current_view = Some(state.views.len() - 1);
    }
}

fn read_inside_a_view(
    keyword: &str,
    arguments: &[String],
    body: &[String],
    number: u32,
    opens_block: bool,
    state: &mut State,
) {
    let Some(index) = state.current_view else {
        if opens_block {
            state.push(Scope::Unknown);
        }
        return;
    };
    let view = &mut state.views[index];
    match keyword.to_lowercase().as_str() {
        "include" => {
            if arguments.iter().any(|a| a == "*") {
                view.includes_everything = true;
            } else {
                view.includes.extend(arguments.iter().cloned());
            }
            return;
        }
        "exclude" => {
            view.excludes.extend(arguments.iter().cloned());
            return;
        }
        "title" => {
            view.title = arguments.first().cloned();
            return;
        }
        "autolayout" => {
            view.auto_layout = Some(C4AutoLayout {
                direction: arguments.first().cloned().unwrap_or_else(|| "tb".to_owned()),
                rank_separation: arguments.get(1).and_then(|a| a.parse().ok()),
                node_separation: arguments.get(2).and_then(|a| a.parse().ok()),
            });
            return;
        }
        _ => {}
    }

    // A dynamic view's body is its ordered interactions: `source -> destination "description"`.
    if body.len() >= 3 && body[1] == "->" {
        let order = (view.interactions.len() + 1).to_string();
        view.interactions.push(C4Interaction {
            source_id: body[0].clone(),
            destination_id: body[2].clone(),
            description: body.get(3).cloned().unwrap_or_default(),
            order,
            line: number,
        });
        return;
    }
    if opens_block {
        state.push(Scope::Unknown);
    }
}

fn read_in_styles(keyword: &str, arguments: &[String], number: u32, opens_block: bool, state: &mut State) {
    if same(keyword, "element") && !arguments.is_empty() {
        state.styles.push(C4ElementStyle {
            tag: arguments[0].clone(),
            background: None,
            color: None,
            shape: None,
            border: None,
            line: number,
        });
        state.push(if opens_block { Scope::Style } else { Scope::Unknown });
        return;
    }
    if opens_block {
        state.push(Scope::Unknown);
    }
}

/// The technology and tags of an element's arguments, whose positions depend on the keyword: a person and a software
/// system take `name, description, tags` and have no technology; an instance takes `<ref> [tags]`; a deployment node
/// `name [description] [technology] [tags] [instances]`, its instance count after its tags.
fn read_technology_and_tags(kind: C4ElementKind, arguments: &[String]) -> (String, Vec<String>) {
    let tags_at = |index: usize| arguments.get(index).map(|t| split_tags(t)).unwrap_or_default();
    match kind {
        C4ElementKind::ContainerInstance | C4ElementKind::SoftwareSystemInstance => (String::new(), tags_at(1)),
        C4ElementKind::Person | C4ElementKind::SoftwareSystem => (String::new(), tags_at(2)),
        _ => (arguments.get(2).cloned().unwrap_or_default(), tags_at(3)),
    }
}

/// A tag list as written, `"A, B"`, split at its commas.
fn split_tags(tags: &str) -> Vec<String> {
    tags.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

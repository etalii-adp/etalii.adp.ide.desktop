//! `reader.markdown` and `render.markdown` (ATD §13.3; their contracts and fixtures are in
//! `specifications/atd/capabilities/`): the ATX headings outside code fences as the outline's elements, and the
//! document as text runs by block for the preview. Moved (not rewritten) from the Markdown editor's module
//! (`modules/editors/markdown`), which calls it here (etalii.adp spec 035, T125), itself after the standalone host's
//! `MarkdownEditorPanel` at `8b6ccfc` (spec 030, FR-020). The preview is a list of blocks the shell draws; this module
//! knows no toolkit.

use adp_fbl::model::{FblElement, Finding};
use adp_fbl::plugins::{
    PersistencePlugin, PluginPlanRequest, PluginPlanResult, PluginReadRequest, PluginReadResult, PluginTemplateRequest,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// One heading of the outline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: usize,
    pub title: String,
    /// From 1, the editor's own line numbering.
    pub line: usize,
}

/// ATX headings only, skipping code fences, exactly as the standalone host's `headingsOf` reads them.
pub fn headings_of(text: &str) -> Vec<Heading> {
    let mut headings = Vec::new();
    let mut in_fence = false;
    for (index, line) in text.split('\n').enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if !(1..=6).contains(&hashes) {
            continue;
        }
        let rest = &line[hashes..];
        if !rest.starts_with(|c: char| c.is_whitespace()) {
            continue;
        }
        // `^(#{1,6})\s+(.+?)\s*#*\s*$`: the title without its closing hashes and the space around them.
        let title = rest.trim().trim_end_matches('#').trim_end();
        let title = if title.is_empty() { rest.trim() } else { title };
        if !title.is_empty() {
            headings.push(Heading {
                level: hashes,
                title: title.to_owned(),
                line: index + 1,
            });
        }
    }
    headings
}

/// A run of text within a block, with how it is set.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub strong: bool,
    pub emphasis: bool,
    pub code: bool,
    /// The link's address, when the run is a link.
    pub link: Option<String>,
}

/// One block of the rendered document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Heading {
        level: usize,
        spans: Vec<Span>,
    },
    Paragraph {
        spans: Vec<Span>,
        quote_depth: usize,
    },
    /// A list item at its nesting depth (from 0), with its number when the list is ordered.
    Item {
        depth: usize,
        number: Option<u64>,
        spans: Vec<Span>,
    },
    Code {
        language: String,
        text: String,
    },
    Rule,
}

/// The document as blocks, CommonMark with tables, strikethrough and task lists read as text.
pub fn preview(text: &str) -> Vec<Block> {
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut blocks = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    let (mut strong, mut emphasis) = (0usize, 0usize);
    let mut link: Option<String> = None;
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut item_number: Option<Option<u64>> = None;
    let mut quote_depth = 0usize;
    let mut code: Option<(String, String)> = None;
    let mut heading: Option<usize> = None;

    let push_text =
        |spans: &mut Vec<Span>, text: &str, strong: usize, emphasis: usize, code: bool, link: &Option<String>| {
            spans.push(Span {
                text: text.to_owned(),
                strong: strong > 0,
                emphasis: emphasis > 0,
                code,
                link: link.clone(),
            });
        };
    let flush =
        |blocks: &mut Vec<Block>, spans: &mut Vec<Span>, item: &mut Option<Option<u64>>, depth: usize, quote: usize| {
            if spans.is_empty() {
                return;
            }
            let taken = std::mem::take(spans);
            match item.take() {
                Some(number) => blocks.push(Block::Item {
                    depth,
                    number,
                    spans: taken,
                }),
                None => blocks.push(Block::Paragraph {
                    spans: taken,
                    quote_depth: quote,
                }),
            }
        };

    for event in Parser::new_ext(text, options) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => heading = Some(level_number(level)),
            Event::End(TagEnd::Heading(_)) => {
                blocks.push(Block::Heading {
                    level: heading.take().unwrap_or(1),
                    spans: std::mem::take(&mut spans),
                });
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or_default().to_owned(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((language, String::new()));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((language, text)) = code.take() {
                    blocks.push(Block::Code {
                        language,
                        text: text.trim_end_matches('\n').to_owned(),
                    });
                }
            }
            Event::Start(Tag::List(start)) => {
                flush(
                    &mut blocks,
                    &mut spans,
                    &mut item_number,
                    lists.len().saturating_sub(1),
                    quote_depth,
                );
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                flush(
                    &mut blocks,
                    &mut spans,
                    &mut item_number,
                    lists.len().saturating_sub(1),
                    quote_depth,
                );
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                flush(
                    &mut blocks,
                    &mut spans,
                    &mut item_number,
                    lists.len().saturating_sub(1),
                    quote_depth,
                );
                let number = lists.last_mut().and_then(|start| {
                    let current = *start;
                    if let Some(n) = start {
                        *n += 1;
                    }
                    current
                });
                item_number = Some(number);
            }
            Event::End(TagEnd::Item) => flush(
                &mut blocks,
                &mut spans,
                &mut item_number,
                lists.len().saturating_sub(1),
                quote_depth,
            ),
            Event::Start(Tag::BlockQuote(_)) => quote_depth += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote_depth = quote_depth.saturating_sub(1),
            // A paragraph inside a list item ends with the item.
            Event::End(TagEnd::Paragraph) if item_number.is_none() => {
                flush(&mut blocks, &mut spans, &mut item_number, 0, quote_depth);
            }
            Event::Start(Tag::Strong) => strong += 1,
            Event::End(TagEnd::Strong) => strong = strong.saturating_sub(1),
            Event::Start(Tag::Emphasis) => emphasis += 1,
            Event::End(TagEnd::Emphasis) => emphasis = emphasis.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => link = None,
            Event::Text(text) => match &mut code {
                Some((_, body)) => body.push_str(&text),
                None => push_text(&mut spans, &text, strong, emphasis, false, &link),
            },
            Event::Code(text) => push_text(&mut spans, &text, strong, emphasis, true, &link),
            Event::SoftBreak => push_text(&mut spans, " ", strong, emphasis, false, &link),
            Event::HardBreak => push_text(&mut spans, "\n", strong, emphasis, false, &link),
            Event::TaskListMarker(done) => push_text(&mut spans, if done { "☑ " } else { "☐ " }, 0, 0, false, &None),
            Event::Rule => blocks.push(Block::Rule),
            Event::End(TagEnd::TableCell) => push_text(&mut spans, "  ", 0, 0, false, &None),
            Event::End(TagEnd::TableRow | TagEnd::TableHead) => {
                flush(&mut blocks, &mut spans, &mut item_number, 0, quote_depth)
            }
            _ => {}
        }
    }
    flush(&mut blocks, &mut spans, &mut item_number, 0, quote_depth);
    blocks
}

fn level_number(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// `reader.markdown` as a persistence plugin (FBL §11): each heading an element `heading` with its `level`, `title` and
/// `line`, in the order of the text. The text is the editor's, so it plans no edit.
pub struct MarkdownReader;

impl PersistencePlugin for MarkdownReader {
    fn id(&self) -> &str {
        "reader.markdown"
    }

    fn read(&self, request: &PluginReadRequest) -> PluginReadResult {
        let text = request
            .files
            .first()
            .map(|file| String::from_utf8_lossy(&file.bytes).into_owned())
            .unwrap_or_default();
        // Where each line starts, for the span of the heading on it.
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(at, _)| at + 1));
        let elements = headings_of(&text)
            .into_iter()
            .map(|heading| {
                let start = starts[heading.line - 1];
                let end = starts.get(heading.line).map_or(text.len(), |next| next - 1);
                let mut attributes = adp_cel::Map::default();
                attributes.insert("level", adp_cel::Value::Int(heading.level as i64));
                attributes.insert("title", adp_cel::Value::String(heading.title));
                attributes.insert("line", adp_cel::Value::Int(heading.line as i64));
                FblElement {
                    id: format!("heading-{}", heading.line),
                    id_is_stored: false,
                    type_name: "heading".into(),
                    rule: "heading".into(),
                    is_relation: false,
                    attributes,
                    parent_id: None,
                    parent_slot: None,
                    source: None,
                    target: None,
                    referenced: Vec::new(),
                    own_span: adp_fbl::text::Span::new(start, end),
                    line: heading.line,
                    file: None,
                }
            })
            .collect();
        PluginReadResult::new(elements, Vec::<Finding>::new(), false)
    }

    fn plan(&self, _request: &PluginPlanRequest<'_>) -> PluginPlanResult {
        PluginPlanResult::Refused("The editor writes the text; its outline plans no edit.".into())
    }

    fn template(&self, _request: &PluginTemplateRequest) -> Vec<u8> {
        Vec::new()
    }
}

/// `render.markdown`'s `$defs/RenderResult` for a text: the preview's runs, each with the block it stands in
/// (`heading-<level>`, `paragraph`, `item`, `code`, `rule`) and how it is set.
pub fn runs(text: &str) -> serde_json::Value {
    use serde_json::{Map, json};

    let span_runs = |spans: &[Span], block: &str, extra: &Map<String, serde_json::Value>| -> Vec<serde_json::Value> {
        spans
            .iter()
            .map(|span| {
                let mut style = extra.clone();
                for (key, on) in [
                    ("strong", span.strong),
                    ("emphasis", span.emphasis),
                    ("code", span.code),
                ] {
                    if on {
                        style.insert(key.into(), json!(true));
                    }
                }
                if let Some(link) = &span.link {
                    style.insert("link".into(), json!(link));
                }
                json!({"text": span.text, "style": style, "block": block})
            })
            .collect()
    };
    let mut runs = Vec::new();
    for block in preview(text) {
        match block {
            Block::Heading { level, spans } => runs.extend(span_runs(&spans, &format!("heading-{level}"), &Map::new())),
            Block::Paragraph { spans, quote_depth } => {
                let mut extra = Map::new();
                if quote_depth > 0 {
                    extra.insert("quoteDepth".into(), json!(quote_depth));
                }
                runs.extend(span_runs(&spans, "paragraph", &extra));
            }
            Block::Item { depth, number, spans } => {
                let mut extra = Map::new();
                extra.insert("depth".into(), json!(depth));
                if let Some(number) = number {
                    extra.insert("number".into(), json!(number));
                }
                runs.extend(span_runs(&spans, "item", &extra));
            }
            Block::Code { language, text } => {
                let mut style = Map::new();
                if !language.is_empty() {
                    style.insert("language".into(), json!(language));
                }
                runs.push(json!({"text": text, "style": style, "block": "code"}));
            }
            Block::Rule => runs.push(json!({"text": "", "style": {}, "block": "rule"})),
        }
    }
    json!({ "runs": runs })
}

/// `render.markdown` at the `render` extension point.
pub struct MarkdownRenderer;

impl super::RenderCapability for MarkdownRenderer {
    fn render(&self, input: &serde_json::Value, _args: &serde_json::Value) -> Result<serde_json::Value, String> {
        let text = input["value"].as_str().ok_or("The input's `value` is not a text.")?;
        Ok(runs(text))
    }
}

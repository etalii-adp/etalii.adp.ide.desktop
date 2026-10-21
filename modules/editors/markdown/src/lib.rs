//! The Markdown editor's outline and preview (etalii.adp spec 030, FR-020, task T030), after the standalone host's
//! `MarkdownEditorPanel` at `8b6ccfc`: every ATX heading outside a code fence, clickable to its line, above the text,
//! and the rendered document beside it. The preview is a list of blocks the shell draws; this crate knows no toolkit.

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

//! The Markdown editor's outline and preview (etalii.adp spec 030, FR-020, task T030) against the standalone host's
//! example `src/editors/markdown/examples/guide.md` at `8b6ccfc`.

use adp_editor_markdown::{Block, Heading, headings_of, preview};

const GUIDE: &str = include_str!("examples/guide.md");

#[test]
fn the_outline_lists_every_heading_and_skips_code_fences() {
    let headings = headings_of(GUIDE);
    let titles: Vec<(usize, &str)> = headings.iter().map(|h| (h.level, h.title.as_str())).collect();
    assert_eq!(
        titles,
        [
            (1, "Field guide"),
            (2, "Reading the preview"),
            (3, "Emphasis"),
            (2, "Code fences"),
            (2, "Closing notes")
        ]
    );
    assert_eq!(headings[0].line, 1);
    let fence = GUIDE.lines().position(|l| l.starts_with("## Code fences")).unwrap();
    assert_eq!(headings[3].line, fence + 1);
}

#[test]
fn closing_hashes_and_lines_without_a_space_are_read_as_the_standalone_host_reads_them() {
    assert_eq!(
        headings_of("## Title ##\n#NotAHeading\n####### seven\n"),
        [Heading {
            level: 2,
            title: "Title".into(),
            line: 1
        }]
    );
}

#[test]
fn the_preview_renders_headings_emphasis_code_and_lists() {
    let blocks = preview(
        "# A\n\nText *em* **strong** `code` [link](http://x).\n\n- one\n- two\n\n1. first\n\n```rust\nfn x() {}\n```\n\n---\n",
    );
    assert!(matches!(&blocks[0], Block::Heading { level: 1, spans } if spans[0].text == "A"));
    let Block::Paragraph { spans, .. } = &blocks[1] else {
        panic!("{:?}", blocks[1])
    };
    assert!(spans.iter().any(|s| s.emphasis && s.text == "em"));
    assert!(spans.iter().any(|s| s.strong && s.text == "strong"));
    assert!(spans.iter().any(|s| s.code && s.text == "code"));
    assert!(spans.iter().any(|s| s.link.as_deref() == Some("http://x")));
    let items: Vec<_> = blocks.iter().filter(|b| matches!(b, Block::Item { .. })).collect();
    assert_eq!(items.len(), 3);
    assert!(matches!(items[2], Block::Item { number: Some(1), .. }));
    assert!(
        blocks
            .iter()
            .any(|b| matches!(b, Block::Code { language, text } if language == "rust" && text == "fn x() {}"))
    );
    assert_eq!(blocks.last(), Some(&Block::Rule));
}

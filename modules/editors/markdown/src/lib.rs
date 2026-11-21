//! The Markdown editor's outline and preview (etalii.adp spec 030, FR-020, task T030), after the standalone host's
//! `MarkdownEditorPanel` at `8b6ccfc`: every ATX heading outside a code fence, clickable to its line, above the text,
//! and the rendered document beside it. The code is the capabilities `reader.markdown` and `render.markdown`, which
//! live in the capability registry (etalii.adp spec 035, T125); the editor calls them there.

pub use adp_atd_runtime::capabilities::markdown::{Block, Heading, Span, headings_of, preview};

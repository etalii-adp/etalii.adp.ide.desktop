//! Positions in a file (ATD §3.1): byte offsets, and 1-based lines and columns counted in code points.

/// The starts of the lines of a text, to turn byte offsets into lines and columns.
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        LineIndex { starts }
    }

    /// The 1-based line and column, in code points, of a byte offset.
    pub fn position(&self, text: &str, offset: usize) -> (usize, usize) {
        let line = self.starts.partition_point(|s| *s <= offset) - 1;
        let start = self.starts[line];
        let offset = offset.min(text.len());
        let column = text[start..offset].chars().count() + 1;
        (line + 1, column)
    }

    /// The byte offset of a 1-based line and a column in code points.
    pub fn offset(&self, text: &str, line: usize, column: usize) -> usize {
        let Some(&start) = self.starts.get(line.saturating_sub(1)) else {
            return text.len();
        };
        let end = self.starts.get(line).copied().unwrap_or(text.len());
        text[start..end]
            .char_indices()
            .nth(column.saturating_sub(1))
            .map(|(i, _)| start + i)
            .unwrap_or(end.saturating_sub(usize::from(end > start && text.as_bytes()[end - 1] == b'\n')))
    }

    /// The column of a byte offset in UTF-16 code units from its line's start, as the Language Server Protocol counts
    /// by default.
    pub fn utf16_column(&self, text: &str, offset: usize) -> usize {
        let line = self.starts.partition_point(|s| *s <= offset) - 1;
        text[self.starts[line]..offset.min(text.len())].encode_utf16().count()
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.starts[line.saturating_sub(1).min(self.starts.len() - 1)]
    }
}

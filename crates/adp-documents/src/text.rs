//! A text file the editors can hold (the standalone host's `TextFileBuffer` at `8b6ccfc`, modular-text-editors
//! Requirements 6 and 7): UTF-8 with or without a byte order mark, or ASCII, each line keeping its own terminator so an
//! unchanged file saves back byte for byte, mixed line endings included. Anything else is refused with a reason, never
//! guessed at: a file over the size limit before it is read, a binary file, an encoding other than UTF-8.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// A design-time bound for a hand-edited text file, as in the standalone host.
pub const SIZE_LIMIT: u64 = 5 * 1024 * 1024;
const BINARY_PROBE: usize = 8 * 1024;
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

/// An open text file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFile {
    path: PathBuf,
    has_bom: bool,
    /// The text exactly as read, terminators included, the byte order mark excluded.
    content: String,
    /// Each line's own terminator: `"\r\n"`, `"\n"`, or `""` for a last line without one.
    terminators: Vec<&'static str>,
    dominant: &'static str,
}

impl TextFile {
    /// Opens a file, or refuses with a reason a person can act on. The size is checked before any byte is read.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let name = file_name(path);
        let metadata = fs::metadata(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => format!("'{name}' does not exist."),
            _ => format!("'{name}' could not be read: {e}"),
        })?;
        if metadata.len() > SIZE_LIMIT {
            return Err(format!(
                "'{name}' is {} MB, over the {} MB limit for text editing.",
                metadata.len() / (1024 * 1024),
                SIZE_LIMIT / (1024 * 1024)
            ));
        }
        let bytes = fs::read(path).map_err(|e| format!("'{name}' could not be read: {e}"))?;
        let has_bom = bytes.starts_with(&BOM);
        let body = if has_bom { &bytes[3..] } else { &bytes[..] };
        if looks_binary(body) {
            return Err(format!(
                "'{name}' looks like a binary file, which no text editor here can open. If it is a diagram body, open it through its diagram instead."
            ));
        }
        let content = std::str::from_utf8(body)
            .map_err(|_| {
                format!("'{name}' is not UTF-8 or ASCII, the encodings text editing supports. Convert it to UTF-8 to edit it here.")
            })?
            .to_owned();
        let (terminators, dominant) = terminators_of(&content);
        Ok(Self {
            path: path.to_path_buf(),
            has_bom,
            content,
            terminators,
            dominant,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// The encoding, as the property grid words it.
    pub fn encoding_name(&self) -> &'static str {
        if self.has_bom { "UTF-8 with BOM" } else { "UTF-8" }
    }

    /// The line-ending style: the one style, or the dominant one named as mixed.
    pub fn line_ending_style(&self) -> String {
        let name = |t: &str| if t == "\r\n" { "CRLF" } else { "LF" };
        let mut used: Vec<&str> = self.terminators.iter().copied().filter(|t| !t.is_empty()).collect();
        used.dedup();
        used.sort();
        used.dedup();
        match used.len() {
            0 => name(self.dominant).into(),
            1 => name(used[0]).into(),
            _ => format!("Mixed (mostly {})", name(self.dominant)),
        }
    }

    pub fn line_count(&self) -> usize {
        self.terminators.len()
    }

    /// The bytes `content` saves as: every line that existed keeps its own terminator, new lines take the dominant
    /// style, a last line keeps the original's choice of ending without a newline, and the byte order mark returns
    /// only when the file had one.
    pub fn render(&self, content: &str) -> Vec<u8> {
        let lines = split_lines(content);
        let mut text = String::with_capacity(content.len() + 16);
        for (index, (line, terminator)) in lines.iter().enumerate() {
            text.push_str(line);
            let is_last = index == lines.len() - 1;
            let original = self.terminators.get(index).copied();
            let ending = if is_last && terminator.is_empty() {
                original.unwrap_or("")
            } else {
                match original {
                    Some(o) if !o.is_empty() => o,
                    _ => self.dominant,
                }
            };
            text.push_str(ending);
        }
        let mut bytes = Vec::with_capacity(text.len() + 3);
        if self.has_bom {
            bytes.extend_from_slice(&BOM);
        }
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }

    /// Saves `content` through a temporary file beside the target, so a reader sees the old file or the new one.
    pub fn save(&mut self, content: &str) -> Result<(), String> {
        let bytes = self.render(content);
        write_atomically(&self.path, &bytes)
            .map_err(|e| format!("'{}' could not be saved: {e}", file_name(&self.path)))?;
        let rendered = String::from_utf8_lossy(if self.has_bom { &bytes[3..] } else { &bytes[..] }).into_owned();
        let (terminators, dominant) = terminators_of(&rendered);
        self.content = rendered;
        self.terminators = terminators;
        self.dominant = dominant;
        Ok(())
    }
}

/// Writes a file through a temporary file in the same folder and a rename over the target.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let folder = path.parent().unwrap_or_else(|| Path::new("."));
    let temporary = folder.join(format!(".{}.{}.adp-save", file_name(path), std::process::id()));
    {
        let mut file = fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn terminators_of(content: &str) -> (Vec<&'static str>, &'static str) {
    let terminators: Vec<&'static str> = split_lines(content).into_iter().map(|(_, t)| t).collect();
    let crlf = terminators.iter().filter(|t| **t == "\r\n").count();
    let lf = terminators.iter().filter(|t| **t == "\n").count();
    (terminators, if crlf > lf { "\r\n" } else { "\n" })
}

/// Splits on `\n`, taking a preceding `\r` into the terminator; a final line without a newline has `""`.
fn split_lines(content: &str) -> Vec<(&str, &'static str)> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = content.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        let carriage = index > 0 && bytes[index - 1] == b'\r';
        let end = if carriage { index - 1 } else { index };
        lines.push((&content[start..end], if carriage { "\r\n" } else { "\n" }));
        start = index + 1;
    }
    if start < content.len() || lines.is_empty() {
        lines.push((&content[start..], ""));
    }
    lines
}

/// A null byte, or a majority of non-printable bytes in the first 8 KB, reads as binary.
fn looks_binary(body: &[u8]) -> bool {
    let probe = &body[..body.len().min(BINARY_PROBE)];
    if probe.is_empty() {
        return false;
    }
    let mut suspicious = 0;
    for &value in probe {
        if value == 0 {
            return true;
        }
        if value < 0x09 || (value > 0x0D && value < 0x20) {
            suspicious += 1;
        }
    }
    suspicious * 2 > probe.len()
}

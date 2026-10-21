//! Open text documents and the editors' analysis (etalii.adp spec 030, FR-014, FR-020): which analysis a file takes
//! by its extension, run for the open editor as the text changes and for the errors and warnings panel when a
//! project is validated, so both say the same.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use adp_documents::TextFile;
use eframe::egui;

use crate::editor::{Mark, MarkLevel, TextEditor};

/// One finding of an editor's analysis, by byte span.
pub struct Found {
    pub span: Range<usize>,
    pub level: MarkLevel,
    pub message: String,
}

/// The editors with an analysis, by extension.
pub fn analysed(extension: &str) -> bool {
    matches!(extension, "json" | "yaml" | "yml" | "xml" | "xsd")
}

/// Runs the analysis of the editor a file's extension takes. `root` is the project the schema may be read from and
/// `document` the file's path in it; `stored` is how the file is stored ("UTF-8" or "UTF-8 with BOM").
pub fn analyse(extension: &str, text: &str, root: &Path, document: &[String], stored: &str) -> (Vec<Found>, String) {
    let root = root.to_path_buf();
    let read: adp_editor_json::ReadFile =
        Arc::new(move |path: &[String]| std::fs::read_to_string(root.join(path.join("/"))).ok());
    let level = |error: bool, warning: bool| {
        if error {
            MarkLevel::Error
        } else if warning {
            MarkLevel::Warning
        } else {
            MarkLevel::Info
        }
    };
    match extension {
        "json" => {
            use adp_editor_json::Severity::*;
            let analysis = adp_editor_json::analyse(text, document, read);
            let found = analysis
                .findings
                .into_iter()
                .map(|f| Found {
                    level: level(f.severity == Error, f.severity == Warning),
                    span: f.span,
                    message: f.message,
                })
                .collect();
            (found, analysis.status)
        }
        "xml" | "xsd" => {
            use adp_editor_xml::Severity::*;
            let analysis = adp_editor_xml::analyse(text, document, stored);
            let found = analysis
                .findings
                .into_iter()
                .map(|f| Found {
                    level: level(f.severity == Error, f.severity == Warning),
                    span: f.span,
                    message: f.message,
                })
                .collect();
            (found, analysis.status)
        }
        "yaml" | "yml" => {
            use adp_editor_yaml::Severity::*;
            let analysis = adp_editor_yaml::analyse(text, document, read);
            let found = analysis
                .findings
                .into_iter()
                .map(|f| Found {
                    level: level(f.severity == Error, f.severity == Warning),
                    span: f.span,
                    message: f.message,
                })
                .collect();
            (found, analysis.status)
        }
        _ => (Vec::new(), String::new()),
    }
}

/// A text file open in a tab: the file as read, for saving it back, and the component showing it.
pub struct OpenText {
    pub file: TextFile,
    pub editor: TextEditor,
    /// The project the file belongs to, or its own folder when it was opened alone.
    pub root: PathBuf,
    /// The buffer revision the analysis was made for, and what it says on the status line.
    analysed: Option<u64>,
    pub status: String,
    /// For Markdown: the outline and the preview, made for the analysed revision.
    pub markdown: Option<(Vec<adp_editor_markdown::Heading>, Vec<adp_editor_markdown::Block>)>,
    /// The file as it is on disk, when it changed there while this tab had other unsaved edits (T058).
    pub disk: Option<TextFile>,
}

impl OpenText {
    pub fn open(path: &Path, root: &Path) -> Result<Self, String> {
        let file = TextFile::open(path)?;
        let extension = extension_of(path);
        let editor = TextEditor::new(file.content(), &extension, egui::Id::unique(path));
        Ok(Self {
            file,
            editor,
            root: root.to_path_buf(),
            analysed: None,
            status: String::new(),
            markdown: None,
            disk: None,
        })
    }

    pub fn name(&self) -> String {
        self.file
            .path()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The tab's title, with VS Code's dot while there are unsaved changes.
    pub fn title(&self) -> String {
        if self.editor.buffer.is_modified() {
            format!("● {}", self.name())
        } else {
            self.name()
        }
    }

    /// Saves the text, keeping its encoding and line endings.
    pub fn save(&mut self) -> Result<(), String> {
        let text = self.editor.buffer.text();
        self.file.save(&text)?;
        self.editor.buffer.mark_saved();
        Ok(())
    }

    /// Runs the editor's analysis when the text changed (FR-020).
    pub fn analyse(&mut self) {
        let revision = self.editor.buffer.revision();
        if self.analysed == Some(revision) {
            return;
        }
        self.analysed = Some(revision);
        let extension = extension_of(self.file.path());
        let text = self.editor.buffer.text();
        if extension == "md" || extension == "markdown" {
            self.markdown = Some((
                adp_editor_markdown::headings_of(&text),
                adp_editor_markdown::preview(&text),
            ));
            return;
        }
        if !analysed(&extension) {
            return;
        }
        let document = document_path(&self.root, self.file.path());
        let (found, status) = analyse(&extension, &text, &self.root, &document, self.file.encoding_name());
        let char_at = |byte: usize| text.get(..byte.min(text.len())).map_or(0, |t| t.chars().count());
        self.editor.marks = found
            .into_iter()
            .map(|f| Mark {
                start: char_at(f.span.start),
                end: char_at(f.span.end),
                level: f.level,
                message: f.message,
            })
            .collect();
        let errors = self.editor.marks.iter().filter(|m| m.level == MarkLevel::Error).count();
        let warnings = self
            .editor
            .marks
            .iter()
            .filter(|m| m.level == MarkLevel::Warning)
            .count();
        self.status = format!("{status} · {errors} errors, {warnings} warnings");
    }
}

pub fn extension_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// A file's path in its project, as segments, its file name last; just the name when it is outside the project.
pub fn document_path(root: &Path, path: &Path) -> Vec<String> {
    match path.strip_prefix(root) {
        Ok(relative) => relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect(),
        Err(_) => path
            .file_name()
            .map(|n| vec![n.to_string_lossy().into_owned()])
            .unwrap_or_default(),
    }
}

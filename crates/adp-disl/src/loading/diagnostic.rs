//! What loading a specification reports (DISL section 14.1; the standalone host's `DislDiagnostic`): where, as the
//! JSON Pointer of the offending value, how bad, and what, in a sentence a tool engineer can act on.

use std::fmt;

/// How bad a diagnostic is: an error makes the specification unusable, a warning does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DislSeverity {
    Warning,
    Error,
}

/// One thing loading a specification found wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DislDiagnostic {
    pub pointer: String,
    pub severity: DislSeverity,
    pub message: String,
}

impl DislDiagnostic {
    pub(crate) fn error(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        DislDiagnostic {
            pointer: pointer.into(),
            severity: DislSeverity::Error,
            message: message.into(),
        }
    }

    pub(crate) fn warning(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        DislDiagnostic {
            pointer: pointer.into(),
            severity: DislSeverity::Warning,
            message: message.into(),
        }
    }
}

impl fmt::Display for DislDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            if self.severity == DislSeverity::Error {
                "error"
            } else {
                "warning"
            },
            if self.pointer.is_empty() { "/" } else { &self.pointer },
            self.message
        )
    }
}

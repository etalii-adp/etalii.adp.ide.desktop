//! What loading a specification reports (DESL section 10.1): where, as the JSON Pointer of the offending value, how
//! bad, and what, in a sentence a tool engineer can act on. The shape of `adp-disl`'s `DislDiagnostic`.

use std::fmt;

/// How bad a diagnostic is: an error makes the specification unusable, a warning does not (DESL section 10.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeslSeverity {
    Warning,
    Error,
}

/// One thing loading a specification found wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeslDiagnostic {
    pub pointer: String,
    pub severity: DeslSeverity,
    pub message: String,
}

impl DeslDiagnostic {
    pub(crate) fn error(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        DeslDiagnostic {
            pointer: pointer.into(),
            severity: DeslSeverity::Error,
            message: message.into(),
        }
    }

    pub(crate) fn warning(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        DeslDiagnostic {
            pointer: pointer.into(),
            severity: DeslSeverity::Warning,
            message: message.into(),
        }
    }
}

impl fmt::Display for DeslDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}: {}",
            if self.severity == DeslSeverity::Error {
                "error"
            } else {
                "warning"
            },
            if self.pointer.is_empty() { "/" } else { &self.pointer },
            self.message
        )
    }
}

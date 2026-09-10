//! # Numeris error model
//!
//! Every error must explain:
//! 1. what happened,
//! 2. why it happened,
//! 3. what the user can do next.
//!
//! This is a product requirement from the authoritative specification.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Machine-readable error kind. Used for tests and UI routing; humans read `what/why/action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// The request itself is invalid (unknown variable, malformed command, bad option).
    InvalidRequest,
    /// There is not enough data to perform the computation.
    InsufficientData,
    /// The statistical model is not identified (collinearity, weak instruments, …).
    NotIdentified,
    /// Numerical computation failed (singular matrix, no convergence, …).
    NumericalFailure,
    /// Convergence failure in iterative estimation.
    ConvergenceFailure,
    /// Input/output error (missing file, permission, …).
    Io,
    /// The command is syntactically invalid.
    Syntax,
}

/// A structured Numeris error: what / why / what to do.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NumerisError {
    pub kind: ErrorKind,
    /// What happened, one line.
    pub what: String,
    /// Why it happened.
    pub why: String,
    /// What the user can do next.
    pub action: String,
}

impl NumerisError {
    pub fn new(
        kind: ErrorKind,
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            what: what.into(),
            why: why.into(),
            action: action.into(),
        }
    }

    pub fn invalid_request(
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::InvalidRequest, what, why, action)
    }

    pub fn insufficient_data(
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::InsufficientData, what, why, action)
    }

    pub fn not_identified(
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::NotIdentified, what, why, action)
    }

    pub fn numerical_failure(
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::NumericalFailure, what, why, action)
    }

    pub fn convergence_failure(
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::ConvergenceFailure, what, why, action)
    }

    pub fn io(what: impl Into<String>, why: impl Into<String>, action: impl Into<String>) -> Self {
        Self::new(ErrorKind::Io, what, why, action)
    }

    pub fn syntax(
        what: impl Into<String>,
        why: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::Syntax, what, why, action)
    }
}

impl fmt::Display for NumerisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n\nWhy: {}\n\nTo do: {}",
            self.what, self.why, self.action
        )
    }
}

impl std::error::Error for NumerisError {}

impl From<serde_json::Error> for NumerisError {
    fn from(err: serde_json::Error) -> Self {
        NumerisError::numerical_failure(
            format!("A structured result could not be serialized: {err}"),
            "The result object contains data that cannot be represented as JSON.",
            "This is an internal error; please report the command that triggered it.",
        )
    }
}

impl From<std::io::Error> for NumerisError {
    fn from(err: std::io::Error) -> Self {
        NumerisError::io(
            format!("Could not complete a file operation: {err}"),
            "The operating system reported an input/output error while reading or writing a file.",
            "Check that the file exists, that the path is correct, and that you have permission to access it.",
        )
    }
}

pub type Result<T> = std::result::Result<T, NumerisError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_explains_what_why_action() {
        let e = NumerisError::not_identified(
            "Regression is not identified.",
            "The design matrix is rank deficient: education is an exact linear combination of other predictors.",
            "Drop one of the collinear predictors or inspect variance inflation factors with 'estat vif'.",
        );
        let text = e.to_string();
        assert!(text.contains("Why:"));
        assert!(text.contains("To do:"));
    }
}

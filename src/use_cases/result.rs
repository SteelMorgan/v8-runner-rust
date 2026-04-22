use std::fmt;

use crate::support::error::AppError;

const VALIDATION_EXIT_CODE: i32 = 2;
const RUNTIME_EXIT_CODE: i32 = 3;
const PLATFORM_EXIT_CODE: i32 = 4;

/// Stable use-case error class used by transport adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseCaseErrorKind {
    Validation,
    Runtime,
    Platform,
}

impl UseCaseErrorKind {
    /// Maps the error kind to the CLI exit code.
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::Validation => VALIDATION_EXIT_CODE,
            Self::Runtime => RUNTIME_EXIT_CODE,
            Self::Platform => PLATFORM_EXIT_CODE,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Validation => "validation error",
            Self::Runtime => "runtime error",
            Self::Platform => "platform error",
        }
    }
}

/// Transport-neutral error metadata returned by use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UseCaseError {
    kind: UseCaseErrorKind,
    message: String,
}

impl UseCaseError {
    /// Creates a new use-case error.
    pub fn new(kind: UseCaseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Returns the error kind.
    pub const fn kind(&self) -> UseCaseErrorKind {
        self.kind
    }

    /// Returns the message without the prefixed kind label.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the CLI exit code associated with this error kind.
    pub const fn exit_code(&self) -> i32 {
        self.kind.exit_code()
    }
}

impl fmt::Display for UseCaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.label(), self.message)
    }
}

impl From<AppError> for UseCaseError {
    fn from(value: AppError) -> Self {
        match value {
            AppError::Validation(message) => Self::new(UseCaseErrorKind::Validation, message),
            AppError::Runtime(message) => Self::new(UseCaseErrorKind::Runtime, message),
            AppError::Platform(message) => Self::new(UseCaseErrorKind::Platform, message),
            AppError::Config(error) => Self::new(UseCaseErrorKind::Validation, error.to_string()),
        }
    }
}

/// A failed use-case execution with structured payload and transport-neutral error metadata.
#[derive(Debug, Clone)]
pub struct UseCaseFailure<T> {
    pub error: UseCaseError,
    pub payload: Option<T>,
}

impl<T> UseCaseFailure<T> {
    /// Creates a failure that should still be rendered as a structured command payload.
    pub fn with_payload(error: impl Into<UseCaseError>, payload: T) -> Self {
        Self {
            error: error.into(),
            payload: Some(payload),
        }
    }

    /// Creates a failure that should not emit a structured command payload.
    pub fn without_payload(error: impl Into<UseCaseError>) -> Self {
        Self {
            error: error.into(),
            payload: None,
        }
    }
}

/// The transport-neutral result contract for use-case execution.
pub type UseCaseResult<T> = Result<T, UseCaseFailure<T>>;

#[cfg(test)]
mod tests {
    use super::UseCaseErrorKind;

    #[test]
    fn use_case_error_kinds_keep_stable_cli_exit_codes() {
        assert_eq!(UseCaseErrorKind::Validation.exit_code(), 2);
        assert_eq!(UseCaseErrorKind::Runtime.exit_code(), 3);
        assert_eq!(UseCaseErrorKind::Platform.exit_code(), 4);
    }
}

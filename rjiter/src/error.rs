use crate::jiter::{JiterError, JiterErrorType, JsonErrorType, JsonType, LinePosition};
use thiserror::Error;

#[cfg(feature = "std")]
use alloc::{format, string::String};

pub type Result<T> = core::result::Result<T, Error>;

/// Custom I/O error for no_std compatibility
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("I/O operation failed")]
pub struct IoError;

/// Like `Jiter::JiterErrorType`, but also with `IoError`
#[derive(Error, Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ErrorType {
    #[error("JSON parsing error: {0}")]
    JsonError(JsonErrorType),
    #[error("expected {expected} but found {actual}")]
    WrongType {
        expected: JsonType,
        actual: JsonType,
    },
    #[error("I/O operation failed")]
    IoError(IoError),
}

/// An error from the `RJiter` iterator.
#[derive(Debug)]
pub struct Error {
    pub error_type: ErrorType,
    pub index: usize,
}

#[cfg(any(feature = "std", feature = "display"))]
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} at index {}", self.error_type, self.index)
    }
}

impl Error {
    pub(crate) fn from_jiter_error(index: usize, jiter_error: JiterError) -> Error {
        Error {
            error_type: match jiter_error.error_type {
                JiterErrorType::JsonError(json_error_type) => ErrorType::JsonError(json_error_type),
                JiterErrorType::WrongType { expected, actual } => {
                    ErrorType::WrongType { expected, actual }
                }
            },
            index: jiter_error.index + index,
        }
    }

    pub(crate) fn from_json_error(index: usize, json_error_type: JsonErrorType) -> Error {
        Error {
            error_type: ErrorType::JsonError(json_error_type),
            index,
        }
    }

    /// Get the position of the error in the stream.
    #[must_use]
    pub fn get_position<R: embedded_io::Read>(&self, rjiter: &crate::RJiter<R>) -> LinePosition {
        rjiter.error_position(self.index)
    }

    /// Write a description of the error with position information to the provided formatter.
    /// This is more embedded-friendly than returning a String as it doesn't allocate.
    #[cfg(any(feature = "std", feature = "display"))]
    pub fn write_description<R: embedded_io::Read>(
        &self,
        rjiter: &crate::RJiter<R>,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        let position = self.get_position(rjiter);
        write!(f, "{} at {}", self.error_type, position)
    }

    /// Get the description of the error with position information as a String.
    /// This is only available with std feature as it allocates.
    #[cfg(feature = "std")]
    pub fn description<R: embedded_io::Read>(&self, rjiter: &crate::RJiter<R>) -> String {
        let position = self.get_position(rjiter);
        format!("{} at {}", self.error_type, position)
    }
}

// Copy-paste from jiter/src/error.rs, where it is private
fn allowed_if_partial(error_type: &JsonErrorType) -> bool {
    matches!(
        error_type,
        JsonErrorType::EofWhileParsingList
            | JsonErrorType::EofWhileParsingObject
            | JsonErrorType::EofWhileParsingString
            | JsonErrorType::EofWhileParsingValue
            | JsonErrorType::ExpectedListCommaOrEnd
            | JsonErrorType::ExpectedObjectCommaOrEnd
    )
}

pub(crate) fn can_retry_if_partial(jiter_error: &JiterError) -> bool {
    if let JiterErrorType::JsonError(error_type) = &jiter_error.error_type {
        return allowed_if_partial(error_type);
    }
    false
}

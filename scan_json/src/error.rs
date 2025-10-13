//! Error types for JSON stream processing.

use thiserror::Error;

/// Error types for the JSON stream processor
#[derive(Error, Debug)]
pub enum Error {
    /// Error from the underlying `RJiter` JSON parser
    #[error("RJiter error: {0:?}")]
    RJiterError(rjiter::Error),
    /// Unhandled peek token encountered at position
    #[error("UnhandledPeek: {peek:?} at position {position}")]
    UnhandledPeek {
        /// The unexpected peek token encountered
        peek: rjiter::jiter::Peek,
        /// The byte position where the error occurred
        position: usize,
    },
    /// JSON structure is unbalanced at position
    #[error("Unbalanced JSON at position {0}")]
    UnbalancedJson(usize),
    /// Internal error with position and description
    #[error("Internal error at position {position}: {message}")]
    InternalError {
        /// The byte position where the error occurred
        position: usize,
        /// Description of the internal error
        message: &'static str,
    },
    /// Maximum nesting depth exceeded (current, max)
    #[error("Max nesting exceeded at position {position} with level {level}")]
    MaxNestingExceeded {
        /// The byte position where the error occurred
        position: usize,
        /// The nesting level that exceeded the maximum
        level: usize,
    },
    /// Error from user action at position
    #[error("Action error: {message} (code {code}) at position {position}")]
    ActionError {
        /// The error message from the user action
        message: &'static str,
        /// User-defined error code
        code: i32,
        /// The byte position where the error occurred
        position: usize,
    },
    /// IO error during processing
    #[error("IO error: {0:?}")]
    IOError(embedded_io::ErrorKind),
}

impl From<rjiter::Error> for Error {
    fn from(error: rjiter::Error) -> Self {
        Error::RJiterError(error)
    }
}

impl From<embedded_io::ErrorKind> for Error {
    fn from(error: embedded_io::ErrorKind) -> Self {
        Error::IOError(error)
    }
}

/// Type alias for Results with `scan_json` Error
pub type Result<B> = core::result::Result<B, Error>;

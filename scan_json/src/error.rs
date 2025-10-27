//! Error types for JSON stream processing.

/// Error types for the JSON stream processor
#[derive(Debug, Clone)]
pub enum Error {
    /// Error from the underlying `RJiter` JSON parser
    RJiterError(rjiter::Error),
    /// Unhandled peek token encountered at position
    UnhandledPeek {
        /// The unexpected peek token encountered
        peek: rjiter::jiter::Peek,
        /// The byte position where the error occurred
        position: usize,
    },
    /// JSON structure is unbalanced at position
    UnbalancedJson(usize),
    /// Internal error with position and description
    InternalError {
        /// The byte position where the error occurred
        position: usize,
        /// Description of the internal error
        message: &'static str,
    },
    /// Maximum nesting depth exceeded (current, max)
    MaxNestingExceeded {
        /// The byte position where the error occurred
        position: usize,
        /// The nesting level that exceeded the maximum
        level: usize,
    },
    /// Error from user action at position
    ActionError {
        /// The error message from the user action
        message: &'static str,
        /// The byte position where the error occurred
        position: usize,
    },
    /// IO error during processing
    IOError(embedded_io::ErrorKind),
}

#[cfg(any(feature = "std", feature = "display"))]
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::RJiterError(err) => write!(f, "{}", err),
            Error::UnhandledPeek { peek, position } => {
                write!(f, "UnhandledPeek: {:?} at position {}", peek, position)
            }
            Error::UnbalancedJson(position) => {
                write!(f, "Unbalanced JSON at position {}", position)
            }
            Error::InternalError { position, message } => {
                write!(f, "Internal error at position {}: {}", position, message)
            }
            Error::MaxNestingExceeded { position, level } => {
                write!(
                    f,
                    "Max nesting exceeded at position {} with level {}",
                    position, level
                )
            }
            Error::ActionError { message, position } => {
                write!(f, "Action error: {} at position {}", message, position)
            }
            Error::IOError(kind) => write!(f, "IO error: {:?}", kind),
        }
    }
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

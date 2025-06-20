//! Error types for JSON stream processing.

/// Error types for the JSON stream processor
///
/// - `ActionError`: Error returned from a trigger action
/// - `RJiterError`: Wraps errors from the underlying `RJiter` parser
/// - `UnhandledPeek`: Encountered an unexpected token type while peeking
/// - `UnbalancedJson`: JSON structure is not properly balanced at the given position
/// - `InternalError`: Internal processing error at the given position with message
/// - `MaxNestingExceeded`: JSON nesting level exceeded maximum at given position
/// - `IOError`: IO error when writing to the output stream. Reading errors are inside `RJiterError`
#[derive(Debug)]
pub enum Error {
    RJiterError(rjiter::Error),
    UnhandledPeek(rjiter::jiter::Peek, usize),
    UnbalancedJson(usize),
    InternalError(usize, String),
    MaxNestingExceeded(usize, usize),
    ActionError(Box<dyn std::error::Error>, usize),
    IOError(std::io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RJiterError(e) => write!(f, "RJiter error: {e}"), // position is inside `e`
            Error::ActionError(e, pos) => write!(f, "Action error: {e} at position {pos}"),
            Error::UnhandledPeek(p, pos) => write!(f, "UnhandledPeek: {p:?} at position {pos}"),
            Error::UnbalancedJson(pos) => write!(f, "Unbalanced JSON at position {pos}"),
            Error::InternalError(pos, msg) => write!(f, "Internal error at position {pos}: {msg}"),
            Error::MaxNestingExceeded(pos, level) => write!(
                f,
                "Max nesting exceeded at position {pos} with level {level}"
            ),
            Error::IOError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl From<rjiter::Error> for Error {
    fn from(error: rjiter::Error) -> Self {
        Error::RJiterError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::IOError(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

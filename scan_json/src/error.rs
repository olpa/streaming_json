#[derive(Debug)]
pub enum Error {
    RJiterError(rjiter::Error),
    UnhandledPeek(rjiter::jiter::Peek),
    UnbalancedJson(usize),
    InternalError(usize, String),
    MaxNestingExceeded(usize, usize),
    ActionError(Box<dyn std::error::Error>),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RJiterError(e) => write!(f, "RJiter error: {e}"),
            Error::ActionError(e) => write!(f, "Action error: {e}"),
            Error::UnhandledPeek(p) => write!(f, "UnhandledPeek: {p:?}"),
            Error::UnbalancedJson(pos) => write!(f, "Unbalanced JSON at position: {pos}"),
            Error::InternalError(pos, msg) => write!(f, "Internal error at position {pos}: {msg}"),
            Error::MaxNestingExceeded(pos, level) => write!(
                f,
                "Max nesting exceeded at position {pos} with level {level}"
            ),
        }
    }
}

impl From<rjiter::Error> for Error {
    fn from(error: rjiter::Error) -> Self {
        Error::RJiterError(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

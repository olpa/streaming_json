#[derive(Debug)]
pub enum Error {
    RJiterError(rjiter::Error),
    UnhandledPeek(rjiter::Peek),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RJiterError(e) => write!(f, e),
            Error::UnhandledPeek(p) => write!(f, "UnhandledPeek: {p:?}"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

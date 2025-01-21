use crate::rjiter::RJiter;
use jiter::{JiterError, JiterErrorType, JsonErrorType};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    JiterError(JiterError),
    IoError(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
    }
}

pub(crate) fn from_jiter_error(rjiter: &RJiter, jiter_error: JiterError) -> Error {
    let jiter_error = JiterError {
        error_type: jiter_error.error_type,
        index: jiter_error.index + rjiter.current_index(),
    };
    Error::JiterError(jiter_error)
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

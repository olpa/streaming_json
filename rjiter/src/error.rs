use jiter::{JiterError, JiterErrorType, JsonErrorType};
use crate::rjiter::RJiter;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    JiterError(JiterError),
    IoError(std::io::Error),
}

impl From<JiterError> for Error {
    fn from(err: JiterError) -> Self {
        Error::JiterError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err)
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

pub(crate) fn rewrite_error_index(rjiter: &RJiter, jiter_error: JiterError) -> Error {
    let jiter_error = JiterError {
        error_type: jiter_error.error_type,
        index: jiter_error.index + rjiter.current_index(),
    };
    return Error::JiterError(jiter_error);
}

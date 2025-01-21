use crate::rjiter::RJiter;
use jiter::{JiterError, JiterErrorType, JsonErrorType, LinePosition};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug]
pub enum ErrorType {
    JsonError(JsonErrorType),
    WrongType { expected: JsonType, actual: JsonType },
    IoError(std::io::Error),
}

#[derive(Debug)]
pub struct Error {
    pub error_type: ErrorType,
    pub index: usize,
}

impl Error {
    pub(crate) fn from_jiter_error(rjiter: &RJiter, jiter_error: JiterError) -> Error {
        Error {
            error_type: ErrorType::JiterError(jiter_error.error_type),
            index: jiter_error.index + rjiter.current_index(),
        }
    }

    pub(crate) fn from_io_error(rjiter: &RJiter, io_error: std::io::Error) -> Error {
        Error {
            error_type: ErrorType::IoError(io_error),
            index: rjiter.current_index(),
        }
    }

    pub fn get_position(&self, rjiter: &RJiter) -> LinePosition {
        return rjiter.error_position(0);
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

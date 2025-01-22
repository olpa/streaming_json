use crate::rjiter::RJiter;
use jiter::{JiterError, JiterErrorType, JsonErrorType, JsonType, LinePosition};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ErrorType {
    JsonError(JsonErrorType),
    WrongType {
        expected: JsonType,
        actual: JsonType,
    },
    IoError(std::io::Error),
}

#[derive(Debug)]
pub struct Error {
    pub error_type: ErrorType,
    pub index: usize,
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

    pub(crate) fn from_io_error(index: usize, io_error: std::io::Error) -> Error {
        Error {
            error_type: ErrorType::IoError(io_error),
            index,
        }
    }

    #[must_use]
    pub fn get_position(&self, rjiter: &RJiter) -> LinePosition {
        rjiter.error_position(self.index)
    }

    #[must_use]
    pub fn description(&self, rjiter: &RJiter) -> String {
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

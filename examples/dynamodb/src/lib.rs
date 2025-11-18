//! Streaming converter between `DynamoDB` JSON and normal JSON formats
//!
//! This library provides allocation-free, streaming conversion between standard
//! JSON and `DynamoDB`'s JSON format using the `scan_json` framework.

#![no_std]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod ddb_to_normal;
mod normal_to_ddb;

pub use ddb_to_normal::{convert_ddb_to_normal, ItemWrapperMode};
pub use normal_to_ddb::convert_normal_to_ddb;

/// Detailed error information for conversion errors
#[derive(Debug, Clone)]
pub enum ConversionError {
    /// `RJiter` error with context
    RJiterError {
        /// The type of `RJiter` error that occurred
        kind: rjiter::error::ErrorType,
        /// Byte position in the input where the error occurred
        position: usize,
        /// Description of what operation was being performed when the error occurred
        context: &'static str,
    },
    /// IO error with context
    IOError {
        /// The kind of IO error that occurred
        kind: embedded_io::ErrorKind,
        /// Byte position in the input where the error occurred
        position: usize,
        /// Description of what operation was being performed when the error occurred
        context: &'static str,
    },
    /// Parse error (invalid `DynamoDB` JSON format)
    ParseError {
        /// Byte position in the input where the error occurred
        position: usize,
        /// Description of what parsing operation failed
        context: &'static str,
        /// Unknown type descriptor bytes (buffer, actual length used)
        unknown_type: Option<([u8; 32], usize)>,
    },
    /// Scan error (from `scan_json` library)
    ScanError(scan_json::Error),
}

#[cfg(feature = "std")]
impl core::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConversionError::RJiterError {
                kind,
                position,
                context,
            } => {
                write!(
                    f,
                    "RJiter error at position {position}: {kind:?} (while {context})"
                )
            }
            ConversionError::IOError {
                kind,
                position,
                context,
            } => {
                write!(
                    f,
                    "IO error at position {position}: {kind:?} (while {context})"
                )
            }
            ConversionError::ParseError {
                position,
                context,
                unknown_type,
            } => {
                if let Some((bytes, len)) = unknown_type {
                    let type_str = std::string::String::from_utf8_lossy(bytes.get(..*len).unwrap_or(&[]));
                    write!(
                        f,
                        "Parse error at position {position}: {context} (unknown type descriptor '{type_str}')"
                    )
                } else {
                    write!(f, "Parse error at position {position}: {context}")
                }
            }
            ConversionError::ScanError(err) => {
                write!(f, "{err}")
            }
        }
    }
}

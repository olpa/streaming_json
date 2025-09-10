#![doc = include_str!("../README.md")]
#![no_std]

/// Buffer management for streaming JSON parsing.
pub mod buffer;
/// Error types and handling for `RJiter`.
pub mod error;
/// Streaming JSON parser implementation.
pub mod rjiter;

pub use error::Result;
pub use error::{Error, IoError};
pub use rjiter::RJiter;

pub use jiter;

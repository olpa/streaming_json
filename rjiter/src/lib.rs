#![doc = include_str!("../README.md")]
#![no_std]

pub mod buffer;
pub mod error;
pub mod rjiter;

pub use error::Result;
pub use error::{Error, IoError};
pub use rjiter::RJiter;

pub use jiter;

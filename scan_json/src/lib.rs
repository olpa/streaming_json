#![doc = include_str!("../README.md")]

pub mod error;
pub mod idtransform;
pub mod matcher;
pub mod scan;
pub mod stack;

pub use error::{Error, Result};
pub use idtransform::idtransform;
pub use matcher::{iter_match, Action, EndAction, StreamOp};
pub use scan::{scan, Options};

pub use rjiter;
pub use rjiter::jiter;
pub use rjiter::RJiter;

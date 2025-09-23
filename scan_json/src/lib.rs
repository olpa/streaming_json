#![doc = include_str!("../README.md")]

pub mod action;
pub mod error;
pub mod idtransform;
pub mod matcher;
pub mod scan;
pub mod stack;

pub use action::{BoxedAction, BoxedEndAction, StreamOp};
pub use error::{Error, Result};
pub use matcher::{
    debug_print_no_match, iter_match,
};
pub use scan::{scan, Options};
pub use idtransform::idtransform;

pub use rjiter;
pub use rjiter::jiter;
pub use rjiter::RJiter;

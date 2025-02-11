#![doc = include_str!("../README.md")]

pub mod action;
pub mod error;
pub mod matcher;
pub mod scan;

pub use action::{BoxedAction, BoxedEndAction, BoxedMatcher, StreamOp, Trigger};
pub use error::{Error, Result};
pub use matcher::{Matcher, Name, ParentAndName, ParentParentAndName};
pub use scan::{scan, ContextFrame};

pub use rjiter;
pub use rjiter::jiter;
pub use rjiter::RJiter;

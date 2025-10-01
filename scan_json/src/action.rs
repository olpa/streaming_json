//! Action module provides types and functionality for defining callbacks
use rjiter::RJiter;
use std::cell::RefCell;

/// Type alias for boxed action functions that can be called during JSON scanning
pub type BoxedAction<T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> StreamOp>;

/// Type alias for boxed end action functions that are called when a matched key ends
pub type BoxedEndAction<T> = Box<dyn Fn(&RefCell<T>) -> Result<(), Box<dyn std::error::Error>>>;

/// Interact from a callback to the `scan` function.
#[derive(Debug)]
pub enum StreamOp {
    /// Indicates no special action needs to be taken
    None,
    /// Indicates that the action advanced the `RJiter` parser, therefore `scan` should update its state
    ValueIsConsumed,
    /// An error
    Error(Box<dyn std::error::Error>),
}

impl<E: std::error::Error + 'static> From<E> for StreamOp {
    fn from(error: E) -> Self {
        StreamOp::Error(Box::new(error))
    }
}

// Actions can be:
// - Function pointers: fn(&RefCell<RJiter>, &RefCell<T>) -> StreamOp
// - Closures: impl Fn(&RefCell<RJiter>, &RefCell<T>) -> StreamOp
// - Structs with call operators
// - Any callable type

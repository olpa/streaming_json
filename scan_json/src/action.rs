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

/// Pair a matcher function with an action.
#[derive(Debug)]
pub struct Trigger<M, A> {
    /// The matcher function that determines when this trigger should activate
    pub matcher: M,
    /// The action to execute when the matcher succeeds
    pub action: A,
}

impl<M, A> Trigger<M, A> {
    #[must_use]
    /// Creates a new trigger with the given matcher function and action
    pub fn new(matcher: M, action: A) -> Self {
        Self { matcher, action }
    }
}

/// Finds the first matching action for a given key and context
///
/// # Arguments
/// * `triggers` - Slice of triggers to search through
/// * `for_key` - The key name as bytes to match against
/// * `context` - Iterator over parent context names as byte slices
///
/// # Returns
/// * `Option<&A>` - Reference to the matching action if found, None otherwise
#[must_use]
pub(crate) fn find_action<'triggers, 'context, M, A, I>(
    triggers: &'triggers [Trigger<M, A>],
    for_key: &[u8],
    context: I,
) -> Option<&'triggers A>
where
    M: Fn(&[u8], I) -> bool,
    I: Iterator<Item = &'context [u8]> + Clone,
{
    triggers
        .iter()
        .find(|trigger| {
            (trigger.matcher)(for_key, context.clone())
        })
        .map(|trigger| &trigger.action)
}

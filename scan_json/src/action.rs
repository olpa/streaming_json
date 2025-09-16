//! Action module provides types and functionality for defining callbacks
use crate::matcher::Matcher;
use crate::scan::ContextFrame;
use rjiter::RJiter;
use std::cell::RefCell;

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

/// Type alias for a boxed matcher trait object
pub type BoxedMatcher<'a> = Box<dyn Matcher + 'a>;

/// Type alias for a boxed action function
///
/// The action takes references to:
/// - An `RJiter` for iterating over the JSON stream
/// - A generic context `T` for maintaining state
///
/// Returns a `StreamOp`
#[allow(clippy::module_name_repetitions)]
pub type BoxedAction<'a, T> = Box<dyn Fn(&RefCell<RJiter>, &RefCell<T>) -> StreamOp + 'a>;

/// Type alias for a boxed action function that is called when a matching key is ended
///
/// The end action takes a reference to:
/// - A generic context `T` for maintaining state
///
/// Returns a `Result`
#[allow(clippy::module_name_repetitions)]
pub type BoxedEndAction<'a, T> =
    Box<dyn Fn(&RefCell<T>) -> std::result::Result<(), Box<dyn std::error::Error>> + 'a>;

/// Pair a matcher with an action.
#[derive(Debug)]
pub struct Trigger<'a, BoxedActionT> {
    /// The matcher that determines when this trigger should activate
    pub matcher: BoxedMatcher<'a>,
    /// The action to execute when the matcher succeeds
    pub action: BoxedActionT,
}

impl<'a, BoxedActionT> Trigger<'a, BoxedActionT> {
    #[must_use]
    /// Creates a new trigger with the given matcher and action
    pub fn new(matcher: BoxedMatcher<'a>, action: BoxedActionT) -> Self {
        Self { matcher, action }
    }
}

/// Finds the first matching action for a given key and context
///
/// # Arguments
/// * `triggers` - Slice of triggers to search through
/// * `for_key` - The key to match against
/// * `context` - The current context frames. The oldest frame (the root) is the first element, the latest frame (the parent) is the last element.
///
/// # Returns
/// * `Option<&BoxedActionT>` - Reference to the matching action if found, None otherwise
#[must_use]
pub(crate) fn find_action<'a, BoxedActionT>(
    triggers: &'a [Trigger<BoxedActionT>],
    for_key: &str,
    context: &[ContextFrame],
) -> Option<&'a BoxedActionT> {
    triggers
        .iter()
        .find(|trigger| trigger.matcher.matches(for_key, context))
        .map(|trigger| &trigger.action)
}

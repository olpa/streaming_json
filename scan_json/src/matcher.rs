//! This module contains functions for matching JSON nodes based on their name and context.

use crate::stack::ContextIter;
use rjiter::RJiter;

/// Represents structural pseudo-names for JSON nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralPseudoname {
    /// The beginning or end of an array element
    Array,
    /// The beginning or end of an object element
    Object,
    /// Anything that is not an array or object (primitives like strings, numbers, booleans, null)
    Atom,
    /// An object key, with its name in `path` (see [`iter_match`] function)
    None,
}

/// Return value from a callback to the `scan` function.
#[derive(Debug)]
pub enum StreamOp {
    /// Indicates that the action did not advance the `RJiter` parser (it may have peeked at the next token without consuming it),
    /// therefore `scan` can work further as if the action was not called at all.
    None,
    /// Indicates that the action advanced the `RJiter` parser, therefore `scan` should update its state:
    /// - Inside an object, the action consumed the key value, therefore the next event should be a key or an end-object
    /// - Inside an array, the action consumed the item, the next event should be a value or an end-array
    /// - At the top level, the action consumed the item, the next event should be a value or end-of-input
    ValueIsConsumed,
    /// An error with a static error message
    Error(&'static str),
}

/// Type alias for action functions that can be called during JSON scanning.
///
/// The type parameter `B` represents the baton (state) type:
/// - For simple batons: `B` is a `Copy` type like `i32`, `bool`, `()`
/// - For mutable state: `B` is `&RefCell<SomeType>` for shared mutable access
pub type Action<B, R> = fn(&mut RJiter<R>, B) -> StreamOp;

/// Type alias for end action functions that are called when a matched key ends.
///
/// The type parameter `B` represents the baton (state) type:
/// - For simple batons: `B` is a `Copy` type like `i32`, `bool`, `()`
/// - For mutable state: `B` is `&RefCell<SomeType>` for shared mutable access
///
/// Returns `Ok(())` on success, or `Err(message)` where `message` is a static error message.
pub type EndAction<B> = fn(B) -> Result<(), &'static str>;

/// Match by name and ancestor names against the current JSON context.
///
/// Additionally, the structural events (begin/end of array/object, primitive values in array/on top)
/// can be matched via structural pseudo-names.
///
/// # Arguments
///
/// * `iter_creator` - A sequence of names to match, as a function that returns an iterator
/// * `structural_pseudoname` - A structural event, a part of the json context
/// * `path` - The json context
///
/// # Details
///
/// In the name-iterator, the first name is the name to match, the second name is
/// its expected parent name, the third name is the expected grandparent name, and so on.
///
/// In the context-iterator, the first element is the most recent name, the second element is
/// the parent name, the third element is the grandparent name, and so on.
///
/// To return `true` (to match), the whole name-iterator should be consumed, and the names
/// should match context names.
///
/// An empty name-iterator always returns true (matches everything).
///
/// # Pseudo names
///
/// There are pseudo names that can appear in `path`:
///
/// - `#top` - The top level context. Always present as the last element in `path`
/// - `#array`
///
/// As a performance optimization, the structural events are not included in `path`,
/// and if there is a structural event, it is passed as a separate argument.
///
/// To match a structural event, the name-iterator should start with a structural pseudo-name:
///
/// - `#object` - Beginning or end of an object, matches `StructuralPseudoname::Object`
/// - `#array` - Beginning or end of an array, matches `StructuralPseudoname::Array`
/// - `#atom` - A primitive value in an array or at the top level, matches `StructuralPseudoname::Atom`
///
/// # Returns
///
/// * `true` if the node matches the criteria
/// * `false` otherwise
pub fn iter_match<F, T, Item>(
    iter_creator: F,
    structural_pseudoname: StructuralPseudoname,
    mut path: ContextIter,
) -> bool
where
    F: Fn() -> T,
    T: IntoIterator<Item = Item>,
    Item: AsRef<[u8]>,
{
    let mut expected = iter_creator().into_iter();

    // Handle structural pseudo-names
    match structural_pseudoname {
        StructuralPseudoname::Array => {
            match expected.next() {
                Some(expected_name) if expected_name.as_ref() == b"#array" => {}
                Some(_) => return false,
                None => return true, // Empty match-iterator always returns true
            }
        }
        StructuralPseudoname::Object => {
            match expected.next() {
                Some(expected_name) if expected_name.as_ref() == b"#object" => {}
                Some(_) => return false,
                None => return true, // Empty match-iterator always returns true
            }
        }
        StructuralPseudoname::Atom => {
            match expected.next() {
                Some(expected_name) if expected_name.as_ref() == b"#atom" => {}
                Some(_) => return false,
                None => return true, // Empty match-iterator always returns true
            }
        }
        StructuralPseudoname::None => {}
    }

    // Compare each path element with expected elements
    for expected_context in expected {
        match path.next() {
            Some(actual_context) if expected_context.as_ref() == actual_context => {}
            _ => return false,
        }
    }

    // Extra path elements are allowed - no need to check for them
    true
}

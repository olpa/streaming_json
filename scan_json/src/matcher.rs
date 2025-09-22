//! This module contains functions for matching JSON nodes based on their name and context.

/// Determines if a node matches against a sequence of names using an iterator-based approach.
///
/// To return true (to match), the whole iterator should be consumed, and the names
/// should match with the current key name and its context. The first element from
/// the iterator is compared with the current key name, the second element with the
/// parent name, the third element with the grandparent name, and so on.
///
/// An empty iterator always returns true (matches everything).
///
/// # Arguments
///
/// * `iter_creator` - A function that creates an iterator over the expected sequence to match against
/// * `name` - A reference to a u8 slice representing the name of the current node being matched
/// * `context` - An iterator over references to u8 slices, where the first element is the parent name,
///   the second element is the grandparent name, etc.
///
/// Special names:
/// - `#top` - The top level context
/// - `#array` - An array
/// - `#object` - An unnamed object inside an array or at the top level
/// - `#atom` - Anything what is not an object or array
///
/// # Returns
///
/// * `true` if the node matches the criteria
/// * `false` otherwise
pub fn iter_match<'a, F, T, Item, I>(iter_creator: F, name: &[u8], mut context: I) -> bool
where
    F: Fn() -> T,
    T: IntoIterator<Item = Item>,
    Item: AsRef<[u8]>,
    I: Iterator<Item = &'a [u8]>,
{
    let mut expected = iter_creator().into_iter();

    // First compare the name
    if let Some(expected_name) = expected.next() {
        if expected_name.as_ref() != name {
            return false;
        }
    } else {
        // Empty iterator always returns true
        return true;
    }

    // Then compare each context element
    for expected_context in expected {
        if let Some(actual_context) = context.next() {
            if expected_context.as_ref() != actual_context {
                return false;
            }
        } else {
            return false;
        }
    }

    // Extra context elements are allowed - no need to check for them
    true
}

/// Prints the context and name of the node being matched and always returns false.
///
/// This function is useful for debugging to see what nodes are being processed.
///
/// # Arguments
///
/// * `name` - A reference to a u8 slice representing the name of the current node being matched
/// * `context` - An iterator over references to u8 slices, where the first element is the parent name,
///   the second element is the grandparent name, etc.
///
/// # Returns
///
/// * Always returns `false`
pub fn debug_print_no_match<'a, I>(name: &[u8], context: I) -> bool
where
    I: Iterator<Item = &'a [u8]>,
{
    println!("debug_print_no_match: name: {:?}", name);
    for (i, ctx) in context.enumerate() {
        println!("  context[{}]: {:?}", i, ctx);
    }
    false
}

//! This module contains functions for matching JSON nodes based on their name and context.

use crate::stack::ContextIter;

/// Represents structural pseudo-names for JSON nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralPseudoname {
    /// A begin or an end of an array element
    Array,
    /// A begin or an end of an object element
    Object,
    /// Anything that is not an array or object (primitives like strings, numbers, booleans, null)
    Atom,
    /// An object key, with its name in `path` (see `iter_match` function)
    None,
}

/// Determines if a node matches against a sequence of names using an iterator-based approach.
///
/// To return true (to match), the whole iterator should be consumed, and the names
/// should match with the path elements. The first element from the iterator is compared
/// with the most recent path element, the second element with the parent name,
/// the third element with the grandparent name, and so on.
///
/// An empty iterator always returns true (matches everything).
///
/// As a performance optimization, the structural pseudo-names are passed not in `path`,
/// but as a separate argument. Logically, they could be part of the iterator as well.
///
/// # Arguments
///
/// * `iter_creator` - A function that creates an iterator over the expected sequence to match against
/// * `structural_pseudoname` - The structural type of the current node
/// * `path` - An iterator over references to u8 slices, where the first element is the most recent name,
///   the second element is the parent name, etc.
///
/// Special names that can appear in `path`:
/// - `#top` - The top level context. Always present as the last element in `path`
/// - `#array` - An array context
///
/// Structural names that can be matched via `iter_creator` iterator:
/// - `#object` - Matches an object element (when `structural_pseudoname` is `Object`)
/// - `#atom` - Matches a primitive value (when `structural_pseudoname` is `Atom`)
/// - `#array` - Matches an array element (when `structural_pseudoname` is `Array`)
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

/// Prints the structural pseudo-name and path of the node being matched and always returns false.
///
/// This function is useful for debugging to see what nodes are being processed.
///
/// # Arguments
///
/// Same as in `iter_match`.
///
/// # Returns
///
/// * Always returns `false`
#[must_use]
pub fn debug_print_no_match(
    structural_pseudoname: StructuralPseudoname,
    path: ContextIter,
) -> bool {
    println!("debug_print_no_match: structural_pseudoname: {structural_pseudoname:?}");
    for (i, ctx) in path.enumerate() {
        println!("  path[{i}]: {ctx:?}");
    }
    false
}

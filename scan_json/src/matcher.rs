//! This module contains the `Matcher` trait and implementations for matching by name
//! and a general iterator-based matcher. There is also a debug-matcher to print the context
//! and name of the node being matched.

/// Defines the interface for matching JSON nodes based on their name and context.
pub trait Matcher: std::fmt::Debug {
    /// Determines if a node matches specific criteria.
    ///
    /// # Arguments
    ///
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
    fn matches<'a, I>(&self, name: &[u8], context: I) -> bool
    where
        I: Iterator<Item = &'a [u8]>;
}

/// A general iterator-based matcher that can match against a sequence of names.
/// The iterator creator provides the sequence to match against.
///
/// To return true (to match), the whole iterator should be consumed, and the names
/// should match with the current key name and its context. The first element from
/// the iterator is compared with the current key name, the second element with the
/// parent name, the third element with the grandparent name, and so on.
///
/// An empty iterator always returns true (matches everything).
pub struct IterMatcher<F> {
    iter_creator: F,
}

impl<F> IterMatcher<F> {
    #[must_use]
    /// Creates a new iterator-based matcher
    pub fn new(iter_creator: F) -> Self {
        Self { iter_creator }
    }
}

impl<F> core::fmt::Debug for IterMatcher<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IterMatcher").finish()
    }
}

impl<F, T, Item> Matcher for IterMatcher<F>
where
    F: Fn() -> T,
    T: IntoIterator<Item = Item>,
    Item: AsRef<[u8]>,
{
    fn matches<'a, I>(&self, name: &[u8], mut context: I) -> bool
    where
        I: Iterator<Item = &'a [u8]>,
    {
        let mut expected = (self.iter_creator)().into_iter();

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

        // Ensure no extra context elements
        context.next().is_none()
    }
}

/// A matcher that prints the context and name of the node being matched.
#[derive(Debug)]
pub struct DebugPrinter;

impl Matcher for DebugPrinter {
    fn matches<'a, I>(&self, name: &[u8], context: I) -> bool
    where
        I: Iterator<Item = &'a [u8]>,
    {
        println!("DebugPrinter::matches: name: {:?}", name);
        for (i, ctx) in context.enumerate() {
            println!("  context[{}]: {:?}", i, ctx);
        }
        false
    }
}

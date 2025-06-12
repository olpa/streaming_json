//! This module contains the `Matcher` trait and implementations for matching by name,
//! matching by parent-name combination, and matching by grandparent-parent-name combination.
//! There is also a debug-matcher to print the context and name of the node being matched.

use crate::scan::ContextFrame;

/// Defines the interface for matching JSON nodes based on their name and context.
pub trait Matcher: std::fmt::Debug {
    /// Determines if a node matches specific criteria.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the current node being matched
    /// * `context` - The stack of parent contexts. The oldest frame (the root) is the first element,
    ///               the latest frame (the parent) is the last element.
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
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool;
}

/// A matcher that checks for exact name matches.
#[derive(Debug)]
pub struct Name {
    name: String,
}

impl Name {
    #[must_use]
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl Matcher for Name {
    fn matches(&self, name: &str, _context: &[ContextFrame]) -> bool {
        self.name == name
    }
}

/// A matcher that checks for both parent and name matches.
#[derive(Debug)]
pub struct ParentAndName {
    parent: String,
    name: String,
}

impl ParentAndName {
    #[must_use]
    pub fn new(parent: String, name: String) -> Self {
        Self { parent, name }
    }
}

impl Matcher for ParentAndName {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        context.last().map_or(false, |parent| {
            self.name == name && parent.current_key == self.parent
        })
    }
}
/// A matcher that checks for grandparent, parent and name matches.
#[derive(Debug)]
pub struct ParentParentAndName {
    grandparent: String,
    parent: String,
    name: String,
}

impl ParentParentAndName {
    #[must_use]
    pub fn new(grandparent: String, parent: String, name: String) -> Self {
        Self {
            grandparent,
            parent,
            name,
        }
    }
}

impl Matcher for ParentParentAndName {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        if context.len() < 2 {
            return false;
        }
        #[allow(clippy::indexing_slicing)]
        let parent = &context[context.len() - 1];
        #[allow(clippy::indexing_slicing)]
        let grandparent = &context[context.len() - 2];
        self.name == name
            && parent.current_key == self.parent
            && grandparent.current_key == self.grandparent
    }
}

/// A matcher that prints the context and name of the node being matched.
#[derive(Debug)]
pub struct DebugPrinter;

impl Matcher for DebugPrinter {
    fn matches(&self, name: &str, context: &[ContextFrame]) -> bool {
        println!("DebugPrinter::matches: name: {name:?} context: {context:?}");
        false
    }
}

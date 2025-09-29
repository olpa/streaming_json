//! Stack management for JSON parsing context
//!
//! Contains the StateFrame definition and ContextIter wrapper for navigating
//! the parsing context stack.

use u8pool::{U8Pool, U8PoolAssocRevIter};

/// Metadata associated with each context frame in the `U8Pool` stack
#[derive(Debug, Clone, Copy)]
pub struct StateFrame {
    pub is_in_object: bool,
    pub is_in_array: bool,
    pub is_elem_begin: bool,
}

/// Wrapper around the U8Pool associated iterator for context iteration
/// Provides a convenient interface with syntactic sugar for for-loops and .next()
pub struct ContextIter<'a> {
    inner: U8PoolAssocRevIter<'a, StateFrame>,
}

impl<'a> ContextIter<'a> {
    pub fn new(pool: &'a U8Pool) -> Self {
        Self { inner: pool.iter_assoc_rev::<StateFrame>() }
    }
}

impl<'a> Iterator for ContextIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_assoc, key_slice)| key_slice)
    }
}

impl<'a> Clone for ContextIter<'a> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
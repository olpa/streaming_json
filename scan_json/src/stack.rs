//! Stack management for JSON parsing context

use crate::scan::StructurePosition;
use u8pool::{U8Pool, U8PoolAssocRevIter};

/// Wrapper around the `U8Pool` associated iterator for context iteration
/// Provides a convenient interface with syntactic sugar for for-loops and `.next()`
pub struct ContextIter<'a> {
    inner: U8PoolAssocRevIter<'a, StructurePosition>,
}

impl<'a> ContextIter<'a> {
    /// Creates a new `ContextIter` from a `U8Pool` reference
    #[must_use]
    pub fn new(pool: &'a U8Pool) -> Self {
        Self {
            inner: pool.iter_assoc_rev::<StructurePosition>(),
        }
    }
}

impl<'a> Iterator for ContextIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_assoc, key_slice)| key_slice)
    }
}

impl Clone for ContextIter<'_> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

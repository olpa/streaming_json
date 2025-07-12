//! `BufVec`: A zero-allocation vector implementation using client-provided buffers.
//!
//! `BufVec` provides vector, stack, and dictionary interfaces while using a single
//! client-provided buffer for storage. All operations are bounds-checked and
//! no internal allocations are performed.
//!
//! Buffer layout: [metadata section][data section]
//! Metadata section stores slice descriptors as (`start_offset`, length) pairs.
//!
//! # Dictionary Convention
//!
//! `BufVec` supports a dictionary interpretation where elements at even indices (0, 2, 4, ...)
//! are treated as keys and elements at odd indices (1, 3, 5, ...) are treated as values.
//! This allows the same data structure to be used as both a vector and a key-value store.
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Add key-value pairs
//! bufvec.add(b"name").unwrap();      // key at index 0
//! bufvec.add(b"Alice").unwrap();     // value at index 1
//! bufvec.add(b"age").unwrap();       // key at index 2
//! bufvec.add(b"30").unwrap();        // value at index 3
//!
//! // Use dictionary interface
//! for (key, value) in bufvec.pairs() {
//!     match value {
//!         Some(v) => println!("{:?} = {:?}", key, v),
//!         None => println!("{:?} = <no value>", key),
//!     }
//! }
//!
//! // Check for unpaired keys
//! if bufvec.has_unpaired_key() {
//!     println!("Last element is an unpaired key");
//! }
//! ```
//!
//! # Iterator Support
//!
//! `BufVec` implements standard Rust iterator patterns:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! bufvec.add(b"hello").unwrap();
//! bufvec.add(b"world").unwrap();
//!
//! // Iterate using for loop
//! for slice in &bufvec {
//!     println!("{:?}", slice);
//! }
//!
//! // Collect into Vec
//! let collected: Vec<_> = bufvec.into_iter().collect();
//! ```

use std::fmt;

#[derive(Debug)]
pub enum BufVecError {
    BufferOverflow,
    IndexOutOfBounds,
    EmptyVector,
    BufferTooSmall,
}

impl fmt::Display for BufVecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufVecError::BufferOverflow => write!(f, "Buffer overflow: insufficient space"),
            BufVecError::IndexOutOfBounds => write!(f, "Index out of bounds"),
            BufVecError::EmptyVector => write!(f, "Operation on empty vector"),
            BufVecError::BufferTooSmall => write!(f, "Buffer too small for metadata"),
        }
    }
}

impl std::error::Error for BufVecError {}

const SLICE_DESCRIPTOR_SIZE: usize = 16; // 2 * size_of::<usize>() on 64-bit
const DEFAULT_MAX_SLICES: usize = 8;

pub struct BufVec<'a> {
    buffer: &'a mut [u8],
    count: usize,
    max_slices: usize,
}

impl<'a> BufVec<'a> {
    /// Creates a new `BufVec` with the specified maximum number of slices.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::BufferTooSmall` if:
    /// - `max_slices` is 0
    /// - The buffer is too small to hold the metadata for `max_slices`
    pub fn new(buffer: &'a mut [u8], max_slices: usize) -> Result<Self, BufVecError> {
        if max_slices == 0 {
            return Err(BufVecError::BufferTooSmall);
        }

        let metadata_space = max_slices * SLICE_DESCRIPTOR_SIZE;

        if buffer.len() <= metadata_space {
            return Err(BufVecError::BufferTooSmall);
        }

        Ok(Self {
            buffer,
            count: 0,
            max_slices,
        })
    }

    fn data_start(&self) -> usize {
        self.max_slices * SLICE_DESCRIPTOR_SIZE
    }

    fn data_used(&self) -> usize {
        if self.count == 0 {
            return 0;
        }

        // Calculate data used by finding the highest end position
        let mut max_end = self.data_start();
        for i in 0..self.count {
            let (slice_start, slice_length) = self.get_slice_descriptor(i);
            max_end = max_end.max(slice_start + slice_length);
        }
        max_end - self.data_start()
    }

    /// Creates a new `BufVec` with the default maximum number of slices (8).
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::BufferTooSmall` if the buffer is too small.
    pub fn with_default_max_slices(buffer: &'a mut [u8]) -> Result<Self, BufVecError> {
        Self::new(buffer, DEFAULT_MAX_SLICES)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[must_use]
    pub fn buffer_capacity(&self) -> usize {
        self.buffer.len()
    }

    #[must_use]
    pub fn used_bytes(&self) -> usize {
        self.data_start() + self.data_used()
    }

    #[must_use]
    pub fn available_bytes(&self) -> usize {
        self.buffer.len() - self.used_bytes()
    }

    #[must_use]
    pub fn max_slices(&self) -> usize {
        self.max_slices
    }

    fn check_bounds(&self, index: usize) -> Result<(), BufVecError> {
        if index >= self.count {
            Err(BufVecError::IndexOutOfBounds)
        } else {
            Ok(())
        }
    }

    fn ensure_capacity(&self, additional_bytes: usize) -> Result<(), BufVecError> {
        // Check if we've reached the maximum number of slices
        if self.count >= self.max_slices {
            return Err(BufVecError::BufferOverflow);
        }

        // Check if we have enough space for the additional bytes
        if self.data_used() + additional_bytes > self.buffer.len() - self.data_start() {
            return Err(BufVecError::BufferOverflow);
        }
        Ok(())
    }

    #[allow(clippy::expect_used)]
    fn get_slice_descriptor(&self, index: usize) -> (usize, usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        let start_bytes = self.buffer.get(offset..offset + 8)
            .expect("Buffer bounds checked during construction");
        let length_bytes = self.buffer.get(offset + 8..offset + 16)
            .expect("Buffer bounds checked during construction");

        let start = usize::from_le_bytes(start_bytes.try_into()
            .expect("Slice is exactly 8 bytes"));
        let length = usize::from_le_bytes(length_bytes.try_into()
            .expect("Slice is exactly 8 bytes"));

        (start, length)
    }

    #[allow(clippy::expect_used)]
    fn set_slice_descriptor(&mut self, index: usize, start: usize, length: usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        self.buffer.get_mut(offset..offset + 8)
            .expect("Buffer bounds checked during construction")
            .copy_from_slice(&start.to_le_bytes());
        self.buffer.get_mut(offset + 8..offset + 16)
            .expect("Buffer bounds checked during construction")
            .copy_from_slice(&length.to_le_bytes());
    }

    /// Gets a slice at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn get(&self, index: usize) -> &[u8] {
        assert!(
            index < self.count,
            "Index {} out of bounds for vector of length {}",
            index,
            self.count
        );
        let (start, length) = self.get_slice_descriptor(index);
        self.buffer.get(start..start + length)
            .expect("Slice bounds validated during add operation")
    }

    /// Tries to get a slice at the specified index.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::IndexOutOfBounds` if `index` is out of bounds.
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[allow(clippy::expect_used)]
    pub fn try_get(&self, index: usize) -> Result<&[u8], BufVecError> {
        self.check_bounds(index)?;
        let (start, length) = self.get_slice_descriptor(index);
        Ok(self.buffer.get(start..start + length)
            .expect("Slice bounds validated during add operation"))
    }

    /// Adds a slice to the vector.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the data
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[allow(clippy::expect_used)]
    pub fn add(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        self.ensure_capacity(data.len())?;

        let start = self.data_start() + self.data_used();
        let end = start + data.len();

        self.buffer.get_mut(start..end)
            .expect("Buffer capacity checked by ensure_capacity")
            .copy_from_slice(data);
        self.set_slice_descriptor(self.count, start, data.len());
        self.count += 1;

        Ok(())
    }

    pub fn clear(&mut self) {
        self.count = 0;
        // data_used is now derived from slice descriptors, so no need to reset it
    }

    /// Removes and returns the last slice from the vector.
    ///
    /// # Panics
    ///
    /// Panics if the vector is empty or if buffer integrity is compromised.
    #[allow(clippy::expect_used)]
    pub fn pop(&mut self) -> &[u8] {
        assert!(self.count > 0, "Cannot pop from empty vector");

        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);

        // data_used is now automatically recalculated when needed
        self.buffer.get(start..start + length)
            .expect("Slice bounds validated during add operation")
    }

    /// Tries to remove and return the last slice from the vector.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::EmptyVector` if the vector is empty.
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[allow(clippy::expect_used)]
    pub fn try_pop(&mut self) -> Result<&[u8], BufVecError> {
        if self.count == 0 {
            return Err(BufVecError::EmptyVector);
        }

        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);

        // data_used is now automatically recalculated when needed
        Ok(self.buffer.get(start..start + length)
            .expect("Slice bounds validated during add operation"))
    }

    /// Returns an iterator over the slices in the vector.
    #[must_use]
    pub fn iter(&self) -> BufVecIter<'_> {
        self.into_iter()
    }

    /// Returns true if the index represents a key (even indices).
    #[must_use]
    pub fn is_key(&self, index: usize) -> bool {
        index % 2 == 0
    }

    /// Returns true if the index represents a value (odd indices).
    #[must_use]
    pub fn is_value(&self, index: usize) -> bool {
        index % 2 == 1
    }

    /// Returns true if the last element is an unpaired key (odd number of elements).
    #[must_use]
    pub fn has_unpaired_key(&self) -> bool {
        self.count % 2 == 1
    }

    /// Returns the number of complete key-value pairs.
    #[must_use]
    pub fn pairs_count(&self) -> usize {
        self.count / 2
    }

    /// Returns an iterator over key-value pairs.
    #[must_use]
    pub fn pairs(&self) -> BufVecPairIter<'_> {
        BufVecPairIter {
            bufvec: self,
            current_pair: 0,
        }
    }
}

pub struct BufVecIter<'a> {
    bufvec: &'a BufVec<'a>,
    current: usize,
}

impl<'a> Iterator for BufVecIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.bufvec.len() {
            let result = self.bufvec.get(self.current);
            self.current += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.bufvec.len() - self.current;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for BufVecIter<'a> {}

impl<'a> IntoIterator for &'a BufVec<'a> {
    type Item = &'a [u8];
    type IntoIter = BufVecIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BufVecIter {
            bufvec: self,
            current: 0,
        }
    }
}

pub struct BufVecPairIter<'a> {
    bufvec: &'a BufVec<'a>,
    current_pair: usize,
}

impl<'a> Iterator for BufVecPairIter<'a> {
    type Item = (&'a [u8], Option<&'a [u8]>);

    fn next(&mut self) -> Option<Self::Item> {
        let key_index = self.current_pair * 2;

        if key_index >= self.bufvec.len() {
            return None;
        }

        let key = self.bufvec.get(key_index);
        let value = if key_index + 1 < self.bufvec.len() {
            Some(self.bufvec.get(key_index + 1))
        } else {
            None
        };

        self.current_pair += 1;
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_pairs = if self.bufvec.is_empty() {
            0
        } else {
            ((self.bufvec.len() + 1) / 2) - self.current_pair
        };
        (remaining_pairs, Some(remaining_pairs))
    }
}

impl<'a> ExactSizeIterator for BufVecPairIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_initialization() {
        let mut buffer = [0u8; 200];
        let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());
        assert_eq!(bufvec.buffer_capacity(), 200);
        assert_eq!(bufvec.max_slices(), 8);
        assert_eq!(bufvec.used_bytes(), 128); // metadata section takes 128 bytes (8 slices * 16 bytes)
        assert!(bufvec.available_bytes() > 0);
    }

    #[test]
    fn test_bounds_checking_empty_buffer() {
        let mut buffer = [0u8; 0];
        assert!(BufVec::with_default_max_slices(&mut buffer).is_err());

        let mut buffer = [0u8; 200];
        let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        assert!(bufvec.try_get(0).is_err());
    }

    #[test]
    #[should_panic(expected = "Cannot pop from empty vector")]
    fn test_pop_empty_vector() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
        bufvec.pop(); // Should panic
    }

    #[test]
    fn test_memory_layout_integrity() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();

        assert_eq!(bufvec.get(0), b"hello");
        assert_eq!(bufvec.get(1), b"world");
        assert_eq!(bufvec.len(), 2);
    }

    #[test]
    fn test_no_internal_allocation() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"test").unwrap();

        // Verify data is stored correctly in the buffer
        assert_eq!(bufvec.get(0), b"test");
        assert_eq!(bufvec.len(), 1);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Fill up the buffer with data
        assert!(bufvec.add(b"hello").is_ok());
        assert!(bufvec.add(b"world").is_ok());

        // Try to add more data than fits in the remaining space
        assert!(bufvec
            .add(b"this_is_a_very_long_string_that_should_not_fit_in_the_remaining_space")
            .is_err());
    }

    #[test]
    fn test_bounds_checking() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"test").unwrap();

        assert_eq!(bufvec.get(0), b"test");
        assert!(bufvec.try_get(1).is_err());
    }

    #[test]
    #[should_panic(expected = "Index 1 out of bounds for vector of length 1")]
    fn test_get_out_of_bounds() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"test").unwrap();
        let _ = bufvec.get(1); // Should panic
    }

    #[test]
    fn test_clear_operation() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();

        assert_eq!(bufvec.len(), 2);

        bufvec.clear();

        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());
    }

    #[test]
    fn test_pop_operation() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();

        let popped = bufvec.pop();
        assert_eq!(popped, b"world");
        assert_eq!(bufvec.len(), 1);

        let popped = bufvec.pop();
        assert_eq!(popped, b"hello");
        assert_eq!(bufvec.len(), 0);

        assert!(bufvec.try_pop().is_err());
    }

    #[test]
    fn test_custom_max_slices() {
        let mut buffer = [0u8; 100];
        let mut bufvec = BufVec::new(&mut buffer, 3).unwrap();

        bufvec.add(b"test").unwrap();
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();

        // Should fail on 4th slice
        assert!(bufvec.add(b"fail").is_err());

        assert_eq!(bufvec.get(0), b"test");
        assert_eq!(bufvec.get(1), b"hello");
        assert_eq!(bufvec.get(2), b"world");
        assert_eq!(bufvec.len(), 3);
        assert_eq!(bufvec.max_slices(), 3);
    }

    #[test]
    fn test_fixed_descriptor_functionality() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Test that derived values work correctly
        assert_eq!(bufvec.max_slices(), 8);
        assert_eq!(bufvec.data_used(), 0);

        bufvec.add(b"test").unwrap();
        assert_eq!(bufvec.data_used(), 4);

        bufvec.add(b"hello").unwrap();
        assert_eq!(bufvec.data_used(), 9);

        bufvec.pop();
        assert_eq!(bufvec.data_used(), 4);

        bufvec.clear();
        assert_eq!(bufvec.data_used(), 0);
    }

    #[test]
    fn test_iterator_empty_vector() {
        let mut buffer = [0u8; 200];
        let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        let mut iter = bufvec.into_iter();
        assert_eq!(iter.next(), None);
        assert_eq!(iter.size_hint(), (0, Some(0)));
    }

    #[test]
    fn test_iterator_populated_vector() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        bufvec.add(b"test").unwrap();

        let mut iter = bufvec.into_iter();
        assert_eq!(iter.size_hint(), (3, Some(3)));

        assert_eq!(iter.next(), Some(&b"hello"[..]));
        assert_eq!(iter.size_hint(), (2, Some(2)));

        assert_eq!(iter.next(), Some(&b"world"[..]));
        assert_eq!(iter.size_hint(), (1, Some(1)));

        assert_eq!(iter.next(), Some(&b"test"[..]));
        assert_eq!(iter.size_hint(), (0, Some(0)));

        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iterator_consumed_completely() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"a").unwrap();
        bufvec.add(b"b").unwrap();

        let collected: Vec<_> = bufvec.into_iter().collect();
        assert_eq!(collected, vec![&b"a"[..], &b"b"[..]]);
    }

    #[test]
    fn test_iterator_partial_iteration() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"first").unwrap();
        bufvec.add(b"second").unwrap();
        bufvec.add(b"third").unwrap();

        let mut iter = bufvec.into_iter();
        assert_eq!(iter.next(), Some(&b"first"[..]));
        assert_eq!(iter.next(), Some(&b"second"[..]));
        // Iterator should still work after partial consumption
        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert_eq!(iter.next(), Some(&b"third"[..]));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_iterator_lifetime_correctness() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"data").unwrap();

        // Test that iterator can be created and used
        {
            let iter = bufvec.into_iter();
            let first = iter.take(1).next().unwrap();
            assert_eq!(first, b"data");
        }

        // BufVec should still be usable after iterator is dropped
        assert_eq!(bufvec.len(), 1);
        assert_eq!(bufvec.get(0), b"data");
    }

    #[test]
    fn test_for_loop_syntax() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();

        let mut results = Vec::new();
        for slice in &bufvec {
            results.push(slice);
        }

        assert_eq!(results, vec![&b"hello"[..], &b"world"[..]]);
    }

    #[test]
    fn test_iter_method() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();

        let collected: Vec<_> = bufvec.iter().collect();
        assert_eq!(collected, vec![&b"hello"[..], &b"world"[..]]);
    }

    #[test]
    fn test_dictionary_helper_methods() {
        let mut buffer = [0u8; 200];
        let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        assert!(bufvec.is_key(0));
        assert!(!bufvec.is_value(0));
        assert!(bufvec.is_key(2));
        assert!(bufvec.is_key(4));

        assert!(bufvec.is_value(1));
        assert!(!bufvec.is_key(1));
        assert!(bufvec.is_value(3));
        assert!(bufvec.is_value(5));
    }

    #[test]
    fn test_key_value_pairing_even_elements() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();
        bufvec.add(b"value2").unwrap();

        assert_eq!(bufvec.len(), 4);
        assert!(!bufvec.has_unpaired_key());
        assert_eq!(bufvec.pairs_count(), 2);

        assert!(bufvec.is_key(0));
        assert!(bufvec.is_value(1));
        assert!(bufvec.is_key(2));
        assert!(bufvec.is_value(3));

        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"key2");
        assert_eq!(bufvec.get(3), b"value2");
    }

    #[test]
    fn test_unpaired_key_handling_odd_elements() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();

        assert_eq!(bufvec.len(), 3);
        assert!(bufvec.has_unpaired_key());
        assert_eq!(bufvec.pairs_count(), 1);

        assert!(bufvec.is_key(0));
        assert!(bufvec.is_value(1));
        assert!(bufvec.is_key(2));

        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"key2");
    }

    #[test]
    fn test_dictionary_iterator_even_elements() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();
        bufvec.add(b"value2").unwrap();

        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
        assert_eq!(pairs[1], (&b"key2"[..], Some(&b"value2"[..])));
    }

    #[test]
    fn test_dictionary_iterator_odd_elements() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();

        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
        assert_eq!(pairs[1], (&b"key2"[..], None));
    }

    #[test]
    fn test_dictionary_iterator_empty() {
        let mut buffer = [0u8; 200];
        let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs.len(), 0);
    }

    #[test]
    fn test_dictionary_iterator_single_key() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"lonely_key").unwrap();

        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0], (&b"lonely_key"[..], None));
    }

    #[test]
    fn test_mixed_usage_vector_and_dictionary() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Use vector operations
        bufvec.add(b"name").unwrap();
        bufvec.add(b"Alice").unwrap();
        bufvec.add(b"age").unwrap();
        bufvec.add(b"30").unwrap();

        // Test vector interface still works
        assert_eq!(bufvec.len(), 4);
        assert_eq!(bufvec.get(0), b"name");
        assert_eq!(bufvec.get(1), b"Alice");

        // Test dictionary interface works
        assert_eq!(bufvec.pairs_count(), 2);
        assert!(!bufvec.has_unpaired_key());

        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs[0], (&b"name"[..], Some(&b"Alice"[..])));
        assert_eq!(pairs[1], (&b"age"[..], Some(&b"30"[..])));

        // Test that popping works and affects dictionary view
        let popped = bufvec.pop();
        assert_eq!(popped, b"30");
        assert!(bufvec.has_unpaired_key());
        assert_eq!(bufvec.pairs_count(), 1);

        let pairs_after_pop: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs_after_pop.len(), 2);
        assert_eq!(pairs_after_pop[0], (&b"name"[..], Some(&b"Alice"[..])));
        assert_eq!(pairs_after_pop[1], (&b"age"[..], None));
    }
}

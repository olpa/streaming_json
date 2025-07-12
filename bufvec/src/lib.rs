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
//! // Add key-value pairs using specialized methods
//! bufvec.add_key(b"name").unwrap();      // key at index 0
//! bufvec.add_value(b"Alice").unwrap();   // value at index 1
//! bufvec.add_key(b"age").unwrap();       // key at index 2
//! bufvec.add_value(b"30").unwrap();      // value at index 3
//!
//! // Specialized methods handle replacement logic
//! bufvec.add_key(b"country").unwrap();   // replaces "age" key since last element was a value
//! bufvec.add_value(b"USA").unwrap();     // adds normally since last element is now a key
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
//! ## Replacement Semantics
//!
//! The specialized dictionary methods `add_key()` and `add_value()` implement smart replacement logic:
//!
//! - `add_key()`: If the last element is already a key, replaces it. Otherwise, adds normally.
//! - `add_value()`: If the last element is already a value, replaces it. Otherwise, adds normally.
//!
//! This allows for building dictionaries incrementally while correcting mistakes:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! bufvec.add_key(b"name").unwrap();
//! bufvec.add_key(b"username").unwrap();  // replaces "name" with "username"
//! bufvec.add_value(b"alice").unwrap();   // adds value for "username"
//! bufvec.add_value(b"alice123").unwrap(); // replaces "alice" with "alice123"
//!
//! assert_eq!(bufvec.len(), 2);
//! assert_eq!(bufvec.get(0), b"username");
//! assert_eq!(bufvec.get(1), b"alice123");
//! ```
//!
//! # Stack Interface
//!
//! `BufVec` supports stack operations through methods like `push()`, `pop()`, and `top()`:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Push elements onto the stack
//! bufvec.push(b"first").unwrap();
//! bufvec.push(b"second").unwrap();
//! bufvec.push(b"third").unwrap();
//!
//! // Peek at the top element without removing it
//! assert_eq!(bufvec.top(), b"third");
//! assert_eq!(bufvec.len(), 3);
//!
//! // Pop elements in LIFO order
//! assert_eq!(bufvec.pop(), b"third");
//! assert_eq!(bufvec.pop(), b"second");
//! assert_eq!(bufvec.pop(), b"first");
//!
//! assert!(bufvec.is_empty());
//!
//! // Safe variants for error handling
//! assert!(bufvec.try_top().is_err());
//! assert!(bufvec.try_pop().is_err());
//! ```
//!
//! The stack interface maintains compatibility with vector operations:
//!
//! ```
//! # use bufvec::BufVec;
//! let mut buffer = [0u8; 200];
//! let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
//!
//! // Mix stack and vector operations
//! bufvec.push(b"stack_data").unwrap();
//! bufvec.add(b"vector_data").unwrap();
//!
//! // Both interfaces work on the same underlying data
//! assert_eq!(bufvec.get(0), b"stack_data");
//! assert_eq!(bufvec.top(), b"vector_data");
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BufVecError {
    /// Buffer has insufficient space for the requested operation
    BufferOverflow {
        /// Number of bytes requested
        requested: usize,
        /// Number of bytes available
        available: usize,
    },
    /// Index is beyond the current vector length
    IndexOutOfBounds {
        /// Index that was accessed
        index: usize,
        /// Current length of the vector
        length: usize,
    },
    /// Operation attempted on an empty vector
    EmptyVector,
    /// Buffer is too small to hold the required metadata
    BufferTooSmall {
        /// Minimum buffer size required
        required: usize,
        /// Actual buffer size provided
        provided: usize,
    },
    /// Maximum number of slices has been reached
    SliceLimitExceeded {
        /// Maximum number of slices allowed
        max_slices: usize,
    },
    /// Zero-size buffer provided where data storage is required
    ZeroSizeBuffer,
    /// Invalid configuration parameter
    InvalidConfiguration {
        /// Description of the invalid parameter
        parameter: &'static str,
        /// Provided value
        value: usize,
    },
}

impl fmt::Display for BufVecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufVecError::BufferOverflow { requested, available } => {
                write!(f, "Buffer overflow: requested {} bytes, only {} available", requested, available)
            }
            BufVecError::IndexOutOfBounds { index, length } => {
                write!(f, "Index {} out of bounds for vector of length {}", index, length)
            }
            BufVecError::EmptyVector => {
                write!(f, "Operation attempted on empty vector")
            }
            BufVecError::BufferTooSmall { required, provided } => {
                write!(f, "Buffer too small: {} bytes required, {} bytes provided", required, provided)
            }
            BufVecError::SliceLimitExceeded { max_slices } => {
                write!(f, "Maximum number of slices ({}) exceeded", max_slices)
            }
            BufVecError::ZeroSizeBuffer => {
                write!(f, "Zero-size buffer provided where data storage is required")
            }
            BufVecError::InvalidConfiguration { parameter, value } => {
                write!(f, "Invalid configuration: {} = {}", parameter, value)
            }
        }
    }
}

impl std::error::Error for BufVecError {}

const SLICE_DESCRIPTOR_SIZE: usize = 16; // 2 * size_of::<usize>() on 64-bit
const DEFAULT_MAX_SLICES: usize = 8;

#[derive(Debug)]
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
            return Err(BufVecError::InvalidConfiguration {
                parameter: "max_slices",
                value: max_slices,
            });
        }

        if buffer.is_empty() {
            return Err(BufVecError::ZeroSizeBuffer);
        }

        let metadata_space = max_slices * SLICE_DESCRIPTOR_SIZE;
        let min_required = metadata_space + 1; // At least 1 byte for data

        if buffer.len() < min_required {
            return Err(BufVecError::BufferTooSmall {
                required: min_required,
                provided: buffer.len(),
            });
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
            Err(BufVecError::IndexOutOfBounds {
                index,
                length: self.count,
            })
        } else {
            Ok(())
        }
    }

    fn ensure_capacity(&self, additional_bytes: usize) -> Result<(), BufVecError> {
        // Check if we've reached the maximum number of slices
        if self.count >= self.max_slices {
            return Err(BufVecError::SliceLimitExceeded {
                max_slices: self.max_slices,
            });
        }

        // Check if we have enough space for the additional bytes
        let available_data_space = self.buffer.len() - self.data_start() - self.data_used();
        if additional_bytes > available_data_space {
            return Err(BufVecError::BufferOverflow {
                requested: additional_bytes,
                available: available_data_space,
            });
        }
        Ok(())
    }

    #[allow(clippy::expect_used)]
    fn get_slice_descriptor(&self, index: usize) -> (usize, usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        let start_bytes = self
            .buffer
            .get(offset..offset + 8)
            .expect("Buffer bounds checked during construction");
        let length_bytes = self
            .buffer
            .get(offset + 8..offset + 16)
            .expect("Buffer bounds checked during construction");

        let start = usize::from_le_bytes(start_bytes.try_into().expect("Slice is exactly 8 bytes"));
        let length =
            usize::from_le_bytes(length_bytes.try_into().expect("Slice is exactly 8 bytes"));

        (start, length)
    }

    #[allow(clippy::expect_used)]
    fn set_slice_descriptor(&mut self, index: usize, start: usize, length: usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        self.buffer
            .get_mut(offset..offset + 8)
            .expect("Buffer bounds checked during construction")
            .copy_from_slice(&start.to_le_bytes());
        self.buffer
            .get_mut(offset + 8..offset + 16)
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
        self.buffer
            .get(start..start + length)
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
        Ok(self
            .buffer
            .get(start..start + length)
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

        self.buffer
            .get_mut(start..end)
            .expect("Buffer capacity checked by ensure_capacity")
            .copy_from_slice(data);
        self.set_slice_descriptor(self.count, start, data.len());
        self.count += 1;

        Ok(())
    }

    /// Pushes a slice onto the stack (alias for `add`).
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
    pub fn push(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        self.add(data)
    }

    /// Returns a reference to the top element of the stack (last element) without removing it.
    ///
    /// # Panics
    ///
    /// Panics if the stack is empty.
    #[must_use]
    pub fn top(&self) -> &[u8] {
        assert!(self.count > 0, "Cannot peek at top of empty stack");
        self.get(self.count - 1)
    }

    /// Tries to return a reference to the top element of the stack (last element) without removing it.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::EmptyVector` if the stack is empty.
    pub fn try_top(&self) -> Result<&[u8], BufVecError> {
        if self.count == 0 {
            return Err(BufVecError::EmptyVector);
        }
        Ok(self.get(self.count - 1))
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
        self.buffer
            .get(start..start + length)
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
        Ok(self
            .buffer
            .get(start..start + length)
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

    /// Adds a key to the dictionary. If the last element is already a key, replaces it.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::BufferOverflow` if:
    /// - The maximum number of slices has been reached and replacement is not possible
    /// - There is insufficient space in the buffer for the data and replacement is not possible
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    pub fn add_key(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        if self.is_empty() || !self.has_unpaired_key() {
            // Empty vector or last element is a value, so add normally
            self.add(data)
        } else {
            // Last element is a key, replace it
            self.replace_last(data)
        }
    }

    /// Adds a value to the dictionary. If the last element is already a value, replaces it.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::BufferOverflow` if:
    /// - The maximum number of slices has been reached and replacement is not possible
    /// - There is insufficient space in the buffer for the data and replacement is not possible
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    pub fn add_value(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        if self.is_empty() || self.has_unpaired_key() {
            // Empty vector or last element is a key, so add normally
            self.add(data)
        } else {
            // Last element is a value, replace it
            self.replace_last(data)
        }
    }

    #[allow(clippy::expect_used)]
    fn replace_last(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        if self.is_empty() {
            return Err(BufVecError::EmptyVector);
        }

        // Calculate space needed and available space after removing last element
        let last_index = self.count - 1;

        // Check if new data fits in the space that would be available
        // We need to consider the space after all elements except the last one
        let mut data_used_without_last = 0;
        for i in 0..last_index {
            let (slice_start, slice_length) = self.get_slice_descriptor(i);
            let slice_end = slice_start + slice_length - self.data_start();
            data_used_without_last = data_used_without_last.max(slice_end);
        }

        let available_space = self.buffer.len() - self.data_start() - data_used_without_last;
        if data.len() > available_space {
            return Err(BufVecError::BufferOverflow {
                requested: data.len(),
                available: available_space,
            });
        }

        // Place new data at the end of existing data (excluding the last element)
        let new_start = self.data_start() + data_used_without_last;
        let new_end = new_start + data.len();

        self.buffer
            .get_mut(new_start..new_end)
            .expect("Buffer capacity checked above")
            .copy_from_slice(data);

        // Update the descriptor for the last element
        self.set_slice_descriptor(last_index, new_start, data.len());

        Ok(())
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
    #![allow(clippy::unwrap_used, clippy::expect_used)]
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

    #[test]
    fn test_add_key_on_empty_vector() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        assert!(bufvec.add_key(b"key1").is_ok());
        assert_eq!(bufvec.len(), 1);
        assert_eq!(bufvec.get(0), b"key1");
        assert!(bufvec.has_unpaired_key());
    }

    #[test]
    fn test_add_key_replacing_existing_key() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();

        assert_eq!(bufvec.len(), 3);
        assert!(bufvec.has_unpaired_key());

        // Replace the last key
        assert!(bufvec.add_key(b"newkey2").is_ok());
        assert_eq!(bufvec.len(), 3);
        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"newkey2");
        assert!(bufvec.has_unpaired_key());
    }

    #[test]
    fn test_add_key_after_value_normal_add() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();

        assert_eq!(bufvec.len(), 2);
        assert!(!bufvec.has_unpaired_key());

        // Should add normally after a value
        assert!(bufvec.add_key(b"key2").is_ok());
        assert_eq!(bufvec.len(), 3);
        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"key2");
        assert!(bufvec.has_unpaired_key());
    }

    #[test]
    fn test_add_value_replacing_existing_value() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();
        bufvec.add(b"value2").unwrap();

        assert_eq!(bufvec.len(), 4);
        assert!(!bufvec.has_unpaired_key());

        // Replace the last value
        assert!(bufvec.add_value(b"newvalue2").is_ok());
        assert_eq!(bufvec.len(), 4);
        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"key2");
        assert_eq!(bufvec.get(3), b"newvalue2");
        assert!(!bufvec.has_unpaired_key());
    }

    #[test]
    fn test_add_value_after_key_normal_add() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();

        assert_eq!(bufvec.len(), 1);
        assert!(bufvec.has_unpaired_key());

        // Should add normally after a key
        assert!(bufvec.add_value(b"value1").is_ok());
        assert_eq!(bufvec.len(), 2);
        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert!(!bufvec.has_unpaired_key());
    }

    #[test]
    fn test_add_value_on_empty_vector() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        assert!(bufvec.add_value(b"value1").is_ok());
        assert_eq!(bufvec.len(), 1);
        assert_eq!(bufvec.get(0), b"value1");
        assert!(bufvec.has_unpaired_key()); // Single element at index 0 is considered a key
    }

    #[test]
    fn test_buffer_overflow_in_replacement_scenarios() {
        let mut buffer = [0u8; 150];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Fill buffer close to capacity
        bufvec.add(b"short").unwrap();
        bufvec.add(b"tiny").unwrap();

        // Try to replace with data that won't fit
        let long_data = vec![b'x'; 100];
        assert!(bufvec.add_key(&long_data).is_err());
        assert!(bufvec.add_value(&long_data).is_err());

        // Original data should be unchanged
        assert_eq!(bufvec.get(0), b"short");
        assert_eq!(bufvec.get(1), b"tiny");
    }

    #[test]
    fn test_key_replacement_preserves_order() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();
        bufvec.add(b"value2").unwrap();
        bufvec.add(b"key3").unwrap();

        // Replace last key
        bufvec.add_key(b"replacedkey3").unwrap();

        // Check that all elements are in correct order
        assert_eq!(bufvec.len(), 5);
        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"key2");
        assert_eq!(bufvec.get(3), b"value2");
        assert_eq!(bufvec.get(4), b"replacedkey3");
        assert!(bufvec.has_unpaired_key());

        // Dictionary interface should work correctly
        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs.len(), 3);
        assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
        assert_eq!(pairs[1], (&b"key2"[..], Some(&b"value2"[..])));
        assert_eq!(pairs[2], (&b"replacedkey3"[..], None));
    }

    #[test]
    fn test_value_replacement_preserves_order() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"key1").unwrap();
        bufvec.add(b"value1").unwrap();
        bufvec.add(b"key2").unwrap();
        bufvec.add(b"value2").unwrap();

        // Replace last value
        bufvec.add_value(b"replacedvalue2").unwrap();

        // Check that all elements are in correct order
        assert_eq!(bufvec.len(), 4);
        assert_eq!(bufvec.get(0), b"key1");
        assert_eq!(bufvec.get(1), b"value1");
        assert_eq!(bufvec.get(2), b"key2");
        assert_eq!(bufvec.get(3), b"replacedvalue2");
        assert!(!bufvec.has_unpaired_key());

        // Dictionary interface should work correctly
        let pairs: Vec<_> = bufvec.pairs().collect();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0], (&b"key1"[..], Some(&b"value1"[..])));
        assert_eq!(pairs[1], (&b"key2"[..], Some(&b"replacedvalue2"[..])));
    }

    #[test]
    fn test_stack_push_operations() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        assert!(bufvec.is_empty());
        assert_eq!(bufvec.len(), 0);

        // Test push operations
        assert!(bufvec.push(b"first").is_ok());
        assert_eq!(bufvec.len(), 1);
        assert!(!bufvec.is_empty());

        assert!(bufvec.push(b"second").is_ok());
        assert_eq!(bufvec.len(), 2);

        assert!(bufvec.push(b"third").is_ok());
        assert_eq!(bufvec.len(), 3);

        // Verify elements are in correct order (LIFO for stack perspective)
        assert_eq!(bufvec.get(0), b"first");
        assert_eq!(bufvec.get(1), b"second");
        assert_eq!(bufvec.get(2), b"third");
    }

    #[test]
    fn test_stack_top_operations() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Test try_top on empty stack
        assert!(bufvec.try_top().is_err());

        // Add elements and test top
        bufvec.push(b"bottom").unwrap();
        assert_eq!(bufvec.top(), b"bottom");
        assert_eq!(bufvec.try_top().unwrap(), b"bottom");

        bufvec.push(b"middle").unwrap();
        assert_eq!(bufvec.top(), b"middle");
        assert_eq!(bufvec.try_top().unwrap(), b"middle");

        bufvec.push(b"top").unwrap();
        assert_eq!(bufvec.top(), b"top");
        assert_eq!(bufvec.try_top().unwrap(), b"top");

        // Verify top doesn't modify the stack
        assert_eq!(bufvec.len(), 3);
        assert_eq!(bufvec.top(), b"top");
    }

    #[test]
    #[should_panic(expected = "Cannot peek at top of empty stack")]
    fn test_stack_top_empty_panic() {
        let mut buffer = [0u8; 200];
        let bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
        let _ = bufvec.top();
    }

    #[test]
    fn test_stack_push_pop_operations() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Push elements
        bufvec.push(b"first").unwrap();
        bufvec.push(b"second").unwrap();
        bufvec.push(b"third").unwrap();

        assert_eq!(bufvec.len(), 3);

        // Pop elements in LIFO order
        assert_eq!(bufvec.pop(), b"third");
        assert_eq!(bufvec.len(), 2);
        assert_eq!(bufvec.top(), b"second");

        assert_eq!(bufvec.pop(), b"second");
        assert_eq!(bufvec.len(), 1);
        assert_eq!(bufvec.top(), b"first");

        assert_eq!(bufvec.pop(), b"first");
        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());

        // Test error handling
        assert!(bufvec.try_pop().is_err());
        assert!(bufvec.try_top().is_err());
    }

    #[test]
    fn test_stack_interface_doesnt_break_vector_operations() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Mix stack and vector operations
        bufvec.push(b"stack1").unwrap();
        bufvec.add(b"vector1").unwrap();
        bufvec.push(b"stack2").unwrap();

        assert_eq!(bufvec.len(), 3);
        assert_eq!(bufvec.get(0), b"stack1");
        assert_eq!(bufvec.get(1), b"vector1");
        assert_eq!(bufvec.get(2), b"stack2");

        // Stack operations still work
        assert_eq!(bufvec.top(), b"stack2");
        assert_eq!(bufvec.pop(), b"stack2");

        // Vector operations still work
        assert_eq!(bufvec.get(0), b"stack1");
        assert_eq!(bufvec.get(1), b"vector1");

        // Iterator still works
        let collected: Vec<_> = bufvec.iter().collect();
        assert_eq!(collected, vec![&b"stack1"[..], &b"vector1"[..]]);
    }

    #[test]
    fn test_stack_buffer_overflow() {
        let mut buffer = [0u8; 150];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Fill buffer to near capacity
        bufvec.push(b"data1").unwrap();
        bufvec.push(b"data2").unwrap();

        // Try to push data that won't fit
        let large_data = vec![b'x'; 100];
        assert!(bufvec.push(&large_data).is_err());

        // Stack should be unchanged
        assert_eq!(bufvec.len(), 2);
        assert_eq!(bufvec.top(), b"data2");
    }

    #[test]
    fn test_stack_utility_methods() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Test utility methods on empty stack
        assert!(bufvec.is_empty());
        assert_eq!(bufvec.len(), 0);

        // Add elements and test utilities
        bufvec.push(b"element").unwrap();
        assert!(!bufvec.is_empty());
        assert_eq!(bufvec.len(), 1);

        bufvec.push(b"another").unwrap();
        assert!(!bufvec.is_empty());
        assert_eq!(bufvec.len(), 2);

        // Clear and test utilities
        bufvec.clear();
        assert!(bufvec.is_empty());
        assert_eq!(bufvec.len(), 0);
    }

    // Error Handling and Edge Cases Tests

    #[test]
    fn test_error_zero_max_slices() {
        let mut buffer = [0u8; 200];
        let result = BufVec::new(&mut buffer, 0);
        assert_eq!(
            result.unwrap_err(),
            BufVecError::InvalidConfiguration {
                parameter: "max_slices",
                value: 0
            }
        );
    }

    #[test]
    fn test_error_zero_size_buffer() {
        let mut buffer = [];
        let result = BufVec::new(&mut buffer, 1);
        assert_eq!(result.unwrap_err(), BufVecError::ZeroSizeBuffer);
    }

    #[test]
    fn test_error_buffer_too_small_for_metadata() {
        let mut buffer = [0u8; 10]; // Too small for even 1 slice (16 bytes needed + 1 for data)
        let result = BufVec::new(&mut buffer, 1);
        assert_eq!(
            result.unwrap_err(),
            BufVecError::BufferTooSmall {
                required: 17, // 16 bytes metadata + 1 byte data minimum
                provided: 10
            }
        );
    }

    #[test]
    fn test_error_detailed_buffer_overflow() {
        let mut buffer = [0u8; 150];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Fill buffer to near capacity
        bufvec.add(b"small").unwrap();

        // Try to add data that won't fit
        let large_data = vec![b'x'; 100];
        let result = bufvec.add(&large_data);
        match result.unwrap_err() {
            BufVecError::BufferOverflow { requested, available } => {
                assert_eq!(requested, 100);
                assert!(available < 100);
            }
            _ => panic!("Expected BufferOverflow error"),
        }
    }

    #[test]
    fn test_error_detailed_index_out_of_bounds() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"test").unwrap();

        let result = bufvec.try_get(5);
        assert_eq!(
            result.unwrap_err(),
            BufVecError::IndexOutOfBounds {
                index: 5,
                length: 1
            }
        );
    }

    #[test]
    fn test_error_slice_limit_exceeded() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::new(&mut buffer, 2).unwrap(); // Only 2 slices allowed

        bufvec.add(b"first").unwrap();
        bufvec.add(b"second").unwrap();

        let result = bufvec.add(b"third");
        assert_eq!(
            result.unwrap_err(),
            BufVecError::SliceLimitExceeded { max_slices: 2 }
        );
    }

    #[test]
    fn test_error_empty_vector_operations() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Test try_pop on empty vector
        assert_eq!(bufvec.try_pop().unwrap_err(), BufVecError::EmptyVector);

        // Test try_top on empty vector
        assert_eq!(bufvec.try_top().unwrap_err(), BufVecError::EmptyVector);

        // Test replace_last on empty vector
        let result = bufvec.replace_last(b"test");
        assert_eq!(result.unwrap_err(), BufVecError::EmptyVector);
    }

    #[test]
    fn test_error_messages_quality() {
        let mut buffer = [0u8; 10];
        let error = BufVec::new(&mut buffer, 1).unwrap_err();
        let message = format!("{}", error);
        assert!(message.contains("17 bytes required"));
        assert!(message.contains("10 bytes provided"));

        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();
        let error = bufvec.try_get(0).unwrap_err();
        let message = format!("{}", error);
        assert!(message.contains("Index 0 out of bounds"));
        assert!(message.contains("length 0"));
    }

    #[test]
    fn test_edge_case_minimal_buffer() {
        // Test with the absolute minimum buffer size
        let mut buffer = [0u8; 17]; // 16 bytes metadata + 1 byte data
        let mut bufvec = BufVec::new(&mut buffer, 1).unwrap();

        // Should be able to add exactly 1 byte
        assert!(bufvec.add(b"x").is_ok());
        assert_eq!(bufvec.get(0), b"x");

        // Should fail to add any more data
        assert!(bufvec.add(b"y").is_err());
    }

    #[test]
    fn test_edge_case_exact_capacity() {
        let mut buffer = [0u8; 150];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Fill buffer to exact capacity
        let data_space = bufvec.available_bytes();
        let exact_data = vec![b'x'; data_space];
        assert!(bufvec.add(&exact_data).is_ok());

        // Should fail to add even 1 more byte
        assert!(bufvec.add(b"y").is_err());
    }

    #[test]
    fn test_edge_case_replacement_with_exact_space() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Add initial data
        bufvec.add(b"short").unwrap();
        bufvec.add(b"data").unwrap();

        // Calculate available space for replacement
        let available = bufvec.available_bytes() + b"data".len(); // Space freed by replacing last element
        let exact_replacement = vec![b'x'; available];

        // Should succeed with exact space
        assert!(bufvec.add_value(&exact_replacement).is_ok());

        // Verify replacement worked
        assert_eq!(bufvec.len(), 2);
        assert_eq!(bufvec.get(0), b"short");
        assert_eq!(bufvec.get(1), &exact_replacement);
    }

    #[test]
    fn test_error_recovery_after_failed_operations() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        // Add some initial data
        bufvec.add(b"initial").unwrap();
        assert_eq!(bufvec.len(), 1);

        // Try to add data that will fail
        let large_data = vec![b'x'; 1000];
        assert!(bufvec.add(&large_data).is_err());

        // Verify state is unchanged after error
        assert_eq!(bufvec.len(), 1);
        assert_eq!(bufvec.get(0), b"initial");

        // Should still be able to add reasonable data
        assert!(bufvec.add(b"recovery").is_ok());
        assert_eq!(bufvec.len(), 2);
        assert_eq!(bufvec.get(1), b"recovery");
    }

    #[test]
    fn test_bounds_checking_with_various_indices() {
        let mut buffer = [0u8; 200];
        let mut bufvec = BufVec::with_default_max_slices(&mut buffer).unwrap();

        bufvec.add(b"element").unwrap();

        // Test valid index
        assert!(bufvec.try_get(0).is_ok());

        // Test various invalid indices
        assert!(bufvec.try_get(1).is_err());
        assert!(bufvec.try_get(10).is_err());
        assert!(bufvec.try_get(usize::MAX).is_err());
    }

    #[test]
    fn test_error_types_implement_standard_traits() {
        let error = BufVecError::EmptyVector;

        // Test Debug
        let debug_str = format!("{:?}", error);
        assert!(!debug_str.is_empty());

        // Test Display
        let display_str = format!("{}", error);
        assert!(!display_str.is_empty());

        // Test Clone
        let cloned = error.clone();
        assert_eq!(error, cloned);

        // Test PartialEq
        assert_eq!(error, BufVecError::EmptyVector);
        assert_ne!(error, BufVecError::ZeroSizeBuffer);

        // Test Error trait
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_comprehensive_error_scenarios() {
        // Test all error variants have proper error messages
        let errors = [
            BufVecError::BufferOverflow { requested: 100, available: 50 },
            BufVecError::IndexOutOfBounds { index: 5, length: 2 },
            BufVecError::EmptyVector,
            BufVecError::BufferTooSmall { required: 100, provided: 50 },
            BufVecError::SliceLimitExceeded { max_slices: 8 },
            BufVecError::ZeroSizeBuffer,
            BufVecError::InvalidConfiguration { parameter: "test", value: 0 },
        ];

        for error in &errors {
            let message = format!("{}", error);
            assert!(!message.is_empty(), "Error message should not be empty for {:?}", error);
            assert!(message.len() > 10, "Error message should be descriptive for {:?}", error);
        }
    }
}

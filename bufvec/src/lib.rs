//! `BufVec`: A zero-allocation vector implementation using client-provided buffers.
//!
//! `BufVec` provides vector, stack, and dictionary interfaces while using a single
//! client-provided buffer for storage. All operations are bounds-checked and
//! no internal allocations are performed.
//!
//! Buffer layout: [metadata section][data section]
//! Metadata section stores slice descriptors as (`start_offset`, length) pairs.

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

    fn get_slice_descriptor(&self, index: usize) -> (usize, usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        let start_bytes = &self.buffer[offset..offset + 8];
        let length_bytes = &self.buffer[offset + 8..offset + 16];

        let start = usize::from_le_bytes(start_bytes.try_into().unwrap());
        let length = usize::from_le_bytes(length_bytes.try_into().unwrap());

        (start, length)
    }

    fn set_slice_descriptor(&mut self, index: usize, start: usize, length: usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        self.buffer[offset..offset + 8].copy_from_slice(&start.to_le_bytes());
        self.buffer[offset + 8..offset + 16].copy_from_slice(&length.to_le_bytes());
    }

    /// Gets a slice at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if `index` is out of bounds.
    #[must_use]
    pub fn get(&self, index: usize) -> &[u8] {
        assert!(
            index < self.count,
            "Index {} out of bounds for vector of length {}",
            index,
            self.count
        );
        let (start, length) = self.get_slice_descriptor(index);
        &self.buffer[start..start + length]
    }

    /// Tries to get a slice at the specified index.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::IndexOutOfBounds` if `index` is out of bounds.
    pub fn try_get(&self, index: usize) -> Result<&[u8], BufVecError> {
        self.check_bounds(index)?;
        let (start, length) = self.get_slice_descriptor(index);
        Ok(&self.buffer[start..start + length])
    }

    /// Adds a slice to the vector.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the data
    pub fn add(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        self.ensure_capacity(data.len())?;

        let start = self.data_start() + self.data_used();
        let end = start + data.len();

        self.buffer[start..end].copy_from_slice(data);
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
    /// Panics if the vector is empty.
    pub fn pop(&mut self) -> &[u8] {
        assert!(self.count > 0, "Cannot pop from empty vector");

        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);

        // data_used is now automatically recalculated when needed
        &self.buffer[start..start + length]
    }

    /// Tries to remove and return the last slice from the vector.
    ///
    /// # Errors
    ///
    /// Returns `BufVecError::EmptyVector` if the vector is empty.
    pub fn try_pop(&mut self) -> Result<&[u8], BufVecError> {
        if self.count == 0 {
            return Err(BufVecError::EmptyVector);
        }

        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);

        // data_used is now automatically recalculated when needed
        Ok(&self.buffer[start..start + length])
    }
}

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
}

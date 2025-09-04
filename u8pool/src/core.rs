use crate::error::U8PoolError;
use crate::iter::{U8PoolIter, U8PoolPairIter};

const SLICE_DESCRIPTOR_SIZE: usize = 16; // 2 * size_of::<usize>() on 64-bit
const DEFAULT_MAX_SLICES: usize = 32;

/// A zero-allocation vector implementation using client-provided buffers
#[derive(Debug)]
pub struct U8Pool<'a> {
    buffer: &'a mut [u8],
    count: usize,
    max_slices: usize,
}

impl<'a> U8Pool<'a> {
    /// Creates a new `U8Pool` with the specified maximum number of slices.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferTooSmall` if:
    /// - `max_slices` is 0
    /// - The buffer is too small to hold the metadata for `max_slices`
    pub fn new(buffer: &'a mut [u8], max_slices: usize) -> Result<Self, U8PoolError> {
        if max_slices == 0 {
            return Err(U8PoolError::InvalidConfiguration {
                parameter: "max_slices",
                value: max_slices,
            });
        }

        if buffer.is_empty() {
            return Err(U8PoolError::ZeroSizeBuffer);
        }

        let metadata_space = max_slices * SLICE_DESCRIPTOR_SIZE;
        let min_required = metadata_space + 1; // At least 1 byte for data

        if buffer.len() < min_required {
            return Err(U8PoolError::BufferTooSmall {
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

    /// Creates a new `U8Pool` with the default maximum number of slices (8).
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferTooSmall` if the buffer is too small.
    pub fn with_default_max_slices(buffer: &'a mut [u8]) -> Result<Self, U8PoolError> {
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

    fn data_used(&self) -> usize {
        if self.count == 0 {
            0
        } else {
            let (last_start, last_length) = self.get_slice_descriptor(self.count - 1);
            last_start + last_length - self.data_start()
        }
    }

    fn check_bounds(&self, index: usize) -> Result<(), U8PoolError> {
        if index >= self.count {
            Err(U8PoolError::IndexOutOfBounds {
                index,
                length: self.count,
            })
        } else {
            Ok(())
        }
    }

    fn ensure_capacity(&self, additional_bytes: usize) -> Result<(), U8PoolError> {
        // Check if we've reached the maximum number of slices
        if self.count >= self.max_slices {
            return Err(U8PoolError::SliceLimitExceeded {
                max_slices: self.max_slices,
            });
        }

        // Check if we have enough space for the additional bytes
        let available_data_space = self.buffer.len() - self.data_start() - self.data_used();
        if additional_bytes > available_data_space {
            return Err(U8PoolError::BufferOverflow {
                requested: additional_bytes,
                available: available_data_space,
            });
        }
        Ok(())
    }

    #[allow(clippy::expect_used)]
    fn get_slice_descriptor(&self, index: usize) -> (usize, usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        // Read both values in a single 16-byte slice operation for better cache efficiency
        let descriptor_bytes = self
            .buffer
            .get(offset..offset + SLICE_DESCRIPTOR_SIZE)
            .expect("Buffer bounds checked during construction");

        let start = usize::from_le_bytes(descriptor_bytes[0..8].try_into().expect("First 8 bytes"));
        let length =
            usize::from_le_bytes(descriptor_bytes[8..16].try_into().expect("Last 8 bytes"));

        (start, length)
    }

    #[allow(clippy::expect_used)]
    fn set_slice_descriptor(&mut self, index: usize, start: usize, length: usize) {
        let offset = index * SLICE_DESCRIPTOR_SIZE;

        // Write both values in a single slice operation for better cache efficiency
        let descriptor_bytes = self
            .buffer
            .get_mut(offset..offset + SLICE_DESCRIPTOR_SIZE)
            .expect("Buffer bounds checked during construction");

        descriptor_bytes[0..8].copy_from_slice(&start.to_le_bytes());
        descriptor_bytes[8..16].copy_from_slice(&length.to_le_bytes());
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
    /// Returns `U8PoolError::IndexOutOfBounds` if `index` is out of bounds.
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[allow(clippy::expect_used)]
    pub fn try_get(&self, index: usize) -> Result<&[u8], U8PoolError> {
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
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the data
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[allow(clippy::expect_used)]
    pub fn add(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
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
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the data
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    pub fn push(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
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
    /// Returns `U8PoolError::EmptyVector` if the stack is empty.
    pub fn try_top(&self) -> Result<&[u8], U8PoolError> {
        if self.count == 0 {
            return Err(U8PoolError::EmptyVector);
        }
        Ok(self.get(self.count - 1))
    }

    pub fn clear(&mut self) {
        self.count = 0;
        // data_used is now derived from slice descriptors, so no need to reset it
    }

    /// Removes and returns the last slice from the vector.
    ///
    /// Returns `None` if the vector is empty.
    pub fn pop(&mut self) -> Option<&[u8]> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);

        // data_used is now automatically recalculated when needed
        self.buffer.get(start..start + length)
    }

    /// Tries to remove and return the last slice from the vector.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::EmptyVector` if the vector is empty.
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[allow(clippy::expect_used)]
    pub fn try_pop(&mut self) -> Result<&[u8], U8PoolError> {
        if self.count == 0 {
            return Err(U8PoolError::EmptyVector);
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
    pub fn iter(&self) -> U8PoolIter<'_> {
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
    pub fn pairs(&self) -> U8PoolPairIter<'_> {
        U8PoolPairIter {
            u8pool: self,
            current_pair: 0,
        }
    }

    /// Adds a key to the dictionary. If the last element is already a key, replaces it.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached and replacement is not possible
    /// - There is insufficient space in the buffer for the data and replacement is not possible
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    pub fn add_key(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
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
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached and replacement is not possible
    /// - There is insufficient space in the buffer for the data and replacement is not possible
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    pub fn add_value(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
        if self.is_empty() || self.has_unpaired_key() {
            // Empty vector or last element is a key, so add normally
            self.add(data)
        } else {
            // Last element is a value, replace it
            self.replace_last(data)
        }
    }

    #[allow(clippy::expect_used)]
    fn replace_last(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
        if self.is_empty() {
            return Err(U8PoolError::EmptyVector);
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
            return Err(U8PoolError::BufferOverflow {
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

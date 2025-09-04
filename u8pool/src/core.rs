use crate::error::U8PoolError;
use crate::iter::{U8PoolIter, U8PoolPairIter, U8PoolRevIter};

const SLICE_DESCRIPTOR_SIZE: usize = 16; // 2 * size_of::<usize>() on 64-bit
const DEFAULT_MAX_SLICES: usize = 32;

/// A zero-allocation stack implementation using client-provided buffers
#[derive(Debug)]
pub struct U8Pool<'a> {
    pub(crate) buffer: &'a mut [u8],
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
    pub(crate) fn get_slice_descriptor(&self, index: usize) -> (usize, usize) {
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

    /// Pushes a slice onto the stack.
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
    pub fn push(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
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

    /// Removes and returns the last slice from the vector.
    ///
    /// Returns `None` if the vector is empty.
    pub fn pop(&mut self) -> Option<&[u8]> {
        if self.count == 0 {
            return None;
        }

        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);

        self.buffer.get(start..start + length)
    }

    /// Gets a slice at the specified index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Panics
    ///
    /// May panic if buffer integrity is compromised (internal validation failure).
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn get(&self, index: usize) -> Option<&[u8]> {
        if index >= self.count {
            return None;
        }
        let (start, length) = self.get_slice_descriptor(index);
        Some(
            self.buffer
                .get(start..start + length)
                .expect("Slice bounds validated during add operation"),
        )
    }

    pub fn clear(&mut self) {
        self.count = 0;
        // data_used is now derived from slice descriptors, so no need to reset it
    }


    /// Returns an iterator over the slices in the vector.
    #[must_use]
    pub fn iter(&self) -> U8PoolIter<'_> {
        self.into_iter()
    }

    /// Returns a reverse iterator over the slices in the vector.
    #[must_use]
    pub fn iter_rev(&self) -> U8PoolRevIter<'_> {
        U8PoolRevIter::new(self)
    }

    /// Returns an iterator over key-value pairs.
    #[must_use]
    pub fn pairs(&self) -> U8PoolPairIter<'_> {
        U8PoolPairIter::new(self)
    }
}

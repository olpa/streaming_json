use crate::error::U8PoolError;
use crate::iter::{U8PoolIter, U8PoolPairIter, U8PoolRevIter};
use crate::slice_descriptor::SliceDescriptor;

const SLICE_DESCRIPTOR_SIZE: usize = 4; // 2 bytes start + 2 bytes length
const DEFAULT_MAX_SLICES: usize = 32;

/// A zero-allocation stack for u8 slices copied to a client-provided buffer
#[derive(Debug)]
pub struct U8Pool<'a> {
    max_slices: usize,
    count: usize,
    descriptor: SliceDescriptor<'a>,
    data: &'a mut [u8],
}

impl<'a> U8Pool<'a> {
    /// Creates a new `U8Pool` with the specified maximum number of slices.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::InvalidInitialization` if:
    /// - `max_slices` is 0
    /// - The buffer is empty
    /// - The buffer is too small to hold the metadata for `max_slices`
    pub fn new(buffer: &'a mut [u8], max_slices: usize) -> Result<Self, U8PoolError> {
        if max_slices == 0 {
            return Err(U8PoolError::InvalidInitialization {
                reason: "max_slices cannot be zero",
            });
        }

        if buffer.is_empty() {
            return Err(U8PoolError::InvalidInitialization {
                reason: "buffer cannot be empty",
            });
        }

        let metadata_space = max_slices * SLICE_DESCRIPTOR_SIZE;
        let min_required = metadata_space + 1; // At least 1 byte for data

        if buffer.len() < min_required {
            return Err(U8PoolError::InvalidInitialization {
                reason: "buffer too small for the requested max_slices",
            });
        }

        let (descriptor_buffer, data) = buffer.split_at_mut(metadata_space);
        let descriptor = SliceDescriptor::new(descriptor_buffer);

        Ok(Self {
            data,
            count: 0,
            max_slices,
            descriptor,
        })
    }

    /// Creates a new `U8Pool` with the default maximum number of slices (32).
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::InvalidInitialization` if the buffer is too small.
    pub fn with_default_max_slices(buffer: &'a mut [u8]) -> Result<Self, U8PoolError> {
        Self::new(buffer, DEFAULT_MAX_SLICES)
    }

    /// Returns the number of slices currently stored in the pool.
    #[must_use]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the pool contains no slices.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn data_used(&self) -> usize {
        if self.count == 0 {
            0
        } else if let Some((last_start, last_length)) = self.descriptor.get(self.count - 1) {
            last_start + last_length
        } else {
            // This branch should never happen. However, defend against it.
            // If descriptor is corrupted, assume whole data buffer is used.
            self.data.len()
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
        let available_data_space = self.data.len() - self.data_used();
        if additional_bytes > available_data_space {
            return Err(U8PoolError::BufferOverflow {
                requested: additional_bytes,
                available: available_data_space,
            });
        }
        Ok(())
    }

    /// Pushes a slice onto the stack.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the data
    ///
    pub fn push(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
        self.ensure_capacity(data.len())?;

        let start = self.data_used();
        let end = start + data.len();
        let available = self.data.len().saturating_sub(start);

        let data_slice = self
            .data
            .get_mut(start..end)
            .ok_or(U8PoolError::BufferOverflow {
                requested: data.len(),
                available,
            })?;
        data_slice.copy_from_slice(data);
        self.descriptor.set(self.count, start, data.len())?;
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
        let (start, length) = self.descriptor.get(self.count)?;

        self.data.get(start..start + length)
    }

    /// Gets a slice at the specified index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&[u8]> {
        if index >= self.count {
            return None;
        }
        let (start, length) = self.descriptor.get(index)?;
        self.data.get(start..start + length)
    }

    /// Removes all slices from the pool, making it empty.
    ///
    /// This does not affect the underlying data buffer, only the slice count.
    pub fn clear(&mut self) {
        self.count = 0;
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

    /// Returns the descriptor iterator (internal use).
    pub(crate) fn descriptor_iter(&self) -> crate::slice_descriptor::SliceDescriptorIter<'_> {
        self.descriptor.iter(self.count)
    }

    /// Returns the reverse descriptor iterator (internal use).
    pub(crate) fn descriptor_iter_rev(
        &self,
    ) -> crate::slice_descriptor::SliceDescriptorRevIter<'_> {
        self.descriptor.iter_rev(self.count)
    }

    /// Returns the data buffer (internal use).
    pub(crate) fn data(&self) -> &[u8] {
        self.data
    }
}

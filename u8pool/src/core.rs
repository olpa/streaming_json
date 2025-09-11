use crate::error::U8PoolError;
use crate::iter::{U8PoolAssocIter, U8PoolAssocRevIter, U8PoolIter, U8PoolPairIter, U8PoolRevIter};
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

    /// Removes all slices from the pool, making it empty.
    ///
    /// This does not affect the underlying data buffer, only the slice count.
    pub fn clear(&mut self) {
        self.count = 0;
    }

    // -------------------------------------------------------------------------
    // Internal accounting
    //

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

    /// Helper function to validate capacity and reserve buffer space.
    ///
    /// Validates that there is capacity for another slice and sufficient buffer space
    /// for the requested size, then returns the buffer positions for the reservation.
    ///
    /// # Returns
    ///
    /// `Ok((start, end))` where:
    /// - `start`: Start position for the new data in the buffer
    /// - `end`: End position for the new data (`start` + `total_size`)
    ///
    /// Returns an error if:
    /// - Maximum slice limit is reached (`SliceLimitExceeded`)
    /// - Insufficient buffer space for the requested size (`BufferOverflow`)
    ///
    /// # Contract
    ///
    /// When this function returns `Ok((start, end))`:
    /// - `self.data[start..end]` is guaranteed to be within bounds
    /// - The range `start..end` has exactly `total_size` bytes
    /// - The range is available for writing (not overlapping with existing data)
    /// - `self.count < self.max_slices` (space for another slice descriptor)
    fn reserve_buffer_space(&mut self, total_size: usize) -> Result<(usize, usize), U8PoolError> {
        // Check if we've reached the maximum number of slices
        if self.count >= self.max_slices {
            return Err(U8PoolError::SliceLimitExceeded {
                max_slices: self.max_slices,
            });
        }

        let start = self.data_used();
        let end = start + total_size;
        let available = self.data.len().saturating_sub(start);

        // Check if we have enough space for the additional bytes
        if total_size > available {
            return Err(U8PoolError::BufferOverflow {
                requested: total_size,
                available,
            });
        }

        Ok((start, end))
    }

    fn finalize_push(&mut self, start: usize, total_size: usize) -> Result<(), U8PoolError> {
        self.descriptor.set(self.count, start, total_size)?;
        self.count += 1;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Basic push/pop/get methods
    //

    /// Pushes a slice onto the stack.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the data
    ///
    pub fn push(&mut self, data: &[u8]) -> Result<(), U8PoolError> {
        let (start, end) = self.reserve_buffer_space(data.len())?;

        // Safe: reserve_buffer_space() guarantees start..end is within bounds
        #[allow(clippy::indexing_slicing)]
        let data_slice = &mut self.data[start..end];
        data_slice.copy_from_slice(data);

        self.finalize_push(start, data.len())
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

        // Safe: descriptor.get() guarantees start..start+length is within bounds
        #[allow(clippy::indexing_slicing)]
        Some(&self.data[start..start + length])
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
        // Safe: descriptor.get() guarantees start..start+length is within bounds
        #[allow(clippy::indexing_slicing)]
        Some(&self.data[start..start + length])
    }

    // -------------------------------------------------------------------------
    // Associated push/pop/get methods
    //

    /// Pushes an associated value followed by a data slice onto the stack.
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the associated value and data
    ///
    pub fn push_assoc<T: Sized>(&mut self, assoc: T, data: &[u8]) -> Result<(), U8PoolError> {
        let assoc_size = core::mem::size_of::<T>();
        let total_size = assoc_size + data.len();
        let (start, _end) = self.reserve_buffer_space(total_size)?;

        let assoc_end = start + assoc_size;
        let data_end = assoc_end + data.len();

        // Safe: reserve_buffer_space() guarantees all ranges are within bounds
        #[allow(clippy::indexing_slicing)]
        let assoc_slice = &mut self.data[start..assoc_end];
        #[allow(unsafe_code)]
        unsafe {
            let assoc_ptr = assoc_slice.as_mut_ptr().cast::<T>();
            core::ptr::write(assoc_ptr, assoc);
        }

        // Safe: reserve_buffer_space() guarantees all ranges are within bounds
        #[allow(clippy::indexing_slicing)]
        let data_slice = &mut self.data[assoc_end..data_end];
        data_slice.copy_from_slice(data);

        self.finalize_push(start, total_size)
    }

    /// Helper function to validate and compute buffer positions for associated data access.
    ///
    /// Validates that the index is within bounds and that the stored data is large enough
    /// to contain an associated value of type T, then returns the buffer positions needed
    /// to access both the associated value and the data portion.
    ///
    /// # Returns
    ///
    /// `Some((assoc_start, assoc_end, data_end))` where:
    /// - `assoc_start`: Start position of the associated value in the buffer
    /// - `assoc_end`: End position of the associated value / start position of data
    /// - `data_end`: End position of the data portion
    ///
    /// Returns `None` if:
    /// - Index is out of bounds
    /// - Descriptor lookup fails
    /// - Stored data is too small to contain an associated value of type T
    ///
    /// # Contract
    ///
    /// When this function returns `Some((start, assoc_end, data_end))`:
    /// - `self.data[start..assoc_end]` is guaranteed to be valid for reading type T
    /// - `self.data[assoc_end..data_end]` is guaranteed to be valid for data access
    /// - Both ranges are within the bounds of `self.data`
    fn get_validated_assoc_positions<T: Sized>(
        &self,
        index: usize,
    ) -> Option<(usize, usize, usize)> {
        if index >= self.count {
            return None;
        }
        let (start, total_length) = self.descriptor.get(index)?;
        let assoc_size = core::mem::size_of::<T>();

        if total_length < assoc_size {
            return None;
        }

        let assoc_end = start + assoc_size;
        let data_end = start + total_length;
        Some((start, assoc_end, data_end))
    }

    /// Helper function to extract associated value reference from buffer positions.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `assoc_end - start >= core::mem::size_of::<T>()`
    /// - The data in `self.data[start..assoc_end]` was written as type `T` using `core::ptr::write`
    /// - Type `T` matches the original type used when storing the data
    /// - All positions are within bounds of `self.data`
    #[allow(unsafe_code)]
    unsafe fn extract_assoc_ref<T: Sized>(
        &self,
        start: usize,
        assoc_end: usize,
        data_end: usize,
    ) -> (&T, &[u8]) {
        // Safe: caller guarantees all positions are within bounds (see safety contract above)
        #[allow(clippy::indexing_slicing)]
        let assoc_slice = &self.data[start..assoc_end];
        #[allow(clippy::indexing_slicing)]
        let data_slice = &self.data[assoc_end..data_end];

        let assoc_ptr = assoc_slice.as_ptr().cast::<T>();
        let assoc_ref = &*assoc_ptr;
        (assoc_ref, data_slice)
    }

    /// Gets an associated value and data slice at the specified index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the item at the specified index was pushed with `push_assoc`
    /// and that the type `T` matches the original associated type.
    #[must_use]
    pub fn get_assoc<T: Sized>(&self, index: usize) -> Option<(&T, &[u8])> {
        let (start, assoc_end, data_end) = self.get_validated_assoc_positions::<T>(index)?;

        // Safe: get_validated_assoc_positions() guarantees all contracts for extract_assoc_ref()
        #[allow(unsafe_code)]
        unsafe {
            Some(self.extract_assoc_ref::<T>(start, assoc_end, data_end))
        }
    }

    /// Removes and returns the last associated value and data slice from the vector.
    ///
    /// Returns `None` if the vector is empty.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the last pushed item was indeed pushed with `push_assoc`
    /// and that the type `T` matches the original associated type.
    pub fn pop_assoc<T: Sized>(&mut self) -> Option<(&T, &[u8])> {
        if self.count == 0 {
            return None;
        }

        let last_index = self.count - 1;
        let (start, assoc_end, data_end) = self.get_validated_assoc_positions::<T>(last_index)?;

        self.count -= 1;

        // Safe: get_validated_assoc_positions() guarantees all contracts for extract_assoc_ref()
        #[allow(unsafe_code)]
        unsafe {
            Some(self.extract_assoc_ref::<T>(start, assoc_end, data_end))
        }
    }

    // -------------------------------------------------------------------------
    // Iterators
    //

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

    /// Returns an iterator over associated values and data slices.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all items in the pool were pushed with `push_assoc`
    /// and that the type `T` matches the original associated type for all items.
    #[must_use]
    pub fn iter_assoc<T: Sized>(&self) -> U8PoolAssocIter<'_, T> {
        U8PoolAssocIter::new(self)
    }

    /// Returns a reverse iterator over associated values and data slices.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all items in the pool were pushed with `push_assoc`
    /// and that the type `T` matches the original associated type for all items.
    #[must_use]
    pub fn iter_assoc_rev<T: Sized>(&self) -> U8PoolAssocRevIter<'_, T> {
        U8PoolAssocRevIter::new(self)
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

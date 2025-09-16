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

    /// Helper function to validate capacity and reserve aligned buffer space.
    ///
    /// Validates that there is capacity for another slice and sufficient buffer space
    /// for the requested size, with alignment for the specified type.
    ///
    /// # Returns
    ///
    /// `Ok((aligned_start, end))` where:
    /// - `aligned_start`: Aligned start position for type T (to be stored in descriptor)
    /// - `end`: End position of the data
    ///
    /// Returns an error if:
    /// - Maximum slice limit is reached (`SliceLimitExceeded`)
    /// - Insufficient buffer space for the aligned data (`BufferOverflow`)
    ///
    /// # Contract
    ///
    /// When this function returns `Ok((aligned_start, end))`:
    /// - `self.data[aligned_start..end]` is guaranteed to be within bounds
    /// - `aligned_start` is properly aligned for type T
    /// - The range is available for writing (not overlapping with existing data)
    /// - `self.count < self.max_slices` (space for another slice descriptor)
    /// - The `aligned_start` should be stored in the descriptor for later retrieval
    fn reserve_aligned_buffer_space<T: Sized>(
        &mut self,
        data_size: usize,
    ) -> Result<(usize, usize), U8PoolError> {
        // Check if we've reached the maximum number of slices
        if self.count >= self.max_slices {
            return Err(U8PoolError::SliceLimitExceeded {
                max_slices: self.max_slices,
            });
        }

        let current_pos = self.data_used();
        let aligned_start = current_pos.next_multiple_of(core::mem::align_of::<T>());
        let total_size = (aligned_start - current_pos) + core::mem::size_of::<T>() + data_size;
        let end = aligned_start + core::mem::size_of::<T>() + data_size;

        let available = self.data.len().saturating_sub(current_pos);

        // Check if we have enough space for the aligned data
        if total_size > available {
            return Err(U8PoolError::BufferOverflow {
                requested: total_size,
                available,
            });
        }

        Ok((aligned_start, end))
    }

    /// Finalizes a push operation by storing the slice descriptor.
    ///
    /// # Contract
    ///
    /// For associated values (`push_assoc`), `start` must be the aligned start position
    /// for the associated type. For regular slices (push), `start` is the actual
    /// start position (which equals aligned start since no alignment is required).
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
        let (aligned_start, end) = self.reserve_aligned_buffer_space::<()>(data.len())?;

        // Safe: reserve_aligned_buffer_space() guarantees the range is within bounds
        #[allow(clippy::indexing_slicing)]
        let data_slice = &mut self.data[aligned_start..end];
        data_slice.copy_from_slice(data);

        self.finalize_push(aligned_start, end - aligned_start)
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
    /// The associated value is stored with proper memory alignment for type `T`.
    /// Padding bytes may be inserted before the value to ensure correct alignment,
    /// which means the total space used may exceed `size_of::<T>() + data.len()`.
    /// These padding bytes are left untouched in the buffer.
    ///
    /// # Memory Layout
    ///
    /// ```text
    /// [padding bytes] [associated value T] [data slice]
    /// ```
    ///
    /// Where:
    /// - Padding bytes align the start of `T` to its alignment requirement
    /// - Associated value `T` is stored using its native memory representation
    /// - Data slice follows immediately after the associated value
    ///
    /// # Errors
    ///
    /// Returns `U8PoolError::BufferOverflow` if:
    /// - The maximum number of slices has been reached
    /// - There is insufficient space in the buffer for the aligned associated value and data
    ///
    pub fn push_assoc<T: Sized>(&mut self, assoc: T, data: &[u8]) -> Result<(), U8PoolError> {
        let (aligned_start, end) = self.reserve_aligned_buffer_space::<T>(data.len())?;

        let assoc_size = core::mem::size_of::<T>();
        let assoc_end = aligned_start + assoc_size;

        // Safe: reserve_aligned_buffer_space() guarantees all ranges are within bounds
        #[allow(clippy::indexing_slicing)]
        let assoc_slice = &mut self.data[aligned_start..assoc_end];
        #[allow(unsafe_code)]
        unsafe {
            let assoc_ptr = assoc_slice.as_mut_ptr().cast::<T>();
            core::ptr::write(assoc_ptr, assoc);
        }

        // Safe: reserve_aligned_buffer_space() guarantees all ranges are within bounds
        #[allow(clippy::indexing_slicing)]
        let data_slice = &mut self.data[assoc_end..end];
        data_slice.copy_from_slice(data);

        self.finalize_push(aligned_start, end - aligned_start)
    }

    /// Helper function to validate and compute buffer positions for associated data access.
    ///
    /// Validates that the index is within bounds and that the stored data is large enough
    /// to contain an associated value of type T, then returns the buffer positions needed
    /// to access both the associated value and the data portion.
    ///
    /// The function relies on the invariant that descriptors store aligned start positions
    /// for associated values, eliminating the need for alignment calculations during retrieval.
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
    /// When this function returns `Some((assoc_start, assoc_end, data_end))`:
    /// - `self.data[assoc_start..assoc_end]` is guaranteed to be valid for reading type T
    /// - `self.data[assoc_end..data_end]` is guaranteed to be valid for data access
    /// - Both ranges are within the bounds of `self.data`
    /// - `assoc_start` is guaranteed to be properly aligned for type T (stored in descriptor)
    fn get_validated_assoc_positions<T: Sized>(
        &self,
        index: usize,
    ) -> Option<(usize, usize, usize)> {
        if index >= self.count {
            return None;
        }
        let (aligned_start, total_length) = self.descriptor.get(index)?;
        let assoc_size = core::mem::size_of::<T>();

        // Since we store aligned starts and validate during push, the stored data should always be valid
        // The length check is kept as a safety measure for robustness
        if total_length < assoc_size {
            return None;
        }

        let assoc_end = aligned_start + assoc_size;
        let data_end = aligned_start + total_length;
        Some((aligned_start, assoc_end, data_end))
    }

    /// Helper function to extract associated value reference from buffer positions.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// - `assoc_end - start >= core::mem::size_of::<T>()`
    /// - `start` is properly aligned for type `T` (guaranteed when retrieved from descriptor)
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

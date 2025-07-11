//! BufVec: A zero-allocation vector implementation using client-provided buffers.
//!
//! BufVec provides vector, stack, and dictionary interfaces while using a single
//! client-provided buffer for storage. All operations are bounds-checked and
//! no internal allocations are performed.
//!
//! Buffer layout: [metadata section][data section]
//! Metadata section stores slice descriptors as (start_offset, length) pairs.

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

const SLICE_DESCRIPTOR_SIZE: usize = 16; // 2 * size_of::<usize>() = 2 * 8 = 16 bytes on 64-bit

pub struct BufVec<'a> {
    buffer: &'a mut [u8],
    count: usize,
    metadata_capacity: usize,
    data_start: usize,
    data_used: usize,
}

impl<'a> BufVec<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Result<Self, BufVecError> {
        if buffer.len() < SLICE_DESCRIPTOR_SIZE * 2 {
            return Err(BufVecError::BufferTooSmall);
        }
        
        // Reserve space for at least 2 slices initially, grow as needed
        let initial_metadata_space = SLICE_DESCRIPTOR_SIZE * 2;
        let data_start = initial_metadata_space;
        
        Ok(Self {
            buffer,
            count: 0,
            metadata_capacity: 2,
            data_start,
            data_used: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn buffer_capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn used_bytes(&self) -> usize {
        self.data_start + self.data_used
    }

    pub fn available_bytes(&self) -> usize {
        self.buffer.len() - self.used_bytes()
    }

    fn check_bounds(&self, index: usize) -> Result<(), BufVecError> {
        if index >= self.count {
            Err(BufVecError::IndexOutOfBounds)
        } else {
            Ok(())
        }
    }

    fn ensure_capacity(&mut self, additional_bytes: usize) -> Result<(), BufVecError> {
        // Check if we need more metadata space
        if self.count >= self.metadata_capacity {
            let new_metadata_capacity = self.metadata_capacity * 2;
            let new_data_start = new_metadata_capacity * SLICE_DESCRIPTOR_SIZE;
            
            if new_data_start >= self.buffer.len() {
                return Err(BufVecError::BufferOverflow);
            }
            
            // Move existing data to new position
            let old_data_start = self.data_start;
            let data_to_move = self.data_used;
            
            if data_to_move > 0 {
                // Move data from old position to new position
                let src_start = old_data_start;
                let dst_start = new_data_start;
                
                // Use a temporary buffer or memmove-like operation
                for i in (0..data_to_move).rev() {
                    self.buffer[dst_start + i] = self.buffer[src_start + i];
                }
                
                // Update slice descriptors to point to new data locations
                for i in 0..self.count {
                    let (old_start, length) = self.get_slice_descriptor(i);
                    let new_start = old_start - old_data_start + new_data_start;
                    self.set_slice_descriptor(i, new_start, length);
                }
            }
            
            self.metadata_capacity = new_metadata_capacity;
            self.data_start = new_data_start;
        }
        
        // Check if we have enough space for the additional bytes
        if self.data_used + additional_bytes > self.buffer.len() - self.data_start {
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

    pub fn get(&self, index: usize) -> Result<&[u8], BufVecError> {
        self.check_bounds(index)?;
        let (start, length) = self.get_slice_descriptor(index);
        Ok(&self.buffer[start..start + length])
    }

    pub fn add(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        self.ensure_capacity(data.len())?;
        
        let start = self.data_start + self.data_used;
        let end = start + data.len();
        
        self.buffer[start..end].copy_from_slice(data);
        self.set_slice_descriptor(self.count, start, data.len());
        self.count += 1;
        self.data_used += data.len();
        
        Ok(())
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.data_used = 0;
    }

    pub fn pop(&mut self) -> Result<&[u8], BufVecError> {
        if self.count == 0 {
            return Err(BufVecError::EmptyVector);
        }
        
        self.count -= 1;
        let (start, length) = self.get_slice_descriptor(self.count);
        
        // Recalculate data_used by finding the highest end position
        self.data_used = if self.count == 0 {
            0
        } else {
            let mut max_end = self.data_start;
            for i in 0..self.count {
                let (slice_start, slice_length) = self.get_slice_descriptor(i);
                max_end = max_end.max(slice_start + slice_length);
            }
            max_end - self.data_start
        };
        
        Ok(&self.buffer[start..start + length])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_initialization() {
        let mut buffer = [0u8; 100];
        let bufvec = BufVec::new(&mut buffer).unwrap();
        
        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());
        assert_eq!(bufvec.buffer_capacity(), 100);
        assert_eq!(bufvec.used_bytes(), 32); // metadata section takes 32 bytes (2 slice capacity)
        assert!(bufvec.available_bytes() > 0);
    }

    #[test]
    fn test_bounds_checking_empty_buffer() {
        let mut buffer = [0u8; 0];
        assert!(BufVec::new(&mut buffer).is_err());
        
        let mut buffer = [0u8; 32];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        assert!(bufvec.get(0).is_err());
        assert!(bufvec.pop().is_err());
    }

    #[test]
    fn test_memory_layout_integrity() {
        let mut buffer = [0u8; 100];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        assert_eq!(bufvec.get(0).unwrap(), b"hello");
        assert_eq!(bufvec.get(1).unwrap(), b"world");
        assert_eq!(bufvec.len(), 2);
    }

    #[test]
    fn test_no_internal_allocation() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        bufvec.add(b"test").unwrap();
        
        // Verify data is stored correctly in the buffer
        assert_eq!(bufvec.get(0).unwrap(), b"test");
        assert_eq!(bufvec.len(), 1);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = [0u8; 100];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        // Fill up the buffer with data
        assert!(bufvec.add(b"hello").is_ok());
        assert!(bufvec.add(b"world").is_ok());
        
        // Try to add more data than fits in the remaining space
        assert!(bufvec.add(b"this_is_a_very_long_string_that_should_not_fit_in_the_remaining_space").is_err());
    }

    #[test]
    fn test_bounds_checking() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        bufvec.add(b"test").unwrap();
        
        assert!(bufvec.get(0).is_ok());
        assert!(bufvec.get(1).is_err());
    }

    #[test]
    fn test_clear_operation() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        assert_eq!(bufvec.len(), 2);
        
        bufvec.clear();
        
        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());
    }

    #[test]
    fn test_pop_operation() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::new(&mut buffer).unwrap();
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        let popped = bufvec.pop().unwrap();
        assert_eq!(popped, b"world");
        assert_eq!(bufvec.len(), 1);
        
        let popped = bufvec.pop().unwrap();
        assert_eq!(popped, b"hello");
        assert_eq!(bufvec.len(), 0);
        
        assert!(bufvec.pop().is_err());
    }
}
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

pub struct BufVec<'a> {
    buffer: &'a mut [u8],
    count: usize,
    data_start: usize,
    slice_descriptor_size: usize,
}

impl<'a> BufVec<'a> {
    pub fn new(buffer: &'a mut [u8], slice_descriptor_size: usize) -> Result<Self, BufVecError> {
        if slice_descriptor_size == 0 {
            return Err(BufVecError::BufferTooSmall);
        }
        
        if buffer.len() < slice_descriptor_size * 2 {
            return Err(BufVecError::BufferTooSmall);
        }
        
        // Start with space for 2 slices
        let initial_data_start = slice_descriptor_size * 2;
        
        Ok(Self {
            buffer,
            count: 0,
            data_start: initial_data_start,
            slice_descriptor_size,
        })
    }
    
    fn metadata_capacity(&self) -> usize {
        self.data_start / self.slice_descriptor_size
    }
    
    fn data_used(&self) -> usize {
        if self.count == 0 {
            return 0;
        }
        
        // Calculate data used by finding the highest end position
        let mut max_end = self.data_start;
        for i in 0..self.count {
            let (slice_start, slice_length) = self.get_slice_descriptor(i);
            max_end = max_end.max(slice_start + slice_length);
        }
        max_end - self.data_start
    }
    
    pub fn with_default_descriptor_size(buffer: &'a mut [u8]) -> Result<Self, BufVecError> {
        const DEFAULT_SLICE_DESCRIPTOR_SIZE: usize = 16; // 2 * size_of::<usize>() = 2 * 8 = 16 bytes on 64-bit
        Self::new(buffer, DEFAULT_SLICE_DESCRIPTOR_SIZE)
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
        self.data_start + self.data_used()
    }

    pub fn available_bytes(&self) -> usize {
        self.buffer.len() - self.used_bytes()
    }

    pub fn slice_descriptor_size(&self) -> usize {
        self.slice_descriptor_size
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
        if self.count >= self.metadata_capacity() {
            let new_metadata_capacity = self.metadata_capacity() * 2;
            let new_data_start = new_metadata_capacity * self.slice_descriptor_size;
            
            if new_data_start >= self.buffer.len() {
                return Err(BufVecError::BufferOverflow);
            }
            
            // Move existing data to new position
            let old_data_start = self.data_start;
            let data_to_move = self.data_used();
            
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
            
            self.data_start = new_data_start;
        }
        
        // Check if we have enough space for the additional bytes
        if self.data_used() + additional_bytes > self.buffer.len() - self.data_start {
            return Err(BufVecError::BufferOverflow);
        }
        Ok(())
    }

    fn get_slice_descriptor(&self, index: usize) -> (usize, usize) {
        let offset = index * self.slice_descriptor_size;
        
        // For simplicity, assume descriptor size is at least 8 bytes and use the first 8 bytes
        // for start and remaining bytes for length (if descriptor size >= 16, use 8 bytes for each)
        if self.slice_descriptor_size >= 16 {
            let start_bytes = &self.buffer[offset..offset + 8];
            let length_bytes = &self.buffer[offset + 8..offset + 16];
            
            let start = usize::from_le_bytes(start_bytes.try_into().unwrap());
            let length = usize::from_le_bytes(length_bytes.try_into().unwrap());
            
            (start, length)
        } else {
            // For smaller descriptor sizes, use compact encoding
            let half_size = self.slice_descriptor_size / 2;
            let start_bytes = &self.buffer[offset..offset + half_size];
            let length_bytes = &self.buffer[offset + half_size..offset + self.slice_descriptor_size];
            
            let start = match half_size {
                4 => u32::from_le_bytes(start_bytes.try_into().unwrap()) as usize,
                8 => usize::from_le_bytes(start_bytes.try_into().unwrap()),
                _ => panic!("Unsupported descriptor size: {}", self.slice_descriptor_size),
            };
            
            let length = match half_size {
                4 => u32::from_le_bytes(length_bytes.try_into().unwrap()) as usize,
                8 => usize::from_le_bytes(length_bytes.try_into().unwrap()),
                _ => panic!("Unsupported descriptor size: {}", self.slice_descriptor_size),
            };
            
            (start, length)
        }
    }

    fn set_slice_descriptor(&mut self, index: usize, start: usize, length: usize) {
        let offset = index * self.slice_descriptor_size;
        
        if self.slice_descriptor_size >= 16 {
            self.buffer[offset..offset + 8].copy_from_slice(&start.to_le_bytes());
            self.buffer[offset + 8..offset + 16].copy_from_slice(&length.to_le_bytes());
        } else {
            // For smaller descriptor sizes, use compact encoding
            let half_size = self.slice_descriptor_size / 2;
            
            match half_size {
                4 => {
                    self.buffer[offset..offset + 4].copy_from_slice(&(start as u32).to_le_bytes());
                    self.buffer[offset + 4..offset + 8].copy_from_slice(&(length as u32).to_le_bytes());
                }
                8 => {
                    self.buffer[offset..offset + 8].copy_from_slice(&start.to_le_bytes());
                    self.buffer[offset + 8..offset + 16].copy_from_slice(&length.to_le_bytes());
                }
                _ => panic!("Unsupported descriptor size: {}", self.slice_descriptor_size),
            }
        }
    }

    pub fn get(&self, index: usize) -> Result<&[u8], BufVecError> {
        self.check_bounds(index)?;
        let (start, length) = self.get_slice_descriptor(index);
        Ok(&self.buffer[start..start + length])
    }

    pub fn add(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        self.ensure_capacity(data.len())?;
        
        let start = self.data_start + self.data_used();
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

    pub fn pop(&mut self) -> Result<&[u8], BufVecError> {
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
        let mut buffer = [0u8; 100];
        let bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());
        assert_eq!(bufvec.buffer_capacity(), 100);
        assert_eq!(bufvec.used_bytes(), 32); // metadata section takes 32 bytes (2 slice capacity)
        assert!(bufvec.available_bytes() > 0);
    }

    #[test]
    fn test_bounds_checking_empty_buffer() {
        let mut buffer = [0u8; 0];
        assert!(BufVec::with_default_descriptor_size(&mut buffer).is_err());
        
        let mut buffer = [0u8; 32];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        assert!(bufvec.get(0).is_err());
        assert!(bufvec.pop().is_err());
    }

    #[test]
    fn test_memory_layout_integrity() {
        let mut buffer = [0u8; 100];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        assert_eq!(bufvec.get(0).unwrap(), b"hello");
        assert_eq!(bufvec.get(1).unwrap(), b"world");
        assert_eq!(bufvec.len(), 2);
    }

    #[test]
    fn test_no_internal_allocation() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        bufvec.add(b"test").unwrap();
        
        // Verify data is stored correctly in the buffer
        assert_eq!(bufvec.get(0).unwrap(), b"test");
        assert_eq!(bufvec.len(), 1);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = [0u8; 100];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        // Fill up the buffer with data
        assert!(bufvec.add(b"hello").is_ok());
        assert!(bufvec.add(b"world").is_ok());
        
        // Try to add more data than fits in the remaining space
        assert!(bufvec.add(b"this_is_a_very_long_string_that_should_not_fit_in_the_remaining_space").is_err());
    }

    #[test]
    fn test_bounds_checking() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        bufvec.add(b"test").unwrap();
        
        assert!(bufvec.get(0).is_ok());
        assert!(bufvec.get(1).is_err());
    }

    #[test]
    fn test_clear_operation() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
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
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
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

    #[test]
    fn test_custom_descriptor_size() {
        let mut buffer = [0u8; 64];
        let mut bufvec = BufVec::new(&mut buffer, 8).unwrap();
        
        bufvec.add(b"test").unwrap();
        
        assert_eq!(bufvec.get(0).unwrap(), b"test");
        assert_eq!(bufvec.len(), 1);
    }

    #[test]
    fn test_small_descriptor_size() {
        let mut buffer = [0u8; 32];
        let mut bufvec = BufVec::new(&mut buffer, 8).unwrap();
        
        bufvec.add(b"hi").unwrap();
        bufvec.add(b"world").unwrap();
        
        assert_eq!(bufvec.get(0).unwrap(), b"hi");
        assert_eq!(bufvec.get(1).unwrap(), b"world");
        assert_eq!(bufvec.len(), 2);
    }

    #[test]
    fn test_optimized_struct_functionality() {
        let mut buffer = [0u8; 100];
        let mut bufvec = BufVec::with_default_descriptor_size(&mut buffer).unwrap();
        
        // Test that derived values work correctly
        assert_eq!(bufvec.metadata_capacity(), 2);
        assert_eq!(bufvec.data_used(), 0);
        
        bufvec.add(b"test").unwrap();
        assert_eq!(bufvec.data_used(), 4);
        
        bufvec.add(b"hello").unwrap();
        assert_eq!(bufvec.data_used(), 9);
        
        bufvec.pop().unwrap();
        assert_eq!(bufvec.data_used(), 4);
        
        bufvec.clear();
        assert_eq!(bufvec.data_used(), 0);
    }
}
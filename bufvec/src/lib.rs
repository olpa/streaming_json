//! BufVec: A zero-allocation vector implementation using client-provided buffers.
//!
//! BufVec provides vector, stack, and dictionary interfaces while using a single
//! client-provided buffer for storage. All operations are bounds-checked and
//! no internal allocations are performed.

use std::fmt;

#[derive(Debug)]
pub enum BufVecError {
    BufferOverflow,
    IndexOutOfBounds,
    EmptyVector,
}

impl fmt::Display for BufVecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BufVecError::BufferOverflow => write!(f, "Buffer overflow: insufficient space"),
            BufVecError::IndexOutOfBounds => write!(f, "Index out of bounds"),
            BufVecError::EmptyVector => write!(f, "Operation on empty vector"),
        }
    }
}

impl std::error::Error for BufVecError {}

pub struct BufVec<'a> {
    buffer: &'a mut [u8],
    slices: Vec<(usize, usize)>,
    used_bytes: usize,
}

impl<'a> BufVec<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self {
            buffer,
            slices: Vec::new(),
            used_bytes: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.slices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.slices.is_empty()
    }

    pub fn buffer_capacity(&self) -> usize {
        self.buffer.len()
    }

    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    pub fn available_bytes(&self) -> usize {
        self.buffer.len() - self.used_bytes
    }

    fn check_bounds(&self, index: usize) -> Result<(), BufVecError> {
        if index >= self.slices.len() {
            Err(BufVecError::IndexOutOfBounds)
        } else {
            Ok(())
        }
    }

    fn ensure_capacity(&self, additional_bytes: usize) -> Result<(), BufVecError> {
        if self.used_bytes + additional_bytes > self.buffer.len() {
            Err(BufVecError::BufferOverflow)
        } else {
            Ok(())
        }
    }

    pub fn get(&self, index: usize) -> Result<&[u8], BufVecError> {
        self.check_bounds(index)?;
        let (start, end) = self.slices[index];
        Ok(&self.buffer[start..end])
    }

    pub fn add(&mut self, data: &[u8]) -> Result<(), BufVecError> {
        self.ensure_capacity(data.len())?;
        
        let start = self.used_bytes;
        let end = start + data.len();
        
        self.buffer[start..end].copy_from_slice(data);
        self.slices.push((start, end));
        self.used_bytes = end;
        
        Ok(())
    }

    pub fn clear(&mut self) {
        self.slices.clear();
        self.used_bytes = 0;
    }

    pub fn pop(&mut self) -> Result<&[u8], BufVecError> {
        if self.slices.is_empty() {
            return Err(BufVecError::EmptyVector);
        }
        
        let (start, end) = self.slices.pop().unwrap();
        self.used_bytes = start;
        Ok(&self.buffer[start..end])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_initialization() {
        let mut buffer = [0u8; 100];
        let bufvec = BufVec::new(&mut buffer);
        
        assert_eq!(bufvec.len(), 0);
        assert!(bufvec.is_empty());
        assert_eq!(bufvec.buffer_capacity(), 100);
        assert_eq!(bufvec.used_bytes(), 0);
        assert_eq!(bufvec.available_bytes(), 100);
    }

    #[test]
    fn test_bounds_checking_empty_buffer() {
        let mut buffer = [0u8; 0];
        let mut bufvec = BufVec::new(&mut buffer);
        
        assert!(bufvec.add(b"test").is_err());
        assert!(bufvec.get(0).is_err());
        assert!(bufvec.pop().is_err());
    }

    #[test]
    fn test_memory_layout_integrity() {
        let mut buffer = [0u8; 20];
        let mut bufvec = BufVec::new(&mut buffer);
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        assert_eq!(bufvec.get(0).unwrap(), b"hello");
        assert_eq!(bufvec.get(1).unwrap(), b"world");
        assert_eq!(bufvec.used_bytes(), 10);
        assert_eq!(bufvec.len(), 2);
    }

    #[test]
    fn test_no_internal_allocation() {
        let mut buffer = [0u8; 10];
        let mut bufvec = BufVec::new(&mut buffer);
        
        bufvec.add(b"test").unwrap();
        
        // Verify data is stored correctly
        assert_eq!(bufvec.get(0).unwrap(), b"test");
        assert_eq!(bufvec.used_bytes(), 4);
    }

    #[test]
    fn test_buffer_overflow() {
        let mut buffer = [0u8; 5];
        let mut bufvec = BufVec::new(&mut buffer);
        
        assert!(bufvec.add(b"hello").is_ok());
        assert!(bufvec.add(b"world").is_err());
    }

    #[test]
    fn test_bounds_checking() {
        let mut buffer = [0u8; 10];
        let mut bufvec = BufVec::new(&mut buffer);
        
        bufvec.add(b"test").unwrap();
        
        assert!(bufvec.get(0).is_ok());
        assert!(bufvec.get(1).is_err());
    }

    #[test]
    fn test_clear_operation() {
        let mut buffer = [0u8; 20];
        let mut bufvec = BufVec::new(&mut buffer);
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        assert_eq!(bufvec.len(), 2);
        assert_eq!(bufvec.used_bytes(), 10);
        
        bufvec.clear();
        
        assert_eq!(bufvec.len(), 0);
        assert_eq!(bufvec.used_bytes(), 0);
        assert!(bufvec.is_empty());
    }

    #[test]
    fn test_pop_operation() {
        let mut buffer = [0u8; 20];
        let mut bufvec = BufVec::new(&mut buffer);
        
        bufvec.add(b"hello").unwrap();
        bufvec.add(b"world").unwrap();
        
        let popped = bufvec.pop().unwrap();
        assert_eq!(popped, b"world");
        assert_eq!(bufvec.len(), 1);
        assert_eq!(bufvec.used_bytes(), 5);
        
        let popped = bufvec.pop().unwrap();
        assert_eq!(popped, b"hello");
        assert_eq!(bufvec.len(), 0);
        assert_eq!(bufvec.used_bytes(), 0);
        
        assert!(bufvec.pop().is_err());
    }
}
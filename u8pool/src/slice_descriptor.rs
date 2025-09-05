use crate::error::U8PoolError;

const SLICE_DESCRIPTOR_SIZE: usize = 4; // 2 bytes start + 2 bytes length

/// Handles reading and writing slice descriptor data from/to buffer
/// Uses 2-byte values for start and length positions
#[derive(Debug)]
pub struct SliceDescriptor<'a> {
    buffer: &'a mut [u8],
}

impl<'a> SliceDescriptor<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer }
    }

    pub fn get(&self, index: usize) -> Option<(usize, usize)> {
        let offset = index * SLICE_DESCRIPTOR_SIZE;
        if offset + SLICE_DESCRIPTOR_SIZE > self.buffer.len() {
            return None;
        }
        
        let start = (self.buffer[offset] as u16) | 
                   ((self.buffer[offset + 1] as u16) << 8);
        
        let length = (self.buffer[offset + 2] as u16) | 
                    ((self.buffer[offset + 3] as u16) << 8);
        
        Some((start as usize, length as usize))
    }

    pub fn set(&mut self, index: usize, start: usize, length: usize) -> Result<(), U8PoolError> {
        if start > u16::MAX as usize {
            return Err(U8PoolError::ValueTooLarge {
                value: start,
                max: u16::MAX as usize,
            });
        }
        if length > u16::MAX as usize {
            return Err(U8PoolError::ValueTooLarge {
                value: length,
                max: u16::MAX as usize,
            });
        }
        
        let offset = index * SLICE_DESCRIPTOR_SIZE;
        if offset + SLICE_DESCRIPTOR_SIZE > self.buffer.len() {
            return Err(U8PoolError::IndexOutOfBounds {
                index,
                length: self.buffer.len() / SLICE_DESCRIPTOR_SIZE,
            });
        }
        
        let start_u16 = start as u16;
        let length_u16 = length as u16;
        
        self.buffer[offset] = start_u16 as u8;
        self.buffer[offset + 1] = (start_u16 >> 8) as u8;
        self.buffer[offset + 2] = length_u16 as u8;
        self.buffer[offset + 3] = (length_u16 >> 8) as u8;
        
        Ok(())
    }
}
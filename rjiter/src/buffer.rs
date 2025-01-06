use std::io::Read;

pub(crate) struct Buffer<'buf> {
    reader: &'buf mut dyn Read,
    pub buffer: &'buf mut [u8],
    pub bytes_in_buffer: usize,
}

impl<'buf> Buffer<'buf> {
    pub fn new(reader: &'buf mut dyn Read, buffer: &'buf mut [u8]) -> Self {
        let bytes_in_buffer = reader.read(buffer).unwrap();
        
        Buffer {
            reader,
            buffer,
            bytes_in_buffer,
        }
    }

    pub fn read_more(&mut self, start_index: usize) -> usize {
        let n_new_bytes = self.reader.read(&mut self.buffer[start_index..]).unwrap();
        self.bytes_in_buffer = start_index + n_new_bytes;
        n_new_bytes
    }

    pub fn shift_buffer(&mut self, pos: usize, is_partial_string: bool) {
        if pos > 0 {
            if pos < self.bytes_in_buffer {
                assert!(
                    !is_partial_string,
                    "Buffer should be completely consumed in partial string case"
                );

                self.buffer.copy_within(pos..self.bytes_in_buffer, 0);
                self.bytes_in_buffer -= pos;
            } else {
                self.bytes_in_buffer = 0;
            }
        }
    }
}
impl<'buf> std::fmt::Debug for Buffer<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,
            "Buffer {{ bytes_in_buffer: {:?}, buffer: {:?} }}",
            self.bytes_in_buffer,
            self.buffer
        )
    }
}

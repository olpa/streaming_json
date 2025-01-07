use std::io::Read;

pub struct Buffer<'buf> {
    reader: &'buf mut dyn Read,
    pub buf: &'buf mut [u8],
    pub n_bytes: usize,
}

impl<'buf> Buffer<'buf> {
    #[allow(clippy::missing_panics_doc)]
    pub fn new(reader: &'buf mut dyn Read, buf: &'buf mut [u8]) -> Self {
        let n_bytes = reader.read(buf).unwrap();

        Buffer {
            reader,
            buf,
            n_bytes,
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn read_more(&mut self, start_index: usize) -> usize {
        println!("Buffer::read_more before read: reading to the buffer: {:?}", &self.buf[start_index..]); // FIXME
        let n_new_bytes = self.reader.read(&mut self.buf[start_index..]).unwrap();
        println!("Buffer::read_more after read: n_new_bytes: {:?}", n_new_bytes); // FIXME
        self.n_bytes += n_new_bytes;
        n_new_bytes
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn shift_buffer(&mut self, to_pos: usize, from_pos: usize) {
        if from_pos > to_pos && from_pos < self.n_bytes {
            self.buf.copy_within(from_pos..self.n_bytes, to_pos);
            self.n_bytes -= from_pos - to_pos;
        } else {
            self.n_bytes = to_pos;
        }
    }

    pub fn skip_spaces(&mut self, pos: usize) {
        let mut i = pos;
        loop {
            while i < self.n_bytes && self.buf[i].is_ascii_whitespace() {
                i += 1;
            }
            
            if i < self.n_bytes {
                // Found non-whitespace
                if i > pos {
                    self.shift_buffer(pos, i);
                }
                break;
            }
            
            println!("Buffer::skip_space before shift: pos: {:?}, i: {:?}, n_bytes: {:?}", pos, i, self.n_bytes); // FIXME

            // Reached end of buffer, shift and read more
            self.shift_buffer(pos, self.n_bytes);
            println!("Buffer::skip_space after shift: pos: {:?}, i: {:?}, n_bytes: {:?}", pos, i, self.n_bytes); // FIXME
            let n_new = self.read_more(self.n_bytes);
            println!("Buffer::skip_space after read: pos: {:?}, i: {:?}, n_bytes: {:?}, n_new: {:?}", pos, i, self.n_bytes, n_new); // FIXME
            if n_new == 0 {
                // EOF reached
                break;
            }
            i = self.n_bytes - n_new;
        }
    }
}

impl<'buf> std::fmt::Debug for Buffer<'buf> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Buffer {{ n_bytes: {:?}, buf: {:?} }}",
            self.n_bytes, self.buf
        )
    }
}

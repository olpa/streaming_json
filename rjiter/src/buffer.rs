use std::cmp::min;
use std::io::Read;

pub struct Buffer<'buf> {
    reader: &'buf mut dyn Read,
    pub buf: &'buf mut [u8],
    pub n_bytes: usize,
    pub n_shifted_out: usize,
}

impl<'buf> Buffer<'buf> {
    #[allow(clippy::missing_panics_doc)]
    pub fn new(reader: &'buf mut dyn Read, buf: &'buf mut [u8]) -> Self {
        let n_bytes = reader.read(buf).unwrap();

        Buffer {
            reader,
            buf,
            n_bytes,
            n_shifted_out: 0,
        }
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn read_more(&mut self) -> usize {
        let n_new_bytes = self.reader.read(&mut self.buf[self.n_bytes..]).unwrap();
        self.n_bytes += n_new_bytes;
        n_new_bytes
    }

    pub fn read_more_to_pos(&mut self, start_index: usize) -> usize {
        let n_new_bytes = self.reader.read(&mut self.buf[start_index..]).unwrap();
        self.n_bytes += n_new_bytes;
        n_new_bytes
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn shift_buffer(&mut self, to_pos: usize, from_pos: usize) {
        if from_pos > to_pos && to_pos < self.n_bytes {
            if from_pos < self.n_bytes {
                self.buf.copy_within(from_pos..self.n_bytes, to_pos);
            }
            let from_pos = min(from_pos, self.n_bytes);
            let n_shifted_out = from_pos - to_pos;
            self.n_bytes -= n_shifted_out;
            self.n_shifted_out += n_shifted_out;
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

            // Reached end of buffer, shift and read more
            self.shift_buffer(pos, self.n_bytes);
            let n_new = self.read_more();
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
            "Buffer {{ n_bytes: {:?}, n_shifted_out: {:?}, buf: {:?} }}",
            self.n_bytes, self.n_shifted_out, self.buf
        )
    }
}

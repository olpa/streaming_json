use std::io::Read;

pub struct Buffer<'buf> {
    reader: &'buf mut dyn Read,
    pub buf: &'buf mut [u8],
    pub n_bytes: usize,
}

impl<'buf> Buffer<'buf> {
    pub fn new(reader: &'buf mut dyn Read, buf: &'buf mut [u8]) -> Self {
        let n_bytes = reader.read(buf).unwrap();

        Buffer {
            reader,
            buf,
            n_bytes,
        }
    }

    pub fn read_more(&mut self, start_index: usize) -> usize {
        let n_new_bytes = self.reader.read(&mut self.buf[start_index..]).unwrap();
        self.n_bytes += n_new_bytes;
        n_new_bytes
    }

    pub fn shift_buffer(&mut self, pos: usize, is_partial_string: bool) {
        if pos > 0 {
            if pos < self.n_bytes {
                assert!(
                    !is_partial_string,
                    "Buffer should be completely consumed in partial string case"
                );

                self.buf.copy_within(pos..self.n_bytes, 0);
                self.n_bytes -= pos;
            } else {
                self.n_bytes = 0;
            }
        }
    }

    pub fn skip_spaces(&mut self, pos: usize) {
        let mut i = pos;
        while i < self.n_bytes && self.buf[i].is_ascii_whitespace() {
            i += 1;
        }
        if i > pos {
            self.buf.copy_within(i..self.n_bytes, pos);
            self.n_bytes -= i - pos;
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

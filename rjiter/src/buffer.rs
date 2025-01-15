use std::cmp::min;
use std::io::Read;

pub struct Buffer<'buf> {
    reader: &'buf mut dyn Read,
    pub buf: &'buf mut [u8],
    pub n_bytes: usize,
    pub n_shifted_out: usize,
}

impl<'buf> Buffer<'buf> {
    pub fn new(reader: &'buf mut dyn Read, buf: &'buf mut [u8]) -> std::io::Result<Self> {
        let n_bytes = reader.read(buf)?;

        Ok(Buffer {
            reader,
            buf,
            n_bytes,
            n_shifted_out: 0,
        })
    }

    pub fn read_more(&mut self) -> std::io::Result<usize> {
        let n_new_bytes = self.reader.read(&mut self.buf[self.n_bytes..])?;
        self.n_bytes += n_new_bytes;
        Ok(n_new_bytes)
    }

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

    pub fn skip_spaces(&mut self, pos: usize) -> std::io::Result<()> {
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
            let n_new = self.read_more()?;
            if n_new == 0 {
                // EOF reached
                break;
            }
            i = self.n_bytes - n_new;
        }
        Ok(())
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

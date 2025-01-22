use std::cmp::min;
use std::io::Read;

use crate::LinePosition;

pub struct Buffer<'buf> {
    reader: &'buf mut dyn Read,
    pub buf: &'buf mut [u8],
    pub n_bytes: usize, // Size of the buffer
    pub n_shifted_out: usize, // Number of bytes shifted out
    pub pos_shifted: LinePosition, // Correction for the error position due to shifting
}

impl<'buf> Buffer<'buf> {
    #[must_use]
    pub fn new(reader: &'buf mut dyn Read, buf: &'buf mut [u8]) -> Self {
        Buffer {
            reader,
            buf,
            n_bytes: 0,
            n_shifted_out: 0,
            pos_shifted: LinePosition::new(0, 0),
        }
    }

    /// Read from the underlying reader into the buffer.
    ///
    /// Returns the number of bytes read.
    ///
    /// # Errors
    ///
    /// From the underlying reader.
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

    /// Skip over any ASCII whitespace characters starting at the given position.
    /// Read-shift-read-shift-read-shift... until non-whitespace is found or EOF is reached.
    ///
    /// # Arguments
    ///
    /// * `pos` - The position in the buffer to start skipping from
    ///
    /// # Errors
    ///
    /// From the underlying reader.
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

pub struct ChangeFlag {
    n_shifted: usize,
    n_bytes: usize,
}

impl ChangeFlag {
    #[must_use]
    pub fn new(buf: &Buffer) -> Self {
        ChangeFlag {
            n_shifted: buf.n_shifted_out,
            n_bytes: buf.n_bytes,
        }
    }

    #[must_use]
    pub fn is_changed(&self, buf: &Buffer) -> bool {
        self.n_shifted != buf.n_shifted_out || self.n_bytes != buf.n_bytes
    }
}

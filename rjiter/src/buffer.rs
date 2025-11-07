use core::cmp::min;
use embedded_io::{Error as _, Read};

use crate::error::{Error, ErrorType, Result as RJiterResult};
use crate::jiter::LinePosition;

/// A buffer for reading JSON data.
/// Is a private struct, the "pub" is only for testing.
pub struct Buffer<'buf, R: Read> {
    reader: &'buf mut R,
    /// The working buffer for reading JSON data.
    pub buf: &'buf mut [u8],
    /// Number of valid bytes in the buffer. Contract: `n_bytes <= buf.len()`
    pub n_bytes: usize,
    /// Number of bytes that have been shifted out of the buffer.
    pub n_shifted_out: usize,
    /// Line position correction due to shifting operations.
    pub pos_shifted: LinePosition,
}

impl<'buf, R: Read> Buffer<'buf, R> {
    /// Creates a new buffer with the given reader and buffer.
    #[must_use]
    pub fn new(reader: &'buf mut R, buf: &'buf mut [u8]) -> Self {
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
    pub fn read_more(&mut self) -> RJiterResult<usize> {
        // The only place where `n_bytes` is increased is this `read_more` function.
        // As long as `read` works correctly, `n_bytes` is less or equal to the buffer size.
        #[allow(clippy::indexing_slicing)]
        let n_new_bytes = self
            .reader
            .read(&mut self.buf[self.n_bytes..])
            .map_err(|e| Error {
                error_type: ErrorType::IoError { kind: e.kind() },
                index: self.n_bytes,
            })?;
        self.n_bytes += n_new_bytes;
        Ok(n_new_bytes)
    }

    /// Shift the buffer to the left, and update the index and line-column position.
    ///
    /// # Arguments
    ///
    /// * `to_pos`: The position to shift to. Usually is 0 or is 1 for strings.
    /// * `from_pos`: The position to shift from. The case of outside the buffer is handled.
    pub fn shift_buffer(&mut self, to_pos: usize, from_pos: usize) {
        let safe_from_pos = min(from_pos, self.n_bytes);
        if to_pos < safe_from_pos {
            // `to_pos>=0` (`usize`), `to_pos < safe_from_pos` (if-branch), `safe_from_pos`<=`n_bytes <= buf.len()` (contract)
            #[allow(clippy::indexing_slicing)]
            for ch in &self.buf[to_pos..safe_from_pos] {
                if *ch == b'\n' {
                    self.pos_shifted.line += 1;
                    self.pos_shifted.column = 0;
                } else {
                    self.pos_shifted.column += 1;
                }
            }
        }

        if from_pos > to_pos && to_pos < self.n_bytes {
            if from_pos < self.n_bytes {
                self.buf.copy_within(from_pos..self.n_bytes, to_pos);
            }
            let n_shifted_out = safe_from_pos - to_pos;
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
    pub fn skip_spaces(&mut self, pos: usize) -> RJiterResult<()> {
        let mut i = pos;
        loop {
            // `i >= 0` (`usize`), `self.n_bytes <= buf.len()` (contract)
            #[allow(clippy::indexing_slicing)]
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

    /// Collect bytes while a predicate is true, starting at the given position.
    /// Returns the offset of the first rejected byte, or EOF.
    /// If buffer is full with all accepted bytes, it's an error.
    /// The function can optionally shift the buffer once to the target position to discard unneeded bytes.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A function that returns true if the byte should be accepted
    /// * `start_pos` - The position in the buffer to start collecting from
    /// * `allow_shift` - If true, allows shifting the buffer once when it fills up
    /// * `shift_target_pos` - The position to shift to if more space is needed (only used if `allow_shift` is true)
    ///
    /// # Errors
    ///
    /// Returns `ErrorType::BufferFull` if the buffer fills up with all accepted bytes.
    /// Also returns errors from the underlying reader.
    pub fn collect_while<F>(&mut self, predicate: F, start_pos: usize, allow_shift: bool, shift_target_pos: usize) -> RJiterResult<usize>
    where
        F: Fn(u8) -> bool,
    {
        let mut i = start_pos;

        loop {
            // Check bytes while predicate is true
            #[allow(clippy::indexing_slicing)]
            while i < self.n_bytes && predicate(self.buf[i]) {
                i += 1;
            }

            if i < self.n_bytes {
                // Found rejected byte
                return Ok(i);
            }

            // Reached end of buffer, try to read more
            let n_new = self.read_more()?;
            if n_new == 0 {
                // EOF reached, all bytes were accepted
                return Ok(self.n_bytes);
            }

            // Check if buffer is full after reading
            if self.n_bytes >= self.buf.len() {
                // Buffer is full
                if !allow_shift {
                    // Shifting not allowed, cannot make progress - error!
                    return Err(Error {
                        error_type: ErrorType::BufferFull,
                        index: self.n_shifted_out + shift_target_pos,
                    });
                }
                // Shift once to make space and recurse with allow_shift = false
                self.shift_buffer(shift_target_pos, self.n_bytes);
                return self.collect_while(predicate, shift_target_pos, false, shift_target_pos);
            }
        }
    }
}

impl<R: Read> core::fmt::Debug for Buffer<'_, R> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Buffer {{ n_bytes: {:?}, buf: {:?}, n_shifted_out: {:?}, pos_shifted: {:?} }}",
            self.n_bytes, self.buf, self.n_shifted_out, self.pos_shifted
        )
    }
}

/// A helper struct to check if the buffer has changed and therefore `Jiter` needs to be recreated.
/// Is a private struct, the "pub" is only for testing.
pub(crate) struct ChangeFlag {
    n_shifted: usize,
    n_bytes: usize,
}

impl ChangeFlag {
    #[must_use]
    pub fn new<R: Read>(buf: &Buffer<R>) -> Self {
        ChangeFlag {
            n_shifted: buf.n_shifted_out,
            n_bytes: buf.n_bytes,
        }
    }

    #[must_use]
    pub fn is_changed<R: Read>(&self, buf: &Buffer<R>) -> bool {
        self.n_shifted != buf.n_shifted_out || self.n_bytes != buf.n_bytes
    }
}
